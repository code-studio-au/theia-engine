// Fixed qualification ABI: 128 handles, 2 chunks/archetype, 32 dense rows/chunk.
// One lane orders small structural batches; this is not the production allocator.
[[vk::binding(0, 0)]] RWByteAddressBuffer arena;
[[vk::binding(1, 0)]] ByteAddressBuffer commands;
[[vk::binding(2, 0)]] RWByteAddressBuffer results;
[[vk::binding(3, 0)]] ByteAddressBuffer retained;
[[vk::binding(4, 0)]] RWByteAddressBuffer readerOutput;
struct Parameters { uint count; uint oldest; float dt; uint unused; };
[[vk::push_constant]] ConstantBuffer<Parameters> parameters;
static const uint SLOTS = 128, ROWS = 128, WORDS = 1541;
static const uint COUNT = 1, GEN = 5, WORLD_LO = 133, WORLD_HI = 261;
static const uint ARCH = 389, ROW = 517, RETIRED = 645, EXHAUSTED = 773;
static const uint HANDLE = 901, PX = 1029, PY = 1157, VX = 1285, VY = 1413;
uint get(uint word) { return arena.Load(word * 4); }
void put(uint word, uint value) { arena.Store(word * 4, value); }
float readFloat(uint word) { return asfloat(get(word)); }
void writeFloat(uint word, float value) { put(word, asuint(value)); }
[numthreads(64, 1, 1)]
void initialize(uint3 tid : SV_DispatchThreadID) {
    uint i = tid.x;
    if (i < WORDS) put(i, i >= GEN && i < GEN + SLOTS ? parameters.unused : 0);
}
void reclaim(uint epoch) {
    for (uint s = 0; s < SLOTS; ++s) {
        uint retiredEpoch = get(RETIRED + s);
        if (retiredEpoch != 0 && parameters.oldest >= retiredEpoch) {
            put(RETIRED + s, 0);
            uint generation = get(GEN + s);
            if (generation == 0xffffffff) put(EXHAUSTED + s, 1);
            else put(GEN + s, generation + 1);
        }
    }
}
uint destination(uint arch) {
    uint first = (arch - 1) * 2;
    for (uint c = first; c < first + 2; ++c)
        if (get(COUNT + c) < 32) return c * 32 + get(COUNT + c);
    return 0xffffffff;
}
void removeRow(uint row) {
    uint chunk = row / 32;
    uint count = get(COUNT + chunk) - 1;
    uint last = chunk * 32 + count;
    if (row != last) {
        uint moved = get(HANDLE + last);
        put(HANDLE + row, moved);
        put(ROW + moved, row);
        put(PX + row, get(PX + last)); put(PY + row, get(PY + last));
        put(VX + row, get(VX + last)); put(VY + row, get(VY + last));
    }
    put(COUNT + chunk, count);
}
void insertRow(uint slot, uint arch, uint row, uint px, uint py, uint vx, uint vy) {
    put(HANDLE + row, slot); put(ARCH + slot, arch); put(ROW + slot, row);
    put(PX + row, px); put(PY + row, py); put(VX + row, vx); put(VY + row, vy);
    put(COUNT + row / 32, get(COUNT + row / 32) + 1);
}
[numthreads(1, 1, 1)]
void mutate(uint3 tid : SV_DispatchThreadID) {
    uint epoch = get(0) + 1;
    put(0, epoch);
    reclaim(epoch);
    for (uint n = 0; n < parameters.count; ++n) {
        uint offset = n * 40;
        uint op = commands.Load(offset), slot = commands.Load(offset + 4);
        uint generation = commands.Load(offset + 8);
        uint lo = commands.Load(offset + 12), hi = commands.Load(offset + 16);
        uint px = commands.Load(offset + 20), py = commands.Load(offset + 24);
        uint vx = commands.Load(offset + 28), vy = commands.Load(offset + 32);
        uint arch = commands.Load(offset + 36) != 0 ? 2 : 1;
        uint status = 0;
        if ((op == 0 && (!isfinite(asfloat(px)) || !isfinite(asfloat(py)))) ||
            ((op == 0 || op == 2) && arch == 2 && (!isfinite(asfloat(vx)) || !isfinite(asfloat(vy))))) status = 5;
        else if (op == 0) {
            uint freeSlot = 0xffffffff;
            for (uint s = 0; s < SLOTS; ++s) {
                if (get(ARCH + s) != 0 && get(WORLD_LO + s) == lo && get(WORLD_HI + s) == hi) status = 2;
                if (freeSlot == 0xffffffff && get(ARCH + s) == 0 && get(RETIRED + s) == 0 && get(EXHAUSTED + s) == 0) freeSlot = s;
            }
            uint row = destination(arch);
            if (status == 0 && freeSlot == 0xffffffff) status = 3;
            if (status == 0 && row == 0xffffffff) status = 4;
            if (status == 0) {
                slot = freeSlot; generation = get(GEN + slot);
                put(WORLD_LO + slot, lo); put(WORLD_HI + slot, hi);
                insertRow(slot, arch, row, px, py, vx, vy);
            }
        } else if (slot >= SLOTS || get(ARCH + slot) == 0 || get(GEN + slot) != generation) status = 1;
        else if (op == 1) {
            removeRow(get(ROW + slot)); put(ARCH + slot, 0); put(RETIRED + slot, epoch);
        } else if (op == 2) {
            uint source = get(ROW + slot);
            if (get(ARCH + slot) == arch) {
                if (arch == 2) { put(VX + source, vx); put(VY + source, vy); }
            } else {
                uint target = destination(arch);
                if (target == 0xffffffff) status = 4;
                else {
                    px = get(PX + source); py = get(PY + source);
                    removeRow(source); insertRow(slot, arch, target, px, py, vx, vy);
                }
            }
        }
        results.Store(n * 12, status);
        results.Store(n * 12 + 4, slot); results.Store(n * 12 + 8, generation);
    }
    reclaim(epoch);
}
[numthreads(64, 1, 1)]
void integrate(uint3 tid : SV_DispatchThreadID) {
    uint row = tid.x + 64;
    uint chunk = row / 32;
    if (row < ROWS && row % 32 < get(COUNT + chunk)) {
        precise float x = readFloat(PX + row) + readFloat(VX + row) * parameters.dt;
        precise float y = readFloat(PY + row) + readFloat(VY + row) * parameters.dt;
        writeFloat(PX + row, x); writeFloat(PY + row, y);
    }
}
[numthreads(64, 1, 1)]
void readRetained(uint3 tid : SV_DispatchThreadID) {
    if (tid.x < WORDS) readerOutput.Store(tid.x * 4, retained.Load(tid.x * 4));
}
