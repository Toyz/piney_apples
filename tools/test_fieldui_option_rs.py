#!/usr/bin/env python3
"""The OPTION pages of crates/piney-fieldui (menus 13-20: Controller,
Vibrate, Adjust Screen, Sound, Title Screen, Data Drain, Voiceover, Movie
Text; crates/piney-fieldui/src/menus/option.rs) against the game's own
ccThMenu in tools/eemu.py, frame by frame, with tools/test_fieldui_rs.py's
harness.

On top of the base harness's hooks, what the pages ask of the rest of the
game is logged on the game's side as events the probe prints from the
port's requests and trace:

  start_thread, delete_thread   ccStartThread(ccThControllerMenu, 34, 2048),
                                ccDeleteThread; the task's flag clears at
                                the breath after its start (a load of one
                                frame, as the port has the picture already)
  get_ccs, set_tex, file_delete ccStream::GetCCSAdrs("xcontrol"),
                                menuMask->SetTex, ccFileListDeleteOne
  camera                        setCameraCtrlType(t)       (Request::CameraType)
  display_offset                ccSystem::SetDisplayOffset (Request::DisplayOffset)
  sound_env                     ccSaveData::SetSoundEnv    (Request::SoundEnv)
  actuater, actuater_sw         ccPad::SetActuater, and ccPad::actuaterSw when
                                it changes                 (Request::Vibration)
  change_mode                   ccGame::ChangeRequest      (Request::ChangeMode)

Every packet's ctrl bits 0x20, 0x40 and 0x10 (mirrored, upside down,
shadowed) are compared, and the options' part of the save (+0x841a -
+0x8431) after every frame.

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_fieldui_rs as base  # noqa: E402
from test_fieldui_rs import CANCEL, DOWN, LEFT, OK, RIGHT, UP, Scenario, sym  # noqa: E402

ACTUATER_SW = sym("actuaterSw__5ccPad")
# The save's options: screenX/Y, the volumes and output, drainDemo,
# vibration, camType, voice, strWinMode.
OPTIONS = (0x841A, 0x18)
SCREEN_X, SCREEN_Y, MAIN_VOL, SE_VOL, BGM_VOL, OUTPUT = 0x841A, 0x841C, 0x841E, 0x8420, 0x8422, 0x8424
DRAIN_DEMO, VIBRATION, CAM_TYPE, VOICE, STR_WIN = 0x8427, 0x8428, 0x8429, 0x842C, 0x8430
# OPTION's rows: Controller, Vibrate, Adjust Screen, Sound, Data Drain,
# Voiceover, Movie Text, Title Screen.
ROWS = {"controller": 0, "vibration": 1, "screen": 2, "sound": 3, "drain": 4, "voice": 5, "strwin": 6, "reset": 7}


def sx(v):
    return v - (1 << 32) if v & 0x80000000 else v


class OptionGame(base.Game):
    """The base harness's game with the OPTION pages' externals logged."""

    def __init__(self, sc):
        self.threads = []
        self.sw_last = 2
        super().__init__(sc)

    def setup(self):
        super().setup()
        # actuaterSw 2: any write the menu makes shows as a change.
        self.m.store(ACTUATER_SW, 1, 2)

    def cstr(self, a):
        return bytes(self.m.mem[a:a + 64]).split(b"\0")[0].decode("latin1")

    def hooks(self):
        super().hooks()
        m = self.m
        h = m.hooks

        def hook(name, fn):
            h[sym(name)] = fn

        def start_thread(mm, fn, pri, stack, *a):
            t = self.alloc(0x60)
            self.threads.append(t)
            self.ev("start_thread", pri, stack)
            return t
        hook("ccStartThread__FPFPv_vii", start_thread)
        hook("ccDeleteThread__FP6ccTscb", lambda mm, t, *a: self.ev("delete_thread") or 0)
        hook("GetCCSAdrs__8ccStreamFPCc", lambda mm, s, *a: self.ev("get_ccs", self.cstr(s)) or 0x1234)
        hook("ccFileListDeleteOne__FP10ccFileList", lambda mm, fl, *a: self.ev("file_delete") or 0)

        def set_tex(mm, this, f, t, n, *a):
            c = self.menu()
            if c and this == mm.load(c + 0xE4, 4):
                self.ev("set_tex", "mask", self.cstr(f), self.cstr(t))
            return 0
        hook("SetTex__8ccSpriteFPcPci", set_tex)
        hook("setCameraCtrlType__Fi", lambda mm, t, *a: self.ev("camera", sx(t)) or 0)
        hook("SetDisplayOffset__8ccSystemFii", lambda mm, this, x, y, *a: self.ev("display_offset", sx(x), sx(y)) or 0)

        def sound_env(mm, sd, *a):
            g = lambda o: mm.load(sd + o, 2, True)  # noqa: E731
            self.ev("sound_env", g(MAIN_VOL), g(BGM_VOL), g(SE_VOL), g(OUTPUT))
            return 0
        hook("SetSoundEnv__10ccSaveDataFv", sound_env)
        hook("SetActuater__5ccPadFiii", lambda mm, pad, a1, a2, a3, *a: self.ev("actuater", a1, a2, a3) or 0)
        hook("ChangeRequest__6ccGameFii", lambda mm, this, n, f, *a: self.ev("change_mode", sx(n), sx(f)) or 0)


    def breath(self, n):
        # The loading task runs after the menu task's breath and loads at
        # once; actuaterSw is looked at as the frame ends.
        for t in self.threads:
            self.m.store(t + 0x14, 4, 0)
        self.threads = []
        v = self.m.load(ACTUATER_SW, 1)
        if v != self.sw_last:
            self.sw_last = v
            self.ev("actuater_sw", v)
        return super().breath(n)


def options(sc, screen=(0, 0), vol=(256, 256, 256), output=1, drain=1, vibration=1, cam=0, voice=1, strwin=1):
    """The save's options, the same on both sides (volumes main, BGM, SE)."""
    sc.saves += [(SCREEN_X, 2, screen[0] & 0xFFFF), (SCREEN_Y, 2, screen[1] & 0xFFFF),
                 (MAIN_VOL, 2, vol[0] & 0xFFFF), (BGM_VOL, 2, vol[1] & 0xFFFF), (SE_VOL, 2, vol[2] & 0xFFFF),
                 (OUTPUT, 2, output & 0xFFFF), (DRAIN_DEMO, 1, drain & 0xFF), (VIBRATION, 1, vibration & 0xFF),
                 (CAM_TYPE, 1, cam & 0xFF), (VOICE, 1, voice & 0xFF), (STR_WIN, 1, strwin & 0xFF)]
    sc.watch.append(OPTIONS)


def enter(sc, row, f=10):
    """OPTION opened at frame f and its cursor moved down to `row`, OK at
    the end: the frame after the OK."""
    sc.opens[f] = 12
    f += 12
    for _ in range(row):
        sc.pads[f] = (0, DOWN)
        f += 3
    f += 3
    sc.pads[f] = (OK, 0)
    return f + 1


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class OptionPagesAgainstGame(unittest.TestCase):
    game_class = OptionGame
    compare = base.FieldUiAgainstGame.compare
    party = base.FieldUiAgainstGame.party

    @classmethod
    def setUpClass(cls):
        base.FieldUiAgainstGame.setUpClass()

    def scenario(self, frames, n=1, **opts):
        sc = Scenario(frames)
        self.party(sc, n)
        options(sc, **opts)
        return sc

    def test_controller(self):
        for cam in (0, 2):
            sc = self.scenario(260, 2, cam=cam)
            f = enter(sc, ROWS["controller"])
            # The picture loads; every scheme in turn, then the one before.
            f += 20
            for pads in ((0, DOWN), (OK, 0), (0, DOWN), (OK, 0), (0, DOWN), (OK, 0), (0, DOWN), (OK, 0),
                         (0, UP), (0, UP), (OK, 0)):
                sc.pads[f] = pads
                f += 9
            sc.pads[f] = (CANCEL, 0)
            sc.pads[f + 40] = (0, DOWN)
            sc.pads[f + 50] = (CANCEL, 0)
            self.compare(sc, f"controller {cam}")

    def test_controller_cancel_at_once(self):
        # Cancel the frame the list is up, and a pad pushed while it loads.
        sc = self.scenario(120, 1, cam=3)
        f = enter(sc, ROWS["controller"])
        for k in range(0, 14):
            sc.pads[f + k] = (0, DOWN) if k % 2 else (0, 0)
        sc.pads[f + 14] = (CANCEL, 0)
        sc.pads[f + 60] = (CANCEL, 0)
        self.compare(sc, "controller cancel")

    def test_controller_open_during_breaths(self):
        # An event's openReqNum while the page breathes inside its handler
        # (the load's wait, the close's two breaths): the task's loop only
        # sees it after the page is done.
        for opens in ((6,), (20, 21)):
            sc = self.scenario(110, 1, cam=1)
            f = enter(sc, ROWS["controller"])
            sc.pads[f + 14] = (CANCEL, 0)
            for k in opens:
                sc.opens[f + k] = 12
            sc.pads[f + 60] = (CANCEL, 0)
            self.compare(sc, f"controller open {opens}")

    def on_off(self, what, **opts):
        sc = self.scenario(160, 2, **opts)
        f = enter(sc, ROWS[what])
        f += 12
        for pads in ((0, DOWN), (OK, 0), (0, UP), (OK, 0), (OK, 0), (0, DOWN), (0, DOWN), (OK, 0), (0, UP)):
            sc.pads[f] = pads
            f += 8
        sc.pads[f] = (CANCEL, 0)
        sc.pads[f + 35] = (CANCEL, 0)
        self.compare(sc, f"{what} {opts}")

    def test_vibration(self):
        self.on_off("vibration", vibration=1)
        self.on_off("vibration", vibration=0)

    def test_drain_demo(self):
        self.on_off("drain", drain=1)
        self.on_off("drain", drain=0)

    def test_voice(self):
        self.on_off("voice", voice=1)
        self.on_off("voice", voice=0)
        self.on_off("voice", voice=2)

    def test_strwin(self):
        self.on_off("strwin", strwin=1)
        self.on_off("strwin", strwin=0)

    def test_screen(self):
        sc = self.scenario(330, 1, screen=(-3, 2))
        f = enter(sc, ROWS["screen"])
        f += 8
        # Right to the edge and past it, left all the way and past, down,
        # up, the diagonals.
        for rep, n in ((RIGHT, 20), (LEFT, 36), (DOWN, 20), (UP, 36), (RIGHT | DOWN, 10), (LEFT | UP, 3),
                       (UP | RIGHT, 4), (DOWN | LEFT, 2)):
            for _ in range(n):
                sc.pads[f] = (0, rep)
                f += 1
            f += 3
        sc.pads[f] = (OK, 0)
        sc.pads[f + 3] = (CANCEL, 0)
        sc.pads[f + 40] = (CANCEL, 0)
        self.compare(sc, "screen")

    def test_sound(self):
        sc = self.scenario(330, 1, vol=(256, 200, 100), output=1)
        f = enter(sc, ROWS["sound"])
        f += 10
        seq = [(0, LEFT)] * 3 + [(0, RIGHT)] * 6 + [(0, DOWN)] + [(0, LEFT)] * 28 + [(0, RIGHT)] * 2 + \
              [(0, DOWN)] + [(0, RIGHT)] * 10 + [(0, DOWN)] + [(0, LEFT), (0, LEFT), (0, RIGHT), (0, RIGHT),
                                                               (0, LEFT)] + [(0, DOWN), (0, UP), (0, UP)] + \
              [(0, LEFT)] * 2 + [(0, UP)] * 2
        for pads in seq:
            sc.pads[f] = pads
            f += 2
        sc.pads[f] = (CANCEL, LEFT)
        sc.pads[f + 40] = (CANCEL, 0)
        self.compare(sc, "sound")

    def test_title_screen(self):
        sc = self.scenario(300, 2)
        sc.opens[10] = 12
        # Up to Title Screen: OK on Cancel, back.
        sc.pads.update({25: (0, UP), 32: (OK, 0), 50: (OK, 0)})
        # Again: OK, then Cancel on the second question.
        sc.pads.update({80: (OK, 0), 95: (0, UP), 100: (OK, 0), 130: (CANCEL, 0)})
        # Again: OK twice, the game leaves the field.
        sc.pads.update({160: (OK, 0), 175: (0, DOWN), 180: (OK, 0), 210: (0, DOWN), 215: (OK, 0),
                        230: (CANCEL, 0), 240: (OK, 0)})
        self.compare(sc, "title screen")

    def test_title_screen_cancel(self):
        sc = self.scenario(120, 1)
        sc.opens[10] = 12
        sc.pads.update({25: (0, UP), 32: (OK, 0), 50: (CANCEL, 0), 80: (CANCEL, 0)})
        self.compare(sc, "title screen cancel")

    def test_random_option_pages(self):
        # Random pads through OPTION and its pages.
        for seed in range(1, 9):
            rnd = random.Random(seed)
            sc = self.scenario(500, rnd.choice((1, 2, 3)), screen=(rnd.randrange(-48, 49, 3), rnd.randrange(-16, 17)),
                               vol=tuple(rnd.choice((0, 8, 100, 256)) for _ in range(3)), output=rnd.choice((0, 1)),
                               drain=rnd.choice((0, 1)), vibration=rnd.choice((0, 1)), cam=rnd.randrange(4),
                               voice=rnd.choice((0, 1)), strwin=rnd.choice((0, 1)))
            for f in range(10, 500, 110):
                sc.opens[f] = 12
            for f in range(12, 500):
                r = rnd.random()
                if r < 0.25:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, DOWN, LEFT, RIGHT, LEFT | DOWN)))
                elif r < 0.33:
                    sc.pads[f] = (OK, 0)
                elif r < 0.36:
                    sc.pads[f] = (CANCEL, 0)
            self.compare(sc, f"random option {seed}")


if __name__ == "__main__":
    unittest.main()
