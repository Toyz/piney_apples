#!/usr/bin/env python3
"""piney_battle::party_ai against the game's ccAI (personal.cpp) and ccFellow
(fellow.cpp) run in tools/eemu.py.

Each check builds a random world: party members (Kite in slot 0, fellows in
1 and 2) and foes on the command lists, their ccAI objects entered on a
ccAISystem with random waiting and delivered messages, the members' skill
and item lists in saveData, and the ccSpcChar/ccFellow fields the AI reads
(actNum, targetChar, moveFlag, nowSpeed, cycle, distTg, SpcListNum). It runs
one of the AI's functions natively in the interpreter and the same case
through the battle_probe example (`pai` requests), and compares the return
value, every field of every ccAI (the message queues included), the message
bus, the ccSpcChar fields, the item lists, the AI globals
(SelectAttackSkillResult, checkSkillOtherSPCFlag, checkHealSkillOtherSPCFlag),
the rand() state, and the calls made in order.

The calls that are not rules are stubbed and recorded: ccSkillRequest,
ccUseItemRequest, ccAI::FollowTarget and FollowBeacon, ccPlayer::Attack,
the ccAI::ChatMessage* functions; ccHitCheckLM and ccAI::CheckGoalBeaconPos
answer from a random script both sides read in the same order.
ccTransPosW2P is the identity (test_battle_rs.GameScene).

    python3 tools/test_battle_party_ai_rs.py            the unit tests
    python3 tools/test_battle_party_ai_rs.py bulk N     N cases of every check
"""

import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_battle as TB                       # noqa: E402
import test_battle_rs as T                     # noqa: E402
from test_battle_rs import fbits               # noqa: E402

AIS = 0x01060000        # the ccAI of scene character i at AIS + 0x400 * i
AISYS = 0x01070000      # the ccAISystem
ENTS = 0x01071000       # g_entCtrl's stand-in: +16 count, +20 first; linked at +0x1c4
SPC_AI_PARAM = inf_va(0x00653CA0)
VT_FELLOW, VT_PLAYER, VT_SPC = inf_va(0x00375C30), inf_va(0x00377A50), inf_va(0x00377A60)
F_M1 = 0xBF800000
KINDS = (2, 3, 4, 5, 6, 7, 8, 9, 10, 11)

# ccAI members in the order the probe reads them: (name, offset, format);
# "C" a character pointer, "P" the spcAIParam row, "E" the sysMsg entry.
AI_LAYOUT = [
    ("flags", 0x0, "<I"), ("strategyCMD", 0x4, "<h"), ("strategy", 0x6, "<h"),
    ("mode", 0x8, "<i"), ("modeOld", 0xC, "<i"), ("count", 0x10, "<i"),
    ("param", 0x14, "P"), ("body", 0x18, "C"), ("target", 0x1C, "C"), ("targetCCmd", 0x20, "C"),
    ("remoteCmd", 0x74, "<h"), ("chatCmd", 0x76, "<h"), ("chatCmdNew", 0x78, "<h"),
    ("chatCmdSkill", 0x7A, "<h"), ("chatCmdItem", 0x7C, "<i"), ("arrivalChatCnt", 0x80, "<h"),
    ("chatCmdTime", 0x82, "<h"), ("atkDellay", 0x84, "<h"), ("actType", 0x86, "<h"),
    ("actTypeOld", 0x88, "<h"), ("actTime", 0x8A, "<h"), ("actStep", 0x8C, "<h"), ("actCnt", 0x8E, "<h"),
    ("atkMsgCnt", 0x90, "<b"), ("dmgMsgCnt", 0x91, "<b"), ("actDummy", 0x92, "<h"), ("actSkill", 0x94, "<h"),
    ("lastMarker", 0x96, "<h"), ("noMoveCnt", 0x98, "<h"), ("gDeg", 0x9A, "<H"), ("gRotSp", 0x9C, "<h"),
    ("gPoint", 0x9E, "<h"), ("distPl", 0xA0, "<I"), ("dircPl", 0xA4, "<I"), ("distTg", 0xA8, "<I"),
    ("dircTg", 0xAC, "<I"), ("attackCycle", 0xB0, "<h"), ("atkTargetCnt", 0xB2, "<h"),
    ("detourCnt", 0xB4, "<h"), ("levelOld", 0xB6, "<h"), ("territory", 0xB8, "<I"),
    ("attackRange", 0xBC, "<I"), ("stopRange", 0xC0, "<I"), ("noTurnRange", 0xC4, "<I"),
    ("posOld", 0xD0, "<4I"), ("gPos", 0xE0, "<4I"), ("naviFinish", 0x10A, "<h"), ("entry", 0x160, "E"),
]
AI_LAYOUT = [(n, volume.ai_at(off), f) for n, off, f in AI_LAYOUT]    # Infection's offsets, on the volume
if volume.NAME != "infection":      # Mutation's new fields: the biggest recent hit, the biggest hit, the
    AI_LAYOUT += [("hitRecent", 0x9A, "<h"), ("hitMax", 0x9C, "<h"),   # attack command's target
                  ("cmdAttackTarget", 0x24, "C")]
# ccSpcChar / ccFellow fields: actNum targetChar moveFlag nowSpeed cycle distTg SpcListNum
SPC_FIELDS = ("act_num", "target_char", "move_flag", "now_speed", "cycle", "dist_tg", "spc_list_num", "run_flag",
              "ghost", "stop_flag", "stop_cnt", "walk_run_cnt")
EVENT_MNG = 0x01072000  # eventMng's stand-in: +0x78c the ocarina lock
GAME_FIELDS = ("in_battle", "area", "field", "field_type", "field_attr", "menu_type", "event_lock",
               "spc_battle_condition", "party_strategy", "player", "field_24", "pg_ride_flag")


def fl(x):
    return fbits(x)


# the world ---------------------------------------------------------------------------

class AiGame:
    """The game side: GameBattle and GameScene (tools/test_battle_rs.py)
    plus the AI's objects and stubs."""

    def __init__(self, checks):
        self.c = checks
        g = checks.game
        self.g = g
        self.m = g.m
        self.sym = g.sym
        m = self.m
        self.p_sys = self.sym("ccAISys")
        self.p_ent = self.sym("g_entCtrl")
        self.p_party = self.sym("ccPartyManager")
        self.g_sel = self.sym("SelectAttackSkillResult")
        self.g_other = self.sym("checkSkillOtherSPCFlag")
        self.g_heal = self.sym("checkHealSkillOtherSPCFlag")
        self.g_ocarina = self.sym("ocarinaUseFlag")
        self.g_sbc = self.sym("spcBattleCondition")
        self.g_pstrat = self.sym("partyStrategy")
        self.p_player = self.sym("plw") + 0x20
        self.g_pgride = self.sym("pgRideFlag")
        m.store(self.p_sys, 4, AISYS)
        m.store(self.p_ent, 4, ENTS)
        m.store(self.sym("eventMng"), 4, EVENT_MNG)
        self.field_type = 0
        self.field_attr = 0
        self.va = checks.scene.va
        self.calls = []
        self.hit = []
        self.goal = []

        def name(v):
            v &= 0xFFFFFFFF
            if v == 0:
                return 0
            if T.SCN <= v < T.SCN + 0x8000 and (v - T.SCN) % 0x1000 == 0:
                return f"c{(v - T.SCN) // 0x1000}"
            return v

        self.name = name

        def body(ai):
            return name(m.load(ai + 0x18, 4))

        def rec(*a):
            self.calls.append(list(a))
            return 0

        def vec(a):
            return [m.load(a + 4 * i, 4) for i in range(4)]

        def hit(mm, a, b, c, *_):
            r = self.hit.pop(0) if self.hit else F_M1
            rec("ccHitCheckLM", vec(a), vec(b), c)
            mm.f[0] = r
            return r

        def goal(mm, ai, p, *_):
            r = self.goal.pop(0) if self.goal else 0
            rec("CheckGoalBeaconPos", body(ai), vec(p))
            return r & 0xFFFFFFFF

        def path(mm, navi, a, b, *_):
            r = self.goal.pop(0) if self.goal else 0
            rec("PathFinding", body(navi - 0xF0), vec(a), vec(b))
            return r & 0xFFFFFFFF

        hooks = {
            "ccSkillRequest__FP6ccCharP6ccChari": lambda mm, a, b, c, *_: rec("ccSkillRequest", name(a), name(b),
                                                                               s32(c)),
            "ccUseItemRequest__FP6ccCharP6ccCharii": lambda mm, a, b, c, d: rec("ccUseItemRequest", name(a), name(b),
                                                                                 s32(c), s32(d)),
            "FollowTarget__4ccAIFP6ccChar": lambda mm, a, b, *_: rec("FollowTarget", body(a), name(b)),
            "FollowBeacon__4ccAIFv": lambda mm, a, *_: rec("FollowBeacon", body(a)),
            "CheckGoalBeaconPos__4ccAIFPf": goal,
            "Attack__8ccPlayerFP6ccChari": lambda mm, a, b, c, *_: rec("PlayerAttack", name(a), name(b), s32(c)),
            "ccHitCheckLM__FPfPfUi": hit,
            "ManualModeAI__9ccSpcCharFi": lambda mm, a, b, *_: rec("ManualModeAI", name(a), s32(b)),
            "GetFieldType__9WORLD_MANFv": lambda mm, *_: self.field_type & 0xFFFFFFFF,
            "GetFieldAttrb__9WORLD_MANFv": lambda mm, *_: self.field_attr & 0xFFFFFFFF,
            "ManualControl__4ccAIFv": lambda mm, a, *_: rec("ManualControl", body(a)),
            "ActInTown__4ccAIFv": lambda mm, a, *_: rec("ActInTown", body(a)),
            "ChatMessageSender__4ccAIFv": lambda mm, a, *_: rec("ChatMessageSender", body(a)),
            "TransferOut__9ccSpcCharFv": lambda mm, a, *_: rec("TransferOut", name(a)),
            "FollowPlayer__4ccAIFv": lambda mm, a, *_: rec("FollowPlayer", body(a)),
            "LeavePlayer__4ccAIFv": lambda mm, a, *_: rec("LeavePlayer", body(a)),
            "FollowTargetDirc__4ccAIFP6ccChar": lambda mm, a, b, *_: rec("FollowTargetDirc", body(a), name(b)),
            "PathFindingInDungeon__6ccNaviFPfPf": path,
            "DEG2RAD__Fs": lambda mm, a, *_: rec("FaceTalk", self.cur, s16(a)),
            "ccSetDirc__FPffi": lambda mm, *_: 0,
        }
        for n, f in hooks.items():
            m.hooks[self.sym(n)] = f
        # the party manager is laid out for real: checkPartyMenberNum runs
        m.hooks.pop(self.sym("checkPartyMenberNum__Fi"), None)
        # every chat line: its name, the AI's member, its int arguments
        for s in self.g.prog.functions():
            n = s.name
            if not n.startswith("ChatMessage") or not n.endswith(("Fv", "Fi", "FP6ccChar", "FP6ccChari",
                                                                  "FP6ccCharii", "FPc")):
                continue
            if n.startswith(("ChatMessage__", "ChatMessageModify__", "ChatMessageSender__")):
                continue
            short = n.split("__")[0]
            sig = n.split("__4ccAIF")[1]

            def make(short, sig):
                def h(mm, ai, a, b, c, *_):
                    args = {"v": [], "i": [s32(a)], "P6ccChar": [name(a)], "P6ccChari": [name(a), s32(b)],
                            "P6ccCharii": [name(a), s32(b), s32(c)], "Pc": ["s"]}[sig]
                    return rec(short, body(ai), *args)
                return h
            m.hooks[s.value] = make(short, sig)
        # the remarks Mutation adds, unnamed in its code
        for va_, (short, item) in volume.remarks().items():
            m.hooks[va_] = (lambda short, item: lambda mm, ai, a, *_:
                            rec(short, body(ai), *([s32(a)] if item else [])))(short, item)

    # memory ---------------------------------------------------------------------------
    def put_msg(self, a, msg):
        name, param, ptr, state, sender, receiver, prio, dt = msg
        self.g.put(a, "<iiIhHHHI", name, param, self.va(ptr) if ptr >= 0 else 0, state, sender, receiver, prio, dt)

    def read_msg(self, a):
        name, param, ptr, state, sender, receiver, prio, dt = self.g.get(a, "<iiIhHHHI")
        return [name, param, self.index(ptr), state, sender, receiver, prio, dt]

    def index(self, v):
        if v == 0:
            return -1
        if T.SCN <= v < T.SCN + 0x8000 and (v - T.SCN) % 0x1000 == 0:
            return (v - T.SCN) // 0x1000
        return -1000 - v

    def put_ai(self, key, ai):
        a = AIS + 0x400 * key
        self.m.mem[a:a + 0x260] = bytes(0x260)
        for (n, off, f), v in zip(AI_LAYOUT, ai):
            if f == "P":
                self.m.store(a + off, 4, SPC_AI_PARAM + 0x24 * v)
            elif f == "C":
                self.m.store(a + off, 4, self.va(v) if v >= 0 else 0)
            elif f == "E":
                e = a + off
                self.g.put(e, "<hhhhh", *v[:5])
                for i, msg in enumerate(v[5]):
                    self.put_msg(e + 0xC + 0x18 * i, msg)
            elif f == "<4I":
                self.g.put(a + off, f, *v)
            else:
                self.g.put(a + off, f, v)

    def read_ai(self, key):
        a = AIS + 0x400 * key
        out = []
        for n, off, f in AI_LAYOUT:
            if f == "P":
                out.append((self.m.load(a + off, 4) - SPC_AI_PARAM) // 0x24)
            elif f == "C":
                out.append(self.index(self.m.load(a + off, 4)))
            elif f == "E":
                e = a + off
                out += list(self.g.get(e, "<hhhhh"))
                out += [self.read_msg(e + 0xC + 0x18 * i) for i in range(10)]
            elif f == "<4I":
                out += list(self.g.get(a + off, f))
            else:
                out.append(self.g.get(a + off, f)[0])
        return out

    def put_sys(self, s):
        g = self.g
        g.put(AISYS, "<Ihhh", s["time"], s["entry_num"], s["history_top"], s["next_id"])
        for i, msg in enumerate(s["buff"]):
            self.put_msg(AISYS + 0xC + 0x18 * i, msg)
        for i, msg in enumerate(s["hist"]):
            self.put_msg(AISYS + 0x18C + 0x18 * i, msg)
        for i, k in enumerate(s["entry"]):
            self.m.store(AISYS + 0x30C + 4 * i, 4, AIS + 0x400 * k + 0x160 if k >= 0 else 0)
            self.m.store(AISYS + 0x320 + 4 * i, 4, AIS + 0x400 * k if k >= 0 else 0)

    def read_sys(self):
        g = self.g
        time, en, ht, nid = g.get(AISYS, "<Ihhh")
        buff = [self.read_msg(AISYS + 0xC + 0x18 * i) for i in range(16)]
        hist = [self.read_msg(AISYS + 0x18C + 0x18 * i) for i in range(16)]
        entry = []
        for i in range(5):
            v = self.m.load(AISYS + 0x30C + 4 * i, 4)
            entry.append((v - 0x160 - AIS) // 0x400 if v else -1)
        return [time, en, ht, nid, buff, hist, entry]

    def put_world(self, w):
        c, g, m = self.c, self.g, self.m
        chars = w["chars"]
        c.scene.put(chars, w["pcs"], w["enes"])
        unused = volume.unused_list()
        if unused is not None:                  # Mutation's candidate list, as the port's Crew starts it
            m.mem[unused:unused + 33 * 4] = bytes(33 * 4)
        for i, ch in enumerate(chars):
            va = self.va(i)
            base = m.load(va, 4)
            g.put(va + 0x40, "<4I", *ch.pos)
            m.store(base + 0x18, 4, ch.height)
            m.store(va + 0x128, 4, AIS + 0x400 * i if i in w["ais"] else 0)
            s = ch.spc
            g.put(va + 0xEE, "<h", s["act_num"])
            m.store(va + 0x78, 4, self.va(s["target_char"]) if s["target_char"] >= 0 else 0)
            m.store(va + 0xE0, 1, m.load(va + 0xE0, 1) | (8 if s["move_flag"] else 0))
            m.store(va + 0x10C, 4, s["now_speed"])
            g.put(va + 0x11C, "<i", s["cycle"])
            m.store(va + 0x20C, 4, s["dist_tg"])
            g.put(va + 0xE8, "<i", s["spc_list_num"])
            m.store(va + 0xE0, 1, (m.load(va + 0xE0, 1) & ~0x70) | (0x20 if s["run_flag"] else 0)
                    | (0x40 if s["ghost"] else 0) | (0x10 if s["stop_flag"] else 0))
            g.put(va + 0xFC, "<h", s["stop_cnt"])
            g.put(va + 0x124, "<i", s["walk_run_cnt"])
            vt = VT_PLAYER if ch.type & 1 else VT_FELLOW if ch.type & 4 else VT_SPC
            m.store(va + 0x1F4, 4, vt)
            if ch.type & 7:
                g.put_spc(ch, g.spc[ch.id])        # ccGetCharParam's copy
        ents = w["ents"]
        m.store(ENTS + 16, 4, len(ents))
        m.store(ENTS + 20, 4, self.va(ents[0]) if ents else 0)
        for a, b in zip(ents, ents[1:] + [None]):
            m.store(self.va(a) + 0x1C4, 4, self.va(b) if b is not None else 0)
        p = w["party"]
        for n in range(3):
            m.store(self.p_party + 4 * n, 4, self.va(p["members"][n]) if p["members"][n] >= 0 else 0)
            g.put(self.p_party + 0xC + 4 * n, "<i", p["ids"][n])
        g.put(self.p_party + 0x18, "<i", p["num"])
        gm = w["game"]
        g.put(TB.GAME + 0x58, "<i", gm["in_battle"])
        g.put(TB.GAME + 0x14, "<i", gm["area"])
        g.put(TB.GAME + 0x28, "<i", gm["field"])
        g.put(TB.GAME + 0x24, "<i", gm["field_24"])
        self.field_type, self.field_attr = gm["field_type"], gm["field_attr"]
        g.env.menu_type = gm["menu_type"]
        g.put(EVENT_MNG + 0x78C, "<i", gm["event_lock"])
        g.put(self.g_sbc, "<h", gm["spc_battle_condition"])
        g.put(self.g_pstrat, "<i", gm["party_strategy"])
        m.store(self.p_player, 4, self.va(gm["player"]) if gm["player"] >= 0 else 0)
        g.put(self.g_pgride, "<i", gm["pg_ride_flag"])
        for id_, (skills, items) in w["lists"].items():
            g.put(TB.SAVE + 0x1EC4 + 40 * id_, "<20h", *skills)
            for j, (i, cat, n) in enumerate(items):
                g.put(TB.SAVE + 0x30 + 160 * id_ + 4 * j, "<hbb", i, cat, n)
        for k, ai in w["ais"].items():
            self.put_ai(k, ai)
        self.put_sys(w["sys"])
        sel, oth, heal, oca = w["globals"]
        g.put(self.g_sel, "<i", sel)
        g.put(self.g_other, "<i", oth)
        g.put(self.g_heal, "<i", heal)
        g.put(self.g_ocarina, "<i", oca)
        g.set_rand(w["rand"])
        self.hit = list(w["hit"])
        self.goal = list(w["goal"])
        self.calls = []
        g.calls = []

    def read_world(self, w, ret):
        g, m = self.g, self.m
        spc = {}
        for i, ch in enumerate(w["chars"]):
            va = self.va(i)
            spc[str(i)] = [g.get(va + 0xEE, "<h")[0], self.index(m.load(va + 0x78, 4)),
                           (m.load(va + 0xE0, 1) >> 3) & 1, m.load(va + 0x10C, 4), g.get(va + 0x11C, "<i")[0],
                           m.load(va + 0x20C, 4), g.get(va + 0xE8, "<i")[0], (m.load(va + 0xE0, 1) >> 5) & 1,
                           (m.load(va + 0xE0, 1) >> 6) & 1, (m.load(va + 0xE0, 1) >> 4) & 1,
                           g.get(va + 0xFC, "<h")[0], g.get(va + 0x124, "<i")[0]]
        items = {}
        for id_ in w["lists"]:
            v = []
            for j in range(40):
                v += list(g.get(TB.SAVE + 0x30 + 160 * id_ + 4 * j, "<hbb"))
            items[str(id_)] = v
        return {"ret": ret, "rand": g.rand_now(), "calls": self.calls,
                "ais": {str(k): self.read_ai(k) for k in w["ais"]}, "sys": self.read_sys(), "spc": spc,
                "items": items,
                "globals": [g.get(self.g_sel, "<i")[0], g.get(self.g_other, "<i")[0], g.get(self.g_heal, "<i")[0],
                            g.get(self.g_ocarina, "<i")[0]]}

    def run(self, w, fn, args, flt=None):
        self.put_world(w)
        self.cur = self.name(self.va(w.get("me", 0)))
        if flt is not None:
            self.m.f[12] = flt
        return self.m.call(fn if isinstance(fn, int) else self.sym(fn), args)


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


# serialising for the probe ------------------------------------------------------------

def ser_msg(msg):
    return " ".join(str(x) for x in msg)


def ser_ai(ai):
    out = []
    for (n, off, f), v in zip(AI_LAYOUT, ai):
        if f == "E":
            out += [str(x) for x in v[:5]] + [ser_msg(x) for x in v[5]]
        elif f == "<4I":
            out += [str(x) for x in v]
        else:
            out.append(str(v))
    return " ".join(out)


def ser_world(w, b):
    chars = w["chars"]
    out = [str(len(chars))]
    for ch in chars:
        s = ch.spc
        out.append(T.ser_char(ch, b))
        out.append(" ".join(str(x) for x in list(ch.pos) + [ch.height] + [s[k] for k in SPC_FIELDS]))
    for lst in (w["pcs"], w["enes"], w["ents"]):
        out.append(" ".join(str(x) for x in [len(lst)] + list(lst)))
    p = w["party"]
    out.append(" ".join(str(x) for x in p["members"] + p["ids"] + [p["num"]]))
    out.append(" ".join(str(w["game"][k]) for k in GAME_FIELDS))
    out.append(str(len(w["lists"])))
    for id_, (skills, items) in w["lists"].items():
        out.append(" ".join(str(x) for x in [id_] + list(skills) + [v for it in items for v in it]))
    out.append(str(len(w["ais"])))
    for k, ai in w["ais"].items():
        out.append(f"{k} {ser_ai(ai)}")
    s = w["sys"]
    out.append(" ".join(str(x) for x in (s["time"], s["entry_num"], s["history_top"], s["next_id"])))
    out += [ser_msg(x) for x in s["buff"]] + [ser_msg(x) for x in s["hist"]]
    out.append(" ".join(str(x) for x in s["entry"]))
    out.append(" ".join(str(x) for x in w["globals"]))
    out.append(str(w["rand"]))
    out.append(" ".join(str(x) for x in [len(w["hit"])] + w["hit"]))
    out.append(" ".join(str(x) for x in [len(w["goal"])] + w["goal"]))
    return " ".join(out)


def request(fn, args, w, b):
    return f"pai {fn} {len(args)} {' '.join(str(a) for a in args)} {ser_world(w, b)}"


# random worlds -------------------------------------------------------------------------

class Pools:
    """Skill and item ids by what the AI makes of them."""

    def __init__(self, prog, data):
        def u32(va):
            return prog.u32(va)

        self.ty = [s32(u32(inf_va(0x0061F4A0) + 0x38 * i + 0x2C)) for i in range(304)]
        self.cost = [s32(u32(inf_va(0x0061F4A0) + 0x38 * i + 0x28)) for i in range(304)]

        def check_type(sid):
            if sid == 1:
                return 0
            t = self.ty[sid] if 0 <= sid < 304 else 0
            if t & 1:
                return 0 if t & 0x1C00 else -1
            if t & 2:
                if t & 0x40000:
                    return 3
                if t & 0x10000:
                    return 2
                return -2 if t & 0x20000 else 1
            return -1

        self.check_type = check_type
        by = {}
        for i in range(304):
            by.setdefault(check_type(i), []).append(i)
        self.arts = [i for i in by.get(0, []) if self.cost[i] > 0]
        self.spells = by.get(1, [])
        self.buffs = by.get(2, [])
        self.debuffs = by.get(-2, [])
        self.heals = by.get(3, [])
        tbl = [(inf_va(0x00623720), 24), (inf_va(0x00623900), 72), (inf_va(0x00623EA0), 34), (inf_va(0x00624150), 3), (inf_va(0x00624190), 22),
               (inf_va(0x00624350), 291)]
        self.items = []             # (id, cat, skill)
        for k, (va, n) in enumerate(tbl):
            for i in range(n):
                self.items.append((i, 10 + k, s32(u32(va + 0x14 * i + 8))))
        self.item_by_skill = {}
        for it in self.items:
            self.item_by_skill.setdefault(it[2], []).append(it)

    def skill(self, rnd):
        k = rnd.random()
        if k < 0.35:
            return -1
        pool = rnd.choice((self.arts, self.spells, self.buffs, self.debuffs, self.heals,
                           [150, 151, 152, 153, 154, 155], [178, 179, 180], [178, 179, 180, 1]))
        return rnd.choice(pool) if pool else -1

    def item(self, rnd):
        if rnd.random() < 0.4:
            return (-1, -1, 0)
        k = rnd.random()
        if k < 0.5:
            sk = rnd.choice((150, 151, 152, 153, 154, 155, 295, 178, 179, 180, 181, 185, 156, 160, 163,
                             rnd.choice(self.spells)))
            cands = self.item_by_skill.get(sk)
            it = rnd.choice(cands) if cands else rnd.choice(self.items)
        else:
            it = rnd.choice(self.items)
        return (it[0], it[1], rnd.choice((1, 1, 2, 3, 0, 5)))


def rnd_msg(rnd, ids, keys, time, pending):
    kind = rnd.choice(KINDS + (0x1,))
    tid = rnd.choice(ids + [0xFFFF, rnd.randrange(0, 5)])
    if kind == 1:
        name = rnd.choice((0x10008, 0x10009, 0x1000A, 0x10013))
    else:
        name = (kind << 16) | (tid & 0xFFFF)
        if tid == 0xFFFF:
            name = -1 if rnd.random() < 0.5 else name
    ptr = rnd.choice(keys + [-1, -1])
    param = rnd.choice((0, -1, rnd.randrange(0, 600), rnd.randrange(150, 200), 0xB0000 | rnd.randrange(0, 20)))
    if kind == 6:       # a skill use carries a skill, as the game's senders give it
        param = rnd.choice((0, rnd.randrange(0, 304), rnd.randrange(150, 200)))
    elif kind == 7:     # an item use an item
        param = 0xB0000 | rnd.randrange(0, 20)
    if pending:
        state = 1
        dt = (time + rnd.randrange(0, 45)) & 0xFFFFFFFF
    else:
        state = rnd.choice((0, 0, 0, 1))
        dt = (time - rnd.choice((0, 1, 5, 30, 89, 90, 91, 179, 180, 181, rnd.randrange(0, 400)))) & 0xFFFFFFFF
    sender = rnd.choice(ids + [0xFFFF])
    receiver = rnd.choice(ids + [0xFFFF])
    return [s32(name), param, ptr, state, sender, receiver, rnd.choice((0, 0, 1)), dt]


class PartyChecks:
    def __init__(self, checks=None):
        self.c = checks or T.Checks()
        self.b = self.c.b
        self.tb = self.c.tb
        self.data = self.c.data
        self.ai = AiGame(self.c)
        self.pools = Pools(self.c.game.prog, self.data)

    def reset(self):
        self.c.reset()

    # a world ---------------------------------------------------------------------------
    def world(self, rnd, nparty=None, nfoes=None, hurt=0.3, conds=0.3, no_free=False):
        """A random world. `no_free` keeps free arts and attack spells (the
        normal attack) out of the skill lists: SelectAttackSkill divides by
        a kept candidate's cost, and eemu leaves LO as it was on a division
        by zero where the EE gives -1 or 1."""
        b, tb, data = self.b, self.tb, self.data
        chars = []
        nparty = nparty if nparty is not None else rnd.choice((1, 2, 3, 3, 3, 3))
        ids = [0] + rnd.sample(range(1, 18), 2)
        if rnd.random() < 0.1:
            ids[0] = rnd.randrange(1, 18)
        members = []
        for n in range(nparty):
            pc = tb.rnd_pc(b, data, rnd)
            pc.id = ids[n]
            pc.type = (1 if rnd.random() < 0.9 else 7) if n == 0 else rnd.choice((4, 4, 4, 6, 2))
            pc.party_flag = rnd.choice((1, 1, 1, 1, 0, 2, 7))
            pc.cond = [0] * 16 if rnd.random() > conds else pc.cond
            pc.cond[0] = rnd.choice((0,) * 8 + (1, 2, 5))
            if rnd.random() < hurt:
                pc.HP = rnd.randrange(0, max(pc.maxHP, 1) + 1)
            members.append(len(chars))
            chars.append(pc)
        if rnd.random() < 0.15:           # a party member off the party
            pc = tb.rnd_pc(b, data, rnd)
            pc.id = rnd.choice([i for i in range(18) if i not in ids[:nparty]])
            pc.type = rnd.choice((4, 2, 6))
            pc.party_flag = rnd.choice((0, 1))
            chars.append(pc)
        used = set()
        nfoes = nfoes if nfoes is not None else rnd.randrange(0, 5)
        for _ in range(nfoes):
            f = tb.rnd_foe(b, data, rnd)
            while (f.kind, f.id) in used:
                f = tb.rnd_foe(b, data, rnd)
            used.add((f.kind, f.id))
            f.cond[0] = rnd.choice((0,) * 8 + (1, 2))
            f.ent_root = rnd.choice((1, 1, 1, 0))
            chars.append(f)
        for ch in chars:
            ch.pos_p = T.rnd_pos(rnd)
            ch.pos = T.rnd_pos(rnd) if rnd.random() < 0.5 else list(ch.pos_p)
            ch.width = rnd.choice((0, fl(rnd.uniform(0, 8))))
            ch.height = fl(rnd.uniform(0, 30))
            ch.skill_id = rnd.choice((0, 0, 1, rnd.randrange(0, 300)))
            ch.skill_status = rnd.choice((0, 0, 1, rnd.randrange(0, 10)))
            ch.anm_flag = rnd.choice((0, 1))
            ch.spc = {"act_num": rnd.choice((0, 0, 1, 4, 5, 6, 7, 8, 9, 14, 15, 16, rnd.randrange(0, 20))),
                      "target_char": rnd.choice([-1, -1] + list(range(len(chars)))),
                      "move_flag": rnd.choice((0, 0, 1)), "now_speed": fl(rnd.uniform(0, 10)),
                      "cycle": rnd.choice((0, 180, 360, rnd.randrange(0, 100000))),
                      "dist_tg": fl(rnd.uniform(-2, 400)), "spc_list_num": rnd.choice((0, 1, 1)),
                      "run_flag": rnd.choice((0, 1)), "ghost": rnd.choice((0, 0, 1)),
                      "stop_flag": rnd.choice((0, 1)), "stop_cnt": rnd.choice((0, 14, 15, rnd.randrange(0, 100))),
                      "walk_run_cnt": rnd.choice((80, 125, 170, 35, rnd.randrange(0, 300)))}
        pcs = [i for i, ch in enumerate(chars) if ch.type & 7 and rnd.random() > 0.05]
        enes = [i for i, ch in enumerate(chars) if not ch.type & 7 and rnd.random() > 0.05]
        ents = [i for i in enes if rnd.random() > 0.1]
        party = {"members": members + [-1] * (3 - len(members)),
                 "ids": [chars[i].id for i in members] + [-1] * (3 - len(members)), "num": len(members)}
        if rnd.random() < 0.05 and nparty == 3:
            party["members"][rnd.randrange(1, 3)] = -1
        game = {"in_battle": rnd.choice((1, 1, 0)), "area": rnd.choice((1, 1, 2, 0)), "field": rnd.choice((0, 1)),
                "field_type": rnd.choice((4, 4, 0, 1)), "field_attr": rnd.randrange(0, 6),
                "menu_type": rnd.choice((-1, -1, -1, 0, 3)), "event_lock": rnd.choice((0, 0, 0, 1)),
                "spc_battle_condition": rnd.choice((0, 1, 3, 5, 2)), "party_strategy": rnd.randrange(0, 6),
                "player": members[0], "field_24": rnd.choice((0, 13)), "pg_ride_flag": rnd.choice((0, 0, 0, 1))}
        pools = self.pools
        lists = {}
        for i, ch in enumerate(chars):
            if ch.type & 7:
                skills = [pools.skill(rnd) for _ in range(20)]
                if no_free:
                    skills = [-1 if x >= 0 and pools.cost[x] == 0 and pools.check_type(x) in (0, 1) else x
                              for x in skills]
                items = [pools.item(rnd) for _ in range(40)]
                lists[ch.id] = (skills, items)
        # the AIs and the bus
        keys = [i for i in members if i > 0 or rnd.random() < 0.4]
        keys += [i for i, ch in enumerate(chars) if ch.type & 7 and i not in members and rnd.random() < 0.5]
        time = rnd.choice((rnd.randrange(300, 100000), rnd.randrange(0, 300)))
        next_id = rnd.randrange(0, 50)
        ais, entry = {}, [-1] * 5
        ids_ = []
        slots = rnd.sample(range(5), min(5, len(keys)))
        for n, k in enumerate(keys):
            enter = rnd.random() < 0.9
            aid = (next_id + n * rnd.choice((1, 2))) if enter else -1
            if enter and n < len(slots):
                entry[slots[n]] = k
                ids_.append(aid)
            else:
                aid = -1
            ais[k] = [aid]
        allkeys = list(range(len(chars)))
        for k in list(ais):
            ais[k] = self.rnd_ai(rnd, k, chars, ais[k][0], ids_, allkeys, time)
        for i, ch in enumerate(chars):
            ch.ai = int(i in ais)
        sys_ = {"time": time, "entry_num": sum(1 for e in entry if e >= 0), "history_top": rnd.randrange(0, 16),
                "next_id": next_id + 2 * len(keys),
                "buff": [rnd_msg(rnd, ids_, allkeys, time, True) if rnd.random() < 0.3 else
                         [-1, 0, -1, 0, 0xFFFF, 0xFFFF, 0, 0] for _ in range(16)],
                "hist": [rnd_msg(rnd, ids_, allkeys, time, False) for _ in range(16)], "entry": entry}
        if rnd.random() < 0.1:              # a full buffer
            sys_["buff"] = [rnd_msg(rnd, ids_, allkeys, time, True) for _ in range(16)]
        return {"chars": chars, "pcs": pcs, "enes": enes, "ents": ents, "party": party, "game": game,
                "lists": lists, "ais": ais, "sys": sys_,
                "globals": [rnd.randrange(-1, 50), rnd.randrange(-1, 5), rnd.randrange(-1, 5),
                            rnd.choice((0, 0, 0, rnd.randrange(1, 18)))],
                "rand": rnd.getrandbits(64),
                "hit": [rnd.choice((F_M1, F_M1, fl(0.5), 0)) for _ in range(rnd.randrange(0, 20))],
                "goal": [rnd.choice((0, 1)) for _ in range(rnd.randrange(0, 6))]}

    def rnd_ai(self, rnd, key, chars, aid, ids, keys, time):
        ch = chars[key]
        flags = 0
        for bit, p in ((0, 0.1), (1, 0.3), (2, 0.2), (3, 0.2), (4, 0.1), (5, 0.2), (6, 0.1), (7, 0.25), (15, 0.4),
                       (16, 0.2)):
            if rnd.random() < p:
                flags |= 1 << bit
        flags |= rnd.randrange(4) << 8 | rnd.randrange(4) << 10 | rnd.randrange(8) << 12
        flags |= rnd.choice((0, 0xFF, 4, 1, 2, 3, 8, 16, rnd.randrange(256))) << 24
        msgs = [rnd_msg(rnd, ids, keys, time, False) for _ in range(10)]
        entry = [aid, 10, rnd.randrange(0, 11), rnd.randrange(0, 10), rnd.randrange(0, 10), msgs]
        r = rnd.randrange
        f = lambda: fl(rnd.uniform(0, 5000))       # noqa: E731
        return [flags, r(0, 6), r(0, 6), r(0, 6), r(0, 6), rnd.choice((0, 90, 180, r(0, 100000))),
                ch.id if rnd.random() < 0.9 else r(0, 18), key, rnd.choice([-1] + keys), rnd.choice([-1] + keys),
                r(0, 5), rnd.choice((-1, -1, 5, 16, 99, 11, 3, 9, r(0, 20))), rnd.choice((-1, r(0, 20))),
                rnd.choice((r(150, 200), r(0, 304))), r(-1, 100), r(-1, 200), r(0, 1000), r(0, 50), r(0, 100),
                r(0, 100), r(0, 100), r(0, 10), r(0, 10), r(0, 8), r(0, 8), r(0, 5), r(0, 5), r(0, 5),
                r(0, 5), r(0, 65536), r(0, 100), r(0, 5),
                rnd.choice((f(), fl(3500.0), fl(1750.0))), f(), rnd.choice((f(), fl(rnd.uniform(0, 300)))), f(),
                r(0, 10), r(0, 10), r(0, 10), rnd.choice((ch.level, ch.level - 1, ch.level + 1, r(0, 100))),
                f(), f(), f(), f(), [f() for _ in range(4)], [f() for _ in range(4)], rnd.choice((0, 1)), entry] + (
                    [] if volume.NAME == "infection" else
                    [rnd.choice((0, 0, r(0, 300), r(0, 3000))), rnd.choice((0, r(0, 300), r(0, 3000))),
                     rnd.choice([-1] + keys)])

    def me(self, rnd, w):
        return rnd.choice(list(w["ais"])) if w["ais"] else None

    def case(self, rnd, fn, mangled, argf, ptr=False, void=False, tweak=None, fret=False, **kw):
        """A world, a member with an AI, the arguments; both sides. `ptr`: the
        function returns a character; `tweak(rnd, w, me)` steers the world."""
        while True:
            w = self.world(rnd, **kw)
            me = self.me(rnd, w)
            if me is not None:
                break
        if tweak:
            tweak(rnd, w, me)
        w["me"] = me
        args, gargs, flt = argf(rnd, w, me)
        va = AIS + 0x400 * me
        ret = self.ai.run(w, mangled, (va,) + tuple(a & 0xFFFFFFFF for a in gargs), flt)
        ret = (0 if void else self.ai.index(ret) if ptr else self.ai.m.f[0] if fret else s32(ret))
        out = self.ai.read_world(w, ret)
        self.c.scene.restore()
        return request(fn, [me] + args, w, self.b), out, fn


def chars_arg(w, rnd, allow_none=False):
    ks = list(range(len(w["chars"])))
    if allow_none and rnd.random() < 0.1:
        return -1
    return rnd.choice(ks)


def cva(ai, i):
    return ai.va(i) if i >= 0 else 0


def make_checks(pc):
    """(name, function) pairs: each function takes rnd and returns (req, game, label)."""
    ai = pc.ai
    pools = pc.pools

    ptrs = {"CheckBossEntry", "CheckSkillOtherSPC", "CheckHealSkillOtherSPC", "SearchTarget", "SearchTargetNear",
            "SearchDebuffUnusedEnemy", "SearchBuffUnusedFellow"}

    def simple(fn, mangled, argf, **kw):
        kw.setdefault("ptr", fn in ptrs)
        if fn in ("CureSPC", "CureOtherSPC", "CurePoisonSPC", "CurePoisonCurseSPC", "ItemFirst", "HealSPC",
                  "ResurrectSPC", "HealPlzNormalMode", "HealPlzBattleMode", "CureOnlyCharmConfusionOtherSPC",
                  "CureOnlyParalysisSleepOtherSPC"):
            kw.setdefault("tweak", lambda r, w, m: (calm(r, w, w["party"]["members"], 0.4), open_up(r, w, 0x1F)))
        kw.setdefault("void", fn in ("ChangeMode", "ChangeStrategy", "ChangeStrategyCMD", "AttackTarget"))
        return lambda rnd: pc.case(rnd, fn, mangled, argf, **kw)

    def no_args(rnd, w, me):
        return [], [], None

    def item_first_arg(rnd, w, me):
        """From Mutation on the cures' and ResurrectSPC's argument: the item
        before the skill."""
        v = 0 if volume.NAME == "infection" else rnd.choice((0, 0, 1))
        return [v], [v], None

    def sid_arg(pool):
        def f(rnd, w, me):
            s = rnd.choice(pool(rnd))
            return [s], [s], None
        return f

    def plan_arg(pool):
        """The skill, and from Mutation on the check argument (0 plan, 1
        report, 2 report with the SP alone)."""
        def f(rnd, w, me):
            s = rnd.choice(pool(rnd))
            chk = 0 if volume.NAME == "infection" else rnd.choice((0, 0, 1, 2))
            return [s, chk], [s, chk], None
        return f

    anysid = lambda rnd: [rnd.randrange(0, 304), rnd.choice((150, 178, 179, 180)), pools.skill(rnd) & 0x1FF]  # noqa

    def others_sid(rnd, w, me):
        """A skill another member has (as a skill or an item's), often."""
        pool = []
        for n, k in enumerate(w["party"]["members"]):
            if k >= 0 and k != me:
                sk, items = w["lists"][w["chars"][k].id]
                pool += [x for x in sk if x >= 0]
                pool += [it[2] for it in pools.items if (it[0], it[1]) in {(i, c) for i, c, _ in items}]
        s_ = rnd.choice(pool) if pool and rnd.random() < 0.8 else rnd.choice(anysid(rnd))
        return [s_], [s_ & 0xFFFFFFFF], None

    def item_code(rnd, w, me):
        body = w["chars"][me]
        items = w["lists"].get(body.id, ([], []))[1]
        have = [it for it in items if it[1] in range(10, 16) and it[0] != -1]
        if have and rnd.random() < 0.7:
            i, c, _ = rnd.choice(have)
        else:
            i, c, _ = rnd.choice(pools.items)
        return (c << 16) | (i & 0xFFFF)

    def code_arg(rnd, w, me):
        v = item_code(rnd, w, me)
        return [v], [v], None

    def rate_or_flag(rnd):
        """CheckNeedHealing's and CheckHealParty's argument: Infection's flag
        (a2), from Mutation on a rate in f12: (for the probe, for a2, f12)."""
        if volume.NAME == "infection":
            f = rnd.choice((0, 0, 1))
            return f, f, None
        r = fl(rnd.choice((100.0, 100.0, 50.0, 30.0, 75.0, rnd.uniform(0, 120))))
        return s32(r), 0, r

    def tp_flag(rnd, w, me):
        t = chars_arg(w, rnd, True)
        v, a2, f12 = rate_or_flag(rnd)
        return [t, v], [cva(ai, t), a2], f12

    def tp_n(rnd, w, me):
        t = chars_arg(w, rnd, True)
        n = rnd.choice((-1, -2, -3, rnd.randrange(150, 180)))
        return [t, n], [cva(ai, t), n], None

    def target_arg(rnd, w, me):
        t = chars_arg(w, rnd)
        return [t], [cva(ai, t)], None

    def boss_arg(rnd, w, me):
        n = rnd.choice((0, 1))
        return [n], [n], None

    def solution(rnd, w, me):
        k = rnd.choice(KINDS + (0, 1, 12)) << 16
        t = chars_arg(w, rnd, True)
        return [k, t], [k, cva(ai, t)], None

    def from_me(rnd, w, me):
        k = rnd.choice(KINDS) << 16
        t = chars_arg(w, rnd)
        d = rnd.choice((0, 30, 5))
        p = rnd.randrange(-1, 100)
        pp = chars_arg(w, rnd, True)
        return [k, t, d, p, pp], [k, cva(ai, t), d, p, cva(ai, pp)], None

    def search(rnd, w, me):
        tt = rnd.choice((6, 0xE0, 0x60, 0x80, 2, 4, 0xE6, 1))
        force = rnd.choice((0, 0, 1))
        return [tt, force], [tt, force], None

    def count_area(rnd, w, me):
        tt = rnd.choice((6, 0xE0, 0x60, 0xE6))
        r = fl(rnd.choice((0.0, 20.0, 40.0, rnd.uniform(0, 90))))
        return [tt, r], [tt], r

    def mode(rnd, w, me):
        n, f = rnd.randrange(0, 6), rnd.choice((0, 0, 1))
        return [n, f], [n, f], None

    def strat(rnd, w, me):
        n = rnd.randrange(0, 6)
        return [n], [n], None

    def use_skill(rnd, w, me):
        s = rnd.choice(anysid(rnd) + [-1, 304, 180])
        t = chars_arg(w, rnd, True)
        return [s, t], [s & 0xFFFFFFFF, cva(ai, t)], None

    def use_item(rnd, w, me):
        v = item_code(rnd, w, me)
        t = chars_arg(w, rnd, True)
        return [v, t], [v, cva(ai, t)], None

    def cure(rnd, w, me):
        t = rnd.choice([m for m in w["party"]["members"][:w["party"]["num"]] if m >= 0] + [me])
        n = rnd.choice((0, 0, -1, -2, rnd.randrange(156, 175)))
        return [t, n], [cva(ai, t), n], None

    def cure_n(rnd, w, me):
        n = rnd.choice((0, 0, -1, -2, rnd.randrange(156, 175)))
        return [n], [n], None

    def heal(rnd, w, me):
        t = rnd.choice([-1, -1] + w["party"]["members"][:w["party"]["num"]] + [me])
        r = rnd.choice((fl(100.0), 0x420554FE, fl(75.0), fl(rnd.uniform(0, 120))))
        # from Mutation on a third argument: the item before the skill
        first = 0 if volume.NAME == "infection" else rnd.choice((0, 0, 1))
        return [t, r, first], [cva(ai, t), first], r

    def fuse(rnd, w, me):
        s = rnd.choice(anysid(rnd))
        t = chars_arg(w, rnd)
        return [s, t], [s, cva(ai, t)], None

    def fattack(rnd, w, me):
        t = chars_arg(w, rnd)
        n = rnd.randrange(0, 6)
        return [t, n], [cva(ai, t), n], None

    def hp_arg(rnd, w, me):
        v = rnd.choice((1, 100, 150, 151, 400, 401, 800, 801, rnd.randrange(-5, 3000)))
        return [v], [v], None

    def list2(rnd, w, me):
        a, b_ = rnd.choice((0, 1, 2, 3, -1, -2)), rnd.choice((224, 6, 0xFF, 1))
        return [a, b_], [a & 0xFFFFFFFF, b_], None

    def itype(rnd, w, me):
        v = rnd.randrange(0, 10)
        return [v], [v], None

    def tcond(rnd, w, me):
        t = chars_arg(w, rnd)
        s = rnd.choice((rnd.randrange(150, 200), rnd.randrange(0, 304)))
        return [t, s], [cva(ai, t), s], None

    buffs = lambda rnd: pools.buffs + [175, 176, 177]  # noqa: E731
    debuffs = lambda rnd: pools.debuffs + list(range(156, 175))  # noqa: E731
    return [
        ("CheckSkillList", simple("CheckSkillList", "CheckSkillList__4ccAIFi", sid_arg(anysid))),
        ("CheckSkillList2", simple("CheckSkillList2", "CheckSkillList2__4ccAIFii", list2)),
        ("CheckBuffSkillListNum", simple("CheckBuffSkillListNum", "CheckBuffSkillListNum__4ccAIFv", no_args)),
        ("CheckDebuffSkillListNum", simple("CheckDebuffSkillListNum", "CheckDebuffSkillListNum__4ccAIFv", no_args)),
        ("CheckHealHpSkill", simple("CheckHealHpSkill", "CheckHealHpSkill__4ccAIFv", no_args)),
        ("CheckHealParty", simple("CheckHealParty", "CheckHealParty__4ccAIFi",
                                  lambda r, w, m: (lambda v, a2, f12: ([v], [a2], f12))(*rate_or_flag(r)))),
        ("SearchHealSkill", simple("SearchHealSkill", "SearchHealSkill__4ccAIFP6ccChar", target_arg)),
        ("CheckItemList", simple("CheckItemList", "CheckItemList__4ccAIFi", code_arg)),
        ("CheckItemList2", simple("CheckItemList2", "CheckItemList2__4ccAIFi", itype)),
        ("ConsumeItemList", simple("ConsumeItemList", "ConsumeItemList__4ccAIFi", code_arg)),
        ("SearchItemListBySkill", simple("SearchItemListBySkill", "SearchItemListBySkill__4ccAIFi",
                                         sid_arg(lambda r: [150, 151, 152, 178, 179, 180, 181, 156, 160] +
                                                 [r.randrange(0, 304)]))),
        ("SearchItemListByHeal", simple("SearchItemListByHeal", "SearchItemListByHeal__4ccAIFi", hp_arg)),
        ("SearchItemListByBuff", simple("SearchItemListByBuff", "SearchItemListByBuff__4ccAIFv", no_args)),
        ("SearchItemListByDebuff", simple("SearchItemListByDebuff", "SearchItemListByDebuff__4ccAIFv", no_args)),
        ("SearchItemListByMagicAttack", simple("SearchItemListByMagicAttack",
                                               "SearchItemListByMagicAttack__4ccAIFv", no_args)),
        ("CheckNeedHealing", simple("CheckNeedHealing", "CheckNeedHealing__4ccAIFP6ccChari", tp_flag, hurt=0.6)),
        ("CheckConditionMinus", simple("CheckConditionMinus", "CheckConditionMinus__4ccAIFP6ccChari", tp_n,
                                       conds=0.8)),
        ("CheckBossEntry", simple("CheckBossEntry", "CheckBossEntry__4ccAIFi", boss_arg)),
        ("CheckHealingSchedule", simple("CheckHealingSchedule", "CheckHealingSchedule__4ccAIFv", no_args)),
        ("CheckSchedule", simple("CheckSchedule", "CheckSchedule__4ccAIFv", no_args)),
        ("CheckSolution", simple("CheckSolution", "CheckSolution__4ccAIFiP6ccChar", solution)),
        ("SysMsgFromMeToMe", simple("SysMsgFromMeToMe", "SysMsgFromMeToMe__4ccAIFiP6ccChariiP6ccChar", from_me)),
        ("CheckSkillOtherSPC", simple("CheckSkillOtherSPC", "CheckSkillOtherSPC__4ccAIFi", others_sid,
                                      nparty=3, tweak=lambda r, w, m: (calm(r, w, w["party"]["members"]),
                                                                       open_up(r, w, 0x1F)))),
        ("CheckHealSkillOtherSPC", simple("CheckHealSkillOtherSPC", "CheckHealSkillOtherSPC__4ccAIFi", hp_arg,
                                          nparty=3, tweak=lambda r, w, m: (calm(r, w, w["party"]["members"]),
                                                                           open_up(r, w, 4)))),
        ("SearchTarget", simple("SearchTarget", "SearchTarget__4ccAIFii", search)),
        ("SearchTargetNear", simple("SearchTargetNear", "SearchTargetNear__4ccAIFii", search)),
        ("CountTargetInArea", simple("CountTargetInArea", "CountTargetInArea__4ccAIFif", count_area)),
        ("levelCheck", simple("levelCheck", "levelCheck__4ccAIFv", no_args)),
        ("SearchDebuffUnusedEnemy", simple("SearchDebuffUnusedEnemy", "SearchDebuffUnusedEnemy__4ccAIFi",
                                           sid_arg(debuffs))),
        ("SearchBuffUnusedFellow", simple("SearchBuffUnusedFellow", "SearchBuffUnusedFellow__4ccAIFi",
                                          sid_arg(buffs))),
        ("DebuffForUnusedEnemy", simple("DebuffForUnusedEnemy", "DebuffForUnusedEnemy__4ccAIFi", plan_arg(debuffs))),
        ("BuffForUnusedFellow", simple("BuffForUnusedFellow", "BuffForUnusedFellow__4ccAIFi", plan_arg(buffs))),
        ("CheckTargetConditionBySkill", simple("CheckTargetConditionBySkill",
                                               "CheckTargetConditionBySkill__4ccAIFP6ccChari", tcond)),
        ("ChangeMode", simple("ChangeMode", "ChangeMode__4ccAIFii", mode)),
        ("ChangeStrategy", simple("ChangeStrategy", "ChangeStrategy__4ccAIFi", strat)),
        ("ChangeStrategyCMD", simple("ChangeStrategyCMD", "ChangeStrategyCMD__4ccAIFi", strat)),
        ("StopNormalAttack", simple("StopNormalAttack", "StopNormalAttack__4ccAIFv", no_args)),
        ("UseSkill", simple("UseSkill", "UseSkill__4ccAIFiP6ccChar", use_skill,
                            tweak=lambda r, w, m: calm(r, w, [m]))),
        ("UseItem", simple("UseItem", "UseItem__4ccAIFiP6ccChar", use_item)),
        ("SelectAttackSkill", simple("SelectAttackSkill", "SelectAttackSkill__4ccAIFP6ccChar", target_arg,
                                     no_free=True)),
        ("CureSPC", simple("CureSPC", "CureSPC__4ccAIFP6ccChari", cure, conds=0.8)),
        ("CureOtherSPC", simple("CureOtherSPC", "CureOtherSPC__4ccAIFi", cure_n, conds=0.8)),
        ("CureOnlyCharmConfusionOtherSPC", simple("CureOnlyCharmConfusionOtherSPC",
                                                  "CureOnlyCharmConfusionOtherSPC__4ccAIFv", item_first_arg,
                                                  conds=0.8)),
        ("CureOnlyParalysisSleepOtherSPC", simple("CureOnlyParalysisSleepOtherSPC",
                                                  "CureOnlyParalysisSleepOtherSPC__4ccAIFv", item_first_arg,
                                                  conds=0.8)),
        ("CurePoisonSPC", simple("CurePoisonSPC", "CurePoisonSPC__4ccAIFv", no_args, conds=0.8)),
        ("HealSPC", simple("HealSPC", "HealSPC__4ccAIFP6ccCharf", heal, hurt=0.8)),
        ("ResurrectSPC", simple("ResurrectSPC", "ResurrectSPC__4ccAIFv", item_first_arg)),
        ("HealPlzNormalMode", simple("HealPlzNormalMode", "HealPlzNormalMode__4ccAIFv", no_args, hurt=0.6,
                                     conds=0.6)),
        ("HealPlzBattleMode", simple("HealPlzBattleMode", "HealPlzBattleMode__4ccAIFv", no_args, hurt=0.6,
                                     conds=0.6)),
        ("ChatCommandHealPlz", simple("ChatCommandHealPlz", "ChatCommandHealPlz__4ccAIFv", no_args, hurt=0.6,
                                      conds=0.6, tweak=lambda r, w, m: (calm(r, w, [m], 0.8), open_up(r, w, 4)))),
        ("AttackTarget", simple("AttackTarget", "AttackTarget__4ccAIFP6ccChar", target_arg, no_free=True)),
    ] + ([] if volume.NAME == "infection" else [
        # functions later volumes add, found by their callers
        ("ItemFirst", simple("ItemFirst", volume.callee("HealPlzNormalMode__4ccAIFv", 0), no_args, hurt=0.6,
                             conds=0.5)),
        ("CurePoisonCurseSPC", simple("CurePoisonCurseSPC", volume.callee("HealPlzNormalMode__4ccAIFv", 1),
                                      item_first_arg, conds=0.8)),
    ])


def fellow_checks(pc):
    """ccFellow's UseSkill and Attack, called on the member's ccChar."""
    ai = pc.ai

    def case(rnd, fn, mangled, argf):
        while True:
            w = pc.world(rnd, no_free=True)
            me = pc.me(rnd, w)
            if me is not None and w["chars"][me].type & 4:
                break
        args, gargs = argf(rnd, w, me)
        ret = ai.run(w, mangled, (ai.va(me),) + tuple(a & 0xFFFFFFFF for a in gargs))
        out = ai.read_world(w, s32(ret))
        pc.c.scene.restore()
        return request(fn, [me] + args, w, pc.b), out, fn

    def fuse(rnd, w, me):
        s = rnd.choice((rnd.randrange(0, 304), 150, 1, rnd.choice(pc.pools.spells)))
        t = chars_arg(w, rnd)
        return [s, t], [s, cva(ai, t)]

    def fattack(rnd, w, me):
        t = chars_arg(w, rnd)
        n = rnd.randrange(0, 6)
        return [t, n], [cva(ai, t), n]

    return [
        ("FellowUseSkill", lambda rnd: case(rnd, "FellowUseSkill", "UseSkill__8ccFellowFiP6ccChar", fuse)),
        ("FellowAttack", lambda rnd: case(rnd, "FellowAttack", "Attack__8ccFellowFP6ccChari", fattack)),
    ]


def sys_checks(pc):
    """The message bus: ccAISystem's methods and the ccAISysMsg* functions."""
    ai = pc.ai

    def case(rnd, fn, mangled, argf, this_sys=True, ptr=False):
        while True:
            w = pc.world(rnd)
            if w["ais"]:
                break
        args, gargs = argf(rnd, w)
        pre = (AISYS,) if this_sys else ()
        ret = ai.run(w, mangled, pre + tuple(a & 0xFFFFFFFF for a in gargs))
        if fn.startswith("SearchHistory"):
            ret = (ret - AISYS - 0x18C) // 0x18 if ret else -1
        elif fn == "ccAISysMsgWithdrawal":
            ret = 0
        else:
            ret = s32(ret)
        out = ai.read_world(w, ret)
        pc.c.scene.restore()
        return request(fn, [0] + args, w, pc.b), out, fn

    def some_name(rnd, w):
        pool = [m[0] for m in w["sys"]["hist"] + w["sys"]["buff"]] + [0x10008, 0x60000, -1]
        return rnd.choice(pool)

    def ids(w):
        return [a[AI_AT["entry"]][0] for a in w["ais"].values()] + [0xFFFF, 7]

    def search(rnd, w):
        n = some_name(rnd, w)
        t = rnd.choice((-1, 5, 90, 180, 0, 300))
        return [n, t], [n, t]

    def search_p(rnd, w):
        m = rnd.choice(w["sys"]["hist"])
        n = m[0] if rnd.random() < 0.7 else some_name(rnd, w)
        p = m[1] if rnd.random() < 0.7 else rnd.randrange(-1, 200)
        t = rnd.choice((-1, 5, 90, 180, 0, 300))
        ptr = m[2] if rnd.random() < 0.6 else rnd.choice([-1] + list(range(len(w["chars"]))))
        return [n, p, t, ptr], [n, p, t, ai.va(ptr) if ptr >= 0 else 0]

    def estimate(rnd, w):
        c = rnd.choice(range(len(w["chars"])))
        t = rnd.choice((-1, 5, 0, 30, 400))
        return [c, t], [ai.va(c), t]

    def delete(rnd, w):
        m = rnd.choice(w["sys"]["buff"])
        n = m[0] if rnd.random() < 0.8 else some_name(rnd, w)
        s_ = m[4] if rnd.random() < 0.5 else rnd.choice(ids(w))
        r = m[5] if rnd.random() < 0.5 else rnd.choice(ids(w))
        return [n, s_, r], [n, s_, r]

    def send(rnd, w):
        n = some_name(rnd, w)
        s_ = rnd.choice(ids(w))
        r = rnd.choice(ids(w))
        pr = rnd.choice((0, 0, 1))
        d = rnd.choice((0, 0, 30, 5))
        p = rnd.randrange(-1, 100)
        ptr = rnd.choice([-1] + list(range(len(w["chars"]))))
        return [n, s_, r, pr, d, p, ptr], [n, 0, s_, r, pr, d, p, ai.va(ptr) if ptr >= 0 else 0]

    def send_msg(rnd, w):
        i = rnd.randrange(16)
        if rnd.random() < 0.5:
            w["sys"]["buff"][i][7] = w["sys"]["time"]
        return [i], [AISYS + 0xC + 0x18 * i]

    def entry(rnd, w):
        k = rnd.choice(list(w["ais"]))
        return [k], [AIS + 0x400 * k, AIS + 0x400 * k + 0x160]

    def withdraw(rnd, w):
        i = rnd.choice(ids(w))
        return [i], [i]

    return [
        ("SearchHistoryMessage", lambda rnd: case(rnd, "SearchHistoryMessage",
                                                  "SearchHistoryMessage__10ccAISystemFUii", search)),
        ("SearchHistoryMessageP", lambda rnd: case(rnd, "SearchHistoryMessageP",
                                                   "SearchHistoryMessageP__10ccAISystemFUiiiPv", search_p)),
        ("CalcEstimateDamage", lambda rnd: case(rnd, "CalcEstimateDamage",
                                                "CalcEstimateDamage__10ccAISystemFP6ccChari", estimate)),
        ("DeleteDelayMessage", lambda rnd: case(rnd, "DeleteDelayMessage",
                                                "DeleteDelayMessage__10ccAISystemFUiUsUs", delete)),
        ("SendMessage", lambda rnd: case(rnd, "SendMessage", "SendMessage__10ccAISystemFP10ccAISysMsg", send_msg)),
        ("ccAISysMsgSendP", lambda rnd: case(rnd, "ccAISysMsgSendP", "ccAISysMsgSendP__FisUsUsUsUsiPv", send,
                                             this_sys=False)),
        ("AddEntry", lambda rnd: case(rnd, "AddEntry", "AddEntry__10ccAISystemFP4ccAIP9ccAIEntry", entry)),
        ("ccAISysMsgWithdrawal", lambda rnd: case(rnd, "ccAISysMsgWithdrawal", "ccAISysMsgWithdrawal__Fi", withdraw,
                                                  this_sys=False)),
    ]


# ccAI fields by their place in the AI list
AI_AT = {n: i for i, (n, _, _) in enumerate(AI_LAYOUT)}
COMMANDS = (0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 99, -1, 6, 21)


def set_cmd_flag(ai, v):
    ai[0] = (ai[0] & ~0x7000) | ((v & 7) << 12)


def calm(rnd, w, who, p=0.6):
    """Leave some of the characters `who` able and idle: no conditions, no
    skill, action 0."""
    for i in who:
        if i < 0 or rnd.random() > p:
            continue
        ch = w["chars"][i]
        dead = ch.cond[0] if rnd.random() < 0.2 else 0
        ch.cond = [0] * 16
        ch.cond[0] = dead
        ch.skill_id, ch.skill_status = rnd.choice((0, 0, 1)), 0
        ch.spc["act_num"] = rnd.choice((0, 0, 3, 6))


def open_up(rnd, w, bits):
    """Give the AIs some of the skill-mask bits."""
    for a in w["ais"].values():
        if rnd.random() < 0.8:
            a[0] |= bits << 24


def give_item(rnd, w, me, code):
    body = w["chars"][me]
    skills, items = w["lists"][body.id]
    items[rnd.randrange(40)] = (code & 0xFFFF, code >> 16, rnd.choice((1, 1, 2)))


def command_checks(pc):
    """The member commands: RequestChatCmd, ChatCommand and the rest."""
    ai = pc.ai
    pools = pc.pools

    def some_item(rnd, w, me):
        body = w["chars"][me]
        have = [it for it in w["lists"][body.id][1] if it[1] in range(10, 16) and it[0] != -1]
        if have and rnd.random() < 0.7:
            i, c, _ = rnd.choice(have)
        else:
            i, c, _ = rnd.choice(pools.items)
        return (c << 16) | (i & 0xFFFF)

    def steer(rnd, w, me, flag_vals, cmd_new=True):
        a = w["ais"][me]
        set_cmd_flag(a, rnd.choice(flag_vals))
        c = rnd.choice(COMMANDS)
        a[AI_AT["chatCmd"]] = c
        a[AI_AT["chatCmdNew"]] = c if rnd.random() < 0.3 else rnd.choice(COMMANDS)
        body = w["chars"][me]
        sk = [x for x in w["lists"][body.id][0] if x >= 0]
        a[AI_AT["chatCmdSkill"]] = rnd.choice(sk) if sk and rnd.random() < 0.6 else pools.skill(rnd) & 0x1FF
        a[AI_AT["chatCmdItem"]] = some_item(rnd, w, me)
        a[AI_AT["targetCCmd"]] = rnd.choice([-1] + list(range(len(w["chars"]))))
        if rnd.random() < 0.3:
            a[AI_AT["target"]] = a[AI_AT["targetCCmd"]]
        a[AI_AT["mode"]] = rnd.choice((0, 1, 2, 3, 4, 5, 6, 4, 3))
        a[AI_AT["chatCmdTime"]] = rnd.choice((0, 899, 900, rnd.randrange(0, 2000)))
        if rnd.random() < 0.3:
            give_item(rnd, w, me, 0xD0001)
        if rnd.random() < 0.5:
            w["chars"][me].party_flag = 1
        calm(rnd, w, [me])
        if rnd.random() < 0.5:
            a[0] &= ~0x20
        if rnd.random() < 0.3:
            a[AI_AT["chatCmd"]] = 19

    def request(rnd, w, me):
        c = rnd.choice(COMMANDS + (rnd.randrange(-3, 25),))
        t = chars_arg(w, rnd, True)
        sid = pools.skill(rnd) & 0x1FF
        return [c, t, sid], [c & 0xFFFFFFFF, cva(ai, t), sid], None

    def no_args(rnd, w, me):
        return [], [], None

    def plz(bit):
        def tw(rnd, w, me):
            a = w["ais"][me]
            calm(rnd, w, [me], 0.7)
            body = w["chars"][me]
            pool = pools.buffs + [175, 176, 177] if bit == 16 else pools.debuffs
            skills = w["lists"][body.id][0]
            for _ in range(4):
                skills[rnd.randrange(20)] = rnd.choice(pool)
            if rnd.random() < 0.8:
                a[0] |= bit << 24
            a[AI_AT["count"]] = rnd.choice((0, 55, 90, 110, 180, 495, rnd.randrange(0, 1000)))
            a[AI_AT["chatCmd"]] = rnd.choice((17, 18, -1, 5))
            w["chars"][me].skill_id = rnd.choice((0, 1, 2))
        return tw

    def cs(fn, mangled, argf, tweak, void=False):
        return lambda rnd: pc.case(rnd, fn, mangled, argf, tweak=tweak, void=void)

    return [
        ("RequestChatCmd", cs("RequestChatCmd", "RequestChatCmd__4ccAIFiP6ccChari", request, None, void=True)),
        ("ChatCommandFulfilCheck", cs("ChatCommandFulfilCheck", "ChatCommandFulfilCheck__4ccAIFv", no_args,
                                      lambda r, w, m: steer(r, w, m, (6,)))),
        ("ChatCommand", cs("ChatCommand", "ChatCommand__4ccAIFv", no_args,
                           lambda r, w, m: steer(r, w, m, (6, 6, 2, 1, 0, 7)), void=True)),
        ("ChatCommandExecute", cs("ChatCommandExecute", "ChatCommandExecute__4ccAIFv", no_args,
                                  lambda r, w, m: steer(r, w, m, (1, 2, 1, 2, 0)))),
        ("ChatCommandDeBuffPlz", cs("ChatCommandDeBuffPlz", "ChatCommandDeBuffPlz__4ccAIFv", no_args, plz(8))),
        ("ChatCommandBuffPlz", cs("ChatCommandBuffPlz", "ChatCommandBuffPlz__4ccAIFv", no_args, plz(16))),
    ]


def all_checks(pc):
    """Every check of this file as (name, fn(rnd)) pairs, in order."""
    return sys_checks(pc) + make_checks(pc) + fellow_checks(pc) + command_checks(pc) + frame_checks(pc)


def frame_checks(pc):
    """The per-frame flow: Brains and what it calls."""
    def no_args(rnd, w, me):
        return [], [], None

    def steer(rnd, w, me):
        calm(rnd, w, [me], 0.5)
        a = w["ais"][me]
        if rnd.random() < 0.8:
            a[0] &= ~0x4                       # not talking, mostly
        a[AI_AT["mode"]] = rnd.choice((0, 1, 2, 3, 4, 5, 6, 1, 3))
    pools = pc.pools

    def queue(rnd, w, me):
        """The member's queue: a few messages of every kind."""
        calm(rnd, w, [me], 0.5)
        a = w["ais"][me]
        e = a[AI_AT["entry"]]
        ids = [x[AI_AT["entry"]][0] for x in w["ais"].values() if x[AI_AT["entry"]][0] >= 0]
        myid = e[0]
        keys = list(range(len(w["chars"])))
        body = w["chars"][me]
        items = [(c << 16) | (i & 0xFFFF) for i, c, n in w["lists"][body.id][1] if c in range(10, 16) and i >= 0]
        num = rnd.randrange(0, 7)
        top = rnd.randrange(0, 10)
        e[2], e[3], e[4] = num, top, (top + max(num, 1) - 1) % 10
        for j in range(num):
            k = rnd.choice((0, 1, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12))
            if k == 0:
                low = rnd.choice((6, 7, 23, 24, 3))
            elif k == 1:
                low = rnd.randrange(0, 25)
            else:
                low = rnd.choice(ids + [myid, 0x7F]) & 0xFFFF
            param = (rnd.choice(pools.buffs + pools.debuffs + [150, 1]) if k == 6 else
                     rnd.choice(items + [rnd.choice(pools.items)[1] << 16]) if k in (7, 8, 9, 10, 11) and items else
                     rnd.randrange(-1, 300))
            sender = rnd.choice(ids + [myid, myid, 0xFFFF]) & 0xFFFF
            e[5][(top + j) % 10] = [s32((k << 16) | low), param, rnd.choice([-1] + keys), 1, sender,
                                    rnd.choice(ids + [0xFFFF]) & 0xFFFF, rnd.choice((0, 1)), w["sys"]["time"]]
        if rnd.random() < 0.8:
            w["globals"][3] = 0

    def act(rnd, w, me):
        calm(rnd, w, [me], 0.6)
        a = w["ais"][me]
        a[AI_AT["mode"]] = rnd.choice((0, 1, 2, 3, 4, 5, 6, 1, 2, 3, 3, 4))
        if rnd.random() < 0.7:
            set_cmd_flag(a, 0)
        if rnd.random() < 0.5:
            a[0] &= ~0x20                                       # not heading back
        a[AI_AT["strategy"]] = rnd.choice((0, 1, 2, 3, 4, 0, 1, 2))
        if rnd.random() < 0.5:
            w["chars"][me].party_flag = 1
        if rnd.random() < 0.5:
            w["globals"][3] = 0
        foes = [i for i, c in enumerate(w["chars"]) if not c.type & 7]
        if foes and rnd.random() < 0.6:
            a[AI_AT["target"]] = rnd.choice(foes)
            a[0] |= 1 << 10                                     # targetFlag 1
        k = w["party"]["members"][0]
        if foes and rnd.random() < 0.5:
            w["chars"][k].spc["target_char"] = rnd.choice(foes)

    def eye(rnd, w, me):
        t = chars_arg(w, rnd)
        return [t], [cva(pc.ai, t)], None

    return [
        ("ReadSysMsg", lambda rnd: pc.case(rnd, "ReadSysMsg", "ReadSysMsg__4ccAIFv", no_args, tweak=queue)),
        ("CheckEyeLineToTarget", lambda rnd: pc.case(rnd, "CheckEyeLineToTarget",
                                                     "CheckEyeLineToTarget__4ccAIFP6ccChar", eye, ptr=False,
                                                     fret=True)),
        ("Reconnoiter", lambda rnd: pc.case(rnd, "Reconnoiter", "Reconnoiter__4ccAIFv", no_args, tweak=act)),
        ("ActInField", lambda rnd: pc.case(rnd, "ActInField", "ActInField__4ccAIFv", no_args, tweak=act)),
        ("ActInDungeon", lambda rnd: pc.case(rnd, "ActInDungeon", "ActInDungeon__4ccAIFv", no_args, tweak=act)),
        ("Brains", lambda rnd: pc.case(rnd, "Brains", "Brains__4ccAIFv", no_args, tweak=steer)),
    ]


class Party(T.Checks):
    """The checks as test_battle_rs's runner takes them: PARTY_AI maps a name
    to fn(checks, rnd) -> (request, game answer, label); `checks` is this
    object (T.Checks plus the AI's world)."""

    def __init__(self):
        super().__init__()
        self.pc = PartyChecks(self)
        self.fn = dict(all_checks(self.pc))


def _check(name):
    return lambda checks, rnd: checks.fn[name](rnd)


CHECK_NAMES = [
    "SearchHistoryMessage", "SearchHistoryMessageP", "CalcEstimateDamage", "DeleteDelayMessage", "SendMessage",
    "ccAISysMsgSendP", "AddEntry", "ccAISysMsgWithdrawal",
    "CheckSkillList", "CheckSkillList2", "CheckBuffSkillListNum", "CheckDebuffSkillListNum", "CheckHealHpSkill",
    "CheckHealParty", "SearchHealSkill", "CheckItemList", "CheckItemList2", "ConsumeItemList",
    "SearchItemListBySkill", "SearchItemListByHeal", "SearchItemListByBuff", "SearchItemListByDebuff",
    "SearchItemListByMagicAttack", "CheckNeedHealing", "CheckConditionMinus", "CheckBossEntry",
    "CheckHealingSchedule", "CheckSchedule", "CheckSolution", "SysMsgFromMeToMe", "CheckSkillOtherSPC",
    "CheckHealSkillOtherSPC", "SearchTarget", "SearchTargetNear", "CountTargetInArea", "levelCheck",
    "SearchDebuffUnusedEnemy", "SearchBuffUnusedFellow", "DebuffForUnusedEnemy", "BuffForUnusedFellow",
    "CheckTargetConditionBySkill", "ChangeMode", "ChangeStrategy", "ChangeStrategyCMD", "StopNormalAttack",
    "UseSkill", "UseItem", "SelectAttackSkill", "CureSPC", "CureOtherSPC", "CureOnlyCharmConfusionOtherSPC",
    "CureOnlyParalysisSleepOtherSPC", "CurePoisonSPC", "HealSPC", "ResurrectSPC", "HealPlzNormalMode",
    "HealPlzBattleMode", "ChatCommandHealPlz", "AttackTarget", "FellowUseSkill", "FellowAttack",
    "RequestChatCmd", "ChatCommandFulfilCheck", "ChatCommand", "ChatCommandExecute", "ChatCommandDeBuffPlz",
    "ChatCommandBuffPlz", "ReadSysMsg", "CheckEyeLineToTarget", "Reconnoiter", "ActInField", "ActInDungeon",
    "Brains",
]
LATER_NAMES = [] if volume.NAME == "infection" else ["ItemFirst", "CurePoisonCurseSPC"]
CHECK_NAMES += LATER_NAMES
PARTY_AI = {name: _check(name) for name in CHECK_NAMES}


def _test(names, seed):
    def test(self):
        for k, name in enumerate(names):
            self.check(name, seed + k)
    return test


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class PartyAiAgainstGame(T.Against):
    CASES = 100
    TABLE = PARTY_AI
    MAKE = Party

    test_bus = _test(CHECK_NAMES[:8], 9200)
    test_lists = _test(CHECK_NAMES[8:23], 9210)
    test_duties = _test(CHECK_NAMES[23:47], 9230)
    test_heal_and_attack = _test(CHECK_NAMES[47:61], 9260)
    test_chat = _test(CHECK_NAMES[61:68], 9280)
    test_frame = _test(CHECK_NAMES[68:len(CHECK_NAMES) - len(LATER_NAMES)], 9290)
    test_later = _test(LATER_NAMES, 9300)


if __name__ == "__main__":
    T.main(CHECK_NAMES, 9200, PARTY_AI, Party)
