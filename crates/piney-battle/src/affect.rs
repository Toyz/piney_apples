//! Affects: what a character does with an `EntryAffect` (gcmn 0x0056b020), the
//! one way damage, healing, SP changes, revival and a Data Drain reach a
//! character. The character's `affectFunc` runs at once: `ccEnemyInfluence`
//! then `ccEnemy::affectEnemy` (0x00432840, 0x00433a70) for an enemy,
//! `ccFellow::Influence` (0x0041bdb0) for a party member, `Influence`
//! (0x0059ac50) for Kite. A combat loop applies the [`Event::Affect`]s a rule
//! returns, in order, with [`apply`]. The kinds are in docs/engine/battle.md.

use crate::chara::{AffectFunc, Char, spc_flag};
use crate::event::{Event, Events, Who};
use crate::exp::Party;
use crate::param::{F_ONE, cond, elm, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::check_type_of;
use crate::tables::Tables;

/// What an affect needs from outside the characters.
pub struct AffectCtx<'a> {
    pub party: &'a Party,
    /// `ccMenu` exists (it does in the field): the party panel can shake.
    pub menu: bool,
    /// `ccSkillCheck(ch)` (gcmn 0x005723e0) for a character: the id of
    /// its running skill that a hit can interrupt, 0 for none.
    pub skill_check: &'a dyn Fn(usize) -> i32,
    /// What a boss's `Affect` needs; None where there is no boss.
    pub boss: Option<&'a crate::boss::BossEnv<'a>>,
    /// The disc's volume: from Mutation on a drained party member speaks.
    pub volume: piney_data::volume::Volume,
}

fn who(i: Option<usize>) -> Who {
    i.map_or(Who::Target, Who::Char)
}

/// `ccChar::EntryAffect(ch, kind, p0, p1, p2)` on `scene.chars[on]` from
/// `by`: nothing for a character off the command lists or a drained foe
/// (its last affect was 13); kind 5 sets `hold`; kind 6 and a character
/// without an `affectFunc` stop here. Otherwise the affect is stored (with
/// the flash kinds 1-4 and 7-10 start) and, unless the character masks
/// the kind, applied.
#[allow(clippy::too_many_arguments)]
pub fn entry_affect(
    t: &Tables,
    scene: &mut Scene,
    ctx: &AffectCtx,
    on: usize,
    by: Option<usize>,
    kind: i16,
    p: [i16; 3],
    rng: &mut dyn Rng,
    ev: &mut Events,
) {
    if !scene.listed(on) {
        return;
    }
    let ch = &mut scene.chars[on];
    if ch.ty() & ty::FOE != 0 && ch.affect.ty == 13 {
        return;
    }
    if kind == 5 {
        ch.cond[cond::HOLD] = 1;
        return;
    }
    if kind == 6 || ch.affect.func == AffectFunc::None {
        return;
    }
    let a = &mut ch.affect;
    a.person = by;
    a.ty = kind;
    a.param = p;
    a.color = 0;
    match kind {
        1..=4 => {
            a.color_cnt = -10;
            a.color_rate = 70;
            a.color = 255;
        }
        7..=10 => {
            a.color_cnt = -5;
            a.color_rate = 65;
            a.color = 0x00d0_ff60;
        }
        _ => {}
    }
    if a.color != 0 {
        a.cond_color = 0;
        a.cond_color_rate = 0;
        a.cond_color_cnt = 12;
    }
    if a.mask & (1 << (kind & 31)) != 0 {
        return;
    }
    match ch.affect.func {
        AffectFunc::Enemy => enemy_influence(t, scene, on, ev),
        AffectFunc::Fellow => fellow_influence(scene, ctx, on, rng, ev),
        AffectFunc::Player => player_influence(scene, ctx, on, rng, ev),
        // ccGimmickAffect (gcmn 0x00453400): the object's affectFlag (the
        // +0xe0 byte, kept on the character until the object's frame
        // takes it: crate::gimmick::take_affect).
        AffectFunc::Gimmick => scene.chars[on].spc_char.flags |= spc_flag::AFFECT,
        AffectFunc::Boss => crate::boss::entry(scene, ctx, on, rng, ev),
        AffectFunc::None => {}
    }
}

/// Applies every [`Event::Affect`] of a rule's events in order, appending
/// what they cause; other events pass through. Events naming
/// [`Who::Char`] indices only (a scene rule's).
pub fn apply(t: &Tables, scene: &mut Scene, ctx: &AffectCtx, events: Events, rng: &mut dyn Rng) -> Events {
    let mut out = Events::new();
    for e in events {
        out.push(e);
        if let Event::Affect { on: Who::Char(on), by, kind, p } = e {
            let by = match by {
                Who::Char(i) => Some(i),
                _ => None,
            };
            entry_affect(t, scene, ctx, on, by, kind, p, rng, &mut out);
        }
    }
    out
}

/// `ccEnemyInfluence` (gcmn 0x00432840): `affectFlag` set, then
/// [`affect_enemy`]; a Data Drain (13) marks the enemy drained.
pub fn enemy_influence(t: &Tables, scene: &mut Scene, on: usize, ev: &mut Events) {
    scene.chars[on].spc_char.flags |= spc_flag::AFFECT;
    if affect_enemy(t, scene, on, ev) {
        scene.chars[on].spc_char.enemy_flags |= crate::chara::enemy_flag::DRAIN;
    }
}

/// `ccEnemy::affectEnemy` (gcmn 0x00433a70): damage and poison lower HP (a
/// virus-flagged enemy takes a tenth, never below half its maxHP), gains and
/// losses stop at the limits, revival restores a downed enemy, and hit or
/// missed the enemy picks its target again. Returns true for a Data Drain
/// (13), which changes nothing here (docs/engine/battle.md, "Affects").
pub fn affect_enemy(t: &Tables, scene: &mut Scene, on: usize, ev: &mut Events) -> bool {
    let me = Who::Char(on);
    let by = scene.chars[on].affect.person;
    let (kind, p0) = {
        let a = &scene.chars[on].affect;
        (a.ty, a.param[0])
    };
    let by_char = by.map(|i| scene.chars[i].clone());
    let ch = &mut scene.chars[on];
    let mut hp = ch.hp;
    let mut sp = ch.sp;
    let (max_hp, max_sp) = (ch.max_hp, ch.max_sp);
    match kind {
        13 => return true,
        20 => {
            if ch.cond[cond::DEAD] == 2 {
                hp = max_hp;
                sp = 0;
            }
        }
        4 => {
            sp = sp.wrapping_sub(p0).max(0);
            ev.push(Event::FlyFont { on: me, kind: 3, value: i32::from(p0) });
        }
        10 => {
            sp = sp.wrapping_add(p0).min(max_sp);
            ev.push(Event::FlyFont { on: me, kind: 5, value: i32::from(p0) });
        }
        1 | 3 => {
            ev.push(Event::FlyFont { on: me, kind: 2, value: i32::from(p0) });
            ev.push(Event::HitMark { on: me, by: who(by) });
            if p0 >= 0 {
                let mut d = p0;
                if let Some(b) = &by_char
                    && b.ty() & 4 != 0
                    && b.party_flag == 0
                {
                    d = 0;
                }
                if ch.spc_char.enemy_flags & crate::chara::enemy_flag::VIRUS != 0 {
                    hp = hp.wrapping_sub(d / 10);
                    let half = max_hp / 2;
                    if hp < half {
                        hp = half;
                    }
                } else {
                    hp = hp.wrapping_sub(d).max(0);
                }
            }
            ev.push(Event::EnemyRetarget(me));
        }
        7 | 9 => {
            hp = hp.wrapping_add(p0).min(max_hp);
            ev.push(Event::FlyFont { on: me, kind: 20, value: i32::from(p0) });
        }
        _ => {}
    }
    ch.hp = hp;
    ch.sp = sp;
    if kind == 1
        && let Some(f) = ch.foe_state()
    {
        let ex = i32::from(f.row.exdefense);
        let sid = i32::from(ch.affect.param[1]);
        if check_type_of(t, sid) == 1 {
            if ex & 2 != 0 && f.real[elm::M_DEF] >= f.row.elm[elm::M_DEF] {
                ev.push(Event::ResistantShield { on: me, magic: 1 });
            }
        } else if ex & 1 != 0 && f.real[elm::P_DEF] >= f.row.elm[elm::P_DEF] {
            ev.push(Event::ResistantShield { on: me, magic: 0 });
        }
    }
    false
}

/// The conditions a Data Drain leaves on a party member (affect 13):
/// poison and curse 5400 frames, paralysis and sleep 450, charm and
/// confusion 300, slow (x0.5) 900; HP and SP cut to half their maxima.
fn drained(ch: &mut Char, hp: &mut i16, sp: &mut i16) {
    let c = &mut ch.cond;
    c[cond::POISON] = 5400;
    c[cond::PARALYSIS] = 450;
    ch.condition_num = 1;
    c[cond::SLEEP] = 450;
    c[cond::CONFUSION] = 300;
    c[cond::CHARM] = 300;
    c[cond::SPEED] = 900;
    c.speed_value = 0x3f00_0000;
    c[cond::CURSE] = 5400;
    let h = ch.max_hp / 2;
    if h < *hp {
        *hp = h;
    }
    let s = ch.max_sp / 2;
    if s < *sp {
        *sp = s;
    }
}

/// `ccChar::ClearCondition()` (gcmn 0x0056cb50): every condition but
/// `dead`, `speedValue` back to 1.0; with the record (`ClearCondition(
/// ccSpcParam *)`, 0x0056cba0) its buffs and debuffs and their timers too.
pub fn clear_condition(ch: &mut Char) {
    clear_conditions(ch);
    if let Some((temp, time)) = ch.temp_time_mut() {
        *temp = [0; 16];
        *time = [0; 16];
    }
    if let Some(f) = ch.foe_state_mut() {
        f.pp = 0;
        f.pp_count = 0;
    }
}

/// `ccChar::ClearCondition()` alone: every condition but `dead`,
/// `speedValue` back to 1.0 (the bosses call it at their death).
pub fn clear_conditions(ch: &mut Char) {
    for i in 1..16 {
        ch.cond[i] = 0;
    }
    ch.cond.speed_value = F_ONE;
}

/// Stores the new HP and SP; a character with `noDeathFlag` only gains.
fn store(ch: &mut Char, hp: i16, sp: i16) {
    if ch.no_death {
        if ch.hp < hp {
            ch.hp = hp;
        }
        if ch.sp < sp {
            ch.sp = sp;
        }
    } else {
        ch.hp = hp;
        ch.sp = sp;
    }
}

/// Which character, Kite or a party member the AI drives.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Player,
    Fellow,
}

/// `ccFellow::Influence` (gcmn 0x0041bdb0), a party member's affects.
pub fn fellow_influence(scene: &mut Scene, ctx: &AffectCtx, on: usize, rng: &mut dyn Rng, ev: &mut Events) {
    spc_influence(scene, ctx, on, Kind::Fellow, rng, ev);
}

/// `Influence` (player.cpp, gcmn 0x0059ac50), Kite's affects.
pub fn player_influence(scene: &mut Scene, ctx: &AffectCtx, on: usize, rng: &mut dyn Rng, ev: &mut Events) {
    spc_influence(scene, ctx, on, Kind::Player, rng, ev);
}

/// Kite's and a party member's affects (`Influence`, `ccFellow::Influence`),
/// one body with the differences marked: damage and poison (a party member
/// loses HP even when down, Kite does not), the hurt act, death at HP 0 unless
/// a level up waits or `noDeathFlag` (act 9, `dead` 2, the "down" message),
/// gains, revival (`dead` 5) and a Data Drain's spread of conditions with HP
/// and SP halved. The rules are in docs/engine/battle.md ("Affects").
fn spc_influence(scene: &mut Scene, ctx: &AffectCtx, on: usize, who_is: Kind, rng: &mut dyn Rng, ev: &mut Events) {
    let me = Who::Char(on);
    let by = scene.chars[on].affect.person;
    let by_w = who(by);
    let (kind, p0, p1) = {
        let a = &scene.chars[on].affect;
        (a.ty, a.param[0], a.param[1])
    };
    let by_info = by.map(|i| (scene.chars[i].ty(), scene.chars[i].id(), scene.listed(i)));
    let slot = ctx.party.slot_of(i32::from(scene.chars[on].id()));
    let running = (ctx.skill_check)(on);
    let ch = &mut scene.chars[on];
    let mut hp = ch.hp;
    let mut sp = ch.sp;
    let fellow = who_is == Kind::Fellow;
    let other_id = |ch: &Char| by_info.is_some_and(|(_, id, _)| id != ch.id());
    match kind {
        // (A party member always has its AI; the game writes through
        // the null pointer otherwise.)
        0 if fellow && ch.has_ai => ev.push(Event::TalkOff(me)),
        1 | 3 => {
            ev.push(Event::FlyFont { on: me, kind: 2, value: i32::from(p0) });
            if p0 == -1 {
                return store(ch, hp, sp);
            }
            ev.push(Event::HitMark { on: me, by: by_w });
            let later = ctx.volume != piney_data::volume::Volume::Inf;
            if fellow {
                hp = hp.wrapping_sub(p0).max(0);
                if later {
                    ev.push(Event::NoteHit { on: me, value: i32::from(p0), ai: ch.has_ai });
                }
                if ch.dead() {
                    return store(ch, hp, sp);
                }
                if ch.party_flag != 0 && ctx.menu {
                    ev.push(Event::PanelBure { slot, n: 10 });
                }
                if kind == 1 && ch.spc_char.fellow_flags & 2 == 0 {
                    ch.spc_char.fellow_flags = (ch.spc_char.fellow_flags | 2) & !1;
                    ch.target_char = by;
                }
            } else {
                ev.push(Event::DamageActuate { value: i32::from(p0), act: ch.spc_char.act_num });
                if ch.party_flag != 0 && ch.spc_char.act_num != 14 && ctx.menu {
                    ev.push(Event::PanelBure { slot: 0, n: 10 });
                }
                if ch.dead() {
                    return store(ch, hp, sp);
                }
                hp = hp.wrapping_sub(p0).max(0);
                if later {
                    ev.push(Event::NoteHit { on: me, value: i32::from(p0), ai: ch.has_ai });
                }
            }
            if hp > 0 && kind != 1 {
                return store(ch, hp, sp);
            }
            if running == 1 {
                ev.push(Event::CancelAttack(me));
            }
            let exp = ch.base().exp;
            let survives = hp > 0 || exp >= 1000 || ch.no_death;
            if survives {
                if ch.spc_char.act_num < 7 && kind == 1 {
                    ch.spc_char.act_num = 7 + ((rng.rand() >> 3) & 1) as i16;
                    let f = &mut ch.spc_char.flags;
                    *f |= spc_flag::STOP | if fellow { spc_flag::PAUSE } else { spc_flag::RESTRAINT };
                    *f &= !(spc_flag::MOVE | spc_flag::TRAJECTORY);
                }
                if fellow && ch.has_ai && by_info.is_some_and(|(t, _, _)| t & ty::FOE != 0) {
                    ev.push(Event::ChatDamage { on: me, value: i32::from(p0), hp: ch.hp });
                }
            } else {
                ch.spc_char.act_num = 9;
                let f = &mut ch.spc_char.flags;
                *f |= spc_flag::STOP | if fellow { spc_flag::PAUSE } else { spc_flag::RESTRAINT };
                *f &= !(spc_flag::MOVE | spc_flag::TRAJECTORY);
                if fellow {
                    *f &= !spc_flag::RESTRAINT;
                    ch.spc_char.attack = 0;
                }
                ch.skill_status = 0;
                ch.skill_id = 0;
                ch.spc_char.arms_effect_sw = 0;
                ch.cond[cond::DEAD] = 2;
                ch.spc_char.cnt = 90;
                ch.spc_char.hit_enabled = false;
                ev.push(Event::HitDisable(me));
                clear_condition(ch);
                sp = 0;
                ch.condition_num = -1;
                ev.push(Event::ClearConditionEffect(me));
                if fellow {
                    ev.push(Event::ChatResurrectPlz(me));
                }
                ev.push(Event::SysMsgDown { on: me, id: sys_msg_id(ch) });
            }
        }
        2 | 4 => {
            ev.push(Event::FlyFont { on: me, kind: 3, value: i32::from(p0) });
            if p0 == -1 {
                return store(ch, hp, sp);
            }
            ev.push(Event::HitMark { on: me, by: by_w });
            if !ch.dead() {
                sp = sp.wrapping_sub(p0).max(0);
            }
        }
        7 | 9 => {
            let v = if ch.dead() { -1 } else { p0 };
            ev.push(Event::FlyFont { on: me, kind: 20, value: i32::from(v) });
            if v != -1 {
                hp = hp.wrapping_add(v).min(ch.max_hp);
                if fellow && other_id(ch) {
                    ev.push(Event::AffectMessages {
                        on: me,
                        kind: i32::from(kind),
                        by: by_w,
                        n: i32::from(hp),
                        hp: ch.hp,
                    });
                }
            }
        }
        8 | 10 => {
            let v = if ch.dead() { -1 } else { p0 };
            ev.push(Event::FlyFont { on: me, kind: 5, value: i32::from(v) });
            if v != -1 {
                sp = sp.wrapping_add(v).min(ch.max_sp);
            }
        }
        13 => {
            ev.push(Event::AfterDrain(me));
            ev.push(Event::DrainCtrl { from: by_w, to: me, kind: 0, a: 5, b: 3 });
            ev.push(Event::DrainCtrl { from: by_w, to: me, kind: 1, a: 5, b: 3 });
            drained(ch, &mut hp, &mut sp);
            // From Mutation on (MUT ccFellow::Influence 0x0043010c) the
            // member remarks on it, as on kinds 16-18.
            if fellow
                && ctx.volume != piney_data::volume::Volume::Inf
                && by_info.is_some_and(|(_, _, listed)| listed)
                && other_id(ch)
            {
                ev.push(Event::AffectMessages { on: me, kind: 13, by: by_w, n: i32::from(p1), hp: ch.hp });
            }
        }
        14 | 15 if fellow => ev.push(Event::Greeting { on: me, by: by_w, n: i32::from(kind - 14) }),
        16..=18 if fellow => {
            if by_info.is_some_and(|(_, _, listed)| listed) && other_id(ch) {
                let n = if kind == 18 { i32::from(p1) } else { 0 };
                ev.push(Event::AffectMessages { on: me, kind: i32::from(kind), by: by_w, n, hp: ch.hp });
            }
        }
        20 => {
            let d = ch.cond[cond::DEAD];
            if d != 0 && d != 5 {
                if ch.spc_char.act_num == 9 || ch.spc_char.act_num == 10 {
                    ch.spc_char.act_num = 2;
                }
                ch.skill_id = 0;
                ch.skill_status = 0;
                ch.cond[cond::DEAD] = 5;
                ch.spc_char.hit_enabled = true;
                ev.push(Event::HitEnable(me));
                ch.spc_char.cnt = 0;
                ch.spc_char.cloak = 0;
                if fellow {
                    if by_info.is_some_and(|(_, _, listed)| listed) && other_id(ch) {
                        ev.push(Event::AffectMessages { on: me, kind: 20, by: by_w, n: 0, hp: ch.hp });
                    }
                    ev.push(Event::SysMsgUp { on: me, id: sys_msg_id(ch) });
                }
            }
        }
        _ => {}
    }
    store(ch, hp, sp)
}

/// `ccSpcChar::CheckSysMsgID()` (gcmn 0x0059f530): the AI's system-message
/// id, -1 for a character without an AI.
pub fn sys_msg_id(ch: &Char) -> i16 {
    if ch.has_ai { ch.spc_char.sys_msg_id } else { -1 }
}

/// `ccSpcChar::CheckSpRegeneSpeed()` (gcmn 0x0059f380): SP comes back
/// faster while the character stands still (`moveFlag` clear) and uses no
/// skill.
pub fn sp_regene_speed(ch: &Char) -> bool {
    ch.spc_char.flags & spc_flag::MOVE == 0 && ch.skill_id == 0
}

/// `ccSpcChar::ConditionAdjustment()` (gcmn 0x0059ec70), called for Kite
/// or a party member when it leaves an area (`ccSpcChar::TransferOut`),
/// when it is set up in one (`ccSPC::Wakeup`) and at a fountain
/// (`ccMenuCtrl::FountainMenu3`): a character down (`dead` 2, 3 or 4)
/// becomes a ghost (`dead` 4): HP and SP 0, conditions cleared, see-
/// through (`cloak` 0.5), and a down act (9, 10) turns into getting up
/// (2); one revived (`dead` 5) stands up alive with full HP and no SP.
pub fn condition_adjustment(ch: &mut Char, me: Who, ev: &mut Events) {
    let f = &mut ch.spc_char.flags;
    match ch.cond[cond::DEAD] {
        5 => {
            ch.hp = ch.max_hp;
            ch.sp = 0;
            ch.spc_char.hit_enabled = true;
            ev.push(Event::HitEnable(me));
            ch.cond[cond::DEAD] = 0;
            *f &= !(spc_flag::GHOST | spc_flag::RESTRAINT);
            ch.spc_char.cloak = F_ONE;
            ch.spc_char.cnt = 0;
            if ch.spc_char.act_num != 2 {
                ch.spc_char.act_num = 2;
                ch.spc_char.act_num_old = -1;
            }
        }
        2..=4 => {
            ch.hp = 0;
            ch.sp = 0;
            ch.spc_char.hit_enabled = false;
            ev.push(Event::HitDisable(me));
            clear_condition(ch);
            ch.condition_num = -1;
            ev.push(Event::ClearConditionEffect(me));
            ch.cond[cond::DEAD] = 4;
            let f = &mut ch.spc_char.flags;
            *f |= spc_flag::GHOST;
            *f &= !(spc_flag::RESTRAINT | spc_flag::MOVE | spc_flag::TRAJECTORY);
            ch.spc_char.cloak = 0x3f00_0000;
            ch.spc_char.cnt = 0;
            if ch.spc_char.act_num == 9 || ch.spc_char.act_num == 10 {
                ch.spc_char.act_num = 2;
                ch.spc_char.act_num_old = -1;
            }
        }
        _ => {}
    }
}
