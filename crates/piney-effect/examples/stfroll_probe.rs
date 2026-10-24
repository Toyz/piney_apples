//! Answers `tools/test_stfroll_rs.py`: Quarantine's field staff roll
//! (`piney_effect::stfroll`) from `Init(0)` frame by frame, each frame one
//! JSON line (floats as bits) for the test to compare with the game's own
//! main run in eemu. A request is `roll FRAMES`. `genrand` is the test's
//! stand-in sequence (a 32-bit LCG from 0x12345678), and no particle
//! frame runs: every generator stays on the list, as the test's
//! `ccCheckParticleGenerator` says.

use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_effect::draw::Camera;
use piney_effect::stfroll::{Ask, Draw, StfRoll};
use piney_effect::{CharRef, Effects, Host, ONE, V4, VecRef};
use piney_world::Rand;

struct Probe {
    rand: Rand,
    word: u32,
}

impl Host for Probe {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn genrand(&mut self) -> u32 {
        self.word = self.word.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.word
    }
    fn player_pos(&self) -> V4 {
        [0, 0, 0, ONE]
    }
    fn camera(&self) -> Camera {
        Camera::default()
    }
    fn char_pos(&self, _c: CharRef) -> V4 {
        [0, 0, 0, ONE]
    }
    fn char_dirc(&self, _c: CharRef) -> V4 {
        [0; 4]
    }
    fn char_height(&self, _c: CharRef) -> u32 {
        0
    }
    fn char_width(&self, _c: CharRef) -> u32 {
        0
    }
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn ilist(v: &[i64]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn text(s: &str) -> String {
    format!("{s:?}")
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/quarantine/quarantine.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let volume = iso.volume().unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let t = piney_data::tables::stfroll::of(volume);
    let ending = SceneFile::read(&archive, piney_effect::stfroll::CCS).unwrap();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.first() != Some(&"roll") {
            continue;
        }
        let frames: usize = w[1].parse().unwrap();
        let mut fx = Effects::new(&archive, volume).unwrap();
        let mut host = Probe { rand: Rand(1), word: 0x1234_5678 };
        let mut asks = Vec::new();
        let mut s = StfRoll::new(t, &ending, &mut asks);
        // Generators by their serial number, in the order made.
        let mut sns: Vec<u32> = Vec::new();
        let gen_index = |sns: &[u32], sn: u32| sns.iter().position(|&x| x == sn).map_or(-1, |k| k as i64);
        for f in 0..=frames {
            let draws = if f == 0 {
                Vec::new()
            } else {
                let (ctrl, mut cx) = fx.split(&mut host);
                let (d, a) = s.main(ctrl, &mut cx, t, &ending);
                asks = a;
                d
            };
            let mut gens = Vec::new();
            let mut started: Vec<_> = fx.particles.gens.iter().collect();
            started.sort_by_key(|g| g.sn);
            for g in started {
                if !sns.contains(&g.sn) {
                    sns.push(g.sn);
                }
                let ff: Vec<u32> = g.ff.iter().map(|f| f.map_or(0, |f| f.va)).collect();
                let sync = match g.sync_pos {
                    Some(VecRef::Anchor(k)) => i64::from(k & 0xffff),
                    _ => 0,
                };
                gens.push(format!(
                    "[{}, {}, {}, {}, {}, {}, {}]",
                    gen_index(&sns, g.sn),
                    g.param.map_or(0, |p| p.va),
                    list(&ff),
                    sync,
                    u8::from(g.sync_pos_type),
                    list(&g.pos),
                    g.kill_flag
                ));
            }
            let comets: Vec<String> = s
                .comets
                .iter()
                .map(|c| {
                    let subs: Vec<i64> = c.subs.iter().map(|&g| gen_index(&sns, g)).collect();
                    format!(
                        "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
                        c.key & 0xffff,
                        list(&c.points.concat()),
                        c.t,
                        c.seg,
                        c.speed,
                        c.delay,
                        c.has_gen,
                        c.dead,
                        c.kill_mode,
                        c.age,
                        c.main_gen.map_or(-1, |g| gen_index(&sns, g)),
                        ilist(&subs),
                        list(&c.pos)
                    )
                })
                .collect();
            let lines: Vec<String> = s
                .lines
                .iter()
                .map(|l| format!("[{}, {}, {}]", u8::from(l.end), l.row, s.rows[usize::from(l.end)][l.row].y.to_bits()))
                .collect();
            let draws: Vec<String> = draws
                .iter()
                .map(|d| match d {
                    Draw::Line { k, x, y, text: tx } => format!("[\"line\", {k}, {x}, {y}, {}]", text(tx)),
                    Draw::Logo { tex, transp } => format!("[\"logo\", {}, {transp}]", text(tex)),
                    Draw::Anm { anim, time, matrix, transparency } => {
                        format!("[\"anm\", {}, {time}, {}, {transparency}]", text(anim.clip()), list(&matrix.concat()))
                    }
                })
                .collect();
            let asks_s: Vec<String> = asks
                .iter()
                .map(|a| match a {
                    Ask::Bgm(n) => format!("[\"bgm\", {n}]"),
                    Ask::BgmStop => "[\"bgmstop\"]".into(),
                    Ask::Camera { pos, view } => format!("[\"camera\", {}, {}]", list(pos), list(view)),
                })
                .collect();
            asks.clear();
            let anms: Vec<String> = s
                .anms
                .iter()
                .map(|a| format!("[{}, {}, {}, {}]", a.time, a.spd, u8::from(a.found), a.transparency))
                .collect();
            let last = s.last.map_or(0, |k| k & 0xffff);
            println!(
                "{{\"frame\": {f}, \"state\": [{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}], \"vecs\": [{}, {}, {}, {}, {}], \"anms\": [{}], \"lines\": [{}], \"comets\": [{}], \"gens\": [{}], \"draws\": [{}], \"asks\": [{}], \"genrand\": {}}}",
                s.state,
                s.phase,
                s.row,
                u8::from(s.at_end),
                u8::from(s.done),
                s.scroll,
                s.hold,
                s.book_wait,
                s.book_drawn,
                s.fade,
                s.alpha,
                s.count,
                s.spawn,
                s.spawned,
                s.spawn_max,
                s.ring_done,
                s.logo_wait,
                s.tail,
                s.spacing.0,
                s.spacing.1,
                last,
                list(&s.pos),
                list(&s.rot),
                list(&s.view),
                list(&s.eye),
                list(&s.last_pos),
                anms.join(", "),
                lines.join(", "),
                comets.join(", "),
                gens.join(", "),
                draws.join(", "),
                asks_s.join(", "),
                host.word
            );
        }
    }
}
