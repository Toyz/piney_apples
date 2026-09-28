#!/usr/bin/env python3
"""crates/piney-data's Model::morph against the game's ccMorpher::Modify.

ccMorpher::Modify (0x0013af10) blends a rigid mmat's positions toward its
targets' in the EE's 16-bit SIMD (psubh, pmaddh, psraw, paddsh) after
bringing each target to the base model's vertex scale. This runs the game's
own function natively in eemu (with test_anim's VU0 macro and MMI support)
on random morphers - one to four targets, weights across and past [0, 1],
positions over the whole s16 range and near it, targets at the base's scale
and at others - and compares every position with what the morph_probe
example (built with cargo) gives.

The run is set up as ccModel::Draw would leave it for the call:
ccDrawModelParam.rwflag bit 0 already set, so the blend is written over the
base positions in place instead of a buffer from ccDrawPacketCtrl::GetWork
(the same arithmetic either way), and ccSys's work pointer (+604) pointing
at free memory for the weight table.

Skipped when the executable is not extracted or cargo is missing.
PINEY_CARGO_ROOT names another cargo workspace to build the example in.
"""

import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
CARGO_ROOT = os.environ.get("PINEY_CARGO_ROOT", ROOT)
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(CARGO_ROOT, "target")), "release", "examples",
                       "morph_probe")

MODIFY = inf_va(0x0013AF10)
CC_SYS = inf_va(0x003788E0)          # the ccSys pointer (gp-relative global)
SYS = 0x01800000             # the struct it points at
WORK = 0x01810000            # ccSys +604: the weight table's scratch
MORPHER = 0x01820000
PARAM = 0x01821000
BASE = 0x01830000            # the base positions, s16 x 3 per vertex
TARGETS = 0x01840000         # per target: ccModel, index, chunk, mmat, positions
CASES = 300


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def bits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data",
                    "--example", "morph_probe"], cwd=CARGO_ROOT, check=True)


def ask(requests):
    p = subprocess.run([EXAMPLE], input="\n".join(requests) + "\n", capture_output=True, text=True, check=True)
    return [[int(v) for v in line.split()] for line in p.stdout.splitlines()]


def case(rng):
    """A random morpher: (base scale, base positions, [(scale, weight, positions)])."""
    n = rng.randint(1, 12)
    spread = rng.choice([100, 3000, 32767])

    def positions():
        return [[rng.randint(-spread, spread) for _ in range(3)] for _ in range(n)]

    base_scale = f32(rng.choice([1.0, 0.5, 2.0, 16.0, rng.uniform(0.1, 40.0)]))
    targets = []
    for _ in range(rng.randint(1, 4)):
        scale = base_scale if rng.random() < 0.6 else f32(rng.uniform(0.1, 40.0))
        weight = f32(rng.choice([0.0, 1.0, 0.5, 0.4, 0.6, rng.uniform(0, 1), rng.uniform(-1.5, 1.5)]))
        tp = positions()
        # Keep a rescaled target inside s16, as the data does.
        if scale != base_scale:
            f = scale / base_scale
            tp = [[max(-32767, min(32767, int(v / max(f, 1.0)))) for v in p] for p in tp]
        targets.append((scale, weight, tp))
    return base_scale, positions(), targets


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted executable and cargo")
class MorphAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import test_anim
        from image import Program
        build()
        cls.machine = test_anim.machine_class()
        cls.prog = Program(ELF)

    def run_game(self, base_scale, base, targets):
        m = self.machine(self.prog)
        n = len(base)
        m.store(CC_SYS, 4, SYS)
        m.store(SYS + 604, 4, WORK)
        m.store(MORPHER + 0x10, 2, len(targets))
        at = TARGETS
        for i, (scale, weight, tp) in enumerate(targets):
            model, index, chunk, mmat, verts = at, at + 0x40, at + 0x80, at + 0x100, at + 0x200
            m.store(MORPHER + 0x14 + 8 * i, 4, model)
            m.store(MORPHER + 0x18 + 8 * i, 4, bits(weight))
            m.store(model + 0x00, 4, index)          # ccModel.index
            m.store(index + 44, 4, chunk)            # the chunk ccModel::Init read
            m.store(chunk + 64, 4, bits(scale))      # ccModelChunk.vertexScale
            m.store(model + 0x08, 4, mmat)           # ccModel.mmat
            m.store(model + 0x0C, 4, bits(scale))    # ccModel.vertexScale
            m.store(model + 0x22, 2, 1)              # ccModel.mmatNum
            m.store(mmat + 0x04, 4, n)               # ccMmat.vertexNum
            m.store(mmat + 0x10, 4, verts)           # ccMmat.vertexData
            for k, p in enumerate(tp):
                for c in range(3):
                    m.store(verts + 6 * k + 2 * c, 2, p[c])
            at += 0x1000
        for k, p in enumerate(base):
            for c in range(3):
                m.store(BASE + 6 * k + 2 * c, 2, p[c])
        m.store(PARAM + 364, 4, bits(base_scale))   # vertexScale
        m.store(PARAM + 372, 2, 0)                  # mmatNum: mmat 0
        m.store(PARAM + 374, 2, 1)                  # rwflag: written in place
        m.store(PARAM + 384, 4, n)                  # vertexNum
        m.store(PARAM + 388, 4, BASE)               # vertexData
        m.call(MODIFY, (MORPHER, PARAM), limit=5_000_000)
        return [m.load(BASE + 6 * k + 2 * c, 2, signed=True) for k in range(n) for c in range(3)]

    def test_modify(self):
        rng = random.Random(0x6d6f7270)
        cases = [case(rng) for _ in range(CASES)]
        # The wrap and saturation at the ends of the range, and weight 1.
        cases.append((1.0, [[-32000, 32000, 0]], [(1.0, 1.0, [[32000, -32000, 5]])]))
        cases.append((1.0, [[32767, -32768, 7]], [(1.0, 0.5, [[-32768, 32767, 7]]), (1.0, 0.5, [[0, 0, 7]])]))
        requests = []
        for base_scale, base, targets in cases:
            words = [f"{bits(base_scale):08x}", str(len(base)), str(len(targets))]
            words += [str(v) for p in base for v in p]
            for scale, weight, tp in targets:
                words += [f"{bits(scale):08x}", f"{bits(weight):08x}"] + [str(v) for p in tp for v in p]
            requests.append(" ".join(words))
        answers = ask(requests)
        self.assertEqual(len(answers), len(cases))
        for i, ((base_scale, base, targets), port) in enumerate(zip(cases, answers)):
            game = self.run_game(base_scale, base, targets)
            self.assertEqual(port, game, f"case {i}: base scale {base_scale}, {len(targets)} targets")
        print(f"\nccMorpher::Modify: {len(cases)} morphers, every position equal", file=sys.stderr)


if __name__ == "__main__":
    unittest.main()
