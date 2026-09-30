//! `battle_probe` requests for the party's navigation and the AI's own movers
//! (`piney_battle::navi`, `piney_battle::ai_move`), as
//! `tools/test_battle_navi_rs.py` sends them: `navi PATH NOPS (OP NARGS
//! ARGS...)... WORLD`. The operations run in order on one world, each answered
//! with its return and a snapshot of what the movers can change (with `PATH` 1
//! also the path finding's maps). `WORLD` is the characters ([`read_char`] and
//! their motion fields), the party, the AIs with their `ccNavi`, the bus, the
//! RNG, the maps, the landmarks and the scripts of the world's answers.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

use piney_battle::ai_move::{self, Nav};
use piney_battle::chara::Body;
use piney_battle::exp::Party;
use piney_battle::fellow::MotionTables;
use piney_battle::follow::FollowState;
use piney_battle::geom::V4;
use piney_battle::navi::{Landmark, Navi, NaviWorld, PathMap, TownMap};
use piney_battle::party_ai::*;
use piney_battle::party_motion::{Keep, Movement};
use piney_battle::rand::{Rand, Rng};
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};
use piney_data::save::SaveData;

use crate::party_ai::{msg_json, opt, read_msg, show};
use crate::{Toks, list, read_char};

/// A `RouteSearchByMap` answer: what it returns and leaves in the
/// navigation. `Scripted` sets the fields the search sets (`goal_pos` the
/// goal, `landmark` 1, `finishFlag` 0); `Found` is what the game's own
/// search left in the whole `ccNavi` (+0x00-+0x63), the harness having run
/// it over Mac Anu's tables.
#[derive(Clone)]
enum Route {
    Scripted { ret: i32, name: i16, step: i16, dist: u32, dirc: u32, route: [u8; 48] },
    Found { ret: i32, navi: Box<Navi>, finish: i16 },
}

thread_local! {
    /// The calls an operation made, the world's and the runtime's in one
    /// order, as the harness records the game's.
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// What `ccHitCheckLM` answers, in the order the game asks: the
    /// decisions' lines of sight ([`Call::HitCheckLm`]) and the
    /// following's ([`World::line`]) alike.
    static LINES: RefCell<VecDeque<u32>> = const { RefCell::new(VecDeque::new()) };
}

fn line_answer(from: V4, to: V4, mask: u32, kind: i32) -> u32 {
    let name = if kind == 1 { "ccHitCheckLM2" } else { "ccHitCheckLM" };
    record(format!("[\"{name}\",{},{},{}]", vec4(from), vec4(to), mask));
    LINES.with(|l| l.borrow_mut().pop_front()).unwrap_or(F_MINUS_ONE)
}

fn record(line: String) {
    CALLS.with(|c| c.borrow_mut().push(line));
}

/// The world of a check: the scripts.
#[derive(Default)]
struct Script {
    info: [i32; 3],
    infos: VecDeque<[i32; 3]>,
    map2d: Vec<u8>,
    routes: VecDeque<Route>,
    points: BTreeMap<i16, V4>,
    markers: BTreeMap<i32, V4>,
}

fn vec4(v: V4) -> String {
    list(v)
}

impl World for Script {
    fn w2p(&mut self, pos: V4) -> V4 {
        pos
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        pos
    }
    fn land(&mut self, _: V4, _: u32) -> u32 {
        0
    }
    fn hit_attribute(&mut self) -> u32 {
        0
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> u32 {
        line_answer(from, to, mask, kind)
    }
    fn collide(&mut self, _: usize, _: &mut CharHit) -> i32 {
        0
    }
    fn hit_char_type(&mut self) -> u32 {
        0
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        record(format!("[\"{}\",\"c{who}\"]", if on { "HitEnable" } else { "HitDisable" }));
        hit.sw = on;
    }
    fn camera_deg(&mut self, _: V4, _: i16) -> bool {
        true
    }
    fn camera_transparency(&mut self, _: V4, _: u32, _: u32, _: u32, _: u32) -> u32 {
        0x3f80_0000
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

impl NaviWorld for Script {
    fn map_2d_info(&mut self) -> [i32; 3] {
        record("[\"Get2DMapInfo\"]".into());
        if let Some(i) = self.infos.pop_front() {
            self.info = i;
        }
        self.info
    }
    fn map_2d(&mut self) -> &[u8] {
        record("[\"Get2DMapPtr\"]".into());
        &self.map2d
    }
    fn route_search_by_map(&mut self, navi: &mut Navi, finish: &mut i16, start: V4, goal: V4) -> i32 {
        record(format!("[\"RouteSearchByMap\",{},{}]", vec4(start), vec4(goal)));
        match self.routes.pop_front() {
            Some(Route::Found { ret, navi: n, finish: f }) => {
                let beacon = std::mem::take(&mut navi.beacon);
                *navi = *n;
                navi.beacon = beacon;
                *finish = f;
                ret
            }
            r => {
                let (ret, name, step, dist, dirc, route) = match r {
                    Some(Route::Scripted { ret, name, step, dist, dirc, route }) => {
                        (ret, name, step, dist, dirc, route)
                    }
                    _ => (0, 0, 0, 0, 0, [0; 48]),
                };
                navi.route = route;
                navi.name = name;
                *finish = 0;
                navi.step = step;
                navi.landmark = 1;
                navi.dist = dist;
                navi.dirc = dirc;
                navi.goal_pos = goal;
                ret
            }
        }
    }
    fn navi_point(&mut self, name: i16) -> V4 {
        record(format!("[\"NaviPoint\",{name}]"));
        self.points.get(&name).copied().unwrap_or_default()
    }
    fn marker(&mut self, k: i32) -> V4 {
        record(format!("[\"Marker\",{k}]"));
        self.markers.get(&k).copied().unwrap_or_default()
    }
}

/// The runtime of the party AI: the calls recorded, `ccTransPosW2P` the
/// identity (the harness's).
struct Rt;

fn who(c: Option<usize>) -> String {
    match c {
        Some(i) => format!("\"c{i}\""),
        None => "0".into(),
    }
}

impl Runtime for Rt {
    fn call(&mut self, c: Call, _: &mut Scene, _: &mut Crew, _: &mut dyn Rng) -> i32 {
        let mut ret = 0;
        let line = match c {
            Call::SkillRequest { me, target, sid } => {
                format!("[\"ccSkillRequest\",{},{},{}]", who(Some(me)), who(target), sid)
            }
            Call::UseItemRequest { me, target, item, flag } => {
                format!("[\"ccUseItemRequest\",{},{},{},{}]", who(Some(me)), who(Some(target)), item, flag)
            }
            Call::FollowTarget { me, target } => format!("[\"FollowTarget\",{},{}]", who(Some(me)), who(target)),
            Call::FollowTargetDirc { me, target } => {
                format!("[\"FollowTargetDirc\",{},{}]", who(Some(me)), who(target))
            }
            Call::FollowPlayer { me } => format!("[\"FollowPlayer\",{}]", who(Some(me))),
            Call::LeavePlayer { me } => format!("[\"LeavePlayer\",{}]", who(Some(me))),
            Call::PlayerAttack { me, target, n } => {
                format!("[\"PlayerAttack\",{},{},{}]", who(Some(me)), who(Some(target)), n)
            }
            Call::ManualModeAi { ch, ev } => format!("[\"ManualModeAI\",{},{}]", who(Some(ch)), ev),
            Call::FaceTalk { me, g_deg } => format!("[\"FaceTalk\",{},{}]", who(Some(me)), g_deg as i16),
            Call::ChatMessageSender { me } => format!("[\"ChatMessageSender\",{}]", who(Some(me))),
            Call::TransferOut { ch } => format!("[\"TransferOut\",{}]", who(Some(ch))),
            Call::TransferIn { ch } => format!("[\"TransferIn\",{}]", who(Some(ch))),
            Call::ResignParty { id } => format!("[\"resignParty\",{id}]"),
            Call::DisbandSpc { id } => format!("[\"disbandSpc\",{id}]"),
            Call::HitEnable { ch } => format!("[\"HitEnable\",{}]", who(Some(ch))),
            Call::HitCheckLm { from, to, mask } => {
                // line_answer records the call itself.
                return line_answer(from, to, mask, 0) as i32;
            }
            Call::Chat { me, msg: Chat::Reencounter } => format!("[\"ChatMessageReencounter\",{}]", who(Some(me))),
            // The line by its bytes, as the harness reads what the game passes.
            Call::Chat { me, msg: Chat::Line(table, i) } => {
                format!("[\"ChatMessage\",{},{}]", who(Some(me)), town_text(table.table(), i))
            }
            Call::Chat { me, msg: Chat::DeadOtherFellow(_) } => {
                format!("[\"ChatMessageDeadOtherFellow\",{},\"s\"]", who(Some(me)))
            }
            Call::Chat { me, msg } => {
                let mut arg = msg.arg().map_or(String::new(), |v| format!(",{v}"));
                if let Some(c) = msg.who() {
                    arg = format!(",{}", who(c));
                }
                format!("[\"{}\",{}{}]", msg.name(), who(Some(me)), arg)
            }
            other => {
                ret = -999;
                format!("[\"unexpected\",\"{other:?}\"]")
            }
        };
        record(line);
        ret
    }

    fn w2p(&mut self, pos: [u32; 4]) -> [u32; 4] {
        pos
    }
}

/// The following's tables (`fpAngleOffset`, `fpOkRange`, `fellowAnimTbl`),
/// read from the disc once.
fn motion_tables() -> &'static MotionTables {
    static T: std::sync::OnceLock<MotionTables> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
        let mut iso = piney_data::iso::Iso::open(&path).expect("the disc image");
        MotionTables::of(iso.volume().expect("the volume"))
    })
}

/// Row `i` of the line table at `at` (Infection's address), the volume's
/// bytes in hex (`-` for a null entry or an empty line).
fn town_text(at: u32, i: i32) -> String {
    static TEXTS: std::sync::OnceLock<piney_battle::party_chat::ChatTexts> = std::sync::OnceLock::new();
    let t = TEXTS.get_or_init(|| piney_battle::party_chat::ChatTexts::of(crate::probe_volume()));
    format!("\"{}\"", hex(t.line(at, i).unwrap_or_default().iter().copied()))
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

/// A hex token ("-" for none) as bytes.
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

/// [`crate::party_ai::read_ai`] and then the member's navigation.
fn read_ai(t: &mut Toks, key: usize) -> Ai {
    let mut a = crate::party_ai::read_ai(t, key);
    let n = &mut a.navi;
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
    a
}

/// [`crate::party_ai::ai_fields`] and then the member's navigation.
fn ai_json(a: &Ai) -> String {
    let n = &a.navi;
    let v: Vec<String> = crate::party_ai::ai_fields(a)
        .into_iter()
        .chain(n.goal_pos.iter().map(|x| x.to_string()))
        .chain([n.start_x, n.start_y, n.goal_x, n.goal_y, n.name, n.step, n.landmark].iter().map(|x| x.to_string()))
        .chain([n.dist, n.dirc].iter().map(|x| x.to_string()))
        .chain([n.map_x, n.map_y, n.map_s].iter().map(|x| x.to_string()))
        .chain([format!("\"{}\"", hex(n.route)), n.pad[0].to_string(), n.pad[1].to_string(), n.beacon_num.to_string()])
        .chain([format!("\"{}\"", hex(n.beacon.iter().flatten().copied()))])
        .collect();
    format!("[{}]", v.join(","))
}

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

/// The table-driven CRC-32 zlib computes.
fn crc32(data: impl IntoIterator<Item = u8>) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
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

/// Everything a check sets up.
struct Setup {
    scene: Scene,
    crew: Crew,
    party: Party,
    game: Game,
    save: SaveData,
    spc_registry_num: i32,
    rng: Rand,
    /// The path map, the town's landmarks and `aiOpenDirc`.
    keep: Keep,
    region: [i32; 4],
    script: Script,
}

fn read_world(t: &mut Toks) -> Setup {
    let n = t.int() as usize;
    let mut scene = Scene::default();
    let mut crew = Crew::default();
    for i in 0..n {
        let mut c = read_char(t);
        let dirc = v4(t);
        c.spc_char.act_num_old = t.i16();
        let transparency = t.u32();
        let set_transparency = t.u32();
        let radius = t.u32();
        let velocity = t.u32();
        let hit_sw = t.int() != 0;
        c.base_mut().height = t.u32();
        if let Body::Spc(p) = &mut c.body {
            p.velocity = velocity;
        }
        let f = c.spc_char.flags;
        let s = Spc {
            act_num: c.spc_char.act_num,
            target_char: c.target_char,
            move_flag: f & 8 != 0,
            stop_flag: f & 0x10 != 0,
            run_flag: f & 0x20 != 0,
            ghost: f & 0x40 != 0,
            dirc,
            transparency,
            set_transparency,
            body_hit: CharHit { radius, sw: hit_sw, ..CharHit::default() },
            ..Spc::default()
        };
        crew.spc.insert(i, s);
        scene.chars.push(c);
    }
    let k = t.int();
    scene.pc_list = (0..k).map(|_| t.int() as usize).collect();
    let k = t.int();
    scene.ene_list = (0..k).map(|_| t.int() as usize).collect();
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
    save.set_u8(0x220d, t.int() as u8);
    let spc_registry_num = t.i32();
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
    let rng = Rand(t.int() as u64);
    let ai_open_dirc = t.u32();
    let k = t.int();
    let lines: VecDeque<u32> = (0..k).map(|_| t.u32()).collect();
    LINES.with(|l| *l.borrow_mut() = lines);
    let mut path = PathMap { ready: t.i32(), ..PathMap::default() };
    let region = [t.i32(), t.i32(), t.i32(), t.i32()];
    let link = hex_bytes(t);
    let cost = hex_bytes(t);
    let [x0, y0, w, h] = region;
    for x in 0..w {
        for y in 0..h {
            let i = (x * h + y) as usize;
            path.set_link(x0 + x, y0 + y, link[i]);
            path.set_cost(x0 + x, y0 + y, i16::from_le_bytes([cost[2 * i], cost[2 * i + 1]]));
        }
    }
    let mut script = Script { map2d: vec![0; 0x1_0000], ..Script::default() };
    let [mx, my, mw, mh] = [t.i32(), t.i32(), t.i32(), t.i32()];
    let m = hex_bytes(t);
    for x in 0..mw {
        for y in 0..mh {
            script.map2d[((mx + x) * 256 + my + y) as usize] = m[(x * mh + y) as usize];
        }
    }
    script.info = [t.i32(), t.i32(), t.i32()];
    let k = t.int();
    script.infos = (0..k).map(|_| [t.i32(), t.i32(), t.i32()]).collect();
    let num = t.i32();
    let marks = (0..=num.max(0)).map(|_| Landmark { pos: v4(t), name: t.i32() }).collect();
    let town = TownMap { num, marks };
    let k = t.int();
    for _ in 0..k {
        let kind = t.int();
        let ret = t.i32();
        if kind == 0 {
            let name = t.i16();
            let step = t.i16();
            let dist = t.u32();
            let dirc = t.u32();
            let r = hex_bytes(t);
            let mut route = [0u8; 48];
            route.copy_from_slice(&r[..48]);
            script.routes.push_back(Route::Scripted { ret, name, step, dist, dirc, route });
        } else {
            let mut n = Navi::new();
            n.goal_pos = v4(t);
            n.start_x = t.i16();
            n.start_y = t.i16();
            n.goal_x = t.i16();
            n.goal_y = t.i16();
            n.name = t.i16();
            let finish = t.i16();
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
            script.routes.push_back(Route::Found { ret, navi: Box::new(n), finish });
        }
    }
    let k = t.int();
    for _ in 0..k {
        let name = t.i16();
        let p = v4(t);
        script.points.insert(name, p);
    }
    let k = t.int();
    for _ in 0..k {
        let i = t.i32();
        let p = v4(t);
        script.markers.insert(i, p);
    }
    let keep = Keep { follow: FollowState { ai_open_dirc }, path, town };
    Setup { scene, crew, party, game, save, spc_registry_num, rng, keep, region, script }
}

impl Setup {
    /// A body's `ccSpcChar` word at +0xe0 as the game holds it.
    fn flags_word(&self, i: usize) -> u32 {
        let c = &self.scene.chars[i];
        let s = self.crew.spc.get(&i).copied().unwrap_or_default();
        (c.spc_char.flags & !0x78 & !0x80 & !(7 << 14))
            | u32::from(s.move_flag) << 3
            | u32::from(s.stop_flag) << 4
            | u32::from(s.run_flag) << 5
            | u32::from(s.ghost) << 6
            | u32::from(c.no_death) << 7
            | ((c.party_flag & 7) as u32) << 14
    }

    fn chars_json(&self) -> String {
        let v: Vec<String> = (0..self.scene.chars.len())
            .map(|i| {
                let c = &self.scene.chars[i];
                let s = self.crew.spc.get(&i).copied().unwrap_or_default();
                format!(
                    "\"{i}\":[{},{},{},{},{},{},{},{},{},{}]",
                    self.flags_word(i),
                    list(s.dirc),
                    s.act_num,
                    c.spc_char.act_num_old,
                    show(s.target_char),
                    s.transparency,
                    s.set_transparency,
                    c.spc_char.cloak,
                    list(c.pos),
                    u8::from(s.body_hit.sw)
                )
            })
            .collect();
        format!("{{{}}}", v.join(","))
    }

    fn path_json(&self) -> String {
        let [x0, y0, w, h] = self.region;
        let mut link = Vec::new();
        let mut cost = Vec::new();
        for x in 0..w {
            for y in 0..h {
                link.push(self.keep.path.link_at(x0 + x, y0 + y));
                cost.extend(self.keep.path.cost_at(x0 + x, y0 + y).to_le_bytes());
            }
        }
        format!(
            "{{\"ready\":{},\"link\":\"{}\",\"cost\":\"{}\",\"crc\":[{},{}]}}",
            self.keep.path.ready,
            hex(link),
            hex(cost),
            crc32(self.keep.path.link.iter().copied()),
            crc32(self.keep.path.cost.iter().flat_map(|v| v.to_le_bytes()))
        )
    }

    fn snapshot(&self, calls: &[String], path: bool, town: bool, sys: bool) -> String {
        let ais: Vec<String> = self.crew.ais.iter().map(|(k, a)| format!("\"{k}\":{}", ai_json(a))).collect();
        let mut s = format!(
            "\"rand\":{},\"calls\":[{}],\"ais\":{{{}}},\"chars\":{},\"open\":{}",
            self.rng.0,
            calls.join(","),
            ais.join(","),
            self.chars_json(),
            self.keep.follow.ai_open_dirc
        );
        if sys {
            s += &format!(",\"sys\":{}", sys_json(&self.crew.sys));
        }
        if path {
            s += &format!(",\"path\":{}", self.path_json());
        }
        if town {
            let v: Vec<String> = self.keep.town.marks.iter().map(|m| list(m.pos)).collect();
            s += &format!(",\"town\":[{}]", v.join(","));
        }
        s
    }
}

/// Runs one operation; returns its result fields (JSON members).
fn run_op(w: &mut Setup, tables: &Tables, op: &str, a: &[i64]) -> String {
    let vec_at = |i: usize| -> V4 { std::array::from_fn(|k| a[i + k] as u32) };
    let key = a.first().map_or(0, |&k| k as usize);
    let area = w.game.area;
    // The ccNavi methods, on the AI's navigation alone.
    macro_rules! navi {
        () => {
            w.crew.ais[&key].navi
        };
    }
    macro_rules! navi_mut {
        () => {
            w.crew.ais.get_mut(&key).expect("an AI").navi
        };
    }
    match op {
        "SetPathFindingMap" => {
            w.keep.path.set_path_finding_map(area, &mut w.script);
            "\"ret\":0".into()
        }
        "PathFindingF" | "PathFinding" => {
            let ai = w.crew.ais.get_mut(&key).expect("an AI");
            let mut finish = ai.navi_finish;
            let r = if op == "PathFindingF" {
                ai.navi.path_finding(&mut finish, area, &mut w.keep.path, &mut w.script, vec_at(1), vec_at(5))
            } else {
                let c = |i: usize| a[i] as i32;
                ai.navi.path_finding_cells(&mut finish, area, &mut w.keep.path, &mut w.script, c(1), c(2), c(3), c(4))
            };
            ai.navi_finish = finish;
            format!("\"ret\":{r}")
        }
        "Heuristic" => {
            let n = w.crew.ais[&key].navi.clone();
            format!("\"ret\":{}", n.heuristic_type1(&mut w.keep.path))
        }
        "ShortPath" => {
            let n = w.crew.ais[&key].navi.clone();
            format!("\"ret\":{}", i32::from(n.short_path(&mut w.keep.path, a[1] as i32, a[2] as i32)))
        }
        "MakeBeaconTbl" => {
            let n = w.crew.ais[&key].navi.clone();
            match n.make_beacon_tbl(&mut w.keep.path, a[1] as i32, a[2] as i32) {
                Some(tbl) => format!("\"ret\":{},\"tbl\":\"{}\"", tbl.len(), hex(tbl.iter().flatten().copied())),
                None => "\"ret\":0,\"tbl\":\"-\"".into(),
            }
        }
        "SetBeacon" => {
            let tbl: Vec<[u8; 4]> = a[1..].chunks(4).map(|c| std::array::from_fn(|k| c[k] as u8)).collect();
            format!("\"ret\":{}", navi_mut!().set_beacon(&tbl))
        }
        "LinkInfo" => format!("\"ret\":{}", navi!().link_info(&w.keep.path, a[1] as i32, a[2] as i32)),
        "MinimumLinkDirc" => format!("\"ret\":{}", navi!().minimum_link_dirc(&w.keep.path, a[1] as i32, a[2] as i32)),
        "LinkNum" => format!("\"ret\":{}", navi!().link_num(&w.keep.path, a[1] as i32, a[2] as i32)),
        "CheckBeaconPos" => format!("\"ret\":0,\"vec\":{}", list(navi!().check_beacon_pos(a[1] as i32))),
        "CheckBeaconPos2D" => format!("\"ret\":0,\"vec\":{}", list(navi!().check_beacon_pos_2d(a[1] as i32))),
        "GetDestination" => {
            let mut p = vec_at(1);
            let r = navi!().get_destination(area, &w.keep.town, &mut p);
            format!("\"ret\":{r},\"vec\":{}", list(p))
        }
        "SetDungeonMapInfo" => {
            let n = &mut w.crew.ais.get_mut(&key).expect("an AI").navi;
            n.set_dungeon_map_info(&mut w.script);
            "\"ret\":0".into()
        }
        "Ctor" => {
            let ai = w.crew.ais.get_mut(&key).expect("an AI");
            let mut n = Navi::new();
            // The fields the constructor leaves alone.
            n.start_x = ai.navi.start_x;
            n.start_y = ai.navi.start_y;
            n.goal_x = ai.navi.goal_x;
            n.goal_y = ai.navi.goal_y;
            n.map_x = ai.navi.map_x;
            n.map_y = ai.navi.map_y;
            n.map_s = ai.navi.map_s;
            n.pad = ai.navi.pad;
            ai.navi_finish = 1;
            if area == 2 {
                n.set_dungeon_map_info(&mut w.script);
            }
            ai.navi = n;
            "\"ret\":0".into()
        }
        "SearchLandmark" => format!("\"ret\":{}", w.keep.town.search_landmark(a[0] as i32)),
        "GetLandmarkPos" => {
            let mut p = vec_at(1);
            let r = match w.keep.town.landmark_pos(a[0] as i32) {
                Some(v) => {
                    p = v;
                    1
                }
                None => 0,
            };
            format!("\"ret\":{r},\"vec\":{}", list(p))
        }
        "SearchNearLandmark" => format!("\"ret\":{}", w.keep.town.search_near_landmark(tables.volume, vec_at(0))),
        "SearchNearLandmarkN" => {
            format!("\"ret\":{}", w.keep.town.search_near_landmark_n(tables.volume, vec_at(0), a[4] as i32))
        }
        "SetNaviMap" => {
            w.keep.town.set_navi_map(&mut w.script);
            "\"ret\":0".into()
        }
        "poke" => {
            let ai = w.crew.ais.get_mut(&key).expect("an AI");
            let v = a[2] as i16;
            match a[1] {
                0x86 => ai.act_type = v,
                0x8a => ai.act_time = v,
                0x98 => ai.no_move_cnt = v,
                _ => return format!("\"error\":\"poke {}\"", a[1]),
            }
            "\"ret\":0".into()
        }
        "setpos" => {
            w.scene.chars[key].pos = vec_at(1);
            "\"ret\":0".into()
        }
        "townfull" | "manualfull" | "brainsfull" => composed(w, tables, op, key, a),
        _ => {
            let mut rt = Rt;
            let mut ctx = Ctx {
                t: tables,
                scene: &mut w.scene,
                party: &w.party,
                save: &mut w.save,
                crew: &mut w.crew,
                game: &w.game,
                ents: &[],
                rng: &mut w.rng,
                rt: &mut rt,
            };
            let mut nav = Nav {
                world: &mut w.script,
                path: &mut w.keep.path,
                town: &w.keep.town,
                spc_registry_num: w.spc_registry_num,
            };
            let mut vec = None;
            let r: i64 = match op {
                "distpl" => {
                    let body = ctx.crew.ais[&key].body;
                    let d = distance_to_target(tables.volume, ctx.scene, body, ctx.party.members[0]);
                    ctx.crew.ais.get_mut(&key).expect("an AI").dist_pl = d;
                    0
                }
                "townframe" | "beaconframe" => {
                    let body = ctx.crew.ais[&key].body;
                    ctx.scene.chars[body].pos = vec_at(1);
                    if op == "townframe" {
                        let kite = ctx.party.members[0];
                        if let Some(k) = kite {
                            ctx.scene.chars[k].pos = vec_at(5);
                        }
                        let d = distance_to_target(tables.volume, ctx.scene, body, kite);
                        ctx.crew.ais.get_mut(&key).expect("an AI").dist_pl = d;
                        ctx.act_in_town(&mut nav, key).into()
                    } else {
                        ctx.follow_beacon(&mut nav, key);
                        0
                    }
                }
                "MoveP2P" => i64::from(ctx.move_p2p(key, vec_at(1), vec_at(5), a[9] as i32)),
                "SetTargetPosDirc" => {
                    ctx.set_target_pos_dirc(key, vec_at(1));
                    0
                }
                "FollowBeacon" => {
                    ctx.follow_beacon(&mut nav, key);
                    0
                }
                "CheckGoalBeaconPos" => ctx.check_goal_beacon_pos(key, vec_at(1)).into(),
                "ManualControl" => ctx.manual_control(&mut nav, key).into(),
                "ManualMode" => {
                    ctx.manual_mode(key);
                    0
                }
                "SetRemoteCmd" => {
                    ctx.set_remote_cmd(key, a[1] as i32);
                    0
                }
                "SetGoalPos" => {
                    ctx.set_goal_pos(&mut nav, key, vec_at(1), a[5] as i32);
                    0
                }
                "FollowTargetTown" => {
                    ctx.follow_target_town(key, usize::try_from(a[1]).ok());
                    0
                }
                "TownNavigator" | "TownNavigatorPoint" => {
                    let mut goal = vec_at(1);
                    let r = if op == "TownNavigator" {
                        ctx.town_navigator(&mut nav, key, &mut goal, vec_at(5), a[9] as i32)
                    } else {
                        ctx.town_navigator_point(&mut nav, key, &mut goal, vec_at(5), a[9] as i32)
                    };
                    vec = Some(goal);
                    r.into()
                }
                "TownNavigatorPos" => ctx.town_navigator_pos(&mut nav, key, vec_at(1), vec_at(5)).into(),
                "MessageIndex" => ctx.message_index(key).into(),
                "ActInTown" => ctx.act_in_town(&mut nav, key).into(),
                "perform" => {
                    let body = ctx.crew.ais[&key].body;
                    let call = match a[1] {
                        0 => Call::PathFinding { me: body, from: vec_at(2), to: vec_at(6) },
                        1 => Call::FollowBeacon { me: body },
                        2 => Call::GoalBeacon { me: body, pos: vec_at(2) },
                        3 => Call::ManualControl { me: body },
                        _ => Call::ActInTown { me: body },
                    };
                    ai_move::perform(call, &mut ctx, &mut nav).map_or(-999, i64::from)
                }
                _ => return format!("\"error\":\"unknown navi op {op}\""),
            };
            match vec {
                Some(v) => format!("\"ret\":{r},\"vec\":{}", list(v)),
                None => format!("\"ret\":{r}"),
            }
        }
    }
}

/// The movers composed as the runtime composes them: the party's movement
/// ([`Movement`]) performs `ActInTown` (`townfull`), `ManualControl`
/// (`manualfull`) or a whole `ccAI::Brains` (`brainsfull`) and what they
/// call (following, path finding, beacons, `HitEnable`), over the scripted
/// world; the rest goes to [`Rt`]. `townfull` and `brainsfull` first put
/// the member at `a[1..5]` and Kite at `a[5..9]` (`townfull` measures
/// `distPl` as `ccAI::Brains` would have).
fn composed(w: &mut Setup, tables: &Tables, op: &str, key: usize, a: &[i64]) -> String {
    let vec_at = |i: usize| -> V4 { std::array::from_fn(|k| a[i + k] as u32) };
    let body = w.crew.ais[&key].body;
    if op != "manualfull" {
        w.scene.chars[body].pos = vec_at(1);
        if let Some(k) = w.party.members[0] {
            w.scene.chars[k].pos = vec_at(5);
            let s = w.crew.spc.entry(k).or_default();
            s.dirc[2] = a[9] as u32;
            s.move_flag = a[10] & 8 != 0;
            s.run_flag = a[10] & 0x20 != 0;
        }
    }
    if op == "townfull" {
        let d = distance_to_target(tables.volume, &w.scene, body, w.party.members[0]);
        w.crew.ais.get_mut(&key).expect("an AI").dist_pl = d;
    }
    let mut rt = Rt;
    let cell = RefCell::new(&mut w.script);
    let mut mov = Movement {
        t: tables,
        mt: motion_tables(),
        party: &w.party,
        game: &w.game,
        keep: &mut w.keep,
        spc_registry_num: w.spc_registry_num,
        world: &cell,
        inner: &mut rt,
    };
    let p = Parts {
        t: tables,
        scene: &mut w.scene,
        party: &w.party,
        save: &mut w.save,
        crew: &mut w.crew,
        game: &w.game,
        ents: &[],
        rng: &mut w.rng,
    };
    let r = match op {
        "townfull" => mov.call_ctx(Call::ActInTown { me: body }, p),
        "manualfull" => mov.call_ctx(Call::ManualControl { me: body }, p),
        _ => p.ctx(&mut mov).brains(key),
    };
    format!("\"ret\":{r}")
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    if cmd != "navi" {
        return None;
    }
    let flags = t.int();
    let (path, sys_last) = (flags & 1 != 0, flags & 2 != 0);
    let nops = t.int();
    let ops: Vec<(String, Vec<i64>)> = (0..nops)
        .map(|_| {
            let name = t.word().to_string();
            let k = t.int();
            (name, (0..k).map(|_| t.int()).collect())
        })
        .collect();
    let mut w = read_world(t);
    let mut out = Vec::new();
    for (k, (op, args)) in ops.iter().enumerate() {
        CALLS.with(|c| c.borrow_mut().clear());
        let r = run_op(&mut w, tables, op, args);
        let calls = CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()));
        let sys = !sys_last || k + 1 == ops.len();
        out.push(format!("{{{r},{}}}", w.snapshot(&calls, path, op == "SetNaviMap", sys)));
    }
    Some(format!("[{}]", out.join(",")))
}
