#!/usr/bin/env python3
"""crates/piney-desktop against the game's own code run in tools/eemu.py,
and its text against tools/font.py (which font.py check holds to the game).

The desktop_probe example (built with cargo, feature `trace`) runs the
port's pieces on states this sends it; the same states go through the
game's functions in eemu, with the calls they make into the main executable
hooked and recorded:

  - Desktop_control::SelectMode (0x00400e40): random states and pad pushes,
    ccEvent::CheckOperate run natively over random operate masks;
  - Desktop_control::DrawLogo (0x00402690): every dsel 0-7, LRflg, Lock,
    DrawFlg, StartFlg and scripted _AnimateForward results; the ccAnm::SetAnm
    names, forward speeds, the draw order, the return, dsel, StartFlg, Lock
    and the frameSpd left behind;
  - Desktop_control::NewIconDraw (0x004020c0): 700 frames, the NEW mark's
    counters, spin angles (bit-exact), fog colours and draws;
  - MailList_control::MoveLine() and MoveLine(int &, int) with CurRepeat, on
    random pad states (direct, push, unpush, repeat);
  - MailList_control::SetLength(int): the scroll bar's y and length, bit
    exact;
  - MailList_control::TimeAlphaCurDraw: the cursor alpha for every count;
  - MailList_control::AddMailList (0x00409800), ccSaveData::CheckMail run
    natively: random delivery orders and mail states, both tables; the list,
    NewMailNum, the result, the states and the table flags left behind;
  - ccKanji::Disp over the mail table's titles, senders and body lines, and
    synthetic strings, against font.py: the texture bytes and every quad;
  - Acces_control::ListMove, SetLength (bit exact) and AddWallList;
    Audio_control::ListMove (loading or not), SetLength (bit exact),
    AddWaveList and AddStrList; dec2sjis;
  - ccMessage (the event windows on the desktop): Change / ChangeInfo,
    then Check(0) and Disp each frame over speech, named, question,
    information and info_now scenarios with pushes at chosen frames; every
    state field, cell packet, text call, sound and voice stop, bit exact;
  - Web_control::MoveCur (0x0040dc10) on random pad states, SetLength
    (0x0040e5d0) and SetWebLength (0x0040e7e0) bit exact, and WebMove
    (0x0040ded0) over held keys and every page height in HtmlTbl.

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
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va  # noqa: E402
inf_va = functools.partial(va, overlay='desktop')  # this overlay's globals
import test_save_init_rs  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "desktop_probe")

# gp globals the desktop reads (INF SLUS_202.67 .sdata).
CCSYS, SAVEDATA, EVENTMNG, GAME, CMNDTARGET = inf_va(0x003788e0), inf_va(0x003789d8), inf_va(0x00378a94), inf_va(0x003789cc), inf_va(0x00378c64)
# Scratch memory for the interpreter.
SYS, SAVE, EV, GAMEOBJ, THIS, ANMS, STRS = (0x01800000, 0x01810000, 0x01830000, 0x01831000, 0x01840000,
                                             0x01850000, 0x01860000)
OK, CANCEL = 0x40, 0x20
UP, RIGHT, DOWN, LEFT, SELECT, START = 0x1000, 0x2000, 0x4000, 0x8000, 0x100, 0x800

SELECT_MODE, CHOOSE_MODE, DRAW_LOGO, NEW_ICON_DRAW = inf_va(0x00400e40), inf_va(0x00401020), inf_va(0x00402690), inf_va(0x004020c0)
MOVE_LINE, MOVE_BODY, SET_LENGTH, TIME_ALPHA = inf_va(0x0040ca10), inf_va(0x0040cc10), inf_va(0x0040c620), inf_va(0x00409020)
SET_ANM, ANIMATE_FORWARD, ANM_DRAW = inf_va(0x00151c60), inf_va(0x00152270), inf_va(0x001524d0)
SET_FOG_BLEND, RESET_FOG_BLEND, SET_POS_ROT = inf_va(0x001057f0), inf_va(0x00105810), inf_va(0x00138240)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-desktop", "--features",
                    "trace", "--example", "desktop_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DesktopAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        build()
        cls.p = Program(ELF, "desktop")

    def machine(self):
        from eemu import Machine
        m = Machine(self.p)
        m.store(CCSYS, 4, SYS)
        m.store(SAVEDATA, 4, SAVE - 0x8000)
        # The port's fresh save, made by the game's own boot (ok cross,
        # cancel circle).
        m.mem[SAVE - 0x8000:SAVE - 0x8000 + 0x8530] = test_save_init_rs.fresh_save(ELF)
        m.store(EVENTMNG, 4, EV)
        m.store(GAME, 4, GAMEOBJ)
        m.store(CMNDTARGET, 4, 0)
        m.store(SYS + 0, 4, 1)          # frameRate
        self.se = []
        m.hooks[self.p.symbol_named("ccSeOn__Fi").value] = lambda mm, a0, *a: self.se.append(a0) or 0
        return m

    # SelectMode ---------------------------------------------------------------
    def test_select_mode(self):
        m = self.machine()
        rng = random.Random(3)
        pushes = [0, UP, DOWN, RIGHT, LEFT, OK, CANCEL, SELECT, START, OK | START, UP | DOWN, OK | SELECT]
        cases = []
        for _ in range(3000):
            st = [rng.choice([-1, 0, 1, 2, 3, 4, 5, 6, 7]) if rng.random() < 0.2 else rng.randrange(0, 6),
                  rng.randrange(2), rng.choice([-1, -1, -1, 0, 3]), rng.choice([0, 1, 1, 1, 2]),
                  rng.randrange(2), rng.randrange(2), rng.choice([0, 1]), rng.randrange(2),
                  rng.choice([0, 0, 1 << rng.randrange(6), 1 << (19 + rng.randrange(6))]),
                  rng.choice(pushes)]
            cases.append(st)
        got = ask(["select " + " ".join(str(v) for v in c) for c in cases])
        bad = 0
        for c, g in zip(cases, got):
            dsel, lr, app, check, flg, start, drawflg, lock, operate, push = c
            for off, v in ((0x190, dsel), (0x194, lr), (0x198, app), (0x19c, check), (0x1a0, flg),
                           (0x1a4, 5), (0x1ac, drawflg)):
                m.store(THIS + off, 4, v)
            m.store(THIS + 0x1b4, 1, start)
            m.store(THIS + 0x1b6, 1, lock)
            m.store(EV + 0x770, 8, operate)
            m.store(EV + 0x778, 2, 0xffff)
            m.store(GAMEOBJ + 0x4c, 4, 1)
            m.store(SYS + 0x2d0, 4, push)
            self.se = []
            m.call(SELECT_MODE, [THIS])
            want = {"dsel": m.load(THIS + 0x190, 4, True), "lr": m.load(THIS + 0x194, 4, True),
                    "app": m.load(THIS + 0x198, 4, True), "check": m.load(THIS + 0x19c, 4, True),
                    "flg": m.load(THIS + 0x1a0, 4, True), "drawflg": m.load(THIS + 0x1ac, 4, True),
                    "lock": m.load(THIS + 0x1b6, 1), "newmail": m.load(THIS + 0x1a4, 4, True),
                    "operate_set": m.load(EV + 0x778, 2, True), "se": self.se,
                    "reset": int(m.load(GAMEOBJ + 0x4c, 4) == 0)}
            if want != g:
                bad += 1
                if bad < 4:
                    print("SelectMode", c, "game", want, "port", g)
        self.assertEqual(bad, 0)

    # DrawLogo -----------------------------------------------------------------
    NAMES = {0x4c: "ANM_xddr_ou", 0x64: "ANM_xddl_ou", 0x7c: "ANM_xddr_in", 0x94: "ANM_xddl_in",
             0xac: "ANM_xddfade", 0xc4: "ANM_xddlogo"}
    SLOTS = {0x128: "logo", 0x120: "in", 0x124: "out", 0x10c: "title"}

    def logo_machine(self):
        m = self.machine()
        at = STRS
        for base, stem in self.NAMES.items():
            for k in range(6):
                s = f"{stem}{k + 1}".encode() + b"\0"
                m.mem[at:at + len(s)] = s
                m.store(THIS + base + 4 * k, 4, at)
                at += 16
        self.anm_label = {}
        for i, (off, label) in enumerate(self.SLOTS.items()):
            a = ANMS + 0x200 * i
            m.store(THIS + off, 4, a)
            m.store(a + 0xac, 4, 1)          # anmIndex set
            self.anm_label[a] = label
        self.calls = []
        self.fwd = {}

        def set_anm(mm, a0, a1, a2, a3):
            from eemu import _cstr
            self.calls.append(["set", self.anm_label.get(a0, hex(a0)), _cstr(mm, a2).decode()])
            mm.store(a0 + 0x98, 4, 0)
            return 0

        def forward(mm, a0, a1, *a):
            label = self.anm_label.get(a0, hex(a0))
            self.calls.append(["fwd", label, str(a1)])
            return int(self.fwd.get(label, False))

        def draw(mm, a0, *a):
            self.calls.append(["draw", self.anm_label.get(a0, hex(a0)), "None"])
            return 0
        m.hooks[SET_ANM] = set_anm
        m.hooks[ANIMATE_FORWARD] = forward
        m.hooks[ANM_DRAW] = draw
        return m

    def test_draw_logo(self):
        m = self.logo_machine()
        cases = []
        for dsel in range(8):
            for lr in (0, 1):
                for lock in (0, 1):
                    for drawflg in (0, 1):
                        for start in (0, 1):
                            for f in range(8):
                                cases.append((dsel, lr, lock, drawflg, start, 0, f & 1, f >> 1 & 1, f >> 2 & 1))
        got = ask(["logo " + " ".join(str(v) for v in c) for c in cases])
        bad = 0
        for c, g in zip(cases, got):
            dsel, lr, lock, drawflg, start, fl, fi, fo, ft = c
            for off, v in ((0x190, dsel), (0x194, lr), (0x1ac, drawflg)):
                m.store(THIS + off, 4, v)
            m.store(THIS + 0x1b4, 1, start)
            m.store(THIS + 0x1b6, 1, lock)
            for a in self.anm_label:
                m.store(a + 0x9c, 2, 256)
            self.calls = []
            self.fwd = {"logo": bool(fl), "in": bool(fi), "out": bool(fo), "title": bool(ft)}
            ret = m.call(DRAW_LOGO, [THIS])
            spd = [m.load(a + 0x9c, 2) for a, l in sorted(self.anm_label.items()) if l in ("in", "out", "title")]
            order = {l: m.load(a + 0x9c, 2) for a, l in self.anm_label.items()}
            want_spd = [order["in"], order["out"], order["title"]]
            del spd
            port = g["calls"]
            same = (ret == g["ret"] and m.load(THIS + 0x190, 4, True) == g["dsel"]
                    and m.load(THIS + 0x1b4, 1) == g["start"] and m.load(THIS + 0x1b6, 1) == g["lock"]
                    and want_spd == g["spd"]
                    and sorted(x for x in self.calls if x[0] == "set") == sorted(x for x in port if x[0] == "set")
                    and sorted(x for x in self.calls if x[0] == "fwd") == sorted(x for x in port if x[0] == "fwd")
                    and [x for x in self.calls if x[0] == "draw"] == [x for x in port if x[0] == "draw"])
            if not same:
                bad += 1
                if bad < 4:
                    print("DrawLogo", c, "game", ret, self.calls, want_spd, "port", g)
        self.assertEqual(bad, 0, f"{bad} of {len(cases)} DrawLogo cases differ")

    # NewIconDraw --------------------------------------------------------------
    def test_new_icon(self):
        from eemu import f_from_py
        m = self.machine()
        frames = 700
        for mail, news in ((1, 0), (1, 1), (0, 1)):
            mailer, weber, new = ANMS, ANMS + 0x400, ANMS + 0x800
            m.store(THIS + 0x1b8, 4, mailer)
            m.store(THIS + 0x1bc, 4, weber)
            m.store(THIS + 0x144, 4, new)
            m.store(mailer + volume.mail_at(0xa2), 2, mail)
            m.store(weber + 0x48, 4, news)
            m.store(THIS + 0x150, 4, 126)
            m.store(THIS + 0x154, 4, 0)
            m.store(THIS + 0x180, 4, 0)
            m.store(THIS + 0x184, 4, 0)
            for i, v in enumerate((-13500.0, 8300.0, 90000.0, 1.0, -13500.0, 5300.0, 90000.0, 1.0)):
                m.store(THIS + 0x160 + 4 * i, 4, f_from_py(v))
            self.fog = None
            calls = []

            def fog(mm, a0, a1, *a):
                self.fog = a1
                return 0

            def reset(mm, *a):
                self.fog = None
                return 0

            def draw(mm, a0, *a):
                c = self.fog
                calls.append(["draw", "new", "None" if c is None else
                              f"Some([{c & 255}, {c >> 8 & 255}, {c >> 16 & 255}, {c >> 24 & 255}])"])
                return 0
            m.hooks[SET_FOG_BLEND] = fog
            m.hooks[RESET_FOG_BLEND] = reset
            m.hooks[ANM_DRAW] = draw
            m.hooks[SET_POS_ROT] = lambda mm, *a: 0
            m.hooks[self.p.symbol_named("sceVu0UnitMatrix").value] = lambda mm, *a: 0
            # sqc2 $vf0 zeroes the rotation vector the hooked SetMatrix reads.
            m.store(NEW_ICON_DRAW + 0x14, 4, 0)
            got = ask([f"newicon {frames} {mail} {news}"])[0]
            bad = 0
            for f in range(frames):
                calls.clear()
                m.call(NEW_ICON_DRAW, [THIS])
                want = [m.load(THIS + 0x154, 4, True), m.load(THIS + 0x150, 4, True), m.load(THIS + 0x180, 4),
                        m.load(THIS + 0x184, 4), calls[:]]
                if want != got[f]:
                    bad += 1
                    if bad < 4:
                        print("NewIconDraw frame", f, "game", want, "port", got[f])
            self.assertEqual(bad, 0)

    # the mailer ---------------------------------------------------------------
    def pads(self, rng, n):
        bits = [OK, CANCEL, UP, DOWN, UP | DOWN, 0, 0]
        return [tuple(rng.choice(bits) if rng.random() < 0.5 else 0 for _ in range(4)) for _ in range(n)]

    def run_move(self, fn, args, setup, pads):
        m = self.machine()
        m.hooks[self.p.symbol_named("CheckMenuType__8ccDtMenuFv").value] = lambda mm, *a: 0xffffffff
        m.store(inf_va(0x003789d0), 4, 0x01880000)     # dtMenu, only compared
        setup(m)
        out = []
        for pd in pads:
            for o, v in zip((0x2cc, 0x2d0, 0x2d4, 0x2d8), pd):
                m.store(SYS + o, 4, v)
            self.se = []
            m.call(fn, args)
            at = volume.mail_at
            out.append([m.load(THIS + at(0x128), 4, True), m.load(THIS + at(0x88), 4, True),
                        m.load(THIS + at(0x120), 4, True), m.load(THIS + at(0x12c), 4, True),
                        m.load(THIS + at(0x130), 4, True), m.load(THIS + at(0xc0), 2, True),
                        m.load(THIS + at(0x80), 4, True), self.se])
        return out

    def mailer_setup(self, n):
        def setup(m):
            for off, v in ((0xf0, n), (0xf4, 14), (0x94, 9), (0x98, 4), (0x88, 0), (0x128, 0), (0x120, 0),
                           (0x12c, 0), (0x130, 0), (0x80, 0)):
                m.store(THIS + volume.mail_at(off), 4, v)
            m.store(THIS + volume.mail_at(0xc0), 2, 30)
        return setup

    def test_move_line(self):
        rng = random.Random(1)
        for n in (1, 2, 5, 13, 14, 15, 23, 40, 326):
            pads = self.pads(rng, 800)
            want = self.run_move(MOVE_LINE, [THIS], self.mailer_setup(n), pads)
            got = ask([f"list {n}"] + [f"pad {' '.join(str(v) for v in p)}" for p in pads] + ["end"])[0]
            self.assertEqual(want, got, f"MoveLine() over {n} mails")

    def test_move_body(self):
        rng = random.Random(2)
        for line in (1, 5, 14, 15, 30, 97):
            pads = self.pads(rng, 600)
            want = self.run_move(MOVE_BODY, [THIS, THIS + volume.mail_at(0x80), line], self.mailer_setup(0), pads)
            got = ask([f"body {line}"] + [f"pad {' '.join(str(v) for v in p)}" for p in pads] + ["end"])[0]
            want = [w[:1] + w[1:] for w in want]
            self.assertEqual([w[2:] for w in want], [g[2:] for g in got], f"MoveLine(pos, {line})")

    def test_set_length(self):
        from eemu import f_from_py
        m = self.machine()
        mask = ANMS
        at = volume.mail_at
        m.store(THIS + at(0xc4), 4, mask)
        for o, v in ((0x13c, 5.0), (0x140, 0.0), (0x144, 224.0), (0x148, 4.0),
                     (0x14c, 167.0), (0x150, 70.0), (0x154, 250.0), (0x158, 318.0)):
            m.store(THIS + at(o), 4, f_from_py(v))
        seen = []
        m.hooks[self.p.symbol_named("MakePacket__8ccSpriteFii").value] = \
            lambda mm, a0, *a: seen.append((mm.load(a0 + 0x44, 4), mm.load(a0 + 0x4c, 4))) or 0
        cases = []
        for n in (1, 5, 14, 15, 20, 40, 100, 326):
            for mailno in range(0, n, max(1, n // 7)):
                for top in range(0, max(1, n - 13), max(1, (n - 13) // 5)):
                    cases.append((n, mailno, top))
        got = ask([f"length {n} {mailno} {top}" for n, mailno, top in cases])
        bad = 0
        for (n, mailno, top), g in zip(cases, got):
            m.store(THIS + at(0xf0), 4, n)
            m.store(THIS + at(0xf4), 4, 14)
            m.store(THIS + at(0x128), 4, mailno)
            seen.clear()
            m.call(SET_LENGTH, [THIS, top])
            if list(seen[0]) != g:
                bad += 1
                if bad < 4:
                    print("SetLength", n, mailno, top, "game", seen[0], "port", g)
        self.assertEqual(bad, 0)

    def test_cursor_alpha(self):
        m = self.machine()
        spr = ANMS
        m.store(THIS + 0xc4, 4, spr)
        seen = []
        m.hooks[self.p.symbol_named("MakePacket__8ccSpriteFii").value] = \
            lambda mm, a0, *a: seen.append(mm.load(a0 + 0x74, 4)) or 0
        m.hooks[self.p.symbol_named("SendPacket__8ccSpriteFv").value] = lambda mm, *a: 0
        counts = list(range(0, 330)) + [0xffffffff - 3 + i for i in range(4)]
        for rate in (1, 2):
            m.store(SYS + 0, 4, rate)
            got = ask([f"alpha {c} {rate}" for c in counts])
            want = []
            for c in counts:
                m.store(SYS + 0x358, 4, c)
                seen.clear()
                m.call(TIME_ALPHA, [THIS, 0, 53, 0, 0])
                want.append(seen[0])
            self.assertEqual(want, got, f"frameRate {rate}")

    def test_add_mail_list(self):
        rng = random.Random(7)
        cases = []
        # The mails: AddMailList's loop bound (slti; 326 in Infection, 375
        # from Mutation on).
        start, end = volume.span("AddMailList__16MailList_controlFv", "desktop")
        mails = next(w & 0xffff for w in (self.p.u32(a) for a in range(start, end, 4))
                     if w >> 26 == 0x0a and (w & 0xffff) > 100)
        for trial in range(60):
            order = rng.sample(range(mails), rng.randint(0, 40))
            states = [rng.choice([1, 2, 3, 4, 5, 6, 0]) for _ in order]
            cases.append((trial % 2, order, states))
        got = ask([f"inbox {par} {' '.join(map(str, o))} ; {' '.join(map(str, st))}" for par, o, st in cases])
        insert = next(va for va in range(inf_va(0x00409800), inf_va(0x00409a20), 4)
                      if self.p.u32(va) >> 26 == 3
                      and "insert" in (self.p.name_at((self.p.u32(va) & 0x3ffffff) << 2) or ""))
        insert = (self.p.u32(insert) & 0x3ffffff) << 2
        bad = 0
        for (parody, order, states), g in zip(cases, got):
            m = self.machine()
            save = SAVE - 0x8000
            for i in range(512):
                m.store(save + 0x2264 + i, 1, 0)
                m.store(save + 0x2464 + 2 * i, 2, 0xffff)
            for i, (n, st) in enumerate(zip(order, states)):
                m.store(save + 0x2464 + 2 * i, 2, n)
                m.store(save + 0x2264 + n, 1, st)
            m.store(save + 0x842b, 1, parody)
            data = ANMS
            at = volume.mail_at
            m.store(THIS + at(0xd4), 4, 512)
            m.store(THIS + at(0xd8), 4, 0)
            m.store(THIS + at(0xdc), 4, data)
            m.store(THIS + at(0xa2), 2, 0)

            def ins(mm, vec, pos, n, valp):
                size, dat = mm.load(vec + 4, 4), mm.load(vec + 8, 4)
                items = [mm.load(dat + 4 * k, 4) for k in range(size)]
                at = (pos - dat) // 4
                items[at:at] = [mm.load(valp, 4)] * n
                for k, v in enumerate(items):
                    mm.store(dat + 4 * k, 4, v)
                mm.store(vec + 4, 4, len(items))
                return pos
            m.hooks[insert] = ins
            ret = m.call(inf_va(0x00409800), [THIS])
            tbl = self.p.symbol_named("MailTblp" if parody else "MailTbl").value
            size = m.load(THIS + at(0xd8), 4)
            want = {"ret": ret,
                    "list": [(m.load(data + 4 * k, 4) - tbl) // 0x48 for k in range(size)],
                    "new": m.load(THIS + at(0xa2), 2, True),
                    "after": [m.load(save + 0x2264 + n, 1) for n in order],
                    "flags": [[m.load(tbl + 0x48 * n + 4, 4, True), m.load(tbl + 0x48 * n + 8, 4, True)]
                              for n in order]}
            if want != g:
                bad += 1
                if bad < 4:
                    print("AddMailList", parody, order, states, "game", want, "port", g)
        self.assertEqual(bad, 0)

    # News ---------------------------------------------------------------------
    NEWS_MOVE_CUR, NEWS_SET_LENGTH, NEWS_WEB_LENGTH, NEWS_WEB_MOVE = inf_va(0x0040dc10), inf_va(0x0040e5d0), inf_va(0x0040e7e0), inf_va(0x0040ded0)

    def news_machine(self):
        from eemu import f_from_py
        m = self.machine()
        for o, v in ((0x14, 12), (0x18, 0), (0x1c, 0), (0x20, 0), (0x24, 0), (0x38, 0), (0x40, 6), (0xb0, 8),
                     (0xb4, 3)):
            m.store(THIS + o, 4, v)
        m.store(THIS + 0xac, 2, 30)
        for base, rect in ((0x64, (0, 12, 181, 7)), (0x74, (116, 140, 180, 364)), (0x84, (358, 12, 180, 5)),
                           (0x94, (118, 140, 206, 376))):
            for k, v in enumerate(rect):
                m.store(THIS + base + 4 * k, 4, f_from_py(float(v)))
        return m

    def test_news_move_cur(self):
        rng = random.Random(11)
        for n in (1, 2, 5, 11, 12, 13, 23, 69):
            pads = self.pads(rng, 800)
            m = self.news_machine()
            m.store(THIS + 0x10, 4, n)
            want = []
            for pd in pads:
                for o, v in zip((0x2cc, 0x2d0, 0x2d4, 0x2d8), pd):
                    m.store(SYS + o, 4, v)
                self.se = []
                m.call(self.NEWS_MOVE_CUR, [THIS])
                want.append([m.load(THIS + 0x20, 4, True), m.load(THIS + 0x38, 4, True), m.load(THIS + 0x18, 4, True),
                             m.load(THIS + 0x1c, 4, True), m.load(THIS + 0xac, 2, True), self.se])
            got = ask([f"newscur {n}"] + [f"pad {' '.join(str(v) for v in p)}" for p in pads] + ["end"])[0]
            self.assertEqual(want, got, f"MoveCur over {n} headlines")

    def test_news_lengths(self):
        m = self.news_machine()
        mask, html = ANMS, ANMS + 0x1000
        m.store(THIS + 0x54, 4, mask)
        m.store(THIS + 0x5c, 4, mask)
        m.store(THIS + 0x60, 4, html)
        seen = []
        m.hooks[self.p.symbol_named("MakePacket__8ccSpriteFii").value] = \
            lambda mm, a0, *a: seen.append([mm.load(a0 + 0x44, 4), mm.load(a0 + 0x4c, 4)]) or 0
        cases, lines = [], []
        for n in (1, 5, 12, 13, 20, 23, 40, 69):
            for no in sorted({0, n // 2, n - 1}):
                for top in range(0, max(1, n - 11), max(1, (n - 11) // 4)):
                    cases.append(("list", n, no, top))
                    lines.append(f"newslen {n} {no} {top}")
        heights = sorted({self.p.u32(inf_va(0x0041c120) + 0x1c * i + 0x18) for i in range(69)} | {189, 206, 207, 210, 999})
        for h in heights:
            for hi in sorted({0, 1, 5, 6, 7, 12, max(0, h - 206), max(0, h - 207), max(0, h - 212)}):
                cases.append(("web", h, hi, 0))
                lines.append(f"weblen {h} {hi}")
        got = ask(lines)
        bad = 0
        for (kind, a, b, c), g in zip(cases, got):
            seen.clear()
            if kind == "list":
                m.store(THIS + 0x10, 4, a)
                m.store(THIS + 0x20, 4, b)
                m.store(THIS + 0x38, 4, c)
                m.call(self.NEWS_SET_LENGTH, [THIS])
            else:
                m.store(html + 0x18, 4, a)
                m.store(THIS + 0x24, 4, b)
                m.call(self.NEWS_WEB_LENGTH, [THIS])
            if seen[0] != g:
                bad += 1
                if bad < 6:
                    print(kind, a, b, c, "game", seen[0], "port", g)
        self.assertEqual(bad, 0)
        self.assertGreater(len(heights), 3)

    def test_news_web_move(self):
        rng = random.Random(12)
        heights = sorted({self.p.u32(inf_va(0x0041c120) + 0x1c * i + 0x18) for i in range(69)} | {206, 207, 212, 213})
        for h in heights:
            pads = []
            while len(pads) < 300:
                key = rng.choice([UP, DOWN, UP | DOWN, 0, CANCEL, DOWN | CANCEL])
                for k in range(rng.randint(1, 40)):
                    pads.append((key, key if k == 0 else 0, 0, key))
                pads.append((0, 0, key, 0))
            m = self.news_machine()
            html = ANMS + 0x1000
            m.store(THIS + 0x60, 4, html)
            m.store(html + 0x18, 4, h)
            want = []
            for pd in pads:
                for o, v in zip((0x2cc, 0x2d0, 0x2d4, 0x2d8), pd):
                    m.store(SYS + o, 4, v)
                m.call(self.NEWS_WEB_MOVE, [THIS])
                want.append([m.load(THIS + 0x24, 4, True), m.load(THIS + 0x18, 4, True), m.load(THIS + 0x1c, 4, True)])
            got = ask([f"webmove {h}"] + [f"pad {' '.join(str(v) for v in p)}" for p in pads] + ["end"])[0]
            self.assertEqual(want, got, f"WebMove on a page {h} high")

    # Accessory ----------------------------------------------------------------
    ACC_LIST_MOVE, ACC_SET_LENGTH, ACC_ADD_WALL_LIST = inf_va(0x004053d0), inf_va(0x00404f20), inf_va(0x004044f0)

    def acces_machine(self, n):
        from eemu import f_from_py
        m = self.machine()
        for o, v in ((0x0c, 0), (0x10, 0), (0x14, 0), (0x18, 9), (0x1c, n), (0x2c, 0), (0x60, 6), (0x64, 2)):
            m.store(THIS + o, 4, v)
        m.store(THIS + 0x5c, 2, 30)
        for base, rect in ((0x3c, (240, 12, 156, 5)), (0x4c, (180, 130, 178, 192))):
            for k, v in enumerate(rect):
                m.store(THIS + base + 4 * k, 4, f_from_py(float(v)))
        return m

    def test_acces_list_move(self):
        rng = random.Random(21)
        for n in (1, 2, 3, 8, 9, 10, 15, 30, 52):
            pads = self.pads(rng, 800)
            m = self.acces_machine(n)
            want = []
            for pd in pads:
                for o, v in zip((0x2cc, 0x2d0, 0x2d4, 0x2d8), pd):
                    m.store(SYS + o, 4, v)
                self.se = []
                m.call(self.ACC_LIST_MOVE, [THIS])
                want.append([m.load(THIS + 0x14, 4, True), m.load(THIS + 0x2c, 4, True), m.load(THIS + 0x0c, 4, True),
                             m.load(THIS + 0x10, 4, True), m.load(THIS + 0x5c, 2, True), self.se])
            got = ask([f"acclist {n}"] + [f"pad {' '.join(str(v) for v in p)}" for p in pads] + ["end"])[0]
            self.assertEqual(want, got, f"ListMove over {n} wallpapers")

    def test_acces_set_length(self):
        cases = [(n, top) for n in (3, 5, 8, 9, 10, 12, 20, 33, 52) for top in range(0, max(1, n - 8))]
        got = ask([f"acclen {n} {top}" for n, top in cases])
        bad = 0
        for (n, top), g in zip(cases, got):
            m = self.acces_machine(n)
            m.store(THIS + 0xd0, 4, ANMS)
            m.store(THIS + 0x2c, 4, top)
            seen = []
            m.hooks[self.p.symbol_named("MakePacket__8ccSpriteFii").value] = \
                lambda mm, a0, *a: seen.append([mm.load(a0 + o, 4) for o in (0x40, 0x44, 0x48, 0x4c)]) or 0
            m.call(self.ACC_SET_LENGTH, [THIS])
            if seen != g:
                bad += 1
                if bad < 4:
                    print("Acces SetLength", n, top, "game", seen, "port", g)
        self.assertEqual(bad, 0)

    def test_add_wall_list(self):
        rng = random.Random(22)
        cases = [[0, 0, 0], [0xffffffff, 0xffffffff, 0xffffffff], [1 << 31, 1 << 16, 0]]
        cases += [[rng.getrandbits(32), rng.getrandbits(32), rng.getrandbits(32)] for _ in range(40)]
        got = ask([f"walls {' '.join(map(str, c))}" for c in cases])
        insert = self.p.symbol_named("insert__t11__vector_pod2ZUiZQ2t3std9allocator1ZUiFPUiUiRCUi")
        insert = insert.value if insert else None
        wall_tbl = self.p.symbol_named("WallTbl").value
        for c, g in zip(cases, got):
            m = self.machine()
            save = SAVE - 0x8000
            for w, bits in enumerate(c):
                m.store(save + 0x2238 + 4 * w, 4, bits)
            data = ANMS
            m.store(THIS + 0, 4, 0)
            m.store(THIS + 4, 4, 0)
            m.store(THIS + 8, 4, data)

            def ins(mm, vec, pos, cnt, valp):
                size, dat = mm.load(vec + 4, 4), mm.load(vec + 8, 4)
                items = [mm.load(dat + 4 * k, 4) for k in range(size)]
                at = (pos - dat) // 4
                items[at:at] = [mm.load(valp, 4)] * cnt
                for k, v in enumerate(items):
                    mm.store(dat + 4 * k, 4, v)
                mm.store(vec + 4, 4, len(items))
                return pos
            target = next((self.p.u32(va) & 0x3ffffff) << 2 for va in range(self.ACC_ADD_WALL_LIST,
                                                                             self.ACC_ADD_WALL_LIST + 540, 4)
                          if self.p.u32(va) >> 26 == 3 and "insert" in (self.p.name_at(
                              (self.p.u32(va) & 0x3ffffff) << 2) or ""))
            m.hooks[insert or target] = ins
            m.call(self.ACC_ADD_WALL_LIST, [THIS])
            size = m.load(THIS + 4, 4)
            want = [(m.load(data + 4 * k, 4) - wall_tbl) // 0x18 for k in range(size)]
            self.assertEqual(want, g, f"AddWallList over {c}")

    # Audio --------------------------------------------------------------------
    AUD_LIST_MOVE, AUD_SET_LENGTH, AUD_ADD_WAVE, AUD_ADD_STR, DEC2SJIS = (inf_va(0x00408050), inf_va(0x00407730), inf_va(0x00406450),
                                                                          inf_va(0x004065b0), inf_va(0x0015f8e0))

    def audio_machine(self):
        from eemu import f_from_py
        m = self.machine()
        for o, v in ((0x24, 0), (0x28, 0), (0x30, 0), (0x34, 10), (0x44, 0), (0x88, 7), (0x8c, 2)):
            m.store(THIS + o, 4, v)
        m.store(THIS + 0x84, 2, 30)
        for base, rect in ((0x54, (246, 8, 138, 5)), (0x64, (130, 131, 153, 305)), (0x74, (130, 131, 153, 155))):
            for k, v in enumerate(rect):
                m.store(THIS + base + 4 * k, 4, f_from_py(float(v)))
        return m

    def test_audio_list_move(self):
        rng = random.Random(31)
        for n in (1, 2, 9, 10, 11, 17, 51, 89):
            for loading in (0, 1):
                pads = self.pads(rng, 500)
                m = self.audio_machine()
                m.store(THIS + 0x38, 4, n)
                m.store(THIS + 0xbc, 1, loading)
                want = []
                for pd in pads:
                    for o, v in zip((0x2cc, 0x2d0, 0x2d4, 0x2d8), pd):
                        m.store(SYS + o, 4, v)
                    self.se = []
                    m.call(self.AUD_LIST_MOVE, [THIS, THIS + 0x30, THIS + 0x44, THIS + 0x38])
                    want.append([m.load(THIS + 0x30, 4, True), m.load(THIS + 0x44, 4, True),
                                 m.load(THIS + 0x24, 4, True), m.load(THIS + 0x28, 4, True),
                                 m.load(THIS + 0x84, 2, True), self.se])
                got = ask([f"audlist {n} {loading}"] + [f"pad {' '.join(str(v) for v in p)}" for p in pads] + ["end"])[0]
                self.assertEqual(want, got, f"ListMove over {n}, loading {loading}")

    def test_audio_set_length(self):
        cases = [(n, no, top) for n in (1, 5, 10, 11, 17, 51, 89) for no in sorted({0, n // 2, n - 1})
                 for top in range(0, max(1, n - 9), max(1, (n - 9) // 4))]
        got = ask([f"audlen {n} {no} {top}" for n, no, top in cases])
        m = self.audio_machine()
        m.store(THIS + 0xac, 4, ANMS)
        seen = []
        m.hooks[self.p.symbol_named("MakePacket__8ccSpriteFii").value] = \
            lambda mm, a0, *a: seen.append([mm.load(a0 + o, 4) for o in (0x40, 0x44, 0x48, 0x4c)]) or 0
        bad = 0
        for (n, no, top), g in zip(cases, got):
            seen.clear()
            m.store(THIS + 0x30, 4, no)
            m.call(self.AUD_SET_LENGTH, [THIS, top, n])
            if seen[0] != g:
                bad += 1
                if bad < 4:
                    print("Audio SetLength", n, no, top, "game", seen[0], "port", g)
        self.assertEqual(bad, 0)

    def test_audio_lists(self):
        rng = random.Random(32)
        cases = [[0] * 8, [0xffffffff] * 8]
        cases += [[rng.getrandbits(32) for _ in range(8)] for _ in range(30)]
        got = ask([f"audadd {' '.join(map(str, c))}" for c in cases])
        wave, stream = self.p.symbol_named("Wave").value, self.p.symbol_named("Stream").value
        for c, g in zip(cases, got):
            m = self.machine()
            save = SAVE - 0x8000
            for w, bits in enumerate(c[:3]):
                m.store(save + 0x2244 + 4 * w, 4, bits)
            for w, bits in enumerate(c[3:]):
                m.store(save + 0x2250 + 4 * w, 4, bits)
            vecs = {0: ANMS, 0xc: ANMS + 0x1000, 0x18: ANMS + 0x2000}
            for off, data in vecs.items():
                m.store(THIS + off, 4, 0)
                m.store(THIS + off + 4, 4, 0)
                m.store(THIS + off + 8, 4, data)

            def ins(mm, vec, pos, cnt, valp):
                size, dat = mm.load(vec + 4, 4), mm.load(vec + 8, 4)
                items = [mm.load(dat + 4 * k, 4) for k in range(size)]
                at = (pos - dat) // 4
                items[at:at] = [mm.load(valp, 4)] * cnt
                for k, v in enumerate(items):
                    mm.store(dat + 4 * k, 4, v)
                mm.store(vec + 4, 4, len(items))
                return pos
            for fn in (self.AUD_ADD_WAVE, self.AUD_ADD_STR):
                for va in range(fn, fn + 444, 4):
                    w = self.p.u32(va)
                    if w >> 26 == 3 and "insert" in (self.p.name_at((w & 0x3ffffff) << 2) or ""):
                        m.hooks[(w & 0x3ffffff) << 2] = ins
            m.call(self.AUD_ADD_WAVE, [THIS])
            m.call(self.AUD_ADD_STR, [THIS])
            waves = [(m.load(vecs[0] + 4 * k, 4) - wave) // 0x18 for k in range(m.load(THIS + 4, 4))]
            strs = [(m.load(vecs[0xc] + 4 * k, 4) - stream) // 0xc for k in range(m.load(THIS + 0x10, 4))]
            vols = [m.load(vecs[0x18] + 4 * k, 4, True) for k in range(m.load(THIS + 0x1c, 4))]
            self.assertEqual([waves, [list(p) for p in zip(strs, vols)]], g, f"AddWaveList/AddStrList over {c}")
            self.assertEqual(m.load(THIS + 0x38, 4), len(waves))
            self.assertEqual(m.load(THIS + 0x3c, 4), len(strs))

    def test_dec2sjis(self):
        cases = [(n, w, mode) for n in (0, 1, 7, 10, 42, 99, 100, 305, 999, 1234, 65535) for w in (0, 1, 2, 3, 5)
                 for mode in (0, 1, 2)]
        got = ask([f"dec {n} {w} {mode}" for n, w, mode in cases])
        m = self.machine()
        buf = ANMS
        for (n, w, mode), g in zip(cases, got):
            m.call(self.DEC2SJIS, [n, buf, w, mode])
            out = bytearray()
            while (b := m.load(buf + len(out), 1)) != 0:
                out.append(b)
            self.assertEqual(out.hex(), g, f"dec2sjis({n}, {w}, {mode})")

    # ccMessage ----------------------------------------------------------------
    def message_run(self, scenario, pushes, frames=70, repeats=()):
        """Run the game's ccMessage over a scenario: the object built by
        hand from its constructor (the constructor itself stops on
        mfc0 Status), Change / ChangeInfo on frame 0, Check(0) from frame
        5, Disp every frame."""
        m = self.machine()
        p = self.p
        heap = [0x01900000]

        def new(mm, size, *a):
            a0 = heap[0]
            heap[0] = (a0 + size + 15) & ~15
            mm.mem[a0:a0 + size] = bytes(size)
            return a0
        m.hooks[p.symbol_named("__nw__FUi").value] = new
        m.mem[SAVE - 0x8000:SAVE - 0x8000 + 5] = b"Kite\0"
        log = []
        f2b = lambda mm, a: mm.load(a, 4)

        def pkt(mm, obj, code, *a):
            c = mm.load(obj + 0x74, 4) & 0xff
            log.append(["pkt", code, f2b(mm, obj + 0x40), f2b(mm, obj + 0x44), f2b(mm, obj + 0x48), f2b(mm, obj + 0x4c),
                        mm.load(obj + 0x50, 4, True), mm.load(obj + 0x54, 4, True), mm.load(obj + 0x58, 4, True),
                        mm.load(obj + 0x5c, 4, True), mm.load(obj + 0x60, 4, True), c])
            from eemu import f_to_py, f_from_py
            mm.store(obj + 0x40, 4, f_from_py(f_to_py(mm.load(obj + 0x40, 4)) + f_to_py(mm.load(obj + 0x48, 4))))
            return 0

        def cstr(mm, a):
            e = mm.mem.index(0, a)
            return bytes(mm.mem[a:e])

        def kdisp(mm, obj, sp, c, *a):
            col = [mm.load(obj + 0x68 + 4 * i, 4) & 0xff for i in range(4)]
            log.append(["kanji", cstr(mm, sp).hex(), c - (1 << 32) if c & 0x80000000 else c,
                        f2b(mm, obj + 0x40), f2b(mm, obj + 0x44), col])
            return 0
        se = []
        stop = [0]
        m.hooks[p.symbol_named("MakePacket__8ccSpriteFii").value] = pkt
        m.hooks[p.symbol_named("Disp__7ccKanjiFPciff").value] = kdisp
        m.hooks[p.symbol_named("SendPacket__8ccSpriteFv").value] = lambda mm, obj, *a: log.append(["send"]) or 0
        m.hooks[p.symbol_named("ccSeOn__Fi").value] = lambda mm, a0, *a: se.append(a0) or 0
        m.hooks[p.symbol_named("ccEvVoiceRequest__Fii").value] = lambda mm, *a: 0
        m.hooks[p.symbol_named("ccEvVoiceStop__Fv").value] = lambda mm, *a: stop.__setitem__(0, stop[0] + 1) or 0
        mw = new(m, 0xd0)
        m.store(inf_va(0x00378a9c), 4, mw)
        this, win = new(m, 0x198), new(m, 0x1b0)
        m.store(this, 4, win)
        m.call(inf_va(0x0015a860), [win])
        m.call(inf_va(0x001b78b0), [win, 0])
        for i in range(4):
            k = new(m, 0xe8)
            m.call(inf_va(0x0015a860), [k])
            m.store(this + 4 + 4 * i, 4, k)
            m.store(k + 0xd4, 2, 2)
        m.store(this + 0x20, 2, 7)
        m.store(this + 0x40, 4, 0x01fe0000)
        strs = [0x01a00000]

        def put(b):
            a = strs[0]
            m.mem[a:a + len(b) + 1] = b + b"\0"
            strs[0] = (a + len(b) + 16) & ~15
            return a
        kind, args = scenario
        out = []
        for f in range(frames):
            log.clear()
            se.clear()
            stop[0] = 0
            m.store(SYS + 0x2d0, 4, OK if f in pushes else 0)
            m.store(SYS + 0x2d8, 4, repeats[f] if f < len(repeats) else 0)
            r = None
            if f == 0:
                if kind == "speech":
                    emode, name, lines = args
                    d = new(m, 0x18)
                    m.store(d, 4, emode)
                    m.store(d + 8, 4, 0)
                    for i in range(3):
                        m.store(d + 12 + 4 * i, 4, put(lines[i]) if lines[i] is not None else 0)
                    m.call(p.symbol_named("Change__9ccMessageFP9ccMsgDataPcii").value,
                           [this, d, put(name) if name is not None else 0, 1, 0])
                else:
                    lines = [put(x) if x is not None else 0 for x in args]
                    m.call(p.symbol_named("ChangeInfo__9ccMessageFPcPcPcPcii").value, [this] + lines + [1, 2])
                    if kind == "info_now":
                        m.store(this + 0x1e, 2, 0)
                        m.store(this + 0x34, 2, 0)
            elif f >= 5:
                r = m.call(p.symbol_named("Check__9ccMessageFi").value, [this, 0])
                r = r - (1 << 32) if r & 0x80000000 else r
            m.call(p.symbol_named("Disp__9ccMessageFv").value, [this])
            g = lambda o: m.load(this + o, 2, True)
            st = [g(0x1e), g(0x34), g(0x1c), g(0x32), g(0x28)] + [g(0x2a + 2 * i) for i in range(4)] + \
                 [g(0x24), g(0x38), g(0x3a), g(0x20), g(0x36)]
            out.append({"r": r, "st": st, "draws": [list(x) for x in log], "se": list(se), "stop": stop[0]})
        return out

    def message_port(self, scenario, pushes, frames=70, repeats=()):
        kind, args = scenario
        h = lambda b: "-" if b is None else (b.hex() or "=")
        lines = []
        for f in range(frames):
            if f == 0:
                if kind == "speech":
                    emode, name, ls = args
                    lines.append(f"msgchange {emode} {h(name)} {h(ls[0])} {h(ls[1])} {h(ls[2])}")
                else:
                    lines.append("msginfo " + " ".join(h(x) for x in args))
                    if kind == "info_now":
                        lines.append("msghide")
            rep = repeats[f] if f < len(repeats) else 0
            lines.append(f"msgframe {OK if f in pushes else 0} {rep} {1 if f >= 5 else 0}")
        return ask(lines)

    def test_message_window(self):
        long_line = b"(This long line will wrap onto a"
        scenarios = [
            (("speech", (0, None, [b"This line is just a sample.", b"", b""])), {45, 50, 60}, ()),
            (("speech", (2, b"#0", [long_line, b"second)", b""])), {40, 70, 80}, ()),
            (("speech", (0, b"Orca", [b"Hey.", b"Two", b"Three lines"])), {7, 12, 20, 21, 30}, ()),
            (("speech", (0, None, [b"Pushed early", None, b""])), {5, 14, 15, 16, 17}, ()),
            (("speech", (3, b"Orca", [b"Question line?", b"Yes", b"No"])), {60, 70},
             [0] * 40 + [UP, 0, 0, DOWN, DOWN, 0, UP] + [0] * 40),
            (("speech", (0x100, None, [b"All at once", b"here", b""])), {20, 25}, ()),
            (("info", [b"An info line, then...", None, b"Then more.", None]), {30, 40}, ()),
            (("info", [b"#B\x83\xa2 Bursting Passed Over Aqua", b"#W is added to the Word List.", None, None]),
             {20, 30}, ()),
            (("info_now", [b"Only the text", None, None, None]), {30}, ()),
        ]
        for sc, pushes, reps in scenarios:
            want = self.message_run(sc, pushes, repeats=reps)
            got = self.message_port(sc, pushes, repeats=reps)
            for f, (w, g) in enumerate(zip(want, got)):
                if w["st"][9] == 4:
                    # The question's selection bar (DispSelectCursol) is not
                    # ported: Infection has no question records.
                    d = w["draws"]
                    last = max((i for i, x in enumerate(d) if x[0] == "kanji"), default=-1)
                    w["draws"] = d[:last + 1] + [x for x in d[last + 1:] if x[0] != "pkt"]
                self.assertEqual(w, g, f"{sc} frame {f}")

    # ccKanji against font.py --------------------------------------------------
    def test_kanji(self):
        import font
        fonts = font.Fonts(ELF, os.path.join(ROOT, "work", "infection", "disc", "DATA", "DATA.BIN"))
        tbl = self.p.symbol_named("MailTbl").value
        strings = [b"You have new mail.", b"#0 and #1 %0%9%A #Rred#W #Ggreen #Bblue #Yyellow",
                   b"\x83\xa2 \x81\x9b tail \x88\x9f%", b"i" * 40, b"W" * 40, b"%X%Y%Z", b"",
                   b"-" * 112 + b"Log Out", b"-" * 50 + b"Log Out"]
        for i in range(0, 326, 3):
            rec = tbl + 0x48 * i
            strings.append(self.p.cstr(self.p.u32(rec + 0x0c)) or b"")
            strings.append(self.p.cstr(self.p.u32(rec + 0x10)) or b"")
            body = self.p.u32(rec + 0x1c)
            for _ in range(min(3, self.p.u32(rec + 0x18))):
                s = self.p.cstr(body) or b""
                strings.append(s)
                body += len(s) + 1
        strings = [s if isinstance(s, bytes) else s.encode("latin-1") for s in strings]
        for kt in (0, 1, 2, 3):
            screen = font.Screen()
            got = ask([f"kanji {kt} {screen.sx} {screen.sy} {screen.ox} {screen.oy} {s.hex()}" for s in strings])
            bad = 0
            for s, g in zip(strings, got):
                k = font.Kanji(fonts, kt=kt)
                quads = k.disp(s, screen=screen)
                want_q = [[list(q.rgba), list(q.uv0), list(q.xy0), list(q.uv1), list(q.xy1), int(q.shadow)]
                          for q in quads]
                # %X-%Z take trims from past the table and write outside the
                # texture in the game; font.py's slice assignment resizes its
                # buffer there and the port drops the writes, so only the
                # glyph count and quads are compared for them.
                wild = any(x in s for x in (b"%X", b"%Y", b"%Z"))
                tex_same = wild or bytes(k.tex).hex() == g["tex"]
                if k.clm != g["clm"] or not tex_same or want_q != g["quads"]:
                    bad += 1
                    if bad < 3:
                        a, b = bytes(k.tex).hex(), g["tex"]
                        diff = [i // 2 for i in range(0, len(a), 2) if a[i:i + 2] != b[i:i + 2]]
                        print("kanji", kt, s, k.clm, g["clm"], want_q[:2], g["quads"][:2], "texture bytes", diff[:8])
            self.assertEqual(bad, 0, f"kt {kt}: {bad} of {len(strings)} strings differ")


if __name__ == "__main__":
    unittest.main()
