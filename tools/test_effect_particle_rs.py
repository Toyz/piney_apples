#!/usr/bin/env python3
"""crates/piney-effect's particle system against the game's own particle code
run in eemu.

The particle_probe example runs the port's particle system on the requests
this sends it; the same requests go through the game's functions in eemu:

  - ParticleMachine (tools/test_effect_rs.py's EffectMachine, with
    startParticleGenerator left to run): ccParticleCtrl built by its own
    constructor (InitParticleCtrl looking particleCcsAnmTbl's 245 objects up
    through the EffectMachine's stream hooks) and set as particleSystem; each
    frame the real ccParticleCtrl::Main. Generators are made with the
    game's constructor, set up in memory and started with
    startParticleGenerator; the starters (ccParticleHitMark, ccParticleHeal,
    ccParticleExplode, ccParticleSetup, startParticleEffect(2),
    ccConditionEffect and its kill/delete) are called as the game calls
    them. What they draw is
    recorded as EffectMachine records it (ccEff::Draw with the ccEff as it
    stands, ccClump::Draw with its matrix and transparency, ccAnm::Draw),
    with the active layer; ChangeClut is recorded;
    effAbilityUp/Down answer the effect slot the test asks for.
  - After every request both sides answer the whole state: every generator
    on the list (all of its fields), every live particle (texID not -1, all
    of its fields and its object's), the frame's draws in order with their
    layer, the live effect slots the starters made, the events, the
    control's counts and serial numbers, and rand's state.

effect.cpp's static constructors (__sinit_effect.cpp: ccpgSmoke,
ccpgPawSmoke, hitPhotonDummyG, hitPhotonDummyStrG, serial numbers 0-3) are run
as at boot, their force fields then wired as ccEffectCtrl's constructor wires
them.

Checks: the arrival's generator (row 82 on a character, 0 0 180 0 up) in a
town and a field; every row of particleGeneratorTbl started with random
settings (following a character or not, random positions, turns, a layer,
texture overrides, paused, killed and deleted part way), each run until its
particles are gone or a cap; rows and force fields patched (in the table on
both sides) to reach the code no row does; more particles than the 2000
slots; every starter, particles counted to the static and to started
generators, and ccConditionEffect for every condition number, killed or
deleted.

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
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

from test_effect_rs import (EFFWORK, ELF, ISO, ONE, PROFILE, ROOT, SCRATCH, TARGET, EffectMachine, build,  # noqa: E402
                            fb, hexs)
from test_effect_spell_rs import Genrand  # noqa: E402

EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "particle_probe")
PARTICLE_SYSTEM = inf_va(0x00378AA0)
PARTICLES = inf_va(0x00388060)
GEN_TBL, GEN_ROWS = inf_va(0x003402F0), 244
EFF_SBL = inf_va(0x00378AD8)
# effect.cpp's static generators (ccpgSmoke, ccpgPawSmoke, hitPhotonDummyG,
# hitPhotonDummyStrG) and force fields (ccpfSmoke1-4).
STATIC_GENS = (inf_va(0x003FEFD0), inf_va(0x003FF0E0), inf_va(0x003FF1F0), inf_va(0x003FF300))
STATIC_FFS = (inf_va(0x003FEF00), inf_va(0x003FEF30), inf_va(0x003FEF60), inf_va(0x003FEF90))
SERIALS = (inf_va(0x00378AA8), inf_va(0x00378AAC))
P2NUM = inf_va(0x00378AB4)
NONE = 0xFFFFFFFF
# Scratch memory for the starters' arguments.
ARGS = SCRATCH + 0x8000


def s32(x):
    x &= 0xFFFFFFFF
    return x - (1 << 32) if x >> 31 else x


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class ParticleMachine(EffectMachine):
    """The game's particle system in eemu, as the module docstring says."""

    def __init__(self, town=True, seed=1):
        super().__init__(town=town, seed=seed)
        m, sym = self.m, self.sym
        del m.hooks[sym("startParticleGenerator__FP19ccParticleGenerator")]
        m.hooks[sym("Duplicate__7ccClumpFUi")] = lambda mm, *a: 0
        m.hooks[sym("ChangeClut__7ccClumpFP11ccClutChunkP11ccClutChunk")] = self.change_clut
        m.hooks[sym("ChangeClut__5ccAnmFP11ccClutChunkP11ccClutChunk")] = self.change_clut
        self.ability = 0
        m.hooks[sym("effAbilityUp__FP6ccChari")] = lambda mm, *a: self.ability
        m.hooks[sym("effAbilityDown__FP6ccChari")] = lambda mm, *a: self.ability
        m.hooks[sym("Draw__5ccEffFUs")] = self.eff_draw2
        self.cluts, self.pat_chunk = {}, {}
        self.layer_pri[m.load(EFF_SBL, 4)] = 60
        # effect.cpp's static constructors (the game runs them at boot,
        # before ccEffectCtrl's constructor wires the force fields in).
        keep = [(a, m.load(a, 4)) for a in STATIC_FFS]
        keep += [(g + 0x34 + 4 * k, m.load(g + 0x34 + 4 * k, 4)) for g in STATIC_GENS for k in range(4)]
        self.call("__sinit_effect.cpp")
        for a, v in keep:
            m.store(a, 4, v)
        # genrand() (ccRand, ccRandF: the dust's turns) as MT19937 seeded
        # with the seed, as the probe seeds its own.
        self.genrand = Genrand(seed)
        m.hooks[sym("genrand__Fv")] = self.genrand.next
        self.pc = self.malloc(m, 0x18)
        self.call("__ct__14ccParticleCtrlFi", self.pc, 0)
        m.store(PARTICLE_SYSTEM, 4, self.pc)
        self.pending = 0
        self.conds = []
        self.draws, self.events = [], []

    # hooks -------------------------------------------------------------------
    def change_clut(self, m, obj, new, old, *a):
        self.cluts[obj] = [self.names.get(new, "?"), self.names.get(old, "?") if old else None]
        return 0

    def chunk_of(self, eff):
        """eff_index, cached by the patterns' address."""
        pat = self.m.load(eff + 0x48, 4)
        if pat not in self.pat_chunk:
            from test_effect_rs import eff_index
            self.pat_chunk[pat] = eff_index(self, eff)
        return self.pat_chunk[pat]

    def clut_name(self, eff):
        c = self.m.load(eff + 0x3C, 4)
        return self.names.get(c, "?") if c else None

    def eff_draw2(self, m, eff, pat, *a):
        self.draws.append(["eff", self.chunk_of(eff), pat & 0xFFFF, self.rvec(eff + 0x10), m.load(eff + 0x20, 4),
                           m.load(eff + 0x24, 4), m.load(eff + 0x28, 4), m.load(eff + 0x2C, 4),
                           m.load(eff + 0x34, 4), self.layer(), self.clut_name(eff)])
        return 0

    # references --------------------------------------------------------------
    def vref(self, p):
        if not p:
            return None
        for cid, ch in self.chars.items():
            if ch <= p < ch + 0x200:
                off = p - ch
                return ["pos", cid] if off == 0x40 else ["dirc", cid] if off == 0x60 else ["at", cid, off]
        if EFFWORK <= p < EFFWORK + 0xC0 * 500:
            k, off = divmod(p - EFFWORK, 0xC0)
            return [{0: "effpos", 0x20: "effrot", 0x50: "effposT"}[off], k]
        return ["?", p]

    def ref_addr(self, words):
        """A probe reference (KIND ...) as an address."""
        k = words[0]
        if k == 0:
            return self.chars[words[1]] + 0x40
        if k == 1:
            return self.chars[words[1]] + 0x60
        if k == 4:
            return self.chars[words[1]] + words[2]
        if k == 5:
            return EFFWORK + 0xC0 * words[1] + 0x50
        return 0

    def gen_by_sn(self, sn):
        for s in STATIC_GENS:
            if self.m.load(s + 0xF4, 4) == sn:
                return s
        g = self.m.load(self.pc, 4)
        while g:
            if self.m.load(g + 0xF4, 4) == sn:
                return g
            g = self.m.load(g + 0x30, 4)
        return 0

    def lay(self, p):
        return None if p == 0 else self.layer_pri.get(p, p)

    # state -------------------------------------------------------------------
    def gen_state(self, g):
        m = self.m
        ff = []
        for k in range(4):
            f = m.load(g + 0x34 + 4 * k, 4)
            ff.append(m.load(f, 4) if f else None)
        return {
            "sn": m.load(g + 0xF4, 4), "param": m.load(g, 4), "bits": m.load(g + 4, 1) | (m.load(g + 5, 1) & 1) << 8,
            "pCnt": m.load(g + 6, 2, True), "gAge": m.load(g + 8, 2, True), "gLife": m.load(g + 10, 2, True),
            "pLife": m.load(g + 12, 2, True), "pNum": m.load(g + 14, 2, True), "regOfst": m.load(g + 16, 2, True),
            "syncRot": self.vref(m.load(g + 0x14, 4)), "syncPos": self.vref(m.load(g + 0x18, 4)),
            "syncPos2": self.vref(m.load(g + 0x1C, 4)), "syncSW": self.vref(m.load(g + 0x20, 4)),
            "gRate": m.load(g + 0x24, 4), "gRateCnt": m.load(g + 0x28, 4), "pTexMod": m.load(g + 0x2C, 2, True),
            "ff": ff, "layer": self.lay(m.load(g + 0x44, 4)), "rot": self.rvec(g + 0x50), "pos": self.rvec(g + 0x60),
            "pos2": self.rvec(g + 0x70), "offset": self.rvec(g + 0x80), "offset2": self.rvec(g + 0x90),
            "velocity": self.rvec(g + 0xA0), "mat": self.rvec(g + 0xB0, 16), "kill": m.load(g + 0xF0, 1, True),
            "end": m.load(g + 0xF1, 1),
        }

    def gen_list(self, pc=None):
        out, g = [], self.m.load(self.pc if pc is None else pc, 4)
        while g:
            out.append(self.gen_state(g))
            g = self.m.load(g + 0x30, 4)
        return out

    def part_list(self, base=PARTICLES, n=2000):
        m = self.m
        raw = bytes(m.mem[base:base + 0xB0 * n])
        out = []
        for i in range(n):
            o = 0xB0 * i
            if struct.unpack_from("<h", raw, o + 0x16)[0] == -1:
                continue
            w = struct.unpack_from("<44I", raw, o)
            h = lambda off: struct.unpack_from("<h", raw, o + off)[0]     # noqa: E731
            bits = w[0] & 0xFFFFF
            style = bits & 7
            gene, obj = w[1], w[2]
            vec = lambda off: list(w[off // 4:off // 4 + 4])              # noqa: E731
            ob, clut = None, None
            if obj:
                if style == 0:
                    ob = ["eff", self.chunk_of(obj), m.load(obj + 0x20, 4), m.load(obj + 0x24, 4),
                          m.load(obj + 0x28, 4), m.load(obj + 0x2C, 4), m.load(obj + 0x34, 4), m.load(obj + 0x60, 2),
                          m.load(obj + 0x62, 2), m.load(obj + 0x58, 4) | m.load(obj + 0x5C, 4) << 32,
                          self.rvec(obj + 0x10)]
                    c = self.clut_name(obj)
                    clut = None if c is None else [c, None]
                elif style == 1:
                    ob = ["clump", self.clump.get(obj, "?"), self.rvec(obj + 0x40, 16)]
                    clut = self.cluts.get(obj)
                elif style == 2:
                    ob = ["anm", self.anm[obj][0], self.anm[obj][1], self.rvec(obj + 0x40, 16)]
                    clut = self.cluts.get(obj)
            out.append({
                "i": i, "bits": bits, "gene": m.load(gene + 0xF4, 4) if gene else None, "obj": ob, "clut": clut,
                "layer": self.lay(w[3]), "sn": w[4], "anmPat": w[5] & 0xFFFF, "texID": h(0x16),
                "pos": vec(0x20), "rot": vec(0x30), "offset": vec(0x40), "dirc": vec(0x50), "scale": vec(0x60),
                "velocity": vec(0x70), "transparency": w[0x20], "speed": w[0x21], "size": w[0x22], "temp": w[0x23],
                "color": h(0x90), "fadeIn": h(0x92), "fadeOut": h(0x94), "life": h(0x96), "age": h(0x98),
                "cnt": h(0x9A), "rotate": [struct.unpack_from("<H", raw, o + 0x9C + 2 * k)[0] for k in range(4)],
                "ofstD": h(0xA4), "ofstR": h(0xA6),
            })
        return out

    def effect_slots(self):
        m, out = self.m, []
        for i in range(500):
            a = EFFWORK + 0xC0 * i
            if not m.load(a + 0x6E, 2):
                continue
            eff = m.load(a + 0x8C, 4)
            test = None
            if eff and eff not in self.clump and eff not in self.anm:
                test = m.load(eff + 0x58, 4) | m.load(eff + 0x5C, 4) << 32
            out.append([i, m.load(a + 0x6C, 2, True), m.load(a + 0x60, 2) & 0x1FF, m.load(a + 0x70, 2, True),
                        m.load(a + 0x80, 4),
                        self.rvec(a + 0x10), m.load(a + 0xA0, 4), self.lay(m.load(a + 0x9C, 4)),
                        m.load(a + 0xB0, 4), test])
        return out

    def state(self, frame=False):
        m = self.m
        st = {"gens": self.gen_list(), "parts": self.part_list(), "draws": self.draws if frame else [],
              "effects": self.effect_slots(), "events": self.events,
              "ctrl": [m.load(self.pc + 12, 2, True), m.load(self.pc + 14, 2, True), m.load(self.pc + 20, 4),
                       m.load(SERIALS[0], 4), m.load(SERIALS[1], 4), m.load(P2NUM, 2)],
              "rand": self.rand.s}
        self.draws, self.events = [], []
        return st

    def paw_smoke(self, left, right, dircz, speed, attr):
        """ccEffPawSmoke on a character whose clump's feet nodes (their LW
        matrices unparented, so copied from the local ones) stand at `left`
        and `right`, heading dircz; ccLandHitCheck answered, and
        checkHitResultAttlibute `attr`."""
        m, sym = self.m, self.sym
        ch = self.malloc(m, 0x100)
        m.store(ch + 0xD0, 4, ch)
        self.vec(ch + 0x60, [0, 0, dircz, 0])
        feet = {}
        for name, p in ((b"OBJ_t0 l foot", left), (b"OBJ_t0 r foot", right)):
            o = self.malloc(m, 0x100)
            for i in range(4):
                self.vec(o + 0x40 + 0x10 * i, [ONE if k == i else 0 for k in range(4)])
            self.vec(o + 0x70, list(p) + [ONE])
            feet[name] = o

        def get(mm, clump, name, *_):
            s = bytes(mm.mem[name:name + 16]).split(b"\0")[0]
            return feet[s]
        hooks = {"GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult": get,
                 "ccLandHitCheck__FPfUi": lambda mm, *_: 0,
                 "checkHitResultAttlibute__Fv": lambda mm, *_: attr}
        saved = {}
        for name, h in hooks.items():
            a = sym(name)
            saved[a] = m.hooks.get(a)
            m.hooks[a] = h
        try:
            return self.call("ccEffPawSmoke__FP6ccCharf", ch, fargs=(speed,)) & 0xFFFFFFFF
        finally:
            for a, h in saved.items():
                if h is None:
                    del m.hooks[a]
                else:
                    m.hooks[a] = h

    # requests ----------------------------------------------------------------
    def args(self, *vecs):
        """Vectors into scratch memory: their addresses."""
        out = []
        for k, v in enumerate(vecs):
            a = ARGS + 0x10 * k
            self.vec(a, list(v) + [ONE] * (4 - len(v)))
            out.append(a)
        return out

    def request(self, line):
        """One probe request, run in eemu; the extra answer fields."""
        w = line.split()
        n = [int(x, 16) for x in w[1:] if all(c in "0123456789abcdef" for c in x)]
        m, extra = self.m, {}
        op = w[0]
        if op == "player":
            self.vec(0x01022000 + 0x40, n[:3] + [ONE])
        elif op == "camera":
            self.vec(0x01023100, n[0:3] + [ONE])
            self.vec(0x01023000, n[3:6] + [ONE])
            self.vec(0x01023010, n[6:9] + [ONE])
        elif op == "char":
            cid = n[0]
            if cid in self.chars:
                self.vec(self.chars[cid] + 0x40, n[1:4] + [ONE])
            else:
                self.add_char(cid, (0.0, 0.0, 0.0), 0.0, 0.0)
                self.vec(self.chars[cid] + 0x40, n[1:4] + [ONE])
            param = m.load(self.chars[cid], 4)
            m.store(param + 0x18, 4, n[4])
            m.store(param + 0x1C, 4, n[5])
        elif op == "charvec":
            self.vec(self.chars[n[0]] + n[1], n[2:6])
        elif op == "charint":
            m.store(self.chars[n[0]] + n[1], 4, n[2])
        elif op == "spawn":
            extra["slot"] = self.spawn(n[0])
        elif op == "gen":
            g = self.malloc(m, 0x100)
            self.call(self.sym("__ct__19ccParticleGeneratorFP24ccParticleGeneratorParamP25ccParticleForceFieldParam"
                               "P25ccParticleForceFieldParamP25ccParticleForceFieldParamP25ccParticleForceFieldParam"),
                      g, GEN_TBL + 0x38 * n[0], 0, 0, 0, 0)
            self.pending = g
            extra["sn"] = m.load(g + 0xF4, 4)
        elif op == "patchgen":
            a = GEN_TBL + 0x38 * n[0]
            shift = {"gtype": 0, "rtype": 4, "dtype": 8}[w[2]]
            m.store(a, 2, (m.load(a, 2) & ~(15 << shift)) | (n[1] & 15) << shift)
        elif op == "patchff":
            a = inf_va(0x00343850) + 0x20 * n[0]
            if w[2] == "calc":
                m.store(a, 1, (m.load(a, 1) & ~1) | (n[1] & 1))
            else:
                m.store(a + {"field": 1, "force": 2}[w[2]], 1, n[1])
        elif op == "gset":
            g, key = self.pending, w[1]
            if key in ("syncpos", "syncpos2", "syncrot"):
                m.store(g + {"syncrot": 0x14, "syncpos": 0x18, "syncpos2": 0x1C}[key], 4, self.ref_addr(n))
            elif key == "syncsw":
                m.store(g + 0x20, 4, 0 if n[0] == NONE else self.chars[n[0]] + n[1])
            elif key in ("pos", "pos2", "offset", "offset2", "rot"):
                self.vec(g + {"rot": 0x50, "pos": 0x60, "pos2": 0x70, "offset": 0x80, "offset2": 0x90}[key], n[:4])
            elif key == "layer":
                m.store(g + 0x44, 4, 0 if n[0] == NONE else self.layer_of(s32(n[0])))
            elif key == "tex":
                m.store(g + 0x2C, 2, n[0])
            elif key in ("spt", "spt2"):
                bit = 0x10 if key == "spt" else 0x20
                m.store(g + 4, 1, (m.load(g + 4, 1) & ~bit) | (bit if n[0] else 0))
            elif key == "pause":
                m.store(g + 5, 1, (m.load(g + 5, 1) & ~1) | (1 if n[0] else 0))
            elif key == "kill":
                m.store(g + 0xF0, 1, n[0])
            elif key == "rate":
                m.store(g + 0x28, 4, n[0])
            else:
                raise KeyError(key)
        elif op == "gstart":
            self.call("startParticleGenerator__FP19ccParticleGenerator", self.pending)
            self.pending = 0
        elif op == "gpause":
            g = self.gen_by_sn(n[0])
            if g:
                m.store(g + 5, 1, (m.load(g + 5, 1) & ~1) | (1 if n[1] else 0))
        elif op in ("pkill", "pdelete"):
            g = self.gen_by_sn(n[0])
            if g:
                self.call("ParticleKill__19ccParticleGeneratorFv" if op == "pkill"
                          else "ParticleDelete__19ccParticleGeneratorFv", g)
        elif op == "hitmark":
            (p,) = self.args(n[0:3])
            self.call("ccParticleHitMark__FPf", p)
        elif op == "heal":
            self.call("ccParticleHeal__FP6ccChari", self.chars[n[0]], 0)
        elif op == "explode":
            p, v = self.args(n[0:3], n[3:6] + [0])
            self.call("ccParticleExplode__FPfPffi", p, v, n[7], fargs=(n[6],))
        elif op == "smoke":
            p, v = self.args(n[0:3], n[3:6])
            extra["ret"] = self.call("effSmoke__FPfPffiiUsUs", p, v, n[7], n[8], n[9], n[10], fargs=(n[6],))
        elif op == "pawsmoke":
            extra["ret"] = self.paw_smoke(n[0:3], n[3:6], n[6], n[7], n[8])
        elif op == "enemydust":
            (p,) = self.args(n[0:3])
            self.call("ccEnemyEffDust__FPfifii", p, n[3], n[5], n[6], fargs=(n[4],))
        elif op == "dustring":
            (p,) = self.args(n[0:3])
            self.call("ccEnemyEffDustRing__FPfffiii", p, n[5], n[6], n[7], fargs=(n[3], n[4]))
        elif op == "dustringch":
            (o,) = self.args(n[1:4])
            self.call("ccEnemyEffDustRing__FP6ccCharPfffiii", self.chars[n[0]], o, n[6], n[7], n[8],
                      fargs=(n[4], n[5]))
        elif op == "psetup":
            (p,) = self.args(n[1:4])
            gp = 0 if n[6] == NONE else self.gen_by_sn(n[6])
            s = self.call("ccParticleSetup__FiPfiiP19ccParticleGenerator", n[0], p, n[4], n[5], gp)
            extra["ret"] = -1 if s == 0 else (s - PARTICLES) // 0xB0
        elif op == "pgene":
            g = 0 if n[1] == NONE else self.gen_by_sn(n[1])
            m.store(PARTICLES + 0xB0 * n[0] + 4, 4, g)
        elif op == "peffect":
            self.call("startParticleEffect__FP6ccChari", self.chars[n[0]], n[1])
        elif op == "peffect2":
            k = 2 if n[0] in (0, 1, 5) else 3
            s = self.ref_addr(n[0:k])
            j = 2 if n[k] in (0, 1, 5) else 3
            e = self.ref_addr(n[k:k + j])
            rest = n[k + j:]
            sw = 0 if rest[1] == NONE else self.chars[rest[1]] + rest[2]
            self.call("startParticleEffect2__FPA4_fPA4_fiPi", s, e, rest[0], sw)
        elif op == "cond":
            self.ability = 0 if n[2] == NONE else EFFWORK + 0xC0 * n[2]
            ep = self.malloc(m, 0x20)
            self.call("__ct__17ccConditionEffectFP6ccChari", ep, self.chars[n[0]], n[1])
            genes = [m.load(ep + 4 * k, 4) for k in range(5)]
            eff = m.load(ep + 0x14, 4)
            extra["cond"] = {"gene": [m.load(g + 0xF4, 4) if g else None for g in genes],
                             "eff": None if eff == 0 else (eff - EFFWORK) // 0xC0, "effSN": m.load(ep + 0x18, 4)}
            extra["h"] = len(self.conds)
            self.conds.append(ep)
        elif op in ("condkill", "conddel"):
            self.call("killConditionEffect__FP17ccConditionEffect" if op == "condkill"
                      else "deleteConditionEffect__FP17ccConditionEffect", self.conds[n[0]])
        elif op == "frame":
            self.call("Main__14ccParticleCtrlFv", self.pc)
            st = self.state(frame=True)
            st.update(extra)
            return st
        else:
            raise KeyError(op)
        st = self.state()
        st.update(extra)
        return st


def diff(want, got, what):
    """The first difference between two states, named; None when equal."""
    for k in want:
        if want[k] == got.get(k):
            continue
        if k in ("gens", "parts") and len(want[k]) == len(got.get(k, [])):
            for a, b in zip(want[k], got[k]):
                if a != b:
                    d = {kk: (a[kk], b.get(kk)) for kk in a if a[kk] != b.get(kk)}
                    return "%s: %s %s differs: %s" % (what, k, a.get("i", a.get("sn")), d)
        if k == "draws" and len(want[k]) == len(got.get(k, [])):
            for i, (a, b) in enumerate(zip(want[k], got[k])):
                if a != b:
                    return "%s: draw %d: game %s port %s" % (what, i, a, b)
        return "%s: %s: game %s\nport %s" % (what, k, str(want[k])[:1500], str(got.get(k))[:1500])
    return None


class Run:
    """A request script run on both sides: `line(s)` adds a request, `go()`
    compares every answer."""

    def __init__(self, test, town, seed):
        self.test, self.town, self.seed = test, town, seed
        self.pm = ParticleMachine(town=town, seed=seed)
        self.lines = ["reset %x %x" % (int(town), seed)]
        self.want = [None]

    def __call__(self, line):
        self.lines.append(line)
        self.want.append(self.pm.request(line))
        return self.want[-1]

    def go(self):
        got = ask(self.lines)
        self.test.assertEqual(len(got), len(self.want))
        frames = 0
        for i, (a, b) in enumerate(zip(self.want, got)):
            if a is None:
                continue
            d = diff(a, b, "%s (request %d of seed %x)" % (self.lines[i], i, self.seed))
            if d:
                self.test.fail(d)
            frames += self.lines[i] == "frame"
        return frames


def fbs(*v):
    return [fb(x) for x in v]


def setup_world(r, rng, town=True, chars=3):
    """Player, camera and characters, near each other (with one sometimes
    far or behind the camera)."""
    px, py = rng.uniform(-3000, 3000), rng.uniform(-3000, 3000)
    pz = rng.uniform(0, 800)
    r("player " + hexs(*fbs(px, py, pz)))
    eye = (px + rng.uniform(-800, 800), py + rng.uniform(-1500, -300), pz + rng.uniform(100, 600))
    look = (px + rng.uniform(-200, 200), py + rng.uniform(-100, 400), pz + rng.uniform(0, 200))
    if rng.random() < 0.15:
        eye = (px + rng.uniform(-6000, 6000), py + rng.uniform(-6000, 6000), pz + 300)
    r("camera " + hexs(*fbs(*eye, *eye, *look)))
    for cid in range(chars):
        r("char %x " % cid + hexs(*fbs(px + rng.uniform(-400, 400), py + rng.uniform(-400, 400),
                                          pz + rng.uniform(-50, 50), rng.uniform(100, 200), 45.0)))
    return px, py, pz


def move_chars(r, rng, base, chars=3):
    px, py, pz = base
    for cid in range(chars):
        if rng.random() < 0.3:
            r("char %x " % cid + hexs(*fbs(px + rng.uniform(-400, 400), py + rng.uniform(-400, 400),
                                              pz + rng.uniform(-50, 50), rng.uniform(100, 200), 45.0)))


def run_frames(r, n, rng=None, base=None, until_empty=True):
    count = 0
    for f in range(n):
        if rng is not None and base is not None and f % 7 == 3:
            move_chars(r, rng, base)
        st = r("frame")
        count += 1
        if until_empty and not st["gens"] and not st["parts"]:
            break
    return count


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ParticlesAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build("particle_probe")
        cls.frames = 0

    @classmethod
    def tearDownClass(cls):
        print("\n%d frames compared" % cls.frames)

    def count(self, n):
        type(self).frames += n

    def test_arrival_generator(self):
        """Row 82 on a character, (0, 0, 180, 0) up, as effTransfer starts
        it; the character walks off part way."""
        for town, seed in ((True, 1), (True, 0x1234), (False, 99), (True, 0xDEADBEEF)):
            r = Run(self, town, seed)
            rng = random.Random(seed)
            base = setup_world(r, rng, town)
            r("gen 52")
            r("gset syncpos 0 0")
            r("gset spt 0")
            r("gset offset " + hexs(0, 0, fb(180.0), 0))
            r("gstart")
            run_frames(r, 300, rng, base)
            self.count(r.go())

    def test_every_generator_row(self):
        """Every row of particleGeneratorTbl, random settings."""
        rng = random.Random(7)
        total = 0
        for batch in range(0, GEN_ROWS, 8):
            seed = rng.randrange(1 << 32)
            r = Run(self, rng.random() < 0.5, seed)
            base = setup_world(r, rng)
            sns = []
            for row in range(batch, min(batch + 8, GEN_ROWS)):
                st = r("gen %x" % row)
                sns.append(st["sn"])
                px, py, pz = base
                if rng.random() < 0.6:
                    r("gset syncpos 0 %x" % rng.randrange(3))
                if rng.random() < 0.4:
                    r("gset syncpos2 0 %x" % rng.randrange(3))
                if rng.random() < 0.2:
                    r("gset syncrot 1 %x" % rng.randrange(3))
                r("gset pos " + hexs(*fbs(px + rng.uniform(-300, 300), py + rng.uniform(-300, 300),
                                          pz + rng.uniform(0, 200), 1.0)))
                r("gset pos2 " + hexs(*fbs(px + rng.uniform(-600, 600), py + rng.uniform(-600, 600),
                                           pz + rng.uniform(0, 400), 1.0)))
                r("gset offset " + hexs(*fbs(rng.uniform(-50, 50), rng.uniform(-50, 50), rng.uniform(0, 200), 0.0)))
                r("gset offset2 " + hexs(*fbs(rng.uniform(-50, 50), rng.uniform(-50, 50), rng.uniform(0, 200), 0.0)))
                if rng.random() < 0.5:
                    r("gset rot " + hexs(*fbs(rng.uniform(-3, 3), rng.uniform(-3, 3), rng.uniform(-3, 3), 0.0)))
                if rng.random() < 0.2:
                    r("gset layer %x" % (rng.choice([10, 30, 60, -10]) & NONE))
                if rng.random() < 0.15:
                    r("gset tex %x" % rng.choice([0, 3, 12, 31, 45, 101, 105, 184, 191, 200, 235, 243]))
                if rng.random() < 0.3:
                    r("gset kill 1")
                r("gstart")
            for f in range(260):
                if f == 40 and rng.random() < 0.5:
                    r("gpause %x 1" % rng.choice(sns))
                if f == 70:
                    for sn in sns:
                        r(("pkill %x" if rng.random() < 0.6 else "pdelete %x") % sn)
                if f % 9 == 4:
                    move_chars(r, rng, base)
                st = r("frame")
                if not st["gens"] and not st["parts"]:
                    break
            total += r.go()
        self.count(total)

    def test_patched_rows(self):
        """Code no table row reaches: gType 1, dType 5, rType and dType
        past the switches, force types 4, 5, 13 and past 16, field types
        past 3 - rows patched in both the executable's table and the
        port's."""
        rng = random.Random(31)
        for case in range(16):
            seed = rng.randrange(1 << 32)
            r = Run(self, False, seed)
            base = setup_world(r, rng)
            px, py, pz = base
            rows = rng.sample(range(GEN_ROWS), 4)
            for row in rows:
                if rng.random() < 0.5:
                    r("patchgen %x gtype 1" % row)
                if rng.random() < 0.4:
                    r("patchgen %x dtype %x" % (row, rng.choice([5, 5, 6, 3])))
                if rng.random() < 0.2:
                    r("patchgen %x rtype %x" % (row, rng.choice([6, 7])))
            for ff in rng.sample(range(148), 12):
                r("patchff %x force %x" % (ff, rng.choice([4, 5, 13, 17, 3, 14, 16])))
                if rng.random() < 0.2:
                    r("patchff %x field %x" % (ff, rng.choice([3, 4, 9])))
                if rng.random() < 0.3:
                    r("patchff %x calc %x" % (ff, rng.randrange(2)))
            for row in rows:
                r("gen %x" % row)
                if rng.random() < 0.6:
                    r("gset syncpos 0 %x" % rng.randrange(3))
                r("gset pos " + hexs(*fbs(px + rng.uniform(-300, 300), py + rng.uniform(-300, 300), pz, 1.0)))
                r("gset pos2 " + hexs(*fbs(px + rng.uniform(-600, 600), py + rng.uniform(-600, 600),
                                           pz + rng.uniform(0, 400), 1.0)))
                if rng.random() < 0.5:
                    r("gset kill 1")
                r("gstart")
            for f in range(160):
                if f == 60:
                    for g in r.want[-1]["gens"]:
                        r("pdelete %x" % g["sn"])
                st = r("frame")
                if not st["gens"] and not st["parts"]:
                    break
            self.count(r.go())

    def test_exhaustion(self):
        """More particles than the 2000 slots: rows 122 and 128 (about 140
        alive each) started thirty times, with others running."""
        rng = random.Random(41)
        r = Run(self, True, 0xABCDEF)
        base = setup_world(r, rng)
        px, py, pz = base
        for k in range(30):
            r("gen %x" % (128 if k % 3 == 0 else 122))
            r("gset pos " + hexs(*fbs(px + rng.uniform(-300, 300), py + rng.uniform(-300, 300), pz, 1.0)))
            r("gstart")
            if k % 5 == 0:
                r("gen 0")
                r("gset syncpos 0 %x" % rng.randrange(3))
                r("gset kill 1")
                r("gstart")
            r("frame")
        for f in range(400):
            if f == 150:
                for g in r.want[-1]["gens"]:
                    r("pkill %x" % g["sn"])
            st = r("frame")
            if not st["gens"] and not st["parts"]:
                break
        self.count(r.go())

    def test_starters(self):
        """The hit spark, the heal, the bursts and the character effects."""
        rng = random.Random(11)
        for case in range(12):
            seed = rng.randrange(1 << 32)
            r = Run(self, False, seed)
            base = setup_world(r, rng, chars=3)
            px, py, pz = base
            r("hitmark " + hexs(*fbs(px + rng.uniform(-200, 200), py, pz + 100)))
            r("heal %x" % rng.randrange(3))
            for t in range(8):
                r("explode " + hexs(*fbs(px + rng.uniform(-100, 100), py + rng.uniform(-100, 100), pz + 50,
                                         rng.uniform(-8, 8), rng.uniform(-8, 8), rng.uniform(0, 10),
                                         rng.uniform(0.5, 2))) + " %x" % t)
            r("psetup %x " % rng.choice([0, 12, 45, 101, 191]) + hexs(*fbs(px, py, pz + 60)) + " 1e 3 ffffffff")
            # Particles counted to effect.cpp's static generators (sn 0-3) and
            # to a started one, as effSmoke, ccEffPawSmoke and the hit photon
            # make them.
            for sn in (0, 1, 3):
                r("psetup %x " % rng.choice([7, 8, 11, 44]) + hexs(*fbs(px + 20, py, pz + 40)) +
                  " %x 0 %x" % (rng.randrange(10, 40), sn))
            st = r("psetup 66 " + hexs(*fbs(px - 20, py, pz + 40)) + " c 0 ffffffff")
            r("pgene %x 2" % st["ret"])
            g = r("gen 3")["sn"]
            r("gset pos " + hexs(*fbs(px, py + 30, pz, 1.0)))
            r("gstart")
            r("psetup 5 " + hexs(*fbs(px, py - 30, pz + 20)) + " 14 2 %x" % g)
            for num in rng.sample(range(49), 6):
                r("peffect %x %x" % (rng.randrange(3), num))
            r("charvec 1 160 " + hexs(*fbs(px + 30, py + 10, pz + 100, 1.0)))
            r("charvec 1 180 " + hexs(*fbs(px + 90, py - 40, pz + 140, 1.0)))
            r("charint 1 e4 1")
            r("peffect2 4 1 160 4 1 180 %x 1 e4" % rng.randrange(49))
            run_frames(r, 60, rng, base, until_empty=False)
            r("charint 1 e4 0")
            run_frames(r, 400, rng, base)
            self.count(r.go())

    def test_smoke_and_dust(self):
        """effSmoke, ccEnemyEffDust, both ccEnemyEffDustRing (an enemy's feet
        turned by its heading, an idol's ring), their puffs run out."""
        rng = random.Random(12)
        for case in range(10):
            seed = rng.randrange(1, 1 << 32)
            r = Run(self, rng.random() < 0.3, seed)
            base = setup_world(r, rng, chars=3)
            px, py, pz = base
            r("charvec 1 60 " + hexs(*fbs(0.0, 0.0, rng.uniform(-3.14, 3.14), 0.0)))
            for _ in range(rng.randrange(1, 4)):
                size = rng.choice([2.0, 4.0, rng.uniform(1, 10)])
                life, tex = rng.choice([6, 30, rng.randrange(1, 90)]), rng.choice([109, 116, 117, 132])
                p = fbs(px + rng.uniform(-300, 300), py + rng.uniform(-300, 300), pz + rng.uniform(-20, 60))
                k = rng.randrange(4)
                if k == 0:
                    st = r("smoke " + hexs(*p, *fbs(rng.uniform(-8, 8), rng.uniform(-8, 8), rng.uniform(0, 8), size)) +
                           " %x %x %x %x" % (life, tex, rng.choice([512, 0, 100]), rng.choice([32, 0, 60])))
                    self.assertEqual(st["ret"], 1)
                elif k == 1:
                    r("enemydust " + hexs(*p) + " %x " % rng.randrange(0, 6) + hexs(fb(size)) +
                      " %x %x" % (life, tex))
                elif k == 2:
                    r("dustring " + hexs(*p, *fbs(size, rng.uniform(20, 200))) +
                      " %x %x %x" % (rng.randrange(0, 13), life, tex))
                else:
                    ofs = fbs(rng.uniform(-100, 100), rng.choice([rng.uniform(-100, 100), 800.0]),
                              rng.choice([rng.uniform(-50, 50), -530.0]))
                    r("dustringch 1 " + hexs(*ofs, *fbs(size, rng.choice([100.0, rng.uniform(20, 200)]))) +
                      " %x %x %x" % (rng.choice([3, 8, rng.randrange(0, 13)]), life, tex))
                run_frames(r, rng.randrange(0, 12), rng, base, until_empty=False)
            run_frames(r, 200, rng, base)
            self.count(r.go())

    def test_paw_smoke(self):
        """ccEffPawSmoke: a runner's feet (either lower), its heading and
        speed, on every kind of ground: none from water and grass, texture 4
        from the grey and dark, else 133. A step a frame for a while."""
        grounds = [0, 0x20000F0F, 0x00B0C000, 0x00C0D000, inf_va(0x0060B0D0), inf_va(0x0070C0E0), 0x008080F0, inf_va(0x00304050),
                   0x00E0E0E0, inf_va(0x00606060), inf_va(0x00405060) | 0x01000101, inf_va(0x00123456)]
        rng = random.Random(13)
        for case in range(8):
            seed = rng.randrange(1, 1 << 32)
            r = Run(self, rng.random() < 0.3, seed)
            base = setup_world(r, rng, chars=1)
            px, py, pz = base
            dz = rng.uniform(-3.14, 3.14)
            for step in range(rng.randrange(20, 60)):
                foot = [px + rng.uniform(-30, 30), py + rng.uniform(-30, 30), pz + rng.uniform(0, 20)]
                other = [foot[0] + rng.uniform(-20, 20), foot[1] + rng.uniform(-20, 20),
                         rng.choice([foot[2], foot[2] + rng.uniform(-10, 10)])]
                attr = rng.choice(grounds)
                r("pawsmoke " + hexs(*fbs(*foot, *other, dz, rng.choice([4.0, 8.0, rng.uniform(0, 15)]))) +
                  " %x" % attr)
                run_frames(r, 1, until_empty=False)
            run_frames(r, 100, rng, base)
            self.count(r.go())

    def test_condition_effects(self):
        """ccConditionEffect for every condition number, killed or deleted
        part way."""
        rng = random.Random(23)
        nums = list(range(-1, 49))
        for start in range(0, len(nums), 5):
            seed = rng.randrange(1 << 32)
            r = Run(self, False, seed)
            base = setup_world(r, rng)
            slot = r("spawn 5")["slot"]
            hs = []
            for num in nums[start:start + 5]:
                st = r("cond %x %x %x" % (rng.randrange(3), num & 0xFFFFFFFF, slot if rng.random() < 0.7 else NONE))
                hs.append(st["h"])
            run_frames(r, 45, rng, base, until_empty=False)
            for h in hs:
                r(("condkill %x" if rng.random() < 0.5 else "conddel %x") % h)
            run_frames(r, 300, rng, base)
            self.count(r.go())


if __name__ == "__main__":
    unittest.main()
