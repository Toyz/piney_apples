//! The Chaos Gate keywords and the area generator
//! (`docs/engine/area-words.md`): three words typed at a gate make an area
//! code, seed the RNG, and pin what they say about the area; a story area
//! on its server pins more. [`sim_generate_code`] is
//! `WORLD_MAN::SimGenerateCode` (`INF SLUS_202.67:0x0019e5c0`) and gives
//! everything it leaves in `WORLD_MAN`.
//!
//! The tables - the twelve `WORDPARAM` tables `word_a1` .. `word_c4`,
//! `dungeonData`, `eventAreaInfo` and `volumeNum` - are each volume's,
//! read from its executable by `piney-gen` (`placement::area`) into the
//! build ([`AreaTables::of`], `plans/build-data.md`); the keywords' text is
//! the executable's, byte for byte. `tools/areas.py` is the reference, and `tools/test_area_rs.py`
//! runs the game's own functions in `tools/eemu.py` against this port
//! (`examples/area_probe.rs`).

use crate::dungeon;
use crate::store::{Load, Reader};
use crate::volume::Volume;

/// `sizeof(WORDPARAM)`.
pub const WORDPARAM_SIZE: u32 = 0x30;
/// `sizeof(EVENTAREA_INFO)`.
pub const EVENTAREA_INFO_SIZE: u32 = 0x54;
/// The row count `WORLD_MAN::IsProtectArea`, `GetEventAreaInfo` and
/// `GetWordParamFromEvCode` search in Infection, a constant in their code
/// ([`AreaTables::search`]: 127 from Mutation on).
pub const EVENT_AREA_SEARCH: usize = 126;
/// A `WORDPARAM` or `EVENTAREA_INFO` field that says nothing (`dungeonSize`
/// uses 0).
pub const UNSET: i32 = 255;
/// The first keyword whose area has `timeSym` set.
pub const TIME_SYM_WORD: i32 = 131;

/// A word's position in the address: `a`, `b` or `c`.
pub type Slot = usize;

/// A `WORDPARAM` row: a keyword and what it pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordParam {
    /// `+0x00 char *word`, one char per byte (the executable's bytes as
    /// Latin-1, so comparing texts compares bytes as `strcmp` does).
    pub text: &'static str,
    /// 0, 1, 2: `a`, `b`, `c`.
    pub slot: Slot,
    /// 1-4: the table `word_<slot><group>` it is in.
    pub group: u8,
    /// `+0x04`: unique across the slots.
    pub id: i32,
    /// `+0x08`: the higher wins.
    pub pri: i32,
    pub attrs: WordAttrs,
}

/// `WORDPARAM +0x0c .. +0x2c`: what a word pins ([`UNSET`] where it pins
/// nothing; `dungeon_size` 0), and what the generator leaves in
/// `*WORLD_MAN.wordparam`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WordAttrs {
    pub field_type: i32,
    pub dungeon_size: i32,
    pub weather: i32,
    pub ground: i32,
    pub object: i32,
    pub area_level: i32,
    pub enemy_ofs: i32,
    pub item_ofs: i32,
    pub circle_ofs: i32,
}

impl WordAttrs {
    /// `CopyWordParam(dst, src)` (0x0019e500): each field `src` sets
    /// overwrites `self`'s.
    pub fn merge(&mut self, src: &WordAttrs) {
        let pairs = [
            (&mut self.field_type, src.field_type),
            (&mut self.weather, src.weather),
            (&mut self.ground, src.ground),
            (&mut self.object, src.object),
            (&mut self.area_level, src.area_level),
            (&mut self.enemy_ofs, src.enemy_ofs),
            (&mut self.item_ofs, src.item_ofs),
            (&mut self.circle_ofs, src.circle_ofs),
        ];
        for (d, s) in pairs {
            if s != UNSET {
                *d = s;
            }
        }
        if src.dungeon_size != 0 {
            self.dungeon_size = src.dungeon_size;
        }
    }
}

/// An `EVENTAREA_INFO` row (0x54 bytes): a story area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventAreaInfo {
    /// `+0x00`: the event area number.
    pub code: i32,
    /// `+0x04 wordA`, `+0x08 wordB`, `+0x0c wordC`; `None` for a NULL
    /// pointer (13 rows have no address).
    pub words: [Option<&'static str>; 3],
    /// `+0x10`: 0 Delta, 1 Theta, 2 Lambda, 3 Sigma, 4 Omega.
    pub server: i32,
    /// `+0x14` -> `circleOfs`, [`UNSET`] keeps it.
    pub circle: i32,
    /// `+0x18` -> `enemyOfs`.
    pub enemy: i32,
    /// `+0x1c` -> `itemOfs`.
    pub item: i32,
    /// `+0x20`: 1 makes `ccSetupGameCtrl` load the area's event bank
    /// (`ccSndSQLoad(5)`) for its field instead of the field's.
    pub model: i32,
    /// `+0x24`: 3 gives the area's dungeons the E types
    /// (`SetDungeonTypeFromField`).
    pub flag: i32,
    /// `+0x28 type` -> `fieldType`.
    pub field_type: i32,
    /// `+0x2c bgnum` -> the weather.
    pub bgnum: i32,
    /// `+0x30`.
    pub dungeon_num: i32,
    /// `+0x34`. `protect[1]` is what `IsProtectArea` returns; the event VM
    /// reads the whole as four (important item, count) pairs.
    pub protect: [i32; 8],
}

impl EventAreaInfo {
    /// The row has an address (`wordA` is not NULL), which
    /// `ccCheckEventAreaNum` requires.
    pub fn has_words(&self) -> bool {
        self.words[0].is_some()
    }
}

/// `DUNGEON_INFO`: `dungeonData[dungeonSize]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DungeonInfo {
    pub level_max: i32,
    pub room_max: i32,
}

/// The area generator's tables, one volume's.
#[derive(Clone, Debug)]
pub struct AreaTables {
    /// Every keyword in the order `GetWordParamPtr(int)` and
    /// `GetWordParamID` search: `word_a1`, `word_b1`, `word_c1`, `word_a2`,
    /// ... `word_c4`.
    pub words: &'static [WordParam],
    /// `dungeonData`.
    pub dungeon_data: [DungeonInfo; 11],
    /// `eventAreaInfo`, `eventAreaInfoNum` rows.
    pub events: &'static [EventAreaInfo],
    /// The records `ccGetEventAreaInfo` returns in place of the table's from
    /// Mutation on (areas 71 and 47, the cases Infection writes inline in
    /// `SimGenerateCode`); none in Infection ([`AreaTables::substitute`]).
    pub substitutes: &'static [EventAreaInfo],
    /// `volumeNum`; `sim_generate_code`'s area 47 case reads it. Public so a
    /// check can force the other volumes' value.
    pub volume: i32,
}

impl AreaTables {
    /// The volume's tables (the build's `PINEY/TABLES/area.bin`, read once a
    /// run).
    pub fn of(v: Volume) -> &'static AreaTables {
        static READ: [std::sync::OnceLock<&'static AreaTables>; 4] = [const { std::sync::OnceLock::new() }; 4];
        READ[v as usize].get_or_init(|| crate::store::group(v, "area"))
    }

    /// `WORLD_MAN::GetWordParamPtr(part, slot, id)` (0x001a2340): the word of
    /// `slot` with this ID, groups 1 to 4; `None` where the game returns
    /// NULL.
    pub fn word(&self, slot: Slot, id: i32) -> Option<&WordParam> {
        self.words.iter().find(|w| w.slot == slot && w.id == id)
    }

    /// `WORLD_MAN::GetWordParamPtr(id)` (0x001a25d0): the word with this ID
    /// in any slot.
    pub fn word_by_id(&self, id: i32) -> Option<&WordParam> {
        self.words.iter().find(|w| w.id == id)
    }

    /// The word of `slot` whose text is `text` (as `GetWordParamFromEvCode`
    /// compares, exactly).
    pub fn word_by_text(&self, slot: Slot, text: &str) -> Option<&WordParam> {
        self.words.iter().find(|w| w.slot == slot && w.text == text)
    }

    /// `WORLD_MAN::GetWordParamID(text)` (0x001a3370): the ID of the first
    /// word with this text in any slot, -1 for none.
    pub fn word_id(&self, text: &str) -> i32 {
        self.words.iter().find(|w| w.text == text).map_or(-1, |w| w.id)
    }

    /// The volume's `SetDungeonTypeFromField` rule.
    pub fn dungeon_rule(&self) -> &'static dungeon::TypeRule {
        &dungeon::tables_of(Volume::from_number(self.volume).unwrap_or(Volume::Inf)).types
    }

    /// The rows `GetEventAreaInfo`, `IsProtectArea` and
    /// `GetWordParamFromEvCode` search, a constant in their code: 126 in
    /// Infection, 127 from Mutation on (whose 127th row is area 126).
    pub fn search(&self) -> usize {
        if self.substitutes.is_empty() { EVENT_AREA_SEARCH } else { 127 }
    }

    /// From Mutation on, the record the lookups return in place of the
    /// table's (MUT `GetEventAreaInfo(n)` 0x001b1fe0 and the others): area
    /// 71's while `flag71` ([`flag71`]), area 47's from volume 3 on
    /// (`volumeNum`).
    pub fn substitute(&self, code: i32, flag71: bool) -> Option<&EventAreaInfo> {
        let on = match code {
            71 => flag71,
            47 => self.volume >= 3,
            _ => false,
        };
        self.substitutes.iter().find(|e| e.code == code).filter(|_| on)
    }

    /// `WORLD_MAN::GetEventAreaInfo(n)` (0x0019d3b0): the first of the first
    /// [`AreaTables::search`] rows whose code is `code`, after the
    /// substitutes.
    pub fn event_area_info(&self, code: i32, flag71: bool) -> Option<&EventAreaInfo> {
        self.substitute(code, flag71).or_else(|| self.events.iter().take(self.search()).find(|e| e.code == code))
    }

    /// `ccGetEventAreaInfo(n)` (0x001a4220): the same over
    /// `eventAreaInfoNum` rows (the generator's lookup). Every volume has
    /// as many rows as it searches, so the two agree.
    pub fn cc_event_area_info(&self, code: i32, flag71: bool) -> Option<&EventAreaInfo> {
        self.substitute(code, flag71).or_else(|| self.events.iter().find(|e| e.code == code))
    }

    /// `WORLD_MAN::IsProtectArea()` (0x0019ceb0) with `eventAreaNumber` =
    /// `event`: the story area's `protect[1]` is not 0; false for 0 and for
    /// an area without a row. What field generation takes as
    /// [`crate::field::Params::protect`].
    pub fn is_protect_area(&self, event: i32, flag71: bool) -> bool {
        event != 0 && self.event_area_info(event, flag71).is_some_and(|e| e.protect[1] != 0)
    }

    /// `ccCheckEventAreaNum(a, b, c)` (0x001a4290) on `server`: the code of
    /// the first row with an address whose server is `server` and whose
    /// three words are the texts of words `a`, `b`, `c` (by ID); 0 for none.
    /// `None` where a word does not exist (the game reads through NULL).
    pub fn check_event_area_num(&self, a: i32, b: i32, c: i32, server: i32) -> Option<i32> {
        let texts = [self.word_by_id(a)?, self.word_by_id(b)?, self.word_by_id(c)?].map(|w| w.text);
        Some(
            self.events
                .iter()
                .find(|e| {
                    e.has_words()
                        && e.server == server
                        && e.words.iter().zip(texts).all(|(w, t)| w.is_some_and(|w| w == t))
                })
                .map_or(0, |e| e.code),
        )
    }

    /// `WORLD_MAN::GetWordParamFromEvCode(code, part)` (0x001a29a0): the
    /// word of slot `part` whose text is the story area's word `part`; the
    /// area is [`AreaTables::event_area_info`]'s. `None` where the game
    /// returns NULL (no such word: area 94's `Howling`) or reads through
    /// NULL (no such row, or a row without an address).
    pub fn word_from_event(&self, code: i32, part: Slot, flag71: bool) -> Option<&WordParam> {
        let e = self.event_area_info(code, flag71)?;
        self.word_by_text(part, (*e.words.get(part)?)?)
    }
}

// Read from the build's file, field by field in declaration order
// (`piney_gen::placement::area` writes them).

impl Load for WordAttrs {
    fn load(r: &mut Reader) -> Self {
        WordAttrs {
            field_type: Load::load(r),
            dungeon_size: Load::load(r),
            weather: Load::load(r),
            ground: Load::load(r),
            object: Load::load(r),
            area_level: Load::load(r),
            enemy_ofs: Load::load(r),
            item_ofs: Load::load(r),
            circle_ofs: Load::load(r),
        }
    }
}

impl Load for WordParam {
    fn load(r: &mut Reader) -> Self {
        WordParam {
            text: Load::load(r),
            slot: u32::load(r) as Slot,
            group: Load::load(r),
            id: Load::load(r),
            pri: Load::load(r),
            attrs: Load::load(r),
        }
    }
}

impl Load for EventAreaInfo {
    fn load(r: &mut Reader) -> Self {
        EventAreaInfo {
            code: Load::load(r),
            words: Load::load(r),
            server: Load::load(r),
            circle: Load::load(r),
            enemy: Load::load(r),
            item: Load::load(r),
            model: Load::load(r),
            flag: Load::load(r),
            field_type: Load::load(r),
            bgnum: Load::load(r),
            dungeon_num: Load::load(r),
            protect: Load::load(r),
        }
    }
}

impl Load for DungeonInfo {
    fn load(r: &mut Reader) -> Self {
        DungeonInfo { level_max: Load::load(r), room_max: Load::load(r) }
    }
}

impl Load for AreaTables {
    fn load(r: &mut Reader) -> Self {
        AreaTables {
            words: Load::load(r),
            dungeon_data: Load::load(r),
            events: Load::load(r),
            substitutes: Load::load(r),
            volume: Load::load(r),
        }
    }
}

/// What `WORLD_MAN::SimGenerateCode` leaves behind: the `WORLD_MAN` fields,
/// the merged `WORDPARAM` it writes through `WORLD_MAN.wordparam`, and the
/// RNG globals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AreaCode {
    /// `WORLD_MAN.A`, `B`, `C` (+0x14c .. +0x154): the three word IDs.
    pub a: i32,
    pub b: i32,
    pub c: i32,
    /// `A * 1000000 + B * 1000 + C`, the RNG's seed; `wordparam->ID`.
    pub code: i32,
    /// `WORLD_MAN.fieldSeed` (+0x44).
    pub field_seed: u32,
    /// `WORLD_MAN.dungeonSeed[3]` (+0x48).
    pub dungeon_seed: [u32; 3],
    /// `wordparam +0x0c .. +0x2c`: the merged attributes. `field_type` and
    /// `weather` are also `WORLD_MAN.fieldtype` (+0x10) and `bgnum` (+0xc),
    /// what `GetFieldType` and `GetBG` return.
    pub attrs: WordAttrs,
    /// `WORLD_MAN.eventAreaNumber` (+0x120): the story area these words make
    /// on this server, 0 for a random area.
    pub event: i32,
    /// `WORLD_MAN.timeSym` (+0x134): the first word is [`TIME_SYM_WORD`].
    pub time_sym: bool,
    /// `WORLD_MAN.dungeonLevelNum` (+0x138) and `dungeonRoomNum` (+0x13c):
    /// `dungeonData[dungeonSize]`.
    pub level_max: i32,
    pub room_max: i32,
    /// The global `seed` (0x00377d20) afterwards.
    pub seed: u32,
    /// Draws made (13): the global `randcnt` (0x00378a80) goes up by this;
    /// `SimGenerateCode` does not reset it.
    pub randcnt: u32,
    /// The save flag area 71 read as it was made ([`flag71`]).
    pub flag71: bool,
}

impl AreaCode {
    /// `WORLD_MAN.dungeonType[0..1]` (+0x34) as `SetDungeonTypeFromField`
    /// (0x0019cf50) sets them at the end of `SimGenerateCode`, which first
    /// sets `dungeonType[0]` to 0, by the volume's rule
    /// ([`AreaTables::dungeon_rule`]); `save_flag` is the save flag the
    /// rule names (Infection's crisis byte, a story bit from Mutation on).
    /// The function leaves `dungeonType[1]` alone outside field type 4;
    /// this gives 0 there, the value of a fresh `WORLD_MAN`.
    pub fn dungeon_type(&self, tables: &AreaTables, save_flag: bool) -> [u8; 2] {
        let flag =
            if self.event != 0 { tables.event_area_info(self.event, self.flag71).map_or(0, |e| e.flag) } else { 0 };
        tables.dungeon_rule().types(self.attrs.field_type as u32, self.event, flag, save_flag)
    }

    /// `game.field` for this area in `ccGame::ChangeScene` terms: the story
    /// area, 0 for a random one.
    pub fn field(&self) -> i32 {
        self.event
    }

    /// What `WORLD_MAN::SetGenerateCode(a, b, c)` (0x0019eea0) asks of
    /// `ccGame` once `SimGenerateCode` has made the area: its field, or for
    /// field type 4 (no field) its first dungeon.
    pub fn go(&self) -> Go {
        if self.attrs.field_type != 4 {
            Go::ChangeArea(1, self.event)
        } else if self.event == 0 {
            Go::ChangeArea(2, 0)
        } else {
            Go::ChangeScene([2, -2, self.event, 0, 0, 0])
        }
    }
}

/// A call of `ccGame::ChangeArea(area, n)` or `ccGame::ChangeScene(area,
/// town, field, dungeon, floor, block)`, for whoever holds the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Go {
    ChangeArea(i32, i32),
    ChangeScene([i32; 6]),
}

/// `saveData+0x5bc0` bit 62 (read as a doubleword), the flag area 71's
/// words read: byte 0x5bc7, bit 6.
pub fn flag71(save: &crate::save::SaveData) -> bool {
    save.u8(0x5bc7) & 0x40 != 0
}

/// `clamp(fieldType, weather)`: the jump tables at 0x003555c0 and
/// 0x003555f0.
pub fn clamp_weather(field_type: i32, weather: i32) -> i32 {
    match field_type {
        0..=4 if weather < 4 => weather,
        5 | 6 if weather == 8 => 4,
        5 | 6 if weather == 9 => 5,
        5 | 6 if weather < 4 => weather,
        7..=10 if weather < 8 => weather,
        0..=10 => 0,
        _ => weather,
    }
}

/// `WORLD_MAN::SimGenerateCode(a, b, c)` (0x0019e5c0) on `server`
/// (`ccGame.server`), with `flag71` the save flag area 71 reads
/// (`saveData+0x5bc0` bit 62). `None` where a word ID is not in its slot's
/// tables (the game reads through NULL).
pub fn sim_generate_code(t: &AreaTables, a: i32, b: i32, c: i32, server: i32, flag71: bool) -> Option<AreaCode> {
    let wa = t.word(0, a)?;
    let wb = t.word(1, b)?;
    let wc = t.word(2, c)?;
    let code = wa.id.wrapping_mul(1_000_000).wrapping_add(wb.id.wrapping_mul(1000)).wrapping_add(wc.id);
    let mut rng = dungeon::Rng::new(code as u32);
    let field_seed = rng.advance();
    let dungeon_seed = [rng.advance(), rng.advance(), rng.advance()];

    // Insertion sort by pri, ascending: the highest-priority word last.
    let mut order = [wa, wb, wc];
    for i in 1..3 {
        let mut j = i;
        while j > 0 && order[j].pri < order[j - 1].pri {
            order.swap(j, j - 1);
            j -= 1;
        }
    }

    let mut base = WordAttrs::default();
    base.field_type = (rng.advance() % 11) as i32;
    base.weather = clamp_weather(base.field_type, (rng.advance() % 10) as i32);
    base.dungeon_size = (rng.advance() % 10 + 1) as i32;
    base.ground = (rng.advance() % 3) as i32;
    base.object = (rng.advance() % 3) as i32;
    base.area_level = (rng.advance() % 5 + 1) as i32;
    base.enemy_ofs = (rng.advance() % 20) as i32 - 9;
    base.item_ofs = (rng.advance() % 20) as i32 - 9;
    base.circle_ofs = (rng.advance() % 3) as i32;

    let mut merged = order[0].attrs;
    merged.merge(&order[1].attrs);
    merged.merge(&order[2].attrs);
    base.merge(&merged);

    let event = t.check_event_area_num(a, b, c, server)?;
    if event != 0 {
        if let Some(info) = t.cc_event_area_info(event, flag71) {
            let pairs = [
                (&mut base.circle_ofs, info.circle),
                (&mut base.enemy_ofs, info.enemy),
                (&mut base.item_ofs, info.item),
                (&mut base.field_type, info.field_type),
                (&mut base.weather, info.bgnum),
            ];
            for (d, s) in pairs {
                if s != UNSET {
                    *d = s;
                }
            }
        }
        // Infection's cases for areas 71 and 47, written inline; from
        // Mutation on the lookups' substitutes carry them.
        if t.substitutes.is_empty() {
            if event == 71 && flag71 {
                base.enemy_ofs = 119;
                base.item_ofs = 119;
            }
            if event == 47 && t.volume >= 3 {
                base.enemy_ofs = 78;
                base.item_ofs = 122;
            }
        }
    }
    base.weather = clamp_weather(base.field_type, base.weather);
    if base.dungeon_size >= 11 {
        base.dungeon_size = 1;
    }
    let size = t.dungeon_data.get(base.dungeon_size.max(0) as usize).copied().unwrap_or_default();
    Some(AreaCode {
        a,
        b,
        c,
        code,
        field_seed,
        dungeon_seed,
        attrs: base,
        event,
        time_sym: a == TIME_SYM_WORD,
        level_max: size.level_max,
        room_max: size.room_max,
        seed: rng.seed,
        randcnt: rng.count,
        flag71,
    })
}

/// What the event instruction 118 `area n` (`ccEvAreaCodeAdd`, in
/// `ccEvent::Execute` at 0x001b04b8, while playing) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvArea {
    /// `n` 1-13: `WORLD_MAN::Quit()` when `game.area` is not 0 (the field or
    /// a dungeon is torn down), then `eventAreaNumber = n`. Nothing is
    /// generated.
    Number(i32),
    /// Any other `n` (0, 14 and on, negative): `SimGenerateCode` from the
    /// IDs of `GetWordParamFromEvCode(n, 0..2)` on the current server.
    Generated(AreaCode),
    /// Area `n` has no row, no address, or a word the tables lack (area 94);
    /// the game reads through NULL.
    Missing,
}

/// Instruction 118 `area n` on `server` (the current `ccGame.server`, which
/// the instruction does not change: on a server other than the story
/// area's the generated area is a random one, `event` 0).
pub fn ev_area(t: &AreaTables, n: i32, server: i32, flag71: bool) -> EvArea {
    if (1..=13).contains(&n) {
        return EvArea::Number(n);
    }
    let words = [0, 1, 2].map(|part| t.word_from_event(n, part, flag71).map(|w| w.id));
    match words {
        [Some(a), Some(b), Some(c)] => {
            sim_generate_code(t, a, b, c, server, flag71).map_or(EvArea::Missing, EvArea::Generated)
        }
        _ => EvArea::Missing,
    }
}

/// Story area `n` as its own address makes it on its own server: `area n`
/// run while `ccGame.server` is the area's. `None` for areas 1-13 (which
/// have no words), and where [`ev_area`] gives [`EvArea::Missing`].
pub fn story_area_code(t: &AreaTables, n: i32, flag71: bool) -> Option<AreaCode> {
    let server = t.event_area_info(n, flag71)?.server;
    match ev_area(t, n, server, flag71) {
        EvArea::Generated(code) => Some(code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exe::Image;
    use crate::iso::Iso;
    use std::path::PathBuf;

    fn tables() -> Option<AreaTables> {
        Some(AreaTables::of(Volume::Inf).clone())
    }

    /// Infection's addresses, for the check against the executable.
    const WORD_TABLES: [[(u32, u32); 3]; 4] = [
        [(0x0031_1790, 1392), (0x0031_1d00, 1344), (0x0031_2240, 1344)],
        [(0x0031_2780, 1200), (0x0031_2c30, 1200), (0x0031_30e0, 1248)],
        [(0x0031_35c0, 1056), (0x0031_39e0, 1152), (0x0031_3e60, 1104)],
        [(0x0031_42b0, 1152), (0x0031_4730, 1248), (0x0031_4c10, 1200)],
    ];
    const DUNGEON_DATA: u32 = 0x0031_50c0;
    const EVENT_AREA_INFO: u32 = 0x0031_5120;
    const EVENT_AREA_INFO_NUM: u32 = 0x0037_7d24;
    const VOLUME_NUM: u32 = 0x0034_bbf8;

    fn text(elf: &Image, va: u32) -> Option<String> {
        (va != 0).then(|| elf.cstr(va).unwrap().into_iter().map(char::from).collect())
    }

    /// The generated tables are what the executable holds, read at
    /// Infection's addresses independently of the generator.
    #[test]
    fn generated_tables_are_the_executables() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !p.exists() {
            return;
        }
        let elf = Image::elf(&Iso::open(p).unwrap().read_path("SLUS_202.67").unwrap()).unwrap();
        let t = AreaTables::of(Volume::Inf);
        let mut n = 0;
        for (g, row) in WORD_TABLES.iter().enumerate() {
            for (slot, &(va, size)) in row.iter().enumerate() {
                for i in 0..size / WORDPARAM_SIZE {
                    let at = va + WORDPARAM_SIZE * i;
                    let v = |k: u32| elf.i32(at + 4 * k).unwrap();
                    let w = &t.words[n];
                    n += 1;
                    assert_eq!(Some(w.text.to_string()), text(&elf, elf.u32(at).unwrap()));
                    assert_eq!((w.slot, w.group, w.id, w.pri), (slot, g as u8 + 1, v(1), v(2)));
                    let a = w.attrs;
                    let attrs = [
                        a.field_type,
                        a.dungeon_size,
                        a.weather,
                        a.ground,
                        a.object,
                        a.area_level,
                        a.enemy_ofs,
                        a.item_ofs,
                        a.circle_ofs,
                    ];
                    assert_eq!(attrs, std::array::from_fn(|k| v(3 + k as u32)));
                }
            }
        }
        assert_eq!(n, t.words.len());
        for (i, d) in t.dungeon_data.iter().enumerate() {
            let at = DUNGEON_DATA + 8 * i as u32;
            assert_eq!((d.level_max, d.room_max), (elf.i32(at).unwrap(), elf.i32(at + 4).unwrap()));
        }
        assert_eq!(t.events.len() as i32, elf.i32(EVENT_AREA_INFO_NUM).unwrap());
        for (i, e) in t.events.iter().enumerate() {
            let at = EVENT_AREA_INFO + EVENTAREA_INFO_SIZE * i as u32;
            let v = |k: u32| elf.i32(at + 4 * k).unwrap();
            let words = [1, 2, 3].map(|k| text(&elf, elf.u32(at + 4 * k).unwrap()));
            assert_eq!(e.words.map(|w| w.map(str::to_string)), words);
            let nums =
                [e.code, e.server, e.circle, e.enemy, e.item, e.model, e.flag, e.field_type, e.bgnum, e.dungeon_num];
            assert_eq!(nums, [0, 4, 5, 6, 7, 8, 9, 10, 11, 12].map(v));
            assert_eq!(e.protect, std::array::from_fn(|k| v(13 + k as u32)));
        }
        assert!(t.substitutes.is_empty());
        assert_eq!(t.volume, elf.i32(VOLUME_NUM).unwrap());
    }

    /// Each volume's tables carry its own `volumeNum`; from Mutation on
    /// areas 71 and 47 have substitutes.
    #[test]
    fn the_four_volumes() {
        for v in Volume::ALL {
            let t = AreaTables::of(v);
            assert_eq!(t.volume, v.number());
            assert_eq!(t.words.len(), 305);
            let subs: Vec<i32> = t.substitutes.iter().map(|e| e.code).collect();
            assert_eq!(subs, if v == Volume::Inf { vec![] } else { vec![71, 47] });
        }
        // Quarantine renames one keyword, slot b's 100.
        assert_eq!(AreaTables::of(Volume::Out).word(1, 100).unwrap().text, "Vengeful");
        assert_eq!(AreaTables::of(Volume::Qua).word(1, 100).unwrap().text, "Vindictive");
    }

    /// From Mutation on the lookups search 127 rows, the 127th area 126's,
    /// and give area 71's substitute only with the flag and area 47's only
    /// from volume 3 on.
    #[test]
    fn mutation_lookups() {
        assert_eq!(AreaTables::of(Volume::Inf).search(), 126);
        let t = AreaTables::of(Volume::Mut);
        assert_eq!(t.search(), 127);
        assert_eq!(t.event_area_info(126, false).map(|e| e.code), Some(126));
        let sub71 = t.event_area_info(71, true).unwrap();
        assert!(std::ptr::eq(sub71, &t.substitutes[0]));
        assert!(!std::ptr::eq(t.event_area_info(71, false).unwrap(), sub71));
        assert!(!std::ptr::eq(t.event_area_info(47, true).unwrap(), &t.substitutes[1]));
        let o = AreaTables::of(Volume::Out);
        assert!(std::ptr::eq(o.event_area_info(47, false).unwrap(), &o.substitutes[1]));
    }

    #[test]
    fn clamp() {
        assert_eq!(clamp_weather(0, 3), 3);
        assert_eq!(clamp_weather(4, 4), 0);
        assert_eq!(clamp_weather(5, 8), 4);
        assert_eq!(clamp_weather(6, 9), 5);
        assert_eq!(clamp_weather(6, 7), 0);
        assert_eq!(clamp_weather(10, 7), 7);
        assert_eq!(clamp_weather(9, 8), 0);
        assert_eq!(clamp_weather(11, 9), 9);
    }

    #[test]
    fn counts() {
        let Some(t) = tables() else { return };
        assert_eq!(t.words.len(), 305);
        assert_eq!(t.events.len(), 126);
        assert_eq!(t.events.iter().filter(|e| e.has_words()).count(), 113);
        assert_eq!(t.volume, 1);
        assert_eq!(t.dungeon_data[4], DungeonInfo { level_max: 3, room_max: 9 });
        assert_eq!(t.word_id("Bursting"), t.word_by_text(0, "Bursting").unwrap().id);
        assert_eq!(t.word_id("no such word"), -1);
    }

    #[test]
    fn bursting_passed_over_aqua_field() {
        let Some(t) = tables() else { return };
        let ids = ["Bursting", "Passed Over", "Aqua Field"].map(|s| t.word_id(s));
        let r = sim_generate_code(&t, ids[0], ids[1], ids[2], 0, false).unwrap();
        assert_eq!(r.code, 13026);
        assert_eq!(r.field_seed, 1_420_855);
        assert_eq!(r.dungeon_seed, [154_874_216, 3_996_388_677, 1_814_669_918]);
        let a = r.attrs;
        assert_eq!((a.field_type, a.weather, a.dungeon_size, a.ground, a.object), (10, 0, 6, 1, 1));
        assert_eq!((r.event, r.level_max, r.room_max, r.randcnt), (14, 4, 9, 13));
        assert!(!r.time_sym);
        // The same address on another server is a random area.
        assert_eq!(sim_generate_code(&t, ids[0], ids[1], ids[2], 1, false).unwrap().event, 0);
        // Instruction 118 reaches it from the area number.
        assert_eq!(ev_area(&t, 14, 0, false), EvArea::Generated(r));
        assert_eq!(story_area_code(&t, 14, false), Some(r));
        assert_eq!(ev_area(&t, 5, 0, false), EvArea::Number(5));
        // Area 94's Howling is not a keyword in Infection.
        assert_eq!(story_area_code(&t, 94, false), None);
        assert_eq!(r.dungeon_type(&t, false), [0, 0]);
    }

    #[test]
    fn protect_areas() {
        let Some(t) = tables() else { return };
        assert!(!t.is_protect_area(0, false));
        let n = (1..=130).filter(|&e| t.is_protect_area(e, false)).count();
        assert_eq!(n, 28);
    }
}
