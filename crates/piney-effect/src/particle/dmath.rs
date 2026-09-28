//! The double-precision maths two particle functions reach for: newlib's
//! (fdlibm's) `sin` and `cos` (main 0x001274c8, 0x00126ee0) with
//! `__ieee754_rem_pio2`, `__kernel_sin` and `__kernel_cos`, run in the
//! game on libgcc's fp-bit soft doubles (`fptodp`, `dptofp`, `dpadd`...),
//! which round to nearest even like the host's doubles; and
//! `ccSetQuaternion` / `ccQuaternionToMatrix` (0x001c2c30, 0x001c2cf0),
//! the force field 16's turn.

use crate::ee::{self, F, ONE, V4};
use crate::vu::{M4, UNIT};

/// `fptodp`: the EE float as a double (exactly; an exponent-0 pattern is
/// a zero).
fn to_d(x: F) -> f64 {
    if x & 0x7f80_0000 == 0 { if x >> 31 == 1 { -0.0 } else { 0.0 } } else { f64::from(f32::from_bits(x)) }
}

/// `dptofp`: rounded to the nearest float, ties to even.
fn to_f(x: f64) -> F {
    (x as f32).to_bits()
}

fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

const S1: f64 = f64::from_bits(0xbfc5_5555_5555_5549);
const S2: f64 = f64::from_bits(0x3f81_1111_1110_f8a6);
const S3: f64 = f64::from_bits(0xbf2a_01a0_19c1_61d5);
const S4: f64 = f64::from_bits(0x3ec7_1de3_57b1_fe7d);
const S5: f64 = f64::from_bits(0xbe5a_e5e6_8a2b_9ceb);
const S6: f64 = f64::from_bits(0x3de5_d93a_5acf_d57c);

/// `__kernel_sin(x, y, iy)`.
fn kernel_sin(x: f64, y: f64, iy: bool) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix < 0x3e40_0000 && x as i32 == 0 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    if !iy { x + v * (S1 + z * r) } else { x - ((z * (0.5 * y - v * r) - y) - v * S1) }
}

const C1: f64 = f64::from_bits(0x3fa5_5555_5555_554c);
const C2: f64 = f64::from_bits(0xbf56_c16c_16c1_5177);
const C3: f64 = f64::from_bits(0x3efa_01a0_19cb_1590);
const C4: f64 = f64::from_bits(0xbe92_7e4f_809c_52ad);
const C5: f64 = f64::from_bits(0x3e21_ee9e_bdb4_b1c4);
const C6: f64 = f64::from_bits(0xbda8_fae9_be88_38d4);

/// `__kernel_cos(x, y)`.
fn kernel_cos(x: f64, y: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix < 0x3e40_0000 && x as i32 == 0 {
        return 1.0;
    }
    let z = x * x;
    let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
    if ix < 0x3fd3_3333 {
        1.0 - (0.5 * z - (z * r - x * y))
    } else {
        let qx = if ix > 0x3fe9_0000 { 0.28125 } else { f64::from_bits(((ix - 0x0020_0000) as u64) << 32) };
        let hz = 0.5 * z - qx;
        let a = 1.0 - qx;
        a - (hz - (z * r - x * y))
    }
}

const INVPIO2: f64 = f64::from_bits(0x3fe4_5f30_6dc9_c883);
const PIO2_1: f64 = f64::from_bits(0x3ff9_21fb_5440_0000);
const PIO2_1T: f64 = f64::from_bits(0x3dd0_b461_1a62_6331);
const PIO2_2: f64 = f64::from_bits(0x3dd0_b461_1a60_0000);
const PIO2_2T: f64 = f64::from_bits(0x3ba3_198a_2e03_7073);
const PIO2_3: f64 = f64::from_bits(0x3ba3_198a_2e00_0000);
const PIO2_3T: f64 = f64::from_bits(0x397b_839a_2520_49c1);
const NPIO2_HW: [i32; 32] = [
    0x3ff921fb, 0x400921fb, 0x4012d97c, 0x401921fb, 0x401f6a7a, 0x4022d97c, 0x4025fdbb, 0x402921fb, 0x402c463a,
    0x402f6a7a, 0x4031475c, 0x4032d97c, 0x40346b9c, 0x4035fdbb, 0x40378fdb, 0x403921fb, 0x403ab41b, 0x403c463a,
    0x403dd85a, 0x403f6a7a, 0x40407e4c, 0x4041475c, 0x4042106c, 0x4042d97c, 0x4043a28c, 0x40446b9c, 0x404534ac,
    0x4045fdbb, 0x4046c6cb, 0x40478fdb, 0x404858eb, 0x404921fb,
];

/// `__ieee754_rem_pio2(x)` up to 2^19 pi/2 (the angles here are within
/// pi/2); past that the host's reduction stands in.
fn rem_pio2(x: f64) -> (i32, f64, f64) {
    let hx = hi(x);
    let ix = hx & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return (0, x, 0.0);
    }
    if ix < 0x4002_d97c {
        let (n, s) = if hx > 0 { (1, -1.0) } else { (-1, 1.0) };
        let mut z = x + s * PIO2_1;
        let (y0, y1) = if ix != 0x3ff9_21fb {
            let y0 = z + s * PIO2_1T;
            (y0, (z - y0) + s * PIO2_1T)
        } else {
            z += s * PIO2_2;
            let y0 = z + s * PIO2_2T;
            (y0, (z - y0) + s * PIO2_2T)
        };
        return (n, y0, y1);
    }
    if ix <= 0x4139_21fb {
        let t = x.abs();
        let n = (t * INVPIO2 + 0.5) as i32;
        let f = f64::from(n);
        let mut r = t - f * PIO2_1;
        let mut w = f * PIO2_1T;
        let mut y0 = r - w;
        if !(n < 32 && ix != NPIO2_HW[(n - 1) as usize]) {
            let j = ix >> 20;
            let i = j - ((hi(y0) >> 20) & 0x7ff);
            if i > 16 {
                let t = r;
                w = f * PIO2_2;
                r = t - w;
                w = f * PIO2_2T - ((t - r) - w);
                y0 = r - w;
                let i = j - ((hi(y0) >> 20) & 0x7ff);
                if i > 49 {
                    let t = r;
                    w = f * PIO2_3;
                    r = t - w;
                    w = f * PIO2_3T - ((t - r) - w);
                    y0 = r - w;
                }
            }
        }
        let y1 = (r - y0) - w;
        return if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) };
    }
    let n = (x / std::f64::consts::FRAC_PI_2).round();
    (n as i32, x - n * std::f64::consts::FRAC_PI_2, 0.0)
}

/// `sin` (main 0x001274c8).
pub fn sin(x: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return kernel_sin(x, 0.0, false);
    }
    if ix >= 0x7ff0_0000 {
        return f64::NAN;
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => kernel_sin(y0, y1, true),
        1 => kernel_cos(y0, y1),
        2 => -kernel_sin(y0, y1, true),
        _ => -kernel_cos(y0, y1),
    }
}

/// `cos` (main 0x00126ee0).
pub fn cos(x: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return kernel_cos(x, 0.0);
    }
    if ix >= 0x7ff0_0000 {
        return f64::NAN;
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => kernel_cos(y0, y1),
        1 => -kernel_sin(y0, y1, true),
        2 => -kernel_cos(y0, y1),
        _ => kernel_sin(y0, y1, true),
    }
}

/// `ccSetQuaternion(q, axis, angle)` (0x001c2c30): `axis` times
/// sin(angle / 2) in all four lanes, then w cos(angle / 2), both in
/// doubles.
pub fn set_quaternion(axis: V4, angle: F) -> V4 {
    let h = ee::mul(0x3f00_0000, angle);
    let mut q = ee::vscale(axis, to_f(sin(to_d(h))));
    q[3] = to_f(cos(to_d(h)));
    q
}

/// `ccQuaternionToMatrix(m, q)` (0x001c2cf0): the rotation of a
/// quaternion (scaled by 2 / |q|^2), into the unit matrix's upper 3x3.
pub fn quaternion_to_matrix(q: V4) -> M4 {
    use ee::{add, div, mul, sub};
    let [x, y, z, w] = q;
    let (xx, yy, zz) = (mul(x, x), mul(y, y), mul(z, z));
    // add.s, adda.s, madd.s: each product rounded first.
    let n = add(add(add(xx, yy), zz), mul(w, w));
    let s = if ee::le(n, 0) { 0 } else { div(0x4000_0000, n) };
    let (sxx, syy, szz) = (mul(s, xx), mul(s, yy), mul(s, zz));
    let sxy = mul(s, mul(x, y));
    let sxz = mul(s, mul(x, z));
    let syz = mul(s, mul(y, z));
    let sxw = mul(s, mul(x, w));
    let syw = mul(s, mul(y, w));
    let swz = mul(s, mul(w, z));
    let mut m = UNIT;
    m[0][0] = sub(ONE, add(syy, szz));
    m[1][0] = add(sxy, swz);
    m[2][0] = sub(sxz, syw);
    m[0][1] = sub(sxy, swz);
    m[1][1] = sub(ONE, add(sxx, szz));
    m[2][1] = add(syz, sxw);
    m[0][2] = add(sxz, syw);
    m[1][2] = sub(syz, sxw);
    m[2][2] = sub(ONE, add(sxx, syy));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fdlibm_sin_and_cos_agree_with_the_host_closely() {
        for i in -40..=40 {
            let x = f64::from(i) * 0.04;
            assert!((sin(x) - x.sin()).abs() < 1e-15, "{x}");
            assert!((cos(x) - x.cos()).abs() < 1e-15, "{x}");
        }
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
    }

    #[test]
    fn a_half_turn_about_z() {
        let q = set_quaternion([0, 0, ONE, 0], ee::PI);
        let m = quaternion_to_matrix(q);
        assert!((ee::f(m[0][0]) + 1.0).abs() < 1e-6 && (ee::f(m[1][1]) + 1.0).abs() < 1e-6);
        assert_eq!(m[2][2], ONE);
    }
}
