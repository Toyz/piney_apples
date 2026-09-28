#!/usr/bin/env python3
"""crates/piney-desktop's name entry (name_entry.rs) against the game's own
NameEntry_Control (desktop.prg, NameEntry.cpp) run in tools/eemu.py.

Both sides start from the constructor and Init (0x004195b0) and run one
Main (0x00414310) a frame on the same pad fields (ccSys push, unpush and
repeat, which CurRepeat reads); the port through the desktop_probe example's
`nerun`. Every frame compares the logic: MainAct, CurAct, the cursor, the
name act and count, SW, the fade (Tlans, Transparency, winTrans), Err, the
dialog, WinAct, InfoCount, the locks, StrMaxNum, Q, NowMode, m_NO,
m_DispCurPos, CurRepeat's counters, the blink, NameFlg, TempMainAct and
TmpWinAct; the user, character and temporary names, m_cbuf, m_Blockbuf,
TrueSurName and the two entry names; the sounds and Main's return. When Main
returns 1 the names SetSaveData wrote (plName, plRealName) are compared.

On the frames asked, every draw call is compared in order: ccKanji::Disp
(object, kt, shadow, position bits, colour, text), ccSprite::MakePacket
(object, cell, dx dy sx sy cx cy bits, the grid, colour, U flip),
SendPacket, and the system font's MakePacketStr, with the game's
ccMenuWindow (DispSquare, DispSelectCursol, DispPageCursol, DispTarget) and
ccFacePanel::Draw running for real.

Runs: random pads (NAME_ENTRY_RUNS of them, 2500 frames each, the logic
every frame; the first two with every draw call as well); a scripted pass
through every screen (the refusals, Default, One Back, a forbidden name, a
full name, NO and YES, the last pages) with every draw call; and
newGameFlag set, where Main returns 1 at once.

Skipped when the disc is not extracted or cargo is missing.
"""

import functools
import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
inf_va = functools.partial(va, overlay='desktop')  # this overlay's globals

from test_desktop_rs import ELF, ISO, ask, build  # noqa: E402
import test_save_init_rs  # noqa: E402

# gp globals (INF SLUS_202.67 .sdata).
CCSYS, SAVEDATA, GAME, MENUWIN, FONT = inf_va(0x003788e0), inf_va(0x003789d8), inf_va(0x003789cc), inf_va(0x00378a9c), inf_va(0x00378954)
# Scratch memory for the interpreter.
SYS, SAVE, GAMEOBJ, HEAP = 0x01800000, 0x01808000, 0x01831000, 0x01900000
OK, CANCEL = 0x40, 0x20
UP, RIGHT, DOWN, LEFT, START = 0x1000, 0x2000, 0x4000, 0x8000, 0x800
# desktop.prg: the constructor, Main, the unit's static initialiser, DrawWindow.
CTOR, MAIN, SINIT, DRAW_WINDOW = inf_va(0x00419520), inf_va(0x00414310), inf_va(0x00469e60), inf_va(0x00415ed0)
EIGO_BLOCK = inf_va(0x0042c7e0)
# spcParam[0]: the name pointer LoadGame sets, maxHP, maxSP.
SPC_NAME, SPC_HP, SPC_SP = 0x7488, 0x74ac, 0x74ae
HP, SP = 100, 40
RUNS = int(os.environ.get("NAME_ENTRY_RUNS", "12"))
FRAMES = 2500


def f_bits(m, a):
    return m.load(a, 4)


class Pads:
    """The pad fields from the buttons held each frame: push and unpush on a
    change, repeat on a change and again from the sixteenth frame held."""

    def __init__(self):
        self.prev = 0
        self.cnt = 0

    def next(self, now):
        d = self.prev
        push, unpush = now & ~d, d & ~now
        if now and now == d:
            if self.cnt < 15:
                self.cnt += 1
                rep = 0
            else:
                rep = now
        else:
            rep = now
            self.cnt = 0
        self.prev = now
        return push, unpush, rep


class Game:
    """NameEntry_Control in eemu: new + the constructor (Init), then Main a
    frame. `events` collects a frame's draw calls in the probe's format."""

    ARRAYS = ((0x120, "List", 18), (0x144, "DamyList", 18), (0x128, "KanjiList", 6), (0x130, "Info", 4),
              (0x134, "NameInfo", 4), (0x138, "RealName", 2), (0x13c, "NamePrev", 6))
    SINGLES = ((0x11c, "menu"), (0x124, "EnterEnd"), (0x12c, "kanjiName"), (0x140, "KanjiInfo"),
               (0x25c, "Windows"), (0x260, "WinWaku"))

    def __init__(self, p, hp=HP, sp=SP, new_game=0, draw=True):
        from eemu import Machine
        self.p = p
        self.m = m = Machine(p)
        self.heap = HEAP
        self.events = []
        self.se = []
        self.names = {}
        m.store(CCSYS, 4, SYS)
        m.store(SAVEDATA, 4, SAVE)
        m.store(GAME, 4, GAMEOBJ)
        m.store(GAMEOBJ, 4, 2)                  # game.status: the desktop
        m.store(SYS + 0, 4, 1)                  # frameRate
        # The port's fresh save, made by the game's own boot (ok cross,
        # cancel circle, the player named Kite).
        m.mem[SAVE:SAVE + 0x8530] = test_save_init_rs.fresh_save(ELF)
        m.store(SAVE + 0x6770, 1, new_game)
        m.store(SAVE + SPC_NAME, 4, SAVE)
        m.store(SAVE + SPC_HP, 2, hp)
        m.store(SAVE + SPC_SP, 2, sp)
        hooks = {
            "__nw__FUi": self._new, "__nwa__FUi": self._new,
            "Init__7ccKanjiFii": self._kanji_init,
            "Init__7ccLayerFsP6ccView": lambda mm, *a: 0,
            "__ct__16ccDrawPacketCtrlFv": lambda mm, a0, *a: a0,
            "SetTex__8ccSpriteFPcPci": lambda mm, *a: 0,
            "CopyTex__8ccSpriteFP8ccSprite": lambda mm, *a: 0,
            "SetPrim__8ccSpriteFii": lambda mm, *a: 0,
            "MakePacket__8ccSpriteFii": self._pkt,
            "MakePacketStr__8ccSpriteFPci": self._pkt_str,
            "SendPacket__8ccSpriteFv": lambda mm, a0, *a: self.events.append(["send", self.name(a0)]) or 0,
            "Trans__8ccSpriteFv": lambda mm, *a: 0,
            "Disp__7ccKanjiFPciff": self._disp,
            "ccSeOn__Fi": lambda mm, a0, *a: self.se.append(a0) or 0,
            "ccPortVolSet__FiUs": lambda mm, *a: 0,
            "strstr": self._strstr, "strlwr": self._strlwr, "strncpy": self._strncpy, "sprintf": self._sprintf,
        }
        for n, f in hooks.items():
            s = p.symbol_named(n)
            assert s is not None, n
            m.hooks[s.value] = f
        if not draw:
            # The logic runs only: DrawWindow draws, and the cursor it and
            # DrawCur move is drawing state.
            m.hooks[DRAW_WINDOW] = lambda mm, *a: 0
            m.hooks[p.symbol_named("DispSelectCursol__12ccMenuWindowFffiiii").value] = lambda mm, *a: 0
        m.call(SINIT)
        # The system font the face panel writes "---/---" with.
        font = self._new(m, 0xd0)
        m.call(p.symbol_named("__ct__8ccSpriteFv").value, [font])
        m.store(FONT, 4, font)
        self.names[font] = "font"
        self.this = t = self._new(m, 0x268)
        m.call(CTOR, [t])
        w = lambda o: m.load(t + o, 4)  # noqa: E731
        for o, n in self.SINGLES:
            self.names[w(o)] = n
        for o, n, c in self.ARRAYS:
            for i in range(c):
                self.names[w(o) + 0xe8 * i] = f"{n}[{i}]"
        face = w(0x264)
        self.names[m.load(face, 4)] = "Face.spr0"
        self.names[m.load(face + 4, 4)] = "Face.spr1"
        self.names[m.load(MENUWIN, 4)] = "menuWin"
        self.done = False

    # -- hooks ------------------------------------------------------------
    def _new(self, m, n, *_):
        a = self.heap
        self.heap = (a + n + 15) & ~15
        m.mem[a:a + n] = bytes(n)
        return a

    def _kanji_init(self, m, obj, *_):
        m.store(obj + 4, 4, m.load(obj + 4, 4) | 0x11)
        m.store(obj + 0xd4, 2, 0)
        return 0

    def name(self, a):
        return self.names.get(a, hex(a))

    def _colour(self, obj):
        return [self.m.load(obj + o, 4) & 0xff for o in (0x68, 0x6c, 0x70, 0x74)]

    def _pkt(self, m, obj, code, *_):
        from eemu import f_add
        f = [f_bits(m, obj + o) for o in (0x40, 0x44, 0x48, 0x4c, 0x34, 0x38)]
        g = [m.load(obj + o, 4, True) for o in (0x50, 0x54, 0x58, 0x5c, 0x60)]
        self.events.append(["pkt", self.name(obj), code, f] + g + [self._colour(obj), m.load(obj + 4, 4) >> 5 & 1])
        m.store(obj + 0x40, 4, f_add(f[0], f[2]))
        return 0

    def _pkt_str(self, m, obj, s, raw, *_):
        from eemu import _cstr, f_add
        text = _cstr(m, s)
        f = [f_bits(m, obj + o) for o in (0x40, 0x44, 0x48, 0x4c)]
        g = [m.load(obj + o, 4, True) for o in (0x50, 0x54, 0x58, 0x5c, 0x60)]
        self.events.append(["str", self.name(obj), f] + g + [self._colour(obj), text.hex()])
        dx = f[0]
        for _ in text:
            dx = f_add(dx, f[2])
        m.store(obj + 0x40, 4, dx)
        return 0

    def _disp(self, m, obj, s, c, *_):
        from eemu import _cstr
        assert c == 0xffffffff, c
        self.events.append(["kanji", self.name(obj), m.load(obj + 0xd4, 2, True), m.load(obj + 4, 4) >> 4 & 1,
                            f_bits(m, obj + 0x40), f_bits(m, obj + 0x44), self._colour(obj), _cstr(m, s).hex()])
        return 0

    def _strstr(self, m, a, b, *_):
        from eemu import _cstr
        i = _cstr(m, a).find(_cstr(m, b))
        return 0 if i < 0 else a + i

    def _strlwr(self, m, a, *_):
        from eemu import _cstr
        s = _cstr(m, a).lower()
        m.mem[a:a + len(s)] = s
        return a

    def _strncpy(self, m, d, s, n, *_):
        from eemu import _cstr
        src = _cstr(m, s)
        m.mem[d:d + n] = (src[:n] + bytes(max(0, n - len(src))))[:n]
        return d

    def _sprintf(self, m, d, fmt, a2, a3, *_):
        from eemu import _cstr
        f = _cstr(m, fmt)
        assert f == b"%s%s%s#W%s", f
        args = [a2, a3, m.r[8] & 0xffffffff, m.r[9] & 0xffffffff]
        out = b"%s%s%s#W%s".replace(b"%s", b"\0")
        parts = out.split(b"\0")
        text = parts[0]
        for k, rest in enumerate(parts[1:]):
            text += _cstr(m, args[k]) + rest
        m.mem[d:d + len(text) + 1] = text + b"\0"
        return len(text)

    # -- state ------------------------------------------------------------
    def cstr(self, a):
        from eemu import _cstr
        return _cstr(self.m, a)

    def state(self):
        m, t = self.m, self.this
        h = lambda o: m.load(t + o, 2, True)  # noqa: E731
        w = lambda o: m.load(t + o, 4, True)  # noqa: E731
        st = [h(0x230), h(0x236), w(0x14c), w(0x150), h(0x23a), h(0x10e), h(0x148), h(0x242), w(0x1f8),
              w(0x21c), h(0x1f4), h(0x23c), w(0x20c), h(0x218), w(0x210), w(0x214), h(0x22c), h(0x240), h(0),
              w(0x200), w(4), w(0x16c), w(0x170), h(0x112), h(0x114), w(0x224), w(0x228), h(0x21a), h(0x232),
              h(0x23e)]
        strs = [self.cstr(t + o).hex() for o in (0x24, 0x38, 0x4c, 0x1e2)]
        strs.append(self.cstr(m.load(t + 8, 4)).hex())
        strs += [self.cstr(t + o).hex() for o in (0x74, 0x9c, 0xe8)]
        return st, strs

    def frame(self, push, unpush, repeat, draw):
        m = self.m
        for o, v in ((0x2cc, push | repeat), (0x2d0, push), (0x2d4, unpush), (0x2d8, repeat)):
            m.store(SYS + o, 4, v)
        self.events, self.se = [], []
        ret = m.call(MAIN, [self.this])
        st, strs = self.state()
        out = {"st": st, "str": strs, "se": self.se, "ret": ret}
        if draw:
            out["ev"] = self.events
        return out

    def names_written(self):
        return [self.cstr(SAVE).hex(), self.cstr(SAVE + 0x18).hex()]


def ask_run(frames, hp=HP, sp=SP, new_game=0):
    """The port over the same pad fields: [(push, unpush, repeat, draw)]."""
    lines = [f"nerun {hp} {sp} {new_game}"] + [f"pad {p} {u} {r} {int(d)}" for p, u, r, d in frames] + ["end"]
    return ask(lines)[0]


def normalise(ev):
    """JSON lists for tuples."""
    return [list(e) if isinstance(e, tuple) else e for e in ev]


def random_pads(rng, n):
    choices = [0, 0, UP, DOWN, LEFT, RIGHT, OK, CANCEL, START, UP | OK, LEFT | DOWN, OK | CANCEL]
    seq = []
    while len(seq) < n:
        seq += [rng.choice(choices)] * rng.choice([1, 1, 1, 2, 3, 20, 45, 80])
        seq += [0] * rng.choice([0, 1, 1, 2, 5])
    return seq[:n]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class NameEntryAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        build()
        cls.p = Program(ELF, "desktop")

    def compare(self, label, game_frames, port):
        """Frame by frame: state, strings, sounds, return and draw calls."""
        self.assertEqual(len(port["frames"]), len(game_frames), label)
        for f, (g, q) in enumerate(zip(game_frames, port["frames"])):
            for k in ("st", "str", "se", "ret"):
                if g[k] != q[k]:
                    self.fail(f"{label} frame {f}: {k}\n game {g[k]}\n port {q[k]}")
            if "ev" in g:
                ge, qe = normalise(g["ev"]), q["ev"]
                if ge != qe:
                    for i, (a, b) in enumerate(zip(ge, qe)):
                        if a != b:
                            self.fail(f"{label} frame {f}: draw call {i}\n game {a}\n port {b}")
                    self.fail(f"{label} frame {f}: {len(ge)} draw calls in the game, {len(qe)} in the port\n"
                              f" game tail {ge[len(qe):len(qe) + 3]}\n port tail {qe[len(ge):len(ge) + 3]}")

    def run_pads(self, label, seq, draw_every=0):
        game = Game(self.p, draw=draw_every != 0)
        pads = Pads()
        frames, game_frames = [], []
        for f, now in enumerate(seq):
            push, unpush, rep = pads.next(now)
            draw = draw_every != 0 and f % draw_every == 0
            g = game.frame(push, unpush, rep, draw)
            frames.append((push, unpush, rep, draw))
            game_frames.append(g)
            if g["ret"]:
                break
        port = ask_run(frames)
        self.compare(label, game_frames, port)
        if game_frames[-1]["ret"]:
            self.assertEqual(port["names"], game.names_written(), label)
        return game_frames

    def test_random_pads(self):
        finished = 0
        for seed in range(RUNS):
            rng = random.Random(seed)
            seq = random_pads(rng, FRAMES)
            gf = self.run_pads(f"seed {seed}", seq, draw_every=1 if seed < 2 else 0)
            finished += gf[-1]["ret"]
        print(f"name entry: {RUNS} random runs of up to {FRAMES} frames, {finished} finished")

    def test_scripted_screens(self):
        d = Drive(self)
        d.idle(20)
        d.press(OK)
        d.idle(10)
        d.press(OK)
        d.wait_ready()
        self.assertEqual(d.st()["MainAct"], 7)
        d.menu(3)                           # Enter on no name: Err 1
        self.assertEqual(d.st()["Err"], 1)
        d.menu(2)                           # One Back on the user name: Err 2
        self.assertEqual(d.st()["Err"], 2)
        d.type(b"   ")
        d.menu(3)                           # spaces only: Err 1
        self.assertEqual(d.st()["Err"], 1)
        d.menu(1)                           # Default
        d.type(b"Helba")
        d.menu(3)
        d.wait_ready()                      # on to the character name
        self.assertEqual(d.st()["NameAct"], 2)
        for _ in range(4):
            d.press(CANCEL)
        d.press(CANCEL)                     # nothing left to delete
        d.type(b"bal Mung")
        d.menu(3)                           # a forbidden name: Err 3
        self.assertEqual(d.st()["Err"], 3)
        d.press(LEFT)
        d.menu(2)
        d.wait_ready()                      # One Back to the user name
        self.assertEqual(d.st()["NameAct"], 1)
        d.menu(3)
        d.wait_ready()
        d.menu(1)
        d.type(b"0123456789")               # full: the cursor jumps to Enter
        self.assertEqual(d.st()["CurAct"], 3)
        d.press(OK)
        d.wait_ready()                      # the dialog
        self.assertEqual(d.st()["MainAct"], 17)
        d.press(CANCEL)
        d.wait_ready()                      # back to Enter
        d.press(OK)
        d.wait_ready()
        d.press(UP)
        d.press(OK)
        d.wait_ready()                      # YES: the last pages
        while not d.game_frames[-1]["ret"]:
            d.step(OK if len(d.pads) % 20 == 0 else 0)
            self.assertLess(len(d.pads), 3000)
        port = ask_run(d.pads)
        self.compare("scripted", d.game_frames, port)
        self.assertEqual(port["names"], d.game.names_written())
        self.assertEqual(bytes.fromhex(port["names"][0]), b"Kite0123456789")
        self.assertEqual(bytes.fromhex(port["names"][1]), b"Helba")

    def test_new_game_flag(self):
        game = Game(self.p, new_game=1)
        g = game.frame(0, 0, 0, True)
        self.assertEqual(g["ret"], 1)
        port = ask_run([(0, 0, 0, True)], new_game=1)
        self.compare("newGameFlag", [g], port)
        self.assertEqual(port["names"], game.names_written())


class Drive:
    """A scripted pass over the game, which decides its pads on the game's
    own state; every frame's draw calls recorded."""

    FIELDS = ["MainAct", "CurAct", "X", "Y", "NameAct", "NameNum", "SW", "Tlans", "Trans", "Err", "dialog", "WinAct",
              "InfoCount", "ALL_Lock", "Play_Lock", "Move_Lock"]

    def __init__(self, test):
        self.game = Game(test.p)
        self.pads = []
        self.game_frames = []
        self.p = Pads()
        # Where each character is on the grid (EigoBlock, read from the image).
        self.pos = {}
        m = self.game.m
        for y in range(6):
            for x in range(15):
                c = m.load(EIGO_BLOCK + 14 * (3 * y + x // 5) + 2 * (x % 5), 1)
                self.pos.setdefault(c, (x, y))

    def st(self):
        return dict(zip(self.FIELDS, self.game.state()[0]))

    def step(self, now=0):
        push, unpush, rep = self.p.next(now)
        self.game_frames.append(self.game.frame(push, unpush, rep, True))
        self.pads.append((push, unpush, rep, True))

    def press(self, b, gap=2):
        self.step(b)
        for _ in range(gap):
            self.step(0)

    def idle(self, n):
        for _ in range(n):
            self.step(0)

    def wait_ready(self):
        for _ in range(200):
            s = self.st()
            if s["Tlans"] == 0 and s["Move_Lock"] == 0:
                return
            self.step(0)
        raise RuntimeError("the fade does not settle")

    def goto(self, x, y):
        if self.st()["CurAct"] != 0:
            self.press(DOWN)
        s = self.st()
        while s["Y"] != y:
            self.press(DOWN if s["Y"] < y else UP)
            s = self.st()
        while s["X"] != x:
            self.press(RIGHT if s["X"] < x else LEFT)
            s = self.st()

    def type(self, text):
        for c in text:
            self.goto(*self.pos[c])
            self.press(OK)

    def menu(self, item):
        if self.st()["CurAct"] == 0:
            self.press(START)
        while self.st()["CurAct"] != item:
            self.press(RIGHT)
        self.press(OK)


if __name__ == "__main__":
    unittest.main()
