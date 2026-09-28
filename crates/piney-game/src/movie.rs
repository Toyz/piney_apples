//! A PSS movie on the screen as the game plays one (`ccDecodeMpeg`): the
//! title's logos and opening, and the launcher's logos.

use std::path::Path;

use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_draw::Frame;
use piney_input::{Buttons, Pad};
use piney_mpeg::movie::{Movie, VBLANKS_PER_PICTURE};

use crate::mode::Event;

/// A PSS movie as `ccDecodeMpeg` plays it: each picture shown for two
/// frames, the sound started with the first; OK, cancel or START stops it
/// once twelve pictures are decoded.
pub struct Playing {
    movie: Movie,
    /// The picture on screen, and the frames it has been up.
    frame: Frame,
    shown: u32,
}

/// How a frame of a movie went.
pub enum Played {
    Showing(Frame),
    Ended,
    Skipped,
}

impl Playing {
    pub fn open(iso: &Path, path: &str, audio: bool, events: &mut Vec<Event>) -> Result<Playing, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let mut movie = Movie::open(&mut disc, path).map_err(|e| format!("{path}: {e}"))?;
        let first = movie.next_frame().map_err(|e| format!("{path}: {e}"))?.unwrap_or_else(Frame::new);
        if let Some(a) = movie.take_audio().filter(|_| audio) {
            events.push(Event::MovieAudio(a.samples));
        }
        Ok(Playing { movie, frame: first, shown: 0 })
    }

    /// A frame, OK and cancel as the save assigns them.
    pub fn step(&mut self, pad: &Pad, save: &SaveData) -> Played {
        let stop = u32::from(save.assign_pad_ok()) | u32::from(save.assign_pad_cancel()) | Buttons::START.bits();
        self.step_stopped_by(pad, stop)
    }

    /// A frame, the buttons in `stop` stopping it.
    pub fn step_stopped_by(&mut self, pad: &Pad, stop: u32) -> Played {
        if pad.push.bits() & stop != 0 && self.movie.skippable() {
            return Played::Skipped;
        }
        if self.shown == VBLANKS_PER_PICTURE {
            self.shown = 0;
            match self.movie.next_frame() {
                Ok(Some(f)) => self.frame = f,
                Ok(None) => return Played::Ended,
                Err(e) => {
                    eprintln!("movie: {e}");
                    return Played::Ended;
                }
            }
        }
        self.shown += 1;
        Played::Showing(self.frame.clone())
    }
}
