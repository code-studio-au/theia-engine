"""Exact rational vertical error over bounded continuous rectangular heightfields.

Input IEEE floats are treated as exact represented values. This does not bound
3D Hausdorff distance, silhouette, normal/material error, or runtime arithmetic.
"""
from fractions import Fraction
import math

MAX_TRIANGLES = 2048
MAX_VERTICES = 2048


def cross(a, b, p):
    return (b[0]-a[0])*(p[1]-a[1])-(b[1]-a[1])*(p[0]-a[0])


def intersection(a, b):
    polygon = [p[:2] for p in a]
    for edge in range(3):
        x, y = b[edge], b[(edge+1) % 3]
        output = []
        if not polygon:
            break
        previous = polygon[-1]
        pd = cross(x, y, previous)
        for point in polygon:
            d = cross(x, y, point)
            if (pd >= 0) != (d >= 0):
                t = pd/(pd-d)
                output.append(tuple(previous[i]+t*(point[i]-previous[i]) for i in range(2)))
            if d >= 0:
                output.append(point)
            previous, pd = point, d
        polygon = output
    return list(dict.fromkeys(polygon))


def area(polygon):
    return sum((a[0]*b[1]-a[1]*b[0] for a, b in
                zip(polygon, polygon[1:]+polygon[:1])), Fraction()) / 2


def height(triangle, point):
    a, b, c = triangle
    denominator = cross(a, b, c)
    v = cross(a, point, c)/denominator
    w = cross(a, b, point)/denominator
    return (1-v-w)*a[2]+v*b[2]+w*c[2]


def overlaps(a, b):
    # Bounds are original represented coordinates, not computed intersections.
    return not (a[2] < b[0] or b[2] < a[0] or a[3] < b[1] or b[3] < a[1])


def triangles(positions, indices):
    if not indices or len(indices) % 3 or len(indices)//3 > MAX_TRIANGLES:
        raise ValueError('Continuous heightfield triangle budget/count invalid')
    if any(type(i) is not int or not 0 <= i < len(positions) for i in indices):
        raise ValueError('Continuous heightfield index invalid')
    result = []
    for offset in range(0, len(indices), 3):
        points = tuple(positions[i] for i in indices[offset:offset+3])
        if cross(*points) <= 0:
            raise ValueError('Continuous heightfield degenerates or reverses XY winding')
        bounds = tuple(float(f(points[i][axis] for i in range(3)))
                       for f, axis in [(min, 0), (min, 1), (max, 0), (max, 1)])
        result.append((points, bounds))
    return result


def validate_surface(surface, rectangle_area):
    if sum((cross(*p)/2 for p, _ in surface), Fraction()) != rectangle_area:
        raise ValueError('Continuous heightfield projected area differs from rectangle')
    # Area equality alone cannot rule out compensated holes/overlaps. Disallow
    # overlapping interiors and require identical heights on all shared boundaries.
    for i, (a, abounds) in enumerate(surface):
        for b, bbounds in surface[i+1:]:
            if not overlaps(abounds, bbounds):
                continue
            polygon = intersection(a, b)
            if area(polygon) > 0:
                raise ValueError('Continuous heightfield has overlapping triangle interiors')
            if any(height(a, p) != height(b, p) for p in polygon):
                raise ValueError('Continuous heightfield shared boundaries disagree')


def continuous_error(positions, original, simplified, maximum_error):
    if type(maximum_error) not in (int, float) or not math.isfinite(maximum_error) or maximum_error < 0:
        raise ValueError('Continuous heightfield error limit invalid')
    if not 3 <= len(positions) <= MAX_VERTICES:
        raise ValueError('Continuous heightfield vertex budget invalid')
    if any(len(p) != 3 or any(type(v) not in (int, float) or not math.isfinite(v)
                             for v in p) for p in positions):
        raise ValueError('Continuous heightfield position invalid')
    exact = [tuple(Fraction(float(v)) for v in p) for p in positions]
    width = max(p[0] for p in exact)-min(p[0] for p in exact)
    height_range = max(p[1] for p in exact)-min(p[1] for p in exact)
    if width <= 0 or height_range <= 0:
        raise ValueError('Continuous heightfield rectangle degenerates')
    fine, coarse = triangles(exact, original), triangles(exact, simplified)
    validate_surface(fine, width*height_range)
    validate_surface(coarse, width*height_range)
    maximum = Fraction()
    points = pairs = 0
    for a, abounds in fine:
        for b, bbounds in coarse:
            if not overlaps(abounds, bbounds):
                continue
            polygon = intersection(a, b)
            if not polygon:
                continue
            pairs += 1
            points += len(polygon)
            maximum = max(maximum, max(abs(height(a, p)-height(b, p)) for p in polygon))
    if maximum > Fraction(float(maximum_error)):
        raise ValueError(f'Continuous vertical error exceeds {maximum_error}')
    rounded = float(maximum)
    upper = math.nextafter(rounded, math.inf) if maximum else 0.0
    if not math.isfinite(upper):
        raise ValueError('Continuous vertical error cannot be represented as finite upper bound')
    return {'continuous_vertical_error_upper_bound': upper,
            'continuous_vertical_error_numerator': str(maximum.numerator),
            'continuous_vertical_error_denominator': str(maximum.denominator),
            'continuous_overlay_pairs': pairs, 'continuous_overlay_vertices': points,
            'continuous_profile': 'exact represented positions; continuous rectangular XY heightfields; vertical error only'}
