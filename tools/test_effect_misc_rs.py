#!/usr/bin/env python3
"""crates/piney-effect's level up, dying blow, drains and magic portal
against the game's own code run in eemu.

The misc_probe example runs the port on the requests this sends it; the
same requests go through the game's functions on test_effect_rs's
EffectMachine (main and GCMN.PRG in eemu, ccEffectCtrl built by its own
constructor, in a field so every effectTbl row resolves), with what these
effects reach for besides: ccCheckTarget per character and for plw,
condition.dead, effSBL (the words' layer, priority 60), the particle
palettes effDrain swaps in (particleCcsAdrs[104] and [118]), and
ccParticleCtrlAddGenerator recorded like startParticleGenerator.

  - MathAgainstGame: acosf (__ieee754_acosf) and the double sin and cos
    (their kernels and __ieee754_rem_pio2 on the soft-float routines),
    called through a small stub.
  - LevelUpAgainstGame: effLevelUp on characters that move, leave the
    lists or go down; every frame of ccEffectCtrl::Main until all ends.
  - DyingAgainstGame: ccParticleDying.
  - GruntyAgainstGame: effEvolvePG and effGrowPG (a Grunty growing up).
  - DrainAgainstGame: effDrainCtrl (the controller, the word, effDrain's
    homing orbs and their generators), effDrain directly with many orbs,
    the orb's quaternion turn forced through its 180-degree case,
    effProtect and effAfterDrain.
  - PortalAgainstGame: ccMagicCircle made by ccEntryGimCircle over
    XMAGCIR.CCS and run by its own main with what ccEntryObj::routine
    leaves (freeze, dispSW, plDist), through all its acts: its state and
    all 128 sparks' each frame, the draws, sounds, entryCircleObject and
    ccRand's state.

Each frame compares every live slot's whole state, the draws in order,
the events, the generators and rand()'s state. Skipped when the disc is
not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

from test_effect_rs import (EFFWORK, ELF, ISO, ONE, PARTICLE_GENERATOR_TBL, PLAYER, ROOT, EffectMachine,  # noqa: E402
                            compare, eff_chunks, fb, hexs)

TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
PROFILE = os.environ.get("PINEY_PROFILE", "debug")
EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "misc_probe")
EFFSBL, PARTICLE_CCS_ADRS, GAME, SAVEDATA, ENTCTRL, MTI = (inf_va(0x00378AD8), inf_va(0x003E4680), 0x01020000, inf_va(0x003789D8),
                                                         inf_va(0x00378BA8), inf_va(0x00377FD0))
STUB = 0x010F0000


def build():
    args = [shutil.which("cargo"), "build", "-q", "-p", "piney-effect", "--example", "misc_probe"]
    if PROFILE == "release":
        args.append("--release")
    subprocess.run(args, cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class MiscMachine(EffectMachine):
    """EffectMachine with the lists, the words' layer, the drain palettes and
    the portal's surroundings."""

    def __init__(self, town=False, seed=1):
        super().__init__(town=town, seed=seed)
        m, sym = self.m, self.sym
        self.listed, self.player_listed, self.sizes = {}, True, {}
        m.hooks[sym("ccCheckTarget__FP6ccChar")] = self.check_target
        m.hooks[sym("ccCheckObjectSize__FP6ccChar")] = lambda mm, ch, *a: self.sizes.get(self.char_of.get(ch), 0)
        m.hooks[sym("ccParticleCtrlAddGenerator__FP19ccParticleGenerator")] = self.start_generator
        self.layer_pri[m.load(EFFSBL, 4)] = 60
        for i, name in ((104, "CLT_x000c3"), (118, "CLT_x002")):
            m.store(PARTICLE_CCS_ADRS + 4 * i, 4, self.chunk_named(name))

    def check_target(self, m, ch, *a):
        if ch == PLAYER:
            return int(self.player_listed)
        cid = self.char_of.get(ch)
        return int(cid is not None and self.listed.get(cid, True))

    def set_dead(self, cid, v):
        self.m.store(self.chars[cid] + 8, 2, v)

    def set_bounds(self, b):
        from test_effect_rs import WM
        for off, v in zip((0x420, 0x424, 0x428, 0x42C), b):
            self.m.store(WM + off, 4, fb(v))

    def ref(self, p):
        for cid, ch in self.chars.items():
            if p == ch + 0x40:
                return ["pos", cid]
        if EFFWORK <= p < EFFWORK + 0xC0 * 500:
            k, off = divmod(p - EFFWORK, 0xC0)
            return ["effpos", k] if off == 0 else ["?", p]
        return None if p == 0 else ["?", p]

    def start_generator(self, m, g, *a):
        row = (m.load(g, 4) - PARTICLE_GENERATOR_TBL) // 0x38
        sw = m.load(g + 0x20, 4)
        swr = None
        if EFFWORK <= sw < EFFWORK + 0xC0 * 500:
            k, off = divmod(sw - EFFWORK, 0xC0)
            swr = [k, (off - 0xA0) // 4]
        self.gens.append([row, self.ref(m.load(g + 0x18, 4)), (m.load(g + 4, 1) >> 4) & 1, self.rvec(g + 0x80), swr])
        return 0

    def slot(self, i, base=EFFWORK, n=500):
        s = super().slot(i, base, n)
        m = self.m
        a = base + 0xC0 * i
        eff = m.load(a + 0x8C, 4)
        if s["obj"] is not None and s["obj"][0] == "eff":
            clut = m.load(eff + 0x3C, 4)
            s["obj"] = s["obj"] + [self.names.get(clut) if clut else None,
                                   m.load(eff + 0x58, 4) | m.load(eff + 0x5C, 4) << 32]
        lay = m.load(a + 0x9C, 4)
        s["layer"] = None if lay == 0 else self.layer_pri.get(lay, lay)
        if s["id"] == -24:
            s["temp"][0] = self.char_of.get(s["temp"][0], s["temp"][0])
        return s

    def start(self, name, *args):
        self.events, self.gens = [], []
        p = self.call(name, *args)
        return {"slot": -1 if p == 0 else (p - EFFWORK) // 0xC0, "events": self.events, "gens": self.gens}


STATS = {}


def tally(key, n):
    STATS[key] = STATS.get(key, 0) + n


class Case:
    """The same world on both sides: a MiscMachine and the probe's lines."""

    def __init__(self, test, seed, rng):
        self.test, self.rng = test, rng
        self.em = MiscMachine(town=False, seed=seed)
        self.lines = ["reset 0 %x" % seed]
        self.want = []
        self.chars = {}

    def bounds(self, b):
        self.em.set_bounds(b)
        self.lines.append("bounds " + hexs(*map(fb, b)))

    def player(self, p):
        self.em.set_player(p)
        self.lines.append("player " + hexs(*map(fb, p)))
        self.player_pos = p

    def camera(self, eye, view):
        self.em.set_camera(eye, eye, view)
        self.lines.append("camera " + hexs(*map(fb, list(eye) + list(eye) + list(view))))

    def char(self, cid, pos, h, w):
        if cid in self.chars:
            self.em.move_char(cid, pos)
        else:
            self.em.add_char(cid, pos, h, w)
        self.chars[cid] = (list(pos), h, w)
        self.lines.append("char %x " % cid + hexs(*map(fb, list(pos) + [h, w])))

    def listed(self, cid, on):
        self.em.listed[cid] = on
        self.lines.append("listed %x %x" % (cid, int(on)))

    def dead(self, cid, v):
        self.em.set_dead(cid, v)
        self.lines.append("dead %x %x" % (cid, v))

    def size(self, cid, v):
        self.em.sizes[cid] = v
        self.lines.append("size %x %x" % (cid, v))

    def start(self, line, name, *args, void=False):
        """A starter on both sides; a void one's slot is not compared."""
        self.lines.append(line)
        w = self.em.start(name, *args)
        if void:
            w["slot"] = None
        self.want.append(("start", w))

    def frame(self):
        self.lines.append("frame")
        self.want.append(("frame", self.em.frame()))

    def check(self, what):
        got = [g for g in ask(self.lines) if g]
        self.test.assertEqual(len(got), len(self.want))
        frames = 0
        for k, ((kind, w), g) in enumerate(zip(self.want, got)):
            if kind == "start" and w["slot"] is None:
                g["slot"] = None
            compare(self.test, w, g, "%s step %d (%s)" % (what, k, kind))
            frames += kind == "frame"
            tally("draws", len(w.get("draws", [])))
            tally("gens", len(w.get("gens", [])))
            tally("events", len(w.get("events", [])))
            tally("slot-frames", len(w.get("slots", [])))
        return frames


def world(case, rng, n_chars=4):
    """A field with random bounds, a player, characters about him and a
    camera."""
    half = rng.choice([24000.0, 12000.0, 6000.0, 30000.0])
    case.bounds([-half, -half, half, half])
    p = [rng.uniform(-half * 0.9, half * 0.9), rng.uniform(-half * 0.9, half * 0.9), rng.uniform(0, 400)]
    case.player(p)
    for c in range(1, n_chars + 1):
        pos = [p[0] + rng.uniform(-900, 900), p[1] + rng.uniform(-900, 900), p[2] + rng.uniform(-50, 50)]
        case.char(c, pos, rng.choice([160.0, 120.0, 200.0, rng.uniform(40, 600)]), rng.uniform(20, 200))
    new_camera(case, rng)
    return p


def new_camera(case, rng):
    p = case.player_pos
    eye = [p[0] + rng.uniform(-2500, 2500), p[1] + rng.uniform(-2500, 2500), p[2] + rng.uniform(100, 900)]
    if rng.random() < 0.2:
        eye = [p[0] + rng.uniform(-8000, 8000), p[1] + rng.uniform(-8000, 8000), p[2] + rng.uniform(100, 900)]
    view = [p[0] + rng.uniform(-200, 200), p[1] + rng.uniform(-200, 200), p[2] + 100.0]
    if rng.random() < 0.1:
        view = [2 * eye[0] - p[0], 2 * eye[1] - p[1], p[2]]
    case.camera(eye, view)


def wander(case, rng, chance=0.4, step=40.0):
    for cid, (pos, h, w) in list(case.chars.items()):
        if rng.random() < chance:
            case.char(cid, [pos[0] + rng.uniform(-step, step), pos[1] + rng.uniform(-step, step),
                            pos[2] + rng.uniform(-step / 4, step / 4)], h, w)


def run_until_idle(case, rng, limit, jitter=None, after=0):
    """Frames until no slot is live (from frame `after` on)."""
    for f in range(limit):
        if jitter:
            jitter(f)
        case.frame()
        if f >= after and not case.want[-1][1]["slots"]:
            break


def need(test):
    if not (os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo")):
        test.skipTest("needs the extracted disc and cargo")


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class MathAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_acosf_sin_cos(self):
        em = MiscMachine()
        m = em.m
        # stub(x, out): *out = sin(*x) (or cos): ld a0; jal; sd v0.
        for k, fn in enumerate(("sin", "cos")):
            code = [0x27BDFFE0, 0xFFBF0010, 0xFFB00000, 0x00A0802D, 0xDC840000,
                    0x0C000000 | (em.sym(fn) >> 2), 0, 0xFE020000, 0xDFBF0010, 0xDFB00000, 0x03E00008, 0x27BD0020]
            for i, w in enumerate(code):
                m.store(STUB + 0x100 * k + 4 * i, 4, w)
        rng = random.Random(5)
        floats = [0, ONE, 0xBF800000, 0x3F000000, 0xBF000000, 0x3F7FFFFF, 0x3F800001, 0xBF800001, 0x23000000,
                  0x22FFFFFF, 0x3EFFFFFF]
        floats += [fb(rng.uniform(-1, 1)) for _ in range(3000)]
        floats += [fb(rng.uniform(-1e-3, 1e-3)) for _ in range(200)]
        floats += [fb(1 - rng.random() * 1e-5) for _ in range(200)]
        doubles = []
        for _ in range(3000):
            x = rng.uniform(-2.5, 2.5)
            if rng.random() < 0.3:
                x = rng.uniform(-0.8, 0.8)
            if rng.random() < 0.05:
                x = rng.uniform(-60, 60)
            doubles.append(struct.unpack("<Q", struct.pack("<d", float(struct.unpack("<f", struct.pack("<f", x))[0])))[0])
        doubles += [struct.unpack("<Q", struct.pack("<d", v))[0] for v in
                    (0.0, 1e-9, -1e-9, 0.7853981633974483, 0.785398185253143310546875, 1.5707963267948966,
                     1.5707963705062866, 2.356194490192345, -1.5707963705062866, 3.0)]
        lines, want = [], []
        for x in floats:
            em.call("acosf", fargs=(x,))
            want.append(m.f[0])
            lines.append("acosf %x" % x)
        for fn_k, fn in enumerate(("sin", "cos")):
            for d in doubles:
                m.store(STUB + 0x800, 4, d & 0xFFFFFFFF)
                m.store(STUB + 0x804, 4, d >> 32)
                em.call(STUB + 0x100 * fn_k, STUB + 0x800, STUB + 0x810)
                want.append([m.load(STUB + 0x814, 4), m.load(STUB + 0x810, 4)])
                lines.append("%s %x %x" % (fn, d >> 32, d & 0xFFFFFFFF))
        got = [g["v"] for g in ask(lines)]
        bad = [(l, w, g) for l, w, g in zip(lines, want, got) if w != g]
        print("math: %d acosf, %d sin, %d cos: %d mismatches" % (len(floats), len(doubles), len(doubles), len(bad)),
              file=sys.stderr)
        self.assertEqual(bad[:5], [])


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class LevelUpAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def run_case(self, seed):
        rng = random.Random(seed)
        case = Case(self, seed, rng)
        world(case, rng)
        chs = list(case.chars)
        starts = {0: rng.choice(chs)}
        for _ in range(rng.randrange(0, 3)):
            starts[rng.randrange(0, 60)] = rng.choice(chs)

        def jitter(f):
            if f in starts:
                c = starts[f]
                case.start("levelup %x" % c, "effLevelUp__FP6ccChar", case.em.chars[c])
            wander(case, rng)
            if rng.random() < 0.15:
                new_camera(case, rng)
            if rng.random() < 0.02:
                case.listed(rng.choice(chs), rng.random() < 0.5)
            if rng.random() < 0.01:
                case.dead(rng.choice(chs), rng.choice([0, 2, 5, 3]))
        run_until_idle(case, rng, 160, jitter, max(starts))
        return case.check("level up seed %d" % seed)

    def test_level_up(self):
        STATS.clear()
        frames = sum(self.run_case(seed) for seed in range(1, 61))
        print("level up: 60 cases, %d frames" % frames, STATS, file=sys.stderr)
        self.assertGreater(frames, 4000)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DyingAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_dying(self):
        STATS.clear()
        frames = 0
        for seed in range(100, 140):
            rng = random.Random(seed)
            case = Case(self, seed, rng)
            world(case, rng)
            chs = list(case.chars)
            starts = {0: rng.choice(chs), rng.randrange(1, 40): rng.choice(chs)}

            def jitter(f):
                if f in starts:
                    c = starts[f]
                    case.start("dying %x" % c, "ccParticleDying__FP6ccChar", case.em.chars[c], void=True)
                wander(case, rng)
                if rng.random() < 0.2:
                    new_camera(case, rng)
            run_until_idle(case, rng, 120, jitter, max(starts))
            frames += case.check("dying seed %d" % seed)
        print("dying: 40 cases, %d frames" % frames, STATS, file=sys.stderr)
        self.assertGreater(frames, 1500)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class GruntyAgainstGame(unittest.TestCase):
    """effEvolvePG (the -7 controller's generators at counts 12 and 30,
    following the character) and effGrowPG (a generator at its place, z
    half its height), on characters that move."""

    @classmethod
    def setUpClass(cls):
        build()

    def test_grunty(self):
        STATS.clear()
        frames = 0
        for seed in range(200, 230):
            rng = random.Random(seed)
            case = Case(self, seed, rng)
            world(case, rng)
            chs = list(case.chars)
            starts = {0: ("evolvepg", rng.choice(chs))}
            for _ in range(rng.randrange(0, 3)):
                starts[rng.randrange(1, 60)] = (rng.choice(("evolvepg", "growpg")), rng.choice(chs))

            def jitter(f):
                if f in starts:
                    what, c = starts[f]
                    if what == "evolvepg":
                        case.start("evolvepg %x" % c, "effEvolvePG__FP6ccChar", case.em.chars[c])
                    else:
                        case.start("growpg %x" % c, "effGrowPG__FP6ccChar", case.em.chars[c], void=True)
                wander(case, rng)
                if rng.random() < 0.2:
                    new_camera(case, rng)
            run_until_idle(case, rng, 200, jitter, max(starts))
            frames += case.check("grunty seed %d" % seed)
        print("grunty: 30 cases, %d frames" % frames, STATS, file=sys.stderr)
        self.assertGreater(frames, 1500)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DrainAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def drain_case(self, seed):
        rng = random.Random(seed)
        case = Case(self, seed, rng)
        world(case, rng)
        chs = list(case.chars)
        plan = {}
        for _ in range(rng.randrange(1, 4)):
            ap, bp = rng.sample(chs, 2)
            kind = rng.choice([0, 1])
            plan[rng.randrange(0, 30)] = (ap, bp, kind, rng.randrange(1, 12), rng.randrange(1, 12))
        moving = rng.random() < 0.7

        def jitter(f):
            if f in plan:
                ap, bp, kind, time, num = plan[f]
                case.start("drainctrl %x %x %x %x %x" % (ap, bp, kind, time, num),
                           "effDrainCtrl__FP6ccCharP6ccChariii", case.em.chars[ap], case.em.chars[bp], kind, time,
                           num)
            if moving:
                wander(case, rng, 0.5, 60.0)
            if rng.random() < 0.1:
                new_camera(case, rng)
            if rng.random() < 0.01:
                case.listed(rng.choice(chs), rng.random() < 0.6)
        run_until_idle(case, rng, 320, jitter, max(plan))
        return case.check("drain seed %d" % seed)

    def test_drain_ctrl(self):
        STATS.clear()
        frames = sum(self.drain_case(seed) for seed in range(1, 41))
        print("drain: 40 cases, %d frames" % frames, STATS, file=sys.stderr)
        self.assertGreater(frames, 3000)

    def test_many_orbs(self):
        """effDrain itself with many orbs at once, from far and near."""
        STATS.clear()
        frames = 0
        for seed in range(200, 215):
            rng = random.Random(seed)
            case = Case(self, seed, rng)
            world(case, rng)
            ap, bp = rng.sample(list(case.chars), 2)
            if rng.random() < 0.5:
                pos, h, w = case.chars[bp]
                case.char(bp, [pos[0] + rng.uniform(-3000, 3000), pos[1] + rng.uniform(-3000, 3000), pos[2]], h, w)
            case.start("drain %x %x %x %x" % (ap, bp, seed & 1, 30), "effDrain__FP6ccCharP6ccCharii",
                       case.em.chars[ap], case.em.chars[bp], seed & 1, 30)
            run_until_idle(case, rng, 260, lambda f: wander(case, rng, 0.3, 30.0))
            frames += case.check("orbs seed %d" % seed)
        print("orbs: 15 cases, %d frames" % frames, STATS, file=sys.stderr)

    def test_half_turn(self):
        """An orb flying straight away from its target: the quaternion's
        180-degree case (s == 0)."""
        STATS.clear()
        frames = 0
        for seed in range(300, 310):
            rng = random.Random(seed)
            case = Case(self, seed, rng)
            case.bounds([-24000.0, -24000.0, 24000.0, 24000.0])
            case.player([0.0, 0.0, 0.0])
            case.camera([0.0, -1500.0, 800.0], [0.0, 0.0, 100.0])
            y = float(rng.randrange(200, 2000))
            case.char(1, [0.0, y, 0.0], 100.0, 40.0)
            case.char(2, [0.0, 0.0, 0.0], 100.0, 40.0)
            case.start("drain 1 2 %x 1" % (seed & 1), "effDrain__FP6ccCharP6ccCharii", case.em.chars[1],
                       case.em.chars[2], seed & 1, 1)
            slot = case.want[-1][1]["slot"]
            for line, key, v in (("set %x pos 0 0 %x %x" % (slot, fb(50.0), ONE), "pos", [0, 0, fb(50.0), ONE]),
                                 ("set %x rot 0 %x 0 0" % (slot, fb(-1.0)), "rot", [0, fb(-1.0), 0, 0]),
                                 ("set %x cnt 3" % slot, "cnt", [3])):
                case.lines.append(line)
                case.em.set_field(slot, key, *v)
            run_until_idle(case, rng, 250)
            frames += case.check("half turn seed %d" % seed)
        print("half turn: 10 cases, %d frames" % frames, STATS, file=sys.stderr)

    def test_protect_and_after_drain(self):
        STATS.clear()
        frames = 0
        for seed in range(400, 440):
            rng = random.Random(seed)
            case = Case(self, seed, rng)
            world(case, rng)
            chs = list(case.chars)
            for c in chs:
                case.size(c, rng.choice([1, 3, 4]))
            plan = {rng.randrange(0, 20): ("protect", rng.choice(chs), rng.choice([0, 1]), rng.choice([0, 1, 2, -1]))
                    for _ in range(2)}
            plan[rng.randrange(0, 20)] = ("afterdrain", rng.choice(chs), rng.choice([0, 1, 2, -1]))

            def jitter(f):
                if f in plan:
                    p = plan[f]
                    if p[0] == "protect":
                        case.start("protect %x %x %x" % (p[1], p[2], p[3] & 0xFFFFFFFF), "effProtect__FP6ccCharii",
                                   case.em.chars[p[1]], p[2], p[3] & 0xFFFFFFFF)
                    else:
                        case.start("afterdrain %x %x" % (p[1], p[2] & 0xFFFFFFFF), "effAfterDrain__FP6ccChari",
                                   case.em.chars[p[1]], p[2] & 0xFFFFFFFF)
                wander(case, rng, 0.5, 50.0)
                if rng.random() < 0.15:
                    new_camera(case, rng)
            run_until_idle(case, rng, 300, jitter, max(plan))
            frames += case.check("protect seed %d" % seed)
        print("protect, after drain: 40 cases, %d frames" % frames, STATS, file=sys.stderr)
        self.assertGreater(frames, 1500)


class CircleMachine(MiscMachine):
    """A MiscMachine that also holds XMAGCIR.CCS and a ccMagicCircle made by
    ccEntryGimCircle, with the entry control, the save data and the portal's
    calls out recorded."""

    def __init__(self, seed=1):
        super().__init__(town=False, seed=seed)
        import anim
        import ccs
        import gzarc
        from test_effect_rs import DATA
        m, sym = self.m, self.sym
        data = gzarc.open_bytes(DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        c = ccs.Ccs(gzarc.inflate(data, members["xmagcir.cmp"]))
        self.ccs["xmagcir"] = c
        for _, a in anim.animations(c):
            self.anims.setdefault(c.objects[a.object][0], a)
        self.effs.update(eff_chunks(c))
        nop = lambda mm, *a: 0                     # noqa: E731
        for name in ("ccEntryChangeCLUT__FP7ccEntryP7ccClump", "ApplyClump__5ccAnmFP7ccClumpP8ccStream",
                     "deleteCmnd__10ccEntryObjFi"):
            m.hooks[sym(name)] = nop
        self.out = []
        m.hooks[sym("entryCircleObject__11ccEntryCtrlFP10ccEntryObj")] = lambda mm, *a: self.out.append(["entry"]) or 0
        m.hooks[sym("ccStartThread__FPFPv_vii")] = lambda mm, *a: self.out.append(["thread"]) or 0
        m.hooks[sym("GetFieldType__9WORLD_MANFv")] = lambda mm, *a: 4
        m.hooks[sym("ccSeOn3D__FiPf")] = lambda mm, n, pos, *a: self.out.append(["se3d", n, self.rvec(pos)]) or 0
        self.save = self.malloc(m, 0x8000)
        m.store(SAVEDATA, 4, self.save)
        self.ent_ctrl = self.malloc(m, 0x100)
        m.store(ENTCTRL, 4, self.ent_ctrl)
        m.store(self.ent_ctrl + 0x1C, 4, 1)
        m.store(GAME + 0x14, 4, 2)

    def make(self, pos, dirc):
        m = self.m
        ep = self.malloc(m, 0x60)
        self.vec(ep, [fb(v) for v in pos] + [ONE])
        self.vec(ep + 0x10, [fb(v) for v in dirc] + [0])
        m.store(ep + 0x24, 4, 15)
        m.store(ep + 0x40, 4, 1)
        entry = inf_va(0x0061E7A8)
        m.store(entry + 0x0C, 4, ep)
        m.store(entry + 0x10, 4, 0x10000 + 1)
        self.obj = self.call("ccEntryGimCircle__FP7ccEntry", entry)
        return self.state()

    def state(self):
        m, o = self.m, self.obj
        bits = m.load(o + 0xE0, 1)
        part_flag = m.load(o + 0x1E0, 1) & 1
        anm = m.load(o + 0xD4, 4)
        parts = []
        for k in range(128):
            p = o + 0x1F0 + 0x70 * k
            eff = m.load(p + 0x68, 4)
            e = None
            if part_flag and eff:
                e = [self.rvec(eff + 0x10), m.load(eff + 0x20, 4), m.load(eff + 0x24, 4), m.load(eff + 0x28, 4),
                     m.load(eff + 0x2C, 4), m.load(eff + 0x34, 4)]
            parts.append([m.load(p + 4, 4, True), m.load(p, 1) & 1, self.rvec(p + 0x10, 16), m.load(p + 0x50, 4),
                          m.load(p + 0x54, 4), m.load(p + 0x58, 2, True), m.load(p + 0x5A, 2),
                          m.load(p + 0x5C, 2, True), m.load(p + 0x5E, 2, True), m.load(p + 0x60, 4),
                          m.load(p + 0x64, 4), e])
        return {"act": m.load(o + 0x1D4, 4, True), "actCnt": m.load(o + 0x1D8, 4, True),
                "anmFlag": m.load(o + 0x1DC, 4), "dest": (bits >> 5) & 1, "disp": (bits >> 4) & 1,
                "partFlag": part_flag, "partTop": m.load(o + 0x1E4, 4, True), "pos": self.rvec(o + 0x40),
                "posP": self.rvec(o + 0x50), "dirc": self.rvec(o + 0x60), "time": self.anm[anm][1], "parts": parts}

    def cframe(self, freeze, disp, pl_dist, set_t, listed, ent_root):
        m, o = self.m, self.obj
        bits = m.load(o + 0xE0, 1) & ~0x14
        m.store(o + 0xE0, 1, bits | (4 if freeze else 0) | (16 if disp else 0))
        m.store(o + 0xE4, 4, pl_dist)
        m.store(o + 0x8C, 4, set_t)
        m.store(o + 0x140, 4, ent_root)
        self.player_listed = bool(listed)
        self.draws, self.out = [], []
        r = self.call("main__13ccMagicCircleFv", o) & 0xFF
        events = []
        for e in self.out:
            if e[0] == "thread":
                continue
            events.append(e)
        if r and ent_root:
            events.append(["opened"])
        return {"state": self.state(), "draws": self.draws, "events": events, "delete": r,
                "mti": m.load(MTI, 4)}


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class PortalAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def run_case(self, seed):
        rng = random.Random(seed)
        em = CircleMachine(seed=seed)
        lines = ["reset 0 %x" % seed]
        half = rng.choice([24000.0, 8000.0, 30000.0])
        b = [-half, -half, half, half]
        em.set_bounds(b)
        lines.append("bounds " + hexs(*map(fb, b)))
        player = [rng.uniform(-half * 0.9, half * 0.9), rng.uniform(-half * 0.9, half * 0.9), rng.uniform(0, 300)]
        em.set_player(player)
        lines.append("player " + hexs(*map(fb, player)))
        warm = rng.randrange(0, 700)
        for _ in range(warm):
            em.call("genrand__Fv")
        lines.append("mt %x" % warm)
        pos = [player[0] + rng.uniform(-4000, 4000), player[1] + rng.uniform(-4000, 4000), rng.uniform(0, 500)]
        dirc = [rng.uniform(-0.3, 0.3), rng.uniform(-0.3, 0.3), rng.uniform(-3.1, 3.1)]
        want = [{"state": em.make(pos, dirc)}]
        lines.append("circle " + hexs(*map(fb, pos + dirc)))

        def camera():
            eye = [player[0] + rng.uniform(-3000, 3000), player[1] + rng.uniform(-3000, 3000), rng.uniform(100, 900)]
            if rng.random() < 0.3:
                eye = [pos[0] + rng.uniform(-250, 250), pos[1] + rng.uniform(-250, 250), pos[2] + rng.uniform(0, 200)]
            if rng.random() < 0.2:
                eye = [pos[0] + rng.uniform(-9000, 9000), pos[1] + rng.uniform(-9000, 9000), 500.0]
            view = [pos[0] + rng.uniform(-300, 300), pos[1] + rng.uniform(-300, 300), pos[2]]
            if rng.random() < 0.15:
                view = [2 * eye[0] - pos[0], 2 * eye[1] - pos[1], pos[2]]
            em.set_camera(eye, eye, view)
            lines.append("camera " + hexs(*map(fb, eye + eye + view)))

        camera()
        dist = rng.uniform(2000, 12000)
        set_t = ONE
        ent_root = rng.choice([0, 1, 1])
        for f in range(400):
            if rng.random() < 0.1:
                camera()
            if rng.random() < 0.3:
                pl = [player[0] + rng.uniform(-30, 30), player[1] + rng.uniform(-30, 30), player[2]]
                em.set_player(pl)
                lines.append("player " + hexs(*map(fb, pl)))
            dist = max(0.0, dist + rng.uniform(-300, 120))
            if rng.random() < 0.05:
                dist = rng.uniform(0, 12000)
            if rng.random() < 0.05:
                set_t = fb(rng.random())
            freeze = dist > 10000
            disp = dist <= 7000 and rng.random() < 0.95
            listed = rng.random() < 0.9
            args = (int(freeze), int(disp), fb(dist), set_t, int(listed), ent_root)
            lines.append("cframe " + hexs(*args))
            want.append(em.cframe(*args))
            if want[-1]["delete"]:
                break
        got = [g for g in ask(lines) if g]
        self.assertEqual(len(got), len(want))
        draws = 0
        for k, (w, g) in enumerate(zip(want, got)):
            if w["state"] != g["state"]:
                ws, gs = w["state"], g["state"]
                diff = {kk: (ws[kk], gs.get(kk)) for kk in ws if kk != "parts" and ws[kk] != gs.get(kk)}
                pd = [(i, a, b2) for i, (a, b2) in enumerate(zip(ws["parts"], gs["parts"])) if a != b2]
                self.fail("portal seed %d frame %d: %s parts %s" % (seed, k, diff, pd[:2]))
            compare(self, w, g, "portal seed %d frame %d" % (seed, k))
            draws += len(w.get("draws", []))
        return len(want) - 1, draws, want[-1].get("delete", 0)

    def test_portal(self):
        frames = draws = deleted = 0
        for seed in range(1, 41):
            f, d, x = self.run_case(seed)
            frames, draws, deleted = frames + f, draws + d, deleted + x
        print("portal: 40 cases, %d frames, %d draws, %d opened to the end" % (frames, draws, deleted),
              file=sys.stderr)
        self.assertGreater(deleted, 10)


if __name__ == "__main__":
    unittest.main()
