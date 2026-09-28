//! The in-engine streams of .hack//Infection: real-time cutscenes played from
//! the `STREAM/*.BIN` archives (`docs/engine/stream.md`). `ccRequestLoadStream`
//! (0x00198da0) plays stream `num` to its end inside the call, for the title's
//! intro, the event scripts' `stream` and the Audio screen's movies.
//! [`Stream`] is that call as a state machine: [`Stream::step`] is one game
//! frame, [`Stream::done`] says the call has returned; the files are read when
//! it is built. A scene's own effect task ([`effect`]) runs inside the step.
//! [`event::EventStream`] is `ccEventStream(num, 1)`, with the subtitles.

pub mod draw;
pub mod effect;
pub mod ending;
pub mod event;
pub mod file;
pub mod load;
pub mod scene;
pub mod subtitle;
pub mod table;

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::{Error, Result};
use piney_desktop::anm::Ctx;
use piney_desktop::view::View;
use piney_draw::{Frame, Rgba};
use piney_input::{Buttons, Pad};

use crate::effect::{Cue, Rand, Task};
use crate::file::StreamFile;
use crate::load::Member;
use crate::scene::{Loaded, Scene};
use crate::table::{BgmCursor, Def, Entry};

/// `sysLayer`'s priority: where a scene without a Layer chunk draws.
pub const SYS_LAYER: i16 = 0;

/// `PlaySceneMain`'s `Breath(2)` after a scene's last frame.
pub const END_BREATHS: u32 = 2;

/// Frames after a skip before the call returns (the scene thread leaves at
/// its next wake without drawing).
pub const SKIP_BREATHS: u32 = 1;

/// PCM: SEWORDS channel 0's rate and the samples in one block (256 left,
/// then 256 right, 16-bit).
pub const PCM_RATE: u32 = 48_000;
pub const PCM_BLOCK_SAMPLES: usize = 256;

/// Who plays the stream and how the save is set: what `WaitEnd` and the
/// file choice read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// `saveData.voice` (+0x842c): `streamTblE` and the `E` archives.
    pub english: bool,
    /// `saveData.assignPADcancel` (+0x8410): circle by default.
    pub cancel: Buttons,
    /// The title's stream after the desktop has run once (`game.mode` 2 and
    /// `DESKTOP_FLG` 1): cancel skips whatever the file's flags say.
    pub title_after_desktop: bool,
    /// `bossSkillNameTbl[0]`'s texture, for the member-drained stream's
    /// banner (`Func_str9000`): give it when the game is running (it makes
    /// no banner while `game` is missing or its status is 2 or 7).
    pub skill_names: Option<SkillNames>,
}

impl Default for Options {
    fn default() -> Self {
        Options { english: false, cancel: Buttons::CIRCLE, title_after_desktop: false, skill_names: None }
    }
}

/// `DATA.BIN`'s `xeffect`, `TEX_detadrain` (`bossSkillNameTbl[0]`): the
/// texture and palette objects and the texture's height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillNames {
    pub texture: u32,
    pub clut: u32,
    pub tex_h: i32,
}

impl SkillNames {
    /// The member of `DATA.BIN` it is in.
    pub const FILE: &'static str = "xeffect";
    /// Its texture object's name.
    pub const TEXTURE: &'static str = "TEX_detadrain";
}

/// What a frame asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// `ccSndStreamCtrl(num, sd, when)`: the sequenced music around a
    /// stream (header `size` bit 0 before, bit 1 after); each volume's
    /// switch picks the streams it acts on (`piney_audio::stream`).
    Music { num: usize, after: bool },
    /// Samples for SEWORDS channel 0: interleaved left, right, 16-bit, at
    /// [`PCM_RATE`]; queued in order, played from [`Request::PcmStart`].
    Pcm(Vec<i16>),
    /// `ccPcmSound::StartPlay` (`PlaySceneMain`): play the queued samples.
    PcmStart,
    /// The scene's last samples are queued; let them play out, then close
    /// the channel (`ccPcmSound::Close` when the call returns).
    PcmEnd,
    /// `EndScene` -> `ccPcmSound::EndPlay`: stop now (a skip, or the next
    /// scene taking over).
    PcmStop,
    /// A note's event 4: `ccSndStreamBGM(param)` when `strSndTbl[num]` has
    /// a BGM table (streams 3, 8 and 58 on this disc). `record` is the
    /// table's record the call carries out, None when its cursor passed
    /// over one or stays at the end ([`table::BgmCursor`]).
    StreamBgm { num: usize, param: u32, record: Option<table::StrBgm> },
    /// Notes 0x8001 / 0x8011, and a skip: `eventMsg`, the line of
    /// `evStrMsgTbl[num]` `ccEventStream` shows under the stream (None:
    /// close it). A skip's close comes with the next step, as the event
    /// task sees it (WaitEnd runs after it); [`Stream::demo_msg`] answers
    /// as `ccGetStreamDemoMsg` does.
    Message(Option<u32>),
    /// Note 0x8010: a cue for the stream's own effect task
    /// (`StreamDemoFuncTbl`). The ported tasks ([`effect::Task`]) draw into
    /// the stream's frames themselves; the other scenes' are not ported.
    Effect { scene: String, param: u32 },
    /// A hit mark `Func_str0120` or `Func_str0240` started (`effHitMarkStr`,
    /// streams 5 and 8): at
    /// the note's object `obj` (its world position `pos`), turned by `rot`
    /// (radians); f32 bits. The mark itself, a `ccEffect`, is not drawn by
    /// the port.
    HitMark { obj: u32, pos: [u32; 3], rot: [u32; 3] },
    /// A transfer an effect task started (`effTransferStr`; streams 7, 12,
    /// 14, 15 and 16): the
    /// arrival's effect at the note's object `obj` (its world position
    /// `pos`) for a character `height` high; f32 bits. The stream draws it
    /// itself when it has the effects ([`Stream::set_effects`]).
    Transfer { obj: u32, pos: [u32; 3], height: u32 },
    /// `Func_str0120`'s cue 12: the scene view's `divZ`, where VU1 clips,
    /// is now this (f32 bits: 500; `ccSetStreamDemoThread` set 1000). The
    /// port's draw does not clip by it.
    DivZ(u32),
}

/// 1.0 as f32 bits: a position's w.
const ONE_BITS: u32 = 0x3f80_0000;

/// What the stream demo's effects ask of the game: the stream's `rand`
/// (the C library's, the effect task's too) and the scene's camera; no
/// characters, no field.
struct FxHost<'a> {
    rand: &'a mut Rand,
    camera: piney_effect::draw::Camera,
}

impl piney_effect::Host for FxHost<'_> {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn player_pos(&self) -> [u32; 4] {
        [0, 0, 0, ONE_BITS]
    }
    fn camera(&self) -> piney_effect::draw::Camera {
        self.camera
    }
    fn char_pos(&self, _c: piney_effect::CharRef) -> [u32; 4] {
        [0, 0, 0, ONE_BITS]
    }
    fn char_dirc(&self, _c: piney_effect::CharRef) -> [u32; 4] {
        [0; 4]
    }
    fn char_height(&self, _c: piney_effect::CharRef) -> u32 {
        0
    }
    fn char_width(&self, _c: piney_effect::CharRef) -> u32 {
        0
    }
}

/// The effects' camera for `scene`: its view's world-to-view and
/// world-to-screen matrices (GS coordinates, as `ccCam` sets them), the
/// eye; the view's default w range and draw environment.
fn fx_camera(scene: &Scene) -> piney_effect::draw::Camera {
    let view = draw::scene_view(scene);
    let bits = |m: glam::Mat4| m.to_cols_array_2d().map(|c| c.map(f32::to_bits));
    let eye = view.camera.pos;
    let eye = [eye.x.to_bits(), eye.y.to_bits(), eye.z.to_bits(), ONE_BITS];
    piney_effect::draw::Camera {
        eye,
        cam_pos: eye,
        cam_view: eye,
        world_view: bits(view.camera.world_view()),
        world_screen: bits(view.world_screen()),
        env: Default::default(),
    }
}

enum Phase {
    /// `scene` is playing; `index` is its place in the scene list.
    Scene,
    /// Frames with nothing drawn before the next scene or the end.
    Gap(u32),
    Done,
}

/// A stream playing.
pub struct Stream {
    def: Def,
    opts: Options,
    /// The ccsTbl.
    scenes: Vec<Entry>,
    /// Index into `scenes` of the one playing (or next).
    index: usize,
    members: Vec<Member>,
    archive: Arc<Archive>,
    loaded: Loaded,
    scene: Option<Scene>,
    phase: Phase,
    requests: Vec<Request>,
    /// `ccGetStreamFrame`: the frame the last step drew.
    frame: u32,
    /// Frames the stream has drawn (steps).
    steps: u32,
    /// `ccSnd +0xe8`: `strSndTbl[num].strbgm`, walked by the notes of
    /// event 4.
    bgm: BgmCursor,
    /// `eventMsg` (0x00377cf4): the line the notes last asked for, -1 to
    /// close, -2 nothing new since `ccGetStreamDemoMsg` last read it.
    event_msg: i32,
    /// A skip's `eventMsg = -1`, which WaitEnd (priority 33) writes after
    /// the event task's pass: it lands at the start of the next step.
    skip_msg: bool,
    /// The scene view's `divZ` an effect task set (f32 bits).
    div_z: Option<u32>,
    /// The scene's effect task, while it runs.
    task: Option<Task>,
    /// What the tasks read from the executable.
    tables: effect::Tables,
    /// The task gets one more pass: the scene ended by itself and is in its
    /// first gap frame.
    task_tail: bool,
    /// Not the game's: a scene that reaches its last frame stays on it,
    /// drawn each frame, and no push skips it ([`Stream::hold_end`]).
    hold: bool,
    /// `Func_str0580` after its scene: it passes on (before the next
    /// scene's task) until it ends.
    tail: Option<Box<ending::Str0580>>,
    /// Stream 15's puff (`EFF_x001` of `str0580e`, as `ccEff::Init(chunk,
    /// 1)` leaves it), once read into the effects' assets.
    puff: Option<piney_effect::eff::Eff>,
    /// The scene files' effect nodes' Eff chunks (file, EFF_ object), each
    /// as `ccEffObj::Init` leaves it (`ccEff::Init(chunk, 1)`), once read
    /// into the effects' assets; None: not found there.
    eff_nodes: std::collections::HashMap<(usize, u32), Option<piney_effect::eff::Eff>>,
    /// The C library's `rand` state, which the task draws from.
    rand: Rand,
    /// The task's packets from the last step (or its first pass).
    effect_draws: effect::Draws,
    /// The stream demo's effects (`effcStr` and the second particle
    /// system), when the player gave them ([`Stream::set_effects`]).
    fx: Option<Box<piney_effect::StreamEffects>>,
}

/// The number a [`Stream::loose`] plays under: stream 47, one of the
/// table's `strdummy` entries, which has no music and no effect task.
const LOOSE_NUM: usize = 47;

impl Stream {
    /// Stream `num` with the default options (Japanese voices, circle
    /// cancels).
    pub fn new(iso: &mut Iso, num: usize) -> Result<Stream> {
        Stream::with_options(iso, num, Options::default())
    }

    pub fn with_options(iso: &mut Iso, num: usize, opts: Options) -> Result<Stream> {
        Stream::with_rand(iso, num, opts, Rand::default())
    }

    /// With the game's `rand` state as the stream starts: the whole game
    /// shares one, which every task drawing random numbers moves on (the
    /// default is its state at boot).
    pub fn with_rand(iso: &mut Iso, num: usize, opts: Options, rand: Rand) -> Result<Stream> {
        Stream::with_resident(iso, num, opts, rand, Vec::new())
    }

    /// [`Stream::with_rand`] over files the game already holds in memory
    /// (`resident`, oldest first): the place's file lists' CCSF files
    /// (`ccSetFileListField` and the others: `strcmnFileList`'s
    /// `STR8000E.CCS` everywhere, `datadrainFileList`'s `STR8001E.CCS` in a
    /// field or dungeon), which a scene's `#` objects resolve against as
    /// they do against its own preloads. Data Drain's streams draw nothing
    /// without them.
    pub fn with_resident(
        iso: &mut Iso,
        num: usize,
        opts: Options,
        rand: Rand,
        resident: Vec<StreamFile>,
    ) -> Result<Stream> {
        let volume = iso.volume()?;
        let def = Def::read(volume, num, opts.english)?;
        let files = def.files(0);
        Stream::from_files(iso, volume, def, files, opts, rand, resident, None)
    }

    /// Not the game's: the records `entries` of the archive at `path`,
    /// which no stream table lists ([`load::scan`], [`load::loose_streams`]:
    /// Outbreak's `STREAM/STRT.BIN`), played as a stream with no music and
    /// no effect task of its own (stream 47's number, `strdummy`).
    pub fn loose(iso: &mut Iso, path: &str, entries: Vec<table::Entry>, opts: Options) -> Result<Stream> {
        let volume = iso.volume()?;
        let header = table::Entry { name: "loose".into(), ofs: 0, size: 0, kind: 0, flag: 0, gzip: 0 };
        let def = Def { num: LOOSE_NUM, english: opts.english, header, entries };
        let files = def.files(0);
        Stream::from_files(iso, volume, def, files, opts, Rand::default(), Vec::new(), Some(path))
    }

    /// `ccRequestLoadStreamGateHack(107, town, field)` (0x00199f00), the
    /// Chaos Gate's movie of a gate hack ([`table::gate_hack_files`]): the
    /// town's departure, `str7200`, `str7300` looping while the field
    /// loads (until [`Stream::exit`], `ccSetupGameCtrl`'s `ResetPause`),
    /// then the arrival at `field`. `crisis`: `saveData` +0x6772.
    pub fn gate_hack(iso: &mut Iso, town: i32, field: i32, crisis: bool, opts: Options, rand: Rand) -> Result<Stream> {
        let volume = iso.volume()?;
        let def = Def::read(volume, table::gate_stream(volume), opts.english)?;
        let files = table::gate_hack_files(volume, &def, town, field, crisis)?;
        Stream::from_files(iso, volume, def, files, opts, rand, Vec::new(), None)
    }

    /// The stream of `def` playing `files`, over the `resident` files, read
    /// from the archive the header names or from `path`.
    #[allow(clippy::too_many_arguments)]
    fn from_files(
        iso: &mut Iso,
        volume: piney_data::volume::Volume,
        def: Def,
        files: table::Files,
        opts: Options,
        rand: Rand,
        resident: Vec<StreamFile>,
        path: Option<&str>,
    ) -> Result<Stream> {
        let num = def.num;
        // Read each named file once: a type -2 scene is also a preload.
        let mut members: Vec<Member> = Vec::new();
        for e in files.preload.iter().chain(&files.scenes) {
            if !members.iter().any(|m| m.entry.name == e.name && m.entry.ofs == e.ofs) {
                members.push(match path {
                    Some(p) => load::read_in(iso, p, &def, e)?,
                    None => load::read(iso, &def, e)?,
                });
            }
        }
        let archive = Arc::new(load::archive(&members.iter().collect::<Vec<_>>())?);
        // Outbreak's and Quarantine's streams carry a PCM track a voice
        // language; the earlier discs have one, and the voice picks the
        // archive instead.
        let pcm_track = matches!(volume, piney_data::volume::Volume::Out | piney_data::volume::Volume::Qua)
            .then_some(u8::from(opts.english));
        let mut loaded = Loaded { files: resident, pcm_track };
        for e in &files.preload {
            let m = member(&members, e)?;
            loaded.files.push(StreamFile::read(&archive, &m.stem())?);
        }
        let mut s = Stream {
            def,
            opts,
            scenes: files.scenes,
            index: 0,
            members,
            archive,
            loaded,
            scene: None,
            phase: Phase::Done,
            requests: Vec::new(),
            frame: 0,
            steps: 0,
            bgm: BgmCursor::new(table::stream_bgm_table(volume, num)),
            tables: effect::Tables {
                skill_names: opts.skill_names.map(|n| {
                    let tex =
                        piney_draw::TexRef::Ccs { file: SkillNames::FILE.into(), texture: n.texture, clut: n.clut };
                    (tex, n.tex_h)
                }),
                ..effect::Tables::read(volume)
            },
            event_msg: subtitle::MSG_NONE,
            skip_msg: false,
            div_z: None,
            task: None,
            task_tail: false,
            hold: false,
            tail: None,
            puff: None,
            eff_nodes: std::collections::HashMap::new(),
            rand,
            effect_draws: Vec::new(),
            fx: None,
        };
        let (before, _) = s.def.music();
        if before {
            s.requests.push(Request::Music { num, after: false });
        }
        s.start_scene()?;
        Ok(s)
    }

    /// The table entry played.
    pub fn def(&self) -> &Def {
        &self.def
    }

    /// The `ccsTbl`: the scenes in the order they play.
    pub fn scenes(&self) -> &[Entry] {
        &self.scenes
    }

    /// The `ccsTbl` record of the scene playing (or next).
    pub fn entry(&self) -> Option<&Entry> {
        self.scenes.get(self.index)
    }

    /// `CheckPause`, roughly: the scene playing is one the reader paused
    /// before (`flag` 1, the Chaos Gate's loop), so the caller's
    /// `ResetPause` ([`Stream::exit`]) ends it.
    pub fn at_pause(&self) -> bool {
        matches!(self.phase, Phase::Scene) && self.entry().is_some_and(|e| e.flag & table::FLAG_PAUSE != 0)
    }

    /// The stream demo's effects: `ccThEffectStr`'s and the second particle
    /// system's, which draw the effect tasks' hit marks and transfers
    /// (their files are `DATA.BIN`'s, so the player makes them). Without
    /// them the marks and transfers only come out as requests.
    pub fn set_effects(&mut self, fx: piney_effect::StreamEffects) {
        self.fx = Some(Box::new(fx));
    }

    /// The stream demo's effects, for inspection.
    pub fn effects(&self) -> Option<&piney_effect::StreamEffects> {
        self.fx.as_deref()
    }

    /// `ccThEffectStr` (priority 80, before the effect task): `MainStr` over
    /// the effects, through the scene's camera.
    fn fx_effects(&mut self) {
        let (Some(fx), Some(scene)) = (self.fx.as_mut(), self.scene.as_ref()) else { return };
        let mut host = FxHost { rand: &mut self.rand, camera: fx_camera(scene) };
        fx.step(&mut host);
    }

    /// The second particle system (`ccThParticle`, 98, after the task), then
    /// both systems' draws into the frame.
    fn fx_particles(&mut self, ctx: &mut Ctx) {
        let (Some(fx), Some(scene)) = (self.fx.as_mut(), self.scene.as_ref()) else { return };
        let camera = fx_camera(scene);
        let mut host = FxHost { rand: &mut self.rand, camera };
        fx.step_particles(&mut host);
        fx.draw(&mut ctx.layers, &camera);
    }

    /// The files the frames' `ModelDraw`s name, by stem: give the renderer
    /// this archive (with or instead of `DATA.BIN`) while the stream plays.
    pub fn archive(&self) -> Arc<Archive> {
        self.archive.clone()
    }

    /// The scene playing, for inspection.
    pub fn scene(&self) -> Option<&Scene> {
        self.scene.as_ref()
    }

    pub fn loaded(&self) -> &Loaded {
        &self.loaded
    }

    /// `ccGetStreamFrame()`: the scene frame the last step drew (0 before
    /// the first and between scenes). `PlayOpeningStream` flashes at 390.
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Game frames stepped so far.
    pub fn steps(&self) -> u32 {
        self.steps
    }

    /// The call has returned: every scene played (or skipped).
    pub fn done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    /// The effect task's packets on the last step, (layer, commands) in the
    /// order it sent them; after [`Stream::new`], its first pass (the frame
    /// before the scene's first). Already in the step's frame.
    pub fn effect_draws(&self) -> &[(i16, Vec<piney_draw::Cmd>)] {
        &self.effect_draws
    }

    /// The effect task, while it runs.
    pub fn effect(&self) -> Option<&Task> {
        self.task.as_ref()
    }

    /// The C library's `rand` state now: hand it on to whatever runs next.
    pub fn rand(&self) -> Rand {
        self.rand
    }

    /// What the frames since the last call asked for, in order.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// `ccGetStreamDemoMsg` (0x00184440), as `ccEventStream`'s pass calls it
    /// after the step: `eventMsg` (a line of `evStrMsgTbl[num]`, -1 to
    /// close, -2 nothing new), which is then -2.
    pub fn demo_msg(&mut self) -> i32 {
        std::mem::replace(&mut self.event_msg, subtitle::MSG_NONE)
    }

    /// `ExitScene` (0x001982c0) with no push: the scene playing ends as a
    /// skip ends it, and the stream goes on to its next scene or returns.
    /// It is how the Chaos Gate's looping scene (stream 107, `str7300`)
    /// ends: `ccSetupGameCtrl`'s `ResetPause` once the field has loaded
    /// lets `WaitEnd` take its skip path.
    pub fn exit(&mut self) {
        if matches!(self.phase, Phase::Scene) {
            self.requests.push(Request::PcmStop);
            self.skip_msg = true;
            self.phase = Phase::Gap(SKIP_BREATHS);
            self.task = None;
        }
    }

    /// DecodeThread's next file: DecodeSetup (or the preloaded file for a
    /// type -2 record), InitScene, and the scene thread's first pass up to
    /// its first breath.
    fn start_scene(&mut self) -> Result<()> {
        let Some(e) = self.scenes.get(self.index).cloned() else {
            self.finish();
            return Ok(());
        };
        let m = member(&self.members, &e)?;
        let stem = m.stem();
        let f = match self.loaded.files.iter().position(|f| f.stem == stem && e.kind == table::TYPE_ON_MEMORY) {
            Some(f) => f,
            None => {
                self.loaded.files.push(StreamFile::read(&self.archive, &stem)?);
                self.loaded.files.len() - 1
            }
        };
        let mut scene = Scene::new(&self.loaded, f, SYS_LAYER);
        if e.kind == table::TYPE_ON_MEMORY {
            scene.on_memory(&self.loaded);
        }
        let preroll = Scene::preroll(&self.loaded, f);
        if !preroll.is_empty() {
            self.requests.push(Request::Pcm(pcm_samples(&preroll)));
            self.requests.push(Request::PcmStart);
        }
        scene.decode(&self.loaded);
        // ccSetStreamDemoThread: nothing to show (`eventMsg` -2; the decoder,
        // priority 27, runs before the event task's pass); the previous task
        // ends, this scene's starts and runs its first pass in the frame
        // before the scene's first.
        self.event_msg = subtitle::MSG_NONE;
        // ccSetStreamDemoThread: the last task's param[1] 1, and 2 for the
        // one in eventExTscb2 (Func_str0580, which goes on passing).
        if let Some(Task::Str0580(mut t)) = self.task.take()
            && t.in_loop()
        {
            t.set_param(2);
            self.tail = Some(t);
        }
        self.task = None;
        self.task_tail = false;
        self.effect_draws.clear();
        if let Some(mut task) = Task::start(&stem, &self.loaded, &mut scene, &self.tables, &mut self.rand) {
            // Stream 15's tasks set the scene view's divZ at their start.
            let div_z = match &task {
                Task::Str0580(_) => Some(ending::DIV_Z_0580),
                Task::Str0581(_) => Some(ending::DIV_Z_0581),
                _ => None,
            };
            if let Some(z) = div_z {
                self.div_z = Some(z);
                self.requests.push(Request::DivZ(z));
            }
            let world = effect::SceneWorld { loaded: &self.loaded, scene: &scene };
            self.effect_draws =
                task.step_in(&world, &[], scene.frame_now, scene.frame_end, scene.paused(), &mut self.rand);
            self.task = Some(task);
        }
        self.scene = Some(scene);
        self.phase = Phase::Scene;
        self.flush_pcm();
        Ok(())
    }

    fn finish(&mut self) {
        let (_, after) = self.def.music();
        if after {
            self.requests.push(Request::Music { num: self.def.num, after: true });
        }
        self.phase = Phase::Done;
        self.scene = None;
        self.task = None;
        self.frame = 0;
    }

    /// The scene's PCM blocks read so far, as requests.
    fn flush_pcm(&mut self) {
        if let Some(s) = self.scene.as_mut()
            && !s.pcm.is_empty()
        {
            let blocks = std::mem::take(&mut s.pcm);
            self.requests.push(Request::Pcm(pcm_samples(&blocks)));
        }
    }

    /// `ccSetStreamDemoNote` then `ccSndStreamSE` on each note; the effect
    /// task's cues, in the order queued.
    fn notes(&mut self) -> Vec<Cue> {
        let mut cues = Vec::new();
        let Some(s) = self.scene.as_mut() else { return cues };
        let name = self.loaded.files[s.file].stem.clone();
        for n in std::mem::take(&mut s.notes) {
            match n.event {
                scene::NOTE_MSG => {
                    self.event_msg = n.param as i32;
                    self.requests.push(Request::Message(Some(n.param)))
                }
                scene::NOTE_MSG_OFF => {
                    self.event_msg = subtitle::MSG_CLOSE;
                    self.requests.push(Request::Message(None))
                }
                scene::NOTE_DEMO => {
                    // eventExNoteTbl holds eight; the rest are dropped.
                    if self.task.is_some() && cues.len() < effect::MAX_CUES {
                        cues.push(Cue { param: n.param, obj: n.obj });
                    }
                    self.requests.push(Request::Effect { scene: name.clone(), param: n.param });
                }
                // ccSndStreamSE: event 4 with a BGM table.
                scene::NOTE_BGM if self.bgm.is_set() => {
                    let record = self.bgm.step();
                    self.requests.push(Request::StreamBgm { num: self.def.num, param: n.param, record })
                }
                _ => {}
            }
        }
        cues
    }

    /// The effect task's pass for this frame, after the scene's: its packets
    /// to the front of their layers.
    fn effect_pass(&mut self, ctx: &mut Ctx, cues: &[Cue], frame_now: u32) {
        self.effect_draws.clear();
        // Func_str0580 after its scene first: the older thread of the two.
        if let (Some(t), Some(s)) = (self.tail.as_mut(), self.scene.as_ref()) {
            let world = effect::SceneWorld { loaded: &self.loaded, scene: s };
            let d = t.step(&world, &[], frame_now, s.paused(), &mut self.rand);
            for (layer, cmds) in &d {
                ctx.layers.prepend(*layer, cmds.clone());
            }
            self.effect_draws.extend(d);
            if t.done() {
                self.tail = None;
            }
        }
        let (Some(task), Some(s)) = (self.task.as_mut(), self.scene.as_ref()) else { return };
        let world = effect::SceneWorld { loaded: &self.loaded, scene: s };
        let d = task.step_in(&world, cues, frame_now, s.frame_end, s.paused(), &mut self.rand);
        for (layer, cmds) in &d {
            ctx.layers.prepend(*layer, cmds.clone());
        }
        self.effect_draws.extend(d);
        let parts = task.part_draws().to_vec();
        if !parts.is_empty() {
            self.draw_parts(ctx, &parts);
        }
        let (Some(task), Some(s)) = (self.task.as_mut(), self.scene.as_ref()) else { return };
        // An object's lwMatrix translation now: the frame the scene has
        // just read (the task runs after the scene's pass).
        let at = |obj: u32| {
            let w = s.by_obj.get(&obj).map_or([0; 4], |&i| s.world(i)[3]);
            [w[0], w[1], w[2]]
        };
        let camera = fx_camera(s);
        for t in task.take_transfers() {
            let pos = at(t.obj);
            if let Some(fx) = self.fx.as_mut() {
                let mut host = FxHost { rand: &mut self.rand, camera };
                fx.transfer(&mut host, [pos[0], pos[1], pos[2], ONE_BITS], t.height, Some(effect::STR_EFF_LAYER));
            }
            self.requests.push(Request::Transfer { obj: t.obj, pos, height: t.height });
        }
        for m in task.take_marks() {
            let pos = at(m.obj);
            if let Some(fx) = self.fx.as_mut() {
                let mut host = FxHost { rand: &mut self.rand, camera };
                let rot = [m.rot[0], m.rot[1], m.rot[2], 0];
                fx.hit_mark(&mut host, [pos[0], pos[1], pos[2], ONE_BITS], rot, Some(effect::STR_EFF_LAYER));
            }
            self.requests.push(Request::HitMark { obj: m.obj, pos: at(m.obj), rot: m.rot });
        }
        let fog_off = matches!(task, Task::Str0110(t) if t.fog_off);
        if let Task::Str0120(t) = task {
            let div_z = t.div_z;
            if let Some(z) = div_z
                && div_z != self.div_z
            {
                self.div_z = div_z;
                self.requests.push(Request::DivZ(z));
            }
        }
        // Func_str0110's cue x3 (SetFog(0, 0, 0, 0, 0)): no fog from the
        // next frame's draw.
        if fog_off && let Some(sc) = self.scene.as_mut() {
            sc.fog = None;
        }
    }

    /// The scene's effect nodes (`ccEffObj::DrawNoAnm` in
    /// `ccStreamDrawLayerList::Draw`), when the player gave the effects:
    /// each one running, its chunk's Eff at its world place with the
    /// pattern, scale, turn and transparency `F_Obj` gave it, on its layer
    /// (str0001's clouds of smoke).
    fn draw_effect_nodes(&mut self, ctx: &mut Ctx) {
        let Stream { scene, fx, eff_nodes, loaded, archive, .. } = self;
        let (Some(scene), Some(fx)) = (scene.as_ref(), fx.as_mut()) else { return };
        if scene.effs.iter().all(|e| e.pattern == scene::PATTERN_STOP) {
            return;
        }
        let camera = fx_camera(scene);
        for (k, e) in scene.effs.iter().enumerate() {
            if e.pattern == scene::PATTERN_STOP {
                continue;
            }
            let template = eff_nodes.entry(e.eff).or_insert_with(|| {
                let f = &loaded.files[e.eff.0];
                let name = f.name(e.eff.1)?.to_string();
                fx.assets.add_file(archive, &f.stem).ok()?;
                let o = fx.assets.find(&f.stem, &name)?;
                let (j, chunk) = fx.assets.eff_chunk(o)?;
                Some(piney_effect::eff::Eff::init(o.file, j, chunk, true, &fx.assets.alpha_blend))
            });
            let Some(t) = template else { continue };
            let mut eff = t.clone();
            let w = scene.eff_world(k);
            eff.pos = [w[3][0], w[3][1], w[3][2], ONE_BITS];
            eff.scale_x = e.scale[0];
            eff.scale_y = e.scale[1];
            eff.rotate = e.rotate;
            eff.transparency = e.transparency;
            eff.render(&fx.assets, &mut ctx.layers, e.layer, e.pattern, &camera);
        }
    }

    /// Stream 15's parts on the scene's layer: each puff a `ccEff::Draw` of
    /// `EFF_x001` (when the player gave the effects, whose assets read
    /// `str0580e` from the stream), each rock its model.
    fn draw_parts(&mut self, ctx: &mut Ctx, parts: &[ending::PartDraw]) {
        let Some(scene) = self.scene.as_ref() else { return };
        if self.puff.is_none()
            && let Some(fx) = self.fx.as_mut()
        {
            self.puff = [ending::EFF_FILE.to_string(), format!("{}p", ending::EFF_FILE)].iter().find_map(|stem| {
                fx.assets.add_file(&self.archive, stem).ok()?;
                let o = fx.assets.find(stem, ending::EFF_NAME)?;
                let (j, chunk) = fx.assets.eff_chunk(o)?;
                Some(piney_effect::eff::Eff::init(o.file, j, chunk, true, &fx.assets.alpha_blend))
            });
        }
        let g = self.loaded.files.iter().position(|f| f.stem.trim_end_matches('p') == ending::EFF_FILE);
        let camera = fx_camera(scene);
        for p in parts {
            match *p {
                ending::PartDraw::Puff { pos, pattern, scale, tp, colour } => {
                    let (Some(fx), Some(e)) = (self.fx.as_ref(), self.puff.as_ref()) else { continue };
                    let mut e = e.clone();
                    e.pos = pos;
                    e.scale_x = scale[0];
                    e.scale_y = scale[1];
                    e.transparency = tp;
                    e.color = colour;
                    e.render(&fx.assets, &mut ctx.layers, SYS_LAYER, pattern, &camera);
                }
                ending::PartDraw::Rock { model, matrix } => {
                    let Some(g) = g else { continue };
                    let Some(obj) = self.loaded.files[g].sf.ccs.find_object(ending::ROCKS[model]) else { continue };
                    let Some(m) = self.loaded.obj_model(g, obj) else { continue };
                    draw::draw_rigid(ctx, &self.loaded, scene, m, &matrix, SYS_LAYER);
                }
            }
        }
    }

    /// `WaitEnd` (0x00197f40) on this frame's push, while a scene plays.
    fn skip_pushed(&self, pad: &Pad) -> bool {
        let flag = self.scenes.get(self.index).map_or(table::FLAG_NO_SKIP, |e| e.flag);
        table::skips(flag, pad.push.bits(), self.opts.cancel.bits(), self.opts.title_after_desktop)
    }

    /// One game frame.
    pub fn step(&mut self, pad: &Pad) -> Frame {
        self.step_with(pad, &mut |_, _| {})
    }

    /// Not the game's: the scene stays on its last frame once there,
    /// drawn each frame, and pushes do not skip it (the port's launcher).
    pub fn hold_end(&mut self, on: bool) {
        self.hold = on;
    }

    /// Not the game's: the scene playing read on to its last frame at once,
    /// undrawn (its notes, sound and effects left out).
    pub fn fast_forward(&mut self) {
        if let Some(scene) = self.scene.as_mut() {
            // One read a frame at most, and one more for the end Top.
            for _ in 0..=scene.frame_end + 1 {
                if scene.ended() {
                    break;
                }
                scene.decode(&self.loaded);
            }
            scene.notes.clear();
            scene.pcm.clear();
        }
    }

    /// The scene playing, for [`Scene::set_alpha`].
    pub fn scene_mut(&mut self) -> Option<&mut Scene> {
        self.scene.as_mut()
    }

    /// [`Stream::step`] with another task drawing into the scene: `extra`
    /// runs after the scene's own draw, while one plays, with the scene (its
    /// camera, lights, layer and the frame it drew): `ccThDrainEnemy`'s
    /// enemy, drawn on the stream's layer through its draw environment.
    pub fn step_with(&mut self, pad: &Pad, extra: &mut dyn FnMut(&mut Ctx, &Scene)) -> Frame {
        let mut ctx = Ctx::new(View::default());
        self.steps += 1;
        if std::mem::take(&mut self.skip_msg) {
            self.event_msg = subtitle::MSG_CLOSE;
            self.requests.push(Request::Message(None));
        }
        match self.phase {
            Phase::Scene => self.scene_frame(pad, &mut ctx, extra),
            Phase::Gap(n) => {
                self.fx_effects();
                self.frame = 0;
                // After a scene's natural end its task still runs in the
                // first gap frame (the scene's DelSceneObject comes a frame
                // later); after a skip it has stopped.
                if std::mem::take(&mut self.task_tail) {
                    let now = self.scene.as_ref().map_or(0, |s| s.frame_now);
                    self.effect_pass(&mut ctx, &[], now);
                } else {
                    self.effect_draws.clear();
                }
                self.task = None;
                if n > 1 {
                    self.phase = Phase::Gap(n - 1);
                } else {
                    self.index += 1;
                    if let Err(e) = self.start_scene() {
                        eprintln!("stream {}: {e}", self.def.num);
                        self.finish();
                    }
                }
            }
            Phase::Done => {}
        }
        self.fx_particles(&mut ctx);
        let mut f = ctx.finish();
        // ccEventStream clears to black (bgColor 0); the title's is black.
        f.clear = Rgba::BLACK;
        f
    }

    /// `PlaySceneMain` after its breath: the notes, the draw, then the next
    /// frame's records; the end as the scene's flags say.
    fn scene_frame(&mut self, pad: &Pad, ctx: &mut Ctx, extra: &mut dyn FnMut(&mut Ctx, &Scene)) {
        let skip = self.skip_pushed(pad);
        let cues = self.notes();
        let Some(scene) = self.scene.as_mut() else { return };
        scene.div_z = self.div_z.map_or(piney_draw::DIV_Z, f32::from_bits);
        draw::draw(ctx, &self.loaded, scene);
        self.draw_effect_nodes(ctx);
        let Some(scene) = self.scene.as_mut() else { return };
        extra(ctx, scene);
        self.frame = scene.frame_now;
        if self.hold && scene.ended() {
            return;
        }
        self.fx_effects();
        let Some(scene) = self.scene.as_mut() else { return };
        if skip && !self.hold {
            // canselFlag, ExitScene: EndScene stops the audio and the scene
            // thread leaves at its next wake. The effect task still runs
            // this frame (the scene read on to its next frame), then stops.
            let next = (scene.frame_now + 1).min(scene.frame_end);
            self.requests.push(Request::PcmStop);
            self.skip_msg = true;
            self.phase = Phase::Gap(SKIP_BREATHS);
            self.effect_pass(ctx, &cues, next);
            self.task = None;
            return;
        }
        if !scene.ended() {
            scene.decode(&self.loaded);
            let now = scene.frame_now;
            self.flush_pcm();
            self.effect_pass(ctx, &cues, now);
            return;
        }
        let now = scene.frame_now;
        self.effect_pass(ctx, &cues, now);
        let Some(scene) = self.scene.as_mut() else { return };
        let e = &self.scenes[self.index];
        let followed = e.kind != table::TYPE_SETUP && self.index + 1 < self.scenes.len();
        if followed {
            // flag 4: the last frame stays until the next scene is set up,
            // then EndScene (EndPlay) and the next one starts.
            if self.loaded.files[scene.file].setup.pcm(self.loaded.pcm_track).is_some() {
                self.requests.push(Request::PcmStop);
            }
            self.index += 1;
            if let Err(e) = self.start_scene() {
                eprintln!("stream {}: {e}", self.def.num);
                self.finish();
            }
        } else {
            if self.loaded.files[scene.file].setup.pcm(self.loaded.pcm_track).is_some() {
                self.requests.push(Request::PcmEnd);
            }
            self.phase = Phase::Gap(END_BREATHS);
            self.task_tail = self.task.is_some();
        }
    }
}

fn member<'a>(members: &'a [Member], e: &Entry) -> Result<&'a Member> {
    members
        .iter()
        .find(|m| m.entry.name == e.name && m.entry.ofs == e.ofs)
        .ok_or_else(|| Error::NotFound(e.name.clone()))
}

/// PCM blocks (256 left samples, then 256 right, little-endian) to
/// interleaved stereo.
pub fn pcm_samples(blocks: &[Vec<u8>]) -> Vec<i16> {
    let mut out = Vec::with_capacity(blocks.len() * 2 * PCM_BLOCK_SAMPLES);
    for b in blocks {
        let n = b.len() / 4;
        let s = |i: usize| i16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
        for i in 0..n {
            out.push(s(i));
            out.push(s(n + i));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_blocks_interleave() {
        let mut b = vec![0u8; 1024];
        b[0] = 1; // left 0
        b[512] = 2; // right 0
        b[2] = 3; // left 1
        let s = pcm_samples(&[b]);
        assert_eq!(s.len(), 512);
        assert_eq!(&s[..4], &[1, 2, 3, 0]);
    }
}
