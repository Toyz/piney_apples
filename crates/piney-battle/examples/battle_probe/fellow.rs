//! `battle_probe` requests for a party member's frame and its following
//! (`piney_battle::fellow`, `piney_battle::follow`), as
//! `tools/test_battle_fellow_rs.py` sends them: `fel FN NARGS ARGS... WORLD
//! FELLOW SCRIPT [NAVI] [RUN]`. `WORLD` is the party AI harness's world,
//! `FELLOW` each character's state ([`read_fellow`]), `SCRIPT` the world's
//! answers. `Run` performs the following ([`Follow`]) and records the rest;
//! `RunNavi` runs the party's movement as the game runtime will ([`Movement`]
//! over [`Share`]), with `NAVI` the members' `ccNavi`s and the dungeon's map.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::OnceLock;

use piney_battle::chara::{Char, Env};
use piney_battle::exp::Party;
use piney_battle::fellow::{self, FellowWorld, Field, Frame, MotionTables, Out};
use piney_battle::follow::{self, Follow, FollowState};
use piney_battle::geom::V4;
use piney_battle::navi::{Navi, NaviWorld};
use piney_battle::party_ai::*;
use piney_battle::party_motion::{Keep, Movement, Share};
use piney_battle::rand::{Rand, Rng};
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};
use piney_data::iso::Iso;
use piney_data::save::SaveData;

use crate::party_ai::{ai_json, msg_json, opt, read_ai, read_msg, show};
use crate::{Names, Toks, calls, list, read_char, read_env, state};

fn motion_tables() -> &'static MotionTables {
    static T: OnceLock<MotionTables> = OnceLock::new();
    T.get_or_init(|| {
        let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
        let mut iso = Iso::open(&path).expect("the disc image");
        MotionTables::of(iso.volume().expect("the volume"))
    })
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

fn who(names: &[String], c: Option<usize>) -> String {
    match c {
        Some(i) => format!("\"{}\"", names[i]),
        None => "0".into(),
    }
}

// the decisions' world, as test_battle_party_ai_rs.py serialises it ---------------------

fn sys_json(s: &AiSystem) -> String {
    format!(
        "[{},{},{},{},{},{},{}]",
        s.time,
        s.entry_num,
        s.history_top,
        s.next_id,
        list(s.msg_buff.iter().map(msg_json)),
        list(s.msg_history.iter().map(msg_json)),
        list(s.entry.iter().map(|&e| show(e)))
    )
}

/// A character's `ccSpcChar` fields in the decisions' shape; a foe's flag
/// bits are its `Char`'s (the rules' affects write those).
fn spc_json(s: &Spc, c: &Char) -> String {
    let mut s = *s;
    if c.ty() & 7 == 0 {
        let f = c.spc_char.flags;
        s.move_flag = f & 8 != 0;
        s.stop_flag = f & 0x10 != 0;
        s.run_flag = f & 0x20 != 0;
        s.ghost = f & 0x40 != 0;
    }
    format!(
        "[{},{},{},{},{},{},{},{},{},{},{},{}]",
        s.act_num,
        show(s.target_char),
        u8::from(s.move_flag),
        s.now_speed,
        s.cycle,
        s.dist_tg,
        s.spc_list_num,
        u8::from(s.run_flag),
        u8::from(s.ghost),
        u8::from(s.stop_flag),
        s.stop_cnt,
        s.walk_run_cnt
    )
}

/// The runtime of a check: the decisions' calls recorded as the harness
/// records the game's, the navigation's queries answered from the
/// scripts, and `ccSkillRequest` as the harness's model: sid 0 ends a
/// running normal attack (`skillStatus` 0), another sets the skill, its
/// status 1 and the target.
struct Script {
    names: Vec<String>,
    hit: VecDeque<u32>,
    goal: VecDeque<i32>,
    calls: Vec<String>,
    model: bool,
}

impl Runtime for Script {
    fn call(&mut self, c: Call, scene: &mut Scene, crew: &mut Crew, _rng: &mut dyn Rng) -> i32 {
        let n = &self.names;
        let (line, ret) = match c {
            Call::SkillRequest { me, target, sid } => {
                if self.model {
                    let ch = &mut scene.chars[me];
                    if sid == 0 {
                        if ch.skill_id == 1 && ch.skill_status != 0 {
                            ch.skill_status = 0;
                        }
                    } else {
                        ch.skill_id = sid as i16;
                        ch.skill_status = 1;
                        ch.target_char = target;
                        crew.spc.entry(me).or_default().target_char = target;
                    }
                }
                (format!("[\"ccSkillRequest\",{},{},{}]", who(n, Some(me)), who(n, target), sid), 0)
            }
            Call::UseItemRequest { me, target, item, flag } => {
                (format!("[\"ccUseItemRequest\",{},{},{},{}]", who(n, Some(me)), who(n, Some(target)), item, flag), 0)
            }
            Call::FollowTarget { me, target } => {
                (format!("[\"FollowTarget\",{},{}]", who(n, Some(me)), who(n, target)), 0)
            }
            Call::FollowTargetDirc { me, target } => {
                (format!("[\"FollowTargetDirc\",{},{}]", who(n, Some(me)), who(n, target)), 0)
            }
            Call::FollowPlayer { me } => (format!("[\"FollowPlayer\",{}]", who(n, Some(me))), 0),
            Call::LeavePlayer { me } => (format!("[\"LeavePlayer\",{}]", who(n, Some(me))), 0),
            Call::PathFinding { me, from, to } => {
                let r = self.goal.pop_front().unwrap_or(0);
                (format!("[\"PathFinding\",{},{},{}]", who(n, Some(me)), list(from), list(to)), r)
            }
            Call::FollowBeacon { me } => (format!("[\"FollowBeacon\",{}]", who(n, Some(me))), 0),
            Call::GoalBeacon { me, pos } => {
                let r = self.goal.pop_front().unwrap_or(0);
                (format!("[\"CheckGoalBeaconPos\",{},{}]", who(n, Some(me)), list(pos)), r)
            }
            Call::PlayerAttack { me, target, n: k } => {
                (format!("[\"PlayerAttack\",{},{},{}]", who(n, Some(me)), who(n, Some(target)), k), 0)
            }
            Call::HitCheckLm { from, to, mask } => {
                let r = self.hit.pop_front().unwrap_or(F_MINUS_ONE);
                (format!("[\"ccHitCheckLM\",{},{},{}]", list(from), list(to), mask), r as i32)
            }
            Call::FaceTalk { me, g_deg } => (format!("[\"FaceTalk\",{},{}]", who(n, Some(me)), g_deg as i16), 0),
            Call::ManualControl { me } => (format!("[\"ManualControl\",{}]", who(n, Some(me))), 0),
            Call::ActInTown { me } => (format!("[\"ActInTown\",{}]", who(n, Some(me))), 0),
            Call::ChatMessageSender { me } => (format!("[\"ChatMessageSender\",{}]", who(n, Some(me))), 0),
            Call::ManualModeAi { ch, ev } => (format!("[\"ManualModeAI\",{},{}]", who(n, Some(ch)), ev), 0),
            Call::TransferOut { ch } => (format!("[\"TransferOut\",{}]", who(n, Some(ch))), 0),
            // The AI's own movers' calls (crate::ai_move), which no fellow
            // check reaches.
            Call::TransferIn { ch } => (format!("[\"TransferIn\",{}]", who(n, Some(ch))), 0),
            Call::ResignParty { id } => (format!("[\"resignParty\",{id}]"), 0),
            Call::DisbandSpc { id } => (format!("[\"disbandSpc\",{id}]"), 0),
            Call::HitEnable { ch } => (format!("[\"HitEnable\",{}]", who(n, Some(ch))), 0),
            Call::Chat { me, msg: Chat::DeadOtherFellow(_) } => {
                (format!("[\"ChatMessageDeadOtherFellow\",{},\"s\"]", who(n, Some(me))), 0)
            }
            Call::Chat { me, msg } => {
                let mut arg = msg.arg().map_or(String::new(), |v| format!(",{v}"));
                if let Some(c) = msg.who() {
                    arg = format!(",{}", who(n, c));
                }
                (format!("[\"{}\",{}{}]", msg.name(), who(n, Some(me)), arg), 0)
            }
        };
        self.calls.push(line);
        ret
    }

    fn w2p(&mut self, pos: [u32; 4]) -> [u32; 4] {
        pos
    }
}

/// The runtime of the frame checks and `Run`: the following performed
/// here ([`Follow`] over the world the frame shares), the rest recorded
/// ([`Script`]).
struct Following<'a, 'c, 'w> {
    t: &'a Tables,
    mt: &'a MotionTables,
    party: &'a Party,
    game: &'a Game,
    state: &'a mut FollowState,
    world: &'c RefCell<&'w mut ScriptWorld>,
    inner: &'a mut Script,
}

impl Runtime for Following<'_, '_, '_> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        match call {
            Call::FollowPlayer { .. }
            | Call::LeavePlayer { .. }
            | Call::FollowTarget { .. }
            | Call::FollowTargetDirc { .. } => {
                let mut w = Share::new(self.world);
                let mut f = Follow {
                    t: self.t,
                    mt: self.mt,
                    party: self.party,
                    game: self.game,
                    state: &mut *self.state,
                    world: &mut w,
                    inner: &mut *self.inner,
                };
                f.call(call, scene, crew, rng)
            }
            other => self.inner.call(other, scene, crew, rng),
        }
    }

    fn w2p(&mut self, pos: V4) -> V4 {
        self.world.borrow_mut().w2p(pos)
    }
}

/// `CollisionDetection`'s script: the result, the push, the ground
/// attribute and `hitResultCharType`.
#[derive(Clone, Copy, Default)]
struct Push {
    ret: i32,
    offset: V4,
    attr: u32,
    kind: u32,
}

/// The world of a check: every query answered from its queue, every call
/// logged as the harness logs the game's.
#[derive(Default)]
struct ScriptWorld {
    names: Vec<String>,
    /// `ccTransPosW2P` adds it, `ccTransPosP2W` takes it off.
    off: V4,
    land: VecDeque<u32>,
    attr: VecDeque<u32>,
    line: VecDeque<u32>,
    push: VecDeque<Push>,
    fwd: VecDeque<i16>,
    notes: VecDeque<Vec<Note>>,
    trans: VecDeque<bool>,
    center: VecDeque<V4>,
    event: VecDeque<bool>,
    draw: VecDeque<bool>,
    char_type: u32,
    /// The dungeon's 2D map (`[x][y]`, 65536 cells) and the room window
    /// `Get2DMapInfo` answers (`RunNavi`).
    map2d: Vec<u8>,
    info: [i32; 3],
    log: Vec<String>,
}

fn f_add(a: u32, b: u32) -> u32 {
    piney_battle::geom::add(a, b)
}

fn f_sub(a: u32, b: u32) -> u32 {
    piney_battle::geom::sub(a, b)
}

impl World for ScriptWorld {
    fn w2p(&mut self, pos: V4) -> V4 {
        self.log.push(format!("[\"w2p\",{}]", list(pos)));
        std::array::from_fn(|i| f_add(pos[i], self.off[i]))
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        self.log.push(format!("[\"p2w\",{}]", list(pos)));
        std::array::from_fn(|i| f_sub(pos[i], self.off[i]))
    }
    fn land(&mut self, pos: V4, mask: u32) -> u32 {
        self.log.push(format!("[\"land\",{},{}]", list(pos), mask));
        self.land.pop_front().unwrap_or(0)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.log.push("[\"attr\"]".into());
        self.attr.pop_front().unwrap_or(0)
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> u32 {
        self.log.push(format!("[\"line\",{},{},{},{}]", list(from), list(to), mask, kind));
        self.line.pop_front().unwrap_or(0xbf80_0000)
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        self.log.push(format!(
            "[\"collide\",\"{}\",{},{},{},{},{},{}]",
            self.names[who],
            u8::from(hit.sw),
            hit.mask,
            hit.mask2,
            hit.radius,
            hit.height,
            list(hit.pos)
        ));
        let p = self.push.pop_front().unwrap_or_default();
        hit.offset = p.offset;
        hit.attribute = p.attr;
        self.char_type = p.kind;
        p.ret
    }
    fn hit_char_type(&mut self) -> u32 {
        self.char_type
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        self.log.push(format!("[\"hit_switch\",\"{}\",{}]", self.names[who], u8::from(on)));
        hit.sw = on;
    }
    fn camera_deg(&mut self, _pos: V4, _deg: i16) -> bool {
        unreachable!("a member's frame does not ask the camera")
    }
    fn camera_transparency(&mut self, _pos: V4, _w: u32, _h: u32, _far: u32, _len: u32) -> u32 {
        unreachable!("a member's frame does not ask the camera")
    }
    fn anim_set(&mut self, who: usize, _slot: AnmSlot, name: &str) {
        self.log.push(format!("[\"anim_set\",\"{}\",\"{}\"]", self.names[who], name));
    }
    fn anim_frame(&mut self, _who: usize, _slot: AnmSlot) -> u16 {
        unreachable!("a member's frame does not read the clip's frame")
    }
    fn anim_forward(&mut self, who: usize, _slot: AnmSlot, step: u16) -> i16 {
        self.log.push(format!("[\"anim_forward\",\"{}\",{}]", self.names[who], step));
        self.fwd.pop_front().unwrap_or(0)
    }
    fn anim_notes(&mut self, who: usize, _slot: AnmSlot) -> Vec<Note> {
        self.log.push(format!("[\"anim_notes\",\"{}\"]", self.names[who]));
        self.notes.pop_front().unwrap_or_default()
    }
}

impl NaviWorld for ScriptWorld {
    fn map_2d_info(&mut self) -> [i32; 3] {
        self.log.push("[\"Get2DMapInfo\"]".into());
        self.info
    }
    fn map_2d(&mut self) -> &[u8] {
        self.log.push("[\"Get2DMapPtr\"]".into());
        &self.map2d
    }
    fn route_search_by_map(&mut self, _: &mut Navi, _: &mut i16, _: V4, _: V4) -> i32 {
        unreachable!("a member's frame outside the Root Town asks no town route")
    }
    fn navi_point(&mut self, name: i16) -> V4 {
        unreachable!("a dungeon way's goal has no name ({name})")
    }
    fn marker(&mut self, _: i32) -> V4 {
        unreachable!("a member's frame places no landmarks")
    }
}

impl FellowWorld for ScriptWorld {
    fn trans_mode(&mut self) -> bool {
        self.log.push("[\"trans_mode\"]".into());
        self.trans.pop_front().unwrap_or(false)
    }
    fn trans_center(&mut self) -> V4 {
        self.log.push("[\"trans_center\"]".into());
        self.center.pop_front().unwrap_or_default()
    }
    fn event_area(&mut self) -> bool {
        self.log.push("[\"event_area\"]".into());
        self.event.pop_front().unwrap_or(false)
    }
    fn draw(&mut self, who: usize) -> bool {
        self.log.push(format!("[\"draw\",\"{}\"]", self.names[who]));
        self.draw.pop_front().unwrap_or(false)
    }
}

/// Everything a check sets up.
struct Case {
    scene: Scene,
    ents: Vec<usize>,
    party: Party,
    game: Game,
    save: SaveData,
    ids: Vec<i16>,
    crew: Crew,
    rng: Rand,
    script: Script,
    world: ScriptWorld,
    field: Field,
    state: FollowState,
    env: Env,
    out: Vec<Out>,
    /// A foe's fellow block, echoed.
    echo: Vec<Option<String>>,
    /// What `ccSkillCheck` answers for every character.
    running: i32,
    menu: bool,
    /// `RunNavi`'s movement.
    nav: Option<Nav>,
}

/// What the party's movement keeps (`RunNavi`), and `ccSpcRegistryNum()`.
struct Nav {
    keep: Keep,
    registry: i32,
}

fn read_case(t: &mut Toks) -> Case {
    let n = t.int() as usize;
    let mut scene = Scene::default();
    let mut crew = Crew::default();
    for i in 0..n {
        let mut c = read_char(t);
        c.pos = v4(t);
        c.base_mut().height = t.u32();
        let s = Spc {
            act_num: t.i16(),
            target_char: opt(t),
            move_flag: t.int() != 0,
            now_speed: t.u32(),
            cycle: t.i32(),
            dist_tg: t.u32(),
            spc_list_num: t.i32(),
            run_flag: t.int() != 0,
            ghost: t.int() != 0,
            stop_flag: t.int() != 0,
            stop_cnt: t.i16(),
            walk_run_cnt: t.i32(),
            ..Spc::default()
        };
        crew.spc.insert(i, s);
        scene.chars.push(c);
    }
    let lists: Vec<Vec<usize>> = (0..3)
        .map(|_| {
            let k = t.int();
            (0..k).map(|_| t.int() as usize).collect()
        })
        .collect();
    scene.pc_list = lists[0].clone();
    scene.ene_list = lists[1].clone();
    let ents = lists[2].clone();
    let mut party = Party::default();
    for m in party.members.iter_mut() {
        *m = opt(t);
    }
    for i in party.ids.iter_mut() {
        *i = t.i32();
    }
    party.num = t.i32();
    let game = Game {
        in_battle: t.i32(),
        area: t.i32(),
        field: t.i32(),
        field_type: t.i32(),
        field_attr: t.i32(),
        menu_type: t.i32(),
        event_lock: t.i32(),
        spc_battle_condition: t.i16(),
        party_strategy: t.i32(),
        player: opt(t),
        field_24: t.i32(),
        pg_ride_flag: t.i32(),
        area_prev: 0,
        server: 0,
    };
    let mut save = SaveData::new();
    let k = t.int();
    let mut ids = Vec::new();
    for _ in 0..k {
        let id = t.i16();
        ids.push(id);
        let at = SAVE_SKILL_LIST + 2 * SKILLS_PER_MEMBER * id as usize;
        for j in 0..SKILLS_PER_MEMBER {
            save.set_i16(at + 2 * j, t.i16());
        }
        let at = SAVE_ITEM_LIST + 4 * ITEMS_PER_MEMBER * id as usize;
        for j in 0..ITEMS_PER_MEMBER {
            save.set_i16(at + 4 * j, t.i16());
            save.set_u8(at + 4 * j + 2, t.int() as u8);
            save.set_u8(at + 4 * j + 3, t.int() as u8);
        }
    }
    let k = t.int();
    for _ in 0..k {
        let key = t.int() as usize;
        let a = read_ai(t, key);
        crew.ais.insert(key, a);
    }
    let s = &mut crew.sys;
    s.time = t.u32();
    s.entry_num = t.i16();
    s.history_top = t.i16();
    s.next_id = t.i16();
    for m in s.msg_buff.iter_mut() {
        *m = read_msg(t);
    }
    for m in s.msg_history.iter_mut() {
        *m = read_msg(t);
    }
    for e in s.entry.iter_mut() {
        *e = opt(t);
    }
    crew.select_attack_skill_result = t.i32();
    crew.check_skill_other_flag = t.i32();
    crew.check_heal_skill_other_flag = t.i32();
    crew.ocarina_use_flag = t.i32();
    let rng = Rand(t.int() as u64);
    let k = t.int();
    let hit = (0..k).map(|_| t.u32()).collect();
    let k = t.int();
    let goal = (0..k).map(|_| t.i32()).collect();
    let names: Vec<String> = (0..n).map(|i| format!("c{i}")).collect();
    // A foe's ccEntryObj lies under these offsets: the harness writes a
    // party member's block only, and echoes a foe's.
    let mut echo = Vec::with_capacity(n);
    for i in 0..n {
        let s = crew.spc.get_mut(&i).expect("every character's Spc");
        if scene.chars[i].ty() & 7 != 0 {
            read_fellow(t, &mut scene.chars[i], s);
            echo.push(None);
        } else {
            let (mut c, mut s) = (scene.chars[i].clone(), *s);
            read_fellow(t, &mut c, &mut s);
            echo.push(Some(fellow_json(&c, &s)));
        }
    }
    let mut world = ScriptWorld { names: names.clone(), ..ScriptWorld::default() };
    read_script(t, &mut world);
    let field = Field { gho_flag: t.int() != 0, warp_flag: t.int() != 0 };
    let state = FollowState { ai_open_dirc: t.u32() };
    let env = read_env(t);
    let model = t.int() != 0;
    let running = t.i32();
    let menu = t.int() != 0;
    Case {
        scene,
        ents,
        party,
        game,
        save,
        ids,
        crew,
        rng,
        script: Script { names, hit, goal, calls: vec![], model },
        world,
        field,
        state,
        env,
        out: vec![],
        echo,
        running,
        menu,
        nav: None,
    }
}

fn hex_bytes(t: &mut Toks) -> Vec<u8> {
    let s = t.word();
    if s == "-" {
        return Vec::new();
    }
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex")).collect()
}

fn hex(b: impl IntoIterator<Item = u8>) -> String {
    let s: String = b.into_iter().map(|x| format!("{x:02x}")).collect();
    if s.is_empty() { "-".into() } else { s }
}

/// The table-driven CRC-32 zlib computes.
fn crc32(data: impl IntoIterator<Item = u8>) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let tbl = TABLE.get_or_init(|| {
        std::array::from_fn(|n| {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            c
        })
    });
    let mut c = !0u32;
    for b in data {
        c = tbl[((c ^ u32::from(b)) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

/// A `ccNavi` as `tools/test_battle_navi_rs.py` serialises it: goalPos[4]
/// startX startY goalX goalY name step landmark dist dirc mapX mapY mapS
/// route(hex) pad[2] beacon.num beacons(hex, x y each).
fn read_navi(t: &mut Toks, n: &mut Navi) {
    n.goal_pos = v4(t);
    n.start_x = t.i16();
    n.start_y = t.i16();
    n.goal_x = t.i16();
    n.goal_y = t.i16();
    n.name = t.i16();
    n.step = t.i16();
    n.landmark = t.i16();
    n.dist = t.u32();
    n.dirc = t.u32();
    n.map_x = t.i16();
    n.map_y = t.i16();
    n.map_s = t.i16();
    let r = hex_bytes(t);
    n.route.copy_from_slice(&r[..48]);
    n.pad = [t.int() as u8, t.int() as u8];
    n.beacon_num = t.i32();
    let b = hex_bytes(t);
    n.beacon = b.chunks(2).map(|c| [c[0], c[1]]).collect();
}

fn navi_json(n: &Navi) -> String {
    let mut v: Vec<String> = n.goal_pos.iter().map(|x| x.to_string()).collect();
    v.extend([n.start_x, n.start_y, n.goal_x, n.goal_y, n.name, n.step, n.landmark].iter().map(|x| x.to_string()));
    v.extend([n.dist, n.dirc].iter().map(|x| x.to_string()));
    v.extend([n.map_x, n.map_y, n.map_s].iter().map(|x| x.to_string()));
    v.push(format!("\"{}\"", hex(n.route)));
    v.extend([n.pad[0].to_string(), n.pad[1].to_string(), n.beacon_num.to_string()]);
    v.push(format!("\"{}\"", hex(n.beacon.iter().flatten().copied())));
    format!("[{}]", v.join(","))
}

/// `NAVI`: `ccSpcRegistryNum()`, whether `SetPathFindingMap` makes the
/// room's map before the first frame, each AI's `ccNavi` (by key), the 2D
/// map's region (x y w h, then its cells in hex, `[x][y]`) and the room
/// window (x y size).
fn read_nav(t: &mut Toks, c: &mut Case) -> bool {
    let registry = t.i32();
    let made = t.int() != 0;
    let k = t.int();
    for _ in 0..k {
        let key = t.int() as usize;
        let a = c.crew.ais.get_mut(&key).expect("the AI of the navigation");
        read_navi(t, &mut a.navi);
    }
    let [mx, my, mw, mh] = [t.i32(), t.i32(), t.i32(), t.i32()];
    let m = hex_bytes(t);
    let w = &mut c.world;
    w.map2d = vec![0; 0x1_0000];
    for x in 0..mw {
        for y in 0..mh {
            w.map2d[((mx + x) * 256 + my + y) as usize] = m[(x * mh + y) as usize];
        }
    }
    w.info = [t.i32(), t.i32(), t.i32()];
    let keep = Keep { follow: c.state, ..Keep::default() };
    c.nav = Some(Nav { keep, registry });
    made
}

/// A character's `ccSpcChar` / `ccFellow` state, both copies set: `flags`
/// (+0xe0 word) `armsEffectSW actNum actNumOld atkAnmCnt anmFlag actCnt
/// reactCnt stopCnt attack transferLag speed speedRate nowSpeed cloak cnt cycle
/// consecutiveCnt walkRunCnt dirc[4] hitAttribute transparency setTransparency`,
/// `bodyHit` (`hitSW mask mask2 type radius height pos[4] offset[4]
/// attribute`), `fellowFlags` (+0x200) `dispWait atkDellay distTg diskOffset[4]
/// movePos[4] motion targetChar skillID skillStatus pos[4] posP[4]`.
fn read_fellow(t: &mut Toks, c: &mut Char, s: &mut Spc) {
    let w = t.u32();
    c.spc_char.flags = w & !(0x80 | 7 << 14);
    c.no_death = w & 0x80 != 0;
    c.party_flag = ((w >> 14) & 7) as i32;
    s.move_flag = w & 8 != 0;
    s.stop_flag = w & 0x10 != 0;
    s.run_flag = w & 0x20 != 0;
    s.ghost = w & 0x40 != 0;
    c.spc_char.arms_effect_sw = t.i32();
    s.act_num = t.i16();
    c.spc_char.act_num = s.act_num;
    c.spc_char.act_num_old = t.i16();
    s.atk_anm_cnt = t.i16();
    c.anm_flag = t.i16();
    s.act_cnt = t.i16();
    s.react_cnt = t.i16();
    s.stop_cnt = t.i16();
    c.spc_char.attack = t.i16();
    s.transfer_lag = t.i16();
    s.speed = t.u32();
    s.speed_rate = t.u32();
    s.now_speed = t.u32();
    c.spc_char.cloak = t.u32();
    c.spc_char.cnt = t.i32();
    s.cycle = t.i32();
    s.consecutive_cnt = t.i32();
    s.walk_run_cnt = t.i32();
    s.dirc = v4(t);
    s.hit_attribute = t.u32();
    s.transparency = t.u32();
    s.set_transparency = t.u32();
    let h = &mut s.body_hit;
    h.sw = t.int() != 0;
    h.mask = t.u32();
    h.mask2 = t.u32();
    h.kind = t.u32();
    h.radius = t.u32();
    h.height = t.u32();
    h.pos = v4(t);
    h.offset = v4(t);
    h.attribute = t.u32();
    c.spc_char.hit_enabled = h.sw;
    c.spc_char.fellow_flags = t.int() as u8;
    s.disp_wait = t.i32();
    s.atk_dellay = t.i32();
    s.dist_tg = t.u32();
    s.disk_offset = v4(t);
    s.move_pos = v4(t);
    s.motion = t.i16();
    s.target_char = opt(t);
    c.target_char = s.target_char;
    c.skill_id = t.i16();
    c.skill_status = t.i16();
    c.pos = v4(t);
    c.pos_p = v4(t);
}

/// The world's queues, in the order [`ScriptWorld`] lists them.
fn read_script(t: &mut Toks, w: &mut ScriptWorld) {
    w.off = v4(t);
    let k = t.int();
    w.land = (0..k).map(|_| t.u32()).collect();
    let k = t.int();
    w.attr = (0..k).map(|_| t.u32()).collect();
    let k = t.int();
    w.line = (0..k).map(|_| t.u32()).collect();
    let k = t.int();
    w.push = (0..k).map(|_| Push { ret: t.i32(), offset: v4(t), attr: t.u32(), kind: t.u32() }).collect();
    let k = t.int();
    w.fwd = (0..k).map(|_| t.i16()).collect();
    let k = t.int();
    w.notes = (0..k)
        .map(|_| {
            let m = t.int();
            (0..m).map(|_| Note { event: t.u32(), param: t.u32() }).collect()
        })
        .collect();
    let k = t.int();
    w.trans = (0..k).map(|_| t.int() != 0).collect();
    let k = t.int();
    w.center = (0..k).map(|_| v4(t)).collect();
    let k = t.int();
    w.event = (0..k).map(|_| t.int() != 0).collect();
    let k = t.int();
    w.draw = (0..k).map(|_| t.int() != 0).collect();
}

fn fellow_json(c: &Char, s: &Spc) -> String {
    let w = c.spc_char.flags | u32::from(c.no_death) << 7 | ((c.party_flag as u32) & 7) << 14;
    let h = &s.body_hit;
    let mut v: Vec<i64> = vec![
        w.into(),
        c.spc_char.arms_effect_sw.into(),
        c.spc_char.act_num.into(),
        c.spc_char.act_num_old.into(),
        s.atk_anm_cnt.into(),
        c.anm_flag.into(),
        s.act_cnt.into(),
        s.react_cnt.into(),
        s.stop_cnt.into(),
        c.spc_char.attack.into(),
        s.transfer_lag.into(),
        s.speed.into(),
        s.speed_rate.into(),
        s.now_speed.into(),
        c.spc_char.cloak.into(),
        c.spc_char.cnt.into(),
        s.cycle.into(),
        s.consecutive_cnt.into(),
        s.walk_run_cnt.into(),
    ];
    v.extend(s.dirc.iter().map(|&x| i64::from(x)));
    v.extend([s.hit_attribute, s.transparency, s.set_transparency].map(i64::from));
    v.extend([i64::from(h.sw), h.mask.into(), h.mask2.into(), h.kind.into(), h.radius.into(), h.height.into()]);
    v.extend(h.pos.iter().chain(h.offset.iter()).map(|&x| i64::from(x)));
    v.push(h.attribute.into());
    v.extend([
        i64::from(c.spc_char.fellow_flags),
        i64::from(s.disp_wait),
        i64::from(s.atk_dellay),
        i64::from(s.dist_tg),
    ]);
    v.extend(s.disk_offset.iter().chain(s.move_pos.iter()).map(|&x| i64::from(x)));
    v.extend([s.motion.into(), show(c.target_char), c.skill_id.into(), c.skill_status.into()]);
    v.extend(c.pos.iter().chain(c.pos_p.iter()).map(|&x| i64::from(x)));
    list(v)
}

/// A character's affect members (`affectPerson`, `affectType`,
/// `affectParam`, the flash, `affectMask`, the condition tint), then the
/// flag byte at +0xe0 and a foe's flag word (+0x250).
fn affect_json(c: &Char) -> String {
    let a = &c.affect;
    list([
        show(a.person),
        a.ty.into(),
        a.param[0].into(),
        a.param[1].into(),
        a.param[2].into(),
        a.color_cnt.into(),
        a.color_rate.into(),
        a.color.into(),
        a.mask.into(),
        a.cond_color_cnt.into(),
        a.cond_color_rate.into(),
        a.cond_color.into(),
        i64::from((c.spc_char.flags | u32::from(c.no_death) << 7) & 0xff),
        c.spc_char.enemy_flags.into(),
    ])
}

fn out_json(o: &Out, names: &Names) -> String {
    let me = names.me;
    match *o {
        Out::Rule(e) => {
            let s = calls(&[e], names);
            s[1..s.len() - 1].to_string()
        }
        Out::EquipWeapon => format!("[\"EquipWeapon\",\"{me}\"]"),
        Out::DeleteWeaponCcs => format!("[\"DeleteWeaponCCS\",\"{me}\"]"),
        Out::LevelUp => format!("[\"effLevelUp\",\"{me}\"]"),
        Out::OpenBox { pos } => format!("[\"effOpenBox\",{}]", list(pos)),
        Out::DeleteCmnd => format!("[\"ccDeleteCmnd\",\"{me}\"]"),
        Out::SetMatrix { pos, dirc } => format!("[\"SetMatrix\",{},{}]", list(pos), list(dirc)),
        Out::ArmsEffect => format!("[\"ArmsEffect\",\"{me}\"]"),
        Out::ClearArmsEffect => format!("[\"ClearArmsEffect\",\"{me}\"]"),
        Out::ArmsEffectColor(sid) => format!("[\"SetArmsEffectColor\",\"{me}\",{sid}]"),
        Out::StartArmsEffect(sid) => format!("[\"StartArmsEffect\",\"{me}\",{sid}]"),
        Out::SkillStart { sid, flag } => format!("[\"effSkillStart\",\"{me}\",{sid},{flag},0]"),
        Out::Transfer => format!("[\"effTransfer\",\"{me}\"]"),
        Out::WarpTransfer => format!("[\"effWarpTransfer\",\"{me}\"]"),
        Out::Se { param } => format!("[\"ccSeSetParamSPC\",{},\"{me}\"]", param as i32),
        Out::PawSmoke { speed } => format!("[\"ccEffPawSmoke\",\"{me}\",{speed}]"),
    }
}

/// Runs `f` on a frame of the case with the following performed and the
/// rest recorded ([`Following`]); the frame's outputs are the case's.
fn with_frame(c: &mut Case, t: &Tables, mt: &MotionTables, f: impl FnOnce(&mut Frame) -> i64) -> i64 {
    let running = c.running;
    let check = move |_: usize| running;
    let cell = RefCell::new(&mut c.world);
    let mut share = Share::new(&cell);
    let mut rt =
        Following { t, mt, party: &c.party, game: &c.game, state: &mut c.state, world: &cell, inner: &mut c.script };
    let mut fr = Frame {
        t,
        mt,
        scene: &mut c.scene,
        party: &c.party,
        save: &mut c.save,
        crew: &mut c.crew,
        game: &c.game,
        field: &c.field,
        ents: &c.ents,
        env: &c.env,
        rng: &mut c.rng,
        world: &mut share,
        rt: &mut rt,
        menu: c.menu,
        skill_check: &check,
        out: vec![],
    };
    let r = f(&mut fr);
    c.out = std::mem::take(&mut fr.out);
    r
}

/// `ccFellow::Main` for `me` with the party's movement as its runtime
/// ([`Movement`]) over the case's world shared through a [`RefCell`].
fn navi_main(c: &mut Case, nav: &mut Nav, t: &Tables, mt: &MotionTables, me: usize) {
    let running = c.running;
    let check = move |_: usize| running;
    let cell = RefCell::new(&mut c.world);
    let mut share = Share::new(&cell);
    let mut rt = Movement {
        t,
        mt,
        party: &c.party,
        game: &c.game,
        keep: &mut nav.keep,
        spc_registry_num: nav.registry,
        world: &cell,
        inner: &mut c.script,
    };
    let mut fr = Frame {
        t,
        mt,
        scene: &mut c.scene,
        party: &c.party,
        save: &mut c.save,
        crew: &mut c.crew,
        game: &c.game,
        field: &c.field,
        ents: &c.ents,
        env: &c.env,
        rng: &mut c.rng,
        world: &mut share,
        rt: &mut rt,
        menu: c.menu,
        skill_check: &check,
        out: vec![],
    };
    fr.main(me);
    c.out = std::mem::take(&mut fr.out);
}

impl Case {
    /// The following with the case's world and runtime.
    fn follow<'a>(
        &'a mut self,
        t: &'a Tables,
        mt: &'a MotionTables,
    ) -> (Follow<'a>, &'a mut Scene, &'a mut Crew, &'a mut Rand) {
        (
            Follow {
                t,
                mt,
                party: &self.party,
                game: &self.game,
                state: &mut self.state,
                world: &mut self.world,
                inner: &mut self.script,
            },
            &mut self.scene,
            &mut self.crew,
            &mut self.rng,
        )
    }

    fn answer(&mut self, ret: i64, me: usize) -> String {
        let n = self.scene.chars.len();
        let names: Vec<String> = (0..n).map(|i| format!("c{i}")).collect();
        let me_name = names[me].clone();
        let nm = Names { me: &me_name, target: "t", chars: names.clone(), ai: true };
        let out: Vec<String> = self.out.iter().map(|o| out_json(o, &nm)).collect();
        let ais: Vec<String> = self.crew.ais.iter().map(|(k, a)| format!("\"{k}\":{}", ai_json(a))).collect();
        let spc: Vec<String> =
            self.crew.spc.iter().map(|(k, s)| format!("\"{k}\":{}", spc_json(s, &self.scene.chars[*k]))).collect();
        let fel: Vec<String> = (0..n)
            .map(|i| match &self.echo[i] {
                Some(e) => e.clone(),
                None => fellow_json(&self.scene.chars[i], &self.crew.spc.get(&i).copied().unwrap_or_default()),
            })
            .collect();
        let affect: Vec<String> = self.scene.chars.iter().map(affect_json).collect();
        let chars: Vec<String> = self.scene.chars.iter().map(state).collect();
        let items: Vec<String> = self
            .ids
            .iter()
            .map(|&id| {
                let at = SAVE_ITEM_LIST + 4 * ITEMS_PER_MEMBER * id as usize;
                let v = (0..ITEMS_PER_MEMBER).flat_map(|j| {
                    let b = at + 4 * j;
                    [
                        i64::from(self.save.i16(b)),
                        i64::from(self.save.u8(b + 2) as i8),
                        i64::from(self.save.u8(b + 3) as i8),
                    ]
                });
                format!("\"{id}\":{}", list(v))
            })
            .collect();
        let mut navi = String::new();
        if let Some(nav) = &self.nav {
            let v: Vec<String> = self.crew.ais.iter().map(|(k, a)| format!("\"{k}\":{}", navi_json(&a.navi))).collect();
            let p = &nav.keep.path;
            navi = format!(
                ",\"navi\":{{{}}},\"path\":[{},{},{}]",
                v.join(","),
                p.ready,
                crc32(p.link.iter().copied()),
                crc32(p.cost.iter().flat_map(|v| v.to_le_bytes()))
            );
        }
        let s = format!(
            "{{\"ret\":{},\"rand\":{},\"calls\":[{}],\"world\":[{}],\"out\":[{}],\"ais\":{{{}}},\"sys\":{},\"spc\":{{{}}},\
             \"items\":{{{}}},\"fel\":[{}],\"affect\":[{}],\"chars\":[{}],\"lists\":[{},{},{}],\"globals\":{}}}",
            ret,
            self.rng.0,
            self.script.calls.join(","),
            self.world.log.join(","),
            out.join(","),
            ais.join(","),
            sys_json(&self.crew.sys),
            spc.join(","),
            items.join(","),
            fel.join(","),
            affect.join(","),
            chars.join(","),
            list(self.scene.pc_list.iter()),
            list(self.scene.ene_list.iter()),
            list(self.scene.obj_list.iter()),
            list([
                i64::from(self.crew.select_attack_skill_result),
                i64::from(self.crew.check_skill_other_flag),
                i64::from(self.crew.check_heal_skill_other_flag),
                i64::from(self.crew.ocarina_use_flag),
                i64::from(self.state.ai_open_dirc)
            ])
        );
        let s = format!("{}{navi}}}", &s[..s.len() - 1]);
        self.script.calls.clear();
        self.world.log.clear();
        self.out.clear();
        s
    }
}

/// One frame of a run's Kite: `pos posP dirc moveFlag runFlag`.
fn read_kite(t: &mut Toks) -> (V4, V4, V4, bool, bool) {
    (v4(t), v4(t), v4(t), t.int() != 0, t.int() != 0)
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    if cmd != "fel" {
        return None;
    }
    let f = t.word().to_string();
    let args: Vec<i64> = (0..t.int()).map(|_| t.int()).collect();
    let mut c = read_case(t);
    let tb: &Tables = tables;
    let mt = motion_tables();
    let me = args[0] as usize;
    let a = |i: usize| args[i];
    let o = |v: i64| if v < 0 { None } else { Some(v as usize) };
    if f == "RunNavi" && read_nav(t, &mut c) {
        let mut nav = c.nav.take().expect("the navigation");
        let area = c.game.area;
        nav.keep.path.set_path_finding_map(area, &mut c.world);
        c.nav = Some(nav);
    }
    if f == "Run" || f == "RunNavi" {
        let frames = a(1) as usize;
        let kite = c.party.members[0];
        let mut snaps = Vec::with_capacity(frames);
        for _ in 0..frames {
            let (pos, pos_p, dirc, mv, run) = read_kite(t);
            if let Some(k) = kite {
                let ch = &mut c.scene.chars[k];
                ch.pos = pos;
                ch.pos_p = pos_p;
                ch.spc_char.flags = (ch.spc_char.flags & !0x28) | if mv { 8 } else { 0 } | if run { 0x20 } else { 0 };
                let s = c.crew.spc.entry(k).or_default();
                s.dirc = dirc;
                s.move_flag = mv;
                s.run_flag = run;
            }
            c.env.count = c.env.count.wrapping_add(1);
            c.crew.tick();
            match c.nav.take() {
                Some(mut nav) => {
                    navi_main(&mut c, &mut nav, tb, mt, me);
                    c.state = nav.keep.follow;
                    c.nav = Some(nav);
                }
                None => {
                    with_frame(&mut c, tb, mt, |fr| {
                        fr.main(me);
                        0
                    });
                }
            }
            snaps.push(c.answer(0, me));
        }
        return Some(format!("{{\"frames\":[{}]}}", snaps.join(",")));
    }
    fellow::sync_in(&c.scene, &mut c.crew, me);
    let ret: i64 = match f.as_str() {
        "Main" | "Move" | "HitCheck" | "Action" | "CheckNote" => {
            let note = Note {
                event: args.get(1).copied().unwrap_or(0) as u32,
                param: args.get(2).copied().unwrap_or(0) as u32,
            };
            with_frame(&mut c, tb, mt, |fr| match f.as_str() {
                "Main" => {
                    fr.main(me);
                    0
                }
                "Move" => {
                    fr.step(me);
                    0
                }
                "HitCheck" => fr.hit_check(me).into(),
                "Action" => {
                    fr.action(me);
                    0
                }
                _ => {
                    fr.check_note(me, note);
                    0
                }
            })
        }
        "FollowPlayer" | "LeavePlayer" | "FollowTarget" | "FollowTargetDirc" => {
            let (mut fw, scene, crew, rng) = c.follow(tb, mt);
            let call = match f.as_str() {
                "FollowPlayer" => Call::FollowPlayer { me },
                "LeavePlayer" => Call::LeavePlayer { me },
                "FollowTarget" => Call::FollowTarget { me, target: o(a(1)) },
                _ => Call::FollowTargetDirc { me, target: o(a(1)) },
            };
            fw.call(call, scene, crew, rng).into()
        }
        "CheckFrontObstacle" => {
            let (mut fw, scene, crew, _) = c.follow(tb, mt);
            i64::from(fw.check_front_obstacle(me, a(1) as u32, scene, crew))
        }
        "CheckFrontObstacleF" => {
            let goal: V4 = [a(1) as u32, a(2) as u32, a(3) as u32, a(4) as u32];
            let (mut fw, scene, crew, _) = c.follow(tb, mt);
            fw.check_front_obstacle_f(me, goal, scene, crew).into()
        }
        "SetDircZ" => {
            follow::set_dirc_z(&mut c.crew, me, a(1) as u16);
            0
        }
        _ => return Some(format!("{{\"error\":\"unknown fel function {f}\"}}")),
    };
    fellow::sync_out(&mut c.scene, &c.crew, me);
    Some(c.answer(ret, me))
}
