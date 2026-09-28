//! The IPU's colour space conversion (the CSC command), YCbCr 4:2:0 to
//! RGBA32, as the game's movies are shown.
//!
//! libmpeg sends CSC with OFM 0 (RGBA32) and DTE 0 (no dither): `_doCSC`
//! (INF SLUS_202.67:0x00116010) writes `0x70000000 | mbc`, `_ch3dmaCSC`
//! likewise; `sceIpuInit` sends SETTH with both thresholds 0, so every
//! alpha is 0x80. The conversion is the integer one the EE User's Manual
//! gives for the IPU (BT.601, studio range), with coefficients in 1/128
//! and a final rounding halving, as PCSX2's `yuv2rgb_reference` transcribes
//! it:
//!
//! ```text
//! lum = (0x95 * max(0, Y - 16)) >> 6          1.1640625
//! rcr = ( 0xcc  * (Cr - 128)) >> 6            1.59375
//! gcr = (-0x68  * (Cr - 128)) >> 6           -0.8125
//! gcb = (-0x32  * (Cb - 128)) >> 6           -0.390625
//! bcb = ( 0x102 * (Cb - 128)) >> 6            2.015625
//! R = clamp((lum + rcr + 1) >> 1), G = clamp((lum + gcr + gcb + 1) >> 1),
//! B = clamp((lum + bcb + 1) >> 1), A = 0x80
//! ```
//!
//! (`>>` is arithmetic: it floors.) Chroma is not interpolated: the four
//! samples of each 2 x 2 square share one Cb and one Cr.

use crate::decoder::Picture;

/// One sample: Y, Cb, Cr to R, G, B, A (A 0x80, GS units).
#[inline]
pub fn ipu_rgba(y: u8, cb: u8, cr: u8) -> [u8; 4] {
    let lum = (0x95 * (i32::from(y) - 16).max(0)) >> 6;
    let cr = i32::from(cr) - 128;
    let cb = i32::from(cb) - 128;
    let rcr = (0xcc * cr) >> 6;
    let gcr = (-0x68 * cr) >> 6;
    let gcb = (-0x32 * cb) >> 6;
    let bcb = (0x102 * cb) >> 6;
    let c = |v: i32| ((v + 1) >> 1).clamp(0, 255) as u8;
    [c(lum + rcr), c(lum + gcr + gcb), c(lum + bcb), 0x80]
}

impl Picture {
    /// The picture as the IPU's RGBA32 output: `width` x `height` texels,
    /// row-major, R, G, B, A.
    pub fn to_rgba(&self) -> Vec<u8> {
        let cw = self.width.div_ceil(2);
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for y in 0..self.height {
            let luma = &self.y[y * self.width..(y + 1) * self.width];
            let crow = (y / 2) * cw;
            for (x, &l) in luma.iter().enumerate() {
                let c = crow + x / 2;
                out.extend_from_slice(&ipu_rgba(l, self.cb[c], self.cr[c]));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::ipu_rgba;

    #[test]
    fn studio_range() {
        // Black and white of BT.601 studio range come out full range.
        assert_eq!(ipu_rgba(16, 128, 128), [0, 0, 0, 0x80]);
        assert_eq!(ipu_rgba(235, 128, 128), [255, 255, 255, 0x80]);
        // Below 16 clamps to black. BT.601 red (81, 90, 240) falls a level
        // short: lum 151, rcr 357, (151 + 357 + 1) >> 1 = 254.
        assert_eq!(ipu_rgba(0, 128, 128), [0, 0, 0, 0x80]);
        assert_eq!(ipu_rgba(81, 90, 240), [254, 0, 0, 0x80]);
        // Mid grey: lum = (0x95 * 110) >> 6 = 256, (256 + 1) >> 1 = 128.
        assert_eq!(ipu_rgba(126, 128, 128), [128, 128, 128, 0x80]);
    }
}
