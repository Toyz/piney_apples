//! Answers `tools/test_ccs.py`: what the port sends VU1 for every bone and
//! skin mmat of a DATA.BIN (the path is the argument), as the words of the
//! packet `Decode_Mmat02` builds unpack them: one line per mmat, `member
//! model-offset mmat-index kind` then hex words. A bone vertex: its position
//! (V3-32), its normal and strip flag (V4-8, signed), its ST (V2-16, the
//! stored halves shifted up 4). A skin entry: weight * 16, its slot's matrix
//! address `20 + 4 slot` and the entries left of its vertex (V3-16), its
//! position, its normal and its vertex's flag; then each vertex's ST.

use std::fmt::Write as _;
use std::io::Write;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::model::{self, Kind};

fn main() {
    let path = std::env::args().nth(1).expect("usage: body_probe DATA.BIN");
    let arc = Archive::new(std::fs::read(path).unwrap()).unwrap();
    let mut stdout = std::io::stdout().lock();
    for m in arc.members() {
        let c = Ccs::parse(arc.inflate(m).unwrap()).unwrap();
        for md in model::models(&c).unwrap() {
            for (k, mm) in md.mmats.iter().enumerate() {
                let mut w = String::new();
                let mut word = |x: u32| write!(w, " {x:x}").unwrap();
                let normal = |n: [i8; 3], flag: u8| [n[0], n[1], n[2], flag as i8].map(|b| i32::from(b) as u32);
                let st = |uv: Option<&[u16; 2]>| model::body_st(uv.copied().unwrap_or([0, 0]));
                match mm.kind {
                    Kind::Bone => {
                        for (i, &p) in mm.positions.iter().enumerate() {
                            md.body_words(p).into_iter().for_each(&mut word);
                            let n = mm.normals.get(i).copied().unwrap_or([0; 3]);
                            normal(n, mm.flags.get(i).copied().unwrap_or(0)).into_iter().for_each(&mut word);
                            st(mm.uvs.get(i)).into_iter().for_each(&mut word);
                        }
                    }
                    Kind::Skin => {
                        for (i, entries) in mm.skin.iter().enumerate() {
                            let flag = mm.flags.get(i).copied().unwrap_or(0);
                            for (j, e) in entries.iter().enumerate() {
                                let left = (entries.len() - j) as u32;
                                [u32::from(e.weight) * 16, 20 + 4 * u32::from(e.slot), left]
                                    .into_iter()
                                    .for_each(&mut word);
                                md.body_words(e.position).into_iter().for_each(&mut word);
                                normal(e.normal, flag).into_iter().for_each(&mut word);
                            }
                        }
                        for i in 0..mm.skin.len() {
                            st(mm.uvs.get(i)).into_iter().for_each(&mut word);
                        }
                    }
                    _ => continue,
                }
                let kind = if mm.kind == Kind::Bone { "bone" } else { "skin" };
                writeln!(stdout, "{} {} {k} {kind}{w}", m.name, md.offset).unwrap();
            }
        }
    }
}
