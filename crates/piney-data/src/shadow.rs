//! A shadow model's mmat made ready for the shadow volume, as
//! `DecodeShadowModel` (main 0x00140920) leaves it (`docs/engine/shadow.md`):
//! the directions of its faces (`SetShadowNormal2` 0x001400e0: integer cross
//! products, one within cosine 0.99609375 of a kept direction merged into
//! it), each triangle with its direction, and the edges between faces
//! (`SetShadowWork2` 0x001403c0: open, dropped between coplanar faces, or a
//! convex or concave join). A model with more concave joins than convex, 495
//! directions or more, or no edges casts no shadow.

use crate::model::{Kind, Mmat};

/// VU1 has room for 494 directions (`DecodeShadowModel`'s check).
pub const MAX_NORMALS: usize = 495;

/// An edge between two faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Its vertices as the first triangle to have it ran them.
    pub v0: u32,
    pub v1: u32,
    /// The direction of the last triangle to take it, and of the one before
    /// (`-1 - face` while only one has).
    pub face_a: i32,
    pub face_b: i32,
}

/// One mmat of a shadow model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShadowMesh {
    /// The stored positions (`s16 * vertexScale / 4096`).
    pub positions: Vec<[i16; 3]>,
    /// Each direction kept: the integer cross product of its first triangle.
    pub normals: Vec<[i32; 3]>,
    /// Each triangle: its vertices and its direction.
    pub tris: Vec<([u32; 3], u16)>,
    pub edges: Vec<Edge>,
}

fn sub(a: [i16; 3], b: [i16; 3]) -> [i64; 3] {
    [0, 1, 2].map(|k| i64::from(a[k]) - i64::from(b[k]))
}

/// `sceVu0ITOF0Vector` then `sceVu0Normalize` of `(x, y, z)`.
fn unit(n: [i32; 3]) -> [f32; 3] {
    let v = n.map(|c| c as f32);
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let q = 1.0 / len;
    v.map(|c| c * q)
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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
            let face = match s.normals.iter().position(|&k| dot(u, unit(k)) > 0.996_093_75) {
                Some(k) => k,
                None => {
                    s.normals.push(n);
                    s.normals.len() - 1
                }
            };
            s.tris.push((*t, face as u16));
            // SetShadowWork2 for each edge, with the third vertex.
            for (a, b, c) in [(t[0], t[1], t[2]), (t[1], t[2], t[0]), (t[2], t[0], t[1])] {
                let face = face as i32;
                match s.edges.iter().position(|e| e.v1 == a && e.v0 == b) {
                    Some(i) if s.edges[i].face_a == face => {
                        s.edges.swap_remove(i);
                    }
                    Some(i) => {
                        let na = s.normals[s.edges[i].face_a as usize].map(i64::from);
                        let cv = sub(p(c), p(a));
                        let side = (na[0] * cv[0]).wrapping_add(na[1] * cv[1]).wrapping_add(na[2] * cv[2]);
                        if side < 0 {
                            convex += 1;
                        } else {
                            concave += 1;
                        }
                        let e = &mut s.edges[i];
                        e.face_b = e.face_a;
                        e.face_a = face;
                    }
                    None => s.edges.push(Edge { v0: a, v1: b, face_a: face, face_b: -1 - face }),
                }
            }
        }
        if convex < concave || s.normals.len() >= MAX_NORMALS || s.edges.is_empty() {
            return None;
        }
        Some(s)
    }
}
