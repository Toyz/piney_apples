#!/usr/bin/env python3
"""The 3D sounds' listener in a boss fight's frame against the game's own
code. ccSeOn3D (INF main 0x00179d90, MUT 0x0017c490, OUT 0x0017c120, QUA
0x0017c080) pans at the call: calcPan (0x0017a4c0) reads activeCamPtr
then. Kite's footsteps (ccPlayer::Main's notes through ccSeSetParamSPC,
main 0x0017aa20, at priority 49) come before the boss's CamMain (66)
moves camera 2, so they are panned by camera 2 as the last frame's
CamMain left it.

The game runs as tools/test_bosscam_rs.py runs it: cameraMain and
ccPlayer::Main (Kite walking, its CameraPosSet and cameraSet) on camera
2, then CamMain with the boss stepping. Neither side's Kite plays its
clips' notes here, so a footstep (param 0x10) is made where its note
would come, after ccPlayer::Main: the game's calcPan runs then, natively,
on Kite's place; the probe's `note` puts Kite's sound into the frame's
show stream at that point. After CamMain the probe answers with the
frame's sounds as the stream hands them on (`piney_world::combat::heard`:
each with the listener the last `Show::Ear` before it left, the camera
as it stands then before any) and Kite's place; the game's calcPan on a
scratch camera holding that listener must give the game's pan, frame by
frame. The pan from the camera after CamMain must differ on some frames,
so the check fails if the listener is taken after the frame.

    PINEY_VOLUME=outbreak python3 tools/test_listener_rs.py

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402

import test_bosscam_rs as tbc  # noqa: E402
import test_evcam_rs as tev  # noqa: E402
import test_world_rs as tw  # noqa: E402
from test_world_rs import ELF, ISO, ONE, fb  # noqa: E402

ACTIVE = inf_va(0x0037896C)
SCRATCH_CAM = 0x01E80000        # the camera calcPan reads for a pan worked out after the frame


FOOTSTEP = 0x10


class Game(tbc.Game):
    """test_bosscam_rs's game, with calcPan run on any camera."""

    def active_ear(self):
        a = self.m.load(ACTIVE, 4)
        return (self.rvec(a, 4), self.rvec(a + 0x10, 4), self.m.load(a + 0x5C, 4))

    def pan(self, pos, ear):
        """calcPan(pos) with activeCamPtr on a camera at `ear`: (pan << 16)
        | cdeg."""
        m = self.m
        was = m.load(ACTIVE, 4)
        m.mem[SCRATCH_CAM:SCRATCH_CAM + 0x70] = bytes(0x70)
        self.vec(SCRATCH_CAM, ear[0])
        self.vec(SCRATCH_CAM + 0x10, ear[1])
        m.store(SCRATCH_CAM + 0x5C, 4, ear[2])
        m.store(ACTIVE, 4, SCRATCH_CAM)
        self.vec(tw.ARGS, pos)
        v = self.call("calcPan__FPf", tw.ARGS)
        m.store(ACTIVE, 4, was)
        return v & 0xFFFFFFFF


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ListenerAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        tbc.build()
        cls.g = Game()
        cls.probe = tbc.Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def run_case(self, label, scheme, rng, frames):
        g, p = self.g, self.probe
        kite = (0, fb(5600.0), fb(600.0))
        dircz = fb(rng.uniform(-3.1, 3.1))
        g.begin(kite, dircz, scheme, 3, 1, {}, {})
        p.ask("start", 0, *kite, dircz, scheme, 3, 1)
        boss = [kite[0], fb(5100.0), kite[2], ONE]
        g.make(tbc.TRANSFER, boss)
        p.ask("boss", *tbc.TRANSFER)
        for f, pad in enumerate(tev.walk_pads(rng, frames)):
            g.pad(*pad)
            g.frame()
            p.ask("frame", *tev.pad_words(pad))
            # The footstep at Kite's Draw: calcPan as the camera stands now.
            pos = g.rvec(tw.PLAYER + 0x40, 4)
            want = g.pan(pos, g.active_ear())
            p.ask("note", FOOTSTEP)
            boss = tbc.step(boss, rng)
            g.cam_main(boss)
            got = p.ask("main", *boss, *tev.pad_words(pad))["sounds"]
            # The camera as the frame ends: what a listener taken after
            # the tasks would be.
            late = g.active_ear()
            what = f"{label} frame {f}"
            self.assertEqual(len(got), 1, f"{what}: Kite's sounds")
            pparam, ppos, pear = got[0]
            self.assertEqual(pparam, FOOTSTEP, f"{what}: param")
            self.assertEqual(ppos, pos, f"{what}: Kite's place")
            self.assertEqual(g.pan(ppos, (pear[0], pear[1], pear[2])), want, f"{what}: calcPan")
            self.seen["sounds"] += 1
            self.seen["late"] += int(g.pan(pos, late) != want)

    def test_footsteps(self):
        self.seen = {"sounds": 0, "late": 0}
        rng = random.Random(11)
        for scheme in (0, 1, 2, 3):
            self.run_case(f"scheme {scheme}", scheme, rng, 200)
        self.assertGreater(self.seen["sounds"], 100, "Kite's footsteps")
        self.assertGreater(self.seen["late"], 10, "the camera after CamMain pans some otherwise")


if __name__ == "__main__":
    unittest.main()
