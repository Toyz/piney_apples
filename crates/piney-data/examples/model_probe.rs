//! Answers `tools/test_ccs.py`: every model `model::models` decodes from a
//! DATA.BIN (the path is the argument), as little-endian words on stdout,
//! for the Python reader to be compared with word for word.
//!
//! ```text
//! member   u32 name length, name, u32 model count
//! model    u32 offset, u32 object, u32 scale bits, u16 mtype, u16 mmat count
//! mmat     u8 kind (0 rigid, 1 bone, 2 skin, 3 shadow), u32 name, material,
//!          slot (0xffffffff for none), then each list as u32 count and items:
//!          positions 3 x i16, normals 3 x i8, flags u8, colours 4 x u8,
//!          uvs 2 x u16, skin vertices (u8 entries, each u8 slot, u16
//!          weight, 3 x i16, 3 x i8), triangles 3 x u32
//! ```

use std::io::Write;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::model::{self, Kind, Mmat};

fn list<T>(out: &mut Vec<u8>, items: &[T], mut item: impl FnMut(&mut Vec<u8>, &T)) {
    out.extend((items.len() as u32).to_le_bytes());
    for t in items {
        item(out, t);
    }
}

fn mmat(out: &mut Vec<u8>, mm: &Mmat) {
    out.push(match mm.kind {
        Kind::Rigid => 0,
        Kind::Bone => 1,
        Kind::Skin => 2,
        Kind::Shadow => 3,
    });
    for v in [mm.name, mm.material, mm.slot] {
        out.extend(v.unwrap_or(u32::MAX).to_le_bytes());
    }
    list(out, &mm.positions, |o, p| p.iter().for_each(|c| o.extend(c.to_le_bytes())));
    list(out, &mm.normals, |o, n| o.extend(n.map(|c| c as u8)));
    list(out, &mm.flags, |o, f| o.push(*f));
    list(out, &mm.colours, |o, c| o.extend(c));
    list(out, &mm.uvs, |o, uv| uv.iter().for_each(|c| o.extend(c.to_le_bytes())));
    list(out, &mm.skin, |o, entries| {
        o.push(entries.len() as u8);
        for e in entries {
            o.push(e.slot);
            o.extend(e.weight.to_le_bytes());
            e.position.iter().for_each(|c| o.extend(c.to_le_bytes()));
            o.extend(e.normal.map(|c| c as u8));
        }
    });
    list(out, &mm.triangles, |o, t| t.iter().for_each(|c| o.extend(c.to_le_bytes())));
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: model_probe DATA.BIN");
    let arc = Archive::new(std::fs::read(path).unwrap()).unwrap();
    let mut stdout = std::io::stdout().lock();
    for m in arc.members() {
        let c = Ccs::parse(arc.inflate(m).unwrap()).unwrap();
        let models = model::models(&c).unwrap();
        let mut out = Vec::new();
        list(&mut out, m.name.as_bytes(), |o, b| o.push(*b));
        out.extend((models.len() as u32).to_le_bytes());
        for md in &models {
            out.extend((md.offset as u32).to_le_bytes());
            out.extend(md.object.to_le_bytes());
            out.extend(md.scale.to_bits().to_le_bytes());
            out.extend(md.mtype.to_le_bytes());
            out.extend((md.mmats.len() as u16).to_le_bytes());
            md.mmats.iter().for_each(|mm| mmat(&mut out, mm));
        }
        stdout.write_all(&out).unwrap();
    }
}
