//! The `.sq` sequence file (`docs/formats/snddata.md`): `Vers`, then `Sequ`
//! (u32 file size and the song, MIDI, SE-sequence and SE-song chunk
//! offsets), then a `Midi` chunk - a count and a table of offsets to MIDI
//! data blocks, relative to the chunk.

use super::hd::table;
use crate::{Bytes, Result, format_err};

const NONE: u32 = 0xffff_ffff;

#[derive(Clone, Debug, Default)]
pub struct Sq {
    /// The whole file.
    pub data: Vec<u8>,
    /// `Sequ` +0x0c.
    pub size: u32,
    /// Chunk offsets from `Sequ`, `None` for 0xffffffff: song, MIDI,
    /// SE sequence, SE song.
    pub chunks: [Option<u32>; 4],
    /// Absolute offsets of the MIDI data blocks, in `Midi` slot order,
    /// empty slots left out.
    pub midi: Vec<usize>,
}

impl Sq {
    pub fn parse(data: Vec<u8>) -> Result<Sq> {
        let d = &data[..];
        let kind = |p: usize| -> Result<([u8; 4], [u8; 4])> {
            let mut c: [u8; 4] = d.slice_at(p, 4)?.try_into().unwrap();
            let mut k: [u8; 4] = d.slice_at(p + 4, 4)?.try_into().unwrap();
            c.reverse();
            k.reverse();
            Ok((c, k))
        };
        if kind(0)? != (*b"SCEI", *b"Vers") {
            return format_err("not an SCEI .sq: no Vers chunk");
        }
        if kind(16)? != (*b"SCEI", *b"Sequ") {
            return format_err("no Sequ chunk at 0x10");
        }
        let size = d.u32_at(28)?;
        let mut chunks = [None; 4];
        for (i, c) in chunks.iter_mut().enumerate() {
            let v = d.u32_at(32 + 4 * i)?;
            *c = (v != NONE).then_some(v);
        }
        let midi = match chunks[1] {
            Some(m) => table(d, m as usize, b"Midi")?.into_iter().flatten().collect(),
            None => Vec::new(),
        };
        Ok(Sq { size, chunks, midi, data })
    }
}
