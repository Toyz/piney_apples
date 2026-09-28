#!/usr/bin/env python3
"""crates/piney-world's minimap (map/) against the game's own map code run
in tools/eemu.py (the Rust eemu_rs.so when present).

The map_probe example draws the port's maps on the requests below; the
game's side runs the real functions over the same state laid out in the
interpreter's memory, frame by frame:

  - ROOTTOWN01::DrawMap (gcmn 0x00422d00) over ROOTTOWN01's three ccMasks
    as its constructor makes them (ccMask::ccMask run natively, the
    mapLayer's view by the real ccLayer::Init and ccView::SetFrame), the
    real RT01ICONPOS carried from frame to frame, WORLD_MAN's townMapMode
    and mapAlpha, and the player at places all over Mac Anu (the map's top
    and bottom stops, the signs fading in and out) with random headings;
  - WORLD::DrawMiniMap (gcmn 0x005a3940) and its six helpers over a WORLD
    laid out as WORLD::Generate leaves it for story area 14's field and
    random fields (fobj2[] and fobj[40][40] from the port's generation,
    which tools/test_field_rs.py checks against the game), each map mode,
    the dungeon entrance's skip list, the fountain and the magic portals
    (fading ones too) on the entry control's lists;
  - WORLD::Generate's painting of the height map into fieldminimap[type]:
    its loop (gcmn 0x005a7c3c - 0x005a7d1c) run natively over the port's
    heights in a FIELD, texel for texel;
  - DUNGEON::MakeMiniMap (gcmn 0x005cd850) over the rooms the real
    DUNGEON::Generate and SetRoom build (tools/test_dungeon_rt.py's Game),
    rays through the real ccHitCheckLM: smallmap square for square; and
    DUNGEON::DrawMap (0x005cc160) over that state with portals, gimmicks,
    the stairs, a fountain dungeon and mapHideFlag's repaint;
  - WORLD_MAN::ChangeMapMode (main 0x001a3fd0) for each area and mode, and
    WORLD_MAN::SetMapAlpha / GetMapAlpha;
  - every packet's vertices: ccSprite::MakePacketStr (main 0x0015aed0) run
    natively on each packet the game queued (the TF sprites and the
    turning TRF ones, mirrored and upside down) through the real layer
    views, its GIF packet read back - XYZ2, UV and RGBAQ - beside the
    corners the port's map::sprite::corners computes.

Compared: every MakePacketStr call (which sprite, whether through
ccMask::MakePacketS, the cell, transp, centre, rotation, place, size,
grid, colour and alpha, ctrl), every SendPacket in order, every
ccKanji::Disp (text, place, colour, transp), RT01ICONPOS after each
frame, the portals' alphas, ccMenu.mapStatus, and the painted textures.

Skipped when the disc is not extracted or cargo is missing.
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
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "map_probe")

ONE = 0x3F800000
M32 = 0xFFFFFFFF
# Scratch memory (as tools/test_dungeon_rt.py lays it out, whose Game the
# dungeon checks share).
SYS, SCRATCH, GAME, WM, MENU, ENT, PLAYER = (0x00D00000, 0x00D10000, 0x00D20000, 0x00D40000, 0x00D41400,
                                            0x00D41600, 0x00D42000)
OBJ, WORK, EV, FIELD = 0x01E00000, 0x01E40000, 0x01E80000, 0x01C00000
PLW = inf_va(0x00730300)
AREA14 = (1420855, 10, 0, 1, 1, 14)


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def bf(b):
    return struct.unpack("<f", struct.pack("<I", b & M32))[0]


def hexs(*v):
    return " ".join("%x" % (x & M32) for x in v)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "map_probe"], cwd=ROOT, check=True)


class Probe:
    """map_probe, one request at a time."""

    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                  cwd=ROOT)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()
        self.p.stdout.close()


def sx32(v):
    v &= M32
    return v - (1 << 32) if v & 0x80000000 else v


class Game:
    """The game's map code in the interpreter: the sprites as their
    constructors make them, the leaf calls recorded."""

    def __init__(self, m=None, prog=None, heap=0x01D00000, heap_end=0x01DF0000, reset=True):
        from image import Program
        from test_anim import machine_class
        self.prog = prog or Program(ELF, "gcmn")
        self.m = m or machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value      # noqa: E731
        m = self.m
        self.heap, self.heap_end = heap, heap_end
        self.sprites = {}           # ccSprite address -> (id, sub)
        self.log = []
        self.sent = []
        m.store(self.sym("ccSys"), 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(self.sym("worldman"), 4, WM)
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("ccMenu"), 4, MENU)
        m.store(self.sym("g_entCtrl"), 4, ENT)
        m.store(self.sym("eventMng"), 4, EV)
        m.store(PLW, 4, PLAYER)
        m.store(self.sym("fontTex"), 4, 1)
        m.store(SYS + 0xC, 2, 512)
        m.store(SYS + 0xE, 2, 448)
        for name in ("CreateSema", "DeleteSema", "SignalSema", "WaitSema", "iSignalSema", "DIntr", "EIntr"):
            m.hooks[self.sym(name)] = lambda mm, *a: 1
        for name in ("ccMalloc__FUi", "__nw__FUi", "__nwa__FUi"):
            m.hooks[self.sym(name)] = self.malloc
        m.hooks[self.sym("_ccMalloc__FUiUiUii")] = lambda mm, al, n, *a: self.malloc(mm, n)
        if reset:
            for base, n in ((WM, 0x4E0), (GAME, 0x88), (MENU, 0x200), (ENT, 0x40), (PLAYER, 0x300),
                            (EV, 0x800)):
                m.mem[base:base + n] = bytes(n)
            m.store(WM + 0x160, 4, M32)

    # --- machine helpers -------------------------------------------------------
    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        if self.heap > self.heap_end:
            raise RuntimeError("heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def call(self, name, *args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name) if isinstance(name, str) else name, list(args), limit=2_000_000_000)

    def f(self, a):
        return self.m.load(a, 4)

    # --- the sprites -------------------------------------------------------------
    def layer(self, pri, frame=None):
        """A ccLayer of priority `pri` with its own view (ccLayer::Init(pri,
        0)), and ccView::SetFrame(*frame) on it."""
        m = self.m
        lay = self.malloc(m, 0x40)
        self.call("__ct__16ccDrawPacketCtrlFv", lay + 8)
        m.store(lay + 0x28, 4, 0)
        self.call("Init__7ccLayerFsP6ccView", lay, pri, 0)
        if frame:
            self.call("SetFrame__6ccViewFffffffff", m.load(lay + 0x2C, 4), fargs=[fb(v) for v in frame])
        return lay

    def mask(self, sid, mm, ms, layer, th=8):
        """new ccMask(mm, ms) on `layer`, its TEX0.TH `th` (the texture's
        1 << th rows)."""
        m = self.m
        a = self.malloc(m, 0xD8)
        self.call("__ct__6ccMaskFii", a, mm, ms)
        m.store(a + 0xA8, 4, layer)
        m.store(a + 0xB0, 8, th << 30 | 8 << 26)
        self.sprites[a] = (sid, 0)
        sub = m.load(a + 0xD0, 4)
        if sub:
            self.sprites[sub] = (sid, 1)
        return a

    def hooks(self, native_packets=False):
        """The leaf calls: MakePacketStr recorded (or run natively for the
        vertices), SendPacket recorded, Disp recorded."""
        m = self.m
        h = m.hooks
        sym = self.sym
        if getattr(self, "own_new", True):
            h[sym("__nw__FUi")] = lambda mm, n, *a: self.malloc(mm, n)
        h[sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm, ctrl, n, *a: self.malloc(mm, n)

        def fields(s):
            g = lambda o: m.load(s + o, 4)  # noqa: E731
            i = lambda o: sx32(m.load(s + o, 4))  # noqa: E731
            rgba = [m.load(s + 0x68 + 4 * k, 4) & 0xFF for k in range(4)]
            return [g(0x30), g(0x34), g(0x38), g(0x3C), g(0x40), g(0x44), g(0x48), g(0x4C), i(0x50), i(0x54),
                    i(0x58), i(0x5C), i(0x60), rgba, m.load(s + 4, 4) & 0x70]

        def make_str(mm, s, st, t, *a):
            code = mm.load(st, 1)
            if (t == 0 and code == 0) or (t != 0 and code >= 0xFE):
                return 0
            self.log.append(("pk", s, code, fields(s)))
            return 0
        if not native_packets:
            h[sym("MakePacketStr__8ccSpriteFPci")] = make_str
        else:
            h.pop(sym("MakePacketStr__8ccSpriteFPci"), None)

        def send(mm, s, *a):
            n = mm.load(s + 0x20, 4)
            pkt = mm.load(s + 0x2C, 4)
            self.log.append(("send", s, n, pkt))
            if native_packets:
                self.sent.append((self.sprites[s], self.read_packets(s, n, pkt)))
            mm.store(s + 0x20, 4, 0)
            return 0
        h[sym("SendPacket__8ccSpriteFv")] = send
        # float to unsigned long long: its dptoul uses dsrlv (as
        # tools/font.py's Game does it).

        def fixunssfdi(mm, *a):
            import eemu
            nn, e = eemu.f_value(mm.f[12])
            if nn <= 0:
                return 0
            return nn << e if e >= 0 else nn >> -e
        h[sym("__fixunssfdi")] = fixunssfdi

        def disp(mm, k, st, c, *a):
            text = []
            while mm.load(st + len(text), 1):
                text.append(mm.load(st + len(text), 1))
            rgba = [mm.load(k + 0x68 + 4 * j, 4) & 0xFF for j in range(4)]
            self.log.append(("text", k, text, mm.load(k + 0x40, 4), mm.load(k + 0x44, 4), rgba,
                             mm.load(k + 0x30, 4), sx32(c)))
            return 0
        h[sym("Disp__7ccKanjiFPciff")] = disp
        h[sym("SetActiveLayer__9WORLD_MANFi")] = lambda mm, *a: 0

    def outs(self):
        """The log as the probe reports what was sent (packets without
        corners)."""
        out, pending = [], {}
        for e in self.log:
            if e[0] == "pk":
                pending.setdefault(e[1], []).append([e[2]] + e[3])
            elif e[0] == "send":
                sid, sub = self.sprites[e[1]]
                out.append(["send", sid, sub, pending.pop(e[1], [])])
            else:
                out.append(["text", self.kanji_id.get(e[1], -1), e[2], e[3], e[4], e[5], e[6]])
        assert not pending, pending
        self.log = []
        return out

    kanji_id = {}

    def read_packets(self, s, n, pkt):
        """The n packets MakePacketStr wrote for sprite `s`: a TF sprite's
        SPRITEs (TEX0 RGBAQ UV XYZ2 UV XYZ2 from +176, 96 bytes each), a TRF
        one's strips (a GIF tag, TEX0, RGBAQ, then UV XYZ2 four times, from
        +160, 176 bytes each): each its corners [x, y, u, v] and RGBA."""
        m = self.m
        typ = m.load(s + 8, 4)
        out = []
        w = lambda a: m.load(a, 4)  # noqa: E731
        for k in range(n):
            if typ == 0:
                q = pkt + 176 + 96 * k
                rgba = [w(q + 16 + 4 * j) for j in range(4)]
                cs = [[w(q + 48), w(q + 52), w(q + 32), w(q + 36)], [w(q + 80), w(q + 84), w(q + 64), w(q + 68)]]
            else:
                q = pkt + 160 + 176 * k
                rgba = [w(q + 32 + 4 * j) for j in range(4)]
                cs = [[w(q + 64 + 32 * v), w(q + 68 + 32 * v), w(q + 48 + 32 * v), w(q + 52 + 32 * v)]
                      for v in range(4)]
            out.append([cs, rgba])
        return out

    def run_twice(self, run, keep):
        """`run` recorded (MakePacketStr hooked), then again from the same
        state (`keep`: the (address, size) ranges it changes) with
        MakePacketStr run natively: what was sent, and each send's packets
        as MakePacketStr wrote them."""
        m = self.m
        snap = [(a, bytes(m.mem[a:a + n])) for a, n in keep]
        run()
        outs = self.outs()
        after = [(a, bytes(m.mem[a:a + n])) for a, n in keep]
        for a, b in snap:
            m.mem[a:a + len(b)] = b
        self.hooks(native_packets=True)
        self.sent = []
        run()
        self.log = []
        sent = self.sent
        self.hooks()
        # Both runs leave the same state.
        for a, b in after:
            assert bytes(m.mem[a:a + len(b)]) == b, hex(a)
        return outs, sent


    def set_player(self, pos, dircz):
        m = self.m
        for k, v in enumerate(pos):
            m.store(PLAYER + 0x40 + 4 * k, 4, v)
        for k, v in enumerate((0, 0, dircz, 0)):
            m.store(PLAYER + 0x60 + 4 * k, 4, v)


def port_vertices(outs):
    """The probe's corners, per send, the culled packets dropped."""
    return [((o[1], o[2]), [p[16] for p in o[3] if p[16] is not None]) for o in outs if o[0] == "send"]


def game_vertices(sent):
    return [((sid, sub), [[[[sx32(v) for v in c] for c in cs], rgba] for cs, rgba in pk]) for (sid, sub), pk in sent]



def strip(outs):
    """The probe's answer without the corners."""
    out = []
    for o in outs:
        if o[0] == "send":
            out.append(["send", o[1], o[2], [p[:16] for p in o[3]]])
        else:
            out.append(o)
    return out


# --- the town ---------------------------------------------------------------------
LATER = volume.NAME != "infection"
# ROOTTOWN's mapw (maph, stepw, steph after it): 4 bytes on from Mutation.
MAPW = 0x80 if LATER else 0x7C
# The towns' steps (stepw, steph).
STEPS = {0: (0x3FC66666, 0x3FCCCCCD), 1: (0x3FA3D70A, 0x3FA5E354), 2: (0x3F828F5C, 0x3F81A9FC)}


def words(start, end):
    p = volume.program(ELF)
    return [(a, struct.unpack("<I", p.read(a, 4))[0]) for a in range(start, end, 4)]


def race_globals():
    """From Mutation on, the flag race as the town maps read it: the
    function DrawMap asks whether it runs in the town (the first unnamed
    one Dun Loireag's calls), the race's state that function reads after
    `game` (gp-relative), and the racers' table DrawMap reads after asking
    (lui/addiu)."""
    ask = volume.callee("DrawMap__10ROOTTOWN02Fv", 0)
    gp = volume.program(ELF).gp
    loads = [w for _, w in words(ask, ask + 0x40) if w >> 26 == 0x23 and (w >> 21) & 31 == 28]
    state = (gp + ((loads[1] & 0xFFFF) ^ 0x8000) - 0x8000) & M32
    code = words(*volume.span("DrawMap__10ROOTTOWN02Fv"))
    at = next(i for i, (_, w) in enumerate(code) if w >> 26 == 3 and (w & 0x03FFFFFF) << 2 == ask)
    hi = next((w for _, w in code[at:] if w >> 26 == 0x0F))
    lo = next((w for _, w in code[at:] if w >> 26 == 0x09 and (w >> 21) & 31 == (hi >> 16) & 31))
    table = ((hi & 0xFFFF) << 16) + ((lo & 0xFFFF) ^ 0x8000) - 0x8000
    return state, table & M32


class Town(Game):
    """ROOTTOWN01 (ROOTTOWN02 `town` 1, ROOTTOWN03 2) as its constructor
    leaves the map part. Infection's Dun Loireag's arrow is on the font
    layer, its steps 1.28 and 1.2921. From Mutation on Dun Loireag and
    Carmina Gade put the map on a layer of priority 40 and a fourth mask,
    the flag race's racers, on one of 45 over the screen."""

    def __init__(self, town=0, **kw):
        super().__init__(**kw)
        m = self.m
        self.hooks()
        self.town = town
        frame = (340, 36, 160, 160, 80, 80, 1, 1)
        self.map_layer = self.layer(50, frame)
        self.font_layer = self.layer(240)
        own = LATER and town
        self.rt = self.malloc(m, 0x260)
        m.store(self.rt + 0x10, 4, self.mask(0, 128, 32, self.layer(40, frame) if own else self.map_layer))
        m.store(self.rt + 0x14, 4, self.mask(1, 128, 32, self.font_layer))
        m.store(self.rt + 0x18, 4, self.mask(2, 6, 6, self.font_layer if town == 1 and not LATER else self.map_layer,
                                             4))
        if own:
            marks = self.layer(45, (0, 0, 512, 384, 256, 192, 1, 1))
            m.store(self.rt + 0x1C, 4, self.mask(3, 128, 32, marks))
        m.store(self.rt + MAPW, 4, 140)
        m.store(self.rt + MAPW + 4, 4, 256)
        m.store(self.rt + MAPW + 8, 4, STEPS[town][0])
        m.store(self.rt + MAPW + 12, 4, STEPS[town][1])
        self.icons_at = self.sym("RT0%dICONPOS" % (town + 1))
        self.draw = "DrawMap__10ROOTTOWN0%dFv" % (town + 1)
        # The arrow's pulse and its first-time flag: the static ccRotate
        # turns (addiu $a0, $gp, ..).
        pulse = next(w for _, w in words(*volume.span(self.draw)) if w >> 16 == 0x2784)
        self.pulse = (self.prog.gp + ((pulse & 0xFFFF) ^ 0x8000) - 0x8000) & M32
        if LATER:
            self.race_state, self.racers = race_globals()
            self.racer_at = [self.malloc(m, 0x200) for _ in range(3)]

    def icons(self):
        return [[sx32(self.m.load(self.icons_at + 36 * i + 4 * k, 4)) for k in range(9)] for i in range(6)]

    def frame(self, mode, alpha, pos, dircz, race=None):
        """`race`: while the flag race runs, each racer as (x, y, fade) or
        None."""
        m = self.m
        m.store(WM + 0x128, 4, mode)
        m.store(WM + 0x148, 4, alpha)
        self.set_player(pos, dircz)
        if LATER:
            m.store(self.race_state, 4, 1 if race else 0)
            for i in range(3):
                r = race[i] if race else None
                m.store(self.racers + 4 * i, 4, self.racer_at[i] if r else 0)
                if r:
                    for k, v in enumerate((r[0], r[1], 0, ONE)):
                        m.store(self.racer_at[i] + 0x40 + 4 * k, 4, v)
                    m.store(self.racer_at[i] + 0x1EC, 4, r[2])
        keep = [(self.icons_at, 216), (self.pulse, 8)]
        return self.run_twice(lambda: self.call(self.draw, self.rt), keep)


def town_frames(rng, start=(0, 0x45AF0000, 0x44160000, ONE)):
    """(mode, alpha, pos, heading) for a walk over Mac Anu (or from another
    town's start): the arrival, the map scrolled to both stops, the signs in
    and out of view, the map off and on again, the alpha fading."""
    out = []
    alpha = 0
    for k in range(12):
        out.append((0, fb(min(1.0, k / 8.5)), start, 0))
    for y in (5600, 4000, 2000, 0, -2000, -4000, -5600, -6200, -3000, 1000, 3500, 6100, 6900):
        for _ in range(3):
            x = rng.uniform(-3000, 3000)
            out.append((0, ONE, (fb(x), fb(y + rng.uniform(-300, 300)), 0x44160000, ONE),
                        fb(rng.uniform(-3.2, 3.2))))
    for k in range(4):
        out.append((1, ONE, (0, fb(-5000.0), 0x44160000, ONE), 0))
    for k in range(40):
        y = rng.uniform(-7000, 7000)
        alpha = rng.choice((ONE, fb(0.5), 0, fb(rng.random())))
        out.append((0, alpha, (fb(rng.uniform(-4000, 4000)), fb(y), 0x44160000, ONE), fb(rng.uniform(-4, 4))))
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class TownMapAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_town(self):
        self.run_town(0, "town01", town_frames(random.Random(3)))

    def test_dun_loireag(self):
        """ROOTTOWN02::DrawMap: from Dun Loireag's start (0, 3500, 0) all
        over the town, and the arrow walked over each sign's balloon (which
        dims it)."""
        rng = random.Random(4)
        frames = town_frames(rng, (0, 0x455AC000, 0, ONE))
        # Over each sign: where the arrow's point lands on its balloon, as
        # DrawMap places both (x 408 + 1.28 x / 100, y 102 - 1.2921 y / 100,
        # the map's scroll taken off), with the neighbours around it.
        for ix, iy in ((0x41, 0x3A), (0x6E, 0x54), (0x74, 0xA7), (0x54, 0x9B), (0x10, 0x78), (0x2E, 0x6C)):
            for dx in (-600, 0, 600, 1500):
                for dy in (-1200, 0, 1200, 2500):
                    x = (ix - 70) * 100 / 1.28 + dx
                    y = -(iy - 128) * 100 / 1.2921 + dy
                    frames.append((0, ONE, (fb(x), fb(y), 0, ONE), fb(rng.uniform(-3.2, 3.2))))
        self.run_town(1, "town02", frames)

    @unittest.skipUnless(LATER, "Carmina Gade's map is Mutation's on")
    def test_carmina_gade(self):
        """ROOTTOWN03::DrawMap: all over Carmina Gade, whose sign 3 draws
        mirrored and flipped."""
        self.run_town(2, "town03", town_frames(random.Random(5), (0, 0, 0, ONE)))

    @unittest.skipUnless(LATER, "the flag race is Mutation's on")
    def test_flag_race(self):
        """Dun Loireag's and Carmina Gade's maps while the flag race runs
        and after: the racers (some missing, some fading, some off the
        map's view) and the signs held back and fading on."""
        for town, stem in ((1, "town02"), (2, "town03")):
            rng = random.Random(6 + town)
            frames = []
            for k, (mode, alpha, pos, dircz) in enumerate(town_frames(rng, (0, 0x455AC000, 0, ONE))):
                race = None
                if k % 40 < 25:
                    race = [None if rng.random() < 0.2 else
                            (fb(rng.uniform(-6000, 6000)), fb(rng.uniform(-9000, 9000)),
                             rng.choice((ONE, 0, fb(rng.random())))) for _ in range(3)]
                frames.append((mode, alpha, pos, dircz, race))
            self.run_town(town, stem, frames)

    def run_town(self, town, stem, frames):
        g = Town(town)
        p = Probe()
        try:
            got = p.ask("town " + stem)
            self.assertEqual(got["icons"], g.icons())
            n = v = dim = marks = 0
            for i, (mode, alpha, pos, dircz, *race) in enumerate(frames):
                race = race[0] if race else None
                line = "tframe " + hexs(mode, alpha, pos[0], pos[1], pos[2], dircz)
                if race:
                    line += " race " + " ".join(hexs(*r) if r else "- - -" for r in race)
                mine = p.ask(line)
                game, sent = g.frame(mode, alpha, pos, dircz, race)
                self.assertEqual(strip(mine["outs"]), game, (i, mode, [bf(v) for v in pos]))
                self.assertEqual(mine["icons"], g.icons(), i)
                self.assertEqual(port_vertices(mine["outs"]), game_vertices(sent), i)
                n += sum(len(o[3]) for o in game if o[0] == "send")
                v += sum(len(pk) for _, pk in sent)
                # A sign dimmed under the arrow: its balloon at alpha 64.
                dim += sum(1 for o in game if o[0] == "send" and o[1] == 1 for pk in o[3] if pk[14][3] == 64)
                marks += sum(len(o[3]) for o in game if o[0] == "send" and o[1] == 3)
            print(f"{stem}: {i + 1} frames, {n} packets, {v} drawn, {dim} dimmed, {marks} racers",
                  file=sys.stderr)
            if town == 1 and not any(f[4:] and f[4] for f in frames):
                self.assertGreater(dim, 0)
            if any(f[4:] and f[4] for f in frames):
                self.assertGreater(marks, 0)
        finally:
            p.close()



# --- a field ---------------------------------------------------------------------
class World(Game):
    """WORLD as Init and Generate leave the map part: its sprites, fobj2[],
    fobj[40][40], water, dx[0] / dy[0], mapFlag, and the entry control's
    portals and gimmicks."""

    def __init__(self, info, **kw):
        super().__init__(**kw)
        m = self.m
        self.hooks()
        self.map_layer = self.layer(50, (340, 28, 160, 160, 80, 80, 1, 1))
        self.w = self.malloc(m, 0x61E0)
        w = self.w
        m.store(w + 0x10, 4, info["type"])
        m.store(w + 0x6134, 4, self.mask(0, 512, 64, self.map_layer))
        m.store(w + 0x6138, 4, self.mask(1, 16, 16, self.map_layer, 4))
        k = self.malloc(m, 0xE8)
        self.call("__ct__8ccSpriteFv", k)
        m.store(k + 4, 4, m.load(k + 4, 4) | 0x10)
        m.store(w + 0x6174, 4, k)
        self.kanji_id = {k: 2}
        for i, (kind, idx, mx, my, wx, wy) in enumerate(info["fobj2"]):
            o = self.malloc(m, 0x80)
            for off, v in ((0, kind), (4, idx), (8, mx), (0xC, my), (0x40, wx), (0x44, wy)):
                m.store(o + off, 4, v)
            m.store(w + 0x5090 + 4 * i, 4, o)
        m.store(w + 0x18, 4, len(info["fobj2"]))
        m.store(w + 0x30, 4, info["water"] & M32)
        for cx, cy, kind, idx, mx, my in info["fobj"]:
            o = self.malloc(m, 0x50)
            for off, v in ((0, kind), (4, idx), (8, mx), (0xC, my)):
                m.store(o + off, 4, v)
            m.store(w + 0x1E90 + 4 * (40 * cx + cy), 4, o)
        m.store(w + 0x53D8, 4, info["dungeon"][0])
        m.store(w + 0x53E4, 4, info["dungeon"][1])
        self.ents = []

    def entities(self, circles, fountain):
        """The entry control's portals (mcHead, +0x20) and, for the fountain,
        a gimmick of id 20 (gimHead +0x2c, gimNum +0x28)."""
        m = self.m
        prev = None
        self.ents = []
        m.store(ENT + 0x20, 4, 0)
        if not hasattr(self, "pool"):
            self.pool = [self.malloc(m, 0x1D0) for _ in range(17)]
        pool = iter(self.pool)
        for mx, my, fading, alpha in circles:
            e = next(pool)
            m.mem[e:e + 0x1D0] = bytes(0x1D0)
            m.store(e + 0x138, 4, mx)
            m.store(e + 0x13C, 4, my)
            m.store(e + 0xE0, 1, 0x20 if fading else 0)
            m.store(e + 0xEC, 4, alpha)
            if prev:
                m.store(prev + 0x1C4, 4, e)
            else:
                m.store(ENT + 0x20, 4, e)
            prev = e
            self.ents.append(e)
        g = next(pool)
        m.mem[g:g + 0x1D0] = bytes(0x1D0)
        m.store(g + 0x124, 4, 20 if fountain else 3)
        m.store(ENT + 0x2C, 4, g)
        m.store(ENT + 0x28, 4, 1)

    def frame(self, mode, alpha, pos, dircz, event, fountain, flag, circles):
        m = self.m
        m.store(WM + 0x12C, 4, mode)
        m.store(WM + 0x148, 4, alpha)
        m.store(WM + 0x120, 4, event)
        m.store(self.w + 0x6170, 4, flag)
        self.set_player(pos, dircz)
        self.entities(circles, fountain)
        keep = [(self.sym("r$3072"), 16)] + [(e + 0xEC, 4) for e in self.ents]
        outs, sent = self.run_twice(lambda: self.call("DrawMiniMap__5WORLDFv", self.w), keep)
        return outs, sent, [m.load(e + 0xEC, 4) for e in self.ents]

    def paint(self, heights, field_type):
        """WORLD::Generate's height map painting (Infection's gcmn
        0x005a7c40 - 0x005a7d1c, `paint_span`) run natively over `heights`
        in a FIELD: the 80 x 80 texels it writes."""
        m = self.m
        fld, chunk, desc, tex = FIELD, OBJ, OBJ + 0x100, OBJ + 0x1000
        m.mem[fld:fld + 0xB6000] = bytes(0xB6000)
        for i, h in enumerate(heights):
            m.store(fld + 0xAF02C + 4 * i, 4, h)
        m.mem[tex:tex + 0x10000] = bytes(0x10000)
        m.store(chunk + 0x28, 4, desc)
        m.store(desc + 4, 4, tex)
        m.store(self.w + 0x10, 4, field_type)
        m.store(self.w + 0x6168, 4, fld)
        h = m.hooks
        h[self.sym("GetChunkAdrsF__8ccStreamFPCci")] = lambda mm, *a: chunk

        def stop(mm, *a):
            mm.set(31, 0xFFFFFFF0)
            return 0
        start, end = paint_span()
        h[end] = stop
        # A trampoline: $s0 = WORLD, then into the loop.
        code = WORK
        w = self.w
        tramp = [0x3C100000 | (w >> 16), 0x36100000 | (w & 0xFFFF), 0x08000000 | (start >> 2), 0]
        for i, word in enumerate(tramp):
            m.store(code + 4 * i, 4, word)
        self.call(code)
        del h[end]
        return [m.load(tex + (255 - y) * 256 + 79 - x, 1) for y in range(80) for x in range(80)]


def paint_span():
    """WORLD::Generate's height map painting in the volume's code: from the
    field type's load before `fieldminimap` is read (lw $v0, 16($s0); sll
    $v1, $v0, 2) to past its rows' loop (slti $v1, $s2, 80; bnez; nop). A
    later volume's Generate grows before it, so the carry's offset within
    the function misses it."""
    code = words(*volume.span("Generate__5WORLDFv"))
    i = next(i for i in range(len(code) - 1) if code[i][1] == 0x8E020010 and code[i + 1][1] == 0x00021880)
    j = next(j for j in range(i, len(code)) if code[j][1] == 0x2A430050)
    return code[i][0], code[j][0] + 12


def field_frames(rng):
    """(mode, alpha, pos, heading, event, fountain, mapFlag, circles):
    both maps and none, story area 14 and a story area with no dungeon on
    its map, the fountain, the portals shown (a fading one among them), the
    player all over the field and across its wrap."""
    out = []
    circles = [(rng.randrange(80), rng.randrange(80), 0, 128) for _ in range(4)] + [(40, 40, 1, 128)]
    for k in range(60):
        mode = (0, 0, 1, 2)[k % 4] if k >= 8 else 0
        pos = (fb(rng.uniform(-3000, 51000)), fb(rng.uniform(-3000, 51000)), fb(rng.uniform(0, 900)), ONE)
        event = 28 if k % 7 == 3 else 14
        flag = int(k >= 20)
        out.append((mode, fb(min(1.0, k / 8.5)), pos, fb(rng.uniform(-3.3, 3.3)), event, int(k % 5 == 1), flag,
                    circles))
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class FieldMapAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def run_field(self, params, rng):
        p = Probe()
        try:
            info = p.ask("field " + hexs(*params))
            g = World(info)
            texels = g.paint(info["map"], info["type"])
            self.assertEqual(texels, info["texels"], params)
            n = v = 0
            circles = None
            for i, (mode, alpha, pos, dircz, event, fountain, flag, cs) in enumerate(field_frames(rng)):
                circles = circles or cs
                req = [mode, alpha, pos[0], pos[1], pos[2], dircz, event, fountain, flag, len(circles)]
                for c in circles:
                    req += list(c)
                mine = p.ask("fframe " + hexs(*req))
                game, sent, alphas = g.frame(mode, alpha, pos, dircz, event, fountain, flag, circles)
                self.assertEqual(strip(mine["outs"]), game, (params, i, mode))
                self.assertEqual(port_vertices(mine["outs"]), game_vertices(sent), (params, i))
                self.assertEqual(mine["circles"], alphas, i)
                circles = [(c[0], c[1], c[2], a) for c, a in zip(circles, alphas)]
                n += sum(len(o[3]) for o in game if o[0] == "send")
                v += sum(len(pk) for _, pk in sent)
            return n, v
        finally:
            p.close()

    def test_area14(self):
        n, v = self.run_field(AREA14, random.Random(5))
        print(f"field 14: {n} packets, {v} drawn", file=sys.stderr)

    def test_random_fields(self):
        rng = random.Random(8)
        total = 0
        for ft in (0, 1, 2, 3, 5, 6, 7, 8, 9):
            n, _ = self.run_field((rng.getrandbits(32), ft, rng.randrange(3), rng.randrange(3), rng.randrange(3), 0),
                                  rng)
            total += n
        print(f"random fields: {total} packets", file=sys.stderr)


# --- a dungeon -------------------------------------------------------------------
DG = 0x00E00000
DEV = 0x01F70000


class DungeonMap(Game):
    """DUNGEON's map sprites over tools/test_dungeon_rt.py's real DUNGEON."""

    def __init__(self, dg):
        super().__init__(m=dg.m, prog=dg.prog, heap=0x01F00000, heap_end=0x01F68000, reset=False)
        m = self.m
        self.dg = dg
        m.store(self.sym("eventMng"), 4, DEV)
        m.mem[DEV:DEV + 0x800] = bytes(0x800)
        self.hooks()
        self.map_layer = self.layer(50, (340, 28, 160, 160, 80, 80, 1, 1))
        self.font_layer = self.layer(240)
        self.masks = [self.mask(0, 2, 0, self.map_layer), self.mask(1, 50, 5, self.map_layer),
                      self.mask(2, 2, 0, self.map_layer), self.mask(3, 50, 5, self.map_layer, 4)]
        k = self.malloc(m, 0xE8)
        self.call("__ct__8ccSpriteFv", k)
        m.store(k + 4, 4, m.load(k + 4, 4) | 0x10)
        m.store(k + 0xA8, 4, self.font_layer)
        self.masks.append(k)
        self.kanji_id = {k: 4}
        # From here the dungeon's own allocator serves SetRoom.
        self.own_new = False
        for name in ("ccMalloc__FUi", "__nw__FUi", "ccMemalign__FUiUi", "__nwa__FUi"):
            m.hooks[self.sym(name)] = dg.malloc
        m.hooks[self.sym("_ccMalloc__FUiUiUii")] = lambda mm, al, n, *a: dg.malloc(mm, n)

    def attach(self):
        """The sprites into the DUNGEON dg.make just made (the constructor's
        map, map1, map2, map3, kanji), the pulse from 0 as a new DUNGEON's
        port starts it."""
        for k, a in enumerate(self.masks):
            self.m.store(DG + 0xD388C + 4 * k, 4, a)
        self.m.store(self.sym("r$8238"), 4, 0)

    def squares(self, f):
        m = self.m
        base = DG + 0x33538 + (f << 16)
        return [[x, y, m.load(base + (x << 8) + y, 1)] for x in range(200) for y in range(200)
                if m.load(base + (x << 8) + y, 1)]

    def make_mini_map(self, f, i):
        self.m.store(DG + 4, 4, f)
        return self.call("MakeMiniMap__7DUNGEONFi", DG, i)

    def texsum(self):
        m = self.m
        ch = self.call("GetChunkAdrsF__8ccStreamFPCci", m.load(DG + 0xD3884, 4), self.cstr_at("TEX_xdsmap02"), 0)
        tex = m.load(m.load(ch + 0x28, 4) + 4, 4)
        data = m.mem[tex:tex + 0x10000]
        return sum((i + 1) * b for i, b in enumerate(data))

    def cstr_at(self, text):
        a = self.malloc(self.m, len(text) + 1)
        self.m.mem[a:a + len(text)] = text.encode()
        return a

    def frame(self, f, mode, special, alpha, pos, dircz, hide, hold, circles, gims):
        m = self.m
        m.store(DG + 4, 4, f)
        m.store(DG + 0x42C, 4, hide)
        m.store(WM + 0x130, 4, mode)
        m.store(WM + 0x160, 4, special)
        m.store(WM + 0x148, 4, alpha)
        m.store(DEV + 0x78C, 4, hold)
        m.store(MENU + 0x10, 2, 0)
        self.set_player(pos, dircz)
        if not hasattr(self, "pool"):
            self.pool = [self.malloc(m, 0x1D0) for _ in range(16)]
        pool = iter(self.pool)
        for head, lst in ((0x20, circles), (0x2C, gims)):
            prev = None
            m.store(ENT + head, 4, 0)
            for eid, fl, blk, x, y, p2 in lst:
                e = next(pool)
                m.mem[e:e + 0x1D0] = bytes(0x1D0)
                for off, v in ((0x124, eid), (0x130, fl), (0x134, blk), (0x100, x), (0x104, y), (0x150, p2)):
                    m.store(e + off, 4, v)
                if prev:
                    m.store(prev + 0x1C4, 4, e)
                else:
                    m.store(ENT + head, 4, e)
                prev = e
        m.store(ENT + 0x28, 4, len(gims))
        keep = [(self.sym("r$8238"), 16), (DG + 0x42C, 4)]
        outs, sent = self.run_twice(lambda: self.call("DrawMap__7DUNGEONFv", DG), keep)
        return outs, sent, m.load(MENU + 0x10, 2)


def dungeon_cases():
    import test_dungeon_rt as dr
    return dr.cases()


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DungeonMapAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_dungeons(self):
        import test_dungeon_rt as dr
        rng = random.Random(11)
        dg = dr.Game()
        g = None
        counts = {"dungeons": 0, "rooms": 0, "squares": 0, "frames": 0, "packets": 0}
        p = Probe()
        try:
            for seeds, ft, weather, lmax, rmax, hack, field, code, server in dungeon_cases():
                what = f"field {field} type {ft} dungeon {code} seeds {seeds}"
                info = p.ask(f"dungeon {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} "
                             f"{field} {code} {server}")
                if info["dtype"] >= 8 or info["event"] in dr.SPECIAL_EVENTS:
                    continue
                dg.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
                g = g or DungeonMap(dg)
                g.attach()
                counts["dungeons"] += 1
                story = {}
                if info["event"]:
                    for i in range(150):
                        a = DG + 0x30894 + 0x4C * i
                        fl, idx, rt = g.m.load(a, 4), g.m.load(a + 4, 4), g.m.load(a + 0x10, 4)
                        if fl < 10:
                            story[(fl, idx)] = rt
                floors = [f for f, fl in enumerate(info["rooms"]) if any(r[0] for r in fl)][:2]
                if len(floors) > 1:
                    self.show_map(p, g, dg, floors[1], info, what)
                    counts["ShowMap"] = counts.get("ShowMap", 0) + 1
                    floors = floors[:1]
                for f in floors:
                    rooms = [i for i, r in enumerate(info["rooms"][f]) if r[0] and story.get((f, i), 0) < 15]
                    for i in rooms:
                        # The room's size as MakeRoom set it.
                        self.assertEqual(info["rooms"][f][i][1], g.m.load(DG + 0x306D0 + 45 * f + 3 * i, 1),
                                         f"{what}: room {f}/{i} size")
                        mine = p.ask(f"droom {f:x} {i:x}")
                        dg.set_room(f, i)
                        new = g.make_mini_map(f, i)
                        self.assertEqual(mine["new"], new, f"{what}: room {f}/{i}")
                        game = g.squares(f)
                        self.assertEqual(mine["squares"], game, f"{what}: room {f}/{i} squares")
                        counts["rooms"] += 1
                        counts["squares"] = len(game)
                    # The map of the floor, every room seen: portals, gimmicks,
                    # the stairs, each mode, the repaint.
                    for k in range(12):
                        seen = [i for i in rooms]
                        circles = [(0, f, rng.choice(seen), fb(rng.uniform(1000, 59000)),
                                    fb(rng.uniform(1000, 59000)), 0) for _ in range(3)]
                        gims = [(rng.choice((0, 3, 5, 17, 20, 38, 44, 45)), f, rng.choice(seen),
                                 fb(rng.uniform(1000, 59000)), fb(rng.uniform(1000, 59000)), rng.choice((0, 1)))
                                for _ in range(5)]
                        mode = 1 if k == 5 else 0
                        special = 3 if k == 6 else -1
                        hide = 1 if k in (2, 9) else 0
                        hold = 1 if k == 9 else 0
                        alpha = fb(rng.random())
                        pos = (fb(rng.uniform(0, 60000)), fb(rng.uniform(0, 60000)), 0, ONE)
                        dircz = fb(rng.uniform(-3.3, 3.3))
                        req = [f, mode, special, alpha, pos[0], pos[1], pos[2], dircz, hide, hold, len(circles)]
                        for c in circles:
                            req += list(c)
                        req.append(len(gims))
                        for c in gims:
                            req += list(c)
                        mine = p.ask("dframe " + hexs(*req))
                        game, sent, status = g.frame(f, mode, special, alpha, pos, dircz, hide, hold, circles,
                                                     gims)
                        self.assertEqual(strip(mine["outs"]), game, f"{what}: floor {f} frame {k}")
                        self.assertEqual(port_vertices(mine["outs"]), game_vertices(sent), f"{what}: {f}/{k}")
                        self.assertEqual(mine["map_status"], status, f"{what}: {f}/{k} mapStatus")
                        if hide:
                            self.assertEqual(mine["texsum"], g.texsum(), f"{what}: {f}/{k} the texture")
                        counts["frames"] += 1
                        counts["packets"] += sum(len(o[3]) for o in game if o[0] == "send")
        finally:
            p.close()
        print(f"dungeons: {counts}", file=sys.stderr)
        self.assertGreater(counts["rooms"], 20)

    def show_map(self, p, g, dg, f, info, what):
        """DUNGEON::ShowMap on floor f, the player at its up room's
        centre, a call a frame until it is done: each call's answer and
        the room it leaves built, then the floor's squares."""
        m = g.m
        up = next(i for i, r in enumerate(info["rooms"][f]) if r[0])
        centre = [m.load(DG + 0x2F6E0 + f * 0xF0 + up * 16 + 4 * k, 4) for k in range(2)]
        m.store(DG + 4, 4, f)
        g.set_player((centre[0], centre[1], 0, ONE), 0)
        # The room he stands in built, as the game has it (ShowMap deletes
        # it first and builds it again at the end).
        p.ask(f"dset {f:x} {up:x}")
        dg.set_room(f, up)
        calls = 0
        while True:
            mine = p.ask(f"dshow {f:x} {centre[0]:x} {centre[1]:x}")
            done = g.call("ShowMap__7DUNGEONFv", DG)
            calls += 1
            self.assertEqual(mine[0], done, f"{what}: ShowMap call {calls}")
            if done:
                break
            self.assertLess(calls, 20)
        self.assertEqual(p.ask(f"dsquares {f:x}"), g.squares(f), f"{what}: ShowMap's squares")


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ModesAgainstGame(unittest.TestCase):
    """WORLD_MAN::ChangeMapMode, SetMapAlpha / GetMapAlpha and ShowMap's
    field and town cases."""

    @classmethod
    def setUpClass(cls):
        build()

    def test_modes_alpha_show_map(self):
        p = Probe()
        try:
            info = p.ask("field " + hexs(*AREA14))
            g = World(info)
            m = g.m
            n = 0
            for flag in (0, 1, 2, 3):
                for t in range(3):
                    for fm in range(4):
                        for d in range(3):
                            m.store(WM + 8, 4, flag)
                            for k, v in enumerate((t, fm, d)):
                                m.store(WM + 0x128 + 4 * k, 4, v)
                            g.call("ChangeMapMode__9WORLD_MANFv", WM)
                            game = [sx32(m.load(WM + 0x128 + 4 * k, 4)) for k in range(3)]
                            self.assertEqual(p.ask(f"modes {flag} {t} {fm} {d}"), game, (flag, t, fm, d))
                            n += 1
            for a in (0.0, 0.25, 1.0, 0.5333333):
                g.call("SetMapAlpha__9WORLD_MANFf", WM, fargs=[fb(a)])
                self.assertEqual(m.load(WM + 0x148, 4), fb(a))
                g.call("GetMapAlpha__9WORLD_MANFv", WM)
                self.assertEqual(m.f[0] & M32, fb(a))
            # ShowMap in a field (WORLD_MAN.flag 1) for every story area's
            # eventAreaNumber and none; in a town (flag 0) it does nothing.
            m.store(WM + 0x434, 4, g.w)
            infos = self.area_ids(g)
            for ev in [0] + infos:
                m.store(WM + 8, 4, 1)
                m.store(WM + 0x120, 4, ev)
                m.store(g.w + 0x6170, 4, 0)
                done = g.call("ShowMap__9WORLD_MANFv", WM)
                game = [done, m.load(g.w + 0x6170, 4)]
                self.assertEqual(p.ask(f"showmap {ev:x}"), game, ev)
            m.store(WM + 8, 4, 0)
            m.store(g.w + 0x6170, 4, 0)
            self.assertEqual((g.call("ShowMap__9WORLD_MANFv", WM), m.load(g.w + 0x6170, 4)), (1, 0))
            print(f"modes: {n} ChangeMapMode cases, ShowMap for {len(infos) + 1} areas", file=sys.stderr)
        finally:
            p.close()

    @staticmethod
    def area_ids(g):
        base = g.sym("eventAreaInfo")
        return [g.m.load(base + 0x54 * k, 4) for k in range(126) if 0 < g.m.load(base + 0x54 * k, 4) < 1000]

if __name__ == "__main__":
    unittest.main()
