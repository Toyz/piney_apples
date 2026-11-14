//! `npcTbl` (npctbl.cpp; the volume's `tables::battle::npcs`): the town NPCs
//! (175 on Infection), each a `ccNpcTable` of 0x70 bytes: its base parameters
//! (`ccNpcParam`, what `ccChar::SetBaseParam` points `ccChar::base` at: name,
//! type flags, id, height and width, lines) and its entry (`ccEntry`: the
//! constructor the entry control calls, the file it loads, a palette to swap
//! in). The layout is in docs/engine/field-game.md.

use piney_data::tables::battle;
use piney_data::tables::types::EntryFunc;
use piney_data::volume::Volume;
use piney_data::{Error, Result};

use crate::chara::char::Char;
use crate::chara::entry_obj::EntryObj;
use crate::ee::{self, F, V4};
use crate::hit::Hits;

/// Half a radian: [`Touch::avoid`]'s turn.
const HALF: F = 0x3f00_0000;

/// Base type flags.
pub const TYPE_PC: u32 = 0x08;
pub const TYPE_SYSOPE: u32 = 0x10;
/// A Grunt Shop's breeder (rows 10, 16, 22, 28): `SetMerchantCamera`'s
/// fixed views.
pub const TYPE_BREEDER: u32 = 0x0800_0000;

/// One `npcTbl` row.
#[derive(Clone, Debug, PartialEq)]
pub struct NpcRow {
    pub row: usize,
    pub name: String,
    pub label: String,
    pub flags: u32,
    pub id: i16,
    pub level: i16,
    pub exp: i16,
    pub gold: i32,
    /// Bits of the base parameters' height and width.
    pub height: u32,
    pub width: u32,
    /// The address of the NPC's message table (`base->msg`).
    pub msg: u32,
    /// `entry.func`: `ccEntryRtownMerchant` or `ccEntryRtownPC`.
    pub func: Option<EntryFunc>,
    /// `entry.clut`, empty when none.
    pub clut: String,
    /// `entry.fileList`: the DATA.BIN category and the file.
    pub category: u32,
    pub file: String,
}

impl NpcRow {
    /// Row `row` of the volume's `npcTbl` (`tables::battle`).
    pub fn of(volume: Volume, row: usize) -> Result<NpcRow> {
        let t = battle::of(volume).npcs.get(row).ok_or_else(|| Error::NotFound(format!("npcTbl row {row}")))?;
        let (b, e) = (&t.param.base, &t.entry);
        let text = |s: Option<&str>| s.unwrap_or_default().to_owned();
        Ok(NpcRow {
            row,
            name: text(b.name),
            label: text(b.ccsname),
            flags: b.kind as u32,
            id: b.id,
            level: b.level,
            exp: b.exp,
            gold: b.gold,
            height: b.height.to_bits(),
            width: b.width.to_bits(),
            msg: b.msg,
            func: e.func,
            clut: e.clut.to_owned(),
            category: e.file_list.category as u32,
            file: text(e.file_list.name),
        })
    }

    /// Every row.
    pub fn all(volume: Volume) -> Result<Vec<NpcRow>> {
        (0..battle::of(volume).npcs.len()).map(|r| NpcRow::of(volume, r)).collect()
    }

    /// The file's DATA.BIN stem: "CTR1.CCS" -> "ctr1".
    pub fn stem(&self) -> String {
        self.file.split('.').next().unwrap_or("").to_ascii_lowercase()
    }

    /// The class the entry control makes of this row for an event's `entry
    /// ty` (3 or 4), or None when it makes nothing: `ccEntryEventMng` hands
    /// type 3 to `ccSetRtownPC(code, -1)` and type 4 to `ccSetMerchant(code)`,
    /// which makes only rows 29 and 158; both go through `entryObject`, which
    /// calls the row's `entry.func` (so `entry 3 29` is the Administrator).
    pub fn event_class(&self, ty: i16) -> Option<EventClass> {
        match (ty, self.row as i32) {
            (3, _) => {}
            (4, id) if crate::town::merchant::SYSOPE.contains(&id) => {}
            _ => return None,
        }
        match self.func? {
            EntryFunc::RtownPC => Some(EventClass::Pc),
            EntryFunc::RtownMerchant => Some(EventClass::Merchant),
            _ => None,
        }
    }
}

/// The parts every NPC class has: `ccChar` with its base (the `npcTbl`
/// row) and `affectType` (+0x9c), `ccEntryObj`, and the act machine and
/// body touch that `ccMerchan`, `ccRtownPC`, `ccDog` and `ccPGuso` each
/// declare over `ccGimmick`'s and run the same way. A class holds its core
/// and adds its own.
#[derive(Clone)]
pub struct Core {
    pub row: NpcRow,
    pub ch: Char,
    pub entry: EntryObj,
    /// +0x9c `affectType`: the command `EntryAffect` last passed.
    pub affect_type: i16,
    pub acts: Acts,
    pub touch: Touch,
}

impl Core {
    /// The core of an NPC made from `row` with the character `ch`, as
    /// `ccEntryObj::ccEntryObj` and the class's constructor leave it.
    /// The entry (`entParam.id`) is the row's.
    pub fn new(row: NpcRow, ch: Char) -> Core {
        let entry = EntryObj { ent_id: i32::from(row.id), ..EntryObj::default() };
        Core { row, ch, entry, affect_type: 0, acts: Acts::default(), touch: Touch::default() }
    }

    /// Its base id (+0x0c): `ccEvent::GetNpc`'s code.
    pub fn code(&self) -> i32 {
        i32::from(self.row.id)
    }

    /// The row its entry made it from (`entParam.id`, +0x124): the NPC's
    /// name in the port ([`crate::world::CharId::Npc`]) and what the
    /// menus read of the list. A Grunty's base changes as it grows; this
    /// does not.
    pub fn id(&self) -> i32 {
        self.entry.ent_id
    }
}

/// A class's act machine: `actNum`, `actProcess` (the step within the act)
/// and `actCnt` (frames counted in it; the merchant's is `ccGimmick`'s
/// +0x1d8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Acts {
    pub num: i32,
    pub process: i32,
    pub cnt: i32,
}

/// `bodyHitFlag` and `bodyHitCnt`: touching another body, and for how many
/// frames running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Touch {
    pub flag: i16,
    pub cnt: i16,
}

impl Touch {
    /// A class's `main`'s `bodyHit.CollisionDetection()` (gcmn 0x00153470)
    /// at `ch`'s place on `hits`: touching sets the flag and counts up,
    /// else both clear; with `push` the class takes the push (its place and
    /// `posP` moved by the offset). True when touching.
    pub fn collide(&mut self, ch: &mut Char, hits: &mut Hits, push: bool) -> bool {
        ch.hit.pos = ch.pos;
        if hits.collision_detection(&mut ch.hit) == 0 {
            *self = Touch::default();
            return false;
        }
        if push {
            let off = ch.hit.offset;
            ch.pos = ee::vadd(ch.pos, off);
            ch.pos_p = ee::vadd(ch.pos_p, off);
            ch.hit.pos = ch.pos;
            hits.sync(&ch.hit);
        }
        self.flag = 1;
        self.cnt = self.cnt.wrapping_add(1);
        true
    }

    /// Each class's `move` (`ccRtownPC` 0x005077f0, `ccDog` 0x00509a80,
    /// `ccPGuso` 0x0050bca0) stepping round a body it touches: the heading
    /// `dir` half a radian right for 40 frames, left from 60 to 100, and
    /// the count over again from 120.
    pub fn avoid(&mut self, dir: F) -> F {
        if self.flag == 0 {
            return dir;
        }
        match self.cnt {
            ..40 => ee::sub(dir, HALF),
            60..100 => ee::add(dir, HALF),
            120.. => {
                self.cnt = 0;
                dir
            }
            _ => dir,
        }
    }
}

/// `posCnt` and `oldPos` of `ccDog` (+0x21c, +0x1f0) and `ccPGuso` (+0x298,
/// +0x210): the stuck check at the end of their `move` (0x00509a80,
/// 0x0050bca0), the same code in both. (`ccRtownPC`'s is its own: 60
/// frames, a distance, `returnRoute`.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stuck {
    pub cnt: i16,
    pub old: V4,
}

impl Stuck {
    /// Frames before the check, and the reach on any axis that counts as
    /// moved (50).
    pub const FRAMES: i16 = 120;
    const MOVED: F = 0x4248_0000;

    /// After the move, the class at `pos`: at 120 frames, moved more than
    /// 50 on any axis since the count began starts it over; at 0 the place
    /// is kept; the count goes on.
    pub fn check(&mut self, pos: V4) {
        if self.cnt == Self::FRAMES {
            let far = |a: F, b: F| {
                let d = ee::sub(a, b);
                !ee::le(d, Self::MOVED) || ee::lt(d, ee::neg(Self::MOVED))
            };
            if (0..3).any(|k| far(pos[k], self.old[k])) {
                self.cnt = 0;
            }
        }
        if self.cnt == 0 {
            self.old = pos;
        }
        self.cnt = self.cnt.wrapping_add(1);
    }

    /// Still under way: not yet past the check's frame.
    pub fn going(&self) -> bool {
        self.cnt <= Self::FRAMES
    }
}

/// An event NPC's class: `ccRtownPC` or `ccMerchan`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventClass {
    Pc,
    Merchant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_anu_npcs() {
        let rows = NpcRow::all(Volume::Inf).unwrap();
        assert_eq!(rows.len(), 175);
        let shops: Vec<(&str, &str, Option<EntryFunc>)> =
            rows[..5].iter().map(|r| (r.name.as_str(), r.file.as_str(), r.func)).collect();
        assert_eq!(
            shops,
            [
                ("Weapon Shop", "CTR1.CCS", Some(EntryFunc::RtownMerchant)),
                ("Elf's Haven", "CTR1.CCS", Some(EntryFunc::RtownMerchant)),
                ("Item Shop", "CTR1.CCS", Some(EntryFunc::RtownMerchant)),
                ("Magic Shop", "CTR1.CCS", Some(EntryFunc::RtownMerchant)),
                ("Recorder", "CTR1.CCS", Some(EntryFunc::RtownMerchant)),
            ]
        );
        assert!(rows.iter().enumerate().all(|(k, r)| r.id as usize == k));
        let wing = &rows[30];
        assert_eq!(
            (wing.name.as_str(), wing.stem().as_str(), wing.flags, wing.func),
            ("Wing", "ctbu2", TYPE_PC, Some(EntryFunc::RtownPC))
        );
        assert_eq!(rows[78].clut, "TEX_cbu5bodyv3");
    }
}
