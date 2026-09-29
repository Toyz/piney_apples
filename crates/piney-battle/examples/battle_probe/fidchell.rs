//! `battle_probe` requests for Fidchell (`piney_battle::boss::fidchell`), as
//! `tools/test_battle_fidchell_rs.py` sends them: `fidchellclips` (the clips
//! of `x41` and `xeffect`) and `fidchell ...`: the boss made on a scene, then
//! FRAMES frames of the manager's pass and `Main` with affects before each
//! (0 a hit, 1 the gauge, 2 affect 13, 3 affect 21, 4 the break count, 6 its
//! own skill over, 7 a member's `dead`). The camera turns by the case's
//! schedule; `checkCameraShakeRange` answers by its frame. One JSON line:
//! the state after the constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::fidchell::{self, Fidchell, Pic};
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
    for stem in [fidchell::FILE, "xeffect"] {
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

/// Answers `fidchellclips` and `fidchell`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "fidchellclips" | "fidchell") {
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
        if cmd == "fidchellclips" {
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
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
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
    };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let volume = crate::probe_volume();
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false };
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
        game_over: false,
        boss_cam: true,
        cam: view_at(0, true),
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = fidchell::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    // bossCamSW: the dead effect hands the boss camera to camera 1 once.
    let mut cam_sw = true;
    let mut max_range = 0u32;
    let outs: Vec<String> = out.iter().filter_map(|o| mout(o, &cam2, &mut cam_sw, &mut max_range)).collect();
    lines.push(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, max_range));
    for f in 0..frames {
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false };
        let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        let mut out = Vec::new();
        for &(_, kind, who, p0) in script.iter().filter(|s| s.0 == f) {
            let mut ev = Events::new();
            let kite = members[0];
            if let Some(b) = scene.chars[me].foe_state_mut().and_then(|x| x.boss.as_mut()) {
                out.extend(b.take_pending());
            }
            match kind {
                0 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 1, [p0, 0, 0], &mut rand, &mut ev),
                1 => scene.chars[me].foe_state_mut().unwrap().pp = p0,
                2 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 13, [0; 3], &mut rand, &mut ev),
                3 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 21, [0; 3], &mut rand, &mut ev),
                4 => scene.chars[me].foe_state_mut().unwrap().pp_count = p0,
                6 => {
                    scene.chars[me].skill_id = 0;
                    scene.chars[me].skill_status = 0;
                }
                7 => {
                    if let Some(m) = members.get(who).copied().flatten() {
                        scene.chars[m].cond[0] = p0;
                    }
                }
                _ => {}
            }
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut all = out;
        all.extend(boss.take_pending());
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
            game_over: false,
            boss_cam: true,
            cam: view_at(f, false),
            land: &mut land,
            disc: piney_battle::boss::DiscView::default(),
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.main(&mut cx);
        all.extend(std::mem::take(&mut cx.out));
        drop(cx);
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
        lines.push(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu, max_range));
    }
    format!("[{}]", lines.join(","))
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

/// A call as the harness records it.
fn mout(o: &Out, cam2: &(V4, V4), cam_sw: &mut bool, max_range: &mut u32) -> Option<String> {
    Some(match o {
        Out::Fidchell(p) => match p {
            Pic::Text { .. } => return None,
            Pic::Voice(n) => format!("[\"voice\",{n}]"),
            Pic::VoiceStop => "[\"voice_stop\"]".into(),
            Pic::Smoke { .. } => "[\"smoke\"]".into(),
            Pic::MeteorLand { .. } => "[\"smokerock\"],[\"explode\"],[\"explode\"]".into(),
            Pic::Radiate { .. } => "[\"radiate\"]".into(),
            Pic::Shock { .. } => {
                "[\"flare\"],[\"radiate\"],[\"radiate\"],[\"radiate\"],[\"radiate\"],[\"explode\"],[\"explode\"],\
                 [\"explode\"],[\"explode\"],[\"explode\"]"
                    .into()
            }
        },
        Out::CameraChange(n) => format!("[\"cam\",{n}]"),
        Out::CameraPos { cam, pos } => format!("[\"campos\",{cam},{}]", v3(pos)),
        Out::CameraView { cam, view } => format!("[\"camview\",{cam},{}]", v3(view)),
        Out::CamMode { mode } => format!("[\"cammode\",{mode}]"),
        Out::FreeCam { pos, view } => format!("[\"freecam\",{},{}]", v3(pos), v3(view)),
        Out::CinemaSkill(sid) => format!("[\"cinema_skill\",{sid}]"),
        // The spells run natively in the harness: seen in the slots.
        Out::Effect {
            kind: EffKind::MeteoSworm { .. } | EffKind::ThunderStorm { .. } | EffKind::RockTower { .. },
            ..
        } => {
            return None;
        }
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

fn fid_json(b: &Boss, x: &Fidchell) -> String {
    let p = &x.pred;
    let v: Vec<String> = vec![
        format!("\"{}\"", x.pred_txt.clip.as_deref().unwrap_or("")),
        x.pred_txt.frame().to_string(),
        v3(&x.pred_pos),
        v3(&x.pred_dirc),
        u8::from(x.eff_magic.is_some()).to_string(),
        u8::from(x.eff_dead.is_some()).to_string(),
        u8::from(x.br).to_string(),
        u8::from(x.eff_start.is_some()).to_string(),
        x.act_sub_proccess.to_string(),
        x.act_sub_count.to_string(),
        x.stage_eff_id.to_string(),
        x.pred_id.to_string(),
        x.reserve_pred.to_string(),
        x.rot_z.to_string(),
        x.transparency.to_string(),
        x.skill_id.to_string(),
        v3(&p.cam_pos),
        v3(&p.cam_view),
        p.quake_sw.to_string(),
        p.quake_offset.to_string(),
        p.transparency.to_string(),
        p.act_count.to_string(),
        u8::from(p.rev_layer).to_string(),
        x.pat_mode.to_string(),
        b.stop_time.to_string(),
        b.stop_counter.to_string(),
        b.pat_index.to_string(),
        b.pat_num.to_string(),
        b.epitaph.to_string(),
        v3(&b.dash_pos),
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
    let Class::Fidchell(x) = &b.class else { return "{}".into() };
    let ch = &scene.chars[me];
    let f = ch.foe_state().unwrap();
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
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"pp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{},\"{}\",{},{}],\"fid\":[{}],\"cam\":[{}],\"eff\":[{}],\"party\":[{}],\"out\":[{}],\
         \"menu\":[{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
        b.act_num,
        b.act_proccess,
        b.act_count,
        b.move_spd,
        b.move_dirc,
        v3(&b.move_vector),
        ch.target_char.map_or(-1, |t| t as i64),
        b.target_dist,
        b.target_dirc,
        v3(&b.target_pos),
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
        u8::from(b.reserve_forbid_chat_except != 0),
        b.stop_count,
        u8::from(scene.listed(me)),
        b.transparency,
        b.set_transparency,
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        b.anm.speed,
        x.anm_w.clip.as_deref().unwrap_or(""),
        x.anm_w.frame(),
        x.anm_w.speed,
        fid_json(b, x),
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
