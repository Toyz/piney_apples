//! A shadow model's mmat made ready for the shadow volume, as
//! `DecodeShadowModel` (main 0x00140920) leaves it (`docs/engine/shadow.md`):
//! the directions of its faces (`SetShadowNormal2` 0x001400e0: integer cross
//! products, one within cosine 0.99609375 of a kept direction merged into
//! it), each triangle with its direction, and the edges between faces
//! (`SetShadowWork2` 0x001403c0: open, dropped between coplanar faces, or a
//! convex or concave join). A model with more concave joins than convex, 495
//! directions or more, or no edges casts no shadow. The same on every disc
//! (MUT 0x00140160, OUT 0x0013e8b0, QUA 0x0013e670), in VU0's arithmetic.

use crate::field::ee;
use crate::model::{Kind, Mmat};
use crate::vu0::{self, ONE};

/// VU1 has room for 494 directions (`DecodeShadowModel`'s check).
pub const MAX_NORMALS: usize = 495;

/// An edge between two faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Its vertices as the first triangle to have it ran them.
    pub v0: u32,
    pub v1: u32,
    /// The direction of the last triangle to take it, and of the one before;
    /// None while only one has (an open edge, whose second side the stream
    /// points at `-1 - face_a`: VU1 reads that flag below the directions).
    pub face_a: u16,
    pub face_b: Option<u16>,
}

/// One mmat of a shadow model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShadowMesh {
    /// The stored positions (`s16 * vertexScale / 4096`).
    pub positions: Vec<[i16; 3]>,
    /// Each direction kept: the integer cross product of its first triangle
    /// ([`direction`] gives the floats VU1 is sent).
    pub normals: Vec<[i32; 3]>,
    /// Each triangle: its vertices and its direction.
    pub tris: Vec<([u32; 3], u16)>,
    pub edges: Vec<Edge>,
}

fn sub(a: [i16; 3], b: [i16; 3]) -> [i64; 3] {
    [0, 1, 2].map(|k| i64::from(a[k]) - i64::from(b[k]))
}

/// `sceVu0ITOF0Vector` then `sceVu0Normalize` of `(x, y, z, 1)`.
fn unit(n: [i32; 3]) -> vu0::V4 {
    vu0::normalize([ee::from_int(n[0]), ee::from_int(n[1]), ee::from_int(n[2]), ONE])
}

/// `c.le.s` against 0.99609375 (`lui 0x3f7f`): a cosine above it merges.
const SAME: u32 = 0x3f7f_0000;
/// 4096.0, `sceVu0DivVector`'s divisor (`lui 0x4580`).
const F4096: u32 = 0x4580_0000;

/// The direction the VIF stream carries for a kept integer one: `vitof12`
/// (a `cvt.s.w` and a divide by 4096), then `sceVu0DivVector` by 4096 (Q =
/// 1 / 4096 by `vdiv`, the lanes times Q by `vmulq`): x, y, z as float bits.
pub fn direction(n: [i32; 3]) -> [u32; 3] {
    let q = ee::div(ONE, F4096);
    n.map(|c| ee::mul(ee::div(ee::from_int(c), F4096), q))
}

impl ShadowMesh {
    /// `DecodeShadowModel` of a shadow mmat; None when it is refused (or
    /// has no vertices).
    pub fn build(mm: &Mmat) -> Option<ShadowMesh> {
        if mm.kind != Kind::Shadow || mm.positions.is_empty() {
            return None;
        }
        let p = |i: u32| mm.positions.get(i as usize).copied().unwrap_or([0; 3]);
        let mut s = ShadowMesh { positions: mm.positions.clone(), ..Default::default() };
        let (mut convex, mut concave) = (0u32, 0u32);
        for t in &mm.triangles {
            // SetShadowNormal2: (p1 - p2) x (p2 - p0), the same product.
            let (d, e) = (sub(p(t[1]), p(t[2])), sub(p(t[2]), p(t[0])));
            let n = [
                (d[1] * e[2]).wrapping_sub(d[2] * e[1]) as i32,
                (d[2] * e[0]).wrapping_sub(d[0] * e[2]) as i32,
                (d[0] * e[1]).wrapping_sub(d[1] * e[0]) as i32,
            ];
            let u = unit(n);
            let face = match s.normals.iter().position(|&k| ee::lt(SAME, vu0::dot(u, unit(k)))) {
                Some(k) => k,
                None => {
                    s.normals.push(n);
                    s.normals.len() - 1
                }
            };
            s.tris.push((*t, face as u16));
            // SetShadowWork2 for each edge, with the third vertex.
            for (a, b, c) in [(t[0], t[1], t[2]), (t[1], t[2], t[0]), (t[2], t[0], t[1])] {
                let face = face as u16;
                match s.edges.iter().position(|e| e.v1 == a && e.v0 == b) {
                    Some(i) if s.edges[i].face_a == face => {
                        s.edges.swap_remove(i);
                    }
                    Some(i) => {
                        let na = s.normals[usize::from(s.edges[i].face_a)].map(i64::from);
                        let cv = sub(p(c), p(a));
                        let side = (na[0] * cv[0]).wrapping_add(na[1] * cv[1]).wrapping_add(na[2] * cv[2]);
                        if side < 0 {
                            convex += 1;
                        } else {
                            concave += 1;
                        }
                        let e = &mut s.edges[i];
                        e.face_b = Some(e.face_a);
                        e.face_a = face;
                    }
                    None => s.edges.push(Edge { v0: a, v1: b, face_a: face, face_b: None }),
                }
            }
        }
        if convex < concave || s.normals.len() >= MAX_NORMALS || s.edges.is_empty() {
            return None;
        }
        Some(s)
    }
}
