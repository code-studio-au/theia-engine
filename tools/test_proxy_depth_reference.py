"""Frozen two-proxy reproduction of the observed adjacent-D32 raster overlap."""
import struct
import unittest
from world_crowd_reference import adjacent_depth_hit
from proxy_benchmark_reference import pixel_reference

ROWS=[[.8829480409622192,0.,0.,0.],
      [0.,-.40444931387901306,-1.516684889793396,3.0333690643310547],
      [0.,.9666215777397156,-.2577657699584961,39.24146270751953],
      [0.,.9662349224090576,-.25766265392303467,39.32576370239258]]
EXTENT=(.30000001192092896,.30000001192092896,.800000011920929)
OBJECTS={5043:(5043,(2.4012908935546875,39.72005844116211,.8520994186401367),EXTENT,False),
         5090:(5090,(2.4278764724731445,39.792789459228516,.852676272392273),EXTENT,False)}
FLAG=0x80000000


def depth(code):return struct.unpack('<f',struct.pack('<I',code))[0]


class AdjacentDepthTests(unittest.TestCase):
    def setUp(self):
        self.proxies=dict(objects=OBJECTS,visible=set(OBJECTS))
        self.nearest=pixel_reference(ROWS,list(OBJECTS.values()),493,215,960,540)
        self.expected=(self.nearest[0]|FLAG,self.nearest[1],self.nearest[2])
        self.actual=(5044|FLAG,3)
    def check_hit(self,actual=None,stored=None,expected=None):
        return adjacent_depth_hit(ROWS,self.proxies,self.actual if actual is None else actual,
            depth(1065338291) if stored is None else stored,self.expected if expected is None else expected,493,215,960,540)
    def test_actual_source_hit_at_adjacent_d32_matches_the_recorded_depth(self):
        self.assertEqual(self.expected[:2],(5091|FLAG,3))
        hit=self.check_hit();self.assertIsNotNone(hit)
        self.assertEqual((hit['nearest_depth_code'],hit['actual_source_depth_code'],hit['stored_depth_code']),
                         (1065338290,1065338291,1065338291))
    def test_rejects_wrong_primitive_foreign_identity_or_unmatched_stored_depth(self):
        self.assertIsNone(self.check_hit(actual=(5044|FLAG,2)))
        self.assertIsNone(self.check_hit(actual=(900000|FLAG,3)))
        self.assertIsNone(self.check_hit(stored=depth(1065338292)))
        self.assertIsNone(self.check_hit(actual=(5044,3)))
    def test_rejects_a_two_code_depth_gap_and_a_pixel_without_that_surface(self):
        self.assertIsNone(self.check_hit(expected=(5091|FLAG,3,depth(1065338289))))
        self.assertIsNone(adjacent_depth_hit(ROWS,self.proxies,self.actual,depth(1065338291),self.expected,0,0,960,540))


if __name__=='__main__':unittest.main()
