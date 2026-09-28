//! Which volume a disc is.
//!
//! The port reads a disc's data files at run time (`DATA/DATA.BIN`, the
//! overlays `DATA/*.PRG`, the streams and the voices), never its boot
//! executable. What the port needs from the executable is generated into
//! this crate, per volume, by the tools (`plans/volumes.md`), as the
//! dungeon, field, sound and statics tables already are.
//!
//! [`Volume::detect`] tells the four discs apart by `DATA/GCMN.PRG`, which
//! the port loads anyway: its size picks the volume and its hash confirms
//! it, so a patched overlay is not taken for a known one.

use std::fmt;

/// One of the four .hack discs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Volume {
    /// .hack//Infection, `SLUS_202.67`.
    #[default]
    Inf,
    /// .hack//Mutation, `SLUS_205.62`.
    Mut,
    /// .hack//Outbreak, `SLUS_205.63`.
    Out,
    /// .hack//Quarantine, `SLUS_205.64`.
    Qua,
}

/// `DATA/GCMN.PRG`'s path on every disc.
pub const GCMN_PATH: &str = "DATA/GCMN.PRG";

/// Each volume's `GCMN.PRG`: its length and its FNV-1a 64-bit hash.
const GCMN: [(Volume, usize, u64); 4] = [
    (Volume::Inf, 3_121_280, 0x9349_383e_707e_0cad),
    (Volume::Mut, 3_322_624, 0x5fb2_3ba6_c872_3733),
    (Volume::Out, 3_335_680, 0xe10a_f1cf_b577_2638),
    (Volume::Qua, 3_377_280, 0x97a3_9d71_5e01_020f),
];

impl Volume {
    pub const ALL: [Volume; 4] = [Volume::Inf, Volume::Mut, Volume::Out, Volume::Qua];

    /// The volume whose `GCMN.PRG` these bytes are.
    pub fn detect(gcmn: &[u8]) -> Result<Volume, Unknown> {
        let (v, _, hash) = GCMN.iter().find(|(_, len, _)| *len == gcmn.len()).ok_or(Unknown::Size(gcmn.len()))?;
        let h = fnv1a64(gcmn);
        if h == *hash { Ok(*v) } else { Err(Unknown::Hash(*v, h)) }
    }

    /// `charTbl`'s rows, the characters: 18, and from Mutation on 21 (the
    /// last three kept in the save's extension).
    pub fn characters(self) -> usize {
        if self == Volume::Inf { 18 } else { 21 }
    }

    /// The volume's number, as the game's `volumeNum` holds it (1-4).
    pub fn number(self) -> i32 {
        self as i32 + 1
    }

    /// The volume `volumeNum` names (1-4).
    pub fn from_number(n: i32) -> Option<Volume> {
        [Volume::Inf, Volume::Mut, Volume::Out, Volume::Qua].get(usize::try_from(n - 1).ok()?).copied()
    }

    /// The game's title.
    pub fn title(self) -> &'static str {
        match self {
            Volume::Inf => ".hack//Infection",
            Volume::Mut => ".hack//Mutation",
            Volume::Out => ".hack//Outbreak",
            Volume::Qua => ".hack//Quarantine",
        }
    }

    /// The boot executable's name, for the tools and the tests' checks; the
    /// port never opens it.
    pub fn executable(self) -> &'static str {
        match self {
            Volume::Inf => "SLUS_202.67",
            Volume::Mut => "SLUS_205.62",
            Volume::Out => "SLUS_205.63",
            Volume::Qua => "SLUS_205.64",
        }
    }
}

impl fmt::Display for Volume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

/// Why a disc is none of the four.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unknown {
    /// No volume's `GCMN.PRG` has this length.
    Size(usize),
    /// The length is this volume's but the bytes are not (the hash found).
    Hash(Volume, u64),
}

impl fmt::Display for Unknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let known = "the port knows .hack//Infection, Mutation, Outbreak and Quarantine (NTSC-U)";
        match self {
            Unknown::Size(n) => write!(f, "{GCMN_PATH} is {n} bytes, no known disc's; {known}"),
            Unknown::Hash(v, h) => {
                write!(f, "{GCMN_PATH} has {v}'s length but not its bytes (hash {h:#018x}); {known}")
            }
        }
    }
}

impl std::error::Error for Unknown {}

/// The volume of the disc in `iso`, from its `GCMN.PRG`
/// ([`crate::iso::Iso::volume`]).
pub fn of_disc(iso: &mut crate::iso::Iso) -> crate::Result<Volume> {
    iso.volume()
}

/// FNV-1a, 64 bits.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_known_values() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn a_wrong_size_or_hash_is_refused() {
        assert_eq!(Volume::detect(&[0; 16]), Err(Unknown::Size(16)));
        let fake = vec![0u8; 3_121_280];
        assert!(matches!(Volume::detect(&fake), Err(Unknown::Hash(Volume::Inf, _))));
    }

    /// Each disc present under `work/` is its own volume.
    #[test]
    fn the_four_discs() {
        let work = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../work");
        for (v, dir) in Volume::ALL.into_iter().zip(["infection", "mutation", "outbreak", "quarantine"]) {
            let path = work.join(dir).join(format!("{dir}.iso"));
            if !path.exists() {
                continue;
            }
            let mut iso = crate::iso::Iso::open(&path).unwrap();
            assert_eq!(of_disc(&mut iso).unwrap(), v);
        }
    }

    /// Each volume's generated tables are its own: its `volumeNum`, and
    /// what changed from Infection where it changed.
    #[test]
    fn each_volume_has_its_tables() {
        use crate::{area::AreaTables, dungeon, field, statics};
        let (inf_models, inf_objs, inf_sets) = statics::of(Volume::Inf);
        for v in Volume::ALL {
            assert_eq!(AreaTables::of(v).volume, v.number());
            let d = dungeon::tables_of(v);
            assert_eq!(d.volume as i32, v.number());
            // From Mutation on, area 125's dungeon has 15 floors.
            assert_eq!(d.edit_floors_by_event.iter().any(|&(e, f)| e == 125 && f == 15), v != Volume::Inf);
            // The towns' and event areas' pieces are the same on every disc;
            // only where they sit in memory moves.
            let (models, objs, sets) = statics::of(v);
            assert_eq!(models.len(), inf_models.len());
            for (a, b) in models.iter().zip(inf_models) {
                assert_eq!((a.name, a.scenes, a.rows), (b.name, b.scenes, b.rows), "{v} {}", a.name);
            }
            assert_eq!(objs.len(), inf_objs.len());
            assert_eq!(sets, inf_sets, "{v}");
            // The fields' tables are Infection's, but for Outbreak's and
            // Quarantine's square root.
            let f = field::tables_of(v);
            assert_eq!(f.volume as i32, v.number());
            let fpu = matches!(v, Volume::Out | Volume::Qua);
            assert_eq!(f.sqrt, if fpu { field::Sqrt::Fpu } else { field::Sqrt::Newlib });
            let same = |t: &field::Tables| {
                format!("{t:?}").replacen(&format!("volume: {}", t.volume), "", 1).replace("sqrt: Fpu", "sqrt: Newlib")
            };
            assert_eq!(same(f), same(field::tables_of(Volume::Inf)), "{v}");
        }
    }

    #[test]
    fn numbers_and_names() {
        assert_eq!(Volume::ALL.map(Volume::number), [1, 2, 3, 4]);
        assert_eq!(Volume::Qua.executable(), "SLUS_205.64");
    }
}
