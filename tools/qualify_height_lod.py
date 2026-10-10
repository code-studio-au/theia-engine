"""Independent XY heightfield checks; sampled error is not a Hausdorff bound."""
from collections import Counter
import math


def topology(positions, indices):
    if not indices or len(indices) % 3 or any(i < 0 or i >= len(positions) for i in indices):
        raise ValueError('Invalid triangle index list')
    edges = Counter()
    triangles = []
    area = 0.0
    for offset in range(0, len(indices), 3):
        ids = indices[offset:offset + 3]
        a, b, c = (positions[i] for i in ids)
        twice = (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
        if not math.isfinite(twice) or twice <= 1e-12:
            raise ValueError('Heightfield triangle is degenerate or reverses XY winding')
        area += twice * .5
        triangles.append((a, b, c, twice))
        for x, y in zip(ids, ids[1:] + ids[:1]):
            edges[tuple(sorted((x, y)))] += 1
    if any(count > 2 for count in edges.values()):
        raise ValueError('Nonmanifold heightfield edge')
    return {edge for edge, count in edges.items() if count == 1}, triangles, area


def height(point, triangles):
    x, y = point
    matches = []
    for a, b, c, denominator in triangles:
        v = ((x-a[0])*(c[1]-a[1])-(y-a[1])*(c[0]-a[0])) / denominator
        w = ((b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0])) / denominator
        u = 1-v-w
        if min(u, v, w) >= -1e-7:
            matches.append(u*a[2]+v*b[2]+w*c[2])
    if not matches:
        raise ValueError('Heightfield sample falls in a coverage hole')
    if max(matches)-min(matches) > 1e-6:
        raise ValueError('Overlapping heightfield surfaces disagree')
    return matches[0]


def qualify(positions, original, simplified, maximum_height_error):
    if not math.isfinite(maximum_height_error) or maximum_height_error < 0:
        raise ValueError('Invalid sampled height error limit')
    if any(not math.isfinite(value) for position in positions for value in position):
        raise ValueError('Nonfinite heightfield position')
    boundary, fine, area = topology(positions, original)
    coarse_boundary, coarse, coarse_area = topology(positions, simplified)
    if coarse_boundary != boundary:
        raise ValueError('Simplification changed an exact boundary edge')
    if not math.isclose(area, coarse_area, rel_tol=0, abs_tol=1e-6):
        raise ValueError('Simplification changed projected coverage area')
    # Vertices + both sets of centroids + a 65x65 interior/boundary grid. Coverage
    # is checked in both directions; no unsampled maximum-error guarantee is made.
    points = {(p[0], p[1]) for p in positions}
    for triangles in (fine, coarse):
        points.update(((a[0]+b[0]+c[0])/3, (a[1]+b[1]+c[1])/3) for a,b,c,_ in triangles)
    minimum = [min(p[axis] for p in positions) for axis in range(2)]
    maximum = [max(p[axis] for p in positions) for axis in range(2)]
    points.update((minimum[0]+(maximum[0]-minimum[0])*x/64,
                   minimum[1]+(maximum[1]-minimum[1])*y/64) for y in range(65) for x in range(65))
    error = max(abs(height(point, fine)-height(point, coarse)) for point in points)
    if error > maximum_height_error + 1e-7:
        raise ValueError(f'Sampled height error {error} exceeds {maximum_height_error}')
    return {'boundary_edges': len(boundary), 'exact_boundary_preserved': True,
            'projected_area': area, 'sample_count': len(points), 'sampled_max_height_error': error,
            'sampled_height_error_limit': maximum_height_error,
            'original_triangles': len(original)//3, 'simplified_triangles': len(simplified)//3}
