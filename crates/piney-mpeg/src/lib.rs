//! The PSS movies of the .hack games (`PSS/LOGO_B.PSS`, `LOGO_C.PSS`,
//! `LOGO_H.PSS` and `OPENING.PSS` on Infection's disc), which the title plays
//! through `ccDecodeMpeg` (DEMO.PRG 0x00409df0): [`decoder`], an MPEG-2 video
//! decoder of our own for what these streams use, bit for bit ffmpeg's
//! `mpeg2video` output on all four files; [`idct`]; [`csc`], the IPU's colour
//! conversion; and [`movie`], the player the runtime drives. The container is
//! `piney_data::pss`; `docs/formats/pss.md` has the formats and the player.

mod bits;
pub mod csc;
pub mod decoder;
pub mod idct;
pub mod movie;
mod vlc;

pub use decoder::{Decoder, Kind, Picture, PictureHeader, Sequence};
pub use movie::{Movie, MovieAudio};
