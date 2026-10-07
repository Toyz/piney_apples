//! `ccSaveSys` (INF SLUS_202.67, sdmng.cpp) as the desktop's Data screen
//! drives it: the requests, `MainProccess` (the `ccThSaveSys` task's frame)
//! and `NextProccess`, over a [`MemoryCard`] (`docs/formats/save.md`). The
//! screen and the task talk through `result` ([`code`]); `proccess` is what
//! the task does next. The card calls complete at once (see `card.rs`), so
//! the busy states ("Saving....", "Formatting....", "Creating save data.")
//! and `gameCntStop` are set and replaced within one `MainProccess`.

use piney_data::save::{SaveData, offset};
use piney_data::volume::Volume;

use crate::card::{MemoryCard, PortState};

/// `ccSaveDataInfo`: 28 bytes, twelve in the index.
pub const INFO_SIZE: usize = 28;
pub const INFO_COUNT: usize = 12;
pub const INDEX_SIZE: usize = INFO_SIZE * INFO_COUNT;
/// The slot file: `ccSaveData`, 0x8530 bytes; from Mutation on the record
/// and its extension, 0x8d84.
pub fn slot_size(volume: Volume) -> usize {
    if volume == Volume::Inf { piney_data::save::SIZE } else { piney_data::save::FULL }
}
/// `StartReq(3)`: the desktop's `operate` (1 and 2 are the title screen's
/// loads).
pub const OPERATE_DESKTOP: i32 = 3;
/// `StartReq(1)`: the title screen's `operate`.
pub const OPERATE_TITLE: i32 = 1;
/// The title's load: "Select data to load." (`LoadSelectReq`), "Load this
/// data?", "Load complete.", the slot unreadable, its sum wrong.
pub const LOAD_SELECT: u32 = 16;
pub const LOAD_QUESTION: u32 = 0x8011;
/// CONVERT's "Load this data?" (message 18; NextProccess's yes is
/// proccess 9, the previous volume's slot).
pub const LOAD_PREV_QUESTION: u32 = 0x8012;
pub const LOAD_DONE: u32 = 0x2017;
pub const LOAD_READ_ERROR: u32 = 0x2018;
pub const LOAD_BAD_SUM: u32 = 0x203f;
/// The byte runs of `saveData` the load does not copy from the slot file
/// (padding between the members it copies): they keep what was there.
pub const LOAD_KEPT: [(usize, usize); 3] = [(0x221e, 0x2220), (0x7486, 0x7488), (0x852c, 0x8530)];
/// From Mutation on, the extension's padding the load keeps too: the byte
/// after `talkNum[3]`, and the three after the trade counts.
pub const EXT_KEPT: [(usize, usize); 2] = [
    (piney_data::save::EXT + 0x49b, piney_data::save::EXT + 0x49c),
    (piney_data::save::EXT + 0x751, piney_data::save::EXT + 0x754),
];
/// `CheckRightInfo`'s bound on a used slot's play time: 999:59:59 and a
/// frame.
pub const PLAY_TIME_LIMIT: i32 = 0x0cdf_e5c5;

/// `result` bits and plain values.
pub mod code {
    /// The message number: `saveSysMsg[result & 0xfff]`.
    pub const MESSAGE: u32 = 0xfff;
    /// A message the player acknowledges (the button shown).
    pub const ACK: u32 = 0x1000;
    /// An error to acknowledge; `MainProccess` stops polling while it is up.
    pub const ERROR: u32 = 0x2000;
    /// A YES / NO question.
    pub const QUESTION: u32 = 0x8000;
    /// A card operation running.
    pub const BUSY: u32 = 0x10000;
    /// Back to choosing the memory card slot.
    pub const BACK: u32 = 0;
    /// Done: the index was read, or a question was answered NO.
    pub const DONE: u32 = 1;
    /// Read the index again (after formatting or creating the directory).
    pub const RELOAD: u32 = 2;
    /// A request made, the task not yet through it (no message).
    pub const WORKING: u32 = 4;
    /// "Select MEMORY CARD slot. "
    pub const SLOT_SELECT: u32 = 13;
    /// "Select a place to save."
    pub const SAVE_SELECT: u32 = 25;
}

/// `ccSaveDataInfo`: one slot of the index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveDataInfo {
    /// +0x00: 0 empty, 1 used.
    pub status: i8,
    /// +0x01: `spcParam[0].base.level`.
    pub level: i8,
    /// +0x02: `saveData.clearFlag`, the volumes cleared.
    pub clear_flag: i8,
    /// +0x03: `saveData.parodyFlag`.
    pub parody_flag: i8,
    /// +0x04: the player's name, NUL-terminated.
    pub name: [u8; 18],
    /// +0x16: the 16-bit sum of the slot file's bytes.
    pub sum: u16,
    /// +0x18: `saveData.playTime`, in 1/60 s.
    pub playtime: i32,
}

impl SaveDataInfo {
    pub fn from_bytes(b: &[u8]) -> Self {
        let mut name = [0; 18];
        name.copy_from_slice(&b[4..0x16]);
        SaveDataInfo {
            status: b[0] as i8,
            level: b[1] as i8,
            clear_flag: b[2] as i8,
            parody_flag: b[3] as i8,
            name,
            sum: u16::from_le_bytes([b[0x16], b[0x17]]),
            playtime: i32::from_le_bytes([b[0x18], b[0x19], b[0x1a], b[0x1b]]),
        }
    }

    pub fn to_bytes(&self) -> [u8; INFO_SIZE] {
        let mut b = [0; INFO_SIZE];
        b[..4].copy_from_slice(&[self.status as u8, self.level as u8, self.clear_flag as u8, self.parody_flag as u8]);
        b[4..0x16].copy_from_slice(&self.name);
        b[0x16..0x18].copy_from_slice(&self.sum.to_le_bytes());
        b[0x18..].copy_from_slice(&self.playtime.to_le_bytes());
        b
    }

    /// The name up to its NUL.
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name.iter().position(|&c| c == 0).unwrap_or(self.name.len())]
    }
}

/// `ccSaveSys::CheckRightInfo` (0x001716e0): every record either used and
/// in range (level 0-99, clear flag 0-4, parody flag 0-1, play time below
/// [`PLAY_TIME_LIMIT`]) or empty with every field but the name zero.
pub fn check_right_info(index: &[u8; INDEX_SIZE]) -> bool {
    index.as_chunks::<INFO_SIZE>().0.iter().map(|r| SaveDataInfo::from_bytes(r)).all(|r| match r.status {
        1 => {
            (0..100).contains(&r.level)
                && (0..5).contains(&r.clear_flag)
                && (0..2).contains(&r.parody_flag)
                && (0..PLAY_TIME_LIMIT).contains(&r.playtime)
        }
        0 => r.level == 0 && r.clear_flag == 0 && r.parody_flag == 0 && r.sum == 0 && r.playtime == 0,
        _ => false,
    })
}

/// `ccSaveSys` (0x2b8 bytes). The game has one, built at boot, so `port`
/// and `fileNum` carry the last card slot and file used (by the title
/// screen's load too) into the desktop.
#[derive(Clone, Debug)]
pub struct SaveSys {
    /// +0x000 `info[12]`: the index as `ReadSys` and `SaveSys` move it.
    pub info: [u8; INDEX_SIZE],
    /// +0x150 `infoPrev[12]`: the previous volume's index, which CONVERT
    /// (`LoadInfoPrevReq`) reads and lists.
    pub info_prev: [u8; INDEX_SIZE],
    /// +0x2a0: the card port, 0 or 1.
    pub port: i32,
    /// +0x2a4: the slot file, 0-11.
    pub file_num: i32,
    /// +0x2a8: see [`code`].
    pub result: u32,
    /// +0x2ac: the task's next step (1 idle, 2 slot select, 3 read the
    /// index, 10 save select, 11 check the slot, 12 and 13 write, 14
    /// format, 15 make the directory).
    pub proccess: i32,
    /// +0x2b0: `StartReq`'s argument.
    pub operate: i32,
    /// +0x2b4 `tscb`: the `ccThSaveSys` task runs.
    pub running: bool,
    /// The disc's volume (`volumeNum`): which messages name the game, and
    /// which clear data is this volume's or later.
    pub volume: Volume,
}

impl SaveSys {
    /// The constructor (0x00171620) on a volume's disc.
    pub fn new(volume: Volume) -> Self {
        SaveSys {
            info: [0; INDEX_SIZE],
            info_prev: [0; INDEX_SIZE],
            port: 0,
            file_num: 0,
            result: code::WORKING,
            proccess: 1,
            operate: 0,
            running: false,
            volume,
        }
    }

    /// Slot `i` of the index.
    pub fn record(&self, i: usize) -> SaveDataInfo {
        SaveDataInfo::from_bytes(&self.info[INFO_SIZE * i..INFO_SIZE * (i + 1)])
    }

    /// Slot `i` of the previous volume's index.
    pub fn record_prev(&self, i: usize) -> SaveDataInfo {
        SaveDataInfo::from_bytes(&self.info_prev[INFO_SIZE * i..INFO_SIZE * (i + 1)])
    }

    /// `InitInfo` (0x00171670): every record's fields cleared but its name,
    /// in `info` and `infoPrev` alike.
    pub fn init_info(&mut self) {
        for r in
            self.info.as_chunks_mut::<INFO_SIZE>().0.iter_mut().chain(self.info_prev.as_chunks_mut::<INFO_SIZE>().0)
        {
            r[..4].fill(0);
            r[0x16..].fill(0);
        }
    }

    /// `StartReq(op)` (0x00171810): starts the task (`ccThSaveSys`,
    /// priority 20).
    pub fn start_req(&mut self, op: i32) {
        self.running = true;
        self.result = code::WORKING;
        self.proccess = 0;
        self.operate = op;
    }

    /// `EndReq` (0x00171870): deletes the task.
    pub fn end_req(&mut self) {
        self.running = false;
    }

    /// `SlotSelectReq` (0x001718b0).
    pub fn slot_select_req(&mut self) {
        self.proccess = 2;
        self.result = code::SLOT_SELECT;
    }

    /// `LoadInfoReq(pn)` (0x001718d0): read port `pn`'s index.
    pub fn load_info_req(&mut self, pn: i32) {
        self.init_info();
        self.proccess = 3;
        self.result = code::WORKING;
        self.port = pn;
        if self.operate == 2 {
            self.operate = 1;
        }
    }

    /// `LoadInfoPrevReq(pn)` (MUT 0x00171b50): CONVERT reads port `pn`'s
    /// index of the previous volume (proccess 4). `operate` 2 lets the
    /// card lack this disc's own directory.
    pub fn load_info_prev_req(&mut self, pn: i32) {
        self.init_info();
        self.proccess = 4;
        self.result = code::WORKING;
        self.operate = 2;
        self.port = pn;
    }

    /// `LoadDataPrevReq(fn)` (MUT 0x00171c00): CONVERT asks to load the
    /// previous volume's slot `fn` (proccess 7).
    pub fn load_data_prev_req(&mut self, file: i32) {
        self.proccess = 7;
        self.result = code::WORKING;
        self.file_num = file;
        self.operate = 2;
    }

    /// `SaveSelectReq` (0x00171a20).
    pub fn save_select_req(&mut self) {
        self.proccess = 10;
        self.result = code::SAVE_SELECT;
    }

    /// `SaveDataReq(fn)` (0x00171a40): save to slot `fn`.
    pub fn save_data_req(&mut self, file: i32) {
        self.proccess = 11;
        self.result = code::WORKING;
        self.file_num = file;
    }

    /// `GetMessage(pn)` (0x00174300): the `saveSysMsg` entry of a result.
    pub fn message_number(pn: u32) -> usize {
        (pn & code::MESSAGE) as usize
    }

    /// The "no save directory" message by `operate` and the volume read:
    /// 0x103b (asked by the title screen) or 0x1032 (by the desktop) for
    /// Infection.
    fn no_directory(&self, volume: Volume) -> u32 {
        if self.operate == 1 || self.operate == 2 { no_dir_load(volume) } else { no_dir_save(volume) }
    }

    /// `MainProccess` (0x00171c20): one frame of the task.
    pub fn main_proccess(&mut self, card: &mut dyn MemoryCard, save: &SaveData) {
        if self.gate(card) {
            self.dispatch(card, save);
        }
    }

    /// `MainProccess` as the title screen's loads drive it (`StartReq(1)`):
    /// as [`SaveSys::main_proccess`], and the loads' steps. 6 asks "Load
    /// this data?" and 8 reads the slot into `save`; CONVERT's (volumes
    /// 2-4) read the previous volume's index (4), ask (7) and read its
    /// slot (9).
    pub fn main_proccess_load(&mut self, card: &mut dyn MemoryCard, save: &mut SaveData) {
        if !self.gate(card) {
            return;
        }
        match self.proccess {
            4 => {
                if let Some(index) = self.read_index(card, prev_volume(self.volume)) {
                    self.info_prev = index;
                }
            }
            6 => self.ask(LOAD_QUESTION),
            7 => self.ask(LOAD_PREV_QUESTION),
            8 => {
                let sum = self.record(self.file_index()).sum;
                self.read_data(card, save, self.volume, sum);
            }
            9 => {
                let sum = self.record_prev(self.file_index()).sum;
                self.read_data(card, save, prev_volume(self.volume), sum);
            }
            _ => self.dispatch(card, save),
        }
    }

    /// proccess 6 and 7: the load's question up, the task idle.
    fn ask(&mut self, question: u32) {
        self.proccess = 1;
        self.result = question;
    }

    /// MainProccess up to its dispatch on `proccess`: the early outs, the
    /// card check and the messages for a card that cannot be used. True
    /// when the step runs.
    fn gate(&mut self, card: &mut dyn MemoryCard) -> bool {
        if self.proccess == 0 {
            return false;
        }
        if self.proccess == 16 {
            // BootCheckProccess: the boot-time card check, not the desktop's.
            return false;
        }
        if self.proccess == 2 || self.idle_result() {
            return false;
        }
        let st = card.check_port(self.port);
        if st == PortState::NoCard {
            self.result = if self.port == 0 { 0x2005 } else { 0x2006 };
            self.proccess = 1;
            return false;
        }
        if self.proccess == 2 || self.idle_result() || self.proccess == 1 {
            return false;
        }
        let r = self.result;
        let m = r & code::MESSAGE;
        let go = r & (code::ACK | code::ERROR) != 0
            || r & code::QUESTION != 0
            || m == 0
            || matches!(m, 32 | 33 | 36 | 37)
            || st == PortState::Ready;
        if !go {
            let (res, proceed) = match st {
                PortState::NoDirectory => match self.operate {
                    2 => (0, true),
                    1 => (no_dir_load(self.volume), false),
                    _ => (no_dir_save(self.volume), false),
                },
                PortState::Unformatted => (if self.operate == 1 || self.operate == 2 { 0x103a } else { 0x100a }, false),
                PortState::Full => match self.operate {
                    2 => (0, true),
                    1 => (no_dir_load(self.volume), false),
                    _ => (0x2008, false),
                },
                PortState::NotPs2 => (0x2007, false),
                PortState::Ready | PortState::NoCard => (0, true),
            };
            if !proceed {
                self.result = res;
                self.proccess = 1;
                return false;
            }
        }
        true
    }

    /// MainProccess's steps (jump table 0x0034cf50, by `proccess - 3`).
    fn dispatch(&mut self, card: &mut dyn MemoryCard, save: &SaveData) {
        match self.proccess {
            3 => {
                if let Some(index) = self.read_index(card, self.volume) {
                    self.info = index;
                }
            }
            11 => self.check_slot(save),
            12 | 13 => self.write(card, save),
            14 => {
                self.proccess = 1;
                self.result = if card.format(self.port) { 0x1022 } else { 0x2023 };
            }
            15 => {
                self.proccess = 1;
                if !card.make_dir(self.port) {
                    self.result = 0x2027;
                } else {
                    self.init_info();
                    // The code tests SaveSys for 5, which it never returns
                    // (it fails with 6): a failed index write still reports
                    // "Save data created."
                    let _ = card.write_index(self.port, &self.info);
                    self.result = 0x1026;
                }
            }
            // 4 and 6-9 are the title screen's loads (main_proccess_load);
            // 5 and 10 wait for the screen.
            _ => {}
        }
    }

    /// proccess 8 (0x0017240c) and 9 (MUT 0x00173580): `DataRead` of the
    /// slot in `volume`'s directory, at that volume's size; a failed read
    /// is 0x2018, a byte sum that is not `want` (its index record's) 0x203f;
    /// else the buffer is copied into `saveData` member by member, all but
    /// three padding runs ([`LOAD_KEPT`]; an 0x8d84 slot's extension too,
    /// [`EXT_KEPT`]), and the result is "Load complete." ([`load_done`]). Infection's
    /// slot (Mutation's CONVERT) leaves the extension as it was. Not the
    /// game's: a save the port wrote before it ran `ccSaveData::Init` whole
    /// is repaired as it is copied in ([`SaveData::repair_port_save`]).
    fn read_data(&mut self, card: &mut dyn MemoryCard, save: &mut SaveData, volume: Volume, want: u16) {
        self.proccess = 1;
        let file = self.file_index();
        let size = slot_size(volume);
        let data = match card.read_slot(self.port, file, volume.number(), size) {
            Some(d) if d.len() >= size => d,
            _ => {
                self.result = LOAD_READ_ERROR;
                return;
            }
        };
        // Not the game's: an Infection slot written at the later volumes'
        // 0x8d84 (as on a PCSX2 card), its record's sum over the whole file,
        // is taken too; its first 0x8530 bytes are the same ccSaveData.
        let sum_of = |n: usize| data[..n].iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b)));
        let full = piney_data::save::FULL;
        if sum_of(size) != want && !(size < full && data.len() >= full && sum_of(full) == want) {
            self.result = LOAD_BAD_SUM;
            return;
        }
        let dst = save.record_mut();
        let kept = LOAD_KEPT.iter().chain(if size > piney_data::save::SIZE { &EXT_KEPT[..] } else { &[] });
        let mut at = 0;
        for &(from, to) in kept {
            dst[at..from].copy_from_slice(&data[at..from]);
            at = to;
        }
        dst[at..size].copy_from_slice(&data[at..size]);
        save.repair_port_save();
        self.result = load_done(self.volume);
    }

    /// `LoadSelectReq` (0x00171990): the save list (result 16 "Select data
    /// to load.", proccess 5).
    pub fn load_select_req(&mut self) {
        self.proccess = 5;
        self.result = LOAD_SELECT;
    }

    /// `LoadDataReq(fn)` (0x001719b0): load slot `fn` (proccess 6).
    pub fn load_data_req(&mut self, file: i32) {
        self.proccess = 6;
        self.result = code::WORKING;
        self.file_num = file;
        if self.operate == 2 {
            self.operate = 1;
        }
    }

    /// MainProccess's early outs: an error up, or a result of 0 or 1.
    fn idle_result(&self) -> bool {
        self.result & code::ERROR != 0 || matches!(self.result & code::MESSAGE, 0 | 1)
    }

    /// proccess 3 (0x00171fc4) and 4 (MUT 0x00172514): `ReadSys` of
    /// `volume`'s index into a buffer of 0xff, then `CheckRightInfo`. A good
    /// index is returned for `info` (or `infoPrev`); a bad one leaves the
    /// table as it was. Either way the result is 1; an unreadable index is
    /// "There is no ... saved data" naming `volume`.
    fn read_index(&mut self, card: &mut dyn MemoryCard, volume: Volume) -> Option<[u8; INDEX_SIZE]> {
        self.proccess = 1;
        let Some(read) = card.read_index(self.port, volume.number()) else {
            self.result = self.no_directory(volume);
            return None;
        };
        let mut tmp = [0xffu8; INDEX_SIZE];
        let n = read.len().min(INDEX_SIZE);
        tmp[..n].copy_from_slice(&read[..n]);
        self.result = code::DONE;
        check_right_info(&tmp).then_some(tmp)
    }

    /// proccess 11 (0x00173c08): an empty slot asks "Create new data?"; a
    /// used one "Overwrite data?" unless it holds clear data this save does
    /// not (its clear flag at least `volumeNum` and not this save's), which
    /// warns first.
    fn check_slot(&mut self, save: &SaveData) {
        self.proccess = 1;
        let rec = self.record(self.file_index());
        self.result = if rec.status == 0 {
            0x801e
        } else if i32::from(rec.clear_flag) < self.volume.number() || rec.clear_flag == save.clear_flag() {
            0x801f
        } else {
            clear_data_warning(self.volume)
        };
    }

    /// proccess 12 and 13 (0x00173cf8): the slot's record from `saveData`,
    /// then `SaveSys` (the index) and `DataWrite` (the slot file). From
    /// Mutation on the slot and its sum take in the extension.
    fn write(&mut self, card: &mut dyn MemoryCard, save: &SaveData) {
        self.proccess = 1;
        self.result = if self.port == 0 { 0x1001a } else { 0x1001b };
        let slot = save.slot_bytes(self.volume);
        let sum = slot.iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b)));
        let at = INFO_SIZE * self.file_index();
        self.info[at] = 1;
        self.info[at + 1] = save.level() as u8;
        // strcpy(name, spcParam[0].base.name): LoadGame points it at plName.
        let b = save.bytes();
        let name = &b[offset::PL_NAME..b[offset::PL_NAME..].iter().position(|&c| c == 0).unwrap_or(b.len())];
        for (i, &c) in name.iter().chain(std::iter::once(&0)).enumerate() {
            if let Some(d) = self.info.get_mut(at + 4 + i) {
                *d = c;
            }
        }
        self.info[at + 0x18..at + 0x1c].copy_from_slice(&save.play_time().to_le_bytes());
        self.info[at + 2] = save.clear_flag() as u8;
        self.info[at + 3] = save.u8(offset::PARODY_FLAG);
        self.info[at + 0x16..at + 0x18].copy_from_slice(&sum.to_le_bytes());
        self.result =
            if !card.write_index(self.port, &self.info) || !card.write_slot(self.port, self.file_index(), slot) {
                0x201d
            } else {
                0x101c
            };
    }

    fn file_index(&self) -> usize {
        self.file_num.clamp(0, INFO_COUNT as i32 - 1) as usize
    }

    /// `NextProccess(sel)` (0x00173f90, jump table 0x0034cf90): the screen's
    /// answer to the message up (`sel` 1 YES, 0 NO or acknowledged).
    pub fn next_proccess(&mut self, sel: i32) {
        let port = self.port;
        let (result, proccess) = match self.result & code::MESSAGE {
            5..=7 | 9 | 24 | 29 | 35 | 39 | 58..=63 => (0, 1),
            8 => (0x2009, 1),
            10 => (0x800b, 1),
            11 => {
                if sel == 0 {
                    (0, 1)
                } else {
                    (if port == 0 { 0x10020 } else { 0x10021 }, 14)
                }
            }
            17 | 18 | 30 | 31 | 43 | 45 | 47 | 49 => {
                if sel == 0 {
                    (1, 1)
                } else {
                    let p = match self.result & code::MESSAGE {
                        17 => 8,
                        18 => 9,
                        30 => 12,
                        _ => 13,
                    };
                    (code::WORKING, p)
                }
            }
            34 | 38 => (code::RELOAD, 1),
            40 | 41 => {
                if sel == 0 {
                    return;
                }
                (1, 1)
            }
            42 => (0x802b, 1),
            44 => (0x802d, 1),
            46 => (0x802f, 1),
            48 => (0x8031, 1),
            50 => (0x8033, 1),
            52 => (0x8035, 1),
            54 => (0x8037, 1),
            56 => (0x8039, 1),
            51 | 53 | 55 | 57 => {
                if sel == 0 {
                    (0, 1)
                } else {
                    (if port == 0 { 0x10024 } else { 0x10025 }, 15)
                }
            }
            _ => {
                // Every other message (28 "Data saved." among them): done,
                // the task's step left as it was.
                self.result = code::DONE;
                return;
            }
        };
        self.result = result;
        self.proccess = proccess;
    }
}

/// "Load complete." as the loads leave it: [`LOAD_DONE`], an error to
/// acknowledge, but Quarantine's (QUA 0x0017297c, 0x001737cc) a plain
/// message to acknowledge, 0x1017.
pub fn load_done(volume: Volume) -> u32 {
    match volume {
        Volume::Qua => code::ACK | 23,
        _ => LOAD_DONE,
    }
}

/// The volume CONVERT reads: `volumeNum - 1`, Infection when that is not
/// 1-3 (`LoadInfoPrevReq` and the slot read clamp it alike).
pub fn prev_volume(volume: Volume) -> Volume {
    Volume::from_number(volume.number() - 1).filter(|&v| v != Volume::Qua).unwrap_or(Volume::Inf)
}

/// "There is no saved data for .hack//INFECTION ..." (the save side) by
/// volume: 0x1032, 0x1034, 0x1036, 0x1038.
fn no_dir_save(volume: Volume) -> u32 {
    match volume {
        Volume::Inf => 0x1032,
        Volume::Mut => 0x1034,
        Volume::Out => 0x1036,
        Volume::Qua => 0x1038,
    }
}

/// The load side's: "There is no .hack//INFECTION saved data." and on.
fn no_dir_load(volume: Volume) -> u32 {
    match volume {
        Volume::Inf => 0x103b,
        Volume::Mut => 0x103c,
        Volume::Out => 0x103d,
        Volume::Qua => 0x103e,
    }
}

/// "Data about to be overwritten is .hack//INFECTION Clear Data." and on.
fn clear_data_warning(volume: Volume) -> u32 {
    match volume {
        Volume::Inf => 0x102a,
        Volume::Mut => 0x102c,
        Volume::Out => 0x102e,
        Volume::Qua => 0x1030,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::FilesCard;

    /// An Infection slot loads at either size: the game's 0x8530, its
    /// record summed over that, and the later volumes' 0x8d84 (a PCSX2
    /// card's), summed over the whole file; a sum that matches neither is
    /// refused. With `work/savecards` (a card exported from PCSX2) its
    /// first slot loads too.
    #[test]
    fn an_infection_slot_loads_at_either_size() {
        use piney_data::save::{FULL, SIZE};
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("piney-inf-slot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut card = FilesCard::slot1(Volume::Inf, &root);
        assert!(card.make_dir(0));
        let sum = |b: &[u8]| b.iter().fold(0u16, |s, &x| s.wrapping_add(u16::from(x)));
        for (len, summed, want) in [(SIZE, SIZE, LOAD_DONE), (FULL, FULL, LOAD_DONE), (FULL, SIZE + 1, LOAD_BAD_SUM)] {
            let mut slot: Vec<u8> = (0..len).map(|i| (i * 31 % 251) as u8).collect();
            slot[4..8].copy_from_slice(b"Kite");
            std::fs::write(card.slot_path(0, 0, 1).unwrap(), &slot).unwrap();
            let mut sys = SaveSys::new(Volume::Inf);
            let mut rec = SaveDataInfo::from_bytes(&[0; INFO_SIZE]);
            rec.status = 1;
            rec.sum = sum(&slot[..summed]);
            sys.info[..INFO_SIZE].copy_from_slice(&rec.to_bytes());
            let mut save = SaveData::new();
            let sum = sys.record(0).sum;
            sys.read_data(&mut card, &mut save, Volume::Inf, sum);
            assert_eq!(sys.result, want, "{len:#x} summed over {summed:#x}");
            if want == LOAD_DONE {
                assert_eq!(&save.bytes()[4..8], b"Kite");
            }
        }
        let _ = std::fs::remove_dir_all(&root);
        let real = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/savecards");
        if real.join("BASLUS-20267DOTHACK").is_dir() {
            let mut card = FilesCard::slot1(Volume::Inf, &real);
            let mut sys = SaveSys::new(Volume::Inf);
            sys.info.copy_from_slice(&card.read_index(0, 1).unwrap()[..INDEX_SIZE]);
            let mut save = SaveData::new();
            let sum = sys.record(0).sum;
            sys.read_data(&mut card, &mut save, Volume::Inf, sum);
            assert_eq!(sys.result, LOAD_DONE, "PCSX2's slot 1");
        }
    }

    /// A card under `target/test-tmp` of its own.
    fn test_card(name: &str) -> std::path::PathBuf {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("piney-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// `save` written to slot `file` of `volume`'s directory as that
    /// volume's save writes it (proccess 12: the record, then the slot).
    fn save_to(root: &std::path::Path, volume: Volume, file: i32, save: &SaveData) {
        let mut card = FilesCard::slot1(volume, root);
        if card.check_port(0) != PortState::Ready {
            assert!(card.make_dir(0));
        }
        let mut sys = SaveSys::new(volume);
        sys.info.copy_from_slice(&card.read_index(0, volume.number()).unwrap());
        (sys.file_num, sys.proccess, sys.result) = (file, 12, code::WORKING);
        sys.main_proccess(&mut card, save);
        assert_eq!(sys.result, 0x101c, "{volume} saved");
    }

    /// The title's task one frame at a time until it leaves `proccess`.
    fn run(sys: &mut SaveSys, card: &mut FilesCard, save: &mut SaveData) {
        sys.main_proccess_load(card, save);
        assert_eq!(sys.proccess, 1);
    }

    /// Issue #55. Mutation's CONVERT on a card with Infection's saves and
    /// no Mutation directory: `LoadInfoPrevReq` reads Infection's index
    /// (`operate` 2 lets the own directory be missing, where the title's
    /// LOAD says "There is no .hack//MUTATION saved data."), then the
    /// cleared slot is asked about and read: Infection's 0x8530 bytes, the
    /// extension as it was. Without Infection's directory the message
    /// names Infection.
    #[test]
    fn mutation_s_convert_reads_infection_s_saves() {
        use piney_data::save::{EXT, FULL, SIZE};
        let root = test_card("convert");
        let mut card = FilesCard::slot1(Volume::Mut, &root);
        let mut sys = SaveSys::new(Volume::Mut);
        let mut save = SaveData::new();
        sys.start_req(OPERATE_TITLE);
        sys.load_info_prev_req(0);
        run(&mut sys, &mut card, &mut save);
        assert_eq!(sys.result, 0x103b, "There is no .hack//INFECTION saved data.");

        let mut inf = SaveData::new();
        inf.bytes_mut()[..5].copy_from_slice(b"Kite\0");
        inf.set_u8(offset::CLEAR_FLAG, 1);
        save_to(&root, Volume::Inf, 2, &inf);
        sys.load_info_prev_req(0);
        run(&mut sys, &mut card, &mut save);
        assert_eq!(sys.result, code::DONE);
        let r = sys.record_prev(2);
        assert_eq!((r.status, r.clear_flag, r.name()), (1, 1, &b"Kite"[..]));
        assert!(sys.info.iter().all(|&b| b == 0), "this volume's index untouched");

        save.record_mut()[EXT..FULL].fill(0x5a);
        sys.load_data_prev_req(2);
        run(&mut sys, &mut card, &mut save);
        assert_eq!(sys.result, LOAD_PREV_QUESTION);
        sys.next_proccess(1);
        assert_eq!((sys.result, sys.proccess), (code::WORKING, 9));
        run(&mut sys, &mut card, &mut save);
        assert_eq!(sys.result, LOAD_DONE);
        // The port's repair of a save with empty lists, as any load.
        let mut want = inf.clone();
        want.repair_port_save();
        assert_eq!(&save.bytes()[..SIZE - 4], &want.bytes()[..SIZE - 4]);
        assert!(save.record()[EXT..FULL].iter().all(|&b| b == 0x5a), "the extension kept");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Outbreak's and Quarantine's CONVERT read the volume before's whole
    /// 0x8d84-byte slot, its extension too; Quarantine's reads Outbreak's.
    #[test]
    fn later_converts_read_the_whole_slot() {
        use piney_data::save::{EXT, FULL};
        for (volume, prev) in [(Volume::Out, Volume::Mut), (Volume::Qua, Volume::Out)] {
            assert_eq!(prev_volume(volume), prev);
            let root = test_card(&format!("convert-{volume}"));
            let mut before = SaveData::new();
            before.bytes_mut()[..4].copy_from_slice(b"Kite");
            before.set_u8(offset::CLEAR_FLAG, prev.number() as u8);
            before.record_mut()[EXT + 0x10..EXT + 0x20].fill(7);
            save_to(&root, prev, 0, &before);
            let mut card = FilesCard::slot1(volume, &root);
            let mut sys = SaveSys::new(volume);
            let mut save = SaveData::new();
            sys.start_req(OPERATE_TITLE);
            sys.load_info_prev_req(0);
            run(&mut sys, &mut card, &mut save);
            assert_eq!(sys.record_prev(0).clear_flag, prev.number() as i8);
            sys.load_data_prev_req(0);
            run(&mut sys, &mut card, &mut save);
            sys.next_proccess(1);
            run(&mut sys, &mut card, &mut save);
            assert_eq!(sys.result, load_done(volume), "{volume}");
            assert_eq!(&save.record()[EXT + 0x10..EXT + 0x20], &[7; 16]);
            assert_eq!(save.record()[FULL - 1], before.record()[FULL - 1]);
            let _ = std::fs::remove_dir_all(&root);
        }
        assert_eq!(prev_volume(Volume::Inf), Volume::Inf, "volumeNum - 1 of 0 reads 1");
        assert_eq!(prev_volume(Volume::Mut), Volume::Inf);
    }

    #[test]
    fn info_round_trips() {
        let mut b = [0u8; INFO_SIZE];
        b[0] = 1;
        b[1] = 7;
        b[4..8].copy_from_slice(b"Kite");
        b[0x16] = 0x34;
        b[0x17] = 0x12;
        b[0x18] = 5;
        let r = SaveDataInfo::from_bytes(&b);
        assert_eq!((r.status, r.level, r.name(), r.sum, r.playtime), (1, 7, &b"Kite"[..], 0x1234, 5));
        assert_eq!(r.to_bytes(), b);
    }

    #[test]
    fn right_info() {
        let mut idx = [0u8; INDEX_SIZE];
        idx[4] = b'x';
        assert!(check_right_info(&idx), "an empty record's name is not checked");
        idx[0] = 1;
        idx[1] = 99;
        idx[0x18..0x1c].copy_from_slice(&(PLAY_TIME_LIMIT - 1).to_le_bytes());
        assert!(check_right_info(&idx));
        idx[0x18..0x1c].copy_from_slice(&PLAY_TIME_LIMIT.to_le_bytes());
        assert!(!check_right_info(&idx));
        assert!(!check_right_info(&[0xff; INDEX_SIZE]));
    }

    #[test]
    fn next_proccess_answers() {
        let mut s = SaveSys::new(Volume::Inf);
        for (res, sel, want) in [
            (0x2005, 0, (0, 1)),
            (0x2008, 0, (0x2009, 1)),
            (0x100a, 0, (0x800b, 1)),
            (0x800b, 1, (0x10020, 14)),
            (0x1032, 0, (0x8033, 1)),
            (0x8033, 1, (0x10024, 15)),
            (0x801e, 1, (4, 12)),
            (0x801f, 1, (4, 13)),
            (0x801f, 0, (1, 1)),
            (0x102a, 0, (0x802b, 1)),
            (0x802b, 1, (4, 13)),
            (0x1022, 0, (2, 1)),
            (0x1026, 0, (2, 1)),
        ] {
            s.result = res;
            s.proccess = 1;
            s.next_proccess(sel);
            assert_eq!((s.result, s.proccess), want, "result {res:#x} answered {sel}");
        }
        s.result = 0x101c;
        s.proccess = 7;
        s.next_proccess(0);
        assert_eq!((s.result, s.proccess), (1, 7), "anything else: done, the step kept");
    }
}
