#!/usr/bin/env python3
"""crates/piney-battle's Gorre (src/boss/gorre.rs, gorre/brother.rs) against
Outbreak's boss05.cpp run natively in the Rust VU0 machine (tools/eemu_rs.so),
frame by frame with battle_probe's `gorre`.

PINEY_VOLUME=outbreak is required: Gorre is fought in Outbreak, and the port
plays it with Outbreak's code (gcmn 0x00497010-0x0049e590, and the base
ccBoss under it, each of the two ccBoss05Brothers too).

A case lays a party of one to three members out on the command lists
(tools/test_battle_rs.py's scene), builds Gorre with the game's own
constructor (ccBoss05::ccBoss05) at BOSS - which builds both brothers in
turn, `operator new[]`'d - and runs ccBoss05::Main frame after frame (which
runs each brother's own Main under it), with ccBossEffManager::Draw's pass
before each, scripted affects between frames (Kite's on Gorre and on either
brother: hits, the drain's 13 and 21, heals; a member down), the menu's
type changing and the camera turning.
Compared each frame: Gorre's own acts, movement, place, target, HP, flags,
clips and pattern; each brother's the same, plus its own formation offset
(`m_posB`) and id; the boss camera's MaxRange, the effects alive (Gorre's
and both brothers', unordered, and their slots left out of the calls: the
game keeps one manager array the three share, the port keeps each its own),
the lists, the party's HP and hold, the calls
(sounds, flashes, cinema, stage, the cameras' changes, eyes and views,
skills, fly fonts, hit marks), the menu's words, newlib's rand() and
ccRand's index.

What is not the game's here:
- The party's affects are off (affectFunc 0): the boss's hits are compared
  by what EntryAffect stores and the rand() they draw.
- The files: GetCCSAdrs, GetChunkAdrsF, the clumps, lights and effect
  objects are stand-ins; ccAnm's SetAnm and _AnimateForward follow the
  clips' lengths (battle_probe `gorreclips`, from x51 and xeffect).
  DMY_center01 is the case's centre.
- The WaveShock, the FinalPhotonFlash and the dead effect are stand-ins
  with the port's lives (test_effect_lives checks those against the
  game's Create and Draw run natively); effSkillStart's controller is a
  stand-in whose endFlag comes after SKILL_START_LIFE.
- The cameras keep what is set on them; cameraGetRot answers the case's
  turn; checkCameraShakeRange answers by the case. The boss camera is a
  zeroed ccBossCam (SetMode and SetFreeCamPosView only note themselves,
  QuakeCam runs). W2P/P2W are identity and CollisionDetection finds
  nothing but in the case's push. A skill's request sets the caster's
  skillID and status 9 when cast through it (flag set), as the port's does.

    PINEY_VOLUME=outbreak python3 tools/test_battle_gorre_rs.py             the tests
    PINEY_VOLUME=outbreak python3 tools/test_battle_gorre_rs.py bulk N [S]  N cases from seed S
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

BOSS = 0x01300000               # the ccBoss05 (0x29410 bytes)
FAKE = 0x01340000               # stand-in streams, chunks, objects and effects
MGR = 0x01380000                # _g_bossEffManager (0x1038 bytes): shared by Gorre and both brothers
STAGE = 0x01390000              # the stage fader
WM = 0x013B0000                 # worldman and its event area
CAM = 0x013C0000                # the boss camera (a zeroed ccBossCam)
HEAP = 0x01600000               # operator new and new[] for the two brothers
PLW, PARTY = inf_va(0x007302E0), inf_va(0x00730310)
MENU = 0x01020000               # test_battle's ccMenu
BOSS_SIZE = 0x29410

# The brothers' scripted affects: the kind's EntryAffect type and p1.
BROTHER_AFFECTS = {5: (1, 1), 10: (1, 0), 6: (13, 0), 8: (7, 0), 9: (21, 0)}

# The stand-in effects' kinds (battle_probe's eff_num).
WAVE, DEAD, FINAL = 0, 5, 16
SKILL_START_LIFE = 8


def eff_life(kind):
    """The Draws piney_battle::boss::Eff::draw gives a stand-in, measured
    against the game's own Create and Draw run natively
    (test_effect_lives): the WaveShock (45), the dead effect (120), and
    ccBossEffFinalPhotonFlashCreate's (72: 1000.0 counted down by 100.0 a
    Draw, 11 calls to go under 0, then 61 more before m_bEnabled clears)."""
    return {WAVE: 45, DEAD: 120, FINAL: 72}[kind]


class Game:
    """The machine with Gorre's hooks."""

    def __init__(self):
        self.c = rs.Checks()
        g = self.c.game
        self.g, self.m = g, g.m
        self.clips = rs.ask(["gorreclips"])[0]
        self.names = {}         # a stand-in chunk or stream -> its name
        self.anms = {}          # a ccAnm -> [clip, time]
        self.effs = {}          # a stand-in effect -> [kind, count, life]
        self.starts = {}        # a stand-in skill start -> its count
        self.cams = {}          # camera -> [pos, view] as set
        self.out = []
        self.fake_at, self.heap_at = FAKE, HEAP
        self.rot_now, self.shake_now = 0, True
        self.center, self.push, self.frame_no = [0, 0, 0, F_ONE], [0, 0], 0
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
                     "Draw__5ccEffFPfUs", "SetFogBlend__9ccDrawEnvFfUi", "ResetFogBlend__9ccDrawEnvFv",
                     "SetType__6ccFontFi", "CamMain__9ccBossCamFv"):
            hook(name, nop)

        def collide(mm, hit, *_):
            if self.push[0] <= self.frame_no < self.push[1]:
                mm.mem[hit + 0x30:hit + 0x40] = bytes(16)
                return 1
            return 0
        hook("CollisionDetection__9ccCharHitFv", collide)

        def init_cam(mm, a, *_):
            mm.mem[CAM:CAM + 0x150] = bytes(0x150)
            # ccBossCam's constructor's MaxRenge (OUT gcmn 0x00472104).
            mm.store(CAM + 0xD8, 4, 0x447A0000)
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
        hook("EntryFlash__8ccScFadeFiiffff",
             lambda mm, f, t, c, *_: out.append(["flash", s32(t), c & 0xFFFFFFFF]) or 0)
        hook("ccSeOn3D__FiPf", lambda mm, se, p, *_: out.append(["se3d", s32(se)]) or 0)
        hook("ccSeOn__Fi", lambda mm, se, *_: out.append(["se", s32(se)]) or 0)
        hook("ccSeOn3DNote__FiPfc", lambda mm, se, p, n, *_: out.append(["se3dnote", s32(se), n & 0xFF]) or 0)
        hook("ccSeOnNote__Fic", lambda mm, se, n, *_: out.append(["senote", s32(se), n & 0xFF]) or 0)
        hook("EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T", lambda mm, *a: out.append(["afterimage"]) or 0)
        hook("OnCinemaMode__16ccBossEffManagerFi", lambda mm, a, n, *_: out.append(["cinema", s32(n)]) or 0)
        hook("OffCinemaMode__16ccBossEffManagerFv", lambda mm, *a: out.append(["cinema", -99]) or 0)
        # SwitchLayer is unnamed in Outbreak's build (0x00417db0, cited in
        # docs/engine/boss-fidchell.md's covers): OnThinkKerse calls it as
        # the 1st and 2nd unnamed callee, OnThinkSkill and OnThinkMagic as
        # the 1st and 3rd (the 2nd is the skill-name cinema helper,
        # 0x004733b0, not compared here).
        switch_layer = lambda mm, *a: out.append(["switch_layer"]) or 0  # noqa: E731
        for fn, idxs in (("OnThinkKerse__8ccBoss05Fv", (0, 1)), ("OnThinkSkill__8ccBoss05Fv", (0, 2)),
                         ("OnThinkMagic__8ccBoss05Fv", (0, 2))):
            for i in idxs:
                h[volume.callee(fn, i)] = switch_layer
        hook("ccEvVoiceRequest__Fii", nop)
        hook("ccEvVoiceStop__Fv", nop)
        hook("ccEntryFlyFontNew__FiiPfP6ccCharff",
             lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccHitMarkDisp__FP6ccCharP6ccChar", lambda mm, *a: out.append(["hitmark"]) or 0)
        # A brother's shield against the kind it resists: noted, no effect.
        hook("effResistantShield__FP6ccCharii", lambda mm, *a: out.append(["shield"]) or 0)
        hook("ccClearSpcCondition__Fv", lambda mm, *a: out.append(["clear_spc"]) or 0)
        hook("checkCameraShakeRange__FPf", lambda mm, *a: int(self.shake_now))
        hook("SetMode__9ccBossCamFifPfP6ccChar", lambda mm, a, mode, *_: out.append(["cammode", s32(mode)]) or 0)

        def free_cam(mm, a, p, v, *_):
            out.append(["freecam"] + [mm.load(p + 4 * k, 4) for k in range(3)]
                       + [mm.load(v + 4 * k, 4) for k in range(3)])
            return 0
        hook("SetFreeCamPosView__9ccBossCamFPfPf", free_cam)

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

        def cam_rot(mm, p, n, *_):
            for j, v in enumerate((0, 0, self.rot_now, 0)):
                mm.store(p + 4 * j, 4, v)
            return 0
        hook("cameraGetRot__FPfi", cam_rot)
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
        hook("ccBossEffFinalPhotonFlashCreate__FPf", effect(FINAL))
        hook("ccBossEffDeadCreate__FPfPfi", effect(DEAD))

        def skill_start(mm, ch, sid, *_):
            out.append(["skill_start", s32(sid)])
            e = self.alloc(0xC0)
            self.starts[e] = 0
            return e
        hook("effSkillStart__FP6ccChariii", skill_start)

        def skill(mm, cp, tp, sid, flag, *_):
            out.append(["skill", s32(sid), int(flag != 0)])
            if flag:
                mm.store(cp + 0x7C, 2, sid & 0xFFFF)
                mm.store(cp + 0x7E, 2, 9)
            return 0
        hook("ccItemSkillRequest__FP6ccCharP6ccCharii", skill)
        hook("ccItemSkillCompel__FP6ccCharP6ccCharii", skill)

    def effects_pass(self):
        """ccBossEffManager::Draw's loop, shared by Gorre and both
        brothers: a disabled effect freed, a stand-in's life counted, else
        run natively; then the skill starts' lives."""
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
        for e in self.starts:
            self.starts[e] += 1
            if self.starts[e] >= SKILL_START_LIFE:
                m.store(e + 0x60, 1, m.load(e + 0x60, 1) | 8)

    def effects(self):
        """Every alive (kind, enabled) pair in the shared manager, sorted:
        the port keeps Gorre's and each brother's own separate, so slot
        index cannot be compared, only which kinds are alive."""
        m = self.m
        pairs = []
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e and e in self.effs:
                pairs.append((self.effs[e][0], m.load(e, 1)))
        pairs.sort()
        return pairs


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    for p in party:
        if rnd.random() < 0.2:
            p.cond[rnd.choice((8, 9, 10, 11, 13, 14))] = rnd.randrange(1, 100)
    frames = rnd.randrange(800, 3000) if rnd.random() < 0.6 else rnd.randrange(3000, 7000)
    max_pp = c.data.bosses[4]["maxPP"]
    script = []
    rough = rnd.random() < 0.5
    # Gorre's own (0-4) and the brothers' (5 a hit with the normal attack's
    # skill, 10 with none, 6 a drain, 8 a heal, 9 the drain's 21 - after
    # which Gorre's gauge is theirs too, so no more pokes at it).
    # A quiet case (no hits on the brothers, the gauge untouched) runs the
    # Normal table through to its Talk.
    quiet = rnd.random() < 0.15
    reset = rnd.randrange(frames // 2, frames) if rnd.random() < 0.3 and not quiet else frames
    hard = rnd.random() < 0.3
    for f in sorted(rnd.sample(range(frames), min(frames // (10 if rough else 30), 300))):
        kinds = (0, 8) if quiet else (0, 0, 1, 4, 5, 5, 10, 8) if f < reset else (0, 5, 5, 10, 8)
        kind = 6 if rnd.random() < 0.02 and not quiet else rnd.choice(kinds)
        p0 = {0: rnd.randrange(-1, 900), 1: rnd.randrange(0, max_pp + 1),
              4: rnd.choice((0, 0, rnd.randrange(0, 300))), 5: rnd.randrange(0, 4000 if hard else 900),
              10: rnd.randrange(0, 900), 6: 0, 8: rnd.randrange(0, 500)}[kind]
        script.append((f, kind, rnd.randrange(2), p0))
    if reset < frames:
        script.append((reset, 9, rnd.randrange(2), 0))
    # A member down for a while now and then.
    if n > 1 and rnd.random() < 0.3:
        f = rnd.randrange(0, frames)
        who = rnd.randrange(1, n)
        script += [(f, 7, who, 1), (min(frames - 1, f + rnd.randrange(30, 600)), 7, who, 0)]
    if rnd.random() < 0.5:
        f = rnd.randrange(frames // 3, frames)
        script += [(f, 2, 0, 0), (f, 3, 0, 0)]
    script.sort(key=lambda s: s[0])
    push = [0, 0]
    if rnd.random() < 0.3:
        a = rnd.randrange(0, frames)
        push = [a, a + rnd.randrange(60, 250)]
    menus = []
    a = 0
    while a < frames and rnd.random() < 0.7:
        a += rnd.randrange(20, 300)
        menus += [(a, rnd.choice((0, 5, 72))), (a + rnd.randrange(1, 60), -1)]
        a = menus[-1][0]
    rots = [(0, fbits(rnd.uniform(-3.1, 3.1)))]
    f = 0
    while rnd.random() < 0.8:
        f += rnd.randrange(30, 900)
        if f >= frames:
            break
        rots.append((f, fbits(rnd.uniform(-3.1, 3.1))))
    center = [fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(-300, 300)), 0, F_ONE]
    cam2 = ([fbits(rnd.uniform(-900, 900)), fbits(rnd.uniform(-900, 900)), fbits(rnd.uniform(100, 600)), F_ONE],
            [fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(-300, 300)), fbits(rnd.uniform(0, 300)), F_ONE])
    return {"party": party, "frames": frames, "script": script, "menus": menus, "center": center, "rots": rots,
            "cam2": cam2, "shake": rnd.choice((0, 0, 2, 3, 7)), "push": push,
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}


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


def shake_at(case, f):
    s = case["shake"]
    return not (s > 0 and f % s == 0)


def rot_at(case, f):
    r = 0
    for k, v in case["rots"]:
        if k <= f:
            r = v
    return r


def br_of(m, k):
    return m.load(BOSS + 0x29350 + 4 * k, 4)


def run_game(game, case):
    c, g, m = game.c, game.g, game.m
    c.reset()
    c.scene.new_at = NEW
    game.fake_at = FAKE
    game.heap_at = HEAP
    game.effs.clear()
    game.anms.clear()
    game.starts.clear()
    game.out.clear()
    game.stage_on = False
    game.center = case["center"]
    game.push = case["push"]
    game.frame_no = 0
    game.shake_now = False
    game.rot_now = rot_at(case, 0)
    game.cams = {2: [list(case["cam2"][0]), list(case["cam2"][1])]}
    m.mem[BOSS:BOSS + BOSS_SIZE] = bytes(BOSS_SIZE)
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
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
    m.call(game.sym("__ct__8ccBoss05Fv"), (BOSS,))
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        game.frame_no = f
        game.shake_now = shake_at(case, f)
        game.rot_now = rot_at(case, f)
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
            elif kind == 7 and who < len(party):
                m.store(SCN + 0x1000 * who + 8, 2, p0 & 0xFFFF)
            elif kind in BROTHER_AFFECTS:
                ty, p1 = BROTHER_AFFECTS[kind]
                m.call(ea, (br_of(m, min(who, 1)), kite, ty, p0 & 0xFFFF, p1, 0))
        game.effects_pass()
        m.call(game.sym("Main__8ccBoss05Fv"), (BOSS,))
        frames.append(game_state(game, len(party)))
    return frames


def f32s(m, a, n):
    return [m.load(a + 4 * k, 4) for k in range(n)]


def char_fields(game, a, has_pat_mode):
    """A `ccBoss`/`ccChar`'s generic fields, the same shape Rust's
    `part_json` gives for Gorre and for each brother: `pat_mode` is
    Gorre's own member (ccBoss05+0x29408); for a brother that memory is
    the middle of `m_posB`, so it is not read - the port's own default
    (0, never touched) is used on both sides instead."""
    m = game.m
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    anm = ld(0xD4)
    clip = game.anms.get(anm, [None, 0])
    tgt = ld(0x78)
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "move": [ld(0x188), ld(0x18C)] + f32s(m, a + 0x190, 3),
        "tgt": [(tgt - SCN) // 0x1000 if tgt else -1, ld(0x180), ld(0x184)] + f32s(m, a + 0x160, 3),
        "pos": f32s(m, a + 0x40, 3),
        "posp": f32s(m, a + 0x50, 3),
        "dirc": f32s(m, a + 0x60, 3),
        "hp": [ld(0x70, 2, True), ld(0x74, 2, True)],
        # Through the character's own `param` (+4): the drain's 21 hands
        # both brothers Gorre's (SendMessage's SetBaseParam).
        "pp": [s16(m.load(ld(4) + 0x60, 2)), s16(m.load(ld(4) + 0x62, 2))],
        "flags": [ld(0x1D0, 1, True), ld(0x1D1, 1, True), ld(0x1D2, 1, True), ld(0x1D3, 1, True),
                  ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1D6, 1, True), ld(0x1D7, 1, True),
                  s32(ld(0x2BC)), s32(ld(0x29334)), int(ld(0x29338) != 0), ld(0x1D8, 1, True)],
        "tr": [ld(0x88), ld(0x8C)],
        "anm": [clip[0] or "", clip[1] >> 8, m.load(anm + 0x9C, 2)],
        "pat": [s32(ld(0x20C)), s32(ld(0x210)), s32(ld(0x29408)) if has_pat_mode else 0],
    }


def game_state(game, n):
    m = game.m
    gorre = char_fields(game, BOSS, True)
    brs = []
    for k in range(2):
        br = br_of(m, k)
        f = char_fields(game, br, False)
        f["posb"] = f32s(m, br + 0x29400, 3)
        f["id"] = s32(m.load(br + 0x2B4, 4))
        brs.append(f)
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    # ccBossCam's MaxRenge: the constructor's 1000, which Gorre's own
    # constructor leaves (Fidchell's sets 600 after InitBossCamera).
    gorre["maxrange"] = m.load(CAM + 0xD8, 4)
    gorre["eff"] = game.effects()
    gorre["party"] = party
    gorre["out"] = list(game.out)
    gorre["menu"] = [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                     s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2))]
    gorre["listed"] = [listed(game, a) for a in (BOSS, br_of(m, 0), br_of(m, 1))]
    gorre["rand"] = game.g.rand_now()
    gorre["cc"] = s32(m.load(eai.MTI, 4))
    gorre["br0"], gorre["br1"] = brs
    return gorre


def boss_char_pre(game):
    """Gorre's ccChar as SetBaseParam leaves it (bossTbl row 4)."""
    import types
    row = dict(game.c.data.bosses[4])
    ch = types.SimpleNamespace()
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level, ch.exp = 4, row.get("level", 1), 0
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
    """The probe's `gorre` request."""
    b = game.c.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    kite = party[0]
    parts = (["gorre", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + [0, 0, 0, 0]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], case["shake"]]
             + case["push"] + list(case["cam2"][0]) + list(case["cam2"][1]))
    parts.append(len(case["rots"]))
    for r in case["rots"]:
        parts += list(r)
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
    if isinstance(a, dict) and isinstance(b_, dict):
        out = []
        for k in a:
            out += diffs(a[k], b_.get(k), f"{path}.{k}")
        return out
    return [] if a == b_ else [f"{path}: game {a} port {b_}"]


def compare(game, case, label):
    pre = boss_char_pre(game)
    g_frames = run_game(game, case)
    p_frames = rs.ask([request(game, case, pre)])[0]
    for f, (a, b_) in enumerate(zip(g_frames, p_frames)):
        a = rs.norm(a)
        # An effect's slot in the manager: one array for the three in the
        # game, each its own in the port (as "eff" above), so not compared.
        for side in (a, b_):
            side["out"] = [o[:2] if o and o[0] == "eff" else o for o in side["out"]]
        for k in a:
            if a[k] != b_.get(k):
                d = "\n  ".join(diffs(a[k], b_.get(k))[:12])
                raise AssertionError(f"{label} frame {f - 1}: {k}\n  {d}\n act {a['act']} port {b_['act']}\n"
                                     f" out {a['out']}\n port {b_['out']}")
    if len(g_frames) != len(p_frames):
        raise AssertionError(f"{label}: {len(g_frames)} game frames, {len(p_frames)} port frames")
    return g_frames


CREATES = [
    ("ccBossEffWaveShockCreate__FPfPff", "pd", [1.0], (), WAVE),
    ("ccBossEffFinalPhotonFlashCreate__FPf", "p", [], (), FINAL),
    ("ccBossEffDeadCreate__FPfPfi", "pp", [], (0,), DEAD),
]


def native_life(game, create):
    """The Draws the game's effect runs until it clears m_bEnabled: its
    Create run natively (the stand-in unhooked), then the Draw of what it
    put in the manager, frame after frame."""
    m, sym = game.m, game.sym
    name, ptrs, floats, ints, _ = create
    game.fake_at = FAKE
    game.heap_at = HEAP
    m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
    m.store(sym("_g_bossEffManager"), 4, MGR)
    m.store(sym("scFadeDef"), 4, STAGE + 0x200)

    def vec(*v):
        a = game.alloc(16)
        for k, x in enumerate(v):
            m.store(a + 4 * k, 4, fbits(x))
        return a
    vecs = {"p": vec(100.0, -200.0, 0.0, 1.0), "d": vec(0.0, 0.0, 0.5, 0.0)}
    args = tuple(vecs[c] for c in ptrs) + ints
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
        if volume.NAME != "outbreak":
            raise unittest.SkipTest("Gorre is Outbreak's: PINEY_VOLUME=outbreak")
        if not (os.path.exists(rs.ELF) and os.path.exists(rs.ISO)):
            raise unittest.SkipTest("the disc is not extracted")
        rs.build()
        cls.game = Game()

    def test_effect_lives(self):
        for create in CREATES:
            kind = create[4]
            with self.subTest(create[0]):
                self.assertEqual(native_life(self.game, create), eff_life(kind))

    def test_cases(self):
        for seed in range(6):
            rnd = random.Random(8500 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=9900):
    rs.build()
    game = Game()
    acts = set()
    total = bad = 0
    for k in range(n):
        rnd = random.Random(seed0 + k)
        try:
            frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        except AssertionError as e:
            bad += 1
            print(str(e) if bad == 1 else str(e).splitlines()[0], flush=True)
            continue
        acts |= {fr["act"][0] for fr in frames}
        total += len(frames) - 1
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print(f"{n} cases, {bad} mismatched, {total} frames matched; acts reached:", sorted(acts))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 9900)
    else:
        unittest.main()
