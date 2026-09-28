#!/usr/bin/env python3
"""Tests for tools/ccs.py and tools/ccsmodel.py against the retail .hack//Infection
disc. Archive tests are skipped when work/infection/disc/DATA/DATA.BIN or
work/infection/infection.iso is absent.
Run: python3 tools/test_ccs.py"""

import os
import unittest

import ccs
import ccsmodel

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "work", "infection")
DATA = os.path.join(ROOT, "disc", "DATA", "DATA.BIN")
ISO = os.path.join(ROOT, "infection.iso")


def stream(name):
    return f"{ISO}::STREAM/{name}.BIN"


class StripTest(unittest.TestCase):
    def test_strips(self):
        # Two strips: 1 1 0 0 | 1 1 0. Winding alternates from each start.
        tris = ccsmodel.strip_triangles([1, 1, 0, 0, 1, 1, 0])
        self.assertEqual(tris, [(1, 0, 2), (1, 2, 3), (5, 4, 6)])


@unittest.skipUnless(os.path.exists(DATA), "needs work/infection/disc/DATA/DATA.BIN")
class DataBinTest(unittest.TestCase):
    def test_survey(self):
        s = ccs.survey(DATA)
        self.assertEqual((s.members, s.clean), (1023, 1023))
        self.assertEqual(dict(s.lost), {})
        # Only textures and skinned models run past or short of their size field.
        self.assertEqual(dict(s.disagree), {0x0300: 4306, 0x0800: 190})
        self.assertEqual(s.counts[0x0800], 17254)
        self.assertEqual(s.counts[0x0300], 4306)
        # Every DATA.BIN file has an empty frame section: Frame, then Top "end".
        self.assertEqual(dict(s.closes), {"end": 1023})
        self.assertEqual(s.frames, 0)

    def test_every_model(self):
        import collections

        import gzarc
        data = gzarc.open_bytes(DATA)
        kinds = collections.Counter()
        verts = tris = 0
        for m in gzarc.members(data):
            c = ccs.Ccs(gzarc.inflate(data, m))
            for model in ccsmodel.models(c):   # raises if a decode overruns
                self.assertEqual(ccsmodel.check_model(model), [], m.name)
                for mm in model.mmats:
                    kinds[mm.kind] += 1
                    verts += mm.vertex_count
                    tris += len(mm.triangles)
        self.assertEqual(dict(kinds), {"rigid": 16407, "shadow": 3129, "bone": 2994,
                                       "skin": 185})
        self.assertEqual((verts, tris), (5417645, 2819008))

    def test_weapon(self):
        c = ccs.Ccs(ccs.load(DATA + "::cw1hst00"))
        models = list(ccsmodel.models(c))
        self.assertEqual(len(models), 2)
        sword, shadow = models
        self.assertEqual((c.objects[sword.obj][0], sword.mtype, sword.scale),
                         ("MDL_cw1hst00", 1, 512.0))
        (mm,) = sword.mmats
        self.assertEqual((mm.kind, mm.vertex_count, len(mm.triangles)), ("rigid", 118, 68))
        self.assertIsNone(mm.colours)   # mtype & 1: read and dropped
        lo, hi = ccsmodel.bounds(mm.positions)
        # s16 * 512 / 4096: the raw extremes are (4, 4, -670) and (194, 48, 446).
        self.assertEqual(lo, (0.5, 0.5, -83.75))
        self.assertEqual(hi, (24.25, 6.0, 55.75))
        us = [u for u, v in mm.uvs]
        vs = [v for u, v in mm.uvs]
        self.assertTrue(0 <= min(us) and max(us) <= 1 and 0 <= min(vs) and max(vs) <= 1)
        for n in mm.normals:
            self.assertAlmostEqual(sum(x * x for x in n) ** 0.5, 1.0, delta=0.05)
        (sh,) = shadow.mmats
        self.assertEqual((sh.kind, sh.vertex_count, len(sh.triangles)), ("shadow", 17, 26))

    def test_skinned_body(self):
        c = ccs.Ccs(ccs.load(DATA + "::cbu1body"))
        scene = ccsmodel.Scene(c)
        (body,) = [m for m in ccsmodel.models(c) if scene.name(m.obj) == "MDL_cbalbody"]
        self.assertEqual(body.mtype, 5)
        self.assertEqual([mm.kind for mm in body.mmats], ["bone"] * 21 + ["skin"])
        skin = body.mmats[-1]
        self.assertEqual(skin.vertex_count, 1137)
        self.assertEqual(sum(len(v) for v in skin.weights), 1713)
        self.assertEqual(len(skin.triangles), 611)
        # Posed at frame 0 of the idle animation: a figure standing on z = 0
        # about 165 units tall, inside the file's BOX_bbox (+-90, +-80, 0..230).
        world = scene.world(scene.anime_locals(scene.find("ANM_cbu1nut0")))
        pts = [v for mm, pos, nrm in ccsmodel.place(scene, body, world) for v in pos]
        lo, hi = ccsmodel.bounds(pts)
        self.assertAlmostEqual(lo[2], 0.0, delta=0.5)
        self.assertAlmostEqual(hi[2], 163.16, delta=0.05)
        self.assertTrue(-90 <= lo[0] and hi[0] <= 90 and -80 <= lo[1] and hi[1] <= 80)


@unittest.skipUnless(os.path.exists(ISO), "needs work/infection/infection.iso")
class StreamTest(unittest.TestCase):
    def test_survey(self):
        want = {"STRCMN": (93, 8494, {"end": 92, "end, reset scene": 1}),
                "STRCMNE": (93, 8494, {"end": 92, "end, reset scene": 1}),
                "STR1": (40, 38569, {"end": 40}),
                "STR1E": (40, 38569, {"end": 40})}
        for name, (members, frames, closes) in want.items():
            s = ccs.survey(stream(name))
            self.assertEqual((s.members, s.clean), (members, members), name)
            self.assertEqual(dict(s.lost), {}, name)
            self.assertEqual(s.frames, frames, name)
            self.assertEqual(s.unordered, 0, name)
            self.assertEqual(dict(s.closes), closes, name)
            # Frame-section chunks all read exactly their size field.
            self.assertEqual(set(s.disagree) - {0x0300, 0x0800}, set(), name)

    def test_frame_section(self):
        c = ccs.Ccs(ccs.load(stream("STRCMN") + "::str6100.tmp"))
        tops = [(c.data[off + 8:off + 12]) for off, t, n, end in c.chunks()
                if t & 0xFFFF == 0xFF01]
        frames = [int.from_bytes(b, "little") for b in tops]
        self.assertEqual(c.frames, 398)
        self.assertEqual(frames, list(range(398)) + [0xFFFFFFFF])

    def test_cutscene_pose(self):
        # Models come from strNNNNe.tmp, frames from strNNNN.tmp.
        c = ccs.Ccs(ccs.load(stream("STR1") + "::str0130e.tmp"))
        scene = ccsmodel.Scene(c)
        frames = ccsmodel.Scene(ccs.Ccs(ccs.load(stream("STR1") + "::str0130.tmp")))
        (kite,) = [m for m in ccsmodel.models(c) if scene.name(m.obj) == "MDL_ckitbod1h"]
        locals_ = scene.frame_locals(0, frames)
        nodes = scene.clump_of(scene.model_owner[kite.obj])
        self.assertTrue(all(n in locals_ for n in nodes[3:9]))   # pelvis .. toe
        placed = ccsmodel.place(scene, kite, scene.world(locals_))
        self.assertTrue(all(pos is not None for mm, pos, nrm in placed))


if __name__ == "__main__":
    unittest.main()
