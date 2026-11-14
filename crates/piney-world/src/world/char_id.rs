//! A character as the world names it. The game names one by its `ccChar`
//! pointer: `ccCheckTarget` (gcmn 0x00519920) walks `cmndPcRoot`,
//! `cmndEneRoot` and `cmndObjRoot` for it, the menus and balloons hold it.
//! Here the name says which list holds the character; the world resolves
//! it ([`super::World::char_place`] and the rest).

/// Where a character is, as the game's pointer would find it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharId {
    /// A party member (`ccSpc`), by its `charTbl` row: on the battle's
    /// scene in every area, whatever its index there.
    Member(i32),
    /// Any other character on the battle's scene, by its index: a field's
    /// enemies, gimmicks, portals and its event NPCs.
    Scene(usize),
    /// The riding Grunty (`pcgs`), the battle's ride object.
    Ride,
    /// One of the area's own NPCs on the entry control's NPC list (the
    /// cast's), by its base id: `ccEvent::GetNpc`'s code.
    Npc(i32),
    /// One of the area's own objects on the entry control's object list
    /// (the cast's), by its `gimmickTbl` row: `ccSetChaosGate`'s
    /// `entryObject` (gcmn 0x00458e98) puts a town's Chaos Gate there.
    Object(i16),
}

impl CharId {
    /// The name as a `u32` for the menus and the scripts (`CharInfo`'s
    /// and `CharRef`'s handle): the list in the top byte, never 0.
    pub fn handle(self) -> u32 {
        let (list, n) = match self {
            CharId::Member(id) => (1, id as u32),
            CharId::Scene(i) => (2, i as u32),
            CharId::Ride => (3, 0),
            CharId::Object(row) => (4, row as u32),
            CharId::Npc(code) => (5, code as u32),
        };
        (list << 24) | (n & 0x00ff_ffff)
    }

    /// The character a [`CharId::handle`] names.
    pub fn from_handle(h: u32) -> Option<CharId> {
        let n = h & 0x00ff_ffff;
        Some(match h >> 24 {
            1 => CharId::Member(n as u16 as i16 as i32),
            2 => CharId::Scene(n as usize),
            3 => CharId::Ride,
            4 => CharId::Object(n as u16 as i16),
            5 => CharId::Npc(n as i32),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CharId;

    #[test]
    fn handles_are_distinct_never_zero_and_round_trip() {
        let ids = [CharId::Member(0), CharId::Scene(0), CharId::Ride, CharId::Object(16), CharId::Npc(16)];
        for (i, a) in ids.iter().enumerate() {
            assert_ne!(a.handle(), 0);
            assert_eq!(CharId::from_handle(a.handle()), Some(*a));
            for b in &ids[i + 1..] {
                assert_ne!(a.handle(), b.handle());
            }
        }
        assert_eq!(CharId::from_handle(0), None);
    }
}
