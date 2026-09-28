//! setbl.cpp's tables (gcmn.prg), what the animation notes' sounds read
//! (`ccSeSetParamSPC`, `ccSeSetParamEnemy`, `ccSeSetParamInu`): its .data
//! from `spc0SeData` to the end of `inuSeData` as `SE_NT` rows, padding and
//! the two pointer tables included, since a note's param past its array's
//! end reads on into them. Read from the build (`PINEY/TABLES/setbl.bin`,
//! `piney-gen`'s `placement::sound::setbl`, `plans/build-data.md`);
//! Infection's, which the port uses for every volume.

use std::sync::LazyLock;

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

fn setbl() -> &'static Setbl {
    static READ: std::sync::OnceLock<&'static Setbl> = std::sync::OnceLock::new();
    READ.get_or_init(|| piney_data::store::group(Volume::Inf, "setbl"))
}

/// gcmn's address of the first row, `spc0SeData`.
pub static BASE: LazyLock<u32> = LazyLock::new(|| setbl().base);
/// The rows (code, note, velocity), padding and pointer tables included.
pub static ROWS: LazyLock<&'static [SeNt]> = LazyLock::new(|| setbl().rows);
/// `spcSeTbl`: by `ccCharBaseParam.id`, the row of [`ROWS`] its table
/// starts at.
pub static SPC: LazyLock<&'static [Option<u16>]> = LazyLock::new(|| setbl().spc);
/// `enemySeTbl`: by the race's `seCategory`.
pub static ENEMY: LazyLock<&'static [Option<u16>]> = LazyLock::new(|| setbl().enemy);
/// `inuSeData`.
pub static INU: LazyLock<u16> = LazyLock::new(|| setbl().inu);
