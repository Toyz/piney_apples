//! Skills (`skill.cpp`): what a skill's type bits mean, its SP cost, range
//! and element; healing (`ccSkillRecovery`), holding (`ccSkillHold`),
//! status conditions and buffs (`ccCheckConditionSkillSuccess`,
//! `_ccSkillModifyCondition`, `ccSkillModifyCondition`), the cures of
//! `ccSkill::RecoverySystem`, and `ccCheckTargetConditionBySkill`.
//!
//! Area rules walk the target side's command list and take every living
//! character of the matching type whose ground distance from the centre,
//! less its width, is within the skill's `targetRange`
//! ([`crate::damage::ground_distance`]). The positional variants take the
//! centre in the player's frame (`posP`): the game converts a world point
//! with `ccTransPosW2P` (`ccPlayer::W2PPos`, gcmn 0x0059b940) first, which
//! is the runtime's.

use piney_data::field::ee;

use crate::chara::{Char, Env};
use crate::damage::ground_distance;
use crate::event::{Event, Events, Who};
use crate::param::{F_ONE, SkillParam, cond, elm, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;

/// `ccSkillParam.type` bits.
pub mod bits {
    pub const PHYSICAL: i32 = 0x1;
    pub const MAGIC: i32 = 0x2;
    /// soil water fire wind thunder dark: `4 << i`.
    pub const ELEMENTS: i32 = 0xfc;
    /// Read only from the rows: attack and support spells.
    pub const ATTACK_SPELL: i32 = 0x100;
    pub const SUPPORT_SPELL: i32 = 0x200;
    /// The three arts of each weapon class (`ccSkillDamageValue` counts
    /// their hits by job).
    pub const ARTS: i32 = 0x1c00;
    /// The area is centred on the user, not the target.
    pub const CENTRED_ON_USER: i32 = 0x2000;
    /// Goes on when its target dies or leaves (`ccSkill::Main`).
    pub const UNTARGETED: i32 = 0x4000;
    /// Others in the area take x0.5.
    pub const SPLASH_HALF: i32 = 0x8000;
    pub const BUFF: i32 = 0x10000;
    pub const DEBUFF: i32 = 0x20000;
    pub const HEAL: i32 = 0x40000;
}

/// `ccSkillAttributeCheck(ccSkillParam *)` (gcmn 0x005725a0): the skill's
/// element, 2 soil .. 7 dark, the last bit set winning; -1 for none.
pub fn skill_attribute(stype: i32) -> i32 {
    let mut v = -1;
    for i in 0..6 {
        if stype & (4 << i) != 0 {
            v = 2 + i;
        }
    }
    v
}

/// `ccSkillCheckTypeAttribute(type)` (gcmn 0x00573af0): the first element
/// bit set, 2..7, else 0.
pub fn check_type_attribute(stype: i32) -> i32 {
    (0..6).find(|i| stype & (4 << i) != 0).map_or(0, |i| 2 + i)
}

/// `_ccSkillCheckType(type)` (gcmn 0x00573c30): -1 a plain physical attack
/// (or no type), 0 an art; a spell: 3 healing, 2 a buff, -2 a debuff, 1
/// an attack spell.
pub fn check_type(stype: i32) -> i32 {
    if stype & bits::PHYSICAL != 0 {
        if stype & bits::ARTS == 0 { -1 } else { 0 }
    } else if stype & bits::MAGIC != 0 {
        if stype & bits::HEAL != 0 {
            3
        } else if stype & bits::BUFF != 0 {
            2
        } else if stype & bits::DEBUFF != 0 {
            -2
        } else {
            1
        }
    } else {
        -1
    }
}

/// `ccSkillCheckType(sid)` (gcmn 0x00573b90): 0 for the normal attack
/// (skill 1), else [`check_type`] of its row.
pub fn check_type_of(t: &Tables, sid: i32) -> i32 {
    if sid == 1 {
        return 0;
    }
    t.skill(sid).map_or(-1, |s| check_type(s.ty))
}

/// `ccSkillCostCheck(ch, sk)` (gcmn 0x00572550): enough SP.
pub fn cost_check(ch: &Char, sk: &SkillParam) -> bool {
    i32::from(ch.sp) >= sk.cost
}

/// `ccSkillCostConsume(ch, sk)` (gcmn 0x00572610): pay the cost if there
/// is enough SP.
pub fn cost_consume(ch: &mut Char, sk: &SkillParam) -> bool {
    let sp = i32::from(ch.sp);
    if sp < sk.cost {
        return false;
    }
    ch.sp = sp.wrapping_sub(sk.cost) as i16;
    true
}

/// `ccSkillRangeCheck(sk, dist)` (gcmn 0x00572690): within `triggerRange`.
pub fn range_check(sk: &SkillParam, dist: u32) -> bool {
    ee::le(dist, sk.trigger_range)
}

/// `ccSkillRecoveryCheck(sid)` (gcmn 0x005726b0): the cure for a
/// condition or debuff skill, 178 (Antidote: poison, paralysis, slow,
/// lowered physical and element stats) or 179 (Restorative).
pub fn recovery_check(sid: i32) -> i32 {
    match sid {
        156..=158 | 163..=165 | 169..=174 => 178,
        _ => 179,
    }
}

/// `ccGetArmsEffectAttribute(n)` (gcmn 0x00573cc0): an element (2..7) to
/// the weapon glow's effect number 40..45.
pub fn arms_effect_attribute(n: i32) -> i32 {
    if (2..8).contains(&n) { 38 + n } else { 0 }
}

/// `ccSpcChar::StartArmsEffect(sid)` (gcmn 0x0059e530) on `ch`: an art or
/// the normal attack whose element lights the weapon turns `armsEffectSW`
/// (+0xe4) on and starts the element's particles along the weapon; one
/// without an element turns it off. What it starts:
/// `startParticleEffect2(&weaponEffPos[a], &weaponEffPos[b], row,
/// &armsEffectSW)` as `[a, b, row]` - job 0's twin blades both (points 0-2
/// and 1-3), the other jobs' one weapon (0-1), none past job 5 (the game
/// writes through a null pointer). Nothing while the switch is already on.
pub fn start_arms_effect(t: &Tables, ch: &mut Char, sid: i32) -> Vec<[i32; 3]> {
    if check_type_of(t, sid) != 0 {
        return Vec::new();
    }
    let stype = t.skill(sid).map_or(0, |s| s.ty);
    let row = arms_effect_attribute(check_type_attribute(stype));
    if row == 0 {
        ch.spc_char.arms_effect_sw = 0;
        return Vec::new();
    }
    if ch.spc_char.arms_effect_sw != 0 {
        return Vec::new();
    }
    ch.spc_char.arms_effect_sw = 1;
    match ch.spc().map_or(-1, |p| p.job) {
        0 => vec![[0, 2, row], [1, 3, row]],
        1..=5 => vec![[0, 1, row]],
        _ => Vec::new(),
    }
}

/// The healing skills' amount (`ccSkillRecovery`): 150 for Repth (150,
/// 153), 400 (151, 154), the target's maxHP (152, 155), the item's
/// `param` for 295. Any other skill heals whatever the register held in
/// the game; here 0.
pub fn recovery_amount(sid: i32, target_max_hp: i16, param: i32) -> i16 {
    match sid {
        150 | 153 => 150,
        151 | 154 => 400,
        152 | 155 => target_max_hp,
        295 => param as i16,
        _ => 0,
    }
}

fn heal_one(scene: &Scene, creator: usize, c: usize, sid: i32, param: i32, ev: &mut Events) {
    let amount = recovery_amount(sid, scene.chars[c].max_hp, param);
    ev.push(Event::affect(Who::Char(c), Who::Char(creator), 7, amount, 0, 0));
    ev.push(Event::HealSkill { on: Who::Char(c), sid });
}

/// The members of a side within `range` of `centre`, in list order, that
/// match `tyb` and are alive.
fn in_area(scene: &Scene, list: &[usize], tyb: i32, centre: [u32; 4], range: u32) -> Vec<usize> {
    list.iter()
        .copied()
        .filter(|&c| {
            let ch = &scene.chars[c];
            tyb & ch.ty() != 0 && ch.cond[cond::DEAD] == 0 && ee::le(ground_distance(ch, centre), range)
        })
        .collect()
}

/// `ccSkillRecovery(creator, target, sid, param)` (gcmn 0x00574bc0): a
/// healing skill. The creator must be listed and alive. Single target:
/// `EntryAffect(7, amount)` on the target, which need not be alive. Area:
/// every living member of the *creator's* side near the target (or the
/// creator, type bit 0x2000). Returns the number healed.
pub fn recovery(
    t: &Tables,
    scene: &Scene,
    creator: usize,
    target: usize,
    sid: i32,
    param: i32,
    ev: &mut Events,
) -> i32 {
    let cr = &scene.chars[creator];
    if !scene.listed(creator) || cr.cond[cond::DEAD] != 0 {
        return 0;
    }
    let Some(sk) = t.skill(sid) else { return 0 };
    let tyb = cr.ty();
    if ee::le(sk.target_range, 0) {
        heal_one(scene, creator, target, sid, param, ev);
        return 1;
    }
    let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { cr.pos_p } else { scene.chars[target].pos_p };
    let Some(list) = scene.side(tyb) else { return 0 };
    let hit = in_area(scene, &list, tyb, centre, sk.target_range);
    for &c in &hit {
        heal_one(scene, creator, c, sid, param, ev);
    }
    hit.len() as i32
}

/// `ccSkillRecovery(creator, pos, sid, param)` (gcmn 0x00574f70): the
/// area heal around a point; nothing for a single-target skill.
pub fn recovery_at(
    t: &Tables,
    scene: &Scene,
    creator: usize,
    pos: [u32; 4],
    sid: i32,
    param: i32,
    ev: &mut Events,
) -> i32 {
    let cr = &scene.chars[creator];
    if !scene.listed(creator) || cr.cond[cond::DEAD] != 0 {
        return 0;
    }
    let Some(sk) = t.skill(sid) else { return 0 };
    if ee::le(sk.target_range, 0) {
        return 0;
    }
    let tyb = cr.ty();
    let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { cr.pos_p } else { pos };
    let Some(list) = scene.side(tyb) else { return 0 };
    let hit = in_area(scene, &list, tyb, centre, sk.target_range);
    for &c in &hit {
        heal_one(scene, creator, c, sid, param, ev);
    }
    hit.len() as i32
}

/// `ccSkillHold(creator, target, sk)` (gcmn 0x005752b0): the characters a
/// skill holds while it runs take `EntryAffect(5)`, which sets their
/// `hold` condition (see [`crate::affect`]). A single-target skill holds
/// its target whatever its state; an area one every living character of
/// the target's side in range.
pub fn hold(scene: &Scene, creator: usize, target: usize, sk: &SkillParam, ev: &mut Events) -> i32 {
    if ee::le(sk.target_range, 0) {
        ev.push(Event::affect(Who::Char(target), Who::Char(creator), 5, 0, 0, 0));
        return 1;
    }
    let tyb = scene.chars[target].ty();
    let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { pos_p_of(scene, creator) } else { scene.chars[target].pos_p };
    let Some(list) = scene.side(tyb) else { return 0 };
    let hit = in_area(scene, &list, tyb, centre, sk.target_range);
    for &c in &hit {
        ev.push(Event::affect(Who::Char(c), Who::Char(creator), 5, 0, 0, 0));
    }
    hit.len() as i32
}

/// `ccSkillHold(creator, pos, ttype, sk)` (gcmn 0x00575520).
pub fn hold_at(scene: &Scene, creator: usize, pos: [u32; 4], ttype: i32, sk: &SkillParam, ev: &mut Events) -> i32 {
    if ee::le(sk.target_range, 0) {
        return 0;
    }
    let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { pos_p_of(scene, creator) } else { pos };
    let Some(list) = scene.side(ttype) else { return 0 };
    let hit = in_area(scene, &list, ttype, centre, sk.target_range);
    for &c in &hit {
        ev.push(Event::affect(Who::Char(c), Who::Char(creator), 5, 0, 0, 0));
    }
    hit.len() as i32
}

/// Skills resisted by body tolerance and by spirit.
fn resisted_by(sid: i32) -> (bool, bool) {
    (matches!(sid, 156..=158 | 163..=165 | 169..=174), matches!(sid, 159..=162 | 166..=168))
}

/// `ccCheckConditionSkillSuccess(tp, sid)` (gcmn 0x00571830): whether a
/// condition or debuff skill takes. Never on a target off the command
/// lists (`listed` is `ccCheckTarget(tp)`) or one that is not a party
/// member or foe. Body tolerance covers 156-158, 163-165, 169-174, spirit
/// 159-162, 166-168: resisted when the tolerance is 1000 or more, or
/// `rand() % 1000 + 100 < tolerance`.
pub fn condition_success(tp: &Char, sid: i32, listed: bool, rng: &mut dyn Rng) -> bool {
    if !listed || tp.ty() & (ty::PC | ty::FOE) == 0 {
        return false;
    }
    let (body, spirit) = resisted_by(sid);
    let re = tp.real();
    let mut ok = true;
    for (on, tol) in [(body, re[elm::BODY]), (spirit, re[elm::SPIRIT])] {
        if !on {
            continue;
        }
        let tol = i32::from(tol);
        if tol >= 1000 || rng.rand() % 1000 + 100 < tol {
            ok = false;
        }
    }
    ok
}

/// What a condition skill does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Effect {
    /// Set a condition's frames.
    Cond(usize),
    /// The same, but not renewed while running.
    Once(usize),
    /// Speed: frames and `speedValue`.
    Speed(u32),
    /// A timed stat change: the stat and where the amount comes from.
    Stat(usize, Src),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Src {
    Atk,
    Hit,
    Attr(usize),
}

/// `_ccSkillModifyCondition`'s table: effect, frames on a foe, frames on
/// the party, `conditionNum`, the `EntryAffect` kind (17 good, 18 bad).
fn cond_skill(sid: i32) -> Option<(Effect, i16, i16, i32, i16)> {
    use Effect::*;
    Some(match sid {
        156 => (Cond(cond::POISON), 5400, 5400, 0, 18),
        157 => (Once(cond::PARALYSIS), 900, 450, 1, 18),
        158 => (Speed(0x3f00_0000), 1800, 900, 2, 18),
        177 => (Speed(0x3fe0_0000), 9000, 9000, 22, 17),
        159 => (Once(cond::CHARM), 600, 300, 3, 18),
        161 => (Once(cond::CONFUSION), 600, 300, 4, 18),
        160 => (Once(cond::SLEEP), 900, 450, 5, 18),
        162 => (Cond(cond::CURSE), 5400, 5400, 6, 18),
        175 | 296 => (Cond(cond::REGENE_HP), 5400, 5400, 20, 17),
        176 | 297 => (Cond(cond::REGENE_SP), 5400, 5400, 21, 17),
        _ => {
            const STAT: [(usize, Src); 12] = [
                (elm::P_ATK, Src::Atk),
                (elm::P_DEF, Src::Atk),
                (elm::P_HIT, Src::Hit),
                (elm::M_ATK, Src::Atk),
                (elm::M_DEF, Src::Atk),
                (elm::M_HIT, Src::Hit),
                (8, Src::Attr(0)),
                (9, Src::Attr(1)),
                (10, Src::Attr(2)),
                (11, Src::Attr(3)),
                (12, Src::Attr(4)),
                (13, Src::Attr(5)),
            ];
            let k = match sid {
                163..=174 => (sid - 163) as usize,
                181..=192 => (sid - 181) as usize,
                298..=303 => (sid - 298) as usize,
                _ => return None,
            };
            let (f, src) = STAT[k];
            if (163..=174).contains(&sid) {
                (Stat(f, src), 1800, 900, 7 + k as i32, 18)
            } else {
                (Stat(f, src), 9000, 9000, 23 + k as i32, 17)
            }
        }
    })
}

/// `_ccSkillModifyCondition(creator, tp, sid)` (gcmn 0x00575cf0): write
/// the condition or the timed buff or debuff (`temp` the skill's atk, hit
/// or element value, `time` its frames) and `conditionNum`, then
/// `EntryAffect(17 or 18, 0, sid)`. Paralysis, charm, confusion and sleep
/// are not renewed while running: `EntryAffect(3, -1, sid)` instead.
/// Frames: poison and curse 5400; paralysis and sleep 900 on a foe, 450
/// on the party; charm and confusion 600 / 300; slow (x0.5) 1800 / 900;
/// haste (x1.75) 9000; regeneration 5400; debuffs 1800 / 900; buffs 9000.
pub fn modify_condition(t: &Tables, tp: &mut Char, sid: i32, ev: &mut Events) {
    if tp.dead() {
        return;
    }
    let Some((eff, dur_foe, dur_pc, num, kind)) = cond_skill(sid) else { return };
    let dur = if tp.ty() & ty::FOE != 0 { dur_foe } else { dur_pc };
    match eff {
        Effect::Once(f) if tp.cond[f] != 0 => {
            ev.push(Event::affect(Who::Target, Who::Me, 3, -1, sid as i16, 0));
            return;
        }
        Effect::Cond(f) | Effect::Once(f) => tp.cond[f] = dur,
        Effect::Speed(v) => {
            tp.cond[cond::SPEED] = dur;
            tp.cond.speed_value = v;
        }
        Effect::Stat(f, src) => {
            let Some(sk) = t.skill(sid) else { return };
            let v = match src {
                Src::Atk => sk.atk,
                Src::Hit => sk.hit,
                Src::Attr(i) => sk.attr[i],
            };
            if let Some((temp, time)) = tp.temp_time_mut() {
                time[f] = dur;
                temp[f] = v;
            }
        }
    }
    tp.condition_num = num;
    ev.push(Event::affect(Who::Target, Who::Me, kind, 0, sid as i16, 0));
}

/// One target of `ccSkillModifyCondition`: the success roll unless forced
/// (or an item's poison or curse, stype 1), then the condition and
/// `effSkillStart`; a resisted skill shows the "miss" font instead.
#[allow(clippy::too_many_arguments)]
fn modify_one(
    t: &Tables,
    scene: &mut Scene,
    creator: usize,
    c: usize,
    sid: i32,
    stype: i32,
    force: bool,
    rng: &mut dyn Rng,
    out: &mut ModifyEvents,
) -> bool {
    let sure = force || (stype == 1 && (sid == 156 || sid == 162));
    if sure || condition_success(&scene.chars[c], sid, scene.listed(c), rng) {
        let mut e = Events::new();
        modify_condition(t, &mut scene.chars[c], sid, &mut e);
        crate::damage::place(&mut e, creator, c);
        out.events.extend(e);
        out.started.push(c);
        true
    } else {
        out.resisted.push(c);
        false
    }
}

/// What `ccSkillModifyCondition` did beyond the affect events: whom
/// `effSkillStart(ch, sid, stype == 2, 1)` started on, and who resisted
/// (`ccEntryFlyFontNew(2, -1, pos)`, the "miss" font).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModifyEvents {
    pub events: Events,
    pub started: Vec<usize>,
    pub resisted: Vec<usize>,
}

/// `ccSkillModifyCondition(creator, target, sid, stype, force)` (gcmn
/// 0x005756f0): a condition or buff skill on its target, or (area) every
/// living character of the target's side near the target. Returns the
/// number affected.
#[allow(clippy::too_many_arguments)]
pub fn skill_modify_condition(
    t: &Tables,
    scene: &mut Scene,
    creator: usize,
    target: usize,
    sid: i32,
    stype: i32,
    force: bool,
    rng: &mut dyn Rng,
    out: &mut ModifyEvents,
) -> i32 {
    let Some(range) = t.skill(sid).map(|s| s.target_range) else { return 0 };
    if ee::le(range, 0) {
        return i32::from(modify_one(t, scene, creator, target, sid, stype, force, rng, out));
    }
    let tyb = scene.chars[target].ty();
    let centre = scene.chars[target].pos_p;
    let Some(list) = scene.side(tyb) else { return 0 };
    let mut n = 0;
    for c in in_area(scene, &list, tyb, centre, range) {
        if modify_one(t, scene, creator, c, sid, stype, force, rng, out) {
            n += 1;
        }
    }
    n
}

/// `ccSkillModifyCondition(creator, pos, ttype, sid, stype, force)` (gcmn
/// 0x00575a50): the area around a point.
#[allow(clippy::too_many_arguments)]
pub fn skill_modify_condition_at(
    t: &Tables,
    scene: &mut Scene,
    creator: usize,
    pos: [u32; 4],
    ttype: i32,
    sid: i32,
    stype: i32,
    force: bool,
    rng: &mut dyn Rng,
    out: &mut ModifyEvents,
) -> i32 {
    let Some(range) = t.skill(sid).map(|s| s.target_range) else { return 0 };
    if ee::le(range, 0) {
        return 0;
    }
    let Some(list) = scene.side(ttype) else { return 0 };
    let mut n = 0;
    for c in in_area(scene, &list, ttype, pos, range) {
        if modify_one(t, scene, creator, c, sid, stype, force, rng, out) {
            n += 1;
        }
    }
    n
}

/// Rip Teyn (178, the Antidote), Rip Synk (179, the Restorative) and Rip
/// Maen (180, the Resurrect): the cures of `ccSkill::RecoverySystem`.
pub const ANTIDOTE: i32 = 178;
pub const RESTORATIVE: i32 = 179;
pub const RESURRECT: i32 = 180;

/// What `ccSkill::RecoverySystem` did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CureResult {
    /// The skill ended this frame (`ccSkill` status 1).
    pub end: bool,
}

/// `ccSkill::RecoverySystem` (gcmn 0x00577070): nothing while the target
/// is off the lists or down (but for Resurrect). On the skill's first
/// frame (`count == 0`): 178 clears poison, paralysis,
/// slowness and every lowered physical and element stat; 179 charm,
/// confusion, sleep, curse and lowered magic stats; 180 revives with full
/// HP and no SP unless the whole party is down (`annihilated`). A party
/// caster of a skill (stype 0) then pays its SP. The skill ends, unless a
/// party caster's attack animation is still running (`anmFlag` 0).
#[allow(clippy::too_many_arguments)]
pub fn recovery_system(
    t: &Tables,
    creator: Option<&mut Char>,
    tp: &mut Char,
    sid: i32,
    count: i16,
    stype: i32,
    annihilated: bool,
    listed: bool,
    ev: &mut Events,
) -> CureResult {
    match cure_target(tp, sid, count, annihilated, listed, ev) {
        None => CureResult { end: false },
        Some(first) => cure_creator(t, creator, sid, stype, first),
    }
}

/// [`recovery_system`]'s work on the target: `None` when it does nothing
/// this frame, else whether the first frame's cure ran (and the caster then
/// pays).
pub(crate) fn cure_target(
    tp: &mut Char,
    sid: i32,
    count: i16,
    annihilated: bool,
    listed: bool,
    ev: &mut Events,
) -> Option<bool> {
    if !listed || (tp.dead() && sid != RESURRECT) {
        return None;
    }
    // ccCheckConditionSkillSuccess: the cures are never resisted, but take
    // only on a party member or foe.
    if count == 0 && tp.ty() & (ty::PC | ty::FOE) != 0 {
        let mut n = 0;
        let fields: &[usize] = match sid {
            ANTIDOTE => {
                for i in [cond::POISON, cond::PARALYSIS] {
                    if tp.cond[i] != 0 {
                        tp.cond[i] = 0;
                        n += 1;
                    }
                }
                if ee::lt(tp.cond.speed_value, F_ONE) {
                    tp.cond[cond::SPEED] = 0;
                    tp.cond.speed_value = F_ONE;
                    n += 1;
                }
                &[0, 1, 2, 8, 9, 10, 11, 12, 13]
            }
            RESTORATIVE => {
                for i in [cond::CHARM, cond::CONFUSION, cond::SLEEP, cond::CURSE] {
                    if tp.cond[i] != 0 {
                        tp.cond[i] = 0;
                        n += 1;
                    }
                }
                &[4, 5, 6]
            }
            _ => &[],
        };
        if let Some((temp, time)) = tp.temp_time_mut() {
            for &i in fields {
                if temp[i] < 0 {
                    temp[i] = 0;
                    time[i] = 0;
                    n += 1;
                }
            }
        }
        match sid {
            ANTIDOTE => ev.push(Event::Cure(Who::Target)),
            RESTORATIVE => ev.push(Event::Sanity(Who::Target)),
            _ => {}
        }
        if (sid == ANTIDOTE || sid == RESTORATIVE) && n > 0 {
            ev.push(Event::affect(Who::Target, Who::Me, 16, 0, sid as i16, 0));
        }
        if sid == RESURRECT && !annihilated {
            tp.hp = tp.max_hp;
            tp.sp = 0;
            ev.push(Event::Resurrect(Who::Target));
            ev.push(Event::affect(Who::Target, Who::Me, 20, 0, sid as i16, 0));
        }
        return Some(true);
    }
    Some(false)
}

/// [`recovery_system`]'s caster, after the target (the same character when
/// it cures itself): on the first frame a party caster of its own skill
/// (`pay`) pays the SP; the skill ends unless a party caster's attack
/// animation still runs.
pub(crate) fn cure_creator(t: &Tables, creator: Option<&mut Char>, sid: i32, stype: i32, pay: bool) -> CureResult {
    let mut creator = creator;
    if pay
        && let Some(c) = creator.as_deref_mut()
        && c.ty() & 5 != 0
        && stype == 0
    {
        let cost = t.skill(sid).map_or(0, |s| s.cost);
        let v = i32::from(c.sp) - cost;
        c.sp = if v >= 0 { v as i16 } else { 0 };
    }
    if let Some(c) = creator
        && (stype == 0 || stype == 2)
        && c.ty() & 5 != 0
    {
        if c.anm_flag == 0 {
            return CureResult { end: false };
        }
        c.skill_id = 0;
        c.skill_status = 0;
    }
    CureResult { end: true }
}

/// `ccCheckTargetConditionBySkill(ch, sid)` (gcmn 0x00579750): whether a
/// character already has what a skill would give: the condition's frames
/// (poison 156, paralysis 157, slow 158 while `speedValue > 0`, charm,
/// sleep, confusion, curse 159-162, regeneration 175-176, haste 177 while
/// `speedValue > 1`), a lowered stat (163-174: the negative `temp`), a
/// raised one (181-192: the positive `temp`); 0 for none. -1 for a
/// character with no `temp` (not a foe, nor type bit 4).
pub fn target_condition_by_skill(ch: &Char, sid: i32) -> i32 {
    let tyb = ch.ty();
    if tyb & (ty::FOE | 4) == 0 {
        return -1;
    }
    let Some((temp, _)) = ch.temp_time() else { return -1 };
    let c = &ch.cond;
    let pos = |v: i16| if v <= 0 { 0 } else { i32::from(v) };
    let neg = |v: i16| if v >= 0 { 0 } else { i32::from(v) };
    const BA: [usize; 6] = [0, 1, 2, 4, 5, 6];
    match sid {
        156 => pos(c[cond::POISON]),
        157 => pos(c[cond::PARALYSIS]),
        158 => {
            if ee::le(c.speed_value, 0) {
                0
            } else {
                i32::from(c[cond::SPEED])
            }
        }
        159 => pos(c[cond::CHARM]),
        160 => pos(c[cond::SLEEP]),
        161 => pos(c[cond::CONFUSION]),
        162 => pos(c[cond::CURSE]),
        163..=168 => neg(temp[BA[(sid - 163) as usize]]),
        169..=174 => neg(temp[8 + (sid - 169) as usize]),
        175 => i32::from(c[cond::REGENE_HP]),
        176 => i32::from(c[cond::REGENE_SP]),
        177 => {
            if ee::le(c.speed_value, F_ONE) {
                0
            } else {
                i32::from(c[cond::SPEED])
            }
        }
        181..=186 => pos(temp[BA[(sid - 181) as usize]]),
        187..=192 => pos(temp[8 + (sid - 187) as usize]),
        _ => 0,
    }
}

/// `CheckCharAttribute` of the `_ccSkillRequest` attribute-critical roll
/// needs the target's table row and its lowered defences: an Exdefense
/// immunity the skill runs into (bit 0x1 with a physical skill, 0x2 with
/// magic) that is not broken rules the roll out.
fn exdefense_blocks(tp: &Char, stype: i32) -> bool {
    let Some(f) = tp.foe_state() else { return false };
    let ex = i32::from(f.row.exdefense);
    (ex & 1 != 0 && stype & 1 != 0 && f.real[elm::P_DEF] >= f.row.elm[elm::P_DEF])
        || (ex & 2 != 0 && stype & 2 != 0 && f.real[elm::M_DEF] >= f.row.elm[elm::M_DEF])
}

/// The rules of `_ccSkillRequest(cp, tp, sid, stype)` (gcmn 0x00572860),
/// the start of every skill use, with the `ccSkill` it would create.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request {
    /// A normal attack of the caster's that was running is ended.
    pub ended_attack: bool,
    /// `effSkillStart(ch, sid, a, b)` calls: (who, a, b).
    pub skill_start: Vec<(Who, i32, i32)>,
    /// `ccWordsPlay(sid, cp)`: the skill's name is shouted.
    pub words: bool,
    /// The new skill's attribute-critical flag.
    pub ac_flag: i16,
    /// Its bit 6: a condition or buff skill (type 0x30000), run by
    /// `ConditionModifySystem`.
    pub modify: bool,
    /// Its hold flag (byte 13 bit 0): not the normal attacks (sids below
    /// 6) nor buff, debuff or heal skills.
    pub hold: bool,
}

/// `_ccSkillRequest(cp, tp, sid, stype)`: `stype` 0 is a character's own
/// skill, 1 an item's effect on the target alone (`ccItemSkillRequest`),
/// 2 an item used through the caster. `running_attack` is whether the
/// caster has a normal attack (skill 1) running. A caster using a skill
/// (stype 0, sid 2 up, not a buff or heal, not 178-180) pays its SP here;
/// buffs, heals and cures pay when they take effect. The caster's
/// `skillID`, `skillStatus` (1, or 9 for stype 2) and target are set. An
/// attack on a foe whose strongest element the skill's opposes is an
/// attribute critical half the time: `(rand() >> 3) % 100 < 50`.
#[allow(clippy::too_many_arguments)]
pub fn request(
    t: &Tables,
    scene: &mut Scene,
    cp: usize,
    tp: usize,
    sid: i32,
    stype: i32,
    running_attack: bool,
    rng: &mut dyn Rng,
) -> Option<Request> {
    let mut out = Request::default();
    let sk = t.skill(sid).cloned();
    if stype == 0 || stype == 2 {
        if running_attack {
            scene.chars[cp].skill_status = 0;
            out.ended_attack = true;
        }
        if let Some(sk) = &sk
            && stype == 0
            && sid >= 2
            && sk.ty & (bits::BUFF | bits::HEAL) == 0
            && !(ANTIDOTE..=RESURRECT).contains(&sid)
        {
            let c = &mut scene.chars[cp];
            c.sp = c.sp.wrapping_sub(sk.cost as i16);
        }
    }
    if sid == 0 {
        return None;
    }
    let sk = sk.unwrap_or_default();
    let not_cure = !(ANTIDOTE..=RESURRECT).contains(&sid);
    if scene.listed(tp)
        && cp != tp
        && sid != 1
        && sk.ty & (bits::BUFF | bits::DEBUFF) == 0
        && not_cure
        && (stype == 0 || stype == 2)
    {
        out.skill_start.push((Who::Target, i32::from(stype == 2), 1));
    }
    out.modify = sk.ty & (bits::BUFF | bits::DEBUFF) != 0 && not_cure;
    if stype == 1 {
        out.skill_start.push((Who::Me, 1, 0));
    } else {
        let c = &mut scene.chars[cp];
        c.skill_id = sid as i16;
        c.skill_status = if stype == 2 { 9 } else { 1 };
        out.words = sid >= 6;
    }
    out.hold = !(sid < 6 || sk.ty & (bits::BUFF | bits::DEBUFF | bits::HEAL) != 0);
    if sid != 1
        && sk.ty & (bits::BUFF | bits::DEBUFF | bits::HEAL) == 0
        && (stype == 0 || stype == 2)
        && scene.chars[cp].ty() & 5 != 0
        && scene.listed(tp)
        && scene.chars[tp].ty() & ty::FOE != 0
        && !exdefense_blocks(&scene.chars[tp], sk.ty)
    {
        let a = skill_attribute(sk.ty);
        if a != -1 && a == crate::chara::check_char_attribute(&scene.chars[tp], 1) && (rng.rand() >> 3) % 100 < 50 {
            out.ac_flag = 1;
        }
    }
    Some(out)
}

/// The globals `ccSkill` reads that are not a character's.
pub type SkillEnv = Env;

/// A character's `posP`; for a caster that is gone (a null pointer in the
/// game, which then reads low memory) all zeros.
fn pos_p_of(scene: &Scene, i: usize) -> [u32; 4] {
    scene.chars.get(i).map_or([0; 4], |c| c.pos_p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_kinds() {
        // _ccSkillCheckType's movz: a spell with no heal or buff bit is an
        // attack spell (1) without the debuff bit, a debuff (-2) with it.
        assert_eq!(check_type(bits::MAGIC | 0x10), 1);
        assert_eq!(check_type(bits::MAGIC | bits::DEBUFF), -2);
        assert_eq!(check_type(bits::MAGIC | bits::BUFF), 2);
        assert_eq!(check_type(bits::MAGIC | bits::HEAL | bits::BUFF), 3);
        assert_eq!(check_type(bits::PHYSICAL), -1);
        assert_eq!(check_type(bits::PHYSICAL | 0x800), 0);
        assert_eq!(check_type(0), -1);
        assert_eq!((skill_attribute(0x14), check_type_attribute(0x14)), (4, 2));
    }
}
