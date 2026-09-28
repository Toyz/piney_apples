//! Palettes (0x0400) and textures (0x0300), as `ccStream::Decode_Clut`
//! (0x0014d660) and `Decode_Texture` (0x0014d9a0) read them.
//!
//! Palette entries are in logical order in the file (the game swizzles
//! 256-entry palettes into the GS's CSM1 order itself), so they are used as
//! they are. GS alpha runs 0..0x80. Pixel rows are stored bottom-up - the art
//! was authored as `.bmp` - and a model's T = 0 is the first stored row, so
//! [`Texture::rgba`] keeps the stored order: upload it as is and sample with
//! v = T / 256. [`flip_rows`] turns it the right way up for viewing.

use std::collections::HashMap;

use crate::ccs::{self, Ccs};
use crate::{Bytes, Result, format_err};

pub const PSMT8: u8 = 0x13;
pub const PSMT4: u8 = 0x14;

#[derive(Clone, Debug)]
pub struct Clut {
    pub object: u32,
    pub flag: u8,
    pub buf_x: u16,
    pub buf_y: u16,
    /// R, G, B, A with A = 0x80 opaque.
    pub colours: Vec<[u8; 4]>,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub buf_x: u16,
    pub buf_y: u16,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub object: u32,
    pub clut: u32,
    pub blt_group: Option<u32>,
    pub flag: u8,
    pub psm: u8,
    pub mipmap: u8,
    /// Alpha-test reference.
    pub aref: u8,
    /// log2 width and height.
    pub tw: u8,
    pub th: u8,
    pub levels: Vec<Level>,
}

impl Texture {
    pub fn width(&self, level: usize) -> u32 {
        (1u32 << self.tw) >> level
    }

    pub fn height(&self, level: usize) -> u32 {
        (1u32 << self.th) >> level
    }

    /// RGBA8 in the stored (bottom-up) row order, alpha scaled from 0..0x80
    /// to 0..255 (0x80 and above is 255).
    pub fn rgba(&self, clut: &Clut, level: usize) -> Result<Vec<u8>> {
        let (w, h) = (self.width(level) as usize, self.height(level) as usize);
        let Some(lv) = self.levels.get(level) else {
            return format_err(format!("no mip level {level}"));
        };
        let palette: Vec<[u8; 4]> =
            clut.colours.iter().map(|c| [c[0], c[1], c[2], (c[3] as u16 * 2).min(255) as u8]).collect();
        let colour = |i: usize| palette.get(i).copied().unwrap_or([255, 0, 255, 255]);
        let mut out = Vec::with_capacity(w * h * 4);
        match self.psm {
            PSMT8 => {
                for i in 0..w * h {
                    out.extend_from_slice(&colour(lv.pixels.u8_at(i)? as usize));
                }
            }
            PSMT4 => {
                for i in 0..w * h {
                    let b = lv.pixels.u8_at(i >> 1)?;
                    let idx = if i & 1 == 1 { b >> 4 } else { b & 15 };
                    out.extend_from_slice(&colour(idx as usize));
                }
            }
            psm => return format_err(format!("psm 0x{psm:02x} not handled")),
        }
        Ok(out)
    }
}

/// Reverse the row order of an RGBA8 image in place.
pub fn flip_rows(rgba: &mut [u8], width: u32) {
    let stride = width as usize * 4;
    let rows = rgba.len() / stride;
    for y in 0..rows / 2 {
        let (top, bottom) = rgba.split_at_mut((rows - 1 - y) * stride);
        top[y * stride..(y + 1) * stride].swap_with_slice(&mut bottom[..stride]);
    }
}

/// Every texture and palette in the file, as far as its walk goes.
pub fn read(c: &Ccs) -> Result<(Vec<Texture>, HashMap<u32, Clut>)> {
    let d = &c.data;
    let mut textures = Vec::new();
    let mut cluts = HashMap::new();
    for ch in c.walk().chunks {
        if ch.in_frames {
            continue;
        }
        let mut q = ch.payload();
        match ch.kind {
            ccs::CLUT => {
                let object = d.u32_at(q)?;
                q += if c.version >= 0x92 { 8 } else { 4 };
                let flag = d.u8_at(q)?;
                let buf_x = d.u16_at(q + 4)?;
                let buf_y = d.u16_at(q + 6)?;
                let count = d.u32_at(q + 8)? as usize;
                q += 12;
                let colours =
                    (0..count).map(|i| Ok(d.slice_at(q + 4 * i, 4)?.try_into().unwrap())).collect::<Result<_>>()?;
                cluts.insert(object, Clut { object, flag, buf_x, buf_y, colours });
            }
            ccs::TEXTURE => {
                if c.version < 0x90 {
                    return format_err("textures before version 0x90 carry no size");
                }
                let object = d.u32_at(q)?;
                let clut = d.u32_at(q + 4)?;
                let blt_group = if c.version >= 0x92 { Some(d.u32_at(q + 8)?) } else { None };
                q += if c.version >= 0x92 { 12 } else { 8 };
                let f = d.slice_at(q, 6)?;
                let (flag, psm, mipmap, aref, tw, th) = (f[0], f[1], f[2], f[3], f[4], f[5]);
                q += 8;
                let mut levels = Vec::new();
                for _ in 0..=mipmap {
                    let buf_x = d.u16_at(q)?;
                    let buf_y = d.u16_at(q + 2)?;
                    let count = d.u32_at(q + 4)? as usize;
                    levels.push(Level { buf_x, buf_y, pixels: d.slice_at(q + 8, 4 * count)?.to_vec() });
                    q += 8 + 4 * count;
                }
                textures.push(Texture { object, clut, blt_group, flag, psm, mipmap, aref, tw, th, levels });
            }
            _ => {}
        }
    }
    Ok((textures, cluts))
}
