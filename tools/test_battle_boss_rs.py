#!/usr/bin/env python3
"""crates/piney-battle's Skeith (src/boss.rs) against the game's boss.cpp and
boss01.cpp run natively in the Rust VU0 machine (tools/eemu_rs.so).

A case lays a party of one to three members out on the command lists
(tools/test_battle_rs.py's scene), builds Skeith with the game's own
constructor (ccBoss01::ccBoss01, gcmn 0x0047b410) at BOSS, and runs
ccBoss01::Main (0x0047bd50) frame after frame, with scripted affects on the
boss between frames (Kite's hits, the protect gauge, the drain's 13 and
21) and the menu's type changing (the member drain waits on menu 74). The
same case goes to battle_probe's `boss` request, and every frame is
compared: the acts, patterns and waits, the movement and position, the
target, HP and protect gauge, the flags, the animations' clips and frames,
the effects, the party's HP and hold, the calls (sounds, flashes, effects,
the menu's bar and cursor, the stream menu, the camera, the stage), newlib's
rand() and ccRand's index.

What is not the game's here:
- The party's affects are off (affectFunc 0): the boss's hits and holds are
  compared by what EntryAffect stores (the hold), not their damage.
- The boss's files: GetCCSAdrs, GetChunkAdrsF and the clump are stand-ins;
  ccAnm's SetAnm and _AnimateForward follow the clips' lengths
  (battle_probe `bossclips`, read from x11 and xeffect) as
  piney_data::anim::forward_clip does.
- The effects: each ccBossEff*Create makes a stand-in whose m_bEnabled
  goes to 0 after the port's lifetime for its kind. test_effect_lives
  checks those lifetimes against the game's Create and Draw run natively.
- No boss camera, no stage fader (the calls are compared), W2P/P2W are
  identity.

    python3 tools/test_battle_boss_rs.py            the unit tests
    python3 tools/test_battle_boss_rs.py bulk N     N random cases
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle_enemy_ai_rs as eai  # noqa: E402
import test_battle_rs as rs  # noqa: E402
from test_battle_rs import F_ONE, SCN, fbits, ser_char  # noqa: E402

BOSS = 0x01300000               # the ccBoss01 (0x296b0 bytes)
FAKE = 0x01340000               # stand-in streams, chunks and effects
MGR = 0x01380000                # _g_bossEffManager (0x1038 bytes)
STAGE = 0x01390000              # the stage fader
PLW, PARTY = inf_va(0x007302E0), inf_va(0x00730310)
MENU = 0x01020000               # test_battle's ccMenu
BOSS_SIZE = 0x296B0


def eff_life(kind, args):
    """The Draws piney_battle::boss::Eff::draw gives each kind (0 WaveShock,
    1 MagicSquare, 2 ForceGenerator [num, life], 3 AutoSamonRing, 4 IceBreak,
    5 Dead)."""
    if kind == 0:
        return 45
    if kind == 1:
        return 91
    if kind == 2:
        return args[1] + int((args[0] - 1) / 2) * 10 + 3
    if kind == 3:
        return 20
    if kind == 4:
        return 1 + 61 + 61
    return 120


def forward_clip(frames, looping, time, step):
    last = max(frames - 1, 0) << 8
    new = (time + step) & 0xFFFFFFFF
    ended = False
    if last < new:
        step = (step - (new - last)) & 0xFFFFFFFF
        new = last
        ended = True
    if new >> 8 != time >> 8:
        ended = False
        if new >> 8 >= max(frames - 1, 0):
            if looping:
                new = 0
            else:
                ended = True
    return new, ended


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


class Game:
    """The machine with Skeith's hooks."""

    def __init__(self):
        self.c = rs.Checks()
        g = self.c.game
        self.g, self.m, self.b = g, g.m, self.c.b
        self.sym = g.sym
        self.clips = rs.ask(["bossclips"])[0]
        self.fake_at = FAKE
        self.names = {}         # a stand-in chunk or stream -> its name
        self.anms = {}          # a ccAnm -> [clip, time]
        self.effs = {}          # a stand-in effect -> [kind, count, life]
        self.out = []
        self.hooks()

    def alloc(self, n, name=None):
        a = self.fake_at
        self.fake_at += (n + 15) & ~15
        self.m.mem[a:a + n] = bytes(n)
        if name is not None:
            self.names[a] = name
        return a

    def cstr(self, a):
        m = self.m
        out = bytearray()
        while m.mem[a]:
            out.append(m.mem[a])
            a += 1
        return out.decode("latin-1")

    def hooks(self):
        m, sym = self.m, self.sym
        h = m.hooks
        out = self.out

        def hook(name, fn):
            h[sym(name)] = fn

        nop = lambda mm, *a: 0  # noqa: E731
        this = lambda mm, a, *_: a  # noqa: E731
        # EntryAffect and checkPartyAnnihilation run natively (the boss's
        # bossAffectFunc, the party's store and hold); GameBattle's stand-ins go.
        h.pop(sym("EntryAffect__6ccCharFP6ccCharssss"), None)
        h.pop(sym("checkPartyAnnihilation__Fv"), None)
        hook("__construct_array", lambda mm, a, *_: a)
        hook("GetCCSAdrs__8ccStreamFPCc", lambda mm, n, *_: self.alloc(16, self.cstr(n)))
        hook("GetChunkAdrsF__8ccStreamFPCci", lambda mm, s, n, *_: self.alloc(16, self.cstr(n)))
        hook("GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult", lambda mm, *a: self.alloc(0x100))
        hook("__ct__7ccCoordFv", this)
        hook("Init__7ccClumpFP12ccClumpChunk", nop)
        hook("ApplyClump__5ccAnmFP7ccClumpP8ccStream", nop)
        hook("__ct__9ccLatticeFii", this)

        def anm_ct(mm, a, *_):
            mm.mem[a:a + 0x110] = bytes(0x110)
            mm.store(a + 0x9C, 2, 256)
            self.anms[a] = [None, 0]
            return a
        hook("__ct__5ccAnmFv", anm_ct)

        def set_anm(a, name):
            self.anms[a] = [name, 0]
            m.store(a + 0xAC, 4, 1)
            m.store(a + 0x98, 4, 0)

        hook("SetAnm__5ccAnmFP10ccAnmChunkUi", lambda mm, a, ch, *_: set_anm(a, self.names.get(ch)) or 0)
        hook("SetAnm__5ccAnmFP8ccStreamPcUi", lambda mm, a, s, n, *_: set_anm(a, self.cstr(n)) or 0)

        def forward(mm, a, step, *_):
            clip, time = self.anms.get(a, [None, 0])
            frames, looping = self.clips.get(clip, [0, 0])
            new, ended = forward_clip(frames, looping, time, step & 0xFFFF)
            self.anms[a] = [clip, new]
            mm.store(a + 0x98, 4, new >> 8)
            return int(ended)
        hook("_AnimateForward__5ccAnmFUi", forward)
        for name in ("SetMatrix_PosRotZYX__7ccCoordFPfPf", "Draw__5ccAnmFv", "Draw__6ccBossFv",
                     "DrawAfterImages__6ccBossFv", "DrawParts__6ccBossFP5ccAnmi", "DrawStageEffect__6ccBossFv",
                     "InitAfterImage__6ccBossFP5ccAnmi", "NextVertex__9ccLatticeFv", "SetPos__9ccLatticeFPfi",
                     "Disp__9ccLatticeFv", "__dl__FPv", "MakePacket__15ccBufferReverceFv"):
            hook(name, nop)
        hook("InitBossCamera__6ccBossFff", nop)
        hook("__ct__15ccBufferReverceFv", this)
        hook("Init__7ccLayerFsP6ccView", nop)
        hook("SetFrame__6ccViewFffffffff", nop)
        hook("__dt__7ccLayerFv", nop)
        hook("__ct__16ccDrawPacketCtrlFv", this)

        def center(mm, a, *_):
            for k in range(4):
                mm.store(a + 0x1E0 + 4 * k, 4, self.center[k])
                mm.store(a + 0x1F0 + 4 * k, 4, self.center[k])
            return 0
        hook("InitCenterPos__6ccBossFv", center)

        def stage_init(mm, a, *_):
            mm.store(a + 0x29340, 4, STAGE + 0x100)
            mm.store(a + 0x29344, 4, STAGE)
            mm.store(a + 0x29348, 1, 1)
            mm.store(a + 0x2934C, 4, 0)
            mm.store(a + 0x2933C, 1, 1)
            return 0
        hook("InitStageEffect__6ccBossFv", stage_init)
        # the stage fader: one element, on from EntryFlash3 to DeleteFade
        self.stage_on = False

        def flash3(mm, f, t0, t1, t2, *_):
            if f == STAGE:
                self.stage_on = True
            return 0
        hook("EntryFlash3__8ccScFadeFiiiiffff", flash3)
        hook("CheckFade__8ccScFadeFi", lambda mm, f, i, *_: int(self.stage_on and i == 0))

        def delete_fade(mm, f, i, *_):
            if f == STAGE:
                self.stage_on = False
            return 0
        hook("DeleteFade__8ccScFadeFi", delete_fade)

        def begin_stage(mm, a, rgba, t0, t1, *_):
            out.append(["stage_begin", rgba & 0xFFFFFFFF, s32(t0), s32(t1), s32(mm.r[8])])
            self.stage_on = True
            return 0
        hook("BeginStageEffect__6ccBossFiiii", begin_stage)

        def end_stage(mm, a, ref, rgba, t, *_):
            i = s32(mm.load(ref, 4))
            if 0 <= i < 4 and self.stage_on:
                out.append(["stage_end", rgba & 0xFFFFFFFF, s32(t)])
                self.stage_on = False
            mm.store(ref, 4, 0xFFFFFFFF)
            return 0
        hook("EndStageEffect__6ccBossFRiii", end_stage)

        def flash(mm, f, t, c, *_):
            out.append(["flash", s32(t), c & 0xFFFFFFFF])
            return 0
        hook("EntryFlash__8ccScFadeFiiffff", flash)
        hook("ccSeOn3D__FiPf", lambda mm, se, p, *_: out.append(["se3d", s32(se)]) or 0)
        hook("ccSeOn__Fi", lambda mm, se, *_: out.append(["se", s32(se)]) or 0)
        hook("ccSeOn3DNote__FiPfc", lambda mm, se, p, n, *_: out.append(["se3dnote", s32(se), n & 0xFF]) or 0)
        hook("EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T", lambda mm, *a: out.append(["afterimage"]) or 0)
        hook("OnCinemaMode__16ccBossEffManagerFi", lambda mm, a, n, *_: out.append(["cinema", s32(n)]) or 0)
        hook("OffCinemaMode__16ccBossEffManagerFv", lambda mm, *a: out.append(["cinema", -99]) or 0)
        hook("SwitchLayer__11EVENTAREAB0Fv", lambda mm, *a: out.append(["switch_layer"]) or 0)
        hook("ccEntryFlyFontNew__FiiPfP6ccCharff",
             lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccHitMarkDisp__FP6ccCharP6ccChar", lambda mm, *a: out.append(["hitmark"]) or 0)
        hook("ccClearSpcCondition__Fv", lambda mm, *a: out.append(["clear_spc"]) or 0)
        hook("ccSqFade__FiUsiUc", nop)
        for name in ("cameraGetPos__FPfi", "cameraGetView__FPfi", "cameraGetRot__FPfi", "cameraSetPos__FPfi",
                     "cameraSetView__FPfi", "cameraSetRot__FPfi", "changeCamera__Fi"):
            hook(name, nop)
        hook("ccTransPosW2P__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)
        hook("ccTransPosP2W__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)
        hook("SetHitSW__9ccCharHitFi", nop)
        hook("CollisionDetection__9ccCharHitFv", lambda mm, *a: 0)

        def effect(kind):
            def create(mm, *a):
                args = [s32(mm.r[6]), s32(mm.r[7])] if kind == 2 else ([s32(mm.r[7])] if kind == 3 else [])
                if kind == 2:
                    args = [s32(mm.r[6]), s32(mm.r[7])]      # num, life
                e = self.alloc(16)
                mm.store(e, 1, 1)
                self.effs[e] = [kind, 0, eff_life(kind, args)]
                for k in range(1024):
                    if mm.load(MGR + 52 + 4 * k, 4) == 0:
                        mm.store(MGR + 52 + 4 * k, 4, e)
                        out.append(["eff", kind, k])
                        return k
                out.append(["eff", kind, -1])
                return 0xFFFFFFFF
            return create
        hook("ccBossEffWaveShockCreate__FPfPff", effect(0))
        hook("ccBossEffMagicSquareCreate__FPfi", effect(1))
        hook("ccBossEffForceGeneratorCreate__FPfPffffiii", effect(2))
        hook("ccBossEffAutoSamonRingCreate__FPfPfPfi", effect(3))
        hook("ccBossEffIceBreakCreate__FPff", effect(4))
        hook("ccBossEffDeadCreate__FPfPfi", effect(5))

        def skill(mm, cp, tp, sid, flag, *_):
            out.append(["skill", s32(sid)])
            return 0
        hook("ccItemSkillRequest__FP6ccCharP6ccCharii", skill)

    def effects_pass(self):
        m = self.m
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if not e:
                continue
            if not m.load(e, 1):
                m.store(MGR + 52 + 4 * k, 4, 0)
                continue
            st = self.effs[e]
            st[1] += 1
            if st[1] >= st[2]:
                m.store(e, 1, 0)

    def effects(self):
        m = self.m
        out = []
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                out.append([k, self.effs[e][0], m.load(e, 1)])
        return out


def rnd_member(c, rnd, slot):
    """A party member standing near the arena's centre, alive."""
    b, tb = c.b, c.tb
    pc = tb.rnd_pc(b, c.data, rnd)
    pc.type = 7 if slot == 0 else 6
    pc.cond = [0] * 16
    pc.HP = rnd.randrange(50, 2000)
    pc.maxHP = max(pc.HP, rnd.randrange(50, 2000))
    pc.pos = [fbits(rnd.uniform(-900, 900)), fbits(rnd.uniform(-900, 900)), 0, F_ONE]
    pc.pos_p = list(pc.pos)
    pc.width = fbits(rnd.uniform(20, 60))
    return pc


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    frames = rnd.randrange(300, 1500)
    script = []
    # hits from Kite, the gauge now and then, the drain late
    for f in sorted(rnd.sample(range(frames), min(frames // 20, 60))):
        kind = rnd.choice((0, 0, 0, 1, 4))
        p0 = {0: rnd.randrange(-1, 400), 1: rnd.randrange(0, 300), 4: rnd.randrange(0, 200)}[kind]
        script.append((f, kind, p0))
    if rnd.random() < 0.6:
        f = rnd.randrange(frames // 3, frames)
        script += [(f, 2, 0), (f, 3, 0)]
        # and now and then Kite finishing it off (4500 HP after the 21)
        if rnd.random() < 0.5:
            g = f + rnd.randrange(1, 60)
            while g < frames:
                script.append((g, 0, rnd.randrange(150, 400)))
                g += rnd.randrange(2, 12)
        script.sort(key=lambda s: s[0])
    menus = []
    if rnd.random() < 0.5:
        for _ in range(rnd.randrange(1, 4)):
            a = rnd.randrange(0, frames)
            menus += [(a, rnd.choice((0, 5, 74))), (a + rnd.randrange(1, 40), -1)]
        menus.sort()
    center = [fbits(rnd.uniform(-200, 200)), fbits(rnd.uniform(-200, 200)), 0, F_ONE]
    seed = rnd.randrange(1, 1 << 32)
    return {"party": party, "frames": frames, "script": script, "menus": menus, "center": center,
            "rand": rnd.randrange(0, 1 << 63), "seed": seed, "mti": rnd.randrange(1, 624), "count": rnd.randrange(0, 1000)}


def run_game(game, case):
    c, g, m = game.c, game.g, game.m
    c.reset()
    game.fake_at = FAKE
    game.effs.clear()
    game.anms.clear()
    game.out.clear()
    game.stage_on = False
    m.mem[BOSS:BOSS + BOSS_SIZE] = bytes(BOSS_SIZE)
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.store(game.sym("_g_bossEffManager"), 4, MGR)
    m.store(game.sym("scFadeDef"), 4, STAGE + 0x200)
    party = case["party"]
    chars = party
    c.scene.put(chars, list(range(len(chars))), [])
    # the command lists' ends, for ccEntryCmnd: the party's last, no enemies
    m.store(inf_va(0x00378C4C), 4, SCN + 0x1000 * (len(chars) - 1))
    m.store(inf_va(0x00378C54), 4, 0)
    m.store(inf_va(0x00378C5C), 4, 0)
    game.center = case["center"]
    kite = SCN
    m.store(PLW + 0x20, 4, kite)
    for k in range(3):
        m.store(PARTY + 4 * k, 4, SCN + 0x1000 * k if k < len(party) else 0)
        m.store(PARTY + 12 + 4 * k, 4, party[k].id if k < len(party) else 0xFFFFFFFF)
    m.store(PARTY + 24, 4, len(party))
    for k in range(len(party)):
        m.store(SCN + 0x1000 * k + 0x94, 4, 0)
    g.set_rand(case["rand"])
    mt = eai.sgenrand(case["seed"])
    for k in range(624):
        m.store(eai.MT + 8 * k, 8, mt[k])
    m.store(eai.MTI, 4, case["mti"])
    g.env.menu_type = -1
    m.mem[MENU:MENU + 0x240] = bytes(0x240)
    sys_ = m.load(g.sym("ccSys"), 4)
    m.store(sys_ + 0x358, 4, case["count"])
    m.call(game.sym("__ct__8ccBoss01Fv"), (BOSS,))
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    kite_va = SCN
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        for sf, kind, p0 in case["script"]:
            if sf != f:
                continue
            if kind == 0:
                m.call(ea, (BOSS, kite_va, 1, p0 & 0xFFFF, 0, 0))
            elif kind == 1:
                m.store(BOSS + 0xE0 + 0x60, 2, p0 & 0xFFFF)
            elif kind == 2:
                m.call(ea, (BOSS, kite_va, 13, 0, 0, 0))
            elif kind == 3:
                m.call(ea, (BOSS, kite_va, 21, 0, 0, 0))
            elif kind == 4:
                m.store(BOSS + 0xE0 + 0x62, 2, p0 & 0xFFFF)
        game.effects_pass()
        m.call(game.sym("Main__8ccBoss01Fv"), (BOSS,))
        # the menu's bar, as Main writes it
        frames.append(game_state(game, len(party)))
    return frames


def game_state(game, n):
    m = game.m
    a = BOSS
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    anm = m.load(a + 0xD4, 4)
    wave = m.load(a + 0x29358, 4)
    clip = game.anms.get(anm, [None, 0])
    wclip = game.anms.get(wave, [None, 0])
    tgt = m.load(a + 0x78, 4)
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "pat": [pat_tbl(m.load(a + 0x208, 4)), s32(ld(0x20C)), s32(ld(0x210)), s32(ld(0x29644))],
        "stop": [s32(ld(0x29328)), s32(ld(0x2932C)), ld(0x1D2, 1, True), ld(0x1A0, 2, True)],
        "move": [ld(0x188), ld(0x18C), ld(0x190), ld(0x194), ld(0x198)],
        "tgt": [(tgt - SCN) // 0x1000 if tgt else -1, ld(0x180), ld(0x184)],
        "pos": [ld(0x40), ld(0x44), ld(0x48)],
        "posp": [ld(0x50), ld(0x54), ld(0x58)],
        "dirc": ld(0x68),
        "hp": [ld(0x70, 2, True), ld(0x74, 2, True)],
        "pp": [ld(0x140, 2, True), ld(0x142, 2, True)],
        "flags": [ld(0x1D8, 1, True), ld(0x1D3, 1, True), ld(0x1D7, 1, True), s32(ld(0x2BC)), ld(0x1D6, 1, True),
                  ld(0x1D5, 1, True), ld(0x1D4, 1, True), ld(0x1D0, 1, True), s32(ld(0x29334)), s32(ld(0x29338)),
                  ld(0x1D1, 1, True)],
        "anm": [clip[0] or "", clip[1] >> 8],
        "wave": [wclip[0] or "", wclip[1] >> 8, m.load(wave + 0x9C, 2)],
        "tr": [ld(0x294B0 + 0x120 - 0x130), ld(0x8C)],
        "center": [ld(0x200), ld(0x204)],
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2))],
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def pat_tbl(p):
    return {inf_va(0x005EB430): 0, inf_va(0x005EB490): 1, inf_va(0x005EB530): 2}.get(p, -1)


def request(game, case, pre):
    """The probe's `boss` request: the scene as the constructor found it."""
    b = game.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    kite = party[0]
    kdirc = [0, 0, 0, 0]
    parts = (["boss", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + kdirc
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], len(case["script"])])
    for s in case["script"]:
        parts += list(s)
    parts.append(len(case["menus"]))
    for mm in case["menus"]:
        parts += list(mm)
    return " ".join(str(x) for x in parts)


def compare(game, case, label):
    # The boss's character before the constructor: the row's, at the
    # origin; the probe makes Skeith on it as the game did.
    pre = boss_char_pre(game)
    g_frames = run_game(game, case)
    p_frames = rs.ask([request(game, case, pre)])[0]
    for f, (a, b_) in enumerate(zip(g_frames, p_frames)):
        a = rs.norm(a)
        for k in a:
            if a[k] != b_.get(k):
                raise AssertionError(f"{label} frame {f - 1}: {k}\n game {a[k]}\n port {b_.get(k)}\n"
                                     f" game {a}\n port {b_}")
    if len(g_frames) != len(p_frames):
        raise AssertionError(f"{label}: {len(g_frames)} game frames, {len(p_frames)} port frames")


def boss_char_pre(game):
    """Skeith's ccChar as SetBaseParam leaves it (bossTbl row 0)."""
    import types
    row = dict(game.c.data.bosses[0])
    ch = types.SimpleNamespace()
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level, ch.exp = 0, row.get("level", 1), 0
    ch.real = list(row["elm"])
    ch.temp, ch.time = [0] * 16, [0] * 16
    ch.PP = ch.PPcount = ch.PPrestore = 0
    ch.HP, ch.SP, ch.maxHP, ch.maxSP = row["maxHP"], row["maxSP"], row["maxHP"], row["maxSP"]
    ch.cond = [0] * 16
    ch.speed_value = F_ONE
    ch.ent_root = 0
    ch.pos = [0, 0, 0, F_ONE]
    ch.pos_p = [0, 0, 0, F_ONE]
    ch.width = row.get("width", 0)
    return ch


# Skeith's calls (the arguments ccBoss01 passes; pos and dirc aside)
CREATES = {
    0: ("ccBossEffWaveShockCreate__FPfPff", "pd", [1.0], ()),
    1: ("ccBossEffMagicSquareCreate__FPfi", "p", [], (0,)),
    2: ("ccBossEffForceGeneratorCreate__FPfPffffiii", "pd", [10.0, 200.0, 200.0], (8, 100, 8)),
    3: ("ccBossEffAutoSamonRingCreate__FPfPfPfi", "pdq", [], (35,)),
    4: ("ccBossEffIceBreakCreate__FPff", "p", [2.0], ()),
    5: ("ccBossEffDeadCreate__FPfPfi", "pp", [], (0,)),
}


def native_life(game, kind):
    """The Draws the game's effect runs until it clears m_bEnabled: its
    ccBossEff*Create run natively (the stand-in unhooked), then the Draw of
    what it put in the manager, frame after frame."""
    m, sym = game.m, game.sym
    name, ptrs, floats, ints = CREATES[kind]
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.store(sym("_g_bossEffManager"), 4, MGR)

    def vec(*v):
        a = game.alloc(16)
        for k, x in enumerate(v):
            m.store(a + 4 * k, 4, fbits(x))
        return a
    pos, dirc = vec(100.0, -200.0, 0.0, 1.0), vec(0.0, 0.0, 0.5, 0.0)
    # AutoSamonRing's param: OnMagicAtk's (0, 1/3, 10)
    prm = vec(0.0, 1.0 / 3, 10.0, 0.0)
    args = tuple({"p": pos, "d": dirc, "q": prm}[c] for c in ptrs) + ints
    for k, x in enumerate(floats):
        m.f[12 + k] = fbits(x)
    stand_in = m.hooks.pop(sym(name))
    try:
        k = s32(m.call(sym(name), args))
    finally:
        m.hooks[sym(name)] = stand_in
    # the effect the id names (MagicSquare's returns -1: the one it made)
    made = [e for e in (m.load(MGR + 52 + 4 * j, 4) for j in range(1024)) if e]
    e = m.load(MGR + 52 + 4 * k, 4) if k >= 0 else made[0]
    assert k >= 0 or len(made) == 1, made
    draw = m.load(m.load(e + 12, 4) + 8, 4)
    n = 0
    while m.load(e, 1) and n < 2000:
        m.call(draw, (e,))
        n += 1
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    return n


class Against(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not (os.path.exists(rs.ELF) and os.path.exists(rs.ISO)):
            raise unittest.SkipTest("the disc is not extracted")
        rs.build()
        cls.game = Game()

    def test_effect_lives(self):
        for kind, (name, _, _, ints) in CREATES.items():
            args = list(ints[:2]) if kind == 2 else []
            with self.subTest(name):
                self.assertEqual(native_life(self.game, kind), eff_life(kind, args))

    def test_cases(self):
        for seed in range(6):
            rnd = random.Random(7000 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=9000):
    rs.build()
    game = Game()
    for k in range(n):
        rnd = random.Random(seed0 + k)
        compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        print(f"case {seed0 + k} ok", flush=True)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]))
    else:
        unittest.main()
