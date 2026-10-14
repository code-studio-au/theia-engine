"""Independent gates for native box-mip qualification, without requiring KTX."""
import json
from pathlib import Path
import struct
import unittest
import zlib
from cook_filtered_texture import filtered_source, png_envelope, verify_generated


class FilteredTextureTests(unittest.TestCase):
    def chain(self):
        blob = bytearray((Path(__file__).parent.parent / 'fixtures/textures/opaque_4x4_mips.ktx2').read_bytes())
        rgba = bytes(c for y in range(4) for x in range(4)
                     for c in ([255, 255, 255, 255] if (x + y) % 2 else [0, 0, 0, 255]))
        for index in range(3):
            offset, size, _ = struct.unpack_from('<3Q', blob, 80 + index * 24)
            blob[offset:offset + size] = rgba if index == 0 else bytes([188, 188, 188, 255]) * (size // 4)
        return blob, rgba

    def test_linear_light_midpoint_rejects_encoded_space_average(self):
        blob, rgba = self.chain()
        levels = verify_generated(blob, 4, 4, rgba)
        self.assertEqual([m['max_rgb_error'] for m in levels], [0, 0, 0])
        offset = struct.unpack_from('<Q', blob, 104)[0]
        blob[offset] = 128  # Encoded-space black/white average is wrong.
        with self.assertRaisesRegex(ValueError, 'linear-sRGB box'):
            verify_generated(blob, 4, 4, rgba)

    def test_one_byte_rounding_bound_is_explicit_and_larger_errors_reject(self):
        blob, rgba = self.chain()
        offset = struct.unpack_from('<Q', blob, 104)[0]
        blob[offset] = 189
        self.assertEqual(verify_generated(blob, 4, 4, rgba)[1]['max_rgb_error'], 1)
        blob[offset] = 190
        with self.assertRaises(ValueError):
            verify_generated(blob, 4, 4, rgba)

    def test_base_alpha_extent_count_and_byte_ranges_cannot_change(self):
        original, rgba = self.chain()
        for position in [0, 12, 20, 24, 40, 80, 88, 96]:
            blob = bytearray(original)
            blob[position] ^= 0x80
            with self.subTest(position=position), self.assertRaises(ValueError):
                verify_generated(blob, 4, 4, rgba)
        for index in range(3):
            offset = struct.unpack_from('<Q', original, 80 + index * 24)[0]
            blob = bytearray(original)
            blob[offset + 3] = 0
            with self.assertRaises(ValueError):
                verify_generated(blob, 4, 4, rgba)
        blob = bytearray(original)
        base = struct.unpack_from('<Q', blob, 80)[0]
        blob[base] = 1
        with self.assertRaisesRegex(ValueError, 'base source'):
            verify_generated(blob, 4, 4, rgba)
        for end in range(len(original)):
            with self.assertRaises(ValueError):
                verify_generated(original[:end], 4, 4, rgba)

    def test_source_gate_requires_power_of_two_dense_opaque_integer_texels(self):
        for width, height in [(1, 1), (8, 4), (1024, 1), (1, 1024)]:
            source = json.dumps({'schema': 1, 'width': width, 'height': height,
                                 'rgba': [[0, 0, 0, 255]] * (width * height)})
            self.assertEqual(filtered_source(source)[:2], (width, height))
        for width, height, color in [(5, 3, [0, 0, 0, 255]), (2, 2, [0, 0, 0, 254]),
                                     (2, 2, [True, 0, 0, 255]), (0, 1, [0, 0, 0, 255])]:
            with self.assertRaises(ValueError):
                filtered_source(json.dumps({'schema': 1, 'width': width, 'height': height,
                                            'rgba': [color] * (width * height)}))

    def test_png_envelope_has_valid_crc_and_lossless_top_left_rgba_scanlines(self):
        rgba = bytes([0, 1, 2, 255, 3, 4, 5, 255, 6, 7, 8, 255, 9, 10, 11, 255])
        png = png_envelope(2, 2, rgba)
        cursor = 8
        chunks = {}
        while cursor < len(png):
            size = struct.unpack_from('>I', png, cursor)[0]
            tag = png[cursor + 4:cursor + 8]
            data = png[cursor + 8:cursor + 8 + size]
            self.assertEqual(zlib.crc32(tag + data), struct.unpack_from('>I', png, cursor + 8 + size)[0])
            chunks[tag] = data
            cursor += 12 + size
        self.assertEqual(cursor, len(png))
        self.assertEqual(struct.unpack('>2I5B', chunks[b'IHDR']), (2, 2, 8, 6, 0, 0, 0))
        self.assertEqual(chunks[b'sRGB'], b'\0')
        self.assertEqual(zlib.decompress(chunks[b'IDAT']), b'\0' + rgba[:8] + b'\0' + rgba[8:])


if __name__ == '__main__':
    unittest.main()
