#!/usr/bin/env python3
"""crates/piney-battle's Magus (src/boss/magus.rs, magus/leaf.rs) against
Mutation's boss03.cpp run natively in the Rust VU0 machine
(tools/eemu_rs.so), frame by frame with battle_probe's `magus`.

PINEY_VOLUME=mutation is required: Magus is fought in Mutation, and the
port plays Mutation with Mutation's code (gcmn 0x0049da20-0x004a7974).

A case lays a party of one to three members out on the command lists
(tools/test_battle_rs.py's scene), builds Magus with the game's own
constructor (ccBoss03::ccBoss03, with its twelve ccBoss03Leaf) at BOSS and
runs ccBoss03::Main frame after frame, with ccBossEffManager::Draw's pass
before each, scripted affects between frames (Kite's hits on Magus and on
its leaves, the protect gauge and its break, the drain's 13 and 21) and the
menu's type changing. Compared each frame: Magus's acts, movement, place,
target, HP and gauge, flags, clips and own members (Mutation's layout);
its laser shocks; each leaf's acts, place, HP, lists, clip and own
members, its countdown, blink and burst; the effects' slots; the party's HP
and hold; the calls (sounds, flashes, cinema, stage, cameras 1 and 3,
shakes, particles, skills, fly fonts, hit marks); the menu's words;
newlib's rand() and ccRand's index.

What is not the game's here:
- The party's affects are off (affectFunc 0): Magus's hits are compared by
  what EntryAffect stores and the rand() they draw.
- The files: GetCCSAdrs, GetChunkAdrsF, the clumps and effect objects are
  stand-ins; ccAnm's SetAnm and _AnimateForward follow the clips' lengths
  (battle_probe `magusclips`, from x31 and xeffect). DMY_center01 is the
  case's centre. The body's leaves (EXT_ex31leaf*, OBJ_ex31ligh,
  OBJ_ex31les*) are stand-in objects: each leaf hangs at the case's place
  (its world matrix), and a shock's beam (ccHitCheckLM) misses every
  case's nth call.
- The WaveShock, the dead effect and a leaf's ring are stand-ins with the
  port's lives (test_effect_lives checks those against the game's); the
  needles (ccBossEffNeedle) run natively, drawing nothing.
- The particle generators, dust, bursts and fonts only note their start;
  checkCameraShakeRange answers yes and cameraShake only notes itself (the
  port's runtime draws its rand()); W2P/P2W are identity and
  CollisionDetection finds nothing. A skill's request sets the caster's
  skillID and status 9 when cast through it (flag set), as the port's
  request does.

    PINEY_VOLUME=mutation python3 tools/test_battle_magus_rs.py             the tests
    PINEY_VOLUME=mutation python3 tools/test_battle_magus_rs.py bulk N [S]  N cases from seed S
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle_enemy_ai_rs as eai  # noqa: E402
import test_battle_rs as rs  # noqa: E402
from test_battle_boss_rs import forward_clip, rnd_member, s16, s32  # noqa: E402
from test_battle_rs import F_ONE, NEW, SCN, fbits, ser_char  # noqa: E402

BOSS = 0x01300000               # the ccBoss03 (Mutation's 0x52a70 bytes)
FAKE = 0x01360000               # stand-in streams, chunks, objects and effects
MGR = 0x01380000                # _g_bossEffManager (0x1038 bytes)
STAGE = 0x01390000              # the stage fader
WM = 0x013B0000                 # worldman and its event area's stream
SLAVES = 0x01400000             # operator new[]: the twelve ccBoss03Leaf
PLW, PARTY = inf_va(0x007302E0), inf_va(0x00730310)
MENU = 0x01020000               # test_battle's ccMenu
BOSS_SIZE = 0x52A70
LEAF_SIZE = 0x29810

# The stand-in effects' kinds (battle_probe's eff_num) and lives.
WAVE, DEAD, NEEDLE, RING = 0, 5, 11, 12


def eff_life(kind):
    """The Draws piney_battle::boss::Eff::draw gives a stand-in."""
    return {WAVE: 45, DEAD: 120, RING: 20}[kind]


def fvec(v):
    return [fbits(x) for x in v]


class Game:
    """The machine with Magus's hooks."""

    def __init__(self):
        self.c = rs.Checks()
        g = self.c.game
        self.g, self.m = g, g.m
        self.clips = rs.ask(["magusclips"])[0]
        self.names = {}         # a stand-in chunk or stream -> its name
        self.anms = {}          # a ccAnm -> [clip, time]
        self.effs = {}          # a stand-in effect -> [kind, count, life]
        self.gens = {}          # a particle generator -> its name
        self.out = []
        self.hooks()

    def sym(self, name):
        return self.g.sym(name)

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

    def obj(self, name, pos=None):
        """A stand-in ccObj named `name` (its chunk's name at +0x90 + 8),
        with a parent and, given `pos`, that world place."""
        m = self.m
        o = self.alloc(0xB0)
        ch = self.alloc(8 + len(name) + 1)
        m.mem[ch + 8:ch + 8 + len(name)] = name.encode()
        m.store(o + 0x90, 4, ch)
        m.store(o + 0x80, 4, 1)
        for k in range(4):
            m.store(o + 0x10 * k + 4 * k, 4, F_ONE)
        if pos is not None:
            for k in range(3):
                m.store(o + 0x30 + 4 * k, 4, pos[k])
        return o

    def hooks(self):
        m = self.m
        h = m.hooks
        out = self.out
        sym = self.sym

        def hook(name, fn):
            h[sym(name)] = fn

        nop = lambda mm, *a: 0  # noqa: E731
        this = lambda mm, a, *_: a  # noqa: E731
        h.pop(sym("EntryAffect__6ccCharFP6ccCharssss"), None)
        h.pop(sym("checkPartyAnnihilation__Fv"), None)

        def new_array(mm, n, *_):
            a = self.slaves_at
            self.slaves_at += (n + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            return a
        hook("__nwa__FUi", new_array)
        hook("GetCCSAdrs__8ccStreamFPCc", lambda mm, n, *_: self.alloc(16, self.cstr(n)))

        def chunk(mm, s, n, *_):
            name = self.cstr(n)
            a = self.alloc(32, name)
            if name == "DMY_center01":
                for k in range(4):
                    mm.store(a + 16 + 4 * k, 4, self.center[k])
            return a
        hook("GetChunkAdrsF__8ccStreamFPCci", chunk)
        hook("GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult", lambda mm, *a: self.alloc(0x100))
        hook("GetSubstAdrsF__5ccAnmFPCcb", lambda mm, *a: self.alloc(0x100))

        def reset(mm, r, n, *_):
            mm.store(r, 4, 0)
            mm.store(r + 4, 2, 0)
            mm.store(r + 6, 2, n & 0xFFFF)
            return 0
        hook("Reset__19ccSubstSearchResultFUs", reset)

        def subst(mm, anm, pat, r, *_):
            p = self.cstr(pat)
            objs = self.subst.get(p, [])
            tbl = self.alloc(8 * max(len(objs), 1))
            for k, o in enumerate(objs):
                mm.store(tbl + 8 * k, 4, o)
                mm.store(tbl + 8 * k + 4, 2, 2304)
            mm.store(r, 4, tbl)
            mm.store(r + 4, 2, len(objs))
            return 0
        hook("GetSubstAdrs__5ccAnmFPCcP19ccSubstSearchResult", subst)
        hook("__ct__7ccCoordFv", this)
        for name in ("Init__7ccClumpFP12ccClumpChunk", "ApplyClump__5ccAnmFP7ccClumpP8ccStream",
                     "Init__8ccEffObjFP10ccEffChunk", "Init__5ccObjFP10ccObjChunkPP7ccCoordUi",
                     "_SetLWMatrix__7ccCoordFv", "__dla__FPv", "__dl__FPv"):
            hook(name, nop)

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
            # anmChunk (+0xa8): its frame count at +0x10.
            ch = self.alloc(32, name)
            m.store(ch + 0x10, 4, self.clips.get(name, [0, 0])[0])
            m.store(a + 0xA8, 4, ch)

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
                     "InitAfterImage__6ccBossFP5ccAnmi", "DrawBossEffect__6ccBossFv",
                     "SetMatrix_PosRotXYZScale__7ccCoordFPfPfPf", "SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf",
                     "Draw__7ccClumpFv", "SetTransparency__7ccClumpFf", "__dt__7ccClumpFv",
                     "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff", "__dt__7ccLayerFv",
                     "ccSqFade__FiUsiUc", "SetHitSW__9ccCharHitFi", "SetActiveLayer__9WORLD_MANFi",
                     "Draw__5ccEffFPfUs", "Draw__5ccObjFf", "SetFogBlend__9ccDrawEnvFfUi",
                     "ResetFogBlend__9ccDrawEnvFv", "SetType__6ccFontFi", "MakeSignedNum__6ccFontFil",
                     "ccParticleExplode__FPfPffi", "DrawPartsClump__6ccBossFP7ccClump"):
            hook(name, nop)
        def collide(mm, hit, *_):
            # The case's push: a hit with no offset (+0x30).
            if self.push[0] <= self.frame_no < self.push[1]:
                mm.mem[hit + 0x30:hit + 0x40] = bytes(16)
                return 1
            return 0
        hook("CollisionDetection__9ccCharHitFv", collide)
        hook("__ct__16ccDrawPacketCtrlFv", this)
        hook("ccCalcTagPos__FRA4_fRA4_iPf", lambda mm, *a: 0)

        def stage_init(mm, a, *_):
            mm.store(a + 0x29340, 4, STAGE + 0x100)
            mm.store(a + 0x29344, 4, STAGE)
            mm.store(a + 0x29348, 1, 1)
            mm.store(a + 0x2934C, 4, 0)
            mm.store(a + 0x2933C, 1, 1)
            return 0
        hook("InitStageEffect__6ccBossFv", stage_init)
        self.stage_on = False

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
        hook("EntryFlash__8ccScFadeFiiffff",
             lambda mm, f, t, c, *_: out.append(["flash", s32(t), c & 0xFFFFFFFF]) or 0)
        hook("EntryFlash2__8ccScFadeFiiiffff",
             lambda mm, f, t0, t1, c, *_: out.append(["flash2", s32(t0), s32(t1), c & 0xFFFFFFFF]) or 0)
        hook("EntryFlash3__8ccScFadeFiiiiffff",
             lambda mm, f, t0, t1, t2, *_: out.append(["flash3", s32(t0), s32(t1), s32(t2),
                                                       mm.r[8] & 0xFFFFFFFF]) or 0)
        hook("ccSeOn3D__FiPf", lambda mm, se, p, *_: out.append(["se3d", s32(se)]) or 0)
        hook("ccSeOn__Fi", lambda mm, se, *_: out.append(["se", s32(se)]) or 0)
        hook("ccSeOn3DNote__FiPfc", lambda mm, se, p, n, *_: out.append(["se3dnote", s32(se), n & 0xFF]) or 0)
        hook("ccSeOnNote__Fic", lambda mm, se, n, *_: out.append(["senote", s32(se), n & 0xFF]) or 0)
        hook("ccSeOn3DLoop__FiPf", lambda mm, se, *_: out.append(["seloop", s32(se)]) or 5)
        hook("ccSeOffLoop__Fii", lambda mm, se, *_: out.append(["seoff", s32(se)]) or 0)
        hook("EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T", lambda mm, *a: out.append(["afterimage"]) or 0)
        hook("EntryAfterImageAnm__6ccBossFP5ccAnmPfPfP17ENTRYAFTERIMAGE_T",
             lambda mm, *a: out.append(["afterimage_leaf"]) or 0)
        hook("OnCinemaMode__16ccBossEffManagerFi", lambda mm, a, n, *_: out.append(["cinema", s32(n)]) or 0)
        hook("OffCinemaMode__16ccBossEffManagerFv", lambda mm, *a: out.append(["cinema", -99]) or 0)
        hook("ccEntryFlyFontNew__FiiPfP6ccCharff",
             lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccHitMarkDisp__FP6ccCharP6ccChar", lambda mm, *a: out.append(["hitmark"]) or 0)
        hook("ccClearSpcCondition__Fv", lambda mm, *a: out.append(["clear_spc"]) or 0)
        hook("ccEnemyEffDust__FPfif", lambda mm, *a: out.append(["dust"]) or 0)
        hook("effSmokeRock__FPfPffiii", lambda mm, *a: out.append(["shock_burst"]) or 0)
        hook("checkCameraShakeRange__FPf", lambda mm, *a: 1)
        hook("cameraShake__Fiiii", lambda mm, *a: out.append(["shake"]) or 0)

        def change_camera(mm, n, *_):
            out.append(["cam", s32(n)])
            mm.store(sym("camID"), 2, n & 0xFFFF)
            return 0
        hook("changeCamera__Fi", change_camera)

        def cam_set(key):
            def put(mm, p, n, *_):
                out.append([key, s32(n)] + [mm.load(p + 4 * k, 4) for k in range(3)])
                return 0
            return put
        hook("cameraSetPos__FPfi", cam_set("campos"))
        hook("cameraSetView__FPfi", cam_set("camview"))
        hook("ccTransPosW2P__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)
        hook("ccTransPosP2W__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)

        def hit_lm(mm, *_):
            # One call a shock's Draw, in order: the case's misses.
            k = self.hit_calls
            self.hit_calls += 1
            miss = self.miss > 0 and (self.frame_no + k) % self.miss == 0
            mm.f[0] = 0xBF800000 if miss else fbits(0.5)
            return 0
        hook("ccHitCheckLM__FPfPfUi", hit_lm)

        gens = {sym("g_thunderGenerator"): "thunder", sym("g_leafDeadGen"): "leafdead",
                sym("g_energyChargeGenerator") + 0x38: "charge", sym("g_leafMarkerGen"): "marker"}

        def gen_ct(mm, a, param, *_):
            self.gens[a] = gens.get(param, "?")
            return a
        hook("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParamP25"
             "ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam", gen_ct)

        def gen_start(mm, a, *_):
            name = self.gens.get(a, "?")
            if name != "marker":
                out.append(["particles", name])
            return 0
        hook("startParticleGenerator__FP19ccParticleGenerator", gen_start)
        hook("ccCheckParticleGenerator__FP19ccParticleGenerator", lambda mm, *a: 1)
        hook("ParticleKill__19ccParticleGeneratorFv", lambda mm, *a: out.append(["kill"]) or 0)

        def effect(kind):
            def create(mm, *a):
                e = self.alloc(16)
                mm.store(e, 1, 1)
                self.effs[e] = [kind, 0, eff_life(kind)]
                for k in range(1024):
                    if mm.load(MGR + 52 + 4 * k, 4) == 0:
                        mm.store(MGR + 52 + 4 * k, 4, e)
                        out.append(["eff", kind, k])
                        return k
                out.append(["eff", kind, -1])
                return 0xFFFFFFFF
            return create
        hook("ccBossEffWaveShockCreate__FPfPff", effect(WAVE))
        hook("ccBossEffDeadCreate__FPfPfi", effect(DEAD))
        hook("ccBossEffAutoSamonRingCreate__FPfPfPfi", effect(RING))

        def skill(mm, cp, tp, sid, flag, *_):
            out.append(["skill", s32(sid), int(flag != 0)])
            if flag:
                mm.store(cp + 0x7C, 2, sid & 0xFFFF)
                mm.store(cp + 0x7E, 2, 9)
            return 0
        hook("ccItemSkillRequest__FP6ccCharP6ccCharii", skill)
        hook("ccItemSkillCompel__FP6ccCharP6ccCharii", skill)

    def effects_pass(self):
        """ccBossEffManager::Draw's loop: a disabled effect freed, a
        stand-in's life counted, a native effect's Draw (vtable +8) run."""
        m = self.m
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if not e:
                continue
            if not m.load(e, 1):
                m.store(MGR + 52 + 4 * k, 4, 0)
                continue
            st = self.effs.get(e)
            if st is not None:
                st[1] += 1
                if st[1] >= st[2]:
                    m.store(e, 1, 0)
                continue
            draw = m.load(m.load(e + 12, 4) + 8, 4)
            m.call(draw, (e,))

    def effects(self):
        m = self.m
        out = []
        needle = self.sym("__vt__15ccBossEffNeedle")
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                kind = self.effs[e][0] if e in self.effs else (NEEDLE if m.load(e + 12, 4) == needle else -1)
                out.append([k, kind, m.load(e, 1)])
        return out


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    for p in party:
        if rnd.random() < 0.2:
            p.cond[rnd.choice((8, 9, 10, 11, 13, 14))] = rnd.randrange(1, 100)
    frames = rnd.randrange(600, 3000) if rnd.random() < 0.7 else rnd.randrange(3000, 7000)
    max_pp = c.data.bosses[2]["maxPP"]
    script = []
    # Hits on the leaves, some hard enough to kill them, and on Magus;
    # the gauge set now and then (past half brings the laser).
    rough = rnd.random() < 0.5
    for f in sorted(rnd.sample(range(frames), min(frames // (8 if rough else 25), 300))):
        kind = rnd.choice((0, 1, 4, 5, 5, 5, 5))
        who = rnd.randrange(0, 12)
        p0 = {0: rnd.randrange(-1, 900), 1: rnd.randrange(0, max_pp + 1), 4: rnd.choice((0, 0, rnd.randrange(0, 300))),
              5: rnd.randrange(-1, 400 if rough else 150)}[kind]
        script.append((f, kind, who, p0))
    if rnd.random() < 0.45:
        f = rnd.randrange(frames // 3, frames)
        script += [(f, 2, 0, 0), (f, 3, 0, 0)]
        # Magus's own skills (pattern 10) end now and then (kind 6), so the
        # Epitaph's patterns go on.
        g = f + rnd.randrange(40, 160)
        while g < frames:
            script.append((g, 6, 0, 0))
            g += rnd.randrange(40, 160)
        if rnd.random() < 0.4:
            g = f + rnd.randrange(1, 600)
            while g < frames:
                script.append((g, 0, 0, rnd.randrange(300, 900)))
                g += rnd.randrange(2, 30)
    script.sort(key=lambda s: s[0])
    # A while when Magus is pushed out of something each frame (act 8).
    push = [0, 0]
    if rnd.random() < 0.4:
        a = rnd.randrange(0, frames)
        push = [a, a + rnd.randrange(60, 250)]
    menus = []
    a = 0
    while a < frames and rnd.random() < 0.7:
        a += rnd.randrange(20, 300)
        menus += [(a, rnd.choice((0, 5, 74))), (a + rnd.randrange(1, 60), -1)]
        a = menus[-1][0]
    center = [fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(-300, 300)), 0, F_ONE]
    leaves = [[fbits(rnd.uniform(-700, 700)), fbits(rnd.uniform(-1200, 200)), fbits(rnd.uniform(0, 500))]
              for _ in range(12)]
    return {"party": party, "frames": frames, "script": script, "menus": menus, "center": center,
            "leaves": leaves, "miss": rnd.choice((0, 0, 3, 5, 7)), "push": push,
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}


def leaf_of(m, k):
    return m.load(BOSS + 0x290, 4) + LEAF_SIZE * k


def listed(game, ch):
    m = game.m
    a = m.load(game.sym("cmndEneRoot"), 4)
    n = 0
    while a and n < 64:
        if a == ch:
            return 1
        a = m.load(a + 0xBC, 4)
        n += 1
    return 0


def run_game(game, case):
    c, g, m = game.c, game.g, game.m
    c.reset()
    c.scene.new_at = NEW
    game.fake_at = FAKE
    game.slaves_at = SLAVES
    game.effs.clear()
    game.anms.clear()
    game.gens.clear()
    game.out.clear()
    game.stage_on = False
    game.center = case["center"]
    game.miss = case["miss"]
    game.push = case["push"]
    game.frame_no = 0
    game.hit_calls = 0
    m.mem[BOSS:BOSS + BOSS_SIZE] = bytes(BOSS_SIZE)
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.store(game.sym("_g_bossEffManager"), 4, MGR)
    m.store(game.sym("scFadeDef"), 4, STAGE + 0x200)
    m.store(game.sym("worldman"), 4, WM)
    m.store(WM + 1092, 4, WM + 0x800)
    m.store(WM + 0x800 + 420, 4, WM + 0x1000)
    game.subst = {
        "EXT_ex31leaf*": [game.obj("OBJ_ex31leaf" if k == 0 else f"OBJ_ex31leaf{k:02}", case["leaves"][k])
                          for k in range(12)],
        "OBJ_ex31ligh": [game.obj(f"OBJ_ex31ligh{k:02}") for k in range(12)],
        "OBJ_ex31les*": [game.obj(f"OBJ_ex31les{5 * k:02}") for k in range(12)],
    }
    party = case["party"]
    c.scene.put(party, list(range(len(party))), [])
    m.store(game.sym("cmndPcLast"), 4, SCN + 0x1000 * (len(party) - 1))
    m.store(game.sym("cmndEneLast"), 4, 0)
    m.store(game.sym("cmndEneRoot"), 4, 0)
    m.store(inf_va(0x00378C5C), 4, 0)
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
    game.out.clear()
    m.call(game.sym("__ct__8ccBoss03Fv"), (BOSS,))
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        game.frame_no = f
        game.hit_calls = 0
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        for sf, kind, who, p0 in case["script"]:
            if sf != f:
                continue
            if kind == 0:
                m.call(ea, (BOSS, kite, 1, p0 & 0xFFFF, 0, 0))
            elif kind == 1:
                m.store(BOSS + 0xE0 + 0x60, 2, p0 & 0xFFFF)
            elif kind == 2:
                m.call(ea, (BOSS, kite, 13, 0, 0, 0))
            elif kind == 3:
                m.call(ea, (BOSS, kite, 21, 0, 0, 0))
            elif kind == 4:
                m.store(BOSS + 0xE0 + 0x62, 2, p0 & 0xFFFF)
            elif kind == 5:
                lf = leaf_of(m, who)
                if listed(game, lf) and s16(m.load(lf + 8, 2)) == 0 and m.load(lf + 0x1D4, 1) == 0:
                    m.call(ea, (lf, kite, 1, p0 & 0xFFFF, 0, 0))
            elif kind == 6:
                m.store(BOSS + 0x7C, 2, 0)
                m.store(BOSS + 0x7E, 2, 0)
        game.effects_pass()
        m.call(game.sym("Main__8ccBoss03Fv"), (BOSS,))
        frames.append(game_state(game, len(party)))
    return frames


def f32s(m, a, n):
    return [m.load(a + 4 * k, 4) for k in range(n)]


def fields(m, base, spec):
    """The members at `base` by (offset, kind): 'i' an int, 'h' a short,
    'b' a signed byte, 'u' an unsigned byte, 'f' a float's bits, 'v' a
    vector's x, y and z, 'n' an int's being other than -1, 'p' a pointer
    set."""
    out = []
    for off, kind in spec:
        a = base + off
        if kind == "i":
            out.append(m.load(a, 4, True))
        elif kind == "h":
            out.append(m.load(a, 2, True))
        elif kind == "b":
            out.append(m.load(a, 1, True))
        elif kind == "u":
            out.append(m.load(a, 1))
        elif kind == "f":
            out.append(m.load(a, 4))
        elif kind == "n":
            out.append(int(s32(m.load(a, 4)) != -1))
        elif kind == "p":
            out.append(int(m.load(a, 4) != 0))
        else:
            out += f32s(m, a, 3)
    return out


# ccBoss03's own members (Mutation's offsets): m_actCnt, m_bIsNeutralAnm,
# m_atkWait, m_fPrevRise, m_laserFlash, m_growInterval, m_bEpitaph,
# m_camID, m_shockIdx, m_bPrepareExplodeAll, m_bDropping, m_breserveLaser,
# m_bExplosion, m_dmgCount, m_bHighDrive, m_bReserveLeafDrop,
# m_explodeLeafNum, m_laserCount, m_leafGrowSE (a voice), +0x52720-c,
# m_blurRad, m_blurScale, m_bNoLeaf, m_camCount, m_vecPrevLaser,
# m_vecPrevDirc; ccBoss's actPos and actDirc, m_animateChase's fade,
# stopTime, stopCounter, patIndex, patNum; the blur's colour.
MAGUS = ([(0x526C0, "i"), (0x526D0, "i"), (0x526D4, "i"), (0x526D8, "f"), (0x526DC, "i"), (0x526E0, "i"),
          (0x526E4, "i"), (0x526E8, "i"), (0x526EC, "i"), (0x526F0, "i"), (0x526F4, "i"), (0x526F8, "i"),
          (0x526FC, "i"), (0x52700, "i"), (0x52704, "i"), (0x5270C, "i"), (0x52710, "i"), (0x52714, "i"),
          (0x5271C, "n"), (0x52720, "i"), (0x52724, "i"), (0x52728, "i"), (0x5272C, "i"), (0x52730, "f"),
          (0x52734, "f"), (0x52A40, "i"), (0x52A44, "i"), (0x52A50, "v"), (0x52A60, "v"), (0x1B0, "v"),
          (0x1C0, "v"), (0x52540 + 0x120, "f"), (0x29328, "i"), (0x2932C, "i"), (0x20C, "i"), (0x210, "i")])
# A leaf's own: orderNum, m_actCnt, m_bAnmEnd, m_expCntr, m_expAlpha,
# m_expAlphaSpd, m_expBaseAlpha, m_expScale, m_expScaleSpd, m_vecRot,
# m_bPrepareExplode, m_bDrop, m_blinkInterval, m_blinkBaseTime, m_bBlink,
# m_bExplode, m_fShockScale, m_transparency, m_transSpd, m_exitAlpha, the
# marker's m_forbid, m_bInitAtOnce and fade, the burst's two DRAWSTATUS,
# m_charge.
LEAF = ([(0x2B8, "i"), (0x296B0, "i"), (0x296B4, "i"), (0x296B8, "i"), (0x296BC, "f"), (0x296C0, "f"),
         (0x296C4, "f"), (0x296C8, "f"), (0x296CC, "f"), (0x296D0, "v"), (0x297E0, "i"), (0x297E4, "i"),
         (0x297E8, "i"), (0x297EC, "i"), (0x297F0, "i"), (0x297F4, "i"), (0x297F8, "f"), (0x297FC, "f"),
         (0x29800, "f"), (0x29804, "f"), (0x29370 + 0x138, "i"), (0x29370 + 0x13C, "i"),
         (0x29370 + 0x120, "f")]
        + [(0x295E0 + 0x30 + 4 * k, "f") for k in range(6)] + [(0x295E0 + 0x48 + 4 * k, "f") for k in range(6)]
        + [(0x29650, "p")])


def leaf_state(game, a):
    m = game.m
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    clip = game.anms.get(ld(0xD4), [None, 0])
    return ([ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)]
            + f32s(m, a + 0x40, 3) + f32s(m, a + 0x60, 3)
            + [ld(0x70, 2, True), ld(0x74, 2, True), ld(0x1D0, 1, True), ld(0x1D1, 1, True), listed(game, a),
               clip[0] or "", clip[1] >> 8, ld(0x88), ld(0x8C)] + fields(m, a, LEAF))


def game_state(game, n):
    m = game.m
    a = BOSS
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    anm = ld(0xD4)
    wave = ld(0x29358)
    clip = game.anms.get(anm, [None, 0])
    wclip = game.anms.get(wave, [None, 0])
    tgt = ld(0x78)
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    shocks = []
    for k in range(12):
        s = a + 0x52740 + 0x40 * k
        shocks.append([m.load(s + 0x30, 4, True), m.load(s + 0x34, 4), m.load(s + 0x38, 4, True),
                       m.load(s + 0x3C, 4, True)])
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "move": [ld(0x188), ld(0x18C)] + f32s(m, a + 0x190, 3),
        "tgt": [(tgt - SCN) // 0x1000 if tgt else -1, ld(0x180), ld(0x184)],
        "pos": f32s(m, a + 0x40, 3),
        "posp": f32s(m, a + 0x50, 3),
        "dirc": f32s(m, a + 0x60, 3),
        "hp": [ld(0x70, 2, True), ld(0x74, 2, True)],
        "pp": [ld(0x140, 2, True), ld(0x142, 2, True)],
        "flags": [ld(0x1D0, 1, True), ld(0x1D1, 1, True), ld(0x1D2, 1, True), ld(0x1D3, 1, True),
                  ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1D6, 1, True), ld(0x1D7, 1, True),
                  s32(ld(0x2BC)), s32(ld(0x29334)), int(ld(0x29338) != 0), ld(0x1A0, 2, True), listed(game, a)],
        "tr": [ld(0x88), ld(0x8C)],
        "anm": [clip[0] or "", clip[1] >> 8, m.load(anm + 0x9C, 2), wclip[0] or "", wclip[1] >> 8,
                m.load(wave + 0x9C, 2) if wave else 256],
        "magus": fields(m, a, MAGUS) + [m.load(MGR + 4 + 28, 4)],
        "shocks": shocks,
        "leaves": [leaf_state(game, leaf_of(m, k)) for k in range(12)],
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2))],
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def boss_char_pre(game):
    """Magus's ccChar as SetBaseParam leaves it (bossTbl row 2)."""
    import types
    row = dict(game.c.data.bosses[2])
    ch = types.SimpleNamespace()
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level, ch.exp = 2, row.get("level", 1), 0
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


def request(game, case, pre):
    """The probe's `magus` request."""
    b = game.c.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    kite = party[0]
    parts = (["magus", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + [0, 0, 0, 0]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], case["miss"]]
             + case["push"])
    for p in case["leaves"]:
        parts += p
    parts.append(len(case["script"]))
    for s in case["script"]:
        parts += list(s)
    parts.append(len(case["menus"]))
    for mm in case["menus"]:
        parts += list(mm)
    return " ".join(str(x) for x in parts)


def diffs(a, b_, path=""):
    """Where two JSON values part: the paths and both sides, a few."""
    if isinstance(a, list) and isinstance(b_, list) and len(a) == len(b_):
        out = []
        for k, (x, y) in enumerate(zip(a, b_)):
            out += diffs(x, y, f"{path}[{k}]")
        return out
    return [] if a == b_ else [f"{path}: game {a} port {b_}"]


def compare(game, case, label):
    pre = boss_char_pre(game)
    g_frames = run_game(game, case)
    p_frames = rs.ask([request(game, case, pre)])[0]
    for f, (a, b_) in enumerate(zip(g_frames, p_frames)):
        a = rs.norm(a)
        for k in a:
            if a[k] != b_.get(k):
                d = "\n  ".join(diffs(a[k], b_.get(k))[:12])
                raise AssertionError(f"{label} frame {f - 1}: {k}\n  {d}\n act {a['act']} port {b_['act']}\n"
                                     f" out {a['out']}\n port {b_['out']}")
    if len(g_frames) != len(p_frames):
        raise AssertionError(f"{label}: {len(g_frames)} game frames, {len(p_frames)} port frames")
    return g_frames


# The stand-ins' Creates as Magus calls them (pos and dirc aside): the
# WaveShock, the dead effect, a leaf's ring (model 195, DeadEffect's
# parameter (0.5, 1, 0, 40)).
CREATES = {
    WAVE: ("ccBossEffWaveShockCreate__FPfPff", "pd", [1.0], ()),
    DEAD: ("ccBossEffDeadCreate__FPfPfi", "pp", [], (0,)),
    RING: ("ccBossEffAutoSamonRingCreate__FPfPfPfi", "pdq", [], (195,)),
}


def native_life(game, kind):
    """The Draws the game's effect runs until it clears m_bEnabled: its
    Create run natively (the stand-in unhooked), then the Draw of what it
    put in the manager, frame after frame."""
    m, sym = game.m, game.sym
    name, ptrs, floats, ints = CREATES[kind]
    game.fake_at = FAKE
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.store(sym("_g_bossEffManager"), 4, MGR)

    def vec(*v):
        a = game.alloc(16)
        for k, x in enumerate(v):
            m.store(a + 4 * k, 4, fbits(x))
        return a
    pos, dirc, q = vec(100.0, -200.0, 0.0, 1.0), vec(0.0, 0.0, 0.5, 0.0), vec(0.5, 1.0, 0.0, 40.0)
    args = tuple({"p": pos, "d": dirc, "q": q}[c] for c in ptrs) + ints
    for k, x in enumerate(floats):
        m.f[12 + k] = fbits(x)
    stand_in = m.hooks.pop(sym(name))
    try:
        k = s32(m.call(sym(name), args))
    finally:
        m.hooks[sym(name)] = stand_in
    made = [e for e in (m.load(MGR + 52 + 4 * j, 4) for j in range(1024)) if e]
    e = m.load(MGR + 52 + 4 * k, 4) if k >= 0 else made[0]
    draw = m.load(m.load(e + 12, 4) + 8, 4)
    # The cases hold every generator alive (the grow's charges); here, as
    # Innis's harness measures them, the game's own check (none is).
    check = sym("ccCheckParticleGenerator__FP19ccParticleGenerator")
    alive = m.hooks.pop(check)
    n = 0
    try:
        while m.load(e, 1) and n < 2000:
            m.call(draw, (e,))
            n += 1
    finally:
        m.hooks[check] = alive
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    return n


class Against(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if volume.NAME != "mutation":
            raise unittest.SkipTest("Magus is Mutation's: PINEY_VOLUME=mutation")
        if not (os.path.exists(rs.ELF) and os.path.exists(rs.ISO)):
            raise unittest.SkipTest("the disc is not extracted")
        rs.build()
        cls.game = Game()

    def test_effect_lives(self):
        for kind, (name, *_) in CREATES.items():
            with self.subTest(name):
                self.assertEqual(native_life(self.game, kind), eff_life(kind))

    def test_cases(self):
        for seed in range(6):
            rnd = random.Random(7500 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=9500):
    rs.build()
    game = Game()
    acts = set()
    leaf_acts = set()
    for k in range(n):
        rnd = random.Random(seed0 + k)
        frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        acts |= {fr["act"][0] for fr in frames}
        leaf_acts |= {lf[2] for fr in frames for lf in fr["leaves"]}
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print("acts reached:", sorted(acts), "leaves':", sorted(leaf_acts))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 9500)
    else:
        unittest.main()
