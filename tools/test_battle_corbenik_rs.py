#!/usr/bin/env python3
"""crates/piney-battle's Corbenik, first form (src/boss/corbenik/first.rs,
seed.rs, eff.rs) against Quarantine's boss08.cpp run natively in the Rust
VU0 machine (tools/eemu_rs.so), frame by frame with battle_probe's
`corbenik`.

PINEY_VOLUME=quarantine is required: Corbenik is fought in Quarantine, and
the port plays it with Quarantine's code (gcmn 0x003acbd0-0x003b1a40, its
effects at 0x00376440-0x0037dec0, and the base ccBoss under it).

A case lays a party of one to three members out on the command lists
(tools/test_battle_rs.py's scene), builds Corbenik with the game's own
constructor (ccBoss08::ccBoss08, its eight seeds inside it) at BOSS and
runs ccBoss08::Main frame after frame, with ccBossEffManager::Draw's pass
before each, scripted affects between frames (Kite's hits of types 1 and 3
on the body and on the seeds, a skill's or a spell's, the stop's affects,
a member down, now and then an act forced), and the menu's type changing.
The land under the seeds ends past the case's x. Compared each frame: the
acts, movement, place, target, HP, flags, clips, Corbenik's own members
(the dead effect, the stage, the counts, the leap, the shake), each seed's
(acts, place, throw, HP, flags, lists, fade, clip, count, blink, spin, the
land, the burst, the charge, the flush), the effects' slots, the party's
HP and hold, the calls, the menu's words, the active camera's type, the
blur, newlib's rand() and ccRand's index.

What is not the game's here:
- The party's affects are off (affectFunc 0): the boss's hits are compared
  by what EntryAffect stores and the rand() they draw.
- The files: GetCCSAdrs, GetChunkAdrsF, the clumps and effect objects are
  stand-ins; ccAnm's SetAnm and _AnimateForward follow the clips' lengths
  (battle_probe `corbenikclips`, from x81 and xeffect). DMY_center01 is the
  case's centre.
- The WaveShock and the dead effect are stand-ins with the port's lives;
  the wave and the flush run natively. The particles' generators are
  stand-ins that only note themselves (ccCheckParticleGenerator: any made
  one lives); the burst, the shield, the after-images and the count's font
  only note themselves.
- The cameras keep what is set on them (changeCamera sets camID);
  activeCamPtr is a stand-in whose type is compared. No boss camera:
  cmn_bossCam stays 0. W2P/P2W are identity and CollisionDetection finds
  nothing. ccHitCheckLM finds the plane z 0 short of the case's x.

    PINEY_VOLUME=quarantine python3 tools/test_battle_corbenik_rs.py             the tests
    PINEY_VOLUME=quarantine python3 tools/test_battle_corbenik_rs.py bulk N [S]  N cases from seed S
"""

import os
import random
import struct
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
import test_battle_boss_rs as bb  # noqa: E402
from test_battle_boss_rs import forward_clip, rnd_member, s16, s32  # noqa: E402
from test_battle_rs import F_ONE, NEW, SCN, fbits, ser_char  # noqa: E402

BOSS = 0x01800000               # the ccBoss08 (0x1732c0 bytes, its seeds inside)
FAKE = 0x01A00000               # stand-in streams, chunks, objects and effects
MGR = 0x01A80000                # _g_bossEffManager (0x1038 bytes)
STAGE = 0x01A90000              # the stage fader
WM = 0x01AB0000                 # worldman and its event area
ACAM = 0x01AC0000               # activeCamPtr's stand-in
FONT = 0x01AC1000               # font's
HEAP = 0x01B00000               # operator new for the wave, the flush and the clips
PLW, PARTY = inf_va(0x007302E0), inf_va(0x00730310)
MENU = 0x01020000               # test_battle's ccMenu
BOSS_SIZE = 0x1732C0
SEED0, SEED_SIZE = 0x29350, 0x293D0

# The stand-in effects' kinds (battle_probe's eff_num) and the natives'.
WAVE, DEAD = 0, 5
CWAVE, FLUSH = 26, 27


def seed_at(k):
    return BOSS + SEED0 + SEED_SIZE * k


def eff_life(kind, a=None):
    """The Draws piney_battle::boss::Eff::draw gives a stand-in."""
    return {WAVE: 45, DEAD: 120}[kind]


class Game:
    """The machine with Corbenik's hooks."""

    def __init__(self):
        self.c = rs.Checks()
        g = self.c.game
        self.g, self.m = g, g.m
        self.clips = rs.ask(["corbenikclips"])[0]
        self.names = {}         # a stand-in chunk or stream -> its name
        self.anms = {}          # a ccAnm -> [clip, time]
        self.effs = {}          # a stand-in effect -> [kind, count, life]
        self.cams = {}          # camera -> [pos, view] as set
        self.out = []
        self.fake_at, self.heap_at = FAKE, HEAP
        self.center, self.hole = [0, 0, 0, F_ONE], 1e30
        sym = self.sym
        self.gens = {sym("MagicSquareGenerator") + 0x188: "ring", sym("MagicSquareGenerator"): "square",
                     sym("g_seedShotSmokeGen"): "smoke", sym("g_energyChargeGenerator"): "charge"}
        self.hooks()
        bb.install_blur(self)
        bb.install_draw(self)

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

        def new(mm, n, *_):
            a = self.heap_at
            self.heap_at += (n + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            return a
        hook("__nw__FUi", new)
        hook("__nwa__FUi", new)
        hook("GetCCSAdrs__8ccStreamFPCc", lambda mm, n, *_: self.alloc(16, self.cstr(n)))

        def chunk(mm, s, n, *_):
            name = self.cstr(n)
            a = self.alloc(32, name)
            if name == "DMY_center01":
                for k in range(4):
                    mm.store(a + 16 + 4 * k, 4, self.center[k])
            return a
        hook("GetChunkAdrsF__8ccStreamFPCci", chunk)
        for name in ("__ct__7ccCoordFv", "__ct__7ccLightFsSc", "__ct__16ccDrawPacketCtrlFv",
                     "__ct__15ccBufferReverceFv"):
            hook(name, this)
        for name in ("Init__7ccClumpFP12ccClumpChunk", "ChangeClut__7ccClumpFP11ccClutChunkP11ccClutChunk",
                     "Init__11ccOmniLightFP12ccLightChunk", "AddGrp__10ccLightGrpFP7ccLight",
                     "DelGrp__10ccLightGrpFP7ccLight", "__dt__11ccOmniLightFv", "ccSetColor__FPfUif",
                     "__dla__FPv", "__dl__FPv", "__dt__5ccAnmFv", "__dt__7ccLayerFv",
                     "MakePacket__15ccBufferReverceFv", "SetRenderState__5ccAnmF20CC_RENDER_STATE_TYPEi"):
            hook(name, nop)

        def eff_init(mm, e, *_):
            mm.store(e + 0x30, 4, 1)
            return 0
        hook("Init__5ccEffFP10ccEffChunki", eff_init)

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
            ch = self.alloc(32, name)
            m.store(ch + 0x10, 4, self.clips.get(name, [0, 0])[0])
            m.store(a + 0xA8, 4, ch)
            m.store(a + 0x94, 4, ch)

        hook("SetAnm__5ccAnmFP10ccAnmChunkUi", lambda mm, a, ch, *_: set_anm(a, self.names.get(ch)) or 0)
        hook("SetAnm__5ccAnmFP8ccStreamPcUi", lambda mm, a, s, n, *_: set_anm(a, self.cstr(n)) or 0)

        def forward(mm, a, step, *_):
            clip, time = self.anms.get(a, [None, 0])
            frames, looping = self.clips.get(clip, [0, 0])
            new_t, ended = forward_clip(frames, looping, time, step & 0xFFFF)
            self.anms[a] = [clip, new_t]
            mm.store(a + 0x98, 4, new_t >> 8)
            return int(ended)
        hook("_AnimateForward__5ccAnmFUi", forward)
        for name in ("SetMatrix_PosRotZYX__7ccCoordFPfPf", "Draw__5ccAnmFv", "Draw__6ccBossFv",
                     "DrawAfterImages__6ccBossFv", "DrawParts__6ccBossFP5ccAnmi", "DrawStageEffect__6ccBossFv",
                     "InitAfterImage__6ccBossFP5ccAnmi", "SetMatrix_PosRotXYZScale__7ccCoordFPfPfPf",
                     "SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf", "Draw__7ccClumpFv", "SetTransparency__7ccClumpFf",
                     "__dt__7ccClumpFv", "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff",
                     "ccSqFade__FiUsiUc", "SetHitSW__9ccCharHitFi", "SetActiveLayer__9WORLD_MANFi",
                     "Draw__5ccEffFPfUs", "ResetFogBlend__9ccDrawEnvFv",
                     "SetType__6ccFontFi", "CamMain__9ccBossCamFv"):
            hook(name, nop)
        hook("CollisionDetection__9ccCharHitFv", nop)
        hook("GetSubstAdrsF__5ccAnmFPCcb", lambda mm, anm, n, *_: self.alloc(0xA0, self.cstr(n)))

        def land(mm, sp, ep, mask, *_):
            z = mm.load(ep + 8, 4)
            below = (z & 0x7FFFFFFF) == 0 or z >> 31 == 1
            x = struct.unpack("<f", struct.pack("<I", mm.load(ep, 4)))[0]
            mm.f[0] = 0 if below and x <= self.hole else 0xBF800000
            return 0
        hook("ccHitCheckLM__FPfPfUi", land)

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

        def flash3(mm, f, t_in, t_out, hold, *_):
            out.append(["flash3", s32(t_in), s32(t_out), s32(hold), mm.r[8] & 0xFFFFFFFF])
            return 0
        hook("EntryFlash3__8ccScFadeFiiiiffff", flash3)
        hook("ccSeOn3D__FiPf", lambda mm, se, p, *_: out.append(["se3d", s32(se)]) or 0)
        hook("ccSeOn__Fi", lambda mm, se, *_: out.append(["se", s32(se)]) or 0)
        hook("ccSeOn3DNote__FiPfc", lambda mm, se, p, n, *_: out.append(["se3dnote", s32(se), n & 0xFF]) or 0)
        hook("ccSeOnNote__Fic", lambda mm, se, n, *_: out.append(["senote", s32(se), n & 0xFF]) or 0)
        hook("EntryAfterImageAnm__6ccBossFP5ccAnmPfPfP17ENTRYAFTERIMAGE_T",
             lambda mm, *a: out.append(["afterimage_anm"]) or 0)
        hook("OnCinemaMode__16ccBossEffManagerFi", lambda mm, a, n, *_: out.append(["cinema", s32(n)]) or 0)
        hook("OffCinemaMode__16ccBossEffManagerFv", lambda mm, *a: out.append(["cinema", -99]) or 0)
        # The arena's layer and the skill's cinema (unnamed in Quarantine).
        h[volume.callee("OnThinkMagic__8ccBoss07Fv", 0)] = lambda mm, *a: out.append(["switch_layer"]) or 0
        h[volume.callee("OnThinkMagic__8ccBoss07Fv", 1)] = \
            lambda mm, a, sid, *_: out.append(["cinema_skill", s32(sid)]) or 0
        hook("effResistantShield__FP6ccCharii", lambda mm, ch, magic, *_: out.append(["shield", s32(magic)]) or 0)
        hook("effExplode3__FPfPffi", lambda mm, p, r, kind, *_: out.append(["burst", s32(kind)]) or 0)
        hook("SetFogBlend__9ccDrawEnvFfUi", bb.own_fog(out))

        def gen(mm, a, param, *_):
            out.append(["gen", self.gens.get(param, hex(param))])
            return a
        hook("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParam"
             "P25ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam", gen)
        hook("ParticleDelete__19ccParticleGeneratorFv", lambda mm, *a: out.append(["gen_del"]) or 0)
        hook("ccCheckParticleGenerator__FP19ccParticleGenerator", lambda mm, g, *_: int(g != 0))
        for name in ("startParticleGenerator__FP19ccParticleGenerator",
                     "SetRenderState__5ccEffF20CC_RENDER_STATE_TYPEi", "SetScaleAnm__15ccAnimateObjectFfff"):
            hook(name, nop)
        hook("ccCalcTagPos__FRA4_fRA4_iPf", lambda mm, *a: 1)
        hook("MakeSignedNum__6ccFontFil", lambda mm, f, k, n, *_: out.append(["count", s32(n)]) or 0)
        hook("ccEntryFlyFontNew__FiiPfP6ccCharff",
             lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccHitMarkDisp__FP6ccCharP6ccChar", lambda mm, *a: out.append(["hitmark"]) or 0)
        hook("ccClearSpcCondition__Fv", lambda mm, *a: out.append(["clear_spc"]) or 0)
        hook("checkCameraShakeRange__FPf", lambda mm, *a: 1)

        def change_camera(mm, n, *_):
            out.append(["cam", s32(n)])
            mm.store(sym("camID"), 2, n & 0xFFFF)
            return 0
        hook("changeCamera__Fi", change_camera)

        def cam_set(key, k):
            def put(mm, p, n, *_):
                v = [mm.load(p + 4 * j, 4) for j in range(4)]
                out.append([key, s32(n)] + v[:3])
                self.cams.setdefault(s32(n), [[0, 0, 0, F_ONE], [0, 0, 0, F_ONE]])[k] = v
                return 0
            return put
        hook("cameraSetPos__FPfi", cam_set("campos", 0))
        hook("cameraSetView__FPfi", cam_set("camview", 1))
        hook("cameraSetRot__FPfi", nop)

        def cam_get(k):
            def get(mm, p, n, *_):
                v = self.cams.get(s32(n), [[0, 0, 0, F_ONE], [0, 0, 0, F_ONE]])[k]
                for j in range(4):
                    mm.store(p + 4 * j, 4, v[j])
                return 0
            return get
        hook("cameraGetPos__FPfi", cam_get(0))
        hook("cameraGetView__FPfi", cam_get(1))
        hook("ccTransPosW2P__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)
        hook("ccTransPosP2W__FPfPf", lambda mm, o, i, *_: mm.mem.__setitem__(slice(o, o + 16), mm.mem[i:i + 16]) or 0)

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

        def skill(mm, cp, tp, sid, flag, *_):
            out.append(["skill", s32(sid), int(flag != 0)])
            if flag:
                mm.store(cp + 0x7C, 2, sid & 0xFFFF)
                mm.store(cp + 0x7E, 2, 9)
                mm.store(cp + 0x78, 4, tp)
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
        bb.blur_main(self, MGR)

    def effects(self):
        m = self.m
        out = []
        vt = {self.sym("__vt__13ccBossEffWave"): CWAVE,
              self.sym("__vt__14ccBossEffFlush"): FLUSH}
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                kind = self.effs[e][0] if e in self.effs else vt.get(m.load(e + 12, 4), -1)
                out.append([k, kind, m.load(e, 1)])
        return out


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    for p in party:
        if rnd.random() < 0.2:
            p.cond[rnd.choice((8, 9, 10, 11, 13, 14))] = rnd.randrange(1, 100)
    frames = rnd.randrange(800, 3000) if rnd.random() < 0.6 else rnd.randrange(3000, 6000)
    script = []
    rough = rnd.random() < 0.5
    nsk = len(c.data.skills)
    for f in sorted(rnd.sample(range(frames), min(frames // (8 if rough else 25), 400))):
        kind = rnd.choice((0, 0, 9, 10, 11, 11, 11, 12, 12, 13, 5, 6))
        who = 0
        if kind == 10:
            who = rnd.randrange(1, nsk)
        elif kind in (11, 12):
            who = rnd.randrange(8) + 8 * rnd.choice((0, 0, rnd.randrange(1, min(nsk, 4000))))
        p0 = rnd.randrange(-1, 900) if rnd.random() < 0.85 else rnd.randrange(900, 4000)
        if kind in (5, 6, 13):
            p0 = 0
        script.append((f, kind, who, p0))
    if n > 1 and rnd.random() < 0.3:
        f = rnd.randrange(0, frames)
        who = rnd.randrange(1, n)
        script += [(f, 7, who, 1), (min(frames - 1, f + rnd.randrange(30, 600)), 7, who, 0)]
    # Acts forced now and then, from where no pattern would start them, and
    # ones no table holds (not the flash, pollen and body press, 3-5: the
    # port leaves those out, nothing in the game entering them).
    if rnd.random() < 0.4:
        for _ in range(rnd.randrange(1, 3)):
            act = rnd.choice((6, 7, 12, 14, 16, 17, 18, 20, 21, 22))
            script.append((rnd.randrange(0, frames), 8, 0, act))
    script.sort(key=lambda s: s[0])
    menus = []
    a = 0
    while a < frames and rnd.random() < 0.7:
        a += rnd.randrange(20, 300)
        menus += [(a, rnd.choice((0, 5, 74))), (a + rnd.randrange(1, 60), -1)]
        a = menus[-1][0]
    center = [fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(-300, 300)), 0, F_ONE]
    cam2 = ([fbits(rnd.uniform(-900, 900)), fbits(rnd.uniform(-900, 900)), fbits(rnd.uniform(100, 600)), F_ONE],
            [fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(0, 300)), F_ONE])
    hole = fbits(rnd.uniform(-600, 900)) if rnd.random() < 0.4 else fbits(1e30)
    case = {"party": party, "frames": frames, "script": script, "menus": menus, "center": center,
            "cam2": cam2, "hole": hole,
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}
    case.update(bb.blur_case(rnd))
    case.update(bb.draw_case(rnd))
    return case


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
    game.heap_at = HEAP
    game.effs.clear()
    game.anms.clear()
    game.out.clear()
    game.stage_on = False
    game.center = case["center"]
    game.hole = struct.unpack("<f", struct.pack("<I", case["hole"]))[0]
    game.cams = {2: [list(case["cam2"][0]), list(case["cam2"][1])]}
    m.store(game.sym("camID"), 2, 1)
    bb.put_draw_camera(game, case, ACAM)
    m.mem[FONT:FONT + 0x100] = bytes(0x100)
    m.store(game.sym("font"), 4, FONT)
    m.store(game.sym("cmn_bossCam__10ccIBossEff"), 4, 0)
    m.mem[BOSS:BOSS + BOSS_SIZE] = bytes(BOSS_SIZE)
    bb.make_manager(game, MGR, case)
    m.store(game.sym("_g_bossEffManager"), 4, MGR)
    m.store(game.sym("scFadeDef"), 4, STAGE + 0x200)
    m.store(game.sym("worldman"), 4, WM)
    m.store(WM + 1092, 4, WM + 0x800)
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
    m.call(game.sym("__ct__8ccBoss08Fv"), (BOSS,))
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        for sf, kind, who, p0 in case["script"]:
            if sf != f:
                continue
            seed = seed_at(who % 8)
            sid = (who // 8) & 0xFFFF
            if kind == 0:
                m.call(ea, (BOSS, kite, 1, p0 & 0xFFFF, 0, 0))
            elif kind in (5, 6):
                m.call(ea, (BOSS, kite, kind, 0, 0, 0))
            elif kind == 7 and who < len(party):
                m.store(SCN + 0x1000 * who + 8, 2, p0 & 0xFFFF)
            elif kind == 8:
                m.store(BOSS + 0x1A2, 2, p0 & 0xFFFF)
                m.store(BOSS + 0x1A4, 2, 0)
                m.store(BOSS + 0x1A6, 2, 0)
            elif kind == 9:
                m.call(ea, (BOSS, kite, 3, p0 & 0xFFFF, 0, 0))
            elif kind == 10:
                m.call(ea, (BOSS, kite, 1, p0 & 0xFFFF, who & 0xFFFF, 0))
            elif kind == 11:
                m.call(ea, (seed, kite, 1, p0 & 0xFFFF, sid, 0))
            elif kind == 12:
                m.call(ea, (seed, kite, 3, p0 & 0xFFFF, sid, 0))
            elif kind == 13:
                m.store(BOSS + 0x7C, 2, 0)
                m.store(BOSS + 0x7E, 2, 0)
        game.effects_pass()
        m.call(game.sym("Main__8ccBoss08Fv"), (BOSS,))
        frames.append(game_state(game, len(party)))
    return frames


def f32s(m, a, n):
    return [m.load(a + 4 * k, 4) for k in range(n)]


def seed_state(game, k):
    m = game.m
    a = seed_at(k)
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    clip = game.anms.get(ld(0xD4), [None, 0])
    return ([ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)] + f32s(m, a + 0x40, 3)
            + f32s(m, a + 0x50, 3) + f32s(m, a + 0x60, 3) + f32s(m, a + 0x190, 3)
            + [ld(0x70, 2, True), ld(0x74, 2, True), ld(0x1D0, 1, True), ld(0x1D1, 1, True), ld(0x1D4, 1, True),
               ld(0x1D5, 1, True), ld(0x1D6, 1, True), s32(ld(0x2BC)), listed(game, a), ld(8, 2, True),
               ld(0x88), ld(0x8C), clip[0] or "", clip[1] >> 8, ld(0x293B4, 4, True), ld(0x293B8, 4, True),
               ld(0x293BC, 2), ld(0x293C0, 2), int(ld(0x293C4) != 0), int(ld(0x293C8) != 0),
               int(ld(0x29350) != 0), int(ld(0x293B0) != 0)])


def game_state(game, n):
    m = game.m
    a = BOSS
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    anm = ld(0xD4)
    clip = game.anms.get(anm, [None, 0])
    tgt = ld(0x78)
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    first = ([int(ld(0x1731D4) != 0), ld(0x1731D8, 4, True), ld(0x1731E4, 4, True), ld(0x1732A8, 4, True),
              int(ld(0x1732AC) != 0), int(ld(0x1732B0) != 0), ld(0x1732B4, 4, True)]
             + f32s(m, a + 0x173260, 3) + f32s(m, a + 0x173270, 3) + f32s(m, a + 0x173280, 3)
             + [ld(0x173290), ld(0x1B8), ld(0x29328, 4, True), ld(0x2932C, 4, True), ld(0x20C, 4, True),
                ld(0x210, 4, True), ld(0x1C8)])
    blur = bb.blur_state(game, MGR)
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "move": [ld(0x188), ld(0x18C)] + f32s(m, a + 0x190, 3),
        "tgt": [n if tgt == BOSS else (tgt - SCN) // 0x1000 if tgt else -1, ld(0x180), ld(0x184)]
        + f32s(m, a + 0x160, 3),
        "pos": f32s(m, a + 0x40, 3),
        "posp": f32s(m, a + 0x50, 3),
        "dirc": f32s(m, a + 0x60, 3),
        "hp": [ld(0x70, 2, True), ld(0x74, 2, True)],
        "flags": [ld(0x1D0, 1, True), ld(0x1D1, 1, True), ld(0x1D2, 1, True), ld(0x1D3, 1, True),
                  ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1D6, 1, True), ld(0x1D7, 1, True),
                  s32(ld(0x2BC)), s32(ld(0x29334)), int(ld(0x29338) != 0), ld(0x1A0, 2, True), listed(game, a)],
        "tr": [ld(0x88), ld(0x8C)],
        "anm": [clip[0] or "", clip[1] >> 8, m.load(anm + 0x9C, 2)],
        "first": first,
        "seeds": [seed_state(game, k) for k in range(8)],
        "cam": [s32(m.load(ACAM + 0x5C, 4))],
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2)),
                 s16(m.load(MENU + 0x0C, 2))],
        "blur": blur,
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def boss_char_pre(game):
    """Corbenik's ccChar as SetBaseParam leaves it (bossTbl row 7)."""
    import types
    row = dict(game.c.data.bosses[7])
    ch = types.SimpleNamespace()
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level, ch.exp = 7, row.get("level", 1), 0
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
    """The probe's `corbenik` request."""
    b = game.c.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    kite = party[0]
    parts = (["corbenik", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + [0, 0, 0, 0]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], case["hole"]]
             + list(case["cam2"][0]) + list(case["cam2"][1]))
    parts.append(len(case["script"]))
    for s in case["script"]:
        parts += list(s)
    parts.append(len(case["menus"]))
    for mm in case["menus"]:
        parts += list(mm)
    parts += bb.blur_tokens(case)
    parts += bb.draw_tokens(case)
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


class Against(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if volume.NAME != "quarantine":
            raise unittest.SkipTest("Corbenik is Quarantine's: PINEY_VOLUME=quarantine")
        if not (os.path.exists(rs.ELF) and os.path.exists(rs.ISO)):
            raise unittest.SkipTest("the disc is not extracted")
        rs.build()
        cls.game = Game()

    def test_cases(self):
        for seed in range(6):
            rnd = random.Random(8800 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=9900):
    rs.build()
    game = Game()
    acts = set()
    seed_acts = set()
    off_land = 0
    total = 0
    for k in range(n):
        rnd = random.Random(seed0 + k)
        frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        acts |= {fr["act"][0] for fr in frames}
        seed_acts |= {s[0] for fr in frames for s in fr["seeds"]}
        off_land += sum(1 for fr in frames for s in fr["seeds"] if s[33])
        total += len(frames) - 1
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print(f"{n} cases, {total} frames; acts reached:", sorted(acts), "seeds':", sorted(seed_acts),
          f"seed-frames off the land: {off_land}")


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 9900)
    else:
        unittest.main()
