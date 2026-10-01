//! Smoke and dust: `effSmoke` (main 0x001ce300), one particle of smoke on
//! effect.cpp's static `ccpgSmoke`; gcmn's enemy dust built on it
//! (`ccEnemyEffDust` 0x0043a3e0, `ccEnemyEffDustRing` 0x0043a590 and
//! 0x0043a7a0), which an enemy's feet, a gold goblin's run and an idol
//! opening raise; and a runner's dust, `ccEffPawSmoke` (main 0x001ce640) on
//! `ccpgPawSmoke`. The rules are in docs/engine/effects.md ("Smoke and
//! dust").

use crate::ee::{self, F, ONE, V4};
use crate::{Cx, particle, portal, space, vu};

/// `ccParticleSetup`'s fades for dust: `fadeInD` 512, `fadeOutD` 32.
pub const DUST_FADE_IN: i16 = 512;
pub const DUST_FADE_OUT: i16 = 32;
/// The three-argument `ccEnemyEffDust`'s life and texture.
pub const DUST_LIFE: i32 = 30;
pub const DUST_TEX: i32 = 109;

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;

/// `effSmoke(pos, v, s, life, t, in, out)` (main 0x001ce300): whether a
/// particle was free.
#[allow(clippy::too_many_arguments)]
pub fn eff_smoke(cx: &mut Cx, pos: V4, v: V4, s: F, life: i32, t: i32, fade_in: i16, fade_out: i16) -> bool {
    smoke(cx, pos, v, s, life, t, (fade_in, fade_out), true)
}

/// `effSmokeN(pos, v, s, life, t, in, out)` (main 0x001ce490): `effSmoke`
/// with the particle's `distSW` cleared.
#[allow(clippy::too_many_arguments)]
pub fn eff_smoke_n(cx: &mut Cx, pos: V4, v: V4, s: F, life: i32, t: i32, fade_in: i16, fade_out: i16) -> bool {
    smoke(cx, pos, v, s, life, t, (fade_in, fade_out), false)
}

/// `effSmoke`'s and `effSmokeN`'s puff, `dist` whether the particle keeps
/// `distSW` as the generator set it.
#[allow(clippy::too_many_arguments)]
fn smoke(cx: &mut Cx, pos: V4, v: V4, s: F, life: i32, t: i32, (fade_in, fade_out): (i16, i16), dist: bool) -> bool {
    let rn = cx.host.rand() >> 3;
    let l = ee::from_int(life);
    let cut = ee::div(ee::mul(ee::mul(0x3f00_0000, l), ee::from_int(rn % 101)), 0x42c8_0000);
    let life = ee::to_int(ee::sub(l, cut));
    let smoke = cx.particles.statics[0];
    let Some(k) = particle::cc_particle_setup(cx, t, pos, life as i16, 0, smoke) else { return false };
    let p = &mut cx.particles.slots[k];
    if !dist {
        p.dist_sw = false;
    }
    p.velocity = v;
    // sqrtf, then fabs through doubles.
    p.speed = ee::fabsf(ee::sqrtf_on(cx.assets.volume, ee::dot(v, v)));
    p.size = s;
    p.scale[1] = s;
    p.scale[0] = s;
    p.fade_in_d = fade_in;
    p.fade_out_d = fade_out;
    p.rotate[3] = ((rn >> 3) & 0xfff0) as u16;
    true
}

/// A puff of `ccEnemyEffDust` or the ring: at `p` plus M (`out`, 0, 0),
/// flying M (`fly`, 0, 0) and s / 4 up.
#[allow(clippy::too_many_arguments)]
fn puff(cx: &mut Cx, p: V4, m: &vu::M4, out: F, fly: F, s: F, life: i32, t: i32) {
    let mut at = ee::apply(m, [out, 0, 0, ONE]);
    at[0] = ee::add(at[0], p[0]);
    at[1] = ee::add(at[1], p[1]);
    at[2] = ee::add(at[2], p[2]);
    at[3] = ONE;
    let mut v = ee::apply(m, [fly, 0, 0, ONE]);
    v[2] = ee::add(v[2], ee::div(s, 0x4080_0000));
    eff_smoke(cx, at, v, s, life, t, DUST_FADE_IN, DUST_FADE_OUT);
}

/// `ccEnemyEffDust(p, n, s, life, t)` (gcmn 0x0043a3e0).
pub fn enemy_eff_dust(cx: &mut Cx, p: V4, n: i32, s: F, life: i32, t: i32) {
    for _ in 0..n {
        let a = portal::rand_f(cx.host, PI);
        let m = vu::rot_z(&vu::UNIT, a);
        puff(cx, p, &m, ee::mul(0x4120_0000, s), ee::div(s, 0x4000_0000), s, life, t);
    }
}

/// `ccEnemyEffDustRing(p, s, r, n, life, t)` (gcmn 0x0043a590).
pub fn enemy_eff_dust_ring(cx: &mut Cx, p: V4, s: F, r: F, n: i32, life: i32, t: i32) {
    if n < 4 {
        enemy_eff_dust(cx, p, n, s, life, t);
        return;
    }
    let mut a = portal::rand_f(cx.host, PI);
    let step = ee::div(TWO_PI, ee::from_int(n));
    for _ in 0..n {
        let m = vu::rot_z(&vu::UNIT, a);
        puff(cx, p, &m, r, s, s, life, t);
        a = ee::add(a, step);
        if !ee::le(a, PI) {
            a = ee::sub(a, TWO_PI);
        }
    }
}

/// `ccEnemyEffDustRing(ch, ofs, s, r, n, life, t)` (gcmn 0x0043a7a0) for a
/// character at `pos` facing `dirc`.
#[allow(clippy::too_many_arguments)]
pub fn enemy_eff_dust_ring_at(cx: &mut Cx, pos: V4, dirc: V4, ofs: V4, s: F, r: F, n: i32, life: i32, t: i32) {
    let o = ee::apply(&vu::rot_zyx(&vu::UNIT, dirc), ofs);
    let mut p = ee::vadd(space::fw2lw(pos, cx.host.player_pos(), cx.host.bounds()), o);
    p[3] = ONE;
    enemy_eff_dust_ring(cx, p, s, r, n, life, t);
}

/// The grounds `ccEffPawSmoke` raises no dust from (water, grass).
const NO_DUST: [u32; 5] = [0x00b0_c000, 0x00c0_d000, 0x0060_b0d0, 0x0070_c0e0, 0x0080_80f0];
/// The grey and dark grounds: texture 4 (else 133).
const DARK: [u32; 11] = [
    0x0030_4050,
    0x0040_5060,
    0x0050_7080,
    0x0070_90a0,
    0x0090_b0c0,
    0x00c0_c0c0,
    0x00d0_d0d0,
    0x00e0_e0e0,
    0x0040_4040,
    0x0050_5050,
    0x0060_6060,
];

/// `ccEffPawSmoke(ch, speed)` (main 0x001ce640) with the feet's places.
pub fn eff_paw_smoke(cx: &mut Cx, feet: [V4; 2], dirc_z: F, speed: F) -> bool {
    let m = vu::rot_z(&vu::UNIT, dirc_z);
    let [mut l, mut r] = feet;
    l[3] = ONE;
    r[3] = ONE;
    let p = if ee::lt(l[2], r[2]) { l } else { r };
    let a = cx.host.ground_attribute(p) & 0x00f0_f0f0;
    if NO_DUST.contains(&a) {
        return false;
    }
    let t = if DARK.contains(&a) { 4 } else { 133 };
    let k = cx.host.rand() % 5;
    let y = ee::mul(0x3dcc_cccd, ee::mul(ee::mul(0x3d19_999a, ee::mul(0xbf80_0000, speed)), ee::from_int(10 - k)));
    let v = ee::apply(&m, [0, y, 0, ONE]);
    let rn = cx.host.rand() >> 3;
    let cut = ee::div(ee::mul(0x40a0_0000, ee::from_int(rn % 101)), 0x42c8_0000);
    let life = ee::to_int(ee::sub(0x4120_0000, cut));
    let paw = cx.particles.statics[1];
    let Some(s) = particle::cc_particle_setup(cx, t, p, life as i16, 0, paw) else { return false };
    let q = &mut cx.particles.slots[s];
    q.velocity = v;
    q.speed = ee::fabsf(ee::sqrtf_on(cx.assets.volume, ee::dot(v, v)));
    q.size = 0x3fc0_0000;
    q.scale[1] = 0x3fc0_0000;
    q.scale[0] = 0x3fc0_0000;
    q.fade_in_d = 1024;
    q.fade_out_d = 64;
    q.rotate[3] = ((rn >> 3) & 0xfff0) as u16;
    true
}
