//! The tumbling debris: `ccEffect::Main`'s shared first-switch case
//! 0x001c4034 (ids -15, 9-11, 15-18, 25-28, 42-55, 66-78, 114-129, 134-149,
//! 155-163) and second-chain cases 0x001c7dd0 (ids -15, 9-11, 25-28,
//! 114-129, 138-149) and 0x001c839c (the ice rocks 15-18): spinning, falling,
//! and bouncing on the land it checks one frame in four. Also the spawners,
//! `effSmokeRock` (main 0x001cda30, rocks 9-11), `effIceRock` (0x001ceaa0)
//! and `effDarkSmoke` (0x001d4a20, controllers -15). The rules are in
//! docs/engine/effects.md ("ConvergenceSystem", "The ice rocks").

use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR, Obj};
use crate::{Cx, NO_HIT, VecRef, pfx, vu};

/// The dark smoke's controller (no object).
pub const DARK_SMOKE: i16 = -15;

/// The ids with the first-switch case.
pub fn has_motion(id: i16) -> bool {
    matches!(id, -15 | 9..=11 | 15..=18 | 25..=28 | 42..=55 | 66..=78 | 114..=129 | 134..=149 | 155..=163)
}

/// The ids with the second-chain case 0x001c7dd0.
pub fn has_bounce(id: i16) -> bool {
    matches!(id, -15 | 9..=11 | 25..=28 | 114..=129 | 138..=149)
}

/// The ice rocks `effIceRock` throws (CMP_x202a-d).
pub const ICE_ROCK_FIRST: i16 = 15;
pub const ICE_ROCK_LAST: i16 = 18;

/// `(short)(u16 + (short)RAD2DEG(a))`, back to radians.
fn spin(a: F, by: u16) -> F {
    ee::deg2rad(by.wrapping_add(ee::rad2deg(a) as u16) as i16)
}

/// 155-163's case of the second chain (main 0x001c834c): FadeOut(10,
/// lifeTime - 10).
pub fn radiate2_post(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let tt = i32::from(e.life_time.wrapping_sub(10));
    e.fade_out(10, tt);
}

/// The first-switch case 0x001c4034.
pub fn debris_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    e.rot[0] = spin(e.rot[0], e.rot_speed[0]);
    e.rot[2] = spin(e.rot[2], e.rot_speed[2]);
    e.pos = ee::vadd(e.pos, e.speed);
    if e.flags & 3 != 3 && ee::lt(e.pos[2], 0xc3fa_0000) {
        e.flags |= 3;
        e.cnt = 80;
        e.age = 80;
    }
    e.pos[3] = ONE;
    if e.flags & 1 == 0 {
        e.speed[2] = ee::sub(e.speed[2], 0x4000_0000);
    }
    Next::Draw
}

/// A bounce: the speeds and spins damped (x, y and the spins by `xy`, z by
/// `z`), stopped below their thresholds.
fn bounce(e: &mut crate::effect::Effect, still: i16, xy: F, z: F) {
    e.speed[0] = ee::mul(e.speed[0], xy);
    e.speed[1] = ee::mul(e.speed[1], xy);
    e.speed[2] = ee::mul(z, e.speed[2]);
    // fabs((double)x) < 4.0, 8.0: exact in single precision.
    if ee::lt(ee::fabsf(e.speed[0]), 0x4080_0000) {
        e.speed[0] = 0;
    }
    if ee::lt(ee::fabsf(e.speed[1]), 0x4080_0000) {
        e.speed[1] = 0;
    }
    if ee::lt(ee::fabsf(e.speed[2]), 0x4100_0000) {
        e.speed[2] = 0;
        e.flags |= 1;
    }
    let damp = |v: u16| ee::to_int(ee::mul(xy, ee::from_int(i32::from(v)))) as u16;
    e.rot_speed[0] = damp(e.rot_speed[0]);
    e.rot_speed[2] = damp(e.rot_speed[2]);
    if e.rot_speed[0] < 16 {
        e.rot_speed[0] = 0;
    }
    if e.rot_speed[2] < 16 {
        e.rot_speed[2] = 0;
    }
    if ee::eq(e.speed[0], 0) && ee::eq(e.speed[1], 0) && ee::eq(e.speed[2], 0) {
        e.rot_speed[2] = 0;
        e.rot_speed[0] = 0;
        e.cnt = still;
        e.age = still;
        e.flags |= 2;
    }
}

/// The second-chain case 0x001c7dd0.
pub fn debris_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    bounce_post(ctrl, cx, i, 0x3f4c_cccd, 0xbf00_0000);
}

/// The ice rocks 15-18's case of the second chain (main 0x001c839c): the
/// landing and bounce, 0.75 a bounce (the same code as 42-45's 0x001c9b44).
pub fn ice_rock_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    bounce_post(ctrl, cx, i, 0x3f40_0000, 0xbf40_0000);
}

/// The landing and bouncing second-chain cases (0x001c7dd0 with 0.8 and
/// -0.5, 0x001c9b44 and 0x001ca0a8 with 0.75 and -0.75).
pub fn bounce_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize, xy: F, zd: F) {
    const TIME: i32 = 10;
    let e = &mut ctrl.effects[i];
    let still = e.life_time.wrapping_sub(TIME as i16);
    if ee::lt(e.speed[2], 0xc040_0000) && (e.cnt & 3) as u32 == e.sn & 3 {
        let z = cx.host.land_hit_check2(e.pos, ee::sub(e.speed[2], 0x41a0_0000), 0x2000_0000);
        if e.flags & 1 != 0 || !ee::eq(z, NO_HIT) {
            e.pos[2] = z;
            bounce(e, still, xy, zd);
        }
    } else if e.cnt & 1 != 0 && e.flags & 3 == 1 {
        bounce(e, still, xy, zd);
    }
    let life = i32::from(e.life_time);
    e.fade_out(TIME, life);
}

/// The random direction `effSmokeRock` and `effDarkSmoke` send each piece
/// along: (0, -1, 0) turned by a random tilt, then by `r`; the draw's
/// `rand() >> 3`.
pub(crate) fn direction(cx: &mut Cx, r: V4) -> (i32, V4) {
    let rn = cx.host.rand() >> 3;
    let a = [0, ee::deg2rad(((rn << 8) & 0xfc00) as i16), ee::deg2rad(((rn & 0xf00) + 4096) as i16), 0];
    let m = vu::rot_zyx(&vu::UNIT, a);
    let d = ee::apply(&m, [0, 0xbf80_0000, 0, ONE]);
    let m = vu::rot_zyx(&vu::UNIT, r);
    (rn, ee::apply(&m, d))
}

/// `v (100 - (rn >> 8) % 50) / 100`.
pub(crate) fn speed_of(v: F, rn: i32) -> F {
    let h = 0x42c8_0000;
    ee::mul(v, ee::div(ee::sub(h, ee::from_int((rn >> 8) % 50)), h))
}

/// `effSmokeRock(p, r, v, _, n, x)` (main 0x001cda30): the last rock.
pub fn eff_smoke_rock(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, n: i32, x: i32) -> Option<usize> {
    let mut last = None;
    for _ in 0..n {
        let (rn, a) = direction(cx, r);
        last = ctrl.new_effect(cx, 9 + ((rn & 0xf) % 3) as i16);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.life_time = 90;
        e.pos = p;
        e.speed = ee::vscale(a, speed_of(v, rn));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0xf000) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0xf000) as u16;
        // SetFogSw(clump, 1).
        let (g1, g2) = if x == 0 { (189, -1) } else { (85, 86) };
        for row in [g1, g2] {
            if row <= 0 {
                continue;
            }
            pfx::start(cx, row as usize, |g| {
                g.sync_pos_type = false;
                g.sync_pos = Some(VecRef::EffectPos(k));
            });
        }
    }
    last
}

/// `effIceRock(p, r, v, s, n)` (main 0x001ceaa0): n ice rocks 15-18
/// (`rn & 3`) thrown out from `p` as `effSmokeRock` throws its rocks, life
/// 90, scale `s`, the rise of their speed cut to 0.6, spinning by `rn`'s
/// bits 8-11 and 12-15 (x and z); the last one. The bosses' ice throws
/// them: `ccBossEffIceBreak::Draw` (gcmn 0x0046cc10) and Innis's missile.
pub fn eff_ice_rock(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32) -> Option<usize> {
    let mut last = None;
    for _ in 0..n {
        let (rn, a) = direction(cx, r);
        last = ctrl.new_effect(cx, 15 + (rn & 3) as i16);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.life_time = 90;
        e.pos = p;
        e.scale = [s, s, s, ONE];
        e.speed = ee::vscale(a, speed_of(v, rn));
        e.speed[2] = ee::mul(e.speed[2], 0x3f19_999a);
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0xf00) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0xf00) as u16;
        // SetFogSw(clump, 1).
    }
    last
}

/// `effDarkSmoke(p, r, v, n)` (main 0x001d4a20): the last controller.
pub fn eff_dark_smoke(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, n: i32) -> Option<usize> {
    let mut last = None;
    for _ in 0..n {
        let (rn, a) = direction(cx, r);
        last = ctrl.new_effect(cx, DARK_SMOKE);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.life_time = 60;
        e.pos = p;
        e.speed = ee::vscale(a, speed_of(v, rn));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0xf000) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0xf000) as u16;
        pfx::start(cx, 189, |g| {
            g.p_tex_mod = 31;
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::EffectPos(k));
        });
    }
    last
}

/// A rotation (x) of -90 degrees: what the landings turn their smoke by.
pub fn down() -> V4 {
    let mut r = VF0;
    r[0] = ee::deg2rad(-16384);
    r
}

/// `(100 - (rn >> 8) % 50) / 100`: the share of the speed (and size) a
/// piece gets.
fn share(rn: i32) -> F {
    let h = 0x42c8_0000;
    ee::div(ee::sub(h, ee::from_int((rn >> 8) % 50)), h)
}

/// `effRadiateSomething(p, r, v, atr, n)` (main 0x001d4cd0): n pieces
/// thrown out from `p` the way `effSmokeRock` throws its rocks, life 25, by
/// element: soil 134-137 and water 114-117 (a random one of the four, half
/// size), fire 118, wind 119, thunder 120, dark 121 (full size, one random
/// pattern for all, fire, wind and thunder re-coloured with CLUT 151, 153,
/// 145 of `particleCcsAdrs`); the last piece.
#[allow(clippy::too_many_arguments)]
pub fn eff_radiate_something(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    p: V4,
    r: V4,
    v: F,
    atr: i32,
    n: i32,
) -> Option<usize> {
    radiate(ctrl, cx, p, r, v, atr, n, false)
}

/// `effRadiateSomething2(p, r, v, atr, n)` (main 0x001d5110): the same
/// with water 155-158, fire 159, wind 160, thunder 161, dark 162 and, for
/// no element, 163.
#[allow(clippy::too_many_arguments)]
pub fn eff_radiate_something2(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    p: V4,
    r: V4,
    v: F,
    atr: i32,
    n: i32,
) -> Option<usize> {
    radiate(ctrl, cx, p, r, v, atr, n, true)
}

#[allow(clippy::too_many_arguments)]
fn radiate(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, atr: i32, n: i32, second: bool) -> Option<usize> {
    const HALF: F = 0x3f00_0000;
    let mut pat = 0;
    let mut clut = 0;
    let mut pick = |mask: i32| (cx.host.rand() >> 3) & mask;
    let k = i16::from(second);
    let (id, s) = match atr & 0xfc {
        crate::spell::attr::SOIL => (134 + pick(3) as i16, HALF),
        crate::spell::attr::WATER => (if second { 155 } else { 114 } + pick(3) as i16, HALF),
        crate::spell::attr::FIRE => {
            pat = pick(7);
            clut = 151;
            (118 + 41 * k, ONE)
        }
        crate::spell::attr::WIND => {
            pat = pick(3);
            clut = 153;
            (119 + 41 * k, ONE)
        }
        crate::spell::attr::THUNDER => {
            pat = pick(7);
            clut = 145;
            (120 + 41 * k, ONE)
        }
        crate::spell::attr::DARK => {
            pat = pick(3);
            (121 + 41 * k, ONE)
        }
        _ if second => (163, ONE),
        // The id is a register the callers never leave unset.
        _ => return None,
    };
    let mut last = None;
    for _ in 0..n {
        let (rn, d) = direction(cx, r);
        last = ctrl.new_effect(cx, id);
        let Some(k) = last else { continue };
        let swap = if clut != 0 { cx.spells.data.particle_adrs(cx.assets, clut) } else { None };
        let e = &mut ctrl.effects[k];
        e.life_time = 25;
        e.pos = p;
        e.tex_anm_pat = pat as u16;
        let f = share(rn);
        e.scale = ee::vscale(ONE_VECTOR, ee::mul(s, f));
        e.speed = ee::vscale(d, ee::mul(v, f));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0xf000) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0xf000) as u16;
        if clut != 0
            && let Obj::Eff(f) = &mut e.obj
        {
            // Into the ccEff's ccTex.clutChunk (+0x3c).
            f.clut = swap;
        }
        // Water and soil: SetFogSw(clump, 1).
    }
    last
}

/// What an `effSmoke*` (main 0x001d6410-0x001d7374) makes: its id from the
/// draw, the smoke generator 85's `pTexMod` (none for the electric), and
/// whether it is the leaf's (a random turn of its sprite, fog on, no
/// spin).
struct Smoke {
    id: fn(i32) -> i16,
    tex: Option<i16>,
    leaf: bool,
}

/// The `effSmoke*` family: n pieces thrown out from `p` (as
/// `effSmokeRock`), life 60, speed and size shares of `v` and `s`, each
/// with generator 85 following it.
#[allow(clippy::too_many_arguments)]
fn smoke(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32, kind: Smoke) -> Option<usize> {
    const SMOKE_ROW: usize = 85;
    let mut last = None;
    for _ in 0..n {
        let (rn, d) = direction(cx, r);
        last = ctrl.new_effect(cx, (kind.id)(rn));
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.life_time = 60;
        e.pos = p;
        let f = share(rn);
        e.scale = ee::vscale(ONE_VECTOR, ee::mul(s, f));
        e.speed = ee::vscale(d, ee::mul(v, f));
        e.velocity = ee::mul(v, f);
        if kind.leaf {
            let a = ee::deg2rad(((cx.host.rand() >> 3) & 0xf100) as i16);
            if let Obj::Eff(eff) = &mut ctrl.effects[k].obj {
                eff.rotate = a;
                eff.prim = (eff.prim & !0x20) | 0x20;
            }
        } else {
            e.rot_speed[0] = ((rn >> 4) & 0xf000) as u16;
            e.rot_speed[2] = ((rn >> 8) & 0xf000) as u16;
        }
        // The ice: SetFogSw(clump, 1).
        if let Some(t) = kind.tex {
            pfx::start(cx, SMOKE_ROW, |g| {
                g.sync_pos_type = false;
                g.sync_pos = Some(VecRef::EffectPos(k));
                g.p_tex_mod = t;
            });
        }
    }
    last
}

/// `effSmokeIce(p, r, v, s, n)` (main 0x001d6410): ice 122-124 (one of
/// three at random), smoke `pTexMod` 110.
pub fn eff_smoke_ice(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32) -> Option<usize> {
    smoke(ctrl, cx, p, r, v, s, n, Smoke { id: |rn| 122 + ((rn & 0xf) % 3) as i16, tex: Some(110), leaf: false })
}

/// `effSmokeSparks(p, r, v, s, n)` (main 0x001d6750): sparks 126, smoke
/// `pTexMod` 151.
pub fn eff_smoke_sparks(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32) -> Option<usize> {
    smoke(ctrl, cx, p, r, v, s, n, Smoke { id: |_| 126, tex: Some(151), leaf: false })
}

/// `effSmokeLeaf(p, r, v, s, n)` (main 0x001d6a70): leaves 127, each
/// sprite turned at random and fogged, smoke `pTexMod` 1.
pub fn eff_smoke_leaf(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32) -> Option<usize> {
    smoke(ctrl, cx, p, r, v, s, n, Smoke { id: |_| 127, tex: Some(1), leaf: true })
}

/// `effSmokeElectric(p, r, v, s, n)` (main 0x001d6db0): sparks 128, no
/// smoke.
pub fn eff_smoke_electric(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32) -> Option<usize> {
    smoke(ctrl, cx, p, r, v, s, n, Smoke { id: |_| 128, tex: None, leaf: false })
}

/// `effSmokeSmoke(p, r, v, s, n)` (main 0x001d7060): smoke 129, smoke
/// `pTexMod` 31.
pub fn eff_smoke_smoke(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, s: F, n: i32) -> Option<usize> {
    smoke(ctrl, cx, p, r, v, s, n, Smoke { id: |_| 129, tex: Some(31), leaf: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::k;
    use crate::{CharRef, Host};

    /// Flat land at 0; rand an LCG.
    struct Land(u32);

    impl Host for Land {
        fn rand(&mut self) -> i32 {
            self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (self.0 >> 1) as i32
        }
        fn player_pos(&self) -> V4 {
            [0, 0, 0, ONE]
        }
        fn camera(&self) -> Camera {
            Camera::default()
        }
        fn char_pos(&self, _c: CharRef) -> V4 {
            [0, 0, 0, ONE]
        }
        fn char_dirc(&self, _c: CharRef) -> V4 {
            [0; 4]
        }
        fn char_height(&self, _c: CharRef) -> F {
            0
        }
        fn char_width(&self, _c: CharRef) -> F {
            0
        }
        fn land_hit_check2(&mut self, pos: V4, offset_z: F, _mask: u32) -> F {
            let (a, b) = (f32::from_bits(pos[2]), f32::from_bits(ee::add(pos[2], offset_z)));
            if a.min(b) <= 0.0 && 0.0 <= a.max(b) { 0 } else { NO_HIT }
        }
    }

    /// An IceBreak's middle throw (25, size 1, five rocks from 100 up):
    /// rocks 15-18 that land on the land, come to rest (flags 3) and fade
    /// out before their life of 90 is over.
    #[test]
    fn ice_rocks_land_rest_and_fade() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut host = Land(7);
        {
            let (ctrl, mut cx) = fx.split(&mut host);
            assert!(eff_ice_rock(ctrl, &mut cx, [0, 0, k(100.0), ONE], [0; 4], k(25.0), ONE, 5).is_some());
        }
        let rocks: Vec<usize> = (0..fx.ctrl.effects.len()).filter(|&i| fx.ctrl.effects[i].status != 0).collect();
        assert_eq!(rocks.len(), 5);
        assert!(rocks.iter().all(|&i| (ICE_ROCK_FIRST..=ICE_ROCK_LAST).contains(&fx.ctrl.effects[i].id)));
        let (mut rested, mut faded) = (0, false);
        for _ in 0..95 {
            fx.step(&mut host);
            for &i in &rocks {
                let e = &fx.ctrl.effects[i];
                if e.status != 0 && e.flags == 3 && e.pos[2] == 0 {
                    rested += 1;
                }
                faded |= e.status != 0 && e.transparency != ONE && e.transparency != 0;
            }
        }
        assert!(rested > 0 && faded);
        assert!(rocks.iter().all(|&i| fx.ctrl.effects[i].status == 0));
    }
}
