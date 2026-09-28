//! The enemies' named blocks the races and their motions point at: the
//! weapon trails' `*WpInfo`, the dust's `*DustInfo`, the breaths'
//! `ehkBreathInfo`, `elBrInfo`, `ehkBrParam` and `elBrParam`. The game
//! passes their addresses; the port passes the block's name and a row
//! ([`InfoRef`]), and finds the rows in the volume's tables
//! (`tables::combat`).

use piney_data::tables::combat;
use piney_data::tables::types::{EnemyBrInfo, EnemyBrParam, EnemyDustInfo, EnemyWpInfo};
use piney_data::volume::Volume;

/// A row of one of the enemies' named blocks: the game's name for the
/// block and the row in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InfoRef {
    pub table: &'static str,
    pub row: usize,
}

impl InfoRef {
    pub const fn new(table: &'static str, row: usize) -> InfoRef {
        InfoRef { table, row }
    }

    /// Where the volume keeps the row: the pointer the game passes.
    pub fn va(self, volume: Volume) -> Option<u32> {
        let t = combat::of(volume);
        let (base, size) = match self.table {
            "ehkBreathInfo" => (t.ehk_breath_info_va(), 0x1c),
            "elBrInfo" => (t.el_br_info_va(), 0x1c),
            "ehkBrParam" => (t.ehk_br_param_va(), 0x40),
            "elBrParam" => (t.el_br_param_va(), 0x40),
            name if name.ends_with("WpInfo") => (find(t.weapon_infos(), name, |b| (b.name, b.va))?, 0x60),
            name if name.ends_with("DustInfo") => (find(t.dust_infos(), name, |b| (b.name, b.va))?, 0x20),
            _ => return None,
        };
        Some(base + size * self.row as u32)
    }
}

fn find<B, T>(blocks: &[B], name: &str, key: impl Fn(&B) -> (&str, T)) -> Option<T> {
    let i = blocks.binary_search_by(|b| key(b).0.cmp(name)).ok()?;
    Some(key(&blocks[i]).1)
}

/// A weapon block's rows from `info`'s row on.
pub fn weapon_rows(volume: Volume, info: InfoRef) -> &'static [EnemyWpInfo] {
    let t = combat::of(volume).weapon_infos();
    find(t, info.table, |b| (b.name, b.rows)).and_then(|r| r.get(info.row..)).unwrap_or_default()
}

/// A dust block's rows from `info`'s row on.
pub fn dust_rows(volume: Volume, info: InfoRef) -> &'static [EnemyDustInfo] {
    let t = combat::of(volume).dust_infos();
    find(t, info.table, |b| (b.name, b.rows)).and_then(|r| r.get(info.row..)).unwrap_or_default()
}

/// A breath's `ccEnemyBrInfo` (`ehkBreathInfo` or `elBrInfo`).
pub fn breath_info(volume: Volume, info: InfoRef) -> Option<&'static EnemyBrInfo> {
    let t = combat::of(volume);
    match info.table {
        "ehkBreathInfo" => t.ehk_breath_info().get(info.row),
        "elBrInfo" => t.el_br_info().get(info.row),
        _ => None,
    }
}

/// A breath's `ccEnemyBrParam` (`ehkBrParam` or `elBrParam`).
pub fn breath_param(volume: Volume, info: InfoRef) -> Option<&'static EnemyBrParam> {
    let t = combat::of(volume);
    match info.table {
        "ehkBrParam" => t.ehk_br_param().get(info.row),
        "elBrParam" => t.el_br_param().get(info.row),
        _ => None,
    }
}

/// `n` weapon rows from `info` as the controller takes them; a row
/// without its node is left out.
pub fn weapons(volume: Volume, info: InfoRef, n: i32) -> Vec<crate::weapon::WpInfo> {
    let rows = weapon_rows(volume, info);
    rows.iter().take(usize::try_from(n).unwrap_or(0)).filter_map(crate::weapon::WpInfo::of).collect()
}

/// `n` dust rows from `info` laid out as the game keeps them.
pub fn dusts(volume: Volume, info: InfoRef, n: i32) -> Vec<[u8; EnemyDustInfo::SIZE]> {
    let rows = dust_rows(volume, info);
    rows.iter()
        .take(usize::try_from(n).unwrap_or(0))
        .map(|r| {
            let mut b = [0; EnemyDustInfo::SIZE];
            r.write(&mut b);
            b
        })
        .collect()
}

/// A breath's info with its names.
pub fn br_info(volume: Volume, info: InfoRef) -> crate::breath::BrInfo {
    let Some(r) = breath_info(volume, info) else { return crate::breath::BrInfo::default() };
    let text = |s: Option<&str>| s.unwrap_or_default().to_owned();
    crate::breath::BrInfo {
        kind: r.axis,
        smoke: r.eff_type,
        node: text(r.obj_name),
        eff: text(r.eff_name),
        clut: text(r.clt_name),
    }
}

/// A breath's parameters.
pub fn br_param(volume: Volume, info: InfoRef) -> crate::breath::BrParam {
    let Some(r) = breath_param(volume, info) else { return crate::breath::BrParam::default() };
    let mut b = [0; EnemyBrParam::SIZE];
    r.write(&mut b);
    crate::breath::BrParam::read(&b)
}
