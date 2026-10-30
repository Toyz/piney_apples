//! `battle_probe` requests for Corbenik's first form
//! (`piney_battle::boss::corbenik`), as `tools/test_battle_corbenik_rs.py`
//! sends them: `corbenikclips` (the clips of `x81` and `xeffect`) and
//! `corbenik ...`: the body and its seeds made on a scene, then FRAMES
//! frames of `Main` (its manager's pass first) with affects before each
//! (0 a hit, 5 and 6 the stop's affects, 7 a member's `dead`, 8 an act
//! forced, 9 a hit of type 3, 10 a hit of type 1 by skill `who`, 11 and 12
//! a hit of type 1 or 3 on seed `who % 8` by skill `who / 8`, 13 its own
//! skill over). The cameras keep what is set on them.
//! One JSON line: the state after the constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::corbenik::first::First;
use piney_battle::boss::corbenik::seed::Seed;
use piney_battle::boss::corbenik::{self, Gen, Pic};
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, EffKind, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::Events;
use piney_battle::exp::Party;
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;

use crate::boss::{eff_num, out_json, with_blur};
use crate::{Toks, read_char};

type Clips = std::collections::HashMap<String, (u32, bool)>;
type V4 = [u32; 4];

thread_local! {
    static DATA: std::cell::RefCell<Option<(BossData, Clips, V4)>> = const { std::cell::RefCell::new(None) };
}

/// The clips of Corbenik's file and the effects', and `DMY_dummy1`'s
/// place in its file (w 1, as `Decode_DummyPos` leaves it): the third
/// form's eye dummy.
fn load(iso_path: &str) -> (BossData, Clips, V4) {
    let mut iso = Iso::open(iso_path).expect("the ISO");
    let data = BossData::of(iso.volume().expect("the volume"));
    let arc = Archive::new(iso.read_path("DATA/DATA.BIN").expect("DATA.BIN")).expect("DATA.BIN");
    let mut clips = std::collections::HashMap::new();
    let mut dummy = [0, 0, 0, 0x3f80_0000];
    for stem in [corbenik::FILE, "xeffect"] {
        let Ok(raw) = arc.inflate_named(stem) else { continue };
        let ccs = Ccs::parse(raw).expect("a CCSF");
        let scene = piney_data::scene::Scene::read(&ccs).expect("its scene");
        for (obj, d) in &scene.dummies {
            if ccs.object_name(*obj) == Some("DMY_dummy1") && stem == corbenik::FILE {
                dummy = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), 0x3f80_0000];
            }
        }
        for a in piney_data::anim::Animation::all(&ccs, &scene).expect("its animations") {
            if let Some(name) = ccs.object_name(a.object) {
                clips.insert(name.to_string(), (a.frames, a.looping));
            }
        }
    }
    (data, clips, dummy)
}

/// Answers `corbenikclips` and `corbenik`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "corbenikclips" | "corbenik" | "corbenikb" | "corbenikc" | "corbenikdummy") {
        return None;
    }
    DATA.with(|d| {
        if d.borrow().is_none() {
            *d.borrow_mut() = Some(load(iso));
        }
    });
    Some(DATA.with(|d| {
        let d = d.borrow();
        let (data, clips, dummy) = d.as_ref().unwrap();
        if cmd == "corbenikclips" {
            let mut v: Vec<_> = clips.iter().collect();
            v.sort();
            let body: Vec<String> = v.iter().map(|(k, (f, l))| format!("\"{k}\":[{f},{}]", u8::from(*l))).collect();
            format!("{{{}}}", body.join(","))
        } else if cmd == "corbenikb" {
            crate::corbenik_b::run(tables, data, clips, t)
        } else if cmd == "corbenikc" {
            crate::corbenik_c::run(tables, data, clips, *dummy, t)
        } else if cmd == "corbenikdummy" {
            format!("[{},{},{}]", dummy[0], dummy[1], dummy[2])
        } else {
            run(tables, data, clips, t)
        }
    }))
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

/// The land as the case has it: the plane z 0, none past x `hole`.
struct Plane {
    hole: f32,
}

impl piney_battle::boss::PartHits for Plane {
    fn switch(&mut self, _: usize, hit: &mut piney_battle::world::CharHit, on: bool) {
        hit.sw = on;
    }

    fn collide(&mut self, _: usize, hit: &mut piney_battle::world::CharHit) -> bool {
        hit.offset = [0; 4];
        false
    }

    fn sync(&mut self, _: usize, _: &piney_battle::world::CharHit) {}

    fn line(&mut self, _sp: V4, ep: V4, _mask: u32) -> Option<V4> {
        (f32::from_bits(ep[2]) <= 0.0 && f32::from_bits(ep[0]) <= self.hole).then_some(ep)
    }
}

/// The cameras as the harness keeps them: each one's eye and view as set,
/// `camID`, the active camera's `type` as written, and the boss camera.
#[derive(Default)]
pub(crate) struct Cams {
    pub(crate) set: std::collections::HashMap<i32, (V4, V4)>,
    pub(crate) id: i32,
    pub(crate) ty: i32,
    pub(crate) boss: BossCam,
    /// `ccBoss::Draw`'s camera fade, as the harness's active camera has it.
    pub(crate) fade: Option<piney_battle::boss::CamFade>,
}

/// The harness's zeroed `ccBossCam` as `SetMode`, `SetFreeCamPosView` and
/// the form's straight writes leave it: `lock`, `ResetFlg`, `ExLock`, its
/// view (+0x10) and eye (+0x20), and the kept ones (+0x40, +0x50).
#[derive(Default)]
pub(crate) struct BossCam {
    lock: bool,
    reset: i32,
    ex_lock: u8,
    view: V4,
    pos: V4,
    keep_view: V4,
    keep_pos: V4,
}

impl BossCam {
    pub(crate) fn json(&self) -> String {
        format!(
            "[{},{},{},{},{},{},{}]",
            u8::from(self.lock),
            self.reset,
            self.ex_lock,
            v3(&self.view),
            v3(&self.pos),
            v3(&self.keep_view),
            v3(&self.keep_pos)
        )
    }
}

impl Cams {
    pub(crate) fn get(&self, n: i32) -> (V4, V4) {
        self.set.get(&n).copied().unwrap_or(([0, 0, 0, 0x3f80_0000], [0, 0, 0, 0x3f80_0000]))
    }

    pub(crate) fn view(&self) -> CamView {
        let (pos, view) = self.get(self.id);
        let (p2, v2) = self.get(2);
        let b = &self.boss;
        CamView {
            pos: p2,
            view: v2,
            active: self.id,
            active_pos: pos,
            active_view: view,
            reset: b.reset,
            eye: b.pos,
            boss_view: b.view,
            // checkCameraType(): the type as last written.
            fade: self.fade.map(|f| piney_battle::boss::CamFade { eye_view: self.ty == 1, ..f }),
            ..CamView::default()
        }
    }

    pub(crate) fn apply(&mut self, out: &[Out]) {
        for o in out {
            let b = &mut self.boss;
            match o {
                Out::CameraChange(n) => self.id = *n,
                Out::CameraPos { cam, pos } => self.set.entry(*cam).or_insert(([0; 4], [0; 4])).0 = *pos,
                Out::CameraView { cam, view } => self.set.entry(*cam).or_insert(([0; 4], [0; 4])).1 = *view,
                Out::CameraType(t) => self.ty = *t,
                Out::CamMode { mode } => {
                    b.reset = *mode;
                    match mode {
                        3 => b.ex_lock |= 0x10,
                        4 => b.ex_lock &= 1,
                        5 => {
                            b.ex_lock |= 1;
                            b.lock = true;
                            (b.keep_view, b.keep_pos) = (b.view, b.pos);
                        }
                        6 => {
                            b.ex_lock &= 0x10;
                            b.lock = false;
                            (b.view, b.pos) = (b.keep_view, b.keep_pos);
                            self.set.insert(2, (b.pos, b.view));
                            b.reset = 0;
                        }
                        8 | 10 => b.reset = 0,
                        _ => {}
                    }
                }
                Out::FreeCam { pos, view } if b.reset == 5 => (b.pos, b.view) = (*pos, *view),
                Out::CamEye(p) => b.pos = *p,
                Out::CamLook(v) => b.view = *v,
                Out::CamLock(on) => b.lock = *on,
                _ => {}
            }
        }
    }
}

fn run(tables: &Tables, data: &BossData, clips: &Clips, t: &mut Toks) -> String {
    let n = t.int() as usize;
    let mut scene = Scene::default();
    for _ in 0..n {
        let c = read_char(t);
        scene.chars.push(c);
    }
    let npc = t.int() as usize;
    scene.pc_list = (0..npc).map(|_| t.int() as usize).collect();
    let members: [Option<usize>; 3] = std::array::from_fn(|_| usize::try_from(t.int()).ok());
    let ids: [i32; 3] = std::array::from_fn(|_| t.i32());
    let me = t.int() as usize;
    let center = [t.u32(), t.u32(), t.u32(), 0x3f80_0000];
    let kpos = v4(t);
    let kdirc = v4(t);
    let mut rand = Rand(t.int() as u64);
    let mut cc = Genrand::seeded(t.u32());
    cc.mti = t.i32();
    let count0 = t.u32();
    let frames = t.int() as usize;
    let hole = f32::from_bits(t.u32());
    let cam2 = (v4(t), v4(t));
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, usize, i16)> =
        (0..ns).map(|_| (t.int() as usize, t.i32(), t.int() as usize, t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    let mut blur = crate::boss::ProbeBlur::read(t);
    let fade = crate::boss::read_fade(t);
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
    crate::boss::row_size(&mut scene, me, tables);
    let party = Party { members, ids, num: members.iter().flatten().count() as i32 };
    let clip = |name: &str| clips.get(name).copied();
    let mut forbid = 0i16;
    let mut menu = [0i32; 7];
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let mut cams = Cams {
        set: [(2, cam2)].into_iter().collect(),
        id: 1,
        ty: fade.map_or(0, |f| i32::from(f.eye_view)),
        fade,
        ..Cams::default()
    };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let volume = crate::probe_volume();
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false, cc: None };
    let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
    let mut none = |_| None;
    let mut land = |p: V4| p[2];
    let mut plane = Plane { hole };
    let mut cx = Cx {
        t: tables,
        data,
        scene: &mut scene,
        party: &party,
        world: World { player: members[0], frame: &IDENTITY, ..World::default() },
        env: &env0,
        actx: &actx,
        rand: &mut rand,
        cc: &mut cc,
        clips: &clip,
        collide: &mut none,
        part_hits: &mut plane,
        game_over: false,
        boss_cam: false,
        cam: cams.view(),
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        gen_alive: &|_| true,
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = corbenik::first::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    cams.apply(&out);
    let outs: Vec<String> = out.iter().filter_map(mout).collect();
    blur.apply(&out);
    lines.push(with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, cams.ty), &blur));
    for f in 0..frames {
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        let seeds = seed_chars(&scene, me);
        let mut out = Vec::new();
        let cell = std::cell::RefCell::new(std::mem::take(&mut cc));
        let benv_hits = BossEnv { cc: Some(&cell), ..benv };
        let actx_hits = AffectCtx { boss: Some(&benv_hits), ..actx };
        for &(_, kind, who, p0) in script.iter().filter(|s| s.0 == f) {
            let actx = &actx_hits;
            let mut ev = Events::new();
            let kite = members[0];
            let on_seed = seeds[who % 8];
            let sid = (who / 8) as i16;
            match kind {
                0 => affect::entry_affect(tables, &mut scene, actx, me, kite, 1, [p0, 0, 0], &mut rand, &mut ev),
                5 | 6 => {
                    affect::entry_affect(tables, &mut scene, actx, me, kite, kind as i16, [0; 3], &mut rand, &mut ev)
                }
                7 => {
                    if let Some(m) = members.get(who).copied().flatten() {
                        scene.chars[m].cond[0] = p0;
                    }
                }
                8 => {
                    if let Some(b) = scene.chars[me].foe_state_mut().and_then(|x| x.boss.as_mut()) {
                        (b.base.act_num, b.base.act_proccess, b.base.act_count) = (p0, 0, 0);
                    }
                }
                9 => affect::entry_affect(tables, &mut scene, actx, me, kite, 3, [p0, 0, 0], &mut rand, &mut ev),
                10 => {
                    let p = [p0, who as i16, 0];
                    affect::entry_affect(tables, &mut scene, actx, me, kite, 1, p, &mut rand, &mut ev)
                }
                11 => {
                    affect::entry_affect(tables, &mut scene, actx, on_seed, kite, 1, [p0, sid, 0], &mut rand, &mut ev)
                }
                12 => {
                    affect::entry_affect(tables, &mut scene, actx, on_seed, kite, 3, [p0, sid, 0], &mut rand, &mut ev)
                }
                13 => {
                    scene.chars[me].skill_id = 0;
                    scene.chars[me].skill_status = 0;
                }
                _ => {}
            }
        }
        cc = cell.into_inner();
        for &s in std::iter::once(&me).chain(seeds.iter()) {
            if let Some(b) = scene.chars[s].foe_state_mut().and_then(|x| x.boss.as_mut()) {
                out.extend(b.base.take_pending());
            }
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut all = out;
        blur.apply(&all);
        blur.frame();
        let pre = all.len();
        let mut cx = Cx {
            t: tables,
            data,
            scene: &mut scene,
            party: &party,
            world: World { player: members[0], frame: &IDENTITY, ..World::default() },
            env: &env,
            actx: &actx,
            rand: &mut rand,
            cc: &mut cc,
            clips: &clip,
            collide: &mut none,
            part_hits: &mut plane,
            game_over: false,
            boss_cam: false,
            cam: cams.view(),
            land: &mut land,
            disc: piney_battle::boss::DiscView { blur_exit: blur.blur.exit, ..Default::default() },
            gen_alive: &|_| true,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.main(&mut cx);
        all.extend(std::mem::take(&mut cx.out));
        drop(cx);
        blur.apply(&all[pre..]);
        cams.apply(&all);
        for o in &all {
            match o {
                Out::MenuForbid { on, chat_except } => {
                    forbid = i16::from(*on);
                    menu[0] = i32::from(*on);
                    if *on {
                        if *chat_except {
                            menu[1] = 1;
                        }
                    } else {
                        menu[1] = 0;
                    }
                }
                Out::CursorOff(on) => menu[2] = i32::from(*on),
                Out::StreamMenu { stream, mask } => {
                    menu[3] = 0x104a;
                    menu[4] = *mask;
                    menu[5] = *stream;
                }
                Out::PanelStatus(p) => menu[6] = i32::from(*p),
                _ => {}
            }
        }
        let outs: Vec<String> = all.iter().filter_map(mout).collect();
        lines.push(with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, cams.ty), &blur));
    }
    format!("[{}]", lines.join(","))
}

/// The seeds' characters, by slot.
fn seed_chars(scene: &Scene, me: usize) -> [usize; 8] {
    match scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
        Some(Class::Corbenik(x)) => x.seed_me,
        _ => [0; 8],
    }
}

pub(crate) fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

/// A call as the harness records it.
pub(crate) fn mout(o: &Out) -> Option<String> {
    Some(match o {
        Out::Corbenik(p) => match p {
            Pic::Particles { which, .. } => format!("[\"gen\",\"{}\"]", gen_name(*which)),
            Pic::Charge { on: true, .. } => "[\"gen\",\"charge\"]".into(),
            Pic::Charge { on: false, .. } => "[\"gen_del\"]".into(),
            Pic::CountDown { n, .. } => format!("[\"count\",{n}]"),
            Pic::Burst { kind, .. } => format!("[\"burst\",{kind}]"),
            Pic::SeedShield { .. } => "[\"shield\",1]".into(),
            Pic::Gen { which, .. } => format!("[\"gen\",\"{}\"]", gen_name(*which)),
            Pic::Barrier
            | Pic::BarrierHit { .. }
            | Pic::PressImage { .. }
            | Pic::Element { .. }
            | Pic::TransformLay { .. }
            | Pic::RootL { .. }
            | Pic::Flush { .. } => return None,
            Pic::ElementSmoke { .. } | Pic::Smoke { .. } => "[\"smoke\"]".into(),
            Pic::LaserRadiate { .. } => "[\"radiate2\"]".into(),
            Pic::Star { .. }
            | Pic::StarCore { .. }
            | Pic::Thunder { .. }
            | Pic::Laser { .. }
            | Pic::LaserHead { .. } => return None,
            Pic::Radiate { .. } => "[\"radiate\"]".into(),
            Pic::Eye { .. }
            | Pic::Killer { .. }
            | Pic::TeraBeam { .. }
            | Pic::Spark { .. }
            | Pic::RefrectHead { .. }
            | Pic::RefrectSeg { .. }
            | Pic::GenMove { .. } => return None,
            Pic::KillerShield { magic, .. } => format!("[\"shield\",{magic}]"),
            Pic::GenKill { .. } => "[\"gen_kill\"]".into(),
            Pic::ArenaClump { on } => format!("[\"clump\",{}]", u8::from(*on)),
            Pic::Explode2 { .. } => "[\"explode2\"]".into(),
        },
        Out::AfterImageAnm { .. } => "[\"afterimage_anm\"]".into(),
        Out::EntryFade { t, from, to } => format!("[\"fade\",{t},{from},{to}]"),
        Out::CameraChange(n) => format!("[\"cam\",{n}]"),
        Out::CameraPos { cam, pos } => format!("[\"campos\",{cam},{}]", v3(pos)),
        Out::CameraView { cam, view } => format!("[\"camview\",{cam},{}]", v3(view)),
        Out::CinemaSkill(sid) => format!("[\"cinema_skill\",{sid}]"),
        Out::CamMode { mode } => format!("[\"cammode\",{mode}]"),
        Out::SeNote { se, note } => format!("[\"senote\",{se},{note}]"),
        Out::FreeCam { pos, view } => format!("[\"freecam\",{},{}]", v3(pos), v3(view)),
        Out::AfterImageWith(p) => format!("[\"afterimage\",{},{}]", p.color, p.trans_spd),
        Out::CamEye(_) | Out::CamLook(_) | Out::CamLock(_) | Out::CameraRot { .. } => return None,
        Out::FlashFade { t_in, t_out, hold, colour } => {
            format!("[\"flash3\",{t_in},{t_out},{},{colour}]", hold.unwrap_or(-1))
        }
        // Corbenik's own effects run natively in the harness: seen in the slots.
        Out::Effect {
            kind:
                EffKind::CorbenikWave
                | EffKind::Flush
                | EffKind::StarDustFlash
                | EffKind::Transform
                | EffKind::RootDrain
                | EffKind::Thunder3
                | EffKind::TeraLaser
                | EffKind::RefrectionLaser,
            ..
        } => return None,
        Out::Skill(_, s) => format!("[\"skill\",{},{}]", s.sid, u8::from(s.stype == 2)),
        _ => return out_json(o),
    })
}

fn gen_name(g: Gen) -> &'static str {
    match g {
        Gen::WaveRing => "ring",
        Gen::WaveSquare => "square",
        Gen::SeedShot => "smoke",
        Gen::RootDrain0 => "drain0",
        Gen::RootDrain1a => "drain1a",
        Gen::RootDrain1b => "drain1b",
        Gen::RootDrain2 => "drain2",
        Gen::Transform => "transform",
        Gen::GenerateEye => "generate_eye",
        Gen::BossDead0 => "dead0",
        Gen::BossDead1a => "dead1a",
        Gen::BossDead1b => "dead1b",
        Gen::LaserCharge => "laser_charge",
        Gen::TeraCharge => "tera_charge",
        Gen::TeraChargeGlow => "tera_glow",
        Gen::TeraFall => "tera_fall",
    }
}

pub(crate) fn some(id: Option<i32>) -> String {
    u8::from(id.is_some_and(|k| k >= 0)).to_string()
}

fn first_json(b: &Boss, x: &First) -> String {
    let j = &x.jump;
    let v: Vec<String> = vec![
        some(x.boss_eff),
        x.stage_eff.to_string(),
        x.dmg_count.to_string(),
        x.pat_mode.to_string(),
        u8::from(x.dead_flag).to_string(),
        u8::from(x.pause_cnt_down).to_string(),
        x.entry_seed_num.to_string(),
        v3(&j.sp),
        v3(&j.ep),
        v3(&j.cp),
        j.time.to_string(),
        x.act_pos[2].to_string(),
        b.base.stop_time.to_string(),
        b.base.stop_counter.to_string(),
        b.base.pat_index.to_string(),
        b.base.pat_num.to_string(),
        b.base.m_act_dirc[2].to_string(),
    ];
    v.join(",")
}

fn seed_json(scene: &Scene, s: usize) -> String {
    let ch = &scene.chars[s];
    let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()) else { return "[]".into() };
    let Class::CorbenikSeed(x) = &b.class else { return "[]".into() };
    let x: &Seed = x;
    format!(
        "[{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{},{},{},{},{},{},{},{},{}]",
        b.base.act_num,
        b.base.act_proccess,
        b.base.act_count,
        v3(&ch.pos),
        v3(&ch.pos_p),
        v3(&b.base.dirc),
        v3(&b.base.move_vector),
        ch.hp,
        ch.max_hp,
        b.base.act_forbid,
        b.base.anm_status,
        b.base.exit,
        b.base.draw_sw,
        b.base.body_hit_sw,
        b.base.erase_target,
        u8::from(scene.listed(s)),
        ch.cond[0],
        b.base.transparency,
        b.base.set_transparency,
        b.base.anm.clip.as_deref().unwrap_or(""),
        b.base.anm.frame(),
        x.exp_count,
        x.blink_time,
        x.rot_speed[0],
        x.rot_speed[1],
        u8::from(x.range_out),
        u8::from(x.explode),
        u8::from(x.charge),
        some(x.eff_flush),
    )
}

#[allow(clippy::too_many_arguments)]
fn frame_json(
    b: &Boss,
    scene: &Scene,
    me: usize,
    outs: &[String],
    rand: &Rand,
    cc: &Genrand,
    menu: &[i32; 7],
    cam_type: i32,
) -> String {
    let Class::Corbenik(x) = &b.class else { return "{}".into() };
    let ch = &scene.chars[me];
    let effs: Vec<String> = b
        .base
        .effects
        .slots
        .iter()
        .enumerate()
        .filter_map(|(k, s)| s.as_ref().map(|e| format!("[{k},{},{}]", eff_num(e.kind), u8::from(e.enabled))))
        .collect();
    let party: Vec<String> = scene
        .pc_list
        .iter()
        .map(|&c| format!("[{},{},{}]", scene.chars[c].hp, scene.chars[c].cond[1], scene.chars[c].cond[0]))
        .collect();
    let seeds: Vec<String> = x.seed_me.iter().map(|&s| seed_json(scene, s)).collect();
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{}],\"first\":[{}],\"seeds\":[{}],\"cam\":[{}],\"eff\":[{}],\"party\":[{}],\"out\":[{}],\
         \"menu\":[{},{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
        b.base.act_num,
        b.base.act_proccess,
        b.base.act_count,
        b.base.move_spd,
        b.base.move_dirc,
        v3(&b.base.move_vector),
        ch.target_char.map_or(-1, |t| t as i64),
        b.base.target_dist,
        b.base.target_dirc,
        v3(&b.base.target_pos),
        v3(&ch.pos),
        v3(&ch.pos_p),
        v3(&b.base.dirc),
        ch.hp,
        ch.max_hp,
        b.base.act_forbid,
        b.base.anm_status,
        b.base.stop,
        b.base.cheat_hp,
        b.base.exit,
        b.base.draw_sw,
        b.base.body_hit_sw,
        b.base.lock_player,
        b.base.erase_target,
        b.base.reserve_forbid_menu,
        u8::from(b.base.reserve_forbid_chat_except != 0),
        b.base.stop_count,
        u8::from(scene.listed(me)),
        b.base.transparency,
        b.base.set_transparency,
        b.base.anm.clip.as_deref().unwrap_or(""),
        b.base.anm.frame(),
        b.base.anm.speed,
        first_json(b, x),
        seeds.join(","),
        cam_type,
        effs.join(","),
        party.join(","),
        outs.join(","),
        menu[0],
        menu[1],
        menu[2],
        menu[3],
        menu[4],
        menu[5],
        menu[6],
        rand.0,
        cc.mti,
    )
}
