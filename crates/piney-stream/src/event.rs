//! `ccEventStream(num, 1)` (0x001b5670): the event instruction `stream`, the
//! stream as the scripts play it - the call [`Stream`] models, the subtitles
//! under it ([`Subtitles`]), the effect task's text lines (Mutation's opening,
//! [`crate::opening`]) and the background colour 0 for the call. A host
//! builds an `EventStream`, then each game frame calls `step(&pad)` and
//! `take_requests()` until `done()`: the PCM, [`Request::Music`] before the
//! first frame and after the last ([`EventStream::music_bits`]) and
//! [`Request::StreamBgm`] at the notes.

use std::sync::Arc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::read_fonts;
use piney_desktop::kanji::{Fonts, Kanji, Names};
use piney_desktop::message::MsgDraw;
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_input::Pad;

use crate::effect::{Rand, task_name};
use crate::opening;
use crate::subtitle::{Look, Subtitles};
use crate::{Options, Request, Stream};

/// The event instruction's stream, with its subtitles.
pub struct EventStream {
    stream: Stream,
    subtitles: Option<Subtitles>,
    /// The window's `Disp` calls on the last step.
    draws: Vec<MsgDraw>,
    /// The fonts the opening's text lines draw with; None for a stream
    /// without `str0710`.
    text_fonts: Option<Fonts>,
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
        EventStream::over(iso, data, stream, save)
    }

    /// [`EventStream::new`] on a stream the caller made (one over the
    /// place's resident files, [`Stream::with_resident`]).
    pub fn over(iso: &mut Iso, data: &Archive, stream: Stream, save: &SaveData) -> Result<EventStream> {
        let num = stream.def().num;
        let volume = iso.volume()?;
        let subtitles = match Subtitles::read(volume, num, save)? {
            Some(s) => Some(s.with_look(Look::read(volume, data)?)),
            None => None,
        };
        let opening = stream.scenes().iter().any(|e| task_name(&e.name) == opening::STR0710);
        let text_fonts = if opening { Some(read_fonts(volume, data)?) } else { None };
        Ok(EventStream { stream, subtitles, draws: Vec::new(), text_fonts })
    }

    /// A stream with subtitles built by the caller (or none): for checks
    /// without fonts.
    pub fn with_subtitles(stream: Stream, subtitles: Option<Subtitles>) -> EventStream {
        EventStream { stream, subtitles, draws: Vec::new(), text_fonts: None }
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
        self.draw_text(&mut f);
        f
    }

    /// The opening's text lines (`ccKanji::Disp` on layer 1000, over the
    /// window's 242).
    fn draw_text(&self, f: &mut Frame) {
        let (Some(fonts), Some(task), Some(scene)) = (&self.text_fonts, self.stream.effect(), self.stream.scene())
        else {
            return;
        };
        let lines = task.text_draws();
        if lines.is_empty() {
            return;
        }
        let mut ctx = Ctx::new(View::default());
        let view = scene.frame.layer();
        let [r, g, b, _] = piney_data::tables::kanji::SPRITE_COLOR_TABLE[opening::TEXT_COLOUR];
        for l in lines {
            let mut k = Kanji::init(opening::KANJI_L, opening::KANJI_PACKETS);
            k.colour = [r, g, b, l.alpha];
            k.dx = l.x as f32;
            k.dy = l.y as f32;
            ctx.disp(fonts, &mut k, opening::TEXT_LAYER, &view, &l.text, &Names::default());
        }
        let t = ctx.finish();
        f.uploads.extend(t.uploads);
        f.cmds.extend(t.cmds);
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
