//! `ccAI` (gcmn ai.cpp, 0x260 bytes) as the event scripts drive it: a party
//! character in manual mode (`manualSW`) standing, turning and transferring
//! by remote command, and the `ccSpcChar` members those commands touch.
//!
//! ```text
//! ccAI  +0x00 manualSW bit 0, followSW 1, talkFlag 2, runFlag 3, remoteFlag 4,
//!             goBackFlag 5, inviteFlag 6, selfFlag 7
//!       +0x01 battleFlag bits 0-1, targetFlag 2-3, chatCmdFlag 4-6, firstTime 7
//!       +0x02 chatRequest bit 0      +0x03 skillMask
//!       +0x04 strategyCMD  +0x06 strategy  +0x08 mode  +0x0c modeOld  +0x10 count
//!       +0x18 bodyPtr  +0x74 remoteCmd  +0x76 chatCmd  +0x80 arrivalChatCnt
//!       +0x9a gDeg (u16)  +0x9c gRotSp  +0xb4 detourCnt  +0xb6 levelOld
//!       +0x160 sysMsg (ccAIEntry: +0 id, -1 until SysMsgEntry)
//! ```
//!
//! The field names follow the game's (and `piney-battle`'s `party_ai::Ai`).
//! What is here:
//!
//! - `ccAI::ManualMode` (gcmn 0x00583270), `SetRemoteCmd` (0x005832e0),
//!   `SetDircZ` (0x00581500), `ChangeMode` (0x00589740), the constructor's
//!   fields (0x0057c5f0);
//! - `ccAI::Brains` (0x0057ca00) on its `talkFlag` and manual paths, with
//!   `ChatCommand` (0x00583d60) and `ChatMessageSender` (0x00586420) as far
//!   as a character with no chat command queued reaches them;
//! - `ccAI::ManualControl` (0x00580ef0) for remote commands 0 and 3-7, and
//!   1 and 2 on a straight goal (`gPoint` -1, the events' walks:
//!   `MoveP2P` 0x00582d50);
//! - `ccSpcChar::ManualModeAI` (0x0059eba0), `SetBootStatus` (0x0059e950),
//!   `TransferIn` (0x0059ea60), `TransferOut` (0x0059ea80);
//! - `ccSetDirc` / `ccGetDircChg` (main 0x001da0b0 / 0x001d9eb0) exactly as
//!   the AI calls them.
//!
//! What is not: the non-manual paths (`ActInTown`, `ActInField`,
//! `ActInDungeon`, following, fighting), remote walks along the town
//! navigator's route (`gPoint` not -1: no event asks for one), the AI message system (`SysMsgEntry`,
//! `ReadSysMsg`, `ccAISysMsgSend*`: with nothing queued `ReadSysMsg` reads
//! nothing), chat commands and chat lines, `distPl` / `dircTg`.

use crate::ee::{self, F, ONE, V4};
use crate::hit::{self, Hits};

/// `ccAI::gRotSp` as the constructor and `ManualMode` leave it: a quarter of
/// the way a frame (`ccGetDircChg`'s `16 d / 64`).
pub const ROT_QUARTER: i16 = 64;

/// A character's `ccAI`: the members the manual paths read and write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ai {
    /// +0x00 bits 0-7.
    pub manual_sw: bool,
    pub follow_sw: bool,
    pub talk_flag: bool,
    pub run_flag: bool,
    pub remote_flag: bool,
    pub go_back_flag: bool,
    pub invite_flag: bool,
    pub self_flag: bool,
    /// +0x01: signed 2-bit `battleFlag` (1 in mode 3, -1 in mode 1),
    /// `targetFlag`, signed 3-bit `chatCmdFlag`, `firstTime`.
    pub battle_flag: i8,
    pub target_flag: i8,
    pub chat_cmd_flag: i8,
    pub first_time: bool,
    /// +0x02 bit 0, +0x03.
    pub chat_request: bool,
    pub skill_mask: u8,
    /// +0x04, +0x06: `partyStrategy` when made.
    pub strategy_cmd: i16,
    pub strategy: i16,
    /// +0x08, +0x0c: 0 until `ChangeMode(1, 1)` (`ccSPC::Reboot`,
    /// `inviteSpc`); `Brains` does nothing in mode 0.
    pub mode: i32,
    pub mode_old: i32,
    /// +0x10: counts `Brains` calls (the constructor starts it at
    /// `rand() >> 3`; 0 here).
    pub count: i32,
    /// +0x74, +0x76.
    pub remote_cmd: i16,
    pub chat_cmd: i16,
    /// +0x80: 150 in a town, counted down by `ChatMessageSender`.
    pub arrival_chat_cnt: i16,
    /// +0x9a the heading to turn to (a 16-bit angle), +0x9c the turn's
    /// `ccSetDirc` rate.
    pub g_deg: u16,
    pub g_rot_sp: i16,
    /// +0xb0, +0xb4.
    pub attack_cycle: i16,
    pub detour_cnt: i16,
    /// `gPoint` and `gPos`: where a remote walk goes (`SetGoalPos`): -1
    /// straight to `gPos`.
    pub g_point: i16,
    pub g_pos: V4,
}

impl Ai {
    /// `ccAI::ccAI(spcID)` (0x0057c5f0) as far as these paths read it:
    /// every flag clear, mode 0, `gRotSp` 64, `chatCmd` -1, and
    /// `arrivalChatCnt` as the area gives it (150 in a town).
    pub fn new(arrival_chat_cnt: i16, strategy: i16) -> Self {
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
            strategy_cmd: strategy,
            strategy,
            mode: 0,
            mode_old: 0,
            count: 0,
            remote_cmd: 0,
            chat_cmd: -1,
            arrival_chat_cnt,
            g_deg: 0,
            g_rot_sp: ROT_QUARTER,
            attack_cycle: 0,
            detour_cnt: 0,
            g_point: 0,
            g_pos: [0; 4],
        }
    }

    /// `ccAI::ChangeMode(n, f)` (0x00589740): nothing when already in mode
    /// `n`; else (the message to the other AIs with `f` 0 is not sent here)
    /// `modeOld = mode`, `mode = n`, and `battleFlag` 1 for mode 3 (with
    /// `attackCycle` 0) or -1 for mode 1.
    pub fn change_mode(&mut self, n: i32) {
        if self.mode == n {
            return;
        }
        self.mode_old = self.mode;
        self.mode = n;
        if n == 3 {
            self.battle_flag = 1;
            self.attack_cycle = 0;
        } else if n == 1 {
            self.battle_flag = -1;
        }
    }

    /// `ccAI::ManualMode` (0x00583270): manual control on, the heading to
    /// keep the body's own, turning a quarter of the way, remote command 0.
    /// `remoteFlag` is left as it is.
    pub fn manual_mode(&mut self, dirc: &V4) {
        self.manual_sw = true;
        self.g_deg = ee::rad2deg(dirc[2]) as u16;
        self.g_rot_sp = ROT_QUARTER;
        self.remote_cmd = 0;
    }

    /// `ccAI::SetDircZ(dd)` (0x00581500): `gDeg = dd` and the body's heading
    /// `DEG2RAD(dd)` at once.
    pub fn set_dirc_z(&mut self, dd: u16, dirc: &mut V4) {
        self.g_deg = dd;
        dirc[2] = ee::deg2rad(dd as i16);
    }

    /// `pc_turn` / `pc_face` with a rate (`ccEvent::Execute` 0x001aedc4,
    /// 0x001aef54): `gRotSp = chg` (1 meaning 64) and `gDeg = dd`; the
    /// remote command is left alone, so the turn happens while it is 0.
    pub fn turn_to(&mut self, dd: u16, chg: i16) {
        self.g_rot_sp = if chg == 1 { ROT_QUARTER } else { chg };
        self.g_deg = dd;
    }
}

/// `ccGetDircChg(cur, tgt, spd)` (main 0x001d9eb0): the step from `cur`
/// toward `tgt` the shorter way, in sixteenths of the game's 16-bit angle:
/// `d = 16 ((tgt - cur) & 0xffff)`, folded to `0x100000 - d` (negated at the
/// end) past half a turn; divided by `spd & 0xffff` (and at least 1) when
/// `spd & 0xf0000` is 0; at most 24576. The game's switch compares
/// `spd & 0xf0000` with 1 and 2, which it can never equal, so any other rate
/// (a negative `gRotSp`) takes the whole `d`. A rate of 0 (never set by the
/// AI: `pc_turn`'s `chg` 0 turns at once) divides by zero in `__divdi3`;
/// here it takes the whole `d`.
pub fn get_dirc_chg(cur: i16, tgt: i16, spd: i32) -> i32 {
    let d = i32::from(tgt).wrapping_sub(i32::from(cur));
    if d == 0 {
        return 0;
    }
    let mut a = i64::from(d as u32 & 0xffff) << 4;
    let neg = a >= 0x8_0001;
    if neg {
        a = 0x10_0000 - a;
    }
    if spd & 0xf_0000 == 0 {
        let div = i64::from(spd & 0xffff);
        if div != 0 {
            a /= div;
            if a == 0 {
                a = 1;
            }
        }
    }
    if a >= 24577 {
        a = 24576;
    }
    (if neg { -a } else { a }) as i32
}

/// `ccSetDirc(&r, to, spd)` (main 0x001da0b0): `r` turned toward `to`, both
/// through `RAD2DEG`, the result through `DEG2RAD`.
pub fn set_dirc(r: &mut F, to: F, spd: i32) {
    let s = ee::rad2deg(*r);
    let chg = get_dirc_chg(s, ee::rad2deg(to), spd);
    *r = ee::deg2rad(s.wrapping_add(chg as i16));
}

/// The `ccSpcChar` members the AI's manual paths and the event
/// instructions touch, borrowed from the player ([`crate::player::Player`])
/// or a party member (the battle's record of it,
/// [`crate::combat::spc::SpcRec`]).
pub struct SpcRef<'a> {
    /// +0x128 `ai` (none before `ccSPC::Reboot` makes it).
    pub ai: Option<&'a mut Ai>,
    /// +0x40, +0x60.
    pub pos: &'a mut V4,
    pub dirc: &'a mut V4,
    /// +0x08 `condition.dead` (0 alive, 4 a ghost, 5 coming back ...).
    pub dead: &'a mut i16,
    /// +0x7c `skillID`, +0x7e `skillStatus`.
    pub skill_id: &'a mut i16,
    pub skill_status: &'a mut i16,
    /// +0x88, +0x8c, +0x90.
    pub transparency: &'a mut F,
    pub set_transparency: &'a mut F,
    pub trans_dist: &'a mut bool,
    /// +0xe0 bits: 1 `dispSW`, 2 `restraintSW`, 3 `moveFlag`, 4 `stopFlag`,
    /// 5 `runFlag`, 6 `ghostFlag`, 7 `noDeathFlag`; +0xe1 bit 4
    /// `recallFlag`; bits 14-16 `partyFlag` (signed 3-bit).
    pub disp: &'a mut bool,
    pub restraint: &'a mut bool,
    pub move_flag: &'a mut bool,
    pub stop_flag: &'a mut bool,
    pub run_flag: &'a mut bool,
    pub ghost: &'a mut bool,
    pub no_death: &'a mut bool,
    pub recall: &'a mut bool,
    pub party_flag: &'a mut i8,
    /// +0xee `actNum`, +0xf0 `actNumOld`, +0xf4 `anmFlag`, +0xf8 `actCnt`,
    /// +0x100 `transferLag`, +0x110 `cloak`.
    pub act: &'a mut i16,
    pub act_old: &'a mut i16,
    pub anm_flag: &'a mut i16,
    pub act_cnt: &'a mut i16,
    pub transfer_lag: &'a mut i16,
    pub cloak: &'a mut F,
    /// +0x1a0 `bodyHit`.
    pub hit: &'a mut hit::Body,
    /// On its command list (`ccEntryCmnd` / `ccDeleteCmnd`).
    pub listed: &'a mut bool,
    /// `personality->velocity`, the stride `MoveP2P` measures by (0 for a
    /// member, whose walks the battle's `ManualControl` runs).
    pub velocity: F,
}

impl SpcRef<'_> {
    /// `ccSpcChar::CheckControlMode` (0x0059f550): the AI's `manualSW`.
    pub fn manual(&self) -> bool {
        self.ai.as_ref().is_some_and(|a| a.manual_sw)
    }

    fn hit_enable(&mut self, hits: &mut Hits) {
        hits.hit_enable(self.hit);
    }

    fn hit_disable(&mut self, hits: &mut Hits) {
        hits.hit_disable(self.hit);
    }

    /// `ccSpcChar::SetBootStatus(cmd)` (0x0059e950), the registry's
    /// `bootParam` applied as the constructors apply it: bit 2 manual mode
    /// (`ccAI::ManualMode`, when there is an AI - the constructors run
    /// before `ccSPC::Reboot` makes it, which then calls `ManualMode`
    /// itself); bit 0 standing (act 2, the body on the character list when
    /// alive); bit 1 out of sight (act 14, `transferLag` 0, transparency
    /// and `cloak` 0, the body off the list); bit 3 arriving (act 13,
    /// `transferLag` 1, the same).
    pub fn set_boot_status(&mut self, cmd: i32, hits: &mut Hits) {
        if cmd & 4 != 0 {
            let dirc = *self.dirc;
            if let Some(ai) = self.ai.as_deref_mut() {
                ai.manual_mode(&dirc);
            }
        }
        if cmd & 1 != 0 {
            *self.act = 2;
            if *self.dead == 0 {
                self.hit_enable(hits);
            }
        }
        if cmd & 2 != 0 {
            *self.act = 14;
            *self.transfer_lag = 0;
            *self.transparency = 0;
            *self.cloak = 0;
            self.hit_disable(hits);
        }
        if cmd & 8 != 0 {
            *self.act = 13;
            *self.transfer_lag = 1;
            *self.transparency = 0;
            *self.cloak = 0;
            self.hit_disable(hits);
        }
    }

    /// `ccSpcChar::ManualModeAI(ev)` (0x0059eba0): with `ev` 1 and the party
    /// not wiped out, a dead character brought back (`dead` 0, `cloak` 1,
    /// not a ghost, the body on the list) and the down acts 9-11 stood up
    /// (act 2); then `ccAI::ManualMode`.
    pub fn manual_mode_ai(&mut self, ev: i32, annihilated: bool, hits: &mut Hits) {
        if ev == 1 && !annihilated {
            if *self.dead != 0 {
                *self.dead = 0;
                *self.cloak = ONE;
                *self.ghost = false;
                self.hit_enable(hits);
            }
            if matches!(*self.act, 9..=11) {
                *self.act = 2;
                *self.act_old = -1;
            }
        }
        let dirc = *self.dirc;
        if let Some(ai) = self.ai.as_deref_mut() {
            ai.manual_mode(&dirc);
        }
    }

    /// `ccAI::SetRemoteCmd(cmd)` (0x005832e0): the command, `remoteFlag`;
    /// for 0 the heading to keep is the body's own; for 5 a character outside
    /// the party (`partyFlag` 0) is marked leaving (-2).
    pub fn set_remote_cmd(&mut self, cmd: i16) {
        let dirc = self.dirc[2];
        let Some(ai) = self.ai.as_deref_mut() else { return };
        ai.remote_cmd = cmd;
        ai.remote_flag = true;
        if cmd == 0 {
            ai.g_deg = ee::rad2deg(dirc) as u16;
        } else if cmd == 5 && *self.party_flag == 0 {
            *self.party_flag = -2;
        }
    }

    /// `ccAI::SetDircZ` on this character's AI.
    pub fn set_dirc_z(&mut self, dd: u16) {
        match self.ai.as_deref_mut() {
            Some(ai) => ai.set_dirc_z(dd, self.dirc),
            None => self.dirc[2] = ee::deg2rad(dd as i16),
        }
    }

    /// `ccSpcChar::TransferIn` (0x0059ea60): the arrival, act 13 with the
    /// old act 13 too (the animation playing goes on), `actCnt` 0.
    pub fn transfer_in(&mut self) {
        *self.act = 13;
        *self.act_old = 13;
        *self.act_cnt = 0;
    }

    /// `ccSpcChar::TransferOut` (0x0059ea80): leaving through the gate
    /// effect. Shown (`dispSW`) and not faded out: `ConditionAdjustment`
    /// (0x0059ec70: nothing for the living; its handling of the dead is not
    /// ported), act 12 - keeping the animation when it stood in act 2 or 4 -
    /// `anmFlag` and `actCnt` 0, not moving, the body off the list. Off the
    /// command list unless in manual mode.
    pub fn transfer_out(&mut self, hits: &mut Hits) {
        if *self.disp && !ee::eq(*self.cloak, 0) {
            *self.act_old = if *self.dead == 4 || !matches!(*self.act, 2 | 4) { -1 } else { 12 };
            *self.act = 12;
            *self.anm_flag = 0;
            *self.act_cnt = 0;
            *self.move_flag = false;
            self.hit_disable(hits);
        }
        if !self.manual() {
            *self.listed = false;
        }
    }
}

/// What the manual paths read of the rest of the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct BrainsInput {
    /// `checkPartyAnnihilation()`: every member down.
    pub annihilated: bool,
    /// `ccSpcChar::SpcListNum` (+0xe8): the registry slot (0 Kite).
    pub list_num: i32,
    /// `game->inBattle` (+0x58).
    pub in_battle: i32,
    /// The disc's volume ([`piney_battle::party_ai::mode_of`]).
    pub volume: piney_data::volume::Volume,
}

/// What remote command 5 asks of the party (`resignParty` or `disbandSpc`
/// of the character's id), done by the caller on the party manager
/// ([`crate::party::Spcs::leave`]); the character's own `partyFlag` is
/// already -2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyLeave {
    /// `resignParty(id)`: out of its party slot (and `disbandSpc`).
    Resign,
    /// `disbandSpc(id)`.
    Disband,
}

/// `ccAI::ManualControl` (0x00580ef0), the remote command's frame:
///
/// ```text
/// 0  stand: moveFlag, runFlag 0; turn toward gDeg at gRotSp (ccSetDirc);
///    remoteFlag while not stopped there (gDeg read unsigned: a heading of
///    0x8000 or more never counts as reached)
/// 3  the arrival: in act 2 done (command 0); in act 14 TransferIn
/// 4  out through the gate: in act 14 done; else TransferOut unless in 12
/// 5  leaving the party: TransferOut (not in 12 or 14), resignParty or
///    disbandSpc, partyFlag -2
/// 6  out of sight at once: act 14, transparency and cloak 0; done
/// 7  back in sight at once: act 14 to 2 (the body on the list when alive
///    or coming back), transparency and cloak 1, dispSW; done
/// ```
///
/// Commands 1 (running: `runFlag`) and 2 (walking) step toward `gPos` by
/// `MoveP2P` and, within 50, stand with the command done, `remoteFlag` 1
/// and `gDeg` the heading the body had this frame; along the town route
/// (`gPoint` not -1) they do nothing here. Returns what command 5 asks of
/// the party.
pub fn manual_control(ch: &mut SpcRef, hits: &mut Hits) -> Option<PartyLeave> {
    let cmd = ch.ai.as_ref().map_or(-1, |a| a.remote_cmd);
    let mut leave = None;
    match cmd {
        1 | 2 => {
            let (g_point, g_pos) = ch.ai.as_ref().map_or((0, [0; 4]), |a| (a.g_point, a.g_pos));
            if g_point == -1 {
                let dirc = *ch.dirc;
                let r = move_p2p(ch, g_pos, cmd != 2);
                if ee::lt(r, F_50) {
                    *ch.move_flag = false;
                    *ch.run_flag = false;
                    if let Some(ai) = ch.ai.as_deref_mut() {
                        ai.remote_cmd = 0;
                        ai.remote_flag = true;
                        ai.g_deg = ee::rad2deg(dirc[2]) as u16;
                    }
                }
            }
        }
        0 => {
            *ch.move_flag = false;
            *ch.run_flag = false;
            let ai = ch.ai.as_deref_mut().expect("remote command 0 needs the AI");
            let mut z = ch.dirc[2];
            set_dirc(&mut z, ee::deg2rad(ai.g_deg as i16), i32::from(ai.g_rot_sp));
            let reached = i64::from(ai.g_deg) == i64::from(ee::rad2deg(z));
            ai.remote_flag = !(*ch.stop_flag && reached);
            ch.dirc[2] = z;
        }
        3 => {
            if *ch.act == 2 {
                done(ch);
            } else {
                if *ch.act == 14 {
                    ch.transfer_in();
                }
                busy(ch);
            }
        }
        4 => {
            if *ch.act == 14 {
                done(ch);
            } else {
                if *ch.act != 12 {
                    ch.transfer_out(hits);
                }
                busy(ch);
            }
        }
        5 => {
            if *ch.act == 14 {
                return None;
            }
            if *ch.act != 12 {
                ch.transfer_out(hits);
                leave = Some(if *ch.party_flag == 1 { PartyLeave::Resign } else { PartyLeave::Disband });
                *ch.party_flag = -2;
            }
            busy(ch);
        }
        6 => {
            if *ch.act != 14 {
                *ch.act = 14;
                *ch.act_old = -1;
            }
            *ch.transparency = 0;
            *ch.set_transparency = 0;
            *ch.cloak = 0;
            done(ch);
        }
        7 => {
            if *ch.act == 14 {
                *ch.act = 2;
                *ch.act_old = -1;
                if matches!(*ch.dead, 0 | 5) {
                    ch.hit_enable(hits);
                }
            }
            *ch.transparency = ONE;
            *ch.set_transparency = ONE;
            *ch.cloak = ONE;
            *ch.disp = true;
            done(ch);
        }
        _ => {}
    }
    leave
}

/// 50.0, where a remote walk ends.
const F_50: F = 0x4248_0000;

/// `ccSpcChar::CheckAction(n)` (gcmn 0x0059ec80) for the two tests
/// `MoveP2P` makes: 1 may start moving, 2 may turn.
fn check_action(ch: &SpcRef, n: i32) -> bool {
    let (id, st, a) = (*ch.skill_id, *ch.skill_status, *ch.act);
    match n {
        1 => id < 2 && st == 0 && a < 5,
        2 => !matches!(a, 7..=9 | 15 | 16),
        _ => false,
    }
}

/// `ccAI::MoveP2P(pos, p2, runFlag)` (gcmn 0x00582d50): a step toward `p2`.
/// The distance on the ground less the body's radius, truncated to a whole
/// number, is returned; the body turns to face `p2` when it may turn and is
/// not within its stride, starts walking (and running as asked) when it may
/// move and is beyond it, and a body already walking starts running when
/// asked.
pub fn move_p2p(ch: &mut SpcRef, p2: V4, run: bool) -> F {
    let p1 = *ch.pos;
    let mut tmp = *ch.dirc;
    let mut d = ee::vsub(p2, p1);
    let ang = ee::deg2rad(ee::rad2deg(ee::atan2f(d[0], ee::neg(d[1]))));
    d[2] = 0;
    let dist = ee::sqrtf(ee::dot(d, d));
    let r = ee::from_int(ee::to_int(ee::sub(dist, ch.hit.radius)));
    let vel = ch.velocity;
    if check_action(ch, 2) && !ee::lt(r, vel) {
        tmp[2] = ang;
    }
    if *ch.move_flag {
        if run {
            *ch.run_flag = true;
        }
    } else if check_action(ch, 1) && !ee::le(r, vel) {
        *ch.move_flag = true;
        *ch.run_flag = run;
    }
    *ch.dirc = tmp;
    r
}

/// The command done: `remoteCmd` 0, `remoteFlag` clear.
fn done(ch: &mut SpcRef) {
    if let Some(ai) = ch.ai.as_deref_mut() {
        ai.remote_cmd = 0;
        ai.remote_flag = false;
    }
}

fn busy(ch: &mut SpcRef) {
    if let Some(ai) = ch.ai.as_deref_mut() {
        ai.remote_flag = true;
    }
}

/// `ccAI::Brains` (0x0057ca00) for a character whose AI is in manual mode
/// or talking (`talkFlag`), the only ways the event scripts drive it:
///
/// ```text
/// mode 0: nothing (-1)
/// talkFlag: the body turns toward gDeg at 64; 0
/// SysMsgEntry or ReadSysMsg (nothing queued: nothing); levelCheck (manual:
///   only levelOld); partyFlag 1: ChatCommand, else skillMask = 3
/// dead 0: mode 6 back to 1; dead 4/5 as a ghost: mode 6; other deaths skip
///   the control
/// manualSW: ManualControl
/// ChatMessageSender; count++; detourCnt--
/// ```
///
/// A character not in manual mode goes to `ActInTown` and the rest, which
/// are not ported: nothing happens then but the counters. Returns
/// `ManualControl`'s party request.
pub fn brains(ch: &mut SpcRef, hits: &mut Hits, input: &BrainsInput) -> Option<PartyLeave> {
    let party_flag = *ch.party_flag;
    let dead = *ch.dead;
    let ghost = *ch.ghost;
    {
        let ai = ch.ai.as_deref_mut()?;
        if ai.mode == 0 {
            return None;
        }
        if ai.talk_flag {
            let mut z = ch.dirc[2];
            set_dirc(&mut z, ee::deg2rad(ai.g_deg as i16), i32::from(ROT_QUARTER));
            ch.dirc[2] = z;
            return None;
        }
        if party_flag == 1 {
            chat_command(ai, input.list_num);
        } else {
            ai.skill_mask = 3;
        }
    }
    let control = match dead {
        0 => {
            let ai = ch.ai.as_deref_mut().expect("checked above");
            if ai.mode == piney_battle::party_ai::mode_of(input.volume, 6) {
                ai.change_mode(1);
            }
            true
        }
        4 | 5 if ghost => {
            let ai = ch.ai.as_deref_mut().expect("checked above");
            let down = piney_battle::party_ai::mode_of(input.volume, 6);
            if ai.mode != down {
                ai.change_mode(down);
            }
            true
        }
        _ => false,
    };
    let mut leave = None;
    if control && ch.manual() {
        leave = manual_control(ch, hits);
    }
    let ai = ch.ai.as_deref_mut().expect("checked above");
    chat_message_sender(ai, input);
    ai.count = ai.count.wrapping_add(1);
    if ai.detour_cnt > 0 {
        ai.detour_cnt -= 1;
    }
    leave
}

/// `ccAI::ChatCommand` (0x00583d60) with no chat command given (`chatCmd`
/// -1, `chatCmdFlag` 0) out of battle (`spcBattleCondition` 0): only the
/// leader's (`SpcListNum` 0) `strategy` is cleared.
fn chat_command(ai: &mut Ai, list_num: i32) {
    if list_num == 0 && ai.strategy != 0 {
        ai.strategy = 0;
    }
}

/// `ccAI::ChatMessageSender` (0x00586420) for the player under his AI:
/// the arrival's chat line counted down (`arrivalChatCnt`, from 150 in a
/// town); `ChatMessageEnteredTown` (0x0058f810) says nothing in manual
/// mode, the only mode this runs in (the members' lines are
/// `piney_battle::party_chat`'s).
fn chat_message_sender(ai: &mut Ai, input: &BrainsInput) {
    if input.annihilated {
        return;
    }
    ai.chat_request = false;
    if ai.arrival_chat_cnt < 0 {
        return;
    }
    ai.arrival_chat_cnt -= 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirc_chg_quirks() {
        // A quarter of the way, at least 1.
        assert_eq!(get_dirc_chg(0, 1024, 64), 256);
        assert_eq!(get_dirc_chg(0, 3, 64), 1);
        assert_eq!(get_dirc_chg(0, -1024, 64), -256);
        // Past half a turn the other way round.
        assert_eq!(get_dirc_chg(-30000, 30000, 64), -(16 * 5536) / 64);
        // Modes 0x10000 and 0x20000 are not told apart: the whole step,
        // capped at 24576.
        assert_eq!(get_dirc_chg(0, 1000, 0x1_0000), 16_000);
        assert_eq!(get_dirc_chg(0, 3000, -5), 24576);
    }

    #[test]
    fn manual_mode_keeps_the_heading() {
        let mut ai = Ai::new(150, 0);
        let dirc = [0, 0, ee::deg2rad(-16384), 0];
        ai.manual_mode(&dirc);
        assert!(ai.manual_sw && !ai.remote_flag);
        assert_eq!((ai.g_deg, ai.g_rot_sp, ai.remote_cmd), (ee::rad2deg(dirc[2]) as u16, 64, 0));
    }
}
