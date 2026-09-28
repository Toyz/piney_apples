//! Answers `tools/test_shadow_rs.py`: one shadow draw a line (the points, the
//! triangles, the scale, both matrices, near, far, clip, light, length, mode
//! and size), and one line back with the polygons `shadow::volume` sends, each
//! with its winding. Floats go as f32 bit patterns in hex both ways; matrices
//! as columns.

use std::io::{BufRead, Write};

use glam::{Mat4, Vec4};
use piney_data::model::{Kind, Mmat};
use piney_data::shadow::ShadowMesh;
use piney_desktop::shadow::{Params, volume};

fn main() {
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut it = line.split_whitespace();
        let mut int = || it.next().unwrap().parse::<i64>().unwrap();
        let (n, t) = (int() as usize, int() as usize);
        let positions: Vec<[i16; 3]> = (0..n).map(|_| [0; 3].map(|_| int() as i16)).collect();
        let triangles: Vec<[u32; 3]> = (0..t).map(|_| [0; 3].map(|_| int() as u32)).collect();
        let rest: Vec<&str> = line.split_whitespace().skip(2 + 3 * n + 3 * t).collect();
        let mut r = rest.iter();
        let mut f = || f32::from_bits(u32::from_str_radix(r.next().unwrap(), 16).unwrap());
        let scale = f();
        let mut mat = || Mat4::from_cols_array(&[0; 16].map(|_| f()));
        let (local_world, world_screen) = (mat(), mat());
        let (near, far) = (f(), f());
        let clip = [f(), f(), f(), f()];
        let light = Vec4::new(f(), f(), f(), f());
        let length = f();
        let ints: Vec<u16> = rest[rest.len() - 3..].iter().map(|s| s.parse().unwrap()).collect();
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
        let Some(mesh) = ShadowMesh::build(&mm) else {
            writeln!(out, "0").unwrap();
            continue;
        };
        let p = Params {
            scale,
            local_world,
            world_screen,
            near,
            far,
            clip,
            light,
            length,
            mode: ints[0] as u8,
            width: ints[1],
            height: ints[2],
        };
        let polys = volume(&mesh, &p);
        let mut w = format!("{}", polys.len());
        for q in &polys {
            w += &format!(" {} {}", u8::from(q.front), q.verts.len());
            for v in &q.verts {
                w += &format!(" {:08x} {:08x} {:08x}", v[0].to_bits(), v[1].to_bits(), v[2].to_bits());
            }
        }
        writeln!(out, "{w}").unwrap();
        out.flush().unwrap();
    }
}
