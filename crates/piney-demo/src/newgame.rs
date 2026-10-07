//! `ccSaveData::NewGame(sw)` (`INF SLUS_202.67:0x00174d70`; MUT 0x00176270):
//! what a new game writes into the save. `ccSetupDemo` calls it with 1 on
//! every title, `ccThDemo` with 0 when New Game (or Parody) leaves: each of
//! the 18 (from Mutation 21) characters from DEMO.PRG's `charTbl`, the play
//! time, `SetDefaultWord`, `InitTradeItem`, and a Parody Kite's level. The
//! save keeps EE addresses: each character's `name` and `ccsname` point at
//! the executable's buffers (character 0's at the save itself, which
//! [`new_game`] takes). See docs/engine/title.md ("ccSaveData::NewGame").

use piney_data::save::{SaveData, by_id};
use piney_data::tables::newgame::{ItemList, NewGame, SpcParamData};
use piney_data::volume::Volume;

/// Entries of each trade list.
pub const TRADE_ITEMS: usize = 16;
/// `spcNameList` (`char[n][24]`) and `ccsNameList` (`char[n][32]`): where
/// the names are copied, outside the save (each volume's own address is its
/// table's `spc_name_list`, `ccs_name_list`).
pub const SPC_NAME_SIZE: usize = 24;
pub const CCS_NAME_SIZE: usize = 32;
/// Bytes of a name copied at most (the loops' bounds): the terminator is
/// not written when a name is this long.
pub const NAME_COPY: usize = 20;
pub const CCS_NAME_COPY: usize = 32;
/// Where the port says the save lives, for character 0's name pointer: the
/// address `tools/save.py` and `tools/test_demo_rs.py` give `saveData` in
/// eemu. The game's is wherever `new ccSaveData` put it on the heap, which
/// has not been measured.
pub const SAVE_VA: u32 = 0x0181_0000;
/// Character 0's level in a Parody game (volume 1; 50, 70, 90 in 2-4).
pub const PARODY_LEVEL: i16 = 20;

/// Kite's level in a Parody game, by `volumeNum`.
pub fn parody_level(volume: Volume) -> i16 {
    match volume {
        Volume::Inf => PARODY_LEVEL,
        Volume::Mut => 50,
        Volume::Out => 70,
        Volume::Qua => 90,
    }
}

/// `SetDefaultWord` for volume 1: the bits it ORs into `wordList`
/// (+0x523c), by offset (volumes 2-4 add more).
pub const DEFAULT_WORDS: [(usize, u32); 2] = [(0x5248, 0xfff0_0000), (0x524c, 0x001d_fffb)];

/// `SetDefaultWord` (main 0x00175500, the same code on every disc, keyed
/// on `volumeNum`): the bits it ORs into `wordList` for each volume, found
/// by running it in eemu with `volumeNum` 1-4.
pub fn default_words(volume: Volume) -> &'static [(usize, u32)] {
    match volume {
        Volume::Inf => &DEFAULT_WORDS,
        Volume::Mut => &[(0x5248, 0xfff0_0000), (0x524c, 0x003d_fffb), (0x5250, 0x9fff_1ffe), (0x5254, 0x0000_00ff)],
        Volume::Out => &[
            (0x5248, 0xfff0_0000),
            (0x524c, 0x003d_fffb),
            (0x5250, 0xdfff_1ffe),
            (0x5254, 0xefff_e0ff),
            (0x5258, 0x000f_f1ff),
        ],
        Volume::Qua => &[
            (0x5240, 0x0001_0000),
            (0x5248, 0xfff0_0000),
            (0x524c, 0x003d_fffb),
            (0x5250, 0xdfff_1ffe),
            (0x5254, 0xefff_e0ff),
            (0x5258, 0xfc0f_f1ff),
            (0x525c, 0xfe3f_fcff),
            (0x5260, 0x0000_00ff),
        ],
    }
}

/// Kite's important items (`impItemList`, +0xcfc, 320 bytes), and the
/// mail and news lists, which the later volumes' `NewGame(0)` writes.
const IMP_ITEM_LIST: usize = 0x0cfc;
const MAIL_LIST: usize = 0x2264;
const WEBNEWS_LIST: usize = 0x2864;

/// One write of the later volumes' `NewGame(0)` after `InitTradeItem`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Extra {
    /// `AddItem(0, 15, id, num)`: Kite's important item, up to 99.
    Item(usize, i32),
    /// `webnewsList[i]` and `mailList[i]` set.
    News(usize, u8),
    Mail(usize, u8),
}

/// The later volumes' `NewGame(0)` after `InitTradeItem`, in the game's
/// order: important item 5 (MUT); then item 70, two each of 0-5 and five
/// each of 26-41 (OUT); then item 285, news 19-22 set to 3, mails 56-60,
/// 62 and 63 cleared, two more each of 0-8 and five more of 26-41 (QUA).
fn new_game_extras(volume: Volume) -> Vec<Extra> {
    let mut out = Vec::new();
    if volume == Volume::Inf {
        return out;
    }
    out.push(Extra::Item(5, 1));
    if volume == Volume::Mut {
        return out;
    }
    out.push(Extra::Item(70, 1));
    out.extend((0..6).map(|id| Extra::Item(id, 2)));
    out.extend((26..42).map(|id| Extra::Item(id, 5)));
    if volume == Volume::Out {
        return out;
    }
    out.push(Extra::Item(285, 1));
    out.extend((19..23).map(|i| Extra::News(i, 3)));
    out.extend([57, 56, 58, 59, 60, 62, 63].map(|i| Extra::Mail(i, 0)));
    out.extend((0..9).map(|id| Extra::Item(id, 2)));
    out.extend((26..42).map(|id| Extra::Item(id, 5)));
    out
}

/// `ccSaveData` members `NewGame` writes (DWARF).
pub mod offset {
    pub const PL_NAME: usize = 0x0000;
    pub const SPC_TRADE_LIST: usize = 0x0e3c;
    pub const NPC_TRADE_LIST: usize = 0x127c;
    pub const TPC_TRADE_LIST_SW: usize = 0x1e7c;
    pub const SPC_PARAM: usize = 0x7488;
    pub const SPC_PARAM_SIZE: usize = 0xdc;
    pub const PLAY_TIME: usize = 0x8400;
    pub const PARODY_FLAG: usize = 0x842b;
    /// `ccSpcParam.velocity` (a float), within a character.
    pub const VELOCITY: usize = 0xd4;
    /// `ccItemList`: short id, char category, char num.
    pub const ITEM_LIST: usize = 4;
}

/// Which bytes of a `charTbl` row go where in its `ccSpcParam` (from, to,
/// length): `base` and `maxHP`/`maxSP` and `elm` in place (not the padding
/// Where `ccSpcParam` (the save's, 0xdc bytes) keeps what NewGame copies
/// from a `ccSpcParamData` (`charTbl`'s row): `base`, `maxHP`, `maxSP` and
/// `elm` at the row's own offsets, the rest further on.
mod spc_param {
    pub const MAX_HP: usize = 0x24;
    pub const MAX_SP: usize = 0x26;
    pub const ELM: usize = 0x28;
    pub const EQUIPMENT: usize = 0xc8;
    pub const VELOCITY: usize = 0xd4;
    pub const JOB: usize = 0xd8;
    pub const FRIENDSHIP: usize = 0xda;
}

/// What `NewGame` copies from the disc: the volume's generated tables
/// (`piney_data::tables::newgame`).
#[derive(Clone, Copy, Debug)]
pub struct NewGameTables {
    pub volume: Volume,
    pub t: &'static NewGame,
}

impl NewGameTables {
    pub fn of(volume: Volume) -> Self {
        NewGameTables { volume, t: piney_data::tables::newgame::of(volume) }
    }
}

/// The bytes a copy loop leaves of `name`: its Shift-JIS, at most `max`
/// bytes, then a NUL when there is room (a missing name copies a NUL).
pub fn copied(name: Option<&str>, max: usize) -> Vec<u8> {
    let mut out = name.map(piney_data::tables::sjis::encode).unwrap_or_default();
    out.truncate(max);
    if out.len() < max {
        out.push(0);
    }
    out
}

/// The names `NewGame` copies outside the save: `spcNameList` and
/// `ccsNameList` as it leaves them (only the bytes it writes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameLists {
    /// Characters 1-17's names (character 0's goes into `plName`).
    pub spc: Vec<Vec<u8>>,
    /// Every character's model file stem (`ctu1body` ...).
    pub ccs: Vec<Vec<u8>>,
}

/// `ccSaveData::NewGame(sw)` on `save`, which lives at EE address `save_va`
/// (only stored, as character 0's name pointer). A Parody game is the
/// save's own `parodyFlag` (+0x842b), as the game reads it.
pub fn new_game(save: &mut SaveData, sw: i32, tables: &NewGameTables, save_va: u32) -> NameLists {
    let mut names = NameLists::default();
    let parody = save.u8(offset::PARODY_FLAG) != 0;
    let b = save.record_mut();
    for (i, c) in tables.t.chars.iter().enumerate() {
        let p = by_id::spc_param(i);
        write_spc_param(&mut b[p..p + offset::SPC_PARAM_SIZE], c);
        // The name pointers: character 0's at the save's plName, the rest in
        // spcNameList; each model name in ccsNameList.
        let name_va = if i == 0 { save_va } else { tables.t.spc_name_list + (SPC_NAME_SIZE * (i - 1)) as u32 };
        let name = copied(c.base.name, NAME_COPY);
        if i == 0 {
            b[offset::PL_NAME..offset::PL_NAME + name.len()].copy_from_slice(&name);
        } else {
            names.spc.push(name);
        }
        b[p..p + 4].copy_from_slice(&name_va.to_le_bytes());
        let ccs_va = tables.t.ccs_name_list + (CCS_NAME_SIZE * i) as u32;
        names.ccs.push(copied(c.base.ccsname, CCS_NAME_COPY));
        b[p + 4..p + 8].copy_from_slice(&ccs_va.to_le_bytes());
        if sw == 0 && i == 0 && parody {
            b[p + 0x0e..p + 0x10].copy_from_slice(&parody_level(tables.volume).to_le_bytes());
        }
    }
    // ccResetPlayTime.
    b[offset::PLAY_TIME..offset::PLAY_TIME + 4].fill(0);
    // SetDefaultWord.
    for &(at, bits) in default_words(tables.volume) {
        let v = u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) | bits;
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    init_trade_item(b, tables);
    if sw == 0 {
        for e in new_game_extras(tables.volume) {
            match e {
                Extra::Item(id, num) => {
                    let at = IMP_ITEM_LIST + id;
                    let v = i32::from(b[at] as i8) + num;
                    b[at] = if v < 100 { v as u8 } else { 99 };
                }
                Extra::News(i, v) => b[WEBNEWS_LIST + i] = v,
                Extra::Mail(i, v) => b[MAIL_LIST + i] = v,
            }
        }
    }
    names
}

/// A `charTbl` row into its character's `ccSpcParam`: what `NewGame` and
/// `ConvGame` copy (`base`, `maxHP`, `maxSP`, `elm` in place, then the
/// equipment, `velocity`, `job` and `friendship`).
fn write_spc_param(r: &mut [u8], c: &SpcParamData) {
    c.base.write(r);
    r[spc_param::MAX_HP..spc_param::MAX_HP + 2].copy_from_slice(&c.max_hp.to_le_bytes());
    r[spc_param::MAX_SP..spc_param::MAX_SP + 2].copy_from_slice(&c.max_sp.to_le_bytes());
    c.elm.write(&mut r[spc_param::ELM..]);
    c.equipment.write(&mut r[spc_param::EQUIPMENT..]);
    r[spc_param::VELOCITY..spc_param::VELOCITY + 4].copy_from_slice(&c.velocity.to_le_bytes());
    r[spc_param::JOB..spc_param::JOB + 2].copy_from_slice(&c.job.to_le_bytes());
    r[spc_param::FRIENDSHIP..spc_param::FRIENDSHIP + 2].copy_from_slice(&c.friendship.to_le_bytes());
}

/// `ccSaveData::InitTradeItem` (MUT 0x00177be0) on the save's record.
fn init_trade_item(b: &mut [u8], tables: &NewGameTables) {
    // Each ccTradeList's item, list by list (the later volumes' last ones
    // in the extension); a list's unused slots are empty items (id -1,
    // category -1, none).
    let lists = [
        (tables.t.spc_trade, by_id::spc_trade_list as fn(usize) -> usize),
        (tables.t.npc_trade, by_id::npc_trade_list),
    ];
    for (src, at) in lists {
        for (list, trades) in src.iter().enumerate() {
            let dst = at(list);
            for k in 0..TRADE_ITEMS {
                let item = trades.get(k).map_or(EMPTY_ITEM, |e| e.lst);
                item.write(&mut b[dst + offset::ITEM_LIST * k..]);
            }
        }
    }
    // tpcTradeListSW[j][k] = tpcTradeList[j][0][0].category >= 0 for every k
    // (the loop never indexes by k).
    for (j, lists) in tables.t.tpc_trade.iter().enumerate() {
        let on = lists[0][0].category >= 0;
        for k in 0..3 {
            b[offset::TPC_TRADE_LIST_SW + 3 * j + k] = u8::from(on);
        }
    }
}

/// `ccSaveData::ConvGame()` (MUT 0x001767e0, OUT 0x00176020, QUA
/// 0x00175f80): CONVERT's save, just read from the previous volume, made
/// this volume's. Every character but Kite not in `partyMemberFlag` is
/// reset to its `charTbl` row, with 40 empty items and 20 empty skills;
/// then `LoadGame`, `InitTradeItem` and `newGameFlag` 2 (`InitSpcParam`
/// re-equips those characters).
pub fn conv_game(save: &mut SaveData, tables: &NewGameTables, save_va: u32) -> NameLists {
    let members = save.i32(piney_data::save::offset::PARTY_MEMBER_FLAG);
    let b = save.record_mut();
    for (i, c) in tables.t.chars.iter().enumerate().skip(1).filter(|&(i, _)| members & (1 << i) == 0) {
        let p = by_id::spc_param(i);
        write_spc_param(&mut b[p..p + offset::SPC_PARAM_SIZE], c);
        let items = by_id::item_list(i);
        for k in 0..CHAR_ITEMS {
            EMPTY_ITEM.write(&mut b[items + offset::ITEM_LIST * k..]);
        }
        let skills = by_id::skill_list(i);
        b[skills..skills + 2 * CHAR_SKILLS].fill(0xff);
    }
    let names = load_game(save, tables, save_va);
    init_trade_item(save.record_mut(), tables);
    save.set_u8(piney_data::save::offset::NEW_GAME_FLAG, 2);
    names
}

/// `ccStartEventConvert` (INF 0x001b55f0, MUT 0x001cac50): before
/// `ConvGame`, the carried save marks event 100, 200 or 300 done
/// (`100 * (volumeNum - 1)`); each later volume's first story event opens
/// on it.
pub fn start_event_convert(save: &mut SaveData, volume: Volume) {
    let n = 100 * (volume.number() - 1).clamp(0, 3) as usize;
    save.set_event_flag(n, save.event_flag(n) | piney_data::save::EVENT_DONE);
}

/// A character's item and skill lists' lengths (`GetItemList`,
/// `GetSkillList`), and an empty item (id -1, category -1, none).
const CHAR_ITEMS: usize = 40;
const CHAR_SKILLS: usize = 20;
const EMPTY_ITEM: ItemList = ItemList { id: -1, category: -1, num: 0 };

/// `ccSaveData::LoadGame()` (`INF SLUS_202.67:0x00175110`; MUT's for 21
/// characters through `GetSpcParam`) on a save just read from a card: each
/// character's `name` and `ccsname` pointed at the buffers again (character
/// 0's at the save, `save_va`), the `ccsNameList` names copied from `charTbl`
/// again, `velocity` (+0xd4) from `charTbl`, and `SetDefaultWord`; the names
/// in `spcNameList` are not touched. Outside the save it also sets the camera
/// type, vibration, display offset and sound environment.
pub fn load_game(save: &mut SaveData, tables: &NewGameTables, save_va: u32) -> NameLists {
    let mut names = NameLists::default();
    let b = save.record_mut();
    for (i, c) in tables.t.chars.iter().enumerate() {
        let p = by_id::spc_param(i);
        let name_va = if i == 0 { save_va } else { tables.t.spc_name_list + (SPC_NAME_SIZE * (i - 1)) as u32 };
        b[p..p + 4].copy_from_slice(&name_va.to_le_bytes());
        names.ccs.push(copied(c.base.ccsname, CCS_NAME_COPY));
        let ccs_va = tables.t.ccs_name_list + (CCS_NAME_SIZE * i) as u32;
        b[p + 4..p + 8].copy_from_slice(&ccs_va.to_le_bytes());
        b[p + offset::VELOCITY..p + offset::VELOCITY + 4].copy_from_slice(&c.velocity.to_le_bytes());
    }
    for &(at, bits) in default_words(tables.volume) {
        let v = u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) | bits;
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    names
}
