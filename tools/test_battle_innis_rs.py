#!/usr/bin/env python3
"""crates/piney-battle's Innis (src/boss/innis.rs) against Mutation's boss02.cpp
and BreakMirror.cpp run natively in the Rust VU0 machine (tools/eemu_rs.so).

PINEY_VOLUME=mutation is required: Innis is Mutation's, and the port plays
Mutation with Mutation's code. The stripped executable's functions are
found through the carried names (tools/image.py, SLUS_205.62.syms).

A case lays a party of one to three members out on the command lists
(tools/test_battle_rs.py's scene), builds Innis with the game's own
constructor (ccBoss02::ccBoss02, MUT gcmn 0x004942c0, with its three
ccBoss02Slave and their BreakMirror) at BOSS, and runs ccBoss02::Main
(0x00495d50) frame after frame, with ccBossEffManager::Draw's pass over the
effects before each, scripted affects on the boss between frames (Kite's
hits, the protect gauge and its break, the drain's 13 and 21), the menu's
type changing, and the camera turning. The same case goes to battle_probe's
`innis` request, and every frame is compared: the acts, the action words
and Innis's own members, the movement and position, the target, HP and
protect gauge, the flags, the animations, the three images (their acts,
places, turns, flights and mirrors), the effects' slots, the boss camera's
mode, eye and pitch, the blur's colour, the party's HP and hold, the calls
(sounds, flashes, cinema, stage, particles, skills, fly fonts, hit marks,
the shield), the menu's words, newlib's rand() and ccRand's index.

What is not the game's here:
- The party's affects are off (affectFunc 0): the boss's hits and holds
  are compared by what EntryAffect stores, not their damage.
- The boss's files: GetCCSAdrs, GetChunkAdrsF and the clumps are stand-ins;
  ccAnm's SetAnm and _AnimateForward follow the clips' lengths (battle_probe
  `innisclips`, from x21 and xeffect). DMY_center01 is the case's centre.
- The effects: the missiles (ccBossEff{Ice,Lightning,Blaze}Missile) and
  the rings (ccEffSamonRing) run natively, their drawing, particles,
  effIceRock and ccEnemyEffDust stubbed (so they draw nothing from rand);
  the WaveShock, the magic squares and the dead effect are stand-ins with
  the port's lives (test_effect_lives checks those against the game's).
- The boss camera is a zeroed ccBossCam that CamMain never runs on
  (SetMode, SetFreeCamPosView, QuakeCam and CheckMoveCamera run natively);
  cameraGetRot/Pos/View answer the case's camera; its catch-up count
  (CheckMoveCamera) is scripted. W2P/P2W are identity, ccLandHitCheck
  answers the case's ground, CollisionDetection nothing.

    PINEY_VOLUME=mutation python3 tools/test_battle_innis_rs.py             the tests
    PINEY_VOLUME=mutation python3 tools/test_battle_innis_rs.py bulk N [S]  N cases from seed S
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

BOSS = 0x01300000               # the ccBoss02 (0x295c0 bytes)
FAKE = 0x01340000               # stand-in streams, chunks and effects
MGR = 0x01380000                # _g_bossEffManager (0x1038 bytes)
STAGE = 0x01390000              # the stage fader
CAM = 0x013A0000                # the boss camera (Mutation's ccBossCam)
WM = 0x013B0000                 # worldman and its event area's stream
SLAVES = 0x01400000             # operator new[]: the three ccBoss02Slave
PLW, PARTY = inf_va(0x007302E0), inf_va(0x00730310)
MENU = 0x01020000               # test_battle's ccMenu
BOSS_SIZE = 0x295C0
SLAVE_SIZE = 0x29410


def eff_life(kind, n=0):
    """The Draws piney_battle::boss::Eff::draw gives a stand-in: 0
    WaveShock, 1 MagicSquare (n), 5 Dead."""
    if kind == 0:
        return 45
    if kind == 1:
        return 71 if n == 1 else 91
    return 120


class Game:
    """The machine with Innis's hooks."""

    def __init__(self):
        self.c = rs.Checks()
        g = self.c.game
        self.g, self.m = g, g.m
        self.clips = rs.ask(["innisclips"])[0]
        self.names = {}         # a stand-in chunk or stream -> its name
        self.anms = {}          # a ccAnm -> [clip, time]
        self.effs = {}          # a stand-in effect -> [kind, count, life]
        self.gens = {}          # a particle generator -> its param
        self.out = []
        self.shards = 0
        self.frame = {"rot": 0, "pos": [0, 0, 0, F_ONE], "view": [0, 0, 0, F_ONE]}
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
        hook("__ct__7ccCoordFv", this)
        hook("Init__7ccClumpFP12ccClumpChunk", nop)
        hook("ApplyClump__5ccAnmFP7ccClumpP8ccStream", nop)

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
                     "InitAfterImage__6ccBossFP5ccAnmi", "__dl__FPv", "DrawBossEffect__6ccBossFv",
                     "CamMain__9ccBossCamFv", "SetMatrix_PosRotXYZScale__7ccCoordFPfPfPf",
                     "SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf", "Draw__7ccClumpFv",
                     "SetTransparency__7ccClumpFf", "Duplicate__5ccObjFUi",
                     "ChangeClut__7ccModelFP11ccClutChunkP11ccClutChunk",
                     "ChangeClut__7ccClumpFP11ccClutChunkP11ccClutChunk", "Duplicate__7ccClumpFUi",
                     "__dt__7ccClumpFv", "ParticleKill__19ccParticleGeneratorFv", "effIceRock__FPfPfffi",
                     "ccEnemyEffDust__FPfif", "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff",
                     "__dt__7ccLayerFv", "ccSqFade__FiUsiUc", "changeCamera__Fi", "SetHitSW__9ccCharHitFi"):
            hook(name, nop)
        hook("CollisionDetection__9ccCharHitFv", lambda mm, *a: 0)
        hook("__ct__16ccDrawPacketCtrlFv", this)

        def shard(mm, *_):
            self.shards += 1
            return 0
        hook("DrawPartsClump__6ccBossFP7ccClump", shard)

        def init_cam(mm, a, *_):
            mm.mem[CAM:CAM + 0x150] = bytes(0x150)
            mm.store(a + 332, 4, CAM)
            mm.store(a + 336, 4, 1)
            return 0
        hook("InitBossCamera__6ccBossFff", init_cam)

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
        hook("ccEntryFlyFontNew__FiiPfP6ccCharff",
             lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccHitMarkDisp__FP6ccCharP6ccChar", lambda mm, *a: out.append(["hitmark"]) or 0)
        hook("ccClearSpcCondition__Fv", lambda mm, *a: out.append(["clear_spc"]) or 0)
        hook("effResistantShield__FP6ccCharii", lambda mm, *a: out.append(["shield"]) or 0)
        hook("cameraSetPos__FPfi", nop)
        hook("cameraSetView__FPfi", nop)

        def cam_rot(mm, p, n, *_):
            mm.store(p, 4, 0)
            mm.store(p + 8, 4, self.frame["rot"])
            return 0
        hook("cameraGetRot__FPfi", cam_rot)

        def cam_vec(key):
            def get(mm, p, n, *_):
                for k in range(4):
                    mm.store(p + 4 * k, 4, self.frame[key][k])
                return 0
            return get
        hook("cameraGetPos__FPfi", cam_vec("pos"))
        hook("cameraGetView__FPfi", cam_vec("view"))
        hook("ccTransPosW2P__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)
        hook("ccTransPosP2W__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)

        def land(mm, *_):
            mm.f[0] = self.ground
            return 0
        hook("ccLandHitCheck__FPfUi", land)

        tables = {sym("InisFieldGenerator"): "InisField", sym("TornadoGenerator"): "Tornado",
                  sym("BurstGenerator"): "Burst"}

        def gen_ct(mm, a, param, *_):
            self.gens[a] = param
            return a
        hook("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParamP25"
             "ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam", gen_ct)

        def gen_start(mm, a, *_):
            param = self.gens.get(a, 0)
            for base, name in tables.items():
                if 0 <= param - base < 0x38 * 8 and (param - base) % 0x38 == 0:
                    out.append(["particles", name, (param - base) // 0x38])
            return 0
        hook("startParticleGenerator__FP19ccParticleGenerator", gen_start)

        def effect(kind):
            def create(mm, *a):
                n = s32(mm.r[5]) if kind == 1 else 0
                e = self.alloc(16)
                mm.store(e, 1, 1)
                self.effs[e] = [kind, 0, eff_life(kind, n)]
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
        hook("ccBossEffDeadCreate__FPfPfi", effect(5))
        # The rings and the missiles run natively; their slots are noted
        # as they are made.
        for name, kind in (("ccBossEffSamonRingCreate__FPfPfPfi", 6),
                           ("ccBossEffIceMissileCreate__FPA4_fiPFPfi_iP9ccBossCam", 7),
                           ("ccBossEffLightningMissileCreate__FPA4_fiPFPfi_iP9ccBossCam", 8),
                           ("ccBossEffBlazeMissileCreate__FPA4_fiPFPfi_iP9ccBossCam", 9)):
            self.natives = getattr(self, "natives", {})
            self.natives[sym(name)] = kind

        def skill(mm, cp, tp, sid, flag, *_):
            out.append(["skill", s32(sid)])
            return 0
        hook("ccItemSkillRequest__FP6ccCharP6ccCharii", skill)

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
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                kind = self.effs[e][0] if e in self.effs else self.kind_of(e)
                out.append([k, kind, m.load(e, 1)])
        return out

    def kind_of(self, e):
        """A native effect's kind by its vtable (the ring 6, the missiles
        7-9)."""
        vt = self.m.load(e + 12, 4)
        return self.vtables.get(vt, -1)


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    for p in party:
        if rnd.random() < 0.3:
            p.cond[rnd.choice((8, 9, 10, 11, 13, 14))] = rnd.randrange(1, 100)
    frames = rnd.randrange(400, 1800)
    script = []
    max_pp = 15000
    quiet = rnd.random() < 0.4
    if quiet:
        # A long fight left alone in one run of the patterns (the gauge set
        # once), where the images and the member drain come round.
        frames = rnd.randrange(2500, 4000)
        script.append((rnd.randrange(0, 30), 1, rnd.choice((rnd.randrange(3001, 10500),
                                                            rnd.randrange(10501, max_pp)))))
    for f in sorted(rnd.sample(range(frames), 0 if quiet else min(frames // 15, 90))):
        kind = rnd.choice((0, 0, 0, 1, 1, 4))
        p0 = {0: rnd.randrange(-1, 1200), 1: rnd.randrange(0, max_pp), 4: rnd.choice((0, 0, rnd.randrange(0, 300)))}[kind]
        script.append((f, kind, p0))
    if rnd.random() < (0.3 if quiet else 0.6):
        f = rnd.randrange(frames * 2 // 3, frames)
        script += [(f, 2, 0), (f, 3, 0)]
        if rnd.random() < 0.6:
            g = f + rnd.randrange(1, 60)
            while g < frames:
                script.append((g, 0, rnd.randrange(300, 900)))
                g += rnd.randrange(2, 10)
        script.sort(key=lambda s: s[0])
    menus = []
    a = 0
    while a < frames and rnd.random() < 0.8:
        a += rnd.randrange(20, 200)
        menus += [(a, rnd.choice((0, 5, 74))), (a + rnd.randrange(1, 60), -1)]
        a = menus[-1][0]
    rot = [0] * frames
    r = rnd.uniform(-3.1, 3.1)
    moving = [0] * frames
    for f in range(frames):
        r += rnd.uniform(-0.05, 0.05)
        if r > 3.14159:
            r -= 6.28318
        if r < -3.14159:
            r += 6.28318
        rot[f] = fbits(r)
        moving[f] = int(rnd.random() < 0.3)
    center = [fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(-300, 300)), 0, F_ONE]
    cam_pos = [fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(100, 400)), F_ONE]
    cam_view = [fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(0, 200)), F_ONE]
    ground = fbits(rnd.choice((0.0, -50.0, rnd.uniform(-40, 10))))
    return {"party": party, "frames": frames, "script": script, "menus": menus, "center": center,
            "rot": rot, "moving": moving, "cam_pos": cam_pos, "cam_view": cam_view, "ground": ground,
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}


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
    game.ground = case["ground"]
    game.frame["pos"] = case["cam_pos"]
    game.frame["view"] = case["cam_view"]
    game.frame["rot"] = case["rot"][0]
    m.mem[BOSS:BOSS + BOSS_SIZE] = bytes(BOSS_SIZE)
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.mem[CAM:CAM + 0x150] = bytes(0x150)
    m.store(game.sym("_g_bossEffManager"), 4, MGR)
    m.store(game.sym("scFadeDef"), 4, STAGE + 0x200)
    # worldman->+1092 (the event area) ->+420 (its stream)
    m.store(game.sym("worldman"), 4, WM)
    m.store(WM + 1092, 4, WM + 0x800)
    m.store(WM + 0x800 + 420, 4, WM + 0x1000)
    party = case["party"]
    c.scene.put(party, list(range(len(party))), [])
    m.store(game.sym("cmndPcLast"), 4, SCN + 0x1000 * (len(party) - 1))
    m.store(game.sym("cmndEneLast"), 4, 0)
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
    game.vtables = {game.sym(n): k for n, k in (("__vt__14ccEffSamonRing", 6), ("__vt__19ccBossEffIceMissile", 7),
                                                  ("__vt__25ccBossEffLightningMissile", 8),
                                                  ("__vt__21ccBossEffBlazeMissile", 9))}
    game.out.clear()
    game.shards = 0
    m.call(game.sym("__ct__8ccBoss02Fv"), (BOSS,))
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        game.shards = 0
        game.frame["rot"] = case["rot"][f]
        m.store(CAM + 0xEC, 4, case["moving"][f])
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        for sf, kind, p0 in case["script"]:
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
        game.effects_pass()
        m.call(game.sym("Main__8ccBoss02Fv"), (BOSS,))
        frames.append(game_state(game, len(party)))
    return frames


def f32s(m, a, n):
    return [m.load(a + 4 * k, 4) for k in range(n)]


def fields(m, base, spec):
    """The members at `base` by (offset, kind): 'i' an int, 'h' a short,
    'b' a signed byte, 'u' an unsigned byte, 'f' a float's bits, 'v' a
    vector's x, y and z."""
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
        else:
            out += f32s(m, a, 3)
    return out


# ccBoss02's own members compared, from +0x29350 (Infection's DWARF):
# ActionFlg, Escape, EnyFlg, Enyint, Enyfloat, EntryFlg, EPITAPH_FLG, ddFlg,
# Hipos, NowMode, PatEndMode[3], PatEndFlg, LockTargetFlg, flg, MoveFlg,
# endFlg, temphi, Rotate, Dircsub, SubHi, ActionStartFlg, PosB, PosA, PFlg,
# pDist, cou, AFcou, NO, SubNO, MirrorPros, MonsterID, Alpha, BackStepDist,
# ExSpinBackFlg, EscapeActCou, DmgCount, CamDist, ZoomVec, ZoomView, CamPos,
# CamView, SkillID, QuakeVector, CamRot.z, RotVec, SubVec, Rot, SubRot.
INNIS = [(0x20, "i"), (0x24, "u"), (0x25, "u"), (0x28, "i"), (0x2C, "f"), (0x30, "i"), (0x34, "i"),
         (0x38, "i"), (0x3C, "f"), (0x98, "i"), (0x9C, "i"), (0xA0, "i"), (0xA4, "i"), (0xA8, "u"),
         (0xAC, "i"), (0xB0, "i"), (0xB4, "i"), (0xB8, "i"), (0xBC, "i"), (0xC0, "f"), (0xC4, "f"),
         (0xC8, "f"), (0xCC, "f"), (0xD4, "f"), (0xD8, "f"), (0xDC, "u"), (0xE0, "f"), (0xEC, "i"),
         (0xF4, "i"), (0xFC, "i"), (0x100, "i"), (0x104, "i"), (0x108, "i"), (0x110, "f"), (0x114, "i"),
         (0x118, "u"), (0x11C, "i"), (0x120, "i"), (0x138, "f"), (0x140, "v"), (0x150, "v"), (0x160, "v"),
         (0x170, "v"), (0x1C0, "i"), (0x1D0, "v"), (0x208, "f"), (0x210, "v"), (0x220, "v"), (0x230, "f"),
         (0x234, "f")]
# A ccBoss02Slave's: exit, drawSW, actNum, actProccess, actCount, pos, dirc,
# moveVector, targetDist, samonID, Dist, Dirc, Transparency, EnyFlg, Pos,
# spin, TargetPOS, StartCount, flg, EndFlg, orderNum, QuakeTime.
SLAVE = [(0x1D4, "b"), (0x1D5, "b"), (0x1A2, "h"), (0x1A4, "h"), (0x1A6, "h"), (0x40, "v"), (0x60, "v"),
         (0x190, "v"), (0x180, "f"), (0x29354, "i"), (0x29358, "f"), (0x2935C, "f"), (0x29360, "f"),
         (0x29364, "u"), (0x29370, "v"), (0x293B0, "v"), (0x293C0, "v"), (0x293D0, "i"), (0x293D8, "i"),
         (0x293DC, "u"), (0x2B8, "i"), (0x29400, "i")]
# Its BreakMirror's: f, SetPointFlg, the first shard's VX, VZ, X, Z.
MIRROR = [(0x110, "b"), (0x111, "u"), (0x3F8, "f"), (0x4E8, "f"), (0x5D8, "f"), (0x6C8, "f")]
# The boss camera's: ResetFlg, lock, CamView, CamPos, Xrot (+0xe0).
CAM_F = [(0x4, "i"), (0x0, "u"), (0x10, "v"), (0x20, "v"), (0xE0, "f")]


def game_state(game, n):
    m = game.m
    a = BOSS
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    anm = m.load(a + 0xD4, 4)
    wave = m.load(a + 0x29350, 4)
    clip = game.anms.get(anm, [None, 0])
    wclip = game.anms.get(wave, [None, 0])
    tgt = m.load(a + 0x78, 4)
    tpos = m.load(a + 0x295B0, 4)
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    slaves = []
    base = m.load(a + 656, 4)
    for k in range(3):
        s = base + SLAVE_SIZE * k
        sanm = game.anms.get(m.load(s + 0xD4, 4), [None, 0])
        mirror = m.load(s + 0x293E0, 4)
        slaves.append(fields(m, s, SLAVE) + [sanm[0] or "", sanm[1] >> 8] + fields(m, mirror, MIRROR))
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "move": [ld(0x188), ld(0x18C)] + f32s(m, a + 0x190, 3),
        "tgt": [(tgt - SCN) // 0x1000 if tgt else -1, ld(0x180), ld(0x184),
                -1 if tpos == a + 0x29360 else (tpos - 0x40 - SCN) // 0x1000],
        "pos": f32s(m, a + 0x40, 3),
        "posp": f32s(m, a + 0x50, 3),
        "dirc": f32s(m, a + 0x60, 3),
        "hp": [ld(0x70, 2, True), ld(0x74, 2, True)],
        "pp": [ld(0x140, 2, True), ld(0x142, 2, True)],
        "flags": [ld(0x1D0, 1, True), ld(0x1D1, 1, True), ld(0x1D2, 1, True), ld(0x1D3, 1, True),
                  ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1D6, 1, True), ld(0x1D7, 1, True),
                  s32(ld(0x2BC)), s32(ld(0x29334)), s32(ld(0x29338)), ld(0x1A0, 2, True)],
        "tr": [ld(0x88), ld(0x8C)],
        "anm": [clip[0] or "", clip[1] >> 8, wclip[0] or "", wclip[1] >> 8],
        "innis": fields(m, a + 0x29350, INNIS),
        "cam": fields(m, CAM, CAM_F),
        "blur": m.load(MGR + 4 + 28, 4),
        "slaves": slaves,
        "shards": game.shards,
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2))],
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def boss_char_pre(game):
    """Innis's ccChar as SetBaseParam leaves it (bossTbl row 1)."""
    import types
    row = dict(game.c.data.bosses[1])
    ch = types.SimpleNamespace()
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level, ch.exp = 1, row.get("level", 1), 0
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
    """The probe's `innis` request."""
    b = game.c.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    kite = party[0]
    parts = (["innis", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + [0, 0, 0, 0]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"]]
             + case["cam_pos"] + case["cam_view"] + [case["ground"]] + case["rot"] + case["moving"]
             + [len(case["script"])])
    for s in case["script"]:
        parts += list(s)
    parts.append(len(case["menus"]))
    for mm in case["menus"]:
        parts += list(mm)
    return " ".join(str(x) for x in parts)


def compare(game, case, label):
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
    return g_frames


# The stand-ins' Creates as Innis calls them (pos and dirc aside): the
# WaveShock (AllAttack's, the Lightning's landing), the magic square by its
# element, the dead effect (BeginDeadEffect).
CREATES = {
    (0, 0): ("ccBossEffWaveShockCreate__FPfPff", "pd", [1.0], ()),
    (1, 0): ("ccBossEffMagicSquareCreate__FPfi", "p", [], (0,)),
    (1, 1): ("ccBossEffMagicSquareCreate__FPfi", "p", [], (1,)),
    (1, 2): ("ccBossEffMagicSquareCreate__FPfi", "p", [], (2,)),
    (5, 0): ("ccBossEffDeadCreate__FPfPfi", "pp", [], (0,)),
}


def native_life(game, key):
    """The Draws the game's effect runs until it clears m_bEnabled: its
    Create run natively (the stand-in unhooked), then the Draw of what it
    put in the manager, frame after frame."""
    m, sym = game.m, game.sym
    name, ptrs, floats, ints = CREATES[key]
    game.fake_at = FAKE
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.store(sym("_g_bossEffManager"), 4, MGR)

    def vec(*v):
        a = game.alloc(16)
        for k, x in enumerate(v):
            m.store(a + 4 * k, 4, fbits(x))
        return a
    pos, dirc = vec(100.0, -200.0, 0.0, 1.0), vec(0.0, 0.0, 0.5, 0.0)
    args = tuple({"p": pos, "d": dirc}[c] for c in ptrs) + ints
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
    n = 0
    while m.load(e, 1) and n < 2000:
        m.call(draw, (e,))
        n += 1
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    return n


class Against(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if volume.NAME != "mutation":
            raise unittest.SkipTest("Innis is Mutation's: PINEY_VOLUME=mutation")
        if not (os.path.exists(rs.ELF) and os.path.exists(rs.ISO)):
            raise unittest.SkipTest("the disc is not extracted")
        rs.build()
        cls.game = Game()

    def test_effect_lives(self):
        for (kind, n), (name, *_) in CREATES.items():
            with self.subTest(name, n=n):
                self.assertEqual(native_life(self.game, (kind, n)), eff_life(kind, n))

    def test_cases(self):
        for seed in range(6):
            rnd = random.Random(7100 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=9100):
    rs.build()
    game = Game()
    acts = set()
    for k in range(n):
        rnd = random.Random(seed0 + k)
        frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        acts |= {fr["act"][0] for fr in frames}
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print("acts reached:", sorted(acts))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 9100)
    else:
        unittest.main()
