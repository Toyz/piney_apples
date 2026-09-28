#!/usr/bin/env python3
"""crates/piney-battle against the game's own battle code run in tools/eemu.py.

Every check builds a random case, runs the game's function on it natively
in the interpreter (gcmn.prg loaded over the executable, the real tables in
memory, the effect, particle and menu functions stubbed to record their
calls), sends the same case to the port's battle_probe example, and compares
everything: the return value, every field of every character touched, the
newlib rand() state, and the calls in order. The case generators are
tools/test_battle.py's (random party members, enemies, bosses and objects,
skills, environments); the game side reuses its GameBattle, and adds a scene
of several characters on the command lists for the area rules.

    python3 tools/test_battle_rs.py                 the unit tests (a sample)
    python3 tools/test_battle_rs.py bulk N          N cases of every check

Checks:
  damage, protect, exdefense   ccChar::CalcBattleDamage (0x0056d910)
  skill_damage                 ccSkillDamage, one target (0x00573e60)
  area_damage                  ccSkillDamage over an area, ccSkillDamage
                               at a point (0x005743c0), ccSkillDamage2
                               (0x005746d0)
  value                        ccSkillDamageValue (0x00594e90)
  recovery, area_recovery      ccSkillRecovery (0x00574bc0, 0x00574f70)
  cure                         ccSkill::RecoverySystem (0x00577070)
  hold                         ccSkillHold (0x005752b0, 0x00575520)
  modify                       ccSkillModifyCondition (0x005756f0, 0x00575a50)
  request                      _ccSkillRequest (0x00572860)
  calc_real                    ccChar::CalcReal (0x0056ba30)
  disp_condition               ccChar::DispConditionEffect (0x0056f950)
  level                        CheckLevelUp, LevelDown, ccSetLevelParam
  modify_condition             _ccSkillModifyCondition (0x00575cf0)
  condition_success            ccCheckConditionSkillSuccess (0x00571830)
  target_condition             ccCheckTargetConditionBySkill (0x00579750)
  exp, erosion                 ccExpDistributor (0x00571290), AddLvErosion
  affect                       ccChar::EntryAffect (0x0056b020) with the
                               character's affect: ccEnemyInfluence,
                               ccFellow::Influence, Influence (Kite)

The skill's life and the normal attack are checked by
tools/test_battle_flow_rs.py, Data Drain by tools/test_battle_drain_rs.py,
the enemies' and the party's AI by tools/test_battle_enemy_ai_rs.py and
tools/test_battle_party_ai_rs.py, and item use by
tools/test_battle_items_rs.py, all on this module's scene and runner
(Against, main).

Skipped when the disc is not extracted or cargo is missing. CARGO_TARGET_DIR
is honoured when locating the probe.
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
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
TARGET = os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target")
EXAMPLE = os.path.join(TARGET, "release", "examples", "battle_probe")

# A scene of characters for the area rules: ccChar at SCN + 0x1000 * i, its
# ccSpcParam / ccEnemyParam at +0x400, an enemy's base at +0x800.
SCN = 0x01100000
NEW = 0x01200000            # operator new's bump region
F_ONE = 0x3F800000


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-battle", "--example",
                    "battle_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, cwd=ROOT)
    out = [json.loads(line) for line in p.stdout.splitlines()]
    if p.returncode:
        raise RuntimeError(f"battle_probe failed on request {len(out)}: {lines[len(out)][:300]}\n{p.stderr[-2000:]}")
    return out


def norm(v):
    """Tuples to lists, as JSON has them."""
    return json.loads(json.dumps(v))


# serialising cases for the probe ------------------------------------------------------

def ser_char(ch, b):
    if isinstance(ch, b.PC):
        head = (["P", ch.type, ch.id, ch.level, ch.exp, ch.p_maxHP, ch.p_maxSP] + list(ch.elm) + list(ch.real)
                + list(ch.tune) + list(ch.temp) + list(ch.time) + list(ch.equip) + [ch.job])
    elif hasattr(ch, "row"):
        r = ch.row
        head = (["B" if ch.type & 0x80 else "E", ch.type, ch.id, ch.level, getattr(ch, "exp", 0), r["maxHP"],
                 r["maxSP"]] + list(r["elm"]) + list(r["beff"]) + [r["maxPP"], r["pDefPP"], r["mDefPP"],
                                                                     r["Exdefense"]] + list(r["item"])
                + list(ch.real) + list(ch.temp) + list(ch.time) + [ch.PP, ch.PPcount, getattr(ch, "PPrestore", 0)])
    else:
        head = ["O", ch.type] + list(ch.real)
    tail = ([ch.HP, ch.SP, ch.maxHP, ch.maxSP] + list(ch.cond)
            + [ch.speed_value, getattr(ch, "condition_num", 0), int(bool(getattr(ch, "no_death", False))),
               ch.ent_root, getattr(ch, "party_flag", 0), int(bool(getattr(ch, "ai", 0))),
               getattr(ch, "anm_flag", 0), getattr(ch, "skill_id", 0), getattr(ch, "skill_status", 0)]
            + list(getattr(ch, "pos_p", [0, 0, 0, F_ONE])) + [getattr(ch, "width", 0)]
            + [getattr(ch, "affect_func", 0), getattr(ch, "affect_type", 0), getattr(ch, "affect_mask", 0),
               getattr(ch, "act_num", 0), getattr(ch, "spc_flags", 0), getattr(ch, "fellow_flags", 0),
               getattr(ch, "enemy_flags", 0), getattr(ch, "sys_msg_id", -1), getattr(ch, "arms_effect_sw", 0),
               getattr(ch, "attack", 0), getattr(ch, "cnt", 0), getattr(ch, "cloak", 0),
               getattr(ch, "target_char", -1)]
            + list(getattr(ch, "pos", [0, 0, 0, F_ONE])))
    return " ".join(str(v) for v in head + tail)


def ser_skill(sk):
    v = ([sk["atk"], sk["hit"]] + list(sk["attr"]) + [sk["condition"], sk["cost"], sk["type"], sk["dmgRate"],
                                                     sk.get("triggerRange_bits", 0), sk.get("targetRange_bits", 0),
                                                     sk.get("targetType", 0)])
    return " ".join(str(x) for x in v)


def ser_env(env):
    return " ".join(str(x) for x in (env.plcol, env.menu_flag, env.in_battle, env.count, env.menu_type,
                                     env.sp_regene_speed, env.area))


def fbits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


# the game side ----------------------------------------------------------------------

class GameScene:
    """GameBattle plus a scene of up to eight characters on the command
    lists, with the VU0 vector routines the area rules call done in Python
    (the interpreter has no COP2) and the calls they make recorded."""

    def __init__(self, gb):
        import eemu
        self.gb = gb
        self.eemu = eemu
        m = gb.m
        sym = gb.sym

        def copy(mm, a, b_, *_):
            mm.mem[a:a + 16] = mm.mem[b_:b_ + 16]
            return 0

        def sub(mm, d, a, b_, *_):
            x = [mm.load(a + 4 * i, 4) for i in range(4)]
            y = [mm.load(b_ + 4 * i, 4) for i in range(4)]
            for i in range(4):
                mm.store(d + 4 * i, 4, eemu.f_sub(x[i], y[i]))
            return 0

        def inner(mm, a, b_, *_):
            x = [mm.load(a + 4 * i, 4) for i in range(3)]
            y = [mm.load(b_ + 4 * i, 4) for i in range(3)]
            v = eemu.f_add(eemu.f_add(eemu.f_mul(x[0], y[0]), eemu.f_mul(x[1], y[1])), eemu.f_mul(x[2], y[2]))
            mm.f[0] = v
            return v

        self.new_at = NEW

        def new(mm, n, *_):
            a = self.new_at
            self.new_at += (n + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            return a

        rec = gb.record
        m.hooks[sym("sceVu0CopyVector")] = copy
        m.hooks[sym("sceVu0SubVector")] = sub
        m.hooks[sym("sceVu0InnerProduct")] = inner
        m.hooks[sym("ccTransPosW2P__FPfPf")] = copy
        m.hooks[sym("effSkillStart__FP6ccChariii")] = lambda mm, a, b_, c, d: rec("effSkillStart", a, b_, c, d)
        m.hooks[sym("ccEntryFlyFontNew__FiiPfP6ccCharff")] = \
            lambda mm, a, b_, c, d: rec("ccEntryFlyFontNew", a, b_, d)
        m.hooks[sym("ccWordsPlay__FiP6ccChar")] = lambda mm, a, b_, *_: rec("ccWordsPlay", a, b_)
        m.hooks[sym("__nw__FUi")] = new

        def skill_ctor(mm, this, sid, *_):
            # ccSkill::ccSkill(int) (0x00572f80), whose sqc2 of vf0 the
            # interpreter cannot run: cleared, the id, 200.0 at +0x50, vf0
            # at +0x30 and +0x60, and linked onto SkillEntryTop/Tail.
            mm.mem[this:this + 0xB0] = bytes(0xB0)
            mm.store(this, 4, sid)
            mm.store(this + 0x3C, 4, F_ONE)
            mm.store(this + 0x6C, 4, F_ONE)
            mm.store(this + 0x50, 4, 0x43480000)
            if mm.load(self.skill_top, 4) == 0:
                mm.store(self.skill_top, 4, this)
            tail = mm.load(self.skill_tail, 4)
            if tail:
                mm.store(tail + 4, 4, this)
            mm.store(self.skill_tail, 4, this)
            mm.store(self.skill_num, 4, mm.load(self.skill_num, 4) + 1)
            return this
        m.hooks[sym("__ct__7ccSkillFi")] = skill_ctor
        self.xnote = sym("xnote_ccs")
        self.skill_top = sym("SkillEntryTop")
        self.skill_tail = sym("SkillEntryTail")
        self.skill_num = sym("SkillEntryNum")

    def va(self, i):
        return SCN + 0x1000 * i

    def put(self, chars, pcs, enes):
        gb = self.gb
        m = gb.m
        m.mem[SCN:SCN + 0x1000 * 8] = bytes(0x1000 * 8)
        gb.names = {self.va(i): f"c{i}" for i in range(len(chars))}
        gb.names[0x01031000] = "ai"
        self.rows = []
        for i, ch in enumerate(chars):
            va = self.va(i)
            gb.put_char(ch, va, va + 0x400, va + 0x800)
            if hasattr(ch, "row"):
                gb.put_row(ch)
                self.rows.append(ch.row["va"])
            base = m.load(va, 4)
            for k, v in enumerate(getattr(ch, "pos_p", [0, 0, 0, F_ONE])):
                m.store(va + 0x50 + 4 * k, 4, v)
            for k, v in enumerate(getattr(ch, "pos", [0, 0, 0, F_ONE])):
                m.store(va + 0x40 + 4 * k, 4, v)
            m.store(base + 0x1C, 4, getattr(ch, "width", 0))
            gb.put(va + 0x7C, "<hh", getattr(ch, "skill_id", 0), getattr(ch, "skill_status", 0))
            gb.put(va + 0xF4, "<h", getattr(ch, "anm_flag", 0))
        for root, lst in ((gb.cmnd_pc, pcs), (gb.cmnd_ene, enes)):
            m.store(root, 4, self.va(lst[0]) if lst else 0)
            for a, b_ in zip(lst, lst[1:] + [None]):
                m.store(self.va(a) + 0xBC, 4, self.va(b_) if b_ is not None else 0)
        m.store(gb.cmnd_obj, 4, 0)

    def read(self, chars):
        gb = self.gb
        return [gb.read_char(self.va(i), self.va(i) + 0x400, bool(ch.type & 7)) for i, ch in enumerate(chars)]

    def restore(self):
        for va in self.rows:
            self.gb.restore(va, 0x68)


# random cases ------------------------------------------------------------------------

def rnd_pos(rnd):
    """A posP near the origin: within a few units, sometimes exactly on it."""
    if rnd.random() < 0.1:
        return [0, 0, 0, F_ONE]
    return [fbits(rnd.uniform(-60, 60)), fbits(rnd.uniform(-60, 60)), fbits(rnd.uniform(-20, 20)), F_ONE]


class Checks:
    """Each check returns (probe request, the game's answer, a label); the
    batch runner compares."""

    def __init__(self):
        import battle
        import test_battle as tb
        self.b = battle
        self.tb = tb
        self.data = battle.Data(ELF)
        self.game = tb.GameBattle(self.data)
        self.scene = GameScene(self.game)

    def reset(self):
        """Empty command lists and no scene left over from the last case
        (GameBattle's single-character calls set only the lists they use)."""
        g = self.game
        for root in (g.cmnd_pc, g.cmnd_ene, g.cmnd_obj):
            g.m.store(root, 4, 0)
        g.m.mem[SCN:SCN + 0x8000] = bytes(0x8000)

    # CalcBattleDamage ----------------------------------------------------------------
    def damage(self, rnd, protect=False, exdef=False):
        b, tb, data = self.b, self.tb, self.data
        pick = rnd.random()
        att = (tb.rnd_pc(b, data, rnd) if pick < 0.45 else tb.rnd_foe(b, data, rnd) if pick < 0.9
               else tb.rnd_other(b, rnd))
        pick = 0 if protect or exdef else rnd.random()
        tgt = (tb.rnd_foe(b, data, rnd) if pick < 0.6 else tb.rnd_pc(b, data, rnd) if pick < 0.97
               else tb.rnd_other(b, rnd))
        tgt.cond[0] = rnd.choice((0,) * 20 + (1,))
        sk = tb.rnd_skill(data, rnd)
        mag = rnd.choice((0x3F800000, 0x3F800000, 0x40000000, 0x3F000000, fbits(rnd.uniform(0, 3))))
        h = rnd.choice((-1, -1, -1, -5, 0, rnd.randrange(1, 130)))
        env = tb.rnd_env(b, rnd)
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
        req = (f"damage {ser_char(att, b)} {ser_char(tgt, b)} {ser_skill(sk)} {mag} {h} {ser_env(env)} "
               f"{state} {int(listed)}")
        return req, game, "damage"

    def protect(self, rnd):
        return self.damage(rnd, protect=True)

    def exdefense(self, rnd):
        return self.damage(rnd, exdef=True)

    def skill_damage(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        att = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.6 else tb.rnd_foe(b, data, rnd)
        att.ai = rnd.choice((0, 1))
        att.party_flag = rnd.choice((0, 1, 3))
        tgt = tb.rnd_foe(b, data, rnd) if rnd.random() < 0.7 else tb.rnd_pc(b, data, rnd)
        tgt.cond[0] = rnd.choice((0,) * 20 + (1,))
        sk = tb.rnd_skill(data, rnd)
        sid = rnd.randrange(0, len(data.skills))
        ac = rnd.choice((0, 0, 1, 1, -1))
        env = tb.rnd_env(b, rnd)
        state = rnd.getrandbits(64)
        game = self.game.skill_damage(att, tgt, sk, sid, ac, env, state)
        req = (f"skilldmg {ser_char(att, b)} {ser_char(tgt, b)} {ser_skill(sk)} {sid} {ac} {ser_env(env)} "
               f"{state}")
        return req, game, "skill_damage"

    def value(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        cp = tb.rnd_pc(b, data, rnd)
        cp.id = rnd.randrange(len(data.chars))
        tp = tb.rnd_foe(b, data, rnd) if rnd.random() < 0.8 else tb.rnd_pc(b, data, rnd)
        tp.cond[0] = rnd.choice((0,) * 20 + (1,))
        if rnd.random() < 0.3:
            tp_elm = tp.row["elm"] if hasattr(tp, "row") else tp.elm
            tp_elm[8:14] = [rnd.choice((10, 20, 30)) for _ in range(6)]
        sid = rnd.randrange(len(data.skills))
        ty = -1
        if rnd.random() < 0.3:
            ty = 1 | rnd.randrange(8) << 10
            self.game.put(data.skills[sid]["va"] + 0x2C, "<i", ty)
        game = list(self.game.skill_damage_value(cp, tp, sid))
        self.game.restore(data.skills[sid]["va"], 0x38)
        return f"value {ser_char(cp, b)} {ser_char(tp, b)} {sid} {ty}", game, "value"

    def recovery(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        creator = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.7 else tb.rnd_foe(b, data, rnd)
        creator.cond[0] = rnd.choice((0, 0, 0, 1))
        tgt = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.7 else tb.rnd_foe(b, data, rnd)
        sid = rnd.choice((150, 151, 152, 153, 154, 155, 295))
        param = rnd.choice((800, tb.rnd_short(rnd)))
        game = self.game.recovery(creator, tgt, sid, param)
        return f"recovery {ser_char(creator, b)} {ser_char(tgt, b)} {sid} {param}", game, "recovery"

    def cure(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        creator = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.7 else tb.rnd_foe(b, data, rnd)
        tgt = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.7 else tb.rnd_foe(b, data, rnd)
        tgt.cond[0] = rnd.choice((0, 0, 0, 1))
        tgt.speed_value = rnd.choice((0x3F800000, 0x3F000000, 0x3FE00000, 0))
        tgt.temp = [rnd.choice((0, tb.rnd_short(rnd, -150, 150))) for _ in range(16)]
        sid = rnd.choice((178, 179, 180, 178, 179, 180, 175, 150))
        count = rnd.choice((0, 0, 0, 1))
        stype = rnd.choice((0, 0, 1, 2, 3))
        ann = rnd.choice((0, 0, 1))
        anm = rnd.choice((0, 1, 1))
        game = self.game.cure(creator, tgt, sid, count, stype, ann, anm)
        req = f"cure {ser_char(creator, b)} {ser_char(tgt, b)} {sid} {count} {stype} {ann} {anm}"
        return req, game, "cure"

    def calc_real(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        ch = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.6 else tb.rnd_foe(b, data, rnd)
        flag = rnd.choice((1, 0, 0, -1, 2))
        env = tb.rnd_env(b, rnd)
        game = self.game.calc_real(ch, flag, env)
        return f"calcreal {ser_char(ch, b)} {flag} {ser_env(env)}", game, "calc_real"

    def disp_condition(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        ch = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.5 else tb.rnd_foe(b, data, rnd)
        # A few conditions at a time, often none, now and then the dead.
        ch.cond = [0] * 16
        for k in rnd.sample((7, 8, 9, 10, 11, 12, 13, 14, 15), rnd.choice((0, 0, 1, 1, 2, 3))):
            ch.cond[k] = tb.rnd_short(rnd, 1, 900)
        ch.cond[0] = rnd.choice((0, 0, 0, 0, 1, 2, 4))
        ch.speed_value = rnd.choice((0x3F800000, 0x3F000000, 0x40000000))
        ch.time = [rnd.choice((0, 0, 0, 0, tb.rnd_short(rnd, 1, 200))) for _ in range(16)]
        ch.temp = [rnd.choice((0, tb.rnd_short(rnd, -50, 50))) for _ in range(16)]
        if rnd.random() < 0.3:
            ch.id = 0
        ch.condition_num = rnd.choice((-1, -1, 0, 1, 5, 6, 20, 23, 7, 22, 2))
        ep = rnd.choice((None, None, ch.condition_num, rnd.randrange(-1, 35)))
        shown, eye = rnd.choice((1, 1, 0)), rnd.choice((0, 0, 1))
        game = self.game.disp_condition(ch, ep, shown, eye)
        req = f"dispcond {ser_char(ch, b)} {-2 if ep is None else ep} {shown} {eye}"
        return req, game, "disp_condition"

    def level(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        pc = tb.rnd_pc(b, data, rnd)
        pc.elm = [rnd.randrange(-20, 1000) for _ in range(16)]
        which = rnd.choice(("up", "down", "set"))
        if which == "up":
            game = self.game.run_char_fn("CheckLevelUp__6ccCharFv", pc)
            game["ret"] = b.s32(game["ret"])
            return f"levelup {ser_char(pc, b)}", game, "levelup"
        if which == "down":
            game = self.game.run_char_fn("LevelDown__6ccCharFv", pc)
            game["ret"] = 0
            return f"leveldown {ser_char(pc, b)}", game, "leveldown"
        lvs = rnd.choice((rnd.randrange(1, 100), rnd.randrange(-3, 130)))
        game = self.game.set_level_param(pc, lvs)
        return f"setlevel {ser_char(pc, b)} {lvs}", game, "setlevel"

    def modify_condition(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        creator = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.5 else tb.rnd_foe(b, data, rnd)
        tgt = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.5 else tb.rnd_foe(b, data, rnd)
        tgt.cond[0] = rnd.choice((0,) * 10 + (1,))
        tgt.condition_num = rnd.randrange(-1, 40)
        sid = rnd.choice(list(b.COND_SKILLS) + [rnd.randrange(0, len(data.skills))])
        game = self.game.modify_condition(creator, tgt, sid)
        return f"modcond {ser_char(creator, b)} {ser_char(tgt, b)} {sid}", game, "modify_condition"

    def condition_success(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        tgt = tb.rnd_pc(b, data, rnd) if rnd.random() < 0.5 else tb.rnd_foe(b, data, rnd)
        tgt.real[14] = rnd.choice((rnd.randrange(0, 1100), tb.rnd_short(rnd)))
        tgt.real[15] = rnd.choice((rnd.randrange(0, 1100), tb.rnd_short(rnd)))
        sid = rnd.randrange(150, 180)
        state = rnd.getrandbits(64)
        game = self.game.condition_success(tgt, sid, state)
        return f"condsucc {ser_char(tgt, b)} {sid} {state}", game, "condition_success"

    def target_condition(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        pick = rnd.random()
        ch = tb.rnd_pc(b, data, rnd) if pick < 0.45 else tb.rnd_foe(b, data, rnd) if pick < 0.9 \
            else tb.rnd_other(b, rnd)
        ch.speed_value = rnd.choice((F_ONE, 0x3F000000, 0x3FE00000, 0, 0x80000000, 0x3F800001))
        ch.temp = [rnd.choice((0, tb.rnd_short(rnd, -150, 150))) for _ in range(16)]
        sid = rnd.choice((rnd.randrange(150, 200), rnd.randrange(0, 304), -3))
        g = self.game
        g.clear()
        g.put_char(ch, 0x01000000, 0x01002000, 0x01004000)
        ret = self.b.s32(g.m.call(g.sym("ccCheckTargetConditionBySkill__FP6ccChari"), (0x01000000, sid & 0xFFFFFFFF)))
        return f"targetcond {ser_char(ch, b)} {sid}", ret, "target_condition"

    def exp(self, rnd):
        b, tb, data = self.b, self.tb, self.data
        lv = rnd.choice((rnd.randrange(0, 100), tb.rnd_short(rnd)))
        n = self.game.members
        ids = rnd.sample(range(n), rnd.randrange(0, 4))
        party = []
        for i in ids:
            pc = tb.rnd_pc(b, data, rnd)
            pc.cond[0] = rnd.choice((0, 0, 0, 1))
            party.append((i, pc))
        members = {i: tb.rnd_pc(b, data, rnd) for i in range(n) if i not in ids}
        flags, exp_flags = rnd.getrandbits(n), rnd.getrandbits(n)
        count = rnd.getrandbits(32)
        erosion = rnd.choice((0, 1, rnd.randrange(0, 101), tb.rnd_short(rnd)))
        self.game.erosion = erosion
        got, er = self.game.exp_distributor(lv, party, members, flags, exp_flags, count)
        req = (f"exp {lv} {erosion} {count} {flags} {exp_flags} {len(party)} "
               + " ".join(f"{i} {ser_char(pc, b)}" for i, pc in party)
               + f" {len(members)} " + " ".join(f"{i} {ser_char(pc, b)}" for i, pc in members.items()))
        return req, {"exp": {str(k): v for k, v in got.items()}, "erosion": er}, "exp"

    def erosion(self, rnd):
        tb = self.tb
        e = rnd.choice((rnd.randrange(0, 101), tb.rnd_short(rnd)))
        n = rnd.choice((rnd.randrange(-8, 14), tb.rnd_short(rnd)))
        f = rnd.choice(list(self.b.DRAIN_FACTOR.values()) + [fbits(rnd.uniform(0, 5))])
        return f"erosion {e} {n} {f}", self.game.add_lv_erosion(e, n, f), "erosion"

    # scenes ----------------------------------------------------------------------------
    def rnd_scene(self, rnd):
        """Two to seven characters: party members and foes (distinct table
        rows), positions near each other, on the lists by side (a few left
        off)."""
        b, tb, data = self.b, self.tb, self.data
        chars = []
        used = set()
        for _ in range(rnd.randrange(2, 8)):
            if rnd.random() < 0.45:
                ch = tb.rnd_pc(b, data, rnd)
                ch.type = rnd.choice((7, 6, 6, 4, 2))
            else:
                ch = tb.rnd_foe(b, data, rnd)
                while (ch.kind, ch.id) in used:
                    ch = tb.rnd_foe(b, data, rnd)
                used.add((ch.kind, ch.id))
            ch.cond[0] = rnd.choice((0,) * 8 + (1,))
            ch.pos_p = rnd_pos(rnd)
            ch.width = rnd.choice((0, fbits(rnd.uniform(0, 8))))
            ch.ai = rnd.choice((0, 1))
            ch.party_flag = rnd.choice((0, 1))
            ch.skill_id, ch.skill_status = rnd.randrange(0, 300), rnd.randrange(0, 10)
            ch.anm_flag = rnd.choice((0, 1))
            chars.append(ch)
        pcs = [i for i, c in enumerate(chars) if c.type & 7 and rnd.random() > 0.05]
        enes = [i for i, c in enumerate(chars) if not c.type & 7 and rnd.random() > 0.05]
        return chars, pcs, enes

    def ser_scene(self, chars, pcs, enes):
        b = self.b
        return (f"scene {len(chars)} " + " ".join(ser_char(c, b) for c in chars)
                + f" {len(pcs)} " + " ".join(map(str, pcs)) + f" {len(enes)} " + " ".join(map(str, enes)))

    def rnd_area_skill(self, rnd):
        sk = self.tb.rnd_skill(self.data, rnd)
        sk["type"] |= rnd.choice((0, 0x2000, 0x8000, 0xA000))
        rng_ = rnd.choice((0.0, -1.0, 20.0, 40.0, rnd.uniform(0, 80)))
        sk["targetRange_bits"] = fbits(rng_)
        return sk

    def run_scene(self, chars, pcs, enes, fn, args, flt=None):
        g = self.game
        self.scene.put(chars, pcs, enes)
        g.calls = []
        if flt is not None:
            g.m.f[12] = flt
        ret = g.m.call(g.sym(fn), args)
        out = {"ret": self.b.s32(ret), "rand": g.rand_now(), "calls": list(g.calls),
               "chars": self.scene.read(chars)}
        self.scene.restore()
        return out

    def put_skill(self, sk, va):
        g = self.game
        g.put(va + 8, "<hh", sk["atk"], sk["hit"])
        g.put_shorts(va + 12, sk["attr"])
        g.put(va + 0x18, "<i", sk["condition"])
        g.put(va + 0x1C, "<II", sk.get("triggerRange_bits", 0), sk.get("targetRange_bits", 0))
        g.put(va + 0x24, "<iii", sk.get("targetType", 0), sk["cost"], sk["type"])
        g.put(va + 0x32, "<h", sk["dmgRate"])

    def area_damage(self, rnd):
        b, tb = self.b, self.tb
        chars, pcs, enes = self.rnd_scene(rnd)
        g = self.game
        sk = self.rnd_area_skill(rnd)
        sid = rnd.randrange(0, 304)
        env = tb.rnd_env(b, rnd)
        g.put_env(env)
        state = rnd.getrandbits(64)
        g.set_rand(state)
        me = rnd.randrange(len(chars))
        tgt = rnd.randrange(len(chars))
        skva = 0x01030000
        self.put_skill(sk, skva)
        which = rnd.choice(("skilldmg", "skilldmgat", "skilldmg2"))
        pos = rnd_pos(rnd)
        ttype = rnd.choice((0x6, 0x7, 0x60, 0x20, 0x80, 0xE0, 0x1, 0))
        ac = rnd.choice((0, 0, 1, -1))
        va = self.scene.va
        g.put(0x01032100, "<h", ac)
        g.m.mem[0x01032200:0x01032210] = struct.pack("<4I", *pos)
        if which == "skilldmg":
            out = self.run_scene(chars, pcs, enes, "ccSkillDamage__FP6ccCharP6ccCharP12ccSkillParamRsi",
                                 (va(me), va(tgt), skva, 0x01032100, sid))
            out["ac"] = g.shorts(0x01032100, 1)[0]
            req = (f"{self.ser_scene(chars, pcs, enes)} skilldmg {me} {tgt} {ser_skill(sk)} {sid} {ac} "
                   f"{ser_env(env)} {state}")
        elif which == "skilldmgat":
            out = self.run_scene(chars, pcs, enes, "ccSkillDamage__FP6ccCharPfiP12ccSkillParami",
                                 (va(me), 0x01032200, ttype, skva, sid))
            req = (f"{self.ser_scene(chars, pcs, enes)} skilldmgat {me} {' '.join(map(str, pos))} {ttype} "
                   f"{ser_skill(sk)} {sid} {ser_env(env)} {state}")
        else:
            g.m.mem[0x01032300:0x01032310] = struct.pack("<iii", sid, 0, 0) + bytes(4)
            out = self.run_scene(chars, pcs, enes, "ccSkillDamage2__FP6ccCharP6ccCharPfiP12ccSkillParamRsi",
                                 (va(me), va(tgt), 0x01032200, ttype, skva, 0x01032100, sid))
            out["ac"] = g.shorts(0x01032100, 1)[0]
            req = (f"{self.ser_scene(chars, pcs, enes)} skilldmg2 {me} {tgt} {' '.join(map(str, pos))} "
                   f"{ttype} {ser_skill(sk)} {sid} {ac} {ser_env(env)} {state}")
        return req, out, which

    def patched(self, rnd, sid):
        """Skill row `sid` with a random area and type bits, in the game's
        table; returns the probe's patch line."""
        sk = dict(self.data.skills[sid])
        sk["attr"] = list(sk["attr"])
        sk["type"] = sk["type"] ^ rnd.choice((0, 0, 0x2000, 0x8000))
        sk["targetRange_bits"] = fbits(rnd.choice((0.0, 0.0, 20.0, 40.0, rnd.uniform(0, 80))))
        sk["triggerRange_bits"] = fbits(sk["triggerRange"])
        self.put_skill(sk, sk["va"])
        return f"patch {sid} {ser_skill(sk)}"

    def area_recovery(self, rnd):
        chars, pcs, enes = self.rnd_scene(rnd)
        g = self.game
        sid = rnd.choice((150, 151, 152, 153, 154, 155, 295))
        patch = self.patched(rnd, sid)
        param = rnd.choice((800, self.tb.rnd_short(rnd)))
        me = rnd.randrange(len(chars))
        tgt = rnd.randrange(len(chars))
        va = self.scene.va
        pos = rnd_pos(rnd)
        g.m.mem[0x01032200:0x01032210] = struct.pack("<4I", *pos)
        if rnd.random() < 0.5:
            out = self.run_scene(chars, pcs, enes, "ccSkillRecovery__FP6ccCharP6ccCharii",
                                 (va(me), va(tgt), sid, param & 0xFFFFFFFF))
            req = f"{self.ser_scene(chars, pcs, enes)} recovery {me} {tgt} {sid} {param}"
        else:
            out = self.run_scene(chars, pcs, enes, "ccSkillRecovery__FP6ccCharPfii",
                                 (va(me), 0x01032200, sid, param & 0xFFFFFFFF))
            req = f"{self.ser_scene(chars, pcs, enes)} recoveryat {me} {' '.join(map(str, pos))} {sid} {param}"
        g.restore(self.data.skills[sid]["va"], 0x38)
        out.pop("rand")
        return [patch, req, f"restore {sid}"], out, "area_recovery"

    def hold(self, rnd):
        chars, pcs, enes = self.rnd_scene(rnd)
        g = self.game
        sk = self.rnd_area_skill(rnd)
        skva = 0x01030000
        self.put_skill(sk, skva)
        me = rnd.randrange(len(chars))
        tgt = rnd.randrange(len(chars))
        va = self.scene.va
        pos = rnd_pos(rnd)
        g.m.mem[0x01032200:0x01032210] = struct.pack("<4I", *pos)
        if rnd.random() < 0.5:
            out = self.run_scene(chars, pcs, enes, "ccSkillHold__FP6ccCharP6ccCharP12ccSkillParam",
                                 (va(me), va(tgt), skva))
            req = f"{self.ser_scene(chars, pcs, enes)} hold {me} {tgt} {ser_skill(sk)}"
        else:
            ttype = rnd.choice((0x6, 0x7, 0x60, 0x20, 0x80, 0xE0, 0))
            out = self.run_scene(chars, pcs, enes, "ccSkillHold__FP6ccCharPfiP12ccSkillParam",
                                 (va(me), 0x01032200, ttype, skva))
            req = f"{self.ser_scene(chars, pcs, enes)} holdat {me} {' '.join(map(str, pos))} {ttype} {ser_skill(sk)}"
        out.pop("rand")
        return req, out, "hold"

    def modify(self, rnd):
        chars, pcs, enes = self.rnd_scene(rnd)
        g = self.game
        for c in chars:
            c.real[14] = rnd.choice((0, rnd.randrange(0, 1100)))
            c.real[15] = rnd.choice((0, rnd.randrange(0, 1100)))
        sid = rnd.choice(list(self.b.COND_SKILLS) + [rnd.randrange(0, 304)])
        patch = self.patched(rnd, sid)
        me = rnd.randrange(len(chars))
        tgt = rnd.randrange(len(chars))
        stype = rnd.choice((0, 1, 2, 1))
        force = rnd.choice((0, 0, 0, 1))
        state = rnd.getrandbits(64)
        g.set_rand(state)
        va = self.scene.va
        pos = rnd_pos(rnd)
        g.m.mem[0x01032200:0x01032210] = struct.pack("<4I", *pos)
        if rnd.random() < 0.5:
            out = self.run_scene(chars, pcs, enes, "ccSkillModifyCondition__FP6ccCharP6ccChariii",
                                 (va(me), va(tgt), sid, stype, force))
            req = f"{self.ser_scene(chars, pcs, enes)} modify {me} {tgt} {sid} {stype} {force} {state}"
        else:
            ttype = rnd.choice((0x6, 0x7, 0x60, 0x20, 0x80, 0xE0, 0))
            out = self.run_scene(chars, pcs, enes, "ccSkillModifyCondition__FP6ccCharPfiiii",
                                 (va(me), 0x01032200, ttype, sid, stype, force))
            req = (f"{self.ser_scene(chars, pcs, enes)} modifyat {me} {' '.join(map(str, pos))} {ttype} {sid} "
                   f"{stype} {force} {state}")
        g.restore(self.data.skills[sid]["va"], 0x38)
        calls = out["calls"]
        idx = {f"c{i}": i for i in range(len(chars))}
        out["started"] = [idx[c[1]] for c in calls if c[0] == "effSkillStart"]
        out["resisted"] = [idx[c[3]] for c in calls if c[0] == "ccEntryFlyFontNew"]
        out["calls"] = [c for c in calls if c[0] not in ("effSkillStart", "ccEntryFlyFontNew")]
        return [patch, req, f"restore {sid}"], out, "modify"

    def request(self, rnd):
        chars, pcs, enes = self.rnd_scene(rnd)
        g = self.game
        m = g.m
        sid = rnd.choice((0, 1, 2, 5, 6, rnd.randrange(0, 304), rnd.randrange(150, 200), 178, 180))
        patch = self.patched(rnd, sid)
        me = rnd.randrange(len(chars))
        tgt = rnd.randrange(len(chars))
        stype = rnd.choice((0, 0, 1, 2))
        running = rnd.choice((0, 1))
        state = rnd.getrandbits(64)
        g.set_rand(state)
        va = self.scene.va
        self.scene.put(chars, pcs, enes)
        m.store(self.scene.xnote, 4, 0)
        m.store(self.scene.skill_top, 4, 0)
        m.store(self.scene.skill_tail, 4, 0)
        m.store(self.scene.skill_num, 4, 0)
        old = 0x01032400
        if running:
            m.mem[old:old + volume.SKILL_SIZE] = bytes(volume.SKILL_SIZE)
            m.store(old, 4, 1)
            m.store(old + volume.skill_at(0x70), 4, va(me))
            m.store(self.scene.skill_top, 4, old)
            m.store(self.scene.skill_tail, 4, old)
        self.scene.new_at = NEW
        g.calls = []
        ret = m.call(g.sym("_ccSkillRequest__FP6ccCharP6ccCharii"), (va(me), va(tgt), sid, stype))
        out = {"ret": int(ret != 0), "rand": g.rand_now(), "calls": [],
               "chars": self.scene.read(chars)}
        if ret:
            idx = {f"c{i}": i for i in range(len(chars))}
            out.update(ac=g.shorts(ret + 0x54, 1)[0], modify=(m.load(ret + 12, 1) >> 6) & 1,
                       hold=m.load(ret + 13, 1) & 1,
                       # ccWordsPlay(sid, ch): its skill and caster, which
                       # the runtime's Show::Words carries to the sound task.
                       words=[[c[1], idx[c[2]]] for c in g.calls if c[0] == "ccWordsPlay"],
                       start=[[idx[c[1]], c[3], c[4]] for c in g.calls if c[0] == "effSkillStart"])
            st = m.load(ret + 12, 1) & 15
            out["new"] = ([m.load(ret, 4, True), st - 16 if st & 8 else st, m.load(ret + 0x28, 4, True),
                           m.load(ret + 0x24, 4, True), m.load(ret + 0x50, 4)]
                          + [m.load(ret + 0x30 + 4 * k, 4) for k in range(4)]
                          + [m.load(ret + 0x60 + 4 * k, 4) for k in range(4)])
            old_st = (m.load(old + 12, 1) >> 4) & 3
            out["old"] = (old_st - 4 if old_st & 2 else old_st) if running else 0
        self.scene.restore()
        g.restore(self.data.skills[sid]["va"], 0x38)
        req = f"{self.ser_scene(chars, pcs, enes)} request {me} {tgt} {sid} {stype} {running} {state}"
        return [patch, req, f"restore {sid}"], out, "request"


def affect_hooks(checks):
    """Hooks for the calls affects make, installed once."""
    if getattr(checks, "_affect_hooked", False):
        return
    checks._affect_hooked = True
    g = checks.game
    m = g.m
    sym = g.sym
    rec = g.record
    b = checks.b

    def owner(a):
        return a - 0x1A0

    hooks = {
        "ccHitMarkDisp__FP6ccCharP6ccChar": lambda mm, a, b_, *_: rec("ccHitMarkDisp", a, b_),
        "DamageActuate__8ccPlayerFi": lambda mm, a, b_, *_: rec("DamageActuate", b_),
        "SetPanelBure__10ccMenuCtrlFis": lambda mm, a, b_, c, *_: rec("SetPanelBure", b_, b.s16(c)),
        "ccSkillCheck__FP6ccChar": lambda mm, *_: checks.running,
        "ccSkillRequest__FP6ccCharP6ccChari": lambda mm, a, b_, c, *_: rec("ccSkillRequest", a, b_, c),
        "HitDisable__9ccCharHitFv": lambda mm, a, *_: rec("HitDisable", owner(a)),
        "HitEnable__9ccCharHitFv": lambda mm, a, *_: rec("HitEnable", owner(a)),
        "ChatMessageDamage__4ccAIFi": lambda mm, a, b_, *_: rec("ChatMessageDamage", a, b_),
        "ChatMessageResurrectPlz__4ccAIFv": lambda mm, a, *_: rec("ChatMessageResurrectPlz", a),
        "AffectMessages__4ccAIFiP6ccChari": lambda mm, a, b_, c, d: rec("AffectMessages", a, b_, c, d),
        "Greeting__4ccAIFP6ccChari": lambda mm, a, b_, c, *_: rec("Greeting", a, b_, c),
        "ccAISysMsgSendP__FisUsUsUsUsiPv":
            lambda mm, a, b_, c, d: rec("ccAISysMsgSendP", a, b.s16(b_), c, d, mm.r[11]),
        "ccAISysMsgDeleteDelay__FUiUsUs": lambda mm, a, b_, c, *_: rec("ccAISysMsgDeleteDelay", a, b_, c),
        "effAfterDrain__FP6ccChari": lambda mm, a, b_, *_: rec("effAfterDrain", a, b_),
        "selectTarget__7ccEnemyFv": lambda mm, a, *_: rec("selectTarget", a),
        "effResistantShield__FP6ccCharii": lambda mm, a, b_, c, *_: rec("effResistantShield", a, b_, b.s32(c)),
    }

    def clear_effect(mm, a, *_):
        rec("ClearConditionEffect", a)
        mm.store(a + 0x30, 4, 0xFFFFFFFF)
        return 0
    hooks["ClearConditionEffect__6ccCharFv"] = clear_effect
    for name, fn in hooks.items():
        m.hooks[sym(name)] = fn
    if volume.NAME != "infection":      # the AI's record of the hit (unnamed; Mutation 0x5c1a60)
        m.hooks[volume.callee("Influence__FP6ccChar", 0)] = lambda mm, a, b_, *_: rec("NoteHit", a, b_)
    checks.entry_affect_hook = m.hooks.get(sym("EntryAffect__6ccCharFP6ccCharssss"))
    checks.affect_funcs = {1: sym("ccEnemyInfluence__FP6ccChar"), 2: sym("Influence__8ccFellowFv"),
                           3: sym("Influence__FP6ccChar")}


def check_affect(checks, rnd):
    """ccChar::EntryAffect on one character of a scene, with its real
    affectFunc: enemies (ccEnemyInfluence, affectEnemy), party members
    (ccFellow::Influence) and Kite (Influence)."""
    affect_hooks(checks)
    b, tb = checks.b, checks.tb
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    for c in chars:
        if c.type & 7:
            c.affect_func = rnd.choice((2, 2, 3, 0))
            c.act_num = rnd.choice((0, 1, 5, 7, 9, 10, 14, rnd.randrange(0, 20)))
            c.spc_flags = rnd.getrandbits(32) & ~(0x80 | 7 << 14)
            c.fellow_flags = rnd.getrandbits(8)
            c.sys_msg_id = rnd.choice((-1, rnd.randrange(0, 20)))
            c.arms_effect_sw = rnd.randrange(0, 3)
            c.attack = rnd.randrange(0, 3)
            c.cnt = rnd.randrange(0, 100)
            c.cloak = rnd.choice((0, F_ONE))
            c.no_death = rnd.random() < 0.15
            c.exp = rnd.choice((0, 500, 999, 1000, 1500))
        else:
            # affectEnemy is ccEnemy's: bosses have their own Affect
            c.affect_func = 0 if c.type & 0x80 else rnd.choice((1, 1, 1, 0))
            c.spc_flags = rnd.getrandbits(8) & ~0x80
            c.enemy_flags = rnd.choice((0, 0x40, rnd.getrandbits(9)))
        c.affect_type = rnd.choice((0, 0, 0, 13, 1))
        c.affect_mask = rnd.choice((0, 0, 0, 1 << rnd.randrange(0, 21)))
        c.cond[0] = rnd.choice((0, 0, 0, 1, 2, 5))
        c.HP = rnd.choice((c.HP, 0, 1, rnd.randrange(0, 50)))
    on = rnd.randrange(len(chars))
    by = rnd.randrange(len(chars))
    kind = rnd.choice(tuple(range(0, 21)) + (1, 1, 1, 3, 7, 13, 20, 20))
    p = [rnd.choice((-1, 0, 1, rnd.randrange(0, 60), rnd.randrange(0, 3000), tb.rnd_short(rnd))),
         rnd.choice((0, 1, rnd.randrange(0, 304))), rnd.randrange(-2, 5)]
    if not chars[on].type & 7 and rnd.random() < 0.3:
        # the resistant shield: an Exdefense immunity against the skill's
        # kind of defence (ccSkillCheckType of the skill)
        chars[on].row["Exdefense"] = rnd.choice((1, 2, 3))
        kind = 1
        p[1] = rnd.choice((1, rnd.randrange(0, 304), rnd.randrange(6, 150), rnd.randrange(150, 200)))
    running = rnd.choice((0, 1, 1, 2))
    menu = rnd.choice((1, 1, 0))
    ids = [rnd.choice((-1, rnd.randrange(0, 18), chars[on].id)) for _ in range(3)]
    state = rnd.getrandbits(64)
    va = checks.scene.va
    checks.scene.put(chars, pcs, enes)
    for i, c in enumerate(chars):
        a = va(i)
        m.store(a + 0x94, 4, checks.affect_funcs.get(c.affect_func, 0))
        g.put(a + 0x9C, "<h", c.affect_type)
        m.store(a + 0xB0, 4, c.affect_mask)
        m.store(a + 0x78, 4, 0)
        if c.type & 7:
            m.store(a + 0xE0, 4, m.load(a + 0xE0, 4) | c.spc_flags)
            g.put(a + 0xEE, "<h", c.act_num)
            m.store(a + 0x200, 1, c.fellow_flags)
            m.store(a + 0xE4, 4, c.arms_effect_sw)
            g.put(a + 0xFE, "<h", c.attack)
            m.store(a + 0x114, 4, c.cnt)
            m.store(a + 0x110, 4, c.cloak)
            g.put(m.load(a, 4) + 0x10, "<h", c.exp)
        else:
            m.store(a + 0xE0, 1, (m.load(a + 0xE0, 1) & 0x80) | c.spc_flags)
            g.put(a + 0x250, "<H", c.enemy_flags)
            m.store(a + 0x238, 4, a + 8)
            m.store(a + 0x244, 4, c.id)
            # ccEnemy.eParam (+0x1d0) is the enemy's ccEnemyParam, which
            # affectEnemy reads directly
            m.mem[a + 0x1D0:a + 0x1D0 + 0x64] = m.mem[a + 0x400:a + 0x400 + 0x64]
    AIV = 0x01031000
    m.store(AIV, 1, 0xFF)
    m.store(g.sym("plw") + 0x20, 4, va(on))     # the player: Kite's Influence is only ever Kite's
    g.put(AIV + 0x160, "<h", getattr(chars[on], "sys_msg_id", -1))
    g.party = {i: n for n, i in enumerate(ids) if i >= 0 and i not in list(ids)[:n]}
    m.store(g.sym("ccMenu"), 4, 0x01020000 if menu else 0)
    checks.running = running
    g.set_rand(state)
    g.calls = []
    saved = m.hooks.pop(g.sym("EntryAffect__6ccCharFP6ccCharssss"), None)
    m.call(g.sym("EntryAffect__6ccCharFP6ccCharssss"),
           (va(on), va(by), kind & 0xFFFFFFFF, p[0] & 0xFFFFFFFF, p[1] & 0xFFFFFFFF, p[2] & 0xFFFFFFFF))
    if saved is not None:
        m.hooks[g.sym("EntryAffect__6ccCharFP6ccCharssss")] = saved
    m.store(g.sym("ccMenu"), 4, 0x01020000)
    calls = list(g.calls)
    if m.load(AIV, 1) != 0xFF:
        calls.append(("TalkOff",))
    a = va(on)
    ch = chars[on]
    person = m.load(a + 0x98, 4)
    out = {"ret": 0, "rand": g.rand_now(), "calls": calls, "chars": checks.scene.read(chars),
           "affect": list(g.shorts(a + 0x9C, 4)) + [(person - SCN) // 0x1000 if person else -1]
           + list(g.shorts(a + 0xA8, 2)) + [m.load(a + 0xAC, 4)] + list(g.shorts(a + 0xB4, 2))
           + [m.load(a + 0xB8, 4)]}
    tc = m.load(a + 0x78, 4)
    out["target"] = (tc - SCN) // 0x1000 if tc else -1
    if ch.type & 7:
        out["spc"] = [g.shorts(a + 0xEE, 1)[0], m.load(a + 0xE0, 4) & ~(0x80 | 7 << 14), m.load(a + 0x200, 1),
                      getattr(ch, "enemy_flags", 0), b.s32(m.load(a + 0xE4, 4)), g.shorts(a + 0xFE, 1)[0],
                      b.s32(m.load(a + 0x114, 4)), m.load(a + 0x110, 4)]
    else:
        out["spc"] = [getattr(ch, "act_num", 0), m.load(a + 0xE0, 4) & ~(0x80 | 7 << 14),
                      getattr(ch, "fellow_flags", 0), m.load(a + 0x250, 2), getattr(ch, "arms_effect_sw", 0),
                      getattr(ch, "attack", 0), getattr(ch, "cnt", 0), getattr(ch, "cloak", 0)]
    checks.scene.restore()
    g.party = {}
    req = (f"{checks.ser_scene(chars, pcs, enes)} affect {on} {by} {kind} {p[0]} {p[1]} {p[2]} {running} {menu} "
           f"{ids[0]} {ids[1]} {ids[2]} {state}")
    return req, out, "affect"


CHECKS = ("damage", "protect", "exdefense", "skill_damage", "area_damage", "value", "recovery", "area_recovery",
          "cure", "hold", "modify", "request", "calc_real", "disp_condition", "level", "modify_condition",
          "condition_success",
          "target_condition", "exp", "erosion", "affect")
EXTRA = {"affect": check_affect}


def diff(port, game, what):
    game = norm(game)
    if port == game:
        return None
    if isinstance(game, dict) and isinstance(port, dict):
        for k in game:
            if port.get(k) != game[k]:
                return f"{what}: {k}: port {port.get(k)!r} game {game[k]!r}"
        for k in port:
            if k not in game:
                return f"{what}: {k}: port has it, game does not"
    return f"{what}: port {port!r} game {game!r}"


def run(checks, name, n, seed):
    """n cases of one check: returns (mismatches, first difference, first
    request)."""
    rnd = random.Random(seed)
    fn = getattr(checks, name, None) or (lambda rnd: EXTRA[name](checks, rnd))
    lines, answers, where = [], [], []
    for _ in range(n):
        checks.reset()
        req, game, label = fn(rnd)
        if isinstance(req, list):
            where.append(len(lines) + 1)
            lines.extend(req)
        else:
            where.append(len(lines))
            lines.append(req)
        answers.append((game, label))
    got = ask(lines)
    bad, first, first_req = 0, None, None
    for k, (game, label) in enumerate(answers):
        port = got[where[k]]
        if isinstance(game, dict) and isinstance(port, dict) and "rand" not in game:
            port.pop("rand", None)
        d = diff(port, game, label)
        if d:
            bad += 1
            if first is None:
                first, first_req = d, lines[where[k]]
    return bad, first, first_req


READY = os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo") is not None


class Against(unittest.TestCase):
    """Checks of the port against the game, CASES random cases each; a
    module with checks of its own names them in TABLE, and MAKE builds the
    object they take (a Checks)."""
    CASES = 150
    TABLE = {}
    MAKE = None

    @classmethod
    def setUpClass(cls):
        EXTRA.update(cls.TABLE)
        build()
        cls.checks = (cls.MAKE or Checks)()

    def check(self, name, seed):
        bad, first, req = run(self.checks, name, self.CASES, seed)
        self.assertEqual(bad, 0, f"{first}\nrequest: {req}")


@unittest.skipUnless(READY, "needs the extracted disc and cargo")
class BattleAgainstGame(Against):
    def test_damage(self):
        self.check("damage", 1)
        self.check("protect", 2)
        self.check("exdefense", 3)

    def test_skill_damage(self):
        self.check("skill_damage", 4)
        self.check("area_damage", 5)

    def test_value(self):
        self.check("value", 6)

    def test_healing(self):
        self.check("recovery", 7)
        self.check("area_recovery", 8)
        self.check("cure", 9)

    def test_hold_modify_request(self):
        self.check("hold", 10)
        self.check("modify", 11)
        self.check("request", 12)

    def test_character(self):
        self.check("calc_real", 13)
        self.check("disp_condition", 17)
        self.check("level", 14)

    def test_conditions(self):
        self.check("modify_condition", 15)
        self.check("condition_success", 16)
        self.check("target_condition", 17)

    def test_rewards(self):
        self.check("exp", 18)
        self.check("erosion", 19)

    def test_affect(self):
        self.check("affect", 20)


def bulk(n, only=None, names=CHECKS, seed0=5000, table=None, make=None):
    """n cases of each check in names (or only those in only); prints a line
    per check and returns the mismatches."""
    EXTRA.update(table or {})
    build()
    checks = (make or Checks)()
    total = 0
    for k, name in enumerate(names):
        if only and name not in only:
            continue
        bad, first, req = run(checks, name, n, seed0 + k)
        total += bad
        print(f"{name:32} {n:6} cases, {bad} mismatches" + (f"; first: {first}" if first else ""))
        if first:
            print(f"  request: {req[:600]}")
    return total


def main(names=CHECKS, seed0=5000, table=None, make=None):
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        n = int(sys.argv[2]) if len(sys.argv) > 2 else 500
        sys.exit(1 if bulk(n, sys.argv[3:], names, seed0, table, make) else 0)
    unittest.main()


if __name__ == "__main__":
    main()
