#!/usr/bin/env python3
"""crates/piney-battle's enemy AI (src/enemy_ai.rs) against the game's own
enemy.cpp run in tools/eemu.py.

Each check lays a random scene out in the interpreter's memory: party
members and enemies (ccChar and ccEnemy at SCN + 0x1000 * i, an enemy's
ccEnemyParam inside it at +0x1d0, its base at +0x800) on the command lists,
the acting enemy's enemyTbl row and spells changed at random (and put
back), eventMng, pgRideFlag, the player, both generators (newlib's rand()
and ccRand's Mersenne Twister, mt/mti), then runs one ccEnemy function
natively and sends the same case to battle_probe. Compared: the return,
every character (conditions, HP/SP, stats, positions), every enemy's
ccEnemy fields, the command lists, both generators, the enemy book's
record, and the calls the function makes in order (EntryAffect,
effSkillStart, _ccSkillRequest, ccSkillDamage, ccExpDistributor,
entryEnemyObject, entryObject, effAfterDrain, CalcReal's). The runtime's
functions are stubbed: ccCheckActiveEnemy answers a random count, the
player's frame is the identity, the model and animation set-up of
initEnemyCCS returns nothing, ccCheckDustColor a random colour.

    python3 tools/test_battle_enemy_ai_rs.py            the unit tests
    python3 tools/test_battle_enemy_ai_rs.py bulk N     N cases of every check

Checks (ENEMY_AI maps a name to fn(checks, rnd) -> (request, game, label)):
  think            ccEnemy::defaultThink (0x00434ac0)
  interrupt        ccEnemy::interruptThink (0x004353f0)
  begin            ccEnemy::main (0x00432cd0) to think(), with the
                   rest of the frame stubbed
  routine          ccEnemy::routineEnemy (0x00433990)
  check_enemy      checkEnemy (0x00433710), checkCrisisRate (0x00433610)
  set_act          setAct (0x00433470)
  select_attack    selectAttack (0x00436940)
  skill_list       checkSkillList (0x004360d0)
  skill_target     selectSkillTarget (0x004366a0)
  select_target    selectTarget(int) (0x00436cb0), selectTarget() (0x004371b0)
  start_affect     startSkill (0x00435ac0), affectSkill (0x00435b80),
                   setInterval (0x00436c40), clearConditionEnemy (0x004335d0)
  init_skills      clearSkillList (0x00435c40), initSkillList (0x00435cf0),
                   setSkillRate (0x00435ef0)
  init_enemy       initEnemy (0x00433260)
  genrand          genrand (main 0x001d9620)
  drain_race       ccGetDrainId (0x0042e3c0), ccGetEnemyRace (0x0042e2b0)
"""

import json
import math
import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_battle_rs as rs  # noqa: E402
from test_battle_rs import F_ONE, SCN, fbits, ser_char, ser_env  # noqa: E402

ENEMY_TBL, ROW = inf_va(0x005F1E70), 0x1C0
SKILL_TBL, SKILL_ROW = inf_va(0x0061F4A0), 0x38
RACE_TBL = inf_va(0x005F1D60)
MT, MTI = inf_va(0x003FF400), inf_va(0x00377FD0)
EVT = 0x01033000        # the ccEvent eventMng points at (puppetShow at +0x78c)
WM = 0x01033800         # worldman (A, B, C at +0x14c)
VT = 0x01033C00         # a ccEnemy vtable for main
ANM = 0x01034000        # an animation name table, 30 bytes a slot
ENT = 0x01034400        # a ccEntry for initEnemy, its ccEntryParam at +0x100
OBJ = 0x01035000        # the ccEnemy initEnemy fills
NEWOBJ = 0x01036000     # what entryObject hands entryDrainEnemy
SAVE = 0x01010000
KILL_COUNT, KILL_AREA = 0x68B7, 0x69F0
# a skill slot's ccSkillParam within the row: atc0, atc1, ski0, ski1
SLOT_OFS = {0: 0xD8, 1: 0x110, 4: 0x150, 5: 0x188}
SPELLS = (0, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 165, 169, 172, 175, 176,
          177, 180, 180, 181, 184, 187, 190, 192, 199, 203, 233, 273)


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def sgenrand(seed):
    """The 1998 sgenrand: what genrand and ccInitRand fill mt with."""
    out = []
    for _ in range(624):
        a = seed & 0xFFFF0000
        seed = (seed * 69069 + 1) & 0xFFFFFFFF
        a |= (seed & 0xFFFF0000) >> 16
        seed = (seed * 69069 + 1) & 0xFFFFFFFF
        out.append(a)
    return out


def rfloat(rnd, lo, hi):
    return fbits(rnd.uniform(lo, hi))


def rangle(rnd):
    return fbits(rnd.uniform(-math.pi, math.pi))


def rvec(rnd, span, z=200.0):
    return [rfloat(rnd, -span, span), rfloat(rnd, -span, span), rfloat(rnd, -z, z), F_ONE]


def default_enemy():
    """The state the constructors leave (ccChar, ccEntryObj, ccEnemy)."""
    return {"dirc": [0, 0, 0, 0], "affect": [0, 0], "eflags": 0x10, "fade": [1, 30], "ent": [0] * 22,
            "ids": [0, 0, 0], "flags": 0, "shorts": [0] * 11, "target": -1, "f5": [0] * 5, "bpos": [0] * 4,
            "mdirc": [0] * 4, "move": [0, 0, 0, F_ONE, 0, 0, 0], "anm": [0, 0, 0, 0], "life": F_ONE,
            "skill": [0, -1, -1, 0], "list": [[0, -1, -1, 0, -1, -1] for _ in range(6)]}


def ser_enemy(e):
    v = (e["dirc"] + e["affect"] + [e["eflags"]] + e["fade"] + e["ent"] + e["ids"] + [e["flags"]] + e["shorts"]
         + [e["target"]] + e["f5"] + e["bpos"] + e["mdirc"] + e["move"] + e["anm"] + [e["life"]] + e["skill"])
    for s in e["list"]:
        v += s
    return " ".join(str(x) for x in v)


class Harness:
    """The enemy set-up on top of test_battle_rs.Checks' machine."""

    def __init__(self, checks):
        self.c = checks
        g = checks.game
        self.g, self.m, self.b, self.tb, self.data = g, g.m, checks.b, checks.tb, checks.data
        m, sym = self.m, g.sym
        self.cur = 0            # the acting enemy's row, for the skill pointers in the calls
        self.active = 0
        self.dust = 116
        self.new_made = False
        self.race_num = [m.load(RACE_TBL + 12 * i + 4, 4) for i in range(22)]
        m.store(sym("eventMng"), 4, EVT)
        m.store(sym("worldman"), 4, WM)
        m.store(sym("cmndTarget"), 4, 0)
        for k, n in enumerate(("main__7ccEnemyFv", "freeze__7ccEnemyFv", "think__7ccEnemyFv",
                               "action__7ccEnemyFv", "exclusive__7ccEnemyFv", "note__7ccEnemyFP9ccAnmNote")):
            m.store(VT + 8 + 4 * k, 4, sym(n))
        rec = g.record
        b = self.b

        def copy(mm, a, b_, *_):
            mm.mem[a:a + 16] = mm.mem[b_:b_ + 16]
            return 0

        def new_obj(mm, ctrl, ep, n, *_):
            ent = [mm.load(ep + 4 * k, 4) for k in range(8)] + [mm.load(ep + 0x20 + 4 * k, 4, True)
                                                                for k in range(14)]
            g.calls.append(("entryObject", ent, b.s32(n)))
            mm.mem[NEWOBJ:NEWOBJ + 0x400] = bytes(0x400)
            self.new_made = True
            return NEWOBJ

        def ret(v):
            return lambda mm, a0, *_: (a0 if v == "a0" else v)

        self.base_hooks = {
            "ccCheckActiveEnemy__Fv": lambda mm, *_: self.active,
            "ccTransPosW2P__FPfPf": copy,
            "ccTransPosP2W__FPfPf": copy,
            "ccExpDistributor__Fs": lambda mm, a, *_: rec("ccExpDistributor", b.s16(a)),
            "effSkillStart__FP6ccCharP12ccSkillParamii":
                lambda mm, a, s, x, y: rec("effSkillStart", a, self.skcode(s), x, y),
            "_ccSkillRequest__FP6ccCharP6ccCharii": lambda mm, a, t, sid, st: rec("request", a, t, sid, st),
            "ccSkillDamage__FP6ccCharP6ccCharP12ccSkillParami":
                lambda mm, a, t, s, sid: rec("ccSkillDamage", a, t, self.skcode(s), sid),
            "entryEnemyObject__11ccEntryCtrlFP10ccEntryObj": lambda mm, c, o, *_: rec("entryEnemyObject", o),
            "entryObject__11ccEntryCtrlFP12ccEntryParami": new_obj,
            "effAfterDrain__FP6ccChari": lambda mm, o, k, *_: rec("effAfterDrain", o, k),
            "ccChangeCmndTarget__FP6ccChar": lambda mm, a, *_: rec("ccChangeCmndTarget", a),
            "deleteConditionEffect__FP17ccConditionEffect": lambda mm, a, *_: rec("deleteConditionEffect", a),
        }
        self.main_hooks = {n: (lambda nm: lambda mm, a, *_: rec(nm, a))(n.split("__")[0])
                           for n in ("interruptThink__7ccEnemyFv", "moveEnemy__7ccEnemyFv", "animEnemy__7ccEnemyFv",
                                     "dispEnemy__7ccEnemyFv")}
        self.init_hooks = {
            "GetChunkAdrsF__8ccStreamFPCci": ret(0),
            "__ct__7ccCoordFv": ret("a0"),
            "Init__7ccClumpFP12ccClumpChunk": ret(0),
            "ccEntryChangeCLUT__FP7ccEntryP7ccClump": ret(0),
            "__ct__5ccAnmFv": ret("a0"),
            "ApplyClump__5ccAnmFP7ccClumpP8ccStream": ret(0),
            "SetAnm__5ccAnmFP10ccAnmChunkUi": ret(0),
            "ccGetNameBossAnm__FPcPc": ret(0),
            "SetShadowSw__5ccAnmFi": ret(0),
            "SetHitSW__9ccCharHitFi": ret(0),
            "ccCheckDustColor__FP6ccChar": lambda mm, *_: self.dust,
        }

    # hooks ------------------------------------------------------------------------
    def call(self, fn, args, extra=None, flt=None):
        """Run a function with this module's stubs in place, then put the
        machine's own hooks back (other checks run the real functions)."""
        m, sym = self.m, self.g.sym
        hooks = dict(self.base_hooks)
        hooks.update(extra or {})
        saved = {}
        for n, h in hooks.items():
            a = sym(n)
            saved[a] = m.hooks.get(a)
            m.hooks[a] = h
        try:
            if flt is not None:
                m.f[12] = flt
            return m.call(sym(fn), tuple(x & 0xFFFFFFFF for x in args))
        finally:
            for a, h in saved.items():
                if h is None:
                    del m.hooks[a]
                else:
                    m.hooks[a] = h

    # pointers ---------------------------------------------------------------------
    def skcode(self, addr, eid=None):
        eid = self.cur if eid is None else eid
        if addr == 0:
            return -1
        row = ENEMY_TBL + ROW * eid
        for k, o in SLOT_OFS.items():
            if addr == row + o:
                return 1000 + k
        if SKILL_TBL <= addr < SKILL_TBL + SKILL_ROW * 304 and (addr - SKILL_TBL) % SKILL_ROW == 0:
            return (addr - SKILL_TBL) // SKILL_ROW
        return addr

    def skptr(self, code, eid):
        if code == -1:
            return 0
        if code >= 1000:
            return ENEMY_TBL + ROW * eid + SLOT_OFS[code - 1000]
        return SKILL_TBL + SKILL_ROW * code

    @staticmethod
    def index(addr):
        if addr == 0:
            return -1
        if SCN <= addr < SCN + 0x8000 and (addr - SCN) % 0x1000 == 0:
            return (addr - SCN) // 0x1000
        return addr

    @staticmethod
    def ptr(i):
        return 0 if i < 0 else SCN + 0x1000 * i

    # generators -------------------------------------------------------------------
    def put_cc(self, seed, mti):
        self.m.mem[MT:MT + 8 * 624] = struct.pack("<624Q", *sgenrand(seed))
        self.m.store(MTI, 4, mti)

    def cc_state(self):
        h = 0xCBF29CE484222325
        words = struct.unpack("<624Q", bytes(self.m.mem[MT:MT + 8 * 624]))
        for w in words:
            for byte in (w & 0xFFFFFFFF).to_bytes(4, "little"):
                h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        return [s32(self.m.load(MTI, 4)), h]

    def race(self, eid):
        total = 0
        for r, n in enumerate(self.race_num):
            first = total
            total += n
            if eid < total:
                return r, eid - first
        return None

    # the enemy block ----------------------------------------------------------------
    def put_enemy(self, va, e, base):
        g, m = self.g, self.m
        eid = e["ids"][0]
        row = ENEMY_TBL + ROW * eid
        m.mem[va + 0xE0:va + 0xE4] = bytes(4)
        g.put(va + 0x60, "<4I", *e["dirc"])
        g.put(va + 0x9C, "<hh", *e["affect"])
        m.store(va + 0xE0, 1, e["eflags"])
        g.put(va + 0xF0, "<hh", *e["fade"])
        g.put(va + 0x100, "<8I14i", *e["ent"])
        m.store(va + 0x1CC, 4, VT)
        m.store(va + 0x234, 4, base)
        m.store(va + 0x238, 4, va + 8)
        m.store(va + 0x23C, 4, row + 0xB0)
        m.store(va + 0x240, 4, row + 0xD8)
        g.put(va + 0x244, "<3i", *e["ids"])
        g.put(va + 0x250, "<H", e["flags"])
        g.put(va + 0x252, "<11h", *e["shorts"])
        m.store(va + 0x268, 4, self.ptr(e["target"]))
        g.put(va + 0x26C, "<5I", *e["f5"])
        g.put(va + 0x280, "<4I", *e["bpos"])
        g.put(va + 0x290, "<4I", *e["mdirc"])
        mv = e["move"]
        g.put(va + 0x2A0, "<IIiIIII", *mv)
        g.put(va + 0x2BC, "<hhHh", *e["anm"])
        m.store(va + 0x2C4, 4, e["life"])
        sid, code, tgt, num = e["skill"]
        g.put(va + 0x2C8, "<iIIi", sid, self.skptr(code, eid), self.ptr(tgt), num)
        for k, (fl, atk, ski, pct, sc, st) in enumerate(e["list"]):
            b = va + 0x2D8 + 16 * k
            g.put(b, "<bbhIII", fl, atk, ski, pct, self.skptr(sc, eid), self.ptr(st))

    def read_enemy(self, va, eid):
        m = self.m

        def L(o, n=4, signed=False):
            return m.load(va + o, n, signed)

        def sh(o):
            return L(o, 2, True)

        lst = []
        for k in range(6):
            b = 0x2D8 + 16 * k
            lst.append([L(b, 1) & 0xF, L(b + 1, 1, True), sh(b + 2), L(b + 4), self.skcode(L(b + 8), eid),
                        self.index(L(b + 12))])
        return {"dirc": [L(0x60 + 4 * k) for k in range(4)], "affect": [sh(0x9C), sh(0x9E)],
                "eflags": L(0xE0, 1), "fade": [sh(0xF0), sh(0xF2)],
                "ent": [L(0x100 + 4 * k) for k in range(8)] + [L(0x120 + 4 * k, 4, True) for k in range(14)],
                "ids": [L(0x244, 4, True), L(0x248, 4, True), L(0x24C, 4, True)], "flags": L(0x250, 2) & 0x1FF,
                "shorts": [sh(0x252 + 2 * k) for k in range(11)], "target": self.index(L(0x268)),
                "f5": [L(0x26C + 4 * k) for k in range(5)], "bpos": [L(0x280 + 4 * k) for k in range(4)],
                "mdirc": [L(0x290 + 4 * k) for k in range(4)],
                "move": [L(0x2A0), L(0x2A4), L(0x2A8, 4, True), L(0x2AC), L(0x2B0), L(0x2B4), L(0x2B8)],
                "anm": [sh(0x2BC), sh(0x2BE), L(0x2C0, 2), sh(0x2C2)], "life": L(0x2C4),
                "skill": [L(0x2C8, 4, True), self.skcode(L(0x2CC), eid), self.index(L(0x2D0)), L(0x2D4, 4, True)],
                "list": lst}

    def read_char(self, va, pva, ch, pc):
        d = self.g.read_char(va, pva, pc)
        if not pc:
            d["PP"][2] = getattr(ch, "PPrestore", 0)
        m = self.m
        d["pos"] = [m.load(va + 0x40 + 4 * k, 4) for k in range(4)]
        d["posP"] = [m.load(va + 0x50 + 4 * k, 4) for k in range(4)]
        d["skill"] = [m.load(va + 0x7C, 2, True), m.load(va + 0x7E, 2, True)]
        return d

    # rows -------------------------------------------------------------------------
    def rnd_patch(self, rnd, eid, maxpp=None):
        """A random think, spells, skill slots, size, type and gauge for a
        row, in memory; returns the probe's line."""
        m, g = self.m, self.g
        va = ENEMY_TBL + ROW * eid
        th = list(struct.unpack("<6f i 2f i", bytes(m.mem[va + 0xB0:va + 0xD8])))
        if rnd.random() < 0.7:
            th[0] = rnd.choice((rnd.uniform(300, 12000), 10000.0, 500.0))
            th[1] = rnd.choice((rnd.uniform(300, 8000), 6000.0))
            th[2] = rnd.choice((rnd.uniform(100, 3000), 6000.0, 1500.0))
            a = rnd.choice((0.0, 0.0, rnd.uniform(50, 600), 150.0, 400.0))
            bb = rnd.uniform(a, a + 800) if rnd.random() < 0.8 else rnd.uniform(0, 1500)
            c = rnd.uniform(bb, bb + 1200) if rnd.random() < 0.8 else rnd.uniform(0, 2000)
            th[3], th[4], th[5] = a, bb, c
            th[6] = rnd.choice((60, 0, 20, 180, rnd.randrange(-5, 300)))
            th[7] = rnd.choice((20.0, 28.0, rnd.uniform(0, 60)))
            th[9] = rnd.choice((0, 0, 1, 1, 2, rnd.randrange(-2, 5)))
        mag = [m.load(va + 0xD8 + 0x70 + 4 * k, 4, True) for k in range(2)]
        if rnd.random() < 0.7:
            mag = [rnd.choice(SPELLS) for _ in range(2)]
        slots = []
        for o in (0xD8, 0x110, 0x150, 0x188):
            ty = m.load(va + o + 0x2C, 4, True)
            trig = m.load(va + o + 0x1C, 4)
            cost = m.load(va + o + 0x28, 4, True)
            if rnd.random() < 0.5:
                ty = rnd.choice((0x41, 0x1, 0x2, -1, 0x40002, 0x10002, 0x20002, 0x3, ty))
                trig = fbits(rnd.choice((rnd.uniform(0, 2000), 300.0, 0.0)))
                cost = rnd.choice((0, 0, 5, 30, 200))
            slots.append((ty, trig, cost))
        esize = m.load(va + 0x70, 4)
        typ = m.load(va + 8, 4, True)
        gold = m.load(va + 0x14, 4, True)
        pp = m.load(va + 0x52, 2, True)
        if rnd.random() < 0.3:
            esize = rnd.choice((1, 3, 4, 0))
        if rnd.random() < 0.2:
            typ = rnd.choice((0x20, 0x40))
            gold = rnd.choice((-1, rnd.randrange(0, 303)))
        if maxpp is not None:
            pp = maxpp
        elif rnd.random() < 0.3:
            pp = rnd.choice((-1, 0, 100))
        g.put(va + 0xB0, "<6f i 2f i", *th)
        g.put(va + 0xD8 + 0x70, "<2i", *mag)
        for (ty, trig, cost), o in zip(slots, (0xD8, 0x110, 0x150, 0x188)):
            g.put(va + o + 0x2C, "<i", ty)
            g.put(va + o + 0x1C, "<I", trig)
            g.put(va + o + 0x28, "<i", cost)
        g.put(va + 0x70, "<I", esize)
        g.put(va + 8, "<i", typ)
        g.put(va + 0x14, "<i", gold)
        g.put(va + 0x52, "<h", pp)
        thb = [struct.unpack("<I", struct.pack("<f", x))[0] if k not in (6, 9) else x for k, x in enumerate(th)]
        v = [eid] + thb + mag + [x for s in slots for x in s] + [esize, typ, gold, pp]
        return " ".join(str(x) for x in v)

    def restore_row(self, eid):
        self.g.restore(ENEMY_TBL + ROW * eid, ROW)

    def patch_spell(self, rnd, sid):
        sk = dict(self.data.skills[sid])
        sk["attr"] = list(sk["attr"])
        sk["triggerRange_bits"] = fbits(rnd.choice((sk["triggerRange"], rnd.uniform(0, 2500), 400.0)))
        sk["targetRange_bits"] = fbits(sk["targetRange"])
        sk["cost"] = rnd.choice((sk["cost"], 0, 10, 300))
        if rnd.random() < 0.15:
            sk["type"] = rnd.choice((0x40002, 0x10002, 0x20002, 0x2, 0x1))
        self.c.put_skill(sk, sk["va"])
        return f"{sid} {rs.ser_skill(sk)}"

    # random states ----------------------------------------------------------------
    def rnd_enemy(self, rnd, ch, eid, n, me):
        b = self.b
        e = default_enemy()
        e["dirc"] = [rangle(rnd), rangle(rnd), rangle(rnd), F_ONE]
        e["affect"] = [rnd.choice((0, 1, 1, 1, 3, 4, 7, 9, 10, 13, 20, 20, rnd.randrange(-3, 30))),
                       rnd.choice((0, 1, 5, -1, rnd.randrange(-50, 500)))]
        ef = rnd.getrandbits(8) & ~0x04 & ~0x40 | 0x10
        if rnd.random() < 0.1:
            ef |= 0x40
        e["eflags"] = ef
        e["fade"] = [rnd.randrange(-1, 3), rnd.randrange(0, 40)]
        ent = rvec(rnd, 1500) + [rangle(rnd), rangle(rnd), rangle(rnd), F_ONE]
        ent += [0, eid] + [rnd.randrange(-5, 50) for _ in range(6)]
        ent += [ch.ent_root, rnd.randrange(0, 5)] + [rnd.randrange(-100, 100) for _ in range(4)]
        e["ent"] = ent
        r = self.race(eid) or (0, 0)
        race_id = rnd.choice((r[1], r[1], 0, 1, -1))
        e["ids"] = [eid, r[0], race_id]
        fl = rnd.getrandbits(9) & ~0x4
        if rnd.random() < 0.65:
            fl |= 1
        e["flags"] = fl
        act = rnd.choice((0, 0, 0, 1, 1, 1, 2, 3, 3, 4, 5, 6, 6, 7, 7, 8, 8, 9, 9, rnd.choice((-1, 10))))
        cnt = rnd.choice((0, 0, 1, 2, 5, 16, 17, 30, 31, 62, 63, 89, 90, 91, 240, 241, 255, 256, rnd.randrange(0, 400),
                          rnd.randrange(-40000, 40000) & 0xFFFF))
        e["shorts"] = [rnd.randrange(0, 6), rnd.randrange(-100, 100), 116, rnd.randrange(0, 4), act, s16(cnt),
                       rnd.choice((0, 1, 2, 3, 4, 5, 4, 5, 6)), rnd.randrange(0, 50),
                       rnd.choice((0, 0, 0, 1, 2, 7, 60, -1)), rnd.choice((0, 0, 0, 0, 1, 2, 30)),
                       rnd.choice((0, 0, 1, 2, 40, -1))]
        e["target"] = rnd.choice((-1, rnd.randrange(n), rnd.randrange(n), rnd.randrange(n), me))
        e["f5"] = [rangle(rnd), rnd.choice((0, rfloat(rnd, -100, 2500), rfloat(rnd, 0, 700), fbits(100.0))),
                   rnd.choice((0, F_ONE, fbits(0.5), rfloat(rnd, 0, 1))), rangle(rnd),
                   rnd.choice((0, rfloat(rnd, -100, 12000), rfloat(rnd, 0, 7000)))]
        e["bpos"] = rvec(rnd, 3000)
        e["mdirc"] = [rangle(rnd), rangle(rnd), rangle(rnd), F_ONE]
        e["move"] = [rfloat(rnd, 0, 40), rfloat(rnd, -3, 3), rnd.choice((0, 90, 91, rnd.randrange(0, 200))),
                     rnd.choice((F_ONE, rfloat(rnd, 0, 2))),
                     rnd.choice((0, F_ONE, fbits(0.5), fbits(0.999), rfloat(rnd, 0, 40))), 0, 0]
        e["anm"] = [rnd.randrange(0, 12), rnd.randrange(-1, 12), rnd.randrange(0, 100), rnd.choice((0, 1))]
        e["life"] = rnd.choice((F_ONE, fbits(0.5), fbits(0.4999), rfloat(rnd, 0, 1.2)))
        num = rnd.choice((0, 1, 2, 3, 4, 5, 6, 6))
        lst = []
        for k in range(6):
            if k < num:
                atk = rnd.choice((0, 1, 2, 3, 4, 5))
                if atk in (2, 3):
                    ski = rnd.choice(SPELLS[1:])
                    code = ski
                else:
                    ski, code = -1, 1000 + atk
                lst.append([rnd.getrandbits(4), atk, ski, rnd.choice((rfloat(rnd, 0, 1), 0, fbits(0.25))), code,
                            rnd.choice((-1, rnd.randrange(n)))])
            else:
                lst.append([0, -1, -1, 0, -1, -1])
        e["list"] = lst
        sc = rnd.choice((1000, 1001, 1004, 1005, rnd.choice(SPELLS[1:])))
        e["skill"] = [rnd.choice((0, -1, 180, sc if 0 <= sc < 1000 else 0, rnd.randrange(0, 304))), sc,
                      rnd.choice((-1, rnd.randrange(n), me)), num]
        return e

    def rnd_scene(self, rnd, far=False):
        """Party members and enemies (distinct rows), the first enemy
        acting; returns chars, foes (state or None), pcs, enes, me."""
        b, tb, data = self.b, self.tb, self.data
        chars, kinds = [], []
        npc = rnd.choice((0, 1, 2, 3, 3, 4)) if rnd.random() < 0.95 else 0
        nfoe = rnd.choice((1, 1, 2, 3, 4))
        used = set()
        span = 12000 if far else rnd.choice((600, 1500, 3000))
        for _ in range(npc):
            pc = tb.rnd_pc(b, data, rnd)
            pc.type = rnd.choice((1, 2, 4, 6, 7, 2))
            pc.cond[0] = rnd.choice((0,) * 6 + (1, 2))
            chars.append(pc)
            kinds.append("pc")
        for _ in range(nfoe):
            f = tb.rnd_foe(b, data, rnd, "enemy")
            while f.row["index"] in used:
                f = tb.rnd_foe(b, data, rnd, "enemy")
            used.add(f.row["index"])
            f.cond[0] = rnd.choice((0,) * 8 + (1, 2, 3))
            if rnd.random() < 0.3:
                f.level = rnd.choice((1, 30, 31, 60))
            f.ai = 0
            f.party_flag = 0
            f.no_death = False
            chars.append(f)
            kinds.append("foe")
        for ch in chars:
            ch.pos = rvec(rnd, span)
            ch.pos_p = list(ch.pos) if rnd.random() < 0.9 else rvec(rnd, span)
            ch.width = rnd.choice((0, fbits(rnd.uniform(0, 60))))
            ch.skill_id, ch.skill_status = rnd.randrange(0, 300), rnd.randrange(0, 10)
            ch.anm_flag = rnd.choice((0, 1))
            ch.HP = rnd.choice((ch.HP, ch.HP, 0, -5, 1))
            for c in (8, 9, 10, 11, 1, 14, 15):
                if rnd.random() < 0.08:
                    ch.cond[c] = rnd.choice((1, 30, 600))
        order = list(range(len(chars)))
        rnd.shuffle(order)
        chars = [chars[i] for i in order]
        kinds = [kinds[i] for i in order]
        n = len(chars)
        foes_idx = [i for i in range(n) if kinds[i] == "foe"]
        me = foes_idx[0]
        foes = [None] * n
        for i in foes_idx:
            foes[i] = self.rnd_enemy(rnd, chars[i], chars[i].row["index"], n, me)
        pcs = [i for i in range(n) if kinds[i] == "pc" and rnd.random() > 0.05]
        enes = [i for i in range(n) if kinds[i] == "foe" and (rnd.random() > 0.05 or i == me)]
        return chars, foes, pcs, enes, me

    # the scene in memory ------------------------------------------------------------
    def put_scene(self, chars, foes, pcs, enes):
        g, m = self.g, self.m
        m.mem[SCN:SCN + 0x8000] = bytes(0x8000)
        g.names = {SCN + 0x1000 * i: f"c{i}" for i in range(len(chars))}
        g.names[NEWOBJ] = "new"
        for i, ch in enumerate(chars):
            va = SCN + 0x1000 * i
            f = foes[i]
            pva = va + 0x1D0 if f else va + 0x400
            g.put_char(ch, va, pva, va + 0x800)
            if hasattr(ch, "row"):
                g.put_row(ch)
            base = m.load(va, 4)
            g.put(va + 0x40, "<4I", *ch.pos)
            g.put(va + 0x50, "<4I", *ch.pos_p)
            m.store(base + 0x1C, 4, ch.width)
            g.put(va + 0x7C, "<hh", ch.skill_id, ch.skill_status)
            if ch.type & 7:
                g.put(va + 0xF4, "<h", ch.anm_flag)
            if f:
                self.put_enemy(va, f, base)
        for root, lst in ((g.cmnd_pc, pcs), (g.cmnd_ene, enes)):
            m.store(root, 4, self.ptr(lst[0]) if lst else 0)
            for a, b_ in zip(lst, lst[1:] + [None]):
                m.store(self.ptr(a) + 0xBC, 4, self.ptr(b_) if b_ is not None else 0)
        m.store(g.cmnd_obj, 4, 0)

    def lists(self):
        g, m = self.g, self.m
        out = []
        for root in (g.cmnd_pc, g.cmnd_ene):
            v, a = [], m.load(root, 4)
            while a and len(v) < 16:
                v.append(self.index(a))
                a = m.load(a + 0xBC, 4)
            out.append(v)
        return out

    def ser_scene(self, chars, foes, pcs, enes):
        s = f"escene {len(chars)} " + " ".join(ser_char(c, self.b) for c in chars)
        s += f" {len(pcs)} " + " ".join(map(str, pcs)) + f" {len(enes)} " + " ".join(map(str, enes))
        s += " " + " ".join(" ".join(map(str, c.pos)) for c in chars)
        for f in foes:
            s += " 0" if f is None else " 1 " + ser_enemy(f)
        return s

    def escene(self, rnd, op, args=(), tweak=None, far=False, patch_row=True):
        """One scene case: random scene, world, generators, then op on the
        first enemy. `tweak(chars, foes, pcs, enes, me)` adjusts it first."""
        g, m, b, tb = self.g, self.m, self.b, self.tb
        chars, foes, pcs, enes, me = self.rnd_scene(rnd, far)
        if tweak:
            tweak(chars, foes, pcs, enes, me)
        f = foes[me]
        eid = f["ids"][0]
        self.cur = eid
        self.last = (json.loads(json.dumps(f)), me, chars)
        row_patch = None
        spells = set()
        if patch_row and rnd.random() < 0.8:
            row_patch = self.rnd_patch(rnd, eid, chars[me].row["maxPP"])
            for k in range(2):
                spells.add(m.load(ENEMY_TBL + ROW * eid + 0xD8 + 0x70 + 4 * k, 4, True))
        for s in f["list"]:
            if s[4] != -1 and s[4] < 1000:
                spells.add(s[4])
        patched = [sid for sid in sorted(x for x in spells if 0 < x < 304) if rnd.random() < 0.5]
        spell_lines = [self.patch_spell(rnd, sid) for sid in patched]
        self.put_scene(chars, foes, pcs, enes)
        puppet = int(rnd.random() < 0.2)
        ride = int(rnd.random() < 0.2)
        self.active = rnd.randrange(0, 6)
        player = rnd.choice((pcs[0] if pcs else -1, -1, rnd.randrange(len(chars)))) if pcs or chars else -1
        if pcs and rnd.random() < 0.7:
            player = pcs[0]
        m.store(EVT + 0x78C, 4, puppet)
        m.store(g.sym("pgRideFlag"), 4, ride)
        m.store(g.sym("ccPartyManager"), 4, self.ptr(player))
        state = rnd.getrandbits(64)
        g.set_rand(state)
        seed, mti = rnd.getrandbits(32), rnd.choice((rnd.randrange(0, 624), 623, 624, 625, 0))
        self.put_cc(seed, mti)
        self.last_cc = [s32(mti), self.cc_state()[1]]
        env = tb.rnd_env(b, rnd)
        g.put_env(env)
        count = rnd.choice((0, 5, 98, 99, 127, rnd.randrange(0, 256)))
        server, area = rnd.randrange(-5, 40), [rnd.randrange(-100, 100) for _ in range(3)]
        m.store(SAVE + KILL_COUNT + eid, 1, count)
        m.mem[SAVE + KILL_AREA + 8 * eid:SAVE + KILL_AREA + 8 * eid + 8] = bytes(8)
        m.store(self.tb.GAME + 0x1C, 4, server & 0xFFFFFFFF)
        g.put(WM + 0x14C, "<3i", *area)
        m.store(g.sym("cmndTarget"), 4, 0)
        g.calls = []
        self.new_made = False
        va = self.ptr(me)
        fn, cargs, extra = {
            "think": ("defaultThink__7ccEnemyFv", (), None),
            "interrupt": ("interruptThink__7ccEnemyFv", (), None),
            "begin": ("main__7ccEnemyFv", (), self.main_hooks),
            "routine": ("routineEnemy__7ccEnemyFv", (), None),
            "check": ("checkEnemy__7ccEnemyFv", (), None),
            "crisis": ("checkCrisisRate__7ccEnemyFv", (), None),
            "setact": ("setAct__7ccEnemyFi", tuple(args), None),
            "attack": ("selectAttack__7ccEnemyFv", (), None),
            "skills": ("checkSkillList__7ccEnemyFv", (), None),
            "target": ("selectTarget__7ccEnemyFi", tuple(args), None),
            "target0": ("selectTarget__7ccEnemyFv", (), None),
            "interval": ("setInterval__7ccEnemyFv", (), None),
            "start": ("startSkill__7ccEnemyFv", (), None),
            "affect": ("affectSkill__7ccEnemyFv", (), None),
            "clearcond": ("clearConditionEnemy__7ccEnemyFv", (), None),
            "skilltarget": ("selectSkillTarget__7ccEnemyFiP12ccSkillParam",
                            (args[0], SKILL_TBL + SKILL_ROW * args[0]) if args else (), None),
        }[op]
        ret = self.call(fn, (va,) + cargs, extra)
        if op in ("attack",):
            ret &= 0xFF
        elif op in ("target", "target0", "begin"):
            ret &= 0xFF
        elif op == "skilltarget":
            ret = self.index(ret)
        elif op == "skills":
            ret = s32(ret)
        else:
            ret = 0
        out = {"ret": ret, "rand": g.rand_now(), "cc": self.cc_state(), "calls": list(g.calls),
               "chars": [self.read_char(self.ptr(i), self.ptr(i) + (0x1D0 if foes[i] else 0x400), c,
                                        bool(c.type & 7)) for i, c in enumerate(chars)],
               "foes": [self.read_enemy(self.ptr(i), f_["ids"][0]) if f_ else None for i, f_ in enumerate(foes)],
               "lists": self.lists(),
               "kill": [m.load(SAVE + KILL_COUNT + eid, 1, True)]
               + [m.load(SAVE + KILL_AREA + 8 * eid + 2 * k, 2, True) for k in range(4)],
               "new": [m.load(NEWOBJ + 8, 2, True), m.load(NEWOBJ + 0x264, 2, True)] if self.new_made else None}
        for c in chars:
            if hasattr(c, "row"):
                self.g.restore(c.row["va"], 0x68)
        req = (self.ser_scene(chars, foes, pcs, enes)
               + f" {puppet} {ride} {self.active} {player} {state} {seed} {mti} {ser_env(env)}"
               + f" {count} {server} {' '.join(map(str, area))} {op} {me}"
               + ("" if not args else " " + " ".join(str(a) for a in args)))
        if row_patch:
            self.restore_row(eid)
        for sid in patched:
            self.g.restore(SKILL_TBL + SKILL_ROW * sid, SKILL_ROW)
        prep = (f"eprep {int(bool(row_patch))}" + (f" {row_patch}" if row_patch else "")
                + f" {len(spell_lines)}" + "".join(" " + x for x in spell_lines))
        unprep = f"eunprep {int(bool(row_patch))} {eid} {len(patched)}" + "".join(f" {x}" for x in patched)
        return [prep, req, unprep], out, op


def harness(checks):
    h = getattr(checks, "_enemy", None)
    if h is None:
        h = checks._enemy = Harness(checks)
    return h


# the checks ---------------------------------------------------------------------------

def check_think(checks, rnd):
    return harness(checks).escene(rnd, "think")


def check_interrupt(checks, rnd):
    def tweak(chars, foes, pcs, enes, me):
        f = foes[me]
        if rnd.random() < 0.5:
            f["eflags"] |= 0x08
        if rnd.random() < 0.3:
            chars[me].cond[0] = rnd.choice((0, 2, 2))
    return harness(checks).escene(rnd, "interrupt", tweak=tweak)


def check_begin(checks, rnd):
    def tweak(chars, foes, pcs, enes, me):
        f = foes[me]
        r = rnd.random()
        if r < 0.15:
            f["eflags"] |= 0x04
        if rnd.random() < 0.2:
            f["flags"] |= 0x04
        if rnd.random() < 0.3:
            f["shorts"][9] = rnd.choice((1, 2, 30))
        chars[me].cond[0] = rnd.choice((0, 0, 0, 1, 2, 3))
    return harness(checks).escene(rnd, "begin", tweak=tweak)


def check_routine(checks, rnd):
    return harness(checks).escene(rnd, "routine")


def check_enemy_(checks, rnd):
    h = harness(checks)

    def tweak(chars, foes, pcs, enes, me):
        f = foes[me]
        if rnd.random() < 0.3:
            f["target"] = me
        if rnd.random() < 0.3:
            f["shorts"][4] = 6
            f["skill"][2] = rnd.choice((-1, me, rnd.randrange(len(chars))))
    return h.escene(rnd, rnd.choice(("check", "check", "crisis")), tweak=tweak)


def check_set_act(checks, rnd):
    return harness(checks).escene(rnd, "setact", (rnd.choice((0, 1, 1, 6, 6, 2, 3, 4, 5, 7, 8, 9, -1)),))


def heal_tweak(rnd):
    """Often a heal or revival in the list, hurt or dying foes about."""
    def tweak(chars, foes, pcs, enes, me):
        if rnd.random() < 0.5:
            return
        f = foes[me]
        if f["skill"][3] == 0:
            f["skill"][3] = 1
        k = rnd.randrange(f["skill"][3])
        sid = rnd.choice((150, 151, 152, 153, 154, 155, 180, 180))
        f["list"][k] = [rnd.getrandbits(4), rnd.choice((2, 3)), sid, rfloat(rnd, 0, 1), sid, -1]
        chars[me].SP = rnd.choice((chars[me].SP, 999))
        for c in (9, 10, 11, 14):
            chars[me].cond[c] = 0
        for i, g in enumerate(foes):
            if g and rnd.random() < 0.6:
                g["life"] = rfloat(rnd, 0, 0.6)
                if i != me:
                    chars[i].cond[0] = rnd.choice((0, 2, 2))
                    chars[i].pos_p = [fbits(struct.unpack("<f", struct.pack("<I", chars[me].pos_p[0]))[0]
                                            + rnd.uniform(-150, 150)), chars[me].pos_p[1], chars[me].pos_p[2], F_ONE]
    return tweak


def check_select_attack(checks, rnd):
    return harness(checks).escene(rnd, "attack", tweak=heal_tweak(rnd))


def check_skill_list(checks, rnd):
    return harness(checks).escene(rnd, "skills", tweak=heal_tweak(rnd))


def check_skill_target(checks, rnd):
    sid = rnd.choice(SPELLS[1:] + (rnd.randrange(0, 304), -1)) if rnd.random() < 0.95 else -1
    if sid == -1:
        sid = 150
    return harness(checks).escene(rnd, "skilltarget", (sid,))


def check_select_target(checks, rnd):
    if rnd.random() < 0.25:
        return harness(checks).escene(rnd, "target0")
    return harness(checks).escene(rnd, "target", (rnd.choice((0, 0, 1, 1, 2, 2, -1, -2, 3)),))


def check_start_affect(checks, rnd):
    return harness(checks).escene(rnd, rnd.choice(("start", "start", "affect", "affect", "interval", "clearcond")))


def check_init_skills(checks, rnd):
    h = harness(checks)
    g, m = h.g, h.m
    eid = rnd.randrange(0, 303)
    patch = "epatch " + h.rnd_patch(rnd, eid)
    anm = [rnd.choice((0, 1, 1)) for _ in range(6)]
    m.mem[OBJ:OBJ + 0x400] = bytes(0x400)
    m.mem[ANM:ANM + 0x100] = bytes(0x100)
    for k, a in enumerate(anm):
        m.store(ANM + 30 * k, 1, rnd.choice((0x41, 0xE0, 0x7F)) if a else 0)
    m.store(OBJ + 0x1B0, 4, ANM)
    m.store(OBJ + 0x240, 4, ENEMY_TBL + ROW * eid + 0xD8)
    m.store(OBJ + 0x244, 4, eid)
    for k in range(6):
        m.store(OBJ + 0x2D8 + 16 * k, 4, rnd.getrandbits(32))
    h.call("clearSkillList__7ccEnemyFv", (OBJ,))
    h.call("initSkillList__7ccEnemyFv", (OBJ,))
    e = h.read_enemy(OBJ, eid)
    out = {"num": e["skill"][3], "list": e["list"]}
    h.restore_row(eid)
    return [patch, f"eskills {eid} {' '.join(map(str, anm))}", f"erestore {eid}"], out, "init_skills"


def check_init_enemy(checks, rnd):
    h = harness(checks)
    g, m = h.g, h.m
    eid = rnd.randrange(0, 303)
    h.cur = eid
    patch = "epatch " + h.rnd_patch(rnd, eid)
    anm = [rnd.choice((0, 1, 1)) for _ in range(6)]
    m.mem[ANM:ANM + 0x100] = bytes(0x100)
    for k, a in enumerate(anm):
        m.store(ANM + 30 * k, 1, 0x41 if a else 0)
    ent = rvec(rnd, 3000) + [rangle(rnd), rangle(rnd), rangle(rnd), F_ONE]
    ent += [0, eid] + [rnd.randrange(-5, 50) for _ in range(6)] + [rnd.choice((0, 1, 7)), rnd.randrange(0, 5)]
    ent += [rnd.randrange(-100000, 100000) for _ in range(4)]
    m.mem[ENT:ENT + 0x200] = bytes(0x200)
    g.put(ENT + 0x100, "<8I14i", *ent)
    m.store(ENT + 0xC, 4, ENT + 0x100)
    m.store(ENT + 0x10, 4, 0x01034800)
    m.store(ENT + 0x18, 4, ANM)
    # the object as the constructors leave it
    m.mem[OBJ:OBJ + 0x400] = bytes(0x400)
    for k in range(16):
        m.store(OBJ + 8 + 2 * k, 2, 0)
    m.store(OBJ + 0x28, 4, F_ONE)
    m.store(OBJ + 0x30, 4, 0xFFFFFFFF)
    g.put(OBJ + 0x7C, "<hh", 0, -1)
    h.put_enemy(OBJ, default_enemy(), 0)
    m.store(OBJ + 0x234, 4, 0)
    h.dust = rnd.choice((116, 117))
    seed, mti = rnd.getrandbits(32), rnd.choice((rnd.randrange(0, 624), 624, 625))
    h.put_cc(seed, mti)
    checks.scene.new_at = rs.NEW      # the scene's operator new, a bump allocator
    h.call("initEnemy__7ccEnemyFP7ccEntry", (OBJ, ENT), h.init_hooks)

    class Stub:
        PPrestore = 0
    out = {"char": h.read_char(OBJ, OBJ + 0x1D0, Stub, False), "foe": h.read_enemy(OBJ, eid), "cc": h.cc_state()}
    h.restore_row(eid)
    req = f"einit {' '.join(map(str, ent))} {' '.join(map(str, anm))} {h.dust} {seed} {mti}"
    return [patch, req, f"erestore {eid}"], out, "init_enemy"


def check_genrand(checks, rnd):
    h = harness(checks)
    seed, mti = rnd.getrandbits(32), rnd.choice((rnd.randrange(0, 624), 622, 623, 624, 625, 700))
    n = rnd.choice((1, 2, 3, 5, 700)) if rnd.random() < 0.9 else 1300
    h.put_cc(seed, mti)
    fn = h.g.sym("genrand__Fv")
    v = [h.m.call(fn) for _ in range(n)]
    return f"genrand {seed} {mti} {n}", {"v": v, "cc": h.cc_state()}, "genrand"


def check_drain_race(checks, rnd):
    h = harness(checks)
    m = h.m
    eid = rnd.randrange(0, 303)
    if rnd.random() < 0.5:
        patch = "epatch " + h.rnd_patch(rnd, eid)
        v = s32(h.call("ccGetDrainId__Fi", (eid,)))
        h.restore_row(eid)
        return [patch, f"drainid {eid}", f"erestore {eid}"], v, "drain_id"
    i = rnd.choice((eid, rnd.randrange(-20, 340)))
    m.store(0x01033F00, 4, 0x7EADBEEF)
    m.store(0x01033F04, 4, 0x7EADBEEF)
    h.call("ccGetEnemyRace__FiPiPi", (i, 0x01033F00, 0x01033F04))
    r, k = m.load(0x01033F00, 4, True), m.load(0x01033F04, 4, True)
    return f"race {i}", None if r == 0x7EADBEEF else [r, k], "race"


ENEMY_AI = {
    "think": check_think,
    "interrupt": check_interrupt,
    "begin": check_begin,
    "routine": check_routine,
    "check_enemy": check_enemy_,
    "set_act": check_set_act,
    "select_attack": check_select_attack,
    "skill_list": check_skill_list,
    "skill_target": check_skill_target,
    "select_target": check_select_target,
    "start_affect": check_start_affect,
    "init_skills": check_init_skills,
    "init_enemy": check_init_enemy,
    "genrand": check_genrand,
    "drain_race": check_drain_race,
}


@unittest.skipUnless(rs.READY, "needs the extracted disc and cargo")
class EnemyAiAgainstGame(rs.Against):
    TABLE = ENEMY_AI

    def test_think(self):
        self.check("think", 9100)
        self.check("interrupt", 9101)
        self.check("begin", 9102)
        self.check("routine", 9103)

    def test_acts(self):
        self.check("check_enemy", 9104)
        self.check("set_act", 9105)

    def test_attacks(self):
        self.check("select_attack", 9106)
        self.check("skill_list", 9107)
        self.check("skill_target", 9108)
        self.check("select_target", 9109)
        self.check("start_affect", 9110)

    def test_setup(self):
        self.check("init_skills", 9111)
        self.check("init_enemy", 9112)
        self.check("genrand", 9113)
        self.check("drain_race", 9114)


if __name__ == "__main__":
    rs.main(list(ENEMY_AI), 9100, ENEMY_AI)
