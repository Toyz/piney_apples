#!/usr/bin/env python3
"""The battle half of ccThGameCtrl (crates/piney-world/src/talk.rs) against
the game's own code run in the EE interpreter (tools/test_anim.py's
machine: crates/piney-eemu's when tools/eemu_rs.so is built).

The gamectrl_probe example answers each case; the same case goes through
the game's functions over command lists laid out in the interpreter's
memory the way ccEntryCmnd links them (Kite at the head of the party's,
or after the members as ccPlayer::AnimCtrl re-enters him).

  - inbattle    ccGame::SetInBattle (main 0x001676a0) from random inBattle,
                inBattleCnt and values;
  - inarea      ccCheckInAreaCmnd (gcmn 0x0051a000) and ccCheckTargetTypeId
                (0x005199e0) over random crowds - the party, enemies,
                objects, some down or falling, Kite listed or not - for the
                leader, a listed character or an unlisted point as the
                centre, Kite at the head of the party's list or further
                down, every type bit, modes 0-3 and 6 and distances
                negative, -2, 0 and ordinary;
  - select      ccSortCmnd, ccCheckTargetRange and ccSelectTarget in and
                out of a fight, on fields 1-12 and others, with conditions
                on the candidates and the leader, party members at a gate,
                crowds of more than eight in reach (the stack arrays'
                overlap), modes 0-2 with cmndTargetPriNum;
  - frames      ccThGameCtrl (gcmn 0x00517800) itself, frame by frame from
                its start, over random runs of a field fight: enemies
                closing in, falling and leaving the lists, the party
                following, conditions coming and going, the buttons, the
                menus opening and closing (a small model of ccThMenu),
                plAttack, the skill check before and after the normal
                attack's request, the events' menu_clear, the loading
                display, gate hacking, inBattleDist changes (battle_ready),
                delayed recoveries and the game over (its frame and the
                next, where it drops the target). Compared after every
                frame: cmndTarget, cmndTargetPrev, cmndTargetPriNum,
                cmndTargetFix, plAttack, Kite's pauseSW, inBattle,
                inBattleCnt, the menu asked for (openReqNum and mode, by
                firstTime), each ccSkillRequest, each recovery's
                EntryAffect, the cmndSortRoot chain and the game over.

What runs in Python in place of the game: ccEvent::CheckOperate (a mask),
ccSkillCheck and ccSkillRequest (the plan's skill value, and the value the
request leaves), checkCameraType, WORLD_MAN::GetDungeonType, ccMenuCtrl
(its fields), ccEntryRecoveryReq's slot write, the lists' links; the game
over runs its first two frames (its menus, message, chat, sound and task
stubbed, ccSpcManager's registry empty) and stops.

    python3 tools/test_gamectrl_rs.py

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import math
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

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "gamectrl_probe")

ONE = 0x3F800000
# gp globals and fixed objects (INF SLUS_202.67 + gcmn.prg).
CCSYS, SAVEDATA, GAME_P, WORLDMAN, EVENTMNG, MENU_P, LD = (inf_va(0x003788E0), inf_va(0x003789D8), inf_va(0x003789CC), inf_va(0x00378A7C),
                                                         inf_va(0x00378A94), inf_va(0x00378C88), inf_va(0x00378A78))
CMND_PC_ROOT, CMND_PC_LAST, CMND_ENE_ROOT, CMND_ENE_LAST = inf_va(0x00378C48), inf_va(0x00378C4C), inf_va(0x00378C50), inf_va(0x00378C54)
CMND_OBJ_ROOT, CMND_OBJ_LAST, CMND_SORT_ROOT = inf_va(0x00378C58), inf_va(0x00378C5C), inf_va(0x00378C60)
CMND_TARGET, CMND_PREV, CMND_FIX, CMND_PRI = inf_va(0x00378C64), inf_va(0x00378C68), inf_va(0x00378C6C), inf_va(0x00378C70)
COMPULSION, MENU_CLR_WAIT, GHO_FLAG = inf_va(0x00378C74), inf_va(0x00378C78), inf_va(0x00378CC0)
PLW, PARTY_MANAGER, RECOVERY_REQ = inf_va(0x00730300), inf_va(0x00730310), inf_va(0x0072EB10)
# Scratch memory for the interpreter.
SYS, SCRATCH, SAVE, GAME, WM, EV, MENU, LDS, AI, TSCB = (0x01000000, 0x01010000, 0x01020000, 0x01030000,
                                                         0x01031000, 0x01033000, 0x01034000, 0x01035000,
                                                         0x01036000, 0x01037000)
BASES, CHARS = 0x01200000, 0x01210000
GAME_OVER_TSCB, SPC_REGISTRY = 0x01037800, inf_va(0x0073035C)
# The buttons the harness assigns (saveData+0x8404 ..): action, personal,
# chat, option, map.
BUTTONS = (0x0001, 0x0002, 0x0004, 0x0008, 0x0010)


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def hexs(*v):
    return " ".join("%x" % (x & 0xFFFFFFFF) for x in v)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "gamectrl_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class Done(Exception):
    """Raised from a hook to leave the task."""


def list_of(flags):
    """The list ccEntryCmnd puts a character of these base flags on."""
    return 0 if flags & 7 else 1 if flags & 0xE0 else 2


class Game:
    """The game's code in the interpreter, with the characters of a case
    laid out at CHARS (0x200 bytes each, the base parameters at BASES)."""

    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value     # noqa: E731
        m = self.m
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(GAME_P, 4, GAME)
        m.mem[GAME:GAME + 0x88] = bytes(0x88)
        self.eye = False
        m.hooks[self.sym("checkCameraType__Fv")] = lambda mm, *a: 1 if self.eye else 3

    def ptr(self, k):
        return CHARS + 0x200 * k

    def clear(self, n):
        m = self.m
        m.mem[BASES:BASES + 0x40 * n] = bytes(0x40 * n)
        m.mem[CHARS:CHARS + 0x200 * n] = bytes(0x200 * n)

    def put_char(self, k, flags, ident, width, pos, cond=(0, 0, 0, 0, 0), act=0, dircz=0):
        m = self.m
        b, c = BASES + 0x40 * k, self.ptr(k)
        m.store(b + 8, 4, flags)
        m.store(b + 0xC, 2, ident)
        m.store(b + 0x1C, 4, width)
        m.store(c, 4, b)
        for i, v in enumerate(list(pos) + [ONE]):
            m.store(c + 0x40 + 4 * i, 4, v)
            m.store(c + 0x50 + 4 * i, 4, v)
        m.store(c + 0x68, 4, dircz)
        for off, v in zip((0x08, 0x1A, 0x1C, 0x1E, 0x24), cond):
            m.store(c + off, 2, v)
        m.store(c + 0xEE, 2, act)

    def link(self, lists):
        """The three command lists (roster indices, in order) as ccEntryCmnd
        leaves them."""
        m = self.m
        for (root, last), ks in zip(((CMND_PC_ROOT, CMND_PC_LAST), (CMND_ENE_ROOT, CMND_ENE_LAST),
                                     (CMND_OBJ_ROOT, CMND_OBJ_LAST)), lists):
            ptrs = [self.ptr(k) for k in ks]
            m.store(root, 4, ptrs[0] if ptrs else 0)
            m.store(last, 4, ptrs[-1] if ptrs else 0)
            for a, b in zip(ptrs, ptrs[1:] + [0]):
                m.store(a + 0xBC, 4, b)

    def call(self, name, args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name), list(args), limit=5_000_000) & 0xFFFFFFFF


# --- the cases ---------------------------------------------------------------

def rand_pos(rng, near=False):
    r = rng.choice([rng.uniform(0, 120), rng.uniform(80, 500)] if near else
                   [rng.uniform(0, 200), rng.uniform(100, 600), rng.uniform(0, 3500), rng.uniform(1500, 3000)])
    a = rng.uniform(-math.pi, math.pi)
    return r * math.cos(a), r * math.sin(a)


OBJ_FLAGS = [0x8, 0x10, 0x100, 0x200, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000, 0x400000, 0x800000, 0x2000000,
             0x0, 0x40000, 0x8000000]
ENE_FLAGS = [0x20, 0x20, 0x20, 0x40, 0xA0, 0x60, 0x20 | 0x8000]


def rand_cond(rng, p=0.15):
    if rng.random() > p:
        return (0, 0, 0, 0, 0)
    c = [0] * 5
    c[rng.randrange(5)] = rng.choice([1, 2, 3, 4, 5, 90, -1])
    if rng.random() < 0.3:
        c[rng.randrange(5)] = rng.randrange(1, 300)
    return tuple(c)


def inarea_cases(rng, n):
    cases = []
    tys = [1, 2, 4, 6, 7, 0x20, 0x40, 0x80, 0xE0, 0xE7, 0x8, 0x100, 0x2000, 0xFFFFFFFF, 0x21, 0x26, 0x2008]
    for _ in range(n):
        lz = fb(rng.uniform(0, 600))
        lx, ly = (fb(0.0), fb(0.0)) if rng.random() < 0.7 else tuple(fb(v) for v in rand_pos(rng))
        cands = []
        for _ in range(rng.randrange(0, 10)):
            kind = rng.random()
            flags = (rng.choice([6, 6, 2, 4, 1]) if kind < 0.3 else rng.choice(ENE_FLAGS) if kind < 0.75
                     else rng.choice(OBJ_FLAGS))
            x, y = rand_pos(rng)
            cands.append((flags, rng.choice([0, 1, 2, 7, 100, rng.randrange(0, 200)]),
                          rng.choice([0, 0, 0, 2, 3, 4, 5]), fb(x), fb(y), lz))
        # The lists as ccEntryCmnd orders them: by list, in entry order.
        cands.sort(key=lambda c: list_of(c[0]))
        kite = (rng.choice([7, 7, 7, 6, 1]), int(rng.random() < 0.85), rng.choice([0, 0, 1, 7]),
                rng.choice([0, 0, 0, 2, 3, 5]), rng.choice([0, 0, 0, 1, 2, 9]))
        ckind = rng.choice([0, 0, 1, 1, 2]) if cands else rng.choice([0, 2])
        cidx = rng.randrange(len(cands)) if cands else 0
        cx, cy = (fb(v) for v in rand_pos(rng))
        dist = rng.choice([fb(-1.0), fb(-2.0), 0, fb(2200.0), fb(rng.uniform(0, 3000)), fb(500.0), fb(150.0)])
        cases.append({"leader": (lx, ly, lz), "kite": kite, "cands": cands, "centre": (ckind, cidx, cx, cy),
                      "ty": rng.choice(tys), "mode": rng.choice([0, 0, 2, 6, 6, 1, 3]), "dist": dist,
                      "tid": rng.choice([0, 1, 2, 7, 100, kite[2]])})
    return cases


def select_cases(rng, n):
    cases = []
    for _ in range(n):
        lz = fb(rng.uniform(0, 600))
        leader = (fb(0.0), fb(0.0), lz, fb(rng.uniform(-math.pi, math.pi)), fb(45.0))
        crowd = rng.random() < 0.12
        cands = []
        for _ in range(rng.randrange(10, 20) if crowd else rng.randrange(1, 10)):
            kind = rng.random()
            flags = (rng.choice([6, 6, 2, 4]) if kind < 0.25 else rng.choice(ENE_FLAGS) if kind < 0.7
                     else rng.choice(OBJ_FLAGS))
            x, y = rand_pos(rng, near=crowd)
            cond = rand_cond(rng)
            cands.append((flags, fb(rng.choice([45.0, 60.0, 100.0, 30.0])), fb(x), fb(y), lz, cond,
                          rng.choice([0, 0, 0, 5, 12, 13, 14, 15])))
        cands.sort(key=lambda c: list_of(c[0]))
        kite = (int(rng.random() < 0.95), rand_cond(rng, 0.1))
        cases.append({"leader": leader, "kite": kite, "cands": cands, "eye": int(rng.random() < 0.2),
                      "mode": rng.choice([0, 1, 1, 2, 2]), "pri": rng.randrange(5),
                      "in_battle": rng.choice([0, 1, 1, 2]), "field": rng.choice([14, 14, 5, 12, 13, 0, -1, 1])})
    return cases


class Units:
    """SetInBattle, ccCheckInAreaCmnd with ccCheckTargetTypeId, and the
    selection, one call each."""

    def __init__(self):
        self.g = Game()

    def in_battle(self, ib, cnt, v):
        m, g = self.g.m, self.g
        m.store(GAME + 0x58, 4, ib)
        m.store(GAME + 0x5C, 4, cnt)
        g.call("SetInBattle__6ccGameFi", [GAME, v])
        return [m.load(GAME + 0x58, 4, True), m.load(GAME + 0x5C, 4, True)]

    def in_area(self, c):
        g, m = self.g, self.g.m
        cands = c["cands"]
        g.clear(len(cands) + 2)
        kflags, klisted, kid, kdead, kat = c["kite"]
        g.put_char(0, kflags, kid, fb(45.0), c["leader"], cond=(kdead, 0, 0, 0, 0))
        for i, (flags, ident, dead, x, y, z) in enumerate(cands):
            g.put_char(i + 1, flags, ident, fb(45.0), (x, y, z), cond=(dead, 0, 0, 0, 0))
        lists = [[], [], []]
        for i, cd in enumerate(cands):
            lists[list_of(cd[0])].append(i + 1)
        if klisted:
            lists[0].insert(min(kat, len(lists[0])), 0)
        g.link(lists)
        m.store(PARTY_MANAGER, 4, g.ptr(0))
        ckind, cidx, cx, cy = c["centre"]
        if ckind == 0:
            ch = g.ptr(0)
        elif ckind == 1:
            ch = g.ptr(cidx + 1)
        else:
            ch = g.ptr(len(cands) + 1)
            g.put_char(len(cands) + 1, 0x20, 0, fb(45.0), (cx, cy, c["leader"][2]))
        inside = g.call("ccCheckInAreaCmnd__FP6ccChariif", [ch, c["ty"], c["mode"]], fargs=[c["dist"]])
        found = g.call("ccCheckTargetTypeId__Fii", [c["ty"], c["tid"]])
        return [int(inside != 0), int(found != 0)]

    def select(self, c):
        g, m = self.g, self.g.m
        cands = c["cands"]
        g.clear(len(cands) + 1)
        g.eye = bool(c["eye"])
        lx, ly, lz, dircz, width = c["leader"]
        klisted, kcond = c["kite"]
        g.put_char(0, 7, 0, width, (lx, ly, lz), cond=kcond, dircz=dircz)
        for i, (flags, w, x, y, z, cond, act) in enumerate(cands):
            g.put_char(i + 1, flags, i + 1, w, (x, y, z), cond=cond, act=act)
        lists = [[0] if klisted else [], [], []]
        for i, cd in enumerate(cands):
            lists[list_of(cd[0])].append(i + 1)
        g.link(lists)
        m.store(PLW, 4, g.ptr(0))
        m.store(PARTY_MANAGER, 4, g.ptr(0))
        m.store(GAME + 0x58, 4, c["in_battle"])
        m.store(GAME + 0x24, 4, c["field"])
        m.store(CMND_SORT_ROOT, 4, 0)
        for root in (CMND_PC_ROOT, CMND_ENE_ROOT, CMND_OBJ_ROOT):
            g.call("ccSortCmnd__FP6ccChar", [m.load(root, 4)])
        ptrs = [g.ptr(i + 1) for i in range(len(cands))]
        # The port gets the candidates in list order.
        order = [k for ks in lists for k in ks if k != 0]
        dd = [[m.load(g.ptr(k) + 0xC4, 4), m.load(g.ptr(k) + 0xC8, 4)] for k in order]
        sorted_, a = [], m.load(CMND_SORT_ROOT, 4)
        while a:
            if a in ptrs:
                sorted_.append(order.index(ptrs.index(a) + 1))
            a = m.load(a + 0xC0, 4)
        ranges = [g.call("ccCheckTargetRange__FP6ccChar", [g.ptr(k)]) for k in order]
        m.store(CMND_PRI, 4, c["pri"])
        t = g.call("ccSelectTarget__Fi", [c["mode"]])
        return {"dd": dd, "sorted": sorted_, "range": ranges,
                "target": order.index(ptrs.index(t) + 1) if t in ptrs else -1, "pri": m.load(CMND_PRI, 4, True)}, order


def inarea_line(c):
    kflags, klisted, kid, kdead, kat = c["kite"]
    args = list(c["leader"]) + [kflags, klisted, kat, kid, kdead] + list(c["centre"]) + [c["ty"], c["mode"],
                                                                                         c["dist"], c["tid"],
                                                                                         len(c["cands"])]
    for flags, ident, dead, x, y, z in c["cands"]:
        args += [flags, ident, dead, x, y, z]
    return "inarea " + hexs(*args)


def select_line(c, order):
    cands = c["cands"]
    klisted, kcond = c["kite"]
    args = list(c["leader"]) + [klisted] + list(kcond) + [c["eye"], c["mode"], c["pri"], c["in_battle"],
                                                          c["field"], len(cands)]
    for k in order:
        flags, w, x, y, z, cond, act = cands[k - 1]
        args += [flags, w, x, y, z] + list(cond) + [act]
    return "select " + hexs(*args)


# --- the task, frame by frame ---------------------------------------------------

class Char:
    def __init__(self, flags, ident, width, pos, listed=True):
        self.flags, self.id, self.width = flags, ident, width
        self.pos = list(pos)
        self.cond = [0, 0, 0, 0, 0]
        self.act = 0
        self.listed = listed
        self.cond_left = 0


class CtrlRun:
    """ccThGameCtrl from its start over one random run; `frames` are the
    inputs each frame was given (for the probe) and `outs` what the game
    left after it."""

    def __init__(self, g, rng, n, style="fight"):
        self.g, self.m, self.rng, self.n = g, g.m, rng, n
        # "deadly": the party falls often; "busy": Kite's conditions and
        # plAttack change often, with a member along
        self.deadly, self.busy = style == "deadly", style == "busy"
        r = rng.random()
        self.area, self.field = ((1, 14) if r < 0.65 else (1, rng.choice([5, 12, 13, 67, 20])) if r < 0.8
                                 else (2, rng.choice([14, 3])) if r < 0.93 else (0, -1))
        self.dtype = rng.choice([0, 0, 8, 9])
        self.dist = fb(2200.0) if rng.random() < 0.8 else rng.choice([fb(-1.0), fb(-2.0), fb(500.0)])
        z = fb(rng.uniform(0, 600))
        self.z = z
        chars = [Char(7, 0, fb(45.0), (0, 0, z))]
        members = rng.choice([1, 2]) if self.busy else rng.choice([0, 1, 1, 2])
        for i in range(members):
            chars.append(Char(6, i + 1, fb(45.0), self.near_pos(rng, 150, 400)))
        crowd = rng.random() < 0.1
        for i in range(rng.randrange(8, 14) if crowd else rng.randrange(1, 8)):
            c = Char(rng.choice(ENE_FLAGS), 100 + i, fb(rng.choice([45.0, 60.0, 100.0])),
                     self.near_pos(rng, 50, 250) if crowd else self.near_pos(rng, 300, 3200))
            chars.append(c)
        for i in range(rng.randrange(0, 4)):
            chars.append(Char(rng.choice(OBJ_FLAGS), 200 + i, fb(rng.choice([45.0, 100.0])),
                              self.near_pos(rng, 0, 2000)))
        self.chars = chars
        self.party = [0] + [k for k in range(1, members + 1)]
        # the lists, Kite at the head of the party's
        self.lists = [[k for k, c in enumerate(chars) if list_of(c.flags) == li] for li in range(3)]
        self.dircz = rng.uniform(-math.pi, math.pi)
        self.pow_l = 0
        self.eye = False
        self.forbid = self.fce = 0
        self.pl_attack = 0
        self.pause = 0
        self.mask = 0x7E00
        self.skill_run = 0
        self.skill_other = 0
        self.menu_open = None
        self.opened_req = False
        self.control = 0
        self.act_left = 0
        self.gt_left = 0
        self.frames, self.outs, self.ins = [], [], []
        self.attacks, self.heals = [], []
        self.breaths = 0
        self.skill = self.after = 0
        # the game over: 1 begun this frame, 2 in its loop
        self.over = 0

    @staticmethod
    def near_pos(rng, lo, hi):
        r, a = rng.uniform(lo, hi), rng.uniform(-math.pi, math.pi)
        return [r * math.cos(a), r * math.sin(a)]

    # --- the world moving between frames ----------------------------------
    def evolve(self, k):
        rng = self.rng
        chars = self.chars
        self.dircz += rng.uniform(-0.3, 0.3) if rng.random() < 0.5 else 0
        if self.dircz > math.pi:
            self.dircz -= 2 * math.pi
        if self.dircz < -math.pi:
            self.dircz += 2 * math.pi
        for i, c in enumerate(chars[1:], 1):
            x, y = c.pos[0], c.pos[1]
            d = math.hypot(x, y) or 1.0
            if c.flags & 0xE0:
                step = rng.uniform(-40, 90) if d > 90 else rng.uniform(-30, 30)
                x, y = x - step * x / d + rng.uniform(-15, 15), y - step * y / d + rng.uniform(-15, 15)
            elif c.flags & 7:
                x, y = x + rng.uniform(-30, 30), y + rng.uniform(-30, 30)
                if math.hypot(x, y) > 500:
                    x, y = x * 0.9, y * 0.9
            c.pos[0], c.pos[1] = x, y
            # conditions come and go
            if c.cond_left > 0:
                c.cond_left -= 1
                if c.cond_left == 0:
                    c.cond[1:] = [0, 0, 0, 0]
            elif rng.random() < 0.01:
                c.cond[rng.randrange(1, 5)] = rng.randrange(1, 200)
                c.cond_left = rng.randrange(5, 60)
            # falling, lying, reviving
            if c.cond[0] in (2, 3, 4) and rng.random() < 0.1:
                c.cond[0] = rng.choice([3, 4, 5, 0]) if c.flags & 7 else 3
            elif c.cond[0] == 5 and rng.random() < 0.2:
                c.cond[0] = 0
            elif c.cond[0] == 0 and rng.random() < (0.006 if c.flags & 0xE0 else 0.03 if self.deadly else 0.004):
                c.cond[0] = 2
            if c.flags & 4:
                c.act = rng.choice([12, 13, 14]) if rng.random() < 0.01 else (c.act if rng.random() < 0.9 else 0)
            # off the lists and back (at the end of its list)
            if c.listed and rng.random() < (0.05 if c.cond[0] else 0.004):
                c.listed = False
                self.lists[list_of(c.flags)].remove(i)
            elif not c.listed and rng.random() < 0.03:
                c.listed = True
                if c.flags & 0xE0:
                    c.cond[0] = 0
                    c.pos[:2] = self.near_pos(rng, 300, 1500)
                self.lists[list_of(c.flags)].append(i)
        kite = chars[0]
        if kite.listed and rng.random() < 0.004:
            kite.listed = False
            self.lists[0].remove(0)
        elif not kite.listed and rng.random() < 0.1:
            # MenuClr and Wakeup enter him first; AnimCtrl at the end of an
            # arrival, after the members
            kite.listed = True
            self.lists[0].insert(0 if rng.random() < 0.5 else len(self.lists[0]), 0)
        if kite.cond_left > 0:
            kite.cond_left -= 1
            if kite.cond_left == 0:
                kite.cond[1:] = [0, 0, 0, 0]
        elif rng.random() < (0.06 if self.busy else 0.008):
            kite.cond[rng.randrange(1, 5)] = rng.randrange(1, 200)
            kite.cond_left = rng.randrange(5, 40)
        if kite.cond[0] == 0 and rng.random() < (0.03 if self.deadly or self.busy else 0.003):
            kite.cond[0] = 2
        elif kite.cond[0] in (2, 3, 4) and rng.random() < (0.1 if self.busy else 0.03):
            kite.cond[0] = rng.choice([5, 0])
        elif kite.cond[0] == 5 and rng.random() < 0.2:
            kite.cond[0] = 0
        if self.act_left > 0:
            self.act_left -= 1
            if self.act_left == 0:
                kite.act = 0
        elif rng.random() < 0.01:
            kite.act, self.act_left = rng.choice([12, 13]), rng.randrange(1, 10)
        # the events' manual control: now and then, for a while
        if rng.random() < (0.15 if self.control else 0.005):
            self.control ^= 1
        # the pad
        if rng.random() < 0.5:
            self.pow_l = rng.choice([0, rng.randrange(0, 64), rng.randrange(64, 256)])
        if rng.random() < 0.02:
            self.eye = not self.eye
        push = 0
        for bit, p in ((1, 0.4), (2, 0.03), (4, 0.03), (8, 0.02)):
            if rng.random() < p:
                push |= bit
        self.push = push
        # the event's lock on operations
        self.mask = 0x7E00
        if rng.random() < 0.1:
            self.mask &= ~(1 << rng.randrange(9, 15))
        if rng.random() < 0.02:
            self.forbid ^= 1
            self.fce = rng.randrange(2)
        # the skill system
        if self.skill_run > 0:
            self.skill_run -= 1
            self.skill = 1
        elif self.skill_other > 0:
            self.skill_other -= 1
            self.skill = self.skill_id
        else:
            self.skill = 0
            if rng.random() < 0.01:
                self.skill_other, self.skill_id = rng.randrange(3, 30), rng.choice([2, 6, 150, 233])
        # what ccSkillCheck answers once the normal attack is asked for: 1
        # when it went through, 0 when it did not, now and then another id
        self.after = rng.choices([1, 0, 6], [0.8, 0.15, 0.05])[0]
        # ccMenu: the pads' attack flag and pauseSW kept, now and then set
        if rng.random() < (0.25 if self.busy else 0.03):
            self.pl_attack = 1
        elif rng.random() < 0.01:
            self.pl_attack = 0
        if rng.random() < 0.1:
            self.pause ^= 1
        # the other readers
        self.load_disp = int(rng.random() < 0.02)
        if self.gt_left > 0:
            self.gt_left -= 1
        elif rng.random() < 0.005:
            self.gt_left = rng.randrange(1, 8)
        self.menu_clear = int(rng.random() < 0.015)
        self.compul = int(rng.random() < (0.004 if self.deadly else 0.0015))
        if rng.random() < 0.01:
            self.dist = rng.choice([fb(-1.0), fb(-2.0), 0, fb(500.0), fb(2200.0), fb(3000.0)])
        if rng.random() < 0.05:
            self.recov = [(rng.randrange(len(chars)), rng.choice([rng.randrange(1, 200), 0x12345, -5, 0]))
                          for _ in range(rng.randrange(1, 3))]
        else:
            self.recov = []

    def menu_model(self):
        """What ccThMenu leaves in the fields the task reads: a menu the
        task asked for opens (or not) and closes after a few frames; now
        and then the stream menu (74) is asked for, Data Drain (66) runs or
        the menu is changing (menuNext differs)."""
        rng, m = self.rng, self.m
        if self.opened_req:
            self.opened_req = False
            req = m.load(MENU + 0x14, 2, True)
            if rng.random() < 0.7:
                self.menu_open = [req, rng.randrange(1, 7)]
        menu = menu_next = -1
        if self.menu_open:
            menu = menu_next = self.menu_open[0]
            self.menu_open[1] -= 1
            if self.menu_open[1] <= 0:
                self.menu_open = None
        elif rng.random() < 0.015:
            self.menu_open = [66, rng.randrange(1, 10)]
        elif rng.random() < 0.02:
            menu_next = 5
        open_req = -1
        if rng.random() < 0.03:
            open_req = rng.choice([74, 0x104A, 75])
        m.store(MENU + 6, 2, menu)
        m.store(MENU + 8, 2, menu_next)
        m.store(MENU + 0x14, 2, open_req)
        m.store(MENU + 0xF8, 2, 0)
        return menu, menu_next, open_req

    def set_inputs(self, k):
        g, m = self.g, self.m
        self.evolve(k)
        menu, menu_next, open_req = self.menu_model()
        chars = self.chars
        kite = chars[0]
        for i, c in enumerate(chars):
            pos = (fb(c.pos[0]), fb(c.pos[1]), self.z)
            ptr = g.ptr(i)
            for j, v in enumerate(list(pos) + [ONE]):
                m.store(ptr + 0x50 + 4 * j, 4, v)
                m.store(ptr + 0x40 + 4 * j, 4, v)
            for off, v in zip((0x08, 0x1A, 0x1C, 0x1E, 0x24), c.cond):
                m.store(ptr + off, 2, v)
            m.store(ptr + 0xEE, 2, c.act)
        m.store(g.ptr(0) + 0x68, 4, fb(self.dircz))
        byte = m.load(g.ptr(0) + 0xE0, 1)
        m.store(g.ptr(0) + 0xE0, 1, (byte & ~1) | self.pause)
        m.store(AI, 1, self.control)
        g.link(self.lists)
        # ccPartyManager: Kite, then the members
        slots = self.party + [None] * (3 - len(self.party))
        for s, k2 in enumerate(slots):
            m.store(PARTY_MANAGER + 4 * s, 4, g.ptr(k2) if k2 is not None else 0)
        m.store(PARTY_MANAGER + 0x18, 4, len(self.party))
        # the pad, the menus, the game
        m.store(SYS + 0x2D0, 4, sum(b for bit, b in zip((1, 2, 4, 8), BUTTONS) if self.push & bit))
        m.store(SYS + 0x2B0, 1, self.pow_l)
        g.eye = self.eye
        m.store(MENU + 0xF4, 2, self.pl_attack)
        m.store(MENU + 0xFE, 2, self.forbid)
        m.store(MENU + 0x100, 2, self.fce)
        m.store(GAME + 0x14, 4, self.area)
        m.store(GAME + 0x24, 4, self.field)
        m.store(GAME + 0x60, 4, self.dist)
        m.store(LDS + 0x19C, 4, self.load_disp)
        m.store(GHO_FLAG, 4, int(self.gt_left > 0))
        m.store(COMPULSION, 4, self.compul)
        if self.menu_clear:
            m.store(MENU_CLR_WAIT, 4, 2)
        for idx, amount in self.recov:
            for s in range(16):
                a = RECOVERY_REQ + 12 * s
                if m.load(a, 4) == 0:
                    m.store(a, 4, g.ptr(idx))
                    m.store(a + 4, 4, amount)
                    m.store(a + 8, 4, 10)
                    break
        # the probe's line
        menu_type = menu if menu == menu_next else 88
        idle = int(menu_type == -1 and (open_req & 0xFFF) != 74)
        args = [self.pow_l, int(self.eye), self.push, idle, self.forbid, self.fce, self.pl_attack, menu,
                int(menu_type == -1), self.load_disp, int(self.gt_left > 0), self.compul, self.menu_clear,
                self.mask, self.skill, self.after, self.area, self.field, self.dtype, self.dist, self.pause]
        at = self.lists[0].index(0) if kite.listed else 0
        args += [int(kite.listed), at, kite.id] + kite.cond + [kite.act, self.control, 0, 0, self.z,
                                                             fb(self.dircz), kite.width]
        args.append(len(self.party))
        for s in range(3):
            if s < len(self.party):
                c = chars[self.party[s]]
                args += [1, self.party[s], fb(c.pos[0]), fb(c.pos[1]), self.z, c.cond[0]]
            else:
                args += [0, 0, 0, 0, 0, 0]
        args.append(len(self.recov))
        for idx, amount in self.recov:
            args += [idx, amount]
        order = [k2 for ks in self.lists for k2 in ks if k2 != 0]
        args.append(len(order))
        for k2 in order:
            c = chars[k2]
            args += [k2, c.flags, c.id, c.width, fb(c.pos[0]), fb(c.pos[1]), self.z] + c.cond + [c.act]
        self.frames.append("ctrl_frame " + hexs(*args))
        self.ins.append({"pl_attack": self.pl_attack, "pause": self.pause, "action": self.push & 1,
                         "held": any(kite.cond[1:]), "dead": kite.cond[0] != 0,
                         "party": [chars[k].cond[0] for k in self.party], "compul": self.compul,
                         "waits": [self.menu_clear, self.gt_left > 0, self.load_disp, self.skill >= 2,
                                   kite.act in (12, 13), self.control, not kite.listed]})

    def idx(self, p):
        return (p - CHARS) // 0x200 if p else -1

    def read_out(self):
        m = self.m
        first = m.load(MENU + 0xF8, 2)
        req = [m.load(MENU + 0x14, 2, True), m.load(MENU + 0x16, 2, True)] if first == 1 else []
        self.opened_req = first == 1
        sorted_, a = [], m.load(CMND_SORT_ROOT, 4)
        while a:
            sorted_.append(self.idx(a))
            a = m.load(a + 0xC0, 4)
        self.pl_attack = m.load(MENU + 0xF4, 2)
        self.pause = m.load(self.g.ptr(0) + 0xE0, 1) & 1
        out = {"target": self.idx(m.load(CMND_TARGET, 4)), "prev": self.idx(m.load(CMND_PREV, 4)),
               "pri": m.load(CMND_PRI, 4, True), "fix": m.load(CMND_FIX, 4), "pl_attack": self.pl_attack,
               "pause": self.pause, "in_battle": m.load(GAME + 0x58, 4, True),
               "cnt": m.load(GAME + 0x5C, 4, True), "req": req, "attack": self.attacks, "heals": self.heals,
               "over": int(self.over == 1), "sorted": sorted_}
        for k in self.attacks:
            if self.after:
                self.skill_run = self.rng.randrange(2, 16)
        self.attacks, self.heals = [], []
        return out

    def run(self):
        g, m, sym = self.g, self.m, self.g.sym
        g.clear(len(self.chars))
        for i, c in enumerate(self.chars):
            g.put_char(i, c.flags, c.id, c.width, (fb(c.pos[0]), fb(c.pos[1]), self.z))
        m.store(g.ptr(0) + 0x128, 4, AI)
        m.store(PLW, 4, g.ptr(0))
        m.store(PARTY_MANAGER, 4, g.ptr(0))
        m.store(SAVEDATA, 4, SAVE)
        for off, b in zip((0x8404, 0x8406, 0x8408, 0x840A, 0x840C), BUTTONS):
            m.store(SAVE + off, 2, b)
        m.store(WORLDMAN, 4, WM)
        m.store(MENU_P, 4, MENU)
        m.mem[MENU:MENU + 0x240] = bytes(0x240)
        m.store(EVENTMNG, 4, EV)
        m.store(LD, 4, LDS)
        m.mem[GAME:GAME + 0x88] = bytes(0x88)
        m.store(MENU_CLR_WAIT, 4, 0)
        m.store(GHO_FLAG, 4, 0)
        # ccSpcManager's registry, which the game over walks: empty
        m.mem[SPC_REGISTRY:SPC_REGISTRY + 0x2C * 5] = bytes(0x2C * 5)

        def breath(mm, tscb, n, *_):
            self.breaths += 1
            if self.breaths == 1:
                return 0
            if self.breaths >= 3:
                self.outs.append(self.read_out())
            if self.over == 2:
                raise Done("over")
            if self.over == 1:
                self.over = 2
            k = self.breaths - 2
            if k >= self.n:
                raise Done("done")
            self.set_inputs(k)
            return 0

        def over(mm, *_):
            self.over = 1
            return 0

        def request(mm, ch, t, sid, *_):
            assert ch == g.ptr(0) and sid == 1
            self.attacks.append(self.idx(t))
            self.skill = self.after
            return 0

        def affect(mm, this, ch, t, p0, *_):
            self.heals.append([self.idx(this), struct.unpack("<h", struct.pack("<H", p0 & 0xFFFF))[0]])
            assert this == ch and t & 0xFFFF == 7
            return 0

        hooks = {
            "Breath__6ccTscbFi": breath,
            "ccInitFlyFont__Fv": lambda mm, *a: 0,
            "CheckOperate__7ccEventFii": lambda mm, this, n, *a: (self.mask >> n) & 1,
            "ChangeMapMode__9WORLD_MANFv": lambda mm, *a: 0,
            "GetDungeonType__9WORLD_MANFv": lambda mm, *a: self.dtype,
            "ccSkillCheck__FP6ccChar": lambda mm, *a: self.skill,
            "ccSkillRequest__FP6ccCharP6ccChari": request,
            "EntryAffect__6ccCharFP6ccCharssss": affect,
            "CloseMenu__10ccMenuCtrlFv": over,
            # the rest of the game over's first two frames
            "Close__9ccMessageFv": lambda mm, *a: 0,
            "ccClearConditionAllEnemy__Fv": lambda mm, *a: 0,
            "CloseChat__9ccChatMsgFv": lambda mm, *a: 0,
            "ccSndGameOver__Fv": lambda mm, *a: 0,
            "ccStartThread__FPFPv_vii": lambda mm, *a: GAME_OVER_TSCB,
        }
        saved = {}
        for name, h in hooks.items():
            a = sym(name)
            saved[a] = m.hooks.get(a)
            m.hooks[a] = h
        try:
            m.call(sym("ccThGameCtrl__FP6ccTscb"), [TSCB], limit=500_000_000)
        except Done:
            pass
        finally:
            for a, h in saved.items():
                if h is None:
                    del m.hooks[a]
                else:
                    m.hooks[a] = h
        return self.frames[:len(self.outs)], self.outs, self.ins[:len(self.outs)]


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class GameCtrlAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.units = Units()

    def test_set_in_battle(self):
        """ccGame::SetInBattle from random states and values."""
        rng = random.Random(61)
        cases = [(rng.choice([0, 1, 2, rng.randrange(-2, 5)]), rng.choice([0, 0, 1, 60, rng.randrange(-3, 70)]),
                  rng.choice([0, 1, 1, 2, rng.randrange(-2, 5)])) for _ in range(2000)]
        got = ask(["inbattle " + hexs(*c) for c in cases])
        bad = 0
        for c, port in zip(cases, got):
            want = self.units.in_battle(*c)
            if want != port:
                bad += 1
                if bad < 4:
                    print("inbattle", c, "game", want, "port", port)
        print("inbattle: %d cases, %d differ" % (len(cases), bad))
        self.assertEqual(bad, 0)

    def test_in_area(self):
        """ccCheckInAreaCmnd and ccCheckTargetTypeId over random crowds."""
        rng = random.Random(62)
        cases = inarea_cases(rng, 3000)
        got = ask([inarea_line(c) for c in cases])
        bad, hits = 0, [0, 0]
        for c, port in zip(cases, got):
            want = self.units.in_area(c)
            hits[0] += want[0]
            hits[1] += want[1]
            if want != port:
                bad += 1
                if bad < 4:
                    print("inarea", c, "game", want, "port", port)
        print("inarea: %d cases, %d differ; %d in the area, %d type-id hits" % (len(cases), bad, *hits))
        self.assertEqual(bad, 0)
        self.assertGreater(hits[0], 300)
        self.assertGreater(hits[1], 300)

    def test_select(self):
        """ccSortCmnd, ccCheckTargetRange and ccSelectTarget in and out of a
        fight, with conditions, gates and crowds."""
        rng = random.Random(63)
        cases = select_cases(rng, 3000)
        wants, lines = [], []
        for c in cases:
            want, order = self.units.select(c)
            wants.append(want)
            lines.append(select_line(c, order))
        got = ask(lines)
        bad = picked = 0
        for c, want, port in zip(cases, wants, got):
            picked += want["target"] >= 0
            if want != port:
                bad += 1
                if bad < 4:
                    print("select", c, "\n game", want, "\n port", port)
        print("select: %d cases, %d differ; %d with a target" % (len(cases), bad, picked))
        self.assertEqual(bad, 0)
        self.assertGreater(picked, 800)

    def check_runs(self, rng, runs, lengths, style="fight"):
        """Runs of ccThGameCtrl and the port side by side: the frames, the
        runs that differ, and how often each path was taken."""
        g = Game()
        total = bad = 0
        seen = dict.fromkeys(("attack", "attack refused", "action menu", "button menu", "cleared by the tail",
                              "cleared held", "cleared down", "pause on", "pause off", "in battle",
                              "battle over", "recovery", "enemy target", "game over", "annihilated",
                              "compulsion", "menu_clear", "ghoFlag", "loading", "skill running", "at a gate",
                              "manual control", "Kite off the lists"), 0)
        for run in range(runs):
            r = CtrlRun(g, rng, rng.choice(lengths), style)
            frames, outs, ins = r.run()
            got = ask(["ctrl_start"] + frames)[1:]
            for i, (want, port, x) in enumerate(zip(outs, got, ins)):
                total += 1
                seen["attack"] += len(want["attack"])
                seen["attack refused"] += bool(want["attack"]) and not want["pl_attack"]
                if want["req"]:
                    seen["action menu" if want["req"][0] >= 21 else "button menu"] += 1
                if x["pl_attack"] and not want["pl_attack"]:
                    seen["cleared held" if x["held"] else "cleared down" if x["dead"] else
                         "cleared by the tail"] += 1
                seen["pause on"] += not x["pause"] and want["pause"]
                seen["pause off"] += x["pause"] and not want["pause"]
                seen["in battle"] += want["in_battle"] == 1
                seen["battle over"] += want["in_battle"] == 2
                seen["recovery"] += len(want["heals"])
                seen["enemy target"] += want["target"] >= 0 and bool(r.chars[want["target"]].flags & 0xE0)
                for k, w in zip(("menu_clear", "ghoFlag", "loading", "skill running", "at a gate",
                                 "manual control", "Kite off the lists"), x["waits"]):
                    seen[k] += bool(w)
                if want["over"]:
                    seen["game over"] += 1
                    seen["compulsion" if x["compul"] else "annihilated"] += 1
                if want != port:
                    bad += 1
                    if bad < 4:
                        print("run %d frame %d\n in   %s\n game %s\n port %s" % (run, i, frames[i], want, port))
                    break
        return total, bad, seen

    def test_frames(self):
        """ccThGameCtrl frame by frame over random runs of a fight."""
        total, bad, seen = self.check_runs(random.Random(64), 500, [150, 300, 450])
        print("frames: 500 runs, %d frames, %d runs differ; %s" % (total, bad, seen))
        self.assertEqual(bad, 0)
        for k in ("attack", "cleared by the tail", "enemy target", "in battle"):
            self.assertGreater(seen[k], 1000, k)
        for k, v in seen.items():
            self.assertGreater(v, 0, k)

    def test_game_over(self):
        """Short runs where the party falls: the game over's frame."""
        total, bad, seen = self.check_runs(random.Random(65), 1500, [40, 80, 120], "deadly")
        print("game over: 1500 runs, %d frames, %d runs differ; %s" % (total, bad, seen))
        self.assertEqual(bad, 0)
        self.assertGreater(seen["game over"], 1000)
        self.assertGreater(seen["compulsion"], 0)

    def test_conditions(self):
        """Runs where Kite is held, falls and gets up with plAttack set:
        the buttons' conditions."""
        total, bad, seen = self.check_runs(random.Random(66), 600, [150, 300], "busy")
        print("conditions: 600 runs, %d frames, %d runs differ; %s" % (total, bad, seen))
        self.assertEqual(bad, 0)
        for k in ("cleared held", "cleared down", "cleared by the tail"):
            self.assertGreater(seen[k], 1000, k)


if __name__ == "__main__":
    unittest.main()
