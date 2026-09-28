//! A physical skill's blow landing: `effPhysicalSkillHitShockWave(pos, attr)`
//! (main 0x001d2790), which `ccSkillCheckNote` (gcmn 0x00573a10) calls at the
//! skill's target position (`nowSkillPtr` +0x60) for each 0x8002 note of the
//! skill's animation, with the skill's element (`ccSkillParam` +0x2c `&
//! 0xfc`, matched exactly). It throws the element's smoke and radiate pieces,
//! shakes the camera and starts the waves 79, 80 and 81 in the element's
//! colours. The cases are in docs/engine/effects.md ("A physical skill's
//! blow").

use piney_data::volume::Volume;

use crate::ee::{self, F, V4};
use crate::effect::{EffectCtrl, Next};
use crate::spell::{attr, camera_shake, check_camera_shake_range};
use crate::{Cx, Event, debris};

pub const WAVE: i16 = 79;
pub const WAVE2: i16 = 80;
pub const WAVE3: i16 = 81;
/// `ccSeOn3D(35, p)`.
pub const SE_SHOCK: i32 = 35;

/// 30.0, 0.5, 1.0, 2.0.
const THIRTY: F = 0x41f0_0000;
const HALF: F = 0x3f00_0000;
const ONE: F = 0x3f80_0000;
const TWO: F = 0x4000_0000;
/// 80's rise with param 1: 12.6.
const RISE: F = 0x4149_999a;

/// The waves' tables (gp, loaded onto `Main`'s stack).
#[derive(Clone, Debug)]
pub struct Tables {
    /// 80 (main 0x00377ef0): x, y from `t[p]` to `t[p + 2]`, z from
    /// `t[p + 4]` to `t[p + 6]`; its turn a frame by param (0x00377f10,
    /// two shorts).
    pub wave2: [F; 8],
    pub wave2_turn: [i16; 2],
    /// 81 (0x00377f18): x, y from `t[p]` to `t[p + 2]`, z from `t[p + 4]`
    /// to `t[p + 8]`; its turns (0x00377f40).
    pub wave3: [F; 10],
    pub wave3_turn: [i16; 2],
}

impl Tables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Tables {
        use crate::spell::{bits, first};
        let t = piney_data::tables::effect::of(volume);
        Tables {
            wave2: bits(t.wave2()),
            wave2_turn: first(t.wave2_turn()),
            wave3: bits(t.wave3()),
            wave3_turn: first(t.wave3_turn()),
        }
    }
}

/// `from + cnt * ((to - from) / life)`.
fn grow(from: F, to: F, life: i16, cnt: i16) -> F {
    let step = ee::div(ee::sub(to, from), ee::from_int(i32::from(life)));
    ee::add(from, ee::mul(ee::from_int(i32::from(cnt)), step))
}

/// A duplicated clump's CLUT swap, as (from, to).
fn clump_clut(cx: &Cx, new: usize, old: usize) -> Option<(u32, u32)> {
    let d = &cx.spells.data;
    d.particle_adrs(cx.assets, old).zip(d.particle_adrs(cx.assets, new)).map(|(f, t)| (f.object, t.object))
}

/// `effPhysicalSkillHitShockWave(pos, attr)` (main 0x001d2790): the last
/// wave.
pub fn eff_physical_skill_hit_shock_wave(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, atr: i32) -> Option<usize> {
    let down = debris::down();
    let (cluts, radiate) = match atr {
        attr::SOIL => {
            debris::eff_smoke_rock(ctrl, cx, p, down, THIRTY, 8, -1);
            ([167, 219, 212], true)
        }
        attr::WATER => {
            debris::eff_smoke_ice(ctrl, cx, p, down, THIRTY, HALF, 8);
            ([0, 0, 0], true)
        }
        attr::FIRE => {
            debris::eff_smoke_sparks(ctrl, cx, p, down, THIRTY, TWO, 8);
            ([162, 218, 211], true)
        }
        attr::WIND => {
            debris::eff_smoke_leaf(ctrl, cx, p, down, THIRTY, ONE, 16);
            ([164, 220, 213], true)
        }
        attr::THUNDER => {
            debris::eff_smoke_electric(ctrl, cx, p, down, THIRTY, ONE, 16);
            ([163, 219, 212], true)
        }
        attr::DARK => {
            debris::eff_smoke_smoke(ctrl, cx, p, down, THIRTY, TWO, 8);
            ([165, 221, 214], true)
        }
        _ => ([166, 222, 215], false),
    };
    if radiate {
        debris::eff_radiate_something(ctrl, cx, p, down, THIRTY, atr, 8);
    }
    if check_camera_shake_range(cx, p) {
        camera_shake(cx, 0, 2, 10, 0);
    }
    cx.raise(Event::Sound3d { se: SE_SHOCK, pos: p });
    let mut last = None;
    for k in 0..5 {
        let (id, life, param, clut, old) = match k {
            0 => (WAVE, 15, 0, cluts[0], 161),
            1 | 2 => (WAVE2, 15, k - 1, cluts[1], 217),
            _ => (WAVE3, 20, k - 3, cluts[2], 210),
        };
        last = ctrl.new_effect(cx, id);
        let Some(i) = last else { continue };
        let swap = if clut != 0 { clump_clut(cx, clut, old) } else { None };
        let e = &mut ctrl.effects[i];
        e.life_time = life;
        e.param = param;
        e.level = 1;
        e.pos = p;
        if clut != 0 {
            e.clut_swap = swap;
        }
    }
    last
}

/// 79's case of the first switch (main 0x001c4b74).
pub fn wave_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let xy = grow(HALF, 0x4033_3333, e.life_time, e.cnt);
    e.scale[0] = xy;
    e.scale[1] = xy;
    e.scale[2] = grow(HALF, 0x3fc0_0000, e.life_time, e.cnt);
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(1092));
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 5, life);
    Next::Draw
}

/// 80's case of the first switch (main 0x001c4c4c).
pub fn wave2_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let t = &cx.spells.data.shock;
    let e = &mut ctrl.effects[i];
    let p = e.param as usize;
    let xy = grow(t.wave2[p], t.wave2[p + 2], e.life_time, e.cnt);
    e.scale[0] = xy;
    e.scale[1] = xy;
    e.scale[2] = grow(t.wave2[p + 4], t.wave2[p + 6], e.life_time, e.cnt);
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(t.wave2_turn[p]));
    if e.param != 0 {
        e.pos[2] = ee::add(e.pos[2], RISE);
    }
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 5, life);
    Next::Draw
}

/// 81's case of the first switch (main 0x001c4da4).
pub fn wave3_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let t = &cx.spells.data.shock;
    let e = &mut ctrl.effects[i];
    let p = e.param as usize;
    let xy = grow(t.wave3[p], t.wave3[p + 2], e.life_time, e.cnt);
    e.scale[0] = xy;
    e.scale[1] = xy;
    e.scale[2] = grow(t.wave3[p + 4], t.wave3[p + 8], e.life_time, e.cnt);
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(t.wave3_turn[p]));
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 5, life);
    Next::Draw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::k;
    use crate::host::Simple;

    /// A fire blow: the sparks and radiate pieces, then the five waves
    /// (79, two 80, two 81) last; the sound; all gone within 100 frames.
    #[test]
    fn a_fire_blow_throws_its_waves() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut n = 0;
        let mut r = || {
            n += 12345;
            n
        };
        let mut host = Simple::new(&mut r, [0, 0, 0, crate::ONE], Camera::default());
        let p = [0, 0, k(60.0), crate::ONE];
        let last = fx.shock_wave(&mut host, p, attr::FIRE).unwrap();
        let live: Vec<i16> = fx.ctrl.effects.iter().filter(|e| e.status != 0).map(|e| e.id).collect();
        assert_eq!(&live[live.len() - 5..], [WAVE, WAVE2, WAVE2, WAVE3, WAVE3]);
        assert_eq!(fx.ctrl.effects[last].id, WAVE3);
        assert!(fx.take_events().contains(&Event::Sound3d { se: SE_SHOCK, pos: p }));
        for _ in 0..100 {
            fx.step(&mut host);
        }
        assert!(fx.ctrl.effects.iter().all(|e| e.status == 0));
    }
}
