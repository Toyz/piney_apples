#!/usr/bin/env python3
"""crates/piney-fieldui (the field UI: ccMenuCtrl and ccThMenu, gcmn.prg
menu.cpp, with the event windows' ccMessage) against the game's own task
run in tools/eemu.py, frame by frame.

The game's `ccThMenu` (gcmn 0x005280d0) runs natively from its start: the
ccMenuCtrl constructor, InitMenuList, OpenMenu / CloseMenu / ChangeMenu,
Select, every handler it reaches, and ccMenuCtrl::Disp with the
ccMenuWindow cells, ccFont's numbers, FacePanelDisp, ConditionIconDisp and
ccMessage (Change, ChangeInfo, Check, Disp). What lies outside the menu
code is hooked: the leaf sprite calls (MakePacket, MakePacketStr, SendPacket,
Extract and ccKanji::Disp are logged), threads and breaths (a breath ends
a frame), sounds, voices, the minimap's alpha, and the world the HUD reads
(ccCheckTarget, ccCalcTagPosChar, ccSkillCheck ...), which each scenario
sets the same on both sides. The event task's calls (ccMsg->Change,
ChangeInfo and Check, which run before the menu task in a frame) are made
at the start of the frame they are due.

The same scenario goes through the fieldui_probe example (feature trace).
Compared each frame: the menu's state (menu, menuNext, statuses and alphas,
proccess, the list's cursor, reverseHead, cmndTargetFix), the message's
state, every packet of every sprite (cell or string, position, size,
offsets, rotation, grid, colour and alpha, mirroring), the kanji rows' text,
the message window's cells and texts, the sounds, the voice calls, the
sleep / wake / still calls and the minimap's alpha.

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

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402
try:
    # crates/piney-eemu's module when it is built into tools/ (identical to
    # eemu.py instruction for instruction, and far faster), else eemu.py.
    import eemu_rs as eemu  # noqa: E402
except ImportError:
    import eemu  # noqa: E402
from image import Program  # noqa: E402
import test_save_init_rs  # noqa: E402

Stop, f_add, f_mul, f_to_py = eemu.Stop, eemu.f_add, eemu.f_mul, eemu.f_to_py

# Exdefense's bits and the ccCharParamElement shorts they guard: pDef,
# mDef, then soil, water, fire, wind, thunder, dark.
EXDEF = ((0x1, 1), (0x2, 5), (0x4, 8), (0x8, 9), (0x10, 10), (0x20, 11), (0x40, 12), (0x80, 13))

ELF = volume.ELF
ISO = volume.ISO
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
EXAMPLE = os.path.join(TARGET, "release", "examples", "fieldui_probe")

# gp globals and fixed objects (INF SLUS_202.67 + gcmn.prg).
CCSYS, SAVEDATA, GAME_P, WORLDMAN, EVENTMNG = inf_va(0x003788E0), inf_va(0x003789D8), inf_va(0x003789CC), inf_va(0x00378A7C), inf_va(0x00378A94)
MENU_P, CCMSG, CCCHAT, FONTTEX, MENUWIN = inf_va(0x00378C88), inf_va(0x00378A8C), inf_va(0x00378A90), inf_va(0x00378960), inf_va(0x00378A9C)
FONT, FONTDEF = inf_va(0x00378954), inf_va(0x00378958)
BOOK_P = inf_va(0x00378B00)                          # book: the open Ryu Book (BOOK)
CMNDTARGET, CMNDTARGETPREV, CMNDTARGETFIX, CMNDSORTROOT = inf_va(0x00378C64), inf_va(0x00378C68), inf_va(0x00378C6C), inf_va(0x00378C60)
CMNDROOTS = (inf_va(0x00378C48), inf_va(0x00378C50), inf_va(0x00378C58))            # cmndPcRoot, cmndEneRoot, cmndObjRoot
PARTY, PLW_PW, GAMEOVER, PGRIDE = inf_va(0x00730310), inf_va(0x00730300), inf_va(0x00378C74), inf_va(0x00378CDC)
# Scratch memory for the interpreter.
SYS, SCRATCH, SAVE, GAME, WM, EV, PLAYER = (0x01000000, 0x01010000, 0x01020000, 0x01030000, 0x01031000,
                                            0x01032000, 0x01033000)
STRS, HEAP, HEAP_END = 0x01040000, 0x01100000, 0x01E00000
PAD = SYS + 0x268

OK, CANCEL, TRIANGLE, SQUARE, START = 0x40, 0x20, 0x10, 0x80, 0x800
UP, RIGHT, DOWN, LEFT = 0x1000, 0x2000, 0x4000, 0x8000
R1_, L1_ = 0x8, 0x4
# The handlers of the menus the port does not have (it closes them at once).
UNPORTED = ()
# GtHackMenu's and DataDrainMenu's code (gcmn 0x005661e0..0x00566ca8,
# 0x00532ae0..0x00535204): the calls they make themselves.
GT_HACK = volume.span("GtHackMenu__10ccMenuCtrlFv")
DRAIN = volume.span("DataDrainMenu__10ccMenuCtrlFv")


def disp_noiz():
    """Disp's interNoiz (INF 0x00522220 up to its ccNoiz::Draw call): the
    SetNoiz calls the events' noise makes. Mutation's block is longer
    (levels 3 to 5 anew), so its end is Disp's own call of Draw."""
    start = inf_va(0x00522220)
    draw = volume.program(ELF).symbol_named("Draw__6ccNoizFv").value
    lo, hi = volume.span("Disp__10ccMenuCtrlFv")
    jal = 0x0C000000 | (draw >> 2)
    end = next(a for a in range(start, hi, 4) if volume.program(ELF).u32(a) == jal)
    return (start, end + 8)


DISP_NOIZ = disp_noiz()
# The Ryu Books' code (BOOK, ccThBook): its EntryFlash calls on menuFade.
BOOK_CODE = (volume.span("CheckBookLimit__4BOOKFii")[0], volume.span("ccThBook__FPv")[1])
OWN_CALLS = (GT_HACK, DRAIN, DISP_NOIZ, BOOK_CODE)


def own_call(ra):
    return any(a <= ra < b for a, b in OWN_CALLS)
# xdhhack's animations: frames, looping (tools/anim.py list).
HACK_ANMS = {"ANM_xdhcamer": (2, False), "ANM_xdhback0": (56, False), "ANM_xdhback1": (301, True),
             "ANM_xdhcrys0": (56, False), "ANM_xdhfram0": (56, False), "ANM_xdhfram1": (56, False),
             "ANM_xdhcomp0": (76, False)}
HACK_LABELS = {"ANM_xdhcamer": "cam", "ANM_xdhback0": "back", "ANM_xdhcrys0": "crystal", "ANM_xdhfram0": "frame",
               "ANM_xdhcomp0": "comp"}
# hackCrystalOn / OnF / Off's numbers, slot i's crystal j at 4 i + j.
HACK_CRYSTAL = (5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 1, 2, 3, 4)
# ccSaveData::Init's button assignment (act, personal menu, chat, option,
# map, ok, cancel) at +0x8404.
ASSIGN = (0x40, 0x10, 0x80, 0x800, 0x100, 0x40, 0x20)

P = Program(ELF, "gcmn")


def sym(name):
    s = P.symbol_named(name)
    assert s is not None, name
    return s.value


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def has_lessons():
    """Whether evMsgTbl's first group (evMsgTblM1) holds the lesson events'
    messages (2, 3, 4), which the tutorial menus (75 - 85) open. Infection's
    does. From Mutation on, events 0 - 49 are Infection's scripts left in
    with the whole group empty: no script there runs, and the menus would
    read their records through NULL."""
    m1 = P.u32(sym("evMsgTbl"))
    return all(P.u32(m1 + 4 * e) for e in (2, 3, 4))


LESSONS = has_lessons()
NO_LESSONS = "this volume has no lesson events (evMsgTblM1 empty)"


def hx(b):
    return b.hex() if b else "="



def sx32(v):
    """A register's word as a signed int."""
    return v - (1 << 32) if v & 0x80000000 else v

class Char:
    """A ccChar the HUD reads: party member, target or enemy."""

    def __init__(self, handle, types, cid, hp, sp, mhp, msp, name, cond=(), tag=None, width=0.0, view=True,
                 pos=(0.0, 0.0, 0.0), chat=(0, 0, 0, 0, 0, 0), item=None, trap=None, size=None, busy=None,
                 exdef=None):
        self.handle, self.types, self.id = handle, types, cid
        self.hp, self.sp, self.mhp, self.msp = hp, sp, mhp, msp
        self.name, self.cond, self.tag = name, list(cond) + [0] * (16 - len(cond)), tag
        self.width, self.view, self.pos, self.chat = width, view, pos, chat
        self.item = item
        # A box's (param[2] at +0x150, skillID at +0x7c).
        self.trap = trap
        # ccCheckObjectSize (the enemy row's +0x70): 1, 3 or 4.
        self.size = size
        # A member's skillID (+0x7c) and actNum (+0xee), which
        # CheckChangeEquip reads.
        self.busy = busy
        # A foe's (Exdefense, lowered): its table row's immunities (+0x64)
        # and the bits of those defences its `real` has below the row's.
        self.exdef = exdef

    def pos_line(self):
        return f"pos {self.handle} {self.width!r} {int(self.view)} " + " ".join(repr(float(v)) for v in self.pos)

    def party_line(self, slot):
        return (f"party {slot} {self.handle} {self.types} {self.id} {self.hp} {self.sp} {self.mhp} {self.msp} "
                f"{hx(self.name)} " + " ".join(str(c) for c in self.cond))

    def target_line(self):
        tx, ty = self.tag if self.tag else (-9999, 0)
        return (f"target {self.handle} {self.types} {self.id} {self.hp} {self.sp} {self.mhp} {self.msp} "
                f"{hx(self.name)} {tx} {ty}")


class Enemy:
    """A character on cmndSortRoot's chain: its life bar's point or, off
    screen (res 0), the direction to it."""

    def __init__(self, char, dist, res, bar=(0, 0), dirc=0.0, pp=0, dead=0):
        self.c, self.dist, self.res, self.bar, self.dirc, self.pp, self.dead = char, dist, res, bar, dirc, pp, dead
        self.arrow = (0, 0)

    def line(self):
        c = self.c
        return (f"sort {c.handle} {c.types} {c.id} {c.hp} {c.sp} {c.mhp} {c.msp} {hx(c.name)} {self.dist} "
                f"{self.res} {self.bar[0]} {self.bar[1]} {self.arrow[0]} {self.arrow[1]} {self.pp} {self.dead}")


class Scenario:
    def __init__(self, frames):
        self.frames = frames
        self.party = {}
        self.target = None
        self.sorted = []
        self.gamefs = {}
        self.game = (0, 0, 0, 0)
        self.dead = 0
        self.saves = []
        self.operate = 0
        self.pads = {}
        self.opens = {}
        self.bans = {}
        self.noises = {}          # frame -> interNoiz (the events' noise, before the menu task)
        self.movie = 0            # Data Drain's stream: frames from its start to its end
        self.msgs = {}
        self.infos = {}
        self.checks = []
        self.skills = {}          # frame -> ccSkillCheck(plw) from then on
        self.firsts = set()       # frames before which firstTime is set (the chat button)
        self.server = (0, 0, 0)   # ccGame.server, town, areaLevel
        self.area_codes = {}      # ccEvent.areaCode[k] = (story area, mode)
        self.chains = ([], [], [])  # Char lists: pc, enemy, object chains
        self.joiners = {}         # member id -> Char inviteSpc makes (AddMember)
        self.spc = {}             # member id -> spcParam (name, level, exp, money, hp, sp, class)
        self.area_item = None     # (field attr, event area, area level, item ofs, floor, field)
        self.watch = []           # (offset, length): save bytes compared after every frame
        self.extra = []           # probe lines a scenario of another test file adds
        self.area_words = None    # the area's keywords, SimGenerateCode'd into WORLD_MAN
        self.rng = 0              # rand()'s value (the menu's too)
        self.modes = {}           # frame -> ccMenu.mode set before it (ccThGameCtrl's)
        self.game_cnt = None      # ccGame.gameCnt[3] (+0x64)
        self.gim_acts = {}        # frame -> (handle, actNum): a gimmick's +0x1d4 (the spring's state)
        self.gones = {}           # frame -> handle: the character gone (ccCheckTarget 0)
        self.bgnum = 0            # WORLD_MAN::GetBG (+0xc)
        self.attacks = {}         # frame -> ccMenuCtrl.plAttack (+0xf4) set before it
        self.book = None          # (page, frame): a Ryu Book read from that frame (ccThBook)
        self.directs = {}         # frame -> pad direct (held) bits

    def spc_saves(self):
        """spcParam[id]'s level, exp, money, HP, SP and class as save writes."""
        out = []
        for i, (name, level, exp, money, hp, sp, cls) in sorted(self.spc.items()):
            at = 0x7488 + 0xDC * i
            out += [(at + 0x0E, 2, level), (at + 0x10, 2, exp), (at + 0x14, 4, money), (at + 0x24, 2, hp),
                    (at + 0x26, 2, sp), (at + 0xD8, 2, cls)]
        return out

    def lines(self):
        out = [f"save {0x8404 + 2 * i} 2 {v}" for i, v in enumerate(ASSIGN)]
        for slot, c in sorted(self.party.items()):
            out.append(c.party_line(slot))
        if self.target:
            out.append(self.target.target_line())
        for e in self.sorted:
            out.append(e.line())
        for f, g in sorted(self.gamefs.items()):
            out.append("gamef %d %d %d %d" % ((f,) + g))
        out.append("game %d %d %d %d" % self.game)
        out.append(f"dead {self.dead}")
        for off, size, v in self.saves + self.spc_saves():
            out.append(f"save {off} {size} {v}")
        out.append(f"operate {self.operate}")
        for f, (p, r) in sorted(self.pads.items()):
            out.append(f"pad {f} {p} {r}")
        for f, n in sorted(self.opens.items()):
            out.append(f"open {f} {n}")
        for f, on in sorted(self.bans.items()):
            out.append(f"ban {f} {int(on)}")
        for f, v in sorted(self.noises.items()):
            out.append(f"noise {f} {v}")
        if self.movie:
            out.append(f"movie {self.movie}")
        for e in self.sorted:
            if e.c.size is not None:
                out.append(f"size {e.c.handle} {e.c.size}")
        for f, (emode, name, ls) in sorted(self.msgs.items()):
            h = lambda b: "-" if b is None else hx(b)  # noqa: E731
            out.append(f"msg {f} {emode} {h(name)} {h(ls[0])} {h(ls[1])} {h(ls[2])}")
        for f, ls in sorted(self.infos.items()):
            h = lambda b: "-" if b is None else hx(b)  # noqa: E731
            out.append(f"info {f} {h(ls[0])} {h(ls[1])} {h(ls[2])}")
        for f in self.checks:
            out.append(f"check {f}")
        seen = set()
        for c in list(self.party.values()) + ([self.target] if self.target else []) + [e.c for e in self.sorted] + \
                [c for ch in self.chains for c in ch]:
            if c.handle not in seen:
                seen.add(c.handle)
                out.append(c.pos_line())
        known = {c.handle for c in list(self.party.values()) + [e.c for e in self.sorted]}
        if self.target:
            known.add(self.target.handle)
        for ch in self.chains:
            for c in ch:
                if c.handle not in known:
                    known.add(c.handle)
                    out.append(f"char {c.handle} {c.types} {c.id} {c.hp} {c.sp} {c.mhp} {c.msp} {hx(c.name)}")
        for c in self.joiners.values():
            if c.handle not in known:
                known.add(c.handle)
                out.append(f"char {c.handle} {c.types} {c.id} {c.hp} {c.sp} {c.mhp} {c.msp} {hx(c.name)}")
        for k, ch in enumerate(self.chains):
            if ch:
                out.append(f"chain {k} " + " ".join(str(c.handle) for c in ch))
        for f, v in sorted(self.skills.items()):
            out.append(f"skill {f} {v}")
        for c in self.party.values():
            if any(c.chat):
                out.append(f"chat {c.handle} " + " ".join(str(v) for v in c.chat))
            if c.busy:
                out.append(f"busy {c.handle} {c.busy[0]} {c.busy[1]}")
        for f in sorted(self.firsts):
            out.append(f"first {f}")
        out.append("server %d %d %d" % self.server)
        seen_items = set()
        for c in list(self.party.values()) + ([self.target] if self.target else []) + [e.c for e in self.sorted] + \
                [c for ch in self.chains for c in ch] + list(self.joiners.values()):
            if c.item is not None and c.handle not in seen_items:
                seen_items.add(c.handle)
                out.append(f"item {c.handle} {c.item}")
                if c.trap is not None:
                    out.append(f"trap {c.handle} {c.trap[0]} {c.trap[1]}")
        for c in ([self.target] if self.target else []) + [e.c for e in self.sorted]:
            if c.exdef is not None:
                out.append(f"exdef {c.handle} {c.exdef[0]} {c.exdef[1]}")
        if self.area_item:
            out.append("areaitem %d %d %d %d %d %d" % self.area_item)
        for k, (code, mode) in sorted(self.area_codes.items()):
            out.append(f"areacode {k} {code} {mode}")
        for off, n in self.watch:
            out.append(f"watch {off} {n}")
        if self.area_words:
            out.append("areawords %d %d %d" % self.area_words)
        if self.rng:
            out.append(f"rng {self.rng}")
        for f, v in sorted(self.modes.items()):
            out.append(f"mode {f} {v}")
        for f, v in sorted(self.attacks.items()):
            out.append(f"attack {f} {v}")
        for f, v in sorted(self.directs.items()):
            out.append(f"direct {f} {v}")
        if self.book:
            out.append("book %d %d" % self.book)
        if self.game_cnt:
            out.append("gamecnt %d %d %d" % self.game_cnt)
        for f, (h, st) in sorted(self.gim_acts.items()):
            out.append(f"gimact {f} {h} {st}")
        for f, h in sorted(self.gones.items()):
            out.append(f"gone {f} {h}")
        if self.bgnum:
            out.append(f"bgnum {self.bgnum}")
        out.extend(self.extra)
        out.append(f"run {self.frames}")
        return out


def port(sc):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(sc.lines()) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class Game:
    """The game's ccThMenu in eemu over a scenario."""

    def __init__(self, sc):
        from test_anim import machine_class
        self.sc = sc
        m = self.m = machine_class()(P)
        self.heap = HEAP
        self.strs = STRS
        self.frame = 0
        self.log = []
        self.kanji_text = {}
        self.kanji_used = set()
        self.checking = False
        self.stop_at = sc.frames
        self.objs = {}
        self.last = 0
        self.skill_now = 0
        self.setup()

    # --- memory ----------------------------------------------------------------
    def alloc(self, n):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        assert self.heap < HEAP_END
        self.m.mem[a:a + n] = bytes(n)
        return a

    def put(self, b):
        a = self.strs
        self.m.mem[a:a + len(b) + 1] = b + b"\0"
        self.strs = (a + len(b) + 16) & ~15
        return a

    def call(self, addr, args=(), fargs=()):
        """A call made from inside a hook: `call` starts a fresh stack at
        STACK_TOP, under the interrupted call's own frames, so those frames
        and every register are put back afterwards."""
        m = self.m
        saved = (list(m.r), list(m.f), m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps)
        vu = [list(v) for v in m.vf] if hasattr(m, "vf") else None
        top = eemu.STACK_TOP
        stack = bytes(m.mem[top - 0x10000:top])
        for i, v in enumerate(fargs):
            m.f[12 + i] = v
        try:
            r = m.call(addr, list(args), limit=50_000_000)
        finally:
            m.mem[top - 0x10000:top] = stack
            (m.r, m.f, m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps) = saved
            if vu is not None:
                m.vf = vu
        return r

    def ev(self, *e):
        self.log.append((self.frame, e))

    # --- set-up ----------------------------------------------------------------
    def setup(self):
        m, sc = self.m, self.sc
        self.threads = []
        self.ais = {}
        self.manual_off = set()
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 0x25C, 4, SCRATCH)
        m.store(SYS + 0x358, 4, 0)
        m.store(SAVEDATA, 4, SAVE)
        # The port's fresh save, made by the game's own boot (the player
        # named Kite, Init's buttons, lists and options), then the
        # scenario's.
        m.mem[SAVE:SAVE + 0x8530] = test_save_init_rs.fresh_save(ELF)
        for off, size, v in sc.saves + sc.spc_saves():
            m.store(SAVE + off, size, v)
        for i, spc in sc.spc.items():
            m.store(SAVE + 0x7488 + 0xDC * i, 4, self.put(spc[0]))
        self.threads = []
        # ccInitRegisterEnemy's range, which Mutation's ccAnalyzeEnemyList
        # reads (the keyword screen's level).
        m.store(sym("ccRegisterEnemyRange"), 4, 3)
        m.store(GAME_P, 4, GAME)
        area, inb, inbc, dtype = sc.game
        m.store(GAME + 0x00, 4, 5)
        m.store(GAME + 0x14, 4, area)
        m.store(GAME + 0x58, 4, inb)
        m.store(GAME + 0x5C, 4, inbc)
        m.store(GAME + 0x80, 4, self.alloc(0x40))
        m.store(GAME + 0x1C, 4, sc.server[0])
        m.store(GAME + 0x20, 4, sc.server[1])
        m.store(GAME + 0x78, 4, sc.server[2])
        for k, v in enumerate(sc.game_cnt or ()):
            m.store(GAME + 0x64 + 4 * k, 4, v)
        self.dungeon_type = dtype
        m.store(WORLDMAN, 4, WM)
        m.store(EVENTMNG, 4, EV)
        m.store(EV + 0x08, 4, 0xFFFFFFFF)                 # bossEntry
        m.store(EV + 0x770, 8, sc.operate)
        m.store(EV + 0x778, 2, 0xFFFF)
        # ccEvent.areaCode[16] (free: -1, -1) and areaCodeSet[3].
        for k in range(16):
            code, mode = sc.area_codes.get(k, (-1, -1))
            m.store(EV + 0x180 + 4 * k, 2, code & 0xFFFF)
            m.store(EV + 0x182 + 4 * k, 2, mode & 0xFFFF)
        for k in range(3):
            m.store(EV + 0x77A + 2 * k, 2, 0xFFFF)
        # WORLD_MAN's merged WORDPARAM (+0x158); bgnum (+0xc).
        wp = self.alloc(0x30)
        m.store(WM + 0x158, 4, wp)
        m.store(WM + 0xC, 4, sc.bgnum)
        if sc.area_item:
            fa, ev, level, ofs, floor, field = sc.area_item
            m.store(WM + 0x120, 4, ev)
            m.store(wp + 0x20, 4, level)
            m.store(wp + 0x28, 4, ofs)
            m.store(GAME + 0x2C, 4, floor)
            m.store(GAME + 0x24, 4, field)
        if sc.area_words:
            # The area as WORLD_MAN holds it (Area Information shows it).
            self.call(sym("SimGenerateCode__9WORLD_MANFiii"), [WM] + list(sc.area_words))
        m.store(PLW_PW, 4, PLAYER)
        m.store(FONTTEX, 4, self.alloc(0xD0))
        m.store(FONTDEF, 4, self.alloc(0xD0))
        # The global font (a ccFont, as ccSprite builds it).
        fnt = self.alloc(0xD0)
        self.call(sym("__ct__8ccSpriteFv"), [fnt])
        m.store(FONT, 4, fnt)
        self.hack_threads, self.anms, self.hack_mask, self.rotz = set(), {}, None, 0
        self.drain_rec = {}
        # The party (ccPartyManager: memberChar[3], memberID[3], num).
        self.chars = {}
        self.rows = {}
        n = 0
        for slot in range(3):
            m.store(PARTY + 0x0C + 4 * slot, 4, 0xFFFFFFFF)
        for slot, c in sorted(sc.party.items()):
            a = self.make_char(c)
            m.store(PARTY + 4 * slot, 4, a)
            m.store(PARTY + 0x0C + 4 * slot, 4, c.id)
            n += 1
            if slot == 0:
                # plw.pw: the player is party slot 0.
                m.store(PLW_PW, 4, a)
        m.store(PARTY + 0x18, 4, n)
        # plw.pw->condition.dead
        m.store(m.load(PLW_PW, 4) + 8, 2, sc.dead)
        if sc.target:
            m.store(CMNDTARGET, 4, self.make_char(sc.target))
        # cmndSortRoot's chain (+0xc0 cmndSort), with cmndDist (+0xc4) and
        # the protect count (personality +98).
        self.enemies = {}
        prev = None
        for e in sc.sorted:
            a = self.make_char(e.c)
            m.store(a + 0x08, 2, e.dead)
            m.store(a + 0xC4, 4, fb(e.dist))
            m.store(m.load(a + 4, 4) + 98, 2, e.pp)
            self.enemies[a] = e
            if prev is None:
                m.store(CMNDSORTROOT, 4, a)
            else:
                m.store(prev + 0xC0, 4, a)
            prev = a
        # cmndPcRoot / EneRoot / ObjRoot, chained by cmndLink (+0xbc).
        for k, chain in enumerate(sc.chains):
            prev = None
            for c in chain:
                a = self.addr_of(c)
                if prev is None:
                    m.store(CMNDROOTS[k], 4, a)
                else:
                    m.store(prev + 0xBC, 4, a)
                prev = a
        cam = self.alloc(0x70)
        m.store(inf_va(0x0037896C), 4, cam)                        # activeCamPtr, rot.z 0
        for e in sc.sorted:
            if e.res == 0:
                e.arrow = self.arrow(e.dirc)
        self.hooks()

    def arrow(self, d):
        """The off-screen arrow's point for direction d (camera turned 0):
        fptosi(256 - 384 sinf(d)), fptosi(256 - 384 cosf(d))."""
        f_to_int = eemu.f_to_int
        out = []
        for fn in ("sinf", "cosf"):
            self.m.f[12] = fb(d)
            self.m.call(sym(fn), [])
            s = self.m.f[0] & 0xFFFFFFFF
            out.append(f_to_int(f_add(fb(256.0), f_mul(fb(-384.0), s))))
        return tuple(out)

    def addr_of(self, c):
        """The ccChar made for `c`, made now if it was not."""
        for a, x in self.chars.items():
            if x is c:
                return a
        return self.make_char(c)

    def make_char(self, c):
        m = self.m
        base = self.alloc(0x24)
        name = self.put(c.name)
        m.store(base + 0x00, 4, name)
        m.store(base + 0x08, 4, c.types)
        m.store(base + 0x0C, 2, c.id)
        m.store(base + 0x18, 4, fb(160.0))
        m.store(base + 0x1C, 4, fb(c.width))
        a = self.alloc(0x200)
        # pos: x names the character for ccCheckCameraDeg's hook; posP.
        m.store(a + 0x40, 4, fb(float(c.handle)))
        for k, v in enumerate(c.pos):
            m.store(a + 0x50 + 4 * k, 4, fb(v))
        m.store(a + 0x5C, 4, fb(1.0))
        m.store(a + 0x00, 4, base)
        pers = self.alloc(0x100)
        m.store(a + 0x04, 4, pers)                        # personality
        for k, v in enumerate(c.chat):
            m.store(pers + 200 + 2 * k, 2, v & 0xFFFF)
        for k, v in enumerate(c.cond):
            m.store(a + 0x08 + 2 * k, 2, v)
        m.store(a + 0x70, 2, c.hp)
        m.store(a + 0x72, 2, c.sp)
        m.store(a + 0x74, 2, c.mhp)
        m.store(a + 0x76, 2, c.msp)
        if c.item is not None:
            m.store(a + 0x14C, 4, c.item & 0xFFFFFFFF)
        if c.trap is not None:
            m.store(a + 0x150, 4, c.trap[0] & 0xFFFFFFFF)
            m.store(a + 0x7C, 2, c.trap[1] & 0xFFFF)
        if c.busy:
            m.store(a + 0x7C, 2, c.busy[0] & 0xFFFF)
            m.store(a + 0xEE, 2, c.busy[1] & 0xFFFF)
        if c.exdef is not None:
            # ccGetEnemyParam / ccGetBossParam(id): the row's elm (+0x28)
            # all 100 and its Exdefense; ccEnemyParam.real (personality +0)
            # 50 where a defence is lowered.
            ex, lowered = c.exdef
            row = self.alloc(0x100)
            m.store(row + 0x64, 2, ex & 0xFFFF)
            for i in range(14):
                m.store(row + 0x28 + 2 * i, 2, 100)
                m.store(pers + 2 * i, 2, 100)
            for bit, i in EXDEF:
                if lowered & bit:
                    m.store(pers + 2 * i, 2, 50)
            self.rows[c.id] = row
        # personality->ai (+296): manualSW set.
        ai = self.alloc(0x260)
        m.store(a + 296, 4, ai)
        m.store(ai, 1, 1)
        self.ais[ai] = c.handle
        # spcParam[id].base.name, which the panels measure.
        if 0 <= c.id < 18:
            m.store(SAVE + 0x7488 + 0xDC * c.id, 4, name)
        self.chars[a] = c
        return a

    def handle(self, a):
        c = self.chars.get(a)
        return c.handle if c else 0

    def hooks(self):
        m = self.m
        h = m.hooks

        def hook(name, fn):
            h[sym(name)] = fn

        nop = lambda mm, *a: 0  # noqa: E731
        hook("__nw__FUi", lambda mm, n, *a: self.alloc(n))
        hook("__ct__16ccDrawPacketCtrlFv", lambda mm, a0, *a: a0)
        hook("Init__7ccLayerFsP6ccView", lambda mm, this, *a: mm.store(this + 0x2C, 4, self.alloc(0x100)) or 0)
        hook("SetFrame__6ccViewFffffffff", nop)
        def set_tex(mm, this, f, t, *a):
            if bytes(mm.mem[t:t + 16]).split(b"\0")[0] == b"TEX_xdhroma1":
                self.hack_mask = this
            return 0
        hook("SetTex__8ccSpriteFPcPci", set_tex)

        def kanji_init(mm, this, l, n, *a):
            mm.store(this + 0x24, 4, n)
            mm.store(this + 0xD4, 2, 0)
            return 0
        hook("Init__7ccKanjiFii", kanji_init)
        hook("__ct__6ccNoizFv", lambda mm, this, *a: mm.store(this + 0x10, 4, self.alloc(0x40)) or this)
        hook("__ct__8ccScFadeFv", lambda mm, this, *a: this)
        def set_noiz(mm, this, a, b, c, *x):
            if own_call(mm.r[31]):
                self.ev("noiz", sx32(a), sx32(b), sx32(c))
                if a or b or c:
                    self.ev("se", 94)
                if DRAIN[0] <= mm.r[31] < DRAIN[1] and (a, b, c) == (8, 8, 8):
                    # Step 10's end: the side effect as the rules left it.
                    lst, men = inf_va(0x0072EFE0) + 32 * 66, self.menu()
                    side = (mm.load(lst + 0x10, 2, True), mm.load(lst + 0xC, 2, True),
                                              sx32(mm.load(men + volume.menu_at(0x23C), 4)))
                    # The party's HP and SP as the side effect left them.
                    hs = []
                    for slot in range(3):
                        a = mm.load(PARTY + 4 * slot, 4)
                        hs.append((self.handle(a), mm.load(a + 0x70, 2, True), mm.load(a + 0x72, 2, True)) if a
                                  else (0, 0, 0))
                    self.drain_rec.setdefault("side", []).append((side, hs))
            return 0
        hook("SetNoiz__6ccNoizFiii", set_noiz)
        hook("cameraShake__Fiiii", lambda mm, a, b, c, d, *x:
             (own_call(mm.r[31]) and self.ev("shake", sx32(a), sx32(b), sx32(c), sx32(d))) or 0)
        for fn, tag in (("SetNoizRn__6ccNoizFi", "noiz_rn"), ("SetNoizBs__6ccNoizFi", "noiz_bs"),
                        ("SetNoizBr__6ccNoizFi", "noiz_br")):
            hook(fn, (lambda t: lambda mm, this, v, *a: (own_call(mm.r[31]) and self.ev(t, sx32(v))) or 0)(tag))
        for name in ("Draw__6ccNoizFv", "Trans__8ccSpriteFv",
                     "Disp__9ccChatMsgFi", "ccCtrlFlyFont__Fi", "CtrlAll__11ccDamUprStrFv",
                     "DrawAll__11ccDamUprStrFv", "OffFlipExcept__7ccLayerFv", "OnFlipExcept__7ccLayerFv"):
            hook(name, nop)
        def fade_send(mm, this, *a):
            # ccScFade::SendPacket (main 0x0015fb80) of the menu's own fader
            # (menuFade, +0xb8): each drawing element logged, then counted
            # on as the function does (the drawing itself is not compared).
            c = self.menu()
            if not c or this != mm.load(c + 0xB8, 4):
                return 0
            for k in range(4):
                e = this + 4 + 36 * k
                st = mm.load(e, 2, True)
                if not st & 1:
                    continue
                cnt, tcnt = mm.load(e + 2, 2, True), mm.load(e + 4, 2, True)
                c0, c1 = mm.load(e + 0x1C, 4), mm.load(e + 0x20, 4)
                self.ev("fade", c0, c1, cnt, tcnt)
                cnt += 1
                if cnt > tcnt:
                    if st & 2:
                        st = 0
                    elif st & 4:
                        st, cnt, tcnt = (st & ~4) | 2, 0, mm.load(e + 6, 2, True)
                        c0, c1 = c1, c1 & 0xFFFFFF
                    elif st & 8:
                        st, cnt, tcnt = (st & ~8) | 4, 0, mm.load(e + 8, 2, True)
                        c0 = c1
                    else:
                        cnt = tcnt
                mm.store(e, 2, st & 0xFFFF)
                mm.store(e + 2, 2, cnt & 0xFFFF)
                mm.store(e + 4, 2, tcnt & 0xFFFF)
                mm.store(e + 0x1C, 4, c0)
                mm.store(e + 0x20, 4, c1)
            return 0
        hook("SendPacket__8ccScFadeFv", fade_send)
        hook("GetFrameRate__8ccSystemFv", lambda mm, *a: 2)
        hook("rand", lambda mm, *a: self.sc.rng)
        hook("Breath__6ccTscbFi", lambda mm, tscb, n, *a: self.breath(n))
        hook("ccBreathThread__Fi", lambda mm, n, *a: self.breath(n))
        hook("ccSeOn__Fi", lambda mm, n, *a: self.ev("se", n) or 0)
        hook("ccSleepAllThread__Fv", lambda mm, *a: self.ev("sleep") or 0)
        hook("ccWakeAllThread__Fv", lambda mm, *a: self.ev("wake") or 0)
        hook("fontOffFlip__Fv", lambda mm, *a: self.ev("still", 1) or 0)
        hook("fontOnFlip__Fv", lambda mm, *a: self.ev("still", 0) or 0)
        hook("ccEvVoiceStop__Fv", lambda mm, *a: self.ev("voice_stop") or 0)
        hook("ccEvVoiceRequest__Fii", lambda mm, g, n, *a: self.ev("voice", sx32(g), sx32(n)) or 0)
        def change_target(mm, c, *a):
            # ccChangeCmndTarget (gcmn 0x005198c0) as the game has it.
            cur = mm.load(CMNDTARGET, 4)
            if c != cur:
                mm.store(CMNDTARGETPREV, 4, cur)
                mm.store(CMNDTARGET, 4, c if c in self.chars else 0)
            self.ev("target", self.handle(c))
            return 0
        hook("ccChangeCmndTarget__FP6ccChar", change_target)
        hook("ccSkillRequest__FP6ccCharP6ccChari",
             lambda mm, u, t, n, *a: self.ev("skill", self.handle(t), n - (1 << 32) if n & 0x80000000 else n) or 0)
        hook("ccSpcMessageOpenTreasureBox__Fv", lambda mm, *a: self.ev("spc_msg_box") or 0)
        hook("BreakSomething__8ccPlayerFP6ccChar", lambda mm, *a: self.ev("break_something") or 0)
        hook("AttackCancel__8ccPlayerFv", lambda mm, *a: self.ev("attack_cancel") or 0)
        hook("ccUseItemRequest__FP6ccCharP6ccCharii",
             lambda mm, u, t, code, *a: self.ev("use_item", self.handle(t), code) or 0)
        def affect(mm, c, who, kind, *a):
            kind = kind & 0xFFFF
            self.ev("affect", self.handle(c), kind - 0x10000 if kind & 0x8000 else kind)
            return 0
        hook("EntryAffect__6ccCharFP6ccCharssss", affect)
        def entry_flash(mm, this, t, c, *a):
            # The gate's white flash is the runtime's (Request::Flash); the
            # gate hack's go on menuFade as EntryFlash (0x00160240) has it.
            if not own_call(mm.r[31]):
                self.ev("flash")
                return 0
            if DRAIN[0] <= mm.r[31] < DRAIN[1] and c == 0x70C0A020:
                # Step 0's flash, just after the rules: what they left.
                men = self.menu()
                self.drain_rec.setdefault("drops", []).append(
                    (mm.load(SAVE + 0x676E, 2, True), sx32(mm.load(men + volume.menu_at(0x1F0), 4)),
                     [sx32(mm.load(men + volume.menu_at(0x1AC) + 4 * k, 4)) for k in range(17)]))
            for k in range(4):
                e = this + 4 + 36 * k
                if mm.load(e, 2, True) == 0:
                    mm.store(e, 2, 3)
                    mm.store(e + 4, 2, t & 0xFFFF)
                    mm.store(e + 2, 2, 0)
                    mm.store(e + 0x1C, 4, c)
                    mm.store(e + 0x20, 4, c & 0xFFFFFF)
                    return k
            return 0xFFFFFFFF
        hook("EntryFlash__8ccScFadeFiiffff", entry_flash)
        self.hack_hooks(hook, nop)
        hook("TransferOut__9ccSpcCharFv", lambda mm, c, *a: self.ev("transfer_out", self.handle(c)) or 0)
        hook("DeleteNoPartyMember__5ccSPCFv", lambda mm, *a: self.ev("delete_no_party") or 0)

        # ccStartThread: a tscb; ccThPartyAdd's body runs as the menu
        # breathes (AddMember, then param[0] = 0).
        def start_thread(mm, fn, *a):
            t = self.alloc(0x60)
            self.threads.append((t, fn))
            # Data Drain's movie: a stream task whose param (the stream,
            # written by the caller) goes to -1 sc.movie frames on; the
            # enemy's task (its target written by the caller).
            if fn == sym("ccThExecuteStream__FP6ccTscb"):
                self.movie = [t, self.frame, False]
            if fn == sym("ccThDrainEnemy__FPv"):
                self.drain_enemy = [t, False]
            if fn == sym("ccThGtHackMenu__FP6ccTscb"):
                self.hack_threads.add(t)
                self.ev("start_thread", "hack")
            return t
        hook("ccStartThread__FPFPv_vii", start_thread)

        def stream_adrs(mm, *a):
            # A stream already playing (+0x17e bit 3).
            if not getattr(self, "fake_stream", 0):
                self.fake_stream = self.alloc(0x200)
                mm.store(self.fake_stream + 0x17E, 2, 8)
            return self.fake_stream
        hook("ccGetStreamAdrs__Fv", stream_adrs)

        def stream_frame(mm, *a):
            d = getattr(self, "drain_enemy", None)
            if d and not d[1]:
                d[1] = True
                self.ev("drain_enemy", self.handle(mm.load(d[0] + 0x14, 4)))
            return self.frame - self.movie[1] if getattr(self, "movie", None) else 0
        hook("ccGetStreamFrame__Fv", stream_frame)
        hook("ccGetStreamNum__Fv", lambda mm, *a: 0)
        hook("ccCheckObjectSize__FP6ccChar",
             lambda mm, c, *a: (self.chars[c].size if c in self.chars and self.chars[c].size is not None else 4))

        def delete_thread(mm, t, *a):
            if t in self.hack_threads:
                self.ev("delete_thread")
            return 0
        hook("ccDeleteThread__FP6ccTscb", delete_thread)

        def invite(mm, cid, *a):
            self.ev("add_member", cid)
            c = self.sc.joiners.get(cid)
            return self.make_char(c) if c else 0
        hook("inviteSpc__Fi", invite)
        hook("SetGenerateCode__9WORLD_MANFiii",
             lambda mm, w, a, b, c, *x: self.ev("goto", a - (1 << 32) if a & 0x80000000 else a,
                                                b - (1 << 32) if b & 0x80000000 else b,
                                                c - (1 << 32) if c & 0x80000000 else c) or 0)
        hook("ChangeArea__6ccGameFii", lambda mm, g, area, n, *a: self.ev("change_area", area, n) or 0)
        hook("RequestChatCmd__9ccSpcCharFiP6ccChari",
             lambda mm, c, cmd, tp, sid, *a: self.ev("chat_cmd", self.handle(c), cmd - (1 << 32) if cmd & 0x80000000
                                                     else cmd, self.handle(tp), sx32(sid)) or 0)
        # FountainMenu3's hold and release of the party: the port's one
        # request (fountain_party) at ccSpcConditionEffectOFF / ON, the
        # member calls around them its runtime's.
        fountain3 = volume.span("FountainMenu3__10ccMenuCtrlFv")
        in_f3 = lambda ra: fountain3[0] <= ra < fountain3[1]  # noqa: E731
        hook("ManualModeAI__9ccSpcCharFi",
             lambda mm, c, e, *a: (in_f3(mm.r[31]) or self.ev("manual_ai", self.handle(c), e)) and 0 or 0)
        hook("SetRemoteCmd__4ccAIFi",
             lambda mm, ai, cmd, *a: (in_f3(mm.r[31]) or self.ev("remote_cmd", self.ais.get(ai, 0), cmd)) and 0 or 0)
        hook("ccSpcConditionEffectOFF__Fv", lambda mm, *a: self.ev("fountain_party", 1) or 0)
        def fountain_release(mm, *a):
            # The loop before it cleared every member's manualSW: part of
            # the one request, not a manual_off of its own.
            for ai in self.ais:
                if not self.m.load(ai, 1) & 1:
                    self.manual_off.add(ai)
            self.ev("fountain_party", 0)
            return 0
        hook("ccSpcConditionEffectON__Fv", fountain_release)
        for name in ("ccStoreSpcCondition__Fv", "ClearCondition__6ccCharFP10ccSpcParam", "ccClearConditionAllEnemy__Fv",
                     "ccEntryCmnd__FP6ccChar", "ccRestoreSpcCondition__FP6ccChar", "ConditionAdjustment__9ccSpcCharFv"):
            hook(name, nop)
        hook("SetFountainCamera__10ccMenuCtrlFv", lambda mm, *a: self.ev("fountain_camera") or 0)
        hook("changeCamera__Fi", lambda mm, n, *a: self.ev("camera", sx32(n)) or 0)
        hook("ChangeEquipReport__9ccSpcCharFi",
             lambda mm, c, n, *a: self.ev("equip_report", self.handle(c), n - (1 << 32) if n & 0x80000000 else n) or 0)

        def open_chat(mm, chat, who, text, *a):
            self.ev("open_chat", bytes(mm.mem[text:text + 200]).split(b"\0")[0].hex())
            return 0
        hook("OpenChat__9ccChatMsgFP6ccCharPc", open_chat)

        def camera_deg(mm, v, deg, *a):
            h = int(f_to_py(mm.load(v, 4)))
            c = next((x for x in self.chars.values() if x.handle == h), None)
            return 1 if c is not None and c.view else 0
        hook("ccCheckCameraDeg__FPfs", camera_deg)
        hook("SetMapAlpha__9WORLD_MANFf", lambda mm, *a: self.ev("map_alpha", f_to_py(mm.f[12])) or 0)
        hook("GetDungeonType__9WORLD_MANFv", lambda mm, *a: self.dungeon_type)
        if self.sc.area_item:
            # The area's element as the scenario gives it (the gate's pages
            # compute theirs from the words).
            hook("GetFieldAttrb__9WORLD_MANFv", lambda mm, *a: self.sc.area_item[0])
        hook("ccSkillCheck__FP6ccChar", lambda mm, *a: self.skill_now)

        # The menus the port has no handler for close at once there
        # (Request::Unported); here too.
        def unported(n):
            def fn(mm, this, *a):
                self.ev("unported", n)
                self.call(sym("CloseMenu__10ccMenuCtrlFv"), [this])
                return 0
            return fn
        for n, name in UNPORTED:
            hook(name + "__10ccMenuCtrlFv", unported(n))
        hook("CheckCharAttribute__6ccCharFi", lambda mm, *a: 0xFFFFFFFF)
        hook("ccGetEnemyParam__Fi", lambda mm, cid, *a: self.rows.get(cid) or self.alloc(0x100))
        hook("ccGetBossParam__Fi", lambda mm, cid, *a: self.rows.get(cid) or self.alloc(0x100))

        def check_target(mm, c, *a):
            return 1 if c in self.chars else 0
        hook("ccCheckTarget__FP6ccChar", check_target)

        def tag_pos_char(mm, c, pos, off, mode, *a):
            self.last = c
            if mode == 1 and c in self.enemies:
                e = self.enemies[c]
                mm.store(pos, 4, e.bar[0])
                mm.store(pos + 4, 4, e.bar[1])
                return e.res
            ch = self.chars.get(c)
            if ch is None or ch.tag is None:
                return 0
            mm.store(pos, 4, ch.tag[0])
            mm.store(pos + 4, 4, ch.tag[1])
            return 1
        hook("ccCalcTagPosChar__FP6ccCharRA4_iPfi", tag_pos_char)

        def tag_pos(mm, v, pos, off, *a):
            e = self.enemies.get(self.last)
            if e is not None:
                mm.store(pos, 4, e.bar[0])
                mm.store(pos + 4, 4, e.bar[1])
            return 0
        hook("ccCalcTagPos__FRA4_fRA4_iPf", tag_pos)

        def get_dirc(mm, a, b, *x):
            e = self.enemies.get(self.last)
            mm.f[0] = fb(e.dirc if e else 0.0)
            return 0
        hook("ccGetDirc__FPfPf", get_dirc)

        # ccMessage's constructor stops on the interrupt code of its kanji
        # buffers; built here as it builds itself.
        def msg_ct(mm, this, *a):
            win = self.alloc(0x1B0)
            self.call(sym("__ct__12ccMenuWindowFv"), [win])
            mm.store(this, 4, win)
            for i in range(4):
                k = self.alloc(0xE8)
                self.call(sym("__ct__8ccSpriteFv"), [k])
                mm.store(k + 0x24, 4, 16)
                mm.store(k + 0xD4, 2, 2)
                mm.store(this + 4 + 4 * i, 4, k)
            mm.store(this + 0x20, 2, 7)
            return this
        hook("__ct__9ccMessageFv", msg_ct)

        def chat_ct(mm, this, *a):
            win = self.alloc(0xD0)
            self.call(sym("__ct__8ccSpriteFv"), [win])
            mm.store(this, 4, win)
            return this
        hook("__ct__9ccChatMsgFv", chat_ct)

        def rd(o):
            return mm_.load(o, 4)
        mm_ = m

        def advance(this, n):
            dx, sx = m.load(this + 0x40, 4), m.load(this + 0x48, 4)
            for _ in range(n):
                dx = f_add(dx, sx)
            m.store(this + 0x40, 4, dx)

        def rec(this, code, text):
            f = lambda o: round(f_to_py(m.load(this + o, 4)), 4)  # noqa: E731
            i = lambda o: m.load(this + o, 4, True)  # noqa: E731
            rgba = [m.load(this + 0x68 + 4 * k, 4) & 0xFF for k in range(4)]
            ctrl = m.load(this + 4, 4)
            # ctrl 0x20, 0x10 and 0x40 (mirrored, shadowed, upside down) as bits 0, 1, 2.
            flip = (1 if ctrl & 0x20 else 0) | (2 if ctrl & 0x10 else 0) | (4 if ctrl & 0x40 else 0)
            return (code, text, f(0x40), f(0x44), f(0x48), f(0x4C), f(0x34), f(0x38), f(0x3C),
                    i(0x50), i(0x54), i(0x58), i(0x5C), i(0x60), rgba, flip)

        def make_packet(mm, this, code, t, *a):
            code &= 0xFF
            if code != 0xFF:
                r = rec(this, code, None)
                c = self.menu()
                if c and this == mm.load(c + 0xDC, 4):
                    # menuDrain is Gouraud: its four corners.
                    r = r + ([[mm.load(this + 0x68 + 16 * v + 4 * k, 4) & 0xFF for k in range(4)] for v in range(4)],)
                self.ev("pk", this, r)
                self.kanji_used.add(this)
            advance(this, 1)
            return 0
        hook("MakePacket__8ccSpriteFii", make_packet)

        def make_str(mm, this, s, mode, *a):
            b = bytes(mm.mem[s:s + 128])
            b = b.split(b"\xff")[0] if mode else b.split(b"\0")[0]
            self.ev("pk", this, rec(this, 0, b.hex() or "="))
            advance(this, len(b))
            return 0
        hook("MakePacketStr__8ccSpriteFPci", make_str)

        def extract(mm, this, s, *a):
            b = bytes(mm.mem[s:s + 400]).split(b"\0")[0]
            self.kanji_text[this] = b
            return 0
        hook("Extract__7ccKanjiFPc", extract)

        def send(mm, this, *a):
            if this in self.kanji_used and this in self.kanji_text:
                self.ev("kanji", this, self.kanji_text[this])
            self.kanji_used.discard(this)
            return 0
        hook("SendPacket__8ccSpriteFv", send)

        def kdisp(mm, obj, s, c, *a):
            b = bytes(mm.mem[s:s + 200]).split(b"\0")[0]
            rgba = [mm.load(obj + 0x68 + 4 * k, 4) & 0xFF for k in range(4)]
            c = c - (1 << 32) if c & 0x80000000 else c
            self.ev("mt", obj, b, c, round(f_to_py(mm.load(obj + 0x40, 4)), 4),
                    round(f_to_py(mm.load(obj + 0x44, 4)), 4), rgba)
            return 0
        hook("Disp__7ccKanjiFPciff", kdisp)

    def hack_hooks(self, hook, nop):
        """ccHackMenu's animations, faked: each ccAnm plays by its length
        (as ccAnm::_AnimateForward ends a play-once one), its objects are
        blocks whose dispSW (+0xa2) the menu writes, and each Draw is
        logged (the crystals' 48 switches and the ring's turn)."""
        m = self.m
        hook("GetCCSAdrs__8ccStreamFPCc", lambda mm, *a: 0x1000)
        # Data Drain's effects and fly fonts are the runtime's; the level
        # lost is logged.
        for name in ("effSkillStartEffect__FP6ccCharii", "effHeal__FP6ccChari", "effAfterDrain__FP6ccChari",
                     "ccEntryFlyFontNewMiss__FPfP6ccChar", "ccEntryFlyFontNewExp__FiiPfP6ccChar",
                     "ccEntryFlyFontNewLevelDown__FiPfP6ccChar"):
            hook(name, nop)
        hook("LevelDown__6ccCharFv", lambda mm, *a: self.ev("level_down") or 0)
        hook("ccFileListDeleteOne__FP10ccFileList", nop)
        hook("CloseChat__9ccChatMsgFv", lambda mm, *a: self.ev("close_chat") or 0)
        hook("ccSndGateHack__Fi", lambda mm, n, *a: self.ev("gate_hack_snd", n) or 0)
        hook("ccSetGtHack__Fv", lambda mm, *a: self.ev("set_gt_hack") or 0)
        hook("SetView__6ccViewFRC5ccCamPA4_f", nop)
        for name in ("__dt__8ccSpriteFv", "__dt__5ccAnmFv", "__dt__7ccLayerFv", "__dl__FPv", "__dla__FPv",
                     "SetFogSw__5ccAnmFi"):
            hook(name, nop)

        def rot_z(mm, *a):
            self.rotz = mm.f[12] & 0xFFFFFFFF
            return 0
        hook("sceVu0RotMatrixZ", rot_z)

        def anm_ct(mm, this, *a):
            self.anms[this] = {"label": None, "name": None, "t": 0, "objs": {}}
            return this
        hook("__ct__5ccAnmFv", anm_ct)

        def set_anm(mm, this, ccs, name, *a):
            a_ = self.anms[this]
            n = bytes(mm.mem[name:name + 32]).split(b"\0")[0].decode()
            if a_["label"] is None:
                a_["label"] = HACK_LABELS.get(n, n)
            a_["name"], a_["t"] = n, 0
            mm.store(this + 0xAC, 4, 1)
            mm.store(this + 0x9C, 2, 256)
            self.ev("anm_set", a_["label"], n)
            return 0
        hook("SetAnm__5ccAnmFP8ccStreamPcUi", set_anm)

        def forward(mm, this, step, *a):
            # piney_data::anim::Animation::forward at 256 a step.
            a_ = self.anms[this]
            self.ev("anm_fwd", a_["label"])
            frames, looping = HACK_ANMS[a_["name"]]
            last = (frames - 1) * 256
            t = a_["t"]
            new, ended = t + step, False
            if last < new:
                step, new, ended = step - (new - last), last, True
            if new >> 8 != t >> 8:
                ended = False
                if new >> 8 >= frames - 1:
                    if looping:
                        new = 0
                    else:
                        ended = True
            a_["t"] = new
            return 1 if ended else 0
        hook("_AnimateForward__5ccAnmFUi", forward)

        def obj(this, name):
            objs = self.anms[this]["objs"]
            if name not in objs:
                o = self.alloc(0xB0)
                m.store(o + 0xA2, 1, 3)
                objs[name] = o
            return objs[name]

        def subst_f(mm, this, name, *a):
            return obj(this, bytes(mm.mem[name:name + 32]).split(b"\0")[0].decode())
        hook("GetSubstAdrsF__5ccAnmFPCcb", subst_f)

        def reset(mm, res, n, *a):
            mm.store(res, 4, self.alloc(8 * n))
            return 0
        hook("Reset__19ccSubstSearchResultFUs", reset)

        def subst(mm, this, name, res, *a):
            # "OBJ_xdhon*": the 32 lit crystals.
            arr = mm.load(res, 4)
            names = [f"OBJ_xdhon{k:02d}" for k in range(1, 17)] + [f"OBJ_xdhon{k:02d}f" for k in range(1, 17)]
            for k, nm in enumerate(names):
                mm.store(arr + 8 * k, 4, obj(this, nm))
            return 0
        hook("GetSubstAdrs__5ccAnmFPCcP19ccSubstSearchResult", subst)

        def draw(mm, this, *a):
            a_ = self.anms[this]
            if a_["label"] != "crystal":
                self.ev("anm_draw", a_["label"])
                return 0
            d = ""
            for k in range(16):
                c = HACK_CRYSTAL[k]
                for nm in (f"OBJ_xdhoff{c:02d}", f"OBJ_xdhon{c:02d}", f"OBJ_xdhon{c:02d}f"):
                    d += str(mm.load(obj(this, nm) + 0xA2, 1))
            self.ev("anm_draw", "crystal", d, f"{self.rotz:08x}")
            return 0
        hook("Draw__5ccAnmFv", draw)

    # --- frames ----------------------------------------------------------------
    def menu(self):
        return self.m.load(MENU_P, 4)

    def state(self):
        m = self.m
        c = self.menu()
        if not c:
            return None, None
        g = lambda o: m.load(c + o, 2, True)  # noqa: E731
        w = lambda o: m.load(c + o, 4, True)  # noqa: E731
        menu = g(6)
        lst = inf_va(0x0072EFE0) + 32 * max(0, min(menu, 88))
        st = [g(6), g(8), g(0xA), g(0x1A), w(0x34), w(0x38), g(0xC), w(0x40), g(0xE), w(0x44), g(0x10), w(0x48),
              w(0x3C), w(0x4C), g(0x20), g(0x18), g(0x14), g(0x1C), g(0x12A), m.load(lst + 0xA, 2, True),
              w(0xEC), m.load(CMNDTARGETFIX, 4, True), w(volume.menu_at(0x238)), m.mem[SAVE + 0x30:SAVE + 0xD0].hex(),
              m.load(EV + 0x77A, 2, True), m.load(EV + 0x77C, 2, True), m.load(EV + 0x77E, 2, True),
              m.load(GAME + 0x78, 4, True), w(volume.menu_at(0x23C)), m.load(SAVE + 0x7440, 2, True)]
        st += [m.mem[SAVE + off:SAVE + off + n].hex() for off, n in self.sc.watch]
        msg = m.load(CCMSG, 4)
        ms = None
        if msg:
            h = lambda o: m.load(msg + o, 2, True)  # noqa: E731
            ms = [h(0x1C), h(0x1E), h(0x20), h(0x22), h(0x24), h(0x28), h(0x32), h(0x34), h(0x38), h(0x3A)]
        return st, ms

    def run_threads(self):
        for t, fn in self.threads:
            if fn == sym("ccThPartyAdd__FP6ccTscb") and self.m.load(t + 20, 4):
                self.call(sym("AddMember__7ccPartyFi"), [PARTY, self.m.load(t + 24, 4)])
                self.m.store(t + 20, 4, 0)
            if fn == sym("ccThGtHackMenu__FP6ccTscb"):
                # ccLoadFLAddOne(flGtHackMenu) done by its first run.
                self.m.store(t + 20, 4, 0)
        self.threads = [x for x in self.threads if self.m.load(x[0] + 20, 4)]

    def manual_offs(self):
        """ai->manualSW cleared since the last frame (ChatMenuT's write)."""
        for ai, h in self.ais.items():
            if not self.m.load(ai, 1) & 1 and ai not in self.manual_off:
                self.manual_off.add(ai)
                self.ev("manual_off", h)

    def breath(self, n):
        mv = getattr(self, "movie", None)
        if mv and not mv[2]:
            mv[2] = True
            self.ev("drain_movie", sx32(self.m.load(mv[0] + 0x14, 4)))
        self.run_threads()
        self.manual_offs()
        if self.m.load(GAMEOVER, 4) and not getattr(self, "game_over", False):
            self.game_over = True
            self.ev("game_over")
        for _ in range(max(n, 1)):
            st, ms = self.state()
            self.ev("end", st, ms)
            self.frame += 1
            if self.frame > self.stop_at:
                raise Stop("done")
            self.start_frame()
            self.book_frame()
        return 0

    def start_frame(self):
        m, sc, f = self.m, self.sc, self.frame
        mv = getattr(self, "movie", None)
        if mv and f == mv[1] + sc.movie:
            m.store(mv[0] + 0x14, 4, 0xFFFFFFFF)
        for k, v in sorted(sc.skills.items()):
            if f >= k:
                self.skill_now = v
        push, rep = sc.pads.get(f, (0, 0))
        m.store(PAD + 0x64, 4, sc.directs.get(f, 0))
        m.store(PAD + 0x68, 4, push)
        m.store(PAD + 0x70, 4, rep)
        if sc.book and f == sc.book[1] and not getattr(self, "book_tcb", 0):
            # ccUseItemRequest's book branch, the menu task's frame: the
            # task stops here and ccThBook (35) runs, the use's wait loop
            # (Disp while its state is 1) at each frame's start.
            raise Stop("book")
        m.store(SYS + 0x358, 4, f)
        if f in sc.gim_acts:
            h, st = sc.gim_acts[f]
            for a, c in self.chars.items():
                if c.handle == h:
                    m.store(a + 0x1D4, 4, st & 0xFFFFFFFF)
        if f in sc.gones:
            for a in [a for a, c in self.chars.items() if c.handle == sc.gones[f]]:
                del self.chars[a]
        if f in sc.gamefs:
            area, inb, cnt = sc.gamefs[f]
            m.store(GAME + 0x14, 4, area)
            m.store(GAME + 0x58, 4, inb)
            m.store(GAME + 0x5C, 4, cnt)
        c = self.menu()
        if not c:
            return
        # The event task's calls, before the menu task in the frame.
        if f in sc.bans:
            if sc.bans[f]:
                m.store(c + 0xFE, 2, 1)
                m.store(c + 0x102, 2, 1)
                m.store(c + 0xC, 2, 0)
                m.store(c + 0x10, 2, 0)
            else:
                m.store(c + 0xFE, 2, 0)
                m.store(c + 0x102, 2, 0)
                m.store(c + 0xC, 2, 1)
                m.store(c + 0x10, 2, 1)
        if f in sc.noises:
            m.store(c + 0x104, 2, sc.noises[f] & 0xFFFF)
        if f in sc.opens:
            m.store(c + 0x14, 2, sc.opens[f])
        if f in sc.firsts:
            m.store(c + 0xF8, 2, 1)
        if f in sc.modes:
            m.store(c + 0x16, 2, sc.modes[f])
        if f in sc.attacks:
            m.store(c + 0xF4, 2, sc.attacks[f])
        msg = m.load(CCMSG, 4)
        if f in sc.msgs:
            emode, name, ls = sc.msgs[f]
            d = self.alloc(0x18)
            m.store(d, 4, emode)
            for i in range(3):
                m.store(d + 12 + 4 * i, 4, self.put(ls[i]) if ls[i] is not None else 0)
            self.call(sym("Change__9ccMessageFP9ccMsgDataPcii"), [msg, d, self.put(name) if name is not None else 0,
                                                                   0xFFFFFFFF, 0xFFFFFFFF])
        if f in sc.infos:
            ls = [self.put(x) if x is not None else 0 for x in sc.infos[f]]
            self.call(sym("ChangeInfo__9ccMessageFPcPcPcPcii"), [msg] + ls + [0, 0xFFFFFFFF, 0xFFFFFFFF])
        if f in sc.checks:
            self.checking = True
        if self.checking:
            r = self.call(sym("Check__9ccMessageFi"), [msg, 0])
            r = r - (1 << 32) if r & 0x80000000 else r
            self.ev("check", r)
            if r != 0:
                self.checking = False

    def run_book(self):
        """ccThBook natively from the book's frame: the use's start (the
        task, `bgStatus` 3, `Disp`), then the task with the use's loop at
        each frame's start; its stream's calls answered as a stream that
        plays `sc.movie` frames."""
        m, page = self.m, self.sc.book[0]
        t = self.book_tcb = self.alloc(0x60)
        m.store(t + 0x14, 4, page)
        m.store(t + 0x18, 4, 1)
        m.store(self.menu() + 0xE, 2, 3)
        self.call(sym("Disp__10ccMenuCtrlFv"), [self.menu()])
        h = m.hooks
        h[sym("GetChunkAdrsF__8ccStreamFPCci")] = lambda mm, *a: self.alloc(0x40)
        h[sym("GetSubstAdrsF__8ccStreamFPCci")] = lambda mm, *a: self.alloc(0x40)
        try:
            self.m.call(sym("ccThBook__FPv"), [t], limit=2_000_000_000)
        except Stop as e:
            if str(e) != "done":
                raise

    def names(self):
        """Sprite addresses to the probe's names, from the constructed ccMenuCtrl."""
        m = self.m
        c = self.menu()
        L = lambda o: m.load(c + o, 4)  # noqa: E731
        n = {L(0x5C): "win", L(0x60): "winPr", L(0x64): "winA", L(0x68): "eneLife", L(0x6C): "target",
             L(0x70): "kanji", L(0x74): "kanjiPr", L(0x78): "name", L(0x7C): "namePr", L(0xB0): "font",
             L(0xB4): "fly", L(0xCC): "icon", L(0xD0): "itemIcon", L(0xD4): "conIcon", L(0xD8): "bg",
             L(0xDC): "drain", L(0xE0): "protect", L(0xE4): "mask"}
        for i in range(8):
            n[L(0x80 + 4 * i)] = f"set{i}"
        for i in range(4):
            n[L(0xBC + 4 * i)] = f"face{i}"
        msg = m.load(CCMSG, 4)
        n[m.load(msg, 4)] = "msgwin"
        n[m.load(FONT, 4)] = "sysfont"
        if self.hack_mask:
            n[self.hack_mask] = "hackmask"
        book = m.load(BOOK_P, 4)
        if book:
            n[m.load(book + 4, 4)] = "bookBg"
            n[m.load(book + 8, 4)] = "bookWin"
            n[m.load(book + 0xC, 4)] = "bookButton"
        return n

    def book_frame(self):
        """The use's wait loop at a frame's start: Disp while state 1."""
        t = getattr(self, "book_tcb", 0)
        if t and self.m.load(t + 0x18, 4) == 1:
            self.call(sym("Disp__10ccMenuCtrlFv"), [self.menu()])

    def run(self):
        tscb = self.alloc(0x60)
        try:
            self.m.call(sym("ccThMenu__FPv"), [tscb], limit=2_000_000_000)
        except Stop as e:
            if str(e) == "book":
                self.run_book()
            elif str(e) != "done":
                raise
        names = self.names()
        frames = {}
        for f, e in self.log:
            o = frames.setdefault(f, {"pk": [], "kanji": [], "mc": [], "mt": [], "ev": [], "st": None, "msg": None})
            k = e[0]
            if k == "pk":
                name = names.get(e[1], hex(e[1]))
                r = e[2]
                if name == "msgwin":
                    o["mc"].append([r[0], r[2], r[3], r[4], r[5], r[14][3]])
                else:
                    o["pk"].append([name] + list(r))
            elif k == "kanji":
                o["kanji"].append([names.get(e[1], hex(e[1])), e[2].hex() or "="])
            elif k == "mt":
                line = None
                msg = self.m.load(CCMSG, 4)
                for i in range(4):
                    if self.m.load(msg + 4 + 4 * i, 4) == e[1]:
                        line = i
                o["mt"].append([line, e[2].hex() or "=", e[3], e[4], e[5], e[6]])
            elif k == "end":
                o["st"], o["msg"] = e[1], e[2]
            else:
                o["ev"].append(list(e))
        return frames


def drain_answers(game, sc):
    """Data Drain's rules are the runtime's: the port is answered with what
    the game's own left (its infection, drops and side effect, the party's
    HP and SP after it), as recorded while it ran."""
    rec = getattr(game, "drain_rec", {})
    for erosion, count, drops in rec.get("drops", []):
        sc.extra.append("drainanswer %d %d %s" % (erosion, count, " ".join(map(str, drops))))
    for side, party in rec.get("side", []):
        sc.extra.append("drainside %d %d %d " % side + " ".join(f"{h} {hp} {sp}" for h, hp, sp in party if h))


def norm_port(frames):
    out = {}
    for j in frames:
        o = {"st": j["st"], "msg": j["msg"], "ev": [list(e) for e in j["ev"]]}
        o["pk"] = [[p[0], p[1], p[2] if p[2] is not None else None] + p[3:] for p in j["pk"]]
        o["kanji"] = [[k[0], k[1] or "="] for k in j["kanji"]]
        o["mc"] = j["mc"]
        o["mt"] = j["mt"]
        out[j["f"]] = o
    return out


def close(a, b):
    if isinstance(a, float) or isinstance(b, float):
        return abs(a - b) < 1e-3
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(close(x, y) for x, y in zip(a, b))
    return a == b


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class FieldUiAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-fieldui", "--features",
                        "trace", "--example", "fieldui_probe"], cwd=ROOT, check=True)

    def compare(self, sc, what, g=None):
        if g is None:
            game = getattr(self, "game_class", Game)(sc)
            g = game.run()
            drain_answers(game, sc)
        p = norm_port(port(sc))
        for f in range(1, sc.frames):
            a, b = g.get(f), p.get(f)
            self.assertIsNotNone(a, f"{what}: no game frame {f}")
            self.assertIsNotNone(b, f"{what}: no port frame {f}")
            self.assertEqual(a["st"], b["st"], f"{what} frame {f}: state")
            self.assertEqual(a["msg"], b["msg"], f"{what} frame {f}: message state")
            key = lambda e: json.dumps(e)  # noqa: E731
            ga = sorted((e for e in a["ev"] if e[0] != "map_alpha"), key=key)
            pa = sorted((e for e in b["ev"] if e[0] != "map_alpha"), key=key)
            self.assertEqual(ga, pa, f"{what} frame {f}: events")
            gm = [e for e in a["ev"] if e[0] == "map_alpha"]
            pm = [e for e in b["ev"] if e[0] == "map_alpha"]
            self.assertTrue(close(gm, pm), f"{what} frame {f}: map alpha {gm} vs {pm}")
            for obj in sorted({x[0] for x in a["pk"]} | {x[0] for x in b["pk"]}):
                ga = [x for x in a["pk"] if x[0] == obj]
                pa = [x for x in b["pk"] if x[0] == obj]
                self.assertEqual(len(ga), len(pa), f"{what} frame {f}: {obj} packets\n{ga}\n{pa}")
                for x, y in zip(ga, pa):
                    self.assertTrue(close(x, y), f"{what} frame {f}: {obj}\n{x}\n{y}")
            self.assertEqual(sorted(a["kanji"]), sorted(b["kanji"]), f"{what} frame {f}: kanji")
            self.assertTrue(close(a["mc"], b["mc"]), f"{what} frame {f}: message cells\n{a['mc']}\n{b['mc']}")
            self.assertTrue(close(a["mt"], b["mt"]), f"{what} frame {f}: message texts\n{a['mt']}\n{b['mt']}")

    def party(self, sc, n=1):
        sc.party[0] = Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite")
        if n > 1:
            sc.party[1] = Char(0x101, 1, 2, 30, 50, 150, 80, b"Orca")
        if n > 2:
            sc.party[2] = Char(0x102, 1, 3, 90, 9, 1500, 100, b"BlackRose")

    def test_panels(self):
        for n in (1, 2, 3):
            sc = Scenario(60)
            self.party(sc, n)
            self.compare(sc, f"panels {n}")

    def test_inter_noiz(self):
        """Disp's interNoiz: 2 a burst once; 1 (and Infection's 3 away from
        town 4) every 64th frame out of the menus, off in one; Infection's
        3 in town 4 every 16th frame. From Mutation on, 3 every 64th or
        128th frame by rand() & 1, 4 every 16th, 5 every 32nd with the
        camera's shake and sound 258 every 8th."""
        cases = [(2, 0, 0), (1, 0, 0), (1, 0, 5), (3, 4, 7), (3, 1, 4)]
        if volume.NAME != "infection":
            cases += [(3, 0, 0), (3, 0, 5), (4, 0, 0), (5, 0, 0), (5, 0, 3)]
        for level, town, rng in cases:
            sc = Scenario(260)
            self.party(sc, 2)
            sc.server = (0, town, 0)
            sc.rng = rng
            sc.noises[10] = level
            # The PERSONAL menu open over frame 128, a ban over 192.
            sc.pads[120] = (TRIANGLE, 0)
            sc.pads[150] = (CANCEL, 0)
            sc.bans[185] = True
            sc.bans[200] = False
            self.compare(sc, f"interNoiz {level} town {town} rng {rng}")

    def test_panels_ban(self):
        sc = Scenario(80)
        self.party(sc, 2)
        sc.bans[20] = True
        sc.bans[50] = False
        self.compare(sc, "menu_ban")

    def test_field_message(self):
        sc = Scenario(120)
        self.party(sc, 2)
        sc.msgs[10] = (0, b"Orca", [b"A sample line!", b"", b""])
        sc.checks.append(15)
        sc.pads[40] = (OK, 0)
        sc.pads[44] = (OK, 0)
        sc.msgs[70] = (2, b"#0", [b"Uh...", b"Second line", b"Third"])
        sc.checks.append(75)
        sc.pads[80] = (OK, 0)
        sc.pads[110] = (OK, 0)
        self.compare(sc, "field message")

    def test_info(self):
        sc = Scenario(70)
        self.party(sc, 1)
        sc.infos[10] = [b"You now have ", b"#YOrca's member address!", None]
        sc.checks.append(15)
        sc.pads[40] = (OK, 0)
        self.compare(sc, "info")

    def test_personal_menu(self):
        for area in (0, 1, 2):
            sc = Scenario(140)
            self.party(sc, 2)
            sc.game = (area, 0, 0, 0)
            sc.opens[10] = area
            rng = random.Random(area)
            f = 20
            while f < 100:
                f += rng.choice([1, 2, 3, 5, 9])
                sc.pads[f] = (0, rng.choice([UP, DOWN]))
            sc.pads[110] = (CANCEL, 0)
            self.compare(sc, f"personal {area}")

    def test_option_list(self):
        sc = Scenario(90)
        self.party(sc, 1)
        sc.opens[10] = 12
        for k, f in enumerate(range(20, 60, 4)):
            sc.pads[f] = (0, DOWN if k % 3 else UP)
        sc.pads[70] = (CANCEL, 0)
        self.compare(sc, "option")

    def test_dead_player(self):
        sc = Scenario(120)
        self.party(sc, 1)
        sc.dead = 1
        sc.opens[10] = 0
        sc.pads[25] = (OK, 0)
        sc.pads[60] = (OK, 0)
        sc.pads[100] = (CANCEL, 0)
        self.compare(sc, "dead")

    def test_target(self):
        for types, name, hp, mhp, sp, msp, tag in ((0x20, b"Goblin", 40, 80, 0, 0, (250, 200)),
                                                   (0x2, b"Orca", 100, 150, 30, 80, (500, 470)),
                                                   (0x8, b"Merchant", 0, 0, 0, 0, (10, 5)),
                                                   (0x80, b"Skeith", 9000, 12000, 0, 0, None),
                                                   (0x40, b"Big One", 1234, 20000, 0, 0, (300, 100))):
            sc = Scenario(70)
            self.party(sc, 2)
            sc.target = Char(0x200, types, 5, hp, sp, mhp, msp, name, tag=tag)
            sc.game = (1, 0, 0, 0)
            self.compare(sc, f"target {name}")

    def test_target_tolerance(self):
        # A foe's immunity right of its HP (nameKanji's sixth line, pulsing
        # with the cursor), struck through once its lowest Exdefense bit's
        # defence is lowered: fire held and broken, magic held with fire
        # broken, a boss's physical broken; none in a town.
        for types, ex, lowered, game in ((0x20, 0x10, 0, (1, 0, 0, 0)), (0x20, 0x10, 0x10, (1, 0, 0, 0)),
                                         (0x40, 0x12, 0x10, (1, 0, 0, 0)), (0x80, 0x1, 0x1, (1, 0, 0, 0)),
                                         (0x20, 0x80, 0x80, (1, 0, 0, 0)), (0x20, 0x10, 0x10, (0, 0, 0, 0))):
            sc = Scenario(70)
            self.party(sc, 2)
            sc.target = Char(0x200, types, 172, 1210, 0, 1210, 0, b"Hell Hound", tag=(250, 200),
                             exdef=(ex, lowered))
            sc.game = game
            self.compare(sc, f"tolerance {types:#x} {ex:#x} {lowered:#x} area {game[0]}")

    def test_attack_cursor(self):
        # plAttack on an enemy target: the attack cursor (0x2912 cell),
        # scaled from the diamond's pulse (0x0051f21c), and an area's marks.
        sc = Scenario(70)
        self.party(sc, 2)
        sc.target = Char(0x200, 0x20, 5, 40, 0, 80, 0, b"Goblin", tag=(250, 200))
        sc.game = (1, 0, 0, 0)
        sc.attacks = {0: 1, 40: 0, 50: 1}
        self.compare(sc, "attack cursor")

    def book(self, sc, page, start=2, movie=4):
        """Ryu Book `page` read from frame `start` (its stream `movie`
        frames long), its rewards' save bytes watched: `hyProccess`,
        `hyItem`, the desktop's wallpapers, BGMs and movies."""
        sc.book = (page, start)
        sc.movie = movie
        sc.watch += [(0x683D, 0x21), (0x2238, 0x2C)]

    def test_book_1(self):
        # Book I: the areas visited (15, past the first reward's 10) and
        # the play time (3:00:02); the reward's windows dismissed, the
        # cursor moved down and up, the book closed.
        # The harness's use ends with the book (frame 240's cancel).
        sc = Scenario(241)
        self.party(sc, 1)
        sc.saves += [(0x6862, 2, 15), (0x8400, 4, 60 * (3 * 3600 + 2))]
        self.book(sc, 0)
        sc.pads = {f: (OK, 0) for f in range(40, 200, 20)}
        sc.pads.update({205: (DOWN, 0), 215: (UP, 0), 225: (DOWN, 0), 240: (CANCEL, 0)})
        self.compare(sc, "book 1")

    def test_book_2(self):
        # Book II: the portals opened (45, past rows of 10 and 15 the
        # first reward's), the fields' all opened (12) and the dungeons'
        # (3); the rewards' windows, the rows walked, the book closed.
        sc = Scenario(400)
        self.party(sc, 1)
        sc.saves += [(0x6864, 2, 45), (0x6866, 2, 12), (0x6868, 2, 3)]
        self.book(sc, 1)
        sc.pads = {f: (OK, 0) for f in range(40, 330, 15)}
        sc.pads.update({340: (DOWN, 0), 350: (DOWN, 0), 360: (DOWN, 0), 370: (UP, 0), 385: (CANCEL, 0)})
        sc.frames = 386
        self.compare(sc, "book 2")

    def test_book_4(self):
        # Book IV: twelve kinds slain (past the first reward), one more
        # than 9 times; where the last was slain (server 1, words 3 5 7);
        # onto the list, down it, an enemy opened and closed, up off it,
        # closed.
        sc = Scenario(420)
        self.party(sc, 1)
        for k in (0, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233):
            sc.saves += [(0x68B7 + k, 1, 3)]
        sc.saves += [(0x68B7 + 34, 1, 12)]
        for k, v in enumerate((1, 3, 5, 7)):
            sc.saves += [(0x69F0 + 8 * 2 + 2 * k, 2, v)]
        self.book(sc, 3)
        sc.pads = {f: (OK, 0) for f in range(40, 200, 15)}
        sc.directs.update({250: DOWN, 251: DOWN})
        sc.pads.update({270: (0, DOWN), 290: (0, DOWN), 300: (0, UP), 310: (OK, 0), 340: (CANCEL, 0),
                        360: (0, UP), 375: (0, UP), 390: (0, UP), 410: (CANCEL, 0)})
        sc.frames = 411
        self.compare(sc, "book 4")

    def test_book_5(self):
        # Book V: BlackRose and Orca met, with gold given (the total past
        # the first reward), their time in the party and friendliness; the
        # reward's windows, onto the list and along it, up off it, closed.
        sc = Scenario(400)
        self.party(sc, 1)
        self.book3(sc)
        sc.saves += [(0x73F8 + 4 * 2, 4, 1200), (0x73F8 + 4 * 3, 4, 300), (0x73B4 + 4 * 2, 4, 60 * 4000),
                     (0x7488 + 0xDC * 2 + 0xDA, 2, 77)]
        self.book(sc, 4)
        sc.pads = {f: (OK, 0) for f in range(40, 200, 15)}
        # The rows move with the held buttons: DOWN held three frames.
        sc.directs.update({260: DOWN, 261: DOWN, 262: DOWN})
        sc.pads.update({280: (0, DOWN), 300: (0, DOWN), 320: (0, UP), 340: (0, UP), 385: (CANCEL, 0)})
        sc.frames = 386
        self.compare(sc, "book 5")

    def test_book_6(self):
        # Book VI: the boxes opened (30), the objects broken (12), the
        # idols opened (5); the rewards' windows, the rows walked, the
        # book closed.
        sc = Scenario(400)
        self.party(sc, 1)
        sc.saves += [(0x7440, 2, 30), (0x7442, 2, 12), (0x746E, 2, 5)]
        self.book(sc, 5)
        sc.pads = {f: (OK, 0) for f in range(40, 330, 15)}
        sc.pads.update({340: (DOWN, 0), 350: (DOWN, 0), 360: (DOWN, 0), 370: (UP, 0), 385: (CANCEL, 0)})
        sc.frames = 386
        self.compare(sc, "book 6")

    def test_book_7(self):
        # Book VII: the springs (4), the mushrooms' grandfather (2), the
        # symbols (3); the rewards' windows, the rows walked, closed.
        sc = Scenario(400)
        self.party(sc, 1)
        sc.saves += [(0x7470, 2, 4), (0x7472, 2, 2), (0x7444, 2, 3)]
        self.book(sc, 6)
        sc.pads = {f: (OK, 0) for f in range(40, 330, 15)}
        sc.pads.update({340: (DOWN, 0), 350: (DOWN, 0), 360: (DOWN, 0), 370: (UP, 0), 385: (CANCEL, 0)})
        sc.frames = 386
        self.compare(sc, "book 7")

    def test_book_8(self):
        # Book VIII: four Grunties met (counts at +0x7474) and seven foods
        # given (+0x7446, 40 in all); the menu, the Grunties' page and
        # back, down to the foods, their page (the total, onto the list,
        # along it, up off it) and back, the book closed.
        sc = Scenario(560)
        self.party(sc, 1)
        for k, v in ((0, 3), (2, 1), (5, 7), (8, 2)):
            sc.saves += [(0x7474 + 2 * k, 2, v)]
        for k, v in ((1, 5), (2, 9), (4, 1), (7, 6), (9, 4), (12, 8), (15, 7)):
            sc.saves += [(0x7446 + 2 * k, 2, v)]
        self.book(sc, 7)
        sc.pads = {40: (OK, 0), 80: (CANCEL, 0), 100: (DOWN, 0), 110: (OK, 0)}
        sc.pads.update({f: (OK, 0) for f in range(140, 260, 15)})
        sc.pads.update({280: (0, DOWN), 300: (0, DOWN), 320: (0, DOWN), 340: (0, UP), 360: (0, UP),
                        380: (0, UP), 400: (0, DOWN), 420: (CANCEL, 0), 450: (UP, 0), 470: (CANCEL, 0)})
        sc.frames = 471
        self.compare(sc, "book 8")

    def book3(self, sc):
        """Book III's characters: BlackRose and Orca in the party flag,
        trades with them, people met (trade counts 0 and 4), a PC's item
        list and an online member."""
        sc.saves += [(0x2220, 4, (1 << 2) | (1 << 3)), (0x686A + 1, 1, 3), (0x686A + 2, 1, 5),
                     (0x686A + 17, 1, 0), (0x686A + 20, 1, 4), (0x686A + 40, 1, 0xFF)]
        # BlackRose's trade list: two items (id, category, num).
        sc.saves += [(0xE3C + 64 + 0, 2, 3), (0xE3C + 64 + 2, 1, 10), (0xE3C + 64 + 3, 1, 1),
                     (0xE3C + 64 + 4, 2, 7), (0xE3C + 64 + 6, 1, 11), (0xE3C + 64 + 7, 1, 2)]
        for k in range(14):
            sc.saves += [(0xE3C + 64 + 8 + 4 * k, 4, 0xFFFFFFFF)]
        # Their names as NewGame leaves them in spcParam (charTbl's).
        sc.spc[2] = (b"Orca", 12, 345, 1500, 150, 80, 1)
        sc.spc[3] = (b"Marlo", 10, 100, 0, 100, 40, 2)

    def given(self, sc, page):
        """Every reward of book `page` given already (`hyProccess`)."""
        sc.saves += [(0x683D + 4 * page + k, 1, 0x7F) for k in range(4)]

    def test_book_3(self):
        # Book III: its reward (12 trades, past 5) and its windows.
        sc = Scenario(300)
        self.party(sc, 1)
        self.book3(sc)
        self.book(sc, 2)
        sc.pads = {f: (OK, 0) for f in range(40, 200, 15)}
        sc.pads[260] = (CANCEL, 0)
        sc.frames = 261
        self.compare(sc, "book 3")

    def test_book_3_list(self):
        # Book III's list: the cursor onto it (to the first one known),
        # down and up past the unknown, a sub-window opened and closed,
        # up off the list, the book closed.
        sc = Scenario(300)
        self.party(sc, 1)
        self.book3(sc)
        self.given(sc, 2)
        self.book(sc, 2)
        sc.pads = {40: (DOWN, 0), 50: (DOWN, 0), 60: (0, DOWN), 70: (0, DOWN), 80: (0, UP), 90: (OK, 0),
                   120: (CANCEL, 0), 130: (0, DOWN), 140: (0, UP), 150: (0, UP), 160: (0, UP), 170: (UP, 0),
                   200: (CANCEL, 0)}
        sc.frames = 201
        self.compare(sc, "book 3 list")

    def test_enemy_bars(self):
        sc = Scenario(60)
        self.party(sc, 3)
        sc.game = (1, 1, 0, 0)
        kinds = [(300.0, 1, (100, 200)), (1000.0, 1, (20, 50)), (3000.0, 1, (490, 440)), (500.0, 0, (0, 0)),
                 (900.0, 0, (0, 0)), (100.0, 1, (250, 300))]
        for k, (d, res, bar) in enumerate(kinds):
            c = Char(0x300 + k, 0x20 if k % 2 else 0x40, 10 + k, 10 * (k + 1), 0, 70, 0, b"Enemy")
            sc.sorted.append(Enemy(c, d, res, bar, dirc=0.7 * k - 1.5, pp=(0, 30, 0, 0, 250, 1)[k], dead=int(k == 5)))
        sc.sorted.append(Enemy(Char(0x3ff, 0x8, 99, 1, 0, 1, 0, b"Npc"), 50.0, 1, (1, 1)))
        self.compare(sc, "enemy bars")

    def test_banner(self):
        sc = Scenario(110)
        self.party(sc, 2)
        sc.game = (1, 1, 0, 0)
        for f in range(40, 100):
            sc.gamefs[f] = (1, 1 if f < 70 else 2, f - 39)
        self.compare(sc, "battle banner")

    def test_new_mail(self):
        sc = Scenario(260)
        self.party(sc, 1)
        sc.saves.append((0x2264 + 7, 1, 1))
        self.compare(sc, "new mail")

    def test_low_hp(self):
        sc = Scenario(80)
        sc.party[0] = Char(0x100, 1, 0, 10, 60, 120, 60, b"Kite")
        sc.party[1] = Char(0x101, 1, 2, 0, 5, 150, 80, b"Orca", cond=[1])
        sc.party[2] = Char(0x102, 1, 5, 90, 9, 100, 100, b"Mistral", cond=[0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1])
        self.compare(sc, "low hp")


    # --- the battle menus ------------------------------------------------------
    KITE_SKILLS = (6, 7, 9, 170, 175, 178, 180, 187, 2, 3)

    def battle(self, sc, sp=60, skills=KITE_SKILLS, plcol=1, evo=9):
        """Kite, Orca and a fallen BlackRose in a field battle, five enemies
        around, Kite's skills in the save."""
        sc.party[0] = Char(0x100, 1, 0, 120, sp, 120, 60, b"Kite", width=40.0, pos=(0.0, 0.0, 0.0))
        sc.party[1] = Char(0x101, 1, 2, 30, 50, 150, 80, b"Orca", width=40.0, pos=(100.0, 50.0, 0.0))
        sc.party[2] = Char(0x102, 1, 3, 0, 9, 1500, 100, b"BlackRose", cond=[1], width=40.0, pos=(-80.0, 0.0, 0.0))
        for k, v in enumerate(skills):
            sc.saves.append((0x1EC4 + 2 * k, 2, v))
        for k in range(len(skills), 20):
            sc.saves.append((0x1EC4 + 2 * k, 2, 0xFFFF))
        sc.saves.append((0x6771, 1, plcol))
        sc.saves.append((0x6860, 2, evo))
        # Data Drain's movie off (drainDemo): the port does not play it.
        sc.saves.append((0x8427, 1, 0))
        sc.game = (1, 1, 0, 0)
        foes = [(0x20, 300.0, 50.0, True, (200.0, 0.0, 0.0), 0, b"Goblin"),
                (0x40, 380.0, 30.0, True, (250.0, 100.0, 0.0), 30, b"Wolf"),
                (0x20, 1000.0, 50.0, True, (900.0, 300.0, 0.0), 50, b"Goblin"),
                (0x20, 200.0, 50.0, False, (0.0, -190.0, 0.0), 0, b"Bat"),
                (0x80, 500.0, 100.0, True, (600.0, 0.0, 0.0), 0, b"Skeith")]
        for k, (t, d, w, view, pos, pp, name) in enumerate(foes):
            c = Char(0x300 + k, t, 10 + k, 40, 0, 80, 0, name, width=w, view=view, pos=pos)
            sc.sorted.append(Enemy(c, d, 1, (100 + 40 * k, 200), pp=pp))
        sc.chains = ([sc.party[0], sc.party[1], sc.party[2]], [e.c for e in sc.sorted], [])

    def items(self, sc, entries):
        for k, (cat, iid, num) in enumerate(entries):
            sc.saves += [(0x30 + 4 * k, 2, iid), (0x32 + 4 * k, 1, cat & 0xFF), (0x33 + 4 * k, 1, num)]
        for k in range(len(entries), 40):
            sc.saves += [(0x30 + 4 * k, 2, 0xFFFF), (0x32 + 4 * k, 1, 0xFF), (0x33 + 4 * k, 1, 0)]

    def test_skill_menu(self):
        R1, L1 = 0x8, 0x4
        sc = Scenario(330)
        self.battle(sc)
        sc.opens[10] = 1
        sc.pads.update({25: (OK, 0), 40: (0, DOWN), 48: (0, R1), 56: (0, R1), 64: (0, R1), 72: (0, R1),
                        80: (0, R1), 88: (0, R1), 96: (0, L1), 104: (0, R1), 112: (OK, 0), 128: (0, DOWN),
                        136: (OK, 0)})
        sc.skills.update({137: 2, 150: 0})
        sc.opens[200] = 1
        sc.pads.update({215: (OK, 0), 230: (0, R1), 238: (0, R1), 246: (0, DOWN), 254: (OK, 0), 268: (CANCEL, 0),
                        282: (CANCEL, 0), 296: (CANCEL, 0)})
        self.compare(sc, "skills")

    def test_revive_and_drain(self):
        R1 = 0x8
        sc = Scenario(215)
        self.battle(sc)
        sc.sorted[0].pp = 10
        sc.opens[10] = 1
        # Recovery page, Rip Maen (180) on the fallen BlackRose.
        sc.pads.update({25: (OK, 0), 40: (0, R1), 48: (0, R1), 56: (0, DOWN), 64: (OK, 0), 80: (OK, 0)})
        sc.opens[120] = 1
        # The Data Drain page: Drain Arc (3) on the Goblin, its protect
        # broken, the Wolf beside it taken too; Data Drain's menu (66) next.
        sc.pads.update({135: (OK, 0), 150: (0, R1), 158: (0, R1), 166: (0, R1), 174: (0, DOWN), 182: (OK, 0),
                        197: (OK, 0)})
        self.compare(sc, "revive and drain")

    def drain(self, sc, rng, erosion):
        """Data Drain (2) on the Goblin, its protect broken, the movie off
        (drainDemo 0), rand() always `rng`, the infection at `erosion`."""
        R1 = 0x8
        self.battle(sc)
        sc.sorted[0].pp = 10
        sc.rng = rng
        sc.saves += [(0x676E, 2, erosion)]
        sc.watch += [(0x676E, 2), (0x685E, 4), (0x30, 0xA0)]
        sc.opens[10] = 1
        sc.pads.update({25: (OK, 0), 40: (0, R1), 48: (0, R1), 56: (0, R1), 64: (0, R1), 72: (0, R1), 88: (OK, 0),
                        103: (OK, 0)})

    def compare_drain(self, sc, what):
        """The game's own rules answer the port's requests (compare feeds
        them)."""
        self.compare(sc, what)

    def test_data_drain(self):
        # No side effect (both rolls above the infection): the warning, the
        # drain count, the drop through 67 and 29.
        sc = Scenario(420)
        self.drain(sc, 99, 20)
        sc.pads.update({190: (OK, 0), 215: (OK, 0), 240: (OK, 0), 265: (OK, 0), 290: (OK, 0)})
        self.compare_drain(sc, "data drain")
        # A side effect (rand() 0): its noise, its message, then 20.
        sc = Scenario(520)
        self.drain(sc, 0, 100)
        sc.pads.update({205: (OK, 0), 230: (OK, 0), 255: (OK, 0), 280: (OK, 0), 305: (OK, 0)})
        self.compare_drain(sc, "data drain side effect")

    def test_drain_movie(self):
        """Data Drain's movie: an enemy's stream by its size with drainDemo
        on (small 109, middle 110, large 111); a boss's by its id (14:
        111) with drainDemo off, after its EntryAffect(21). The stream is
        faked: it ends 40 frames after it starts."""
        for size, num in ((4, 109), (3, 110), (1, 111)):
            sc = Scenario(470)
            self.drain(sc, 99, 20)
            sc.saves.append((0x8427, 1, 1))
            sc.sorted[0].c.size = size
            sc.movie = 40
            sc.pads.update({235: (OK, 0), 260: (OK, 0), 285: (OK, 0), 310: (OK, 0), 335: (OK, 0)})
            self.compare_drain(sc, f"drain movie {num}")
        # The boss first on the chain, its protect broken.
        sc = Scenario(470)
        self.drain(sc, 99, 20)
        boss = sc.sorted.pop()
        boss.c.pos = (200.0, 0.0, 0.0)
        boss.dist, boss.pp = 300.0, 10
        sc.sorted.insert(0, boss)
        sc.chains = (sc.chains[0], [e.c for e in sc.sorted], [])
        sc.movie = 40
        sc.pads.update({235: (OK, 0), 260: (OK, 0), 285: (OK, 0), 310: (OK, 0), 335: (OK, 0)})
        self.compare_drain(sc, "drain movie boss")

    def test_skill_refusals(self):
        # In town; short of SP; Data Drain while the events hold operation 17.
        for what, area, sp, operate, pads in (
                ("town", 0, 60, 0, {25: (OK, 0), 40: (OK, 0), 60: (OK, 0)}),
                ("sp", 1, 5, 0, {25: (OK, 0), 40: (OK, 0), 60: (OK, 0)}),
                ("drain", 1, 60, 1 << 17, {25: (OK, 0), 40: (0, 0x4), 50: (OK, 0), 70: (OK, 0)})):
            sc = Scenario(110)
            self.battle(sc, sp=sp)
            sc.game = (area, 0, 0, 0)
            sc.operate = operate
            sc.opens[10] = 0 if area == 0 else 1
            sc.pads.update(pads)
            sc.pads[100] = (CANCEL, 0)
            self.compare(sc, f"skill refusal {what}")

    def test_target_warnings(self):
        # Saber Dance with every enemy out of reach; Data Drain with no
        # protect broken.
        for what, skills, pads in (("reach", (6,), {25: (OK, 0), 40: (OK, 0), 60: (OK, 0)}),
                                   ("drain", (2, 6), {25: (OK, 0), 40: (0, 0x4), 50: (OK, 0), 70: (OK, 0)})):
            sc = Scenario(110)
            self.battle(sc, skills=skills)
            for e in sc.sorted:
                e.pp = 0
                if what == "reach":
                    e.dist = 5000.0
            sc.opens[10] = 1
            sc.pads.update(pads)
            self.compare(sc, f"target warning {what}")

    def test_item_menu(self):
        R1 = 0x8
        sc = Scenario(345)
        self.battle(sc)
        self.items(sc, [(10, 0, 3), (10, 18, 1), (13, 0, 1), (11, 0, 2), (12, 0, 1), (14, 3, 1), (6, 5, 1),
                        (0, 2, 1), (13, 1, 2), (15, 44, 1)])
        sc.saves += [(0xB70, 2, 3), (0xB72, 1, 10), (0xB73, 1, 1)]
        sc.opens[10] = 1
        # Items: Health Drink on Orca.
        sc.pads.update({25: (0, DOWN), 33: (OK, 0), 48: (0, DOWN), 56: (0, UP), 64: (OK, 0), 80: (0, DOWN),
                        88: (OK, 0)})
        sc.skills.update({89: 2, 100: 0})
        # Through the pages; the Power Book at once (PERSONAL keeps its
        # cursor on Items).
        sc.opens[140] = 1
        sc.pads.update({163: (OK, 0), 178: (0, R1), 186: (0, R1), 194: (OK, 0)})
        # Treasure refused ("You cannot use this item."); equipment: its
        # status (triangle).
        sc.opens[230] = 1
        sc.pads.update({253: (OK, 0), 268: (0, R1), 276: (OK, 0), 295: (OK, 0), 318: (0, R1),
                        326: (TRIANGLE, 0)})
        self.compare(sc, "items")

    def test_item_special(self):
        # Mage's Soul (10/18) on the standing party; the Fortune Wire (13/0)
        # with nothing near; the Sprite Ocarina (13/1) refused in a field,
        # asked in a dungeon: Cancel, then OK.
        sc = Scenario(345)
        self.battle(sc)
        self.items(sc, [(10, 18, 1), (13, 0, 1), (13, 1, 2)])
        sc.opens[10] = 1
        sc.pads.update({25: (0, DOWN), 33: (OK, 0), 48: (OK, 0), 60: (0, DOWN), 68: (OK, 0)})
        sc.opens[100] = 1
        sc.pads.update({115: (OK, 0), 130: (OK, 0), 150: (OK, 0), 165: (0, DOWN), 173: (OK, 0), 195: (OK, 0),
                        215: (CANCEL, 0), 228: (CANCEL, 0)})
        sc.gamefs[240] = (2, 0, 0)
        sc.opens[250] = 2
        sc.pads.update({258: (0, DOWN), 265: (OK, 0), 280: (OK, 0), 295: (OK, 0), 310: (OK, 0), 318: (0, UP),
                        326: (OK, 0)})
        self.compare(sc, "item special")

    def test_random_battle_menus(self):
        # Random pads through PERSONAL, Skills, Items and TARGET in a field
        # battle, the skill check going up and down.
        for seed in range(1, 11):
            rnd = random.Random(seed)
            sc = Scenario(420)
            sc.area_words = (3, 105, 208)
            self.battle(sc, sp=rnd.choice((5, 25, 60)))
            sc.sorted[rnd.randrange(5)].pp = 20
            self.items(sc, [(10, 0, 3), (10, 18, 1), (13, 0, 1), (11, 0, 2), (12, 0, 1), (14, 3, 1), (6, 5, 1),
                            (13, 1, 2), (10, 5, 1)])
            for f in range(10, 420, 60):
                sc.opens[f] = 1
            for f in range(12, 420):
                r = rnd.random()
                if r < 0.14:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, DOWN, 0x4, 0x8)))
                elif r < 0.24:
                    sc.pads[f] = (OK, 0)
                elif r < 0.25:
                    sc.pads[f] = (CANCEL, 0)
                elif r < 0.255:
                    sc.pads[f] = (TRIANGLE, 0)
            for f in range(20, 420, 25):
                sc.skills[f] = rnd.choice((0, 0, 2))
            self.compare(sc, f"random {seed}")


    def test_chat_menu(self):
        R1, L1 = 0x8, 0x4
        # In the field: Skill Usage's orders, a strategy, the members' page.
        sc = Scenario(260)
        self.battle(sc)
        sc.party[1].chat = (1, 2, 3, 0, 5, 6)
        sc.opens[10] = 3
        sc.firsts.add(10)
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 41: (OK, 0)})
        sc.opens[70] = 3
        sc.pads.update({85: (0, R1), 93: (0, DOWN), 101: (OK, 0)})
        sc.opens[130] = 3
        sc.pads.update({145: (0, R1), 153: (0, DOWN), 161: (0, UP), 169: (0, L1), 177: (CANCEL, 0)})
        sc.opens[200] = 3
        sc.pads.update({215: (0, L1), 223: (OK, 0)})
        self.compare(sc, "chat field")
        # A dungeon floor: eight orders (the Sprite Ocarina's).
        sc = Scenario(80)
        self.battle(sc)
        sc.game = (2, 0, 0, 0)
        sc.opens[10] = 3
        sc.pads.update({25: (0, UP), 33: (OK, 0)})
        self.compare(sc, "chat dungeon")
        # In town: the town's two orders and the members.
        sc = Scenario(140)
        self.battle(sc)
        sc.game = (0, 0, 0, 0)
        sc.opens[10] = 3
        sc.pads.update({25: (0, DOWN), 33: (OK, 0)})
        sc.opens[70] = 3
        sc.pads.update({85: (0, R1), 93: (OK, 0)})
        self.compare(sc, "chat town")
        # Alone: "You cannot use this command unless in a party."
        sc = Scenario(80)
        self.party(sc, 1)
        sc.opens[10] = 3
        sc.pads[30] = (OK, 0)
        self.compare(sc, "chat alone")


    # --- CHAT's member (71 - 73) ----------------------------------------------------
    ORCA_SKILLS = (6, 150, 153, 170, 187)

    def chat_member(self, sc, member=0, skills=ORCA_SKILLS, game=(1, 1, 0, 0)):
        """The field battle; Orca's skills in the save; CHAT opened on the
        chat button and its Members page's `member`th row taken (71)."""
        R1 = 0x8
        self.battle(sc)
        for k in range(20):
            v = skills[k] if k < len(skills) else 0xFFFF
            sc.saves.append((0x1EC4 + 40 * 2 + 2 * k, 2, v))
        sc.game = game
        sc.opens[10] = 3
        sc.firsts.add(10)
        pages = 1 if game[0] == 0 else 2
        f = 25
        for _ in range(pages):
            sc.pads[f] = (0, R1)
            f += 8
        for _ in range(member):
            sc.pads[f] = (0, DOWN)
            f += 8
        sc.pads[f] = (OK, 0)
        return f + 15

    def seq(self, sc, f, steps):
        """Pads from frame f: (gap after, push, repeat) each."""
        for gap, push, rep in steps:
            sc.pads[f] = (push, rep)
            f += gap
        return f

    def test_chat_member_skill(self):
        R1 = 0x8
        # Designate Skill: Repth on Kite ("Orca, use Repth!"), then La Repth
        # (an area) on Orca himself, the rows' help on the way.
        for what, rows, target in (("repth", (0, 0), 0), ("la repth", (1,), 1)):
            sc = Scenario(260)
            f = self.chat_member(sc)
            f = self.seq(sc, f, [(8, 0, DOWN), (8, 0, DOWN), (8, 0, UP), (8, 0, UP), (15, OK, 0),
                                 (8, 0, R1), (8, 0, R1)])
            for r in rows:
                f = self.seq(sc, f, [(8, 0, DOWN if r else UP)])
            f = self.seq(sc, f, [(15, OK, 0)])
            for _ in range(target):
                f = self.seq(sc, f, [(8, 0, DOWN)])
            self.seq(sc, f, [(8, OK, 0)])
            self.compare(sc, f"chat member {what}")

    def test_chat_member_orders(self):
        # Designate Target (the enemies in reach: "Orca, attack Wolf!"),
        # then First Aid, Assemble and Standby.
        sc = Scenario(160)
        f = self.chat_member(sc)
        f = self.seq(sc, f, [(8, 0, DOWN), (8, 0, DOWN), (8, 0, DOWN), (15, OK, 0), (8, 0, DOWN), (8, 0, DOWN),
                             (8, 0, UP), (8, OK, 0)])
        self.compare(sc, "chat member target")
        for row in (2, 4, 5):
            sc = Scenario(110)
            f = self.chat_member(sc)
            self.seq(sc, f, [(8, 0, DOWN)] * row + [(8, OK, 0)])
            self.compare(sc, f"chat member order {row}")

    def test_chat_member_refusals(self):
        R1 = 0x8
        # The fallen BlackRose: her skill, aid and target refused ("Cannot
        # be used while dead."), her equipment open (63) and back.
        sc = Scenario(260)
        f = self.chat_member(sc, member=1)
        f = self.seq(sc, f, [(20, OK, 0), (30, OK, 0), (8, 0, DOWN), (8, 0, DOWN), (20, OK, 0), (30, OK, 0),
                             (8, 0, UP), (15, OK, 0), (25, CANCEL, 0), (20, CANCEL, 0), (15, CANCEL, 0)])
        self.compare(sc, "chat member dead")
        # Orca in a skill (chatMenuWarn 1), in act 12 and paralysed
        # (chatMenuWarn 2): Change Equipment refused.
        for what, busy, cond in (("skill", (150, 0), []), ("act", (0, 12), []), ("paralysis", None, [0] * 14 + [1])):
            sc = Scenario(130)
            f = self.chat_member(sc)
            sc.party[1].busy = busy
            sc.party[1].cond = cond + [0] * (16 - len(cond))
            self.seq(sc, f, [(8, 0, DOWN), (20, OK, 0), (30, OK, 0), (8, 0, DOWN)])
            self.compare(sc, f"chat member busy {what}")
        # No skill known; short of SP; no enemy in reach.
        sc = Scenario(130)
        f = self.chat_member(sc, skills=())
        self.seq(sc, f, [(20, OK, 0), (30, OK, 0), (8, 0, DOWN)])
        self.compare(sc, "chat member no skills")
        sc = Scenario(170)
        f = self.chat_member(sc)
        sc.party[1].sp = 5
        self.seq(sc, f, [(15, OK, 0), (8, 0, R1), (8, 0, R1), (20, OK, 0), (30, OK, 0), (8, 0, DOWN)])
        self.compare(sc, "chat member short of sp")
        sc = Scenario(130)
        f = self.chat_member(sc)
        for e in sc.sorted:
            e.dist = 5000.0
        self.seq(sc, f, [(8, 0, DOWN), (8, 0, DOWN), (8, 0, DOWN), (20, OK, 0), (30, OK, 0), (8, 0, DOWN)])
        self.compare(sc, "chat member no target")

    def test_chat_member_places(self):
        R1 = 0x8
        # Backing out: 73 to 72 to 71 to CHAT, and CHAT shut.
        sc = Scenario(160)
        f = self.chat_member(sc)
        self.seq(sc, f, [(15, OK, 0), (8, 0, R1), (8, 0, R1), (15, OK, 0), (8, 0, DOWN), (15, CANCEL, 0),
                         (15, CANCEL, 0), (15, CANCEL, 0), (15, CANCEL, 0)])
        self.compare(sc, "chat member back")
        # The field out of battle: three rows; town: Change Equipment only
        # (greyed while Orca is in a skill).
        sc = Scenario(120)
        f = self.chat_member(sc, game=(1, 0, 0, 0))
        self.seq(sc, f, [(8, 0, DOWN), (8, 0, DOWN), (8, 0, DOWN), (8, OK, 0)])
        self.compare(sc, "chat member field")
        for busy in (None, (150, 0)):
            sc = Scenario(120)
            f = self.chat_member(sc, game=(0, 0, 0, 0))
            sc.party[1].busy = busy
            self.seq(sc, f, [(8, 0, DOWN), (20, OK, 0), (30, OK, 0)])
            self.compare(sc, f"chat member town {busy}")

    def test_chat_member_random(self):
        # Random pads through CHAT and a member's menus in a field battle.
        for seed in range(1, 9):
            rnd = random.Random(seed)
            sc = Scenario(400)
            self.chat_member(sc, member=rnd.randrange(2))
            sc.party[1].sp = rnd.choice((5, 40))
            for f in range(12, 400):
                if f in sc.pads or f < 60:
                    continue
                r = rnd.random()
                if r < 0.14:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, DOWN, 0x4, 0x8)))
                elif r < 0.22:
                    sc.pads[f] = (OK, 0)
                elif r < 0.24:
                    sc.pads[f] = (CANCEL, 0)
            for f in range(100, 400, 70):
                sc.opens[f] = 3
            self.compare(sc, f"chat member random {seed}")

    # --- the Chaos Gate ------------------------------------------------------------
    def gate(self, sc, words=0x0C006006, order=(14, 19, 15), records=(2014027,), towns=0b11, marks=(14,)):
        """Kite and Orca at Mac Anu's Chaos Gate (cmndTarget), the save's
        keywords (wordList bits by ID: A Hidden, Expansive; B Passed Over,
        Forbidden; C Aqua Field, Holy Ground), the Word List, the history and
        the towns."""
        self.party(sc, 2)
        sc.target = Char(0x500, 0x2000, 16, 0, 0, 0, 0, b"Chaos Gate", tag=(256, 180))
        sc.game = (0, 0, 0, 0)
        sc.server = (0, 0, 0)
        sc.saves.append((0x523C, 4, words))
        for k in range(64):
            v = order[k] if k < len(order) else -1
            sc.saves.append((0x5278 + 2 * k, 2, v & 0xFFFF))
        for k in range(20):
            v = records[k] if k < len(records) else -1
            sc.saves.append((0x50AC + 4 * k, 4, v & 0xFFFFFFFF))
        sc.saves.append((0x2234, 2, towns))
        for a in marks:
            sc.saves.append((0x5048 + 4 * (a >> 5), 4, 1 << (a & 31)))
        sc.opens[10] = 28
        sc.firsts.add(10)

    def test_gate_random(self):
        sc = Scenario(300)
        self.gate(sc)
        # Random, Warp: the party leaves, the area change asked for.
        sc.pads.update({25: (OK, 0), 45: (OK, 0)})
        self.compare(sc, "gate random")
        # Random, Cancel, then out of the gate.
        sc = Scenario(120)
        self.gate(sc)
        sc.pads.update({25: (OK, 0), 45: (0, DOWN), 53: (OK, 0), 80: (CANCEL, 0)})
        self.compare(sc, "gate random cancel")

    def test_gate_new_keyword(self):
        sc = Scenario(340)
        self.gate(sc)
        sc.pads.update({25: (0, DOWN), 33: (OK, 0), 48: (0, DOWN), 56: (OK, 0), 70: (OK, 0), 84: (CANCEL, 0),
                        96: (0, DOWN), 104: (OK, 0), 118: (OK, 0), 132: (0, DOWN), 140: (OK, 0), 154: (OK, 0),
                        168: (OK, 0)})
        self.compare(sc, "gate new keyword")

    def test_gate_lists(self):
        # The Word List: the protected area 19 (the gate hack, 62), then
        # area 14 barred by the events.
        sc = Scenario(150)
        self.gate(sc)
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 41: (OK, 0), 56: (0, DOWN), 64: (OK, 0), 80: (OK, 0)})
        self.compare(sc, "gate word list hack")
        sc = Scenario(150)
        self.gate(sc)
        sc.area_codes[0] = (14, 0)
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 41: (OK, 0), 56: (OK, 0), 72: (OK, 0)})
        self.compare(sc, "gate word list barred")
        # The history: Warp.
        sc = Scenario(320)
        self.gate(sc)
        sc.pads.update({25: (0, UP), 33: (0, UP), 41: (OK, 0), 56: (OK, 0), 72: (0, DOWN), 80: (0, UP),
                        88: (OK, 0)})
        self.compare(sc, "gate history")

    def test_gate_refusal(self):
        """From Mutation on, with the bracelet off (eventStatus[40]), a
        story area on the gate's refusal list: the Word List's Warp turns to
        Kite's line, then the gate shuts."""
        if volume.NAME == "infection":
            self.skipTest("Mutation on")
        sc = Scenario(200)
        self.gate(sc, order=(16, 19, 15))
        sc.saves.append((0x64F8 + 40, 1, 1))
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 41: (OK, 0), 56: (OK, 0), 72: (OK, 0), 120: (OK, 0),
                        140: (OK, 0)})
        self.compare(sc, "gate refusal")

    def hack(self, sc, cores):
        """The Word List's area 19 (protected: two of core 12, M) through
        the gate hack, with `cores` of M in Key Items."""
        self.gate(sc)
        sc.saves.append((0x0CFC + 12, 1, cores))
        sc.watch += [(0x0CFC, 26), (0x65C8, 20), (0x6862, 2)]
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 41: (OK, 0), 56: (0, DOWN), 64: (OK, 0), 80: (OK, 0)})

    def test_gate_hack(self):
        # The screen from the red flashes to the cores' slots (363 on): two
        # M put in, a third refused, the ring turned both ways (an empty
        # slot's down refused), OK: the area opened, the cores gone.
        sc = Scenario(600)
        self.hack(sc, 3)
        sc.pads.update({370: (0, UP), 380: (0, UP), 390: (0, UP), 400: (0, LEFT), 420: (0, DOWN),
                        430: (0, RIGHT), 450: (OK, 0)})
        self.compare(sc, "gate hack")
        # One M only: OK refused, then cancel, back to the gate's town.
        sc = Scenario(520)
        self.hack(sc, 1)
        sc.pads.update({370: (0, UP), 380: (0, UP), 390: (OK, 0), 400: (CANCEL, 0)})
        self.compare(sc, "gate hack cancel")

    def test_gate_servers(self):
        # Other Servers: Cancel on the question, then Warp to town 1.
        sc = Scenario(320)
        self.gate(sc)
        sc.pads.update({25: (0, UP), 41: (OK, 0), 56: (OK, 0), 70: (OK, 0), 76: (0, DOWN), 88: (OK, 0),
                        100: (OK, 0), 114: (OK, 0), 128: (OK, 0)})
        self.compare(sc, "gate servers")

    def test_gate_empty(self):
        # Nothing listed: the warnings, back to the gate, out.
        sc = Scenario(200)
        self.gate(sc, order=(), records=(), towns=0b1, marks=())
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 41: (OK, 0), 60: (OK, 0), 80: (0, DOWN), 88: (OK, 0),
                        108: (OK, 0), 125: (0, DOWN), 133: (OK, 0), 150: (OK, 0), 170: (CANCEL, 0)})
        self.compare(sc, "gate empty")


    @unittest.skipUnless(LESSONS, NO_LESSONS)
    def test_gate_tutorial(self):
        # Event 2's gate: menu 78 (messages 30-33, only New Keyword), then
        # 79 (messages 35-56, the first words only, the party's leave).
        sc = Scenario(600)
        self.gate(sc, words=0x0C006006)
        sc.opens = {10: 78}
        sc.firsts = set()
        sc.chains = ([], [], [sc.target])
        sc.target = None
        for f in range(20, 600, 9):
            sc.pads[f] = (OK, 0)
        for f, p in self.GATE_TUTORIAL_PADS.items():
            sc.pads[f] = p
        self.compare(sc, "gate tutorial")

    GATE_TUTORIAL_PADS = {70: (0, DOWN)}

    # --- the party tutorial (event 2) -------------------------------------------
    def party_tutorial(self, sc):
        """Kite alone in Mac Anu, Orca (member 2) able to join: menu 75
        from the event, triangle, down to Party, then OK every 9 frames."""
        self.party(sc, 1)
        sc.saves.append((0x2220, 4, 0b101))
        sc.spc[2] = (b"Orca", 12, 345, 1500, 150, 80, 1)
        sc.joiners[2] = Char(0x101, 1, 2, 150, 80, 150, 80, b"Orca")
        sc.opens[10] = 75
        sc.pads[20] = (TRIANGLE, 0)
        for k in range(6):
            sc.pads[30 + 8 * k] = (0, DOWN)
        for f in range(80, sc.frames, 9):
            sc.pads[f] = (OK, 0)

    def lesson(self, sc):
        """Event 3's field: Kite and Orca, Kite's skills in the save."""
        sc.party[0] = Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite", width=40.0)
        sc.party[1] = Char(0x101, 1, 2, 30, 50, 150, 80, b"Orca", width=40.0, pos=(100.0, 50.0, 0.0))
        for k, v in enumerate((6, 7, 150, 170, 175)):
            sc.saves.append((0x1EC4 + 2 * k, 2, v))
        for k in range(5, 20):
            sc.saves.append((0x1EC4 + 2 * k, 2, 0xFFFF))
        sc.saves.append((0x6771, 1, 1))
        sc.game = (1, 0, 0, 0)
        sc.chains = ([sc.party[0], sc.party[1]], [], [])

    @unittest.skipUnless(LESSONS, NO_LESSONS)
    def test_skill_tutorial(self):
        # 80 (only Skills), 81 (only the third page's first skill), 82
        # (the target, skill 150).
        R1 = 0x8
        sc = Scenario(200)
        self.lesson(sc)
        sc.opens[10] = 80
        sc.pads.update({20: (TRIANGLE, 0), 28: (0, DOWN), 36: (OK, 0), 44: (0, UP), 52: (OK, 0), 70: (OK, 0),
                        78: (0, R1), 86: (0, R1), 94: (0, DOWN), 102: (OK, 0), 110: (0, UP), 118: (OK, 0),
                        140: (0, DOWN), 148: (OK, 0)})
        self.compare(sc, "skill tutorial")

    @unittest.skipUnless(LESSONS, NO_LESSONS)
    def test_chat_tutorial(self):
        # 83: square opens CHAT; only the heal order goes on.
        sc = Scenario(120)
        self.lesson(sc)
        sc.opens[10] = 83
        sc.pads.update({20: (SQUARE, 0), 40: (OK, 0), 48: (0, DOWN), 56: (OK, 0)})
        self.compare(sc, "chat tutorial")

    # --- the item tutorial (event 4) and an item got ------------------------------
    def box(self, sc, item, menu=84, full=0, have=None):
        """Kite at a treasure box (cmndTarget) holding `item` (-1: a draw);
        `full` kinds already in the bag (the item's own count `have`).
        Menu None: the lesson's box (84) where the volume has the lesson,
        else the action button's (32); both hand the item to GetItemMenu."""
        plain = menu is None and not LESSONS
        if menu is None:
            menu = 84 if LESSONS else 0x1020
        self.party(sc, 1)
        sc.target = Char(0x600, 0x4000, 5, 0, 0, 0, 0, b"Treasure", tag=(256, 200), item=item)
        sc.game = (1, 0, 0, 0)
        entries = [(10, k, 1) if k < 24 else (0, k - 4, 1) for k in range(full)]
        if have is not None:
            entries[0] = ((item >> 16) & 0xFF, item & 0xFFFF, have)
        self.items(sc, entries)
        sc.opens[10] = menu
        if plain:
            # Not trapped, as test_item_box's.
            sc.target.trap = (-1, 0)

    @unittest.skipUnless(LESSONS, NO_LESSONS)
    def test_item_tutorial(self):
        # 84: the box opened, the item into the bag.
        sc = Scenario(140)
        self.box(sc, (0 << 16) | 3)
        for f in range(20, 140, 9):
            sc.pads[f] = (OK, 0)
        self.compare(sc, "item tutorial")
        # 85 with a draw from the area's list.
        sc = Scenario(140)
        self.box(sc, -1, menu=85)
        sc.area_item = (2, 0, 3, 1, 0, 0)
        for f in range(20, 140, 9):
            sc.pads[f] = (OK, 0)
        self.compare(sc, "item tutorial draw")

    def test_item_box(self):
        # 32 (the action button on a box): opened, its item, into 29.
        sc = Scenario(140)
        self.box(sc, (0 << 16) | 3, menu=0x1020)
        sc.target.trap = (-1, 0)
        for f in range(20, 140, 9):
            sc.pads[f] = (OK, 0)
        self.compare(sc, "item box")
        # A draw from the area's lists: a disarmed box's (param[2] 3: the
        # danger list) and a plain one's (rand 0: the danger list too).
        for trap in (3, -1):
            sc = Scenario(140)
            self.box(sc, -1, menu=0x1020)
            sc.target.trap = (trap, 0)
            sc.area_item = (2, 0, 3, 1, 0, 0)
            for f in range(20, 140, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"item box draw {trap}")

    def test_trap_box(self):
        # 33 (a trapped box opened as it is): "Set off trap!" and the
        # trap's line, the button, then the box's item (or the dud list's).
        for skill, item in ((1, (0 << 16) | 3), (156, -1), (162, -1), (-1, (1 << 16) | 2), (7, -1)):
            sc = Scenario(180)
            self.box(sc, item, menu=0x1021)
            sc.target.trap = (0, skill)
            sc.area_item = (2, 0, 3, 1, 0, 0)
            for f in range(20, 180, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"trap box {skill}")

    def test_item_idol(self):
        # 38 (a Gott statue): idolCount up, three items through 67 and 29,
        # the idol's own last in the list, given first.
        for item in ((0 << 16) | 3, -1):
            sc = Scenario(300)
            self.box(sc, item, menu=0x1026)
            sc.target.types = 0x100000
            sc.watch.append((0x746E, 2))
            sc.area_item = (2, 0, 3, 1, 0, 0)
            for f in range(20, 300, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"item idol {item}")

    # --- the field objects (34 - 37, 39, 43) ---------------------------------------
    def obj(self, sc, types, name, menu, item=-1, trap=None, game=(2, 0, 0, 0)):
        """Kite and Orca in a dungeon at an object (cmndTarget) of base type
        `types`, opened as ccThGameCtrl opens it: openReqNum 0x1000 | menu,
        mode 1, firstTime."""
        self.party(sc, 2)
        sc.target = Char(0x600, types, 7, 0, 0, 0, 0, name, tag=(256, 200), item=item, trap=trap)
        sc.game = game
        sc.area_item = (2, 0, 3, 1, 0, 0)
        sc.opens[10] = 0x1000 | menu
        sc.modes[10] = 1
        sc.firsts.add(10)

    def test_item_obj(self):
        # 34 (a breakable): Kite breaks it; rand() % 3 0: its item (or a
        # draw from the area's list) into 29; else the menu shuts.
        for rng, item in ((0, (0 << 16) | 3), (0, -1), (1, -1), (5, (1 << 16) | 2)):
            sc = Scenario(160)
            self.obj(sc, 0x10000, b"Wooden Box", 34, item=item, trap=(-1, 0))
            sc.rng = rng
            sc.watch.append((0x7442, 2))
            for f in range(60, 160, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"item obj {rng} {item}")

    def test_trap_obj(self):
        # 35 (a trapped breakable): broken, "Set off trap!" and the trap's
        # line until OK, then its item; no trap or another skill: stuck.
        for skill in (1, 156, 162, -1, 7):
            sc = Scenario(200)
            self.obj(sc, 0x20000, b"Barrel", 35, item=-1, trap=(0, skill))
            sc.watch.append((0x7442, 2))
            for f in range(20, 200, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"trap obj {skill}")

    def test_symbol(self):
        # 36 (a symbol): "Received effects of <skill>!" until Check, then
        # shut; symbolCount up.
        for skill in (175, 150):
            sc = Scenario(120)
            self.obj(sc, 0x40000, b"Symbol", 36, trap=(0, skill))
            sc.watch.append((0x7444, 2))
            for f in range(20, 120, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"symbol {skill}")

    def test_virus(self):
        # 37 (a virus crystal): its item, or a Virus Core (key item 0).
        for item in (-1, (15 << 16) | 3):
            sc = Scenario(140)
            self.obj(sc, 0x80000, b"Virus Crystal", 37, item=item)
            for f in range(40, 140, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"virus {item}")

    def test_food(self):
        # 43 (a Grunty food, gimmick rows 22-37): taken (EntryAffect 11),
        # key item 26 + (row - 22) into 29, its foodCount up.
        for cid in (22, 29, 37):
            sc = Scenario(140)
            self.obj(sc, 0x800000, b"Golden Egg", 43)
            sc.target.id = cid
            sc.watch.append((0x7446, 32))
            for f in range(40, 140, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"food {cid}")

    def test_time_idol(self):
        # 39 (the Zeit statue): the time since the town against the
        # ranking (Balmung 2:12 ... NOG 4:45 in a fresh save): first place
        # with no best yet (five items), second after a best of third (one
        # item), fourth with a best of fourth (one, the rank kept), outside
        # the ranking.
        for secs, best in ((90, None), (140, 2), (200, 3), (400, None)):
            sc = Scenario(420)
            self.obj(sc, 0x200000, b"Zeit Statue", 39)
            sc.game_cnt = (0, 0, secs * 60 + 7)
            if best is not None:
                sc.saves.append((0x6774, 1, best))
            sc.watch.append((0x6774, 201))
            for f in range(40, 420, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"time idol {secs} {best}")

    # --- the Spring of Myst (40 - 42) -------------------------------------------
    def spring(self, sc, bag=(), level=2, bg=2, server=0):
        """Kite and Orca at the Spring of Myst (base type 0x400000), menu 40
        opened as ccThGameCtrl opens it; the bag's first slots `bag` ((cat,
        id, num) each, the rest empty), area level `level`, bgnum `bg`."""
        self.party(sc, 2)
        sc.target = Char(0x600, 0x400000, 20, 0, 0, 0, 0, b"Spring of Myst", tag=(256, 200))
        sc.game = (2, 0, 0, 8)
        sc.server = (server, 0, 0)
        sc.area_item = (4, 0, level, 0, 0, 0)
        sc.bgnum = bg
        for k in range(40):
            cat, cid, num = bag[k] if k < len(bag) else (-1, -1, 0)
            sc.saves += [(0x30 + 4 * k, 2, cid & 0xFFFF), (0x32 + 4 * k, 1, cat & 0xFF), (0x33 + 4 * k, 1, num)]
        sc.opens[10] = 40
        sc.watch.append((0x30, 160))
        sc.watch.append((0x7470, 4))

    def spring_states(self, sc, h=0x600, states=((80, 3), (170, 5), (270, 7)), gone=None):
        """The spring's ctrlFountain states as the affects bring them (its
        frames are piney-battle's gimetc, checked on their own)."""
        for f, st in states:
            sc.gim_acts[f] = (h, st)
        if gone:
            sc.gones[gone] = h

    def test_spring_leave(self):
        # 40: cancel; "No"; Yes with nothing to throw (41's line, then shut).
        for pads in ({30: (CANCEL, 0)}, {30: (0, DOWN), 40: (OK, 0)}):
            sc = Scenario(90)
            self.spring(sc, bag=((10, 1, 3),))
            sc.pads.update(pads)
            self.compare(sc, f"spring leave {sorted(pads)}")
        sc = Scenario(120)
        self.spring(sc)
        sc.pads.update({30: (OK, 0), 60: (OK, 0), 80: (OK, 0)})
        self.compare(sc, "spring nothing to throw")

    def test_spring_list(self):
        # 41: ten items of categories 0-9 among others (the scroll bar), the
        # cursor down past the eighth, triangle on one (64), back, cancel.
        bag = [(k % 10, k, 1 + k) for k in range(10)] + [(10, 2, 5), (11, 1, 1)]
        sc = Scenario(200)
        self.spring(sc, bag=bag)
        sc.pads.update({30: (OK, 0), 50: (0, DOWN), 58: (0, DOWN), 66: (0, UP), 74: (0, UP), 82: (0, UP),
                        100: (TRIANGLE, 0), 140: (CANCEL, 0), 170: (CANCEL, 0)})
        self.compare(sc, "spring list")

    def test_spring_throw(self):
        # 42 to the end: a known blade (0/1) under bgnum 0-4 (one or two rows
        # on, or back one), the Golden or the Silver Axe answered, an item the
        # spring does not know (both axes and the item back); "No" first
        # once (back to 41); level 4 on (talkNum 15, the other count).
        cases = [((0, 1, 1), 2, 2, 0, 0, False), ((0, 1, 1), 0, 2, 0, 0, False), ((6, 20, 1), 1, 2, 0, 0, False),
                 ((6, 20, 1), 4, 2, 0, 0, False), ((0, 3, 1), 2, 2, 1, 0, False), ((0, 3, 1), 2, 2, 2, 0, False),
                 ((0, 60, 1), 2, 2, 0, 0, False), ((2, 1, 1), 3, 5, 0, 1, True)]
        for item, bg, level, answer, server, no_first in cases:
            sc = Scenario(560)
            self.spring(sc, bag=(item,), level=level, bg=bg, server=server)
            t = 30
            sc.pads[t] = (OK, 0)
            t += 20
            sc.pads[t] = (OK, 0)
            if no_first:
                t += 20
                sc.pads[t] = (0, DOWN)
                t += 10
                sc.pads[t] = (OK, 0)
                t += 20
                sc.pads[t] = (OK, 0)
            t += 20
            sc.pads[t] = (OK, 0)
            for _ in range(answer):
                t += 70
                sc.pads[t] = (0, DOWN)
            base = t
            self.spring_states(sc, states=((base + 40, 3), (base + 120, 5), (base + 260, 7)), gone=base + 330)
            for f in range(base + 60, 560, 9):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"spring throw {item} bg {bg} level {level} answer {answer} server {server}")

    def test_get_item_99(self):
        # 99 already: "You already have 99", "Gave up".
        sc = Scenario(160)
        self.box(sc, (0 << 16) | 3, full=3, have=99, menu=None)
        for f in range(20, 160, 9):
            sc.pads[f] = (OK, 0)
        self.compare(sc, "get item 99")

    def test_replace_item(self):
        # A full bag: exchange; the list, an item discarded for the new one.
        sc = Scenario(200)
        self.box(sc, (1 << 16) | 3, full=40, menu=None)
        sc.pads.update({20: (OK, 0), 58: (OK, 0), 75: (0, DOWN), 91: (0, DOWN), 99: (OK, 0), 112: (0, DOWN),
                        120: (0, DOWN), 128: (OK, 0), 150: (OK, 0), 170: (OK, 0), 185: (OK, 0)})
        self.compare(sc, "replace item")

    def test_replace_give_up(self):
        # The exchange's list: Cancel, "Give up?" Cancel (back to the
        # list), triangle on equipment (64); then Cancel, "Give up?" OK.
        sc = Scenario(200)
        self.box(sc, (1 << 16) | 3, full=40, menu=None)
        sc.pads.update({20: (OK, 0), 58: (OK, 0), 70: (OK, 0), 112: (CANCEL, 0), 128: (0, DOWN), 136: (OK, 0),
                        160: (0, L1_), 170: (TRIANGLE, 0)})
        self.compare(sc, "replace item status")
        sc = Scenario(200)
        self.box(sc, (1 << 16) | 3, full=40, menu=None)
        sc.pads.update({20: (OK, 0), 58: (OK, 0), 70: (OK, 0), 112: (CANCEL, 0), 136: (OK, 0), 170: (OK, 0)})
        self.compare(sc, "replace item give up")
        # "Exchange?" Cancel: given up at once.
        sc = Scenario(140)
        self.box(sc, (1 << 16) | 3, full=40, menu=None)
        sc.pads.update({20: (OK, 0), 58: (OK, 0), 70: (0, DOWN), 80: (OK, 0), 110: (OK, 0)})
        self.compare(sc, "get item give up")

    @unittest.skipUnless(LESSONS, NO_LESSONS)
    def test_party_tutorial(self):
        # 75 (only Party), 76 (only Add), 77: Orca's lines, the question,
        # the call, AddMember, the panels in and out.
        sc = Scenario(520)
        self.party_tutorial(sc)
        self.compare(sc, "party tutorial")


if __name__ == "__main__":
    unittest.main()
