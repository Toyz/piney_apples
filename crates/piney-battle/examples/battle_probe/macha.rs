//! `battle_probe` requests for Macha (`piney_battle::boss::macha`), as
//! `tools/test_battle_macha_rs.py` sends them: `machaclips` (the clips of
//! `x61` and `xeffect`) and `macha ...`: the boss made on a scene, then
//! FRAMES frames of the manager's pass and `Main` with affects before each
//! (0 a hit, 1 the gauge, 2 affect 13, 3 affect 21, 4 the break count, 6 its
//! own skill over, 7 a member's `dead`, 8 an act forced, 9 a hit of type
//! 3). The camera turns by the case's schedule; `checkCameraShakeRange`
//! answers by its frame; the eyes stand where the case puts them; the land
//! is the plane z 0 (a segment ending at or under it meets it at its end).
//! One JSON line: the state after the constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::macha::{self, Macha, Pic};
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
    static DATA: std::cell::RefCell<Option<(BossData, Clips)>> = const { std::cell::RefCell::new(None) };
}

fn load(iso_path: &str) -> (BossData, Clips) {
    let mut iso = Iso::open(iso_path).expect("the ISO");
    let data = BossData::of(iso.volume().expect("the volume"));
    let arc = Archive::new(iso.read_path("DATA/DATA.BIN").expect("DATA.BIN")).expect("DATA.BIN");
    let mut clips = std::collections::HashMap::new();
    for stem in [macha::FILE, "xeffect"] {
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

/// Answers `machaclips` and `macha`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "machaclips" | "macha") {
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
        if cmd == "machaclips" {
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

/// `checkCameraShakeRange` as the harness answers it: yes but on every
/// `m`th frame (always for 0).
fn shake(m: i64, f: usize) -> bool {
    !(m > 0 && f as i64 % m == 0)
}

/// The land as the case has it: the plane z 0.
struct Plane;

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
        (f32::from_bits(ep[2]) <= 0.0).then_some(ep)
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
    let shake_mod = t.int();
    let push = (t.int() as usize, t.int() as usize);
    let cam2 = (v4(t), v4(t));
    let nr = t.int() as usize;
    let rots: Vec<(usize, u32)> = (0..nr).map(|_| (t.int() as usize, t.u32())).collect();
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, usize, i16)> =
        (0..ns).map(|_| (t.int() as usize, t.i32(), t.int() as usize, t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    let eyes = [v4(t), v4(t)];
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
    let mut menu = [0i32; 6];
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let rot_at = |f: usize| rots.iter().rfind(|(k, _)| *k <= f).map_or(0, |(_, v)| *v);
    let view_at = |f: usize, first: bool| CamView {
        rot: [0, 0, rot_at(f), 0],
        pos: cam2.0,
        view: cam2.1,
        moving: false,
        reset: 0,
        shake: !first && shake(shake_mod, f),
        eye: [0; 4],
        active: 1,
        active_pos: [0; 4],
        active_view: [0; 4],
        boss_view: [0; 4],
        fade,
    };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let volume = crate::probe_volume();
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false, cc: None };
    let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
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
        part_hits: &mut Plane,
        game_over: false,
        boss_cam: true,
        cam: view_at(0, true),
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        gen_alive: &|_| true,
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = macha::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    // bossCamSW: the dead effect hands the boss camera to camera 1 once.
    let mut cam_sw = true;
    let mut max_range = 0u32;
    let outs: Vec<String> = out.iter().filter_map(|o| mout(o, &cam2, &mut cam_sw, &mut max_range)).collect();
    blur.apply(&out);
    lines.push(with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, max_range), &blur));
    for f in 0..frames {
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        let mut out = Vec::new();
        for &(_, kind, who, p0) in script.iter().filter(|s| s.0 == f) {
            let mut ev = Events::new();
            let kite = members[0];
            if let Some(b) = scene.chars[me].foe_state_mut().and_then(|x| x.boss.as_mut()) {
                out.extend(b.base.take_pending());
            }
            match kind {
                0 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 1, [p0, 0, 0], &mut rand, &mut ev),
                1 => scene.param_mut(me).unwrap().pp = p0,
                2 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 13, [0; 3], &mut rand, &mut ev),
                3 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 21, [0; 3], &mut rand, &mut ev),
                4 => scene.param_mut(me).unwrap().pp_count = p0,
                6 => {
                    scene.chars[me].skill_id = 0;
                    scene.chars[me].skill_status = 0;
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
                9 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 3, [p0, 0, 0], &mut rand, &mut ev),
                _ => {}
            }
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        if let Class::Macha(x) = &mut boss.class {
            x.eyes = eyes;
        }
        let mut all = out;
        all.extend(boss.base.take_pending());
        // The affects' writes, then the manager's pass, then the boss's.
        blur.apply(&all);
        blur.frame();
        let pre = all.len();
        let pushing = (push.0..push.1).contains(&f);
        let mut pushed = move |_: V4| pushing.then_some([0u32; 4]);
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
            collide: &mut pushed,
            part_hits: &mut Plane,
            game_over: false,
            boss_cam: true,
            cam: view_at(f, false),
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
                _ => {}
            }
        }
        let outs: Vec<String> = all.iter().filter_map(|o| mout(o, &cam2, &mut cam_sw, &mut max_range)).collect();
        lines.push(with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, max_range), &blur));
    }
    format!("[{}]", lines.join(","))
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

/// A call as the harness records it.
fn mout(o: &Out, cam2: &(V4, V4), cam_sw: &mut bool, max_range: &mut u32) -> Option<String> {
    Some(match o {
        Out::Macha(p) => match p {
            // Drawn: the runtime's.
            Pic::Ball { .. } | Pic::Light(_) | Pic::Wave { .. } | Pic::Laser { .. } | Pic::Blast { .. } => {
                return None;
            }
            Pic::Smoke { .. } => "[\"smoke\"]".into(),
            Pic::LaserHit { .. } => "[\"explode\"],[\"explode\"],[\"smokerock\"]".into(),
            Pic::Burst { .. } => "[\"explode\"],[\"smoke\"],[\"smokerock\"]".into(),
        },
        Out::AfterImageWith(p) => format!("[\"afterimage\",{},{}]", p.color, p.trans_spd),
        Out::BlurSinRot(_) => return None,
        Out::CameraChange(n) => format!("[\"cam\",{n}]"),
        Out::CameraPos { cam, pos } => format!("[\"campos\",{cam},{}]", v3(pos)),
        Out::CameraView { cam, view } => format!("[\"camview\",{cam},{}]", v3(view)),
        Out::CamMode { mode } => format!("[\"cammode\",{mode}]"),
        Out::FreeCam { pos, view } => format!("[\"freecam\",{},{}]", v3(pos), v3(view)),
        Out::CinemaSkill(sid) => format!("[\"cinema_skill\",{sid}]"),
        // Macha's own effects run natively in the harness: seen in the slots.
        Out::Effect { kind: EffKind::LaserShot | EffKind::LightBallShock, .. } => return None,
        Out::CamMaxRange(v) => {
            *max_range = *v;
            return None;
        }
        Out::Skill(_, s) => format!("[\"skill\",{},{}]", s.sid, u8::from(s.stype == 2)),
        Out::SkillStart { sid, .. } => format!("[\"skill_start\",{sid}]"),
        Out::SeNote { se, note } => format!("[\"senote\",{se},{note}]"),
        // BeginDeadEffect with the boss camera on: camera 2's eye and view
        // to camera 1, camera 1 on; then camera 3 at the eye.
        Out::DeadCamera { eye, view, .. } => {
            let mut s = String::new();
            if std::mem::take(cam_sw) {
                s = format!("[\"campos\",1,{}],[\"camview\",1,{}],[\"cam\",1],", v3(&cam2.0), v3(&cam2.1));
            }
            format!("{s}[\"cam\",3],[\"campos\",3,{}],[\"camview\",3,{}]", v3(eye), v3(view))
        }
        _ => return out_json(o),
    })
}

fn macha_json(b: &Boss, x: &Macha) -> String {
    let v: Vec<String> = vec![
        format!("\"{}\"", x.anm_ball_charge.clip.as_deref().unwrap_or("")),
        x.anm_ball_charge.frame().to_string(),
        format!("\"{}\"", x.anm_ball.clip.as_deref().unwrap_or("")),
        x.anm_ball.frame().to_string(),
        u8::from(x.eff_laser.is_some()).to_string(),
        u8::from(x.eff_dead.is_some()).to_string(),
        u8::from(x.eff_start.is_some()).to_string(),
        x.skill_id.to_string(),
        x.pat_mode.to_string(),
        u8::from(x.light).to_string(),
        x.light_rot.to_string(),
        x.stage_eff.to_string(),
        v3(&x.ball_pos),
        v3(&x.transfer_pos),
        x.ball_speed.to_string(),
        b.base.stop_time.to_string(),
        b.base.stop_counter.to_string(),
        b.base.pat_index.to_string(),
        b.base.pat_num.to_string(),
        b.base.epitaph.to_string(),
        v3(&b.base.dash_pos),
        b.base.m_act_dirc[2].to_string(),
    ];
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
    max_range: u32,
) -> String {
    let Class::Macha(x) = &b.class else { return "{}".into() };
    let ch = &scene.chars[me];
    let f = scene.param(me).unwrap();
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
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"pp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{},\"{}\",{},{}],\"macha\":[{}],\"cam\":[{}],\"eff\":[{}],\"party\":[{}],\"out\":[{}],\
         \"menu\":[{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
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
        f.pp,
        f.pp_count,
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
        x.anm_w.clip.as_deref().unwrap_or(""),
        x.anm_w.frame(),
        x.anm_w.speed,
        macha_json(b, x),
        max_range,
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
