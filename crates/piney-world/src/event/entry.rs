//! How an event instruction's `type` and code name a character: [`Kind`]
//! (`ccEvent::GetSpc`, `GetNpc` 0x001b2cc0, `GetEnemy`; the NPCs are
//! [`crate::systems::npcs`]).

/// What kind of character an event instruction's `type` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The player's party (`ccEvent::GetSpc`: `ccSpcManager` entry id; a
    /// negative code is a party slot).
    Spc,
    /// A town NPC (`ccEvent::GetNpc`): a merchant (base flags bit 4) when
    /// the code is 29 or 158, else a PC (base flags bit 3).
    Npc,
    /// An enemy (`ccEvent::GetEnemy`: base flags & 0x60).
    Enemy,
    /// A gimmick (the Chaos Gate: `gimmickTbl` row).
    Gimmick,
}
