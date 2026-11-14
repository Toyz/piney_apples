//! `ccAI` (gcmn ai.cpp, 0x260 bytes) as the event scripts drive it: a party
//! character in manual mode (`manualSW`) standing, turning and transferring by
//! remote command. Ported: `ManualMode` (0x00583270), `SetRemoteCmd`
//! (0x005832e0), `SetDircZ` (0x00581500), `ChangeMode` (0x00589740), the
//! `ccSpcChar` transfers (`Brains` and `ManualControl` run as the battle's); `ccSetDirc` / `ccGetDircChg` (main 0x001da0b0 /
//! 0x001d9eb0). Not: the non-manual paths and the AI's message system. The
//! layout and the AI proper are in docs/engine/battle.md.

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

/// `ccGetDircChg(cur, tgt, spd)` (main 0x001d9eb0); the game's switch on
/// `spd & 0xf0000` can never match 1 or 2, so any other rate takes the
/// whole step.
pub use piney_battle::geom::get_dirc_chg;

/// `ccSetDirc(&r, to, spd)` (main 0x001da0b0): `r` turned toward `to`, both
/// through `RAD2DEG`, the result through `DEG2RAD`.
pub fn set_dirc(r: &mut F, to: F, spd: i32) {
    let s = ee::rad2deg(*r);
    let chg = get_dirc_chg(s, ee::rad2deg(to), spd);
    *r = ee::deg2rad(s.wrapping_add(chg as i16));
}

/// The `ccSpcChar` members the event instructions touch, borrowed from
/// Kite or a party member (the battle's record of it,
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

/// What remote command 5 asks of the party (`resignParty` or `disbandSpc`
/// of the character's id), done by the caller on the party manager
/// ([`crate::chara::party::Spcs::leave`]); the character's own `partyFlag` is
/// already -2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyLeave {
    /// `resignParty(id)`: out of its party slot (and `disbandSpc`).
    Resign,
    /// `disbandSpc(id)`.
    Disband,
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
        // A rate of 0 (the game traps): the whole step, not a panic.
        assert_eq!(get_dirc_chg(0, 1000, 0), 16_000);
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
