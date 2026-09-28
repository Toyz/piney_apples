//! Prints what `piney_data::field` makes, one JSON line per request, for
//! `tools/test_field_rs.py` to compare with `tools/field.py` and
//! `tools/eemu.py` (`field_snapshot [DATA.BIN] < requests`). The requests are
//! `field`, `render` (what the game draws of a field under a light: vertex
//! normals and colours, the ground tiles and covers, `WORLD::GetHeight` at 200
//! fixed points), `light` (a background file's frame-0 light), `objcol`
//! (`CalcObjectVertexColor`) and `fpu` (one FPU operation), with the arguments
//! the test sends. Floats are printed as their bit patterns, in decimal.

use std::fmt::Write as _;
use std::io::{BufRead, Write as _};

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::field::{self, INF, Light, Params, ee};

/// The points `render` asks WORLD::GetHeight for; tools/test_field_rs.py
/// makes the same.
fn height_points() -> Vec<[f32; 2]> {
    (0..200u32)
        .map(|k| {
            let x = ((k * 7919) % 48000) as f32 + 0.25 * (k % 4) as f32;
            let y = ((k * 104_729) % 48000) as f32 + 0.5 * (k % 3) as f32;
            [x, y]
        })
        .collect()
}

fn main() {
    let archive = std::env::args().nth(1).map(|p| Archive::new(std::fs::read(p).expect("DATA.BIN")).expect("archive"));
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| -> i64 { w[i].parse().unwrap() };
        let json = match w.first().copied() {
            Some("field") => snapshot(&Params {
                seed: n(1) as u32,
                field_type: n(2) as u32,
                weather: n(3) as u32,
                ground: n(4) as u32,
                object: n(5) as u32,
                event: n(6) as i32,
                protect: n(7) != 0,
                skip_init: n(8) != 0,
            }),
            Some("render") => {
                let p = Params {
                    seed: n(1) as u32,
                    field_type: n(2) as u32,
                    weather: n(3) as u32,
                    ground: n(4) as u32,
                    object: n(5) as u32,
                    event: n(6) as i32,
                    protect: n(7) != 0,
                    skip_init: n(8) != 0,
                };
                let light = Light {
                    ambient: n(9) as u32,
                    rotation: [n(10) as u32, n(11) as u32, n(12) as u32],
                    colour: n(13) as u32,
                };
                render(&p, &light)
            }
            Some("light") => {
                let arc = archive.as_ref().expect("light needs DATA.BIN");
                match arc.inflate_named(w[1]).and_then(Ccs::parse).and_then(|c| Light::from_anime(&c, w[2], w[3])) {
                    Ok(l) => format!(
                        "{{\"ambient\": {}, \"rotation\": {}, \"colour\": {}}}",
                        l.ambient,
                        nums(&l.rotation),
                        l.colour
                    ),
                    Err(e) => format!("{{\"error\": \"{e}\"}}"),
                }
            }
            Some("objcol") => {
                let light = Light {
                    ambient: n(1) as u32,
                    rotation: [n(2) as u32, n(3) as u32, n(4) as u32],
                    colour: n(5) as u32,
                };
                let words: Vec<u32> = w[6..].iter().map(|v| v.parse().unwrap()).collect();
                let normals: Vec<[u8; 4]> = words.iter().step_by(2).map(|v| v.to_le_bytes()).collect();
                let colours: Vec<[u8; 4]> = words.iter().skip(1).step_by(2).map(|v| v.to_le_bytes()).collect();
                let lit = field::object_colours(&normals, &colours, &light);
                list(lit, |c| u32::from_le_bytes(c).to_string())
            }
            Some("fpu") => fpu(w[1], n(2) as u32, w.get(3).map_or(0, |_| n(3) as u32)),
            _ => panic!("bad request: {line}"),
        };
        writeln!(out, "{json}").unwrap();
    }
}

fn fpu(op: &str, a: u32, b: u32) -> String {
    let v: i64 = match op {
        "add" => ee::add(a, b).into(),
        "sub" => ee::sub(a, b).into(),
        "mul" => ee::mul(a, b).into(),
        "div" => ee::div(a, b).into(),
        "cmp" => ee::cmp(a, b) as i64,
        "from_int" => ee::from_int(a as i32).into(),
        "to_int" => (ee::to_int(a) as u32).into(),
        "sqrt" => ee::sqrt(a).into(),
        "sqrtf" => ee::sqrtf(a).into(),
        _ => panic!("bad op {op}"),
    };
    v.to_string()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

fn list<T>(items: impl IntoIterator<Item = T>, f: impl Fn(T) -> String) -> String {
    format!("[{}]", items.into_iter().map(f).collect::<Vec<_>>().join(", "))
}

fn nums(v: &[u32]) -> String {
    list(v, |x| x.to_string())
}

fn snapshot(p: &Params) -> String {
    let fl = match field::generate(&INF, p) {
        Ok(f) => f,
        Err(e) => return format!("{{\"error\": \"{e}\"}}"),
    };
    let mut map = String::with_capacity(fl.map.len() * 8);
    for v in &fl.map {
        write!(map, "{v:08x}").unwrap();
    }
    let hills = list(&fl.hills, |h| nums(&[h.x, h.y, h.size]));
    let objects = list(&fl.objects, |o| {
        format!(
            "[{}, {}, {}, {}, {}, {}, \"{}\", {}, {}, {}, {}, {}, {}]",
            o.kind.number(),
            o.index,
            o.x,
            o.y,
            o.w,
            o.h,
            o.info.clump,
            o.pos[0],
            o.pos[1],
            o.cell[0],
            o.cell[1],
            o.level,
            o.z
        )
    });
    let covers = list(&fl.covers, |c| format!("[{}, {}, {}, \"{}\"]", c.x, c.y, c.quadrant, c.mesh));
    let opt = |v: Option<&[u32]>| v.map_or("null".to_string(), nums);
    format!(
        "{{\"init_draws\": {}, \"map\": \"{map}\", \"check\": \"{}\", \"check3\": \"{}\", \"mnt\": \"{}\", \
         \"hills\": {hills}, \"objects\": {objects}, \"covers\": {covers}, \"entrance\": {}, \
         \"dungeon_pos\": {}, \"start\": {}, \"start_pos\": {}, \"seed\": {}, \"randcnt\": {}}}",
        fl.init_draws,
        hex(&fl.check),
        hex(&fl.check3),
        hex(&fl.mnt),
        opt(fl.entrance.as_ref().map(|e| &e[..])),
        opt(fl.dungeon_pos.as_ref().map(|d| &d[..])),
        nums(&fl.start),
        nums(&fl.start_pos),
        fl.rng.seed,
        fl.rng.count
    )
}

fn words_hex(v: impl IntoIterator<Item = u32>) -> String {
    let mut s = String::new();
    for x in v {
        write!(s, "{x:08x}").unwrap();
    }
    s
}

fn render(p: &Params, light: &Light) -> String {
    let fl = match field::generate(&INF, p) {
        Ok(f) => f,
        Err(e) => return format!("{{\"error\": \"{e}\"}}"),
    };
    let normals = fl.vertex_normals();
    let colours = fl.vertex_colours(light);
    let mut tiles = Vec::with_capacity(1600);
    for x in 0..40 {
        for y in 0..40 {
            let t = fl.tile(&INF, x, y, 600f32.to_bits(), Some(&colours));
            tiles.push(format!(
                "[{}, {}, {}, {}]",
                t.visible,
                nums(&t.pos),
                list(&t.z, |(_, z)| z.to_string()),
                list(&t.colours, |(_, c)| format!("[{}, {}, {}]", c[0], c[1], c[2]))
            ));
        }
    }
    let covers = list(&fl.covers, |c| {
        let m = fl.cover_mesh(&INF, c, 300f32.to_bits(), Some(&colours));
        format!(
            "[{}, {}, {}]",
            nums(&m.pos),
            list(&m.z, |(_, z)| z.to_string()),
            list(&m.colours, |(_, c)| format!("[{}, {}, {}]", c[0], c[1], c[2]))
        )
    });
    let heights = list(height_points(), |[x, y]| fl.get_height(x.to_bits(), y.to_bits()).to_string());
    format!(
        "{{\"dirc\": {}, \"direction\": {}, \"ambient\": {}, \"colour\": {}, \"normals\": \"{}\", \
         \"colours\": \"{}\", \"tiles\": [{}], \"covers\": {covers}, \"heights\": {heights}}}",
        nums(&light.dirc()),
        nums(&light.direction()),
        nums(&light.ambient_rgb()),
        nums(&light.colour_rgb()),
        words_hex(normals.iter().flatten().copied()),
        words_hex(colours.iter().flatten().copied()),
        tiles.join(", ")
    )
}
