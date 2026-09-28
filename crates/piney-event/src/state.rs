//! The saved state the scripts read and write.
//!
//! It is the game's own save record, [`SaveData`] (`ccSaveData`, 0x8530
//! bytes, `docs/formats/save.md`), shared with the desktop and the rest of
//! the port as in the game. The caller owns it; the interpreter reaches it
//! through [`crate::host::Host::save`]. Whatever is not in the save (phases,
//! answers, intercepted operations) is the event manager's, in
//! [`crate::vm::EventMng`].
//!
//! [`ScriptSave`] adds to [`SaveData`] the members and the `ccSaveData`
//! functions the instructions use. The members, at their Infection offsets
//! (the same on every volume):
//!
//! ```text
//! +0x0030  itemList[18][40]        short id, char category, char count; -1 -1 0 when empty
//! +0x0cfc  impItemList[320]        counts of Kite's important items (category 15)
//! +0x1ec4  skillList[18][20]       sorted skill ids, -1 after the last
//! +0x220c  talkNum[18]
//! +0x2220  partyMemberFlag         bit per character who is a member
//! +0x2224  partyMemberCall         bit per character who can be called; bit 31 while locked
//! +0x2228  partyMemberCallStore    the call bits kept while locked
//! +0x222c  partyMemberExp
//! +0x2230  partyMemberSave         the party save_party recorded
//! +0x2234  townMoveFlag            bit per town (short)
//! +0x2238  dtWallpaperList[3]      bit per desktop wallpaper
//! +0x2244  dtBgmList[3]            bit per desktop music track
//! +0x2250  dtStrList[5]            bit per desktop movie
//! +0x2264  mailList[512]           0 none, 1 arrived, 2 seen, 4 read, 5 6 (the mailer's)
//! +0x2464  mailOrderList[512]      arrival order, -1 after the last
//! +0x2864  webnewsList[128]        0 none, 1 posted, 3 read
//! +0x28e4  bbsList[128][48]        0 none, 1 posted, 3 read, 7 posted (bbs_post7)
//! +0x4fe4  gateList[5][5]          per server, bit per story area on the Chaos Gate
//! +0x5048  gateListMark[5][5]      per server, bit per marked story area
//! +0x523c  wordList[15]            bit per keyword known
//! +0x5278  gateOrderList[5][64]    per server, story areas newest first, -1 empty
//! +0x54f8  eventFlag[512]          per event: bit b < 62 block b ran, 62 done, 63 closed
//! +0x64f8  eventStatus[80]         the scripts' counters
//! +0x6548  areaBan[32][4]          (field, dungeon, floor, block) barred, -1 free
//! +0x65c8  protectArea[5]          bit per story area whose virus core is gone
//! +0x6771  plcol                   1 once Kite has Data Drain
//! +0x6772  crisis
//! +0x73b8  partyTime[17]           frames characters 1-17 have been in the party
//! +0x7488  spcParam[18]            0xdc each: base.gold +0x14 (int), friendship +0xda (short)
//! +0x8426  lastTown
//! +0x842a  clearFlag               volumes cleared
//! +0x842b  parodyFlag
//! ```
//!
//! Arrays keep the game's flat indexing: `bbsList[thread][post]` is byte
//! `thread * 48 + post`, so a post past 47 lands in the next thread as it
//! does in the game. A write that would fall outside its array is dropped
//! (the game would write some other member).

pub use piney_data::save::SaveData;
use piney_data::save::offset as off;

pub const DONE: u64 = 1 << 62;
pub const CLOSED: u64 = 1 << 63;

/// `ccItemList`: one slot of a character's item list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    pub id: i16,
    pub category: i8,
    pub count: i8,
}

impl Item {
    /// What `ccSaveData::Init` and the item functions leave in a free slot.
    pub const EMPTY: Item = Item { id: -1, category: -1, count: 0 };
}

/// `addItemCategoryTbl` (`INF SLUS_202.67:0x00307140`): the order
/// `AddItem` sorts a list into, category by category, and for each the
/// first id that does not fit (an item with a larger id is dropped).
pub const ADD_ITEM_ORDER: [(i16, i16); 15] = [
    (10, 24),
    (13, 3),
    (11, 72),
    (12, 34),
    (14, 22),
    (0, 82),
    (1, 77),
    (2, 97),
    (3, 75),
    (4, 74),
    (5, 76),
    (6, 69),
    (7, 68),
    (8, 67),
    (9, 68),
];

/// `AddFriendship`'s cap, indexed by `volumeNum` (`INF 0x00307180`).
pub const FRIENDSHIP_CAP: [i32; 5] = [1000, 250, 500, 750, 1000];

pub const CHARACTERS: usize = 18;
const ITEMS: usize = 40;
const SKILLS: usize = 20;

/// Bit `n` of an int array, with C's `n / 32` and `n % 32` as the game
/// computes them: None for a negative word index.
fn bit_word(n: i32) -> Option<(usize, u32)> {
    let word = n.wrapping_div(32);
    let bit = n.wrapping_rem(32);
    usize::try_from(word).ok().map(|w| (w, 1u32 << (bit & 31)))
}

/// A 32-bit `sllv` by `n`: `1 << (n & 31)`.
pub fn bit32(n: i32) -> u32 {
    1u32 << (n & 31)
}

/// The script-facing members and functions of `ccSaveData`.
pub trait ScriptSave {
    fn flags(&self, n: i32) -> u64;
    fn set_flags(&mut self, n: i32, v: u64);
    fn update_flags(&mut self, n: i32, f: impl FnOnce(u64) -> u64) {
        let v = self.flags(n);
        self.set_flags(n, f(v));
    }
    fn event_done(&self, n: i32) -> bool {
        self.flags(n) & DONE != 0
    }

    /// A byte of a flat byte array (`None` outside it).
    fn byte_at(&self, base: usize, len: usize, i: i32) -> Option<i8>;
    fn set_byte_at(&mut self, base: usize, len: usize, i: i32, v: i8);
    fn word_at(&self, base: usize, len: usize, i: i32) -> Option<u32>;
    fn set_word_at(&mut self, base: usize, len: usize, i: i32, v: u32);

    fn status(&self, i: i32) -> i8 {
        self.byte_at(off::EVENT_STATUS, 80, i).unwrap_or(0)
    }
    fn mail_state(&self, m: i32) -> i8 {
        self.byte_at(off::MAIL_LIST, 512, m).unwrap_or(0)
    }
    fn news_state(&self, n: i32) -> i8 {
        self.byte_at(off::WEBNEWS_LIST, 128, n).unwrap_or(0)
    }
    fn bbs_state(&self, thread: i32, post: i32) -> i8 {
        self.byte_at(off::BBS_LIST, 128 * 48, thread.wrapping_mul(48).wrapping_add(post)).unwrap_or(0)
    }
    fn set_bbs_state(&mut self, thread: i32, post: i32, v: i8) {
        self.set_byte_at(off::BBS_LIST, 128 * 48, thread.wrapping_mul(48).wrapping_add(post), v);
    }
    fn member_word(&self, at: usize) -> u32;
    fn set_member_word(&mut self, at: usize, v: u32);

    fn item(&self, pc: usize, k: usize) -> Item;
    fn set_item(&mut self, pc: usize, k: usize, it: Item);
    fn skill(&self, pc: usize, k: usize) -> i16;
    fn set_skill(&mut self, pc: usize, k: usize, v: i16);
    fn friendship(&self, pc: i32) -> i16;
    fn set_friendship(&mut self, pc: i32, v: i16);
    fn gold(&self, pc: i32) -> i32;
    fn set_gold(&mut self, pc: i32, v: i32);
    fn parody_on(&self) -> bool;
    fn clear_flag(&self) -> i8;
    fn set_clear_flag(&mut self, v: i8);

    /// Set bit `n` of an int bit list (`dtWallpaperList`, `wordList`, ...).
    fn set_list_bit(&mut self, base: usize, words: usize, n: i32) {
        if let Some((w, b)) = bit_word(n)
            && let Some(v) = self.word_at(base, words, w as i32)
        {
            self.set_word_at(base, words, w as i32, v | b);
        }
    }
    fn clear_list_bit(&mut self, base: usize, words: usize, n: i32) {
        if let Some((w, b)) = bit_word(n)
            && let Some(v) = self.word_at(base, words, w as i32)
        {
            self.set_word_at(base, words, w as i32, v & !b);
        }
    }

    /// `ccSaveData::AddItem(pc, category, id, num)` (`INF 0x00177730`).
    fn add_item(&mut self, pc: i16, category: i16, id: i16, num: i16);
    /// `ccSaveData::DelItem(pc, category, id, num)` (`INF 0x00177af0`).
    fn del_item(&mut self, pc: i16, category: i16, id: i16, num: i16);
    /// `ccSaveData::AddSkill(pc, skill)` (`ccAddSkill`, `INF 0x00177590`).
    fn add_skill(&mut self, pc: i16, skill: i16);
    /// `ccSaveData::AddFriendship(pc, num)` (`INF 0x00177eb0`).
    fn add_friendship(&mut self, pc: i32, num: i32, volume: i32);
    /// `ccSaveData::SetGateList(server, area)` (`INF 0x00178160`).
    fn set_gate_list(&mut self, server: i32, area: i32);
    /// `ccSaveData::SetAreaBan` (`INF 0x00178480`).
    fn set_area_ban(&mut self, room: [i16; 4]);
    /// `ccSaveData::ClearAreaBan` (`INF 0x00178570`).
    fn clear_area_ban(&mut self, room: [i16; 4]);
}

fn in_range(len: usize, i: i32) -> Option<usize> {
    usize::try_from(i).ok().filter(|&i| i < len)
}

impl ScriptSave for SaveData {
    fn flags(&self, n: i32) -> u64 {
        in_range(512, n).map_or(0, |i| self.event_flag(i))
    }
    fn set_flags(&mut self, n: i32, v: u64) {
        if let Some(i) = in_range(512, n) {
            self.set_event_flag(i, v);
        }
    }
    fn byte_at(&self, base: usize, len: usize, i: i32) -> Option<i8> {
        in_range(len, i).map(|i| self.u8(base + i) as i8)
    }
    fn set_byte_at(&mut self, base: usize, len: usize, i: i32, v: i8) {
        if let Some(i) = in_range(len, i) {
            self.set_u8(base + i, v as u8);
        }
    }
    fn word_at(&self, base: usize, len: usize, i: i32) -> Option<u32> {
        in_range(len, i).map(|i| self.i32(base + 4 * i) as u32)
    }
    fn set_word_at(&mut self, base: usize, len: usize, i: i32, v: u32) {
        if let Some(i) = in_range(len, i) {
            self.set_i32(base + 4 * i, v as i32);
        }
    }
    fn member_word(&self, at: usize) -> u32 {
        self.i32(at) as u32
    }
    fn set_member_word(&mut self, at: usize, v: u32) {
        self.set_i32(at, v as i32);
    }
    fn item(&self, pc: usize, k: usize) -> Item {
        let at = off::ITEM_LIST + 4 * (ITEMS * pc + k);
        Item { id: self.i16(at), category: self.u8(at + 2) as i8, count: self.u8(at + 3) as i8 }
    }
    fn set_item(&mut self, pc: usize, k: usize, it: Item) {
        let at = off::ITEM_LIST + 4 * (ITEMS * pc + k);
        self.set_i16(at, it.id);
        self.set_u8(at + 2, it.category as u8);
        self.set_u8(at + 3, it.count as u8);
    }
    fn skill(&self, pc: usize, k: usize) -> i16 {
        self.i16(off::SKILL_LIST + 2 * (SKILLS * pc + k))
    }
    fn set_skill(&mut self, pc: usize, k: usize, v: i16) {
        self.set_i16(off::SKILL_LIST + 2 * (SKILLS * pc + k), v);
    }
    fn friendship(&self, pc: i32) -> i16 {
        in_range(CHARACTERS, pc).map_or(0, |c| self.i16(off::SPC_PARAM + off::SPC_PARAM_SIZE * c + off::SPC_FRIENDSHIP))
    }
    fn set_friendship(&mut self, pc: i32, v: i16) {
        if let Some(c) = in_range(CHARACTERS, pc) {
            self.set_i16(off::SPC_PARAM + off::SPC_PARAM_SIZE * c + off::SPC_FRIENDSHIP, v);
        }
    }
    fn gold(&self, pc: i32) -> i32 {
        in_range(CHARACTERS, pc).map_or(0, |c| self.i32(off::SPC_PARAM + off::SPC_PARAM_SIZE * c + off::SPC_GOLD))
    }
    fn set_gold(&mut self, pc: i32, v: i32) {
        if let Some(c) = in_range(CHARACTERS, pc) {
            self.set_i32(off::SPC_PARAM + off::SPC_PARAM_SIZE * c + off::SPC_GOLD, v);
        }
    }
    fn parody_on(&self) -> bool {
        self.u8(off::PARODY_FLAG) != 0
    }
    fn clear_flag(&self) -> i8 {
        self.u8(off::CLEAR_FLAG) as i8
    }
    fn set_clear_flag(&mut self, v: i8) {
        self.set_u8(off::CLEAR_FLAG, v as u8);
    }

    fn add_item(&mut self, pc: i16, category: i16, id: i16, num: i16) {
        if pc == 0 && category == 15 {
            if let Some(i) = in_range(320, id as i32) {
                let mut n = num as i32 + self.u8(off::IMP_ITEM_LIST + i) as i8 as i32;
                if n >= 100 {
                    n = 99;
                }
                self.set_u8(off::IMP_ITEM_LIST + i, n as u8);
            }
            return;
        }
        let Some(pc) = in_range(CHARACTERS, pc as i32) else { return };
        // Copy the list out, clearing it, and find the slot to add to: the
        // matching item, else the first empty slot.
        let mut copy = [Item::EMPTY; ITEMS];
        let mut slot = None;
        for (k, c) in copy.iter_mut().enumerate() {
            let it = self.item(pc, k);
            *c = it;
            // The matching item wins; otherwise the first empty slot.
            let matching = it.id == id && it.category as i16 == category;
            if matching || (slot.is_none() && it.id < 0) {
                slot = Some(k);
            }
            self.set_item(pc, k, Item::EMPTY);
        }
        if let Some(k) = slot {
            let mut n = copy[k].count as i32 + num as i32;
            if n >= 100 {
                n = 99;
            }
            copy[k] = Item { id, category: category as i8, count: n as i8 };
        }
        // Put it back category by category, lowest id first.
        for &(cat, limit) in &ADD_ITEM_ORDER {
            for _ in 0..ITEMS {
                let mut best: Option<(usize, i16, i8)> = None;
                let mut low = limit;
                for (k, c) in copy.iter().enumerate() {
                    if c.category as i16 == cat && c.id < low {
                        low = c.id;
                        best = Some((k, c.id, c.count));
                    }
                }
                let Some((k, bid, count)) = best else { break };
                copy[k] = Item::EMPTY;
                if let Some(j) = (0..ITEMS).find(|&j| self.item(pc, j).id < 0) {
                    self.set_item(pc, j, Item { id: bid, category: cat as i8, count });
                }
            }
        }
    }

    fn del_item(&mut self, pc: i16, category: i16, id: i16, num: i16) {
        if pc == 0 && category == 15 {
            if let Some(i) = in_range(320, id as i32) {
                let mut n = self.u8(off::IMP_ITEM_LIST + i) as i8 as i32 - num as i32;
                if n < 0 {
                    n = 0;
                }
                self.set_u8(off::IMP_ITEM_LIST + i, n as u8);
            }
            return;
        }
        let Some(pc) = in_range(CHARACTERS, pc as i32) else { return };
        for k in 0..ITEMS {
            let it = self.item(pc, k);
            if it.id == id && it.category as i16 == category {
                if (num as i32) < it.count as i32 {
                    self.set_item(pc, k, Item { count: (it.count as i32 - num as i32) as i8, ..it });
                } else {
                    self.set_item(pc, k, Item::EMPTY);
                }
                return;
            }
        }
    }

    fn add_skill(&mut self, pc: i16, skill: i16) {
        let Some(pc) = in_range(CHARACTERS, pc as i32) else { return };
        // Skills 2-5 are one skill (Data Drain's levels share id 2).
        let skill = if (2..6).contains(&skill) { 2 } else { skill };
        let mut copy = [0i16; SKILLS];
        let mut free = None;
        for (k, c) in copy.iter_mut().enumerate() {
            let s = self.skill(pc, k);
            if s == skill {
                return;
            }
            if free.is_none() && s == -1 {
                free = Some(k);
            }
            *c = s;
        }
        if let Some(k) = free {
            self.set_skill(pc, k, skill);
            copy[k] = skill;
        }
        for t in 0..SKILLS {
            let mut low = 304i16;
            let mut at = None;
            for (k, &s) in copy.iter().enumerate() {
                if s >= 0 && s < low {
                    low = s;
                    at = Some(k);
                }
            }
            let v = match at {
                Some(k) if low != 304 => {
                    copy[k] = -1;
                    low
                }
                _ => -1,
            };
            self.set_skill(pc, t, v);
        }
    }

    fn add_friendship(&mut self, pc: i32, num: i32, volume: i32) {
        if in_range(CHARACTERS, pc).is_none() {
            return;
        }
        let f = (self.friendship(pc) as i32 + num) as i16;
        let v = if f < 0 {
            0
        } else {
            let cap = FRIENDSHIP_CAP.get(volume as usize).copied().unwrap_or(1000);
            if cap < f as i32 { cap as i16 } else { f }
        };
        self.set_friendship(pc, v);
    }

    fn set_gate_list(&mut self, server: i32, area: i32) {
        let Some(sv) = in_range(5, server) else { return };
        if let Some((w, b)) = bit_word(area)
            && w < 5
        {
            let at = off::GATE_LIST + 20 * sv + 4 * w;
            self.set_i32(at, (self.i32(at) as u32 | b) as i32);
        }
        let order = off::GATE_ORDER_LIST + 128 * sv;
        for k in 0..64 {
            if self.i16(order + 2 * k) as i32 == area {
                self.set_i16(order + 2 * k, -1);
            }
        }
        for k in (1..64).rev() {
            let prev = self.i16(order + 2 * (k - 1));
            self.set_i16(order + 2 * k, prev);
        }
        self.set_i16(order, area as i16);
    }

    fn set_area_ban(&mut self, room: [i16; 4]) {
        let entry = |s: &SaveData, i: usize| -> [i32; 4] {
            std::array::from_fn(|k| s.u8(off::AREA_BAN + 4 * i + k) as i8 as i32)
        };
        let want = room.map(|v| v as i32);
        if (0..32).any(|i| entry(self, i) == want) {
            return;
        }
        if let Some(i) = (0..32).find(|&i| entry(self, i) == [-1; 4]) {
            for (k, v) in room.iter().enumerate() {
                self.set_u8(off::AREA_BAN + 4 * i + k, *v as u8);
            }
        }
    }

    fn clear_area_ban(&mut self, room: [i16; 4]) {
        let want = room.map(|v| v as i32);
        for i in 0..32 {
            let e: [i32; 4] = std::array::from_fn(|k| self.u8(off::AREA_BAN + 4 * i + k) as i8 as i32);
            if e == want {
                for k in 0..4 {
                    self.set_u8(off::AREA_BAN + 4 * i + k, 0xff);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialised() -> SaveData {
        let mut s = SaveData::new();
        for pc in 0..CHARACTERS {
            for k in 0..ITEMS {
                s.set_item(pc, k, Item::EMPTY);
            }
            for k in 0..SKILLS {
                s.set_skill(pc, k, -1);
            }
        }
        s
    }

    #[test]
    fn items_sort_on_add() {
        let mut s = initialised();
        s.add_item(1, 0, 5, 2);
        s.add_item(1, 10, 3, 1);
        s.add_item(1, 0, 2, 1);
        s.add_item(1, 0, 5, 200);
        let got: Vec<_> = (0..4).map(|k| s.item(1, k)).collect();
        assert_eq!(
            got,
            [
                Item { id: 3, category: 10, count: 1 },
                Item { id: 2, category: 0, count: 1 },
                Item { id: 5, category: 0, count: 99 },
                Item::EMPTY
            ]
        );
        s.del_item(1, 0, 5, 98);
        assert_eq!(s.item(1, 2).count, 1);
        s.del_item(1, 0, 5, 1);
        assert_eq!(s.item(1, 2), Item::EMPTY);
    }

    #[test]
    fn skills_sort_and_merge() {
        let mut s = initialised();
        s.add_skill(0, 40);
        s.add_skill(0, 4);
        s.add_skill(0, 2);
        assert_eq!((0..3).map(|k| s.skill(0, k)).collect::<Vec<_>>(), [2, 40, -1]);
    }
}
