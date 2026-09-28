//! Answers `tools/test_shadow_rs.py`: one shadow mmat a line, what
//! `ShadowMesh::build` makes of it, one line back.
//!
//! ```text
//! request   N T  x y z (N times)  a b c (T times)
//! answer    R                                  (refused)
//!           NN x y z ... NT a b c face ... NE v0 v1 faceA faceB ...
//! ```

use std::io::{BufRead, Write};

use piney_data::model::{Kind, Mmat};
use piney_data::shadow::ShadowMesh;

fn main() {
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut it = line.split_whitespace().map(|t| t.parse::<i64>().unwrap());
        let mut next = || it.next().unwrap();
        let (n, t) = (next() as usize, next() as usize);
        let positions: Vec<[i16; 3]> = (0..n).map(|_| [0; 3].map(|_| next() as i16)).collect();
        let triangles: Vec<[u32; 3]> = (0..t).map(|_| [0; 3].map(|_| next() as u32)).collect();
        let mm = Mmat {
            kind: Kind::Shadow,
            name: None,
            material: None,
            slot: None,
            positions,
            normals: Vec::new(),
            flags: Vec::new(),
            colours: Vec::new(),
            uvs: Vec::new(),
            skin: Vec::new(),
            triangles,
        };
        let Some(s) = ShadowMesh::build(&mm) else {
            writeln!(out, "R").unwrap();
            continue;
        };
        let mut w = format!("{}", s.normals.len());
        for n in &s.normals {
            w += &format!(" {} {} {}", n[0], n[1], n[2]);
        }
        w += &format!(" {}", s.tris.len());
        for (v, f) in &s.tris {
            w += &format!(" {} {} {} {}", v[0], v[1], v[2], f);
        }
        w += &format!(" {}", s.edges.len());
        for e in &s.edges {
            w += &format!(" {} {} {} {}", e.v0, e.v1, e.face_a, e.face_b);
        }
        writeln!(out, "{w}").unwrap();
        out.flush().unwrap();
    }
}
