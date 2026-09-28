//! `battle_probe` requests for the party's AI (`piney_battle::party_ai`), as
//! `tools/test_battle_party_ai_rs.py` sends them: `pai FN ARGS... WORLD`, and
//! `chat FN N ME ARGS... SAVE220D AREAPREV SERVER K TRICKS[K] L TEXT[L] WORLD`
//! for a chat line ([`piney_battle::party_chat`]). `WORLD` is every character
//! (the `read_char` format and its motion fields), the lists, the party, the
//! game globals, the save's lists, the AIs, the bus, the AI globals, the RNG
//! and the scripts of `ccHitCheckLM` and `CheckGoalBeaconPos`. The answer is
//! everything the call can change (a chat also each AI's text and the balloon).

use std::collections::VecDeque;

use piney_battle::exp::Party;
use piney_battle::party_ai::*;
use piney_battle::rand::{Rand, Rng};
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_data::save::SaveData;

use crate::{Toks, list, read_char};

/// The runtime of a check: the calls recorded as the harness records the
/// game's, the queries answered from the scripts.
struct Script {
    names: Vec<String>,
    hit: VecDeque<u32>,
    goal: VecDeque<i32>,
    calls: Vec<String>,
    /// A chat check's: each character's enemy skills that change
    /// conditions; the characters named `N<index>`.
    tricks: Vec<i32>,
    chat_names: bool,
}

impl Script {
    fn who(&self, c: Option<usize>) -> String {
        match c {
            Some(i) => format!("\"{}\"", self.names[i]),
            None => "0".into(),
        }
    }
}

fn floats(v: &[u32]) -> String {
    list(v.iter().copied())
}

impl Runtime for Script {
    fn call(&mut self, c: Call, _scene: &mut Scene, _crew: &mut Crew, _rng: &mut dyn Rng) -> i32 {
        let (line, ret) = match c {
            Call::SkillRequest { me, target, sid } => {
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
                let r = self.hit.pop_front().unwrap_or(F_MINUS_ONE);
                (format!("[\"ccHitCheckLM\",{},{},{}]", floats(&from), floats(&to), mask), r as i32)
            }
            Call::FaceTalk { me, g_deg } => (format!("[\"FaceTalk\",{},{}]", self.who(Some(me)), g_deg as i16), 0),
            Call::ManualControl { me } => (format!("[\"ManualControl\",{}]", self.who(Some(me))), 0),
            Call::ActInTown { me } => (format!("[\"ActInTown\",{}]", self.who(Some(me))), 0),
            Call::ChatMessageSender { me } => (format!("[\"ChatMessageSender\",{}]", self.who(Some(me))), 0),
            Call::ManualModeAi { ch, ev } => (format!("[\"ManualModeAI\",{},{}]", self.who(Some(ch)), ev), 0),
            Call::TransferOut { ch } => (format!("[\"TransferOut\",{}]", self.who(Some(ch))), 0),
            // navi: the AI's own movers' calls.
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

    fn char_name(&mut self, t: &Tables, save: &SaveData, scene: &Scene, ch: usize) -> Vec<u8> {
        if self.chat_names {
            format!("N{ch}").into_bytes()
        } else {
            piney_battle::party_chat::char_name(t, save, scene, ch)
        }
    }

    fn foe_condition_skills(&mut self, ch: usize) -> i32 {
        self.tricks.get(ch).copied().unwrap_or(0)
    }
}

pub(crate) fn opt(t: &mut Toks) -> Option<usize> {
    let v = t.int();
    if v < 0 { None } else { Some(v as usize) }
}

pub(crate) fn show(c: Option<usize>) -> i64 {
    c.map_or(-1, |c| c as i64)
}

pub(crate) fn read_msg(t: &mut Toks) -> SysMsg {
    SysMsg {
        name: t.i32(),
        param: t.i32(),
        pointer: opt(t),
        state: t.i16(),
        sender: t.int() as u16,
        receiver: t.int() as u16,
        priority: t.int() as u16,
        delivery_time: t.u32(),
    }
}

pub(crate) fn msg_json(m: &SysMsg) -> String {
    format!(
        "[{},{},{},{},{},{},{},{}]",
        m.name,
        m.param,
        show(m.pointer),
        m.state,
        m.sender,
        m.receiver,
        m.priority,
        m.delivery_time
    )
}

pub(crate) fn read_ai(t: &mut Toks, key: usize) -> Ai {
    let mut rng = || 0;
    let mut a = Ai::new(key, 0, 0, 0, &mut rng);
    a.set_flags(t.u32());
    a.strategy_cmd = t.i16();
    a.strategy = t.i16();
    a.mode = t.i32();
    a.mode_old = t.i32();
    a.count = t.i32();
    a.param = t.int() as usize;
    a.body = t.int() as usize;
    a.target = opt(t);
    a.target_ccmd = opt(t);
    a.remote_cmd = t.i16();
    a.chat_cmd = t.i16();
    a.chat_cmd_new = t.i16();
    a.chat_cmd_skill = t.i16();
    a.chat_cmd_item = t.i32();
    a.arrival_chat_cnt = t.i16();
    a.chat_cmd_time = t.i16();
    a.atk_dellay = t.i16();
    a.act_type = t.i16();
    a.act_type_old = t.i16();
    a.act_time = t.i16();
    a.act_step = t.i16();
    a.act_cnt = t.i16();
    a.atk_msg_cnt = t.int() as i8;
    a.dmg_msg_cnt = t.int() as i8;
    a.act_dummy = t.i16();
    a.act_skill = t.i16();
    a.last_marker = t.i16();
    a.no_move_cnt = t.i16();
    a.g_deg = t.int() as u16;
    a.g_rot_sp = t.i16();
    a.g_point = t.i16();
    a.dist_pl = t.u32();
    a.dirc_pl = t.u32();
    a.dist_tg = t.u32();
    a.dirc_tg = t.u32();
    a.attack_cycle = t.i16();
    a.atk_target_cnt = t.i16();
    a.detour_cnt = t.i16();
    a.level_old = t.i16();
    a.territory = t.u32();
    a.attack_range = t.u32();
    a.stop_range = t.u32();
    a.no_turn_range = t.u32();
    a.pos_old = std::array::from_fn(|_| t.u32());
    a.g_pos = std::array::from_fn(|_| t.u32());
    a.navi_finish = t.i16();
    let e = &mut a.sys_msg;
    e.id = t.i16();
    e.msg_max = t.i16();
    e.msg_num = t.i16();
    e.msg_top = t.i16();
    e.msg_tail = t.i16();
    for m in e.msg.iter_mut() {
        *m = read_msg(t);
    }
    if crate::probe_volume() != piney_data::volume::Volume::Inf {
        a.hit_recent = t.i16();
        a.hit_max = t.i16();
        a.cmd_attack_target = opt(t);
    }
    a
}

pub(crate) fn ai_json(a: &Ai) -> String {
    format!("[{}]", ai_fields(a).join(","))
}

/// The AI's fields as [`ai_json`] lists them (the navigation harness adds
/// its own after them).
pub(crate) fn ai_fields(a: &Ai) -> Vec<String> {
    let e = &a.sys_msg;
    [
        i64::from(a.flags()),
        a.strategy_cmd.into(),
        a.strategy.into(),
        a.mode.into(),
        a.mode_old.into(),
        a.count.into(),
        a.param as i64,
        a.body as i64,
        show(a.target),
        show(a.target_ccmd),
        a.remote_cmd.into(),
        a.chat_cmd.into(),
        a.chat_cmd_new.into(),
        a.chat_cmd_skill.into(),
        a.chat_cmd_item.into(),
        a.arrival_chat_cnt.into(),
        a.chat_cmd_time.into(),
        a.atk_dellay.into(),
        a.act_type.into(),
        a.act_type_old.into(),
        a.act_time.into(),
        a.act_step.into(),
        a.act_cnt.into(),
        a.atk_msg_cnt.into(),
        a.dmg_msg_cnt.into(),
        a.act_dummy.into(),
        a.act_skill.into(),
        a.last_marker.into(),
        a.no_move_cnt.into(),
        a.g_deg.into(),
        a.g_rot_sp.into(),
        a.g_point.into(),
        a.dist_pl.into(),
        a.dirc_pl.into(),
        a.dist_tg.into(),
        a.dirc_tg.into(),
        a.attack_cycle.into(),
        a.atk_target_cnt.into(),
        a.detour_cnt.into(),
        a.level_old.into(),
        a.territory.into(),
        a.attack_range.into(),
        a.stop_range.into(),
        a.no_turn_range.into(),
        a.pos_old[0].into(),
        a.pos_old[1].into(),
        a.pos_old[2].into(),
        a.pos_old[3].into(),
        a.g_pos[0].into(),
        a.g_pos[1].into(),
        a.g_pos[2].into(),
        a.g_pos[3].into(),
        a.navi_finish.into(),
        e.id.into(),
        e.msg_max.into(),
        e.msg_num.into(),
        e.msg_top.into(),
        e.msg_tail.into(),
    ]
    .iter()
    .map(|x| x.to_string())
    .chain(e.msg.iter().map(msg_json))
    .chain(later_ai(a))
    .collect()
}

/// Mutation's new ccAI fields, after the entry.
fn later_ai(a: &Ai) -> Vec<String> {
    if crate::probe_volume() == piney_data::volume::Volume::Inf {
        return Vec::new();
    }
    vec![a.hit_recent.to_string(), a.hit_max.to_string(), show(a.cmd_attack_target).to_string()]
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
struct World {
    scene: Scene,
    ents: Vec<usize>,
    party: Party,
    game: Game,
    save: SaveData,
    ids: Vec<i16>,
    crew: Crew,
    rng: Rand,
    script: Script,
}

fn read_world(t: &mut Toks) -> World {
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
    let hit = (0..k).map(|_| t.u32()).collect();
    let k = t.int();
    let goal = (0..k).map(|_| t.i32()).collect();
    let names = (0..n).map(|i| format!("c{i}")).collect();
    let script = Script { names, hit, goal, calls: vec![], tricks: Vec::new(), chat_names: false };
    World { scene, ents, party, game, save, ids, crew, rng, script }
}

impl World {
    fn ctx<'a>(&'a mut self, t: &'a Tables) -> Ctx<'a> {
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
        format!(
            "{{\"ret\":{},\"rand\":{},\"calls\":[{}],\"ais\":{{{}}},\"sys\":{},\"spc\":{{{}}},\"items\":{{{}}},\"globals\":{}}}",
            ret,
            self.rng.0,
            self.script.calls.join(","),
            ais.join(","),
            sys_json(&self.crew.sys),
            spc.join(","),
            items.join(","),
            list([
                self.crew.select_attack_skill_result,
                self.crew.check_skill_other_flag,
                self.crew.check_heal_skill_other_flag,
                self.crew.ocarina_use_flag
            ])
        )
    }
}

/// The line `f` names, with its arguments (`a(1)` on).
fn chat_of(f: &str, a: &dyn Fn(usize) -> i64, hp: i16) -> Option<Chat> {
    let o = |v: i64| if v < 0 { None } else { Some(v as usize) };
    let i = |k: usize| a(k) as i32;
    Some(match f {
        "ChatMessageConditionMinus" => Chat::ConditionMinus,
        "ChatMessageGhost" => Chat::Ghost,
        "ChatMessageCanNot" => Chat::CanNot,
        "ChatMessageOOM" => Chat::Oom,
        "ChatMessageNoBattleModeDeny" => Chat::NoBattleModeDeny,
        "ChatMessageAccept" => Chat::Accept,
        "ChatMessageDisableOcarinaDeny" => Chat::DisableOcarinaDeny,
        "ChatMessageNoOcarinaDeny" => Chat::NoOcarinaDeny,
        "ChatMessageEquipNOT" => Chat::EquipNot(i(1)),
        "ChatMessageChatCmdAccept" => Chat::ChatCmdAccept(i(1)),
        "ChatMessageVictory" => Chat::Victory,
        "ChatMessageUseOcarina" => Chat::UseOcarina,
        "ChatMessageHealStart" => Chat::HealStart,
        "ChatMessageCureStart" => Chat::CureStart,
        "ChatMessageResurrectStart" => Chat::ResurrectStart,
        "ChatMessageLevelDown" => Chat::LevelDown,
        "ChatMessageLevelUp" => Chat::LevelUp,
        "ChatMessageGratsLevelUp" => Chat::GratsLevelUp(o(a(1))),
        "ChatMessageDeadOtherFellow" => Chat::DeadOtherFellow(o(a(1))),
        "ChatMessageGhostCondition" => Chat::GhostCondition,
        "ChatMessageQuitPucciguso" => Chat::QuitPucciguso,
        "ChatMessageOpenTrapBox" => Chat::OpenTrapBox,
        "ChatMessageOpenTreasureBox" => Chat::OpenTreasureBox,
        "ChatMessagePresentOtherFellow" => Chat::PresentOtherFellow(o(a(1))),
        "ChatMessageTreatmentPlz" => Chat::TreatmentPlz,
        "ChatMessageWaitPlz" => Chat::WaitPlz,
        "ChatMessageWalkingTalk" => Chat::WalkingTalk,
        "ChatMessageNeutral" => Chat::Neutral,
        "ChatMessageHealPlz" => Chat::HealPlz,
        "ChatMessageAttackTarget" => Chat::AttackTarget(o(a(1))),
        "ChatMessageReencounter" => Chat::Reencounter,
        "ChatMessage" => {
            use piney_battle::ai_move::TownLine as L;
            let table = [
                L::Byebye,
                L::GotoWeaponShop,
                L::GotoMagicShop,
                L::GotoRecordShop,
                L::GotoGoodsShop,
                L::GotoFairyShop,
                L::GotoEtc,
            ]
            .into_iter()
            .find(|l| i64::from(l.table()) == a(1))?;
            Chat::Line(table, i(2))
        }
        "ChatMessageEnteredField" => Chat::EnteredField,
        "ChatMessageEnteredTown" => Chat::EnteredTown,
        "ChatMessageEquipOK" => Chat::EquipOk,
        "ChatMessageAttributeCritical" => Chat::AttributeCritical,
        "ChatMessageAttack" => Chat::Attack(o(a(1)), i(2), i(3)),
        "ChatMessageDamage" => Chat::Damage(i(1), hp),
        "ChatMessageResurrectPlz" => Chat::ResurrectPlz,
        "AffectMessages" => Chat::Affect(i(1), o(a(2)), i(3), hp),
        "ChatMessageConditionModify" => Chat::ConditionModify(i(1)),
        "ChatMessageAttributeGuard" => Chat::AttributeGuard(i(1)),
        "ChatMessageAttributeFollow" => Chat::AttributeFollow(i(1)),
        _ => return None,
    })
}

/// A `chat` request (see the module doc).
fn chat(t: &mut Toks, tables: &Tables) -> String {
    let f = t.word().to_string();
    let n = t.int() as usize;
    let args: Vec<i64> = (0..n).map(|_| t.int()).collect();
    let save220d = t.int();
    let area_prev = t.i32();
    let server = t.i32();
    let k = t.int() as usize;
    let tricks: Vec<i32> = (0..k).map(|_| t.i32()).collect();
    let l = t.int() as usize;
    let text: Vec<u8> = (0..l).map(|_| t.int() as u8).collect();
    let mut w = read_world(t);
    w.save.set_u8(0x220d, save220d as u8);
    w.game.area_prev = area_prev;
    w.game.server = server;
    w.script.tricks = tricks;
    w.script.chat_names = true;
    let me = args[0] as usize;
    if let Some(x) = w.crew.ais.get_mut(&me) {
        x.chat_text = text;
    }
    let a = |i: usize| args.get(i).copied().unwrap_or(0);
    let body = w.crew.ais.get(&me).map_or(me, |x| x.body);
    let hp = w.scene.chars.get(body).map_or(0, |c| c.hp);
    let mut opened: Option<Vec<u8>> = None;
    let ret: i64 = {
        let mut c = w.ctx(tables);
        match f.as_str() {
            "ChatMessageSender" => {
                opened = c.chat_sender(me);
                0
            }
            "Greeting" => {
                c.greeting(me, if a(1) < 0 { None } else { Some(a(1) as usize) });
                0
            }
            _ => match chat_of(&f, &a, hp) {
                Some(line) => c.chat_line(me, line).into(),
                None => return format!("{{\"error\":\"unknown chat function {f}\"}}"),
            },
        }
    };
    let mut out = w.answer(ret);
    out.pop();
    let texts: Vec<String> = w
        .crew
        .ais
        .iter()
        .map(|(k, x)| format!("\"{k}\":{}", list(x.chat_text.iter().map(|&b| i64::from(b)))))
        .collect();
    let dircs = list((0..w.scene.chars.len()).map(|i| w.crew.spc.get(&i).map_or(0, |s| s.dirc[2])));
    let opened = opened.map_or("null".to_string(), |v| list(v.iter().map(|&b| i64::from(b))));
    out.push_str(&format!(",\"texts\":{{{}}},\"dircs\":{dircs},\"opened\":{opened}}}", texts.join(",")));
    out
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    if cmd == "chat" {
        return Some(chat(t, tables));
    }
    if cmd != "pai" {
        return None;
    }
    let f = t.word().to_string();
    let args: Vec<i64> = (0..t.int()).map(|_| t.int()).collect();
    let mut w = read_world(t);
    let tb: &Tables = tables;
    let o = |v: i64| if v < 0 { None } else { Some(v as usize) };
    let a = |i: usize| args[i];
    let sys: Option<i64> = {
        let c = &mut w.crew;
        match f.as_str() {
            "SearchHistoryMessage" => Some(show(c.sys.search_history(a(1) as i32, a(2) as i32))),
            "SearchHistoryMessageP" => {
                Some(show(c.sys.search_history_p(a(1) as i32, a(2) as i32, a(3) as i32, o(a(4)))))
            }
            "CalcEstimateDamage" => Some(c.sys.calc_estimate_damage(a(1) as usize, a(2) as i32).into()),
            "DeleteDelayMessage" => Some(c.sys.delete_delay(a(1) as i32, a(2) as u16, a(3) as u16).into()),
            "SendMessage" => Some(c.send_message(a(1) as usize).into()),
            "ccAISysMsgSendP" => Some(
                c.send(a(1) as i32, a(2) as u16, a(3) as u16, a(4) as u16, a(5) as u16, a(6) as i32, o(a(7))).into(),
            ),
            "AddEntry" => Some(c.add_entry(a(1) as usize).into()),
            "ccAISysMsgWithdrawal" => {
                c.withdraw(a(1) as i16);
                Some(0)
            }
            _ => None,
        }
    };
    if let Some(r) = sys {
        return Some(w.answer(r));
    }
    let ret: i64 = {
        let mut c = w.ctx(tb);
        let me = a(0) as usize;
        match f.as_str() {
            "CheckSkillList" => i64::from(c.check_skill_list(me, a(1) as i32)),
            "CheckSkillList2" => c.check_skill_list2(me, a(1) as i32, a(2) as i32).into(),
            "CheckBuffSkillListNum" => c.check_buff_skill_list_num(me).into(),
            "CheckDebuffSkillListNum" => c.check_debuff_skill_list_num(me).into(),
            "CheckHealHpSkill" => c.check_heal_hp_skill(me).into(),
            "CheckHealParty" => c.check_heal_party(me, a(1) as i32).into(),
            "SearchHealSkill" => c.search_heal_skill(me, a(1) as usize).into(),
            "CheckItemList" => c.check_item_list(me, a(1) as i32).into(),
            "CheckItemList2" => c.check_item_list2(me, a(1) as i32).into(),
            "ConsumeItemList" => c.consume_item_list(me, a(1) as i32).into(),
            "SearchItemListBySkill" => c.search_item_list_by_skill(me, a(1) as i32).into(),
            "SearchItemListByHeal" => c.search_item_list_by_heal(me, a(1) as i32).into(),
            "SearchItemListByBuff" => i64::from(c.search_item_list_by_buff(me)),
            "SearchItemListByDebuff" => i64::from(c.search_item_list_by_debuff(me)),
            "SearchItemListByMagicAttack" => i64::from(c.search_item_list_by_magic_attack(me)),
            "CheckNeedHealing" => i64::from(c.check_need_healing(me, o(a(1)), a(2) as i32)),
            "CheckConditionMinus" => c.check_condition_minus(me, o(a(1)), a(2) as i32).into(),
            "CheckBossEntry" => show(c.check_boss_entry(a(1) as i32)),
            "CheckHealingSchedule" => c.check_healing_schedule(me).into(),
            "CheckSchedule" => c.check_schedule(me).into(),
            "CheckSolution" => i64::from(c.check_solution(a(1) as i32, o(a(2)))),
            "SysMsgFromMeToMe" => {
                c.sys_msg_from_me_to_me(me, a(1) as i32, a(2) as usize, a(3) as u16, a(4) as i32, o(a(5))).into()
            }
            "CheckSkillOtherSPC" => show(c.check_skill_other_spc(me, a(1) as i32)),
            "CheckHealSkillOtherSPC" => show(c.check_heal_skill_other_spc(me, a(1) as i32)),
            "SearchTarget" => show(c.search_target(me, a(1) as i32, a(2) as i32)),
            "SearchTargetNear" => show(c.search_target_near(me, a(1) as i32, a(2) as i32)),
            "CountTargetInArea" => c.count_target_in_area(me, a(1) as i32, a(2) as u32).into(),
            "levelCheck" => c.level_check(me).into(),
            "SearchDebuffUnusedEnemy" => show(c.search_debuff_unused_enemy(me, a(1) as i32)),
            "SearchBuffUnusedFellow" => show(c.search_buff_unused_fellow(me, a(1) as i32)),
            "DebuffForUnusedEnemy" => c.debuff_for_unused_enemy(me, a(1) as i32, a(2) as i32).into(),
            "BuffForUnusedFellow" => c.buff_for_unused_fellow(me, a(1) as i32, a(2) as i32).into(),
            "CheckTargetConditionBySkill" => c.check_target_condition_by_skill(a(1) as usize, a(2) as i32).into(),
            "ChangeMode" => {
                c.change_mode(me, a(1) as i32, a(2) as i32);
                0
            }
            "ChangeStrategy" => {
                c.change_strategy(me, a(1) as i16);
                0
            }
            "ChangeStrategyCMD" => {
                c.change_strategy_cmd(me, a(1) as i16);
                0
            }
            "StopNormalAttack" => c.stop_normal_attack(me).into(),
            "UseSkill" => c.use_skill(me, a(1) as i32, o(a(2))).into(),
            "UseItem" => c.use_item(me, a(1) as i32, o(a(2))).into(),
            "SelectAttackSkill" => c.select_attack_skill(me, a(1) as usize).into(),
            "CureSPC" => c.cure_spc(me, a(1) as usize, a(2) as i32).into(),
            "CureOtherSPC" => c.cure_other_spc(me, a(1) as i32).into(),
            "CureOnlyCharmConfusionOtherSPC" => {
                c.cure_only_charm_confusion_other_spc(me, args.get(1).copied().unwrap_or(0) as i32).into()
            }
            "CureOnlyParalysisSleepOtherSPC" => {
                c.cure_only_paralysis_sleep_other_spc(me, args.get(1).copied().unwrap_or(0) as i32).into()
            }
            "CurePoisonSPC" => c.cure_poison_spc(me).into(),
            "CurePoisonCurseSPC" => c.cure_poison_curse_spc(me, args.get(1).copied().unwrap_or(0) as i32).into(),
            "ItemFirst" => c.item_first(me).into(),
            "LiveMembers" => c.live_members().into(),
            "HealSPC" => c.heal_spc(me, o(a(1)), a(2) as u32, args.get(3).copied().unwrap_or(0) as i32).into(),
            "ResurrectSPC" => c.resurrect_spc(me, args.get(1).copied().unwrap_or(0) as i32).into(),
            "HealPlzNormalMode" => c.heal_plz_normal_mode(me).into(),
            "HealPlzBattleMode" => c.heal_plz_battle_mode(me).into(),
            "ChatCommandHealPlz" => c.chat_command_heal_plz(me).into(),
            "AttackTarget" => {
                c.attack_target(me, a(1) as usize);
                0
            }
            "RequestChatCmd" => {
                c.request_chat_cmd(me, a(1) as i32, o(a(2)), a(3) as i32);
                0
            }
            "ChatCommandFulfilCheck" => c.chat_command_fulfil_check(me).into(),
            "ChatCommand" => {
                c.chat_command(me);
                0
            }
            "ChatCommandExecute" => c.chat_command_execute(me).into(),
            "ChatCommandDeBuffPlz" => c.chat_command_debuff_plz(me).into(),
            "ChatCommandBuffPlz" => c.chat_command_buff_plz(me).into(),
            "Brains" => c.brains(me).into(),
            "ReadSysMsg" => c.read_sys_msg(me).into(),
            "Reconnoiter" => c.reconnoiter(me).into(),
            "ActInField" => c.act_in_field(me).into(),
            "ActInDungeon" => c.act_in_dungeon(me).into(),
            "CheckEyeLineToTarget" => i64::from(c.check_eye_line_to_target(me, a(1) as usize)),
            "FellowUseSkill" => c.fellow_use_skill(me, a(1) as i32, a(2) as usize).into(),
            "FellowAttack" => c.fellow_attack(me, a(1) as usize, a(2) as i32).into(),
            _ => return Some(format!("{{\"error\":\"unknown pai function {f}\"}}")),
        }
    };
    Some(w.answer(ret))
}
