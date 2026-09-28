//! The entry control (`ccThEntryCtrl`, gcmn entctrl.cpp, priority 64): the
//! characters and gimmicks a Root Town places - its merchants, walking PCs
//! and Chaos Gate - each a `ccEntryObj` run once a frame after the player.
//!
//! [`NpcCtx`] is what an entry's task reads and writes of the rest of the
//! frame; [`Kind`] and a code name a character the way the event scripts
//! do (`ccEvent::GetNpc` 0x001b2cc0: the NPC whose base parameters' id,
//! +0x0c, is the code).

use crate::camera::Cam;
use crate::char::View;
use crate::ee::V4;
use crate::hit::Hits;

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

/// The rest of the frame as an entry's task sees it.
pub struct NpcCtx<'a> {
    /// Kite: `plw->pos` and `plw->dirc` after his task this frame.
    pub player: V4,
    pub player_dirc: V4,
    /// The camera after `CameraPosSet`: for `ccChar::Draw`'s fade and the
    /// view cone.
    pub view: View,
    pub cam: &'a Cam,
    /// The town's collision mesh.
    pub hits: &'a mut Hits,
    /// newlib's `rand()`.
    pub rand: &'a mut crate::Rand,
}

/// An entry-controlled town character (a merchant, a walking PC): what the
/// world asks of it. `ccEntryCtrl` keeps them on its NPC list and runs each
/// one's `ccEntryObj::routine` and class `main` once a frame.
pub trait Npc {
    /// Its `npcTbl` row (the base parameters' id, +0x0c): what events call
    /// the NPC's code.
    fn code(&self) -> i32;
    /// The base parameters' type flags (+0x08).
    fn flags(&self) -> u32;
    fn char(&self) -> &crate::char::Char;
    fn char_mut(&mut self) -> &mut crate::char::Char;
    /// One frame: `ccEntryObj::routine` and the class's `main`.
    fn step(&mut self, ctx: &mut NpcCtx);
    /// On a command list this frame (`ccEntryCmnd`): it can be targeted and
    /// spoken to.
    fn listed(&self) -> bool;
    /// `affectFunc` (`ccChar` +0x94): what the menu's command `cmd` does to
    /// it while it is spoken to (the merchants' `breederInfluence`, the
    /// PCs' `rTownNPCInfluence`).
    fn influence(&mut self, _cmd: i16) {}
    /// What the talk shows, once the menu opens: the message table's
    /// address (`base->msg`) and the line, when the NPC chooses one.
    fn talk_line(&self) -> Option<(u32, i32)> {
        None
    }
    /// An event's NPC instruction (`npc_act`, `npc_walk_*`, a gradual
    /// `npc_turn` or `npc_face`) through the NPC's own event mode, with the
    /// marker it names (position, heading) and the character it faces:
    /// true when handled. The world does puts and instant turns itself
    /// when this declines.
    fn command(
        &mut self,
        _c: &piney_event::host::NpcCommand,
        _marker: Option<(V4, crate::ee::F)>,
        _target: Option<V4>,
    ) -> bool {
        false
    }
}
