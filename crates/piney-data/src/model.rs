//! The Model chunk, 0x0800 (`docs/formats/ccs-model.md`), as
//! `ccStream::Decode_Model` (0x0014bce0) reads it: a header, then per "mmat"
//! (a run of vertices sharing one material) a few header words and a body in
//! one of four layouts chosen by the model's `mtype`:
//!
//! - `mtype & 4`: `Decode_Mmat02` (0x0014a600). With `offsetNum` 0, "bone":
//!   every vertex rides one clump node. Otherwise "skin": each vertex is one
//!   or more weighted entries, each in its own node's space.
//! - `mtype & 8`: `DecodeShadowModel` (0x00140920), a triangle list.
//! - otherwise: `Decode_Mmat01` (0x0014b890), "rigid", in the owning
//!   object's space.
//!
//! Values are kept as stored; the accessors convert them. Positions are
//! `s16 * vertexScale / 4096`, normals `s8 / 64`, colours RGBA with 0x80 =
//! 1.0, ST `u16 / 256` per texture repeat. The normal word's fourth byte is
//! the GS strip flag: 1 starts a strip, 0 closes a triangle with the two
//! vertices before it.

use glam::Vec3;

use crate::ccs::{self, Ccs};
use crate::field::ee;
use crate::{Bytes, Result, align4, format_err};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rigid,
    Bone,
    Skin,
    Shadow,
}

#[derive(Clone, Copy, Debug)]
pub struct SkinEntry {
    /// Clump node slot (bits 10-15 of the weight word).
    pub slot: u8,
    /// 0..=256, 256 = 1.0 (bits 0-8).
    pub weight: u16,
    pub position: [i16; 3],
    pub normal: [i8; 3],
}

#[derive(Clone, Debug)]
pub struct Mmat {
    pub kind: Kind,
    /// The `MDL_x_N` object, where the layout has one.
    pub name: Option<u32>,
    /// The `MAT_` object.
    pub material: Option<u32>,
    /// Bone: the clump node slot every vertex follows.
    pub slot: Option<u32>,
    /// Rigid, bone and shadow vertices; empty for skin.
    pub positions: Vec<[i16; 3]>,
    pub normals: Vec<[i8; 3]>,
    /// Strip flags, one per vertex (skin: per vertex, from its last entry).
    pub flags: Vec<u8>,
    /// Rigid unlit models only.
    pub colours: Vec<[u8; 4]>,
    pub uvs: Vec<[u16; 2]>,
    /// Skin: the weighted entries of each vertex.
    pub skin: Vec<Vec<SkinEntry>>,
    pub triangles: Vec<[u32; 3]>,
}

impl Mmat {
    fn new(kind: Kind) -> Self {
        Mmat {
            kind,
            name: None,
            material: None,
            slot: None,
            positions: Vec::new(),
            normals: Vec::new(),
            flags: Vec::new(),
            colours: Vec::new(),
            uvs: Vec::new(),
            skin: Vec::new(),
            triangles: Vec::new(),
        }
    }

    pub fn vertex_count(&self) -> usize {
        if self.kind == Kind::Skin { self.skin.len() } else { self.positions.len() }
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub offset: usize,
    pub end: usize,
    /// The `MDL_` object.
    pub object: u32,
    pub scale: f32,
    pub mtype: u16,
    pub flag: u16,
    pub zoffs: i16,
    pub mmats: Vec<Mmat>,
}

/// A morph weight as `ccMorpher::Modify` spreads it over the x, y and z
/// halfword lanes: `vftoi12` gives `x` (the 1/4096 fixed point, truncated)
/// in the low word of a register whose next word is the float's `lw` sign
/// extension `y`; `(r << 16 | r) << 16 | r` then leaves `x.lo` in lane 0,
/// `x.lo | x.hi` in lane 1 and `x.lo | x.hi | y.lo` in lane 2. All three are
/// the weight for weights in [0, 8); a negative one leaves y and z at -1.
fn lanes(w: f32) -> [i16; 3] {
    let x = (w * 4096.0) as i32 as u32;
    let y: u32 = if w.to_bits() >> 31 != 0 { 0xffff_ffff } else { 0 };
    let (lo, hi) = (x & 0xffff, x >> 16);
    [lo, lo | hi, lo | hi | (y & 0xffff)].map(|v| v as u16 as i16)
}

impl Model {
    /// World units per stored position unit.
    pub fn unit(&self) -> f32 {
        self.scale / 4096.0
    }

    /// `ccMorpher::Modify` (0x0013af10): mmat `index`'s positions blended
    /// toward the same mmat of each `(target, weight)`, as the EE does it.
    /// A target is first brought to this model's vertex scale (each value
    /// `trunc(v * (target.scale / scale))` in EE arithmetic, once, in place). Each weight is
    /// `vftoi12`'s 1/4096 fixed point, spread over the lanes as [`lanes`]
    /// says; per axis the differences from the
    /// base are 16-bit (`psubh`), the products summed in 32 bits
    /// (`pmaddh`), shifted down 12 (`psraw`) and added back with 16-bit
    /// saturation (`paddsh`). Only rigid positions are touched; None when
    /// the mmat has none.
    pub fn morph(&self, index: usize, targets: &[(&Model, f32)]) -> Option<Vec<[i16; 3]>> {
        let base = &self.mmats.get(index)?.positions;
        if base.is_empty() {
            return None;
        }
        let targets: Vec<(Vec<[i16; 3]>, [i16; 3])> = targets
            .iter()
            .filter_map(|&(t, w)| {
                let tp = &t.mmats.get(index)?.positions;
                // `div.s`, then per value `vitof0`, `mul.s` and `vftoi0`, in
                // the EE's truncating arithmetic.
                let rescale = t.scale != self.scale;
                let f = ee::div(t.scale.to_bits(), self.scale.to_bits());
                let tp = tp
                    .iter()
                    .map(
                        |p| if rescale { p.map(|v| ee::to_int(ee::mul(f, ee::from_int(v.into()))) as i16) } else { *p },
                    )
                    .collect();
                Some((tp, lanes(w)))
            })
            .collect();
        Some(
            base.iter()
                .enumerate()
                .map(|(i, b)| {
                    std::array::from_fn(|c| {
                        let acc = targets.iter().fold(0i32, |acc, (tp, w)| {
                            let t = tp.get(i).map_or(b[c], |p| p[c]);
                            acc.wrapping_add(i32::from(t.wrapping_sub(b[c])) * i32::from(w[c]))
                        });
                        ((acc >> 12) as i16).saturating_add(b[c])
                    })
                })
                .collect(),
        )
    }

    pub fn position(&self, p: [i16; 3]) -> Vec3 {
        Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) * self.unit()
    }
}

pub fn normal(n: [i8; 3]) -> Vec3 {
    Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32) / 64.0
}

pub fn uv(st: [u16; 2]) -> [f32; 2] {
    [st[0] as f32 / 256.0, st[1] as f32 / 256.0]
}

/// Triangles the GS draws for a strip-flagged vertex run. Every vertex with
/// flag 0 closes a triangle with the two before it; winding alternates from
/// the start of each strip (the first of the flag-1 vertices opening it).
pub fn strip_triangles(flags: &[u8]) -> Vec<[u32; 3]> {
    let mut tris = Vec::new();
    let mut start = 0usize;
    for (i, &f) in flags.iter().enumerate() {
        if f != 0 {
            if i == 0 || flags[i - 1] == 0 {
                start = i;
            }
            continue;
        }
        if i < 2 {
            continue;
        }
        let i = i as u32;
        if (i as usize - start) % 2 == 1 {
            tris.push([i - 2, i - 1, i]);
        } else {
            tris.push([i - 1, i - 2, i]);
        }
    }
    tris
}

fn positions(d: &[u8], q: usize, n: usize) -> Result<(Vec<[i16; 3]>, usize)> {
    let out = (0..n)
        .map(|i| Ok([d.i16_at(q + 6 * i)?, d.i16_at(q + 6 * i + 2)?, d.i16_at(q + 6 * i + 4)?]))
        .collect::<Result<_>>()?;
    Ok((out, align4(q + 6 * n)))
}

fn normals(d: &[u8], q: usize, n: usize) -> Result<(Vec<[i8; 3]>, Vec<u8>, usize)> {
    let mut nrm = Vec::with_capacity(n);
    let mut flags = Vec::with_capacity(n);
    for i in 0..n {
        let w = d.slice_at(q + 4 * i, 4)?;
        nrm.push([w[0] as i8, w[1] as i8, w[2] as i8]);
        flags.push(w[3]);
    }
    Ok((nrm, flags, q + 4 * n))
}

fn uvs(d: &[u8], q: usize, n: usize) -> Result<(Vec<[u16; 2]>, usize)> {
    let out = (0..n).map(|i| Ok([d.u16_at(q + 4 * i)?, d.u16_at(q + 4 * i + 2)?])).collect::<Result<_>>()?;
    Ok((out, q + 4 * n))
}

/// Decode the model chunk at `p`; fails if the decode does not end where the
/// chunk walk says it does.
pub fn decode(c: &Ccs, p: usize) -> Result<Model> {
    let d = &c.data;
    let mut q = p + 8;
    let object = d.u32_at(q)?;
    let scale = d.f32_at(q + 4)?;
    let mut mtype = d.u16_at(q + 8)?;
    let count = d.u16_at(q + 10)?;
    let flag = d.u16_at(q + 12)?;
    let zoffs = d.i16_at(q + 14)?;
    if c.version < 0x100 {
        mtype &= 0xff;
    }
    q += 20;
    let mut mmats = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (name, material, vn, on);
        if mtype & 2 != 0 {
            name = Some(d.u32_at(q)?);
            material = Some(d.u32_at(q + 4)?);
            vn = d.u32_at(q + 8)? as usize;
            on = d.u32_at(q + 16)? as usize;
            q += 20;
        } else if mtype & 4 != 0 {
            name = None;
            material = Some(d.u32_at(q)?);
            vn = d.u32_at(q + 4)? as usize;
            on = d.u32_at(q + 8)? as usize;
            q += 12;
        } else if mtype & 8 != 0 {
            name = None;
            material = None;
            vn = 0;
            on = 0;
        } else {
            name = Some(d.u32_at(q)?);
            material = Some(d.u32_at(q + 4)?);
            vn = d.u32_at(q + 8)? as usize;
            on = 0;
            q += 12;
        }
        let mut mm;
        if mtype & 4 != 0 {
            if on > 0 {
                mm = Mmat::new(Kind::Skin);
                let mut entries = Vec::with_capacity(on);
                for k in 0..on {
                    let e = q + 8 * k;
                    entries.push(([d.i16_at(e)?, d.i16_at(e + 2)?, d.i16_at(e + 4)?], d.u16_at(e + 6)?));
                }
                q += 8 * on;
                let (nrm, flags, nq) = normals(d, q, on)?;
                q = nq;
                let mut vertex = Vec::new();
                for (((pos, w), n), f) in entries.into_iter().zip(nrm).zip(flags) {
                    // Low 9 bits weight, bit 9 last entry of the vertex, top 6
                    // bits the clump node (0x0014ab5c).
                    vertex.push(SkinEntry { slot: (w >> 10) as u8, weight: w & 0x1ff, position: pos, normal: n });
                    if w & 0x200 != 0 {
                        mm.skin.push(std::mem::take(&mut vertex));
                        mm.flags.push(f);
                    }
                }
                let (u, nq) = uvs(d, q, mm.skin.len())?;
                mm.uvs = u;
                q = nq;
            } else {
                mm = Mmat::new(Kind::Bone);
                mm.slot = Some(d.u32_at(q)?);
                let (p, nq) = positions(d, q + 4, vn)?;
                let (n, f, nq) = normals(d, nq, vn)?;
                let (u, nq) = uvs(d, nq, vn)?;
                (mm.positions, mm.normals, mm.flags, mm.uvs, q) = (p, n, f, u, nq);
            }
            mm.triangles = strip_triangles(&mm.flags);
        } else if mtype & 8 != 0 {
            mm = Mmat::new(Kind::Shadow);
            let vnum = d.i32_at(q)?;
            let inum = d.i32_at(q + 4)?;
            q += 8;
            if vnum != 0 {
                let (p, nq) = positions(d, q, vnum as usize)?;
                mm.positions = p;
                q = nq;
                for k in 0..(inum / 3) as usize {
                    let t = q + 12 * k;
                    mm.triangles.push([d.i32_at(t)? as u32, d.i32_at(t + 4)? as u32, d.i32_at(t + 8)? as u32]);
                }
                q += 12 * (inum / 3) as usize;
            }
        } else {
            mm = Mmat::new(Kind::Rigid);
            let (p, nq) = positions(d, q, vn)?;
            mm.positions = p;
            q = nq;
            if mtype & 0x40 == 0 {
                let (n, f, nq) = normals(d, q, vn)?;
                (mm.normals, mm.flags, q) = (n, f, nq);
            }
            if mtype & 0x200 == 0 {
                // Read either way; with mtype & 1 (lit) the decoder drops them.
                if mtype & 1 == 0 {
                    mm.colours =
                        (0..vn).map(|i| Ok(d.slice_at(q + 4 * i, 4)?.try_into().unwrap())).collect::<Result<_>>()?;
                }
                q += 4 * vn;
            }
            if mtype & 0x400 == 0 {
                let (u, nq) = uvs(d, q, vn)?;
                (mm.uvs, q) = (u, nq);
            }
            if !mm.flags.is_empty() {
                mm.triangles = strip_triangles(&mm.flags);
            }
        }
        mm.name = name;
        mm.material = material;
        mmats.push(mm);
    }
    let end = c.model_end(p)?;
    if q != end {
        return format_err(format!("model at 0x{p:x} decodes to 0x{q:x}, the walk says 0x{end:x}"));
    }
    Ok(Model { offset: p, end: q, object, scale, mtype, flag, zoffs, mmats })
}

/// Every model in the file's setup section.
pub fn models(c: &Ccs) -> Result<Vec<Model>> {
    c.walk().chunks.iter().filter(|ch| !ch.in_frames && ch.kind == ccs::MODEL).map(|ch| decode(c, ch.offset)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_mmat(scale: f32, positions: Vec<[i16; 3]>) -> Model {
        let mut mm = Mmat::new(Kind::Rigid);
        mm.positions = positions;
        Model { offset: 0, end: 0, object: 0, scale, mtype: 0, flag: 0, zoffs: 0, mmats: vec![mm] }
    }

    /// `ccMorpher::Modify`'s arithmetic: 12-bit weights, an arithmetic
    /// shift (floor), the saturating add, and a target at another scale
    /// brought to the base's first.
    #[test]
    fn morph_blends_as_the_ee_does() {
        let base = one_mmat(1.0, vec![[0, 100, -100], [32000, 0, 0]]);
        let a = one_mmat(1.0, vec![[1000, 200, -101], [32767, 0, 0]]);
        let b = one_mmat(2.0, vec![[500, 50, -50], [0, 0, 0]]);
        // 0.4 is 1638/4096, 0.6 is 2457/4096.
        let m = base.morph(0, &[(&a, 0.4), (&b, 0.6)]).unwrap();
        // x: (1000 * 1638 + 1000 * 2457) >> 12 = 999.
        assert_eq!(m[0][0], 999);
        // y: b brought to scale 1 is 100: (100 * 1638 + 0) >> 12 = 39, + 100.
        assert_eq!(m[0][1], 139);
        // z: (-1 * 1638 + 0) >> 12 = -1 (floor), + -100.
        assert_eq!(m[0][2], -101);
        // (767 * 1638 + -32000 * 2457) >> 12 = -18889, + 32000.
        assert_eq!(m[1][0], 13111);
        assert_eq!(base.morph(0, &[]).unwrap(), base.mmats[0].positions);
        // 32000 - -32000 wraps to -1536 in 16 bits; -32000 - 1536 saturates.
        let low = one_mmat(1.0, vec![[-32000, 0, 0]]);
        let high = one_mmat(1.0, vec![[32000, 0, 0]]);
        assert_eq!(low.morph(0, &[(&high, 1.0)]).unwrap(), vec![[-32768, 0, 0]]);
    }

    #[test]
    fn strips_alternate_from_each_start() {
        // Two strips: 1 1 0 0 | 1 1 0
        let t = strip_triangles(&[1, 1, 0, 0, 1, 1, 0]);
        assert_eq!(t, vec![[1, 0, 2], [1, 2, 3], [5, 4, 6]]);
    }
}
