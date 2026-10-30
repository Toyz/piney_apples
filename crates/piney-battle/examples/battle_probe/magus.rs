//! `battle_probe` requests for Magus (`piney_battle::boss::magus`), as
//! `tools/test_battle_magus_rs.py` sends them: `magusclips` (the clips of
//! `x31` and `xeffect`) and `magus ...`: the boss and its twelve leaves made
//! on a scene, then FRAMES frames of the manager's pass and `Main` with
//! affects before each (0 a hit on Magus, 1 the gauge, 2 affect 13, 3
//! affect 21, 4 the break count, 5 a hit on a leaf). The leaves hang at the
//! case's places; a shock's beam misses on the case's frames. One JSON line:
//! the state after the constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::magus::leaf::Leaf;
use piney_battle::boss::magus::{self, Gen, Held, Magus, Pic, Place};
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, EffKind, GenOp, LoopOp, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::Events;
use piney_battle::exp::Party;
use piney_battle::param::cond;
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
    for stem in ["x31", "xeffect"] {
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

/// Answers `magusclips` and `magus`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "magusclips" | "magus") {
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
        if cmd == "magusclips" {
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

/// Whether shock `k`'s beam meets the ground on frame `f`: all but every
/// `miss`th (none for 0), as the harness's `ccHitCheckLM` answers.
fn hits(miss: i64, f: usize) -> [bool; 12] {
    std::array::from_fn(|k| !(miss > 0 && (f as i64 + k as i64) % miss == 0))
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
    let miss = t.int();
    let push = (t.int() as usize, t.int() as usize);
    let leaf_pos: [V4; 12] = std::array::from_fn(|_| [t.u32(), t.u32(), t.u32(), 0x3f80_0000]);
    let shock_pos: [V4; 12] = std::array::from_fn(|_| [t.u32(), t.u32(), t.u32(), 0x3f80_0000]);
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
    let mut menu = [0i32; 6];
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let view = CamView { fade, ..CamView::default() };
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false, cc: None };
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
        part_hits: &mut piney_battle::boss::NoHits,
        game_over: false,
        boss_cam: false,
        cam: view,
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        gen_alive: &|_| true,
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = magus::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    HELD.with(|h| h.borrow_mut().clear());
    if let Class::Magus(x) = &mut boss.class {
        x.shock_pos = shock_pos;
        LEAVES.with(|l| *l.borrow_mut() = x.leaves.clone());
    }
    let outs: Vec<String> = out.iter().filter_map(mout).collect();
    blur.apply(&out);
    lines.push(with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu), &blur));
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
        if let Class::Magus(x) = &mut boss.class {
            x.leaf_pos = leaf_pos;
            x.shock_pos = shock_pos;
            let hit = hits(miss, f);
            x.shock_at = std::array::from_fn(|k| hit[k].then_some(shock_pos[k]));
        }
        let leaves = match &boss.class {
            Class::Magus(x) => x.leaves.clone(),
            _ => Vec::new(),
        };
        LEAVES.with(|l| *l.borrow_mut() = leaves.clone());
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
                5 => {
                    let Some(&l) = leaves.get(who) else { continue };
                    let up = scene.listed(l)
                        && scene.chars[l].cond[cond::DEAD] == 0
                        && scene.chars[l].foe_state().and_then(|x| x.boss.as_ref()).is_some_and(|b| b.base.exit == 0);
                    if !up {
                        continue;
                    }
                    affect::entry_affect(tables, &mut scene, &actx, l, kite, 1, [p0, 0, 0], &mut rand, &mut ev);
                    if let Some(b) = scene.chars[l].foe_state_mut().and_then(|x| x.boss.as_mut()) {
                        out.extend(b.base.take_pending());
                    }
                }
                // Magus's own skill over.
                6 => {
                    scene.chars[me].skill_id = 0;
                    scene.chars[me].skill_status = 0;
                }
                // Forced, outside the game's reach: m_bDropping 0, the
                // laser reserved (act 30's way).
                7 => {
                    if let Some(Class::Magus(x)) =
                        scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
                    {
                        x.dropping = 0;
                        x.reserve_laser = 1;
                    }
                }
                _ => {}
            }
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
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
            part_hits: &mut piney_battle::boss::NoHits,
            game_over: false,
            boss_cam: false,
            cam: view,
            land: &mut land,
            disc: piney_battle::boss::DiscView::default(),
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
        let outs: Vec<String> = all.iter().filter_map(mout).collect();
        lines.push(with_blur(&frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu), &blur));
    }
    format!("[{}]", lines.join(","))
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

/// A call as the harness records it.
fn mout(o: &Out) -> Option<String> {
    Some(match o {
        Out::Magus(p) => match p {
            Pic::Particles { which, at, held } => gen_json(*which, at, *held),
            Pic::Gen { held, op } => return gen_op_json(*held, *op),
            Pic::Count { n, .. } => format!("[\"count\",{n}]"),
            Pic::Dust { .. } => "[\"dust\"]".into(),
            Pic::ShockBurst { turn, .. } => format!("[\"shock_burst\",{turn}]"),
            Pic::LeafAfterImage { frame, .. } => format!("[\"afterimage_leaf\",{frame}]"),
            Pic::Shocks { .. } => return None,
        },
        Out::CameraChange(n) => format!("[\"cam\",{n}]"),
        Out::CameraPos { cam, pos } => format!("[\"campos\",{cam},{}]", v3(pos)),
        Out::CameraView { cam, view } => format!("[\"camview\",{cam},{}]", v3(view)),
        Out::CameraShake { .. } => "[\"shake\"]".into(),
        Out::Skill(_, s) => format!("[\"skill\",{},{}]", s.sid, u8::from(s.stype == 2)),
        // BeginDeadEffect with no boss camera: camera 3 at the eye.
        Out::DeadCamera { eye, view, .. } => {
            format!("[\"cam\",3],[\"campos\",3,{}],[\"camview\",3,{}]", v3(eye), v3(view))
        }
        Out::FlashFade { t_in, t_out, hold: None, colour } => format!("[\"flash2\",{t_in},{t_out},{colour}]"),
        Out::FlashFade { t_in, t_out, hold: Some(h), colour } => {
            format!("[\"flash3\",{t_in},{t_out},{h},{colour}]")
        }
        Out::SeNote { se, note } => format!("[\"senote\",{se},{note}]"),
        Out::SeLoop { se, op: LoopOp::On(_) } => format!("[\"seloop\",{se}]"),
        Out::SeLoop { se, op: LoopOp::Off { .. } } => format!("[\"seoff\",{se}]"),
        // The needles run natively in the harness: seen in the slots.
        Out::Effect { kind: EffKind::Needle { .. }, .. } => return None,
        _ => return out_json(o),
    })
}

fn magus_json(b: &Boss, x: &Magus) -> String {
    let v: Vec<String> = vec![
        x.act_cnt.to_string(),
        x.is_neutral_anm.to_string(),
        x.atk_wait.to_string(),
        x.prev_rise.to_string(),
        x.laser_flash.to_string(),
        x.grow_interval.to_string(),
        x.epitaph.to_string(),
        x.cam_id.to_string(),
        x.shock_idx.to_string(),
        x.prepare_explode_all.to_string(),
        x.dropping.to_string(),
        x.reserve_laser.to_string(),
        x.explosion.to_string(),
        x.dmg_count.to_string(),
        x.high_drive.to_string(),
        x.reserve_leaf_drop.to_string(),
        x.explode_leaf_num.to_string(),
        x.laser_count.to_string(),
        u8::from(x.grow_se).to_string(),
        x.stuck.to_string(),
        x.pushed.to_string(),
        x.stage_id.to_string(),
        x.regrow.to_string(),
        x.blur_rad.to_string(),
        x.blur_scale.to_string(),
        x.no_leaf.to_string(),
        x.cam_count.to_string(),
        v3(&x.prev_laser),
        v3(&x.prev_dirc),
        v3(&x.act_pos),
        v3(&b.base.m_act_dirc),
        x.animate_chase.transparency.to_string(),
        b.base.stop_time.to_string(),
        b.base.stop_counter.to_string(),
        b.base.pat_index.to_string(),
        b.base.pat_num.to_string(),
    ];
    v.join(",")
}

fn leaf_json(b: &Boss, x: &Leaf, scene: &Scene, who: usize) -> String {
    let ch = &scene.chars[who];
    let d = |s: &piney_battle::boss::magus::leaf::DrawStatus| {
        format!("{},{},{},{},{},{}", s.alpha, s.base_alpha, s.alpha_spd, s.scale_max, s.scale_spd, s.scale)
    };
    format!(
        "[{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\
         {},{},{},{},{},{},{},{},{},{},{},{}]",
        b.base.exit,
        b.base.draw_sw,
        b.base.act_num,
        b.base.act_proccess,
        b.base.act_count,
        v3(&ch.pos),
        v3(&b.base.dirc),
        ch.hp,
        ch.max_hp,
        b.base.act_forbid,
        b.base.anm_status,
        u8::from(scene.listed(who)),
        b.base.anm.clip.as_deref().unwrap_or(""),
        b.base.anm.frame(),
        b.base.transparency,
        b.base.set_transparency,
        x.order_num,
        x.act_cnt,
        x.anm_end,
        x.exp_cntr,
        x.exp_alpha,
        x.exp_alpha_spd,
        x.exp_base_alpha,
        x.exp_scale,
        x.exp_scale_spd,
        v3(&x.vec_rot),
        x.prepare_explode,
        x.drop,
        x.blink_interval,
        x.blink_base_time,
        x.blink,
        x.explode,
        x.shock_scale,
        x.transparency,
        x.trans_spd,
        x.exit_alpha,
        x.marker_forbid,
        u8::from(x.marker_init),
        x.marker_alpha,
        d(&x.exp.eff),
        d(&x.exp.exp),
        u8::from(x.charge),
        v3(&x.marker_pos),
        paused(Held::Marker(who, 0)),
        paused(Held::Marker(who, 1)),
    )
}

thread_local! {
    /// The leaves' characters by slave, and the generators the leaves
    /// keep with their `pauseFlag` (kills print only for those kept).
    static LEAVES: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    static HELD: std::cell::RefCell<std::collections::HashMap<Held, bool>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn leaf_no(c: usize) -> i64 {
    LEAVES.with(|l| l.borrow().iter().position(|&x| x == c).map_or(-1, |k| k as i64))
}

fn held_json(h: Held) -> String {
    match h {
        Held::Marker(c, j) => format!("[\"marker\",{},{j}]", leaf_no(c)),
        Held::Charge(c) => format!("[\"charge\",{}]", leaf_no(c)),
    }
}

/// A kept generator's `pauseFlag` as the fight's calls left it.
fn paused(h: Held) -> u8 {
    HELD.with(|s| u8::from(s.borrow().get(&h).copied().unwrap_or(false)))
}

/// A start as the harness reads it: the table and row, the force fields'
/// rows, distSW, gSync (1 where the maker sets it), pTexMod, where it stands
/// and who keeps it.
fn gen_json(which: Gen, at: &Place, held: Option<Held>) -> String {
    let r = which.rows();
    let ff: Vec<String> = r.ff.iter().map(|f| f.to_string()).collect();
    let place = match *at {
        Place::At(p) => format!("[\"at\",{}]", v3(&p)),
        Place::Marker(c) => format!("[\"marker\",{}]", leaf_no(c)),
    };
    let held = held.map_or("null".into(), |h| {
        HELD.with(|s| s.borrow_mut().insert(h, false));
        held_json(h)
    });
    let g_sync = r.g_sync.map_or(0, i32::from);
    format!(
        "[\"particles\",\"{}\",{},[{}],{},{g_sync},{},{place},{held}]",
        r.table,
        r.row,
        ff.join(","),
        u8::from(r.dist_sw),
        r.tex
    )
}

fn gen_op_json(held: Held, op: GenOp) -> Option<String> {
    match op {
        GenOp::Kill => HELD.with(|s| s.borrow().contains_key(&held)).then(|| format!("[\"kill\",{}]", held_json(held))),
        GenOp::Pause(on) => {
            HELD.with(|s| {
                if let Some(p) = s.borrow_mut().get_mut(&held) {
                    *p = on;
                }
            });
            None
        }
        GenOp::Forget => {
            HELD.with(|s| s.borrow_mut().remove(&held));
            None
        }
    }
}

fn frame_json(
    b: &Boss,
    scene: &Scene,
    me: usize,
    outs: &[String],
    rand: &Rand,
    cc: &Genrand,
    menu: &[i32; 6],
) -> String {
    let Class::Magus(x) = &b.class else { return "{}".into() };
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
    let shocks: Vec<String> =
        x.shocks.iter().map(|s| format!("[{},{},{},{}]", s.index, s.transparency, s.draw, s.act_count)).collect();
    let leaves: Vec<String> = x
        .leaves
        .iter()
        .map(|&l| {
            let lb = scene.chars[l].foe_state().and_then(|f| f.boss.as_ref()).unwrap();
            let Class::MagusLeaf(lx) = &lb.class else { return "[]".into() };
            leaf_json(lb, lx, scene, l)
        })
        .collect();
    format!(
        "{{\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"pp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{},\"{}\",{},{}],\"magus\":[{}],\"shocks\":[{}],\"leaves\":[{}],\"eff\":[{}],\
         \"party\":[{}],\"out\":[{}],\"menu\":[{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
        b.base.act_num,
        b.base.act_proccess,
        b.base.act_count,
        b.base.move_spd,
        b.base.move_dirc,
        v3(&b.base.move_vector),
        ch.target_char.map_or(-1, |t| t as i64),
        b.base.target_dist,
        b.base.target_dirc,
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
        magus_json(b, x),
        shocks.join(","),
        leaves.join(","),
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
