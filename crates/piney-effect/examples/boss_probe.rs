//! Answers `tools/test_boss_effect_rs.py`: makes the boss's effects
//! (`piney_effect::boss`) on the requests it is sent, runs the manager's pass
//! frame by frame, and prints each answer as one JSON line, so the test can
//! run the same through the game's own `ccBossEff*Create` and `Draw` in eemu
//! (`boss_probe ISO < requests`). Numbers are hex; floats travel as their bit
//! patterns. The requests (`reset`, `player`, `camera`, `field`, the Creates
//! `wave`, `square`, `force`, `ring`, `ice`, `dead`, Fidchell's `meteo`,
//! `storm`, `tower`, `frame`, and the lattice's `lattice`, `lnext`, `lpos`,
//! `ldisp`, `lclear`, `lcnt`, `ltype`) are the test's. `genrand` is the
//! test's stand-in sequence ([`next_genrand`]).

use std::cell::Cell;
use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_effect::boss::{BossEff, Make};
use piney_effect::draw::{Camera, DrawRec};
use piney_effect::particle::Generator;
use piney_effect::{CharRef, Effects, Event, Host, ONE, V4, VecRef, ee};
use piney_world::Rand;
use piney_world::lattice::Lattice;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

struct Probe {
    rand: Rand,
    /// `genrand` drawn so far, and the last word.
    genrand: Cell<u32>,
    word: Cell<u32>,
    player: V4,
    eye: V4,
    view: V4,
    /// `game` +0x24.
    field: i32,
}

/// The test's stand-in for `genrand()`: a 32-bit LCG from 0x12345678.
fn next_genrand(w: u32) -> u32 {
    w.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
}

impl Host for Probe {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn genrand(&mut self) -> u32 {
        let n = self.genrand.get();
        self.genrand.set(n + 1);
        let w = next_genrand(self.word.get());
        self.word.set(w);
        w
    }
    fn game_field(&self) -> i32 {
        self.field
    }
    fn player_pos(&self) -> V4 {
        self.player
    }
    fn camera(&self) -> Camera {
        Camera { eye: self.eye, cam_pos: self.eye, cam_view: self.view, ..Camera::default() }
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
    /// `cameraGetRot(out, 0)`: x the pitch, z the heading of camera 0.
    fn camera_rot(&self, out: V4) -> V4 {
        let mut d = ee::vsub(self.view, self.eye);
        d[2] = 0;
        let across = ee::sqrtf(ee::dot(d, d));
        let mut r = out;
        r[0] = ee::atan2f(ee::sub(self.eye[2], self.view[2]), across);
        r[2] = ee::atan2f(ee::sub(self.view[0], self.eye[0]), ee::mul(0xbf80_0000, ee::sub(self.view[1], self.eye[1])));
        r
    }
}

fn lattice_state(l: &Lattice) -> String {
    let rows: Vec<String> = l.rows.iter().map(|r| list(&r[..l.cols].concat())).collect();
    let life: Vec<String> = l.life.iter().map(|x| x.to_string()).collect();
    format!(
        "{{\"head\": {}, \"tail\": {}, \"life\": [{}], \"rows\": [{}], \"made\": {}, \"full\": {}, \"first\": {}, \"clear\": {}, \"ty\": {}}}",
        l.head,
        l.tail,
        life.join(", "),
        rows.join(", "),
        l.made,
        u8::from(l.full),
        u8::from(l.first),
        u8::from(l.clear),
        l.ty
    )
}

fn vref(r: Option<VecRef>) -> String {
    match r {
        Some(VecRef::Anchor(k)) => format!("[\"anchor\", {}, {}]", k >> 8, k & 0xff),
        Some(VecRef::EffectPos(k)) => format!("[\"effpos\", {k}]"),
        None => "null".into(),
        Some(other) => format!("\"{other:?}\""),
    }
}

fn generator(g: &Generator) -> String {
    let ff: Vec<u32> = g.ff.iter().map(|f| f.map_or(0, |f| f.va)).collect();
    format!(
        "[{}, {}, {}, {}, {}, {}, {}, {}]",
        g.param.map_or(0, |p| p.va),
        list(&ff),
        u8::from(g.dist_sw),
        u8::from(g.sync_pos_type),
        g.p_tex_mod,
        list(&g.pos),
        list(&g.offset),
        vref(g.sync_pos)
    )
}

fn draw(fx: &Effects, d: &DrawRec) -> String {
    match d {
        DrawRec::Clump { obj, matrix, alpha, layer, .. } => {
            format!("[\"clump\", \"{}\", {}, {}, {}]", fx.assets.name(*obj), list(&matrix.concat()), alpha, layer)
        }
        DrawRec::Anm { obj, play, matrix, alpha, layer } => format!(
            "[\"anm\", \"{}\", {}, {}, {}, {}]",
            fx.assets.name(*obj),
            play.time,
            list(&matrix.concat()),
            alpha,
            layer
        ),
        DrawRec::Eff { eff, pat, layer } => format!(
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, \"{}\", {}]",
            eff.chunk,
            pat,
            list(&eff.pos),
            eff.scale_x,
            eff.scale_y,
            eff.rotate,
            eff.color,
            eff.transparency,
            eff.clut.map_or(String::new(), |c| fx.assets.name(c).to_string()),
            layer
        ),
    }
}

fn event(e: &Event) -> String {
    match e {
        Event::Sound3d { se, pos } => format!("[\"se3d\", {se}, {}]", list(pos)),
        Event::Sound { se } => format!("[\"se\", {se}]"),
        Event::Sound3dNote { se, pos, note } => format!("[\"se3dnote\", {se}, {}, {note}]", list(pos)),
        Event::SoundNote { se, note } => format!("[\"senote\", {se}, {note}]"),
        Event::Flash { time, color, rect } => format!("[\"flash\", {time}, {color}, {}]", list(rect)),
        other => format!("[\"?\", \"{other:?}\"]"),
    }
}

/// The events and the generators started since the last answer.
fn news(fx: &mut Effects) -> (String, String) {
    let events: Vec<String> = fx.take_events().iter().map(event).collect();
    let gens: Vec<String> = std::mem::take(&mut fx.particles.started).iter().map(generator).collect();
    (events.join(", "), gens.join(", "))
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let mut fx = Effects::new(&archive, iso.volume().unwrap()).unwrap();
    fx.assets.add_file(&archive, "xeffect").unwrap();
    let fresh = |seed: u64| Probe {
        rand: Rand(seed),
        genrand: Cell::new(0),
        word: Cell::new(0x1234_5678),
        player: [0, 0, 0, ONE],
        eye: [0, 0, 0, ONE],
        view: [0, 0, 0, ONE],
        field: 0,
    };
    let mut host = fresh(0);
    let mut lattice = Lattice::new(2, 7);
    // Every generator started, in order: its serial number.
    let mut started: Vec<u32> = Vec::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let n = |i: usize| hex(w[i]);
        let v4 = |i: usize| [hex(w[i]), hex(w[i + 1]), hex(w[i + 2]), hex(w[i + 3])];
        let make = match w[0] {
            "lattice" => {
                lattice = Lattice::new(n(1) as usize, n(2) as i32);
                println!("{{\"state\": {}}}", lattice_state(&lattice));
                continue;
            }
            // ClearArmsEffect's +0x72c = 1, then ClearCnt.
            "lclear" => {
                lattice.clear = true;
                println!("{{\"state\": {}}}", lattice_state(&lattice));
                continue;
            }
            "lcnt" => {
                lattice.clear_cnt();
                println!("{{\"state\": {}}}", lattice_state(&lattice));
                continue;
            }
            // _SetArmsEffectColor's +4.
            "ltype" => {
                lattice.ty = n(1) as i32;
                println!("{{\"state\": {}}}", lattice_state(&lattice));
                continue;
            }
            "lnext" => {
                lattice.next_vertex();
                println!("{{\"state\": {}}}", lattice_state(&lattice));
                continue;
            }
            "lpos" => {
                lattice.set_pos(v4(1), n(5) as usize);
                println!("{{\"state\": {}}}", lattice_state(&lattice));
                continue;
            }
            "ldisp" => {
                let (strip, key) = lattice.disp();
                let verts: Vec<String> = strip
                    .iter()
                    .map(|v| {
                        format!("[{}, {}, {}, {}, {}]", v.rgba[0], v.rgba[1], v.rgba[2], v.rgba[3], u8::from(v.restart))
                    })
                    .collect();
                println!(
                    "{{\"strip\": [{}], \"key\": {key}, \"state\": {}}}",
                    verts.join(", "),
                    lattice_state(&lattice)
                );
                continue;
            }
            "reset" => {
                fx.boss = Default::default();
                fx.ctrl = piney_effect::effect::EffectCtrl::new();
                fx.ctrl.town = false;
                fx.particles = Default::default();
                fx.particles.install_statics(&fx.assets);
                // The test builds no ccParticleCtrl: a particle is never
                // free (effSmoke's puff is made on neither side).
                fx.particles.slots.clear();
                let _ = fx.take_events();
                host = fresh(u64::from(n(1)));
                started.clear();
                println!("{{}}");
                continue;
            }
            "player" => {
                host.player = [n(1), n(2), n(3), ONE];
                println!("{{}}");
                continue;
            }
            "field" => {
                host.field = n(1) as i32;
                println!("{{}}");
                continue;
            }
            "camera" => {
                host.eye = [n(1), n(2), n(3), ONE];
                host.view = [n(4), n(5), n(6), ONE];
                println!("{{}}");
                continue;
            }
            "frame" => {
                fx.boss_step(&mut host);
                let draws: Vec<String> = fx.boss_draws().iter().map(|d| draw(&fx, d)).collect();
                started.extend(fx.particles.started.iter().map(|g| g.sn));
                let (events, gens) = news(&mut fx);
                let kills: Vec<u32> =
                    started.iter().map(|&sn| fx.particles.get(sn).map_or(0, |g| g.kill_flag as u32)).collect();
                let mut lights = Vec::new();
                let mut enabled = Vec::new();
                for (k, s) in fx.boss.slots.iter().enumerate() {
                    let Some(s) = s else { continue };
                    enabled.push(format!("[{k}, {}]", u8::from(s.enabled)));
                    // sceVu0TransMatrix of the unit matrix: 0 + x.
                    let at = |p: V4| list(&[ee::add(0, p[0]), ee::add(0, p[1]), ee::add(0, p[2])]);
                    if let BossEff::Light(l) = &s.eff {
                        lights.push(format!("[{}, {}, {}]", u8::from(l.in_group), at(l.light_pos), list(&l.colour)));
                    }
                    // A meteor swarm's light: in the group, its intensity,
                    // fall-off's start and end.
                    if let BossEff::Spell(x) = &s.eff
                        && let Some((p, c, far)) = x.light()
                    {
                        lights.push(format!("[\"meteo\", 1, {}, {}, {ONE}, 0, {far}]", at(p), list(&c)));
                    }
                }
                let slots: Vec<String> = fx
                    .ctrl
                    .effects
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.status != 0)
                    .map(|(i, e)| {
                        format!(
                            "[{i}, {}, {}, {}, {}, {}, {}, {}, {}]",
                            e.id,
                            e.life_time,
                            list(&e.pos),
                            list(&e.scale),
                            list(&e.speed),
                            e.velocity,
                            list(&e.rot_speed.map(u32::from)),
                            e.sn
                        )
                    })
                    .collect();
                println!(
                    "{{\"draws\": [{}], \"events\": [{events}], \"gens\": [{gens}], \"kills\": {}, \"lights\": [{}], \"enabled\": [{}], \"slots\": [{}], \"rand\": {}, \"ccrand\": {}}}",
                    draws.join(", "),
                    list(&kills),
                    lights.join(", "),
                    enabled.join(", "),
                    slots.join(", "),
                    host.rand.0,
                    host.genrand.get()
                );
                continue;
            }
            "wave" => Make::WaveShock { pos: v4(1), dirc: v4(5), scale: n(9) },
            "square" => Make::MagicSquare { pos: v4(1), n: n(5) as i32 },
            "force" => Make::ForceGenerator {
                p: v4(1),
                rot: v4(5),
                speed: n(9),
                r0: n(10),
                r1: n(11),
                num: n(12) as i32,
                life: n(13) as i32,
                clt: n(14) as i32,
            },
            "ring" => Make::AutoSamonRing { pos: v4(1), rot: v4(5), param: v4(9), n: n(13) as i32 },
            "ice" => Make::IceBreak { pos: v4(1), scale: n(5) },
            "dead" => Make::Dead { pos: v4(1) },
            "meteo" => Make::MeteoSworm { sp: v4(1), ep: v4(5), n: n(9) as i32, radius: n(10), v: n(11) },
            "storm" => Make::ThunderStorm { pos: v4(1), radius: n(5), n: n(6) as i32 },
            "tower" => Make::RockTower { pos: v4(1), n: n(5) as i32 },
            other => panic!("unknown request {other}"),
        };
        let id = fx.boss_create(&mut host, make);
        started.extend(fx.particles.started.iter().map(|g| g.sn));
        let (events, gens) = news(&mut fx);
        println!("{{\"id\": {id}, \"events\": [{events}], \"gens\": [{gens}], \"ccrand\": {}}}", host.genrand.get());
    }
}
