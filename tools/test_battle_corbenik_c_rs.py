#!/usr/bin/env python3
"""crates/piney-battle's Corbenik, third form (src/boss/corbenik/third.rs,
eye.rs, laser.rs) against Quarantine's boss08_c.cpp run natively in the
Rust VU0 machine (tools/eemu_rs.so), frame by frame with battle_probe's
`corbenikc`.

PINEY_VOLUME=quarantine is required. A case lays a party out as
tools/test_battle_corbenik_rs.py does, builds the third form with the
game's own constructor (ccBoss08_c::ccBoss08_c: its three killer eyes
inside it, its sixteen eyes new[]'d) at BOSS and runs ccBoss08_c::Main
frame after frame, the manager's pass before each, with scripted affects
(Kite's hits on the body and on the killer eyes, a skill's, the drain's
13, the stop's affects, a member down, the HP set to reach the half, now
and then an act forced). Compared each frame: the body's acts, movement,
place, target, HP, flags, clips and own members; each eye's and killer
eye's; the effects' slots, the party, the calls, the menu, the blur, the
boss camera, newlib's rand() and ccRand's index.

What is not the game's here (beyond the second form's harness):
- ccBoss::Draw and SetMatrix_PosRotZYX run natively (the camera's fade
  from activeCamPtr, a stand-in at the case's eye, game.area 1); the
  clip's draw only leaves its world matrix (+0) its local one (+0x40).
- DMY_dummy1 is a stand-in chunk with the file's place at +0x10
  (battle_probe `corbenikdummy`).
- ccBossEffExplode2Create and the arena's clump switch (0x0030aa40) only
  note themselves; the tera and reflected lasers, the eye birth and the
  charge's sparks run natively.

    PINEY_VOLUME=quarantine python3 tools/test_battle_corbenik_c_rs.py             the tests
    PINEY_VOLUME=quarantine python3 tools/test_battle_corbenik_c_rs.py bulk N [S]  N cases from seed S
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402

import test_battle_corbenik_rs as cb  # noqa: E402
import test_battle_corbenik_b_rs as cbb  # noqa: E402
import test_battle_enemy_ai_rs as eai  # noqa: E402
import test_battle_rs as rs  # noqa: E402
import test_battle_boss_rs as bb  # noqa: E402
from test_battle_boss_rs import rnd_member, s16, s32  # noqa: E402
from test_battle_rs import F_ONE, NEW, SCN, fbits, ser_char  # noqa: E402

BOSS = 0x01800000               # the ccBoss08_c (0xa4fb0 bytes, its killer eyes inside)
BOSS_SIZE = 0xA4FB0
KILLER0, KILLER_SIZE = 0x293A0, 0x293E0
EYE_SIZE = 0x293D0
SCRATCH = 0x01AC4000            # ccSys's matrix stack for SetMatrix_*
FAKE, MGR, STAGE, WM, ACAM, FONT, HEAP = cb.FAKE, cb.MGR, cb.STAGE, cb.WM, cb.ACAM, cb.FONT, cb.HEAP
PLW, PARTY, MENU = cb.PLW, cb.PARTY, cb.MENU
CAM, DENV = cbb.CAM, cbb.DENV

WAVE, RING = 0, 3
TERA, REFRECT = 32, 33
ROW = 9


def eff_life(kind, a=None):
    return {WAVE: 45, RING: 20}[kind]


def f32(bits):
    import struct
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


class Game(cbb.Game):
    """The second form's machine with the third form's hooks."""

    def __init__(self):
        super().__init__()
        self.dummy = rs.ask(["corbenikdummy"])[0]
        sym = self.sym
        energy = sym("g_energyChargeGenerator")
        self.gens.update({sym("g_generateEyeGen"): "generate_eye", energy + 0x70: "laser_charge",
                          energy + 0xE0: "tera_charge", 0x00515490: "tera_glow", 0x005154F0: "tera_fall"})
        self.dead_gen = sym("BossDeadGenerator")
        self.dead_ff = sym("BossDeadForceField")
        self.hooks_c()

    def hooks_c(self):
        m = self.m
        h = m.hooks
        out = self.out
        sym = self.sym

        def hook(name, fn):
            h[sym(name)] = fn

        nop = lambda mm, *a: 0  # noqa: E731
        unit = [F_ONE, 0, 0, 0, 0, F_ONE, 0, 0, 0, 0, F_ONE, 0, 0, 0, 0, F_ONE]

        def anm_ct(mm, a, *_):
            mm.mem[a:a + 0x110] = bytes(0x110)
            for k, w in enumerate(unit):
                mm.store(a + 4 * k, 4, w)
                mm.store(a + 0x40 + 4 * k, 4, w)
            mm.store(a + 0x8D, 1, 1)
            mm.store(a + 0x9C, 2, 256)
            self.anms[a] = [None, 0]
            return a
        hook("__ct__5ccAnmFv", anm_ct)
        # The clip's local matrix natively; ccBoss::Draw natively (its
        # camera fade), the clip's draw only leaving its world matrix.
        h.pop(sym("SetMatrix_PosRotZYX__7ccCoordFPfPf"), None)
        h.pop(sym("Draw__6ccBossFv"), None)

        def anm_draw(mm, a, *_):
            mm.mem[a:a + 0x40] = mm.mem[a + 0x40:a + 0x80]
            return 0
        hook("Draw__5ccAnmFv", anm_draw)
        for name in ("GetAmbient__9ccDrawEnvFPf", "SetAmbient__9ccDrawEnvFPf", "SleepDistantLight__9WORLD_MANFv"):
            hook(name, nop)

        base_chunk = h[sym("GetChunkAdrsF__8ccStreamFPCci")]

        def chunk(mm, s, n, *a):
            name = self.cstr(n)
            if name != "DMY_dummy1":
                return base_chunk(mm, s, n, *a)
            c = self.alloc(32, name)
            for k in range(3):
                mm.store(c + 16 + 4 * k, 4, self.dummy[k])
            mm.store(c + 28, 4, F_ONE)
            return c
        hook("GetChunkAdrsF__8ccStreamFPCci", chunk)

        base_gen = h[sym("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParam"
                         "P25ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam")]

        def gen(mm, a, param, ff1, *rest):
            if param == self.dead_gen:
                out.append(["gen", "dead0"])
                return a
            if param == self.dead_gen + 0x38:
                out.append(["gen", "dead1a" if ff1 == self.dead_ff + 0xC0 else "dead1b"])
                return a
            return base_gen(mm, a, param, ff1, *rest)
        hook("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParam"
             "P25ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam", gen)
        hook("ParticleKill__19ccParticleGeneratorFv", lambda mm, *a: out.append(["gen_kill"]) or 0)
        hook("ccBossEffExplode2Create__FPf", lambda mm, *a: out.append(["explode2"]) or 0xFFFFFFFF)
        h[0x0030AA40] = lambda mm, ea, on, *_: out.append(["clump", int(on == 1)]) or 0
        hook("EntryFade__8ccScFadeFiiiffff",
             lambda mm, f, t, mode, colour, *_: out.append(["fade", s32(t), s32(mode), colour & 0xFFFFFFFF]) or 0)
        for name in ("SetBlendTypeAfterImage__6ccBossFUc", "ChangeClut__5ccEffFP11ccClutChunkP11ccClutChunk",
                     "Draw__5ccObjFf", "Init__5ccObjFP10ccObjChunkPP7ccCoordUi", "SetBlendType__5ccAnmFUc"):
            hook(name, nop)

    def effects(self):
        m = self.m
        out = []
        vt = {self.sym("__vt__18ccBossEffTeraLaser"): TERA, self.sym("__vt__24ccBossEffRefrectionLaser"): REFRECT,
              self.sym("__vt__14ccBossEffDrain"): cbb.DRAIN}
        for k in range(1024):
            e = m.load(MGR + 52 + 4 * k, 4)
            if e:
                kind = self.effs[e][0] if e in self.effs else vt.get(m.load(e + 12, 4), -1)
                out.append([k, kind, m.load(e, 1)])
        return out


def killer_at(k):
    return BOSS + KILLER0 + KILLER_SIZE * k


def rnd_case(c, rnd):
    n = rnd.randrange(1, 4)
    party = [rnd_member(c, rnd, k) for k in range(n)]
    frames = rnd.randrange(800, 3000) if rnd.random() < 0.6 else rnd.randrange(3000, 6000)
    max_hp = c.data.bosses[ROW]["maxHP"]
    script = []
    nsk = len(c.data.skills)
    rough = rnd.random() < 0.5
    for f in sorted(rnd.sample(range(frames), min(frames // (8 if rough else 25), 400))):
        kind = rnd.choice((0, 0, 0, 9, 10, 13, 5, 6, 15, 15, 15))
        who = rnd.randrange(1, nsk) if kind == 10 else rnd.randrange(3) if kind == 15 else 0
        p0 = rnd.randrange(-1, 900) if rnd.random() < 0.8 else rnd.randrange(900, 6000)
        p1 = rnd.randrange(1, nsk) if kind == 15 and rnd.random() < 0.5 else 0
        if kind in (5, 6, 13):
            p0 = 0
        script.append((f, kind, who, p0, p1))
    if rnd.random() < 0.6:
        f = rnd.randrange(0, frames // 2)
        script.append((f, 14, 0, rnd.choice((max_hp // 2 + 10, max_hp // 2 - 10, 50)), 0))
    if rnd.random() < 0.2:
        script.append((rnd.randrange(0, frames), 2, 0, 0, 0))
    if n > 1 and rnd.random() < 0.3:
        f = rnd.randrange(0, frames)
        who = rnd.randrange(1, n)
        script += [(f, 7, who, 1, 0), (min(frames - 1, f + rnd.randrange(30, 600)), 7, who, 0, 0)]
    if rnd.random() < 0.3:
        for _ in range(rnd.randrange(1, 3)):
            script.append((rnd.randrange(0, frames), 16, rnd.randrange(3), rnd.randrange(1, 200), 0))
    if rnd.random() < 0.6:
        for _ in range(rnd.randrange(1, 4)):
            act = rnd.choice((1, 2, 3, 3, 4, 4, 5, 5, 6, 8, 14, 20, 20, 21))
            script.append((rnd.randrange(0, frames), 8, 0, act, 0))
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
    # The active camera: near, or far enough that the eyes and the body
    # fade (7000 out, over 600).
    r = 2500 if rnd.random() < 0.6 else 9000
    acam = [fbits(rnd.uniform(-r, r)), fbits(rnd.uniform(-r, r)), fbits(rnd.uniform(0, 1500)), F_ONE]
    case = {"party": party, "frames": frames, "script": script, "menus": menus, "center": center,
            "cam2": cam2, "acam": acam, "gen_life": rnd.randrange(30, 400),
            "rand": rnd.randrange(0, 1 << 63), "seed": rnd.randrange(1, 1 << 32), "mti": rnd.randrange(1, 624),
            "count": rnd.randrange(0, 1000)}
    case.update(bb.blur_case(rnd))
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
    m.mem[ACAM:ACAM + 0x70] = bytes(0x70)
    for k in range(4):
        m.store(ACAM + 4 * k, 4, case["acam"][k])
    m.store(game.sym("activeCamPtr"), 4, ACAM)
    # A field's game.area: ccBoss::Draw's fade from 7000 over 600.
    m.store(m.load(game.sym("game"), 4) + 0x14, 4, 1)
    m.mem[FONT:FONT + 0x100] = bytes(0x100)
    m.store(game.sym("font"), 4, FONT)
    m.mem[DENV:DENV + 0x200] = bytes(0x200)
    m.store(game.sym("active__9ccDrawEnv"), 4, DENV)
    m.store(game.sym("cmn_bossCam__10ccIBossEff"), 4, 0)
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
    m.store(sys_ + 604, 4, SCRATCH)
    game.out.clear()
    m.call(game.sym("__ct__10ccBoss08_cFv"), (BOSS,))
    eyes = m.load(BOSS + 656, 4)
    frames = [game_state(game, len(party), eyes)]
    menus = case["menus"]
    ea = game.sym("EntryAffect__6ccCharFP6ccCharssss")
    for f in range(case["frames"]):
        game.out.clear()
        game.frame_no = f
        m.store(sys_ + 0x358, 4, case["count"] + 1 + f)
        mt_now = [v for k, v in menus if k <= f]
        g.env.menu_type = mt_now[-1] if mt_now else -1
        for sf, kind, who, p0, p1 in case["script"]:
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
            elif kind == 15:
                m.call(ea, (killer_at(who % 3), kite, 1, p0 & 0xFFFF, p1 & 0xFFFF, 0))
            elif kind == 16:
                m.store(killer_at(who % 3) + 0x70, 2, p0 & 0xFFFF)
        game.effects_pass()
        m.call(game.sym("Main__10ccBoss08_cFv"), (BOSS,))
        frames.append(game_state(game, len(party), eyes))
    return frames


f32s = cbb.f32s


def target_of(n, tgt):
    if tgt == 0:
        return -1
    if tgt == BOSS:
        return n
    if SCN <= tgt < SCN + 0x1000 * 3:
        return (tgt - SCN) // 0x1000
    return -2


def eye_state(game, a):
    m = game.m
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    return ([ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1D0, 1, True)] + f32s(m, a + 0x40, 3)
            + f32s(m, a + 0x60, 3) + f32s(m, a + 0x29350, 3) + f32s(m, a + 0x29380, 3)
            + [ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x88), ld(0x8C), ld(0x293B0)]
            + f32s(m, a + 0x293A0, 3) + f32s(m, a + 0x293C0, 3) + [ld(0x293B4), ld(0x293B8)]
            + f32s(m, a + 0x1B0, 3))


def killer_state(game, a, n):
    m = game.m
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    clip = game.anms.get(ld(0xD4), [None, 0])
    return ([ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)] + f32s(m, a + 0x40, 3)
            + f32s(m, a + 0x50, 3) + f32s(m, a + 0x60, 3)
            + [ld(0x70, 2, True), ld(0x74, 2, True), ld(0x1D4, 1, True), ld(0x1D5, 1, True), ld(0x1D6, 1, True),
               ld(0x1D0, 1, True), ld(0x88), ld(0x8C), ld(0x188), ld(0x18C)] + f32s(m, a + 0x190, 3)
            + [target_of(n, ld(0x78)), cb.listed(game, a), int(ld(0x29350) != 0)] + f32s(m, a + 0x293B0, 3)
            + [ld(0x293C0), ld(0x293C4), clip[0] or "", clip[1] >> 8, ld(0x29328, 4, True), ld(0x20C, 4, True)]
            + f32s(m, a + 0x1B0, 3))


def game_state(game, n, eyes):
    m = game.m
    a = BOSS
    ld = lambda o, s=4, sg=False: m.load(a + o, s, sg)  # noqa: E731
    anm = ld(0xD4)
    clip = game.anms.get(anm, [None, 0])
    party = []
    for k in range(n):
        va = SCN + 0x1000 * k
        party.append([s16(m.load(va + 0x70, 2)), s16(m.load(va + 10, 2)), s16(m.load(va + 8, 2))])
    third = ([ld(0xA4F44, 4, True), ld(0xA4F48, 4, True), int(ld(0xA4F4C) != 0)] + f32s(m, a + 0xA4F60, 3)
             + f32s(m, a + 0xA4F70, 3) + [int(ld(0xA4F80) != 0), ld(0xA4F88, 4, True), ld(0xA4F8C, 4, True),
                                          int(ld(0xA4F90) != 0)]
             + f32s(m, a + 0xA4FA0, 3) + f32s(m, a + 0x1B0, 3)
             + [[int(ld(0x29358 + 4 * k) != 0) for k in range(16)]]
             + [ld(0x29328, 4, True), ld(0x2932C, 4, True), ld(0x20C, 4, True), ld(0x210, 4, True)])
    return {
        "act": [ld(0x1A2, 2, True), ld(0x1A4, 2, True), ld(0x1A6, 2, True)],
        "move": [ld(0x188), ld(0x18C)] + f32s(m, a + 0x190, 3),
        "tgt": [target_of(n, ld(0x78)), ld(0x180), ld(0x184)] + f32s(m, a + 0x160, 3),
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
        "third": third,
        "eyes": [eye_state(game, eyes + EYE_SIZE * k) for k in range(16)],
        "killers": [killer_state(game, killer_at(k), n) for k in range(3)],
        "eff": game.effects(),
        "party": party,
        "out": list(game.out),
        "menu": [s16(m.load(MENU + 0xFE, 2)), s16(m.load(MENU + 0x100, 2)), s16(m.load(MENU + 0x26, 2)),
                 s16(m.load(MENU + 0x14, 2)), s16(m.load(MENU + 0xF0, 2)), s16(m.load(MENU + 0xF2, 2)),
                 s16(m.load(MENU + 0x0C, 2))],
        "blur": cbb.blur_state(game),
        "bosscam": cbb.cam_state(game),
        "rand": game.g.rand_now(),
        "cc": s32(m.load(eai.MTI, 4)),
    }


def boss_char_pre(game):
    """The third form's ccChar as SetBaseParam leaves it (bossTbl row 9)."""
    ch = cb.boss_char_pre(game)
    row = dict(game.c.data.bosses[ROW])
    ch.row = row
    ch.type = row.get("type", 0x80)
    ch.id, ch.level = ROW, row.get("level", 1)
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
    parts = (["corbenikc", len(chars)] + chars + [n] + list(range(n)) + members + ids + [n]
             + case["center"][:3] + list(kite.pos) + [0, 0, 0, 0]
             + [case["rand"], case["seed"], case["mti"], case["count"], case["frames"], case["gen_life"]]
             + list(case["cam2"][0]) + list(case["cam2"][1]) + list(case["acam"]))
    parts.append(len(case["script"]))
    for s in case["script"]:
        parts += list(s)
    parts.append(len(case["menus"]))
    for mm in case["menus"]:
        parts += list(mm)
    parts += bb.blur_tokens(case)
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
            rnd = random.Random(9100 + seed)
            compare(self.game, rnd_case(self.game.c, rnd), f"case {seed}")


def bulk(n, seed0=10300):
    rs.build()
    game = Game()
    acts = set()
    kacts = set()
    total = 0
    for k in range(n):
        rnd = random.Random(seed0 + k)
        frames = compare(game, rnd_case(game.c, rnd), f"case {seed0 + k}")
        acts |= {fr["act"][0] for fr in frames}
        kacts |= {kl[0] for fr in frames for kl in fr["killers"]}
        total += len(frames) - 1
        print(f"case {seed0 + k} ok ({len(frames) - 1} frames)", flush=True)
    print(f"{n} cases, {total} frames; acts reached:", sorted(acts), "killer acts:", sorted(kacts))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 10300)
    else:
        unittest.main()
