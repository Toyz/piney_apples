//! The port's tables, from the build (`plans/build-data.md`): each generator
//! group's values for a volume are a file, `PINEY/TABLES/<group>.bin`, in the
//! disc registered for the volume ([`use_disc`]; an image's in
//! [`crate::pack::image_data_dir`]), or for the tools and the checks in
//! `work/data/<volume>/TABLES/`. [`group`] reads one once a run and hands out
//! `&'static` references into it. The format is the types' own ([`Load`]),
//! little-endian: numbers as they are, `f32` as bits, text and slices a u32
//! count first, `Option` a u8 tag, a struct field by field, an enum by name.

use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

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

/// Each group read so far, by volume and name.
type Loaded = HashMap<(usize, &'static str), &'static (dyn Any + Send + Sync)>;
static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);

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

/// Volume `v`'s values of group `name`, read once a run.
pub fn group<T: Load + Send + Sync + 'static>(v: Volume, name: &'static str) -> &'static T {
    let key = (v as usize, name);
    if let Some(x) = LOADED.lock().unwrap().get_or_insert_with(HashMap::new).get(&key) {
        return x.downcast_ref::<T>().expect("a group read as another type");
    }
    let buf: &'static [u8] = Box::leak(bytes(v, name).into_boxed_slice());
    let mut r = Reader::new(buf, format!("{v}'s {name}"));
    let value = T::load(&mut r);
    assert!(r.done(), "{v}'s {name}: {} bytes left over", buf.len() - r.at);
    let leaked: &'static T = Box::leak(Box::new(value));
    LOADED.lock().unwrap().get_or_insert_with(HashMap::new).entry(key).or_insert(leaked);
    leaked
}

/// A group's values that are the same on every volume: from whichever
/// volume's tables can be had first.
pub fn shared<T: Load + Send + Sync + 'static>(name: &'static str) -> &'static T {
    let registered: Vec<Volume> = {
        let d = DISCS.lock().unwrap();
        Volume::ALL.into_iter().filter(|&v| d[v as usize].is_some()).collect()
    };
    let v = registered
        .first()
        .copied()
        .or_else(|| Volume::ALL.into_iter().find(|&v| work_tables(v).join(format!("{name}.bin")).is_file()))
        .unwrap_or(Volume::Inf);
    group(v, name)
}
