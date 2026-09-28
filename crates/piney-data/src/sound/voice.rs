//! Voice lines: `VOICE/` and `VOICE_E/` (`docs/formats/voice.md`).
//!
//! A voice file is headerless mono signed 16-bit little-endian PCM at
//! 48 kHz; the executable's `VOICE_DATA` tables give each line's byte range.
//! The tables and the file names (the event dialogue's, the field's, the
//! food's and the skill words') are generated into
//! [`crate::tables::voice`]; which row a request plays, and how
//! SEWORDS.IRX streams it, is `piney-audio`'s.

use crate::iso::Iso;
use crate::{Error, Result};

/// The event number the port's runtime gives `ccVoicePgFood(food)` (main
/// 0x001800e0): a Grunty food calling out, row `food` of `voiceFoodTbl`
/// (`voiceFoodTblE`) from `FOOD.BIN` (`voiceFile` 18). The game sets the
/// request itself rather than through `ccVoiceRequest` (which has no such
/// group: `Driver::voice_request` plays nothing for it);
/// `piney_audio::Audio::voice` sends it to `Driver::food_voice_request`.
pub const FOOD_GROUP: i32 = -100;

/// A line's samples: its `size` bytes as 16-bit little-endian, as
/// `tools/sound.py export-voices` writes them.
pub fn voice_pcm(bytes: &[u8]) -> Vec<i16> {
    bytes.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect()
}

/// `len` bytes of voice file `path` (`VOICE_E/EVVOL1_E.BIN`) from `ofs` on
/// (fewer at the file's end).
pub fn read_voice(iso: &mut Iso, path: &str, ofs: u64, len: usize) -> Result<Vec<u8>> {
    let e = iso.find(path)?;
    if ofs > e.size as u64 {
        return Err(Error::Format(format!("{path}: offset {ofs} is past the end")));
    }
    iso.read_at(&e, ofs, len)
}
