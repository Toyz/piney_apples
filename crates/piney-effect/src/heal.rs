//! Healing, the cures, a box opening and a trap removed (main effect.cpp):
//! `effHeal` (main 0x001ccd20) and `effHealSkill` (0x001cccf0) for the healing
//! skills, the items and the Data Drain menu; `effCure`, `effSanity`,
//! `effResurrect` (0x001cef50, 0x001cf0d0, 0x001cedd0) from
//! `ccSkill::RecoverySystem`; `effOpenBox` (0x001ced40) when a body lying down
//! ends or a box, fountain, food or idol opens; `effRemoveTrap` (0x001d0980).
//! The effects are 130, -19, -1..-3 and -6; their cases are in
//! docs/engine/effects.md ("Heals, cures, a box, a trap").

use piney_data::volume::Volume;

use crate::ee::{self, F, V4};
use crate::effect::{EffectCtrl, Next};
use crate::{CharRef, Cx, Event, VecRef, particle, pfx};

/// Effect ids.
pub const HEAL_RING: i16 = 130;
pub const HEAL: i16 = -19;
pub const RESURRECT: i16 = -1;
pub const CURE: i16 = -2;
pub const SANITY: i16 = -3;
pub const REMOVE_TRAP: i16 = -6;

/// `ccSeOn3D(75, ch's pos)`: the heal.
pub const SE_HEAL: i32 = 75;
/// `particleGeneratorTbl` rows.
pub const HEAL_ROW: usize = 211;
pub const OPEN_BOX_ROW: usize = 94;
pub const REMOVE_TRAP_ROW: usize = 118;
pub const REMOVE_TRAP_END_ROW: usize = 119;
/// By -id - 1: resurrect, cure, sanity.
const CURE_ROWS: [usize; 3] = [61, 107, 108];

const HALF: F = 0x3f00_0000;
const HUNDRED: F = 0x42c8_0000;

/// The cures' generator rows by count (main 0x00340170 A, 0x00340180 B,
/// 0x00340190 C, 0x003401a0 D), each by -id - 1.
#[derive(Clone, Debug)]
pub struct Tables {
    pub a: [i32; 3],
    pub b: [i32; 3],
    pub c: [i32; 3],
    pub d: [i32; 3],
}

impl Tables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Tables {
        use crate::spell::first;
        let t = piney_data::tables::effect::of(volume);
        Tables { a: first(t.cure_a()), b: first(t.cure_b()), c: first(t.cure_c()), d: first(t.cure_d()) }
    }
}

/// `effHeal(ch, n)` (main 0x001ccd20): the controller -19 (the game
/// answers 0).
pub fn eff_heal(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, n: i32) -> Option<usize> {
    if let Some(i) = ctrl.new_effect(cx, HEAL_RING) {
        let e = &mut ctrl.effects[i];
        e.life_time = 30;
        e.pos_ptr = Some(VecRef::CharPos(ch));
    }
    let i = ctrl.new_effect(cx, HEAL)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 30;
    e.level = (((n & 0xf) << 4) as i8) >> 4;
    e.pos_ptr = Some(VecRef::CharPos(ch));
    particle::cc_particle_heal(cx, ch, 1);
    let pos = cx.host.char_pos(ch);
    cx.raise(Event::Sound3d { se: SE_HEAL, pos });
    Some(i)
}

/// `effHealSkill(ch, sid)` (main 0x001cccf0): `effHeal(ch, sid - 149)`.
pub fn eff_heal_skill(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, sid: i32) -> Option<usize> {
    eff_heal(ctrl, cx, ch, sid - 149)
}

/// 130's case of the first switch (main 0x001c5b6c).
pub fn ring_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    const ONE: F = 0x3f80_0000;
    const TOP: F = 0x4013_3333;
    const THIN: F = 0x3e4c_cccd;
    const WIDE: F = 0x3f99_999a;
    const OUT: F = 0x4080_0000;
    let e = &mut ctrl.effects[i];
    let (cnt, life) = (i32::from(e.cnt), i32::from(e.life_time));
    let ten = ee::from_int(10);
    e.scale[2] = if cnt < life - 10 {
        let step = ee::div(ee::sub(TOP, ONE), ee::from_int(life - 10));
        ee::add(ONE, ee::mul(ee::from_int(cnt), step))
    } else {
        let k = ee::from_int(10 - (life - cnt));
        ee::sub(TOP, ee::mul(ee::div(TOP, ten), k))
    };
    e.scale[0] = if cnt < 10 {
        ee::add(THIN, ee::div(ee::mul(ee::from_int(cnt), ee::sub(WIDE, THIN)), ten))
    } else if cnt >= life - 10 {
        ee::add(OUT, ee::div(ee::mul(ee::sub(WIDE, OUT), ee::from_int(life - cnt)), ten))
    } else {
        WIDE
    };
    e.scale[1] = e.scale[0];
    e.fade_in_out(5, 5, life);
    Next::Draw
}

/// -19's case of the second chain (main 0x001cb354).
pub fn heal_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    if e.cnt & 3 != 0 {
        return;
    }
    let old = e.flags;
    e.flags = old.wrapping_add(1);
    if old < i32::from(e.level) {
        let sync = e.pos_ptr;
        pfx::start(cx, HEAL_ROW, |g| {
            g.sync_pos_type = false;
            g.sync_pos = sync;
        });
    } else {
        e.end_flag = true;
    }
}

/// `effCure(ch)`, `effSanity(ch)`, `effResurrect(ch)`: controller `id`.
fn cure(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, id: i16) -> Option<usize> {
    let i = ctrl.new_effect(cx, id)?;
    let pos = cx.host.char_pos(ch);
    let h = cx.host.char_height(ch);
    let e = &mut ctrl.effects[i];
    e.life_time = 120;
    e.target = Some(ch);
    e.pos_t = pos;
    e.temp[0] = ee::mul(HALF, h);
    let over = ee::add(HUNDRED, h);
    pfx::start(cx, CURE_ROWS[(-id - 1) as usize], |g| {
        g.offset = [0, 0, over, 0];
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::CharPos(ch));
    });
    Some(i)
}

/// `effCure(ch)` (main 0x001cef50): an antidote.
pub fn eff_cure(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    cure(ctrl, cx, ch, CURE)
}

/// `effSanity(ch)` (main 0x001cf0d0): a restorative.
pub fn eff_sanity(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    cure(ctrl, cx, ch, SANITY)
}

/// `effResurrect(ch)` (main 0x001cedd0): a revival.
pub fn eff_resurrect(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    cure(ctrl, cx, ch, RESURRECT)
}

/// -1, -2 and -3's case of the second chain (main 0x001c9348).
pub fn cure_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let Some(t) = e.target.filter(|&t| cx.host.check_target(t)) else {
        e.end_flag = true;
        return;
    };
    let k = (-e.id - 1) as usize;
    let tb = &cx.spells.data.heal;
    let (rows, n) = match e.cnt {
        12 => ([tb.a[k], tb.b[k]], 2),
        55 => ([tb.c[k], 0], 1),
        30 => ([tb.d[k], 0], 1),
        _ => ([0, 0], 0),
    };
    let up = e.temp[0];
    for &row in &rows[..n] {
        pfx::start(cx, row as usize, |g| {
            g.offset = [0, 0, up, 0];
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::CharPos(t));
        });
    }
}

/// `effOpenBox(pos)` (main 0x001ced40): a generator 94 at `pos`.
pub fn eff_open_box(cx: &mut Cx, pos: V4) {
    pfx::start(cx, OPEN_BOX_ROW, |g| g.pos = pos);
}

/// `effRemoveTrap(pos, a, b)` (main 0x001d0980): the controller -6.
pub fn eff_remove_trap(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4, a: i32, b: i32) -> Option<usize> {
    let a = if a == -1 { 101 } else { a };
    let b = if b == -1 { 118 } else { b };
    let i = ctrl.new_effect(cx, REMOVE_TRAP)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 5;
    e.pos = pos;
    e.param = b;
    pfx::start(cx, REMOVE_TRAP_ROW, |g| {
        g.pos = pos;
        g.p_tex_mod = a as i16;
    });
    Some(i)
}

/// -6's case of the second chain (main 0x001c98a8).
pub fn remove_trap_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &ctrl.effects[i];
    if e.cnt != e.life_time {
        return;
    }
    let (pos, tex) = (e.pos, e.param as i16);
    pfx::start(cx, REMOVE_TRAP_END_ROW, |g| {
        g.pos = pos;
        g.p_tex_mod = tex;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::{ONE, k};
    use crate::host::{CharState, Simple};

    fn world(fx: &mut crate::Effects) {
        fx.ctrl.town = false;
    }

    /// Repth: the ring and one burst of sparks; a full heal (152): three,
    /// four frames apart.
    #[test]
    fn a_heal_bursts_once_a_level() {
        let Some(mut fx) = crate::testing::effects() else { return };
        world(&mut fx);
        let mut r = || 0;
        let eye = [0, k(-600.0), k(300.0), ONE];
        let cam = Camera { eye, cam_pos: eye, cam_view: [0, 0, k(100.0), ONE], ..Camera::default() };
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], cam);
        host.chars.push(CharState { id: 5, pos: [0, 0, 0, ONE], dirc: [0; 4], height: k(160.0), width: k(45.0) });
        for (sid, bursts) in [(150, 1), (152, 3)] {
            fx.heal_skill(&mut host, 5, sid).unwrap();
            let mut rows = Vec::new();
            for _ in 0..40 {
                fx.step(&mut host);
                rows.extend(fx.particles.started.drain(..).map(|g| g.row));
            }
            assert_eq!(rows.iter().filter(|&&r| r == HEAL_ROW).count(), bursts);
            assert!(fx.ctrl.effects.iter().all(|e| e.status == 0));
        }
    }

    /// A cure's generators: its own at the start, then A and B at 12, D
    /// at 30, C at 55.
    #[test]
    fn a_cure_runs_its_generators() {
        let Some(mut fx) = crate::testing::effects() else { return };
        world(&mut fx);
        let mut r = || 0;
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], Camera::default());
        host.chars.push(CharState { id: 5, pos: [0, 0, 0, ONE], dirc: [0; 4], height: k(160.0), width: k(45.0) });
        fx.cure(&mut host, 5).unwrap();
        let mut rows: Vec<usize> = fx.particles.started.drain(..).map(|g| g.row).collect();
        for _ in 0..130 {
            fx.step(&mut host);
            rows.extend(fx.particles.started.drain(..).map(|g| g.row));
        }
        assert_eq!(rows, [107, 99, 101, 102, 100]);
    }
}
