//! `battle_probe` requests for Gorre (`piney_battle::boss::gorre`), as
//! `tools/test_battle_gorre_rs.py` sends them: `gorreclips` (the clips of
//! `x51` and `xeffect`) and `gorre ...`: Gorre and its two brothers made on
//! a scene, then FRAMES frames of `Main` with affects before each, as
//! `fidchell.rs`'s does. One JSON line: the state after the constructor,
//! then each frame; Gorre's own fields, then `br0`/`br1` (each a brother's
//! own [`piney_battle::boss::Boss`], taken out of its own scene character
//! for the frame and put back, [`piney_battle::boss::kyvia::Part`]'s way).

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::gorre;
use piney_battle::boss::gorre::brother::Brother;
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, Out};
use piney_battle::chara::{AffectFunc, Char, Env};
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
    for stem in [gorre::FILE, "xeffect"] {
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

/// Answers `gorreclips` and `gorre`; None for any other command.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    if !matches!(cmd, "gorreclips" | "gorre") {
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
        if cmd == "gorreclips" {
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
    let mut boss = gorre::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    let outs: Vec<String> = out.iter().filter_map(mout).collect();
    lines.push(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu));
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
        let outs: Vec<String> = all.iter().filter_map(mout).collect();
        lines.push(frame_json(&boss, &scene, me, &outs, &rand, &cc, &menu));
    }
    format!("[{}]", lines.join(","))
}

fn v3(v: &V4) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

/// A call as the harness records it: Gorre's own camera focuses
/// (`focus_camera`'s `CameraChange(3)`/`CameraPos`/`CameraView`), its
/// skill casts (its own and its brothers'), else the generic fallback
/// ([`out_json`]). `CamMaxRange` is not compared: under the stand-in
/// `InitBossCamera` (no real `ccBossCam` built), the game's own `MaxRenge`
/// measures 0 here, unlike Fidchell's 600 - the real mechanism setting it
/// is not yet found.
fn mout(o: &Out) -> Option<String> {
    Some(match o {
        Out::CameraChange(n) => format!("[\"cam\",{n}]"),
        Out::CameraPos { cam, pos } => format!("[\"campos\",{cam},{}]", v3(pos)),
        Out::CameraView { cam, view } => format!("[\"camview\",{cam},{}]", v3(view)),
        Out::CamMaxRange(_) => return None,
        Out::Skill(_, s) => format!("[\"skill\",{},{}]", s.sid, u8::from(s.stype == 2)),
        Out::SkillFrom { skill, .. } => format!("[\"skill\",{},{}]", skill.sid, u8::from(skill.stype == 2)),
        Out::SkillStart { sid, .. } => format!("[\"skill_start\",{sid}]"),
        Out::SeNote { se, note } => format!("[\"senote\",{se},{note}]"),
        _ => return out_json(o),
    })
}

/// One character's generic `ccBoss`/`ccChar` fields, the same shape for
/// Gorre and each brother (fidchell.rs's own `frame_json`'s, minus the
/// class-specific tail).
#[allow(clippy::too_many_arguments)]
fn part_json(b: &Boss, ch: &Char) -> String {
    let f = ch.foe_state().unwrap();
    format!(
        "\"act\":[{},{},{}],\"move\":[{},{},{}],\"tgt\":[{},{},{},{}],\"pos\":[{}],\"posp\":[{}],\"dirc\":[{}],\
         \"hp\":[{},{}],\"pp\":[{},{}],\"flags\":[{},{},{},{},{},{},{},{},{},{},{},{}],\"tr\":[{},{}],\
         \"anm\":[\"{}\",{},{}],\"pat\":[{},{},{}]",
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
        b.epitaph,
        b.transparency,
        b.set_transparency,
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        b.anm.speed,
        b.pat_index,
        b.pat_num,
        b.pat_mode,
    )
}

/// A brother's own tail: its formation offset (`m_posB`) and id.
fn brother_json(x: &Brother) -> String {
    format!("\"posb\":[{}],\"id\":{}", v3(&x.offset), x.id)
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
) -> String {
    let Class::Gorre(x) = &b.class else { return "{}".into() };
    // `x.brothers` is scratch (filled only for the span of `main`/`new`);
    // the persistent copy sits on each brother's own scene character
    // (`brother_me`, `kyvia::Part`'s way), fetched fresh here.
    let brother = |bm: usize| -> (&Boss, &Brother) {
        let gb = scene.chars[bm].foe_state().unwrap().boss.as_ref().unwrap();
        let Class::GorreBrother(bx) = &gb.class else { unreachable!("a brother's own character") };
        (gb, bx)
    };
    let (b0, x0) = brother(x.brother_me[0]);
    let (b1, x1) = brother(x.brother_me[1]);
    // The game's `_g_bossEffManager` is one array shared by Gorre and both
    // brothers; the port keeps each's own separate, so slot indices never
    // line up. Compared unordered instead: every (kind, enabled) alive.
    let mut eff_pairs: Vec<(i32, u8)> = [b, b0, b1]
        .iter()
        .flat_map(|p| p.effects.slots.iter())
        .flatten()
        .map(|e| (eff_num(e.kind), u8::from(e.enabled)))
        .collect();
    eff_pairs.sort_unstable();
    let effs: Vec<String> = eff_pairs.iter().map(|(k, e)| format!("[{k},{e}]")).collect();
    let party: Vec<String> = scene
        .pc_list
        .iter()
        .map(|&c| format!("[{},{},{}]", scene.chars[c].hp, scene.chars[c].cond[1], scene.chars[c].cond[0]))
        .collect();
    let gorre = part_json(b, &scene.chars[me]);
    let br0 = format!("{{{},{}}}", part_json(b0, &scene.chars[x.brother_me[0]]), brother_json(x0));
    let br1 = format!("{{{},{}}}", part_json(b1, &scene.chars[x.brother_me[1]]), brother_json(x1));
    format!(
        "{{{},\"br0\":{},\"br1\":{},\"eff\":[{}],\"party\":[{}],\"out\":[{}],\
         \"menu\":[{},{},{},{},{},{}],\"rand\":{},\"cc\":{}}}",
        gorre,
        br0,
        br1,
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
