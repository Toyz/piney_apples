//! An in-engine stream (`ccRequestLoadStream`) as its callers play it (the
//! title's intro, the scripts' `stream`, the Audio screen's movies): the caller
//! holds its work while the stream steps a game frame at a time, and takes it
//! up again at [`done`](StreamPlayer::step). The stream owns SEWORDS channel 0 from
//! `ccPcmSound::Open` to `Close`. The scripts' `stream` ([`StreamPlayer::event`],
//! `ccEventStream(num, 1)`) also shows subtitles and changes the music around
//! it (`ccSndStreamCtrl`, `ccSndStreamBGM`); docs/engine/stream.md.

use std::path::Path;
use std::sync::Arc;

use piney_audio::stream::{StrBgm, StreamGame};
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_desktop::save::SaveState;
use piney_draw::Frame;
use piney_input::{Buttons, Pad};
use piney_stream::event::EventStream;
use piney_stream::file::StreamFile;
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
        parody: save.parody(),
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
/// The game's one `rand()` as the mode starting a stream has it.
fn rand_of(state: &SaveState) -> piney_stream::effect::Rand {
    piney_stream::effect::Rand { next: state.rand }
}

fn stream_effects(disc: &mut Iso, data: &Archive) -> Option<piney_effect::StreamEffects> {
    let made = disc.volume().and_then(|v| piney_effect::StreamEffects::new(data, v));
    made.map_err(|e| tracing::warn!("the stream's effects: {e}")).ok()
}

/// The file list of the place a stream plays in, for the streams' common
/// files it keeps in memory: a scene's `#` objects resolve against them as against its own
/// preloads (`CompleteIndexChunkAdrs`). `strcmnFileList` is `STR8000E.CCS`,
/// `datadrainFileList` `STR8001E.CCS`, `happyakuyujunFileList`
/// `STR8800E.CCS` (`DATA.BIN`'s `str8000e`, `str8001e`, `str8800e`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileList {
    /// `ccSetFileListTown`: strcmn and happyakuyujun.
    Town,
    /// `ccSetFileListField` and `ccSetFileListDungeon`: strcmn and datadrain.
    Field,
    /// `ccSetFileListDesktop`: strcmn, datadrain and happyakuyujun.
    Desktop,
}

impl FileList {
    /// The files, oldest first, as the list loads them.
    fn resident(self) -> &'static [&'static str] {
        match self {
            FileList::Town => &["str8000e", "str8800e"],
            FileList::Field => &["str8000e", "str8001e"],
            FileList::Desktop => &["str8000e", "str8001e", "str8800e"],
        }
    }

    fn files(self, data: &Archive) -> piney_data::Result<Vec<StreamFile>> {
        self.resident().iter().map(|stem| StreamFile::read(data, stem)).collect()
    }
}

impl StreamPlayer {
    /// The game's `rand()` as the stream has left it, for the mode it
    /// played in to go on from.
    pub fn rand(&self) -> u64 {
        self.stream.stream().rand().next
    }

    /// Stream `num` from the disc at `iso`, set up from `save` (the voice
    /// language and the cancel button). `title_after_desktop`: the title's
    /// stream once the desktop has run (`DESKTOP_FLG`), which cancel always
    /// skips. No subtitles, no music: the title's stream (0 or 1, which
    /// name nothing outside their own files and start no effect).
    pub fn start(
        iso: &Path,
        num: usize,
        save: &SaveState,
        title_after_desktop: bool,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let opts = options(&save.save, title_after_desktop);
        let stream =
            Stream::with_rand(&mut disc, num, opts, rand_of(save)).map_err(|e| format!("stream {num}: {e}"))?;
        // `ccPcmSound::Open`: the channel is the stream's.
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream: EventStream::with_subtitles(stream, None), num, music: None })
    }

    /// `ccThExecuteStream(num)` over `list`'s files with the stream demo's
    /// effects (`RequestStrPlay` starts `ccThEffectStr` for every stream):
    /// no subtitles, no music.
    fn over(
        iso: &Path,
        data: &Archive,
        num: usize,
        state: &SaveState,
        list: FileList,
        opts: Options,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let err = |e: piney_data::Error| format!("stream {num}: {e}");
        let resident = list.files(data).map_err(err)?;
        let mut stream = Stream::with_resident(&mut disc, num, opts, rand_of(state), resident).map_err(err)?;
        if let Some(fx) = stream_effects(&mut disc, data) {
            stream.set_effects(fx);
        }
        // `ccPcmSound::Open`: the channel is the stream's.
        events.extend([Event::VoiceStop, Event::MovieAudioStop]);
        Ok(StreamPlayer { stream: EventStream::with_subtitles(stream, None), num, music: None })
    }

    /// A menu's movie (`ccThExecuteStream`): Data Drain's from
    /// `DataDrainMenu`'s step 0 and `StreamMenu`'s in a field or dungeon,
    /// a Ryu Book's cover in a town. The drain streams' models are
    /// `str8000e`'s and `str8001e`'s, the book's `str8800e`'s.
    pub fn drain(
        iso: &Path,
        data: &Archive,
        num: usize,
        state: &SaveState,
        list: FileList,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let opts = Options { skill_names: skill_names(data), ..options(&state.save, false) };
        StreamPlayer::over(iso, data, num, state, list, opts, events)
    }

    /// The Audio screen's movie (`SimplePlayStream`, desktop.prg
    /// 0x00407100): `ccThExecuteStream(num)` over the desktop's files, with
    /// its effects; no subtitles, no music. `game.status` is the desktop's
    /// 2, so `Func_str9000` (stream 20) makes no banner.
    pub fn movie(
        iso: &Path,
        data: &Archive,
        num: usize,
        state: &SaveState,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        StreamPlayer::over(iso, data, num, state, FileList::Desktop, options(&state.save, false), events)
    }

    /// `ccEventStream(num, 1)`, the event instruction `stream`: stream `num`
    /// with its subtitles (`evStrMsgTbl[num]`, shown as the save's Movie
    /// Text option says; drawn with `data`, `DATA.BIN`'s fonts and window,
    /// or not drawn without it) and the music around it on the loaded bank,
    /// `game` being what `ccSndStreamCtrl` reads (`game.status`, and the
    /// story area in `game.field`), over `list`'s files.
    #[allow(clippy::too_many_arguments)]
    pub fn event(
        iso: &Path,
        data: Option<&Archive>,
        num: usize,
        state: &SaveState,
        game: StreamGame,
        list: FileList,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let save = &state.save;
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let err = |e: piney_data::Error| format!("stream {num}: {e}");

        let opts = Options { skill_names: data.and_then(skill_names), ..options(save, false) };
        let mut stream = match data {
            Some(data) => list.files(data).and_then(|resident| {
                let s = Stream::with_resident(&mut disc, num, opts, rand_of(state), resident)?;
                EventStream::over(&mut disc, data, s, save)
            }),
            None => Stream::with_rand(&mut disc, num, opts, rand_of(state))
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
        state: &SaveState,
        game: StreamGame,
        events: &mut Vec<Event>,
    ) -> Result<StreamPlayer, String> {
        let save = &state.save;
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let volume = disc.volume().map_err(|e| format!("{}: {e}", iso.display()))?;
        let num = piney_stream::table::gate_stream(volume);
        let err = |e: piney_data::Error| format!("stream {num} (the gate hack): {e}");
        let opts = options(save, false);
        let stream = Stream::gate_hack(&mut disc, town, field, crisis, opts, rand_of(state)).map_err(err)?;
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

    /// `ccGetStreamFrame()`: the scene frame the last step drew, and
    /// whether any step has run.
    pub fn frame(&self) -> (u32, bool) {
        let s = self.stream.stream();
        (s.frame(), s.steps() > 0)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use piney_data::volume::Volume;
    use piney_stream::Options;

    use super::*;

    /// The `#` objects of stream `num`'s files that neither another of its
    /// files nor `list`'s define, by name.
    fn unresolved(disc: &mut Iso, data: &Archive, num: usize, list: FileList) -> Vec<String> {
        let s = Stream::with_options(disc, num, Options::default()).unwrap();
        let mut own: Vec<StreamFile> = Vec::new();
        for e in &s.def().entries {
            if !own.iter().any(|f| f.stem == e.name) {
                own.push(StreamFile::read(&s.archive(), &e.name).unwrap());
            }
        }
        let resident = list.files(data).unwrap();
        let defines = |g: &StreamFile, name: &str| g.sf.ccs.find_object(name).is_some_and(|o| !g.external(o));
        let mut out = Vec::new();
        for f in &own {
            for obj in (1..f.sf.ccs.objects.len() as u32).filter(|&o| f.external(o)) {
                let name = f.name(obj).unwrap_or_default();
                let mut others = own.iter().filter(|g| g.stem != f.stem).chain(&resident);
                if !others.any(|g| defines(g, name)) {
                    out.push(format!("{}:{name}", f.stem));
                }
            }
        }
        out
    }

    /// Every `#` object of the streams a place plays is defined there: the
    /// Audio screen's movies over the desktop's list (#54: Movie 16, Kite's
    /// drain of Skeith, drew Skeith alone), the drains and `StreamMenu`'s
    /// over a field's, a Ryu Book's cover (`str8801`-`str8808`, its
    /// backdrop `str8800e`'s) over a town's.
    #[test]
    fn every_hash_object_resolves_where_its_stream_plays() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let data = Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let movies = piney_desktop::content::streams(Volume::Inf).into_iter().enumerate();
        let on_disc = movies.filter(|(i, _)| piney_desktop::content::stream_volume(*i) == 1);
        let cases: Vec<(FileList, usize)> = on_disc
            .map(|(_, m)| (FileList::Desktop, m.str_num as usize))
            .chain([18, 20, 109, 110, 111].map(|n| (FileList::Field, n)))
            .chain((112..120).map(|n| (FileList::Town, n)))
            .collect();
        assert_eq!(cases.len(), 18 + 5 + 8);
        for &(list, num) in &cases {
            let missing = unresolved(&mut disc, &data, num, list);
            assert!(missing.is_empty(), "stream {num} over {list:?}: {missing:?}");
        }
        // Without the lists: the drain of Skeith and the book's covers.
        assert!(!unresolved(&mut disc, &data, 18, FileList::Town).is_empty());
        assert!(!unresolved(&mut disc, &data, 112, FileList::Field).is_empty());
    }
}
