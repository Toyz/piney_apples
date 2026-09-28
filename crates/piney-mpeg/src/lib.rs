//! The PSS movies of the .hack games: `PSS/LOGO_B.PSS`, `LOGO_C.PSS`,
//! `LOGO_H.PSS` and `OPENING.PSS` on Infection's disc, which the title plays
//! through `ccDecodeMpeg` (DEMO.PRG 0x00409df0).
//!
//! - [`decoder`]: an MPEG-2 video decoder of our own for what these streams
//!   use (main profile, 4:2:0, progressive frame pictures, I / P / B), bit
//!   for bit ffmpeg's `mpeg2video` output on all four files.
//! - [`idct`]: the integer IDCT it uses (IEEE 1180 compliant).
//! - [`csc`]: the IPU's YCbCr to RGBA32 conversion.
//! - [`movie`]: the player the runtime drives: pictures in display order as
//!   `piney_draw` frames, the audio as 48 kHz stereo PCM, the skip rule.
//!
//! The container is `piney_data::pss`; `docs/formats/pss.md` has the
//! formats and the game's player.

mod bits;
pub mod csc;
pub mod decoder;
pub mod idct;
pub mod movie;
mod vlc;

pub use decoder::{Decoder, Kind, Picture, PictureHeader, Sequence};
pub use movie::{Movie, MovieAudio};
