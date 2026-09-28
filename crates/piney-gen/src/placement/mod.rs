//! The placement groups: tables `piney-data` keeps in hand-written types
//! (the area generator's words, the dungeons, the fields, the sound tables,
//! the towns' statics), read out of a volume's executable and written as
//! data for the build, in the form `piney_data::store::Load` reads
//! (`plans/build-data.md`, step 3). Each group is a function of the volume
//! giving its file's bytes.

pub mod area;
pub mod dungeon;
pub mod field;
pub mod sound;
pub mod statics;

use crate::volume::Vol;

/// What makes a placement group's file for a volume: None where the
/// volume has no such table.
pub type Maker = fn(Vol) -> Result<Option<Vec<u8>>, String>;

/// The placement groups, by their file's name.
pub const GROUPS: &[(&str, Maker)] = &[
    ("area", |v| area::bytes(v).map(Some)),
    ("dungeon", |v| dungeon::bytes(v).map(Some)),
    ("field", |v| field::bytes(v).map(Some)),
    ("setbl", sound::setbl),
    ("sound", |v| sound::bytes(v).map(Some)),
    ("statics", |v| statics::bytes(v).map(Some)),
];

/// Bytes in `piney_data::store`'s form.
#[derive(Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// A count before a slice's items.
    pub fn count(&mut self, n: usize) {
        self.u32(n as u32);
    }
    pub fn str(&mut self, s: &str) {
        self.count(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    /// `Option`'s tag.
    pub fn some(&mut self, yes: bool) {
        self.u8(u8::from(yes));
    }
}

/// Bytes as text one char per byte (Latin-1), as the port compares the
/// executable's texts byte for byte.
pub fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}
