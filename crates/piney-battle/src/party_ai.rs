//! The party members' decisions: `ccAI` (`personal.cpp`, gcmn
//! 0x0057c5f0-0x005977cc), the message bus `ccAISystem` it talks through
//! ([`AiSystem`], [`Crew::tick`]) and the rules half of `ccFellow`. A member's
//! AI is an [`Ai`] in a [`Crew`] under its character's scene index, with the
//! `ccSpcChar` fields in [`Spc`]; a decision is a method of [`Ctx`]. What the
//! decisions call beyond the rules (skills, items, movement, chat) is a
//! [`Call`] through [`Runtime::call`] where the game makes it. The rules and
//! the message kinds are in docs/engine/battle.md ("Party AI").

use std::collections::BTreeMap;

use piney_data::field::ee;
use piney_data::save::SaveData;
use piney_data::volume::Volume;

use crate::chara::{Char, cdiv, cmod};
use crate::damage::{fptosi, skill_damage_value};
use crate::exp::Party;
use crate::geom;
use crate::param::{F_ONE, cond};
use crate::party_chat::Remark;
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::target_condition_by_skill;
use crate::tables::Tables;

/// -1.0f: `ccHitCheckLM`'s "nothing in the way", and the searches' "no
/// best distance yet".
pub const F_MINUS_ONE: u32 = 0xbf80_0000;
/// 0.8f: the eye height, as a fraction of the character's height, a line
/// of sight is checked from and to.
const F_EYE: u32 = 0x3f4c_cccd;
/// 100000.0f, the distance to a character that is not on the lists.
pub const F_FAR: u32 = 0x47c3_5000;
/// `saveData.skillList[18][20]` (+0x1ec4) and `saveData.itemList[18][40]`
/// (+0x30).
pub const SAVE_SKILL_LIST: usize = piney_data::save::offset::SKILL_LIST;
pub const SAVE_ITEM_LIST: usize = piney_data::save::offset::ITEM_LIST;
pub const SKILLS_PER_MEMBER: usize = 20;
pub const ITEMS_PER_MEMBER: usize = 40;

// ---------------------------------------------------------------------------
// The message bus

/// `ccAISysMsg` (0x18 bytes). `pointer` is a character (a scene index), the
/// only thing the party's messages point at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SysMsg {
    pub name: i32,
    pub param: i32,
    pub pointer: Option<usize>,
    /// Non-zero while the message waits in `msgBuff`.
    pub state: i16,
    pub sender: u16,
    /// 0xffff: every entry.
    pub receiver: u16,
    /// Non-zero: delivered to the front of the queue.
    pub priority: u16,
    /// The `AiSystem::time` it is delivered at.
    pub delivery_time: u32,
}

impl Default for SysMsg {
    fn default() -> Self {
        SysMsg { name: -1, param: 0, pointer: None, state: 0, sender: 0, receiver: 0, priority: 0, delivery_time: 0 }
    }
}

/// `ccAIEntry` (`ccAI.sysMsg`, 0xfc bytes): an AI's id on the bus and its
/// queue of delivered messages, a ring of ten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiEntry {
    /// -1 while not entered.
    pub id: i16,
    pub msg_max: i16,
    pub msg_num: i16,
    pub msg_top: i16,
    pub msg_tail: i16,
    pub msg: [SysMsg; 10],
}

impl Default for AiEntry {
    fn default() -> Self {
        AiEntry { id: -1, msg_max: 0, msg_num: 0, msg_top: 0, msg_tail: 0, msg: [SysMsg::default(); 10] }
    }
}

impl AiEntry {
    /// `ccAISysMsgReceiveTop` (gcmn 0x0058c980): a priority message goes in
    /// front of the others; a full queue loses its newest to make room.
    /// Returns whether the queue grew.
    pub fn receive_top(&mut self, m: &SysMsg) -> bool {
        let mut taken = false;
        if self.msg_num < self.msg_max {
            self.msg_num += 1;
            taken = true;
        } else {
            self.msg_tail -= 1;
            if self.msg_tail < 0 {
                self.msg_tail = 9;
            }
        }
        self.msg_top -= 1;
        if self.msg_top < 0 {
            self.msg_top = 9;
        }
        self.msg[self.msg_top as usize] = *m;
        taken
    }

    /// `ccAISysMsgReceiveTail` (gcmn 0x0058ca50): a message goes behind the
    /// others, if there is room.
    pub fn receive_tail(&mut self, m: &SysMsg) -> bool {
        if self.msg_num >= self.msg_max {
            return false;
        }
        self.msg_num += 1;
        if self.msg_num >= 2 {
            self.msg_tail += 1;
            if self.msg_tail >= 10 {
                self.msg_tail = 0;
            }
        }
        self.msg[self.msg_tail as usize] = *m;
        true
    }
}

/// `ccAISystem` (`ccAISys`, 0x334 bytes): the messages waiting to be
/// delivered, the last sixteen delivered, and the five AIs entered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSystem {
    /// Frames since the bus started (the thread counts one a frame).
    pub time: u32,
    pub entry_num: i16,
    pub history_top: i16,
    pub next_id: i16,
    pub msg_buff: [SysMsg; 16],
    pub msg_history: [SysMsg; 16],
    /// `entry[i]` / `entryAI[i]`: the AI (by its body's scene index)
    /// whose `sysMsg` is entered in slot `i`.
    pub entry: [Option<usize>; 5],
}

impl Default for AiSystem {
    /// `ccAISystem::ccAISystem` (gcmn 0x0058bee0).
    fn default() -> Self {
        AiSystem {
            time: 0,
            entry_num: 0,
            history_top: 0,
            next_id: 0,
            msg_buff: [SysMsg::default(); 16],
            msg_history: [SysMsg::default(); 16],
            entry: [None; 5],
        }
    }
}

impl AiSystem {
    /// The first message waiting in `msgBuff` whose name matches.
    fn pending(&self, f: impl Fn(&SysMsg) -> bool) -> bool {
        self.msg_buff.iter().any(|m| m.state != 0 && f(m))
    }

    /// A waiting message of kind `kind` (the name's high half) for
    /// `receiver`.
    fn pending_for(&self, kind: i32, receiver: u16) -> bool {
        self.pending(|m| m.name as u32 & 0xffff_0000 == (kind as u32) << 16 && m.receiver == receiver)
    }

    /// The inline allocation every sender uses: the first free slot of
    /// `msgBuff`, cleared (param and pointer 0, state 1, sender and
    /// receiver 0xffff, priority 0).
    fn alloc(&mut self) -> Option<usize> {
        let i = self.msg_buff.iter().position(|m| m.state == 0)?;
        let m = &mut self.msg_buff[i];
        m.param = 0;
        m.pointer = None;
        m.state = 1;
        m.sender = 0xffff;
        m.receiver = 0xffff;
        m.priority = 0;
        Some(i)
    }

    /// `ccAISystem::SearchHistoryMessage(n, t)` (gcmn 0x0058c470): a
    /// delivered message named `n` among the last sixteen; with `t` other
    /// than -1, only those delivered at `time - t` or later, newest first
    /// (the walk stops at the first older one).
    pub fn search_history(&self, n: i32, t: i32) -> Option<usize> {
        self.search_history_by(t, |m| m.name == n)
    }

    /// `ccAISystem::SearchHistoryMessageP(n, p, t, ptr)` (gcmn 0x0058c560):
    /// the same, matching `param` too, and `pointer` unless `ptr` is null.
    pub fn search_history_p(&self, n: i32, p: i32, t: i32, ptr: Option<usize>) -> Option<usize> {
        self.search_history_by(t, |m| m.name == n && m.param == p && (ptr.is_none() || m.pointer == ptr))
    }

    /// Mutation's search of gcmn 0x005b4cc0: a message of kind `kind` (the
    /// name's high half) carrying `param`, waiting or delivered (all for
    /// `t` -1, else within `t` frames), whoever it names.
    pub fn search_kind_param(&self, kind: i32, param: i32, t: i32) -> bool {
        let hit = |m: &SysMsg| (m.name as u32 & 0xffff_0000) == kind as u32 && m.param == param;
        self.pending(hit) || self.search_history_by(t, hit).is_some()
    }

    /// Mutation's revive search (gcmn 0x005b4660, from `CheckSolution`):
    /// a revive of `tid` (the name's low half; 0xffff any) for `receiver`
    /// (0xffff any) - kind 5 or 0xb, kind 6 with skill 180, kind 7 with an
    /// item that casts it - waiting, or delivered: all sixteen for `t` -1,
    /// else within the last `t` frames. `item_skill` is an item's skill.
    pub fn search_revive(&self, tid: u16, receiver: u16, t: i32, item_skill: impl Fn(i32) -> i32) -> bool {
        let hit = |m: &SysMsg| {
            let kind = match (m.name as u32) >> 16 {
                5 | 0xb => true,
                6 => m.param == 180,
                7 => item_skill(m.param) == 180,
                _ => false,
            };
            kind && (tid == 0xffff || m.name as u16 == tid) && (receiver == 0xffff || m.receiver == receiver)
        };
        self.pending(hit) || self.search_history_by(t, hit).is_some()
    }

    fn search_history_by(&self, t: i32, f: impl Fn(&SysMsg) -> bool) -> Option<usize> {
        if t == -1 {
            return (0..16).find(|&i| f(&self.msg_history[i]));
        }
        let since = self.time.wrapping_sub(t as u32);
        let mut i = self.history_top as i32 - 1;
        if i < 0 {
            i = 15;
        }
        for _ in 0..16 {
            let m = &self.msg_history[i as usize];
            if m.delivery_time < since {
                break;
            }
            if f(m) {
                return Some(i as usize);
            }
            i -= 1;
            if i < 0 {
                i = 15;
            }
        }
        None
    }

    /// `ccAISystem::CalcEstimateDamage(p, t)` (gcmn 0x0058c6a0): the damage
    /// the party has announced (0x10008 messages) against `p` in the last
    /// `t` frames (all sixteen for -1).
    pub fn calc_estimate_damage(&self, p: usize, t: i32) -> i32 {
        let hit = |m: &SysMsg| m.name == 0x10008 && m.pointer == Some(p);
        let mut v = 0i32;
        if t == -1 {
            for m in self.msg_history.iter().filter(|m| hit(m)) {
                v = v.wrapping_add(m.param);
            }
            return v;
        }
        let since = self.time.wrapping_sub(t as u32);
        let mut i = self.history_top as i32 - 1;
        if i < 0 {
            i = 15;
        }
        for _ in 0..16 {
            let m = &self.msg_history[i as usize];
            if m.delivery_time < since {
                break;
            }
            if hit(m) {
                v = v.wrapping_add(m.param);
            }
            i -= 1;
            if i < 0 {
                i = 15;
            }
        }
        v
    }

    /// `ccAISystem::DeleteDelayMessage(n, s, r)` (gcmn 0x0058c7b0): drop the
    /// waiting messages named `n` from `s` to `r` (0xffff matching any).
    pub fn delete_delay(&mut self, n: i32, s: u16, r: u16) -> i32 {
        let mut k = 0;
        for m in self.msg_buff.iter_mut() {
            if m.state != 0 && m.name == n && (s == 0xffff || m.sender == s) && (r == 0xffff || m.receiver == r) {
                m.state = 0;
                k += 1;
            }
        }
        k
    }
}

/// The party AI's world: the bus, each driven character's [`Ai`] and the
/// `ccSpcChar` fields of the party members ([`Spc`]), by scene index, and
/// the globals the decisions leave for their callers.
#[derive(Clone, Debug, Default)]
pub struct Crew {
    pub sys: AiSystem,
    pub ais: BTreeMap<usize, Ai>,
    pub spc: BTreeMap<usize, Spc>,
    /// `SelectAttackSkillResult`: the skill or item [`Ctx::select_attack_skill`]
    /// chose.
    pub select_attack_skill_result: i32,
    /// `checkSkillOtherSPCFlag`, `checkHealSkillOtherSPCFlag`: the item the
    /// member [`Ctx::check_skill_other_spc`] / [`Ctx::check_heal_skill_other_spc`]
    /// found would use, -1 for its own skill.
    pub check_skill_other_flag: i32,
    pub check_heal_skill_other_flag: i32,
    /// `ocarinaUseFlag` (0x00378cb0): the id of the member who played the
    /// Sprite Ocarina, 0 for none.
    pub ocarina_use_flag: i32,
    /// From Mutation on, the buff and debuff searches' candidates (gcmn MUT
    /// 0x00774730, 33 words): the nearest-so-far pushed in front (the first
    /// five kept), `[0]` and a terminator (`[5]` for members, `[32]` for
    /// foes) cleared at each search, the rest left from earlier ones.
    pub unused: Vec<Option<usize>>,
}

impl Crew {
    fn unused_at(&self, i: usize) -> Option<usize> {
        self.unused.get(i).copied().flatten()
    }

    fn set_unused(&mut self, i: usize, v: Option<usize>) {
        if self.unused.len() < 33 {
            self.unused.resize(33, None);
        }
        self.unused[i] = v;
    }
}

impl Crew {
    /// `ccAISystem::AddEntry(a, &a->sysMsg)` (gcmn 0x0058bfa0), through
    /// `ccAI::SysMsgEntry`: enter an AI on the bus under the next free id.
    /// Returns the id, or -1 with five entered.
    pub fn add_entry(&mut self, key: usize) -> i16 {
        let s = &mut self.sys;
        if s.entry_num >= 5 {
            return -1;
        }
        let slot = (0..5).find(|&i| s.entry[i].is_none()).unwrap_or(5);
        // Skip ids in use (the game's walk restarts after every bump).
        'again: loop {
            for i in 0..5 {
                if let Some(k) = s.entry[i]
                    && self.ais.get(&k).is_some_and(|a| a.sys_msg.id == s.next_id)
                {
                    s.next_id = s.next_id.wrapping_add(1) & 0x7fff;
                    continue 'again;
                }
            }
            break;
        }
        // Slot 5 would write past the arrays: the game only gets here with
        // entryNum out of step with the slots.
        if slot < 5 {
            s.entry[slot] = Some(key);
        }
        let id = s.next_id;
        if let Some(a) = self.ais.get_mut(&key) {
            a.sys_msg.id = id;
            a.sys_msg.msg_max = 10;
            a.sys_msg.msg_num = 0;
            a.sys_msg.msg_top = 0;
            a.sys_msg.msg_tail = 0;
        }
        s.next_id = s.next_id.wrapping_add(1);
        s.entry_num += 1;
        id
    }

    /// `ccAISysMsgWithdrawal(id)` (gcmn 0x0058ccf0): take an AI off the bus.
    pub fn withdraw(&mut self, id: i16) {
        for i in 0..5 {
            if let Some(k) = self.sys.entry[i]
                && self.ais.get(&k).is_some_and(|a| a.sys_msg.id == id)
            {
                if let Some(a) = self.ais.get_mut(&k) {
                    a.sys_msg.id = -1;
                }
                self.sys.entry[i] = None;
                self.sys.entry_num -= 1;
                return;
            }
        }
    }

    /// The slot whose AI has `id` (the entry's signed id against the
    /// message's unsigned field, so 0xffff never matches).
    fn slot_of(&self, id: u16) -> Option<usize> {
        (0..5).find(|&i| {
            self.sys.entry[i]
                .is_some_and(|k| self.ais.get(&k).is_some_and(|a| i32::from(a.sys_msg.id) == i32::from(id)))
        })
    }

    /// `ccAISystem::SendMessage(m)` (gcmn 0x0058c0b0) on `msgBuff[i]`: due
    /// this frame and from an entered AI, the message goes to its receiver
    /// (every entry for 0xffff; the front of the queue with a priority) and
    /// into the history if anyone took it; the slot is freed either way.
    /// Returns 1 if taken, 0 if not, -1 if not due yet.
    pub fn send_message(&mut self, i: usize) -> i32 {
        let m = self.sys.msg_buff[i];
        if m.delivery_time != self.sys.time {
            return -1;
        }
        let taken = self.deliver(&m);
        if taken == -1 {
            let b = &mut self.sys.msg_buff[i];
            b.name = -1;
            b.state = 0;
            return -1;
        }
        let b = &mut self.sys.msg_buff[i];
        b.name = -1;
        b.state = 0;
        taken
    }

    /// `SendMessage` on a message outside `msgBuff` (the same rules; the
    /// caller's copy is what the game clears).
    fn deliver(&mut self, m: &SysMsg) -> i32 {
        if self.slot_of(m.sender).is_none() {
            return -1;
        }
        let mut taken = false;
        let top = m.priority != 0;
        let mut put = |crew: &mut Crew, k: usize| {
            if let Some(a) = crew.ais.get_mut(&k) {
                let ok = if top { a.sys_msg.receive_top(m) } else { a.sys_msg.receive_tail(m) };
                taken |= ok;
            }
        };
        if m.receiver == 0xffff {
            for i in 0..5 {
                if let Some(k) = self.sys.entry[i] {
                    put(self, k);
                }
            }
        } else if let Some(i) = self.slot_of(m.receiver)
            && let Some(k) = self.sys.entry[i]
        {
            put(self, k);
        }
        if taken {
            let h = self.sys.history_top as usize;
            self.sys.msg_history[h] = *m;
            self.sys.history_top += 1;
            if self.sys.history_top >= 16 {
                self.sys.history_top = 0;
            }
        }
        i32::from(taken)
    }

    /// One frame of `ccThAISystem` (gcmn 0x0058c850): the clock advances and
    /// every waiting message due now is sent.
    pub fn tick(&mut self) {
        self.sys.time = self.sys.time.wrapping_add(1);
        for i in 0..16 {
            if self.sys.msg_buff[i].state != 0 && self.send_message(i) == 1 {
                self.sys.msg_buff[i].name = -1;
                self.sys.msg_buff[i].state = 0;
            }
        }
    }

    /// `ccAISysMsgSend(name, state, sender, receiver, priority, delay)`
    /// (gcmn 0x0058cb10) and `ccAISysMsgSendP` (0x0058cbe0, with `param`
    /// and `pointer`): queue a message for `time + delay` and try it now.
    /// Returns what `SendMessage` returns, -1 with no free slot.
    #[allow(clippy::too_many_arguments)]
    pub fn send(
        &mut self,
        name: i32,
        sender: u16,
        receiver: u16,
        priority: u16,
        delay: u16,
        param: i32,
        pointer: Option<usize>,
    ) -> i32 {
        let Some(i) = self.sys.alloc() else { return -1 };
        let time = self.sys.time;
        let m = &mut self.sys.msg_buff[i];
        m.name = name;
        m.param = param;
        m.pointer = pointer;
        m.sender = sender;
        m.receiver = receiver;
        m.priority = priority;
        m.delivery_time = u32::from(delay).wrapping_add(time);
        self.send_message(i)
    }

    /// The inline "from me to me in 30 frames" the healing decisions build:
    /// a free slot filled with name, param and pointer, sender and receiver
    /// `me`, no priority; the message is then sent. Nothing with no slot.
    fn post(&mut self, name: i32, param: i32, pointer: Option<usize>, sender: u16, receiver: u16) {
        self.post_in(name, param, pointer, sender, receiver, 30);
    }

    /// [`Crew::post`] due in `delay` frames.
    fn post_in(&mut self, name: i32, param: i32, pointer: Option<usize>, sender: u16, receiver: u16, delay: u32) {
        let Some(i) = self.sys.alloc() else { return };
        let time = self.sys.time;
        let m = &mut self.sys.msg_buff[i];
        m.name = name;
        m.param = param;
        m.pointer = pointer;
        m.sender = sender;
        m.receiver = receiver;
        m.priority = 0;
        m.delivery_time = time.wrapping_add(delay);
        self.send_message(i);
    }

    /// `ccSpcChar::CheckSysMsgID(ch)` (gcmn 0x0059f530): the AI id of a
    /// character, -1 for one without an AI.
    pub fn sys_msg_id(&self, ch: usize) -> i16 {
        self.ais.get(&ch).map_or(-1, |a| a.sys_msg.id)
    }
}

// ---------------------------------------------------------------------------
// The AI

/// `ccAI` (0x260 bytes), the fields the decisions use. Characters are
/// scene indices; floats are bit patterns. Bytes 0-3 are bit fields (see
/// [`Ai::flags`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ai {
    pub manual_sw: bool,
    pub follow_sw: bool,
    pub talk_flag: bool,
    pub run_flag: bool,
    pub remote_flag: bool,
    pub go_back_flag: bool,
    pub invite_flag: bool,
    /// Only look after itself (strategy 3's healer keeps to the party).
    pub self_flag: bool,
    /// Signed 2-bit fields: `battleFlag` (1 in mode 3, -1 in mode 1),
    /// `targetFlag`.
    pub battle_flag: i8,
    pub target_flag: i8,
    /// Signed 3-bit: 0 none, -2 (6) a command just given, -1 (7) refused,
    /// 1 carrying one out, 2 waiting for the member to be free.
    pub chat_cmd_flag: i8,
    /// Set until the member has done something itself (lifts the mercy
    /// rule of [`Ctx::select_attack_skill`]).
    pub first_time: bool,
    pub chat_request: bool,
    /// What the member may use: 1 arts, 2 attack spells and items, 4
    /// healing, 8 debuffs, 16 buffs.
    pub skill_mask: u8,
    pub strategy_cmd: i16,
    pub strategy: i16,
    pub mode: i32,
    pub mode_old: i32,
    pub count: i32,
    /// `param`: the `spcAIParam` row (`ccAI::Imprint(spcID)`).
    pub param: usize,
    /// `bodyPtr`: the character driven (its key in the [`Crew`]).
    pub body: usize,
    pub target: Option<usize>,
    pub target_ccmd: Option<usize>,
    /// Mutation on (+0x24): the target of the attack command (11), taken
    /// from `targetCCmd` when the command is accepted. On Infection the
    /// attack follows `targetCCmd` itself ([`Ctx::attack_cmd_target`]).
    pub cmd_attack_target: Option<usize>,
    pub remote_cmd: i16,
    pub chat_cmd: i16,
    pub chat_cmd_new: i16,
    pub chat_cmd_skill: i16,
    pub chat_cmd_item: i32,
    pub arrival_chat_cnt: i16,
    pub chat_cmd_time: i16,
    pub atk_dellay: i16,
    pub act_type: i16,
    pub act_type_old: i16,
    pub act_time: i16,
    pub act_step: i16,
    pub act_cnt: i16,
    pub atk_msg_cnt: i8,
    pub dmg_msg_cnt: i8,
    pub act_dummy: i16,
    pub act_skill: i16,
    /// Mutation on (+0x9a): the biggest hit the member has taken lately
    /// (capped at half its maxHP); `ReadSysMsg` clears it when a battle
    /// ends. [`Ctx::item_first`] measures the party's danger by it.
    pub hit_recent: i16,
    /// Mutation on (+0x9c): the biggest hit since the last victory.
    pub hit_max: i16,
    pub last_marker: i16,
    pub no_move_cnt: i16,
    pub g_deg: u16,
    pub g_rot_sp: i16,
    pub g_point: i16,
    /// Distance and direction to the player and to the target, as the
    /// runtime's movement leaves them.
    pub dist_pl: u32,
    pub dirc_pl: u32,
    pub dist_tg: u32,
    pub dirc_tg: u32,
    pub attack_cycle: i16,
    pub atk_target_cnt: i16,
    pub detour_cnt: i16,
    /// `levelOld` (+0xb6): the constructor leaves it as the heap had it
    /// (`_ccMalloc` does not clear), so the first `levelCheck` outside act
    /// 14 compares against whatever was there. The runtime starts it at
    /// the character's level: no level line at an arrival.
    pub level_old: i16,
    pub territory: u32,
    pub attack_range: u32,
    pub stop_range: u32,
    pub no_turn_range: u32,
    pub pos_old: [u32; 4],
    pub g_pos: [u32; 4],
    /// `navi.finishFlag` (+0x10a): the navigation has brought the member
    /// to its target (movement state the attack decision reads and sets).
    pub navi_finish: i16,
    pub sys_msg: AiEntry,
    /// +0x24: the text [`Ctx::chat_line`] left for `ChatMessageSender`
    /// (80 bytes in the game).
    pub chat_text: Vec<u8>,
    // navi: the rest of the member's ccNavi.
    /// `navi` (+0xf0): the navigation ([`crate::navi::Navi`]); its
    /// `finishFlag` is [`Ai::navi_finish`].
    pub navi: crate::navi::Navi,
}

impl Ai {
    /// `ccAI::ccAI(spcID)` (gcmn 0x0057c5f0) with `Imprint(spcID)`: every
    /// flag clear, the strategy the party's (`partyStrategy`), `count` and
    /// the message counters from the RNG (`rand() >> 3`, then `(rand() >>
    /// 3) & 7` and its complement), `arrivalChatCnt` as the area sets it
    /// (150, or -1).
    pub fn new(body: usize, spc_id: usize, party_strategy: i16, arrival_chat_cnt: i16, rng: &mut dyn Rng) -> Ai {
        let count = rng.rand() >> 3;
        let atk = ((rng.rand() >> 3) & 7) as i8;
        Ai {
            manual_sw: false,
            follow_sw: false,
            talk_flag: false,
            run_flag: false,
            remote_flag: false,
            go_back_flag: false,
            invite_flag: false,
            self_flag: false,
            battle_flag: 0,
            target_flag: 0,
            chat_cmd_flag: 0,
            first_time: false,
            chat_request: false,
            skill_mask: 0,
            strategy_cmd: party_strategy,
            strategy: party_strategy,
            mode: 0,
            mode_old: 0,
            count,
            param: spc_id,
            body,
            target: None,
            target_ccmd: None,
            cmd_attack_target: None,
            remote_cmd: 0,
            chat_cmd: -1,
            chat_cmd_new: -1,
            chat_cmd_skill: 0,
            chat_cmd_item: 0,
            arrival_chat_cnt,
            chat_cmd_time: 0,
            atk_dellay: 0,
            act_type: 95,
            act_type_old: 0,
            act_time: 0,
            act_step: 0,
            act_cnt: 0,
            atk_msg_cnt: atk,
            dmg_msg_cnt: !atk & 7,
            act_dummy: 0,
            act_skill: 1,
            hit_recent: 0,
            hit_max: 0,
            last_marker: 0,
            no_move_cnt: 0,
            g_deg: 0,
            g_rot_sp: 64,
            g_point: 0,
            dist_pl: 0,
            dirc_pl: 0,
            dist_tg: 0,
            dirc_tg: 0,
            attack_cycle: 0,
            atk_target_cnt: 0,
            detour_cnt: 0,
            level_old: 0,
            territory: 0,
            attack_range: 0,
            stop_range: 0,
            no_turn_range: 0,
            pos_old: [0; 4],
            g_pos: [0; 4],
            navi_finish: 1,
            sys_msg: AiEntry::default(),
            chat_text: Vec::new(),
            // navi
            navi: crate::navi::Navi::new(),
        }
    }

    /// Mutation's record of a hit (gcmn 0x005c1a60, unnamed): `dmg`,
    /// capped at half of `max_hp` (the body's), raises [`Ai::hit_recent`]
    /// and [`Ai::hit_max`].
    pub fn note_hit(&mut self, dmg: i32, max_hp: i16) {
        let dmg = dmg.min(i32::from(max_hp) / 2);
        if i32::from(self.hit_recent) < dmg {
            self.hit_recent = dmg as i16;
        }
        if i32::from(self.hit_max) < dmg {
            self.hit_max = dmg as i16;
        }
    }

    /// Bytes 0-3 as the game lays them out: byte 0 manualSW followSW
    /// talkFlag runFlag remoteFlag goBackFlag inviteFlag selfFlag (bit 0
    /// up); byte 1 battleFlag (bits 0-1), targetFlag (2-3), chatCmdFlag
    /// (4-6), firstTime (7); byte 2 chatRequest (bit 0); byte 3 skillMask.
    pub fn flags(&self) -> u32 {
        let b0 = u32::from(self.manual_sw)
            | u32::from(self.follow_sw) << 1
            | u32::from(self.talk_flag) << 2
            | u32::from(self.run_flag) << 3
            | u32::from(self.remote_flag) << 4
            | u32::from(self.go_back_flag) << 5
            | u32::from(self.invite_flag) << 6
            | u32::from(self.self_flag) << 7;
        let b1 = (self.battle_flag as u32 & 3)
            | (self.target_flag as u32 & 3) << 2
            | (self.chat_cmd_flag as u32 & 7) << 4
            | u32::from(self.first_time) << 7;
        b0 | b1 << 8 | u32::from(self.chat_request) << 16 | u32::from(self.skill_mask) << 24
    }

    pub fn set_flags(&mut self, w: u32) {
        let bit = |n: u32| w >> n & 1 != 0;
        self.manual_sw = bit(0);
        self.follow_sw = bit(1);
        self.talk_flag = bit(2);
        self.run_flag = bit(3);
        self.remote_flag = bit(4);
        self.go_back_flag = bit(5);
        self.invite_flag = bit(6);
        self.self_flag = bit(7);
        self.set_battle_flag((w >> 8) as i32);
        self.set_target_flag((w >> 10) as i32);
        self.set_chat_cmd_flag((w >> 12) as i32);
        self.first_time = bit(15);
        self.chat_request = bit(16);
        self.skill_mask = (w >> 24) as u8;
    }

    pub fn set_battle_flag(&mut self, v: i32) {
        self.battle_flag = ((v << 30) >> 30) as i8;
    }

    pub fn set_target_flag(&mut self, v: i32) {
        self.target_flag = ((v << 30) >> 30) as i8;
    }

    pub fn set_chat_cmd_flag(&mut self, v: i32) {
        self.chat_cmd_flag = ((v << 29) >> 29) as i8;
    }

    /// The command given is dropped: `chatCmd = -1`, `chatCmdFlag = 0`.
    fn drop_chat_cmd(&mut self) {
        self.chat_cmd = -1;
        self.chat_cmd_flag = 0;
    }
}

/// The `ccSpcChar` / `ccFellow` fields of a party member the decisions
/// read and write beyond its [`Char`]: animation and movement state the
/// runtime keeps up to date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spc {
    /// `actNum` (+0xee): the action (animation) playing.
    pub act_num: i16,
    /// `ccChar.targetChar` (+0x78).
    pub target_char: Option<usize>,
    /// `moveFlag` (+0xe0 bit 3): moving under its own power.
    pub move_flag: bool,
    /// `nowSpeed` (+0x10c).
    pub now_speed: u32,
    /// `cycle` (+0x11c): the frame counter of the member's own task.
    pub cycle: i32,
    /// `ccFellow.distTg` (+0x20c): the distance to `targetChar` the last
    /// skill measured.
    pub dist_tg: u32,
    /// `SpcListNum` (+0xe8): the character's slot in the registry
    /// (`ccSpcManager`), which `ccSPC::Reboot` makes it with
    /// (`ccFellowNN(n)`); Kite's is 0, and `ChatCommand` holds a character
    /// of slot 0 to strategy 0.
    pub spc_list_num: i32,
    /// `runFlag` (+0xe0 bit 5): running.
    pub run_flag: bool,
    /// `ghostFlag` (+0xe0 bit 6): down but still acting as a ghost.
    pub ghost: bool,
    /// `stopFlag` (+0xe0 bit 4): told to stand still.
    pub stop_flag: bool,
    /// `stopCnt` (+0xfc): frames standing still.
    pub stop_cnt: i16,
    /// `walkRunCnt` (+0x124): frames walking or running.
    pub walk_run_cnt: i32,
    // The motion layer's members (movement and acts).
    /// `ccChar.dirc` (+0x60): z is the heading, radians; 0 faces -y.
    pub dirc: [u32; 4],
    /// `speed` (+0x104): the running speed (`personality->velocity`).
    pub speed: u32,
    /// `speedRate` (+0x108).
    pub speed_rate: u32,
    /// `movePos`: the move this frame (`ccFellow` +0x230, `ccPlayer`
    /// +0x290).
    pub move_pos: [u32; 4],
    // fellow and navi: the ccChar / ccSpcChar / ccFellow members of a
    // party member's frame (crate::fellow), its following (crate::follow)
    // and the AI's own movers (crate::ai_move).
    /// `ccChar.hitAttribute` (+0x80): the ground under the feet.
    pub hit_attribute: u32,
    /// `ccChar.transparency`, `setTransparency` (+0x88, +0x8c): `cloak`
    /// each frame, for the draw; `ManualControl` hides and shows the body
    /// through them.
    pub transparency: u32,
    pub set_transparency: u32,
    /// `atkAnmCnt` (+0xf2): frames since the normal attack's swing began.
    pub atk_anm_cnt: i16,
    /// `actCnt` (+0xf8): frames into the act (the transfers time by it).
    pub act_cnt: i16,
    /// `reactCnt` (+0xfa): frames idle, toward a fidget.
    pub react_cnt: i16,
    /// `transferLag` (+0x100): frames `actCnt` waits.
    pub transfer_lag: i16,
    /// `consecutiveCnt` (+0x120): frames between the combo's swings.
    pub consecutive_cnt: i32,
    /// `bodyHit` (+0x1a0): the body in the collision; `MoveP2P` measures
    /// from its `radius`, `ManualControl` enters it into the world
    /// (`HitEnable`).
    pub body_hit: crate::world::CharHit,
    /// `ccFellow.dispWait` (+0x204): frames until shown and listed.
    pub disp_wait: i32,
    /// `ccFellow.atkDellay` (+0x208): frames before the next attack.
    pub atk_dellay: i32,
    /// `ccFellow.diskOffset` (+0x220): its place on a carrier.
    pub disk_offset: [u32; 4],
    /// `ccFellow.motionTbl` (+0x240): the `charTbl` row (1-17) whose
    /// `fellowAnimTbl` names its acts' clips.
    pub motion: i16,
}

/// The globals the decisions consult.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Game {
    /// `ccGame.inBattle` (+0x58): 1 while fighting.
    pub in_battle: i32,
    /// `ccGame.area` (+0x14): 0 the Root Town, 1 a field, 2 a dungeon.
    pub area: i32,
    /// `ccGame` +0x28: non-zero in a dungeon where the Sprite Ocarina works.
    pub field: i32,
    /// `WORLD_MAN::GetFieldType()` and `GetFieldAttrb()` (the field's
    /// element, 0-5) of the world manager.
    pub field_type: i32,
    pub field_attr: i32,
    /// `ccMenuCtrl::CheckMenuType()`, -1 with no menu open.
    pub menu_type: i32,
    /// `eventMng` +0x78c: 1 while an event forbids the ocarina.
    pub event_lock: i32,
    /// `spcBattleCondition` (0x00378d00): 3 when a fight starts, 1 while
    /// it runs, 5 when it is won.
    pub spc_battle_condition: i16,
    /// `partyStrategy` (0x00378ce0): the strategy the party menu sets.
    pub party_strategy: i32,
    /// `plw` +0x20: Kite's character.
    pub player: Option<usize>,
    /// `ccGame` +0x24: 13 for the Root Town areas that act as a field.
    pub field_24: i32,
    /// `pgRideFlag` (0x00378cdc): 1 while riding the Grunty.
    pub pg_ride_flag: i32,
    /// `ccGame.areaPrev` (+0x18): 0 when the area was entered from the
    /// Root Town (the arrival's line).
    pub area_prev: i32,
    /// `ccGame.server` (+0x1c): the server, 0 Delta .. 4 Omega (the Root
    /// Town's arrival line).
    pub server: i32,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            in_battle: 1,
            area: 1,
            field: 0,
            field_type: 0,
            field_attr: 0,
            menu_type: -1,
            event_lock: 0,
            spc_battle_condition: 0,
            party_strategy: 0,
            player: None,
            field_24: 0,
            pg_ride_flag: 0,
            area_prev: 0,
            server: 0,
        }
    }
}

/// What the decisions ask of the rest of the game: every call into code
/// that is not rules, at the point the game makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// `ccSkillRequest(me, target, sid)` (gcmn 0x00572700, `_ccSkillRequest`
    /// with stype 0): start a skill. The runtime runs
    /// [`crate::skill::request`] and creates the `ccSkill`. Skill 0 ends a
    /// running normal attack.
    SkillRequest { me: usize, target: Option<usize>, sid: i32 },
    /// `ccUseItemRequest(me, target, item, 0)` (useitem.cpp): use an item.
    UseItemRequest { me: usize, target: usize, item: i32, flag: i32 },
    /// `ccAI::FollowTarget(target)` (gcmn 0x00582090): walk to the target.
    FollowTarget { me: usize, target: Option<usize> },
    /// `ccAI::FollowTargetDirc(target)` (gcmn 0x00582ad0): turn to face it.
    FollowTargetDirc { me: usize, target: Option<usize> },
    /// `ccAI::FollowPlayer()` (gcmn 0x00581580): walk after Kite.
    FollowPlayer { me: usize },
    /// `ccAI::LeavePlayer()` (gcmn 0x00581cb0): step away from Kite.
    LeavePlayer { me: usize },
    /// `ccNavi::PathFindingInDungeon(from, to)` (gcmn 0x005148e0) of the
    /// member's navigation: a route exists. Returns it.
    PathFinding { me: usize, from: [u32; 4], to: [u32; 4] },
    /// `ccAI::FollowBeacon()` (gcmn 0x00582930): walk the navigation route.
    FollowBeacon { me: usize },
    /// `ccAI::CheckGoalBeaconPos(pos)` (gcmn 0x005975b0): whether `pos` is
    /// at the route's goal. Returns it.
    GoalBeacon { me: usize, pos: [u32; 4] },
    /// `ccPlayer::Attack(target, strategy)` (gcmn 0x0059c580): Kite's own
    /// attack when his AI drives him.
    PlayerAttack { me: usize, target: usize, n: i32 },
    /// `ccSpcChar::ManualModeAI(ev)` (gcmn 0x0059eba0) on Kite's character:
    /// his AI takes manual control.
    ManualModeAi { ch: usize, ev: i32 },
    /// The facing a talking member (`talkFlag`) turns to: `ccSetDirc(&dirc[2],
    /// DEG2RAD(gDeg), 64)` on its body (`ccAI::Brains`).
    FaceTalk { me: usize, g_deg: u16 },
    /// `ccAI::ManualControl()` (gcmn 0x00580ef0): the member under manual
    /// control this frame. Returns its result.
    ManualControl { me: usize },
    /// `ccAI::ActInTown()` (gcmn 0x0057f660): the member in a town. Returns
    /// its result.
    ActInTown { me: usize },
    /// `ccAI::ChatMessageSender()` (gcmn 0x00586420): the member's kept
    /// line opened in its balloon and the arrival's count run down
    /// (`Ctx::chat_sender`).
    ChatMessageSender { me: usize },
    /// `ccSpcChar::TransferOut()` (gcmn 0x0059ea80): the member leaves the
    /// area (the Sprite Ocarina's second message).
    TransferOut { ch: usize },
    /// `ccHitCheckLM(from, to, mask)` (gcmn): the line of sight in a
    /// dungeon. Returns the float bits, -1.0 when nothing is in the way.
    HitCheckLm { from: [u32; 4], to: [u32; 4], mask: u32 },
    /// A `ccAI::ChatMessage*` of `me`'s AI.
    Chat { me: usize, msg: Chat },
    // navi: the calls of the AI's own movers (crate::ai_move).
    /// `ccSpcChar::TransferIn()` (gcmn 0x0059ea60): the member appears in
    /// the area (`ManualControl`'s remote command 3).
    TransferIn { ch: usize },
    /// `resignParty(id)` (gcmn 0x0059d150): the member with character id
    /// `id` leaves the party (`ManualControl`'s remote command 5).
    ResignParty { id: i32 },
    /// `disbandSpc(id)` (gcmn 0x005a0f50): a member of no party let go.
    DisbandSpc { id: i32 },
    /// `ccCharHit::HitEnable()` (main 0x00153310) on `ch`'s `bodyHit`
    /// ([`Spc::body_hit`]): into the world's list of bodies.
    HitEnable { ch: usize },
}

/// The chat lines the decisions and the rules call for
/// (`ccAI::ChatMessage*`): [`crate::party_chat`] picks each and keeps it
/// on the AI for `ChatMessageSender` ([`Ctx::chat_line`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chat {
    /// `ChatMessageConditionMinus` (0x0058e390).
    ConditionMinus,
    /// `ChatMessageGhost` (0x0058e2b0).
    Ghost,
    /// `ChatMessageCanNot` (0x0058e130).
    CanNot,
    /// `ChatMessageOOM` (0x0058e070).
    Oom,
    /// `ChatMessageNoBattleModeDeny` (0x0058e1f0).
    NoBattleModeDeny,
    /// `ChatMessageAccept` (0x0058dc80).
    Accept,
    /// `ChatMessageDisableOcarinaDeny` (0x0058e8d0).
    DisableOcarinaDeny,
    /// `ChatMessageNoOcarinaDeny` (0x0058e810).
    NoOcarinaDeny,
    /// `ChatMessageEquipNOT(stat)` (0x00590d50).
    EquipNot(i32),
    /// `ChatMessageChatCmdAccept(cmd)` (0x0058fb40).
    ChatCmdAccept(i32),
    /// `ChatMessageVictory` (0x0058eae0).
    Victory,
    /// `ChatMessageUseOcarina` (0x0058e750).
    UseOcarina,
    /// `ChatMessageHealStart` (0x0058ddd0), `ChatMessageCureStart`
    /// (0x0058deb0), `ChatMessageResurrectStart` (0x0058df90).
    HealStart,
    CureStart,
    ResurrectStart,
    /// `ChatMessageLevelDown` (0x00591a90), `ChatMessageLevelUp`
    /// (0x00591b50), `ChatMessageGratsLevelUp(ch)` (0x00592750).
    LevelDown,
    LevelUp,
    GratsLevelUp(Option<usize>),
    /// `ChatMessageDeadOtherFellow(name)` (0x00591e00): the name of `ch`.
    DeadOtherFellow(Option<usize>),
    /// `ChatMessageGhostCondition` (0x005919d0), `ChatMessageQuitPucciguso`
    /// (0x00592070), `ChatMessageOpenTrapBox` (0x00591ec0),
    /// `ChatMessageOpenTreasureBox` (0x00592210),
    /// `ChatMessagePresentOtherFellow(ch)` (0x00591c10),
    /// `ChatMessageTreatmentPlz` (0x0058f540), `ChatMessageWaitPlz`
    /// (0x005918f0), `ChatMessageWalkingTalk` (0x00592900),
    /// `ChatMessageNeutral` (0x0058ec30), `ChatMessageHealPlz` (0x0058f480),
    /// `ChatMessageAttackTarget(tp)` (0x0058d7b0).
    GhostCondition,
    QuitPucciguso,
    OpenTrapBox,
    OpenTreasureBox,
    PresentOtherFellow(Option<usize>),
    TreatmentPlz,
    WaitPlz,
    WalkingTalk,
    Neutral,
    HealPlz,
    AttackTarget(Option<usize>),
    // navi: ActInTown's lines.
    /// `ChatMessageReencounter` (0x0058f6e0): meeting Kite again (draws
    /// `rand()`).
    Reencounter,
    /// `ccAI::ChatMessage(table[index], 0, 0, 0)` (0x00586130): a line of
    /// one of the town's tables, `index` the AI's `MessageIndex()`.
    Line(crate::ai_move::TownLine, i32),
    // The lines the rest of the game calls for (a hit, an affect, the
    // arrival, the CHAT menu's equipment report), and the ones lines call.
    /// `ChatMessageEnteredField` (0x0058e990), `ChatMessageEnteredTown`
    /// (0x0058f810): `ChatMessageSender`'s arrival lines.
    EnteredField,
    EnteredTown,
    /// `ChatMessageEquipOK` (0x00590c90), from `ccSpcChar::ChangeEquipReport`.
    EquipOk,
    /// `ChatMessageAttributeCritical` (0x00590ea0), from a skill's damage.
    AttributeCritical,
    /// `ChatMessageAttack(tp, dmg, sid)` (0x00590790): the member's hit.
    Attack(Option<usize>, i32, i32),
    /// A remark Mutation adds ([`crate::party_chat::Remark`]), with the
    /// item's code for an item's.
    Remark(crate::party_chat::Remark, i32),
    /// `ChatMessageDamage(dmg)` (0x005901d0), from `ccFellow::Influence`,
    /// with the member's HP as the affect found it (Influence stores the
    /// new HP after the line).
    Damage(i32, i16),
    /// `ChatMessageResurrectPlz` (0x0058f600), from `ccFellow::Influence`.
    ResurrectPlz,
    /// `ccAI::AffectMessages(kind, from, n)` (0x00586570), from
    /// `ccFellow::Influence`: 7, 8 and 16 `ChatMessageThanksHeal(from, n)`,
    /// 17 `ThanksBuff(from)`, 20 `ThanksResurrect(from)`, 18
    /// `ChatMessageConditionModify(n)`; with the HP as the affect found it.
    Affect(i32, Option<usize>, i32, i16),
    /// `ChatMessageConditionModify(sid)` (0x00591380).
    ConditionModify(i32),
    /// `ChatMessageAttributeGuard(attr)` (0x00590f60),
    /// `ChatMessageAttributeFollow(attr)` (0x00591040).
    AttributeGuard(i32),
    AttributeFollow(i32),
}

impl Chat {
    /// The game function's name.
    pub fn name(&self) -> &'static str {
        match self {
            Chat::ConditionMinus => "ChatMessageConditionMinus",
            Chat::Ghost => "ChatMessageGhost",
            Chat::CanNot => "ChatMessageCanNot",
            Chat::Oom => "ChatMessageOOM",
            Chat::NoBattleModeDeny => "ChatMessageNoBattleModeDeny",
            Chat::Accept => "ChatMessageAccept",
            Chat::DisableOcarinaDeny => "ChatMessageDisableOcarinaDeny",
            Chat::NoOcarinaDeny => "ChatMessageNoOcarinaDeny",
            Chat::EquipNot(_) => "ChatMessageEquipNOT",
            Chat::ChatCmdAccept(_) => "ChatMessageChatCmdAccept",
            Chat::Victory => "ChatMessageVictory",
            Chat::UseOcarina => "ChatMessageUseOcarina",
            Chat::HealStart => "ChatMessageHealStart",
            Chat::CureStart => "ChatMessageCureStart",
            Chat::ResurrectStart => "ChatMessageResurrectStart",
            Chat::LevelDown => "ChatMessageLevelDown",
            Chat::LevelUp => "ChatMessageLevelUp",
            Chat::GratsLevelUp(_) => "ChatMessageGratsLevelUp",
            Chat::DeadOtherFellow(_) => "ChatMessageDeadOtherFellow",
            Chat::GhostCondition => "ChatMessageGhostCondition",
            Chat::QuitPucciguso => "ChatMessageQuitPucciguso",
            Chat::OpenTrapBox => "ChatMessageOpenTrapBox",
            Chat::OpenTreasureBox => "ChatMessageOpenTreasureBox",
            Chat::PresentOtherFellow(_) => "ChatMessagePresentOtherFellow",
            Chat::TreatmentPlz => "ChatMessageTreatmentPlz",
            Chat::WaitPlz => "ChatMessageWaitPlz",
            Chat::WalkingTalk => "ChatMessageWalkingTalk",
            Chat::Neutral => "ChatMessageNeutral",
            Chat::HealPlz => "ChatMessageHealPlz",
            Chat::AttackTarget(_) => "ChatMessageAttackTarget",
            Chat::Reencounter => "ChatMessageReencounter",
            Chat::Line(..) => "ChatMessage",
            Chat::EnteredField => "ChatMessageEnteredField",
            Chat::EnteredTown => "ChatMessageEnteredTown",
            Chat::EquipOk => "ChatMessageEquipOK",
            Chat::AttributeCritical => "ChatMessageAttributeCritical",
            Chat::Attack(..) => "ChatMessageAttack",
            Chat::Damage(..) => "ChatMessageDamage",
            Chat::ResurrectPlz => "ChatMessageResurrectPlz",
            Chat::Affect(..) => "AffectMessages",
            Chat::ConditionModify(_) => "ChatMessageConditionModify",
            Chat::AttributeGuard(_) => "ChatMessageAttributeGuard",
            Chat::AttributeFollow(_) => "ChatMessageAttributeFollow",
            // Unnamed in Mutation's code: named here by their tables.
            Chat::Remark(r, _) => match r {
                Remark::PhysicalTolerance => "ChatMessagePhysicalTolerance",
                Remark::MagicTolerance => "ChatMessageMagicTolerance",
                Remark::UseItem => "ChatMessageUseItem",
                Remark::UseLastItem => "ChatMessageUseLastItem",
                Remark::OnlyBuff => "ChatMessageOnlyBuff",
                Remark::OnlyDebuff => "ChatMessageOnlyDebuff",
            },
        }
    }

    /// Its character argument, if it takes one.
    pub fn who(&self) -> Option<Option<usize>> {
        match *self {
            Chat::GratsLevelUp(c)
            | Chat::DeadOtherFellow(c)
            | Chat::PresentOtherFellow(c)
            | Chat::AttackTarget(c)
            | Chat::Attack(c, ..)
            | Chat::Affect(_, c, ..) => Some(c),
            _ => None,
        }
    }

    /// Its integer argument, if it takes one.
    pub fn arg(&self) -> Option<i32> {
        match *self {
            Chat::EquipNot(v)
            | Chat::ChatCmdAccept(v)
            | Chat::Line(_, v)
            | Chat::Damage(v, _)
            | Chat::ConditionModify(v)
            | Chat::AttributeGuard(v)
            | Chat::AttributeFollow(v) => Some(v),
            Chat::Remark(Remark::UseItem | Remark::UseLastItem, code) => Some(code),
            _ => None,
        }
    }
}

/// `ccAI::DirectionOfTarget(tt)` (gcmn 0x00583030) for a target: the
/// heading from the body to `tt` in the player's frame, `atan2f(d.x, -d.y)`
/// of `d = tt.posP - body.posP`.
pub fn direction_of_target(scene: &Scene, me: usize, tt: usize) -> u32 {
    let (a, b) = (scene.chars[tt].pos_p, scene.chars[me].pos_p);
    let d0 = ee::sub(a[0], b[0]);
    let d1 = ee::sub(a[1], b[1]);
    piney_data::libm::atan2f(d0, piney_data::libm::neg(d1))
}

/// The rest of the game, as the decisions see it.
pub trait Runtime {
    /// Make `call`, returning what the game's function returns (0 for
    /// none). The runtime may change the scene, the AIs' movement state
    /// and draw from the RNG, as the game's code does at that point.
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32;
    /// `ccTransPosW2P(out, pos)` (`ccPlayer::W2PPos`, gcmn 0x0059b940): a
    /// world position in the player's frame.
    fn w2p(&mut self, pos: [u32; 4]) -> [u32; 4];
    /// `call` as a decision makes it, with the decision's whole context:
    /// a runtime that performs a call with the AI's own code (`ActInTown`,
    /// `ManualControl`, `FollowBeacon`, `ccPlayer::Attack`, which are
    /// methods of the same `ccAI` in the game) builds a [`Ctx`] from `p`
    /// ([`crate::party_motion::Movement`] does). The default passes it on
    /// to [`Runtime::call`].
    fn call_ctx(&mut self, call: Call, p: Parts<'_>) -> i32 {
        self.call(call, p.scene, p.crew, p.rng)
    }
    /// The name a chat line gives a character (`ccChar.base->name`,
    /// [`crate::party_chat::char_name`]).
    fn char_name(&mut self, t: &Tables, save: &SaveData, scene: &Scene, ch: usize) -> Vec<u8> {
        crate::party_chat::char_name(t, save, scene, ch)
    }
    /// How many of an enemy's skills (`skillList[0..skillNum]`, their
    /// `skiParam`) change conditions (type bit 0x20000), for
    /// `ChatMessageAttackTarget`; 0 for anything else.
    fn foe_condition_skills(&mut self, _ch: usize) -> i32 {
        0
    }
}

/// A [`Ctx`] without its runtime: what [`Runtime::call_ctx`] is handed.
pub struct Parts<'b> {
    pub t: &'b Tables,
    pub scene: &'b mut Scene,
    pub party: &'b Party,
    pub save: &'b mut SaveData,
    pub crew: &'b mut Crew,
    pub game: &'b Game,
    pub ents: &'b [usize],
    pub rng: &'b mut dyn Rng,
}

impl<'b> Parts<'b> {
    /// The [`Ctx`] of these parts with `rt` for its runtime.
    pub fn ctx<'r>(self, rt: &'r mut dyn Runtime) -> Ctx<'r>
    where
        'b: 'r,
    {
        Ctx {
            t: self.t,
            scene: self.scene,
            party: self.party,
            save: self.save,
            crew: self.crew,
            game: self.game,
            ents: self.ents,
            rng: self.rng,
            rt,
        }
    }
}

/// Everything a decision reads and writes, borrowed for the call.
pub struct Ctx<'a> {
    pub t: &'a Tables,
    pub scene: &'a mut Scene,
    pub party: &'a Party,
    pub save: &'a mut SaveData,
    pub crew: &'a mut Crew,
    pub game: &'a Game,
    /// `g_entCtrl`'s list of the enemies spawned (`+0x14`, linked at
    /// +0x1c4), in its order.
    pub ents: &'a [usize],
    pub rng: &'a mut dyn Rng,
    pub rt: &'a mut dyn Runtime,
}

// ---------------------------------------------------------------------------
// Small rules the decisions share

/// `ccSkillCheckType(sid)` (gcmn 0x00573b90): 0 for the normal attack (1)
/// and the arts, -1 another physical skill, and for a spell 3 healing, 2 a
/// buff, -2 a debuff, 1 an attack spell ([`crate::skill::check_type_of`]).
pub fn skill_check_type(t: &Tables, sid: i32) -> i32 {
    crate::skill::check_type_of(t, sid)
}

/// `ccSpcChar::CheckAction(actType)` (gcmn 0x0059e660): whether a member
/// can start something of kind `n` now, from its skill state and the
/// action playing (`actNum`): 0 idle-ish (no skill running, action below
/// 7), 1 below 5 with no skill, 3 and 6-7 also during actions 15 and 16
/// (6-7 up to action 8), 2 anything but actions 7-9 and 15-16, 4 actions
/// 5 and 6 only, 5 action 0 only. From Mutation on 0 also holds during a
/// normal attack (skill 0 or 1) whatever its status.
pub fn check_action(ch: &Char, act_num: i16, n: i32, volume: Volume) -> bool {
    let (id, st, a) = (ch.skill_id, ch.skill_status, act_num);
    match n {
        // From Mutation on a skill's status matters only past the normal
        // attack.
        0 if volume != Volume::Inf => (id < 2 || st == 0) && a < 7,
        0 => st == 0 && a < 7,
        1 => id < 2 && st == 0 && a < 5,
        2 => !matches!(a, 7..=9 | 15 | 16),
        3 => id < 2 && (a == 15 || a == 16 || (st == 0 && a < 7)),
        4 => id < 2 && st == 0 && (a == 6 || a == 5),
        5 => a == 0,
        6 | 7 => id < 2 && (a == 15 || a == 16 || (st == 0 && a < 9)),
        _ => false,
    }
}

/// `ccSpcChar::DistanceToTarget(tt)` (gcmn 0x0059e860) and
/// `ccAI::DistanceToTarget` (0x00582f50; OUT 0x005a5eb0): the ground
/// distance between two characters' `posP` (the volume's `sqrtf`), less
/// both widths; 100000.0 for a target off the lists.
pub fn distance_to_target(volume: Volume, scene: &Scene, me: usize, tt: Option<usize>) -> u32 {
    let Some(tt) = tt.filter(|&c| scene.listed(c)) else { return F_FAR };
    let (a, b) = (&scene.chars[me], &scene.chars[tt]);
    let d = geom::plane_dist(volume, a.pos_p, b.pos_p);
    ee::sub(d, ee::add(b.base().width, a.base().width))
}

/// The `spcAIParam` row of an AI.
fn param_of(t: &Tables, a: &Ai) -> crate::param::AiParam {
    t.ai_params.get(a.param).copied().unwrap_or_default()
}

/// A living or merely down character (`condition.dead` 0 or 5).
fn up(ch: &Char) -> bool {
    let d = ch.cond[cond::DEAD];
    d == 0 || d == 5
}

/// Free to help: not held, asleep, paralysed, charmed or confused.
fn free(ch: &Char) -> bool {
    let c = &ch.cond;
    c[cond::HOLD] == 0
        && c[cond::SLEEP] == 0
        && c[cond::PARALYSIS] == 0
        && c[cond::CHARM] == 0
        && c[cond::CONFUSION] == 0
}

/// `ccSpcChar.partyFlag`, the signed 3-bit field.
fn party_flag(ch: &Char) -> i32 {
    ((ch.party_flag & 7) << 29) >> 29
}

/// `(cat << 16) | id` with the id sign-extended, as the item-list walks
/// build a code.
fn item_code(cat: i8, id: i16) -> i32 {
    (i32::from(cat) << 16) | i32::from(id)
}

/// The EE's integer division as the hardware gives it: by zero, -1 for a
/// non-negative dividend and 1 for a negative one; `INT_MIN / -1` stays
/// `INT_MIN`.
fn ee_div(a: i32, b: i32) -> i32 {
    if b == 0 { if a >= 0 { -1 } else { 1 } } else { a.wrapping_div(b) }
}

// ---------------------------------------------------------------------------
// The decisions

impl Ctx<'_> {
    fn ai(&self, me: usize) -> &Ai {
        &self.crew.ais[&me]
    }

    fn ai_mut(&mut self, me: usize) -> &mut Ai {
        self.crew.ais.get_mut(&me).expect("an AI for the member")
    }

    /// A member's `saveData.skillList` row.
    pub fn skill_list(&self, id: i16) -> [i16; SKILLS_PER_MEMBER] {
        let at = piney_data::save::by_id::skill_list(id as usize);
        std::array::from_fn(|i| self.save.i16(at + 2 * i))
    }

    /// A member's `saveData.itemList` row: (id, category, count).
    pub fn item_list(&self, id: i16) -> [(i16, i8, i8); ITEMS_PER_MEMBER] {
        let at = piney_data::save::by_id::item_list(id as usize);
        std::array::from_fn(|i| {
            let b = at + 4 * i;
            (self.save.i16(b), self.save.u8(b + 2) as i8, self.save.u8(b + 3) as i8)
        })
    }

    fn body_id(&self, me: usize) -> i16 {
        self.scene.chars[me].id()
    }

    /// `ccSkillCostCheck(ch, sid)`: SP for the skill.
    fn cost_ok(&self, ch: usize, sid: i32) -> bool {
        let cost = self.t.skill(sid).map_or(0, |s| s.cost);
        i32::from(self.scene.chars[ch].sp) >= cost
    }

    /// The skill an item casts (`ccGetItemParam(code)->skillID`).
    fn item_skill(&self, code: i32) -> i32 {
        self.t.item(code).map_or(0, |p| p.skill_id)
    }

    /// `ccSkillDamageValue(cp, tp, sid)` (gcmn 0x00594e90) between two
    /// characters of the scene ([`skill_damage_value`]).
    fn value(&self, cp: usize, tp: usize, sid: i32) -> i32 {
        let (a, b) = (self.scene.listed(cp), self.scene.listed(tp));
        skill_damage_value(self.t, &self.scene.chars[cp], &self.scene.chars[tp], sid, a, b, &Default::default()).dmg
    }

    fn act_num(&self, ch: usize) -> i16 {
        self.crew.spc.get(&ch).map_or(0, |s| s.act_num)
    }

    /// `ch->CheckAction(n)` (the `ccSpcChar` virtual).
    pub(crate) fn can_act(&self, ch: usize, n: i32) -> bool {
        check_action(&self.scene.chars[ch], self.act_num(ch), n, self.t.volume)
    }

    /// `ccChar.targetChar = 0` when the member may act (`CheckAction(0)`),
    /// as dropping a target does.
    fn drop_target_char(&mut self, ch: usize) {
        if self.can_act(ch, 0) {
            self.crew.spc.entry(ch).or_default().target_char = None;
        }
    }

    fn call(&mut self, c: Call) -> i32 {
        let p = Parts {
            t: self.t,
            scene: &mut *self.scene,
            party: self.party,
            save: &mut *self.save,
            crew: &mut *self.crew,
            game: self.game,
            ents: self.ents,
            rng: &mut *self.rng,
        };
        self.rt.call_ctx(c, p)
    }

    fn chat(&mut self, me: usize, msg: Chat) {
        self.call(Call::Chat { me, msg });
    }

    /// The first time a member acts on its own, its AI's `firstTime` clears.
    fn done_first(&mut self, me: usize) {
        self.ai_mut(me).first_time = false;
    }

    /// `ccAI::CheckSkillList(sid)` (gcmn 0x00587a00): the skill is in the
    /// member's list.
    pub fn check_skill_list(&self, me: usize, sid: i32) -> bool {
        self.skill_list(self.body_id(me)).iter().any(|&s| i32::from(s) == sid)
    }

    /// `ccAI::CheckSkillList2(stype, ttype)` (gcmn 0x00587a70): the member's
    /// skills of type `stype` ([`skill_check_type`]) whose target type
    /// meets `ttype`.
    pub fn check_skill_list2(&self, me: usize, stype: i32, ttype: i32) -> i32 {
        let mut n = 0;
        for s in self.skill_list(self.body_id(me)) {
            if s != -1
                && skill_check_type(self.t, i32::from(s)) == stype
                && self.t.skill(i32::from(s)).map_or(0, |p| p.target_type) & ttype != 0
            {
                n += 1;
            }
        }
        n
    }

    /// `ccAI::CheckBuffSkillListNum` (gcmn 0x00587b60): the buffs in the
    /// list (type 2), not counting the cures 178-180.
    pub fn check_buff_skill_list_num(&self, me: usize) -> i32 {
        let l = self.skill_list(self.body_id(me));
        l.iter()
            .filter(|&&s| s != -1 && !(178..=180).contains(&s) && skill_check_type(self.t, i32::from(s)) == 2)
            .count() as i32
    }

    /// `ccAI::CheckDebuffSkillListNum` (gcmn 0x00587c20): the debuffs in the
    /// list (type -2).
    pub fn check_debuff_skill_list_num(&self, me: usize) -> i32 {
        let l = self.skill_list(self.body_id(me));
        l.iter().filter(|&&s| s != -1 && skill_check_type(self.t, i32::from(s)) == -2).count() as i32
    }

    /// `ccAI::CheckHealHpSkill` (gcmn 0x00587cd0): the most HP one of the
    /// member's affordable healing skills restores (150 for Repth 150 and
    /// 153, 400 for 151 and 154, 9999 for 152 and 155), 0 with none.
    pub fn check_heal_hp_skill(&self, me: usize) -> i32 {
        let mut best = 0;
        for s in self.skill_list(self.body_id(me)) {
            let s = i32::from(s);
            if s == -1 || !(150..=155).contains(&s) || !self.cost_ok(me, s) {
                continue;
            }
            let v = match s - 150 {
                0 | 3 => 150,
                1 | 4 => 400,
                _ => 9999,
            };
            if best < v {
                best = v;
            }
        }
        best
    }

    /// `ccAI::CheckHealParty(flag)` (gcmn 0x00587e20): what the party needs (a
    /// bit per kind: 1 down, 2 hurt, 4 a body condition, 8 a spirit condition)
    /// and whether the member's skills or items cover it. Returns -2 nothing
    /// needed, 2 all covered, 1 some, -1 none but a skill lacked only the SP, 0
    /// otherwise. From Mutation on `flag` is the rate for
    /// [`Ctx::check_need_healing`] (docs/engine/battle.md, "The decisions").
    pub fn check_heal_party(&mut self, me: usize, flag: i32) -> i32 {
        let mut need = 0;
        for n in 0..3 {
            let Some(c) = self.party.members[n] else { continue };
            if self.scene.chars[c].cond[cond::DEAD] != 0 {
                need |= 1;
            }
            if self.scene.chars[c].cond[cond::DEAD] == 0 && self.check_need_healing(me, Some(c), flag) {
                need |= 2;
            }
            if self.scene.chars[c].cond[cond::DEAD] == 0 && self.check_condition_minus(me, Some(c), -1) != 0 {
                need |= 4;
            }
            if self.scene.chars[c].cond[cond::DEAD] == 0 && self.check_condition_minus(me, Some(c), -2) != 0 {
                need |= 8;
            }
        }
        if need == 0 {
            return -2;
        }
        let first = need;
        let mut short = 0;
        for s in self.skill_list(self.body_id(me)) {
            let s = i32::from(s);
            let bit = match s {
                -1 => continue,
                180 => 1,
                150..=155 => 2,
                178 => 4,
                179 => 8,
                _ => continue,
            };
            if self.cost_ok(me, s) {
                need &= !bit;
            } else {
                short |= bit;
            }
        }
        for (id, cat, _) in self.item_list(self.body_id(me)) {
            if !(10..16).contains(&cat) || id == -1 {
                continue;
            }
            let code = item_code(cat, id);
            let sk = self.item_skill(code);
            if sk == 180 {
                need &= !1;
            }
            let heal_cat = if self.t.volume == Volume::Inf { 0 } else { 10 };
            if cat == heal_cat && (id == 21 || id == 22) {
                need &= !2;
            }
            if self.t.skill(sk).map_or(0, |p| p.ty) & 0x40000 != 0 {
                need &= !2;
            }
            if sk == 178 {
                need &= !4;
            }
            if sk == 179 {
                need &= !8;
            }
        }
        if need == 0 {
            2
        } else if need != first {
            1
        } else if short != 0 {
            -1
        } else {
            0
        }
    }

    /// `ccAI::SearchHealSkill(tp)` (gcmn 0x005882a0): the healing skill for
    /// `tp`'s missing HP: of the affordable ones, the smallest that heals
    /// at least the whole loss, else the largest (150 or 400 HP, or the
    /// target's maxHP for 152 and 155); 0 with none. Skills are taken in
    /// list order, a later one of equal value winning the first choice and
    /// not the second.
    pub fn search_heal_skill(&self, me: usize, tp: usize) -> i32 {
        self.search_heal_skill_of(me, Some(tp))
    }

    /// [`Ctx::search_heal_skill`] for a target that may be missing (the
    /// game then reads zeros).
    fn search_heal_skill_of(&self, me: usize, tp: Option<usize>) -> i32 {
        let (hp, max_hp) = tp.map_or((0, 0), |c| (self.scene.chars[c].hp, self.scene.chars[c].max_hp));
        let lost = i32::from(max_hp) - i32::from(hp);
        let (mut small, mut big) = (9999, 0);
        let (mut small_sid, mut big_sid) = (-1, -1);
        for s in self.skill_list(self.body_id(me)) {
            let s = i32::from(s);
            if s == -1 {
                continue;
            }
            let v = match s - 150 {
                0 | 3 => 150,
                1 | 4 => 400,
                2 | 5 => i32::from(max_hp),
                _ => 0,
            };
            if v == 0 || !self.cost_ok(me, s) {
                continue;
            }
            if v >= lost {
                if v <= small {
                    small_sid = s;
                    small = v;
                }
            } else if big < v {
                big_sid = s;
                big = v;
            }
        }
        if small_sid != -1 {
            small_sid
        } else if big_sid != -1 {
            big_sid
        } else {
            0
        }
    }

    /// `ccAI::CheckUsableItem(code)` (gcmn 0x00588480): categories 10-15.
    pub fn check_usable_item(code: i32) -> bool {
        (10..16).contains(&(code >> 16))
    }

    /// `ccAI::CheckItemList(code)` (gcmn 0x005884d0): how many of the item
    /// the member carries (the first stack of it; the id compared
    /// unsigned).
    pub fn check_item_list(&self, me: usize, code: i32) -> i32 {
        let (cat, id) = (code >> 16, code & 0xffff);
        for (i, c, n) in self.item_list(self.body_id(me)) {
            if i32::from(c) == cat && i32::from(i) == id {
                return i32::from(n);
            }
        }
        0
    }

    /// `ccAI::CheckItemList2(itype)` (gcmn 0x00588560): the stacks (counts
    /// not consulted) of usable items of a kind: 0 Rip Maen (180), 2
    /// healing, 4 Rip Teyn (178), 5 Rip Synk (179), 6 buff, 7 debuff, 8
    /// attack (atk above 0 aimed at foes). Kinds 1 and 3 ask for category
    /// 0 among categories 10-15 and never count.
    pub fn check_item_list2(&self, me: usize, itype: i32) -> i32 {
        let mut n = 0;
        for (id, cat, _) in self.item_list(self.body_id(me)) {
            if !(10..16).contains(&cat) || id == -1 {
                continue;
            }
            let sk = self.item_skill(item_code(cat, id));
            let p = if sk == -1 { None } else { self.t.skill(sk) };
            let hit = match itype {
                0 => sk == 180,
                1 => cat == 0 && (id == 21 || id == 22),
                2 => p.is_some_and(|p| p.ty & 0x40000 != 0),
                3 => cat == 0 && matches!(id, 18..=22),
                4 => sk == 178,
                5 => sk == 179,
                6 => p.is_some_and(|p| p.ty & 0x10000 != 0),
                7 => p.is_some_and(|p| p.ty & 0x20000 != 0),
                8 => p.is_some_and(|p| p.atk > 0 && p.target_type & 0xe0 != 0),
                _ => false,
            };
            if hit {
                n += 1;
            }
        }
        n
    }

    /// `ccAI::ConsumeItemList(code)` (gcmn 0x00588860): one of the item is
    /// used up; an empty stack becomes (-1, -1, 0). Returns the count
    /// left, -1 without the item.
    pub fn consume_item_list(&mut self, me: usize, code: i32) -> i32 {
        let (cat, id) = (code >> 16, code & 0xffff);
        let at = piney_data::save::by_id::item_list(self.body_id(me) as usize);
        for i in 0..ITEMS_PER_MEMBER {
            let b = at + 4 * i;
            if i32::from(self.save.u8(b + 2) as i8) != cat || i32::from(self.save.i16(b)) != id {
                continue;
            }
            let n = (self.save.u8(b + 3) as i8).wrapping_sub(1);
            self.save.set_u8(b + 3, n as u8);
            if n <= 0 {
                self.save.set_i16(b, -1);
                self.save.set_u8(b + 2, 0xff);
                self.save.set_u8(b + 3, 0);
            }
            return i32::from(self.save.u8(b + 3) as i8);
        }
        -1
    }

    /// The usable stacks of the member's list, in order, as the searches
    /// walk them: categories 10-15, an id, a count.
    fn usable_items(&self, id: i16) -> Vec<i32> {
        self.item_list(id)
            .iter()
            .filter(|&&(i, c, n)| (10..16).contains(&c) && i != -1 && n != 0)
            .map(|&(i, c, _)| item_code(c, i))
            .collect()
    }

    /// `ccAI::SearchItemListBySkill(sid)` (gcmn 0x00588920): the first item
    /// the member has that casts `sid`, or -1.
    pub fn search_item_list_by_skill(&self, me: usize, sid: i32) -> i32 {
        self.usable_items(self.body_id(me)).into_iter().find(|&c| self.item_skill(c) == sid).unwrap_or(-1)
    }

    /// `ccAI::SearchItemListByHeal(hp)` (gcmn 0x00588a50): the healing item
    /// for a loss of `hp`: the first whose band it falls in (Repth items
    /// 150/153 under 151, 151/154 151-400, 295 401-800, 152/155 and the
    /// category-10 recovery items 21 and 22 above 800), else the last
    /// healing item seen; -1 with none.
    pub fn search_item_list_by_heal(&self, me: usize, hp: i32) -> i32 {
        let mut last = -1;
        for (id, cat, n) in self.item_list(self.body_id(me)) {
            if !(10..16).contains(&cat) || id == -1 || n == 0 {
                continue;
            }
            let code = item_code(cat, id);
            let sk = self.item_skill(code);
            if self.t.skill(sk).map_or(0, |p| p.ty) & 0x40000 != 0 {
                let fits = match sk {
                    150 | 153 => hp < 151,
                    151 | 154 => (151..401).contains(&hp),
                    152 | 155 => hp >= 801,
                    295 => (401..801).contains(&hp),
                    _ => continue,
                };
                if fits {
                    return code;
                }
                last = code;
            } else if cat == 10 && (id == 21 || id == 22) {
                if hp >= 801 {
                    return code;
                }
                last = code;
            }
        }
        last
    }

    /// `ccAI::SearchItemListByBuff` (gcmn 0x00588d30): the member has a buff
    /// item (not one of the cures 178-180).
    pub fn search_item_list_by_buff(&self, me: usize) -> bool {
        self.usable_items(self.body_id(me)).into_iter().any(|c| {
            let sk = self.item_skill(c);
            self.t.skill(sk).map_or(0, |p| p.ty) & 0x10000 != 0 && !(178..=180).contains(&sk)
        })
    }

    /// `ccAI::SearchItemListByDebuff` (gcmn 0x00588e90): a debuff item.
    pub fn search_item_list_by_debuff(&self, me: usize) -> bool {
        self.usable_items(self.body_id(me))
            .into_iter()
            .any(|c| self.t.skill(self.item_skill(c)).map_or(0, |p| p.ty) & 0x20000 != 0)
    }

    /// `ccAI::SearchItemListByMagicAttack` (gcmn 0x00588fc0): an attack
    /// spell item (`ccSkillCheckType(ccSkillParam *)` 1).
    pub fn search_item_list_by_magic_attack(&self, me: usize) -> bool {
        self.usable_items(self.body_id(me)).into_iter().any(|c| skill_check_type(self.t, self.item_skill(c)) == 1)
    }

    /// `ccAI::CheckNeedHealing(tp, flag)` (gcmn 0x00589820): `tp` (the
    /// member itself for none), alive or down, is below its heal line: its
    /// maxHP out of battle, with `flag`, or for a strategy-3 healer looking
    /// at itself; else `maxHP * healRate / 100` of the `spcAIParam` row of
    /// the member's character id.
    ///
    /// From Mutation on the second argument is a rate (float bits, `f12`)
    /// and the line `maxHP * rate / 100` whatever else holds.
    pub fn check_need_healing(&self, me: usize, tp: Option<usize>, flag: i32) -> bool {
        let a = self.ai(me);
        let tp = tp.unwrap_or(a.body);
        let t = &self.scene.chars[tp];
        if !up(t) {
            return false;
        }
        if self.t.volume != Volume::Inf {
            let line = ee::div(ee::mul(ee::from_int(i32::from(t.max_hp)), flag as u32), F_100);
            return i32::from(t.hp) < crate::damage::fptosi(line);
        }
        let line = if flag != 0 || self.game.in_battle != 1 || (a.strategy == 3 && a.body == tp) {
            i32::from(t.max_hp)
        } else {
            // The row of the member's character id, not the AI's param.
            let id = self.body_id(me) as usize;
            let rate = i32::from(self.t.ai_params.get(id).copied().unwrap_or_default().heal_rate);
            cdiv(i32::from(t.max_hp).wrapping_mul(rate), 100)
        };
        i32::from(t.hp) < line
    }

    /// `ccAI::CheckConditionMinus(p, n)` (gcmn 0x00589910): the cure skill
    /// id of a bad condition `p` (the member for none) has, 0 for none or
    /// for a character that is neither a foe nor of type 4. `n` -1 looks
    /// at what Rip Teyn cures (lowered elements 169-174, pAtk pDef pHit
    /// 163-165, poison 156, slow 158, paralysis 157), -2 at Rip Synk's
    /// (lowered magic 166-168, curse 162, sleep 160, confusion 161, charm
    /// 159), -3 at both; the last found in that order wins. Any other `n`
    /// (156-174) asks after that one condition.
    pub fn check_condition_minus(&self, me: usize, p: Option<usize>, n: i32) -> i32 {
        let p = p.unwrap_or(self.ai(me).body);
        let ch = &self.scene.chars[p];
        if !up(ch) {
            return 0;
        }
        let tyb = ch.ty();
        let temp = if tyb & 0x60 != 0 || tyb & 0x80 != 0 || tyb & 4 != 0 {
            match ch.temp_time() {
                Some((temp, _)) => *temp,
                None => return 0,
            }
        } else {
            return 0;
        };
        let c = &ch.cond;
        let low = |i: usize| temp[i] < 0;
        let slow = ee::lt(c.speed_value, F_ONE);
        let mut v = 0;
        let mut test = |on: bool, sid: i32| {
            if on {
                v = sid;
            }
        };
        match n {
            -1 => {
                for i in 0..6 {
                    test(low(8 + i), 169 + i as i32);
                }
                test(low(0), 163);
                test(low(1), 164);
                test(low(2), 165);
                test(c[cond::POISON] != 0, 156);
                test(slow, 158);
                test(c[cond::PARALYSIS] != 0, 157);
            }
            -2 => {
                test(low(4), 166);
                test(low(5), 167);
                test(low(6), 168);
                test(c[cond::CURSE] != 0, 162);
                test(c[cond::SLEEP] != 0, 160);
                test(c[cond::CONFUSION] != 0, 161);
                test(c[cond::CHARM] != 0, 159);
            }
            -3 => {
                for i in 0..6 {
                    test(low(8 + i), 169 + i as i32);
                }
                test(low(0), 163);
                test(low(1), 164);
                test(low(2), 165);
                test(low(4), 166);
                test(low(5), 167);
                test(low(6), 168);
                test(slow, 158);
                test(c[cond::CURSE] != 0, 162);
                test(c[cond::PARALYSIS] != 0, 157);
                test(c[cond::SLEEP] != 0, 160);
                test(c[cond::POISON] != 0, 156);
                test(c[cond::CONFUSION] != 0, 161);
                test(c[cond::CHARM] != 0, 159);
            }
            156 => test(c[cond::POISON] != 0, 156),
            157 => test(c[cond::PARALYSIS] != 0, 157),
            158 => test(slow, 158),
            159 => test(c[cond::CHARM] != 0, 159),
            160 => test(c[cond::SLEEP] != 0, 160),
            161 => test(c[cond::CONFUSION] != 0, 161),
            162 => test(c[cond::CURSE] != 0, 162),
            163..=165 => test(low((n - 163) as usize), n),
            166..=168 => test(low((n - 166 + 4) as usize), n),
            169..=174 => test(low((n - 169 + 8) as usize), n),
            _ => {}
        }
        v
    }

    /// `ccAI::CheckBossEntry(n)` (gcmn 0x00595940): the first foe on the
    /// list that makes a boss fight: a boss, or (for `n` 0) an enemy of
    /// type 0x40 with no entry behind it (`entRoot` 0).
    pub fn check_boss_entry(&self, n: i32) -> Option<usize> {
        let mask = if n != 0 { 0x80 } else { 0xc0 };
        self.scene.ene_list.iter().copied().find(|&c| {
            let ch = &self.scene.chars[c];
            let tyb = ch.ty();
            tyb & mask != 0 && (tyb & 0x80 != 0 || ch.ent_root == 0)
        })
    }

    /// `ccAI::CheckHealingSchedule` (gcmn 0x00595bf0): a heal, cure or
    /// revive this member is due to carry out (a waiting message of kind 2,
    /// 8, 3, 9, 4, 10, 5 or 11 addressed to it, tested in that order).
    /// Returns the kind in the high half, 0 for none.
    pub fn check_healing_schedule(&self, me: usize) -> i32 {
        let id = self.ai(me).sys_msg.id as u16;
        for k in [2, 8, 3, 9, 4, 10, 5, 11] {
            if self.crew.sys.pending_for(k, id) {
                return k << 16;
            }
        }
        0
    }

    /// `ccAI::CheckSchedule` (gcmn 0x00596040): [`Ctx::check_healing_schedule`],
    /// then a buff or debuff due (kinds 6, 7).
    pub fn check_schedule(&self, me: usize) -> i32 {
        let v = self.check_healing_schedule(me);
        if v != 0 {
            return v;
        }
        let id = self.ai(me).sys_msg.id as u16;
        for k in [6, 7] {
            if self.crew.sys.pending_for(k, id) {
                return k << 16;
            }
        }
        0
    }

    /// `ccAI::CheckSolution(kind, tp)` (gcmn 0x005959c0): someone is already
    /// seeing to `tp` for this need: a message of the skill or item form of
    /// the kind (2/8 heal, 3/9 and 4/10 cure, 5/11 revive) naming `tp`
    /// waits, or was delivered within 90 frames (skill form) or 180 (item
    /// form). From Mutation on the item form's window is 20 frames, and a
    /// revive is looked for in any form ([`AiSystem::search_revive`]).
    pub fn check_solution(&self, kind: i32, tp: Option<usize>) -> bool {
        let tid = tp.map_or(0, |c| i32::from(self.crew.sys_msg_id(c)));
        let later = self.t.volume != Volume::Inf;
        let (a, b) = match (kind as u32 & 0xffff_0000) >> 16 {
            0xb | 0x5 if later => {
                return self.crew.sys.search_revive(tid as u16, 0xffff, 90, |c| self.item_skill(c));
            }
            0xb | 0x5 => (0x5, 0xb),
            0xa | 0x4 => (0x4, 0xa),
            0x9 | 0x3 => (0x3, 0x9),
            0x8 | 0x2 => (0x2, 0x8),
            _ => return false,
        };
        let (a, b) = (tid | a << 16, tid | b << 16);
        let s = &self.crew.sys;
        let item_window = if later { 20 } else { 180 };
        s.search_history(a, 90).is_some()
            || s.pending(|m| m.name == a)
            || s.search_history(b, item_window).is_some()
            || s.pending(|m| m.name == b)
    }

    /// `ccAI::SysMsgFromMeToMe(kind, tp, delay, param, pp)` (gcmn
    /// 0x005961a0): a note to itself, `kind | tp's id`, due in `delay`
    /// frames.
    pub fn sys_msg_from_me_to_me(
        &mut self,
        me: usize,
        kind: i32,
        tp: usize,
        delay: u16,
        param: i32,
        pp: Option<usize>,
    ) -> i32 {
        let name = kind | i32::from(self.crew.sys_msg_id(tp));
        let id = self.crew.sys_msg_id(me) as u16;
        self.crew.send(name, id, id, 0, delay, param, pp)
    }

    /// `ccAI::CheckSkillOtherSPC(sid)` (gcmn 0x00595400): with a full party,
    /// the other AI-driven member (slots 1 and 2) better placed to use
    /// `sid`: up, free, allowed that kind of skill (its `skillMask`), with
    /// nothing of its own scheduled, and with the skill and the SP, or an
    /// item that casts it (left in `checkSkillOtherSPCFlag`, -1 for the
    /// skill).
    pub fn check_skill_other_spc(&mut self, me: usize, sid: i32) -> Option<usize> {
        self.crew.check_skill_other_flag = -1;
        if self.party.num < 3 {
            return None;
        }
        let mine = self.party.slot_of(i32::from(self.body_id(me)));
        for n in 1..3 {
            if n == mine {
                continue;
            }
            let Some(c) = self.party.members[n as usize] else { continue };
            let ch = &self.scene.chars[c];
            if !up(ch) || !free(ch) {
                continue;
            }
            let Some(mask) = self.crew.ais.get(&c).map(|a| a.skill_mask) else { continue };
            if mask == 0 {
                continue;
            }
            let bit = match skill_check_type(self.t, sid) {
                -2 => 8,
                3 => 4,
                2 => 16,
                1 => 2,
                0 => 1,
                _ => continue,
            };
            if mask & bit == 0 || self.check_healing_schedule(c) != 0 {
                continue;
            }
            let cid = self.scene.chars[c].id();
            if self.skill_list(cid).iter().any(|&s| i32::from(s) == sid) && self.cost_ok(c, sid) {
                return Some(c);
            }
            let item = self.usable_items(cid).into_iter().find(|&code| self.item_skill(code) == sid).unwrap_or(-1);
            self.crew.check_skill_other_flag = item;
            if item != -1 {
                return Some(c);
            }
        }
        None
    }

    /// `ccAI::CheckHealSkillOtherSPC(hp)` (gcmn 0x005957b0): the same for
    /// healing `hp`: another member (slots 1 and 2) alive, free, allowed to
    /// heal, with nothing scheduled, whose best healing skill restores more
    /// than `hp`, or with a healing item for it (in
    /// `checkHealSkillOtherSPCFlag`).
    pub fn check_heal_skill_other_spc(&mut self, me: usize, hp: i32) -> Option<usize> {
        self.crew.check_heal_skill_other_flag = -1;
        if self.party.num < 3 {
            return None;
        }
        let mine = self.party.slot_of(i32::from(self.body_id(me)));
        for n in 1..3 {
            if n == mine {
                continue;
            }
            let Some(c) = self.party.members[n as usize] else { continue };
            let ch = &self.scene.chars[c];
            if ch.cond[cond::DEAD] != 0 || !free(ch) {
                continue;
            }
            if self.crew.ais.get(&c).is_none_or(|a| a.skill_mask & 4 == 0) {
                continue;
            }
            if self.check_healing_schedule(c) != 0 {
                continue;
            }
            if hp < self.check_heal_hp_skill(c) {
                return Some(c);
            }
            let item = self.search_item_list_by_heal(c, hp);
            self.crew.check_heal_skill_other_flag = item;
            if item != -1 {
                return Some(c);
            }
        }
        None
    }
}

impl Ctx<'_> {
    /// The body's eye point (`pos` raised by 0.8 of its height) and the
    /// line of sight from it to `c`'s eye at the same height, in a dungeon
    /// (`game.area` 2) only: `ccHitCheckLM(from, to, 8)` must be -1.0.
    fn in_sight(&mut self, me: usize, c: usize) -> bool {
        if self.game.area != 2 {
            return true;
        }
        let rise = ee::mul(F_EYE, self.scene.chars[me].base().height);
        let mut from = self.scene.chars[me].pos;
        from[2] = ee::add(from[2], rise);
        let mut to = self.scene.chars[c].pos;
        to[2] = ee::add(to[2], rise);
        let v = self.call(Call::HitCheckLm { from, to, mask: 8 }) as u32;
        ee::cmp(F_MINUS_ONE, v) == std::cmp::Ordering::Equal
    }

    /// The target types a charmed member (sides swapped) or a confused one
    /// (everyone, 230) looks for.
    fn swayed(&self, me: usize, mut tt: i32) -> i32 {
        let c = &self.scene.chars[me].cond;
        if c[cond::CHARM] != 0 {
            if tt & 6 != 0 {
                tt = 224;
            } else if tt & 0xe0 != 0 {
                tt = 6;
            }
        }
        if c[cond::CONFUSION] != 0 {
            tt = 230;
        }
        tt
    }

    /// Keep `c` at `d` if it is the nearest so far: when no distance is
    /// kept yet (below 0) or `d` is not beyond it.
    fn nearer(best: &mut u32, d: u32) -> bool {
        if ee::lt(*best, d) && !ee::lt(*best, 0) {
            return false;
        }
        *best = d;
        true
    }

    /// `ccAI::SearchTarget(tt, force)` (gcmn 0x00586a50): whom to fight. A
    /// member in the party (`partyFlag` 1) under strategy 1 takes Kite's
    /// target when it is listed, alive and not on its own side; otherwise,
    /// and for any other strategy or member, [`Ctx::search_target_near`].
    pub fn search_target(&mut self, me: usize, tt: i32, force: i32) -> Option<usize> {
        let body = self.ai(me).body;
        if party_flag(&self.scene.chars[body]) != 1 || self.ai(me).strategy != 1 {
            return self.search_target_near(me, tt, force);
        }
        let kite = self.party.members[0].and_then(|k| self.crew.spc.get(&k)).and_then(|s| s.target_char);
        // From Mutation on only a charmed or confused member takes Kite's
        // target, and only one on its own side (not itself); a foe of his
        // is left for the search.
        let bc = self.scene.chars[body].cond;
        let later = self.t.volume != Volume::Inf;
        let swayed = bc[cond::CHARM] != 0 || bc[cond::CONFUSION] != 0;
        let t = kite.filter(|&c| {
            let same = self.scene.chars[c].ty() & self.scene.chars[body].ty() != 0;
            let take = if later { same && swayed && c != body } else { !same };
            self.scene.listed(c) && self.scene.chars[c].cond[cond::DEAD] == 0 && take
        });
        match t {
            Some(c) => Some(c),
            None => self.search_target_near(me, tt, force),
        }
    }

    /// `ccAI::SearchTargetNear(tt, force)` (gcmn 0x00586bd0): the nearest
    /// character of the types `tt` (swapped when charmed, all when
    /// confused) on the party's list (in the party, not down past 1, not
    /// the member) or the foes' list (not down past 1), in sight in a
    /// dungeon, and unless `force` within the member's `cautionRange`. With
    /// a boss fight on ([`Ctx::check_boss_entry`]) every foe passing the
    /// sight test is taken in turn, so the last one wins.
    pub fn search_target_near(&mut self, me: usize, tt: i32, force: i32) -> Option<usize> {
        let body = self.ai(me).body;
        let caution = param_of(self.t, self.ai(me)).caution_range;
        let tt = self.swayed(body, tt);
        let later = self.t.volume != Volume::Inf;
        let bc = self.scene.chars[body].cond;
        // From Mutation on a charmed or confused member looks as far as it
        // likes on the party's side.
        let far = later && (bc[cond::CHARM] != 0 || bc[cond::CONFUSION] != 0);
        let mut best = F_MINUS_ONE;
        let mut found = None;
        if tt & 6 != 0 {
            for c in self.scene.pc_list.clone() {
                let ch = &self.scene.chars[c];
                if tt & ch.ty() == 0 || c == body || ch.cond[cond::DEAD] >= 2 || party_flag(ch) == 0 {
                    continue;
                }
                if !self.in_sight(body, c) {
                    continue;
                }
                let d = geom::plane_dist(self.t.volume, self.scene.chars[c].pos_p, self.scene.chars[body].pos_p);
                if force == 0 && !far && !ee::lt(d, caution) {
                    continue;
                }
                if Self::nearer(&mut best, d) {
                    found = Some(c);
                }
            }
        }
        // From Mutation on a member holding its place under command 14
        // looks as far as 2000; in a boss fight any distance does, the
        // nearest still taken.
        let a = self.ai(me);
        let hold14 = a.chat_cmd_flag == 2 && a.chat_cmd == 14 && a.strategy == 6;
        let range = if later && hold14 { F_2000 } else { caution };
        let boss = later && self.check_boss_entry(0).is_some();
        if tt & 0xe0 != 0 {
            for c in self.scene.ene_list.clone() {
                let ch = &self.scene.chars[c];
                if tt & ch.ty() == 0 || c == body || ch.cond[cond::DEAD] >= 2 {
                    continue;
                }
                if !self.in_sight(body, c) {
                    continue;
                }
                let d = geom::plane_dist(self.t.volume, self.scene.chars[c].pos_p, self.scene.chars[body].pos_p);
                if !later && self.check_boss_entry(0).is_some() {
                    found = Some(c);
                    best = d;
                    continue;
                }
                if !boss && force == 0 && !ee::lt(d, range) {
                    continue;
                }
                if Self::nearer(&mut best, d) {
                    found = Some(c);
                }
            }
        }
        found
    }

    /// `ccAI::CountTargetInArea(tt, range)` (gcmn 0x00587060): the
    /// characters of the types `tt` (swayed as for the search) within
    /// `range` of the member: party members alive on the party's list, and
    /// the enemies spawned (`g_entCtrl`'s list, their `pos` taken to the
    /// player's frame) not down past 1; in sight in a dungeon.
    pub fn count_target_in_area(&mut self, me: usize, tt: i32, range: u32) -> i32 {
        let body = self.ai(me).body;
        let tt = self.swayed(body, tt);
        let mut n = 0;
        if tt & 6 != 0 {
            for c in self.scene.pc_list.clone() {
                let ch = &self.scene.chars[c];
                if tt & ch.ty() == 0 || c == body || ch.cond[cond::DEAD] != 0 {
                    continue;
                }
                if !self.in_sight(body, c) {
                    continue;
                }
                let d = geom::plane_dist(self.t.volume, self.scene.chars[c].pos_p, self.scene.chars[body].pos_p);
                if ee::le(d, range) {
                    n += 1;
                }
            }
        }
        if tt & 0xe0 != 0 {
            for &c in self.ents {
                let ch = &self.scene.chars[c];
                if tt & ch.ty() == 0 || c == body || ch.cond[cond::DEAD] >= 2 {
                    continue;
                }
                if !self.in_sight(body, c) {
                    continue;
                }
                let p = self.rt.w2p(self.scene.chars[c].pos);
                let d = geom::plane_dist(self.t.volume, p, self.scene.chars[body].pos_p);
                if ee::le(d, range) {
                    n += 1;
                }
            }
        }
        n
    }

    /// `ccAI::levelCheck` (gcmn 0x00587440): a level gained or lost since
    /// the last look is announced to everyone in 30 frames (0x1000a up,
    /// 0x10009 down, pointer the member); nothing while under manual
    /// control or during action 14. Returns 1, -1 or 0; the level is noted
    /// either way.
    pub fn level_check(&mut self, me: usize) -> i32 {
        let body = self.ai(me).body;
        let mut v = 0;
        if !self.ai(me).manual_sw && self.act_num(body) != 14 {
            let level = self.scene.chars[body].level();
            let old = self.ai(me).level_old;
            let id = self.crew.sys_msg_id(body) as u16;
            if level < old {
                self.crew.send(0x10009, id, 0xffff, 0, 30, -1, Some(body));
                v = -1;
            } else if old < level {
                self.crew.send(0x1000a, id, 0xffff, 0, 30, -1, Some(body));
                v = 1;
            }
        }
        self.ai_mut(me).level_old = self.scene.chars[body].level();
        v
    }

    /// The nearest foe (`ene` true, not down past 1) or party member
    /// (alive) in sight and within `cautionRange` that does not yet have
    /// what `sid` gives ([`target_condition_by_skill`] 0).
    ///
    /// From Mutation on the members are not checked for sight, and each
    /// nearer candidate is pushed in front of [`Crew::unused`].
    fn search_unused(&mut self, me: usize, sid: i32, ene: bool) -> Option<usize> {
        let body = self.ai(me).body;
        let caution = param_of(self.t, self.ai(me)).caution_range;
        let later = self.t.volume != Volume::Inf;
        if later {
            self.crew.set_unused(0, None);
            self.crew.set_unused(if ene { 32 } else { 5 }, None);
        }
        let mut best = F_MINUS_ONE;
        let mut found = None;
        let list = if ene { self.scene.ene_list.clone() } else { self.scene.pc_list.clone() };
        for c in list {
            let d = self.scene.chars[c].cond[cond::DEAD];
            if (ene && d >= 2) || (!ene && d != 0) {
                continue;
            }
            if ((ene || !later) && !self.in_sight(body, c)) || target_condition_by_skill(&self.scene.chars[c], sid) != 0
            {
                continue;
            }
            let d = geom::plane_dist(self.t.volume, self.scene.chars[c].pos_p, self.scene.chars[body].pos_p);
            if !ee::lt(d, caution) {
                continue;
            }
            if Self::nearer(&mut best, d) {
                found = Some(c);
                if later {
                    for i in (1..5).rev() {
                        let v = self.crew.unused_at(i - 1);
                        self.crew.set_unused(i, v);
                    }
                    self.crew.set_unused(0, Some(c));
                }
            }
        }
        found
    }

    /// `ccAI::SearchDebuffUnusedEnemy(sid)` (gcmn 0x00587550).
    pub fn search_debuff_unused_enemy(&mut self, me: usize, sid: i32) -> Option<usize> {
        self.search_unused(me, sid, true)
    }

    /// `ccAI::SearchBuffUnusedFellow(sid)` (gcmn 0x00587790) (the member
    /// itself included).
    pub fn search_buff_unused_fellow(&mut self, me: usize, sid: i32) -> Option<usize> {
        self.search_unused(me, sid, false)
    }

    /// `ccAI::DebuffForUnusedEnemy(sid)` (gcmn 0x005937d0) and
    /// `BuffForUnusedFellow` (0x00593ce0): plan `sid` on the nearest foe (or
    /// member) without it, a 0x60000 (skill) or 0x70000 (item) message to
    /// itself due in 30 frames, unless the same plan waits or went out within
    /// 90 (skill) or 180 (item) frames. Returns 1 when planned (or with no free
    /// slot). Mutation's `check` rules are in docs/engine/battle.md.
    fn plan_for_unused(&mut self, me: usize, sid: i32, ene: bool, check: i32) -> i32 {
        let body = self.ai(me).body;
        let later = self.t.volume != Volume::Inf;
        let mut kind = 0;
        if self.check_skill_list(me, sid) {
            let sp_only = later
                && check == 2
                && self.t.skill(sid).is_some_and(|k| i32::from(self.scene.chars[body].max_sp) >= k.cost);
            if sp_only || self.cost_ok(body, sid) {
                kind = 1;
            }
        }
        let item = self.search_item_list_by_skill(me, sid);
        if item != -1 {
            kind = 2;
        }
        if later {
            return self.plan_later(me, sid, ene, check, kind, item);
        }
        if kind == 0 {
            return 0;
        }
        let Some(tgt) = self.search_unused(me, sid, ene) else { return 0 };
        let (name, param, t) = if kind == 1 { (0x60000, sid, 90) } else { (0x70000, item, 180) };
        let s = &self.crew.sys;
        if s.search_history_p(name, param, t, Some(tgt)).is_some()
            || s.pending(|m| m.name == name && m.param == param && m.pointer == Some(tgt))
        {
            return 0;
        }
        let id = self.crew.sys_msg_id(body) as u16;
        self.crew.post(name, param, Some(tgt), id, id);
        1
    }

    /// Mutation's [`Ctx::plan_for_unused`] past the choice of form (`kind`
    /// 1 the skill, 2 the item `item`, 0 neither).
    fn plan_later(&mut self, me: usize, sid: i32, ene: bool, check: i32, kind: i32, item: i32) -> i32 {
        if self.t.skill(sid).is_some_and(|k| k.ty & 0x6000 != 0) {
            let s = &self.crew.sys;
            if s.search_kind_param(0x60000, sid, 90) || (item != -1 && s.search_kind_param(0x70000, item, 90)) {
                return 0;
            }
        }
        if kind == 0 {
            return 0;
        }
        let (name, param, window) = if kind == 1 { (0x60000, sid, 90) } else { (0x70000, item, 180) };
        let mut tgt = self.search_unused(me, sid, ene);
        let mut next = 0;
        loop {
            let Some(t) = tgt else { return 0 };
            if check != 0 {
                return 1;
            }
            let s = &self.crew.sys;
            if s.search_history_p(name, param, window, Some(t)).is_some()
                || s.pending(|m| m.name == name && m.param == param && m.pointer == Some(t))
            {
                next += 1;
                tgt = self.crew.unused_at(next);
                continue;
            }
            let body = self.ai(me).body;
            let id = self.crew.sys_msg_id(body) as u16;
            let delay = if self.ai(me).first_time { 30 } else { 1 };
            self.crew.post_in(name, param, Some(t), id, id, delay);
            return 1;
        }
    }

    /// `ccAI::DebuffForUnusedEnemy(sid)` (gcmn 0x005937d0); from Mutation on
    /// `(sid, check)`.
    pub fn debuff_for_unused_enemy(&mut self, me: usize, sid: i32, check: i32) -> i32 {
        self.plan_for_unused(me, sid, true, check)
    }

    /// `ccAI::BuffForUnusedFellow(sid)` (gcmn 0x00593ce0).
    pub fn buff_for_unused_fellow(&mut self, me: usize, sid: i32, check: i32) -> i32 {
        self.plan_for_unused(me, sid, false, check)
    }

    /// `ccAI::CheckTargetConditionBySkill(tp, sid)` (gcmn 0x005937a0).
    pub fn check_target_condition_by_skill(&self, tp: usize, sid: i32) -> i32 {
        target_condition_by_skill(&self.scene.chars[tp], sid)
    }

    /// `ccAI::CheckSkillMask` (gcmn 0x00594e80).
    pub fn check_skill_mask(&self, me: usize) -> i32 {
        i32::from(self.ai(me).skill_mask)
    }

    /// `ccAI::ChangeStrategyCMD(n)` (gcmn 0x00593770): the commanded
    /// strategy and the strategy both become `n`.
    pub fn change_strategy_cmd(&mut self, me: usize, n: i16) {
        let a = self.ai_mut(me);
        a.strategy_cmd = n;
        a.strategy = n;
    }

    /// `ccAI::ChangeStrategy(n)` (gcmn 0x00593780): for a member with
    /// `SpcListNum`.
    pub fn change_strategy(&mut self, me: usize, n: i16) {
        let body = self.ai(me).body;
        if self.crew.spc.get(&body).is_some_and(|s| s.spc_list_num != 0) {
            self.ai_mut(me).strategy = n;
        }
    }

    /// `ccAI::ChangeMode(n, f)` (gcmn 0x00589740): to mode `n` (3 sets
    /// `battleFlag` 1 and clears `attackCycle`, 1 sets it -1). Unless `f`,
    /// the change is first announced to the member itself (`0xc0000 | n`,
    /// now) and a member off the bus stays as it is.
    pub fn change_mode(&mut self, me: usize, n: i32, f: i32) {
        if self.ai(me).mode == n {
            return;
        }
        if f == 0 {
            let id = self.ai(me).sys_msg.id;
            if id == -1 {
                return;
            }
            self.crew.send(n | 0xc0000, id as u16, id as u16, 0, 0, 0, None);
        }
        let a = self.ai_mut(me);
        a.mode_old = a.mode;
        a.mode = n;
        if n == 3 {
            a.battle_flag = 1;
            a.attack_cycle = 0;
        } else if n == 1 {
            a.battle_flag = -1;
        }
    }

    /// `ccAI::StopNormalAttack` (gcmn 0x005890f0): a running normal attack
    /// (skill 1) is ended (`ccSkillRequest(body, 0, 0)`). Returns the
    /// skill id it saw.
    pub fn stop_normal_attack(&mut self, me: usize) -> i32 {
        let body = self.ai(me).body;
        let s = i32::from(self.scene.chars[body].skill_id);
        if s == 1 {
            self.call(Call::SkillRequest { me: body, target: None, sid: 0 });
        }
        s
    }

    /// Giving up on the target: a command in hand is dropped (a strategy-5
    /// order reset to itself), `targetFlag` and the target cleared, and
    /// the body's `targetChar` if it may act.
    fn give_up(&mut self, me: usize, strategy_reset: bool) {
        let body = self.ai(me).body;
        if self.ai(me).chat_cmd != -1 {
            self.ai_mut(me).drop_chat_cmd();
            if strategy_reset && self.ai(me).strategy_cmd == 5 {
                self.change_strategy_cmd(me, 5);
            }
        }
        let a = self.ai_mut(me);
        a.target_flag = 0;
        a.target = None;
        self.drop_target_char(body);
    }

    /// `ccAI::UseSkill(sid, target)` (gcmn 0x00589140): the member uses a
    /// skill on `target` (listed, alive, or down for Rip Maen): only a
    /// party member of type 4 does, through [`Ctx::fellow_use_skill`];
    /// out of range it walks there (`FollowTarget`) and a command to use
    /// that skill (chat command 5) is done once it starts. With no usable
    /// target the command is dropped and the target let go. Returns 1
    /// started, -1 not able to act (or out of range or SP), 0 otherwise.
    pub fn use_skill(&mut self, me: usize, sid: i32, target: Option<usize>) -> i32 {
        let body = self.ai(me).body;
        let later = self.t.volume != Volume::Inf;
        let c = self.scene.chars[body].cond;
        // From Mutation on a member asleep, paralysed, charmed or confused
        // cannot, nor one held with a boss in; and it must be free to act
        // as for a skill (6).
        if later && (c[cond::SLEEP] != 0 || c[cond::PARALYSIS] != 0 || c[cond::CHARM] != 0 || c[cond::CONFUSION] != 0) {
            return -1;
        }
        if !(0..304).contains(&sid) {
            return 0;
        }
        if !self.can_act(body, if later { 6 } else { 3 }) {
            return -1;
        }
        if later && c[cond::HOLD] != 0 && self.check_boss_entry(1).is_some() {
            return -1;
        }
        if let Some(t) = target.filter(|&t| self.scene.listed(t)) {
            let d = self.scene.chars[t].cond[cond::DEAD];
            if d == 0 || (sid == 180 && d != 5) {
                let tyb = self.scene.chars[body].ty();
                if tyb & 1 != 0 || tyb & 4 == 0 {
                    return 0;
                }
                let r = self.fellow_use_skill(body, sid, t);
                if r == -1 {
                    self.call(Call::FollowTarget { me: body, target: Some(t) });
                } else if r == 1 && self.ai(me).chat_cmd == 5 {
                    self.ai_mut(me).drop_chat_cmd();
                }
                return r;
            }
        }
        self.give_up(me, true);
        0
    }

    /// `ccFellow::UseSkill(sid, tp)` (gcmn 0x0041e470): not asleep,
    /// paralysed, charmed or confused and able to act (`CheckAction(6)`),
    /// the member faces `tp` (`targetChar`), measures the distance
    /// (`distTg`) and starts the skill if it has the SP and `tp` is within
    /// the skill's `triggerRange`. Returns 1 started, -1 short of SP or
    /// range, 0 unable.
    pub fn fellow_use_skill(&mut self, body: usize, sid: i32, tp: usize) -> i32 {
        let c = &self.scene.chars[body].cond;
        if c[cond::SLEEP] != 0 || c[cond::PARALYSIS] != 0 || c[cond::CHARM] != 0 || c[cond::CONFUSION] != 0 {
            return 0;
        }
        if !self.can_act(body, 6) {
            return 0;
        }
        let d = distance_to_target(self.t.volume, self.scene, body, Some(tp));
        let s = self.crew.spc.entry(body).or_default();
        s.target_char = Some(tp);
        s.dist_tg = d;
        if !self.cost_ok(body, sid) {
            return -1;
        }
        if !self.t.skill(sid).is_some_and(|k| crate::skill::range_check(k, d)) {
            return -1;
        }
        self.call(Call::SkillRequest { me: body, target: Some(tp), sid });
        1
    }

    /// `ccAI::UseItem(code, target)` (gcmn 0x00589410): a member able to act
    /// uses an item it carries (categories 10-15) on a listed target not down
    /// past 1 (or down, for a Rip Maen item) and a command 99 is done. Returns
    /// 1 for categories 11, 12 and 10 other than ids 18-22, 0 otherwise;
    /// failing, the command is dropped and the target let go.
    pub fn use_item(&mut self, me: usize, code: i32, target: Option<usize>) -> i32 {
        let body = self.ai(me).body;
        let later = self.t.volume != Volume::Inf;
        let c = self.scene.chars[body].cond;
        let mut calm = c[cond::SLEEP] == 0
            && (later || c[cond::HOLD] == 0)
            && c[cond::PARALYSIS] == 0
            && c[cond::CHARM] == 0
            && c[cond::CONFUSION] == 0;
        if later && calm {
            // From Mutation on the member must be free to act, and held it
            // may still use one unless a boss is in.
            calm = self.can_act(body, 7);
            if calm && c[cond::HOLD] != 0 && self.check_boss_entry(1).is_some() {
                return -1;
            }
        }
        let cat = code >> 16;
        let id = code & 0xffff;
        if calm && (10..16).contains(&cat) {
            let n = self
                .item_list(self.body_id(me))
                .iter()
                .find(|&&(i, c, _)| i32::from(c) == cat && i32::from(i) == id)
                .map_or(0, |&(_, _, n)| n);
            if n != 0
                && let Some(target) = target.filter(|&t| self.scene.listed(t))
                && (self.scene.chars[target].cond[cond::DEAD] < 2 || self.item_skill(code) == 180)
            {
                self.call(Call::UseItemRequest { me: body, target, item: code, flag: 0 });
                self.consume_item_list(me, code);
                if later {
                    let r = if n == 1 { Remark::UseLastItem } else { Remark::UseItem };
                    self.chat(me, Chat::Remark(r, code));
                }
                if self.ai(me).chat_cmd == 99 {
                    self.ai_mut(me).drop_chat_cmd();
                }
                return match cat {
                    13..=15 => 0,
                    10 if (18..=22).contains(&id) => 0,
                    _ => 1,
                };
            }
        }
        self.give_up(me, false);
        0
    }
}

impl Ctx<'_> {
    /// `ccAI::SelectAttackSkill(tp)` (gcmn 0x005941f0): the skill or item to
    /// open on `tp` with, by [`crate::damage::skill_damage_value`]: the four
    /// best candidates within the SP it may spend, `mercy` holding back
    /// overkill; the best, or 22% of the time (`(rand() >> 3) % 100 < 22`)
    /// another rank. Returns 1 for a skill, 2 for an item (the id or code in
    /// `SelectAttackSkillResult`), -1 for none. The rules, and Mutation's, are
    /// in docs/engine/battle.md ("The decisions").
    pub fn select_attack_skill(&mut self, me: usize, tp: usize) -> i32 {
        self.crew.select_attack_skill_result = -1;
        let body = self.ai(me).body;
        if self.scene.chars[tp].ty() == 0 {
            return -1;
        }
        let list = self.skill_list(self.body_id(me));
        let affordable = |k: i32| {
            list.iter().any(|&s| s != -1 && skill_check_type(self.t, i32::from(s)) == k && self.cost_ok(body, s.into()))
        };
        if !affordable(0) && !affordable(1) && self.check_item_list2(me, 8) == 0 {
            return -1;
        }
        let mut dmg = [0i32; 4];
        let mut cost = [99999i32; 4];
        let mut sid = [-1i32; 4];
        let mut item = [-1i32; 4];
        let mut pick = -1;
        let later = self.t.volume != Volume::Inf;
        // From Mutation on the chance of a lesser rank grows as the target's
        // level falls below the member's: 22% at the same level, 5 points a
        // level, 42% and 2% at the ends.
        let chance = if later {
            let diff = i32::from(self.scene.chars[tp].level()) - i32::from(self.scene.chars[body].level());
            if diff < -3 {
                42
            } else if diff >= 4 {
                2
            } else {
                22 - 5 * diff
            }
        } else {
            22
        };
        if (self.rng.rand() >> 3) % 100 < chance {
            pick = cmod(self.rng.rand() >> 3, 4) + 1;
        }
        let boss = self.check_boss_entry(0).is_some();
        if boss && pick == 4 {
            pick = 1;
        }
        let hp = i32::from(self.scene.chars[tp].hp).wrapping_sub(self.crew.sys.calc_estimate_damage(tp, 5));
        if hp <= 0 {
            return -1;
        }
        let a = self.ai(me);
        let first_time = a.first_time;
        let mask = a.skill_mask;
        let prm = param_of(self.t, a);
        let job = self.scene.chars[body].spc().map_or(0, |p| p.job);
        let sp = i32::from(self.scene.chars[body].sp);
        let normal = self.value(body, tp, 1);
        if hp < normal.wrapping_mul(3) && prm.mercy != 0 && !first_time && job != 5 {
            return -1;
        }
        let max_sp = i32::from(self.scene.chars[body].max_sp);
        let saving = i32::from(prm.saving_sp);
        let keep = max_sp.wrapping_mul(saving);
        let limit = if sp < cdiv(keep, 100) {
            if sp < cdiv(keep, 400) { None } else { Some(cdiv(keep, 400)) }
        } else {
            Some(sp)
        };
        let mercy = prm.mercy != 0;
        if let Some(limit) = limit {
            for (kind, allowed) in [(1, mask & 2 != 0), (0, job != 5 && mask & 1 != 0)] {
                if !allowed {
                    continue;
                }
                for s in list {
                    let s = i32::from(s);
                    if s == -1 || skill_check_type(self.t, s) != kind || (kind == 1 && (2..=5).contains(&s)) {
                        continue;
                    }
                    let c = self.t.skill(s).map_or(0, |p| p.cost);
                    if limit < c {
                        continue;
                    }
                    let d = self.value(body, tp, s);
                    for k in 0..4 {
                        if d < dmg[k] {
                            continue;
                        }
                        if dmg[k] != 0 && cost[k] < c && ee_div(d, dmg[k]) < ee_div(c, cost[k]) {
                            continue;
                        }
                        // From Mutation on of two that do nothing the cheaper
                        // stays.
                        if later && dmg[k] == 0 && d == 0 && cost[k] < c {
                            continue;
                        }
                        if (kind == 0 || job != 5) && mercy && hp < dmg[k] && hp < d && !first_time {
                            continue;
                        }
                        for j in (k + 1..4).rev() {
                            dmg[j] = dmg[j - 1];
                            cost[j] = cost[j - 1];
                            sid[j] = sid[j - 1];
                        }
                        dmg[k] = d;
                        cost[k] = c;
                        sid[k] = s;
                        break;
                    }
                }
            }
        }
        if mask & 2 != 0 {
            for (id, cat, _) in self.item_list(self.body_id(me)) {
                if !(10..16).contains(&cat) || id == -1 {
                    continue;
                }
                let code = item_code(cat, id);
                let s = self.item_skill(code);
                if skill_check_type(self.t, s) != 1 {
                    continue;
                }
                let d = self.value(body, tp, s);
                for k in 0..4 {
                    if d < dmg[k] {
                        continue;
                    }
                    if job != 5 && mercy && hp < dmg[k] && hp < d {
                        continue;
                    }
                    for j in (k + 1..4).rev() {
                        dmg[j] = dmg[j - 1];
                        cost[j] = cost[j - 1];
                        sid[j] = sid[j - 1];
                        item[j] = item[j - 1];
                    }
                    dmg[k] = d;
                    cost[k] = 0;
                    sid[k] = s;
                    item[k] = code;
                    break;
                }
            }
        }
        let mut ret = -1;
        let choose = |crew: &mut Crew, k: usize| {
            if cost[k] == 0 {
                crew.select_attack_skill_result = item[k];
                2
            } else {
                crew.select_attack_skill_result = sid[k];
                1
            }
        };
        let mut fall_back = pick == -1;
        if pick == 4 {
            // The fifth rank is never filled: redraw among the others.
            pick = cmod(self.rng.rand() >> 3, 3) + 1;
        }
        if !fall_back {
            loop {
                let k = pick as usize;
                if sid[k] == -1 && item[k] == -1 {
                    fall_back = true;
                    break;
                }
                ret = choose(self.crew, k);
                if !boss || dmg[k] != 0 {
                    break;
                }
                pick -= 1;
                if pick < 0 {
                    ret = -2;
                    break;
                }
            }
        }
        if fall_back {
            if !first_time && dmg[0] <= 0 {
                ret = -2;
            }
            if item[0] != -1 {
                self.crew.select_attack_skill_result = item[0];
                if ret != -2 {
                    ret = 2;
                }
            } else if sid[0] != -1 {
                self.crew.select_attack_skill_result = sid[0];
                if ret != -2 {
                    ret = 1;
                }
            } else {
                ret = -1;
            }
        }
        if ret == -2 { -1 } else { ret }
    }

    /// Hand a cure, heal or revive to the better placed member `other`
    /// found by [`Ctx::check_skill_other_spc`] (skill form `skill_kind`, or
    /// the item form `item_kind` with the item it found), in 30 frames.
    fn ask_other(&mut self, me: usize, tp: usize, other: usize, skill_kind: i32, item_kind: i32, flag: i32) {
        let body = self.ai(me).body;
        let kind = if flag != -1 { item_kind } else { skill_kind };
        let name = i32::from(self.crew.sys_msg_id(tp)) | kind;
        let s = self.crew.sys_msg_id(body) as u16;
        let r = self.crew.sys_msg_id(other) as u16;
        let delay = self.plan_delay(me);
        self.crew.send(name, s, r, 0, delay, flag, None);
    }

    /// Whether to hand the job over: out of battle, not keeping to itself
    /// (`self_ok`), another member better placed, and (unless it would use
    /// its own skill, `skill_only`) at an even draw of `(rand() >> 3) & 1`.
    fn delegate(&mut self, me: usize, sid: i32, self_ok: bool, skill_only: bool) -> Option<usize> {
        if (self_ok && self.ai(me).self_flag) || self.game.in_battle == 1 {
            return None;
        }
        let other = self.check_skill_other_spc(me, sid)?;
        let flag = self.crew.check_skill_other_flag;
        if skill_only {
            if flag == -1 && (self.rng.rand() >> 3) & 1 != 0 { Some(other) } else { None }
        } else if flag == -1 || (self.rng.rand() >> 3) & 1 != 0 {
            Some(other)
        } else {
            None
        }
    }

    /// The cure kinds of a cure skill: (skill kind, item kind).
    fn cure_kinds(cure: i32) -> Option<(i32, i32)> {
        match cure {
            178 => Some((0x30000, 0x90000)),
            179 => Some((0x40000, 0xa0000)),
            _ => None,
        }
    }

    /// The common tail of the cures: with the cure skill and the SP the
    /// member plans it (a note to itself in 30 frames); else with an item
    /// casting it, it hands the job to a better placed member when
    /// `delegate` finds one (returning 0) or plans the item itself. Returns
    /// 1 when planned. With `item_first` (Mutation on) the item comes
    /// first and the skill only without one.
    fn cure_with(
        &mut self,
        me: usize,
        tp: usize,
        cure: i32,
        self_ok: bool,
        skill_delegates: bool,
        item_first: bool,
    ) -> i32 {
        let Some((sk, it)) = Self::cure_kinds(cure) else { return 0 };
        if !item_first && self.cure_skill_ok(me, cure) {
            if skill_delegates && let Some(other) = self.delegate(me, cure, true, true) {
                self.ask_other(me, tp, other, it, it, 0);
                return 0;
            }
            return self.plan_cure(me, sk, tp, 0);
        }
        let item = self.search_item_list_by_skill(me, cure);
        if item == -1 {
            return if item_first && self.cure_skill_ok(me, cure) { self.plan_cure(me, sk, tp, 0) } else { 0 };
        }
        if let Some(other) = self.delegate(me, cure, self_ok, false) {
            let flag = self.crew.check_skill_other_flag;
            self.ask_other(me, tp, other, sk, it, flag);
            return 0;
        }
        self.plan_cure(me, it, tp, item)
    }

    /// The member has the cure skill and the SP for it.
    fn cure_skill_ok(&self, me: usize, cure: i32) -> bool {
        self.check_skill_list(me, cure) && self.cost_ok(self.ai(me).body, cure)
    }

    /// Plan the cure (`kind`, with `item` for an item's) on `tp`: a note
    /// to itself. Returns 1.
    fn plan_cure(&mut self, me: usize, kind: i32, tp: usize, item: i32) -> i32 {
        self.sys_msg_from_me_to_me(me, kind, tp, self.plan_delay(me), item, None);
        self.done_first(me);
        1
    }

    /// `ccAI::CureSPC(tp, n)` (gcmn 0x00584380): cure `tp` (only itself
    /// when `selfFlag`) of the condition [`Ctx::check_condition_minus`]
    /// finds (`n`, or -3 for 0), unless someone is on it already
    /// ([`Ctx::check_solution`]). Out of battle a member that would use its
    /// own skill hands it, half the time, to another member that has the
    /// skill too.
    pub fn cure_spc(&mut self, me: usize, tp: usize, n: i32) -> i32 {
        if self.ai(me).self_flag && tp != self.ai(me).body {
            return 0;
        }
        let c = self.check_condition_minus(me, Some(tp), if n == 0 { -3 } else { n });
        if c == 0 {
            return 0;
        }
        let cure = crate::skill::recovery_check(c);
        let Some((sk, _)) = Self::cure_kinds(cure) else { return 0 };
        if self.check_solution(sk, Some(tp)) {
            return 0;
        }
        self.cure_with(me, tp, cure, true, true, false)
    }

    /// The members the party cures reach: slots 0-2 other than the member,
    /// in the party (`partyFlag` above 0), alive or down.
    fn patients(&self, me: usize) -> Vec<usize> {
        let id = i32::from(self.body_id(me));
        (0..3)
            .filter(|&n| self.party.ids[n] != id)
            .filter_map(|n| self.party.members[n])
            .filter(|&c| party_flag(&self.scene.chars[c]) > 0 && up(&self.scene.chars[c]))
            .collect()
    }

    /// `ccAI::CureOtherSPC(n)` (gcmn 0x00584820): the same for the first
    /// other member (in slot order) with a condition no one is curing;
    /// only an item cure may be handed over. Nothing when `selfFlag`.
    pub fn cure_other_spc(&mut self, me: usize, n: i32) -> i32 {
        if self.ai(me).self_flag {
            return 0;
        }
        for c in self.patients(me) {
            let k = self.check_condition_minus(me, Some(c), if n == 0 { -3 } else { n });
            if k == 0 {
                continue;
            }
            let cure = crate::skill::recovery_check(k);
            let Some((sk, _)) = Self::cure_kinds(cure) else { continue };
            if self.check_solution(sk, Some(c)) {
                continue;
            }
            match self.cure_other_with(me, c, cure, true, false) {
                Some(v) => return v,
                None => continue,
            }
        }
        0
    }

    /// [`Ctx::cure_with`] for the "other member" cures: an item handed over
    /// moves on to the next member (None) instead of stopping.
    fn cure_other_with(&mut self, me: usize, tp: usize, cure: i32, self_ok: bool, item_first: bool) -> Option<i32> {
        let (sk, it) = Self::cure_kinds(cure)?;
        if !item_first && self.cure_skill_ok(me, cure) {
            return Some(self.plan_cure(me, sk, tp, 0));
        }
        let item = self.search_item_list_by_skill(me, cure);
        if item == -1 {
            return (item_first && self.cure_skill_ok(me, cure)).then(|| self.plan_cure(me, sk, tp, 0));
        }
        if let Some(other) = self.delegate(me, cure, self_ok, false) {
            let flag = self.crew.check_skill_other_flag;
            self.ask_other(me, tp, other, sk, it, flag);
            return None;
        }
        Some(self.plan_cure(me, it, tp, item))
    }

    /// `CureOnlyCharmConfusionOtherSPC` (gcmn 0x00584c50) and
    /// `CureOnlyParalysisSleepOtherSPC` (0x00585050): [`Ctx::cure_other_spc`]
    /// for charm (else confusion), or paralysis (else sleep), only; the item
    /// may be handed over whatever `selfFlag`. From Mutation on both take
    /// `item_first` (see [`Ctx::cure_with`]).
    fn cure_only_other(&mut self, me: usize, first: (usize, i32), second: (usize, i32), item_first: i32) -> i32 {
        if self.ai(me).self_flag {
            return 0;
        }
        for c in self.patients(me) {
            let cd = &self.scene.chars[c].cond;
            let k = if cd[first.0] != 0 {
                first.1
            } else if cd[second.0] != 0 {
                second.1
            } else {
                continue;
            };
            let cure = crate::skill::recovery_check(k);
            let Some((sk, _)) = Self::cure_kinds(cure) else { continue };
            if self.check_solution(sk, Some(c)) {
                continue;
            }
            match self.cure_other_with(me, c, cure, false, item_first != 0) {
                Some(v) => return v,
                None => continue,
            }
        }
        0
    }

    /// `ccAI::CureOnlyCharmConfusionOtherSPC` (gcmn 0x00584c50).
    pub fn cure_only_charm_confusion_other_spc(&mut self, me: usize, item_first: i32) -> i32 {
        self.cure_only_other(me, (cond::CHARM, 159), (cond::CONFUSION, 161), item_first)
    }

    /// `ccAI::CureOnlyParalysisSleepOtherSPC` (gcmn 0x00585050).
    pub fn cure_only_paralysis_sleep_other_spc(&mut self, me: usize, item_first: i32) -> i32 {
        self.cure_only_other(me, (cond::PARALYSIS, 157), (cond::SLEEP, 160), item_first)
    }

    /// `ccAI::CurePoisonSPC` (gcmn 0x00585450): cure the poisoned member
    /// with the least HP no one is curing (the member itself only, with
    /// `selfFlag`), with Rip Teyn or an item casting it.
    pub fn cure_poison_spc(&mut self, me: usize) -> i32 {
        self.cure_poison_with(me, false, false)
    }

    /// Mutation's `CurePoisonSPC` with curse (gcmn 0x005ac400, unnamed):
    /// the same for a member poisoned (cured by Rip Teyn's kind) or cursed
    /// (by 162's, kind 0x40000), poison first, with `item_first`.
    pub fn cure_poison_curse_spc(&mut self, me: usize, item_first: i32) -> i32 {
        self.cure_poison_with(me, true, item_first != 0)
    }

    /// The conditions [`Ctx::cure_poison_spc`] looks for and whether
    /// someone is on each already: poison, and with `curse` the curse.
    fn poison_needs(&mut self, c: usize, curse: bool) -> Option<bool> {
        let cd = self.scene.chars[c].cond;
        let poison = cd[cond::POISON] != 0;
        let cursed = curse && cd[cond::CURSE] != 0;
        if !poison && !cursed {
            return None;
        }
        Some((poison && self.check_solution(0x30000, Some(c))) || (cursed && self.check_solution(0x40000, Some(c))))
    }

    fn cure_poison_with(&mut self, me: usize, curse: bool, item_first: bool) -> i32 {
        let body = self.ai(me).body;
        let mut tgt = None;
        if self.ai(me).self_flag {
            if self.poison_needs(body, curse) != Some(false) {
                return 0;
            }
            tgt = Some(body);
        } else {
            let mut best = 9999;
            for n in 0..3 {
                let Some(c) = self.party.members[n] else { continue };
                let ch = &self.scene.chars[c];
                if party_flag(ch) <= 0 || !up(ch) {
                    continue;
                }
                if self.poison_needs(c, curse) != Some(false) {
                    continue;
                }
                let hp = i32::from(self.scene.chars[c].hp);
                if hp < best {
                    best = hp;
                    tgt = Some(c);
                }
            }
        }
        let Some(tgt) = tgt else { return 0 };
        let k = if self.scene.chars[tgt].cond[cond::POISON] != 0 { 156 } else { 162 };
        self.cure_with(me, tgt, crate::skill::recovery_check(k), true, false, item_first)
    }

    /// `ccAI::HealSPC(tp, rate)` (gcmn 0x00596b00): heal `tp` (or the neediest
    /// member) when below `rate`% of its maxHP: plan a healing skill (0x20000)
    /// or item (0x80000), or hand the heal to a member that heals better.
    /// Returns 1 when planned. From Mutation on ties and hand-overs change and
    /// `item_first` tries the item first (docs/engine/battle.md, "The
    /// decisions").
    pub fn heal_spc(&mut self, me: usize, tp: Option<usize>, rate: u32, item_first: i32) -> i32 {
        if self.t.volume != Volume::Inf {
            return self.heal_spc_later(me, tp, rate, item_first);
        }
        let body = self.ai(me).body;
        let tp = if self.ai(me).self_flag { Some(body) } else { tp };
        let below = |ctx: &Self, c: usize| {
            let ch = &ctx.scene.chars[c];
            if !up(ch) {
                return false;
            }
            let line = fptosi(ee::div(ee::mul(rate, ee::from_int(i32::from(ch.max_hp))), 0x42c8_0000));
            i32::from(ch.hp) < line
        };
        let tgt = match tp {
            None => {
                let mut best = 9999;
                let mut tgt = None;
                for n in 0..3 {
                    let Some(c) = self.party.members[n] else { continue };
                    if party_flag(&self.scene.chars[c]) <= 0 || !up(&self.scene.chars[c]) {
                        continue;
                    }
                    if below(self, c) && !self.check_solution(0x20000, Some(c)) {
                        let hp = i32::from(self.scene.chars[c].hp);
                        if hp < best {
                            best = hp;
                            tgt = Some(c);
                        }
                    }
                }
                tgt
            }
            Some(c) => {
                if !below(self, c) || self.check_solution(0x20000, Some(c)) {
                    return 0;
                }
                Some(c)
            }
        };
        let Some(tgt) = tgt else { return 0 };
        let me_id = self.crew.sys_msg_id(body) as u16;
        let tid = i32::from(self.crew.sys_msg_id(tgt));
        if self.check_heal_hp_skill(me) != 0 {
            if !self.ai(me).self_flag && self.game.in_battle != 1 {
                let other = self.check_heal_skill_other_spc(me, 1);
                if let Some(other) = other
                    && self.crew.check_heal_skill_other_flag == -1
                    && (self.rng.rand() >> 3) & 1 != 0
                {
                    let r = self.crew.sys_msg_id(other) as u16;
                    self.crew.post_in(tid | 0x20000, 0, None, me_id, r, u32::from(self.plan_delay(me)));
                    return 0;
                }
            }
            self.crew.post_in(tid | 0x20000, 0, None, me_id, me_id, u32::from(self.plan_delay(me)));
            self.done_first(me);
            return 1;
        }
        let t = &self.scene.chars[tgt];
        let item = self.search_item_list_by_heal(me, i32::from(t.max_hp) - i32::from(t.hp));
        if item == -1 {
            return 0;
        }
        if !self.ai(me).self_flag && self.game.in_battle != 1 {
            let other = self.check_heal_skill_other_spc(me, 1);
            if let Some(other) = other {
                let flag = self.crew.check_heal_skill_other_flag;
                if flag == -1 || (self.rng.rand() >> 3) != 0 {
                    let kind = if flag != -1 { 0x80000 } else { 0x20000 };
                    let r = self.crew.sys_msg_id(other) as u16;
                    self.crew.post_in(tid | kind, flag, None, me_id, r, u32::from(self.plan_delay(me)));
                    return 0;
                }
            }
        }
        self.crew.post_in(tid | 0x80000, item, None, me_id, me_id, u32::from(self.plan_delay(me)));
        self.done_first(me);
        1
    }

    /// Mutation's [`Ctx::resurrect_spc`] (gcmn MUT 0x005bfbb0), `item_first`
    /// its argument: every slot's party member down (not 5) no one is
    /// reviving, the item tried before the skill with `item_first`; an item
    /// handed to a better placed member moves on to the next member.
    fn resurrect_spc_later(&mut self, me: usize, item_first: i32) -> i32 {
        let body = self.ai(me).body;
        let item = self.usable_items(self.body_id(me)).into_iter().find(|&c| self.item_skill(c) == 180).unwrap_or(-1);
        let skill = self.check_skill_list(me, 180) && self.cost_ok(body, 180);
        if item == -1 && !skill {
            return 0;
        }
        let id = i32::from(self.body_id(me));
        for slot in 0..3 {
            if self.party.ids[slot] == id {
                continue;
            }
            let Some(c) = self.party.members[slot] else { continue };
            let d = self.scene.chars[c].cond[cond::DEAD];
            if party_flag(&self.scene.chars[c]) <= 0 || d == 0 || d == 5 || self.check_solution(0x50000, Some(c)) {
                continue;
            }
            let me_id = self.crew.sys_msg_id(body) as u16;
            let cid = i32::from(self.crew.sys_msg_id(c));
            let delay = u32::from(self.plan_delay(me));
            let by_skill = |ctx: &mut Self| {
                ctx.crew.post_in(cid | 0x50000, 0, None, me_id, me_id, delay);
                ctx.done_first(me);
                1
            };
            if item_first == 0 && skill {
                return by_skill(self);
            }
            if item == -1 {
                if skill {
                    return by_skill(self);
                }
                continue;
            }
            if self.game.in_battle != 1
                && let Some(other) = self.check_skill_other_spc(me, 180)
            {
                let flag = self.crew.check_skill_other_flag;
                if flag == -1 || (self.rng.rand() >> 3) & 1 != 0 {
                    let kind = if flag != -1 { 0xb0000 } else { 0x50000 };
                    let r = self.crew.sys_msg_id(other) as u16;
                    self.crew.post_in(cid | kind, flag, None, me_id, r, delay);
                    continue;
                }
            }
            self.crew.post_in(cid | 0xb0000, item, None, me_id, me_id, delay);
            self.done_first(me);
            return 1;
        }
        0
    }

    /// Mutation's [`Ctx::heal_spc`] (gcmn MUT 0x005c06b0).
    fn heal_spc_later(&mut self, me: usize, tp: Option<usize>, rate: u32, item_first: i32) -> i32 {
        let body = self.ai(me).body;
        let in_battle = self.game.in_battle == 1;
        let tp = if self.ai(me).self_flag { Some(body) } else { tp };
        let below = |ctx: &Self, c: usize| {
            let ch = &ctx.scene.chars[c];
            up(ch) && {
                let line = fptosi(ee::div(ee::mul(rate, ee::from_int(i32::from(ch.max_hp))), F_100));
                i32::from(ch.hp) < line
            }
        };
        let tgt = match tp {
            None => {
                let (mut best, mut best_sp, mut tgt) = (9999, 0, None);
                for n in 0..3 {
                    let Some(c) = self.party.members[n] else { continue };
                    if party_flag(&self.scene.chars[c]) <= 0 || !up(&self.scene.chars[c]) {
                        continue;
                    }
                    if !below(self, c) || self.check_solution(0x20000, Some(c)) {
                        continue;
                    }
                    let (hp, sp) = (i32::from(self.scene.chars[c].hp), i32::from(self.scene.chars[c].sp));
                    if best < hp || (in_battle && best == hp && sp < best_sp) {
                        continue;
                    }
                    (best, best_sp, tgt) = (hp, sp, Some(c));
                }
                tgt
            }
            Some(c) => {
                if !below(self, c) || self.check_solution(0x20000, Some(c)) {
                    return 0;
                }
                Some(c)
            }
        };
        let Some(tgt) = tgt else { return 0 };
        let loss = i32::from(self.scene.chars[tgt].max_hp) - i32::from(self.scene.chars[tgt].hp);
        if item_first != 0 {
            if let Some(v) = self.heal_by_item(me, tgt, loss) {
                return v;
            }
            return self.heal_by_skill(me, tgt, loss).unwrap_or(0);
        }
        if let Some(v) = self.heal_by_skill(me, tgt, loss) {
            return v;
        }
        self.heal_by_item(me, tgt, loss).unwrap_or(0)
    }

    /// Mutation's heal by skill: None without a healing skill.
    fn heal_by_skill(&mut self, me: usize, tgt: usize, loss: i32) -> Option<i32> {
        let heal = self.check_heal_hp_skill(me);
        if heal == 0 {
            return None;
        }
        let body = self.ai(me).body;
        let me_id = self.crew.sys_msg_id(body) as u16;
        let tid = i32::from(self.crew.sys_msg_id(tgt));
        let delay = u32::from(self.plan_delay(me));
        if !self.ai(me).self_flag && heal < loss && self.game.in_battle != 1 {
            let other = self.check_heal_skill_other_spc(me, heal);
            if let Some(other) = other
                && self.crew.check_heal_skill_other_flag == -1
                && (self.rng.rand() >> 3) & 1 != 0
            {
                let r = self.crew.sys_msg_id(other) as u16;
                self.crew.post_in(tid | 0x20000, 0, None, me_id, r, delay);
                return Some(0);
            }
        }
        self.crew.post_in(tid | 0x20000, 0, None, me_id, me_id, delay);
        self.done_first(me);
        Some(1)
    }

    /// Mutation's heal by item: None without a healing item for the loss.
    fn heal_by_item(&mut self, me: usize, tgt: usize, loss: i32) -> Option<i32> {
        let item = self.search_item_list_by_heal(me, loss);
        if item == -1 {
            return None;
        }
        let body = self.ai(me).body;
        let me_id = self.crew.sys_msg_id(body) as u16;
        let tid = i32::from(self.crew.sys_msg_id(tgt));
        let delay = u32::from(self.plan_delay(me));
        if !self.ai(me).self_flag && self.game.in_battle != 1 {
            let heal = item_heal(self.item_skill(item));
            if let Some(other) = self.check_heal_skill_other_spc(me, heal) {
                let flag = self.crew.check_heal_skill_other_flag;
                if flag == -1 || (self.rng.rand() >> 3) != 0 {
                    let kind = if flag != -1 { 0x80000 } else { 0x20000 };
                    let r = self.crew.sys_msg_id(other) as u16;
                    self.crew.post_in(tid | kind, flag, None, me_id, r, delay);
                    return Some(0);
                }
            }
        }
        self.crew.post_in(tid | 0x80000, item, None, me_id, me_id, delay);
        self.done_first(me);
        Some(1)
    }

    /// `ccAI::ResurrectSPC` (gcmn 0x00596490): revive the first member down
    /// (dead, not 5) no one is reviving, with Rip Maen (0x50000) if it has
    /// it and the SP, else with a Rip Maen item (0xb0000), handed out of
    /// battle to a better placed member as the cures are. Nothing when
    /// `selfFlag`.
    pub fn resurrect_spc(&mut self, me: usize, item_first: i32) -> i32 {
        if self.ai(me).self_flag {
            return 0;
        }
        if self.t.volume != Volume::Inf {
            return self.resurrect_spc_later(me, item_first);
        }
        let body = self.ai(me).body;
        let item = self.usable_items(self.body_id(me)).into_iter().find(|&c| self.item_skill(c) == 180).unwrap_or(-1);
        let skill = self.check_skill_list(me, 180) && self.cost_ok(body, 180);
        if item == -1 && !skill {
            return 0;
        }
        let id = i32::from(self.body_id(me));
        let mut n = 0;
        while n < self.party.num {
            let slot = n as usize;
            n += 1;
            if self.party.ids.get(slot).copied().unwrap_or(-1) == id {
                continue;
            }
            let Some(c) = self.party.members.get(slot).copied().flatten() else { continue };
            let d = self.scene.chars[c].cond[cond::DEAD];
            if d == 0 || d == 5 || self.check_solution(0x50000, Some(c)) {
                continue;
            }
            let me_id = self.crew.sys_msg_id(body) as u16;
            let cid = i32::from(self.crew.sys_msg_id(c));
            if skill {
                self.crew.post_in(cid | 0x50000, 0, None, me_id, me_id, u32::from(self.plan_delay(me)));
                self.done_first(me);
                return 1;
            }
            if item == -1 {
                continue;
            }
            if self.game.in_battle != 1
                && let Some(other) = self.check_skill_other_spc(me, 180)
            {
                let flag = self.crew.check_skill_other_flag;
                if flag == -1 || (self.rng.rand() >> 3) & 1 != 0 {
                    let kind = if flag != -1 { 0xb0000 } else { 0x50000 };
                    let r = self.crew.sys_msg_id(other) as u16;
                    self.crew.post_in(cid | kind, flag, None, me_id, r, u32::from(self.plan_delay(me)));
                    return 0;
                }
            }
            self.crew.post_in(cid | 0xb0000, item, None, me_id, me_id, u32::from(self.plan_delay(me)));
            self.done_first(me);
            return 1;
        }
        0
    }

    /// `ccAI::HealPlzNormalMode` (gcmn 0x005972d0): out of battle, the first
    /// of: cure charm or confusion, revive, cure poison, heal anyone below
    /// full HP, cure paralysis or sleep, cure Kite, cure itself, cure the
    /// others.
    ///
    /// From Mutation on the member decides first whether items come
    /// before skills ([`Ctx::item_first`]), and the poison cure also cures
    /// curse ([`Ctx::cure_poison_curse_spc`]).
    pub fn heal_plz_normal_mode(&mut self, me: usize) -> i32 {
        let later = self.t.volume != Volume::Inf;
        let first = i32::from(later && self.item_first(me) == 1);
        if self.cure_only_charm_confusion_other_spc(me, first) != 0
            || self.resurrect_spc(me, first) != 0
            || (if later { self.cure_poison_curse_spc(me, first) } else { self.cure_poison_spc(me) }) != 0
            || self.heal_spc(me, None, 0x42c8_0000, first) != 0
            || self.cure_only_paralysis_sleep_other_spc(me, 0) != 0
        {
            return 1;
        }
        let kite = self.party.members[0];
        if let Some(k) = kite
            && self.cure_spc(me, k, 0) != 0
        {
            return 1;
        }
        let body = self.ai(me).body;
        if self.cure_spc(me, body, 0) != 0 {
            return 1;
        }
        i32::from(self.cure_other_spc(me, 0) != 0)
    }

    /// `ccAI::HealPlzBattleMode` (gcmn 0x00597410): in battle: revive, cure
    /// charm or confusion, heal anyone below a third of their HP (all of it
    /// while a heal command is being carried out, `chatCmdFlag` 2), cure
    /// paralysis or sleep, cure poison, cure Kite, itself, the others, heal
    /// anyone below 75%.
    ///
    /// From Mutation on (with [`Ctx::item_first`]'s answer) paralysis and
    /// sleep are cured before the heal.
    pub fn heal_plz_battle_mode(&mut self, me: usize) -> i32 {
        let later = self.t.volume != Volume::Inf;
        let first = i32::from(later && self.item_first(me) == 1);
        if self.resurrect_spc(me, first) != 0 || self.cure_only_charm_confusion_other_spc(me, first) != 0 {
            return 1;
        }
        if later && self.cure_only_paralysis_sleep_other_spc(me, first) != 0 {
            return 1;
        }
        let rate = if self.ai(me).chat_cmd_flag == 2 { 0x42c8_0000 } else { 0x4205_54fe };
        if self.heal_spc(me, None, rate, first) != 0
            || (!later && self.cure_only_paralysis_sleep_other_spc(me, 0) != 0)
            || self.cure_poison_spc(me) != 0
        {
            return 1;
        }
        let kite = self.party.members[0];
        if let Some(k) = kite
            && self.cure_spc(me, k, 0) != 0
        {
            return 1;
        }
        let body = self.ai(me).body;
        if self.cure_spc(me, body, 0) != 0 || self.cure_other_spc(me, 0) != 0 {
            return 1;
        }
        i32::from(self.heal_spc(me, None, 0x4296_0000, 0) != 0)
    }

    /// Mutation's choice of items before skills (gcmn 0x005c1ae0,
    /// unnamed): -1 when the member cannot act (down, asleep, confused,
    /// charmed or paralysed); 1 when it is the only one standing
    /// ([`Ctx::live_members`]), or when every other member is unable, or
    /// every other member is in danger (HP at most its biggest recent hit,
    /// [`Ai::hit_recent`], or at most a tenth of its maxHP); else 0.
    pub fn item_first(&self, me: usize) -> i32 {
        let body = self.ai(me).body;
        let unable = |ch: &Char| {
            let c = &ch.cond;
            !up(ch) || c[cond::SLEEP] != 0 || c[cond::CONFUSION] != 0 || c[cond::CHARM] != 0 || c[cond::PARALYSIS] != 0
        };
        if unable(&self.scene.chars[body]) {
            return -1;
        }
        if self.live_members() == 1 {
            return 1;
        }
        let (mut out, mut danger) = (0, 0);
        for c in self.party.members.iter().flatten().copied().filter(|&c| c != body) {
            let ch = &self.scene.chars[c];
            if unable(ch) {
                out += 1;
            }
            // A member with no AI reads its hit as 0.
            let hit = self.crew.ais.get(&c).map_or(0, |a| a.hit_recent);
            if ch.hp <= hit || ch.hp <= ch.max_hp / 10 {
                danger += 1;
            }
        }
        let n = self.party.num - 1;
        i32::from(!(out < n && danger < n))
    }

    /// Mutation's count of the members standing (gcmn 0x005c8300,
    /// unnamed): `partyNum` less the members down or dead.
    pub fn live_members(&self) -> i32 {
        let down = self.party.members.iter().flatten().filter(|&&c| !up(&self.scene.chars[c])).count();
        self.party.num - down as i32
    }

    /// `ccAI::ChatCommandHealPlz` (gcmn 0x005962d0): a member allowed to heal
    /// (`skillMask` 4), able, not in a skill and with nothing scheduled looks
    /// after the party; out of battle it then goes back to its strategy and a
    /// heal command (16) is done. Mutation's changes are in
    /// docs/engine/battle.md ("The decisions").
    pub fn chat_command_heal_plz(&mut self, me: usize) -> i32 {
        if self.ai(me).skill_mask & 4 == 0 {
            return 0;
        }
        let later = self.t.volume != Volume::Inf;
        if later && !self.ai(me).first_time && self.ai(me).count % 10 != 0 {
            return 0;
        }
        let body = self.ai(me).body;
        let c = &self.scene.chars[body].cond;
        if c[cond::CONFUSION] != 0
            || c[cond::SLEEP] != 0
            || c[cond::CHARM] != 0
            || (!later && c[cond::HOLD] != 0)
            || c[cond::PARALYSIS] != 0
            || c[cond::DEAD] != 0
        {
            return 0;
        }
        if self.scene.chars[body].skill_id >= 2 {
            return 0;
        }
        if later && !self.can_act(body, 6) && !self.can_act(body, 7) {
            return 0;
        }
        if self.check_healing_schedule(me) != 0 {
            return 0;
        }
        let v = if self.game.in_battle == 1 { self.heal_plz_battle_mode(me) } else { self.heal_plz_normal_mode(me) };
        if self.game.in_battle != 1 && !(later && self.check_heal_party(me, F_100 as i32) > 0) {
            let a = self.ai_mut(me);
            if a.strategy == 3 {
                a.self_flag = true;
                a.skill_mask = 4;
            } else {
                a.skill_mask = 0;
            }
            if a.chat_cmd == 16 {
                a.drop_chat_cmd();
            }
        }
        self.done_first(me);
        v
    }

    /// `ccAI::AttackTarget(tp)` (gcmn 0x00586630): attack `tp`. A target of
    /// the member's own side is let go; a party member in a field with no boss
    /// fight more than 3500 from Kite turns back to him (0x10013 to itself in
    /// 40 frames). Otherwise it closes in and attacks: Kite through
    /// `ccPlayer::Attack`, a member of type 4 through [`Ctx::fellow_attack`].
    pub fn attack_target(&mut self, me: usize, tp: usize) {
        let body = self.ai(me).body;
        let c = &self.scene.chars[body].cond;
        let (confused, charmed) = (c[cond::CONFUSION] != 0, c[cond::CHARM] != 0);
        let tgt_ty = self.ai(me).target.map(|t| self.scene.chars[t].ty());
        let stop = if !confused && !charmed {
            tgt_ty.is_some_and(|t| t as u32 & 0x0700_000f != 0)
        } else {
            charmed && tgt_ty.is_some_and(|t| t & 0xe0 != 0)
        };
        if stop {
            self.ai_mut(me).target_flag = 0;
            if self.scene.chars[body].skill_id == 1 {
                self.stop_normal_attack(me);
            }
            return;
        }
        let later = self.t.volume != Volume::Inf;
        // From Mutation on a charmed or confused member does not turn back.
        if party_flag(&self.scene.chars[body]) == 1 && !(later && (confused || charmed)) {
            if self.ai(me).go_back_flag {
                return;
            }
            if self.check_boss_entry(0).is_none() && self.game.area == 1 && !ee::le(self.ai(me).dist_pl, 0x455a_c000) {
                self.ai_mut(me).go_back_flag = true;
                let id = self.ai(me).sys_msg.id as u16;
                self.crew.sys.delete_delay(0x10013, id, id);
                self.crew.send(0x10013, id, id, 0, 40, -1, Some(body));
                let a = self.ai_mut(me);
                a.follow_sw = true;
                a.target_flag = 0;
                a.target = None;
                self.drop_target_char(body);
                return;
            }
        }
        // From Mutation on a member holding its place (strategy 6) stays
        // where it is for a target within 1900, and attacks from there.
        let held = later
            && !confused
            && !charmed
            && self.ai(me).strategy == 6
            && ee::le(distance_to_target(self.t.volume, self.scene, body, Some(tp)), F_1900);
        if held {
            if self.ai(me).navi_finish == 0 {
                self.ai_mut(me).navi_finish = 1;
            }
            if self.crew.spc.get(&body).is_some_and(|s| s.move_flag) {
                self.halt(body);
            }
        } else if self.ai(me).navi_finish != 0 {
            self.call(Call::FollowTarget { me: body, target: Some(tp) });
        } else {
            self.call(Call::FollowBeacon { me: body });
            let d = distance_to_target(self.t.volume, self.scene, body, Some(tp));
            if ee::le(d, 0x4316_0000) {
                self.ai_mut(me).navi_finish = 1;
            } else if cmod(self.ai(me).count, 90) == 0 {
                let pos = self.scene.chars[tp].pos;
                if self.call(Call::GoalBeacon { me: body, pos }) != 0 {
                    self.ai_mut(me).navi_finish = 1;
                }
            }
        }
        let tyb = self.scene.chars[body].ty();
        let n = i32::from(self.ai(me).strategy);
        if tyb & 1 != 0 {
            self.call(Call::PlayerAttack { me: body, target: tp, n });
        } else if tyb & 4 != 0 {
            self.fellow_attack(body, tp, n);
        }
    }

    /// `ccFellow::Attack(tp, n)` (gcmn 0x0041e590): a member's attack on `tp`.
    /// Before its first action or every 180th `cycle` it may choose an art or
    /// spell ([`Ctx::select_attack_skill`]); then the normal attack starts
    /// within `armsRange`, a skill within its `triggerRange`, an item through
    /// [`Ctx::use_item`], each announcing its estimate (0x10008). Returns 1
    /// done, -1 out of range, 0 unable.
    pub fn fellow_attack(&mut self, body: usize, tp: usize, _n: i32) -> i32 {
        let mut s0 = 1;
        let mut item = false;
        if self.scene.chars[body].skill_id >= 2 {
            return 1;
        }
        let me = body;
        let has_ai = self.crew.ais.contains_key(&me);
        let first = has_ai && self.ai(me).first_time;
        let cycle = self.crew.spc.get(&body).map_or(0, |s| s.cycle);
        if first || cmod(cycle, 180) == 0 {
            let c = &self.scene.chars[body].cond;
            if c[cond::CONFUSION] == 0
                && c[cond::CHARM] == 0
                && (self.can_act(body, 6) || self.can_act(body, 7))
                && self.check_skill_mask(me) & 3 != 0
            {
                let r = self.select_attack_skill(me, tp);
                if r < 0 {
                    s0 = 1;
                } else {
                    item = r == 2;
                    s0 = self.crew.select_attack_skill_result;
                }
            }
        }
        self.crew.spc.entry(body).or_default().target_char = Some(tp);
        let dist = distance_to_target(self.t.volume, self.scene, body, Some(tp));
        let dead1 = self.scene.chars[tp].cond[cond::DEAD] == 1;
        let request = if s0 == 1 {
            if !self.can_act(body, 3) {
                return 0;
            }
            if self.scene.chars[body].skill_id == 1 {
                if dead1 {
                    self.call(Call::SkillRequest { me: body, target: Some(tp), sid: 0 });
                }
                return 1;
            }
            if dead1 {
                return 0;
            }
            let spc = self.crew.spc.get(&body).copied().unwrap_or_default();
            let near = if spc.move_flag {
                ee::lt(ee::sub(dist, spc.now_speed), 0x41f0_0000)
            } else {
                let arms = param_of(self.t, self.ai(me)).arms_range;
                if ee::cmp(F_MINUS_ONE, dist) == std::cmp::Ordering::Equal {
                    ee::le(self.ai(me).dist_tg, arms)
                } else {
                    ee::le(dist, arms)
                }
            };
            if !near {
                return -1;
            }
            true
        } else if item {
            if !self.can_act(body, 7) {
                return 0;
            }
            let sk = self.item_skill(s0);
            if !self.t.skill(sk).is_some_and(|k| crate::skill::range_check(k, dist)) {
                return -1;
            }
            if dead1 {
                return 0;
            }
            if self.use_item(me, s0, Some(tp)) == 1 {
                self.done_first(me);
            }
            false
        } else {
            if !self.can_act(body, 6) {
                return 0;
            }
            if !self.t.skill(s0).is_some_and(|k| crate::skill::range_check(k, dist)) {
                return -1;
            }
            if dead1 {
                return 0;
            }
            self.done_first(me);
            true
        };
        if request {
            let target = self.crew.spc.get(&body).and_then(|s| s.target_char);
            self.call(Call::SkillRequest { me: body, target, sid: s0 });
        }
        let a = self.ai_mut(me);
        a.atk_target_cnt = a.atk_target_cnt.wrapping_add(1);
        let target = self.crew.spc.get(&body).and_then(|s| s.target_char);
        let id = self.crew.sys_msg_id(body) as u16;
        if item {
            let sk = self.item_skill(s0);
            let v = target.map_or(0, |t| self.value(body, t, sk));
            self.crew.send(0x10008, id, id, 0, 0, v, target);
        } else if s0 != 1 {
            let v = target.map_or(0, |t| self.value(body, t, s0));
            self.crew.send(0x10008, id, id, 0, 0, v, target);
        }
        1
    }
}

/// The member commands (`ccAI::RequestChatCmd`'s `cmd`, the chat window's
/// orders): 0-4 and 7-10 set a strategy, 11 attack Kite's target, 13 use
/// arts, 14 use spells, 16 heal, 17 debuff, 18 buff, 19 play the Sprite
/// Ocarina, 5 use a given skill, 99 (set by the check) use an item for it.
pub mod cmd {
    pub const ATTACK_TARGET: i16 = 11;
    pub const USE_SKILL: i16 = 5;
    pub const HEAL: i16 = 16;
    pub const DEBUFF: i16 = 17;
    pub const BUFF: i16 = 18;
    pub const OCARINA: i16 = 19;
    pub const USE_ITEM: i16 = 99;
    /// The Sprite Ocarina's item code.
    pub const OCARINA_ITEM: i32 = 0xd_0001;
}

impl Ctx<'_> {
    /// The body is paralysed, asleep, confused or charmed.
    fn troubled(&self, body: usize) -> bool {
        let c = &self.scene.chars[body].cond;
        c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 || c[cond::CONFUSION] != 0 || c[cond::CHARM] != 0
    }

    /// `ccAI::RequestChatCmd(cmd, tp, sid)` (gcmn 0x00583440): the player gives
    /// the member a command. A member paralysed, asleep, confused or charmed
    /// only answers that it cannot (`ChatMessageConditionMinus`); otherwise the
    /// command sets the strategy and `skillMask` at once (the table is in
    /// docs/engine/battle.md, "The decisions") and is noted for
    /// [`Ctx::chat_command`]. From Mutation on see [`Ctx::request_chat_cmd_later`].
    pub fn request_chat_cmd(&mut self, me: usize, cmd: i32, tp: Option<usize>, sid: i32) {
        let body = self.ai(me).body;
        if self.troubled(body) {
            self.chat(me, Chat::ConditionMinus);
            return;
        }
        if self.t.volume != Volume::Inf {
            return self.request_chat_cmd_later(me, cmd, tp, sid);
        }
        let reset_cmd = |ctx: &mut Self| {
            if matches!(ctx.ai(me).strategy_cmd, 0 | 2 | 3) {
                ctx.change_strategy_cmd(me, 0);
            }
        };
        match cmd {
            7..=10 | 1..=4 => {
                let n = (if cmd >= 7 { cmd - 7 } else { cmd - 1 }) as i16;
                if cmd >= 7 {
                    self.ai_mut(me).strategy_cmd = n;
                }
                self.change_strategy(me, n);
                if n == 3 {
                    let a = self.ai_mut(me);
                    a.skill_mask = 4;
                    a.self_flag = true;
                } else {
                    self.ai_mut(me).skill_mask = 0;
                }
            }
            0 | 13 | 14 => {
                reset_cmd(self);
                self.ai_mut(me).skill_mask = match cmd {
                    0 => 3,
                    13 => 1,
                    _ => 2,
                };
            }
            15 => self.ai_mut(me).skill_mask = 0,
            16 | 18 => {
                let a = self.ai_mut(me);
                a.skill_mask = if cmd == 16 { 4 } else { 16 };
                a.self_flag = false;
            }
            17 => self.ai_mut(me).skill_mask = 8,
            11 => {
                self.change_strategy(me, 4);
                if self.scene.chars[body].spc().is_some_and(|p| p.job == 5) {
                    self.ai_mut(me).skill_mask = 3;
                }
            }
            5 | 12 | 19 => {}
            _ => {
                self.change_strategy(me, 0);
                self.ai_mut(me).skill_mask = 0;
            }
        }
        let a = self.ai_mut(me);
        a.chat_cmd_new = cmd as i16;
        a.chat_cmd_skill = sid as i16;
        a.target_ccmd = tp;
        a.chat_cmd_flag = -2;
    }

    /// Mutation's `RequestChatCmd` (gcmn 0x005a97e0) past the condition check:
    /// Infection's table with a standing hold (`strategyCMD` 6), which most
    /// commands turn into 3; 0 and 13 keep a standing 1 or 4 and otherwise
    /// clear it; 14 makes a standing 3 (or 0 or 2 with strategy 3) into 6.
    fn request_chat_cmd_later(&mut self, me: usize, cmd: i32, tp: Option<usize>, sid: i32) {
        let body = self.ai(me).body;
        let unhold = |ctx: &mut Self| {
            if ctx.ai(me).strategy_cmd == 6 {
                ctx.change_strategy_cmd(me, 3);
            }
        };
        match cmd {
            7..=10 | 1..=4 => {
                let n = (if cmd >= 7 { cmd - 7 } else { cmd - 1 }) as i16;
                if cmd >= 7 {
                    self.ai_mut(me).strategy_cmd = n;
                }
                unhold(self);
                self.change_strategy(me, n);
                if n == 3 {
                    let a = self.ai_mut(me);
                    a.skill_mask = 4;
                    a.self_flag = true;
                } else {
                    self.ai_mut(me).skill_mask = 0;
                }
            }
            0 | 13 => {
                let sc = self.ai(me).strategy_cmd;
                if sc == 1 || sc == 4 {
                    self.change_strategy(me, sc);
                } else {
                    self.change_strategy_cmd(me, 0);
                }
                self.ai_mut(me).skill_mask = if cmd == 0 { 3 } else { 1 };
            }
            14 => {
                let (sc, st) = (self.ai(me).strategy_cmd, self.ai(me).strategy);
                if matches!(sc, 0 | 2) {
                    self.change_strategy_cmd(me, if st == 3 { 6 } else { 0 });
                } else if sc == 3 || st == 3 {
                    self.change_strategy_cmd(me, 6);
                }
                self.ai_mut(me).skill_mask = 2;
            }
            15..=18 => {
                unhold(self);
                let a = self.ai_mut(me);
                a.skill_mask = [0, 4, 8, 16][(cmd - 15) as usize];
                if cmd == 16 || cmd == 18 {
                    a.self_flag = false;
                }
            }
            11 => {
                unhold(self);
                self.change_strategy(me, 4);
                if self.scene.chars[body].spc().is_some_and(|p| p.job == 5) {
                    self.ai_mut(me).skill_mask = 3;
                }
            }
            5 | 12 | 19 => {}
            _ => {
                self.change_strategy(me, 0);
                self.ai_mut(me).skill_mask = 0;
            }
        }
        let a = self.ai_mut(me);
        a.chat_cmd_new = cmd as i16;
        a.chat_cmd_skill = sid as i16;
        a.target_ccmd = tp;
        a.chat_cmd_flag = -2;
    }

    /// The target an attack command (11) follows: `targetCCmd` on
    /// Infection, from Mutation on the copy taken at the command's start.
    fn attack_cmd_target(&self, me: usize) -> Option<usize> {
        let a = self.ai(me);
        if self.t.volume == Volume::Inf { a.target_ccmd } else { a.cmd_attack_target }
    }

    /// A command refused: the member goes back to its standing strategy
    /// (`strategyCMD`, or with `current` the strategy in force: 3 keeps to
    /// itself and heals, anything else allows nothing), the command is
    /// dropped and `chatCmdFlag` becomes -1 (0 with `current`).
    ///
    /// From Mutation on the standing strategy that heals is a hold (6),
    /// which goes back to 3.
    fn refuse(&mut self, me: usize, current: bool) -> i32 {
        let heals = if current {
            self.ai(me).strategy == 3
        } else if self.t.volume != Volume::Inf {
            let hold = self.ai(me).strategy_cmd == 6;
            if hold {
                self.change_strategy_cmd(me, 3);
            }
            hold
        } else {
            self.ai(me).strategy_cmd == 3
        };
        let a = self.ai_mut(me);
        if heals {
            a.skill_mask = 4;
            a.self_flag = true;
        } else {
            a.skill_mask = 0;
        }
        a.chat_cmd = -1;
        a.chat_cmd_flag = if current { 0 } else { -1 };
        0
    }

    /// Accepted: the member's next action counts as its first
    /// (`firstTime`).
    fn accept(&mut self, me: usize) -> i32 {
        self.ai_mut(me).first_time = true;
        1
    }

    /// `ccAI::ChatCommandFulfilCheck` (gcmn 0x00583750): can the member do what
    /// it was told (`chatCmdNew`)? A dead or troubled member says it cannot;
    /// then each command needs its means (the skill and SP, a battle, a heal
    /// the party needs, the ocarina). Returns 1 accepted (`firstTime` set), 0
    /// refused (back to the standing strategy, the command dropped).
    pub fn chat_command_fulfil_check(&mut self, me: usize) -> i32 {
        let body = self.ai(me).body;
        if self.scene.chars[body].cond[cond::DEAD] != 0 {
            if (self.rng.rand() >> 3) & 3 == 0 {
                self.chat(me, Chat::Ghost);
            }
            return self.refuse(me, false);
        }
        if self.troubled(body) {
            self.chat(me, Chat::ConditionMinus);
            return self.refuse(me, false);
        }
        let new = self.ai(me).chat_cmd_new;
        let later = self.t.volume != Volume::Inf;
        match new {
            5 => {
                let sid = i32::from(self.ai(me).chat_cmd_skill);
                let (have, miss) = if !self.check_skill_list(me, sid) {
                    (false, Chat::CanNot)
                } else if !self.cost_ok(body, sid) {
                    (false, Chat::Oom)
                } else {
                    (true, Chat::CanNot)
                };
                if have {
                    return self.accept(me);
                }
                let item = self.search_item_list_by_skill(me, sid);
                if item != -1 {
                    let a = self.ai_mut(me);
                    a.chat_cmd_new = cmd::USE_ITEM;
                    a.chat_cmd_item = item;
                    return 1;
                }
                self.chat(me, miss);
                self.refuse(me, false)
            }
            13 | 14 | 0 => {
                if self.game.in_battle != 1 {
                    self.chat(me, Chat::NoBattleModeDeny);
                    return self.refuse(me, false);
                }
                let ok = match new {
                    13 => self.check_skill_list2(me, 0, 224) != 0,
                    14 => self.check_skill_list2(me, 1, 224) != 0 || self.search_item_list_by_magic_attack(me),
                    _ => true,
                };
                if !ok
                    || (self.ai(me).chat_cmd_new == 0
                        && self.check_skill_list2(me, 0, 224) == 0
                        && self.check_skill_list2(me, 1, 224) == 0
                        && !(later && self.search_item_list_by_magic_attack(me)))
                {
                    self.chat(me, Chat::CanNot);
                    return self.refuse(me, false);
                }
                self.accept(me)
            }
            16 => match self.check_heal_party(me, self.heal_party_arg()) {
                -2 if later && self.game.in_battle == 1 => self.accept(me),
                -2 => {
                    self.chat(me, Chat::Accept);
                    self.refuse(me, true)
                }
                -1 => {
                    self.chat(me, Chat::Oom);
                    self.refuse(me, false)
                }
                0 => {
                    self.chat(me, Chat::CanNot);
                    self.refuse(me, false)
                }
                _ => self.accept(me),
            },
            18 => {
                if self.check_buff_skill_list_num(me) == 0 && !self.search_item_list_by_buff(me) {
                    self.chat(me, Chat::CanNot);
                    return self.refuse(me, false);
                }
                if later && self.plz_check(me, true) == -1 && !self.plz_planned(me, 2, None) {
                    self.chat(me, Chat::Remark(Remark::OnlyBuff, 0));
                    return self.refuse(me, true);
                }
                self.accept(me)
            }
            17 => {
                if self.game.in_battle != 1 {
                    self.chat(me, Chat::NoBattleModeDeny);
                    return self.refuse(me, true);
                }
                if self.check_skill_list2(me, -2, 224) == 0 && self.check_item_list2(me, 7) == 0 {
                    self.chat(me, Chat::CanNot);
                    return self.refuse(me, false);
                }
                if later && self.plz_check(me, false) == -1 && !self.plz_planned(me, -2, None) {
                    self.chat(me, Chat::Remark(Remark::OnlyDebuff, 0));
                    return self.refuse(me, true);
                }
                self.accept(me)
            }
            19 => {
                let g = self.game;
                if g.area != 2 || (g.field_type == 4 && g.field == 0) {
                    self.chat(me, Chat::DisableOcarinaDeny);
                    return self.refuse(me, true);
                }
                if self.check_item_list(me, cmd::OCARINA_ITEM) == 0 {
                    self.chat(me, Chat::NoOcarinaDeny);
                    return self.refuse(me, false);
                }
                if self.can_act(body, 0) && self.scene.chars[body].skill_id == 0 {
                    return 1;
                }
                self.chat(me, Chat::EquipNot(1));
                self.refuse(me, true)
            }
            _ => 1,
        }
    }

    /// `ccAI::ChatCommand` (gcmn 0x00583d60), once a frame: the command state.
    /// A new command (`chatCmdFlag` -2) is checked
    /// ([`Ctx::chat_command_fulfil_check`]) and accepted (`chatCmdFlag` 2), a
    /// waiting one starts once the member can act (1), and a command in force
    /// ends on its own condition; the battle's start resets the strategy to
    /// the party's. The rules are in docs/engine/battle.md ("The decisions").
    pub fn chat_command(&mut self, me: usize) {
        let body = self.ai(me).body;
        if self.crew.spc.get(&body).is_none_or(|s| s.spc_list_num == 0) && self.ai(me).strategy != 0 {
            self.ai_mut(me).strategy = 0;
        }
        let flag = self.ai(me).chat_cmd_flag;
        if flag == 2 {
            if self.can_act(body, 0) {
                self.ai_mut(me).chat_cmd_flag = 1;
            }
        } else if flag == -2 && !(self.ai(me).go_back_flag && !ee::le(self.ai(me).dist_pl, 0x44da_c000)) {
            self.ai_mut(me).go_back_flag = false;
            let mut go = false;
            if self.t.volume == Volume::Inf && self.ai(me).chat_cmd == self.ai(me).chat_cmd_new {
                if self.scene.chars[body].cond[cond::DEAD] != 0 {
                    if (self.rng.rand() >> 3) & 3 == 0 {
                        self.chat(me, Chat::Ghost);
                    }
                } else if self.troubled(body) {
                    self.chat(me, Chat::ConditionMinus);
                } else {
                    self.ai_mut(me).first_time = true;
                    go = true;
                }
                if !go {
                    let a = self.ai_mut(me);
                    a.chat_cmd = -1;
                    a.chat_cmd_flag = -1;
                }
            }
            if go || self.chat_command_fulfil_check(me) != 0 {
                let a = self.ai_mut(me);
                a.chat_cmd = a.chat_cmd_new;
                a.chat_cmd_time = 0;
                a.chat_cmd_flag = 2;
                let c = i32::from(a.chat_cmd);
                self.chat(me, Chat::ChatCmdAccept(c));
                self.ai_mut(me).navi_finish = 1;
                if c == 11 {
                    let a = self.ai_mut(me);
                    a.cmd_attack_target = a.target_ccmd;
                }
                match c {
                    11 if self.scene.chars[body].spc().is_some_and(|p| p.job == 5) => {
                        self.ai_mut(me).skill_mask = 2;
                    }
                    9 | 3 if self.game.area != 0 => self.ai_mut(me).go_back_flag = true,
                    _ => {}
                }
            }
        }
        let c = self.ai(me).chat_cmd;
        if c != -1 {
            let keep = match c {
                1 | 2 | 4 | 17 | 14 | 13 => {
                    if self.game.in_battle == 1 {
                        true
                    } else {
                        if self.ai(me).strategy != 3 {
                            self.ai_mut(me).skill_mask = 0;
                        }
                        false
                    }
                }
                18 | 16 => true,
                9 | 3 => {
                    if self.game.area == 0 {
                        self.ai(me).chat_cmd_time < 900
                    } else {
                        self.game.in_battle == 1
                    }
                }
                5 => {
                    let a = self.ai(me);
                    let ch = &self.scene.chars[body];
                    let tc = self.crew.spc.get(&body).and_then(|s| s.target_char);
                    let running = ch.skill_id == a.chat_cmd_skill && tc == a.target_ccmd;
                    !running && ch.cond[cond::CONFUSION] == 0 && ch.cond[cond::CHARM] == 0
                }
                11 => {
                    let t = self.ai(me).target_ccmd;
                    match t.filter(|&t| self.scene.listed(t)) {
                        Some(t) if self.scene.chars[t].cond[cond::DEAD] == 0 => {
                            self.game.area == 2 || ee::le(self.ai(me).dist_pl, 0x455a_c000)
                        }
                        _ => false,
                    }
                }
                _ => true,
            };
            if !keep {
                self.ai_mut(me).drop_chat_cmd();
            }
            let a = self.ai_mut(me);
            a.chat_cmd_time = a.chat_cmd_time.wrapping_add(1);
        }
        let sbc = self.game.spc_battle_condition;
        if sbc == 3 {
            if self.ai(me).strategy == 3 {
                let a = self.ai_mut(me);
                a.skill_mask = 0;
                a.self_flag = false;
            }
            let n = self.game.party_strategy as i16;
            self.change_strategy_cmd(me, n);
            let a = self.ai_mut(me);
            if a.strategy == 3 {
                a.skill_mask = 4;
                a.self_flag = true;
            } else {
                a.skill_mask &= 0xfc;
                a.self_flag = false;
            }
            if a.chat_cmd != 16 && a.chat_cmd != 18 {
                a.drop_chat_cmd();
            }
        } else if sbc == 1 && self.ai(me).strategy == 3 {
            let a = self.ai_mut(me);
            a.skill_mask = 4;
            a.self_flag = true;
        }
        if self.game.spc_battle_condition == 5 {
            if self.ai(me).battle_flag != 0 {
                self.chat(me, Chat::Victory);
                self.ai_mut(me).battle_flag = 0;
            }
            // From Mutation on the hits are forgotten.
            let a = self.ai_mut(me);
            a.hit_recent = 0;
            a.hit_max = 0;
        }
    }

    /// End a running normal attack of the member's before it acts on a
    /// command.
    fn stop_attack_for_command(&mut self, body: usize) {
        if self.scene.chars[body].skill_id == 1 {
            self.call(Call::SkillRequest { me: body, target: None, sid: 0 });
        }
    }

    /// Take `target` as the member's target (`targetFlag` 1) and face it if
    /// able.
    fn take_target(&mut self, me: usize, t: Option<usize>) {
        let body = self.ai(me).body;
        let a = self.ai_mut(me);
        a.target_flag = 1;
        a.target = t;
        if self.can_act(body, 0) {
            self.crew.spc.entry(body).or_default().target_char = t;
        }
    }

    /// `ccAI::ChatCommandExecute` (gcmn 0x0058ce10): carry out the command in
    /// force (`chatCmdFlag` 1 or 2) for a member in the party, not in mode 6,
    /// troubled, held or heading back: 11 attack, 2 and 8 let go, 4 and 10
    /// fall in, 5 close in for the skill, 99 the item, 19 the Sprite Ocarina.
    /// Returns 0 when carried out or not applicable here (the caller goes on
    /// with its own decisions), 1 while it holds the member.
    pub fn chat_command_execute(&mut self, me: usize) -> i32 {
        let flag = self.ai(me).chat_cmd_flag;
        if flag != 2 && flag != 1 {
            return 0;
        }
        let body = self.ai(me).body;
        if party_flag(&self.scene.chars[body]) != 1 || self.ai(me).mode == self.mode_no(6) {
            return 0;
        }
        let c = &self.scene.chars[body].cond;
        if c[cond::CONFUSION] != 0
            || c[cond::SLEEP] != 0
            || c[cond::CHARM] != 0
            || c[cond::HOLD] != 0
            || c[cond::PARALYSIS] != 0
        {
            return 0;
        }
        if self.ai(me).go_back_flag {
            return 0;
        }
        let mode = self.ai(me).mode;
        let later = self.t.volume != Volume::Inf;
        match self.ai(me).chat_cmd {
            11 => {
                let t = self.attack_cmd_target(me);
                if self.ai(me).target == t && mode == 3 {
                    return 1;
                }
                self.stop_attack_for_command(body);
                if self.scene.chars[body].skill_id != 0 {
                    return 1;
                }
                self.take_target(me, t);
                self.change_mode(me, 3, 0);
                1
            }
            8 | 2 => {
                let k = self.party.members[0].and_then(|k| self.crew.spc.get(&k)).and_then(|s| s.target_char);
                let Some(k) = k.filter(|&k| self.scene.listed(k)) else { return 1 };
                let kc = &self.scene.chars[k];
                if kc.ty() & 0xe0 == 0 || kc.cond[cond::DEAD] >= 2 || self.ai(me).target == Some(k) {
                    return 1;
                }
                self.stop_attack_for_command(body);
                if self.scene.chars[body].skill_id != 0 {
                    return 1;
                }
                let a = self.ai_mut(me);
                a.target_flag = 0;
                a.target = None;
                1
            }
            10 | 4 => {
                if self.game.in_battle != 1 {
                    return 0;
                }
                self.stop_attack_for_command(body);
                let a = self.ai(me);
                if !(a.follow_sw && a.go_back_flag) && mode != self.mode_no(5) {
                    let a = self.ai_mut(me);
                    a.target_flag = 0;
                    a.follow_sw = false;
                    a.target = None;
                    let s = self.crew.spc.entry(body).or_default();
                    s.move_flag = false;
                    s.run_flag = false;
                }
                if !(mode == 1 || mode == 2 || mode == self.mode_no(5)) {
                    self.change_mode(me, 1, 0);
                }
                0
            }
            // From Mutation on the spells command in battle with a
            // standing hold (strategy 6): stop and hold.
            14 if later => {
                if self.game.in_battle != 1 || self.ai(me).strategy != 6 {
                    return 1;
                }
                self.stop_attack_for_command(body);
                let a = self.ai(me);
                if !(a.follow_sw && a.go_back_flag) && mode != MODE_HOLD {
                    let a = self.ai_mut(me);
                    a.target_flag = 0;
                    a.follow_sw = false;
                    a.target = None;
                    let s = self.crew.spc.entry(body).or_default();
                    s.move_flag = false;
                    s.run_flag = false;
                }
                if mode != MODE_HOLD {
                    self.change_mode(me, MODE_HOLD, 0);
                }
                0
            }
            5 => {
                if mode != self.mode_no(4) {
                    self.stop_attack_for_command(body);
                    if self.scene.chars[body].skill_id != 0 {
                        return 1;
                    }
                    self.change_mode(me, self.mode_no(4), 0);
                    self.ai_mut(me).navi_finish = 1;
                    let t = self.ai(me).target_ccmd;
                    self.take_target(me, t);
                    // From Mutation on the command's target is taken once.
                    if later {
                        self.ai_mut(me).target_ccmd = None;
                    }
                    return 1;
                }
                let t = self.ai(me).target_ccmd;
                if (!later || t.is_some()) && self.ai(me).target != t {
                    self.ai_mut(me).navi_finish = 1;
                    self.take_target(me, t);
                    if later {
                        self.ai_mut(me).target_ccmd = None;
                    }
                }
                if self.ai(me).target.is_some_and(|t| self.scene.listed(t)) {
                    return 1;
                }
                let a = self.ai_mut(me);
                a.navi_finish = 1;
                a.target_flag = 0;
                a.target = None;
                self.drop_target_char(body);
                self.ai_mut(me).drop_chat_cmd();
                self.change_mode(me, 1, 0);
                1
            }
            99 => {
                self.stop_attack_for_command(body);
                if self.scene.chars[body].skill_id != 0 {
                    return 1;
                }
                let (item, t) = (self.ai(me).chat_cmd_item, self.ai(me).target_ccmd);
                if self.use_item(me, item, t) == 0 {
                    return 1;
                }
                self.done_first(me);
                if later {
                    self.ai_mut(me).target_ccmd = None;
                }
                0
            }
            19 => {
                let s = &self.crew.sys;
                if self.crew.ocarina_use_flag != 0
                    || self.game.event_lock == 1
                    || s.search_history(7, 90).is_some()
                    || s.pending(|m| m.name == 7)
                {
                    self.ai_mut(me).drop_chat_cmd();
                    return 1;
                }
                self.stop_attack_for_command(body);
                if self.game.menu_type != -1 || self.scene.chars[body].skill_id != 0 || !self.can_act(body, 0) {
                    return 1;
                }
                self.call(Call::UseItemRequest { me: body, target: body, item: cmd::OCARINA_ITEM, flag: 0 });
                // From Mutation on the last ocarina gets its own remark.
                if self.consume_item_list(me, cmd::OCARINA_ITEM) == 0 && later {
                    self.chat(me, Chat::Remark(Remark::UseLastItem, cmd::OCARINA_ITEM));
                } else {
                    self.chat(me, Chat::UseOcarina);
                }
                // Kite's AI's manualSW (a character without one reads as
                // not manual, as the game's read through its null pointer
                // does in eemu).
                if let Some(p) = self.game.player
                    && self.crew.ais.get(&p).is_none_or(|a| !a.manual_sw)
                {
                    self.call(Call::ManualModeAi { ch: p, ev: 0 });
                }
                let id = self.crew.sys_msg_id(body) as u16;
                self.crew.send(6, id, 0xffff, 0, 1, 0, None);
                self.crew.send(7, id, 0xffff, 0, 50, 0, None);
                self.crew.ocarina_use_flag = i32::from(self.scene.chars[body].id());
                self.ai_mut(me).drop_chat_cmd();
                0
            }
            _ => 1,
        }
    }

    /// `ccAI::ChatCommandDeBuffPlz` (gcmn 0x005857c0) and `ChatCommandBuffPlz`
    /// (0x00585c70): a member allowed debuffs (8) or buffs (16), on its first
    /// action or every 55th (90th) count, able and idle, plans the first entry
    /// of its character's priority list some foe (member) still lacks. A
    /// debuff planned returns 1 at once. From Mutation on see
    /// [`Ctx::chat_command_plz_later`].
    fn chat_command_plz(&mut self, me: usize, buff: bool) -> i32 {
        if self.t.volume != Volume::Inf {
            return self.chat_command_plz_later(me, buff);
        }
        let (bit, every) = if buff { (16, 90) } else { (8, 55) };
        if self.ai(me).skill_mask & bit == 0 {
            return 0;
        }
        if !self.ai(me).first_time && cmod(self.ai(me).count, every) != 0 {
            return 0;
        }
        let body = self.ai(me).body;
        let c = &self.scene.chars[body].cond;
        if c[cond::CONFUSION] != 0
            || c[cond::SLEEP] != 0
            || c[cond::CHARM] != 0
            || c[cond::HOLD] != 0
            || c[cond::PARALYSIS] != 0
            || c[cond::DEAD] != 0
        {
            return 0;
        }
        if self.scene.chars[body].skill_id >= 2 || self.check_schedule(me) != 0 {
            return 0;
        }
        let have = if buff {
            self.check_buff_skill_list_num(me) != 0 || self.search_item_list_by_buff(me)
        } else {
            self.check_debuff_skill_list_num(me) != 0 || self.search_item_list_by_debuff(me)
        };
        if !have {
            return 0;
        }
        let mut done = false;
        for sid in self.plz_tries(me, buff) {
            let r =
                if buff { self.buff_for_unused_fellow(me, sid, 0) } else { self.debuff_for_unused_enemy(me, sid, 0) };
            if r != 0 {
                self.done_first(me);
                done = true;
                break;
            }
        }
        if done && !buff {
            return 1;
        }
        if self.game.in_battle != 1 {
            let c = if buff { 18 } else { 17 };
            let a = self.ai_mut(me);
            if a.strategy == 3 {
                a.self_flag = true;
                a.skill_mask = 4;
            } else {
                a.skill_mask = 0;
            }
            if a.chat_cmd == c {
                a.drop_chat_cmd();
                a.first_time = false;
            }
        }
        i32::from(done)
    }

    /// Mutation's `ChatCommandDeBuffPlz` (gcmn 0x005acaf0) and
    /// `ChatCommandBuffPlz` (0x005ad010): every 35th count, a held member too,
    /// only when free to use a skill or an item; a plan returns 1 at once. Out
    /// of battle, once [`Ctx::plz_check`] finds no one in need, the member goes
    /// back to its strategy, drops the command and says there is nothing to do.
    fn chat_command_plz_later(&mut self, me: usize, buff: bool) -> i32 {
        let bit = if buff { 16 } else { 8 };
        if self.ai(me).skill_mask & bit == 0 {
            return 0;
        }
        if !self.ai(me).first_time && cmod(self.ai(me).count, 35) != 0 {
            return 0;
        }
        let body = self.ai(me).body;
        let c = &self.scene.chars[body].cond;
        if c[cond::CONFUSION] != 0
            || c[cond::SLEEP] != 0
            || c[cond::CHARM] != 0
            || c[cond::PARALYSIS] != 0
            || c[cond::DEAD] != 0
        {
            return 0;
        }
        if self.scene.chars[body].skill_id >= 2 || (!self.can_act(body, 6) && !self.can_act(body, 7)) {
            return 0;
        }
        if self.check_schedule(me) != 0 {
            return 0;
        }
        let have = if buff {
            self.check_buff_skill_list_num(me) != 0 || self.search_item_list_by_buff(me)
        } else {
            self.check_debuff_skill_list_num(me) != 0 || self.search_item_list_by_debuff(me)
        };
        if !have {
            return 0;
        }
        for sid in self.plz_tries(me, buff) {
            let r =
                if buff { self.buff_for_unused_fellow(me, sid, 0) } else { self.debuff_for_unused_enemy(me, sid, 0) };
            if r != 0 {
                self.done_first(me);
                return 1;
            }
        }
        if self.game.in_battle == 1 {
            if buff {
                self.done_first(me);
            }
            return 0;
        }
        if self.plz_check(me, buff) != -1 || (buff && self.plz_planned(me, 2, Some(37))) {
            return 0;
        }
        let c = if buff { 18 } else { 17 };
        let a = self.ai_mut(me);
        if a.strategy == 3 {
            a.self_flag = true;
            a.skill_mask = 4;
        } else {
            a.skill_mask = 0;
        }
        if a.chat_cmd == c {
            a.drop_chat_cmd();
            a.first_time = false;
        }
        let r = if buff { Remark::OnlyBuff } else { Remark::OnlyDebuff };
        self.chat(me, Chat::Remark(r, 0));
        if buff {
            self.done_first(me);
        }
        0
    }

    /// The skills [`Ctx::chat_command_plz`] tries, in order: the rows of
    /// the character's priority list, a skill id as it is, -1 the six
    /// stat skills, -2 the field's element skill and then the other five.
    fn plz_tries(&self, me: usize, buff: bool) -> Vec<i32> {
        let id = self.body_id(me) as usize;
        let t = self.t;
        let row: Vec<i16> = if buff {
            let k = t.buff_table_index.get(id).copied().unwrap_or(0) as usize;
            t.buff_priority.get(k).map_or(vec![], |r| r.to_vec())
        } else {
            let k = t.debuff_table_index.get(id).copied().unwrap_or(0) as usize;
            t.debuff_priority.get(k).map_or(vec![], |r| r.to_vec())
        };
        let (ability, attribute) = if buff {
            (t.buff_skill_ability, t.buff_skill_attribute)
        } else {
            (t.debuff_skill_ability, t.debuff_skill_attribute)
        };
        let mut out = Vec::new();
        for v in row {
            let v = i32::from(v);
            if v >= 0 {
                out.push(v);
            } else if v == -1 {
                out.extend(ability.iter().map(|&s| i32::from(s)));
            } else if v == -2 {
                let f = self.game.field_attr;
                out.push(i32::from(attribute.get(f as usize).copied().unwrap_or(0)));
                out.extend((0..6).filter(|&i| i != f).map(|i| i32::from(attribute[i as usize])));
            }
        }
        out
    }

    /// Mutation's check that a buff (debuff) command has work (gcmn
    /// 0x005c2060, 0x005c1cc0; unnamed): 0 without such skills or items, 1
    /// when [`Ctx::plz_tries`] finds a member (a foe) for one (the planners
    /// asked with check 2), -1 when no one needs any.
    pub fn plz_check(&mut self, me: usize, buff: bool) -> i32 {
        let have = if buff {
            self.check_buff_skill_list_num(me) != 0 || self.search_item_list_by_buff(me)
        } else {
            self.check_debuff_skill_list_num(me) != 0 || self.search_item_list_by_debuff(me)
        };
        if !have {
            return 0;
        }
        for sid in self.plz_tries(me, buff) {
            let r =
                if buff { self.buff_for_unused_fellow(me, sid, 2) } else { self.debuff_for_unused_enemy(me, sid, 2) };
            if r != 0 {
                return 1;
            }
        }
        -1
    }

    /// Mutation's search of the bus for a buff (`ty` 2) or debuff (-2)
    /// already on its way to the member (gcmn 0x005b4020, 0x005b4170,
    /// unnamed): a waiting skill use (kind 6) of that type for it, or an
    /// item use (7) casting one; with `window` (0x005b42c0) also one
    /// delivered (all sixteen for -1, else within `window` frames). (The
    /// game's windowed walk stalls on an item that casts nothing; such an
    /// item is passed over here.)
    fn plz_planned(&self, me: usize, ty: i32, window: Option<i32>) -> bool {
        let (t, id) = (self.t, self.ai(me).sys_msg.id as u16);
        let hit = |m: &SysMsg| {
            m.receiver == id
                && match (m.name as u32) >> 16 {
                    6 => skill_check_type(t, m.param) == ty,
                    7 => {
                        let sk = t.item(m.param).map_or(0, |p| p.skill_id);
                        sk != -1 && skill_check_type(t, sk) == ty
                    }
                    _ => false,
                }
        };
        self.crew.sys.pending(hit) || window.is_some_and(|w| self.crew.sys.search_history_by(w, hit).is_some())
    }

    /// `ccAI::ChatCommandDeBuffPlz` (gcmn 0x005857c0).
    pub fn chat_command_debuff_plz(&mut self, me: usize) -> i32 {
        self.chat_command_plz(me, false)
    }

    /// `ccAI::ChatCommandBuffPlz` (gcmn 0x00585c70).
    pub fn chat_command_buff_plz(&mut self, me: usize) -> i32 {
        self.chat_command_plz(me, true)
    }
}

impl Ctx<'_> {
    /// `ccAI::Brains` (gcmn 0x0057ca00): a member's AI, once a frame: the
    /// distances, the bus (`ReadSysMsg`), `levelCheck`, `ChatCommand`, then the
    /// act by area (`ActInField`, `ActInDungeon`, the runtime's `ActInTown` and
    /// `ManualControl`). Returns what the act returned (0 without one; the game
    /// leaves a stale register there), -1 with no mode or body. The order is in
    /// docs/engine/battle.md ("The decisions").
    pub fn brains(&mut self, me: usize) -> i32 {
        if self.ai(me).mode == 0 {
            return -1;
        }
        let body = self.ai(me).body;
        if self.scene.chars[body].ty() & 0xffff != 1 {
            let kite = self.party.members[0];
            let d = distance_to_target(self.t.volume, self.scene, body, kite);
            self.ai_mut(me).dist_pl = d;
            if let Some(t) = self.ai(me).target {
                let h = direction_of_target(self.scene, body, t);
                self.ai_mut(me).dirc_tg = h;
            }
        }
        if self.ai(me).talk_flag {
            let g_deg = self.ai(me).g_deg;
            self.call(Call::FaceTalk { me: body, g_deg });
            return 0;
        }
        if self.ai(me).sys_msg.id == -1 {
            self.crew.add_entry(me);
        } else {
            self.read_sys_msg(me);
        }
        self.level_check(me);
        if party_flag(&self.scene.chars[body]) == 1 {
            self.chat_command(me);
        } else {
            self.ai_mut(me).skill_mask = 3;
        }
        let dead = self.scene.chars[body].cond[cond::DEAD];
        let act = if dead == 0 {
            if self.ai(me).mode == self.mode_no(6) {
                self.change_mode(me, 1, 0);
            }
            true
        } else if (dead == 4 || dead == 5) && self.crew.spc.get(&body).is_some_and(|s| s.ghost) {
            if self.ai(me).mode != self.mode_no(6) {
                self.change_mode(me, self.mode_no(6), 0);
            }
            true
        } else {
            false
        };
        let mut r = 0;
        if act {
            r = if self.ai(me).manual_sw {
                self.call(Call::ManualControl { me: body })
            } else {
                match self.game.area {
                    2 => self.act_in_dungeon(me),
                    1 => self.act_in_field(me),
                    0 if self.game.field_24 == 13 => self.act_in_field(me),
                    0 => self.call(Call::ActInTown { me: body }),
                    _ => 0,
                }
            };
        }
        self.call(Call::ChatMessageSender { me: body });
        let a = self.ai_mut(me);
        a.count = a.count.wrapping_add(1);
        if a.detour_cnt > 0 {
            a.detour_cnt -= 1;
        }
        r
    }
}

impl Crew {
    /// `ccAISysMsgCheckEntryChar(id)` (gcmn 0x0058cd70): the body of the
    /// AI entered with `id`.
    pub fn entry_char(&self, id: i32) -> Option<usize> {
        self.sys
            .entry
            .iter()
            .flatten()
            .copied()
            .find(|k| self.ais.get(k).is_some_and(|a| i32::from(a.sys_msg.id) == id))
    }
}

impl Ctx<'_> {
    /// The command in hand is done when it is `cmds` and out of battle:
    /// skillMask 0, the command dropped.
    fn done_command(&mut self, me: usize, cmds: &[i16]) {
        if cmds.contains(&self.ai(me).chat_cmd) && self.game.in_battle != 1 {
            let a = self.ai_mut(me);
            a.skill_mask = 0;
            a.drop_chat_cmd();
        }
    }

    /// `ccAI::ReadSysMsg` (gcmn 0x0058aeb0), once a frame: the member reads its
    /// whole queue, front first, and carries each message out (the skill and
    /// item uses of kinds 2-0xb, the broadcasts' answers and idle talk). A
    /// skill out of reach is asked again in 45 frames. Once a message the
    /// member sent itself is read, the rest of the batch is read as its own
    /// too (`of`). Returns the last use's result, 0 without one. The kinds are
    /// in docs/engine/battle.md ("The decisions").
    pub fn read_sys_msg(&mut self, me: usize) -> i32 {
        let body = self.ai(me).body;
        let mut of = false;
        let mut result = 0;
        while self.ai(me).sys_msg.msg_num != 0 {
            let e = &self.ai(me).sys_msg;
            let m = e.msg[e.msg_top as usize];
            let id = e.id;
            if i32::from(m.sender) == i32::from(id) {
                of = true;
            }
            let me_id = id as u16;
            let kind = (m.name as u32 >> 16) as i32;
            let low = m.name & 0xffff;
            let oca = self.crew.ocarina_use_flag != 0;
            match kind {
                0 => {
                    if party_flag(&self.scene.chars[body]) != 0 {
                        if low == 6 {
                            if self.can_act(body, 5) {
                                self.crew.spc.entry(body).or_default().act_num = 1;
                                if !of {
                                    self.crew.send(0x10002, me_id, m.sender, 0, 0, 0, None);
                                }
                            }
                        } else if low == 7 {
                            self.call(Call::TransferOut { ch: body });
                            if !of {
                                self.crew.send(0x10002, me_id, m.sender, 0, 0, 0, None);
                            }
                        }
                    }
                }
                1 if !oca && low < 23 => self.read_broadcast(me, &m, low, of),
                2..=0xb if !oca && self.t.volume != Volume::Inf => {
                    result = self.read_use_later(me, &m, kind, low, result)
                }
                2..=5 if !oca => {
                    let tp = self.crew.entry_char(low);
                    let sid = match kind {
                        2 => self.search_heal_skill_of(me, tp),
                        3 => 178,
                        4 => 179,
                        _ => 180,
                    };
                    result = self.use_skill(me, sid, tp);
                    if result == -1 {
                        self.crew.send(m.name, m.sender, m.receiver, m.priority, 45, 0, None);
                    } else if result == 1 && i32::from(self.ai(me).sys_msg.id) != low {
                        let c = match kind {
                            2 => Chat::HealStart,
                            5 => Chat::ResurrectStart,
                            _ => Chat::CureStart,
                        };
                        self.chat(me, c);
                    }
                    self.done_command(me, &[16]);
                }
                6 if !oca => {
                    result = self.use_skill(me, m.param, m.pointer);
                    if result == -1 {
                        self.crew.send(m.name, m.sender, m.receiver, m.priority, 45, m.param, m.pointer);
                    }
                    self.done_command(me, &[18, 17]);
                }
                7 if !oca => {
                    result = self.use_item(me, m.param, m.pointer);
                    self.done_command(me, &[16]);
                }
                8..=0xb if !oca => {
                    let tp = self.crew.entry_char(low);
                    result = self.use_item(me, m.param, tp);
                    if result == 1 && i32::from(self.ai(me).sys_msg.id) != low {
                        let c = match kind {
                            0xb => Chat::ResurrectStart,
                            0xa | 9 => Chat::CureStart,
                            _ => Chat::HealStart,
                        };
                        self.chat(me, c);
                    }
                    self.done_command(me, &[16]);
                }
                _ => {}
            }
            let e = &mut self.ai_mut(me).sys_msg;
            e.msg_num -= 1;
            if e.msg_top == e.msg_tail {
                e.msg_tail += 1;
                if e.msg_tail >= e.msg_max {
                    e.msg_tail = 0;
                }
            }
            e.msg_top += 1;
            if e.msg_top >= e.msg_max {
                e.msg_top = 0;
            }
        }
        result
    }

    /// Mutation's skill and item uses in [`Ctx::read_sys_msg`] (kinds 2-0xb,
    /// gcmn 0x005b25f0): skipped unless the patient needs it (down for a
    /// revive; else alive, hurt for a heal, or with the condition 178 or 179
    /// cures). Returns the new result, `result` when skipped.
    fn read_use_later(&mut self, me: usize, m: &SysMsg, kind: i32, low: i32, result: i32) -> i32 {
        let tp = if kind == 6 || kind == 7 { m.pointer } else { self.crew.entry_char(low) };
        // A missing patient reads as all zeros, as the game's null does.
        let dead = tp.map_or(0, |c| self.scene.chars[c].cond[cond::DEAD]);
        let full = tp.is_none_or(|c| self.scene.chars[c].hp == self.scene.chars[c].max_hp);
        let heal_type = |ctx: &Self, sid: i32| sid != -1 && ctx.t.skill(sid).map_or(0, |p| p.ty) & 0x40000 != 0;
        let casts = match kind {
            2 | 8 => None,
            3 | 9 => Some(178),
            4 | 0xa => Some(179),
            5 | 0xb => Some(180),
            6 => Some(m.param),
            _ => Some(self.item_skill(m.param)),
        };
        let needed = if casts == Some(180) {
            matches!(dead, 2..=4)
        } else if dead != 0 && dead != 5 {
            false
        } else {
            let heals = match casts {
                None => true,
                Some(sid) => (kind == 6 || kind == 7) && heal_type(self, sid),
            };
            !(heals && full)
                && !(casts == Some(178) && self.check_condition_minus(me, tp, -1) == 0)
                && !(casts == Some(179) && self.check_condition_minus(me, tp, -2) == 0)
        };
        if !needed {
            return result;
        }
        let own = i32::from(self.ai(me).sys_msg.id) == low;
        match kind {
            2..=5 => {
                let sid = match kind {
                    2 => self.search_heal_skill_of(me, tp),
                    3 => 178,
                    4 => 179,
                    _ => 180,
                };
                let r = self.use_skill(me, sid, tp);
                if r == -1 {
                    self.crew.send(m.name, m.sender, m.receiver, m.priority, 45, 0, None);
                } else if r == 1 && !own {
                    let c = match kind {
                        2 => Chat::HealStart,
                        5 => Chat::ResurrectStart,
                        _ => Chat::CureStart,
                    };
                    self.chat(me, c);
                }
                r
            }
            6 => {
                let r = self.use_skill(me, m.param, m.pointer);
                if r == -1 {
                    self.crew.send(m.name, m.sender, m.receiver, m.priority, 45, m.param, m.pointer);
                }
                self.debuff_command_done(me);
                r
            }
            7 => {
                let r = self.use_item(me, m.param, m.pointer);
                self.debuff_command_done(me);
                r
            }
            _ => self.use_item(me, m.param, tp),
        }
    }

    /// Mutation's end of a debuff command (17) out of battle after a skill
    /// or item use: back to the strategy (3 keeps to itself and heals) and
    /// the command dropped.
    fn debuff_command_done(&mut self, me: usize) {
        if self.ai(me).chat_cmd != 17 || self.game.in_battle == 1 {
            return;
        }
        let a = self.ai_mut(me);
        if a.strategy == 3 {
            a.self_flag = true;
            a.skill_mask = 4;
        } else {
            a.skill_mask = 0;
        }
        a.drop_chat_cmd();
    }

    /// The broadcasts (0x1xxxx) of [`Ctx::read_sys_msg`].
    fn read_broadcast(&mut self, me: usize, m: &SysMsg, low: i32, of: bool) {
        let body = self.ai(me).body;
        let me_id = self.ai(me).sys_msg.id as u16;
        let idle = |ctx: &mut Self, base: i32| (cmod(ctx.rng.rand() >> 3, 3) * 11 + base) as u16;
        match low {
            1 => {
                self.crew.send(0x10003, me_id, m.sender, 0, 0, 0, None);
            }
            4 => {
                let mode = self.ai(me).mode;
                if mode == 1 || mode == 2 {
                    if !of {
                        self.change_mode(me, self.mode_no(5), 0);
                        if self.ai(me).navi_finish == 0 {
                            self.ai_mut(me).navi_finish = 1;
                        }
                        self.crew.send(0x10002, me_id, m.sender, 0, 0, 0, None);
                    }
                } else {
                    self.crew.send(0x10003, me_id, m.sender, 0, 0, 0, None);
                }
            }
            9 => {
                if of {
                    self.chat(me, Chat::LevelDown);
                }
            }
            10 => {
                if of {
                    self.chat(me, Chat::LevelUp);
                } else {
                    self.crew.send(0x1000b, me_id, me_id, 0, 15, -1, m.pointer);
                }
            }
            11 => self.chat(me, Chat::GratsLevelUp(m.pointer)),
            12 => {
                if of {
                    self.crew.sys.delete_delay(0x1000d, me_id, me_id);
                    self.crew.send(0x1000d, me_id, me_id, 0, 60, -1, Some(body));
                } else {
                    self.chat(me, Chat::DeadOtherFellow(m.pointer));
                }
            }
            13 => {
                let d = self.scene.chars[body].cond[cond::DEAD];
                if (2..=4).contains(&d) {
                    if (self.rng.rand() >> 3) & 1 != 0 {
                        self.chat(me, Chat::GhostCondition);
                    }
                    self.crew.send(0x1000d, me_id, me_id, 0, 90, -1, Some(body));
                }
            }
            14 => self.chat(me, Chat::QuitPucciguso),
            15 => {
                if !of && self.game.in_battle != 1 {
                    self.chat(me, Chat::OpenTrapBox);
                }
            }
            16 => {
                if !of && self.game.in_battle != 1 {
                    self.chat(me, Chat::OpenTreasureBox);
                }
            }
            17 => {
                if self.game.in_battle != 1 {
                    self.chat(me, Chat::PresentOtherFellow(m.pointer));
                }
            }
            18 => {
                if self.check_condition_minus(me, Some(body), -3) != 0 {
                    if (self.rng.rand() >> 3) & 1 != 0 {
                        self.chat(me, Chat::TreatmentPlz);
                    }
                    self.crew.send(0x10012, m.sender, m.receiver, 0, 150, -1, m.pointer);
                }
            }
            19 => {
                if self.ai(me).go_back_flag {
                    self.chat(me, Chat::WaitPlz);
                    self.crew.send(0x10013, m.sender, m.receiver, 0, 300, -1, m.pointer);
                }
            }
            20 => {
                if self.crew.spc.get(&body).is_some_and(|s| s.move_flag) && self.game.in_battle != 1 {
                    // The game tests goBackFlag against -1, which a 1-bit
                    // field never is.
                    self.chat(me, Chat::WalkingTalk);
                    let d = idle(self, 300);
                    self.crew.send(0x10014, m.sender, m.receiver, 0, d, -1, m.pointer);
                }
            }
            21 => {
                if self.game.area != 0 && self.can_act(body, 1) && self.game.in_battle != 1 {
                    if !self.crew.spc.get(&body).is_some_and(|s| s.ghost) {
                        self.chat(me, Chat::Neutral);
                    }
                    let d = idle(self, 500);
                    self.crew.send(0x10015, m.sender, m.receiver, 0, d, -1, m.pointer);
                }
            }
            22 => {
                let ch = &self.scene.chars[body];
                if i32::from(ch.hp) < cdiv(i32::from(ch.max_hp), 3) && self.check_skill_mask(me) & 4 == 0 {
                    if !self.crew.spc.get(&body).is_some_and(|s| s.ghost) {
                        self.chat(me, Chat::HealPlz);
                    }
                    let d = idle(self, 500);
                    self.crew.send(0x10016, m.sender, m.receiver, 0, d, -1, m.pointer);
                }
            }
            _ => {}
        }
    }
}

/// 280.0f, 150.0f, 260.0f, 50.0f, 3000.0f, 3500.0f: the distances the
/// frame's decisions use.
const F_280: u32 = 0x438c_0000;
const F_150: u32 = 0x4316_0000;
const F_260: u32 = 0x4382_0000;
const F_50: u32 = 0x4248_0000;
const F_3000: u32 = 0x453b_8000;
const F_3500: u32 = 0x455a_c000;
/// 0.75f: the eye height of `CheckEyeLineToTarget`.
const F_EYE_LINE: u32 = 0x3f40_0000;
/// 2000.0f: Mutation's search range under command 14 while holding.
const F_2000: u32 = 0x44fa_0000;
/// 100.0f: Mutation's heal-line rate divisor and `CheckHealParty`'s rate.
const F_100: u32 = 0x42c8_0000;
/// 1900.0f: how near its target a member holding its place stays put.
const F_1900: u32 = 0x44ed_8000;

/// From Mutation on, `ccAI.mode` 4: the member holds its place and fights
/// what comes (strategy 6). Infection's modes 4 to 6 are 5 to 7 there.
pub const MODE_HOLD: i32 = 4;

/// Mutation's heal of a healing skill (gcmn MUT 0x005c2380, by an item's
/// skill): Repth 150 and 153 150, 151 and 154 400, 152 and 155 9999, 295
/// 800, else 0.
fn item_heal(sid: i32) -> i32 {
    match sid {
        150 | 153 => 150,
        151 | 154 => 400,
        152 | 155 => 9999,
        295 => 800,
        _ => 0,
    }
}

/// Infection's mode number `m` as the volume numbers it ([`MODE_HOLD`]).
pub fn mode_of(volume: Volume, m: i32) -> i32 {
    if volume != Volume::Inf && m >= 4 { m + 1 } else { m }
}

impl Ctx<'_> {
    /// Point the member at `tgt` (or clear it): `targetChar`, `targetPtr`,
    /// `targetFlag`.
    fn aim(&mut self, me: usize, tgt: Option<usize>) {
        let body = self.ai(me).body;
        self.crew.spc.entry(body).or_default().target_char = tgt;
        let a = self.ai_mut(me);
        a.target = tgt;
        a.target_flag = i8::from(tgt.is_some());
    }

    /// When the heal, cure and revive notes fall due: in 30 frames; from
    /// Mutation on in 30 on the member's first turn (`firstTime`), else in 1.
    pub(crate) fn plan_delay(&self, me: usize) -> u16 {
        if self.t.volume == Volume::Inf || self.ai(me).first_time { 30 } else { 1 }
    }

    /// `CheckHealParty`'s argument from its callers: flag 1, from Mutation
    /// on the rate 100.0.
    pub(crate) fn heal_party_arg(&self) -> i32 {
        if self.t.volume == Volume::Inf { 1 } else { F_100 as i32 }
    }

    /// The volume's number for Infection's mode `m` ([`mode_of`]).
    pub(crate) fn mode_no(&self, m: i32) -> i32 {
        mode_of(self.t.volume, m)
    }

    /// The member's mode as Infection numbers it (Mutation's
    /// [`MODE_HOLD`] is not one of them).
    pub(crate) fn inf_mode(&self, m: i32) -> i32 {
        if self.t.volume != Volume::Inf && m > MODE_HOLD { m - 1 } else { m }
    }

    /// A charmed or confused member in modes 1 and 2 stops following:
    /// halted if it was following Kite; from Mutation on halted if moving,
    /// and no longer heading back.
    fn swayed_stop(&mut self, me: usize, body: usize) {
        if self.t.volume == Volume::Inf {
            if self.ai(me).follow_sw {
                self.halt(body);
            }
            return;
        }
        if self.crew.spc.get(&body).is_some_and(|s| s.move_flag) {
            self.halt(body);
        }
        self.ai_mut(me).go_back_flag = false;
    }

    /// Mutation's mode 4 ([`MODE_HOLD`]; `ActInField` 0x005a3810,
    /// `ActInDungeon` 0x005a5104): a member holding its place (strategy 6)
    /// stays and fights what comes, attacking its target while it is up and
    /// otherwise halting; with none it looks around. Always returns 0.
    fn hold(&mut self, me: usize, dungeon: bool) -> i32 {
        let body = self.ai(me).body;
        let c = self.scene.chars[body].cond;
        if c[cond::CHARM] == 0 && c[cond::CONFUSION] == 0 {
            if party_flag(&self.scene.chars[body]) == 1 && self.ai(me).strategy != 6 {
                self.change_mode(me, 3, 0);
                return 0;
            }
            if self.ai(me).go_back_flag {
                self.change_mode(me, 1, 0);
                return 0;
            }
        }
        let moving = |ctx: &Self| ctx.crew.spc.get(&body).is_some_and(|s| s.move_flag);
        if self.ai(me).target_flag != 0 {
            let own = self.ai(me).target == Some(body);
            let drop = own || {
                if self.ai(me).atk_target_cnt == 0 {
                    self.reconnoiter(me);
                }
                let t = self.ai(me).target.filter(|&t| self.scene.listed(t));
                match t.filter(|&t| self.scene.chars[t].cond[cond::DEAD] < 2) {
                    Some(t) => {
                        let a = self.ai(me);
                        let go = dungeon
                            || self.scene.chars[t].ty() & 0xc0 != 0
                            || a.strategy == 1
                            || a.strategy == 4
                            || a.chat_cmd == 11
                            || a.chat_cmd == 5
                            || ee::le(self.dist(me, Some(t)), param_of(self.t, a).caution_range);
                        if go {
                            self.attack_target(me, t);
                        }
                        !go
                    }
                    None => self.can_act(body, 0),
                }
            };
            if drop {
                self.ai_mut(me).atk_target_cnt = 0;
                self.aim(me, None);
                if moving(self) {
                    self.halt(body);
                }
            }
        } else if self.scene.chars[body].skill_id < 2 {
            self.ai_mut(me).atk_target_cnt = 0;
            if self.reconnoiter(me) != 0 {
                return 0;
            }
            if moving(self) {
                self.halt(body);
            }
        }
        let a = self.ai_mut(me);
        a.attack_cycle = a.attack_cycle.wrapping_add(1);
        0
    }

    /// [`Ctx::aim`] at `tgt`, not following Kite, the navigation reset.
    fn engage(&mut self, me: usize, tgt: Option<usize>) {
        self.aim(me, tgt);
        let a = self.ai_mut(me);
        a.follow_sw = false;
        a.navi_finish = 1;
    }

    /// `ChangeMode(n, 0)` if not in `n` already; 1 if it changed.
    fn enter(&mut self, me: usize, n: i32) -> i32 {
        if self.ai(me).mode != n {
            self.change_mode(me, n, 0);
            1
        } else {
            0
        }
    }

    fn dist(&self, me: usize, t: Option<usize>) -> u32 {
        distance_to_target(self.t.volume, self.scene, self.ai(me).body, t)
    }

    /// `ccAI::Reconnoiter` (gcmn 0x00592aa0): choose the member's target and
    /// mode by strategy (0 and 3 the nearest foe, 1 Kite's target, 2 a foe
    /// while Kite fights, 4 the commanded target), then go for it (mode 3),
    /// stay wary (2) or stand down (1) by `territory` and `cautionRange`.
    /// Returns 1 when the mode changed. The rules, and Mutation's, are in
    /// docs/engine/battle.md ("The decisions").
    pub fn reconnoiter(&mut self, me: usize) -> i32 {
        let body = self.ai(me).body;
        let mut ret = 0;
        let bc = &self.scene.chars[body].cond;
        let swayed = bc[cond::CHARM] != 0 || bc[cond::CONFUSION] != 0;
        let later = self.t.volume != Volume::Inf;
        // From Mutation on a charmed or confused member looks for a foe
        // whatever else holds it back.
        let tgt = if party_flag(&self.scene.chars[body]) != 1 || (later && swayed) {
            self.search_target(me, 224, 0)
        } else if self.crew.ocarina_use_flag != 0 || self.ai(me).go_back_flag || self.game.pg_ride_flag == 1 {
            None
        } else {
            match self.ai(me).strategy {
                s if s == 0 || swayed || s == 3 || (later && s == 6) => self.search_target(me, 224, 0),
                1 => {
                    let k = self.party.members[0].and_then(|k| self.crew.spc.get(&k)).and_then(|s| s.target_char);
                    let ok = k.filter(|&t| {
                        let c = &self.scene.chars[t];
                        self.scene.listed(t) && c.ty() as u32 & 0x0700_000f == 0 && matches!(c.cond[cond::DEAD], 0 | 1)
                    });
                    match ok {
                        Some(t) => Some(t),
                        None => self.search_target(me, 224, 0),
                    }
                }
                2 => {
                    let mode = self.ai(me).mode;
                    if mode == 1 || mode == 3 {
                        let k = self.party.members[0];
                        let attacking = k.is_some_and(|k| self.can_act(k, 4));
                        let still = k.and_then(|k| self.crew.spc.get(&k)).map_or(0, |s| s.stop_cnt);
                        if attacking || still < 15 { None } else { self.search_target(me, 224, 0) }
                    } else {
                        self.search_target(me, 224, 0)
                    }
                }
                4 => {
                    let t = self.attack_cmd_target(me);
                    if !(self.ai(me).target == t && self.ai(me).mode == 3) {
                        self.stop_attack_for_command(body);
                        if self.scene.chars[body].skill_id == 0 {
                            self.take_target(me, t);
                            self.ai_mut(me).follow_sw = false;
                            self.change_mode(me, 3, 0);
                        }
                    }
                    self.ai(me).target
                }
                _ => None,
            }
        };
        let Some(tgt) = tgt else {
            let mode = self.ai(me).mode;
            // From Mutation on a member holding its place stays too.
            let held = self.t.volume != Volume::Inf && mode == MODE_HOLD;
            if mode == 1 || mode == self.mode_no(5) || held || !self.can_act(body, 0) {
                return ret;
            }
            self.engage(me, None);
            return self.enter(me, 1);
        };
        if !self.can_act(body, 0) {
            return 0;
        }
        let bc = &self.scene.chars[body].cond;
        if bc[cond::CHARM] != 0 || bc[cond::CONFUSION] != 0 || party_flag(&self.scene.chars[body]) == 0 {
            self.engage(me, Some(tgt));
            let d = self.dist(me, Some(tgt));
            self.ai_mut(me).dist_tg = d;
            return self.enter(me, 3);
        }
        let s = self.ai(me).strategy;
        if s == 1 {
            self.engage(me, Some(tgt));
            let d = self.dist(me, Some(tgt));
            self.ai_mut(me).dist_tg = d;
            ret = self.enter(me, 3);
            if ret != 0 {
                self.chat(me, Chat::AttackTarget(None));
            }
            return ret;
        }
        if s == 3 || s == 4 || (later && s == 6) {
            self.aim(me, Some(tgt));
            let d = self.dist(me, Some(tgt));
            self.ai_mut(me).dist_tg = d;
            return ret;
        }
        let prm = param_of(self.t, self.ai(me));
        let boss = self.scene.chars[tgt].ty() & 0xc0 != 0;
        if ee::le(self.dist(me, Some(tgt)), prm.territory) {
            ret = self.enter(me, 3);
            if ret != 0 {
                self.chat(me, Chat::AttackTarget(Some(tgt)));
            }
            self.engage(me, Some(tgt));
            let d = self.dist(me, Some(tgt));
            self.ai_mut(me).dist_tg = d;
            return ret;
        }
        if self.ai(me).mode == 2 && !boss && !ee::le(self.dist(me, Some(tgt)), prm.caution_range) {
            self.engage(me, None);
            return self.enter(me, 1);
        }
        self.engage(me, Some(tgt));
        let d = self.dist(me, Some(tgt));
        self.ai_mut(me).dist_tg = d;
        if boss {
            ret = self.enter(me, 3);
            if ret != 0 {
                self.chat(me, Chat::AttackTarget(None));
            }
            return ret;
        }
        self.enter(me, 2)
    }

    /// `ccAI::CheckEyeLineToTarget(tp)` (gcmn 0x005976c0): the line of sight
    /// from the body's `pos` raised by 0.75 of its height to `tp`'s `pos` at
    /// that same height (`ccHitCheckLM(from, to, 8)`, float bits).
    pub fn check_eye_line_to_target(&mut self, me: usize, tp: usize) -> u32 {
        let body = self.ai(me).body;
        let mut from = self.scene.chars[body].pos;
        let mut to = self.scene.chars[tp].pos;
        from[2] = ee::add(from[2], ee::mul(F_EYE_LINE, self.scene.chars[body].base().height));
        to[2] = from[2];
        self.call(Call::HitCheckLm { from, to, mask: 8 }) as u32
    }

    fn kite_dist(&self, me: usize) -> u32 {
        self.dist(me, self.party.members[0])
    }

    /// Following Kite by distance alone (a field): `followSW` set beyond 280.
    fn far_from_kite(&mut self, me: usize) {
        if !ee::lt(self.kite_dist(me), F_280) {
            self.ai_mut(me).follow_sw = true;
        }
    }

    /// The dungeon's test (only while not following): beyond 280 from Kite,
    /// out of sight or with a route to him.
    fn eye_line(&mut self, me: usize) {
        if self.ai(me).follow_sw || ee::lt(self.kite_dist(me), F_280) {
            return;
        }
        let Some(k) = self.party.members[0] else { return };
        if ee::cmp(self.check_eye_line_to_target(me, k), 0) == std::cmp::Ordering::Equal {
            self.ai_mut(me).follow_sw = true;
            return;
        }
        let body = self.ai(me).body;
        let (from, to) = (self.scene.chars[body].pos, self.scene.chars[k].pos);
        if self.call(Call::PathFinding { me: body, from, to }) != 0 {
            self.ai_mut(me).follow_sw = true;
        }
    }

    /// The dungeon's way after Kite: straight once the navigation has
    /// arrived, else along the route, arriving within 150 or (every 90th
    /// count) when the route's goal is where he stands.
    fn follow_kite_dungeon(&mut self, me: usize) {
        let body = self.ai(me).body;
        if self.ai(me).navi_finish != 0 {
            self.call(Call::FollowPlayer { me: body });
            return;
        }
        self.call(Call::FollowBeacon { me: body });
        if ee::le(self.kite_dist(me), F_150) {
            self.ai_mut(me).navi_finish = 1;
        } else if cmod(self.ai(me).count, 90) == 0
            && let Some(k) = self.party.members[0]
        {
            let pos = self.scene.chars[k].pos;
            if self.call(Call::GoalBeacon { me: body, pos }) != 0 {
                self.ai_mut(me).navi_finish = 1;
            }
        }
    }

    /// Stop moving (`moveFlag`, `runFlag`).
    fn halt(&mut self, body: usize) {
        let s = self.crew.spc.entry(body).or_default();
        s.move_flag = false;
        s.run_flag = false;
    }

    /// In battle, a member following Kite stops once near him (280), near
    /// its target (`cautionRange`) or told to stand (`stopFlag`).
    fn trim_follow(&mut self, me: usize, need_not_back: bool) {
        let a = self.ai(me);
        if !a.follow_sw || (need_not_back && a.go_back_flag) {
            return;
        }
        let body = a.body;
        if ee::lt(a.dist_pl, F_280) {
            self.ai_mut(me).follow_sw = false;
        }
        let caution = param_of(self.t, self.ai(me)).caution_range;
        if ee::lt(self.dist(me, self.ai(me).target), caution) {
            self.ai_mut(me).follow_sw = false;
        }
        if self.crew.spc.get(&body).is_some_and(|s| s.stop_flag) {
            self.ai_mut(me).follow_sw = false;
        }
        if !self.ai(me).follow_sw {
            self.halt(body);
        }
    }

    /// The field's turn back to Kite from beyond 3500 (no boss fight).
    fn go_back_check(&mut self, me: usize) {
        let a = self.ai(me);
        if a.go_back_flag || self.check_boss_entry(0).is_some() || self.game.area != 1 || ee::le(a.dist_pl, F_3500) {
            return;
        }
        let a = self.ai_mut(me);
        a.follow_sw = true;
        a.go_back_flag = true;
        let id = a.sys_msg.id as u16;
        let body = a.body;
        self.crew.sys.delete_delay(0x10013, id, id);
        self.crew.send(0x10013, id, id, 0, 40, -1, Some(body));
    }

    /// The walking talk out of battle: with no foe within 3000, while Kite
    /// and the member walk, on the member's own beat (`walkRunCnt` = its
    /// slot x 45 + 80) the member queues 0x10014 to itself in 0, 11, 22 or
    /// 33 frames.
    fn walk_talk(&mut self, me: usize) {
        if self.count_target_in_area(me, 224, F_3000) != 0 {
            return;
        }
        let body = self.ai(me).body;
        let kite_walks = self.game.player.and_then(|p| self.crew.spc.get(&p)).is_some_and(|s| s.move_flag);
        let spc = self.crew.spc.get(&body).copied().unwrap_or_default();
        if !kite_walks || !spc.move_flag {
            return;
        }
        let k = self.party.slot_of(i32::from(self.scene.chars[body].id()));
        if spc.walk_run_cnt != k.wrapping_mul(45).wrapping_add(80) {
            return;
        }
        let id = self.ai(me).sys_msg.id as u16;
        self.crew.sys.delete_delay(0x10014, id, id);
        let q = cmod(self.rng.rand() >> 3, 4);
        self.crew.send(0x10014, id, id, 0, (q * 11) as u16, -1, Some(body));
    }

    /// `ccAI::ActInField` (gcmn 0x0057cd00).
    pub fn act_in_field(&mut self, me: usize) -> i32 {
        self.act_in(me, false)
    }

    /// `ccAI::ActInDungeon` (gcmn 0x0057e0c0).
    pub fn act_in_dungeon(&mut self, me: usize) -> i32 {
        self.act_in(me, true)
    }

    /// `ccAI::ActInField` (gcmn 0x0057cd00) and `ActInDungeon` (0x0057e0c0),
    /// one function with the dungeon's differences switched: the command
    /// ([`Ctx::chat_command_execute`]), nothing more while asleep, held or
    /// paralysed, the heal, debuff and buff duties, then by mode (1 idle, 2
    /// wary, 3 fighting, 4 a skill command, 5 called over, 6 down; the table is
    /// in docs/engine/battle.md, "The decisions"). Always returns 0.
    fn act_in(&mut self, me: usize, dungeon: bool) -> i32 {
        let body = self.ai(me).body;
        let later = self.t.volume != Volume::Inf;
        self.chat_command_execute(me);
        let c = self.scene.chars[body].cond;
        if c[cond::SLEEP] != 0 || c[cond::PARALYSIS] != 0 {
            return 0;
        }
        // From Mutation on a held member still acts unless a boss is in.
        if c[cond::HOLD] != 0 && (!later || self.check_boss_entry(1).is_some()) {
            return 0;
        }
        if self.chat_command_heal_plz(me) != 0
            || self.chat_command_debuff_plz(me) != 0
            || self.chat_command_buff_plz(me) != 0
        {
            return 0;
        }
        let swayed = |ctx: &Self| {
            let c = &ctx.scene.chars[body].cond;
            c[cond::CHARM] != 0 || c[cond::CONFUSION] != 0
        };
        let in_battle = self.game.in_battle;
        let mode = self.ai(me).mode;
        if later && mode == MODE_HOLD {
            return self.hold(me, dungeon);
        }
        match self.inf_mode(mode) {
            1 => {
                if self.reconnoiter(me) != 0 {
                    if swayed(self) {
                        return 0;
                    }
                    let pf = party_flag(&self.scene.chars[body]);
                    if pf == 0 || (pf == 1 && self.ai(me).strategy != 3) {
                        return 0;
                    }
                }
                self.ai_mut(me).atk_target_cnt = 0;
                if party_flag(&self.scene.chars[body]) != 1 {
                    if self.crew.spc.get(&body).is_some_and(|s| s.move_flag) {
                        self.halt(body);
                    }
                    return 0;
                }
                if swayed(self) {
                    self.swayed_stop(me, body);
                    return 0;
                }
                if self.ai(me).strategy == 3 {
                    if in_battle == 0 {
                        if dungeon {
                            self.eye_line(me);
                            self.follow_kite_dungeon(me);
                        } else {
                            self.far_from_kite(me);
                            if self.ai(me).follow_sw {
                                self.call(Call::FollowPlayer { me: body });
                            }
                        }
                        self.walk_talk(me);
                        return 0;
                    }
                    if dungeon {
                        self.trim_follow(me, false);
                        if self.ai(me).follow_sw {
                            self.follow_kite_dungeon(me);
                        }
                    } else {
                        self.go_back_check(me);
                        self.trim_follow(me, true);
                        if self.ai(me).follow_sw || self.ai(me).go_back_flag {
                            self.call(Call::FollowPlayer { me: body });
                        }
                    }
                    return 0;
                }
                if dungeon {
                    self.eye_line(me);
                } else {
                    self.far_from_kite(me);
                }
                if self.ai(me).follow_sw || self.ai(me).go_back_flag {
                    if dungeon {
                        self.follow_kite_dungeon(me);
                    } else {
                        self.call(Call::FollowPlayer { me: body });
                    }
                }
                if in_battle != 0 {
                    return 0;
                }
                self.walk_talk(me);
                0
            }
            2 => {
                if self.reconnoiter(me) != 0 {
                    return 0;
                }
                self.ai_mut(me).atk_target_cnt = 0;
                if party_flag(&self.scene.chars[body]) != 1 {
                    let t = self.ai(me).target;
                    self.call(Call::FollowTargetDirc { me: body, target: t });
                    return 0;
                }
                if swayed(self) {
                    self.swayed_stop(me, body);
                    return 0;
                }
                if self.ai(me).strategy == 3 {
                    if self.scene.chars[body].skill_id == 1 {
                        self.stop_normal_attack(me);
                    }
                    if in_battle == 0 {
                        self.far_from_kite(me);
                        if self.ai(me).follow_sw {
                            self.call(Call::FollowPlayer { me: body });
                        }
                    } else {
                        if !dungeon {
                            self.go_back_check(me);
                        }
                        self.trim_follow(me, true);
                    }
                } else {
                    self.far_from_kite(me);
                }
                if self.ai(me).follow_sw || self.ai(me).go_back_flag {
                    self.call(Call::FollowPlayer { me: body });
                    return 0;
                }
                let t = self.ai(me).target;
                self.call(Call::FollowTargetDirc { me: body, target: t });
                self.halt(body);
                0
            }
            3 => {
                if !swayed(self) {
                    if party_flag(&self.scene.chars[body]) == 1 {
                        match self.ai(me).strategy {
                            3 => {
                                self.change_mode(me, 2, 0);
                                return 0;
                            }
                            2 => {
                                let k = self.party.members[0];
                                if k.is_some_and(|k| self.can_act(k, 4)) && !ee::lt(self.kite_dist(me), F_280) {
                                    self.change_mode(me, 1, 0);
                                    return 0;
                                }
                            }
                            1 => {
                                let kt = self.party.members[0]
                                    .and_then(|k| self.crew.spc.get(&k))
                                    .and_then(|s| s.target_char);
                                if let Some(pt) = kt.filter(|&t| self.scene.listed(t)) {
                                    let c = &self.scene.chars[pt];
                                    if c.ty() & 0xe0 != 0
                                        && c.cond[cond::DEAD] < 2
                                        && self.ai(me).target != Some(pt)
                                        && self.can_act(body, 0)
                                    {
                                        self.aim(me, None);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if self.ai(me).go_back_flag {
                        self.change_mode(me, 1, 0);
                        return 0;
                    }
                }
                if self.ai(me).target_flag != 0 {
                    // From Mutation on a member aiming at itself lets go.
                    let own = later && self.ai(me).target == Some(body);
                    if !own && !dungeon && self.ai(me).atk_target_cnt == 0 {
                        self.reconnoiter(me);
                    }
                    let t = self.ai(me).target.filter(|&t| self.scene.listed(t));
                    let up = t.filter(|&t| self.scene.chars[t].cond[cond::DEAD] < 2);
                    let drop = own
                        || match up {
                            Some(t) => {
                                let a = self.ai(me);
                                let go = dungeon
                                    || self.scene.chars[t].ty() & 0xc0 != 0
                                    || a.strategy == 1
                                    || a.strategy == 4
                                    || a.chat_cmd == 11
                                    || a.chat_cmd == 5
                                    || ee::le(self.dist(me, Some(t)), param_of(self.t, a).caution_range);
                                if go {
                                    let tt = self.ai(me).target.expect("the target just checked");
                                    self.attack_target(me, tt);
                                    false
                                } else {
                                    true
                                }
                            }
                            None => self.can_act(body, 0),
                        };
                    if drop {
                        self.ai_mut(me).atk_target_cnt = 0;
                        self.aim(me, None);
                        // From Mutation on in a dungeon too.
                        if (!dungeon || later) && self.crew.spc.get(&body).is_some_and(|s| s.move_flag) {
                            self.halt(body);
                        }
                        if self.ai(me).strategy == 4 {
                            let n = self.ai(me).strategy_cmd;
                            self.change_strategy_cmd(me, n);
                        }
                    }
                } else if self.scene.chars[body].skill_id < 2 {
                    if !dungeon {
                        self.ai_mut(me).atk_target_cnt = 0;
                    }
                    if self.reconnoiter(me) != 0 {
                        return 0;
                    }
                    if later && self.crew.spc.get(&body).is_some_and(|s| s.move_flag) {
                        self.halt(body);
                    }
                }
                let a = self.ai_mut(me);
                a.attack_cycle = a.attack_cycle.wrapping_add(1);
                0
            }
            4 => {
                // From Mutation on, in a field, a charmed or confused member
                // drops the command and fights.
                if later && !dungeon && swayed(self) {
                    self.ai_mut(me).drop_chat_cmd();
                    self.change_mode(me, 3, 0);
                    return 0;
                }
                if self.ai(me).go_back_flag {
                    self.change_mode(me, 1, 0);
                    return 0;
                }
                let t = self.ai(me).target;
                let sid = i32::from(self.ai(me).chat_cmd_skill);
                // From Mutation on the field checks too.
                if (dungeon || later) && (sid == 0 || !t.is_some_and(|t| self.scene.listed(t))) {
                    self.change_mode(me, 3, 0);
                    return 0;
                }
                let d = self.dist(me, t);
                let reach = ee::add(F_50, d);
                if self.t.skill(sid).is_some_and(|k| crate::skill::range_check(k, reach)) {
                    let r = self.use_skill(me, sid, t);
                    if r == 1 {
                        self.done_first(me);
                        let t = self.ai(me).target;
                        let foe = t.is_some_and(|t| self.scene.chars[t].ty() & 0xe0 != 0);
                        self.change_mode(me, 3, 0);
                        if foe {
                            let id = self.ai(me).sys_msg.id as u16;
                            let t = self.ai(me).target;
                            let sid = i32::from(self.ai(me).chat_cmd_skill);
                            let dmg = t.map_or(0, |t| self.value(body, t, sid));
                            self.crew.send(0x10008, id, id, 0, 0, dmg, t);
                        }
                        return 0;
                    }
                    if r != 0 {
                        return 0;
                    }
                    self.ai_mut(me).navi_finish = 1;
                    if self.can_act(body, 0) {
                        self.aim(me, None);
                    }
                    self.ai_mut(me).drop_chat_cmd();
                    self.change_mode(me, 3, 0);
                    return 0;
                }
                if !dungeon || self.ai(me).navi_finish != 0 {
                    self.call(Call::FollowTarget { me: body, target: t });
                    return 0;
                }
                self.call(Call::FollowBeacon { me: body });
                if ee::le(self.dist(me, t), F_150) {
                    self.ai_mut(me).navi_finish = 1;
                    return 0;
                }
                if cmod(self.ai(me).count, 90) != 0 {
                    return 0;
                }
                let pos = t.map_or([0; 4], |t| self.scene.chars[t].pos);
                if self.call(Call::GoalBeacon { me: body, pos }) != 0 {
                    self.ai_mut(me).navi_finish = 1;
                }
                0
            }
            5 => {
                if swayed(self) {
                    // From Mutation on, in a field, the call is dropped and
                    // the member fights.
                    if later && !dungeon {
                        self.ai_mut(me).drop_chat_cmd();
                        self.change_mode(me, 3, 0);
                    }
                    return 0;
                }
                let d = self.kite_dist(me);
                if !self.ai(me).follow_sw && ee::lt(d, F_150) {
                    self.ai_mut(me).follow_sw = true;
                }
                if self.ai(me).follow_sw {
                    if !dungeon || self.ai(me).navi_finish != 0 {
                        self.call(Call::LeavePlayer { me: body });
                    } else {
                        self.call(Call::FollowBeacon { me: body });
                    }
                    if !self.crew.spc.get(&body).is_some_and(|s| s.move_flag) {
                        self.ai_mut(me).follow_sw = false;
                    }
                    return 0;
                }
                if ee::le(d, F_260) {
                    self.reconnoiter(me);
                    return 0;
                }
                if self.can_act(body, 0) {
                    self.crew.spc.entry(body).or_default().target_char = None;
                }
                let a = self.ai_mut(me);
                a.target = None;
                a.target_flag = 0;
                self.change_mode(me, 1, 0);
                0
            }
            6 => {
                if !dungeon && !self.crew.spc.get(&body).is_some_and(|s| s.ghost) {
                    return 0;
                }
                if party_flag(&self.scene.chars[body]) != 1 {
                    return 0;
                }
                self.far_from_kite(me);
                if self.ai(me).follow_sw {
                    self.call(Call::FollowPlayer { me: body });
                }
                0
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::SkillParam;

    fn ai(body: usize) -> Ai {
        let mut r = || 0;
        Ai::new(body, 0, 0, 150, &mut r)
    }

    fn crew_of(keys: &[usize]) -> Crew {
        let mut c = Crew::default();
        for &k in keys {
            c.ais.insert(k, ai(k));
            c.add_entry(k);
        }
        c
    }

    #[test]
    fn flags_round_trip() {
        let mut a = ai(0);
        for w in [0u32, 0xff00_7fff, 0x0401_6a25, 0x1000_0180, 0x03ff_ffff] {
            a.set_flags(w);
            assert_eq!(a.flags(), w & 0xff01_ffff);
        }
        a.set_chat_cmd_flag(6);
        assert_eq!(a.chat_cmd_flag, -2);
        a.set_battle_flag(3);
        assert_eq!(a.battle_flag, -1);
    }

    #[test]
    fn entries_get_fresh_ids() {
        let mut c = crew_of(&[3, 5]);
        assert_eq!(c.ais[&3].sys_msg.id, 0);
        assert_eq!(c.ais[&5].sys_msg.id, 1);
        c.withdraw(0);
        assert_eq!(c.ais[&3].sys_msg.id, -1);
        assert_eq!(c.sys.entry_num, 1);
        // The next id skips any still in use.
        c.sys.next_id = 1;
        assert_eq!(c.add_entry(3), 2);
    }

    #[test]
    fn delayed_messages_wait_for_their_frame() {
        let mut c = crew_of(&[1, 2]);
        let (a, b) = (c.ais[&1].sys_msg.id as u16, c.ais[&2].sys_msg.id as u16);
        assert_eq!(c.send(0x20000, a, b, 0, 2, 7, Some(4)), -1);
        assert!(c.sys.msg_buff.iter().any(|m| m.state != 0));
        c.tick();
        assert_eq!(c.ais[&2].sys_msg.msg_num, 0);
        c.tick();
        assert_eq!(c.ais[&2].sys_msg.msg_num, 1);
        assert_eq!(c.ais[&2].sys_msg.msg[0].param, 7);
        assert!(c.sys.msg_buff.iter().all(|m| m.state == 0));
        assert_eq!(c.sys.search_history(0x20000, -1), Some(0));
        // Everyone hears a broadcast; an unknown sender is dropped.
        assert_eq!(c.send(0x10009, a, 0xffff, 0, 0, 0, None), 1);
        assert_eq!(c.ais[&1].sys_msg.msg_num, 1);
        assert_eq!(c.send(0x10009, 0x77, 0xffff, 0, 0, 0, None), -1);
    }

    #[test]
    fn history_window_walks_back_from_the_newest() {
        let mut s = AiSystem { time: 100, history_top: 2, ..AiSystem::default() };
        s.msg_history[1] = SysMsg { name: 5, delivery_time: 99, ..SysMsg::default() };
        s.msg_history[0] = SysMsg { name: 6, delivery_time: 40, ..SysMsg::default() };
        s.msg_history[15] = SysMsg { name: 6, delivery_time: 99, ..SysMsg::default() };
        assert_eq!(s.search_history(5, 5), Some(1));
        // The walk stops at the first message older than the window.
        assert_eq!(s.search_history(6, 5), None);
        assert_eq!(s.search_history(6, -1), Some(0));
        s.msg_history[1] = SysMsg { name: 0x10008, param: 30, pointer: Some(2), delivery_time: 99, ..s.msg_history[1] };
        assert_eq!(s.calc_estimate_damage(2, 5), 30);
    }

    #[test]
    fn queues_are_rings_of_ten() {
        let mut e = AiEntry { msg_max: 10, ..AiEntry::default() };
        for i in 0..12 {
            let taken = e.receive_tail(&SysMsg { param: i, ..SysMsg::default() });
            assert_eq!(taken, i < 10);
        }
        assert_eq!(e.msg_tail, 9);
        assert!(!e.receive_top(&SysMsg { param: 99, ..SysMsg::default() }));
        assert_eq!((e.msg_top, e.msg_tail), (9, 8));
        assert_eq!(e.msg[9].param, 99);
    }

    #[test]
    fn check_type_by_bits() {
        let row = |ty| SkillParam { ty, ..SkillParam::default() };
        let t = Tables {
            skills: vec![row(0), row(1), row(1 | 0x400), row(2), row(2 | 0x20000), row(2 | 0x40000)],
            ..Tables::default()
        };
        let v: Vec<i32> = (0..6).map(|s| skill_check_type(&t, s)).collect();
        // Skill 1 is the normal attack whatever its row says.
        assert_eq!(v, [-1, 0, 0, 1, -2, 3]);
        assert_eq!(skill_check_type(&t, -1), -1);
    }

    #[test]
    fn actions_by_kind() {
        let inf = Volume::Inf;
        let mut ch = Char::other(Default::default(), [0; 16]);
        assert!(check_action(&ch, 6, 0, inf) && !check_action(&ch, 7, 0, inf));
        assert!(check_action(&ch, 16, 3, inf) && check_action(&ch, 8, 6, inf) && !check_action(&ch, 8, 3, inf));
        assert!(!check_action(&ch, 9, 2, inf) && check_action(&ch, 10, 2, inf));
        ch.skill_status = 1;
        assert!(!check_action(&ch, 0, 0, inf) && check_action(&ch, 0, 0, Volume::Mut));
        ch.skill_id = 2;
        assert!(!check_action(&ch, 0, 0, Volume::Mut));
        assert!(!check_action(&ch, 15, 3, inf) && check_action(&ch, 0, 5, inf));
    }

    #[test]
    fn division_by_zero_as_the_ee() {
        assert_eq!(ee_div(7, 0), -1);
        assert_eq!(ee_div(-7, 0), 1);
        assert_eq!(ee_div(i32::MIN, -1), i32::MIN);
        assert_eq!(ee_div(-7, 2), -3);
    }

    #[test]
    fn distance_on_the_ground() {
        let one = F_ONE;
        let three = ee::from_int(3);
        let four = ee::from_int(4);
        let (a, b) = ([three, four, ee::from_int(9), one], [0, 0, 0, one]);
        assert_eq!(geom::plane_dist(Volume::Inf, a, b), ee::from_int(5));
        // |(1, 2)| = sqrt(5): newlib rounds up, Outbreak's sqrt.s cuts.
        let (a, b) = ([one, ee::from_int(2), 0, one], [0, 0, 0, one]);
        assert_eq!(geom::plane_dist(Volume::Mut, a, b), 0x400f_1bbd);
        assert_eq!(geom::plane_dist(Volume::Out, a, b), 0x400f_1bbc);
    }
}
