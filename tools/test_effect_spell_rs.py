#!/usr/bin/env python3
"""crates/piney-effect's spells against the game's own code run in eemu.

An attack spell runs its element's system from ccSkill::Main (gcmn
0x005731d0; FallSystem 0x00577a30, TornadoSystem 0x00577540,
ConvergenceSystem 0x00578060, UpheavalSystem 0x005787c0, SummonsSystem
0x00579000), which spawns effect objects (main effect.cpp's eff* functions,
run by ccEffect::Main) and, from level 3, GCMN.PRG's effect elements (run by
ccEffectElementManager::Main); they call ccSkillDamage / ccSkillDamage2 as
they land.

SpellMachine is test_effect_rs.EffectMachine (main and GCMN.PRG in eemu, the
effect files served from DATA.BIN, draws recorded) in a field, plus what the
spells reach for:

  - a ccSkill built by its own constructor and set up as _ccSkillRequest
    does (creator, target, param, cPos, cDirc, cHeight, tPos, tType,
    skillType), run each frame by the real ccSkill::Main after
    ccEffectCtrl::Main and ccEffectElementManager::Main (ccThEffect at 80,
    ccThSkill at 82), deleted by ~ccSkill when Main returns non-zero;
  - characters (ccChar with a parameter row) whose presence on the command
    lists (ccCheckTarget), condition.dead and size (ccCheckObjectSize) the
    test chooses;
  - rand() as newlib's LCG and genrand() (ccRand, ccRandF) as MT19937, both
    seeded as the probe seeds them;
  - recorded, not run: the damage calls (ccSkillDamage and its overloads,
    ccSkillDamage2 and its overload), ccSkillHold, sounds (ccSeOn3D,
    ccSeOn3DNote), cameraShake, SetNoizBs, EntryFlash, particle starters
    (startParticleGenerator and ccParticleCtrlAddGenerator with the
    generator's fields; startParticleEffect runs and starts its rows'
    generators), clump duplication and CLUT swaps (ccParticleAdrs answers
    tagged addresses; ccEff::ChangeClut compares with the CLUT the sprite's
    texture names), and the ccEff draw at a position;
  - masked on both sides: what neither can know (a convergence piece's
    pos.w until it moves, a summoned creature's offset.z and w: the stack's
    leftovers in the game).

The spell_probe example runs the port on the same requests; each frame every
live effect slot, ccEffect2 slot, element, the ccSkill's own fields, the draws
in order, the events, the generators, and both generators' states are
compared.

    python3 tools/test_effect_spell_rs.py            the unit tests
    python3 tools/test_effect_spell_rs.py bulk N [SYSTEMS]
                                                     N random casts of every
                                                     ported spell
    python3 tools/test_effect_spell_rs.py ids N SID,SID,...
                                                     N casts of each, a line each
    python3 tools/test_effect_spell_rs.py one SID [N [SEED]]
    python3 tools/test_effect_spell_rs.py look SID   the game's run, printed

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_effect_rs as base  # noqa: E402
from test_effect_rs import ELF, ISO, ONE, PARTICLE_GENERATOR_TBL, EFFWORK, fb, hexs  # noqa: E402

TARGET = base.TARGET
PROFILE = base.PROFILE
EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "spell_probe")

SKILL_TOP, SKILL_TAIL, SKILL_NUM = inf_va(0x00378C94), inf_va(0x00378C98), inf_va(0x00378C8C)
M_INSTANCE = inf_va(0x00378C04)
EFFWORK2 = inf_va(0x003FC160)
SKILL_TBL = inf_va(0x0061F4A0)
PARTICLE_CCS_ADRS = inf_va(0x003E4680)
# ccParticleAdrs(n) answers CLUT_TAG + 0x10 n.
CLUT_TAG = 0x0E000000
CCMENU = inf_va(0x00378C88)
EFF_SBL = inf_va(0x00378AD8)
PARTICLE_CCS_ANM_TBL = inf_va(0x003739E0)
FF_TBL, FF_TBL_SIZE = inf_va(0x00343850), 4736
GAME = base.GAME


def fu(b):
    return struct.unpack("<f", struct.pack("<I", b & 0xFFFFFFFF))[0]


_DWARF = None
_FIELDS = {}


def dwarf():
    global _DWARF
    if _DWARF is None:
        import dwarf1
        _DWARF = dwarf1.Dwarf(ELF)
    return _DWARF


def fields(cls):
    """A class's members as DWARF lays them out, its bases' first:
    (name, offset, type, bit offset, bit size)."""
    if cls in _FIELDS:
        return _FIELDS[cls]
    import dwarf1
    dw = dwarf()
    ds = [d for d in dw.aggregates(cls)
          if any(c.tag in (dwarf1.TAG_member, dwarf1.TAG_inheritance) for c in d.children)]
    d = ds[0]
    out = []
    for c in d.children:
        if c.tag == dwarf1.TAG_inheritance:
            off = dwarf1.member_offset(c) or 0
            out.extend((n, off + o, t, bo, bs) for n, o, t, bo, bs in fields(dw.type_of(c)[1]))
        elif c.tag == dwarf1.TAG_member:
            out.append((c.name, dwarf1.member_offset(c), dw.type_of(c), c.attrs.get(dwarf1.AT_bit_offset),
                        c.attrs.get(dwarf1.AT_bit_size)))
    _FIELDS[cls] = out
    return out


BASE_SIZE = {"float": 4, "int": 4, "unsigned int": 4, "long": 4, "unsigned long": 4, "short": 2,
             "unsigned short": 2, "char": 1, "unsigned char": 1, "signed char": 1}


def read_value(m, a, t, ptr):
    k = t[0]
    if k == "cv":
        return read_value(m, a, t[2], ptr)
    if k == "array":
        import dwarf1
        n, et = t[1], t[2]
        size = dwarf().sizeof(et)
        return [read_value(m, a + size * i, et, ptr) for i in range(n)]
    if k == "ptr":
        return ptr(t[1], m.load(a, 4))
    name = t[1]
    if name == "float":
        return m.load(a, 4)
    if name in BASE_SIZE:
        n = BASE_SIZE[name]
        return m.load(a, n, not name.startswith("unsigned"))
    if name in ("ccClump", "ccEff"):
        # An element's own clump or sprite: named as a pointer to it is.
        return ptr(t, a)
    if len(t) > 2 and getattr(t[2], "children", None):
        return read_members(m, a, die_fields(t[2]), ptr)
    return read_struct(m, a, name, ptr)


def die_fields(d):
    """The members of the aggregate DIE `d` (its bases' first)."""
    import dwarf1
    dw = dwarf()
    if not any(c.tag in (dwarf1.TAG_member, dwarf1.TAG_inheritance) for c in d.children):
        return fields(d.name)
    out = []
    for c in d.children:
        if c.tag == dwarf1.TAG_inheritance:
            off = dwarf1.member_offset(c) or 0
            out.extend((n, off + o, t, bo, bs) for n, o, t, bo, bs in fields(dw.type_of(c)[1]))
        elif c.tag == dwarf1.TAG_member:
            out.append((c.name, dwarf1.member_offset(c), dw.type_of(c), c.attrs.get(dwarf1.AT_bit_offset),
                        c.attrs.get(dwarf1.AT_bit_size)))
    return out


def read_struct(m, a, cls, ptr):
    """Every member of the object at `a`: floats as bits, pointers through
    `ptr(pointee type, value)`."""
    return read_members(m, a, fields(cls), ptr)


def read_members(m, a, members, ptr):
    out = {}
    for name, off, t, bo, bs in members:
        if name in out:
            name = "%s@%x" % (name, off)
        if bs is not None:
            size = BASE_SIZE.get(t[1], 4)
            w = m.load(a + off, size)
            v = (w >> bo) & ((1 << bs) - 1)
            if not t[1].startswith("unsigned") and v >> (bs - 1):
                v -= 1 << bs
            out[name] = v
        else:
            out[name] = read_value(m, a + off, t, ptr)
    return out


NO_HIT = 0x461AB000


def land(z, offset_z, ground):
    """The flat land's answer to ccLandHitCheck2: the ground when it lies
    on the segment, as the EE adds and compares."""
    from eemu import f_add
    end = f_add(z, offset_z)
    lo, hi = sorted((fu(z), fu(end)))
    return ground if lo <= fu(ground) <= hi else NO_HIT


class Genrand:
    """genrand() (main 0x001d9620): MT19937 seeded as sgenrand does, the
    probe's piney_world::mt::Mt::seeded."""

    N, M = 624, 397

    def __init__(self, seed):
        mt = []
        s = seed & 0xFFFFFFFF
        for _ in range(self.N):
            w = s & 0xFFFF0000
            s = (s * 69069 + 1) & 0xFFFFFFFF
            w |= (s & 0xFFFF0000) >> 16
            s = (s * 69069 + 1) & 0xFFFFFFFF
            mt.append(w)
        self.mt, self.mti = mt, self.N
        self.count = 0

    def next(self, *_):
        N, M = self.N, self.M
        mt = self.mt
        if self.mti >= N:
            for k in range(N):
                y = (mt[k] & 0x80000000) | (mt[(k + 1) % N] & 0x7FFFFFFF)
                mt[k] = mt[(k + M) % N] ^ (y >> 1) ^ (0x9908B0DF if y & 1 else 0)
            self.mti = 0
        y = mt[self.mti]
        self.mti += 1
        y ^= y >> 11
        y ^= (y << 7) & 0x9D2C5680
        y ^= (y << 15) & 0xEFC60000
        y ^= y >> 18
        self.count += 1
        return y & 0xFFFFFFFF


class SpellMachine(base.EffectMachine):
    """The effect system in a field with a ccSkill, its characters and the
    element manager, as the module docstring says."""

    def __init__(self, seed=1, gseed=4352, area=1, dungeon=0, field_type=0):
        super().__init__(town=(area == 0), seed=seed)
        m, sym = self.m, self.sym
        m.store(GAME + 0x14, 4, area)
        m.store(GAME + 0x28, 4, dungeon)
        m.store(base.WM + 0x10, 4, field_type)
        self.genrand = Genrand(gseed)
        m.hooks[sym("genrand__Fv")] = self.genrand.next
        # The element manager runs this time.
        for name in ("__ct__22ccEffectElementManagerFv", "Main__22ccEffectElementManagerFv"):
            del m.hooks[sym(name)]
        mgr = self.malloc(m, 0x1000)
        self.call("__ct__22ccEffectElementManagerFv", mgr)
        m.store(M_INSTANCE, 4, mgr)
        self.mgr = mgr
        m.store(SKILL_TOP, 4, 0)
        m.store(SKILL_TAIL, 4, 0)
        m.store(SKILL_NUM, 4, 0)
        self.listed, self.dead, self.size = {}, {}, {}
        self.ground, self.lands = 0, 0
        self.skills = {}                 # key -> ccSkill address
        self.skill_key = {}
        self.clut = {}                   # clump or ccEff -> [(new, old)]
        self.clump_alpha_of = {}
        self.dup = {}                    # duplicated clump -> source
        for n in range(245):
            m.store(PARTICLE_CCS_ADRS + 4 * n, 4, CLUT_TAG + 0x10 * n)
        # ccMenu with its ccNoiz at +0xe8.
        menu = self.malloc(m, 0x200)
        noiz = self.malloc(m, 0x40)
        m.store(menu + 0xE8, 4, noiz)
        m.store(CCMENU, 4, menu)
        self.noiz = noiz
        # effSBL: the layer ccEffectCtrl made (its Init hooked out), priority 60.
        self.layer_pri[m.load(EFF_SBL, 4)] = 60
        rec = self.record
        hooks = {
            "ccCheckTarget__FP6ccChar": lambda mm, ch, *a: int(self.listed.get(self.char_of.get(ch), False)),
            "ccCheckObjectSize__FP6ccChar": lambda mm, ch, *a: self.size.get(self.char_of.get(ch), 0),
            # a0-a3 are the hook's arguments, t0-t2 (r8-r10) the rest.
            "ccSkillDamage__FP6ccCharP6ccCharP12ccSkillParamRsi":
                lambda mm, a, t, sk, ac: rec(["damage", self.who(a), self.who(t), self.skey(ac - 0x54),
                                              mm.r[8] & 0xFFFFFFFF]),
            "ccSkillDamage__FP6ccCharPfiP12ccSkillParami":
                lambda mm, a, pos, tt, sk: rec(["damage_at", self.who(a), self.rvec(pos), tt,
                                                mm.r[8] & 0xFFFFFFFF]),
            "ccSkillDamage2__FP6ccCharP6ccCharPfiP12ccSkillParamRsi":
                lambda mm, a, t, pos, tt: rec(["damage2", self.who(a), self.who(t), self.rvec(pos), tt,
                                               self.skey((mm.r[9] & 0xFFFFFFFF) - 0x54), mm.r[10] & 0xFFFFFFFF]),
            "ccSkillHold__FP6ccCharP6ccChari": lambda mm, *a: 0,
            "ccSkillHold__FP6ccCharPfii": lambda mm, *a: 0,
            "ccSeOn3DNote__FiPfc": lambda mm, se, pos, note, *a: rec(["se3dnote", se, self.rvec(pos), note]),
            "cameraShake__Fiiii": lambda mm, p, c, t, d, *a: rec(["shake", p, c, t, d]),
            "SetNoizBs__6ccNoizFi": lambda mm, nz, bs, *a: rec(["noise", bs]),
            "Duplicate__7ccClumpFUi": self.duplicate,
            "ChangeClut__7ccClumpFP11ccClutChunkP11ccClutChunk": self.change_clut,
            "Draw__5ccEffFPfUs": self.eff_draw_at,
            "ChangeClut__5ccEffFP11ccClutChunkP11ccClutChunk": self.eff_change_clut,
            "ccSkillDamage2__FP6ccCharP6ccCharPfiRsi":
                lambda mm, a, t, pos, tt: rec(["damage2", self.who(a), self.who(t), self.rvec(pos), tt,
                                               self.skey((mm.r[8] & 0xFFFFFFFF) - 0x54), mm.r[9] & 0xFFFFFFFF]),
            "ccHitCheckLM2__FPfPfUi": self.hit_check_lm2,
            "GetSubstAdrsF__5ccAnmFPCcb": self.subst_adrs,
            "Duplicate__5ccObjFUi": lambda mm, *a: 0,
            "ChangeClut__7ccModelFP11ccClutChunkP11ccClutChunk": self.model_change_clut,
            # startParticleEffect's way onto the list.
            "ccParticleCtrlAddGenerator__FP19ccParticleGenerator": self.start_generator,
            "ccLandHitCheck2__FPffUi": self.land_hit_check2,
            "EntryFlash__8ccScFadeFiiffff":
                lambda mm, fade, time, color, *a: rec(["flash", time, color, [mm.f[12 + i] for i in range(4)]]),
            "SetShadowSw__7ccClumpFi": lambda mm, *a: 0,
            "SetShadowSw__5ccAnmFi": lambda mm, *a: 0,
            "ccLandHitCheck__FPfUi": self.land_hit_check,
        }
        for name, fn in hooks.items():
            m.hooks[sym(name)] = fn
        # Locals the game reads before it writes them (the stack's leftovers,
        # which neither side can know): zeroed as the function starts, which
        # the probe's port takes them as. At the move of a0 just after the
        # prologue (ra saved): the locals cleared, the move done, then on.
        for pc, lo, hi, dst, src in ((inf_va(0x004FCF00), 160, 176, 19, 4),   # ccTreeUpheavalElement::Draw's throw
                                     (inf_va(0x004EE724), 64, 80, 17, 4),     # ccIceUpheavalMngrElement::Delete's
                                     (inf_va(0x004F7040), 192, 208, 18, 5)):  # ccGoblinSummonsElement's rings
            m.hooks[pc] = self.zeroing(pc, lo, hi, dst, src)
        m.hooks[inf_va(0x004F80DC)] = self.goblin_status

    @staticmethod
    def goblin_status(mm, *a):
        """ccGoblinSummonsElement::_EntryElement's search reads an
        element's status (lw v1, 544(v1)); after its first find it counts
        from that element and reads past the table's end, into whatever the
        heap holds there. Such a read answers busy, as the port takes it."""
        e = mm.r[3] & 0xFFFFFFFF
        table = mm.load((mm.r[19] & 0xFFFFFFFF) + 0x5C0, 4)
        mm.r[3] = mm.load(e + 544, 4, True) if e < table + 8 * 688 else 1
        mm.r[31] = inf_va(0x004F80E0)
        return mm.r[2] & 0xFFFFFFFF

    @staticmethod
    def zeroing(pc, lo, hi, dst, src):
        def hook(mm, *a):
            sp = mm.r[29] & 0xFFFFFFFF
            for o in range(lo, hi, 4):
                mm.store(sp + o, 4, 0)
            mm.r[dst] = mm.r[src]
            mm.r[31] = pc + 4
            return mm.r[2] & 0xFFFFFFFF
        return hook

    # the land ----------------------------------------------------------------
    def land_hit_check2(self, m, pos, mask, *a):
        """ccLandHitCheck2(pos, offsetZ, mask): the test's flat land at
        `ground`, met by the segment from pos.z to pos.z + offsetZ; 9900.0
        for none (the probe's host answers the same)."""
        m.f[0] = land(m.load(pos + 8, 4), m.f[12], self.ground)
        self.lands += 1
        return m.f[0]

    def land_hit_check(self, m, pos, mask, *a):
        """ccLandHitCheck(pos, mask): the test's flat land at `ground` when
        it lies from 105 above pos.z to 1000 below, else pos.z (the probe's
        host answers the same)."""
        from eemu import f_add, f_sub
        z = m.load(pos + 8, 4)
        top, bottom = fu(f_add(z, 0x42D20000)), fu(f_sub(z, 0x447A0000))
        m.f[0] = self.ground if bottom <= fu(self.ground) <= top else z
        return m.f[0]

    # recording ---------------------------------------------------------------
    def record(self, ev):
        self.events.append(ev)
        return 0

    def who(self, ch):
        if ch == 0:
            return -1
        return self.char_of.get(ch, ["?", ch])

    def skey(self, skill):
        return self.skill_key.get(skill, ["?", skill])

    def duplicate(self, m, clump, flag, *a):
        # ccClump::Duplicate(flag): the clump becomes its own copy (new
        # models the CLUT swap then changes); nothing else moves.
        self.dup[clump] = flag
        return 0

    def change_clut(self, m, clump, new, old, *a):
        self.clut.setdefault(clump, []).append([self.clut_n(new), self.clut_n(old)])
        return 0

    def hit_check_lm2(self, m, frm, to, mask, *a):
        """ccHitCheckLM2: the test's field has no models (-1.0, `to` left);
        the probe's host answers the same."""
        m.f[0] = 0xBF800000
        return m.f[0]

    def subst_adrs(self, m, anm, name, flag, *a):
        """ccAnm::GetSubstAdrsF(name): a stand-in ccObj (its model at +0x94)
        for the test's ccAnm, which has none."""
        from eemu import _cstr
        obj = self.malloc(m, 0xB0)
        model = self.malloc(m, 0x40)
        m.store(obj + 0x94, 4, model)
        self.subst_of = getattr(self, "subst_of", {})
        self.subst_of[model] = (anm, _cstr(m, name).decode())
        return obj

    def model_change_clut(self, m, model, new, old, *a):
        anm, name = self.subst_of[model]
        self.anm_clut = getattr(self, "anm_clut", {})
        self.anm_clut.setdefault(anm, []).append([name, self.clut_n(new)])
        return 0

    def eff_change_clut(self, m, eff, new, old, *a):
        """ccEff::ChangeClut(new, old): the ccTex's CLUT becomes `new` while
        it is `old`. The test's ccEffs have no texture chunk, so the CLUT
        they start with is named from the file: their texture's."""
        cur = m.load(eff + 0x3C, 4)
        if CLUT_TAG <= cur < CLUT_TAG + 0x10 * 245:
            name = self.clut_n(cur)
        else:
            if not hasattr(self, "_tex_clut"):
                import ccstex
                self._tex_clut = {}
                for c in self.ccs.values():
                    for tx in ccstex.read(c)[0]:
                        self._tex_clut[c.objects[tx.obj][0]] = c.objects[tx.clut][0]
            pat = m.load(eff + 0x48, 4)
            chunk = next((n for n, adr in self.chunks.items() if m.load(adr + 0x34, 4) == pat and n in self.effs),
                         None)
            name = self._tex_clut.get(self.effs[chunk]["tex"]) if chunk else None
        if name is not None and name == self.clut_n(old):
            m.store(eff + 0x3C, 4, new)
        return 0

    def clut_n(self, a):
        """A CLUT chunk by name: ccParticleAdrs(n)'s particleCcsAnmTbl[n], or
        a chunk GetChunkAdrsF served."""
        if a == 0:
            return None
        if CLUT_TAG <= a < CLUT_TAG + 0x10 * 245:
            n = (a - CLUT_TAG) // 0x10
            return self.prog.cstr(self.m.load(PARTICLE_CCS_ANM_TBL + 8 * n + 4, 4)).decode()
        return self.names.get(a, ["?", a])

    def clump_transparency(self, m, clump, *a):
        self.clump_alpha_of[clump] = m.f[12]
        return 0

    def clump_draw(self, m, clump, *a):
        # ccClump::Init leaves every object's transparency 1.
        self.draws.append(["clump", self.clump.get(clump, "?"), self.rvec(clump + 0x40, 16),
                           self.clump_alpha_of.get(clump, ONE), self.layer()])
        if clump in self.clut:
            self.draws[-1].append(self.clut[clump])
        return 0

    def eff_draw_at(self, m, eff, pos, pat, *a):
        for i in range(3):
            m.store(eff + 0x10 + 4 * i, 4, m.load(pos + 4 * i, 4))
        return self.eff_draw(m, eff, pat)

    def start_generator(self, m, g, *a):
        """The generator as its starter left it: the row, the force fields
        passed to the constructor (the ones outside particleForceFieldTbl,
        which the row's own come from), what it follows, and every field an
        effect sets after the constructor."""
        param = m.load(g, 4)
        row = (param - PARTICLE_GENERATOR_TBL) // 0x38
        if not (0 <= param - PARTICLE_GENERATOR_TBL < 0x38 * 300 and (param - PARTICLE_GENERATOR_TBL) % 0x38 == 0):
            row = ["va", param]

        def ref(p):
            if not p:
                return None
            for cid, ch in self.chars.items():
                if p == ch + 0x40:
                    return ["pos", cid]
                if p == ch + 0x60:
                    return ["dirc", cid]
                if ch <= p < ch + 0x200:
                    # A word in the character (an idol's effsw).
                    return ["at", cid, p - ch]
            if EFFWORK <= p < EFFWORK + 0xC0 * 500:
                k, off = divmod(p - EFFWORK, 0xC0)
                return ["eff", k, off]
            if base.HEAP <= p < base.HEAP_END:
                # An element's own vector (a ball's or a star's position):
                # the port's element moves its generator itself
                # (summoned::follow) and names none.
                return None
            return ["?", p]
        ff = []
        for i in range(4):
            obj = m.load(g + 0x34 + 4 * i, 4)
            va = m.load(obj, 4) if obj else 0
            ff.append(va if va and not (FF_TBL <= va < FF_TBL + FF_TBL_SIZE) else None)
        self.gens.append({"row": row, "ff": ff,
                          "sync": [ref(m.load(g + 0x14, 4)), ref(m.load(g + 0x18, 4)), ref(m.load(g + 0x1C, 4))],
                          "syncSW": ref(m.load(g + 0x20, 4)),
                          "types": (m.load(g + 4, 1) >> 4) & 3, "pTexMod": m.load(g + 0x2C, 2, True),
                          "rot": self.rvec(g + 0x50), "pos": self.rvec(g + 0x60), "pos2": self.rvec(g + 0x70),
                          "offset": self.rvec(g + 0x80), "offset2": self.rvec(g + 0x90),
                          "velocity": self.rvec(g + 0xA0)})
        return 0

    def slot(self, i, base=EFFWORK, n=500):
        """The base's slot, with a ccEff's PRIM and ccTex CLUT (+0x62,
        +0x3c; the CLUT by name when ccParticleAdrs gave it), and the
        convergence pieces' pointer to their controller's temp[0] as its
        slot."""
        d = super().slot(i, base, n)
        m = self.m
        a = base + 0xC0 * i
        if d["obj"] and d["obj"][0] == "eff":
            eff = m.load(a + 0x8C, 4)
            c = m.load(eff + 0x3C, 4)
            d["obj"] = d["obj"] + [m.load(eff + 0x62, 2),
                                   self.clut_n(c) if CLUT_TAG <= c < CLUT_TAG + 0x10 * 245 else None]
        if d["id"] == 168:
            # The drill's two ccAnm.
            d["temp"] = [(["anm"] + self.anm.get(p, ["?", p])) if p else None for p in d["temp"][:2]] + d["temp"][2:]
        if d["id"] == -26 and EFFWORK <= d["temp"][0] < EFFWORK + 0xC0 * 500:
            d["temp"][0] = ["eff", (d["temp"][0] - EFFWORK) // 0xC0]
        if 102 <= d["id"] <= 112:
            p = d["temp"][0]
            d["temp"][0] = ["eff", (p - 0xA0 - EFFWORK) // 0xC0] if EFFWORK <= p < EFFWORK + 0xC0 * 500 else p
        return d

    # the world ---------------------------------------------------------------
    def add_spell_char(self, cid, pos, height, width, dirc=(0.0, 0.0, 0.0), listed=True, dead=0, ctype=0x20,
                       size=0):
        ch = self.add_char(cid, pos, height, width, dirc)
        self.m.store(ch, 4, ch + 0x100)
        self.m.store(ch + 0x100 + 8, 4, ctype)
        self.set_state(cid, listed, dead, size)
        return ch

    def set_state(self, cid, listed, dead, size):
        self.listed[cid] = bool(listed)
        self.dead[cid] = dead
        self.size[cid] = size
        self.m.store(self.chars[cid] + 8, 2, dead & 0xFFFF)

    # the skill ---------------------------------------------------------------
    def request(self, key, sid, stype, creator, target):
        """new ccSkill(sid) as _ccSkillRequest sets it up (its rules - SP,
        the attribute critical, the name - are the battle crate's)."""
        m = self.m
        sk = self.malloc(m, 0xB0)
        self.call("__ct__7ccSkillFi", sk, sid)
        m.store(sk + 0x0C, 1, (m.load(sk + 0x0C, 1) & 0xF0) | (stype & 0xF))
        cp = self.chars[creator] if creator >= 0 else 0
        tp = self.chars[target] if target >= 0 else 0
        m.store(sk + 0x70, 4, cp)
        m.store(sk + 0x74, 4, tp)
        param = SKILL_TBL + 0x38 * sid
        m.store(sk + 0x08, 4, param)
        if cp:
            self.vec(sk + 0x30, self.rvec(cp + 0x40))
            self.vec(sk + 0x40, self.rvec(cp + 0x60))
            m.store(sk + 0x50, 4, m.load(m.load(cp, 4) + 0x18, 4))
        if tp and self.listed.get(target):
            self.vec(sk + 0x60, self.rvec(tp + 0x40))
            m.store(sk + 0x28, 4, m.load(m.load(tp, 4) + 8, 4))
        m.store(sk + 0x24, 4, m.load(param + 0x2C, 4))
        if cp:
            m.store(cp + 0x7C, 2, sid)
            m.store(cp + 0x7E, 2, 1)
        self.skills[key] = sk
        self.skill_key[sk] = key
        return sk

    def skill_state(self, key):
        m, sk = self.m, self.skills[key]
        flags = m.load(sk + 0x0C, 2)

        def slot(p):
            return -1 if p == 0 else ((p - EFFWORK) // 0xC0 if EFFWORK <= p < EFFWORK + 0xC0 * 500 else ["?", p])
        elm = m.load(sk + 0xA4, 4)
        return {"key": key, "live": self.live_skill(sk), "id": m.load(sk, 4, True), "flags": flags & 0x3FF,
                "level": m.load(sk + 0x10, 4, True), "count": m.load(sk + 0x14, 2, True),
                "trigger": m.load(sk + 0x16, 2, True), "step": m.load(sk + 0x18, 2, True),
                "effNum": m.load(sk + 0x1A, 2, True), "atkCnt": m.load(sk + 0x1C, 4, True),
                "tempCnt": m.load(sk + 0x20, 4, True), "skillType": m.load(sk + 0x24, 4, True),
                "tType": m.load(sk + 0x28, 4, True), "cPos": self.rvec(sk + 0x30), "cDirc": self.rvec(sk + 0x40),
                "cHeight": m.load(sk + 0x50, 4), "tPos": self.rvec(sk + 0x60),
                "creator": self.who(m.load(sk + 0x70, 4)), "target": self.who(m.load(sk + 0x74, 4)),
                "effPtr": [slot(m.load(sk + 0x84 + 4 * i, 4)) for i in range(8)],
                "effElm": -1 if elm == 0 else self.element_slot(elm, "gone")}

    def live_skill(self, sk):
        p = self.m.load(SKILL_TOP, 4)
        while p:
            if p == sk:
                return True
            p = self.m.load(p + 4, 4)
        return False

    def element_slot(self, e, gone=None):
        for i in range(1024):
            if self.m.load(self.mgr + 4 * i, 4) == e:
                return i
        return gone if gone is not None else ["gone", e]

    def char_state(self, cid):
        ch = self.chars[cid]
        return [self.m.load(ch + 0x7C, 2, True), self.m.load(ch + 0x7E, 2, True)]

    # a frame -----------------------------------------------------------------
    def frame(self):
        """ccThEffect (ccEffectCtrl::Main, ccEffectElementManager::Main),
        then ccThSkill (each ccSkill's Main, deleting the ones over)."""
        self.draws, self.events, self.gens = [], [], []
        self.call("Main__12ccEffectCtrlFv", self.effc)
        self.call("Main__22ccEffectElementManagerFv", self.mgr)
        p = self.m.load(SKILL_TOP, 4)
        while p:
            nxt = self.m.load(p + 4, 4)
            if self.call("Main__7ccSkillFv", p) & 0xFFFFFFFF:
                self.call("__dt__7ccSkillFv", p, 1)
            p = nxt
        return {"slots": self.live(), "draws": self.draws, "events": self.events, "gens": self.gens,
                "skills": [self.skill_state(k) for k in sorted(self.skills)],
                "chars": {str(c): self.char_state(c) for c in sorted(self.chars)},
                "elements": self.elements(), "eff2": self.effects2(),
                "rand": self.rand.s, "genrand": self.genrand.count}

    def effects2(self):
        m = self.m
        out = []
        for i in range(100):
            a = EFFWORK2 + 0x14 * i
            if m.load(a + 4, 2):
                out.append({"i": i, "end": m.load(a, 1) & 1, "id": m.load(a + 2, 2, True),
                            "status": m.load(a + 4, 2, True), "life": m.load(a + 6, 2, True),
                            "age": m.load(a + 8, 2, True), "cnt": m.load(a + 10, 2, True),
                            "target": self.who(m.load(a + 12, 4)),
                            "eff": self.element(m.load(a + 16, 4)) if m.load(a + 16, 4) else None})
        return out

    def element_name(self, e):
        return None if e == 0 else self.vtable_name(e)

    def vtable_name(self, e):
        """An element's class, from its vtable's symbol."""
        vt = self.m.load(e + 0x188, 4)
        if not hasattr(self, "_vt"):
            self._vt = {}
            for sym in self.prog.symbols:
                if sym.name.startswith("__vt__"):
                    import re
                    mm = re.match(r"__vt__(\d+)(.*)", sym.name)
                    if mm:
                        self._vt[sym.value] = mm.group(2)[:int(mm.group(1))]
        return self._vt.get(vt, hex(vt))

    def ptr(self, t, v):
        """A pointer member, named as the probe names it."""
        if v == 0:
            return None
        k = t[0]
        name = t[1] if k == "base" else None
        if name == "ccSkill":
            return self.skey(v)
        if name == "ccChar":
            return self.who(v)
        if name == "ccClump":
            return [self.clump.get(v, ["?", v]), self.clut.get(v)]
        if name == "ccEff":
            return self.eff_state(v)
        if name == "char":
            return self.prog.cstr(v).decode()
        if name == "ccStream":
            return self.files[v - 0x10000] if 0x10000 <= v < 0x10000 + len(self.files) else ["?", v]
        if name == "ccAnm":
            return ["anm"] + self.anm.get(v, ["?", v])
        if name == "ccEffect":
            return (v - EFFWORK) // 0xC0 if EFFWORK <= v < EFFWORK + 0xC0 * 500 else ["?", v]
        if name and name.endswith("Element"):
            return self.element_slot(v, "gone")
        if k == "array":
            for cid, ch in self.chars.items():
                if v == ch + 0x40:
                    return ["pos", cid]
                if v == ch + 0x60:
                    return ["dirc", cid]
            for key, sk in self.skills.items():
                if sk <= v < sk + 0xB0:
                    return ["skill", key, v - sk]
            return ["?", v]
        return "ptr"

    def element(self, e):
        """An element's every member (DWARF's layout), by its class."""
        cls = self.vtable_name(e)
        d = read_struct(self.m, e, cls, self.ptr)
        expand = getattr(self, "expand_" + cls, None)
        if expand:
            expand(e, d)
        return [cls, d]

    def expand_ccFallElement(self, e, d):
        arr, n = self.m.load(e + 0x5B8, 4), self.m.load(e + 0x590, 4, True)
        d["m_elements"] = [self.fall_slot(arr + 28 * i) for i in range(n)] if arr else None

    def fall_slot(self, a):
        m = self.m
        return {"elm": self.element(m.load(a, 4)), "height": m.load(a + 4, 4), "radius": m.load(a + 8, 4),
                "rad": m.load(a + 12, 4), "interval": m.load(a + 16, 4, True), "status": m.load(a + 20, 4, True),
                "seOn": m.load(a + 24, 4, True)}

    def expand_ccThunderFallElement(self, e, d):
        m = self.m
        arr, n = m.load(e + 0x5A0, 4), m.load(e + 0x590, 4, True)
        d["m_elements"] = [{"elm": self.element(m.load(arr + 12 * i, 4)), "interval": m.load(arr + 12 * i + 4, 4, True),
                            "status": m.load(arr + 12 * i + 8, 4, True)} for i in range(n)] if arr else None

    def expand_ccConvergenceElement(self, e, d):
        m = self.m
        arr, n = m.load(e + 0x5A0, 4), m.load(e + 0x590, 4, True)
        d["m_elements"] = [{"elm": self.element(m.load(a, 4)), "interval": m.load(a + 4, 4, True),
                            "cp": [self.rvec(a + 16), self.rvec(a + 32)], "time": m.load(a + 48, 4),
                            "status": m.load(a + 52, 4, True), "life": m.load(a + 56, 4, True)}
                           for a in (arr + 64 * i for i in range(n))] if arr else None

    def expand_ccIceUpheavalMngrElement(self, e, d):
        m = self.m
        d["m_elements"] = [{"elm": self.element(m.load(a, 4)) if m.load(a, 4) else None,
                            "status": m.load(a + 4, 4, True)} for a in (e + 0x5A4 + 8 * i for i in range(128))]
        b = m.load(e + 0x9A4, 4)
        d["m_bigIce"] = self.element(b) if b else None

    def eff_state(self, a):
        """A ccEff: its chunk, position, scale, turn, colour, transparency,
        PRIM and CLUT (by name)."""
        from test_effect_rs import eff_index
        m = self.m
        c = m.load(a + 0x3C, 4)
        clut = None if c == 0 else (self.clut_n(c) if CLUT_TAG <= c < CLUT_TAG + 0x10 * 245
                                    else self.names.get(c, ["?", c]))
        return [eff_index(self, a), self.rvec(a + 0x10), m.load(a + 0x20, 4), m.load(a + 0x24, 4),
                m.load(a + 0x28, 4), m.load(a + 0x2C, 4), m.load(a + 0x34, 4), m.load(a + 0x62, 2), clut]

    def struct_array(self, e, cls, field, n, stride):
        """The `n` structs a member of `e` points at, each by DWARF."""
        for name, off, t, _, _ in fields(cls):
            if name == field:
                arr = self.m.load(e + off, 4)
                if not arr:
                    return None
                members = die_fields(t[1][2])
                return [read_members(self.m, arr + stride * i, members, self.ptr) for i in range(n)]
        raise KeyError(field)

    def owned(self, e, off):
        p = self.m.load(e + off, 4)
        return self.element(p) if p else None

    def expand_ccSummonsSystemElement(self, e, d):
        d["m_effSummon"] = self.owned(e, 0x5A4)

    def expand_ccFireSummonElement(self, e, d):
        m = self.m
        arr, n = m.load(e + 0x5A0, 4), m.load(e + 0x590, 4, True)
        d["m_elements"] = [{"elm": self.element(m.load(arr + 8 * i, 4)), "interval": m.load(arr + 8 * i + 4, 4, True)}
                           for i in range(n)] if arr else None

    def expand_ccWaterSummonsElement(self, e, d):
        d["m_elements"] = self.struct_array(e, "ccWaterSummonsElement", "m_elements", self.m.load(e + 0x590, 4, True),
                                            0x1B0)
        d["m_iceFall"] = self.owned(e, 0x5A8)

    def expand_ccDarkSummonsElement(self, e, d):
        d["m_elements"] = self.struct_array(e, "ccDarkSummonsElement", "m_elements", 32, 0x200)
        d["m_ballTbl"] = self.struct_array(e, "ccDarkSummonsElement", "m_ballTbl", 4, 0x1E0)
        d["m_darkBall"] = self.owned(e, 0x5B4)

    def expand_ccSoilSummonsElement(self, e, d):
        d["m_elements"] = self.struct_array(e, "ccSoilSummonsElement", "m_elements", self.m.load(e + 0x590, 4, True),
                                            0x1B0)

    def expand_ccTreeSummonsElement(self, e, d):
        d["m_needleTbl"] = self.struct_array(e, "ccTreeSummonsElement", "m_needleTbl", 32, 0x1E0)

    def expand_ccGoblinSummonsElement(self, e, d):
        d["m_elements"] = self.struct_array(e, "ccGoblinSummonsElement", "m_elements", 8, 0x2B0)
        d["m_starTbl"] = self.struct_array(e, "ccGoblinSummonsElement", "m_starTbl", 8, 0x220)

    def expand_ccEnergyGrowElement(self, e, d):
        d["m_energyTbl"] = self.struct_array(e, "ccEnergyGrowElement", "m_energyTbl", self.m.load(e + 0x204, 4, True),
                                             0x40)

    def expand_ccExplodeElement(self, e, d):
        d["subst"] = getattr(self, "anm_clut", {}).get(self.m.load(e + 0x190, 4), [])

    def expand_ccThunderBoltElement(self, e, d):
        tbl = self.m.load(e + 0x190, 4)
        n = self.m.load(e + 0x1D4, 4, True)
        d["m_thunderTbl"] = [[self.rvec(tbl + 32 * i), self.m.load(tbl + 32 * i + 16, 4, True)]
                             for i in range(n + 1)] if tbl else None

    def elements(self):
        out = []
        for i in range(1024):
            e = self.m.load(self.mgr + 4 * i, 4)
            if e:
                out.append([i] + self.element(e))
        return out


class Case:
    """One cast, sent to both sides: the world, the characters, the skill
    and the frames; `changes` {frame: [(cid, field, value)]} move or change
    characters between frames."""

    def __init__(self, sid, seed, gseed=4352, area=1, dungeon=0, field_type=0):
        self.sid, self.seed, self.gseed = sid, seed, gseed
        self.area, self.dungeon, self.field_type = area, dungeon, field_type
        self.player = (0.0, 0.0, 0.0)
        self.camera = ((0.0, -900.0, 500.0), (0.0, -900.0, 500.0), (0.0, 0.0, 100.0))
        self.chars = {}
        self.stype, self.creator, self.target = 0, 0, 1
        self.changes = {}
        self.ground = 0.0

    def char(self, cid, pos, height, width, dirc=(0.0, 0.0, 0.0), listed=True, dead=0, ctype=0x20, size=0):
        self.chars[cid] = dict(pos=pos, height=height, width=width, dirc=dirc, listed=listed, dead=dead,
                               ctype=ctype, size=size)

    @staticmethod
    def char_line(cid, c):
        return "char %x " % cid + hexs(*map(fb, c["pos"] + (c["height"], c["width"]) + c["dirc"])) + \
            " %x %x %x %x" % (int(c["listed"]), c["dead"] & 0xFFFFFFFF, c["ctype"], c["size"])

    def run(self, frames):
        sm = SpellMachine(seed=self.seed, gseed=self.gseed, area=self.area, dungeon=self.dungeon,
                          field_type=self.field_type)
        sm.set_player(self.player)
        sm.set_camera(*self.camera)
        sm.ground = fb(self.ground)
        lines = ["reset %x %x %x %x %x" % (self.seed, self.gseed, self.area, self.dungeon, self.field_type),
                 "player " + hexs(*map(fb, self.player)), "ground " + hexs(fb(self.ground)),
                 "camera " + hexs(*map(fb, self.camera[0] + self.camera[1] + self.camera[2]))]
        for cid, c in self.chars.items():
            sm.add_spell_char(cid, c["pos"], c["height"], c["width"], c["dirc"], c["listed"], c["dead"], c["ctype"],
                              c["size"])
            lines.append(self.char_line(cid, c))
        sm.request(0, self.sid, self.stype, self.creator, self.target)
        lines.append("skill 0 %x %x %x %x" % (self.sid, self.stype, self.creator & 0xFFFFFFFF,
                                             self.target & 0xFFFFFFFF))
        want = []
        for f in range(frames):
            for cid, field, value in self.changes.get(f, []):
                c = self.chars[cid]
                c[field] = value
                if field == "pos":
                    sm.move_char(cid, value)
                else:
                    sm.set_state(cid, c["listed"], c["dead"], c["size"])
                lines.append(self.char_line(cid, c))
            lines.append("frame")
            fr = sm.frame()
            want.append(fr)
            if not fr["skills"][0]["live"] and not fr["slots"] and not fr["elements"] and not fr["eff2"] \
                    and f > 2:
                break
        got = [g for g in ask(lines) if g]
        return want, got


def ask(lines):
    return base.ask(lines, EXAMPLE)


def mask(frame):
    """What neither side can know: a convergence piece's pos.w until it
    moves (effSkillChargeObj adds 1 to the stack's leftover), a summoned
    creature's offset.z and w (never set)."""
    for s in frame.get("slots") or []:
        if 102 <= s["id"] <= 112 and s["flags"] == 0:
            s["pos"] = s["pos"][:3] + [None]
        # effSummonsElement leaves offset.z and w as the stack had them.
        if 57 <= s["id"] <= 63 or s["id"] in (164, 165):
            s["offset"] = s["offset"][:2] + [None, None]
    return frame


def compare(want, got, what):
    """The first difference of two frames, or None."""
    mask(want)
    mask(got)
    for k in want:
        if want[k] != got.get(k):
            if k in ("slots", "elements", "eff2", "skills") and isinstance(want[k], list):
                if len(want[k]) != len(got.get(k) or []):
                    return "%s: %s: %d against %d" % (what, k, len(want[k]), len(got.get(k) or []))
                for a, b in zip(want[k], got[k]):
                    if a != b:
                        if isinstance(a, dict):
                            diff = {kk: (a[kk], b.get(kk)) for kk in a if a[kk] != b.get(kk)}
                        else:
                            diff = (a, b)
                        return "%s: %s differs: %s" % (what, k, diff)
            if k == "draws" and len(want[k]) == len(got.get(k) or []):
                for i, (a, b) in enumerate(zip(want[k], got[k])):
                    if a != b:
                        return "%s: draw %d: %s against %s" % (what, i, a, b)
            return "%s: %s: %s against %s" % (what, k, str(want[k])[:1500], str(got.get(k))[:1500])
    return None


def random_case(rnd, sid):
    """A cast at random places, sizes, headings and seeds; sometimes the
    target is off the lists, down, or moves."""
    c = Case(sid, rnd.randrange(1, 1 << 31), gseed=rnd.randrange(1, 1 << 31), area=rnd.choice([1, 1, 2]),
             dungeon=rnd.choice([0, 0, 1]), field_type=rnd.choice([0, 4]))
    px, py = rnd.uniform(-20000, 20000), rnd.uniform(-20000, 20000)
    c.player = (px, py, rnd.uniform(-100, 300))
    tx, ty = px + rnd.uniform(-900, 900), py + rnd.uniform(-900, 900)
    tz = rnd.uniform(-50, 200)
    c.ground = rnd.choice([tz, tz - rnd.uniform(0, 100), tz + rnd.uniform(-20, 60), 0.0])
    ex, ey = tx + rnd.uniform(-1500, 1500), ty + rnd.uniform(-1500, 1500)
    c.camera = ((ex, ey, tz + rnd.uniform(100, 900)), (ex, ey, tz + rnd.uniform(100, 900)),
                (tx + rnd.uniform(-200, 200), ty + rnd.uniform(-200, 200), tz))
    c.char(0, (px + rnd.uniform(-300, 300), py + rnd.uniform(-300, 300), rnd.uniform(-50, 200)),
           rnd.uniform(80, 240), rnd.uniform(20, 80), (0.0, 0.0, rnd.uniform(-3.14, 3.14)), ctype=0x7)
    c.char(1, (tx, ty, tz), rnd.uniform(60, 400), rnd.uniform(10, 300), (0.0, 0.0, rnd.uniform(-3.14, 3.14)),
           ctype=rnd.choice([0x20, 0x20, 0xA0]), size=rnd.choice([0, 1, 2, 3, 4]))
    for i in range(2, 2 + rnd.randrange(0, 3)):
        c.char(i, (tx + rnd.uniform(-300, 300), ty + rnd.uniform(-300, 300), tz), rnd.uniform(60, 200),
               rnd.uniform(10, 100), ctype=0x20)
    r = rnd.random()
    if r < 0.05:
        c.chars[1]["listed"] = False
    elif r < 0.12:
        c.changes[rnd.randrange(1, 90)] = [(1, "dead", 2)]
    elif r < 0.2:
        c.changes[rnd.randrange(1, 90)] = [(1, "listed", False)]
    elif r < 0.4:
        for f in range(0, 120, rnd.randrange(3, 20)):
            c.changes.setdefault(f, []).append((1, "pos", (tx + rnd.uniform(-100, 100), ty + rnd.uniform(-100, 100),
                                                           tz + rnd.uniform(-30, 30))))
    if rnd.random() < 0.1:
        c.creator = -1
    return c


def _levels(bases, levels):
    return [b + lv for b in bases for lv in levels]


# The ported spells by system: every attack spell.
SPELLS = {
    "tornado": _levels([196, 208, 228, 240, 260], [1, 2, 3, 4]),
    "fall": _levels([192, 224, 256, 272], [1, 2, 3, 4]),
    "convergence": _levels([212, 232, 244, 264, 276], [1, 2, 3, 4]),
    "upheaval": _levels([200, 216, 248, 280], [1, 2, 3, 4]),
    "summons": _levels([204, 220, 236, 252, 268, 284], [1, 2, 3, 4]) + [289, 290, 291, 292, 293, 294],
}


def run_cases(sids, n, seed, frames=400, verbose=False):
    """n random casts of each spell; returns (frames compared, the first
    mismatch or None)."""
    rnd = random.Random(seed)
    total = 0
    for i in range(n):
        for sid in sids:
            c = random_case(rnd, sid)
            want, got = c.run(frames)
            for f, (a, b) in enumerate(zip(want, got)):
                d = compare(a, b, "sid %d case %d frame %d" % (sid, i, f))
                if d:
                    return total, d
                total += 1
            if len(want) != len(got):
                return total, "sid %d case %d: %d frames against %d" % (sid, i, len(want), len(got))
    return total, None


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class SpellsAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        base.build("spell_probe")

    def check(self, name, n=2, seed=1):
        total, d = run_cases(SPELLS[name], n, seed)
        self.assertIsNone(d)
        self.assertGreater(total, 0)

    def test_tornado(self):
        self.check("tornado")

    def test_fall(self):
        self.check("fall")

    def test_convergence(self):
        self.check("convergence")

    def test_upheaval(self):
        self.check("upheaval")

    def test_summons(self):
        self.check("summons")


def game_spell(sid, seed=1, frames=200, target_size=0):
    """Run one spell in the game alone and print what happens: a look."""
    sm = SpellMachine(seed=seed, gseed=seed)
    sm.set_player((0.0, 0.0, 0.0))
    sm.set_camera((0.0, -900.0, 500.0), (0.0, -900.0, 500.0), (0.0, 0.0, 100.0))
    sm.add_spell_char(0, (0.0, -300.0, 0.0), 160.0, 40.0, ctype=0x7)
    sm.add_spell_char(1, (30.0, 250.0, 0.0), 140.0, 60.0, ctype=0x20, size=target_size)
    sm.request(0, sid, 0, 0, 1)
    out = []
    for f in range(frames):
        fr = sm.frame()
        out.append(fr)
        if not fr["skills"][0]["live"] and not fr["slots"] and not fr["elements"] and not fr["eff2"]:
            break
    return out


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        names = sys.argv[3].split(",") if len(sys.argv) > 3 else list(SPELLS)
        base.build("spell_probe")
        for name in names:
            total, d = run_cases(SPELLS[name], int(sys.argv[2]), 12345)
            print(name, "frames compared", total, "mismatch" if d else "ok", d or "")
    elif len(sys.argv) > 3 and sys.argv[1] == "ids":
        # ids N SID,SID,...: N random casts of each, one line a spell
        base.build("spell_probe")
        grand = 0
        for sid in map(int, sys.argv[3].split(",")):
            total, d = run_cases([sid], int(sys.argv[2]), 12345 + sid)
            grand += total
            print(sid, "frames compared", total, "mismatch" if d else "ok", d or "", flush=True)
        print("all", grand)
    elif len(sys.argv) > 2 and sys.argv[1] == "one":
        # one SID [N [SEED]]: N random casts of one spell
        base.build("spell_probe")
        n = int(sys.argv[3]) if len(sys.argv) > 3 else 1
        seed = int(sys.argv[4]) if len(sys.argv) > 4 else 1
        print(run_cases([int(sys.argv[2])], n, seed))
    elif len(sys.argv) > 2 and sys.argv[1] == "look":
        for f, fr in enumerate(game_spell(int(sys.argv[2]))):
            s = fr["skills"][0]
            print(f, "skill", s["live"], s["count"], s["level"], s["flags"], s["effPtr"], s["effElm"],
                  "slots", [(x["i"], x["id"]) for x in fr["slots"]], "elm", fr["elements"], "eff2",
                  [(x["i"], x["id"], x["eff"]) for x in fr["eff2"]], "draws", len(fr["draws"]), "gens",
                  [g["row"] for g in fr["gens"]], "ev", fr["events"], "rand", fr["rand"], fr["genrand"])
    else:
        unittest.main()
