//! The arrival: `effTransfer` (main 0x001ce020), the rings and sparks
//! around a character who arrives in a town or a field (Kite's act 13 at
//! its count 30), and `effTransferRing` (0x001cdef0).
//!
//! ```text
//! effTransfer(ch)        effect 4 (no object): target ch, posT its pos,
//!                        lifeTime 12, temp[0] 20 + its height (Kite 180);
//!                        ccSeOn3D(76, its pos)
//!   its second-chain case, by cnt:
//!     0, 3, 6            effTransferRing(ch, temp[0])
//!     10                 a particle generator (particleGeneratorTbl[82]) on
//!                        ch's pos, offset (0, 0, temp[0], 0)
//! effTransferRing(ch, h) effect 3 (CMP_x032, a clump): lifeTime 40, scale
//!                        (0, 0, 0, 1), offset (0, 0, h, 1), turned about z by
//!                        DEG2RAD(rand() & 0x3f00); target ch, posT its pos
//!   first switch:        scale 0.1 + 0.9 cnt / 10 in all four lanes up to
//!                        cnt 10, then 1; FadeInOut(10, 5, lifeTime); pos =
//!                        ch's pos, z plus offset.z
//!   second chain:        at cnt 10 speed.z = (20 - offset.z) / (lifeTime -
//!                        15); offset.z += speed.z while cnt < lifeTime - 5;
//!                        rot.z on by 2048 (11.25 degrees) every frame
//! ```
//!
//! So three rings appear at the head three frames apart, spin, sink to 20
//! over the feet in their last 25 frames, and fade out over the last 5.

use crate::ee::{self, F, ONE};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR};
use crate::{CharRef, Cx, Event};

pub const TRANSFER: i16 = 4;
pub const TRANSFER_RING: i16 = 3;
pub const WARP_TRANSFER: i16 = -25;
/// `ccSeOn3D(76, pos)`.
pub const SE_TRANSFER: i32 = 76;
/// `particleGeneratorTbl` row the arrival starts at its count 10
/// (0x003414e0).
pub const GENERATOR_ROW: usize = 82;

/// `effTransfer(ch)` (main 0x001ce020).
pub fn eff_transfer(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let i = ctrl.new_effect(cx, TRANSFER)?;
    let pos = cx.host.char_pos(ch);
    let e = &mut ctrl.effects[i];
    e.target = Some(ch);
    e.pos_t = pos;
    e.life_time = 12;
    e.temp[0] = ee::add(0x41a0_0000, cx.host.char_height(ch));
    cx.events.push(Event::Sound3d { se: SE_TRANSFER, pos });
    Some(i)
}

/// `effWarpTransfer(ch)` (main 0x001ce120): effect -25, the sparks
/// without the rings: target `ch`, posT its pos, lifeTime 2, temp[0] 20 +
/// its height; `ccSeOn3D(76, its pos)`. Kite's act 13 starts it instead of
/// `effTransfer` (at count 0 rather than 30) when `WORLD_MAN` +0x168 is set.
pub fn eff_warp_transfer(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let i = ctrl.new_effect(cx, WARP_TRANSFER)?;
    let pos = cx.host.char_pos(ch);
    let e = &mut ctrl.effects[i];
    e.target = Some(ch);
    e.pos_t = pos;
    e.life_time = 2;
    e.temp[0] = ee::add(0x41a0_0000, cx.host.char_height(ch));
    cx.events.push(Event::Sound3d { se: SE_TRANSFER, pos });
    Some(i)
}

/// Effect -25's case of the second chain (main 0x001c92a4): the sparks at
/// count 0.
pub fn warp_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &ctrl.effects[i];
    if e.cnt != 0 {
        return;
    }
    let (target, h) = (e.target, e.temp[0]);
    sparks(cx, target, h);
}

/// `new ccParticleGenerator(&particleGeneratorTbl[82], 0, 0, 0, 0)` on
/// the character's pos, `h` over it; `startParticleGenerator`.
fn sparks(cx: &mut Cx, target: Option<CharRef>, h: F) {
    let mut g = cx.particles.generator(cx.assets, GENERATOR_ROW);
    g.sync_pos = target.map(crate::VecRef::CharPos);
    g.sync_pos_type = false;
    g.offset = [0, 0, h, 0];
    cx.particles.start(g);
}

/// `effTransferRing(ch, height)` (main 0x001cdef0).
pub fn eff_transfer_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, height: F) -> Option<usize> {
    let i = ctrl.new_effect(cx, TRANSFER_RING)?;
    let turn = ee::deg2rad((cx.host.rand() & 0x3f00) as i16);
    let pos = cx.host.char_pos(ch);
    let e = &mut ctrl.effects[i];
    e.life_time = 40;
    e.scale = [0, 0, 0, ONE];
    e.offset = [0, 0, height, ONE];
    e.rot = [0, 0, turn, ONE];
    e.target = Some(ch);
    e.pos_t = pos;
    Some(i)
}

/// Effect 3's case of the first switch (main 0x001c4248).
pub fn ring_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    const TENTH: F = 0x3dcc_cccd;
    let e = &mut ctrl.effects[i];
    if e.cnt < 10 {
        let s = ee::add(TENTH, ee::div(ee::mul(ee::sub(ONE, TENTH), ee::from_int(i32::from(e.cnt))), 0x4120_0000));
        e.scale = ee::vscale(ONE_VECTOR, s);
    } else if e.cnt == 10 {
        e.scale = ONE_VECTOR;
    }
    let life = i32::from(e.life_time);
    e.fade_in_out(10, 5, life);
    if let Some(t) = e.target {
        e.pos = cx.host.char_pos(t);
    }
    e.pos[2] = ee::add(e.pos[2], e.offset[2]);
    Next::Draw
}

/// Effect 3's case of the second chain (main 0x001c90f0).
pub fn ring_post(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let life = i32::from(e.life_time);
    if e.cnt == 10 {
        e.speed[2] = ee::div(ee::sub(0x41a0_0000, e.offset[2]), ee::from_int(life - 10 - 5));
    }
    if i32::from(e.cnt) < life - 5 {
        e.offset[2] = ee::add(e.offset[2], e.speed[2]);
    }
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(2048));
}

/// Effect 4's case of the second chain (main 0x001c9194).
pub fn transfer_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let (cnt, target, h) = {
        let e = &ctrl.effects[i];
        (e.cnt, e.target, e.temp[0])
    };
    let Some(t) = target else { return };
    match cnt {
        0 | 3 | 6 => {
            eff_transfer_ring(ctrl, cx, t, h);
        }
        10 => sparks(cx, Some(t), h),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Camera, DrawRec};
    use crate::host::{CharState, Simple};

    /// Kite arriving in Mac Anu: the sound at once, three rings three
    /// frames apart (each a rand() draw), the generator at count 10, the
    /// rings sinking to 20 over his feet, everything over in 52 frames.
    #[test]
    fn kite_arrives() {
        let Some(mut fx) = crate::testing::effects() else { return };
        let mut rng = piney_world::Rand(1);
        let mut draws = 0;
        let mut next = || {
            draws += 1;
            rng.rand()
        };
        let kite = [0, 0x45af_0000, 0x4416_0000, ONE];
        let cam = [0, 0x45be_a000, 0x4443_0000, ONE];
        let camera = Camera { eye: cam, cam_pos: cam, cam_view: kite, ..Camera::default() };
        let mut host = Simple::new(&mut next, kite, camera);
        host.chars.push(CharState { id: 0, pos: kite, dirc: [0; 4], height: 0x4320_0000, width: 0x4234_0000 });
        assert_eq!(fx.transfer(&mut host, 0), Some(0));
        assert_eq!(fx.take_events(), [Event::Sound3d { se: SE_TRANSFER, pos: kite }]);
        assert_eq!(ee::f(fx.ctrl.effects[0].temp[0]), 180.0);
        let mut rings = Vec::new();
        let mut last = 0;
        for f in 0..60 {
            fx.step(&mut host);
            for (i, e) in fx.ctrl.effects.iter().enumerate() {
                if e.status != 0 && e.id == TRANSFER_RING && e.cnt == 1 {
                    rings.push((f, i));
                }
            }
            if f == 10 {
                assert_eq!(fx.particles.started.len(), 1);
                assert_eq!(fx.particles.started[0].offset[2], 0x4334_0000);
            }
            if fx.ctrl.effects.iter().any(|e| e.status != 0) {
                last = f;
            }
            let drawn = fx.draws().iter().filter(|d| matches!(d, DrawRec::Clump { .. })).count();
            assert!(drawn <= 3);
        }
        assert_eq!(rings, [(0, 1), (3, 2), (6, 3)]);
        assert_eq!(last, 47);
        drop(host);
        assert_eq!(draws, 3);
    }
}
