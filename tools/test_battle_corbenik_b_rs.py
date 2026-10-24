#!/usr/bin/env python3
"""crates/piney-battle's Corbenik, second form (src/boss/corbenik/second.rs,
stardust.rs, eff.rs) against Quarantine's boss08_b.cpp run natively in the
Rust VU0 machine (tools/eemu_rs.so), frame by frame with battle_probe's
`corbenikb`.

PINEY_VOLUME=quarantine is required. A case lays a party out as
tools/test_battle_corbenik_rs.py does, builds the second form with the
game's own constructor (ccBoss08_b::ccBoss08_b, its five roots inside it)
at BOSS and runs ccBoss08_b::Main frame after frame, the manager's pass
before each, with scripted affects (Kite's hits of types 1 and 3, a
skill's, the drain's 13, the stop's affects, a member down, the boss's HP
set to reach its thresholds, now and then an act forced). Compared each
frame: the acts, movement, place, target, HP, flags, clips, the form's own
members (stage, effects, barrier, its counts, the press's and root drain's
places, the random drive's), the roots, the elements and the press's
images, the effects' slots, the party, the calls, the menu, the blur,
newlib's rand() and ccRand's index.

What is not the game's here (beyond the first form's harness):
- The boss camera is a ccBossCam zeroed at InitBossCamera; SetMode and
  SetFreeCamPosView do to it what the game's do (mode 6's camera 2 is
  set silently), QuakeCam runs, CamMain does not (the battle's, not the
  boss's).
- A particle generator lives the case's `gen_life` frames
  (ccCheckParticleGenerator answers by when it was made).
- The WaveShock and the rings are stand-ins with the port's lives; the
  star dust, its bolts and laser rain, the transform and the drain run
  natively.

    PINEY_VOLUME=quarantine python3 tools/test_battle_corbenik_b_rs.py             the tests
    PINEY_VOLUME=quarantine python3 tools/test_battle_corbenik_b_rs.py bulk N [S]  N cases from seed S
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402

import test_battle_corbenik_rs as cb  # noqa: E402
import test_battle_enemy_ai_rs as eai  # noqa: E402
import test_battle_rs as rs  # noqa: E402
import test_battle_boss_rs as bb  # noqa: E402
from test_battle_boss_rs import rnd_member, s16, s32  # noqa: E402
from test_battle_rs import F_ONE, NEW, SCN, fbits, ser_char  # noqa: E402

BOSS = 0x01800000               # the ccBoss08_b (0xfa5e0 bytes, its roots inside)
BOSS_SIZE = 0xFA5E0
ROOT0, ROOT_SIZE = 0x294C0, 0x29360
ELEM0, AI0, ELEM_SIZE = 0xF75B0, 0xF8DB0, 0x180
CAM = 0x01AC2000                # the boss camera (a zeroed ccBossCam)
DENV = 0x01AC3000               # ccDrawEnv::active's stand-in
FAKE, MGR, STAGE, WM, ACAM, FONT, HEAP = cb.FAKE, cb.MGR, cb.STAGE, cb.WM, cb.ACAM, cb.FONT, cb.HEAP
PLW, PARTY, MENU = cb.PLW, cb.PARTY, cb.MENU

WAVE, RING = 0, 3
STAR, TRANSFORM, DRAIN, THUNDER = 28, 29, 30, 31


def eff_life(kind, a=None):
    return {WAVE: 45, RING: 20}[kind]


class Game(cb.Game):
    """The first form's machine with the second form's hooks."""

    def __init__(self):
        self.gen_made = {}
        self.gen_life = 1 << 30
        self.frame_no = 0
        super().__init__()
        sym = self.sym
        self.gens = {sym("MagicSquareGenerator") + 0x188: "ring", sym("MagicSquareGenerator"): "square",
                     sym("g_seedShotSmokeGen"): "smoke", sym("g_energyChargeGenerator"): "charge",
                     sym("g_transformGen"): "transform"}
        self.drain_gen = sym("g_rootDrainGen")
        self.drain_ff = sym("g_rootDrainFF")
        self.hooks_b()

    def hooks_b(self):
        m = self.m
        h = m.hooks
        out = self.out
        sym = self.sym

        def hook(name, fn):
            h[sym(name)] = fn

        nop = lambda mm, *a: 0  # noqa: E731
        for name in ("Init__5ccObjFP10ccObjChunkPP7ccCoordUi", "Draw__5ccObjFf", "__dt__5ccObjFv",
                     "ApplyClump__5ccAnmFP7ccClumpP8ccStream", "SetRenderState__7ccModelF20CC_RENDER_STATE_TYPEi",
                     "SetBlendType__5ccAnmFUc", "SetBlendType__5ccEffFi", "SetMatrix_PosRotXYZ__7ccCoordFPfPf",
                     # The laser rain's fog under each segment: its picture.
                     "SetFogBlend__9ccDrawEnvFfUi"):
            hook(name, nop)

        def init_cam(mm, a, *_):
            mm.mem[CAM:CAM + 0x150] = bytes(0x150)
            mm.store(a + 332, 4, CAM)
            mm.store(a + 336, 4, 1)
            mm.store(sym("cmn_bossCam__10ccIBossEff"), 4, CAM)
            return 0
        hook("InitBossCamera__6ccBossFff", init_cam)
        hook("SetMode__9ccBossCamFifPfP6ccChar", self.set_mode)

        def free_cam(mm, a, p, v, *_):
            out.append(["freecam"] + [mm.load(p + 4 * k, 4) for k in range(3)]
                       + [mm.load(v + 4 * k, 4) for k in range(3)])
            # SetFreeCamPosView (0x00363f20): only while ResetFlg is 5.
            if s32(mm.load(a + 4, 4)) == 5:
                mm.mem[a + 0x10:a + 0x20] = mm.mem[v:v + 16]
                mm.mem[a + 0x20:a + 0x30] = mm.mem[p:p + 16]
            return 0
        hook("SetFreeCamPosView__9ccBossCamFPfPf", free_cam)
        hook("EntryFlash2__8ccScFadeFiiiffff",
             lambda mm, f, a, b_, c, *_: out.append(["flash3", s32(a), s32(b_), -1, c & 0xFFFFFFFF]) or 0)
        hook("effRadiateSomething__FPfPffii", lambda mm, *a: out.append(["radiate"]) or 0)
        hook("effRadiateSomething2__FPfPffii", lambda mm, *a: out.append(["radiate2"]) or 0)
        hook("effSmoke__FPfPffiiUsUs", lambda mm, *a: out.append(["smoke"]) or 0)

        def after_image(mm, a, t, *_):
            if t:
                out.append(["afterimage", mm.load(t, 4), mm.load(t + 0x14, 4)])
            else:
                out.append(["afterimage"])
            return 0
        hook("EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T", after_image)
        hook("ccEntryFlyFontNum__FiiPfffi", lambda mm, k, n, *_: out.append(["flyfont", s32(k), s32(n)]) or 0)
        hook("ccBossEffWaveShockCreate__FPfPff", self.effect_b(WAVE))
        hook("ccBossEffAutoSamonRingCreate__FPfPfPfi", self.effect_b(RING))

        def gen(mm, a, param, ff1, *_):
            if param == self.drain_gen:
                name = "drain0"
            elif param == self.drain_gen + 0x38:
                name = "drain1a" if ff1 == self.drain_ff + 192 else "drain1b"
            elif param == self.drain_gen + 0x70:
                name = "drain2"
            else:
                name = self.gens.get(param, hex(param))
            out.append(["gen", name])
            self.gen_made[a] = self.frame_no
            return a
        hook("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParam"
             "P25ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam", gen)

        def check(mm, g, *_):
            if not g or g not in self.gen_made:
                return int(g != 0)
            return int(self.frame_no - self.gen_made[g] < self.gen_life)
        hook("ccCheckParticleGenerator__FP19ccParticleGenerator", check)

    def set_mode(self, mm, a, mode, sp, ch):
        """ccBossCam::SetMode(mode, f, sp, ch) (0x00363d70) on the zeroed
        camera, noted and done: ResetFlg is the mode; 5 locks the eye and
        keeps it, 6 puts it back on camera 2 (silently here, as the port's
        one CamMode 6) and clears ResetFlg."""
        mode = s32(mode)
        self.out.append(["cammode", mode])
        mm.store(a + 4, 4, mode)
        mm.store(a + 0xD0, 4, cvt_w(mm.f[12]))
        if sp:
            mm.store(a + 0xD4, 4, cvt_w(mm.load(sp + 8, 4)))
        ex = mm.load(a + 0x138, 1)
        cp = lambda d, s_: mm.mem.__setitem__(slice(a + d, a + d + 16), mm.mem[a + s_:a + s_ + 16])  # noqa: E731
        if mode == 3:
            mm.store(a + 0x138, 1, ex | 0x10)
        elif mode == 4:
            mm.store(a + 0x138, 1, ex & 1)
        elif mode == 5:
            mm.store(a + 0x138, 1, ex | 1)
            mm.store(a, 1, 1)
            cp(0x40, 0x10)
            cp(0x50, 0x20)
            cp(0x60, 0x30)
        elif mode == 6:
            mm.store(a + 0x138, 1, ex & 0x10)
            mm.store(a, 1, 0)
            cp(0x10, 0x40)
            cp(0x20, 0x50)
            self.cams[2] = [f32s(mm, a + 0x20, 4), f32s(mm, a + 0x10, 4)]
            mm.store(a + 4, 4, 0)
        elif mode in (7, 9):
            mm.store(a + 0x13C, 4, ch)
        elif mode in (8, 10):
            mm.store(a + 4, 4, 0)
            mm.store(a + 0x13C, 4, mm.load(PLW + 0x20, 4))
        return 0

    def effect_b(self, kind):
        m, out = self.m, self.out

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

    def effects(self):
        m = self.m
        out = []
        vt = {self.sym("__vt__22ccBossEffStarDustFlash"): STAR, self.sym("__vt__18ccBossEffTransform"): TRANSFORM,
              self.sym("__vt__14ccBossEffDrain"): DRAIN, self.sym("__vt__17ccBossEffThunder3"): THUNDER}
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                kind = self.effs[e][0] if e in self.effs else vt.get(m.load(e + 12, 4), -1)
                out.append([k, kind, m.load(e, 1)])
        return out


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    frames = rnd.randrange(800, 3000) if rnd.random() < 0.6 else rnd.randrange(3000, 6000)
    max_hp = c.data.bosses[8]["maxHP"]
    script = []
    nsk = len(c.data.skills)
    rough = rnd.random() < 0.5
    for f in sorted(rnd.sample(range(frames), min(frames // (8 if rough else 25), 400))):
        kind = rnd.choice((0, 0, 0, 9, 10, 13, 5, 6))
        who = rnd.randrange(1, nsk) if kind == 10 else 0
        p0 = rnd.randrange(-1, 900) if rnd.random() < 0.8 else rnd.randrange(900, 6000)
        if kind in (5, 6, 13):
            p0 = 0
        script.append((f, kind, who, p0))
    # The thresholds: half, a fifth, the barrier's count.
    if rnd.random() < 0.7:
        f = rnd.randrange(0, frames // 2)
        script.append((f, 14, 0, rnd.choice((max_hp // 2 + 10, max_hp // 5 + 10, max_hp // 6))))
    if rnd.random() < 0.2:
        script.append((rnd.randrange(0, frames), 2, 0, 0))
    if n > 1 and rnd.random() < 0.3:
        f = rnd.randrange(0, frames)
        who = rnd.randrange(1, n)
        script += [(f, 7, who, 1), (min(frames - 1, f + rnd.randrange(30, 600)), 7, who, 0)]
    if rnd.random() < 0.7:
        for _ in range(rnd.randrange(1, 4)):
            act = rnd.choice((3, 4, 4, 5, 5, 6, 7, 11, 14, 16, 17, 18, 20, 20, 21, 22, 22, 23, 24))
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
    case = {"party": party, "frames": frames, "script": script, "menus": menus, "center": center,
            "cam2": cam2, "gen_life": rnd.randrange(30, 400),
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}
    case.update(bb.blur_case(rnd))
    case.update(bb.draw_case(rnd))
    return case


def run_game(game, case):
    c, g, m = game.c, game.g, game.m
    c.reset()
    c.scene.new_at = NEW
    game.fake_at = FAKE
    game.heap_at = HEAP
    game.effs.clear()
    game.anms.clear()
    game.out.clear()
    game.gen_made.clear()
    game.gen_life = case["gen_life"]
    game.frame_no = 0
    game.stage_on = False
    game.center = case["center"]
    game.cams = {2: [list(case["cam2"][0]), list(case["cam2"][1])]}
    m.store(game.sym("camID"), 2, 1)
    bb.put_draw_camera(game, case, ACAM)
    m.mem[FONT:FONT + 0x100] = bytes(0x100)
    m.store(game.sym("font"), 4, FONT)
    m.mem[DENV:DENV + 0x200] = bytes(0x200)
    m.store(game.sym("active__9ccDrawEnv"), 4, DENV)
    m.store(game.sym("cmn_bossCam__10ccIBossEff"), 4, 0)
    # boss08BBlurParamCB's blurRad: from 0, as the port's statics start.
    m.store(game.sym("blurRad$1026"), 4, 0)
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
    m.call(game.sym("__ct__10ccBoss08_bFv"), (BOSS,))
    frames = [game_state(game, len(party))]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        game.frame_no = f
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        for sf, kind, who, p0 in case["script"]:
            if sf != f:
                continue
            if kind == 0:
                m.call(ea, (BOSS, kite, 1, p0 & 0xFFFF, 0, 0))
            elif kind == 2:
                m.call(ea, (BOSS, kite, 13, 0, 0, 0))
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
            elif kind == 13:
                m.store(BOSS + 0x7C, 2, 0)
                m.store(BOSS + 0x7E, 2, 0)
            elif kind == 14:
                m.store(BOSS + 0x70, 2, p0 & 0xFFFF)
        game.effects_pass()
        m.call(game.sym("Main__10ccBoss08_bFv"), (BOSS,))
        frames.append(game_state(game, len(party)))
    return frames


def f32s(m, a, n):
    return [m.load(a + 4 * k, 4) for k in range(n)]


def cvt_w(bits):
    """cvt.w.s: the float's integer part (toward zero)."""
    import struct
    v = struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]
    if v != v:
        return 0x7FFFFFFF
    return max(-(1 << 31), min((1 << 31) - 1, int(v))) & 0xFFFFFFFF


def root_state(game, k):
    m = game.m
    a = BOSS + ROOT0 + ROOT_SIZE * k
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    clip = game.anms.get(ld(0xD4), [None, 0])
    return ([ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)] + f32s(m, a + 0x40, 3)
            + f32s(m, a + 0x60, 3) + [ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x88), clip[0] or "",
                                      clip[1] >> 8, ld(0x29350, 4, True)])


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
    second = ([ld(0x2935C, 4, True), int(ld(0x29360) != 0), int(ld(0x29364) != 0), int(ld(0x29410) != 0),
               int(ld(0x29414) != 0), ld(0x29418, 4, True), ld(0x29430), int(ld(0x29434) != 0),
               int(ld(0x29438) != 0), ld(0x2943C, 4, True), ld(0x294B4, 4, True), int(ld(0x294B8) != 0),
               int(ld(0x294BC) != 0)] + f32s(m, a + 0x1B0, 3) + [ld(0x1C0)] + f32s(m, a + 0xFA5C0, 3)
              + [ld(0xFA5D0), ld(0xFA5D4), ld(0xFA5D8, 4, True), ld(0xFA5DC, 4, True), ld(0x29328, 4, True),
                 ld(0x2932C, 4, True), ld(0x20C, 4, True), ld(0x210, 4, True), ld(0x1C8)])
    elements = [[ld(ELEM0 + ELEM_SIZE * k + 64, 4, True), ld(ELEM0 + ELEM_SIZE * k + 68)]
                + f32s(m, a + ELEM0 + ELEM_SIZE * k, 3) for k in range(16)]
    ais = [[ld(AI0 + ELEM_SIZE * k + 356), ld(AI0 + ELEM_SIZE * k + 360), ld(AI0 + ELEM_SIZE * k + 68)]
           + f32s(m, a + AI0 + ELEM_SIZE * k, 3) for k in range(16)]
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
                  s32(ld(0x2BC)), s32(ld(0x29334)), int(ld(0x29338) != 0), ld(0x1A0, 2, True),
                  cb.listed(game, a)],
        "tr": [ld(0x88), ld(0x8C)],
        "anm": [clip[0] or "", clip[1] >> 8, m.load(anm + 0x9C, 2)],
        "second": second,
        "roots": [root_state(game, k) for k in range(5)],
        "elements": elements,
        "ai": ais,
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2)),
                 s16(m.load(MENU + 0x0C, 2))],
        "blur": blur_state(game),
        "bosscam": cam_state(game),
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def cam_state(game):
    """The boss camera as the form leaves it: lock, ResetFlg, ExLock, its
    view (+0x10) and eye (+0x20), the kept ones (+0x40, +0x50)."""
    m = game.m
    if m.load(game.sym("cmn_bossCam__10ccIBossEff"), 4) != CAM:
        return []
    return ([m.load(CAM, 1), s32(m.load(CAM + 4, 4)), m.load(CAM + 0x138, 1)] + f32s(m, CAM + 0x10, 3)
            + f32s(m, CAM + 0x20, 3) + f32s(m, CAM + 0x40, 3) + f32s(m, CAM + 0x50, 3))


def blur_state(game):
    blur = bb.blur_state(game, MGR)
    if blur[8] == hex(game.sym("boss08BBlurParamCB__FPfPfPUi")):
        blur[8] = "boss08b"
    return blur


def boss_char_pre(game):
    """The second form's ccChar as SetBaseParam leaves it (bossTbl row 8)."""
    ch = cb.boss_char_pre(game)
    row = dict(game.c.data.bosses[8])
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level = 8, row.get("level", 1)
    ch.real = list(row["elm"])
    ch.HP, ch.SP, ch.maxHP, ch.maxSP = row["maxHP"], row["maxSP"], row["maxHP"], row["maxSP"]
    ch.width = row.get("width", 0)
    return ch


def request(game, case, pre):
    b = game.c.b
    party = case["party"]
    chars = [ser_char(p, b) for p in party] + [ser_char(pre, b)]
    n = len(party)
    members = [k if k < n else -1 for k in range(3)]
    ids = [party[k].id if k < n else -1 for k in range(3)]
    kite = party[0]
    parts = (["corbenikb", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + [0, 0, 0, 0]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], case["gen_life"]]
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


def compare(game, case, label):
    pre = boss_char_pre(game)
    g_frames = run_game(game, case)
    p_frames = rs.ask([request(game, case, pre)])[0]
    for f, (a, b_) in enumerate(zip(g_frames, p_frames)):
        a = rs.norm(a)
        for k in a:
            if a[k] != b_.get(k):
                d = "\n  ".join(cb.diffs(a[k], b_.get(k))[:12])
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
            rnd = random.Random(8900 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=10100):
    rs.build()
    game = Game()
    acts = set()
    total = 0
    for k in range(n):
        rnd = random.Random(seed0 + k)
        frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        acts |= {fr["act"][0] for fr in frames}
        total += len(frames) - 1
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print(f"{n} cases, {total} frames; acts reached:", sorted(acts))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 10100)
    else:
        unittest.main()
