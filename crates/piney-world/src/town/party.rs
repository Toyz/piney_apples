//! The party in a Root Town: Kite and the registered characters are the
//! battle's characters in every area, run by the battle's tasks
//! (`ccPlayer::Main`, `ccFellow::Main`, `ccAI::Brains`, `ActInTown`) with the
//! town's navigation ([`crate::combat::stage::TownNavData`]). What the town
//! reads of them by `charTbl` row comes from the battle's scene and cast.

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;

use crate::chara::body::{Body, TRALL};
use crate::combat::Combat;
use crate::ee::{F, V4};
use crate::hit::Hits;

/// The town's party as the battle's characters, by `charTbl` row.
impl Combat {
    /// `ccFellow::Initialize` in a town and `ccSPC::Reboot`'s AI for
    /// registered character `id` (`ccSpcStart[id]`) in registry slot
    /// `list_num`: its body from `file`,
    /// at `pos` facing `rot`, arriving (act 13) after `(rand() % 4) * 5`
    /// frames. None when its body cannot be read.
    #[allow(clippy::too_many_arguments)]
    pub fn build_member(
        &mut self,
        archive: &Arc<Archive>,
        save: &piney_data::save::SaveData,
        id: i32,
        file: &str,
        pos: V4,
        rot: F,
        boot: i32,
        party_flag: i8,
        list_num: i32,
        hits: &mut Hits,
    ) -> Option<usize> {
        if let Some(w) = self.who(id) {
            return Some(w);
        }
        let body = match Body::read(archive, file, TRALL) {
            Ok(mut b) => {
                // ccPlayer::ccPlayer's EquipWeapon.
                if let Some((w, job)) = crate::chara::body::weapon_of(&self.data.t, save, id) {
                    b.equip_weapon(archive, &w, job);
                }
                Rc::new(b)
            }
            Err(e) => {
                tracing::warn!("party member {id}: {e}");
                return None;
            }
        };
        let w = self.add_member(save, id, pos, rot, boot, party_flag, body, hits, 0, list_num);
        Some(w)
    }

    /// Member `id`'s character as the town's frames keep it (Kite's
    /// stand-in for 0): its conditions from the last `CalcReal`.
    pub fn member_char(&self, id: i32) -> Option<&piney_battle::Char> {
        self.who(id).and_then(|w| self.scene.chars.get(w))
    }

    /// The members other than Kite, (`charTbl` row, scene index).
    pub fn party_members(&self) -> impl Iterator<Item = (i32, usize)> + '_ {
        self.members.iter().filter(|m| m.0 != 0).copied()
    }

    /// Member `id`'s record as the event instructions see it.
    pub fn member_rec(&self, id: i32) -> Option<crate::combat::spc::SpcRec> {
        let w = self.who(id)?;
        let c = self;
        let td = c.cast.get(w).is_none_or(|a| a.trans_dist);
        Some(crate::combat::spc::SpcRec::read(&c.scene, &c.crew, w, c.kite, td))
    }

    /// Member `id`'s place and heading.
    pub fn member_place(&self, id: i32) -> Option<(V4, V4)> {
        let w = self.who(id)?;
        let dirc = self.crew.spc.get(&w).map_or([0; 4], |s| s.dirc);
        Some((self.scene.chars[w].pos, dirc))
    }

    /// Member `id`'s drawn stand-in (its body, clip, size, tint).
    pub fn member_actor(&self, id: i32) -> Option<&crate::combat::Actor> {
        self.cast.get(self.who(id)?)
    }

    pub fn member_actor_mut(&mut self, id: i32) -> Option<&mut crate::combat::Actor> {
        let w = self.who(id)?;
        self.cast.get_mut(w)
    }

    /// `ccSPC::ChangeWeapon` on member `id`: the weapon the save's record
    /// has on its model's hands.
    pub fn change_member_weapon(&mut self, archive: &Arc<Archive>, save: &piney_data::save::SaveData, id: i32) {
        let Some((w, job)) = crate::chara::body::weapon_of(&self.data.t, save, id) else { return };
        if let Some(a) = self.member_actor_mut(id) {
            let mut b = (*a.ch.body).clone();
            if b.equip_weapon(archive, &w, job) {
                a.ch.body = Rc::new(b);
                // EquipWeapon's dummies of the new file; the trails kept.
                if let Some(arms) = a.weapon.as_mut() {
                    arms.equip(&a.ch.body, job);
                }
            }
        }
    }

    /// Whether member `id` is on the party's command list.
    pub fn member_listed(&self, id: i32) -> bool {
        self.who(id).is_some_and(|w| self.scene.pc_list.contains(&w))
    }

    /// Member `id`'s base type flags (`spcParam.type`).
    pub fn member_flags(&self, id: i32) -> u32 {
        self.who(id).map_or(0, |w| self.scene.chars[w].ty() as u32)
    }

    /// Member `id` taken off the town.
    pub fn drop_party_member(&mut self, id: i32, hits: &mut Hits) {
        if let Some(w) = self.who(id) {
            self.drop_member(w, hits);
        }
    }
}
