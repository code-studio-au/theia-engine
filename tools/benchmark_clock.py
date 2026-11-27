"""Independently check optional benchmark deadlines; report misses without hiding them."""
import math

from world_camera_reference import require


def verify(report, hz):
    require(report['metadata']['frame_pacing']['target_hz'] == hz, 'Clock target differs')
    samples = report['samples']
    require(len(samples) == report['frames'] and samples, 'Incomplete clock trace')
    late = []
    previous = -1.
    for frame, sample in enumerate(samples):
        require(sample['frame'] == frame, 'Clock skipped a logical frame')
        point = sample['frame_pacing']
        require(all(isinstance(point[k], (float, int)) and math.isfinite(point[k]) and point[k] >= 0
                    for k in ('scheduled_ms', 'elapsed_ms', 'lateness_ms', 'wait_ms')), 'Invalid clock sample')
        expected = (frame * 1_000_000_000 // hz) / 1e6
        require(abs(point['scheduled_ms'] - expected) < 1e-6, 'Clock was rebased or has drifted')
        require(point['elapsed_ms'] >= previous and point['elapsed_ms'] >= expected, 'Clock ran early/backwards')
        require(abs(point['elapsed_ms'] - expected - point['lateness_ms']) < 1e-6, 'Clock lateness differs')
        require(point['wait_ms'] <= point['elapsed_ms'] + 1e-6, 'Wait exceeds elapsed time')
        previous = point['elapsed_ms']
        late.append(point['lateness_ms'])
    ordered = sorted(late)
    return dict(frames=len(samples), target_hz=hz, mean_lateness_ms=sum(late)/len(late),
                p95_lateness_ms=ordered[math.ceil((len(late)-1)*.95)], max_lateness_ms=max(late),
                frames_at_least_one_period_late=sum(v >= 1000/hz for v in late),
                final_elapsed_ms=previous,
                scope='Recorded frame starts only; process epochs are independent; I/O/publication/presentation completion deadlines are not measured here')
