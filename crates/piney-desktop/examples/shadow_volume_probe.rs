//! Answers `tools/test_shadow_rs.py`: one shadow draw a line (the points, the
//! triangles, then as f32 bit patterns in hex the scale, the world matrix,
//! the view's `world_screen` and `view_screen`, near, far, the clip, the
//! light, the length, then the mode, tw and th, then the two depths the
//! buffer is counted against, and optionally the owner's `ccObj::Draw`:
//! 1 its `_SetLWMatrix`, or 2 and four words its model's checked greatest
//! corner), and one line back: the box `ccBbox_SetBox` gives, whether the
//! draw goes on and VU0's vf21 after the check, each GIF packet VU1 kicks,
//! and the buffer's nonzero counts at each depth. VU0's vf21 and VU1's
//! memory carry from one draw to the next, as on the console.

use std::fmt::Write as _;
use std::io::{BufRead, Write};

use piney_data::model::{Kind, Mmat};
use piney_data::shadow::ShadowMesh;
use piney_desktop::shadow::{Mode, Params, Prelude, ShadowView, Vf21, Vu1Carry, draw, gs_triangles, kicks, model_box};

fn main() {
    let alpha = piney_data::tables::kanji::shared().alpha_blend_tbl;
    let (mut vf21, mut carry) = (Vf21::Known([0; 4]), Vu1Carry::default());
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let words: Vec<&str> = line.split_whitespace().collect();
        let int = |k: usize| words[k].parse::<i64>().unwrap();
        let (n, t) = (int(0) as usize, int(1) as usize);
        let positions: Vec<[i16; 3]> = (0..n).map(|i| [0, 1, 2].map(|j| int(2 + 3 * i + j) as i16)).collect();
        let triangles: Vec<[u32; 3]> = (0..t).map(|i| [0, 1, 2].map(|j| int(2 + 3 * n + 3 * i + j) as u32)).collect();
        let mut r = words[2 + 3 * n + 3 * t..].iter();
        let mut hex = || u32::from_str_radix(r.next().unwrap(), 16).unwrap();
        let scale = hex();
        let mut mat = || [0; 4].map(|_| [0; 4].map(|_| hex()));
        let (local_world, world_screen, view_screen) = (mat(), mat(), mat());
        let (near, far) = (hex(), hex());
        let clip = [hex(), hex(), hex(), hex()];
        let light = [hex(), hex(), hex(), hex()];
        let length = hex();
        let ints: Vec<u32> = r.map(|s| s.parse().unwrap()).collect();
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
            writeln!(out, "R").unwrap();
            continue;
        };
        let view = ShadowView { world_screen, view_screen, near, far, clip, offset: [0.0; 2] };
        let mode = Mode::of(ints[0] as u8).expect("a mode the code sets");
        let p = Params { scale, local_world, light, length, mode, tw: ints[1] as u8, th: ints[2] as u8 };
        let mut w = String::new();
        for x in model_box(&mesh, scale).as_flattened() {
            write!(w, "{x:08x} ").unwrap();
        }
        let prelude = match ints.get(5) {
            Some(1) => Prelude { parent: true, checked: None },
            Some(2) => Prelude { parent: true, checked: Some([6, 7, 8, 9].map(|k| ints[k])) },
            _ => Prelude::default(),
        };
        vf21 = prelude.vf21(vf21, &view, &local_world);
        let setup = draw(&mesh, &view, &p, alpha, &mut vf21);
        write!(w, "{}", u8::from(setup.is_some())).unwrap();
        let Vf21::Known(after) = vf21 else { unreachable!("a check leaves its corner") };
        for x in after {
            write!(w, " {x:08x}").unwrap();
        }
        let packets = setup.as_ref().map(|s| kicks(&mesh, s, &mut carry)).unwrap_or_default();
        write!(w, " {}", packets.len()).unwrap();
        for k in packets.iter().map(|k| k.words()) {
            write!(w, " {}", k.len()).unwrap();
            for q in &k {
                write!(w, " {:08x} {:08x} {:08x} {:08x}", q[0], q[1], q[2], q[3]).unwrap();
            }
        }
        let (size_w, size_h) = (1usize << p.tw, 1usize << p.th);
        let tris = setup.map(|s| gs_triangles(&packets, s.offset, alpha[1])).unwrap_or_default();
        for depth in [ints[3], ints[4]] {
            let mut count = vec![0i32; size_w * size_h];
            for tri in &tris {
                tri.pixels(size_w, size_h, |x, y, z| {
                    if z > depth {
                        count[y * size_w + x] += if tri.add { 1 } else { -1 };
                    }
                });
            }
            let nonzero: Vec<(usize, i32)> = count.iter().copied().enumerate().filter(|&(_, c)| c != 0).collect();
            write!(w, " {}", nonzero.len()).unwrap();
            for (i, c) in nonzero {
                write!(w, " {i}:{c}").unwrap();
            }
        }
        writeln!(out, "{w}").unwrap();
        out.flush().unwrap();
    }
}
