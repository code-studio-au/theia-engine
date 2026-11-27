"""Check independent source-index remapping and fail-closed table production."""
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from cook_texture_table import cook


class TextureTableTests(unittest.TestCase):
    def fixture(self,directory):
        payload=b'mat\0'+struct.pack('<12f',*([1.]*12))+struct.pack('<3I',0,1,0)
        materials=struct.pack('<8s6I32s32s',b'THEIAMAT',1,3,3,1,1,len(payload),bytes([42])*32,hashlib.sha256(payload).digest())+payload
        raw=(Path(__file__).parent.parent/'fixtures/textures/opaque_2x2.ktx2').read_bytes()
        paths=[]
        for value in (128,240):
            data=raw[:-16]+bytes([value])+raw[-15:]
            path=directory/(hashlib.sha256(data).hexdigest()+'.ktx2');path.write_bytes(data);paths.append(str(path))
        source={'schema':1,'material_sha256':hashlib.sha256(materials).hexdigest(),'textures':paths,'material_textures':[1,0,1]}
        return materials,source

    def body(self,encoded):
        meta=struct.unpack_from('<I',encoded,12)[0];start=88+((meta+3)&~3)
        return encoded[start:]

    def test_texture_source_order_remaps_to_same_canonical_physical_material_choices(self):
        with tempfile.TemporaryDirectory() as directory:
            materials,source=self.fixture(Path(directory))
            first=cook(materials,json.dumps(source).encode())
            source['textures'].reverse();source['material_textures']=[0,1,0]
            second=cook(materials,json.dumps(source).encode())
            self.assertEqual(self.body(first),self.body(second))
            # Full content ID includes the different source provenance by design.
            self.assertNotEqual(first,second)
            self.assertEqual(first,cook(materials,json.dumps(dict(source,textures=list(reversed(source['textures'])),material_textures=[1,0,1])).encode()))

    def test_mapping_binding_duplicates_and_missing_children_fail_before_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            materials,source=self.fixture(Path(directory))
            for key,value in [('schema',True),('material_sha256','0'*64),('material_textures',[0,0,0]),
                               ('material_textures',[0,1,2]),('material_textures',[False,1,0]),('material_textures',[0]),
                               ('textures',[]),('textures',[source['textures'][0]]*2)]:
                with self.subTest(key=key,value=value),self.assertRaises(ValueError):cook(materials,json.dumps(dict(source,**{key:value})).encode())
            Path(source['textures'][0]).unlink()
            with self.assertRaises(OSError):cook(materials,json.dumps(source).encode())


if __name__=='__main__':unittest.main()
