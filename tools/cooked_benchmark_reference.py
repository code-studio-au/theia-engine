"""Pure offline cooked geometry/ray oracle shared by benchmark replay checks."""
from pathlib import Path
import struct


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def decode(cook):
    blobs = [Path(r['path']).read_bytes() for r in [cook, cook['materials'], cook['uv'], cook['texture_table']]]
    mesh, material, uv, _ = blobs
    meta, count, clusters, refs = struct.unpack_from('<4I', mesh, 12)
    offset = 76 + ((meta + 3) & ~3)
    positions = list(struct.iter_unpack('<3f', mesh[offset:offset + count * 12]))
    cluster_offset = offset + count * 12
    refs_offset = cluster_offset + clusters * 16
    triangles_offset = refs_offset + refs * 4
    global_refs = struct.unpack_from(f'<{refs}I', mesh, refs_offset)
    meta, materials = struct.unpack_from('<2I', material, 12)
    assignments_offset = 96 + ((meta + 3) & ~3) + materials * 16 + (clusters + 1) * 4
    meta, count = struct.unpack_from('<2I', uv, 12)
    offset = 88 + ((meta + 3) & ~3)
    texcoords = list(struct.iter_unpack('<2f', uv[offset:offset + count * 8]))
    triangles, hulls = [], []
    for cluster in range(clusters):
        vo, to, vc, tc = struct.unpack_from('<4I', mesh, cluster_offset + cluster * 16)
        hulls.append([positions[global_refs[vo + i]] for i in range(vc)])
        for primitive in range(tc):
            indices = [global_refs[vo + mesh[triangles_offset + to + primitive * 3 + i]] for i in range(3)]
            assignment = struct.unpack_from('<I', material, assignments_offset + len(triangles) * 4)[0]
            triangles.append((cluster + 1, primitive, [positions[i] for i in indices],
                              [texcoords[i] for i in indices], assignment))
    pixels = []
    for record in cook['textures']:
        image = Path(record['path']).read_bytes()
        offset, size = struct.unpack_from('<2Q', image, 80)
        pixels.append(image[offset:offset + size])
    return blobs, triangles, hulls, pixels


def ray(rows, triangles, x, y, width, height):
    basis = [rows[i][:3] for i in (0, 1, 3)]
    eye = [sum(-rows[i][3] * rows[i][a] / dot(rows[i][:3], rows[i][:3]) for i in (0, 1, 3)) for a in range(3)]
    ndc = [(x + .5) / width * 2 - 1, (y + .5) / height * 2 - 1]
    direction = [basis[2][a] + sum(ndc[i] * basis[i][a] / dot(basis[i], basis[i]) for i in (0, 1)) for a in range(3)]
    closest, result = float('inf'), (0, 0, 1., None, None)
    for identity, primitive, (a, b, c), uvs, material in triangles:
        e1, e2 = sub(b, a), sub(c, a)
        p = cross(direction, e2)
        det = dot(e1, p)
        if abs(det) < 1e-12:
            continue
        tv = sub(eye, a)
        u = dot(tv, p) / det
        q = cross(tv, e1)
        v = dot(direction, q) / det
        distance = dot(e2, q) / det
        if not (0 <= u <= 1 and 0 <= v and u + v <= 1 and 0 < distance < closest):
            continue
        point = [eye[i] + direction[i] * distance for i in range(3)]
        clip = [dot(row[:3], point) + row[3] for row in rows]
        depth = clip[2] / clip[3]
        if not 0 <= depth < 1:
            continue
        texcoord = [uvs[0][i] * (1 - u - v) + uvs[1][i] * u + uvs[2][i] * v for i in range(2)]
        closest = distance
        result = (identity, primitive, depth, material, texcoord)
    return result




def material_pixels(cook, pixels):
    """Canonical table hashes map dense materials to source-order decoded pixels."""
    table = Path(cook['texture_table']['path']).read_bytes()
    metadata, materials, images = struct.unpack_from('<3I', table, 12)
    records = 88 + ((metadata + 3) & ~3)
    source = {record['content_id']: value for record, value in zip(cook['textures'], pixels)}
    hashes = [table[records + i * 40:records + i * 40 + 32].hex() for i in range(images)]
    mapping = struct.unpack_from('<%dI' % materials, table, records + images * 40)
    return [source[hashes[index]] for index in mapping]


def material_factors(cook):
    page = Path(cook['materials']['path']).read_bytes()
    metadata, count = struct.unpack_from('<2I', page, 12)
    offset = 96 + ((metadata + 3) & ~3)
    return list(struct.iter_unpack('<4f', page[offset:offset + count * 16]))
