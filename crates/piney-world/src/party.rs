//! The party manager and the SPC registry as the event scripts use them:
//! `ccSpcManager` (gcmn 0x00730340, a `ccSPC`) and `ccPartyManager`
//! (0x00730310, a `ccParty`), and the party instructions of
//! `ccEvent::Execute` (main 0x001a8d20) that act through them - `pc_act`,
//! `pc_mode`, `pc_turn`, `pc_face`, `party_add`, `party_remove`, `menu_ban`
//! and `menu_clear`'s party part.
//!
//! ```text
//! ccSPC          +0x00 registry[5] (ccSPCRegistry, 0x2c each)  +0xdc registryNum
//! ccSPCRegistry  +0x00 id (charTbl row, -1 free)  +0x04 partyFlag  +0x08 bootParam
//!                +0x0c reserveWeapon  +0x0e oldWeapon  +0x10 equip (6 shorts)
//!                +0x1c charPtr  +0x20 body  +0x24 weapon  +0x28 weaponNew
//! ccParty        +0x00 memberChar[3]  +0x0c memberID[3] (-1 empty)  +0x18 num
//! ```
//!
//! A new game starts with Kite alone (`ccSPC::Initialise` 0x0059f5f0:
//! registry 0 is `charTbl` row 0 in the party, `bootParam` 0;
//! `ccParty::InitParty` 0x0059ce00: member 0 Kite, `num` 1). An event's
//! `entry` of a party character adds it to the registry with its `param` as
//! the `bootParam` (`ccRegisterEventMng` main 0x001b6d70, `EntrySpc`
//! 0x0059f740) when the area's files are listed; `ccSPC::Reboot`
//! (0x005a00d0) then builds every registered character (`ccSpcStart[id]`,
//! each constructor applying `SetBootStatus(bootParam)`), gives each its
//! `ccAI` (mode 1, manual when `bootParam` has bit 2) and fills
//! `memberChar` (`SetParty` 0x0059fe80).
//!
//! The characters themselves are the World's (Kite's `Player`, the members
//! the battle's characters: [`crate::town_party`]); [`SpcChars`] lends them
//! by id as [`SpcRef`]s.

use crate::ai::{self, PartyLeave, SpcRef};
use crate::ee::{self, V4};
use crate::hit::Hits;

/// `ccSPC::registry`'s length.
pub const REGISTRY: usize = 5;
/// `ccParty`'s slots.
pub const SLOTS: usize = 3;

/// One `ccSPCRegistry`: the members events and the party use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registry {
    /// +0x00: the `charTbl` row, -1 free.
    pub id: i32,
    /// +0x04: 1 while in the party.
    pub party_flag: i32,
    /// +0x08: the constructor's `SetBootStatus` argument (`pc_mode`,
    /// `entry`'s `param`).
    pub boot_param: i32,
}

impl Registry {
    const FREE: Registry = Registry { id: -1, party_flag: 0, boot_param: 0 };
}

/// `inviteOffsetTbl` (gcmn 0x00654130): by registry slot, the turn (16-bit
/// angle) from the Chaos Gate's heading at which `inviteSpc` stands a new
/// character.
const INVITE_OFFSET: [u16; 5] = [0x0000, 0x2000, 0xe000, 0x4000, 0xa000];
/// And how far from the gate (150.0).
const INVITE_DIST: crate::ee::F = 0x4316_0000;

/// `ccStoreCondition` (0x70 bytes; `storeCondition[18]`, gcmn 0x0072fb00,
/// by `charTbl` row): what `ccStoreSpcConditionOne` (0x0056ccd0) keeps of
/// a party character when a scene ends, for `ccRestoreSpcCondition` to put
/// back when the next one builds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredCondition {
    /// `ccChar.condition` (+0x08: the 16 shorts, +0x28 `speedValue`).
    pub cond: [i16; 16],
    pub speed_value: u32,
    /// Its `ccSpcParam`'s `temp` and `time` (+0x88, +0xa8: the timed buffs
    /// and debuffs, and their frames left).
    pub temp: [i16; 16],
    pub time: [i16; 16],
    /// `HP`, `SP`, `maxHP`, `maxSP` (+0x70..+0x76).
    pub hp: i16,
    pub sp: i16,
    pub max_hp: i16,
    pub max_sp: i16,
    /// `conditionNum` (+0x30).
    pub condition_num: i32,
}

impl StoredCondition {
    /// `ccStoreSpcConditionOne(ch)`.
    pub fn of(ch: &piney_battle::Char) -> Option<StoredCondition> {
        let p = ch.spc()?;
        Some(StoredCondition {
            cond: ch.cond.v,
            speed_value: ch.cond.speed_value,
            temp: p.temp,
            time: p.time,
            hp: ch.hp,
            sp: ch.sp,
            max_hp: ch.max_hp,
            max_sp: ch.max_sp,
            condition_num: ch.condition_num,
        })
    }

    /// `ccRestoreSpcCondition(ch)`'s copy back, into the character and its
    /// `ccSpcParam`.
    pub fn restore(&self, ch: &mut piney_battle::Char) {
        ch.cond.v = self.cond;
        ch.cond.speed_value = self.speed_value;
        if let piney_battle::chara::Body::Spc(p) = &mut ch.body {
            p.temp = self.temp;
            p.time = self.time;
        }
        ch.hp = self.hp;
        ch.sp = self.sp;
        ch.max_hp = self.max_hp;
        ch.max_sp = self.max_sp;
        ch.condition_num = self.condition_num;
    }
}

/// Whether `ccRestoreSpcCondition` (gcmn 0x0056cfc0) puts the stored
/// condition back in the scene `area` (`game.area`, +0x14) entered from
/// `area_prev` (+0x18), of field `field` (+0x24), with `eventStatus[39]`
/// (`saveData` +0x651f): not in a town (area 0: the party arrives whole),
/// not when no area came before (-1: logged in), and not in field 13 while
/// `eventStatus[39]` is 1.
pub fn restores_condition(area: i32, area_prev: i32, field: i32, event_status_39: i8) -> bool {
    if area == 1 && field == 13 && event_status_39 == 1 {
        return false;
    }
    area != 0 && area_prev != -1
}

/// The characters the registry names, lent by id (`registry[i].charPtr`).
pub trait SpcChars {
    /// The character built for registry id `id`, if any.
    fn spc(&mut self, id: i32) -> Option<SpcRef<'_>>;
    /// Its position (+0x40).
    fn pos(&self, id: i32) -> Option<V4>;
}

/// `ccSpcManager` and `ccPartyManager`, with `eventMng.spcMode[3]`
/// (+0x79c: what `menu_ban` found in each slot's `manualSW`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spcs {
    pub registry: [Registry; REGISTRY],
    /// +0xdc `registryNum`.
    pub registry_num: i32,
    /// `ccParty::memberID` by slot, -1 empty.
    pub member_id: [i32; SLOTS],
    /// `ccParty::memberChar` by slot, as the registry id of the character
    /// it points at (set by `SetParty` and `AddMember`, cleared by
    /// `DelMember`); None while null.
    pub member_char: [Option<i32>; SLOTS],
    /// `ccParty::num`.
    pub num: i32,
    pub spc_mode: [bool; SLOTS],
    /// `storeCondition[18]` by `charTbl` row (never stored: None).
    pub store: [Option<StoredCondition>; 18],
}

impl Default for Spcs {
    fn default() -> Self {
        Spcs::new_game()
    }
}

impl Spcs {
    /// `ccSPC::Initialise` (0x0059f5f0) and `ccParty::InitParty`
    /// (0x0059ce00) as `ccSetupNewGame` runs them: Kite (row 0) registered
    /// in the party, `bootParam` 0, alone in slot 0. `memberChar[0]` is
    /// whatever `plw` held then; `SetParty` sets it when the town starts.
    pub fn new_game() -> Self {
        let mut registry = [Registry::FREE; REGISTRY];
        registry[0] = Registry { id: 0, party_flag: 1, boot_param: 0 };
        Spcs {
            registry,
            registry_num: 1,
            member_id: [0, -1, -1],
            member_char: [None; SLOTS],
            num: 1,
            spc_mode: [false; SLOTS],
            store: [None; 18],
        }
    }

    /// `ccStoreSpcCondition()` (gcmn 0x0056cc40), at the start of the next
    /// scene's `ccSetupGameCtrl`: each built character (`id`, the one it
    /// is) stored under its row.
    pub fn store_conditions<'a>(&mut self, built: impl IntoIterator<Item = (i32, &'a piney_battle::Char)>) {
        for (id, ch) in built {
            if let (Some(slot), Some(c)) =
                (usize::try_from(id).ok().and_then(|i| self.store.get_mut(i)), StoredCondition::of(ch))
            {
                *slot = Some(c);
            }
        }
    }

    /// `ccThSpcDelete` (gcmn 0x005a06d0), the delete hook `ccThSpc` sets
    /// on its task: `bootParam` 0 in all five registry slots. The area's
    /// `ccThSpc` is deleted with every other task when the next area's
    /// `ccSetupGameCtrl` starts (`ccDeleteAllThread`), before that area's
    /// event passes, so a `pc_mode` there holds for that area's
    /// `rebootSpcManager` only.
    pub fn th_spc_delete(&mut self) {
        for r in self.registry.iter_mut() {
            r.boot_param = 0;
        }
    }

    /// `ccSPC::CheckSpc(id)` (0x0059f6e0): the registry slot holding `id`
    /// (compared as its low 16 bits), the first.
    pub fn find(&self, id: i32) -> Option<usize> {
        let want = id & 0xffff;
        self.registry.iter().position(|r| r.id == want)
    }

    /// The registry slot of `id` as the event instructions look it up: the
    /// whole word compared (`pc_act`, `pc_mode`, `GetSpc`).
    fn slot_of(&self, id: i32) -> Option<usize> {
        self.registry.iter().position(|r| r.id == id)
    }

    /// `ccSPC::EntrySpc(id)` (0x0059f740): its slot, or the first free one
    /// taken (`registryNum` + 1); -1 when the registry is full.
    pub fn entry_spc(&mut self, id: i32) -> i32 {
        if self.registry_num == REGISTRY as i32 {
            return -1;
        }
        if let Some(i) = self.find(id) {
            return i as i32;
        }
        let Some(i) = self.registry.iter().position(|r| r.id == -1) else { return -1 };
        self.registry_num += 1;
        self.registry[i].id = id & 0xffff;
        i as i32
    }

    /// `ccSPC::DelSpc(i)` (0x0059f850): the slot freed (the character
    /// itself is not deleted here).
    pub fn del_spc(&mut self, i: usize) {
        self.registry[i] = Registry::FREE;
        self.registry_num -= 1;
    }

    /// `ccParty::CheckMemberID(id)` (0x0059cfe0): the slot, or -1.
    pub fn check_member_id(&self, id: i32) -> i32 {
        let want = id & 0xffff;
        self.member_id.iter().position(|&m| m == want).map_or(-1, |s| s as i32)
    }

    /// `ccSPC::SetParty` (0x0059fe80): each slot's `memberChar` the
    /// registered character of its id.
    pub fn set_party(&mut self) {
        for s in 0..SLOTS {
            let id = self.member_id[s];
            if id != -1 && self.slot_of(id).is_some() {
                self.member_char[s] = Some(id);
            }
        }
    }

    /// Party members by slot (`memberID`), -1 empty.
    pub fn party(&self) -> [i32; SLOTS] {
        self.member_id
    }

    /// `checkPartyAnnihilation` (gcmn 0x0059d080): as many members down
    /// (`dead` neither 0 nor 5) as `num`.
    pub fn annihilated(&self, chars: &mut dyn SpcChars) -> bool {
        let mut down = 0;
        for id in self.member_char.into_iter().flatten() {
            if let Some(c) = chars.spc(id)
                && !matches!(*c.dead, 0 | 5)
            {
                down += 1;
            }
        }
        down == self.num
    }

    /// `inviteSpc(id)` (gcmn 0x005a08e0) for a registered character: its
    /// registry `partyFlag` 1, and the character's: leaving (-2) turns
    /// `recallFlag` on, left (-1) comes back (`recallFlag` off, 1), outside
    /// (0) joins (1). False when it is not registered (the town builds
    /// one at the Chaos Gate first: `World::invite_new`) or not built.
    fn invite(&mut self, id: i32, chars: &mut dyn SpcChars) -> bool {
        let Some(i) = self.find(id) else { return false };
        self.registry[i].party_flag = 1;
        let Some(c) = chars.spc(self.registry[i].id) else { return false };
        match *c.party_flag {
            -2 => *c.recall = true,
            -1 => {
                *c.recall = false;
                *c.party_flag = 1;
            }
            0 => *c.party_flag = 1,
            _ => {}
        }
        true
    }

    /// `ccParty::AddMember(id)` (0x0059ce80): into the first empty slot
    /// through `inviteSpc`; the slot, or -1 (no slot, or no character).
    pub fn add_member(&mut self, id: i32, chars: &mut dyn SpcChars) -> i32 {
        for s in 0..SLOTS {
            if self.member_id[s] != -1 {
                continue;
            }
            if !self.invite(id, chars) {
                self.member_char[s] = None;
                return -1;
            }
            self.member_char[s] = self.find(id).map(|i| self.registry[i].id);
            self.member_id[s] = id & 0xffff;
            self.num += 1;
            return s as i32;
        }
        -1
    }

    /// `disbandSpc(id)` (gcmn 0x005a0f50): the registry's `partyFlag` 0;
    /// the character leaving (-2, when it had been recalled, `recallFlag`
    /// off) or left (-1).
    fn disband(&mut self, id: i32, chars: &mut dyn SpcChars) {
        let Some(i) = self.find(id) else { return };
        self.registry[i].party_flag = 0;
        if let Some(c) = chars.spc(self.registry[i].id) {
            if *c.recall {
                *c.party_flag = -2;
                *c.recall = false;
            } else {
                *c.party_flag = -1;
            }
        }
    }

    /// `ccParty::DelMember(slot)` (0x0059cf60): slots 1 and 2 only - the
    /// member disbanded, the slot emptied, `num` - 1.
    pub fn del_member(&mut self, slot: i32, chars: &mut dyn SpcChars) {
        if slot <= 0 || slot as usize >= SLOTS {
            return;
        }
        let s = slot as usize;
        if self.member_id[s] == -1 {
            return;
        }
        self.member_char[s] = None;
        self.disband(self.member_id[s], chars);
        self.member_id[s] = -1;
        self.num -= 1;
    }

    /// `resignParty(id)` (gcmn 0x0059d150): `DelMember` of its slot.
    fn resign(&mut self, id: i32, chars: &mut dyn SpcChars) {
        let slot = self.check_member_id(id);
        if slot > 0 {
            self.del_member(slot, chars);
        }
    }

    /// Remote command 5's party request for character `id`
    /// ([`ai::manual_control`]).
    pub fn leave(&mut self, id: i32, how: PartyLeave, chars: &mut dyn SpcChars) {
        match how {
            PartyLeave::Resign => self.resign(id, chars),
            PartyLeave::Disband => self.disband(id, chars),
        }
        // ManualControl sets the character's partyFlag -2 afterwards.
        if let Some(c) = chars.spc(id) {
            *c.party_flag = -2;
        }
    }

    /// `expulsionSpc(listNum)` (gcmn 0x005a1070) and the fellow task's
    /// delete: out of its party slot; the registry slot freed unless it is
    /// still a member (`partyFlag` 1).
    pub fn expulsion(&mut self, list_num: usize, party_flag: i8, chars: &mut dyn SpcChars) {
        let id = self.registry[list_num].id;
        let slot = self.check_member_id(id);
        if slot != -1 {
            self.del_member(slot, chars);
        }
        if party_flag != 1 {
            self.del_spc(list_num);
        }
    }

    /// `party_add pc` (`ccEvent::Execute` case 77, main 0x001af270, level
    /// 2): `ccParty::AddMember(pc)` (the menu face `SetMenuFace` is the
    /// field UI's).
    pub fn party_add(&mut self, pc: i32, chars: &mut dyn SpcChars) -> i32 {
        self.add_member(pc, chars)
    }

    /// `party_remove pc` (case 78, 0x001af2c8): a character's slot, all
    /// three slots (-3) or slot `-pc`, through `DelMember` (which keeps
    /// slot 0).
    pub fn party_remove(&mut self, pc: i32, chars: &mut dyn SpcChars) {
        if pc >= 0 {
            if let Some(s) = self.member_id.iter().position(|&m| m == pc) {
                self.del_member(s as i32, chars);
            }
        } else if pc == -3 {
            for s in 0..SLOTS as i32 {
                self.del_member(s, chars);
            }
        } else {
            for s in 0..SLOTS as i32 {
                if -pc == s {
                    self.del_member(s, chars);
                    break;
                }
            }
        }
    }

    /// `pc_mode pc param` (case 65, main 0x001adf74, level 2): `bootParam`
    /// of the registered character `pc` (the first slot with that id), of
    /// every party member's (-3), or of slot `-pc`'s member. It takes effect
    /// when the character is next built.
    pub fn pc_mode(&mut self, pc: i32, param: i32) {
        let set = |s: &mut Spcs, id: i32| {
            if let Some(i) = s.slot_of(id) {
                s.registry[i].boot_param = param;
            }
        };
        if pc >= 0 {
            set(self, pc);
        } else if pc == -3 {
            for s in 0..SLOTS {
                let id = self.member_id[s];
                if id >= 0 {
                    set(self, id);
                }
            }
        } else if let Some(&id) = self.member_id.get((-pc) as usize)
            && id >= 0
        {
            set(self, id);
        }
    }

    /// The characters a `pc` operand names through the registry (`pc_act`'s
    /// three branches): registered id `pc`; every slot's member (-3); slot
    /// `-pc`'s member.
    fn named(&self, pc: i32) -> Vec<i32> {
        if pc >= 0 {
            return self.slot_of(pc).map(|i| vec![self.registry[i].id]).unwrap_or_default();
        }
        let ids: Vec<i32> = if pc == -3 {
            self.member_id.to_vec()
        } else {
            self.member_id.get((-pc) as usize).copied().into_iter().collect()
        };
        ids.into_iter().filter(|&id| id >= 0 && self.slot_of(id).is_some()).collect()
    }

    /// `GetSpc(code)` (main 0x001b2bb0): the registered character `code`;
    /// for a negative code the member `memberID[-code]` names (-3 reads
    /// `num` there).
    pub fn get_spc(&self, code: i32) -> Option<i32> {
        let id = if code >= 0 {
            code
        } else {
            let k = (-code) as usize;
            let id = if k < SLOTS {
                self.member_id[k]
            } else if k == SLOTS {
                self.num
            } else {
                return None;
            };
            if id < 0 {
                return None;
            }
            id
        };
        self.slot_of(id).map(|i| self.registry[i].id)
    }

    /// The character `pc_turn` and `pc_face` act on: `GetSpc(pc)`, or
    /// `memberChar[-pc]` for a slot. -3 reads `memberID[0]` as the pointer
    /// (Kite's id 0: nothing), so it names no one.
    pub(crate) fn turned(&self, pc: i32) -> Option<i32> {
        match pc {
            0.. => self.get_spc(pc),
            -2 | -1 => self.member_char[(-pc) as usize],
            _ => None,
        }
    }
}

/// `pc_act pc act` (`ccEvent::Execute` case 60, main 0x001ad0c0, level 2),
/// on each character `pc` names (jump tables 0x00355fa0 for `pc` >= 0,
/// 0x00355f70 for -3, 0x00355f40 for -1/-2):
///
/// ```text
/// 0      manual control off
/// 1      pc >= 0 and the character is Kite (base id 0): ManualModeAI(1);
///        otherwise manual control off
/// 2, 8   ManualModeAI(1), SetRemoteCmd(0)
/// 3-7    ManualModeAI(1), SetRemoteCmd(act)
/// ```
///
/// then `noDeathFlag` (+0xe0 bit 7) set for 1, 5 and 8 - 1 and 8 also
/// putting it on the command list - and cleared for the rest. True when
/// some character was acted on.
pub fn pc_act(spcs: &Spcs, chars: &mut dyn SpcChars, hits: &mut Hits, pc: i32, act: i32) -> bool {
    let ids = spcs.named(pc);
    let annihilated = spcs.annihilated(chars);
    let mut any = false;
    for id in ids {
        let Some(mut c) = chars.spc(id) else {
            if pc >= 0 {
                break;
            }
            continue;
        };
        any = true;
        match act {
            0 => set_manual_off(&mut c),
            1 if pc >= 0 && id == 0 => c.manual_mode_ai(1, annihilated, hits),
            1 => set_manual_off(&mut c),
            2 | 8 => {
                c.manual_mode_ai(1, annihilated, hits);
                c.set_remote_cmd(0);
            }
            3..=7 => {
                c.manual_mode_ai(1, annihilated, hits);
                c.set_remote_cmd(act as i16);
            }
            _ => {}
        }
        *c.no_death = matches!(act, 1 | 5 | 8);
        if matches!(act, 1 | 8) {
            *c.listed = true;
        }
    }
    any
}

/// What a remote walk's goal is read from (`pc_walk_*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WalkGoal {
    /// `(10 x, 10 y, 10 z, 1)`.
    Pos(V4),
    /// `10 dist` toward `rot` from where the character stands (w kept).
    Dir { rot: i16, dist: i16 },
    /// A place as it is (a marker's dummy, w 1; a character's position
    /// offset by `rot` and `dist`, its w).
    At(V4),
}

/// `v + 10 dist (sinf(a), -cosf(a))`, `a = DEG2RAD(rot)`: the offset of
/// `pc_walk_dir` and `pc_walk_char`.
pub fn walk_offset(mut v: V4, rot: i16, dist: i16) -> V4 {
    let a = ee::deg2rad(rot);
    let d = ee::mul(0x4120_0000, ee::from_int(i32::from(dist)));
    v[0] = ee::add(v[0], ee::mul(d, ee::sinf(a)));
    v[1] = ee::sub(v[1], ee::mul(d, ee::cosf(a)));
    v
}

/// A remote walk (`pc_walk_pos` case 61, `pc_run_pos` 164, `pc_walk_dir`
/// 62, `pc_walk_marker` 63, `pc_walk_char` 64) on character `id`:
/// `ManualModeAI(1)`, `SetRemoteCmd(cmd)` (1, or 2 for `pc_run_pos`),
/// `SetGoalPos(goal, 0)` (straight, `gPoint` -1); the AI's `ManualControl`
/// walks there ([`crate::ai::manual_control`] for Kite, the battle's for
/// a member). False when `id` has no character.
pub fn pc_walk(
    chars: &mut dyn SpcChars,
    hits: &mut Hits,
    id: i32,
    cmd: i16,
    goal: WalkGoal,
    annihilated: bool,
) -> bool {
    let Some(mut c) = chars.spc(id) else { return false };
    c.manual_mode_ai(1, annihilated, hits);
    c.set_remote_cmd(cmd);
    let v = match goal {
        WalkGoal::Pos(v) | WalkGoal::At(v) => v,
        WalkGoal::Dir { rot, dist } => walk_offset(*c.pos, rot, dist),
    };
    if let Some(ai) = c.ai.as_deref_mut() {
        ai.g_point = -1;
        ai.g_pos = v;
    }
    true
}

/// `pc_command pc on` (case 66, main 0x001ae164): the characters `pc`
/// names onto the command list (`ccEntryCmnd`) or, with `on` 0, off it
/// (`ccDeleteCmnd`). True when someone was named.
pub fn pc_command(spcs: &Spcs, chars: &mut dyn SpcChars, pc: i32, on: bool) -> bool {
    let mut any = false;
    for id in spcs.named(pc) {
        if let Some(c) = chars.spc(id) {
            *c.listed = on;
            any = true;
        }
    }
    any
}

fn set_manual_off(c: &mut SpcRef) {
    if let Some(ai) = c.ai.as_deref_mut() {
        ai.manual_sw = false;
    }
}

/// `pc_turn pc dirc chg` (case 71, main 0x001aed5c, level 2): `chg` 0 turns
/// at once (`ccAI::SetDircZ`); otherwise the AI turns toward `dirc` at
/// `chg` a frame (1 meaning 64) while its remote command is 0. False when
/// `pc` names no built character.
pub fn pc_turn(spcs: &Spcs, chars: &mut dyn SpcChars, pc: i32, dirc: i16, chg: i16) -> bool {
    let Some(id) = spcs.turned(pc) else { return false };
    let Some(mut c) = chars.spc(id) else { return false };
    turn(&mut c, dirc as u16, chg)
}

/// The turn itself: false for a gradual one on a character without its AI
/// yet (the port's Kite before the town's set-up).
fn turn(c: &mut SpcRef, dd: u16, chg: i16) -> bool {
    if chg != 0 {
        let Some(ai) = c.ai.as_deref_mut() else { return false };
        ai.turn_to(dd, chg);
    } else {
        c.set_dirc_z(dd);
    }
    true
}

/// Who `pc_face` turns to (`1 << type`: 4 the party, 0x18 town NPCs, 0x60
/// enemies and objects; types 0 and 1 name no one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceTarget {
    Spc(i32),
    Npc(i32),
    Enemy(i32),
    None,
}

impl FaceTarget {
    pub fn of(ty: i16, code: i16) -> FaceTarget {
        let bit = 1u32.checked_shl(ty as u32).unwrap_or(0);
        let code = i32::from(code);
        if bit & 4 != 0 {
            FaceTarget::Spc(code)
        } else if bit & 0x18 != 0 {
            FaceTarget::Npc(code)
        } else if bit & 0x60 != 0 {
            FaceTarget::Enemy(code)
        } else {
            FaceTarget::None
        }
    }
}

/// `pc_face pc type code chg` (case 72, main 0x001aee30, level 2) with the
/// target's position `to` found: `RAD2DEG(ccGetDirc(pos, to))` taken as
/// `pc_turn` takes its heading.
pub fn pc_face(spcs: &Spcs, chars: &mut dyn SpcChars, pc: i32, to: V4, chg: i16) -> bool {
    let Some(id) = spcs.turned(pc) else { return false };
    let Some(mut c) = chars.spc(id) else { return false };
    let d = crate::event::dirc_to(*c.pos, to);
    turn(&mut c, d as u16, chg)
}

/// `ccEvent::MenuBan` (main 0x001b2460), its party part: each party slot's
/// member noted (`spcMode`: its `manualSW`) and put under manual control at
/// remote command 0 (`ManualModeAI(1)`, `SetRemoteCmd(0)`); then every
/// registered character off the command list (`ccDeleteCmnd`) and its near
/// fade off (`transDist` 0). Its `ccStoreSpcCondition` and each
/// character's `ClearCondition` are the field's
/// (`FieldWorld::menu_ban_party`, `Combat::menu_ban_conditions`);
/// `ccSpcConditionEffectOFF`, which hides the condition effects, is not
/// kept.
pub fn menu_ban(spcs: &mut Spcs, chars: &mut dyn SpcChars, hits: &mut Hits) {
    let annihilated = spcs.annihilated(chars);
    for s in 0..SLOTS {
        let Some(id) = spcs.member_char[s] else { continue };
        let Some(mut c) = chars.spc(id) else { continue };
        spcs.spc_mode[s] = c.manual();
        c.manual_mode_ai(1, annihilated, hits);
        c.set_remote_cmd(0);
    }
    for r in spcs.registry {
        if r.id == -1 {
            continue;
        }
        if let Some(c) = chars.spc(r.id) {
            *c.listed = false;
            *c.trans_dist = false;
        }
    }
}

/// `FountainMenu3`'s hold of the party slots (gcmn 0x00548d88, `on`):
/// each member under manual control at remote command 0
/// (`ManualModeAI(1)`, `SetRemoteCmd(0)`), off the command list, and every
/// one but Kite unseen (`cloak`, +0x110, 0). Its release (0x005498bc,
/// after the conditions are back): each on the list, `skillID` and
/// `skillStatus` 0, manual control off, the others seen again (`cloak` 1)
/// unless down.
pub fn fountain_party(spcs: &mut Spcs, chars: &mut dyn SpcChars, hits: &mut Hits, on: bool) {
    let annihilated = spcs.annihilated(chars);
    for s in 0..SLOTS {
        let Some(id) = spcs.member_char[s] else { continue };
        let Some(mut c) = chars.spc(id) else { continue };
        if on {
            c.manual_mode_ai(1, annihilated, hits);
            c.set_remote_cmd(0);
            *c.listed = false;
            if s != 0 {
                *c.cloak = 0;
            }
        } else {
            *c.listed = true;
            *c.skill_id = 0;
            *c.skill_status = 0;
            set_manual_off(&mut c);
            if s != 0 && *c.dead == 0 {
                *c.cloak = crate::ee::ONE;
            }
        }
    }
}

/// `ccEvent::MenuClr` (main 0x001b25c0), its party part: every registered
/// character back on the command list unless out of sight (act 14), its
/// conditions restored, `transDist` 1, `skillID` and `skillStatus` 0; then in
/// a town (`area` 0) each slot's member back to what `menu_ban` found (manual
/// again through `ManualModeAI(1)`, or manual control off), Kite always off;
/// elsewhere manual control off for all.
pub fn menu_clr(spcs: &mut Spcs, chars: &mut dyn SpcChars, hits: &mut Hits, area: i32) {
    for r in spcs.registry {
        if r.id == -1 {
            continue;
        }
        if let Some(c) = chars.spc(r.id) {
            if *c.act != 14 {
                *c.listed = true;
            }
            *c.trans_dist = true;
            *c.skill_id = 0;
            *c.skill_status = 0;
        }
    }
    let annihilated = spcs.annihilated(chars);
    for s in 0..SLOTS {
        let Some(id) = spcs.member_char[s] else { continue };
        let Some(mut c) = chars.spc(id) else { continue };
        if area == 0 {
            if spcs.spc_mode[s] {
                c.manual_mode_ai(1, annihilated, hits);
            } else {
                set_manual_off(&mut c);
            }
            if s == 0 {
                set_manual_off(&mut c);
            }
        } else {
            set_manual_off(&mut c);
        }
    }
}

/// What the AI's manual control asks of the party, done (remote command 5).
pub fn apply_leave(spcs: &mut Spcs, chars: &mut dyn SpcChars, id: i32, leave: Option<ai::PartyLeave>) {
    if let Some(how) = leave {
        spcs.leave(id, how, chars);
    }
}

/// A 3-bit signed field's value from the registry's word (`partyFlag & 7`).
pub(crate) fn party_flag_bits(w: i32) -> i8 {
    (((w & 7) << 5) as i8) >> 5
}

impl crate::World {
    /// The registry and Kite and the party members as [`SpcChars`]
    /// ([`crate::town_party::TownChars`]) with the town's collision, lent
    /// to `f`; the members' records written back after.
    pub(crate) fn with_party<R>(
        &mut self,
        f: impl FnOnce(&mut Spcs, &mut crate::town_party::TownChars, &mut Hits) -> R,
    ) -> R {
        self.party.with_chars(&mut self.player, &mut self.spcs, &mut self.town.base.hits, f)
    }

    /// `ccSPC::Reboot` (gcmn 0x005a00d0) as the town's set-up runs it
    /// (`rebootSpcManager`, before the fade in): every registered character
    /// built afresh with its `bootParam` - Kite at the start position
    /// ([`crate::player::Player::build`], with his stand-in among the
    /// battle's characters), the others through `ccFellow::Initialize` as
    /// the battle's characters ([`crate::town_party::TownParty::build`]) at
    /// their `StartPos` (a party member beside Kite, one the events
    /// registered outside the party at the origin, where its entry's marker
    /// then puts it) - each with its AI, then `SetParty`.
    /// [`crate::World::place_entries`] runs it first.
    pub(crate) fn reboot(&mut self) {
        if self.rebooted {
            return;
        }
        self.rebooted = true;
        for i in 0..REGISTRY {
            self.build_spc(i);
        }
        self.spcs.set_party();
        self.party.combat.set_party(self.spcs.party());
    }

    /// Reboot's step for registry slot `i`: its character built.
    fn build_spc(&mut self, i: usize) {
        let r = self.spcs.registry[i];
        if r.id == -1 {
            return;
        }
        let spc = piney_data::save::by_id::spc_param(r.id as usize);
        let word = |at: usize| self.save.save.i32(spc + at) as u32;
        let (velocity, height, width) = (word(0xd4), word(0x18), word(0x1c));
        if r.id == 0 {
            let mut p = crate::player::Player::build(
                self.start.0,
                self.start.1,
                velocity,
                width,
                height,
                r.boot_param,
                &mut self.town.base.hits,
            );
            p.party_flag = party_flag_bits(r.party_flag);
            p.list_num = i as i32;
            // ccPlayer::ccPlayer's cycle and Reboot's ccAI (count, the
            // message counters) draw rand() in that order: his stand-in's
            // are made with the town's generator, and his own taken from it.
            if self.party.combat.kite.is_none() {
                self.party.combat.rand = piney_battle::rand::Rand(self.rand.0);
                let k = self.party.combat.add_leader(&self.save.save, r.boot_param, party_flag_bits(r.party_flag));
                self.rand.0 = self.party.combat.rand.0;
                p.cycle = self.party.combat.crew.spc.get(&k).map_or(0, |s| s.cycle);
                if let (Some(ai), Some(a)) = (p.ai.as_mut(), self.party.combat.crew.ais.get(&k)) {
                    ai.count = a.count;
                }
            }
            self.player = p;
            self.party.combat.mirror_leader(&self.player);
            return;
        }
        if self.party.combat.kite.is_none() {
            // The leader's stand-in first: the members place themselves
            // relative to him.
            self.party.combat.add_leader(&self.save.save, 0, 1);
            self.party.combat.mirror_leader(&self.player);
        }
        let Some(file) = self.char_files.get(r.id as usize).cloned() else { return };
        let (pos, dirc) = self.start_of(r.id);
        self.party.build(
            &self.archive,
            &self.save.save,
            r.id,
            &file,
            pos,
            dirc[2],
            r.boot_param,
            party_flag_bits(r.party_flag),
            &mut self.town.base.hits,
            &mut self.rand,
        );
    }

    /// `StartPos[listNum]` as `ccGetStartPositions(1, 2, 3)` (gcmn
    /// 0x0059ff50) leaves it in a town for registered character `id`: a
    /// party member at its slot's place (`WORLD_MAN::SetCharPosition`: the
    /// start + (200, 100), + (-200, 100), + (0, 300), facing as Kite does),
    /// anyone else at the origin facing 0.
    fn start_of(&self, id: i32) -> (crate::ee::V4, crate::ee::V4) {
        let slot = self.spcs.check_member_id(id);
        if slot <= 0 {
            return (crate::ee::VF0, [0; 4]);
        }
        let (pos, dirc) = self.start;
        let starts = crate::field_world::party_starts(crate::area::kind::TOWN, pos, dirc[2]);
        (starts[slot as usize - 1], [0, 0, dirc[2], 0])
    }

    /// The registry and the party the last area left (`ccSpcManager` and
    /// `ccPartyManager` are the game's globals): taken before the town's
    /// set-up builds them (`rebootSpcManager`), with every `bootParam` 0 as
    /// the old area's `ccThSpcDelete` left them.
    pub fn set_spcs(&mut self, mut spcs: Spcs) {
        spcs.th_spc_delete();
        self.spcs = spcs;
    }

    /// `ccEntryEventMng`'s party entries (main 0x001b62e0-0x001b654c): each
    /// event's registered character put at its marker (the origin without
    /// one), `ccAI::SetDircZ(RAD2DEG(rot.z))`, and on the command list when
    /// `param` is 5.
    pub(crate) fn place_spc_entries(&mut self) {
        for (_, code, marker, param) in std::mem::take(&mut self.spc_entries) {
            self.place_spc(code, marker, param);
        }
    }

    fn place_spc(&mut self, code: i16, marker: i16, param: i16) {
        if self.spcs.slot_of(i32::from(code)).is_none() {
            return;
        }
        let (pos, rot) =
            if marker >= 0 { self.marker_bits(marker).unwrap_or((crate::ee::VF0, 0)) } else { (crate::ee::VF0, 0) };
        self.with_party(|_, chars, _| {
            let Some(mut c) = chars.spc(i32::from(code)) else { return };
            *c.pos = pos;
            c.set_dirc_z(crate::ee::rad2deg(rot) as u16);
            if param == 5 {
                *c.listed = true;
            }
        });
        if code != 0 {
            self.party.combat.mirror_leader(&self.player);
        }
    }

    /// An event's `entry` of a party character (types 0-2): registered
    /// (`EntrySpc`, `bootParam = param`: `ccRegisterEventMng`) and placed by
    /// the town's set-up ([`crate::World::place_entries`]); after the
    /// set-up a new party member is built and placed at once (the game
    /// would only at the next area).
    pub(crate) fn entry_spc(&mut self, code: i16, marker: i16, param: i16) -> bool {
        let i = self.spcs.entry_spc(i32::from(code));
        if i < 0 {
            return false;
        }
        self.spcs.registry[i as usize].boot_param = i32::from(param);
        if !self.placed {
            self.spc_entries.push((2, code, marker, param));
            return true;
        }
        if code != 0 && self.party.member(i32::from(code)).is_none() {
            self.build_spc(i as usize);
            self.place_spc(code, marker, param);
            self.party.combat.set_party(self.spcs.party());
        }
        true
    }

    /// The party members' task frames (`ccThFellowNN`, priority 50) through
    /// the battle's machinery ([`crate::combat::town`]): Kite's stand-in
    /// brought up to date, then each member's `ccFellow::Main`; a leaver
    /// its task lets go (`exitFlag`: `expulsionSpc`, the task's delete)
    /// taken off the town.
    pub(crate) fn party_frame(&mut self) {
        let menu = self.menu_view;
        // The Root Town's server: its number (Mac Anu 0 Delta, Dun Loireag
        // 1 Theta).
        let server = i32::from(self.save.save.u8(piney_data::save::offset::LAST_TOWN) as i8);
        let mut x = crate::town_party::TownFrame {
            hits: &mut self.town.base.hits,
            camera: &mut self.camera,
            save: &mut self.save.save,
            rand: &mut self.rand,
            // CheckMenuType(): -1 with no menu open (the town knows whether
            // one is, not which).
            menu_type: if menu.idle { -1 } else { 0 },
            menu_forbid: menu.forbid,
            server,
        };
        self.party.frame(&mut self.player, &mut self.spcs, &mut x);
    }

    /// The registry and the party (`ccSpcManager`, `ccPartyManager`).
    pub fn spcs(&self) -> &Spcs {
        &self.spcs
    }

    /// `ccStoreSpcCondition()` as the next scene's `ccSetupGameCtrl` runs
    /// it: the town's party (built whole: a town restores nothing) stored
    /// for the next area.
    pub fn store_conditions(&mut self) {
        let c = &self.party.combat;
        self.spcs.store_conditions(c.members.iter().map(|&(id, who)| (id, &c.scene.chars[who])));
    }

    /// The party members as the battle's characters (and Kite's
    /// stand-in).
    pub fn town_party(&self) -> &crate::town_party::TownParty {
        &self.party
    }

    /// Party members' `charTbl` rows by slot (`ccParty::memberID`), -1
    /// empty.
    pub fn party(&self) -> [i32; SLOTS] {
        self.spcs.party()
    }

    /// `ccParty::AddMember(code)` (gcmn 0x0059ce80): the slot it joined, or
    /// -1. A member not registered here (called from the party menu by his
    /// address) is first built at the Chaos Gate ([`Self::invite_new`]).
    pub fn party_add(&mut self, code: i32) -> i32 {
        if self.spcs.member_id.contains(&-1) && self.spcs.find(code).is_none() && !self.invite_new(code) {
            return -1;
        }
        self.with_party(|spcs, chars, _| spcs.add_member(code, chars))
    }

    /// `inviteSpc(id)` (gcmn 0x005a08e0) for a character the registry does
    /// not hold: `EntrySpc` (none when the registry is full), the registry's
    /// `partyFlag` 1 and the character built (`ccSpcStart[id]`), standing
    /// at `StartPos[slot]`: the Chaos Gate's position (`ccGetChgatePosDirc`:
    /// the gate's +0x40 and +0x60) moved 150 along the gate's heading plus
    /// `inviteOffsetTbl[slot]`, facing half a turn from the gate. The
    /// angle goes through `fptosi` as radians before `sinf` and `cosf`, so
    /// only whole radians are used; x gains 150 sin, y loses 150 cos.
    fn invite_new(&mut self, code: i32) -> bool {
        use crate::ee::{add, cosf, deg2rad, from_int, mul, rad2deg, sinf, sub, to_int};
        let i = self.spcs.entry_spc(code);
        let Ok(i) = usize::try_from(i) else { return false };
        self.spcs.registry[i].party_flag = 1;
        self.build_spc(i);
        let heading = rad2deg(self.gate.dirc[2]);
        let a = from_int(to_int(deg2rad(heading.wrapping_add(INVITE_OFFSET[i % INVITE_OFFSET.len()] as i16))));
        let mut pos = self.gate.pos;
        pos[0] = add(pos[0], mul(INVITE_DIST, sinf(a)));
        pos[1] = sub(pos[1], mul(INVITE_DIST, cosf(a)));
        pos[3] = crate::ee::ONE;
        let facing = heading.wrapping_add(i16::MIN);
        let id = self.spcs.registry[i].id;
        self.with_party(|_, chars, _| {
            let Some(mut c) = chars.spc(id) else { return };
            *c.pos = pos;
            c.set_dirc_z(facing as u16);
        });
        self.party.combat.mirror_leader(&self.player);
        true
    }

    /// `party_remove code` (`ccParty::DelMember` of its slot; -3 all, -1/-2
    /// a slot).
    pub fn party_remove(&mut self, code: i32) {
        self.with_party(|spcs, chars, _| spcs.party_remove(code, chars));
    }

    /// `pc_mode pc param`: a registered character's `bootParam`, for when
    /// the town's set-up builds it (callable before and after).
    pub fn set_boot_param(&mut self, pc: i16, param: i16) {
        self.spcs.pc_mode(i32::from(pc), i32::from(param));
    }

    /// `ccSpcChar::TransferOut` on party character `code` (0 Kite), as the
    /// Chaos Gate's menus call it for each member leaving (the party's
    /// warp out). False when there is no such character.
    pub fn transfer_out(&mut self, code: i32) -> bool {
        self.with_party(|_, chars, hits| {
            let Some(mut c) = chars.spc(code) else { return false };
            c.transfer_out(hits);
            true
        })
    }

    /// `ccSpcChar::RequestChatCmd(member, cmd, target, skill)` from the
    /// CHAT menu (`ccAI::RequestChatCmd` 0x00583440) on party member `code`:
    /// in a town `ActInTown` reads 9 (follow Kite) and 12 (stop).
    pub fn chat_cmd(&mut self, code: i32, cmd: i32, target: Option<i32>, sid: i32) {
        let Some(m) = self.party.member(code).filter(|&m| self.party.combat.crew.ais.contains_key(&m)) else {
            return;
        };
        let t = target.and_then(|c| self.party.member(c));
        let scene = crate::area::Scene::init();
        let c = &mut self.party.combat;
        c.rand = piney_battle::rand::Rand(self.rand.0);
        c.with_ai(&mut self.town.base.hits, &mut self.camera, &scene, &mut self.save.save, 0, |ctx| {
            ctx.request_chat_cmd(m, cmd, t, sid)
        });
        self.rand.0 = c.rand.0;
        c.shows.clear();
    }

    /// `ccSpcMessagePresentOtherFellow(ch)` (gcmn 0x005a1a70) after party
    /// member `code` was given a present: with three in the party, the
    /// other member (not Kite, not `code`) is sent message 0x10011
    /// (`ccAISysMsgSendP(0x10011, -1, id, id, 0, 30, -1, ch)`, its own
    /// sys-msg id), which its AI answers with
    /// `ChatMessagePresentOtherFellow(ch)`.
    pub fn present_other(&mut self, code: i32) {
        let ids = self.spcs.party();
        if self.spcs.num < 3 {
            return;
        }
        let Some(me) = ids.iter().position(|&i| i == code) else { return };
        let Some(ch) = self.party.member(code) else { return };
        for (slot, &id) in ids.iter().enumerate() {
            if slot == 0 || slot == me {
                continue;
            }
            let Some(m) = self.party.member(id) else { continue };
            let c = &mut self.party.combat;
            let sid = c.crew.sys_msg_id(m) as u16;
            c.crew.send(0x10011, sid, sid, 0, 30, -1, Some(ch));
        }
    }

    /// `member->ai->manualSW = 0` on party member `code`.
    pub fn manual_off(&mut self, code: i32) {
        if let Some(a) = self.party.member(code).and_then(|m| self.party.combat.crew.ais.get_mut(&m)) {
            a.manual_sw = false;
        }
    }

    /// `ccSpcChar::ManualModeAI(member, ev)` (gcmn 0x0059eba0) on party
    /// member `code`.
    pub fn manual_mode_ai(&mut self, code: i32, ev: i32) {
        let annihilated = {
            let mut chars = self.party.read_chars(&mut self.player);
            let a = self.spcs.annihilated(&mut chars);
            self.party.finish(chars.members);
            a
        };
        self.with_party(|_, chars, hits| {
            if let Some(mut c) = chars.members.spc(code) {
                c.manual_mode_ai(ev, annihilated, hits);
            }
        });
    }

    /// `ccAI::SetRemoteCmd(member->ai, cmd)` (gcmn 0x005832e0) on party
    /// member `code`.
    pub fn remote_cmd(&mut self, code: i32, cmd: i32) {
        self.with_party(|_, chars, _| {
            if let Some(mut c) = chars.members.spc(code) {
                c.set_remote_cmd(cmd as i16);
            }
        });
    }

    /// `menu_ban` (true) / `menu_clear` (false): their party part
    /// ([`menu_ban`], [`menu_clr`]; the menus' part is the field UI's).
    pub fn menu_ban_party(&mut self, on: bool) {
        self.with_party(|spcs, chars, hits| {
            if on {
                menu_ban(spcs, chars, hits);
            } else {
                menu_clr(spcs, chars, hits, 0);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_game_and_its_registry() {
        let mut s = Spcs::new_game();
        assert_eq!(s.party(), [0, -1, -1]);
        assert_eq!(s.registry_num, 1);
        // Event 2: entry of Orca (row 2) with param 5, pc_mode -3 6.
        let i = s.entry_spc(2);
        assert_eq!(i, 1);
        s.registry[i as usize].boot_param = 5;
        s.pc_mode(-3, 6);
        assert_eq!((s.registry[0].boot_param, s.registry[1].boot_param), (6, 5));
        assert_eq!(s.entry_spc(2), 1);
        assert_eq!(s.registry_num, 2);
        assert_eq!(s.get_spc(2), Some(2));
        assert_eq!(s.get_spc(-1), None);
    }
}
