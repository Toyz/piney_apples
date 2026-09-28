//! The game's save data: `ccSaveData`, 0x8530 bytes, the one record every
//! part of the game reads and writes, and what a memory card slot file holds
//! (`docs/formats/save.md`). [`SaveData`] keeps the bytes as the game lays
//! them out, so a slot file reads and writes as it is; the members the port
//! uses have accessors, at Infection's DWARF offsets (the same on every
//! volume up to +0x8432). From Mutation on, the 0x854-byte extension for
//! characters 18-20 follows the 0x8530 bytes as in the slot file ([`EXT`]);
//! on Infection it is zero and unwritten.

use crate::{Error, Result};

mod init;
pub use init::{ASSIGN_PAD_DEFAULT, InitText, MAX_WAVE_NUM, ORIGINAL_WALL_1, SoundLevels};

/// `sizeof(ccSaveData)`: the constructor's clear loop bound, and Infection's
/// slot file size.
pub const SIZE: usize = 0x8530;
/// The extension's size (`new(0x854)` in the constructor, Mutation on) and
/// where the record keeps it: after `ccSaveData`, as the slot file does.
pub const EXT_SIZE: usize = 0x854;
pub const EXT: usize = SIZE;
/// The record with its extension: the later volumes' slot file.
pub const FULL: usize = SIZE + EXT_SIZE;

/// The extension's members, as offsets into the record (save.md, "The
/// extension"; the accessors of MUT 0x0017a010 - 0x0017b050).
pub mod ext {
    use super::EXT;
    /// `itemList[3][40]`, characters 18-20.
    pub const ITEM_LIST: usize = EXT;
    /// `spcTradeList[3][16]`: trade lists 17-19.
    pub const SPC_TRADE_LIST: usize = EXT + 0x1e0;
    /// `npcTradeList[6][16]`: NPC trade lists 48-53.
    pub const NPC_TRADE_LIST: usize = EXT + 0x2a0;
    /// `skillList[3][20]`.
    pub const SKILL_LIST: usize = EXT + 0x420;
    /// `talkNum[3]`, `partyTime[3]`, `spcPresent[3]`, `spcParam[3]`.
    pub const TALK_NUM: usize = EXT + 0x498;
    pub const PARTY_TIME: usize = EXT + 0x49c;
    pub const SPC_PRESENT: usize = EXT + 0x4a8;
    pub const SPC_PARAM: usize = EXT + 0x4b4;
    /// The trade counts moved here: characters 18-20's, then NPCs 181,
    /// 180, 182, 183, 121 and 120's (the setter MUT 0x0017a010).
    pub const PC_TRADE_COUNT: usize = EXT + 0x748;
    pub const NPC_TRADE_COUNT: usize = EXT + 0x74b;
    /// The NPC codes whose trade counts are here, in order.
    pub const NPC_TRADE_CODES: [i32; 6] = [181, 180, 182, 183, 121, 120];
    /// 16 x 4 words the constructor's helper (MUT 0x0017b050) clears.
    pub const TAIL: usize = EXT + 0x754;
}

/// Where a character's records are: ids 0-17 in `ccSaveData`, 18-20 in the
/// extension (Mutation's `GetItemList` 0x0017a9d0, `GetSkillList`
/// 0x0017aba0, `GetSpcParam` 0x0017aec0, and the setters of `talkNum`
/// 0x0017ac30, `partyTime` 0x0017aca0 and `spcPresent` 0x0017adf0).
/// `partyTime` and `spcPresent` count from character 1, so id 0 is the
/// word before each (the game's own loops from 0 write it).
pub mod by_id {
    use super::{ext, offset};
    const IN_MAIN: usize = 18;

    pub fn item_list(id: usize) -> usize {
        if id < IN_MAIN { offset::ITEM_LIST + 0xa0 * id } else { ext::ITEM_LIST + 0xa0 * (id - IN_MAIN) }
    }
    pub fn skill_list(id: usize) -> usize {
        if id < IN_MAIN { offset::SKILL_LIST + 40 * id } else { ext::SKILL_LIST + 40 * (id - IN_MAIN) }
    }
    pub fn spc_param(id: usize) -> usize {
        if id < IN_MAIN {
            offset::SPC_PARAM + SPC_PARAM_SIZE * id
        } else {
            ext::SPC_PARAM + SPC_PARAM_SIZE * (id - IN_MAIN)
        }
    }
    pub fn talk_num(id: usize) -> usize {
        if id < IN_MAIN { offset::TALK_NUM + id } else { ext::TALK_NUM + (id - IN_MAIN) }
    }
    pub fn party_time(id: usize) -> usize {
        if id < IN_MAIN { offset::PARTY_TIME + 4 * id - 4 } else { ext::PARTY_TIME + 4 * (id - IN_MAIN) }
    }
    pub fn spc_present(id: usize) -> usize {
        if id < IN_MAIN { offset::SPC_PRESENT + 4 * id - 4 } else { ext::SPC_PRESENT + 4 * (id - IN_MAIN) }
    }
    /// `ccSpcParam`'s size.
    pub const SPC_PARAM_SIZE: usize = 0xdc;

    /// Character trade list `i` (character `i + 1`'s): 0-16 in
    /// `ccSaveData`, 17-19 in the extension (MUT `GetSpcTradeList`
    /// 0x0017aa20).
    pub fn spc_trade_list(i: usize) -> usize {
        if i + 1 < IN_MAIN { offset::SPC_TRADE_LIST + 64 * i } else { ext::SPC_TRADE_LIST + 64 * (i + 1 - IN_MAIN) }
    }
    /// NPC trade list `k`: 0-47 in `ccSaveData`, 48-53 in the extension
    /// (MUT `InitTradeItem` 0x00177be0).
    pub fn npc_trade_list(k: usize) -> usize {
        if k < 48 { offset::NPC_TRADE_LIST + 64 * k } else { ext::NPC_TRADE_LIST + 64 * (k - 48) }
    }

    /// The byte of `pcTradeCount` the later volumes' setter (MUT
    /// 0x0017a010) writes for a trader: kind bit 4 a party character (1-17
    /// in `ccSaveData`, 18-20 in the extension), bits 0x18 an NPC (codes
    /// 30-79 in `ccSaveData`, and the six of [`ext::NPC_TRADE_CODES`]); None
    /// for any other.
    pub fn trade_count(kind: i32, code: i32) -> Option<usize> {
        if kind & 4 != 0 {
            let c = usize::try_from(code).ok()?;
            return Some(if code < 18 { offset::PC_TRADE_COUNT - 1 + c } else { ext::PC_TRADE_COUNT - 18 + c });
        }
        if kind & 0x18 == 0 {
            return None;
        }
        if (30..80).contains(&code) {
            return Some(offset::HY_ITEM + code as usize);
        }
        let k = ext::NPC_TRADE_CODES.iter().position(|&c| c == code)?;
        Some(ext::NPC_TRADE_COUNT + k)
    }
}

/// Member offsets (`ccSaveData`, from the DWARF).
pub mod offset {
    pub const PL_NAME: usize = 0x0000;
    pub const PL_REAL_NAME: usize = 0x0018;
    pub const TOWN_MOVE_FLAG: usize = 0x2234;
    pub const DT_WALLPAPER: usize = 0x2236;
    pub const DT_BGM: usize = 0x2237;
    pub const DT_WALLPAPER_LIST: usize = 0x2238;
    pub const DT_BGM_LIST: usize = 0x2244;
    pub const DT_STR_LIST: usize = 0x2250;
    pub const MAIL_LIST: usize = 0x2264;
    pub const MAIL_ORDER_LIST: usize = 0x2464;
    pub const WEBNEWS_LIST: usize = 0x2864;
    pub const BBS_LIST: usize = 0x28e4;
    pub const EVENT_ENTRY: usize = 0x40e4;
    pub const GATE_LIST: usize = 0x4fe4;
    pub const WORD_LIST: usize = 0x523c;
    pub const EVENT_FLAG: usize = 0x54f8;
    pub const EVENT_STATUS: usize = 0x64f8;
    pub const AREA_BAN: usize = 0x6548;
    pub const PROTECT_AREA: usize = 0x65c8;
    pub const EROSION: usize = 0x676e;
    pub const NEW_GAME_FLAG: usize = 0x6770;
    pub const PLCOL: usize = 0x6771;
    pub const CRISIS: usize = 0x6772;
    pub const PLAY_TIME: usize = 0x8400;
    pub const LAST_TOWN: usize = 0x8426;
    pub const CLEAR_FLAG: usize = 0x842a;
    /// `assignPADaction`: the field's action button (talk, act on a target).
    pub const ASSIGN_PAD_ACTION: usize = 0x8404;
    pub const ASSIGN_PAD_OK: usize = 0x840e;
    pub const ASSIGN_PAD_CANCEL: usize = 0x8410;
    pub const PARODY_FLAG: usize = 0x842b;
    /// The options the START menu sets (`ccSaveData`, shorts then bytes).
    pub const SCREEN_X: usize = 0x841a;
    pub const SCREEN_Y: usize = 0x841c;
    pub const MAIN_VOL: usize = 0x841e;
    pub const SE_VOL: usize = 0x8420;
    pub const BGM_VOL: usize = 0x8422;
    pub const OUTPUT: usize = 0x8424;
    pub const VIBRATION: usize = 0x8428;
    pub const CAM_TYPE: usize = 0x8429;
    pub const VOICE: usize = 0x842c;
    pub const STR_WIN_MODE: usize = 0x8430;
    /// `cameraMode` (s8): the field camera's type, 3 following, 1 the eye
    /// view (L2 switches and writes it).
    pub const CAMERA_MODE: usize = 0x8431;
    /// `ccItemList itemList[18][40]`: `short id; char category; char count`.
    pub const ITEM_LIST: usize = 0x0030;
    pub const IMP_ITEM_LIST: usize = 0x0cfc;
    /// `spcTradeList[17][16]`, `npcTradeList[48][16]`, `tpcTradeListSW[24][3]`.
    pub const SPC_TRADE_LIST: usize = 0x0e3c;
    pub const NPC_TRADE_LIST: usize = 0x127c;
    pub const TPC_TRADE_LIST_SW: usize = 0x1e7c;
    /// `short skillList[18][20]`.
    pub const SKILL_LIST: usize = 0x1ec4;
    pub const TALK_NUM: usize = 0x220c;
    pub const PARTY_MEMBER_FLAG: usize = 0x2220;
    pub const PARTY_MEMBER_CALL: usize = 0x2224;
    pub const PARTY_MEMBER_CALL_STORE: usize = 0x2228;
    pub const PARTY_MEMBER_EXP: usize = 0x222c;
    pub const PARTY_MEMBER_SAVE: usize = 0x2230;
    pub const GATE_LIST_MARK: usize = 0x5048;
    /// `short gateOrderList[5][64]`.
    pub const GATE_ORDER_LIST: usize = 0x5278;
    /// `int partyTime[17]`, for characters 1-17.
    pub const PARTY_TIME: usize = 0x73b8;
    /// `ccSpcParam spcParam[18]`, `SPC_PARAM_SIZE` bytes each.
    pub const SPC_PARAM: usize = 0x7488;
    pub const SPC_PARAM_SIZE: usize = 0xdc;
    /// `spcParam[i].base.gold` (int) and `.friendship` (short), from the record's start.
    pub const SPC_GOLD: usize = 0x14;
    pub const SPC_FRIENDSHIP: usize = 0xda;
    /// `spcParam[i].base.level` (char), from the record's start.
    pub const SPC_LEVEL: usize = 0x0e;
    /// The rest of what `ccSaveData::Init` writes ([`super::init`]).
    /// `ccItemList plItemList[99]`.
    pub const PL_ITEM_LIST: usize = 0x0b70;
    /// `GROWTH_PARAM growth[5]`, `GROWTH_SIZE` bytes each: seven shorts
    /// (level, size, smell, crooked, cruel, iq, pure), `short type[3]`,
    /// `int foodNum` at +0x14.
    pub const GROWTH: usize = 0x2194;
    pub const GROWTH_SIZE: usize = 0x18;
    pub const GATE_RECORD: usize = 0x50ac;
    pub const FOUNTAIN_RECORD: usize = 0x65dc;
    pub const FOUNTAIN_NUM: usize = 0x676c;
    pub const TACTICS: usize = 0x6773;
    pub const TIME_IDOL_RANK: usize = 0x6774;
    /// `char timeIdolRankStr[5][2][20]`: each rank's name, then its time.
    pub const TIME_IDOL_RANK_STR: usize = 0x6775;
    pub const HY_PROCCESS: usize = 0x683d;
    pub const HY_ITEM: usize = 0x685d;
    /// `drainCount` then five more shorts to `circleCompleteDungeonCount`.
    pub const DRAIN_COUNT: usize = 0x685e;
    pub const PC_TRADE_COUNT: usize = 0x686a;
    pub const ENEMY_KILL_COUNT: usize = 0x68b7;
    /// `short enemyKillArea[313][4]`.
    pub const ENEMY_KILL_AREA: usize = 0x69f0;
    pub const SPC_PRESENT: usize = 0x73fc;
    /// `itemBoxCount`, `itemObjCount`, `symbolCount` (shorts).
    pub const ITEM_BOX_COUNT: usize = 0x7440;
    pub const PUCCI_FOOD_COUNT: usize = 0x7446;
    /// `itemIdolCount`, then `short fountainCount[2]`.
    pub const ITEM_IDOL_COUNT: usize = 0x746e;
    pub const INU_COUNT: usize = 0x7474;
    pub const DRAIN_DEMO: usize = 0x8427;
    pub const MAP_MODE: usize = 0x842d;
}

/// Array lengths.
pub const MAIL_SLOTS: usize = 512;
pub const WEBNEWS_SLOTS: usize = 128;
pub const BBS_THREADS: usize = 128;
pub const BBS_MESSAGES: usize = 48;
pub const EVENT_FLAGS: usize = 512;
pub const EVENT_ENTRIES: usize = 160;
pub const EVENT_STATUS_SLOTS: usize = 80;
/// `dtWallpaperList[3]` and `dtBgmList[3]` hold unlock bits.
pub const DT_LIST_WORDS: usize = 3;

/// `ccSaveData`, as the game lays it out, and the later volumes' extension
/// after it.
#[derive(Clone, PartialEq, Eq)]
pub struct SaveData(Box<[u8; FULL]>);

impl std::fmt::Debug for SaveData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SaveData({} bytes, name {:?})", SIZE, self.name())
    }
}

impl Default for SaveData {
    fn default() -> Self {
        SaveData::new()
    }
}

impl SaveData {
    /// All zero, as the constructor leaves it.
    pub fn new() -> Self {
        SaveData(Box::new([0; FULL]))
    }

    /// A slot file's (or a dump's) first [`SIZE`] bytes, and the extension
    /// after them when there are [`FULL`] bytes (else zero).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < SIZE {
            return Err(Error::Format(format!("save data is {} bytes, want {SIZE}", bytes.len())));
        }
        let mut s = SaveData::new();
        let n = bytes.len().min(FULL);
        let n = if n == FULL { FULL } else { SIZE };
        s.0[..n].copy_from_slice(&bytes[..n]);
        Ok(s)
    }

    /// `ccSaveData`'s bytes: Infection's slot file.
    pub fn bytes(&self) -> &[u8; SIZE] {
        (&self.0[..SIZE]).try_into().expect("the record holds ccSaveData")
    }

    pub fn bytes_mut(&mut self) -> &mut [u8; SIZE] {
        (&mut self.0[..SIZE]).try_into().expect("the record holds ccSaveData")
    }

    /// The whole record: `ccSaveData` and the extension after it.
    pub fn record(&self) -> &[u8; FULL] {
        &self.0
    }

    pub fn record_mut(&mut self) -> &mut [u8; FULL] {
        &mut self.0
    }

    /// The extension's bytes (Mutation on).
    pub fn ext_bytes(&self) -> &[u8; EXT_SIZE] {
        (&self.0[EXT..]).try_into().expect("the record holds the extension")
    }

    /// What a volume's slot file holds: `ccSaveData`, and from Mutation on
    /// the extension after it.
    pub fn slot_bytes(&self, volume: crate::volume::Volume) -> &[u8] {
        if volume == crate::volume::Volume::Inf { &self.0[..SIZE] } else { &self.0[..] }
    }

    /// The slot index's `sum`: the 16-bit sum of every byte of
    /// `ccSaveData`.
    pub fn sum(&self) -> u16 {
        self.0[..SIZE].iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b)))
    }

    pub fn u8(&self, at: usize) -> u8 {
        self.0[at]
    }

    pub fn set_u8(&mut self, at: usize, v: u8) {
        self.0[at] = v;
    }

    pub fn i16(&self, at: usize) -> i16 {
        i16::from_le_bytes([self.0[at], self.0[at + 1]])
    }

    pub fn set_i16(&mut self, at: usize, v: i16) {
        self.0[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }

    pub fn i32(&self, at: usize) -> i32 {
        i32::from_le_bytes(self.0[at..at + 4].try_into().unwrap())
    }

    pub fn set_i32(&mut self, at: usize, v: i32) {
        self.0[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn u64(&self, at: usize) -> u64 {
        u64::from_le_bytes(self.0[at..at + 8].try_into().unwrap())
    }

    pub fn set_u64(&mut self, at: usize, v: u64) {
        self.0[at..at + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// A NUL-terminated string field of `len` bytes, as raw bytes.
    pub fn cstr(&self, at: usize, len: usize) -> &[u8] {
        let field = &self.0[at..at + len];
        &field[..field.iter().position(|&b| b == 0).unwrap_or(len)]
    }

    /// `plName`.
    pub fn name(&self) -> &[u8] {
        self.cstr(offset::PL_NAME, 24)
    }

    /// `eventFlag[n]`: bits 0-61 the blocks run, 62 done, 63 closed.
    pub fn event_flag(&self, n: usize) -> u64 {
        self.u64(offset::EVENT_FLAG + 8 * n)
    }

    pub fn set_event_flag(&mut self, n: usize, v: u64) {
        self.set_u64(offset::EVENT_FLAG + 8 * n, v);
    }

    /// `mailList[i]`.
    pub fn mail(&self, i: usize) -> u8 {
        self.u8(offset::MAIL_LIST + i)
    }

    pub fn set_mail(&mut self, i: usize, v: u8) {
        self.set_u8(offset::MAIL_LIST + i, v);
    }

    /// `mailOrderList[i]`.
    pub fn mail_order(&self, i: usize) -> i16 {
        self.i16(offset::MAIL_ORDER_LIST + 2 * i)
    }

    pub fn set_mail_order(&mut self, i: usize, v: i16) {
        self.set_i16(offset::MAIL_ORDER_LIST + 2 * i, v);
    }

    /// `webnewsList[i]`.
    pub fn webnews(&self, i: usize) -> u8 {
        self.u8(offset::WEBNEWS_LIST + i)
    }

    pub fn set_webnews(&mut self, i: usize, v: u8) {
        self.set_u8(offset::WEBNEWS_LIST + i, v);
    }

    /// `bbsList[thread][message]`.
    pub fn bbs(&self, thread: usize, message: usize) -> u8 {
        self.u8(offset::BBS_LIST + BBS_MESSAGES * thread + message)
    }

    pub fn set_bbs(&mut self, thread: usize, message: usize, v: u8) {
        self.set_u8(offset::BBS_LIST + BBS_MESSAGES * thread + message, v);
    }

    /// `eventEntry[i][j]`.
    pub fn event_entry(&self, i: usize, j: usize) -> i32 {
        self.i32(offset::EVENT_ENTRY + 4 * (6 * i + j))
    }

    pub fn set_event_entry(&mut self, i: usize, j: usize, v: i32) {
        self.set_i32(offset::EVENT_ENTRY + 4 * (6 * i + j), v);
    }

    /// `eventStatus[i]`.
    pub fn event_status(&self, i: usize) -> u8 {
        self.u8(offset::EVENT_STATUS + i)
    }

    pub fn set_event_status(&mut self, i: usize, v: u8) {
        self.set_u8(offset::EVENT_STATUS + i, v);
    }

    /// `dtWallpaperList[word]`, `dtBgmList[word]`: unlock bits.
    pub fn dt_wallpaper_list(&self, word: usize) -> i32 {
        self.i32(offset::DT_WALLPAPER_LIST + 4 * word)
    }

    pub fn dt_bgm_list(&self, word: usize) -> i32 {
        self.i32(offset::DT_BGM_LIST + 4 * word)
    }

    /// `dtStrList[5]`: a bit per movie (`Stream` entry) unlocked.
    pub fn dt_str_list(&self, word: usize) -> i32 {
        self.i32(offset::DT_STR_LIST + 4 * word)
    }

    /// `playTime`, in 1/60 s.
    pub fn play_time(&self) -> i32 {
        self.i32(offset::PLAY_TIME)
    }

    /// `spcParam[0].base.level`: the player's level, as the slot index
    /// records it.
    pub fn level(&self) -> i8 {
        self.u8(offset::SPC_PARAM + offset::SPC_LEVEL) as i8
    }

    /// `clearFlag`: the volumes cleared.
    pub fn clear_flag(&self) -> i8 {
        self.u8(offset::CLEAR_FLAG) as i8
    }

    /// `plRealName`: the player's own name.
    pub fn real_name(&self) -> &[u8] {
        self.cstr(offset::PL_REAL_NAME, 24)
    }

    /// `assignPADok`, `assignPADcancel`: the pad bits that decide and
    /// cancel (`ccSaveData::Init` sets cross 0x40 and circle 0x20).
    pub fn assign_pad_ok(&self) -> u16 {
        self.i16(offset::ASSIGN_PAD_OK) as u16
    }

    pub fn assign_pad_cancel(&self) -> u16 {
        self.i16(offset::ASSIGN_PAD_CANCEL) as u16
    }

    /// `parodyFlag`: text from the parody tables.
    pub fn parody(&self) -> bool {
        self.u8(offset::PARODY_FLAG) != 0
    }

    fn mail_ordered(&self, n: usize) -> bool {
        (0..MAIL_SLOTS).any(|i| i32::from(self.mail_order(i)) == n as i32)
    }

    fn deliver_mail(&mut self, n: usize, state: u8) {
        if n >= MAIL_SLOTS || self.mail_ordered(n) {
            return;
        }
        self.set_mail(n, state);
        if let Some(i) = (0..MAIL_SLOTS).find(|&i| self.mail_order(i) < 0) {
            self.set_mail_order(i, n as i16);
        }
    }

    /// `ccSaveData::NewMail(n)` (`INF SLUS_202.67:0x00178af0`): unless `n`
    /// is already in `mailOrderList`, `mailList[n] = 1` (delivered, not yet
    /// seen) and `n` goes in the first empty (negative) order slot.
    pub fn new_mail(&mut self, n: usize) {
        self.deliver_mail(n, 1);
    }

    /// `ccSaveData::ReadNewMail(n)` (0x00178b80): the same, delivered
    /// already read (`mailList[n] = 4`).
    pub fn read_new_mail(&mut self, n: usize) {
        self.deliver_mail(n, 4);
    }

    /// `ccSaveData::CheckMail(n)` (0x00178ac0): `mailList[n]` as it was; a
    /// 1 becomes 2 (seen by the desktop, unread).
    pub fn check_mail(&mut self, n: usize) -> u8 {
        let s = self.mail(n);
        if s == 1 {
            self.set_mail(n, 2);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_land_at_their_offsets() {
        let mut s = SaveData::new();
        s.set_event_flag(3, 1 << 62 | 5);
        assert_eq!(s.bytes()[0x54f8 + 24], 5);
        assert_eq!(s.bytes()[0x54f8 + 31], 0x40);
        s.set_mail_order(2, -1);
        assert_eq!(&s.bytes()[0x2464 + 4..0x2464 + 6], &[0xff, 0xff]);
        s.set_bbs(1, 2, 7);
        assert_eq!(s.bytes()[0x28e4 + 48 + 2], 7);
        s.set_event_entry(1, 2, 9);
        assert_eq!(s.i32(0x40e4 + 4 * 8), 9);
        assert_eq!(s.sum(), 5 + 0x40 + 0xff + 0xff + 7 + 9);
        assert_eq!(SaveData::from_bytes(s.bytes()).unwrap(), s);
        assert!(SaveData::from_bytes(&[0; 16]).is_err());
    }

    #[test]
    fn mail_is_delivered_once() {
        let mut s = SaveData::new();
        for i in 0..MAIL_SLOTS {
            s.set_mail_order(i, -1);
        }
        s.new_mail(320);
        s.new_mail(320);
        s.read_new_mail(5);
        assert_eq!((s.mail(320), s.mail(5)), (1, 4));
        assert_eq!((s.mail_order(0), s.mail_order(1), s.mail_order(2)), (320, 5, -1));
        assert_eq!(s.check_mail(320), 1);
        assert_eq!(s.check_mail(320), 2);
    }
}
