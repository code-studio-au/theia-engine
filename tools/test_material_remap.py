import unittest
from remap_materials import remap


class MaterialRemapTests(unittest.TestCase):
    def test_reordering_and_cyclic_rotation_preserve_source_assignment(self):
        source = [(0,1,2),(3,4,5),(0,2,6)]
        self.assertEqual(remap(source,[2,0,1],[(4,5,3),(2,6,0),(2,0,1)]),[0,1,2])

    def test_duplicate_coverage_with_same_material_is_preserved(self):
        self.assertEqual(remap([(0,1,2),(1,2,0)],[1,1],[(2,0,1),(0,1,2)]),[1,1])

    def test_conflicting_materials_on_identical_indexed_triangle_fail(self):
        with self.assertRaises(ValueError):
            remap([(0,1,2),(1,2,0)],[0,1],[(2,0,1),(0,1,2)])

    def test_changed_winding_coverage_counts_and_invalid_assignments_fail(self):
        for materials, cooked in [([0],[(0,2,1)]),([0],[(0,1,3)]),([0],[]),([],[(0,1,2)]),([True],[(0,1,2)]),([-1],[(0,1,2)])]:
            with self.assertRaises(ValueError):
                remap([(0,1,2)],materials,cooked)


if __name__ == '__main__':
    unittest.main()
