//! The party's bookkeeping task, `ccThSpc` (gcmn 0x005a0530, priority 48):
//! once a frame, before Kite and the party members move, it levels up the
//! members away from the party, sets the party's strategy, has Kite shout it
//! when a fight starts, and turns `ccGame.inBattle` into the battle condition
//! the party AI reads ([`crate::party_ai::Game::spc_battle_condition`]). Its
//! start also starts `ccThAISystem` ([`crate::party_ai::Crew::tick`]) and
//! builds the path-finding map (docs/engine/battle.md, "A frame of the field").

use piney_data::save::SaveData;

use crate::exp::{self, Party};
use crate::tables::Tables;

/// `saveData` +0x6773 (a signed byte): the party's operation, the chat
/// menu's 7-10.
pub const SAVE_OPERATION: usize = 0x6773;

/// The globals `ccThSpc` keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpcThread {
    /// `spcCheckInBattleOld` (main 0x00378cfc): `inBattle` a frame ago.
    pub in_battle_old: i16,
    /// `spcOpnCnt` (main 0x003782c0, 60 in the executable): frames until
    /// Kite shouts the operation, counting down from 60 while a fight runs.
    pub opn_cnt: i16,
    /// `spcBattleCondition` (main 0x00378d00).
    pub battle_condition: i16,
    /// `partyStrategy` (main 0x00378ce0).
    pub party_strategy: i32,
}

impl Default for SpcThread {
    /// The executable's values.
    fn default() -> Self {
        SpcThread { in_battle_old: 0, opn_cnt: 60, battle_condition: 0, party_strategy: 0 }
    }
}

/// What a frame of `ccThSpc` asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpcOut {
    /// `ccChatMsg::OpenChat(ccChat, member 0, text)`: Kite's chat bubble
    /// naming the operation, the text the pieces
    /// `ccKanjiStrSeparate(chatActionStr, 5)`,
    /// `ccKanjiStrSeparate(chatMenuStr, operation + 1)` and
    /// `ccKanjiStrSeparate(chatActionStr, 6)` joined (game text, read from
    /// the disc by the runtime).
    Shout { operation: i8 },
}

/// `ccSpcSetOperation()` (gcmn 0x005a1930): `partyStrategy` from the
/// save's operation: 8 is 1, 9 is 2, 10 is 3, anything else 0.
pub fn set_operation(save: &SaveData) -> i32 {
    match save.u8(SAVE_OPERATION) as i8 {
        8 => 1,
        9 => 2,
        10 => 3,
        _ => 0,
    }
}

impl SpcThread {
    /// `ccThSpc` up to its loop: the strategy set, then `spcCheckInBattleOld`
    /// is `inBattle` (the task's other set-up is the runtime's: the
    /// condition effects on, `ccThAISystem` started, `SetPathFindingMap`).
    pub fn start(&mut self, save: &SaveData, in_battle: i32) {
        self.party_strategy = set_operation(save);
        self.in_battle_old = in_battle as i16;
    }

    /// One pass of `ccThSpc`'s loop: `ccSpcCheckLevelUp`
    /// ([`exp::level_up_absent`]), `ccSpcSetOperation`,
    /// `ccSpcShoutOperationName` (0x005a17e0), then the battle condition from
    /// `inBattle` and the last frame's (0: 5 after 2, else 0; 1: 1 after 0,
    /// else 2; 2: 3 after 1, else 4). `area` is `ccGame.area`, `in_battle`
    /// `ccGame.inBattle`, `puppet_show` `eventMng->puppetShow` (+0x78c).
    pub fn frame(
        &mut self,
        t: &Tables,
        save: &mut SaveData,
        party: &Party,
        area: i32,
        in_battle: i32,
        puppet_show: bool,
    ) -> Option<SpcOut> {
        exp::level_up_absent(t, save, party, area);
        self.party_strategy = set_operation(save);
        let out = self.shout_operation_name(save, party, area, in_battle, puppet_show);
        let old = self.in_battle_old;
        match in_battle {
            0 => self.battle_condition = if old == 2 { 5 } else { 0 },
            1 => self.battle_condition = if old == 0 { 1 } else { 2 },
            2 => self.battle_condition = if old == 1 { 3 } else { 4 },
            _ => {}
        }
        self.in_battle_old = in_battle as i16;
        out
    }

    /// `ccSpcShoutOperationName()` (gcmn 0x005a17e0): in a field or
    /// dungeon with two or more in the party, a fight (`inBattle` 1) counts
    /// `spcOpnCnt` down to 0, and Kite shouts the operation when it passes
    /// 20 (not while an event's scene runs, which also stops the count);
    /// `inBattle` 0 or 2, the Root Town or Kite alone put it back to 60.
    pub fn shout_operation_name(
        &mut self,
        save: &SaveData,
        party: &Party,
        area: i32,
        in_battle: i32,
        puppet_show: bool,
    ) -> Option<SpcOut> {
        if area == 0 || party.num < 2 {
            self.opn_cnt = 60;
            return None;
        }
        match in_battle {
            0 | 2 => {
                self.opn_cnt = 60;
                None
            }
            1 if !puppet_show => {
                let out = (self.opn_cnt == 20).then(|| SpcOut::Shout { operation: save.u8(SAVE_OPERATION) as i8 });
                if self.opn_cnt > 0 {
                    self.opn_cnt -= 1;
                }
                out
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fight_starts_runs_and_ends() {
        let t = Tables::default();
        let mut save = SaveData::new();
        save.set_u8(SAVE_OPERATION, 9);
        let party = Party { members: [Some(0), Some(1), None], ids: [0, 2, -1], num: 2 };
        let mut s = SpcThread::default();
        s.start(&save, 0);
        assert_eq!(s.party_strategy, 2);
        let mut conds = Vec::new();
        let mut shouts = 0;
        for in_battle in [0, 1, 1, 2, 2, 0, 0] {
            for _ in 0..(if in_battle == 1 { 30 } else { 1 }) {
                shouts += usize::from(s.frame(&t, &mut save, &party, 1, in_battle, false).is_some());
                conds.push(s.battle_condition);
            }
        }
        conds.dedup();
        assert_eq!(conds, [0, 1, 2, 3, 4, 5, 0]);
        // 60 counts down to 20 over the first 40 frames of a fight: the two
        // spells of inBattle 1 here come to 60 frames, one shout.
        assert_eq!(shouts, 1);
    }
}
