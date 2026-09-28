#!/usr/bin/env python3
"""crates/piney-battle's fire breath (breath.rs, `Breath`) against the
game's own ccEnemyBreath and ccEnemyBrPart (gcmn 0x0043ae90-0x0043c134) run
in eemu over test_battle's GameBattle (GCMN.PRG loaded).

A breath is made by the game's initBreath over a made-up ccEnemyBrInfo
(kind 0 or 1, a smoke kind) whose node is a matrix in scratch memory (the
flames' effects: ccEff::Init stubbed, the chunk a dummy). Then 400 frames
as a race's exclusive() runs it: ctrlBreath, then setBreath with one of the
game's breath rows (ehkBrParam, elBrParam's three) or a made-up one, the
enemy's attack frame counting through it, now and then not displayed or
faded, the node turned and moved every frame. What is not rules is
answered from the harness's dice and recorded: ccHitCheckLM2 (mostly
nothing in the way), ccLandHitCheck (the ground under, at or above the
flame), ccTransPosW2P (x + 0.5, y - 0.25) and P2W (x - 0.5, y + 0.25);
ccEff::Draw and effSmoke are recorded, not run. ccRandF, cosf and atan2f
run natively; ccRand is the game's Mersenne Twister, seeded alike on both
sides.

The breath_probe example runs the port on the same steps; after each step
the breath (its count and every flame with its effect's place, size, turn
and alpha), the node's matrix after setBreath, what ctrlBreath put out
(the draws, the bursts) and ccRand's state are compared.

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
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "breath_probe")

BASE = 0x01E00000
OBJ, BR, INFO, NODE, PARAM, NAMES, CHUNK = (BASE, BASE + 0x400, BASE + 0x500, BASE + 0x600, BASE + 0x700,
                                           BASE + 0x800, BASE + 0x900)
HEAP, HEAP_END = 0x01C00000, 0x01E00000
MT, MTI = inf_va(0x003FF400), inf_va(0x00377FD0)
# the game's breath rows: ccEnemyH's, ccEnemyL's dragons' two and wyrms'
PARAMS = (inf_va(0x005E2760), inf_va(0x005E5A20), inf_va(0x005E5A60), inf_va(0x005E5AA0))
MINUS_ONE = 0xBF800000


def f32(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def fl(v):
    return struct.unpack("<f", struct.pack("<I", v & 0xFFFFFFFF))[0]


def s32(v):
    return v - 0x100000000 if v & 0x80000000 else v


def sgenrand(seed):
    out = []
    for _ in range(624):
        w = seed & 0xFFFF0000
        seed = (seed * 69069 + 1) & 0xFFFFFFFF
        w |= (seed & 0xFFFF0000) >> 16
        seed = (seed * 69069 + 1) & 0xFFFFFFFF
        out.append(w)
    return out


def matrix(rx, ry, rz, t):
    """A node's world matrix (rows x, y, z, translation) as float bits."""
    cx, sx, cy, sy, cz, sz = math.cos(rx), math.sin(rx), math.cos(ry), math.sin(ry), math.cos(rz), math.sin(rz)
    r = [[cy * cz, cy * sz, -sy], [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy],
         [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy]]
    rows = [r[0] + [0.0], r[1] + [0.0], r[2] + [0.0], list(t) + [1.0]]
    return [f32(x) for rr in rows for x in rr]


class Game:
    def __init__(self):
        import battle
        import test_battle as tb
        self.g = tb.GameBattle(battle.Data(ELF))
        m, sym = self.g.m, self.g.sym
        self.m, self.sym = m, sym
        m.hooks.pop(sym("sceVu0CopyVector"), None)
        m.mem[BASE:BASE + 0x10000] = bytes(0x10000)
        self.heap = HEAP
        self.outs = []
        self.lands, self.lines = [], []
        self.rng = random.Random(0)
        m.store(sym("worldman"), 4, BASE + 0xA00)
        hooks = {
            "__nw__FUi": self.new,
            "GetChunkAdrsF__8ccStreamFPCci": lambda mm, *_: CHUNK,
            "Init__5ccEffFP10ccEffChunki": lambda mm, *_: 0,
            "SetActiveLayer__9WORLD_MANFi": lambda mm, *_: 0,
            "Draw__5ccEffFUs": self.draw,
            "ccHitCheckLM2__FPfPfUi": self.line,
            "ccLandHitCheck__FPfUi": self.land,
            "ccTransPosW2P__FPfPf": self.w2p,
            "ccTransPosP2W__FPfPf": self.p2w,
            "effSmoke__FPfPffiiUsUs": self.smoke,
        }
        for n, h in hooks.items():
            m.hooks[sym(n)] = h

    # memory ---------------------------------------------------------------------------
    def put(self, va, fmt, *v):
        self.m.mem[va:va + struct.calcsize(fmt)] = struct.pack(fmt, *v)

    def get(self, va, fmt):
        return list(struct.unpack(fmt, bytes(self.m.mem[va:va + struct.calcsize(fmt)])))

    def vec(self, a):
        return self.get(a, "<4I")

    # hooks ----------------------------------------------------------------------------
    def new(self, mm, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        if self.heap > HEAP_END:
            raise eemu.Stop("the harness's heap ran out")
        mm.mem[a:a + n] = bytes(n)
        return a

    def parts(self):
        out, p = [], self.m.load(BR + 12, 4)
        for _ in range(64):
            out.append(p)
            p = self.m.load(p + 68, 4)
        return out

    def draw(self, mm, eff, pat, *_):
        k = next(i for i, p in enumerate(self.parts()) if mm.load(p + 56, 4) == eff)
        self.outs.append(["draw", k, self.vec(eff + 16), self.get(eff + 32, "<2I"), mm.load(eff + 40, 4),
                          mm.load(eff + 52, 4), pat & 0xFFFF])
        return 0

    def line(self, mm, a, b, mask, *_):
        v = MINUS_ONE if self.rng.random() < 0.9 else f32(self.rng.uniform(0, 1))
        self.lines.append(v)
        mm.f[0] = v
        return v

    def land(self, mm, p, mask, *_):
        z = fl(mm.load(p + 8, 4))
        r = self.rng.random()
        v = mm.load(p + 8, 4) if r < 0.1 else f32(z + (self.rng.uniform(5, 80) if r < 0.3
                                                      else -self.rng.uniform(10, 600)))
        self.lands.append(v)
        mm.f[0] = v
        return v

    def w2p(self, mm, o, i, *_):
        x, y, z, w = self.get(i, "<4I")
        self.put(o, "<4I", eemu.f_add(x, 0x3F000000), eemu.f_add(y, 0xBE800000), z, w)
        return 0

    def p2w(self, mm, o, i, *_):
        x, y, z, w = self.get(i, "<4I")
        self.put(o, "<4I", eemu.f_sub(x, 0x3F000000), eemu.f_sub(y, 0xBE800000), z, w)
        return 0

    def smoke(self, mm, pos, vel, n, kind, *_):
        t0, t1 = mm.r[8] & 0xFFFF, mm.r[9] & 0xFFFF
        self.outs.append(["smoke", self.vec(pos), self.vec(vel), mm.f[12] & 0xFFFFFFFF, s32(n), s32(kind), t0, t1])
        return 0

    # the breath -----------------------------------------------------------------------
    def build(self, kind, smoke, seed):
        self.heap = HEAP
        self.put(INFO, "<7I", kind, smoke, NODE, BASE + 0xB00, NAMES, NAMES, NAMES + 0x10)
        self.m.mem[NAMES:NAMES + 0x20] = b"OBJ_x\0".ljust(0x10, b"\0") + b"\0" * 0x10
        self.m.mem[BR:BR + 16] = bytes(16)
        self.m.mem[MT:MT + 8 * 624] = struct.pack("<624Q", *sgenrand(seed))
        self.m.store(MTI, 4, 624)
        self.m.call(self.sym("initBreath__13ccEnemyBreathFP13ccEnemyBrInfo"), (BR, INFO))

    def read(self):
        m = self.m
        ps = []
        for p in self.parts():
            e = m.load(p + 56, 4)
            fl0 = m.load(p, 1)
            v = [m.load(p + 48, 4), fl0 & 1, (fl0 >> 1) & 1, m.load(p + 2, 2), m.load(p + 6, 2), m.load(p + 8, 2),
                 m.load(p + 10, 2), m.load(p + 12, 4)]
            v += self.vec(p + 16) + self.vec(p + 32) + [m.load(p + 52, 4)]
            v += self.vec(e + 16) + self.get(e + 32, "<2I") + [m.load(e + 40, 4), m.load(e + 52, 4)]
            ps.append(v)
        return [s32(m.load(BR, 4)), ps]

    def cc_state(self):
        h = 0xCBF29CE484222325
        words = struct.unpack("<624Q", bytes(self.m.mem[MT:MT + 8 * 624]))
        for w in words:
            for byte in (w & 0xFFFFFFFF).to_bytes(4, "little"):
                h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        return [s32(self.m.load(MTI, 4)), h]

    def set(self, param, disp, trans, cnt, node):
        self.put(NODE, "<16I", *node)
        self.m.store(OBJ + 0xE0, 1, 0x10 if disp else 0)
        self.m.store(OBJ + 0x88, 4, trans)
        self.put(OBJ + 0x25C, "<h", cnt)
        self.m.call(self.sym("setBreath__13ccEnemyBreathFP14ccEnemyBrParamP10ccEntryObj"), (BR, param, OBJ))
        return self.read(), self.get(NODE, "<16I")

    def ctrl(self):
        self.outs, self.lands, self.lines = [], [], []
        self.m.call(self.sym("ctrlBreath__13ccEnemyBreathFv"), (BR,))
        return self.read(), self.outs, self.cc_state()


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def diffs(a, b, path="", out=None):
    """Where two nested lists differ (paths and the two values), at most 12."""
    out = [] if out is None else out
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append("%s: lengths %d != %d" % (path, len(a), len(b)))
        for i, (x, y) in enumerate(zip(a, b)):
            if len(out) < 12:
                diffs(x, y, "%s[%d]" % (path, i), out)
    elif a != b:
        out.append("%s: %r != %r" % (path, a, b))
    return out


def made_up(rng):
    """A breath row of the harness's: a short attack, flames every frame or
    every second, fast or slow, some turning to the sides."""
    life = rng.choice((8, 16, 24, 40))
    start = rng.randrange(0, 30)
    end = start + rng.randrange(0, 60)
    mask = rng.choice((0, 1, 3))
    v = [rng.uniform(10, 60), rng.uniform(-10, 10), rng.uniform(-10, 10)]
    return struct.pack("<4H8x4I4I4I", life, start, end, mask, *[f32(x) for x in v], f32(rng.uniform(0.5, 3)),
                       *[f32(rng.uniform(-0.05, 0.02)) for _ in range(3)], f32(rng.uniform(-0.1, 0.1)),
                       *[f32(rng.uniform(-0.3, 0.3)) for _ in range(3)], f32(rng.uniform(-0.1, 0.1)))


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class EnemyBreathAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-battle", "--example",
                        "breath_probe"], cwd=ROOT, check=True)
        cls.game = Game()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def run_case(self, seed, frames=400):
        game, probe = self.game, self.probe
        rng = random.Random(seed)
        game.rng = random.Random(seed + 1)
        kind, smoke, cc_seed = rng.choice((0, 1)), rng.choice((0xCC, 0xCF, 0xD1, rng.randrange(0, 300))), \
            rng.getrandbits(31)
        game.build(kind, smoke, cc_seed)
        got = probe.ask("new %d %d %d 624" % (kind, smoke, cc_seed))
        self.assertEqual(game.read(), got["b"], "case %d built" % seed)
        if rng.random() < 0.7:
            param = rng.choice(PARAMS)
        else:
            param = PARAM
            game.m.mem[PARAM:PARAM + 0x40] = made_up(rng)
        phex = bytes(game.m.mem[param:param + 0x40]).hex()
        pose = [rng.uniform(-3, 3) for _ in range(3)] + [rng.uniform(-2000, 2000) for _ in range(2)] + \
            [rng.uniform(0, 400)]
        cnt, disp, trans = 0, 1, f32(1.0)
        stats = {"draw": 0, "smoke": 0, "set": 0}
        for frame in range(frames):
            want = game.ctrl()
            lands = " ".join("%x" % v for v in game.lands)
            lines = " ".join("%x" % v for v in game.lines)
            got = probe.ask("ctrl %d %s %d %s" % (len(game.lands), lands, len(game.lines), lines))
            self.assertEqual(want[0], got["b"], "case %d frame %d ctrl: %s" % (seed, frame, diffs(want[0], got["b"])))
            self.assertEqual(want[1], got["out"], "case %d frame %d outs: %s"
                             % (seed, frame, diffs(want[1], got["out"])))
            self.assertEqual(want[2], got["cc"], "case %d frame %d ccRand" % (seed, frame))
            for o in want[1]:
                stats[o[0]] += 1
            for i in range(3):
                pose[i] += rng.uniform(-0.2, 0.2)
                pose[3 + i] += rng.uniform(-30, 30)
            node = matrix(*pose[:3], pose[3:])
            cnt = 0 if cnt > 140 or rng.random() < 0.01 else cnt + 1
            if rng.random() < 0.03:
                disp = 1 - disp
            if rng.random() < 0.03:
                trans = rng.choice((f32(1.0), f32(1.0), f32(0.04), f32(0.05), f32(0.5)))
            want = game.set(param, disp, trans, cnt, node)
            got = probe.ask("set %s %d %x %d %s" % (phex, disp, trans, cnt, " ".join("%x" % x for x in node)))
            self.assertEqual(want[0], got["b"], "case %d frame %d set: %s" % (seed, frame, diffs(want[0], got["b"])))
            self.assertEqual(want[1], got["node"], "case %d frame %d node" % (seed, frame))
            stats["set"] += s32(game.m.load(BR, 4))
        return stats

    def test_breaths(self):
        seen = {"draw": 0, "smoke": 0, "set": 0}
        for seed in range(24):
            with self.subTest(case=seed):
                s = self.run_case(3000 + seed)
                for k in seen:
                    seen[k] += s[k]
        print("flames drawn %(draw)d, bursts %(smoke)d" % seen)
        self.assertTrue(seen["draw"] > 0 and seen["smoke"] > 0, seen)


if __name__ == "__main__":
    unittest.main()
