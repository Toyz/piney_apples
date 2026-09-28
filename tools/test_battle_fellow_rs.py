#!/usr/bin/env python3
"""piney_battle::fellow and ::follow against the game's ccFellow (fellow.cpp)
and ccAI's following (personal.cpp) run in the EE interpreter.

Each check builds a random world on tools/test_battle_party_ai_rs.py's (party
members, foes, their ccAI objects on a ccAISystem with waiting and delivered
messages, the save's skill and item lists) and adds every ccSpcChar/ccFellow
member a party member's frame reads and writes: the flag word at +0xe0, the
act counters, speed, the heading, bodyHit, the carrier offset, movePos, the
animation table. It runs one function natively in the interpreter and the
same case through the battle_probe example (`fel` requests), and compares
the return value, every character's rule state and frame state, every ccAI
(the message queues included), the bus, the command lists, the AI globals,
aiOpenDirc, the rand() state, and three logs in order: the calls into the
rest of the game (ccSkillRequest, the navigation, the chat lines), the world's
calls, and the outputs (effects, the rules' events).

The world is stubbed in the interpreter: every call is logged with its
arguments and answered from a random script both sides read in the same
order - ccTransPosW2P/P2W (a translation), ccLandHitCheck,
checkHitResultAttlibute, ccHitCheckLM and ccHitCheckLM2 (the lines of sight
the decisions ask stay theirs), ccCharHit::CollisionDetection
(the push and hitResultCharType), HitEnable/HitDisable, ccAnm::SetAnm (the
clip's name), _AnimateForward, NoteProcess (which calls ccFellowCheckNote for
each scripted note, natively), WORLD_MAN::GetTransMode, GetTransCenter,
CheckEventArea and ccChar::Draw. The effects are recorded. ccSkillRequest is
recorded and modelled the same on both sides (sid 0 ends a running normal
attack; another starts the skill on its target). ccChar::EntryAffect is
recorded and then run for real with each character's affectFunc
(ccFellow::Influence, Kite's Influence, ccEnemyInfluence and affectEnemy),
only their presentation stubbed (numbers, hit marks, the panel, the pad, the
AI's lines, ccEnemy::selectTarget recorded; ccSkillCheck answers the case's
constant), so the checks see the affects land where the game makes them.
Everything else - the decisions, the bus, CalcReal, CheckLevelUp,
CalcBattleDamage, ccSkillDamageValue, rand - runs natively. The port
compares its affect state too (affectPerson, affectType, affectParam, the
flash, the mask and tint, the flag byte, a foe's flag word).

The runs (RunField, RunDungeon, and each with foes about) play 60 to 300
frames of ccThAISystem and ccFellow::Main for Orca (charTbl row 2) following
Kite along a scripted path, and compare everything after every frame.

RunDungeonNavi and RunFieldNavi check the party's movement assembled: the
port's frame runs with piney_battle::party_motion::Movement for its runtime
(the following, the dungeon's path finding, FollowBeacon and
CheckGoalBeaconPos all the port's), and in the game PathFindingInDungeon
with everything under it, SetPathFindingMap (before the first frame),
FollowBeacon and CheckGoalBeaconPos run natively, WORLD_MAN::Get2DPos and
Get3DPos too; Get2DMapInfo answers the room's window and Get2DMapPtr the
map, ccMalloc is a bump allocator. The world is a room of the dungeon's 2D
map (10 or 20 cells, one or two walls across it, pillars, doors with
corridors; or a room of the tutorial dungeon or a random dungeon laid out as
tools/test_battle_navi_rs.py lays it): the lines (ccHitCheckLM, LM2, the
decisions' included) cross it cell by cell, the ground is flat, a body is
pushed out of another's; the game's answers are kept and sent to the probe
as its script. Kite walks the room's open ways (now and then out of a door,
from outside it, or standing in a pocket no way leads into; now and then the
room's map is never made); Orca starts behind a wall within the path
finding's reach (sometimes beyond it). Each frame compares, besides the
rest, the AI's whole ccNavi (its beacon table included) and bufFlag with the
CRC-32 of buf and buf2.

    python3 tools/test_battle_fellow_rs.py            the unit tests
    python3 tools/test_battle_fellow_rs.py bulk N     N cases of every check
"""

import math
import os
import random
import struct
import sys
import unittest
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import eemu                                    # noqa: E402
import test_anim                               # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle as TB                       # noqa: E402
import test_battle_navi_rs as NV               # noqa: E402
import test_battle_party_ai_rs as P            # noqa: E402
import test_battle_rs as T                     # noqa: E402
from test_battle_rs import fbits               # noqa: E402

F_M1 = 0xBF800000
F_ONE = 0x3F800000
ANM = 0x01080000        # a ccAnm per character at ANM + 0x200 * i
WORLDMAN = 0x01088000
NOTE = 0x01089000       # the ccAnmNote NoteProcess hands on
STREAM = 0x0108A000
CHUNK = 0x0108B000
SCRATCH = 0x0108C000
FELLOW_ANIM_TBL = inf_va(0x005D4050)
HEAP = 0x01400000       # ccMalloc's bump region in the navigation runs
MAP2D = 0x01500000      # the dungeon's 2D map there
NAVI = 0xF0             # ccAI.navi
VT_FELLOW = P.VT_FELLOW
# ccAI::CheckFrontObstacle and CheckFrontObstacleF: their ccHitCheckLM is the
# world's; the decisions' (CheckEyeLineToTarget, the searches) stays theirs
FRONT = (inf_va(0x00589E90), inf_va(0x0058AE50))


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def fl(x):
    return fbits(x)


def ff(b):
    return struct.unpack("<f", struct.pack("<I", b & 0xFFFFFFFF))[0]


def nested_call(m, addr, args):
    """m.call from inside a hook: the machine's call starts afresh at the
    top of the stack with every register cleared, so the caller's frames
    and registers are kept aside and put back."""
    sp = m.r[29] & 0xFFFFFFFF
    frames = bytes(m.mem[sp:eemu.STACK_TOP])
    regs = [m.r[i] for i in range(32)]
    hi, lo = m.hi, m.lo
    r = m.call(addr, args)
    m.mem[sp:eemu.STACK_TOP] = frames
    for i in range(32):
        m.r[i] = regs[i]
    m.hi, m.lo = hi, lo
    return r


# the fellow's state ---------------------------------------------------------------------

# (name, offset, struct format) in the order the probe reads them
FEL_LAYOUT = [
    ("flags", 0xE0, "<I"), ("arms", 0xE4, "<i"), ("act", 0xEE, "<h"), ("act_old", 0xF0, "<h"),
    ("atk_anm", 0xF2, "<h"), ("anm_flag", 0xF4, "<h"), ("act_cnt", 0xF8, "<h"), ("react", 0xFA, "<h"),
    ("stop_cnt", 0xFC, "<h"), ("attack", 0xFE, "<h"), ("lag", 0x100, "<h"), ("speed", 0x104, "<I"),
    ("rate", 0x108, "<I"), ("now_speed", 0x10C, "<I"), ("cloak", 0x110, "<I"), ("cnt", 0x114, "<i"),
    ("cycle", 0x11C, "<i"), ("consec", 0x120, "<i"), ("walk", 0x124, "<i"),
    ("dirc", 0x60, "<4I"), ("hit_attr", 0x80, "<I"), ("trans", 0x88, "<I"), ("set_trans", 0x8C, "<I"),
    ("hsw", 0x1A0, "<i"), ("hmask", 0x1A4, "<I"), ("hmask2", 0x1A8, "<I"), ("htype", 0x1AC, "<I"),
    ("hradius", 0x1B4, "<I"), ("hheight", 0x1B8, "<I"), ("hpos", 0x1C0, "<4I"), ("hoff", 0x1D0, "<4I"),
    ("hattr", 0x1E0, "<I"), ("fbyte", 0x200, "<B"), ("disp_wait", 0x204, "<i"), ("atk_dellay", 0x208, "<i"),
    ("dist_tg", 0x20C, "<I"), ("disk", 0x220, "<4I"), ("move_pos", 0x230, "<4I"), ("motion", 0x240, "M"),
    ("target", 0x78, "C"), ("skill_id", 0x7C, "<h"), ("skill_status", 0x7E, "<h"), ("pos", 0x40, "<4I"),
    ("pos_p", 0x50, "<4I"),
]


def rnd_angle(rnd):
    return fl(rnd.uniform(-math.pi, math.pi))


def rnd_vec(rnd, span, z=40.0):
    return [fl(rnd.uniform(-span, span)), fl(rnd.uniform(-span, span)), fl(rnd.uniform(-z, z)), F_ONE]


class FellowGame:
    """The party harness's world plus a party member's frame: the ccFellow
    members, the world's stubs and the logs."""

    def __init__(self, pc):
        self.pc = pc
        self.ai = pc.ai
        self.g = pc.c.game
        self.m = self.g.m
        self.sym = self.g.sym
        m, sym, g = self.m, self.sym, self.g
        self.va = pc.c.scene.va
        # the functions under test run for real
        for n in ("FollowTarget__4ccAIFP6ccChar", "FollowPlayer__4ccAIFv", "LeavePlayer__4ccAIFv",
                  "FollowTargetDirc__4ccAIFP6ccChar", "DEG2RAD__Fs", "CheckSpRegeneSpeed__9ccSpcCharFv",
                  "checkPartyAnnihilation__Fv", "ccSetDirc__FPffi"):
            m.hooks.pop(sym(n), None)
        m.store(sym("worldman"), 4, WORLDMAN)
        self.p_hrct = sym("hitResultCharType")
        self.p_open = sym("aiOpenDirc")
        self.p_gho = sym("ghoFlag")
        self.cmnd = [(g.cmnd_pc, sym("cmndPcLast")), (g.cmnd_ene, sym("cmndEneLast")),
                     (g.cmnd_obj, sym("cmndObjLast"))]
        self.wlog = []
        self.q = {}
        self.off = [0, 0, 0, 0]
        self.model = True
        self.last_name = ""
        # a navigation run's map ({(x, y): cell}): the world's queries are
        # answered from it and the answers kept (`rec`) for the probe
        self.geo = None
        self.rec = {}
        self.info = [0, 0, 0]
        self.heap = HEAP
        self.sizes = {}
        self.saved = {}

        def vec(a):
            return [m.load(a + 4 * i, 4) for i in range(4)]

        def put_vec(a, v):
            for i in range(4):
                m.store(a + 4 * i, 4, v[i] & 0xFFFFFFFF)

        self.vec = vec
        self.put_vec = put_vec
        name = self.ai.name

        def pop(k, d, fn=None):
            if self.geo is not None and fn is not None:
                r = fn()
                self.rec.setdefault(k, []).append(r)
                return r
            q = self.q.get(k)
            return q.pop(0) if q else d

        def w2p(mm, out, inp, *_):
            v = vec(inp)
            self.wlog.append(["w2p", v])
            put_vec(out, [eemu.f_add(v[i], self.off[i]) for i in range(4)])
            return 0

        def p2w(mm, out, inp, *_):
            v = vec(inp)
            self.wlog.append(["p2w", v])
            put_vec(out, [eemu.f_sub(v[i], self.off[i]) for i in range(4)])
            return 0

        def land(mm, p, mask, *_):
            self.wlog.append(["land", vec(p), mask & 0xFFFFFFFF])
            r = pop("land", 0, lambda: 0)
            mm.f[0] = r
            return r

        def attr(mm, *_):
            self.wlog.append(["attr"])
            return pop("attr", 0, lambda: 0)

        def line(kind):
            def h(mm, a, b, mask, *_):
                ra = mm.r[31] & 0xFFFFFFFF
                if kind == 0 and not FRONT[0] <= ra < FRONT[1]:
                    if self.geo is not None:
                        r = self.geo_line(vec(a), vec(b))
                        self.rec.setdefault("hit", []).append(r)
                    else:
                        r = self.ai.hit.pop(0) if self.ai.hit else F_M1
                    self.ai.calls.append(["ccHitCheckLM", vec(a), vec(b), mask & 0xFFFFFFFF])
                    mm.f[0] = r
                    return r
                self.wlog.append(["line", vec(a), vec(b), mask & 0xFFFFFFFF, kind])
                r = pop("line", F_M1, lambda: self.geo_line(vec(a), vec(b)))
                mm.f[0] = r
                return r
            return h

        def who_hit(h):
            return name(h - 0x1A0)

        def collide(mm, h, *_):
            self.wlog.append(["collide", who_hit(h), m.load(h, 4), m.load(h + 4, 4), m.load(h + 8, 4),
                              m.load(h + 0x14, 4), m.load(h + 0x18, 4), vec(h + 0x20)])
            ret, off, at, kind = pop("push", (0, [0, 0, 0, 0], 0, 0), lambda: self.geo_push(h))
            put_vec(h + 0x30, off)
            m.store(h + 0x40, 4, at)
            m.store(self.p_hrct, 4, kind)
            return ret & 0xFFFFFFFF

        def hit_switch(on):
            def h(mm, hh, *_):
                self.wlog.append(["hit_switch", who_hit(hh), on])
                m.store(hh, 4, on)
                return 0
            return h

        def cstr(a):
            out = b""
            while True:
                c = m.load(a, 1)
                if c == 0:
                    return out.decode("latin-1")
                out += bytes([c])
                a += 1

        def chunk(mm, st, nm, *_):
            self.last_name = cstr(nm)
            return CHUNK

        def who_anm(a):
            return name(self.va((a - ANM) // 0x200))

        def set_anm(mm, a, *_):
            self.wlog.append(["anim_set", who_anm(a), self.last_name])
            return 0

        def forward(mm, a, step, *_):
            self.wlog.append(["anim_forward", who_anm(a), step & 0xFFFF])
            return pop("fwd", 0) & 0xFFFFFFFF

        def notes(mm, a, *_):
            # NoteProcess (main 0x00152210) hands each note to
            # funcNoteProcess, here ccFellowCheckNote, run natively; the
            # nested call leaves the caller's registers as they were
            self.wlog.append(["anim_notes", who_anm(a)])
            for ev, param in pop("notes", []):
                m.store(NOTE + 4, 4, ev)
                m.store(NOTE + 8, 4, param)
                nested_call(mm, sym("ccFellowCheckNote__FP9ccAnmNote"), (NOTE,))
            return 0

        def trans_mode(mm, *_):
            self.wlog.append(["trans_mode"])
            return pop("trans", 0)

        def trans_center(mm, wm, out, *_):
            self.wlog.append(["trans_center"])
            put_vec(out, pop("center", [0, 0, 0, 0]))
            return 0

        def event_area(mm, *_):
            self.wlog.append(["event_area"])
            return pop("event", 0)

        def draw(mm, ch, *_):
            self.wlog.append(["draw", name(ch)])
            return pop("draw", 0)

        rec = g.record

        def out_vec(fn):
            def h(mm, a, *_):
                g.calls.append((fn, vec(a)))
                return 0
            return h

        def set_matrix(mm, anm, pos, dirc, *_):
            g.calls.append(("SetMatrix", vec(pos), vec(dirc)))
            return 0

        def paw(mm, a, *_):
            g.calls.append(("ccEffPawSmoke", g.name(a), mm.f[12]))
            return 0

        def delete_cmnd(mm, ch, *_):
            # ccDeleteCmnd (0x00519700): off the list of its kind
            ty = m.load(m.load(ch, 4) + 8, 4)
            k = 0 if ty & 7 else 1 if ty & 0xE0 else 2
            listed = any(ch in self.walk(root) for root, _ in self.cmnd)
            if not listed:
                return 0
            root, last = self.cmnd[k]
            prev, c = 0, m.load(root, 4)
            while c and c != ch:
                prev, c = c, m.load(c + 0xBC, 4)
            if c:
                nxt = m.load(c + 0xBC, 4)
                if prev:
                    m.store(prev + 0xBC, 4, nxt)
                else:
                    m.store(root, 4, nxt)
                if m.load(last, 4) == c:
                    m.store(last, 4, prev)
            g.calls.append(("ccDeleteCmnd", g.name(ch)))
            return 0

        def skill_req(mm, a, b, c, *_):
            self.ai.calls.append(["ccSkillRequest", name(a), name(b), s32(c)])
            if self.model:
                sid, st = s16(m.load(a + 0x7C, 2)), s16(m.load(a + 0x7E, 2))
                if s32(c) == 0:
                    if sid == 1 and st != 0:
                        m.store(a + 0x7E, 2, 0)
                else:
                    m.store(a + 0x7C, 2, c & 0xFFFF)
                    m.store(a + 0x7E, 2, 1)
                    m.store(a + 0x78, 4, b)
            return 0

        def face_talk(mm, p, *_):
            ai = P.AIS + 0x400 * self.me
            self.ai.calls.append(["FaceTalk", name(self.va(self.me)), s16(m.load(ai + volume.ai_at(0x9A), 2))])
            return 0

        def chat_attack(mm, a, b, c, d):
            g.calls.append(("ChatMessageAttack", g.name(a), g.name(b), s32(c), s32(d)))
            return 0

        entry_va = sym("EntryAffect__6ccCharFP6ccCharssss")

        def entry_affect(mm, this, ch, t, p0):
            # ccChar::EntryAffect (0x0056b020): recorded, then run for real
            # with the character's affectFunc (ccFellow::Influence,
            # Influence, ccEnemyInfluence)
            p1, p2 = mm.r[8], mm.r[9]
            g.calls.append(("EntryAffect", g.name(this), g.name(ch), s16(t), s16(p0), s16(p1), s16(p2)))
            h = mm.hooks.pop(entry_va)
            # Kite's Influence is only ever Kite's: the player global is he
            player = sym("plw") + 0x20
            was = mm.load(player, 4)
            if mm.load(this + 0x94, 4) == sym("Influence__FP6ccChar"):
                mm.store(player, 4, this)
            try:
                nested_call(mm, entry_va, (this, ch, t, p0, p1, p2))
            finally:
                mm.hooks[entry_va] = h
                mm.store(player, 4, was)
            return 0

        def clear_effect(mm, a, *_):
            g.calls.append(("ClearConditionEffect", g.name(a)))
            mm.store(a + 0x30, 4, 0xFFFFFFFF)
            return 0

        hooks = {
            "ccTransPosW2P__FPfPf": w2p, "ccTransPosP2W__FPfPf": p2w, "ccLandHitCheck__FPfUi": land,
            "checkHitResultAttlibute__Fv": attr, "ccHitCheckLM__FPfPfUi": line(0), "ccHitCheckLM2__FPfPfUi": line(1),
            "CollisionDetection__9ccCharHitFv": collide, "HitEnable__9ccCharHitFv": hit_switch(1),
            "HitDisable__9ccCharHitFv": hit_switch(0), "GetChunkAdrsF__8ccStreamFPCci": chunk,
            "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm, "_AnimateForward__5ccAnmFUi": forward,
            "NoteProcess__5ccAnmFv": notes, "GetTransMode__9WORLD_MANFv": trans_mode,
            "GetTransCenter__9WORLD_MANFPf": trans_center, "CheckEventArea__9WORLD_MANFv": event_area,
            "Draw__6ccCharFv": draw,
            "EquipWeapon__9ccSpcCharFv": lambda mm, a, *_: rec("EquipWeapon", a),
            "DeleteWeaponCCS__9ccSpcCharFv": lambda mm, a, *_: rec("DeleteWeaponCCS", a),
            "effLevelUp__FP6ccChar": lambda mm, a, *_: rec("effLevelUp", a),
            "effOpenBox__FPf": out_vec("effOpenBox"),
            "SetMatrix_PosRotZYX__7ccCoordFPfPf": set_matrix,
            "ArmsEffect__9ccSpcCharFv": lambda mm, a, *_: rec("ArmsEffect", a),
            "ClearArmsEffect__9ccSpcCharFv": lambda mm, a, *_: rec("ClearArmsEffect", a),
            "SetArmsEffectColor__9ccSpcCharFi": lambda mm, a, b, *_: rec("SetArmsEffectColor", a, b),
            "StartArmsEffect__9ccSpcCharFi": lambda mm, a, b, *_: rec("StartArmsEffect", a, b),
            "effSkillStart__FP6ccChariii": lambda mm, a, b, c, d: rec("effSkillStart", a, b, c, d),
            "effTransfer__FP6ccChar": lambda mm, a, *_: rec("effTransfer", a),
            "effWarpTransfer__FP6ccChar": lambda mm, a, *_: rec("effWarpTransfer", a),
            "ccSeSetParamSPC__FUiP6ccChar": lambda mm, a, b, *_: rec("ccSeSetParamSPC", a, b),
            "ccEffPawSmoke__FP6ccCharf": paw,
            "ccDeleteCmnd__FP6ccChar": delete_cmnd,
            "ccSkillRequest__FP6ccCharP6ccChari": skill_req,
            "ccSetDirc__FPffi": face_talk,
            "ChatMessageAttack__4ccAIFP6ccCharii": chat_attack,
            # EntryAffect and what the affect functions call: the bus runs
            # for real, ccSkillCheck answers the case's constant
            "EntryAffect__6ccCharFP6ccCharssss": entry_affect,
            "ccHitMarkDisp__FP6ccCharP6ccChar": lambda mm, a, b, *_: rec("ccHitMarkDisp", a, b),
            "DamageActuate__8ccPlayerFi": lambda mm, a, b, *_: rec("DamageActuate", b),
            "SetPanelBure__10ccMenuCtrlFis": lambda mm, a, b, c, *_: rec("SetPanelBure", b, s16(c)),
            "ccSkillCheck__FP6ccChar": lambda mm, *_: self.running & 0xFFFFFFFF,
            "ChatMessageDamage__4ccAIFi": lambda mm, a, b, *_: rec("ChatMessageDamage", a, b),
            "ChatMessageResurrectPlz__4ccAIFv": lambda mm, a, *_: rec("ChatMessageResurrectPlz", a),
            "AffectMessages__4ccAIFiP6ccChari": lambda mm, a, b, c, d: rec("AffectMessages", a, b, c, d),
            "Greeting__4ccAIFP6ccChari": lambda mm, a, b, c, *_: rec("Greeting", a, b, c),
            "effAfterDrain__FP6ccChari": lambda mm, a, b, *_: rec("effAfterDrain", a, b),
            "selectTarget__7ccEnemyFv": lambda mm, a, *_: rec("selectTarget", a),
            "effResistantShield__FP6ccCharii": lambda mm, a, b, c, *_: rec("effResistantShield", a, b, s32(c)),
            "ClearConditionEffect__6ccCharFv": clear_effect,
        }
        self.running = 0
        self.funcs = {1: sym("ccEnemyInfluence__FP6ccChar"), 2: sym("Influence__8ccFellowFv"),
                      3: sym("Influence__FP6ccChar")}
        for n, f in hooks.items():
            m.hooks[sym(n)] = f
        if volume.NAME != "infection":      # the AI's record of the hit (unnamed; Mutation 0x5c1a60)
            m.hooks[volume.callee("Influence__FP6ccChar", 0)] = lambda mm, a, b, *_: rec("NoteHit", a, b)
        self.me = 0

    # a navigation run's world -----------------------------------------------------------
    def walkable(self, x, y):
        return self.geo.get((math.floor(x / 300.0), math.floor(y / 300.0)), 0) not in (0, 4)

    def geo_line(self, a, b):
        """ccHitCheckLM / LM2 over the map: -1.0 when the way from a to b
        (on the ground) crosses no wall cell (0 or 4), else how far along
        it the first one is."""
        ax, ay, bx, by = ff(a[0]), ff(a[1]), ff(b[0]), ff(b[1])
        n = max(1, int(math.hypot(bx - ax, by - ay) / 5.0))
        for i in range(n + 1):
            t = i / n
            if not self.walkable(ax + (bx - ax) * t, ay + (by - ay) * t):
                return fl(t)
        return F_M1

    def geo_line_cells(self, cells, a, b):
        """geo_line over `cells` between two points (floats)."""
        saved, self.geo = self.geo, cells
        try:
            return self.geo_line([fl(a[0]), fl(a[1])], [fl(b[0]), fl(b[1])])
        finally:
            self.geo = saved

    def geo_push(self, h):
        """CollisionDetection: the body pushed out of the first other body
        it overlaps (each 15 across), a party member's with
        hitResultCharType 2."""
        m = self.m
        me = (h - 0x1A0 - self.va(0)) // (self.va(1) - self.va(0))
        p = [ff(x) for x in self.vec(h + 0x20)]
        r = ff(m.load(h + 0x14, 4))
        for i, ch in enumerate(self.case["w"]["chars"]):
            if i == me:
                continue
            q = [ff(x) for x in self.vec(self.va(i) + 0x40)]
            dx, dy = p[0] - q[0], p[1] - q[1]
            d = math.hypot(dx, dy)
            if 1e-3 < d < r + 15.0:
                k = (r + 15.0 - d) / d
                return (1, [fl(dx * k), fl(dy * k), 0, 0], 0, 2 if ch.type & 7 else 0x20)
        return (0, [0, 0, 0, 0], 0, 0)

    def malloc(self, mm, n, *_):
        a = self.heap
        self.heap += (max(n, 1) + 15) & ~15
        mm.mem[a:a + n] = bytes(n)
        self.sizes[a] = n
        return a

    def navi_on(self, nav):
        """A navigation run: the path finding (PathFindingInDungeon and
        what it calls), FollowBeacon and CheckGoalBeaconPos run natively;
        Get2DMapInfo answers the room window, Get2DMapPtr the map; ccMalloc
        is a bump allocator; buf, buf2 and bufFlag start empty and
        SetPathFindingMap makes them, natively."""
        m, sym, g = self.m, self.sym, self.g
        own = {
            "ccMalloc__FUi": self.malloc,
            "ccFree__FPv": lambda mm, *_: 0,
            "Get2DMapInfo__9WORLD_MANFPi": self.map_info,
            "Get2DMapPtr__9WORLD_MANFv": self.map_ptr,
        }
        native = ("PathFindingInDungeon__6ccNaviFPfPf", "FollowBeacon__4ccAIFv", "CheckGoalBeaconPos__4ccAIFPf")
        self.saved = {n: m.hooks.get(sym(n)) for n in list(own) + list(native)}
        for n in native:
            m.hooks.pop(sym(n), None)
        for n, f in own.items():
            m.hooks[sym(n)] = f
        self.geo = nav["cells"]
        self.rec = {}
        self.info = list(nav["info"])
        self.heap = HEAP
        self.sizes = {}
        m.store(WORLDMAN + 8, 4, 2 if nav["area"] == 2 else 1)
        g.put(sym("ccSpcManager") + 0xDC, "<i", nav["registry"])
        buf, buf2 = sym("buf"), sym("buf2")
        m.mem[buf:buf + 0x10000] = bytes(0x10000)
        m.mem[buf2:buf2 + 0x20000] = bytes(0x20000)
        g.put(sym("bufFlag"), "<i", 0)
        m.mem[MAP2D:MAP2D + 0x10000] = bytes(0x10000)
        for (x, y), v in nav["cells"].items():
            if 0 <= x < 256 and 0 <= y < 256:
                m.mem[MAP2D + x * 256 + y] = v
        for k, nv in nav["navi"].items():
            self.put_navi(P.AIS + 0x400 * k + NAVI, nv)
        if nav["made"]:
            m.call(sym("SetPathFindingMap__Fv"), (), 400_000_000)

    def navi_off(self):
        m, sym = self.m, self.sym
        for n, h in self.saved.items():
            if h is None:
                m.hooks.pop(sym(n), None)
            else:
                m.hooks[sym(n)] = h
        self.saved = {}
        self.geo = None
        m.store(WORLDMAN + 8, 4, 0)

    def map_info(self, mm, wm, p, *_):
        self.wlog.append(["Get2DMapInfo"])
        for i, v in enumerate(self.info):
            mm.store(p + 4 * i, 4, v & 0xFFFFFFFF)
        return 0

    def map_ptr(self, mm, *_):
        self.wlog.append(["Get2DMapPtr"])
        return MAP2D

    def put_navi(self, n, nv):
        g, m = self.g, self.m
        g.put(n, "<4I", *nv["goal_pos"])
        g.put(n + 0x10, "<hhhh", nv["start_x"], nv["start_y"], nv["goal_x"], nv["goal_y"])
        g.put(n + 0x18, "<h", nv["name"])
        g.put(n + 0x1C, "<hh", nv["step"], nv["landmark"])
        g.put(n + 0x20, "<II", nv["dist"], nv["dirc"])
        g.put(n + 0x28, "<hhh", nv["map_x"], nv["map_y"], nv["map_s"])
        m.mem[n + 0x2E:n + 0x5E] = bytes(nv["route"])
        m.mem[n + 0x5E:n + 0x60] = bytes(nv["pad"])
        g.put(n + 0x60, "<i", nv["beacon_num"])
        m.store(n + 0x64, 4, 0)
        m.mem[n + 0x68:n + 0x70] = bytes(8)

    def read_navi(self, key):
        """The AI's ccNavi as the navigation harness reads it."""
        g, m = self.g, self.m
        n = P.AIS + 0x400 * key + NAVI
        out = list(g.get(n, "<4I"))
        out += list(g.get(n + 0x10, "<hhhhh")) + list(g.get(n + 0x1C, "<hh"))
        out += list(g.get(n + 0x20, "<II")) + list(g.get(n + 0x28, "<hhh"))
        out.append(NV.hexs(m.mem[n + 0x2E:n + 0x5E]))
        out += [m.mem[n + 0x5E], m.mem[n + 0x5F], g.get(n + 0x60, "<i")[0]]
        p = m.load(n + 0x64, 4)
        out.append("-" if p == 0 else NV.hexs(m.mem[p:p + 2 * (self.sizes.get(p, 0) // 4)]))
        return out

    def read_path(self):
        """bufFlag and the CRC-32 of buf and buf2."""
        m, sym = self.m, self.sym
        buf, buf2 = sym("buf"), sym("buf2")
        return [self.g.get(sym("bufFlag"), "<i")[0], zlib.crc32(bytes(m.mem[buf:buf + 0x10000])),
                zlib.crc32(bytes(m.mem[buf2:buf2 + 0x20000]))]

    # the lists ---------------------------------------------------------------------------
    def walk(self, root):
        out, c, n = [], self.m.load(root, 4), 0
        while c and n < 64:
            out.append(c)
            c = self.m.load(c + 0xBC, 4)
            n += 1
        return out

    def lists(self):
        return [[self.ai.index(c) for c in self.walk(root)] for root, _ in self.cmnd]

    def put_lasts(self):
        for root, last in self.cmnd:
            w = self.walk(root)
            self.m.store(last, 4, w[-1] if w else 0)

    # memory ------------------------------------------------------------------------------
    def put_fel(self, i, fel):
        g, m = self.g, self.m
        va = self.va(i)
        for n, off, f in FEL_LAYOUT:
            v = fel[n]
            if f == "M":
                m.store(va + off, 4, FELLOW_ANIM_TBL + 0x60 * (v - 1) if v > 0 else 0)
            elif f == "C":
                m.store(va + off, 4, self.va(v) if v >= 0 else 0)
            elif f == "<4I":
                g.put(va + off, f, *[x & 0xFFFFFFFF for x in v])
            else:
                g.put(va + off, f, v & 0xFFFFFFFF if f in ("<I",) else v)
        m.store(va + 0xD4, 4, ANM + 0x200 * i)
        m.store(va + 0xCC, 4, STREAM)
        anm = ANM + 0x200 * i
        m.mem[anm:anm + 0x110] = bytes(0x110)
        m.store(anm + 0xAC, 4, 1)

    def read_fel(self, i):
        g, m = self.g, self.m
        va = self.va(i)
        out = []
        for n, off, f in FEL_LAYOUT:
            if f == "M":
                p = m.load(va + off, 4)
                out.append((p - FELLOW_ANIM_TBL) // 0x60 + 1 if p else 0)
            elif f == "C":
                out.append(self.ai.index(m.load(va + off, 4)))
            elif f == "<4I":
                out += list(g.get(va + off, f))
            else:
                out.append(g.get(va + off, f)[0])
        return out

    def put_case(self, w, c):
        """A world (party harness), the fellow state of every character and
        the script."""
        ai = self.ai
        ai.put_world(w)
        self.case = c
        m, g = self.m, self.g
        for i, fel in enumerate(c["fel"]):
            # a foe's ccEntryObj lies under these offsets (its entry link at
            # +0x1c4): only the party's are written, a foe's are echoed
            ch = w["chars"][i]
            a = self.va(i)
            if ch.type & 7:
                self.put_fel(i, fel)
            else:
                # what affectEnemy reads of a foe (as test_battle_rs's affect
                # check lays it out)
                m.store(a + 0xE0, 1, (m.load(a + 0xE0, 1) & 0x80) | ch.spc_flags)
                g.put(a + 0x250, "<H", ch.enemy_flags)
                m.store(a + 0x238, 4, a + 8)
                m.store(a + 0x244, 4, ch.id)
                m.mem[a + 0x1D0:a + 0x1D0 + 0x64] = m.mem[a + 0x400:a + 0x400 + 0x64]
                # (ccEnemy.eParam covers what the decisions' harness reads as
                # distTg: the probe is sent what is there)
                ch.spc["dist_tg"] = m.load(a + 0x20C, 4)
            m.store(a + 0x94, 4, self.funcs.get(ch.affect_func, 0))
            g.put(a + 0x9C, "<h", ch.affect_type)
            m.store(a + 0xB0, 4, ch.affect_mask & 0xFFFFFFFF)
        self.put_lasts()
        env = c["env"]
        self.g.put_env(env)
        self.g.env = env
        self.m.store(self.p_gho, 4, c["gho"])
        self.m.store(WORLDMAN + 0x168, 4, c["warp"])
        self.m.store(self.p_open, 4, c["open"])
        self.off = list(c["off"])
        self.q = {k: list(v) for k, v in c["q"].items()}
        self.model = c["model"]
        self.wlog = []
        self.g.calls = []
        self.ai.calls = []
        self.g.names = {self.va(i): f"c{i}" for i in range(len(w["chars"]))}
        for k in w["ais"]:
            self.g.names[P.AIS + 0x400 * k] = "ai"
        self.running = c["running"]
        self.m.store(self.sym("ccMenu"), 4, TB.MENU if c["menu"] else 0)
        self.me = c["me"]
        self.ai.cur = f"c{c['me']}"

    def read_case(self, w, ret):
        out = self.ai.read_world(w, ret)
        g = self.g
        out["world"] = self.wlog
        out["out"] = [list(x) for x in g.calls]
        out["fel"] = [self.read_fel(i) if ch.type & 7 else ser_fel(self.case["fel"][i], True)
                      for i, ch in enumerate(w["chars"])]
        out["affect"] = [self.read_affect(i) for i in range(len(w["chars"]))]
        out["chars"] = self.pc.c.scene.read(w["chars"])
        out["lists"] = self.lists()
        out["globals"] = out["globals"] + [self.m.load(self.p_open, 4)]
        self.wlog = []
        g.calls = []
        self.ai.calls = []
        return out

    def read_affect(self, i):
        """The affect members, the flag byte at +0xe0 and a foe's flag word
        (+0x250)."""
        m, g = self.m, self.g
        a = self.va(i)
        person = self.ai.index(m.load(a + 0x98, 4))
        return ([person] + list(g.get(a + 0x9C, "<4h")) + list(g.get(a + 0xA8, "<hh")) + [m.load(a + 0xAC, 4),
                s32(m.load(a + 0xB0, 4))] + list(g.get(a + 0xB4, "<hh")) + [m.load(a + 0xB8, 4), m.load(a + 0xE0, 1),
                g.get(a + 0x250, "<H")[0]])

    def tick(self):
        """One frame of ccThAISystem (0x0058c850): the clock, then every
        waiting message sent (ccAISystem::SendMessage, natively)."""
        m = self.m
        m.store(P.AISYS, 4, (m.load(P.AISYS, 4) + 1) & 0xFFFFFFFF)
        for i in range(16):
            a = P.AISYS + 0xC + 0x18 * i
            if s16(m.load(a + 0xC, 2)) != 0:
                r = m.call(self.sym("SendMessage__10ccAISystemFP10ccAISysMsg"), (P.AISYS, a))
                if s32(r) == 1:
                    m.store(a, 4, 0xFFFFFFFF)
                    m.store(a + 0xC, 2, 0)


# serialising --------------------------------------------------------------------------------

def ser_fel(fel, as_list=False):
    out = []
    for n, off, f in FEL_LAYOUT:
        v = fel[n]
        if f == "<4I":
            out += [x & 0xFFFFFFFF for x in v]
        elif f == "<B":
            out.append(v & 0xFF)
        else:
            out.append(v & 0xFFFFFFFF if f == "<I" else v)
    return out if as_list else " ".join(str(x) for x in out)


def ser_case(c, b):
    w = c["w"]
    out = [P.ser_world(w, b)]
    out += [ser_fel(f) for f in c["fel"]]
    q = c["q"]
    out.append(" ".join(str(x & 0xFFFFFFFF) for x in c["off"]))
    for k in ("land", "attr", "line"):
        out.append(" ".join(str(x & 0xFFFFFFFF) for x in [len(q[k])] + q[k]))
    out.append(" ".join(str(x) for x in [len(q["push"])] + [v for p in q["push"] for v in
                                                              [p[0]] + [x & 0xFFFFFFFF for x in p[1]] + [p[2], p[3]]]))
    out.append(" ".join(str(x) for x in [len(q["fwd"])] + q["fwd"]))
    notes = [len(q["notes"])]
    for ns in q["notes"]:
        notes += [len(ns)] + [v for e in ns for v in e]
    out.append(" ".join(str(x) for x in notes))
    out.append(" ".join(str(x) for x in [len(q["trans"])] + q["trans"]))
    out.append(" ".join(str(x) for x in [len(q["center"])] + [x & 0xFFFFFFFF for v in q["center"] for x in v]))
    out.append(" ".join(str(x) for x in [len(q["event"])] + q["event"]))
    out.append(" ".join(str(x) for x in [len(q["draw"])] + q["draw"]))
    out.append(f"{c['gho']} {c['warp']} {c['open']}")
    out.append(T.ser_env(c["env"]))
    out.append(str(int(c["model"])))
    out.append(f"{c['running']} {int(c['menu'])}")
    return " ".join(out)


def request(fn, args, c, b, extra=""):
    return f"fel {fn} {len(args)} {' '.join(str(a) for a in args)} {ser_case(c, b)} {extra}".rstrip()


# random cases -------------------------------------------------------------------------------

def rnd_flags(rnd, ch, fel):
    """The word at +0xe0."""
    w = 0
    for bit, p in ((0, 0.2), (1, 0.7), (2, 0.3), (10, 0.05), (11, 0.3), (12, 0.1), (13, 0.05)):
        if rnd.random() < p:
            w |= 1 << bit
    w |= rnd.choice((0, 0, 0, 0, 1, 3, 2)) << 8           # weaponChangeSW
    if fel["move"]:
        w |= 8
    if fel["stop"]:
        w |= 0x10
    if fel["run"]:
        w |= 0x20
    if fel["ghost"]:
        w |= 0x40
    if getattr(ch, "no_death", False):
        w |= 0x80
    w |= (getattr(ch, "party_flag", 0) & 7) << 14
    return w


def rnd_fel(rnd, ch, n):
    fel = {}
    s = ch.spc
    fel["move"], fel["run"] = s["move_flag"], s["run_flag"]
    fel["stop"], fel["ghost"] = s["stop_flag"], s["ghost"]
    fel["flags"] = rnd_flags(rnd, ch, fel)
    fel["arms"] = rnd.choice((0, 0, 1, 2, -1))
    fel["act"] = s["act_num"]
    fel["act_old"] = rnd.choice((s["act_num"], s["act_num"], -1, rnd.randrange(0, 22)))
    fel["atk_anm"] = rnd.choice((0, 1, 5, 10, rnd.randrange(0, 40)))
    fel["anm_flag"] = ch.anm_flag
    fel["act_cnt"] = rnd.choice((0, 1, 20, 21, 22, 29, 30, 40, 41, 49, 50, 51, 69, 70, 71, 140, 141, rnd.randrange(-3, 200)))
    fel["react"] = rnd.choice((0, 449, 450, 451, 500, rnd.randrange(0, 460)))
    fel["stop_cnt"] = s["stop_cnt"] if rnd.random() < 0.8 else 32767
    fel["attack"] = rnd.randrange(0, 5)
    fel["lag"] = rnd.choice((0, 0, 0, 1, 3))
    fel["speed"] = fl(rnd.uniform(3, 14))
    fel["rate"] = rnd.choice((F_ONE, F_ONE, fl(rnd.uniform(0.3, 1.6))))
    fel["now_speed"] = s["now_speed"]
    fel["cloak"] = rnd.choice((F_ONE, fl(0.5), 0, fl(rnd.uniform(0, 1))))
    fel["cnt"] = rnd.choice((-2, -1, 0, 1, 17, 18, 29, 30, 49, 50, 77, 78, 90, rnd.randrange(-5, 100)))
    fel["cycle"] = s["cycle"]
    fel["consec"] = rnd.choice((0, 3, 10, rnd.randrange(0, 30)))
    fel["walk"] = s["walk_run_cnt"]
    fel["dirc"] = [0, 0, rnd_angle(rnd), rnd.choice((0, F_ONE))]
    fel["hit_attr"] = rnd.randrange(0, 1 << 16)
    fel["trans"] = fl(rnd.uniform(0, 1))
    fel["set_trans"] = fl(rnd.uniform(0, 1))
    fel["hsw"] = rnd.choice((0, 1, 1))
    fel["hmask"] = rnd.choice((0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFF8, rnd.getrandbits(32)))
    fel["hmask2"] = rnd.choice((0x40000001, 0))
    fel["htype"] = rnd.choice((2, 4, 7))
    fel["hradius"] = fl(rnd.uniform(5, 30))
    fel["hheight"] = fl(rnd.uniform(40, 120))
    fel["hpos"] = rnd_vec(rnd, 300)
    fel["hoff"] = rnd_vec(rnd, 5)
    fel["hattr"] = rnd.randrange(0, 256)
    fel["fbyte"] = rnd.getrandbits(8)
    fel["disp_wait"] = rnd.choice((0, 0, 0, 1, 2, 5))
    fel["atk_dellay"] = rnd.choice((0, 0, 1, 50, 90))
    fel["dist_tg"] = s["dist_tg"]
    fel["disk"] = rnd_vec(rnd, 50)
    fel["move_pos"] = rnd_vec(rnd, 10)
    fel["motion"] = rnd.randrange(1, 18)
    fel["target"] = s["target_char"]
    fel["skill_id"] = ch.skill_id
    fel["skill_status"] = ch.skill_status
    fel["pos"] = list(ch.pos)
    fel["pos_p"] = list(ch.pos_p)
    return fel


def sync_fel(ch, fel):
    """The party world's copies of what the fellow block also holds."""
    s = ch.spc
    w = fel["flags"]
    s["move_flag"], s["stop_flag"] = (w >> 3) & 1, (w >> 4) & 1
    s["run_flag"], s["ghost"] = (w >> 5) & 1, (w >> 6) & 1
    s["act_num"], s["target_char"] = fel["act"], fel["target"]
    s["now_speed"], s["cycle"], s["dist_tg"] = fel["now_speed"], fel["cycle"], fel["dist_tg"]
    s["stop_cnt"], s["walk_run_cnt"] = fel["stop_cnt"], fel["walk"]
    ch.skill_id, ch.skill_status, ch.anm_flag = fel["skill_id"], fel["skill_status"], fel["anm_flag"]
    ch.pos, ch.pos_p = list(fel["pos"]), list(fel["pos_p"])
    ch.no_death = bool(w & 0x80)
    ch.party_flag = (w >> 14) & 7


def affects(rnd, chars, plain=False):
    """Each character's affectFunc and last affect: ccFellow::Influence for
    a member (2), Influence for Kite (3), ccEnemyInfluence for an enemy
    (1), none for a boss; a foe's flag byte and word."""
    for ch in chars:
        if ch.type & 1:
            ch.affect_func = 3
        elif ch.type & 6:
            ch.affect_func = 2
        else:
            ch.affect_func = 0 if ch.type & 0x80 else 1
        ch.affect_type = 0 if plain else rnd.choice((0, 0, 0, 1, 13))
        ch.affect_mask = 0 if plain or rnd.random() < 0.85 else 1 << rnd.randrange(0, 21)
        if not ch.type & 7:
            ch.spc_flags = 0 if plain else rnd.getrandbits(8) & ~0x80
            ch.enemy_flags = 0 if plain else rnd.choice((0, 0, 0x40, rnd.getrandbits(9)))
            s = ch.spc
            s["move_flag"], s["stop_flag"] = (ch.spc_flags >> 3) & 1, (ch.spc_flags >> 4) & 1
            s["run_flag"], s["ghost"] = (ch.spc_flags >> 5) & 1, (ch.spc_flags >> 6) & 1


def rnd_queues(rnd, k=40, walls=0.3):
    line = [rnd.choice((F_M1, F_M1, fl(0.4), 0)) if rnd.random() < walls else F_M1 for _ in range(k)]
    push = []
    for _ in range(k):
        r = rnd.choice((0, 0, 0, 1, 2, 3))
        push.append((r, rnd_vec(rnd, 6, 1) if r else [0, 0, 0, 0], rnd.randrange(0, 16), rnd.choice((0, 2, 4, 6, 1))))
    notes = []
    for _ in range(k):
        ns = []
        for _ in range(rnd.choice((0, 0, 0, 1, 1, 2))):
            ns.append((rnd.choice((0x8005, 0x8005, 1, 2, 3, 0x8002, 0x8003, 0x8004, 0x8001, 7)),
                       rnd.choice((0, 3, 17, rnd.randrange(0, 1000)))))
        notes.append(ns)
    return {
        "land": [rnd.choice((0, fl(rnd.uniform(-30, 30)))) for _ in range(k)],
        "attr": [rnd.randrange(0, 1 << 20) for _ in range(k)],
        "line": line,
        "push": push,
        "fwd": [rnd.choice((0, 0, 0, 1)) for _ in range(k)],
        "notes": notes,
        "trans": [int(rnd.random() < 0.2) for _ in range(k)],
        "center": [rnd_vec(rnd, 200) for _ in range(k)],
        "event": [int(rnd.random() < 0.3) for _ in range(k)],
        "draw": [int(rnd.random() < 0.8) for _ in range(k)],
    }


class FellowChecks:
    def __init__(self, checks):
        self.c = checks
        self.pc = P.PartyChecks(checks)
        self.b = checks.b
        self.fg = FellowGame(self.pc)
        self._floors = None

    def floor_wins(self):
        """The room windows of the tutorial dungeon's floors and two random
        dungeons', as the navigation harness takes them."""
        if self._floors is None:
            import dungeon
            d = dungeon.Data(T.ELF)
            floors = list(dungeon.Edited(d, 14).floors)
            floors += list(dungeon.Generator(d, 3996388677, 0, 4, 9).generate().floors)
            floors += list(dungeon.Generator(d, 1814669918, 3, 3, 7).generate().floors)
            self._floors = [NV.windows_of(f) for f in floors]
        return self._floors

    def case(self, rnd, tweak=None, **kw):
        """A world with a fellow (an AI, type 4) to run; its fellow state and
        a script. `tweak(rnd, c)` steers it."""
        pc = self.pc
        while True:
            w = pc.world(rnd, **kw)
            mes = [k for k in w["ais"] if w["chars"][k].type & 5 == 4]
            if mes:
                break
        me = rnd.choice(mes)
        chars = w["chars"]
        for ch in chars:                        # spread out: the following works in hundreds
            if rnd.random() < 0.8:
                ch.pos_p = rnd_vec(rnd, 500)
                ch.pos = list(ch.pos_p) if rnd.random() < 0.5 else rnd_vec(rnd, 500)
        k = w["party"]["members"][0]
        if k >= 0 and k != me and rnd.random() < 0.6:        # Kite nearby
            p = chars[me].pos_p
            chars[k].pos_p = [fl(ff(p[0]) + rnd.uniform(-400, 400)), fl(ff(p[1]) + rnd.uniform(-400, 400)),
                              p[2], F_ONE]
            chars[k].pos = list(chars[k].pos_p)
        fel = [rnd_fel(rnd, ch, len(chars)) for ch in chars]
        a = w["ais"][me]
        a[P.AI_AT["distPl"]] = rnd.choice((fl(rnd.uniform(0, 400)), fl(rnd.choice((119.9, 120.0, 150.0, 200.0, 260.0))),
                                           fl(rnd.uniform(0, 4000))))
        a[P.AI_AT["detourCnt"]] = rnd.choice((0, 0, -1, 1, 5, 6, 30, 60))
        a[P.AI_AT["distTg"]] = rnd.choice((F_M1, fl(rnd.uniform(0, 300))))
        a[P.AI_AT["param"]] = rnd.choice((chars[me].id, chars[me].id, rnd.randrange(0, 18)))
        a[0] &= ~0x4 if rnd.random() < 0.9 else ~0                     # not talking, mostly
        env = self.b.Env(plcol=rnd.choice((0, 1)), menu_flag=rnd.choice((0, 0, 1)), in_battle=w["game"]["in_battle"],
                         count=rnd.choice((0, 59, 60, rnd.getrandbits(32))), menu_type=w["game"]["menu_type"],
                         sp_regene_speed=0, area=w["game"]["area"])
        c = {"w": w, "me": me, "fel": fel, "env": env, "gho": int(rnd.random() < 0.05),
             "warp": int(rnd.random() < 0.3), "open": rnd_angle(rnd) if rnd.random() < 0.5 else 0,
             "off": [0, 0, 0, 0] if rnd.random() < 0.5 else rnd_vec(rnd, 100, 10)[:3] + [0],
             "q": rnd_queues(rnd), "model": rnd.random() < 0.7,
             "running": rnd.choice((0, 0, 1, 1, 2)), "menu": True}
        affects(rnd, chars)
        if tweak:
            tweak(rnd, c)
        for ch, f in zip(chars, c["fel"]):
            sync_fel(ch, f)
        c["env"].in_battle, c["env"].area = w["game"]["in_battle"], w["game"]["area"]
        c["env"].menu_type = w["game"]["menu_type"]
        return c

    def run(self, c, fn, args, flt=None):
        fg = self.fg
        fg.put_case(c["w"], c)
        if flt is not None:
            fg.m.f[12] = flt
        ret = fg.m.call(fg.sym(fn), args)
        return ret

    def answer(self, c, ret):
        out = self.fg.read_case(c["w"], ret)
        self.c.scene.restore()
        return out


def me_va(fc, c):
    return fc.fg.va(c["me"])


def me_ai(c):
    return P.AIS + 0x400 * c["me"]


def fel_of(c):
    return c["fel"][c["me"]]


def make_checks(fc):
    b = fc.b
    va = fc.fg.va

    def steer_ai(rnd, c, **fields):
        a = c["w"]["ais"][c["me"]]
        for k, v in fields.items():
            a[P.AI_AT[k]] = v

    def calm(rnd, c, p=0.7):
        ch = c["w"]["chars"][c["me"]]
        if rnd.random() < p:
            dead = ch.cond[0] if rnd.random() < 0.2 else 0
            ch.cond = [0] * 16
            ch.cond[0] = dead

    def area(rnd, c, v=None):
        c["w"]["game"]["area"] = v if v is not None else rnd.choice((1, 1, 2, 2, 0))

    # ccAI::SetDircZ -------------------------------------------------------------------
    def set_dirc_z(rnd):
        c = fc.case(rnd)
        dd = rnd.randrange(0, 65536)
        ret = fc.run(c, "SetDircZ__4ccAIFUs", (me_ai(c), dd))
        return request("SetDircZ", [c["me"], dd], c, b), fc.answer(c, 0), "SetDircZ"

    # CheckFrontObstacle(F) ----------------------------------------------------------------
    def cfo(rnd):
        c = fc.case(rnd)
        d = rnd.choice((0xC479C000, 0xC479C000, rnd_angle(rnd)))
        ret = fc.run(c, "CheckFrontObstacle__4ccAIFf", (me_ai(c),), d)
        return request("CheckFrontObstacle", [c["me"], d], c, b), fc.answer(c, s32(ret)), "CheckFrontObstacle"

    def probes(rnd):
        """The lines CheckFrontObstacleF draws: blocked straight ahead
        (mostly), then each probe's way out and on to the goal."""
        out = [rnd.choice((0, fl(0.3))) if rnd.random() < 0.85 else F_M1]
        pattern = rnd.choice(("open", "blocked", "mixed", "late"))
        n = rnd.randrange(0, 6)
        for k in range(6):
            if pattern == "open":
                way = rnd.random() < 0.7
            elif pattern == "blocked":
                way = rnd.random() < 0.05
            elif pattern == "late":
                way = k >= n
            else:
                way = rnd.random() < 0.5
            if way:
                out += [F_M1, rnd.choice((F_M1, F_M1, 0))]
            else:
                out.append(0)
        return out

    def cfo_f(rnd):
        def tw(rnd, c):
            c["q"]["line"] = probes(rnd)
        c = fc.case(rnd, tw)
        goal = rnd_vec(rnd, 600) if rnd.random() < 0.7 else list(c["w"]["chars"][c["me"]].pos)
        fc.fg.put_vec(SCRATCH, goal)
        fc.fg.put_case(c["w"], c)
        ret = fc.fg.m.call(fc.fg.sym("CheckFrontObstacleF__4ccAIFPf"), (me_ai(c), SCRATCH))
        return (request("CheckFrontObstacleF", [c["me"]] + goal, c, b), fc.answer(c, s32(ret)),
                "CheckFrontObstacleF")

    # the following ------------------------------------------------------------------------
    def near(p, r, rnd):
        a = rnd.uniform(-math.pi, math.pi)
        return [fl(ff(p[0]) + r * math.sin(a)), fl(ff(p[1]) - r * math.cos(a)), p[2], F_ONE]

    def put_at(f, p):
        f["pos_p"] = list(p)
        f["pos"] = list(p)

    def follow(fn, mangled, target=False):
        def check(rnd):
            chosen = {}

            def tw(rnd, c):
                w = c["w"]
                me = c["me"]
                area(rnd, c)
                calm(rnd, c, 0.6)
                f = fel_of(c)
                f["act"] = rnd.choice((0, 0, 2, 4, 5, 6, 7, 15, 16, 9))
                f["skill_id"], f["skill_status"] = rnd.choice(((0, 0), (0, 0), (0, 0), (1, 0), (1, 2), (5, 1)))
                if rnd.random() < 0.5:
                    f["flags"] = (f["flags"] & ~0x28) | rnd.choice((0, 8, 0x28))
                q = c["q"]
                q["line"] = [rnd.choice((F_M1, F_M1, 0, fl(0.5))) for _ in range(20)]
                w["goal"] = [rnd.choice((0, 1, 2, -1)) for _ in range(6)]
                k = w["party"]["members"][0]
                kf = c["fel"][k] if k >= 0 and k != me else None
                if kf is not None:
                    kf["flags"] = (kf["flags"] & ~0x28) | rnd.choice((0, 8, 0x28, 8))
                a = w["ais"][me]
                s_ = rnd.random()
                if s_ < 0.35 and kf is not None:
                    # Kite at the origin of the player's frame; the member about
                    # the point 200 out along his heading
                    put_at(kf, [0, 0, kf["pos_p"][2], F_ONE])
                    h = ff(kf["dirc"][2])
                    ids = w["party"]["ids"]
                    slot = ids.index(w["chars"][me].id) if w["chars"][me].id in ids else -1
                    off = (0x6E, 0, 20480, 45056, 32768)[slot + 1]
                    ang = h + off * math.pi / 32768.0
                    g = [fl(200 * math.sin(ang)), fl(-200 * math.cos(ang)), 0, F_ONE]
                    put_at(f, near(g, rnd.choice((5, 15, 30, 45, 60, 75, 150, 300)), rnd))
                    f["flags"] |= 8
                    a[P.AI_AT["distPl"]] = fl(rnd.choice((rnd.uniform(0, 400), 100.0, 119.0, 121.0, 250.0)))
                elif s_ < 0.6:
                    # a field's detour
                    w["game"]["area"] = 1
                    q["line"] = probes(rnd) + probes(rnd)
                    if rnd.random() < 0.4:                  # only the widest ways open
                        q["line"] = [0, 0, 0, 0, 0] + [F_M1, rnd.choice((F_M1, 0))] * 2
                    a[P.AI_AT["detourCnt"]] = rnd.choice((0, 0, -1, 1, 5, 6, 30))
                elif s_ < 0.8:
                    # a dungeon's wall
                    w["game"]["area"] = 2
                    q["line"] = [rnd.choice((0, 0, F_M1))] + q["line"]
                if kf is not None and rnd.random() < 0.4:
                    put_at(kf, near(f["pos_p"], rnd.choice((100, 300, 600, 900, 1500, 2500, 4000)), rnd))
                if target:
                    a[P.AI_AT["mode"]] = rnd.choice((2, 2, 3, 1, 4))
                    w["chars"][me].width = rnd.choice((0, fl(rnd.uniform(0, 20))))
                    t = rnd.choice(list(range(len(w["chars"]))) + [-1])
                    chosen["t"] = t
                    if t >= 0 and t != me:
                        pr = fc.pc.pools
                        prow = a[P.AI_AT["param"]]
                        d = rnd.choice((20, 50, 90, 110, 130, 160, 300, 900, 1850, 2500))
                        tf = c["fel"][t]
                        put_at(tf, near(f["pos_p"], d, rnd))
                        tf["pos"] = [eemu.f_sub(v, o) for v, o in zip(tf["pos_p"], c["off"])]
                        w["chars"][t].cond[0] = rnd.choice((0, 0, 0, 1, 2))
            c = fc.case(rnd, tw)
            args, gargs = [c["me"]], [me_ai(c)]
            if target:
                t = chosen["t"]
                args.append(t)
                gargs.append(va(t) if t >= 0 else 0)
            fc.run(c, mangled, tuple(gargs))
            return request(fn, args, c, b), fc.answer(c, 0), fn
        return check

    # ccFellow::Move, ccSpcChar::HitCheck ----------------------------------------------------
    def move(rnd):
        def tw(rnd, c):
            f = fel_of(c)
            f["skill_status"] = rnd.choice((0, 2, 3, 1, 6))
            f["skill_id"] = rnd.choice((0, 1, 1, 5))
            c["w"]["chars"][c["me"]].cond[0] = rnd.choice((0, 0, 0, 2, 3, 1, 4))
        c = fc.case(rnd, tw)
        fc.run(c, "Move__8ccFellowFv", (me_va(fc, c),))
        return request("Move", [c["me"]], c, b), fc.answer(c, 0), "Move"

    def hit_check(rnd):
        def tw(rnd, c):
            f = fel_of(c)
            f["act"] = rnd.choice((0, 5, 6, 14, 2))
            c["w"]["chars"][c["me"]].cond[0] = rnd.choice((0, 0, 0, 4, 2))
            c["w"]["chars"][c["me"]].type = rnd.choice((4, 6, 4, 5))
            q = c["q"]
            q["line"] = [rnd.choice((F_M1, F_M1, F_M1, 0, fl(0.5))) for _ in range(10)]
            if rnd.random() < 0.5:
                f["pos_p"] = rnd_vec(rnd, 7000) if rnd.random() < 0.5 else rnd_vec(rnd, 400)
            if rnd.random() < 0.5:
                f["move_pos"] = rnd_vec(rnd, 1.5)
            c["w"]["ais"][c["me"]][0] = (c["w"]["ais"][c["me"]][0] & ~1) | int(rnd.random() < 0.3)
        c = fc.case(rnd, tw)
        a = me_va(fc, c)
        ret = fc.run(c, "HitCheck__9ccSpcCharFPf", (a, a + 0x230))
        return request("HitCheck", [c["me"]], c, b), fc.answer(c, s32(ret)), "HitCheck"

    def arms(c):
        """The member's armsRange (its AI's spcAIParam row)."""
        row = c["w"]["ais"][c["me"]][P.AI_AT["param"]]
        return ff(fc.c.game.prog.u32(P.SPC_AI_PARAM + 0x24 * row + 0xC))

    def in_reach(rnd, c, t, exact=0.15):
        """The target about the member: within armsRange, beyond, or at
        exactly -1.0 (DistanceToTarget's stand-in for distTg)."""
        f, tf = fel_of(c), c["fel"][t]
        w = c["w"]
        k = rnd.random()
        if k < exact:
            w["chars"][c["me"]].width = fl(0.5)
            w["chars"][t].width = fl(0.5)
            put_at(tf, f["pos_p"])
        else:
            wd = ff(w["chars"][c["me"]].width) + ff(w["chars"][t].width)
            r = arms(c) + wd + rnd.choice((-100, -40, -5, 5, 60, 400)) if k < 0.85 else rnd.uniform(0, 3000)
            put_at(tf, near(f["pos_p"], max(r, 0), rnd))

    # ccFellow::CheckNote ---------------------------------------------------------------
    def check_note(rnd):
        def tw(rnd, c):
            f = fel_of(c)
            f["act"] = rnd.choice((5, 5, 6, 0, 15))
            f["skill_id"] = rnd.choice((1, 1, rnd.randrange(0, 304)))
            w = c["w"]
            listed = [i for i in w["pcs"] + w["enes"] if i != c["me"]]
            if listed and rnd.random() < 0.85:
                t = rnd.choice(listed)
                f["target"] = t
                w["chars"][t].cond[0] = rnd.choice((0, 0, 0, 0, 1))
                if rnd.random() < 0.2:
                    w["chars"][t].HP = 0
                in_reach(rnd, c, t)
                c["w"]["ais"][c["me"]][P.AI_AT["distTg"]] = fl(arms(c) + rnd.choice((-10, 10)))
        c = fc.case(rnd, tw)
        ev = rnd.choice((0x8005, 0x8005, 0x8005, 1, 2, 3, 0x8001, 0x8002, 0x8003, 0x8004, 9))
        param = rnd.choice((0, 5, rnd.randrange(0, 1 << 31)))
        fc.fg.put_case(c["w"], c)
        m = fc.fg.m
        m.store(NOTE + 4, 4, ev)
        m.store(NOTE + 8, 4, param)
        m.call(fc.fg.sym("CheckNote__8ccFellowFP9ccAnmNote"), (me_va(fc, c), NOTE))
        return request("CheckNote", [c["me"], ev, param], c, b), fc.answer(c, 0), "CheckNote"

    # ccFellow::Action -------------------------------------------------------------------
    def action(rnd):
        def target(rnd, c, f, foes, others, dead=(0, 0, 0, 0, 1, 2), itself=0.04, exact=0.15):
            w = c["w"]
            k = rnd.random()
            if k < 0.12:
                f["target"] = -1
            elif k < 0.2 and others:
                t = rnd.choice(others)                      # off the lists
                f["target"] = t
                for lst in (w["pcs"], w["enes"], w["ents"]):
                    if t in lst:
                        lst.remove(t)
            elif k < 0.2 + itself:
                f["target"] = c["me"]
            elif others:
                t = rnd.choice(foes if foes and rnd.random() < 0.7 else others)
                f["target"] = t
                w["chars"][t].cond[0] = rnd.choice(dead)
                in_reach(rnd, c, t, exact)

        def tw(rnd, c):
            calm(rnd, c, 0.7)
            f = fel_of(c)
            w = c["w"]
            ch = w["chars"][c["me"]]
            foes = [i for i in w["enes"] if i != c["me"]]
            others = [i for i in w["pcs"] + w["enes"] if i != c["me"]]
            if rnd.random() < 0.25:
                w["ais"][c["me"]][0] |= 1                  # manual control
            else:
                w["ais"][c["me"]][0] &= ~1
            k = rnd.choice(("request", "odd", "combo15", "combo15", "combo12", "combo3", "combo4", "combo4", "over",
                            "fade", "transfer", "fidget", "end", "walk"))
            if k == "request":                  # a skill requested
                f["skill_id"] = rnd.choice((1, 1, 1, 180, 180, rnd.randrange(2, 304), rnd.choice(fc.pc.pools.spells),
                                            rnd.choice(fc.pc.pools.arts)))
                f["skill_status"] = rnd.choice((1, 1, 1, 9, 3))
                target(rnd, c, f, foes, others)
                f["act"] = rnd.choice((0, 2, 5, 6, 3, 8, 15))
                if rnd.random() < 0.4:
                    ch.cond[rnd.choice((10, 11))] = rnd.randrange(1, 50)
                if rnd.random() < 0.3 and f["target"] >= 0 and hasattr(w["chars"][f["target"]], "row"):
                    # confused or charmed at a non-fighter
                    w["chars"][f["target"]].type = rnd.choice((0x10, 0x100, 0x8))
                    ch.cond[rnd.choice((10, 11))] = rnd.randrange(1, 50)
            elif k == "odd":                    # an attack confused or charmed at a non-fighter
                f["skill_id"], f["skill_status"] = 1, rnd.choice((0, 1, 2))
                t = rnd.choice([i for i in foes if hasattr(w["chars"][i], "row")] or [-1])
                f["target"] = t
                if t >= 0:
                    w["chars"][t].type = rnd.choice((0x10, 0x100, 0x20))
                for i in (10, 11):
                    if rnd.random() < 0.6:
                        ch.cond[i] = rnd.randrange(1, 50)
            elif k.startswith("combo"):         # the normal attack's combo
                f["skill_id"] = rnd.choice((1, 1, 1, 1, 1, 1, 1, 2))
                f["skill_status"] = rnd.choice((2, 2, 2, 2, 2, 6, 0))
                if k == "combo15":
                    f["attack"], f["act"], f["anm_flag"] = rnd.choice((1, 2)), 15, 1
                elif k == "combo12":
                    f["attack"] = rnd.choice((1, 2))
                    f["act"], f["anm_flag"] = rnd.choice(((15, 0), (0, 1), (6, 0), (7, 1), (9, 0), (16, 1)))
                elif k == "combo3":
                    f["attack"], f["act"] = 3, rnd.choice((0, 2, 4, 5, 6, 15))
                    f["atk_anm"], f["consec"] = rnd.choice(((5, 10), (10, 10), (11, 10), (0, 0), (30, 5)))
                    f["anm_flag"] = rnd.choice((0, 1))
                else:
                    f["attack"], f["act"] = rnd.choice((4, 4, 4, 4, 0, 5)), rnd.choice((0, 2, 5, 6, 6, 7, 9))
                    f["anm_flag"] = rnd.choice((0, 0, 1))
                if k in ("combo4", "combo15"):
                    target(rnd, c, f, foes, others, (0, 0, 0, 0, 1, 2), itself=0.15, exact=0.3)
                else:
                    target(rnd, c, f, foes, others, (0, 0, 0, 0, 1, 2))
                if rnd.random() < 0.12:
                    ch.cond[rnd.choice((9, 14, 10, 11))] = rnd.randrange(1, 50)
                if rnd.random() < 0.5:
                    f["flags"] |= 8
                c["w"]["ais"][c["me"]][P.AI_AT["distTg"]] = fl(arms(c) + rnd.choice((-10, 10)))
            elif k == "over":                   # the skill over
                f["skill_id"] = rnd.choice((1, 1, 0, 5))
                f["skill_status"] = rnd.choice((0, 0, 4, 8))
            elif k == "fade":                   # going down, a ghost, getting up
                ch.cond[0] = rnd.choice((3, 4, 4, 5, 5, 2))
                f["act"] = rnd.choice((10, 10, 10, 0, 2, 5, 6, 9))
                f["cnt"] = rnd.choice((0, 17, 18, 29, 30, 30, 49, 50, 50, 77, 78, 78, rnd.randrange(-2, 100)))
                if rnd.random() < 0.5:
                    f["flags"] ^= 0x40
                if rnd.random() < 0.4:                      # the whole party down
                    for m_ in w["party"]["members"]:
                        if m_ >= 0 and m_ != c["me"]:
                            w["chars"][m_].cond[0] = rnd.choice((2, 3, 4, 1))
            elif k == "transfer":               # transfers
                f["act"] = rnd.choice((12, 12, 13, 13, 14))
                c["warp"] = rnd.choice((0, 1))
                if f["act"] == 13:
                    at, lo, hi = (0, 20, 40) if c["warp"] else (30, 50, 70)
                    f["act_cnt"] = rnd.choice((at, at, at + 1, lo, lo + 1, hi, hi + 1, rnd.randrange(-2, 80)))
                else:
                    f["act_cnt"] = rnd.choice((0, 1, 1, 1, 2, 21, 22, 40, 41, 42, 140, 141, 142,
                                               rnd.randrange(-2, 150)))
                f["lag"] = rnd.choice((0, 0, 2))
                f["anm_flag"] = rnd.choice((0, 0, 0, 1))
                f["flags"] = (f["flags"] & ~0x38) | 0x10
                ch.cond[0] = rnd.choice((0, 0, 0, 1, 4))
                if rnd.random() < 0.5:
                    f["flags"] ^= 0x40
            elif k == "fidget":                 # standing and fidgeting
                f["act"] = rnd.choice((0, 0, 1, 2, 2, 2, 3, 4))
                f["react"] = rnd.choice((449, 450, 450, 450, 451, 460, 0))
                c["w"]["game"]["in_battle"] = rnd.choice((0, 0, 0, 1, 2))
                c["w"]["game"]["area"] = rnd.choice((0, 0, 1, 1, 2))
                ch.cond = [0] * 16
                ch.cond[0] = rnd.choice((0, 0, 0, 1))
                w["ais"][c["me"]][0] &= ~1 if rnd.random() < 0.8 else ~0
                if rnd.random() < 0.3:
                    f["flags"] |= 0x40
            elif k == "end":                    # an act ends
                f["act"] = rnd.choice(tuple(range(0, 22)) + (22, 30, -1, 13, 13, 12, 15, 17))
                f["anm_flag"] = 1
                c["w"]["game"]["area"] = rnd.choice((0, 1, 2))
                ch.cond[0] = rnd.choice((0, 0, 0, 1, 4))
            else:                               # walking, running, standing
                f["flags"] = (f["flags"] & ~0x38) | rnd.choice((0, 8, 0x10, 0x18, 0x28, 0x38, 0x30))
                f["act"] = rnd.choice((f["act"], 5, 6, 6, 5, 0, 2))
                c["w"]["game"]["area"] = rnd.choice((0, 1, 2))
                if rnd.random() < 0.2:
                    ch.cond[0] = 4
        c = fc.case(rnd, tw)
        fc.run(c, "Action__8ccFellowFv", (me_va(fc, c),))
        return request("Action", [c["me"]], c, b), fc.answer(c, 0), "Action"

    # ccFellow::Main ---------------------------------------------------------------------
    def main(rnd):
        def tw(rnd, c):
            calm(rnd, c, 0.5)
            f = fel_of(c)
            ch = c["w"]["chars"][c["me"]]
            k = rnd.random()
            if k < 0.1:
                ch.cond[0] = 2
                f["cnt"] = rnd.choice((-1, 0, 1, -3))
            elif k < 0.25:
                f["act"] = rnd.choice((14, 14, 14, 13, 2))
                f["flags"] = (f["flags"] & ~(7 << 14)) | rnd.choice((6, 7, 5, 1, 6)) << 14 | rnd.choice((0, 1 << 12))
            elif k < 0.35:
                old, ch.id = ch.id, rnd.choice((8, 8, 2))
                w = c["w"]
                if ch.id not in w["lists"]:           # the save's lists follow the id
                    w["lists"][ch.id] = w["lists"][old]
                p = w["party"]
                p["ids"] = [ch.id if m == c["me"] else i for m, i in zip(p["members"], p["ids"])]
                c["w"]["game"]["event_lock"] = rnd.choice((1, 1, 0))
                c["w"]["ais"][c["me"]][0] |= rnd.choice((1, 1, 0))
                f["flags"] |= rnd.choice((0x80, 0x80, 0))
            elif k < 0.55:
                ch.cond[0] = rnd.choice((4, 4, 5, 3, 0, 0))
                f["flags"] = (f["flags"] | 2) ^ rnd.choice((0, 0x40))
                f["act"] = rnd.choice((0, 2, 12, 13, 14, 5))
            elif k < 0.8:
                # the timers' affects: poison down to nothing, regeneration,
                # the curse, in and out of the menus
                ch.cond = [0] * 16
                ch.cond[0] = rnd.choice((0, 0, 0, 1, 4, 5))
                for i, v in ((13, (91, 181, 1, 90, 5)), (12, (61, 1, 5)), (7, (61, 1, 7)), (8, (91, 1, 3))):
                    if rnd.random() < 0.5:
                        ch.cond[i] = rnd.choice(v)
                ch.HP = rnd.choice((1, 2, 3, ch.HP))
                ch.no_death = rnd.random() < 0.1
                if rnd.random() < 0.4:              # poisoned down, then the SP ticks
                    ch.cond[0], ch.cond[13], ch.HP, ch.no_death = 0, 91, 1, False
                    ch.cond[8] = rnd.choice((91, 0, 5))
                    ch.cond[7] = rnd.choice((61, 0))
                    ch.exp = rnd.choice((0, 500))
                f["flags"] = (f["flags"] & ~0x80) | (0x80 if ch.no_death else 0)
                c["w"]["game"]["menu_type"] = rnd.choice((-1, -1, -1, 3))
                ch.exp = ch.exp if rnd.random() < 0.4 else rnd.choice((0, 500, 999, 1000))
            if rnd.random() < 0.3:
                ch.cond[rnd.choice((1, 9, 14))] = rnd.randrange(1, 100)
        c = fc.case(rnd, tw)
        fc.run(c, "Main__8ccFellowFv", (me_va(fc, c),))
        return request("Main", [c["me"]], c, b), fc.answer(c, 0), "Main"

    return [
        ("SetDircZ", set_dirc_z),
        ("CheckFrontObstacle", cfo),
        ("CheckFrontObstacleF", cfo_f),
        ("FollowPlayer", follow("FollowPlayer", "FollowPlayer__4ccAIFv")),
        ("LeavePlayer", follow("LeavePlayer", "LeavePlayer__4ccAIFv")),
        ("FollowTarget", follow("FollowTarget", "FollowTarget__4ccAIFP6ccChar", True)),
        ("FollowTargetDirc", follow("FollowTargetDirc", "FollowTargetDirc__4ccAIFP6ccChar", True)),
        ("Move", move),
        ("HitCheck", hit_check),
        ("CheckNote", check_note),
        ("Action", action),
        ("Main", main),
    ]


# multi-frame runs ---------------------------------------------------------------------------

def run_checks(fc):
    """Orca following Kite for 60 to 300 frames: ccThAISystem, then
    ccFellow::Main, each frame, in a field or a dungeon, alone or with
    foes about."""
    b, tb, data = fc.b, fc.c.tb, fc.c.data
    pools = fc.pc.pools

    def world(rnd, area, foes):
        chars = []
        kite = tb.rnd_pc(b, data, rnd)
        kite.id, kite.type, kite.party_flag = 0, 7, 1
        orca = tb.rnd_pc(b, data, rnd)
        orca.id, orca.type, orca.party_flag = 2, 6, 1
        for pc in (kite, orca):
            pc.cond = [0] * 16
            pc.HP = max(pc.HP, 1)
            pc.exp = rnd.choice((0, 100, 500))
            pc.no_death = False
        if rnd.random() < 0.35:                 # Orca poisoned, cursed, regenerating: down, a ghost
            orca.maxHP = orca.p_maxHP = rnd.randrange(300, 3000)
            orca.HP = rnd.randrange(1, 30)
            orca.cond[13] = rnd.randrange(100, 3000)
            for i in (7, 8, 12):
                if rnd.random() < 0.4:
                    orca.cond[i] = rnd.randrange(50, 2000)
        chars += [kite, orca]
        used = set()
        for _ in range(foes):
            f = tb.rnd_foe(b, data, rnd, "enemy")
            while (f.kind, f.id) in used:
                f = tb.rnd_foe(b, data, rnd, "enemy")
            used.add((f.kind, f.id))
            f.cond = [0] * 16
            f.HP = max(f.HP, 1)
            f.ent_root = 1
            chars.append(f)
        start = [rnd.uniform(-300, 300), rnd.uniform(-300, 300)]
        for i, ch in enumerate(chars):
            if i == 0:
                p = start
            elif i == 1:
                a = rnd.uniform(-math.pi, math.pi)
                r = rnd.uniform(100, 500)
                p = [start[0] + r * math.sin(a), start[1] - r * math.cos(a)]
            else:
                a = rnd.uniform(-math.pi, math.pi)
                r = rnd.uniform(150, 1200)
                p = [start[0] + r * math.sin(a), start[1] - r * math.cos(a)]
            ch.pos = [fl(p[0]), fl(p[1]), 0, F_ONE]
            ch.pos_p = list(ch.pos)
            ch.width = fl(rnd.uniform(5, 20))
            ch.height = fl(rnd.uniform(60, 110))
            ch.skill_id = ch.skill_status = 0
            ch.anm_flag = 0
            ch.spc = {"act_num": 2 if i < 2 else 0, "target_char": -1, "move_flag": 0, "now_speed": 0,
                      "cycle": rnd.randrange(0, 1000), "dist_tg": 0, "spc_list_num": 1, "run_flag": 0,
                      "ghost": 0, "stop_flag": 1, "stop_cnt": 0, "walk_run_cnt": 0}
        pcs = [0, 1]
        enes = list(range(2, len(chars)))
        party = {"members": [0, 1, -1], "ids": [0, 2, -1], "num": 2}
        game = {"in_battle": 1 if foes and rnd.random() < 0.7 else 0, "area": area, "field": 0,
                "field_type": 4, "field_attr": rnd.randrange(0, 6), "menu_type": -1, "event_lock": 0,
                "spc_battle_condition": 0, "party_strategy": rnd.randrange(0, 4), "player": 0, "field_24": 0,
                "pg_ride_flag": 0}
        lists = {}
        for ch in chars[:2]:
            skills = [pools.skill(rnd) for _ in range(20)]
            skills = [-1 if x >= 0 and pools.cost[x] == 0 and pools.check_type(x) in (0, 1) else x for x in skills]
            items = [pools.item(rnd) if rnd.random() < 0.3 else (-1, -1, 0) for _ in range(40)]
            lists[ch.id] = (skills, items)
        time = rnd.randrange(300, 100000)
        ais = {}
        a = fc.pc.rnd_ai(rnd, 1, chars, 0, [0], list(range(len(chars))), time)
        a[0] = rnd.choice((0, 0, 1 << 15)) | 3 << 24                 # flags: firstTime maybe, arts and spells
        a[P.AI_AT["strategyCMD"]] = a[P.AI_AT["strategy"]] = rnd.choice((0, 0, 1, 2))
        a[P.AI_AT["mode"]] = 1
        a[P.AI_AT["modeOld"]] = 1
        a[P.AI_AT["param"]] = 2
        a[P.AI_AT["target"]] = a[P.AI_AT["targetCCmd"]] = -1
        a[P.AI_AT["chatCmd"]] = a[P.AI_AT["chatCmdNew"]] = -1
        a[P.AI_AT["detourCnt"]] = 0
        a[P.AI_AT["naviFinish"]] = 1
        e = a[P.AI_AT["entry"]]
        e[2], e[3], e[4] = 0, 0, 0
        e[5] = [[-1, 0, -1, 0, 0xFFFF, 0xFFFF, 0, 0] for _ in range(10)]
        ais[1] = a
        entry = [1, -1, -1, -1, -1]
        for ch in chars:
            ch.ai = 0
        chars[1].ai = 1
        sys_ = {"time": time, "entry_num": 1, "history_top": 0, "next_id": 1,
                "buff": [[-1, 0, -1, 0, 0xFFFF, 0xFFFF, 0, 0] for _ in range(16)],
                "hist": [[-1, 0, -1, 0, 0xFFFF, 0xFFFF, 0, 0] for _ in range(16)], "entry": entry}
        return {"chars": chars, "pcs": pcs, "enes": enes, "ents": list(enes), "party": party, "game": game,
                "lists": lists, "ais": ais, "sys": sys_, "globals": [-1, -1, -1, 0], "rand": rnd.getrandbits(64),
                "hit": [rnd.choice((F_M1, F_M1, F_M1, 0)) for _ in range(400)],
                "goal": [rnd.choice((0, 1, 1)) for _ in range(400)]}

    def fellow_state(rnd, ch, i):
        f = rnd_fel(rnd, ch, 0)
        f["flags"] = (0x2 | 0x10 | (1 << 14)) if i < 2 else 0x2
        f["arms"] = 0
        f["act"], f["act_old"] = ch.spc["act_num"], -1
        f["atk_anm"] = f["act_cnt"] = f["react"] = f["stop_cnt"] = f["attack"] = f["lag"] = 0
        f["speed"] = fl(rnd.uniform(6, 12))
        f["rate"] = F_ONE
        f["now_speed"] = 0
        f["cloak"] = F_ONE
        f["cnt"] = 0
        f["consec"] = rnd.choice((0, 5, 10))
        f["dirc"] = [0, 0, rnd_angle(rnd), 0]
        f["hsw"] = 1
        f["hmask"] = 0xFFFFFFFF
        f["hradius"] = ch.width
        f["hheight"] = fl(ff(ch.height) * 0.5)
        f["fbyte"] = 0
        f["disp_wait"] = 0
        f["atk_dellay"] = 0
        f["motion"] = 2
        f["target"] = -1
        f["skill_id"] = f["skill_status"] = 0
        f["move_pos"] = [0, 0, 0, F_ONE]
        f["disk"] = [0, 0, 0, F_ONE]
        return f

    def path(rnd, w, frames):
        """Kite's state each frame: walking or running between waypoints,
        stopping now and then."""
        k = w["chars"][0]
        o = w["chars"][1]
        x, y = ff(k.pos[0]), ff(k.pos[1])
        out = []
        # first away from Orca, then as he pleases
        h = math.atan2(x - ff(o.pos[0]), -(y - ff(o.pos[1]))) + rnd.uniform(-1.0, 1.0)
        mode = rnd.choice((1, 2, 2))
        left = rnd.randrange(40, 120)
        for _ in range(frames):
            if left <= 0:
                mode = rnd.choice((0, 1, 2, 2))          # stand, walk, run
                left = rnd.randrange(20, 120) if mode else rnd.randrange(10, 60)
                h = rnd.uniform(-math.pi, math.pi)
            left -= 1
            spd = (0, 3.0, 9.0)[mode]
            x += spd * math.sin(h)
            y -= spd * math.cos(h)
            pos = [fl(x), fl(y), 0, F_ONE]
            out.append((pos, [0, 0, fl(h), 0], int(mode > 0), int(mode == 2)))
        return out

    def check(area, foes_range):
        def run(rnd):
            foes = rnd.choice(foes_range)
            w = world(rnd, area, foes)
            fel = [fellow_state(rnd, ch, i) for i, ch in enumerate(w["chars"])]
            frames = rnd.randrange(60, 301)
            q = {
                "land": [0] * (4 * frames + 50),
                "attr": [0] * (frames + 20),
                "line": [F_M1 if rnd.random() > 0.03 else rnd.choice((0, fl(0.5))) for _ in range(20 * frames)],
                "push": [(0, [0, 0, 0, 0], 0, 0) if rnd.random() > 0.05 else
                         (1, rnd_vec(rnd, 4, 0)[:3] + [0], 0, 4) for _ in range(3 * frames)],
                "fwd": [int(rnd.random() < 0.04) for _ in range(frames + 10)],
                "notes": [[] if rnd.random() > 0.06 else [rnd.choice(((0x8005, 0), (1, 5), (2, 6)))]
                          for _ in range(frames + 10)],
                "trans": [0] * (2 * frames + 10),
                "center": [],
                "event": [int(rnd.random() < 0.2) for _ in range(frames + 10)],
                "draw": [int(rnd.random() < 0.9) for _ in range(frames + 10)],
            }
            env = b.Env(plcol=1, menu_flag=0, in_battle=w["game"]["in_battle"], count=rnd.randrange(0, 1000),
                        menu_type=-1, sp_regene_speed=0, area=area)
            c = {"w": w, "me": 1, "fel": fel, "env": env, "gho": 0, "warp": 0, "open": 0,
                 "off": [0, 0, 0, 0], "q": q, "model": True, "running": 0, "menu": True}
            affects(rnd, w["chars"], plain=True)
            for ch, f in zip(w["chars"], fel):
                sync_fel(ch, f)
            steps = path(rnd, w, frames)
            fg = fc.fg
            fg.put_case(w, c)
            m = fg.m
            game = []
            kva = fg.va(0)
            for pos, dirc, mv, run_ in steps:
                fg.g.put(kva + 0x40, "<4I", *pos)
                fg.g.put(kva + 0x50, "<4I", *pos)
                fg.g.put(kva + 0x60, "<4I", *dirc)
                fl_ = m.load(kva + 0xE0, 1) & ~0x28
                m.store(kva + 0xE0, 1, fl_ | (8 if mv else 0) | (0x20 if run_ else 0))
                env.count = (env.count + 1) & 0xFFFFFFFF
                m.store(TB.SYS + 0x358, 4, env.count)
                fg.tick()
                m.call(fg.sym("Main__8ccFellowFv"), (fg.va(1),))
                game.append(fg.read_case(w, 0))
            fc.c.scene.restore()
            env.count = (env.count - frames) & 0xFFFFFFFF
            extra = " ".join(" ".join(str(x) for x in pos + pos + dirc + [mv, run_]) for pos, dirc, mv, run_ in steps)
            name = "Run" + ("Dungeon" if area == 2 else "Field") + ("Foes" if foes_range[0] else "")
            return request("Run", [1, frames], c, b, extra), {"frames": game}, name
        return run

    def navi_check(area):
        """Orca after Kite round the walls of a room: the world answers
        from the room's map (the lines, the ground, the pushes), the path
        finding runs natively in the game and through the party's movement
        (party_motion::Movement) in the port."""
        def run(rnd):
            foes = rnd.choice((0, 0, 1, 2, 3))
            w = world(rnd, area, foes)
            chars = w["chars"]
            orca = chars[1]
            if orca.cond[13] and rnd.random() < 0.75:     # mostly well enough to follow
                orca.cond = [0] * 16
                orca.HP = orca.maxHP
            while True:
                win, cells, doors = room_map(fc, rnd)
                x0, y0, sz = win
                start = rnd_open_cell(rnd, cells, win)
                reach = reachable(cells, win, start)
                if len(reach) > 8:
                    break
            cx, cy = (x0 + sz / 2) * 300.0, (y0 + sz / 2) * 300.0
            off = [fl(-cx), fl(-cy), 0, 0]
            dist = reachable(cells, win, start, steps=True)
            # Orca behind a wall within the path finding's reach (its wave
            # runs size * 14 / 10 steps), now and then beyond it
            limit = sz * 14 // 10
            behind = [c for c in reach if 2 <= dist[c] <= limit - 2
                      and fc.fg.geo_line_cells(cells, centre(start), centre(c)) != F_M1]
            far = [c for c in reach if dist[c] > limit]
            near = [c for c in reach if 2 <= dist[c] <= 10] or list(reach)
            r_ = rnd.random()
            pool = far if far and r_ < 0.15 else behind if behind and r_ < 0.85 else near
            spots = [start, rnd.choice(pool)]
            spots += [rnd.choice(list(reach)) for _ in chars[2:]]
            # Kite may walk out of the room by a door (the window's way ends
            # there), or start outside it; or stand in a pocket no way leads
            # into; or the room's map is never made (bufFlag 0)
            walk_reach = reach
            v = rnd.random()
            made = 1
            if doors and v < 0.5:
                walk_reach = reachable(cells, None, start)
                out = [c for c in walk_reach if not (x0 <= c[0] < x0 + sz and y0 <= c[1] < y0 + sz)]
                if out and v < 0.15:
                    spots[0] = start = rnd.choice(out)
            elif v < 0.6:
                inner = [c for c in reach if x0 < c[0] < x0 + sz - 1 and y0 < c[1] < y0 + sz - 1
                         and c not in spots and all((c[0] + dx, c[1] + dy) not in spots
                                                    for dx, dy in ((0, -1), (1, 0), (0, 1), (-1, 0)))]
                if inner:
                    pk = rnd.choice(inner)
                    for dx, dy in ((0, -1), (1, 0), (0, 1), (-1, 0)):
                        cells[(pk[0] + dx, pk[1] + dy)] = 4
                    reach = reachable(cells, win, spots[1])
                    spots[0] = start = pk
                    walk_reach = {pk}
                    spots[2:] = [rnd.choice(list(reach)) for _ in chars[2:]]
            elif v < 0.67:
                made = 0
            for ch, c in zip(chars, spots):
                x, y = centre(c)
                ch.pos = [fl(x + rnd.uniform(-60, 60)), fl(y + rnd.uniform(-60, 60)), 0, F_ONE]
                ch.pos_p = [eemu.f_add(ch.pos[i], off[i]) for i in range(4)]
            fel = [fellow_state(rnd, ch, i) for i, ch in enumerate(chars)]
            frames = rnd.randrange(60, 301)
            q = {
                "land": [], "attr": [], "line": [], "push": [],
                "fwd": [int(rnd.random() < 0.04) for _ in range(frames + 10)],
                "notes": [[] if rnd.random() > 0.06 else [rnd.choice(((0x8005, 0), (1, 5), (2, 6)))]
                          for _ in range(frames + 10)],
                "trans": [0] * (2 * frames + 10),
                "center": [],
                "event": [int(rnd.random() < 0.2) for _ in range(frames + 10)],
                "draw": [int(rnd.random() < 0.9) for _ in range(frames + 10)],
            }
            env = b.Env(plcol=1, menu_flag=0, in_battle=w["game"]["in_battle"], count=rnd.randrange(0, 1000),
                        menu_type=-1, sp_regene_speed=0, area=area)
            c = {"w": w, "me": 1, "fel": fel, "env": env, "gho": 0, "warp": 0, "open": 0,
                 "off": off, "q": q, "model": True, "running": 0, "menu": True}
            affects(rnd, chars, plain=True)
            for ch, f in zip(chars, fel):
                sync_fel(ch, f)
            nv = {"goal_pos": [0, 0, 0, F_ONE], "start_x": 0, "start_y": 0, "goal_x": 0, "goal_y": 0, "name": -1,
                  "step": 0, "landmark": 0, "dist": 0, "dirc": 0,
                  "map_x": x0 if area == 2 else 0, "map_y": y0 if area == 2 else 0, "map_s": sz if area == 2 else 0,
                  "route": bytes(48), "pad": [0, 0], "beacon_num": 0, "beacon": None}
            nav = {"area": area, "cells": cells, "info": [x0, y0, sz], "registry": 2, "navi": {1: nv}, "made": made}
            steps = kite_walk(rnd, cells, walk_reach, start, frames, off)
            fg = fc.fg
            fg.put_case(w, c)
            fg.navi_on(nav)
            m = fg.m
            game = []
            kva = fg.va(0)
            try:
                for pos, pos_p, dirc, mv, run_ in steps:
                    fg.g.put(kva + 0x40, "<4I", *pos)
                    fg.g.put(kva + 0x50, "<4I", *pos_p)
                    fg.g.put(kva + 0x60, "<4I", *dirc)
                    fl_ = m.load(kva + 0xE0, 1) & ~0x28
                    m.store(kva + 0xE0, 1, fl_ | (8 if mv else 0) | (0x20 if run_ else 0))
                    env.count = (env.count + 1) & 0xFFFFFFFF
                    m.store(TB.SYS + 0x358, 4, env.count)
                    fg.tick()
                    m.call(fg.sym("Main__8ccFellowFv"), (fg.va(1),), 400_000_000)
                    out = fg.read_case(w, 0)
                    out["navi"] = {"1": fg.read_navi(1)}
                    out["path"] = fg.read_path()
                    game.append(out)
            finally:
                rec = fg.rec
                fg.navi_off()
            fc.c.scene.restore()
            env.count = (env.count - frames) & 0xFFFFFFFF
            for k in ("land", "attr", "line", "push"):
                q[k] = rec.get(k, [])
            w["hit"] = rec.get("hit", [])
            xs = [x for x, _ in cells if 0 <= x < 256]
            ys = [y for _, y in cells if 0 <= y < 256]
            mx, my = min(xs), min(ys)
            mw, mh = max(xs) + 1 - mx, max(ys) + 1 - my
            cells_hex = NV.hexs(bytes(cells.get((mx + i, my + j), 0) for i in range(mw) for j in range(mh)))
            navi = f"{nav['registry']} {made} 1 1 {NV.ser_navi(nv)} {mx} {my} {mw} {mh} {cells_hex} {x0} {y0} {sz}"
            extra = " ".join(" ".join(str(x) for x in pos + pos_p + dirc + [mv, run_])
                             for pos, pos_p, dirc, mv, run_ in steps)
            name = "Run" + ("Dungeon" if area == 2 else "Field") + "Navi"
            return request("RunNavi", [1, frames], c, b, navi + " " + extra), {"frames": game}, name
        return run

    return [
        ("RunField", check(1, (0,))),
        ("RunFieldFoes", check(1, (1, 2, 3))),
        ("RunDungeon", check(2, (0,))),
        ("RunDungeonFoes", check(2, (1, 2, 3))),
        ("RunDungeonNavi", navi_check(2)),
        ("RunFieldNavi", navi_check(1)),
    ]


# the rooms of the navigation runs -----------------------------------------------------------

def centre(c):
    return c[0] * 300.0 + 150.0, c[1] * 300.0 + 150.0


def open_cell(v):
    return v not in (0, 4)


def room_map(fc, rnd):
    """A room window (x, y, size) and its 2D map: a room of the tutorial
    dungeon or a random dungeon as the navigation harness lays it out
    (walled round but for its doors, blocks inside), or a room of 10 or 20
    cells with one or two walls across it, each leaving a way round, and
    pillars."""
    if rnd.random() < 0.3:
        floors = fc.floor_wins()
        wins = rnd.choice([f for f in floors if f])
        x, y, sz, exits = rnd.choice(wins)
        if sz <= 20:
            return (x, y, sz), NV.floor_map(rnd, [(x, y, sz, exits)]), False
    # (rooms of 10 and 20: their ways stay within 28 cells; from 49 the
    # game's route runs over beacon.num and from 55 over the beacon table's
    # pointer, which the port does not follow - see crate::navi)
    sz = rnd.choice((10, 10, 20, 20, 20))
    x0, y0 = rnd.randrange(2, 253 - sz), rnd.randrange(2, 253 - sz)
    cells = {}
    for x in range(x0 - 1, x0 + sz + 1):
        for y in range(y0 - 1, y0 + sz + 1):
            edge = x in (x0 - 1, x0 + sz) or y in (y0 - 1, y0 + sz)
            cells[(x, y)] = 4 if edge else rnd.choice((1,) * 12 + (2, 3))
    for _ in range(rnd.choice((1, 1, 2))):
        across = rnd.random() < 0.5
        at = rnd.randrange(sz // 4, 3 * sz // 4)
        n = rnd.randrange(sz // 2, sz - 1)
        low = rnd.random() < 0.5
        for k in range(n):
            j = k if low else sz - 1 - k
            cells[(x0 + at, y0 + j) if across else (x0 + j, y0 + at)] = rnd.choice((4, 4, 4, 0))
    for _ in range(rnd.randrange(0, 1 + sz // 5)):
        ax, ay = rnd.randrange(x0, x0 + sz), rnd.randrange(y0, y0 + sz)
        v = rnd.choice((4, 4, 0))
        for x in range(ax, min(ax + rnd.randrange(1, 3), x0 + sz)):
            for y in range(ay, min(ay + rnd.randrange(1, 3), y0 + sz)):
                cells[(x, y)] = v
    # a door or two, each with a corridor of 3 to 8 cells beyond
    doors = rnd.random() < 0.6
    if doors:
        for _ in range(rnd.choice((1, 1, 2))):
            side = rnd.randrange(4)
            k = rnd.randrange(1, sz - 1)
            n = rnd.randrange(3, 9)
            for j in range(n + 1):
                c = ((x0 + k, y0 - 1 - j), (x0 + k, y0 + sz + j), (x0 - 1 - j, y0 + k), (x0 + sz + j, y0 + k))[side]
                if 0 <= c[0] < 256 and 0 <= c[1] < 256:
                    cells[c] = 1
    return (x0, y0, sz), cells, doors


def rnd_open_cell(rnd, cells, win):
    x0, y0, sz = win
    for _ in range(200):
        c = (rnd.randrange(x0, x0 + sz), rnd.randrange(y0, y0 + sz))
        if open_cell(cells.get(c, 0)):
            return c
    return (x0, y0)


def reachable(cells, win, start, steps=False):
    """The open cells of the window (or of the whole map, `win` None) a
    walk from `start` reaches (with the number of steps to each)."""
    x0, y0, sz = win or (0, 0, 256)
    dist = {start: 0}
    todo = [start]
    for c in todo:
        for dx, dy in ((0, -1), (1, 0), (0, 1), (-1, 0)):
            n = (c[0] + dx, c[1] + dy)
            if n in dist or not (x0 <= n[0] < x0 + sz and y0 <= n[1] < y0 + sz) or not open_cell(cells.get(n, 0)):
                continue
            dist[n] = dist[c] + 1
            todo.append(n)
    return dist if steps else set(dist)


def way(cells, reach, a, b):
    """The cells of a shortest walk from a to b within `reach`."""
    prev = {a: None}
    todo = [a]
    for c in todo:
        if c == b:
            break
        for dx, dy in ((0, -1), (1, 0), (0, 1), (-1, 0)):
            n = (c[0] + dx, c[1] + dy)
            if n in reach and n not in prev:
                prev[n] = c
                todo.append(n)
    out = []
    c = b if b in prev else a
    while c is not None:
        out.append(c)
        c = prev[c]
    return out[::-1]


def kite_walk(rnd, cells, reach, start, frames, off):
    """Kite's state each frame (pos, posP, dirc, moveFlag, runFlag):
    walking or running from cell to cell along the room's open ways to one
    place after another, standing now and then."""
    x, y = (ff(fl(v)) for v in centre(start))
    cell = start
    route = []
    h = rnd.uniform(-math.pi, math.pi)
    mode = rnd.choice((1, 2, 2))
    left = rnd.randrange(40, 120)
    out = []
    targets = list(reach)
    while len(out) < frames:
        if not route:
            route = [centre(c) for c in way(cells, reach, cell, rnd.choice(targets))[1:]]
            if not route:
                route = [centre(cell)]
        if left <= 0:
            mode = rnd.choice((0, 1, 2, 2))
            left = rnd.randrange(20, 120) if mode else rnd.randrange(10, 60)
        left -= 1
        if mode:
            tx, ty = route[0]
            dx, dy = tx - x, ty - y
            d = math.hypot(dx, dy)
            spd = (3.0, 9.0)[mode - 1]
            if d <= spd:
                x, y = tx, ty
                route.pop(0)
                cell = (math.floor(x / 300.0), math.floor(y / 300.0))
            else:
                x += dx / d * spd
                y += dy / d * spd
            if d > 0:
                h = math.atan2(dx, -dy)
        pos = [fl(x), fl(y), 0, F_ONE]
        pos_p = [eemu.f_add(pos[i], off[i]) for i in range(4)]
        out.append((pos, pos_p, [0, 0, fl(h), 0], int(mode > 0), int(mode == 2)))
    return out


# the runner -----------------------------------------------------------------------------------

def diff_frames(port, game, what):
    """The first frame that differs, and where."""
    if not isinstance(port, dict) or "frames" not in port:
        return T.diff(port, game, what)
    pf, gf = port["frames"], T.norm(game)["frames"]
    for k, (a, g) in enumerate(zip(pf, gf)):
        d = T.diff(a, g, f"{what} frame {k}")
        if d:
            return d
    if len(pf) != len(gf):
        return f"{what}: {len(pf)} frames, game {len(gf)}"
    return None


def run(checks, name, n, seed):
    """test_battle_rs.run for these checks: the requests go to the probe in
    batches (a run's answer is large), runs compare frame by frame."""
    rnd = random.Random(seed)
    fn = T.EXTRA[name]
    bad, first, first_req = 0, None, None
    batch = 8 if name.startswith("Run") else 100
    done = 0
    while done < n:
        k = min(batch, n - done)
        lines, answers = [], []
        for _ in range(k):
            checks.reset()
            req, game, label = fn(checks, rnd)
            lines.append(req)
            answers.append((game, label))
        got = T.ask(lines)
        for j, (game, label) in enumerate(answers):
            d = diff_frames(got[j], game, label)
            if d:
                bad += 1
                if first is None:
                    first, first_req = d, lines[j]
        done += k
    return bad, first, first_req


class Fellow(T.Checks):
    """The checks as test_battle_rs's runner takes them."""

    def __init__(self):
        super().__init__()
        self.fc = FellowChecks(self)
        self.fn = dict(make_checks(self.fc) + run_checks(self.fc))


def _check(name):
    return lambda checks, rnd: checks.fn[name](rnd)


CHECK_NAMES = ["SetDircZ", "CheckFrontObstacle", "CheckFrontObstacleF", "FollowPlayer", "LeavePlayer",
               "FollowTarget", "FollowTargetDirc", "Move", "HitCheck", "CheckNote", "Action", "Main",
               "RunField", "RunFieldFoes", "RunDungeon", "RunDungeonFoes", "RunDungeonNavi", "RunFieldNavi"]
FELLOW = {name: _check(name) for name in CHECK_NAMES}
T.run = run


def _test(names, seed):
    def test(self):
        for k, name in enumerate(names):
            self.check(name, seed + k)
    return test


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class FellowAgainstGame(T.Against):
    CASES = 150
    TABLE = FELLOW
    MAKE = Fellow

    test_following = _test(CHECK_NAMES[:7], 22000)
    test_move = _test(CHECK_NAMES[7:10], 22100)
    test_action = _test(CHECK_NAMES[10:12], 22200)

    def test_runs(self):
        for k, name in enumerate(CHECK_NAMES[12:]):
            bad, first, req = run(self.checks, name, 12, 22300 + k)
            self.assertEqual(bad, 0, f"{first}\nrequest: {(req or '')[:2000]}")


if __name__ == "__main__":
    T.main(CHECK_NAMES, 22500, FELLOW, Fellow)
