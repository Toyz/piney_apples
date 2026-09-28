//! The story dungeons' event rooms: `ROOMDATA` rows of `room_type` 16 and
//! up, which `MakeRoom(ROOMDATA *)` (`INF gcmn.prg:0x005bcbb0`), `SetRoom`
//! (0x005c1ca0) and `GotoNextRoom` (0x005c9e10) treat apart.
//!
//! - 16-24 and 34: no room. `MakeRoom` stores the centre and deletes the
//!   anm; walking into one is `GotoNextRoom`'s business (a way out of the
//!   dungeon for the areas that have one).
//! - 25-35 but 27 and 34: a room from a scene file of its own
//!   ([`EventRoom`]), loaded into `DUNGEON.spccs` (+0xd3888).
//! - 15, 27 and 36 on: an empty anm.
//!
//! The names are the code's own constants (lui/addiu pairs in those
//! functions), not a table.

/// A room of type 25-35 built from its own scene file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRoom {
    /// The scene file `MakeRoom` loads (`ccStream::GetCCSAdrs`).
    pub ccs: &'static str,
    /// The Anime chunk `MakeRoom` and `SetRoom` play from it.
    pub anm: &'static str,
    /// The `LGT_` object `SetRoom` stands at `750 (x + 4, y + 4)` and adds
    /// to the light group, the ambient then taken from the anm
    /// (`GetAmbient`, `SetAmbient`); none for types 26 and 35, whose
    /// `SetRoom` reads the anm's ambient and never sets it.
    pub light: Option<&'static str>,
    /// `rotate[f][i]` as `SetRoom` leaves it (float bits): pi for type 26,
    /// else 0.
    pub rotate: u32,
}

/// `SetRoom`'s `SetFog(32767, 65536, 0, 100, 0)` for every event room:
/// near, far, the near rate, the far rate (bits), and colour 0.
pub const EVENT_ROOM_FOG: [u32; 4] = [0x46ff_fe00, 0x4780_0000, 0, 0x42c8_0000];

/// What `MakeRoom(ROOMDATA *)` (cases at 0x005bd000-0x005bd4d8) and
/// `SetRoom` (jump table @5452, 0x006f9890) build for a row of type `ty` in
/// the story area `area` (`WORLD_MAN.eventAreaNumber`): type 25 is Aura's
/// shrine of area 23 (`se1_3`) or of area 25 (`se1_4`), nothing elsewhere.
pub fn event_room(ty: i32, area: i32) -> Option<EventRoom> {
    const ZERO: u32 = 0;
    const PI: u32 = 0x4049_0fdb;
    let r = |ccs, anm, light, rotate| Some(EventRoom { ccs, anm, light, rotate });
    match ty {
        25 if area == 23 => r("se1_3", "ANM_se1_3_1a", Some("LGT_se1_3lig1"), ZERO),
        25 if area == 25 => r("se1_4", "ANM_se1_4_1a", Some("LGT_se1_3lig1"), ZERO),
        26 => r("se1_7_2", "ANM_se1_7sh2a", None, PI),
        28 => r("se1_4", "ANM_se1_4_1a", Some("LGT_se1_3lig1"), ZERO),
        29 => r("se2_2", "ANM_se2_2_1a", Some("LGT_se2_2lig1"), ZERO),
        30 => r("se3_1", "ANM_se3_1_1a", Some("LGT_se3_1lig1"), ZERO),
        31 => r("se3_3", "ANM_se3_3_1a", Some("LGT_se3_3lig1"), ZERO),
        32 => r("se4_2", "ANM_se4_2_1a", Some("LGT_se4_2lig1"), ZERO),
        33 => r("se4_6", "ANM_se4_6_1a", Some("LGT_se4_6lig1"), ZERO),
        35 => r("se3_5_2", "ANM_se3_5_2a", None, ZERO),
        _ => None,
    }
}

/// Where `GotoNextRoom` stands the party in a story area's event room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrival {
    /// A temporary anm of the room at its centre: its `OBJ_user_point`'s
    /// world position (w 1); `WORLD_MAN.specialRoom` 0.
    UserPoint(&'static str),
    /// The ordinary doorway, after a temporary anm of the room is made and
    /// deleted (area 16).
    Door(&'static str),
    /// The spccs chunk `DMY_marker01`'s decoded position (x, y, z, 1) plus
    /// the centre (x, y, 0, 1), so w 2 (area 91).
    Marker(&'static str, &'static str),
    /// No room: `GotoNextRoom` answers -255 and `WORLD_MAN::Enter` changes
    /// the area ([`exit_field`]).
    Leave,
}

/// `GotoNextRoom`'s branch for its `isEventArea` (the dispatch at
/// 0x005ca1bc): the room types it looks for at the room entered (a row
/// of type 16 or more on that floor with that index; empty: any), the area
/// its `ccSaveData::CheckAreaBan(area, game.dungeon, level, next)` names
/// (a banned room rebuilds the room left and answers -100), and the
/// arrival. Areas 16 and 91 check area 108's bans.
pub fn event_branch(area: i32) -> Option<(&'static [i32], i32, Arrival)> {
    use Arrival::*;
    Some(match area {
        108 | 73 | 47 | 66 | 46 | 27 => (&[], area, Leave),
        91 => (&[35], 108, Marker("ANM_se3_5_2a", "DMY_marker01")),
        16 => (&[26], 108, Door("ANM_se1_7sh2a")),
        101 => (&[32], 101, UserPoint("ANM_se4_2_1a")),
        77 => (&[31], 77, UserPoint("ANM_se3_3_1a")),
        71 => (&[30], 71, UserPoint("ANM_se3_1_1a")),
        48 => (&[29], 48, UserPoint("ANM_se2_2_1a")),
        25 => (&[28, 25], 25, UserPoint("ANM_se1_4_1a")),
        23 => (&[25], 23, UserPoint("ANM_se1_3_1a")),
        _ => return None,
    })
}

/// `WORLD_MAN::Enter` on -255 (main 0x0019e2cc): the field
/// `ChangeArea(1, n)` takes the party to, by `WORLD_MAN.eventAreaNumber`.
/// Area 47 goes to field 9 on volume 2 and 10 otherwise. Area 27's call
/// falls through into area 46's `ChangeArea(1, 2)`, but the first call's
/// `ChangeRequest(6, 7)` sleeps the calling task in
/// `ccSleepNoSleepThread(1, 1)`, so the second never runs: field 1.
pub fn exit_field(area: i32, volume: u32) -> Option<i32> {
    Some(match area {
        66 => 67,
        108 => 9,
        47 if volume == 2 => 9,
        47 => 10,
        73 => 4,
        46 => 2,
        27 => 1,
        _ => return None,
    })
}

/// `DUNGEON::RoomSelect`'s own branches (gcmn 0x005c95c0): in area 71 a
/// row of type 30, in 77 of type 31, stands the party at the room's
/// `OBJ_user_point` (`WORLD_MAN.specialRoom` 0).
pub fn room_select_type(area: i32) -> Option<i32> {
    match area {
        71 => Some(30),
        77 => Some(31),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infections_event_rooms() {
        // Aura's shrines in areas 23 and 25, the room of area 16, 91's.
        assert_eq!(event_room(25, 23).unwrap().ccs, "se1_3");
        assert_eq!(event_room(25, 25).unwrap().ccs, "se1_4");
        assert_eq!(event_room(25, 14), None);
        assert_eq!(event_room(26, 16).unwrap().rotate, 0x4049_0fdb);
        assert_eq!(event_room(34, 66), None);
        assert_eq!(event_branch(27), Some((&[][..], 27, Arrival::Leave)));
        assert_eq!(exit_field(27, 1), Some(1));
        assert_eq!(exit_field(47, 1), Some(10));
        assert_eq!(exit_field(14, 1), None);
    }
}
