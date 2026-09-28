//! Answers `tools/test_morph_rs.py`: one request a line, the rigid positions
//! `Model::morph` gives, one line of numbers back. A request is `BASE_SCALE N
//! K`, N base positions `x y z`, then K times `SCALE WEIGHT` and N positions;
//! the answer is N positions. Scales and weights are f32 bit patterns in hex.

use std::io::{BufRead, Write};

use piney_data::model::{Kind, Mmat, Model};

fn model(scale: f32, positions: Vec<[i16; 3]>) -> Model {
    let mmat = Mmat {
        kind: Kind::Rigid,
        name: None,
        material: None,
        slot: None,
        positions,
        normals: Vec::new(),
        flags: Vec::new(),
        colours: Vec::new(),
        uvs: Vec::new(),
        skin: Vec::new(),
        triangles: Vec::new(),
    };
    Model { offset: 0, end: 0, object: 0, scale, mtype: 0, flag: 0, zoffs: 0, mmats: vec![mmat] }
}

struct Cursor<'a>(std::str::SplitWhitespace<'a>);

impl Cursor<'_> {
    fn bits(&mut self) -> f32 {
        f32::from_bits(u32::from_str_radix(self.0.next().unwrap(), 16).unwrap())
    }
    fn int(&mut self) -> i64 {
        self.0.next().unwrap().parse().unwrap()
    }
    fn positions(&mut self, n: usize) -> Vec<[i16; 3]> {
        (0..n).map(|_| [0; 3].map(|_| self.int() as i16)).collect()
    }
}

fn main() {
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut c = Cursor(line.split_whitespace());
        let base_scale = c.bits();
        let (n, k) = (c.int() as usize, c.int() as usize);
        let base = model(base_scale, c.positions(n));
        let targets: Vec<(Model, f32)> = (0..k)
            .map(|_| {
                let (scale, weight) = (c.bits(), c.bits());
                (model(scale, c.positions(n)), weight)
            })
            .collect();
        let targets: Vec<(&Model, f32)> = targets.iter().map(|(m, w)| (m, *w)).collect();
        let m = base.morph(0, &targets).unwrap();
        let s: Vec<String> = m.iter().flatten().map(|v| v.to_string()).collect();
        writeln!(out, "{}", s.join(" ")).unwrap();
    }
}
