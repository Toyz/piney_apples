#!/usr/bin/env python3
"""crates/piney-battle's entry control, magic circle and race constructors
(src/entry.rs, src/races.rs) against the game's own entctrl.cpp,
gmcircle.cpp and enemy*.cpp run in the EE interpreter (the Rust machine).

Each check lays a scene out in the interpreter's memory: g_entCtrl and its
four lists (enemies, magic circles, gimmicks, NPCs), the objects (Kite, and
ccEnemy, ccMagicCircle, ccGimmick objects at OBJS + 0x8000 * i with their
particles' effects and animation players), the command lists, the
registered rows, game, worldman, plw, the save's event entries, fountains
and counters, both generators (ccRand's Mersenne Twister and ccRandS's
lastRnd), then runs one game function natively and sends the same scene to
battle_probe's `spawn` request. What is not logic is stubbed: the player's
frame (W2PPos/P2WPos: an offset on the ground), ccLandHitCheck,
checkHitResultAttlibute, ccCheckCameraDeg, ccGetCameraTransparency and
ccAnm::_AnimateForward answer from scripts; SetAnm, SetHitSW/HitDisable and
those queries are recorded as the world's calls; sounds, the trap-removal
effect, ccStartThread, the draws (SetActiveLayer, ccAnm::Draw, ccEff::Draw),
the weapon and dust controllers, palette swaps and the destructors' hooks
are recorded as outputs; every object's main but the magic circle's
(ccEnemy::main, the gimmicks' and NPCs') is stubbed to record the call and
answer from a script, and the gimmicks' and NPCs' constructors make a bare
ccGimmick with the entry copied in (the probe's seam does the same) - but
the classes piney-battle holds, whose constructors run natively (ccGimBox,
ccGimIdol, ccGimSymbol, ccGimFood: their own members compared) and whose
mains run natively in gim_frame (ccGimBox, ccGimIdol, ccGimFood).
Compared: the control and its lists, every object (ccChar, ccEntryObj,
ccEnemy with the race's members, the circle and its 128 particles), the
command lists, the calls and outputs in order, ccRand's state, lastRnd, the
save, and each operation's own results.

    python3 tools/test_battle_spawn_rs.py            the unit tests
    python3 tools/test_battle_spawn_rs.py bulk N     N cases of every check

Checks (SPAWN maps a name to fn(checks, rnd) -> (request, game, label)):
  routine        ccEntryObj::routine (0x0042fa60) on an enemy, a circle, a
                 gimmick
  check          ccEntryCtrl::entryObjectCheck (0x00430530)
  entry          entryObject(ep) (0x00430c90), entryObject(ep, n)
                 (0x004307f0), entryEnemy, entryGimmick, entryNpc,
                 entryMagicCircle (0x004313f0), initObject, the race and
                 circle constructors
  race           every row of races B, G, K, P, V through ccEntryRaceTbl's
                 ccEntryEnemyX (ccEnemy::ccEnemy, initEnemy, the race's
                 constructor, checkGold)
  circle_object  entryCircleObject (0x00430360)
  enemy_object   entryEnemyObject (0x00430250)
  init_object    initObject (0x00430ec0)
  delete         deleteEnemy, deleteMagicCircle, deleteGimmick, deleteNpc
  active         ccCheckActiveEnemy, ccCheckActiveObject(), (floor, block)
  circle_main    ccMagicCircle::main (0x00455b60) with createPart, setPart,
                 ccMcPart::main and fade
  frame          ccThEntryCtrl's per-frame loop (0x00431b08), 20-90 frames,
                 Kite walking up to magic circles and away
  world_mc       WORLD::SetMagicCircle (0x005ab610) on random fields
  dungeon_mc     DUNGEON::SetMagicCircle (0x005bf590)
  event_mc       ccEntryEventMng's entry_mc (main 0x001b62e0) and an event
                 enemy's entry
  entry_gimmick  WORLD_MAN::EntryGimmick (main 0x001a1f20)
  restore        restoreEntry (0x00431070)
  leave          ccThEntryCtrlDelete (0x00431d10): what survives leaving
                 an area
  register       ccInitRegisterEnemy, ccRegisterEnemyList,
                 ccRegisterEnemyOne, ccAnalyzeEnemyList
  save           ccSaveData::CheckEventEntry, CheckFountain,
                 ClearEventEntry, deleteEventEntry (0x00430200);
                 ccCheckDustColor (0x0043a250); ccRandS (main 0x001d9c10)
  frame_enemies  ccThEntryCtrl's frames (100-200) in a field or a dungeon
                 with the enemies' own ccEnemy::main (0x00432cd0) and the
                 races' vtable functions, EntryAffect and the affect
                 functions all run natively, against entry::EnemySeam
                 (enemy_motion::Motion::enemy_main). Kite and one or two
                 party members (tools/test_battle_enemy_motion_rs.py's
                 party: ccSpcParam, affect functions, the words their
                 influence reads) walk up to a magic circle from beyond its
                 reach and linger; the circle opens and gives enemies of the
                 first field and dungeon's rows (and an event may have
                 placed some), which notice the party, chase and attack
                 (0x8005 notes land through ccSkillDamage and the party's
                 Influence); between frames the harness kills some (HP 0:
                 dying, the corpse taken away, entryEnemyObject's treasure
                 box), drains some (the drained form spawned by
                 entryDrainEnemy) and gives some an affect. The world is
                 the enemy motion harness's script (land, collision, walls,
                 the camera shake's range and transparency, the animation
                 players' results, frames and notes), drawn as the game
                 asks; the presentation and ccSkillRequest,
                 ccExpDistributor, effSkillStart are recorded. Compared after
                 every frame: the control and its lists, every object with
                 every character in the enemy motion harness's full format
                 and every ccEnemy field with the body hit, the command
                 lists, the calls in order, ccRand, ccRandS, newlib's
                 rand(), the save's counters and the enemy book.
"""

import json
import math
import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import volume  # noqa: E402

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle as TB  # noqa: E402
import test_battle_rs as rs  # noqa: E402
import test_battle_enemy_ai_rs as EA  # noqa: E402

F_ONE = 0x3F800000
ENEMY_TBL, ROW = inf_va(0x005F1E70), 0x1C0
GIMMICK_TBL, NPC_TBL = inf_va(0x0061E0F0), inf_va(0x00619460)
RACE_TBL = inf_va(0x005F1D60)
MT, MTI = inf_va(0x003FF400), inf_va(0x00377FD0)
CCENEMY_VT = inf_va(0x003760C0)
G_VT = inf_va(0x00376250)
# each race's vtable: ccEnemy1-4, A-I, K, L, P, S-W, Z (0x00376110 on, 0x20 apart)
RACE_VTS = {inf_va(0x00376110) + 0x20 * i: k for i, k in enumerate(
    ("1", "2", "3", "4", "A", "B", "C", "D", "E", "F", "G", "H", "I", "K", "L", "P", "S", "T", "U", "V", "W",
     "Z"))}
# the races races.rs constructs: all (but ccEnemyL's type 3, rows 203-206)
BUILT = tuple(range(22))
ELG = range(203, 207)
# where a race keeps its wc and dc (ccEnemyWeaponCtrl*, ccEnemyDustCtrl*)
CTRL_AT = {"G": (0x374, 0x378), "C": (0x348, 0x34C)}
ENEMY_INFLUENCE = inf_va(0x00432840)
CIRCLE_FUNC = inf_va(0x00455820)            # ccEntryGimCircle

OBJS, STRIDE = 0x01300000, 0x8000   # pre-placed objects; a circle's effects at +0x3a00, its anm at +0x7400
CTRL = 0x013C0000
KITE_BASE = 0x013C1000
WMAN = 0x013C2000
EVT = 0x013C3000
WP = 0x013C4000                     # worldman->wordparam
VT_GIM, VT_NPC = 0x013C5000, 0x013C5100
STUB_GIM, STUB_NPC = 0x013C5200, 0x013C5300
TRAMP, TSCB = 0x013C6000, 0x013C6100
EP = 0x013C7000                     # an entry param handed to the functions
ENTRY = 0x013C7100                  # a ccEntry for the race constructors
CHUNKS = 0x013D0000                 # fake chunk addresses, 16 bytes a name
WORLDO = 0x013E0000                 # WORLD (0x61e0)
NEWR, NEWR_END = 0x01400000, 0x01800000
FIELD = 0x01800000                  # FIELD (0xd0000)
DUNG = 0x01900000                   # DUNGEON (0x33520)
GIMPOS, EDIT, EDIT_ROWS = 0x01940000, 0x01950000, 0x01950100
ROTATE = 0x01958000                 # from Mutation on DUNGEON's rotate is allocated

SAVE = 0x01010000
EV_ENTRY, FOUNTAIN = 0x40CC, 0x65DC
KILL_COUNT, KILL_AREA = 0x68B7, 0x69F0
COUNTERS = (0x6864, 0x6866, 0x6868)
PATNUM = 6
PARTS = 128
OBJ_SIZES = {0x380, 0x350, 0x3B0, 0x39F0}   # ccEnemyG, the other races, ccEnemyL, the circle
GIM_SIZE = 0x1F0                    # ccGimBox, ccGimIdol
FOOD_SIZE = 0x240                   # ccGimFood
ETC_SIZE = 0x270                    # ccGimEtc
SYMBOL_SIZE = 0x710                 # ccGimSymbol
BOX_FUNC, IDOL_FUNC, FOOD_FUNC, SYMBOL_FUNC = inf_va(0x00453420), inf_va(0x00459620), inf_va(0x00458220), inf_va(0x0045A010)
BOX_VT, IDOL_VT, FOOD_VT, SYMBOL_VT = inf_va(0x003763E0), inf_va(0x00376440), inf_va(0x00376420), inf_va(0x00376450)
ETC_FUNC, ETC_VT = inf_va(0x00456450), inf_va(0x00376400)
ETC_LIGHTS = 0x7600                 # a pre-placed spring's two omni lights (0xd0 each)
FOUNTAIN_COUNT = 0x676C
IBD_UNSET = 0x7F7F7F7F              # game.inBattleDist before an operation
BOXRAD_VT, BOX_RAD_INFO = inf_va(0x003760F0), inf_va(0x005D5990)
DRAWENV_ACTIVE = inf_va(0x003788C4)         # ccDrawEnv::active
DRAWENV = 0x013C8000                # a ccDrawEnv for the boxes' draws
RAD_AT, PARTS_AT, LIGHT_AT = 0x3000, 0x3100, 0x3E00   # a pre-placed box's rays
GIMRAD_VT, GIM_RAD_INFO = inf_va(0x00376410), inf_va(0x005EAF40)
RRAD_AT, RPARTS_AT, RLIGHT_AT = 0x5000, 0x5100, 0x6C00   # a pre-placed radiator's 32 rays
OMNI_VT = inf_va(0x00375A50)
HAND, KITE_AI = 0x013C9000, 0x013C9100    # Kite's objHandR (a ccObj) and AI (manualSW)
GIM_ANM = 0x7400                    # its animation player


# The destructors (ccEntryObj +0x1c8) by Infection's address, as the port
# names them
DESTS = (
    ("ccDestEnemyG__FP10ccEntryObj", 0x00446010),
    ("ccDestEnemyP__FP10ccEntryObj", 0x0044E4F0),
    ("ccDestEnemyK__FP10ccEntryObj", 0x0044A780),
    ("ccDestEnemyV__FP10ccEntryObj", 0x00451400),
    ("ccDestEnemyB__FP10ccEntryObj", 0x00442130),
    ("ccDestEnemy1__FP10ccEntryObj", 0x0043ED40),
    ("ccDestEnemy2__FP10ccEntryObj", 0x0043F960),
    ("ccDestEnemy3__FP10ccEntryObj", 0x00440320),
    ("ccDestEnemy4__FP10ccEntryObj", 0x00440BB0),
    ("ccDestEnemyA__FP10ccEntryObj", 0x004415A0),
    ("ccDestEnemyC__FP10ccEntryObj", 0x00442AD0),
    ("ccDestEnemyD__FP10ccEntryObj", 0x00443930),
    ("ccDestEnemyE__FP10ccEntryObj", 0x00444860),
    ("ccDestEnemyF__FP10ccEntryObj", 0x004454F0),
    ("ccDestEnemyH__FP10ccEntryObj", 0x00449320),
    ("ccDestEnemyI__FP10ccEntryObj", 0x00449EB0),
    ("ccDestEnemyS__FP10ccEntryObj", 0x0044EEC0),
    ("ccDestEnemyT__FP10ccEntryObj", 0x0044F990),
    ("ccDestEnemyU__FP10ccEntryObj", 0x00450380),
    ("ccDestEnemyW__FP10ccEntryObj", 0x00451FE0),
    ("ccDestEnemyZ__FP10ccEntryObj", 0x00452A50),
    ("ccDestEnemyL__FP10ccEntryObj", 0x0044B180),
    ("ccDestMagicCircle__FP10ccEntryObj", 0x00455870),
    ("ccDestGimBox__FP10ccEntryObj", 0x00453470),
    ("ccDestGimIdol__FP10ccEntryObj", 0x00459670),
    ("ccDestGimFood__FP10ccEntryObj", 0x00458270),
    ("ccDestGimEtc__FP10ccEntryObj", 0x004564A0),
)
# Every destructor (ccDest*) Infection names, for the address mapping.
DEST_ADDRS = {f.value for f in volume.program(volume.INF_ELF, "gcmn").functions() if f.name.startswith("ccDest")}


def DEST_BACK():  # noqa: N802
    """The volume's destructor addresses to Infection's."""
    global _DEST_BACK
    if _DEST_BACK is None:
        _DEST_BACK = {inf_va(a): a for a in DEST_ADDRS}
    return _DEST_BACK


_DEST_BACK = None

class FrameDone(Exception):
    pass


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def rf(rnd, lo, hi):
    return fb(rnd.uniform(lo, hi))


def digest(words):
    """The probe's parts_hash: the sum of each word times its odd position
    weight, modulo 2^64."""
    return sum((w & 0xFFFFFFFF) * (2 * i + 1) for i, w in enumerate(words)) & 0xFFFFFFFFFFFFFFFF


BOX_CLIPS = {0: "ANM_xgbbox00", 1: "ANM_xgbbox00", 2: "ANM_xgbcont0", 3: "ANM_xgbcont0", 4: "ANM_xgbbutt0",
             5: "ANM_xgbbutt0", 6: "ANM_xgvirus0", 7: "ANM_xgbcont0", 8: "ANM_xgbbutt0", 9: "ANM_xgbpot1",
             10: "ANM_xgbpot0", 11: "ANM_xgbske1", 12: "ANM_xgbske0", 13: "ANM_xgbegg0", 14: "ANM_xgbegg1"}
IDOL_CLIPS = "anut fnut wnut lnut enut dnut tnut".split()


def gim_clip(gid, act):
    """The clip a pre-placed box or idol plays (only whether one is set
    matters to the checks)."""
    if gid in BOX_CLIPS:
        return BOX_CLIPS[gid]
    if 38 <= gid <= 44:
        return f"ANM_xgs{IDOL_CLIPS[gid - 38]}{0 if act == 0 else 1}"
    if 22 <= gid <= 37:
        return f"ANM_xgfood{gid - 22:02x}a"
    return "ANM_none"


class Harness:
    """The entry control's scene on the machine, its hooks, and the
    probe's request for the same scene."""

    def __init__(self, checks):
        self.c = checks
        g = checks.game
        self.g, self.m = g, g.m
        self.ea = EA.harness(checks)
        m, sym = self.m, g.sym
        self.sym = sym
        self.race_num = [m.load(RACE_TBL + 12 * i + 4, 4) for i in range(22)]
        self.race_first = []
        t = 0
        for n in self.race_num:
            self.race_first.append(t)
            t += n
        self.race_funcs = []
        for i in range(303):
            r = next((k for k in range(22) if self.race_first[k] <= i < self.race_first[k] + self.race_num[k]), 21)
            self.race_funcs.append(m.load(RACE_TBL + 12 * r, 4))
        self.gim_funcs = ({m.load(GIMMICK_TBL + 0x70 * i + 0x2C, 4) for i in range(45)}
                          - {CIRCLE_FUNC, BOX_FUNC, IDOL_FUNC, FOOD_FUNC, ETC_FUNC, SYMBOL_FUNC, 0})
        self.npc_funcs = {m.load(NPC_TBL + 0x70 * i + 0x2C, 4) for i in range(175)} - {0}
        self.gim_esize = [s32(m.load(GIMMICK_TBL + 0x70 * i + 0x30, 4)) for i in range(45)]
        self.affect_funcs = {1: sym("ccEnemyInfluence__FP6ccChar"), 2: sym("Influence__8ccFellowFv"),
                             3: sym("Influence__FP6ccChar")}
        self.npc_esize = [s32(m.load(NPC_TBL + 0x70 * i + 0x30, 4)) for i in range(175)]
        self.esize = [m.load(ENEMY_TBL + ROW * i + 0x70, 4) for i in range(303)]
        self.addr = {n: sym(n) for n in (
            "g_entCtrl", "game", "saveData", "worldman", "eventMng", "plw", "ccRegisterEnemyTbl",
            "ccRegisterEnemyNum", "ccRegisterDrainNum", "ccRegisterEnemyRange", "lastRnd", "seed", "randcnt",
            "g_entryList")}
        self.gb_cmnd = (g.cmnd_pc, g.cmnd_ene, g.cmnd_obj)
        # the trampoline into ccThEntryCtrl's frame loop: s0 = the control,
        # s1 = its task block, then the loop's head (move a0, s1; Breath).
        code = [0x3C100000 | (CTRL >> 16), 0x36100000 | (CTRL & 0xFFFF),
                0x3C110000 | (TSCB >> 16), 0x36310000 | (TSCB & 0xFFFF),
                0x08000000 | ((inf_va(0x00431B08) >> 2) & 0x3FFFFFF), 0]
        for k, w in enumerate(code):
            m.store(TRAMP + 4 * k, 4, w)
        m.store(VT_GIM + 8, 4, STUB_GIM)
        m.store(VT_NPC + 8, 4, STUB_NPC)
        # the enemy AI harness's reads of pointers: our objects by index
        self.ea.index = lambda a: -1 if a == 0 else self.idx.get(a & 0xFFFFFFFF, a)
        self.motion = False
        self.reset_state()
        self.install()

    # state ---------------------------------------------------------------------------
    def reset_state(self):
        self.g.names = {}
        self.idx = {}           # object address -> index
        self.kinds = []         # index -> kind (0 plain, 1 enemy, 2 circle, 3 gimmick, 4 npc)
        self.addrs = []
        self.deleted = set()
        self.new_at = NEWR
        self.chunks = {}
        self.chunk_names = {}
        self.wcalls, self.scalls, self.outs = [], [], []
        self.k = [0, 0]
        self.land = []
        self.attr = []
        self.camdeg = []
        self.camtr = []
        self.fwd = []
        self.emain, self.gmain, self.nmain = [], [], []
        self.breaths = 0

    def index_of(self, a):
        return self.idx.get(a & 0xFFFFFFFF, a)

    def add_obj(self, va, kind):
        i = len(self.addrs)
        self.idx[va] = i
        self.addrs.append(va)
        self.kinds.append(kind)
        self.g.names[va] = f"c{i}"
        return i

    def owner_anm(self, anm):
        for i, va in enumerate(self.addrs):
            if self.m.load(va + 0xD4, 4) == anm:
                return i, 0
            if self.kinds[i] == 1 and self.m.load(va + 0x1BC, 4) == anm:
                return i, 1
        return anm, -1

    def owner_eff(self, eff):
        for i, va in enumerate(self.addrs):
            if self.kinds[i] != 2:
                continue
            for k in range(PARTS):
                if self.m.load(va + 0x1F0 + 0x70 * k + 0x68, 4) == eff:
                    return i, k
        return eff, -1

    def chunk(self, name):
        if name not in self.chunks:
            a = CHUNKS + 16 * (len(self.chunks) + 1)
            self.chunks[name] = a
            self.chunk_names[a] = name
        return self.chunks[name]

    # hooks ---------------------------------------------------------------------------
    def install(self):
        m, sym = self.m, self.sym
        H = self

        def pop(lst, dflt=0):
            return lst.pop(0) if lst else dflt

        def vec(a):
            return [m.load(a + 4 * k, 4) for k in range(4)]

        def w2p(mm, this, out, inp, *_):
            v = vec(inp)
            vals = [eemu.f_sub(v[0], H.k[0]), eemu.f_sub(v[1], H.k[1]), v[2], v[3]]
            for k in range(4):
                mm.store(out + 4 * k, 4, vals[k])
            return 0

        def p2w(mm, this, out, inp, *_):
            v = vec(inp)
            vals = [eemu.f_add(v[0], H.k[0]), eemu.f_add(v[1], H.k[1]), v[2], v[3]]
            for k in range(4):
                mm.store(out + 4 * k, 4, vals[k])
            return 0

        def tw2p(mm, out, inp, *_):
            return w2p(mm, 0, out, inp)

        def tp2w(mm, out, inp, *_):
            return p2w(mm, 0, out, inp)

        def land(mm, pos, mask, *_):
            H.wcalls.append(["land", vec(pos), mask])
            v = pop(H.land)
            mm.f[0] = v
            return v

        def attr(mm, *_):
            H.wcalls.append(["attr"])
            return pop(H.attr)

        def set_hit_sw(mm, this, sw, *_):
            cur = mm.load(this, 4)
            if cur == sw:
                return 0
            if sw == 0:
                H.wcalls.append(["hit", H.index_of(this - 0x160), 0])
                mm.store(this, 4, 0)
            elif cur == 0:
                H.wcalls.append(["hit", H.index_of(this - 0x160), 1])
                mm.store(this, 4, 1)
            return 0

        def hit_disable(mm, this, *_):
            H.wcalls.append(["hit", H.index_of(this - 0x160), 0])
            mm.store(this, 4, 0)
            return 0

        def hit_enable(mm, this, *_):
            H.wcalls.append(["hit", H.index_of(this - 0x160), 1])
            mm.store(this, 4, 1)
            return 0

        def camdeg(mm, pos, deg, *_):
            H.wcalls.append(["camdeg", vec(pos), s16(deg)])
            return int(pop(H.camdeg, False))

        def camtr(mm, pos, *_):
            H.wcalls.append(["camtr", vec(pos), [mm.f[12], mm.f[13], mm.f[14], mm.f[15]]])
            v = pop(H.camtr)
            mm.f[0] = v
            return v

        def chunk(mm, stream, name, *_):
            n = bytes(mm.mem[name:name + 64]).split(b"\0")[0].decode("latin-1")
            return H.chunk(n)

        def set_anm(mm, anm, chunk_, *_):
            who, slot = H.owner_anm(anm)
            H.wcalls.append(["anim_set", who, slot, H.chunk_names.get(chunk_, "?")])
            mm.store(anm + 0xAC, 4, chunk_ or 1)
            return 0

        def fwd(mm, anm, step, *_):
            who, slot = H.owner_anm(anm)
            H.wcalls.append(["anim_forward", who, slot, step & 0xFFFF])
            return pop(H.fwd) & 0xFFFFFFFF

        def anm_ct(mm, this, *_):
            mm.mem[this:this + 0x110] = bytes(0x110)
            mm.store(this + 0x9C, 2, 256)
            return this

        def ret0(mm, *_):
            return 0

        def reta0(mm, a0, *_):
            return a0

        def clut(mm, clump, *_):
            for i, va in enumerate(H.addrs):
                if mm.load(va + 0xD0, 4) == clump:
                    H.outs.append(["clut", i])
            return 0

        def anm_draw(mm, anm, *_):
            who, _slot = H.owner_anm(anm)
            if 0 <= who < len(H.kinds) and H.kinds[who] == 3 and mm.load(H.addrs[who] + 0x1CC, 4) == ETC_VT:
                H.outs.append(["etc_draw", who])
                return 0
            if 0 <= who < len(H.kinds) and H.kinds[who] == 3:
                env = mm.load(DRAWENV_ACTIVE, 4)
                H.outs.append(["gim_draw", who, mm.load(anm + 0x88, 4), mm.load(env + 0xA4, 1),
                               mm.load(env + 0xA0, 4)])
                return 0
            H.outs.append(["circle_draw", who, mm.load(anm + 0x88, 4)])
            return 0

        def eff_init(mm, eff, *_):
            mm.store(eff + 0x30, 2, PATNUM)
            return 0

        def eff_draw(mm, eff, pat, *_):
            who, k = H.owner_eff(eff)
            H.outs.append(["part_draw", who, k, mm.load(eff + 0x34, 4), pat & 0xFFFF])
            return 0

        def layer(mm, wm, lay, *_):
            H.outs.append(["layer", s32(lay)])
            return 0

        def weapon(mm, this, n, info, obj, *_):
            H.outs.append(["weapon", H.index_of(obj), info, s32(n)])
            mm.store(this, 4, s32(n) & 0xFFFFFFFF)
            mm.store(this + 4, 4, info)
            return this

        def dust(mm, this, n, info, obj, *_):
            H.outs.append(["dust", H.index_of(obj), info, s32(n)])
            mm.store(this, 4, s32(n) & 0xFFFFFFFF)
            mm.store(this + 4, 4, info)
            return this

        def se(mm, n, pos, *_):
            H.outs.append(["se", s32(n), vec(pos)])
            return 0

        def trap(mm, pos, a, b, *_):
            H.outs.append(["remove_trap", vec(pos), s32(a), s32(b)])
            return 0

        def rad_owner(rad):
            for i, va in enumerate(H.addrs):
                if H.kinds[i] != 3:
                    continue
                vt = mm_load(va + 0x1CC)
                if (vt == BOX_VT and mm_load(va + 0x1E0) == rad) or (vt == IDOL_VT and mm_load(va + 0x1E8) == rad) \
                        or (vt == ETC_VT and mm_load(va + 0x268) == rad):
                    return i
            return -1

        def light_owner(lgt):
            for i, va in enumerate(H.addrs):
                if H.kinds[i] != 3:
                    continue
                r = H.rad_of(va)
                if r and mm_load(r + 0x58) == lgt:
                    return i
            return -1

        def etc_light(lgt):
            """(object, k) of a spring's light, or None."""
            for i, va in enumerate(H.addrs):
                if H.kinds[i] == 3 and mm_load(va + 0x1CC) == ETC_VT:
                    for k in range(2):
                        if mm_load(va + 0x260 + 4 * k) == lgt:
                            return i, k
            return None

        def add_grp(mm, g_, l_, *_):
            e = etc_light(l_)
            if e is None:
                H.outs.append(["light_on", light_owner(l_)])
            else:
                H.outs.append(["etc_light", e[0], e[1], mm.load(l_ + 0xB0, 4)] + H.get(l_ + 0xC0, "<4I")
                              + H.get(l_ + 0x70, "<3I"))
            return 0

        def del_grp(mm, g_, l_, *_):
            e = etc_light(l_)
            H.outs.append(["light_off", light_owner(l_)] if e is None else ["etc_light_off", e[0], e[1]])
            return 0

        def etc_eff(row):
            def f(mm, pos, sw, *_):
                H.outs.append(["etc_effect", row, H.index_of(sw - 0x1E0), vec(pos)])
                return 1
            return f

        def etc_matrix(mm, anm, pos, rot, scale, *_):
            # A spring's matrix; a food's draw sets one too, which is not kept.
            who, slot = H.owner_anm(anm)
            if slot >= 0 and m.load(H.addrs[who] + 0x1CC, 4) == ETC_VT:
                H.outs.append(["etc_matrix", who, vec(pos), vec(rot), vec(scale)])
            return 0

        def flash(mm, this, t, colour, *_):
            H.outs.append(["flash", s32(t), colour & 0xFFFFFFFF])
            return 0

        def flash2(mm, this, t0, t1, colour, *_):
            H.outs.append(["flash2", s32(t0), s32(t1), colour & 0xFFFFFFFF])
            return 0

        def dust_ring_at(mm, pos, n, life, tex, *_):
            H.outs.append(["dust_ring_at", vec(pos), mm.f[12] & 0xFFFFFFFF, mm.f[13] & 0xFFFFFFFF, s32(n), s32(life),
                           s32(tex)])
            return 0

        def mm_load(a):
            return m.load(a, 4)

        def set_color(mm, out, rgb, *_):
            mm.store(out, 4, rgb)
            for k in (1, 2, 3):
                mm.store(out + 4 * k, 4, 0)
            return 0

        def rays(mm, rad, part, *_):
            mm.store(rad + 0x44, 4, (mm.load(rad + 0x44, 4) + 1) & 1)
            n = s16(mm.load(rad + 4, 2))
            for k in range(n):
                for v in range(4):
                    a = part + 0xD0 * k + 0x20 + 0x30 * v
                    tw2p(mm, a, a)
                    tp2w(mm, a, a)
            H.outs.append(["rays", rad_owner(rad)])
            return 0

        def rec_out(name, *conv):
            def f(mm, *args):
                H.outs.append([name] + [c(mm, a) for c, a in zip(conv, args)])
                return 0
            return f

        def c_int(mm, a):
            return s32(a)

        def c_vec(mm, a):
            return vec(a)

        def c_obj(mm, a):
            return H.index_of(a)

        def trap_damage(mm, this, t, sid, *_):
            H.wcalls.append(["trap_damage", H.index_of(t) if t else -1, s32(sid)])
            return 0

        def trap_skill(mm, cp, tp, sid, flag, *_):
            H.wcalls.append(["trap_skill", H.index_of(tp) if tp else -1, s32(sid), s32(flag)])
            return 0

        def statue(mm, pos, sw, *_):
            H.outs.append(["statue_of_god", H.index_of(sw - 0x1E0), vec(pos)])
            return 1

        def dust_ring(mm, ch, ofs, *_):
            H.outs.append(["dust_ring", H.index_of(ch), vec(ofs)])
            return 0

        def idol_draw(mm, ch, *_):
            what = "food_draw" if mm.load(ch + 0x1CC, 4) == FOOD_VT else "idol_draw"
            H.outs.append([what, H.index_of(ch)])
            return 1

        def se_note(mm, n, pos, note, *_):
            H.outs.append(["se_note", s32(n), vec(pos), note & 0xFF])
            return 0

        def thread(mm, *_):
            H.outs.append(["area_cleared"])
            return 0

        def new(mm, n, *_):
            a = H.new_at
            H.new_at = (H.new_at + n + 15) & ~15
            if H.new_at > NEWR_END:
                raise eemu.Stop("the harness's heap ran out")
            mm.mem[a:a + n] = bytes(n)
            if n in OBJ_SIZES:
                H.add_obj(a, 2 if n == 0x39F0 else 1)
            elif n in (GIM_SIZE, FOOD_SIZE, ETC_SIZE, SYMBOL_SIZE):
                H.add_obj(a, 3)
            return a

        def new_arr(mm, n, *_):
            a = H.new_at
            H.new_at = (H.new_at + n + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            return a

        def delete(mm, a, *_):
            if a in H.idx:
                H.deleted.add(H.idx[a])
            return 0

        def dest(mm, obj, *_):
            H.outs.append(["destroyed", H.index_of(obj), H.cur_dest])
            return 0

        def main_enemy(mm, obj, *_):
            H.scalls.append(["enemy_main", H.index_of(obj)])
            return int(pop(H.emain, False))

        def main_gim(mm, obj, *_):
            H.scalls.append(["gimmick_main", H.index_of(obj)])
            return int(pop(H.gmain, False))

        def main_npc(mm, obj, *_):
            H.scalls.append(["npc_main", H.index_of(obj)])
            return int(pop(H.nmain, False))

        def maker(table, vt, kind):
            def f(mm, entry, *_):
                ep = mm.load(entry + 0xC, 4)
                a = H.new_at
                H.new_at = (H.new_at + 0x200 + 15) & ~15
                mm.mem[a:a + 0x200] = bytes(0x200)
                i = H.add_obj(a, kind)
                ent = H.read_ent(ep)
                H.scalls.append(["make_gimmick" if kind == 3 else "make_npc", i, ent])
                row = entry - 0x28
                H.put_bare(a, row, ent, vt)
                return a
            return f

        def breath(mm, *_):
            H.breaths += 1
            if H.breaths > 1:
                raise FrameDone()
            return 0

        def init_breath(mm, this, info, *_):
            who = max(i for i, k in enumerate(H.kinds) if k == 1)
            H.outs.append(["breath", who, info])
            return 0

        def change_target(mm, a, *_):
            H.outs.append(["change_target", H.index_of(a)])
            return 0

        hooks = {
            "W2PPos__8ccPlayerFPfPf": w2p, "P2WPos__8ccPlayerFPfPf": p2w,
            "ccTransPosW2P__FPfPf": tw2p, "ccTransPosP2W__FPfPf": tp2w,
            "ccLandHitCheck__FPfUi": land, "checkHitResultAttlibute__Fv": attr,
            "SetHitSW__9ccCharHitFi": set_hit_sw, "HitDisable__9ccCharHitFv": hit_disable,
            "HitEnable__9ccCharHitFv": hit_enable,
            "ccCheckCameraDeg__FPfs": camdeg, "ccGetCameraTransparency__FPfffff": camtr,
            "GetChunkAdrsF__8ccStreamFPCci": chunk, "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm,
            "_AnimateForward__5ccAnmFUi": fwd, "__ct__5ccAnmFv": anm_ct,
            "ApplyClump__5ccAnmFP7ccClumpP8ccStream": ret0, "Init__7ccClumpFP12ccClumpChunk": ret0,
            "__ct__7ccCoordFv": reta0, "SetFogSw__7ccClumpFi": ret0,
            "ccEntryChangeCLUT__FP7ccEntryP7ccClump": ret0, "SetShadowSw__5ccAnmFi": ret0,
            "Duplicate__7ccClumpFUi": ret0,
            "ChangeClut__7ccClumpFP11ccClutChunkP15ccMaterialChunk": clut,
            "SetMatrix_PosRotZYX__7ccCoordFPfPf": ret0, "Draw__5ccAnmFv": anm_draw,
            "Init__5ccEffFP10ccEffChunki": eff_init, "Draw__5ccEffFUs": eff_draw,
            "SetActiveLayer__9WORLD_MANFi": layer,
            "__ct__17ccEnemyWeaponCtrlFiP13ccEnemyWpInfoP10ccEntryObj": weapon,
            "__ct__15ccEnemyDustCtrlFiP15ccEnemyDustInfoP10ccEntryObj": dust,
            "ccSeOn3D__FiPf": se, "effRemoveTrap__FPfii": trap, "ccStartThread__FPFPv_vii": thread,
            "__nw__FUi": new, "__dl__FPv": delete, "__dt__5ccAnmFv": reta0, "__dt__7ccClumpFv": reta0,
            "ccChangeCmndTarget__FP6ccChar": change_target,
            "main__7ccEnemyFv": main_enemy, "Breath__6ccTscbFi": breath,
            "initBreath__13ccEnemyBreathFP13ccEnemyBrInfo": init_breath,
            "GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult": ret0,
            "CloseDoor__7DUNGEONFii": ret0,
            "__nwa__FUi": new_arr, "__ct__7ccLightFsSc": reta0, "Init__11ccOmniLightFP12ccLightChunk": ret0,
            "ccSetColor__FPfUif": set_color,
            "AddGrp__10ccLightGrpFP7ccLight": add_grp, "DelGrp__10ccLightGrpFP7ccLight": del_grp,
            "effFountain__FPfPi": etc_eff(20), "effBossRoomEntrance__FPfPi": etc_eff(19),
            "effDungeonEntrance__FPfPii": etc_eff(21),
            "SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf": etc_matrix,
            "EntryFlash__8ccScFadeFiiffff": flash, "EntryFlash2__8ccScFadeFiiiffff": flash2,
            "ccEnemyEffDustRing__FPfffiii": dust_ring_at,
            "disp__13ccPrimRadiateFP10ccPrimPart": rays,
            "ccSeOn__Fi": rec_out("se2d", c_int),
            "effOpenBox__FPf": rec_out("open_box", c_vec),
            "effOpenTrapBox__FPfii": rec_out("open_trap_box", c_vec, c_int, c_int),
            "effCrushBarrel__FPfi": lambda mm, p, *_: H.outs.append(["crush", 0, vec(p)]) or 0,
            "effCrushPot__FPfi": lambda mm, p, *_: H.outs.append(["crush", 1, vec(p)]) or 0,
            "effCrushCorpse__FPfi": lambda mm, p, *_: H.outs.append(["crush", 2, vec(p)]) or 0,
            "effCrushEgg__FPfi": lambda mm, p, *_: H.outs.append(["crush", 3, vec(p)]) or 0,
            "effVirusCrystal__FPf": rec_out("virus_crystal", c_vec),
            "effStatueOfGod__FPfPi": statue,
            "ccEnemyEffDustRing__FP6ccCharPfffiii": dust_ring,
            "ccSpcMessageOpenTrapBox__Fv": lambda mm, *_: H.outs.append(["spc_msg", 0x1000F]) or 0,
            "CalcBattleDamage__6ccCharFP6ccCharifi": trap_damage,
            "ccItemSkillRequest__FP6ccCharP6ccCharii": trap_skill,
            "Draw__6ccCharFv": idol_draw,
            "ccVoicePgFood__Fi": rec_out("food_voice", c_int), "ccSeOn3DNote__FiPfc": se_note,
        }
        for n, h in hooks.items():
            m.hooks[sym(n)] = h
        # the port names a destructor by Infection's address
        for n, a in DESTS:
            m.hooks[sym(n)] = (lambda aa: lambda mm, obj, *_: (H.outs.append(
                ["destroyed", H.index_of(obj), aa]), 0)[1])(a)
        m.hooks[STUB_GIM] = main_gim
        m.hooks[STUB_NPC] = main_npc
        for f in self.gim_funcs:
            m.hooks[f] = maker(GIMMICK_TBL, VT_GIM, 3)
        for f in self.npc_funcs:
            m.hooks[f] = maker(NPC_TBL, VT_NPC, 4)
        del dest

    # memory helpers ------------------------------------------------------------------
    def put(self, va, fmt, *v):
        self.m.mem[va:va + struct.calcsize(fmt)] = struct.pack(fmt, *v)

    def get(self, va, fmt):
        return list(struct.unpack(fmt, bytes(self.m.mem[va:va + struct.calcsize(fmt)])))

    def ptr(self, i):
        return 0 if i is None or i < 0 else self.addrs[i]

    def read_ent(self, a):
        return self.get(a, "<8I") + self.get(a + 0x20, "<14i")

    def put_ent(self, a, ent):
        self.put(a, "<8I", *ent[:8])
        self.put(a + 0x20, "<14i", *ent[8:])
        self.m.mem[a + 0x58:a + 0x60] = bytes(8)

    def put_bare(self, a, row, ent, vt):
        """What a gimmick or NPC constructor stands-in leaves (the probe's
        seam makes the same): ccChar, ccEntryObj and ccGimmick as their
        constructors leave them, the entry copied, placed at its position
        and heading."""
        m = self.m
        m.store(a, 4, row)
        m.store(a + 0x28, 4, F_ONE)
        m.store(a + 0x30, 4, 0xFFFFFFFF)
        self.put(a + 0x40, "<4I", *ent[:4])
        k = self.k
        self.put(a + 0x50, "<4I", eemu.f_sub(ent[0], k[0]), eemu.f_sub(ent[1], k[1]), ent[2], ent[3])
        self.put(a + 0x60, "<4I", *ent[4:8])
        self.put(a + 0x7C, "<hh", 0, 0)
        m.store(a + 0x88, 4, F_ONE)
        m.store(a + 0x8C, 4, F_ONE)
        m.store(a + 0x90, 4, 1)
        m.store(a + 0xE0, 1, 0x10)
        m.store(a + 0xEC, 4, 128)
        self.put_ent(a + 0x100, ent)
        self.put_hit(a, [0, 0xFFFFFFFF, 0xFFFFFFFF, 0, F_ONE, 0, 0, 0, 0, F_ONE])
        m.store(a + 0x1CC, 4, vt)
        m.store(a + 0x1D0, 4, ent[9] & 0xFFFFFFFF)

    def put_hit(self, va, h):
        self.put(va + 0x160, "<IIIII", h[0], h[1], h[2], h[3], 0)
        self.put(va + 0x174, "<II", h[4], h[5])
        self.put(va + 0x180, "<4I", *h[6:10])

    def read_hit(self, va):
        return self.get(va + 0x160, "<IIII") + self.get(va + 0x174, "<II") + self.get(va + 0x180, "<4I")

    # the boxes' and idols' own members ------------------------------------------------
    def rad_of(self, va):
        vt = self.m.load(va + 0x1CC, 4)
        if vt == BOX_VT:
            return self.m.load(va + 0x1E0, 4)
        if vt == IDOL_VT:
            return self.m.load(va + 0x1E8, 4)
        if vt == ETC_VT:
            return self.m.load(va + 0x268, 4)
        return 0

    def read_rad(self, r):
        """A ccPrimRadiate as the probe's rad_vec lists it."""
        m = self.m
        v = [m.load(r, 1) & 3, s16(m.load(r + 2, 2)), s16(m.load(r + 4, 2))]
        v += self.get(r + 8, "<11I")
        user = m.load(r + 0x34, 4)
        v += [self.index_of(user) if user else -1, s32(m.load(r + 0x44, 4))]
        v += self.get(r + 0x60, "<4I") + self.get(r + 0x70, "<4I") + self.get(r + 0x80, "<4h")
        v += self.get(r + 0x88, "<4I")
        lg = m.load(r + 0x58, 4)
        v += self.get(lg + 0x40, "<16I") + [m.load(lg + 0x8D, 1), m.load(lg + 0xB0, 4), m.load(lg + 0xC0, 4)]
        v += self.get(lg + 0xC4, "<3I")
        parts = m.load(r + 0x3C, 4)
        for k in range(s16(m.load(r + 4, 2))):
            a = parts + 0xD0 * k
            v += [m.load(a + 8, 4), m.load(a + 0xC, 4)]
            for q in range(4):
                b = a + 0x10 + 0x30 * q
                v += [m.load(b + 4, 4)] + self.get(b + 0x10, "<4I")
        return v

    def put_rad(self, va, w, user, radiator=False):
        """A radiate's words (read_rad's) into memory: a box's or idol's
        (16 rays, boxRadInfo), or with radiator a ccGimRadiator's (32 rays,
        gimRadInfo, its light's vtable for the destructor)."""
        m = self.m
        at_, vt, info = (RRAD_AT, RPARTS_AT, RLIGHT_AT), GIMRAD_VT, GIM_RAD_INFO
        if not radiator:
            at_, vt, info = (RAD_AT, PARTS_AT, LIGHT_AT), BOXRAD_VT, BOX_RAD_INFO
        r, parts, lg = (va + x for x in at_)
        n = w[2]
        m.mem[r:r + 0xA0] = bytes(0xA0)
        m.mem[parts:parts + 0xD0 * n] = bytes(0xD0 * n)
        m.mem[lg:lg + 0xD0] = bytes(0xD0)
        m.store(r, 1, w[0])
        self.put(r + 2, "<hh", w[1], w[2])
        self.put(r + 8, "<11I", *w[3:14])
        m.store(r + 0x34, 4, user)
        m.store(r + 0x38, 4, info)
        m.store(r + 0x3C, 4, parts)
        m.store(r + 0x44, 4, w[15] & 0xFFFFFFFF)
        m.store(r + 0x58, 4, lg)
        self.put(r + 0x60, "<4I", *w[16:20])
        self.put(r + 0x70, "<4I", *w[20:24])
        self.put(r + 0x80, "<4h", *w[24:28])
        self.put(r + 0x88, "<4I", *w[28:32])
        m.store(r + 0x98, 4, vt)
        if radiator:
            m.store(lg + 0xA4, 4, OMNI_VT)
        self.put(lg + 0x40, "<16I", *w[32:48])
        m.store(lg + 0x8D, 1, w[48])
        m.store(lg + 0xB0, 4, w[49])
        m.store(lg + 0xC0, 4, w[50])
        self.put(lg + 0xC4, "<3I", *w[51:54])
        at = 54
        for k in range(n):
            a = parts + 0xD0 * k
            m.store(a + 8, 4, w[at])
            m.store(a + 0xC, 4, w[at + 1])
            at += 2
            for q in range(4):
                b = a + 0x10 + 0x30 * q
                m.store(b + 4, 4, w[at])
                self.put(b + 0x10, "<4I", *w[at + 1:at + 5])
                at += 5
        return r

    def read_class(self, va):
        vt = self.m.load(va + 0x1CC, 4)
        if vt == BOX_VT:
            return [1] + self.read_rad(self.m.load(va + 0x1E0, 4))
        if vt == IDOL_VT:
            return [2, s32(self.m.load(va + 0x1E0, 4)), s32(self.m.load(va + 0x1E4, 4))] + self.read_rad(
                self.m.load(va + 0x1E8, 4))
        if vt == ETC_VT:
            return self.read_etc(va)
        if vt == SYMBOL_VT:
            # act and count are ccGimmick's +0x1d4 and +0x1d8.
            m = self.m
            return [3, s32(m.load(va + 0x1D4, 4)), s32(m.load(va + 0x1D8, 4)), m.load(va + 0x700, 1) & 1]
        if vt == FOOD_VT:
            m = self.m
            # actNum, actCnt, anmFlag (+0x1d4 ..) are ccGimmick's: the obj's.
            v = [4, m.load(va + 0x1E0, 1) & 1, s32(m.load(va + 0x1E4, 4))]
            v += self.get(va + 0x1F0, "<4I") + self.get(va + 0x200, "<2I") + self.get(va + 0x210, "<3I")
            v += [s32(x) for x in self.get(va + 0x21C, "<3I")]
            v += [s16(m.load(va + 0x228, 2)), s16(m.load(va + 0x22C, 2)), m.load(va + 0x230, 4)]
            return v
        return [0]

    def read_etc(self, va):
        """A ccGimEtc's words as the probe's etc_vec lists them."""
        m = self.m
        v = [5, s32(m.load(va + 0x1E0, 4)), m.load(va + 0x1E4, 1) & 3]
        v += self.get(va + 0x1F0, "<4I") + self.get(va + 0x200, "<4I") + self.get(va + 0x210, "<2I")
        v += self.get(va + 0x220, "<4I") + [s32(m.load(va + 0x230, 4))] + self.get(va + 0x234, "<4h")
        v += self.get(va + 0x23C, "<I") + self.get(va + 0x240, "<4I") + self.get(va + 0x250, "<3I")
        v.append(m.load(va + 0x25C, 1) & 1)
        for k in range(2):
            lg = m.load(va + 0x260 + 4 * k, 4)
            if lg:
                v += [m.load(lg + 0xB0, 4)] + self.get(lg + 0xC0, "<4I") + self.get(lg + 0x70, "<3I")
            else:
                v += [0] * 8
        rad = m.load(va + 0x268, 4)
        return v + ([1] + self.read_rad(rad) if rad else [0])

    def put_etc(self, va, w, gid):
        m = self.m
        m.store(va + 0x1CC, 4, ETC_VT)
        m.store(va + 0x1E0, 4, w[1] & 0xFFFFFFFF)
        m.store(va + 0x1E4, 1, w[2])
        self.put(va + 0x1F0, "<4I", *w[3:7])
        self.put(va + 0x200, "<4I", *w[7:11])
        self.put(va + 0x210, "<2I", *w[11:13])
        self.put(va + 0x220, "<4I", *w[13:17])
        m.store(va + 0x230, 4, w[17] & 0xFFFFFFFF)
        self.put(va + 0x234, "<4h", *w[18:22])
        self.put(va + 0x23C, "<I", w[22])
        self.put(va + 0x240, "<4I", *w[23:27])
        self.put(va + 0x250, "<3I", *w[27:30])
        m.store(va + 0x25C, 1, w[30])
        at = 31
        for k in range(2):
            lg = va + ETC_LIGHTS + 0xD0 * k
            m.mem[lg:lg + 0xD0] = bytes(0xD0)
            if gid == 20:
                m.store(lg + 0xB0, 4, w[at])
                self.put(lg + 0xC0, "<4I", *w[at + 1:at + 5])
                self.put(lg + 0x40, "<16I", F_ONE, 0, 0, 0, 0, F_ONE, 0, 0, 0, 0, F_ONE, 0, *w[at + 5:at + 8], F_ONE)
                m.store(lg + 0xA4, 4, inf_va(0x00375A50))
            m.store(va + 0x260 + 4 * k, 4, lg if gid == 20 else 0)
            at += 8
        rad = w[at:]
        user = self.ptr(rad[15]) if len(rad) > 15 else 0
        m.store(va + 0x268, 4, self.put_rad(va, rad[1:], user, radiator=True) if rad and rad[0] else 0)

    def put_class(self, va, cls, chunk_name):
        """A box's or idol's class words into memory (and its vtable and
        animation player); a bare gimmick keeps the stub's vtable."""
        m = self.m
        if cls[0] == 0:
            m.store(va + 0x1CC, 4, VT_GIM)
            return
        if cls[0] == 5:
            self.put_etc(va, cls, s32(m.load(va + 0x1D0, 4)))
        elif cls[0] == 1:
            m.store(va + 0x1CC, 4, BOX_VT)
            m.store(va + 0x1E0, 4, self.put_rad(va, cls[1:], va))
        elif cls[0] == 4:
            m.store(va + 0x1CC, 4, FOOD_VT)
            m.store(va + 0x1E0, 1, cls[1] & 1)
            self.put(va + 0x1E4, "<i", cls[2])
            self.put(va + 0x1F0, "<4I", *cls[3:7])
            self.put(va + 0x200, "<2I", *cls[7:9])
            self.put(va + 0x210, "<3I", *cls[9:12])
            self.put(va + 0x21C, "<3i", *cls[12:15])
            self.put(va + 0x228, "<h", cls[15])
            self.put(va + 0x22C, "<h", cls[16])
            m.store(va + 0x230, 4, cls[17])
        else:
            m.store(va + 0x1CC, 4, IDOL_VT)
            m.store(va + 0x1E0, 4, cls[1] & 0xFFFFFFFF)
            m.store(va + 0x1E4, 4, cls[2] & 0xFFFFFFFF)
            m.store(va + 0x1E8, 4, self.put_rad(va, cls[3:], va))
        anm = va + GIM_ANM
        m.mem[anm:anm + 0x110] = bytes(0x110)
        m.store(va + 0xD4, 4, anm)
        m.store(anm + 0x9C, 2, 256)
        m.store(anm + 0xAC, 4, self.chunk(chunk_name))

    # objects -------------------------------------------------------------------------
    def put_char(self, va, kind, ch):
        """ch: [base, pos4, posP4, HP, SP, maxHP, maxSP, cond16, speed,
        condNum, skillID, skillStatus, entRoot]."""
        m = self.m
        b = ch[0]
        if kind == 0:
            m.store(KITE_BASE + 8, 4, b & 0xFFFFFFFF)
            base = KITE_BASE
        elif kind == 1:
            base = ENEMY_TBL + ROW * b
        elif kind in (2, 3):
            base = GIMMICK_TBL + 0x70 * b
        else:
            base = NPC_TBL + 0x70 * b
        m.store(va, 4, base)
        m.store(va + 4, 4, va + 0x1D0 if kind == 1 else 0)
        self.put(va + 8, "<16h", *ch[13:29])
        m.store(va + 0x28, 4, ch[29])
        m.store(va + 0x30, 4, ch[30] & 0xFFFFFFFF)
        self.put(va + 0x40, "<4I", *ch[1:5])
        self.put(va + 0x50, "<4I", *ch[5:9])
        self.put(va + 0x70, "<4h", *ch[9:13])
        self.put(va + 0x7C, "<hh", ch[31], ch[32])
        m.store(va + 0x94, 4, ENEMY_INFLUENCE if kind == 1 else 0)
        m.store(va + 0x140, 4, ch[33] & 0xFFFFFFFF)

    def read_char(self, va):
        m = self.m
        base = m.load(va, 4)
        v = [s32(m.load(base + 8, 4)), s16(m.load(base + 0xC, 2))]
        v += self.get(va + 0x40, "<4I") + self.get(va + 0x50, "<4I") + self.get(va + 0x70, "<4h")
        v += self.get(va + 8, "<16h") + [m.load(va + 0x28, 4), s32(m.load(va + 0x30, 4))]
        v += self.get(va + 0x7C, "<hh") + [m.load(va + 0x140, 4)]
        v.append(1 if m.load(va + 0x94, 4) == ENEMY_INFLUENCE else 0)
        return v

    def put_common(self, va, o):
        """o: the probe's 52 words (see spawn.rs obj_vec)."""
        m = self.m
        self.put(va + 0x60, "<4I", *o[0:4])
        m.store(va + 0x88, 4, o[4])
        m.store(va + 0x8C, 4, o[5])
        m.store(va + 0xE0, 1, o[6])
        m.store(va + 0xE4, 4, o[7])
        m.store(va + 0xE8, 4, o[8])
        m.store(va + 0xEC, 4, o[9] & 0xFFFFFFFF)
        self.put(va + 0xF0, "<4h", *o[10:14])
        self.put_ent(va + 0x100, o[14:36])
        self.put_hit(va, o[36:46])
        m.store(va + 0x1B0, 4, o[46])
        m.store(va + 0x1C8, 4, inf_va(o[47]) if o[47] in DEST_ADDRS else o[47])
        self.put(va + 0x1D0, "<4i", *o[48:52])

    def read_common(self, va):
        m = self.m
        v = self.get(va + 0x60, "<4I") + [m.load(va + 0x88, 4), m.load(va + 0x8C, 4), m.load(va + 0xE0, 1)]
        v += [m.load(va + 0xE4, 4), m.load(va + 0xE8, 4), s32(m.load(va + 0xEC, 4))]
        v += self.get(va + 0xF0, "<4h") + self.read_ent(va + 0x100) + self.read_hit(va)
        dest = m.load(va + 0x1C8, 4)
        v += [m.load(va + 0x1B0, 4), DEST_BACK().get(dest, dest)] + self.get(va + 0x1D0, "<4i")
        if m.load(va + 0x1CC, 4) == ETC_VT and radiator_words(v):
            v[35] = self.index_of(v[35] & 0xFFFFFFFF) + 1 if v[35] else 0
        return v

    def put_parts(self, va, parts, eff_base):
        m = self.m
        for k, p in enumerate(parts):
            a = va + 0x1F0 + 0x70 * k
            m.store(a, 1, p[1] & 1)
            m.store(a + 4, 4, p[0] & 0xFFFFFFFF)
            self.put(a + 0x10, "<16I", *p[2:18])
            self.put(a + 0x50, "<II", p[18], p[19])
            self.put(a + 0x58, "<hHhh", p[20], p[21], p[22], p[23])
            self.put(a + 0x60, "<II", p[24], p[25])
            e = eff_base + 0x70 * k
            m.store(a + 0x68, 4, e)
            self.put(e + 0x10, "<4I", *p[26:30])
            self.put(e + 0x20, "<IIII", *p[30:34])
            self.put(e + 0x30, "<H", p[34])
            m.store(e + 0x34, 4, p[35])

    def read_parts(self, va):
        mem = self.m.mem
        out = []
        n = len(mem)
        for k in range(PARTS):
            a = va + 0x1F0 + 0x70 * k
            b = bytes(mem[a:a + 0x70])
            flag = b[0] & 1
            status, = struct.unpack_from("<i", b, 4)
            p = [status, flag] + list(struct.unpack_from("<16I2IhHhh2I", b, 0x10))
            e, = struct.unpack_from("<I", b, 0x68)
            if 0 < e < n - 0x70:
                eb = bytes(mem[e + 0x10:e + 0x38])
                p += list(struct.unpack_from("<4I4IH", eb, 0)) + list(struct.unpack_from("<I", eb, 0x24))
            else:
                p += [0] * 10
            out.append(p)
        return out

    def put_enemy(self, va, e, ex):
        """e: the enemy AI harness's dict; ex: plDist plDirc alpha grotDeg
        grotSpd transparency setTransparency destFunc hit[10] anmTbl."""
        m = self.m
        self.ea.put_enemy(va, e, m.load(va, 4))
        m.store(va + 0x1CC, 4, CCENEMY_VT)
        m.store(va + 0xE4, 4, ex[0])
        m.store(va + 0xE8, 4, ex[1])
        m.store(va + 0xEC, 4, ex[2] & 0xFFFFFFFF)
        self.put(va + 0xF4, "<hh", ex[3], ex[4])
        m.store(va + 0x88, 4, ex[5])
        m.store(va + 0x8C, 4, ex[6])
        m.store(va + 0x1C8, 4, inf_va(ex[7]) if ex[7] in DEST_ADDRS else ex[7])
        self.put_hit(va, ex[8:18])
        m.store(va + 0x1B0, 4, ex[18])

    def anm_row(self, va):
        """The enemy's anmTbl as the port keeps it: the enemyTbl row whose
        entry.anm it is (the character's own base row) plus one, 0 for none."""
        m = self.m
        anm, row = m.load(va + 0x1B0, 4), (m.load(va, 4) - ENEMY_TBL) // ROW
        if anm and 0 <= row < 303 and m.load(ENEMY_TBL + ROW * row + 0x80, 4) == anm:
            return row + 1
        return anm

    def read_enemy_extra(self, va):
        m = self.m
        v = [m.load(va + 0xE4, 4), m.load(va + 0xE8, 4), s32(m.load(va + 0xEC, 4))]
        dest = m.load(va + 0x1C8, 4)
        v += self.get(va + 0xF4, "<hh") + [m.load(va + 0x88, 4), m.load(va + 0x8C, 4), DEST_BACK().get(dest, dest)]
        v += self.read_hit(va) + [self.anm_row(va)]
        race = RACE_VTS.get(m.load(va + 0x1CC, 4))
        wc_at, dc_at = CTRL_AT.get(race, (0x340, 0x344))
        for at in (wc_at, dc_at):
            p = m.load(va + at, 4) if race else 0
            v += [m.load(p + 4, 4), s32(m.load(p, 4))] if p else [0, 0]
        if race == "G":
            fl = m.load(va + 0x340, 2)
            v += [fl & 0xF] + self.get(va + 0x342, "<hhh") + self.get(va + 0x348, "<4h")
            v += self.get(va + 0x350, "<4I") + self.get(va + 0x360, "<4I") + [s32(m.load(va + 0x370, 4))]
        else:
            v += [0] * 17
        # ccEnemyC's spin (+0x340), ccEnemyH's and L's breaths made (+0x348),
        # ccEnemyL's four ccRandS (+0x390)
        v += [m.load(va + 0x340, 4) if race == "C" else 0, int(race in ("H", "L") and m.load(va + 0x348, 4) != 0)]
        v += list(self.get(va + 0x390, "<4h")) if race == "L" else [0] * 4
        return v

    # the scene -----------------------------------------------------------------------
    def rnd_scene(self, rnd, ne=None, nc=None, ng=None, nn=None, area=None):
        """A random scene: returns a dict the game and the probe are set up
        from."""
        sc = {}
        area = area if area is not None else rnd.choice((1, 1, 2, 2, 0))
        field = rnd.choice((14, 14, 0, rnd.randrange(1, 60)))
        game = [area, rnd.randrange(0, 5), field, rnd.randrange(0, 3), rnd.randrange(-1, 4), rnd.randrange(-1, 6),
                rnd.randrange(0, 5), rnd.choice((4, rnd.randrange(0, 11))), rnd.randrange(0, 1 << 24), 0,
                rnd.choice((1, 2, 3, 4, 5, 7))]
        sc["game"] = game
        kx, ky = (rf(rnd, -20000, 20000), rf(rnd, -20000, 20000)) if rnd.random() < 0.8 else (0, 0)
        sc["k"] = [kx, ky]
        an = {0: game[1], 1: game[2], 2: game[3]}.get(area, 0)
        ctrl = [area, an, game[4], game[5]]
        if rnd.random() < 0.15:
            ctrl = [rnd.randrange(0, 3), rnd.randrange(0, 3), rnd.randrange(-1, 3), rnd.randrange(-1, 3)]
        sc["ctrl"] = ctrl
        rows = self.ported_rows(rnd)
        num = rnd.randrange(1, 9)
        cells = [-1] * 24
        for k in range(num):
            cells[k] = rnd.choice(rows)
        sc["reg"] = {"cells": cells, "num": num, "dnum": rnd.randrange(0, 5), "range": 3,
                     "exist": sorted(set(rnd.choice(rows) for _ in range(rnd.randrange(0, 6))))}
        sc["rng"] = [rnd.getrandbits(32), rnd.choice((rnd.randrange(0, 624), 623, 624, 625)), rnd.getrandbits(16)]
        ev = [rnd.getrandbits(32) if rnd.random() < 0.5 else 0 for _ in range(6)]
        code = game[8] | (game[6] << 29)
        fount = [rnd.choice((0, rnd.getrandbits(31), code if rnd.random() < 0.3 else 0)) for _ in range(100)]
        sc["save"] = ev + [s32(x) for x in fount] + [rnd.choice((0, 5, 9999, 10000, rnd.randrange(-100, 12000)))
                                                    for _ in range(3)]
        sc["land"] = [rf(rnd, -200, 800) if rnd.random() < 0.9 else fb(rnd.choice((0.0, -1.0)))
                      for _ in range(rnd.randrange(0, 40))]
        dusts = (0x00B0C000, inf_va(0x00304050), inf_va(0x00606060), 0x00E0E0E0, 0x008080F0, inf_va(0x0070C0E0))
        sc["attr"] = [rnd.choice(dusts) | rnd.getrandbits(32) & 0xFF0F0F0F if rnd.random() < 0.5
                      else rnd.getrandbits(32) for _ in range(rnd.randrange(0, 12))]
        sc["camdeg"] = [int(rnd.random() < 0.7) for _ in range(rnd.randrange(0, 12))]
        sc["camtr"] = [rnd.choice((F_ONE, rf(rnd, -0.2, 1.3), 0)) for _ in range(rnd.randrange(0, 12))]
        sc["fwd"] = [int(rnd.random() < 0.3) for _ in range(rnd.randrange(0, 12))]
        sc["emain"] = [int(rnd.random() < 0.15) for _ in range(rnd.randrange(0, 12))]
        sc["gmain"] = [int(rnd.random() < 0.2) for _ in range(rnd.randrange(0, 12))]
        sc["nmain"] = [int(rnd.random() < 0.2) for _ in range(rnd.randrange(0, 12))]
        objs = []
        kite_type = rnd.choice((1, 1, 7))
        kz = rf(rnd, -100, 400)
        objs.append({"kind": 0, "char": self.rnd_char(rnd, 0, kite_type, [kx, ky, kz, F_ONE], sc), "link": [-1, -1]})
        ne = rnd.randrange(0, 4) if ne is None else ne
        nc = rnd.randrange(0, 3) if nc is None else nc
        ng = rnd.randrange(0, 3) if ng is None else ng
        nn = rnd.randrange(0, 2) if nn is None else nn
        for kind, n in ((1, ne), (2, nc), (3, ng), (4, nn)):
            for _ in range(n):
                objs.append(self.rnd_obj(rnd, kind, sc))
        sc["objs"] = objs
        lists = []
        for kind in (1, 2, 3, 4):
            members = [i for i, o in enumerate(objs) if o["kind"] == kind and rnd.random() < 0.92]
            rnd.shuffle(members)
            for a, b in zip([None] + members, members + [None]):
                if a is not None:
                    objs[a]["link"][1] = b if b is not None else -1
                if b is not None:
                    objs[b]["link"][0] = a if a is not None else -1
            lists.append([len(members), members[0] if members else -1, members[-1] if members else -1])
        sc["lists"] = lists
        pcs = [0] if rnd.random() < 0.75 else []
        enes = [i for i, o in enumerate(objs) if o["kind"] == 1 and rnd.random() < 0.6]
        obs = [i for i, o in enumerate(objs) if o["kind"] in (2, 3, 4) and rnd.random() < 0.4]
        rnd.shuffle(enes)
        rnd.shuffle(obs)
        sc["cmnd"] = [pcs, enes, obs]
        game[9] = 0 if rnd.random() < 0.95 else -1
        return sc

    def ported_rows(self, rnd=None):
        rows = []
        for r in BUILT:
            rows += [i for i in range(self.race_first[r], self.race_first[r] + self.race_num[r]) if i not in ELG]
        # The middle bosses among them use their base forms' animation
        # tables, copied in as the area loads (put_scene; the port's
        # tables.rs).
        return rows

    def rnd_pos(self, rnd, sc, span=6000.0):
        k = sc["k"]
        return [eemu.f_add(k[0], rf(rnd, -span, span)), eemu.f_add(k[1], rf(rnd, -span, span)),
                rf(rnd, -300, 900), F_ONE]

    def rnd_char(self, rnd, kind, base, pos, sc):
        k = sc["k"]
        posp = [eemu.f_sub(pos[0], k[0]), eemu.f_sub(pos[1], k[1]), pos[2], pos[3]]
        if rnd.random() < 0.1:
            posp = [rf(rnd, -3000, 3000), rf(rnd, -3000, 3000), rf(rnd, -100, 100), F_ONE]
        cond = [rnd.choice((0, 0, 0, 1, 30)) for _ in range(16)]
        hp = [rnd.randrange(0, 300), rnd.randrange(0, 100), rnd.randrange(1, 300), rnd.randrange(0, 100)]
        return ([base] + pos + posp + hp + cond + [rnd.choice((F_ONE, fb(0.5))), rnd.choice((-1, 0, 3)),
                                                   rnd.randrange(-1, 300), rnd.randrange(-1, 10),
                                                   rnd.choice((0, 1, 2, 0xFFFFFFFF))])

    def rnd_ent(self, rnd, sc, ty=None, id_=None, pos=None):
        g = sc["game"]
        pos = pos or self.rnd_pos(rnd, sc)
        dirc = [0, 0, rf(rnd, -math.pi, math.pi), F_ONE]
        if ty is None:
            ty = rnd.choice((0, 1, 1, 2))
        if id_ is None:
            if ty == 0:
                id_ = rnd.choice(self.ported_rows() + [-1])
            elif ty == 1:
                id_ = rnd.choice((0, 1, 2, 3, 4, 5, 6, 15, 15, 20, 38, 40, 44, rnd.randrange(0, 45)))
            else:
                id_ = rnd.randrange(0, 175)
        c = sc["ctrl"]
        here = rnd.random() < 0.8
        area = [c[0], c[1], c[2], c[3]] if here else [rnd.randrange(0, 3), rnd.randrange(0, 60),
                                                      rnd.randrange(-1, 3), rnd.randrange(-1, 3)]
        ent = (pos + dirc + [ty, id_] + area + [rnd.randrange(-1, 40), rnd.randrange(-1, 40)]
               + [rnd.choice((-1, 0, 1, 2, 3)), rnd.choice((-1, -1, 0))]
               + [rnd.choice((-1, -1, rnd.randrange(0, 6 * 32), -40)), rnd.choice((-1, -1, 0, 1)),
                  rnd.choice((-1, -1, 0, rnd.choice(self.ported_rows()), 1)), rnd.choice((-1, -1, 1, 2, 3))])
        # The warning (row 19) with param[2] 0 takes param[3] for a
        # character's address (its rays' user); nothing places one so.
        if ty == 1 and id_ == 19 and ent[20] == 0:
            ent[20] = -1
        return ent

    def rnd_gift(self, rnd):
        """A circle's param[1] (what it gives) and param[2] (which)."""
        kind = rnd.choice((-1, -1, 0, 0, 1, 2))
        if kind == -1:
            return -1, -1
        if kind == 0:
            return 0, rnd.choice((-1, rnd.choice(self.ported_rows())))
        if kind == 1:
            return 1, rnd.choice((-1, 0, 1, 2, 5, 6, 15, 20, 38, 44, rnd.randrange(0, 45)))
        return 2, rnd.randrange(0, 175)

    def rnd_hit(self, rnd, pos):
        return [rnd.choice((0, 1)), rnd.choice((0xFFFFFFFF, 0x40000002)), rnd.choice((0xFFFFFFFF, 0)),
                rnd.choice((0, 0x20)), rf(rnd, 0, 80), rf(rnd, 0, 100)] + list(pos)

    def rnd_common(self, rnd, sc, kind, ent, gim_id):
        pos = ent[:4]
        return ([0, 0, rf(rnd, -math.pi, math.pi), F_ONE]
                + [rnd.choice((F_ONE, 0, rf(rnd, 0, 1))), rnd.choice((F_ONE, 0, rf(rnd, 0, 1)))]
                + [rnd.getrandbits(8) & ~0x80 | (0x80 if rnd.random() < 0.05 else 0)]
                + [rnd.choice((0, rf(rnd, 0, 12000), fb(3000.0), fb(7000.0), fb(10000.0))), rf(rnd, -3.2, 3.2),
                   rnd.choice((128, 0, 255))]
                + [rnd.choice((0, 0, 1, 2)), rnd.choice((0, 1, 30, 7))]
                + [rnd.randrange(-32768, 32768), rnd.choice((0, 0, 0, 64, 1, -1, 512))]
                + ent + self.rnd_hit(rnd, pos)
                + [0, rnd.choice((0, 0x00455870))]
                + [gim_id, rnd.choice((0, 1, 2, 3, 4)) if kind == 2 else rnd.randrange(0, 5),
                   rnd.choice((0, 1, 15, 16, 20, 21, 22, 30, 31, 32, 64, 65, 66)), rnd.choice((0, 0, 1))])

    def rnd_part(self, rnd, sc):
        st = rnd.choice((0, 0, 1, 2, 3, 4))
        cnt = rnd.choice((0, 0, 1, 5, 9, 10, 11, 29, 30, 31, 119, 120, 121))
        life = {1: 120, 2: 10, 3: 30, 4: 120}.get(st, 0)
        a, b, c = (rnd.uniform(-math.pi, math.pi) for _ in range(3))
        ca, sa, cb, sb = math.cos(a), math.sin(a), math.cos(b), math.sin(b)
        mat = [ca * cb, sa * cb, -sb, 0, -sa, ca, 0, 0, ca * sb, sa * sb, cb, 0]
        mat = [fb(x) for x in mat] + self.rnd_pos(rnd, sc, 3000)
        eff = self.rnd_pos(rnd, sc, 3000) + [rf(rnd, 0.5, 5), rf(rnd, 0.5, 5), 0,
                                             rnd.getrandbits(32), PATNUM, rf(rnd, 0, 1)]
        return ([st, int(rnd.random() < 0.05)] + mat + [rf(rnd, -40, 40), rf(rnd, -3.1, 3.1)]
                + [rnd.randrange(0, 70), rnd.randrange(0, PATNUM), cnt if st else 0, life]
                + [F_ONE, rf(rnd, 0, 0.2)] + eff)

    def rnd_obj(self, rnd, kind, sc):
        if kind == 1:
            row = rnd.choice(self.ported_rows())
            ent = self.rnd_ent(rnd, sc, 0, row)
            pos = ent[:4]
            e = EA.default_enemy()
            e["dirc"] = [0, 0, rf(rnd, -math.pi, math.pi), F_ONE]
            ef = rnd.getrandbits(8) & 0x7F
            e["eflags"] = ef
            e["fade"] = [rnd.choice((0, 1, 2)), rnd.choice((1, 30, 7))]
            e["ent"] = ent
            e["ids"] = [row, 0, 0]
            ex = ([rnd.choice((0, rf(rnd, 0, 12000))), rf(rnd, -3.2, 3.2), 128, rnd.randrange(-32768, 32768),
                   rnd.choice((0, 0, 64))] + [rnd.choice((0, F_ONE, rf(rnd, 0, 1))) for _ in range(2)]
                  + [rnd.choice((0, 0x00446010, 0x0044A780))] + self.rnd_hit(rnd, pos) + [0])
            return {"kind": 1, "char": self.rnd_char(rnd, 1, row, pos, sc), "link": [-1, -1], "enemy": e,
                    "extra": ex}
        gid = {2: 15, 3: rnd.choice([i for i in range(45) if i != 15]), 4: rnd.randrange(0, 175)}[kind]
        ent = self.rnd_ent(rnd, sc, 1 if kind != 4 else 2, gid)
        pos = ent[:4]
        o = {"kind": kind, "char": self.rnd_char(rnd, kind, gid, pos, sc), "link": [-1, -1],
             "obj": self.rnd_common(rnd, sc, kind, ent, gid)}
        if kind == 3:
            o["cls"], o["aff"] = [0], [0, -1]
        if kind == 2:
            o["circle"] = [int(rnd.random() < 0.8), rnd.randrange(0, PARTS)]
            o["parts"] = [self.rnd_part(rnd, sc) for _ in range(PARTS)]
        return o

    # the scene in memory ---------------------------------------------------------------
    @staticmethod
    def normalize(sc):
        """Every entry object's +0x140 is its entry's entRoot."""
        for o in sc["objs"]:
            if o["kind"] == 1:
                o["char"][33] = o["enemy"]["ent"][16] & 0xFFFFFFFF
            elif o["kind"] in (2, 3, 4):
                o["char"][33] = o["obj"][30] & 0xFFFFFFFF

    def put_scene(self, sc):
        m, g, a = self.m, self.g, self.addr
        self.normalize(sc)
        self.reset_state()
        game = sc["game"]
        m.mem[CTRL:CTRL + 0x40] = bytes(0x40)
        m.store(a["g_entCtrl"], 4, CTRL)
        m.store(TB.GAME + 0x60, 4, IBD_UNSET)
        self.put(TB.GAME + 0x14, "<i", game[0])
        self.put(TB.GAME + 0x1C, "<iiiiii", game[6], game[1], game[2], game[3], game[4], game[5])
        m.store(a["worldman"], 4, WMAN)
        m.mem[WMAN:WMAN + 0x4E0] = bytes(0x4E0)
        m.store(WMAN + 0x10, 4, game[7] & 0xFFFFFFFF)
        m.store(WMAN + 0x158, 4, WP)
        m.mem[WP:WP + 0x30] = bytes(0x30)
        m.store(WP + 4, 4, game[8] & 0xFFFFFFFF)
        m.store(WP + 0x20, 4, game[10] if len(game) > 10 else 0)
        self.k = list(sc["k"])
        c = sc["ctrl"]
        self.put(CTRL, "<4i", *c)
        # registered rows
        reg = sc["reg"]
        self.put(a["ccRegisterEnemyTbl"], "<24i", *reg["cells"])
        self.put(a["ccRegisterEnemyNum"], "<i", reg["num"])
        self.put(a["ccRegisterDrainNum"], "<i", reg["dnum"])
        self.put(a["ccRegisterEnemyRange"], "<i", reg["range"])
        for i in range(303):
            m.store(ENEMY_TBL + ROW * i + 0x68, 4, 1 if i in reg["exist"] else 0)
            # ccInitRegisterEnemy's entry.func: the row's race constructor
            m.store(ENEMY_TBL + ROW * i + 0x6C, 4, self.race_funcs[i])
        # ccAddRequestFileListEntry (0x0042f5a0) as the area loads: a middle
        # boss's row (type 0x40) takes its base form's (gold) entry.anm,
        # clut and fileList, as the port's tables hold them.
        for i in range(303):
            row = ENEMY_TBL + ROW * i
            if m.load(row + 0x8, 4) == 0x40:
                base = ENEMY_TBL + ROW * m.load(row + 0x14, 4)
                m.mem[row + 0x80:row + 0x84] = m.mem[base + 0x80:base + 0x84]
                m.mem[row + 0x88:row + 0xA6] = m.mem[base + 0x88:base + 0xA6]
                m.mem[row + 0xA8:row + 0xB0] = m.mem[base + 0xA8:base + 0xB0]
        seed, mti, rnds = sc["rng"]
        self.ea.put_cc(seed, mti)
        m.store(a["lastRnd"], 2, rnds)
        # the save
        sv = sc["save"]
        field = min(max(game[2], 0), 160)
        m.mem[SAVE + EV_ENTRY:SAVE + 0x6870] = bytes(0x6870 - EV_ENTRY)
        self.put(SAVE + EV_ENTRY + 24 * field, "<6I", *[x & 0xFFFFFFFF for x in sv[:6]])
        self.put(SAVE + FOUNTAIN, "<100i", *sv[6:106])
        for at, v in zip(COUNTERS, sv[106:109]):
            self.put(SAVE + at, "<h", s16(v))
        # scripts
        self.land, self.attr = list(sc["land"]), list(sc["attr"])
        self.camdeg, self.camtr, self.fwd = list(sc["camdeg"]), list(sc["camtr"]), list(sc["fwd"])
        self.emain, self.gmain, self.nmain = list(sc["emain"]), list(sc["gmain"]), list(sc["nmain"])
        # objects
        objs = sc["objs"]
        for i, o in enumerate(objs):
            va = OBJS + STRIDE * i
            m.mem[va:va + STRIDE] = bytes(STRIDE)
            self.add_obj(va, o["kind"])
        for i, o in enumerate(objs):
            va = self.addrs[i]
            if o["kind"] == 5:
                self.put_pc(va, o["pc"])
                continue
            self.put_char(va, o["kind"], o["char"])
            if o["kind"] == 1:
                self.put_enemy(va, o["enemy"], o["extra"])
            elif o["kind"] in (2, 3, 4):
                w = list(o["obj"])
                # a radiator's param[3]: its user (index + 1) as the pointer
                if o["kind"] == 3 and o.get("cls", [0])[0] == 5 and radiator_words(w):
                    w[35] = self.ptr(w[35] - 1) if w[35] > 0 else 0
                self.put_common(va, w)
                m.store(va + 0x1CC, 4, {2: inf_va(0x003763F0), 3: VT_GIM, 4: VT_NPC}[o["kind"]])
                if o["kind"] == 3:
                    gid = o["obj"][48]
                    self.put_class(va, o.get("cls", [0]), gim_clip(gid, o["obj"][49]))
                    aff = o.get("aff", [0, -1])
                    self.put(va + 0x9C, "<h", aff[0])
                    m.store(va + 0x98, 4, 0)
                if o["kind"] == 2:
                    m.store(va + 0x1E0, 1, o["circle"][0])
                    m.store(va + 0x1E4, 4, o["circle"][1])
                    self.put_parts(va, o["parts"], va + 0x3A00)
                    anm = va + 0x7400
                    m.store(va + 0xD4, 4, anm)
                    m.store(anm + 0x9C, 2, 256)
                    m.store(anm + 0xAC, 4, self.chunk("ANM_xmagcir1"))
            m.store(va + 0x1C0, 4, self.ptr(o["link"][0]))
            m.store(va + 0x1C4, 4, self.ptr(o["link"][1]))
            if o["kind"] == 3:
                m.store(va + 0x98, 4, self.ptr(o.get("aff", [0, -1])[1]))
        for k, l in enumerate(sc["lists"]):
            self.put(CTRL + 0x10 + 12 * k, "<iII", l[0], self.ptr(l[1]), self.ptr(l[2]))
        for root, lst in zip(self.gb_cmnd, sc["cmnd"]):
            m.store(root, 4, self.ptr(lst[0]) if lst else 0)
            m.store(root + 4, 4, self.ptr(lst[-1]) if lst else 0)
            for x, y in zip(lst, lst[1:] + [None]):
                m.store(self.addrs[x] + 0xBC, 4, self.ptr(y) if y is not None else 0)
        m.store(a["plw"] + 0x20, 4, self.ptr(0 if game[9] == 0 else -1))
        self.put_hand(sc.get("hand"))
        m.store(DRAWENV_ACTIVE, 4, DRAWENV)
        m.mem[DRAWENV:DRAWENV + 0x100] = bytes(0x100)
        self.g.m.store(self.g.sym("cmndTarget"), 4, 0)

    def put_hand(self, hand):
        """Kite's right hand and AI as a radiator reads them (ccSpcChar
        +0x130 objHandR, its lwMatrix; +0x128 ai, byte 0 bit 0 manualSW):
        hand is [manual] + the matrix's 16 words, or None for none."""
        m, k = self.m, self.addrs[0]
        if not hand:
            m.store(k + 0x128, 4, 0)
            m.store(k + 0x130, 4, 0)
            return
        m.mem[HAND:HAND + 0xB0] = bytes(0xB0)
        self.put(HAND, "<16I", *hand[1:17])
        m.store(KITE_AI, 4, hand[0] & 1)
        m.store(k + 0x128, 4, KITE_AI)
        m.store(k + 0x130, 4, HAND)

    def ser_scene(self, sc):
        v = []
        game = sc["game"]
        v += game[:9] + [0 if game[9] == 0 else -1] + [game[10] if len(game) > 10 else 0]
        v += sc["k"]
        v += sc["ctrl"]
        for l in sc["lists"]:
            v += l
        reg = sc["reg"]
        v += reg["cells"] + [reg["num"], reg["dnum"], reg["range"], len(reg["exist"])] + reg["exist"]
        v += sc["rng"]
        v += sc["save"]
        for key in ("land", "attr", "camdeg", "camtr", "fwd", "emain", "gmain", "nmain"):
            v += [len(sc[key])] + sc[key]
        v.append(len(sc["objs"]))
        parts = [" ".join(str(x) for x in v)]
        for o in sc["objs"]:
            if o["kind"] == 5:
                parts.append("5 -1 -1 " + rs.ser_char(o["pc"], self.c.b))
                continue
            w = [o["kind"]] + o["link"] + o["char"][:34]
            s = " ".join(str(x) for x in w)
            if o["kind"] == 1:
                s += " " + EA.ser_enemy(o["enemy"]) + " " + " ".join(str(x) for x in o["extra"])
            elif o["kind"] in (2, 3, 4):
                s += " " + " ".join(str(x) for x in o["obj"])
                if o["kind"] == 3:
                    s += " " + " ".join(str(x) for x in o.get("cls", [0]) + o.get("aff", [0, -1]))
                if o["kind"] == 2:
                    s += " " + " ".join(str(x) for x in o["circle"])
                    s += " " + " ".join(str(x) for p in o["parts"] for x in p)
            parts.append(s)
        cm = sc["cmnd"]
        parts.append(" ".join(str(x) for x in [len(cm[0])] + cm[0] + [len(cm[1])] + cm[1] + [len(cm[2])] + cm[2]))
        parts.append(str(PATNUM))
        hand = sc.get("hand")
        parts.append(" ".join(str(x) for x in [1] + hand) if hand else "0")
        return " ".join(parts)

    def walk(self, head, n=64):
        out, a = [], head
        while a and len(out) < n:
            out.append(self.index_of(a))
            a = self.m.load(a + 0x1C4, 4)
        return out

    def walk_cmnd(self, root):
        out, a = [], self.m.load(root, 4)
        while a and len(out) < 64:
            out.append(self.index_of(a))
            a = self.m.load(a + 0xBC, 4)
        return out

    def read_scene(self, full=True):
        m, a = self.m, self.addr
        ctrl = self.get(CTRL, "<4i")
        lists = []
        for k in range(4):
            num, head, foot = self.get(CTRL + 0x10 + 12 * k, "<iII")
            ctrl += [num, self.index_of(head) if head else -1, self.index_of(foot) if foot else -1]
            lists.append(self.walk(head))
        objs = []
        for i, va in enumerate(self.addrs):
            kind = 0 if i in self.deleted else self.kinds[i]
            pre, nxt = m.load(va + 0x1C0, 4), m.load(va + 0x1C4, 4)
            if kind == 5:
                objs.append({"kind": 5, "char": self.read_full(i, False), "link": [-1, -1]})
                continue
            o = {"kind": kind, "char": self.read_char(va),
                 "link": [self.index_of(pre) if pre else -1, self.index_of(nxt) if nxt else -1]}
            if kind == 1:
                eid = s32(m.load(va + 0x244, 4))
                self.ea.cur = eid
                o["enemy"] = self.ea.read_enemy(va, eid)
                o["extra"] = self.read_enemy_extra(va)
                if self.motion:
                    o["char"] = self.read_full(i, True)
                    o["extra"] += self.get(va + 0x190, "<4I") + [m.load(va + 0x1A0, 4)]
            elif kind in (2, 3, 4):
                o["obj"] = self.read_common(va)
                if kind == 3:
                    o["cls"] = self.read_class(va)
                    person = m.load(va + 0x98, 4)
                    o["aff"] = [s16(m.load(va + 0x9C, 2)), self.index_of(person) if person else -1]
                if kind == 2:
                    o["circle"] = [m.load(va + 0x1E0, 1) & 1, s32(m.load(va + 0x1E4, 4))]
                    parts = self.read_parts(va)
                    o["parts"] = parts if full else digest([x for p in parts for x in p])
            if kind == 0 and i < len(self.addrs) and self.kinds[i] == 0:
                o["link"] = [self.index_of(pre) if pre else -1, self.index_of(nxt) if nxt else -1]
            objs.append(o)
        g = self.g
        save = [s16(m.load(SAVE + at, 2)) for at in COUNTERS]
        save += [s32(x) for x in self.get(SAVE + FOUNTAIN, "<100I")] + [s16(m.load(SAVE + FOUNTAIN_COUNT, 2))]
        gm = s32(m.load(TB.GAME + 0x24, 4))
        field = min(max(gm, 0), 160)
        ev = self.get(SAVE + EV_ENTRY + 24 * field, "<6i")
        extra = {"ibd": m.load(TB.GAME + 0x60, 4)}
        return {**extra, "ctrl": ctrl, "lists": lists, "objs": objs,
                "cmnd": [self.walk_cmnd(r) for r in self.gb_cmnd],
                "calls": self.wcalls + self.scalls, "outs": list(self.outs),
                "cc": self.ea.cc_state(), "rnds": m.load(a["lastRnd"], 2), "save": save, "ev": ev}

    def restore(self):
        for i in range(303):
            self.g.restore(ENEMY_TBL + ROW * i, ROW)

    # the party and the enemies' full state (the enemy motion harness's) ---------------
    def put_pc(self, va, pc):
        """A party member (a battle.PC) as the enemy motion harness lays one
        out: its ccSpcParam at +0x400, positions, skill, the affect words
        its influence reads."""
        g, m = self.g, self.m
        g.put_char(pc, va, va + 0x400, va + 0x800)
        self.put(va + 0x40, "<4I", *pc.pos)
        self.put(va + 0x50, "<4I", *pc.pos_p)
        m.store(m.load(va, 4) + 0x1C, 4, pc.width)
        self.put(va + 0x7C, "<hh", pc.skill_id, pc.skill_status)
        self.put(va + 0xF4, "<h", pc.anm_flag)
        m.store(va + 0x94, 4, self.affect_funcs.get(pc.affect_func, 0))
        self.put(va + 0x9C, "<h", pc.affect_type)
        m.store(va + 0xB0, 4, pc.affect_mask)
        m.store(va + 0x78, 4, 0)
        m.store(va + 0xE0, 4, m.load(va + 0xE0, 4) | pc.spc_flags)
        self.put(va + 0xEE, "<h", pc.act_num)
        m.store(va + 0x200, 1, pc.fellow_flags)
        m.store(va + 0xE4, 4, pc.arms_effect_sw)
        self.put(va + 0xFE, "<h", pc.attack)
        m.store(va + 0x114, 4, pc.cnt)
        m.store(va + 0x110, 4, pc.cloak)

    def read_full(self, i, foe):
        """A character as the enemy motion harness reads it."""
        g, m = self.g, self.m
        a = self.addrs[i]
        pva = m.load(a + 4, 4) if foe else a + 0x400
        d = g.read_char(a, pva, not foe)
        if foe:
            # PPrestore is ccBossParam's; an enemy's ccEnemyParam has the
            # base pointer there (the enemy AI harness keeps its own too)
            d["PP"][2] = 0
        d["pos"] = self.get(a + 0x40, "<4I")
        d["posP"] = self.get(a + 0x50, "<4I")
        d["skill"] = self.get(a + 0x7C, "<hh")
        person = m.load(a + 0x98, 4)
        d["affect"] = (list(g.shorts(a + 0x9C, 4)) + [self.index_of(person) if person else -1]
                       + list(g.shorts(a + 0xA8, 2)) + [m.load(a + 0xAC, 4)] + list(g.shorts(a + 0xB4, 2))
                       + [m.load(a + 0xB8, 4)])
        d["spc"] = None if foe else [g.shorts(a + 0xEE, 1)[0], m.load(a + 0xE0, 4) & ~(0x80 | 7 << 14),
                                     m.load(a + 0x200, 1), 0, s32(m.load(a + 0xE4, 4)), g.shorts(a + 0xFE, 1)[0],
                                     s32(m.load(a + 0x114, 4)), m.load(a + 0x110, 4)]
        tc = m.load(a + 0x78, 4)
        d["target"] = self.index_of(tc) if tc else -1
        return d

    def kill_rows(self):
        """The enemy book's rows with a record: row, count, server, A, B, C."""
        m = self.m
        out = []
        for r in range(313):
            n = m.load(SAVE + KILL_COUNT + r, 1, True)
            if n:
                a = SAVE + KILL_AREA + 8 * r
                out.append([r, n] + [m.load(a + 2 * k, 2, True) for k in range(4)])
        return out

    def call(self, fn, args=(), flt=None):
        if flt is not None:
            for k, v in enumerate(flt):
                self.m.f[12 + k] = v
        return self.m.call(fn if isinstance(fn, int) else self.sym(fn), tuple(x & 0xFFFFFFFF for x in args))


def harness(checks):
    h = getattr(checks, "_spawn", None)
    if h is None:
        h = checks._spawn = Harness(checks)
    return h


def request(h, sc, op, args=""):
    return f"spawn {h.ser_scene(sc)} {op}" + (f" {args}" if args else "")


def ent_str(ent):
    return " ".join(str(x) for x in ent)


# the checks ------------------------------------------------------------------------------

def check_routine(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(0, 2), nc=rnd.randrange(0, 2), ng=rnd.randrange(0, 2), nn=0)
    cands = [i for i, o in enumerate(sc["objs"]) if o["kind"] in (1, 2, 3)]
    if not cands:
        sc = h.rnd_scene(rnd, ne=1, nc=0, ng=0, nn=0)
        cands = [1]
    w = rnd.choice(cands)
    o = sc["objs"][w]
    if rnd.random() < 0.5:
        # put it near Kite's 7000 and 10000 marks
        d = rnd.choice((6999.0, 7000.0, 7001.0, 9999.5, 10000.0, 10000.5, rnd.uniform(0, 14000)))
        a = rnd.uniform(-math.pi, math.pi)
        k = sc["k"]
        pos = [eemu.f_add(k[0], fb(d * math.sin(a))), eemu.f_add(k[1], fb(-d * math.cos(a))), o["char"][3], F_ONE]
        o["char"][1:5] = pos
    h.put_scene(sc)
    h.call("routine__10ccEntryObjFv", (h.addrs[w],))
    out = h.read_scene()
    h.restore()
    return request(h, sc, "routine", str(w)), out, "routine"


def check_obj_check(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd)
    ent = h.rnd_ent(rnd, sc)
    if rnd.random() < 0.3:
        ent[9] = rnd.choice((6, 20, 15, 0, 3, 40, 44))
        ent[8] = 1
    h.put_scene(sc)
    h.put_ent(EP, ent)
    ret = h.call("entryObjectCheck__11ccEntryCtrlFP12ccEntryParamP10ccEntryObj", (CTRL, EP, 0)) & 0xFF
    out = h.read_scene()
    out["ret"] = ret
    out["ep"] = h.read_ent(EP)
    h.restore()
    return request(h, sc, "check", ent_str(ent)), out, "check"


def check_entry(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(0, 3), nc=rnd.randrange(0, 2), ng=rnd.randrange(0, 2), nn=0)
    ent = h.rnd_ent(rnd, sc)
    op = rnd.choice(("entry", "entry", "entryn", "mc"))
    n = rnd.choice((1, 1, 2, 3, 4, 5, 0))
    if op == "mc":
        ent[8], ent[9] = 1, 15
    h.put_scene(sc)
    h.put_ent(EP, ent)
    fn = {"entry": "entryObject__11ccEntryCtrlFP12ccEntryParam",
          "entryn": "entryObject__11ccEntryCtrlFP12ccEntryParami",
          "mc": "entryMagicCircle__11ccEntryCtrlFP12ccEntryParam"}[op]
    ret = h.call(fn, (CTRL, EP, n))
    out = h.read_scene()
    out["ret"] = h.index_of(ret) if ret else -1
    out["ep"] = h.read_ent(EP)
    h.restore()
    return request(h, sc, op, ent_str(ent) + (f" {n}" if op == "entryn" else "")), out, op


def check_race(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0)
    rows = h.ported_rows()
    k = getattr(checks, "_race_k", 0)
    row = rows[k % len(rows)]
    checks._race_k = k + 1
    race = next(r for r in BUILT if h.race_first[r] <= row < h.race_first[r] + h.race_num[r])
    ent = h.rnd_ent(rnd, sc, 0, row)
    ent[21] = rnd.choice((0, 1, 2, 3, -1, rnd.randrange(-40000, 40000)))
    sc["attr"] = sc["attr"] or [inf_va(0x00304050)]
    h.put_scene(sc)
    h.put_ent(EP, ent)
    entry = ENEMY_TBL + ROW * row + 0x68
    h.m.store(entry + 0xC, 4, EP)
    func = h.m.load(RACE_TBL + 12 * race, 4)
    h.call(func, (entry,))
    out = h.read_scene()
    out["think"] = h.get(ENEMY_TBL + ROW * row + 0xB0, "<3I")
    h.restore()
    return request(h, sc, "race", f"{race} {ent_str(ent)}"), out, "race"


def check_circle_object(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, nc=1)
    w = next(i for i, o in enumerate(sc["objs"]) if o["kind"] == 2)
    o = sc["objs"][w]
    ent = o["obj"][14:36]
    ent[19], ent[20] = h.rnd_gift(rnd)
    ent[21] = rnd.choice((-1, -1, 1, 2, 3, 0))
    ent[16] = rnd.choice((0, 0, -1, 1))
    o["obj"][14:36] = ent
    h.put_scene(sc)
    h.call("entryCircleObject__11ccEntryCtrlFP10ccEntryObj", (CTRL, h.addrs[w]))
    out = h.read_scene()
    h.restore()
    return request(h, sc, "circleobj", str(w)), out, "circle_object"


def check_enemy_object(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(1, 3))
    w = rnd.choice([i for i, o in enumerate(sc["objs"]) if o["kind"] == 1])
    o = sc["objs"][w]
    o["enemy"]["ent"][20] = rnd.choice((0, 1, -1, 5))
    h.put_scene(sc)
    h.call("entryEnemyObject__11ccEntryCtrlFP10ccEntryObj", (CTRL, h.addrs[w]))
    out = h.read_scene()
    h.restore()
    return request(h, sc, "enemyobj", str(w)), out, "enemy_object"


def check_init_object(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=1, nc=1, ng=1, nn=rnd.randrange(0, 2))
    w = rnd.choice([i for i, o in enumerate(sc["objs"]) if o["kind"] in (1, 2, 3, 4)])
    o = sc["objs"][w]
    ent = o["enemy"]["ent"] if o["kind"] == 1 else o["obj"][14:36]
    if rnd.random() < 0.6:
        ent[10:14] = list(sc["ctrl"])
    if o["kind"] != 1:
        o["obj"][14:36] = ent
        o["obj"][36] = rnd.choice((0, 1))
    h.put_scene(sc)
    h.call("initObject__11ccEntryCtrlFP10ccEntryObj", (CTRL, h.addrs[w]))
    out = h.read_scene()
    h.restore()
    return request(h, sc, "init", str(w)), out, "init_object"


def check_delete(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(1, 4), nc=rnd.randrange(1, 3), ng=rnd.randrange(1, 3),
                     nn=rnd.randrange(1, 2))
    kind = rnd.choice((1, 2, 3, 4))
    lst = sc["lists"][kind - 1]
    members, a = [], lst[1]
    while a != -1 and len(members) < 20:
        members.append(a)
        a = sc["objs"][a]["link"][1]
    if not members:
        return check_delete(checks, rnd)
    w = rnd.choice(members)
    h.put_scene(sc)
    fn = {1: "deleteEnemy", 2: "deleteMagicCircle", 3: "deleteGimmick", 4: "deleteNpc"}[kind]
    h.call(fn + "__11ccEntryCtrlFP10ccEntryObj", (CTRL, h.addrs[w]))
    out = h.read_scene()
    h.restore()
    return request(h, sc, "delete", f"{kind - 1} {w}"), out, "delete"


def check_active(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(0, 4), nc=rnd.randrange(0, 3), ng=rnd.randrange(0, 2), nn=0)
    for o in sc["objs"]:
        if o["kind"] == 1 and rnd.random() < 0.5:
            o["enemy"]["eflags"] &= ~5
        if rnd.random() < 0.5:
            f, b = rnd.randrange(-1, 3), rnd.randrange(-1, 3)
            if o["kind"] == 1:
                o["enemy"]["ent"][12:14] = [f, b]
            elif o["kind"] in (2, 3):
                o["obj"][14 + 12:14 + 14] = [f, b]
    f, b = rnd.randrange(-1, 3), rnd.randrange(-1, 3)
    h.put_scene(sc)
    r = [s32(h.call("ccCheckActiveEnemy__Fv")), h.call("ccCheckActiveObject__Fv") & 0xFF,
         h.call("ccCheckActiveObject__Fii", (f, b)) & 0xFF]
    out = h.read_scene()
    out["ret"] = r
    h.restore()
    return request(h, sc, "active", f"{f} {b}"), out, "active"


def circle_tweak(h, rnd, sc, w):
    o = sc["objs"][w]
    ob = o["obj"]
    ob[49] = rnd.choice((0, 0, 1, 2, 3, 4))
    ob[50] = rnd.choice((0, 1, 15, 16, 20, 21, 22, 30, 31, 32, 64, 65, 66))
    ob[51] = rnd.choice((0, 1))
    ob[7] = rnd.choice((fb(2999.0), fb(3000.0), fb(3001.0), fb(6999.0), fb(7000.0), rf(rnd, 0, 9000)))
    if rnd.random() < 0.7:
        ob[6] &= ~4
    if rnd.random() < 0.3:
        ob[6] |= 0x10
    ent = ob[14:36]
    ent[16] = rnd.choice((0, 1, 1, -1))
    ent[19], ent[20] = h.rnd_gift(rnd)
    ent[21] = rnd.choice((-1, 1, 2))
    ob[14:36] = ent
    return o


def check_circle_main(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, nc=rnd.randrange(1, 3))
    w = rnd.choice([i for i, o in enumerate(sc["objs"]) if o["kind"] == 2])
    circle_tweak(h, rnd, sc, w)
    if rnd.random() < 0.5:
        sc["lists"][1][0] = rnd.choice((1, 2))
    h.put_scene(sc)
    va = h.addrs[w]
    ret = h.call("main__13ccMagicCircleFv", (va,)) & 0xFF
    out = h.read_scene()
    out["ret"] = ret
    h.restore()
    return request(h, sc, "cmain", str(w)), out, "circle_main"


def check_frame(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(0, 3), nc=rnd.randrange(1, 4), ng=rnd.randrange(0, 2),
                     nn=rnd.randrange(0, 2), area=rnd.choice((1, 2)))
    for w, o in enumerate(sc["objs"]):
        if o["kind"] == 2:
            circle_tweak(h, rnd, sc, w)
            o["obj"][49], o["obj"][50] = rnd.choice(((0, 0), (0, 0), (1, 25), (2, 15), (4, 60)))
            o["obj"][6] |= 1
    n = rnd.randrange(20, 91)
    # Kite walks from afar toward a circle and away (the player's frame
    # moves with him; his position is the frame's origin).
    target = next(o for o in sc["objs"] if o["kind"] == 2)["char"][1:3]
    start = [eemu.f_add(target[0], fb(rnd.uniform(-9000, 9000))), eemu.f_add(target[1], fb(rnd.uniform(-9000, 9000)))]
    path = []
    for f in range(n):
        t = (f / max(n - 1, 1)) * 2
        t = t if t <= 1 else 2 - t
        x = struct.unpack("<f", struct.pack("<I", start[0]))[0]
        y = struct.unpack("<f", struct.pack("<I", start[1]))[0]
        tx = struct.unpack("<f", struct.pack("<I", target[0]))[0]
        ty = struct.unpack("<f", struct.pack("<I", target[1]))[0]
        path.append([fb(x + (tx - x) * t * 1.05), fb(y + (ty - y) * t * 1.05)])
    sc["k"] = path[0]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(200)]
    sc["attr"] = [inf_va(0x00304050), 0x00B0C000] * 20
    sc["camdeg"] = [int(rnd.random() < 0.8) for _ in range(400)]
    sc["camtr"] = [rnd.choice((F_ONE, rf(rnd, 0, 1))) for _ in range(400)]
    sc["fwd"] = [int(rnd.random() < 0.05) for _ in range(400)]
    sc["emain"] = [int(rnd.random() < 0.02) for _ in range(400)]
    sc["gmain"] = [int(rnd.random() < 0.02) for _ in range(400)]
    sc["nmain"] = [int(rnd.random() < 0.02) for _ in range(400)]
    h.put_scene(sc)
    frames = []
    for f in range(n):
        h.k = list(path[f])
        # Kite (object 0) stands at the frame's origin.
        h.put(h.addrs[0] + 0x40, "<II", *path[f])
        h.wcalls, h.scalls, h.outs = [], [], []
        h.breaths = 0
        try:
            h.call(TRAMP)
        except FrameDone:
            pass
        frames.append(h.read_scene(full=False))
    last = h.read_scene(full=True)
    last["calls"], last["outs"] = [], []
    h.restore()
    req = request(h, sc, "frame", f"{n} " + " ".join(f"{p[0]} {p[1]}" for p in path))
    return req, {"frames": frames, "last": last}, "frame"


def check_world_mc(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=1)
    sc["game"][2] = rnd.choice((0, 0, 14, rnd.randrange(1, 60)))
    sc["ctrl"] = [1, sc["game"][2], sc["game"][4], sc["game"][5]]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc["attr"] = [inf_va(0x00304050)] * 40
    dens = rnd.choice((0.0, 0.2, 0.4, 0.6))
    check = [int(rnd.random() < dens) for _ in range(1600)]
    sx, sy = rnd.uniform(0, 48000), rnd.uniform(0, 48000)
    start = [fb(sx), fb(sy), 0, F_ONE]
    # every block keeps a free chip more than 4000 from the start, or the
    # game (and the port) would search it for ever
    for bx in range(4):
        for by in range(4):
            for _ in range(12):
                while True:
                    cx, cy = bx * 10 + rnd.randrange(10), by * 10 + rnd.randrange(10)
                    if math.hypot(600 + 1200 * cx - sx, 600 + 1200 * cy - sy) > 4000:
                        break
                check[cx * 40 + cy] = 0
    heights = [rf(rnd, -50, 600) if rnd.random() < 0.8 else 0 for _ in range(6400)]
    check3 = [int(rnd.random() < 0.05) for _ in range(1600)]
    seed = rnd.getrandbits(32) % 0xFFFFFFFE
    ofs = rnd.choice((0, 1, 2, 3, 7))
    ev_area = rnd.choice((0, 14, 14, rnd.randrange(1, 100)))
    h.put_scene(sc)
    m = h.m
    m.mem[WORLDO:WORLDO + 0x61E0] = bytes(0x61E0)
    m.store(WORLDO + 0x6168, 4, FIELD)
    m.mem[FIELD + 0xAF02C:FIELD + 0xAF02C + 4 * 6400] = struct.pack("<6400I", *heights)
    m.mem[FIELD + 0xCE430:FIELD + 0xCE430 + 1600] = bytes(check)
    m.mem[FIELD + 0xCF0B0:FIELD + 0xCF0B0 + 1600] = bytes(check3)
    m.store(WMAN + 0x120, 4, ev_area)
    m.store(WP + 0x2C, 4, ofs)
    h.put(WMAN + 0x450, "<4I", *start)
    m.store(h.addr["seed"], 4, seed)
    h.call("SetMagicCircle__5WORLDFv", (WORLDO,))
    out = h.read_scene()
    out["seed"] = m.load(h.addr["seed"], 4)
    h.restore()
    args = (" ".join(map(str, check)) + " " + " ".join(map(str, heights)) + " " + " ".join(map(str, check3))
            + f" {seed} {ofs} {ev_area} " + " ".join(map(str, start)))
    return request(h, sc, "worldmc", args), out, "world_mc"


def check_dungeon_mc(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=2)
    sc["ctrl"] = [2, sc["game"][3], rnd.randrange(0, 3), rnd.randrange(0, 5)]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    dtype = rnd.choice((0, 1, 3, 8, 9))
    story = int(rnd.random() < 0.5)
    counter = rnd.randrange(0, 20)
    nslot = rnd.randrange(0, 30)
    slots = []
    for _ in range(nslot):
        slots.append([rf(rnd, 0, 60000), rf(rnd, 0, 60000), rf(rnd, -100, 100), F_ONE,
                      rnd.randrange(0, 3), rnd.randrange(0, 12), rnd.choice((0, 1, 1, 2, 3, 4, 5, 6, -1))])
    slots += [[0, 0, 0, 0, 0, 0, 0]] * (750 - nslot)
    edit = None
    if story and rnd.random() < 0.9:
        edit = [[rnd.randrange(0, 3), rnd.randrange(0, 8), rnd.randrange(0, 60000), rnd.randrange(0, 60000),
                 rnd.choice((0, 1, 1, 2, 3))] for _ in range(rnd.randrange(0, 12))]
    h.put_scene(sc)
    m = h.m
    m.mem[DUNG:DUNG + 0x33520] = bytes(0x33520)
    m.store(DUNG + 0x10, 4, dtype)
    m.store(DUNG + 0x14, 4, story)
    m.store(DUNG + 0x3C, 4, counter)
    m.store(DUNG + 0x424, 4, GIMPOS)
    later = volume.NAME != "infection"
    if later:       # ten floors (put_dungeon)
        m.store(DUNG + 0x430, 4, 10)
    for k, s in enumerate(slots):
        a = GIMPOS + 0x30 * k
        h.put(a, "<4I", *s[:4])
        h.put(a + 0x20, "<bbb", *s[4:7])
    if edit is not None:
        m.store(DUNG + (0x934 if later else 0x3351C), 4, EDIT)
        m.store(EDIT + 0x10, 4, EDIT_ROWS)
        m.store(EDIT + 0x14, 4, len(edit))
        for k, r in enumerate(edit):
            h.put(EDIT_ROWS + 0x20 * k, "<8i", r[0], r[1], r[2], r[3], r[4], 0, 0, 0)
    # the w a story circle's position takes from the stack (see entry.rs)
    m.store(eemu.STACK_TOP - 4, 4, F_ONE)
    h.call("SetMagicCircle__7DUNGEONFv", (DUNG,))
    out = h.read_scene()
    out["counter"] = s32(m.load(DUNG + 0x3C, 4))
    out["kinds"] = [s16(m.load(GIMPOS + 0x30 * k + 0x22, 1) << 8) >> 8 for k in range(750)]
    h.restore()
    args = f"{dtype} {story} {counter} 750 " + " ".join(" ".join(map(str, s)) for s in slots)
    args += " -1" if edit is None else f" {len(edit)} " + " ".join(" ".join(map(str, r)) for r in edit)
    return request(h, sc, "dungeonmc", args), out, "dungeon_mc"


def rnd_evpos(rnd):
    out = []
    nums = rnd.sample(range(0, 8), 5) + [100, 101]
    for k in range(16):
        num = nums[k] if k < len(nums) and rnd.random() < 0.8 else -1
        fl = rnd.choice((9999, 9999, 0, 1, 10000))
        bl = rnd.choice((9999, 9999, 3, 10000)) if fl >= 9999 else rnd.randrange(0, 5)
        out.append([fl, bl, num, rf(rnd, -3.1, 3.1), rf(rnd, -5000, 5000), rf(rnd, -5000, 5000),
                    rf(rnd, 0, 300), F_ONE])
    return out


def check_event_mc(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=rnd.choice((1, 2)))
    sc["game"][9] = 0
    sc["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc["attr"] = [inf_va(0x00304050)] * 40
    kite = sc["objs"][0]["char"][1:5]
    evp = rnd_evpos(rnd)
    enemy = rnd.random() < 0.35
    h.put_scene(sc)
    m = h.m
    m.store(h.addr["eventMng"], 4, EVT)
    m.mem[EVT:EVT + 0x800] = bytes(0x800)
    for k in range(16):
        h.put(EVT + 0x80 + 8 * k, "<4h", -1, 0, 0, 0)
        h.put(EVT + 0x100 + 8 * k, "<4h", -1, 0, 0, 0)
    for k, p in enumerate(evp):
        a = EVT + 0x1C0 + 32 * k
        h.put(a, "<hhiI", p[0], p[1], p[2], p[3])
        h.put(a + 0x10, "<4I", *p[4:8])
    if enemy:
        e = [rnd.choice((5, 6)), rnd.choice(h.ported_rows()), rnd.choice((0, 1, 2, 3, 100, 101, 7)),
             rnd.choice((0, 1, 2, 3))]
        h.put(EVT + 0x80, "<4h", *e)
        mcs = []
    else:
        mcs = [[-1, 0, 0, 0] for _ in range(16)]
        for k in rnd.sample(range(16), rnd.randrange(1, 5)):
            ty = rnd.choice((0, 0, 1, 2))
            code = rnd.choice(h.ported_rows()) if ty == 0 else rnd.choice((0, 1, 2))
            mcs[k] = [ty, code, rnd.choice((0, 1, 2, 3, 100, 7)), rnd.choice((0, 1, 2, 3))]
        for k, e in enumerate(mcs):
            h.put(EVT + 0x100 + 8 * k, "<4h", *e)
    h.call("ccEntryEventMng__Fv")
    out = h.read_scene()
    h.restore()
    ev = " ".join(" ".join(map(str, p)) for p in evp)
    if enemy:
        req = request(h, sc, "evenemy", " ".join(map(str, kite)) + " " + ev + " " + " ".join(map(str, e)))
    else:
        req = request(h, sc, "eventmc", " ".join(map(str, kite)) + " " + ev + " "
                      + " ".join(" ".join(map(str, e)) for e in mcs))
    return req, out, "event_mc"


def check_entry_gimmick(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=rnd.choice((1, 2)))
    flag = rnd.choice((1, 2, 0))
    ef = [rnd.choice((0, 0, 1)) for _ in range(4)]
    eaf = rnd.choice((0, 0, 1))
    h.put_scene(sc)
    m, sym = h.m, h.sym
    m.store(WMAN + 8, 4, flag)
    h.put(WMAN + 0x54, "<4i", *ef)
    m.store(WMAN + 0x124, 4, eaf)
    for k in range(3):
        m.store(WMAN + 0x438 + 4 * k, 4, DUNG)
    m.store(WMAN + 0x434, 4, WORLDO)
    names = {"SetFood__5WORLDFv": "SetFood", "SetMagicCircle__5WORLDFv": "WORLD::SetMagicCircle",
             "SetSpecialObj__5WORLDFv": "SetSpecialObj", "SetItemBox__7DUNGEONFv": "SetItemBox",
             "SetMagicCircle__7DUNGEONFv": "DUNGEON::SetMagicCircle", "SetIDOL__7DUNGEONFv": "SetIDOL",
             "EntryBreakObject__7DUNGEONFv": "EntryBreakObject"}
    saved = {}
    for n, label in names.items():
        a = sym(n)
        saved[a] = m.hooks.get(a)
        m.hooks[a] = (lambda lb: lambda mm, *_: (h.scalls.append([lb]), 0)[1])(label)
    try:
        h.call("EntryGimmick__9WORLD_MANFv", (WMAN,))
    finally:
        for a, x in saved.items():
            if x is None:
                del m.hooks[a]
            else:
                m.hooks[a] = x
    out = h.read_scene()
    out["wm"] = h.get(WMAN + 0x54, "<4i")
    h.restore()
    return request(h, sc, "entrygim", f"{flag} {' '.join(map(str, ef))} {eaf}"), out, "entry_gimmick"


def check_restore(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(0, 3), nc=rnd.randrange(0, 3), ng=rnd.randrange(0, 2),
                     nn=rnd.randrange(0, 2))
    for o in sc["objs"]:
        ent = o["enemy"]["ent"] if o["kind"] == 1 else (o["obj"][14:36] if o["kind"] in (2, 3, 4) else None)
        if ent is not None and rnd.random() < 0.6:
            ent[10:14] = list(sc["ctrl"])
            if o["kind"] != 1:
                o["obj"][14:36] = ent
    saved = [list(l) for l in sc["lists"]]
    sc["lists"] = [[0, -1, -1] for _ in range(4)]
    h.put_scene(sc)
    gl = h.addr["g_entryList"]
    for k, l in enumerate(saved):
        h.put(gl + 4 + 12 * k, "<iII", l[0], h.ptr(l[1]), h.ptr(l[2]))
    h.call("restoreEntry__11ccEntryCtrlFv", (CTRL,))
    out = h.read_scene()
    h.restore()
    return request(h, sc, "restore", " ".join(str(x) for l in saved for x in l)), out, "restore"


def check_leave(checks, rnd):
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=rnd.randrange(0, 4), nc=rnd.randrange(0, 3), ng=rnd.randrange(0, 3),
                     nn=rnd.randrange(0, 2))
    for o in sc["objs"]:
        if o["kind"] == 1:
            o["enemy"]["ent"][16] = rnd.choice((-1, 0, 1, 2))
        elif o["kind"] in (2, 3, 4):
            o["obj"][30] = rnd.choice((-1, 0, 1, 2))
            o["obj"][10] = rnd.choice((0, 1, 2))
    replace, mode = rnd.choice((0, 0, 1)), rnd.choice((5, 5, 6))
    h.put_scene(sc)
    m = h.m
    saved = m.hooks.get(h.sym("CheckSceneReplace__6ccGameFv"))
    m.hooks[h.sym("CheckSceneReplace__6ccGameFv")] = lambda mm, *_: replace
    m.store(TB.GAME, 4, mode)
    try:
        h.call("ccThEntryCtrlDelete__FPv", (CTRL,))
    finally:
        if saved is None:
            del m.hooks[h.sym("CheckSceneReplace__6ccGameFv")]
        else:
            m.hooks[h.sym("CheckSceneReplace__6ccGameFv")] = saved
    out = h.read_scene()
    gl = h.addr["g_entryList"]
    sv = []
    for k in range(4):
        num, head, foot = h.get(gl + 4 + 12 * k, "<iII")
        sv += [num, h.index_of(head) if head else -1, h.index_of(foot) if foot else -1]
    out["saved"] = sv
    # the control itself is freed; the probe keeps its lists as they were
    out["ctrl"] = out["ctrl"][:4] + sv
    out["lists"] = [h.walk(h.ptr(l[1])) for l in [sv[3 * k:3 * k + 3] for k in range(4)]]
    h.restore()
    keep = int(replace == 0 and mode == 5)
    return request(h, sc, "leave", str(keep)), out, "leave"


def check_register(checks, rnd):
    h = harness(checks)
    m = h.m
    ops = [[0]]
    for _ in range(rnd.randrange(1, 5)):
        r = rnd.random()
        s, t = rnd.randrange(0, 5), rnd.randrange(0, 7)
        n = m.load(inf_va(0x005DA200) + 56 * s + 8 * t + 4, 4)
        rank = rnd.randrange(0, max(n, 1))
        if r < 0.4:
            ops.append([1, s, t, rank])
        elif r < 0.7:
            ops.append([2, rnd.randrange(0, 303)])
        else:
            ops.append([3, s, t, rank])
    if rnd.random() < 0.2:
        m.store(h.addr["ccRegisterEnemyRange"], 4, 3)
    an = []
    for op in ops:
        if op[0] == 0:
            h.call("ccInitRegisterEnemy__Fv")
        elif op[0] == 1:
            h.call("ccRegisterEnemyList__Fiii", tuple(op[1:]))
        elif op[0] == 2:
            h.call("ccRegisterEnemyOne__Fi", (op[1],))
        else:
            an.append(s32(h.call("ccAnalyzeEnemyList__Fiii", tuple(op[1:]))))
    a = h.addr
    out = {"cells": h.get(a["ccRegisterEnemyTbl"], "<24i"),
           "nums": [s32(m.load(a["ccRegisterEnemyNum"], 4)), s32(m.load(a["ccRegisterDrainNum"], 4)),
                    s32(m.load(a["ccRegisterEnemyRange"], 4))],
           "exist": [i for i in range(303) if m.load(ENEMY_TBL + ROW * i + 0x68, 4)], "an": an}
    h.restore()
    req = f"sreg {len(ops)} " + " ".join(" ".join(map(str, op)) for op in ops)
    return req, out, "register"


def check_save(checks, rnd):
    h = harness(checks)
    m = h.m
    kind = rnd.choice(("ev", "ev", "dust", "rands"))
    if kind == "dust":
        attr = rnd.choice((0x00B0C000, inf_va(0x00304050), inf_va(0x00606060), 0x00E0E0E0, 0x008080F0, inf_va(0x0070C0E0),
                           0x00D0D0D0, rnd.getrandbits(32))) | (rnd.getrandbits(32) & 0xFF0F0F0F if rnd.random() < 0.5 else 0)
        pos = [rf(rnd, -1000, 1000) for _ in range(3)] + [F_ONE]
        h.reset_state()
        h.land, h.attr = [0], [attr]
        h.put(EP, "<4I", *pos)
        m.mem[EP + 0x40:EP + 0x50] = struct.pack("<4I", *pos)
        v = s16(h.call("ccCheckDustColor__FP6ccChar", (EP,)))
        return f"dust {attr} {' '.join(map(str, pos))}", {"ret": v, "calls": h.wcalls}, "dust"
    if kind == "rands":
        s = rnd.getrandbits(16)
        n = rnd.randrange(1, 40)
        m.store(h.addr["lastRnd"], 2, s)
        v = [s16(h.call("ccRandS__Fv")) for _ in range(n)]
        return f"rands {s} {n}", {"v": v, "s": m.load(h.addr["lastRnd"], 2)}, "rands"
    field = rnd.choice((0, 14, 160, 161, rnd.randrange(1, 160)))
    server = rnd.randrange(0, 5)
    words = [rnd.getrandbits(32) for _ in range(6)]
    fount = [rnd.getrandbits(31) for _ in range(100)]
    ops = []
    for _ in range(rnd.randrange(1, 10)):
        op = rnd.choice((0, 0, 1, 2))
        if op == 1:
            code = rnd.getrandbits(24)
            if rnd.random() < 0.5:
                k = rnd.randrange(100)
                fount[k] = s32(code | (server << 29))
            ops.append([1, code])
        else:
            ops.append([op, rnd.choice((rnd.randrange(0, 192), -2, -33, -40, 191, 192, 200, 31, 32))])
            if rnd.random() < 0.3:
                ops[-1] = [3, ops[-1][1], rnd.choice((0, 0, -1, 1))]
    m.store(TB.GAME + 0x24, 4, field)
    m.store(TB.GAME + 0x1C, 4, server)
    f = min(max(field, 0), 160)
    m.mem[SAVE + EV_ENTRY - 0x40:SAVE + 0x6870] = bytes(0x6870 - EV_ENTRY + 0x40)
    h.put(SAVE + EV_ENTRY + 24 * f, "<6I", *words)
    h.put(SAVE + FOUNTAIN, "<100i", *[s32(x) for x in fount])
    r = []
    for op in ops:
        if op[0] == 3:
            h.put_ent(EP, [0] * 16 + [op[2], -1, op[1], -1, -1, -1])
            h.call("deleteEventEntry__11ccEntryCtrlFP12ccEntryParam", (CTRL, EP))
            r.append(0)
            continue
        fn = {0: "CheckEventEntry__10ccSaveDataFi", 1: "CheckFountain__10ccSaveDataFi",
              2: "ClearEventEntry__10ccSaveDataFi"}[op[0]]
        v = h.call(fn, (SAVE, op[1])) & 0xFF
        r.append(v if op[0] != 2 else 0)
    out = {"r": r, "words": h.get(SAVE + EV_ENTRY + 24 * f, "<6i")}
    req = (f"evsave {field} {server} " + " ".join(map(str, words)) + " "
           + " ".join(str(s32(x)) for x in fount) + f" {len(ops)} " + " ".join(" ".join(map(str, o)) for o in ops))
    return req, out, "evsave"


# the enemies' own frame -------------------------------------------------------------------

AREA14 = (130, 151, 219, 185, 189, 268, 67)       # the first field and dungeon's rows
GOLD = (131, 134, 138, 140, 143, 147, 150, 154, 157)   # gold goblins (goldFlag): each volume and type
# the races' drained forms (their first rows)
DRAINED = (129, 218, 184, 267, 66, 0, 13, 22, 35, 48, 79, 89, 104, 116, 167, 178, 193, 229, 239, 245, 279, 288)
MIDDLE_BOSSES = (54, 55, 58, 59, 60, 64, 65, 72, 76, 77, 78, 88, 95, 96, 110, 111, 112, 115, 127, 128, 164, 165,
                 166, 177, 201, 202, 213, 215, 216, 217, 224, 225, 235, 255, 256, 257, 260, 261, 265, 266, 295,
                 296, 300, 301, 302)
# the other races with their own motion (all but B, G, K, P, V and L): their
# rows but the drained forms and the middle bosses (no animation table)
MORE = tuple(r for r in list(range(0, 66)) + list(range(79, 129)) + list(range(167, 184)) + list(range(193, 203))
             + list(range(207, 218)) + list(range(229, 267)) + list(range(279, 303))
             if r not in DRAINED and r not in MIDDLE_BOSSES)
FOOT_OBJ_TBL = inf_va(0x005E0640)   # ccEnemyE's feet (30-byte names)
NOTE_EVENTS = (1, 2, 0x8005, 0x8003, 0x8002, 3, 0x8004, 0x8001, 0, 5)
NOTES = 0x01A00000                                # the notes _AnimateForward passes
MINUS_ONE = 0xBF800000
ENEMY_G = (inf_va(0x00446070), inf_va(0x004492D0))                # enemyG.cpp: a gold goblin's clink


def m_answer(h, kind, circle=False):
    """The next answer of a world function, drawn when the game asks and
    kept for the probe (the enemy motion harness's distributions)."""
    rnd = h.mrnd
    if kind == "land":
        v = rnd.choice((0, fb(rnd.uniform(-30, 30)), fb(rnd.uniform(-300, 300))))
    elif kind == "attr":
        v = rnd.choice((0x00B0C000, inf_va(0x00304050), inf_va(0x00606060), rnd.getrandbits(32)))
    elif kind == "camdeg":
        v = int(rnd.random() < 0.8)
    elif kind == "camtr":
        v = rnd.choice((F_ONE, F_ONE, rf(rnd, 0, 1), 0))
    elif kind == "collide":
        r = 0 if rnd.random() > h.collide_p else rnd.choice((1, 1, 2, 3))
        v = (r, [rf(rnd, -40, 40), rf(rnd, -40, 40), rf(rnd, -10, 10), rnd.choice((0, F_ONE))], rnd.randrange(0, 8))
    elif kind == "line":
        v = rnd.choice((MINUS_ONE,) * 5 + (fb(rnd.uniform(0, 1)), 0))
    elif kind == "shake":
        v = rnd.choice((0, 1, 1))
    elif circle:
        v = (int(rnd.random() < 0.05), 0, [])
    else:
        ret = rnd.choice((0,) * 5 + (1,))
        nxt = rnd.randrange(0, 0x10000) if rnd.random() < 0.3 else rnd.randrange(0, 200)
        notes = []
        for _ in range(rnd.choice((0, 0, 0, 1, 1, 2, 3))):
            ev = rnd.choice(NOTE_EVENTS + (rnd.randrange(0, 0x10000),))
            notes.append((ev, rnd.choice((0, 1, 2, 3, rnd.randrange(0, 300)))))
        v = (ret, nxt, notes)
    h.mscript[kind].append(v)
    return v


def motion_hooks(h):
    """The hooks of the enemies' own frame: ccEnemy::main, the races' code,
    EntryAffect and the affect functions run; the world answers as the
    game asks (m_answer) and every call is recorded in order."""
    g, m, H = h.g, h.m, h
    rec, b = g.record, h.c.b

    def vec(a):
        return [m.load(a + 4 * k, 4) for k in range(4)]

    def land(mm, pos, mask, *_):
        H.wcalls.append(["land", vec(pos), mask])
        v = m_answer(H, "land")
        mm.f[0] = v
        return v

    def attr(mm, *_):
        H.wcalls.append(["attr"])
        return m_answer(H, "attr")

    def camdeg(mm, pos, deg, *_):
        H.wcalls.append(["camdeg", vec(pos), s16(deg)])
        return m_answer(H, "camdeg")

    def camtr(mm, pos, *_):
        H.wcalls.append(["camtr", vec(pos), [mm.f[12 + k] & 0xFFFFFFFF for k in range(4)]])
        v = m_answer(H, "camtr")
        mm.f[0] = v
        return v

    def collide(mm, this, *_):
        H.wcalls.append(["collide", H.index_of(this - 0x160), mm.load(this + 0x14, 4), mm.load(this + 0x18, 4),
                         vec(this + 0x20)])
        r, off, at = m_answer(H, "collide")
        for k in range(4):
            mm.store(this + 0x30 + 4 * k, 4, off[k])
        mm.store(this + 0x40, 4, at)
        return r

    def line(mm, a, b_, mask, *_):
        H.wcalls.append(["line", vec(a), vec(b_), mask, 1])
        v = m_answer(H, "line")
        mm.f[0] = v
        return v

    def shake(mm, pos, *_):
        H.wcalls.append(["shake", vec(pos)])
        return m_answer(H, "shake")

    def set_anm(mm, anm, chunk_, *_):
        who, slot = H.owner_anm(anm)
        H.wcalls.append(["anim_set", who, slot, H.chunk_names.get(chunk_, "?")])
        mm.store(anm + 0xAC, 4, chunk_ or 1)
        mm.store(anm + 0x98, 4, 0)
        return 0

    def fwd(mm, anm, step, *_):
        who, slot = H.owner_anm(anm)
        H.wcalls.append(["anim_forward", who, slot, step & 0xFFFF])
        circle = 0 <= who < len(H.kinds) and H.kinds[who] == 2
        ret, nxt, notes = m_answer(H, "anim", circle)
        mm.store(anm + 0x98, 4, nxt)
        if slot == 0:
            head = 0
            for ev, p in reversed(notes):
                a = H.note_at
                H.note_at += 0x18
                mm.mem[a:a + 0x18] = bytes(0x18)
                mm.store(a, 4, head)
                mm.store(a + 4, 4, ev)
                mm.store(a + 8, 4, p)
                head = a
            mm.store(anm + 0xA0, 4, head)
        return ret

    def se(mm, n, pos, *_):
        if ENEMY_G[0] <= (mm.r[31] & 0xFFFFFFFF) < ENEMY_G[1]:
            H.wcalls.append(["se3d", s32(n), vec(pos)])
        else:
            H.outs.append(["se", s32(n), vec(pos)])
        return 0

    def char_draw(mm, ch, *_):
        anm = mm.load(ch + 0xD4, 4)
        mat = [mm.load(anm + 0x40 + 4 * k, 4) for k in range(16)]
        H.wcalls.append(["draw", g.name(ch)] + mat)
        # ccAnm::Draw's nodes set the anm's world matrix (_SetLWMatrix: the
        # root has no parent, so its own matrix)
        for k, w in enumerate(mat):
            mm.store(anm + 4 * k, 4, w)
        return 1

    def breath_owner(mm, br):
        for va in H.addrs:
            for slot in (0, 1):
                if mm.load(va + 0x348 + 4 * slot, 4) == br and RACE_VTS.get(mm.load(va + 0x1CC, 4)) in ("H", "L"):
                    return va, slot
        return 0, -1

    def hit_switch(on):
        def f(mm, this, *_):
            i = H.idx.get(this - 0x160)
            if i is not None and H.kinds[i] in (1, 2, 3, 4):
                H.wcalls.append(["hit", i, on])
                mm.store(this, 4, on)
            else:
                rec("HitEnable" if on else "HitDisable", this - 0x1A0)
            return 0
        return f

    def skill_start(mm, a, sk, x, y):
        eid = s32(mm.load(a + 0x244, 4))
        H.wcalls.append(["effSkillStart", g.name(a), H.ea.skcode(sk, eid), s32(x), s32(y)])
        return 0

    return {
        "ccLandHitCheck__FPfUi": land, "checkHitResultAttlibute__Fv": attr,
        "ccCheckCameraDeg__FPfs": camdeg, "ccGetCameraTransparency__FPfffff": camtr,
        "CollisionDetection__9ccCharHitFv": collide, "ccHitCheckLM2__FPfPfUi": line,
        "checkCameraShakeRange__FPf": shake, "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm,
        "_AnimateForward__5ccAnmFUi": fwd, "ccSeOn3D__FiPf": se,
        "HitDisable__9ccCharHitFv": hit_switch(0), "HitEnable__9ccCharHitFv": hit_switch(1),
        "cameraShake__Fiiii": lambda mm, a, b_, c, d: H.wcalls.append(["cameraShake", s32(a), b_, c, d]) or 0,
        "ccSeSetParamEnemy__FUiP6ccChari":
            lambda mm, p, ch, cat, *_: H.wcalls.append(["sound", p, g.name(ch), s32(cat)]) or 0,
        "note__17ccEnemyWeaponCtrlFP10ccEntryObjP9ccAnmNote":
            lambda mm, this, obj, note, *_: H.wcalls.append(
                ["weaponNote", g.name(obj), mm.load(note + 4, 4), mm.load(note + 8, 4)]) or 0,
        "ctrl__17ccEnemyWeaponCtrlFP10ccEntryObj":
            lambda mm, this, obj, *_: H.wcalls.append(["weaponCtrl", g.name(obj)]) or 0,
        "ctrl__15ccEnemyDustCtrlFP10ccEntryObji":
            lambda mm, this, obj, flag, *_: H.wcalls.append(["dustCtrl", g.name(obj), s32(flag)]) or 0,
        "Draw__6ccCharFv": char_draw,
        "GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult": lambda mm, c, name, *_: (
            H.wcalls.append(["footDust", (name - FOOT_OBJ_TBL) // 30])
            if FOOT_OBJ_TBL <= name < FOOT_OBJ_TBL + 120 else None, 0)[1],
        "ctrlBreath__13ccEnemyBreathFv": lambda mm, br, *_: H.wcalls.append(
            ["breathCtrl", g.name(breath_owner(mm, br)[0]), breath_owner(mm, br)[1]]) or 0,
        "setBreath__13ccEnemyBreathFP14ccEnemyBrParamP10ccEntryObj": lambda mm, br, prm, obj, *_: H.wcalls.append(
            ["breathSet", g.name(obj), breath_owner(mm, br)[1], prm]) or 0,
        "ccEnemyEffDust__FPfifii": lambda mm, p, kind, a2, a3: H.wcalls.append(
            ["dust", vec(p), s32(kind), mm.f[12] & 0xFFFFFFFF, s32(a2), s32(s16(a3))]) or 0,
        "effAfterDrain__FP6ccChari": lambda mm, o, k, *_: H.outs.append(["after_drain", H.index_of(o), s32(k)]) or 0,
        "ccExpDistributor__Fs": lambda mm, a, *_: H.wcalls.append(["ccExpDistributor", s16(a)]) or 0,
        "effSkillStart__FP6ccCharP12ccSkillParamii": skill_start,
        "_ccSkillRequest__FP6ccCharP6ccCharii":
            lambda mm, a, t, sid, st: H.wcalls.append(["request", g.name(a), g.name(t), s32(sid), s32(st)]) or 0,
        "deleteConditionEffect__FP17ccConditionEffect":
            lambda mm, a, *_: H.wcalls.append(["deleteConditionEffect", a]) or 0,
        # the affects' presentation (the enemy motion harness's stubs)
        "ccHitMarkDisp__FP6ccCharP6ccChar": lambda mm, a, b_, *_: rec("ccHitMarkDisp", a, b_),
        "DamageActuate__8ccPlayerFi": lambda mm, a, b_, *_: rec("DamageActuate", b_),
        "SetPanelBure__10ccMenuCtrlFis": lambda mm, a, b_, c, *_: rec("SetPanelBure", b_, b.s16(c)),
        "ccSkillCheck__FP6ccChar": lambda mm, *_: H.running,
        "ChatMessageDamage__4ccAIFi": lambda mm, a, b_, *_: rec("ChatMessageDamage", a, b_),
        "ChatMessageResurrectPlz__4ccAIFv": lambda mm, a, *_: rec("ChatMessageResurrectPlz", a),
        "AffectMessages__4ccAIFiP6ccChari": lambda mm, a, b_, c, d: rec("AffectMessages", a, b_, c, d),
        "Greeting__4ccAIFP6ccChari": lambda mm, a, b_, c, *_: rec("Greeting", a, b_, c),
        "ccAISysMsgSendP__FisUsUsUsUsiPv":
            lambda mm, a, b_, c, d: rec("ccAISysMsgSendP", a, b.s16(b_), c, d, mm.r[11]),
        "ccAISysMsgDeleteDelay__FUiUsUs": lambda mm, a, b_, c, *_: rec("ccAISysMsgDeleteDelay", a, b_, c),
        "effResistantShield__FP6ccCharii": lambda mm, a, b_, c, *_: rec("effResistantShield", a, b_, b.s32(c)),
    }


def enter_motion(h):
    """The motion hooks in place (the scripted ccEnemy::main and the
    recorded EntryAffect out); returns what to put back."""
    m, saved = h.m, {}
    for n, f in motion_hooks(h).items():
        a = h.sym(n)
        saved.setdefault(a, m.hooks.get(a))
        m.hooks[a] = f
    if volume.NAME != "infection":      # the AI's record of the hit (unnamed; Mutation 0x5c1a60)
        a, rec = volume.callee("Influence__FP6ccChar", 0), h.g.record
        saved.setdefault(a, m.hooks.get(a))
        m.hooks[a] = lambda mm, ai, v, *_: rec("NoteHit", ai, v)
    for n in ("main__7ccEnemyFv", "EntryAffect__6ccCharFP6ccCharssss"):
        a = h.sym(n)
        saved.setdefault(a, m.hooks.get(a))
        m.hooks.pop(a, None)
    h.motion = True
    return saved


def leave_motion(h, saved):
    for a, f in saved.items():
        if f is None:
            h.m.hooks.pop(a, None)
        else:
            h.m.hooks[a] = f
    h.motion = False


def make_pc(h, rnd, kite):
    """A party member as the enemy motion harness makes one."""
    b, data = h.c.b, h.c.data
    pc = TB.rnd_pc(b, data, rnd)
    pc.type = 1 if kite else rnd.choice((2, 4))
    pc.cond[0] = rnd.choice((0,) * 14 + (2,))
    pc.width = rnd.choice((0, fb(rnd.uniform(0, 60))))
    pc.height = 0
    pc.skill_id, pc.skill_status = rnd.choice(((0, 0), (0, 0), (rnd.randrange(0, 300), rnd.randrange(0, 10))))
    pc.anm_flag = rnd.choice((0, 1))
    pc.affect_func = 3 if kite else rnd.choice((2, 2, 3))
    pc.affect_type = rnd.choice((0, 0, 0, 13, 1))
    pc.affect_mask = rnd.choice((0,) * 6 + (1 << rnd.randrange(0, 21),))
    pc.act_num = rnd.choice((0, 1, 5, 7, 9, 10, 14, rnd.randrange(0, 20)))
    pc.spc_flags = rnd.getrandbits(32) & ~(0x80 | 7 << 14)
    pc.fellow_flags = rnd.getrandbits(8)
    pc.arms_effect_sw = rnd.randrange(0, 3)
    pc.attack = rnd.randrange(0, 3)
    pc.cnt = rnd.randrange(0, 100)
    pc.cloak = rnd.choice((0, F_ONE))
    pc.no_death = rnd.random() < 0.1
    return pc


def fl(v):
    return struct.unpack("<f", struct.pack("<I", v & 0xFFFFFFFF))[0]


def check_frame_enemies(checks, rnd):
    """ccThEntryCtrl's frames with the enemies' own ccEnemy::main: Kite and
    the party walk to a magic circle and away, the circle opens, the
    enemies it gives (and those an event placed) notice them, chase and
    attack; between frames some are killed (HP 0) or drained."""
    h = harness(checks)
    g, m = h.g, h.m
    h.mrnd = rnd
    h.mscript = {k: [] for k in ("land", "attr", "camdeg", "camtr", "collide", "line", "shake", "anim")}
    h.collide_p = rnd.choice((0.35, 0.35, 0.8))
    h.running = rnd.choice((0, 0, 1, 2))
    area = rnd.choice((1, 1, 2))
    sc = h.rnd_scene(rnd, ne=0, nc=rnd.randrange(1, 4), ng=rnd.randrange(0, 2), nn=rnd.randrange(0, 2), area=area)
    game = sc["game"]
    game[9] = 0
    num = rnd.randrange(1, 8)
    cells = [-1] * 24
    gold = rnd.random() < 0.4
    for k in range(num):
        cells[k] = (rnd.choice(GOLD) if gold and rnd.random() < 0.7 else
                    rnd.choice(MORE) if rnd.random() < 0.6 else rnd.choice(AREA14))
    sc["reg"] = {"cells": cells, "num": num, "dnum": 0, "range": 3, "exist": []}
    objs = sc["objs"]
    circles = [i for i, o in enumerate(objs) if o["kind"] == 2]
    for w in circles:
        ob = objs[w]["obj"]
        ob[49], ob[50] = rnd.choice(((0, 0),) * 6 + ((1, 25), (2, 15)))
        ob[6] = (ob[6] | 1) & ~4
        ent = ob[14:36]
        ent[10:14] = list(sc["ctrl"])
        ent[16] = rnd.choice((-1, 0, 1, 1))
        ent[19], ent[20] = rnd.choice(((-1, -1), (0, -1), (0, rnd.choice(AREA14)), (0, rnd.choice(AREA14)),
                                       (1, rnd.choice((-1, 0, 1)))))
        ent[21] = rnd.choice((-1, -1, 1, 2, 3))
        ob[14:36] = ent
    # Kite walks up to a circle from beyond its reach, lingers near it (the
    # enemies it gives come for the party) and sometimes walks away; the
    # party keeps near him
    target = objs[circles[0]]["char"][1:3]
    tx, ty = fl(target[0]), fl(target[1])
    d0 = rnd.uniform(3100, 5000)
    a0 = rnd.uniform(-math.pi, math.pi)
    x, y = tx + d0 * math.cos(a0), ty + d0 * math.sin(a0)
    v = rnd.uniform(8, 30)
    stop = rnd.uniform(200, 900)
    n = rnd.randrange(100, 201)
    leave = n - rnd.randrange(20, 60) if rnd.random() < 0.4 else n + 1
    heading = a0 + math.pi
    path = []
    for f in range(n + 1):
        path.append([fb(x), fb(y)])
        dx, dy = tx - x, ty - y
        dist = math.hypot(dx, dy)
        if f >= leave:
            heading = math.atan2(-dy, -dx)
            step = 2.5 * v
        elif dist > stop:
            heading = math.atan2(dy, dx)
            step = v
        else:
            heading += rnd.uniform(-0.3, 0.3)
            step = v / 3
        x += step * math.cos(heading)
        y += step * math.sin(heading)
    sc["k"] = path[0]
    kz = rf(rnd, -100, 300)
    kite = make_pc(h, rnd, True)
    kite.pos = [path[0][0], path[0][1], kz, F_ONE]
    kite.pos_p = [0, 0, kz, F_ONE]
    objs[0] = {"kind": 5, "pc": kite, "link": [-1, -1]}
    party = [0]
    offs = {0: (0.0, 0.0, 0.0)}
    for _ in range(rnd.choice((0, 1, 1, 2))):
        pc = make_pc(h, rnd, False)
        objs.append({"kind": 5, "pc": pc, "link": [-1, -1]})
        i = len(objs) - 1
        party.append(i)
        offs[i] = (rnd.uniform(100, 600), rnd.uniform(-math.pi, math.pi), rnd.uniform(-0.05, 0.05))
    pcs = party if rnd.random() < 0.95 else party[1:]
    sc["cmnd"] = [pcs, [], sc["cmnd"][2]]

    pz = {i: rf(rnd, -100, 300) for i in party[1:]}

    def pc_pos(i, f):
        if i == 0:
            return [path[f][0], path[f][1], kz, F_ONE]
        r, a, spin = offs[i]
        x = fl(path[f][0]) + r * math.cos(a + spin * f)
        y = fl(path[f][1]) + r * math.sin(a + spin * f)
        return [fb(x), fb(y), pz[i], F_ONE]

    for i in party[1:]:
        pc = objs[i]["pc"]
        pc.pos = pc_pos(i, 0)
        pc.pos_p = [eemu.f_sub(pc.pos[0], path[0][0]), eemu.f_sub(pc.pos[1], path[0][1]), pc.pos[2], F_ONE]
    # enemies an event placed, made before the first frame
    entries = []
    for _ in range(rnd.choice((0, 0, 1, 2))):
        pos = [eemu.f_add(path[0][0], rf(rnd, -2500, 2500)), eemu.f_add(path[0][1], rf(rnd, -2500, 2500)),
               rf(rnd, -100, 300), F_ONE]
        ent = h.rnd_ent(rnd, sc, 0, rnd.choice(AREA14), pos)
        ent[10:14] = list(sc["ctrl"])
        ent[16] = rnd.choice((-1, 0, 1))
        ent[17] = rnd.choice((-1, -1, 0))
        ent[18:22] = [-1, -1, rnd.choice((-1, 0)), -1]
        entries.append(ent)
    sc["gmain"] = [int(rnd.random() < 0.02) for _ in range(400)]
    sc["nmain"] = [int(rnd.random() < 0.02) for _ in range(400)]
    sc["emain"], sc["fwd"] = [], []
    for k in ("land", "attr", "camdeg", "camtr"):
        sc[k] = []
    env = TB.rnd_env(h.c.b, rnd)
    env.area = game[0]
    puppet = int(rnd.random() < 0.05)
    ride = int(rnd.random() < 0.05)
    book = [rnd.randrange(-100, 100) for _ in range(3)]
    state = rnd.getrandbits(64)
    ids = ([objs[i]["pc"].id for i in party] + [-1, -1, -1])[:3]
    saved = enter_motion(h)
    try:
        h.put_scene(sc)
        m.mem[SAVE + 0x6870:SAVE + KILL_AREA + 8 * 313] = bytes(KILL_AREA + 8 * 313 - 0x6870)
        g.put_env(env)
        g.set_rand(state)
        m.store(m.load(g.sym("eventMng"), 4) + 0x78C, 4, puppet)
        m.store(g.sym("pgRideFlag"), 4, ride)
        m.store(g.sym("ccPartyManager"), 4, h.addrs[0])
        m.store(g.sym("ccMenu"), 4, TB.MENU)
        g.party = {x: k for k, x in enumerate(ids) if x >= 0 and x not in ids[:k]}
        h.put(WMAN + 0x14C, "<3i", *book)
        frames = []

        def frame_out():
            o = h.read_scene(full=False)
            o["rand"] = g.rand_now()
            o["kill"] = h.kill_rows()
            return o

        h.wcalls, h.scalls, h.outs = [], [], []
        g.calls = h.wcalls
        h.note_at = NOTES
        for ent in entries:
            h.put_ent(EP, ent)
            h.call("entryObject__11ccEntryCtrlFP12ccEntryParam", (CTRL, EP))
        frames.append(frame_out())
        steps = []
        kill_at = {}
        for f in range(1, n + 1):
            h.k = list(path[f])
            moves = []
            for i in party:
                va = h.addrs[i]
                pos = pc_pos(i, f)
                pos_p = [eemu.f_sub(pos[0], path[f][0]), eemu.f_sub(pos[1], path[f][1]), pos[2], F_ONE]
                dead = m.load(va + 8, 2, True)
                if rnd.random() < 0.003:
                    dead = 2 if dead == 0 else 0
                h.put(va + 0x40, "<4I", *pos)
                h.put(va + 0x50, "<4I", *pos_p)
                h.put(va + 8, "<h", dead)
                moves.append(f"{i} {' '.join(map(str, pos + pos_p))} {dead}")
            events = []
            for i, k in enumerate(h.kinds):
                if k != 1 or i in h.deleted:
                    continue
                if i not in kill_at:
                    # some die soon after they appear, so that their corpses
                    # are taken away within the case
                    kill_at[i] = f + rnd.randrange(1, 20) if rnd.random() < 0.4 else -1
                if kill_at[i] != f and rnd.random() > 0.02:
                    continue
                va = h.addrs[i]
                eid = s32(m.load(va + 0x244, 4))
                kind = 0 if kill_at[i] == f else rnd.choice((0, 1, 1, 3) + ((2, 2, 2, 1) if eid in GOLD else ()))
                if kind == 3 and eid in DRAINED:
                    kind = 0
                a = b_ = 0
                if kind == 0:
                    a = 0 if kill_at[i] == f else rnd.choice((0, 0, 0, 1, 5))
                    h.put(va + 0x70, "<h", a)
                elif kind == 1:
                    a = rnd.choice((1, 1, 1, 3, 20, 4, 7))
                    b_ = rnd.choice((0, 1, 5, -1, rnd.randrange(1, 300)))
                    m.store(va + 0xE0, 1, m.load(va + 0xE0, 1) | 0x08)
                    h.put(va + 0x9C, "<hh", a, b_)
                elif kind == 2:
                    # a gold goblin held, asleep, paralysed, confused or
                    # charmed, or let go (its escapes and wearing off)
                    a = rnd.choice((1, 1, 1, 9, 14, 10, 11))
                    b_ = rnd.choice((0, 0, rnd.randrange(1, 120)))
                    h.put(va + 8 + 2 * a, "<h", b_)
                else:
                    m.store(va + 0x250, 1, m.load(va + 0x250, 1) | 0x04)
                events.append(f"{i} {kind} {a} {b_}")
            steps.append(f"{path[f][0]} {path[f][1]} {len(moves)} " + " ".join(moves)
                         + f" {len(events)}" + "".join(" " + x for x in events))
            h.wcalls, h.scalls, h.outs = [], [], []
            g.calls = h.wcalls
            h.note_at = NOTES
            h.breaths = 0
            try:
                h.call(TRAMP)
            except FrameDone:
                pass
            frames.append(frame_out())
        h.wcalls, h.scalls, h.outs = [], [], []
        g.calls = h.wcalls
        last = h.read_scene(full=True)
    finally:
        leave_motion(h, saved)
        g.party = {}
        h.restore()
    for k in ("land", "attr", "camdeg", "camtr"):
        sc[k] = list(h.mscript[k])
    sc_ = h.mscript
    scripts = [len(sc_["collide"])]
    for r, off, at in sc_["collide"]:
        scripts += [r] + off + [at]
    scripts += [len(sc_["line"])] + sc_["line"] + [len(sc_["shake"])] + sc_["shake"] + [len(sc_["anim"])]
    for ret, nxt, notes in sc_["anim"]:
        scripts += [ret, nxt, len(notes)]
        for ev, p in notes:
            scripts += [ev, p]
    args = (f"{rs.ser_env(env)} {h.running} 1 {' '.join(map(str, ids))} {puppet} {ride} 0 "
            f"{' '.join(map(str, book))} {state} {len(entries)}" + "".join(" " + ent_str(e) for e in entries)
            + f" {len(steps)} " + " ".join(steps) + " " + " ".join(map(str, scripts)))
    return request(h, sc, "fenemies", args), {"frames": frames, "last": last}, "frame_enemies"



# the boxes and the idols ---------------------------------------------------------------

GIM_IDS = (0, 0, 1, 1, 1, 1, 2, 3, 4, 5, 5, 6, 6, 9, 11, 13, 38, 41, 44, 22, 22, 25, 30, 37)


def scene_from_memory(h, sc):
    """The scene the machine holds now, in rnd_scene's form (the objects
    the constructors made included), for a request that starts from it."""
    m = h.m
    out = dict(sc)
    objs = []
    for i, va in enumerate(h.addrs):
        kind = 0 if i in h.deleted else h.kinds[i]
        pre, nxt = m.load(va + 0x1C0, 4), m.load(va + 0x1C4, 4)
        link = [h.index_of(pre) if pre else -1, h.index_of(nxt) if nxt else -1]
        if i == 0:
            o = dict(sc["objs"][0])
            o["link"] = link
            objs.append(o)
            continue
        assert kind == 3, "only boxes and idols are made here"
        common = h.read_common(va)
        ch = [common[48]] + h.read_char(va)[2:35]
        person = m.load(va + 0x98, 4)
        objs.append({"kind": 3, "char": ch, "link": link, "obj": common, "cls": h.read_class(va),
                     "aff": [s16(m.load(va + 0x9C, 2)), h.index_of(person) if person else -1]})
    out["objs"] = objs
    lists = []
    for k in range(4):
        num, head, foot = h.get(CTRL + 0x10 + 12 * k, "<iII")
        lists.append([num, h.index_of(head) if head else -1, h.index_of(foot) if foot else -1])
    out["lists"] = lists
    out["cmnd"] = [h.walk_cmnd(r) for r in h.gb_cmnd]
    out["rng"] = [sc["rng"][0], s32(m.load(MTI, 4)), m.load(h.addr["lastRnd"], 2)]
    return out


def radiator_words(o):
    """Whether common words o (read_common's) are a radiator's: row 19
    with param[2] 0."""
    return o[48] == 19 and o[34] == 0


def rnd_hand(rnd, k, z):
    """A right hand near Kite at (k, z): [manual] + an lwMatrix (a turn of
    random Euler angles, its rows as words, the place last)."""
    a, b, c = (rnd.uniform(-math.pi, math.pi) for _ in range(3))
    ca, sa, cb, sb, cc, sc_ = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rows = [[cb * cc, cb * sc_, -sb], [sa * sb * cc - ca * sc_, sa * sb * sc_ + ca * cc, sa * cb],
            [ca * sb * cc + sa * sc_, ca * sb * sc_ - sa * cc, ca * cb]]
    words = [w for r in rows for w in [fb(x) for x in r] + [0]]
    words += [eemu.f_add(k[0], rf(rnd, -60, 60)), eemu.f_add(k[1], rf(rnd, -60, 60)),
              eemu.f_add(z, rf(rnd, 40, 140)), F_ONE]
    return [1] + words


def check_radiator_entry(checks, rnd):
    """The event's radiator (ccEvent::Execute case 45): entryObject(ep, 1)
    of row 19 with param[2] 0 and param[3] Kite, whose right hand stands
    somewhere near him: ccGimEtc's constructor, the ccGimRadiator and
    setGimRadPos."""
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=rnd.randrange(0, 2), nn=0, area=rnd.choice((0, 1, 2)))
    sc["hand"] = rnd_hand(rnd, sc["k"], sc["objs"][0]["char"][3])
    sc["hand"][0] = rnd.choice((0, 1))
    ent = h.rnd_ent(rnd, sc, 1, 19)
    ent[16] = 0
    ent[20] = 0
    h.put_scene(sc)
    ent[21] = h.addrs[0]
    h.put_ent(EP, ent)
    ret = h.call("entryObject__11ccEntryCtrlFP12ccEntryParami", (CTRL, EP, 1))
    out = h.read_scene()
    out["ret"] = h.index_of(ret) if ret else -1
    out["ep"] = h.read_ent(EP)
    out["ep"][21] = 1
    h.restore()
    ent[21] = 1
    return request(h, sc, "entryn", ent_str(ent) + " 1"), out, "entryn"


def check_gim_frame(checks, rnd):
    """Boxes and idols made by their own constructors (entryObject), some
    of them opened (affect 11), disarmed (12) or hit (1) by Kite, then
    ccThEntryCtrl's frames with their own mains (ccGimBox::main,
    ccGimIdol::main) against entry::EnemySeam."""
    h = harness(checks)
    area = rnd.choice((1, 2))
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=area)
    sc["rng"][1] = rnd.randrange(0, 300)
    sc["cmnd"] = [[0], [], []]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc["game"][9] = 0
    h.put_scene(sc)
    c = sc["ctrl"]
    k = sc["k"]
    for _ in range(rnd.randrange(1, 4)):
        gid = rnd.choice(GIM_IDS)
        pos = [eemu.f_add(k[0], rf(rnd, -4000, 4000)), eemu.f_add(k[1], rf(rnd, -4000, 4000)), rf(rnd, -100, 500),
               F_ONE]
        ent = h.rnd_ent(rnd, sc, 1, gid, pos=pos)
        ent[10:14] = c
        ent[16] = rnd.choice((0, 0, -1, 1))
        ent[18] = rnd.choice((-1, rnd.randrange(0, 6 * 32)))
        ent[19] = rnd.choice((-1, 0x000A0003, 0x000F0010))
        ent[20] = rnd.choice((-1, -1, 0, 1, 2, 3))
        h.put_ent(EP, ent)
        h.call("entryObject__11ccEntryCtrlFP12ccEntryParam", (CTRL, EP))
    sc2 = scene_from_memory(h, sc)
    h.restore()
    for o in sc2["objs"][1:]:
        r = rnd.random()
        if r < 0.6:
            o["obj"][6] |= 8
            o["aff"] = [rnd.choice((11, 11, 11, 11, 12, 12, 1, 0)), 0]
        elif r < 0.75:
            o["obj"][30] = 1
            o["char"][33] = 1
            o["obj"][49], o["obj"][50] = 0, rnd.choice((895, 899, 900, 901, 902))
    in_battle = rnd.choice((0, 0, 1))
    n = rnd.randrange(30, 130)
    path = [[eemu.f_add(k[0], rf(rnd, -300, 300)), eemu.f_add(k[1], rf(rnd, -300, 300))] for _ in range(n)]
    sc2["k"] = path[0]
    sc2["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc2["camdeg"] = [int(rnd.random() < 0.8) for _ in range(600)]
    sc2["camtr"] = [rnd.choice((F_ONE, rf(rnd, 0, 1), rf(rnd, -0.2, 1.3))) for _ in range(600)]
    sc2["fwd"] = [int(rnd.random() < 0.08) for _ in range(600)]
    sc2["gmain"] = []
    h.put_scene(sc2)
    h.put(TB.GAME + 0x58, "<i", in_battle)
    frames = []
    for f in range(n):
        h.k = list(path[f])
        h.put(h.addrs[0] + 0x40, "<II", *path[f])
        h.wcalls, h.scalls, h.outs = [], [], []
        h.breaths = 0
        try:
            h.call(TRAMP)
        except FrameDone:
            pass
        frames.append(h.read_scene(full=True))
    h.restore()
    env_s = f"1 0 {in_battle} 0 -1 0 {area}"
    req = request(h, sc2, "gframe", f"{env_s} 1 {n} " + " ".join(f"{p[0]} {p[1]}" for p in path))
    return req, {"frames": frames}, "gim_frame"


ETC_IDS = (20, 20, 20, 20, 19, 19, 21)
# ctrlFountain's counts where something happens.
ETC_COUNTS = (0, 1, 2, 9, 10, 11, 29, 30, 31, 59, 60, 79, 80, 89, 90)


def check_etc_frame(checks, rnd):
    """The spring and the boss room's warning (ccGimEtc, rows 19-21) made by
    their own constructor (entryObject; the spring's initFountain), then
    put into a random state of ctrlFountain (actNum 0-11 and a count, the
    spirit on and the lights set as state 2 leaves them, the bounce and a
    spring running), some given affect 11 (FountainMenu3's EntryAffect)
    or another, and ccThEntryCtrl's frames with ccGimEtc::main. Some
    scenes hold the event's radiator (row 19, param[2] 0, param[3] Kite):
    its rays follow his right hand, moved each frame, until his AI leaves
    manual mode (ctrlGimRadiator, ccGimRadiator::ctrl)."""
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=2)
    sc["rng"][1] = rnd.randrange(0, 300)
    sc["cmnd"] = [[0], [], []]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc["game"][9] = 0
    # A spring whose area is among the fountains used is not made
    # (CheckFountain): the list without this area's code, so that the
    # spring's SetFountain adds it.
    code = (sc["game"][8] | (sc["game"][6] << 29)) & 0xFFFFFFFF
    sc["save"][6:106] = [x if (x & 0xFFFFFFFF) != code else 0 for x in sc["save"][6:106]]
    radiator = rnd.random() < 0.4
    kz = sc["objs"][0]["char"][3]
    if radiator:
        sc["hand"] = rnd_hand(rnd, sc["k"], kz)
    h.put_scene(sc)
    c = sc["ctrl"]
    k = sc["k"]
    for j in range(rnd.randrange(1, 3)):
        gid = 19 if radiator and j == 0 else rnd.choice(ETC_IDS)
        pos = [eemu.f_add(k[0], rf(rnd, -3000, 3000)), eemu.f_add(k[1], rf(rnd, -3000, 3000)), rf(rnd, -100, 500),
               F_ONE]
        ent = h.rnd_ent(rnd, sc, 1, gid, pos=pos)
        ent[10:14] = c
        ent[16] = rnd.choice((0, 0, -1, 1))
        # param[2] 0: row 19's rays, their user param[3] (Kite).
        ent[20] = 0 if radiator and j == 0 else rnd.choice((-1, -1, 3))
        if ent[20] == 0:
            ent[21] = h.addrs[0]
        h.put_ent(EP, ent)
        h.call("entryObject__11ccEntryCtrlFP12ccEntryParam", (CTRL, EP))
    sc2 = scene_from_memory(h, sc)
    h.restore()
    for o in sc2["objs"][1:]:
        cls = o["cls"]
        if o["obj"][48] == 20 and cls[0] == 5 and rnd.random() < 0.8:
            st = rnd.randrange(0, 12)
            o["obj"][49] = st
            o["obj"][50] = rnd.choice(ETC_COUNTS + (rnd.randrange(0, 95),))
            if st >= 2:
                cls[2] |= 1
                rgb = 0x008080FF if cls[2] & 2 else 0x00FF8080
                for j in range(2):
                    b = 31 + 8 * j
                    cls[b:b + 5] = [rgb, 0, 0, fb(1500.0), fb(2250000.0)]
                cls[30] = rnd.choice((0, 1))
                cls[11] = rf(rnd, 0, 6)
                o["char"][3] = rf(rnd, -400, 380)
            if st >= 3:
                cls[17] = 1
            if rnd.random() < 0.5:
                cls[27] = rnd.choice((0, fb(0.005), rf(rnd, 0, 7)))
                cls[28] = rf(rnd, 0, 0.1)
                cls[29] = rf(rnd, 0, 0.3)
            if st >= 10 and rnd.random() < 0.5:
                cls[1] = 0
        if rnd.random() < 0.5:
            o["obj"][6] |= 8
            o["aff"] = [rnd.choice((11, 11, 11, 12, 1)), 0]
    n = rnd.randrange(30, 130)
    path = [[eemu.f_add(k[0], rf(rnd, -300, 300)), eemu.f_add(k[1], rf(rnd, -300, 300))] for _ in range(n)]
    sc2["k"] = path[0]
    sc2["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc2["camdeg"] = [int(rnd.random() < 0.8) for _ in range(600)]
    sc2["camtr"] = [F_ONE for _ in range(600)]
    sc2["fwd"] = [int(rnd.random() < 0.08) for _ in range(600)]
    sc2["gmain"] = []
    h.put_scene(sc2)
    # The hand each frame, and the frame the event lets Kite go.
    off_at = rnd.randrange(0, n + 20)
    hands = [rnd_hand(rnd, p, kz) for p in path] if radiator else []
    for f, hd in enumerate(hands):
        hd[0] = int(f < off_at)
    frames = []
    for f in range(n):
        h.k = list(path[f])
        h.put(h.addrs[0] + 0x40, "<II", *path[f])
        if radiator:
            h.put_hand(hands[f])
        h.wcalls, h.scalls, h.outs = [], [], []
        h.breaths = 0
        try:
            h.call(TRAMP)
        except FrameDone:
            pass
        frames.append(h.read_scene(full=True))
    h.restore()
    if radiator:
        req = request(h, sc2, "gframeh", "1 0 0 0 -1 0 2 1 " + str(n) + " "
                      + " ".join(" ".join(map(str, p + hd)) for p, hd in zip(path, hands)))
    else:
        req = request(h, sc2, "gframe", "1 0 0 0 -1 0 2 1 " + str(n) + " "
                      + " ".join(f"{p[0]} {p[1]}" for p in path))
    return req, {"frames": frames}, "gim_frame"


def put_dungeon(h, d):
    """DUNGEON, its gimPos slots, its edit data and WORLD_MAN's timeSym as
    SetItemBox and SetIDOL read them."""
    m = h.m
    m.mem[DUNG:DUNG + 0x33520] = bytes(0x33520)
    m.store(DUNG + 0x10, 4, d["dtype"])
    m.store(DUNG + 0x14, 4, d["story"])
    m.store(DUNG + 0x3C, 4, d["counter"])
    m.store(DUNG + 0x44, 4, d["lake"])
    m.store(DUNG + 0x424, 4, GIMPOS)
    for k in range(750):
        a = GIMPOS + 0x30 * k
        s_ = d["slots"][k] if k < len(d["slots"]) else [0] * 10 + [-1]
        h.put(a, "<8I", *s_[:8])
        h.put(a + 0x20, "<bbb", *s_[8:11])
    # From Mutation on DUNGEON allocates its floors: their number at +0x430
    # (75 gimPos slots each), rotate through a pointer at +0x7cc and edit
    # at +0x934.
    later = volume.NAME != "infection"
    rotate = ROTATE if later else DUNG + 0x2F488
    if later:
        m.store(DUNG + 0x430, 4, 10)
        m.store(DUNG + 0x7CC, 4, ROTATE)
    for f in range(10):
        h.put(rotate + 60 * f, "<15I", *d["rotate"][f])
    m.store(WMAN + 0x134, 4, d["time_sym"])
    m.store(WMAN + 0x164, 4, 0)
    if d["edit"] is not None:
        m.store(DUNG + (0x934 if later else 0x3351C), 4, EDIT)
        rows = EDIT_ROWS + 0x400
        m.store(EDIT + 8, 4, rows)
        m.store(EDIT + 0xC, 4, len(d["rooms"]))
        m.store(EDIT + 0x10, 4, EDIT_ROWS)
        m.store(EDIT + 0x14, 4, len(d["edit"]))
        for k, r in enumerate(d["edit"]):
            h.put(EDIT_ROWS + 0x20 * k, "<8i", *r)
        for k, r in enumerate(d["rooms"]):
            a = rows + 0x4C * k
            m.mem[a:a + 0x4C] = bytes(0x4C)
            h.put(a, "<ii", r[0], r[1])
            h.put(a + 0x10, "<i", r[2])
            h.put(a + 0x20, "<i", r[3])
            h.put(a + 0x48, "<i", r[4])


def ser_dungeon(d):
    v = [d["dtype"], d["story"], d["counter"], d["lake"], d["time_sym"], d["attr"], d["food"], len(d["slots"])]
    for s_ in d["slots"]:
        v += s_
    if d["edit"] is None:
        v.append(-1)
    else:
        v.append(len(d["edit"]))
        for r in d["edit"]:
            v += r
    v.append(len(d["rooms"]))
    for r in d["rooms"]:
        v += r
    for f in range(10):
        v += d["rotate"][f]
    return " ".join(str(x) for x in v)


def rnd_dungeon(h, rnd, sc, kinds):
    story = int(rnd.random() < 0.6)
    d = {"dtype": rnd.choice((0, 1, 3, 8, 9)), "story": story, "counter": rnd.randrange(0, 20),
         "lake": rnd.choice((0, 0, 1)), "time_sym": rnd.choice((0, 0, 1)), "attr": rnd.randrange(0, 7),
         "food": rnd.randrange(22, 38)}
    slots = []
    for _ in range(rnd.randrange(0, 12)):
        slots.append([rf(rnd, 0, 60000), rf(rnd, 0, 60000), rf(rnd, -100, 100), F_ONE,
                      0, 0, rf(rnd, -math.pi, math.pi), F_ONE,
                      rnd.randrange(0, 3), rnd.randrange(0, 12), rnd.choice(kinds)])
    d["slots"] = slots
    d["rotate"] = [[rnd.choice((0, fb(math.pi), fb(math.pi / 2), fb(-math.pi / 2), rf(rnd, -3.2, 3.2)))
                    for _ in range(15)] for _ in range(10)]
    d["edit"] = None
    d["rooms"] = []
    if story and rnd.random() < 0.9:
        d["rooms"] = [[rnd.randrange(0, 3), rnd.randrange(0, 12), rnd.choice((0, 0, 12, 13, 14, 16, 20, 34)),
                       rnd.choice((0, 0, 1, 5)), rnd.choice((0, 0, 3211265, 5373957, 0x4F0012))]
                      for _ in range(rnd.randrange(0, 10))]
        edit = []
        for k in range(rnd.randrange(0, 12)):
            ty = rnd.choice((0, 0, 1, 2, 3, 3)) if k else rnd.choice((0, 1, 2))
            kind = rnd.choice((0, 1, 2, 3, 4, 5, 7, 12, 27, 30)) if ty == 3 else rnd.randrange(0, 3)
            flag = rnd.choice((0, 5373957, 4456504, 5373953, 0x45000A)) if ty == 0 else 0
            edit.append([rnd.randrange(0, 3), rnd.randrange(0, 12), rnd.randrange(0, 60000), rnd.randrange(0, 60000),
                         ty, kind, flag, rnd.randrange(0, 5)])
        d["edit"] = edit
    return d


def run_setter(h, rnd, sc, d, fn, label, op):
    m = h.m
    seed = rnd.getrandbits(32) % 0xFFFFFFFE
    h.put_scene(sc)
    put_dungeon(h, d)
    m.store(h.addr["seed"], 4, seed)
    m.store(h.addr["randcnt"], 4, 0)
    saved = {}
    stubs = {"SetRoom__7DUNGEONFii": lambda mm, *_: 0, "DeleteRoom__7DUNGEONFii": lambda mm, *_: 0,
             "GetFood__9WORLD_MANFv": lambda mm, *_: d["food"],
             "GetFieldAttrb__9WORLD_MANFv": lambda mm, *_: d["attr"]}
    for n_, f in stubs.items():
        a = h.sym(n_)
        saved[a] = m.hooks.get(a)
        m.hooks[a] = f
    try:
        h.call(fn, (DUNG,))
    finally:
        for a, x in saved.items():
            if x is None:
                del m.hooks[a]
            else:
                m.hooks[a] = x
    out = h.read_scene()
    out["counter"] = s32(m.load(DUNG + 0x3C, 4))
    out["kinds"] = [s16(m.load(GIMPOS + 0x30 * k + 0x22, 1) << 8) >> 8 for k in range(len(d["slots"]))]
    out["seed"] = m.load(h.addr["seed"], 4)
    h.restore()
    return request(h, sc, op, f"{ser_dungeon(d)} {seed}"), out, label


def check_item_box(checks, rnd):
    """DUNGEON::SetItemBox (0x005beb30): the slots' boxes and food, and a
    story dungeon's GIMMICKDATA boxes and traps (no boss room's object:
    kind 6 rows are left out)."""
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=2)
    sc["ctrl"] = [2, sc["game"][3], rnd.randrange(0, 3), rnd.randrange(0, 12)]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(60)]
    sc["rng"][1] = rnd.randrange(0, 200)
    d = rnd_dungeon(h, rnd, sc, (0, 0, 0, 1, 2, -1))
    return run_setter(h, rnd, sc, d, "SetItemBox__7DUNGEONFv", "item_box", "itembox")


def check_idol(checks, rnd):
    """DUNGEON::SetIDOL (0x005be750): an idol (or the spring, or the Zeit
    statue) on every statue slot, with a story room's item."""
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=2)
    sc["ctrl"] = [2, sc["game"][3], rnd.randrange(0, 3), rnd.randrange(0, 12)]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(60)]
    d = rnd_dungeon(h, rnd, sc, (2, 2, 0, 1))
    return run_setter(h, rnd, sc, d, "SetIDOL__7DUNGEONFv", "idol", "idol")


def check_field_gims(checks, rnd):
    """WORLD::SetFood (0x005aa990) then WORLD::SetSpecialObj (0x005aae90)
    on a WORLD of random objects: fobj2[0 .. KeyObjNum] and chips' fobj,
    each with or without an anm, with 0-5 OBJ_xgfood0a* and 0-3
    OBJ_xgsymb* nodes (GetSubstAdrs and GetObjAdrsF answer them; the
    nodes' coords carry their places, _SetLWMatrix leaves them), a lake
    or none (water), the in-points and the area (NO_ENTRANCE's too)."""
    h = harness(checks)
    sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0, area=1)
    sc["game"][2] = rnd.choice((0, 0, 28, 41, 57, 80, 111, rnd.randrange(1, 120)))
    sc["ctrl"] = [1, sc["game"][2], sc["game"][4], sc["game"][5]]
    sc["land"] = [rf(rnd, -100, 600) for _ in range(40)]
    sc["attr"] = [inf_va(0x00304050)] * 40
    seed = rnd.getrandbits(32) % 0xFFFFFFFE
    ftype = rnd.randrange(0, 12)

    def obj():
        o = {"wp": [rf(rnd, 0, 48000), rf(rnd, 0, 48000), rf(rnd, 0, 300), F_ONE],
             "anm": int(rnd.random() < 0.6),
             "food": [], "symb": []}
        nf = rnd.choice((0, 0, 0, 1, 2, 3, 4, 5))
        ns = rnd.choice((0, 0, 1, 2, 3))
        o["food"] = [[rf(rnd, -500, 500), rf(rnd, -500, 500), rf(rnd, 0, 200), F_ONE] for _ in range(nf)]
        o["symb"] = [[rf(rnd, -500, 500), rf(rnd, -500, 500), rf(rnd, 0, 200), F_ONE] for _ in range(ns)]
        return o

    fobj2 = [obj() for _ in range(rnd.randrange(1, 8))]
    water = rnd.choice((0, rnd.randrange(0, len(fobj2))))
    grid = {}
    for _ in range(rnd.randrange(0, 25)):
        grid[(rnd.randrange(40), rnd.randrange(40))] = obj()
    # the setters' order: fobj[a][b] at a * 0xa0 + b * 4, b the outer loop
    fobj = [grid[(a, b)] for b in range(40) for a in range(40) if (a, b) in grid]
    inp = [[rf(rnd, 0, 48000), rf(rnd, 0, 48000), rf(rnd, 0, 300), F_ONE] for _ in range(2)]
    h.put_scene(sc)
    m, sym = h.m, h.sym
    m.mem[WORLDO:WORLDO + 0x61E0] = bytes(0x61E0)
    m.store(WORLDO + 0x18, 4, len(fobj2))
    m.store(WORLDO + 0x30, 4, water)
    m.store(WMAN + 8, 4, 1)
    m.store(WMAN + 0x10, 4, ftype)
    m.store(WMAN + 0x120, 4, sc["game"][2])
    h.put(WMAN + 0xD0, "<4I", *inp[0])
    h.put(WMAN + 0xE0, "<4I", *inp[1])
    lists = {}
    coord = [WORLDO + 0x9000]
    tbl = WORLDO + 0x19000

    def nodes(ptr, key, pts):
        cs = []
        for p in pts:
            c = coord[0]
            coord[0] += 0x90
            m.mem[c:c + 0x90] = bytes(0x90)
            m.store(c + 0x80, 4, 1)
            h.put(c + 0x30, "<4I", *p)
            cs.append(c)
        lists[(ptr, key)] = cs

    def place(o, base, wp_off, clump_off, anm_off, k):
        ptr = 0x0FA00000 + 0x100 * k
        h.put(base + wp_off, "<4I", *o["wp"])
        m.store(base + (anm_off if o["anm"] else clump_off), 4, ptr)
        nodes(ptr, "OBJ_xgfood0a*", o["food"])
        nodes(ptr, "OBJ_xgsymb*", o["symb"])

    for k, o in enumerate(fobj2):
        base = WORLDO + 0x6400 + 0x80 * k
        m.mem[base:base + 0x80] = bytes(0x80)
        place(o, base, 0x40, 0x70, 0x78, k)
        m.store(WORLDO + 0x5090 + 4 * k, 4, base)
    for k, ((a, b), o) in enumerate(sorted(grid.items())):
        base = WORLDO + 0x7000 + 0x50 * k
        m.mem[base:base + 0x50] = bytes(0x50)
        place(o, base, 0x30, 0x48, 0x4C, 100 + k)
        m.store(WORLDO + 0x1E90 + a * 0xA0 + b * 4, 4, base)

    def cstr(a):
        out = bytearray()
        while m.mem[a]:
            out.append(m.mem[a])
            a += 1
        return out.decode("latin-1")

    def answer(ptr, pat, res):
        cs = lists.get((ptr, cstr(pat)), [])
        for i, c in enumerate(cs):
            m.store(tbl + 8 * i, 4, c)
        m.store(res, 4, tbl)
        m.store(res + 4, 2, len(cs))
        return 0

    hooks = {"GetSubstAdrs__5ccAnmFPCcP19ccSubstSearchResult": lambda mm, a, pat, res, *_: answer(a, pat, res),
             "GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult": lambda mm, c, pat, f, res, *_: answer(c, pat, res),
             "_SetLWMatrix__7ccCoordFv": lambda mm, *_: 0,
             "Reset__19ccSubstSearchResultFUs": lambda mm, res, *_: 0,
             "__dla__FPv": lambda mm, *_: 0}
    saved = {}
    for n, f in hooks.items():
        a = sym(n)
        saved[a] = m.hooks.get(a)
        m.hooks[a] = f
    m.store(h.addr["seed"], 4, seed)
    try:
        h.call("SetFood__5WORLDFv", (WORLDO,))
        h.call("SetSpecialObj__5WORLDFv", (WORLDO,))
    finally:
        for a, x in saved.items():
            if x is None:
                del m.hooks[a]
            else:
                m.hooks[a] = x
    out = h.read_scene()
    out["seed"] = m.load(h.addr["seed"], 4)
    h.restore()

    def ser(o):
        v = list(o["wp"]) + [o["anm"], len(o["food"])] + [x for p in o["food"] for x in p]
        return " ".join(map(str, v + [len(o["symb"])] + [x for p in o["symb"] for x in p]))

    args = (f"{seed} {water} {sc['game'][2]} {ftype} " + " ".join(map(str, inp[0] + inp[1]))
            + f" {len(fobj2)} " + " ".join(ser(o) for o in fobj2)
            + f" {len(fobj)} " + " ".join(ser(o) for o in fobj))
    return request(h, sc, "fieldgims", args), out, "field_gims"


SPAWN = {
    "routine": check_routine,
    "check": check_obj_check,
    "entry": check_entry,
    "race": check_race,
    "circle_object": check_circle_object,
    "enemy_object": check_enemy_object,
    "init_object": check_init_object,
    "delete": check_delete,
    "active": check_active,
    "circle_main": check_circle_main,
    "frame": check_frame,
    "world_mc": check_world_mc,
    "dungeon_mc": check_dungeon_mc,
    "event_mc": check_event_mc,
    "entry_gimmick": check_entry_gimmick,
    "leave": check_leave,
    "restore": check_restore,
    "register": check_register,
    "save": check_save,
    "frame_enemies": check_frame_enemies,
    "gim_frame": check_gim_frame,
    "item_box": check_item_box,
    "idol": check_idol,
    "etc_frame": check_etc_frame,
    "radiator_entry": check_radiator_entry,
    "field_gims": check_field_gims,
}


@unittest.skipUnless(rs.READY, "needs the extracted disc and cargo")
class SpawnAgainstGame(rs.Against):
    TABLE = SPAWN

    def test_objects(self):
        self.check("routine", 20000)
        self.check("check", 20001)
        self.check("entry", 20002)
        self.check("race", 20003)
        self.check("circle_object", 20004)
        self.check("enemy_object", 20005)
        self.check("init_object", 20006)
        self.check("delete", 20007)
        self.check("active", 20008)

    def test_circle(self):
        self.check("circle_main", 20009)
        self.check("frame", 20010)

    def test_placement(self):
        self.check("world_mc", 20011)
        self.check("dungeon_mc", 20012)
        self.check("event_mc", 20013)
        self.check("entry_gimmick", 20014)
        self.check("restore", 20015)
        self.check("leave", 20018)
        self.check("field_gims", 20024)

    def test_tables(self):
        self.check("register", 20016)
        self.check("save", 20017)

    def test_enemies(self):
        self.check("frame_enemies", 20019)

    def test_gimmicks(self):
        self.check("gim_frame", 20020)
        self.check("item_box", 20021)
        self.check("idol", 20022)
        self.check("etc_frame", 20023)
        self.check("radiator_entry", 20025)


if __name__ == "__main__":
    rs.main(list(SPAWN), 20000, SPAWN)
