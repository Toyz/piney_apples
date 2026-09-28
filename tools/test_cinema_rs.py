#!/usr/bin/env python3
"""crates/piney-world's boss cinema (cinema.rs, `Cinema`) against the
game's own ccBossEffCinemaFade (gcmn 0x0046a7e0-0x0046b35c) run in eemu
beside the cinema_probe example.

The game's cinema is built by its constructor (its ccMask, its layer on
sysLayer's view and SetFrame, its two ccScFade elements, all native), with
a ccBossEffManager holding it at +0 for OnCinemaMode and OffCinemaMode.
Answered: operator new and new[] from a heap, checkPartyAnnihilation (0),
the game (its +0x24 0). Hooked and recorded: ccSprite::SetTex (the name's
file and texture), ccSprite::MakePacket (the name's transparency and cell,
place and size), ccScFade::EntryFlash (each bar's frames, colour, place
and size); the sends do nothing.

Runs: cinema on and off at random frames with numbers 2 (Skeith's magic,
a name), -1 (the Data Drain, none), 0, 5 and 70 (no name in the table),
on again while it slides in or out, Draw every frame. Compared each frame:
the name drawn (transparency bits, wv) or not, the bars' y (bits) or none,
and whether an on set the name up.

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

import test_anim  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "cinema_probe")

HEAP, HEAP_END = 0x01100000, 0x01F00000
SYS = 0x01200000                    # a ccSystem: screen 512 x 448
CC_SYS = inf_va(0x003788E0)
SYS_LAYER_VIEW = inf_va(0x00379B3C)         # sysLayer +0x2c
GAME_PTR = inf_va(0x003789CC)
COMPULSION_GAME_OVER = inf_va(0x00378C74)


def f32(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


class Game:
    def __init__(self):
        from image import Program
        self.p = Program(ELF, overlay="gcmn")
        self.m = test_anim.machine_class()(self.p)
        m = self.m
        self.sym = lambda n: self.p.symbol_named(n).value
        self.heap = HEAP
        self.rec = []
        m.mem[SYS:SYS + 0x1000] = bytes(0x1000)
        m.store(SYS + 12, 2, 512)
        m.store(SYS + 14, 2, 448)
        m.store(CC_SYS, 4, SYS)
        view = self.alloc(0x300)
        m.store(SYS_LAYER_VIEW, 4, view)
        game = self.alloc(0x100)
        m.store(GAME_PTR, 4, game)
        m.store(COMPULSION_GAME_OVER, 4, 0)
        m.hooks[self.sym("__nw__FUi")] = lambda mm, n, *_: self.alloc(n)
        m.hooks[self.sym("__nwa__FUi")] = lambda mm, n, *_: self.alloc(n)
        m.hooks[self.sym("checkPartyAnnihilation__Fv")] = lambda mm, *_: 0
        m.hooks[self.sym("SetTex__8ccSpriteFPcPci")] = self.set_tex
        m.hooks[self.sym("MakePacket__8ccSpriteFii")] = self.make_packet
        m.hooks[self.sym("SendPacket__8ccSpriteFv")] = lambda mm, *_: 0
        m.hooks[self.sym("EntryFlash__8ccScFadeFiiffff")] = self.entry_flash
        m.hooks[self.sym("SendPacket__8ccScFadeFv")] = lambda mm, *_: 0
        self.cinema = self.alloc(0x60)
        m.call(self.sym("__ct__19ccBossEffCinemaFadeFP13CinemaDataSet"), (self.cinema, 0))
        self.mgr = self.alloc(0x20)
        m.store(self.mgr, 4, self.cinema)

    def alloc(self, n):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        assert self.heap < HEAP_END
        self.m.mem[a:a + n] = bytes(n)
        return a

    def cstr(self, a):
        return bytes(self.m.mem[a:a + 32]).split(b"\0")[0].decode()

    def set_tex(self, mm, s, f, t, *_):
        self.rec.append(("settex", self.cstr(f), self.cstr(t)))
        return 0

    def make_packet(self, mm, s, code, n, *_):
        fields = [mm.load(s + o, 4) for o in (0x40, 0x44, 0x48, 0x4C, 0x50, 0x54, 0x58, 0x60)]
        self.rec.append(("name", mm.load(s + 0x30, 4), mm.load(s + 0x5C, 4, True), code, n, fields))
        return 0

    def entry_flash(self, mm, fade, t, col, *_):
        f = [mm.f[12 + k] & 0xFFFFFFFF for k in range(4)]
        self.rec.append(("flash", t, col & 0xFFFFFFFF, f))
        return 0

    def on(self, n):
        self.rec = []
        self.m.call(self.sym("OnCinemaMode__16ccBossEffManagerFi"), (self.mgr, n & 0xFFFFFFFF))
        return [r for r in self.rec if r[0] == "settex"]

    def off(self):
        self.m.call(self.sym("OffCinemaMode__16ccBossEffManagerFv"), (self.mgr,))

    def draw(self):
        self.rec = []
        self.m.call(self.sym("Draw__19ccBossEffCinemaFadeFv"), (self.cinema,))
        return self.rec


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
        self.p.stdout.close()


NAME_FIELDS = [0x43000000, 0x41200000, 0x43800000, 0x41A00000, 256, 20, 0, 1]


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class CinemaAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                        "cinema_probe"], cwd=ROOT, check=True)

    def test_frames(self):
        rng = random.Random(3)
        named = barred = drawn_names = 0
        for case in range(8):
            g, p = Game(), Probe()
            for f in range(rng.randrange(300, 600)):
                r = rng.random()
                if r < 0.03:
                    n = rng.choice([2, 2, -1, 0, 5, 70])
                    settex = g.on(n)
                    got = p.ask("on %d" % n)
                    self.assertEqual(bool(settex), got["settex"], "case %d frame %d on %d" % (case, f, n))
                    for s in settex:
                        self.assertEqual(s[1:], ("x11", "TEX_ske_skl"))
                        named += 1
                elif r < 0.06:
                    g.off()
                    p.ask("off")
                rec = g.draw()
                got = p.ask("draw")
                names = [x for x in rec if x[0] == "name"]
                flashes = [x for x in rec if x[0] == "flash"]
                want_name = [names[0][1], names[0][2]] if names else None
                for x in names:
                    self.assertEqual((x[3], x[4], x[5]), (0, 1, NAME_FIELDS))
                for x in flashes:
                    self.assertEqual((x[1], x[2], x[3][0], x[3][2], x[3][3]),
                                     (1, 0x80000000, 0, 0x44000000, 0x41F00000))
                want_bars = [x[3][1] for x in flashes] or None
                self.assertEqual((want_name, want_bars), (got["name"], got["bars"]), "case %d frame %d" % (case, f))
                barred += bool(want_bars)
                drawn_names += bool(want_name)
            p.close()
        self.assertGreater(named, 3)
        self.assertGreater(barred, 100)
        self.assertGreater(drawn_names, 50)


if __name__ == "__main__":
    unittest.main()
