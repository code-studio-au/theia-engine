"""Analytic and deliberately invalid inputs for the independent LOD checker."""
import unittest
from qualify_height_lod import qualify, height, topology


class HeightLodTests(unittest.TestCase):
    positions = [(0.,0.,0.),(1.,0.,0.),(1.,1.,0.),(0.,1.,0.),(.5,.5,.1)]
    fine = [0,1,4,1,2,4,2,3,4,3,0,4]
    coarse = [0,1,2,0,2,3]

    def test_analytic_center_peak_error_and_boundary(self):
        result = qualify(self.positions,self.fine,self.coarse,.11)
        self.assertAlmostEqual(result['sampled_max_height_error'],.1)
        self.assertEqual(result['boundary_edges'],4)
        self.assertTrue(result['exact_boundary_preserved'])
        with self.assertRaisesRegex(ValueError,'Sampled height error'):
            qualify(self.positions,self.fine,self.coarse,.05)

    def test_boundary_winding_and_nonmanifold_rejections(self):
        for triangles in ([0,1,2],[0,2,1,0,2,3],[0,1,2]*3):
            with self.assertRaises(ValueError):
                qualify(self.positions,self.fine,triangles,.11)

    def test_coverage_holes_and_overlapping_heights(self):
        _, triangles, _ = topology(self.positions,[0,1,4])
        with self.assertRaisesRegex(ValueError,'coverage hole'):
            height((.5,.9),triangles)
        shifted = [(x,y,z+1) for x,y,z in self.positions]
        _, other, _ = topology(shifted,[0,1,4])
        with self.assertRaisesRegex(ValueError,'Overlapping'):
            height((.5,.2),triangles+other)

    def test_nonfinite_and_invalid_limits(self):
        for limit in (float('nan'),float('inf'),-.1):
            with self.assertRaises(ValueError):
                qualify(self.positions,self.fine,self.coarse,limit)
        invalid = list(self.positions)
        invalid[0] = (float('nan'),0,0)
        with self.assertRaisesRegex(ValueError,'Nonfinite'):
            qualify(invalid,self.fine,self.coarse,.11)


if __name__ == '__main__':
    unittest.main()
