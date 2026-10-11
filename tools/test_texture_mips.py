"""Offline authored mip source and canonical relocation boundary checks."""
import json
from pathlib import Path
import struct
import unittest
from cook_texture_mips import package_mips, source_levels

FIXTURES = Path(__file__).parent.parent / 'fixtures/textures'


class TextureMipCookTests(unittest.TestCase):
    def test_explicit_levels_preserve_authored_texels(self):
        levels = source_levels((FIXTURES / 'opaque_4x4_mips.json').read_bytes())
        self.assertEqual([(w, h, len(p)) for w, h, p in levels], [(4, 4, 64), (2, 2, 16), (1, 1, 4)])
        for index, (w, h, p) in enumerate(levels):
            self.assertEqual(p, bytes([10 + index * 70, 50, 120, 255]) * w * h)

    def test_count_dimensions_types_and_opaque_tail_fail_closed(self):
        valid = json.loads((FIXTURES / 'opaque_4x4_mips.json').read_bytes())
        for key, value in [('schema', True), ('levels', []), ('levels', valid['levels'][:2]),
                           ('levels', valid['levels'] + [valid['levels'][-1]]), ('levels', [None])]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                source_levels(json.dumps(dict(valid, **{key: value})))
        for index in range(3):
            for key, value in [('width', True), ('height', 1025), ('width', 3), ('rgba', [])]:
                bad = json.loads(json.dumps(valid)); bad['levels'][index][key] = value
                with self.subTest(index=index, key=key), self.assertRaises(ValueError):
                    source_levels(json.dumps(bad))
            bad = json.loads(json.dumps(valid)); bad['levels'][index]['rgba'][0][3] = 254
            with self.assertRaises(ValueError): source_levels(json.dumps(bad))

    def test_all_indices_relocate_preserving_dfd_payload_and_order(self):
        native = (FIXTURES / 'opaque_4x4_mips.ktx2').read_bytes()
        cooked = package_mips(native, {'producer': 'test', 'source': 'authored'})
        self.assertEqual(cooked[152:244], native[152:244])
        for index in range(3):
            old, size, unpacked = struct.unpack_from('<3Q', native, 80 + index * 24)
            new, got_size, got_unpacked = struct.unpack_from('<3Q', cooked, 80 + index * 24)
            self.assertEqual((got_size, got_unpacked), (size, unpacked))
            self.assertEqual(cooked[new:new + size], native[old:old + size])
        self.assertEqual(cooked, package_mips(native, {'source': 'authored', 'producer': 'test'}))
        for offset in [0, 20, 24, 40, 48, 52, 56, 60, 80, 88, 96, 104, 112, 120, 128, 136, 144]:
            bad = bytearray(native); bad[offset] ^= 0x80
            with self.subTest(offset=offset), self.assertRaises(ValueError): package_mips(bytes(bad), {})
        with self.assertRaises(ValueError): package_mips(native, {'oversized': 'x' * 16384})
        with self.assertRaises(ValueError): package_mips(native + b'\0', {})


if __name__ == '__main__':
    unittest.main()
