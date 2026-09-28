//! `ccEventStream(num, 1)` (0x001b5670): the event instruction `stream`, the
//! stream as the scripts play it - the call [`Stream`] models, the subtitles
//! under it ([`Subtitles`]) and the background colour 0 for the call (`ccSys
//! +0x18`, restored after). A host builds an `EventStream`, then each game
//! frame calls `step(&pad)` and `take_requests()` until `done()`: the PCM for
//! SEWORDS channel 0, [`Request::Music`] before the first frame and after the
//! last ([`EventStream::music_bits`]) and [`Request::StreamBgm`] at the notes,
//! which `piney_audio::stream` carries out on the area's bank.

use std::sync::Arc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_desktop::message::MsgDraw;
use piney_draw::Frame;
use piney_input::Pad;

use crate::effect::Rand;
use crate::subtitle::{Look, Subtitles};
use crate::{Options, Request, Stream};

/// The event instruction's stream, with its subtitles.
pub struct EventStream {
    stream: Stream,
    subtitles: Option<Subtitles>,
    /// The window's `Disp` calls on the last step.
    draws: Vec<MsgDraw>,
}

impl EventStream {
    /// Stream `num` as `ccEventStream(num, 1)` plays it for `save` (the
    /// voice language and cancel button in `opts`, the Movie Text option,
    /// Parody Mode and the player's names from the save). `data` is
    /// `DATA.BIN` (the fonts and the window's texture).
    pub fn new(
        iso: &mut Iso,
        data: &Archive,
        num: usize,
        save: &SaveData,
        opts: Options,
        rand: Rand,
    ) -> Result<EventStream> {
        let stream = Stream::with_rand(iso, num, opts, rand)?;
        let volume = iso.volume()?;
        let subtitles = match Subtitles::read(volume, num, save)? {
            Some(s) => Some(s.with_look(Look::read(volume, data)?)),
            None => None,
        };
        Ok(EventStream { stream, subtitles, draws: Vec::new() })
    }

    /// A stream with subtitles built by the caller (or none): for checks
    /// without fonts.
    pub fn with_subtitles(stream: Stream, subtitles: Option<Subtitles>) -> EventStream {
        EventStream { stream, subtitles, draws: Vec::new() }
    }

    /// One game frame: the stream's step, then the call's pass - the line
    /// `ccGetStreamDemoMsg` answers, the window's `Disp` - drawn on layer
    /// 242 over the stream. Once the stream has returned the window closes
    /// without being drawn, and the frame is the stream's (empty).
    pub fn step(&mut self, pad: &Pad) -> Frame {
        self.step_with(pad, &mut |_, _| {})
    }

    /// [`EventStream::step`] with another task drawing into the scene
    /// ([`Stream::step_with`]).
    pub fn step_with(
        &mut self,
        pad: &Pad,
        extra: &mut dyn FnMut(&mut piney_desktop::anm::Ctx, &crate::scene::Scene),
    ) -> Frame {
        self.draws.clear();
        if self.stream.done() {
            if let Some(s) = &mut self.subtitles {
                s.close();
            }
            return self.stream.step_with(pad, extra);
        }
        let mut f = self.stream.step_with(pad, extra);
        let msg = self.stream.demo_msg();
        if let Some(s) = &mut self.subtitles {
            self.draws = s.pass(msg);
            s.draw_into(&self.draws, &mut f);
        }
        f
    }

    /// The window's `Disp` calls on the last step (none without a table).
    pub fn window_draws(&self) -> &[MsgDraw] {
        &self.draws
    }

    /// The call has returned.
    pub fn done(&self) -> bool {
        self.stream.done()
    }

    /// What the steps since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        self.stream.take_requests()
    }

    /// `sd->size` of the stream's header: the music bits `ccSndStreamCtrl`
    /// reads (bit 0 before, bit 1 after).
    pub fn music_bits(&self) -> i32 {
        self.stream.def().header.size
    }

    /// The stream's own files, which its frames name (look them up before
    /// `DATA.BIN`).
    pub fn archive(&self) -> Arc<Archive> {
        self.stream.archive()
    }

    pub fn stream(&self) -> &Stream {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut Stream {
        &mut self.stream
    }

    pub fn subtitles(&self) -> Option<&Subtitles> {
        self.subtitles.as_ref()
    }
}
