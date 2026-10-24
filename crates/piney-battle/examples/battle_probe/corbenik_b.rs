//! `battle_probe`'s `corbenikb`: Corbenik's second form
//! (`piney_battle::boss::corbenik::second`) as `tools/test_battle_corbenik_b_rs.py`
//! sends it: the body and its five roots made on a scene, then FRAMES
//! frames of `Main` (its manager's pass first) with affects before each
//! (0 a hit, 5 and 6 the stop's affects, 2 the drain's 13, 7 a member's
//! `dead`, 8 an act forced, 9 a hit of type 3, 10 a hit of type 1 by skill
//! `who`, 13 its own skill over, 14 its HP set). A generator the form keeps
//! lives the case's frames. One JSON line: the state after the
//! constructor, then each frame.

use std::cell::RefCell;
use std::collections::HashMap;

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::corbenik::second::{self, Second};
use piney_battle::boss::corbenik::{self, Pic};
use piney_battle::boss::{Boss, BossData, BossEnv, Class, Cx, Effects, Out};
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

pub(crate) fn run(tables: &Tables, data: &BossData, clips: &Clips, t: &mut Toks) -> String {
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
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, usize, i16)> =
        (0..ns).map(|_| (t.int() as usize, t.i32(), t.int() as usize, t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    let mut blur = crate::boss::ProbeBlur::read(t);
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
    let party = Party { members, ids, num: members.iter().flatten().count() as i32 };
    let clip = |name: &str| clips.get(name).copied();
    let mut forbid = 0i16;
    let mut menu = [0i32; 7];
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let mut cams = Cams { set: [(2, cam2)].into_iter().collect(), id: 1, ty: 0 };
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
    let mut boss = second::new(&mut cx, kpos, kdirc, center, Effects::default());
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    cams.apply(&out);
    let outs: Vec<String> = out.iter().filter_map(mout).collect();
    blur.apply(&out);
    lines.push(with_blur(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu), &blur));
    for f in 0..frames {
        frame_no.set(f);
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume };
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        let mut out = Vec::new();
        let cell = RefCell::new(std::mem::take(&mut cc));
        let benv_hits = BossEnv { cc: Some(&cell), ..benv };
        let actx_hits = AffectCtx { boss: Some(&benv_hits), ..actx };
        for &(_, kind, who, p0) in script.iter().filter(|s| s.0 == f) {
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
                        (b.act_num, b.act_proccess, b.act_count) = (p0, 0, 0);
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
                _ => {}
            }
        }
        cc = cell.into_inner();
        if let Some(b) = scene.chars[me].foe_state_mut().and_then(|x| x.boss.as_mut()) {
            out.extend(b.take_pending());
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
                Out::Corbenik(Pic::Gen { key, .. }) => {
                    gens.borrow_mut().insert(*key, f);
                }
                _ => {}
            }
        }
        let outs: Vec<String> = all.iter().filter_map(mout).collect();
        lines.push(with_blur(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu), &blur));
    }
    format!("[{}]", lines.join(","))
}

fn second_json(b: &Boss, x: &Second) -> String {
    let rd = &x.rd;
    let v: Vec<String> = vec![
        x.stage_eff.to_string(),
        some(x.eff_star),
        some(x.boss_eff),
        u8::from(x.barrier).to_string(),
        u8::from(x.barrier_anm_status).to_string(),
        x.pat_mode.to_string(),
        x.root_transparency.to_string(),
        u8::from(x.reserve_setup_barrier).to_string(),
        u8::from(x.flash_barrier).to_string(),
        x.life_counter.to_string(),
        x.msg_count.to_string(),
        u8::from(x.msg_shown).to_string(),
        u8::from(x.time_up).to_string(),
        v3(&x.act_pos),
        b.m_act_dirc[0].to_string(),
        v3(&rd.point),
        rd.dir.to_string(),
        rd.spd.to_string(),
        rd.after.to_string(),
        rd.step.to_string(),
        b.stop_time.to_string(),
        b.stop_counter.to_string(),
        b.pat_index.to_string(),
        b.pat_num.to_string(),
        b.m_act_dirc[2].to_string(),
    ];
    v.join(",")
}

fn root_json(scene: &Scene, r: usize) -> String {
    let ch = &scene.chars[r];
    let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()) else { return "[]".into() };
    let Class::CorbenikRoot(x) = &b.class else { return "[]".into() };
    format!(
        "[{},{},{},{},{},{},{},{},\"{}\",{},{}]",
        b.act_num,
        b.act_proccess,
        b.act_count,
        v3(&ch.pos),
        v3(&b.dirc),
        b.exit,
        b.draw_sw,
        b.transparency,
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        x.quake_time,
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
    let Class::CorbenikB(x) = &b.class else { return "{}".into() };
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
    let roots: Vec<String> = x.root_me.iter().map(|&r| root_json(scene, r)).collect();
    let els: Vec<String> = x.elements.iter().map(|e| format!("[{},{},{}]", e.state, e.alpha, v3(&e.pos))).collect();
    let ais: Vec<String> =
        x.ai_elements.iter().map(|e| format!("[{},{},{},{}]", e.angle, e.turn, e.alpha, v3(&e.pos))).collect();
    let _ = corbenik::ROW;
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{}],\"second\":[{}],\"roots\":[{}],\"elements\":[{}],\"ai\":[{}],\"eff\":[{}],\
         \"party\":[{}],\"out\":[{}],\"menu\":[{},{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
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
        second_json(b, x),
        roots.join(","),
        els.join(","),
        ais.join(","),
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
