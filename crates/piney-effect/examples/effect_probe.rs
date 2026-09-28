//! Answers `tools/test_effect_rs.py`: runs the port's effects on the requests
//! it is sent and prints each answer as one JSON line, so the test can run the
//! same frames through the game's own code in eemu (`effect_probe ISO <
//! requests`). Numbers are hex; floats travel as their bit patterns. The
//! requests are `reset TOWN SEED`, `player`, `camera`, `char`, `transfer`,
//! `warp`, `spawn`, `set SLOT KEY V...` (a slot's field, as the game's code
//! would set it), `patch` (an `effectTbl` row naming another object) and
//! `frame` (one `ccEffectCtrl::Main`), with the arguments the test sends.

use std::collections::HashMap;
use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_effect::draw::{Camera, DrawRec};
use piney_effect::effect::{Effect, Obj};
use piney_effect::{CharRef, Effects, Event, Host, ONE, V4, VecRef};
use piney_world::Rand;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

struct Probe {
    rand: Rand,
    player: V4,
    camera: Camera,
    chars: HashMap<CharRef, (V4, V4, u32, u32)>,
}

impl Probe {
    fn new(seed: u64) -> Probe {
        Probe { rand: Rand(seed), player: [0, 0, 0, ONE], camera: Camera::default(), chars: HashMap::new() }
    }
}

impl Host for Probe {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn player_pos(&self) -> V4 {
        self.player
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
        Obj::Eff(f) => format!(
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}]",
            f.scale_x,
            f.scale_y,
            f.rotate,
            f.color,
            f.transparency,
            f.flag,
            list(&f.pos)
        ),
    };
    format!(
        concat!(
            "{{\"i\": {}, \"id\": {}, \"status\": {}, \"life\": {}, \"age\": {}, \"cnt\": {}, \"pat\": {}, ",
            "\"bits\": {}, \"param\": {}, \"flags\": {}, \"pos\": {}, \"offset\": {}, \"rot\": {}, ",
            "\"speed\": {}, \"scale\": {}, \"posT\": {}, \"rotSpeed\": {}, \"velocity\": {}, ",
            "\"transparency\": {}, \"target\": {}, \"posPtr\": {}, \"rotPtr\": {}, \"link\": {}, ",
            "\"temp\": {}, \"sn\": {}, \"obj\": {}}}"
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

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let mut fx = Effects::new(&archive, iso.volume().unwrap()).unwrap();
    let mut host = Probe::new(0);
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
                println!("{{}}");
            }
            "player" => {
                host.player = v3(1);
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
            "transfer" | "warp" => {
                let i = if w[0] == "warp" { fx.warp_transfer(&mut host, n(1)) } else { fx.transfer(&mut host, n(1)) };
                let events: Vec<String> = fx.take_events().iter().map(event).collect();
                println!("{{\"slot\": {}, \"events\": [{}]}}", i.map_or(-1, |i| i as i64), events.join(", "));
            }
            "spawn" => {
                let (ctrl, cx) = fx.split(&mut host);
                let i = ctrl.new_effect(&cx, n(1) as i16);
                println!("{{\"slot\": {}}}", i.map_or(-1, |i| i as i64));
            }
            "set" => {
                let e = &mut fx.ctrl.effects[n(1) as usize];
                let v4 = [
                    n(3),
                    w.get(4).map_or(0, |s| hex(s)),
                    w.get(5).map_or(0, |s| hex(s)),
                    w.get(6).map_or(0, |s| hex(s)),
                ];
                let opt = |x: u32| (x != 0xffff_ffff).then_some(x);
                let vref = |k: u32, x: u32| match k {
                    0 => Some(VecRef::CharPos(x)),
                    1 => Some(VecRef::CharDirc(x)),
                    2 => Some(VecRef::EffectPos(x as usize)),
                    3 => Some(VecRef::EffectRot(x as usize)),
                    _ => None,
                };
                match w[2] {
                    "pos" => e.pos = v4,
                    "offset" => e.offset = v4,
                    "rot" => e.rot = v4,
                    "speed" => e.speed = v4,
                    "scale" => e.scale = v4,
                    "temp" => e.temp = v4,
                    "bits" => {
                        let b = n(3);
                        e.dist_sw = b & 1 != 0;
                        e.disp_sw = b & 2 != 0;
                        e.pause_sw = b & 4 != 0;
                        e.end_flag = b & 8 != 0;
                        e.zyx_flag = b & 16 != 0;
                        e.level = (((b >> 5) & 15) as i8) << 4 >> 4;
                    }
                    "life" => e.life_time = n(3) as i16,
                    "age" => e.age = n(3) as i16,
                    "cnt" => e.cnt = n(3) as i16,
                    "pat" => e.tex_anm_pat = n(3) as u16,
                    "transparency" => e.transparency = n(3),
                    "target" => e.target = opt(n(3)),
                    "posptr" => e.pos_ptr = opt(n(3)).and_then(|k| vref(k, n(4))),
                    "rotptr" => e.rot_ptr = opt(n(3)).and_then(|k| vref(k, n(4))),
                    "link" => e.link = opt(n(3)).map(|l| l as usize),
                    "layer" => e.layer = opt(n(3)).map(|l| l as i16),
                    other => panic!("unknown field {other}"),
                }
                println!("{{}}");
            }
            "patch" => {
                let row = &mut fx.assets.tbl[n(1) as usize];
                row.ccs = w[2].to_string();
                row.name = w[3].to_string();
                row.kind = piney_effect::files::Kind::of(n(4));
                println!("{{}}");
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
                let gens: Vec<String> = std::mem::take(&mut fx.particles.started)
                    .iter()
                    .map(|g| {
                        format!("[{}, {}, {}, {}]", g.row, vref(g.sync_pos), u8::from(g.sync_pos_type), list(&g.offset))
                    })
                    .collect();
                println!(
                    "{{\"slots\": [{}], \"draws\": [{}], \"events\": [{}], \"gens\": [{}], \"rand\": {}}}",
                    slots.join(", "),
                    draws.join(", "),
                    events.join(", "),
                    gens.join(", "),
                    host.rand.0
                );
            }
            other => panic!("unknown request {other}"),
        }
    }
}
