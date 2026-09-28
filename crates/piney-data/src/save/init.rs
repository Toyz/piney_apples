//! `ccSaveData::Init(flag)` (`INF SLUS_202.67:0x001743d0`): the record's
//! starting values, and the boot that runs it twice.
//!
//! Init is a run of member fills, not a clear: what it does not name keeps
//! its bytes. It never touches `plName`, the trade lists (`spcTradeList`,
//! `npcTradeList`, `tpcTradeListSW`: `NewGame`'s `InitTradeItem`),
//! `partyMemberCallStore`, `spcParam`, `reserved` or the padding. `flag`
//! 0 also writes the options (the camera type, vibration, voice, the
//! display offset, the volumes from `ccSnd`, `drainDemo`, `strWinMode`);
//! 1 leaves them. The game builds its one save in `ccThMother`
//! (0x00167940): `new ccSaveData` (the constructor 0x00174320 zeroes the
//! 0x8530 bytes and runs `Init(0)`), then, after `ccGame::Init`,
//! `Init(1)` ([`SaveData::boot`]).
//!
//! One loop runs long: `protectArea[5]` (+0x65c8) is cleared with the
//! bound of the `areaBan` loop before it, 32, so the 27 words after it,
//! `fountainRecord[0..27]`, end 0 although the loop before set every
//! `fountainRecord` to -1. The port keeps it.

use super::{SIZE, SaveData, by_id, ext, offset::*};
use crate::Result;
use crate::iso::Iso;
use crate::volume::Volume;

/// The later volumes' blocks Init clears (save.md, "Unknown": what reads
/// them is not known).
const LATER_BLOCK: usize = 0x8432;
const LATER_BYTES: usize = 0x8462;

/// `ORIGINAL_WALL_1` (`int`, 0x00378710, read as its low byte): a new
/// save's wallpaper, `WallTbl[49]`.
pub const ORIGINAL_WALL_1: i8 = 49;
/// `MAX_WAVE_NUM` (0x00378754): a new save's desktop music, `Wave[50]`.
pub const MAX_WAVE_NUM: i8 = 50;
/// The eleven button assignments from `assignPADaction` (+0x8404) as Init
/// writes them: action cross, personal menu triangle, chat square, option
/// start, map select, ok cross, cancel circle, camera in R1, out R2, reset
/// L1, mode L2 (the pad's bits as `ccPad` reads them).
pub const ASSIGN_PAD_DEFAULT: [i16; 11] = [0x40, 0x10, 0x80, 0x800, 0x100, 0x40, 0x20, 0x08, 0x02, 0x04, 0x01];

const TIME_IDOL_RANKS: usize = 5;
const TIME_IDOL_FIELD: usize = 20;

/// What `Init(0)` copies from `ccSnd` into the options: `mainVol` (+0x0),
/// `bgmVol` (+0x4), `seVol` (+0x8), `outputMode` (+0xc), each stored as a
/// short.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundLevels {
    pub main: i32,
    pub bgm: i32,
    pub se: i32,
    pub output: i32,
}

impl SoundLevels {
    /// `ccSound::ccSound` (0x00180ed0): volumes 256, output 1 (stereo;
    /// `ccSoundMain` writes the 1 again). The sound task is running and
    /// waited for (`ccSndSysWait`) before `ccThMother` builds the save, so
    /// these are what the constructor's `Init(0)` copies.
    pub const BOOT: SoundLevels = SoundLevels { main: 256, bgm: 256, se: 256, output: 1 };
}

/// The text Init copies: `timeIdolRankDefStr`, read from the executable,
/// and the volume whose Init runs. [`InitText::default`] has none and is
/// Infection's: each string is empty, so Init writes a lone NUL where the
/// game writes the name or the time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InitText {
    /// `timeIdolRankDefStr[2 * rank + k]`, k 0 the name, 1 the time.
    pub time_idol_rank: Vec<Vec<u8>>,
    pub volume: Volume,
}

impl InitText {
    /// The volume's: `timeIdolRankDefStr` (`crate::tables::title`, alike on
    /// every disc).
    pub fn of(volume: Volume) -> Self {
        let time_idol_rank =
            crate::tables::title::TIME_IDOL_RANK.iter().map(|s| crate::tables::sjis::encode(s)).collect();
        InitText { time_idol_rank, volume }
    }

    /// The disc's volume's ([`InitText::of`]).
    pub fn from_disc(disc: &mut Iso) -> Result<Self> {
        Ok(Self::of(disc.volume()?))
    }
}

/// A list's fill: Init's writes for it.
type Fill = fn(&mut SaveData);

impl SaveData {
    /// The save `ccThMother` builds at boot: zeroed, `Init(0)` with the
    /// sound task's starting levels (the constructor), then `Init(1)`.
    pub fn boot(text: &InitText) -> SaveData {
        let mut s = SaveData::new();
        s.init(0, &SoundLevels::BOOT, text);
        s.init(1, &SoundLevels::BOOT, text);
        s
    }

    /// `ccSaveData::Init(flag)` (0x001743d0; MUT 0x00175740, the same in
    /// OUT and QUA), in the game's order, for `text.volume`. `snd` is read
    /// only when `flag` is 0.
    ///
    /// The later volumes' Init differs in five places, each keyed below on
    /// `later`: the lists of 21 characters through the accessors
    /// ([`by_id`]), 18-20's in the extension; `talkNum` through its setter;
    /// the trade counts through theirs (characters 1-20, NPCs 30-79 and the
    /// extension's six), which leaves Infection's last 10 bytes of
    /// `pcTradeCount` alone; `partyTime` and `spcPresent` through theirs
    /// for ids 0-19, id 0 writing the word before each (the last of
    /// `enemyKillArea`, then of `partyTime`) and 20 left; and the blocks at
    /// +0x8432 and +0x8462 cleared, then the extension's tail.
    pub fn init(&mut self, flag: i32, snd: &SoundLevels, text: &InitText) {
        let later = text.volume != Volume::Inf;
        self.set_u8(PL_REAL_NAME, 0);
        self.fill_items(PL_ITEM_LIST, 99);
        self.fill(IMP_ITEM_LIST, 320, 0);
        for c in 0..text.volume.characters() {
            self.fill_items(by_id::item_list(c), 40);
            self.fill_i16(by_id::skill_list(c), 20, -1);
        }
        self.set_i32(PARTY_MEMBER_FLAG, 0);
        self.set_i32(PARTY_MEMBER_CALL, 0x3fffe);
        self.set_i32(PARTY_MEMBER_EXP, 0);
        self.set_i32(PARTY_MEMBER_SAVE, 0);
        self.set_i16(TOWN_MOVE_FLAG, 1);
        self.set_u8(LAST_TOWN, 0);
        for (k, &b) in ASSIGN_PAD_DEFAULT.iter().enumerate() {
            self.set_i16(ASSIGN_PAD_ACTION + 2 * k, b);
        }
        self.set_u8(NEW_GAME_FLAG, 0);
        if flag == 0 {
            self.set_u8(CAM_TYPE, 0);
            self.set_u8(VIBRATION, 1);
            self.set_u8(VOICE, 1);
            self.set_i16(SCREEN_X, 0);
            self.set_i16(SCREEN_Y, 0);
            self.set_i16(MAIN_VOL, snd.main as i16);
            self.set_i16(SE_VOL, snd.se as i16);
            self.set_i16(BGM_VOL, snd.bgm as i16);
            self.set_i16(OUTPUT, snd.output as i16);
            self.set_u8(DRAIN_DEMO, 1);
        }
        self.set_i32(PLAY_TIME, 0);
        self.set_u8(CLEAR_FLAG, 0);
        self.set_u8(PARODY_FLAG, 0);
        self.fill(GATE_LIST, 4 * 25, 0);
        self.fill(GATE_LIST_MARK, 4 * 25, 0);
        self.fill_i16(GATE_ORDER_LIST, 5 * 64, -1);
        self.fill(GATE_RECORD, 4 * 5 * 20, 0xff);
        self.fill(WORD_LIST, 4 * 15, 0);
        self.fill(EVENT_FLAG, 8 * 512, 0);
        self.fill(EVENT_STATUS, 80, 0);
        self.fill(EVENT_ENTRY, 4 * 160 * 6, 0xff);
        self.set_u8(DT_WALLPAPER, ORIGINAL_WALL_1 as u8);
        self.fill(DT_WALLPAPER_LIST, 4 * 3, 0);
        self.set_u8(DT_BGM, MAX_WAVE_NUM as u8);
        self.fill(DT_BGM_LIST, 4 * 3, 0);
        self.fill(DT_STR_LIST, 4 * 5, 0);
        self.fill(MAIL_LIST, 512, 0);
        self.fill_i16(MAIL_ORDER_LIST, 512, -1);
        self.fill(WEBNEWS_LIST, 128, 0);
        self.fill(BBS_LIST, 128 * 48, 0);
        self.set_i16(FOUNTAIN_NUM, 0);
        self.fill(FOUNTAIN_RECORD, 4 * 100, 0xff);
        self.fill(AREA_BAN, 32 * 4, 0xff);
        // `protectArea[i] = 0` for i < 32: through fountainRecord[26].
        self.fill(PROTECT_AREA, 4 * 32, 0);
        self.set_i16(EROSION, 0);
        self.set_u8(CRISIS, 0);
        self.set_u8(TACTICS, 7);
        self.set_u8(PLCOL, 0);
        for g in 0..5 {
            let at = GROWTH + GROWTH_SIZE * g;
            self.fill(at, 0x14, 0);
            self.set_i32(at + 0x14, -1);
        }
        if later {
            for id in 0..text.volume.characters() {
                self.set_u8(by_id::talk_num(id), 0);
            }
        } else {
            self.fill(TALK_NUM, 18, 0);
        }
        self.set_u8(TIME_IDOL_RANK, 0xff);
        for rank in 0..TIME_IDOL_RANKS {
            for k in 0..2 {
                let s = text.time_idol_rank.get(2 * rank + k).map_or(&[][..], Vec::as_slice);
                self.strcpy(TIME_IDOL_RANK_STR + 2 * TIME_IDOL_FIELD * rank + TIME_IDOL_FIELD * k, s);
            }
        }
        self.fill(HY_PROCCESS, 8 * 4, 0);
        self.set_u8(HY_ITEM, 0);
        self.fill(DRAIN_COUNT, 2 * 6, 0);
        if later {
            let codes = (1..21).map(|c| (4, c)).chain((30..80).map(|c| (8, c)));
            for (kind, code) in codes.chain(ext::NPC_TRADE_CODES.iter().map(|&c| (8, c))) {
                if let Some(at) = by_id::trade_count(kind, code) {
                    self.set_u8(at, 0xff);
                }
            }
        } else {
            self.fill(PC_TRADE_COUNT, 77, 0xff);
        }
        self.fill(ENEMY_KILL_COUNT, 313, 0);
        self.fill_i16(ENEMY_KILL_AREA, 313 * 4, -1);
        if later {
            for id in 0..20 {
                self.set_i32(by_id::party_time(id), 0);
            }
            for id in 0..20 {
                self.set_i32(by_id::spc_present(id), 0);
            }
        } else {
            self.fill(PARTY_TIME, 4 * 17, 0);
            self.fill(SPC_PRESENT, 4 * 17, 0);
        }
        self.fill(ITEM_BOX_COUNT, 2 * 3, 0);
        self.fill(PUCCI_FOOD_COUNT, 2 * 20, 0);
        self.fill(ITEM_IDOL_COUNT, 2 * 3, 0);
        self.fill(INU_COUNT, 2 * 9, 0);
        self.fill(MAP_MODE, 3, 0);
        if flag == 0 {
            self.set_u8(STR_WIN_MODE, 1);
        }
        self.set_u8(CAMERA_MODE, 3);
        if later {
            // Four rows of three: two shorts at +0x8432 (12 bytes a row),
            // a byte at +0x8462 (3 a row).
            for r in 0..4 {
                for k in 0..3 {
                    self.set_i16(LATER_BLOCK + 12 * r + 4 * k, 0);
                    self.set_i16(LATER_BLOCK + 12 * r + 4 * k + 2, 0);
                    self.set_u8(LATER_BYTES + 3 * r + k, 0);
                }
            }
            self.fill(ext::TAIL, 0x100, 0);
        }
    }

    /// A repair for saves the port itself wrote before it ran `Init`
    /// whole, applied when a slot is loaded: those saves hold only ok and
    /// cancel of the eleven button assignments and zero bytes where Init
    /// fills lists with -1. Neither is a state the game can reach, so a
    /// save the game wrote is never changed:
    ///
    /// - `assignPADaction` .. `assignPADmap` (+0x8404 .. +0x840e) all zero
    ///   with `assignPADok` and `assignPADcancel` set: each assignment that
    ///   is zero gets Init's (the game has no way to leave one unassigned);
    /// - a whole list that is zero bytes where Init writes -1 gets Init's
    ///   fill: `itemList` and `plItemList` (id and category -1, count 0;
    ///   `AddItem` merges an item into its entry, so at most one entry of a
    ///   list can be item 0 of category 0), `skillList` (`ccAddSkill` adds
    ///   a skill once), `mailOrderList` (`NewMail` adds a mail once),
    ///   `gateOrderList`, `gateRecord`, `fountainRecord` and `areaBan`
    ///   (`SetGateList`, `SetGateRecord`, `SetFountain` and `SetAreaBan`
    ///   add an entry once). `fountainRecord` gets Init's own result, the
    ///   first 27 left 0.
    ///
    /// Left alone because zero is a state the game can reach:
    /// `pcTradeCount` (-1 no trade, `AddTradeCount` counts from there),
    /// `eventEntry` (bits `ClearEventEntry` clears), `enemyKillArea` and
    /// `growth`. True when anything changed.
    pub fn repair_port_save(&mut self) -> bool {
        let mut changed = false;
        let zero = |s: &SaveData, at: usize, len: usize| s.0[at..at + len].iter().all(|&b| b == 0);
        if zero(self, ASSIGN_PAD_ACTION, ASSIGN_PAD_OK - ASSIGN_PAD_ACTION)
            && self.i16(ASSIGN_PAD_OK) != 0
            && self.i16(ASSIGN_PAD_CANCEL) != 0
        {
            for (k, &b) in ASSIGN_PAD_DEFAULT.iter().enumerate() {
                if self.i16(ASSIGN_PAD_ACTION + 2 * k) == 0 {
                    self.set_i16(ASSIGN_PAD_ACTION + 2 * k, b);
                }
            }
            changed = true;
        }
        let lists: [(usize, usize, Fill); 8] = [
            (ITEM_LIST, 18 * 40 * 4, |s| (0..18).for_each(|c| s.fill_items(ITEM_LIST + 0xa0 * c, 40))),
            (PL_ITEM_LIST, 99 * 4, |s| s.fill_items(PL_ITEM_LIST, 99)),
            (SKILL_LIST, 18 * 20 * 2, |s| s.fill_i16(SKILL_LIST, 18 * 20, -1)),
            (MAIL_ORDER_LIST, 512 * 2, |s| s.fill_i16(MAIL_ORDER_LIST, 512, -1)),
            (GATE_ORDER_LIST, 5 * 64 * 2, |s| s.fill_i16(GATE_ORDER_LIST, 5 * 64, -1)),
            (GATE_RECORD, 5 * 20 * 4, |s| s.fill(GATE_RECORD, 5 * 20 * 4, 0xff)),
            (FOUNTAIN_RECORD, 100 * 4, |s| s.fill(FOUNTAIN_RECORD + 27 * 4, 73 * 4, 0xff)),
            (AREA_BAN, 32 * 4, |s| s.fill(AREA_BAN, 32 * 4, 0xff)),
        ];
        for (at, len, fill) in lists {
            if zero(self, at, len) {
                fill(self);
                changed = true;
            }
        }
        changed
    }

    fn fill(&mut self, at: usize, len: usize, v: u8) {
        self.0[at..at + len].fill(v);
    }

    fn fill_i16(&mut self, at: usize, n: usize, v: i16) {
        for i in 0..n {
            self.set_i16(at + 2 * i, v);
        }
    }

    /// `n` `ccItemList` entries: id -1, category -1, count 0.
    fn fill_items(&mut self, at: usize, n: usize) {
        for i in 0..n {
            self.0[at + 4 * i..at + 4 * i + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0]);
        }
    }

    /// `strcpy`: the bytes and their NUL, however long (clipped at the end
    /// of the record).
    fn strcpy(&mut self, at: usize, s: &[u8]) {
        for (k, &b) in s.iter().chain(&[0]).enumerate() {
            if at + k < SIZE {
                self.0[at + k] = b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> InitText {
        InitText { time_idol_rank: (0..10).map(|i| vec![b'a' + i as u8; i + 1]).collect(), ..InitText::default() }
    }

    #[test]
    fn init_fills_the_lists() {
        let s = SaveData::boot(&InitText::default());
        // The bag: every entry empty (id -1, category -1, count 0).
        assert_eq!(&s.bytes()[ITEM_LIST..ITEM_LIST + 4], &[0xff, 0xff, 0xff, 0]);
        assert_eq!(&s.bytes()[PL_ITEM_LIST + 98 * 4..PL_ITEM_LIST + 99 * 4], &[0xff, 0xff, 0xff, 0]);
        assert_eq!(s.i16(SKILL_LIST + 2 * (18 * 20 - 1)), -1);
        assert_eq!(s.i32(PARTY_MEMBER_CALL), 0x3fffe);
        assert_eq!((s.i16(ASSIGN_PAD_OK), s.i16(ASSIGN_PAD_CANCEL)), (0x40, 0x20));
        assert_eq!((s.i16(MAIN_VOL), s.i16(OUTPUT), s.u8(STR_WIN_MODE), s.u8(CAMERA_MODE)), (256, 1, 1, 3));
        // protectArea's loop runs over fountainRecord[0..27].
        assert_eq!((s.i32(FOUNTAIN_RECORD + 4 * 26), s.i32(FOUNTAIN_RECORD + 4 * 27)), (0, -1));
        assert_eq!(s.i32(GROWTH + GROWTH_SIZE * 4 + 0x14), -1);
        assert_eq!(s.u8(TIME_IDOL_RANK) as i8, -1);
    }

    #[test]
    fn init_1_leaves_the_options() {
        let mut s = SaveData::new();
        s.set_i16(MAIN_VOL, 99);
        s.set_u8(STR_WIN_MODE, 0);
        s.init(1, &SoundLevels { main: 1, bgm: 2, se: 3, output: 4 }, &text());
        assert_eq!((s.i16(MAIN_VOL), s.u8(STR_WIN_MODE), s.u8(DRAIN_DEMO)), (99, 0, 0));
        s.init(0, &SoundLevels { main: 1, bgm: 2, se: 3, output: 4 }, &text());
        assert_eq!((s.i16(MAIN_VOL), s.i16(BGM_VOL), s.i16(SE_VOL), s.i16(OUTPUT)), (1, 2, 3, 4));
        assert_eq!(s.cstr(TIME_IDOL_RANK_STR + 20, 20), b"bb");
        assert_eq!(s.cstr(TIME_IDOL_RANK_STR + 4 * 40 + 20, 20), b"jjjjjjjjjj");
    }

    #[test]
    fn repair_restores_the_port_s_old_saves_only() {
        let good = SaveData::boot(&text());
        // An old port save: ok and cancel only, the lists zero.
        let mut old = good.clone();
        old.bytes_mut()[ASSIGN_PAD_ACTION..ASSIGN_PAD_ACTION + 22].fill(0);
        old.set_i16(ASSIGN_PAD_OK, 0x40);
        old.set_i16(ASSIGN_PAD_CANCEL, 0x20);
        for (at, len) in [
            (ITEM_LIST, 0xb40),
            (PL_ITEM_LIST, 0x18c),
            (SKILL_LIST, 0x2d0),
            (GATE_ORDER_LIST, 0x280),
            (GATE_RECORD, 0x190),
            (FOUNTAIN_RECORD, 0x190),
            (AREA_BAN, 0x80),
        ] {
            old.bytes_mut()[at..at + len].fill(0);
        }
        assert!(old.repair_port_save());
        assert_eq!(old, good);
        assert!(!old.repair_port_save());
        // What the game writes is kept: a save with an item, a moved
        // button, pcTradeCount zero.
        let mut real = good.clone();
        real.set_i16(ITEM_LIST, 0);
        real.set_u8(ITEM_LIST + 2, 0);
        real.set_u8(ITEM_LIST + 3, 1);
        real.set_i16(ASSIGN_PAD_ACTION, 0x20);
        real.bytes_mut()[PC_TRADE_COUNT..PC_TRADE_COUNT + 77].fill(0);
        let before = real.clone();
        assert!(!real.repair_port_save());
        assert_eq!(real, before);
    }
}
