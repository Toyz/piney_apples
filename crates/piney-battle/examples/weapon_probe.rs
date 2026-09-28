//! The enemies' weapon controller ([`piney_battle::weapon::WeaponCtrl`])
//! driven line by line for `tools/test_enemy_weapon_rs.py`, which runs the
//! game's own `ccEnemyWeaponCtrl` in eemu and compares: `new` (a controller
//! over `ccEnemyWpInfo` rows), `chunk` (a chunk's place and turn), `mat` (the
//! node's matrix), `note` and `ctrl` (with `AT = DISP ACT CNT ATK ATTR` and
//! the camera's rotation). Each line answers `{"w": [weapon...], "out":
//! [...]}`. ccTransPosFW2LW is (x + 0.5, y - 0.25, z, w), as the harness hooks it.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};

use piney_battle::geom::{self, M4, V4};
use piney_battle::prim::{RadOut, RadWorld, Radiate};
use piney_battle::weapon::{Pair, Weapon, WeaponAt, WeaponCtrl, WeaponOut, WeaponWorld, WpInfo};
use piney_battle::world::Note;

struct World {
    mats: BTreeMap<String, Option<M4>>,
    chunks: BTreeMap<String, (V4, V4)>,
    cam: V4,
}

impl RadWorld for World {
    fn fw2lw(&mut self, p: V4) -> V4 {
        [geom::add(p[0], 0x3f00_0000), geom::add(p[1], 0xbe80_0000), p[2], p[3]]
    }

    fn rand(&mut self) -> i32 {
        panic!("the weapons draw no random number")
    }

    fn camera_rot(&mut self) -> V4 {
        self.cam
    }
}

impl WeaponWorld for World {
    fn node_matrix(&mut self, name: &str, _ext: bool) -> Option<M4> {
        self.mats.get(name).copied().flatten()
    }

    fn chunk(&mut self, name: &str) -> Option<(V4, V4)> {
        self.chunks.get(name).copied()
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn num(s: &str) -> i64 {
    s.parse().unwrap_or_else(|_| i64::from_str_radix(s, 16).unwrap())
}

fn hexw(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap()
}

fn list<T: ToString>(v: impl IntoIterator<Item = T>) -> String {
    let parts: Vec<String> = v.into_iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn ix(v: Option<usize>) -> i64 {
    v.map_or(-1, |k| k as i64)
}

fn rad_vec(r: &Radiate) -> Vec<i64> {
    let mut v = vec![i64::from(u8::from(r.rad_flag) | u8::from(r.lgt_flag) << 1), i64::from(r.ty), i64::from(r.pnum)];
    v.extend(
        [r.center, r.scale, r.lscale, r.angle, r.bank, r.alpha, r.length, r.width, r.fzoom, r.dp_length, r.dp_bank]
            .map(i64::from),
    );
    v.push(i64::from(r.packet));
    v.extend(r.pos.map(i64::from));
    v.extend(r.rot.map(i64::from));
    v.extend([r.attr, r.life, r.act_num, r.act_cnt].map(i64::from));
    v.extend(r.param.map(i64::from));
    let l = &r.light;
    v.extend(l.matrix.iter().flatten().map(|&x| i64::from(x)));
    v.extend([i64::from(l.mat_calc), i64::from(l.rgb), i64::from(l.intensity)]);
    v.extend([l.far_start, l.far_end, l.far_end2].map(i64::from));
    for p in &r.parts {
        v.extend([i64::from(p.col0), i64::from(p.col1)]);
        for q in &p.vert {
            v.push(i64::from(q.color));
            v.extend(q.pos.map(i64::from));
        }
    }
    v
}

/// A weapon as the harness reads it from memory.
fn weapon_vec(w: &Weapon) -> String {
    let mut head = vec![
        i64::from(u8::from(w.ext) | u8::from(w.on) << 1 | u8::from(w.skill) << 2),
        i64::from(w.flags),
        i64::from(w.attr),
        i64::from(w.edges),
    ];
    head.extend(w.rate.map(i64::from));
    head.extend([ix(w.newest), ix(w.oldest), i64::from(w.count), i64::from(w.rad_life)]);
    head.extend(w.packet.map(i64::from));
    let n = usize::try_from(w.edges).unwrap_or(0).min(4);
    let cells: Vec<String> = w
        .cells
        .iter()
        .map(|c| {
            let mut v = vec![i64::from(c.between), i64::from(c.life), i64::from(c.alpha), ix(c.newer), ix(c.older)];
            for p in &c.vert[..n] {
                v.extend([i64::from(p.drawn), i64::from(p.color)]);
                v.extend(p.pos.map(i64::from));
            }
            list(v)
        })
        .collect();
    format!("[{},{},{}]", list(head), list(cells), list(rad_vec(&w.rad)))
}

fn pairs(p: &Option<Vec<Pair>>) -> String {
    match p {
        None => "null".into(),
        Some(v) => list(v.iter().map(|q| {
            let mut x: Vec<i64> = q.pos[0].iter().chain(&q.pos[1]).map(|&a| i64::from(a)).collect();
            x.extend([i64::from(q.color[0]), i64::from(q.color[1]), i64::from(q.cont)]);
            list(x)
        })),
    }
}

fn out_vec(o: &WeaponOut) -> String {
    match o {
        WeaponOut::Trail { weapon, polys, lines } => {
            format!("[\"trail\",{},{},{}]", weapon, pairs(polys), pairs(lines))
        }
        WeaponOut::Rad(RadOut::LightOn(l)) => {
            let mut v: Vec<i64> = l.matrix.iter().flatten().map(|&x| i64::from(x)).collect();
            v.extend([i64::from(l.mat_calc), i64::from(l.rgb), i64::from(l.intensity)]);
            format!("[\"light_on\",{}]", list(v))
        }
        WeaponOut::Rad(RadOut::LightOff) => "[\"light_off\"]".into(),
        WeaponOut::Rad(RadOut::Draw(_)) => "[\"rays\"]".into(),
    }
}

fn at(t: &[&str]) -> WeaponAt {
    WeaponAt {
        disp_sw: num(t[0]) != 0,
        act_num: num(t[1]) as i16,
        act_cnt: num(t[2]) as i16,
        atk_num: num(t[3]) as i16,
        attr: num(t[4]) as i32,
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let mut world = World { mats: BTreeMap::new(), chunks: BTreeMap::new(), cam: [0; 4] };
    let mut ctrl = WeaponCtrl::default();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let t: Vec<&str> = line.split_whitespace().collect();
        let mut out = Vec::new();
        match t.first().copied() {
            Some("new") => {
                let rows = t[1..]
                    .iter()
                    .map(|r| {
                        let v: Vec<&str> = r.split(':').collect();
                        let text = |h: &str| String::from_utf8(unhex(h)).unwrap();
                        WpInfo::from_bytes(&unhex(v[0]), text(v[1]), text(v[2]))
                    })
                    .collect();
                ctrl = WeaponCtrl::new(rows);
            }
            Some("mat") => {
                let name = String::from_utf8(unhex(t[1])).unwrap();
                let m =
                    (t[2] != "none").then(|| std::array::from_fn(|r| std::array::from_fn(|c| hexw(t[2 + 4 * r + c]))));
                world.mats.insert(name, m);
            }
            Some("chunk") => {
                let name = String::from_utf8(unhex(t[1])).unwrap();
                let v = |k: usize| std::array::from_fn(|i| hexw(t[k + i]));
                world.chunks.insert(name, (v(2), v(6)));
            }
            Some("note") => {
                let note = Note { event: num(t[1]) as u32, param: num(t[2]) as u32 };
                ctrl.note(&at(&t[3..8]), note);
            }
            Some("ctrl") => {
                world.cam = std::array::from_fn(|i| hexw(t[6 + i]));
                ctrl.ctrl(&at(&t[1..6]), &mut world, &mut out);
            }
            _ => continue,
        }
        let ws: Vec<String> = ctrl.weapons.iter().map(weapon_vec).collect();
        let os: Vec<String> = out.iter().map(out_vec).collect();
        writeln!(stdout, "{{\"w\":[{}],\"out\":[{}]}}", ws.join(","), os.join(",")).unwrap();
        stdout.flush().unwrap();
    }
}
