//! The CCSF scene file (`docs/formats/ccs.md`).
//!
//! A flat run of chunks: `u16 kind, u16 tag, u32 size in words, payload`.
//! The game never skips a chunk by `size`: `ccStream::DecodeSetupSection`
//! dispatches on `kind` and each decoder reads what it needs. `size` is right
//! for every kind except textures and `mtype & 4` models, whose lengths
//! follow from their own fields; with those two rules every file on the
//! discs walks to its last byte. After the Frame chunk (0x0005) the frame
//! section has its own kinds, each exactly `size` words.

use crate::{Bytes, Result, align4, cstr, format_err};

pub const HEADER: u16 = 0x0001;
pub const INDEX: u16 = 0x0002;
pub const SETUP: u16 = 0x0003;
pub const FRAME: u16 = 0x0005;
pub const OBJ: u16 = 0x0100;
pub const MATERIAL: u16 = 0x0200;
pub const TEXTURE: u16 = 0x0300;
pub const CLUT: u16 = 0x0400;
pub const ANIME: u16 = 0x0700;
pub const MODEL: u16 = 0x0800;
pub const CLUMP: u16 = 0x0900;
pub const EXT_OBJ: u16 = 0x0a00;
pub const DUMMY_POS: u16 = 0x1300;
pub const DUMMY_POS_ROT: u16 = 0x1400;
pub const TOP: u16 = 0xff01;
pub const F_OBJ: u16 = 0x0101;

/// Kinds `DecodeSetupSection` dispatches on (0x00149ff0).
const SETUP_KINDS: &[u16] = &[
    0x0003, 0x0005, 0x0100, 0x0200, 0x0300, 0x0400, 0x0500, 0x0600, 0x0700, 0x0800, 0x0900, 0x0a00, 0x0b00, 0x0c00,
    0x0d00, 0x0e00, 0x1000, 0x1100, 0x1200, 0x1300, 0x1400, 0x1700, 0x1800, 0x1900, 0x2000, 0x2200,
];
/// Kinds `DecodeFrameChunk` dispatches on (0x0014e1b0).
const FRAME_KINDS: &[u16] = &[
    0xff01, 0x0005, 0x0101, 0x0108, 0x0201, 0x0502, 0x0601, 0x0602, 0x0604, 0x0606, 0x0608, 0x0802, 0x0803, 0x1801,
    0x1901, 0x2201,
];

pub fn kind_name(kind: u16) -> &'static str {
    match kind {
        0x0001 => "Header",
        0x0002 => "Index",
        0x0003 => "Setup",
        0x0005 => "Frame",
        0x0100 => "Obj",
        0x0200 => "Material",
        0x0300 => "Texture",
        0x0400 => "Clut",
        0x0500 => "Camera",
        0x0600 => "Light",
        0x0700 => "Anime",
        0x0800 => "Model",
        0x0900 => "Clump",
        0x0a00 => "ExtObj",
        0x0b00 => "Hit",
        0x0c00 => "Bbox",
        0x0d00 => "Particle",
        0x0e00 => "Eff",
        0x1000 => "BltGrp",
        0x1100 => "FBRect",
        0x1200 => "FBPage",
        0x1300 => "DummyPos",
        0x1400 => "DummyPosRot",
        0x1700 => "Layer",
        0x1800 => "Shadow",
        0x1900 => "Morpher",
        0x2000 => "Obj2",
        0x2200 => "Pcm",
        0xff01 => "Top",
        0x0101 => "F_Obj",
        0x0108 => "F_Note",
        0x0201 => "F_Material",
        0x0502 => "F_Camera",
        0x0601 => "F_Ambient",
        0x0602 => "F_DistantLight",
        0x0604 => "F_DirectLight",
        0x0606 => "F_SpotLight",
        0x0608 => "F_OmniLight",
        0x0802 => "F_ModelVertex",
        0x0803 => "F_ModelNormal",
        0x1801 => "F_Shadow",
        0x1901 => "F_Morpher",
        0x2201 => "F_Pcm",
        _ => "?",
    }
}

#[derive(Clone, Debug)]
pub struct Object {
    /// `PREFIX_name`: `TEX_`, `MDL_`, `OBJ_`, `MAT_`, `ANM_` ...
    pub name: String,
    /// Index into [`Ccs::files`]; a path starting `#` is defined in another
    /// file loaded alongside.
    pub file: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Chunk {
    pub offset: usize,
    pub kind: u16,
    pub tag: u16,
    /// The header's size field, in words; not always the real length.
    pub size_words: u32,
    /// Where the next chunk starts, as the decoder reads it.
    pub end: usize,
    /// True from the first chunk after the Frame chunk on.
    pub in_frames: bool,
}

impl Chunk {
    pub fn payload(&self) -> usize {
        self.offset + 8
    }
}

/// The result of walking a file's chunks.
pub struct Walk {
    pub chunks: Vec<Chunk>,
    /// Where the walk met something that is not a chunk header, if it did.
    pub lost_at: Option<usize>,
}

impl Walk {
    pub fn clean(&self, len: usize) -> bool {
        self.lost_at.is_none() && self.chunks.last().map(|c| c.end) == Some(len)
    }
}

pub struct Ccs {
    pub data: Vec<u8>,
    pub name: String,
    pub version: u16,
    pub frames: u32,
    pub header_words: Vec<u32>,
    pub files: Vec<String>,
    pub objects: Vec<Object>,
}

impl Ccs {
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.u32_at(0)? != 0xcccc_0001 || data.slice_at(8, 4)? != b"CCSF" {
            return format_err("not a CCSF file");
        }
        let name = cstr(data.slice_at(12, 32)?);
        // DecodeHeaderSection (0x00149d20): u16 version, pad to 4, u32 frames,
        // u32 count, u32[count].
        let version = data.u16_at(44)?;
        let frames = data.u32_at(48)?;
        let count = data.u32_at(52)? as usize;
        let header_words = (0..count).map(|k| data.u32_at(56 + 4 * k)).collect::<Result<_>>()?;
        let index = 56 + 4 * count;
        if data.u32_at(index)? != 0xcccc_0002 {
            return format_err(format!("chunk at 0x{index:x} is not the name table"));
        }
        let fc = data.u32_at(index + 8)? as usize;
        let oc = data.u32_at(index + 12)? as usize;
        let base = index + 16;
        let files = (0..fc).map(|k| Ok(cstr(data.slice_at(base + 32 * k, 32)?))).collect::<Result<_>>()?;
        let objects = (0..oc)
            .map(|k| {
                let raw = data.slice_at(base + 32 * (fc + k), 32)?;
                Ok(Object { name: cstr(&raw[..30]), file: raw.u16_at(30)? })
            })
            .collect::<Result<_>>()?;
        Ok(Ccs { data, name, version, frames, header_words, files, objects })
    }

    pub fn object_name(&self, index: u32) -> Option<&str> {
        match index as usize {
            0 => None,
            i => self.objects.get(i).map(|o| o.name.as_str()),
        }
    }

    pub fn find_object(&self, name: &str) -> Option<u32> {
        self.objects.iter().position(|o| o.name == name).map(|i| i as u32)
    }

    /// The object's defining file path, without the leading ' ' or '#'.
    pub fn object_file(&self, index: u32) -> Option<&str> {
        let o = self.objects.get(index as usize)?;
        self.files.get(o.file as usize).map(|f| f.get(1..).unwrap_or(""))
    }

    /// End of the texture chunk at `p`, as `ccStream::Decode_Texture`
    /// (0x0014d9a0) reads it.
    pub fn texture_end(&self, p: usize) -> Result<usize> {
        let d = &self.data;
        let mut q = p + 8 + if self.version >= 0x92 { 12 } else { 8 };
        let mut mipmaps = 0;
        if self.version >= 0x90 {
            mipmaps = d.u8_at(q + 2)?;
            q += 8;
        } else {
            q += 4;
        }
        for _ in 0..=mipmaps {
            let n = d.u32_at(q + 4)? as usize;
            q += 8 + 4 * n;
        }
        Ok(q)
    }

    /// End of the model chunk at `p`, as `ccStream::Decode_Model` (0x0014bce0)
    /// and the mmat decoders it calls read it.
    pub fn model_end(&self, p: usize) -> Result<usize> {
        let d = &self.data;
        let mut mtype = d.u16_at(p + 16)?;
        let count = d.u16_at(p + 18)?;
        if self.version < 0x100 {
            mtype &= 0xff;
        }
        let mut q = p + 8 + 20;
        for _ in 0..count {
            let (vn, on);
            if mtype & 2 != 0 {
                vn = d.u32_at(q + 8)? as usize;
                on = d.u32_at(q + 16)? as usize;
                q += 20;
            } else if mtype & 4 != 0 {
                vn = d.u32_at(q + 4)? as usize;
                on = d.u32_at(q + 8)? as usize;
                q += 12;
            } else if mtype & 8 != 0 {
                vn = 0;
                on = 0;
            } else {
                vn = d.u32_at(q + 8)? as usize;
                on = 0;
                q += 12;
            }
            if mtype & 4 != 0 {
                if on > 0 {
                    let mut ends = 0;
                    for k in 0..on {
                        ends += (d.u8_at(q + 8 * k + 7)? >> 1 & 1) as usize;
                    }
                    q += 12 * on + 4 * ends;
                } else {
                    q = align4(q + 4 + 6 * vn) + 8 * vn;
                }
            } else if mtype & 8 != 0 {
                let vnum = d.i32_at(q)?;
                let inum = d.i32_at(q + 4)?;
                q += 8;
                if vnum != 0 {
                    q = align4(q + 6 * vnum as usize) + 12 * (inum / 3) as usize;
                }
            } else {
                q = align4(q + 6 * vn);
                for bit in [0x40, 0x200, 0x400] {
                    if mtype & bit == 0 {
                        q += 4 * vn;
                    }
                }
            }
        }
        Ok(q)
    }

    /// Walk every chunk, as far as the walk stays on the rails.
    pub fn walk(&self) -> Walk {
        let d = &self.data;
        let mut chunks = Vec::new();
        let mut p = 0usize;
        let mut in_frames = false;
        while p + 8 <= d.len() {
            let (Ok(word), Ok(size)) = (d.u32_at(p), d.u32_at(p + 4)) else { break };
            let kind = word as u16;
            let tag = (word >> 16) as u16;
            let known = if in_frames {
                FRAME_KINDS.contains(&kind)
            } else {
                kind == HEADER || kind == INDEX || SETUP_KINDS.contains(&kind)
            };
            if !(tag == 0xcccc || tag == 0) || !known {
                return Walk { chunks, lost_at: Some(p) };
            }
            let end = if in_frames {
                Ok(p + 8 + 4 * size as usize)
            } else if kind == TEXTURE {
                self.texture_end(p)
            } else if kind == MODEL {
                self.model_end(p)
            } else {
                Ok(p + 8 + 4 * size as usize)
            };
            let Ok(end) = end else { return Walk { chunks, lost_at: Some(p) } };
            if end > d.len() {
                return Walk { chunks, lost_at: Some(p) };
            }
            chunks.push(Chunk { offset: p, kind, tag, size_words: size, end, in_frames });
            if kind == FRAME && !in_frames {
                in_frames = true;
            }
            p = end;
        }
        Walk { chunks, lost_at: None }
    }
}
