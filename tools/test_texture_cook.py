"""Adversarial source bounds and KTX metadata packaging checks (offline only)."""
import json
import struct
import unittest
from pathlib import Path
from cook_texture import package, pixels


class TextureCookTests(unittest.TestCase):
    def test_source_dimensions_counts_channels_and_alpha_fail_closed(self):
        valid = {'schema': 1, 'width': 1, 'height': 1, 'rgba': [[1, 2, 3, 255]]}
        self.assertEqual(pixels(json.dumps(valid)), (1, 1, bytes([1, 2, 3, 255])))
        for key, value in [('schema', True), ('width', True), ('width', 0), ('height', 1025),
                           ('rgba', []), ('rgba', [[1, 2, 3]]), ('rgba', [[1, 2, 3, 254]]),
                           ('rgba', [[True, 2, 3, 255]]), ('rgba', [[256, 2, 3, 255]])]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                pixels(json.dumps(dict(valid, **{key: value})))

    def test_metadata_relocation_preserves_native_dfd_and_exact_pixels(self):
        native = (Path(__file__).parent.parent/'fixtures/textures/opaque_2x2.ktx2').read_bytes()
        cooked = package(native, {'producer': 'test', 'source': 'synthetic'})
        self.assertEqual(cooked[104:196], native[104:196])
        offset, size, unpacked = struct.unpack_from('<3Q', cooked, 80)
        self.assertEqual(cooked[offset:], native[268:])
        self.assertEqual(size, unpacked)
        self.assertEqual(offset, 196+struct.unpack_from('<I', cooked, 60)[0])
        self.assertEqual(cooked, package(native, {'source': 'synthetic', 'producer': 'test'}))
        for index in (0, 48, 52, 56, 60, 80, 88, 96):
            bad = bytearray(native); bad[index] ^= 0x80
            with self.subTest(index=index), self.assertRaises(ValueError):
                package(bytes(bad), {})
        with self.assertRaises(ValueError):
            package(native, {'oversized': 'x'*16384})


if __name__ == '__main__':
    unittest.main()
