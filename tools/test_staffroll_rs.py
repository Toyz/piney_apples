#!/usr/bin/env python3
"""piney-desktop's staff roll (crates/piney-desktop/src/staffroll.rs)
against the game's own ccThStaffRollCtrl (desktop.prg 0x004126d0) run in
eemu (the Rust eemu_rs when it is built into tools/, as
test_anim.machine_class picks).

The controller is built natively (its lines, ccTransCode2Name over a save
whose names are "Kite" and "Tester", srand of ccSys+0x358), then Main runs
a frame at a time until `done` (+0x13a8). Native: the controller and its
lines, ccTransCode2Name, rand/srand, ccRand (its Mersenne Twister unseeded,
`mti` 625, as at boot), memset, memcpy, strlen, abs. Hooked: operator new (a
bump allocator), the constructors of ccSprite, ccMask, ccDrawPacketCtrl and
ccBufferSampling (their memory zeroed), ccKanji::Init and ccLayer::Init
(nothing), WORLD_MAN::SetActiveLayer (nothing), and the draws, recorded:

- ccView::SetFrame (its eight float arguments' bits) and SetLayerCenter;
- ccSprite::SetTex (the texture's name);
- ccKanji::Disp: which sprite (a line's, or a credit's first or second), the
  sprite's x (+0x40), y (+0x44) and transp (+0x30) as bits, and the text's
  length and FNV-1a hash;
- ccSprite::MakePacket on the mask: transp, dx, dy, sx, sy (bits), su, sv,
  wu, wv, wi and its second argument.

After each frame the controller's words +0x1390-+0x13e0 and an FNV-1a hash
of every line's bytes +0x0c-+0xec with its +0xf0 and +0xf4 are recorded too.
The results go to crates/piney-desktop/tests/staffroll_fixture.txt;
staffroll.rs's test replays them.

    python3 tools/test_staffroll_rs.py            check the fixture is the game's
    python3 tools/test_staffroll_rs.py fixture    write it
    python3 tools/test_staffroll_rs.py detail N   frame N's (or "new"'s) draws in full
"""

import functools
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va  # noqa: E402
inf_va = functools.partial(va, overlay='desktop')  # this overlay's globals

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
FIXTURE = os.path.join(ROOT, "crates", "piney-desktop", "tests", "staffroll_fixture.txt")

CCSYS, SAVEDATA = inf_va(0x003788E0), inf_va(0x003789D8)
VOLUME, MTI = inf_va(0x0034BBF8), inf_va(0x00377FD0)
SYSOBJ, SAVE, HEAP = 0x01800000, 0x01810000, 0x01900000
SEED = 12345
NAME0, NAME1 = b"Kite", b"Tester"
LINES, LINE = 19, 260
MAX_FRAMES = 20000


def fnv(b):
    h = 0x811C9DC5
    for c in b:
        h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF
    return h


class Roll:
    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.p = Program(ELF, "desktop")
        self.m = m = machine_class()(self.p)
        sym = self.sym
        self.heap = HEAP
        m.mem[SYSOBJ:SYSOBJ + 0x400] = bytes(0x400)
        m.store(SYSOBJ + 0x358, 4, SEED)
        m.mem[SAVE:SAVE + 0x100] = bytes(0x100)
        m.mem[SAVE:SAVE + len(NAME0)] = NAME0
        m.mem[SAVE + 24:SAVE + 24 + len(NAME1)] = NAME1
        m.store(CCSYS, 4, SYSOBJ)
        m.store(SAVEDATA, 4, SAVE)
        m.store(VOLUME, 4, 1)
        m.store(MTI, 4, 625)
        self.draws = []

        def new(mm, n, *a):
            p = self.heap
            self.heap = (self.heap + n + 15) & ~15
            mm.mem[p:p + n] = bytes(n)
            return p

        def this(mm, t, *a):
            return t

        def nothing(mm, *a):
            return 0

        def set_frame(mm, this_, *a):
            self.draws.append("F " + " ".join(f"{mm.f[i] & 0xFFFFFFFF:08x}" for i in range(12, 20)))
            return 0

        def centre(mm, this_, *a):
            self.draws.append(f"C {mm.f[12] & 0xFFFFFFFF:08x} {mm.f[13] & 0xFFFFFFFF:08x}")
            return 0

        def set_tex(mm, this_, ccs, tex, *a):
            self.draws.append("X " + self.cstr(tex).decode("latin-1"))
            return 0

        def disp(mm, spr, s, *a):
            who = self.who(spr)
            text = self.cstr(s)
            ld = self.m.load
            self.draws.append(
                f"T {who} {ld(spr + 0x40, 4):08x} {ld(spr + 0x44, 4):08x} {ld(spr + 0x30, 4):08x}"
                f" {len(text)} {fnv(text)}"
            )
            return 0

        def make_packet(mm, spr, code, flag, *a):
            ld = self.m.load
            fl = " ".join(f"{ld(spr + o, 4):08x}" for o in (0x30, 0x40, 0x44, 0x48, 0x4C))
            ints = " ".join(str(s32(ld(spr + o, 4))) for o in (0x50, 0x54, 0x58, 0x5C, 0x60))
            self.draws.append(f"P {fl} {ints} {code} {flag}")
            return 0

        m.hooks[sym("__nw__FUi")] = new
        m.hooks[sym("__dl__FPv")] = nothing
        for name in ("__ct__8ccSpriteFv", "__ct__16ccDrawPacketCtrlFv", "__ct__16ccBufferSamplingFv",
                     "__ct__6ccMaskFii"):
            m.hooks[sym(name)] = this
        for name in ("Init__7ccKanjiFii", "Init__7ccLayerFsP6ccView", "SetActiveLayer__9WORLD_MANFi",
                     "SendPacket__8ccSpriteFv"):
            m.hooks[sym(name)] = nothing
        m.hooks[sym("SetFrame__6ccViewFffffffff")] = set_frame
        m.hooks[sym("SetLayerCenter__6ccViewFff")] = centre
        m.hooks[sym("SetTex__8ccSpriteFPcPci")] = set_tex
        m.hooks[sym("Disp__7ccKanjiFPciff")] = disp
        m.hooks[sym("MakePacket__8ccSpriteFii")] = make_packet

    def sym(self, name):
        return self.p.symbol_named(name).value

    def cstr(self, p):
        out = bytearray()
        while True:
            c = self.m.load(p + len(out), 1)
            if c == 0:
                return bytes(out)
            out.append(c)

    def who(self, spr):
        ld = self.m.load
        for i in range(LINES):
            line = self.ctrl + LINE * i
            if ld(line, 4) == spr:
                return f"L{i}"
            node, k = ld(line + 0x100, 4), 0
            while node:
                if spr == node + 8:
                    return f"N{i}.{k}"
                if spr == node + 0xF0:
                    return f"S{i}.{k}"
                node, k = ld(node + 4, 4), k + 1
        return "?"

    def state(self):
        ld = self.m.load
        c = self.ctrl
        words = [s32(ld(c + 0x1390 + 4 * k, 4)) for k in range(7)]
        floats = [f"{ld(c + o, 4):08x}" for o in (0x13CC, 0x13D0, 0x13D4, 0x13D8)]
        tail = [s32(ld(c + 0x13DC, 4)), s32(ld(c + 0x13E0, 4))]
        h = 0x811C9DC5
        for i in range(LINES):
            line = c + LINE * i
            b = bytes(self.m.mem[line + 0x0C:line + 0xEC]) + bytes(self.m.mem[line + 0xF0:line + 0xF8])
            for ch in b:
                h = ((h ^ ch) * 0x01000193) & 0xFFFFFFFF
        return " ".join(map(str, words)) + " " + " ".join(floats) + " " + " ".join(map(str, tail)) + f" {h}"

    def run(self, detail=None):
        """Every frame's record; with `detail`, that frame's draws in full
        instead."""
        out = []
        self.ctrl = self.heap
        self.heap += 0x1400
        self.m.mem[self.ctrl:self.ctrl + 0x1400] = bytes(0x1400)
        self.draws = []
        self.m.call(self.sym("__ct__17ccThStaffRollCtrlFv"), [self.ctrl])
        out.append(f"new | {self.state()} | {draws(self.draws)}")
        if detail == "new":
            return self.draws
        for f in range(MAX_FRAMES):
            self.draws = []
            self.m.call(self.sym("Main__17ccThStaffRollCtrlFv"), [self.ctrl])
            out.append(f"{f} | {self.state()} | {draws(self.draws)}")
            if detail == str(f):
                return [self.state()] + self.draws
            if self.m.load(self.ctrl + 0x13A8, 4):
                break
        return out


VIEW = 0x01880000
POINTS = [(-262.0, -192.0), (-250.0, -72.0), (0.0, 0.0), (230.0, 168.0), (-256.0, -192.0), (-256.0, -7.0)]
SCALES = [(1.0, 1.0), (5.0, 5.0), (5.0, 4.924242496490479), (5.0, 1.0), (1.0, 0.5), (1.0, 0.0333333)]


def views():
    """The layer's view as the game builds it: ccView::SetFrame(0, 0, 512,
    384, 256, 192, s1, s1), SetLayerCenter(256, 192), SetFrame again at s2
    (as _Random and _End do each frame), then ApplyLayerScreenMatrix of
    each point: "view S1 S2 X Y GX GY" (floats as bits)."""
    import struct
    from image import Program
    from test_anim import machine_class
    p = Program(ELF, "desktop")
    m = machine_class()(p)
    sym = lambda n: p.symbol_named(n).value
    m.mem[SYSOBJ:SYSOBJ + 0x400] = bytes(0x400)
    m.store(SYSOBJ + 0x0C, 2, 512)
    m.store(SYSOBJ + 0x0E, 2, 448)
    m.store(CCSYS, 4, SYSOBJ)
    bits = lambda f: struct.unpack("<I", struct.pack("<f", f))[0]
    out = []

    def frame(s):
        for i, v in enumerate((0.0, 0.0, 512.0, 384.0, 256.0, 192.0, s, s)):
            m.f[12 + i] = bits(v)
        m.call(sym("SetFrame__6ccViewFffffffff"), [VIEW])

    for s1, s2 in SCALES:
        m.mem[VIEW:VIEW + 0x300] = bytes(0x300)
        frame(s1)
        m.f[12], m.f[13] = bits(256.0), bits(192.0)
        m.call(sym("SetLayerCenter__6ccViewFff"), [VIEW])
        frame(s2)
        for x, y in POINTS:
            vec = VIEW + 0x280
            for i, v in enumerate((x, y, 0.0, 1.0)):
                m.store(vec + 4 * i, 4, bits(v))
            m.call(sym("ApplyLayerScreenMatrix__6ccViewFPiPf"), [VIEW, VIEW + 0x2C0, vec])
            gx, gy = s32(m.load(VIEW + 0x2C0, 4)), s32(m.load(VIEW + 0x2C4, 4))
            out.append(f"view {bits(s1):08x} {bits(s2):08x} {bits(x):08x} {bits(y):08x} {gx} {gy}")
    return out


def draws(d):
    """A frame's draws as the fixture keeps them: how many, and the FNV-1a
    hash of them joined by newlines."""
    return f"{len(d)} {fnv(chr(10).join(d).encode('latin-1'))}"


def s32(v):
    v &= 0xFFFFFFFF
    return v - 0x100000000 if v & 0x80000000 else v


def fixture_lines():
    head = [
        "# tools/test_staffroll_rs.py: ccThStaffRollCtrl run in eemu, seed 12345, names Kite / Tester",
        "# FRAME | status count last sub page pages done sx sy alpha bg stop_every stop_n LINESHASH"
        " | NDRAWS HASH of the draws (F setframe x8, C centre, X settex, T who x y transp len hash, P mask)",
    ]
    return head + Roll().run() + views()


@unittest.skipUnless(os.path.exists(ELF), "needs the extracted disc")
class StaffRollFixture(unittest.TestCase):
    def test_fixture_is_the_games(self):
        with open(FIXTURE) as f:
            have = f.read().splitlines()
        self.assertEqual(have, fixture_lines())


if __name__ == "__main__":
    if sys.argv[1:2] == ["detail"]:
        for line in Roll().run(detail=sys.argv[2]):
            print(line)
    elif sys.argv[1:] == ["fixture"]:
        lines = fixture_lines()
        with open(FIXTURE, "w") as f:
            f.write("\n".join(lines) + "\n")
        print(f"{len(lines) - 2} records -> {FIXTURE}")
    else:
        unittest.main()
