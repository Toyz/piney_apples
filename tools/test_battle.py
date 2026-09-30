#!/usr/bin/env python3
"""tools/battle.py against the game's own battle code.

Each gcmn.prg function is run in tools/eemu.py over scratch ccChar,
ccSpcParam / ccEnemyParam and ccCharBaseParam structures filled with random
values, with the real enemy, boss, skill, equipment and level-up tables in
memory (an enemy row the case needs different is written into enemyTbl and
put back afterwards). Everything the function draws or writes is compared
with battle.py: the return value, every field of the structures, the
newlib rand() state, and the calls it makes to the effect, particle, menu
and EntryAffect functions, which are stubbed to record their arguments.
Skipped when the executable is not present.

The other volumes (Mutation, Outbreak, Quarantine), when extracted with a
.syms sidecar, run the same checks against their own code: every address the
harness uses is a carried name checked against the code that uses it, or is
read from that code (cmndEneRoot, the ccSkill fields RecoverySystem reads,
where ccGetCharParam keeps each character's saved ccSpcParam, the number of
characters ccExpDistributor walks). The struct offsets the harness writes
are the same immediates in all four volumes' functions (compared per
function, register allocation aside) except the ccSkill ones.

    python3 tools/test_battle.py                 the unit tests (a sample)
    python3 tools/test_battle.py bulk N [ELF]    N cases of every check, with counts
"""

import os
import random
import struct
import sys
import types
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
OTHERS = [os.path.join(ROOT, "work", v, "disc", e) for v, e in
          (("mutation", "SLUS_205.62"), ("outbreak", "SLUS_205.63"),
           ("quarantine", "SLUS_205.64"))]

ATT, TGT = 0x01000000, 0x01001000           # ccChar-derived objects, 0x400 each
ATT_P, TGT_P = 0x01002000, 0x01003000       # ccSpcParam / ccEnemyParam / ccBossParam
ATT_B, TGT_B = 0x01004000, 0x01005000       # ccCharBaseParam of an enemy or boss
SAVE, MENU, GAME, SYS = 0x01010000, 0x01020000, 0x01021000, 0x01022000
SKILL, AI, SCRATCH = 0x01030000, 0x01031000, 0x01032000
SAVE2 = 0x01040000                          # the second save block (Mutation on)
DC_EP, DC_NEW = 0x01050000, 0x01050100      # condition effects: the old, the new


class GameBattle:
    """The battle functions of gcmn.prg in the interpreter."""

    def __init__(self, data):
        import battle
        import eemu
        from image import Program
        self.b = battle
        self.eemu = eemu
        self.data = data
        self.prog = Program(data.elf_path, "gcmn")
        self.m = eemu.Machine(self.prog)
        self.sym = self._sym
        m = self.m
        m.store(self.sym("saveData"), 4, SAVE)
        m.store(self.sym("ccMenu"), 4, MENU)
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("ccSys"), 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)           # a bump allocator the skill code borrows from
        self.rand_state = m.load(self.sym("_impure_ptr"), 4) + 168
        # ccCheckTarget reads cmndPcRoot, cmndEneRoot, cmndObjRoot in that
        # order; cmndEneRoot is not carried to Outbreak and Quarantine
        self.cmnd_pc, self.cmnd_ene, self.cmnd_obj = [
            a for _, a in battle.formed(self.prog, "ccCheckTarget__FP6ccChar")]
        # ccSkill.creator and .target: +0x70/+0x74 in Infection, +0x80/+0x84
        # from Mutation on. RecoverySystem's first load from `this` is the
        # target; the creator is the word before it.
        _, w = battle.function_words(self.prog, "RecoverySystem__7ccSkillFv")
        self.sk_target = next(x & 0xFFFF for x in w if x >> 26 == 0x23 and (x >> 21) & 31 == 4)
        self.sk_creator = self.sk_target - 4
        # Where ccGetCharParam finds each character's saved ccSpcParam:
        # saveData +0x7488 + 0xDC * id in Infection; from Mutation on ids
        # 18-20 are in a second block behind a pointer next to saveData
        # (+0x4b4). Point every other global it reads at scratch memory and
        # ask the game's own function.
        for a in self.globals_read("ccGetCharParam__Fi"):
            if a != self.sym("saveData"):
                m.store(a, 4, SAVE2)
        self.spc = [m.call(self.sym("ccGetCharParam__Fi"), (i,)) for i in range(len(data.chars))]
        # ccExpDistributor walks every character (18, then 21)
        self.members = battle.loop_bound(self.prog, "ccExpDistributor__Fs")
        self.calls = []
        self.names = {}
        self.env = battle.Env()
        self.annihilated = 0
        rec = self.record
        hooks = {
            "ccParticleAttributeGuard__FP6ccCharP6ccChar":
                lambda m, a, b, *_: rec("ccParticleAttributeGuard", a, b),
            "ccParticleCritical__FP6ccChar": lambda m, a, *_: rec("ccParticleCritical", a),
            "ccParticleDying__FP6ccChar": lambda m, a, *_: rec("ccParticleDying", a),
            "ccParticleNoDamage__FP6ccChar": lambda m, a, *_: rec("ccParticleNoDamage", a),
            "EntryAffect__6ccCharFP6ccCharssss": self.entry_affect,
            "effDrainCtrl__FP6ccCharP6ccChariii":
                lambda m, a, b, c, d: rec("effDrainCtrl", a, b, c, d, m.r[8]),
            "effProtect__FP6ccCharii": lambda m, a, b, c, *_: rec("effProtect", a, b, c),
            "SetProtect__10ccMenuCtrlFiP6ccChar": lambda m, a, b, c, *_: rec("SetProtect", b, c),
            "ccEntryRecoveryReq__FP6ccChari": lambda m, a, b, *_: rec("ccEntryRecoveryReq", a, b),
            "DispConditionEffect__6ccCharFv": lambda m, a, *_: rec("DispConditionEffect", a),
            "CheckMenuType__10ccMenuCtrlFv": lambda m, *_: self.env.menu_type,
            "CheckSpRegeneSpeed__9ccSpcCharFv": lambda m, *_: self.env.sp_regene_speed,
            "ccEntryFlyFontNewExp__FiiPfP6ccChar": lambda m, *_: 0,
            "sceVu0CopyVector": lambda m, *_: 0,
            "checkPartyMenberNum__Fi": lambda m, i, *_: self.party_index(i),
            "ChatMessageAttributeCritical__4ccAIFv":
                lambda m, a, *_: rec("ChatMessageAttributeCritical", a),
            "ccParticleAttributeCritical__FP6ccChar":
                lambda m, a, *_: rec("ccParticleAttributeCritical", a),
            "effHealSkill__FP6ccChari": lambda m, a, b, *_: rec("effHealSkill", a, b),
            "effCure__FP6ccChar": lambda m, a, *_: rec("effCure", a),
            "effSanity__FP6ccChar": lambda m, a, *_: rec("effSanity", a),
            "effResurrect__FP6ccChar": lambda m, a, *_: rec("effResurrect", a),
            "checkPartyAnnihilation__Fv": lambda m, *_: self.annihilated,
            "ChatMessageAttack__4ccAIFP6ccCharii":
                lambda m, a, b, c, d: rec("ChatMessageAttack", a, b, c, d),
        }
        for name, fn in hooks.items():
            m.hooks[self.sym(name)] = fn

    def _sym(self, name):
        """The volume's address of a symbol; a name the volume's sidecar
        lacks (Outbreak's cmndTarget) is Infection's, carried (volume.va)."""
        s = self.prog.symbol_named(name)
        if s is not None:
            return s.value
        inf = volume.program(volume.INF_ELF, "gcmn").symbol_named(name)
        if inf is None:
            raise KeyError(f"{name} is in neither {volume.NAME} nor Infection")
        return inf_va(inf.value)

    def globals_read(self, name):
        """The addresses a function and the functions it calls form (one
        level: ccGetCharParam's helper in main has no name)."""
        import xfer
        va, w = self.b.function_words(self.prog, name)
        out = [a for _, a in self.b.formed(self.prog, name)]
        for i, x in enumerate(w):
            if x >> 26 == 3:                                        # jal
                t = ((va + 4 * i + 4) & 0xF0000000) | ((x & 0x3FFFFFF) << 2)
                cw = []
                while len(cw) < 256 and (not cw or cw[-2:-1] != [0x03E00008]):
                    cw.append(self.prog.u32(t + 4 * len(cw)))      # to jr $ra + delay slot
                out += xfer.addresses(cw, t, self.prog.gp).values()
        return out

    # stubs ------------------------------------------------------------------
    def name(self, v):
        v &= 0xFFFFFFFF
        return self.names.get(v, v - (1 << 32) if v & 0x80000000 else v)

    def record(self, fn, *args):
        self.calls.append((fn,) + tuple(self.name(a) for a in args))
        return 0

    def entry_affect(self, m, this, ch, t, p0):
        s16 = self.b.s16
        self.calls.append(("EntryAffect", self.name(this), self.name(ch), s16(t), s16(p0),
                           s16(m.r[8]), s16(m.r[9])))
        return 0

    def party_index(self, i):
        return self.party.get(i, -1)

    # memory helpers -----------------------------------------------------------
    def put(self, va, fmt, *v):
        self.m.mem[va:va + struct.calcsize(fmt)] = struct.pack(fmt, *v)

    def get(self, va, fmt):
        return struct.unpack(fmt, bytes(self.m.mem[va:va + struct.calcsize(fmt)]))

    def put_shorts(self, va, vals):
        self.put(va, f"<{len(vals)}h", *vals)

    def shorts(self, va, n):
        return list(self.get(va, f"<{n}h"))

    def set_rand(self, state):
        self.m.store(self.rand_state, 8, state)

    def rand_now(self):
        return self.m.load(self.rand_state, 8)

    def restore(self, va, n):
        self.m.mem[va:va + n] = self.m.pristine[va:va + n]

    def clear(self):
        for va in (ATT, TGT, ATT_P, TGT_P, ATT_B, TGT_B):
            self.m.mem[va:va + 0x400] = bytes(0x400)

    # fighters --------------------------------------------------------------------
    def put_char(self, ch, va, pva, bva):
        """A ccChar and what it points at; ch is a battle.PC, battle.Foe or
        a namespace with the same fields."""
        m = self.m
        if ch.type & self.b.PC_TYPES:
            bva = pva               # a party member's base is its ccSpcParam
            self.put_spc(ch, pva)
        else:
            self.put(bva + 8, "<i", ch.type)
            self.put(bva + 12, "<hhh", ch.id, ch.level, getattr(ch, "exp", 0))
            self.put_shorts(pva, ch.real + ch.temp + ch.time)
            self.put(pva + 0x60, "<hhh", ch.PP, ch.PPcount, getattr(ch, "PPrestore", 0))
        m.store(va, 4, bva)
        m.store(va + 4, 4, pva)
        self.put_shorts(va + 8, ch.cond)
        m.store(va + 0x28, 4, ch.speed_value)
        m.store(va + 0x30, 4, getattr(ch, "condition_num", 0))
        self.put(va + 0x70, "<hhhh", ch.HP, ch.SP, ch.maxHP, ch.maxSP)
        m.store(va + 0xE0, 1, 0x80 if getattr(ch, "no_death", False) else 0)
        m.store(va + 0x140, 4, ch.ent_root)     # entParam.entRoot; weaponPos[0] on a ccSpcChar
        # ccSpcChar.partyFlag and .ai (on an enemy these bytes are other fields)
        m.store(va + 0xE0, 4, m.load(va + 0xE0, 4) | (getattr(ch, "party_flag", 0) & 7) << 14)
        m.store(va + 0x128, 4, AI if getattr(ch, "ai", 0) else 0)

    def put_spc(self, pc, pva):
        self.put(pva + 8, "<i", pc.type)
        self.put(pva + 12, "<hhh", pc.id, pc.level, pc.exp)
        self.put(pva + 0x24, "<hh", pc.p_maxHP, pc.p_maxSP)
        self.put_shorts(pva + 0x28, pc.elm + pc.real + pc.tune + pc.temp + pc.time)
        self.put_shorts(pva + 0xC8, pc.equip)
        self.put(pva + 0xD8, "<h", pc.job)

    def read_char(self, va, pva, pc):
        out = {"cond": self.shorts(va + 8, 16), "speedValue": self.m.load(va + 0x28, 4),
               "conditionNum": self.m.load(va + 0x30, 4, True),
               "HP/SP": self.shorts(va + 0x70, 4)}
        if pc:
            out.update(level_exp=self.shorts(pva + 14, 2), pmax=self.shorts(pva + 0x24, 2),
                       elm=self.shorts(pva + 0x28, 16), real=self.shorts(pva + 0x48, 16),
                       tune=self.shorts(pva + 0x68, 16), temp=self.shorts(pva + 0x88, 16),
                       time=self.shorts(pva + 0xA8, 16))
        else:
            out.update(real=self.shorts(pva, 16), temp=self.shorts(pva + 0x20, 16),
                       time=self.shorts(pva + 0x40, 16), PP=self.shorts(pva + 0x60, 3))
        return out

    @staticmethod
    def py_char(ch, pc):
        out = {"cond": list(ch.cond), "speedValue": ch.speed_value,
               "conditionNum": getattr(ch, "condition_num", 0),
               "HP/SP": [ch.HP, ch.SP, ch.maxHP, ch.maxSP]}
        if pc:
            out.update(level_exp=[ch.level, ch.exp], pmax=[ch.p_maxHP, ch.p_maxSP], elm=ch.elm,
                       real=ch.real, tune=ch.tune, temp=ch.temp, time=ch.time)
        else:
            out.update(real=ch.real, temp=ch.temp, time=ch.time,
                       PP=[ch.PP, ch.PPcount, getattr(ch, "PPrestore", 0)])
        return out

    def put_row(self, f):
        """Write the foe's (possibly altered) table row into the image."""
        va = f.row["va"]
        self.put(va + 0x24, "<hh", f.row["maxHP"], f.row["maxSP"])
        self.put_shorts(va + 0x28, f.row["elm"] + f.row["beff"])
        self.put(va + 0x52, "<hhh", f.row["maxPP"], f.row["pDefPP"], f.row["mDefPP"])
        self.put(va + 0x64, "<h", f.row["Exdefense"])

    def put_env(self, env):
        self.env = env
        self.m.store(SAVE + 0x6771, 1, env.plcol)
        self.put(MENU + 0xFE, "<h", env.menu_flag)
        self.m.store(GAME + 0x14, 4, env.area)
        self.m.store(GAME + 0x58, 4, env.in_battle)
        self.m.store(SYS + 0x358, 4, env.count)

    def put_skill(self, sk):
        self.put(SKILL + 8, "<hh", sk["atk"], sk["hit"])
        self.put_shorts(SKILL + 12, sk["attr"])
        self.put(SKILL + 0x18, "<i", sk["condition"])
        self.put(SKILL + 0x28, "<ii", sk["cost"], sk["type"])
        self.put(SKILL + 0x32, "<h", sk["dmgRate"])

    # the functions ---------------------------------------------------------------
    def calc_battle_damage(self, att, tgt, sk, mag, h, env, state, listed=True):
        self.clear()
        self.names = {ATT: "att", TGT: "t"}
        self.calls = []
        self.put_char(att, ATT, ATT_P, ATT_B)
        self.put_char(tgt, TGT, TGT_P, TGT_B)
        rows = [f for f in (att, tgt) if hasattr(f, "row")]
        for f in rows:
            self.put_row(f)
        self.put_skill(sk)
        self.put_env(env)
        m = self.m
        m.store(self.cmnd_pc, 4, ATT)
        m.store(ATT + 0xBC, 4, TGT if listed else 0)
        m.store(TGT + 0xBC, 4, 0)
        m.store(self.cmnd_ene, 4, 0)
        m.store(self.cmnd_obj, 4, 0)
        self.set_rand(state)
        m.f[12] = mag
        ret = m.call(self.sym("CalcBattleDamage__6ccCharFP6ccCharP12ccSkillParamfi"),
                     (ATT, TGT, SKILL, h & 0xFFFFFFFF))
        out = {"ret": self.b.s32(ret), "rand": self.rand_now(), "calls": self.calls,
               "tgt": self.read_char(TGT, TGT_P, tgt.type & self.b.PC_TYPES)}
        for f in rows:
            self.restore(f.row["va"], 0x68)
        return out

    def skill_damage(self, att, tgt, sk, sid, ac_flag, env, state):
        """ccSkillDamage with a single-target skill (targetRange 0)."""
        self.clear()
        self.names = {ATT: "att", TGT: "t", AI: "ai"}
        self.calls = []
        self.put_char(att, ATT, ATT_P, ATT_B)
        self.put_char(tgt, TGT, TGT_P, TGT_B)
        rows = [f for f in (att, tgt) if hasattr(f, "row")]
        for f in rows:
            self.put_row(f)
        self.put_skill(sk)
        self.put(SKILL + 0x20, "<f", 0.0)
        self.put_env(env)
        m = self.m
        m.store(self.cmnd_pc, 4, ATT)
        m.store(ATT + 0xBC, 4, TGT)
        m.store(TGT + 0xBC, 4, 0)
        m.store(self.cmnd_ene, 4, 0)
        m.store(self.cmnd_obj, 4, 0)
        self.set_rand(state)
        self.put(SCRATCH + 0x100, "<h", ac_flag)
        ret = m.call(self.sym("ccSkillDamage__FP6ccCharP6ccCharP12ccSkillParamRsi"),
                     (ATT, TGT, SKILL, SCRATCH + 0x100, sid))
        out = {"ret": self.b.s32(ret), "ac": self.shorts(SCRATCH + 0x100, 1)[0],
               "rand": self.rand_now(), "calls": self.calls,
               "tgt": self.read_char(TGT, TGT_P, tgt.type & self.b.PC_TYPES)}
        for f in rows:
            self.restore(f.row["va"], 0x68)
        return out

    def recovery(self, creator, tgt, sid, param):
        """ccSkillRecovery with a single-target heal."""
        self.clear()
        self.names = {ATT: "creator", TGT: "t"}
        self.calls = []
        self.put_char(creator, ATT, ATT_P, ATT_B)
        self.put_char(tgt, TGT, TGT_P, TGT_B)
        m = self.m
        m.store(self.cmnd_pc, 4, ATT)
        m.store(ATT + 0xBC, 4, 0)
        ret = m.call(self.sym("ccSkillRecovery__FP6ccCharP6ccCharii"), (ATT, TGT, sid, param))
        return {"ret": ret, "calls": self.calls}

    def cure(self, creator, tgt, sid, count, stype, annihilated, anm_flag):
        """ccSkill::RecoverySystem over a scratch ccSkill."""
        self.clear()
        self.names = {ATT: "creator", TGT: "t"}
        self.calls = []
        self.annihilated = annihilated
        self.put_char(creator, ATT, ATT_P, ATT_B)
        self.put_char(tgt, TGT, TGT_P, TGT_B)
        self.put(ATT + 0x7C, "<hh", 55, 3)                  # skillID, skillStatus
        self.put(ATT + 0xF4, "<h", anm_flag)                # ccSpcChar.anmFlag
        m = self.m
        m.store(self.cmnd_pc, 4, TGT)
        m.store(TGT + 0xBC, 4, 0)
        sko = SCRATCH + 0x200
        m.mem[sko:sko + 0xB0] = bytes(0xB0)
        self.put(sko, "<i", sid)
        m.store(sko + 12, 1, stype & 0xF)
        self.put(sko + 0x14, "<h", count)
        m.store(sko + self.sk_creator, 4, ATT)
        m.store(sko + self.sk_target, 4, TGT)
        m.call(self.sym("RecoverySystem__7ccSkillFv"), (sko,))
        out = self.read_char(TGT, TGT_P, tgt.type & self.b.PC_TYPES)
        out.update(calls=self.calls, end=(m.load(sko + 12, 1) >> 4) & 3,
                   caster=self.shorts(ATT + 0x70, 2) + self.shorts(ATT + 0x7C, 2))
        return out

    def skill_damage_value(self, cp, tp, sid):
        self.clear()
        self.names = {}
        self.calls = []
        self.put_char(cp, ATT, ATT_P, ATT_B)
        self.put_char(tp, TGT, TGT_P, TGT_B)
        rows = [f for f in (cp, tp) if hasattr(f, "row")]
        for f in rows:
            self.put_row(f)
        for f in (cp, tp):               # ccGetCharParam reads the saved copy
            if f.type & self.b.PC_TYPES and 0 <= f.id < len(self.spc):
                self.put_spc(f, self.spc[f.id])
        m = self.m
        m.store(self.cmnd_pc, 4, ATT)
        m.store(ATT + 0xBC, 4, TGT)
        m.store(TGT + 0xBC, 4, 0)
        ret = m.call(self.sym("ccSkillDamageValue__FP6ccCharP6ccChari"), (ATT, TGT, sid))
        out = (self.b.s32(ret), m.load(self.sym("SkillDamageValueAttributeCritical"), 4),
               m.load(self.sym("SkillDamageValueAttributeGuard"), 4))
        for f in rows:
            self.restore(f.row["va"], 0x68)
        return out

    def calc_real(self, ch, flag, env):
        self.clear()
        self.names = {ATT: "self"}
        self.calls = []
        pc = bool(ch.type & self.b.PC_TYPES)
        self.put_char(ch, ATT, ATT_P, ATT_B)
        if not pc:
            self.put_row(ch)
        self.put_env(env)
        self.m.call(self.sym("CalcReal__6ccCharFi"), (ATT, flag & 0xFFFFFFFF))
        out = self.read_char(ATT, ATT_P, pc)
        out["calls"] = self.calls
        if not pc:
            self.restore(ch.row["va"], 0x68)
        return out

    def run_char_fn(self, name, ch, args=()):
        self.clear()
        self.names = {ATT: "self"}
        self.calls = []
        self.put_char(ch, ATT, ATT_P, ATT_B)
        ret = self.m.call(self.sym(name), (ATT,) + tuple(a & 0xFFFFFFFF for a in args))
        out = self.read_char(ATT, ATT_P, True)
        out["ret"] = ret
        return out

    def disp_condition(self, ch, ep_num, shown, eye):
        """ccChar::DispConditionEffect run itself on `ch`, whose effect
        (+0x2c) is one of number `ep_num` (None: no effect), with
        ccSpcConditionEffectSW() answering `shown` and checkCameraType() 1
        for `eye`: the number left (+0x30), the effect left, and the
        effect calls (a kill of no effect left out)."""
        self.clear()
        self.names = {ATT: "self", DC_EP: "old", DC_NEW: "new"}
        self.calls = []
        self.put_char(ch, ATT, ATT_P, ATT_B)
        m = self.m
        if ep_num is None:
            m.store(ATT + 0x2C, 4, 0)
        else:
            m.store(ATT + 0x2C, 4, DC_EP)
            m.store(DC_EP + 0x1C, 4, ep_num & 0xFFFFFFFF)
        rec = self.record

        def kill(m_, a, *_):
            if a:
                rec("kill", a)
            return 0

        def new(m_, a, *_):
            self.calls.append(("set", m.load(ATT + 0x30, 4, True)))
            return DC_NEW

        hooks = {
            "killConditionEffect__FP17ccConditionEffect": kill,
            "deleteConditionEffect__FP17ccConditionEffect": lambda m_, a, *_: rec("delete", a),
            "setConditionEffect__FP6ccChar": new,
            "ccSpcConditionEffectSW__Fv": lambda m_, *_: int(shown),
            "checkCameraType__Fv": lambda m_, *_: 1 if eye else 0,
        }
        saved = {}
        for name, fn in hooks.items():
            va = self.sym(name)
            saved[va] = m.hooks.get(va)
            m.hooks[va] = fn
        disp = self.sym("DispConditionEffect__6ccCharFv")
        saved[disp] = m.hooks.pop(disp, None)
        try:
            m.call(disp, (ATT,))
        finally:
            for va, fn in saved.items():
                if fn is None:
                    m.hooks.pop(va, None)
                else:
                    m.hooks[va] = fn
        ep = m.load(ATT + 0x2C, 4)
        return {"num": m.load(ATT + 0x30, 4, True), "ep": self.name(ep) if ep else 0,
                "calls": [list(c) for c in self.calls]}

    def set_level_param(self, pc, lvs):
        self.clear()
        self.put_spc(pc, ATT_P)
        self.m.call(self.sym("ccSetLevelParam__FP10ccSpcParami"), (ATT_P, lvs & 0xFFFFFFFF))
        return {"level": self.shorts(ATT_P + 14, 1)[0], "pmax": self.shorts(ATT_P + 0x24, 2),
                "elm": self.shorts(ATT_P + 0x28, 16)}

    def modify_condition(self, creator, tgt, sid):
        self.clear()
        self.names = {ATT: "creator", TGT: "t"}
        self.calls = []
        self.put_char(creator, ATT, ATT_P, ATT_B)
        self.put_char(tgt, TGT, TGT_P, TGT_B)
        self.m.call(self.sym("_ccSkillModifyCondition__FP6ccCharP6ccChari"), (ATT, TGT, sid))
        out = self.read_char(TGT, TGT_P, tgt.type & self.b.PC_TYPES)
        out["calls"] = self.calls
        return out

    def condition_success(self, tgt, sid, state):
        self.clear()
        self.put_char(tgt, TGT, TGT_P, TGT_B)
        m = self.m
        m.store(self.cmnd_pc, 4, TGT)
        m.store(TGT + 0xBC, 4, 0)
        self.set_rand(state)
        ret = m.call(self.sym("ccCheckConditionSkillSuccess__FP6ccChari"), (TGT, sid))
        return {"ret": ret, "rand": self.rand_now()}

    def exp_distributor(self, lv, party, members, flags, exp_flags, count):
        """ccExpDistributor over up to three party ccChars and the saved
        ccSpcParam of the others; returns every exp and the erosion."""
        m = self.m
        self.clear()
        self.party = {}
        slots = [ATT, TGT, TGT + 0x800]
        for n, (i, pc) in enumerate(party):
            self.party[i] = n
            pva = [ATT_P, TGT_P, TGT_P + 0x800][n]
            self.m.mem[pva:pva + 0x100] = bytes(0x100)
            self.put_char(pc, slots[n], pva, None)
            m.store(self.sym("ccPartyManager") + 4 * n, 4, slots[n])
        m.mem[SAVE:SAVE + 0x8500] = bytes(0x8500)
        m.mem[SAVE2:SAVE2 + 0x1000] = bytes(0x1000)
        for i, pc in members.items():
            self.put_spc(pc, self.spc[i])
        m.store(SAVE + 0x2220, 4, flags)
        m.store(SAVE + 0x222C, 4, exp_flags)
        self.put(SAVE + 0x676E, "<h", self.erosion)
        m.store(SYS + 0x358, 4, count)
        m.call(self.sym("ccExpDistributor__Fs"), (lv & 0xFFFFFFFF,))
        got = {i: self.shorts([ATT_P, TGT_P, TGT_P + 0x800][n] + 16, 1)[0]
               for n, (i, _) in enumerate(party)}
        got.update({i: self.shorts(self.spc[i] + 16, 1)[0] for i in members})
        return got, self.shorts(SAVE + 0x676E, 1)[0]

    def add_lv_erosion(self, erosion, n, f_bits):
        self.put(SAVE + 0x676E, "<h", erosion)
        self.m.f[12] = f_bits
        self.m.call(self.sym("AddLvErosion__10ccSaveDataFif"), (SAVE, n & 0xFFFFFFFF))
        return self.shorts(SAVE + 0x676E, 1)[0]


# random inputs -----------------------------------------------------------------------

def rnd_short(rnd, lo=-60, hi=400):
    k = rnd.random()
    if k < 0.03:
        return rnd.randrange(-32768, 32768)
    if k < 0.1:
        return rnd.choice((0, 1, -1, 998, 999, 1000, 32767, -32768))
    return rnd.randrange(lo, hi)


def rnd_elm(rnd, lo=-60, hi=400):
    return [rnd_short(rnd, lo, hi) for _ in range(16)]


def rnd_cond(rnd, dead_ok=True):
    c = [0] * 16
    for i in range(16):
        if rnd.random() < 0.3:
            c[i] = rnd_short(rnd, -3, 120) if rnd.random() < 0.8 else rnd.randrange(0, 6000)
    c[0] = rnd.choice((0,) * 12 + (1, 2)) if dead_ok else 0
    return c


def rnd_foe(b, data, rnd, kind=None):
    kind = kind or rnd.choice(("enemy", "boss"))
    rows = data.enemies if kind == "enemy" else data.bosses
    row = dict(rnd.choice(rows))
    f = b.Foe(row, kind)
    row["elm"] = rnd_elm(rnd, 0, 300) if rnd.random() < 0.7 else list(row["elm"])
    row["beff"] = [rnd.choice((0, 0, rnd.randrange(0, 101))) for _ in range(5)]
    row["maxPP"] = rnd.choice((rnd.randrange(0, 400), rnd_short(rnd), row["maxPP"]))
    row["pDefPP"], row["mDefPP"] = rnd_short(rnd, -5, 300), rnd_short(rnd, -5, 300)
    row["Exdefense"] = rnd.choice((0, 0, 0, rnd.randrange(1, 256), rnd.randrange(-32768, 32768)))
    row["maxHP"], row["maxSP"] = rnd_short(rnd, 1, 3000), rnd_short(rnd, 0, 300)
    f.type = rnd.choice({"enemy": (0x20, 0x40, 0x60), "boss": (0x80, 0x88)}[kind])
    f.real = [rnd.choice((v, rnd_short(rnd, 0, 300))) for v in row["elm"]]
    f.temp, f.time = rnd_elm(rnd, -50, 50), [rnd.choice((0, rnd_short(rnd, -3, 200))) for _ in range(16)]
    f.PP = rnd.choice((0, rnd_short(rnd, -3, 400), b.s16(row["maxPP"] - rnd.randrange(0, 40))))
    f.PPcount = rnd.choice((0, 0, rnd_short(rnd, -3, 301)))
    f.PPrestore = rnd.randrange(-1, 4)
    f.HP, f.SP = rnd_short(rnd, 0, 3000), rnd_short(rnd, 0, 300)
    f.maxHP, f.maxSP = rnd_short(rnd, 1, 3000), rnd_short(rnd, 1, 300)
    f.cond = rnd_cond(rnd)
    f.speed_value = rnd.choice((0x3F800000, 0x3F000000, 0x40000000))
    f.ent_root = rnd.choice((0, 1, 7))
    return f


def rnd_pc(b, data, rnd):
    pc = b.PC(data, rnd.choice(data.chars))
    pc.type = rnd.choice((7, 6, 6, 1, 2, 4))
    pc.level = rnd.randrange(1, 100)
    pc.exp = rnd.choice((0, rnd.randrange(0, 1000), rnd.randrange(0, 32768), rnd_short(rnd)))
    pc.p_maxHP, pc.p_maxSP = rnd_short(rnd, 1, 9999), rnd_short(rnd, 1, 999)
    pc.elm = rnd_elm(rnd, -50, 999)
    pc.real, pc.tune = rnd_elm(rnd), rnd_elm(rnd)
    pc.temp = [rnd.choice((0, rnd_short(rnd, -200, 200))) for _ in range(16)]
    pc.time = [rnd.choice((0, rnd_short(rnd, -3, 200))) for _ in range(16)]
    pc.job = rnd.randrange(6)
    pc.equip = [rnd.randrange(len(data.equip[s])) for s in ("head", "body", "arm", "leg")]
    pc.equip += [rnd.randrange(len(data.weapons[pc.job])), 0]
    pc.HP, pc.SP = rnd_short(rnd, 0, 9999), rnd_short(rnd, 0, 999)
    pc.maxHP, pc.maxSP = rnd_short(rnd, 1, 9999), rnd_short(rnd, 1, 999)
    pc.cond = rnd_cond(rnd)
    pc.speed_value = rnd.choice((0x3F800000, 0x3F000000))
    pc.no_death = rnd.random() < 0.2
    pc.ent_root = rnd.choice((0, 1))
    return pc


def rnd_other(b, rnd):
    """A gimmick or other object: none of the PC, enemy or boss bits."""
    o = types.SimpleNamespace(type=rnd.choice((0x100, 0x8, 0x10, 0)), id=0, level=1, exp=0,
                              real=rnd_elm(rnd), temp=[0] * 16, time=[0] * 16, PP=0, PPcount=0,
                              HP=rnd_short(rnd, 0, 999), SP=0, maxHP=100, maxSP=0,
                              cond=rnd_cond(rnd), speed_value=0x3F800000, ent_root=1)
    return o


def rnd_skill(data, rnd):
    if rnd.random() < 0.4:
        return dict(rnd.choice(data.skills))
    t = rnd.choice((1, 2, 1, 2, 3, 0, 1 | 0x1c00, 2 | 0x100))
    t |= rnd.randrange(64) << 2 if rnd.random() < 0.6 else 0
    return {"atk": rnd_short(rnd, -50, 300), "hit": rnd_short(rnd, -200, 300),
            "attr": [rnd.choice((0, 0, rnd_short(rnd, -150, 300))) for _ in range(6)],
            "condition": rnd.choice((0, 0, 0, 1)), "type": t, "cost": rnd.choice((0, 0, 10)),
            "dmgRate": rnd.choice((100, 80, 40, 18, rnd_short(rnd, -50, 300)))}


def rnd_env(b, rnd):
    return b.Env(plcol=rnd.choice((0, 1, 1)), menu_flag=rnd.choice((0, 0, 0, 1)),
                 in_battle=rnd.choice((0, 1)), count=rnd.choice((0, 60, 120, rnd.getrandbits(32))),
                 menu_type=rnd.choice((-1, -1, 0, 3)), sp_regene_speed=rnd.choice((0, 1)),
                 area=rnd.choice((0, 1, 2)))


def copy_fighter(f):
    import copy
    g = copy.copy(f)
    for k in ("real", "temp", "time", "cond", "elm", "tune", "equip"):
        if hasattr(f, k):
            setattr(g, k, list(getattr(f, k)))
    return g


# the checks -------------------------------------------------------------------------

class Checker:
    """Each check runs one random case in the game and in battle.py and
    returns a description of the first difference, or None."""

    def __init__(self, data):
        import battle
        import collections
        self.b = battle
        self.data = data
        self.game = GameBattle(data)
        self.coverage = collections.Counter()

    def damage(self, rnd, protect=False, exdef=False):
        """protect=True aims at the protect gauge: a live hit on a foe with
        plcol set and a low pDefPP/mDefPP. exdef=True aims at Exdefense: a
        foe whose row has some of its bits (and 0x100, the bit that cancels
        them from Mutation on), with the defences they name sometimes
        lowered below the row, and a skill with some of the same bits."""
        b, data = self.b, self.data
        pick = rnd.random()
        att = (rnd_pc(b, data, rnd) if pick < 0.45 else rnd_foe(b, data, rnd) if pick < 0.9
               else rnd_other(b, rnd))
        pick = 0 if protect or exdef else rnd.random()
        tgt = (rnd_foe(b, data, rnd) if pick < 0.6 else rnd_pc(b, data, rnd) if pick < 0.97
               else rnd_other(b, rnd))
        tgt.cond[0] = rnd.choice((0,) * 20 + (1,))
        sk = rnd_skill(data, rnd)
        mag = rnd.choice((0x3F800000, 0x3F800000, 0x40000000, 0x3F000000,
                          self.game.eemu.f_from_py(rnd.uniform(0, 3))))
        h = rnd.choice((-1, -1, -1, -5, 0, rnd.randrange(1, 130)))
        env = rnd_env(b, rnd)
        if protect:
            h, env.plcol, env.menu_flag = -1, 1, 0
            tgt.PPcount = 0
            tgt.row["Exdefense"] = rnd.choice((0, 0, 0, tgt.row["Exdefense"]))
            tgt.row["pDefPP"], tgt.row["mDefPP"] = rnd.randrange(-2, 30), rnd.randrange(-2, 30)
            tgt.row["maxPP"] = rnd.choice((rnd.randrange(0, 30000), rnd.randrange(0, 300)))
            tgt.PP = rnd.randrange(0, max(tgt.row["maxPP"], 1))
        if exdef:
            h = rnd.choice((-1, -1, 0))
            tgt.row["Exdefense"] = rnd.randrange(1, 0x100) | rnd.choice((0, 0, 0x100, 0x200))
            for i in (b.P_DEF, b.M_DEF) + tuple(range(8, 14)):
                tgt.real[i] = b.s16(tgt.row["elm"][i] - rnd.choice((0, 0, 1, 40)))
            sk["type"] = sk["type"] & ~0xFF | rnd.choice((1, 2, 3)) | rnd.randrange(64) << 2
        state = rnd.getrandbits(64)
        listed = rnd.random() > 0.02
        game = self.game.calc_battle_damage(att, tgt, sk, mag, h, env, state, listed)
        rng = b.Rand(state)
        t2 = copy_fighter(tgt)
        res = b.calc_battle_damage(att, t2, sk, mag, h, rng, env, data, listed)
        py = {"ret": res.dmg, "rand": rng.state, "calls": res.events,
              "tgt": self.game.py_char(t2, tgt.type & b.PC_TYPES)}
        self.tally(res)
        return diff(py, game, ("damage", h))

    def tally(self, res):
        """Which paths the damage cases went down, for the coverage line."""
        c = self.coverage
        c["hit" if res.dmg > 0 else "zero" if res.dmg == 0 else "miss"] += 1
        c.update(e[0] for e in res.events)
        if res.pp is not None:
            c["protect"] += 1

    def skill_damage(self, rnd):
        b, data = self.b, self.data
        att = rnd_pc(b, data, rnd) if rnd.random() < 0.6 else rnd_foe(b, data, rnd)
        att.ai = rnd.choice((0, 1))
        att.party_flag = rnd.choice((0, 1, 3))
        tgt = rnd_foe(b, data, rnd) if rnd.random() < 0.7 else rnd_pc(b, data, rnd)
        tgt.cond[0] = rnd.choice((0,) * 20 + (1,))
        sk = rnd_skill(data, rnd)
        sid = rnd.randrange(0, len(data.skills))
        ac = rnd.choice((0, 0, 1, 1, -1))
        env = rnd_env(b, rnd)
        state = rnd.getrandbits(64)
        game = self.game.skill_damage(att, tgt, sk, sid, ac, env, state)
        rng = b.Rand(state)
        t2 = copy_fighter(tgt)
        n, ac2, res = b.skill_damage(att, t2, sk, sid, ac, rng, env, data)
        py = {"ret": n, "ac": ac2, "rand": rng.state, "calls": res.events,
              "tgt": self.game.py_char(t2, tgt.type & b.PC_TYPES)}
        return diff(py, game, ("skill_damage", ac))

    def recovery(self, rnd):
        b, data = self.b, self.data
        creator = rnd_pc(b, data, rnd) if rnd.random() < 0.7 else rnd_foe(b, data, rnd)
        creator.cond[0] = rnd.choice((0, 0, 0, 1))
        tgt = rnd_pc(b, data, rnd) if rnd.random() < 0.7 else rnd_foe(b, data, rnd)
        sid = rnd.choice((150, 151, 152, 295))
        param = rnd.choice((800, rnd_short(rnd)))
        game = self.game.recovery(creator, tgt, sid, param)
        n, ev = b.recovery(creator, tgt, sid, param)
        return diff({"ret": n, "calls": ev}, game, ("recovery", sid))

    def cure(self, rnd):
        b, data = self.b, self.data
        creator = rnd_pc(b, data, rnd) if rnd.random() < 0.7 else rnd_foe(b, data, rnd)
        tgt = rnd_pc(b, data, rnd) if rnd.random() < 0.7 else rnd_foe(b, data, rnd)
        tgt.cond[0] = rnd.choice((0, 0, 0, 1))
        tgt.speed_value = rnd.choice((0x3F800000, 0x3F000000, 0x3FE00000, 0))
        tgt.temp = [rnd.choice((0, rnd_short(rnd, -150, 150))) for _ in range(16)]
        sid = rnd.choice((178, 179, 180, 178, 179, 180, 175, 150))
        count = rnd.choice((0, 0, 0, 1))
        stype = rnd.choice((0, 0, 1, 2, 3))
        ann = rnd.choice((0, 0, 1))
        anm = rnd.choice((0, 1, 1))
        game = self.game.cure(creator, tgt, sid, count, stype, ann, anm)
        c2, t2 = copy_fighter(creator), copy_fighter(tgt)
        c2.skill_id, c2.skill_status = 55, 3
        ev, end = b.cure(c2, t2, sid, data, count, stype, ann, anm)
        py = self.game.py_char(t2, tgt.type & b.PC_TYPES)
        py.update(calls=ev, end=1 if end else 0, caster=[c2.HP, c2.SP, c2.skill_id, c2.skill_status])
        return diff(py, game, ("cure", sid))

    def skill_damage_value(self, rnd):
        b, data = self.b, self.data
        cp = rnd_pc(b, data, rnd)
        cp.id = rnd.randrange(len(data.chars))
        tp = rnd_foe(b, data, rnd) if rnd.random() < 0.8 else rnd_pc(b, data, rnd)
        tp.cond[0] = rnd.choice((0,) * 20 + (1,))
        if rnd.random() < 0.3:      # ties and clear winners in the attributes
            tp_elm = tp.row["elm"] if hasattr(tp, "row") else tp.elm
            tp_elm[8:14] = [rnd.choice((10, 20, 30)) for _ in range(6)]
        sid = rnd.randrange(len(data.skills))
        if rnd.random() < 0.3:      # art bits in combination, through a patched row
            self.data.skills[sid] = dict(self.data.skills[sid], type=1 | rnd.randrange(8) << 10)
            self.game.put(self.data.skills[sid]["va"] + 0x2C, "<i", self.data.skills[sid]["type"])
        game = self.game.skill_damage_value(cp, tp, sid)
        py = b.skill_damage_value(cp, copy_fighter(tp), sid, data)
        self.game.restore(self.data.skills[sid]["va"], 0x38)
        self.data.skills[sid] = self.data._skill(self.data.skills[sid]["va"])
        return diff(py, game, ("skill_damage_value", sid))

    def protect(self, rnd):
        return self.damage(rnd, protect=True)

    def exdefense(self, rnd):
        return self.damage(rnd, exdef=True)

    def calc_real(self, rnd):
        b, data = self.b, self.data
        ch = rnd_pc(b, data, rnd) if rnd.random() < 0.6 else rnd_foe(b, data, rnd)
        pc = isinstance(ch, b.PC)
        flag = rnd.choice((1, 0, 0, -1, 2))
        env = rnd_env(b, rnd)
        game = self.game.calc_real(ch, flag, env)
        c2 = copy_fighter(ch)
        ev = b.calc_real_pc(c2, flag, env) if pc else b.calc_real_foe(c2, flag, env)
        py = self.game.py_char(c2, pc)
        py["calls"] = ev
        return diff(py, game, ("CalcReal", flag))

    def level(self, rnd):
        b, data = self.b, self.data
        pc = rnd_pc(b, data, rnd)
        pc.elm = [rnd.randrange(-20, 1000) for _ in range(16)]
        which = rnd.choice(("up", "down", "set"))
        c2 = copy_fighter(pc)
        if which == "up":
            game = self.game.run_char_fn("CheckLevelUp__6ccCharFv", pc)
            ret = b.check_level_up(c2)
            py = self.game.py_char(c2, True)
            py["ret"] = ret
            game["ret"] = b.s32(game["ret"])
        elif which == "down":
            game = self.game.run_char_fn("LevelDown__6ccCharFv", pc)
            b.level_down(c2)
            py = self.game.py_char(c2, True)
            py["ret"] = game["ret"]
        else:
            lvs = rnd.choice((rnd.randrange(1, 100), rnd.randrange(-3, 130)))
            game = self.game.set_level_param(pc, lvs)
            b.set_level_param(c2, lvs)
            py = {"level": c2.level, "pmax": [c2.p_maxHP, c2.p_maxSP], "elm": c2.elm}
        return diff(py, game, ("level", which))

    def modify_condition(self, rnd):
        b, data = self.b, self.data
        creator = rnd_pc(b, data, rnd) if rnd.random() < 0.5 else rnd_foe(b, data, rnd)
        tgt = rnd_pc(b, data, rnd) if rnd.random() < 0.5 else rnd_foe(b, data, rnd)
        tgt.cond[0] = rnd.choice((0,) * 10 + (1,))
        tgt.condition_num = rnd.randrange(-1, 40)
        sid = rnd.choice(list(b.COND_SKILLS) + [rnd.randrange(0, len(data.skills))])
        game = self.game.modify_condition(creator, tgt, sid)
        t2 = copy_fighter(tgt)
        ev = b.modify_condition(creator, t2, sid, data)
        py = self.game.py_char(t2, tgt.type & b.PC_TYPES)
        py["calls"] = ev
        return diff(py, game, ("modify_condition", sid))

    def condition_success(self, rnd):
        b, data = self.b, self.data
        tgt = rnd_pc(b, data, rnd) if rnd.random() < 0.5 else rnd_foe(b, data, rnd)
        tgt.real[14] = rnd.choice((rnd.randrange(0, 1100), rnd_short(rnd)))
        tgt.real[15] = rnd.choice((rnd.randrange(0, 1100), rnd_short(rnd)))
        sid = rnd.randrange(150, 180)
        state = rnd.getrandbits(64)
        game = self.game.condition_success(tgt, sid, state)
        rng = b.Rand(state)
        py = {"ret": b.condition_success(tgt, sid, rng), "rand": rng.state}
        return diff(py, game, ("condition_success", sid))

    def exp(self, rnd):
        b, data = self.b, self.data
        lv = rnd.choice((rnd.randrange(0, 100), rnd_short(rnd)))
        n = self.game.members          # every character, 18 in Infection
        ids = rnd.sample(range(n), rnd.randrange(0, 4))
        party = []
        for i in ids:
            pc = rnd_pc(b, data, rnd)
            pc.cond[0] = rnd.choice((0, 0, 0, 1))
            party.append((i, pc))
        members = {}
        for i in range(n):
            if i not in ids:
                pc = rnd_pc(b, data, rnd)
                members[i] = pc
        flags, exp_flags = rnd.getrandbits(n), rnd.getrandbits(n)
        count = rnd.getrandbits(32)
        erosion = rnd.choice((0, 1, rnd.randrange(0, 101), rnd_short(rnd)))
        self.game.erosion = erosion
        got, er = self.game.exp_distributor(lv, party, members, flags, exp_flags, count)
        want = {}
        for i, pc in party:
            gain = b.exp_for_kill(data, lv, pc.level) if pc.cond[0] == 0 else 0
            want[i] = b.s16(pc.exp + gain)
        for i, pc in members.items():
            on = flags >> i & 1 and exp_flags >> i & 1
            want[i] = b.s16(pc.exp + (b.exp_for_kill(data, lv, pc.level, False) if on else 0))
        want_er = b.s16(erosion - (count % 3 + 2))
        want_er = max(want_er, 0)
        return diff({"exp": want, "erosion": want_er}, {"exp": got, "erosion": er}, ("exp", lv))

    def erosion(self, rnd):
        b = self.b
        e = rnd.choice((rnd.randrange(0, 101), rnd_short(rnd)))
        n = rnd.choice((rnd.randrange(-8, 14), rnd_short(rnd)))
        f = rnd.choice(list(b.DRAIN_FACTOR.values()) + [self.game.eemu.f_from_py(rnd.uniform(0, 5))])
        game = self.game.add_lv_erosion(e, n, f)
        return diff(b.add_lv_erosion(self.data, e, n, f), game, ("erosion", e, n))


def diff(py, game, what):
    if py == game:
        return None
    if isinstance(py, dict):
        for k in py:
            if py[k] != game.get(k):
                return f"{what}: {k}: battle.py {py[k]!r} game {game.get(k)!r}"
    return f"{what}: battle.py {py!r} game {game!r}"


CHECKS = ("damage", "protect", "skill_damage", "skill_damage_value", "recovery", "cure", "calc_real",
          "level", "modify_condition", "condition_success", "exp", "erosion", "exdefense")


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
class TestBattle(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import battle
        cls.b = battle
        cls.data = battle.Data(ELF)
        cls.check = Checker(cls.data)

    def run_check(self, name, n, seed):
        rnd = random.Random(seed)
        fn = getattr(self.check, name)
        for _ in range(n):
            self.assertIsNone(fn(rnd))

    def test_damage_matches_game(self):
        self.run_check("damage", 600, 1)

    def test_protect_matches_game(self):
        self.run_check("protect", 300, 12)

    def test_skill_damage_matches_game(self):
        self.run_check("skill_damage", 200, 8)

    def test_ai_estimate_matches_game(self):
        self.run_check("skill_damage_value", 200, 11)

    def test_heal_and_cure_match_game(self):
        self.run_check("recovery", 100, 9)
        self.run_check("cure", 200, 10)

    def test_calc_real_matches_game(self):
        self.run_check("calc_real", 300, 2)

    def test_levels_match_game(self):
        self.run_check("level", 300, 3)

    def test_conditions_match_game(self):
        self.run_check("modify_condition", 300, 4)
        self.run_check("condition_success", 200, 5)

    def test_exp_matches_game(self):
        self.run_check("exp", 100, 6)
        self.run_check("erosion", 200, 7)

    def test_kite_hits_goblin(self):
        """Kite at level 1 with his starting equipment against a Goblin."""
        b = self.b
        kite = b.make_pc(self.data, "Kite")
        self.assertEqual(kite.real[:8], [35, 54, 43, 73, 24, 54, 36, 66])
        goblin = b.Foe(self.data.find(self.data.enemies, "Goblin", "enemy"), "enemy")
        sk = self.data.skill("ATTACK")
        dmg = [b.calc_battle_damage(kite, goblin, sk, roll=r).dmg for r in range(101)]
        self.assertEqual(sum(d >= 0 for d in dmg), 55)
        self.assertEqual((min(d for d in dmg if d >= 0), max(dmg)), (8, 16))

    def test_level_table(self):
        b = self.b
        pc = b.make_pc(self.data, "Kite", level=10)
        self.assertEqual((pc.level, pc.maxHP, pc.maxSP), (10, 225, 40))
        self.assertEqual(pc.elm[:4], [60, 50, 150, 150])
        self.assertEqual(b.exp_for_kill(self.data, 5, 5), 60)

    def test_exdefense_matches_game(self):
        self.run_check("exdefense", 200, 13)

    def test_tables_found_from_code(self):
        """Tables located and sized from the code give the DWARF answer."""
        from image import Program
        d = self.data
        self.assertIsNone(d.exdef_cancel)
        sizes = {"skills": ("skillTbl", 0x38), "boss_skills": ("BossSkillTbl", 0x38),
                 "enemies": ("enemyTbl", 0x1C0), "bosses": ("bossTbl", 0x68)}
        sizes.update({s: (f"equipment{s.title()}Tbl", 0x48)
                      for s in ("head", "body", "arm", "leg")})
        sizes.update({f"weapon{j}": (f"equipmentWeapon{j}Tbl", 0x48) for j in range(1, 7)})
        sizes.update({f"item{c}": (f"itemTbl{n}", 20) for c, n in enumerate("RDUXTE")})
        for k, (name, row) in sizes.items():
            s = d.p.symbol_named(name)
            self.assertEqual(d.rows[k], (s.value, s.size // row), name)
        charTbl = Program(ELF, "demo").symbol_named("charTbl")
        self.assertEqual(len(d.chars), charTbl.size // 0x5C)
        self.assertEqual(d.erosion_va, inf_va(0x003071A0))


class TestOtherVolumes(unittest.TestCase):
    """Mutation, Outbreak and Quarantine, each against its own gcmn.prg:
    every check, a modest number of cases. From Mutation on the Exdefense
    rule differs (battle.exdefense_cancel); there are 21 characters
    (charTbl, LevelUpParamTbl, ccExpDistributor, and a second save block
    for ids 18-20); ccSkill's creator and target moved by 0x10; and
    equipmentWeapon1/4/6Tbl and BossSkillTbl have more rows."""

    CASES = 100

    def test_volumes(self):
        import battle
        present = [p for p in OTHERS if os.path.exists(p) and os.path.exists(p + ".syms")]
        if not present:
            self.skipTest("no other volume extracted with a .syms sidecar")
        for path in present:
            with self.subTest(path=path):
                data = battle.Data(path)
                self.assertEqual(data.exdef_cancel, 0x100)
                self.assertEqual((len(data.chars), len(data.levelup)), (21, 21))
                self.assertEqual([len(w) for w in data.weapons], [83, 77, 97, 76, 74, 77])
                self.assertEqual((len(data.skills), len(data.enemies), len(data.bosses),
                                  len(data.boss_skills)), (304, 303, 49, 79))
                check = Checker(data)
                g = check.game
                self.assertEqual((g.sk_creator, g.sk_target, g.members), (0x80, 0x84, 21))
                for name in CHECKS:
                    rnd = random.Random(2000 + CHECKS.index(name))
                    for _ in range(self.CASES):
                        d = getattr(check, name)(rnd)
                        self.assertIsNone(d, path)


def bulk(n, elf=ELF, seed=1000):
    """Every check n times with fresh seeds, against any volume's
    executable; prints counts per check."""
    import battle
    data = battle.Data(elf)
    ch = Checker(data)
    for name in CHECKS:
        rnd = random.Random(seed + CHECKS.index(name))
        bad = 0
        first = None
        for _ in range(n):
            d = getattr(ch, name)(rnd)
            if d:
                bad += 1
                first = first or d
        print(f"{name:18} {n:6} cases, {bad} mismatches" + (f"; first: {first}" if first else ""))
    print("damage paths: " + ", ".join(f"{k} {v}" for k, v in sorted(ch.coverage.items())))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]) if len(sys.argv) > 2 else 500,
             sys.argv[3] if len(sys.argv) > 3 else ELF)
    else:
        unittest.main()
