//! The Grunties' growing up (main effect.cpp): `effEvolvePG` (main 0x001d0f00,
//! the controller -7 and its generators at counts 12 and 30), which
//! `ccPGuso`'s growing-up acts call at their change, and `effGrowPG`
//! (0x001d0fd0, generator 129), which `evoActAdult` calls as a young one
//! becomes a grown one. The -7 controller draws nothing and does not look at
//! whether its character is still listed. See docs/engine/effects.md ("A
//! Grunty growing up").

use crate::ee::{self, F, V4};
use crate::effect::EffectCtrl;
use crate::{CharRef, Cx, VecRef, pfx};

/// The controller `effEvolvePG` starts.
pub const EVOLVE: i16 = -7;
/// Its life (frames).
pub const EVOLVE_LIFE: i16 = 120;
/// `particleGeneratorTbl` rows (+0x1b90, +0x1bc8, +0x1c00, +0x1c38).
pub const EVOLVE_ROWS_12: [usize; 2] = [126, 127];
pub const EVOLVE_ROW_30: usize = 128;
pub const GROW_ROW: usize = 129;

const HALF: F = 0x3f00_0000;

/// `effEvolvePG(ch)` (main 0x001d0f00).
pub fn eff_evolve_pg(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let i = ctrl.new_effect(cx, EVOLVE)?;
    let pos = cx.host.char_pos(ch);
    let e = &mut ctrl.effects[i];
    e.life_time = EVOLVE_LIFE;
    e.target = Some(ch);
    e.pos_t = pos;
    Some(i)
}

/// -7's case of the second chain (main 0x001c9920).
pub fn evolve_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &ctrl.effects[i];
    let rows: &[usize] = match e.cnt {
        12 => &EVOLVE_ROWS_12,
        30 => &[EVOLVE_ROW_30],
        _ => return,
    };
    let Some(t) = e.target else { return };
    for &row in rows {
        let up = ee::mul(HALF, cx.host.char_height(t));
        pfx::start(cx, row, |g| {
            g.offset = [0, 0, up, 0];
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::CharPos(t));
        });
    }
}

/// `effGrowPG(ch)` (main 0x001d0fd0): a generator 129 at `pos` (the
/// character's), its z half the character's `height`.
pub fn eff_grow_pg(cx: &mut Cx, pos: V4, height: F) {
    let p = [pos[0], pos[1], ee::mul(HALF, height), pos[3]];
    pfx::start(cx, GROW_ROW, |g| g.pos = p);
}
