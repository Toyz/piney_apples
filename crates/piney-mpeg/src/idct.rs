//! The inverse DCT: an integer 8x8 IDCT in the "simple IDCT" arithmetic
//! (rows then columns, cosines scaled by 2^14 and rounded, a 2^11 row and a
//! 2^20 column shift, and a DC-only row shortcut), chosen because it gives
//! ffmpeg's `mpeg2video` output bit for bit, and it meets IEEE 1180-1990
//! (the check is the `ieee1180` test below).
//!
//! The PS2 does the IDCT in the IPU (the BDEC command libmpeg sends,
//! `_decMB0` at INF SLUS_202.67:0x00113ab0), whose arithmetic is not
//! documented to the bit; this one agrees with any IEEE 1180 IDCT to within
//! one level per sample.

// 2^14 sqrt(2) cos(i pi / 16), rounded; W4 is one less than the rounding.
const W1: i32 = 22725;
const W2: i32 = 21407;
const W3: i32 = 19266;
const W4: i32 = 16383;
const W5: i32 = 12873;
const W6: i32 = 8867;
const W7: i32 = 4520;
const ROW_SHIFT: u32 = 11;
const COL_SHIFT: u32 = 20;

/// One row, in place. Values are stored back as 16 bits, as the reference
/// arithmetic does.
#[inline]
fn row(r: &mut [i16]) {
    if r[1..].iter().all(|&c| c == 0) {
        // DC only: the row is DC << 3 (wrapped to 16 bits).
        let dc = (i32::from(r[0]) << 3) as i16;
        r.fill(dc);
        return;
    }
    let c: [i32; 8] = std::array::from_fn(|i| i32::from(r[i]));
    let mut a0 = W4.wrapping_mul(c[0]).wrapping_add(1 << (ROW_SHIFT - 1));
    let mut a1 = a0;
    let mut a2 = a0;
    let mut a3 = a0;
    a0 = a0.wrapping_add(W2.wrapping_mul(c[2]));
    a1 = a1.wrapping_add(W6.wrapping_mul(c[2]));
    a2 = a2.wrapping_sub(W6.wrapping_mul(c[2]));
    a3 = a3.wrapping_sub(W2.wrapping_mul(c[2]));
    let mut b0 = W1.wrapping_mul(c[1]).wrapping_add(W3.wrapping_mul(c[3]));
    let mut b1 = W3.wrapping_mul(c[1]).wrapping_sub(W7.wrapping_mul(c[3]));
    let mut b2 = W5.wrapping_mul(c[1]).wrapping_sub(W1.wrapping_mul(c[3]));
    let mut b3 = W7.wrapping_mul(c[1]).wrapping_sub(W5.wrapping_mul(c[3]));
    a0 = a0.wrapping_add(W4.wrapping_mul(c[4])).wrapping_add(W6.wrapping_mul(c[6]));
    a1 = a1.wrapping_sub(W4.wrapping_mul(c[4])).wrapping_sub(W2.wrapping_mul(c[6]));
    a2 = a2.wrapping_sub(W4.wrapping_mul(c[4])).wrapping_add(W2.wrapping_mul(c[6]));
    a3 = a3.wrapping_add(W4.wrapping_mul(c[4])).wrapping_sub(W6.wrapping_mul(c[6]));
    b0 = b0.wrapping_add(W5.wrapping_mul(c[5])).wrapping_add(W7.wrapping_mul(c[7]));
    b1 = b1.wrapping_sub(W1.wrapping_mul(c[5])).wrapping_sub(W5.wrapping_mul(c[7]));
    b2 = b2.wrapping_add(W7.wrapping_mul(c[5])).wrapping_add(W3.wrapping_mul(c[7]));
    b3 = b3.wrapping_add(W3.wrapping_mul(c[5])).wrapping_sub(W1.wrapping_mul(c[7]));
    r[0] = (a0.wrapping_add(b0) >> ROW_SHIFT) as i16;
    r[7] = (a0.wrapping_sub(b0) >> ROW_SHIFT) as i16;
    r[1] = (a1.wrapping_add(b1) >> ROW_SHIFT) as i16;
    r[6] = (a1.wrapping_sub(b1) >> ROW_SHIFT) as i16;
    r[2] = (a2.wrapping_add(b2) >> ROW_SHIFT) as i16;
    r[5] = (a2.wrapping_sub(b2) >> ROW_SHIFT) as i16;
    r[3] = (a3.wrapping_add(b3) >> ROW_SHIFT) as i16;
    r[4] = (a3.wrapping_sub(b3) >> ROW_SHIFT) as i16;
}

/// Column `x` of the row-transformed block: the eight output samples.
#[inline]
fn column(b: &[i16; 64], x: usize) -> [i32; 8] {
    let c: [i32; 8] = std::array::from_fn(|i| i32::from(b[8 * i + x]));
    // (1 << (COL_SHIFT - 1)) / W4 = 32: the rounding, folded into the DC.
    let mut a0 = W4.wrapping_mul(c[0] + ((1 << (COL_SHIFT - 1)) / W4));
    let mut a1 = a0;
    let mut a2 = a0;
    let mut a3 = a0;
    a0 = a0.wrapping_add(W2.wrapping_mul(c[2])).wrapping_add(W4.wrapping_mul(c[4])).wrapping_add(W6.wrapping_mul(c[6]));
    a1 = a1.wrapping_add(W6.wrapping_mul(c[2])).wrapping_sub(W4.wrapping_mul(c[4])).wrapping_sub(W2.wrapping_mul(c[6]));
    a2 = a2.wrapping_sub(W6.wrapping_mul(c[2])).wrapping_sub(W4.wrapping_mul(c[4])).wrapping_add(W2.wrapping_mul(c[6]));
    a3 = a3.wrapping_sub(W2.wrapping_mul(c[2])).wrapping_add(W4.wrapping_mul(c[4])).wrapping_sub(W6.wrapping_mul(c[6]));
    let b0 = W1
        .wrapping_mul(c[1])
        .wrapping_add(W3.wrapping_mul(c[3]))
        .wrapping_add(W5.wrapping_mul(c[5]))
        .wrapping_add(W7.wrapping_mul(c[7]));
    let b1 = W3
        .wrapping_mul(c[1])
        .wrapping_sub(W7.wrapping_mul(c[3]))
        .wrapping_sub(W1.wrapping_mul(c[5]))
        .wrapping_sub(W5.wrapping_mul(c[7]));
    let b2 = W5
        .wrapping_mul(c[1])
        .wrapping_sub(W1.wrapping_mul(c[3]))
        .wrapping_add(W7.wrapping_mul(c[5]))
        .wrapping_add(W3.wrapping_mul(c[7]));
    let b3 = W7
        .wrapping_mul(c[1])
        .wrapping_sub(W5.wrapping_mul(c[3]))
        .wrapping_add(W3.wrapping_mul(c[5]))
        .wrapping_sub(W1.wrapping_mul(c[7]));
    [
        a0.wrapping_add(b0) >> COL_SHIFT,
        a1.wrapping_add(b1) >> COL_SHIFT,
        a2.wrapping_add(b2) >> COL_SHIFT,
        a3.wrapping_add(b3) >> COL_SHIFT,
        a3.wrapping_sub(b3) >> COL_SHIFT,
        a2.wrapping_sub(b2) >> COL_SHIFT,
        a1.wrapping_sub(b1) >> COL_SHIFT,
        a0.wrapping_sub(b0) >> COL_SHIFT,
    ]
}

fn rows(block: &mut [i16; 64]) {
    for r in block.as_chunks_mut::<8>().0 {
        row(r);
    }
}

/// The IDCT of `block` (coefficients in natural order, row-major), clipped
/// to 0..255 and written to the 8x8 area at `dst[at..]` with `stride`.
pub fn put(block: &mut [i16; 64], dst: &mut [u8], at: usize, stride: usize) {
    rows(block);
    for x in 0..8 {
        for (y, v) in column(block, x).into_iter().enumerate() {
            dst[at + y * stride + x] = v.clamp(0, 255) as u8;
        }
    }
}

/// The IDCT of `block` added to the 8x8 area at `dst[at..]`, clipped.
pub fn add(block: &mut [i16; 64], dst: &mut [u8], at: usize, stride: usize) {
    rows(block);
    for x in 0..8 {
        for (y, v) in column(block, x).into_iter().enumerate() {
            let p = &mut dst[at + y * stride + x];
            *p = (i32::from(*p) + v).clamp(0, 255) as u8;
        }
    }
}

/// The IDCT of `block` without clipping: the residual samples, row-major.
pub fn residual(block: &[i16; 64]) -> [i32; 64] {
    let mut b = *block;
    rows(&mut b);
    let mut out = [0i32; 64];
    for x in 0..8 {
        for (y, v) in column(&b, x).into_iter().enumerate() {
            out[y * 8 + x] = v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::residual;

    /// `k[x][u]` = C(u) cos((2x + 1) u pi / 16) / 2, C(0) = 1 / sqrt(2).
    fn basis() -> [[f64; 8]; 8] {
        std::array::from_fn(|x| {
            std::array::from_fn(|u| {
                let c = if u == 0 { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 };
                c * ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 16.0).cos() / 2.0
            })
        })
    }

    /// The double-precision reference IDCT of IEEE 1180, rounded to the
    /// nearest integer and clipped to -256..255.
    fn reference(block: &[i16; 64]) -> [i32; 64] {
        let k = basis();
        let mut out = [0i32; 64];
        for y in 0..8 {
            for x in 0..8 {
                let mut s = 0.0;
                for v in 0..8 {
                    for u in 0..8 {
                        s += k[x][u] * k[y][v] * f64::from(block[v * 8 + u]);
                    }
                }
                out[y * 8 + x] = s.round().clamp(-256.0, 255.0) as i32;
            }
        }
        out
    }

    /// IEEE 1180-1990's random number generator and input blocks: the
    /// forward DCT of random samples in `-low..=high`, rounded and clipped
    /// to -2048..2047.
    struct Ieee {
        randx: i64,
    }

    impl Ieee {
        fn rand(&mut self, low: i64, high: i64) -> i64 {
            self.randx = (self.randx * 1_103_515_245 + 12345) & 0xffff_ffff;
            let i = self.randx & 0x7fff_fffe;
            let x = i as f64 / 0x7fff_ffff as f64 * (high - low + 1) as f64;
            x as i64 + low
        }

        fn block(&mut self, low: i64, high: i64, sign: i64) -> [i16; 64] {
            let mut px = [0f64; 64];
            for p in px.iter_mut() {
                *p = (sign * self.rand(-low, high)) as f64;
            }
            let k = basis();
            let mut out = [0i16; 64];
            for v in 0..8 {
                for u in 0..8 {
                    let mut s = 0.0;
                    for y in 0..8 {
                        for x in 0..8 {
                            s += px[y * 8 + x] * k[x][u] * k[y][v];
                        }
                    }
                    out[v * 8 + u] = s.round().clamp(-2048.0, 2047.0) as i16;
                }
            }
            out
        }
    }

    /// IEEE 1180-1990: for each range (L, H) and each sign, 10,000 blocks
    /// against the reference: peak error at most 1, per-sample mean square
    /// error at most 0.06, overall at most 0.02, per-sample mean error at
    /// most 0.015 in magnitude, overall at most 0.0015; an all-zero block
    /// gives zero. The blocks are fewer here in debug builds.
    #[test]
    fn ieee1180() {
        let n = if cfg!(debug_assertions) { 1000 } else { 10000 };
        for (low, high) in [(256, 255), (5, 5), (300, 300)] {
            for sign in [1, -1] {
                let mut g = Ieee { randx: 1 };
                let mut err = [0i64; 64];
                let mut sq = [0i64; 64];
                let mut peak = 0;
                for _ in 0..n {
                    let block = g.block(low, high, sign);
                    let ours = residual(&block).map(|v| v.clamp(-256, 255));
                    let refr = reference(&block);
                    for i in 0..64 {
                        let e = i64::from(ours[i] - refr[i]);
                        peak = peak.max(e.abs());
                        err[i] += e;
                        sq[i] += e * e;
                    }
                }
                let n = n as f64;
                assert!(peak <= 1, "peak error {peak} for ({low}, {high}) sign {sign}");
                for i in 0..64 {
                    assert!(sq[i] as f64 / n <= 0.06, "mse at {i}");
                    assert!((err[i] as f64 / n).abs() <= 0.015, "mean error at {i}");
                }
                assert!(sq.iter().sum::<i64>() as f64 / (64.0 * n) <= 0.02);
                assert!((err.iter().sum::<i64>() as f64 / (64.0 * n)).abs() <= 0.0015);
            }
        }
        assert_eq!(residual(&[0; 64]), [0; 64]);
    }
}
