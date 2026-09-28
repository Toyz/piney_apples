//! `npcTbl` (npctbl.cpp; the volume's `tables::battle::npcs`): the town
//! NPCs (175 on Infection), each a `ccNpcTable` of 0x70 bytes - its base
//! parameters (`ccNpcParam`, what `ccChar::SetBaseParam` points
//! `ccChar::base` at) and its entry (`ccEntry`: the constructor the entry
//! control calls, the file it loads, a palette to swap in).
//!
//! ```text
//! +0x00 name        "Weapon Shop", "Wing"...
//! +0x04 label       "EQUIPSHOP", "PC0"... (not a file)
//! +0x08 type        flags: 0x08 a walking PC, 0x10 an administrator
//!                   (merchant-like), 0x100-0x1000 the five shops
//! +0x0c id          the row (`ccEvent::GetNpc` matches it)
//! +0x0e level  +0x10 exp  +0x14 gold
//! +0x18 height (180) +0x1c width (100 for shops, 45 for PCs): the size
//!                   ccChar::Draw's camera fade measures
//! +0x20 msg         the NPC's lines (a table of message pointers)
//! +0x28 entry: exist, func (ccEntryRtownMerchant or ccEntryRtownPC),
//!       esize, ep, ccsc, ccsc2, anm,
//!       +0x48 clut[30] (a CLUT/TEX name, may be empty),
//!       +0x68 fileList: DATA.BIN category, file name ("CTR1.CCS")
//! ```

use piney_data::tables::battle;
use piney_data::tables::types::EntryFunc;
use piney_data::volume::Volume;
use piney_data::{Error, Result};

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
        let t = battle::of(volume).npcs().get(row).ok_or_else(|| Error::NotFound(format!("npcTbl row {row}")))?;
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
        (0..battle::of(volume).npcs().len()).map(|r| NpcRow::of(volume, r)).collect()
    }

    /// The file's DATA.BIN stem: "CTR1.CCS" -> "ctr1".
    pub fn stem(&self) -> String {
        self.file.split('.').next().unwrap_or("").to_ascii_lowercase()
    }
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
