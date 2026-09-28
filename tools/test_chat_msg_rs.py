#!/usr/bin/env python3
"""crates/piney-fieldui's chat balloons (chat_msg.rs, `ChatMsg`) against the
game's own ccChatMsg (main 0x001a6160-0x001a6c68) run in eemu (the Rust VU0
machine) beside the chat_probe example.

The game's ccChatMsg is built by its constructor, then driven as
ccMenuCtrl drives it: OpenChat and CloseChat now and then, Disp(still)
every frame. What they reach outside the class is answered: operator new
from a heap, the sprites and kanji made empty (ccSprite's constructor and
SetPrim, ccInitKanji), the characters (a ccChar with a base row; the
player ccPartyManager.memberChar[0]); ccCheckTarget and ccCalcTagPosChar
answer each speaker's state this frame. ccKanjiStrWidth runs natively.
Recorded: every ccSprite::MakePacket of the window (its cell, place, size,
grid, colour and alpha), every ccKanji::Disp (its text, place, colour and
alpha), and after each Disp the slots' cf, cn and scope.

The steps: four speakers, the player one of them; English lines of every
length up to the slot's 70 bytes; speakers on and off the lists, on and
off the screen, and crowded together so CheckScope stacks their balloons;
still frames; a balloon reopened over one still up; all closed.

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
                       "chat_probe")

HEAP, HEAP_END = 0x01100000, 0x01F00000
PARTY = inf_va(0x00730310)          # ccPartyManager.memberChar[0]
# gcmn: ccCheckTarget(ccChar *), ccCalcTagPosChar(ccChar *, int (&)[4], float *, int).
CHECK_TARGET, CALC_TAG_POS_CHAR = inf_va(0x00519920), inf_va(0x0051A500)

# Sample lines of the lengths the members' lines have (short, a line, two
# lines; no more than 70 bytes, `data[i]`'s room, which the game's strcpy
# overruns into the next balloon's), not the game's own.
LINES = [b"One!", b"Two words", b"A third sample.", b"The fourth sample, some words more.",
         b"The fifth sample line! It runs a bit past the others.",
         b"The sixth sample line... the longest one here, long enough to wrap!",
         b"OK", b"Sample eight.", b"Eight.", b"Ten!"]


def f32(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


class Game:
    def __init__(self):
        from image import Program
        self.p = Program(ELF)
        self.m = test_anim.machine_class()(self.p)
        self.sym = lambda n: self.p.symbol_named(n).value
        m, sym = self.m, self.sym
        self.heap = HEAP
        self.rec = []
        self.state = {}
        m.hooks[sym("__nw__FUi")] = lambda mm, n, *_: self.alloc(n)
        m.hooks[sym("__ct__8ccSpriteFv")] = lambda mm, a, *_: a
        m.hooks[sym("SetPrim__8ccSpriteFii")] = self.set_prim
        m.hooks[sym("ccInitKanji__Fii")] = self.init_kanji
        m.hooks[sym("MakePacket__8ccSpriteFii")] = self.make_packet
        m.hooks[sym("SendPacket__8ccSpriteFv")] = lambda mm, a, *_: 0
        m.hooks[sym("Disp__7ccKanjiFPciff")] = self.kanji_disp
        # ccCheckTarget and ccCalcTagPosChar are GCMN.PRG's, which is not loaded.
        m.hooks[CHECK_TARGET] = lambda mm, c, *_: int(self.state[c][0])
        m.hooks[CALC_TAG_POS_CHAR] = self.tag
        self.chars = {}
        for who in range(1, 5):
            c = self.alloc(0x100)
            base = self.alloc(0x40)
            m.store(c, 4, base)
            m.store(base + 0x18, 4, struct.unpack("<I", struct.pack("<f", 160.0))[0])
            self.chars[who] = c
        self.chat = self.alloc(0x1A0)
        m.call(sym("__ct__9ccChatMsgFv"), (self.chat,))
        self.window = m.load(self.chat, 4)

    def alloc(self, n):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        assert self.heap < HEAP_END
        self.m.mem[a:a + n] = bytes(n)
        return a

    def set_prim(self, mm, s, n, t, *_):
        mm.store(s + 0x24, 4, n)
        return 0

    def init_kanji(self, mm, n, kt, *_):
        k = self.alloc(0xE8)
        mm.store(k + 0xD4, 2, kt)
        return k

    def colour(self, s):
        m = self.m
        return [m.load(s + 0x68, 1), m.load(s + 0x6C, 1), m.load(s + 0x70, 1), m.load(s + 0x74, 4)]

    def make_packet(self, mm, s, code, n, *_):
        if s == self.window:
            r, g, b, a = self.colour(s)
            self.rec.append(("cell", [code] + [mm.load(s + o, 4) for o in (0x40, 0x44, 0x48, 0x4C)] +
                             [mm.load(s + o, 4, True) for o in (0x50, 0x54, 0x58, 0x5C, 0x60)] + [r, g, b, a & 0xFF]))
        return 0

    def kanji_disp(self, mm, k, text, *_):
        s = bytes(mm.mem[text:text + 71]).split(b"\0")[0]
        r, g, b, a = self.colour(k)
        self.rec.append(("text", [list(s), mm.load(k + 0x40, 4), mm.load(k + 0x44, 4), r, g, b, a & 0xFF]))
        return 0

    def tag(self, mm, c, pos, off, mode, *_):
        listed, on, x, y = self.state[c]
        mm.store(pos, 4, x & 0xFFFFFFFF)
        mm.store(pos + 4, 4, y & 0xFFFFFFFF)
        return int(on)

    def open(self, who, text):
        m = self.m
        a = self.alloc(len(text) + 1)
        m.mem[a:a + len(text)] = text
        m.store(a + len(text), 1, 0)
        m.call(self.sym("OpenChat__9ccChatMsgFP6ccCharPc"), (self.chat, self.chars[who], a))

    def close(self):
        self.m.call(self.sym("CloseChat__9ccChatMsgFv"), (self.chat,))

    def disp(self, still, player, state):
        m = self.m
        m.store(PARTY, 4, self.chars[player])
        self.state = {self.chars[w]: s for w, s in state.items()}
        self.rec = []
        m.call(self.sym("Disp__9ccChatMsgFi"), (self.chat, int(still)))
        c = self.chat
        slots = [[m.load(c + 0x130 + 2 * i, 2, True), m.load(c + 0x138 + 2 * i, 2, True)] +
                 [m.load(c + 0x160 + 16 * i + 4 * k, 4, True) for k in range(3)] for i in range(4)]
        return {"slots": slots, "cells": [r for k, r in self.rec if k == "cell"],
                "texts": [r for k, r in self.rec if k == "text"]}


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


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class ChatMsgAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-fieldui", "--example",
                        "chat_probe"], cwd=ROOT, check=True)

    def test_frames(self):
        rng = random.Random(7)
        shown = stacked = faded = 0
        for case in range(12):
            g, p = Game(), Probe()
            player = rng.randrange(1, 5)
            base = (rng.randrange(40, 480), rng.randrange(40, 400))
            where = {w: [True, True, base[0] + rng.randrange(-60, 60), base[1] + rng.randrange(-30, 30)]
                     for w in range(1, 5)}
            for f in range(rng.randrange(150, 400)):
                r = rng.random()
                if r < 0.06:
                    who, text = rng.randrange(1, 5), rng.choice(LINES)
                    g.open(who, text)
                    p.ask("open %x %s" % (who, text.hex()))
                elif r < 0.065:
                    g.close()
                    p.ask("close")
                for w, s in where.items():
                    if rng.random() < 0.03:
                        s[0] = rng.random() < 0.85
                    if rng.random() < 0.05:
                        s[1] = rng.random() < 0.85
                    if rng.random() < 0.3:
                        s[2] += rng.randrange(-6, 7)
                        s[3] += rng.randrange(-4, 5)
                still = rng.random() < 0.05
                want = g.disp(still, player, {w: tuple(s) for w, s in where.items()})
                args = " ".join("%x %x %x %x %x" % (w, int(s[0]), int(s[1]), s[2] & 0xFFFFFFFF, s[3] & 0xFFFFFFFF)
                                for w, s in where.items())
                got = p.ask("disp %x %x %s" % (int(still), player, args))
                self.assertEqual(want, got, "case %d frame %d" % (case, f))
                shown += len(want["texts"])
                ys = [s[3] for s in want["slots"] if s[0] > 0]
                stacked += len(ys) != len(set(ys)) or any(abs(a - b) == 30 for a in ys for b in ys)
                faded += any(t[6] < 128 for t in want["texts"])
            p.close()
        self.assertGreater(shown, 100)
        self.assertGreater(stacked, 0)
        self.assertGreater(faded, 0)


if __name__ == "__main__":
    unittest.main()
