//! An in-engine stream (`ccRequestLoadStream`) as its callers play it: the
//! title's intro, the event scripts' `stream`, the desktop Audio screen's
//! movies. The caller holds its own work while the stream plays, one step a
//! game frame at its own frame rate, and takes it up again at [`done`].
//!
//! The stream owns SEWORDS channel 0 from `ccPcmSound::Open` to `Close`:
//! voice and `BGM.BIN` are stopped as it starts, and its sound is queued
//! there until it ends.
//!
//! The event scripts' `stream` ([`StreamPlayer::event`], `ccEventStream(num,
//! 1)`) also shows the stream's subtitles over its frames and changes the
//! area's music around it: [`Event::StreamMusic`] before the first frame
//! and after the last (`ccSndStreamCtrl`), [`Event::StreamBgm`] at its notes
//! (`ccSndStreamBGM`). The title's stream and the Audio screen's movies
//! ([`StreamPlayer::start`]) have neither: no table, and the movie player
//! holds the music (`ccSndMoviePlayer`, `ccSnd +0x62`).
//!
//! Hosting a stream anywhere - the desktop, the town, a field or dungeon -
//! is the same calls:
//!
//! ```text
//! // the event instruction `stream num` (the host's Host::stream):
//! let p = StreamPlayer::event(&iso, Some(&data_bin), num, &save,
//!                             StreamGame { status: game.status, field: game.field }, &mut events)?;
//! // each game frame while the instruction waits (Host::busy(Wait::Stream)):
//! match p.step(&pad, &mut events) {
//!     Some(frame) => show frame instead of the host's own (the stream's picture,
//!                    the subtitle window over it); give the renderer p.archive()
//!                    as the overlay archive,
//!     None => the call has returned: drop p, the instruction goes on,
//! }
//! ```
//!
//! `events` carries the PCM (`Event::StreamPcm`, `MovieAudioStop`), the
//! music (`StreamMusic`, `StreamBgm`) and the voice stop; the host passes
//! them on as it does its own.
//!
//! [`done`]: StreamPlayer::step

use std::path::Path;
use std::sync::Arc;

use piney_audio::stream::{StrBgm, StreamGame};
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_draw::Frame;
use piney_input::{Buttons, Pad};
use piney_stream::event::EventStream;
use piney_stream::subtitle::Subtitles;
use piney_stream::{Options, Request, SkillNames, Stream};

use crate::mode::Event;

pub struct StreamPlayer {
    stream: EventStream,
    num: usize,
    /// The event instruction's stream: the game `ccSndStreamCtrl` reads.
    /// None for the title's and the Audio screen's.
    music: Option<StreamGame>,
}

fn options(save: &SaveData, title_after_desktop: bool) -> Options {
    Options {
        english: crate::mode::voice_english(save),
        cancel: Buttons(u32::from(save.assign_pad_cancel())),
        title_after_desktop,
        skill_names: None,
    }
}

/// `bossSkillNameTbl[0]`'s texture in `DATA.BIN` (`xeffect`'s
/// `TEX_detadrain`): the banner the member-drained stream shows while the
/// game runs.
fn skill_names(data: &Archive) -> Option<SkillNames> {
    let w = piney_desktop::message::WindowTexture::read_named(data, SkillNames::FILE, SkillNames::TEXTURE, None)?;
    let piney_draw::TexRef::Ccs { texture, clut, .. } = w.tex else { return None };
    Some(SkillNames { texture, clut, tex_h: w.tex_h })
}

/// The stream demo's effects (`ccThEffectStr`'s `effcStr` and the second
/// particle system, [`piney_effect::StreamEffects`]), which the streams'
/// effect tasks start their hit marks and transfers in, on the volume's
/// tables. None, said, when they cannot be read.
fn stream_effects(disc: &mut Iso, data: &Archive) -> Option<piney_effect::StreamEffects> {
    let made = disc.volume().and_then(|v| piney_effect::StreamEffects::new(data, v));
    made.map_err(|e| eprintln!("the stream's effects: {e}")).ok()
}

impl StreamPlayer {
    /// Stream `num` from the disc at `iso`, set up from `save` (the voice
    /// language and the cancel button). `title_after_desktop`: the title's
    /// stream once the desktop has run (`DESKTOP_FLG`), which cancel always
    /// skips. No subtitles, no music: the title's stream and the Audio
    /// screen's movies.
    pub fn start(
        iso: &Path,
        num: usize,
        save: &SaveData,
        title_after_desktop: bool,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let opts = options(save, title_after_desktop);
        let stream = Stream::with_options(&mut disc, num, opts).map_err(|e| format!("stream {num}: {e}"))?;
        // `ccPcmSound::Open`: the channel is the stream's.
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream: EventStream::with_subtitles(stream, None), num, music: None })
    }

    /// Data Drain's movie (`ccThExecuteStream` from `DataDrainMenu`'s step
    /// 0): stream `num` as [`StreamPlayer::start`] plays one, over the
    /// files a field or dungeon holds (`strcmnFileList`'s and
    /// `datadrainFileList`'s `STR8000E.CCS` and `STR8001E.CCS`, `DATA.BIN`'s
    /// `str8000e` and `str8001e`), which its models are.
    pub fn drain(
        iso: &Path,
        data: &Archive,
        num: usize,
        save: &SaveData,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let err = |e: piney_data::Error| format!("stream {num}: {e}");

        let resident = ["str8000e", "str8001e"]
            .iter()
            .map(|stem| piney_stream::file::StreamFile::read(data, stem))
            .collect::<piney_data::Result<Vec<_>>>()
            .map_err(err)?;
        let opts = Options { skill_names: skill_names(data), ..options(save, false) };
        let mut stream = Stream::with_resident(&mut disc, num, opts, Default::default(), resident).map_err(err)?;
        if let Some(fx) = stream_effects(&mut disc, data) {
            stream.set_effects(fx);
        }
        // `ccPcmSound::Open`: the channel is the stream's.
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream: EventStream::with_subtitles(stream, None), num, music: None })
    }

    /// `ccEventStream(num, 1)`, the event instruction `stream`: stream `num`
    /// with its subtitles (`evStrMsgTbl[num]`, shown as the save's Movie
    /// Text option says; drawn with `data`, `DATA.BIN`'s fonts and window,
    /// or not drawn without it) and the music around it on the loaded bank,
    /// `game` being what `ccSndStreamCtrl` reads (`game.status`, and the
    /// story area in `game.field`).
    pub fn event(
        iso: &Path,
        data: Option<&Archive>,
        num: usize,
        save: &SaveData,
        game: StreamGame,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let err = |e: piney_data::Error| format!("stream {num}: {e}");

        let opts = Options { skill_names: data.and_then(skill_names), ..options(save, false) };
        let mut stream = match data {
            Some(data) => EventStream::new(&mut disc, data, num, save, opts, Default::default()),
            None => Stream::with_rand(&mut disc, num, opts, Default::default())
                .and_then(|s| Ok(EventStream::with_subtitles(s, Subtitles::read(disc.volume()?, num, save)?))),
        }
        .map_err(err)?;
        if let Some(fx) = data.and_then(|d| stream_effects(&mut disc, d)) {
            stream.stream_mut().set_effects(fx);
        }
        // `ccPcmSound::Open`: the channel is the stream's.
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream, num, music: Some(game) })
    }

    /// `ccRequestLoadStreamGateHack(107, town, field)` (0x00199f00; 113 from
    /// Mutation on), as `ccSetupGameCtrl` plays it after a gate hack
    /// (`setupMode` 1): the Chaos Gate's movie from `town` to `field`
    /// ([`piney_stream::Stream::gate_hack`]), with the gate stream's music
    /// around it; no subtitles. `crisis`: `saveData` +0x6772.
    pub fn gate_hack(
        iso: &Path,
        town: i32,
        field: i32,
        crisis: bool,
        save: &SaveData,
        game: StreamGame,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let volume = disc.volume().map_err(|e| format!("{}: {e}", iso.display()))?;
        let num = piney_stream::table::gate_stream(volume);
        let err = |e: piney_data::Error| format!("stream {num} (the gate hack): {e}");
        let opts = options(save, false);
        let stream = Stream::gate_hack(&mut disc, town, field, crisis, opts, Default::default()).map_err(err)?;
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream: EventStream::with_subtitles(stream, None), num, music: Some(game) })
    }

    /// Not the game's: `entries` of the archive at `path` on the disc, which
    /// no stream table lists ([`piney_stream::load::scan`]), played as the
    /// title's stream is: no subtitles, no music, English voices.
    pub fn loose(
        iso: &Path,
        path: &str,
        entries: Vec<piney_stream::table::Entry>,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let opts = Options { english: true, ..Options::default() };
        let stream = Stream::loose(&mut disc, path, entries, opts).map_err(|e| format!("{path}: {e}"))?;
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream: EventStream::with_subtitles(stream, None), num: 0, music: None })
    }

    /// The scene playing is the one the reader paused before (`CheckPause`:
    /// the Chaos Gate's loop), and how far into it (its frame, and its
    /// last).
    pub fn at_pause(&self) -> Option<(u32, u32)> {
        let s = self.stream.stream();
        if !s.at_pause() {
            return None;
        }
        s.scene().map(|sc| (sc.frame_now, sc.frame_end))
    }

    /// The name of the scene playing (or next).
    #[allow(dead_code)]
    pub fn entry_name(&self) -> Option<String> {
        self.stream.stream().entry().map(|e| e.name.clone())
    }

    /// `ResetPause` with `WaitEnd` waiting on it: the paused scene ends as
    /// a skip ends it, and the stream goes on.
    pub fn reset_pause(&mut self) {
        self.stream.stream_mut().exit();
    }

    /// One game frame: the frame to show, or None once the call has
    /// returned (the channel closed).
    pub fn step(&mut self, pad: &Pad, events: &mut Vec<Event>) -> Option<Frame> {
        self.step_with(pad, events, &mut |_, _| {})
    }

    /// [`StreamPlayer::step`] with another task drawing into the scene
    /// (`piney_stream::Stream::step_with`).
    pub fn step_with(
        &mut self,
        pad: &Pad,
        events: &mut Vec<Event>,
        extra: &mut dyn FnMut(&mut piney_desktop::anm::Ctx, &piney_stream::scene::Scene),
    ) -> Option<Frame> {
        if self.stream.done() {
            events.push(Event::MovieAudioStop);
            return None;
        }
        let frame = self.stream.step_with(pad, extra);
        for r in self.stream.take_requests() {
            match r {
                Request::Pcm(pcm) => events.push(Event::StreamPcm(pcm)),
                // The samples play as they are queued; the last ones drain
                // before the channel closes at the end.
                Request::PcmStart | Request::PcmEnd => {}
                Request::PcmStop => events.push(Event::MovieAudioStop),
                Request::Music { num, after } => {
                    if let Some(game) = self.music {
                        events.push(Event::StreamMusic { num, size: self.stream.music_bits(), after, game });
                    }
                }
                Request::StreamBgm { record: Some(r), .. } if self.music.is_some() => {
                    events.push(Event::StreamBgm(StrBgm { sq: r.sq, sq2: r.sq2, time: r.time, vol: r.vol, cmd: r.cmd }))
                }
                // Reports of what the stream drew itself: the subtitles, the
                // effect tasks with their hit marks and transfers (the
                // stream demo's effects, `stream_effects`), and the near
                // clip, which its scene takes.
                Request::StreamBgm { .. }
                | Request::Message(_)
                | Request::Effect { .. }
                | Request::HitMark { .. }
                | Request::Transfer { .. }
                | Request::DivZ(_) => {}
            }
        }
        Some(frame)
    }

    /// The stream under the player (the launcher holds its last frame).
    pub fn stream_mut(&mut self) -> &mut piney_stream::Stream {
        self.stream.stream_mut()
    }

    /// The stream's own files, which its frames name.
    pub fn archive(&self) -> Arc<Archive> {
        self.stream.archive()
    }

    /// Which stream, and its frame (`ccGetStreamFrame`).
    pub fn status(&self) -> String {
        format!("stream {} frame {}", self.num, self.stream.stream().frame())
    }
}
