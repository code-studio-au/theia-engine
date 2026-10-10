"""Map material assignments through winding-preserving native triangle reordering."""
from collections import Counter


def canonical(triangle):
    return min(triangle, triangle[1:] + triangle[:1], triangle[2:] + triangle[:2])


def remap(source, materials, cooked):
    if len(source) != len(materials) or any(type(value) is not int or value < 0 for value in materials):
        raise ValueError('Source triangle material count/type mismatch')
    if any(len(triangle) != 3 for triangle in source + cooked):
        raise ValueError('Triangle remapping requires three indices')
    if Counter(map(canonical, source)) != Counter(map(canonical, cooked)):
        raise ValueError('Material remapping changed triangle coverage or winding')
    mapping = {}
    for triangle, material in zip(source, materials):
        key = canonical(triangle)
        if key in mapping and mapping[key] != material:
            raise ValueError('Coincident indexed triangles have ambiguous different materials')
        mapping[key] = material
    return [mapping[canonical(triangle)] for triangle in cooked]
