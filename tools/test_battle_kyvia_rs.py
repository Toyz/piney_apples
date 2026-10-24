#!/usr/bin/env python3
"""crates/piney-battle's Kyvia (src/boss/kyvia.rs, kyvia/core.rs, gomora.rs,
thunder.rs, magic_ball.rs, fourth.rs, hell.rs) against the game's
kyvia01.cpp to kyvia04.cpp with kyviacore.cpp and kyviagomora.cpp run
natively (tools/eemu_rs.so), frame by frame with battle_probe's `kyvia`:
Mutation's first fight (PINEY_VOLUME=mutation), Outbreak's second
(PINEY_VOLUME=outbreak; PINEY_KYVIA_FIGHT=1 runs Outbreak's copy of the
first) or Quarantine's third (PINEY_VOLUME=quarantine; PINEY_KYVIA_FIGHT=4
its fourth). What is compared and what is stood in for:
docs/engine/boss-kyvia.md "Checks".

    PINEY_VOLUME=mutation python3 tools/test_battle_kyvia_rs.py             the tests
    PINEY_VOLUME=outbreak python3 tools/test_battle_kyvia_rs.py bulk N [S]  N cases from seed S
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
import test_battle_boss_rs as bb  # noqa: E402
from test_battle_boss_rs import forward_clip, rnd_member, s16, s32  # noqa: E402
from test_battle_rs import F_ONE, NEW, SCN, fbits, ser_char  # noqa: E402

BOSS = 0x01300000               # the ccBossKyvia01 (0x296a0 bytes) or 02 (0x29740)
FAKE = 0x01340000               # stand-in streams, chunks and effects
MGR = 0x01380000                # _g_bossEffManager (0x1038 bytes)
STAGE = 0x01390000              # the stage fader
CAM = 0x013A0000                # the boss camera (Mutation's ccBossCam)
WM = 0x013B0000                 # worldman, its event area and the area's stream
SLAVES = 0x01400000             # operator new[]: the core, the gomoras, the meteors' arrays
PLW, PARTY = inf_va(0x007302E0), inf_va(0x00730310)
MENU = 0x01020000               # test_battle's ccMenu
CORE_SIZE = 0x295A0
GOMORA_SIZE = 0x295F0
AREA = WM + 0x2000              # the EVENTAREAB8
AREA_CCS = WM + 0x4000
# The boss task's own stack (its thread's): what its frames leave there is
# what the next frame's calls read (the fourth fight reads two unwritten
# words of it); everything else the harness calls runs on eemu's.
BOSS_STACK = eemu.STACK_TOP - 0x80000

# The particle tables by symbol, rows 0x38 bytes.
# Outbreak's names the sidecar lacks: the two Mains (vtable +0x1c), the
# disc's Move and IsMove (called from CheckDiscMove) and KyviaGenerator
# (0x690 below KyviaDmgGenerator, as in Mutation).
# Quarantine's likewise: ccBossKyvia03's Main (its vtable, 0x00277f00,
# +0x1c), the disc's Move and IsMove (CheckDiscMove's two calls into
# EVENTAREAB8) and KyviaGenerator (0x690 below KyviaDmgGenerator).
UNNAMED = {"outbreak": {"Main__13ccBossKyvia01Fv": 0x004D0780, "Main__13ccBossKyvia02Fv": 0x004D7300,
                        "IsMove__11EVENTAREAB8Fv": 0x00419D10, "Move__11EVENTAREAB8Fv": 0x00419C80,
                        "KyviaGenerator": 0x0061D0D0},
           "quarantine": {"Main__13ccBossKyvia03Fv": 0x003D24D0, "IsMove__11EVENTAREAB8Fv": 0x0030C610,
                          "Move__11EVENTAREAB8Fv": 0x0030C580, "KyviaGenerator": 0x00514450,
                          "__ct__13ccBossKyvia04Fi": 0x003D9C40, "Main__13ccBossKyvia04Fv": 0x003DB950,
                          "HitOff__11EVENTAREAB8Fi": 0x0030C620, "HitOn__11EVENTAREAB8Fi": 0x0030C690}}
FIGHT = int(os.environ.get("PINEY_KYVIA_FIGHT")
            or {"outbreak": 2, "quarantine": 3}.get(volume.NAME, 1))

TABLES = ("KyviaGenerator", "KyviaDmgGenerator", "CoreGenerator", "GomoraGenerator", "GomoraAuraGenerator",
          "MissileSmokeGenerator", "BurstSmokeGenerator")


class Game:
    """The machine with Kyvia's hooks."""

    def __init__(self):
        self.c = rs.Checks()
        g = self.c.game
        self.g, self.m = g, g.m
        self.clips = rs.ask([f"kyviaclips {FIGHT}"])[0]
        self.names = {}         # a stand-in chunk or stream -> its name
        self.anms = {}          # a ccAnm -> [clip, time]
        self.gens = {}          # a particle generator -> (param, first force field)
        self.out = []
        self.moving = 0
        self.ride = 0           # frames the disc rides on after Move
        self.frame = {"rot": 0, "pos": [0, 0, 0, F_ONE], "view": [0, 0, 0, F_ONE]}
        self.hooks()
        bb.install_blur(self)
        bb.install_draw(self)

    def sym(self, name):
        s = self.g.prog.symbol_named(name)
        return s.value if s is not None else UNNAMED[volume.NAME][name]

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

    def member(self, mm, ptr, kind):
        """Who keeps generator `ptr` ("held"), or whose vector `ptr` is
        ("sync"), as battle_probe names them; None for no one."""
        parts = [(BOSS, "body")]
        core = core_of(mm)
        if core:
            parts.append((core, "core"))
            parts += [(gomora_of(mm, k), ["gomora", k]) for k in range(5)]
        if kind == "held":
            for off, name in held_slots().items():
                if mm.load(BOSS + off, 4) == ptr:
                    return name
            if FIGHT == 4:
                for k in range(4):
                    for j, name in enumerate(("wall", "wall2", "coreapp", "coreapp2")):
                        if mm.load(BOSS + 0x297C0 + 0x10 * j + 4 * k, 4) == ptr:
                            return [name, k]
            for a, role in parts[1:]:
                if role == "core":
                    for k in range(2):
                        if mm.load(a + 0x29360 + 4 * k, 4) == ptr:
                            return [role, "aura", k]
                elif mm.load(a + 0x293E0, 4) == ptr:
                    return [role, "aura"]
            return None
        for a, role in parts:
            if ptr == a + 0x40:
                return [role, "pos"]
        for off, name in sync_slots().items():
            if ptr == BOSS + off:
                return name
        for a, role in parts[2:]:
            if ptr == a + 0x293F0:
                return [role, "aura"]
        return ["?", ptr]

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
            if name == "DMY_marker01":
                for k in range(4):
                    mm.store(a + 16 + 4 * k, 4, self.marker[k])
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
            # ccCoord's: its scale and transparency (+0x84, +0x88) 1.
            mm.store(a + 0x84, 4, F_ONE)
            mm.store(a + 0x88, 4, F_ONE)
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
                     "DrawAfterImages__6ccBossFv", "DrawStageEffect__6ccBossFv",
                     "InitAfterImage__6ccBossFP5ccAnmi", "__dl__FPv", "DrawBossEffect__6ccBossFv",
                     "CamMain__9ccBossCamFv", "SetMatrix_PosRotXYZScale__7ccCoordFPfPfPf",
                     "SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf", "SetMatrix_PosRotXYZ__7ccCoordFPfPf",
                     "Draw__7ccClumpFv", "SetTransparency__7ccClumpFf", "__dt__7ccClumpFv",
                     "ParticleDelete__19ccParticleGeneratorFv",
                     "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff", "__dt__7ccLayerFv",
                     "changeCamera__Fi", "DrawPartsClump__6ccBossFP7ccClump", "Init__5ccEffFP10ccEffChunki"):
            hook(name, nop)

        # The hits' list (SetHitSW, HitEnable, HitDisable and
        # CollisionDetection) runs natively from an empty list each case:
        # the body's and the gomoras' bodies. No ground: ccModelHitCheckQ
        # finds nothing (-1.0).
        def no_ground(mm, *_):
            mm.f[0] = 0xBF800000
            return 0
        hook("ccModelHitCheckQ__FPfPffUii", no_ground)
        hook("__ct__16ccDrawPacketCtrlFv", this)

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
        hook("ccSeOn3D__FiPf", lambda mm, se, p, *_: out.append(["se3d", s32(se)]) or 0)
        hook("ccSeOn__Fi", lambda mm, se, *_: out.append(["se", s32(se)]) or 0)
        hook("ccSeOn3DNote__FiPfc", lambda mm, se, p, n, *_: out.append(["se3dnote", s32(se), n & 0xFF]) or 0)
        hook("EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T", lambda mm, *a: out.append(["afterimage"]) or 0)
        hook("DrawParts__6ccBossFP5ccAnmi",
             lambda mm, this, anm, flag, *_: out.append(["draw_parts", self.anms.get(anm, [None, 0])[1] >> 8,
                                                         s32(flag)]) or 0)

        def afterimage_anm(mm, this, anm, pos, dirc, *_):
            eai = mm.r[8] & 0xFFFFFFFF  # t0
            # The arm's frame (ccBossAfterImageEx::Init reads +0x98), its
            # place and turn, and the ENTRYAFTERIMAGE_T's words.
            words = [mm.load(eai + o, 4) for o in (0, 4, 8, 12, 16, 20, 32, 36, 40, 44, 48, 52, 56, 60, 64)]
            vec = [mm.load(p + 4 * k, 4) for p in (pos, dirc) for k in range(3)]
            out.append(["afterimage_anm", s32(mm.load(anm + 0x98, 4))] + vec + words)
            return 0
        hook("EntryAfterImageAnm__6ccBossFP5ccAnmPfPfP17ENTRYAFTERIMAGE_T", afterimage_anm)
        hook("OnCinemaMode__16ccBossEffManagerFi", lambda mm, a, n, *_: out.append(["cinema", s32(n)]) or 0)
        hook("OffCinemaMode__16ccBossEffManagerFv", lambda mm, *a: out.append(["cinema", -99]) or 0)
        hook("ccEntryFlyFontNew__FiiPfP6ccCharff",
             lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccHitMarkDisp__FP6ccCharP6ccChar", lambda mm, *a: out.append(["hitmark"]) or 0)
        hook("ccClearSpcCondition__Fv", lambda mm, *a: out.append(["clear_spc"]) or 0)
        hook("effResistantShield__FP6ccCharii", lambda mm, ch, k, *_: out.append(["shield", s32(k)]) or 0)
        hook("ccSqFade__FiUsiUc", lambda mm, a, b_, t, *_: out.append(["music_fade", s32(t)]) or 0)
        hook("effSmokeRock__FPfPffiii", lambda mm, *a: out.append(["smoke_rock"]) or 0)
        hook("effSkillStart__FP6ccChariii", lambda mm, ch, sid, *_: out.append(["skill_start", s32(sid)]) or 0)
        hook("ccItemSkillRequest__FP6ccCharP6ccCharii",
             lambda mm, cp, tp, sid, *_: out.append(["skill", s32(sid)]) or 0)

        def own_skill(mm, cp, tp, sid, *_):
            # _ccSkillRequest's caster side for an own skill (the heal,
            # 295): skillID and skillStatus 1, as skill::request sets them.
            out.append(["skill", s32(sid)])
            mm.store(cp + 0x7C, 2, sid & 0xFFFF)
            mm.store(cp + 0x7E, 2, 1)
            return 0
        hook("ccSkillRequestParam__FP6ccCharP6ccCharii", own_skill)
        hook("cameraSetPos__FPfi", nop)
        hook("cameraSetView__FPfi", nop)
        hook("IsMove__11EVENTAREAB8Fv", lambda mm, *a: int(self.moving or self.ride > 0))

        def disc_next(mm, *a):
            out.append(["disc_next"])
            self.ride = self.ride2
            return 0
        hook("Move__11EVENTAREAB8Fv", disc_next)
        hook("ccSeOnNote__Fic", lambda mm, se, n, *_: out.append(["senote", s32(se), n & 0xFF]) or 0)

        def flash(mm, f, t, c, *_):
            # Into the first free element as the game's (status 3), which
            # the fourth fight's EntryFlash3 and CheckFade see.
            out.append(["flash", s32(t), c & 0xFFFFFFFF])
            for k in range(4):
                e = f + 4 + 36 * k
                if s16(mm.load(e, 2)) == 0:
                    mm.store(e, 2, 3)
                    mm.store(e + 2, 2, 0)
                    mm.store(e + 4, 2, t & 0xFFFF)
                    return k
            return 0xFFFFFFFF
        hook("EntryFlash__8ccScFadeFiiffff", flash)
        if FIGHT == 4:
            # The EX ball's eye (MagicBallAtk's sixth step) is a stack vector
            # whose x the code never writes: the word ccGetDirc's atan2f left
            # there, its scratch pointer (a ccSys work address, a float under
            # 2^-120) that only shows when ccRandF drew 0 exactly. The port
            # reads it as 0; the harness puts 0 there (docs, "The eyes").
            unit_at = sym("sceVu0UnitMatrix")

            def unit(mm, a, *_):
                for i in range(4):
                    for j in range(4):
                        mm.store(a + 16 * i + 4 * j, 4, F_ONE if i == j else 0)
                if 0x003E3C60 <= mm.r[31] & 0xFFFFFFFF < 0x003E4370:
                    mm.store(a + 64, 4, 0)
                return 0
            h[unit_at] = unit
            # EVENTAREAB8's body[n] hit off and on (field 12's walls).
            hook("HitOff__11EVENTAREAB8Fi", lambda mm, a, n, *_: out.append(["gate", s32(n), 0]) or 0)
            hook("HitOn__11EVENTAREAB8Fi", lambda mm, a, n, *_: out.append(["gate", s32(n), 1]) or 0)
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
        # The thunderbolts' clump (their picture is not compared).
        hook("Duplicate__7ccClumpFUi", nop)
        hook("ChangeClut__7ccClumpFP11ccClutChunkP11ccClutChunk", nop)

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
        hook("ccLandHitCheck__FPfUi", lambda mm, *_: 0)

        tables = {sym(n): n[:-len("Generator")] for n in TABLES}
        aura_ff = sym("GomoraAuraGeneratorForceField")
        ff_tables = {n[:-len("Generator")]: sym(n.replace("Generator", "ForceField") if n.endswith("SmokeGenerator")
                                                else n + "ForceField") for n in TABLES}

        def gen_ct(mm, a, param, ff, *_):
            # What the constructor (not run) leaves that the makers read on:
            # distSW 1, syncPosType 0, pStop 0, no syncPos, pos and offset
            # the vf0 (as piney-effect's Generator::new).
            self.gens[a] = (param, [ff] + [mm.r[k] & 0xFFFFFFFF for k in (7, 8, 9)])
            mm.store(a + 4, 1, 1)
            mm.store(a + 5, 1, 0)
            mm.store(a + 0x18, 4, 0)
            mm.mem[a + 0x70:a + 0x80] = bytes(16)
            for v in (0x60, 0x80):
                for k in range(4):
                    mm.store(a + v + 4 * k, 4, F_ONE if k == 3 else 0)
            return a
        hook("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParamP25"
             "ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam", gen_ct)

        def gen_start(mm, a, *_):
            param, ffs = self.gens.get(a, (0, [0, 0, 0, 0]))
            # The table nearest below the row (they lie back to back).
            near = [b_ for b_ in tables if b_ <= param and (param - b_) % 0x38 == 0]
            if not near:
                return 0
            base = max(near)
            name, row = tables[base], (param - base) // 0x38
            if name == "GomoraAura":
                lv = (ffs[0] - aura_ff) // 0x20 if row < 4 else 0
                head = ["particles", name, row, lv]
            else:
                head = ["particles", name, row]
            # The missiles' and bolts' own are piney-effect's (their
            # pictures' harness); the rest in full.
            if name in ("MissileSmoke", "BurstSmoke") and row != 17:
                out.append(head)
                return 0
            # A force field of another table's (one of the fourth fight's)
            # by its row there.
            def ff_row(f):
                base = ff_tables[name] if f >= ff_tables[name] else max(t for t in ff_tables.values() if t <= f)
                return (f - base) // 0x20
            ff = [ff_row(f) for f in ffs if f]
            sync = mm.load(a + 0x18, 4)
            if sync:
                place = ["follow", self.member(mm, sync, "sync"), *f32s(mm, a + 0x80, 3)]
            elif any(f32s(mm, a + 0x70, 3)):
                place = ["line", *f32s(mm, a + 0x60, 3), *f32s(mm, a + 0x70, 3)]
            else:
                place = ["at", *f32s(mm, a + 0x60, 3)]
            out.append(head + [ff, mm.load(a + 4, 1) & 1, place, self.member(mm, a, "held")])
            return 0
        hook("startParticleGenerator__FP19ccParticleGenerator", gen_start)

        def gen_kill(mm, a, *_):
            if a:
                out.append(["kill", self.member(mm, a, "held")])
            return 0
        hook("ParticleKill__19ccParticleGeneratorFv", gen_kill)

    def effects_pass(self):
        """ccBossEffManager::Draw's loop: a disabled effect freed, an
        enabled one's Draw (vtable +8) run natively."""
        m = self.m
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if not e:
                continue
            if not m.load(e, 1):
                m.store(MGR + 52 + 4 * k, 4, 0)
                continue
            draw = m.load(m.load(e + 12, 4) + 8, 4)
            # The top quadword of a Draw's callee's frame (the magical
            # ball's DrawPlasma reads its spline point from there before
            # writing it): in the game ccScFade::SendPacket's `sq s7`
            # under the cinema's Draw left the task's s7 (0) there the
            # frame before.
            m.mem[eemu.STACK_TOP - 80:eemu.STACK_TOP - 64] = bytes(16)
            m.call(draw, (e,))
        bb.blur_main(self, MGR)

    def effects(self):
        m = self.m
        out = []
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                vt = m.load(e + 12, 4)
                kind = {self.meteor_vt: 10, self.bolt_vt: 17, self.ball_vt: 18, self.hell_vt: 19}.get(vt, -1)
                out.append([k, kind, m.load(e, 1)])
        return out


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    # The later fights' core dies a time a stage before the body.
    second = FIGHT >= 2
    frames = rnd.randrange(300, {1: 2500, 4: 14000}.get(FIGHT, 6000))
    marker = [fbits(rnd.uniform(-500, 500)), fbits(rnd.uniform(2000, 4000)), fbits(rnd.uniform(0, 400)), F_ONE]
    # The disc rides in for a while, then stops.
    ride = rnd.randrange(0, 200)
    at = [rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), rnd.uniform(-100, 100)]
    step = [rnd.uniform(-30, 30), rnd.uniform(-30, 30), 0.0]
    prev, moving = [], []
    for f in range(frames):
        if f < ride:
            at = [at[k] + step[k] for k in range(3)]
        prev.append([fbits(v) for v in at])
        moving.append(int(f < ride))
    script = []
    rough = rnd.random() < 0.5
    hits = rnd.sample(range(frames), min(frames // (6 if rough else 20), 1000 if second else 400))
    strong = {1: 600, 4: 3000}.get(FIGHT, 1200)
    for f in sorted(hits):
        kind = rnd.choice((0, 0, 0, 1, 1, 2))
        who = rnd.randrange(0, 5)
        p0 = {0: rnd.randrange(-1, strong if rough else 250), 1: rnd.randrange(-1, 700), 2: rnd.randrange(0, 300)}[kind]
        p1 = rnd.choice((0, 1, 1, 1, 2, 30, 60, 100, 150, 200, 250, 300))
        script.append((f, kind, who, p0, p1))
    menus = []
    a = 0
    while a < frames and rnd.random() < 0.6:
        a += rnd.randrange(20, 300)
        menus += [(a, rnd.choice((0, 5))), (a + rnd.randrange(1, 60), -1)]
        a = menus[-1][0]
    rot = [0] * frames
    r = rnd.uniform(-3.1, 3.1)
    for f in range(frames):
        r += rnd.uniform(-0.05, 0.05)
        if r > 3.14159:
            r -= 6.28318
        if r < -3.14159:
            r += 6.28318
        rot[f] = fbits(r)
    cam_pos = [fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(100, 400)), F_ONE]
    cam_view = [fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(-2000, 2000)), fbits(rnd.uniform(0, 200)), F_ONE]
    case = {"party": party, "frames": frames, "script": script, "menus": menus, "marker": marker,
            "rot": rot, "moving": moving, "prev": prev, "cam_pos": cam_pos, "cam_view": cam_view,
            "delay": rnd.randrange(1, 40), "ride2": rnd.randrange(0, 150),
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}
    if FIGHT == 4:
        # Kite set down at the places in turn, near each (250 starts its
        # core) or off it, now and then.
        walks = []
        f, k = rnd.randrange(1, 60), 0
        while f < frames:
            x, y, z = PLACES[k % 4]
            r = rnd.uniform(0, 200 if rnd.random() < 0.8 else 600)
            a = rnd.uniform(-3.14, 3.14)
            import math
            walks.append((f, [fbits(x + r * math.cos(a)), fbits(y + r * math.sin(a)), fbits(z)]))
            k += 1 if rnd.random() < 0.7 else 0
            f += rnd.randrange(40, 700)
        case["walks"] = walks
    # Drawn last, so the cases of a seed are the ones before the blur.
    case.update(bb.blur_case(rnd))
    case.update(bb.draw_case(rnd))
    return case


# The fourth fight's places (`CoreAppearPos`, the constructor's).
PLACES = [(-576.0, -9903.0, 910.0), (-2975.0, -7733.0, 910.0), (-733.0, -4163.0, 900.0), (145.0, -2176.0, 900.0)]


def fade_step(m, f):
    """scFadeDef's SendPacket's count each frame (main fade.cpp; the font
    layer sends it after the tasks): each drawing element on, the next
    ramp or the end past its count."""
    for k in range(4):
        e = f + 4 + 36 * k
        st = s16(m.load(e, 2))
        if not st & 1:
            continue
        cnt = s16(m.load(e + 2, 2)) + 1
        tcnt = s16(m.load(e + 4, 2))
        if cnt > tcnt:
            if st & 2:
                st = 0
            elif st & 4:
                st, cnt, tcnt = (st & ~4) | 2, 0, s16(m.load(e + 6, 2))
            elif st & 8:
                st, cnt, tcnt = (st & ~8) | 4, 0, s16(m.load(e + 8, 2))
            else:
                cnt = tcnt
        m.store(e, 2, st & 0xFFFF)
        m.store(e + 2, 2, cnt & 0xFFFF)
        m.store(e + 4, 2, tcnt & 0xFFFF)


def held_slots():
    """The body's members that keep a generator, by fight: the light, the
    beams' heads and tails, DmgGp and DeadGp."""
    d = 0 if FIGHT == 1 else 0x10
    out = {0x29500 + d: "light"}
    for k in range(3):
        out[0x29504 + d + 4 * k] = ["head", k]
        out[0x29510 + d + 4 * k] = ["tail", k]
    smoke = {1: 0x29550, 4: 0x29600}.get(FIGHT, 0x295F0)
    for k in range(5):
        out[smoke + 4 * k] = ["smoke", k]
    return out


def sync_slots():
    """The body's vectors a generator follows: DedDmgVec, LBpos, LBposSub."""
    d = 0 if FIGHT == 1 else 0x10
    out = {(0x29540 if FIGHT == 1 else 0x295E0): "dead"}
    for k in range(3):
        out[0x294A0 + d + 16 * k] = ["head", k]
        out[0x294D0 + d + 16 * k] = ["tail", k]
    return out


def core_of(m):
    return m.load(BOSS + 0x290, 4)


def gomora_of(m, k):
    return m.load(core_of(m) + 0x290, 4) + GOMORA_SIZE * k


def run_game(game, case):
    c, g, m = game.c, game.g, game.m
    c.reset()
    # bossTbl as on the disc: a core's deaths add to its rows' maxHP.
    rows = c.data.bosses
    g.restore(rows[0]["va"], 0x68 * len(rows))
    c.scene.new_at = NEW
    game.fake_at = FAKE
    game.slaves_at = SLAVES
    game.anms.clear()
    game.gens.clear()
    game.out.clear()
    game.marker = case["marker"]
    game.frame["pos"] = case["cam_pos"]
    game.frame["view"] = case["cam_view"]
    game.frame["rot"] = case["rot"][0]
    game.moving = case["moving"][0]
    game.meteor_vt = game.sym("__vt__25ccBossEffMeteoriteMissile")
    game.bolt_vt = game.sym("__vt__20ccBossEffThunderbolt")
    game.ball_vt = game.sym("__vt__20ccBossEffMagicalBall")
    game.hell_vt = game.sym("__vt__18HellSpecialExpress") if FIGHT == 4 else -1
    game.ride, game.ride2, game.stage_on = 0, case["ride2"], False
    f_ = FIGHTS[FIGHT]
    m.mem[BOSS:BOSS + f_["size"]] = bytes(f_["size"])
    bb.put_draw_camera(game, case)
    bb.make_manager(game, MGR, case)
    m.mem[CAM:CAM + 0x150] = bytes(0x150)
    m.mem[WM:WM + 0x5000] = bytes(0x5000)
    m.store(game.sym("_g_bossEffManager"), 4, MGR)
    for name in ("ccCharHitTop", "ccCharHitTail", "ccCharHitEntryNum"):
        m.store(game.sym(name), 4, 0)
    m.store(game.sym("scFadeDef"), 4, STAGE + 0x200)
    # worldman->eventmap (+1092), its stream (+420) and discPrevPos (+0xca0)
    m.store(game.sym("worldman"), 4, WM)
    m.store(WM + 1092, 4, AREA)
    m.store(AREA + 420, 4, AREA_CCS)
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

    def disc(f):
        p = case["prev"][f]
        for k in range(3):
            m.store(AREA + 0xCA0 + 4 * k, 4, p[k])
        m.store(AREA + 0xCAC, 4, F_ONE)
        game.moving = case["moving"][f]
    disc(0)
    game.out.clear()
    m.mem[STAGE + 0x200:STAGE + 0x300] = bytes(0x100)
    m.mem[BOSS_STACK - 0x10000:BOSS_STACK] = bytes([int(os.environ.get("PINEY_STACK_POISON", "0"), 16)]) * 0x10000
    m.call(game.sym(f_["ct"]), (BOSS, f_["level"]), sp=BOSS_STACK)
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    left = 0
    for f in range(case["frames"]):
        game.out.clear()
        game.frame["rot"] = case["rot"][f]
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        # The camera's mode 3 or 4 cleared after the case's delay.
        reset = s32(m.load(CAM + 4, 4))
        if reset in (3, 4):
            if left == 0:
                left = case["delay"]
            left -= 1
            if left == 0:
                m.store(CAM + 4, 4, 0)
        else:
            left = 0
        for sf, kind, who, p0, p1 in case["script"]:
            if sf != f:
                continue
            on = gomora_of(m, who) if kind == 1 else core_of(m)
            if s16(m.load(on + 8, 2)) != 0 or m.load(on + 0x1D4, 1) != 0:
                continue
            ty = 7 if kind == 2 else 1
            m.call(ea, (on, kite, ty, p0 & 0xFFFF, p1 & 0xFFFF, 0))
        disc(f)
        for wf, p in case.get("walks", []):
            if wf == f:
                for k in range(3):
                    m.store(kite + 0x40 + 4 * k, 4, p[k])
                m.store(kite + 0x4C, 4, F_ONE)
        if game.ride > 0:
            game.ride -= 1
        game.effects_pass()
        m.call(game.sym(f_["main"]), (BOSS,), sp=BOSS_STACK)
        fade_step(m, STAGE + 0x200)
        frames.append(game_state(game, len(party)))
    return frames


def f32s(m, a, n):
    return [m.load(a + 4 * k, 4) for k in range(n)]


def fields(m, base, spec):
    """The members at `base` by (offset, kind): 'i' an int, 'h' a short,
    'b' a signed byte, 'u' an unsigned byte, 'f' a float's bits, 'v' a
    vector's x, y and z, 'p' a pointer set or not."""
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
        elif kind == "p":
            out.append(int(m.load(a, 4) != 0))
        elif kind == "q":
            g = m.load(a, 4)
            out.append(int(g != 0 and m.load(g + 5, 1) & 1))
        else:
            out += f32s(m, a, 3)
    return out


def vs(off, n):
    return [(off + 16 * k, "v") for k in range(n)]


# ccBossKyvia01's own members (Infection's DWARF; +0x295f8 Mutation's
# cinema flag): StFlg, WaitDiscCount, DiscLV, LiveFlg, AtkPatMode,
# DmgAnmFlg, HpProccess, AFcou, the cinema, timeMode, MaxHP, SwitchHp,
# DiscPos, LOffset, Atk_vec, Z, SPoint, Btime, AddTime, SubBtime, LBpos,
# LBposSub, LChar, LPoint, tesPOS, tesDIRC, SndPOS_L, SndPOS_R, DedDmgVec,
# QuakeVector, MoveTransfer, DmgGp, DeadGp.
KYVIA = ([(0x29670, "u"), (0x29688, "h"), (0x29370, "h"), (0x2968A, "u"), (0x293E0, "i"), (0x29534, "i"),
          (0x29530, "i"), (0x29674, "i"), (0x295F8, "u"), (0x295F0, "i"), (0x29528, "i"), (0x2952C, "i"),
          (0x29360, "v"), (0x29400, "v"), (0x295D0, "v"), (0x29580, "f"), (0x29600, "v"), (0x2951C, "f"),
          (0x29520, "f"), (0x29524, "f")] + vs(0x294A0, 3) + vs(0x294D0, 3) + vs(0x29470, 3) + vs(0x29410, 6)
         + [(0x293A0, "v"), (0x293B0, "v"), (0x293C0, "v"), (0x293D0, "v"), (0x29540, "v"), (0x295E0, "v"),
            (0x29570, "v")] + [(0x29550 + 4 * k, "p") for k in range(4)] + [(0x29560, "p")]
         + [(0x29500, "p")] + [(0x29504 + 4 * k, "p") for k in range(3)] + [(0x29510 + 4 * k, "p") for k in range(3)])
# ccBossKyvia02's (Outbreak's layout: Infection's DWARF with a byte at
# +0x29350 before the rest, the cinema's, and the rest 0x10 on): the same
# members, then ThunderType, ThunderTime, T_NUM, T_Range, StageID.
KYVIA2 = ([(0x29710, "u"), (0x29728, "h"), (0x29380, "h"), (0x2972A, "u"), (0x293F0, "i"), (0x295D4, "i"),
           (0x295D0, "i"), (0x29714, "i"), (0x29350, "u"), (0x29690, "i"), (0x295C8, "i"), (0x295CC, "i"),
           (0x29370, "v"), (0x29410, "v"), (0x29670, "v"), (0x29620, "f"), (0x296A0, "v"), (0x2952C, "f"),
           (0x29530, "f"), (0x29534, "f")] + vs(0x294B0, 3) + vs(0x294E0, 3) + vs(0x29480, 3) + vs(0x29420, 6)
          + [(0x293B0, "v"), (0x293C0, "v"), (0x293D0, "v"), (0x293E0, "v"), (0x295E0, "v"), (0x29680, "v"),
             (0x29610, "v")] + [(0x295F0 + 4 * k, "p") for k in range(4)] + [(0x29600, "p")]
          + [(0x29510, "p")] + [(0x29514 + 4 * k, "p") for k in range(3)] + [(0x29520 + 4 * k, "p") for k in range(3)]
          + [(0x29538, "i"), (0x2953C, "i"), (0x295C0, "h"), (0x295C4, "i"), (0x29734, "i")])
# ccBossKyvia03's (Quarantine's: the second's, with ThunderGp before MaxHP
# moving MaxHP, SwitchHp, HpProccess and DmgAnmFlg 4 on), then the member
# DWARF leaves out at +0x293f4, the last stage's count.
KYVIA3 = [(o + 4 if 0x295C8 <= o <= 0x295D4 else o, k) for o, k in KYVIA2] + [(0x293F4, "i")]
# ccBossKyvia04's (Quarantine's: Infection's DWARF as it is but +0x29614
# and +0x29618, the rebirth's flag and count, and the cinema's two bytes at
# +0x29350, the second the core's rise): the second's members, then
# FightFlg, the rise, CoreLvCount, CoreAppearPos, DeadSetPos, RebirthFlg,
# RebirthCnt, WallGpFlg, each place's four generators held and paused
# (WallGp, WallGp2, CoreAppGp, CoreAppGp2), FadeNum.
KYVIA4 = ([(0x29860, "u"), (0x29878, "h"), (0x29380, "h"), (0x2987A, "u"), (0x293F0, "i"), (0x295D8, "i"),
           (0x295D4, "i"), (0x29864, "i"), (0x29350, "u"), (0x296C0, "i"), (0x295CC, "i"), (0x295D0, "i"),
           (0x29370, "v"), (0x29410, "v"), (0x296A0, "v"), (0x29650, "f"), (0x296D0, "v"), (0x2952C, "f"),
           (0x29530, "f"), (0x29534, "f")] + vs(0x294B0, 3) + vs(0x294E0, 3) + vs(0x29480, 3) + vs(0x29420, 6)
          + [(0x293B0, "v"), (0x293C0, "v"), (0x293D0, "v"), (0x293E0, "v"), (0x295E0, "v"), (0x296B0, "v"),
             (0x29640, "v")] + [(0x29600 + 4 * k, "p") for k in range(4)] + [(0x29610, "p")]
          + [(0x29510, "p")] + [(0x29514 + 4 * k, "p") for k in range(3)] + [(0x29520 + 4 * k, "p") for k in range(3)]
          + [(0x29538, "i"), (0x2953C, "i"), (0x295C0, "h"), (0x295C4, "i"), (0x29880, "i")]
          + [(0x2987B, "u"), (0x29351, "u"), (0x29730, "i")] + vs(0x296F0, 4) + [(0x295F0, "v")]
          + [(0x29614, "i"), (0x29618, "i"), (0x29800, "i")]
          + [(0x297C0 + 0x10 * j + 4 * k, kind) for k in range(4) for j in range(4) for kind in ("p", "q")]
          + [(0x29884, "i")])
FIGHTS = {
    1: {"ct": "__ct__13ccBossKyvia01Fi", "main": "Main__13ccBossKyvia01Fv", "level": 1, "size": 0x296A0,
        "row": 12, "fields": KYVIA, "anmw": 0x29358},
    2: {"ct": "__ct__13ccBossKyvia02Fi", "main": "Main__13ccBossKyvia02Fv", "level": 2, "size": 0x29740,
        "row": 13, "fields": KYVIA2, "anmw": 0x2935C},
    3: {"ct": "__ct__13ccBossKyvia03Fi", "main": "Main__13ccBossKyvia03Fv", "level": 3, "size": 0x29740,
        "row": 13, "fields": KYVIA3, "anmw": 0x2935C},
    4: {"ct": "__ct__13ccBossKyvia04Fi", "main": "Main__13ccBossKyvia04Fv", "level": 4, "size": 0x29890,
        "row": 15, "fields": KYVIA4, "anmw": 0x2935C},
}
# Where ThunderType falls in each fight's fields (a vector is three).
THUNDER_AT = {f: sum(3 if k == "v" else 1 for _, k in KYVIA2[:-5]) for f in (2, 3, 4)}
# kyviaCore's: Myattribute, NO, endFlg, CoreState, tempState, DeadFlg,
# DeadCount, kAtkFlg, kDmgFlg, KyviaLV, ConeVoiceCou, NowGomoraNum,
# GomoraCount, GsCount, Gomorahold, GomoraMeter, GomoraLock, time,
# timeCount, timeFlg, timeMode, TempTime, SpVec1-4, DeadPos, StFlg, Alpha,
# DiscPos, EscapeCou, DmgCount, DmgWait, NowDmgtime, Dmgtime, ThinkProccess,
# AFcou, FadeFlg, Aura, Aura2.
CORE = ([(0x2935C, "i"), (0x293A0, "i"), (0x293A8, "i"), (0x293AC, "i"), (0x293B0, "i"), (0x293B8, "u"),
         (0x293BC, "i"), (0x293C2, "h"), (0x293C4, "h"), (0x293CC, "h"), (0x293D0, "h"), (0x293DC, "i"),
         (0x293E0, "i"), (0x293E8, "i"), (0x293F6, "h"), (0x293F8, "i"), (0x293FC, "i"), (0x29420, "f"),
         (0x29424, "f"), (0x29428, "f"), (0x29430, "i"), (0x2942C, "f")] + vs(0x29440, 4)
        + [(0x29480, "v"), (0x29540, "u"), (0x29544, "f"), (0x29550, "v"), (0x29570, "h"), (0x29574, "i"),
           (0x29578, "i"), (0x2957C, "i"), (0x29580, "i"), (0x29584, "i"), (0x2958C, "i"), (0x29594, "u"),
           (0x29360, "p"), (0x29364, "p"), (0x29360, "q"), (0x29364, "q")])
# kyviaGomora's: LifeFlg, Enyint, Enyfloat, AtkFlg, TmpAtkFlg, MyAttribute,
# AuraLv, AuraPos, ripusNUM, AFcou, AtkWait, AtkTime, AtkTmpFLg, AtkTarPos,
# DpCou, StopMoveCount, time, timeCount, timeMode, timeFlg, FadeFlg,
# NowListNum, KyviaStep, SpVec1-4, AtkVec1-8, DpVec1-4, DiscPos,
# MovePattern, DeadVoiceFlg, Alpha, StFlg, state, tmpstate.
GOMORA = ([(0x29368, "u"), (0x2936C, "i"), (0x29370, "f"), (0x293D0, "i"), (0x293D4, "i"), (0x293D8, "i"),
           (0x293DC, "i"), (0x293F0, "v"), (0x29404, "i"), (0x29408, "i"), (0x29410, "i"), (0x29414, "i"),
           (0x29418, "u"), (0x29420, "v"), (0x29434, "i"), (0x29438, "i"), (0x2943C, "f"), (0x29440, "f"),
           (0x29444, "i"), (0x29448, "i"), (0x29450, "u"), (0x29452, "h"), (0x2945E, "h")]
          + vs(0x29460, 4) + vs(0x294A0, 8) + vs(0x29520, 4)
          + [(0x29580, "v"), (0x295D0, "i"), (0x295D4, "u"), (0x295D8, "f"), (0x295E0, "u"), (0x295E4, "i"),
             (0x295E8, "i"), (0x293E0, "p")]
          # bodyHit: hitSW, mask, mask2, type, radius, height, pos, offset.
          + [(0x240, "i"), (0x244, "i"), (0x248, "i"), (0x24C, "i"), (0x254, "f"), (0x258, "f"), (0x260, "v"),
             (0x270, "v")])
# The boss camera's: ResetFlg, lock, CamView, CamPos, the pitch (+0xe0),
# ExLock and ZrotControlFlg (the stick's turn, +0x139).
CAM_F = [(0x4, "i"), (0x0, "u"), (0x10, "v"), (0x20, "v"), (0xE0, "f"), (0x138, "u"), (0x139, "u")]


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


def boss_part(game, a):
    """A ccBoss's as battle_probe's boss_json has them."""
    m = game.m
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    tgt = ld(0x78)
    clip = game.anms.get(ld(0xD4), [None, 0])
    return ([ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)]
            + f32s(m, a + 0x40, 3) + f32s(m, a + 0x60, 3)
            + [ld(0x70, 2, True), ld(0x74, 2, True), (tgt - SCN) // 0x1000 if tgt else -1,
               ld(0x1D0, 1, True), ld(0x1D2, 1, True), ld(0x1D3, 1, True), s32(ld(0x2BC)), listed(game, a),
               clip[0] or "", clip[1] >> 8, ld(0x88), ld(0x8C)])


def stream(game, a):
    """The part's stream (+0xcc), the name CoreInit or GomoraInit asked
    GetCCSAdrs for: the file its clips and models come from."""
    return game.names.get(game.m.load(a + 0xCC, 4), "")


def game_state(game, n):
    m = game.m
    a = BOSS
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    clip = game.anms.get(ld(0xD4), [None, 0])
    f_ = FIGHTS[FIGHT]
    wclip = game.anms.get(ld(f_["anmw"]), [None, 0])
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    core = core_of(m)
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "pos": f32s(m, a + 0x40, 3),
        "flags": [ld(0x1D0, 1, True), ld(0x1D1, 1, True), ld(0x1D2, 1, True), ld(0x1D4, 1, True),
                  ld(0x1D5, 1, True), ld(0x1D7, 1, True), s32(ld(0x29334)), s32(ld(0x29338))],
        "tr": [ld(0x88), ld(0x8C)],
        "anm": [clip[0] or "", clip[1] >> 8, wclip[0] or "", wclip[1] >> 8],
        "kyvia": fields(m, a, f_["fields"]),
        "core": boss_part(game, core) + fields(m, core, CORE) + [stream(game, core)],
        "gomoras": [boss_part(game, gomora_of(m, k)) + fields(m, gomora_of(m, k), GOMORA)
                    + [stream(game, gomora_of(m, k))] for k in range(5)],
        "cam": fields(m, CAM, CAM_F),
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2)),
                 int(m.load(MENU + 0x238, 4) != 0)],
        "blur": bb.blur_state(game, MGR),
        "ball": ball_state(game),
        "hell": hell_state(game) if FIGHT == 4 else None,
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def ball_state(game):
    """The magical ball's members (ccBossEffMagicalBall, Infection's DWARF)
    as battle_probe's ball_json has them; None without one."""
    m = game.m
    for k in range(1024):
        e = m.load(MGR + 52 + 4 * k, 4)
        if not e or m.load(e + 12, 4) != game.ball_vt:
            continue
        ld = lambda o: m.load(e + o, 4)  # noqa: E731
        anm = ld(0x24)
        # The words unsigned, as the probe prints them.
        out = [ld(0x2C), ld(0x14), ld(0x90), ld(0x94), ld(0xA8), ld(0xAC), ld(0x64), ld(0x68), ld(0xB0),
               m.load(anm + 0x88, 4) if anm else 0] + f32s(m, e + 0x40, 3)
        for j in range(s32(ld(0x18))):
            out += [m.load(ld(0x88) + 4 * j, 4), m.load(ld(0x8C) + 4 * j, 4), m.load(ld(0x98) + 4 * j, 4),
                    m.load(ld(0x9C) + 4 * j, 4)]
            for o in (0x70, 0x74, 0x78, 0xBC, 0xC0):
                out += f32s(m, ld(o) + 16 * j, 3)
        return out
    return None


def hell_state(game):
    """Hell's express's members (HellSpecialExpress, Infection's DWARF) as
    battle_probe's hell_json has them; None without one."""
    m = game.m
    for k in range(1024):
        e = m.load(MGR + 52 + 4 * k, 4)
        if not e or m.load(e + 12, 4) != game.hell_vt:
            continue
        ld = lambda o: m.load(e + o, 4, True)  # noqa: E731
        tgt = m.load(e + 0x1C, 4)
        anm = m.load(e + 0x170, 4)
        out = [ld(0x164), ld(0x168), ld(0x15C), ld(0x160), m.load(e + 0x174, 1), ld(0x20),
               (tgt - SCN) // 0x1000 if tgt else -1, game.anms.get(anm, [None, 0])[1] >> 8]
        for o in (0x70, 0x80, 0x90, 0xA0, 0xC0):
            out += f32s(m, e + o, 3)
        for j in range(3):
            out += f32s(m, e + 0xD0 + 16 * j, 3) + [m.load(e + 0x134 + 4 * j, 4), ld(0x148 + 4 * j)]
        return out
    return None


def boss_char_pre(game):
    """The body's ccChar as SetBaseParam leaves it (bossTbl row 12 or 13)."""
    import types
    r = FIGHTS[FIGHT]["row"]
    row = dict(game.c.data.bosses[r])
    ch = types.SimpleNamespace()
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level, ch.exp = r, row.get("level", 1), 0
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
    """The probe's `kyvia` request."""
    b = game.c.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    parts = (["kyvia", FIGHT, len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["marker"][:3]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], case["ride2"]]
             + case["cam_pos"] + case["cam_view"] + [case["delay"]] + case["rot"] + case["moving"])
    for p in case["prev"]:
        parts += p
    parts.append(len(case["script"]))
    for s in case["script"]:
        parts += list(s)
    parts.append(len(case["menus"]))
    for mm in case["menus"]:
        parts += list(mm)
    if FIGHT == 4:
        parts.append(len(case["walks"]))
        for wf, p in case["walks"]:
            parts += [wf] + p
    parts += bb.blur_tokens(case)
    parts += bb.draw_tokens(case)
    return " ".join(str(x) for x in parts)


def first_diff(a, b):
    """Where two values first differ: a list's index with a few around."""
    if isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                return f" at [{i}] game {x}\n       port {y}\n around game {a[max(0, i - 3):i + 4]}\n" \
                       f"        port {b[max(0, i - 3):i + 4]}"
        return f" lengths {len(a)} {len(b)}: game tail {a[len(b):][:4]} port tail {b[len(a):][:4]}"
    return f" game {a} port {b}"


def compare(game, case, label):
    pre = boss_char_pre(game)
    g_frames = run_game(game, case)
    p_frames = rs.ask([request(game, case, pre)])[0]
    for f, (a, b_) in enumerate(zip(g_frames, p_frames)):
        a = rs.norm(a)
        for k in a:
            if a[k] != b_.get(k):
                raise AssertionError(f"{label} frame {f - 1}: {k}\n{first_diff(a[k], b_.get(k))}\n"
                                     f" game {a[k]}\n port {b_.get(k)}\n game {a}\n port {b_}")
    if len(g_frames) != len(p_frames):
        raise AssertionError(f"{label}: {len(g_frames)} game frames, {len(p_frames)} port frames")
    return g_frames


class Against(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if volume.NAME not in ("mutation", "outbreak", "quarantine") or (FIGHT == 4 and volume.NAME != "quarantine"):
            raise unittest.SkipTest("Kyvia is Mutation's and Outbreak's: PINEY_VOLUME=mutation or outbreak")
        if not (os.path.exists(rs.ELF) and os.path.exists(rs.ISO)):
            raise unittest.SkipTest("the disc is not extracted")
        rs.build()
        cls.game = Game()

    def test_parts_shapes(self):
        """The core's and gomoras' clips in their stream (kyvia::stream):
        the game's ccAnm stepping them (_AnimateForward, DecodeF_Morpher
        into the anm's own ccMorpher) against anim.py, whose weights the
        port's Play::morph draws with (worklog 430)."""
        import collections
        g = test_anim.GameAnim()
        counts = collections.Counter()
        c = test_anim.member(f"x0{FIGHTS[FIGHT]['level']}.cmp")
        for anm in ("ANM_ex0gnut0", "ANM_ex0gatc0", "ANM_ex0gdmg0", "ANM_ex0gmag0", "ANM_ex0cnut0", "ANM_ex0cdmg0"):
            scene, off, a = test_anim.find(c, anm)
            for speed, steps in ((256, 100), (384, 60)):
                self.assertEqual(test_anim.check_playback(g, c, scene, off, a, speed, steps, counts), [], anm)
        self.assertGreater(counts["morph states"], 500)

    def test_forced_cameras(self):
        """ccBoss::Draw's fade against cameras in and about the fade band
        (bb.FORCED_DRAW_CAMS); Kyvia's third and fourth bodies clear
        transDist and never fade (worklog 455)."""
        for k, case in bb.forced_draw_cases(self.game.c, rnd_case, 9320):
            compare(self.game, case, f"camera {k}")

    def test_cases(self):
        for seed in range(6):
            rnd = random.Random(7300 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=9300):
    rs.build()
    game = Game()
    acts = set()
    core_acts = set()
    gomora_acts = set()
    thunders = set()
    pushes = 0
    streams = set()
    total = 0
    for k in range(n):
        rnd = random.Random(seed0 + k)
        frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        acts |= {fr["act"][0] for fr in frames}
        core_acts |= {fr["core"][2] for fr in frames}
        gomora_acts |= {g[2] for fr in frames for g in fr["gomoras"]}
        # The second fight's ThunderType while its act 6 runs.
        thunders |= {fr["kyvia"][THUNDER_AT[FIGHT]] for fr in frames if FIGHT >= 2 and fr["act"][0] == 6}
        # Gomoras pushed (their bodyHit's offset, the three before the
        # stream).
        pushes += sum(1 for fr in frames for g in fr["gomoras"] if any(g[-4:-1]))
        streams |= {g[-1] for fr in frames for g in fr["gomoras"]} | {fr["core"][-1] for fr in frames}
        total += len(frames) - 1
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print(f"{n} cases, {total} frames; acts reached:", sorted(acts), "core's:", sorted(core_acts),
          "gomoras':", sorted(gomora_acts), "thunders:", sorted(thunders), "gomoras pushed:", pushes,
          "streams:", sorted(streams))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 9300)
    else:
        unittest.main()
