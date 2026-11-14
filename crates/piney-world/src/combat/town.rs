//! The party in a Root Town: Kite and the members are the battle's
//! characters as in every area ([`Combat::add_kite`], [`Combat::add_member`],
//! area 0), run by the battle's tasks ([`Combat::frame`]: `ccPlayer::Main`,
//! each member's `ccFellow::Main` walking by `ccAI::ActInTown` or, under an
//! event, `ManualControl`) over the town's navigation
//! ([`super::stage::TownNavData`]). What is the town's alone is here.

use super::{Combat, bit};
use crate::ee::V4;
use crate::hit::Hits;

impl Combat {
    /// Member `w` taken off the town (`remove 2 code`, or its task's
    /// delete): off the command lists and the collision, not drawn.
    pub fn drop_member(&mut self, w: usize, hits: &mut Hits) {
        piney_battle::fellow::delete_cmnd(&mut self.scene, w);
        if let Some(s) = self.crew.spc.get_mut(&w)
            && s.body_hit.sw
        {
            let mut b = super::spc::to_body(w, self.kite, &s.body_hit, false);
            hits.hit_disable(&mut b);
            s.body_hit.sw = false;
        }
        self.scene.chars[w].spc_char.flags &= !bit::DISP;
        self.members.retain(|m| m.1 != w);
        self.cast.actors.remove(&w);
    }

    /// The members' drawn stand-ins: each actor's place (its last
    /// `SetMatrix`), for the town's lists.
    pub fn member_pos(&self, id: i32) -> Option<V4> {
        self.who(id).map(|w| self.scene.chars[w].pos)
    }
}
