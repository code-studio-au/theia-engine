"""Independent coverage and shared-edge checks for mixed fine/coarse quadrants."""
from collections import Counter
import itertools
import struct
import unittest
from cook_benchmark_scene import geometry, fallback_geometry, fallback_ground_error


def triangles(primitive):
    p, uv, indices = (primitive[k] for k in ('positions', 'uvs', 'indices'))
    return Counter(tuple((*p[v], *uv[v]) for v in indices[i:i + 3])
                   for i in range(0, len(indices), 3))


def closed_world(parts):
    edges = Counter();heights = {};area = 0
    for part in parts:
        ground = part[0];positions = ground['positions']
        for offset in range(0, len(ground['indices']), 3):
            a, b, c = [positions[i] for i in ground['indices'][offset:offset + 3]]
            cross = (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
            if cross <= 0:raise ValueError('Degenerate/reversed ground')
            area += cross / 2
            for p in (a,b,c):
                encoded = struct.pack('<f',p[2])
                if heights.setdefault(p[:2],encoded) != encoded:
                    raise ValueError('Shared XY vertex heights differ')
            edges.update(tuple(sorted(edge)) for edge in ((a,b),(b,c),(c,a)))
    if area != 128*128:raise ValueError('Projected ground area differs')
    for (a,b), count in edges.items():
        outer = any(a[axis]==b[axis] and abs(a[axis])==64 for axis in (0,1))
        if count != (1 if outer else 2):raise ValueError('Open, overlapping or unmatched ground edge')
    return len(edges)


class FallbackTests(unittest.TestCase):
    def test_all_sixteen_mixed_worlds_are_closed_and_edge_matched(self):
        fine, _ = geometry();coarse = fallback_geometry()
        for mask in itertools.product((0,1),repeat=4):
            with self.subTest(mask=mask):
                self.assertGreater(closed_world([coarse[i] if level else fine[i] for i,level in enumerate(mask)]),0)
        # A mismatched seam height must be detected, even with equal XY coverage.
        ground = coarse[0][0]
        i = next(i for i,p in enumerate(ground['positions']) if p[:2]==(0,0))
        p = ground['positions'][i];ground['positions'][i]=(p[0],p[1],p[2]+.1)
        with self.assertRaises(ValueError):closed_world([coarse[0],*fine[1:]])

    def test_props_uvs_materials_and_triangle_reduction(self):
        fine, counts = geometry();coarse = fallback_geometry()
        for source, fallback in zip(fine,coarse):
            for material in range(1,4):
                self.assertEqual(triangles(source[material]),triangles(fallback[material]))
            ground = fallback[0]
            self.assertEqual(len(ground['positions']),41)
            self.assertEqual(len(ground['indices'])//3,48)
            self.assertTrue(all(uv==(p[0]/8,p[1]/8) for uv,p in zip(ground['uvs'],ground['positions'])))
        self.assertEqual(sum(len(p['indices'])//3 for part in coarse for p in part),1728)
        self.assertEqual(sum(c['buildings'] for c in counts),64)
        self.assertEqual(sum(c['trees'] for c in counts),128)

    def test_continuous_error_and_missing_triangle_rejection(self):
        fine, _ = geometry();coarse = fallback_geometry()
        for source, fallback in zip(fine,coarse):
            result = fallback_ground_error(source[0],fallback[0])
            self.assertLess(result['continuous_vertical_error_upper_bound'],.12)
        coarse[0][0]['indices'] = coarse[0][0]['indices'][:-3]
        with self.assertRaises(ValueError):fallback_ground_error(fine[0][0],coarse[0][0])


if __name__=='__main__':unittest.main()
