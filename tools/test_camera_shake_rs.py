#!/usr/bin/env python3
"""crates/piney-world's screen shake (camera.rs `Shake`: cameraShake and
cameraShockAbsorber) against the game's own cameraShake (main 0x00162cd0)
and cameraShockAbsorber (0x001629b0) run in eemu over test_battle's
GameBattle.

camera.cpp's statics start as ccThCamera leaves them (every sfList slot
free, the force and offset 0; vibrateRotate, vibrateCycle, the matrix and
the oscillator at the executable's 0). newlib's rand() is hooked to the
same generator the port uses (state * 6364136223846793005 + 1, the top 31
bits) from one seed. Each run is 400 frames: on some frames one to three
cameraShake calls (power 0-2, cycle 0-2, time 0-40, dirc 0-3), then
cameraShockAbsorber; the first frame always shakes (vibrateCycle 0 would
make the game divide by zero). After every call the slots, vibrateForce,
vibrateCycle, vibrateRotate, vibrateMatrix, vibrateOffset's z, the
oscillator and rand()'s state are compared with the shake_probe example's.
Then cameraSet runs for camera 1 at a random eye and target (plw 0, so no
map wrap; SetMatrix_PosTarget hooked to take the eye and target it is
given, SetView a no-op), and the two points are compared with the port's
`Shake::apply`.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "shake_probe")

SF_LIST, FORCE, CYCLE, ROTATE = inf_va(0x00383E30), inf_va(0x00378998), inf_va(0x00378994), inf_va(0x0037899C)
MATRIX, OFFSET, OSC = inf_va(0x00383E80), inf_va(0x00383E70), inf_va(0x003789A0)
CAM_ID, ACTIVE, PLW = inf_va(0x0037897C), inf_va(0x0037896C), inf_va(0x00730300)
CAM = 0x01E00000                    # a scratch camera: pos +0, view +0x10, cptr +0x50
MUL = 6364136223846793005
M64 = (1 << 64) - 1


def nib(v):
    v &= 0xF
    return v - 16 if v & 8 else v


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


class Game:
    def __init__(self):
        import battle
        import test_battle as tb
        self.g = tb.GameBattle(battle.Data(ELF))
        self.m, self.sym = self.g.m, self.g.sym
        self.m.hooks[self.sym("rand")] = self.rand
        self.m.hooks[self.sym("SetMatrix_PosTarget__5ccCamFPfPf")] = self.pos_target
        self.m.hooks[self.sym("SetView__6ccViewFRC5ccCamPA4_f")] = lambda mm, *_: 0
        self.state = 1
        self.set_to = None

    def pos_target(self, mm, cam, p, v, *_):
        self.set_to = [[mm.load(p + 4 * i, 4) for i in range(4)], [mm.load(v + 4 * i, 4) for i in range(4)]]
        return 0

    def camera_set(self, pos, view):
        m = self.m
        m.mem[CAM:CAM + 0x60] = bytes(0x60)
        for i, x in enumerate(pos + view):
            m.store(CAM + 4 * i, 4, x)
        m.store(CAM + 0x50, 4, CAM + 0x100)
        m.store(CAM_ID, 2, 1)
        m.store(ACTIVE, 4, CAM)
        m.store(PLW, 4, 0)
        self.set_to = None
        m.call(self.sym("cameraSet__Fv"), [])
        return self.set_to

    def rand(self, mm, *_):
        self.state = (self.state * MUL + 1) & M64
        return (self.state >> 32) & 0x7FFFFFFF

    def reset(self, seed):
        m = self.m
        self.state = seed
        m.mem[SF_LIST:SF_LIST + 0x40] = bytes([0x0F, 0, 0, 0, 0, 0, 0, 0] * 8)
        for a, n in ((FORCE, 4), (CYCLE, 2), (ROTATE, 4), (OSC, 2)):
            m.store(a, n, 0)
        m.mem[MATRIX:MATRIX + 0x40] = bytes(0x40)
        m.mem[OFFSET:OFFSET + 0x10] = bytes(0x10)

    def shake(self, p, c, t, d):
        self.m.call(self.sym("cameraShake__Fiiii"), [p, c, t, d])

    def absorb(self):
        self.m.call(self.sym("cameraShockAbsorber__Fv"), [])

    def state_json(self):
        m = self.m
        sf = []
        for k in range(8):
            a = SF_LIST + 8 * k
            b0, b1 = m.load(a, 1), m.load(a + 1, 1)
            sf.append([nib(b0), (b0 >> 4) & 0xF, b1 & 0xF, s16(m.load(a + 2, 2)), s16(m.load(a + 4, 2))])
        z = m.load(OFFSET + 8, 4)
        return {"sf": sf, "force": m.load(FORCE, 4), "cycle": s16(m.load(CYCLE, 2)),
                "rotate": s32(m.load(ROTATE, 4)), "matrix": [m.load(MATRIX + 4 * i, 4) for i in range(16)],
                "z": 0 if z == 0x80000000 else z, "osc": s16(m.load(OSC, 2)), "rand": self.state}


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class CameraShakeAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                        "shake_probe"], cwd=ROOT, check=True)
        cls.game = Game()

    def test_shakes(self):
        rng = random.Random(41)
        counts = {"shakes": 0, "frames shaking": 0, "turning": 0}
        for run in range(12):
            g, p = self.game, Probe()
            seed = rng.getrandbits(63)
            g.reset(seed)
            p.ask(f"seed {seed}")
            for f in range(400):
                calls = rng.choice((1, 2, 3)) if f == 0 or rng.random() < 0.08 else 0
                for _ in range(calls):
                    args = (rng.randrange(3), rng.randrange(3), rng.randrange(41), rng.randrange(4))
                    g.shake(*args)
                    mine = p.ask("shake " + " ".join(map(str, args)))
                    self.assertEqual(mine, g.state_json(), f"run {run} frame {f} shake {args}")
                    counts["shakes"] += 1
                g.absorb()
                mine = p.ask("absorb")
                game = g.state_json()
                self.assertEqual(mine, game, f"run {run} frame {f} absorb")
                import struct
                fb = lambda x: struct.unpack("<I", struct.pack("<f", x))[0]  # noqa: E731
                px, py, pz = rng.uniform(0, 48000), rng.uniform(0, 48000), rng.uniform(-100, 600)
                pos = [fb(px), fb(py), fb(pz), 0x3F800000]
                view = [fb(px + rng.uniform(-1500, 1500)), fb(py + rng.uniform(-1500, 1500)),
                        fb(pz + rng.uniform(-400, 100)), 0x3F800000]
                if rng.random() < 0.1:
                    view = [fb(px + rng.uniform(-150, 150)), fb(py + rng.uniform(-150, 150)), view[2], 0x3F800000]
                got = g.camera_set(pos, view)
                mine = p.ask("set " + " ".join(map(str, pos[:3] + view[:3])))
                self.assertEqual(mine, got, f"run {run} frame {f} cameraSet {pos} {view}")
                counts["cameraSet"] = counts.get("cameraSet", 0) + 1
                if game["force"]:
                    counts["frames shaking"] += 1
                if game["rotate"] >= 0 and game["force"]:
                    counts["turning"] += 1
            p.p.stdin.close()
            p.p.stdout.close()
            p.p.wait()
        print(" ".join(f"{k} {v}" for k, v in counts.items()))


if __name__ == "__main__":
    unittest.main()
