#!/usr/bin/env python3
"""crates/piney-battle's evparty.rs against the game's own ccEvent::Execute
(main 0x001a8d20) and ccThEvHold (main 0x001b5040) run in the EE
interpreter (the Rust machine of tools/test_anim.py when it is built).

Every check builds a random world - Kite, party members with their ccAI,
enemies, NPCs and objects (up to ten characters, each with its base type
and id, dead and hold, affectType, position, posP, heading, act, old act,
the +0xe0 flag word, cloak, bodyHit's hitSW, entParam.param[2]), the three
command lists in a random order with their tails, ccSpcManager's registry
and ccPartyManager (ids, characters, memberChar, memberID, num, with free
slots, repeated ids, null characters), the entry control's enemy and NPC
lists (g_entCtrl), the event manager's 16 event positions, the Root Town's
33 markers (markerEvTbl's dummies, found through a hooked
ccStream::GetChunkAdrsF), game.area, plw.pw, cmndTarget, and an offset the
player's frame is moved by (ccTransPosW2P adds it, ccTransPosP2W takes it
off) - runs the instruction through the game's own Execute at level 2 on a
short array, and sends the same world and instruction to the battle_probe
example (`evp` requests). Compared after each: every field above of every
character, its AI's flags byte, remoteCmd, gDeg, gRotSp, gPoint and gPos,
the three command lists in order, cmndTarget, and the calls made in order
(HitEnable, the markers looked up, deleteEnemy, ccStartThread,
ccDeleteThread).

  command        pc_command (case 66)       ccEntryCmnd / ccDeleteCmnd natively
  walk_pos       pc_walk_pos (61), pc_run_pos (164)
  walk_dir       pc_walk_dir (62)
  walk_marker    pc_walk_marker (63)        markers that exist (see below)
  walk_char      pc_walk_char (64)          GetSpc, GetNpc, GetEnemy natively
  put            pc_put (70), party_put (69)
  put_marker     pc_put_marker (67), party_put_marker (68)
  turn           pc_turn (71), pc_face (72)
  enemy_put      enemy_put (74)
  remove         remove (13) of types 5 and 6: whom deleteEnemy is given
  hold_op        hold (132), hold_end (133): the task started or stopped
  hold_task      ccThEvHold from its start over 20-120 frames, types 5, 6
                 and 7, with holds cleared, positions moved, affectType 13,
                 foes taken off and put back on the command list and the
                 entry control's enemy list reordered between frames;
                 EntryAffect, ccCheckTarget, ccGetDirc and
                 ccCheckTargetTypeId native
  player_skill   player_skill (153) from its set-up over 1-60 frames, the
                 pad, ccSkillCheck's answer, the target's death and the
                 command target changing between frames; ccSkillCheck and
                 ccSkillRequest recorded
  battle_ready   battle_ready (163): game.inBattleDist

ManualModeAI, ManualMode, SetRemoteCmd, SetGoalPos, SetDircZ,
checkPartyAnnihilation, ccGetDircPL, sinf, cosf, DEG2RAD and RAD2DEG run
natively. pc_walk_marker with no event position of that number (or in an
area other than 0-2) walks to what Execute's stack holds; those cases are
not generated.

    python3 tools/test_battle_event_rs.py            the unit tests
    python3 tools/test_battle_event_rs.py bulk N     N cases of every check
"""

import math
import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle_rs as rs  # noqa: E402
import volume  # noqa: E402
from test_battle import GAME, MENU, SAVE, SYS  # noqa: E402

AIS = 0x01060000        # the ccAI of character i at AIS + 0x400 * i
ENTS = 0x01071000       # g_entCtrl: +0x10 enNum, +0x14 enHead, +0x34 npcNum, +0x38 npcHead
EVT = 0x01072000        # the ccEvent (0x7e0 bytes)
TSCB = 0x01073000       # the task ccStartThread hands back
TSCB_OLD = 0x01073100   # a hold task already running (type 99, code 99)
BOSST = 0x01073200      # eventMng.bossTscb
WM = 0x01074000         # worldman
WM2 = 0x01074800        # what worldman +0x430 points at
CHUNKS = 0x01075000     # the markers' dummies, 0x40 each
EVBUF, EVCELL = 0x01076000, 0x01076100
F_ONE = 0x3F800000
MAXC = 10
MARKERS = 33
PI = math.pi


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


# random worlds -------------------------------------------------------------------------

PARTY_IDS = (2, 3, 5, 8, 11)
ENEMY_IDS = (130, 131, 7, 44)
NPC_IDS = (29, 158, 5, 137)


def rnd_vec(rnd, w=None):
    v = [fb(rnd.uniform(-3000, 3000)), fb(rnd.uniform(-3000, 3000)), fb(rnd.uniform(-50, 50))]
    return v + [F_ONE if w is None else w]


def rnd_char(rnd, kind, cid):
    """A character of a kind: kite, pc, enemy, boss, npc, obj."""
    ty = {"kite": 1, "pc": rnd.choice((2, 4)), "enemy": rnd.choice((0x20, 0x40, 0x60)), "boss": 0x80,
          "npc": rnd.choice((0x08, 0x10, 0x18)), "obj": rnd.choice((0x100, 0x2000))}[kind]
    dead = rnd.choice((0, 0, 0, 0, 1, 2, 3, 4, 5))
    act = rnd.choice((0, 1, 2, 5, 6, 9, 10, 11, 12, 13, 14, 15, 24))
    word = rnd.getrandbits(32) & ~(0xFF << 24)
    c = {"kind": kind, "ty": ty, "id": cid, "dead": dead, "hold": rnd.choice((0, 0, 1)),
         "aff": rnd.choice((0, 1, 5, 13, 13 if kind in ("enemy", "boss") else 0)),
         "pos": rnd_vec(rnd, rnd.choice((F_ONE, F_ONE, fb(rnd.uniform(0, 2))))), "posp": rnd_vec(rnd),
         "dirc": [0, 0, fb(rnd.uniform(-PI, PI)), F_ONE], "act": act, "actold": rnd.choice((-1, 2, act)),
         "word": word, "cloak": rnd.choice((F_ONE, 0, fb(rnd.random()))), "hitsw": rnd.choice((0, 1)),
         "param2": rnd.choice((0, -1, 1, 7)) if kind in ("enemy", "boss") else 0,
         "enemy": int(kind in ("enemy", "boss")), "ai": None}
    if kind in ("kite", "pc") and rnd.random() < 0.95:
        c["ai"] = [rnd.getrandbits(8), rnd.choice((0, 1, 2, 3, 5, 7)), rnd.getrandbits(16),
                   rnd.choice((64, 1, 10)), rnd.choice((0, -1, -2, 3)), rnd_vec(rnd)]
    return c


def rnd_world(rnd, area=None, n_pc=None, n_enemy=None):
    kinds = ["kite"]
    kinds += ["pc"] * (rnd.choice((0, 1, 2, 2)) if n_pc is None else n_pc)
    kinds += ["enemy"] * (rnd.randrange(0, 4) if n_enemy is None else n_enemy)
    kinds += rnd.choice(([], ["boss"], ["npc"], ["npc", "npc"], ["obj"], ["npc", "obj"]))
    kinds = kinds[:MAXC]
    chars = []
    pc_ids = rnd.sample(PARTY_IDS, len(PARTY_IDS))
    for k in kinds:
        cid = {"kite": 0, "pc": pc_ids.pop() if pc_ids else 2, "enemy": rnd.choice(ENEMY_IDS),
               "boss": rnd.choice(ENEMY_IDS), "npc": rnd.choice(NPC_IDS), "obj": rnd.choice((16, 3))}[k]
        chars.append(rnd_char(rnd, k, cid))
    n = len(chars)
    pcs = [i for i, c in enumerate(chars) if c["kind"] in ("kite", "pc")]
    # The registry: Kite first, the others in some order, free slots, now and then a null
    # character or a stray id.
    ids, cptr = [-1] * 5, [-1] * 5
    slots = list(range(5))
    ids[0], cptr[0] = 0, 0
    free = slots[1:]
    rnd.shuffle(free)
    for i in pcs[1:]:
        s = free.pop()
        ids[s] = chars[i]["id"]
        cptr[s] = i if rnd.random() < 0.9 else -1
    if free and rnd.random() < 0.3:
        s = free.pop()
        ids[s] = rnd.choice(PARTY_IDS + (0,))
        cptr[s] = rnd.choice(pcs + [-1])
    # The party: Kite in slot 0; companions in slots 1, 2 (some empty).
    members, mids = [0, -1, -1], [0, -1, -1]
    comp = pcs[1:]
    rnd.shuffle(comp)
    for s in (1, 2):
        if comp and rnd.random() < 0.8:
            i = comp.pop()
            members[s], mids[s] = i, chars[i]["id"]
        elif rnd.random() < 0.1:
            mids[s] = rnd.choice(PARTY_IDS)
    num = sum(1 for m in mids if m >= 0)
    if rnd.random() < 0.1:
        num = rnd.randrange(0, 4)
    # Command lists: each character on its type's list or not, in a random order.
    lists = [[], [], []]
    for i, c in enumerate(chars):
        if rnd.random() < 0.7:
            k = 0 if c["ty"] & 7 else (1 if c["ty"] & 0xE0 else 2)
            lists[k].append(i)
    for lst in lists:
        rnd.shuffle(lst)
    en = [i for i, c in enumerate(chars) if c["enemy"] and rnd.random() < 0.9]
    rnd.shuffle(en)
    npc = [i for i, c in enumerate(chars) if c["kind"] == "npc"]
    rnd.shuffle(npc)
    positions = []
    for _ in range(16):
        num_ = rnd.choice((-1, -1, rnd.randrange(0, 8), rnd.randrange(0, 8), rnd.randrange(-3, 110)))
        positions.append([rnd.randrange(-2, 10), rnd.randrange(-2, 10), num_, fb(rnd.uniform(-PI, PI)),
                          rnd_vec(rnd)])
    markers = [[rnd_vec(rnd), fb(rnd.uniform(-PI, PI))] for _ in range(MARKERS)]
    listed = [i for lst in lists for i in lst]
    return {"area": rnd.choice((0, 1, 1, 2, 2)) if area is None else area, "kite": 0,
            "target": rnd.choice(listed + [-1]) if listed else -1,
            "d": [fb(rnd.uniform(-500, 500)) for _ in range(3)] + [0], "ids": ids, "cptr": cptr,
            "members": members, "mids": mids, "num": num, "chars": chars, "lists": lists, "en": en,
            "npc": npc, "positions": positions, "markers": markers}


def ser_world(w):
    out = [w["area"], w["kite"], w["target"]] + w["d"] + w["ids"] + w["cptr"] + w["members"] + w["mids"]
    out += [w["num"], len(w["chars"])]
    for c in w["chars"]:
        out += [c["ty"], c["id"], c["dead"], c["hold"], c["aff"]] + c["pos"] + c["posp"] + c["dirc"]
        out += [c["act"], c["actold"], c["word"], c["cloak"], c["hitsw"], c["param2"], c["enemy"],
                int(c["ai"] is not None)]
        if c["ai"] is not None:
            a = c["ai"]
            out += [a[0], a[1], a[2], a[3], a[4]] + a[5]
    for lst in w["lists"] + [w["en"], w["npc"]]:
        out += [len(lst)] + lst
    out.append(len(w["positions"]))
    for p in w["positions"]:
        out += p[:4] + p[4]
    out.append(len(w["markers"]))
    for pos, rot in w["markers"]:
        out += pos + [rot]
    return " ".join(str(v) for v in out)


# the game side ---------------------------------------------------------------------------

class EvGame:
    """The world in the game's memory, ccEvent::Execute and ccThEvHold run on it."""

    def __init__(self, checks):
        g = checks.game
        self.g, self.m, self.sym = g, g.m, g.sym
        m, sym = self.m, self.sym
        self.spc = sym("ccSpcManager")
        self.party = sym("ccPartyManager")
        self.plw = sym("plw")
        self.cmnd = [(sym("cmndPcRoot"), sym("cmndPcLast")), (sym("cmndEneRoot"), sym("cmndEneLast")),
                     (sym("cmndObjRoot"), sym("cmndObjLast"))]
        self.target = sym("cmndTarget")
        self.target_fix = sym("cmndTargetFix")
        self.execute = sym("Execute__7ccEventFRPsiii")
        self.hold_task = sym("ccThEvHold__FP6ccTscb")
        m.store(sym("g_entCtrl"), 4, ENTS)
        m.store(sym("eventMng"), 4, EVT)
        m.store(sym("worldman"), 4, WM)
        m.store(WM + 0x430, 4, WM2)
        m.store(WM2 + 0x1A4, 4, 0x1234)
        tbl = sym("markerEvTbl")
        self.marker_of = {m.load(tbl + 4 * k, 4): k for k in range(MARKERS)}
        self.calls = []
        self.d = [0, 0, 0, 0]
        # The characters' own: ccCheckTarget's walk, EntryAffect, the party's annihilation.
        for name in ("checkPartyAnnihilation__Fv", "EntryAffect__6ccCharFP6ccCharssss"):
            m.hooks.pop(sym(name), None)

        def owner(a, off):
            return (a - off - rs.SCN) // 0x1000

        def hit_enable(mm, hit, *_):
            self.calls.append(["HitEnable", owner(hit, 0x1A0)])
            mm.store(hit, 4, 1)
            return 0

        def w2p(mm, out, v, *_):
            x = [mm.load(v + 4 * i, 4) for i in range(4)]
            for i in range(4):
                mm.store(out + 4 * i, 4, eemu.f_add(x[i], self.d[i]))
            return 0

        def p2w(mm, out, v, *_):
            x = [mm.load(v + 4 * i, 4) for i in range(4)]
            for i in range(4):
                mm.store(out + 4 * i, 4, eemu.f_sub(x[i], self.d[i]))
            return 0

        def chunk(mm, stream, name, *_):
            k = self.marker_of[name]
            self.calls.append(["marker", k])
            return CHUNKS + 0x40 * k

        def start(mm, fn, prio, size, *_):
            mm.mem[TSCB:TSCB + 0x40] = bytes(0x40)
            self.calls.append(["start", prio, size])
            return TSCB

        def delete(mm, t, *_):
            self.calls.append(["delete"])
            return 0

        def delete_enemy(mm, ctrl, obj, *_):
            self.calls.append(["deleteEnemy", owner(obj, 0)])
            return 0

        hooks = {"HitEnable__9ccCharHitFv": hit_enable, "ccTransPosW2P__FPfPf": w2p, "ccTransPosP2W__FPfPf": p2w,
                 "GetChunkAdrsF__8ccStreamFPCci": chunk, "ccStartThread__FPFPv_vii": start,
                 "ccDeleteThread__FP6ccTscb": delete, "deleteEnemy__11ccEntryCtrlFP10ccEntryObj": delete_enemy,
                 "deleteAllObject__11ccEntryCtrlFv": lambda mm, *_: self.calls.append(["deleteAllObject"]) or 0}
        for n, h in hooks.items():
            m.hooks[sym(n)] = h

    def va(self, i):
        return rs.SCN + 0x1000 * i

    def ix(self, a):
        a &= 0xFFFFFFFF
        if a == 0:
            return -1
        if rs.SCN <= a < rs.SCN + 0x1000 * MAXC and (a - rs.SCN) % 0x1000 == 0:
            return (a - rs.SCN) // 0x1000
        return -1000 - a

    def vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x & 0xFFFFFFFF)

    def rvec(self, a):
        return [self.m.load(a + 4 * i, 4) for i in range(4)]

    def put_lists(self, lists):
        m = self.m
        for (root, last), lst in zip(self.cmnd, lists):
            m.store(root, 4, self.va(lst[0]) if lst else 0)
            m.store(last, 4, self.va(lst[-1]) if lst else 0)
            for a, b in zip(lst, lst[1:] + [None]):
                m.store(self.va(a) + 0xBC, 4, self.va(b) if b is not None else 0)

    def put_entries(self, en, npc):
        m = self.m
        for (num, head, foot), lst in (((0x10, 0x14, 0x18), en), ((0x34, 0x38, 0x3C), npc)):
            m.store(ENTS + num, 4, len(lst))
            m.store(ENTS + head, 4, self.va(lst[0]) if lst else 0)
            m.store(ENTS + foot, 4, self.va(lst[-1]) if lst else 0)
            for k, c in enumerate(lst):
                m.store(self.va(c) + 0x1C0, 4, self.va(lst[k - 1]) if k else 0)
                m.store(self.va(c) + 0x1C4, 4, self.va(lst[k + 1]) if k + 1 < len(lst) else 0)

    def put(self, w):
        m = self.m
        m.mem[rs.SCN:rs.SCN + 0x1000 * MAXC] = bytes(0x1000 * MAXC)
        m.mem[AIS:AIS + 0x400 * MAXC] = bytes(0x400 * MAXC)
        m.mem[ENTS:ENTS + 0x40] = bytes(0x40)
        m.mem[EVT:EVT + 0x7E0] = bytes(0x7E0)
        self.calls = []
        self.d = w["d"]
        for i, c in enumerate(w["chars"]):
            va = self.va(i)
            base = va + 0x800
            m.store(va, 4, base)
            m.store(base + 8, 4, c["ty"])
            m.store(base + 0xC, 2, c["id"] & 0xFFFF)
            m.store(va + 8, 2, c["dead"] & 0xFFFF)
            m.store(va + 0xA, 2, c["hold"] & 0xFFFF)
            m.store(va + 0x9C, 2, c["aff"] & 0xFFFF)
            self.vec(va + 0x40, c["pos"])
            self.vec(va + 0x50, c["posp"])
            self.vec(va + 0x60, c["dirc"])
            m.store(va + 0xE0, 4, c["word"])
            m.store(va + 0xEE, 2, c["act"] & 0xFFFF)
            m.store(va + 0xF0, 2, c["actold"] & 0xFFFF)
            m.store(va + 0x110, 4, c["cloak"])
            m.store(va + 0x1A0, 4, c["hitsw"])
            m.store(va + 0x150, 4, c["param2"] & 0xFFFFFFFF)
            if c["ai"] is not None:
                a, ai = AIS + 0x400 * i, c["ai"]
                m.store(va + 0x128, 4, a)
                m.store(a + 0x18, 4, va)
                m.store(a, 1, ai[0])
                m.store(a + volume.ai_at(0x74), 2, ai[1] & 0xFFFF)
                m.store(a + volume.ai_at(0x9A), 2, ai[2])
                m.store(a + volume.ai_at(0x9C), 2, ai[3] & 0xFFFF)
                m.store(a + volume.ai_at(0x9E), 2, ai[4] & 0xFFFF)
                self.vec(a + 0xE0, ai[5])
        self.put_lists(w["lists"])
        self.put_entries(w["en"], w["npc"])
        m.mem[self.spc:self.spc + 0xE0] = bytes(0xE0)
        for i in range(5):
            r = self.spc + 0x2C * i
            m.store(r, 4, w["ids"][i] & 0xFFFFFFFF)
            m.store(r + 0x1C, 4, self.va(w["cptr"][i]) if w["cptr"][i] >= 0 else 0)
        for s in range(3):
            m.store(self.party + 4 * s, 4, self.va(w["members"][s]) if w["members"][s] >= 0 else 0)
            m.store(self.party + 0xC + 4 * s, 4, w["mids"][s] & 0xFFFFFFFF)
        m.store(self.party + 0x18, 4, w["num"] & 0xFFFFFFFF)
        for k, p in enumerate(w["positions"]):
            a = EVT + 0x1C0 + 0x20 * k
            m.store(a, 2, p[0] & 0xFFFF)
            m.store(a + 2, 2, p[1] & 0xFFFF)
            m.store(a + 4, 4, p[2] & 0xFFFFFFFF)
            m.store(a + 8, 4, p[3])
            self.vec(a + 0x10, p[4])
        for k, (pos, rot) in enumerate(w["markers"]):
            a = CHUNKS + 0x40 * k
            m.mem[a:a + 0x40] = bytes(0x40)
            self.vec(a + 0x10, pos)
            m.store(a + 0x28, 4, rot)
        m.store(GAME + 0x14, 4, w["area"])
        m.store(self.plw + 0x20, 4, self.va(w["kite"]) if w["kite"] >= 0 else 0)
        m.store(self.target, 4, self.va(w["target"]) if w["target"] >= 0 else 0)

    def lists(self):
        m = self.m
        out = []
        for root, _ in self.cmnd:
            lst, c = [], m.load(root, 4)
            while c and len(lst) <= MAXC:
                lst.append(self.ix(c))
                c = m.load(c + 0xBC, 4)
            out.append(lst)
        return out

    def state(self, w):
        m = self.m
        chars = []
        for i, c in enumerate(w["chars"]):
            va = self.va(i)
            ai = None
            a = m.load(va + 0x128, 4)
            if a:
                at = volume.ai_at
                ai = [m.load(a, 1), m.load(a + at(0x74), 2, True), m.load(a + at(0x9A), 2),
                      m.load(a + at(0x9C), 2, True), m.load(a + at(0x9E), 2, True), self.rvec(a + 0xE0)]
            chars.append({"dead": m.load(va + 8, 2, True), "hold": m.load(va + 0xA, 2, True),
                          "pos": self.rvec(va + 0x40), "dirc": self.rvec(va + 0x60),
                          "act": m.load(va + 0xEE, 2, True), "actold": m.load(va + 0xF0, 2, True),
                          "flags": m.load(va + 0xE0, 4), "cloak": m.load(va + 0x110, 4),
                          "hitsw": int(m.load(va + 0x1A0, 4) != 0),
                          "param2": s32(m.load(va + 0x150, 4)) if c["enemy"] else 0, "ai": ai})
        return {"chars": chars, "lists": self.lists(), "target": self.ix(m.load(self.target, 4))}

    def run(self, shorts):
        m = self.m
        for k, v in enumerate(list(shorts) + [0]):
            m.store(EVBUF + 2 * k, 2, v & 0xFFFF)
        m.store(EVCELL, 4, EVBUF)
        m.call(self.execute, [EVT, EVCELL, 2, 63, 2], limit=20_000_000)


# the checks --------------------------------------------------------------------------------

OPS = {"command": 0x42, "walkpos": 0x3D, "runpos": 0xA4, "walkdir": 0x3E, "walkmarker": 0x3F, "walkchar": 0x40,
       "put": 0x46, "partyput": 0x45, "putmarker": 0x43, "partyputmarker": 0x44, "enemyput": 0x4A, "remove": 0x0D,
       "hold": 0x84, "holdend": 0x85, "turn": 0x47, "face": 0x48}


def rnd_pc(rnd, w, named=True):
    """A pc operand: a registered id, a party slot (-1, -2, -3), or another."""
    pool = [i for i in w["ids"] if i >= 0] + [-1, -2, -3, rnd.randrange(0, 12)]
    if not named:
        pool += [-1, rnd.randrange(-6, 0)]
    return rnd.choice(pool)


# What the cases reached, by check: filled from the game's side, printed after a bulk run and
# checked by the unit tests.
COVER = {}


def cover(label, tag, n=1):
    COVER.setdefault(label, {}).setdefault(tag, 0)
    COVER[label][tag] += n


def note(label, op, w, before, after, calls):
    cover(label, op + " area %d" % w["area"])
    for k, (a, b) in enumerate(zip(before["chars"], after["chars"])):
        for f in ("ai", "pos", "dirc", "act", "dead", "hold", "param2"):
            if a[f] != b[f]:
                cover(label, "%s %s changed" % (op, f))
    if before["lists"] != after["lists"]:
        cover(label, op + " lists changed")
    if before["target"] != after["target"]:
        cover(label, op + " target changed")
    for c in calls:
        cover(label, "%s call %s" % (op, c[0]))


def ev_check(checks, rnd, label, make):
    """One instruction on a random world: make(rnd, w) -> (probe op and args, Execute shorts) or None
    to draw again."""
    g = getattr(checks, "_evg", None) or EvGame(checks)
    checks._evg = g
    while True:
        w = rnd_world(rnd)
        r = make(rnd, w)
        if r is not None:
            break
    (op, args), shorts = r
    g.put(w)
    if op in ("holdop", "holdend"):
        running = args[-1]
        g.m.store(EVT + 0x7C0, 4, TSCB_OLD if running else 0)
        g.m.store(TSCB_OLD + 0x14, 4, 99)
        g.m.store(TSCB_OLD + 0x18, 4, 99)
    before = g.state(w)
    g.run(shorts)
    out = {"state": g.state(w), "calls": g.calls}
    note(label, op + (" neg" if op.endswith("marker") and args[1] < 0 else ""), w, before, out["state"], g.calls)
    if op in ("holdop", "holdend"):
        t = g.m.load(EVT + 0x7C0, 4)
        out["task"] = None if t == 0 else [s32(g.m.load(t + 0x14, 4)), s32(g.m.load(t + 0x18, 4))]
    req = "evp %s %s %s" % (op, " ".join(str(a) for a in args), ser_world(w))
    return req, out, label


def check_command(checks, rnd):
    def make(rnd, w):
        pc, on = rnd_pc(rnd, w), rnd.choice((0, 1, 1, 2))
        return ("command", [pc, on]), [OPS["command"], pc, on]
    return ev_check(checks, rnd, "command", make)


def check_walk_pos(checks, rnd):
    def make(rnd, w):
        pc = rnd_pc(rnd, w)
        x, y, z = (rnd.randrange(-3000, 3000) for _ in range(3))
        run = rnd.choice((0, 1))
        return ("walkpos", [pc, x, y, z, run]), [OPS["runpos" if run else "walkpos"], pc, x, y, z]
    return ev_check(checks, rnd, "walk_pos", make)


def check_walk_dir(checks, rnd):
    def make(rnd, w):
        pc = rnd_pc(rnd, w, named=False)
        rot, dist = rnd.randrange(-32768, 32768), rnd.choice((0, 140, rnd.randrange(-500, 500)))
        return ("walkdir", [pc, rot, dist]), [OPS["walkdir"], pc, rot, dist]
    return ev_check(checks, rnd, "walk_dir", make)


def found(w, marker):
    return any(p[2] == marker for p in w["positions"])


def check_walk_marker(checks, rnd):
    def make(rnd, w):
        pc = rnd_pc(rnd, w, named=False)
        if w["area"] == 0:
            marker = rnd.randrange(0, MARKERS)
        else:
            nums = [p[2] for p in w["positions"]]
            marker = rnd.choice(nums)
        if w["area"] in (1, 2) and not found(w, marker):
            return None
        return ("walkmarker", [pc, marker]), [OPS["walkmarker"], pc, marker]
    return ev_check(checks, rnd, "walk_marker", make)


def check_walk_char(checks, rnd):
    def make(rnd, w):
        pc = rnd_pc(rnd, w, named=False)
        ty = rnd.choice((2, 2, 3, 4, 5, 6, 0, 1, 7, 31, -1))
        codes = [c["id"] for c in w["chars"]] + [-1, -2, -3, rnd.randrange(0, 200)]
        code = rnd.choice(codes)
        if ty == 2 and code < -3:
            return None
        rot, dist = rnd.randrange(-32768, 32768), rnd.randrange(-400, 400)
        return ("walkchar", [pc, ty, code, rot, dist]), [OPS["walkchar"], pc, ty, code, rot, dist]
    return ev_check(checks, rnd, "walk_char", make)


def check_put(checks, rnd):
    def make(rnd, w):
        party = rnd.random() < 0.5
        pc = rnd_pc(rnd, w)
        if party:
            pc = rnd.choice([c["id"] for c in w["chars"]] + [0, 2, rnd.randrange(-5, 20)])
        x, y, z = (rnd.randrange(-3000, 3000) for _ in range(3))
        op = "partyput" if party else "put"
        return (op, [pc, x, y, z]), [OPS[op], pc, x, y, z]
    return ev_check(checks, rnd, "put", make)


def check_put_marker(checks, rnd):
    def make(rnd, w):
        party = rnd.random() < 0.5
        pc = rnd_pc(rnd, w)
        if party:
            pc = rnd.choice([c["id"] for c in w["chars"]] + [0, 2, rnd.randrange(-5, 20)])
        if w["area"] == 0:
            marker = rnd.randrange(0, MARKERS)
        else:
            marker = rnd.choice([p[2] for p in w["positions"]] + [rnd.randrange(-3, 12)])
        op = "partyputmarker" if party else "putmarker"
        return (op, [pc, marker]), [OPS[op], pc, marker]
    return ev_check(checks, rnd, "put_marker", make)


def check_turn(checks, rnd):
    def make(rnd, w):
        pc = rnd_pc(rnd, w)
        chg = rnd.choice((0, 0, 1, 64, 10, -5, 300))
        if rnd.random() < 0.5:
            dirc = rnd.randrange(-32768, 32768)
            return ("turn", [pc, dirc, chg]), [OPS["turn"], pc, dirc, chg]
        ty = rnd.choice((2, 2, 3, 4, 5, 6, 0, 1, 7, 31, -1))
        code = rnd.choice([c["id"] for c in w["chars"]] + [-1, -2, -3, rnd.randrange(0, 200)])
        return ("face", [pc, ty, code, chg]), [OPS["face"], pc, ty, code, chg]
    return ev_check(checks, rnd, "turn", make)


def check_enemy_put(checks, rnd):
    def make(rnd, w):
        enemy = rnd.choice([c["id"] for c in w["chars"]] + [rnd.randrange(0, 200)])
        posnum = rnd.choice([p[2] for p in w["positions"]] + [rnd.randrange(-3, 12)])
        return ("enemyput", [enemy, posnum]), [OPS["enemyput"], enemy, posnum]
    return ev_check(checks, rnd, "enemy_put", make)


def check_remove(checks, rnd):
    def make(rnd, w):
        ty = rnd.choice((5, 5, 6, 6, 0, 3, 7))
        code = rnd.choice([c["id"] for c in w["chars"]] + [rnd.randrange(0, 200)])
        return ("remove", [ty, code]), [OPS["remove"], ty, code]
    return ev_check(checks, rnd, "remove", make)


def check_hold_op(checks, rnd):
    def make(rnd, w):
        running = rnd.choice((0, 1))
        if rnd.random() < 0.5:
            ty, code = rnd.choice((5, 6, 7, 2, -1)), rnd.randrange(-5, 200)
            return ("holdop", [ty, code, running]), [OPS["hold"], ty, code]
        return ("holdend", [running]), [OPS["holdend"]]
    return ev_check(checks, rnd, "hold_op", make)


def check_battle_ready(checks, rnd):
    g = getattr(checks, "_evg", None) or EvGame(checks)
    checks._evg = g
    g.m.store(GAME + 0x60, 4, 0x45098000)
    g.run([0xA3])
    return "battleready", {"dist": g.m.load(GAME + 0x60, 4)}, "battle_ready"


# ccThEvHold over frames -----------------------------------------------------------------------

def rnd_deltas(rnd, w, kite):
    """Between two frames: holds cleared, positions moved, affectType 13, foes off and on the
    command list, the enemy list reordered."""
    out = []
    n = len(w["chars"])
    for _ in range(rnd.choice((0, 0, 1, 2, 4))):
        who = rnd.randrange(n)
        k = rnd.choice((0, 0, 1, 1, 2, 3, 4))
        if k == 0:
            out.append((who, 0, [0]))
        elif k == 1:
            out.append((who, 1, rnd_vec(rnd)))
        elif k == 2:
            out.append((who, 2, [rnd.choice((0, 13, 1))]))
        elif w["chars"][who]["ty"] & 0xE0:
            out.append((who, k, []))
    if rnd.random() < 0.3:
        out.append((kite, 1, rnd_vec(rnd)))
    return out


def apply_deltas(g, w, deltas, en):
    m = g.m
    for who, k, v in deltas:
        va = g.va(who)
        if k == 0:
            m.store(va + 0xA, 2, v[0])
        elif k == 1:
            g.vec(va + 0x40, v)
        elif k == 2:
            m.store(va + 0x9C, 2, v[0])
        elif k == 3:
            w["lists"][1] = [c for c in w["lists"][1] if c != who]
        elif k == 4:
            if not any(who in lst for lst in w["lists"]):
                w["lists"][1].append(who)
    g.put_lists(w["lists"])
    g.put_entries(en, w["npc"])


def check_hold_task(checks, rnd):
    g = getattr(checks, "_evg", None) or EvGame(checks)
    checks._evg = g
    m, sym = g.m, g.sym
    w = rnd_world(rnd, n_enemy=rnd.randrange(1, 6))
    for c in w["chars"]:
        if c["enemy"] and rnd.random() < 0.6:
            c["id"] = 130
    ty = rnd.choice((5, 5, 6, 6, 7, 7, 3))
    code = rnd.choice((130, 130, 131, 7, 44))
    if ty == 7 and rnd.random() < 0.8:
        # The event's boss: an enemy made a boss of that id, usually listed.
        foes = [i for i, c in enumerate(w["chars"]) if c["enemy"]]
        b = rnd.choice(foes)
        w["chars"][b].update(ty=0x80, id=code, kind="boss")
        if rnd.random() < 0.8 and b not in w["lists"][1]:
            w["lists"][1].append(b)
    boss_entry = rnd.choice((code, code, 5))
    boss_param = rnd.choice((-1, 0, 1, 1))
    n = rnd.choice((20, 40, 120))
    g.put(w)
    m.store(EVT + 8, 4, boss_entry)
    if boss_param >= 0:
        m.store(EVT + 0x7BC, 4, BOSST)
        m.store(BOSST + 0x14, 4, boss_param)
    m.mem[TSCB:TSCB + 0x40] = bytes(0x40)
    m.store(TSCB + 0x14, 4, ty & 0xFFFFFFFF)
    m.store(TSCB + 0x18, 4, code)
    frames_in, frames_out = [], []
    ens = []
    en = list(w["en"])
    for _ in range(n):
        d = rnd_deltas(rnd, w, w["kite"])
        if rnd.random() < 0.15:
            rnd.shuffle(en)
        if rnd.random() < 0.05 and en:
            en = en[1:]
        frames_in.append(d)
        ens.append(list(en))
    req = "evhold %d %d %d %d %d %s" % (ty, code, boss_entry, boss_param, n, ser_world(w))
    for d, e in zip(frames_in, ens):
        req += " %d" % len(d)
        for who, k, v in d:
            req += " %d %d %s" % (who, k, " ".join(str(x) for x in v))
        req += " %d %s" % (len(e), " ".join(str(x) for x in e))

    def breath(mm, *_):
        frames_out.append(g.state(w))
        k = len(frames_out)
        if k > n:
            raise eemu.Stop("done")
        apply_deltas(g, w, frames_in[k - 1], ens[k - 1])
        return 0

    a = sym("Breath__6ccTscbFi")
    saved = m.hooks.get(a)
    m.hooks[a] = breath
    try:
        m.call(g.hold_task, [TSCB], limit=50_000_000)
    except eemu.Stop as e:
        if str(e) != "done":
            raise
    finally:
        if saved is None:
            del m.hooks[a]
        else:
            m.hooks[a] = saved
    for a, b in zip(frames_out, frames_out[1:]):
        for x, y in zip(a["chars"], b["chars"]):
            if x["hold"] != 1 and y["hold"] == 1:
                cover("hold_task", "type %d hold set" % ty)
            if x["dirc"] != y["dirc"]:
                cover("hold_task", "type %d turned" % ty)
    cover("hold_task", "type %d" % ty)
    cover("hold_task", "frames compared", len(frames_out))
    return req, {"frames": frames_out}, "hold_task %d" % ty


# player_skill over frames ----------------------------------------------------------------------

def check_player_skill(checks, rnd):
    g = getattr(checks, "_evg", None) or EvGame(checks)
    checks._evg = g
    m, sym = g.m, g.sym
    w = rnd_world(rnd, n_enemy=rnd.randrange(1, 4))
    foes = [i for i, c in enumerate(w["chars"]) if c["enemy"]]
    w["target"] = rnd.choice(foes)
    for i in foes:
        w["chars"][i]["dead"] = rnd.choice((0, 0, 0, 0, 1)) if i == w["target"] else w["chars"][i]["dead"]
    g.put(w)
    n = rnd.choice((1, 5, 20, 60))
    assign = rnd.choice((0x40, 0x40, 0x20, -1, 0x4000))
    menu = [rnd.randrange(-1, 4), rnd.randrange(-1, 4), rnd.choice((0, 1)), rnd.choice((0, 1)), rnd.choice((0, 1))]
    m.store(SAVE + 0x840E, 2, assign & 0xFFFF)
    m.store(MENU + 0xC, 2, menu[0] & 0xFFFF)
    m.store(MENU + 0x10, 2, menu[1] & 0xFFFF)
    m.store(MENU + 0xF4, 2, menu[2])
    m.store(MENU + 0x102, 2, menu[3])
    m.store(g.target_fix, 4, menu[4])
    frames = []
    for f in range(n):
        push = rnd.choice((0, 0, 0x40, 0x20, 0x4000, rnd.getrandbits(16)))
        check = rnd.choice((0, 0, 0, 1, 3))
        dead = rnd.choice((0,) * 12 + (1, 2))
        new = rnd.choice([-1] * 10 + foes)
        frames.append((push, check, dead, new))
    req = "evskill %d %d %s %s" % (n, assign, " ".join(str(v) for v in menu), ser_world(w))
    for fr in frames:
        req += " %d %d %d %d" % fr
    out = []
    calls = []
    cur = {"frame": 0, "check": 0}

    def snap():
        t = m.load(g.target, 4)
        ti = g.ix(t)
        p2 = s32(m.load(t + 0x150, 4)) if ti >= 0 and w["chars"][ti]["enemy"] else 0
        out.append({"menu": [m.load(MENU + 0xC, 2, True), m.load(MENU + 0x10, 2, True), m.load(MENU + 0xF4, 2, True),
                             m.load(MENU + 0x102, 2, True), s32(m.load(g.target_fix, 4))],
                    "target": ti, "param2": p2, "calls": list(calls)})

    def breath(mm, *_):
        snap()
        k = len(out)
        if k > n:
            raise eemu.Stop("done")
        push, check, dead, new = frames[k - 1]
        cur["frame"], cur["check"] = k, check
        mm.store(SYS + 0x2D0, 4, push)
        if new >= 0:
            mm.store(g.target, 4, g.va(new))
        t = mm.load(g.target, 4)
        if t:
            mm.store(t + 8, 2, dead)
        return 0

    def skill_check(mm, ch, *_):
        calls.append(["ccSkillCheck", g.ix(ch)])
        return cur["check"]

    def skill_request(mm, ch, tgt, sid, *_):
        calls.append(["ccSkillRequest", g.ix(ch), g.ix(tgt), sid, cur["frame"]])
        return 0

    hooks = {"ccBreathThread__Fi": breath, "ccSkillCheck__FP6ccChar": skill_check,
             "ccSkillRequest__FP6ccCharP6ccChari": skill_request}
    saved = {}
    for name, h in hooks.items():
        a = sym(name)
        saved[a] = m.hooks.get(a)
        m.hooks[a] = h
    try:
        g.run([0x99])
        snap()
    except eemu.Stop as e:
        if str(e) != "done":
            raise
    finally:
        for a, h in saved.items():
            if h is None:
                del m.hooks[a]
            else:
                m.hooks[a] = h
    last = out[-1]
    cover("player_skill", "frames compared", len(out))
    cover("player_skill", "ended" if len(out) <= n else "still waiting")
    if any(c[0] == "ccSkillRequest" for c in last["calls"]):
        cover("player_skill", "attack requested")
    if len(out) == 1:
        cover("player_skill", "dead at once")
    return req, {"frames": out}, "player_skill"


EVENT = {"command": check_command, "walk_pos": check_walk_pos, "walk_dir": check_walk_dir,
         "walk_marker": check_walk_marker, "walk_char": check_walk_char, "put": check_put,
         "put_marker": check_put_marker, "turn": check_turn, "enemy_put": check_enemy_put, "remove": check_remove,
         "hold_op": check_hold_op, "hold_task": check_hold_task, "player_skill": check_player_skill,
         "battle_ready": check_battle_ready}


@unittest.skipUnless(rs.READY, "needs the extracted disc and cargo")
class EventAgainstGame(rs.Against):
    TABLE = EVENT
    CASES = 150

    def test_command(self):
        self.check("command", 1)

    def test_walks(self):
        self.check("walk_pos", 2)
        self.check("walk_dir", 3)
        self.check("walk_marker", 4)
        self.check("walk_char", 5)

    def test_puts(self):
        self.check("put", 6)
        self.check("put_marker", 7)
        self.check("enemy_put", 8)
        self.check("turn", 14)

    def test_remove_and_hold(self):
        self.check("remove", 9)
        self.check("hold_op", 10)

    def test_hold_task(self):
        self.check("hold_task", 11)

    def test_player_skill(self):
        self.check("player_skill", 12)

    def test_battle_ready(self):
        self.check("battle_ready", 13)


def print_cover():
    for label in sorted(COVER):
        print(label + ":")
        for tag, n in sorted(COVER[label].items()):
            print("    %-40s %d" % (tag, n))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        import atexit
        atexit.register(print_cover)
    rs.main(list(EVENT), 7000, EVENT)
