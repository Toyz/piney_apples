#!/usr/bin/env python3
"""crates/piney-effect's boss effects (src/boss.rs) against the game's own
ccBossEff*Create and Draw (gcmn bosseff.cpp) run in eemu.

The boss_probe example makes the effects Skeith makes and runs the
manager's pass frame by frame; the same goes through the game on
tools/test_effect_rs.py's EffectMachine (main and GCMN.PRG, the effect
files and xeffect served from DATA.BIN, what the effects draw recorded):

  - each case calls the real Create with its arguments, then runs
    ccBossEffManager::Draw's loop over _g_bossEffManager's 1024 slots (the
    SetActiveLayer(3), each slot's Draw through its vtable, a disabled one
    deleted through its destructor) until every slot is free;
  - recorded each frame and compared with the port's: the draws in order
    (ccAnm and its step, ccClump with its transparency, ccEff with its
    place, scale, turn, colour, transparency, CLUT and pattern; the matrix
    each is drawn at, the layer), the sounds (ccSeOn3DNote, ccSeOnNote,
    ccSeOn3D), the screen flashes (ccScFade::EntryFlash), the particle
    generators started (the parameter row and force fields by address,
    distSW, syncPosType, pTexMod, pos, offset, and syncPos as the photon it
    follows) and each one's killFlag (ParticleKill), the omni light
    (ccBossEffLight's: in the light group or not, its place, its colour),
    each slot's m_bEnabled, the ccEffect slots effIceRock fills, rand()'s
    state and how many genrand() were drawn.

Stand-ins in the game: the particle system does not run
(startParticleGenerator and ParticleKill are recorded;
ccCheckParticleGenerator answers whether a generator was started, as the
port's list, not stepped, keeps it; no ccParticleCtrl is built, so
ccParticleSetup finds no particle and the probe's has none free either);
genrand() (behind ccRand and ccRandF)
answers a 32-bit LCG from 0x12345678, as the probe's host does (counted);
cameraGetRot reads camera 0 as the probe's host gives it; ccClump::Duplicate
and ChangeClut are recorded, not run.

Fidchell's spells (Outbreak's disc only): the meteors, the thunder storm
and the rock towers, whose rules are piney-battle's (the probe's manager
runs them on its own generators) and whose pictures are piney-effect's
fidchell.rs; the magic squares 1 and 2 with their lights (modes 1 and 2),
and the swarm's omni light (its place, colour, intensity and fall-off).

test_skeiths_trail drives the game's ccLattice (the cross's trail,
ccBoss01::DrawCross) against piney_world::lattice through the probe's
lattice requests; its own docstring lists its stand-ins.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_effect_rs as tfx  # noqa: E402
from test_effect_rs import ELF, ISO, ONE, TARGET, EffectMachine, fb, hexs  # noqa: E402

PROFILE = os.environ.get("PINEY_PROFILE", "debug")
EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "boss_probe")
MGR = 0x01A00000                # _g_bossEffManager (0x1038 bytes)
SCFADE = 0x01A02000             # a stand-in ccScFade
MGR_P, SCFADE_P = inf_va(0x00378BBC), inf_va(0x00378968)
LIGHT_VT = inf_va(0x00376700)           # ccBossEffLight's vtable
BRIGHT_VT = inf_va(0x00376640)          # ccBossEffBrightMagicSquare's
METEO_VT = inf_va(0x00376690)           # ccBossEffMeteoSworm's


def next_genrand(w):
    """The probe's stand-in genrand: a 32-bit LCG."""
    return (w * 1664525 + 1013904223) & 0xFFFFFFFF


def build():
    tfx.build("boss_probe")


def ask(lines):
    return tfx.ask(lines, EXAMPLE)


class BossFx(EffectMachine):
    """EffectMachine with xeffect and the boss effect manager."""

    def __init__(self, seed=1):
        super().__init__(town=False, seed=seed)
        import anim
        import ccs
        import gzarc
        m, sym = self.m, self.sym
        data = gzarc.open_bytes(tfx.DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        c = ccs.Ccs(gzarc.inflate(data, members["xeffect.cmp"]))
        self.files.append("xeffect")
        self.ccs["xeffect"] = c
        for _, a in anim.animations(c):
            self.anims.setdefault(c.objects[a.object][0], a)
        self.effs.update(tfx.eff_chunks(c))
        m.mem[MGR:MGR + 0x1038] = bytes(0x1038)
        m.store(MGR_P, 4, MGR)
        m.store(SCFADE_P, 4, SCFADE)
        self.ccrand = 0
        self.word = 0x12345678
        self.clump_alpha = ONE
        self.started, self.killed, self.deleted, self.grp = [], set(), set(), set()
        # Kyvia's meteors' Create: the first trail's meteor place (the
        # array's start) while it makes them.
        self.meteor_base = None
        self.clut = {}

        def note(mm, n, pos, v, *a):
            self.events.append(["se3dnote", n, self.rvec(pos), v & 0xFF])
            return 0

        def senote(mm, n, v, *a):
            self.events.append(["senote", n, v & 0xFF])
            return 0

        def flash(mm, fade, t, color, *a):
            self.events.append(["flash", t, color & 0xFFFFFFFF, [mm.f[12 + k] for k in range(4)]])
            return 0

        def genrand(mm, *a):
            self.ccrand += 1
            self.word = next_genrand(self.word)
            return self.word

        def kill(mm, g, *a):
            self.killed.add(g)
            return 0

        m.hooks[sym("ccSeOn3DNote__FiPfc")] = note
        m.hooks[sym("ccSeOnNote__Fic")] = senote
        m.hooks[sym("EntryFlash__8ccScFadeFiiffff")] = flash
        m.hooks[sym("genrand__Fv")] = genrand
        m.hooks[sym("ccSeOn__Fi")] = lambda mm, n, *a: self.events.append(["se", n]) or 0
        m.hooks[sym("ParticleKill__19ccParticleGeneratorFv")] = kill
        m.hooks[sym("ParticleDelete__19ccParticleGeneratorFv")] = lambda mm, g, *a: self.deleted.add(g) or 0
        m.hooks[sym("ccCheckParticleGenerator__FP19ccParticleGenerator")] = lambda mm, g, *a: int(g in self.started)
        m.hooks[sym("AddGrp__10ccLightGrpFP7ccLight")] = lambda mm, grp, l, *a: self.grp.add(l) or 0
        m.hooks[sym("DelGrp__10ccLightGrpFP7ccLight")] = lambda mm, grp, l, *a: self.grp.discard(l) or 0
        m.hooks[sym("Duplicate__7ccClumpFUi")] = lambda mm, *a: 0

        def change_clut(mm, clump, a, b, *r):
            # ChangeClut(new, old); with no old (Fidchell's bolt: its own
            # palette again) nothing the draws compare.
            if b:
                self.clut[clump] = (self.names.get(a, "?"), self.names.get(b, "?"))
            return 0

        m.hooks[sym("ChangeClut__7ccClumpFP11ccClutChunkP11ccClutChunk")] = change_clut

        # Innis's ice burst: its explosion's substitutes stand in as bare
        # objects with no model, so no palette is changed here (their
        # names are piney-effect's lib test's).
        def subst(mm, anm, name, *a):
            return self.malloc(mm, 0xA0)

        m.hooks[sym("GetSubstAdrsF__5ccAnmFPCcb")] = subst
        m.hooks[sym("Duplicate__5ccObjFUi")] = lambda mm, *a: 0
        self.meteor_vt = sym("__vt__25ccBossEffMeteoriteMissile")
        self.missile_vts = {sym("__vt__19ccBossEffIceMissile"), sym("__vt__25ccBossEffLightningMissile"),
                            sym("__vt__21ccBossEffBlazeMissile")}

        def coord(mm, a, *r):
            # ccCoord::ccCoord (main 0x00138000): both matrices the unit,
            # no parent, +0x84 and +0x88 1.0, the dirty flag set.
            for base in (a, a + 0x40):
                for i in range(4):
                    for j in range(4):
                        mm.store(base + 16 * i + 4 * j, 4, ONE if i == j else 0)
            mm.store(a + 0x80, 4, 0)
            mm.store(a + 0x84, 4, ONE)
            mm.store(a + 0x88, 4, ONE)
            mm.store(a + 0x8C, 1, 0)
            mm.store(a + 0x8D, 1, 1)
            return a

        m.hooks[sym("__ct__7ccCoordFv")] = coord
        m.hooks[sym("__ct__5ccAnmFv")] = coord

    def start_generator(self, m, g, *a):
        self.started.append(g)
        sync = m.load(g + 0x18, 4)
        ref = None
        if sync:
            # a photon's pos: its effect (slot) and its index
            for k in range(1024):
                e = m.load(MGR + 52 + 4 * k, 4)
                if e and m.load(e + 12, 4) == BRIGHT_VT:
                    ph = m.load(e + 0x20, 4)
                    if ph <= sync < ph + 0x60 * m.load(e + 0x4C, 4):
                        ref = ["anchor", k, (sync - 0x10 - ph) // 0x60]
            # a ccEffect's pos (effSmokeRock's rocks)
            if ref is None and tfx.EFFWORK <= sync < tfx.EFFWORK + 0xC0 * 500:
                k, off = divmod(sync - tfx.EFFWORK, 0xC0)
                ref = ["effpos", k] if off == 0 else ["?", sync]
            # one of Innis's missiles (its pos at +0x10): its slot, or the
            # next free one while its Create makes it
            obj = sync - 0x10
            if ref is None and m.load(obj + 12, 4) in self.missile_vts:
                held = [k for k in range(1024) if self.slot_ptr(k) == obj]
                ref = ["missile", held[0] if held else next(k for k in range(1024) if not self.slot_ptr(k))]
            # one of Kyvia's meteors' places (its `now` array at +0x60), by
            # its slot, or while its Create makes it the next free one
            if ref is None:
                for k in range(1024):
                    e = self.slot_ptr(k)
                    if e and m.load(e + 12, 4) == self.meteor_vt:
                        now, n = m.load(e + 0x60, 4), m.load(e + 0x84, 4)
                        if now <= sync < now + 16 * n:
                            ref = ["meteor", k, (sync - now) // 16]
                if ref is None and self.meteor_base is not None:
                    if self.meteor_base == -1:
                        self.meteor_base = sync
                    free = next(k for k in range(1024) if not self.slot_ptr(k))
                    ref = ["meteor", free, (sync - self.meteor_base) // 16]
            # made in the Create before the manager holds it: the next free slot
            if ref is None:
                ref = ["anchor", -1, sync]
        flags = m.load(g + 4, 1)
        self.gens.append([m.load(g, 4), [m.load(m.load(g + 0x34 + 4 * k, 4), 4) if m.load(g + 0x34 + 4 * k, 4) else 0 for k in range(4)], flags & 1,
                          (flags >> 4) & 1, m.load(g + 0x2C, 2, True), self.rvec(g + 0x60), self.rvec(g + 0x80),
                          ref])
        return 0

    def eff_draw(self, m, eff, pat, *a):
        super().eff_draw(m, eff, pat, *a)
        d = self.draws[-1]
        clut = m.load(eff + 0x3C, 4)
        d.insert(9, self.names.get(clut, "") if clut else "")
        return 0

    def clump_draw(self, m, clump, *a):
        # its ChangeClut(to, from) as (from, to), or none
        clut = list(reversed(self.clut[clump])) if clump in self.clut else None
        self.draws.append(["clump", self.clump[clump], self.rvec(clump + 0x40, 16), self.clump_alpha, self.layer(),
                           clut])
        return 0

    def slot_ptr(self, k):
        return self.m.load(MGR + 52 + 4 * k, 4)

    def create(self, name, ints=(), floats=()):
        self.events, self.gens, self.draws = [], [], []
        k = self.call(name, *ints, fargs=[fb(x) for x in floats])
        return k - (1 << 32) if k & 0x80000000 else k

    def manager_pass(self):
        """ccBossEffManager::Draw's loop: SetActiveLayer(3), each slot."""
        m = self.m
        self.events, self.gens, self.draws = [], [], []
        self.call("SetActiveLayer__9WORLD_MANFi", tfx.WM, 3)
        for k in range(1024):
            e = self.slot_ptr(k)
            if not e:
                continue
            vt = m.load(e + 12, 4)
            if not m.load(e, 1):
                self.call(m.load(vt + 12, 4), e, 1)
                m.store(MGR + 52 + 4 * k, 4, 0)
            else:
                self.call(m.load(vt + 8, 4), e)
        lights, enabled = [], []
        for k in range(1024):
            e = self.slot_ptr(k)
            if not e:
                continue
            enabled.append([k, m.load(e, 1)])
            if m.load(e + 12, 4) == LIGHT_VT:
                lt = m.load(e + 0x9C, 4)
                lights.append([int(lt in self.grp), self.rvec(lt + 0x70, 3), self.rvec(lt + 0xB0)])
            # A meteor swarm's m_light while it has one.
            if m.load(e + 12, 4) == METEO_VT and m.load(e + 0x30, 4):
                lt = m.load(e + 0x30, 4)
                lights.append(["meteo", int(lt in self.grp), self.rvec(lt + 0x70, 3), self.rvec(lt + 0xB0)]
                              + self.rvec(lt + 0xC0, 3))
        slots = []
        for s in self.live():
            slots.append([s["i"], s["id"], s["life"], s["pos"], s["scale"], s["speed"], s["velocity"],
                          s["rotSpeed"], s["sn"]])
        return {"draws": self.draws, "events": self.events, "gens": self.gens,
                "kills": [3 if g in self.deleted else 2 if g in self.killed else 0 for g in self.started],
                "lights": lights,
                "enabled": enabled, "slots": slots, "rand": self.rand.s, "ccrand": self.ccrand}

    def free(self):
        return all(not self.slot_ptr(k) for k in range(1024))

    def set_field(self, n):
        self.m.store(tfx.GAME + 0x24, 4, n)


def vec_args(em, *vs):
    out = []
    for v in vs:
        a = em.malloc(em.m, 16)
        em.vec(a, list(v))
        out.append(a)
    return out


# The cases: (name, probe line, the Create's symbol, its vector arguments
# (bits), its ints, its floats (bits)).
def cases(rnd):
    def pos():
        return [fb(rnd.uniform(-3000, 3000)), fb(rnd.uniform(-3000, 3000)), fb(rnd.uniform(-50, 300)), ONE]
    out = []
    for _ in range(2):
        p, d = pos(), [0, 0, fb(rnd.uniform(-3, 3)), 0]
        out.append(("wave", "wave %s %s %x" % (hexs(*p), hexs(*d), fb(1.0)),
                    "ccBossEffWaveShockCreate__FPfPff", [p, d], [], [1.0]))
        p = pos()
        out.append(("square", "square %s 0" % hexs(*p), "ccBossEffMagicSquareCreate__FPfi", [p], [0], []))
        p = pos()
        p[2] = fb(1000.0)
        r = [fb(1.5707963705062866), 0, 0, ONE]
        out.append(("force", "force %s %s %s 8 64 8" % (hexs(*p), hexs(*r), hexs(fb(10.0), fb(200.0), fb(200.0))),
                    "ccBossEffForceGeneratorCreate__FPfPffffiii", [p, r], [8, 100, 8], [10.0, 200.0, 200.0]))
        p, r = pos(), [0, 0, fb(rnd.uniform(-3, 3)), 0]
        q = [0, fb(1.0 / 3), fb(10.0), 0]
        out.append(("ring", "ring %s %s %s 23" % (hexs(*p), hexs(*r), hexs(*q)),
                    "ccBossEffAutoSamonRingCreate__FPfPfPfi", [p, r, q], [35], []))
        p = pos()
        out.append(("ice", "ice %s %x" % (hexs(*p), fb(2.0)), "ccBossEffIceBreakCreate__FPff", [p], [], [2.0]))
        p = pos()
        out.append(("dead", "dead %s" % hexs(*p), "ccBossEffDeadCreate__FPfPfi", [p, p], [0], []))
    return out


def fidchell_cases(rnd):
    """Fidchell's spells as OnMeteoSworm, OnThunderStorm and OnGroundQuake
    make them (50 meteors from 3000 up to within 500, 16 thunders 200 round,
    128 towers), and its magic squares 1 and 2 (1 in fields 4 and 2)."""
    def pos(z=0.0):
        return [fb(rnd.uniform(-3000, 3000)), fb(rnd.uniform(-3000, 3000)), fb(z), ONE]
    out = []
    t = pos(rnd.uniform(-20, 60))
    sp = [fb(f32(t[0]) + rnd.uniform(-900, 900)), fb(f32(t[1]) + rnd.uniform(-900, 900)), fb(3000.0), ONE]
    out.append(("meteo", "meteo %s %s 32 %x %x" % (hexs(*sp), hexs(*t), fb(500.0), fb(50.0)),
                "ccBossEffMeteoSwormCreate__FPfPfiffPA4_f", [sp, t, [0, 0, 0, ONE]],
                [("vec", 0), ("vec", 1), 50, ("vec", 2)], [500.0, 50.0], None))
    p = pos(rnd.uniform(-20, 60))
    out.append(("storm", "storm %s %x 10" % (hexs(*p), fb(200.0)), "ccBossEffThunderStormCreate__FPffi",
                [p], [16], [200.0], None))
    p = pos(rnd.uniform(-20, 60))
    out.append(("tower", "tower %s 80" % hexs(*p), "ccBossEffRockTowerCreate__FPfi", [p], [128], [], None))
    for n, field in ((1, 4), (1, 2), (2, 4)):
        p = pos(rnd.uniform(-20, 60))
        out.append(("square %d" % n, "square %s %x" % (hexs(*p), n), "ccBossEffMagicSquareCreate__FPfi", [p], [n],
                    [], field))
    return out


def innis_cases(rnd):
    """Innis's missiles as IceAttack, ThunderAttack and BlazeAttack make
    them (Innis's place, two spline points, the target 10 up), each
    element as the first (NO 1) and a later one (0), with no callback and
    no camera; and the rings EnemyBurst and AllAttack make (models 191-193,
    their burst's scale step 0.5 and fade 0.04 or 0.02)."""
    out = []
    for el in range(3):
        for no in (1, 0):
            boss = [fb(rnd.uniform(-800, 800)), fb(rnd.uniform(-800, 800)), fb(1500.0), ONE]
            t = [fb(rnd.uniform(-800, 800)), fb(rnd.uniform(-800, 800)), fb(10.0), ONE]
            v2 = [fb(f32(boss[0]) + rnd.uniform(-1700, 1700)), fb(f32(boss[1]) + rnd.uniform(-1700, 1700)),
                  fb(rnd.uniform(500, 900)), ONE]
            v3 = [fb(f32(t[0]) + rnd.uniform(-600, 600)), fb(f32(t[1]) + rnd.uniform(-600, 600)),
                  fb(rnd.uniform(800, 1200)), ONE]
            ctrl = boss + v2 + v3 + t
            sym = ["ccBossEffIceMissileCreate", "ccBossEffLightningMissileCreate", "ccBossEffBlazeMissileCreate"][el]
            out.append(("missile %d %d" % (el, no), "missile %x %x %s" % (el, no, hexs(*ctrl)),
                        sym + "__FPA4_fiPFPfi_iP9ccBossCam", ctrl, [no, 0, 0], None))
    for model, tpoint in ((191, 0.04), (192, 0.02), (193, 0.04)):
        p = [fb(rnd.uniform(-800, 800)), fb(rnd.uniform(-800, 800)), fb(rnd.uniform(0, 300)), ONE]
        r = [fb(rnd.uniform(-1, 1)), fb(rnd.uniform(-1, 1)), fb(rnd.uniform(-3, 3)), 0]
        sc = [fb(1.5)] * 3 + [ONE]
        sp = [fb(0.5)] * 4
        out.append(("ring %d" % model, "sring %s %s %s %x %s %x" % (hexs(*p), hexs(*r), hexs(*sc), model, hexs(*sp),
                                                                     fb(tpoint)),
                    "ccBossEffSamonRingCreate__FPfPfPfi", None, [p, r, sc, model], (sp, tpoint)))
    return out


def kyvia_cases(rnd):
    """Kyvia's meteors as MegidFlame makes them (the disc's place, range
    500, seven) with no callback and no camera; and on Outbreak its bolts as
    ThunderboltAtk makes them: each kind's on the disc (raised 500 by the
    offset the caller writes) and a member's (no smoke, no offset)."""
    out = []
    for _ in range(2):
        pos = [fb(rnd.uniform(-800, 800)), fb(rnd.uniform(-800, 800)), fb(rnd.uniform(0, 200)), ONE]
        line = "meteorite %s %x 7" % (hexs(*pos), fb(500.0))
        out.append(("meteorite", line, "ccBossEffMeteoriteMissileCreate__FPA4_ffiiPFPf_iP9ccBossCam",
                    ("meteor", pos, 500.0, 7)))
    if volume.NAME != "outbreak":
        return out
    half = fb(0.5)
    member = (fb(0.5), fb(0.9), 2, 3, [fb(10.0)] * 3 + [half], 0, 50, 3)
    kinds = [(fb(1.0), 1, 4, [fb(50.0)] * 3 + [half], 3, 100, 90),
             (fb(3.14159274101), 2, 5, [fb(20.0), fb(30.0), fb(50.0), ONE], 4, 550, 90),
             (fb(3.14159274101), 3, 10, [fb(50.0)] * 3 + [ONE], 15, 450, 120)]
    for ra, ds, rs_, rp, num, rng, time in kinds:
        pos = [fb(rnd.uniform(-800, 800)), fb(rnd.uniform(-800, 800)), fb(rnd.uniform(0, 200)), ONE]
        off = [0, 0, fb(500.0), ONE]
        dat = (0, ra, ds, rs_, rp, 1, rng, num)
        out.append(("bolt %d" % num, bolt_line(pos, time, dat, off), "ccBossEffThunderboltCreate__FPfiisP11ThunderData",
                    ("bolt", pos, time, dat, off)))
        mpos = [fb(rnd.uniform(-800, 800)), fb(rnd.uniform(-800, 800)), 0, ONE]
        da, ra2, ds2, rs2, rp2, sw, rng2, num2 = member
        mdat = (da, ra2, ds2, rs2, rp2, sw, rng2, num2)
        out.append(("bolt member", bolt_line(mpos, time, mdat, None), "ccBossEffThunderboltCreate__FPfiisP11ThunderData",
                    ("bolt", mpos, time, mdat, None)))
    return out


def bolt_line(pos, time, dat, off):
    """The probe's `bolt`: pos, time, range, num, the ThunderData (angle,
    its spread, scale, its spread, mode 1, the effect switch, the points'
    spread), the offset (zeros for none)."""
    da, ra, ds, rs_, rp, sw, rng, num = dat
    return "bolt %s %x %x %x %x %x %x %x 1 %x %s %s" % (hexs(*pos), time, rng, num, da, ra, ds, rs_, sw, hexs(*rp),
                                                       hexs(*(off or [0, 0, 0, 0])))


def f32(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class BossEffectsAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def run_case(self, seed, case, camera):
        """One Create, then the manager's passes until every slot is free.
        A case is (name, probe line, symbol, vectors, ints, floats) and,
        for Fidchell's, the game's field (or None). The Create's arguments
        are the vectors' addresses, then the ints; an int ("vec", k) puts
        vector k's address there instead (the ints are then all of them)."""
        name, line, sym, vecs, ints, floats = case[:6]
        field = case[6] if len(case) > 6 else None
        em = BossFx(seed=seed)
        eye, view = camera
        em.vec(tfx.EYE, [fb(v) for v in eye] + [ONE])
        em.vec(tfx.EYE + 0x10, [fb(v) for v in view] + [ONE])
        ptrs = vec_args(em, *vecs)
        if any(isinstance(x, tuple) for x in ints):
            args = [ptrs[x[1]] if isinstance(x, tuple) else x for x in ints]
        else:
            args = ptrs + list(ints)
        pre = []
        if field is not None:
            em.set_field(field)
            pre.append("field %x" % field)
        k = em.create(sym, *[], ints=args, floats=floats)
        made = {"id": k, "events": em.events, "gens": em.gens, "ccrand": em.ccrand}
        frames = []
        while not em.free() and len(frames) < 400:
            frames.append(em.manager_pass())
        lines = ["reset %x" % seed, "camera " + hexs(*map(fb, eye + view))] + pre + [line] + ["frame"] * len(frames)
        got = [g for g in ask(lines) if g]
        self.assertEqual(got[0], made, "%s: the Create" % name)
        for f, (want, port) in enumerate(zip(frames, got[1:])):
            for key in want:
                self.assertEqual(want[key], port.get(key), "%s frame %d: %s" % (name, f, key))
        self.assertEqual(len(got[1:]), len(frames))
        return frames

    def run_script(self, seed, script, camera, frames_max=420):
        """Creates made between the manager's passes: `script` {frame: [case,
        ...]}, each made before that frame's pass; then passes until every
        slot is free."""
        em = BossFx(seed=seed)
        eye, view = camera
        em.vec(tfx.EYE, [fb(v) for v in eye] + [ONE])
        em.vec(tfx.EYE + 0x10, [fb(v) for v in view] + [ONE])
        lines = ["reset %x" % seed, "camera " + hexs(*map(fb, eye + view))]
        want = []
        f = 0
        while f < frames_max and (f <= max(script) or not em.free()):
            for name, line, sym, vecs, ints, floats in script.get(f, []):
                args = vec_args(em, *vecs) + list(ints)
                k = em.create(sym, ints=args, floats=floats)
                want.append(("create %s" % name, {"id": k, "events": em.events, "gens": em.gens,
                                                  "ccrand": em.ccrand}))
                lines.append(line)
            want.append(("frame %d" % f, em.manager_pass()))
            lines.append("frame")
            f += 1
        got = [g for g in ask(lines) if g]
        self.assertEqual(len(got), len(want))
        for (what, w), g in zip(want, got):
            for key in w:
                self.assertEqual(w[key], g.get(key), "%s: %s" % (what, key))
        return f

    def test_skeiths_magic(self):
        """OnMagicAtk's effects as the boss makes them: the MagicSquare, a
        ForceGenerator 1000 over each of three members at count 30, the ring
        at the target once they are gone, an IceBreak at each member once it
        is gone; a WaveShock and the dead effect alongside."""
        rnd = random.Random(23)
        members = [[fb(rnd.uniform(-1500, 1500)), fb(rnd.uniform(-1500, 1500)), 0, ONE] for _ in range(3)]
        boss = [fb(rnd.uniform(-500, 500)), fb(rnd.uniform(-500, 500)), 0, ONE]
        up = [fb(1.5707963705062866), 0, 0, ONE]

        def force(p):
            p = [p[0], p[1], fb(1000.0), ONE]
            return ("force", "force %s %s %s 8 64 8" % (hexs(*p), hexs(*up), hexs(fb(10.0), fb(200.0), fb(200.0))),
                    "ccBossEffForceGeneratorCreate__FPfPffffiii", [p, up], [8, 100, 8], [10.0, 200.0, 200.0])
        rot = [0, 0, fb(0.7), 0]
        q = [0, fb(1.0 / 3), fb(10.0), 0]
        ring = ("ring", "ring %s %s %s 23" % (hexs(*members[0]), hexs(*rot), hexs(*q)),
                "ccBossEffAutoSamonRingCreate__FPfPfPfi", [members[0], rot, q], [35], [])

        def ice(p):
            return ("ice", "ice %s %x" % (hexs(*p), fb(2.0)), "ccBossEffIceBreakCreate__FPff", [p], [], [2.0])
        d = [0, 0, fb(0.3), 0]
        script = {
            0: [("square", "square %s 0" % hexs(*boss), "ccBossEffMagicSquareCreate__FPfi", [boss], [0], [])],
            30: [force(p) for p in members],
            40: [("wave", "wave %s %s %x" % (hexs(*boss), hexs(*d), fb(1.0)), "ccBossEffWaveShockCreate__FPfPff",
                  [boss, d], [], [1.0])],
            165: [ring],
            186: [ice(p) for p in members],
            200: [("dead", "dead %s" % hexs(*boss), "ccBossEffDeadCreate__FPfPfi", [boss, boss], [0], [])],
        }
        cam = ((1200.0, -900.0, 700.0), (0.0, 0.0, 150.0))
        frames = self.run_script(77, script, cam)
        print("\nmagic frames %d" % frames, file=sys.stderr)

    def test_skeiths_effects(self):
        rnd = random.Random(11)
        counts = {}
        for i, case in enumerate(cases(rnd)):
            cam = ((rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), 600.0),
                   (rnd.uniform(-500, 500), rnd.uniform(-500, 500), 100.0))
            frames = self.run_case(1000 + i, case, cam)
            counts[case[0]] = counts.get(case[0], 0) + len(frames)
        print("\nframes " + " ".join("%s %d" % kv for kv in sorted(counts.items())), file=sys.stderr)

    @unittest.skipUnless(volume.NAME == "outbreak", "Fidchell's spells are Outbreak's")
    def test_fidchells_spells(self):
        rnd = random.Random(29)
        counts = {}
        for rep_ in range(2):
            for i, case in enumerate(fidchell_cases(rnd)):
                cam = ((rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), 600.0),
                       (rnd.uniform(-500, 500), rnd.uniform(-500, 500), 100.0))
                frames = self.run_case(2000 + 16 * rep_ + i, case, cam)
                counts[case[0]] = counts.get(case[0], 0) + len(frames)
        print("\nframes " + " ".join("%s %d" % kv for kv in sorted(counts.items())), file=sys.stderr)

    def run_innis_case(self, seed, case, camera):
        """One of innis_cases: the Create (a missile's control points one
        block, its +0x40, which the constructor leaves unset, taken as the
        port's (0, 0, 0, 1); a ring's burst set after it), then the
        manager's passes until every slot is free."""
        name, line, sym, ctrl, args, burst = case
        em = BossFx(seed=seed)
        eye, view = camera
        em.vec(tfx.EYE, [fb(v) for v in eye] + [ONE])
        em.vec(tfx.EYE + 0x10, [fb(v) for v in view] + [ONE])
        if ctrl is not None:
            block = em.malloc(em.m, 64)
            em.vec(block, ctrl)
            k = em.create(sym, ints=[block] + args)
            em.vec(em.slot_ptr(k) + 0x40, [0, 0, 0, ONE])
        else:
            p, r, sc, model = args
            ptrs = vec_args(em, p, r, sc)
            k = em.create(sym, ints=ptrs + [model])
            sp, tpoint = burst
            ring = em.slot_ptr(k)
            (spp,) = vec_args(em, sp)
            em.call("BurstScale__14ccEffSamonRingFPf", ring, spp)
            em.call("BurstTransparency__14ccEffSamonRingFf", ring, fargs=[fb(tpoint)])
            em.m.store(ring + 0x60, 1, 1)
        made = {"id": k, "events": em.events, "gens": em.gens, "ccrand": em.ccrand}
        frames = []
        while not em.free() and len(frames) < 400:
            frames.append(em.manager_pass())
        lines = ["reset %x" % seed, "camera " + hexs(*map(fb, eye + view)), line] + ["frame"] * len(frames)
        got = [g for g in ask(lines) if g]
        self.assertEqual(got[0], made, "%s: the Create" % name)
        for f, (want, port) in enumerate(zip(frames, got[1:])):
            for key in want:
                self.assertEqual(want[key], port.get(key), "%s frame %d: %s" % (name, f, key))
        self.assertEqual(len(got[1:]), len(frames))
        return frames

    @unittest.skipUnless(volume.NAME == "mutation", "Innis's effects are Mutation's")
    def test_innis_effects(self):
        """Innis's missiles and rings: their pictures (the crystal, bolt and
        fireball in flight; the first's burst), trails (MissileSmoke
        generators following the missile) and landings (the trails killed,
        the BurstSmoke generators, the ice's rocks, the lightning's
        WaveShock, the sounds), and the rings' clump, palette and fade."""
        rnd = random.Random(31)
        counts = {}
        for rep in range(2):
            for i, case in enumerate(innis_cases(rnd)):
                cam = ((rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), 600.0),
                       (rnd.uniform(-500, 500), rnd.uniform(-500, 500), 100.0))
                frames = self.run_innis_case(3000 + 32 * rep + i, case, cam)
                counts[case[0]] = counts.get(case[0], 0) + len(frames)
        print("\nframes " + " ".join("%s %d" % kv for kv in sorted(counts.items())), file=sys.stderr)

    def run_kyvia_case(self, seed, case, camera):
        """One of kyvia_cases: the Create (a bolt's ThunderData a block of
        its own, the offset written after as ThunderboltAtk does), then the
        manager's passes until every slot is free."""
        name, line, sym, spec = case
        em = BossFx(seed=seed)
        eye, view = camera
        em.vec(tfx.EYE, [fb(v) for v in eye] + [ONE])
        em.vec(tfx.EYE + 0x10, [fb(v) for v in view] + [ONE])
        # The meteors face camera 2 (the boss's): the same camera here.
        cams = em.sym("cameraList")
        em.m.store(cams + 8, 4, em.m.load(cams, 4))
        if spec[0] == "meteor":
            _, pos, rng, n = spec
            (pp,) = vec_args(em, pos)
            em.meteor_base = -1
            k = em.create(sym, ints=[pp, n, 0, 0, 0], floats=[rng])
            em.meteor_base = None
        else:
            _, pos, time, dat, off = spec
            da, ra, ds, rs_, rp, sw, rng, num = dat
            (pp,) = vec_args(em, pos)
            d = em.malloc(em.m, 0x40)
            em.m.mem[d:d + 0x40] = bytes(0x40)
            for o, v in ((0, da), (4, ra), (8, ds), (12, rs_), (16, 1)):
                em.m.store(d + o, 4, v)
            em.m.store(d + 0x14, 1, sw)
            em.vec(d + 0x20, rp)
            k = em.create(sym, ints=[pp, time, rng, num, d])
            if off is not None:
                em.vec(em.slot_ptr(k) + 0x90, off)
        made = {"id": k, "events": em.events, "gens": em.gens, "ccrand": em.ccrand}
        frames = []
        while not em.free() and len(frames) < 400:
            frames.append(em.manager_pass())
        lines = ["reset %x" % seed, "camera " + hexs(*map(fb, eye + view)), line] + ["frame"] * len(frames)
        got = [g for g in ask(lines) if g]
        self.assertEqual(got[0], made, "%s: the Create" % name)
        for f, (want, port) in enumerate(zip(frames, got[1:])):
            for key in want:
                self.assertEqual(want[key], port.get(key), "%s frame %d: %s" % (name, f, key))
        self.assertEqual(len(got[1:]), len(frames))
        return frames

    @unittest.skipUnless(volume.NAME in ("mutation", "outbreak"), "Kyvia's effects are Mutation's and Outbreak's")
    def test_kyvia_effects(self):
        """Kyvia's meteors: their fireballs (never stepped), trails following
        each, the landings (trails deleted, sounds, the first's smoke and
        rocks) and their lives; Outbreak's bolts: the segments on their
        palette, a disc bolt's smoke (its row as Draw writes it) and burst,
        their ccRand draws and lives."""
        rnd = random.Random(37)
        counts = {}
        for rep in range(2):
            for i, case in enumerate(kyvia_cases(rnd)):
                cam = ((rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), 600.0),
                       (rnd.uniform(-500, 500), rnd.uniform(-500, 500), 100.0))
                frames = self.run_kyvia_case(3500 + 32 * rep + i, case, cam)
                counts[case[0]] = counts.get(case[0], 0) + len(frames)
        print("\nframes " + " ".join("%s %d" % kv for kv in sorted(counts.items())), file=sys.stderr)

    def test_skeiths_trail(self):
        """The cross's trail: the game's ccLattice(2, 7) (gcmn lattice.cpp)
        driven as ccBoss01::DrawCross drives it - runs of frames in the
        cross (NextVertex, SetPos of both ends, Disp) between runs of Disp
        alone - against piney_world::lattice. After each call: head, tail,
        each row's life and vertices, rows made, full, the first-row flag.
        After each Disp: the strip SendPacket sent (none, or each vertex's
        RGBA and ADC as MakePacket wrote them) and its ccDLSort key.

        Stand-ins: ccTransPosFW2LW copies; sceVu0RotTransPers puts every
        vertex on the screen in front of the eye (so ADC is the strip's
        first pair alone); ccDLSort::Add records the packet."""
        frames, strips = self.run_lattice(2, 7, False, 31)
        print("\ntrail frames %d, strips sent %d" % (frames, strips), file=sys.stderr)

    def test_weapon_trails(self):
        """The party's weapon trails: ccLattice(2, 0) and job 4's
        ccLattice(3, 0) driven as ccSpcChar::ArmsEffect drives them - each
        frame ClearCnt, then (in a swing) NextVertex and SetPos of every
        column, then Disp - with ClearArmsEffect's +0x72c and
        _SetArmsEffectColor's +4 set between frames. As the cross's check,
        with the flag and the type in the state; job 4's second strip (its
        columns 1-2) follows the first in the one packet."""
        for cols, seed in ((2, 41), (3, 43)):
            frames, strips = self.run_lattice(cols, 0, True, seed)
            print("\n%d columns: frames %d, strips sent %d" % (cols, frames, strips), file=sys.stderr)

    def run_lattice(self, cols, ty, arms, seed):
        em = BossFx(seed=5)
        m, sym = em.m, em.sym
        sent = []

        def fw2lw(mm, out, v, *a):
            em.vec(out, em.rvec(v))
            return 0

        def rot_trans_pers(mm, out, mat, v, mode, *a):
            x, y = (struct.unpack("<f", struct.pack("<I", w))[0] for w in em.rvec(v, 2))
            em.vec(out, [0x7000 + 16 * int(x), 0x7200 + 16 * int(y), 0x1000, 0])
            return 0

        def dl_add(mm, sort, key, tag, *a):
            key = mm.f[12]
            n = mm.load(obj + 2, 2)
            verts = []
            for r in range(n):
                for c in range(2):
                    rgba = em.rvec(tag + 16 * (6 + 4 * r + 2 * c))
                    xyz = em.rvec(tag + 16 * (7 + 4 * r + 2 * c))
                    verts.append([rgba[0], rgba[1], rgba[2], rgba[3], int(xyz[3] & 0x8000 != 0)])
            sent.append((verts, key))
            return 0

        m.hooks[sym("ccTransPosFW2LW__FPfPf")] = fw2lw
        m.hooks[sym("sceVu0RotTransPers")] = rot_trans_pers
        m.hooks[sym("Add__8ccDLSortFfP10_sceDmaTagP10_sceDmaTag")] = dl_add
        obj = em.malloc(m, 0x740)
        em.call("__ct__9ccLatticeFii", obj, cols, ty)

        def state():
            rows = []
            for r in range(16):
                rows.append(em.rvec(obj + 0x20 + 0x70 * r, 4 * cols))
            return {"head": m.load(obj + 0x72E, 2, True), "tail": m.load(obj + 0x730, 2, True),
                    "life": [m.load(obj + 0x86 + 0x70 * r, 2, True) for r in range(16)], "rows": rows,
                    "made": m.load(obj + 0x728, 2, True), "full": m.load(obj + 0x72A, 2, True),
                    "first": m.load(obj + 0, 2, True), "clear": m.load(obj + 0x72C, 2, True),
                    "ty": m.load(obj + 4, 4, True)}

        rnd = random.Random(seed)
        lines, want = ["lattice %x %x" % (cols, ty)], [{"state": state()}]
        pos = em.malloc(m, 16)
        cross, frames, strips = False, 0, 0
        while frames < 600:
            cross = not cross
            if arms and rnd.random() < 0.3:
                m.store(obj + 0x72C, 2, 1)
                lines.append("lclear")
                want.append({"state": state()})
            if arms and rnd.random() < 0.3:
                t = rnd.randint(0, 7)
                m.store(obj + 4, 4, t)
                lines.append("ltype %x" % t)
                want.append({"state": state()})
            for _ in range(rnd.randint(1, 40) if cross else rnd.randint(1, 25)):
                if arms:
                    em.call("ClearCnt__9ccLatticeFv", obj)
                    lines.append("lcnt")
                    want.append({"state": state()})
                if cross:
                    em.call("NextVertex__9ccLatticeFv", obj)
                    lines.append("lnext")
                    want.append({"state": state()})
                    for col in range(cols):
                        p = [fb(float(rnd.randint(-200, 200))), fb(float(rnd.randint(-200, 200))),
                             fb(float(rnd.randint(1, 400))), ONE]
                        em.vec(pos, p)
                        em.call("SetPos__9ccLatticeFPfi", obj, pos, col)
                        lines.append("lpos %s %x" % (hexs(*p), col))
                        want.append({"state": state()})
                sent.clear()
                em.call("Disp__9ccLatticeFv", obj)
                verts, key = sent[0] if sent else ([], None)
                strips += bool(sent)
                lines.append("ldisp")
                want.append({"strip": verts, "key": key, "state": state()})
                frames += 1
        got = [g for g in ask(lines) if g]
        self.assertEqual(len(got), len(want))
        for i, (w, g) in enumerate(zip(want, got)):
            self.assertEqual(w["state"], g["state"], "%d %s" % (i, lines[i]))
            if "strip" in w:
                self.assertEqual(w["strip"], g["strip"], "%d strip" % i)
                if w["key"] is not None:
                    self.assertEqual(w["key"], g["key"], "%d key" % i)
        self.assertGreater(strips, 100)
        return frames, strips

if __name__ == "__main__":
    unittest.main()
