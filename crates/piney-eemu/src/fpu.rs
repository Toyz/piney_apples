//! The EE's single-precision arithmetic on raw bit patterns, as
//! `tools/eemu.py`'s `f_*` functions have it: [`piney_data::field::ee`]'s
//! functions, plus `rsqrt.s`, which that module does not need, and the
//! integer conversions on raw `u32` patterns the way the interpreter holds
//! them. `tools/test_eemu_rs.py` fuzzes every one against eemu's.

pub use piney_data::field::ee::{FMAX, add, div, mul, sqrt, sub};

use piney_data::field::ee;

/// FCR31's condition bit, what `bc1t` / `bc1f` test.
pub const C_BIT: u32 = 1 << 23;
const BIAS: i32 = 150;

/// `cvt.s.w` on a raw word.
pub fn from_int(i: u32) -> u32 {
    ee::from_int(i as i32)
}

/// `cvt.w.s` as a raw word.
pub fn to_int(v: u32) -> u32 {
    ee::to_int(v) as u32
}

/// `f_cmp`: -1, 0 or 1 comparing exact values (both zeros equal).
pub fn cmp(a: u32, b: u32) -> i32 {
    ee::cmp(a, b) as i32
}

/// (sign bit, exponent field, mantissa with the hidden bit); the mantissa is
/// 0 when the exponent field is.
fn unpack(v: u32) -> (u32, i32, u128) {
    let e = (v >> 23) & 0xff;
    let m = if e == 0 { 0 } else { u128::from(v & 0x007f_ffff | 0x0080_0000) };
    (v >> 31, e as i32, m)
}

/// eemu's `_round`: sign and the exact magnitude `n * 2^e2`, truncated to 24
/// bits, saturating at Fmax and flushing to a signed zero.
fn round(sign: u32, n: u128, e2: i32) -> u32 {
    if n == 0 {
        return sign << 31;
    }
    let len = (128 - n.leading_zeros()) as i32;
    let m = if len >= 24 { n >> (len - 24) } else { n << (24 - len) } as u32;
    let exp = e2 + len - 24 + BIAS;
    if exp > 255 {
        return sign << 31 | FMAX;
    }
    if exp < 1 {
        return sign << 31;
    }
    sign << 31 | (exp as u32) << 23 | (m & 0x007f_ffff)
}

/// `rsqrt.s`: `a / sqrt(b)` rounded once, as eemu's `f_rsqrt`:
/// `floor(A / sqrt(B)) = isqrt(A * A // B)`. The numerator's `2^120` and the
/// denominator's `2^t` cancel exactly, so the quotient fits in 128 bits.
pub fn rsqrt(a: u32, b: u32) -> u32 {
    let (sa, ea, ma) = unpack(a);
    let (_, eb, mb) = unpack(b);
    if mb == 0 {
        return sa << 31 | FMAX;
    }
    let t = 60 + ((eb - BIAS - 60) & 1);
    let q = ((ma * ma) << (120 - t)) / (mb);
    round(sa, q.isqrt(), ea - BIAS - 60 - (eb - BIAS - t) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: u32 = 0x3f80_0000;

    #[test]
    fn rsqrt_of_squares() {
        let four = 0x4080_0000;
        let half = 0x3f00_0000;
        assert_eq!(rsqrt(ONE, four), half);
        assert_eq!(rsqrt(ONE, ONE), ONE);
        assert_eq!(rsqrt(0xbf80_0000, four), 0xbf00_0000);
        // Division by a zero (any exponent-0 pattern) is +/-Fmax.
        assert_eq!(rsqrt(ONE, 0x0000_1234), FMAX);
        assert_eq!(rsqrt(0xbf80_0000, 0), 0x8000_0000 | FMAX);
        // A zero numerator is a zero.
        assert_eq!(rsqrt(0, four), 0);
    }

    #[test]
    fn raw_conversions() {
        assert_eq!(from_int(0xffff_fff9), 0xc0e0_0000); // -7
        assert_eq!(to_int(0xc0e0_0000), 0xffff_fff9);
        assert_eq!(cmp(0x8000_0000, 0), 0);
        assert_eq!(cmp(ONE, 0), 1);
    }
}
