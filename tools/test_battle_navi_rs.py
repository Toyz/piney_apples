#!/usr/bin/env python3
"""piney_battle::navi and piney_battle::ai_move against the game's ccNavi
(ccnavi.cpp) and the party AI's movers (personal.cpp) run in tools/eemu.py.

Each check builds a random world - a party member with its ccAI (and the
ccNavi inside it), Kite and maybe others on the command lists, the path
finding's globals (buf, buf2, bufFlag) around a dungeon window, the
dungeon's 2D map, Mac Anu's landmark table with its dummies placed, the
message bus - and runs a list of operations natively in the interpreter
and through the battle_probe example (`navi` requests), comparing after
every operation its return value, every field of the AIs (their ccNavi and
beacon table included), the bodies (the +0xe0 flag word, heading, actNum,
actNumOld, targetChar, transparency, cloak, position), the bus, the
rand() state, the calls made in order, and in the path checks buf and buf2
(a region cell by cell and both whole arrays by CRC-32).

What is not the navigation's is stubbed and recorded: Get2DMapInfo and
Get2DMapPtr answer from the case, RouteSearchByMap (the town's route,
piney-world's) and the dummies ccStream::GetChunkAdrsF finds answer from
a script both sides read in order; FollowPlayer, LeavePlayer, TransferIn,
TransferOut, resignParty, disbandSpc, HitEnable, ChatMessage and
ChatMessageReencounter are recorded. ccMalloc is a bump allocator.

The composed runs (ActInTownFull, ManualControlFull, BrainsDungeon) check
the movement as crates/piney-battle/src/party_motion.rs assembles it: on the
game's side ccAI::ActInTown, ManualControl or the whole ccAI::Brains run
natively with everything they call (FollowPlayer, LeavePlayer,
FollowTargetTown, the TownNavigators, HitEnable, ActInDungeon's path
finding, FollowBeacon, CheckGoalBeaconPos), RouteSearchByMap included over
Mac Anu's own tables (the landmarks moved to town01's dummies by
ccSetNaviMap first, its ccHitCheckLM2 scripted); on the port's
party_motion::Movement performs the calls, over a world whose route search
answers what the game's own search found and whose line checks answer the
game's script. They compare aiOpenDirc and each body's hitSW too.

The dungeon checks run on windows of the tutorial dungeon's floors
(field 14, "Bursting Passed Over Aqua Field", its edited dungeon) and of
random dungeons, and on random windows; the multi-frame checks run
ActInTown and FollowBeacon for 60-300 frames with the bodies moved
between frames and every field compared after each.

    python3 tools/test_battle_navi_rs.py            the unit tests
    python3 tools/test_battle_navi_rs.py bulk N     N cases of every check
"""

import math
import os
import random
import struct
import sys
import unittest
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import eemu                                    # noqa: E402
import test_anim                               # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle as TB                       # noqa: E402
import test_battle_party_ai_rs as PA           # noqa: E402
import test_battle_rs as T                     # noqa: E402
import volume                                   # noqa: E402
from test_battle_rs import fbits               # noqa: E402

AIS = PA.AIS                # the ccAI of scene character i at AIS + 0x400 * i
AISYS = PA.AISYS
WM = 0x01072800             # WORLD_MAN's stand-in: +8 the world type (2 a dungeon)
WM2 = 0x01073000            # what WORLD_MAN +0x430 points at
CHUNKS = 0x01074000         # the dummies GetChunkAdrsF finds, 0x20 each
OUTV = 0x01075000           # vectors passed to the functions
PATHS = 0x01075800          # a ccPath
HEAP = 0x01400000           # ccMalloc's bump region
ENTS = 0x01076000           # g_entCtrl's stand-in: no enemies spawned
EVENT_MNG = 0x01076100      # eventMng's stand-in: +0x78c the ocarina lock
MAP2D = 0x01500000          # the dungeon's 2D map
TOWN = 0x01520000           # the landmark table naviMapPtr points at (256 entries)
F_ONE = 0x3F800000
F_M1 = 0xBF800000
NAVI = 0xF0                 # ccAI.navi


def fl(x):
    return fbits(x)


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def hexs(b):
    return bytes(b).hex() or "-"


def nested_call(m, addr, args):
    """m.call from inside a hook (tools/test_battle_fellow_rs.py's): the
    caller's frames and registers are kept aside and put back."""
    sp = m.r[29] & 0xFFFFFFFF
    frames = bytes(m.mem[sp:eemu.STACK_TOP])
    regs = [m.r[i] for i in range(32)]
    hi, lo = m.hi, m.lo
    r = m.call(addr, tuple(a & 0xFFFFFFFF for a in args), 400_000_000)
    m.mem[sp:eemu.STACK_TOP] = frames
    for i in range(32):
        m.r[i] = regs[i]
    m.hi, m.lo = hi, lo
    return r


class NaviGame:
    """The game side: test_battle_rs's GameBattle and GameScene plus the
    navigation's memory and stubs."""

    def __init__(self, checks):
        self.c = checks
        g = checks.game
        self.g = g
        self.m = g.m
        m = self.m
        self.sym = g.sym
        sym = self.sym
        self.va = checks.scene.va
        self.buf = sym("buf")
        self.buf2 = sym("buf2")
        self.buf_flag = sym("bufFlag")
        self.p_navimap = sym("naviMapPtr")
        self.p_marknum = sym("landMarkNum")
        self.p_party = sym("ccPartyManager")
        self.p_registry = sym("ccSpcManager") + 0xDC
        self.names_tbl = sym("naviPointNameTable")
        m.store(sym("worldman"), 4, WM)
        m.store(sym("ccAISys"), 4, AISYS)
        m.mem[WM:WM + 0x800] = bytes(0x800)
        m.store(WM + 8, 4, 2)
        m.store(WM + 0x430, 4, WM2)
        # the town names: naviPointNameTable[1..7] (0 is null)
        self.point_names = {}
        for i in range(1, 8):
            p = m.load(self.names_tbl + 4 * i, 4)
            self.point_names.setdefault(self.cstr(p), i)
        self.calls = []

        def rec(*a):
            self.calls.append(list(a))
            return 0

        def body(ai):
            return self.name(m.load(ai + 0x18, 4))

        def vec(a):
            return [m.load(a + 4 * i, 4) for i in range(4)]

        def info(mm, this, p, *_):
            rec("Get2DMapInfo")
            if self.info_script:
                self.info = self.info_script.pop(0)
            for i, v in enumerate(self.info):
                mm.store(p + 4 * i, 4, v & 0xFFFFFFFF)
            return 0

        def mapptr(mm, *_):
            rec("Get2DMapPtr")
            return MAP2D

        def malloc(mm, n, *_):
            a = self.heap
            self.heap += (max(n, 1) + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            self.sizes[a] = n
            return a

        def route(mm, navi, start, goal, *_):
            rec("RouteSearchByMap", vec(start), vec(goal))
            r = self.routes.pop(0) if self.routes else (0, 0, 0, 0, 0, bytes(48))
            ret, name, step, dist, dirc, rt = r
            mm.mem[navi + 0x2E:navi + 0x5E] = bytes(rt)
            self.g.put(navi + 0x18, "<hhhh", name, 0, step, 1)
            mm.store(navi + 0x20, 4, dist)
            mm.store(navi + 0x24, 4, dirc)
            mm.mem[navi:navi + 16] = mm.mem[goal:goal + 16]
            return ret & 0xFFFFFFFF

        def chunk(mm, stream, name, *_):
            s = self.cstr(name)
            if s.startswith(b"DMY_marker"):
                k = int(s[10:])
                rec("Marker", k)
                pos = self.markers.get(k, [0, 0, 0, 0])
            else:
                k = self.point_names.get(s, -1)
                rec("NaviPoint", k)
                pos = self.points.get(k, [0, 0, 0, 0])
            # a ring of 64 blocks: the callers copy the position at once
            a = self.chunk_at
            self.chunk_at = CHUNKS + (self.chunk_at + 0x20 - CHUNKS) % 0x800
            mm.mem[a:a + 0x20] = bytes(0x20)
            for i, v in enumerate(pos):
                mm.store(a + 16 + 4 * i, 4, v)
            return a

        def chat(mm, ai, msg, *_):
            # the line by its bytes (the same text sits at other addresses
            # on another volume)
            text = bytes(mm.mem[msg:msg + 256]).split(b"\0")[0].hex() if msg else None
            return rec("ChatMessage", body(ai), text or "-")

        def line(mm, a, b, mask, *_):
            rec("ccHitCheckLM", vec(a), vec(b), mask)
            r = self.lines.pop(0) if self.lines else F_M1
            mm.f[0] = r
            return r

        def line2(mm, a, b, mask, *_):
            # RouteSearchByMap's check between two points at one landmark:
            # inside the world's route search, not a call of the AI's.
            r = self.route_lines.pop(0) if self.route_lines else F_M1
            mm.f[0] = r
            return r

        def native(name, before=None, after=None):
            """A hook that records the call, then runs the game's own
            function (the hook set aside meanwhile)."""
            addr = sym(name)

            def h(mm, a0, a1, a2, a3):
                if before:
                    before(a0, a1, a2, a3)
                hook = mm.hooks.pop(addr)
                try:
                    r = nested_call(mm, addr, (a0, a1, a2, a3))
                finally:
                    mm.hooks[addr] = hook
                if after:
                    after(r, a0, a1, a2, a3)
                return r
            return h

        def found(r, navi, *_):
            g = self.g
            self.found.append([1, s32(r)] + list(g.get(navi, "<4I")) + list(g.get(navi + 0x10, "<hhhhhhhh"))
                              + list(g.get(navi + 0x20, "<II")) + list(g.get(navi + 0x28, "<hhh"))
                              + [hexs(mm_bytes(navi + 0x2E, 48))] + list(mm_bytes(navi + 0x5E, 2))
                              + [g.get(navi + 0x60, "<i")[0]])

        def mm_bytes(a, n):
            return bytes(m.mem[a:a + n])

        self.route_scripted = route
        self.route_found = native("RouteSearchByMap__6ccNaviFPfPf",
                                  before=lambda navi, st, go, _: rec("RouteSearchByMap", vec(st), vec(go)),
                                  after=found)
        self.hit_native = native("HitEnable__9ccCharHitFv",
                                 before=lambda a, *_: rec("HitEnable", self.name(a - 0x1A0)))
        self.hit_recorded = lambda mm, a, *_: rec("HitEnable", self.name(a - 0x1A0))
        self.follow_hooks = {
            "FollowPlayer__4ccAIFv": lambda mm, a, *_: rec("FollowPlayer", body(a)),
            "LeavePlayer__4ccAIFv": lambda mm, a, *_: rec("LeavePlayer", body(a)),
        }

        hooks = {
            "Get2DMapInfo__9WORLD_MANFPi": info,
            "Get2DMapPtr__9WORLD_MANFv": mapptr,
            "ccMalloc__FUi": malloc,
            "ccFree__FPv": lambda mm, *_: 0,
            "RouteSearchByMap__6ccNaviFPfPf": route,
            "GetChunkAdrsF__8ccStreamFPCci": chunk,
            "FollowPlayer__4ccAIFv": lambda mm, a, *_: rec("FollowPlayer", body(a)),
            "LeavePlayer__4ccAIFv": lambda mm, a, *_: rec("LeavePlayer", body(a)),
            "TransferOut__9ccSpcCharFv": lambda mm, a, *_: rec("TransferOut", self.name(a)),
            "TransferIn__9ccSpcCharFv": lambda mm, a, *_: rec("TransferIn", self.name(a)),
            "resignParty__Fi": lambda mm, a, *_: rec("resignParty", s32(a)),
            "disbandSpc__Fi": lambda mm, a, *_: rec("disbandSpc", s32(a)),
            "HitEnable__9ccCharHitFv": lambda mm, a, *_: rec("HitEnable", self.name(a - 0x1A0)),
            "ChatMessage__4ccAIFPcPcPcPc": chat,
            "ChatMessageReencounter__4ccAIFv": lambda mm, a, *_: rec("ChatMessageReencounter", body(a)),
            "ccHitCheckLM__FPfPfUi": line,
            "ccHitCheckLM2__FPfPfUi": line2,
            # what the decisions call outside the AI, for Brains
            "ccSkillRequest__FP6ccCharP6ccChari": lambda mm, a, b, c, *_: rec("ccSkillRequest", self.name(a),
                                                                               self.name(b), s32(c)),
            "ccUseItemRequest__FP6ccCharP6ccCharii": lambda mm, a, b, c, d: rec("ccUseItemRequest", self.name(a),
                                                                                 self.name(b), s32(c), s32(d)),
            "Attack__8ccPlayerFP6ccChari": lambda mm, a, b, c, *_: rec("PlayerAttack", self.name(a),
                                                                        self.name(b), s32(c)),
            "ManualModeAI__9ccSpcCharFi": lambda mm, a, b, *_: rec("ManualModeAI", self.name(a), s32(b)),
            "ChatMessageSender__4ccAIFv": lambda mm, a, *_: rec("ChatMessageSender", body(a)),
            "GetFieldType__9WORLD_MANFv": lambda mm, *_: self.field_type & 0xFFFFFFFF,
            "GetFieldAttrb__9WORLD_MANFv": lambda mm, *_: self.field_attr & 0xFFFFFFFF,
        }
        for n, f in hooks.items():
            m.hooks[sym(n)] = f
        # every other chat line of the AI's, as the party AI's harness
        # records them: its name, the AI's member, its arguments
        for sy in g.prog.functions():
            n = sy.name
            if not n.startswith("ChatMessage") or not n.endswith(("Fv", "Fi", "FP6ccChar", "FP6ccChari",
                                                                  "FP6ccCharii", "FPc")):
                continue
            if n.startswith(("ChatMessage__", "ChatMessageModify__", "ChatMessageSender__",
                             "ChatMessageReencounter__")):
                continue
            short = n.split("__")[0]
            sig = n.split("__4ccAIF")[1]

            def make(short, sig):
                def h(mm, ai, a, b, c, *_):
                    args = {"v": [], "i": [s32(a)], "P6ccChar": [self.name(a)], "P6ccChari": [self.name(a), s32(b)],
                            "P6ccCharii": [self.name(a), s32(b), s32(c)], "Pc": ["s"]}[sig]
                    return rec(short, body(ai), *args)
                return h
            m.hooks[sy.value] = make(short, sig)
        # the party manager is laid out for real: the following's
        # checkPartyMenberNum runs
        m.hooks.pop(sym("checkPartyMenberNum__Fi"), None)
        m.store(sym("g_entCtrl"), 4, ENTS)
        m.mem[ENTS:ENTS + 0x40] = bytes(0x40)
        m.store(sym("eventMng"), 4, EVENT_MNG)
        self.full = False
        self.lines = []
        self.route_lines = []
        self.found = []
        self.field_type = self.field_attr = 0
        self.p_open = sym("aiOpenDirc")
        self.p_sbc = sym("spcBattleCondition")
        self.p_pstrat = sym("partyStrategy")
        self.p_player = sym("plw") + 0x20
        self.p_pgride = sym("pgRideFlag")
        self.info = [0, 0, 0]
        self.info_script = []
        self.routes = []
        self.points = {}
        self.markers = {}
        self.heap = HEAP
        self.sizes = {}
        self.chunk_at = CHUNKS

    def set_full(self, full):
        """The composed checks run the AI's own following, the town's route
        search and HitEnable natively; the others record them."""
        m, sym = self.m, self.sym
        self.full = full
        m.hooks[sym("RouteSearchByMap__6ccNaviFPfPf")] = self.route_found if full else self.route_scripted
        m.hooks[sym("HitEnable__9ccCharHitFv")] = self.hit_native if full else self.hit_recorded
        for n, f in self.follow_hooks.items():
            if full:
                m.hooks.pop(sym(n), None)
            else:
                m.hooks[sym(n)] = f

    def cstr(self, a):
        out = bytearray()
        while a and len(out) < 64:
            b = self.m.mem[a]
            if b == 0:
                break
            out.append(b)
            a += 1
        return bytes(out)

    def name(self, v):
        v &= 0xFFFFFFFF
        if v == 0:
            return 0
        if T.SCN <= v < T.SCN + 0x8000 and (v - T.SCN) % 0x1000 == 0:
            return f"c{(v - T.SCN) // 0x1000}"
        return v

    def index(self, v):
        if v == 0:
            return -1
        if T.SCN <= v < T.SCN + 0x8000 and (v - T.SCN) % 0x1000 == 0:
            return (v - T.SCN) // 0x1000
        return -1000 - v

    # memory -------------------------------------------------------------------------
    def alloc(self, n):
        a = self.heap
        self.heap += (max(n, 1) + 15) & ~15
        self.m.mem[a:a + n] = bytes(n)
        self.sizes[a] = n
        return a

    def put_navi(self, a, nv):
        g, m = self.g, self.m
        n = a + NAVI
        g.put(n, "<4I", *nv["goal_pos"])
        g.put(n + 0x10, "<hhhh", nv["start_x"], nv["start_y"], nv["goal_x"], nv["goal_y"])
        g.put(n + 0x18, "<h", nv["name"])
        g.put(n + 0x1C, "<hh", nv["step"], nv["landmark"])
        g.put(n + 0x20, "<II", nv["dist"], nv["dirc"])
        g.put(n + 0x28, "<hhh", nv["map_x"], nv["map_y"], nv["map_s"])
        m.mem[n + 0x2E:n + 0x5E] = bytes(nv["route"])
        m.mem[n + 0x5E:n + 0x60] = bytes(nv["pad"])
        g.put(n + 0x60, "<i", nv["beacon_num"])
        if nv["beacon"] is None:
            m.store(n + 0x64, 4, 0)
        else:
            p = self.alloc(max(4 * len(nv["beacon"]), 0x200))
            self.sizes[p] = 4 * len(nv["beacon"])
            for i, (x, y) in enumerate(nv["beacon"]):
                m.mem[p + 2 * i] = x
                m.mem[p + 2 * i + 1] = y
            m.store(n + 0x64, 4, p)
        m.mem[n + 0x68:n + 0x70] = bytes(8)

    def read_navi(self, a):
        g, m = self.g, self.m
        n = a + NAVI
        out = list(g.get(n, "<4I"))
        # startX startY goalX goalY name, then (past finishFlag) step landmark
        out += list(g.get(n + 0x10, "<hhhhh")) + list(g.get(n + 0x1C, "<hh"))
        out += list(g.get(n + 0x20, "<II")) + list(g.get(n + 0x28, "<hhh"))
        out.append(hexs(m.mem[n + 0x2E:n + 0x5E]))
        out += [m.mem[n + 0x5E], m.mem[n + 0x5F], g.get(n + 0x60, "<i")[0]]
        p = m.load(n + 0x64, 4)
        if p == 0:
            out.append("-")
        else:
            k = self.sizes.get(p, 0) // 4
            out.append(hexs(m.mem[p:p + 2 * k]))
        return out

    def put_ai(self, key, ai, nv):
        a = AIS + 0x400 * key
        m, g = self.m, self.g
        m.mem[a:a + 0x260] = bytes(0x260)
        for (n, off, f), v in zip(PA.AI_LAYOUT, ai):
            if f == "P":
                m.store(a + off, 4, PA.SPC_AI_PARAM + 0x24 * v)
            elif f == "C":
                m.store(a + off, 4, self.va(v) if v >= 0 else 0)
            elif f == "E":
                e = a + off
                g.put(e, "<hhhhh", *v[:5])
                for i, msg in enumerate(v[5]):
                    self.put_msg(e + 0xC + 0x18 * i, msg)
            elif f == "<4I":
                g.put(a + off, f, *v)
            else:
                g.put(a + off, f, v)
        self.put_navi(a, nv)

    def read_ai(self, key):
        a = AIS + 0x400 * key
        m, g = self.m, self.g
        out = []
        for n, off, f in PA.AI_LAYOUT:
            if f == "P":
                out.append((m.load(a + off, 4) - PA.SPC_AI_PARAM) // 0x24)
            elif f == "C":
                out.append(self.index(m.load(a + off, 4)))
            elif f == "E":
                e = a + off
                out += list(g.get(e, "<hhhhh"))
                out += [self.read_msg(e + 0xC + 0x18 * i) for i in range(10)]
            elif f == "<4I":
                out += list(g.get(a + off, f))
            else:
                out.append(g.get(a + off, f)[0])
        return out + self.read_navi(a)

    def put_msg(self, a, msg):
        name, param, ptr, state, sender, receiver, prio, dt = msg
        self.g.put(a, "<iiIhHHHI", name, param, self.va(ptr) if ptr >= 0 else 0, state, sender, receiver, prio, dt)

    def read_msg(self, a):
        name, param, ptr, state, sender, receiver, prio, dt = self.g.get(a, "<iiIhHHHI")
        return [name, param, self.index(ptr), state, sender, receiver, prio, dt]

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

    def flags_word(self, ch):
        return ((ch.spc_flags & ~(0x80 | 7 << 14)) | (0x80 if ch.no_death else 0)
                | (ch.party_flag & 7) << 14) & 0xFFFFFFFF

    def put_world(self, w):
        c, g, m = self.c, self.g, self.m
        c.reset()
        self.heap = HEAP
        self.sizes = {}
        self.chunk_at = CHUNKS
        chars = w["chars"]
        c.scene.put(chars, w["pcs"], w["enes"])
        for i, ch in enumerate(chars):
            va = self.va(i)
            base = m.load(va, 4)
            m.store(base + 0x18, 4, ch.height)
            g.put(va + 0x60, "<4I", *ch.dirc)
            g.put(va + 0xEE, "<hh", ch.act_num, ch.act_num_old)
            m.store(va + 0x78, 4, self.va(ch.target_char) if ch.target_char >= 0 else 0)
            m.store(va + 0x88, 4, ch.transparency)
            m.store(va + 0x8C, 4, ch.set_transparency)
            m.store(va + 0x110, 4, ch.cloak)
            m.store(va + 0xE0, 4, self.flags_word(ch))
            m.store(va + 0x1B4, 4, ch.radius)
            m.store(va + 0x1A0, 4, getattr(ch, "hit_sw", 0))
            m.store(va + 0x400 + 0xD4, 4, ch.velocity)      # personality->velocity
            m.store(va + 0x128, 4, AIS + 0x400 * i if i in w["ais"] else 0)
            vt = PA.VT_PLAYER if ch.type & 1 else PA.VT_FELLOW if ch.type & 4 else PA.VT_SPC
            m.store(va + 0x1F4, 4, vt)
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
        g.put(self.p_sbc, "<h", gm["spc_battle_condition"])
        g.put(self.p_pstrat, "<i", gm["party_strategy"])
        m.store(self.p_player, 4, self.va(gm["player"]) if gm["player"] >= 0 else 0)
        g.put(self.p_pgride, "<i", gm["pg_ride_flag"])
        g.put(TB.GAME + 0x20, "<i", w.get("town_index", 0))
        m.mem[ENTS:ENTS + 0x40] = bytes(0x40)
        m.store(self.p_open, 4, w["open"])
        self.lines = list(w["lines"])
        self.route_lines = list(w.get("route_lines", []))
        self.found = []
        for sid, lst in w.get("lists", {}).items():
            g.put(TB.SAVE + 0x1EC4 + 40 * sid, "<20h", *lst[0])
            for j, (i_, cat, n_) in enumerate(lst[1]):
                g.put(TB.SAVE + 0x30 + 160 * sid + 4 * j, "<hbb", i_, cat, n_)
        self.set_full(w.get("full", False))
        m.store(TB.SAVE + 0x220D, 1, w["save220d"])
        g.put(self.p_registry, "<i", w["registry"])
        for k, (ai, nv) in w["ais"].items():
            self.put_ai(k, ai, nv)
        self.put_sys(w["sys"])
        g.set_rand(w["rand"])
        # the path finding's globals
        m.mem[self.buf:self.buf + 0x10000] = bytes(0x10000)
        m.mem[self.buf2:self.buf2 + 0x20000] = bytes(0x20000)
        g.put(self.buf_flag, "<i", w["ready"])
        x0, y0, wd, ht = w["region"]
        for x in range(wd):
            for y in range(ht):
                i = (x0 + x) * 256 + y0 + y
                k = x * ht + y
                m.mem[self.buf + i] = w["link"][k]
                m.mem[self.buf2 + 2 * i:self.buf2 + 2 * i + 2] = struct.pack("<h", w["cost"][k])
        m.mem[MAP2D:MAP2D + 0x10000] = bytes(0x10000)
        mx, my, mw, mh = w["map_region"]
        for x in range(mw):
            for y in range(mh):
                m.mem[MAP2D + (mx + x) * 256 + my + y] = w["map"][x * mh + y]
        self.info = list(w["info"])
        self.info_script = [list(v) for v in w.get("info_script", [])]
        # the town
        tn = w["town"]
        m.mem[TOWN:TOWN + 0x3000] = bytes(0x3000)
        for k, (pos, name) in enumerate(tn["marks"]):
            g.put(TOWN + 0x30 * k, "<4Ii", *pos, name)
        m.store(self.p_navimap, 4, TOWN)
        g.put(self.p_marknum, "<i", tn["num"])
        self.routes = [tuple(r[:5]) + (bytes(r[5]),) for r in w["routes"]]
        self.points = dict(w["points"])
        self.markers = dict(w["markers"])
        self.calls = []

    def read_chars(self, w):
        g, m = self.g, self.m
        out = {}
        for i in range(len(w["chars"])):
            va = self.va(i)
            out[str(i)] = [m.load(va + 0xE0, 4), list(g.get(va + 0x60, "<4I")), g.get(va + 0xEE, "<h")[0],
                           g.get(va + 0xF0, "<h")[0], self.index(m.load(va + 0x78, 4)), m.load(va + 0x88, 4),
                           m.load(va + 0x8C, 4), m.load(va + 0x110, 4), list(g.get(va + 0x40, "<4I")),
                           m.load(va + 0x1A0, 4)]
        return out

    def read_path(self, w):
        m = self.m
        x0, y0, wd, ht = w["region"]
        link, cost = bytearray(), bytearray()
        for x in range(wd):
            i = (x0 + x) * 256 + y0
            link += m.mem[self.buf + i:self.buf + i + ht]
            cost += m.mem[self.buf2 + 2 * i:self.buf2 + 2 * (i + ht)]
        return {"ready": self.g.get(self.buf_flag, "<i")[0], "link": hexs(link), "cost": hexs(cost),
                "crc": [zlib.crc32(bytes(m.mem[self.buf:self.buf + 0x10000])),
                        zlib.crc32(bytes(m.mem[self.buf2:self.buf2 + 0x20000]))]}

    def snapshot(self, w, path, town):
        out = {"rand": self.g.rand_now(), "calls": self.calls,
               "ais": {str(k): self.read_ai(k) for k in w["ais"]}, "sys": self.read_sys(),
               "chars": self.read_chars(w), "open": self.m.load(self.p_open, 4)}
        if path:
            out["path"] = self.read_path(w)
        if town:
            out["town"] = [list(self.g.get(self.m.load(self.p_navimap, 4) + 0x30 * k, "<4I"))
                           for k in range(len(w["town"]["marks"]))]
        return out

    # operations ----------------------------------------------------------------------------
    def vec_in(self, a, v):
        self.g.put(a, "<4I", *v)

    def run_op(self, w, op, args):
        """One operation on the game's memory; returns its result fields."""
        m, g, sym = self.m, self.g, self.sym
        key = args[0] if args else 0
        ai = AIS + 0x400 * key
        navi = ai + NAVI

        def call(fn, *a):
            return m.call(sym(fn), tuple(x & 0xFFFFFFFF for x in a), 400_000_000)

        def vec(i):
            return list(args[i:i + 4])

        V1, V2 = OUTV, OUTV + 0x10
        if op == "SetPathFindingMap":
            call("SetPathFindingMap__Fv")
            return {"ret": 0}
        if op == "PathFindingF":
            self.vec_in(V1, vec(1))
            self.vec_in(V2, vec(5))
            return {"ret": s32(call("PathFindingInDungeon__6ccNaviFPfPf", navi, V1, V2))}
        if op == "PathFinding":
            return {"ret": s32(call("PathFindingInDungeon__6ccNaviFiiii", navi, *args[1:5]))}
        if op == "Heuristic":
            return {"ret": s32(call("HeuristicType1__6ccNaviFPs", navi, self.buf2))}
        if op in ("ShortPath", "LinkInfo", "MinimumLinkDirc", "LinkNum"):
            fn = {"ShortPath": "ShortPath__6ccNaviFPsii", "LinkInfo": "LinkInfo__6ccNaviFPsii",
                  "MinimumLinkDirc": "MinimumLinkDirc__6ccNaviFPsii", "LinkNum": "LinkNum__6ccNaviFPsii"}[op]
            r = call(fn, navi, self.buf2, args[1], args[2])
            return {"ret": s16(r) if op == "MinimumLinkDirc" else s32(r)}
        if op == "MakeBeaconTbl":
            m.mem[PATHS:PATHS + 12] = bytes(12)
            r = s32(call("MakeBeaconTbl__6ccNaviFP6ccPathPsii", navi, PATHS, self.buf2, args[1], args[2]))
            if r == 0:
                return {"ret": 0, "tbl": "-"}
            num, _, p = g.get(PATHS, "<iiI")
            return {"ret": r, "tbl": hexs(m.mem[p:p + 4 * num])}
        if op == "SetBeacon":
            ent = args[1:]
            p = self.alloc(max(len(ent), 4))
            m.mem[p:p + len(ent)] = bytes(x & 0xFF for x in ent)
            g.put(PATHS, "<iiI", len(ent) // 4, len(ent) // 4, p)
            return {"ret": s32(call("SetBeacon__6ccNaviFP6ccPath", navi, PATHS))}
        if op in ("CheckBeaconPos", "CheckBeaconPos2D"):
            self.vec_in(V1, [0x12345678] * 4)
            fn = "CheckBeaconPos__6ccNaviFPfi" if op == "CheckBeaconPos" else "CheckBeaconPos2D__6ccNaviFPfi"
            call(fn, navi, V1, args[1])
            return {"ret": 0, "vec": list(g.get(V1, "<4I"))}
        if op == "GetDestination":
            self.vec_in(V1, vec(1))
            r = call("GetDestination__6ccNaviFPf", navi, V1)
            return {"ret": s32(r), "vec": list(g.get(V1, "<4I"))}
        if op == "SetDungeonMapInfo":
            call("SetDungeonMapInfo__6ccNaviFv", navi)
            return {"ret": 0}
        if op == "Ctor":
            p = m.load(navi + 0x64, 4)
            call("__ct__6ccNaviFv", navi)
            return {"ret": 0}
        if op == "SearchLandmark":
            return {"ret": s32(call("ccNaviSearchLandmark__Fi", args[0]))}
        if op == "GetLandmarkPos":
            self.vec_in(V1, vec(1))
            r = s32(call("ccNaviGetLandmarkPos__FPfi", V1, args[0]))
            return {"ret": r, "vec": list(g.get(V1, "<4I"))}
        if op == "SearchNearLandmark":
            self.vec_in(V1, vec(0))
            return {"ret": s32(call("ccNaviSearchNearLandmark__FPf", V1))}
        if op == "SearchNearLandmarkN":
            self.vec_in(V1, vec(0))
            return {"ret": s32(call("ccNaviSearchNearLandmarkN__FPfi", V1, args[4]))}
        if op == "SetNaviMap":
            call("ccSetNaviMap__Fv")
            return {"ret": 0}
        if op == "setpos":
            g.put(self.va(key) + 0x40, "<4I", *vec(1))
            return {"ret": 0}
        if op == "distpl":
            call("DistanceToTarget__4ccAIFP6ccChar", ai, m.load(self.p_party, 4))
            m.store(ai + volume.ai_at(0xA0), 4, m.f[0])
            return {"ret": 0}
        if op == "MoveP2P":
            self.vec_in(V1, vec(1))
            self.vec_in(V2, vec(5))
            call("MoveP2P__4ccAIFPfPfi", ai, V1, V2, args[9])
            return {"ret": m.f[0] & 0xFFFFFFFF}
        if op == "SetTargetPosDirc":
            self.vec_in(V1, vec(1))
            call("SetTargetPosDirc__4ccAIFPf", ai, V1)
            return {"ret": 0}
        if op == "FollowBeacon":
            call("FollowBeacon__4ccAIFv", ai)
            return {"ret": 0}
        if op == "CheckGoalBeaconPos":
            self.vec_in(V1, vec(1))
            return {"ret": s32(call("CheckGoalBeaconPos__4ccAIFPf", ai, V1))}
        if op == "ManualControl":
            return {"ret": s32(call("ManualControl__4ccAIFv", ai))}
        if op == "ManualMode":
            call("ManualMode__4ccAIFv", ai)
            return {"ret": 0}
        if op == "SetRemoteCmd":
            call("SetRemoteCmd__4ccAIFi", ai, args[1])
            return {"ret": 0}
        if op == "SetGoalPos":
            self.vec_in(V1, vec(1))
            call("SetGoalPos__4ccAIFPfi", ai, V1, args[5])
            return {"ret": 0}
        if op == "FollowTargetTown":
            t = args[1]
            call("FollowTargetTown__4ccAIFP6ccChar", ai, self.va(t) if t >= 0 else 0)
            return {"ret": 0}
        if op in ("TownNavigator", "TownNavigatorPoint"):
            self.vec_in(V1, vec(1))
            self.vec_in(V2, vec(5))
            fn = "TownNavigator__4ccAIFPfPfi" if op == "TownNavigator" else "TownNavigatorPoint__4ccAIFPfPfi"
            r = s32(call(fn, ai, V1, V2, args[9]))
            return {"ret": r, "vec": list(g.get(V1, "<4I"))}
        if op == "TownNavigatorPos":
            self.vec_in(V1, vec(1))
            self.vec_in(V2, vec(5))
            return {"ret": s32(call("TownNavigatorPos__4ccAIFPfPf", ai, V1, V2))}
        if op == "MessageIndex":
            return {"ret": s32(call("MessageIndex__4ccAIFv", ai))}
        if op == "ActInTown":
            return {"ret": s32(call("ActInTown__4ccAIFv", ai))}
        if op in ("townfull", "brainsfull"):
            g.put(self.va(key) + 0x40, "<4I", *vec(1))
            k = m.load(self.p_party, 4)
            if k:
                g.put(k + 0x40, "<4I", *vec(5))
                m.store(k + 0x68, 4, args[9])
                m.store(k + 0xE0, 1, (m.load(k + 0xE0, 1) & ~0x28) | (args[10] & 0x28))
            if op == "townfull":
                call("DistanceToTarget__4ccAIFP6ccChar", ai, k)
                m.store(ai + volume.ai_at(0xA0), 4, m.f[0])
                return {"ret": s32(call("ActInTown__4ccAIFv", ai))}
            return {"ret": s32(call("Brains__4ccAIFv", ai))}
        if op == "manualfull":
            return {"ret": s32(call("ManualControl__4ccAIFv", ai))}
        if op == "poke":
            g.put(ai + volume.ai_at(args[1]), "<h", args[2])
            return {"ret": 0}
        if op in ("townframe", "beaconframe"):
            g.put(self.va(key) + 0x40, "<4I", *vec(1))
            if op == "townframe":
                k = m.load(self.p_party, 4)
                if k:
                    g.put(k + 0x40, "<4I", *vec(5))
                call("DistanceToTarget__4ccAIFP6ccChar", ai, k)
                m.store(ai + volume.ai_at(0xA0), 4, m.f[0])
                return {"ret": s32(call("ActInTown__4ccAIFv", ai))}
            call("FollowBeacon__4ccAIFv", ai)
            return {"ret": 0}
        if op == "perform":
            which = args[1]
            if which == 0:
                self.vec_in(V1, vec(2))
                self.vec_in(V2, vec(6))
                return {"ret": s32(call("PathFindingInDungeon__6ccNaviFPfPf", navi, V1, V2))}
            if which == 1:
                call("FollowBeacon__4ccAIFv", ai)
                return {"ret": 0}
            if which == 2:
                self.vec_in(V1, vec(2))
                return {"ret": s32(call("CheckGoalBeaconPos__4ccAIFPf", ai, V1))}
            if which == 3:
                return {"ret": s32(call("ManualControl__4ccAIFv", ai))}
            return {"ret": s32(call("ActInTown__4ccAIFv", ai))}
        raise ValueError(op)

    def run(self, w, plan, path, sys_last=False):
        """The operations `plan(i, answers)` gives, one after another until
        it gives None, on the game's side, a snapshot after each (the bus in
        the last only with `sys_last`). Returns the operations and the
        answers."""
        self.put_world(w)
        ops, out = [], []
        while True:
            o = plan(len(ops), out)
            if o is None:
                break
            op, args = o
            self.calls = []
            r = self.run_op(w, op, list(args))
            r.update(self.snapshot(w, path, op == "SetNaviMap"))
            ops.append((op, list(args)))
            out.append(r)
        if sys_last:
            for r in out[:-1]:
                r.pop("sys", None)
        self.restore()
        return ops, out

    def restore(self):
        self.c.scene.restore()


# serialising for the probe ------------------------------------------------------------------

def ser_navi(nv):
    b = nv["beacon"] or []
    return " ".join(str(x) for x in list(nv["goal_pos"]) + [nv["start_x"], nv["start_y"], nv["goal_x"],
                                                              nv["goal_y"], nv["name"], nv["step"], nv["landmark"],
                                                              nv["dist"], nv["dirc"], nv["map_x"], nv["map_y"],
                                                              nv["map_s"]]) + \
        f" {hexs(nv['route'])} {nv['pad'][0]} {nv['pad'][1]} {nv['beacon_num']} " + \
        hexs(bytes(v for e in b for v in e))


def ser_world(w, b):
    chars = w["chars"]
    out = [str(len(chars))]
    for ch in chars:
        out.append(T.ser_char(ch, b))
        out.append(" ".join(str(x) for x in list(ch.dirc) + [ch.act_num_old, ch.transparency, ch.set_transparency,
                                                              ch.radius, ch.velocity, getattr(ch, "hit_sw", 0),
                                                              ch.height]))
    for lst in (w["pcs"], w["enes"]):
        out.append(" ".join(str(x) for x in [len(lst)] + list(lst)))
    p = w["party"]
    out.append(" ".join(str(x) for x in p["members"] + p["ids"] + [p["num"]]))
    out.append(" ".join(str(w["game"][k]) for k in PA.GAME_FIELDS))
    out.append(f"{w['save220d']} {w['registry']}")
    out.append(str(len(w["ais"])))
    for k, (ai, nv) in w["ais"].items():
        out.append(f"{k} {PA.ser_ai(ai)} {ser_navi(nv)}")
    s = w["sys"]
    out.append(" ".join(str(x) for x in (s["time"], s["entry_num"], s["history_top"], s["next_id"])))
    out += [PA.ser_msg(x) for x in s["buff"]] + [PA.ser_msg(x) for x in s["hist"]]
    out.append(" ".join(str(x) for x in s["entry"]))
    out.append(str(w["rand"]))
    out.append(f"{w['open']} {len(w['lines'])} " + " ".join(str(x) for x in w["lines"]))
    x0, y0, wd, ht = w["region"]
    out.append(f"{w['ready']} {x0} {y0} {wd} {ht} {hexs(w['link'])} "
               f"{hexs(b''.join(struct.pack('<h', v) for v in w['cost']))}")
    mx, my, mw, mh = w["map_region"]
    out.append(f"{mx} {my} {mw} {mh} {hexs(w['map'])}")
    out.append(" ".join(str(x) for x in w["info"]))
    scr = w.get("info_script", [])
    out.append(" ".join(str(x) for x in [len(scr)] + [v for i in scr for v in i]))
    tn = w["town"]
    out.append(str(tn["num"]) + " " + " ".join(" ".join(str(x) for x in list(pos) + [name])
                                                for pos, name in tn["marks"]))
    if w.get("found") is not None:
        out.append(str(len(w["found"])) + " " + " ".join(" ".join(str(x) for x in r) for r in w["found"]))
    else:
        out.append(str(len(w["routes"])) + " " + " ".join(
            f"0 {r[0]} {r[1]} {r[2]} {r[3]} {r[4]} {hexs(r[5])}" for r in w["routes"]))
    out.append(str(len(w["points"])) + " " + " ".join(
        " ".join(str(x) for x in [k] + list(v)) for k, v in w["points"].items()))
    out.append(str(len(w["markers"])) + " " + " ".join(
        " ".join(str(x) for x in [k] + list(v)) for k, v in w["markers"].items()))
    return " ".join(out)


def request(ops, w, b, path, sys_last=False):
    parts = [f"navi {int(path) | (2 if sys_last else 0)} {len(ops)}"]
    for op, args in ops:
        parts.append(f"{op} {len(args)} " + " ".join(str(a) for a in args))
    parts.append(ser_world(w, b))
    return " ".join(parts)




# random worlds ------------------------------------------------------------------------------

SIDES = {0: 10, 1: 20, 2: 40}           # a room's size code -> its window in map cells
ROOM_CELLS = {0: 4, 1: 8, 2: 16}        # ... and in the dungeon's 750-unit cells


def windows_of(fl):
    """GetRoom2DPos for each room of a floor: (x, y, size) in map cells."""
    out = []
    for r in fl.rooms:
        half = ROOM_CELLS[r["size"]] // 2
        px = eemu.f_mul(eemu.f_from_py(750.0), eemu.f_from_int(r["x"] + half))
        py = eemu.f_mul(eemu.f_from_py(750.0), eemu.f_from_int(r["y"] + half))
        s = SIDES[r["size"]]
        x = eemu.f_to_int(eemu.f_div(px, fl_300())) - s // 2
        y = eemu.f_to_int(eemu.f_div(py, fl_300())) - s // 2
        out.append((x, y, s, r["exits"]))
    return out


def fl_300():
    return 0x43960000


def floor_map(rnd, wins):
    """A 2D map for a floor's windows, as MakeMiniMap might leave it: each
    room open floor (1, some 2 and 3) walled round (4) but for its doors,
    with pillars, walls and holes inside."""
    cells = {}
    for (x0, y0, s, exits) in wins:
        for x in range(x0 - 1, x0 + s + 1):
            for y in range(y0 - 1, y0 + s + 1):
                edge = x in (x0 - 1, x0 + s) or y in (y0 - 1, y0 + s)
                if edge:
                    cells.setdefault((x, y), 4)
                else:
                    cells[(x, y)] = rnd.choice((1,) * 12 + (2, 3))
        mid = s // 2
        for bit, xs, ys in ((1, range(x0 + mid - 2, x0 + mid + 3), [y0 - 1]),
                            (2, range(x0 + mid - 2, x0 + mid + 3), [y0 + s]),
                            (4, [x0 - 1], range(y0 + mid - 2, y0 + mid + 3)),
                            (8, [x0 + s], range(y0 + mid - 2, y0 + mid + 3))):
            if exits & bit:
                for x in xs:
                    for y in ys:
                        cells[(x, y)] = 1
        for _ in range(rnd.randrange(0, 1 + s // 4)):
            w, h = rnd.randrange(1, max(2, s // 3)), rnd.randrange(1, max(2, s // 3))
            ax, ay = rnd.randrange(x0, x0 + s), rnd.randrange(y0, y0 + s)
            v = rnd.choice((4, 4, 4, 0))
            for x in range(ax, min(ax + w, x0 + s)):
                for y in range(ay, min(ay + h, y0 + s)):
                    cells[(x, y)] = v
    return cells


def region_of(wins, pad=1):
    x0 = max(0, min(w[0] for w in wins) - pad)
    y0 = max(0, min(w[1] for w in wins) - pad)
    x1 = min(256, max(w[0] + w[2] for w in wins) + pad)
    y1 = min(256, max(w[1] + w[2] for w in wins) + pad)
    return x0, y0, x1 - x0, y1 - y0


def rnd_cell_pos(rnd, x, y):
    """A world position in map cell (x, y)."""
    return [fl(x * 300 + rnd.uniform(0, 299.9)), fl(y * 300 + rnd.uniform(0, 299.9)), fl(rnd.uniform(-50, 50)),
            F_ONE]


class NaviChecks(T.Checks):
    def __init__(self):
        super().__init__()
        self.ng = NaviGame(self)
        import dungeon
        d = dungeon.Data(T.ELF)
        self.floors = list(dungeon.Edited(d, 14).floors)            # the tutorial dungeon
        self.floors += list(dungeon.Generator(d, 3996388677, 0, 4, 9).generate().floors)
        self.floors += list(dungeon.Generator(d, 1814669918, 3, 3, 7).generate().floors)
        self.floor_wins = [windows_of(fl) for fl in self.floors]
        # Mac Anu's landmark table as the executable holds it
        g = self.game
        base = g.m.load(g.sym("naviMapTownTable"), 4)
        self.town_num = s16(g.m.load(g.sym("naviMarkTable"), 2))
        self.town_names = [s32(g.m.load(base + 0x30 * k + 0x10, 4)) for k in range(self.town_num + 1)]
        self.town_base = base
        self.town01 = town01_dummies()

    # pieces of a world --------------------------------------------------------------------
    def rnd_char(self, rnd, kind, pos):
        b, tb, data = self.b, self.tb, self.data
        ch = tb.rnd_pc(b, data, rnd)
        ch.type = kind
        ch.pos = list(pos)
        ch.pos_p = list(pos) if rnd.random() < 0.9 else [fl(fbits_f(pos[0]) + rnd.uniform(-20, 20)),
                                                          fl(fbits_f(pos[1]) + rnd.uniform(-20, 20)), pos[2], F_ONE]
        ch.height = fl(rnd.uniform(10, 30))
        ch.width = rnd.choice((0, fl(rnd.uniform(0, 8))))
        h = rnd.uniform(-math.pi, math.pi)
        ch.dirc = [0, 0, fl(h), F_ONE] if rnd.random() < 0.8 else [fl(rnd.uniform(-1, 1)), 0, fl(h), 0]
        ch.act_num = rnd.choice((0, 0, 0, 1, 2, 2, 5, 6, 7, 8, 9, 12, 13, 14, 15, 16, 3, 4))
        ch.act_num_old = rnd.randrange(-1, 20)
        f = 0
        for bit, pr in ((1, 0.1), (2, 0.5), (4, 0.1), (8, 0.5), (16, 0.3), (32, 0.4), (64, 0.1), (1 << 8, 0.1),
                        (1 << 10, 0.1), (1 << 11, 0.1), (1 << 12, 0.3), (1 << 13, 0.1)):
            if rnd.random() < pr:
                f |= bit
        ch.spc_flags = f
        ch.party_flag = rnd.choice((1, 1, 1, 1, 0, 6, 7))
        ch.no_death = rnd.random() < 0.2
        ch.target_char = -1
        ch.transparency = rnd.choice((F_ONE, 0, fl(rnd.uniform(0, 1))))
        ch.set_transparency = rnd.choice((F_ONE, 0, fl(rnd.uniform(0, 1))))
        ch.cloak = rnd.choice((F_ONE, 0, fl(rnd.uniform(0, 1))))
        ch.radius = rnd.choice((F_ONE, fl(rnd.uniform(0, 40))))
        ch.velocity = fl(rnd.uniform(2, 20))
        ch.skill_id = rnd.choice((0, 0, 0, 1, rnd.randrange(0, 300)))
        ch.skill_status = rnd.choice((0, 0, 0, 1, rnd.randrange(0, 10)))
        ch.cond[0] = rnd.choice((0,) * 8 + (1, 4, 5))
        ch.anm_flag = 0
        return ch

    def rnd_navi(self, rnd, area, win=None, n_beacons=None):
        wx, wy, ws = win or (rnd.randrange(0, 200), rnd.randrange(0, 200), rnd.choice((10, 20, 40)))
        nv = {"goal_pos": [fl(rnd.uniform(-5000, 5000)), fl(rnd.uniform(-5000, 5000)), fl(rnd.uniform(-50, 50)),
                           F_ONE],
              "start_x": rnd.randrange(-5, 250), "start_y": rnd.randrange(-5, 250),
              "goal_x": rnd.randrange(-5, 250), "goal_y": rnd.randrange(-5, 250),
              "name": rnd.choice((0, 0, 1, 2, 3, 4, 5, 6, 7)), "step": 0, "landmark": 0,
              "dist": fl(rnd.uniform(0, 3000)), "dirc": fl(rnd.uniform(-3, 3)),
              "map_x": wx, "map_y": wy, "map_s": ws,
              "route": bytes(48), "pad": [rnd.randrange(256), rnd.randrange(256)], "beacon_num": 0, "beacon": None}
        if area == 2 or (n_beacons is not None):
            n = n_beacons if n_beacons is not None else rnd.randrange(1, 55)    # 55 on: the pointer
            cells = [(rnd.randrange(wx, wx + ws) & 0xFF, rnd.randrange(wy, wy + ws) & 0xFF) for _ in range(n)]
            nv["beacon"] = cells
            route = bytearray(48)
            for i in range(min(n, 48)):
                route[i] = i
            nv["route"] = bytes(route)
            if n > 48:
                nv["pad"] = [48, 49 if n > 49 else nv["pad"][1]]
            nv["beacon_num"] = n if rnd.random() < 0.9 else rnd.randrange(0, n + 1)
            nv["step"] = n
            nv["landmark"] = rnd.choice((0, rnd.randrange(0, n + 1), n - 1, n))
            nv["name"] = rnd.choice((0, 0, 0, nv["name"]))
        else:
            k = rnd.randrange(1, 12)
            marks = [rnd.randrange(1, self.town_num) for _ in range(k)]
            route = bytearray(48)
            route[0] = 254
            for i, v in enumerate(marks):
                route[1 + i] = v
            route[1 + k] = 255
            nv["route"] = bytes(route)
            nv["step"] = k + 1
            nv["landmark"] = rnd.choice((1, 1, rnd.randrange(0, k + 3), k, k + 1))
        return nv

    def rnd_route(self, rnd):
        """A RouteSearchByMap answer."""
        k = rnd.randrange(1, 10)
        route = bytearray(48)
        route[0] = 254
        for i in range(k):
            route[1 + i] = rnd.randrange(1, self.town_num)
        route[1 + k] = 255
        return (rnd.choice((1, 1, 1, 0)), rnd.choice((0, 0, 1, 2, 3, 4, 5, 6, 7)), k + 1,
                fl(rnd.uniform(0, 3000)), fl(rnd.uniform(-3, 3)), bytes(route))

    def rnd_town(self, rnd, centre, spread):
        marks = []
        for k in range(self.town_num + 1):
            p = [fl(fbits_f(centre[0]) + rnd.uniform(-spread, spread)),
                 fl(fbits_f(centre[1]) + rnd.uniform(-spread, spread)),
                 fl(rnd.choice((0.0, rnd.uniform(-20, 20), rnd.uniform(-600, 600)))), F_ONE]
            marks.append((p, self.town_names[k]))
        return {"num": self.town_num, "marks": marks}

    def world(self, rnd, area, member_pos=None, kite_pos=None, extra=0, town_spread=3000.0):
        """A member (scene index 1, with its AI) and Kite (0), maybe more;
        the party, the bus, the town, empty path maps."""
        centre = [fl(rnd.uniform(-3000, 3000)), fl(rnd.uniform(-3000, 3000)), 0, F_ONE]
        mp = member_pos or [fl(fbits_f(centre[0]) + rnd.uniform(-800, 800)),
                            fl(fbits_f(centre[1]) + rnd.uniform(-800, 800)), fl(rnd.uniform(-10, 10)), F_ONE]
        near = rnd.choice((30, 150, 300, 800, 3000))
        kp = kite_pos or [fl(fbits_f(mp[0]) + rnd.uniform(-near, near)), fl(fbits_f(mp[1]) + rnd.uniform(-near, near)),
                          mp[2], F_ONE]
        kite = self.rnd_char(rnd, rnd.choice((1, 1, 7)), kp)
        kite.id = 0
        member = self.rnd_char(rnd, 4, mp)
        member.id = rnd.choice((1, 1, 2, rnd.randrange(1, 18)))
        chars = [kite, member]
        for _ in range(extra):
            o = self.rnd_char(rnd, rnd.choice((4, 2, 6)), [fl(fbits_f(mp[0]) + rnd.uniform(-500, 500)),
                                                           fl(fbits_f(mp[1]) + rnd.uniform(-500, 500)), mp[2], F_ONE])
            o.id = rnd.randrange(2, 18)
            chars.append(o)
        for ch in chars:
            ch.target_char = rnd.choice([-1, -1] + list(range(len(chars))))
        pcs = [i for i in range(len(chars)) if rnd.random() > 0.08]
        party = {"members": [0, 1, 2 if len(chars) > 2 and rnd.random() < 0.5 else -1],
                 "ids": [chars[0].id, chars[1].id, chars[2].id if len(chars) > 2 else -1], "num": 2}
        if rnd.random() < 0.03:
            party["members"][0] = -1
        time = rnd.randrange(300, 100000)
        aid = rnd.randrange(0, 50)
        keys = list(range(len(chars)))
        ai = PA.PartyChecks.rnd_ai(None, rnd, 1, chars, aid, [aid], keys, time)
        ai[PA.AI_AT["body"]] = 1
        ai[PA.AI_AT["param"]] = member.id if member.id < 18 else 1
        ais = {1: (ai, self.rnd_navi(rnd, area))}
        for ch in chars:
            ch.ai = 0
        member.ai = 1
        entry = [1, -1, -1, -1, -1]
        sys_ = {"time": time, "entry_num": 1, "history_top": rnd.randrange(0, 16), "next_id": aid + 1,
                "buff": [PA.rnd_msg(rnd, [aid], keys, time, True) if rnd.random() < 0.2 else
                         [-1, 0, -1, 0, 0xFFFF, 0xFFFF, 0, 0] for _ in range(16)],
                "hist": [PA.rnd_msg(rnd, [aid], keys, time, False) for _ in range(16)], "entry": entry}
        game = {"in_battle": 0, "area": area, "field": 0, "field_type": 0, "field_attr": 0, "menu_type": -1,
                "event_lock": 0, "spc_battle_condition": 0, "party_strategy": 0, "player": 0, "field_24": 0,
                "pg_ride_flag": 0}
        return {"chars": chars, "pcs": pcs, "enes": [], "party": party, "area": area, "game": game,
                "open": rnd.choice((0, fl(rnd.uniform(-3, 3)))), "lines": [],
                "save220d": rnd.choice((0, 0, 1)), "registry": rnd.choice((5, 3, 4, 5)), "ais": ais, "sys": sys_,
                "rand": rnd.getrandbits(64), "ready": 1, "region": (0, 0, 0, 0), "link": b"", "cost": [],
                "map_region": (0, 0, 0, 0), "map": b"", "info": [0, 0, 0],
                "town": self.rnd_town(rnd, mp, town_spread),
                "routes": [self.rnd_route(rnd) for _ in range(rnd.randrange(0, 6))],
                "points": {k: [fl(fbits_f(mp[0]) + rnd.uniform(-500, 500)), fl(fbits_f(mp[1]) + rnd.uniform(-500, 500)),
                               0, F_ONE] for k in range(1, 8)},
                "markers": {}}

    def set_path(self, rnd, w, wins, cells, stale=0.5, region=None):
        """The dungeon's 2D map for these windows, and buf/buf2 around them
        left as earlier finds might have left them."""
        rx, ry, rw, rh = region or region_of(wins)
        mp = bytearray(rw * rh)
        for (x, y), v in cells.items():
            if rx <= x < rx + rw and ry <= y < ry + rh:
                mp[(x - rx) * rh + y - ry] = v
        w["map_region"] = (rx, ry, rw, rh)
        w["map"] = bytes(mp)
        w["region"] = (rx, ry, rw, rh)
        link, cost = bytearray(rw * rh), [0] * (rw * rh)
        if rnd.random() < stale:
            for i in range(rw * rh):
                if rnd.random() < 0.6:
                    link[i] = rnd.choice((rnd.randrange(16), rnd.randrange(16) | 0x80, rnd.randrange(16) | 0x20,
                                          rnd.randrange(256)))
                    cost[i] = rnd.choice((-1, -1, -2, 0, rnd.randrange(0, 60), rnd.randrange(-32768, 32768)))
        w["link"] = bytes(link)
        w["cost"] = cost

    def finish(self, w, plan, path, label, sys_last=False, bad=None):
        """Run the case on the game; None when `bad(answers)` rejects it."""
        try:
            ops, game = self.ng.run(w, plan, path, sys_last)
        except eemu.Stop:
            # a way of 55 cells or more overwrote the beacon table's
            # pointer and the game read through it (see too_long)
            if not w.get("full"):
                raise
            self.ng.restore()
            return None
        if bad is not None and bad(game):
            return None
        if w.get("full"):
            w["found"] = self.ng.found
        return request(ops, w, self.b, path, sys_last), game, label

    def rad2deg(self, r):
        """The game's RAD2DEG (main 0x001dab50), run."""
        m = self.game.m
        m.f[12] = r
        return s16(m.call(self.game.sym("RAD2DEG__Ff"), ()))

    @staticmethod
    def too_long(game):
        """A way of more than 54 cells overwrites the beacon table's pointer
        (the port does not follow the game there)."""
        for r in game:
            for a in r["ais"].values():
                if a[AI_STEP] > 54 or a[AI_STEP] < 0:
                    return True
        return False


def fbits_f(v):
    return struct.unpack("<f", struct.pack("<I", v & 0xFFFFFFFF))[0]


# the AI list's navigation fields, after PA.AI_LAYOUT's (the entry: 5 + 10)
_AI_BASE = sum(4 if f == "<4I" else 15 if f == "E" else 1 for _, _, f in PA.AI_LAYOUT)
AI_NAVI = {n: _AI_BASE + i for i, n in enumerate(
    ("gx", "gy", "gz", "gw", "start_x", "start_y", "goal_x", "goal_y", "name", "step", "landmark", "dist",
     "dirc", "map_x", "map_y", "map_s", "route", "pad0", "pad1", "beacon_num", "beacon"))}
AI_STEP = AI_NAVI["step"]


def static(ops):
    return lambda i, out: ops[i] if i < len(ops) else None


# the checks ---------------------------------------------------------------------------------

def rnd_window(rnd):
    s = rnd.choice((10, 10, 20, 20, 40, 80)) if rnd.random() < 0.97 else rnd.randrange(1, 12)
    x, y = rnd.randrange(1, 255 - s), rnd.randrange(1, 255 - s)
    if rnd.random() < 0.1:
        x = rnd.choice((0, 256 - s))
    if rnd.random() < 0.1:
        y = rnd.choice((0, 256 - s))
    return (x, y, s, rnd.randrange(16))


def cell_in(rnd, win, out=0.05):
    x0, y0, s, _ = win
    if rnd.random() < out:
        return rnd.randrange(max(0, x0 - 3), min(255, x0 + s + 3)), rnd.randrange(max(0, y0 - 3), min(255, y0 + s + 3))
    return rnd.randrange(x0, x0 + s), rnd.randrange(y0, y0 + s)


def near_cell(rnd, win, c, reach):
    x0, y0, s, _ = win
    return (min(x0 + s - 1, max(x0, c[0] + rnd.randrange(-reach, reach + 1))),
            min(y0 + s - 1, max(y0, c[1] + rnd.randrange(-reach, reach + 1))))


def check_set_path_finding_map(nc, rnd):
    w = nc.world(rnd, rnd.choice((2, 2, 2, 2, 0, 1)))
    win = rnd_window(rnd)
    cells = floor_map(rnd, [win])
    for _ in range(rnd.randrange(0, 40)):
        cells[cell_in(rnd, win, 0.3)] = rnd.choice((0, 1, 2, 3, 4, 5, 255))
    nc.set_path(rnd, w, [win], cells, stale=0.7)
    w["info"] = list(win[:3])
    w["ready"] = rnd.choice((0, 1))
    return nc.finish(w, static([("SetPathFindingMap", [])]), True, "SetPathFindingMap")


def path_case(nc, rnd, label, ints=False):
    """Finds in a random window: SetPathFindingMap, then one to four ways."""
    area = 2 if rnd.random() < 0.95 else rnd.choice((0, 1))
    w = nc.world(rnd, area)
    win = rnd_window(rnd)
    if rnd.random() < 0.2:
        cells = {}
        for x in range(win[0] - 1, win[0] + win[2] + 1):
            for y in range(win[1] - 1, win[1] + win[2] + 1):
                cells[(x, y)] = rnd.choice((1,) * rnd.randrange(1, 12) + (4, 0))
    else:
        cells = floor_map(rnd, [win])
    nc.set_path(rnd, w, [win], cells, stale=0.5)
    w["info"] = list(win[:3])
    nv = w["ais"][1][1]
    nv["map_x"], nv["map_y"], nv["map_s"] = win[:3]
    if rnd.random() < 0.05:
        nv["map_s"] = rnd.randrange(-2, 3)
    if rnd.random() < 0.2:
        nv["beacon"] = None             # no table yet: SetBeacon frees none
    ops = []
    if rnd.random() < 0.9:
        ops.append(("SetPathFindingMap", []))
    else:
        w["ready"] = rnd.choice((0, 1))
    reach = rnd.choice((3, 6, 12, 25, win[2]))
    for _ in range(rnd.randrange(1, 5)):
        s = cell_in(rnd, win)
        g = near_cell(rnd, win, s, reach) if rnd.random() < 0.9 else cell_in(rnd, win, 0.2)
        if rnd.random() < 0.05:
            g = s
        if ints and rnd.random() < 0.3:
            # one end, one coordinate just outside the window
            x0, y0, sz, _ = win
            k = rnd.randrange(8)
            v = rnd.choice((x0 - 1, x0 + sz)) if k % 2 == 0 else rnd.choice((y0 - 1, y0 + sz))
            if k < 4:
                s = (v, s[1]) if k % 2 == 0 else (s[0], v)
            else:
                g = (v, g[1]) if k % 2 == 0 else (g[0], v)
        if ints:
            ops.append(("PathFinding", [1, s[0], s[1], g[0], g[1]]))
        else:
            a, b = rnd_cell_pos(rnd, *s), rnd_cell_pos(rnd, *g)
            ops.append(("PathFindingF", [1] + a + b))
    return nc.finish(w, static(ops), True, label, bad=NaviChecks.too_long)


def check_path_finding(nc, rnd):
    return path_case(nc, rnd, "PathFinding")


def check_path_finding_cells(nc, rnd):
    return path_case(nc, rnd, "PathFindingCells", ints=True)


def maze_case(nc, rnd):
    """ShortPath down a full breadth-first field over a serpentine window
    (40 or 80 cells), the goal far along it: a wave value with bit 7 set
    stops the walk."""
    w = nc.world(rnd, 2)
    s = rnd.choice((40, 80))
    x0, y0 = rnd.randrange(1, 255 - s), rnd.randrange(1, 255 - s)
    win = (x0, y0, s, 0)
    cells = {}
    for x in range(x0, x0 + s):
        for y in range(y0, y0 + s):
            row = y - y0
            wall = row % 2 == 1 and not ((row // 2) % 2 == 0 and x == x0 + s - 1 or (row // 2) % 2 == 1 and x == x0)
            cells[(x, y)] = 4 if wall else 1
    nc.set_path(rnd, w, [win], cells, stale=0)
    rx, ry, rw, rh = w["region"]
    start = (x0, y0)
    dist = {start: 0}
    queue = [start]
    for c in queue:
        for d in ((0, -1), (1, 0), (0, 1), (-1, 0)):
            n = (c[0] + d[0], c[1] + d[1])
            if n in dist or not (x0 <= n[0] < x0 + s and y0 <= n[1] < y0 + s) or cells[n] == 4:
                continue
            dist[n] = dist[c] + 1
            queue.append(n)
    cost = [-1] * (rw * rh)
    for (x, y), v in dist.items():
        cost[(x - rx) * rh + y - ry] = min(v, 32767)
    far = [c for c, v in dist.items() if 120 < v < 600]
    g = rnd.choice(far) if far else rnd.choice(list(dist))
    cost[(g[0] - rx) * rh + g[1] - ry] = -2
    w["cost"] = cost
    nv = w["ais"][1][1]
    nv["map_x"], nv["map_y"], nv["map_s"] = win[:3]
    return nc.finish(w, static([("ShortPath", [1, g[0], g[1]])]), True, "PathPieces")


def check_path_pieces(nc, rnd):
    """HeuristicType1, ShortPath, MakeBeaconTbl and SetBeacon one by one,
    on the state PathFindingInDungeon leaves before each."""
    if rnd.random() < 0.15:
        return maze_case(nc, rnd)
    w = nc.world(rnd, 2)
    win = rnd_window(rnd)
    win = (win[0], win[1], min(win[2], 40), win[3])
    cells = floor_map(rnd, [win])
    nc.set_path(rnd, w, [win], cells, stale=0.5)
    w["info"] = list(win[:3])
    nv = w["ais"][1][1]
    nv["map_x"], nv["map_y"], nv["map_s"] = win[:3]
    s = cell_in(rnd, win, 0)
    g = near_cell(rnd, win, s, rnd.choice((3, 8, 20)))
    if g == s:
        g = (win[0] + (s[0] - win[0] + 1) % win[2], s[1])
    link = bytearray(w["link"])
    rx, ry, rw, rh = w["region"]
    # PathFindingInDungeon's preparation, done here: the window's buf2 at
    # -1 and its buf masked, the start 0 and the goal -2 (buf after
    # SetPathFindingMap).
    mp = {k: v for k, v in cells.items()}

    def walk(c):
        return c not in (0, 4)
    for x in range(rw):
        for y in range(rh):
            X, Y = rx + x, ry + y
            if win[0] <= X < win[0] + win[2] and win[1] <= Y < win[1] + win[2]:
                c = mp.get((X, Y), 0)
                v = 0
                if walk(c):
                    v = ((1 if Y > 0 and walk(mp.get((X, Y - 1), 0)) else 0)
                         + (2 if X < 255 and walk(mp.get((X + 1, Y), 0)) else 0)
                         + (4 if Y < 255 and walk(mp.get((X, Y + 1), 0)) else 0)
                         + (8 if X > 0 and walk(mp.get((X - 1, Y), 0)) else 0))
                link[x * rh + y] = v
                w["cost"][x * rh + y] = -1
    w["link"] = bytes(link)
    w["cost"][(s[0] - rx) * rh + s[1] - ry] = 0
    w["cost"][(g[0] - rx) * rh + g[1] - ry] = -2

    def plan(i, out):
        if i == 0:
            return ("Heuristic", [1])
        if i == 1:
            return ("ShortPath", [1, g[0], g[1]])
        if i == 2:
            return ("MakeBeaconTbl", [1, g[0], g[1]]) if out[1]["ret"] == 1 else None
        if i == 3 and out[2]["ret"] > 0:
            tbl = bytes.fromhex(out[2]["tbl"]) if out[2]["tbl"] != "-" else b""
            if len(tbl) // 4 > 54:
                return None
            return ("SetBeacon", [1] + list(tbl))
        if i == 4:
            n = out[3]["ret"]
            return ("CheckGoalBeaconPos", [1] + rnd_cell_pos(rnd, *(g if rnd.random() < 0.5 else s)))
        return None
    return nc.finish(w, plan, True, "PathPieces")


def check_link_queries(nc, rnd):
    """LinkInfo, MinimumLinkDirc and LinkNum on any buf2."""
    w = nc.world(rnd, 2)
    win = rnd_window(rnd)
    win = (win[0], win[1], min(win[2], 20), win[3])
    nc.set_path(rnd, w, [win], {}, stale=0)
    rx, ry, rw, rh = w["region"]
    w["cost"] = [rnd.choice((-1, -1, -2, 0, 1, 16, 32, 15, 17, rnd.randrange(0, 200), 32767, -32768,
                             rnd.randrange(-32768, 32768))) for _ in range(rw * rh)]
    nv = w["ais"][1][1]
    nv["map_x"], nv["map_y"], nv["map_s"] = win[:3]
    ops = []
    for _ in range(rnd.randrange(1, 12)):
        x, y = cell_in(rnd, win, 0.2)
        ops.append((rnd.choice(("LinkInfo", "MinimumLinkDirc", "LinkNum")), [1, x, y]))
    return nc.finish(w, static(ops), False, "LinkQueries")


def check_beacon_pos(nc, rnd):
    """CheckBeaconPos, CheckBeaconPos2D, GetDestination, CheckGoalBeaconPos."""
    area = rnd.choice((2, 2, 0, 1, 3))
    w = nc.world(rnd, area)
    nv = w["ais"][1][1]
    if area == 2 and nv["beacon"] is None:
        nv["beacon"] = [(0, 0)]
    if area != 2 and rnd.random() < 0.5:
        nv.update(nc.rnd_navi(rnd, 2, n_beacons=rnd.randrange(1, 55)))
    n = len(nv["beacon"] or [])
    ops = []
    for _ in range(rnd.randrange(1, 8)):
        k = rnd.choice(("CheckBeaconPos", "CheckBeaconPos2D", "GetDestination", "CheckGoalBeaconPos"))
        if k in ("CheckBeaconPos", "CheckBeaconPos2D"):
            # route[54] on is the beacon table's pointer (not modelled)
            ops.append((k, [1, rnd.randrange(-1, min(max(n, 1) + 2, 54)) if n else rnd.randrange(0, 48)]))
        elif k == "GetDestination":
            ops.append((k, [1] + [fl(rnd.uniform(-9, 9)) for _ in range(4)]))
        else:
            b = nv["beacon"] or [(0, 0)]
            c = rnd.choice(b) if rnd.random() < 0.6 else (rnd.randrange(0, 256), rnd.randrange(0, 256))
            ops.append((k, [1] + rnd_cell_pos(rnd, *c)))
    if n == 0:
        ops = [o for o in ops if o[0] not in ("CheckBeaconPos", "CheckBeaconPos2D", "CheckGoalBeaconPos")] or \
            [("GetDestination", [1, 0, 0, 0, 0])]
    return nc.finish(w, static(ops), False, "BeaconPos")


def check_navi_ctor(nc, rnd):
    area = rnd.choice((2, 0, 1))
    w = nc.world(rnd, area)
    w["info"] = [rnd.randrange(-5, 250), rnd.randrange(-5, 250), rnd.choice((10, 20, 40, 80))]
    ops = [("Ctor", [1])] if rnd.random() < 0.7 else [("SetDungeonMapInfo", [1])]
    return nc.finish(w, static(ops), False, "NaviCtor")


def check_real_floors(nc, rnd):
    """Rooms of the tutorial dungeon's floors and of random dungeons: the
    member's room built (SetPathFindingMap), ways found in it, then the
    next room's, with what the earlier rooms left in buf and buf2."""
    k = rnd.randrange(len(nc.floors))
    wins = nc.floor_wins[k]
    w = nc.world(rnd, 2)
    cells = floor_map(rnd, wins)
    start = rnd.randrange(len(wins))
    seq = [start]
    for _ in range(rnd.randrange(0, 3)):
        a = wins[seq[-1]]
        nxt = [i for i, b in enumerate(wins) if i != seq[-1] and abs(b[0] - a[0]) <= a[2] + b[2]
               and abs(b[1] - a[1]) <= a[2] + b[2]]
        seq.append(rnd.choice(nxt) if nxt else rnd.randrange(len(wins)))
    used = [wins[i] for i in seq]
    nc.set_path(rnd, w, used, cells, stale=0.2, region=region_of(used, 2))
    # Each room: the member's AI made anew there (SetDungeonMapInfo), the
    # room built (SetPathFindingMap), both asking Get2DMapInfo for the
    # room's window; then ways found in it.
    ops = []
    for i in seq:
        win = wins[i]
        ops += [("info", list(win[:3])), ("SetDungeonMapInfo", [1]), ("SetPathFindingMap", [])]
        for _ in range(rnd.randrange(1, 4)):
            s = cell_in(rnd, win, 0.02)
            g = near_cell(rnd, win, s, rnd.choice((4, 10, 20, win[2])))
            ops.append(("PathFindingF", [1] + rnd_cell_pos(rnd, *s) + rnd_cell_pos(rnd, *g)))
    nv = w["ais"][1][1]
    nv["map_x"], nv["map_y"], nv["map_s"] = wins[seq[0]][:3]
    w["info"] = list(wins[seq[0]][:3])
    return nc.finish(w, room_plan(w, ops), True, "RealFloors", bad=NaviChecks.too_long)


def room_plan(w, ops):
    """The operations without the ("info", [x, y, s]) steps, which become
    what Get2DMapInfo answers from then on (a script both sides read)."""
    real = [o for o in ops if o[0] != "info"]
    script = []
    cur = list(w["info"])
    for op, args in ops:
        if op == "info":
            cur = list(args)
        elif op in ("SetDungeonMapInfo", "SetPathFindingMap"):
            script.append(list(cur))
    w["info_script"] = script
    return static(real)


# the movers -------------------------------------------------------------------------------------

def near(rnd, p, r):
    """A point within about r of p on the ground."""
    a = rnd.uniform(-math.pi, math.pi)
    d = rnd.uniform(0, r)
    return [fl(fbits_f(p[0]) + d * math.sin(a)), fl(fbits_f(p[1]) - d * math.cos(a)), p[2], F_ONE]


def destination(w, nv, area):
    """Where GetDestination points (None in a field)."""
    if area == 1:
        return None
    if nv["landmark"] < nv["step"]:
        i = nv["landmark"]
        rb = (list(nv["route"]) + nv["pad"] + list(struct.pack("<i", nv["beacon_num"])))
        r = rb[i] if 0 <= i < len(rb) else 0
        if area == 0:
            marks = w["town"]["marks"]
            return list(marks[r][0]) if r < len(marks) else [0, 0, 0, 0]
        b = nv["beacon"] or []
        x, y = b[r] if r < len(b) else (0, 0)
        return [fl(300 * x + 150), fl(300 * y + 150), 0, F_ONE]
    return list(nv["goal_pos"])


def ai_set(ai, **kw):
    for k, v in kw.items():
        ai[PA.AI_AT[k]] = v


def set_flag(ai, bit, on):
    f = ai[PA.AI_AT["flags"]]
    ai[PA.AI_AT["flags"]] = (f | 1 << bit) if on else (f & ~(1 << bit))


def check_move_p2p(nc, rnd):
    w = nc.world(rnd, rnd.choice((0, 1, 2)))
    me = w["chars"][1]
    me.act_num = rnd.choice((0, 0, 2, 7, 8, 15, 16, 5, 9))
    if rnd.random() < 0.6:
        me.skill_id, me.skill_status = 0, 0
    ops = []
    for _ in range(rnd.randrange(1, 6)):
        p1 = me.pos if rnd.random() < 0.7 else near(rnd, me.pos, 500)
        r = rnd.choice((5, 20, 40, 100, 1000, fbits_f(me.velocity) + fbits_f(me.radius)))
        p2 = near(rnd, p1, r)
        if rnd.random() < 0.05:
            p2 = list(p1)
        ops.append(("MoveP2P", [1] + list(p1) + p2 + [rnd.choice((0, 1, 1, 2, -1))]))
    return nc.finish(w, static(ops), False, "MoveP2P")


def check_set_target_pos_dirc(nc, rnd):
    w = nc.world(rnd, rnd.choice((0, 1, 2)))
    me = w["chars"][1]
    ops = [("SetTargetPosDirc", [1] + near(rnd, me.pos, rnd.choice((1, 50, 5000)))) for _ in range(rnd.randrange(1, 4))]
    return nc.finish(w, static(ops), False, "SetTargetPosDirc")


def beacon_world(nc, rnd, area):
    """A member walking a route: a town's (area 0) or a dungeon's beacons
    (2), near where it leads."""
    w = nc.world(rnd, area, town_spread=rnd.choice((300.0, 1500.0)))
    nv = w["ais"][1][1]
    if area == 2:
        win = rnd_window(rnd)
        nv.update(nc.rnd_navi(rnd, 2, win[:3], n_beacons=rnd.randrange(1, 55)))
        if rnd.random() < 0.7:
            # a walk: neighbouring cells
            x, y = cell_in(rnd, win, 0)
            cells = []
            for _ in range(len(nv["beacon"])):
                cells.append((x & 0xFF, y & 0xFF))
                dx, dy = rnd.choice(((1, 0), (-1, 0), (0, 1), (0, -1)))
                x, y = x + dx, y + dy
            nv["beacon"] = cells
    ai = w["ais"][1][0]
    ai[PA.AI_AT["naviFinish"]] = rnd.choice((0, 0, 0, 1))
    d = destination(w, nv, area)
    if d is not None and rnd.random() < 0.6:
        w["chars"][1].pos = near(rnd, d, rnd.choice((100, 150, 160, 400)))
        w["chars"][1].pos_p = list(w["chars"][1].pos)
    return w


def check_follow_beacon(nc, rnd):
    # Not in a field: GetDestination leaves the destination unset there
    # (stack garbage in the game), and no field calls FollowBeacon.
    area = rnd.choice((0, 2))
    w = beacon_world(nc, rnd, area)
    return nc.finish(w, static([("FollowBeacon", [1])]), False, "FollowBeacon")


def check_manual_control(nc, rnd):
    """ManualControl on every remote command, after ManualMode,
    SetRemoteCmd and SetGoalPos as the events set it up."""
    area = rnd.choice((0, 0, 2, 1))
    w = beacon_world(nc, rnd, area)
    me = w["chars"][1]
    ai = w["ais"][1][0]
    heading = rnd.randrange(-32768, 32768)
    # In a field only straight walks: the route's destination is unset
    # there (GetDestination).
    ai_set(ai, remoteCmd=rnd.choice((0, 1, 2, 3, 4, 5, 6, 7, 1, 2, 0, -1, 8)),
           gPoint=rnd.choice((-1, -1, -2, rnd.randrange(1, 72), 0, 80)) if area != 1 else -1,
           gDeg=rnd.choice((heading & 0xFFFF, rnd.randrange(0, 65536))),
           gRotSp=rnd.choice((64, 64, 16, 1, -1, 256, 3, -32768)))
    ai[PA.AI_AT["gPos"]] = near(rnd, me.pos, rnd.choice((30, 49, 60, 500)))
    me.act_num = rnd.choice((2, 14, 14, 12, 0, 13, 5))
    me.cond[0] = rnd.choice((0, 0, 5, 1, 4))
    if rnd.random() < 0.4:
        # facing gDeg already, stopFlag set: done turning. Only heading 0
        # ever compares equal (gDeg is read unsigned against RAD2DEG's
        # sign-extended short, and RAD2DEG(DEG2RAD(s)) is s - 1 for s > 0).
        if rnd.random() < 0.5:
            me.dirc[2] = 0
        ai_set(ai, gDeg=nc.rad2deg(me.dirc[2]) & 0xFFFF)
        me.spc_flags |= 0x10
        if rnd.random() < 0.5:
            ai_set(ai, remoteCmd=0)
    nv = w["ais"][1][1]
    d = destination(w, nv, area)
    if d is not None and rnd.random() < 0.3:
        me.pos = near(rnd, d, rnd.choice((5, 20)))
        me.pos_p = list(me.pos)
        ai_set(ai, gPoint=-2)
    ops = []
    for _ in range(rnd.randrange(0, 3)):
        k = rnd.random()
        if k < 0.3:
            ops.append(("ManualMode", [1]))
        elif k < 0.6:
            ops.append(("SetRemoteCmd", [1, rnd.choice((0, 1, 2, 3, 4, 5, 6, 7, 5))]))
        else:
            t = rnd.choice((0, 1, 2)) if area != 1 else rnd.choice((0, 2))
            ops.append(("SetGoalPos", [1] + near(rnd, me.pos, rnd.choice((30, 500))) + [t]))
    for _ in range(rnd.randrange(1, 5)):
        ops.append(("ManualControl", [1]))
    return nc.finish(w, static(ops), False, "ManualControl")


def check_follow_target_town(nc, rnd):
    w = nc.world(rnd, 0, extra=rnd.choice((0, 1)))
    me = w["chars"][1]
    t = rnd.choice((0, 0, 0, len(w["chars"]) - 1, -1))
    if t >= 0 and rnd.random() < 0.7:
        w["chars"][t].pos = near(rnd, me.pos_p, rnd.choice((40, 60, 170, 200, 400)))
    ops = [("FollowTargetTown", [1, t]) for _ in range(rnd.randrange(1, 4))]
    return nc.finish(w, static(ops), False, "FollowTargetTown")


def check_town_navigator(nc, rnd):
    w = nc.world(rnd, 0)
    w["routes"] = [nc.rnd_route(rnd) for _ in range(6)]
    me = w["chars"][1]
    ops = []
    for _ in range(rnd.randrange(1, 5)):
        k = rnd.random()
        goal = near(rnd, me.pos, 3000)
        start = list(me.pos) if rnd.random() < 0.7 else near(rnd, me.pos, 500)
        if k < 0.3:
            ops.append(("TownNavigator", [1] + goal + start + [rnd.choice((1, 2, 3, 4, 5, 6, 0, 7, -1))]))
        elif k < 0.6:
            ops.append(("TownNavigatorPoint", [1] + goal + start + [rnd.choice((1, 30, 71, 72, 73, 0, -1))]))
        elif k < 0.85:
            ops.append(("TownNavigatorPos", [1] + start + goal))
        else:
            ops.append(("MessageIndex", [1]))
    return nc.finish(w, static(ops), False, "TownNavigator")


def check_landmarks(nc, rnd):
    w = nc.world(rnd, 0, town_spread=rnd.choice((500.0, 3000.0, 8000.0)))
    me = w["chars"][1]
    ops = []
    for _ in range(rnd.randrange(1, 6)):
        k = rnd.random()
        p = near(rnd, me.pos, rnd.choice((100, 3000, 12000)))
        if rnd.random() < 0.3:
            p[2] = fl(rnd.uniform(-500, 500))
        if k < 0.25:
            ops.append(("SearchLandmark", [rnd.choice((1, 2, 3, 4, 5, 6, 0, 7, -1))]))
        elif k < 0.5:
            ops.append(("GetLandmarkPos", [rnd.choice((1, 30, 71, 72, 73, 0, -1))] + p))
        elif k < 0.75:
            ops.append(("SearchNearLandmark", p))
        else:
            ops.append(("SearchNearLandmarkN", p + [rnd.choice((3, 3, 1, 2, 5, 80))]))
    return nc.finish(w, static(ops), False, "Landmarks")


def check_set_navi_map(nc, rnd):
    """ccSetNaviMap on a town's own table: its landmarks moved to their
    DMY_marker dummies."""
    w = nc.world(rnd, 0)
    t = rnd.randrange(0, 5)
    g = nc.game
    m = g.m
    base = s32(m.load(g.sym("naviMapTownTable") + 4 * t, 4))
    num = s16(m.load(g.sym("naviMarkTable") + 2 * t, 2))
    marks = []
    for k in range(num + 1):
        a = base + 0x30 * k
        marks.append((list(g.get(a, "<4I")), s32(m.load(a + 0x10, 4))))
    w["town"] = {"num": num, "marks": marks}
    w["town_index"] = t
    w["markers"] = {k: [fl(rnd.uniform(-9000, 9000)), fl(rnd.uniform(-9000, 9000)), fl(rnd.uniform(-100, 100)),
                        F_ONE] for k in range(1, num) if rnd.random() < 0.95}
    r = nc.finish(w, static([("SetNaviMap", [])]), False, "SetNaviMap")
    m.mem[base:base + 0x30 * (num + 1)] = m.pristine[base:base + 0x30 * (num + 1)]
    return r


def town_world(nc, rnd):
    """Orca in Mac Anu: an ActInTown state, landmarks and Kite about. Most
    worlds let the state's own code run (no recall, in the party, no chat
    command, Kite listed); the state's tests are then set up to go either
    way about half the time."""
    w = nc.world(rnd, 0, town_spread=rnd.choice((200.0, 600.0, 1500.0)))
    w["routes"] = [nc.rnd_route(rnd) for _ in range(rnd.randrange(3, 12))]
    ai = w["ais"][1][0]
    me, kite = w["chars"][1], w["chars"][0]
    nv = w["ais"][1][1]
    half = lambda: rnd.random() < 0.5                    # noqa: E731
    act = rnd.choice((0, 1, 2, 3, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 20, 94, 94, 95, 96, 96, 97, 97, 98, 99, 99,
                      100, 101, 13, 50))
    mode = rnd.choice((1,) * 8 + (5, 5, 5, 0, 3))
    ai_set(ai, mode=mode, actType=act,
           actTypeOld=rnd.choice((0, 3, 5, 6, 7, 8, 9, 10, 97, 99, 11)),
           actTime=rnd.choice((1, 1, 1, 2, 5, 0, rnd.randrange(-5, 300))),
           actDummy=rnd.randrange(1, nc.town_num),
           lastMarker=rnd.choice((0, 1, 2, 3, 4, 5, 6)),
           noMoveCnt=rnd.choice((0, 5, 199, 200)) if half() else rnd.choice((201, 250, 32767)),
           distPl=fl(rnd.uniform(0, 279.9) if half() else rnd.choice((280.0, 499.0, 500.0, 501.0,
                                                                       rnd.uniform(280, 3000)))),
           chatCmd=rnd.choice((12, 9, -1, 3, 12, 9)))
    ai[PA.AI_AT["naviFinish"]] = rnd.choice((0, 1))
    ai[PA.AI_AT["posOld"]] = list(me.pos) if half() else near(rnd, me.pos, 3)
    f = ai[PA.AI_AT["flags"]]
    f &= ~(0x7 << 12)
    f |= rnd.choice((0, 0, 0, 1, 2, 6, 7)) << 12             # chatCmdFlag
    ai[PA.AI_AT["flags"]] = f
    set_flag(ai, 0, False)                                    # not under remote control
    set_flag(ai, 6, half())                                   # inviteFlag
    set_flag(ai, 1, half())                                   # followSW
    me.party_flag = rnd.choice((1, 1, 1, 1, 7, 6, 0))
    me.act_num = rnd.choice((0, 0, 0, 2, 12, 13, 14, 5))
    if half():
        me.skill_id, me.skill_status = 0, 0
    me.spc_flags = (me.spc_flags & ~(1 << 12)) | (1 << 12 if rnd.random() < 0.3 else 0)
    kite.spc_flags = (kite.spc_flags & ~0x10) | (0x10 if half() else 0)        # Kite's stopFlag
    w["registry"] = rnd.choice((5, 5, 3))
    nv["name"] = rnd.choice((0, 1, 2, 3, 4, 5, 6))
    if rnd.random() < 0.7:
        # the state's own code runs
        me.party_flag = 1
        me.spc_flags &= ~(1 << 12)
        ai[PA.AI_AT["flags"]] &= ~(0x7 << 12)
        if 0 not in w["pcs"]:
            w["pcs"].insert(0, 0)
        if mode == 0 or mode == 3:
            ai_set(ai, mode=1)
    elif rnd.random() < 0.5 and 0 in w["pcs"]:
        w["pcs"].remove(0)
    if half():
        nv["landmark"] = max(nv["step"] - 1, 0)
    marks = w["town"]["marks"]
    vel, rad = fbits_f(me.velocity), fbits_f(me.radius)

    def put_me(target, r):
        me.pos = near(rnd, target, r)
        me.pos_p = list(me.pos)
    kite.pos = near(rnd, me.pos, rnd.choice((100, 140, 160, 190, 210, 400, 499, 600, 3000)))
    kite.pos_p = list(kite.pos)
    d = destination(w, nv, 0)
    if act in (5, 6, 7, 8, 9, 10, 20, 11, 12, 4, 0) and d is not None and half():
        put_me(d, (2 * vel + rad) * rnd.choice((0.2, 0.8, 1.2)))
    if act in (5, 6, 7, 8, 9) and half():
        ai_set(ai, lastMarker={5: 2, 6: 3, 7: 6, 8: 4, 9: 5}[act])
    if act == 2 and half():
        # walking to landmark 1: arriving at the route's next landmark, the
        # last or not
        ai[PA.AI_AT["naviFinish"]] = 0
        if nv["step"] > 1:
            nv["landmark"] = rnd.choice((nv["step"] - 1, rnd.randrange(0, nv["step"])))
        r_ = nv["route"][nv["landmark"]] if 0 <= nv["landmark"] < 48 else 0
        put_me(marks[r_][0] if r_ < len(marks) else [0, 0, 0, F_ONE], rnd.choice((10, 40, 49, 60)))
    if act in (4, 12) and half():
        put_me(marks[ai[PA.AI_AT["actDummy"]]][0], rnd.choice((10, 40, 60)))
    if act == 3:
        if half():
            ai[PA.AI_AT["naviFinish"]] = 0
        if d is not None and half():
            put_me(d, rnd.choice((20, 100, 140, 160)))
        kite.pos = near(rnd, me.pos, rnd.choice((150, 190, 199)) if rnd.random() < 0.3 else rnd.choice((250, 600)))
        if half():
            last = nv["route"][nv["step"] - 1] if 0 < nv["step"] <= 48 else 0
            if last < len(marks):
                kite.pos = near(rnd, marks[last][0], 5)
        kite.pos_p = list(kite.pos)
    if act in (94, 97) or mode == 5:
        kite.pos = near(rnd, me.pos, rnd.choice((100, 149, 160, 169, 175, 250, 270, 499, 520, 900)))
        kite.pos_p = list(kite.pos)
    if act == 94 and half():
        ai_set(ai, noMoveCnt=rnd.choice((201, 300)))
        kite.pos = near(rnd, me.pos, rnd.choice((400, 499, 520, 900)))
        kite.pos_p = list(kite.pos)
    if mode == 5 and half():
        me.act_num = rnd.choice((7, 8, 15, 16, 0))          # not able to turn (CheckAction(2))
    if act in (3, 94, 97, 0, 5) and rnd.random() < 0.25:
        # the follow command (9) or stop (12) while following or not
        f = ai[PA.AI_AT["flags"]] & ~(0x7 << 12)
        ai[PA.AI_AT["flags"]] = f | 1 << 12
        ai_set(ai, chatCmd=rnd.choice((9, 12)))
    if act == 96 and half():
        me.act_num = 13
    if act == 99 and half():
        me.id = 1
    if act == 100 and half():
        me.act_num = 2
    if act == 101 and half():
        ai_set(ai, actTypeOld=97)
    return w


def check_act_in_town(nc, rnd):
    w = town_world(nc, rnd)
    return nc.finish(w, static([("ActInTown", [1])]), False, "ActInTown")


def mover(nc, w, key, speed_walk, speed_run, stuck=0.05, kite=None):
    """The next position of a moving body from the game's memory: along its
    heading at its walking or running pace while moveFlag is set."""
    m, g = nc.game.m, nc.game
    va = nc.scene.va(key)
    f = m.load(va + 0xE0, 4)
    pos = list(g.get(va + 0x40, "<4I"))
    if not f & 8:
        return pos
    rnd = kite
    if rnd is not None and rnd.random() < stuck:
        return pos
    h = fbits_f(m.load(va + 0x68, 4))
    v = speed_run if f & 0x20 else speed_walk
    return [fl(fbits_f(pos[0]) + v * math.sin(h)), fl(fbits_f(pos[1]) - v * math.cos(h)), pos[2], pos[3]]


def check_act_in_town_run(nc, rnd):
    """ActInTown for 60-300 frames: the member moved along its heading at
    its stride while it walks (or held in place for a while), Kite
    wandering, distPl measured each frame; now and then a timer cut short
    or the state changed as an event would (`poke`)."""
    w = town_world(nc, rnd)
    me = w["chars"][1]
    me.act_num = rnd.choice((0, 0, 0, 2, 13))
    me.skill_id, me.skill_status = 0, 0
    ai = w["ais"][1][0]
    ai_set(ai, mode=rnd.choice((1, 1, 1, 1, 5)), actTime=rnd.choice((1, 2, 10, 40)))
    w["town"] = nc.rnd_town(rnd, me.pos, rnd.choice((200.0, 400.0, 800.0)))
    frames = rnd.randrange(60, 301)
    w["routes"] = [nc.rnd_route(rnd) for _ in range(60)]
    vel = fbits_f(me.velocity)
    walk, run_ = vel * rnd.uniform(0.6, 1.2), vel * rnd.uniform(1.0, 1.8)
    kite_state = {"pos": list(w["chars"][0].pos)}
    prng = random.Random(rnd.getrandbits(32))
    stuck = {"until": -1}

    def plan(i, out):
        if i >= frames:
            return None
        k = prng.random()
        if i and k < 0.04:
            return ("poke", [1, 0x8A, prng.choice((1, 1, 2, 0))])              # actTime
        if i and k < 0.06:
            return ("poke", [1, 0x86, prng.choice((0, 2, 3, 5, 6, 7, 8, 9, 10, 20, 94, 95, 96, 97, 99, 100, 101))])
        if i and k < 0.07:
            return ("poke", [1, 0x98, prng.choice((195, 200, 201))])           # noMoveCnt
        if i and prng.random() < 0.01:
            stuck["until"] = i + prng.randrange(5, 250)
        mp = list(w["chars"][1].pos) if not i else (
            current_pos(nc, 1) if i < stuck["until"] else mover(nc, w, 1, walk, run_, kite=prng))
        kp = kite_state["pos"]
        k = prng.random()
        if k < 0.02:
            kp = near(prng, mp, prng.choice((100, 190, 600, 3000)))
        elif k < 0.5:
            kp = near(prng, kp, 15)
        kite_state["pos"] = kp
        return ("townframe", [1] + list(mp) + list(kp))
    return nc.finish(w, plan, False, "ActInTownRun", sys_last=True)


def current_pos(nc, key):
    return list(nc.game.get(nc.scene.va(key) + 0x40, "<4I"))


def check_follow_beacon_run(nc, rnd):
    """FollowBeacon for 60-300 frames along a town route or a dungeon way
    found first, the member moved along its heading."""
    area = rnd.choice((0, 2, 2))
    frames = rnd.randrange(60, 301)
    prng = random.Random(rnd.getrandbits(32))
    if area == 2:
        w = nc.world(rnd, 2)
        win = rnd_window(rnd)
        win = (win[0], win[1], min(win[2], 40), win[3])
        cells = floor_map(rnd, [win])
        nc.set_path(rnd, w, [win], cells, stale=0.3)
        w["info"] = list(win[:3])
        nv = w["ais"][1][1]
        nv["map_x"], nv["map_y"], nv["map_s"] = win[:3]
        s = cell_in(rnd, win, 0)
        gcell = near_cell(rnd, win, s, rnd.choice((4, 10, 20)))
        start = rnd_cell_pos(rnd, *s)
        w["chars"][1].pos = list(start)
        w["chars"][1].pos_p = list(start)
        first = [("SetPathFindingMap", []), ("PathFindingF", [1] + start + rnd_cell_pos(rnd, *gcell))]
    else:
        w = beacon_world(nc, rnd, 0)
        first = []
    me = w["chars"][1]
    me.act_num = rnd.choice((0, 0, 0, 2))
    me.skill_id, me.skill_status = 0, 0
    vel = max(fbits_f(me.velocity), 5.0)
    walk, run_ = vel * rnd.uniform(0.6, 1.2), vel * rnd.uniform(1.0, 1.8)

    def plan(i, out):
        if i < len(first):
            return first[i]
        if i >= len(first) + frames:
            return None
        mp = mover(nc, w, 1, walk, run_, kite=prng) if i > len(first) else list(w["chars"][1].pos)
        return ("beaconframe", [1] + list(mp))
    return nc.finish(w, plan, area == 2, "FollowBeaconRun", sys_last=True, bad=NaviChecks.too_long)


def check_perform(nc, rnd):
    """The runtime's building block (ai_move::perform) on each call it
    takes."""
    which = rnd.randrange(5)
    if which in (0, 2):
        w = beacon_world(nc, rnd, 2)
        win = (w["ais"][1][1]["map_x"], w["ais"][1][1]["map_y"], w["ais"][1][1]["map_s"], 15)
        nc.set_path(rnd, w, [win], floor_map(rnd, [win]), stale=0.3)
        w["info"] = list(win[:3])
        a = rnd_cell_pos(rnd, *cell_in(rnd, win, 0))
        b = rnd_cell_pos(rnd, *cell_in(rnd, win, 0))
        ops = [("SetPathFindingMap", []), ("perform", [1, which] + a + b)]
    elif which == 1:
        w = beacon_world(nc, rnd, rnd.choice((0, 2)))
        ops = [("perform", [1, 1])]
    elif which == 3:
        w = beacon_world(nc, rnd, 0)
        # gRotSp 0 would divide by zero in ccSetDirc
        ai_set(w["ais"][1][0], remoteCmd=rnd.randrange(0, 8), gPoint=rnd.choice((-1, -2, 5)),
               gRotSp=rnd.choice((64, 16, 1)))
        ops = [("perform", [1, 3])]
    else:
        w = town_world(nc, rnd)
        ops = [("perform", [1, 4])]
    return nc.finish(w, static(ops), which in (0, 2), "Perform", bad=NaviChecks.too_long)


# the movers composed -------------------------------------------------------------------------

def town01_dummies():
    """Mac Anu's dummies (town01's DMY_* objects, as tools/test_world_rs.py
    reads them from DATA.BIN): name -> (x, y, z, 1.0) bits."""
    import ccs
    import gzarc
    data = gzarc.open_bytes(os.path.join(T.ROOT, "work", "infection", "disc", "DATA", "DATA.BIN"))
    members = {mm.name.lower(): mm for mm in gzarc.members(data)}
    c = ccs.Ccs(gzarc.inflate(data, members["town01.cmp"]))
    out = {}
    for off, t, _n, _end in c.chunks():
        if t is None or t & 0xFFFF not in (0x1300, 0x1400):
            continue
        obj = struct.unpack_from("<I", c.data, off + 8)[0]
        out.setdefault(c.objects[obj][0], list(struct.unpack_from("<3I", c.data, off + 12)) + [F_ONE])
    return out


def mac_anu(nc, w, rnd):
    """The composed checks' Mac Anu: the landmark table as GCMN.PRG holds it
    (ccSetNaviMap moves it to the town01 dummies first), the landmarks'
    DMY_markerNN and the name table's dummies (DMY_gate, DMY_merchant1-5)."""
    g = nc.game
    m = g.m
    base = nc.town_base
    marks = []
    for k in range(nc.town_num + 1):
        a = base + 0x30 * k
        marks.append((list(g.get(a, "<4I")), s32(m.load(a + 0x10, 4))))
    w["town"] = {"num": nc.town_num, "marks": marks}
    w["town_index"] = 0
    d = nc.town01
    w["markers"] = {k: d[f"DMY_marker{k:02d}"] for k in range(1, nc.town_num) if f"DMY_marker{k:02d}" in d}
    names = [None, "DMY_gate"] + [f"DMY_merchant{i}" for i in range(1, 7)]
    w["points"] = {k: d.get(names[k], [fl(rnd.uniform(-3000, 3000)), fl(rnd.uniform(-3000, 3000)), 0, F_ONE])
                   for k in range(1, 8)}
    w["route_lines"] = [rnd.choice((F_M1, F_M1, fl(0.5))) for _ in range(40)]
    w["full"] = True
    return [w["markers"][k] for k in sorted(w["markers"])]


def restore_town(nc):
    g = nc.game
    n = 0x30 * (nc.town_num + 1)
    g.m.mem[nc.town_base:nc.town_base + n] = g.m.pristine[nc.town_base:nc.town_base + n]


def check_act_in_town_full(nc, rnd):
    """Orca in Mac Anu for 60-300 frames with the movement composed: on the
    game's side ccAI::ActInTown and all it calls run natively - FollowPlayer,
    LeavePlayer, FollowTargetTown, the TownNavigators, and RouteSearchByMap
    over Mac Anu's own tables (the landmarks first moved to town01's
    dummies by ccSetNaviMap); on the port's, party_motion::Movement performs
    Call::ActInTown (ai_move through its Nested runtime, follow::Follow),
    over a world whose route search answers what the game's found. The
    member moves along its heading between frames, Kite wanders the town."""
    w = town_world(nc, rnd)
    pts = mac_anu(nc, w, rnd)
    me, kite = w["chars"][1], w["chars"][0]
    me.act_num = rnd.choice((0, 0, 0, 2, 13))
    me.skill_id, me.skill_status = 0, 0
    kite.skill_id, kite.skill_status = 0, 0
    ai = w["ais"][1][0]
    ai_set(ai, mode=rnd.choice((1, 1, 1, 1, 5)), actTime=rnd.choice((1, 2, 10, 40)))
    # Orca among the landmarks, Kite near him
    start = rnd.choice(pts)
    me.pos = near(rnd, start, rnd.choice((50, 300, 800)))
    me.pos_p = list(me.pos)
    kite.pos = near(rnd, me.pos, rnd.choice((100, 190, 400, 600, 3000)))
    kite.pos_p = list(kite.pos)
    kite.dirc = [0, 0, fl(rnd.uniform(-math.pi, math.pi)), F_ONE]
    ai[PA.AI_AT["posOld"]] = list(me.pos)
    nv = w["ais"][1][1]
    nv["landmark"] = min(nv["landmark"], max(nv["step"] - 1, 0))
    frames = rnd.randrange(60, 301)
    vel = fbits_f(me.velocity)
    walk, run_ = vel * rnd.uniform(0.6, 1.2), vel * rnd.uniform(1.0, 1.8)
    kite_state = {"pos": list(kite.pos)}
    prng = random.Random(rnd.getrandbits(32))
    stuck = {"until": -1}

    def plan(i, out):
        if i == 0:
            return ("SetNaviMap", [])
        i -= 1
        if i >= frames:
            return None
        k = prng.random()
        if i and k < 0.03:
            return ("poke", [1, 0x8A, prng.choice((1, 1, 2, 0))])              # actTime
        if i and k < 0.05:
            return ("poke", [1, 0x86, prng.choice((0, 2, 3, 5, 6, 7, 8, 9, 10, 20, 94, 95, 96, 97, 99, 100, 101))])
        if i and prng.random() < 0.01:
            stuck["until"] = i + prng.randrange(5, 250)
        mp = list(w["chars"][1].pos) if not i else (
            current_pos(nc, 1) if i < stuck["until"] else mover(nc, w, 1, walk, run_, kite=prng))
        kp, heading, flags = kite_step(prng, mp, kite_state["pos"], prng.choice((100, 190, 600, 3000)))
        kite_state["pos"] = kp
        return ("townfull", [1] + list(mp) + list(kp) + [heading, flags])
    r = nc.finish(w, plan, False, "ActInTownFull", sys_last=True)
    restore_town(nc)
    return r


def check_manual_control_full(nc, rnd):
    """ManualControl under an event's commands for 30-120 frames, composed:
    Movement performs Call::ManualControl (its HitEnable on the member's
    bodyHit through the world, its route through the world's search, the
    game's run natively over Mac Anu's tables); the commands change as the
    events give them (ManualMode, SetRemoteCmd, SetGoalPos by landmark or
    point), the member moving along its heading between frames."""
    w = town_world(nc, rnd)
    pts = mac_anu(nc, w, rnd)
    me = w["chars"][1]
    me.skill_id, me.skill_status = 0, 0
    me.act_num = rnd.choice((2, 14, 14, 12, 0))
    me.cond[0] = rnd.choice((0, 0, 5, 1))
    me.hit_sw = 0
    ai = w["ais"][1][0]
    ai_set(ai, remoteCmd=rnd.choice((0, 1, 2, 3, 4, 5, 6, 7)), gPoint=rnd.choice((-1, -2, 5, 30)),
           gRotSp=rnd.choice((64, 16, 1)))
    me.pos = near(rnd, rnd.choice(pts), 400)
    me.pos_p = list(me.pos)
    ai[PA.AI_AT["gPos"]] = near(rnd, me.pos, rnd.choice((30, 500, 2000)))
    frames = rnd.randrange(30, 121)
    vel = fbits_f(me.velocity)
    walk, run_ = vel * rnd.uniform(0.6, 1.2), vel * rnd.uniform(1.0, 1.8)
    prng = random.Random(rnd.getrandbits(32))

    def plan(i, out):
        if i == 0:
            return ("SetNaviMap", [])
        i -= 1
        if i >= 2 * frames:
            return None
        if i % 2 == 1:
            mp = mover(nc, w, 1, walk, run_, kite=prng)
            return ("setpos", [1] + list(mp))
        k = prng.random()
        if k < 0.04:
            return ("ManualMode", [1])
        if k < 0.1:
            return ("SetRemoteCmd", [1, prng.choice((0, 1, 2, 3, 4, 5, 6, 7))])
        if k < 0.14:
            goal = prng.choice(pts) if prng.random() < 0.5 else near(prng, current_pos(nc, 1), 1500)
            return ("SetGoalPos", [1] + list(goal) + [prng.choice((0, 1, 1))])
        return ("manualfull", [1])
    r = nc.finish(w, plan, False, "ManualControlFull", sys_last=True)
    restore_town(nc)
    return r


def kite_step(prng, mp, kp, reach):
    """Kite's next position, heading and walking (moveFlag 8, runFlag 0x20
    of his flag byte)."""
    k = prng.random()
    if k < 0.02:
        nkp = near(prng, mp, reach)
    elif k < 0.55:
        nkp = near(prng, kp, prng.choice((5, 15, 30)))
    else:
        nkp = list(kp)
    dx, dy = fbits_f(nkp[0]) - fbits_f(kp[0]), fbits_f(nkp[1]) - fbits_f(kp[1])
    moving = (dx, dy) != (0.0, 0.0) and k >= 0.02
    heading = fl(math.atan2(dx, -dy)) if moving else fl(prng.uniform(-math.pi, math.pi))
    flags = (8 if moving else 0) | (0x20 if moving and prng.random() < 0.4 else 0)
    return nkp, heading, flags


def check_brains_dungeon(nc, rnd):
    """A dungeon room for 60-300 frames with the party AI's whole frame
    composed: on the game's side ccAI::Brains runs natively and all it
    calls (ActInDungeon, CheckEyeLineToTarget, PathFindingInDungeon,
    FollowBeacon, CheckGoalBeaconPos, FollowPlayer, LeavePlayer,
    FollowTarget...), ccHitCheckLM answering from a script; on the port's,
    party_ai::Ctx::brains with party_motion::Movement for its runtime. Orca
    idle in a room (no foes), Kite wandering out of sight so that Brains
    asks for a way and walks it."""
    w = nc.world(rnd, 2)
    win = rnd_window(rnd)
    win = (min(win[0], 200), min(win[1], 200), rnd.choice((10, 20, 20, 40)), win[3])
    cells = floor_map(rnd, [win])
    nc.set_path(rnd, w, [win], cells, stale=0.3)
    w["info"] = list(win[:3])
    w["full"] = True
    w["game"].update(in_battle=0, area=2, field=rnd.choice((0, 1)), party_strategy=rnd.randrange(0, 4))
    me, kite = w["chars"][1], w["chars"][0]
    me.skill_id, me.skill_status = 0, 0
    kite.skill_id, kite.skill_status = 0, 0
    me.act_num = rnd.choice((0, 0, 2))
    me.party_flag = 1
    me.cond = [0] * 16
    me.cond[0] = 0
    kite.cond = [0] * 16
    for ch in (me, kite):
        ch.target_char = -1
    w["pcs"] = [0, 1]
    # Kite in the party: without him the game's eye line reads a null
    # character's position (party_ai skips it)
    w["party"]["members"][0] = 0
    ai, nv = w["ais"][1]
    nv["map_x"], nv["map_y"], nv["map_s"] = win[:3]
    nv["beacon"] = None
    nv["beacon_num"], nv["step"], nv["landmark"] = 0, 0, 0
    ai[PA.AI_AT["naviFinish"]] = 1
    f = ai[PA.AI_AT["flags"]] & ~(0x7 << 12) & ~(3 << 8) & ~(3 << 10)
    f &= ~(1 | 4 | 16 | 32 | 64 | 128)                    # no manual, talk, remote, going back, invite, self
    f = (f & ~2) | (2 if rnd.random() < 0.4 else 0)       # followSW
    ai[PA.AI_AT["flags"]] = f
    ai_set(ai, mode=1, target=-1, targetCCmd=-1, chatCmd=-1, chatCmdNew=-1, detourCnt=0,
           levelOld=me.level, count=rnd.randrange(0, 1000))
    ai[PA.AI_AT["entry"]][2] = 0                          # no messages waiting
    w["sys"]["buff"] = [[-1, 0, -1, 0, 0xFFFF, 0xFFFF, 0, 0] for _ in range(16)]
    w["lists"] = {me.id: ([0] * 20, [(0, 0, 0)] * 40), kite.id: ([0] * 20, [(0, 0, 0)] * 40)}

    def spot():
        x, y = cell_in(rnd, win, 0)
        return rnd_cell_pos(rnd, x, y)
    me.pos = spot()
    me.pos_p = list(me.pos)
    kite.pos = spot()
    kite.pos_p = list(kite.pos)
    ai[PA.AI_AT["posOld"]] = list(me.pos)
    w["lines"] = [rnd.choice((F_M1, F_M1, F_M1, fl(0.5), 0)) for _ in range(3000)]
    frames = rnd.randrange(60, 301)
    vel = max(fbits_f(me.velocity), 5.0)
    walk, run_ = vel * rnd.uniform(0.6, 1.2), vel * rnd.uniform(1.0, 1.8)
    prng = random.Random(rnd.getrandbits(32))
    st = {"kp": list(kite.pos)}
    x0, y0, sz, _ = win

    def inside(p):
        return [fl(min(max(fbits_f(p[0]), 300 * x0 + 1), 300 * (x0 + sz) - 1)),
                fl(min(max(fbits_f(p[1]), 300 * y0 + 1), 300 * (y0 + sz) - 1)), p[2], p[3]]

    def plan(i, out):
        if i == 0:
            return ("SetPathFindingMap", [])
        i -= 1
        if i >= frames:
            return None
        mp = list(w["chars"][1].pos) if not i else inside(mover(nc, w, 1, walk, run_, kite=prng))
        kp, heading, flags = kite_step(prng, mp, st["kp"], 300 * sz)
        kp = inside(kp)
        st["kp"] = kp
        return ("brainsfull", [1] + list(mp) + list(kp) + [heading, flags])
    return nc.finish(w, plan, True, "BrainsDungeon", sys_last=True, bad=NaviChecks.too_long)


# the runner -----------------------------------------------------------------------------------

def first_diff(port, game, path=""):
    """Where two answers first differ."""
    if isinstance(game, dict) and isinstance(port, dict):
        for k in game:
            if k not in port:
                return f"{path}.{k}: port lacks it"
            d = first_diff(port[k], game[k], f"{path}.{k}")
            if d:
                return d
        for k in port:
            if k not in game:
                return f"{path}.{k}: game lacks it"
        return None
    if isinstance(game, list) and isinstance(port, list):
        if len(game) != len(port):
            return (f"{path}: port has {len(port)} items, game {len(game)}: port {str(port)[:300]} "
                    f"game {str(game)[:300]}")
        for i, (a, b) in enumerate(zip(port, game)):
            d = first_diff(a, b, f"{path}[{i}]")
            if d:
                return d
        return None
    if port != game:
        return f"{path}: port {port!r} game {game!r}"
    return None


def diff(port, game, what):
    game = T.norm(game)
    if port == game:
        return None
    d = first_diff(port, game)
    return f"{what}: {d}"


BATCH = 40


def run(checks, name, n, seed):
    """n cases of one check (retrying those a check rejects), sent to the
    probe in batches."""
    rnd = random.Random(seed)
    fn = CHECKS_FN[name]
    bad, first, first_req = 0, None, None
    done = 0
    while done < n:
        lines, answers = [], []
        rejected = 0
        while len(lines) < min(BATCH, n - done):
            checks.reset()
            r = fn(checks, rnd)
            if r is None:
                rejected += 1
                if rejected > 200:
                    raise RuntimeError(f"{name}: 200 cases in a row rejected")
                continue
            rejected = 0
            req, game, label = r
            lines.append(req)
            answers.append((game, label))
        got = T.ask(lines)
        for k, (game, label) in enumerate(answers):
            d = diff(got[k], game, label)
            if d:
                bad += 1
                if first is None:
                    first, first_req = d, lines[k]
        done += len(lines)
    return bad, first, first_req


CHECKS_FN = {}


def register():
    CHECKS_FN.update({
        "SetPathFindingMap": check_set_path_finding_map,
        "PathFinding": check_path_finding,
        "PathFindingCells": check_path_finding_cells,
        "PathPieces": check_path_pieces,
        "LinkQueries": check_link_queries,
        "BeaconPos": check_beacon_pos,
        "NaviCtor": check_navi_ctor,
        "RealFloors": check_real_floors,
        "Landmarks": check_landmarks,
        "SetNaviMap": check_set_navi_map,
        "MoveP2P": check_move_p2p,
        "SetTargetPosDirc": check_set_target_pos_dirc,
        "FollowBeacon": check_follow_beacon,
        "ManualControl": check_manual_control,
        "FollowTargetTown": check_follow_target_town,
        "TownNavigator": check_town_navigator,
        "ActInTown": check_act_in_town,
        "ActInTownRun": check_act_in_town_run,
        "FollowBeaconRun": check_follow_beacon_run,
        "Perform": check_perform,
        "ActInTownFull": check_act_in_town_full,
        "ManualControlFull": check_manual_control_full,
        "BrainsDungeon": check_brains_dungeon,
    })


def bulk(n, only=None):
    register()
    T.build()
    checks = NaviChecks()
    total = 0
    for k, name in enumerate(CHECKS_FN):
        if only and name not in only:
            continue
        bad, first, req = run(checks, name, n, 23000 + 37 * k)
        total += bad
        print(f"{name:24} {n:6} cases, {bad} mismatches" + (f"; first: {first[:1500]}" if first else ""),
              flush=True)
        if first:
            print(f"  request: {req[:600]}")
    return total


def _test(names, seed):
    def test(self):
        for k, name in enumerate(names):
            self.check(name, seed + 37 * k)
    return test


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class NaviAgainstGame(unittest.TestCase):
    CASES = 150

    @classmethod
    def setUpClass(cls):
        register()
        T.build()
        cls.checks = NaviChecks()

    def check(self, name, seed):
        bad, first, req = run(self.checks, name, self.CASES, seed)
        self.assertEqual(bad, 0, f"{first}\nrequest: {(req or '')[:600]}")

    test_path_finding = _test(["SetPathFindingMap", "PathFinding", "PathFindingCells", "PathPieces",
                               "LinkQueries", "BeaconPos", "NaviCtor", "RealFloors"], 23500)
    test_town = _test(["Landmarks", "SetNaviMap", "TownNavigator"], 23600)
    test_movers = _test(["MoveP2P", "SetTargetPosDirc", "FollowBeacon", "ManualControl", "FollowTargetTown"], 23700)
    test_act_in_town = _test(["ActInTown", "Perform"], 23800)
    test_runs = _test(["ActInTownRun", "FollowBeaconRun"], 23900)
    test_composed = _test(["ActInTownFull", "ManualControlFull", "BrainsDungeon"], 23950)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        n = int(sys.argv[2]) if len(sys.argv) > 2 else 200
        sys.exit(1 if bulk(n, sys.argv[3:]) else 0)
    unittest.main()
