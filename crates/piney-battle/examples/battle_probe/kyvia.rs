//! `battle_probe` requests for Kyvia (`piney_battle::boss::kyvia`), as
//! `tools/test_battle_kyvia_rs.py` sends them: `kyviaclips` (the clips of
//! `x01`, `xeffect`, `particle`) and `kyvia ...`: the body made on a scene,
//! then FRAMES frames of the manager's pass and `Main`, the disc and the
//! camera scripted, affects before each (0 a hit on the core, 1 on gomora
//! P1, 2 a heal on the core). One JSON line: the state after the
//! constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::kyvia::core::Core;
use piney_battle::boss::kyvia::gomora::Gomora;
use piney_battle::boss::kyvia::{Gen, Kyvia};
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, DiscView, EffKind, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::{Event, Events, Who};
use piney_battle::exp::Party;
use piney_battle::param::cond;
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::field::ee;
use piney_data::iso::Iso;

use crate::boss::{eff_num, out_json};
use crate::{Toks, read_char};

type Clips = std::collections::HashMap<String, (u32, bool)>;
type V4 = [u32; 4];

thread_local! {
    static DATA: std::cell::RefCell<Option<(BossData, Clips)>> = const { std::cell::RefCell::new(None) };
}

fn load(iso_path: &str) -> (BossData, Clips) {
    let mut iso = Iso::open(iso_path).expect("the ISO");
    let data = BossData::of(iso.volume().expect("the volume"));
    let arc = Archive::new(iso.read_path("DATA/DATA.BIN").expect("DATA.BIN")).expect("DATA.BIN");
    let mut clips = std::collections::HashMap::new();
    for stem in ["x01", "xeffect", "particle"] {
        let Ok(raw) = arc.inflate_named(stem) else { continue };
        let ccs = Ccs::parse(raw).expect("a CCSF");
        let scene = piney_data::scene::Scene::read(&ccs).expect("its scene");
        for a in piney_data::anim::Animation::all(&ccs, &scene).expect("its animations") {
            if let Some(name) = ccs.object_name(a.object) {
                clips.insert(name.to_string(), (a.frames, a.looping));
            }
        }
    }
    (data, clips)
}

/// Answers `kyviaclips` and `kyvia`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "kyviaclips" | "kyvia") {
        return None;
    }
    DATA.with(|d| {
        if d.borrow().is_none() {
            *d.borrow_mut() = Some(load(iso));
        }
    });
    Some(DATA.with(|d| {
        let d = d.borrow();
        let (data, clips) = d.as_ref().unwrap();
        if cmd == "kyviaclips" {
            let mut v: Vec<_> = clips.iter().collect();
            v.sort();
            let body: Vec<String> = v.iter().map(|(k, (f, l))| format!("\"{k}\":[{f},{}]", u8::from(*l))).collect();
            format!("{{{}}}", body.join(","))
        } else {
            run(tables, data, clips, t)
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
}

impl FakeCam {
    fn take(&mut self, o: &Out) {
        match o {
            Out::CamMode { mode } | Out::CamModeRange { mode, .. } => {
                self.reset = *mode;
                match mode {
                    5 => {
                        self.lock = 1;
                        self.temp_view = self.view;
                        self.temp_pos = self.pos;
                    }
                    6 => {
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
            Out::CamPitch { add, v } => self.xrot = if *add { ee::add(self.xrot, *v) } else { *v },
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
    let marker = [t.u32(), t.u32(), t.u32(), 0x3f80_0000];
    let mut rand = Rand(t.int() as u64);
    let mut cc = Genrand::seeded(t.u32());
    cc.mti = t.i32();
    let count0 = t.u32();
    let frames = t.int() as usize;
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
    let case = Case { frames, rot, moving, prev, marker, cam_pos, cam_view, delay };
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
    let party = Party { members, ids, num: members.iter().flatten().count() as i32 };
    let clip = |name: &str| clips.get(name).copied();
    let mut forbid = 0i16;
    let mut menu = [0i32; 6];
    let mut cam = FakeCam::default();
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let view = |f: usize, reset: i32| CamView {
        rot: [0, 0, case.rot[f.min(case.frames.saturating_sub(1))], 0],
        pos: case.cam_pos,
        view: case.cam_view,
        moving: false,
        reset,
    };
    let disc = |f: usize| DiscView {
        moving: case.moving.get(f).copied().unwrap_or(false),
        prev_pos: case.prev.get(f).copied().unwrap_or(case.marker),
        marker: case.marker,
    };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false };
    let actx =
        AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume: crate::probe_volume() };
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
        game_over: false,
        boss_cam: true,
        cam: view(0, 0),
        land: &mut land,
        disc: disc(0),
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = piney_battle::boss::kyvia::new(&mut cx);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    for o in &out {
        cam.take(o);
    }
    let mut blur = blur_of(&boss);
    let outs: Vec<String> = out.iter().filter_map(kout).collect();
    lines.push(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, &cam, blur));
    for f in 0..frames {
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false };
        let actx = AffectCtx {
            party: &party,
            menu: true,
            skill_check: &check,
            boss: Some(&benv),
            volume: crate::probe_volume(),
        };
        cam.step(case.delay);
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
            game_over: false,
            boss_cam: true,
            cam: view(f, cam.reset),
            land: &mut land,
            disc: disc(f),
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.main(&mut cx);
        let main_out = std::mem::take(&mut cx.out);
        drop(cx);
        outs.extend(main_out.iter().filter_map(kout));
        out.extend(main_out);
        for o in &out {
            cam.take(o);
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
                _ => {}
            }
        }
        blur = blur_of(&boss);
        lines.push(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, &cam, blur));
    }
    format!("[{}]", lines.join(","))
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

fn blur_of(b: &Boss) -> [u32; 4] {
    match &b.class {
        Class::Kyvia(x) => [u32::from(x.blur.enabled), u32::from(x.blur.exit), x.blur.scale, x.blur.abgr],
        _ => [0; 4],
    }
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

fn gen_json(g: Gen) -> String {
    let (name, row) = match g {
        Gen::Kyvia(r) => ("Kyvia", r),
        Gen::KyviaDmg(r) => ("KyviaDmg", r),
        Gen::Core(r) => ("Core", r),
        Gen::Gomora(r) => ("Gomora", r),
        Gen::GomoraAura(r, l) => return format!("[\"particles\",\"GomoraAura\",{r},{l}]"),
        Gen::MissileSmoke(r) => ("MissileSmoke", r),
        Gen::BurstSmoke(r) => ("BurstSmoke", r),
    };
    format!("[\"particles\",\"{name}\",{row}]")
}

fn kout(o: &Out) -> Option<String> {
    Some(match o {
        Out::KyviaParticles { which, .. } => gen_json(*which),
        Out::SkillFrom { skill, .. } => format!("[\"skill\",{}]", skill.sid),
        Out::SkillStart { sid, .. } => format!("[\"skill_start\",{sid}]"),
        Out::DiscNextStage => "[\"disc_next\"]".into(),
        Out::SmokeRock { .. } => "[\"smoke_rock\"]".into(),
        Out::MusicFade { t } => format!("[\"music_fade\",{t}]"),
        // The meteors run natively in the harness: their making is seen in
        // the effects' slots.
        Out::Effect { kind: EffKind::Meteorite { .. }, .. } => return None,
        Out::CamModeRange { .. } | Out::CamInitLock | Out::CamRotXLimit(_) | Out::CamSway(_) | Out::DrawArm { .. } => {
            return None;
        }
        Out::PartHit { .. } => return None,
        _ => return out_json(o),
    })
}

fn boss_json(b: &Boss, scene: &Scene, who: usize) -> String {
    let ch = &scene.chars[who];
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{}",
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
    ];
    v.join(",")
}

fn gomora_json(g: &Gomora) -> String {
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
    v.join(",")
}

#[allow(clippy::too_many_arguments)]
fn frame_json(
    b: &Boss,
    scene: &Scene,
    me: usize,
    outs: &[String],
    rand: &Rand,
    cc: &Genrand,
    menu: &[i32; 6],
    cam: &FakeCam,
    blur: [u32; 4],
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
                    format!("[{},{}]", boss_json(gb, scene, g), gomora_json(gx))
                })
                .collect();
            (format!("[{},{}]", boss_json(cb, scene, core), core_json(c)), gs)
        }
        _ => ("[]".into(), Vec::new()),
    };
    format!(
        "{{\"act\":[{},{},{}],\"pos\":[{}],\"flags\":[{},{},{},{},{},{},{},{}],\"anm\":[\"{}\",{},\"{}\",{}],\
         \"kyvia\":[{}],\"core\":{},\"gomoras\":[{}],\"cam\":[{},{},{},{},{}],\"blur\":[{},{},{},{}],\
         \"eff\":[{}],\"party\":[{}],\"out\":[{}],\"menu\":[{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
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
        blur[0],
        blur[1],
        blur[2],
        blur[3],
        effs.join(","),
        party.join(","),
        outs.join(","),
        menu[0],
        menu[1],
        menu[2],
        menu[3],
        menu[4],
        menu[5],
        rand.0,
        cc.mti,
    )
}
