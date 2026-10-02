//! The player: what `ccDecodeMpeg` (DEMO.PRG 0x00409df0) shows and plays, for
//! the runtime to drive on its own frame clock (`docs/formats/pss.md`, "The
//! player"). Each picture fills the whole screen as one bilinear sprite
//! ([`draw`]), no letterbox or crop. Pacing is by vblank, not time stamps: a
//! picture lasts [`VBLANKS_PER_PICTURE`] frames, the display and audio
//! starting together once two pictures (and the audio) are ready. OK, cancel
//! or START skip once the decoder's `frameCount` is at least 11
//! ([`Movie::skippable`]).

use std::collections::VecDeque;

use piney_data::iso::Iso;
use piney_data::pss::{self, SoundHeader};
use piney_data::{Error, Result};
use piney_draw::{
    AlphaTest, Cmd, Depth, DrawState, Filter, Frame, Prim, PrimKind, Rgba, Scissor, TexFunc, TexRef, TexState, Upload,
    UploadFormat, Vertex, Wrap,
};

use crate::decoder::{Decoder, Picture, Sequence};

/// Frames of the game's 60 Hz clock each picture stays up (an even and an
/// odd field, `vblankHandler` demo.prg 0x0040ad40).
pub const VBLANKS_PER_PICTURE: u32 = 2;

/// `readMpeg` (demo.prg 0x00409fe0) takes a skip once `frameCount >= 11`.
pub const SKIP_FRAME_COUNT: u32 = 11;

/// Pictures the game's decoder keeps ready beyond the one shown: `voBuf`
/// holds two (`voBufCreate(voBuf, 2)` in `initAll`).
pub const AHEAD: usize = 2;

/// START in `ccPad`'s bits; the other two skip buttons are the save's
/// `assignPADok` and `assignPADcancel` (+0x840e, +0x8410).
pub const PAD_START: u32 = 0x800;

/// The Upload id [`draw`] gives the picture.
pub const UPLOAD_ID: u32 = 1;

/// The game's movie name to its disc path, as `strFileOpen` (demo.prg
/// 0x0040b560) builds it: `\PSS\` + the name upper-cased (`logo_b.pss` ->
/// `PSS/LOGO_B.PSS`).
pub fn disc_path(name: &str) -> String {
    format!("PSS/{}", name.to_ascii_uppercase())
}

/// A movie's sound as the SPU2 plays it: 16-bit PCM at `rate`, `channels`
/// interleaved (left, right, left, ...). It goes to core 0's sound data
/// input at full volume (`changeInputVolume(0x7fff)`: BVOLL = BVOLR =
/// 0x7fff) and on through the master volumes like any streamed sound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovieAudio {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
    /// The stream's `SShd` header as it is on the disc.
    pub header: SoundHeader,
}

impl MovieAudio {
    /// Sample frames (one sample of every channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// Sample frames `from..from + n`, interleaved, fewer at the end.
    pub fn chunk(&self, from: usize, n: usize) -> &[i16] {
        let ch = usize::from(self.channels.max(1));
        let a = (from * ch).min(self.samples.len());
        let b = ((from + n) * ch).min(self.samples.len());
        &self.samples[a..b]
    }
}

/// A PSS movie, decoded as it is played.
pub struct Movie {
    decoder: Decoder,
    sequence: Sequence,
    ahead: VecDeque<Picture>,
    ended: bool,
    shown: u32,
    audio: Option<MovieAudio>,
}

impl Movie {
    /// Open the movie at `path` on the disc (`PSS/OPENING.PSS`; see
    /// [`disc_path`] for the game's own names).
    pub fn open(iso: &mut Iso, path: &str) -> Result<Movie> {
        Movie::from_pss(&iso.read_path(path)?)
    }

    /// A movie from the bytes of a PSS file.
    pub fn from_pss(bytes: &[u8]) -> Result<Movie> {
        let d = pss::demux(bytes)?;
        let audio = match &d.audio {
            Some(a) if a.kind == 0xa0 => Some(MovieAudio {
                rate: a.header.rate as u32,
                channels: a.header.channels as u16,
                samples: a.pcm()?,
                header: a.header,
            }),
            Some(a) => return Err(Error::Format(format!("PSS audio kind {:02x} (only PCM)", a.kind))),
            None => None,
        };
        let mut decoder = Decoder::new(d.video);
        let sequence = decoder.read_sequence()?.clone();
        Ok(Movie { decoder, sequence, ahead: VecDeque::new(), ended: false, shown: 0, audio })
    }

    pub fn width(&self) -> usize {
        usize::from(self.sequence.width)
    }

    pub fn height(&self) -> usize {
        usize::from(self.sequence.height)
    }

    /// The frame rate the stream's sequence header gives (30 for every
    /// PSS here). The game does not use it: it shows a picture every
    /// [`VBLANKS_PER_PICTURE`] vblanks.
    pub fn frame_rate(&self) -> f64 {
        let (n, d) = self.sequence.frame_rate();
        f64::from(n) / f64::from(d.max(1))
    }

    pub fn sequence(&self) -> &Sequence {
        &self.sequence
    }

    /// Pictures the decoder has produced, in display order (the game's
    /// `frameCount` + 1).
    pub fn frames_decoded(&self) -> u32 {
        self.decoder.output()
    }

    /// Pictures handed out by [`Movie::next_picture`] / [`Movie::next_frame`].
    pub fn frames_shown(&self) -> u32 {
        self.shown
    }

    /// Whether a push of OK, cancel or START stops the movie now: the
    /// decoder, which runs [`AHEAD`] pictures ahead of the display, has
    /// produced the picture with index [`SKIP_FRAME_COUNT`].
    pub fn skippable(&self) -> bool {
        self.decoder.output() > SKIP_FRAME_COUNT
    }

    /// The next picture to show, in display order; None once the movie
    /// has ended.
    pub fn next_picture(&mut self) -> Result<Option<Picture>> {
        while !self.ended && self.ahead.len() <= AHEAD {
            match self.decoder.next_picture()? {
                Some(p) => self.ahead.push_back(p),
                None => self.ended = true,
            }
        }
        let p = self.ahead.pop_front();
        self.shown += p.is_some() as u32;
        Ok(p)
    }

    /// The next picture as the frame the runtime draws ([`draw`]).
    pub fn next_frame(&mut self) -> Result<Option<Frame>> {
        Ok(self.next_picture()?.map(|p| draw(&p)))
    }

    /// The movie's sound, if it has any (`OPENING.PSS`).
    pub fn audio(&self) -> Option<&MovieAudio> {
        self.audio.as_ref()
    }

    /// The sound, moved out (for handing to the audio engine whole).
    pub fn take_audio(&mut self) -> Option<MovieAudio> {
        self.audio.take()
    }
}

/// One picture as the game draws it (`setImageTag`, demo.prg 0x0040a940):
/// the IPU's RGBA32 image uploaded (PSMCT32), and one SPRITE over the whole
/// screen - TME, FST, no ABE; TEX0 DECAL with TCC 0; TEX1 bilinear
/// (MMAG = MMIN = 1); UV (0.5, 0.5) to (w + 0.5, h + 0.5); ZTST ALWAYS,
/// ZMSK - here from (0, 0) to (512, 448), the port's frame, in field mode
/// (`ccDecodeMpeg`'s `SetScreenMode(640, 448, 2)`). The game's
/// CLAMP register is left as it was (REPEAT over a 1024 x 1024 texture
/// whose rest is whatever VRAM holds); the edge texels are clamped here.
pub fn draw(picture: &Picture) -> Frame {
    let (w, h) = (picture.width as u16, picture.height as u16);
    let mut f = Frame::new();
    f.clear = Rgba::BLACK;
    f.field_mode = true;
    f.uploads.push(Upload {
        id: UPLOAD_ID,
        width: w,
        height: h,
        format: UploadFormat::Psmct32,
        pixels: picture.to_rgba(),
        clut: Vec::new(),
    });
    let state = DrawState {
        blend: None,
        alpha_test: AlphaTest::Off,
        depth: Depth::NONE,
        texture: Some(TexState {
            tex: TexRef::Upload(UPLOAD_ID),
            func: TexFunc::Decal,
            use_alpha: false,
            filter: Filter::Linear,
            wrap: Wrap::Clamp,
        }),
        scissor: Scissor::FULL,
    };
    let v = |x: f32, y: f32, u: f32, v: f32| Vertex { x, y, z: 0, u, v, rgba: Rgba::NEUTRAL };
    f.cmds.push(Cmd::Prim(Prim {
        kind: PrimKind::Sprite,
        gouraud: false,
        state,
        verts: vec![
            v(0.0, 0.0, 0.5, 0.5),
            v(f32::from(f.width), f32::from(f.height), f32::from(w) + 0.5, f32::from(h) + 0.5),
        ],
    }));
    f
}

#[cfg(test)]
mod tests {
    use super::disc_path;

    #[test]
    fn game_names_to_disc_paths() {
        assert_eq!(disc_path("logo_b.pss"), "PSS/LOGO_B.PSS");
        assert_eq!(disc_path("opening.pss"), "PSS/OPENING.PSS");
    }
}
