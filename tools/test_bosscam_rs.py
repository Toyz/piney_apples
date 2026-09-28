#!/usr/bin/env python3
"""crates/piney-world's boss camera (bosscam.rs, `ccBossCam`) against the
game's own code run in eemu over the field as tools/test_evcam_rs.py's Game
sets it up (test_world_rs's Field: Mac Anu's collision mesh, Kite arriving,
then ccThCamera itself for the camera's set-up, tcam copied to ecam and
bcam).

A ccBossCam is built in the game's memory by its constructor (gcmn
0x00460ca0) as ccBoss::InitBossCamera(200, 1000) makes it: transfer (0,
1000, 200, 1), PView a boss position 500 below Kite, tempChar Kite (plw+0x20
set to him). The bosscam_probe example does the same through
BossCam::new. Each frame, in the kernel's order: cameraMain and
ccPlayer::Main (the field's camera and player tasks, now drawing through
bcam, Kite walking by its heading), then now and then QuakeCam, then
CamMain with the boss somewhere new (a step, or a jump as Skeith's dashes
make). QuakeCam draws from ccRand natively; the vector it made is handed to
the probe, whose ccRandF is checked in piney-battle. At the end,
OffBossCamera (on a ccBoss holding the camera and bossCamSW) hands bcam to
tcam, and more frames run on camera 1. OffBossCamera's cameraGetRot(r, 2)
leaves r's y and w as the stack had them, and they go to tcam.rot; they
are cleared in the game after it and the port takes zero.

Compared bit for bit after the constructor and after every CamMain: the
ccBossCam's InitLock, ResetFlg, CamView, CamPos, CamRot, MoveTransfer, deg,
memDircZ, RemitRotMax, count, CameraType, QuakeFlg and Quakevector; after
every step camID, tcam and bcam (all but cptr), and cameraSet's world_view
and world_screen.

The pads: the left stick often, the right stick and the camera buttons
(L1, R1, L2, R2, pressures) now and then, over the four control schemes.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import math
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_evcam_rs as tev  # noqa: E402
import test_world_rs as tw  # noqa: E402
from test_world_rs import ELF, ISO, ONE, ROOT, fb  # noqa: E402

EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "bosscam_probe")

TCAM, BCAM, CAMID = inf_va(0x00383CE0), inf_va(0x00383D50), inf_va(0x0037897C)
# plw+0x20: the player's ccChar, the constructor's tempChar.
PLW_CHAR = inf_va(0x00730300)
TRANSFER = [0, fb(1000.0), fb(200.0), ONE]
IDLE = (0, 0, 0, 0, 0, 0, [0] * 12)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "bosscam_probe"], cwd=ROOT, check=True)


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                  cwd=ROOT)

    def ask(self, *words):
        self.p.stdin.write(" ".join("%x" % (w & 0xFFFFFFFF) if isinstance(w, int) else w for w in words) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


class Game(tev.Game):
    def cameras(self):
        m = self.m

        def cam(a):
            return {"pos": self.rvec(a), "view": self.rvec(a + 16), "rot": self.rvec(a + 32)}
        st = self.state()
        return {"cam_id": m.load(CAMID, 2, True), "tcam": cam(TCAM), "bcam": cam(BCAM),
                "world_view": st["world_view"], "world_screen": st["world_screen"]}

    def boss_state(self):
        m, c = self.m, self.bc
        return {
            "init_lock": m.load(c + 1, 1), "reset": m.load(c + 4, 4, True), "view": self.rvec(c + 0x10),
            "pos": self.rvec(c + 0x20), "rot": self.rvec(c + 0x30), "move_transfer": self.rvec(c + 0xB0),
            "deg": m.load(c + 0xE0, 2, True), "mem_dirc_z": m.load(c + 0xE2, 2, True),
            "remit_rot_max": m.load(c + 0xE4, 2, True), "count": m.load(c + 0xE8, 4, True),
            "camera_type": m.load(c + 0x10C, 4, True), "quake_flg": m.load(c + 2, 1), "quake": self.rvec(c + 0xC0),
        }

    def make(self, transfer, boss):
        """ccBossCam::ccBossCam(PView, transfer) with PView at `boss`."""
        m = self.m
        m.store(PLW_CHAR, 4, tw.PLAYER)
        self.bc = self.malloc(m, 0x140)
        self.bpos = self.malloc(m, 0x10)
        self.vec(self.bpos, boss)
        self.vec(tw.ARGS, transfer)
        self.call("__ct__9ccBossCamFPfPA4_f", self.bc, tw.ARGS, self.bpos)
        return {"boss": self.boss_state(), "cameras": self.cameras()}

    def quake(self, q):
        self.vec(tw.ARGS, q)
        self.call("QuakeCam__9ccBossCamFPf", self.bc, tw.ARGS)
        return self.rvec(self.bc + 0xC0, 3)

    def cam_main(self, boss):
        self.vec(self.bpos, boss)
        self.call("CamMain__9ccBossCamFv", self.bc)
        return {"boss": self.boss_state(), "cameras": self.cameras()}

    def off(self):
        """ccBoss::OffBossCamera on a boss holding the camera, bossCamSW 1.
        cameraGetRot(r, 2) leaves r's y and w as the stack had them, and
        cameraSetRot copies them to tcam: the port takes zero."""
        m = self.m
        b = self.malloc(m, 0x200)
        m.store(b + 0x14C, 4, self.bc)
        m.store(b + 0x150, 4, 1)
        self.call("OffBossCamera__6ccBossFv", b)
        m.store(TCAM + 0x24, 4, 0)
        m.store(TCAM + 0x2C, 4, 0)
        return self.cameras()


def pads(rng, n, l2=True):
    ps = tev.walk_pads(rng, n)
    return [p if l2 else (p[0] & ~1, p[1] & ~1) + tuple(p[2:]) for p in ps]


def step(boss, rng):
    """The boss moved: mostly a step, sometimes a jump across the arena."""
    x, y = to_f(boss[0]), to_f(boss[1])
    if rng.random() < 0.1:
        a, d = rng.uniform(-math.pi, math.pi), rng.uniform(300, 2500)
        return [fb(x + d * math.cos(a)), fb(y + d * math.sin(a)), boss[2], ONE]
    return [fb(x + rng.uniform(-40, 40)), fb(y + rng.uniform(-40, 40)), boss[2], ONE]


def to_f(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class BossCameraAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.g = Game()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def same(self, want, got, what):
        for k in want:
            self.assertEqual(want[k], got[k], f"{what}: {k}")

    def frame(self, pad, what):
        self.g.pad(*pad)
        self.g.frame()
        self.same(self.g.cameras(), self.probe.ask("frame", *tev.pad_words(pad)), what)

    def run_case(self, label, scheme, rng, frames):
        kite = (0, fb(5600.0), fb(600.0))
        dircz = fb(rng.uniform(-math.pi, math.pi))
        self.g.begin(kite, dircz, scheme, 3, 1, {}, {})
        self.probe.ask("start", 0, *kite, dircz, scheme, 3, 1)
        for f in range(2):
            self.frame(IDLE, f"{label}: frame {f} before")
        boss = [kite[0], fb(5100.0), kite[2], ONE]
        want = self.g.make(TRANSFER, boss)
        got = self.probe.ask("boss", *TRANSFER)
        self.same(want["boss"], got["boss"], f"{label}: made")
        self.same(want["cameras"], got["cameras"], f"{label}: made")
        for f, pad in enumerate([IDLE] * 2 + pads(rng, frames)):
            self.frame(pad, f"{label}: frame {f} on camera 2")
            boss = step(boss, rng) if f >= 2 else boss
            if rng.random() < 0.15:
                v = self.g.quake([fb(35.0), fb(35.0), fb(35.0), ONE])
                self.probe.ask("quake", *v)
            want = self.g.cam_main(boss)
            got = self.probe.ask("main", *boss, *tev.pad_words(pad))
            self.same(want["boss"], got["boss"], f"{label}: CamMain {f}")
            self.same(want["cameras"], got["cameras"], f"{label}: CamMain {f}")
            b = want["boss"]
            self.seen["reset"].add(b["reset"])
            self.seen["count"] = max(self.seen["count"], b["count"])
            self.seen["zoom"].add(b["move_transfer"][1])
        self.same(self.g.off(), self.probe.ask("off"), f"{label}: OffBossCamera")
        for f, pad in enumerate([IDLE] * 2 + pads(rng, 6, l2=False)):
            self.frame(pad, f"{label}: frame {f} back on camera 1")

    def setUp(self):
        self.seen = {"reset": set(), "count": 0, "zoom": set()}

    def test_schemes(self):
        rng = random.Random(1)
        for scheme in (0, 1, 2, 3):
            self.run_case(f"scheme {scheme}", scheme, rng, 120)
        # Both resets, and the zoom between its ends.
        self.assertEqual(self.seen["reset"], {0, 1, 2})
        self.assertGreater(len(self.seen["zoom"]), 20)

    def test_long_chase(self):
        """Long runs of catching up: count past 20 doubles the turn."""
        rng = random.Random(2)
        self.run_case("chase", 0, rng, 400)
        self.assertGreater(self.seen["count"], 20)


if __name__ == "__main__":
    unittest.main()
