//! `battle_probe` requests for the events' battle instructions
//! (`piney_battle::evparty`), as `tools/test_battle_event_rs.py` sends them.
//! Every request ends with a world (see [`read_world`]):
//!
//! ```text
//! evp OP ARGS... WORLD     one instruction: command pc on | walkpos pc x y z run |
//!                          walkdir pc rot dist | walkmarker pc marker |
//!                          walkchar pc ty code rot dist | put pc x y z |
//!                          partyput pc x y z | putmarker pc marker |
//!                          partyputmarker pc marker | enemyput enemy posnum |
//!                          turn pc dirc chg | face pc ty code chg |
//!                          remove ty code | holdop ty code running | holdend running
//! evhold TY CODE BOSSENTRY BOSSPARAM N WORLD FRAME*N
//!                          ccThEvHold from its start over N + 1 frames; a frame is
//!                          ND (WHO KIND VALUE...)*ND then the entry control's enemy list
//! evskill N ASSIGNOK MENU[5] WORLD FRAME*N
//!                          player_skill; a frame is PUSH SKILLCHECK TARGETDEAD NEWTARGET
//! battleready              battle_ready's inBattleDist
//! ```
//!
//! Replies are the world as the harness reads the game's (see [`state`]).

use std::cell::RefCell;

use piney_battle::affect::AffectCtx;
use piney_battle::chara::{Char, spc_flag};
use piney_battle::enemy_ai::Enemy;
use piney_battle::entry::{EntryCtrl, EvPos, Kind, Link, List, Out};
use piney_battle::evparty::{self, Boss, EvParty, Hold, Roster, SkillMenu};
use piney_battle::exp::Party;
use piney_battle::geom::{self, V4};
use piney_battle::param::{Base, cond};
use piney_battle::party_ai::{Ai, Crew};
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};

use crate::{Toks, list};

thread_local! {
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn record(s: String) {
    CALLS.with(|c| c.borrow_mut().push(s));
}

fn take_calls() -> Vec<String> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    Some(match cmd {
        "evp" => one(t),
        "evhold" => hold_run(t, tables),
        "evskill" => skill_run(t),
        "battleready" => {
            let mut d = 0x4509_8000;
            evparty::battle_ready(&mut d);
            format!("{{\"dist\":{d}}}")
        }
        _ => return None,
    })
}

/// The world of a check: the player's frame is the world moved by `d`
/// (`ccTransPosW2P` adds it, `ccTransPosP2W` takes it off, lane by lane);
/// `HitEnable` is recorded.
struct Probe {
    d: V4,
}

impl World for Probe {
    fn w2p(&mut self, pos: V4) -> V4 {
        std::array::from_fn(|i| geom::add(pos[i], self.d[i]))
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        std::array::from_fn(|i| geom::sub(pos[i], self.d[i]))
    }
    fn land(&mut self, _: V4, _: u32) -> u32 {
        0
    }
    fn hit_attribute(&mut self) -> u32 {
        0
    }
    fn line(&mut self, _: V4, _: V4, _: u32, _: i32) -> u32 {
        geom::MINUS_ONE
    }
    fn collide(&mut self, _: usize, _: &mut CharHit) -> i32 {
        0
    }
    fn hit_char_type(&mut self) -> u32 {
        0
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        record(format!("[\"{}\",{who}]", if on { "HitEnable" } else { "HitDisable" }));
        hit.sw = on;
    }
    fn camera_deg(&mut self, _: V4, _: i16) -> bool {
        true
    }
    fn camera_transparency(&mut self, _: V4, _: u32, _: u32, _: u32, _: u32) -> u32 {
        geom::ONE
    }
    fn anim_set(&mut self, _: usize, _: AnmSlot, _: &str) {}
    fn anim_frame(&mut self, _: usize, _: AnmSlot) -> u16 {
        0
    }
    fn anim_forward(&mut self, _: usize, _: AnmSlot, _: u16) -> i16 {
        0
    }
    fn anim_notes(&mut self, _: usize, _: AnmSlot) -> Vec<Note> {
        Vec::new()
    }
}

/// A check's state.
struct W {
    area: i32,
    kite: Option<usize>,
    target: Option<usize>,
    d: V4,
    roster: Roster,
    scene: Scene,
    crew: Crew,
    foes: Vec<Option<Enemy>>,
    ctrl: EntryCtrl,
    positions: Vec<EvPos>,
    markers: Vec<(V4, u32)>,
}

fn opt(v: i64) -> Option<usize> {
    usize::try_from(v).ok()
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

fn idx_list(t: &mut Toks) -> Vec<usize> {
    let n = t.int() as usize;
    (0..n).map(|_| t.int() as usize).collect()
}

/// `AREA KITE TARGET D[4]`, the roster (`ids[5] chars[5] members[3]
/// memberIDs[3] num`), `N` characters (`type id dead hold affectType pos[4]
/// posP[4] dirc[4] act actOld flagword cloak hitSW param2 enemy hasAI [ai
/// flags remoteCmd gDeg gRotSp gPoint gPos[4]]`), the command lists (party,
/// foes, objects: `n idx...` each), the entry control's enemy and NPC lists,
/// the event positions (`n (floor block num dirc pos[4])...`) and the town
/// markers (`n (pos[4] rotz)...`, by marker number).
fn read_world(t: &mut Toks) -> W {
    let area = t.i32();
    let kite = opt(t.int());
    let target = opt(t.int());
    let d = v4(t);
    let mut roster = Roster::default();
    for i in 0..5 {
        roster.ids[i] = t.i32();
    }
    for i in 0..5 {
        roster.chars[i] = opt(t.int());
    }
    let members: [Option<usize>; 3] = std::array::from_fn(|_| opt(t.int()));
    let ids: [i32; 3] = std::array::from_fn(|_| t.i32());
    roster.party = Party { members, ids, num: t.i32() };
    let n = t.int() as usize;
    let mut scene = Scene::default();
    let mut crew = Crew::default();
    let mut foes = Vec::new();
    for i in 0..n {
        let base = Base { ty: t.i32(), id: t.i16(), ..Base::default() };
        let mut c = Char::other(base, [0; 16]);
        c.cond[cond::DEAD] = t.i16();
        c.cond[cond::HOLD] = t.i16();
        c.affect.ty = t.i16();
        c.pos = v4(t);
        c.pos_p = v4(t);
        let dirc = v4(t);
        c.spc_char.act_num = t.i16();
        c.spc_char.act_num_old = t.i16();
        let w = t.u32();
        c.spc_char.flags = w & !(1 << 7 | 7 << 14);
        c.no_death = w >> 7 & 1 != 0;
        c.party_flag = (w >> 14 & 7) as i32;
        c.spc_char.cloak = t.u32();
        let sw = t.int() != 0;
        c.spc_char.hit_enabled = sw;
        let param2 = t.i32();
        let enemy = t.int() != 0;
        let has_ai = t.int() != 0;
        c.has_ai = has_ai;
        let f = c.spc_char.flags;
        let s = crew.spc.entry(i).or_default();
        s.dirc = dirc;
        s.act_num = c.spc_char.act_num;
        s.move_flag = f & spc_flag::MOVE != 0;
        s.stop_flag = f & spc_flag::STOP != 0;
        s.ghost = f & spc_flag::GHOST != 0;
        s.body_hit.sw = sw;
        let mut e = None;
        if enemy {
            let mut en = Enemy { dirc, ..Enemy::default() };
            en.ent.param[2] = param2;
            e = Some(en);
        }
        foes.push(e);
        if has_ai {
            let mut a = Ai::new(i, 0, 0, 150, &mut Rand(1));
            a.set_flags(t.u32());
            a.remote_cmd = t.i16();
            a.g_deg = t.int() as u16;
            a.g_rot_sp = t.i16();
            a.g_point = t.i16();
            a.g_pos = v4(t);
            crew.ais.insert(i, a);
        }
        scene.chars.push(c);
    }
    scene.pc_list = idx_list(t);
    scene.ene_list = idx_list(t);
    scene.obj_list = idx_list(t);
    let mut ctrl = EntryCtrl { links: vec![Link::default(); n], ..EntryCtrl::default() };
    set_list(&mut ctrl, Kind::Enemy, &idx_list(t));
    set_list(&mut ctrl, Kind::Npc, &idx_list(t));
    let np = t.int() as usize;
    let positions =
        (0..np).map(|_| EvPos { floor: t.i16(), block: t.i16(), num: t.i32(), dirc: t.u32(), pos: v4(t) }).collect();
    let nm = t.int() as usize;
    let markers = (0..nm).map(|_| (v4(t), t.u32())).collect();
    W { area, kite, target, d, roster, scene, crew, foes, ctrl, positions, markers }
}

/// A list of the entry control, head to foot, linked through `pre`/`next`.
fn set_list(ctrl: &mut EntryCtrl, k: Kind, v: &[usize]) {
    for (i, &c) in v.iter().enumerate() {
        ctrl.links[c] = Link { pre: i.checked_sub(1).map(|j| v[j]), next: v.get(i + 1).copied() };
    }
    ctrl.lists[k as usize] = List { num: v.len() as i32, head: v.first().copied(), foot: v.last().copied() };
}

fn ix(o: Option<usize>) -> i64 {
    o.map_or(-1, |v| v as i64)
}

/// Every character (the fields the instructions touch), the command lists
/// and the command target.
fn state(w: &W) -> String {
    let chars: Vec<String> = w
        .scene
        .chars
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let s = w.crew.spc.get(&i).copied().unwrap_or_default();
            let dirc = match &w.foes[i] {
                Some(e) => e.dirc,
                None => s.dirc,
            };
            let word = c.spc_char.flags | u32::from(c.no_death) << 7 | ((c.party_flag & 7) as u32) << 14;
            let ai = match w.crew.ais.get(&i) {
                Some(a) => format!(
                    "[{},{},{},{},{},{}]",
                    a.flags() & 0xff,
                    a.remote_cmd,
                    a.g_deg,
                    a.g_rot_sp,
                    a.g_point,
                    list(a.g_pos)
                ),
                None => "null".into(),
            };
            let param2 = w.foes[i].as_ref().map_or(0, |e| e.ent.param[2]);
            format!(
                "{{\"dead\":{},\"hold\":{},\"pos\":{},\"dirc\":{},\"act\":{},\"actold\":{},\"flags\":{},\
                 \"cloak\":{},\"hitsw\":{},\"param2\":{},\"ai\":{}}}",
                c.cond[cond::DEAD],
                c.cond[cond::HOLD],
                list(c.pos),
                list(dirc),
                c.spc_char.act_num,
                c.spc_char.act_num_old,
                word,
                c.spc_char.cloak,
                u8::from(s.body_hit.sw),
                param2,
                ai
            )
        })
        .collect();
    format!(
        "{{\"chars\":[{}],\"lists\":[{},{},{}],\"target\":{}}}",
        chars.join(","),
        list(w.scene.pc_list.iter()),
        list(w.scene.ene_list.iter()),
        list(w.scene.obj_list.iter()),
        ix(w.target)
    )
}

/// The command target after the `ccDeleteCmnd`s: `ccChangeCmndTarget(0)`
/// when the character taken off was it.
fn apply_outs(w: &mut W, outs: &[Out]) {
    for o in outs {
        if let Out::DeleteCmnd(c) = o
            && w.target == Some(*c)
        {
            w.target = None;
        }
    }
}

fn one(t: &mut Toks) -> String {
    let op = t.word().to_string();
    let args: Vec<i16> = match op.as_str() {
        "command" | "walkmarker" | "putmarker" | "partyputmarker" | "enemyput" | "remove" => {
            (0..2).map(|_| t.i16()).collect()
        }
        "walkdir" | "holdop" | "turn" => (0..3).map(|_| t.i16()).collect(),
        "walkpos" | "walkchar" => (0..5).map(|_| t.i16()).collect(),
        "put" | "partyput" | "face" => (0..4).map(|_| t.i16()).collect(),
        "holdend" => vec![t.i16()],
        _ => panic!("unknown evp op {op}"),
    };
    let mut w = read_world(t);
    let mut outs = Vec::new();
    let mut extra = String::new();
    take_calls();
    {
        let markers = w.markers.clone();
        let town = move |m: i16| {
            record(format!("[\"marker\",{m}]"));
            usize::try_from(m).ok().and_then(|k| markers.get(k).copied())
        };
        let mut world = Probe { d: w.d };
        let mut ev = EvParty {
            scene: &mut w.scene,
            crew: &mut w.crew,
            roster: &w.roster,
            ctrl: &w.ctrl,
            foes: &mut w.foes,
            world: &mut world,
            area: w.area,
            positions: &w.positions,
            town_marker: &town,
            out: &mut outs,
        };
        let a = &args;
        match op.as_str() {
            "command" => {
                ev.pc_command(a[0], a[1]);
            }
            "walkpos" => {
                ev.pc_walk_pos(a[0], a[1], a[2], a[3], a[4] != 0);
            }
            "walkdir" => {
                ev.pc_walk_dir(a[0], a[1], a[2]);
            }
            "walkmarker" => {
                ev.pc_walk_marker(a[0], a[1]);
            }
            "walkchar" => {
                ev.pc_walk_char(a[0], a[1], a[2], a[3], a[4]);
            }
            "put" => {
                ev.pc_put(a[0], a[1], a[2], a[3]);
            }
            "partyput" => {
                ev.party_put(a[0], a[1], a[2], a[3]);
            }
            "putmarker" => {
                ev.pc_put_marker(a[0], a[1]);
            }
            "partyputmarker" => {
                ev.party_put_marker(a[0], a[1]);
            }
            "enemyput" => {
                ev.enemy_put(a[0], a[1]);
            }
            "turn" => {
                ev.pc_turn(a[0], a[1], a[2]);
            }
            "face" => {
                ev.pc_face(a[0], a[1], a[2], a[3]);
            }
            "remove" => {
                // deleteEnemy's choice (the deletion is EntryCtrl::delete_enemy's).
                if a[0] == 5 || a[0] == 6 {
                    let e = evparty::get_enemy(ev.ctrl, ev.scene, i32::from(a[1]));
                    if let Some(e) = e {
                        record(format!("[\"deleteEnemy\",{e}]"));
                    }
                }
            }
            "holdop" | "holdend" => {
                // The last argument: a task runs already (type 99, code 99).
                let running = a[a.len() - 1] != 0;
                let mut task = running.then_some(Hold { ty: 99, code: 99, armed: true });
                // The task's start (ccStartThread(ccThEvHold, 33, 2048)) and stop
                // (ccDeleteThread) are the runtime's; recorded as the harness records them.
                if op == "holdop" {
                    evparty::hold(&mut task, a[0], a[1]);
                    if !running {
                        record("[\"start\",33,2048]".into());
                    }
                } else {
                    evparty::hold_end(&mut task);
                    if running {
                        record("[\"delete\"]".into());
                    }
                }
                extra = match task {
                    Some(h) => format!(",\"task\":[{},{}]", h.ty, h.code),
                    None => ",\"task\":null".into(),
                };
            }
            _ => unreachable!(),
        }
    }
    apply_outs(&mut w, &outs);
    let s = state(&w);
    let calls = take_calls();
    format!("{{\"state\":{s},\"calls\":[{}]{extra}}}", calls.join(","))
}

/// `WHO KIND VALUE...`: 0 `hold` v, 1 `pos` v[4], 2 `affectType` v, 3 off
/// the foes' command list, 4 onto it (at its end).
fn apply_delta(w: &mut W, t: &mut Toks) {
    let n = t.int() as usize;
    for _ in 0..n {
        let who = t.int() as usize;
        match t.int() {
            0 => w.scene.chars[who].cond[cond::HOLD] = t.i16(),
            1 => w.scene.chars[who].pos = v4(t),
            2 => w.scene.chars[who].affect.ty = t.i16(),
            3 => w.scene.ene_list.retain(|&c| c != who),
            4 => {
                if !w.scene.listed(who) {
                    w.scene.ene_list.push(who);
                }
            }
            k => panic!("unknown delta {k}"),
        }
    }
    let v = idx_list(t);
    set_list(&mut w.ctrl, Kind::Enemy, &v);
}

fn hold_run(t: &mut Toks, tables: &Tables) -> String {
    let (ty, code) = (t.i16(), t.i16());
    let boss = Boss {
        entry: t.i32(),
        task_param: {
            let p = t.int();
            if p == -1 { None } else { Some(p as i32) }
        },
    };
    let n = t.int() as usize;
    let mut w = read_world(t);
    let mut task = None;
    evparty::hold(&mut task, ty, code);
    let mut task = task.expect("started");
    let party = w.roster.party;
    let skill_check = |_: usize| 0;
    let ctx =
        AffectCtx { party: &party, menu: true, skill_check: &skill_check, boss: None, volume: crate::probe_volume() };
    let mut rng = Rand(1);
    let mut frames = Vec::new();
    for f in 0..=n {
        if f > 0 {
            apply_delta(&mut w, t);
        }
        let mut ev = Vec::new();
        evparty::hold_frame(
            &mut task,
            tables,
            &mut w.scene,
            &w.ctrl,
            &mut w.foes,
            w.kite,
            boss,
            &ctx,
            &mut rng,
            &mut ev,
        );
        frames.push(state(&w));
    }
    format!("{{\"frames\":[{}]}}", frames.join(","))
}

fn skill_run(t: &mut Toks) -> String {
    let n = t.int() as usize;
    let assign_ok = t.i16();
    let mut menu = SkillMenu {
        panel_status: t.i16(),
        map_status: t.i16(),
        pl_attack: t.i16(),
        target_forbid: t.i16(),
        cmnd_target_fix: t.i32(),
    };
    let mut w = read_world(t);
    let mut frames = Vec::new();
    let snap = |w: &W, menu: &SkillMenu, calls: &[String]| {
        let p2 = w.target.and_then(|c| w.foes[c].as_ref()).map_or(0, |e| e.ent.param[2]);
        format!(
            "{{\"menu\":[{},{},{},{},{}],\"target\":{},\"param2\":{},\"calls\":[{}]}}",
            menu.panel_status,
            menu.map_status,
            menu.pl_attack,
            menu.target_forbid,
            menu.cmnd_target_fix,
            ix(w.target),
            p2,
            calls.join(",")
        )
    };
    evparty::player_skill_begin(&mut menu, w.target, &mut w.ctrl, &mut w.foes);
    let mut calls = Vec::new();
    let mut done = !evparty::player_skill_busy(&w.scene, w.target);
    if done {
        evparty::player_skill_end(&mut menu);
    }
    frames.push(snap(&w, &menu, &calls));
    for f in 1..=n {
        let (push, check, dead, new_target) = (t.u32(), t.i32(), t.i16(), t.int());
        if done {
            continue;
        }
        if new_target >= 0 {
            w.target = Some(new_target as usize);
        }
        if let Some(c) = w.target {
            w.scene.chars[c].cond[cond::DEAD] = dead;
        }
        calls.push(format!("[\"ccSkillCheck\",{}]", ix(w.kite)));
        let step = evparty::player_skill_step(check, push, assign_ok);
        if evparty::player_skill_apply(&mut menu, step) {
            calls.push(format!("[\"ccSkillRequest\",{},{},1,{f}]", ix(w.kite), ix(w.target)));
        }
        if !evparty::player_skill_busy(&w.scene, w.target) {
            evparty::player_skill_end(&mut menu);
            done = true;
        }
        frames.push(snap(&w, &menu, &calls));
    }
    format!("{{\"frames\":[{}]}}", frames.join(","))
}
