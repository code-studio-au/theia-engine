"""Check readiness window semantics, including missing and preexisting detail."""
import unittest

from streaming_comparison import episodes, selections


class ReadinessTests(unittest.TestCase):
    def test_unresolved_windows_are_null_and_reentry_starts_a_new_window(self):
        trace=[]
        for frame,(wanted,selected) in enumerate([([0,1],[]),([0,1],[0]),([],[]),([0],[0]),([0],[])]):
            trace.append(dict(frame=frame,warmup=frame<2,near_demand=wanted,
                              bevy_selected=selected,bevy_elapsed_ms=frame*20.))
        windows=episodes(trace,'bevy')
        self.assertEqual(len(windows),3)
        self.assertEqual((windows[0]['observed_delay_frames'],windows[0]['observed_delay_ms']),(1,20.))
        self.assertEqual(windows[0]['end_frame_exclusive'],2)
        self.assertIsNone(windows[1]['first_fine_frame'])
        self.assertIsNone(windows[1]['observed_delay_ms'])
        self.assertEqual(windows[2]['observed_delay_frames'],0)
        self.assertEqual(windows[2]['end_frame_exclusive'],5)
        self.assertTrue(windows[0]['warmup'])
        self.assertFalse(windows[2]['warmup'])

    def test_missing_coverage_and_foreign_region_handles_reject(self):
        self.assertEqual(selections([0,5,2,7]),{0,2})
        for invalid in ([0,1,2],[4,4,6,7],[4,5,6,8]):
            with self.assertRaises(RuntimeError):selections(invalid)


if __name__=='__main__':unittest.main()
