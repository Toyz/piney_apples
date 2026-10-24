//! The pictures of Macha's effects and dust (QUA gcmn bosseff.cpp): the
//! eyes' lasers (`ccBossEffLaserShot::Draw` 0x00374440) and the light
//! ball's blast (`ccBossEffLightBallShock::Draw` 0x003748a0), both clumps
//! of xeffect drawn as the rules' last `Draw` left them, and the bursts,
//! rocks and dust those and the back dash raise. The rules own every step
//! and `ccRand` (`piney_battle::boss::macha::eff`).

use piney_battle::boss::macha::Pic;

use crate::draw::DrawRec;
use crate::ee::{self, ONE, VF0};
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::{Cx, debris, dust, particle, vu};

const FIVE: u32 = 0x40a0_0000;
const TENTH: u32 = 0x3dcc_cccd;
const NEG_HALF_PI: u32 = 0xbfc9_0fdb;

/// What a picture of Macha's asks of the effects.
pub fn calls(ctrl: &mut EffectCtrl, cx: &mut Cx, p: &Pic) {
    let down = ee::deg2rad(-16384);
    match *p {
        // The bolt at a tenth: Rx(-pi/2) Rx(pitch) Rz(heading), then the
        // place.
        Pic::Laser { pos, pitch, heading } => {
            let mut m = vu::UNIT;
            (m[0][0], m[1][1], m[2][2]) = (TENTH, TENTH, TENTH);
            let m = vu::rot_x(&m, NEG_HALF_PI);
            let m = vu::rot_x(&m, pitch);
            let m = vu::rot_z(&m, heading);
            let m = vu::trans(&m, pos);
            if let Some(obj) = cx.assets.find("xeffect", "CMP_ex31les0") {
                cx.draws.push(DrawRec::Clump { obj, matrix: m, alpha: ONE, layer: EFFECT_LAYER, clut: None });
            }
        }
        // ccParticleExplode(pos, (0, 0, 5), 5, 1), (0, 0, 2), then
        // effSmokeRock(pos, (-pi/2, 0, turn), 45, 2, 2, -1).
        Pic::LaserHit { pos, turn } => {
            for z in [FIVE, 0x4000_0000] {
                particle::cc_particle_explode(cx, pos, [0, 0, z, ONE], FIVE, 1);
            }
            debris::eff_smoke_rock(ctrl, cx, pos, [down, 0, turn, ONE], 0x4234_0000, 2, -1);
        }
        // The blast at its scale and transparency, unturned.
        Pic::Blast { pos, scale, transparency } => {
            let m = vu::pos_rot_zyx_scale(pos, VF0, [scale, scale, scale, ONE]);
            if let Some(obj) = cx.assets.find("xeffect", "CMP_ex31exp1") {
                cx.draws.push(DrawRec::Clump { obj, matrix: m, alpha: transparency, layer: EFFECT_LAYER, clut: None });
            }
        }
        // ccParticleExplode(pos, 0, size, kind), effSmoke(pos, 0, size, 30,
        // 31, 512, 32), effSmokeRock(pos, (-pi/2, 0, 0), rock, 0, 1, -1).
        Pic::Burst { pos, rock, size, kind } => {
            particle::cc_particle_explode(cx, pos, VF0, size, kind);
            dust::eff_smoke(cx, pos, VF0, size, 30, 31, 512, 32);
            debris::eff_smoke_rock(ctrl, cx, pos, [down, 0, 0, ONE], rock, 1, -1);
        }
        // The back dash: effSmoke(pos, vel, 8, 60, 114, 512, 32).
        Pic::Smoke { pos, vel } => {
            dust::eff_smoke(cx, pos, vel, 0x4100_0000, 60, 114, 512, 32);
        }
        // The ball, its light and the wave: the field's (the boss's file).
        Pic::Ball { .. } | Pic::Light(_) | Pic::Wave { .. } => {}
    }
}
