//! The EE's single-precision arithmetic as the field code uses it: the FPU's
//! scalar operations ([`piney_data::field::ee`]), newlib's maths
//! ([`piney_data::libm`]) and `libvu0`'s macro-mode vector routines, on raw bit
//! patterns so the results are the game's to the bit. VU0 follows the FPU's
//! rules here (no denormals, no infinities, truncated); its multiply-adds round
//! the product first.

pub use piney_data::field::ee::{add, cmp, div, eq, from_int, le, lt, mul, sqrt, sub, to_int};
pub use piney_data::libm::{atan2f, cosf, dsqrt_on, fabsf, fmodf, neg, sinf, sqrtf, sqrtf_on, tanf};
pub use piney_data::vu0::{F, V4, apply, cross, dot, normalize, vadd, vscale, vscale_xyz, vsub};

/// `sqrt(dx dx + dy dy)` in double (`mula`, `madd` in singles, then
/// `fptodp`, `sqrt`, `dptofp`): the ground distance the statics, objects,
/// clouds and fireflies measure; `sqrt.s` from Outbreak on ([`dsqrt_on`]).
pub fn ground_dist(volume: piney_data::volume::Volume, dx: F, dy: F) -> F {
    dsqrt_on(volume, add(mul(dx, dx), mul(dy, dy)))
}

pub const ZERO: F = 0;
pub const ONE: F = 0x3f80_0000;
/// `3.14159274` (0x40490fdb), the code's pi.
pub const PI: F = 0x4049_0fdb;
/// 32768.0 (0x47000000), half a turn in `RAD2DEG`'s units.
pub const HALF_TURN: F = 0x4700_0000;

/// `vf0`: (0, 0, 0, 1).
pub const VF0: V4 = [0, 0, 0, ONE];

/// A constant's bits.
pub const fn k(x: f32) -> F {
    x.to_bits()
}

/// `vftoi12(x)`: fixed point with 12 fraction bits, as the 16 bits a
/// material's UV offset keeps.
pub fn vftoi12(x: F) -> u16 {
    (to_int(mul(x, 0x4580_0000)) & 0xffff) as u16
}

/// A vector's x, y, z as floats, for drawing.
pub fn f3(v: V4) -> [f32; 3] {
    [f(v[0]), f(v[1]), f(v[2])]
}

/// Bits to an `f32`, for reporting and drawing.
pub fn f(x: F) -> f32 {
    f32::from_bits(x)
}

/// `sceVu0RotTransPers(out, m, v, 0)`: [`apply`], x, y and z times
/// `Q = 1 / w`, then `vftoi4` (fixed point with 4 fraction bits) of all
/// four.
pub fn rot_trans_pers(m: &[V4; 4], v: V4) -> [i32; 4] {
    let p = apply(m, v);
    let q = div(ONE, p[3]);
    let s = |x: F| to_int(mul(x, 0x4180_0000));
    [s(mul(p[0], q)), s(mul(p[1], q)), s(mul(p[2], q)), s(p[3])]
}

/// `RAD2DEG` (`INF SLUS_202.67:0x001dab50`): radians to the game's 16-bit
/// angle (32768 is pi), `(short)(int)(32768 (pi + r) / pi - 32768)`.
pub fn rad2deg(r: F) -> i16 {
    let x = div(mul(HALF_TURN, add(PI, r)), PI);
    to_int(sub(x, HALF_TURN)) as i16
}

/// `DEG2RAD` (0x001dabb0): `pi * s / 32768`.
pub fn deg2rad(s: i16) -> F {
    div(mul(PI, from_int(i32::from(s))), HALF_TURN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_round_trip_in_sixteen_bits() {
        assert_eq!(rad2deg(0), 0);
        // Truncation: pi/2 as a float lands one short (checked in eemu).
        assert_eq!(rad2deg(k(std::f32::consts::FRAC_PI_2)), 16383);
        assert_eq!(rad2deg(ONE), 10430);
        assert_eq!(rad2deg(neg(PI)), -32768);
        assert_eq!(deg2rad(-32768), neg(PI));
        assert_eq!(f(deg2rad(16384)), std::f32::consts::FRAC_PI_2);
    }
}
