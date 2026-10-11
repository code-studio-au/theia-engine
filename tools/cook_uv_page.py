"""Cook dense finite float2 TEXCOORD_0 values bound to exact THEIAMES geometry."""
import hashlib
import json
import math
from pathlib import Path
import struct


def cook(mesh, source):
    if len(mesh) < 76 or mesh[:8] != b'THEIAMES' or struct.unpack_from('<I',mesh,8)[0] != 1:
        raise ValueError('UV cooking requires THEIAMES v1')
    geometry = hashlib.sha256(mesh).hexdigest()
    count = struct.unpack_from('<I',mesh,16)[0]
    if not 1 <= count <= 65536 or struct.unpack_from('<I',mesh,40)[0] != len(mesh)-76 or hashlib.sha256(mesh[76:]).digest() != mesh[44:76]:
        raise ValueError('Invalid UV geometry count/digest/length')
    document = json.loads(source)
    if not isinstance(document,dict) or type(document.get('schema')) is not int or document['schema'] != 1 or document.get('geometry_sha256') != geometry:
        raise ValueError('UV assignments require schema 1 and exact geometry SHA-256')
    values = document.get('texcoords')
    if not isinstance(values,list) or len(values) != count:
        raise ValueError('UV count must match geometry vertex stream')
    for value in values:
        if not isinstance(value,list) or len(value)!=2 or any(type(v) not in (int,float) or not math.isfinite(v) or abs(v)>3.4028234663852886e38 for v in value):
            raise ValueError('UV values must be finite float2')
    provenance = json.dumps({'producer':'theia-uv-v1','source_sha256':hashlib.sha256(source).hexdigest(),
                            'cooker_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                            'coordinates':'source TEXCOORD_0 unchanged; dense global vertex order'},sort_keys=True,separators=(',',':')).encode()
    payload = provenance+bytes((-len(provenance))%4)+b''.join(struct.pack('<2f',*value) for value in values)
    return struct.pack('<8s4I32s32s',b'THEIAUVS',1,len(provenance),count,len(payload),bytes.fromhex(geometry),hashlib.sha256(payload).digest())+payload
