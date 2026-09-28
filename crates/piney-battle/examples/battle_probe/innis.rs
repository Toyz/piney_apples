//! `battle_probe` requests for Innis (`piney_battle::boss::innis`), as
//! `tools/test_battle_innis_rs.py` sends them: `innisclips` (every clip of
//! `x21` and `xeffect`) and `innis ...`: a scene with the boss, Innis made
//! on it, then FRAMES frames of the manager's pass and `Main` with the
//! camera's turn and catch-up scripted and affects on the boss before each
//! (as `boss`'s: 0 a hit, 1 the gauge, 2 affect 13, 3 affect 21, 4 the
//! break count). One JSON line: the state after the constructor, then each
//! frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::Events;
use piney_battle::exp::Party;
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
    for stem in ["x21", "xeffect"] {
        let ccs = Ccs::parse(arc.inflate_named(stem).expect("the file")).expect("a CCSF");
        let scene = piney_data::scene::Scene::read(&ccs).expect("its scene");
        for a in piney_data::anim::Animation::all(&ccs, &scene).expect("its animations") {
            if let Some(name) = ccs.object_name(a.object) {
                clips.insert(name.to_string(), (a.frames, a.looping));
            }
        }
    }
    (data, clips)
}

/// Answers `innisclips` and `innis`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "innisclips" | "innis") {
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
        if cmd == "innisclips" {
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
/// rules' `SetMode`, `SetFreeCamPosView` and pitch (CamMain never runs).
#[derive(Default)]
struct FakeCam {
    reset: i32,
    lock: u8,
    view: V4,
    pos: V4,
    temp_view: V4,
    temp_pos: V4,
    xrot: u32,
}

impl FakeCam {
    fn take(&mut self, o: &Out) {
        match o {
            Out::CamMode { mode } => {
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
    let cam_pos = v4(t);
    let cam_view = v4(t);
    let ground = t.u32();
    let rot: Vec<u32> = (0..frames).map(|_| t.u32()).collect();
    let moving: Vec<bool> = (0..frames).map(|_| t.int() != 0).collect();
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, i16)> = (0..ns).map(|_| (t.int() as usize, t.i32(), t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
    let party = Party { members, ids, num: members.iter().flatten().count() as i32 };
    let clip = |name: &str| clips.get(name).copied();
    let mut forbid = 0i16;
    let mut menu = [0i32; 6];
    let mut cam = FakeCam::default();
    let mut blur = 0u32;
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let view = |r: u32, moving: bool| CamView { rot: [0, 0, r, 0], pos: cam_pos, view: cam_view, moving, reset: 0 };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false };
    let actx =
        AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume: crate::probe_volume() };
    let mut none = |_| None;
    let mut land = |_: V4| ground;
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
        cam: view(rot.first().copied().unwrap_or(0), false),
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = piney_battle::boss::innis::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    for o in &out {
        if let Out::Blur { colour } = o {
            blur = *colour;
        }
    }
    lines.push(frame_json(&boss, &scene, me, &out, &rand, &cc, &menu, &cam, blur));
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
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        for &(_, kind, p0) in script.iter().filter(|s| s.0 == f) {
            let mut ev = Events::new();
            let kite = members[0];
            match kind {
                0 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 1, [p0, 0, 0], &mut rand, &mut ev),
                1 => scene.chars[me].foe_state_mut().unwrap().pp = p0,
                2 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 13, [0; 3], &mut rand, &mut ev),
                3 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 21, [0; 3], &mut rand, &mut ev),
                4 => scene.chars[me].foe_state_mut().unwrap().pp_count = p0,
                _ => {}
            }
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut out = boss.take_pending();
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
            cam: view(rot[f], moving[f]),
            land: &mut land,
            disc: piney_battle::boss::DiscView::default(),
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.main(&mut cx);
        out.extend(std::mem::take(&mut cx.out));
        drop(cx);
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
                Out::Blur { colour } => blur = *colour,
                _ => {}
            }
        }
        lines.push(frame_json(&boss, &scene, me, &out, &rand, &cc, &menu, &cam, blur));
    }
    format!("[{}]", lines.join(","))
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

#[allow(clippy::too_many_arguments)]
fn frame_json(
    b: &Boss,
    scene: &Scene,
    me: usize,
    out: &[Out],
    rand: &Rand,
    cc: &Genrand,
    menu: &[i32; 6],
    cam: &FakeCam,
    blur: u32,
) -> String {
    let Class::Innis(x) = &b.class else { return "{}".into() };
    let ch = &scene.chars[me];
    let f = ch.foe_state().unwrap();
    // The rings and the missiles run natively in the harness, where their
    // making is seen in the effects' slots only.
    let native = |o: &&Out| {
        matches!(
            o,
            Out::Effect {
                kind: piney_battle::boss::EffKind::SamonRing { .. } | piney_battle::boss::EffKind::Missile { .. },
                ..
            }
        )
    };
    let outs: Vec<String> = out.iter().filter(|o| !native(o)).filter_map(out_json).collect();
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
    let innis = [
        x.action_flg.to_string(),
        x.escape.to_string(),
        x.eny_flg.to_string(),
        x.eny_int.to_string(),
        x.eny_float.to_string(),
        x.entry_flg.to_string(),
        x.epitaph_flg.to_string(),
        x.dd_flg.to_string(),
        x.hipos.to_string(),
        x.now_mode.to_string(),
        x.pat_end_mode[0].to_string(),
        x.pat_end_mode[1].to_string(),
        x.pat_end_mode[2].to_string(),
        x.pat_end_flg.to_string(),
        x.lock_target_flg.to_string(),
        x.flg.to_string(),
        x.move_flg.to_string(),
        x.end_flg.to_string(),
        x.temphi.to_string(),
        x.rotate.to_string(),
        x.dircsub.to_string(),
        x.sub_hi.to_string(),
        x.action_start_flg.to_string(),
        x.pos_b.to_string(),
        x.pos_a.to_string(),
        x.p_flg.to_string(),
        x.p_dist.to_string(),
        x.cou.to_string(),
        x.af_cou.to_string(),
        x.no.to_string(),
        x.sub_no.to_string(),
        x.mirror_pros.to_string(),
        x.monster_id.to_string(),
        x.alpha.to_string(),
        x.back_step_dist.to_string(),
        x.ex_spin_back_flg.to_string(),
        x.escape_act_cou.to_string(),
        x.dmg_count.to_string(),
        x.cam_dist.to_string(),
        v3(&x.zoom_vec),
        v3(&x.zoom_view),
        v3(&x.cam_pos),
        v3(&x.cam_view),
        x.skill_id.to_string(),
        v3(&x.quake_vector),
        x.cam_rot[2].to_string(),
        v3(&x.rot_vec),
        v3(&x.sub_vec),
        x.rot.to_string(),
        x.sub_rot.to_string(),
    ]
    .join(",");
    let mut shards = 0;
    let slaves: Vec<String> = x
        .slaves
        .iter()
        .map(|s| {
            shards += s.mirror.drawn.len();
            let sb = &s.b;
            let sc = &scene.chars[s.me];
            let m = &s.mirror;
            format!(
                "[{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{},{},{},{},{},{},{}]",
                sb.exit,
                sb.draw_sw,
                sb.act_num,
                sb.act_proccess,
                sb.act_count,
                v3(&sc.pos),
                v3(&sb.dirc),
                v3(&sb.move_vector),
                sb.target_dist,
                s.samon_id,
                s.dist,
                s.dirc,
                s.transparency,
                s.eny_flg,
                v3(&s.pos),
                v3(&s.spin),
                v3(&s.target_pos),
                s.start_count,
                s.flg,
                s.end_flg,
                s.order_num,
                s.quake_time,
                sb.anm.clip.as_deref().unwrap_or(""),
                sb.anm.frame(),
                m.f,
                m.set_point_flg,
                m.vx[0],
                m.vz[0],
                m.x[0],
                m.z[0],
            )
        })
        .collect();
    let tpos = x.target_pos_of.map_or(-1, |t| t as i64);
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"pp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},\"{}\",{}],\"innis\":[{}],\"cam\":[{},{},{},{},{}],\"blur\":{},\"slaves\":[{}],\
         \"shards\":{},\"eff\":[{}],\"party\":[{}],\"out\":[{}],\"menu\":[{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
        b.act_num,
        b.act_proccess,
        b.act_count,
        b.move_spd,
        b.move_dirc,
        v3(&b.move_vector),
        ch.target_char.map_or(-1, |t| t as i64),
        b.target_dist,
        b.target_dirc,
        tpos,
        v3(&ch.pos),
        v3(&ch.pos_p),
        v3(&b.dirc),
        ch.hp,
        ch.max_hp,
        f.pp,
        f.pp_count,
        b.act_forbid,
        b.anm_status,
        b.stop,
        b.cheat_hp,
        b.exit,
        b.draw_sw,
        b.body_hit_sw,
        b.lock_player,
        b.erase_target,
        b.reserve_forbid_menu,
        b.reserve_forbid_chat_except,
        b.stop_count,
        b.transparency,
        b.set_transparency,
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        x.anm_w.clip.as_deref().unwrap_or(""),
        x.anm_w.frame(),
        innis,
        cam.reset,
        cam.lock,
        v3(&cam.view),
        v3(&cam.pos),
        cam.xrot,
        blur,
        slaves.join(","),
        shards,
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
