//! Readers for the .hack PS2 discs.
//!
//! Everything here follows the reference pages under `docs/` and the Python
//! tools under `tools/` that established them; the integration tests check
//! the results against the counts those tools measured.
//!
//! - [`iso`]: the DVD image (ISO 9660, 2048-byte sectors).
//! - [`archive`]: `DATA/DATA.BIN` and `STREAM/*.BIN`, sector-aligned gzip
//!   members named by their FNAME field (`docs/formats/data-bin.md`).
//! - [`ccs`]: the CCSF scene file and its chunk walk (`docs/formats/ccs.md`).
//! - [`texture`]: palettes and 4/8-bit textures.
//! - [`model`]: the Model chunk (`docs/formats/ccs-model.md`).
//! - [`scene`]: objects, clumps, materials, dummy positions and animation
//!   poses, and models placed in world space.
//! - [`anim`]: Anime chunks evaluated at any time as `ccAnm` plays them:
//!   object poses, texture offsets, morph weights and morph blending
//!   (`tools/anim.py`, checked against the game by `tools/test_anim.py`).
//! - [`libm`]: the game's single-precision maths library (newlib's `sinf`,
//!   `cosf`, `atan2f`, ...) in the EE FPU's arithmetic, bit for bit.
//! - [`area`]: the Chaos Gate keywords, the story areas and the area
//!   generator (`WORLD_MAN::SimGenerateCode`), the tables read from the
//!   executable.
//! - [`save`]: the game's save data, `ccSaveData`, as its bytes with the
//!   members the port uses.
//! - [`statics`]: the tables that place a town's pieces, read from the
//!   executable into the build by `piney-gen`.
//! - [`dungeon`]: random dungeons from `dungeonSeed` and the story dungeons'
//!   layouts, with the tables `piney-gen` reads from the executable into the
//!   build.
//! - [`field`]: field terrain and objects from `fieldSeed`, the heights
//!   bit-exact EE floats, and what the game draws of it (ground and cover
//!   tiles, object heights, the lit vertex colours, water and background),
//!   with the tables `piney-gen` reads from the executable into the build.
//! - [`events`]: each volume's event scripts and messages, generated.
//! - [`volume`]: which of the four discs this is, from `DATA/GCMN.PRG`.

pub mod anim;
pub mod archive;
pub mod area;
pub mod ccs;
pub mod dungeon;
pub mod exe;
pub mod field;
pub mod iso;
pub mod libm;
pub mod model;
pub mod pack;
pub mod pss;
pub mod save;
pub mod scene;
pub mod shadow;
pub mod sound;
pub mod statics;
pub mod store;
pub mod tables;
pub mod texture;
pub mod volume;
pub mod world;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// The bytes are not what the format says they should be.
    Format(String),
    NotFound(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Format(s) => write!(f, "format: {s}"),
            Error::NotFound(s) => write!(f, "not found: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn format_err<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Format(msg.into()))
}

/// Little-endian reads that fail instead of panicking on short data.
pub(crate) trait Bytes {
    fn u8_at(&self, at: usize) -> Result<u8>;
    fn u16_at(&self, at: usize) -> Result<u16>;
    fn i16_at(&self, at: usize) -> Result<i16>;
    fn u32_at(&self, at: usize) -> Result<u32>;
    fn i32_at(&self, at: usize) -> Result<i32>;
    fn f32_at(&self, at: usize) -> Result<f32>;
    fn slice_at(&self, at: usize, len: usize) -> Result<&[u8]>;
}

impl Bytes for [u8] {
    fn u8_at(&self, at: usize) -> Result<u8> {
        self.get(at).copied().ok_or_else(|| short(at, 1, self.len()))
    }
    fn u16_at(&self, at: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.slice_at(at, 2)?.try_into().unwrap()))
    }
    fn i16_at(&self, at: usize) -> Result<i16> {
        Ok(i16::from_le_bytes(self.slice_at(at, 2)?.try_into().unwrap()))
    }
    fn u32_at(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.slice_at(at, 4)?.try_into().unwrap()))
    }
    fn i32_at(&self, at: usize) -> Result<i32> {
        Ok(i32::from_le_bytes(self.slice_at(at, 4)?.try_into().unwrap()))
    }
    fn f32_at(&self, at: usize) -> Result<f32> {
        Ok(f32::from_le_bytes(self.slice_at(at, 4)?.try_into().unwrap()))
    }
    fn slice_at(&self, at: usize, len: usize) -> Result<&[u8]> {
        at.checked_add(len).and_then(|end| self.get(at..end)).ok_or_else(|| short(at, len, self.len()))
    }
}

fn short(at: usize, len: usize, have: usize) -> Error {
    Error::Format(format!("read of {len} bytes at 0x{at:x} runs past the end (0x{have:x})"))
}

/// A NUL-terminated name in a fixed field, as Latin-1.
pub(crate) fn cstr(field: &[u8]) -> String {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    field[..end].iter().map(|&b| b as char).collect()
}

pub(crate) fn align4(q: usize) -> usize {
    (q + 3) & !3
}
