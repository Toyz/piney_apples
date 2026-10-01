//! The game's single-precision maths library, newlib's fdlibm float functions
//! as linked into `SLUS_202.67`, computed with the EE FPU's own rules
//! ([`crate::field::ee`]) on raw `u32` bit patterns, so results match the game
//! to the bit. Each function is a transcription of the compiled routine,
//! operation for operation; the addresses are Infection's. The library was
//! built with `_FLT_LARGEST_EXPONENT_IS_NORMAL`: there are no NaN or infinity
//! cases (`docs/engine/field.md`, EE floating point).

use crate::field::ee::{add, div, from_int, lt, mul, sub, to_int};
use crate::volume::Volume;

const SIGN: u32 = 0x8000_0000;
const ABS: u32 = 0x7fff_ffff;
const ONE: u32 = 0x3f80_0000;
const HALF: u32 = 0x3f00_0000;

/// `fabsf` (0x001279b8): the sign bit cleared.
pub fn fabsf(x: u32) -> u32 {
    x & ABS
}

/// `neg.s`.
pub fn neg(x: u32) -> u32 {
    x ^ SIGN
}

/// `__kernel_sinf` (0x00126600): sin(x + y) for |x| <= pi/4, `y` the tail
/// of a reduced argument, used when `iy` is set.
pub fn kernel_sinf(x: u32, y: u32, iy: bool) -> u32 {
    const S1: u32 = 0xbe2a_aaab;
    const S2: u32 = 0x3c08_8889;
    const S3: u32 = 0xb950_0d01;
    const S4: u32 = 0x3638_ef1b;
    const S5: u32 = 0xb2d7_2f34;
    const S6: u32 = 0x2f2e_c9d3;
    let ix = x & ABS;
    // |x| < 2**-27: (int)x == 0 returns x (always, for such x).
    if ix < 0x3200_0000 && to_int(x) == 0 {
        return x;
    }
    let z = mul(x, x);
    let v = mul(z, x);
    let mut r = add(mul(z, S6), S5);
    r = add(mul(z, r), S4);
    r = add(mul(z, r), S3);
    r = add(mul(z, r), S2);
    if !iy {
        add(x, mul(v, add(mul(z, r), S1)))
    } else {
        let t = sub(mul(z, sub(mul(y, HALF), mul(v, r))), y);
        sub(x, sub(t, mul(v, S1)))
    }
}

/// `__kernel_cosf` (0x00125be8): cos(x + y) for |x| <= pi/4.
pub fn kernel_cosf(x: u32, y: u32) -> u32 {
    const C1: u32 = 0x3d2a_aaab;
    const C2: u32 = 0xbab6_0b61;
    const C3: u32 = 0x37d0_0d01;
    const C4: u32 = 0xb493_f27c;
    const C5: u32 = 0x310f_74f6;
    const C6: u32 = 0xad47_d74e;
    let ix = x & ABS;
    if ix < 0x3200_0000 && to_int(x) == 0 {
        return ONE;
    }
    let z = mul(x, x);
    let mut r = add(mul(z, C6), C5);
    r = add(mul(z, r), C4);
    r = add(mul(z, r), C3);
    r = add(mul(z, r), C2);
    r = add(mul(z, r), C1);
    let r = mul(z, r);
    if ix < 0x3e99_999a {
        sub(ONE, sub(mul(z, HALF), sub(mul(z, r), mul(x, y))))
    } else {
        let qx = if ix > 0x3f48_0000 { 0x3e90_0000 } else { ix - 0x0100_0000 };
        let hz = sub(mul(z, HALF), qx);
        let a = sub(ONE, qx);
        sub(a, sub(hz, sub(mul(z, r), mul(x, y))))
    }
}

/// `npio2_hw` (0x00349a38): the high bits of n * pi/2, n = 1..32.
const NPIO2_HW: [u32; 32] = [
    0x3fc9_0f00,
    0x4049_0f00,
    0x4096_cb00,
    0x40c9_0f00,
    0x40fb_5300,
    0x4116_cb00,
    0x412f_ed00,
    0x4149_0f00,
    0x4162_3100,
    0x417b_5300,
    0x418a_3a00,
    0x4196_cb00,
    0x41a3_5c00,
    0x41af_ed00,
    0x41bc_7e00,
    0x41c9_0f00,
    0x41d5_a000,
    0x41e2_3100,
    0x41ee_c200,
    0x41fb_5300,
    0x4203_f200,
    0x420a_3a00,
    0x4210_8300,
    0x4216_cb00,
    0x421d_1400,
    0x4223_5c00,
    0x4229_a500,
    0x422f_ed00,
    0x4236_3600,
    0x423c_7e00,
    0x4242_c700,
    0x4249_0f00,
];

/// `__ieee754_rem_pio2f` (0x001246e8): x reduced by a multiple n of pi/2,
/// as (n, head, tail). Arguments beyond 2^7 pi/2 (about 201) take
/// `__kernel_rem_pio2f`'s Payne-Hanek path, which the field code never
/// reaches: those are reduced here in f64 and rounded, so they may differ
/// from the game in the last bit.
pub fn rem_pio2f(x: u32) -> (i32, u32, u32) {
    const PIO2_1: u32 = 0x3fc9_0f80;
    const PIO2_1T: u32 = 0x3735_4443;
    const PIO2_2: u32 = 0x3735_4400;
    const PIO2_2T: u32 = 0x2e85_a308;
    const PIO2_3: u32 = 0x2e85_a300;
    const PIO2_3T: u32 = 0x248d_3132;
    const INVPIO2: u32 = 0x3f22_f984;
    let hx = x as i32;
    let ix = x & ABS;
    if ix <= 0x3f49_0fd8 {
        return (0, x, 0);
    }
    if ix < 0x4016_cbe4 {
        // |x| ~< 3pi/4: one step.
        return if hx > 0 {
            let z = sub(x, PIO2_1);
            if ix & 0xffff_fff0 != 0x3fc9_0fd0 {
                let y0 = sub(z, PIO2_1T);
                (1, y0, sub(sub(z, y0), PIO2_1T))
            } else {
                let z = sub(z, PIO2_2);
                let y0 = sub(z, PIO2_2T);
                (1, y0, sub(sub(z, y0), PIO2_2T))
            }
        } else {
            let z = add(x, PIO2_1);
            if ix & 0xffff_fff0 != 0x3fc9_0fd0 {
                let y0 = add(z, PIO2_1T);
                (-1, y0, add(sub(z, y0), PIO2_1T))
            } else {
                let z = add(z, PIO2_2);
                let y0 = add(z, PIO2_2T);
                (-1, y0, add(sub(z, y0), PIO2_2T))
            }
        };
    }
    if ix <= 0x4349_0f80 {
        // |x| ~<= 2^7 pi/2: medium size.
        let t = fabsf(x);
        let n = to_int(add(mul(t, INVPIO2), HALF));
        let fn_ = from_int(n);
        let mut r = sub(t, mul(fn_, PIO2_1));
        let mut w = mul(fn_, PIO2_1T);
        let mut y0;
        if n < 32 && ix & 0xffff_ff00 != NPIO2_HW[(n - 1) as usize] {
            y0 = sub(r, w);
        } else {
            let j = (ix >> 23) as i32;
            y0 = sub(r, w);
            let i = j - ((y0 >> 23) & 0xff) as i32;
            if i > 8 {
                // Second iteration: 2^-25 more bits.
                let t = r;
                w = mul(fn_, PIO2_2);
                r = sub(t, w);
                w = sub(mul(fn_, PIO2_2T), sub(sub(t, r), w));
                y0 = sub(r, w);
                let i = j - ((y0 >> 23) & 0xff) as i32;
                if i > 25 {
                    // Third iteration.
                    let t = r;
                    w = mul(fn_, PIO2_3);
                    r = sub(t, w);
                    w = sub(mul(fn_, PIO2_3T), sub(sub(t, r), w));
                    y0 = sub(r, w);
                }
            }
        }
        let y1 = sub(sub(r, y0), w);
        return if hx < 0 { (-n, neg(y0), neg(y1)) } else { (n, y0, y1) };
    }
    // Beyond the medium range (not reached by the field code).
    let v = f64::from(f32::from_bits(x));
    let q = (v / std::f64::consts::FRAC_PI_2).round();
    let rem = v - q * std::f64::consts::FRAC_PI_2;
    let y0 = (rem as f32).to_bits();
    let y1 = ((rem - f64::from(f32::from_bits(y0))) as f32).to_bits();
    ((q as i64 & 0x7fff_ffff) as i32, y0, y1)
}

/// `sinf` (0x00127c28).
pub fn sinf(x: u32) -> u32 {
    if x & ABS <= 0x3f49_0fd8 {
        return kernel_sinf(x, 0, false);
    }
    let (n, y0, y1) = rem_pio2f(x);
    match n & 3 {
        0 => kernel_sinf(y0, y1, true),
        1 => kernel_cosf(y0, y1),
        2 => neg(kernel_sinf(y0, y1, true)),
        _ => neg(kernel_cosf(y0, y1)),
    }
}

/// `cosf` (0x001278e0).
pub fn cosf(x: u32) -> u32 {
    if x & ABS <= 0x3f49_0fd8 {
        return kernel_cosf(x, 0);
    }
    let (n, y0, y1) = rem_pio2f(x);
    match n & 3 {
        0 => kernel_cosf(y0, y1),
        1 => neg(kernel_sinf(y0, y1, true)),
        2 => neg(kernel_cosf(y0, y1)),
        _ => kernel_sinf(y0, y1, true),
    }
}

/// `atanhi`, `atanlo` (0x00349db0, 0x00349dc0) and `aT` (0x00349dd0).
const ATANHI: [u32; 4] = [0x3eed_6338, 0x3f49_0fda, 0x3f7b_985e, 0x3fc9_0fda];
const ATANLO: [u32; 4] = [0x31ac_3769, 0x3322_2168, 0x3314_0fb4, 0x33a2_2168];
const AT: [u32; 11] = [
    0x3eaa_aaab,
    0xbe4c_cccd,
    0x3e12_4925,
    0xbde3_8e38,
    0x3dba_2e6e,
    0xbd9d_8795,
    0x3d88_6b35,
    0xbd6e_f16b,
    0x3d4b_da59,
    0xbd15_a221,
    0x3c85_69d7,
];

/// `atanf` (0x00127600).
pub fn atanf(x: u32) -> u32 {
    let hx = x as i32;
    let ix = x & ABS;
    if ix > 0x507f_ffff {
        // |x| >= 2^34.
        return if hx > 0 { add(ATANHI[3], ATANLO[3]) } else { sub(neg(ATANHI[3]), ATANLO[3]) };
    }
    let (id, x) = if ix < 0x3ee0_0000 {
        // |x| < 7/16.
        if ix < 0x3100_0000 {
            // |x| < 2^-29: huge + x > one returns x (always).
            if lt(ONE, add(x, 0x7149_f2ca)) {
                return x;
            }
        }
        (-1, x)
    } else {
        let x = fabsf(x);
        if ix < 0x3f98_0000 {
            if ix < 0x3f30_0000 {
                // 7/16 <= |x| < 11/16.
                (0, div(sub(add(x, x), ONE), add(x, 0x4000_0000)))
            } else {
                // 11/16 <= |x| < 19/16.
                (1, div(sub(x, ONE), add(x, ONE)))
            }
        } else if ix < 0x401c_0000 {
            // |x| < 39/16.
            (2, div(sub(x, 0x3fc0_0000), add(mul(x, 0x3fc0_0000), ONE)))
        } else {
            (3, div(0xbf80_0000, x))
        }
    };
    let z = mul(x, x);
    let w = mul(z, z);
    // The odd and even terms, each a Horner chain, interleaved as compiled.
    let s1 = mul(
        z,
        add(AT[0], mul(w, add(AT[2], mul(w, add(AT[4], mul(w, add(AT[6], mul(w, add(AT[8], mul(w, AT[10])))))))))),
    );
    let s2 = mul(w, add(AT[1], mul(w, add(AT[3], mul(w, add(AT[5], mul(w, add(AT[7], mul(w, AT[9])))))))));
    if id < 0 {
        return sub(x, mul(x, add(s1, s2)));
    }
    let id = id as usize;
    let z = sub(ATANHI[id], sub(sub(mul(x, add(s1, s2)), ATANLO[id]), x));
    if hx < 0 { neg(z) } else { z }
}

/// `__ieee754_atan2f` (0x00124330), which `atan2f` jumps to: the angle of
/// (x, y), `y` first as in C.
pub fn atan2f(y: u32, x: u32) -> u32 {
    const PI: u32 = 0x4049_0fda;
    const PI_LO: u32 = 0x3422_2168;
    const PI_O_2: u32 = 0x3fc9_0fdb;
    let (hx, hy) = (x as i32, y as i32);
    let (ix, iy) = (x & ABS, y & ABS);
    if x == ONE {
        return atanf(y);
    }
    let m = ((hy as u32 >> 31) & 1) | ((hx >> 30) as u32 & 2);
    if iy <= 0x007f_ffff {
        // y = 0.
        match m {
            0 | 1 => return y,
            2 => return PI,
            _ => return neg(PI),
        }
    }
    if ix <= 0x007f_ffff {
        // x = 0.
        return if hy < 0 { neg(PI_O_2) } else { PI_O_2 };
    }
    let k = (iy as i32 - ix as i32) >> 23;
    let z = if k > 60 {
        // |y / x| > 2^60: pi/2 + 0.5 pi_lo.
        0x3fc9_0fdc
    } else if hx < 0 && k < -60 {
        0
    } else {
        atanf(fabsf(div(y, x)))
    };
    match m {
        0 => z,
        1 => neg(z),
        2 => sub(PI, sub(z, PI_LO)),
        _ => sub(sub(z, PI_LO), PI),
    }
}

/// `sqrtf` (0x00127e30), `__ieee754_sqrtf`'s bit-by-bit root.
pub fn sqrtf(x: u32) -> u32 {
    crate::field::ee::sqrtf(x)
}

/// The code's `sqrtf` on the volume: newlib's, rounding to nearest, on
/// Infection and Mutation. Outbreak's and Quarantine's executables call no
/// `sqrtf` but from `acosf` and `asinf`: the FPU's truncating `sqrt.s` is
/// inline at every other call, in main and the overlays alike (OUT gcmn
/// 0x005a5f54 in `ccAI::DistanceToTarget`, main 0x001e6e70 `ccGetDist`).
pub fn sqrtf_on(volume: Volume, x: u32) -> u32 {
    match volume {
        Volume::Out | Volume::Qua => crate::field::ee::sqrt(x),
        Volume::Inf | Volume::Mut => sqrtf(x),
    }
}

/// `(float)sqrt((double)x)` as the volume's code takes it: newlib's double
/// `sqrt` between `fptodp` and `dptofp` on Infection and Mutation (the
/// root rounded once); from Outbreak on the drawing code's `sqrt.s`
/// (OUT gcmn 0x005f7fb8 in `STATICOBJECT::Draw`). `BIRD::Move`, `TOBJ::Move`,
/// `STATICMODEL::Draw` and `CheckFrontObstacleF` keep the double.
pub fn dsqrt_on(volume: Volume, x: u32) -> u32 {
    match volume {
        Volume::Out | Volume::Qua => crate::field::ee::sqrt(x),
        Volume::Inf | Volume::Mut => (f64::from(f32::from_bits(x)).sqrt() as f32).to_bits(),
    }
}

/// `T` (0x00349c88): `__kernel_tanf`'s coefficients.
const TAN_T: [u32; 13] = [
    0x3eaa_aaab,
    0x3e08_8889,
    0x3d5d_0dd1,
    0x3cb3_27a4,
    0x3c11_371f,
    0x3b6b_6916,
    0x3abe_de48,
    0x3a1a_26c8,
    0x3981_37b9,
    0x38a3_f445,
    0x3895_c07a,
    0xb79b_ae5f,
    0x37d9_5384,
];

/// `__kernel_tanf` (0x00126728): tan(x + y) for |x| <= pi/4, or -1/tan
/// when `iy` is -1.
pub fn kernel_tanf(x: u32, y: u32, iy: i32) -> u32 {
    const PIO4: u32 = 0x3f49_0fda;
    const PIO4_LO: u32 = 0x3322_2168;
    let t = &TAN_T;
    let hx = x as i32;
    let ix = x & ABS;
    let (mut x, mut y) = (x, y);
    // |x| < 2**-28: (int)x is 0.
    if ix < 0x3180_0000 && to_int(x) == 0 {
        return if ix | (iy + 1) as u32 == 0 {
            div(ONE, fabsf(x))
        } else if iy == 1 {
            x
        } else {
            div(0xbf80_0000, x)
        };
    }
    let big = ix >= 0x3f2c_a140;
    if big {
        if hx < 0 {
            x = neg(x);
            y = neg(y);
        }
        x = add(sub(PIO4, x), sub(PIO4_LO, y));
        y = 0;
    }
    let z = mul(x, x);
    let w = mul(z, z);
    let s = mul(z, x);
    // The two halves of the polynomial, interleaved as compiled.
    let mut odd = add(t[10], mul(w, t[12]));
    let mut even = add(t[9], mul(w, t[11]));
    let t0s = mul(t[0], s);
    for (o, e) in [(t[8], t[7]), (t[6], t[5]), (t[4], t[3]), (t[2], t[1])] {
        odd = add(o, mul(w, odd));
        even = add(e, mul(w, even));
    }
    let v = mul(z, odd);
    let r = add(add(y, mul(z, add(mul(s, add(even, v)), y))), t0s);
    let w = add(x, r);
    if big {
        let v = from_int(iy);
        let sign = from_int(1 - ((hx >> 30) & 2));
        let q = sub(x, sub(div(mul(w, w), add(w, v)), r));
        return mul(sign, sub(v, add(q, q)));
    }
    if iy == 1 {
        return w;
    }
    // -1/(x + r), with the parts kept apart for accuracy.
    let z = w & 0xffff_f000;
    let v = sub(r, sub(z, x));
    let a = div(0xbf80_0000, w);
    let t = a & 0xffff_f000;
    let s = add(mul(t, z), ONE);
    add(t, mul(a, add(s, mul(t, v))))
}

/// `tanf` (0x00127d10).
pub fn tanf(x: u32) -> u32 {
    if x & ABS <= 0x3f49_0fda {
        return kernel_tanf(x, 0, 1);
    }
    let (n, y0, y1) = rem_pio2f(x);
    kernel_tanf(y0, y1, 1 - ((n & 1) << 1))
}

/// `fmodf` (0x00127e18), `__ieee754_fmodf` (0x00124518): x - n y for the
/// n that leaves the sign of x and a magnitude below |y|, exact, by
/// integer shift-and-subtract on the significands.
pub fn fmodf(x: u32, y: u32) -> u32 {
    const MANT: i32 = 0x7f_ffff;
    let sx = x & SIGN;
    let mut hx = (x ^ sx) as i32;
    let mut hy = (y & ABS) as i32;
    // y is 0 (or would be subnormal): (x * y) / (x * y).
    if hy <= MANT {
        let p = mul(x, y);
        return div(p, p);
    }
    if hx < hy {
        return x;
    }
    let zero = sx;
    if hx == hy {
        return zero;
    }
    let ix = (hx >> 23) - 127;
    let mut iy = (hy >> 23) - 127;
    hx = if ix < -126 { hx << (-126 - ix) } else { (hx & MANT) | 0x80_0000 };
    hy = if iy < -126 { hy << (-126 - iy) } else { (hy & MANT) | 0x80_0000 };
    let mut n = ix - iy;
    let mut hz;
    loop {
        n -= 1;
        hz = hx.wrapping_sub(hy);
        if n == -1 {
            break;
        }
        if hz < 0 {
            hx = hx.wrapping_shl(1);
        } else if hz == 0 {
            return zero;
        } else {
            hx = hz.wrapping_shl(1);
        }
    }
    if hz >= 0 {
        hx = hz;
    }
    if hx == 0 {
        return zero;
    }
    while hx <= MANT {
        hx = hx.wrapping_shl(1);
        iy -= 1;
    }
    let out = if iy < -126 { hx >> (-126 - iy) } else { hx.wrapping_sub(0x80_0000) | ((iy + 127) << 23) };
    out as u32 | sx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sqrt(5) is 0x400f1bbc.8...: newlib rounds it up, the later
    /// volumes' `sqrt.s` truncates; the double root of the first two
    /// volumes rounds as newlib does.
    #[test]
    fn the_later_volumes_truncate_the_root() {
        let five = 5f32.to_bits();
        assert_eq!(sqrtf_on(Volume::Inf, five), 0x400f_1bbd);
        assert_eq!(sqrtf_on(Volume::Mut, five), 0x400f_1bbd);
        assert_eq!(sqrtf_on(Volume::Out, five), 0x400f_1bbc);
        assert_eq!(sqrtf_on(Volume::Qua, five), 0x400f_1bbc);
        assert_eq!(dsqrt_on(Volume::Inf, five), 0x400f_1bbd);
        assert_eq!(dsqrt_on(Volume::Out, five), 0x400f_1bbc);
    }

    fn f(x: f32) -> u32 {
        x.to_bits()
    }

    fn close(a: u32, b: f32, tol: f32) -> bool {
        (f32::from_bits(a) - b).abs() <= tol
    }

    #[test]
    fn trig_is_close_to_the_true_values() {
        for i in -400..=400 {
            let x = i as f32 * 0.0173;
            assert!(close(sinf(f(x)), x.sin(), 2e-6), "sin {x}");
            assert!(close(cosf(f(x)), x.cos(), 2e-6), "cos {x}");
        }
        for (y, x) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (0.3, -2.0), (5.0, 0.01), (-3.0, 7.0)]
        {
            assert!(close(atan2f(f(y), f(x)), y.atan2(x), 2e-6), "atan2 {y} {x}");
        }
        assert_eq!(atan2f(0, f(-1.0)), 0x4049_0fda);
        assert_eq!(atan2f(f(2.0), 0), 0x3fc9_0fdb);
        assert_eq!(sinf(0), 0);
        assert_eq!(cosf(0), ONE);
        for i in -300..=300 {
            let x = i as f32 * 0.0101;
            assert!(close(tanf(f(x)), x.tan(), 2e-6 * (1.0 + x.tan().abs())), "tan {x}");
        }
        for (x, y) in [(10.5f32, 3.0f32), (-10.5, 3.0), (7.25, 0.5), (1.0, 3.0), (1234.5678, 0.37), (3.0, 3.0)] {
            assert_eq!(f32::from_bits(fmodf(f(x), f(y))), x % y, "fmod {x} {y}");
        }
    }
}
