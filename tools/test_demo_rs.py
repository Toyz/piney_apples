#!/usr/bin/env python3
"""crates/piney-demo against the game's own code run in tools/eemu.py.

The demo_probe example (built with cargo, feature `trace`) runs the port's
pieces of the title screen on states this sends it; the same states go
through DEMO.PRG's functions in eemu, with the calls they make into the main
executable hooked and recorded (ccAnm::SetAnm, _AnimateForward,
_AnimateFrame and Draw, GetSubstAdrsF, the matrix setters, ccSeOn,
ccDtMenu::CheckMenuType, DataLoad_Control::Main_Control):

  - ccOpening_Control::MoveCurNut (0x004070c0) over random pad sequences
    (push, release, repeat of up and down), with SwitchCur run natively;
  - SwitchCur, PlayNeutral, ChangeMainAct, MOVEcount (its fade entered by
    the game's own ccScFade::EntryFlash3);
  - ccScFade::SendPacket (0x0015fb80) itself: the colours it packs and its
    counters, frame by frame, after EntryFlash, EntryFlash3 and EntryFade;
  - AllTransparency (bit-exact floats and every animation's localtp),
    AllAnimate (the steps, the dummies, the icons' turn bit-exact), AllDraw;
  - SetNeutral, SetNewGame, SetParodyGame, SetDataLoad, SetOption;
  - PlayDataLoad, PlayOption, PlayNewGame, PlayParodyGame;
  - Main (0x004043b0) over every action, mask and pad push;
  - LogoMain (0x00404540): the movies, the 60 frames of wait, the music;
  - PlayBootMemCard with BootMem_Control, ccSaveSys::BootCheckReq and
    NextProccess and Data_Control::Main run natively, frame by frame, the
    saveSys result scripted;
  - the addresses of the memory-card question's text;
  - ccSaveData::NewGame (0x00174d70), (1) then (0), with and without the
    parody flag, from ccSaveData::Init(1) and from random bytes: the whole
    0x8530-byte save and the names copied outside it;
  - ccThDemo (0x00400900) itself with PlayBootMemCard and Main scripted and
    LogoMain and PlayOpeningStream run natively: every breath, movie, sound
    request, save call and the ChangeRequest, in order;
  - the load screen: DataLoad_Control::Main_Control (0x004037f0) with every
    Data_Control step, ccSaveSys (constructor, StartReq(1), MainProccess
    and its load, NextProccess) and the save it loads all run natively,
    frame by frame over scripted pads, ccMcard answered from a card kept as
    files (the port reads the same files through
    piney_desktop::card::FilesCard): the control's and saveSys's state, the
    texts drawn (ccKanji::Disp: which kanji, where, colour, bytes), the
    cursor and button packets (ccSprite::MakePacket), the sounds, and the
    whole save at the end;
  - ccSaveData::LoadGame (0x00175110) on random saves: the whole save and
    the names copied to ccsNameList;
  - the icons' light: the port's whole title run to the menu, and for each
    lit object drawn (at two frames) its lwMatrix goes through the game's
    light (ccLight/ccOmniLight::Init, ccSetColor, the matrix, AddGrp as
    Init makes it), ccDrawEnv::SetLightMatrix and ccSetMatrixPacket (eemu
    with VU0 macro mode, tools/test_anim.py), then VU1's mc_SetMatrix,
    mc_SetObjParam and mc02_Start00/01 + mc_DrawTriL (tools/vu.py Vu) over
    the model's vertices: the light registers against the port's
    piney_draw::Lights, and every vertex colour VU1 outputs against the one
    piney_gs::convert::mmat computes from the port's ModelDraw.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import functools
import os
import random
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va  # noqa: E402
inf_va = functools.partial(va, overlay='demo')  # this overlay's globals
import test_save_init_rs  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "demo_probe")
TITLE1 = volume.DATA + "::title1"

# gp globals (INF SLUS_202.67 .sdata).
CCSYS, SAVEDATA, DTMENU, SCFADEDEF, SAVESYS, GAME = (inf_va(0x003788e0), inf_va(0x003789d8), inf_va(0x003789d0), inf_va(0x00378968), inf_va(0x003789d4),
                                                     inf_va(0x003789cc))
RESETOPFLG, FONTLAYER = inf_va(0x003789c0), inf_va(0x00378950)
# Scratch memory.
SYS, SAVE, DTM, FADE, SSYS, THIS, ANMS, STRS = (0x01800000, 0x01810000, 0x01830000, 0x01831000, 0x01832000,
                                                 0x01840000, 0x01850000, 0x01870000)
COORD, BOOTMEM, DATALOAD, WORK, TSCB, STREAM, LAYER = (0x01880000, 0x01881000, 0x01882000, 0x01890000, 0x018a0000,
                                                       0x018a1000, 0x018a2000)
OK, CANCEL, UP, DOWN = 0x40, 0x20, 0x1000, 0x4000
ONE = 0x3f800000

# ccOpening_Control (DWARF), and the functions under test (demo.prg).
O = dict(next=0x1fc, nextsw=0x21c, mask=0x4, nowvol=0x8, pushflg=0xa, pushcnt=0xc, repeat=0xe, newgamesw=0x10, demo=0x14, flash=0x18,
         alpha=0x1c, tr=0x20, trico=0x24, trwin=0x28, rot=0x190, camera=0x1c0, a_ico=0x1c4, a_ico_d=0x1c8,
         a_app=0x1cc, a_app_d=0x1d0, a_win=0x1d4, a_back=0x1d8, a_boot=0x1dc, cur=0x1e0, logo=0x1e4, main=0x1e8,
         nut=0x1f0, dat=0x1f4, opt=0x1f8, boot=0x200, err=0x204, paro=0x20c, nutlock=0x20d, curno=0x210,
         nesw=0x214, datsw=0x218, loadsw=0x220, optsw=0x224, wait=0x228, maxcur=0x22c, flashflg=0x230,
         start=0x234, dataload=0x238, bootmem=0x23c)
FN = dict(main=inf_va(0x004043b0), logo=inf_va(0x00404540), movecount=inf_va(0x00404710), animate=inf_va(0x004047e0), transp=inf_va(0x00405420),
          draw=inf_va(0x004059d0), change=inf_va(0x00405d20), stream=inf_va(0x004066b0), bootcard=inf_va(0x00406950), neutral=inf_va(0x00406b50),
          playnew=inf_va(0x00406bf0), playparo=inf_va(0x00406c20), playload=inf_va(0x00406c60), playopt=inf_va(0x00406de0),
          movecur=inf_va(0x004070c0), switch=inf_va(0x004072a0), setboot=inf_va(0x004076f0), setn=inf_va(0x00407950), setnew=inf_va(0x00408240),
          setload=inf_va(0x004082c0), setopt=inf_va(0x00408ba0), setparo=inf_va(0x00409d40), thdemo=inf_va(0x00400900),
          setnext=inf_va(0x00409470), playnext=inf_va(0x00406f40))
# The sqc2 $vf0 stores the interpreter does not run; their vectors only
# feed hooked matrix setters.
SQC2 = (inf_va(0x00407978), inf_va(0x00407714), inf_va(0x0040771c), inf_va(0x00407724))
ANM = 0x110
GROUPS = (("ico", "a_ico", 5), ("icod", "a_ico_d", 5), ("app", "a_app", 5), ("appd", "a_app_d", 5),
          ("win", "a_win", 1), ("back", "a_back", 4))


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-demo", "--features", "trace",
                    "--example", "demo_probe"], cwd=ROOT, check=True)


# The volume the tests under `volumes` run as: the control's m_NowVol in
# eemu (Infection's overlay holds every volume's branches) and the probe's
# `vol N` (its title<N>).
VOL = [1]


def ask(lines):
    lines = [f"vol {VOL[0]}"] + list(lines)
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


def volumes(test):
    """Run a test once for each volume, 1-4."""
    def run(self):
        try:
            for v in (1, 2, 3, 4):
                VOL[0] = v
                with self.subTest(volume=v):
                    test(self)
        finally:
            VOL[0] = 1
    run.__name__ = test.__name__
    run.__doc__ = test.__doc__
    return run


class Stop(Exception):
    pass


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DemoAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        build()
        cls.p = Program(ELF, "demo")

    # the machine ----------------------------------------------------------------
    def machine(self, menu_type=-1, load=0):
        from eemu import Machine, _cstr
        m = Machine(self.p)
        for a in SQC2:
            m.store(a, 4, 0)
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, WORK)
        m.store(SAVEDATA, 4, SAVE)
        # The port's fresh save, made by the game's own boot (ok cross,
        # cancel circle), the main volume the scenarios want.
        m.mem[SAVE:SAVE + 0x8530] = test_save_init_rs.fresh_save(ELF)
        m.store(SAVE + 0x841e, 2, 0x3fff)
        m.store(DTMENU, 4, DTM)
        m.store(SCFADEDEF, 4, FADE)
        m.store(SAVESYS, 4, SSYS)
        m.store(FONTLAYER, 4, LAYER)
        self.labels = {}
        at = ANMS
        for name, member, n in GROUPS:
            m.store(THIS + O[member], 4, at)
            for i in range(n):
                label = name if n == 1 else f"{name}{i}"
                self.labels[at] = label
                m.store(at + 0x9c, 2, 256)
                m.store(at + 0x88, 4, ONE)
                at += ANM
        self.labels[at] = "camera"
        m.store(THIS + O["camera"], 4, at)
        self.boot_anm = at + ANM
        self.labels[self.boot_anm] = "boot"
        m.store(THIS + O["nowvol"], 2, VOL[0])
        m.store(THIS + O["alpha"], 4, 0x3d4ccccd)
        for k in ("tr", "trico", "trwin"):
            m.store(THIS + O[k], 4, ONE)
        m.store(THIS + O["repeat"], 2, 30)
        m.store(THIS + O["maxcur"], 4, 3)
        m.store(THIS + O["curno"], 4, 1)
        m.store(THIS + O["nutlock"], 1, 1)
        m.store(THIS + O["dataload"], 4, DATALOAD)
        m.store(THIS + O["bootmem"], 4, BOOTMEM)
        self.calls, self.se, self.fwd = [], [], {}
        self.menu_type, self.load = menu_type, load
        self.events = []

        def label(a):
            return self.labels.get(a, hex(a))

        def set_anm(mm, a0, a1, a2, a3):
            name = _cstr(mm, a2).decode() if a2 else ""
            self.calls.append(["set", label(a0), name])
            mm.store(a0 + 0xac, 4, 1 if name else 0)
            return 0

        def forward(mm, a0, a1, *a):
            self.calls.append(["fwd", label(a0), str(a1)])
            return int(self.fwd.get(label(a0), False))

        def frame(mm, a0, a1, *a):
            if mm.load(a0 + 0xac, 4):
                self.calls.append(["fwd", label(a0), str(a1)])
            return 0

        def draw(mm, a0, *a):
            self.calls.append(["draw", label(a0), "None"])
            return 0

        def subst(mm, a0, a1, *a):
            self.calls.append(["subst", label(a0), _cstr(mm, a1).decode()])
            return COORD

        def trans(mm, a0, *a):
            if a0 - 64 in self.labels:
                self.calls.append(["place", label(a0 - 64), "0"])
            return 0

        def pos_rot(mm, a0, a1, a2, *a):
            self.calls.append(["place", label(a0), str(mm.load(a2 + 4, 4))])
            return 0

        def new(mm, a0, *a):
            return self.boot_anm

        def ctor(mm, a0, *a):
            mm.store(a0 + 0x9c, 2, 256)
            mm.store(a0 + 0x88, 4, ONE)
            mm.store(a0 + 0xac, 4, 0)
            return a0

        sym = self.p.symbol_named
        nop = lambda mm, *a: 0          # noqa: E731
        m.hooks.update({
            inf_va(0x00151c60): set_anm, inf_va(0x00152270): forward, inf_va(0x00152400): frame, inf_va(0x001524d0): draw,
            inf_va(0x00151da0): subst, inf_va(0x001108e8): trans, inf_va(0x00138240): pos_rot, inf_va(0x00138050): nop, inf_va(0x00138380): nop,
            inf_va(0x00110990): nop, inf_va(0x00110928): nop, inf_va(0x00110918): nop, inf_va(0x00100c90): new, inf_va(0x0014fc80): ctor,
            inf_va(0x0014fcf0): nop,
            sym("ccSeOn__Fi").value: lambda mm, a0, *a: self.se.append(a0) or 0,
            sym("CheckMenuType__8ccDtMenuFv").value: lambda mm, *a: self.menu_type & 0xffffffff,
            sym("Main_Control__16DataLoad_ControlFv").value: lambda mm, *a: self.load & 0xffffffff,
            sym("Main_Control__20NextDataLoad_ControlFv").value: lambda mm, *a: self.load & 0xffffffff,
            sym("YesNoDialogue__12Data_ControlFib").value: nop,
            sym("InfoMessage__12Data_ControlFib").value:
                lambda mm, a0, a1, *a: self.events.append(a1) or 0,
        })
        return m

    def neutral(self, m, paro=0):
        """SetNeutral once, as the probe does before every request."""
        m.store(THIS + O["paro"], 1, paro)
        m.call(FN["setn"], [THIS])
        self.calls, self.se = [], []

    def tps(self, m):
        out = []
        for member, n in (("a_ico", 5), ("a_app", 5), ("a_win", 1)):
            base = m.load(THIS + O[member], 4)
            out += [m.load(base + ANM * i + 0x88, 4) for i in range(n)]
        return out

    def fade_state(self, m):
        out = []
        for i in range(4):
            e = FADE + 4 + 36 * i
            out.append([m.load(e, 2, True), m.load(e + 2, 2, True), m.load(e + 4, 2, True), m.load(e + 6, 2, True),
                        m.load(e + 8, 2, True), m.load(e + 28, 4), m.load(e + 32, 4)])
        return out

    def check(self, what, cases, want, got):
        bad = 0
        for c, w, g in zip(cases, want, got):
            if w != g:
                bad += 1
                if bad < 4:
                    print(what, c, "\n  game", w, "\n  port", g)
        self.assertEqual(len(want), len(got))
        self.assertEqual(bad, 0, f"{bad} of {len(cases)} {what} cases differ")

    # the cursor -----------------------------------------------------------------
    @volumes
    def test_move_cur_nut(self):
        rng = random.Random(4)
        bits = [UP, DOWN, UP | DOWN, 0, 0, 0]
        cases, want, lines = [], [], []
        for run in range(40):
            st = [rng.randrange(-1, 5), rng.randrange(2), rng.randrange(0, 12), rng.choice([30, 9, 5]),
                  rng.randrange(100), rng.randrange(2)]
            pads = [tuple(rng.choice(bits) if rng.random() < 0.4 else 0 for _ in range(3)) for _ in range(120)]
            m = self.machine()
            self.neutral(m, st[5])
            for k, v, n in (("curno", st[0], 4), ("pushflg", st[1], 2), ("pushcnt", st[2], 2),
                            ("repeat", st[3], 2), ("demo", st[4], 4)):
                m.store(THIS + O[k], n, v)
            out = []
            for pd in pads:
                for o, v in zip((0x2d0, 0x2d4, 0x2d8), pd):
                    m.store(SYS + o, 4, v)
                self.calls, self.se = [], []
                m.call(FN["movecur"], [THIS])
                out.append([m.load(THIS + O["curno"], 4, True), m.load(THIS + O["pushflg"], 2, True),
                            m.load(THIS + O["pushcnt"], 2, True), m.load(THIS + O["repeat"], 2, True),
                            m.load(THIS + O["demo"], 4, True), self.se, self.calls])
            cases.append(st)
            want.append(out)
            lines += [f"movecur {' '.join(map(str, st))}"] + [f"pad {a} {b} {c}" for a, b, c in pads] + ["end"]
        self.check("MoveCurNut", cases, want, ask(lines))

    @volumes
    def test_switch_and_neutral(self):
        cases, want, lines = [], [], []
        for cur in range(-1, 6):
            for paro in (0, 1):
                m = self.machine()
                self.neutral(m, paro)
                m.call(FN["switch"], [THIS, cur & 0xffffffff])
                cases.append(("switch", cur, paro))
                want.append(self.calls)
                lines.append(f"switch {cur} {paro}")
        for nut in (0, 1, 2):
            for cur in (0, 1, 2):
                for push in (0, OK, CANCEL, UP, DOWN, OK | UP):
                    for paro in (0, 1):
                        m = self.machine()
                        self.neutral(m, paro)
                        m.store(THIS + O["nut"], 4, nut)
                        m.store(THIS + O["curno"], 4, cur)
                        m.store(SYS + 0x2d0, 4, push)
                        m.call(FN["neutral"], [THIS])
                        cases.append(("neutral", nut, cur, push, paro))
                        want.append([m.load(THIS + O["nut"], 4, True), m.load(THIS + O["main"], 4, True),
                                     m.load(THIS + O["curno"], 4, True), m.load(THIS + O["flashflg"], 1),
                                     self.se, self.calls])
                        lines.append(f"neutral {nut} {cur} {push} {paro}")
        for mode in range(-1, 9):
            m = self.machine()
            m.store(THIS + O["main"], 4, 0)
            m.call(FN["change"], [THIS, mode & 0xffffffff])
            cases.append(("change", mode))
            want.append([m.load(THIS + O["main"], 4, True), m.load(THIS + O["flashflg"], 1)])
            lines.append(f"change {mode}")
        self.check("SwitchCur/PlayNeutral/ChangeMainAct", cases, want, ask(lines))

    # the idle count and the fader ------------------------------------------------------
    @volumes
    def test_move_count(self):
        cases, want, lines = [], [], []
        for act in (2, 3, 5, 7, 9, 14, 15):
            for demo in (0, 5, 2378, 2379, 2380, 2398, 2399, 2400, 2401):
                m = self.machine()
                m.store(THIS + O["main"], 4, act)
                m.store(THIS + O["demo"], 4, demo)
                m.store(THIS + O["start"], 4, 2)
                m.call(FN["movecount"], [THIS])
                cases.append((act, demo))
                want.append([m.load(THIS + O["main"], 4, True), m.load(THIS + O["demo"], 4, True),
                             m.load(THIS + O["start"], 4, True), self.fade_state(m)])
                lines.append(f"movecount {act} {demo}")
        self.check("MOVEcount", cases, want, ask(lines))

    def send_packet(self, m):
        """ccScFade::SendPacket with the packet machinery hooked; the colour
        of each element drawn, as packed into the GS packet."""
        sym = self.p.symbol_named
        m.hooks[sym("GetWork__16ccDrawPacketCtrlFi").value] = lambda mm, *a: WORK + 0x1000
        m.hooks[sym("ApplyLayerScreenMatrix__6ccViewFPiPf").value] = lambda mm, *a: 0
        m.hooks[sym("sceVu0FTOI0Vector").value] = lambda mm, *a: 0
        m.hooks[sym("SetTag__8ccScFadeFP13ccScFadeTexDL").value] = lambda mm, *a: 0
        m.store(WORK + 0x1000, 0x1000, 0)
        n = sum(1 for i in range(4) if m.load(FADE + 4 + 36 * i, 2) & 1)
        m.call(inf_va(0x0015fb80), [FADE])
        out = []
        for i in range(n):
            q = WORK + 0x1000 + 128 + 144 * i + 16
            out.append([m.load(q, 1), m.load(q + 4, 1), m.load(q + 8, 1), m.load(q + 12, 1)])
        return out

    def test_fades(self):
        scripts = [
            "flash3:20:1:1:2147483648 send:30",
            "flash3:20:20:5:2164260863 send:60",
            "flash:50:2164260863 send:55",
            "fadein:20:0:2147483648 send:25",
            "flash3:3:2:4:2155905152 flash:7:2147516416 send:6 fadein:4:255:2147483903 send:12",
        ]
        want, lines = [], []
        for s in scripts:
            m = self.machine()
            m.store(FADE, 0x98, 0)
            out = []
            for op in s.split():
                name, *a = op.split(":")
                a = [int(x) for x in a]
                if name == "flash":
                    m.call(inf_va(0x00160240), [FADE, a[0], a[1]])
                elif name == "flash3":
                    m.call(inf_va(0x00160360), [FADE, a[0], a[1], a[2], a[3]])
                elif name == "fadein":
                    m.call(inf_va(0x00160400), [FADE, a[0], a[1], a[2]])
                else:
                    for _ in range(a[0]):
                        drawn = self.send_packet(m)
                        out.append([drawn, self.fade_state(m)])
            want.append(out)
            lines.append("fade " + s)
        self.check("ccScFade", scripts, want, ask(lines))

    # every frame ---------------------------------------------------------------------
    @volumes
    def test_transparency(self):
        from eemu import f_from_py
        rng = random.Random(6)
        cases, want, lines = [], [], []
        for _ in range(1500):
            act = rng.choice([3, 5, 7, 7, 9, 9, 13])
            st = [act, rng.randrange(4), rng.randrange(5), rng.choice([0, 29, 30, 31, rng.randrange(60)]),
                  f_from_py(rng.choice([0.0, 1.0, 0.05, rng.random()])),
                  f_from_py(rng.choice([0.0, 1.0, 0.96, rng.random()])),
                  f_from_py(rng.choice([0.0, 1.0, 0.95, rng.random()])),
                  rng.choice([1, -1, 12]), rng.randrange(2)]
            m = self.machine(menu_type=st[7])
            m.store(THIS + O["paro"], 1, st[8])
            for k, v in zip(("main", "dat", "opt", "wait", "tr", "trico", "trwin"), st):
                m.store(THIS + O[k], 4, v)
            m.call(FN["transp"], [THIS])
            cases.append(st)
            want.append([m.load(THIS + O["tr"], 4), m.load(THIS + O["trico"], 4), m.load(THIS + O["trwin"], 4),
                         m.load(THIS + O["wait"], 4, True), self.tps(m)])
            lines.append("transp " + " ".join(str(v) for v in st))
        self.check("AllTransparency", cases, want, ask(lines))

    @volumes
    def test_animate(self):
        from eemu import f_from_py
        rng = random.Random(7)
        labels = ["back0", "back1", "back2", "back3", "win", "app0", "ico0", "icod0", "appd1"]
        cases, want, lines = [], [], []
        for _ in range(400):
            act = rng.choice([2, 3, 4, 5, 6, 7, 7, 8, 9, 9, 12, 13, 14, 15])
            rot = rng.choice([0.0, 3.14, 3.13, -3.14159, 3.1415927, rng.uniform(-3.2, 3.2)])
            st = [act, rng.choice([0, 1, 2, 3]), rng.choice([0, 1, 2, 3, 4]), rng.randrange(2), f_from_py(rot)]
            fwd = [lab for lab in labels if rng.random() < 0.4]
            m = self.machine()
            self.neutral(m, st[3])
            for k, v in zip(("main", "dat", "opt"), st):
                m.store(THIS + O[k], 4, v)
            m.store(THIS + O["rot"] + 4, 4, st[4])
            for k in ("nesw", "datsw", "optsw"):
                m.store(THIS + O[k], 4, 7)
            self.fwd = {lab: True for lab in fwd}
            m.call(FN["animate"], [THIS])
            cases.append(st + fwd)
            want.append([m.load(THIS + O["nesw"], 4, True), m.load(THIS + O["datsw"], 4, True),
                         m.load(THIS + O["optsw"], 4, True), m.load(THIS + O["rot"] + 4, 4), self.calls])
            lines.append("animate " + " ".join(str(v) for v in st + fwd))
        self.check("AllAnimate", cases, want, ask(lines))

    @volumes
    def test_draw(self):
        cases, want, lines = [], [], []
        for act in range(17):
            for paro in (0, 1):
                m = self.machine()
                self.neutral(m, paro)
                m.store(THIS + O["main"], 4, act)
                m.call(FN["draw"], [THIS])
                cases.append((act, paro))
                want.append(self.calls)
                lines.append(f"draw {act} {paro}")
        self.check("AllDraw", cases, want, ask(lines))

    @volumes
    def test_main(self):
        rng = random.Random(10)
        labels = ["back3", "win", "app0", "icod0"]
        cases, want, lines = [], [], []
        for _ in range(600):
            # 10 and 11 (SetNextData, PlayNextData) are volumes 2-4 only.
            st = [rng.choice([a for a in range(17) if VOL[0] > 1 or a not in (10, 11)]), rng.randrange(4),
                  rng.choice([0, OK, UP, DOWN, CANCEL]), rng.randrange(2),
                  rng.choice([0, 0, 1, 2, 3]), rng.choice([0, 0, 1, 2, 3, 4])]
            fwd = [lab for lab in labels if rng.random() < 0.3]
            m = self.machine(menu_type=1, load=0)
            self.neutral(m)
            for k, v in zip(("main", "mask"), st):
                m.store(THIS + O[k], 4, v)
            for k, v in (("nut", st[3]), ("dat", st[4]), ("opt", st[5]), ("start", 9)):
                m.store(THIS + O[k], 4, v)
            m.store(SYS + 0x2d0, 4, st[2])
            self.fwd = {lab: True for lab in fwd}
            r = m.call(FN["main"], [THIS])
            cases.append(st + fwd)
            want.append([r] + [m.load(THIS + O[k], 4, True) for k in ("main", "mask", "nut", "dat", "opt", "demo",
                                                                      "curno")] + [self.se, self.calls])
            lines.append("main " + " ".join(str(v) for v in st + fwd))
        self.check("Main", cases, want, ask(lines))

    # the actions ---------------------------------------------------------------------
    @volumes
    def test_set(self):
        from eemu import f_from_py
        rng = random.Random(8)
        cases, want, lines = [], [], []
        for _ in range(60):
            st = [rng.randrange(2), rng.randrange(-1, 5), rng.randrange(2), f_from_py(rng.random()),
                  f_from_py(rng.random())]
            m = self.machine()
            self.neutral(m, st[2])
            m.store(THIS + O["nutlock"], 1, st[0])
            m.store(THIS + O["curno"], 4, st[1] & 0xffffffff)
            m.store(THIS + O["tr"], 4, st[3])
            m.store(THIS + O["trico"], 4, st[4])
            m.store(THIS + O["main"], 4, 0)
            m.store(THIS + O["demo"], 4, 99)
            for base, n in (("a_ico", 5), ("a_app", 5)):
                b = m.load(THIS + O[base], 4)
                for i in range(n):
                    m.store(b + ANM * i + 0x88, 4, ONE)
            m.store(m.load(THIS + O["a_win"], 4) + 0x88, 4, ONE)
            m.call(FN["setn"], [THIS])
            cases.append(("setn", st))
            want.append([m.load(THIS + O["main"], 4, True), m.load(THIS + O["demo"], 4, True),
                         m.load(THIS + O["nutlock"], 1), self.tps(m), self.calls])
            lines.append("setn " + " ".join(str(v) for v in st))
        for cmd in ("setnew", "setparo"):
            m = self.machine()
            self.neutral(m)
            m.store(THIS + O["main"], 4, 0)
            m.call(FN[cmd], [THIS])
            cases.append((cmd,))
            want.append([m.load(THIS + O["main"], 4, True), self.calls])
            lines.append(cmd)
        for cmd in ("setload", "setopt", "setnext"):
            for mode in (0, 1):
                for paro in (0, 1):
                    m = self.machine()
                    self.neutral(m, paro)
                    m.store(THIS + O["main"], 4, 0)
                    m.store(THIS + O["wait"], 4, 9)
                    m.store(THIS + O["optsw"], 4, 9)
                    if cmd == "setload":
                        m.store(THIS + O["dat"], 4, mode)
                        m.call(FN[cmd], [THIS])
                    else:
                        m.call(FN[cmd], [THIS, mode])
                    cases.append((cmd, mode, paro))
                    want.append([m.load(THIS + O["main"], 4, True), m.load(THIS + O["wait"], 4, True),
                                 m.load(THIS + O["optsw"], 4, True), self.calls])
                    lines.append(f"{cmd} {mode} {paro}")
        self.check("Set*", cases, want, ask(lines))

    @volumes
    def test_play(self):
        from eemu import f_from_py
        cases, want, lines = [], [], []
        half, quarter = f_from_py(0.5), f_from_py(0.25)
        for dat in range(4):
            for datsw in (0, 1):
                for flash in (0, 19, 20, 21):
                    for loadsw in (0, 1):
                        for res in (-1, 0, 1):
                            if flash not in (0, 20) and dat != 2:
                                continue
                            m = self.machine(load=res)
                            self.neutral(m)
                            for k, v in (("dat", dat), ("datsw", datsw), ("flash", flash), ("main", 7),
                                         ("start", 0), ("mask", 0), ("tr", half), ("trico", quarter)):
                                m.store(THIS + O[k], 4, v)
                            m.store(THIS + O["loadsw"], 1, loadsw)
                            m.call(FN["playload"], [THIS])
                            st = (dat, datsw, flash, loadsw, res, 0)
                            cases.append(("playload",) + st)
                            want.append([m.load(THIS + O[k], 4, True) for k in ("dat", "datsw", "flash")]
                                        + [m.load(THIS + O["loadsw"], 1)]
                                        + [m.load(THIS + O[k], 4, True) for k in ("main", "start", "mask")]
                                        + [m.load(THIS + O["tr"], 4), m.load(THIS + O["trico"], 4),
                                           self.fade_state(m), self.calls])
                            lines.append("playload " + " ".join(str(v) for v in st))
        for nxt in range(4):
            for nextsw in (0, 1):
                for flash in (0, 19, 20, 21):
                    for loadsw in (0, 1):
                        for res in (-1, 0, 1):
                            if flash not in (0, 20) and nxt != 2:
                                continue
                            m = self.machine(load=res)
                            self.neutral(m)
                            for k, v in (("next", nxt), ("nextsw", nextsw), ("flash", flash), ("main", 11),
                                         ("start", 0), ("mask", 0), ("tr", half), ("trico", quarter)):
                                m.store(THIS + O[k], 4, v)
                            m.store(THIS + O["loadsw"], 1, loadsw)
                            m.call(FN["playnext"], [THIS])
                            st = (nxt, nextsw, flash, loadsw, res, 0)
                            cases.append(("playnext",) + st)
                            want.append([m.load(THIS + O[k], 4, True) for k in ("next", "nextsw", "flash")]
                                        + [m.load(THIS + O["loadsw"], 1)]
                                        + [m.load(THIS + O[k], 4, True) for k in ("main", "start", "mask")]
                                        + [m.load(THIS + O["tr"], 4), m.load(THIS + O["trico"], 4),
                                           self.fade_state(m), self.calls])
                            lines.append("playnext " + " ".join(str(v) for v in st))
        for opt in range(5):
            for optsw in (0, 1):
                for menu in (1, -1, 12):
                    for push in (0, CANCEL, OK):
                        m = self.machine(menu_type=menu)
                        self.neutral(m)
                        for k, v in (("opt", opt), ("optsw", optsw), ("main", 9), ("mask", 0), ("tr", half),
                                     ("trico", quarter), ("wait", 5)):
                            m.store(THIS + O[k], 4, v)
                        m.store(SYS + 0x2d0, 4, push)
                        m.store(DTM + 6, 2, 0)
                        m.call(FN["playopt"], [THIS])
                        st = (opt, optsw, menu, push, 0)
                        cases.append(("playopt",) + st)
                        want.append([m.load(THIS + O[k], 4, True) for k in ("opt", "optsw", "main", "mask")]
                                    + [m.load(THIS + O["tr"], 4), m.load(THIS + O["trico"], 4),
                                       m.load(THIS + O["wait"], 4, True), m.load(DTM + 6, 2, True), self.calls])
                        lines.append("playopt " + " ".join(str(v) for v in st))
        for cmd in ("playnew", "playparo"):
            for nesw in (0, 1):
                m = self.machine()
                for k, v in (("nesw", nesw), ("main", 99), ("newgamesw", 9), ("start", 9)):
                    m.store(THIS + O[k], 4, v)
                m.store(SAVE + 0x842b, 1, 0)
                m.call(FN[cmd], [THIS])
                cases.append((cmd, nesw))
                want.append([m.load(THIS + O["main"], 4, True), m.load(THIS + O["newgamesw"], 4, True),
                             m.load(THIS + O["start"], 4, True), m.load(SAVE + 0x842b, 1)])
                lines.append(f"{cmd} {nesw}")
        self.check("Play*", cases, want, ask(lines))

    # the logos -----------------------------------------------------------------------
    def test_logo_main(self):
        from eemu import _cstr
        cases, want, lines = [], [], []
        for act in range(5):
            for skip in (0, 1):
                m = self.machine()
                ev, breaths = [], [0]
                m.hooks[inf_va(0x00409df0)] = lambda mm, a0, a1, *a: ev.append(
                    ["mpeg", "PSS/" + _cstr(mm, a0).decode().upper(), a1]) or (0xffffffff if skip else 0)
                m.hooks[inf_va(0x0015a320)] = lambda mm, *a: breaths.__setitem__(0, breaths[0] + 1) or 0
                m.hooks[inf_va(0x001794b0)] = lambda mm, a0, *a: ev.append(["vol", a0]) or 0
                m.hooks[inf_va(0x001798f0)] = lambda mm, a0, *a: ev.append(["sq", a0]) or 0
                m.store(THIS + O["logo"], 4, act)
                ret = m.call(FN["logo"], [THIS])
                cases.append((act, skip))
                want.append([m.load(THIS + O["logo"], 4, True), ret, breaths[0], ev])
                lines.append(f"logo {act} {skip}")
        self.check("LogoMain", cases, want, ask(lines))

    # the memory card -----------------------------------------------------------------
    @volumes
    def test_boot_mem_card(self):
        rng = random.Random(9)
        scripts = []
        # A card at once; a card late; no card, then yes; no card, no then
        # yes; no room; a card put in while asked.
        scripts.append([(-99, 0)] * 2 + [(1, 0)] + [(-99, 0)] * 5)
        scripts.append([(-99, 0)] * 3 + [(4, 0)] * 3 + [(1, 0)] + [(-99, 0)] * 4)
        scripts.append([(-99, 0)] * 3 + [(0x8028, 0)] * 4 + [(0x8028, UP), (-99, 0), (-99, OK)] + [(-99, 0)] * 40)
        scripts.append([(-99, 0)] * 3 + [(0x8028, OK), (0x8028, CANCEL), (-99, DOWN), (-99, UP), (-99, UP),
                                         (-99, UP), (-99, OK)] + [(-99, 0)] * 36)
        scripts.append([(-99, 0)] * 3 + [(0x8029, 0), (0x8029, UP), (-99, OK)] + [(-99, 0)] * 36)
        scripts.append([(-99, 0)] * 3 + [(0x8028, 0)] * 5 + [(1, 0)] + [(-99, 0)] * 36)
        for _ in range(6):
            scripts.append([(-99, 0)] * 3 + [(rng.choice([0x8028, 0x8029, 4, -99]),
                                             rng.choice([0, 0, UP, DOWN, OK, CANCEL])) for _ in range(30)]
                           + [(1, 0)] + [(-99, 0)] * 40)
        want, lines = [], []
        for s in scripts:
            m = self.machine()
            self.neutral(m)
            # BootMem_Control as its constructor and Init leave it.
            m.store(BOOTMEM, 0x94, 0)
            for off, n, v in ((0x18, 2, 1), (0x30, 2, 30), (0x10, 1, 1), (0x8c, 4, 0), (0x90, 4, 0xffffffff)):
                m.store(BOOTMEM + off, n, v)
            m.store(BOOTMEM + 0x88, 4, inf_va(0x00377a70))
            m.store(SSYS + 0x2a8, 4, 0)
            out = []
            prev = 0
            for res, push in s:
                if res != -99:
                    m.store(SSYS + 0x2a8, 4, res & 0xffffffff)
                m.store(SYS + 0x2d0, 4, push)
                m.store(SYS + 0x2d4, 4, prev & ~push)
                m.store(SYS + 0x2d8, 4, push)
                prev = push
                self.calls, self.se, self.events = [], [], []
                r = m.call(FN["bootcard"], [THIS])
                shown = [e for e in self.events if e & 0x8000]
                out.append([r, m.load(THIS + O["boot"], 4, True), m.load(THIS + O["err"], 4, True),
                            m.load(BOOTMEM + 0x8c, 4, True), m.load(BOOTMEM + 0x90, 4, True),
                            m.load(BOOTMEM + 0x2a, 2, True), m.load(SSYS + 0x2a8, 4, True),
                            shown[0] if shown else -1, self.se, self.calls])
            want.append(out)
            pads, prev = [], 0
            for res, push in s:
                pads.append(f"frame {res} {push} {prev & ~push} {push}")
                prev = push
            lines += ["boot"] + pads + ["end"]
        self.check("PlayBootMemCard", scripts, want, ask(lines))

    def test_question_texts(self):
        """The boot questions' texts in the port's tables against the
        game's: saveSysMsg[0x28] and [0x29] as __sinit_sdmng.cpp fills them
        (their first five NUL-separated pieces), STR_YES, STR_NO and the
        "#G" SetLoadPar highlights with."""
        from eemu import Machine, _cstr
        m = Machine(self.p)
        m.call(self.p.symbol_named("__sinit_sdmng.cpp").value)
        table = self.p.symbol_named("saveSysMsg").value

        def pieces(va, n=5):
            out = []
            for _ in range(n):
                s = _cstr(m, va)
                out.append(s.hex())
                va += len(s) + 1
            return out

        no_card, no_room, yes, no, hi = ask(["consts"])[0]
        self.assertEqual(pieces(m.load(table + 4 * 0x28, 4)), no_card)
        self.assertEqual(pieces(m.load(table + 4 * 0x29, 4)), no_room)
        self.assertEqual(bytes.fromhex(yes), _cstr(m, m.load(inf_va(0x003783ec), 4)))
        self.assertEqual(bytes.fromhex(no), _cstr(m, m.load(inf_va(0x003783f0), 4)))
        self.assertEqual(bytes.fromhex(hi), b"#G")

    # ccSaveData::NewGame ------------------------------------------------------------
    def test_new_game(self):
        """The whole save after ccSaveData::NewGame(1) and (0), run by the
        game from the boot's save (ccSaveData's constructor, then Init(1)),
        against the port given
        the same bytes; and the names it copies to spcNameList and
        ccsNameList."""
        from eemu import Machine
        rng = random.Random(11)
        sym = self.p.symbol_named
        new_game = sym("NewGame__10ccSaveDataFi").value
        spc, ccs = sym("spcNameList").value, sym("ccsNameList").value
        size = 0x8530

        def machine():
            m = Machine(self.p)
            m.store(SAVEDATA, 4, SAVE)
            m.store(GAME, 4, 0x018b0000)
            m.store(CCSYS, 4, SYS)
            # Outside the save: the display offset (GS registers) and the
            # sound settings (IOP calls).
            m.hooks[sym("SetDisplayOffset__8ccSystemFii").value] = lambda mm, *a: 0
            m.hooks[sym("SetSoundEnv__10ccSaveDataFv").value] = lambda mm, *a: 0
            return m

        def names(m, before_spc, before_ccs):
            # Only the bytes NewGame wrote: up to and with each NUL.
            out = []
            for base, n, size_, cap, before in ((spc, 17, 24, 20, before_spc), (ccs, 18, 32, 32, before_ccs)):
                lst = []
                for i in range(n):
                    b = bytes(m.mem[base + size_ * i:base + size_ * i + cap])
                    lst.append((b[:b.index(0) + 1] if 0 in b else b).hex())
                out.append(lst)
            return out

        runs = []
        for parody in (0, 1):
            m = machine()
            m.mem[SAVE:SAVE + size] = test_save_init_rs.boot_save(ELF)
            m.store(SAVE + 0x842b, 1, parody)
            runs.append((m, [1, 0, 0]))
        m = machine()
        m.mem[SAVE:SAVE + size] = bytes(rng.randrange(256) for _ in range(size))
        runs.append((m, [0, 1]))
        cases, want, lines = [], [], []
        for m, sws in runs:
            for sw in sws:
                before = bytes(m.mem[SAVE:SAVE + size])
                m.store(spc, 17 * 24, 0)
                m.store(ccs, 18 * 32, 0)
                m.call(new_game, [SAVE, sw])
                after = bytes(m.mem[SAVE:SAVE + size])
                cases.append((sw, m.load(SAVE + 0x842b, 1), after != before))
                want.append([after.hex()] + names(m, None, None))
                lines.append(f"newgame {sw} {SAVE:#x} {before.hex()}")
        got = ask(lines)
        for c, w, g in zip(cases, want, got):
            if w != g:
                diff = [hex(i // 2) for i in range(0, len(w[0]), 2) if w[0][i:i + 2] != g[0][i:i + 2]][:12]
                print("NewGame", c, "bytes differing at", diff, "names", w[1:] == g[1:])
        self.check("ccSaveData::NewGame", cases, want, got)

    # ccThDemo ------------------------------------------------------------------------
    def run_thread(self, reset, skips, boots, mains, frames):
        from eemu import _cstr
        m = self.machine()
        out, left = [], [frames]
        boots, mains, skips = list(boots), list(mains), list(skips)
        m.store(RESETOPFLG, 1, reset)
        m.store(DTM + 0x18, 2, 0xffff)
        m.store(GAME, 4, 0x018b0000)

        def breath(mm, a0=None, a1=1, *a):
            for _ in range(a1 if a0 is not None else 1):
                out.append(["breath", mm.load(DTM + 0x18, 2, True)])
                left[0] -= 1
                if left[0] == 0:
                    raise Stop()
            return 0

        def tbreath(mm, a0, a1, *a):
            return breath(mm, a0, a1)

        def cbreath(mm, a0, *a):
            m.store(TSCB + 0x14, 4, 0xffffffff)
            return breath(mm)

        def boot(mm, *a):
            out.append(["boot"])
            return boots.pop(0) if boots else 1

        def main(mm, *a):
            out.append(["main"])
            return (mains.pop(0) if mains else 0) & 0xffffffff

        def mpeg(mm, a0, a1, *a):
            # The movie plays inside ccDecodeMpeg; the port stands for all
            # its frames with one step, which ends where the game's last
            # frame of it does: the ccBreathThread(1) after termAll.
            out.append(["mpeg", "PSS/" + _cstr(mm, a0).decode().upper(), a1])
            r = 0xffffffff if (skips.pop(0) if skips else 0) else 0
            breath(mm)
            return r

        def stream_adrs(mm, *a):
            if not out or out[-1][0] != "stream":
                out.append(["stream", mm.load(TSCB + 0x14, 4, True)])
            return STREAM

        m.store(STREAM + 0x17e, 2, 8)
        m.hooks.update({
            inf_va(0x00159e10): tbreath, inf_va(0x0015a320): cbreath,
            FN["bootcard"]: boot, FN["main"]: main, inf_va(0x00409df0): mpeg,
            inf_va(0x00100c90): lambda mm, *a: THIS, inf_va(0x00406610): lambda mm, a0, *a: a0, inf_va(0x00405e80): lambda mm, *a: 0,
            inf_va(0x00179aa0): lambda mm, a0, *a: out.append(["sqstop", a0]) or 0,
            inf_va(0x001798f0): lambda mm, a0, *a: out.append(["sq", a0]) or 0,
            inf_va(0x001794b0): lambda mm, a0, *a: out.append(["vol", a0]) or 0,
            inf_va(0x00179b50): lambda mm, a0, a1, a2, a3: out.append(["sqfade", a0, a1, a2, a3]) or 0,
            inf_va(0x00174d70): lambda mm, a0, a1, *a: out.append(["newgame", a1]) or 0,
            inf_va(0x00175110): lambda mm, *a: out.append(["loadgame"]) or 0,
            inf_va(0x001752a0): lambda mm, *a: out.append(["convgame"]) or 0,
            inf_va(0x001b55f0): lambda mm, *a: 0,
            inf_va(0x001671e0): lambda mm, a0, a1, a2, *a: out.append(["change", a1, a2]) or 0,
            inf_va(0x00159ed0): lambda mm, *a: TSCB, inf_va(0x00159f70): lambda mm, *a: 0,
            inf_va(0x0019ab80): stream_adrs, inf_va(0x0019aaf0): lambda mm, *a: 0,
            inf_va(0x00160240): lambda mm, *a: 0, inf_va(0x00160360): lambda mm, *a: 0,
        })
        try:
            m.call(FN["thdemo"], [0x018c0000], limit=50_000_000)
        except Stop:
            pass
        return out

    def test_thread(self):
        runs = [
            (0, [0, 0, 0, 0], [0, 0, 0, 0, 1], [0] * 5 + [2], 110),
            (0, [0, 1], [1], [0, 0, 3], 90),
            (0, [1], [0, 1], [0] * 3 + [5] + [0] * 2 + [2], 170),
            (1, [], [0, 0, 1], [0, 0, 0, 0, 2], 20),
            (0, [0, 0, 0, 1], [1], [0, 0, 5, 0, 0], 180),
            (1, [], [1], [0, 1, 0, 4, 0], 30),
        ]
        want, lines = [], []
        for reset, skips, boots, mains, frames in runs:
            want.append(self.run_thread(reset, skips, boots, mains, frames))
            lst = lambda v: ",".join(map(str, v)) if v else "-"  # noqa: E731
            lines.append(f"thread {reset} {lst(skips)} {lst(boots)} {lst(mains)} {frames}")
        got = ask(lines)
        # ccThDemo's dispFlag reads as it is at each breath; the port reports
        # its last setting (-1 before the first).
        self.check("ccThDemo", runs, want, got)

    # the load screen -----------------------------------------------------------------
    def load_card(self, root, rng, kind):
        """A card kept as files under `root`: FilesCard's layout. kind
        "none" has no card, "empty" a card with no save directory, "saves"
        an index with used, empty, corrupt and missing slots."""
        slots = {}
        if kind == "none":
            return None, None, slots
        os.makedirs(root, exist_ok=True)
        if kind == "empty":
            return root, None, slots
        d = os.path.join(root, volume.card_dir())
        os.makedirs(d)
        index = bytearray(336)
        for i in range(12):
            status = rng.choice([0, 1, 1, 1])
            rec = bytearray(28)
            if status:
                data = bytearray(rng.randrange(256) for _ in range(volume.SLOT_SIZE))
                ok, cancel = rng.choice([(0x40, 0x20), (0x40, 0x20), (0x20, 0x40)])
                data[0x840e:0x8410] = struct.pack("<h", ok)
                data[0x8410:0x8412] = struct.pack("<h", cancel)
                name = bytes(rng.choice(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ") for _ in range(rng.randrange(1, 12)))
                rec[0] = 1
                rec[1] = rng.randrange(100)
                rec[2] = rng.choice([0, 0, 1])
                rec[3] = rng.choice([0, 0, 1]) if rec[2] == 0 else 0
                rec[4:4 + len(name)] = name
                good = sum(data) & 0xffff
                fate = rng.choice(["ok", "ok", "ok", "sum", "missing", "short"])
                struct.pack_into("<H", rec, 0x16, good if fate != "sum" else (good + 1) & 0xffff)
                struct.pack_into("<i", rec, 0x18, rng.randrange(0x0cdfe5c5))
                if fate != "missing":
                    with open(os.path.join(d, "dhdata%02d" % (i + 1)), "wb") as f:
                        f.write(bytes(data) if fate != "short" else bytes(data[:0x8000]))
                slots[i] = bytes(data) if fate in ("ok", "sum") else None
            index[28 * i:28 * i + 28] = rec
        with open(os.path.join(d, volume.card_dir()), "wb") as f:
            f.write(bytes(index))
        return root, bytes(index), slots

    def load_machine(self, root, index, slots, port, file):
        """ccSaveSys and DataLoad_Control built as the title builds them,
        ccMcard answered from the card, the draws recorded."""
        from eemu import Machine, _cstr
        sym = lambda n: self.p.symbol_named(n).value  # noqa: E731
        m = Machine(self.p)
        m.store(CCSYS, 4, SYS)
        m.store(SAVEDATA, 4, SAVE)
        m.store(SAVESYS, 4, SSYS)
        m.store(GAME, 4, 0)
        # The port's fresh save, made by the game's own boot (ok cross,
        # cancel circle): what the title holds when it loads.
        m.mem[SAVE:SAVE + 0x8530] = test_save_init_rs.fresh_save(ELF)
        heap = [0x01a00000]

        def alloc(mm, a0, *a):
            r = heap[0]
            heap[0] += (a0 + 15) & ~15
            mm.mem[r:r + a0] = bytes(a0)
            return r

        def kanji(mm, a0, *a):
            mm.mem[a0:a0 + 232] = bytes(232)
            for off in (104, 108, 112, 116):
                mm.store(a0 + off, 4, 0x80)
            return a0

        def mask_ct(mm, a0, *a):
            mm.mem[a0:a0 + 216] = bytes(216)
            mm.store(a0 + 116, 4, 0x80)
            return a0

        names = {}

        def kname(a):
            for name, off, n in (("info", 0x44, 4), ("slot", 0x3c, 2), ("data", 0x40, 12), ("dia", 0x48, 2),
                                 ("mc", 0x4c, 2), ("par", 0x50, 4)):
                base = m.load(DATALOAD + off, 4)
                if base <= a < base + 232 * n:
                    return f"{name}[{(a - base) // 232}]"
            return hex(a)

        def disp(mm, a0, a1, *a):
            text = _cstr(mm, a1)
            self.events.append(["disp", kname(a0), 129, mm.load(a0 + 64, 4), mm.load(a0 + 68, 4),
                                [mm.load(a0 + 104, 4), mm.load(a0 + 108, 4), mm.load(a0 + 112, 4),
                                 mm.load(a0 + 116, 4)], text.hex()])
            return 0

        def packet(mm, a0, *a):
            self.events.append(["pkt", "cur", 129, [mm.load(a0 + o, 4) for o in (64, 68, 72, 76, 52, 56)],
                                mm.load(a0 + 80, 4, True), mm.load(a0 + 84, 4, True), mm.load(a0 + 88, 4, True),
                                mm.load(a0 + 92, 4, True), mm.load(a0 + 116, 4)])
            return 0

        def check_port(mm, a0, a1, *a):
            if a1 != 0 or root is None:
                return 0
            return 0xffffffff if index is not None else 4

        def read_sys(mm, a0, a1, a2, a3):
            if a1 != 0 or index is None:
                return 5
            mm.mem[a3:a3 + 336] = index
            return 0

        def data_read(mm, a0, a1, a2, a3):
            buf = mm.r[8] & 0xffffffff
            data = slots.get(a3) if a1 == 0 else None
            if data is None:
                return 5
            mm.mem[buf:buf + 0x8530] = data
            return 0

        nop = lambda mm, *a: 0  # noqa: E731
        m.hooks.update({
            sym("__nw__FUi"): alloc, sym("__nwa__FUi"): alloc, inf_va(0x00403600): kanji,
            sym("__ct__6ccMaskFii"): mask_ct, sym("__ct__16ccDrawPacketCtrlFv"): nop,
            sym("Init__7ccLayerFsP6ccView"): nop, sym("SetFrame__6ccViewFffffffff"): nop,
            sym("SetTex__8ccSpriteFPcPci"): nop, sym("ccStartThread__FPFPv_vii"): lambda mm, *a: 1,
            sym("Disp__7ccKanjiFPciff"): disp, sym("MakePacket__8ccSpriteFii"): packet,
            sym("SendPacket__8ccSpriteFv"): nop, sym("GetFrameRate__8ccSystemFv"): lambda mm, *a: 1,
            sym("ccSeOn__Fi"): lambda mm, a0, *a: self.events.append(["se", a0]) or 0,
            sym("CheckPort__7ccMcardFii"): check_port, sym("ReadSys__7ccMcardFiiPvii"): read_sys,
            sym("DataRead__7ccMcardFiiiPvii"): data_read, sym("ccMalloc__FUi"): alloc,
            sym("ccFree__FPv"): nop,
        })
        self.events = []
        # saveSysMsg's pointers.
        m.call(sym("__sinit_sdmng.cpp"))
        m.call(sym("__ct__9ccSaveSysFv"), [SSYS])
        m.store(SSYS + 0x2a0, 4, port)
        m.store(SSYS + 0x2a4, 4, file)
        m.mem[DATALOAD:DATALOAD + 0x90] = bytes(0x90)
        m.call(sym("__ct__16DataLoad_ControlFv"), [DATALOAD])
        m.call(sym("StartReq__9ccSaveSysFi"), [SSYS, 1])
        # As the boot check leaves it.
        m.store(SSYS + 0x2a8, 4, 1)
        m.store(SSYS + 0x2ac, 4, 1)
        return m

    def test_data_load(self):
        rng = random.Random(21)
        scripts = []
        # (card kind, port, file, pads): pads are (push, repeat) per frame.
        go = [(0, 0)] * 3
        scripts.append(("saves", 0, 0, go + [(OK, OK)] + [(0, 0)] * 3 + [(DOWN, DOWN), (0, 0), (OK, OK), (0, 0),
                                                                          (UP, UP), (0, 0), (OK, OK), (0, 0),
                                                                          (0, 0), (OK, OK)] + [(0, 0)] * 3))
        scripts.append(("saves", 0, 3, go + [(CANCEL, CANCEL)] + [(0, 0)] * 2))
        scripts.append(("empty", 0, 0, go + [(OK, OK)] + [(0, 0)] * 4 + [(OK, OK)] + [(0, 0)] * 4))
        scripts.append(("none", 1, 5, go + [(OK, OK)] + [(0, 0)] * 4 + [(CANCEL, CANCEL)] + [(0, 0)] * 4))
        scripts.append(("saves", 0, 0, go + [(DOWN, DOWN), (0, 0), (OK, OK)] + [(0, 0)] * 4 + [(OK, OK)]
                        + [(0, 0)] * 4))
        for _ in range(10):
            pads = [(0, 0)] * 2
            held = 0
            for _ in range(rng.randrange(60, 160)):
                b = rng.choice([0, 0, 0, OK, OK, CANCEL, UP, DOWN, "hold"])
                if b == "hold":
                    held = rng.choice([UP, DOWN])
                    for _ in range(rng.randrange(5, 45)):
                        pads.append((0, held))
                    pads.append((0, 0))
                    continue
                pads.append((b, b))
            scripts.append((rng.choice(["saves", "saves", "saves", "empty"]), rng.choice([0, 0, 1]),
                            rng.randrange(12), pads))
        want, lines, cases = [], [], []
        tmp = tempfile.mkdtemp(prefix="piney-load-")
        try:
            for n, (kind, port, file, pads) in enumerate(scripts):
                root, index, slots = self.load_card(os.path.join(tmp, f"card{n}"), random.Random(n), kind)
                m = self.load_machine(root, index, slots, port, file)
                out = []
                prev = 0
                sym = lambda nm: self.p.symbol_named(nm).value  # noqa: E731
                for f, (push, repeat) in enumerate(pads):
                    unpush = prev & ~repeat
                    m.store(SYS + 0x2d0, 4, push)
                    m.store(SYS + 0x2d4, 4, unpush)
                    m.store(SYS + 0x2d8, 4, repeat)
                    m.store(SYS + 856, 4, 100 + f)
                    prev = repeat
                    self.events = []
                    m.call(sym("MainProccess__9ccSaveSysFv"), [SSYS])
                    r = m.call(inf_va(0x004037f0), [DATALOAD])
                    out.append([r - (1 << 32) if r & 0x80000000 else r, m.load(DATALOAD + 0x14, 4, True),
                                m.load(DATALOAD + 0x28, 2, True), m.load(DATALOAD + 0x2a, 2, True),
                                m.load(DATALOAD + 8, 1), m.load(DATALOAD + 0x1c, 4, True),
                                m.load(DATALOAD + 0xc, 4, True), m.load(DATALOAD, 4, True),
                                m.load(SSYS + 0x2a8, 4, True), m.load(SSYS + 0x2ac, 4, True),
                                m.load(SSYS + 0x2a0, 4, True), m.load(SSYS + 0x2a4, 4, True), self.events])
                    if r == 1:
                        break
                want.append({"frames": out, "save": bytes(m.mem[SAVE:SAVE + 0x8530]).hex()})
                cases.append((kind, port, file, len(out)))
                prev = 0
                frames = []
                for f, (push, repeat) in enumerate(pads[:len(out)]):
                    frames.append(f"frame {push} {prev & ~repeat} {repeat} {100 + f}")
                    prev = repeat
                lines += [f"load {root or os.path.join(tmp, 'nocard')} {port} {file}"] + frames + ["end"]
            got = ask(lines)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        loads = sum(1 for w in want if w["frames"][-1][0] == 1)
        self.assertGreater(loads, 0, "no script loaded a save")
        for c, w, g in zip(cases, want, got):
            for i, (a, b) in enumerate(zip(w["frames"], g["frames"])):
                self.assertEqual(a, b, f"{c} frame {i}")
            self.assertEqual(len(w["frames"]), len(g["frames"]), f"{c}")
            self.assertEqual(w["save"], g["save"], f"{c}: the save")

    def test_load_game(self):
        from eemu import Machine
        sym = lambda n: self.p.symbol_named(n).value  # noqa: E731
        ccs = inf_va(0x00387600)
        rng = random.Random(22)
        want, lines = [], []
        for _ in range(6):
            data = bytes(rng.randrange(256) for _ in range(0x8530))
            m = Machine(self.p)
            m.store(SAVEDATA, 4, SAVE)
            m.store(CCSYS, 4, SYS)
            m.hooks[sym("SetDisplayOffset__8ccSystemFii")] = lambda mm, *a: 0
            m.hooks[sym("SetSoundEnv__10ccSaveDataFv")] = lambda mm, *a: 0
            m.hooks[sym("setCameraCtrlType__Fi")] = lambda mm, *a: 0
            m.mem[SAVE:SAVE + 0x8530] = data
            m.call(sym("LoadGame__10ccSaveDataFv"), [SAVE])
            names = []
            for i in range(18):
                # The bytes LoadGame wrote: up to and with the NUL, at most 32.
                b = bytes(m.mem[ccs + 32 * i:ccs + 32 * i + 32])
                names.append((b[:b.index(0) + 1] if 0 in b else b).hex())
            want.append([bytes(m.mem[SAVE:SAVE + 0x8530]).hex(), names])
            lines.append(f"loadgame {SAVE:#x} {data.hex()}")
        got = ask(lines)
        for i, (w, g) in enumerate(zip(want, got)):
            self.assertEqual(w[0], g[0], f"save {i}")
            self.assertEqual(w[1], g[1], f"names {i}")

    # the icons' light -----------------------------------------------------------
    def vu_light(self, world, model, mm, t):
        """VU1's light registers and per-vertex RGBAQ for one mmat of a lit
        model with lwMatrix `world`, through the game's code."""
        import eemu
        import vu
        from test_anim import machine_class
        F, sym = eemu.f_from_py, lambda n: self.p.symbol_named(n).value  # noqa: E731
        light, env, param, lw, ctrl, view, pkt, vec = (WORK + 0x4000 + 0x400 * i for i in range(8))
        m = machine_class()(self.p)
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, WORK)
        # ccOpening_Control::Init's light (0x00406390..0x00406444).
        m.call(sym("__ct__7ccLightFsSc"), [light, 4, 1])
        m.store(light + 164, 4, sym("__vt__11ccOmniLight"))
        m.call(sym("Init__11ccOmniLightFP12ccLightChunk"), [light, 0])
        m.f[12] = F(1.0)
        m.call(sym("ccSetColor__FPfUif"), [light + 176, 0xffffff])
        m.call(sym("sceVu0UnitMatrix"), [light + 64])
        for k, v in enumerate((-8500.0, 0.0, 6500.0, 1.0)):
            m.store(vec + 4 * k, 4, F(v))
        m.call(sym("sceVu0TransMatrix"), [light + 64, light + 64, vec])
        m.store(light + 141, 1, 1)
        # cc3d: ccDrawEnv::Reset's ambient, the light in its group.
        m.mem[env + 48:env + 64] = m.mem[inf_va(0x002f7340):inf_va(0x002f7350)]
        m.call(sym("AddGrp__10ccLightGrpFP7ccLight"), [env + 128, light])
        # ccModel::Draw's ccDrawModelParam; the view only feeds positions.
        for k, v in enumerate(world):
            m.store(lw + 4 * k, 4, F(v))
        for off, n, v in ((144, 4, lw), (340, 4, env), (348, 4, ctrl), (362, 2, model.mtype),
                          (364, 4, F(model.scale)), (292, 4, F(t))):
            m.store(param + off, n, v)
        m.store(ctrl + 44, 4, view)
        for off in (208, 228, 248, 268, 476, 492):
            m.store(view + off, 4, F(1.0))
        m.call(sym("SetLightMatrix__9ccDrawEnvFPA4_fPA4_fPf"), [env, param + 160, param + 224, lw + 48])
        m.hooks[sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm_, *a: pkt
        m.call(sym("ccSetMatrixPacket__FP16ccDrawModelParam"), [param])
        u = vu.Vu(self.microcode().vu1)
        # The matrix packet's UNPACK V4-32 addr 8 num 17, MSCAL mc_SetMatrix.
        for q in range(17):
            u.mem[8 + q] = [m.load(pkt + 64 + 16 * q + 4 * k, 4) for k in range(4)]
        u.run(self.microcode().vu1.labels["mc_SetMatrix"])
        regs = {r: list(u.vf[r]) for r in (5, 6, 7, 9, 10, 11, 12)}
        # ccSetMaterialPacket's qwords 8-9: z the program ccModel::Draw
        # picked (mc_DrawTriL), w the transparency.
        u.mem[8] = [0, 0, self.microcode().vu1.labels["mc_DrawTriL"], F(t)]
        u.mem[9] = [0, 0, 0, 0]
        u.run(self.microcode().vu1.labels["mc_SetObjParam"])
        # The model packet (tools/vu.py packet): 48-vertex batches, STCYCL
        # 4,1 from addr 9: position (V3-16), normal + strip flag (V4-8),
        # ST (V2-16, STROW 0), then MSCAL mc02_Start00 / mc02_Start01.
        unit = model.scale / 4096
        out = []
        for b0 in range(0, len(mm.positions), 48):
            cnt = min(48, len(mm.positions) - b0)
            u.mem[0] = [cnt | 0x8000, 0, 0, 0]
            for i in range(cnt):
                v = b0 + i
                u.mem[9 + 4 * i][:3] = [round(x / unit) & 0xffffffff for x in mm.positions[v]]
                u.mem[10 + 4 * i] = [round(x * 64) & 0xffffffff for x in mm.normals[v]] + [mm.flags[v]]
                u.mem[11 + 4 * i][:2] = [round(x * 256) for x in mm.uvs[v]]
            u.kicks = []
            u.run(self.microcode().vu1.labels["mc02_Start00" if b0 == 0 else "mc02_Start01"])
            # The GIFtag, then per vertex FOG, ST, RGBAQ, XYZ2.
            out += [list(u.mem[u.kicks[0] + 3 + 4 * i]) for i in range(cnt)]
        return regs, out

    def microcode(self):
        import vu
        if not hasattr(self, "_mc"):
            DemoAgainstGame._mc = vu.Microcode(ELF)
        return self._mc

    def test_icon_lighting(self):
        import ccs
        import ccsmodel
        from eemu import f_to_py
        models = {m.obj: m for m in ccsmodel.models(ccs.Ccs(ccs.load(TITLE1)))}
        objs = ask(["lit 0 20"])[0]
        self.assertGreater(len(objs), 0)
        vertices, off_by_one, bad = 0, 0, []
        for e in objs:
            model = models[e["model"]]
            for md in e["mmats"]:
                mm = model.mmats[md["index"]]
                regs, got = self.vu_light(e["world"], model, mm, md["alpha"])
                # mc_SetMatrix's registers: vf05-07 the model-space
                # directions (one light per lane), vf09-11 the colours
                # (doubled), vf12 the ambient.
                for i in range(3):
                    d = [f_to_py(regs[r][i]) for r in (5, 6, 7)] if e["colours"][i] != [0, 0, 0] else None
                    if d is not None:
                        for a, b in zip(d, e["dirs"][i]):
                            self.assertAlmostEqual(a, b, delta=2e-6, msg=f"{e['label']} light {i} dir")
                    for a, b in zip([f_to_py(x) for x in regs[9 + i][:3]], e["colours"][i]):
                        self.assertAlmostEqual(a, b, delta=1e-6, msg=f"{e['label']} light {i} colour")
                for a, b in zip([f_to_py(x) for x in regs[12][:3]], e["ambient"]):
                    self.assertAlmostEqual(a, b, delta=1e-6, msg=f"{e['label']} ambient")
                for i, (g, q) in enumerate(zip(got, md["colours"])):
                    vertices += 1
                    port = [int(x) for x in q]
                    diff = max(abs(a - b) for a, b in zip(g, port))
                    off_by_one += diff == 1
                    if diff > 1:
                        bad.append((e["frame"], e["label"], e["model"], md["index"], i, g, port))
        if bad:
            self.fail(f"{len(bad)} of {vertices} vertex colours differ from VU1 (first: frame, anm, model, "
                      f"mmat, vertex, VU1 RGBA, port RGBA): {bad[:4]}")
        print(f"\nicon lighting: {vertices} vertices, {off_by_one} off by one", file=sys.stderr)


if __name__ == "__main__":
    unittest.main()
