//! `battle_probe` requests for Kite's code (`piney_battle::kite`), as
//! `tools/test_battle_kite_rs.py` sends them: `kite FN FRAMES NARGS ARGS...
//! WORLD KITE SCRIPT NPOKES POKES...` and `kite frames BOUNDS[4] PLAYER[4]
//! POINT[4]`. `WORLD` is the party AI harness's world, `KITE` Kite's
//! `ccPlayer` beyond it, `SCRIPT` what the world's calls answer. `MainRunAi`
//! runs `ccPlayer::Main` FRAMES times with the party AI's calls performed by
//! the port ([`kite::Host`]) and answers every frame's state. The answer is
//! everything the call can change, as the harness reads the game's memory.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::OnceLock;

use piney_battle::chara::{Char, Env};
use piney_battle::event::{Event, Who};
use piney_battle::exp::Party;
use piney_battle::fellow::MotionTables;
use piney_battle::geom::V4;
use piney_battle::kite::{self, Cx, Host, Input, KiteTables, KiteWorld, MapBounds, Out, Pad, Player};
use piney_battle::navi::{Navi, NaviWorld};
use piney_battle::party_ai::*;
use piney_battle::party_motion::Keep;
use piney_battle::rand::{Rand, Rng};
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};
use piney_data::iso::Iso;
use piney_data::save::SaveData;

use crate::party_ai::{ai_json, msg_json, opt, read_ai, read_msg, show};
use crate::{Names, Toks, list, read_char, read_env, state};

/// The disc's player tables, read once.
fn kite_tables() -> &'static KiteTables {
    static K: OnceLock<KiteTables> = OnceLock::new();
    K.get_or_init(|| {
        let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
        let mut iso = Iso::open(&path).expect("the disc image");
        KiteTables::of(iso.volume().expect("the volume"))
    })
}

/// The disc's motion tables (the following reads `CFOFP`), read once.
fn motion_tables() -> &'static MotionTables {
    static M: OnceLock<MotionTables> = OnceLock::new();
    M.get_or_init(|| {
        let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
        let mut iso = Iso::open(&path).expect("the disc image");
        MotionTables::of(iso.volume().expect("the volume"))
    })
}

fn floats(v: &[u32]) -> String {
    list(v.iter().copied())
}

fn vec4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

/// What the world's calls answer, in order, and what they recorded.
#[derive(Default)]
struct Script {
    names: Vec<String>,
    hit_lm: VecDeque<u32>,
    goal: VecDeque<i32>,
    calls: Vec<String>,
    req: VecDeque<i64>,
    fwd: VecDeque<i64>,
    notes: VecDeque<Vec<Note>>,
    land: VecDeque<u32>,
    result: VecDeque<i64>,
    attr: VecDeque<u32>,
    hit: VecDeque<(i32, u32, u32)>,
    trans: VecDeque<i64>,
    center: VecDeque<V4>,
    draw: VecDeque<(i64, u32)>,
    eye: VecDeque<u32>,
    /// `ccHitCheckLM`/`ccHitCheckLM2` of the following
    /// (`CheckFrontObstacle(F)`, [`World::line`]).
    line: VecDeque<u32>,
    /// `aiOpenDirc` at the start.
    open: u32,
    last_result: Option<u32>,
    pending: Vec<Note>,
    cam_type: i32,
    cam_id: i32,
    cam_rot: V4,
    cam_reset: i32,
    cam_reset_dirc: u32,
    plw: (i32, bool),
    /// What `ccSkillCheck` answers: 0 nothing, 1 a running normal attack
    /// (skill 1 with a status), 2 always a skill.
    skc: i64,
}

impl Script {
    fn who(&self, c: Option<usize>) -> String {
        match c {
            Some(i) => format!("\"{}\"", self.names[i]),
            None => "0".into(),
        }
    }

    fn rec(&mut self, s: String) {
        self.calls.push(s);
    }

    fn who_w(&self, w: Who) -> String {
        match w {
            Who::Char(i) => self.who(Some(i)),
            _ => "0".into(),
        }
    }
}

impl Runtime for Script {
    fn call(&mut self, c: Call, scene: &mut Scene, _crew: &mut Crew, _rng: &mut dyn Rng) -> i32 {
        let (line, ret) = match c {
            Call::SkillRequest { me, target, sid } => {
                if self.req.pop_front().unwrap_or(0) != 0 {
                    scene.chars[me].skill_status = 0;
                }
                (format!("[\"ccSkillRequest\",{},{},{}]", self.who(Some(me)), self.who(target), sid), 0)
            }
            Call::UseItemRequest { me, target, item, flag } => (
                format!("[\"ccUseItemRequest\",{},{},{},{}]", self.who(Some(me)), self.who(Some(target)), item, flag),
                0,
            ),
            Call::FollowTarget { me, target } => {
                (format!("[\"FollowTarget\",{},{}]", self.who(Some(me)), self.who(target)), 0)
            }
            Call::FollowTargetDirc { me, target } => {
                (format!("[\"FollowTargetDirc\",{},{}]", self.who(Some(me)), self.who(target)), 0)
            }
            Call::FollowPlayer { me } => (format!("[\"FollowPlayer\",{}]", self.who(Some(me))), 0),
            Call::LeavePlayer { me } => (format!("[\"LeavePlayer\",{}]", self.who(Some(me))), 0),
            Call::PathFinding { me, from, to } => {
                let r = self.goal.pop_front().unwrap_or(0);
                (format!("[\"PathFinding\",{},{},{}]", self.who(Some(me)), floats(&from), floats(&to)), r)
            }
            Call::FollowBeacon { me } => (format!("[\"FollowBeacon\",{}]", self.who(Some(me))), 0),
            Call::GoalBeacon { me, pos } => {
                let r = self.goal.pop_front().unwrap_or(0);
                (format!("[\"CheckGoalBeaconPos\",{},{}]", self.who(Some(me)), floats(&pos)), r)
            }
            Call::PlayerAttack { me, target, n } => {
                (format!("[\"PlayerAttack\",{},{},{}]", self.who(Some(me)), self.who(Some(target)), n), 0)
            }
            Call::HitCheckLm { from, to, mask } => {
                let r = self.hit_lm.pop_front().unwrap_or(F_MINUS_ONE);
                (format!("[\"ccHitCheckLM\",{},{},{}]", floats(&from), floats(&to), mask), r as i32)
            }
            Call::FaceTalk { me, g_deg } => (format!("[\"FaceTalk\",{},{}]", self.who(Some(me)), g_deg as i16), 0),
            Call::ManualControl { me } => (format!("[\"ManualControl\",{}]", self.who(Some(me))), 0),
            Call::ActInTown { me } => (format!("[\"ActInTown\",{}]", self.who(Some(me))), 0),
            Call::ChatMessageSender { me } => (format!("[\"ChatMessageSender\",{}]", self.who(Some(me))), 0),
            Call::ManualModeAi { ch, ev } => (format!("[\"ManualModeAI\",{},{}]", self.who(Some(ch)), ev), 0),
            Call::TransferOut { ch } => (format!("[\"TransferOut\",{}]", self.who(Some(ch))), 0),
            // The AI's own movers' calls (crate::ai_move), which no kite
            // check reaches.
            Call::TransferIn { ch } => (format!("[\"TransferIn\",{}]", self.who(Some(ch))), 0),
            Call::ResignParty { id } => (format!("[\"resignParty\",{id}]"), 0),
            Call::DisbandSpc { id } => (format!("[\"disbandSpc\",{id}]"), 0),
            Call::HitEnable { ch } => (format!("[\"HitEnable\",{}]", self.who(Some(ch))), 0),
            Call::Chat { me, msg: Chat::DeadOtherFellow(_) } => {
                (format!("[\"ChatMessageDeadOtherFellow\",{},\"s\"]", self.who(Some(me))), 0)
            }
            Call::Chat { me, msg } => {
                let mut arg = msg.arg().map_or(String::new(), |v| format!(",{v}"));
                if let Some(c) = msg.who() {
                    arg = format!(",{}", self.who(c));
                }
                (format!("[\"{}\",{}{}]", msg.name(), self.who(Some(me)), arg), 0)
            }
        };
        self.calls.push(line);
        ret
    }

    fn w2p(&mut self, pos: [u32; 4]) -> [u32; 4] {
        pos
    }
}

impl World for Script {
    fn w2p(&mut self, pos: V4) -> V4 {
        pos
    }

    fn p2w(&mut self, pos: V4) -> V4 {
        pos
    }

    fn land(&mut self, pos: V4, mask: u32) -> u32 {
        self.rec(format!("[\"ccLandHitCheck\",{},{}]", floats(&pos), mask));
        let z = self.land.pop_front().unwrap_or(0);
        let r = self.result.pop_front().unwrap_or(-1);
        self.last_result = if r == -1 { None } else { Some(r as u32) };
        z
    }

    fn hit_attribute(&mut self) -> u32 {
        self.rec("[\"checkHitResultAttlibute\"]".into());
        self.attr.pop_front().unwrap_or(0)
    }

    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> u32 {
        self.rec(format!("[\"line\",{},{},{mask},{kind}]", floats(&from), floats(&to)));
        self.line.pop_front().unwrap_or(F_MINUS_ONE)
    }

    fn collide(&mut self, _who: usize, _hit: &mut CharHit) -> i32 {
        self.rec("[\"unexpected collide\"]".into());
        0
    }

    fn hit_char_type(&mut self) -> u32 {
        0
    }

    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        let n = if on { "HitEnable" } else { "HitDisable" };
        self.rec(format!("[\"{n}\",{}]", self.who(Some(who))));
        hit.sw = on;
    }

    fn camera_deg(&mut self, _pos: V4, _deg: i16) -> bool {
        false
    }

    fn camera_transparency(&mut self, _pos: V4, _w: u32, _h: u32, _far: u32, _len: u32) -> u32 {
        0x3f80_0000
    }

    fn anim_set(&mut self, _who: usize, _slot: AnmSlot, name: &str) {
        self.rec(format!("[\"SetAnm\",\"{name}\"]"));
    }

    fn anim_frame(&mut self, _who: usize, _slot: AnmSlot) -> u16 {
        0
    }

    fn anim_forward(&mut self, _who: usize, _slot: AnmSlot, step: u16) -> i16 {
        self.rec(format!("[\"AnimateForward\",{step}]"));
        self.pending = self.notes.pop_front().unwrap_or_default();
        self.fwd.pop_front().unwrap_or(0) as i16
    }

    fn anim_notes(&mut self, _who: usize, _slot: AnmSlot) -> Vec<Note> {
        std::mem::take(&mut self.pending)
    }
}

/// The dungeon's and the town's navigation, which no kite check reaches (the
/// runs are in a field).
impl NaviWorld for Script {
    fn map_2d_info(&mut self) -> [i32; 3] {
        self.rec("[\"unexpected Get2DMapInfo\"]".into());
        [0; 3]
    }

    fn map_2d(&mut self) -> &[u8] {
        self.rec("[\"unexpected Get2DMapPtr\"]".into());
        &[]
    }

    fn route_search_by_map(&mut self, _navi: &mut Navi, _finish: &mut i16, _start: V4, _goal: V4) -> i32 {
        self.rec("[\"unexpected RouteSearchByMap\"]".into());
        0
    }

    fn navi_point(&mut self, name: i16) -> V4 {
        self.rec(format!("[\"unexpected NaviPoint\",{name}]"));
        [0; 4]
    }

    fn marker(&mut self, k: i32) -> V4 {
        self.rec(format!("[\"unexpected Marker\",{k}]"));
        [0; 4]
    }
}

impl KiteWorld for Script {
    fn out(&mut self, o: Out) {
        let s = match o {
            // The AI's lines as the harness's stubs record them: the
            // member's name for its AI.
            Out::Rule(Event::ChatDamage { on, value, .. }) => {
                format!("[\"ChatMessageDamage\",{},{value}]", self.who_w(on))
            }
            Out::Rule(Event::ChatResurrectPlz(on)) => format!("[\"ChatMessageResurrectPlz\",{}]", self.who_w(on)),
            Out::Rule(Event::NoteHit { on, value, .. }) => format!("[\"NoteHit\",{},{value}]", self.who_w(on)),
            Out::Rule(Event::AffectMessages { on, kind, by, n, .. }) => {
                format!("[\"AffectMessages\",{},{kind},{},{n}]", self.who_w(on), self.who_w(by))
            }
            Out::Rule(Event::Greeting { on, by, n }) => {
                format!("[\"Greeting\",{},{},{n}]", self.who_w(on), self.who_w(by))
            }
            Out::Rule(e) => {
                let names = Names { me: "", target: "", chars: self.names.clone(), ai: false };
                let v = crate::calls(&[e], &names);
                v[1..v.len() - 1].to_string()
            }
            Out::ArmsEffectColor { sid } => format!("[\"SetArmsEffectColor\",{sid}]"),
            Out::ArmsParticles { a, b, attr } => format!("[\"startParticleEffect2\",{a},{b},{attr}]"),
            Out::ArmsEffect => "[\"ArmsEffect\"]".into(),
            Out::ClearArmsEffect => "[\"ClearArmsEffect\"]".into(),
            Out::SkillStart { sid, item } => format!("[\"effSkillStart\",{sid},{item},0]"),
            Out::Transfer => "[\"effTransfer\"]".into(),
            Out::WarpTransfer => "[\"effWarpTransfer\"]".into(),
            Out::Sound { param } => format!("[\"ccSeSetParamSPC\",{param}]"),
            Out::PawSmoke { speed } => format!("[\"ccEffPawSmoke\",{speed}]"),
            Out::LevelUp => "[\"effLevelUp\"]".into(),
            Out::OpenBox { pos } => format!("[\"effOpenBox\",{}]", floats(&pos)),
            Out::EquipWeapon => "[\"EquipWeapon\"]".into(),
            Out::DeleteWeaponCcs => "[\"DeleteWeaponCCS\"]".into(),
            Out::AddCenter { x, y } => format!("[\"AddCenter\",{x},{y}]"),
            Out::Enter { pos } => format!("[\"Enter\",{}]", floats(&pos)),
            Out::Matrix { pos, dirc } => format!("[\"SetMatrix\",{},{}]", floats(&pos), floats(&dirc)),
            Out::PlayerWork { attack, pause } => {
                self.plw = (attack, pause);
                return;
            }
            Out::GateArms => return,
            Out::Actuate { power } => format!("[\"SetActuater\",1,{power},100]"),
        };
        self.rec(s);
    }

    fn camera_type(&mut self) -> i32 {
        self.cam_type
    }

    fn camera_id(&mut self) -> i32 {
        self.cam_id
    }

    fn camera_rot(&mut self) -> V4 {
        self.cam_rot
    }

    fn camera_reset(&mut self) -> Option<u32> {
        (self.cam_reset != 0).then_some(self.cam_reset_dirc)
    }

    fn clear_camera_reset(&mut self) {
        self.cam_reset = 0;
    }

    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4) {
        self.rec(format!("[\"cameraSetEyeLevel\",{},{}]", floats(&pos_eye), floats(angle)));
        angle[2] = self.eye.pop_front().unwrap_or(0);
    }

    fn camera_set_manual(&mut self, pos_view: V4) {
        self.rec(format!("[\"cameraSetManual\",{}]", floats(&pos_view)));
    }

    fn camera_set(&mut self) {
        self.rec("[\"cameraSet\"]".into());
    }

    fn hit_check(&mut self, me: usize, _ch: &Char, _hit: &mut CharHit, _ns: u32, _manual: bool, mp: &mut V4) -> i32 {
        self.rec(format!("[\"HitCheck\",{},{}]", self.who(Some(me)), floats(mp)));
        let (r, x, y) = self.hit.pop_front().unwrap_or((0, 0, 0));
        mp[0] = x;
        mp[1] = y;
        r
    }

    fn hit_result(&mut self) -> Option<u32> {
        self.last_result
    }

    fn trans_mode(&mut self) -> bool {
        self.rec("[\"GetTransMode\"]".into());
        self.trans.pop_front().unwrap_or(0) != 0
    }

    fn trans_center(&mut self) -> V4 {
        self.rec("[\"GetTransCenter\"]".into());
        self.center.pop_front().unwrap_or([0, 0, 0, 0x3f80_0000])
    }

    fn draw(&mut self, me: usize, _pos: V4, set_transparency: u32, transparency: &mut u32) -> bool {
        self.rec(format!("[\"Draw\",{},{}]", self.who(Some(me)), set_transparency));
        let (d, t) = self.draw.pop_front().unwrap_or((1, 0x3f80_0000));
        *transparency = t;
        d != 0
    }

    fn skill_check(&mut self, scene: &Scene, ch: usize) -> i32 {
        match self.skc {
            0 => 0,
            1 => {
                let c = &scene.chars[ch];
                i32::from(c.skill_id == 1 && c.skill_status != 0)
            }
            _ => 2,
        }
    }

    fn gate_hacking_out(&mut self, _me: usize, _ch: &mut Char, _p: &mut Player, _spc: &mut Spc) {
        self.rec("[\"GateHackingOut\"]".into());
    }
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

fn spc_json(s: &Spc) -> String {
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
    me: usize,
    p: Player,
    input: Input,
    env: Env,
    /// `aiOpenDirc` as the last run left it.
    open: u32,
}

fn read_world(t: &mut Toks) -> Case {
    let n = t.int() as usize;
    let mut scene = Scene::default();
    let mut crew = Crew::default();
    for i in 0..n {
        let mut c = read_char(t);
        c.pos = std::array::from_fn(|_| t.u32());
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
    let hit_lm = (0..k).map(|_| t.u32()).collect();
    let k = t.int();
    let goal = (0..k).map(|_| t.i32()).collect();
    let names = (0..n).map(|i| format!("c{i}")).collect();
    Case {
        scene,
        ents,
        party,
        game,
        save,
        ids,
        crew,
        rng,
        script: Script { names, hit_lm, goal, ..Script::default() },
        me: 0,
        p: Player::default(),
        input: Input::default(),
        env: Env::default(),
        open: 0,
    }
}

/// The `KITE` block: Kite's `ccPlayer` beyond the world, the globals, the
/// camera, the environment.
fn read_kite(t: &mut Toks, w: &mut Case) {
    let me = w.me;
    let c = &mut w.scene.chars[me];
    c.spc_char.flags = t.u32();
    c.spc_char.act_num = t.i16();
    c.spc_char.act_num_old = t.i16();
    let p = &mut w.p;
    let s = w.crew.spc.entry(me).or_default();
    s.atk_anm_cnt = t.i16();
    c.anm_flag = t.i16();
    s.act_cnt = t.i16();
    s.react_cnt = t.i16();
    c.spc_char.attack = t.i16();
    s.transfer_lag = t.i16();
    c.spc_char.cloak = t.u32();
    c.spc_char.cnt = t.i32();
    c.spc_char.arms_effect_sw = t.i32();
    c.target_char = opt(t);
    s.dirc = vec4(t);
    s.speed = t.u32();
    s.speed_rate = t.u32();
    s.move_pos = vec4(t);
    s.body_hit.sw = t.int() != 0;
    c.spc_char.hit_enabled = s.body_hit.sw;
    s.hit_attribute = t.u32();
    s.transparency = t.u32();
    s.set_transparency = t.u32();
    let pf = t.int();
    p.camera_flag = pf & 1 != 0;
    p.inactive = pf & 2 != 0;
    p.install = pf & 4 != 0;
    p.ctrl_type = t.i32();
    p.prog_ctrl_flag = t.i32();
    p.dist_tg = t.i32();
    p.dirc_tg = t.u32();
    p.pos_view = vec4(t);
    p.pos_eye = vec4(t);
    p.angle = vec4(t);
    p.disk_offset = vec4(t);
    p.target_count = t.i16();
    p.act_one_two_cnt = t.i16();
    p.stress = t.i16();
    p.frame_spd = t.int() as u16;
    let pad = Pad { pow_l: t.int() as u8, dirc_l: t.u32() };
    let cmnd_target = t.int() != 0;
    let warp = t.int() != 0;
    let dne = t.int() != 0;
    let b: [u32; 4] = std::array::from_fn(|_| t.u32());
    let bounds = MapBounds { min: [b[0], b[1]], max: [b[2], b[3]] };
    let sc = &mut w.script;
    sc.cam_type = t.i32();
    sc.cam_id = t.i32();
    sc.cam_rot = vec4(t);
    sc.cam_reset = t.i32();
    sc.cam_reset_dirc = t.u32();
    sc.skc = t.int();
    let menu = t.int() != 0;
    let s = w.crew.spc.entry(me).or_default();
    s.body_hit.radius = t.u32();
    s.body_hit.height = t.u32();
    w.input = Input { pad, cmnd_target, warp, dne, bounds, menu };
    w.env = read_env(t);
    // The id the character's own copy keeps (affect's CheckSysMsgID).
    let id = w.crew.sys_msg_id(me);
    w.scene.chars[me].spc_char.sys_msg_id = id;
}

fn read_script(t: &mut Toks, sc: &mut Script) {
    let n = t.int();
    sc.req = (0..n).map(|_| t.int()).collect();
    let n = t.int();
    sc.fwd = (0..n).map(|_| t.int()).collect();
    let n = t.int();
    sc.notes = (0..n)
        .map(|_| {
            let k = t.int();
            (0..k).map(|_| Note { event: t.u32(), param: t.u32() }).collect()
        })
        .collect();
    let n = t.int();
    sc.land = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    sc.result = (0..n).map(|_| t.int()).collect();
    let n = t.int();
    sc.attr = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    sc.hit = (0..n).map(|_| (t.i32(), t.u32(), t.u32())).collect();
    let n = t.int();
    sc.trans = (0..n).map(|_| t.int()).collect();
    let n = t.int();
    sc.center = (0..n).map(|_| vec4(t)).collect();
    let n = t.int();
    sc.draw = (0..n).map(|_| (t.int(), t.u32())).collect();
    let n = t.int();
    sc.eye = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    sc.line = (0..n).map(|_| t.u32()).collect();
    sc.open = t.u32();
}

impl Case {
    fn cx<'a>(&'a mut self, t: &'a Tables, kt: &'a KiteTables) -> Cx<'a> {
        Cx {
            t,
            kt,
            scene: &mut self.scene,
            party: &self.party,
            save: &mut self.save,
            crew: &mut self.crew,
            game: &self.game,
            ents: &self.ents,
            env: &self.env,
            input: &self.input,
            p: &mut self.p,
            rng: &mut self.rng,
            w: &mut self.script,
        }
    }

    fn ai_ctx<'a>(&'a mut self, t: &'a Tables) -> Ctx<'a> {
        Ctx {
            t,
            scene: &mut self.scene,
            party: &self.party,
            save: &mut self.save,
            crew: &mut self.crew,
            game: &self.game,
            ents: &self.ents,
            rng: &mut self.rng,
            rt: &mut self.script,
        }
    }

    /// What the rest of the game does to Kite between two frames, as the
    /// harness's `poke` does it in the game's memory.
    fn poke(&mut self, kind: i64, a: i64, b: i64) {
        use piney_battle::chara::spc_flag::*;
        let me = self.me;
        let c = &mut self.scene.chars[me];
        match kind {
            0 => {
                c.skill_id = a as i16;
                c.skill_status = b as i16;
            }
            1 => c.target_char = usize::try_from(a).ok(),
            2 => {
                c.spc_char.act_num = a as i16;
                c.spc_char.flags = (c.spc_char.flags | STOP | RESTRAINT) & !(MOVE | TRAJECTORY);
            }
            3 => {
                c.spc_char.act_num = 9;
                c.spc_char.flags = (c.spc_char.flags | STOP | RESTRAINT) & !(MOVE | TRAJECTORY);
                c.skill_id = 0;
                c.skill_status = 0;
                c.cond.v[0] = 2;
                c.spc_char.cnt = 90;
            }
            4 => {
                if matches!(c.spc_char.act_num, 9 | 10) {
                    c.spc_char.act_num = 2;
                }
                c.skill_id = 0;
                c.skill_status = 0;
                c.cond.v[0] = 5;
                c.spc_char.cnt = 0;
                c.spc_char.cloak = 0;
            }
            5 => self.input.pad = Pad { pow_l: a as u8, dirc_l: b as u32 },
            6 => c.cond.v[a as usize] = b as i16,
            7 => self.script.cam_type = a as i32,
            8 => c.spc_char.flags ^= (a as u32) & 0x3f7f,
            _ => {}
        }
        kite::to_spc(&self.scene, &mut self.crew, me);
    }

    fn kite_json(&self) -> String {
        let me = self.me;
        let c = &self.scene.chars[me];
        let s = self.crew.spc.get(&me).copied().unwrap_or_default();
        let p = &self.p;
        let mut v: Vec<i64> = vec![
            i64::from(c.spc_char.flags),
            c.spc_char.act_num.into(),
            c.spc_char.act_num_old.into(),
            s.atk_anm_cnt.into(),
            c.anm_flag.into(),
            s.act_cnt.into(),
            s.react_cnt.into(),
            c.spc_char.attack.into(),
            s.transfer_lag.into(),
            c.spc_char.cloak.into(),
            c.spc_char.cnt.into(),
            c.spc_char.arms_effect_sw.into(),
            c.skill_id.into(),
            c.skill_status.into(),
            show(c.target_char),
        ];
        for x in c.pos.iter().chain(&c.pos_p).chain(&s.dirc) {
            v.push((*x).into());
        }
        v.extend([i64::from(s.speed), i64::from(s.speed_rate), i64::from(s.now_speed)]);
        v.extend(s.move_pos.iter().map(|&x| i64::from(x)));
        v.extend([i64::from(s.cycle), i64::from(s.stop_cnt), i64::from(s.walk_run_cnt)]);
        v.extend([
            i64::from(s.body_hit.sw),
            i64::from(s.hit_attribute),
            i64::from(s.transparency),
            i64::from(s.set_transparency),
        ]);
        let pf = i64::from(p.camera_flag) | i64::from(p.inactive) << 1 | i64::from(p.install) << 2;
        v.extend([pf, i64::from(p.ctrl_type), i64::from(p.prog_ctrl_flag), i64::from(p.dist_tg), i64::from(p.dirc_tg)]);
        for x in p.pos_view.iter().chain(&p.pos_eye).chain(&p.angle).chain(&p.disk_offset) {
            v.push((*x).into());
        }
        v.extend([
            i64::from(p.target_count),
            i64::from(p.act_one_two_cnt),
            i64::from(p.stress),
            i64::from(p.frame_spd),
        ]);
        list(v)
    }

    fn answer(&self, ret: i64) -> String {
        let ais: Vec<String> = self.crew.ais.iter().map(|(k, a)| format!("\"{k}\":{}", ai_json(a))).collect();
        let spc: Vec<String> = self.crew.spc.iter().map(|(k, s)| format!("\"{k}\":{}", spc_json(s))).collect();
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
        let chars: Vec<String> = self.scene.chars.iter().map(state).collect();
        let affect: Vec<String> = self
            .scene
            .chars
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let a = &c.affect;
                let extra = if i == self.me { [0, 0] } else { [i64::from(c.spc_char.fellow_flags), 0] };
                let enemy = if c.is_foe() { i64::from(c.spc_char.enemy_flags) } else { extra[1] };
                list([
                    i64::from(a.ty),
                    i64::from(a.param[0]),
                    i64::from(a.param[1]),
                    i64::from(a.param[2]),
                    show(a.person),
                    i64::from(a.color_cnt),
                    i64::from(a.color_rate),
                    i64::from(a.color),
                    i64::from(a.cond_color_cnt),
                    i64::from(a.cond_color_rate),
                    i64::from(a.cond_color),
                    if c.is_pc() { extra[0] } else { 0 },
                    enemy,
                ])
            })
            .collect();
        let sc = &self.script;
        format!(
            "{{\"ret\":{},\"rand\":{},\"calls\":[{}],\"ais\":{{{}}},\"sys\":{},\"spc\":{{{}}},\"items\":{{{}}},\
             \"globals\":{},\"chars\":[{}],\"affect\":[{}],\"lists\":[{},{},{}],\"kite\":{},\"cam\":[{}],\
             \"plw\":[{},{}],\"open\":{}}}",
            ret,
            self.rng.0,
            sc.calls.join(","),
            ais.join(","),
            sys_json(&self.crew.sys),
            spc.join(","),
            items.join(","),
            list([
                self.crew.select_attack_skill_result,
                self.crew.check_skill_other_flag,
                self.crew.check_heal_skill_other_flag,
                self.crew.ocarina_use_flag
            ]),
            chars.join(","),
            affect.join(","),
            list(self.scene.pc_list.iter()),
            list(self.scene.ene_list.iter()),
            list(self.scene.obj_list.iter()),
            self.kite_json(),
            sc.cam_reset,
            sc.plw.0,
            u8::from(sc.plw.1),
            self.open
        )
    }
}

impl Case {
    /// `ccPlayer::Main` with the party AI's calls performed by the ported
    /// code: a [`Host`] over the script (its world and, for what the
    /// movement passes on, its runtime).
    fn main_with_movement(&mut self, t: &Tables, kt: &KiteTables, keep: &mut Keep) {
        let Case { scene, ents, party, game, save, crew, rng, script, me, p, input, env, .. } = self;
        let cell = RefCell::new(script);
        let mut host = Host { t, mt: motion_tables(), party, game, keep, spc_registry_num: 0, world: &cell };
        let mut cx = Cx { t, kt, scene, party, save, crew, game, ents, env, input, p, rng, w: &mut host };
        kite::main(&mut cx, *me);
    }
}

/// `MainRunAi`: FRAMES frames of `Main` through [`Case::main_with_movement`],
/// the state after each (the calls each frame made).
fn main_run_ai(w: &mut Case, pokes: &[[i64; 4]], n: i64, t: &Tables, kt: &KiteTables) -> String {
    let mut keep = Keep::default();
    keep.follow.ai_open_dirc = w.script.open;
    let mut frames = Vec::new();
    for frame in 0..n {
        for p in pokes.iter().filter(|p| p[0] == frame) {
            w.poke(p[1], p[2], p[3]);
        }
        w.main_with_movement(t, kt, &mut keep);
        w.open = keep.follow.ai_open_dirc;
        frames.push(w.answer(0));
        w.script.calls.clear();
    }
    format!("{{\"frames\":[{}]}}", frames.join(","))
}

/// `kite frames BOUNDS PLAYER POINT`: the frame conversions.
fn frames(t: &mut Toks) -> String {
    let b: [u32; 4] = std::array::from_fn(|_| t.u32());
    let b = MapBounds { min: [b[0], b[1]], max: [b[2], b[3]] };
    let me = vec4(t);
    let p = vec4(t);
    let with = |(v, r): (V4, bool)| {
        let mut l: Vec<i64> = v.iter().map(|&x| i64::from(x)).collect();
        l.push(i64::from(r));
        list(l)
    };
    let (w2m, w2p, p2w) = (kite::w2m_pos(&b, p), kite::w2p_pos(&b, me, p), kite::p2w_pos(&b, me, p));
    format!(
        "{{\"W2MPos\":{},\"W2PPos\":{},\"P2WPos\":{},\"ccTransPosW2M\":{},\"ccTransPosW2P\":{},\
         \"ccTransPosP2W\":{},\"ccTransPosFW2LW\":{}}}",
        with(w2m),
        with(w2p),
        with(p2w),
        floats(&w2m.0),
        floats(&w2p.0),
        floats(&p2w.0),
        floats(&kite::fw2lw(&b, me, p))
    )
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    if cmd != "kite" {
        return None;
    }
    let f = t.word().to_string();
    if f == "frames" {
        return Some(frames(t));
    }
    let n = t.int();
    let args: Vec<i64> = (0..t.int()).map(|_| t.int()).collect();
    let mut w = read_world(t);
    read_kite(t, &mut w);
    read_script(t, &mut w.script);
    let pokes: Vec<[i64; 4]> = (0..t.int()).map(|_| std::array::from_fn(|_| t.int())).collect();
    let tb: &Tables = tables;
    let kt = kite_tables();
    let me = w.me;
    w.open = w.script.open;
    if f == "MainRunAi" {
        return Some(main_run_ai(&mut w, &pokes, n, tb, kt));
    }
    let o = |v: i64| if v < 0 { None } else { Some(v as usize) };
    let mut ret: i64 = 0;
    for frame in 0..n {
        for p in pokes.iter().filter(|p| p[0] == frame) {
            w.poke(p[1], p[2], p[3]);
        }
        ret = match f.as_str() {
            "AnimCtrl__8ccPlayerFv" => {
                kite::anim_ctrl(&mut w.cx(tb, kt), me);
                0
            }
            "Main__8ccPlayerFv" => {
                kite::main(&mut w.cx(tb, kt), me);
                0
            }
            "ControlMove__8ccPlayerFv" => kite::control_move(&mut w.cx(tb, kt), me).into(),
            "Attack__8ccPlayerFP6ccChari" => {
                let r = kite::attack(&mut w.ai_ctx(tb), me, args[0] as usize, args[1] as i32);
                r.into()
            }
            "CheckNote" => {
                let note = Note { event: args[0] as u32, param: args[1] as u32 };
                kite::check_note(&mut w.cx(tb, kt), me, note);
                kite::to_spc(&w.scene, &mut w.crew, me);
                0
            }
            "AttackCancel" => {
                kite::attack_cancel(&mut w.scene, &mut w.crew, me);
                0
            }
            "BreakSomething" => {
                kite::break_something(&mut w.cx(tb, kt), me, o(args[0]));
                0
            }
            "DamageActuate" => {
                if let Some(out) = kite::damage_actuate(kt, w.scene.chars[me].spc_char.act_num, args[0] as i32) {
                    w.script.out(out);
                }
                0
            }
            "ResultOfConditions" => {
                kite::result_of_conditions(&w.scene.chars[me], &mut w.p);
                0
            }
            "ccPlayerMenuCheck" => i64::from(kite::menu_check(&w.party, &w.scene, me, args[0] as i32)),
            "CheckControlMode" => i64::from(kite::check_control_mode(&w.crew, me)),
            "SetTargetDist" => {
                kite::set_target_dist(&mut w.scene, me, &mut w.p);
                kite::to_spc(&w.scene, &mut w.crew, me);
                0
            }
            "SetTargetDirc" => {
                kite::set_target_dirc(&mut w.cx(tb, kt), me);
                kite::to_spc(&w.scene, &mut w.crew, me);
                0
            }
            "ReadSysMsg2" => {
                let won = crate::probe_volume() != piney_data::volume::Volume::Inf && w.game.spc_battle_condition == 5;
                kite::read_sys_msg2(&mut w.crew, me, won);
                0
            }
            _ => return Some(format!("{{\"error\":\"unknown kite function {f}\"}}")),
        };
    }
    Some(w.answer(ret))
}
