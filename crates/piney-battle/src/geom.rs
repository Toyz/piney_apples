//! The EE arithmetic the motion layer computes with: the FPU's scalar
//! operations ([`piney_data::field::ee`]), newlib's maths ([`piney_data::libm`]),
//! libvu0's macro-mode vector routines (`sceVu0*`, main) and the angle helpers
//! (`ccGetDirc`, `ccSetDirc`, `RAD2DEG` ...), on raw bit patterns. VU0 follows
//! the FPU's rules (no denormals or infinities, results truncated) and rounds
//! a multiply-add's product first (piney-world's `ee.rs`). Angles are radians
//! in -pi..pi (0 faces -y, growing toward +x), or 16-bit units where 32768 is
//! pi (`RAD2DEG`).

pub use piney_data::field::ee::{add, cmp, div, from_int, le, lt, mul, sqrt, sub, to_int};
pub use piney_data::libm::{atan2f, cosf, fabsf, neg, sinf, sqrtf};

pub use crate::enemy_ai::{get_dirc, get_dist, get_dist_on, rad_disperse, rand_f};

/// A float as bits.
pub type F = u32;
/// A `sceVu0FVECTOR`: x, y, z, w as bits.
pub type V4 = [F; 4];

pub const ONE: F = 0x3f80_0000;
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

/// `c.eq.s`: equal values (every zero pattern equals every other).
pub fn eq(a: F, b: F) -> bool {
    cmp(a, b) == std::cmp::Ordering::Equal
}

/// `sceVu0AddVector` (main 0x00110888): every lane.
pub fn vadd(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| add(a[i], b[i]))
}

/// `sceVu0SubVector` (main 0x001108a0): every lane.
pub fn vsub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| sub(a[i], b[i]))
}

/// `sceVu0ScaleVector` (main 0x001108d0): every lane times `s`.
pub fn vscale(a: V4, s: F) -> V4 {
    a.map(|x| mul(x, s))
}

/// `sceVu0ScaleVectorXYZ` (main 0x00111200): x, y, z times `s`, w kept.
pub fn vscale_xyz(a: V4, s: F) -> V4 {
    [mul(a[0], s), mul(a[1], s), mul(a[2], s), a[3]]
}

/// `sceVu0InnerProduct` (main 0x00110700): `vmul.xyz`, then x + y, then + z.
pub fn dot(a: V4, b: V4) -> F {
    add(add(mul(a[0], b[0]), mul(a[1], b[1])), mul(a[2], b[2]))
}

/// `sceVu0Normalize` (main 0x00110728): Q = sqrt(x x + y y + z z), then
/// 1 / Q, x, y, z times that; w 0.
pub fn normalize(a: V4) -> V4 {
    let q = sqrt(dot(a, a));
    let r = div(ONE, add(0, q));
    [mul(a[0], r), mul(a[1], r), mul(a[2], r), 0]
}

/// The ground distance between two points: `(a - b)` with the third lane
/// zeroed, `sqrtf` of the dot product (`sceVu0SubVector`,
/// `sceVu0InnerProduct`), as the AI's and the entry control's distances
/// are taken.
pub fn plane_dist(a: V4, b: V4) -> F {
    let d0 = sub(a[0], b[0]);
    let d1 = sub(a[1], b[1]);
    sqrtf(add(add(mul(d0, d0), mul(d1, d1)), mul(0, 0)))
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
/// `(to - from) * 16 / mode`, at least 1, at most 24576.
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
    if mode & 0xf_0000 == 0 {
        x /= i64::from(mode & 0xffff);
        if x == 0 {
            x = 1;
        }
    }
    if x >= 24577 {
        x = 24576;
    }
    if neg { -x as i32 } else { x as i32 }
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

/// A 4x4 matrix as `libvu0` stores it: four rows of four floats (`sceVu0FMATRIX`).
pub type M4 = [V4; 4];

/// `S5432` (main 0x002f7520): the sine polynomial's coefficients of
/// `_sceVu0ecossin`, t^9, t^7, t^5, t^3 in x, y, z, w.
const S5432: V4 = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];

/// `_sceVu0ecossin` (main 0x001109b8), VU0 macro code: `s` = the odd
/// polynomial of `t` (t + t^3 w + t^5 z + t^7 y + t^9 x, each term a chain
/// of products, added from t^3 up), then `c = sqrt(1 - s s)` (`vsqrt`),
/// negated when `negative`. Returns (vf4.x, vf4.y) = (c, s); the rotation
/// routines pass `pi/2 -+ angle`, so these are the angle's sine and cosine.
fn ecossin(t: F, negative: bool) -> (F, F) {
    let x = add(0, t);
    let t2 = mul(t, t);
    let mut v8: V4 = S5432.map(|s| mul(s, t));
    v8 = v8.map(|v| mul(v, t2));
    for v in v8.iter_mut().take(3) {
        *v = mul(*v, t2);
    }
    let mut x = add(x, v8[3]);
    for v in v8.iter_mut().take(2) {
        *v = mul(*v, t2);
    }
    x = add(x, v8[2]);
    v8[0] = mul(v8[0], t2);
    x = add(x, v8[1]);
    x = add(x, v8[0]);
    let s = add(0, x);
    let q = sqrt(sub(ONE, mul(s, s)));
    let c = if negative { sub(0, add(0, q)) } else { add(0, add(0, q)) };
    (c, s)
}

/// The sine and cosine the `sceVu0RotMatrix*` routines take of `r`:
/// `ecossin(pi/2 + r)` negated for a negative `r`, else `ecossin(pi/2 -
/// r)`. Returns (sin, cos) as vf4.x, vf4.y.
fn rot_sin_cos(r: F) -> (F, F) {
    if lt(r, 0) { ecossin(add(HALF_PI, r), true) } else { ecossin(sub(HALF_PI, r), false) }
}

/// The VU0 product of a matrix routine: each row of `m` times the four
/// basis rows `b`, accumulated in order (`vmulax`, `vmadday`, `vmaddaz`,
/// `vmaddw`).
fn rows_times(b: &M4, m: &M4) -> M4 {
    m.map(|r| {
        std::array::from_fn(|i| {
            let acc = mul(b[0][i], r[0]);
            let acc = add(acc, mul(b[1][i], r[1]));
            let acc = add(acc, mul(b[2][i], r[2]));
            add(acc, mul(b[3][i], r[3]))
        })
    })
}

/// `sceVu0MulMatrix(out, a, b)`: each row of `b` through `a` (`out[i] =
/// a[0] b[i].x + a[1] b[i].y + a[2] b[i].z + a[3] b[i].w`).
pub fn mul_matrix(a: &M4, b: &M4) -> M4 {
    rows_times(a, b)
}

/// `sceVu0UnitMatrix` (main 0x00110990).
pub fn unit_matrix() -> M4 {
    [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]
}

/// `sceVu0RotMatrixZ(out, m, r)` (main 0x00110a30): `m` turned about z.
pub fn rot_matrix_z(m: &M4, r: F) -> M4 {
    let (x, y) = rot_sin_cos(r);
    let b = [[add(0, y), add(0, x), 0, 0], [sub(0, x), add(0, y), 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
    rows_times(&b, m)
}

/// `sceVu0RotMatrixX(out, m, r)` (main 0x00110ad8): `m` turned about x.
pub fn rot_matrix_x(m: &M4, r: F) -> M4 {
    let (x, y) = rot_sin_cos(r);
    let b =
        [[add(0, ONE), 0, 0, 0], [0, add(0, y), add(0, x), 0], [0, sub(0, x), add(0, y), 0], [0, 0, 0, add(0, ONE)]];
    rows_times(&b, m)
}

/// `sceVu0RotMatrixY(out, m, r)` (main 0x00110b80): `m` turned about y.
pub fn rot_matrix_y(m: &M4, r: F) -> M4 {
    let (x, y) = rot_sin_cos(r);
    let b =
        [[add(0, y), 0, sub(0, x), 0], [0, add(0, ONE), 0, 0], [add(0, x), 0, add(0, y), 0], [0, 0, 0, add(0, ONE)]];
    rows_times(&b, m)
}

/// `sceVu0RotMatrix(out, m, rot)` (main 0x00110c28): about z by `rot.z`,
/// then y by `rot.y`, then x by `rot.x`.
pub fn rot_matrix(m: &M4, rot: V4) -> M4 {
    let m = rot_matrix_z(m, rot[2]);
    let m = rot_matrix_y(&m, rot[1]);
    rot_matrix_x(&m, rot[0])
}

/// `sceVu0ApplyMatrix(out, m, v)` (main 0x00110668): the rows of `m`
/// weighted by `v`'s lanes, ((r0 x + r1 y) + r2 z) + r3 w.
pub fn apply_matrix(m: &M4, v: V4) -> V4 {
    std::array::from_fn(|i| {
        let acc = mul(m[0][i], v[0]);
        let acc = add(acc, mul(m[1][i], v[1]));
        let acc = add(acc, mul(m[2][i], v[2]));
        add(acc, mul(m[3][i], v[3]))
    })
}

/// `sceVu0TransMatrix(out, m, t)` (main 0x001108e8): `m` with `t`'s x, y,
/// z added to its last row (its w kept).
pub fn trans_matrix(m: &M4, t: V4) -> M4 {
    let r = m[3];
    [m[0], m[1], m[2], [add(r[0], t[0]), add(r[1], t[1]), add(r[2], t[2]), r[3]]]
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let m = rot_matrix_z(&unit_matrix(), deg2rad(16384));
        // a quarter turn about z sends x to y
        let v = apply_matrix(&m, [ONE, 0, 0, ONE]);
        assert!(f32::from_bits(v[0]).abs() < 1e-3 && (f32::from_bits(v[1]) - 1.0).abs() < 1e-3);
        let t = trans_matrix(&unit_matrix(), [k(1.0), k(2.0), k(3.0), 0]);
        assert_eq!(t[3], [k(1.0), k(2.0), k(3.0), ONE]);
        assert_eq!(rot_matrix(&unit_matrix(), [0, 0, 0, ONE])[0], [ONE, 0, 0, 0]);
    }
}
