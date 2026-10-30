//! The port's tables, from the build (`plans/build-data.md`): each generator
//! group's values for a volume are a file, `PINEY/TABLES/<group>.bin`, in the
//! disc registered for the volume ([`use_disc`]; an image's in
//! [`crate::pack::image_data_dir`]), or for the tools and the checks in
//! `work/data/<volume>/TABLES/`. Each group's type is a [`Table`], whose
//! `of(volume)` reads it once a run and hands out `&'static` references into
//! it. The format is the types' own ([`Load`]),
//! little-endian: numbers as they are, `f32` as bits, text and slices a u32
//! count first, `Option` a u8 tag, a struct field by field, an enum by name.

use std::any::Any;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::tables::{GROUPS, NAMES};
use crate::volume::Volume;

/// Where a group's file lives on a disc, under `PINEY/`.
pub const TABLES_DIR: &str = "PINEY/TABLES";

/// A generated type read out of a group's file.
pub trait Load: Sized {
    fn load(r: &mut Reader) -> Self;
}

/// A group's bytes, kept for the run (the `&'static` text and slices the
/// values hold point into them or are made from them once).
pub struct Reader {
    buf: &'static [u8],
    at: usize,
    what: String,
}

impl Reader {
    pub fn new(buf: &'static [u8], what: impl Into<String>) -> Reader {
        Reader { buf, at: 0, what: what.into() }
    }

    fn take(&mut self, n: usize) -> &'static [u8] {
        let Some(b) = self.buf.get(self.at..self.at + n) else {
            panic!("{}: ends at {} of {} wanting {n} more", self.what, self.at, self.buf.len())
        };
        self.at += n;
        b
    }

    fn count(&mut self) -> usize {
        u32::load(self) as usize
    }

    /// The whole file read.
    pub fn done(&self) -> bool {
        self.at == self.buf.len()
    }
}

macro_rules! load_num {
    ($($t:ty),*) => {$(
        impl Load for $t {
            fn load(r: &mut Reader) -> Self {
                <$t>::from_le_bytes(r.take(size_of::<$t>()).try_into().unwrap())
            }
        }
    )*};
}
load_num!(u8, i8, u16, i16, u32, i32, u64, i64);

impl Load for f32 {
    fn load(r: &mut Reader) -> Self {
        f32::from_bits(u32::load(r))
    }
}

impl Load for bool {
    fn load(r: &mut Reader) -> Self {
        u8::load(r) != 0
    }
}

impl Load for &'static str {
    fn load(r: &mut Reader) -> Self {
        let n = r.count();
        let what = r.what.clone();
        std::str::from_utf8(r.take(n)).unwrap_or_else(|e| panic!("{what}: text not UTF-8: {e}"))
    }
}

impl<T: Load + 'static> Load for &'static [T] {
    fn load(r: &mut Reader) -> Self {
        let n = r.count();
        let v: Vec<T> = (0..n).map(|_| T::load(r)).collect();
        Box::leak(v.into_boxed_slice())
    }
}

impl<T: Load, const N: usize> Load for [T; N] {
    fn load(r: &mut Reader) -> Self {
        std::array::from_fn(|_| T::load(r))
    }
}

impl<T: Load> Load for Option<T> {
    fn load(r: &mut Reader) -> Self {
        match u8::load(r) {
            0 => None,
            _ => Some(T::load(r)),
        }
    }
}

/// Where each volume's port data comes from, once the game says.
static DISCS: Mutex<[Option<PathBuf>; 4]> = Mutex::new([None, None, None, None]);

/// A group's values, read once by whichever thread asks first.
type Slot = OnceLock<&'static (dyn Any + Send + Sync)>;

/// Where [`Table::shared`]'s values sit in a group's row, after the four
/// volumes'.
const SHARED: usize = 4;

/// Each group's values by [`Table::ID`]: a slot per volume and one for the
/// shared values. Each slot reads its group once however many threads ask,
/// and a read once made is one atomic load.
static SLOTS: [[Slot; 5]; GROUPS] = [const { [const { OnceLock::new() }; 5] }; GROUPS];

/// A group of tables: one file per volume under [`TABLES_DIR`], read once a
/// run. The generated groups and the hand-written ones (`field`, `dungeon`,
/// `sound`, `area`, `statics`, the audio's `setbl`) alike.
pub trait Table: Load + Send + Sync + 'static {
    /// The group's place in [`tables::id`](crate::tables::id), which names
    /// its file (`tables::NAMES`).
    const ID: usize;

    /// Volume `v`'s values.
    fn of(v: Volume) -> &'static Self {
        let values = SLOTS[Self::ID][v as usize].get_or_init(|| read::<Self>(v));
        values.downcast_ref::<Self>().expect("a group read as another type")
    }

    /// The values every volume has alike, from whichever volume's tables
    /// can be had first (fixed at the first call).
    fn shared() -> &'static Self {
        let values = SLOTS[Self::ID][SHARED].get_or_init(|| Self::of(shared_volume::<Self>()));
        values.downcast_ref::<Self>().expect("a group read as another type")
    }
}

/// The disc (an image or a build's `.disc`) whose port data is volume
/// `v`'s tables from here on.
pub fn use_disc(v: Volume, disc: PathBuf) {
    DISCS.lock().unwrap()[v as usize] = Some(disc);
}

/// The repository's `work/` (this crate is `crates/piney-data`).
fn work() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work")
}

/// Where the tools' and checks' copy of a volume's tables is.
pub fn work_tables(v: Volume) -> PathBuf {
    work().join("data").join(crate::pack::disc_name(v).trim_end_matches(".disc")).join("TABLES")
}

/// A group's file for a volume.
fn bytes(v: Volume, name: &str) -> Vec<u8> {
    let path = format!("{TABLES_DIR}/{name}.bin");
    let disc = DISCS.lock().unwrap()[v as usize].clone();
    if let Some(d) = disc {
        let mut iso = crate::iso::Iso::open(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display()));
        return iso.read_path(&path).unwrap_or_else(|e| panic!("{} {path}: {e}", d.display()));
    }
    let file = work_tables(v).join(format!("{name}.bin"));
    std::fs::read(&file).unwrap_or_else(|e| {
        panic!(
            "{}: {e}: {v}'s tables are made by `cargo run --release -p piney-gen -- gen` (the tools' and checks' copy) or by piney-build",
            file.display()
        )
    })
}

/// Volume `v`'s values of group `T`, read from its file.
fn read<T: Table>(v: Volume) -> &'static T {
    let name = NAMES[T::ID];
    let buf: &'static [u8] = Box::leak(bytes(v, name).into_boxed_slice());
    let mut r = Reader::new(buf, format!("{v}'s {name}"));
    let value = T::load(&mut r);
    assert!(r.done(), "{v}'s {name}: {} bytes left over", buf.len() - r.at);
    Box::leak(Box::new(value))
}

/// The volume whose group `T` stands for every volume's: the first with a
/// disc registered, else the first whose tools' copy is there, else
/// Infection.
fn shared_volume<T: Table>() -> Volume {
    let registered: Vec<Volume> = {
        let d = DISCS.lock().unwrap();
        Volume::ALL.into_iter().filter(|&v| d[v as usize].is_some()).collect()
    };
    registered
        .first()
        .copied()
        .or_else(|| Volume::ALL.into_iter().find(|&v| work_tables(v).join(format!("{}.bin", NAMES[T::ID])).is_file()))
        .unwrap_or(Volume::Inf)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Threads that ask for one group at once all get the one read.
    #[test]
    fn a_group_is_read_once() {
        let barrier = std::sync::Barrier::new(8);
        let got: Vec<usize> = std::thread::scope(|s| {
            let asks: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        barrier.wait();
                        std::ptr::from_ref(crate::field::Tables::of(Volume::Mut)) as usize
                    })
                })
                .collect();
            asks.into_iter().map(|t| t.join().unwrap()).collect()
        });
        assert!(got.iter().all(|&p| p == got[0]), "{got:x?}");
    }

    /// Every group has its own place, and every place a group: the
    /// generated ones, the hand-written ones here, and piney-audio's setbl.
    #[test]
    fn each_group_has_its_own_id() {
        use crate::tables::*;
        let mut ids = vec![
            title::Title::ID,
            fonts::Fonts::ID,
            loaddisp::LoadDisp::ID,
            newgame::NewGame::ID,
            dtmenu::DtMenu::ID,
            desktop::Desktop::ID,
            mail::Mail::ID,
            kanji::Kanji::ID,
            staffroll::StaffRoll::ID,
            nameentry::NameEntry::ID,
            toppage::TopPage::ID,
            game::Game::ID,
            fieldui::FieldUi::ID,
            battle::Battle::ID,
            combat::Combat::ID,
            stream::Stream::ID,
            effect::Effect::ID,
            world::World::ID,
            party_chat::PartyChat::ID,
            talk::Talk::ID,
            voice::Voice::ID,
            book::Book::ID,
            race::Race::ID,
            crate::area::AreaTables::ID,
            crate::dungeon::Tables::ID,
            crate::field::Tables::ID,
            crate::sound::Tables::ID,
            crate::statics::Statics::ID,
            id::SETBL,
        ];
        ids.sort_unstable();
        assert_eq!(ids, (0..GROUPS).collect::<Vec<_>>());
    }

    /// The shared values are a volume's own, read once.
    #[test]
    fn shared_is_a_volumes_values() {
        let shared = crate::field::Tables::shared();
        assert!(Volume::ALL.into_iter().any(|v| std::ptr::eq(shared, crate::field::Tables::of(v))));
        assert!(std::ptr::eq(shared, crate::field::Tables::shared()));
    }
}
