"""Independent topology/terrain invariants for the bounded static benchmark world."""
from collections import Counter
import math
import struct
import unittest
from cook_benchmark_scene import geometry, texture, png


class SceneTests(unittest.TestCase):
    def test_ground_tiles_form_one_closed_edge_matched_rectangle(self):
        parts, counts = geometry()
        edges = Counter()
        vertices = {}
        area = 0
        for part in parts:
            primitive = part[0]
            positions = primitive['positions']
            for i in range(0, len(primitive['indices']), 3):
                a, b, c = (positions[v] for v in primitive['indices'][i:i + 3])
                cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
                self.assertGreater(cross, 0)
                area += cross / 2
                for p, q in ((a, b), (b, c), (c, a)):
                    edges[tuple(sorted((p, q)))] += 1
                for p in (a, b, c):
                    key = p[:2]
                    encoded = struct.pack('<f', p[2])
                    if key in vertices:
                        self.assertEqual(vertices[key], encoded)
                    vertices[key] = encoded
        self.assertEqual(area, 128 * 128)
        self.assertEqual(len(vertices), 17 * 17)
        boundary = 0
        for (a, b), count in edges.items():
            outside = any(a[axis] == b[axis] and abs(a[axis]) == 64 for axis in (0, 1))
            self.assertEqual(count, 1 if outside else 2)
            boundary += int(outside)
        self.assertEqual(boundary, 64)
        self.assertEqual(sum(v['ground'] for v in counts), 256)

    def test_population_bounds_finite_nondegenerate_and_material_triangle_counts(self):
        parts, counts = geometry()
        totals = [0] * 4
        for primitives, count in zip(parts, counts):
            self.assertEqual(count['buildings'], 16)
            self.assertEqual(count['ground'], 64)
            self.assertLessEqual(sum(len(p['positions']) for p in primitives), 8192)
            self.assertLessEqual(sum(len(p['indices']) for p in primitives), 6144)
            for material, primitive in enumerate(primitives):
                positions, uvs, indices = (primitive[k] for k in ('positions', 'uvs', 'indices'))
                self.assertEqual(len(positions), len(uvs))
                self.assertTrue(all(math.isfinite(v) for p in positions for v in p))
                self.assertTrue(all(abs(v) <= 64 for uv in uvs for v in uv))
                self.assertTrue(all(-64 <= p[0] <= 64 and -64 <= p[1] <= 64 and -2 < p[2] < 11 for p in positions))
                for i in range(0, len(indices), 3):
                    a, b, c = (positions[v] for v in indices[i:i + 3])
                    ab, ac = [b[j] - a[j] for j in range(3)], [c[j] - a[j] for j in range(3)]
                    cross = [ab[(j + 1) % 3] * ac[(j + 2) % 3] - ab[(j + 2) % 3] * ac[(j + 1) % 3] for j in range(3)]
                    self.assertGreater(sum(v * v for v in cross), 0)
                totals[material] += len(indices) // 3
        self.assertEqual(totals, [512, 768, 256, 512])
        self.assertEqual(sum(v['trees'] for v in counts), 128)
        self.assertEqual(sum(totals), 2048)

    def test_textures_are_distinct_opaque_spatial_patterns_with_bounded_pngs(self):
        textures = [texture(i) for i in range(4)]
        self.assertEqual(len(set(textures)), 4)
        for rgba in textures:
            self.assertEqual(len(rgba), 64 * 64 * 4)
            self.assertEqual(set(rgba[3::4]), {255})
            self.assertNotEqual(rgba[:4], rgba[8 * 4:9 * 4])
            encoded = png(rgba)
            self.assertEqual(struct.unpack_from('>2I', encoded, 16), (64, 64))
            self.assertLess(len(encoded), 4 * 1024 * 1024)


if __name__ == '__main__':
    unittest.main()
