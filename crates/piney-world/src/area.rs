//! Where The World is: `ccGame`'s scene (main 0x003789cc's object, 0x88 bytes):
//! the area (0 a Root Town, 1 a field, 2 a dungeon), town, field, dungeon, floor
//! and block, each with the one before, and the calls that change it:
//! `ChangeArea(a, n)` (0x001674a0) and `ChangeScene` (0x00167380), which set
//! `lastTown` and the server by town, clear the battle and ask
//! `ChangeRequest(6, 7)`. -2 keeps a value, -1 is none
//! (docs/engine/field-game.md).

use piney_data::save::{SaveData, offset};

/// `@1489` (main 0x00306dc0): each town's server.
pub const SERVER_OF_TOWN: [i32; 8] = [0, 1, 2, 3, 4, 0, 1, 2];

/// The areas.
pub mod kind {
    pub const TOWN: i32 = 0;
    pub const FIELD: i32 = 1;
    pub const DUNGEON: i32 = 2;
}

/// `ccGame`'s scene fields (+0x14..+0x48).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene {
    pub area: i32,
    pub area_prev: i32,
    pub server: i32,
    pub town: i32,
    pub field: i32,
    pub dungeon: i32,
    pub floor: i32,
    pub block: i32,
    pub server_prev: i32,
    pub town_prev: i32,
    pub field_prev: i32,
    pub dungeon_prev: i32,
    pub floor_prev: i32,
    pub block_prev: i32,
    /// `ccGame.gameCnt` (+0x64 .. +0x70): four clocks in sixtieths that
    /// `ccAddPlayTime` runs with the play time ([`Scene::add_play_time`]);
    /// every change zeroes [0], a new scene [1], and leaving a town (or
    /// nothing) [2]: [2] is the time since the Chaos Gate, the Zeit
    /// statue's.
    pub game_cnt: [i32; 4],
}

/// `ccAddPlayTime`'s limit: 0x0cdfe5c4 sixtieths, 999:59:59.
pub const PLAY_TIME_MAX: i32 = 0x0cdf_e5c4;

impl Scene {
    /// `ccGame::InitScene` (0x00167320): every field and its previous -1,
    /// area 0 (`ccGame::ChangeRequest(5)` for a new game).
    pub fn init() -> Scene {
        Scene {
            area: 0,
            area_prev: -1,
            server: -1,
            town: -1,
            field: -1,
            dungeon: -1,
            floor: -1,
            block: -1,
            server_prev: -1,
            town_prev: -1,
            field_prev: -1,
            dungeon_prev: -1,
            floor_prev: -1,
            block_prev: -1,
            game_cnt: [0; 4],
        }
    }

    /// `ccAddPlayTime` (main 0x00167740)'s clocks: each of `gameCnt` plus
    /// `rate` (the frame rate, vertical blanks a frame), held at
    /// [`PLAY_TIME_MAX`]. Nothing while `gameCntStop` (+0x74) is set, which
    /// only a card write does, within its frame.
    pub fn add_play_time(&mut self, rate: i32) {
        for c in &mut self.game_cnt {
            *c = c.wrapping_add(rate).min(PLAY_TIME_MAX);
        }
    }

    /// What TOPPAGE's Log in leaves: `InitScene`, then
    /// `ChangeArea(0, saveData.lastTown)`.
    pub fn log_in(save: &mut SaveData) -> Scene {
        let mut s = Scene::init();
        let town = i32::from(save.u8(offset::LAST_TOWN) as i8);
        s.change_area(kind::TOWN, town, save);
        s
    }

    /// `ccGame::ChangeScene(a, t, fd, d, f, b)` and its `ChangeRequest(6,
    /// 7)`'s clocks (0x001671e0): `gameCnt[0]` zeroed, [1] when the scene
    /// is replaced (`CheckSceneReplace`), [2] when the area left was a town
    /// or none (`areaPrev` 0 or -1). The rest of `ChangeRequest` is the
    /// caller's.
    #[allow(clippy::too_many_arguments)]
    pub fn change_scene(&mut self, a: i32, t: i32, fd: i32, d: i32, f: i32, b: i32, save: &mut SaveData) {
        self.town_prev = self.town;
        if t >= -1 {
            self.town = t;
            save.set_u8(offset::LAST_TOWN, t as u8);
        }
        self.server_prev = self.server;
        if self.town >= 0 {
            self.server = SERVER_OF_TOWN[(self.town as usize).min(7)];
        }
        self.field_prev = self.field;
        if fd >= -1 {
            self.field = fd;
        }
        self.dungeon_prev = self.dungeon;
        if d >= -1 {
            self.dungeon = d;
        }
        self.floor_prev = self.floor;
        if f >= -1 {
            self.floor = f;
        }
        self.block_prev = self.block;
        if b >= -1 {
            self.block = b;
        }
        self.area_prev = self.area;
        if a >= -1 {
            self.area = a;
        }
        self.game_cnt[0] = 0;
        if self.changed() {
            self.game_cnt[1] = 0;
        }
        if matches!(self.area_prev, 0 | -1) {
            self.game_cnt[2] = 0;
        }
    }

    /// `ccGame::ChangeArea(a, n)`.
    pub fn change_area(&mut self, a: i32, n: i32, save: &mut SaveData) {
        match a {
            kind::TOWN => self.change_scene(0, n, -1, -1, -1, -1, save),
            kind::FIELD => self.change_scene(1, -2, n, -1, -1, -1, save),
            kind::DUNGEON => self.change_scene(2, -2, -2, n, 0, 0, save),
            _ => self.change_scene(0, 0, -1, -1, -1, -1, save),
        }
    }

    /// A `ChangeArea` or `ChangeScene` asked by someone who does not hold
    /// the scene (a menu, an event instruction).
    pub fn go(&mut self, go: piney_data::area::Go, save: &mut SaveData) {
        match go {
            piney_data::area::Go::ChangeArea(a, n) => self.change_area(a, n, save),
            piney_data::area::Go::ChangeScene([a, t, fd, d, f, b]) => self.change_scene(a, t, fd, d, f, b, save),
        }
    }

    /// `ccSetupGameCtrl`'s test (0x00168974): the area, the town, the field or
    /// the dungeon differs from the one before - a new scene, not a room or
    /// a floor of the same dungeon. A change fades the sound out and loads
    /// the area's sequence bank.
    pub fn changed(&self) -> bool {
        self.area != self.area_prev
            || self.town != self.town_prev
            || self.field != self.field_prev
            || self.dungeon != self.dungeon_prev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `gameCnt`: [2] runs from the town's gate through the field and
    /// the dungeon's floors (the Zeit statue's time); [1] restarts with
    /// each new scene, [0] at every change.
    #[test]
    fn the_clocks_run_from_the_gate() {
        let mut save = SaveData::default();
        let mut s = Scene::log_in(&mut save);
        for _ in 0..10 {
            s.add_play_time(1);
        }
        assert_eq!(s.game_cnt, [10, 10, 10, 10]);
        s.change_scene(1, -2, 14, -1, -1, -1, &mut save);
        assert_eq!(s.game_cnt, [0, 0, 0, 10]);
        s.add_play_time(2);
        s.change_area(kind::DUNGEON, 14, &mut save);
        assert_eq!(s.game_cnt, [0, 0, 2, 12]);
        s.add_play_time(3);
        // The next floor of the same dungeon: not a new scene.
        s.change_scene(2, -2, -2, -2, 1, 0, &mut save);
        assert_eq!(s.game_cnt, [0, 3, 5, 15]);
        s.game_cnt = [PLAY_TIME_MAX; 4];
        s.add_play_time(2);
        assert_eq!(s.game_cnt, [PLAY_TIME_MAX; 4]);
    }

    #[test]
    fn to_the_field_and_back() {
        let mut save = SaveData::default();
        let mut s = Scene::log_in(&mut save);
        assert_eq!((s.area, s.town, s.server, s.field), (0, 0, 0, -1));
        // Event 2's `scene area=1 town=0 field=14`.
        s.change_scene(1, 0, 14, -1, -1, -1, &mut save);
        assert_eq!((s.area, s.area_prev, s.town, s.field, s.dungeon), (1, 0, 0, 14, -1));
        assert!(s.changed());
        // WORLD_MAN::Enter at the entrance: ChangeArea(2, 0).
        s.change_area(2, 0, &mut save);
        assert_eq!((s.area, s.area_prev, s.town, s.field, s.dungeon, s.floor, s.block), (2, 1, 0, 14, 0, 0, 0));
        // A door: ChangeScene(-2, ..., room 3) - the same scene.
        s.change_scene(-2, -2, -2, -2, -2, 3, &mut save);
        assert_eq!((s.area, s.area_prev, s.block, s.block_prev), (2, 2, 3, 0));
        assert!(!s.changed());
        // Gate Out: ChangeArea(0, town).
        s.change_area(0, s.town.max(0), &mut save);
        assert_eq!((s.area, s.area_prev, s.town, s.field, s.dungeon), (0, 2, 0, -1, -1));
        assert_eq!(save.u8(offset::LAST_TOWN), 0);
    }

    #[test]
    fn the_gates_calls() {
        use piney_data::area::Go;
        let mut save = SaveData::default();
        let mut s = Scene::log_in(&mut save);
        // A story area's field from the Chaos Gate.
        s.go(Go::ChangeArea(1, 14), &mut save);
        assert_eq!((s.area, s.town, s.field, s.dungeon), (1, 0, 14, -1));
        // Field type 4: a story area's first dungeon at once.
        let mut s = Scene::log_in(&mut save);
        s.go(Go::ChangeScene([2, -2, 31, 0, 0, 0]), &mut save);
        assert_eq!((s.area, s.area_prev, s.town, s.field, s.dungeon, s.floor, s.block), (2, 0, 0, 31, 0, 0, 0));
        assert!(s.changed());
    }
}

/// What `WORLD_MAN` holds of the area three words make
/// (`WORLD_MAN::SimGenerateCode` main 0x0019e5c0, `docs/engine/area-words.md`)
/// and `GO` reads: the field's and dungeons' seeds and attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldMan {
    /// +0x44 `fieldSeed`, +0x48 `dungeonSeed[3]`.
    pub field_seed: u32,
    pub dungeon_seed: [u32; 3],
    /// +0x10 `fieldtype` (0-10), +0x0c `bgnum` (the weather).
    pub field_type: u32,
    pub weather: u32,
    /// The merged word parameters' `ground` and `object` (0-2).
    pub ground: u32,
    pub object: u32,
    /// The story area the words make (0 none): `GO` sets
    /// `eventAreaNumber` from `game.field` itself.
    pub event: i32,
    /// `IsProtectArea()`: the story area's `protect[1]`.
    pub protect: bool,
    /// +0x138 `dungeonLevelNum`, +0x13c `dungeonRoomNum`.
    pub level_max: u32,
    pub room_max: u32,
    /// The story area's `EVENTAREA_INFO.flag` (+0x24): `GO(1)`'s
    /// `SetHackFlag` (`hackFlag`, +0xf0); 2 for a random area, 3 in the
    /// crisis.
    pub hack: u32,
    /// The story area's `EVENTAREA_INFO.model` (+0x20): 1 for a story map
    /// of its own (an `EVENTAREA`), which also takes the event sound bank.
    pub field_model: i32,
    /// `dungeonType[]` (+0x34) as `SetDungeonTypeFromField` sets it.
    pub dungeon_type: [u8; 3],
    /// +0x14c .. +0x154 `A`, `B`, `C`: the three word IDs (the first, 131,
    /// makes a random dungeon four floors deep; Area Information shows
    /// them).
    pub words: [i32; 3],
    /// The merged `WORDPARAM` (`WORLD_MAN` +0x158): `areaLevel` (+0x20),
    /// `enemyOfs` (+0x24), `circleOfs`, `itemOfs`.
    pub area_level: i32,
    pub enemy_ofs: i32,
    pub circle_ofs: i32,
    pub item_ofs: i32,
    /// The story area's `EVENTAREA_INFO.enemy` (+0x18), 255 when it sets
    /// none (`ccRegisterDifficultyEnemy` then ranks by the level); 255
    /// outside a story area.
    pub ev_enemy: i32,
}

/// The save flag the volume's `SetDungeonTypeFromField` reads: the crisis
/// byte in Infection, `eventFlag[314]` bit 62 from Mutation on
/// (`piney_data::dungeon::SaveFlag`).
pub fn dungeon_flag(tables: &piney_data::area::AreaTables, save: &SaveData) -> bool {
    tables.dungeon_rule().save_flag_of(save)
}

impl WorldMan {
    /// `WORLD_MAN` as `SimGenerateCode` leaves it for `code`
    /// (`piney_data::area`), and what `GO` reads of the story area it makes.
    /// `crisis` is `saveData.crisis`; `dungeon_flag` the save flag the
    /// volume's `SetDungeonTypeFromField` reads ([`dungeon_flag`]).
    pub fn from_code(
        code: &piney_data::area::AreaCode,
        tables: &piney_data::area::AreaTables,
        crisis: bool,
        dungeon_flag: bool,
    ) -> WorldMan {
        let info = tables.event_area_info(code.event, code.flag71).filter(|_| code.event != 0);
        let dt = code.dungeon_type(tables, dungeon_flag);
        let random_hack = if crisis { 3 } else { 2 };
        WorldMan {
            field_seed: code.field_seed,
            dungeon_seed: code.dungeon_seed,
            field_type: code.attrs.field_type as u32,
            weather: code.attrs.weather as u32,
            ground: code.attrs.ground as u32,
            object: code.attrs.object as u32,
            event: code.event,
            protect: tables.is_protect_area(code.event, code.flag71),
            level_max: code.level_max as u32,
            room_max: code.room_max as u32,
            hack: match info {
                Some(i) if !crisis => i.flag as u32,
                _ => random_hack,
            },
            field_model: info.map_or(0, |i| i.model),
            dungeon_type: [dt[0], dt[1], 0],
            words: [code.a, code.b, code.c],
            area_level: code.attrs.area_level,
            enemy_ofs: code.attrs.enemy_ofs,
            circle_ofs: code.attrs.circle_ofs,
            item_ofs: code.attrs.item_ofs,
            ev_enemy: info.map_or(255, |i| i.enemy),
        }
    }

    /// `WORLD_MAN.timeSym` (+0x134, `SimGenerateCode`): the first word is
    /// "Chronicling" (131), whose dungeon ends at the Zeit statue.
    pub fn time_sym(&self) -> bool {
        self.words[0] == piney_data::area::TIME_SYM_WORD
    }

    /// `WORLD_MAN::GetFieldAttrb()`: the area's element (0-5) by field type
    /// and weather, as `docs/engine/area-words.md`'s `FIELD_ATTRIB` gives it
    /// (the function run over every input).
    pub fn field_attr(&self) -> i32 {
        let w = self.weather;
        match self.field_type {
            0..=3 => 2,
            4 => {
                if w <= 5 {
                    3
                } else {
                    4
                }
            }
            5 | 6 => 1,
            7 => 0,
            8 => {
                if w <= 5 {
                    5
                } else {
                    4
                }
            }
            9 => match w {
                0..=3 => 3,
                4 | 5 => 1,
                _ => 4,
            },
            10 => match w {
                0..=3 => 3,
                4 | 5 => 5,
                _ => 4,
            },
            _ => 0,
        }
    }

    /// `ccRegisterDifficultyEnemy()` (main 0x001b7020) in a field or
    /// dungeon: the list type (6 in a story area, else
    /// `WORLD_MAN::GetFieldAttrb()`, `field_attr`) and the rank (the story
    /// area's `enemy` unless 255, else `25 (areaLevel - 1) + 10 +
    /// enemyOfs`, at least 0; plus `floor + 1` in a dungeon) that
    /// `ccRegisterEnemyList(server, type, rank)` registers.
    pub fn enemy_rank(&self, area: i32, field: i32, floor: i32, field_attr: i32) -> (i32, i32) {
        let by_level = 25 * (self.area_level - 1) + 10 + self.enemy_ofs;
        let (ty, mut rank) = if field > 0 {
            (6, if self.ev_enemy != 255 { self.ev_enemy } else { by_level })
        } else {
            (field_attr, by_level)
        };
        if rank < 0 {
            rank = 0;
        }
        if area == 2 {
            rank += (floor + 1).max(0);
        }
        (ty, rank)
    }

    /// `WORLD_MAN::SetGenerateCode(a, b, c)` (main 0x0019eea0) on `server`
    /// (`ccGame.server`) with `save`'s flags: the area the three word IDs
    /// make, and the change of scene to it (its field, or its dungeon where
    /// there is no field). `None` for a word ID not in its slot's table.
    pub fn set_generate_code(
        tables: &piney_data::area::AreaTables,
        words: [i32; 3],
        server: i32,
        save: &SaveData,
    ) -> Option<(WorldMan, piney_data::area::Go)> {
        let [a, b, c] = words;
        let code = piney_data::area::sim_generate_code(tables, a, b, c, server, piney_data::area::flag71(save))?;
        let crisis = save.u8(offset::CRISIS) != 0;
        Some((WorldMan::from_code(&code, tables, crisis, dungeon_flag(tables, save)), code.go()))
    }

    /// `WORLD::Generate`'s inputs (`piney_data::field::Params`).
    pub fn field_params(&self) -> piney_data::field::Params {
        piney_data::field::Params {
            seed: self.field_seed,
            field_type: self.field_type,
            weather: self.weather,
            ground: self.ground,
            object: self.object,
            event: self.event,
            protect: self.protect,
            skip_init: false,
        }
    }
}
