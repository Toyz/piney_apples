//! EE single-precision arithmetic on raw bit patterns, as `tools/eemu.py`'s
//! `f_*` functions model the Emotion Engine's FPU (`docs/engine/field.md`, "EE
//! floating point"): no denormals (exponent 0 is zero), no infinities or NaNs
//! (exponent 255 is a number), overflow to +/-[`FMAX`] and underflow to a
//! signed zero, division by zero +/-[`FMAX`], every result the exact one
//! truncated toward zero to 24 bits, worked on the integer mantissas.
//! `madd.s`/`msub.s` round the product first, so the game's multiply-adds are
//! a [`mul`] then an [`add`].

use std::cmp::Ordering;

/// The EE's largest float, what overflow and division by zero give.
pub const FMAX: u32 = 0x7fff_ffff;
const SIGN: u32 = 0x8000_0000;
const FRACTION: u32 = 0x007f_ffff;
const HIDDEN: u32 = 0x0080_0000;
/// Exponent bias plus the 23 fraction bits: value = mantissa * 2^(exp - 150).
const BIAS: i32 = 150;

/// (sign bit, exponent field, mantissa with the hidden bit), the mantissa 0
/// when the exponent field is 0.
fn unpack(v: u32) -> (u32, i32, u64) {
    let e = (v >> 23) & 0xff;
    let m = if e == 0 { 0 } else { u64::from(v & FRACTION | HIDDEN) };
    (v >> 31, e as i32, m)
}

/// The float of sign `sign` and exact magnitude `n * 2^e2`, truncated.
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
    sign << 31 | (exp as u32) << 23 | (m & FRACTION)
}

/// `add.s`.
pub fn add(a: u32, b: u32) -> u32 {
    let (sa, ea, ma) = unpack(a);
    let (sb, eb, mb) = unpack(b);
    match (ma, mb) {
        (0, 0) => return a & b & SIGN, // -0 only when both are -0
        (0, _) => return b,
        (_, 0) => return a,
        _ => {}
    }
    // x has the larger exponent.
    let ((sx, ex, mx, x), (sy, ey, my)) =
        if ea >= eb { ((sa, ea, ma, a), (sb, eb, mb)) } else { ((sb, eb, mb, b), (sa, ea, ma)) };
    let d = (ex - ey) as u32;
    if d > 64 {
        // y lies wholly below x's last bit: the same sign leaves x, the
        // other sign takes x one step toward zero (a bit pattern's
        // magnitude is monotonic, and x's exponent is far above 1).
        return if sx == sy { x } else { x - 1 };
    }
    let signed = |s: u32, m: i128| if s == 1 { -m } else { m };
    let n = signed(sx, i128::from(mx) << d) + signed(sy, i128::from(my));
    round(u32::from(n < 0), n.unsigned_abs(), ey - BIAS)
}

/// `sub.s`.
pub fn sub(a: u32, b: u32) -> u32 {
    add(a, b ^ SIGN)
}

/// `mul.s`.
pub fn mul(a: u32, b: u32) -> u32 {
    let (sa, ea, ma) = unpack(a);
    let (sb, eb, mb) = unpack(b);
    round(sa ^ sb, u128::from(ma * mb), ea + eb - 2 * BIAS)
}

/// `div.s`.
pub fn div(a: u32, b: u32) -> u32 {
    let (sa, ea, ma) = unpack(a);
    let (sb, eb, mb) = unpack(b);
    if mb == 0 {
        return (sa ^ sb) << 31 | FMAX;
    }
    round(sa ^ sb, (u128::from(ma) << 60) / u128::from(mb), ea - eb - 60)
}

/// `sqrt.s`: the square root of the magnitude, truncated.
pub fn sqrt(v: u32) -> u32 {
    let (_, e, m) = unpack(v);
    let t = 60 + ((e - BIAS - 60) & 1);
    let root = (u128::from(m) << t).isqrt();
    round(0, root, (e - BIAS - t) / 2)
}

/// `__ieee754_sqrtf` (`INF SLUS_202.67:0x00124ab0`): newlib's bit-by-bit
/// square root, rounding to nearest; exponent 0 comes back unchanged, a
/// negative number gives [`FMAX`] (the EE's 0/0).
pub fn sqrtf(v: u32) -> u32 {
    if v & 0x7f80_0000 == 0 {
        return v;
    }
    if v & SIGN != 0 {
        return FMAX;
    }
    let mut ix = (v >> 23) as i32 - 127;
    let mut m = u64::from(v & FRACTION | HIDDEN) << (ix & 1);
    ix >>= 1;
    m <<= 1;
    let (mut q, mut s, mut r) = (0u64, 0u64, 0x0100_0000u64);
    while r != 0 {
        let t = s + r;
        if m >= t {
            m -= t;
            s = t + r;
            q += r;
        }
        m <<= 1;
        r >>= 1;
    }
    if m != 0 {
        q += q & 1;
    }
    ((q >> 1) as i64 + 0x3f00_0000 + (i64::from(ix) << 23)) as u32
}

/// `cvt.s.w`.
pub fn from_int(i: i32) -> u32 {
    round(u32::from(i < 0), u128::from(i.unsigned_abs()), 0)
}

/// `cvt.w.s`: truncate toward zero, saturating at the int range.
pub fn to_int(v: u32) -> i32 {
    let (s, e, m) = unpack(v);
    let e = e - BIAS;
    let mag: u128 = if m == 0 {
        0
    } else if e >= 0 {
        if e > 40 { u128::MAX } else { u128::from(m) << e }
    } else if e <= -64 {
        0
    } else {
        u128::from(m >> -e)
    };
    if s == 1 && m != 0 {
        if mag > 0x8000_0000 { i32::MIN } else { (mag as i64).wrapping_neg() as i32 }
    } else if mag > 0x7fff_ffff {
        i32::MAX
    } else {
        mag as i32
    }
}

/// `c.lt.s` / `c.le.s` / `c.eq.s` as an ordering of exact values: both
/// zeros (any pattern with exponent 0) are equal.
pub fn cmp(a: u32, b: u32) -> Ordering {
    key(a).cmp(&key(b))
}

/// A float's value order: for normalised numbers the pattern's magnitude
/// bits order like the value, exponent 255 included.
fn key(v: u32) -> i64 {
    if v & 0x7f80_0000 == 0 {
        return 0;
    }
    let mag = i64::from(v & !SIGN);
    if v & SIGN != 0 { -mag } else { mag }
}

/// `a < b`.
pub fn lt(a: u32, b: u32) -> bool {
    cmp(a, b) == Ordering::Less
}

/// `a <= b`.
pub fn le(a: u32, b: u32) -> bool {
    cmp(a, b) != Ordering::Greater
}

/// An `f32` constant's bits, for the constants the code loads with lui/ori.
pub const fn bits(x: f32) -> u32 {
    x.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: u32 = 0x3f80_0000;

    #[test]
    fn truncates_toward_zero() {
        // 1 + 2^-24 is below 1's last bit: stays 1; 1 - 2^-24 truncates to
        // the float below 1, not back up to 1.
        let tiny = 0x3380_0000; // 2^-24
        assert_eq!(add(ONE, tiny), ONE);
        assert_eq!(sub(ONE, tiny), 0x3f7f_ffff);
        // 1/3 truncates (IEEE rounds 0x3eaaaaab).
        assert_eq!(div(ONE, bits(3.0)), 0x3eaa_aaaa);
        assert_eq!(div(bits(-1.0), bits(3.0)), 0xbeaa_aaaa);
        // A product truncated: (1 + 2^-23)^2 = 1 + 2^-22 + 2^-46.
        assert_eq!(mul(0x3f80_0001, 0x3f80_0001), 0x3f80_0002);
    }

    #[test]
    fn far_apart_operands() {
        let big = bits(1.0e30);
        assert_eq!(add(big, ONE), big);
        assert_eq!(sub(big, ONE), big - 1);
        assert_eq!(add(bits(-1.0e30), ONE), bits(-1.0e30) - 1);
        // A power of two drops into the binade below.
        assert_eq!(sub(0x7000_0000, ONE), 0x6fff_ffff);
    }

    #[test]
    fn no_denormals_infinities_or_nans() {
        // Exponent 0 is zero whatever the mantissa.
        assert_eq!(add(0x0000_0001, ONE), ONE);
        assert_eq!(mul(0x0000_1234, ONE), 0);
        assert_eq!(cmp(0x0000_1234, 0x8000_0000), Ordering::Equal);
        // Underflow gives a signed zero, overflow and x/0 Fmax.
        assert_eq!(mul(0x0080_0000, 0x0080_0000), 0);
        assert_eq!(mul(0x8080_0000, 0x0080_0000), SIGN);
        assert_eq!(mul(0x7f00_0000, 0x7f00_0000), FMAX);
        assert_eq!(div(bits(-2.0), 0), SIGN | FMAX);
        assert_eq!(div(0, 0), FMAX);
        // Exponent 255 is a number.
        assert_eq!(cmp(0x7f80_0000, 0x7f7f_ffff), Ordering::Greater);
        assert_eq!(sub(0x7f80_0000, 0x7f80_0000), 0);
        // Zeros: -0 only from -0 + -0.
        assert_eq!(add(SIGN, SIGN), SIGN);
        assert_eq!(add(SIGN, 0), 0);
        assert_eq!(sub(ONE, ONE), 0);
    }

    #[test]
    fn conversions() {
        assert_eq!(from_int(600), bits(600.0));
        assert_eq!(from_int(-7), bits(-7.0));
        assert_eq!(from_int(0x0100_0001), bits(16_777_216.0)); // truncated
        assert_eq!(from_int(-0x0100_0001), bits(-16_777_216.0));
        assert_eq!(from_int(i32::MIN), 0xcf00_0000);
        assert_eq!(to_int(bits(27.99)), 27);
        assert_eq!(to_int(bits(-27.99)), -27);
        assert_eq!(to_int(bits(3.0e9)), i32::MAX);
        assert_eq!(to_int(bits(-3.0e9)), i32::MIN);
        assert_eq!(to_int(bits(0.5)), 0);
        assert_eq!(to_int(SIGN), 0);
    }

    #[test]
    fn square_roots() {
        assert_eq!(sqrtf(bits(4.0)), bits(2.0));
        assert_eq!(sqrt(bits(4.0)), bits(2.0));
        // sqrt(2): newlib rounds to nearest (0x3fb504f3), sqrt.s truncates.
        assert_eq!(sqrtf(bits(2.0)), 0x3fb5_04f3);
        assert_eq!(sqrt(bits(2.0)), 0x3fb5_04f3);
        assert_eq!(sqrtf(bits(3.0)), 0x3fdd_b3d7);
        assert_eq!(sqrt(bits(3.0)), 0x3fdd_b3d7);
        assert_eq!(sqrtf(bits(5.0)), 0x400f_1bbd);
        assert_eq!(sqrt(bits(5.0)), 0x400f_1bbc);
        assert_eq!(sqrtf(bits(-4.0)), FMAX);
        assert_eq!(sqrt(bits(-4.0)), bits(2.0));
        assert_eq!(sqrtf(0x0000_0005), 0x0000_0005);
        assert_eq!(sqrt(0x0000_0005), 0);
    }
}
