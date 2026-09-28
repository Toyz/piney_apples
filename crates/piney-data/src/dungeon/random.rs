//! A random dungeon: `DUNGEON::Generate` (`INF gcmn.prg:0x005c12a0`) and
//! what it calls, as `tools/dungeon.py`'s `Generator` models them.
//!
//! ```text
//! floors = levelMax (4 if the first keyword is word 131; types 8, 9: 1)
//! for each floor:
//!   floorRoomNum = fieldrand(5) + roomMax;  >= 15 becomes 12
//!   MakeFloor: grow rooms from a room at the map centre until there are
//!     floorRoomNum; start over (RNG not rewound) after 2 * floorRoomNum
//!     passes, or with fewer than 2 rooms that opened one exit
//!   down stairs (every floor but the last): a room with one connection
//!     passing fieldrand(100) >= 91, not room 0; up stairs: room 0
//!   MakeRoom per room: a model by size, exits and stairs; the Gott statue
//!     room; SetAllGim rolls for the model's dummies
//! ```

use super::{CELL, Exits, MAP};
use super::{
    Cell, DOWN, DummySource, GenerateError, Gim, NO_ROOM, RealMap, RoomSize, RoomTable, Rotation, SIDES, Tables, UP,
    is_lake,
};

/// The area generator's RNG, `fieldrand` (`INF SLUS_202.67:0x0019c460`):
/// `seed = (seed * 109 + 1021) mod 0xfffffffe` in 32-bit arithmetic,
/// returning `seed % n`; `count` is the game's `randcnt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    pub seed: u32,
    pub count: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng { seed, count: 0 }
    }

    /// One step: the new seed.
    pub fn advance(&mut self) -> u32 {
        self.count = self.count.wrapping_add(1);
        self.seed = self.seed.wrapping_mul(109).wrapping_add(1021) % 0xffff_fffe;
        self.seed
    }

    /// `fieldrand(n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.advance() % n
    }
}

/// What `DUNGEON::Generate` reads for a random dungeon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// `dungeonSeed[game.dungeon]`, which `WORLD_MAN::GO` seeds the RNG with.
    pub seed: u32,
    /// `dungeonType[game.dungeon]` (0-9).
    pub dtype: u8,
    /// `dungeonData[dungeonSize]`.
    pub level_max: u32,
    pub room_max: u32,
    /// `game.server`: 0 Delta ... 4 Omega.
    pub server: u32,
    /// `volumeNum`.
    pub volume: u32,
    /// The first keyword's ID (`WORLD_MAN+0x14c`): [`FOUR_FLOORS_WORD`]
    /// makes four floors.
    pub word_a: u32,
    /// `WORLD_MAN::GetFieldType`: field type 4 leads straight into a lake
    /// dungeon, whose first room then faces the player in.
    pub field_type: u32,
    /// `DUNGEON.code`: which of the area's dungeons (0-2).
    pub code: u32,
}

/// The first keyword that makes any non-lake dungeon four floors deep
/// (`Generate`'s `li 131`; the area generator's `timeSym`).
pub const FOUR_FLOORS_WORD: u32 = 131;
/// `floorRoomNum` at or above this becomes [`ROOMS_CAPPED`].
const ROOMS_LIMIT: u32 = 15;
const ROOMS_CAPPED: u32 = 12;
/// The map centre room 0 is placed around.
const CENTRE: i32 = 40;
/// A dead end becomes the down stairs on `fieldrand(100)` at or above this.
const DOWN_STAIRS_FROM: u32 = 91;
/// `ChooseRoomSize`: `fieldrand(100)` below 20 small, below this large
/// (volume 1 on servers 0-1: 50; otherwise 40), else medium.
const SMALL_BELOW: u32 = 20;
const LARGE_BELOW_EARLY: u32 = 50;
const LARGE_BELOW: u32 = 40;
/// Volume 2 on, servers 2 on: a large room 0 becomes medium on
/// `fieldrand(100)` at or above this.
const LARGE_START_MEDIUM_FROM: u32 = 31;

/// A generated random dungeon: what `Generate` leaves in `DUNGEON`.
#[derive(Clone, Debug)]
pub struct Dungeon {
    pub dtype: u8,
    /// `levelMax` as `Generate` left it (4 after [`FOUR_FLOORS_WORD`]).
    pub level_max: u32,
    pub floors: Vec<Floor>,
    /// The Gott statue room (`DUNGEON` +0x30 flag, +0x34 floor, +0x38 room;
    /// for the lake types +0x44 flag).
    pub statue: Option<Statue>,
    /// `gimPos`: every dummy slot kept, in order.
    pub gimmicks: Vec<GimPos>,
    /// The RNG after the last floor.
    pub rng: Rng,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statue {
    pub floor: usize,
    pub room: usize,
}

/// A `gimPos` entry: floor (as a byte), room, and what the slot holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GimPos {
    pub floor: u8,
    pub room: u8,
    pub gim: Gim,
}

#[derive(Clone, Debug)]
pub struct Floor {
    pub level: usize,
    pub map: RealMap,
    /// Rooms 0 .. `floorRoomNum`.
    pub rooms: Vec<Room>,
    /// `UpRoom`: always room 0.
    pub up: usize,
    /// `DownRoom`; none on the last floor.
    pub down: Option<usize>,
    /// How many times `MakeFloor` started the floor over.
    pub retries: u32,
    /// `startpos[0]` (arriving by the up stairs) and `startpos[1]` (by the
    /// down stairs, coming back up).
    pub start: [Option<Start>; 2],
}

/// A `startpos` entry as far as `MakeRoom` sets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start {
    /// x, y: the room's centre for a lake dungeon's up room; otherwise the
    /// model's `OBJ_0ppp` dummy, which is not modelled here.
    pub pos: Option<[f32; 2]>,
    /// w: the way the player faces; `None` leaves w at 1.0 (a lake up room
    /// not entered straight from the field).
    pub facing: Option<Rotation>,
}

impl Start {
    /// The w component the game stores.
    pub fn w(&self) -> f32 {
        self.facing.map_or(1.0, Rotation::radians)
    }
}

#[derive(Clone, Debug)]
pub struct Room {
    pub index: usize,
    /// Top-left map cell.
    pub x: i32,
    pub y: i32,
    pub size: RoomSize,
    /// The sides that meet a neighbour's door, with [`UP`] and [`DOWN`].
    pub exits: Exits,
    /// The centre, in world units (`DUNGEON.pos`).
    pub pos: [f32; 2],
    /// `None` when no row of the table has these exits (never seen).
    pub model: Option<RoomModel>,
}

/// What `MakeRoom` picked for a room.
#[derive(Clone, Debug)]
pub struct RoomModel {
    pub table: &'static RoomTable,
    /// The row, `animIdx.k`.
    pub row: usize,
    /// `fieldrand(num)`, `animIdx.r`: the model's index in the row.
    pub pick: u32,
    /// The ANM_ object: the row's model, or the statue room's.
    pub name: &'static str,
    pub rotate: Rotation,
    pub minimap: Minimap,
    /// The Gott statue room.
    pub statue: bool,
    /// SetAllGim's rolls for the model's dummies, in order.
    pub dummies: Vec<DummyRoll>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DummyRoll {
    /// Index into [`Tables::gim_patterns`].
    pub pattern: usize,
    pub gim: Gim,
    pub keep: bool,
}

/// `FOOT` (0x60 bytes): one room while the floor grows.
#[derive(Clone, Debug)]
struct Foot {
    x: i32,
    y: i32,
    direc: u8,
    live: bool,
    size: RoomSize,
    /// `exitSize[4]`: the size of the room behind each side; `None` is the
    /// game's 128, no exit.
    exit_size: [Option<RoomSize>; 4],
    exit_num: u32,
    next: [(i32, i32); 4],
    pos: [f32; 2],
    old_direc: u8,
}

impl Foot {
    fn new(x: i32, y: i32, size: RoomSize) -> Foot {
        Foot {
            x,
            y,
            direc: 0,
            live: true,
            size,
            exit_size: [None; 4],
            exit_num: 0,
            next: [(0, 0); 4],
            pos: [0.0; 2],
            old_direc: 0,
        }
    }

    fn set_pos(&mut self) {
        let half = self.size.cells() / 2;
        self.pos = [CELL * (self.x + half) as f32, CELL * (self.y + half) as f32];
    }
}

/// Generate a random dungeon. `dummies` gives each room model's dummy
/// counts, normally a [`super::Dummies`] read from
/// `tables.ccs_name(dtype, texType)`.
pub fn generate(tables: &Tables, p: &Params, dummies: &impl DummySource) -> Result<Dungeon, GenerateError> {
    if p.dtype >= 10 {
        return Err(GenerateError::Type(p.dtype));
    }
    let mut g = Gen {
        tables,
        p: *p,
        level_max: p.level_max,
        rng: Rng::new(p.seed),
        dummies,
        statue: None,
        gimmicks: Vec::new(),
    };
    let mut levels = p.level_max;
    if p.word_a == FOUR_FLOORS_WORD && !is_lake(p.dtype) {
        levels = 4;
        g.level_max = 4;
    }
    if is_lake(p.dtype) {
        levels = 1;
    }
    let mut floors = Vec::new();
    for level in 0..levels as usize {
        let mut n = g.rng.below(5).saturating_add(p.room_max);
        if n >= ROOMS_LIMIT {
            n = ROOMS_CAPPED;
        }
        floors.push(g.make_floor(level, n as usize)?);
    }
    Ok(Dungeon { dtype: p.dtype, level_max: g.level_max, floors, statue: g.statue, gimmicks: g.gimmicks, rng: g.rng })
}

struct Gen<'a, D: DummySource> {
    tables: &'a Tables,
    p: Params,
    level_max: u32,
    rng: Rng,
    dummies: &'a D,
    statue: Option<Statue>,
    gimmicks: Vec<GimPos>,
}

impl<D: DummySource> Gen<'_, D> {
    fn last_floor(&self, level: usize) -> bool {
        level as u64 + 1 == self.level_max as u64
    }

    /// `ChooseRoomSize` (0x005b6790).
    fn choose_room_size(&mut self) -> RoomSize {
        let r = self.rng.below(100);
        let large_below = if self.p.volume == 1 && self.p.server <= 1 { LARGE_BELOW_EARLY } else { LARGE_BELOW };
        if r < SMALL_BELOW {
            RoomSize::Small
        } else if r < large_below {
            RoomSize::Large
        } else {
            RoomSize::Medium
        }
    }

    /// `FOOT::FOOT(int level, MAP_INFO (*)[80][80])` (0x005b6840): room 0 at
    /// the map centre, one exit south.
    fn first_foot(&mut self, map: &mut RealMap) -> Foot {
        self.rng.below(1000); // drawn and dropped
        let mut size = self.choose_room_size();
        if !(self.p.volume == 1 || self.p.server < 2)
            && size == RoomSize::Large
            && self.rng.below(100) >= LARGE_START_MEDIUM_FROM
        {
            size = RoomSize::Medium;
        }
        let half = size.cells() / 2;
        let mut f = Foot::new(CENTRE - half, CENTRE - half, size);
        f.set_pos();
        f.exit_num = 1;
        f.direc = SIDES[1];
        f.exit_size[1] = Some(self.choose_room_size());
        paint(map, &f, 0);
        mark_doors(map, &f, f.direc);
        f
    }

    /// `FOOT::FOOT(level, n, xpos, ypos, s, d, RealMap)` (0x005b6ce0): room
    /// `n` at `(x, y)`, behind an exit, with 1-3 exits of its own; `d` is the
    /// side it was entered by.
    fn next_foot(&mut self, map: &mut RealMap, level: usize, n: usize, (x, y): (i32, i32), s: RoomSize, d: u8) -> Foot {
        let mut f = Foot::new(x, y, s);
        f.old_direc = d;
        paint(map, &f, n as u8);
        f.set_pos();
        loop {
            f.exit_num = (self.rng.below(4000) / 1000).max(1);
            f.exit_size = [None; 4];
            let mut k = f.exit_num;
            while k > 0 {
                let r = self.rng.below(400) >> 2;
                if r <= 3 {
                    let bit = SIDES[r as usize];
                    if f.direc & bit == 0 {
                        f.direc |= bit;
                        f.exit_size[r as usize] = Some(self.choose_room_size());
                        k -= 1;
                    }
                }
            }
            if f.direc & d != 0 {
                // An exit back where we came from is no exit.
                f.exit_size[side_index(d)] = None;
                f.direc -= d;
                f.exit_num -= 1;
                if f.exit_num == 0 {
                    continue;
                }
            }
            break;
        }
        if self.last_floor(level) {
            // The last floor only leads on to medium rooms.
            for e in f.exit_size.iter_mut().flatten() {
                *e = RoomSize::Medium;
            }
        }
        mark_doors(map, &f, f.old_direc);
        f
    }

    /// `DUNGEON::MakeFloor` (0x005c0110), random branch.
    fn make_floor(&mut self, level: usize, room_num: usize) -> Result<Floor, GenerateError> {
        let mut retries = 0;
        let (mut map, feet) = loop {
            let mut map = RealMap::default();
            let mut feet = vec![self.first_foot(&mut map)];
            if self.grow(&mut map, &mut feet, level, room_num)
                && feet.iter().take(room_num).filter(|f| f.exit_num == 1).count() >= 2
            {
                break (map, feet);
            }
            retries += 1;
        };
        let up = 0;
        let mut down = None;
        if !self.last_floor(level) {
            let dead_end = |d: u8| matches!(d, 1 | 2 | 4 | 8);
            if !(0..room_num).any(|i| dead_end(check_direction(&mut map, &feet[i])) && i != up) {
                return Err(GenerateError::NoDownStairs { floor: level });
            }
            while down.is_none() {
                for (i, f) in feet.iter().enumerate().take(room_num) {
                    if dead_end(check_direction(&mut map, f)) && self.rng.below(100) >= DOWN_STAIRS_FROM && up != i {
                        down = Some(i);
                    }
                }
            }
        }
        let mut floor = Floor { level, map, rooms: Vec::new(), up, down, retries, start: [None; 2] };
        for (i, f) in feet.iter().enumerate().take(room_num) {
            self.make_room(&mut floor, i, f);
        }
        Ok(floor)
    }

    /// Passes over the live rooms opening exits until there are `target`
    /// rooms; false when 2 * target passes were not enough.
    fn grow(&mut self, map: &mut RealMap, feet: &mut Vec<Foot>, level: usize, target: usize) -> bool {
        let mut passes = 0;
        loop {
            passes += 1;
            if target * 2 < passes {
                return false;
            }
            let mut i = 0;
            while i < feet.len() {
                if feet[i].live {
                    for (k, bit) in SIDES.into_iter().enumerate() {
                        if feet[i].exit_size[k].is_some() && !fits(map, &mut feet[i], k, false) {
                            feet[i].exit_size[k] = None;
                            feet[i].direc = feet[i].direc.wrapping_sub(bit);
                        }
                    }
                    for k in 0..4 {
                        let Some(size) = feet[i].exit_size[k] else { continue };
                        if fits(map, &mut feet[i], k, true) {
                            feet[i].live = false;
                            let (n, at) = (feet.len(), feet[i].next[k]);
                            let f = self.next_foot(map, level, n, at, size, SIDES[back(k)]);
                            feet.push(f);
                        }
                        if feet.len() == target {
                            return true;
                        }
                    }
                }
                i += 1;
            }
        }
    }

    /// `DUNGEON::MakeRoom(int, FOOT *, ROOM_INFO *, int)` (0x005ba1d0).
    fn make_room(&mut self, floor: &mut Floor, i: usize, f: &Foot) {
        let second = !(self.p.volume == 1 && self.p.server <= 1);
        let table = self.tables.room_table(f.size, self.p.dtype, second);
        let mut direc = check_direction(&mut floor.map, f);
        if i == floor.up {
            direc |= UP;
        }
        if floor.down == Some(i) {
            direc |= DOWN;
        }
        let exits = Exits(direc);
        let mut room = Room { index: i, x: f.x, y: f.y, size: f.size, exits, pos: f.pos, model: None };
        let lake = is_lake(self.p.dtype);
        for (k, row) in table.rows.iter().enumerate() {
            if row.exits != exits {
                continue;
            }
            let pick = self.rng.below(row.models.len() as u32);
            let mut name = row.models[pick as usize];
            let mut statue = false;
            if direc & (UP | DOWN) == 0 && exits.count() == 1 && f.size == RoomSize::Medium {
                // One per dungeon: on the last floor, or for the lake types
                // on their one floor (the game keeps a flag for each).
                statue = self.statue.is_none() && (lake || self.last_floor(floor.level));
                if statue {
                    name = self.tables.symroom[self.p.dtype as usize];
                    self.statue = Some(Statue { floor: floor.level, room: i });
                }
            }
            let minimap = Minimap::of(exits);
            // The player's start on this floor (startpos[0]) and where the
            // down stairs put him back (startpos[1]).
            if exits.has(UP) {
                floor.start[0] = Some(if lake {
                    let entry = self.p.field_type == 4 && self.p.code == 0 && floor.level == 0 && i == 0;
                    Start { pos: Some(f.pos), facing: entry.then(|| row.rotate.facing()) }
                } else {
                    Start { pos: None, facing: Some(row.rotate.facing()) }
                });
            }
            if exits.has(DOWN) {
                floor.start[1] = Some(Start { pos: None, facing: Some(row.rotate.facing()) });
            }
            let dummies = self.set_all_gim(floor.level, i, name);
            room.model = Some(RoomModel { table, row: k, pick, name, rotate: row.rotate, minimap, statue, dummies });
        }
        floor.rooms.push(room);
    }

    /// `DUNGEON::SetAllGim` (0x005bb340): each dummy of the model, in the
    /// order of the searches, rolls to keep its slot.
    fn set_all_gim(&mut self, level: usize, room: usize, model: &str) -> Vec<DummyRoll> {
        let counts = self.dummies.counts(model);
        let mut out = Vec::new();
        for (pattern, (pat, &n)) in self.tables.gim_patterns.iter().zip(&counts).enumerate() {
            for _ in 0..n {
                let keep = match pat.gim.keep_from() {
                    Some(from) => self.rng.below(100) >= from,
                    None => true,
                };
                out.push(DummyRoll { pattern, gim: pat.gim, keep });
                if keep {
                    self.gimmicks.push(GimPos { floor: level as u8, room: room as u8, gim: pat.gim });
                }
            }
        }
        out
    }
}

/// `MakeRoom`'s minimap code for a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Minimap {
    /// Three sides (0).
    Tee,
    /// Four sides (1).
    Cross,
    /// One side, no stairs (2).
    DeadEnd,
    /// The up stairs (3).
    Up,
    /// The down stairs (4).
    Down,
    /// Two opposite sides (5).
    Straight,
    /// Two sides at a corner (6).
    Corner,
    /// No side (128).
    Other,
}

impl Minimap {
    fn of(exits: Exits) -> Minimap {
        let sides = exits.sides();
        if exits.has(DOWN) {
            Minimap::Down
        } else if exits.has(UP) {
            Minimap::Up
        } else if exits.count() == 2 && (sides & 3 == 3 || sides & 12 == 12) {
            Minimap::Straight
        } else if exits.count() == 2 {
            Minimap::Corner
        } else {
            match exits.count() {
                3 => Minimap::Tee,
                4 => Minimap::Cross,
                1 => Minimap::DeadEnd,
                _ => Minimap::Other,
            }
        }
    }

    /// The game's number.
    pub fn code(self) -> u8 {
        match self {
            Minimap::Tee => 0,
            Minimap::Cross => 1,
            Minimap::DeadEnd => 2,
            Minimap::Up => 3,
            Minimap::Down => 4,
            Minimap::Straight => 5,
            Minimap::Corner => 6,
            Minimap::Other => 128,
        }
    }
}

/// The side index of a side bit.
fn side_index(bit: u8) -> usize {
    SIDES.iter().position(|&b| b == bit).expect("a single side bit")
}

/// The side opposite side `k` (`MakeFloor`'s `rDirec[4]`).
fn back(k: usize) -> usize {
    k ^ 1
}

/// Every cell of the room gets its number.
fn paint(map: &mut RealMap, f: &Foot, n: u8) {
    let size = f.size.cells();
    for y in f.y..f.y + size {
        for x in f.x..f.x + size {
            map.at(x, y).here = n;
        }
    }
}

/// The two door cells of each side in `bits`, as the `FOOT` constructors
/// and `Move` write them.
fn mark_doors(map: &mut RealMap, f: &Foot, bits: u8) {
    let (x, y, door, size) = (f.x, f.y, f.size.door(), f.size.cells());
    let cells = [
        [(x + door, y), (x + door + 1, y)],
        [(x + door, y + size - 1), (x + door + 1, y + size - 1)],
        [(x, y + door), (x, y + door + 1)],
        [(x + size - 1, y + door), (x + size - 1, y + door + 1)],
    ];
    for (bit, pair) in SIDES.into_iter().zip(cells) {
        if bits & bit != 0 {
            for (cx, cy) in pair {
                map.at(cx, cy).door = bit;
            }
        }
    }
}

/// `FOOT::Move` (0x005b7330): does the room behind side `k` fit? With
/// `mark`, open this room's door on that side.
fn fits(map: &mut RealMap, f: &mut Foot, k: usize, mark: bool) -> bool {
    let Some(es) = f.exit_size[k] else { return false };
    let size = f.size.cells();
    let side = es.cells();
    let ofs = (size - side) / 2;
    let (nx, ny) =
        [(f.x + ofs, f.y - side), (f.x + ofs, f.y + size), (f.x - side, f.y + ofs), (f.x + size, f.y + ofs)][k];
    f.next[k] = (nx, ny);
    let on_map = |v: i32| (0..MAP as i32).contains(&v);
    if !(on_map(nx) && on_map(ny)) {
        return false;
    }
    for j in ny..ny + side {
        for i in nx..nx + side {
            match map.get(i, j) {
                Some(Cell { here: NO_ROOM, .. }) => {}
                _ => return false,
            }
        }
    }
    if mark {
        mark_doors(map, f, SIDES[k]);
    }
    true
}

/// `FOOT::CheckDirection` (0x005b78c0): the sides whose door meets another
/// door, and each such door cell's `next` set to the room beyond it.
fn check_direction(map: &mut RealMap, f: &Foot) -> u8 {
    let (x, y, door, size) = (f.x, f.y, f.size.door(), f.size.cells());
    let m = MAP as i32;
    let d = |map: &RealMap, x: i32, y: i32| map.get(x, y).is_some_and(|c| c.door != 0);
    let here = |map: &RealMap, x: i32, y: i32| map.get(x, y).map_or(NO_ROOM, |c| c.here);
    let mut out = 0;
    if y - 1 > 0 && d(map, x + door, y) && d(map, x + door, y - 1) && d(map, x + door + 1, y - 1) {
        let n = here(map, x + door, y - 1);
        map.at(x + door, y).next = n;
        map.at(x + door + 1, y).next = n;
        out |= SIDES[0];
    }
    let yy = y + size;
    if yy < m && d(map, x + door, yy - 1) && d(map, x + door, yy) && d(map, x + door + 1, yy) {
        map.at(x + door, yy - 1).next = here(map, x + door, yy);
        map.at(x + door + 1, yy - 1).next = here(map, x + door + 1, yy);
        out |= SIDES[1];
    }
    if x - 1 > 0 && d(map, x, y + door) && d(map, x - 1, y + door) && d(map, x - 1, y + door + 1) {
        map.at(x, y + door).next = here(map, x - 1, y + door);
        map.at(x, y + door + 1).next = here(map, x - 1, y + door + 1);
        out |= SIDES[2];
    }
    let xx = x + size;
    if xx < m && d(map, xx - 1, y + door) && d(map, xx, y + door) && d(map, xx, y + door + 1) {
        map.at(xx - 1, y + door).next = here(map, xx, y + door);
        map.at(xx - 1, y + door + 1).next = here(map, xx, y + door + 1);
        out |= SIDES[3];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::{DummyCounts, INF};
    use super::*;

    /// Floors as (rooms, down stairs, restarts).
    type Shape = Vec<(usize, Option<usize>, u32)>;

    fn run(p: Params) -> (Rng, u32, Option<(usize, usize)>, Shape) {
        // No dummies: the numbers tools/dungeon.py gives with empty counts.
        let g = generate(&INF, &p, &|_: &str| DummyCounts::default()).unwrap();
        assert!(g.gimmicks.is_empty());
        let shape = g.floors.iter().map(|f| (f.rooms.len(), f.down, f.retries)).collect();
        (g.rng, g.level_max, g.statue.map(|s| (s.floor, s.room)), shape)
    }

    fn params(seed: u32, dtype: u8, level_max: u32, room_max: u32) -> Params {
        Params { seed, dtype, level_max, room_max, server: 0, volume: 1, word_a: 0, field_type: 0, code: 0 }
    }

    #[test]
    fn fixed_seeds() {
        let (rng, levels, statue, shape) = run(params(12345, 1, 3, 7));
        assert_eq!((rng.seed, rng.count, levels), (1_317_830_364, 1999, 3));
        assert_eq!(statue, Some((2, 7)));
        assert_eq!(shape, vec![(8, Some(6), 0), (8, Some(7), 0), (11, None, 0)]);

        let (rng, levels, statue, shape) = run(Params { field_type: 4, ..params(0x0246_8ace, 8, 3, 7) });
        assert_eq!((rng.seed, rng.count, levels), (1_846_032_535, 525, 3));
        assert_eq!(statue, None);
        assert_eq!(shape, vec![(7, Some(5), 0)]);
    }

    #[test]
    fn word_131_and_restarts() {
        let (rng, levels, statue, shape) = run(Params { word_a: FOUR_FLOORS_WORD, ..params(0x0123_4567, 1, 2, 5) });
        assert_eq!((rng.seed, rng.count, levels), (3_887_392_981, 1954, 4));
        assert_eq!(statue, Some((3, 3)));
        assert_eq!(shape, vec![(8, Some(6), 0), (5, Some(3), 2), (7, Some(6), 1), (8, None, 0)]);

        let (rng, levels, statue, shape) = run(Params { server: 3, volume: 2, ..params(7919, 3, 4, 9) });
        assert_eq!((rng.seed, rng.count, levels), (1_658_230_983, 1928, 4));
        assert_eq!(statue, Some((3, 5)));
        assert_eq!(shape, vec![(11, Some(9), 0), (9, Some(6), 1), (11, Some(7), 0), (12, None, 0)]);
    }

    #[test]
    fn rng() {
        let mut r = Rng::new(0);
        assert_eq!(r.advance(), 1021);
        assert_eq!(r.advance(), 1021 * 109 + 1021);
        let mut r = Rng::new(0xffff_fffd);
        // (0xfffffffd * 109 + 1021) mod 2^32 = 694, then mod 0xfffffffe.
        assert_eq!(r.below(1000), 694);
        assert_eq!(r.count, 1);
    }

    #[test]
    fn bad_type() {
        assert_eq!(generate(&INF, &params(1, 10, 2, 5), &|_: &str| [0; 8]).unwrap_err(), GenerateError::Type(10));
    }
}
