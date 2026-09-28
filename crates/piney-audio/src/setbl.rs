//! setbl.cpp's tables (gcmn.prg), what the animation notes' sounds read
//! (`ccSeSetParamSPC`, `ccSeSetParamEnemy`, `ccSeSetParamInu`): its .data
//! from `spc0SeData` to the end of `inuSeData` as `SE_NT` rows, padding and
//! the two pointer tables included, since a note's param past its array's
//! end reads on into them. Read from the build (`PINEY/TABLES/setbl.bin`,
//! `piney-gen`'s `placement::sound::setbl`, `plans/build-data.md`), one
//! file per volume.

use piney_data::store::{Load, Reader};
use piney_data::volume::Volume;

use crate::se3d::SeNt;

/// The file as the build keeps it.
struct Setbl {
    base: u32,
    rows: &'static [SeNt],
    spc: &'static [Option<u16>],
    enemy: &'static [Option<u16>],
    inu: u16,
}

impl Load for SeNt {
    fn load(r: &mut Reader) -> Self {
        SeNt { code: Load::load(r), note: Load::load(r), velocity: Load::load(r) }
    }
}

impl Load for Setbl {
    fn load(r: &mut Reader) -> Self {
        Setbl {
            base: Load::load(r),
            rows: Load::load(r),
            spc: Load::load(r),
            enemy: Load::load(r),
            inu: Load::load(r),
        }
    }
}

/// Volume `v`'s tables, read once a run.
fn of(v: Volume) -> &'static Setbl {
    static READ: [std::sync::OnceLock<&'static Setbl>; 4] = [const { std::sync::OnceLock::new() }; 4];
    READ[v as usize].get_or_init(|| piney_data::store::group(v, "setbl"))
}

/// gcmn's address of the first row, `spc0SeData`.
pub fn base(v: Volume) -> u32 {
    of(v).base
}

/// The rows (code, note, velocity), padding and pointer tables included.
pub fn rows(v: Volume) -> &'static [SeNt] {
    of(v).rows
}

/// `spcSeTbl`: by `ccCharBaseParam.id`, the row of [`rows`] its table
/// starts at.
pub fn spc(v: Volume) -> &'static [Option<u16>] {
    of(v).spc
}

/// `enemySeTbl`: by the race's `seCategory`.
pub fn enemy(v: Volume) -> &'static [Option<u16>] {
    of(v).enemy
}

/// `inuSeData`.
pub fn inu(v: Volume) -> u16 {
    of(v).inu
}
