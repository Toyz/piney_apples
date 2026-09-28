//! The maths the drain's homing orbs reach for that `piney_data::libm`
//! lacks: newlib's `acosf` (`__ieee754_acosf`, main 0x00123b38) on the EE
//! FPU's rules, and the double-precision `sin` (0x001274c8) and `cos`
//! (0x00126ee0) with their kernels (`__kernel_sin` 0x001259b0,
//! `__kernel_cos` 0x00124b88, `__ieee754_rem_pio2` 0x00123260).
//!
//! The EE has no double-precision unit: `sin` and `cos` run on the
//! compiler's soft-float routines (`dpadd` 0x0012a1b0, `dpsub`, `dpmul`
//! 0x0012a270, `dptofp` 0x0012a990, `fptodp` 0x00129cc8), which round to
//! nearest like IEEE doubles, so `f64` arithmetic in the same order gives
//! the same bits (checked against the game's own routines in eemu by
//! `tools/test_effect_misc_rs.py`). `acosf` is single precision on the
//! FPU: each operation is [`crate::ee`]'s, in the compiled order.

use crate::ee::{self, F, ONE};

const HALF: F = 0x3f00_0000;
/// `pS0`..`pS5`, `qS1`..`qS4` of e_acosf.c.
const PS: [F; 6] = [0x3e2a_aaab, 0xbea6_b090, 0x3e4e_0aa8, 0xbd24_1146, 0x3a4f_7f04, 0x3811_ef08];
const QS: [F; 4] = [0xc019_d139, 0x4001_572d, 0xbf30_3361, 0x3d9d_c62e];
const PIO2_HI: F = 0x3fc9_0fda;
const PIO2_LO: F = 0x33a2_2168;

/// `p / q` of e_acosf.c's rational approximation at `z`, in the compiled
/// order: the two polynomials stepped together.
fn pq(z: F) -> (F, F) {
    let mut p = ee::add(ee::mul(z, PS[5]), PS[4]);
    let mut q = ee::add(ee::mul(z, QS[3]), QS[2]);
    p = ee::add(ee::mul(z, p), PS[3]);
    q = ee::add(ee::mul(z, q), QS[1]);
    p = ee::add(ee::mul(z, p), PS[2]);
    q = ee::add(ee::mul(z, q), QS[0]);
    p = ee::add(ee::mul(z, p), PS[1]);
    q = ee::add(ee::mul(z, q), ONE);
    p = ee::mul(z, ee::add(ee::mul(z, p), PS[0]));
    (p, q)
}

/// `acosf(x)` (main 0x00127dd0, `__ieee754_acosf` 0x00123b38).
pub fn acosf(x: F) -> F {
    let ix = x & 0x7fff_ffff;
    let neg = x & 0x8000_0000 != 0;
    if ix == ONE {
        return if neg { 0x4049_0fdb } else { 0 };
    }
    if ix > ONE {
        // (x - x) / (x - x): the EE's 0 / 0.
        let d = ee::sub(x, x);
        return ee::div(d, d);
    }
    if ix < HALF {
        if ix <= 0x2300_0000 {
            return 0x3fc9_0fdb;
        }
        let z = ee::mul(x, x);
        let (p, q) = pq(z);
        let r = ee::div(p, q);
        return ee::sub(PIO2_HI, ee::sub(x, ee::sub(PIO2_LO, ee::mul(x, r))));
    }
    if neg {
        let z = ee::mul(ee::add(x, ONE), HALF);
        let (p, q) = pq(z);
        let s = ee::sqrtf(z);
        let r = ee::div(p, q);
        let w = ee::sub(ee::mul(r, s), PIO2_LO);
        let t = ee::add(s, w);
        return ee::sub(0x4049_0fda, ee::add(t, t));
    }
    let z = ee::mul(ee::sub(ONE, x), HALF);
    let s = ee::sqrtf(z);
    let df = s & 0xffff_f000;
    let c = ee::div(ee::sub(z, ee::mul(df, df)), ee::add(s, df));
    let (p, q) = pq(z);
    let r = ee::div(p, q);
    let w = ee::add(ee::mul(r, s), c);
    let t = ee::add(df, w);
    ee::add(t, t)
}

/// The high word of a double, as fdlibm's `GET_HIGH_WORD`.
fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

const S: [f64; 6] = [
    -1.666_666_666_666_663_2e-1,
    8.333_333_333_322_49e-3,
    -1.984_126_982_985_795e-4,
    2.755_731_370_707_006_8e-6,
    -2.505_076_025_340_686_3e-8,
    1.589_690_995_211_55e-10,
];
const C: [f64; 6] = [
    4.166_666_666_666_66e-2,
    -1.388_888_888_887_411e-3,
    2.480_158_728_947_673e-5,
    -2.755_731_435_139_066_3e-7,
    2.087_572_321_298_175e-9,
    -1.135_964_755_778_819_5e-11,
];

/// `__kernel_sin(x, y, iy)` (main 0x001259b0).
fn kernel_sin(x: f64, y: f64, iy: bool) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix < 0x3e40_0000 && x as i32 == 0 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = S[1] + z * (S[2] + z * (S[3] + z * (S[4] + z * S[5])));
    if !iy { x + v * (S[0] + z * r) } else { x - ((z * (0.5 * y - v * r) - y) - v * S[0]) }
}

/// `__kernel_cos(x, y)` (main 0x00124b88).
fn kernel_cos(x: f64, y: f64) -> f64 {
    let ix = hi(x) & 0x7fff_ffff;
    if ix < 0x3e40_0000 && x as i32 == 0 {
        return 1.0;
    }
    let z = x * x;
    let r = z * (C[0] + z * (C[1] + z * (C[2] + z * (C[3] + z * (C[4] + z * C[5])))));
    if ix < 0x3fd3_3333 {
        1.0 - (0.5 * z - (z * r - x * y))
    } else {
        let qx = if ix > 0x3fe9_0000 { 0.28125 } else { f64::from_bits((((ix - 0x0020_0000) as u32) as u64) << 32) };
        let hz = 0.5 * z - qx;
        let a = 1.0 - qx;
        a - (hz - (z * r - x * y))
    }
}

const PIO2_1: f64 = f64::from_bits(0x3ff9_21fb_5440_0000);
const PIO2_1T: f64 = f64::from_bits(0x3dd0_b461_1a62_6331);
const PIO2_2: f64 = f64::from_bits(0x3dd0_b461_1a60_0000);
const PIO2_2T: f64 = f64::from_bits(0x3ba3_198a_2e03_7073);
const PIO2_3: f64 = f64::from_bits(0x3ba3_198a_2e00_0000);
const PIO2_3T: f64 = f64::from_bits(0x397b_839a_2520_49c1);
const INVPIO2: f64 = f64::from_bits(0x3fe4_5f30_6dc9_c883);
const NPIO2_HW: [i32; 32] = [
    0x3ff9_21fb,
    0x4009_21fb,
    0x4012_d97c,
    0x4019_21fb,
    0x401f_6a7a,
    0x4022_d97c,
    0x4025_fdbb,
    0x4029_21fb,
    0x402c_463a,
    0x402f_6a7a,
    0x4031_475c,
    0x4032_d97c,
    0x4034_6b9c,
    0x4035_fdbb,
    0x4037_8fdb,
    0x4039_21fb,
    0x403a_b41b,
    0x403c_463a,
    0x403d_d85a,
    0x403f_6a7a,
    0x4040_7e4c,
    0x4041_475c,
    0x4042_106c,
    0x4042_d97c,
    0x4043_a28c,
    0x4044_6b9c,
    0x4045_34ac,
    0x4045_fdbb,
    0x4046_c6cb,
    0x4047_8fdb,
    0x4048_58eb,
    0x4049_21fb,
];

/// `__ieee754_rem_pio2(x, y)` (main 0x00123260) for |x| up to 2^19 pi/2
/// (the medium case; the callers here stay under pi): `x` less `n` pi/2
/// as the head and tail `y`, and `n`.
fn rem_pio2(x: f64) -> (i32, f64, f64) {
    let hx = hi(x);
    let ix = hx & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return (0, x, 0.0);
    }
    if ix < 0x4002_d97c {
        if hx > 0 {
            let z = x - PIO2_1;
            if ix != 0x3ff9_21fb {
                let y0 = z - PIO2_1T;
                return (1, y0, (z - y0) - PIO2_1T);
            }
            let z = z - PIO2_2;
            let y0 = z - PIO2_2T;
            return (1, y0, (z - y0) - PIO2_2T);
        }
        let z = x + PIO2_1;
        if ix != 0x3ff9_21fb {
            let y0 = z + PIO2_1T;
            return (-1, y0, (z - y0) + PIO2_1T);
        }
        let z = z + PIO2_2;
        let y0 = z + PIO2_2T;
        return (-1, y0, (z - y0) + PIO2_2T);
    }
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
    if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) }
}

/// `sin(x)` (main 0x001274c8), double precision.
pub fn sin(x: f64) -> f64 {
    if hi(x) & 0x7fff_ffff <= 0x3fe9_21fb {
        return kernel_sin(x, 0.0, false);
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => kernel_sin(y0, y1, true),
        1 => kernel_cos(y0, y1),
        2 => -kernel_sin(y0, y1, true),
        _ => -kernel_cos(y0, y1),
    }
}

/// `cos(x)` (main 0x00126ee0), double precision.
pub fn cos(x: f64) -> f64 {
    if hi(x) & 0x7fff_ffff <= 0x3fe9_21fb {
        return kernel_cos(x, 0.0);
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => kernel_cos(y0, y1),
        1 => -kernel_sin(y0, y1, true),
        2 => -kernel_cos(y0, y1),
        _ => kernel_sin(y0, y1, true),
    }
}

/// `(float)sin((double)x)`: `fptodp`, `sin`, `dptofp`.
pub fn sin_fd(x: F) -> F {
    (sin(f64::from(f32::from_bits(x))) as f32).to_bits()
}

/// `(float)cos((double)x)`.
pub fn cos_fd(x: F) -> F {
    (cos(f64::from(f32::from_bits(x))) as f32).to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_the_library() {
        for i in -200..=200 {
            let x = i as f64 * 0.0117;
            assert!((sin(x) - x.sin()).abs() < 1e-15, "sin {x}");
            assert!((cos(x) - x.cos()).abs() < 1e-15, "cos {x}");
            let y = ee::k(i as f32 / 200.0);
            assert!((ee::f(acosf(y)) - (i as f32 / 200.0).acos()).abs() < 1e-6, "acosf {}", i as f32 / 200.0);
        }
        assert_eq!(acosf(ONE), 0);
    }
}
