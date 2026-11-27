"""Independent analytic extrema and malformed surfaces for continuous LOD error."""
import random
import unittest
from fractions import Fraction
from continuous_height_error import continuous_error


class ContinuousHeightErrorTests(unittest.TestCase):
    fine = [0, 1, 2, 0, 2, 3]
    coarse = [0, 1, 3, 1, 2, 3]

    def test_overlay_crossing_is_an_extremum_not_an_input_vertex(self):
        positions = [(0., 0., 0.), (1., 0., 0.), (1., 1., 1.), (0., 1., 0.)]
        result = continuous_error(positions, self.fine, self.coarse, .5)
        self.assertEqual(Fraction(int(result['continuous_vertical_error_numerator']),
                                  int(result['continuous_vertical_error_denominator'])), Fraction(1, 2))
        self.assertGreaterEqual(result['continuous_vertical_error_upper_bound'], .5)
        with self.assertRaisesRegex(ValueError, 'Continuous vertical error'):
            continuous_error(positions, self.fine, self.coarse, .499)

    def test_random_diagonal_changes_match_independent_analytic_center_formula(self):
        rng = random.Random(719)
        for _ in range(100):
            z = [rng.randrange(-20, 21)/4 for _ in range(4)]
            positions = [(0., 0., z[0]), (2., 0., z[1]), (2., 3., z[2]), (0., 3., z[3])]
            expected = abs(Fraction(z[0])+Fraction(z[2])-Fraction(z[1])-Fraction(z[3]))/2
            result = continuous_error(positions, self.fine, self.coarse, float(expected))
            actual = Fraction(int(result['continuous_vertical_error_numerator']),
                              int(result['continuous_vertical_error_denominator']))
            self.assertEqual(actual, expected)
            self.assertGreaterEqual(result['continuous_vertical_error_upper_bound'], expected)

    def test_identity_flat_and_center_peak(self):
        positions = [(0., 0., 0.), (1., 0., 0.), (1., 1., 0.), (0., 1., 0.), (.5, .5, .125)]
        peak = [0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4]
        result = continuous_error(positions, peak, self.fine, .125)
        self.assertEqual(result['continuous_vertical_error_numerator'], '1')
        self.assertEqual(result['continuous_vertical_error_denominator'], '8')
        self.assertEqual(continuous_error(positions, peak, peak, 0.)['continuous_vertical_error_upper_bound'], 0.)

    def test_compensated_holes_overlap_and_height_discontinuity_fail(self):
        p = [(0., 0., 0.), (1., 0., 0.), (1., 1., 0.), (0., 1., 0.)]
        # Two copies of half the rectangle have correct summed area but a hole.
        with self.assertRaisesRegex(ValueError, 'overlapping triangle interiors'):
            continuous_error(p, [0, 1, 2, 0, 1, 2], self.fine, 1.)
        split = p + [(0., 0., 1.), (1., 1., 1.)]
        with self.assertRaisesRegex(ValueError, 'shared boundaries disagree'):
            continuous_error(split, [0, 1, 2, 4, 5, 3], self.fine, 1.)
        with self.assertRaisesRegex(ValueError, 'projected area'):
            continuous_error(p, [0, 1, 2], self.fine, 1.)

    def test_invalid_profile_limits_and_positions_fail_closed(self):
        p = [(0., 0., 0.), (1., 0., 0.), (1., 1., 0.), (0., 1., 0.)]
        for bad in ([0, 2, 1], [0, 1, 99], [0, True, 2], [], self.fine*1025):
            with self.assertRaises(ValueError): continuous_error(p, bad, self.coarse, 1.)
        for limit in (-1., float('nan'), float('inf')):
            with self.assertRaises(ValueError): continuous_error(p, self.fine, self.coarse, limit)
        for bad in ([*p, (float('nan'), 0., 0.)], [(0., 0., 0.)]*4, p*513):
            with self.assertRaises(ValueError): continuous_error(bad, self.fine, self.coarse, 1.)


if __name__ == '__main__':
    unittest.main()
