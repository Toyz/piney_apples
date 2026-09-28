//! `ccEff` (main 0x0013ba20 `Init`, 0x0013bcb0 `Draw`): a textured quad
//! facing the camera, one pattern (a UV rectangle and a transparency) at a
//! time, from a scene file's `EFF_` object, the 0x0e00 chunk
//! (`ccStream::Decode_Eff` 0x0014cca0; its layout is in
//! docs/engine/effects.md, "The Eff chunk"). What an effect sets before
//! `Draw(pattern)` is [`Eff`]; how it reaches the GS is [`Eff::render`]
//! (`crate::sprite`).

use piney_data::ccs::Ccs;

use crate::ee::{F, ONE, V4};
use crate::files::ObjRef;

/// One pattern: `ccEffUVPat`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UvPat {
    pub u: u16,
    pub v: u16,
    /// 4096 is opaque.
    pub transparency: u16,
}

/// A decoded 0x0e00 chunk (`ccEffChunk`).
#[derive(Clone, Debug, PartialEq)]
pub struct EffChunk {
    pub object: u32,
    pub texture: u32,
    pub flag: u16,
    pub zoffs: i16,
    pub unknown_0c: u16,
    pub x0: F,
    pub y0: F,
    pub x1: F,
    pub y1: F,
    pub w: u16,
    pub h: u16,
    pub pats: Vec<UvPat>,
}

impl EffChunk {
    /// `patNum`.
    pub fn pat_num(&self) -> u16 {
        self.pats.len() as u16
    }
}

/// Every Eff chunk of a file's setup section.
pub fn decode(ccs: &Ccs) -> Vec<EffChunk> {
    let d = &ccs.data;
    let u16_at = |p: usize| u16::from_le_bytes([d[p], d[p + 1]]);
    let u32_at = |p: usize| u32::from_le_bytes(d[p..p + 4].try_into().unwrap());
    let mut out = Vec::new();
    for ch in ccs.walk().chunks {
        if ch.in_frames || ch.kind != 0x0e00 {
            continue;
        }
        let p = ch.payload();
        if p + 36 > d.len() {
            continue;
        }
        let n = usize::from(u16_at(p + 14));
        if p + 36 + 8 * n > d.len() {
            continue;
        }
        let pats = (0..n)
            .map(|i| {
                let q = p + 36 + 8 * i;
                UvPat { u: u16_at(q), v: u16_at(q + 2), transparency: u16_at(q + 4) }
            })
            .collect();
        out.push(EffChunk {
            object: u32_at(p),
            texture: u32_at(p + 4),
            flag: u16_at(p + 8),
            zoffs: u16_at(p + 10) as i16,
            unknown_0c: u16_at(p + 12),
            x0: u32_at(p + 16),
            y0: u32_at(p + 20),
            x1: u32_at(p + 24),
            y1: u32_at(p + 28),
            w: u16_at(p + 32),
            h: u16_at(p + 34),
            pats,
        });
    }
    out
}

/// A `ccEff` (0x70 bytes): its chunk, and what `Init` sets and the effects
/// change before each `Draw`.
#[derive(Clone, Debug, PartialEq)]
pub struct Eff {
    /// Which file of the effect files, and which of its Eff chunks.
    pub file: usize,
    pub chunk: usize,
    /// +0x00 x0, y0, x1, y1.
    pub x0: F,
    pub y0: F,
    pub x1: F,
    pub y1: F,
    /// +0x10: where it is drawn, set by the caller before each `Draw`.
    pub pos: V4,
    /// +0x20, +0x24, +0x28.
    pub scale_x: F,
    pub scale_y: F,
    pub rotate: F,
    /// +0x2c: RGB (0x80 = 1), the alpha byte ignored.
    pub color: u32,
    /// +0x30 `patNum`, +0x32 `zoffs`.
    pub pat_num: u16,
    pub zoffs: i16,
    /// +0x34.
    pub transparency: F,
    /// +0x4c `wh`.
    pub wh: u32,
    /// +0x50 ALPHA_1, +0x58 TEST_1.
    pub alpha: u64,
    pub test: u64,
    /// +0x60 `flag`, +0x62 `prim`.
    pub flag: u16,
    pub prim: u16,
    /// +0x3c `tex.clutChunk` when a particle's texture id swaps in a CLUT
    /// of its own (None: the texture's, as `Init` sets it).
    pub clut: Option<ObjRef>,
}

impl Eff {
    /// `ccEff::Init(chunk, fogSw)` (main 0x0013ba20): the chunk's corners,
    /// patterns and flag; at the origin, unscaled, unturned, grey (0x808080),
    /// opaque; PRIM a textured, blended triangle strip (fog by `fog_sw`),
    /// TEST z GEQUAL, then `SetBlendType(flag & 7)`.
    pub fn init(file: usize, chunk: usize, c: &EffChunk, fog_sw: bool, alpha_blend: &[u64; 8]) -> Eff {
        let mut e = Eff {
            file,
            chunk,
            x0: c.x0,
            y0: c.y0,
            x1: c.x1,
            y1: c.y1,
            pos: [0, 0, 0, ONE],
            scale_x: ONE,
            scale_y: ONE,
            rotate: 0,
            color: 0x0080_8080,
            pat_num: c.pat_num(),
            zoffs: c.zoffs,
            transparency: ONE,
            wh: u32::from(c.w) | u32::from(c.h) << 16,
            alpha: 0,
            test: 0x5_0000,
            flag: c.flag,
            prim: (u16::from(fog_sw) << 5) | 0x54,
            clut: None,
        };
        e.set_blend_type(i32::from(c.flag & 7), alpha_blend);
        e
    }

    /// `ccEff::SetBlendType(type)` (0x0013bc00): ALPHA from
    /// `alphaBlendTbl[type]`; TEST's alpha half becomes, for type 0, alpha
    /// GEQUAL the reference (ATE, ATST 5; `Draw` fills the reference in) and
    /// flag bit 0x20 clears; for any other, ATST 0 (never) with AFAIL 1
    /// (frame buffer only: colour written, depth not) and the bit sets.
    pub fn set_blend_type(&mut self, t: i32, alpha_blend: &[u64; 8]) {
        self.alpha = alpha_blend[(t & 7) as usize];
        let keep = self.test & 0xffff_ffff_ffff_cff0;
        if t == 0 {
            self.test = keep | 0x100b;
            self.flag &= 0xffdf;
        } else {
            self.test = keep | 0x1001;
            self.flag |= 0x20;
        }
    }
}
