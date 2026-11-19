//! The EE arithmetic the motion layer computes with: the FPU's scalar
//! operations ([`piney_data::field::ee`]), newlib's maths ([`piney_data::libm`]),
//! libvu0's routines ([`piney_data::vu0`]) and the angle helpers (`ccGetDirc`,
//! `ccSetDirc`, `RAD2DEG` ...), on raw bit patterns. Angles are radians in
//! -pi..pi (0 faces -y, growing toward +x), or 16-bit units where 32768 is pi
//! (`RAD2DEG`).

pub use piney_data::field::ee::{add, cmp, div, eq, from_int, le, lt, mul, sqrt, sub, to_int};
pub use piney_data::libm::{atan2f, cosf, fabsf, neg, sinf, sqrtf};
use piney_data::volume::Volume;
pub use piney_data::vu0::{
    F, M4, ONE, UNIT, V4, apply, dot, mul_matrix, normalize, rot_matrix, rot_matrix_x, rot_matrix_y, rot_matrix_z,
    trans_matrix, vadd, vscale, vscale_xyz, vsub,
};

pub use crate::enemy_ai::{get_dist, get_dist_on, rad_disperse, rand_f};

/// `3.14159274` (0x40490fdb), the code's pi.
pub const PI: F = 0x4049_0fdb;
pub const TWO_PI: F = 0x40c9_0fdb;
pub const HALF_PI: F = 0x3fc9_0fdb;
/// 32768.0, half a turn in `RAD2DEG`'s units.
pub const HALF_TURN: F = 0x4700_0000;
/// -1.0: the line checks' "nothing in the way".
pub const MINUS_ONE: F = 0xbf80_0000;

/// `vf0`: (0, 0, 0, 1).
pub const VF0: V4 = [0, 0, 0, ONE];

/// A constant's bits.
pub const fn k(x: f32) -> F {
    x.to_bits()
}

/// The code's `sqrtf` on the volume ([`piney_data::libm::sqrtf_on`]):
/// newlib's on Infection and Mutation, the FPU's truncating `sqrt.s` from
/// Outbreak on.
pub fn sqrt_on(volume: Volume, v: F) -> F {
    piney_data::libm::sqrtf_on(volume, v)
}

/// The ground distance between two points: `(a - b)` with the third lane
/// zeroed, the volume's `sqrtf` of the dot product (`sceVu0SubVector`,
/// `sceVu0InnerProduct`), as the AI's distances are taken.
pub fn plane_dist(volume: Volume, a: V4, b: V4) -> F {
    let d0 = sub(a[0], b[0]);
    let d1 = sub(a[1], b[1]);
    sqrt_on(volume, add(add(mul(d0, d0), mul(d1, d1)), mul(0, 0)))
}

/// The volume's `sqrtf` of a vector's dot product with itself: its length
/// (the ground distance when the third lane is zeroed).
pub fn length_on(volume: Volume, v: V4) -> F {
    sqrt_on(volume, dot(v, v))
}

/// `RAD2DEG` (main 0x001dab50): radians to the game's 16-bit angle,
/// `(short)(int)(32768 (pi + r) / pi - 32768)`.
pub fn rad2deg(r: F) -> i16 {
    let x = div(mul(HALF_TURN, add(PI, r)), PI);
    to_int(sub(x, HALF_TURN)) as i16
}

/// `DEG2RAD` (main 0x001dabb0): `pi * s / 32768`.
pub fn deg2rad(s: i16) -> F {
    div(mul(PI, from_int(i32::from(s))), HALF_TURN)
}

/// libgcc's `fptoui`: a float to an unsigned int (through `cvt.w.s`, with
/// 2^31 taken off first for the values above it).
pub fn fptoui(v: F) -> u32 {
    const TWO_31: F = 0x4f00_0000;
    if lt(v, TWO_31) { to_int(v) as u32 } else { (to_int(sub(v, TWO_31)) as u32) ^ 0x8000_0000 }
}

/// `ccGetDircChg(from, to, mode)` (main 0x001d9eb0): the turn toward `to`,
/// the short way, in 16-bit units; with a mode below 0x10000,
/// `(to - from) * 16 / mode`, at least 1, at most 24576. A mode of 0 traps
/// in the game (`__divdi3` divides by zero on purpose); here it takes the
/// whole turn, as any mode of 0x10000 or more does.
pub fn get_dirc_chg(from: i16, to: i16, mode: i32) -> i32 {
    let d = i32::from(to) - i32::from(from);
    if d == 0 {
        return 0;
    }
    let mut x = i64::from((d & 0xffff) << 4);
    let neg = x >= 0x8_0001;
    if neg {
        x = 0x10_0000 - x;
    }
    let div = i64::from(mode & 0xffff);
    if mode & 0xf_0000 == 0 && div != 0 {
        x = (x / div).max(1);
    }
    if x >= 24577 {
        x = 24576;
    }
    if neg { -x as i32 } else { x as i32 }
}

/// `ccGetDirc(a, b)` (main 0x001d9ce0): the heading from `a` to `b` on the
/// ground (0 faces -y), `atan2f(dy, dx) + pi/2` through [`pi_limit`].
pub fn get_dirc(a: V4, b: V4) -> F {
    pi_limit(add(HALF_PI, atan2f(sub(b[1], a[1]), sub(b[0], a[0]))))
}

/// `ccGetDirc0(a, b)` (main 0x001d9c50): the same through the 16-bit
/// angle, `DEG2RAD(RAD2DEG(atan2f(dy, dx)) + 16384)`.
pub fn get_dirc0(a: V4, b: V4) -> F {
    let s = rad2deg(atan2f(sub(b[1], a[1]), sub(b[0], a[0])));
    deg2rad((i32::from(s) + 16384) as i16)
}

/// `ccCheckCameraDeg(pos, deg)` (main 0x001da710) on its three points
/// already through the caller's `W2PPos`: is `p` within `deg` either side
/// of the line from the eye `c` to the point `v` it looks at, on the
/// ground? The bounds are open.
pub fn camera_deg(p: V4, c: V4, v: V4, deg: i16) -> bool {
    let a = rad2deg(atan2f(sub(p[1], c[1]), sub(p[0], c[0])));
    let b = rad2deg(atan2f(sub(v[1], c[1]), sub(v[0], c[0])));
    let d = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
    d > 0 && i32::from(d) < 2 * i32::from(deg)
}

/// `ccSetDirc(&r, to, mode)` (main 0x001da0b0): `r` turned toward `to` by
/// [`get_dirc_chg`]; returns the new `r`.
pub fn set_dirc(r: F, to: F, mode: i32) -> F {
    let s = rad2deg(r);
    let chg = get_dirc_chg(s, rad2deg(to), mode);
    deg2rad((i32::from(s) + chg) as i16)
}

// The enemies' motion helpers (emove) ---------------------------------------

/// -pi, as the code loads it (`0xc0490fdb`).
pub const NEG_PI: F = 0xc049_0fdb;

/// `ccPiLimit(float)` (gcmn 0x0043ad60): `v` brought into -pi..pi by one
/// turn each way: above pi a turn off, then below -pi a turn on.
pub fn pi_limit(mut v: F) -> F {
    if !le(v, PI) {
        v = sub(v, TWO_PI);
    }
    if lt(v, NEG_PI) {
        v = add(v, TWO_PI);
    }
    v
}

/// `ccPiLimit(float *)` (gcmn 0x0043ace0): [`pi_limit`] in place (the
/// game stores after each step; the value is the same).
pub fn pi_limit_mut(v: &mut F) {
    *v = pi_limit(*v);
}

/// The difference `d = to - from` of two angles taken the short way round
/// as `ccSetRad` does: above pi it becomes `2 pi - d` and the turn is
/// negated after scaling (`true`), below -pi `2 pi + d`.
fn short_way(d: F) -> (F, bool) {
    if !le(d, PI) {
        (sub(TWO_PI, d), true)
    } else if lt(d, neg(PI)) {
        (add(TWO_PI, d), false)
    } else {
        (d, false)
    }
}

/// `ccSetRad(&r, to, rate)` (main 0x001da150): `r` turned toward `to` by
/// `rate` (held to -1..1) of the way, the short way round. A negative rate
/// turns toward the opposite heading (`to + pi`) by `-rate` of the way and
/// then takes the whole step; both angles are measured 0..2 pi for the
/// step, which is added to the old `r` and wrapped into -pi..pi. Nothing
/// is stored when the step is zero.
pub fn set_rad(r: &mut F, to: F, rate: F) {
    let f3 = *r;
    let mut f2 = f3;
    let mut to = to;
    let mut rate = rate;
    if !le(rate, ONE) {
        rate = ONE;
    }
    if lt(rate, MINUS_ONE) {
        rate = MINUS_ONE;
    }
    if lt(rate, 0) {
        to = pi_limit(add(to, PI));
        rate = neg(rate);
        let (d, flip) = short_way(sub(to, f2));
        let mut d = mul(d, rate);
        if flip {
            d = neg(d);
        }
        to = pi_limit(add(f2, d));
        rate = ONE;
    }
    if lt(f2, 0) {
        f2 = add(f2, TWO_PI);
    }
    if lt(to, 0) {
        to = add(to, TWO_PI);
    }
    let d = sub(to, f2);
    if eq(0, d) {
        return;
    }
    let (d, flip) = short_way(d);
    let mut d = mul(d, rate);
    if flip {
        d = neg(d);
    }
    *r = pi_limit(add(f3, d));
}

/// `ccSetDist(&v, to, rate)` (main 0x001da5d0): `v` moved `rate` of the
/// way to `to` (`v + (to - v) rate`); nothing is stored when `to - v` is 0.
pub fn set_dist(v: &mut F, to: F, rate: F) {
    let d = sub(to, *v);
    if !eq(d, 0) {
        *v = add(*v, mul(d, rate));
    }
}

/// `ccGetDircChgF(from, to, mode)` (main 0x001da030): [`get_dirc_chg`] of
/// the two headings in 16-bit units, back in radians (`DEG2RAD` of the
/// short).
pub fn get_dirc_chg_f(from: F, to: F, mode: i32) -> F {
    deg2rad(get_dirc_chg(rad2deg(from), rad2deg(to), mode) as i16)
}

/// `ccGetCameraTransparency`'s far end and fade as its callers pick them
/// by `game.area`: a town (0) 4000 and 400, a field or dungeon 7000 and 600.
pub const TOWN_FAR: F = 0x457a_0000;
pub const TOWN_FADE: F = 0x43c8_0000;
pub const AREA_FAR: F = 0x45da_c000;
pub const AREA_FADE: F = 0x4416_0000;

/// `ccGetCameraTransparency(pos, width, height, far, fade, &hide)` (QUA
/// main 0x001eefa0) with both points already in the player's frame: toward 0 as the camera comes within 1.25 r of the point (r the
/// pitch's blend of `width` and `height`, at least 180; `hide` set, unless
/// it was), 1 out to `far`, then down to 0 over `fade`.
#[allow(clippy::too_many_arguments)]
pub fn camera_transparency(
    volume: Volume,
    pos_p: V4,
    cam_p: V4,
    deg1: i16,
    width: F,
    height: F,
    far: F,
    fade: F,
    hide: &mut bool,
) -> F {
    let d = vsub(cam_p, pos_p);
    let dist = sqrt_on(volume, dot(d, d));
    let t = div(from_int(8192 - i32::from(deg1)), 0x4600_0000);
    let u = sub(ONE, t);
    let mut r = add(mul(width, t), mul(height, u));
    if lt(r, 0x4334_0000) {
        r = 0x4334_0000;
    }
    let r2 = mul(0x3fa0_0000, r);
    if !*hide && lt(dist, r2) {
        *hide = true;
        let near = mul(0x3f4c_cccd, r);
        let f = sub(dist, near);
        return if le(f, 0) { 0 } else { div(f, sub(r2, near)) };
    }
    *hide = false;
    if le(far, 0) || le(dist, far) {
        return ONE;
    }
    let over = sub(dist, far);
    if le(over, fade) { div(sub(fade, over), fade) } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirc_chg_quirks() {
        // A quarter of the way, at least 1.
        assert_eq!(get_dirc_chg(0, 1024, 64), 256);
        assert_eq!(get_dirc_chg(0, 3, 64), 1);
        assert_eq!(get_dirc_chg(0, -1024, 64), -256);
        // Past half a turn the other way round.
        assert_eq!(get_dirc_chg(-30000, 30000, 64), -(16 * 5536) / 64);
        // The switch on `mode & 0xf0000` compares with 2, 1 and 0 (main
        // 0x001d9f20): only 0 can match, so 0x10000 and 0x20000 are not
        // told apart: the whole step, capped at 24576.
        assert_eq!(get_dirc_chg(0, 1000, 0x1_0000), 16_000);
        assert_eq!(get_dirc_chg(0, 3000, -5), 24576);
        // A rate of 0 (the game traps): the whole step, not a panic.
        assert_eq!(get_dirc_chg(0, 1000, 0), 16_000);
        // RAD2DEG truncates: 2048 comes back as 2047, and DEG2RAD of that
        // as 2046.
        assert_eq!(rad2deg(set_dirc(0, deg2rad(8192), 64)), 2046);
    }

    #[test]
    fn angles_round_trip_in_sixteen_bits() {
        assert_eq!(rad2deg(0), 0);
        assert_eq!(rad2deg(k(std::f32::consts::FRAC_PI_2)), 16383);
        assert_eq!(rad2deg(neg(PI)), -32768);
        assert_eq!(deg2rad(-32768), neg(PI));
    }

    #[test]
    fn turning_takes_the_short_way() {
        assert_eq!(get_dirc_chg(0, 0x4000, 64), 0x4000 * 16 / 64);
        // 0x7000 to -0x7000 is 0x2000 forward, capped at 24576 unscaled.
        assert_eq!(get_dirc_chg(0x7000, -0x7000, 0x10000), 24576);
        assert_eq!(get_dirc_chg(0x7000, -0x7000, 64), 0x2000 * 16 / 64);
        assert_eq!(get_dirc_chg(0, 1, 64), 1);
        assert_eq!(fptoui(k(3.9)), 3);
        assert_eq!(fptoui(k(3_000_000_000.0)), 3_000_000_000);
    }

    #[test]
    fn set_rad_turns_the_short_way_and_wraps() {
        let f = |x: F| f32::from_bits(x);
        // halfway from 3 to -3 is across pi, not through 0
        let mut r = k(3.0);
        set_rad(&mut r, k(-3.0), k(0.5));
        assert!(f(r).abs() > 3.1);
        // a negative rate turns toward the opposite heading
        let mut r = 0;
        set_rad(&mut r, k(0.5), k(-1.0));
        assert!((f(r) - (0.5 - std::f32::consts::PI)).abs() < 1e-5);
        // no step, nothing stored
        let mut r = k(1.0);
        set_rad(&mut r, k(1.0), k(0.5));
        assert_eq!(r, k(1.0));
        assert_eq!(pi_limit(k(4.0)), sub(k(4.0), TWO_PI));
        let mut v = k(2.0);
        set_dist(&mut v, k(4.0), k(0.5));
        assert_eq!(v, k(3.0));
    }

    #[test]
    fn rotation_matrices_are_rotations() {
        let m = rot_matrix_z(&UNIT, deg2rad(16384));
        // a quarter turn about z sends x to y
        let v = apply(&m, [ONE, 0, 0, ONE]);
        assert!(f32::from_bits(v[0]).abs() < 1e-3 && (f32::from_bits(v[1]) - 1.0).abs() < 1e-3);
        let t = trans_matrix(&UNIT, [k(1.0), k(2.0), k(3.0), 0]);
        assert_eq!(t[3], [k(1.0), k(2.0), k(3.0), ONE]);
        assert_eq!(rot_matrix(&UNIT, [0, 0, 0, ONE])[0], [ONE, 0, 0, 0]);
    }
}
