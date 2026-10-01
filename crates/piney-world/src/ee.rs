//! The EE's single-precision arithmetic as the field code uses it: the FPU's
//! scalar operations ([`piney_data::field::ee`]), newlib's maths
//! ([`piney_data::libm`]) and `libvu0`'s macro-mode vector routines, on raw bit
//! patterns so the results are the game's to the bit. VU0 follows the FPU's
//! rules here (no denormals, no infinities, truncated); its multiply-adds round
//! the product first.

pub use piney_data::field::ee::{add, cmp, div, from_int, le, lt, mul, sqrt, sub, to_int};
pub use piney_data::libm::{atan2f, cosf, dsqrt_on, fabsf, fmodf, neg, sinf, sqrtf, sqrtf_on, tanf};

/// A float as bits.
pub type F = u32;
/// A `sceVu0FVECTOR`: x, y, z, w as bits.
pub type V4 = [F; 4];

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

/// Bits to an `f32`, for reporting and drawing.
pub fn f(x: F) -> f32 {
    f32::from_bits(x)
}

/// `c.eq.s`: equal values (every zero pattern equals every other).
pub fn eq(a: F, b: F) -> bool {
    cmp(a, b) == std::cmp::Ordering::Equal
}

/// `sceVu0AddVector` (0x00110888): every lane.
pub fn vadd(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| add(a[i], b[i]))
}

/// `sceVu0SubVector` (0x001108a0): every lane.
pub fn vsub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| sub(a[i], b[i]))
}

/// `sceVu0ScaleVector` (0x001108d0): every lane times `s`.
pub fn vscale(a: V4, s: F) -> V4 {
    a.map(|x| mul(x, s))
}

/// `sceVu0ScaleVectorXYZ` (0x00111200): x, y, z times `s`, w kept.
pub fn vscale_xyz(a: V4, s: F) -> V4 {
    [mul(a[0], s), mul(a[1], s), mul(a[2], s), a[3]]
}

/// `sceVu0InnerProduct` (0x00110700): `vmul.xyz`, then x + y, then + z.
pub fn dot(a: V4, b: V4) -> F {
    add(add(mul(a[0], b[0]), mul(a[1], b[1])), mul(a[2], b[2]))
}

/// `sceVu0Normalize` (0x00110728): Q = sqrt(x x + y y + z z), then 1 / Q,
/// x, y, z times that; w 0.
pub fn normalize(a: V4) -> V4 {
    let q = sqrt(dot(a, a));
    let r = div(ONE, add(0, q));
    [mul(a[0], r), mul(a[1], r), mul(a[2], r), 0]
}

/// `sceVu0OuterProduct` (0x001106e0): `vopmula` then `vopmsub`, each
/// product rounded; w 0.
pub fn cross(a: V4, b: V4) -> V4 {
    [
        sub(mul(a[1], b[2]), mul(b[1], a[2])),
        sub(mul(a[2], b[0]), mul(b[2], a[0])),
        sub(mul(a[0], b[1]), mul(b[0], a[1])),
        0,
    ]
}

/// `sceVu0ApplyMatrix` (0x00110668): the stored columns `m` times `v`,
/// ((c0 x + c1 y) + c2 z) + c3 w.
pub fn apply(m: &[V4; 4], v: V4) -> V4 {
    std::array::from_fn(|i| {
        let acc = mul(m[0][i], v[0]);
        let acc = add(acc, mul(m[1][i], v[1]));
        let acc = add(acc, mul(m[2][i], v[2]));
        add(acc, mul(m[3][i], v[3]))
    })
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

    #[test]
    fn vectors() {
        let a = [k(1.0), k(2.0), k(3.0), ONE];
        let b = [k(4.0), k(5.0), k(6.0), ONE];
        assert_eq!(f(dot(a, b)), 32.0);
        assert_eq!(cross(a, b).map(f), [-3.0, 6.0, -3.0, 0.0]);
        let n = normalize([k(3.0), k(4.0), 0, 0]);
        assert!((f(n[0]) - 0.6).abs() < 1e-6 && (f(n[1]) - 0.8).abs() < 1e-6);
    }
}
