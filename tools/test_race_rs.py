#!/usr/bin/env python3
"""piney_world::race (the Flag Race, from Mutation on: MUT gcmn race.cpp)
against the game's own code run in tools/eemu.py (the Rust machine,
crates/piney-eemu, when built) over test_battle's GameBattle (GCMN.PRG
loaded), each case through the race_probe example too.

The race's functions have no names; they are found from the task's: the
string PG_RACE names the task (0x005ff680 in Mutation), whose second
unnamed call is the race's main loop (0x005fded0); the main loop's are the
set-up, the countdown (0x005fdd10), the place before the camera
(0x005ff880), the finish (0x005fe290), the quit, the HUD (0x005fee10); the
finish's second is the rank (0x005fe830), whose first is main's
0x0017a860, the time into the save; the HUD's first is the split
(0x005ff3d0). The flags' main (0x005fce60) is their vtable's third slot,
the vtable their constructor (the function naming ANM_xpflnut0) stores.

Checks:
  split     0x005ff3d0 over times 0-32767 and beyond
  enter     0x0017a860: a time into a town's three ranks (the save's
            records, the town's racers where they are 0), the rank back
  hud       0x005fee10: the timer, a flag newly taken's split, the stop at
            three or at the limit; every MakePacket (its mask's texture,
            cell, place, size, texels and grid) in order
  count     0x005fdd10's count: the start (Kite let go, the field camera,
            the map, music 0), the numbers' sounds, the clip's place
  camera    0x005ff880 on random camera turns and places
  flag      0x005fce60: taken within 200 (sound 240, the race's order, the
            sparks, the fade), the map mark's fade, the draw within 8500
            (the draw environment's alpha and shadow length, the clip's)

Skipped when the disc is not extracted, the volume is Infection (no race)
or cargo is missing.
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
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "race_probe")

BASE = 0x01200000
RACE = BASE                 # the race object (0xb0)
MASKS = BASE + 0x400        # its seven ccMasks (0xd8 each)
FLAG = BASE + 0x1000        # a flag (0x200) and its base, anm
FLAG_BASE, FLAG_ANM = BASE + 0x1400, BASE + 0x1500
ANM = BASE + 0x1800         # the countdown's ccAnm
CAM = BASE + 0x2000         # cameraGetRot's and cameraGetPos's answers
DRAWENV = BASE + 0x3000
ONE = 0x3F800000
# The masks' slots in the object: the timer, the three flags, the splits.
MASK_SLOTS = (0x34, 0x38, 0x3C, 0x40, 0x44, 0x48, 0x4C)
TEXTURE = {0x34: 0, 0x38: 1, 0x3C: 2, 0x40: 3, 0x44: 0, 0x48: 0, 0x4C: 0}


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def bf(v):
    return struct.unpack("<f", struct.pack("<I", v & 0xFFFFFFFF))[0]


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v >> 15 else v


def s8(v):
    v &= 0xFF
    return v - 0x100 if v >> 7 else v


class Code:
    """The race's functions in the volume, found as the docstring says."""

    def __init__(self, prog):
        self.p = prog
        name = self.string(b"PG_RACE\0")
        thread = self.start_of(self.pair(name, 2))
        main = self.unnamed(thread)[1]
        mains = self.unnamed(main)
        self.countdown, self.before, self.finish, self.hud = mains[1], mains[2], mains[3], mains[5]
        self.rank = self.unnamed(self.finish)[1]
        self.enter = self.unnamed(self.rank)[0]
        self.split = self.unnamed(self.hud)[0]
        ctor = self.start_of(self.pair(self.string(b"ANM_xpflnut0\0"), 5))
        # The vtable: `lui r; addiu r, r; sw r, 0x1cc(this)` in the ctor.
        words = [w for _, w in self.words(ctor)]
        vt = None
        for k in range(len(words) - 4):
            w, w2 = words[k], words[k + 1]
            rt = (w >> 16) & 31
            if w >> 26 == 0x0F and w2 >> 26 == 0x09 and (w2 >> 21) & 31 == rt and (w2 >> 16) & 31 == rt and any(
                    x >> 26 == 0x2B and (x >> 16) & 31 == rt and x & 0xFFFF == 0x1CC for x in words[k + 2:k + 5]):
                lo = w2 & 0xFFFF
                vt = ((w & 0xFFFF) << 16) + (lo - 0x10000 if lo & 0x8000 else lo)
                break
        assert vt is not None, "the flag's vtable"
        self.flag_main = prog.u32(vt + 8)
        # The race's object pointer: the task stores the new object there.
        store = next(w for _, w in self.words(thread) if w >> 26 == 0x2B and (w >> 21) & 31 == 28)
        imm = store & 0xFFFF
        self.race_ptr = (prog.gp + (imm - 0x10000 if imm & 0x8000 else imm)) & 0xFFFFFFFF

    def string(self, s):
        ov = self.p.overlay
        i = bytes(ov.data).find(s)
        assert i >= 0, s
        return ov.base + i

    def all_words(self):
        ov = self.p.overlay
        n = len(ov.data) // 4
        return ov.base, struct.unpack_from("<%dI" % n, bytes(ov.data))

    def pair(self, va, reg):
        """The address of `lui reg, hi(va); addiu reg, reg, lo(va)`."""
        base, words = self.all_words()
        hi = ((va + 0x8000) >> 16) & 0xFFFF
        lui, addiu = 0x3C000000 | reg << 16 | hi, 0x24000000 | reg << 21 | reg << 16 | (va & 0xFFFF)
        k = next(k for k in range(len(words) - 1) if words[k] == lui and words[k + 1] == addiu)
        return base + 4 * k

    def start_of(self, a):
        while not (self.p.u32(a) >> 16 == 0x27BD and self.p.u32(a) & 0x8000):
            a -= 4
        return a

    def words(self, start):
        out, a = [], start
        while True:
            w = self.p.u32(a)
            out.append((a, w))
            a += 4
            if w == 0x03E00008:
                out.append((a, self.p.u32(a)))
                return out

    def unnamed(self, start):
        """The functions `start` calls that have no symbol of their own,
        each once, in order."""
        out = []
        for a, w in self.words(start):
            if w >> 26 != 3:
                continue
            t = ((w & 0x3FFFFFF) << 2) | (a & 0xF0000000)
            hit = self.p.symbol_at(t)
            if (hit is None or hit[1] != 0) and t not in out:
                out.append(t)
        return out


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)

    def ask(self, words):
        self.p.stdin.write(" ".join(str(w) for w in words) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


class RaceGame:
    def __init__(self):
        import battle
        import test_battle as tb
        self.g = tb.GameBattle(battle.Data(ELF))
        self.m, self.sym = self.g.m, self.g.sym
        self.code = Code(self.g.prog)
        m = self.m
        # The battle harness's own sceVu0CopyVector stand-in: the game's.
        m.hooks.pop(self.sym("sceVu0CopyVector"), None)
        m.mem[BASE:BASE + 0x10000] = bytes(0x10000)
        m.store(self.sym("active__9ccDrawEnv"), 4, DRAWENV)
        self.calls = []

    def vec(self, a):
        return [self.m.load(a + 4 * i, 4) for i in range(4)]

    def put_vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    def run(self, addr, args, hooks):
        m = self.m
        saved = {}
        for name, f in hooks.items():
            a = name if isinstance(name, int) else self.sym(name)
            saved[a] = m.hooks.get(a)
            m.hooks[a] = f
        try:
            return m.call(addr, list(args), limit=50_000_000)
        finally:
            for a, f in saved.items():
                if f is None:
                    m.hooks.pop(a, None)
                else:
                    m.hooks[a] = f


def rnd_time(rnd):
    return rnd.choice([rnd.randrange(0, 18001), rnd.randrange(0, 32768), rnd.choice([0, 29, 30, 1799, 1800,
                                                                                      18000, 18001, 32767])])


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
@unittest.skipIf(volume.NAME == "infection", "Mutation on: the Flag Race")
class RaceAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                        "race_probe"], cwd=ROOT, check=True)
        cls.game = RaceGame()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def test_split(self):
        g, m = self.game, self.game.m
        rnd = random.Random(1)
        out = BASE + 0x8000
        for _ in range(400):
            t = rnd_time(rnd)
            g.run(g.code.split, [out, RACE, t], {})
            want = [s8(m.load(out + k, 1)) for k in range(4)]
            self.assertEqual(want, self.probe.ask(["split", s16(t)]), t)

    def test_enter(self):
        g, m = self.game, self.game.m
        rnd = random.Random(2)
        import test_battle as tb
        save = tb.SAVE
        for _ in range(300):
            server = rnd.randrange(1, 5)
            recs = []
            for k in range(3):
                t = rnd.choice([0, 0, rnd.randrange(1, 4000), rnd.randrange(1, 18001)])
                recs += [t, rnd.randrange(145, 154) if t else 0]
            time, row = rnd.randrange(1, 4000), rnd.randrange(145, 154)
            at = save + 0x8432 + 12 * (server - 1)
            for k, v in enumerate(recs):
                m.store(at + 2 * k, 2, v & 0xFFFF)
            rank = s32(g.run(g.code.enter, [save, server, time, row], {}))
            want = {"rank": rank, "recs": [s16(m.load(at + 2 * k, 2)) for k in range(6)]}
            got = self.probe.ask(["enter", server, time, row] + recs)
            self.assertEqual(want, got, (server, recs, time, row))

    def test_hud(self):
        g, m = self.game, self.game.m
        rnd = random.Random(3)
        masks = {}
        for k, slot in enumerate(MASK_SLOTS):
            a = MASKS + 0xE0 * k
            m.store(RACE + slot, 4, a)
            masks[a] = slot
        for _ in range(300):
            time = rnd.choice([rnd_time(rnd), 17999, 18000])
            splits = [rnd_time(rnd) for _ in range(3)]
            digits = [rnd.randrange(-2, 100) for _ in range(4)]
            taken = rnd.randrange(0, 4)
            order = [rnd.choice([-1, 0, 1, 2]) for _ in range(3)]
            for k in range(taken):
                order[k] = rnd.randrange(3)
            state, running = rnd.randrange(0, 3), rnd.choice([0, 1, 1])
            m.store(RACE + 0x90, 2, time & 0xFFFF)
            for k, v in enumerate(splits):
                m.store(RACE + 0x92 + 2 * k, 2, v & 0xFFFF)
            for k, v in enumerate(digits):
                m.store(RACE + 0x9A + k, 1, v & 0xFF)
            m.store(RACE + 0x9E, 1, taken)
            for k, v in enumerate(order):
                m.store(RACE + 0x9F + k, 1, v & 0xFF)
            m.store(RACE + 0xA3, 1, state)
            m.store(RACE + 0xA4, 1, running)
            cells = []

            def packet(mm, mask, code, flag, *_):
                f = lambda o: bf(mm.load(mask + o, 4))  # noqa: E731
                w = lambda o: s32(mm.load(mask + o, 4))  # noqa: E731
                cells.append([TEXTURE[masks[mask]], s32(code), f(0x40), f(0x44), f(0x48), f(0x4C), w(0x50), w(0x54),
                              w(0x58), w(0x5C), w(0x60)])
                return 0
            g.run(g.code.hud, [RACE], {"MakePacket__8ccSpriteFii": packet,
                                         "SendPacketS__6ccMaskFv": lambda mm, *_: 0})
            want = [s16(m.load(RACE + 0x90, 2))] + [s16(m.load(RACE + 0x92 + 2 * k, 2)) for k in range(3)]
            want += [s8(m.load(RACE + 0x9A + k, 1)) for k in range(4)] + [s8(m.load(RACE + 0x9E, 1))]
            want += [s8(m.load(RACE + 0x9F + k, 1)) for k in range(3)]
            want += [s8(m.load(RACE + 0xA3, 1)), m.load(RACE + 0xA4, 1)]
            got = self.probe.ask(["hud", s16(time)] + [s16(v) for v in splits] + digits + [taken] + order
                                 + [state, running])
            self.assertEqual(want, got["race"], (time, splits, taken, order))
            # The port's cells, with the grid each texture's masks have
            # (8 cells a row on the timer's, 1 on a flag's).
            mine = [c[:8] + [0, 0, 8 if c[0] == 0 else 1] for c in got["cells"]]
            self.assertEqual(cells, mine, (time, splits, taken, order))

    def test_count(self):
        g, m = self.game, self.game.m
        rnd = random.Random(4)
        m.store(RACE + 0x08, 4, ANM)
        m.store(ANM + 0xAC, 4, 1)
        plw = self.game.sym("plw")
        menu = m.load(self.game.sym("ccMenu"), 4)
        for _ in range(200):
            tick, count = rnd.randrange(0, 31), rnd.choice([0, 1, 2, 3, 4])
            running, phase = rnd.choice([0, 0, 1]), 0
            server = rnd.randrange(1, 5)
            rot = [fb(rnd.uniform(-1.4, 0.2)), fb(rnd.uniform(-1, 1)), fb(rnd.uniform(-3.1, 3.1)), fb(rnd.uniform(-1, 1))]
            eye = [fb(rnd.uniform(-9000, 9000)), fb(rnd.uniform(-9000, 9000)), fb(rnd.uniform(0, 900)), ONE]
            m.store(RACE + 0x8C, 2, tick)
            m.store(RACE + 0x8E, 2, count)
            m.store(RACE + 0xA4, 1, running)
            m.store(RACE + 0x80, 4, phase)
            m.store(RACE + 0x7C, 4, server)
            m.store(plw, 1, 1)
            m.store(menu + 0x10, 2, 3)
            calls = []
            rec = calls.append

            def get_rot(mm, out, n, *_):
                for k in range(4):
                    if k != 1 and k != 3:
                        mm.store(out + 4 * k, 4, rot[k])
                return 0

            def get_pos(mm, out, n, *_):
                g.put_vec(out, eye)
                return 0

            def matrix(mm, anm, pos, r, *_):
                rec(["matrix", g.vec(pos), g.vec(r)])
                return 0
            hooks = {
                "changeCamera__Fi": lambda mm, n, *_: rec(["camera", s32(n)]) or 0,
                "ccBgmPlay__Fi": lambda mm, n, *_: rec(["bgm", s32(n)]) or 0,
                "ccSeOn__Fi": lambda mm, n, *_: rec(["se", s32(n)]) or 0,
                "cameraGetRot__FPfi": get_rot,
                "cameraGetPos__FPfi": get_pos,
                "_AnimateForward__5ccAnmFUi": lambda mm, *_: rec(["forward"]) or 0,
                "SetMatrix_PosRotZYX__7ccCoordFPfPf": matrix,
                "Draw__5ccAnmFv": lambda mm, *_: rec(["draw"]) or 0,
                "SetActiveLayer__9WORLD_MANFi": lambda mm, w, n, *_: rec(["layer", s32(n)]) or 0,
            }
            g.run(g.code.countdown, [RACE], hooks)
            race = [s16(m.load(RACE + 0x8C, 2)), s16(m.load(RACE + 0x8E, 2)), m.load(RACE + 0xA4, 1),
                    s32(m.load(RACE + 0x80, 4))]
            got = self.probe.ask(["count", tick, count, running, phase])
            self.assertEqual(race, got["race"], (tick, count, running))
            start = got["start"] == 1
            want = [["camera", 1], ["bgm", 0]] if start else []
            want += [e for e in got["ev"] if e[0] == "se"]
            # The clip at the place before the camera: the port's 0x005ff880.
            p = self.probe.ask(["cam"] + [x if k in (0, 2) else 0 for k, x in enumerate(rot)] + eye
                               + [fb(-800.0), COUNT_HEIGHT[server]])
            want += [["forward"], ["matrix", p["pos"], p["rot"]], ["draw"], ["layer", 0]]
            self.assertEqual(calls, want, (tick, count, running))
            self.assertEqual(m.load(plw, 1) & 1, 0 if start else 1)
            self.assertEqual(m.load(menu + 0x10, 2), 1 if start else 3)

    def test_camera(self):
        rnd = random.Random(5)
        g, m = self.game, self.game.m
        pos, rot = BASE + 0x8000, BASE + 0x8010
        for _ in range(300):
            r = [fb(rnd.uniform(-3.2, 3.2)), fb(rnd.uniform(-1, 1)), fb(rnd.uniform(-3.2, 3.2)), fb(rnd.uniform(-1, 1))]
            eye = [fb(rnd.uniform(-24000, 24000)), fb(rnd.uniform(-24000, 24000)), fb(rnd.uniform(-500, 2000)), ONE]
            dist, height = fb(rnd.choice([-800.0, -500.0, rnd.uniform(-2000, 2000)])), rnd.choice(
                [0, fb(rnd.uniform(-300, 300)), fb(220.0)])

            def get_rot(mm, out, n, *_):
                g.put_vec(out, r)
                return 0
            m.f[12], m.f[13] = dist, height
            g.run(g.code.before, [pos, rot], {"cameraGetRot__FPfi": get_rot,
                                               "cameraGetPos__FPfi": lambda mm, out, n, *_: g.put_vec(out, eye) or 0})
            got = self.probe.ask(["cam"] + r + eye + [dist, height])
            self.assertEqual({"pos": g.vec(pos), "rot": g.vec(rot)}, got, (r, eye, dist, height))

    def test_flag(self):
        g, m = self.game, self.game.m
        rnd = random.Random(6)
        m.store(FLAG, 4, FLAG_BASE)
        m.store(FLAG + 0xD4, 4, FLAG_ANM)
        m.store(FLAG_ANM + 0xAC, 4, 1)
        m.store(FLAG_ANM + 0x9C, 2, 256)
        ptr = g.code.race_ptr
        for _ in range(400):
            n = rnd.randrange(3)
            pos = [fb(rnd.uniform(-9000, 9000)), fb(rnd.uniform(-9000, 9000)), fb(rnd.uniform(0, 100)), ONE]
            height = fb(rnd.choice([200.0, rnd.uniform(0, 400)]))
            state = rnd.choice([0, 0, 1, 2])
            cnt = rnd.choice([0, rnd.randrange(0, 20), 15, 16])
            alpha = fb(rnd.choice([1.0, rnd.uniform(-0.1, 1.0), 0.05]))
            taken = rnd.randrange(2)
            dist = fb(rnd.choice([rnd.uniform(0, 400), rnd.uniform(0, 9000), 199.99, 200.0, 8499.0, 8500.0]))
            sett = fb(rnd.uniform(0, 1))
            fflag, fcnt = rnd.choice([0, 1, 2]), rnd.choice([5, 15])
            fade = fb(rnd.uniform(0, 1))
            race_taken = rnd.randrange(0, 3)
            order = [rnd.choice([-1, 0, 1, 2]) for _ in range(3)]
            with_race = rnd.random() < 0.8
            g.put_vec(FLAG + 0x40, pos)
            m.store(FLAG_BASE + 0x18, 4, height)
            m.store(FLAG + 0x1E0, 4, n)
            m.store(FLAG + 0x1D4, 4, state)
            m.store(FLAG + 0x1E8, 4, cnt)
            m.store(FLAG + 0x1EC, 4, alpha)
            m.store(FLAG + 0x1F0, 1, taken)
            m.store(FLAG + 0xE4, 4, dist)
            m.store(FLAG + 0x8C, 4, sett)
            m.store(FLAG + 0xF0, 2, fflag)
            m.store(FLAG + 0xF2, 2, fcnt)
            m.store(RACE + 0x9E, 1, race_taken)
            for k, v in enumerate(order):
                m.store(RACE + 0x9F + k, 1, v & 0xFF)
            m.store(ptr, 4, RACE if with_race else 0)
            calls = []
            rec = calls.append

            def trans(mm, p, *_):
                rec(["trans", g.vec(p), mm.f[12], mm.f[13], mm.f[14], mm.f[15]])
                mm.f[0] = fade
                return fade

            def draw(mm, anm, *_):
                rec(["draw", mm.load(DRAWENV + 0xA4, 1), mm.load(DRAWENV + 0xA0, 4), mm.load(anm + 0x88, 4)])
                return 0
            hooks = {
                "ccTransPosW2P__FPfPf": lambda mm, *_: 0,
                "ccSeOn3D__FiPf": lambda mm, se, p, *_: rec(["se3d", s32(se), g.vec(p)]) or 0,
                "effOpenBox__FPf": lambda mm, p, *_: rec(["open_box", g.vec(p)]) or 0,
                "_AnimateForward__5ccAnmFUi": lambda mm, a, spd, *_: rec(["forward", spd]) or 0,
                "SetMatrix_PosRotZYX__7ccCoordFPfPf": lambda mm, a, p, r, *_: rec(["matrix", g.vec(p)]) or 0,
                "ccGetCameraTransparency__FPfffff": trans,
                "Draw__5ccAnmFv": draw,
                "SetActiveLayer__9WORLD_MANFi": lambda mm, w, k, *_: rec(["layer", s32(k)]) or 0,
            }
            g.run(g.code.flag_main, [FLAG], hooks)
            flag = [s32(m.load(FLAG + 0x1D4, 4)), s32(m.load(FLAG + 0x1E8, 4)), m.load(FLAG + 0x1EC, 4),
                    m.load(FLAG + 0x1F0, 1), s16(m.load(FLAG + 0xF0, 2)), s16(m.load(FLAG + 0xF2, 2))]
            got = self.probe.ask(["flag", n] + pos + [height, state, cnt, alpha, taken, dist, sett, fflag, fcnt, fade,
                                                      race_taken] + order)
            what = (state, cnt, bf(dist), taken)
            self.assertEqual(flag, got["flag"], what)
            if with_race:
                self.assertEqual([s8(m.load(RACE + 0x9F + k, 1)) for k in range(3)], got["order"], what)
            want = []
            if got["taken"]:
                want += [["se3d", 240, pos], ["open_box", pos]]
            if got["drawn"] is not None:
                alpha, length, t = got["drawn"]
                want += [["forward", 256], ["matrix", pos], ["trans", pos, 0, 0, fb(7000.0), fb(600.0)],
                         ["layer", 5], ["matrix", pos], ["draw", alpha, length, t], ["layer", 0]]
            self.assertEqual(calls, want, what)


COUNT_HEIGHT = {}


def _count_heights():
    """count_height (0x006d8280 in Mutation): the countdown's first lwc1
    from a lui/addiu table, read for servers 0-4."""
    from image import Program
    p = Program(ELF, "gcmn")
    code = Code(p)
    words = code.words(code.countdown)
    for k in range(len(words) - 1):
        a, w = words[k]
        if w >> 16 == 0x3C02 and words[k + 1][1] >> 16 == 0x2442:
            lo = words[k + 1][1] & 0xFFFF
            tbl = ((w & 0xFFFF) << 16) + (lo - 0x10000 if lo & 0x8000 else lo)
            for s in range(5):
                COUNT_HEIGHT[s] = p.u32(tbl + 4 * s)
            return


if os.path.exists(ELF) and volume.NAME != "infection":
    _count_heights()


if __name__ == "__main__":
    unittest.main()
