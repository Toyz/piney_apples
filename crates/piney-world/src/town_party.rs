//! The party in a Root Town: the registered characters other than Kite as
//! the battle's characters, run by the battle's machinery
//! ([`crate::combat::town`]: `ccFellow::Main`, `ccAI::Brains`, `ActInTown`)
//! with the town's navigation. The event instructions reach them as they
//! reach the field's ([`TownChars`], [`crate::combat::spc::SpcRec`]); what
//! the town reads of them comes from the battle's scene and cast.

use std::rc::Rc;
use std::sync::Arc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;

use crate::ai::SpcRef;
use crate::body::{Body, TRALL};
use crate::camera::Camera;
use crate::combat::spc::BattleChars;
use crate::combat::stage::TownNav;
use crate::combat::{BattleData, Combat, Look};
use crate::ee::{self, F, V4};
use crate::hit::Hits;
use crate::navi::NaviMap;
use crate::party::{SpcChars, Spcs};
use crate::player::Player;

/// `naviPointNameTable` (INF gcmn 0x00653c80, `world::navi_points`): eight
/// names, the first none.
const NAVI_POINTS: u32 = 8;

/// The party of a Root Town.
pub struct TownParty {
    /// The members (and Kite's stand-in) as the battle's characters.
    pub combat: Combat,
    /// `ccSetNaviMap`'s landmarks, with their links and main lines.
    pub navi: NaviMap,
    /// `naviPointNameTable`'s dummies by name (0 none).
    pub points: Vec<V4>,
    /// Frames the party has run (`ccSys.count` for the rules' timers).
    pub count: u32,
    /// The chat balloons the members' `ChatMessageSender` opened since the
    /// town last took them: (`charTbl` row, text).
    pub chats: Vec<(i32, Vec<u8>)>,
    /// The members' `effTransfer` (false) and `effWarpTransfer` (true)
    /// since the town last took them: (`charTbl` row, warp).
    pub transfers: Vec<(i32, bool)>,
}

/// Kite and the party members as [`SpcChars`]: id 0 the town's player,
/// any other the battle's record of that `charTbl` row, written back by
/// [`TownParty::finish`].
pub struct TownChars<'a> {
    pub player: &'a mut Player,
    pub members: BattleChars,
}

/// What a party frame in a town reads and moves of the town.
pub struct TownFrame<'a> {
    pub hits: &'a mut Hits,
    pub camera: &'a mut Camera,
    pub save: &'a mut piney_data::save::SaveData,
    pub rand: &'a mut crate::Rand,
    /// `ccMenuCtrl::CheckMenuType()`, `forbid`.
    pub menu_type: i32,
    pub menu_forbid: bool,
    /// `ccGame.server`.
    pub server: i32,
}

impl SpcChars for TownChars<'_> {
    fn spc(&mut self, id: i32) -> Option<SpcRef<'_>> {
        if id == 0 {
            return Some(self.player.spc());
        }
        self.members.spc(id)
    }

    fn pos(&self, id: i32) -> Option<V4> {
        if id == 0 {
            return Some(self.player.body.pos);
        }
        self.members.pos(id)
    }
}

impl TownParty {
    /// Town `town`'s party machinery (0 Mac Anu), over its scene file's
    /// dummies (`ccSetNaviMap`, `naviPointNameTable`); `exe` is the
    /// executable with GCMN.PRG over it.
    pub fn new(iso: &mut Iso, town: usize, file: &SceneFile) -> Result<TownParty> {
        let data = Rc::new(BattleData::read(iso)?);
        let mut combat = Combat::new(data, 1);
        let volume = iso.volume()?;
        let navi = NaviMap::read(volume, town, file)?;
        combat.keep.town = piney_battle::navi::TownMap {
            num: navi.num,
            marks: navi.marks.iter().map(|m| piney_battle::navi::Landmark { pos: m.pos, name: m.name }).collect(),
        };
        let mut points = vec![ee::VF0; NAVI_POINTS as usize];
        for (k, p) in points.iter_mut().enumerate().skip(1) {
            let Some(name) = piney_data::tables::world::of(volume).navi_points().get(k).copied().flatten() else {
                continue;
            };
            if let Some(d) = file.ccs.find_object(name).and_then(|o| file.scene.dummies.get(&o)) {
                *p = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ee::ONE];
            }
        }
        Ok(TownParty { combat, navi, points, count: 0, chats: Vec::new(), transfers: Vec::new() })
    }

    /// The town's navigation for the members' frames.
    pub fn nav(&self) -> TownNav<'_> {
        TownNav { map: &self.navi, points: &self.points }
    }

    /// The members' records (not Kite's stand-in) lent with Kite as
    /// [`TownChars`].
    pub fn read_chars<'a>(&self, player: &'a mut Player) -> TownChars<'a> {
        let c = &self.combat;
        let td = |w: usize| c.cast.get(w).is_none_or(|a| a.trans_dist);
        let ids: Vec<(i32, usize)> = c.members.iter().filter(|m| m.0 != 0).copied().collect();
        TownChars { player, members: BattleChars::read(&c.scene, &c.crew, &ids, c.kite, &td) }
    }

    /// The records of [`TownParty::read_chars`] written back.
    pub fn finish(&mut self, members: BattleChars) {
        let c = &mut self.combat;
        let cast = &mut c.cast;
        members.finish(&mut c.scene, &mut c.crew, &mut |w, t| {
            if let Some(a) = cast.get_mut(w) {
                a.trans_dist = t;
            }
        });
        for &(_, w) in c.members.iter().filter(|m| m.0 != 0) {
            piney_battle::fellow::sync_in(&c.scene, &mut c.crew, w);
        }
    }

    /// The registry, Kite and the members as [`SpcChars`] with the town's
    /// collision, lent to `f`; the members' records written back after,
    /// then the battle's party (`SetParty`) brought up to date.
    pub fn with_chars<R>(
        &mut self,
        player: &mut Player,
        spcs: &mut Spcs,
        hits: &mut Hits,
        f: impl FnOnce(&mut Spcs, &mut TownChars, &mut Hits) -> R,
    ) -> R {
        let mut chars = self.read_chars(player);
        let r = f(spcs, &mut chars, hits);
        self.finish(chars.members);
        self.combat.set_party(spcs.party());
        r
    }

    /// The members' task frames (`ccThFellowNN`, priority 50) after Kite's
    /// (`ccThPlayer`): his stand-in brought up to date, then `ccThSpc`,
    /// `ccThAISystem` and each member's `ccFellow::Main`
    /// ([`Combat::town_frame`]); a leaver its task let go the frame before
    /// (`exitFlag`: `expulsionSpc`, the task's delete) first taken off the
    /// town.
    pub fn frame(&mut self, player: &mut Player, spcs: &mut Spcs, x: &mut TownFrame) {
        // A leaver's task wakes to the exit an earlier frame's Main set
        // (ccThFellowNN, gcmn 0x0041ed60) and ends instead of running Main:
        // expulsionSpc(listNum), then its delete (DelSpc unless partyFlag 1).
        let exit = piney_battle::fellow::flag::EXIT;
        let gone: Vec<(i32, usize)> =
            self.members().filter(|&(_, w)| self.combat.scene.chars[w].spc_char.flags & exit != 0).collect();
        for (id, w) in gone {
            let pf = crate::party::party_flag_bits(self.combat.scene.chars[w].party_flag);
            if let Some(list_num) = spcs.find(id) {
                self.with_chars(player, spcs, x.hits, |spcs, chars, _| spcs.expulsion(list_num, pf, chars));
            }
            self.drop_member(id, x.hits);
        }
        self.combat.mirror_leader(player);
        let annihilated = {
            let mut chars = self.read_chars(player);
            let a = spcs.annihilated(&mut chars);
            self.finish(chars.members);
            a
        };
        let puppet_show = x.camera.puppet_show;
        let nav_map = std::mem::take(&mut self.navi);
        let points = std::mem::take(&mut self.points);
        {
            let mut t = crate::combat::town::TownTasks {
                hits: &mut *x.hits,
                camera: &mut *x.camera,
                save: &mut *x.save,
                spcs: &mut *spcs,
                nav: TownNav { map: &nav_map, points: &points },
                menu_type: x.menu_type,
                menu_forbid: x.menu_forbid,
                puppet_show,
                count: self.count,
                annihilated,
                server: x.server,
            };
            self.combat.rand = piney_battle::rand::Rand(x.rand.0);
            self.combat.town_frame(&mut t);
            // A town has no fights and no items used by the members' AI;
            // nothing carries out a use queued here.
            self.combat.member_items.clear();
            x.rand.0 = self.combat.rand.0;
        }
        self.navi = nav_map;
        self.points = points;
        self.count = self.count.wrapping_add(1);
        for s in &self.combat.shows {
            if let crate::combat::Show::Chat(w, text) = s
                && let Some(&(id, _)) = self.combat.members.iter().find(|m| m.1 == *w)
            {
                self.chats.push((id, text.clone()));
            }
            // ccFellow::Action's effTransfer / effWarpTransfer.
            if let crate::combat::Show::Member(w, o) = s
                && let Some(&(id, _)) = self.combat.members.iter().find(|m| m.1 == *w)
            {
                match o {
                    piney_battle::fellow::Out::Transfer => self.transfers.push((id, false)),
                    piney_battle::fellow::Out::WarpTransfer => self.transfers.push((id, true)),
                    _ => {}
                }
            }
        }
        self.combat.shows.clear();
        self.combat.set_party(spcs.party());
    }

    /// `ccFellow::Initialize` in a town and `ccSPC::Reboot`'s AI for
    /// registered character `id` (`ccSpcStart[id]`) in registry slot
    /// `list_num`: its body from `file`,
    /// at `pos` facing `rot`, arriving (act 13) after `(rand() % 4) * 5`
    /// frames. None when its body cannot be read.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
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
        rand: &mut crate::Rand,
    ) -> Option<usize> {
        if let Some(w) = self.combat.member(id) {
            return Some(w);
        }
        let body = match Body::read(archive, file, TRALL) {
            Ok(mut b) => {
                // ccPlayer::ccPlayer's EquipWeapon.
                if let Some((w, job)) = crate::body::weapon_of(&self.combat.data.t, save, id) {
                    b.equip_weapon(archive, &w, job);
                }
                Rc::new(b)
            }
            Err(e) => {
                eprintln!("party member {id}: {e}");
                return None;
            }
        };
        self.combat.rand = piney_battle::rand::Rand(rand.0);
        let w = self.combat.add_member(save, id, pos, rot, boot, party_flag, body, hits, 0, list_num);
        rand.0 = self.combat.rand.0;
        Some(w)
    }

    /// Member `id`'s scene index.
    pub fn member(&self, id: i32) -> Option<usize> {
        self.combat.member(id)
    }

    /// Member `id`'s character as the town's frames keep it (Kite's
    /// stand-in for 0): its conditions from the last `CalcReal`.
    pub fn char(&self, id: i32) -> Option<&piney_battle::Char> {
        self.member(id).and_then(|w| self.combat.scene.chars.get(w))
    }

    /// The members other than Kite, (`charTbl` row, scene index).
    pub fn members(&self) -> impl Iterator<Item = (i32, usize)> + '_ {
        self.combat.members.iter().filter(|m| m.0 != 0).copied()
    }

    /// Member `id`'s record as the event instructions see it.
    pub fn rec(&self, id: i32) -> Option<crate::combat::spc::SpcRec> {
        let w = self.member(id)?;
        let c = &self.combat;
        let td = c.cast.get(w).is_none_or(|a| a.trans_dist);
        Some(crate::combat::spc::SpcRec::read(&c.scene, &c.crew, w, c.kite, td))
    }

    /// Member `id`'s place and heading.
    pub fn place(&self, id: i32) -> Option<(V4, V4)> {
        let w = self.member(id)?;
        let dirc = self.combat.crew.spc.get(&w).map_or([0; 4], |s| s.dirc);
        Some((self.combat.scene.chars[w].pos, dirc))
    }

    /// Member `id`'s drawn stand-in (its body, clip, size, tint).
    pub fn actor(&self, id: i32) -> Option<&crate::combat::Actor> {
        self.combat.cast.get(self.member(id)?)
    }

    pub fn actor_mut(&mut self, id: i32) -> Option<&mut crate::combat::Actor> {
        let w = self.member(id)?;
        self.combat.cast.get_mut(w)
    }

    /// `ccSPC::ChangeWeapon` on member `id`: the weapon the save's record
    /// has on its model's hands.
    pub fn change_weapon(&mut self, archive: &Arc<Archive>, save: &piney_data::save::SaveData, id: i32) {
        let Some((w, job)) = crate::body::weapon_of(&self.combat.data.t, save, id) else { return };
        if let Some(a) = self.actor_mut(id) {
            let mut b = (*a.ch.body).clone();
            if b.equip_weapon(archive, &w, job) {
                a.ch.body = Rc::new(b);
            }
        }
    }

    /// Whether member `id` is on the party's command list.
    pub fn listed(&self, id: i32) -> bool {
        self.member(id).is_some_and(|w| self.combat.scene.pc_list.contains(&w))
    }

    /// Member `id`'s base type flags (`spcParam.type`).
    pub fn flags(&self, id: i32) -> u32 {
        self.member(id).map_or(0, |w| self.combat.scene.chars[w].ty() as u32)
    }

    /// Member `id` taken off the town.
    pub fn drop_member(&mut self, id: i32, hits: &mut Hits) {
        if let Some(w) = self.member(id) {
            self.combat.drop_member(w, hits);
        }
    }

    /// The members as the town draws them: each drawn stand-in with its
    /// body, where its last frame set it, at its transparency and blend.
    pub fn draw(
        &self,
        layers: &mut piney_desktop::layers::Layers,
        to_screen: glam::Mat4,
        lights: &crate::town::TownLights,
    ) {
        for (_, w) in self.members() {
            let Some(a) = self.combat.cast.get(w) else { continue };
            if !a.drawn || !matches!(a.look, Look::Member(_)) {
                continue;
            }
            let shaded = a.ch.hit_attribute & crate::char::SHADED != 0;
            crate::draw::char_shadow(layers, a.alpha, a.ch.height, |layers| {
                a.ch.body.draw_char_fog(
                    layers,
                    to_screen,
                    &a.ch.play,
                    a.ch.pos,
                    a.ch.dirc,
                    ee::f(a.alpha),
                    lights,
                    shaded,
                    &a.swaps,
                    a.ch.blend.into(),
                )
            });
        }
    }
}
