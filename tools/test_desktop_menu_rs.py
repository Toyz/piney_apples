#!/usr/bin/env python3
"""crates/piney-desktop's menu task (dtmenu.rs: ccDtMenu / ccThDtMenu, the
START menu and its option menus) against the game's own task run in
tools/eemu.py, frame by frame.

The game's `ccThDtMenu` (0x0016ac00) runs natively with the ccDtMenu
constructor, OpenMenu, SystemMenu, the option menus (Controller, Vibrate,
Adjust Screen, Sound, Voiceover, Movie Text), ResetMenu, Disp,
ccMenuWindow's cells and cursor, ccScFade and CheckOperate; drawing,
threads, loads and the message window are hooked and logged, and a breath
ends a frame. The same pad scripts go through desktop_probe's `menu`
command. Compared each frame: the menu's state (menu, menuNext, status,
proccess, alpha), every packet of menuKanji, menuWindow, menuWindowA and
menuMask (positions to 0.01), menuFont's strings, the sounds, the
information lines opened and changed, closes, and the requests (display
offset, sound environment, vibration, camera scheme, the title's reset).
The Controller's frames compare its picture's labelled boxes too: the
cells of menuWindow and the labels of the setting kanji (set0-set3). The
title screen's menu (ccGame.status
1: OPTION opened by openReqNum 1, no reset) runs the same way.

Skipped when the disc is not extracted or cargo is missing.
"""
import json
import random
import shutil
import subprocess
import unittest

import functools
import os
import struct
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va  # noqa: E402
inf_va = functools.partial(va, overlay='desktop')  # this overlay's globals
from eemu import Machine, Stop, f_to_py  # noqa: E402
from image import Program  # noqa: E402
import test_save_init_rs  # noqa: E402

ELF = volume.ELF
P = Program(ELF, None)

CCSYS, SAVEDATA, EVENTMNG, GAME, DTMENU = inf_va(0x003788e0), inf_va(0x003789d8), inf_va(0x00378a94), inf_va(0x003789cc), inf_va(0x003789d0)
FONTTEX, FONTDEF, FONT, SCFADEDEF, CCMSG, MENUWIN = inf_va(0x00378960), inf_va(0x00378958), inf_va(0x00378954), inf_va(0x00378968), inf_va(0x00378a8c), inf_va(0x00378a9c)
SYS, SAVE, EV, GAMEOBJ, SCRATCH, HEAP = 0x01800000, 0x01810000, 0x01830000, 0x01831000, 0x01840000, 0x01900000
OK, CANCEL = 0x40, 0x20
UP, DOWN, START = 0x1000, 0x4000, 0x800


def sym(name):
    s = P.symbol_named(name)
    assert s is not None, name
    return s.value


class BaseEmu:
    def __init__(self, status=2):
        m = self.m = Machine(P)
        self.heap = HEAP
        self.log = []
        self.frame = 0
        self.script = {}
        self.max_frames = 0
        self.on_frame = None
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 0x25c, 4, SCRATCH)       # the scratch stack SystemMenu takes 192 bytes from
        m.store(SAVEDATA, 4, SAVE - 0x8000)
        # The port's fresh save, made by the game's own boot (ok cross,
        # cancel circle).
        test_save_init_rs.place_save(m, SAVE - 0x8000, test_save_init_rs.fresh_slot(ELF), ELF)
        m.store(EVENTMNG, 4, EV)
        m.store(EV + 0x770, 8, 0)
        m.store(EV + 0x778, 2, 0xffff)
        m.store(GAME, 4, GAMEOBJ)
        m.store(GAMEOBJ + 0, 4, status)
        m.store(GAMEOBJ + 0x4c, 4, 1)          # enableReset
        m.store(GAMEOBJ + 0x80, 4, self.alloc(0x40))
        for g in (FONTTEX, FONTDEF, SCFADEDEF):
            m.store(g, 4, self.alloc(0x100))
        self.sprites = {}
        h = m.hooks

        def hook(name, fn):
            h[sym(name)] = fn

        hook("__nw__FUi", lambda mm, n, *a: self.alloc(n))
        hook("__ct__16ccDrawPacketCtrlFv", lambda mm, a0, *a: a0)

        def layer_init(mm, this, pri, view, *a):
            mm.store(this + 4, 2, pri)
            mm.store(this + 0x2c, 4, self.alloc(0x100))
            self.ev("layer_init", pri=pri - 0x10000 if pri & 0x8000 else pri)
            return 0
        hook("Init__7ccLayerFsP6ccView", layer_init)
        hook("SetFrame__6ccViewFffffffff",
             lambda mm, *a: self.ev("set_frame", args=[round(f_to_py(mm.f[12 + i]), 6) for i in range(8)]) or 0)

        def set_tex(mm, this, ccs, chunk, *a):
            self.ev("set_tex", ccs=self.cstr(ccs), chunk=self.cstr(chunk))
            return 0
        hook("SetTex__8ccSpriteFPcPci", set_tex)

        def kanji_init(mm, this, l, n, *a):
            mm.store(this + 0x24, 4, n)
            mm.store(this + 0xd4, 2, 0)
            self.ev("kanji_init", obj=hex(this), l=l, m=n)
            return 0
        hook("Init__7ccKanjiFii", kanji_init)

        def noiz_ct(mm, this, *a):
            mm.store(this + 0x10, 4, self.alloc(0x40))
            return this
        hook("__ct__6ccNoizFv", noiz_ct)
        hook("__ct__9ccMessageFv", lambda mm, this, *a: this)
        hook("Breath__6ccTscbFi", lambda mm, tscb, n, *a: self.breath(n, "task"))
        hook("ccBreathThread__Fi", lambda mm, n, *a: self.breath(n, "breath_thread"))
        for name in ("ccSleepAllThread__Fv", "ccWakeAllThread__Fv", "fontOffFlip__Fv", "fontOnFlip__Fv",
                     "OffFlipExcept__7ccLayerFv", "OnFlipExcept__7ccLayerFv", "Draw__6ccNoizFv",
                     "Disp__9ccMessageFv", "Close__9ccMessageFv", "Trans__8ccSpriteFv"):
            hook(name, (lambda n: (lambda mm, *a: self.ev(n) or 0))(name.split("__")[0]))
        hook("ccSeOn__Fi", lambda mm, n, *a: self.ev("se", n=n) or 0)
        hook("ChangeRequest__6ccGameFii", lambda mm, this, n, sf, *a: self.ev("change_request", num=n, sf=sf) or 0)
        hook("GetFrameRate__8ccSystemFv", lambda mm, *a: 1)

        def open_info(mm, this, s0, s1, s2, *a):
            self.ev("open_info", lines=[self.cstr(x) if x else None for x in (s0, s1, s2)])
            return 0
        hook("OpenInfo__9ccMessageFPcPcPcPcii", open_info)

        def extract(mm, this, s, *a):
            self.ev("extract", obj=hex(this), text=self.cstr(s, 400))
            return 0
        hook("Extract__7ccKanjiFPc", extract)

        def make_packet(mm, this, code, t, *a):
            rd = lambda o: f_to_py(mm.load(this + o, 4))
            ri = lambda o: mm.load(this + o, 4, True)
            rec = dict(obj=hex(this), code=code, dx=rd(0x40), dy=rd(0x44), sx=rd(0x48), sy=rd(0x4c),
                       su=ri(0x50), sv=ri(0x54), wu=ri(0x58), wv=ri(0x5c), wi=ri(0x60),
                       rgba=[mm.load(this + 0x68 + 4 * i, 4) & 0xff for i in range(4)])
            self.ev("packet", **rec)
            mm.store(this + 0x40, 4, struct.unpack("<I", struct.pack("<f", rd(0x40) + rd(0x48)))[0])
            return 0
        hook("MakePacket__8ccSpriteFii", make_packet)
        hook("SendPacket__8ccSpriteFv", lambda mm, this, *a: self.ev("send", obj=hex(this)) or 0)

        def fade_send(mm, this, *a):
            for i in range(4):
                e = this + 4 + 0x24 * i
                st, cnt, tcnt = mm.load(e, 2), mm.load(e + 2, 2, True), mm.load(e + 4, 2, True)
                if st & 1:
                    c0, c1 = mm.load(e + 0x1c, 4), mm.load(e + 0x20, 4)
                    a0, a1 = c0 >> 24, c1 >> 24
                    alpha = a0 + (a1 - a0) * min(cnt, tcnt) // max(tcnt, 1)
                    self.ev("fade", el=i, cnt=cnt, tcnt=tcnt, alpha=alpha)
                    if cnt < tcnt:
                        mm.store(e + 2, 2, cnt + 1)
            return 0
        hook("SendPacket__8ccScFadeFv", fade_send)
        for name in ("ControllerMenu__8ccDtMenuFv", "VibrationMenu__8ccDtMenuFv", "ScreenMenu__8ccDtMenuFv",
                     "SoundMenu__8ccDtMenuFv", "VoiceMenu__8ccDtMenuFv", "StrwinMenu__8ccDtMenuFv",
                     "SaveSelMenu__8ccDtMenuFv", "SaveMenu__8ccDtMenuFv"):
            hook(name, (lambda n: (lambda mm, *a: self.ev("submenu", name=n) or 0))(name.split("__")[0]))
        self.tscb = self.alloc(0x60)

    def alloc(self, n):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        self.m.mem[a:a + n] = bytes(n)
        return a

    def cstr(self, a, n=200):
        b = bytes(self.m.mem[a:a + n])
        return b.split(b"\0")[0].decode("latin1")

    def ev(self, kind, **kw):
        self.log.append((self.frame, kind, kw))

    def dt(self):
        return self.m.load(DTMENU, 4)

    def state(self):
        d = self.dt()
        if not d:
            return None
        g = lambda o: self.m.load(d + o, 2, True)
        return dict(menu=g(0), next=g(2), status=g(4), req=g(6), still=g(8), proc=g(0xa), wait=g(0xc),
                    ext=g(0x10), fade=g(0x14), forbid=g(0x16), alpha=self.m.load(d + 0x1c, 4, True),
                    type=g(0) if g(0) == g(2) else 12)

    def breath(self, n, who):
        for _ in range(max(n, 1)):
            st = self.state()
            self.ev("frame_end", who=who, state=st)
            self.frame += 1
            if self.frame >= self.max_frames:
                raise Stop("done")
            push, rep = self.script.get(self.frame, (0, 0))
            self.m.store(SYS + 0x2d0, 4, push)
            self.m.store(SYS + 0x2d8, 4, rep)
            if self.on_frame:
                self.on_frame(self)
        return 0

    def run(self, frames, script, on_frame=None):
        self.max_frames = frames
        self.script = script
        self.on_frame = on_frame
        try:
            self.m.call(sym("ccThDtMenu__FPv"), [self.tscb], limit=200_000_000)
        except Stop as e:
            if str(e) != "done":
                raise
        return self.log




LEFT, RIGHT = 0x8000, 0x2000
SUBS = ("ControllerMenu__8ccDtMenuFv", "VibrationMenu__8ccDtMenuFv", "ScreenMenu__8ccDtMenuFv",
        "SoundMenu__8ccDtMenuFv", "VoiceMenu__8ccDtMenuFv", "StrwinMenu__8ccDtMenuFv")
SAVE_BASE = SAVE - 0x8000


def fbits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


class Emu(BaseEmu):
    def __init__(self, status=2, save=None, load_frames=3):
        super().__init__(status)
        m = self.m
        h = m.hooks
        for n in SUBS:
            del h[sym(n)]
        self.load_frames = load_frames
        self.threads = []
        for k, v in (save or {}).items():
            off, size = k
            m.store(SAVE_BASE + off, size, v & ((1 << (8 * size)) - 1))

        def hook(name, fn):
            h[sym(name)] = fn

        def start_thread(mm, fn, pri, stack, *a):
            t = self.alloc(0x60)
            self.threads.append([t, self.frame])
            self.ev("start_thread", fn=hex(fn), pri=pri, stack=stack)
            return t
        hook("ccStartThread__FPFPv_vii", start_thread)
        hook("ccDeleteThread__FP6ccTscb", lambda mm, t, *a: self.ev("delete_thread") or 0)
        hook("GetCCSAdrs__8ccStreamFPCc", lambda mm, s, *a: self.ev("get_ccs", name=self.cstr(s)) or 0x1234)
        hook("ccFileListDeleteOne__FP10ccFileList", lambda mm, fl, *a: self.ev("file_delete", fl=hex(fl)) or 0)
        hook("setCameraCtrlType__Fi", lambda mm, t, *a: self.ev("set_camera_ctrl_type", t=t) or 0)
        hook("SetActuater__5ccPadFiii", lambda mm, pad, a1, a2, a3, *a: self.ev(
            "set_actuater", pad=hex(pad - SYS), a=[a1, a2, a3], sw=mm.load(sym("actuaterSw__5ccPad"), 1)) or 0)

        def disp_off(mm, this, x, y, *a):
            sx = lambda v: v - (1 << 32) if v & 0x80000000 else v
            self.ev("display_offset", x=sx(x), y=sx(y))
            return 0
        hook("SetDisplayOffset__8ccSystemFii", disp_off)

        def snd_env(mm, sd, *a):
            g = lambda o: mm.load(sd + o, 2, True)
            self.ev("sound_env", main=g(0x841e), se=g(0x8420), bgm=g(0x8422), output=g(0x8424))
            return 0
        hook("SetSoundEnv__10ccSaveDataFv", snd_env)

        def change_info(name):
            def f(mm, this, s0, s1, s2, *a):
                s3 = mm.r[8] & 0xFFFFFFFF
                self.ev(name, lines=[self.cstr(x) if x else None for x in (s0, s1, s2, s3)])
                return 0
            return f
        hook("ChangeInfo__9ccMessageFPcPcPcPcii", change_info("change_info"))
        hook("OpenInfo__9ccMessageFPcPcPcPcii", change_info("open_info"))

        def msg_disp(mm, this, *a):
            self.ev("msg_disp", dx=f_to_py(mm.load(this + 0x184, 4)), dy=f_to_py(mm.load(this + 0x188, 4)))
            return 0
        hook("Disp__9ccMessageFv", msg_disp)

        def make_str(mm, this, s, mode, *a):
            rd = lambda o: f_to_py(mm.load(this + o, 4))
            ri = lambda o: mm.load(this + o, 4, True)
            b = bytes(mm.mem[s:s + 64])
            if mode:
                b = b.split(b"\xff")[0]
            else:
                b = b.split(b"\0")[0]
            rec = dict(obj=hex(this), text=b.decode("latin1"), mode=mode, dx=rd(0x40), dy=rd(0x44), sx=rd(0x48),
                       sy=rd(0x4c), su=ri(0x50), sv=ri(0x54), wu=ri(0x58), wv=ri(0x5c), wi=ri(0x60),
                       rgba=[mm.load(this + 0x68 + 4 * i, 4) & 0xff for i in range(4)])
            self.ev("str", **rec)
            mm.store(this + 0x40, 4, fbits(rd(0x40) + rd(0x48) * len(b)))
            return 0
        hook("MakePacketStr__8ccSpriteFPci", make_str)
        parent_mp = h[sym("MakePacket__8ccSpriteFii")]

        def mp(mm, this, code, t, *a):
            self.ctrl_now = mm.load(this + 4, 4)
            r = parent_mp(mm, this, code, t, *a)
            self.log[-1][2]["ctrl"] = self.ctrl_now
            return r
        hook("MakePacket__8ccSpriteFii", mp)

    def objnames(self):
        d = self.dt()
        L = lambda o: self.m.load(d + o, 4)
        n = {hex(L(0x24)): "win", hex(L(0x28)): "winA", hex(L(0x2c)): "kanji", hex(L(0x40)): "font",
             hex(L(0x48)): "mask"}
        for i in range(4):
            n[hex(L(0x30 + 4 * i))] = f"set{i}"
        return n

    def breath(self, n, who):
        r = super().breath(n, who)
        for t in self.threads:
            if t[1] is not None and self.frame - t[1] >= self.load_frames:
                self.m.store(t[0] + 0x14, 4, 0)
                t[1] = None
        return r

    def save16(self, off):
        return self.m.load(SAVE_BASE + off, 2, True)

    def save8(self, off):
        return self.m.load(SAVE_BASE + off, 1, True)




EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "desktop_probe")
ISO = volume.ISO
FRESH = {(0x841e, 2): 256, (0x8420, 2): 256, (0x8422, 2): 256, (0x8424, 2): 1, (0x8428, 1): 1, (0x842c, 1): 1,
         (0x8430, 1): 1}


def game(script, frames, title_open=None):
    """With `title_open`, the title screen's menu: ccGame.status 1, no
    reset, and openReqNum 1 (PlayOption's request) at that frame."""
    e = Emu(status=1 if title_open else 2, save=FRESH, load_frames=1)
    on_frame = None
    if title_open:
        e.m.store(GAMEOBJ + 0x4c, 4, 0)

        def on_frame(emu):
            if emu.frame == title_open:
                emu.m.store(emu.dt() + 6, 2, 1)
    log = e.run(frames, script, on_frame)
    names = e.objnames()
    out = {}
    for f, k, kw in log:
        o = out.setdefault(f, {"pk": [], "ev": [], "st": None})
        if k == "packet":
            n = names.get(kw["obj"], kw["obj"])
            if n in ("win", "winA", "kanji", "mask", "set0", "set1", "set2", "set3"):
                o["pk"].append((n, kw["code"], kw["dx"], kw["dy"], kw["sx"], kw["sy"], list(kw["rgba"])))
        elif k == "str":
            o["ev"].append(("str", kw["text"], kw["dx"], kw["dy"]))
        elif k == "se":
            o["ev"].append(("se", kw["n"]))
        elif k in ("open_info", "change_info"):
            o["ev"].append((k, [x for x in kw["lines"] if x is not None]))
        elif k == "Close":
            o["ev"].append(("close",))
        elif k == "display_offset":
            o["ev"].append(("display_offset", kw["x"], kw["y"]))
        elif k == "sound_env":
            o["ev"].append(("sound_env", kw["main"], kw["se"], kw["bgm"], kw["output"]))
        elif k == "set_actuater":
            o["ev"].append(("vibration", kw["sw"]))
        elif k == "set_camera_ctrl_type":
            o["ev"].append(("camera", kw["t"]))
        elif k == "change_request":
            o["ev"].append(("change_request", kw["num"], kw["sf"]))
        elif k == "frame_end":
            s = kw["state"]
            o["st"] = [s["menu"], s["next"], s["status"], s["proc"], s["alpha"]]
    return out


def port(script, frames, title_open=None):
    head = f"menu {frames}" + (f" {title_open}" if title_open else "")
    lines = [head] + [f"pad {f} {p} {r}" for f, (p, r) in sorted(script.items())] + ["end"]
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    out = {}
    for line in p.stdout.splitlines():
        j = json.loads(line)
        o = {"pk": [], "ev": [], "st": j["st"][:5]}
        for e in j["ev"]:
            if e[0] == "pk":
                o["pk"].append((e[1], e[2], e[3], e[4], e[5], e[6], e[7]))
            elif e[0] == "str":
                o["ev"].append(("str", e[1], e[2], e[3]))
            elif e[0] in ("open_info", "change_info"):
                o["ev"].append((e[0], [x for x in e[1] if x is not None]))
            elif e[0] == "close":
                o["ev"].append(("close",))
        for r in j["req"]:
            if r[0] == "vibration":
                if r[1]:
                    o["ev"].append(("vibration", 1))
            else:
                o["ev"].append(tuple(r))
        out[j["f"]] = o
    return out


def same_pk(a, b):
    return (a[0] == b[0] and a[1] == b[1] and a[6] == b[6]
            and all(abs(x - y) < 0.01 for x, y in zip(a[2:6], b[2:6])))


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class MenuAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-desktop", "--features",
                        "trace", "--example", "desktop_probe"], cwd=ROOT, check=True)

    def compare(self, script, frames, what, title_open=None):
        g, p = game(script, frames, title_open), port(script, frames, title_open)
        for f in range(1, frames - 1):
            a, b = g.get(f), p.get(f)
            self.assertIsNotNone(a, f"{what}: no game frame {f}")
            self.assertEqual(a["st"], b["st"], f"{what} frame {f}: state")
            key = lambda e: (e[0], json.dumps(e[1:]))
            self.assertEqual(sorted(a["ev"], key=key), sorted(b["ev"], key=key), f"{what} frame {f}: events")
            for obj in ("win", "winA", "kanji", "mask", "set0", "set1", "set2", "set3"):
                ga = [x for x in a["pk"] if x[0] == obj]
                pa = [x for x in b["pk"] if x[0] == obj]
                self.assertEqual(len(ga), len(pa), f"{what} frame {f}: {obj} packets {ga} vs {pa}")
                for x, y in zip(ga, pa):
                    self.assertTrue(same_pk(x, y), f"{what} frame {f}: {obj} {x} vs {y}")

    def test_option_list(self):
        rng = random.Random(5)
        script = {2: (START, 0)}
        f = 8
        while f < 100:
            f += rng.choice([1, 2, 3, 5, 9, 17])
            script[f] = (0, rng.choice([UP, DOWN]))
        script[110] = (CANCEL, 0)
        self.compare(script, 140, "OPTION")

    def test_each_option(self):
        # START, down to item k, OK; then moves and OK / cancel inside.
        for k, inner in ((1, [(0, DOWN), (OK, 0), (0, UP), (OK, 0)]),          # Vibrate
                         (4, [(0, DOWN), (OK, 0), (OK, 0)]),                    # Voiceover
                         (5, [(0, UP), (OK, 0), (0, DOWN), (OK, 0)]),            # Movie Text
                         (2, [(0, RIGHT), (0, RIGHT), (0, DOWN), (0, LEFT | DOWN)]),  # Adjust Screen
                         (3, [(0, LEFT), (0, LEFT), (0, DOWN), (0, RIGHT), (0, DOWN), (0, DOWN), (0, LEFT)]),
                         (0, [(0, DOWN), (OK, 0), (0, DOWN), (OK, 0)])):         # Controller
            script = {2: (START, 0)}
            f = 16
            for _ in range(k):
                script[f] = (0, DOWN)
                f += 3
            script[f] = (OK, 0)
            f += 30
            for push, rep in inner:
                script[f] = (push, rep)
                f += 4
            script[f + 10] = (CANCEL, 0)
            self.compare(script, f + 50, f"option {k}")

    def test_title_option_list(self):
        # The title's OPTION (menu 1, ccGame.status 1): opened by
        # PlayOption's openReqNum, moved through, closed with cancel.
        rng = random.Random(6)
        script = {}
        f = 10
        while f < 90:
            f += rng.choice([1, 2, 3, 5, 9, 17])
            script[f] = (0, rng.choice([UP, DOWN]))
        script[100] = (CANCEL, 0)
        self.compare(script, 130, "title OPTION", title_open=4)

    def test_title_each_option(self):
        # The title's OPTION list (list 1): Controller, Vibrate, Adjust
        # Screen, Sound, Voiceover, Movie Text; down to item k, OK, then
        # moves and OK / cancel inside, then cancel twice.
        for k, inner in ((0, [(0, DOWN), (OK, 0), (0, UP), (OK, 0)]),
                         (1, [(0, DOWN), (OK, 0), (OK, 0)]),
                         (2, [(0, UP), (OK, 0), (0, DOWN), (OK, 0)]),
                         (3, [(0, RIGHT), (0, RIGHT), (0, DOWN), (0, LEFT | DOWN)]),
                         (4, [(0, LEFT), (0, LEFT), (0, DOWN), (0, RIGHT), (0, DOWN), (0, DOWN), (0, LEFT)]),
                         (5, [(0, DOWN), (OK, 0), (0, DOWN), (OK, 0)])):
            script = {}
            f = 16
            for _ in range(k):
                script[f] = (0, DOWN)
                f += 3
            script[f] = (OK, 0)
            f += 30
            for push, rep in inner:
                script[f] = (push, rep)
                f += 4
            script[f + 10] = (CANCEL, 0)
            script[f + 40] = (CANCEL, 0)
            self.compare(script, f + 70, f"title option {k}", title_open=4)

    def test_title_screen(self):
        script = {2: (START, 0), 16: (0, UP), 19: (OK, 0), 50: (0, UP), 55: (OK, 0), 90: (0, UP), 95: (OK, 0)}
        self.compare(script, 110, "Title Screen")


if __name__ == "__main__":
    unittest.main()
