//! The seam to the event engine: the game's own `ccSaveData` record
//! ([`piney_data::save::SaveData`], shared with the event scripts) and the two
//! members of `ccEvent` the desktop touches (`operate`, `operateSet`). The
//! scripts deliver mail and unlock wallpapers and music; the desktop marks
//! mail seen, read and replied. It reads and writes `plName`, `plRealName`,
//! `dtWallpaper`, `dtBgm`, the unlock bits, `mailList` ([`MailState`]),
//! `mailOrderList`, `webnewsList`, `assignPADok` / `assignPADcancel` and
//! `parodyFlag` (their offsets are in docs/formats/save.md).

pub use piney_data::save::{InitText, MAIL_SLOTS, MAX_WAVE_NUM, ORIGINAL_WALL_1, SaveData, offset};
/// `assignPADaction` (+0x8404): the first of the eleven button assignments.
pub const ASSIGN_PAD: usize = 0x8404;

/// `mailList[n]`: one mail's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MailState {
    /// 0: not delivered, or removed (event opcode 110).
    #[default]
    None,
    /// 1: delivered, not yet seen by the desktop (`ccSaveData::NewMail`).
    New,
    /// 2: seen in the list, unread (`ccSaveData::CheckMail` turns 1 to 2).
    Unread,
    /// 4: read (closing a mail; or `ccSaveData::ReadNewMail`).
    Read,
    /// 5: replied with its first reply (`oneRes`).
    RepliedOne,
    /// 6: replied with its second reply (`twoRes`).
    RepliedTwo,
    /// Any other byte (nothing writes 3).
    Other(u8),
}

impl MailState {
    pub fn from_byte(b: u8) -> MailState {
        match b {
            0 => MailState::None,
            1 => MailState::New,
            2 => MailState::Unread,
            4 => MailState::Read,
            5 => MailState::RepliedOne,
            6 => MailState::RepliedTwo,
            x => MailState::Other(x),
        }
    }

    pub fn byte(self) -> u8 {
        match self {
            MailState::None => 0,
            MailState::New => 1,
            MailState::Unread => 2,
            MailState::Read => 4,
            MailState::RepliedOne => 5,
            MailState::RepliedTwo => 6,
            MailState::Other(x) => x,
        }
    }
}

/// What the desktop reads and writes of the game's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveState {
    /// `ccSaveData`, shared with the event engine.
    pub save: SaveData,
    /// `ccEvent.operate` (+0x770): a bit per operation the events have
    /// locked; the desktop asks about icon `n` and `n + 19`.
    pub operate: u64,
    /// `ccEvent.operateSet` (+0x778): the first operation the player tried
    /// since the events last cleared it, -1 for none; events wait on it.
    pub operate_set: i16,
}

impl SaveState {
    pub fn new(save: SaveData) -> Self {
        SaveState { save, operate: 0, operate_set: -1 }
    }

    /// The save the port starts from when it does not boot through the
    /// title: [`SaveData::boot`] (`ccThMother`'s `ccSaveData`: zeroed,
    /// `Init(0)`, `Init(1)`), with the player named Kite, as `NewGame` names
    /// him from `charTbl`. Init's one piece of text, the time idols'
    /// default ranks, needs the executable; this leaves those strings
    /// empty. [`SaveState::fresh_with`] has them.
    pub fn fresh() -> Self {
        Self::fresh_with(&InitText::default())
    }

    /// [`SaveState::fresh`] with Init's text ([`InitText::from_disc`]).
    pub fn fresh_with(text: &InitText) -> Self {
        let mut s = SaveData::boot(text);
        s.bytes_mut()[offset::PL_NAME..offset::PL_NAME + 4].copy_from_slice(b"Kite");
        SaveState::new(s)
    }

    pub fn dt_wallpaper(&self) -> usize {
        (self.save.u8(offset::DT_WALLPAPER) as i8).max(0) as usize
    }

    pub fn dt_bgm(&self) -> usize {
        (self.save.u8(offset::DT_BGM) as i8).max(0) as usize
    }

    /// `dtWallpaper = (char)No` (`Acces_control::ChangeWall`).
    pub fn set_dt_wallpaper(&mut self, no: i8) {
        self.save.set_u8(offset::DT_WALLPAPER, no as u8);
    }

    /// `dtBgm = n` (`Audio_control::ChangeWeve`).
    pub fn set_dt_bgm(&mut self, n: i8) {
        self.save.set_u8(offset::DT_BGM, n as u8);
    }

    /// Bit `k` of `dtWallpaperList`: `WallTbl[k]` unlocked.
    pub fn wallpaper_unlocked(&self, k: usize) -> bool {
        k < 96 && self.save.dt_wallpaper_list(k >> 5) as u32 & (1 << (k & 31)) != 0
    }

    /// Bit `i` of `dtBgmList`: `Wave[i]` unlocked.
    pub fn bgm_unlocked(&self, i: usize) -> bool {
        i < 96 && self.save.dt_bgm_list(i >> 5) as u32 & (1 << (i & 31)) != 0
    }

    /// Bit `i` of `dtStrList`: `Stream[i]` (a movie) unlocked.
    pub fn movie_unlocked(&self, i: usize) -> bool {
        i < 160 && self.save.dt_str_list(i >> 5) as u32 & (1 << (i & 31)) != 0
    }

    pub fn ok(&self) -> u32 {
        u32::from(self.save.assign_pad_ok())
    }

    pub fn cancel(&self) -> u32 {
        u32::from(self.save.assign_pad_cancel())
    }

    pub fn parody(&self) -> bool {
        self.save.parody()
    }

    /// `#0` and `#1`.
    pub fn names(&self) -> crate::kanji::Names {
        crate::kanji::Names { name: self.save.name().to_vec(), real: self.save.real_name().to_vec() }
    }

    pub fn mail(&self, n: usize) -> MailState {
        if n < MAIL_SLOTS { MailState::from_byte(self.save.mail(n)) } else { MailState::None }
    }

    pub fn set_mail(&mut self, n: usize, s: MailState) {
        if n < MAIL_SLOTS {
            self.save.set_mail(n, s.byte());
        }
    }

    /// `ccEvent::CheckOperate(n, 0)` (0x001b32f0): whether operation `n` is
    /// allowed; records `n` in `operateSet` when that is negative.
    pub fn check_operate(&mut self, n: i32) -> bool {
        if n < 0 {
            return true;
        }
        if self.operate_set < 0 {
            self.operate_set = n as i16;
        }
        !(self.operate != 0 && n < 64 && self.operate & (1u64 << n) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_game() {
        let s = SaveState::fresh();
        assert_eq!((s.dt_wallpaper(), s.dt_bgm()), (49, 50));
        assert_eq!((s.ok(), s.cancel()), (0x40, 0x20));
        assert_eq!(s.save.mail_order(0), -1);
        assert_eq!(s.mail(4), MailState::None);
    }

    #[test]
    fn operate_records_first_try() {
        let mut s = SaveState::fresh();
        s.operate = 1 << 1;
        assert!(!s.check_operate(1));
        assert!(s.check_operate(2));
        assert_eq!(s.operate_set, 1);
        assert!(s.check_operate(-1));
    }
}
