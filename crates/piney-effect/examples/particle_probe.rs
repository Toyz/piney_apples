//! Answers `tools/test_effect_particle_rs.py`: runs the port's particle system
//! on the requests it is sent and prints each answer, the whole state, as one
//! JSON line, so the test can run the same frames through the game's own code
//! in eemu (`particle_probe ISO < requests`). Numbers are hex; floats travel
//! as their bit patterns. The requests are the test's: the scene (`reset`,
//! `player`, `camera`, `char`, `charvec`, `charint`, `spawn`), a generator
//! (`gen`, `gset`, `gstart`, `pkill`, `pdelete`), table patches to reach code
//! no row reaches, the starters, and `frame` (one `ccParticleCtrl::Main`).

use std::collections::HashMap;
use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_effect::draw::{Camera, DrawRec};
use piney_effect::effect::Obj as EObj;
use piney_effect::particle::{self, ConditionEffect, Generator, Obj, Particle};
use piney_effect::{CharRef, Effects, Event, Host, IntRef, ONE, V4, VecRef};
use piney_world::Rand;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

struct Probe {
    rand: Rand,
    /// `genrand()` (`ccRand`), seeded as the harness seeds its MT19937.
    mt: piney_world::mt::Mt,
    /// What `checkHitResultAttlibute` answers.
    attr: u32,
    player: V4,
    camera: Camera,
    chars: HashMap<CharRef, (V4, V4, u32, u32)>,
    vecs: HashMap<(CharRef, u16), V4>,
    ints: HashMap<(CharRef, u16), i32>,
}

impl Probe {
    fn new(seed: u64) -> Probe {
        Probe {
            rand: Rand(seed),
            mt: piney_world::mt::Mt::seeded(seed as u32),
            attr: 0,
            player: [0, 0, 0, ONE],
            camera: Camera::default(),
            chars: HashMap::new(),
            vecs: HashMap::new(),
            ints: HashMap::new(),
        }
    }
}

impl Host for Probe {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn genrand(&mut self) -> u32 {
        self.mt.genrand()
    }
    fn ground_attribute(&mut self, _pos: V4) -> u32 {
        self.attr
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
    fn char_vec(&self, c: CharRef, off: u16) -> V4 {
        match off {
            0x40 => self.char_pos(c),
            0x60 => self.char_dirc(c),
            _ => self.vecs.get(&(c, off)).copied().unwrap_or([0; 4]),
        }
    }
    fn char_int(&self, c: CharRef, off: u16) -> i32 {
        self.ints.get(&(c, off)).copied().unwrap_or(0)
    }
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
        Obj::None => "null".into(),
        Obj::Eff(e) => format!(
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
        Obj::Clump { obj, matrix } => format!("[\"clump\", {}, {}]", name(*obj), list(&matrix.concat())),
        Obj::Anm { obj, play, matrix } => {
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
    }
}

fn event(e: &Event) -> String {
    match e {
        Event::Sound3d { se, pos } => format!("[\"se3d\", {se}, {}]", list(pos)),
        Event::CameraShake(a) => format!("[\"shake\", {}, {}, {}, {}]", a[0], a[1], a[2], a[3]),
        other => format!("[\"?\", \"{other:?}\"]"),
    }
}

/// The live effect slots as the starters leave them: id, the +0x60 bits,
/// life, velocity,
/// offset, temp[0], layer, serial number, and the sprite's TEST.
fn effects_json(fx: &Effects) -> String {
    let v: Vec<String> = fx
        .ctrl
        .effects
        .iter()
        .enumerate()
        .filter(|(_, e)| e.status != 0)
        .map(|(i, e)| {
            let test = match &e.obj {
                EObj::Eff(f) => f.test.to_string(),
                _ => "null".into(),
            };
            let bits = u32::from(e.dist_sw)
                | u32::from(e.disp_sw) << 1
                | u32::from(e.pause_sw) << 2
                | u32::from(e.end_flag) << 3
                | u32::from(e.zyx_flag) << 4
                | (u32::from(e.level as u8) & 15) << 5;
            format!(
                "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
                i,
                e.id,
                bits,
                e.life_time,
                e.velocity,
                list(&e.offset),
                e.temp[0],
                opt(e.layer),
                e.sn,
                test
            )
        })
        .collect();
    format!("[{}]", v.join(", "))
}

fn cond_json(c: &ConditionEffect) -> String {
    let g: Vec<String> = c.gene.iter().map(|g| opt(*g)).collect();
    format!("{{\"gene\": [{}], \"eff\": {}, \"effSN\": {}}}", g.join(", "), opt(c.eff), c.eff_sn)
}

fn state(fx: &mut Effects, host: &Probe, extra: &str, frame: bool) -> String {
    let gens: Vec<String> = fx.particles.gens.iter().map(gen_json).collect();
    let parts: Vec<String> = fx.particles.live().map(|(i, p)| part_json(fx, i, p)).collect();
    let draws: Vec<String> = if frame { fx.particle_draws().iter().map(|d| draw(fx, d)).collect() } else { Vec::new() };
    let events: Vec<String> = fx.take_events().iter().map(event).collect();
    let p = &fx.particles;
    format!(
        concat!(
            "{{{}\"gens\": [{}], \"parts\": [{}], \"draws\": [{}], \"effects\": {}, \"events\": [{}], ",
            "\"ctrl\": [{}, {}, {}, {}, {}, {}], \"rand\": {}}}"
        ),
        extra,
        gens.join(", "),
        parts.join(", "),
        draws.join(", "),
        effects_json(fx),
        events.join(", "),
        p.g_num,
        p.p_num,
        p.search,
        p.generator_serial,
        p.particle_serial,
        p.p2_num,
        host.rand.0
    )
}

/// A syncpos-style reference: KIND and its arguments from `w[i..]`, and
/// how many words it took.
fn vec_ref(w: &[&str], i: usize) -> (Option<VecRef>, usize) {
    let k = hex(w[i]);
    match k {
        0 => (Some(VecRef::CharPos(hex(w[i + 1]))), 2),
        1 => (Some(VecRef::CharDirc(hex(w[i + 1]))), 2),
        4 => (Some(VecRef::CharAt(hex(w[i + 1]), hex(w[i + 2]) as u16)), 3),
        5 => (Some(VecRef::EffectPosT(hex(w[i + 1]) as usize)), 2),
        _ => (None, 1),
    }
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let mut fx = Effects::new(&archive, iso.volume().unwrap()).unwrap();
    let mut host = Probe::new(0);
    let mut pending: Option<Generator> = None;
    let mut conds: Vec<Option<ConditionEffect>> = Vec::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let n = |i: usize| hex(w[i]);
        let v3 = |i: usize| [hex(w[i]), hex(w[i + 1]), hex(w[i + 2]), ONE];
        let v4 = |i: usize| [hex(w[i]), hex(w[i + 1]), hex(w[i + 2]), hex(w[i + 3])];
        let none = |x: u32| (x != 0xffff_ffff).then_some(x);
        let mut extra = String::new();
        match w[0] {
            "reset" => {
                fx.ctrl = piney_effect::effect::EffectCtrl::new();
                fx.ctrl.town = n(1) != 0;
                fx.particles = Default::default();
                fx.particles.install_statics(&fx.assets);
                fx.take_events();
                host = Probe::new(u64::from(n(2)));
                pending = None;
                conds.clear();
            }
            "player" => host.player = v3(1),
            "camera" => {
                host.camera.eye = v3(1);
                host.camera.cam_pos = v3(4);
                host.camera.cam_view = v3(7);
            }
            "char" => {
                // A character moved keeps its heading, as the machine's does.
                let dirc = host.chars.get(&n(1)).map_or([0; 4], |c| c.1);
                host.chars.insert(n(1), (v3(2), dirc, n(5), n(6)));
            }
            "charvec" => {
                host.vecs.insert((n(1), n(2) as u16), v4(3));
                if n(2) == 0x60
                    && let Some(c) = host.chars.get_mut(&n(1))
                {
                    c.1 = v4(3);
                }
            }
            "smoke" => {
                let ok = fx.smoke(&mut host, v3(1), v3(4), n(7), n(8) as i32, n(9) as i32, n(10) as i16, n(11) as i16);
                extra = format!("\"ret\": {}, ", u8::from(ok));
            }
            "pawsmoke" => {
                host.attr = n(9);
                let ok = fx.paw_smoke(&mut host, [v3(1), v3(4)], n(7), n(8));
                extra = format!("\"ret\": {}, ", u8::from(ok));
            }
            "enemydust" => fx.enemy_dust(&mut host, v3(1), n(4) as i32, n(5), n(6) as i32, n(7) as i32),
            "dustring" => fx.dust_ring(&mut host, v3(1), n(4), n(5), n(6) as i32, n(7) as i32, n(8) as i32),
            "dustringch" => {
                let (pos, dirc) = host.chars.get(&n(1)).map_or(([0, 0, 0, ONE], [0; 4]), |c| (c.0, c.1));
                fx.enemy_dust_ring(&mut host, pos, dirc, v3(2), n(5), n(6), n(7) as i32, n(8) as i32, n(9) as i32);
            }
            "charint" => {
                host.ints.insert((n(1), n(2) as u16), n(3) as i32);
            }
            "spawn" => {
                let (ctrl, cx) = fx.split(&mut host);
                let i = ctrl.new_effect(&cx, n(1) as i16);
                extra = format!("\"slot\": {}, ", i.map_or(-1, |i| i as i64));
            }
            "gen" => {
                let g = fx.particles.generator(&fx.assets, n(1) as usize);
                extra = format!("\"sn\": {}, ", g.sn);
                pending = Some(g);
            }
            "patchgen" => {
                let g = &mut fx.assets.particle.generators[n(1) as usize];
                match w[2] {
                    "gtype" => g.g_type = n(3) as u8,
                    "rtype" => g.r_type = n(3) as u8,
                    "dtype" => g.d_type = n(3) as u8,
                    other => panic!("unknown field {other}"),
                }
            }
            "patchff" => {
                let f = &mut fx.assets.particle.force_fields[n(1) as usize];
                match w[2] {
                    "calc" => f.calc_type = n(3) as u8,
                    "field" => f.field_type = n(3) as u8,
                    "force" => f.force_type = n(3) as u8,
                    other => panic!("unknown field {other}"),
                }
            }
            "gset" => {
                let g = pending.as_mut().unwrap();
                match w[1] {
                    "syncpos" => g.sync_pos = vec_ref(&w, 2).0,
                    "syncpos2" => g.sync_pos2 = vec_ref(&w, 2).0,
                    "syncrot" => g.sync_rot = vec_ref(&w, 2).0,
                    "syncsw" => g.sync_sw = none(n(2)).map(|c| IntRef::CharAt(c, n(3) as u16)),
                    "pos" => g.pos = v4(2),
                    "pos2" => g.pos2 = v4(2),
                    "offset" => g.offset = v4(2),
                    "offset2" => g.offset2 = v4(2),
                    "rot" => g.rot = v4(2),
                    "layer" => g.layer = none(n(2)).map(|l| l as i16),
                    "tex" => g.p_tex_mod = n(2) as i16,
                    "spt" => g.sync_pos_type = n(2) != 0,
                    "spt2" => g.sync_pos_type2 = n(2) != 0,
                    "pause" => g.pause = n(2) != 0,
                    "kill" => g.kill_flag = n(2) as i8,
                    "rate" => g.g_rate_cnt = n(2),
                    other => panic!("unknown field {other}"),
                }
            }
            "gstart" => {
                let g = pending.take().unwrap();
                fx.particles.start(g);
            }
            "gpause" => {
                if let Some(g) = fx.particles.get_mut(n(1)) {
                    g.pause = n(2) != 0;
                }
            }
            "pkill" => fx.particles.particle_kill(n(1)),
            "pdelete" => fx.particles.particle_delete(n(1)),
            "hitmark" => {
                let (_, mut cx) = fx.split(&mut host);
                particle::cc_particle_hit_mark(&mut cx, v3(1));
            }
            "heal" => {
                let (_, mut cx) = fx.split(&mut host);
                particle::cc_particle_heal(&mut cx, n(1), 0);
            }
            "explode" => {
                let (_, mut cx) = fx.split(&mut host);
                let v = [n(4), n(5), n(6), 0];
                particle::cc_particle_explode(&mut cx, v3(1), v, n(7), n(8) as i32);
            }
            "psetup" => {
                let (_, mut cx) = fx.split(&mut host);
                let s = particle::cc_particle_setup(&mut cx, n(1) as i32, v3(2), n(5) as i16, n(6) as i32, none(n(7)));
                extra = format!("\"ret\": {}, ", s.map_or(-1, |s| s as i64));
            }
            "pgene" => fx.particles.slots[n(1) as usize].gene = none(n(2)),
            "peffect" => {
                let (_, mut cx) = fx.split(&mut host);
                particle::start_particle_effect(&mut cx, n(1), n(2) as usize);
            }
            "peffect2" => {
                let (s, k) = vec_ref(&w, 1);
                let (e, j) = vec_ref(&w, 1 + k);
                let i = 1 + k + j;
                let sw = none(hex(w[i + 1])).map(|c| IntRef::CharAt(c, hex(w[i + 2]) as u16));
                let (ctrl, mut cx) = fx.split(&mut host);
                particle::start_particle_effect2(ctrl, &mut cx, s.unwrap(), e.unwrap(), hex(w[i]) as usize, sw);
            }
            "cond" => {
                let slot = none(n(3)).map(|s| s as usize);
                let (ctrl, mut cx) = fx.split(&mut host);
                let c = particle::set_condition_effect(ctrl, &mut cx, n(1), n(2) as i32, |_, _, _, _, _| slot);
                extra = format!("\"cond\": {}, \"h\": {}, ", cond_json(&c), conds.len());
                conds.push(Some(c));
            }
            "condkill" | "conddel" => {
                let c = conds[n(1) as usize].take().unwrap();
                let (ctrl, mut cx) = fx.split(&mut host);
                if w[0] == "condkill" {
                    particle::kill_condition_effect(ctrl, &mut cx, c);
                } else {
                    particle::delete_condition_effect(ctrl, &mut cx, c);
                }
            }
            "frame" => fx.step_particles(&mut host),
            other => panic!("unknown request {other}"),
        }
        println!("{}", state(&mut fx, &host, &extra, w[0] == "frame"));
    }
}
