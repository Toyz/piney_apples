//! Hit and damage: `ccChar::CalcBattleDamage` (gcmn 0x0056d910) with the
//! protect gauge, Exdefense, element absorption and the equipment's effects;
//! `ccSkillDamage` (0x00573e60), a skill on its target or area; and
//! `ccSkillDamageValue` (0x00594e90), the party AI's estimate. Integer
//! arithmetic is the EE's C (products wrap at 32 bits, division truncates,
//! stored fields wrap at 16); `mag` is a float's bits, scaled by `cvt.s.w`,
//! `mul.s` and libgcc `__fixsfsi`.

use piney_data::field::ee;
use piney_data::volume::Volume;

use crate::chara::{AffectFunc, Char, Env, cdiv, check_char_attribute};
use crate::event::{Event, Events, Who};
use crate::param::{SkillParam, cond, elm, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::{bits, skill_attribute};
use crate::tables::Tables;

/// 2.0f: an attribute critical.
pub const F_TWO: u32 = 0x4000_0000;
/// 0.5f: an area skill's splash (type bit 0x8000) on the others.
pub const F_HALF: u32 = 0x3f00_0000;
pub use crate::param::F_ONE;

/// libgcc's `__fixsfsi` (main 0x00129c18) on a float's bits: truncate,
/// saturate at the int range; exponent 0 is 0.
pub fn fptosi(bits: u32) -> i32 {
    if (bits >> 23) & 0xff == 0 {
        return 0;
    }
    ee::to_int(bits)
}

/// `(int)((float)v * mag)` as the game computes it.
pub fn scale(v: i32, mag: u32) -> i32 {
    fptosi(ee::mul(ee::from_int(v), mag))
}

/// What `CalcBattleDamage` worked out, beyond its return value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Damage {
    /// The return value: the damage, 0 for an immunity or a nullified hit,
    /// -1 for a miss (or no damage at all).
    pub dmg: i32,
    /// `rand() % 101`, when drawn.
    pub roll: Option<i32>,
    /// The roll after the accuracy adjustment.
    pub hit: Option<i32>,
    /// HP the target absorbs (an element at 999 or more).
    pub recover: i32,
    /// Protect damage dealt, when the gauge was filled.
    pub pp: Option<i32>,
}

/// How the hit is decided: the game's `h` argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roll {
    /// `h < 0`: draw `rand() % 101`, with side effects.
    Draw,
    /// `h == 0`: a sure hit at 100, without side effects.
    Sure,
    /// `h > 0`: this roll, with side effects.
    Given(i32),
    /// This roll without side effects and without the RNG (for tables of
    /// outcomes; not a game call).
    Quiet(i32),
}

impl Roll {
    pub fn from_h(h: i32) -> Roll {
        match h {
            h if h < 0 => Roll::Draw,
            0 => Roll::Sure,
            h => Roll::Given(h),
        }
    }
}

/// Mutation's `0x005c24b0(tp, sid)` (from Mutation on): the Exdefense
/// kinds foe `tp` still holds (its row's bits whose defence is not
/// lowered below the table's) that skill `sid` runs into. For sid -1 every
/// one; none for a target off the lists or not a foe, or a skill with type
/// bits 0x70000; else physical and magic by the skill's type, and for a
/// skill from 2 with an element the elements by its type too.
pub fn exdefense_held(t: &Tables, scene: &Scene, tp: usize, sid: i32) -> i32 {
    let ch = &scene.chars[tp];
    if !scene.listed(tp) || ch.ty() & 0xe0 == 0 {
        return 0;
    }
    let Some(f) = ch.foe_state() else { return 0 };
    let ex = i32::from(f.row.exdefense);
    let held = |bits: &[(i32, usize)], stype: i32| {
        bits.iter()
            .filter(|&&(bit, i)| ex & bit != 0 && stype & bit != 0 && f.real[i] >= f.row.elm[i])
            .fold(0, |a, &(bit, _)| a | bit)
    };
    if sid == -1 {
        return held(&EXDEF, -1);
    }
    let Some(sk) = t.skill(sid) else { return 0 };
    let stype = sk.ty;
    if stype & 0x70000 != 0 {
        return 0;
    }
    let mut r = held(&EXDEF[..2], stype);
    if sid >= 2 && skill_attribute(stype) != -1 {
        r |= held(&EXDEF[2..], stype);
    }
    r
}

/// The Exdefense bits by defence: (bit, stat index).
const EXDEF: [(i32, usize); 8] = [
    (0x01, elm::P_DEF),
    (0x02, elm::M_DEF),
    (0x04, elm::SOIL),
    (0x08, elm::WATER),
    (0x10, elm::FIRE),
    (0x20, elm::WIND),
    (0x40, elm::THUNDER),
    (0x80, elm::DARK),
];

/// `ccChar::CalcBattleDamage(t, sk, mag, h)` (gcmn 0x0056d910): `att` hits
/// `tgt` with `sk`. `listed` is `ccCheckTarget(this) && ccCheckTarget(t)`:
/// both must be on the command lists. Emits the calls the game makes with
/// `att` as [`Who::Me`] and `tgt` as [`Who::Target`], and updates `tgt`'s
/// protect gauge. The roll and the formula are in docs/engine/battle.md
/// ("Hit and damage").
#[allow(clippy::too_many_arguments)]
pub fn calc_battle_damage(
    t: &Tables,
    att: &Char,
    tgt: &mut Char,
    sk: &SkillParam,
    mag: u32,
    roll: Roll,
    listed: bool,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> Damage {
    calc_battle_damage_on(t, att, tgt, sk, mag, roll, listed, false, rng, env, ev)
}

/// [`calc_battle_damage`] knowing whether the attacker is the target
/// itself (`self_hit`: a confused or charmed character's swing on itself),
/// whose drains then read its HP after the HP drain.
#[allow(clippy::too_many_arguments)]
fn calc_battle_damage_on(
    t: &Tables,
    att: &Char,
    tgt: &mut Char,
    sk: &SkillParam,
    mag: u32,
    roll: Roll,
    listed: bool,
    self_hit: bool,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> Damage {
    let mut r = Damage { dmg: -1, ..Damage::default() };
    let ad = att.cond[cond::DEAD];
    if !listed || !(ad == 0 || ad == 1) || tgt.cond[cond::DEAD] != 0 {
        return r;
    }
    let (mut hit, live) = match roll {
        Roll::Draw => {
            let v = rng.rand() % 101;
            r.roll = Some(v);
            (v, true)
        }
        Roll::Sure => (100, false),
        Roll::Given(h) => (h, true),
        Roll::Quiet(h) => (h, false),
    };
    let mut rec = 0i32;
    let sat = sk.attr;
    let at = att.ty();
    let (bbt, bat): ([i16; 8], [i16; 6]) = if at & (ty::PC | ty::FOE) != 0 {
        let re = att.real();
        (std::array::from_fn(|i| re[i]), std::array::from_fn(|i| re[8 + i]))
    } else {
        // A trap or other object never misses and hits with nothing.
        hit = 100;
        ([0, 0, 999, 0, 0, 0, 999, 0], [0; 6])
    };
    let npd = (at & ty::PC != 0 && att.no_death) || env.menu_forbid != 0;
    let tt = tgt.ty();
    let (tbt, tat): ([i16; 8], [i16; 6]) = {
        let re = tgt.real();
        (std::array::from_fn(|i| re[i]), std::array::from_fn(|i| re[8 + i]))
    };
    let mut adef = 0i32;
    if tt & ty::PC != 0 {
    } else if tt & ty::FOE != 0 {
        let Some(f) = tgt.foe_state() else { return r };
        let base = f.row.elm;
        adef = i32::from(f.row.exdefense);
        let stype = sk.ty;
        let holds = |&&(bit, i): &&(i32, usize)| adef & bit != 0 && stype & bit != 0 && tgt.real()[i] >= base[i];
        if t.volume == Volume::Inf {
            // Immune to all of them while any defence the skill works
            // against is not lowered below the table's value.
            if !EXDEF.iter().any(|e| holds(&e)) {
                adef = 0;
            }
        } else {
            // From Mutation on, immune only to those still held; none
            // with bit 0x100.
            let held = EXDEF.iter().filter(holds).fold(0, |a, &(bit, _)| a | bit);
            adef = if adef & 0x100 != 0 { 0 } else { held };
        }
    } else {
        return r;
    }
    let stype = sk.ty;
    let mut el = [0i32; 6];
    let mut elements = |rec: &mut i32| {
        for i in 0..6 {
            if stype & (4 << i) == 0 {
                continue;
            }
            if sat[i] != 0 {
                let a = cdiv(i32::from(sat[i]) + i32::from(bat[i]), 10).max(1);
                let d = cdiv(i32::from(tat[i]), 10).max(1);
                el[i] = cdiv(a.wrapping_mul(a), 2 * d);
            }
            if el[i] < 0 {
                el[i] = 0;
            } else if tat[i] >= 999 {
                let v = cdiv(el[i], 10);
                *rec += if v > 0 { v } else { 1 };
            }
        }
    };
    let kind = if stype & bits::PHYSICAL != 0 {
        Some((elm::P_ATK, elm::P_DEF, elm::P_HIT, elm::P_EVA, 0xfd, true))
    } else if stype & bits::MAGIC != 0 {
        Some((elm::M_ATK, elm::M_DEF, elm::M_HIT, elm::M_EVA, 0xfe, false))
    } else {
        None
    };
    let mut dmg = -1;
    if let Some((atk_i, def_i, hit_i, eva_i, mask, physical)) = kind {
        if hit < 5 {
            dmg = -1;
        } else if hit >= 95 {
            dmg = 1;
            hit = 100;
        } else {
            hit += cdiv(i32::from(bbt[hit_i]) + i32::from(sk.hit) - cdiv(2 * i32::from(tbt[eva_i]), 3), 10);
            if hit < 50 {
                dmg = -1;
                hit = hit.max(0);
            } else {
                dmg = 1;
                hit = hit.min(100);
            }
        }
        r.hit = Some(hit);
        if dmg > 0 {
            let pf = adef & mask;
            if pf == 0 {
                let a = (i32::from(bbt[atk_i]) + i32::from(sk.atk)).max(1);
                let d = i32::from(tbt[def_i]).max(1);
                dmg = cdiv(a.wrapping_mul(a), 2 * d);
                dmg = cdiv(dmg.wrapping_mul(hit), 100);
                if dmg <= 0 {
                    dmg = 1;
                }
                elements(&mut rec);
                dmg = dmg.wrapping_add(el.iter().sum::<i32>());
                dmg = cdiv(dmg.wrapping_mul(i32::from(sk.dmg_rate)), 100);
                dmg = scale(dmg, mag);
                if dmg >= 10000 {
                    dmg = 9999;
                }
            } else {
                if live && adef as u32 & 0xffff_ff03 == 0 {
                    ev.push(Event::AttributeGuard { on: Who::Target, by: Who::Me });
                }
                dmg = 0;
            }
            if live && env.plcol != 0 && pf == 0 {
                let ppdef = physical;
                r.pp = protect(t, tgt, sk, mag, hit, adef, npd, &el, dmg, i32::from(bbt[atk_i]), ppdef, att.id(), ev);
            }
            if stype & bits::PHYSICAL != 0 && live && sk.cost == 0 && pf == 0 {
                dmg = battle_effects(att, tgt, dmg, self_hit, rng, ev);
            }
        }
    }
    if live && rec > 0 {
        ev.push(Event::RecoveryReq { on: Who::Target, amount: rec });
    }
    r.dmg = dmg;
    r.recover = rec;
    r
}

/// The protect gauge (PP): a live hit on a foe fills `ccEnemyParam.PP`
/// by the attack against the row's `pDefPP`/`mDefPP`; full, it breaks for
/// 300 frames (the Data Drain window). A boss takes `hit / 80` instead of
/// `hit / 100`, and a physical hit on a boss is capped at 9999 before
/// `mag` rather than after. The drained form of an enemy never fills.
#[allow(clippy::too_many_arguments)]
fn protect(
    t: &Tables,
    tgt: &mut Char,
    sk: &SkillParam,
    mag: u32,
    hit: i32,
    adef: i32,
    npd: bool,
    el: &[i32; 6],
    dmg: i32,
    atk: i32,
    physical: bool,
    att_id: i16,
    ev: &mut Events,
) -> Option<i32> {
    let tt = tgt.ty();
    let boss = if tt & ty::ENEMY != 0 {
        if t.check_drain_enemy(i32::from(tgt.id())) {
            return None;
        }
        false
    } else if tt & ty::BOSS != 0 {
        true
    } else {
        return None;
    };
    let hp = i32::from(tgt.hp);
    let f = tgt.foe_state_mut()?;
    if f.pp_count != 0 || f.pp < 0 || f.row.max_pp < 0 || npd {
        return None;
    }
    let v = if adef == 0 {
        let a = (atk + i32::from(sk.atk)).max(1);
        let d = i32::from(if physical { f.row.p_def_pp } else { f.row.m_def_pp }).max(1);
        let mut v = cdiv(a.wrapping_mul(a), 2 * d);
        v = cdiv(v.wrapping_mul(hit), if boss { 80 } else { 100 });
        if v <= 0 {
            v = 1;
        }
        v = v.wrapping_add(el.iter().sum::<i32>());
        v = cdiv(v.wrapping_mul(i32::from(sk.dmg_rate)), 100);
        if boss && physical {
            if v >= 10000 {
                v = 9999;
            }
            scale(v, mag)
        } else {
            let v = scale(v, mag);
            if v >= 10000 { 9999 } else { v }
        }
    } else {
        0
    };
    f.pp = f.pp.wrapping_add(v as i16);
    if f.pp < f.row.max_pp {
        return Some(v);
    }
    if !boss {
        f.pp = f.row.max_pp;
        if hp - dmg <= 0 {
            return Some(v);
        }
        f.pp_count = 300;
        f.pp = 0;
        ev.push(Event::Protect { on: Who::Target, broken: 0, kind: -1 });
    } else {
        f.pp_count = 300;
        f.pp = 0;
        ev.push(Event::Protect { on: Who::Target, broken: 0, kind: if att_id != 0 { 2 } else { 1 } });
    }
    ev.push(Event::SetProtect { state: 0, on: Who::Target });
    Some(v)
}

/// The attacker's equipment effects on a live physical hit by a skill that
/// costs no SP (the normal attack), each `rand() % 100 <= value`:
/// critical doubles; dying raises the damage to 3/4 of the target's HP
/// (not on a boss); drainHP and drainSP take half the damage as HP or SP.
/// Last, a target with the invincible condition ignores the hit, with the
/// *attacker's* invincible value as the chance (the game's own mix-up).
fn battle_effects(att: &Char, tgt: &Char, mut dmg: i32, self_hit: bool, rng: &mut dyn Rng, ev: &mut Events) -> i32 {
    let c = &att.cond;
    if c[cond::CRITICAL] != 0 && rng.rand() % 100 <= i32::from(c[cond::CRITICAL]) {
        dmg = dmg.wrapping_add(dmg);
        ev.push(Event::Critical(Who::Target));
    }
    if c[cond::DYING] != 0
        && tgt.ty() & ty::BOSS == 0
        && tgt.ent_root != 0
        && rng.rand() % 100 <= i32::from(c[cond::DYING])
    {
        let v = cdiv(i32::from(tgt.hp).wrapping_mul(3), 4);
        if dmg < v {
            dmg = v;
            ev.push(Event::Dying(Who::Target));
        }
    }
    // The target's HP as the drains read it: the game applies the HP
    // drain's EntryAffect on the attacker at once, so on a character hitting
    // itself the SP drain reads the HP the HP drain left.
    let mut tgt_hp = i32::from(tgt.hp);
    let half = |dmg: i32, hp: i32| if hp < dmg { cdiv(hp + 1, 2) } else { cdiv(dmg + 1, 2) };
    if c[cond::DRAIN_HP] != 0 && rng.rand() % 100 <= i32::from(c[cond::DRAIN_HP]) {
        let v = half(dmg, tgt_hp);
        ev.push(Event::affect(Who::Me, Who::Target, 9, v as i16, 0, 0));
        ev.push(Event::DrainCtrl { from: Who::Me, to: Who::Target, kind: 0, a: 5, b: 5 });
        if self_hit {
            tgt_hp = i32::from(hp_after_gain(tgt, v as i16));
        }
    }
    if c[cond::DRAIN_SP] != 0 && rng.rand() % 100 <= i32::from(c[cond::DRAIN_SP]) {
        let mut v = half(dmg, tgt_hp);
        if i32::from(tgt.sp) < v {
            v = i32::from(tgt.sp);
        }
        ev.push(Event::affect(Who::Me, Who::Target, 10, v as i16, 0, 0));
        ev.push(Event::affect(Who::Target, Who::Me, 4, v as i16, 0, 0));
        ev.push(Event::DrainCtrl { from: Who::Me, to: Who::Target, kind: 1, a: 5, b: 5 });
    }
    if tgt.cond[cond::INVINCIBLE] != 0 && rng.rand() % 100 <= i32::from(c[cond::INVINCIBLE]) {
        dmg = 0;
        ev.push(Event::NoDamage(Who::Target));
    }
    dmg
}

/// The HP `EntryAffect(9, v)` leaves on `ch` itself (`ccChar::EntryAffect`
/// 0x0056b020 with the character's `affectFunc`): nothing for a drained
/// foe, a masked kind or no `affectFunc`; an enemy's HP rises to at most
/// its maxHP (`affectEnemy` 0x00433a70); a party member's or Kite's the
/// same unless it is down (`ccFellow::Influence` 0x0041bdb0, `Influence`
/// 0x0059ac50).
fn hp_after_gain(ch: &Char, v: i16) -> i16 {
    let a = &ch.affect;
    if (ch.ty() & ty::FOE != 0 && a.ty == 13) || a.func == AffectFunc::None || a.mask & (1 << 9) != 0 {
        return ch.hp;
    }
    if a.func != AffectFunc::Enemy && ch.dead() {
        return ch.hp;
    }
    ch.hp.wrapping_add(v).min(ch.max_hp)
}

/// Moves a call's events onto scene indices: [`Who::Me`] to `me`,
/// [`Who::Target`] to `target`.
pub fn place(ev: &mut [Event], me: usize, target: usize) {
    for e in ev {
        e.map_who(|w| match w {
            Who::Me => Who::Char(me),
            Who::Target => Who::Char(target),
            c => c,
        });
    }
}

/// `CalcBattleDamage` between two characters of a scene, with the events
/// on their scene indices.
#[allow(clippy::too_many_arguments)]
pub fn calc_damage_in(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    target: usize,
    sk: &SkillParam,
    mag: u32,
    roll: Roll,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> Damage {
    let listed = scene.listed(me) && scene.listed(target);
    let att = scene.chars[me].clone();
    let mut e = Events::new();
    let r =
        calc_battle_damage_on(t, &att, &mut scene.chars[target], sk, mag, roll, listed, me == target, rng, env, &mut e);
    place(&mut e, me, target);
    ev.extend(e);
    r
}

/// `ccSkillDamage(attacker, target, sk, acFlag, sid)` (gcmn 0x00573e60): a
/// single-target skill (`targetRange <= 0`) hits the target; an area skill
/// every living character of the target's side within `targetRange` (less its
/// width) of the centre (the attacker for type bit 0x2000, else the target).
/// Each takes `EntryAffect(1, dmg, sid)`. `ac_flag` 1 (an attribute critical)
/// makes the aimed target a sure hit at x2 and stays -1 so later hits are
/// doubled too; a splash skill's others (bit 0x8000) take x0.5. Returns the
/// number of characters hit.
#[allow(clippy::too_many_arguments)]
pub fn skill_damage(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    target: usize,
    sk: &SkillParam,
    ac_flag: &mut i16,
    sid: i32,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> i32 {
    let ad = scene.chars[me].cond[cond::DEAD];
    // Infection gives up on a downed target at once; from Mutation on only
    // a single-target skill does (MUT gcmn 0x00599988), an area one still
    // takes in those around it.
    let down = scene.chars[target].dead();
    if !scene.listed(me) || !(ad == 0 || ad == 1) || !scene.listed(target) || (down && t.volume == Volume::Inf) {
        return 0;
    }
    let splash = sk.ty & bits::SPLASH_HALF != 0;
    let centred = sk.ty & bits::CENTRED_ON_USER != 0;
    let tty = side_types(t.volume, scene.chars[target].ty());
    let mut h = Roll::Draw;
    let crit = |scene: &Scene, ac_flag: &mut i16, h: &mut Roll, ev: &mut Events| {
        if *ac_flag == 1 {
            ev.push(Event::AttributeCriticalParticle(Who::Char(target)));
            if scene.chars[me].id() != 0 {
                ev.push(Event::ChatAttributeCritical(Who::Char(me)));
            }
            *h = Roll::Given(100);
            *ac_flag = -1;
        }
    };
    let chat = |scene: &Scene, c: usize, dmg: i16, ev: &mut Events| {
        let a = &scene.chars[me];
        if c == target
            && a.ty() & 6 != 0
            && a.id() != 0
            && a.party_flag != 0
            && a.has_ai
            && scene.chars[target].ty() & ty::FOE != 0
        {
            ev.push(Event::ChatAttack { ai: Who::Char(me), on: Who::Char(target), dmg: i32::from(dmg), sid });
        }
    };
    if !ee::le(sk.target_range, 0) {
        let centre = if centred { scene.chars[me].pos_p } else { scene.chars[target].pos_p };
        let list: Vec<usize> = if tty & 6 != 0 {
            scene.pc_list.clone()
        } else if tty & ty::FOE != 0 {
            scene.ene_list.clone()
        } else {
            return 0;
        };
        let mut n = 0;
        for c in list {
            let ch = &scene.chars[c];
            if tty & ch.ty() == 0 || ch.cond[cond::DEAD] != 0 {
                continue;
            }
            if !ee::le(ground_distance(t.volume, ch, centre), sk.target_range) {
                continue;
            }
            if c == target {
                crit(scene, ac_flag, &mut h, ev);
            }
            let mag = if *ac_flag != 0 && c == target {
                F_TWO
            } else if splash && c != target {
                F_HALF
            } else {
                F_ONE
            };
            let r = calc_damage_in(t, scene, me, c, sk, mag, h, rng, env, ev);
            let dmg = r.dmg as i16;
            ev.push(Event::affect(Who::Char(c), Who::Char(me), 1, dmg, sid as i16, 0));
            chat(scene, c, dmg, ev);
            n += 1;
        }
        n
    } else if down {
        0
    } else {
        crit(scene, ac_flag, &mut h, ev);
        let mag = if *ac_flag != 0 { F_TWO } else { F_ONE };
        let r = calc_damage_in(t, scene, me, target, sk, mag, h, rng, env, ev);
        let dmg = r.dmg as i16;
        ev.push(Event::affect(Who::Char(target), Who::Char(me), 1, dmg, sid as i16, 0));
        chat(scene, target, dmg, ev);
        1
    }
}

/// `ccSkillDamage(attacker, pos, ttype, sk, sid)` (gcmn 0x005743c0): an
/// area skill around a point (`pos` in the player's frame, or the
/// attacker for type bit 0x2000) on the living characters of type
/// `ttype`; the others of a splash skill take x0.5 (there is no aimed
/// target here, so every one of them does), no attribute critical.
/// Nothing for a single-target skill. Returns the number hit.
#[allow(clippy::too_many_arguments)]
pub fn skill_damage_at(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    pos: [u32; 4],
    ttype: i32,
    sk: &SkillParam,
    sid: i32,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> i32 {
    let ad = scene.chars[me].cond[cond::DEAD];
    if !scene.listed(me) || !(ad == 0 || ad == 1) {
        return 0;
    }
    if ee::le(sk.target_range, 0) {
        return 0;
    }
    let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { scene.chars[me].pos_p } else { pos };
    let Some(list) = scene.side(ttype) else { return 0 };
    let ttype = side_types(t.volume, ttype);
    let mag = if sk.ty & bits::SPLASH_HALF != 0 { F_HALF } else { F_ONE };
    let mut n = 0;
    for c in list {
        let ch = &scene.chars[c];
        if ttype & ch.ty() == 0
            || ch.cond[cond::DEAD] != 0
            || !ee::le(ground_distance(t.volume, ch, centre), sk.target_range)
        {
            continue;
        }
        let r = calc_damage_in(t, scene, me, c, sk, mag, Roll::Draw, rng, env, ev);
        ev.push(Event::affect(Who::Char(c), Who::Char(me), 1, r.dmg as i16, sid as i16, 0));
        n += 1;
    }
    n
}

/// `ccSkillDamage2(attacker, target, pos, ttype, sk, acFlag, sid)` (gcmn
/// 0x005746d0), what the spell effects (falls, upheavals, summons) call as
/// they land: [`skill_damage`] with the area around `pos` (or the
/// attacker) over characters of type `ttype`; the aimed `target` gets the
/// attribute critical if it is listed. A single-target call hits `target`
/// if it is listed. Returns the number hit.
#[allow(clippy::too_many_arguments)]
pub fn skill_damage2(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    target: usize,
    pos: [u32; 4],
    ttype: i32,
    sk: &SkillParam,
    ac_flag: &mut i16,
    sid: i32,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> i32 {
    let ad = scene.chars[me].cond[cond::DEAD];
    if !scene.listed(me) || !(ad == 0 || ad == 1) {
        return 0;
    }
    let splash = sk.ty & bits::SPLASH_HALF != 0;
    let mut h = Roll::Draw;
    let crit = |scene: &Scene, ac_flag: &mut i16, h: &mut Roll, ev: &mut Events| {
        if *ac_flag == 1 {
            ev.push(Event::AttributeCriticalParticle(Who::Char(target)));
            if scene.chars[me].id() != 0 {
                ev.push(Event::ChatAttributeCritical(Who::Char(me)));
            }
            *h = Roll::Given(100);
            *ac_flag = -1;
        }
    };
    let chat = |scene: &Scene, c: usize, dmg: i16, ev: &mut Events| {
        let a = &scene.chars[me];
        if c == target
            && a.ty() & 6 != 0
            && a.id() != 0
            && a.party_flag != 0
            && a.has_ai
            && scene.chars[target].ty() & ty::FOE != 0
        {
            ev.push(Event::ChatAttack { ai: Who::Char(me), on: Who::Char(target), dmg: i32::from(dmg), sid });
        }
    };
    if ee::le(sk.target_range, 0) {
        if !scene.listed(target) {
            return 0;
        }
        crit(scene, ac_flag, &mut h, ev);
        let mag = if *ac_flag != 0 { F_TWO } else { F_ONE };
        let r = calc_damage_in(t, scene, me, target, sk, mag, h, rng, env, ev);
        let dmg = r.dmg as i16;
        ev.push(Event::affect(Who::Char(target), Who::Char(me), 1, dmg, sid as i16, 0));
        chat(scene, target, dmg, ev);
        return 1;
    }
    let centre = if sk.ty & bits::CENTRED_ON_USER != 0 { scene.chars[me].pos_p } else { pos };
    let Some(list) = scene.side(ttype) else { return 0 };
    let ttype = side_types(t.volume, ttype);
    let mut n = 0;
    for c in list {
        let ch = &scene.chars[c];
        if ttype & ch.ty() == 0
            || ch.cond[cond::DEAD] != 0
            || !ee::le(ground_distance(t.volume, ch, centre), sk.target_range)
        {
            continue;
        }
        if c == target && scene.listed(target) {
            crit(scene, ac_flag, &mut h, ev);
        }
        let mag = if *ac_flag != 0 && c == target {
            F_TWO
        } else if splash && c != target {
            F_HALF
        } else {
            F_ONE
        };
        let r = calc_damage_in(t, scene, me, c, sk, mag, h, rng, env, ev);
        let dmg = r.dmg as i16;
        ev.push(Event::affect(Who::Char(c), Who::Char(me), 1, dmg, sid as i16, 0));
        chat(scene, c, dmg, ev);
        n += 1;
    }
    n
}

/// The types an area skill takes in on the side it is aimed at: from
/// Outbreak on, every foe type (0xe0) once the foes' list is the one
/// walked; before it, only the types aimed at. OUT gcmn: the damages
/// 0x00595dac, 0x005962f8, 0x0059664c; `ccSkillHold` 0x00597200,
/// 0x00597478; `ccSkillModifyCondition` 0x00597684, 0x005979dc (QUA the
/// same, 0x004886ec on).
pub(crate) fn side_types(volume: Volume, tyb: i32) -> i32 {
    if volume >= Volume::Out && tyb & 6 == 0 && tyb & ty::FOE != 0 { ty::FOE } else { tyb }
}

/// A character's distance on the ground from a point, less its width:
/// `sqrtf(|posP - centre|^2 with the third lane zeroed) - base.width`
/// (`sceVu0SubVector`, `sceVu0InnerProduct`, the volume's `sqrtf`).
pub fn ground_distance(volume: Volume, ch: &Char, centre: [u32; 4]) -> u32 {
    ee::sub(crate::geom::plane_dist(volume, ch.pos_p, centre), ch.base().width)
}

/// The AI's hit count for a physical art, by job and art bit (tested in
/// this order), and for spells by skill.
const ART_HITS: [&[(i32, i32)]; 6] = [
    &[(0x400, 4), (0x800, 6), (0x1000, 16)],
    &[(0x400, 2), (0x1000, 4)],
    &[(0x800, 3), (0x1000, 3)],
    &[(0x800, 2)],
    &[(0x400, 3), (0x800, 4), (0x1000, 2)],
    &[],
];

/// The hits `ccSkillDamageValue` counts for a spell.
pub fn spell_hits(sid: i32) -> i32 {
    const RUNS: [(i32, [i32; 3]); 8] = [
        (192, [194, 195, 196]),
        (224, [226, 227, 228]),
        (256, [258, 259, 260]),
        (272, [274, 275, 276]),
        (200, [202, 203, 204]),
        (216, [218, 219, 220]),
        (248, [250, 251, 252]),
        (280, [282, 283, 284]),
    ];
    for (base, ids) in RUNS {
        if ids.contains(&sid) {
            return sid - base;
        }
    }
    const NINE: [i32; 20] =
        [197, 198, 199, 200, 209, 210, 211, 212, 229, 230, 231, 232, 241, 242, 243, 244, 261, 262, 263, 264];
    if NINE.contains(&sid) { 9 } else { 1 }
}

/// What [`skill_damage_value`] returns and leaves in its globals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DamageValue {
    pub dmg: i32,
    /// `SkillDamageValueAttributeCritical`: the skill's element opposes the
    /// target's strongest.
    pub critical: bool,
    /// `SkillDamageValueAttributeGuard`: the skill's element is the
    /// target's strongest.
    pub guard: bool,
}

/// `ccSkillDamageValue(cp, tp, sid)` (gcmn 0x00594e90), the party AI's
/// estimate of a skill: `CalcBattleDamage` as a sure hit (no RNG, no side
/// effects), x2 when the skill's element opposes the target's strongest
/// (an attribute critical), times the hits the AI counts for the art or
/// spell. 0 for a target off the command lists (`tp_listed`,
/// `ccCheckTarget(tp)`) or down; -1 times the hits when the caster is off
/// them (`cp_listed`: `CalcBattleDamage` checks both).
pub fn skill_damage_value(
    t: &Tables,
    cp: &Char,
    tp: &Char,
    sid: i32,
    cp_listed: bool,
    tp_listed: bool,
    env: &Env,
) -> DamageValue {
    let mut out = DamageValue::default();
    if !tp_listed || tp.dead() {
        return out;
    }
    let Some(sk) = t.skill(sid) else { return out };
    let mut mag = F_ONE;
    if cp.ty() & 5 != 0 && tp.ty() & ty::FOE != 0 {
        let a = skill_attribute(sk.ty);
        if a != -1 {
            if a == check_char_attribute(tp, 1) {
                mag = F_TWO;
                out.critical = true;
            } else if a == check_char_attribute(tp, 0) {
                out.guard = true;
            }
        }
    }
    let mut tp2 = tp.clone();
    let mut none = || 0;
    let mut ev = Events::new();
    let mut dmg = calc_battle_damage(t, cp, &mut tp2, sk, mag, Roll::Sure, cp_listed, &mut none, env, &mut ev).dmg;
    let s = sk.ty;
    if s & bits::PHYSICAL != 0 {
        let job = cp.spc().map(|p| i32::from(p.job)).unwrap_or(0);
        if (0..6).contains(&job) {
            for &(bit, n) in ART_HITS[job as usize] {
                if s & bit != 0 {
                    dmg = dmg.wrapping_mul(n);
                    break;
                }
            }
        }
    } else if s & bits::MAGIC != 0 {
        dmg = dmg.wrapping_mul(spell_hits(sid));
    }
    out.dmg = dmg;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An area skill at the foes' side around a point where a plain
    /// enemy (type 0x20) and a Data Bug (0x40) stand, aimed at type 0x20:
    /// Mutation's takes in the enemy alone, Outbreak's and Quarantine's
    /// both (`ttype` made 0xe0).
    fn taken_in(volume: Volume) -> i32 {
        let t = Tables::of(volume);
        let bug = t.enemies.iter().position(|e| e.param.base.ty == ty::MIDDLE_BOSS).expect("a Data Bug row");
        let mut scene = Scene::default();
        let me = scene.add(Char::pc(t.chars[0].param), 0);
        scene.add(Char::foe(t.enemies[1].param.clone()), 1);
        scene.add(Char::foe(t.enemies[bug].param.clone()), 1);
        let mut sk = t.skill(1).unwrap().clone();
        sk.target_range = 100f32.to_bits();
        let mut none = || 0;
        let env = Env::default();
        skill_damage_at(&t, &mut scene, me, [0; 4], 0x20, &sk, 1, &mut none, &env, &mut Events::new())
    }

    /// An area skill aimed at a downed enemy with another beside it:
    /// Infection's hits no one, Mutation's the other.
    fn around_a_downed_target(volume: Volume) -> i32 {
        let t = Tables::of(volume);
        let mut scene = Scene::default();
        let me = scene.add(Char::pc(t.chars[0].param), 0);
        let down = scene.add(Char::foe(t.enemies[1].param.clone()), 1);
        scene.add(Char::foe(t.enemies[1].param.clone()), 1);
        scene.chars[down].cond[cond::DEAD] = 2;
        let mut sk = t.skill(1).unwrap().clone();
        sk.target_range = 100f32.to_bits();
        let (mut none, mut ac) = (|| 0, 0);
        let env = Env::default();
        skill_damage(&t, &mut scene, me, down, &sk, &mut ac, 1, &mut none, &env, &mut Events::new())
    }

    #[test]
    fn an_area_skill_at_a_downed_foe_hits_those_around_it_from_mutation_on() {
        assert_eq!(around_a_downed_target(Volume::Inf), 0);
        assert_eq!(around_a_downed_target(Volume::Mut), 1);
    }

    /// A sure hit's damage from Kite with skill `sid` on Outbreak's `row`,
    /// its defence `def` lowered by `by`.
    fn sure_hit(row: usize, sid: i32, def: usize, by: i16) -> i32 {
        let t = Tables::of(Volume::Out);
        let kite = Char::pc(t.chars[0].param);
        let mut foe = Char::foe(t.enemies[row].param.clone());
        foe.real_mut()[def] -= by;
        let (sk, mut none) = (t.skill(sid).unwrap().clone(), || 0);
        let env = Env::default();
        calc_battle_damage(&t, &kite, &mut foe, &sk, F_ONE, Roll::Quiet(100), true, &mut none, &env, &mut Events::new())
            .dmg
    }

    /// Exdefense (docs/engine/battle.md): Black Death (row 176, 2) takes
    /// nothing from a spell (Juk Kruz, 245) while its magic defence holds
    /// and takes a blow (ATTACK, 1); Gaia Turtle (114, 1) the reverse. Once
    /// the defence is below the table's, it no longer guards.
    #[test]
    fn exdefense_bars_a_kind_while_its_defence_holds() {
        assert_eq!(sure_hit(176, 245, elm::M_DEF, 0), 0);
        assert!(sure_hit(176, 1, elm::M_DEF, 0) > 0);
        assert!(sure_hit(176, 245, elm::M_DEF, 1) > 0);
        assert_eq!(sure_hit(114, 1, elm::P_DEF, 0), 0);
        assert!(sure_hit(114, 245, elm::P_DEF, 0) > 0);
        assert!(sure_hit(114, 1, elm::P_DEF, 1) > 0);
    }

    #[test]
    fn an_area_skill_takes_in_every_foe_type_from_outbreak_on() {
        assert_eq!(taken_in(Volume::Mut), 1);
        assert_eq!(taken_in(Volume::Out), 2);
        assert_eq!(taken_in(Volume::Qua), 2);
    }
}
