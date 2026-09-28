//! Readers for the .hack PS2 discs, following the reference pages under
//! `docs/` and the Python tools under `tools/` that established them: the
//! image ([`iso`], [`archive`] for `DATA.BIN` and the streams), CCSF scenes
//! ([`ccs`], [`texture`], [`model`], [`scene`], [`anim`]), the game's maths
//! ([`libm`]), the Chaos Gate words and areas ([`area`]), dungeons and fields
//! ([`dungeon`], [`field`]), town statics ([`statics`]), the save ([`save`])
//! and which disc it is ([`volume`]). The tables read from each volume's
//! executable are generated into the build by `piney-gen`.

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
