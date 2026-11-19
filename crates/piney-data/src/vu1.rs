//! VU1's upper-pipe arithmetic on float bits, as `tools/vu.py` runs the
//! microcode: every lane through the EE's FPU model ([`crate::field::ee`]),
//! the lane-wise ops and the multiply-adds (`mulax`/`madday`/`maddaz`/`maddw`
//! is [`crate::vu0::apply`]) [`crate::vu0`]'s, `ITOFn` a `cvt.s.w` then a
//! divide by `2^n`, `FTOIn` a multiply by `2^n` then a truncation. A flag the
//! code tests (`FMAND`, `FSAND`) reads a lane's sign as the MAC flag does:
//! the result's bit 31, so a -0 is negative.

use crate::field::ee;
use crate::vu0::{F, ONE, V4};

/// 16.0 and 4096.0, the `FTOI4` and `ITOF12` scales.
const F16: F = 0x4180_0000;
const F4096: F = 0x4580_0000;

/// `ITOF12` of a lane's integer word.
pub fn itof12(word: u32) -> F {
    ee::div(ee::from_int(word as i32), F4096)
}

/// `FTOI4` of a lane: the integer word, in sixteenths.
pub fn ftoi4(f: F) -> u32 {
    ee::to_int(ee::mul(f, F16)) as u32
}

/// The MAC flag's sign bit for a lane's result.
pub fn sign(f: F) -> bool {
    f >> 31 != 0
}

/// `ADDx.dest vf, vf, vf00x`: a lane plus 0, which leaves it but makes a
/// -0 (or an exponent-0 pattern) +0.
pub fn plus0(v: V4) -> V4 {
    v.map(|x| ee::add(x, 0))
}

/// `DIV Q, vf00w, v.w` then `MULQ.xyz`: x, y and z over w, w kept.
pub fn project(v: V4) -> V4 {
    let q = ee::div(ONE, v[3]);
    [ee::mul(v[0], q), ee::mul(v[1], q), ee::mul(v[2], q), v[3]]
}

/// `MULAy.x ACC, a, by` then `MSUBy.x d, b, ay`: `a.x b.y - b.x a.y`, the
/// winding of two screen edges.
pub fn cross_xy(a: V4, b: V4) -> F {
    ee::sub(ee::mul(a[0], b[1]), ee::mul(b[0], a[1]))
}

/// `ADDA` then `MADDq`: `from + to * q`, every lane.
pub fn lerp(from: V4, to: V4, q: F) -> V4 {
    std::array::from_fn(|k| ee::add(ee::add(from[k], 0), ee::mul(to[k], q)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(f32::from_bits(itof12(4096)), 1.0);
        assert_eq!(f32::from_bits(itof12(-2048i32 as u32)), -0.5);
        assert_eq!(ftoi4(1.5f32.to_bits()), 24);
        assert_eq!(ftoi4((-0.03f32).to_bits()), 0);
        // Past the int range FTOI4 holds.
        assert_eq!(ftoi4(3.0e9f32.to_bits()), 0x7fff_ffff);
        assert!(sign(0x8000_0000) && !sign(0));
        assert_eq!(plus0([0x8000_0000, 0x0000_0001, ONE, 0xbf80_0000]), [0, 0, ONE, 0xbf80_0000]);
    }

    #[test]
    fn projection_and_winding() {
        let k = f32::to_bits;
        assert_eq!(project([k(8.0), k(-4.0), k(2.0), k(2.0)]), [k(4.0), k(-2.0), k(1.0), k(2.0)]);
        // (1, 0) then (0, 1): counter-clockwise with y up, positive.
        assert_eq!(cross_xy([k(1.0), 0, 0, 0], [0, k(1.0), 0, 0]), k(1.0));
        assert_eq!(lerp([k(1.0); 4], [k(2.0); 4], k(0.5)), [k(2.0); 4]);
    }
}
