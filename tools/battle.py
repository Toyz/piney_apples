#!/usr/bin/env python3
"""The battle rules of .hack//Infection, reproduced from the game code.

    tools/battle.py stats   ELF CHAR [--level N] [--equip SLOT=NAME ...]
    tools/battle.py damage  ELF ATTACKER TARGET [--skill NAME] [--seed N]
                            [--level N] [--target-level N] [--equip SLOT=NAME ...]
                            [--mag X] [--plcol]
    tools/battle.py levelup ELF CHAR [--to N]
    tools/battle.py skills  ELF [--all]
    tools/battle.py exp     ELF [--pc-level N]
    tools/battle.py drain   ELF TARGET [--skill NAME] [--erosion N] [--kite-level N]

CHAR is a party member from charTbl (demo.prg): a name ("Kite") or a row
number. ATTACKER and TARGET are a party member, an enemy from enemyTbl
("Goblin", or "enemy:12" for a row) or a boss from bossTbl ("boss:Skeith",
"boss:0"). SLOT is head, body, arm, leg or weapon; NAME is an item name or a
row number of that slot's table (weapons come from the table of the
character's job). NAME for --skill is a skillTbl name or row.

Everything here is a transcription of gcmn.prg functions; the ones marked
"checked" in the comments are compared against the game's own code run in
tools/eemu.py by tools/test_battle.py:

  ccChar::CalcReal            0x0056ba30  effective stats            checked
  ccChar::ConditionTimeCount  0x0056bfd0  buff timers, poison ...    checked
  ccChar::ConditionBattleEffect 0x0056c940 equipment battle effects  checked
  ccChar::CheckLevelUp        0x0056d430  1000 exp per level         checked
  ccChar::LevelDown           0x0056d640  Data Drain level loss      checked
  ccChar::CalcBattleDamage    0x0056d910  hit, damage, protect ...   checked
  ccSkillDamage               0x00573e60  attribute critical, x2     checked (single target)
  ccExpDistributor            0x00571290  exp per kill               checked
  ccCheckConditionSkillSuccess 0x00571830 status resistance          checked
  ccSetLevelParam             0x00571a00  stats at a level (new game) checked
  _ccSkillModifyCondition     0x00575cf0  status and buff skills     checked
  ccSkillRecovery             0x00574bc0  heal amounts               checked (single target)
  ccSkillDamageValue          0x00594e90  the AI's damage estimate   checked
  ccSkill::RecoverySystem     0x00577070  Antidote, Restorative ...  checked
  ccSaveData::AddLvErosion    main 0x001787a0  infection from Data Drain  checked
  Data Drain drop roll        in ccMenuCtrl::DataDrainMenu 0x00533100  read only

The RNG is newlib's rand() (main 0x00133a38): a 64-bit LCG in the reentrancy
struct, next = next * 6364136223846793005 + 1, returning bits 32-62. --seed
sets that state, as srand() would.

ELF can be any of the four volumes. The tables are found from the code that
reads them (see Data), and the one rule that changed after Infection, the
Exdefense immunity in CalcBattleDamage, is read from the executable
(exdefense_cancel). The addresses above are Infection's.
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from image import Program  # noqa: E402
import eemu  # noqa: E402

BA = ("pAtk", "pDef", "pHit", "pEva", "mAtk", "mDef", "mHit", "mEva")
AT = ("soil", "water", "fire", "wind", "thunder", "dark")
TO = ("spirit", "body")
ELM = BA + AT + TO                      # ccCharParamElement, 16 shorts
P_ATK, P_DEF, P_HIT, P_EVA, M_ATK, M_DEF, M_HIT, M_EVA = range(8)
COND = ("dead", "hold", "drainHP", "drainSP", "critical", "dying", "invincible",
        "regeneSP", "curse", "sleep", "confusion", "charm", "regeneHP", "poison",
        "paralysis", "speed")           # ccCondition, 16 shorts then float speedValue
(DEAD, HOLD, DRAIN_HP, DRAIN_SP, CRITICAL, DYING, INVINCIBLE, REGENE_SP, CURSE,
 SLEEP, CONFUSION, CHARM, REGENE_HP, POISON, PARALYSIS, SPEED) = range(16)
BEFF = ("drainHP", "drainSP", "critical", "dying", "invincible")
SLOTS = ("head", "body", "arm", "leg", "weapon", "shield")   # ccEquipment
F_ONE = 0x3F800000                      # 1.0f

# ccCharBaseParam.type: which ccChar a record belongs to
PC_TYPES, ENEMY_TYPES, BOSS_TYPES = 0x07, 0x60, 0x80

# ccSkillParam.type bits. 1, 2 and the elements are read by CalcBattleDamage;
# the rest by the functions named.
SKILL_BITS = [
    (0x00001, "physical"), (0x00002, "magic"),
    (0x00004, "soil"), (0x00008, "water"), (0x00010, "fire"), (0x00020, "wind"),
    (0x00040, "thunder"), (0x00080, "dark"),
    (0x00100, "attack-spell"), (0x00200, "support-spell"),     # inferred from the rows
    # the three arts of each weapon class: ccSkillDamageValue counts their hits
    # by job (ART_HITS); what else tells them apart is not traced
    (0x00400, "art400"), (0x00800, "art800"), (0x01000, "art1000"),
    (0x02000, "centred-on-user"),   # ccSkillDamage 0x00573f28: area centre is the caster
    (0x04000, "untargeted"),        # ccSkill::Main 0x00573378: goes on when its target dies or leaves
    (0x08000, "splash-half"),       # ccSkillDamage: other targets in range take x0.5
    (0x10000, "buff"), (0x20000, "debuff"), (0x40000, "heal"),  # _ccSkillCheckType
]

M32 = 0xFFFFFFFF


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= M32
    return v - 0x100000000 if v & 0x80000000 else v


def cdiv(a, b):
    """C division, truncating toward zero."""
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q


def cmod(a, b):
    return a - cdiv(a, b) * b


def fptosi(bits):
    """libgcc fptosi (main 0x00129c18) on a float's bits: truncate,
    saturate at the int range."""
    if (bits >> 23) & 0xFF == 0:
        return 0
    return s32(eemu.f_to_int(bits))


def scale(v, mag_bits):
    """(int)((float)v * mag) as the game computes it: cvt.s.w, mul.s, fptosi."""
    return fptosi(eemu.f_mul(eemu.f_from_int(v & M32), mag_bits))


class Rand:
    """newlib rand(): the state is _impure_ptr->_new._reent._rand_next."""

    def __init__(self, state=1):
        self.state = state & ((1 << 64) - 1)
        self.count = 0

    def __call__(self):
        self.state = (self.state * 6364136223846793005 + 1) & ((1 << 64) - 1)
        self.count += 1
        return (self.state >> 32) & 0x7FFFFFFF


# reading the code ----------------------------------------------------------------
# The later volumes are stripped and their names carried over from Infection
# (piney-gen syms): a carried data name can be missing or wrong (Mutation's
# "skillTbl" sits 0x11b0 bytes inside the real table) and its size is
# Infection's. So every table is found from the code that indexes it, and its
# length from a loop bound or from where the next table starts.

def function_words(prog, name):
    """(start, [words]) of a named function."""
    s = prog.symbol_named(name)
    if s is None:
        raise KeyError(f"{prog.path}: no function {name}")
    f = prog.function_at(s.value) or s
    n = f.size // 4
    return f.value, list(struct.unpack(f"<{n}I", prog.read(f.value, 4 * n)))


def formed(prog, name):
    """[(instruction index, address)] for every absolute address the function
    builds (lui/lo pairs, $gp offsets), in instruction order."""
    import xfer
    va, w = function_words(prog, name)
    a = xfer.addresses(w, va, prog.gp)
    return [(i, a[i]) for i in sorted(a)]


def first_formed(prog, name):
    return formed(prog, name)[0][1]


def loop_bound(prog, name, around=None):
    """N of a counted loop's `slti rt, rs, N` / `bnez rt, back`: the
    innermost such loop containing instruction index `around`, or else the
    last one in the function."""
    _, w = function_words(prog, name)
    best = None
    for i, x in enumerate(w):
        if x >> 26 != 0x0A:                              # slti
            continue
        rt = (x >> 16) & 31
        for j in range(i + 1, min(i + 4, len(w))):
            y = w[j]
            if y >> 26 == 0x05 and (y >> 21) & 31 == rt and (y >> 16) & 31 == 0:   # bnez rt
                top = j + 1 + s16(y)
                if top <= i and (around is None or top <= around <= j):
                    if around is None or best is None or j - top < best[0]:
                        best = (j - top, s16(x))
                break
    if best is None:
        raise KeyError(f"{prog.path}: no counted loop in {name}")
    return best[1]


def exdefense_cancel(prog):
    """The Exdefense rule of ccChar::CalcBattleDamage. Infection (gcmn
    0x0056dc64) keeps the target's whole Exdefense value when any defence
    the skill works against is not lowered, and drops it otherwise: returns
    None. From Mutation on (gcmn MUT 0x005933f8, OUT 0x0058f5c0, QUA
    0x00481f00) the code
    collects just those bits, one `ori r, r, bit` each for bits 0x1..0x80,
    and then clears them all when Exdefense has the bit of the `andi` that
    follows (0x100): returns that bit. Detected by the `ori r, r, 0x80`."""
    _, w = function_words(prog, "CalcBattleDamage__6ccCharFP6ccCharP12ccSkillParamfi")
    for i, x in enumerate(w):
        if x >> 26 == 0x0D and x & 0xFFFF == 0x80 and (x >> 21) & 31 == (x >> 16) & 31:
            for y in w[i + 1:i + 8]:
                if y >> 26 == 0x0C:                     # andi
                    return y & 0xFFFF
    return None


# the game data ---------------------------------------------------------------

class Data:
    """The battle tables, read straight from gcmn.prg (the overlay's
    constructors do not touch them) and charTbl from demo.prg (main on
    Outbreak and Quarantine), each found from the code that reads it."""

    TABLES = [("skills", "ccGetSkillParam__Fi", 0x38),
              ("boss_skills", "ccGetBossSkillParam__Fi", 0x38),
              ("enemies", "ccGetEnemyParam__Fi", 0x1C0), ("bosses", "ccGetBossParam__Fi", 0x68)]
    TABLES += [(s, f"ccGetEquip{s.title()}Param__Fi", 0x48) for s in ("head", "body", "arm", "leg")]
    TABLES += [(f"weapon{j}", f"ccGetEquipWeapon{j}Param__Fi", 0x48) for j in range(1, 7)]

    def __init__(self, elf_path):
        self.elf_path = elf_path
        self.p = Program(elf_path, "gcmn")
        self.demo = Program(elf_path, "demo")
        self.main = Program(elf_path)
        p = self.p
        base = {k: first_formed(p, fn) for k, fn, _ in self.TABLES}
        # ccGetItemParam (0x00570d70): categories 10-15 through a jump table;
        # ccGetEquipParam (0x00570f60): 0-5 the job weapons, 6-9 head, body, arm, leg
        va, _ = function_words(p, "ccGetItemParam__Fi")
        got = formed(p, "ccGetItemParam__Fi")
        jt = got[0][1]
        for c in range(6):
            case = (p.u32(jt + 4 * c) - va) // 4
            base[f"item{c}"] = next(a for i, a in got if i >= case)
        # a table ends at the next one or at the first row with no name
        starts = sorted(base.values())
        self.rows = {}
        for k, fn, size in self.TABLES + [(f"item{c}", None, 20) for c in range(6)]:
            later = [s for s in starts if s > base[k]]
            end = later[0] if later else 1 << 32
            n = 0
            while base[k] + size * (n + 1) <= end:
                name = p.u32(base[k] + size * n)
                if not name or not p.mapped(name):
                    break
                n += 1
            self.rows[k] = (base[k], n)
        self.skills = [self._skill(a) for a in self._rows("skills", 0x38)]
        self.boss_skills = [self._skill(a) for a in self._rows("boss_skills", 0x38)]
        self.enemies = [self._param(a, i) for i, a in enumerate(self._rows("enemies", 0x1C0))]
        self.bosses = [self._param(a, i) for i, a in enumerate(self._rows("bosses", 0x68))]
        self.equip = {s: self._equip_table(s) for s in ("head", "body", "arm", "leg")}
        # ccGetJobWeaponParam (0x00571100): job j wears equipmentWeapon<j+1>Tbl
        self.weapons = [self._equip_table(f"weapon{j + 1}") for j in range(6)]
        self.items = {10 + c: [self._cstr(p.u32(a)) for a in self._rows(f"item{c}", 20)]
                      for c in range(6)}
        # charTbl: ccSaveData::NewGame (main 0x00174d70) copies one row per
        # character, 18 in Infection and 21 from Mutation on
        at, cbase = formed(self.main, "NewGame__10ccSaveDataFi")[0]
        nchar = loop_bound(self.main, "NewGame__10ccSaveDataFi", at)
        self.chars = [self._char(cbase + 0x5C * i) for i in range(nchar)]
        # LevelUpParamTbl is indexed by the character id: a row each
        base = first_formed(p, "CheckLevelUp__6ccCharFv")
        self.levelup = [self._shorts(base + 0x24 * i, 18) for i in range(nchar)]
        # ccExpDistributor reads expCalcTbl[diff + 10], the level difference
        # clamped to -10..10; the only initialised-data address it forms
        ov = p.overlay
        data0 = ov.text + ov.text_size
        base = next(a for _, a in formed(p, "ccExpDistributor__Fs")
                    if data0 <= a < data0 + ov.data_size) - 0x14
        self.exp_calc = self._shorts(base, 21)
        base = first_formed(p, "ccCheckDrainEnemy__Fi") - 4
        self.race_num = [struct.unpack("<i", p.read(base + 12 * i + 4, 4))[0]
                         for i in range(loop_bound(p, "ccCheckDrainEnemy__Fi"))]
        # indexed by erosion / 25, erosion 0..100; the name is carried right on
        # every volume (it is the address DataDrainMenu forms after `sll 6`)
        base = p.symbol_named("dataDrainErosionTbl").value
        self.drain_effects = [list(struct.unpack("<16i", p.read(base + 64 * i, 64))) for i in range(5)]
        # ccSaveData::AddLvErosion (main 0x001787a0) copies this into erosionTbl[16]
        self.erosion_va = first_formed(self.main, "AddLvErosion__10ccSaveDataFif")
        self.erosion = list(struct.unpack("<16i", self.main.read(self.erosion_va, 64)))
        self.exdef_cancel = exdefense_cancel(p)

    def _rows(self, key, size):
        va, n = self.rows[key]
        return [va + size * i for i in range(n)]

    # raw readers
    def _shorts(self, va, n, prog=None):
        return list(struct.unpack(f"<{n}h", (prog or self.p).read(va, 2 * n)))

    def _cstr(self, va, prog=None):
        prog = prog or self.p
        if not va:
            return ""
        b = prog.read(va, 64).split(b"\0", 1)[0]
        return b.decode("shift_jis", "backslashreplace")

    def _skill(self, va):
        p = self.p
        name, fname = struct.unpack("<II", p.read(va, 8))
        atk, hit = struct.unpack("<hh", p.read(va + 8, 4))
        cond, trig, rng, ttype, cost, typ = struct.unpack("<iffiii", p.read(va + 0x18, 24))
        level, rate = struct.unpack("<hh", p.read(va + 0x30, 4))
        return {"name": self._cstr(name), "file": self._cstr(fname), "atk": atk, "hit": hit,
                "attr": self._shorts(va + 0xC, 6), "condition": cond, "triggerRange": trig,
                "targetRange": rng, "targetType": ttype, "cost": cost, "type": typ,
                "level": level, "dmgRate": rate, "va": va}

    def _param(self, va, index):
        """ccEnemyParamData / ccBossParamData (same layout, 0x68 bytes)."""
        p = self.p
        name, ccs, typ = struct.unpack("<IIi", p.read(va, 12))
        idn, level, exp = struct.unpack("<hhh", p.read(va + 12, 6))
        maxhp, maxsp = struct.unpack("<hh", p.read(va + 0x24, 4))
        maxpp, pdefpp, mdefpp = struct.unpack("<hhh", p.read(va + 0x52, 6))
        items = list(struct.unpack("<3i", p.read(va + 0x58, 12)))
        exdef = struct.unpack("<h", p.read(va + 0x64, 2))[0]
        return {"index": index, "name": self._cstr(name), "ccs": self._cstr(ccs), "type": typ,
                "id": idn, "level": level, "exp": exp, "maxHP": maxhp, "maxSP": maxsp,
                "elm": self._shorts(va + 0x28, 16), "beff": self._shorts(va + 0x48, 5),
                "maxPP": maxpp, "pDefPP": pdefpp, "mDefPP": mdefpp, "item": items,
                "Exdefense": exdef, "va": va}

    def _equip_table(self, key):
        out = []
        for i, va in enumerate(self._rows(key, 0x48)):
            nm = struct.unpack("<I", self.p.read(va, 4))[0]
            out.append({"index": i, "name": self._cstr(nm), "elm": self._shorts(va + 0xA, 16),
                        "beff": self._shorts(va + 0x2A, 5),
                        "level": struct.unpack("<h", self.p.read(va + 0x38, 2))[0], "va": va})
        return out

    def _char(self, va):
        d = self.demo
        name, ccs, typ = struct.unpack("<IIi", d.read(va, 12))
        idn, level, exp = struct.unpack("<hhh", d.read(va + 12, 6))
        maxhp, maxsp = struct.unpack("<hh", d.read(va + 0x24, 4))
        job = struct.unpack("<h", d.read(va + 0x58, 2))[0]
        return {"name": self._cstr(name, d), "ccs": self._cstr(ccs, d), "type": typ, "id": idn,
                "level": level, "exp": exp, "maxHP": maxhp, "maxSP": maxsp,
                "elm": self._shorts(va + 0x28, 16, d), "equip": self._shorts(va + 0x48, 6, d),
                "job": job}

    # lookups
    def weapon(self, job, index):
        """ccGetJobWeaponParam; None for a job outside 0-5 (the game reads
        through a null pointer there)."""
        return self.weapons[job][index] if 0 <= job < 6 else None

    def item_name(self, code):
        """An item code, (category << 16) | row, as ccItemList and the
        enemies' item[] hold it."""
        cat, i = code >> 16, code & 0xFFFF
        if 0 <= cat < 6:
            rows = [r["name"] for r in self.weapons[cat]]
        elif 6 <= cat < 10:
            rows = [r["name"] for r in self.equip[("head", "body", "arm", "leg")[cat - 6]]]
        else:
            rows = self.items.get(cat, [])
        return rows[i] if 0 <= i < len(rows) else f"0x{code:x}"

    def check_drain_enemy(self, eid):
        """ccCheckDrainEnemy (gcmn 0x0042e330): true for the first row of
        each ccEntryRaceTbl group, the drained forms."""
        total = 0
        for n in self.race_num:
            first = total
            total += n
            if eid < total:
                eid -= first
                break
        return eid == 0

    def find(self, rows, key, what):
        if isinstance(key, int) or key.isdigit():
            if int(key) >= len(rows):
                raise SystemExit(f"no {what} row {key} (there are {len(rows)})")
            return rows[int(key)]
        for r in rows:
            if r["name"] == key:
                return r
        low = [r for r in rows if r["name"].lower() == key.lower()]
        if low:
            return low[0]
        pre = [r for r in rows if r["name"].lower().startswith(key.lower())]
        if len({r["name"] for r in pre}) == 1:
            return pre[0]
        names = sorted({r["name"] for r in pre})[:8]
        raise SystemExit(f"no {what} {key!r}" + (f" (could be {', '.join(names)})" if pre else ""))

    def skill(self, key):
        rows = [dict(s, index=i) for i, s in enumerate(self.skills)]
        return self.find(rows, key, "skill")

    def char(self, key):
        rows = [dict(c, index=i) for i, c in enumerate(self.chars)]
        return self.find(rows, key, "party member")


# element helpers (ccAddBattleAbility and friends, 0x005704c0 ...) -------------

def add_elm(d, s, hi, lo, fields=range(16)):
    """ccAddParamElement: each field wraps to a short, then is clamped."""
    for i in fields:
        v = s16(d[i] + s[i])
        d[i] = hi if v > hi else lo if v < lo else v


def mul_add_elm(d, s, n, fields):
    """ccMulBattleAbility / ccMulAttribute: d += s * n, clamped to 0..999."""
    for i in fields:
        v = s16(d[i] + s32(s[i] * n))
        d[i] = 999 if v >= 1000 else 0 if v < 0 else v


# party members: stats, levels, equipment ------------------------------------

class PC:
    """A party member: the ccSpcParam fields and the ccChar fields that go
    with it."""

    def __init__(self, data, row):
        self.data = data
        self.name = row["name"]
        self.type = row["type"]
        self.id = row["id"]
        self.level = row["level"]
        self.exp = row["exp"]
        self.p_maxHP = row["maxHP"]          # ccSpcParam.maxHP
        self.p_maxSP = row["maxSP"]
        self.elm = list(row["elm"])
        self.real = [0] * 16
        self.tune = [0] * 16
        self.temp = [0] * 16
        self.time = [0] * 16
        self.equip = list(row["equip"])
        self.job = row["job"]
        # ccChar
        self.HP, self.SP = self.p_maxHP, self.p_maxSP
        self.maxHP, self.maxSP = self.p_maxHP, self.p_maxSP
        self.cond = [0] * 16
        self.speed_value = F_ONE
        self.no_death = False                # ccSpcChar.noDeathFlag
        self.ent_root = 1                    # ccChar+0x140 (weaponPos[0]), read by "dying"

    def equipment(self, slot):
        i = self.equip[SLOTS.index(slot)]
        if slot == "weapon":
            return self.data.weapon(self.job, i)
        return self.data.equip[slot][i]


def calc_real_pc(pc, flag=1, env=None):
    """ccChar::CalcReal (0x0056ba30), party-member branch: real = elm + head
    + body + leg + arm + weapon (each step clamped to -999..999), then the
    timed buffs in temp. flag 0 also runs the per-frame timers first."""
    ev = []
    pc.real = list(pc.elm)
    pc.tune = list(pc.equipment("head")["elm"])
    for slot in ("body", "leg", "arm", "weapon"):
        add_elm(pc.tune, pc.equipment(slot)["elm"], 999, -999)
    add_elm(pc.real, pc.tune, 999, -999)
    pc.maxHP, pc.maxSP = pc.p_maxHP, pc.p_maxSP
    if flag == 0:
        ev += condition_time_count(pc, env or Env())
    condition_battle_effect(pc)
    add_elm(pc.real, pc.temp, 999, -999)
    if flag <= 0 and (env or Env()).area:
        ev.append(("DispConditionEffect", "self"))
    return ev


def condition_battle_effect(ch):
    """ccChar::ConditionBattleEffect (0x0056c940): drainHP, drainSP,
    critical, dying and invincible are the largest value any equipped piece
    (or the enemy's table row) carries."""
    for i in range(5):
        ch.cond[DRAIN_HP + i] = 0
    if isinstance(ch, PC):
        pieces = [ch.equipment(s) for s in ("head", "body", "arm", "leg")]
        pieces.append(ch.equipment("weapon") if 0 <= ch.job < 6 else None)
        for eq in pieces:
            if eq is None:
                continue      # the game merges the leg piece again: no change
            for i in range(5):
                if ch.cond[DRAIN_HP + i] < eq["beff"][i]:
                    ch.cond[DRAIN_HP + i] = eq["beff"][i]
    elif getattr(ch, "kind", None) in ("enemy", "boss"):
        for i in range(5):
            if ch.cond[DRAIN_HP + i] < ch.row["beff"][i]:
                ch.cond[DRAIN_HP + i] = ch.row["beff"][i]


def check_level_up(pc):
    """ccChar::CheckLevelUp (0x0056d430): every 1000 exp is a level; each
    level adds the character's LevelUpParamTbl row once. Returns the number
    of levels gained."""
    t = pc.data.levelup[pc.id]
    n = 0
    while pc.exp - 999 > 0:
        pc.exp = s16(pc.exp - 1000)
        pc.level = s16(pc.level + 1)
        # the row is maxHP, maxSP, then an element block, all clamped 0..999
        add_elm(pc.elm, t[2:], 999, 0)
        pc.p_maxHP = s16(pc.p_maxHP + t[0])
        pc.p_maxSP = s16(pc.p_maxSP + t[1])
        pc.p_maxHP = 1 if pc.p_maxHP <= 0 else 9999 if pc.p_maxHP >= 10000 else pc.p_maxHP
        pc.p_maxSP = 1 if pc.p_maxSP <= 0 else 999 if pc.p_maxSP >= 1000 else pc.p_maxSP
        pc.maxHP, pc.maxSP = pc.p_maxHP, pc.p_maxSP
        pc.HP = s16(pc.HP + t[0])
        pc.SP = s16(pc.SP + t[1])
        if pc.HP >= 10000:
            pc.HP = 9999
        if pc.SP >= 1000:
            pc.SP = 999
        n += 1
        if pc.level >= 99:
            pc.level, pc.exp = 99, 0
            break
    return n


def level_down(pc):
    """ccChar::LevelDown (0x0056d640), called by the Data Drain menu: one
    level's growth is taken back."""
    pc.level = s16(pc.level - 1)
    if pc.level <= 0:
        pc.level = 1
        return
    t = pc.data.levelup[pc.id]
    neg = [s16(-v) for v in t[2:]]
    add_elm(pc.elm, neg, 999, 0)
    pc.p_maxHP = s16(pc.p_maxHP - t[0])
    pc.p_maxSP = s16(pc.p_maxSP - t[1])
    pc.p_maxHP = 1 if pc.p_maxHP <= 0 else 9999 if pc.p_maxHP >= 10000 else pc.p_maxHP
    pc.p_maxSP = 1 if pc.p_maxSP <= 0 else 999 if pc.p_maxSP >= 1000 else pc.p_maxSP
    pc.maxHP, pc.maxSP = pc.p_maxHP, pc.p_maxSP
    if pc.HP > pc.p_maxHP:
        pc.HP = pc.p_maxHP
    if pc.SP > pc.p_maxSP:
        pc.SP = pc.p_maxSP


def set_level_param(pc, lvs):
    """ccSetLevelParam (0x00571a00): jump to level lvs in one step (used by
    ccSaveData::InitSpcParam for a debug new game)."""
    t = pc.data.levelup[pc.id]
    lv = lvs - pc.level
    if lv > 0:
        mul_add_elm(pc.elm, t[2:], lv, range(16))
        pc.p_maxHP = s16(pc.p_maxHP + s32(t[0] * lv))
        pc.p_maxSP = s16(pc.p_maxSP + s32(t[1] * lv))
    elif lv < 0:
        for _ in range(-lv):
            pc.level = s16(pc.level - 1)
            if pc.level <= 0:
                pc.level = 1
                break
            add_elm(pc.elm, [s16(-v) for v in t[2:]], 999, 0)
            pc.p_maxHP = s16(pc.p_maxHP - t[0])
            pc.p_maxSP = s16(pc.p_maxSP - t[1])
    pc.p_maxHP = 1 if pc.p_maxHP <= 0 else 9999 if pc.p_maxHP >= 10000 else pc.p_maxHP
    pc.p_maxSP = 1 if pc.p_maxSP <= 0 else 999 if pc.p_maxSP >= 1000 else pc.p_maxSP
    pc.level = s16(lvs)


def make_pc(data, key, level=None, equip=()):
    """A party member at a level (reached by CheckLevelUp, or LevelDown for
    a level below the table's) with optional equipment changes, stats
    computed by CalcReal."""
    pc = PC(data, data.char(key))
    if level is not None:
        while pc.level < min(level, 99):
            pc.exp = 1000
            check_level_up(pc)
        while pc.level > max(level, 1):
            level_down(pc)
    pc.HP, pc.SP = pc.p_maxHP, pc.p_maxSP
    for spec in equip:
        slot, _, name = spec.partition("=")
        if slot not in SLOTS[:5]:
            raise SystemExit(f"slot {slot!r}: one of head, body, arm, leg, weapon")
        rows = data.weapons[pc.job] if slot == "weapon" else data.equip[slot]
        pc.equip[SLOTS.index(slot)] = data.find(rows, name, slot)["index"]
    calc_real_pc(pc)
    return pc


# enemies and bosses ------------------------------------------------------------

class Foe:
    """An enemy or boss: its table row plus the ccEnemyParam/ccBossParam
    and ccChar state the battle code reads and writes."""

    def __init__(self, row, kind):
        self.kind = kind
        self.row = row
        self.name = row["name"]
        self.type = row["type"]
        self.id = row["id"]
        self.level = row["level"]
        self.real = list(row["elm"])
        self.temp = [0] * 16
        self.time = [0] * 16
        self.PP = self.PPcount = self.PPrestore = 0
        self.HP, self.SP = row["maxHP"], row["maxSP"]
        self.maxHP, self.maxSP = row["maxHP"], row["maxSP"]
        self.cond = [0] * 16
        self.speed_value = F_ONE
        self.ent_root = 1        # ccEntryObj.entParam.entRoot, non-zero for a placed enemy
        condition_battle_effect(self)


def calc_real_foe(f, flag=1, env=None):
    """ccChar::CalcReal, enemy and boss branches: real = the table's elm
    plus temp (clamped to 0..32767). flag 0 runs the timers and counts the
    protect-break recovery down."""
    env = env or Env()
    ev = []
    f.real = list(f.row["elm"])
    if f.kind == "enemy":
        f.maxHP, f.maxSP = f.row["maxHP"], f.row["maxSP"]
    if flag == 0:
        ev += condition_time_count(f, env)
    condition_battle_effect(f)
    add_elm(f.real, f.temp, 32767, 0)
    if flag == 0 and f.PPcount != 0 and (f.kind == "enemy" or f.PP >= 0):
        f.PPcount = s16(f.PPcount - 1)
        if f.PPcount <= 0:
            f.PPcount = 0
            if f.kind == "enemy":
                f.PP = cdiv(f.row["maxPP"], 2)
                ev.append(("effProtect", "self", 1, -1))
            else:
                f.PPrestore = s16(f.PPrestore + 1)
                if f.PPrestore >= 2:
                    f.PPrestore = 2
                    f.PP = cdiv(f.row["maxPP"] * 3, 4)
                else:
                    f.PP = cdiv(f.row["maxPP"], 2)
                ev.append(("effProtect", "self", 1, 2 if f.id else 1))
            ev.append(("SetProtect", 1, "self"))
    if flag <= 0 and env.area:
        ev.append(("DispConditionEffect", "self"))
    return ev


class Env:
    """The globals the battle code consults."""

    def __init__(self, plcol=1, menu_flag=0, in_battle=1, count=0, menu_type=-1,
                 sp_regene_speed=0, area=1):
        self.plcol = plcol                  # saveData.plcol (+0x6771)
        self.menu_flag = menu_flag          # ccMenuCtrl.forbid (+0xfe): no protect damage while set
        self.in_battle = in_battle          # ccGame.inBattle
        self.count = count                  # ccSystem.count, the frame counter
        self.menu_type = menu_type          # ccMenuCtrl::CheckMenuType(), -1 = no menu
        self.sp_regene_speed = sp_regene_speed   # ccSpcChar::CheckSpRegeneSpeed()
        self.area = area                    # ccGame.area


# per-frame conditions ------------------------------------------------------------

TIMED = (0, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13)    # pEva, mEva, tolerances never time out


def condition_time_count(ch, env):
    """ccChar::ConditionTimeCount (0x0056bfd0), once a frame. Returns the
    EntryAffect calls it makes: (type 9, HP) regeneration, (3, HP) poison,
    (10, SP) SP regeneration, (4, SP) curse."""
    ev = []
    for i in TIMED:
        t = ch.time[i]
        if t > 0:
            ch.time[i] = t = s16(t - 1)
            if t == 0:
                ch.temp[i] = 0
        elif t < 0:
            ch.time[i] = 0
            ch.temp[i] = 0
    c = ch.cond
    for i in (SLEEP, CONFUSION, CHARM, PARALYSIS):
        if c[i] > 0:
            c[i] -= 1
        elif c[i] < 0:
            c[i] = 0
    if c[SPEED] > 0:
        c[SPEED] -= 1
        if c[SPEED] == 0:
            ch.speed_value = F_ONE
    elif c[SPEED] < 0:
        c[SPEED] = 0
        ch.speed_value = F_ONE
    if c[REGENE_HP]:
        c[REGENE_HP] = s16(c[REGENE_HP] - 1)
        if cmod(c[REGENE_HP], 60) == 0:
            ev.append(("EntryAffect", "self", "self", 9, s16(cdiv(ch.maxHP, 50) or 1), 0, 0))
    is_pc = isinstance(ch, PC)
    if c[POISON] and env.menu_type == -1:
        c[POISON] = s16(c[POISON] - 1)
        if cmod(c[POISON], 90) == 0 and not (is_pc and ch.no_death):
            ev.append(("EntryAffect", "self", "self", 3, s16(cdiv(ch.maxHP, 100) or 1), 0, 0))
    if c[CURSE] == 0:
        # SP comes back once a second (count % 60): maxSP/50 in battle and
        # maxSP/20 out of it with the regeneration bonus, maxSP/100 and
        # maxSP/33 without; enemies always maxSP/50
        tick = (env.count & M32) % 60 == 0
        if is_pc:
            if env.sp_regene_speed:
                div = 50 if env.in_battle else 20
            else:
                div = 100 if env.in_battle else 33
            if tick:
                ch.SP = s16(ch.SP + (cdiv(ch.maxSP, div) or 1))
            if ch.SP > ch.maxSP:
                ch.SP = ch.maxSP
        elif tick:
            ch.SP = s16(ch.SP + (cdiv(ch.maxSP, 50) or 1))
            if ch.SP > ch.maxSP:
                ch.SP = ch.maxSP
    if c[REGENE_SP]:
        c[REGENE_SP] = s16(c[REGENE_SP] - 1)
        if cmod(c[REGENE_SP], 60) == 0:
            ev.append(("EntryAffect", "self", "self", 10, s16(cdiv(ch.maxSP, 50) or 1), 0, 0))
    if c[CURSE]:
        c[CURSE] = s16(c[CURSE] - 1)
        if cmod(c[CURSE], 90) == 0:
            ev.append(("EntryAffect", "self", "self", 4, s16(cdiv(ch.maxSP, 100) or 1), 0, 0))
    return ev


# damage ---------------------------------------------------------------------------

class Result:
    def __init__(self):
        self.dmg = -1
        self.roll = None        # rand() % 101, when drawn
        self.hit = None         # the roll after the accuracy adjustment
        self.recover = 0        # HP the target absorbs (attribute 999)
        self.pp = None          # protect damage dealt, when computed
        self.events = []        # the calls the game makes, in order


EXDEF = [(0x01, "ba", P_DEF), (0x02, "ba", M_DEF)] + [(4 << i, "at", i) for i in range(6)]


def calc_battle_damage(att, tgt, sk, mag=F_ONE, h=-1, rng=None, env=None,
                       data=None, listed=True, roll=None):
    """ccChar::CalcBattleDamage(ccChar *t, ccSkillParam *sk, float mag, int h)
    (gcmn 0x0056d910). h < 0 draws the hit roll and applies the side
    effects (protect gauge, equipment effects, absorption); h > 0 is a given
    roll, with side effects; h == 0 is a sure hit at 100% without them.
    roll= gives the roll with no side effects and no RNG, for tabulating.
    mag is the float's bit pattern. The target's protect gauge is updated
    in place, as the game does. data gives the executable's Exdefense rule
    and the drained enemy forms; without it, Infection's rule and no
    drained-form check."""
    env = env or Env()
    rng = rng or Rand()
    r = Result()
    ev = r.events
    if not listed or att.cond[DEAD] not in (0, 1) or tgt.cond[DEAD]:
        return r
    dmg = -1
    if roll is not None:
        hit, live = roll, False
    elif h < 0:
        hit, live = rng() % 101, True
        r.roll = hit
    elif h == 0:
        hit, live = 100, False
    else:
        hit, live = h, True
    rec = 0
    sat = sk["attr"]
    at = att.type
    if at & (PC_TYPES | ENEMY_TYPES | BOSS_TYPES):
        bbt, bat = att.real[0:8], att.real[8:14]
    else:
        # a trap or other object: never misses, hits with nothing
        bbt, bat = [0, 0, 999, 0, 0, 0, 999, 0], [0] * 6
        hit = 100
    npd = bool(at & PC_TYPES and getattr(att, "no_death", False)) or bool(env.menu_flag)
    tt = tgt.type
    adef = 0
    if tt & PC_TYPES:
        tbt, tat = tgt.real[0:8], tgt.real[8:14]
    elif tt & (ENEMY_TYPES | BOSS_TYPES):
        tbt, tat = tgt.real[0:8], tgt.real[8:14]
        base_ba, base_at = tgt.row["elm"][0:8], tgt.row["elm"][8:14]
        adef = tgt.row["Exdefense"]
    else:
        return r
    stype = sk["type"]
    cancel = data.exdef_cancel if data is not None else None
    if adef and cancel is None:
        # Exdefense (Infection): immune while any of the defences the skill
        # works against is not lowered below the table value
        keep = False
        for bit, blk, i in EXDEF:
            if adef & bit and stype & bit:
                cur, base = (tbt[i], base_ba[i]) if blk == "ba" else (tat[i], base_at[i])
                if not cur < base:
                    keep = True
                    break
        if not keep:
            adef = 0
    elif adef:
        # Mutation on (see exdefense_cancel): just those bits, and none at
        # all when the row has the cancel bit (0x100)
        got = 0
        for bit, blk, i in EXDEF:
            if adef & bit and stype & bit:
                cur, base = (tbt[i], base_ba[i]) if blk == "ba" else (tat[i], base_at[i])
                if not cur < base:
                    got |= bit
        adef = 0 if adef & cancel else got
    el = [0] * 6

    def elements():
        nonlocal rec
        for i in range(6):
            if not stype & (4 << i):
                continue
            if sat[i]:
                a = cdiv(sat[i] + bat[i], 10)
                d = cdiv(tat[i], 10)
                a = a if a > 0 else 1
                d = d if d > 0 else 1
                el[i] = cdiv(s32(a * a), 2 * d)
            if el[i] < 0:
                el[i] = 0
            elif tat[i] >= 999:
                v = cdiv(el[i], 10)
                rec += v if v > 0 else 1

    if stype & 1:
        kind = (P_ATK, P_DEF, P_HIT, P_EVA, 0xFD, "pDefPP")
    elif stype & 2:
        kind = (M_ATK, M_DEF, M_HIT, M_EVA, 0xFE, "mDefPP")
    else:
        kind = None
    if kind:
        ATK, DEF, HIT, EVA, mask, ppdef = kind
        if hit < 5:
            dmg = -1
        elif hit >= 95:
            dmg, hit = 1, 100
        else:
            hit += cdiv(bbt[HIT] + sk["hit"] - cdiv(2 * tbt[EVA], 3), 10)
            if hit < 50:
                dmg = -1
                hit = max(hit, 0)
            else:
                dmg = 1
                hit = min(hit, 100)
        r.hit = hit
        if dmg > 0:
            pf = adef & mask
            if not pf:
                a = bbt[ATK] + sk["atk"]
                d = tbt[DEF]
                a = a if a > 0 else 1
                d = d if d > 0 else 1
                dmg = cdiv(s32(a * a), 2 * d)
                dmg = cdiv(s32(dmg * hit), 100)
                if dmg <= 0:
                    dmg = 1
                elements()
                dmg = s32(dmg + sum(el))
                dmg = cdiv(s32(dmg * sk["dmgRate"]), 100)
                dmg = scale(dmg, mag)
                if dmg >= 10000:
                    dmg = 9999
            else:
                if live and not adef & 0xFFFFFF03:
                    ev.append(("ccParticleAttributeGuard", "t", "att"))
                dmg = 0
            if live and env.plcol and not pf:
                protect(tgt, sk, mag, hit, adef, npd, el, dmg, bbt[ATK], ppdef, r, data,
                        getattr(att, "id", 0), ATK == P_ATK)
            if stype & 1 and live and sk["cost"] == 0 and not pf:
                dmg = battle_effects(att, tgt, dmg, rng, ev)
    if live and rec > 0:
        ev.append(("ccEntryRecoveryReq", "t", rec))
    r.dmg = dmg
    r.recover = rec
    return r


def protect(tgt, sk, mag, hit, adef, npd, el, dmg, atk, ppdef, r, data, att_id, physical):
    """The protect gauge (PP): damage by pAtk/mAtk against the table's
    pDefPP/mDefPP fills it; full, it breaks for 300 frames - the window for
    Data Drain. A boss takes hit / 80 instead of hit / 100, and a physical
    hit on a boss is capped at 9999 before mag rather than after."""
    ev = r.events
    row = tgt.row if tgt.type & (ENEMY_TYPES | BOSS_TYPES) else None
    if tgt.type & ENEMY_TYPES:
        if data is not None and data.check_drain_enemy(tgt.id):
            return
        boss = False
    elif tgt.type & BOSS_TYPES:
        boss = True
    else:
        return
    if tgt.PPcount != 0 or tgt.PP < 0 or row["maxPP"] < 0 or npd:
        return
    if adef == 0:
        a = atk + sk["atk"]
        d = row[ppdef]
        a = a if a > 0 else 1
        d = d if d > 0 else 1
        v = cdiv(s32(a * a), 2 * d)
        v = cdiv(s32(v * hit), 80 if boss else 100)
        if v <= 0:
            v = 1
        v = s32(v + sum(el))
        v = cdiv(s32(v * sk["dmgRate"]), 100)
        if boss and physical:
            if v >= 10000:
                v = 9999
            v = scale(v, mag)
        else:
            v = scale(v, mag)
            if v >= 10000:
                v = 9999
    else:
        v = 0
    r.pp = v
    tgt.PP = s16(tgt.PP + v)
    if tgt.PP < row["maxPP"]:
        return
    if not boss:
        tgt.PP = row["maxPP"]
        if tgt.HP - dmg <= 0:
            return
        tgt.PPcount, tgt.PP = 300, 0
        ev.append(("effProtect", "t", 0, -1))
    else:
        tgt.PPcount, tgt.PP = 300, 0
        ev.append(("effProtect", "t", 0, 2 if att_id else 1))
    ev.append(("SetProtect", 0, "t"))


def battle_effects(att, tgt, dmg, rng, ev):
    """The attacker's equipment effects on a physical hit by a skill that
    costs no SP (the normal attack), each a
    rand() % 100 <= value roll: critical doubles, dying raises the hit to
    3/4 of the target's HP, drains take half the damage as HP or SP. A
    target with the invincible condition then ignores the hit with the
    attacker's (sic) invincible value as the chance."""
    c = att.cond
    if c[CRITICAL]:
        if rng() % 100 <= c[CRITICAL]:
            dmg = s32(dmg + dmg)
            ev.append(("ccParticleCritical", "t"))
    if c[DYING] and not tgt.type & BOSS_TYPES and tgt.ent_root:
        if rng() % 100 <= c[DYING]:
            v = cdiv(s32(tgt.HP * 3), 4)
            if dmg < v:
                dmg = v
                ev.append(("ccParticleDying", "t"))
    if c[DRAIN_HP]:
        if rng() % 100 <= c[DRAIN_HP]:
            v = cdiv(dmg + 1, 2)
            if tgt.HP < dmg:
                v = cdiv(tgt.HP + 1, 2)
            ev.append(("EntryAffect", "att", "t", 9, s16(v), 0, 0))
            ev.append(("effDrainCtrl", "att", "t", 0, 5, 5))
    if c[DRAIN_SP]:
        if rng() % 100 <= c[DRAIN_SP]:
            v = cdiv(dmg + 1, 2)
            if tgt.HP < dmg:
                v = cdiv(tgt.HP + 1, 2)
            if tgt.SP < v:
                v = tgt.SP
            ev.append(("EntryAffect", "att", "t", 10, s16(v), 0, 0))
            ev.append(("EntryAffect", "t", "att", 4, s16(v), 0, 0))
            ev.append(("effDrainCtrl", "att", "t", 1, 5, 5))
    if tgt.cond[INVINCIBLE]:
        if rng() % 100 <= c[INVINCIBLE]:
            dmg = 0
            ev.append(("ccParticleNoDamage", "t"))
    return dmg


def skill_damage(att, tgt, sk, sid, ac_flag=0, rng=None, env=None, data=None):
    """ccSkillDamage(attacker, target, skillParam, short &acFlag, sid)
    (0x00573e60) for a single-target skill (targetRange <= 0): an attribute
    critical (acFlag 1, set by _ccSkillRequest) is a sure hit (h = 100) at
    double damage, and acFlag stays -1 so later hits of the same skill are
    doubled too. The damage goes to the target as EntryAffect type 1.
    Returns (hits, acFlag, Result). Area skills loop over the target list
    with the distance test and give other targets x0.5 when the skill has
    type bit 0x8000; that part is read from the code, not modelled."""
    r = Result()
    if att.cond[DEAD] not in (0, 1) or tgt.cond[DEAD]:
        return 0, ac_flag, r
    h = -1
    ai = "ai" if getattr(att, "ai", 0) else 0     # ccSpcChar.ai, a ccAI *
    if ac_flag == 1:
        r.events.append(("ccParticleAttributeCritical", "t"))
        if att.id:
            r.events.append(("ChatMessageAttributeCritical", ai))
        h, ac_flag = 100, -1
    mag = 0x40000000 if ac_flag else F_ONE
    res = calc_battle_damage(att, tgt, sk, mag, h, rng, env, data)
    dmg = s16(res.dmg)
    res.events[:0] = r.events
    res.events.append(("EntryAffect", "t", "att", 1, dmg, s16(sid), 0))
    if (att.type & 6 and att.id and getattr(att, "party_flag", 0) and ai
            and tgt.type & (ENEMY_TYPES | BOSS_TYPES)):
        res.events.append(("ChatMessageAttack", ai, "t", dmg, sid))
    return 1, ac_flag, res


OPPOSITE = {2: 5, 5: 2, 3: 4, 4: 3, 6: 7, 7: 6}      # soil-wind, water-fire, thunder-dark


def skill_attribute(stype):
    """ccSkillAttributeCheck (0x005725a0): the skill's element as 2 (soil)
    .. 7 (dark), the last bit set winning; -1 for none."""
    v = -1
    for i in range(6):
        if stype & (4 << i):
            v = 2 + i
    return v


def check_char_attribute(ch, flag):
    """ccChar::CheckCharAttribute (0x005701d0): the element a character's
    table attributes (the saved ccSpcParam of a party member, the enemy or
    boss row) are strongest in, 2-7, or -1 on a tie at the top; flag 1
    gives the opposite element instead - the one an attribute critical
    needs."""
    t = ch.type
    if t & 4:
        at = ch.elm[8:14]
    elif t & (ENEMY_TYPES | BOSS_TYPES):
        at = ch.row["elm"][8:14]
    else:
        return -1
    best, v = at[0], 2
    for i in range(1, 6):
        if best < at[i]:
            best, v = at[i], 2 + i
        elif best == at[i]:
            v = -1
    if v < 0 or not flag:
        return v
    return OPPOSITE[v]


# ccSkillDamageValue: the AI's hit count for a physical art, by job and art
# bit (0x400, 0x800, 0x1000); spells by skill row
ART_HITS = [{0x400: 4, 0x800: 6, 0x1000: 16}, {0x400: 2, 0x1000: 4}, {0x800: 3, 0x1000: 3},
            {0x800: 2}, {0x400: 3, 0x800: 4, 0x1000: 2}, {}]
SPELL_HITS = {}
for _base, _ids in ((192, (194, 195, 196)), (224, (226, 227, 228)), (256, (258, 259, 260)),
                    (272, (274, 275, 276)), (200, (202, 203, 204)), (216, (218, 219, 220)),
                    (248, (250, 251, 252)), (280, (282, 283, 284))):
    for _i in _ids:
        SPELL_HITS[_i] = _i - _base
for _i in (197, 198, 199, 200, 209, 210, 211, 212, 229, 230, 231, 232, 241, 242, 243, 244,
           261, 262, 263, 264):
    SPELL_HITS[_i] = 9


def skill_damage_value(cp, tp, sid, data):
    """ccSkillDamageValue (0x00594e90), the party AI's estimate of what a
    skill will do: CalcBattleDamage as a sure hit (h = 0, no RNG) at x2
    when the skill's element is the one opposing the target's strongest
    (an attribute critical), times the hits the AI counts for the art or
    spell. Returns (estimate, attribute critical, attribute guard)."""
    if tp.cond[DEAD]:
        return 0, 0, 0
    mag, crit, guard = F_ONE, 0, 0
    sk = data.skills[sid]
    if cp.type & 5 and tp.type & (ENEMY_TYPES | BOSS_TYPES):
        a = skill_attribute(sk["type"])
        if a != -1:
            if a == check_char_attribute(tp, 1):
                mag, crit = 0x40000000, 1
            elif a == check_char_attribute(tp, 0):
                guard = 1
    dmg = calc_battle_damage(cp, tp, sk, mag, h=0, data=data).dmg
    t = sk["type"]
    if t & 1:
        job = getattr(cp, "job", 0)
        for bit, n in (ART_HITS[job] if 0 <= job < 6 else {}).items():   # tested in this order
            if t & bit:
                dmg = s32(dmg * n)
                break
    elif t & 2:
        dmg = s32(dmg * SPELL_HITS.get(sid, 1))
    return dmg, crit, guard


# status skills, healing, experience, Data Drain ---------------------------------

# _ccSkillModifyCondition (0x00575cf0): skill -> (what, slot, duration on an
# enemy, on a party member, skill field giving the amount, message number,
# EntryAffect type). A duration of None means the fixed value in the tuple.
COND_SKILLS = {
    156: ("cond", POISON, 5400, 5400, None, 0, 18),           # Duk Lei
    157: ("cond1", PARALYSIS, 900, 450, None, 1, 18),         # Suvi Lei
    158: ("speed", SPEED, 1800, 900, 0x3F000000, 2, 18),      # Dek Do: speed x0.5
    177: ("speed", SPEED, 9000, 9000, 0x3FE00000, 22, 17),    # Ap Do: speed x1.75
    159: ("cond1", CHARM, 600, 300, None, 3, 18),             # Miu Lei
    161: ("cond1", CONFUSION, 600, 300, None, 4, 18),         # Ranki Lei
    160: ("cond1", SLEEP, 900, 450, None, 5, 18),             # Mumyn Lei
    162: ("cond", CURSE, 5400, 5400, None, 6, 18),            # Maj Lei
    175: ("cond", REGENE_HP, 5400, 5400, None, 20, 17),       # Rig Saem
    296: ("cond", REGENE_HP, 5400, 5400, None, 20, 17),
    176: ("cond", REGENE_SP, 5400, 5400, None, 21, 17),       # Rig Geam
    297: ("cond", REGENE_SP, 5400, 5400, None, 21, 17),
}
# stat skills: (element field, skill field for the amount, debuff message)
_STAT = [(P_ATK, "atk"), (P_DEF, "atk"), (P_HIT, "hit"), (M_ATK, "atk"), (M_DEF, "atk"),
         (M_HIT, "hit")] + [(8 + i, f"attr{i}") for i in range(6)]
for _k, (_f, _src) in enumerate(_STAT):
    _deb = (163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174)[_k]
    COND_SKILLS[_deb] = ("stat", _f, 1800, 900, _src, 7 + _k, 18)
    for _buf in ((181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192)[_k],
                 (298, 299, 300, 301, 302, 303, None, None, None, None, None, None)[_k]):
        if _buf is not None:
            COND_SKILLS[_buf] = ("stat", _f, 9000, 9000, _src, 23 + _k, 17)


def modify_condition(creator, tgt, sid, data):
    """_ccSkillModifyCondition(creator, target, ID): writes the condition or
    the timed buff/debuff (temp = the skill's atk, hit or attribute value,
    time = its duration in frames) and conditionNum. Returns the
    EntryAffect calls. A paralysis, charm, confusion or sleep already
    running is not renewed: the game reports (3, -1) instead."""
    ev = []
    if tgt.cond[DEAD] or sid not in COND_SKILLS:
        return ev
    what, f, dur_foe, dur_pc, src, num, eat = COND_SKILLS[sid]
    foe = bool(tgt.type & (ENEMY_TYPES | BOSS_TYPES))
    dur = dur_foe if foe else dur_pc
    if what == "cond1" and tgt.cond[f]:
        return [("EntryAffect", "t", "creator", 3, -1, sid, 0)]
    if what in ("cond", "cond1"):
        tgt.cond[f] = dur
    elif what == "speed":
        tgt.cond[f] = dur
        tgt.speed_value = src
    else:
        sk = data.skills[sid]
        v = sk["atk"] if src == "atk" else sk["hit"] if src == "hit" else sk["attr"][int(src[4:])]
        tgt.time[f] = dur
        tgt.temp[f] = v
    tgt.condition_num = num
    ev.append(("EntryAffect", "t", "creator", eat, 0, sid, 0))
    return ev


RESIST_BODY = set(range(156, 159)) | set(range(163, 166)) | set(range(169, 175))
RESIST_SPIRIT = set(range(159, 163)) | {166, 167, 168}


def condition_success(tgt, sid, rng):
    """ccCheckConditionSkillSuccess (0x00571830): the target resists with
    rand() % 1000 + 100 < tolerance (body or spirit, from its real
    stats); 1000 or more always resists."""
    ok = 1
    tol = tgt.real[14:16]         # spirit, body
    if sid in RESIST_BODY:
        if tol[1] >= 1000:
            ok = 0
        elif rng() % 1000 + 100 < tol[1]:
            ok = 0
    if sid in RESIST_SPIRIT:
        if tol[0] >= 1000:
            ok = 0
        elif rng() % 1000 + 100 < tol[0]:
            ok = 0
    return ok


def recovery_amount(sid, target_max_hp, param=0):
    """ccSkillRecovery (0x00574bc0): the Repth family heals a fixed amount,
    the Pha ones the target's maxHP; 295 (Para Repth, the Recovery Drink)
    heals its param. None for other skills (the game uses a stale register)."""
    return {150: 150, 153: 150, 151: 400, 154: 400, 152: target_max_hp,
            155: target_max_hp, 295: param}.get(sid)


def recovery(creator, tgt, sid, param=0):
    """ccSkillRecovery(creator, target, id, param) for a single-target skill
    (targetRange <= 0): the heal is EntryAffect type 7 on the target, which
    need not be alive or listed; the caster must be alive (dead == 0).
    Returns (1 or 0, events)."""
    if creator is None or creator.cond[DEAD]:
        return 0, []
    amount = recovery_amount(sid, tgt.maxHP, param)
    return 1, [("EntryAffect", "t", "creator", 7, s16(amount), 0, 0), ("effHealSkill", "t", sid)]


def cure(creator, tgt, sid, data, count=0, stype=0, annihilated=False, anm_flag=1):
    """ccSkill::RecoverySystem (0x00577070) for Rip Teyn (178, the
    Antidote), Rip Synk (179, the Restorative) and Rip Maen (180, the
    Resurrect): 178 clears poison, paralysis, slowness and every lowered
    physical and elemental stat; 179 clears charm, confusion, sleep, curse
    and lowered magic stats; 180 revives with full HP and no SP unless the
    whole party is down. The caster then pays the SP cost. Returns
    (events, endFlag set)."""
    ev = []
    if tgt.cond[DEAD] and sid != 180:
        return ev, False
    if count == 0:
        n = 0
        c = tgt.cond
        if sid == 178:
            for i in (POISON, PARALYSIS):
                if c[i]:
                    c[i] = 0
                    n += 1
            if eemu.f_cmp(tgt.speed_value, F_ONE) < 0:
                c[SPEED] = 0
                tgt.speed_value = F_ONE
                n += 1
            fields = (0, 1, 2, 8, 9, 10, 11, 12, 13)
        elif sid == 179:
            for i in (CHARM, CONFUSION, SLEEP, CURSE):
                if c[i]:
                    c[i] = 0
                    n += 1
            fields = (4, 5, 6)
        else:
            fields = ()
        for i in fields:
            if tgt.temp[i] < 0:
                tgt.temp[i] = tgt.time[i] = 0
                n += 1
        if sid == 178:
            ev.append(("effCure", "t"))
        elif sid == 179:
            ev.append(("effSanity", "t"))
        if sid in (178, 179) and n > 0:
            ev.append(("EntryAffect", "t", "creator", 16, 0, sid, 0))
        if sid == 180 and not annihilated:
            tgt.HP, tgt.SP = tgt.maxHP, 0
            ev.append(("effResurrect", "t"))
            ev.append(("EntryAffect", "t", "creator", 20, 0, sid, 0))
        if creator is not None and creator.type & 5 and stype == 0:
            v = creator.SP - data.skills[sid]["cost"]
            creator.SP = s16(v) if v >= 0 else 0
    end = True
    if creator is not None and stype in (0, 2) and creator.type & 5:
        if not anm_flag:
            return ev, False
        creator.skill_id = creator.skill_status = 0
    return ev, end


def exp_for_kill(data, enemy_level, pc_level, in_party=True):
    """ccExpDistributor (0x00571290): expCalcTbl by the level difference,
    clamped to -10..10; members not in the party but flagged in
    partyMemberFlag and partyMemberExp get 60%. The enemy's own exp field is
    not used."""
    if pc_level >= 99:
        return 0
    d = max(-10, min(10, enemy_level - pc_level))
    e = data.exp_calc[d + 10]
    return e if in_party else cdiv(e * 60, 100)


def add_lv_erosion(data, erosion, n, f_bits):
    """ccSaveData::AddLvErosion (main 0x001787a0): the infection a Data
    Drain adds, erosionTbl[level difference + 5] times the drain's factor,
    capped at 100. n is target level - Kite's level."""
    n = max(-5, min(10, n))
    add = s16(fptosi(eemu.f_mul(f_bits, eemu.f_from_int(data.erosion[n + 5] & M32))))
    erosion = s16(erosion + add)
    return 100 if erosion >= 101 else erosion


# DataDrainMenu (0x00533010): the multiplier by drain skill
DRAIN_FACTOR = {2: 0x3F800000, 3: 0x3FC00000, 4: 0x3FC00000, 5: 0x40400000}


def drain_drop(row, erosion, sid, rng, boss=False):
    """The item a Data Drain gives (ccMenuCtrl::DataDrainMenu 0x00533100,
    read from the code, not run): a boss always gives item[0]; for an enemy
    the roll is rand() % 100 + 60 for 2128 Drain and Drain Heart, else
    rand() % 100 + erosion / 2 (erosion already raised by this drain):
    96 and up item[2], 50 and up item[1], else item[0]."""
    if boss:
        return row["item"][0], None
    if sid >= 4:
        roll = rng() % 100 + 60
    else:
        roll = rng() % 100 + cdiv(erosion, 2)
    return row["item"][2 if roll >= 96 else 1 if roll >= 50 else 0], roll


# The side effect a Data Drain can bring (DataDrainMenu 0x00533534 on, read
# from the code): after the drain, rand() % 100 <= erosion / 2 triggers one,
# picked as dataDrainErosionTbl[erosion / 25][rand() & 15]; the frames of
# warning in between draw from the same RNG, so the sequence is not a pure
# function of the seed. Resistible ones compare rand() % 1001 with the
# victim's real tolerance (body or spirit): below it, a miss.
DRAIN_EFFECTS = {0: "party fully healed (HP and SP)"}
for _i, _f in enumerate(("pAtk", "pDef", "pHit", "mAtk", "mDef", "mHit")):
    DRAIN_EFFECTS[1 + _i] = f"Kite's {_f} -20 for 900 frames"
for _i, (_c, _t, _r) in enumerate((("poisoned", 5400, "body"), ("paralysed", 450, "body"),
                                   ("slowed to x0.5", 900, "body"), ("charmed", 300, "spirit"),
                                   ("confused", 300, "spirit"), ("asleep", 450, "spirit"),
                                   ("cursed", 5400, "spirit"))):
    DRAIN_EFFECTS[7 + _i] = f"Kite {_c} for {_t} frames unless rand() % 1001 < {_r}"
    DRAIN_EFFECTS[14 + _i] = f"every living member {_c} for {_t} frames, each rolling against {_r}"
DRAIN_EFFECTS.update({
    21: "every living member's HP halved (rounded up)",
    22: "every living member's SP halved (rounded up)",
    28: "every living member left with 1 HP and 1 SP",
    29: "one item stack lost (searched from a random slot 0-19; a key item, category 15, stops it)",
    30: "nothing"})
for _i, _e in enumerate((200, 400, 600, 800, 1000)):
    DRAIN_EFFECTS[23 + _i] = (f"Kite loses {_e} exp; below 0 at level 2 or more he gets 1000 back "
                              "and loses a level (LevelDown)")


def drain_side_effect_chance(erosion):
    """The chance a drain brings a side effect, and the effect table row."""
    return (cdiv(erosion, 2) + 1) / 100, cdiv(erosion, 25)


# the command line ------------------------------------------------------------------

def skill_kind(t):
    """_ccSkillCheckType (0x00573c30)."""
    if t & 1:
        return "attack" if not t & 0x1C00 else "art"
    if t & 2:
        return "heal" if t & 0x40000 else "buff" if t & 0x10000 else "debuff" if t & 0x20000 else "spell"
    return "-"


CURES = {178: "cures poison, paralysis, slowness, lowered pAtk/pDef/pHit and elements",
         179: "cures charm, confusion, sleep, curse, lowered mAtk/mDef/mHit",
         180: "revives with full HP and 0 SP (not when the whole party is down)"}


def skill_effect(data, sid, sk):
    def dur(a, b):
        return f"{a} frames" if a == b else f"{a} frames on a foe, {b} on the party"
    if sid in COND_SKILLS:
        what, f, dfoe, dpc, src, num, _ = COND_SKILLS[sid]
        if what == "stat":
            v = sk["atk"] if src == "atk" else sk["hit"] if src == "hit" else sk["attr"][int(src[4:])]
            return f"{ELM[f]} {v:+d} for {dur(dfoe, dpc)}"
        if what == "speed":
            return f"speed x{eemu.f_to_py(src):g} for {dur(dfoe, dpc)}"
        return f"{COND[f]} for {dur(dfoe, dpc)}" + (
            "; resisted by body" if sid in RESIST_BODY else
            "; resisted by spirit" if sid in RESIST_SPIRIT else "")
    if sid in CURES:
        return CURES[sid]
    amount = recovery_amount(sid, "the target's max", "the item's amount of")
    if amount is not None:
        return f"heals {amount} HP"
    if sid in DRAIN_FACTOR:
        return f"Data Drain, infection x{eemu.f_to_py(DRAIN_FACTOR[sid]):g}"
    if sid in SPELL_HITS:
        return f"the AI counts {SPELL_HITS[sid]} hits"
    return ""


def cmd_skills(data, show_all):
    print(f"{'#':>3} {'name':16} {'kind':6} {'elem':7} {'atk':>4} {'hit':>4} {'rate':>4} "
          f"{'cost':>4} {'lv':>2} {'trig':>5} {'range':>5} {'tgt':>5}  flags | effect")
    for i, sk in enumerate(data.skills):
        t = sk["type"]
        if not show_all and not t:
            continue
        elem = ",".join(n for b, n in SKILL_BITS[2:8] if t & b) or "-"
        flags = " ".join(n for b, n in SKILL_BITS if t & b and n not in AT
                         and n not in ("physical", "magic"))
        eff = skill_effect(data, i, sk)
        tt = sk["targetType"]
        tgt = "foe" if tt & 0xE0 and not tt & 7 else "party" if tt & 7 and not tt & 0xE0 else f"{tt:x}"
        print(f"{i:3} {sk['name'][:16]:16} {skill_kind(t):6} {elem:7} {sk['atk']:4} {sk['hit']:4} "
              f"{sk['dmgRate']:4} {sk['cost']:4} {sk['level']:2} {sk['triggerRange']:5.0f} "
              f"{sk['targetRange']:5.0f} {tgt:>5}  {flags}"
              + (f" | {eff}" if eff else ""))


def fmt_elm(e):
    return (" ".join(f"{k} {v}" for k, v in zip(BA, e[:8])) + "\n  " +
            " ".join(f"{k} {v}" for k, v in zip(AT + TO, e[8:])))


def cmd_stats(data, key, level, equip):
    pc = make_pc(data, key, level, equip)
    print(f"{pc.name} (charTbl {data.char(key)['index']}, job {pc.job}) level {pc.level}: "
          f"HP {pc.maxHP} SP {pc.maxSP}")
    print("base (elm):\n  " + fmt_elm(pc.elm))
    for slot in ("head", "body", "arm", "leg", "weapon"):
        eq = pc.equipment(slot)
        nz = ", ".join(f"{k} {v:+d}" for k, v in zip(ELM, eq["elm"]) if v)
        bf = ", ".join(f"{k} {v}" for k, v in zip(BEFF, eq["beff"]) if v)
        print(f"{slot:6} {eq['index']:3} {eq['name']:20} {nz}" + (f"  [{bf}]" if bf else ""))
    print("effective (real):\n  " + fmt_elm(pc.real))
    print("battle effects: " + (", ".join(f"{COND[DRAIN_HP + i]} {pc.cond[DRAIN_HP + i]}"
                                          for i in range(5) if pc.cond[DRAIN_HP + i]) or "none"))


def cmd_levelup(data, key, to):
    pc = PC(data, data.char(key))
    t = data.levelup[pc.id]
    print(f"{pc.name}: LevelUpParamTbl[{pc.id}] adds maxHP {t[0]:+d} maxSP {t[1]:+d} " +
          " ".join(f"{k} {v:+d}" for k, v in zip(ELM, t[2:]) if v) + " per level; 1000 exp a level")
    cols = ("maxHP", "maxSP") + ELM
    print(f"{'lv':>3} " + " ".join(f"{c[:6]:>6}" for c in cols))

    def row():
        vals = [pc.p_maxHP, pc.p_maxSP] + pc.elm
        print(f"{pc.level:3} " + " ".join(f"{v:6}" for v in vals))
    row()
    while pc.level < min(to, 99):
        pc.exp = 1000
        check_level_up(pc)
        row()


def resolve_fighter(data, key, level, equip):
    if key.startswith("boss:"):
        return Foe(data.find(data.bosses, key[5:], "boss"), "boss")
    if key.startswith("enemy:"):
        return Foe(data.find(data.enemies, key[6:], "enemy"), "enemy")
    try:
        data.char(key)
        return make_pc(data, key, level, equip)
    except SystemExit:
        return Foe(data.find(data.enemies, key, "enemy or party member"), "enemy")


def describe(f):
    if isinstance(f, PC):
        return f"{f.name} (party, level {f.level})"
    return f"{f.name} ({f.kind} row {f.row['index']}, level {f.level}, HP {f.maxHP})"


def cmd_damage(data, args):
    att = resolve_fighter(data, args.attacker, args.level, args.equip)
    tgt = resolve_fighter(data, args.target, args.target_level, [])
    sk = data.skill(args.skill)
    mag = eemu.f_from_py(args.mag)
    env = Env(plcol=int(args.plcol))
    t = sk["type"]
    print(f"{describe(att)} -> {describe(tgt)}")
    print(f"skill {sk['index']} {sk['name']}: {skill_kind(t)}, atk {sk['atk']:+d} hit {sk['hit']:+d} "
          f"rate {sk['dmgRate']}%" + (f", x{args.mag:g}" if args.mag != 1 else ""))
    if not t & 3:
        print("not a damage skill (CalcBattleDamage returns -1)")
        return
    phys = bool(t & 1)
    ATK, DEF, HIT, EVA = (P_ATK, P_DEF, P_HIT, P_EVA) if phys else (M_ATK, M_DEF, M_HIT, M_EVA)
    print(f"attacker {BA[ATK]} {att.real[ATK]} {BA[HIT]} {att.real[HIT]}; "
          f"target {BA[DEF]} {tgt.real[DEF]} {BA[EVA]} {tgt.real[EVA]}")
    adj = cdiv(att.real[HIT] + sk["hit"] - cdiv(2 * tgt.real[EVA], 3), 10)
    print(f"roll r = rand() % 101: r < 5 misses, r >= 95 hits at 100%, otherwise "
          f"hit = r {adj:+d}, a miss below 50")
    outcomes = [calc_battle_damage(att, tgt, sk, mag, roll=roll, env=Env(plcol=0), data=data)
                for roll in range(101)]
    hits = [(i, o) for i, o in enumerate(outcomes) if o.dmg >= 0]
    print(f"hit chance {len(hits)}/101 = {100 * len(hits) / 101:.1f}%")
    if hits:
        ds = [o.dmg for _, o in hits]
        print(f"damage on a hit: min {min(ds)} max {max(ds)} mean {sum(ds) / len(ds):.2f}; "
              f"expected per use {sum(ds) / 101:.2f}")
        if outcomes[-1].recover:
            print(f"the target absorbs {outcomes[-1].recover} HP (attribute 999)")
    c = att.cond
    if phys and any(c[DRAIN_HP:INVINCIBLE + 1]):
        print("equipment effects (rand() % 100 <= value): " +
              ", ".join(f"{COND[i]} {min(c[i] + 1, 100)}%"
                        for i in range(DRAIN_HP, INVINCIBLE + 1) if c[i]))
    if args.seed is not None:
        rng = Rand(args.seed)
        res = calc_battle_damage(att, tgt, sk, mag, h=-1, rng=rng, env=env, data=data)
        pp = ""
        if res.pp is not None:
            pp = f", protect +{res.pp} (PP {tgt.PP}/{tgt.row['maxPP']}"
            pp += ", broken for 300 frames)" if tgt.PPcount == 300 else ")"
        print(f"seed {args.seed}: roll {res.roll}, hit {res.hit}, "
              + (f"damage {res.dmg}" if res.dmg >= 0 else "miss") + pp
              + f"; {rng.count} rand() calls, state 0x{rng.state:016x}")
        for e in res.events:
            print(f"  {e}")


def cmd_drain(data, args):
    foe = resolve_fighter(data, args.target, None, [])
    if isinstance(foe, PC):
        raise SystemExit("drain an enemy or a boss")
    sk = data.skill(args.skill)
    sid = sk["index"]
    if sid not in DRAIN_FACTOR:
        raise SystemExit("--skill: Data Drain, Drain Arc, 2128 Drain or Drain Heart")
    kite = args.kite_level
    after = add_lv_erosion(data, args.erosion, foe.level - kite, DRAIN_FACTOR[sid])
    print(f"{sk['name']} on {describe(foe)}, Kite level {kite}, infection {args.erosion}%")
    n = max(-5, min(10, foe.level - kite)) + 5
    print(f"infection: +{after - args.erosion} -> {after}% (erosionTbl[{n}]"
          f" x {eemu.f_to_py(DRAIN_FACTOR[sid]):g})")
    items = foe.row["item"]
    if foe.kind == "boss":
        print(f"drop: item[0] {data.item_name(items[0])} always")
    else:
        base = 60 if sid >= 4 else cdiv(after, 2)
        p = [0, 0, 0]
        for r in range(100):
            roll = r + base
            p[2 if roll >= 96 else 1 if roll >= 50 else 0] += 1
        print(f"drop: roll = rand() % 100 + {base}: " +
              ", ".join(f"{data.item_name(items[i])} {p[i]}%" for i in range(3)))
    chance, row = drain_side_effect_chance(after)
    print(f"side effect: {100 * chance:.0f}% (rand() % 100 <= {cdiv(after, 2)}), "
          f"dataDrainErosionTbl[{row}]:")
    for e in sorted(set(data.drain_effects[row])):
        n = data.drain_effects[row].count(e)
        print(f"  {n:2}/16  {e:2}  {DRAIN_EFFECTS[e]}")


def cmd_exp(data, pc_level):
    print(f"exp per kill for a level-{pc_level} member (1000 exp a level); "
          f"out-of-party members flagged for exp get 60%")
    for d in range(-10, 11):
        lv = pc_level + d
        if lv < 0:
            continue
        print(f"  enemy level {lv:3} ({d:+3d}): {exp_for_kill(data, lv, pc_level):4} "
              f"/ {exp_for_kill(data, lv, pc_level, False):4}")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("stats")
    p.add_argument("elf")
    p.add_argument("char")
    p.add_argument("--level", type=int)
    p.add_argument("--equip", action="append", default=[], metavar="SLOT=NAME")
    p = sub.add_parser("damage")
    p.add_argument("elf")
    p.add_argument("attacker")
    p.add_argument("target")
    p.add_argument("--skill", default="ATTACK")
    p.add_argument("--seed", type=lambda s: int(s, 0))
    p.add_argument("--level", type=int, help="the attacker's level, for a party member")
    p.add_argument("--target-level", type=int)
    p.add_argument("--equip", action="append", default=[], metavar="SLOT=NAME")
    p.add_argument("--mag", type=float, default=1.0,
                   help="damage multiplier: 2 for an attribute critical, 0.5 for splash")
    p.add_argument("--plcol", action="store_true",
                   help="saveData.plcol set (the protect gauge only fills then)")
    p = sub.add_parser("levelup")
    p.add_argument("elf")
    p.add_argument("char")
    p.add_argument("--to", type=int, default=99)
    p = sub.add_parser("skills")
    p.add_argument("elf")
    p.add_argument("--all", action="store_true", help="also rows with no type")
    p = sub.add_parser("exp")
    p.add_argument("elf")
    p.add_argument("--pc-level", type=int, default=1)
    p = sub.add_parser("drain")
    p.add_argument("elf")
    p.add_argument("target")
    p.add_argument("--skill", default="Data Drain")
    p.add_argument("--erosion", type=int, default=0, help="the infection before the drain, 0-100")
    p.add_argument("--kite-level", type=int, default=1)
    args = parser.parse_args()

    data = Data(args.elf)
    if args.cmd == "stats":
        cmd_stats(data, args.char, args.level, args.equip)
    elif args.cmd == "damage":
        cmd_damage(data, args)
    elif args.cmd == "levelup":
        cmd_levelup(data, args.char, args.to)
    elif args.cmd == "skills":
        cmd_skills(data, args.all)
    elif args.cmd == "exp":
        cmd_exp(data, args.pc_level)
    elif args.cmd == "drain":
        cmd_drain(data, args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
