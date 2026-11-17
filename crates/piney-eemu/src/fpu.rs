//! The EE's single-precision arithmetic on raw bit patterns, as
//! `tools/eemu.py`'s `f_*` functions have it: [`piney_data::field::ee`]'s
//! functions (`rsqrt.s` among them), and the
//! integer conversions on raw `u32` patterns the way the interpreter holds
//! them. `tools/test_eemu_rs.py` fuzzes every one against eemu's.

pub use piney_data::field::ee::{FMAX, add, div, mul, rsqrt, sqrt, sub};

use piney_data::field::ee;

/// FCR31's condition bit, what `bc1t` / `bc1f` test.
pub const C_BIT: u32 = 1 << 23;

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
