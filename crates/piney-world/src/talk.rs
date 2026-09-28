//! Talking and fighting: the command target and the buttons, as
//! `ccThGameCtrl` (gcmn 0x00517800, priority 33: before the camera and the
//! player) runs them in a town, a field and a dungeon.
//!
//! Characters that can be spoken to or acted on put themselves on one of
//! three command lists with `ccEntryCmnd` (0x00519630): the party (base
//! type & 7), enemies (& 0xe0) and everything else (NPCs, merchants,
//! gimmicks), linked through `ccChar::cmndLink` (+0xbc). Each frame:
//!
//! ```text
//! checkPartyAnnihilation (0x0059d080) or compulsionGameOver: the game over
//! ccCtrlRecoveryReq (0x0051a7f0)  the delayed recoveries (RecoveryReqs)
//! ccSortCmnd (0x00518af0)       for every listed character but the leader:
//!                               d = posP - leader.posP (z 0), cmndDist =
//!                               sqrtf(d.d), cmndDirc = atan2f(d.y, d.x);
//!                               those within 3000 into one list by distance
//!                               (cmndSort +0xc0; equal distances keep their
//!                               order)
//! game.inBattle                 unless ccMenu.menu is 66 (Data Drain):
//!                               SetInBattle (main 0x001676a0) of 1 when an
//!                               enemy is within inBattleDist of a party
//!                               member (ccCheckInAreaCmnd 0x0051a000)
//! ghoFlag, menuClrWait          wait
//! ccPlayerMenuCheck (0x0059cd70) the party standing, no skill of the
//!                               player's past the normal attack, not at a
//!                               gate (acts 12, 13); the first 5 frames wait
//! ccSelectTarget (0x00518cc0)   (mode 0 with no target, 2 on a fresh lean of
//!                               the left stick past 64, else 1): the nearest
//!                               in range (ccCheckTargetRange 0x00519240:
//!                               within 60 + both widths, or within 300 + both
//!                               widths and 1.2 rad of straight ahead; 500 /
//!                               0.49 rad in the eye view; in battle 150 or
//!                               60, and 2.0 rad for the wider ring), enemies
//!                               first, then characters, then the party; mode
//!                               2 steps through them (cmndTargetPriNum)
//! ccChangeCmndTarget (0x005198c0)
//! the buttons                   chat, option, personal: a menu; the action
//!                               button (assignPADaction, saveData+0x8404,
//!                               X by default) on the target: ccMenu opens
//!                               with the type the target's base flags choose
//!                               - 22 a walking PC, 24 a weapon, item or
//!                               magic shop, 25 the Recorder, 26 Elf's Haven,
//!                               28 the Chaos Gate ... - and the player's
//!                               +0xe0 bit 0 (pauseSW) is set until the menu
//!                               closes; on an enemy the normal attack,
//!                               ccSkillRequest(plw, target, 1), with
//!                               ccMenu.plAttack and cmndTargetFix set
//! ```
//!
//! [`Targeting::frame`] is the whole loop body; [`Targeting::step`] is the
//! town's call of it. The menus themselves (the message window, the shops)
//! are not this crate's: the port raises a [`TalkRequest`]
//! ([`crate::World::take_talk`]) and waits for [`crate::World::close_menu`].

use crate::ee::{self, F, V4};
use crate::entry::Kind;

/// Base type flags of a character (`ccCharBaseParam::type`).
pub mod flags {
    /// The party (spc characters).
    pub const PARTY: u32 = 0x07;
    pub const ENEMY: u32 = 0xe0;
    pub const PC: u32 = 0x08;
    pub const SYSOPE: u32 = 0x10;
    pub const EQUIP_SHOP: u32 = 0x100;
    pub const FAIRY_SHOP: u32 = 0x200;
    pub const ITEM_SHOP: u32 = 0x400;
    pub const MAGIC_SHOP: u32 = 0x800;
    pub const RECORDER: u32 = 0x1000;
    pub const CHAOS_GATE: u32 = 0x2000;
}

/// `ccSortCmnd`'s reach: 3000.
const SORT_REACH: F = 0x453b_8000;
/// `ccCheckTargetRange`'s constants.
const NEAR: F = 0x4270_0000; // 60
const REACH: F = 0x4396_0000; // 300
const REACH_EYE: F = 0x43fa_0000; // 500
const BATTLE_REACH_OBJ: F = 0x4316_0000; // 150
const BATTLE_REACH_EYE: F = 0x43c8_0000; // 400
const CONE: F = 0x3f99_999a; // 1.2
const BATTLE_CONE: F = 0x4000_0000; // 2.0
const CONE_EYE: F = 0x3efa_e148; // 0.49
const HALF_PI: F = 0x3fc9_0fdb;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
/// `inBattleDist`'s -2.0: a fight whatever the distance.
const ALWAYS: F = 0xc000_0000;

/// `ccChar::condition` (+0x08, a `ccCondition`): the shorts the command
/// target and the buttons read (`ccCondition` +0x00, +0x12, +0x14, +0x16,
/// +0x1c).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cond {
    /// +0x08 `dead`: 0 standing; 2 falling, 3 lying, 4 a ghost, 5 reviving.
    pub dead: i16,
    /// +0x1a `sleep`, +0x1c `confusion`, +0x1e `charm`, +0x24 `paralysis`:
    /// nonzero while it lasts.
    pub sleep: i16,
    pub confusion: i16,
    pub charm: i16,
    pub paralysis: i16,
}

impl Cond {
    /// From `ccCondition`'s sixteen shorts in order (dead 0, sleep 9,
    /// confusion 10, charm 11, paralysis 14).
    pub fn from_shorts(v: &[i16; 16]) -> Cond {
        Cond { dead: v[0], sleep: v[9], confusion: v[10], charm: v[11], paralysis: v[14] }
    }

    /// Asleep, confused, charmed or paralysed: what holds the player's
    /// buttons and bars a party member from the target.
    pub fn held(&self) -> bool {
        self.sleep != 0 || self.confusion != 0 || self.charm != 0 || self.paralysis != 0
    }
}

/// One character on a command list, as the targeting reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cmnd {
    /// Who it is, for the caller (an event's type and code).
    pub kind: Kind,
    pub code: i32,
    /// The base parameters' type flags (+0x08) and width (+0x1c). The
    /// flags choose its list, as `ccEntryCmnd` does: the party's for & 7,
    /// else the enemies' for & 0xe0, else the others'.
    pub flags: u32,
    pub width: F,
    /// The base parameters' id (+0x0c), which `ccCheckTargetTypeId`
    /// matches.
    pub id: i16,
    /// +0x50 `posP`.
    pub pos_p: V4,
    /// Its `ccChar::condition`: `dead` bars anyone from the target; the
    /// party (type & 0x0700000f) is barred while held as well.
    pub cond: Cond,
    /// `ccSpcChar` +0xee `actNum`, read for type bit 4 (a party member):
    /// acts 12-14 (at a gate) bar it from the target.
    pub act: i16,
    /// +0xc4 `cmndDist`, +0xc8 `cmndDirc`, as the last sort left them.
    pub dist: F,
    pub dirc: F,
}

impl Cmnd {
    /// A standing character with no conditions, id 0 and act 0.
    pub fn new(kind: Kind, code: i32, flags: u32, width: F, pos_p: V4) -> Cmnd {
        Cmnd { kind, code, flags, width, id: 0, pos_p, cond: Cond::default(), act: 0, dist: 0, dirc: 0 }
    }

    /// The list `ccEntryCmnd` puts it on: 0 the party's, 1 the enemies',
    /// 2 the others'.
    pub fn list(&self) -> u8 {
        if self.flags & 7 != 0 {
            0
        } else if self.flags & 0xe0 != 0 {
            1
        } else {
            2
        }
    }
}

/// The leader (`ccPartyManager`'s first, Kite) as the targeting reads him.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leader {
    pub pos_p: V4,
    /// +0x60 `dirc` (z the heading).
    pub dirc: V4,
    /// His base width (+0x1c).
    pub width: F,
}

/// The leader as the rest of the task reads him beyond [`Leader`]: the
/// character `ccPartyManager` held when the task started (`$s4`) and the
/// player `plw` (0x00730300), which are both Kite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaderState {
    /// On the party's command list (`ccCheckTarget(plw)`).
    pub listed: bool,
    /// His place on the party's list when listed, among the party's
    /// candidates in order: 0 the head. `ccEntryCmnd` appends, so he heads
    /// it when entered before the members (the party built, `ccSPC::Wakeup`,
    /// `MenuClr`) and follows them when `ccPlayer::AnimCtrl` (0x0059a0c8)
    /// enters him at the end of an arrival. Only `ccCheckInAreaCmnd`'s walk
    /// of the party's list, which skips its head, reads it.
    pub at: usize,
    /// His base type flags and id (7 and 0 for Kite).
    pub flags: u32,
    pub id: i16,
    pub cond: Cond,
    /// +0xee `actNum` (12, 13: leaving or arriving through a gate).
    pub act: i16,
    /// `ccSpcChar::CheckControlMode()` (gcmn 0x0059f550): his AI's (+0x128)
    /// byte 0 bit 0, under the events' manual control.
    pub control: bool,
}

impl Default for LeaderState {
    /// Kite listed and standing.
    fn default() -> LeaderState {
        LeaderState { listed: true, at: 0, flags: 7, id: 0, cond: Cond::default(), act: 0, control: false }
    }
}

/// Who a character is on the command lists, for `ccCheckInAreaCmnd`,
/// which never counts its centre against itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    /// The leader.
    Leader,
    /// The candidate at this index of the frame's.
    Cand(usize),
    /// Someone not on the lists.
    Unlisted,
}

/// A `ccPartyManager` slot as the task reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Member {
    /// +0x50 `posP`.
    pub pos_p: V4,
    /// +0x08 `dead`.
    pub dead: i16,
    pub who: Who,
}

/// `ccPartyManager` (gcmn 0x00730310): three slots, the leader first, and
/// +0x18 how many there are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Party {
    pub slots: [Option<Member>; 3],
    pub num: i32,
}

impl Party {
    /// The leader alone.
    pub fn alone(leader: &Leader, dead: i16) -> Party {
        Party { slots: [Some(Member { pos_p: leader.pos_p, dead, who: Who::Leader }), None, None], num: 1 }
    }
}

/// `checkPartyAnnihilation()` (gcmn 0x0059d080): every one of the
/// `ccPartyManager` count is down (`dead` neither 0 nor 5).
pub fn annihilated(party: &Party) -> bool {
    let down = party.slots.iter().flatten().filter(|m| m.dead != 0 && m.dead != 5).count() as i32;
    down == party.num
}

/// `ccGame`'s battle state: +0x58 `inBattle` (0 none, 1 a fight, 2 a
/// fight just over), +0x5c `inBattleCnt`, +0x60 `inBattleDist`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InBattle {
    pub in_battle: i32,
    pub cnt: i32,
    pub dist: F,
}

impl Default for InBattle {
    /// As `ccGame::ChangeScene` leaves it: 0, 0, 2200.
    fn default() -> InBattle {
        InBattle { in_battle: 0, cnt: 0, dist: InBattle::DIST }
    }
}

impl InBattle {
    /// `inBattleDist` after `ChangeScene`: 2200.
    pub const DIST: F = 0x4509_8000;

    /// `ccGame::SetInBattle(v)` (main 0x001676a0): while `inBattleCnt`
    /// runs it counts down and nothing changes; otherwise a new value
    /// starts it at 60, and 1 going to 0 becomes 2.
    pub fn set(&mut self, v: i32) {
        if self.cnt > 0 {
            self.cnt -= 1;
            return;
        }
        if self.in_battle == v {
            return;
        }
        self.cnt = 60;
        self.in_battle = if self.in_battle == 1 && v == 0 { 2 } else { v };
    }

    /// The event instruction `battle_ready` (0x001b23a8):
    /// `inBattleDist = -1.0`, a fight with any enemy listed.
    pub fn battle_ready(&mut self) {
        self.dist = 0xbf80_0000;
    }
}

/// `ccSortCmnd` over the three lists in order (party, enemies, others):
/// every candidate's `dist` and `dirc`, and the indices of those within
/// 3000 by distance (each placed before the first that is farther).
pub fn sort(leader: &Leader, cands: &mut [Cmnd]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for i in 0..cands.len() {
        let c = &mut cands[i];
        let d = [ee::sub(c.pos_p[0], leader.pos_p[0]), ee::sub(c.pos_p[1], leader.pos_p[1]), 0, ee::ONE];
        c.dist = ee::sqrtf(ee::dot(d, d));
        c.dirc = ee::atan2f(d[1], d[0]);
        if !ee::le(c.dist, SORT_REACH) {
            continue;
        }
        let dist = c.dist;
        let at = out.iter().position(|&k| !ee::le(cands[k].dist, dist)).unwrap_or(out.len());
        out.insert(at, i);
    }
    out
}

/// `ccCheckTargetRange(t)`: 0 out of reach, 1 in reach, 2 in the wider
/// battle reach. `in_battle` is `game.inBattle != 0`.
pub fn check_range(leader: &Leader, c: &Cmnd, eye: bool, in_battle: bool) -> u8 {
    let widths = |r: F| ee::add(ee::add(r, leader.width), c.width);
    // The angle off his heading, from +y (pi/2 ahead), wrapped into
    // (-pi, pi].
    let off = || {
        let mut a = ee::add(HALF_PI, ee::sub(c.dirc, leader.dirc[2]));
        if !ee::le(a, PI) {
            a = ee::sub(a, TWO_PI);
        } else if ee::lt(a, ee::neg(PI)) {
            a = ee::add(a, TWO_PI);
        }
        a
    };
    let within = |a: F, cone: F| ee::le(a, cone) && !ee::lt(a, ee::neg(cone));
    if !eye {
        let mut reach = REACH;
        if in_battle {
            if c.flags & 0x8000 != 0 {
                return 0;
            }
            reach = if c.flags & 0x0784_00ef == 0 { NEAR } else { BATTLE_REACH_OBJ };
        }
        let near = ee::add(ee::add(NEAR, leader.width), c.width);
        if ee::le(c.dist, near) {
            return 1;
        }
        if !ee::le(c.dist, ee::add(c.width, ee::add(reach, leader.width))) {
            return 0;
        }
        let a = off();
        if within(a, CONE) {
            return 1;
        }
        if in_battle && within(a, BATTLE_CONE) {
            return 2;
        }
        return 0;
    }
    let mut reach = REACH_EYE;
    if in_battle {
        if c.flags & 0x8000 != 0 {
            return 0;
        }
        reach = if c.flags & 0x0784_00ef == 0 { NEAR } else { BATTLE_REACH_EYE };
    } else if c.flags & 0x0800_1f00 != 0 {
        reach = REACH;
    }
    if !ee::le(c.dist, widths(reach)) {
        return 0;
    }
    u8::from(within(off(), CONE_EYE))
}

/// What `ccSelectTarget` and `ccCheckTargetRange` read of the game: the
/// eye view (`checkCameraType() == 1`), `game.inBattle != 0` and
/// `game.field` (+0x24).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub eye: bool,
    pub in_battle: bool,
    pub field: i32,
}

/// `ccSelectTarget(mode)` (gcmn 0x00518cc0): the target among the sorted
/// candidates, `pri` the running `cmndTargetPriNum` (mode 2 steps it).
///
/// None while the leader is off the lists, down or held. Anyone down is
/// passed over; the party and the walking PCs (type & 0x0700000f) are
/// passed over in a fight, on fields 1-12, while held, and a party member
/// (bit 4) at a gate (acts 12-14). The rest in reach go into two stack
/// arrays of 8 - ranged 1 and ranged 2 - that lie one after the other, so
/// a ninth in reach overwrites the wider ring's first (kept here).
pub fn select_target(
    leader: &Leader,
    kite: &LeaderState,
    cands: &[Cmnd],
    sorted: &[usize],
    mode: i32,
    pri: &mut i32,
    scope: Scope,
) -> Option<usize> {
    if !kite.listed || kite.cond.dead != 0 || kite.cond.held() {
        return None;
    }
    // near[k] is slot k, wide[k] slot 8 + k; slots 0-15 start null.
    let mut slots: Vec<Option<usize>> = vec![None; 16];
    let put = |slots: &mut Vec<Option<usize>>, at: usize, i: usize| {
        if slots.len() <= at {
            slots.resize(at + 1, None);
        }
        slots[at] = Some(i);
    };
    let (mut near, mut wide) = (0usize, 0usize);
    for &i in sorted {
        let c = &cands[i];
        if c.cond.dead != 0 {
            continue;
        }
        let r = check_range(leader, c, scope.eye, scope.in_battle);
        if r == 0 {
            continue;
        }
        if c.flags & 0x0700_000f != 0
            && (scope.in_battle
                || (1..13).contains(&scope.field)
                || c.cond.held()
                || (c.flags & 4 != 0 && matches!(c.act, 12..=14)))
        {
            continue;
        }
        if r == 1 {
            put(&mut slots, near, i);
            near += 1;
        } else if r == 2 {
            put(&mut slots, 8 + wide, i);
            wide += 1;
        }
    }
    let total = (near + wide) as i32;
    if mode == 2 && total != 0 {
        *pri += 1;
        if *pri >= total {
            *pri = 0;
        }
    }
    let passes: [fn(u32) -> bool; 3] = [|f| f & flags::ENEMY != 0, |f| f & 0x0700_00ef == 0, |f| f & 0x0700_000f != 0];
    let mut n = 0;
    for test in passes {
        for (base, count, wider) in [(0, near, false), (8, wide, true)] {
            if wider && mode == 0 {
                continue;
            }
            for k in 0..count {
                let Some(i) = slots.get(base + k).copied().flatten() else { continue };
                if !test(cands[i].flags) {
                    continue;
                }
                if n >= *pri {
                    return Some(i);
                }
                n += 1;
            }
        }
    }
    None
}

/// `ccSelectTarget(mode)` for a standing, listed leader on a field other
/// than 1-12 (the town's call).
pub fn select(
    leader: &Leader,
    cands: &[Cmnd],
    sorted: &[usize],
    mode: i32,
    pri: &mut i32,
    eye: bool,
    in_battle: bool,
) -> Option<usize> {
    select_target(leader, &LeaderState::default(), cands, sorted, mode, pri, Scope { eye, in_battle, field: 0 })
}

/// The command lists as the lookups walk them: each list's candidates in
/// order, the leader at his place on the party's when he is listed.
fn list_walk(kite: &LeaderState, cands: &[Cmnd], list: u8) -> Vec<(Who, u32, i16)> {
    let mut walk: Vec<(Who, u32, i16)> = cands
        .iter()
        .enumerate()
        .filter(|(_, c)| c.list() == list)
        .map(|(i, c)| (Who::Cand(i), c.flags, c.id))
        .collect();
    if list == 0 && kite.listed {
        walk.insert(kite.at.min(walk.len()), (Who::Leader, kite.flags, kite.id));
    }
    walk
}

/// `ccCheckTargetTypeId(type, id)` (gcmn 0x005199e0): someone on the lists
/// whose type flags share a bit with `ty` (the party's list for & 7, the
/// enemies' for & 0xe0, the others' for & ~0xe7) and whose base id is `id`.
pub fn type_id_listed(kite: &LeaderState, cands: &[Cmnd], ty: u32, id: i16) -> bool {
    [(0u8, 7u32), (1, 0xe0), (2, 0xffff_ff18)]
        .into_iter()
        .filter(|&(_, m)| ty & m != 0)
        .any(|(list, _)| list_walk(kite, cands, list).into_iter().any(|(_, f, i)| f & ty != 0 && i == id))
}

/// `ccCheckInAreaCmnd(ch, type, mode, dist)` (gcmn 0x0051a000): someone of
/// type `ty` within `dist` of `ch` on the ground (any distance when `dist`
/// is negative), counting by `mode` those standing (0), falling (2) or
/// any (6). Type bit 0 is the leader, when `ccCheckTargetTypeId` finds his
/// type and id listed (`ch` not excluded); then the party's list from its
/// second character, the enemies' and the others' from their first, `ch`
/// itself passed over.
pub fn in_area(leader: &Leader, kite: &LeaderState, cands: &[Cmnd], ch: &Member, ty: u32, mode: i32, dist: F) -> bool {
    let counts = |dead: i16| mode == 6 || (mode == 0 && dead == 0) || (mode == 2 && dead == 2);
    let within = |p: V4| {
        let d = [ee::sub(p[0], ch.pos_p[0]), ee::sub(p[1], ch.pos_p[1]), 0, ee::ONE];
        let len = ee::sqrtf(ee::dot(d, d));
        ee::lt(dist, 0) || ee::lt(len, dist)
    };
    if ty & 1 != 0 && type_id_listed(kite, cands, kite.flags, kite.id) && counts(kite.cond.dead) && within(leader.pos_p)
    {
        return true;
    }
    for (list, mask, skip) in [(0u8, 6u32, 1usize), (1, 0xe0, 0), (2, 0xffff_ff18, 0)] {
        if ty & mask == 0 {
            continue;
        }
        for (who, f, _) in list_walk(kite, cands, list).into_iter().skip(skip) {
            if f & ty == 0 || who == ch.who {
                continue;
            }
            let (dead, pos_p) = match who {
                Who::Cand(i) => (cands[i].cond.dead, cands[i].pos_p),
                _ => (kite.cond.dead, leader.pos_p),
            };
            if counts(dead) && within(pos_p) {
                return true;
            }
        }
    }
    false
}

/// The value `ccThGameCtrl` gives `SetInBattle` (0x00517c50-0x00517d2c):
/// 1 when `inBattleDist` is negative on fields 1-12, when it is -2.0, or
/// when an enemy is within it of one of the party (`ccCheckInAreaCmnd(member,
/// 0xe0, 6, inBattleDist)`), else 0.
pub fn battle_now(leader: &Leader, kite: &LeaderState, party: &Party, cands: &[Cmnd], field: i32, dist: F) -> i32 {
    if ee::lt(dist, 0) && (1..13).contains(&field) {
        return 1;
    }
    if ee::eq(ALWAYS, dist) {
        return 1;
    }
    let near = party.slots.iter().flatten().any(|m| in_area(leader, kite, cands, m, flags::ENEMY, 6, dist));
    i32::from(near)
}

/// What the action button opens on a target of these base flags
/// (`ccMenu.type`, ccThGameCtrl 0x00518388-0x00518a0c), or None (an enemy:
/// a skill request instead, or nothing).
pub fn menu_for(flags: u32) -> Option<u16> {
    const TABLE: [(u32, u16); 19] = [
        (0x6, 21),
        (0x8, 22),
        (0x10, 23),
        (0xd00, 24),
        (0x200, 26),
        (0x1000, 25),
        (0x0800_0000, 27),
        (0x2000, 28),
        (0x4000, 4128),
        (0x8000, 4129),
        (0x1_0000, 4130),
        (0x2_0000, 4131),
        (0x4_0000, 4132),
        (0x8_0000, 4133),
        (0x10_0000, 4134),
        (0x20_0000, 4135),
        (0x40_0000, 40),
        (0x80_0000, 4139),
        (0x0400_0000, 44),
    ];
    const TAIL: [(u32, u16); 2] = [(0x0200_0000, 45), (0x0100_0000, 46)];
    TABLE.iter().chain(TAIL.iter()).find(|(m, _)| flags & m != 0).map(|&(_, t)| t)
}

/// `recoveryReq` (gcmn 0x0072eb10): 16 slots of 12 bytes - the character,
/// +4 the amount, +8 the frames left - that `ccEntryRecoveryReq` fills when
/// a hit's element heals its target (`CalcBattleDamage`) and
/// `ccCtrlRecoveryReq` runs down in `ccThGameCtrl`. `T` names a character
/// for the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryReqs<T> {
    pub slots: [Option<(T, i32, i32)>; 16],
}

impl<T> Default for RecoveryReqs<T> {
    /// Empty, as `ccInitRecoveryReq` (0x0051a750) leaves it when the task
    /// starts.
    fn default() -> RecoveryReqs<T> {
        RecoveryReqs { slots: std::array::from_fn(|_| None) }
    }
}

impl<T: Copy> RecoveryReqs<T> {
    /// `ccEntryRecoveryReq(ch, amount)` (0x0051a790): the first free slot,
    /// 10 frames; with all 16 taken the request is dropped.
    pub fn entry(&mut self, ch: T, amount: i32) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.is_none()) {
            *s = Some((ch, amount, 10));
        }
    }

    /// `ccCtrlRecoveryReq()` (0x0051a7f0): each slot in order; one whose
    /// character is off the lists or down (`alive` false) is freed, the
    /// others count down and at 0 call `heal(ch, amount)` -
    /// `ccChar::EntryAffect(ch, ch, 7, (short) amount, 0, 0)` - and are
    /// freed.
    pub fn ctrl(&mut self, mut alive: impl FnMut(T) -> bool, mut heal: impl FnMut(T, i16)) {
        for s in &mut self.slots {
            let Some((ch, amount, cnt)) = s else { continue };
            if !alive(*ch) {
                *s = None;
                continue;
            }
            *cnt -= 1;
            if *cnt <= 0 {
                heal(*ch, *amount as i16);
                *s = None;
            }
        }
    }
}

/// The command target and the buttons across frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Targeting {
    /// `cmndTarget` (an index into the last frame's candidates, with who it
    /// is), `cmndTargetPrev`, `cmndTargetPriNum`, `cmndTargetFix`.
    pub target: Option<(Kind, i32)>,
    pub prev: Option<(Kind, i32)>,
    pub pri: i32,
    pub fix: bool,
    /// The left stick's power last frame.
    pub pow_l_prev: u8,
    /// Frames since the task started (it waits 5).
    pub frames: u32,
    /// States 1-5: a menu was asked for; waiting for it to close.
    pub in_menu: bool,
    /// `cmndSortRoot`'s chain as the last frame's `ccSortCmnd` left it:
    /// each listed character within 3000 (kind, code, `cmndDist`,
    /// `cmndDirc`), nearest first.
    pub sorted: Vec<(Kind, i32, F, F)>,
    /// `menuClrWait` (gcmn 0x00378c78): frames the task waits after the
    /// events' `menu_clear` (`ccEvent::MenuClr` sets 2).
    pub menu_clr_wait: i32,
    /// The game over began ([`Ctrl::GameOver`]): the task has left its
    /// loop.
    pub over: bool,
    /// The game over's `ccChangeCmndTarget(0)` is still to come (the
    /// frame after).
    over_clear: bool,
    /// The last target's base flags, from when it was chosen: the action
    /// button reads `cmndTarget->base->type` even once the target has left
    /// the lists (it is not re-checked while the leader is off them).
    seen: Option<((Kind, i32), u32)>,
}

/// What the action button did this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub kind: Kind,
    pub code: i32,
    pub flags: u32,
    /// `ccMenu.type`.
    pub menu: u16,
}

/// What `ccThGameCtrl` reads besides the characters: this frame's pushes
/// (`ccSys` +0x2d0) against the save's button assignment, the field's
/// menus (`ccMenu`), the player and the area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    /// The left stick's power (`pow_l`).
    pub pow_l: u8,
    /// The camera looks through Kite's eyes.
    pub eye: bool,
    /// Pushed: action (+0x8404), personal menu (+0x8406), chat (+0x8408),
    /// option (+0x840a).
    pub action: bool,
    pub personal: bool,
    pub chat: bool,
    pub option: bool,
    /// `ccPlayerMenuCheck()` (gcmn 0x0059cd70: the party not wiped out,
    /// `ccSkillCheck(plw) < 2`, the player's act neither 12 nor 13) and
    /// `ccLoadDispCheck()` 0. [`Targeting::frame`] works these out itself
    /// and reads this as `ccLoadDispCheck() == 0` only.
    pub player_ok: bool,
    /// `ccMenu`: `CheckMenuType() == -1` and `openReqNum & 0xfff != 74`.
    pub menu_idle: bool,
    /// `ccMenu` +0xfe `forbid` (the events' `menu_ban`) and +0x100
    /// `forbidChatExcept`.
    pub forbid: bool,
    pub forbid_chat_except: bool,
    /// `ccMenu` +0xf4 `plAttack`.
    pub pl_attack: bool,
    /// The player's condition: sleep, paralysis, confusion or charm
    /// (+0x1a, +0x24, +0x1c, +0x1e); dead (+0x08). Only [`Targeting::step`]
    /// reads these; [`Targeting::frame`] takes [`LeaderState::cond`].
    pub held: bool,
    pub dead: bool,
    /// `ccSkillCheck(plw) == 1`. Only [`Targeting::step`] reads it;
    /// [`Targeting::frame`] asks [`Host::skill_check`].
    pub skill_one: bool,
    /// `game.area` (+0x14: 0 town, 1 field, 2 dungeon), `game.field`
    /// (+0x24), `WORLD_MAN::GetDungeonType()`.
    pub area: i32,
    pub field: i32,
    pub dungeon_type: i32,
}

impl Input {
    /// A town frame with no menu open and nothing holding the player.
    pub fn town(pow_l: u8, action: bool, eye: bool) -> Input {
        Input {
            pow_l,
            eye,
            action,
            personal: false,
            chat: false,
            option: false,
            player_ok: true,
            menu_idle: true,
            forbid: false,
            forbid_chat_except: false,
            pl_attack: false,
            held: false,
            dead: false,
            skill_one: false,
            area: 0,
            field: 0,
            dungeon_type: 0,
        }
    }
}

/// What [`Targeting::frame`] reads besides [`Leader`], the candidates and
/// [`Input`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battle {
    pub kite: LeaderState,
    pub party: Party,
    /// `ccMenu` +0x06 `menu`: 66 (Data Drain) leaves `inBattle` alone.
    pub menu: i16,
    /// `ccMenuCtrl::CheckMenuType() == -1` (gcmn 0x00526150: `menu` when
    /// it equals `menuNext`, else 88): what a requested menu's wait ends
    /// on.
    pub menu_closed: bool,
    /// `ccLoadDispCheck()` (main 0x0019c430: `ld` +0x19c).
    pub load_disp: bool,
    /// `ccCheckGtHackAnm()` (gcmn 0x0059cd60): `ghoFlag`, set by
    /// `ccPlayer`'s constructor when he arrives by gate hacking and cleared
    /// by `GateHackingOut`.
    pub gt_hack: bool,
    /// `compulsionGameOver` (0x00378c74), which `DataDrainMenu` sets.
    pub compulsion_game_over: bool,
}

impl Battle {
    /// Kite alone and standing, no menu, nothing loading.
    pub fn alone(leader: &Leader) -> Battle {
        Battle {
            kite: LeaderState::default(),
            party: Party::alone(leader, 0),
            menu: -1,
            menu_closed: true,
            load_disp: false,
            gt_hack: false,
            compulsion_game_over: false,
        }
    }
}

/// What the task calls outside the command lists, at the point in the
/// frame the game calls it.
pub trait Host {
    /// `ccEvent::CheckOperate(n, 0)`: 9 action, 10 chat, 11 personal, 12
    /// option, 14 the personal menu's field-13 case.
    fn check_operate(&mut self, n: i32) -> bool;
    /// `ccSkillCheck(plw)` (gcmn 0x005723e0): the id of the player's
    /// interruptible running skill (1 the normal attack), 0 for none -
    /// piney-battle's `flow::Skills::check`.
    fn skill_check(&mut self) -> i32;
    /// `ccSkillRequest(plw, target, 1)` (gcmn 0x00572700): the normal
    /// attack on the target - piney-battle's `flow::Skills::request(..,
    /// kite, Some(target), 1, 0, ..)`. The next `skill_check` of the frame
    /// must see it.
    fn skill_request(&mut self, kind: Kind, code: i32);
    /// `ccCtrlRecoveryReq()`: [`RecoveryReqs::ctrl`], where the game runs
    /// it. Nothing by default.
    fn recovery(&mut self) {}
}

/// What a frame of [`Targeting::frame`] asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctrl {
    /// The action button on the target: `ccMenu.openReqNum = menu`, `mode
    /// = action_mode(menu, area)`, `firstTime = 1`; the player's pauseSW
    /// set ([`Out::pause`]); state 5 until the menu closes.
    Action(Action),
    /// A button asked for a menu: `openReqNum = menu`, `mode = 0`,
    /// `firstTime = 1` (states 1 personal, 2 chat, 3 option).
    Open(i16),
    /// The action button on an enemy with no skill running: the normal
    /// attack was requested ([`Host::skill_request`]) and
    /// `cmndTargetFix` set; [`Out::pl_attack`] carries `plAttack`.
    Attack { kind: Kind, code: i32 },
    /// The party is wiped out (or `compulsionGameOver`): the task leaves
    /// its loop for the game over (0x0051791c on), which the caller runs.
    GameOver,
}

/// One frame of [`Targeting::frame`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Out {
    pub step: Option<Ctrl>,
    /// `ccMenu.plAttack` as the frame leaves it.
    pub pl_attack: bool,
    /// The player's pauseSW (+0xe0 bit 0) when the frame wrote it: set
    /// with an action menu, cleared when that wait ends and when the action
    /// button is pushed with no target.
    pub pause: Option<bool>,
}

/// `ccMenu.mode` as the action button sets it with the menu (+0x16): 1
/// (the other tasks keep running, the player held by `pauseSW`) but for
/// a party member outside a town (21) and a fountain (40).
pub fn action_mode(menu: u16, area: i32) -> i16 {
    match menu {
        21 if area != 0 => 0,
        40 => 0,
        _ => 1,
    }
}

/// The PERSONAL menu the triangle opens (gcmn 0x005181d0): 0 in a town; in
/// a field 2 on fields 1-12 and 67, and on 13 while operation 14 is held,
/// else 1; in a dungeon 1 on the special floors (types 8 and 9), else 2.
/// (Another area keeps the last menu chosen; 0 here.)
fn personal_menu(input: &Input, host: &mut dyn Host) -> i16 {
    match input.area {
        1 => {
            let f = input.field;
            if (1..13).contains(&f) || f == 67 || (f == 13 && !host.check_operate(14)) { 2 } else { 1 }
        }
        2 => {
            if matches!(input.dungeon_type, 8 | 9) {
                1
            } else {
                2
            }
        }
        _ => 0,
    }
}

/// The town's [`Host`]: `ccEvent::CheckOperate` through a closure and a
/// fixed `ccSkillCheck`.
struct Town<'a> {
    check: &'a mut dyn FnMut(i32) -> bool,
    skill: i32,
}

impl Host for Town<'_> {
    fn check_operate(&mut self, n: i32) -> bool {
        (self.check)(n)
    }
    fn skill_check(&mut self) -> i32 {
        self.skill
    }
    fn skill_request(&mut self, _: Kind, _: i32) {}
}

impl Targeting {
    /// One frame of `ccThGameCtrl` (gcmn 0x00517800) in its play states:
    /// sort, select the command target, then the buttons in the game's
    /// order - chat (or only chat under `menu_ban`), option, personal, and
    /// the action button on the target. `check` is
    /// `ccEvent::CheckOperate(n, 0)`: 10 chat, 12 option, 11 personal, 14
    /// the personal menu's field case, 9 action.
    ///
    /// The town's call of [`Targeting::frame`]: Kite alone, `held`,
    /// `dead` and `skill_one` from `input`, `player_ok` false for a frame
    /// that waits; a menu's wait ends with [`Targeting::close_menu`].
    pub fn step(
        &mut self,
        leader: &Leader,
        cands: &mut [Cmnd],
        input: &Input,
        check: &mut dyn FnMut(i32) -> bool,
    ) -> Option<Step> {
        let mut b = Battle::alone(leader);
        b.kite.cond.dead = i16::from(input.dead);
        b.kite.cond.sleep = i16::from(input.held);
        b.menu_closed = false;
        b.load_disp = !input.player_ok;
        let mut game = InBattle::default();
        let mut host = Town { check, skill: i32::from(input.skill_one) };
        let out = self.frame(leader, cands, input, &b, &mut game, &mut host);
        match out.step {
            Some(Ctrl::Open(m)) => Some(Step::Open(m)),
            Some(Ctrl::Action(a)) => Some(Step::Action(a)),
            _ if input.pl_attack && !out.pl_attack => Some(Step::ClearAttack),
            _ => None,
        }
    }

    /// One frame of `ccThGameCtrl`'s loop (gcmn 0x005178f0-0x00518a98), in
    /// the game's order:
    ///
    /// - the party wiped out or `compulsionGameOver`: [`Ctrl::GameOver`]
    ///   once, then nothing (the next frame clears the target, as the game
    ///   over's `ccChangeCmndTarget(0)` does);
    /// - [`Host::recovery`], [`sort`];
    /// - unless `ccMenu.menu` is 66, [`InBattle::set`] of [`battle_now`]
    ///   while the leader's type and id are listed
    ///   (`ccCheckTargetTypeId`), else `inBattle` and `inBattleCnt` 0;
    /// - `ghoFlag`, `menuClrWait` (counted down), `ccPlayerMenuCheck` and
    ///   `ccLoadDispCheck`, the first 5 frames: wait;
    /// - a requested menu: wait for `CheckMenuType() == -1`, then pauseSW
    ///   off;
    /// - [`Input::menu_idle`], then [`select_target`] (unless
    ///   `cmndTargetFix`, which only drops a target gone from the lists)
    ///   while the leader is listed and `ccSkillCheck < 2`;
    /// - the events' manual control of the leader: nothing more;
    /// - the buttons: `menu_ban`, chat, option, the leader held (clears
    ///   `plAttack` and `cmndTargetFix`), personal, the leader down (the
    ///   same), the action button;
    /// - `plAttack` with `ccSkillCheck != 1`: `plAttack` and
    ///   `cmndTargetFix` cleared.
    ///
    /// `game` is `ccGame`'s battle state, read and written; the map button
    /// (select) is not here.
    pub fn frame(
        &mut self,
        leader: &Leader,
        cands: &mut [Cmnd],
        input: &Input,
        b: &Battle,
        game: &mut InBattle,
        host: &mut dyn Host,
    ) -> Out {
        let mut out = Out { step: None, pl_attack: input.pl_attack, pause: None };
        if self.over {
            if self.over_clear {
                self.over_clear = false;
                if self.target.is_some() {
                    self.prev = self.target.take();
                }
            }
            return out;
        }
        if annihilated(&b.party) || b.compulsion_game_over {
            self.over = true;
            self.over_clear = true;
            out.step = Some(Ctrl::GameOver);
            return out;
        }
        host.recovery();
        let sorted = sort(leader, cands);
        self.sorted = sorted.iter().map(|&i| (cands[i].kind, cands[i].code, cands[i].dist, cands[i].dirc)).collect();
        if b.menu != 66 {
            if type_id_listed(&b.kite, cands, b.kite.flags, b.kite.id) {
                game.set(battle_now(leader, &b.kite, &b.party, cands, input.field, game.dist));
            } else {
                game.in_battle = 0;
                game.cnt = 0;
            }
        }
        if b.gt_hack {
            return out;
        }
        if self.menu_clr_wait > 0 {
            self.menu_clr_wait -= 1;
            return out;
        }
        // ccPlayerMenuCheck: the party is standing (above), the skill, the
        // act; then ccLoadDispCheck.
        if host.skill_check() >= 2 || matches!(b.kite.act, 12 | 13) || b.load_disp {
            return out;
        }
        // The task's first frames only sort.
        if self.frames < 5 {
            self.frames += 1;
            return out;
        }
        // States 1-5: waiting for the menu (CheckMenuType == -1).
        if self.in_menu {
            if b.menu_closed {
                self.in_menu = false;
                out.pause = Some(false);
            }
            return out;
        }
        // State 0 waits while a menu is open or the stream menu (74) is
        // asked for.
        if !input.menu_idle {
            return out;
        }
        let listed = |t: Option<(Kind, i32)>| t.and_then(|t| cands.iter().position(|c| (c.kind, c.code) == t));
        let prev_pow = self.pow_l_prev;
        self.pow_l_prev = input.pow_l;
        if !self.fix {
            if b.kite.listed && host.skill_check() < 2 {
                let mode = if self.target.is_none() {
                    self.pri = 0;
                    0
                } else if input.pow_l >= 64 && prev_pow < 64 {
                    2
                } else {
                    1
                };
                let scope = Scope { eye: input.eye, in_battle: game.in_battle != 0, field: input.field };
                let t = select_target(leader, &b.kite, cands, &sorted, mode, &mut self.pri, scope);
                self.change(t.map(|i| (cands[i].kind, cands[i].code)), cands);
            }
        } else if self.target.is_some() {
            self.pri = 0;
            if listed(self.target).is_none() {
                self.change(None, cands);
            }
        }
        if b.kite.control {
            return out;
        }
        let open = |t: &mut Targeting, mut out: Out, menu: i16| {
            t.in_menu = true;
            out.step = Some(Ctrl::Open(menu));
            out
        };
        if input.forbid {
            if input.forbid_chat_except && input.chat && host.check_operate(10) {
                return open(self, out, 3);
            }
            return out;
        }
        if input.chat && host.check_operate(10) {
            return open(self, out, 3);
        }
        if input.option && host.check_operate(12) {
            return open(self, out, 12);
        }
        if b.kite.cond.held() {
            return self.clear_attack(out);
        }
        if input.personal && host.check_operate(11) {
            let menu = personal_menu(input, host);
            return open(self, out, menu);
        }
        if b.kite.cond.dead != 0 {
            return self.clear_attack(out);
        }
        if input.action && host.check_operate(9) {
            let Some(t) = self.target else {
                // No target: the player's pauseSW off, state 0.
                out.pause = Some(false);
                return out;
            };
            // cmndTarget->base->type, as last seen for a target gone from
            // the lists.
            let seen = self.seen.filter(|&(w, _)| w == t).map(|(_, f)| f);
            if let Some(f) = listed(Some(t)).map(|i| cands[i].flags).or(seen) {
                let (kind, code) = t;
                if let Some(menu) = menu_for(f) {
                    self.in_menu = true;
                    out.pause = Some(true);
                    out.step = Some(Ctrl::Action(Action { kind, code, flags: f, menu }));
                } else if f & flags::ENEMY != 0 && host.skill_check() == 0 {
                    host.skill_request(kind, code);
                    out.pl_attack = true;
                    self.fix = true;
                    out.step = Some(Ctrl::Attack { kind, code });
                }
            }
        }
        if out.pl_attack && host.skill_check() != 1 {
            out.pl_attack = false;
            self.fix = false;
        }
        out
    }

    /// `plAttack = 0; cmndTargetFix = 0` when `plAttack` is set.
    fn clear_attack(&mut self, mut out: Out) -> Out {
        if out.pl_attack {
            out.pl_attack = false;
            self.fix = false;
        }
        out
    }

    /// `ccChangeCmndTarget(t)`: `t` if it is listed (`ccCheckTarget`),
    /// else none.
    fn change(&mut self, t: Option<(Kind, i32)>, cands: &[Cmnd]) {
        if t == self.target {
            return;
        }
        self.prev = self.target;
        let found = t.and_then(|t| cands.iter().find(|c| (c.kind, c.code) == t));
        self.target = found.map(|c| (c.kind, c.code));
        if let Some(c) = found {
            self.seen = Some(((c.kind, c.code), c.flags));
        }
    }

    /// `ccChangeCmndTarget(t)` as the field's menus call it: the target
    /// becomes `cmndTargetPrev` and `t` the target.
    pub fn set_target(&mut self, t: Option<(Kind, i32)>) {
        if t != self.target {
            self.prev = self.target;
            self.target = t;
        }
    }

    /// `ccMenuCtrl::CheckMenuType() == -1`: the menu closed; back to state
    /// 0 ([`Targeting::step`]'s callers; [`Targeting::frame`] reads
    /// [`Battle::menu_closed`] itself).
    pub fn close_menu(&mut self) {
        self.in_menu = false;
    }

    /// The events' `menu_clear` (`ccEvent::MenuClr`, 0x001b25c0): the task
    /// waits 2 frames.
    pub fn menu_clear(&mut self) {
        self.menu_clr_wait = 2;
    }
}

/// What a frame of [`Targeting::step`] asks of the field's menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The action button on the command target (state 5).
    Action(Action),
    /// A button asked for a menu: `openReqNum = menu`, `mode = 0`,
    /// `firstTime = 1` (states 1 personal, 2 chat, 3 option).
    Open(i16),
    /// `ccMenu.plAttack` and `cmndTargetFix` cleared.
    ClearAttack,
}

/// What the action button asks the field's menus for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkRequest {
    /// A walking PC spoken to (`ccMenu` type 22). `msg` is its lines' table
    /// (`npcTbl[npc].param.msg`, a GCMN.PRG address) and `line` the one it
    /// says, when the PC chose it.
    Talk { npc: i32, msg: u32, line: Option<i32> },
    /// A merchant: menu 24 (weapons, items or magic), 25 (the Recorder) or
    /// 26 (Elf's Haven). The shop opens with the greeting
    /// `base->msg[game.server]` (`ccMessage::Open` with the merchant's
    /// name): `msg` the table, `line` the server.
    Shop { npc: i32, shop: Shop, msg: u32, line: i32 },
    /// Any other target: `ccMenu` type `menu` for the character
    /// `kind`/`code` (the Chaos Gate's 28, a party member's 21, an
    /// administrator's 23, ...).
    Menu { menu: u16, kind: Kind, code: i32 },
    /// Talking to a character an event took over (`add_target`): the event
    /// runs instead of a menu (`ccEvent::CheckOperate(9)`).
    Event { kind: Kind, code: i32 },
    /// A button asked for menu `menu` (`openReqNum`), `mode` 0 and
    /// `firstTime` 1: PERSONAL (0-2), CHAT (3) or OPTION (12).
    Open { menu: i16 },
    /// `ccMenu.plAttack = 0`.
    ClearAttack,
}

/// What `ccThGameCtrl` reads of the field's menus (`ccMenu`) each frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuView {
    /// `CheckMenuType() == -1` and `openReqNum & 0xfff != 74`.
    pub idle: bool,
    /// +0xfe `forbid`, +0x100 `forbidChatExcept`, +0xf4 `plAttack`.
    pub forbid: bool,
    pub forbid_chat_except: bool,
    pub pl_attack: bool,
}

impl Default for MenuView {
    /// No menu open, nothing forbidden.
    fn default() -> MenuView {
        MenuView { idle: true, forbid: false, forbid_chat_except: false, pl_attack: false }
    }
}

/// What a merchant sells (its base type flags).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shop {
    /// 0x100, "EQUIPSHOP".
    Weapon,
    /// 0x200, "FAIRYSHOP" (Elf's Haven).
    Fairy,
    /// 0x400.
    Item,
    /// 0x800.
    Magic,
    /// 0x1000: saving (the Recorder).
    Recorder,
}

impl Shop {
    /// The shop a merchant's base flags name.
    pub fn of(f: u32) -> Option<Shop> {
        [
            (flags::EQUIP_SHOP, Shop::Weapon),
            (flags::FAIRY_SHOP, Shop::Fairy),
            (flags::ITEM_SHOP, Shop::Item),
            (flags::MAGIC_SHOP, Shop::Magic),
            (flags::RECORDER, Shop::Recorder),
        ]
        .into_iter()
        .find(|(m, _)| f & m != 0)
        .map(|(_, s)| s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus_by_flags() {
        assert_eq!(menu_for(flags::PC), Some(22));
        assert_eq!(menu_for(flags::EQUIP_SHOP), Some(24));
        assert_eq!(menu_for(flags::ITEM_SHOP), Some(24));
        assert_eq!(menu_for(flags::MAGIC_SHOP), Some(24));
        assert_eq!(menu_for(flags::FAIRY_SHOP), Some(26));
        assert_eq!(menu_for(flags::RECORDER), Some(25));
        assert_eq!(menu_for(flags::SYSOPE), Some(23));
        assert_eq!(menu_for(flags::CHAOS_GATE), Some(28));
        assert_eq!(menu_for(0x20), None);
        assert_eq!(Shop::of(0x1000), Some(Shop::Recorder));
    }

    #[test]
    fn nearest_in_front_is_the_target() {
        let v = |x: f32, y: f32| [x.to_bits(), y.to_bits(), 0, ee::ONE];
        let leader = Leader { pos_p: v(0.0, 0.0), dirc: [0; 4], width: 45f32.to_bits() };
        let w = 45f32.to_bits();
        // Heading 0 faces -y: one ahead, one behind, one far ahead.
        let mut cands = vec![
            Cmnd::new(Kind::Npc, 1, flags::PC, w, v(0.0, 250.0)),
            Cmnd::new(Kind::Npc, 2, flags::PC, w, v(0.0, -250.0)),
            Cmnd::new(Kind::Npc, 3, flags::PC, w, v(0.0, -2000.0)),
        ];
        let sorted = sort(&leader, &mut cands);
        assert_eq!(sorted, [0, 1, 2]);
        let mut pri = 0;
        assert_eq!(select(&leader, &cands, &sorted, 1, &mut pri, false, false), Some(1));
        let mut t = Targeting::default();
        let input = Input::town(0, true, false);
        let mut ok = |_| true;
        for _ in 0..5 {
            assert_eq!(t.step(&leader, &mut cands, &input, &mut ok), None);
        }
        let Some(Step::Action(a)) = t.step(&leader, &mut cands, &input, &mut ok) else { panic!() };
        assert_eq!((a.code, a.menu), (2, 22));
        assert!(t.in_menu);
        assert_eq!(t.step(&leader, &mut cands, &input, &mut ok), None);
        t.close_menu();
        assert!(!t.in_menu);
    }

    #[test]
    fn menu_buttons_in_the_game_order() {
        let leader = Leader { pos_p: [0, 0, 0, ee::ONE], dirc: [0; 4], width: 45f32.to_bits() };
        let mut t = Targeting { frames: 5, ..Targeting::default() };
        let mut ok = |_| true;
        let step = |t: &mut Targeting, input: &Input, check: &mut dyn FnMut(i32) -> bool| {
            let r = t.step(&leader, &mut [], input, check);
            t.close_menu();
            r
        };
        // Chat wins over option, personal and action pushed together.
        let all = Input { chat: true, option: true, personal: true, ..Input::town(0, true, false) };
        assert_eq!(step(&mut t, &all, &mut ok), Some(Step::Open(3)));
        // Under menu_ban only chat, and only with forbidChatExcept.
        let banned = Input { forbid: true, ..all };
        assert_eq!(step(&mut t, &banned, &mut ok), None);
        let chat_only = Input { forbid_chat_except: true, ..banned };
        assert_eq!(step(&mut t, &chat_only, &mut ok), Some(Step::Open(3)));
        let option = Input { chat: false, ..all };
        assert_eq!(step(&mut t, &option, &mut ok), Some(Step::Open(12)));
        // The events' lock on chat (10) lets option through.
        let mut no_chat = |n| n != 10;
        assert_eq!(step(&mut t, &all, &mut no_chat), Some(Step::Open(12)));
        // Personal by area.
        let personal = Input { personal: true, ..Input::town(0, false, false) };
        assert_eq!(step(&mut t, &personal, &mut ok), Some(Step::Open(0)));
        let field = |f| Input { area: 1, field: f, ..personal };
        assert_eq!(step(&mut t, &field(5), &mut ok), Some(Step::Open(2)));
        assert_eq!(step(&mut t, &field(67), &mut ok), Some(Step::Open(2)));
        assert_eq!(step(&mut t, &field(20), &mut ok), Some(Step::Open(1)));
        assert_eq!(step(&mut t, &field(13), &mut ok), Some(Step::Open(1)));
        let mut held14 = |n| n != 14;
        assert_eq!(step(&mut t, &field(13), &mut held14), Some(Step::Open(2)));
        let dungeon = |d| Input { area: 2, dungeon_type: d, ..personal };
        assert_eq!(step(&mut t, &dungeon(9), &mut ok), Some(Step::Open(1)));
        assert_eq!(step(&mut t, &dungeon(0), &mut ok), Some(Step::Open(2)));
        // Asleep or paralysed: no personal menu, the attack flag cleared.
        let held = Input { held: true, pl_attack: true, ..personal };
        assert_eq!(step(&mut t, &held, &mut ok), Some(Step::ClearAttack));
        // Nothing while a menu is open or the player cannot act.
        assert_eq!(step(&mut t, &Input { menu_idle: false, ..all }, &mut ok), None);
        assert_eq!(step(&mut t, &Input { player_ok: false, ..all }, &mut ok), None);
    }

    #[test]
    fn set_in_battle_counts_down_between_changes() {
        let mut g = InBattle::default();
        g.set(1);
        assert_eq!((g.in_battle, g.cnt), (1, 60));
        g.set(0);
        assert_eq!((g.in_battle, g.cnt), (1, 59));
        for _ in 0..59 {
            g.set(0);
        }
        assert_eq!((g.in_battle, g.cnt), (1, 0));
        g.set(0);
        assert_eq!((g.in_battle, g.cnt), (2, 60));
        g.cnt = 0;
        g.set(0);
        assert_eq!((g.in_battle, g.cnt), (0, 60));
    }

    /// A host with a normal attack that goes through.
    struct Fight {
        skill: i32,
        asked: Vec<(Kind, i32)>,
    }

    impl Host for Fight {
        fn check_operate(&mut self, _: i32) -> bool {
            true
        }
        fn skill_check(&mut self) -> i32 {
            self.skill
        }
        fn skill_request(&mut self, kind: Kind, code: i32) {
            self.asked.push((kind, code));
            self.skill = 1;
        }
    }

    #[test]
    fn the_action_button_attacks_an_enemy_in_battle() {
        let v = |x: f32, y: f32| [x.to_bits(), y.to_bits(), 0, ee::ONE];
        let leader = Leader { pos_p: v(0.0, 0.0), dirc: [0; 4], width: 45f32.to_bits() };
        let w = 45f32.to_bits();
        // A goblin 100 ahead, Orca beside him; another goblin 2000 off.
        let mut cands = vec![
            Cmnd::new(Kind::Spc, 2, 6, w, v(80.0, 0.0)),
            Cmnd::new(Kind::Enemy, 7, 0x20, w, v(0.0, -100.0)),
            Cmnd::new(Kind::Enemy, 8, 0x20, w, v(0.0, -2000.0)),
        ];
        let mut b = Battle::alone(&leader);
        b.party.slots[1] = Some(Member { pos_p: cands[0].pos_p, dead: 0, who: Who::Cand(0) });
        b.party.num = 2;
        let mut game = InBattle::default();
        let mut t = Targeting { frames: 5, ..Targeting::default() };
        let mut host = Fight { skill: 0, asked: vec![] };
        let input = Input { area: 1, field: 14, ..Input::town(0, true, false) };
        let out = t.frame(&leader, &mut cands, &input, &b, &mut game, &mut host);
        assert_eq!((game.in_battle, game.cnt), (1, 60));
        assert_eq!(t.target, Some((Kind::Enemy, 7)));
        assert_eq!(out.step, Some(Ctrl::Attack { kind: Kind::Enemy, code: 7 }));
        assert!(out.pl_attack && t.fix);
        assert_eq!(host.asked, [(Kind::Enemy, 7)]);
        // The attack over, plAttack and the fix go.
        host.skill = 0;
        let input = Input { pl_attack: true, action: false, ..input };
        let out = t.frame(&leader, &mut cands, &input, &b, &mut game, &mut host);
        assert!(!out.pl_attack && !t.fix);
        // The goblin down, the other out of reach, and Orca is no target
        // in a fight.
        cands[1].cond.dead = 2;
        let out = t.frame(&leader, &mut cands, &input, &b, &mut game, &mut host);
        assert_eq!((t.target, out.step), (None, None));
        // Kite down, alone: the game over.
        b.party.slots[0].as_mut().unwrap().dead = 2;
        b.party.num = 1;
        b.party.slots[1] = None;
        let out = t.frame(&leader, &mut cands, &input, &b, &mut game, &mut host);
        assert_eq!(out.step, Some(Ctrl::GameOver));
    }

    #[test]
    fn recoveries_land_after_ten_frames() {
        let mut r = RecoveryReqs::default();
        r.entry(3usize, 0x1_0005);
        r.entry(4usize, 7);
        let mut healed = vec![];
        for f in 0..10 {
            r.ctrl(|ch| ch == 3 || f < 5, |ch, n| healed.push((f, ch, n)));
        }
        assert_eq!(healed, [(9, 3, 5)]);
        assert!(r.slots.iter().all(|s| s.is_none()));
    }
}
