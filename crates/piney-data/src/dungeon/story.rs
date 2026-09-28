//! A story dungeon: `EditDungeon` names a `ROOMDATA` table per (event area,
//! dungeon index); the `DUNGEON` constructor copies it and `Generate` runs
//! `DUNGEON::MakeRealMap` (`INF gcmn.prg:0x005be020`) for every floor. No
//! RNG decides the layout. Which model each room gets
//! (`MakeRoom(ROOMDATA *)`) is not traced yet.

use super::{Cell, DOWN, EditDungeon, MAP, RealMap, RoomData, SIDES, Tables, UP};

/// A story dungeon's floors that have rooms.
#[derive(Clone, Debug)]
pub struct Story {
    pub edit: &'static EditDungeon,
    pub floors: Vec<StoryFloor>,
}

#[derive(Clone, Debug)]
pub struct StoryFloor {
    pub level: usize,
    pub map: RealMap,
    /// The floor's rows, in table order.
    pub rooms: Vec<&'static RoomData>,
    /// The room with [`UP`] (0 if none has it).
    pub up: usize,
    /// The room with [`DOWN`].
    pub down: Option<usize>,
}

/// The story dungeon of (event area, dungeon index), if `EditDungeon` has
/// one: `MakeRealMap` over `tables.edit_floor_count(event)` floors, keeping
/// the floors with rooms.
pub fn story(tables: &Tables, event: i32, index: i32) -> Option<Story> {
    let edit = tables.edit(event, index)?;
    let floors = (0..tables.edit_floor_count(event) as usize)
        .map(|level| real_map(edit, level))
        .filter(|f| !f.rooms.is_empty())
        .collect();
    Some(Story { edit, floors })
}

/// `DUNGEON::MakeRealMap` for one floor: each room of the floor painted
/// with its index, and each door its `dirc` names marked with the room its
/// `next` names.
pub fn real_map(edit: &'static EditDungeon, level: usize) -> StoryFloor {
    let mut floor = StoryFloor { level, map: RealMap::default(), rooms: Vec::new(), up: 0, down: None };
    let cells = &mut floor.map.cells;
    for rd in edit.rooms.iter().filter(|r| r.floor as i64 == level as i64) {
        let (x, y, size, door) = (rd.x, rd.y, rd.size.cells(), rd.size.door());
        for a in 0..size {
            for b in 0..size {
                if let Some(c) = flat(cells, x + a, y + b) {
                    c.here = rd.index as u8;
                }
            }
        }
        let doors = [
            [(x + door, y), (x + door + 1, y)],
            [(x + door, y + size - 1), (x + door + 1, y + size - 1)],
            [(x, y + door), (x, y + door + 1)],
            [(x + size - 1, y + door), (x + size - 1, y + door + 1)],
        ];
        for ((bit, pair), next) in SIDES.into_iter().zip(doors).zip(rd.next) {
            if rd.exits.has(bit) {
                for (cx, cy) in pair {
                    if let Some(c) = flat(cells, cx, cy) {
                        c.door = bit;
                        c.next = next as u8;
                    }
                }
            }
        }
        floor.rooms.push(rd);
        if rd.exits.has(UP) {
            floor.up = rd.index as usize;
        }
        if rd.exits.has(DOWN) {
            floor.down = Some(rd.index as usize);
        }
    }
    floor
}

/// `realmap[x][y]` as the game indexes it, with no bounds check on either
/// index: a cell past the end of a column is the next column's.
fn flat(cells: &mut [Cell], x: i32, y: i32) -> Option<&mut Cell> {
    usize::try_from(x * MAP as i32 + y).ok().and_then(|i| cells.get_mut(i))
}
