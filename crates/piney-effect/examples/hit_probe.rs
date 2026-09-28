//! Answers `tools/test_effect_hit_rs.py`: runs the port's hits, fly fonts and
//! stacked numbers on the requests it is sent and prints each answer as one
//! JSON line, so the test can run the same through the game's own code in eemu
//! (`hit_probe ISO < requests`). Numbers are hex; floats travel as their bit
//! patterns. The requests (`reset`, `player`, `camera`, `screen`, `camid`,
//! `char`, the starters, the `fly*` entries, the stream demo's `strreset`,
//! `strhit`, `strtransfer`, `frame`, `fly`) are the test's; each answer that
//! changes state carries the events, the particles, the guard list and `flyFont`.

use std::collections::HashMap;
use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_effect::damupr::DamUprStr;
use piney_effect::draw::{Camera, DrawRec};
use piney_effect::effect::{Effect, Obj};
use piney_effect::flyfont::{FlyFonts, Font};
use piney_effect::particle::{Generator, Obj as PObj, Particle};
use piney_effect::{CharRef, Effects, Event, F, Host, IntRef, ONE, V4, VecRef};
use piney_world::Rand;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn ilist(v: &[i32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn bytes(v: &[u8]) -> String {
    ilist(&v.iter().map(|&b| i32::from(b)).collect::<Vec<_>>())
}

#[derive(Clone, Copy, Default)]
struct Char {
    pos: V4,
    dirc: V4,
    height: F,
    width: F,
    kind: i32,
    slot: i32,
    icons: i32,
    size: i32,
    alive: bool,
    attribute: i32,
}

struct Probe {
    rand: Rand,
    player: V4,
    camera: Camera,
    cam_id: i16,
    cam_type: i32,
    chars: HashMap<CharRef, Char>,
}

impl Probe {
    fn new(seed: u64) -> Probe {
        Probe {
            rand: Rand(seed),
            player: [0, 0, 0, ONE],
            camera: Camera::default(),
            cam_id: 0,
            cam_type: 0,
            chars: HashMap::new(),
        }
    }

    fn get(&self, c: CharRef) -> Char {
        self.chars.get(&c).copied().unwrap_or_default()
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
        self.get(c).pos
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        self.get(c).dirc
    }
    fn char_height(&self, c: CharRef) -> F {
        self.get(c).height
    }
    fn char_width(&self, c: CharRef) -> F {
        self.get(c).width
    }
    fn check_target(&self, c: CharRef) -> bool {
        self.chars.get(&c).is_some_and(|c| c.alive)
    }
    fn char_type(&self, c: CharRef) -> i32 {
        self.get(c).kind
    }
    fn party_slot(&self, c: CharRef) -> i32 {
        self.get(c).slot
    }
    fn condition_icon_num(&self, c: CharRef) -> i32 {
        self.get(c).icons
    }
    fn camera_id(&self) -> i16 {
        self.cam_id
    }
    fn camera_type(&self) -> i32 {
        self.cam_type
    }
    fn object_size(&self, c: CharRef) -> i32 {
        self.get(c).size
    }
    fn char_attribute(&self, c: CharRef, _flag: i32) -> i32 {
        self.get(c).attribute
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

fn iref(r: Option<IntRef>) -> String {
    match r {
        None => "null".into(),
        Some(IntRef::CharAt(c, off)) => format!("[\"at\", {c}, {off}]"),
        Some(IntRef::EffectTemp(s, k)) => format!("[\"efftemp\", {s}, {k}]"),
        Some(IntRef::EffectFlags(s)) => format!("[\"effflags\", {s}]"),
    }
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or("null".into(), |v| v.to_string())
}

fn gen_json(g: &Generator) -> String {
    let bits = u32::from(g.dist_sw)
        | u32::from(g.g_sync) << 1
        | u32::from(g.p_sync) << 2
        | u32::from(g.p_stop) << 3
        | u32::from(g.sync_pos_type) << 4
        | u32::from(g.sync_pos_type2) << 5
        | (u32::from(g.str_flag as u8) & 3) << 6
        | u32::from(g.pause) << 8;
    let ff: Vec<String> = g.ff.iter().map(|f| opt(f.map(|f| f.va))).collect();
    format!(
        concat!(
            "{{\"sn\": {}, \"param\": {}, \"bits\": {}, \"pCnt\": {}, \"gAge\": {}, \"gLife\": {}, \"pLife\": {}, ",
            "\"pNum\": {}, \"regOfst\": {}, \"syncRot\": {}, \"syncPos\": {}, \"syncPos2\": {}, \"syncSW\": {}, ",
            "\"gRate\": {}, \"gRateCnt\": {}, \"pTexMod\": {}, \"ff\": [{}], \"layer\": {}, \"rot\": {}, ",
            "\"pos\": {}, \"pos2\": {}, \"offset\": {}, \"offset2\": {}, \"velocity\": {}, \"mat\": {}, ",
            "\"kill\": {}, \"end\": {}}}"
        ),
        g.sn,
        g.param.map_or(0, |p| p.va),
        bits,
        g.p_cnt,
        g.g_age,
        g.g_life,
        g.p_life,
        g.p_num,
        g.regularly_ofst,
        vref(g.sync_rot),
        vref(g.sync_pos),
        vref(g.sync_pos2),
        iref(g.sync_sw),
        g.g_rate,
        g.g_rate_cnt,
        g.p_tex_mod,
        ff.join(", "),
        opt(g.layer),
        list(&g.rot),
        list(&g.pos),
        list(&g.pos2),
        list(&g.offset),
        list(&g.offset2),
        list(&g.velocity),
        list(&g.mat.concat()),
        g.kill_flag,
        u8::from(g.end_flag)
    )
}

fn part_json(fx: &Effects, i: usize, p: &Particle) -> String {
    let bits = (u32::from(p.style as u8) & 7)
        | u32::from(p.dist_sw) << 3
        | u32::from(p.disp_sw) << 4
        | u32::from(p.end_flag) << 5
        | u32::from(p.kill_flag) << 6
        | u32::from(p.sync_flag) << 7
        | (u32::from(p.fade_flag as u8) & 15) << 8
        | (u32::from(p.anm_type as u8) & 15) << 12
        | (u32::from(p.str_flag) & 15) << 16;
    let name = |o| format!("\"{}\"", fx.assets.name(o));
    let obj = match &p.obj {
        PObj::None => "null".into(),
        PObj::Eff(e) => format!(
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
            e.chunk,
            e.scale_x,
            e.scale_y,
            e.rotate,
            e.color,
            e.transparency,
            e.flag,
            e.prim,
            e.test,
            list(&e.pos)
        ),
        PObj::Clump { obj, matrix } => format!("[\"clump\", {}, {}]", name(*obj), list(&matrix.concat())),
        PObj::Anm { obj, play, matrix } => {
            format!("[\"anm\", {}, {}, {}]", name(*obj), play.time, list(&matrix.concat()))
        }
    };
    let clut = match p.clut {
        None => "null".into(),
        Some((a, b)) => format!("[{}, {}]", name(a), b.map_or("null".into(), name)),
    };
    format!(
        concat!(
            "{{\"i\": {}, \"bits\": {}, \"gene\": {}, \"obj\": {}, \"clut\": {}, \"layer\": {}, \"sn\": {}, ",
            "\"anmPat\": {}, \"texID\": {}, \"pos\": {}, \"rot\": {}, \"offset\": {}, \"dirc\": {}, ",
            "\"scale\": {}, \"velocity\": {}, \"transparency\": {}, \"speed\": {}, \"size\": {}, \"temp\": {}, ",
            "\"color\": {}, \"fadeIn\": {}, \"fadeOut\": {}, \"life\": {}, \"age\": {}, \"cnt\": {}, ",
            "\"rotate\": {}, \"ofstD\": {}, \"ofstR\": {}}}"
        ),
        i,
        bits,
        opt(p.gene),
        obj,
        clut,
        opt(p.layer),
        p.sn,
        p.anm_pat,
        p.tex_id,
        list(&p.pos),
        list(&p.rot),
        list(&p.offset),
        list(&p.dirc),
        list(&p.scale),
        list(&p.velocity),
        p.transparency,
        p.speed,
        p.size,
        p.temp,
        p.color,
        p.fade_in_d,
        p.fade_out_d,
        p.life_time,
        p.age,
        p.cnt,
        list(&p.rotate.map(u32::from)),
        p.ofst_d,
        p.ofst_r
    )
}

/// A particle's draw as test_effect_particle_rs records it (its CLUT last).
fn pdraw(fx: &Effects, d: &DrawRec) -> String {
    match d {
        DrawRec::Eff { eff, pat, layer } => format!(
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
            eff.chunk,
            pat,
            list(&eff.pos),
            eff.scale_x,
            eff.scale_y,
            eff.rotate,
            eff.color,
            eff.transparency,
            layer,
            eff.clut.map_or("null".into(), |c| format!("\"{}\"", fx.assets.name(c)))
        ),
        _ => draw(fx, d),
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
    let clut = match (&e.obj, e.clut_swap) {
        (Obj::Clump(o), Some((a, b))) => {
            let f = &fx.assets.files[o.file].ccs;
            format!("[\"{}\", \"{}\"]", f.object_name(a).unwrap_or_default(), f.object_name(b).unwrap_or_default())
        }
        _ => "null".into(),
    };
    // A sprite's TEST and its own palette (ccEff +0x3c, `Eff::clut`).
    let (efftest, effclut) = match &e.obj {
        Obj::Eff(f) => (f.test.to_string(), f.clut.map_or("null".into(), |c| format!("\"{}\"", fx.assets.name(c)))),
        _ => ("null".into(), "null".into()),
    };
    format!(
        concat!(
            "{{\"i\": {}, \"id\": {}, \"status\": {}, \"life\": {}, \"age\": {}, \"cnt\": {}, \"pat\": {}, ",
            "\"bits\": {}, \"param\": {}, \"flags\": {}, \"pos\": {}, \"offset\": {}, \"rot\": {}, ",
            "\"speed\": {}, \"scale\": {}, \"posT\": {}, \"rotSpeed\": {}, \"velocity\": {}, ",
            "\"transparency\": {}, \"target\": {}, \"posPtr\": {}, \"rotPtr\": {}, \"link\": {}, ",
            "\"temp\": {}, \"sn\": {}, \"obj\": {}, \"clut\": {}, \"efftest\": {}, \"effclut\": {}}}"
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
        obj,
        clut,
        efftest,
        effclut
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

/// The events since the last answer, the particle system's whole state
/// (every generator on the list, every live particle, the control's counts
/// and serial numbers), the guard list and `flyFont`'s fields.
fn extras(fx: &mut Effects) -> String {
    let events: Vec<String> = fx.take_events().iter().map(event).collect();
    let gens: Vec<String> = fx.particles.gens.iter().map(gen_json).collect();
    let parts: Vec<String> = fx.particles.live().map(|(i, p)| part_json(fx, i, p)).collect();
    let p = &fx.particles;
    let ctrl = format!(
        "[{}, {}, {}, {}, {}, {}]",
        p.g_num, p.p_num, p.search, p.generator_serial, p.particle_serial, p.p2_num
    );
    let guard: Vec<String> = fx.hits.guard.iter().map(|g| who(*g).to_string()).collect();
    format!(
        "\"events\": [{}], \"gens\": [{}], \"parts\": [{}], \"ctrl\": {}, \"guard\": [{}], \"font\": {}",
        events.join(", "),
        gens.join(", "),
        parts.join(", "),
        ctrl,
        guard.join(", "),
        font(&fx.font)
    )
}

fn font(f: &Font) -> String {
    format!(
        "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
        f.ctrl,
        f.transp,
        f.cx,
        f.cy,
        f.dx,
        f.dy,
        f.sx,
        f.sy,
        f.su,
        f.sv,
        f.wu,
        f.wv,
        f.wi,
        list(&f.rgba),
        f.packet_num
    )
}

fn fly(ff: &FlyFonts) -> String {
    let e: Vec<String> = ff
        .entries
        .iter()
        .map(|e| {
            format!(
                "[{}, {}, {}, {}, {}, {}, {}, {}]",
                e.color,
                e.timer,
                e.num,
                list(&e.pos),
                bytes(&e.text),
                e.ofs_x,
                e.sx,
                e.sy
            )
        })
        .collect();
    format!("[{}]", e.join(", "))
}

fn dam(d: &DamUprStr) -> String {
    let nodes: Vec<String> = d
        .nodes
        .iter()
        .map(|n| {
            let u = &n.uproll;
            let lines: Vec<String> = u
                .lines
                .iter()
                .map(|l| {
                    format!(
                        "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
                        bytes(&l.text),
                        l.color,
                        l.alpha_cnt,
                        l.alpha,
                        l.lx,
                        l.ly,
                        l.addly,
                        l.ftype,
                        l.sx,
                        l.sy
                    )
                })
                .collect();
            format!(
                "[{}, [{}], {}, {}, {}, {}, {}]",
                who(n.ch),
                lines.join(", "),
                u.line_num,
                u.line_top,
                u.x,
                u.y,
                u8::from(u.disp_sw)
            )
        })
        .collect();
    format!("[{}]", nodes.join(", "))
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
        let opt = |x: u32| (x != 0xffff_ffff).then_some(x);
        match w[0] {
            "reset" => {
                fx.ctrl = piney_effect::effect::EffectCtrl::new();
                fx.ctrl.town = n(1) != 0;
                fx.particles = Default::default();
                fx.particles.install_statics(&fx.assets);
                fx.hits.flip.clear();
                fx.hits.guard = Default::default();
                fx.fly = Default::default();
                fx.dam = Default::default();
                fx.font = Font::fly();
                fx.take_events();
                host = Probe::new(u64::from(n(2)));
                println!("{{}}");
            }
            "strreset" => {
                // The stream demo's systems in place of the field's:
                // ccEffectCtrl(1) and ccParticleCtrl(1).
                fx.ctrl = piney_effect::effect::EffectCtrl::stream();
                fx.particles = piney_effect::particle::Particles::stream();
                fx.particles.install_statics(&fx.assets);
                fx.take_events();
                host = Probe::new(u64::from(n(1)));
                println!("{{}}");
            }
            "strhit" => {
                let (ctrl, mut cx) = fx.split(&mut host);
                let rot = [n(4), n(5), n(6), 0];
                piney_effect::strfx::eff_hit_mark_str(ctrl, &mut cx, v3(1), rot, opt(n(7)).map(|l| l as i16));
                println!("{{{}}}", extras(&mut fx));
            }
            "strtransfer" => {
                let (ctrl, mut cx) = fx.split(&mut host);
                piney_effect::strfx::eff_transfer_str(ctrl, &mut cx, v3(1), n(4), opt(n(5)).map(|l| l as i16));
                println!("{{{}}}", extras(&mut fx));
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
            "screen" => {
                for k in 0..16 {
                    host.camera.world_screen[k / 4][k % 4] = n(1 + k);
                }
                println!("{{}}");
            }
            "camid" => {
                host.cam_id = n(1) as i16;
                host.cam_type = n(2) as i32;
                println!("{{}}");
            }
            "char" => {
                let c = Char {
                    pos: v3(2),
                    height: n(5),
                    width: n(6),
                    dirc: [n(7), n(8), n(9), 0],
                    kind: n(10) as i32,
                    slot: n(11) as i32,
                    icons: n(12) as i32,
                    size: n(13) as i32,
                    alive: n(14) != 0,
                    attribute: w.get(15).map_or(-1, |s| hex(s) as i32),
                };
                host.chars.insert(n(1), c);
                println!("{{}}");
            }
            "hit" => {
                fx.hit_mark(&mut host, n(1), n(2));
                println!("{{{}}}", extras(&mut fx));
            }
            "protect" => {
                let i = fx.protect(&mut host, n(1), n(2) as i32, n(3) as i32);
                println!("{{\"slot\": {}, {}}}", i.map_or(-1, |i| i as i64), extras(&mut fx));
            }
            "critical" | "crit" | "dying" | "nodamage" => {
                let i = match w[0] {
                    "critical" => fx.attribute_critical(&mut host, n(1)),
                    "crit" => fx.critical(&mut host, n(1)),
                    "dying" => fx.dying(&mut host, n(1)),
                    _ => fx.no_damage(&mut host, n(1)),
                };
                println!("{{\"slot\": {}, {}}}", i.map_or(-1, |i| i as i64), extras(&mut fx));
            }
            "guard" => {
                let i = fx.attribute_guard(&mut host, n(1), n(2));
                println!("{{\"slot\": {}, {}}}", i.map_or(-1, |i| i as i64), extras(&mut fx));
            }
            "flynum" => {
                let i = fx.fly_font_num(n(1) as i32, n(2) as i32, v3(3), n(6), n(7), n(8) as i32);
                println!(
                    "{{\"slot\": {}, \"fly\": {}, {}}}",
                    i.map_or(-1, |i| i as i64),
                    fly(&fx.fly),
                    extras(&mut fx)
                );
            }
            "flynew" | "flyexp" | "flylevel" | "flymiss" => {
                let i = match w[0] {
                    "flynew" => fx.fly_font(opt(n(3)), n(1) as i32, n(2) as i32),
                    "flyexp" => fx.fly_font_exp(opt(n(3)), n(1) as i32, n(2) as i32),
                    "flylevel" => fx.fly_font_level_down(opt(n(2)), n(1) as i32),
                    _ => fx.fly_font_miss(opt(n(1))),
                };
                println!("{{\"node\": {}, \"dam\": {}, {}}}", i, dam(&fx.dam), extras(&mut fx));
            }
            "frame" => {
                fx.step(&mut host);
                fx.step_particles(&mut host);
                let slots: Vec<String> = fx
                    .ctrl
                    .effects
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.status != 0)
                    .map(|(i, e)| slot(&fx, i, e))
                    .collect();
                let draws: Vec<String> = fx.draws().iter().map(|d| draw(&fx, d)).collect();
                let pdraws: Vec<String> = fx.particle_draws().iter().map(|d| pdraw(&fx, d)).collect();
                let x = extras(&mut fx);
                println!(
                    "{{\"slots\": [{}], \"draws\": [{}], \"pdraws\": [{}], {}, \"rand\": {}}}",
                    slots.join(", "),
                    draws.join(", "),
                    pdraws.join(", "),
                    x,
                    host.rand.0
                );
            }
            "fly" => {
                fx.fly_fonts(&host, n(1) != 0, n(2) != 0);
                let quads: Vec<String> = fx
                    .font
                    .take()
                    .iter()
                    .map(|q| {
                        format!(
                            "[{}, {}, {}, {}, {}, {}, {}, {}, {}]",
                            list(&q.rgba),
                            q.uv0.0,
                            q.uv0.1,
                            q.xy0.0,
                            q.xy0.1,
                            q.uv1.0,
                            q.uv1.1,
                            q.xy1.0,
                            q.xy1.1
                        )
                    })
                    .collect();
                // flyFont's fields after the send (packetNum 0).
                let x = extras(&mut fx);
                println!(
                    "{{\"fly\": {}, \"dam\": {}, \"quads\": [{}], {}}}",
                    fly(&fx.fly),
                    dam(&fx.dam),
                    quads.join(", "),
                    x
                );
            }
            other => panic!("unknown request {other}"),
        }
    }
}
