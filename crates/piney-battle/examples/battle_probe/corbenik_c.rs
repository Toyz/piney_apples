//! `battle_probe`'s `corbenikc`: Corbenik's third form as
//! `tools/test_battle_corbenik_c_rs.py` sends it: the body, its killer eyes
//! and eyes made on a scene, then FRAMES frames of `Main` with affects
//! before each (the second form's kinds, and 15 a hit by skill `p1` on
//! killer eye `who`, 16 a killer eye's HP set). A kept generator lives the
//! case's frames. One JSON line: the state after the constructor, then
//! each frame.

use std::cell::RefCell;
use std::collections::HashMap;

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::corbenik::eye::{Eye, Killer};
use piney_battle::boss::corbenik::third::{self, Third};
use piney_battle::boss::corbenik::{Pic, laser};
use piney_battle::boss::{Boss, BossData, BossEnv, Class, Cx, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::Events;
use piney_battle::exp::Party;
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;

use crate::boss::{eff_num, with_blur};
use crate::corbenik::{Cams, mout, some, v3};
use crate::{Toks, read_char};

type Clips = HashMap<String, (u32, bool)>;
type V4 = [u32; 4];

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

/// The land everywhere; nothing pushes.
struct Land;

impl piney_battle::boss::PartHits for Land {
    fn switch(&mut self, _: usize, hit: &mut piney_battle::world::CharHit, on: bool) {
        hit.sw = on;
    }

    fn collide(&mut self, _: usize, hit: &mut piney_battle::world::CharHit) -> bool {
        hit.offset = [0; 4];
        false
    }

    fn sync(&mut self, _: usize, _: &piney_battle::world::CharHit) {}

    fn line(&mut self, _sp: V4, ep: V4, _mask: u32) -> Option<V4> {
        Some(ep)
    }
}

pub(crate) fn run(tables: &Tables, data: &BossData, clips: &Clips, dummy: V4, t: &mut Toks) -> String {
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
    let gen_life = t.int() as usize;
    let cam2 = (v4(t), v4(t));
    let acam = v4(t);
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, usize, i16, i16)> =
        (0..ns).map(|_| (t.int() as usize, t.i32(), t.int() as usize, t.i16(), t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    let mut blur = crate::boss::ProbeBlur::read(t);
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
    let fade = piney_battle::boss::CamFade { eye: acam, deg1: 0, eye_view: false, far: 0x45da_c000, fade: 0x4416_0000 };
    let mut cams = Cams { set: [(2, cam2)].into_iter().collect(), id: 1, fade: Some(fade), ..Cams::default() };
    let gens: RefCell<HashMap<u32, usize>> = RefCell::new(HashMap::new());
    let frame_no = std::cell::Cell::new(0usize);
    let alive = |k: u32| gens.borrow().get(&k).is_none_or(|&c| frame_no.get() - c < gen_life);
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let volume = crate::probe_volume();
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false, cc: None };
    let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
    let mut none = |_| None;
    let mut land = |p: V4| p[2];
    let mut hits = Land;
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
        part_hits: &mut hits,
        game_over: false,
        boss_cam: true,
        cam: cams.view(),
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        gen_alive: &alive,
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = third::new(&mut cx, kpos, kdirc, center, dummy);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    for c in me + 1..scene.chars.len() {
        scene.chars[c].affect.func = AffectFunc::Boss;
    }
    cams.apply(&out);
    note_gens(&out, &gens, 0);
    let outs: Vec<String> = out.iter().filter_map(mout).collect();
    blur.apply(&out);
    lines.push(with_cam(&with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu), &blur), &cams));
    for f in 0..frames {
        frame_no.set(f);
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
        let killers = match &boss.class {
            Class::CorbenikC(x) => x.killer_me,
            _ => [0; 3],
        };
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        let mut out = Vec::new();
        let cell = RefCell::new(std::mem::take(&mut cc));
        let benv_hits = BossEnv { cc: Some(&cell), ..benv };
        let actx_hits = AffectCtx { boss: Some(&benv_hits), ..actx };
        for &(_, kind, who, p0, p1) in script.iter().filter(|s| s.0 == f) {
            let actx = &actx_hits;
            let mut ev = Events::new();
            let kite = members[0];
            match kind {
                0 => affect::entry_affect(tables, &mut scene, actx, me, kite, 1, [p0, 0, 0], &mut rand, &mut ev),
                2 => affect::entry_affect(tables, &mut scene, actx, me, kite, 13, [0; 3], &mut rand, &mut ev),
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
                13 => {
                    scene.chars[me].skill_id = 0;
                    scene.chars[me].skill_status = 0;
                }
                14 => scene.chars[me].hp = p0,
                15 => {
                    let k = killers[who % 3];
                    affect::entry_affect(tables, &mut scene, actx, k, kite, 1, [p0, p1, 0], &mut rand, &mut ev)
                }
                16 => scene.chars[killers[who % 3]].hp = p0,
                _ => {}
            }
        }
        cc = cell.into_inner();
        if let Some(b) = scene.chars[me].foe_state_mut().and_then(|x| x.boss.as_mut()) {
            out.extend(b.base.take_pending());
        }
        for &k in &killers {
            if let Some(b) = scene.chars[k].foe_state_mut().and_then(|x| x.boss.as_mut()) {
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
            part_hits: &mut hits,
            game_over: false,
            boss_cam: true,
            cam: cams.view(),
            land: &mut land,
            disc: piney_battle::boss::DiscView { blur_exit: blur.blur.exit, ..Default::default() },
            gen_alive: &alive,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.main(&mut cx);
        all.extend(std::mem::take(&mut cx.out));
        drop(cx);
        blur.apply(&all[pre..]);
        cams.apply(&all);
        note_gens(&all, &gens, f);
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
        lines.push(with_cam(&with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu), &blur), &cams));
    }
    format!("[{}]", lines.join(","))
}

/// Each kept generator noted with the frame it was made in.
fn note_gens(out: &[Out], gens: &RefCell<HashMap<u32, usize>>, f: usize) {
    for o in out {
        if let Out::Corbenik(Pic::Gen { key, .. }) = o {
            gens.borrow_mut().insert(*key, f);
        }
    }
}

/// The boss camera beside the frame, as the harness's `bosscam`.
fn with_cam(line: &str, cams: &Cams) -> String {
    line.replacen("\"rand\":", &format!("\"bosscam\":{},\"rand\":", cams.boss.json()), 1)
}

fn third_json(b: &Boss, x: &Third) -> String {
    let lasers: Vec<String> = x.eff_laser.iter().map(|s| some(*s)).collect();
    let v: Vec<String> = vec![
        x.stage_eff.to_string(),
        x.pat_mode.to_string(),
        some(x.boss_eff),
        v3(&x.eye_center_offset),
        v3(&x.old_pos),
        u8::from(x.pause).to_string(),
        x.attack_slave.to_string(),
        x.slv_atk_mode.to_string(),
        u8::from(x.half).to_string(),
        v3(&x.dead_pos),
        v3(&x.act_pos),
        format!("[{}]", lasers.join(",")),
        b.base.stop_time.to_string(),
        b.base.stop_counter.to_string(),
        b.base.pat_index.to_string(),
        b.base.pat_num.to_string(),
    ];
    v.join(",")
}

fn eye_json(scene: &Scene, e: usize) -> String {
    let ch = &scene.chars[e];
    let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()) else { return "[]".into() };
    let Class::CorbenikEye(x) = &b.class else { return "[]".into() };
    let x: &Eye = x;
    format!(
        "[{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]",
        b.base.act_num,
        b.base.act_proccess,
        b.base.act_forbid,
        v3(&ch.pos),
        v3(&b.base.dirc),
        v3(&x.pos_b),
        v3(&x.rot_dirc),
        b.base.exit,
        b.base.draw_sw,
        b.base.transparency,
        b.base.set_transparency,
        x.color_blend,
        v3(&x.from),
        v3(&x.to),
        x.speed,
        x.step,
        v3(&x.act_pos),
    )
}

fn killer_json(scene: &Scene, k: usize) -> String {
    let ch = &scene.chars[k];
    let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()) else { return "[]".into() };
    let Class::CorbenikKiller(x) = &b.class else { return "[]".into() };
    let x: &Killer = x;
    format!(
        "[{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{},{},{},{}]",
        b.base.act_num,
        b.base.act_proccess,
        b.base.act_count,
        v3(&ch.pos),
        v3(&ch.pos_p),
        v3(&b.base.dirc),
        ch.hp,
        ch.max_hp,
        b.base.exit,
        b.base.draw_sw,
        b.base.body_hit_sw,
        b.base.act_forbid,
        b.base.transparency,
        b.base.set_transparency,
        b.base.move_spd,
        b.base.move_dirc,
        v3(&b.base.move_vector),
        ch.target_char.map_or(-1, |t| t as i64),
        u8::from(scene.listed(k)),
        u8::from(x.pause),
        v3(&x.spin),
        x.press_r,
        x.press_step,
        b.base.anm.clip.as_deref().unwrap_or(""),
        b.base.anm.frame(),
        b.base.stop_time,
        b.base.pat_index,
        v3(&x.act_pos),
    )
}

fn frame_json(
    b: &Boss,
    scene: &Scene,
    me: usize,
    outs: &[String],
    rand: &Rand,
    cc: &Genrand,
    menu: &[i32; 7],
) -> String {
    let Class::CorbenikC(x) = &b.class else { return "{}".into() };
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
    let eyes: Vec<String> = x.eye_me.iter().map(|&e| eye_json(scene, e)).collect();
    let killers: Vec<String> = x.killer_me.iter().map(|&k| killer_json(scene, k)).collect();
    let _ = laser::SEGS;
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{}],\"third\":[{}],\"eyes\":[{}],\"killers\":[{}],\"eff\":[{}],\
         \"party\":[{}],\"out\":[{}],\"menu\":[{},{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
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
        third_json(b, x),
        eyes.join(","),
        killers.join(","),
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
