//! A fighter's battle state (`ccChar` and what it points at) and the
//! per-character rules of `common.cpp`: `CalcReal`, `ConditionTimeCount`,
//! `ConditionBattleEffect`, levels (`CheckLevelUp`, `LevelDown`,
//! `ccSetLevelParam`) and `CheckCharAttribute`. A [`Char`] keeps the `ccChar`
//! members the rules use and a [`Body`] (a party member's `ccSpcParam`, an
//! enemy's or boss's row, or an object with no stats); which one a rule treats
//! it as is decided, as in the game, by `base.type`.

use crate::event::{Event, Events, Who};
use crate::param::*;
use crate::tables::Tables;
use piney_data::volume::Volume;

/// An enemy's or boss's own state: its table row (as the game reads it
/// through `ccChar.base`) and `ccEnemyParam` / `ccBossParam`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Foe {
    pub row: FoeRow,
    pub real: Elm,
    pub temp: Elm,
    pub time: Elm,
    /// The protect gauge and its break's frames left.
    pub pp: i16,
    pub pp_count: i16,
    /// `ccBossParam.PPrestore`: how many breaks a boss has recovered from.
    pub pp_restore: i16,
    /// A boss's own state (`ccBoss` and its class), whose `Affect` an
    /// affect on it runs ([`AffectFunc::Boss`]).
    pub boss: Option<Box<crate::boss::Boss>>,
}

impl Foe {
    /// A fresh foe from its row, as `SetBaseParam` and the spawn leave it:
    /// `real` the row's stats, no buffs, an empty gauge.
    pub fn new(row: FoeRow) -> Foe {
        Foe { real: row.elm, row, ..Foe::default() }
    }
}

/// What a `ccChar`'s `base` and `personality` point at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// A party member: `ccSpcParam` is both.
    Spc(Box<SpcParam>),
    /// An enemy or boss.
    Foe(Box<Foe>),
    /// Anything else (a trap, a gimmick): only the base.
    Other { base: Base, real: Elm },
}

/// A fighter: `ccChar` (and the `ccSpcChar` members battle reads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Char {
    pub cond: Condition,
    pub condition_num: i32,
    pub hp: i16,
    pub sp: i16,
    pub max_hp: i16,
    pub max_sp: i16,
    pub skill_id: i16,
    pub skill_status: i16,
    /// `ccSpcChar.noDeathFlag`.
    pub no_death: bool,
    /// `ccSpcChar.partyFlag` (3 bits).
    pub party_flag: i32,
    /// Whether `ccSpcChar.ai` is set (a party member the AI drives).
    pub has_ai: bool,
    /// `ccSpcChar.anmFlag`.
    pub anm_flag: i16,
    /// The word at +0x140: `weaponPos[0]` on a party member,
    /// `entParam.entRoot` on an enemy. "Dying" needs it non-zero.
    pub ent_root: u32,
    /// `ccChar.pos` and `posP` (+0x40, +0x50): where it stands, as float
    /// bits; area rules measure from `posP`, the position in the player's
    /// frame.
    pub pos: [u32; 4],
    pub pos_p: [u32; 4],
    /// `ccChar.targetChar` (+0x78): whom the character's skill aims at; a
    /// party member hit by a foe turns to it.
    pub target_char: Option<usize>,
    /// How [`crate::affect`] applies an affect to this character, and what
    /// the last one left (`affectType`, `affectParam`, `affectPerson`).
    pub affect: AffectState,
    /// The `ccSpcChar` (and `ccFellow`, `ccEnemy`) members the affects
    /// read and write.
    pub spc_char: SpcChar,
    pub body: Body,
}

/// Which `affectFunc` a character has: `ccEnemyInfluence` (enemies),
/// `ccFellow::Influence` (party members the AI drives), `Influence`
/// (Kite, player.cpp), `ccGimmickAffect` (the gimmicks: the object's
/// `affectFlag`), or none (the affect is ignored).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AffectFunc {
    #[default]
    None,
    Enemy,
    Fellow,
    Player,
    Gimmick,
    /// `bossAffectFunc` (gcmn 0x0045f090): the boss's `Affect`
    /// ([`crate::boss::entry`]).
    Boss,
}

/// The affect members of `ccChar` (+0x94-+0xb8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AffectState {
    pub func: AffectFunc,
    pub person: Option<usize>,
    /// The kind of the last affect taken; a drained foe (13) takes no more.
    pub ty: i16,
    pub param: [i16; 3],
    /// `affectMask`: a bit per kind the character ignores.
    pub mask: i32,
    /// The flash an affect starts: `affectColorCnt`, `affectColorRate`,
    /// `affectColor`, and the condition tint it resets
    /// (`conditionColorCnt`, `conditionColorRate`, `conditionColor`).
    pub color_cnt: i16,
    pub color_rate: i16,
    pub color: u32,
    pub cond_color_cnt: i16,
    pub cond_color_rate: i16,
    pub cond_color: u32,
}

/// `ccSpcChar` members (+0xe0 on): the flag word (besides `noDeathFlag`
/// and `partyFlag`, which are [`Char::no_death`] and [`Char::party_flag`];
/// on an enemy the byte is `ccEntryObj`'s flags), the act the animation
/// plays, and the counters death and revival set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpcChar {
    /// The word at +0xe0 without bits 7 and 14-16.
    pub flags: u32,
    /// `armsEffectSW`.
    pub arms_effect_sw: i32,
    /// `actNum`: the act (animation) the character is in; 7 and 8 are
    /// hurt, 9 and 10 down, 2 getting up.
    pub act_num: i16,
    /// `actNumOld`.
    pub act_num_old: i16,
    pub attack: i16,
    /// `cnt` and `cloak`.
    pub cnt: i32,
    pub cloak: u32,
    /// `bodyHit` enabled (`ccCharHit::HitEnable`/`HitDisable`).
    pub hit_enabled: bool,
    /// `ccFellow`'s flag byte (+0x200): bit 1 turned to its attacker.
    pub fellow_flags: u8,
    /// `ccEnemy`'s flag word (+0x250): targetFlag, actionFlag,
    /// drainFlag (bit 2), moveFlag (2 bits), madFlag, virusFlag (bit 6),
    /// optFlag0, optFlag1.
    pub enemy_flags: u16,
    /// `CheckSysMsgID()`: the AI's system-message id, -1 without one.
    pub sys_msg_id: i16,
}

/// Bits of [`SpcChar::flags`].
pub mod spc_flag {
    pub const PAUSE: u32 = 1 << 0;
    pub const RESTRAINT: u32 = 1 << 2;
    pub const MOVE: u32 = 1 << 3;
    pub const STOP: u32 = 1 << 4;
    /// `ghostFlag`: a party member down, see-through.
    pub const GHOST: u32 = 1 << 6;
    /// `ccEntryObj.affectFlag` on an enemy.
    pub const AFFECT: u32 = 1 << 3;
    /// `trajectorySW` (+0xe1 bit 3).
    pub const TRAJECTORY: u32 = 1 << 11;
}

/// Bits of [`SpcChar::enemy_flags`].
pub mod enemy_flag {
    pub const DRAIN: u16 = 1 << 2;
    pub const VIRUS: u16 = 1 << 6;
}

impl Char {
    fn with_body(body: Body) -> Char {
        Char {
            cond: Condition::default(),
            condition_num: 0,
            hp: 0,
            sp: 0,
            max_hp: 0,
            max_sp: 0,
            skill_id: 0,
            skill_status: 0,
            no_death: false,
            party_flag: 0,
            has_ai: false,
            anm_flag: 0,
            ent_root: 1,
            pos: [0, 0, 0, F_ONE],
            pos_p: [0, 0, 0, F_ONE],
            target_char: None,
            affect: AffectState::default(),
            spc_char: SpcChar { sys_msg_id: -1, hit_enabled: true, ..SpcChar::default() },
            body,
        }
    }

    /// A party member from its record (`SetBaseParam(ccSpcParam *)` then
    /// full HP and SP).
    pub fn pc(p: SpcParam) -> Char {
        let mut c = Char::with_body(Body::Spc(Box::new(p)));
        c.max_hp = p.max_hp;
        c.max_sp = p.max_sp;
        c.hp = p.max_hp;
        c.sp = p.max_sp;
        c
    }

    /// An enemy or boss from its table row, at full HP and SP.
    pub fn foe(row: FoeRow) -> Char {
        let (hp, sp) = (row.max_hp, row.max_sp);
        let mut c = Char::with_body(Body::Foe(Box::new(Foe::new(row))));
        c.max_hp = hp;
        c.max_sp = sp;
        c.hp = hp;
        c.sp = sp;
        c
    }

    /// Something else with a base (a trap's `ccCharBaseParam`).
    pub fn other(base: Base, real: Elm) -> Char {
        Char::with_body(Body::Other { base, real })
    }

    pub fn base(&self) -> &Base {
        match &self.body {
            Body::Spc(p) => &p.base,
            Body::Foe(f) => &f.row.base,
            Body::Other { base, .. } => base,
        }
    }

    pub fn base_mut(&mut self) -> &mut Base {
        match &mut self.body {
            Body::Spc(p) => &mut p.base,
            Body::Foe(f) => &mut f.row.base,
            Body::Other { base, .. } => base,
        }
    }

    /// `base->type`.
    pub fn ty(&self) -> i32 {
        self.base().ty
    }

    pub fn id(&self) -> i16 {
        self.base().id
    }

    pub fn level(&self) -> i16 {
        self.base().level
    }

    pub fn is_pc(&self) -> bool {
        self.ty() & ty::PC != 0
    }

    pub fn is_enemy(&self) -> bool {
        self.ty() & ty::ENEMY != 0
    }

    pub fn is_boss(&self) -> bool {
        self.ty() & ty::BOSS != 0
    }

    pub fn is_foe(&self) -> bool {
        self.ty() & ty::FOE != 0
    }

    pub fn spc(&self) -> Option<&SpcParam> {
        match &self.body {
            Body::Spc(p) => Some(p),
            _ => None,
        }
    }

    pub fn spc_mut(&mut self) -> Option<&mut SpcParam> {
        match &mut self.body {
            Body::Spc(p) => Some(p),
            _ => None,
        }
    }

    pub fn foe_state(&self) -> Option<&Foe> {
        match &self.body {
            Body::Foe(f) => Some(f),
            _ => None,
        }
    }

    pub fn foe_state_mut(&mut self) -> Option<&mut Foe> {
        match &mut self.body {
            Body::Foe(f) => Some(f),
            _ => None,
        }
    }

    /// The `real` block: the party member's `ccSpcParam.real`, the foe's
    /// `ccEnemyParam.real`, or an object's.
    pub fn real(&self) -> &Elm {
        match &self.body {
            Body::Spc(p) => &p.real,
            Body::Foe(f) => &f.real,
            Body::Other { real, .. } => real,
        }
    }

    pub fn real_mut(&mut self) -> &mut Elm {
        match &mut self.body {
            Body::Spc(p) => &mut p.real,
            Body::Foe(f) => &mut f.real,
            Body::Other { real, .. } => real,
        }
    }

    /// The `temp` and `time` blocks (none on an object).
    pub fn temp_time_mut(&mut self) -> Option<(&mut Elm, &mut Elm)> {
        match &mut self.body {
            Body::Spc(p) => Some((&mut p.temp, &mut p.time)),
            Body::Foe(f) => Some((&mut f.temp, &mut f.time)),
            Body::Other { .. } => None,
        }
    }

    pub fn temp_time(&self) -> Option<(&Elm, &Elm)> {
        match &self.body {
            Body::Spc(p) => Some((&p.temp, &p.time)),
            Body::Foe(f) => Some((&f.temp, &f.time)),
            Body::Other { .. } => None,
        }
    }

    pub fn dead(&self) -> bool {
        self.cond[cond::DEAD] != 0
    }
}

/// The globals the rules consult.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Env {
    /// `saveData.plcol` (+0x6771): the protect gauge only fills while set.
    pub plcol: u8,
    /// `ccMenuCtrl.forbid` (+0xfe): no protect damage while set.
    pub menu_forbid: i16,
    /// `ccGame.inBattle`.
    pub in_battle: i32,
    /// `ccSystem.count` (+0x358), the frame counter.
    pub count: u32,
    /// `ccMenuCtrl::CheckMenuType()`, -1 with no menu open.
    pub menu_type: i32,
    /// `ccSpcChar::CheckSpRegeneSpeed()` for the character updated.
    pub sp_regene_speed: bool,
    /// `ccGame.area`: 0 is the Root Town, where no condition effect shows.
    pub area: i32,
}

impl Default for Env {
    fn default() -> Self {
        Env { plcol: 1, menu_forbid: 0, in_battle: 1, count: 0, menu_type: -1, sp_regene_speed: false, area: 1 }
    }
}

/// C division, truncating toward zero (no trap on zero: callers guard).
pub(crate) fn cdiv(a: i32, b: i32) -> i32 {
    a.wrapping_div(b)
}

pub(crate) fn cmod(a: i32, b: i32) -> i32 {
    a.wrapping_rem(b)
}

/// `ccAddParamElement(d, s, hi, lo)` (gcmn 0x00570440): each field wraps to
/// a short, then is clamped.
pub fn add_elm(d: &mut Elm, s: &Elm, hi: i32, lo: i32) {
    for (x, y) in d.iter_mut().zip(s) {
        let v = i32::from(x.wrapping_add(*y));
        *x = if v > hi {
            hi as i16
        } else if v < lo {
            lo as i16
        } else {
            v as i16
        };
    }
}

/// `ccMulBattleAbility` / `ccMulAttribute`: `d += s * n`, clamped 0..999.
fn mul_add_elm(d: &mut Elm, s: &Elm, n: i32) {
    for (x, y) in d.iter_mut().zip(s) {
        let v = i32::from(x.wrapping_add(i32::from(*y).wrapping_mul(n) as i16));
        *x = if v >= 1000 {
            999
        } else if v < 0 {
            0
        } else {
            v as i16
        };
    }
}

const ZERO_ELM: Elm = [0; 16];
const ZERO_BEFF: Beff = [0; 5];

/// A party member's equipped piece in a slot (head body arm leg weapon),
/// its `elm` and `beff`; zeros for a row past the table's end (the game
/// reads the next table there) or a job outside 0-5 (the game reads
/// through a null pointer).
fn piece<'a>(t: &'a Tables, p: &SpcParam, s: usize) -> (&'a Elm, &'a Beff) {
    let i = p.equipment[s];
    let e = if s == slot::WEAPON {
        t.job_weapon(i32::from(p.job), i32::from(i))
    } else {
        let k = [0, 1, 2, 3][s];
        usize::try_from(i).ok().and_then(|i| t.armor[k].get(i))
    };
    match e {
        Some(e) => (&e.elm, &e.beff),
        None => (&ZERO_ELM, &ZERO_BEFF),
    }
}

/// `ccChar::CalcReal(flag)` (gcmn 0x0056ba30): effective stats. A party
/// member's `real` is `elm` plus the equipment's `tune` plus `temp`, each
/// addition clamped to +-999; a foe's is its row's `elm` plus `temp`, held to
/// 0..32767. `flag == 0` (once a frame) first runs [`condition_time_count`]
/// and, for a foe, counts a protect break down; `flag <= 0` outside the Root
/// Town then shows the condition effect (docs/engine/battle.md).
pub fn calc_real(t: &Tables, ch: &mut Char, flag: i32, env: &Env, ev: &mut Events) {
    let tyb = ch.ty();
    if tyb & ty::PC != 0 {
        let Body::Spc(p) = &mut ch.body else { return };
        p.real = p.elm;
        p.tune = *piece(t, p, slot::HEAD).0;
        for s in [slot::BODY, slot::LEG, slot::ARM, slot::WEAPON] {
            let e = *piece(t, p, s).0;
            add_elm(&mut p.tune, &e, 999, -999);
        }
        let tune = p.tune;
        add_elm(&mut p.real, &tune, 999, -999);
        ch.max_hp = p.max_hp;
        ch.max_sp = p.max_sp;
        if flag == 0 {
            condition_time_count(ch, env, ev);
        }
        condition_battle_effect(t, ch);
        if let Body::Spc(p) = &mut ch.body {
            let temp = p.temp;
            add_elm(&mut p.real, &temp, 999, -999);
        }
    } else if tyb & ty::FOE != 0 {
        let enemy = tyb & ty::ENEMY != 0;
        let Body::Foe(f) = &mut ch.body else { return };
        f.real = f.row.elm;
        if enemy {
            ch.max_hp = f.row.max_hp;
            ch.max_sp = f.row.max_sp;
        }
        if flag == 0 {
            condition_time_count(ch, env, ev);
        }
        condition_battle_effect(t, ch);
        let Body::Foe(f) = &mut ch.body else { return };
        let temp = f.temp;
        add_elm(&mut f.real, &temp, 32767, 0);
        if flag == 0 && f.pp_count != 0 && (enemy || f.pp >= 0) {
            f.pp_count = f.pp_count.wrapping_sub(1);
            if f.pp_count <= 0 {
                f.pp_count = 0;
                if enemy {
                    f.pp = (i32::from(f.row.max_pp) / 2) as i16;
                    ev.push(Event::Protect { on: Who::Me, broken: 1, kind: -1 });
                } else {
                    f.pp_restore = f.pp_restore.wrapping_add(1);
                    if f.pp_restore >= 2 {
                        f.pp_restore = 2;
                        f.pp = cdiv(i32::from(f.row.max_pp) * 3, 4) as i16;
                    } else {
                        f.pp = (i32::from(f.row.max_pp) / 2) as i16;
                    }
                    let kind = if f.row.base.id != 0 { 2 } else { 1 };
                    ev.push(Event::Protect { on: Who::Me, broken: 1, kind });
                }
                ev.push(Event::SetProtect { state: 1, on: Who::Me });
            }
        }
    } else {
        return;
    }
    if flag <= 0 && env.area != 0 {
        ev.push(Event::DispCondition(Who::Me));
    }
}

/// What `ccChar::DispConditionEffect` does with the character's
/// condition effect (`ccChar` +0x2c, a `ccConditionEffect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CondFx {
    /// Nothing.
    Keep,
    /// `killConditionEffect(ep)`: it fades out.
    Kill,
    /// `deleteConditionEffect(ep)`: it ends at once.
    Delete,
    /// `setConditionEffect(ch)` with [`Char::condition_num`]; an effect
    /// the character had is left running, unheld (the game overwrites the
    /// pointer when the number was -1 already).
    Set,
    /// `killConditionEffect(ep)`, then `setConditionEffect(ch)`.
    Replace,
}

/// `ccChar::DispConditionEffect()` (gcmn 0x0056f950): which condition the
/// character shows (`conditionNum`, +0x30), by priority (paralysis, sleep,
/// confusion, charm, speed, the stat changes, regain, poison, curse), and
/// what becomes of its effect. `ep` is the live effect's own number, `shown`
/// is `ccSpcConditionEffectSW()` for a party character, and `eye` the eye
/// view, in which Kite shows none. The order is in docs/engine/battle.md
/// ("Which condition shows").
pub fn disp_condition_effect(ch: &mut Char, ep: Option<i32>, shown: bool, eye: bool, volume: Volume) -> CondFx {
    let later = volume != Volume::Inf;
    let dead = ch.cond[cond::DEAD];
    if dead != 0 && dead != 1 {
        if ep.is_some() || later {
            ch.condition_num = -1;
        }
        return if ep.is_some() { CondFx::Kill } else { CondFx::Keep };
    }
    if ch.ty() & 4 != 0 && !(shown && (ch.id() != 0 || !eye)) {
        if ep.is_some() || later {
            ch.condition_num = -1;
        }
        return if ep.is_some() { CondFx::Delete } else { CondFx::Keep };
    }
    let num = ch.condition_num;
    let mut pick = -1;
    // The first active whose number is the current one keeps it.
    let mut keep = false;
    let mut cand = |x: i32| {
        if keep {
            return;
        }
        if num == x {
            keep = true;
        } else if pick == -1 {
            pick = x;
        }
    };
    let c = ch.cond;
    if c[cond::PARALYSIS] != 0 {
        cand(1);
    }
    for (k, x) in [(cond::SLEEP, 5), (cond::CONFUSION, 4), (cond::CHARM, 3)] {
        if c[k] != 0 {
            cand(x);
        }
    }
    if c[cond::SPEED] != 0 {
        let v = f32::from_bits(c.speed_value);
        if v > 1.0 {
            cand(22);
        }
        if v < 1.0 {
            cand(2);
        }
    }
    if let Some((temp, time)) = ch.temp_time() {
        for (j, k) in [0usize, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13].into_iter().enumerate() {
            if time[k] != 0 {
                if temp[k] > 0 {
                    cand(23 + j as i32);
                }
                if temp[k] < 0 {
                    cand(7 + j as i32);
                }
            }
        }
    }
    for (k, x) in [(cond::REGENE_HP, 20), (cond::REGENE_SP, 21), (cond::POISON, 0), (cond::CURSE, 6)] {
        if c[k] != 0 {
            cand(x);
        }
    }
    if keep {
        return match ep {
            None => CondFx::Set,
            Some(n) if n == num => CondFx::Keep,
            Some(_) => CondFx::Replace,
        };
    }
    // The kill only when the number was not -1: an effect it had then is
    // left behind.
    let killed = num != -1 && ep.is_some();
    if num != -1 {
        ch.condition_num = -1;
    }
    if pick == -1 {
        return if killed { CondFx::Kill } else { CondFx::Keep };
    }
    ch.condition_num = pick;
    if killed { CondFx::Replace } else { CondFx::Set }
}

/// `ccChar::ConditionBattleEffect` (gcmn 0x0056c940): drainHP, drainSP,
/// critical, dying and invincible become the largest value of any
/// equipped piece (head, body, arm, leg, weapon), or of the foe's row.
pub fn condition_battle_effect(t: &Tables, ch: &mut Char) {
    for i in 0..5 {
        ch.cond[cond::DRAIN_HP + i] = 0;
    }
    let tyb = ch.ty();
    let take = |c: &mut Condition, b: &Beff| {
        for i in 0..5 {
            if c[cond::DRAIN_HP + i] < b[i] {
                c[cond::DRAIN_HP + i] = b[i];
            }
        }
    };
    match &ch.body {
        Body::Spc(p) if tyb & ty::PC != 0 => {
            for s in [slot::HEAD, slot::BODY, slot::ARM, slot::LEG] {
                take(&mut ch.cond, piece(t, p, s).1);
            }
            // A job outside 0-5 has no weapon table; the game merges the
            // leg piece again there, which changes nothing.
            if (0..6).contains(&p.job) {
                take(&mut ch.cond, piece(t, p, slot::WEAPON).1);
            }
        }
        Body::Foe(f) if tyb & ty::FOE != 0 => take(&mut ch.cond, &f.row.beff),
        _ => {}
    }
}

/// The stats a buff or debuff times out on: pEva, mEva and the tolerances
/// are never timed.
const TIMED: [usize; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13];

/// `ccChar::ConditionTimeCount` (gcmn 0x0056bfd0), once a frame from
/// `CalcReal(0)`: buffs and debuffs expire; sleep, confusion, charm,
/// paralysis and speed count down; regeneration, poison and curse tick
/// through `EntryAffect` (9 HP, 3 poison, 10 SP, 4 curse); SP comes back
/// naturally once a second unless cursed.
pub fn condition_time_count(ch: &mut Char, env: &Env, ev: &mut Events) {
    if let Some((temp, time)) = ch.temp_time_mut() {
        for i in TIMED {
            let t = time[i];
            if t > 0 {
                time[i] = t - 1;
                if t - 1 == 0 {
                    temp[i] = 0;
                }
            } else if t < 0 {
                time[i] = 0;
                temp[i] = 0;
            }
        }
    }
    let is_pc = ch.is_pc();
    let c = &mut ch.cond;
    for i in [cond::SLEEP, cond::CONFUSION, cond::CHARM, cond::PARALYSIS] {
        if c[i] > 0 {
            c[i] -= 1;
        } else if c[i] < 0 {
            c[i] = 0;
        }
    }
    if c[cond::SPEED] > 0 {
        c[cond::SPEED] -= 1;
        if c[cond::SPEED] == 0 {
            c.speed_value = F_ONE;
        }
    } else if c[cond::SPEED] < 0 {
        c[cond::SPEED] = 0;
        c.speed_value = F_ONE;
    }
    let at_least_1 = |v: i32| if v == 0 { 1 } else { v } as i16;
    if c[cond::REGENE_HP] != 0 {
        c[cond::REGENE_HP] = c[cond::REGENE_HP].wrapping_sub(1);
        if cmod(i32::from(c[cond::REGENE_HP]), 60) == 0 {
            ev.push(Event::affect(Who::Me, Who::Me, 9, at_least_1(i32::from(ch.max_hp) / 50), 0, 0));
        }
    }
    let c = &mut ch.cond;
    if c[cond::POISON] != 0 && env.menu_type == -1 {
        c[cond::POISON] = c[cond::POISON].wrapping_sub(1);
        if cmod(i32::from(c[cond::POISON]), 90) == 0 && !(is_pc && ch.no_death) {
            ev.push(Event::affect(Who::Me, Who::Me, 3, at_least_1(i32::from(ch.max_hp) / 100), 0, 0));
        }
    }
    if ch.cond[cond::CURSE] == 0 {
        // Once a second (count % 60): maxSP/50 in battle and maxSP/20 out
        // of it with the regeneration bonus, maxSP/100 and maxSP/33
        // without; enemies always maxSP/50.
        let tick = env.count.is_multiple_of(60);
        let max_sp = i32::from(ch.max_sp);
        if is_pc {
            let div = match (env.sp_regene_speed, env.in_battle != 0) {
                (true, true) => 50,
                (true, false) => 20,
                (false, true) => 100,
                (false, false) => 33,
            };
            if tick {
                ch.sp = ch.sp.wrapping_add(at_least_1(max_sp / div));
            }
            if ch.sp > ch.max_sp {
                ch.sp = ch.max_sp;
            }
        } else if tick {
            ch.sp = ch.sp.wrapping_add(at_least_1(max_sp / 50));
            if ch.sp > ch.max_sp {
                ch.sp = ch.max_sp;
            }
        }
    }
    let c = &mut ch.cond;
    if c[cond::REGENE_SP] != 0 {
        c[cond::REGENE_SP] = c[cond::REGENE_SP].wrapping_sub(1);
        if cmod(i32::from(c[cond::REGENE_SP]), 60) == 0 {
            ev.push(Event::affect(Who::Me, Who::Me, 10, at_least_1(i32::from(ch.max_sp) / 50), 0, 0));
        }
    }
    let c = &mut ch.cond;
    if c[cond::CURSE] != 0 {
        c[cond::CURSE] = c[cond::CURSE].wrapping_sub(1);
        if cmod(i32::from(c[cond::CURSE]), 90) == 0 {
            ev.push(Event::affect(Who::Me, Who::Me, 4, at_least_1(i32::from(ch.max_sp) / 100), 0, 0));
        }
    }
}

fn clamp_max(p: &mut SpcParam) {
    p.max_hp = if p.max_hp <= 0 {
        1
    } else if p.max_hp >= 10000 {
        9999
    } else {
        p.max_hp
    };
    p.max_sp = if p.max_sp <= 0 {
        1
    } else if p.max_sp >= 1000 {
        999
    } else {
        p.max_sp
    };
}

fn growth(t: &Tables, id: i16) -> LevelUp {
    usize::try_from(id).ok().and_then(|i| t.level_up.get(i)).copied().unwrap_or_default()
}

/// `ccChar::CheckLevelUp` (gcmn 0x0056d430): every 1000 exp is a level;
/// each adds the character's `LevelUpParamTbl` row (stats clamped 0..999,
/// maxHP to 1..9999, maxSP to 1..999; HP and SP rise by the same amounts).
/// Level 99 zeroes exp. Returns the levels gained.
pub fn check_level_up(t: &Tables, ch: &mut Char) -> i32 {
    let Body::Spc(p) = &mut ch.body else { return 0 };
    let g = growth(t, p.base.id);
    let mut n = 0;
    while i32::from(p.base.exp) - 999 > 0 {
        p.base.exp = p.base.exp.wrapping_sub(1000);
        p.base.level = p.base.level.wrapping_add(1);
        add_elm(&mut p.elm, &g.elm, 999, 0);
        p.max_hp = p.max_hp.wrapping_add(g.max_hp);
        p.max_sp = p.max_sp.wrapping_add(g.max_sp);
        clamp_max(p);
        ch.max_hp = p.max_hp;
        ch.max_sp = p.max_sp;
        ch.hp = ch.hp.wrapping_add(g.max_hp);
        ch.sp = ch.sp.wrapping_add(g.max_sp);
        if ch.hp >= 10000 {
            ch.hp = 9999;
        }
        if ch.sp >= 1000 {
            ch.sp = 999;
        }
        n += 1;
        if p.base.level >= 99 {
            p.base.level = 99;
            p.base.exp = 0;
            break;
        }
    }
    n
}

/// `ccChar::LevelDown` (gcmn 0x0056d640), from Data Drain's side effects:
/// one level's growth taken back; HP and SP are cut to the new maxima.
pub fn level_down(t: &Tables, ch: &mut Char) {
    let Body::Spc(p) = &mut ch.body else { return };
    p.base.level = p.base.level.wrapping_sub(1);
    if p.base.level <= 0 {
        p.base.level = 1;
        return;
    }
    let g = growth(t, p.base.id);
    let neg: Elm = std::array::from_fn(|i| g.elm[i].wrapping_neg());
    add_elm(&mut p.elm, &neg, 999, 0);
    p.max_hp = p.max_hp.wrapping_sub(g.max_hp);
    p.max_sp = p.max_sp.wrapping_sub(g.max_sp);
    clamp_max(p);
    ch.max_hp = p.max_hp;
    ch.max_sp = p.max_sp;
    if ch.hp > p.max_hp {
        ch.hp = p.max_hp;
    }
    if ch.sp > p.max_sp {
        ch.sp = p.max_sp;
    }
}

/// `ccSetLevelParam(p, lvs)` (gcmn 0x00571a00): a record taken to level
/// `lvs` in one step (the debug new game's `InitSpcParam`).
pub fn set_level_param(t: &Tables, p: &mut SpcParam, lvs: i32) {
    let g = growth(t, p.base.id);
    let lv = lvs - i32::from(p.base.level);
    if lv > 0 {
        mul_add_elm(&mut p.elm, &g.elm, lv);
        p.max_hp = p.max_hp.wrapping_add(i32::from(g.max_hp).wrapping_mul(lv) as i16);
        p.max_sp = p.max_sp.wrapping_add(i32::from(g.max_sp).wrapping_mul(lv) as i16);
    } else if lv < 0 {
        let neg: Elm = std::array::from_fn(|i| g.elm[i].wrapping_neg());
        for _ in 0..-lv {
            p.base.level = p.base.level.wrapping_sub(1);
            if p.base.level <= 0 {
                p.base.level = 1;
                break;
            }
            add_elm(&mut p.elm, &neg, 999, 0);
            p.max_hp = p.max_hp.wrapping_sub(g.max_hp);
            p.max_sp = p.max_sp.wrapping_sub(g.max_sp);
        }
    }
    clamp_max(p);
    p.base.level = lvs as i16;
}

/// The element opposing each (2 soil .. 7 dark): soil-wind, water-fire,
/// thunder-dark.
pub fn opposite_attribute(a: i32) -> i32 {
    match a {
        2 => 5,
        5 => 2,
        3 => 4,
        4 => 3,
        6 => 7,
        7 => 6,
        _ => -1,
    }
}

/// `ccChar::CheckCharAttribute(flag)` (gcmn 0x005701d0): the element (2
/// soil .. 7 dark) the character's table attributes are strongest in (a
/// party member's `ccSpcParam.elm`, a foe's row), -1 on a tie at the top
/// or for anything else; `flag` 1 gives the opposing element, the one an
/// attribute critical needs.
pub fn check_char_attribute(ch: &Char, flag: i32) -> i32 {
    let tyb = ch.ty();
    let at: [i16; 6] = if tyb & 4 != 0 {
        match &ch.body {
            Body::Spc(p) => std::array::from_fn(|i| p.elm[8 + i]),
            _ => return -1,
        }
    } else if tyb & ty::FOE != 0 {
        match &ch.body {
            Body::Foe(f) => std::array::from_fn(|i| f.row.elm[8 + i]),
            _ => return -1,
        }
    } else {
        return -1;
    };
    let mut best = at[0];
    let mut v = 2;
    for (i, &a) in at.iter().enumerate().skip(1) {
        if best < a {
            best = a;
            v = 2 + i as i32;
        } else if best == a {
            v = -1;
        }
    }
    if v < 0 || flag == 0 { v } else { opposite_attribute(v) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_elm_wraps_then_clamps() {
        let mut d: Elm = [0; 16];
        d[0] = 32000;
        d[1] = -5;
        let mut s: Elm = [0; 16];
        s[0] = 1000; // wraps to a negative short, then clamps low
        s[1] = 3;
        add_elm(&mut d, &s, 999, -999);
        assert_eq!(d[0], -999);
        assert_eq!(d[1], -2);
    }

    #[test]
    fn attribute_ties_and_opposites() {
        let mut row = FoeRow::default();
        row.base.ty = 0x20;
        row.elm[8..14].copy_from_slice(&[10, 30, 20, 5, 5, 5]);
        let c = Char::foe(row.clone());
        assert_eq!(check_char_attribute(&c, 0), 3);
        assert_eq!(check_char_attribute(&c, 1), 4);
        row.elm[10] = 30;
        let c = Char::foe(row);
        assert_eq!(check_char_attribute(&c, 0), -1);
    }
}
