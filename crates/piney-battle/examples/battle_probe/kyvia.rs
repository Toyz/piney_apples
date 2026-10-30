//! `battle_probe` requests for Kyvia (`piney_battle::boss::kyvia`), as
//! `tools/test_battle_kyvia_rs.py` sends them: `kyviaclips F` (the clips of
//! fight F's file, `x01` or `x02`, `xeffect`, `particle`) and `kyvia F ...`:
//! the body made on a scene, then FRAMES frames of the manager's pass and
//! `Main`, the disc and the camera scripted (the disc rides RIDE2 frames
//! after each `Move`), affects before each (0 a hit on the core, 1 on
//! gomora P1, 2 a heal on the core). One JSON line: the state after the
//! constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::kyvia::core::Core;
use piney_battle::boss::kyvia::gomora::Gomora;
use piney_battle::boss::kyvia::{Fight, Follow, Gen, GenOp, Held, Kyvia, Place};
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, DiscView, EffKind, FadeView, Out, PartHits};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::{Event, Events, Who};
use piney_battle::exp::Party;
use piney_battle::param::cond;
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{CharHit, char_push};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::field::ee;
use piney_data::iso::Iso;
use std::cell::RefCell;

use crate::boss::{eff_num, out_json};
use crate::{Toks, read_char};

type Clips = std::collections::HashMap<String, (u32, bool)>;
type V4 = [u32; 4];

thread_local! {
    static DATA: std::cell::RefCell<Option<(Fight, BossData, Clips)>> = const { std::cell::RefCell::new(None) };
}

fn load(iso_path: &str, fight: Fight) -> (Fight, BossData, Clips) {
    let mut iso = Iso::open(iso_path).expect("the ISO");
    let data = BossData::of(iso.volume().expect("the volume"));
    let arc = Archive::new(iso.read_path("DATA/DATA.BIN").expect("DATA.BIN")).expect("DATA.BIN");
    let mut clips = std::collections::HashMap::new();
    for stem in [fight.file(), "xeffect", "particle"] {
        let Ok(raw) = arc.inflate_named(stem) else { continue };
        let ccs = Ccs::parse(raw).expect("a CCSF");
        let scene = piney_data::scene::Scene::read(&ccs).expect("its scene");
        for a in piney_data::anim::Animation::all(&ccs, &scene).expect("its animations") {
            if let Some(name) = ccs.object_name(a.object) {
                clips.insert(name.to_string(), (a.frames, a.looping));
            }
        }
    }
    (fight, data, clips)
}

/// Answers `kyviaclips` and `kyvia`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "kyviaclips" | "kyvia") {
        return None;
    }
    let fight = Fight::of_code(t.i32() + 11).expect("fight 1 or 2");
    DATA.with(|d| {
        if d.borrow().as_ref().is_none_or(|(f, ..)| *f != fight) {
            *d.borrow_mut() = Some(load(iso, fight));
        }
    });
    Some(DATA.with(|d| {
        let d = d.borrow();
        let (fight, data, clips) = d.as_ref().unwrap();
        if cmd == "kyviaclips" {
            let mut v: Vec<_> = clips.iter().collect();
            v.sort();
            let body: Vec<String> = v.iter().map(|(k, (f, l))| format!("\"{k}\":[{f},{}]", u8::from(*l))).collect();
            format!("{{{}}}", body.join(","))
        } else {
            run(*fight, tables, data, clips, t)
        }
    }))
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

/// The boss camera as the game's zeroed `ccBossCam` holds it under the
/// rules' `SetMode`, `SetFreeCamPosView` and pitch (CamMain never runs):
/// the harness clears a mode 3 or 4 (`ResetFlg`) `delay` frames after it
/// was set, standing in for `SetTransfer`'s ease.
#[derive(Default)]
struct FakeCam {
    reset: i32,
    lock: u8,
    view: V4,
    pos: V4,
    temp_view: V4,
    temp_pos: V4,
    xrot: u32,
    left: i32,
    /// +0x138 `ExLock` (modes 3 and 5 set bits 4 and 0, 4 and 6 clear
    /// them) and +0x139 `ZrotControlFlg`.
    ex_lock: u8,
    sway: u8,
}

impl FakeCam {
    fn take(&mut self, o: &Out) {
        match o {
            Out::CamMode { mode } | Out::CamModeRange { mode, .. } => {
                self.reset = *mode;
                match mode {
                    3 => self.ex_lock |= 0x10,
                    4 => self.ex_lock &= 1,
                    5 => {
                        self.ex_lock |= 1;
                        self.lock = 1;
                        self.temp_view = self.view;
                        self.temp_pos = self.pos;
                    }
                    6 => {
                        self.ex_lock &= 0x10;
                        self.lock = 0;
                        self.view = self.temp_view;
                        self.pos = self.temp_pos;
                        self.reset = 0;
                    }
                    _ => {}
                }
            }
            Out::FreeCam { pos, view } if self.reset == 5 => {
                self.view = *view;
                self.pos = *pos;
            }
            Out::CamEye(pos) => self.pos = *pos,
            Out::CamPitch { add, v } => self.xrot = if *add { ee::add(self.xrot, *v) } else { *v },
            Out::CamSway(on) => self.sway = u8::from(*on),
            _ => {}
        }
    }

    /// The harness's step before a frame: a mode 3 or 4 counts down and
    /// clears.
    fn step(&mut self, delay: i32) {
        if matches!(self.reset, 3 | 4) {
            if self.left == 0 {
                self.left = delay;
            }
            self.left -= 1;
            if self.left == 0 {
                self.reset = 0;
            }
        } else {
            self.left = 0;
        }
    }
}

/// `scFadeDef`'s four elements as the harness steps them (main fade.cpp:
/// `EntryFlash`, `EntryFlash2`, `EntryFlash3` the first free one, each
/// frame `SendPacket`'s count).
#[derive(Default)]
struct FakeFade {
    elm: [[i16; 5]; 4],
}

impl FakeFade {
    fn entry(&mut self, e: [i16; 5]) {
        if let Some(s) = self.elm.iter_mut().find(|s| s[0] == 0) {
            *s = e;
        }
    }

    fn take(&mut self, o: &Out) {
        match *o {
            Out::Flash { t, .. } => self.entry([3, 0, t as i16, 0, 0]),
            Out::FlashFade { t_in, t_out, hold: None, .. } => self.entry([5, 0, t_in as i16, t_out as i16, 0]),
            Out::FlashFade { t_in, t_out, hold: Some(h), .. } => {
                self.entry([9, 0, t_in as i16, t_out as i16, h as i16]);
            }
            _ => {}
        }
    }

    fn step(&mut self) {
        for e in self.elm.iter_mut().filter(|e| e[0] & 1 != 0) {
            e[1] += 1;
            if e[1] > e[2] {
                if e[0] & 2 != 0 {
                    e[0] = 0;
                } else if e[0] & 4 != 0 {
                    e[0] = (e[0] & !4) | 2;
                    e[1] = 0;
                    e[2] = e[3];
                } else if e[0] & 8 != 0 {
                    e[0] = (e[0] & !8) | 4;
                    e[1] = 0;
                    e[2] = e[4];
                } else {
                    e[1] = e[2];
                }
            }
        }
    }

    fn view(&self) -> [FadeView; 4] {
        self.elm.map(|e| FadeView { status: e[0], cnt: e[1], tcnt: e[2] })
    }
}

/// The collision list (`ccCharHitTop`) as the harness runs it natively:
/// the body's hit from its constructor, then each gomora's as it enables
/// it, in order; no ground (`ccModelHitCheckQ` finds nothing there).
struct HitList {
    volume: piney_data::volume::Volume,
    chars: Vec<(usize, CharHit)>,
}

impl HitList {
    fn sync(&mut self, who: usize, hit: &CharHit) {
        if let Some(e) = self.chars.iter_mut().find(|(w, _)| *w == who) {
            e.1 = *hit;
        }
    }

    fn push_of(&self, who: usize, hit: &CharHit) -> Option<(V4, u32)> {
        let others = self.chars.iter().filter(|(w, _)| *w != who).map(|(_, h)| (h.pos, h.radius, h.kind));
        char_push(self.volume, hit.pos, hit.radius, hit.mask, others)
    }
}

struct Shared<'l>(&'l RefCell<HitList>);

impl PartHits for Shared<'_> {
    fn switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        let mut l = self.0.borrow_mut();
        if on && !hit.sw {
            hit.sw = true;
            l.chars.push((who, *hit));
        } else if !on && hit.sw {
            hit.sw = false;
            l.chars.retain(|(w, _)| *w != who);
        }
    }

    fn collide(&mut self, who: usize, hit: &mut CharHit) -> bool {
        hit.offset = [0; 4];
        let mut l = self.0.borrow_mut();
        l.sync(who, hit);
        let push = if hit.sw { l.push_of(who, hit) } else { None };
        push.map(|(off, _)| hit.offset = off).is_some()
    }

    fn sync(&mut self, who: usize, hit: &CharHit) {
        self.0.borrow_mut().sync(who, hit);
    }
}

fn ee_vadd(a: V4, b: V4) -> V4 {
    std::array::from_fn(|k| ee::add(a[k], b[k]))
}

struct Case {
    frames: usize,
    rot: Vec<u32>,
    moving: Vec<bool>,
    prev: Vec<V4>,
    marker: V4,
    cam_pos: V4,
    cam_view: V4,
    delay: i32,
}

fn run(fight: Fight, tables: &Tables, data: &BossData, clips: &Clips, t: &mut Toks) -> String {
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
    let marker = [t.u32(), t.u32(), t.u32(), 0x3f80_0000];
    let mut rand = Rand(t.int() as u64);
    let mut cc = Genrand::seeded(t.u32());
    cc.mti = t.i32();
    let count0 = t.u32();
    let frames = t.int() as usize;
    let ride2 = t.i32();
    let cam_pos = v4(t);
    let cam_view = v4(t);
    let delay = t.i32();
    let rot: Vec<u32> = (0..frames).map(|_| t.u32()).collect();
    let moving: Vec<bool> = (0..frames).map(|_| t.int() != 0).collect();
    let prev: Vec<V4> = (0..frames).map(|_| [t.u32(), t.u32(), t.u32(), 0x3f80_0000]).collect();
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, usize, i16, i16)> =
        (0..ns).map(|_| (t.int() as usize, t.i32(), t.int() as usize, t.i16(), t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    // The fourth fight's: Kite set down at a place on a frame.
    let walks: Vec<(usize, V4)> = if fight == Fight::Fourth {
        let n = t.int() as usize;
        (0..n).map(|_| (t.int() as usize, [t.u32(), t.u32(), t.u32(), 0x3f80_0000])).collect()
    } else {
        Vec::new()
    };
    let mut blur = crate::boss::ProbeBlur::read(t);
    let cam_fade = crate::boss::read_fade(t);
    let mut fade = FakeFade::default();
    let case = Case { frames, rot, moving, prev, marker, cam_pos, cam_view, delay };
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
    crate::boss::row_size(&mut scene, me, tables);
    let party = Party { members, ids, num: members.iter().flatten().count() as i32 };
    let clip = |name: &str| clips.get(name).copied();
    let mut forbid = 0i16;
    let mut menu = [0i32; 7];
    let mut cam = FakeCam::default();
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let view = |f: usize, reset: i32, eye: V4| CamView {
        rot: [0, 0, case.rot[f.min(case.frames.saturating_sub(1))], 0],
        pos: case.cam_pos,
        view: case.cam_view,
        moving: false,
        reset,
        shake: false,
        eye,
        active: 2,
        active_pos: case.cam_pos,
        active_view: case.cam_view,
        boss_view: [0; 4],
        fade: cam_fade,
    };
    // The frames the disc rides on after the body's `Move`.
    let mut ride = 0;
    let disc = |f: usize, ride: i32, blur: &crate::boss::ProbeBlur, fade: &FakeFade| DiscView {
        moving: case.moving.get(f).copied().unwrap_or(false) || ride > 0,
        prev_pos: case.prev.get(f).copied().unwrap_or(case.marker),
        marker: case.marker,
        blur_abgr: blur.blur.abgr,
        blur_scale: blur.blur.scale,
        blur_exit: blur.blur.exit,
        fades: fade.view(),
    };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false, cc: None };
    let actx =
        AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume: crate::probe_volume() };
    let list = RefCell::new(HitList { volume: crate::probe_volume(), chars: Vec::new() });
    let mut none = |_| None;
    let mut land = |p: V4| p[2];
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
        part_hits: &mut Shared(&list),
        game_over: false,
        boss_cam: true,
        cam: view(0, 0, [0; 4]),
        land: &mut land,
        disc: disc(0, 0, &blur, &fade),
        gen_alive: &|_| true,
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = piney_battle::boss::kyvia::new(&mut cx, fight);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    // The constructor's SetHitSW: the body first in the list.
    let body = CharHit {
        sw: true,
        kind: scene.chars[me].ty() as u32,
        radius: piney_battle::boss::kyvia::BODY_RADIUS,
        height: scene.chars[me].base().height,
        pos: scene.chars[me].pos,
        ..CharHit::default()
    };
    if boss.body_hit_sw != 0 {
        list.borrow_mut().chars.push((me, body));
    }
    // ccBoss::Move's CollisionDetection of the body; after a push its hit
    // stands where the body was put (the push's height dropped).
    let mut body_push = |pos: V4| {
        let mut l = list.borrow_mut();
        let mut h = l.chars.iter().find(|(w, _)| *w == me)?.1;
        h.pos = pos;
        l.sync(me, &h);
        let (off, _) = l.push_of(me, &h)?;
        h.pos = ee_vadd(pos, [off[0], off[1], 0, off[3]]);
        l.sync(me, &h);
        Some(off)
    };
    HELD.with(|s| s.borrow_mut().clear());
    PAUSED.with(|s| s.borrow_mut().clear());
    ROLES.with(|r| *r.borrow_mut() = roles(&boss, &scene));
    for o in &out {
        cam.take(o);
        fade.take(o);
    }
    blur.apply(&out);
    let outs: Vec<String> = out.iter().filter_map(kout).collect();
    lines.push(with_ball(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, &cam, &blur.json()), &boss));
    for f in 0..frames {
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx = AffectCtx {
            party: &party,
            menu: true,
            skill_check: &check,
            boss: Some(&benv),
            volume: crate::probe_volume(),
        };
        cam.step(case.delay);
        ride = (ride - 1).max(0);
        for &(_, p) in walks.iter().filter(|w| w.0 == f) {
            if let Some(k) = members[0] {
                scene.chars[k].pos = p;
            }
        }
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        let (core, gomoras) = parts(&scene, me);
        let mut outs = Vec::new();
        let mut out = Vec::new();
        for &(_, kind, who, p0, p1) in script.iter().filter(|s| s.0 == f) {
            let on = match kind {
                1 => gomoras.get(who).copied(),
                _ => core,
            };
            let Some(on) = on else { continue };
            let alive = scene.chars[on].cond[cond::DEAD] == 0
                && scene.chars[on].foe_state().and_then(|x| x.boss.as_ref()).is_some_and(|b| b.exit == 0);
            if !alive {
                continue;
            }
            let mut ev = Events::new();
            let kite = members[0];
            let ty = if kind == 2 { 7 } else { 1 };
            affect::entry_affect(tables, &mut scene, &actx, on, kite, ty, [p0, p1, 0], &mut rand, &mut ev);
            if let Some(b) = scene.chars[on].foe_state_mut().and_then(|x| x.boss.as_mut()) {
                let p = b.take_pending();
                outs.extend(p.iter().filter_map(kout));
                out.extend(p);
            }
            for e in ev {
                if let Event::ResistantShield { on: Who::Char(_), magic } = e {
                    outs.push(format!("[\"shield\",{magic}]"));
                }
            }
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let pending = boss.take_pending();
        outs.extend(pending.iter().filter_map(kout));
        out.extend(pending);
        // The affects' writes, then the manager's pass (the effects, then
        // the blur's Main), then the boss's.
        blur.apply(&out);
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
            collide: &mut body_push,
            part_hits: &mut Shared(&list),
            game_over: false,
            boss_cam: true,
            cam: view(f, cam.reset, cam.pos),
            land: &mut land,
            disc: disc(f, ride, &blur, &fade),
            gen_alive: &|_| true,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.effects_pass(&mut cx);
        let fx_out = std::mem::take(&mut cx.out);
        blur.apply(&fx_out);
        blur.frame();
        // The effects' camera modes before the boss reads ResetFlg.
        for o in fx_out.iter().filter(|o| matches!(o, Out::CamMode { .. } | Out::CamModeRange { .. })) {
            cam.take(o);
        }
        for o in &fx_out {
            fade.take(o);
        }
        cx.cam = view(f, cam.reset, cam.pos);
        cx.disc.blur_abgr = blur.blur.abgr;
        cx.disc.blur_scale = blur.blur.scale;
        cx.disc.fades = fade.view();
        boss.main(&mut cx);
        let main_out = std::mem::take(&mut cx.out);
        drop(cx);
        outs.extend(fx_out.iter().chain(&main_out).filter_map(kout));
        let fx_at = out.len()..out.len() + fx_out.len();
        out.extend(fx_out);
        let pre = out.len();
        out.extend(main_out);
        let early =
            |k: usize, o: &Out| fx_at.contains(&k) && matches!(o, Out::CamMode { .. } | Out::CamModeRange { .. });
        for o in &out[pre..] {
            fade.take(o);
        }
        for (_, o) in out.iter().enumerate().filter(|&(k, o)| !early(k, o)) {
            cam.take(o);
            match o {
                Out::DummyTarget(t) => menu[6] = t.map_or(0, |_| 1),
                Out::DiscNextStage => ride = ride2,
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
                _ => {}
            }
        }
        blur.apply(&out[pre..]);
        fade.step();
        lines.push(with_ball(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, &cam, &blur.json()), &boss));
    }
    format!("[{}]", lines.join(","))
}

/// The core's and the gomoras' characters, from the body.
fn roles(b: &Boss, scene: &Scene) -> (Option<usize>, Vec<usize>) {
    let Class::Kyvia(x) = &b.class else { return (None, Vec::new()) };
    let gs = match scene.chars[x.core].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
        Some(Class::KyviaCore(c)) => c.gomoras.clone(),
        _ => Vec::new(),
    };
    (Some(x.core), gs)
}

/// The core's and the gomoras' characters.
fn parts(scene: &Scene, me: usize) -> (Option<usize>, Vec<usize>) {
    let Some(b) = scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()) else { return (None, Vec::new()) };
    let Class::Kyvia(x) = &b.class else { return (None, Vec::new()) };
    let core = x.core;
    let gs = match scene.chars[core].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
        Some(Class::KyviaCore(c)) => c.gomoras.clone(),
        _ => Vec::new(),
    };
    (Some(core), gs)
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

thread_local! {
    /// The core's and the gomoras' characters, and the generators the
    /// fight holds (as the game's pointers: kills print only for these).
    static ROLES: std::cell::RefCell<(Option<usize>, Vec<usize>)> = const { std::cell::RefCell::new((None, Vec::new())) };
    static HELD: std::cell::RefCell<std::collections::HashSet<Held>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    static PAUSED: std::cell::RefCell<std::collections::HashSet<Held>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

/// Who a character is in the fight: the core or gomora `k`.
fn role(c: usize) -> String {
    ROLES.with(|r| {
        let r = r.borrow();
        match r.1.iter().position(|&g| g == c) {
            Some(k) => format!("[\"gomora\",{k}]"),
            None if r.0 == Some(c) => "\"core\"".into(),
            None => "\"body\"".into(),
        }
    })
}

fn held_json(h: Held) -> String {
    match h {
        Held::Light => "\"light\"".into(),
        Held::BeamHead(k) => format!("[\"head\",{k}]"),
        Held::BeamTail(k) => format!("[\"tail\",{k}]"),
        Held::Smoke(k) => format!("[\"smoke\",{k}]"),
        Held::CoreAura(c, k) => format!("[{},\"aura\",{k}]", role(c)),
        Held::GomoraAura(c) => format!("[{},\"aura\"]", role(c)),
        Held::Wall(k) => format!("[\"wall\",{k}]"),
        Held::Wall2(k) => format!("[\"wall2\",{k}]"),
        Held::CoreApp(k) => format!("[\"coreapp\",{k}]"),
        Held::CoreApp2(k) => format!("[\"coreapp2\",{k}]"),
        Held::Soul(k, j) => format!("[\"soul\",{k},{j}]"),
        Held::LastSoul => "\"lastsoul\"".into(),
    }
}

/// A start as the harness reads it: the table and row (a gomora aura's
/// level), then, but for the missiles' own, the force fields' rows,
/// `distSW`, where it stands and who holds it.
fn gen_json(which: Gen, at: &Place, held: Option<Held>) -> String {
    let r = which.rows();
    let head = match which {
        Gen::GomoraAura(attr, lv) => format!("\"particles\",\"GomoraAura\",{attr},{lv}"),
        Gen::GomoraAttack(attr) => format!("\"particles\",\"GomoraAura\",{},0", 4 + attr),
        _ => format!("\"particles\",\"{}\",{}", r.table.name(), r.row),
    };
    if matches!(which, Gen::Trail(_) | Gen::Burst(_) | Gen::Ball(_) | Gen::Hell(_))
        || matches!(which, Gen::Fourth(piney_battle::boss::kyvia::fourth::Make::Skill))
    {
        return format!("[{head}]");
    }
    let ff: Vec<String> = r.ff.iter().map(|f| f.to_string()).collect();
    let place = match *at {
        Place::At(p) => format!("[\"at\",{}]", v3(&p)),
        Place::Follows(f, off) => {
            let f = match f {
                Follow::Char(c) => format!("[{},\"pos\"]", role(c)),
                Follow::DeadSmoke => "\"dead\"".into(),
                Follow::BeamHead(k) => format!("[\"head\",{k}]"),
                Follow::BeamTail(k) => format!("[\"tail\",{k}]"),
                Follow::Aura(c) => format!("[{},\"aura\"]", role(c)),
                Follow::Plasma(k) => format!("[\"plasma\",{k}]"),
                Follow::Plasma2(k) => format!("[\"plasma2\",{k}]"),
                Follow::Soul(k) => format!("[\"soul\",{k}]"),
            };
            format!("[\"follow\",{f},{}]", v3(&off))
        }
        Place::Line(p, p2) => format!("[\"line\",{},{}]", v3(&p), v3(&p2)),
    };
    let held = held.map_or("null".into(), |h| {
        HELD.with(|s| s.borrow_mut().insert(h));
        held_json(h)
    });
    format!("[{head},[{}],{},{place},{held}]", ff.join(","), u8::from(r.dist_sw))
}

/// A kept generator killed (printed when the fight holds one there), or
/// let go.
fn gen_op_json(held: Held, op: GenOp) -> Option<String> {
    match op {
        GenOp::Kill => HELD.with(|s| s.borrow().contains(&held)).then(|| format!("[\"kill\",{}]", held_json(held))),
        GenOp::Forget => {
            HELD.with(|s| s.borrow_mut().remove(&held));
            None
        }
        GenOp::Pause(on) => {
            PAUSED.with(|s| {
                let mut s = s.borrow_mut();
                if on {
                    s.insert(held);
                } else {
                    s.remove(&held);
                }
            });
            None
        }
    }
}

/// Whether a held generator is paused (`pauseFlag`, +5 bit 0).
fn paused(h: Held) -> String {
    let on = HELD.with(|s| s.borrow().contains(&h)) && PAUSED.with(|s| s.borrow().contains(&h));
    u8::from(on).to_string()
}

fn kout(o: &Out) -> Option<String> {
    Some(match o {
        Out::KyviaParticles { which, at, held } => gen_json(*which, at, *held),
        Out::KyviaGen { held, op } => return gen_op_json(*held, *op),
        Out::SkillFrom { skill, .. } => format!("[\"skill\",{}]", skill.sid),
        Out::SkillStart { sid, .. } => format!("[\"skill_start\",{sid}]"),
        Out::DiscNextStage => "[\"disc_next\"]".into(),
        Out::SeNote { se, note } => format!("[\"senote\",{se},{note}]"),
        Out::SmokeRock { .. } => "[\"smoke_rock\"]".into(),
        Out::MusicFade { t } => format!("[\"music_fade\",{t}]"),
        // The meteors run natively in the harness: their making is seen in
        // the effects' slots.
        Out::Effect {
            kind: EffKind::Meteorite { .. } | EffKind::Thunderbolt { .. } | EffKind::MagicalBall | EffKind::HellExpress,
            ..
        } => {
            return None;
        }
        Out::FieldBodyHit { n, on } => format!("[\"gate\",{n},{}]", u8::from(*on)),
        Out::CamModeRange { .. } | Out::CamInitLock | Out::CamRotXLimit(_) | Out::CamSway(_) => return None,
        // DrawParts(anmw, 0) at the arm's frame (its place is tesPOS).
        Out::DrawArm { time, .. } => format!("[\"draw_parts\",{},0]", time >> 8),
        Out::AfterImageAnm { frame, pos, dirc, params: e } => format!(
            "[\"afterimage_anm\",{frame},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]",
            v3(pos),
            v3(dirc),
            e.color,
            e.color_alpha,
            e.color_spd,
            e.max_frame,
            e.transparency,
            e.trans_spd,
            e.ss[0],
            e.ss[1],
            e.ss[2],
            e.ss[3],
            e.es[0],
            e.es[1],
            e.es[2],
            e.es[3],
            e.scale_time
        ),
        _ => return out_json(o),
    })
}

fn boss_json(b: &Boss, scene: &Scene, who: usize) -> String {
    let ch = &scene.chars[who];
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{},{},{}",
        b.exit,
        b.draw_sw,
        b.act_num,
        b.act_proccess,
        b.act_count,
        v3(&ch.pos),
        v3(&b.dirc),
        ch.hp,
        ch.max_hp,
        ch.target_char.map_or(-1, |t| t as i64),
        b.act_forbid,
        b.stop,
        b.cheat_hp,
        b.erase_target,
        u8::from(scene.listed(who)),
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        b.transparency,
        b.set_transparency,
    )
}

fn core_json(c: &Core) -> String {
    let v: Vec<String> = vec![
        c.attr.to_string(),
        c.no.to_string(),
        c.end_flg.to_string(),
        c.core_state.to_string(),
        c.temp_state.to_string(),
        c.dead_flg.to_string(),
        c.dead_count.to_string(),
        c.k_atk_flg.to_string(),
        c.k_dmg_flg.to_string(),
        c.kyvia_lv.to_string(),
        c.cone_voice_cou.to_string(),
        c.now_gomora_num.to_string(),
        c.gomora_count.to_string(),
        c.gs_count.to_string(),
        c.gomora_hold.to_string(),
        c.gomora_meter.to_string(),
        c.gomora_lock.to_string(),
        c.time.to_string(),
        c.time_count.to_string(),
        c.time_flg.to_string(),
        c.time_mode.to_string(),
        c.temp_time.to_string(),
        v3(&c.sp_vec[0]),
        v3(&c.sp_vec[1]),
        v3(&c.sp_vec[2]),
        v3(&c.sp_vec[3]),
        v3(&c.dead_pos),
        c.st_flg.to_string(),
        c.alpha.to_string(),
        v3(&c.disc_pos),
        c.escape_cou.to_string(),
        c.dmg_count.to_string(),
        c.dmg_wait.to_string(),
        c.now_dmg_time.to_string(),
        c.dmg_time.to_string(),
        c.think_proccess.to_string(),
        c.af_cou.to_string(),
        c.fade_flg.to_string(),
        u8::from(c.aura).to_string(),
        u8::from(c.aura2).to_string(),
        // The auras' pauseFlag, as the game's generators carry it.
        u8::from(c.aura && c.aura_stop).to_string(),
        u8::from(c.aura2 && c.aura2_stop).to_string(),
        // CoreInit's stream (+0xcc): its level's (level 5 is the EX turn's
        // row, set after).
        format!("\"{}\"", piney_battle::boss::kyvia::stream(c.kyvia_lv.min(4)).unwrap_or("")),
    ];
    v.join(",")
}

/// Whether the fight holds a generator there (the game's pointer set).
fn holds(h: Held) -> String {
    u8::from(HELD.with(|s| s.borrow().contains(&h))).to_string()
}

fn gomora_json(g: &Gomora, me: usize) -> String {
    let mut v: Vec<String> = vec![
        g.life_flg.to_string(),
        g.eny_int.to_string(),
        g.eny_float.to_string(),
        g.atk_flg.to_string(),
        g.tmp_atk_flg.to_string(),
        g.my_attribute.to_string(),
        g.aura_lv.to_string(),
        v3(&g.aura_pos),
        g.ripus_num.to_string(),
        g.af_cou.to_string(),
        g.atk_wait.to_string(),
        g.atk_time.to_string(),
        g.atk_tmp_flg.to_string(),
        v3(&g.atk_tar_pos),
        g.dp_cou.to_string(),
        g.stop_move_count.to_string(),
        g.time.to_string(),
        g.time_count.to_string(),
        g.time_mode.to_string(),
        g.time_flg.to_string(),
        g.fade_flg.to_string(),
        g.now_list_num.to_string(),
        g.kyvia_step.to_string(),
    ];
    v.extend(g.sp_vec.iter().map(v3));
    v.extend(g.atk_vec.iter().map(v3));
    v.extend(g.dp_vec.iter().map(v3));
    v.extend([
        v3(&g.disc_pos),
        g.move_pattern.to_string(),
        g.dead_voice_flg.to_string(),
        g.alpha.to_string(),
        g.st_flg.to_string(),
        g.state.to_string(),
        g.tmp_state.to_string(),
    ]);
    v.push(holds(Held::GomoraAura(me)));
    let h = &g.hit;
    v.extend([
        u8::from(h.sw).to_string(),
        (h.mask as i32).to_string(),
        (h.mask2 as i32).to_string(),
        h.kind.to_string(),
        h.radius.to_string(),
        h.height.to_string(),
        v3(&h.pos),
        v3(&h.offset),
    ]);
    // GomoraInit's stream (+0xcc): what it draws from.
    v.push(format!("\"{}\"", piney_battle::boss::kyvia::stream(g.kyvia_lv).unwrap_or("")));
    v.join(",")
}

fn kyvia_json(x: &Kyvia) -> String {
    let mut v: Vec<String> = vec![
        x.st_flg.to_string(),
        x.wait_disc_count.to_string(),
        x.disc_lv.to_string(),
        x.live_flg.to_string(),
        x.atk_pat_mode.to_string(),
        x.dmg_anm_flg.to_string(),
        x.hp_proccess.to_string(),
        x.af_cou.to_string(),
        x.cinema.to_string(),
        x.time_mode.to_string(),
        x.max_hp.to_string(),
        x.switch_hp.to_string(),
        v3(&x.disc_pos),
        v3(&x.l_offset),
        v3(&x.atk_vec),
        x.z.to_string(),
        v3(&x.s_point),
        x.btime.to_string(),
        x.add_time.to_string(),
        x.sub_btime.to_string(),
    ];
    v.extend(x.l_bpos.iter().map(v3));
    v.extend(x.l_bpos_sub.iter().map(v3));
    v.extend(x.l_char.iter().map(v3));
    v.extend(x.l_point.iter().map(v3));
    v.extend([
        v3(&x.tes_pos),
        v3(&x.tes_dirc),
        v3(&x.snd_pos_l),
        v3(&x.snd_pos_r),
        v3(&x.ded_dmg_vec),
        v3(&x.quake_vector),
        v3(&x.move_transfer),
    ]);
    v.extend(x.dmg_gp.iter().map(|&g| u8::from(g).to_string()));
    v.push(u8::from(x.dead_gp).to_string());
    v.push(holds(Held::Light));
    v.extend((0..3).map(|k| holds(Held::BeamHead(k))));
    v.extend((0..3).map(|k| holds(Held::BeamTail(k))));
    if let Some(t) = &x.thunder {
        v.extend([t.kind as i32, t.time, i32::from(t.num), t.range, t.stage_id].map(|n| n.to_string()));
    }
    if x.fight == Fight::Third {
        v.push(x.pat_count.to_string());
    }
    if let Some(f) = &x.fourth {
        v.extend([f.fight_flg, f.rising].map(|n| n.to_string()));
        v.push(f.core_lv_count.to_string());
        v.extend(f.core_appear_pos.iter().map(v3));
        v.push(v3(&f.dead_set_pos));
        v.extend([f.rebirth_flg, f.rebirth_cnt, f.wall_gp_flg].map(|n| n.to_string()));
        for k in 0..4u8 {
            for h in [Held::Wall(k), Held::Wall2(k), Held::CoreApp(k), Held::CoreApp2(k)] {
                v.push(holds(h));
                v.push(paused(h));
            }
        }
        v.push(f.fade_num.to_string());
    }
    v.join(",")
}

/// The frame's line with the ball's and hell's express's members before
/// `rand`.
fn with_ball(line: &str, b: &Boss) -> String {
    line.replacen("\"rand\":", &format!("\"ball\":{},\"hell\":{},\"rand\":", ball_json(b), hell_json(b)), 1)
}

/// Hell's express's members as the harness reads them (null without one):
/// `HellAct`, `HellCou`, `EndCount`, `SoulNum`, `BungStartFlg`, `KyviaLV`,
/// the target, `BungAnm`'s frame, the spline, the eye, and each soul's
/// `Pos`, `time` and `ActProccess`.
fn hell_json(b: &Boss) -> String {
    let Some(h) = b.effects.slots.iter().flatten().find_map(|e| e.hell.as_ref()) else { return "null".into() };
    let mut v: Vec<String> = [h.hell_act, h.hell_cou, h.end_count, h.soul_num, i32::from(h.bung_start), h.kyvia_lv]
        .iter()
        .map(|n| n.to_string())
        .collect();
    v.push(h.target.map_or(-1, |t| t as i64).to_string());
    v.push(h.bung_anm.frame().to_string());
    v.extend(h.spoint.iter().map(v3));
    v.push(v3(&h.target_cam_pos));
    for k in 0..3 {
        v.push(v3(&h.pos[k]));
        v.push(h.time[k].to_string());
        v.push(h.act_proccess[k].to_string());
    }
    format!("[{}]", v.join(","))
}

/// The magical ball's members as the harness reads them (null without
/// one): `EffMode`, `Delay`, `ScalePoint`, `MaxScale`, `Count`, `CFLG`,
/// `ABGR`, `DePoint`, `BlurDelay`, `EffAnm`'s transparency, `Rot`, then
/// each plasma's `time`, `Mode`, `SetT`, `TempT`, three `SBpoint`s,
/// `PlaPos` and `PlaPos2`.
fn ball_json(b: &Boss) -> String {
    let Some(ball) = b.effects.slots.iter().flatten().find_map(|e| e.ball.as_ref()) else { return "null".into() };
    let mut v: Vec<String> = [
        ball.eff_mode,
        ball.delay,
        ball.scale_point as i32,
        ball.max_scale as i32,
        ball.count,
        ball.cflg,
        ball.abgr as i32,
        ball.de_point,
        ball.blur_delay,
        ball.anm_transparency as i32,
    ]
    .iter()
    .map(|n| (*n as u32).to_string())
    .collect();
    v.push(v3(&ball.rot));
    for k in 0..ball.time.len() {
        v.extend([ball.time[k], ball.mode[k] as u32, ball.set_t[k], ball.temp_t[k]].map(|n| n.to_string()));
        v.extend(ball.sb.iter().map(|s| v3(&s[k])));
        v.push(v3(&ball.pla_pos[k]));
        v.push(v3(&ball.pla_pos2[k]));
    }
    format!("[{}]", v.join(","))
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
    cam: &FakeCam,
    blur: &str,
) -> String {
    let Class::Kyvia(x) = &b.class else { return "{}".into() };
    let ch = &scene.chars[me];
    let effs: Vec<String> = b
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
    let core = x.core;
    let cb = scene.chars[core].foe_state().and_then(|f| f.boss.as_ref());
    let (core_s, gs) = match cb.map(|b| &b.class) {
        Some(Class::KyviaCore(c)) => {
            let cb = cb.unwrap();
            let gs: Vec<String> = c
                .gomoras
                .iter()
                .map(|&g| {
                    let gb = scene.chars[g].foe_state().and_then(|f| f.boss.as_ref()).unwrap();
                    let Class::Gomora(gx) = &gb.class else { return "[]".into() };
                    format!("[{},{}]", boss_json(gb, scene, g), gomora_json(gx, g))
                })
                .collect();
            (format!("[{},{}]", boss_json(cb, scene, core), core_json(c)), gs)
        }
        _ => ("[]".into(), Vec::new()),
    };
    format!(
        "{{\"act\":[{},{},{}],\"pos\":[{}],\"flags\":[{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\"anm\":[\"{}\",{},\"{}\",{}],\
         \"kyvia\":[{}],\"core\":{},\"gomoras\":[{}],\"cam\":[{},{},{},{},{},{},{}],\"blur\":{},\
         \"eff\":[{}],\"party\":[{}],\"out\":[{}],\"menu\":[{},{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
        b.act_num,
        b.act_proccess,
        b.act_count,
        v3(&ch.pos),
        b.act_forbid,
        b.anm_status,
        b.stop,
        b.exit,
        b.draw_sw,
        b.lock_player,
        b.reserve_forbid_menu,
        b.reserve_forbid_chat_except,
        b.transparency,
        b.set_transparency,
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        x.anm_w.clip.as_deref().unwrap_or(""),
        x.anm_w.frame(),
        kyvia_json(x),
        core_s,
        gs.join(","),
        cam.reset,
        cam.lock,
        v3(&cam.view),
        v3(&cam.pos),
        cam.xrot,
        cam.ex_lock,
        cam.sway,
        blur,
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
