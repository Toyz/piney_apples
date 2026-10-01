//! Answers `tools/test_gamectrl_rs.py`: the battle half of `ccThGameCtrl`
//! (`piney_world::talk`) on the cases it is sent, one JSON line each.
//! Requests: `inbattle` (`ccGame::SetInBattle`), `inarea`
//! (`ccCheckInAreaCmnd`, `ccCheckTargetTypeId`), `select` (`ccSortCmnd`,
//! `ccCheckTargetRange`, `ccSelectTarget`), `ctrl_start` (a new task) and
//! `ctrl_frame` (`Targeting::frame`; see `ctrl_frame` below). Numbers hex,
//! floats their bits, signed values 32-bit two's complement.

use std::collections::HashMap;
use std::io::BufRead;

use piney_world::ee::{self, V4};
use piney_world::entry::Kind;
use piney_world::talk::{
    self, Battle, Cmnd, Cond, Ctrl, Host, InBattle, Input, Leader, LeaderState, Member, Party, RecoveryReqs, Scope,
    Targeting, Who,
};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[i64]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

/// A cursor over a request's numbers.
struct Args<'a> {
    w: &'a [&'a str],
    at: usize,
}

impl Args<'_> {
    fn u(&mut self) -> u32 {
        self.at += 1;
        hex(self.w[self.at - 1])
    }
    fn i(&mut self) -> i32 {
        self.u() as i32
    }
    fn s(&mut self) -> i16 {
        self.u() as i16
    }
    fn b(&mut self) -> bool {
        self.u() != 0
    }
    fn v3(&mut self) -> V4 {
        [self.u(), self.u(), self.u(), ee::ONE]
    }
    fn cond(&mut self) -> Cond {
        Cond { dead: self.s(), sleep: self.s(), confusion: self.s(), charm: self.s(), paralysis: self.s() }
    }
}

/// The kind a candidate's list gives it.
fn kind_of(flags: u32) -> Kind {
    match Cmnd::new(Kind::Npc, 0, flags, 0, ee::VF0).list() {
        0 => Kind::Spc,
        1 => Kind::Enemy,
        _ => Kind::Npc,
    }
}

/// The frame's host: `ccEvent::CheckOperate` from a mask, `ccSkillCheck`
/// as the test's plan has it (SKILL, then AFTER once the normal attack is
/// requested), the recoveries against who is listed and standing.
struct Plan<'a> {
    mask: u32,
    skill: i32,
    after: i32,
    attacks: Vec<i64>,
    reqs: &'a mut RecoveryReqs<i32>,
    alive: HashMap<i32, bool>,
    heals: Vec<(i32, i16)>,
}

impl Host for Plan<'_> {
    fn check_operate(&mut self, n: i32) -> bool {
        (self.mask >> n) & 1 != 0
    }
    fn skill_check(&mut self) -> i32 {
        self.skill
    }
    fn skill_request(&mut self, _: Kind, code: i32) {
        self.attacks.push(i64::from(code));
        self.skill = self.after;
    }
    fn recovery(&mut self) {
        let alive = &self.alive;
        let heals = &mut self.heals;
        self.reqs.ctrl(|ch| alive.get(&ch).copied().unwrap_or(false), |ch, n| heals.push((ch, n)));
    }
}

struct Task {
    t: Targeting,
    game: InBattle,
    reqs: RecoveryReqs<i32>,
}

fn new_task() -> Task {
    Task { t: Targeting::default(), game: InBattle::default(), reqs: RecoveryReqs::default() }
}

fn main() {
    let mut task = new_task();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let mut a = Args { w: &w, at: 1 };
        match w.first().copied() {
            Some("inbattle") => {
                let mut g = InBattle { in_battle: a.i(), cnt: a.i(), dist: InBattle::DIST };
                g.set(a.i());
                println!("[{}, {}]", g.in_battle, g.cnt);
            }
            Some("inarea") => {
                let leader = Leader { volume: piney_data::volume::Volume::Inf, pos_p: a.v3(), dirc: [0; 4], width: 0 };
                let (flags, listed, at, id, dead) = (a.u(), a.b(), a.u() as usize, a.s(), a.s());
                let cond = Cond { dead, ..Cond::default() };
                let kite = LeaderState { flags, listed, at, id, cond, ..LeaderState::default() };
                let (ckind, cidx, cx, cy) = (a.u(), a.u() as usize, a.u(), a.u());
                let (ty, mode, dist, tid) = (a.u(), a.i(), a.u(), a.s());
                let n = a.u() as usize;
                let cands: Vec<Cmnd> = (0..n)
                    .map(|k| {
                        let (flags, id, dead) = (a.u(), a.s(), a.s());
                        let mut c = Cmnd::new(kind_of(flags), k as i32, flags, 0, a.v3());
                        c.id = id;
                        c.cond.dead = dead;
                        c
                    })
                    .collect();
                let ch = match ckind {
                    0 => Member { pos_p: leader.pos_p, dead: 0, who: Who::Leader },
                    1 => Member { pos_p: cands[cidx].pos_p, dead: 0, who: Who::Cand(cidx) },
                    _ => Member { pos_p: [cx, cy, 0, ee::ONE], dead: 0, who: Who::Unlisted },
                };
                let inside = talk::in_area(&leader, &kite, &cands, &ch, ty, mode, dist);
                let found = talk::type_id_listed(&kite, &cands, ty, tid);
                println!("[{}, {}]", u8::from(inside), u8::from(found));
            }
            Some("select") => {
                let (pos_p, dircz, width) = (a.v3(), a.u(), a.u());
                let leader = Leader { volume: piney_data::volume::Volume::Inf, pos_p, dirc: [0, 0, dircz, 0], width };
                let kite = LeaderState { listed: a.b(), cond: a.cond(), ..LeaderState::default() };
                let (eye, mode, mut pri, in_battle, field) = (a.b(), a.i(), a.i(), a.b(), a.i());
                let n = a.u() as usize;
                let mut cands: Vec<Cmnd> = (0..n)
                    .map(|k| {
                        let (flags, width) = (a.u(), a.u());
                        let mut c = Cmnd::new(kind_of(flags), k as i32, flags, width, a.v3());
                        c.cond = a.cond();
                        c.act = a.s();
                        c
                    })
                    .collect();
                let sorted = talk::sort(&leader, &mut cands);
                let ranges: Vec<i64> =
                    cands.iter().map(|c| i64::from(talk::check_range(&leader, c, eye, in_battle))).collect();
                let scope = Scope { eye, in_battle, field };
                let t = talk::select_target(&leader, &kite, &cands, &sorted, mode, &mut pri, scope);
                let dd: Vec<String> = cands.iter().map(|c| format!("[{}, {}]", c.dist, c.dirc)).collect();
                println!(
                    "{{\"dd\": [{}], \"sorted\": {}, \"range\": {}, \"target\": {}, \"pri\": {}}}",
                    dd.join(", "),
                    list(&sorted.iter().map(|&i| i as i64).collect::<Vec<_>>()),
                    list(&ranges),
                    t.map_or(-1, |i| i as i64),
                    pri
                );
            }
            Some("ctrl_start") => {
                task = new_task();
                println!("{{}}");
            }
            // `ctrl_frame POWL EYE PUSH MENUIDLE FORBID FCE PLATT MENU
            // MENUCLOSED LOADDISP GTHACK COMPUL MENUCLEAR OPMASK SKILL AFTER
            // AREA FIELD DTYPE DIST PAUSE`, Kite `LISTED AT ID (DEAD SLEEP
            // CONF CHARM PARA) ACT CONTROL X Y Z DIRCZ WIDTH`, the party `NUM
            // (PRESENT IDX X Y Z DEAD)*3`, the recoveries asked before the
            // frame `N (IDX AMOUNT)*N`, the candidates in list order `N (IDX
            // FLAGS ID WIDTH X Y Z (DEAD SLEEP CONF CHARM PARA) ACT)*N`.
            // PUSH: 1 action, 2 personal, 4 chat, 8 option.
            Some("ctrl_frame") => ctrl_frame(&mut task, &mut a),
            _ => println!("null"),
        }
    }
}

fn ctrl_frame(task: &mut Task, a: &mut Args) {
    let (pow_l, eye, push, menu_idle) = (a.u() as u8, a.b(), a.u(), a.b());
    let (forbid, fce, pl_attack, menu, menu_closed) = (a.b(), a.b(), a.b(), a.s(), a.b());
    let (load_disp, gt_hack, compul, menu_clear) = (a.b(), a.b(), a.b(), a.b());
    let (mask, skill, after) = (a.u(), a.i(), a.i());
    let (area, field, dungeon_type, dist, pause) = (a.i(), a.i(), a.i(), a.u(), a.b());
    let kite = LeaderState {
        listed: a.b(),
        at: a.u() as usize,
        flags: 7,
        id: a.s(),
        cond: a.cond(),
        act: a.s(),
        control: a.b(),
    };
    let (pos_p, dircz, width) = (a.v3(), a.u(), a.u());
    let leader = Leader { volume: piney_data::volume::Volume::Inf, pos_p, dirc: [0, 0, dircz, 0], width };
    let num = a.i();
    let slots: Vec<Option<(i32, V4, i16)>> = (0..3)
        .map(|_| {
            let (present, idx, pos_p, dead) = (a.b(), a.i(), a.v3(), a.s());
            present.then_some((idx, pos_p, dead))
        })
        .collect();
    for _ in 0..a.u() {
        let (idx, amount) = (a.i(), a.i());
        task.reqs.entry(idx, amount);
    }
    let n = a.u() as usize;
    let mut cands: Vec<Cmnd> = (0..n)
        .map(|_| {
            let (idx, flags, id, width) = (a.i(), a.u(), a.s(), a.u());
            let mut c = Cmnd::new(kind_of(flags), idx, flags, width, a.v3());
            c.id = id;
            c.cond = a.cond();
            c.act = a.s();
            c
        })
        .collect();
    let party = Party {
        slots: std::array::from_fn(|k| {
            slots[k].map(|(idx, pos_p, dead)| Member {
                pos_p,
                dead,
                who: match cands.iter().position(|c| c.code == idx) {
                    _ if idx == 0 => Who::Leader,
                    Some(i) => Who::Cand(i),
                    None => Who::Unlisted,
                },
            })
        }),
        num,
    };
    let input = Input {
        pow_l,
        eye,
        action: push & 1 != 0,
        personal: push & 2 != 0,
        chat: push & 4 != 0,
        option: push & 8 != 0,
        player_ok: true,
        menu_idle,
        forbid,
        forbid_chat_except: fce,
        pl_attack,
        held: false,
        dead: false,
        skill_one: false,
        area,
        field,
        dungeon_type,
    };
    let b = Battle { kite, party, menu, menu_closed, load_disp, gt_hack, compulsion_game_over: compul };
    if menu_clear {
        task.t.menu_clear();
    }
    task.game.dist = dist;
    let mut alive: HashMap<i32, bool> = cands.iter().map(|c| (c.code, c.cond.dead == 0)).collect();
    alive.insert(0, kite.listed && kite.cond.dead == 0);
    let mut host = Plan { mask, skill, after, attacks: vec![], reqs: &mut task.reqs, alive, heals: vec![] };
    let out = task.t.frame(&leader, &mut cands, &input, &b, &mut task.game, &mut host);
    let who = |t: Option<(Kind, i32)>| t.map_or(-1, |(_, c)| i64::from(c));
    let req = match out.step {
        Some(Ctrl::Open(m)) => vec![i64::from(m), 0],
        Some(Ctrl::Action(x)) => vec![i64::from(x.menu), i64::from(talk::action_mode(x.menu, area))],
        _ => vec![],
    };
    let heals: Vec<String> = host.heals.iter().map(|(c, n)| format!("[{c}, {n}]")).collect();
    let t = &task.t;
    println!(
        "{{\"target\": {}, \"prev\": {}, \"pri\": {}, \"fix\": {}, \"pl_attack\": {}, \"pause\": {}, \
         \"in_battle\": {}, \"cnt\": {}, \"req\": {}, \"attack\": {}, \"heals\": [{}], \"over\": {}, \
         \"sorted\": {}}}",
        who(t.target),
        who(t.prev),
        t.pri,
        u8::from(t.fix),
        u8::from(out.pl_attack),
        u8::from(out.pause.unwrap_or(pause)),
        task.game.in_battle,
        task.game.cnt,
        list(&req),
        list(&host.attacks),
        heals.join(", "),
        u8::from(out.step == Some(Ctrl::GameOver)),
        list(&t.sorted.iter().map(|&(_, c, _, _)| i64::from(c)).collect::<Vec<_>>()),
    );
}
