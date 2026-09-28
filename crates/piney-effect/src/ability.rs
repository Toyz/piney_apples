//! The stat changes' effects (main effect.cpp): `effAbilityUp(ch, num)`
//! (main 0x001d7380) and `effAbilityDown(ch, num)` (0x001d7630), which
//! `ccConditionEffect`'s constructor (0x001c13f0, `setConditionEffect`)
//! starts for conditions 20-34 and 0-18 and ends with the condition
//! (`killConditionEffect`, `deleteConditionEffect`: its serial number).
//!
//! ```text
//! checkEffAbilityColor(num) (0x001d78c0): 13, 29 -> 0; 14, 21, 30 -> 1;
//!   15, 31 -> 2; 16, 20, 32 -> 3; 17, 33 -> 4; 0, 1, 4, 6, 18, 34 -> 5;
//!   anything else 6: the textures' column in the tables below
//! effAbilityUp(ch, num)   the controller -20: target ch, posT its pos,
//!                         param num; with it two generators on ch's pos,
//!                         212 (texture UpS1) and 213 50 up (UpS2); then
//!                         the controller -21 (life -1, the same fields),
//!                         answered
//! -20's second chain (0x001cb418): ch off the lists: endFlag; count 4:
//!   214 (UpS3); count 10: 215 (UpS4); from 11 endFlag
//! -21's (0x001cb5cc): ch off the lists: endFlag; (count - 30) % 45 == 0:
//!   212 (UpS1) and 213 50 up (UpS2); == 20: 214 (UpS3)
//! effAbilityDown(ch, num) three generators on ch's pos, 217 (DnS1), 218
//!                         (DnS2), 219 (DnS2); the controller -22 (life
//!                         -1, target ch, posT its pos, param num), answered
//! -22's (0x001cb83c): ch off the lists: endFlag; (count - 30) % 30 == 0
//!   (count 0 too): 220 (DnS1), 221 (DnS2), 219 50 up (DnS3)
//! ```
//!
//! The tables (main 0x0033fdf0 `tableAbilityUpS1` to 0x0033feb0
//! `tableAbilityDownS3`, eight ints each, read by their low halves) name
//! the generators' textures (`pTexMod`); `%` is C's, toward zero.

use piney_data::volume::Volume;

use crate::ee::F;
use crate::effect::EffectCtrl;
use crate::particle::Ability;
use crate::{CharRef, Cx, VecRef, pfx};

/// Effect ids.
pub const UP_START: i16 = -20;
pub const UP: i16 = -21;
pub const DOWN: i16 = -22;

/// 50.0: the second generator's lift.
const FIFTY: F = 0x4248_0000;

/// `tableAbilityUpS1`-`S4`, `tableAbilityDownS1`-`S3`: textures by
/// `checkEffAbilityColor`.
#[derive(Clone, Debug)]
pub struct Tables {
    pub up: [[i16; 8]; 4],
    pub down: [[i16; 8]; 3],
}

impl Tables {
    /// The volume's (`tables::effect`): the words, read as shorts.
    pub fn read(volume: Volume) -> Tables {
        let t = piney_data::tables::effect::of(volume);
        let row = |r: &[i32]| -> [i16; 8] { std::array::from_fn(|i| r.get(i).map_or(0, |&v| v as i16)) };
        Tables {
            up: std::array::from_fn(|k| row(t.ability_up()[k])),
            down: std::array::from_fn(|k| row(t.ability_down()[k])),
        }
    }
}

/// `checkEffAbilityColor(num)` (main 0x001d78c0).
pub fn color(num: i32) -> usize {
    match num {
        13 | 29 => 0,
        14 | 21 | 30 => 1,
        15 | 31 => 2,
        16 | 20 | 32 => 3,
        17 | 33 => 4,
        0 | 1 | 4 | 6 | 18 | 34 => 5,
        _ => 6,
    }
}

/// A generator of `row` on `ch`'s pos, `lift` up (None: the constructor's
/// offset), texture `tex`.
fn spark(cx: &mut Cx, row: usize, ch: CharRef, lift: Option<F>, tex: i16) {
    pfx::start(cx, row, |g| {
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::CharPos(ch));
        if let Some(z) = lift {
            g.offset = [0, 0, z, 0];
        }
        g.p_tex_mod = tex;
    });
}

/// A controller `id` on `ch`: target, posT, param `num`.
fn controller(ctrl: &mut EffectCtrl, cx: &mut Cx, id: i16, ch: CharRef, num: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, id)?;
    let pos = cx.host.char_pos(ch);
    let e = &mut ctrl.effects[i];
    e.life_time = -1;
    e.target = Some(ch);
    e.pos_t = pos;
    e.param = num;
    Some(i)
}

/// `effAbilityUp(ch, num)` (main 0x001d7380): the controller -21.
pub fn eff_ability_up(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, num: i32) -> Option<usize> {
    let c = color(num);
    if controller(ctrl, cx, UP_START, ch, num).is_some() {
        let t = cx.spells.data.ability.up;
        spark(cx, 212, ch, None, t[0][c]);
        spark(cx, 213, ch, Some(FIFTY), t[1][c]);
    }
    controller(ctrl, cx, UP, ch, num)
}

/// `effAbilityDown(ch, num)` (main 0x001d7630): the controller -22.
pub fn eff_ability_down(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, num: i32) -> Option<usize> {
    let c = color(num);
    let t = cx.spells.data.ability.down;
    spark(cx, 217, ch, None, t[0][c]);
    spark(cx, 218, ch, None, t[1][c]);
    spark(cx, 219, ch, None, t[1][c]);
    controller(ctrl, cx, DOWN, ch, num)
}

/// The helper `setConditionEffect` takes ([`crate::particle::set_condition_effect`]):
/// `effAbilityDown` or `effAbilityUp(cp, num)`.
pub fn eff_ability(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, num: i32, which: Ability) -> Option<usize> {
    match which {
        Ability::Down => eff_ability_down(ctrl, cx, ch, num),
        Ability::Up => eff_ability_up(ctrl, cx, ch, num),
    }
}

/// The target, when on the lists; else the effect ends.
fn listed(ctrl: &mut EffectCtrl, cx: &Cx, i: usize) -> Option<CharRef> {
    let e = &mut ctrl.effects[i];
    let t = e.target.filter(|&t| cx.host.check_target(t));
    if t.is_none() {
        e.end_flag = true;
    }
    t
}

/// -20's case of the second chain (main 0x001cb418).
pub fn up_start_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let Some(t) = listed(ctrl, cx, i) else { return };
    let e = &ctrl.effects[i];
    let (cnt, c) = (e.cnt, color(e.param));
    let up = cx.spells.data.ability.up;
    match cnt {
        4 => spark(cx, 214, t, None, up[2][c]),
        10 => spark(cx, 215, t, None, up[3][c]),
        _ => {}
    }
    if cnt >= 11 {
        ctrl.effects[i].end_flag = true;
    }
}

/// -21's case of the second chain (main 0x001cb5cc).
pub fn up_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let Some(t) = listed(ctrl, cx, i) else { return };
    let e = &ctrl.effects[i];
    let (k, c) = ((i32::from(e.cnt) - 30) % 45, color(e.param));
    let up = cx.spells.data.ability.up;
    if k == 0 {
        spark(cx, 212, t, None, up[0][c]);
        spark(cx, 213, t, Some(FIFTY), up[1][c]);
    } else if k == 20 {
        spark(cx, 214, t, None, up[2][c]);
    }
}

/// -22's case of the second chain (main 0x001cb83c).
pub fn down_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let Some(t) = listed(ctrl, cx, i) else { return };
    let e = &ctrl.effects[i];
    let (k, c) = ((i32::from(e.cnt) - 30) % 30, color(e.param));
    let down = cx.spells.data.ability.down;
    if k == 0 {
        spark(cx, 220, t, None, down[0][c]);
        spark(cx, 221, t, None, down[1][c]);
        spark(cx, 219, t, Some(FIFTY), down[2][c]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::{ONE, k};
    use crate::host::{CharState, Simple};

    #[test]
    fn the_colours() {
        assert_eq!([13, 21, 31, 20, 33, 18, 22, 19].map(color), [0, 1, 2, 3, 4, 5, 6, 6]);
    }

    /// A stat up's rows: its start's, the start's own two, then every 45
    /// frames from 30 the pair and 20 frames later one more; it runs until
    /// the condition ends it.
    #[test]
    fn a_stat_up_repeats_until_ended() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut r = || 0;
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], Camera::default());
        host.chars.push(CharState { id: 5, pos: [0, 0, 0, ONE], dirc: [0; 4], height: k(160.0), width: k(45.0) });
        let i = fx.ability_up(&mut host, 5, 20).unwrap();
        let mut rows: Vec<usize> = fx.particles.started.drain(..).map(|g| g.row).collect();
        for _ in 0..80 {
            fx.step(&mut host);
            rows.extend(fx.particles.started.drain(..).map(|g| g.row));
        }
        assert_eq!(rows, [212, 213, 214, 215, 212, 213, 214, 212, 213]);
        assert_eq!(fx.ctrl.effects[i].status, 1);
        fx.ctrl.effects[i].end_flag = true;
        fx.step(&mut host);
        assert!(fx.ctrl.effects.iter().all(|e| e.status == 0));
    }
}
