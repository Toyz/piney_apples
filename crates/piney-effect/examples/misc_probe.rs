//! Answers `tools/test_effect_misc_rs.py`: runs the port's level up, dying
//! blow, drains and magic portal on the requests it is sent and prints
//! each answer as one JSON line, for the test to compare with the game's
//! own code in eemu.
//!
//! ```text
//! cargo build -p piney-effect --example misc_probe
//! misc_probe ISO < requests
//! ```
//!
//! Numbers are hex; floats travel as their bit patterns. Requests, one a
//! line (those of `effect_probe` first):
//! - `reset TOWN SEED`, `player X Y Z`, `camera E.. P.. V..`, `char ID X Y
//!   Z H W [DX DY DZ]`, `frame`: as `effect_probe`'s.
//! - `bounds X0 Y0 X1 Y1`: `WORLD_MAN`'s map bounds.
//! - `listed ID 0|1`: `ccCheckTarget(ID)`; `listed player 0|1` for `plw`.
//! - `dead ID N`: `condition.dead`; `size ID N`: `ccCheckObjectSize`.
//! - `levelup ID`, `dying ID`, `drainctrl AP BP TYPE TIME NUM`, `drain AP BP
//!   TYPE NUM`, `protect ID SW SIZE`, `afterdrain ID SIZE`: the starters;
//!   each answers its slot and events.
//! - `set SLOT KEY V..`: a slot's `pos`, `rot`, `speed`, `temp` (4 each),
//!   `cnt` or `age`, to force a case.
//! - `mt N`: `N` draws of `ccRand` thrown away.
//! - `circle X Y Z DX DY DZ`: a new magic portal (`ccMagicCircle`).
//! - `cframe FREEZE DISP PLDIST SETT LISTED ENTROOT`: one `ccMagicCircle::
//!   main` with what `ccEntryObj::routine` left; answers the portal's whole
//!   state, its draws and events.
//! - `acosf X`, `sin HI LO`, `cos HI LO`: the maths (double as two words).

use std::collections::HashMap;
use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_effect::draw::{Camera, DrawRec};
use piney_effect::eff::Eff;
use piney_effect::effect::{Effect, Obj};
use piney_effect::portal::{CircleEvent, CircleInput, MagicCircle};
use piney_effect::{CharRef, Effects, Event, Host, ONE, V4, VecRef, dmath, drain, space};
use piney_world::Rand;
use piney_world::mt::Mt;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

struct Probe {
    rand: Rand,
    player: V4,
    bounds: [u32; 4],
    camera: Camera,
    chars: HashMap<CharRef, (V4, V4, u32, u32)>,
    listed: HashMap<CharRef, bool>,
    dead: HashMap<CharRef, i16>,
    size: HashMap<CharRef, i32>,
    mt: Mt,
}

impl Probe {
    fn new(seed: u64) -> Probe {
        Probe {
            rand: Rand(seed),
            player: [0, 0, 0, ONE],
            bounds: space::TOWN_BOUNDS,
            camera: Camera::default(),
            chars: HashMap::new(),
            listed: HashMap::new(),
            dead: HashMap::new(),
            size: HashMap::new(),
            mt: Mt::default(),
        }
    }
}

impl Host for Probe {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn player_pos(&self) -> V4 {
        self.player
    }
    fn bounds(&self) -> [u32; 4] {
        self.bounds
    }
    fn camera(&self) -> Camera {
        self.camera
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        self.chars.get(&c).map_or([0, 0, 0, ONE], |c| c.0)
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        self.chars.get(&c).map_or([0; 4], |c| c.1)
    }
    fn char_height(&self, c: CharRef) -> u32 {
        self.chars.get(&c).map_or(0, |c| c.2)
    }
    fn char_width(&self, c: CharRef) -> u32 {
        self.chars.get(&c).map_or(0, |c| c.3)
    }
    fn check_target(&self, c: CharRef) -> bool {
        self.chars.contains_key(&c) && self.listed.get(&c).copied().unwrap_or(true)
    }
    fn char_dead(&self, c: CharRef) -> i16 {
        self.dead.get(&c).copied().unwrap_or(0)
    }
    fn object_size(&self, c: CharRef) -> i32 {
        self.size.get(&c).copied().unwrap_or(0)
    }
    fn genrand(&mut self) -> u32 {
        self.mt.genrand()
    }
}

fn who(r: Option<CharRef>) -> i64 {
    r.map_or(-1, i64::from)
}

fn vref(r: Option<VecRef>) -> String {
    match r {
        None => "null".into(),
        Some(VecRef::CharPos(c)) => format!("[\"pos\", {c}]"),
        Some(VecRef::CharDirc(c)) => format!("[\"dirc\", {c}]"),
        Some(VecRef::EffectPos(k)) => format!("[\"effpos\", {k}]"),
        Some(VecRef::EffectRot(k)) => format!("[\"effrot\", {k}]"),
        Some(VecRef::EffectPosT(k)) => format!("[\"effposT\", {k}]"),
        Some(VecRef::CharAt(c, off)) => format!("[\"at\", {c}, {off}]"),
        Some(VecRef::Anchor(k)) => format!("[\"anchor\", {k}]"),
    }
}

fn eff_state(fx: &Effects, f: &Eff) -> String {
    let clut = f.clut.map_or("null".to_string(), |o| format!("\"{}\"", fx.assets.name(o)));
    format!(
        "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, {}]",
        f.scale_x,
        f.scale_y,
        f.rotate,
        f.color,
        f.transparency,
        f.flag,
        list(&f.pos),
        clut,
        f.test
    )
}

fn slot(fx: &Effects, i: usize, e: &Effect) -> String {
    let bits = u32::from(e.dist_sw)
        | u32::from(e.disp_sw) << 1
        | u32::from(e.pause_sw) << 2
        | u32::from(e.end_flag) << 3
        | u32::from(e.zyx_flag) << 4
        | (u32::from(e.level as u8) & 15) << 5;
    let obj = match &e.obj {
        Obj::None => "null".into(),
        Obj::Clump(o) => format!("[\"clump\", \"{}\"]", fx.assets.name(*o)),
        Obj::Anm { obj, play, .. } => format!("[\"anm\", \"{}\", {}]", fx.assets.name(*obj), play.time),
        Obj::Eff(f) => eff_state(fx, f),
    };
    format!(
        concat!(
            "{{\"i\": {}, \"id\": {}, \"status\": {}, \"life\": {}, \"age\": {}, \"cnt\": {}, \"pat\": {}, ",
            "\"bits\": {}, \"param\": {}, \"flags\": {}, \"pos\": {}, \"offset\": {}, \"rot\": {}, ",
            "\"speed\": {}, \"scale\": {}, \"posT\": {}, \"rotSpeed\": {}, \"velocity\": {}, ",
            "\"transparency\": {}, \"target\": {}, \"posPtr\": {}, \"rotPtr\": {}, \"link\": {}, ",
            "\"temp\": {}, \"sn\": {}, \"layer\": {}, \"obj\": {}}}"
        ),
        i,
        e.id,
        e.status,
        e.life_time,
        e.age,
        e.cnt,
        e.tex_anm_pat,
        bits,
        e.param,
        e.flags,
        list(&e.pos),
        list(&e.offset),
        list(&e.rot),
        list(&e.speed),
        list(&e.scale),
        list(&e.pos_t),
        list(&e.rot_speed.map(u32::from)),
        e.velocity,
        e.transparency,
        who(e.target),
        vref(e.pos_ptr),
        vref(e.rot_ptr),
        e.link.map_or(-1, |l| l as i64),
        list(&e.temp),
        e.sn,
        e.layer.map_or("null".to_string(), |l| l.to_string()),
        obj
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
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, {}]",
            eff.chunk,
            pat,
            list(&eff.pos),
            eff.scale_x,
            eff.scale_y,
            eff.rotate,
            eff.color,
            eff.transparency,
            layer
        ),
    }
}

fn event(e: &Event) -> String {
    match e {
        Event::Sound3d { se, pos } => format!("[\"se3d\", {se}, {}]", list(pos)),
        Event::CameraShake(a) => format!("[\"shake\", {}, {}, {}, {}]", a[0], a[1], a[2], a[3]),
        other => format!("[\"?\", \"{other:?}\"]"),
    }
}

fn gens(fx: &mut Effects) -> String {
    let g: Vec<String> = std::mem::take(&mut fx.particles.started)
        .iter()
        .map(|g| {
            let sw = g.sync_sw.map_or("null".to_string(), |s| match s {
                piney_effect::IntRef::EffectTemp(s, k) => format!("[{s}, {k}]"),
                other => format!("\"{other:?}\""),
            });
            format!("[{}, {}, {}, {}, {}]", g.row, vref(g.sync_pos), u8::from(g.sync_pos_type), list(&g.offset), sw)
        })
        .collect();
    format!("[{}]", g.join(", "))
}

fn started(fx: &mut Effects, i: Option<usize>) {
    let events: Vec<String> = fx.take_events().iter().map(event).collect();
    let g = gens(fx);
    println!("{{\"slot\": {}, \"events\": [{}], \"gens\": {}}}", i.map_or(-1, |i| i as i64), events.join(", "), g);
}

fn circle_state(c: &MagicCircle) -> String {
    let parts: Vec<String> = c
        .parts
        .iter()
        .map(|p| {
            let eff = match (&p.eff, c.part_flag) {
                (Some(f), true) => format!(
                    "[{}, {}, {}, {}, {}, {}]",
                    list(&f.pos),
                    f.scale_x,
                    f.scale_y,
                    f.rotate,
                    f.color,
                    f.transparency
                ),
                _ => "null".into(),
            };
            format!(
                "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
                p.status,
                u8::from(p.anm_flag),
                list(&p.mat.concat()),
                p.speed,
                p.rad_cnt,
                p.mc_act_cnt,
                p.eff_anm_pat,
                p.cnt,
                p.life,
                p.transparency,
                p.rnd,
                eff
            )
        })
        .collect();
    format!(
        concat!(
            "{{\"act\": {}, \"actCnt\": {}, \"anmFlag\": {}, \"dest\": {}, \"disp\": {}, \"partFlag\": {}, ",
            "\"partTop\": {}, \"pos\": {}, \"posP\": {}, \"dirc\": {}, \"time\": {}, \"parts\": [{}]}}"
        ),
        c.act,
        c.act_cnt,
        u8::from(c.anm_flag),
        u8::from(c.dest_flag),
        u8::from(c.disp_sw),
        u8::from(c.part_flag),
        c.part_top,
        list(&c.pos),
        list(&c.pos_p),
        list(&c.dirc),
        c.play.time,
        parts.join(", ")
    )
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let mut fx = Effects::new(&archive, iso.volume().unwrap()).unwrap();
    fx.assets.add_file(&archive, piney_effect::portal::FILE).unwrap();
    let mut host = Probe::new(0);
    let mut player_listed = true;
    let mut circle: Option<MagicCircle> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let n = |i: usize| hex(w[i]);
        let v3 = |i: usize| [hex(w[i]), hex(w[i + 1]), hex(w[i + 2]), ONE];
        match w[0] {
            "reset" => {
                fx.ctrl = piney_effect::effect::EffectCtrl::new();
                fx.ctrl.town = n(1) != 0;
                fx.particles = Default::default();
                fx.take_events();
                host = Probe::new(u64::from(n(2)));
                player_listed = true;
                circle = None;
                println!("{{}}");
            }
            "player" => {
                host.player = v3(1);
                println!("{{}}");
            }
            "bounds" => {
                host.bounds = [n(1), n(2), n(3), n(4)];
                println!("{{}}");
            }
            "camera" => {
                host.camera.eye = v3(1);
                host.camera.cam_pos = v3(4);
                host.camera.cam_view = v3(7);
                println!("{{}}");
            }
            "char" => {
                let dirc = if w.len() > 9 { [n(7), n(8), n(9), 0] } else { [0; 4] };
                host.chars.insert(n(1), (v3(2), dirc, n(5), n(6)));
                println!("{{}}");
            }
            "listed" => {
                if w[1] == "player" {
                    player_listed = n(2) != 0;
                } else {
                    host.listed.insert(n(1), n(2) != 0);
                }
                println!("{{}}");
            }
            "dead" => {
                host.dead.insert(n(1), n(2) as i16);
                println!("{{}}");
            }
            "size" => {
                host.size.insert(n(1), n(2) as i32);
                println!("{{}}");
            }
            "levelup" => {
                let i = fx.level_up(&mut host, n(1));
                started(&mut fx, i);
            }
            "dying" => {
                let i = fx.dying(&mut host, n(1));
                started(&mut fx, i);
            }
            "evolvepg" => {
                let i = fx.evolve_pg(&mut host, n(1));
                started(&mut fx, i);
            }
            "growpg" => {
                let c = n(1);
                let (pos, height) = (host.char_pos(c), host.char_height(c));
                fx.grow_pg(&mut host, pos, height);
                started(&mut fx, None);
            }
            "drainctrl" => {
                let i = fx.drain_ctrl(&mut host, n(1), n(2), n(3) as i32, n(4) as i32, n(5) as i32);
                started(&mut fx, i);
            }
            "drain" => {
                let (ctrl, mut cx) = fx.split(&mut host);
                let i = drain::eff_drain(ctrl, &mut cx, n(1), n(2), n(3) as i32, n(4) as i32);
                started(&mut fx, i);
            }
            "protect" => {
                let i = fx.protect(&mut host, n(1), n(2) as i32, n(3) as i32);
                started(&mut fx, i);
            }
            "afterdrain" => {
                let i = fx.after_drain(&mut host, n(1), n(2) as i32);
                started(&mut fx, i);
            }
            "frame" => {
                fx.step(&mut host);
                let slots: Vec<String> = fx
                    .ctrl
                    .effects
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.status != 0)
                    .map(|(i, e)| slot(&fx, i, e))
                    .collect();
                let draws: Vec<String> = fx.draws().iter().map(|d| draw(&fx, d)).collect();
                let events: Vec<String> = fx.take_events().iter().map(event).collect();
                let g = gens(&mut fx);
                println!(
                    "{{\"slots\": [{}], \"draws\": [{}], \"events\": [{}], \"gens\": {}, \"rand\": {}}}",
                    slots.join(", "),
                    draws.join(", "),
                    events.join(", "),
                    g,
                    host.rand.0
                );
            }
            "set" => {
                let e = &mut fx.ctrl.effects[n(1) as usize];
                let v4 = || [n(3), n(4), n(5), n(6)];
                match w[2] {
                    "pos" => e.pos = v4(),
                    "rot" => e.rot = v4(),
                    "speed" => e.speed = v4(),
                    "temp" => e.temp = v4(),
                    "cnt" => e.cnt = n(3) as i16,
                    "age" => e.age = n(3) as i16,
                    other => panic!("unknown field {other}"),
                }
                println!("{{}}");
            }
            "mt" => {
                for _ in 0..n(1) {
                    host.mt.genrand();
                }
                println!("{{}}");
            }
            "circle" => {
                let pos = v3(1);
                let dirc = [n(4), n(5), n(6), 0];
                circle = MagicCircle::new(&fx.assets, pos, dirc, host.player, host.bounds);
                println!("{{\"state\": {}}}", circle_state(circle.as_ref().unwrap()));
            }
            "cframe" => {
                let input = CircleInput {
                    freeze: n(1) != 0,
                    disp_sw: n(2) != 0,
                    pl_dist: n(3),
                    set_transparency: n(4),
                    player_listed: player_listed && n(5) != 0,
                    ent_root: n(6) as i32,
                };
                let c = circle.as_mut().unwrap();
                let out = c.main(&fx.assets, &mut host, &input);
                let draws: Vec<String> = out.draws.iter().map(|d| draw(&fx, d)).collect();
                let events: Vec<String> = out
                    .events
                    .iter()
                    .map(|e| match e {
                        CircleEvent::Sound3d { se, pos } => format!("[\"se3d\", {se}, {}]", list(pos)),
                        CircleEvent::EntryObject => "[\"entry\"]".into(),
                        CircleEvent::Opened => "[\"opened\"]".into(),
                    })
                    .collect();
                println!(
                    "{{\"state\": {}, \"draws\": [{}], \"events\": [{}], \"delete\": {}, \"mti\": {}}}",
                    circle_state(c),
                    draws.join(", "),
                    events.join(", "),
                    u8::from(out.delete),
                    host.mt.mti
                );
            }
            "acosf" => println!("{{\"v\": {}}}", dmath::acosf(n(1))),
            "sin" | "cos" => {
                let x = f64::from_bits(u64::from(n(1)) << 32 | u64::from(n(2)));
                let y = if w[0] == "sin" { dmath::sin(x) } else { dmath::cos(x) };
                let b = y.to_bits();
                println!("{{\"v\": [{}, {}]}}", b >> 32, b & 0xffff_ffff);
            }
            other => panic!("unknown request {other}"),
        }
    }
}
