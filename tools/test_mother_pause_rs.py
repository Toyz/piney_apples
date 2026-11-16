#!/usr/bin/env python3
"""ccThMother's pause with no controller in port 1 (main, INF
0x00167d28-0x00168018) against crates/piney-demo's pause (pause_probe).

The game's ccThMother runs in tools/eemu.py from its loop (the request
queue empty, so each pass comes to the pause's test), frame by frame: a
`Breath` or `ccBreathThread` ends a frame. Its own code runs natively, and
so do `ccSleepNoSleepThread`, `ccSleepThread`, `ccWakeAllThread`,
`ccWakeThread`, `ccFont::SetType`, `ccScFade::EntryFade`, `DeleteFade` and
`SendPacket` (the fade's counting) over a task list of the mother, one
task and the event task. Hooked: the layers' and font's flips (logged),
the drawing under `SendPacket`, and `MakePacketStr` (logged with the
sprite's fields). Each frame's scenario sets `eventMng.phase`,
`game.pauseFlag`, `ccSys.pad[0].status` and the pad's push.

Compared each frame: the calls, the texts (floats as bits), `ccGame.fade`'s
four elements, and whether the tasks and the event task run (a task runs
in a frame when its sleep count was 0 at the last frame's end and still is
after the mother's pass).

PINEY_VOLUME picks the disc. Skipped when it is not extracted or cargo is
missing.
"""

import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import volume  # noqa: E402
try:
    import eemu_rs as eemu  # noqa: E402
except ImportError:
    import eemu  # noqa: E402

ELF = volume.ELF
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
PROBE = os.path.join(TARGET, "release", "examples", "pause_probe")
Stop = eemu.Stop

GAME, FADE, LAYER, VIEW, EVMNG, SYS = 0x01000000, 0x01001000, 0x01002000, 0x01003000, 0x01004000, 0x01010000
FONTSYS, FONTDEF, WORK, TRAMP = 0x01020000, 0x01021000, 0x01100000, 0x01200000
TSCBS = 0x01030000          # head, mother, a task, the event task: 0x100 apart
START = 0x800


def u32(v):
    return v & 0xFFFFFFFF


class Game:
    def __init__(self, frames):
        from test_anim import machine_class
        from image import Program
        self.p = Program(ELF)
        self.m = machine_class()(self.p)
        self.frames = frames
        self.out = []
        self.log = []
        self.sleeps = [0, 0]
        self.f = 0

    def sym(self, name):
        s = self.p.symbol_named(name)
        assert s is not None, name
        return s.value

    def mother(self):
        """ccThMother's span, the loop's head (the last backward `b`'s
        target), the registers its prologue keeps the task in and its
        start-up keeps the event task in, and its frame size."""
        lo = self.sym("ccThMother__FPv")
        hi = lo + self.p.symbol_named("ccThMother__FPv").size
        words = [self.p.u32(a) for a in range(lo, hi, 4)]
        frame = 0x10000 - (words[0] & 0xFFFF)
        own = next((w >> 11) & 31 for w in words[:12] if w & 0xFFE007FF == 0x0080002D)
        ev_fn, start_fn = self.sym("ccThEvent__FP6ccTscb"), self.sym("ccStartThread__FPFPv_vii")
        jal = 0x0C000000 | (start_fn >> 2)
        ev = None
        for i, w in enumerate(words):
            if w == jal and any(x & 0xFFFF == ev_fn & 0xFFFF and x >> 16 == 0x2484 for x in words[i - 4:i]):
                ev = next((x >> 11) & 31 for x in words[i + 1:i + 4] if x & 0xFFFF07FF == 0x0040002D)
                break
        head = None
        for i in range(len(words) - 1, 0, -1):
            w = words[i]
            if w >> 16 == 0x1000 and w & 0x8000:
                head = lo + 4 * (i + 1) + 4 * ((w & 0xFFFF) - 0x10000)
                break
        return head, own, ev, frame

    def setup(self, sc):
        m, sym = self.m, self.sym
        for a in (GAME, FADE, LAYER, VIEW, EVMNG, SYS, FONTSYS, FONTDEF, TSCBS):
            m.mem[a:a + 0x1000] = bytes(0x1000)
        m.store(GAME + 0x80, 4, LAYER)
        m.store(GAME + 0x84, 4, FADE)
        m.store(FADE + 0x94, 4, LAYER)
        m.store(LAYER + 0x2C, 4, VIEW)
        m.store(SYS + 0x186, 1, sc["still"])
        gp = lambda name, v: m.store(sym(name), 4, v)  # noqa: E731
        gp("game", GAME)
        gp("eventMng", EVMNG)
        gp("ccSys", SYS)
        gp("fontSys", FONTSYS)
        gp("font", FONTDEF)
        gp("tscbCtrl", TSCBS)
        m.call(sym("__ct__8ccSpriteFv"), [FONTSYS], limit=1_000_000)
        # The task list: head -> mother (mask 3) -> a task -> the event task (mask 3).
        for k in range(4):
            t = TSCBS + 0x100 * k
            m.store(t + 0x04, 2, k)
            m.store(t + 0x10, 4, {1: 3, 3: 3}.get(k, 0))
            m.store(t + 0x34, 4, TSCBS + 0x100 * (k + 1) if k < 3 else 0)
        self.mother_t, self.task_t, self.event_t = TSCBS + 0x100, TSCBS + 0x200, TSCBS + 0x300
        self.sleeps = [0, 0]

    def hooks(self):
        m, sym = self.m, self.sym
        h = m.hooks
        for name, tag in (("fontOffFlip__Fv", "font_off"), ("fontOnFlip__Fv", "font_on"),
                          ("OffFlipExcept__7ccLayerFv", "layers_off"), ("OnFlipExcept__7ccLayerFv", "layers_on")):
            h[sym(name)] = (lambda t: lambda mm, *a: self.log.append(t) or 0)(tag)
        h[sym("GetThreadId")] = lambda mm, *a: 1
        h[sym("SleepThread")] = lambda mm, *a: self.log.append("kernel_sleep") or 0
        h[sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm, *a: WORK
        h[sym("ApplyLayerScreenMatrix__6ccViewFPiPf")] = lambda mm, *a: 0
        h[sym("SetTag__8ccScFadeFP13ccScFadeTexDL")] = lambda mm, *a: 0
        h[sym("sceVu0FTOI0Vector")] = lambda mm, *a: 0

        def make_str(mm, this, s, mode, *a):
            b = bytes(mm.mem[s:s + 128]).split(b"\0")[0]
            i = lambda o: mm.load(this + o, 4, True)  # noqa: E731
            rgba = [mm.load(this + 0x68 + 4 * k, 4) & 0xFF for k in range(4)]
            self.log.append(["str", b.hex(), mm.load(this + 0x40, 4), mm.load(this + 0x44, 4),
                             mm.load(this + 0x48, 4), mm.load(this + 0x4C, 4), i(0x50), i(0x54), i(0x58),
                             i(0x5C), i(0x60), rgba])
            return 0
        h[sym("MakePacketStr__8ccSpriteFPci")] = make_str
        h[sym("Breath__6ccTscbFi")] = lambda mm, *a: self.breath()
        h[sym("ccBreathThread__Fi")] = lambda mm, *a: self.breath()

    def frame_input(self):
        playing, flag, plugged, push = self.sc["frames"][self.f]
        m = self.m
        m.store(EVMNG + 0x0C, 4, 4 if playing else 2)
        m.store(GAME + 0x7C, 4, flag)
        m.store(SYS + 0x26C, 4, 6 if plugged else 0)
        m.store(SYS + 0x2D0, 4, push)

    def breath(self):
        m = self.m
        s = [m.load(self.task_t + 8, 2, True), m.load(self.event_t + 8, 2, True)]
        runs = [self.sleeps[k] == 0 and s[k] == 0 for k in range(2)]
        elems = []
        for k in range(4):
            e = FADE + 4 + 0x24 * k
            elems.append([m.load(e, 2, True), m.load(e + 2, 2, True), m.load(e + 4, 2, True), m.load(e + 0x1C, 4),
                          m.load(e + 0x20, 4)])
        self.out.append({"log": self.log, "fade": elems, "tasks": runs[0], "event": runs[1],
                         "still": m.load(SYS + 0x186, 1, True), "ev_mask": m.load(self.event_t + 0x10, 4)})
        self.log = []
        self.sleeps = s
        self.f += 1
        if self.f >= len(self.sc["frames"]):
            raise Stop("done")
        self.frame_input()
        return 0

    def run(self, sc):
        self.sc = sc
        self.setup(sc)
        self.hooks()
        head, own, ev, size = self.mother()
        m = self.m
        # A trampoline: the two task registers and the frame, then into the
        # loop (call() starts every register afresh).
        code = []
        for r, v in ((own, self.mother_t), (ev, self.event_t)):
            code += [0x3C000000 | r << 16 | v >> 16, 0x34000000 | r << 21 | r << 16 | v & 0xFFFF]
        code += [0x27BD0000 | (-size & 0xFFFF), 0x08000000 | (head >> 2) & 0x03FFFFFF, 0]
        for k, w in enumerate(code):
            m.store(TRAMP + 4 * k, 4, w)
        self.frame_input()
        try:
            m.call(TRAMP, [self.mother_t], limit=500_000_000)
        except Stop as e:
            if str(e) != "done":
                raise
        return self.out


def port(sc):
    lines = ["f %d %d %d %d" % tuple(f) for f in sc["frames"]]
    p = subprocess.run([PROBE, str(volume.number())], input="\n".join(lines) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(x) for x in p.stdout.splitlines()]


CALLS = {"flip_off": ["font_off", "layers_off"], "flip_on": ["font_on", "layers_on"]}


def norm_game(fr):
    """A game frame as the probe prints it: the flips paired, the sleeps and
    wakes read from the task list's counts, the texts."""
    calls, strs = [], []
    for e in fr["log"]:
        if isinstance(e, list):
            strs.append(e[1:])
        else:
            calls.append(e)
    return calls, strs


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class MotherPause(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-demo", "--example",
                        "pause_probe"], cwd=ROOT, check=True)

    def compare(self, sc, what):
        g = Game(len(sc["frames"])).run(sc)
        p = port(sc)
        self.assertEqual(len(g), len(p), what)
        still, mask = sc["still"], 3
        for f, (a, b) in enumerate(zip(g, p)):
            calls, strs = norm_game(a)
            want = []
            for c in b["calls"]:
                want += CALLS.get(c, [])
            self.assertEqual([c for c in calls if c not in ("kernel_sleep",)], want, f"{what} frame {f}: calls")
            self.assertEqual(strs, b["str"], f"{what} frame {f}: texts")
            self.assertEqual(a["fade"], b["fade"], f"{what} frame {f}: fade")
            self.assertEqual((a["tasks"], a["event"]), (b["tasks"], b["event"]), f"{what} frame {f}: tasks")
            for c in b["calls"]:
                if c == "still_down":
                    still = max(still - 1, 0)
                elif c == "sleep_event":
                    mask &= ~2
                elif c == "wake_event":
                    mask |= 2
            self.assertEqual(a["still"], still, f"{what} frame {f}: still layer")
            self.assertEqual(a["ev_mask"], mask, f"{what} frame {f}: the event task's mask")

    def test_unplugged_and_back(self):
        # Playing; the controller out at 3; back at 8; START at 12.
        frames = [[1, 0, 1, 0]] * 3 + [[1, 0, 0, 0]] * 5 + [[1, 0, 1, 0]] * 4 + [[1, 0, 1, START]] + [[1, 0, 1, 0]] * 4
        self.compare({"frames": frames, "still": 1}, "unplugged and back")

    def test_held_off(self):
        # Not playing, then pauseFlag set: no pause; then the pause with the
        # still layer at 0 (it stays 0).
        frames = [[0, 0, 0, 0]] * 3 + [[1, 1, 0, 0]] * 3 + [[1, 0, 0, START]] * 2 + [[1, 0, 1, START]] + [[1, 0, 1, 0]] * 3
        self.compare({"frames": frames, "still": 0}, "held off")

    def test_random(self):
        rnd = random.Random(7)
        for seed in range(6):
            frames = []
            for _ in range(60):
                frames.append([int(rnd.random() < 0.9), int(rnd.random() < 0.1), int(rnd.random() < 0.6),
                               START if rnd.random() < 0.3 else 0])
            self.compare({"frames": frames, "still": rnd.randrange(3)}, f"random {seed}")


if __name__ == "__main__":
    unittest.main()
