//! The characters as the event scripts see and drive them: `piney-event` asks
//! its host for markers, the command target and the characters, and hands it
//! the character instructions ([`NpcCommand`], [`PcCommand`], `entry`,
//! `remove`), which the host forwards to these [`World`] methods. A character
//! is named by an event `type` and `code` ([`crate::event_kind`]), a place by
//! a marker (`markerEvTbl`, gcmn 0x00317da0, in a Root Town). Puts, instant
//! turns, facing, the command target and talk are here; NPCs' walks go to
//! their own event mode. False for what is not done (docs/engine/events.md).

#[doc(alias = "ccThEntryCtrl")]
pub mod entry;
#[doc(alias = "ccThGameOver")]
pub mod gameover;
#[doc(alias = "ccChgate")]
pub mod gate;
pub mod talk;

use crate::chara::party;
use crate::ee::{self, F, ONE, V4};
use crate::event::entry::Kind;

/// `markerEvTbl` (INF main 0x00317da0, `world::markers`): 33 dummy names.
pub const MARKERS: i16 = 33;

/// `markerEvTbl[n]` as a dummy of `file` (a town's, or an event map's for
/// `marker_pos` in a field): its position (w 1) and its rotation's z, bits.
pub fn marker_in(
    file: &piney_desktop::assets::SceneFile,
    volume: piney_data::volume::Volume,
    n: i16,
) -> Option<(V4, F)> {
    marker_dummy_in(file, volume, n).map(|(pos, rot)| (pos, rot[2]))
}

/// [`marker_in`] with the dummy's whole rotation (+0x20, radians, w 0), as
/// `ccEntryEventMng` copies it into an event NPC's `dirc`.
pub fn marker_dummy_in(
    file: &piney_desktop::assets::SceneFile,
    volume: piney_data::volume::Volume,
    n: i16,
) -> Option<(V4, V4)> {
    if !(0..MARKERS).contains(&n) {
        return None;
    }
    let name = piney_data::tables::world::of(volume).markers.get(n as usize)?.to_string();
    dummy_in(file, &name)
}

/// The dummy `name` of `file` as `Decode_DummyPos` leaves its chunk: its
/// position (w 1) and its whole rotation (+0x20, radians, w 0).
pub fn dummy_in(file: &piney_desktop::assets::SceneFile, name: &str) -> Option<(V4, V4)> {
    let obj = file.ccs.find_object(name)?;
    let d = file.scene.dummies.get(&obj)?;
    let pos = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE];
    let [x, y, z] = d.rot.map_or([0; 3], |r| piney_data::anim::const_radians([r.x, r.y, r.z].map(f32::to_bits)));
    Some((pos, [x, y, z, 0]))
}

/// Where `ccEntryEventMng` puts an event's town NPC (main 0x001cbf10 on
/// Mutation): `pos` (+0x40) and `dirc` (+0x60) from [`marker_dummy_in`], or
/// both `vf0` for a marker below 0; None for a marker with no dummy.
pub fn event_marker_in(
    file: &piney_desktop::assets::SceneFile,
    volume: piney_data::volume::Volume,
    marker: i16,
) -> Option<(V4, V4)> {
    if marker < 0 { Some((ee::VF0, ee::VF0)) } else { marker_dummy_in(file, volume, marker) }
}

/// `ccGetDirc(from, to)` (0x001d9ce0): the heading that faces `to` from
/// `from` - `pi/2 + atan2f(dy, dx)` wrapped into (-pi, pi] - as the 16-bit
/// angle `pc_face` hands `ccAI::SetDircZ` (`RAD2DEG`).
pub fn dirc_to(from: V4, to: V4) -> i16 {
    ee::rad2deg(piney_battle::geom::get_dirc(from, to))
}

impl crate::town::cast::TownCast {
    /// `ccEntryEventMng`'s step for one of `eventMng.entry[]` the town
    /// places itself (the event's `entry type code marker param`): a town
    /// NPC (types 3 and 4, `npcTbl` row `code`: a PC or the Administrator)
    /// made and set at the marker, with the entry control's set-up
    /// ([`World::place_entries`]) or at once after it. The party's entries
    /// (0-2) are the registry's, as in every area; false for them and for
    /// objects and enemies (5-7).
    pub fn entry(&mut self, world: &mut crate::world::World, ty: i16, code: i16, marker: i16, param: i16) -> bool {
        match crate::event_kind(ty) {
            Some(Kind::Npc) => self.entry_npc(world, ty, code, marker, param),
            _ => false,
        }
    }

    /// `remove type code` (`ccEvent::Execute` 0x001aa290): type 2 takes a
    /// party member out of the party (`ccParty::DelMember` of its slot) and
    /// the registry (`ccSPC::DelSpc` of each slot with its id), and the port
    /// takes it off the field; types 5/6 delete an enemy or object
    /// (`ccEntryCtrl::deleteEnemy`) and -1 every object
    /// (`deleteAllObject`), neither of which the town has yet; other types
    /// do nothing.
    pub fn remove(&mut self, world: &mut crate::world::World, ty: i16, code: i16) {
        if ty == 2 {
            let code = i32::from(code);
            world.with_party(|spcs, chars, _| {
                if let Some(s) = spcs.member_id.iter().position(|&m| m == code) {
                    spcs.del_member(s as i32, chars);
                }
            });
            for i in 0..party::REGISTRY {
                if world.spcs.registry[i].id == code {
                    world.spcs.del_spc(i);
                }
            }
            world.combat.drop_party_member(code, &mut world.area.core_mut().hits);
            world.combat.set_party(world.spcs.party());
        }
    }
}
