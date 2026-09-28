#!/usr/bin/env python3
"""piney-game's field host (crates/piney-game/src/field_host.rs) against the
game's own code run in tools/eemu.py:

- ccScFade (fade.cpp), the fader the event scripts' `fade` and `fade_more`
  drive: EntryFade (main 0x00160400), ContinueFade (0x00160490) and
  SendPacket (0x0015fb80) with its packet work refused (GetWork answers 0),
  so that only its counting runs, over random sequences of the three on
  faders with random elements (every status bit, both chained counts,
  colours); after each call, the whole fader.
- ccGame::ChangeScene (0x00167380), the `scene` instruction, on random
  ccGame states and arguments (its ChangeRequest hooked): every field of
  ccGame and saveData.lastTown after it.

The results go to crates/piney-game/tests/field_host_fixture.txt (numbers
only); the unit tests in field_host.rs replay them through the port.

    python3 tools/test_field_host.py            check the fixture is the game's
    python3 tools/test_field_host.py fixture    write it
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
FIXTURE = os.path.join(ROOT, "crates", "piney-game", "tests", "field_host_fixture.txt")

M32 = 0xFFFFFFFF
CCSYS, SAVEDATA = inf_va(0x003788E0), inf_va(0x003789D8)
SYS, SCRATCH, SAVE, FADE, GAME = 0x01000000, 0x01010000, 0x01020000, 0x01030000, 0x01031000
LAST_TOWN = 0x8426
FADE_SIZE = 0x98
ELEM = 36


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= M32
    return v - (1 << 32) if v & 0x80000000 else v


class Game:
    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.p = Program(ELF)
        self.m = machine_class()(self.p)
        m = self.m
        self.sym = lambda n: self.p.symbol_named(n).value     # noqa: E731
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(SAVEDATA, 4, SAVE)
        m.hooks[self.sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm, *a: 0
        m.hooks[self.sym("ChangeRequest__6ccGameFii")] = lambda mm, *a: 0

    # --- ccScFade -----------------------------------------------------------------------
    def fade_state(self):
        m = self.m
        out = [s32(m.load(FADE, 4))]
        for i in range(4):
            e = FADE + 4 + ELEM * i
            out += [s16(m.load(e, 2)), s16(m.load(e + 2, 2)), s16(m.load(e + 4, 2)), s16(m.load(e + 6, 2)),
                    s16(m.load(e + 8, 2)), m.load(e + 0x1C, 4), m.load(e + 0x20, 4)]
        return out

    def fade_set(self, st):
        m = self.m
        m.mem[FADE:FADE + FADE_SIZE] = bytes(FADE_SIZE)
        m.store(FADE, 4, st[0] & M32)
        for i in range(4):
            e = FADE + 4 + ELEM * i
            status, cnt, tcnt, back, hold, c0, c1 = st[1 + 7 * i:8 + 7 * i]
            for off, v in ((0, status), (2, cnt), (4, tcnt), (6, back), (8, hold)):
                m.store(e + off, 2, v & 0xFFFF)
            m.store(e + 0x1C, 4, c0)
            m.store(e + 0x20, 4, c1)

    def fade_op(self, op):
        m = self.m
        kind = op[0]
        if kind == "entry":
            _, n, c0, c1 = op
            for i, f in enumerate([0, 0, 0x44000000, 0x43E00000]):
                m.f[12 + i] = f
            return s32(m.call(self.sym("EntryFade__8ccScFadeFiiiffff"), [FADE, n & M32, c0, c1]))
        if kind == "continue":
            _, i, n, c1 = op
            m.call(self.sym("ContinueFade__8ccScFadeFiii"), [FADE, i, n & M32, c1])
            return 0
        m.call(self.sym("SendPacket__8ccScFadeFv"), [FADE])
        return 0

    # --- ccGame::ChangeScene ------------------------------------------------------------
    def game_state(self):
        m = self.m
        return [s32(m.load(GAME + 4 * k, 4)) for k in range(0x88 // 4)] + [s16(m.load(SAVE + LAST_TOWN, 1) << 8) >> 8]

    def change_scene(self, words, last_town, args):
        m = self.m
        for k, v in enumerate(words):
            m.store(GAME + 4 * k, 4, v & M32)
        m.store(SAVE + LAST_TOWN, 1, last_town & 0xFF)
        m.call(self.sym("ChangeScene__6ccGameFiiiiii"), [GAME] + [a & M32 for a in args])
        return self.game_state()


def random_fade(rng):
    st = [rng.choice([0, 0, 0, 1])]
    for _ in range(4):
        st += [rng.choice([0, 0, 1, 1, 3, 5, 9, 13, 2, 4]), rng.randrange(-2, 30), rng.randrange(-1, 30),
               rng.randrange(0, 30), rng.randrange(0, 30), rng.getrandbits(32), rng.getrandbits(32)]
    return st


def fade_cases(rng, n):
    """[(start state, [ops])]."""
    out = []
    for _ in range(n):
        st = random_fade(rng) if rng.random() < 0.7 else [0] + [0] * 28
        ops = []
        for _ in range(rng.randrange(5, 25)):
            r = rng.random()
            if r < 0.15:
                ops.append(("entry", rng.randrange(1, 32), rng.getrandbits(32), rng.getrandbits(32)))
            elif r < 0.25:
                ops.append(("continue", rng.randrange(0, 4), rng.randrange(1, 32), rng.getrandbits(32)))
            else:
                ops.append(("send",))
        out.append((st, ops))
    # The events' own: fade 10 128, ten frames, fade_more 15 0, twenty.
    ops = [("entry", 10, 0, 128 << 24)] + [("send",)] * 11 + [("continue", 0, 15, 0)] + [("send",)] * 20
    out.append(([0] + [0] * 28, ops))
    return out


def scene_cases(rng, n):
    out = []
    for _ in range(n):
        # Towns stay within the server table (8 rows): past it the game
        # reads its own stack.
        words = [rng.randrange(-3, 12) for _ in range(0x88 // 4)]
        words[8] = rng.randrange(-3, 8)
        args = [rng.randrange(-3, 9) for _ in range(6)]
        args[1] = rng.randrange(-3, 8)
        out.append((words, rng.randrange(0, 8), args))
    # Log in's ChangeArea(0, 0) after InitScene, and event 2's scene.
    init = [5] + [0] * 4 + [-1] * 16 + [0] * 13
    out.append((init, 0, [0, 0, -1, -1, -1, -1]))
    out.append((init, 0, [1, 0, 14, -1, -1, -1]))
    return out


def write_fixture(path):
    g = Game()
    rng = random.Random(0x0E2)
    lines = [
        "# tools/test_field_host.py: ccScFade and ccGame::ChangeScene run in eemu (numbers only)",
        "# fade START...(29)        a fader: off, then 4 x (status cnt tcnt back hold col0 col1)",
        "# op entry N C0 C1 -> R STATE... / op continue I N C1 -> 0 STATE... / op send -> 0 STATE...",
        "# scene WORDS...(34) LASTTOWN ARGS...(6) -> WORDS...(34) LASTTOWN",
    ]
    for st, ops in fade_cases(rng, 40):
        g.fade_set(st)
        lines.append("fade " + " ".join(map(str, st)))
        for op in ops:
            r = g.fade_op(op)
            lines.append("op " + " ".join(map(str, op)) + " -> " + " ".join(map(str, [r] + g.fade_state())))
    for words, last, args in scene_cases(rng, 120):
        after = g.change_scene(words, last, args)
        lines.append("scene " + " ".join(map(str, words + [last] + args)) + " -> " + " ".join(map(str, after)))
    with open(path, "w") as f:
        f.write("\n".join(lines) + "\n")
    return len(lines)


@unittest.skipUnless(os.path.exists(ELF), "needs the extracted disc")
class FieldHostAgainstGame(unittest.TestCase):
    def test_fixture_is_the_games(self):
        """The committed fixture is what the game's code gives now."""
        import tempfile
        with tempfile.NamedTemporaryFile("r", suffix=".txt") as t:
            write_fixture(t.name)
            got = open(t.name).read()
        with open(FIXTURE) as f:
            self.assertEqual(f.read(), got)


if __name__ == "__main__":
    if sys.argv[1:] == ["fixture"]:
        print(write_fixture(FIXTURE), "lines ->", FIXTURE)
    else:
        unittest.main()
