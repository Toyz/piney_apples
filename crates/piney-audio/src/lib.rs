//! The game's sound, made the way the PlayStation 2 makes it
//! (`docs/engine/sound.md`). The EE decides what to play ([`driver`], [`se3d`]);
//! the IOP's MIDI sequencer ([`midi`], MODMIDI.IRX) plays the banks'
//! sequences into the hardware synthesizer ([`hsyn`], MODHSYN.IRX), which also
//! takes the sound effects and drives the SPU2's voices ([`spu`]), stepped on
//! SNDBASE.IRX's 4,167 us tick and mixed at 48 kHz; BGM.BIN's tracks and the
//! voice lines ([`seword`]) go to the sound-data input. [`Audio::open`] plays
//! through the default output (silence when none); [`Audio::headless`] renders.

pub mod driver;
pub mod hsyn;
pub mod midi;
pub mod output;
pub mod reverb;
pub mod scene;
pub mod se3d;
#[rustfmt::skip]
pub mod setbl;
pub mod seword;
pub mod spu;
pub mod stream;
pub mod wav;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use piney_data::iso::Iso;
use piney_data::sound::{self, Bank, SndData, Tables, voice};

use crate::driver::{Command, Driver, VoiceCmd};
use crate::hsyn::Synth;
use crate::midi::Sequencer;
use crate::seword::Word;
use crate::spu::Spu;

pub use driver::{AreaMusic, BgmPlan, BgmWorld, SqContext, bgm_plan, setup_context};
pub use piney_data::Error;
pub use se3d::{Listener, NoteSe};
pub type Result<T> = std::result::Result<T, Error>;

/// SNDBASE.IRX's hard timer: sceMidi_ATick and sceHSyn_ATick every
/// 4,167 us.
pub const TICK_US: u64 = 4167;
/// Where the banks' sample data sits in sound RAM: the common SE bank at
/// 0x5010 (`ccSndCommSeLoad`), a music bank after it at
/// `commseTbl[2] + 0x5020` (`ccSndSQLoad`).
pub const SE_SPU_ADDR: u32 = 0x5010;
pub const MUSIC_SPU_BASE: u32 = 0x5020;

/// A `BGM.BIN` track playing into the sound-data input.
struct Stream {
    pcm: Vec<i16>,
    pos: usize,
    looped: bool,
}

/// What SEWORDS's channel 0 streams into core 0's sound-data input: a
/// `BGM.BIN` track or a voice line, never both - `wordPlay` and `bgmPlay`
/// refuse while the channel has a file open.
enum Input {
    Bgm(Stream),
    Voice(Word),
}

/// Everything behind the output: the SPU2, the synthesizer's four ports,
/// the three sequencers and the EE driver, advanced sample by sample.
pub struct Engine {
    pub spu: Spu,
    pub synth: Synth,
    pub seq: [Sequencer; 3],
    pub driver: Driver,
    tables: &'static Tables,
    /// Port 0's messages from the EE, delivered at the next tick.
    port0: Vec<u8>,
    /// SNDBASE's `sqPlayFlag[]`: set by play, cleared by stop.
    play_flag: [bool; 3],
    /// SNDBASE's `gIsInPlay_port0..2`: each sequencer playing, as ATick
    /// saw it at the start of the last tick.
    in_play: [bool; 3],
    /// SNDBASE's `playtype` (`ccSndCmd3(0x300, t)`).
    play_type: i32,
    music: Option<Arc<Bank>>,
    input: Option<Input>,
    /// Time towards the next tick, in 1/48,000 us.
    clock: u64,
    pub ticks: u64,
}

impl Engine {
    fn new(se_bank: Bank, tables: &'static Tables) -> Engine {
        let mut spu = Spu::new();
        spu.write_ram(SE_SPU_ADDR as usize, &se_bank.bd);
        let mut synth = Synth::new();
        let se_bank = Arc::new(se_bank);
        // ccSndCommSeLoad (0x00182050): port 0 holds the SE bank, priority
        // 0x40, 16 voices, volume 256.
        synth.load(0, se_bank, SE_SPU_ADDR);
        synth.set_attr(0, 0x1040);
        synth.set_volume(0, 256);
        Engine {
            spu,
            synth,
            seq: [Sequencer::new(), Sequencer::new(), Sequencer::new()],
            driver: Driver::new(),
            tables,
            port0: Vec::new(),
            play_flag: [false; 3],
            in_play: [false; 3],
            play_type: 0,
            music: None,
            input: None,
            clock: 0,
            ticks: 0,
        }
    }

    /// The driver's commands, in order; `voices` holds the file bytes read
    /// for each [`Command::Voice`] (see [`seword::read_len`]), `None` where
    /// the file is not on the disc.
    fn apply(&mut self, snd: &SndData, cmds: Vec<Command>, voices: Vec<Option<Vec<u8>>>) {
        let mut voices = voices.into_iter();
        for c in cmds {
            match c {
                Command::Voice(v) => self.word_play(&v, voices.next().flatten()),
                Command::VoiceStop => self.channel_stop(),
                Command::Msg(m) => self.port0.extend(m),
                Command::PortVolume(p, v) => self.synth.set_volume(p, v),
                Command::Master(v) => self.spu.master = (v, v),
                Command::Load(row) => self.load_music(snd, &row),
                // SNDBASE's allSoundOff (command 0x140, 0x2e5c): each port's
                // notes and voices off and its volume 0, then sqAllStop -
                // the music stops, not only the voices sounding now.
                Command::AllSoundOff => {
                    for port in 0..4 {
                        self.synth.all_note_off(port, &mut self.spu);
                        self.synth.all_sound_off(port, &mut self.spu);
                        self.synth.set_volume(port, 0);
                    }
                    for (s, f) in self.seq.iter_mut().zip(&mut self.play_flag) {
                        s.stop();
                        *f = false;
                    }
                }
                Command::Seq(i) => self.set_seq(i),
                Command::Play(n) => {
                    self.seq[n].play();
                    self.play_flag[n] = true;
                }
                Command::Stop(n) => {
                    self.seq[n].stop();
                    self.play_flag[n] = false;
                }
                // SNDBASE only prints the context.
                Command::Area(_) => {}
                Command::PlayType(t) => self.play_type = t,
                Command::BgmChange => self.bgm_change(),
            }
        }
    }

    /// SNDBASE's `bgmChange` (0x7e4, command 0x130): with play type 0 or
    /// 1, sequencer 1 located to sequencer 0's position and played when 0
    /// plays and 1 does not, else sequencer 0 to 1's when only 1 plays;
    /// with play type 2, sequencer 1 from its start unless it plays. The
    /// play flag is set when the play switch takes.
    fn bgm_change(&mut self) {
        let [p0, p1, _] = self.in_play;
        let (from, to) = match self.play_type {
            0 | 1 if p0 && !p1 => (Some(0), 1),
            0 | 1 if !p0 && p1 => (Some(1), 0),
            2 if !p1 => (None, 1),
            _ => return,
        };
        let pos = from.map_or(0, |f: usize| self.seq[f].position);
        self.seq[to].set_location(pos);
        if self.seq[to].play_switch(true) {
            self.play_flag[to] = true;
        }
    }

    /// `wordPlay` (SEWORDS 0x2c74): refused while channel 0 has a file open
    /// (a line or a `BGM.BIN` track), or when the file cannot be opened or
    /// preloaded; else the line streams from the next sample, the input
    /// volume set from `vol` (`BgmStart`'s `BgmSetVolumeDirect`).
    fn word_play(&mut self, v: &VoiceCmd, data: Option<Vec<u8>>) {
        if self.input.is_some() {
            return;
        }
        let Some(w) = data.and_then(|d| Word::open(d, v.size)) else { return };
        self.spu.input_vol = ((v.vol >> 16) as u16, v.vol as u16);
        self.input = Some(Input::Voice(w));
    }

    /// `sewordCmd(0x120, 0)` (SEWORDS `bgmFunc`): `BgmStop`, `BgmClose`,
    /// `BgmQuit`, `BgmSetVolumeDirect(0, 0)` - whatever channel 0 streams.
    fn channel_stop(&mut self) {
        self.input = None;
        self.spu.input_vol = (0, 0);
    }

    /// Whether channel 0 is streaming a voice line.
    pub fn voice_playing(&self) -> bool {
        matches!(&self.input, Some(Input::Voice(w)) if !w.stopped())
    }

    /// `ccSndChangeData`'s load (`ccSQDataLoadCD`, SNDBASE 0x1e44, then the
    /// 0x9051-0x9053 port loads and `ccSetSq` for each sequence): every
    /// port silenced, the bank's samples streamed into sound RAM, ports 1-3
    /// given the bank (priority 0x10, 32 voices), sequence i loaded into
    /// sequencer i and located to its start.
    fn load_music(&mut self, snd: &SndData, row: &sound::SqLoad) {
        let Ok(bank) = snd.load_row(row) else { return };
        for port in 0..4 {
            self.synth.reset_all_controllers(port);
            self.synth.all_note_off(port, &mut self.spu);
            self.synth.all_sound_off(port, &mut self.spu);
            self.synth.set_volume(port, 0);
        }
        // sqAllStop (SNDBASE 0x2db4).
        for (s, f) in self.seq.iter_mut().zip(&mut self.play_flag) {
            s.stop();
            *f = false;
        }
        let at = MUSIC_SPU_BASE + self.tables.commse[2];
        self.spu.write_ram(at as usize, &bank.bd);
        self.music = Some(Arc::new(bank));
    }

    /// `ccSndCmd(0x9051 + i)`, `(0xa1 + i, 0x2010)`, `(0x40 + i)`: port
    /// `i + 1` gets the bank (priority 0x10, 32 voices), sequencer `i` the
    /// sequence (`ccSetSq`).
    fn set_seq(&mut self, i: usize) {
        let Some(bank) = self.music.clone() else { return };
        let at = MUSIC_SPU_BASE + self.tables.commse[2];
        self.synth.load(i + 1, bank.clone(), at);
        self.synth.set_attr(i + 1, 0x2010);
        if let Some(sq) = bank.sequence(i) {
            self.seq[i].load(sq, i);
        }
    }

    /// One SNDBASE tick (ATick, 0x3860): the EE's messages into port 0; a
    /// sequence of sequencer 0 or 1 that stopped by itself rewound
    /// (`resetSqData`); `sceMidi_ATick`, its stream buffer `p` feeding port
    /// `p + 1`; then `sceHSyn_ATick`.
    fn tick(&mut self) {
        if !self.port0.is_empty() {
            let m = std::mem::take(&mut self.port0);
            self.synth.input(0, &m);
        }
        for (f, s) in self.in_play.iter_mut().zip(&self.seq) {
            *f = s.playing();
        }
        for i in 0..2 {
            if self.play_flag[i] && !self.seq[i].playing() {
                self.seq[i].reset();
                self.play_flag[i] = false;
            }
        }
        for s in &mut self.seq {
            for (p, m) in s.tick() {
                self.synth.input(p + 1, &m);
            }
        }
        self.synth.tick(&mut self.spu);
        self.ticks += 1;
    }

    /// Fill `out` with interleaved 48 kHz stereo.
    pub fn render(&mut self, out: &mut [i16]) {
        const PERIOD: u64 = TICK_US * spu::RATE as u64;
        for frame in out.as_chunks_mut::<2>().0 {
            self.clock += 1_000_000;
            if self.clock >= PERIOD {
                self.clock -= PERIOD;
                self.tick();
            }
            let input = self.input_sample();
            let (l, r) = self.spu.sample(input);
            frame[0] = l;
            frame[1] = r;
        }
    }

    /// Channel 0's next sample. A stream that has ended closes the channel
    /// and sets the input volume to 0, as `loopEnd` does.
    fn input_sample(&mut self) -> (i16, i16) {
        let v = match &mut self.input {
            None => return (0, 0),
            Some(Input::Bgm(s)) if s.pos + 1 < s.pcm.len() => {
                let v = (s.pcm[s.pos], s.pcm[s.pos + 1]);
                s.pos += 2;
                if s.pos + 1 >= s.pcm.len() && s.looped {
                    s.pos = 0;
                }
                Some(v)
            }
            Some(Input::Bgm(_)) => None,
            Some(Input::Voice(w)) => w.next_sample(),
        };
        v.unwrap_or_else(|| {
            self.channel_stop();
            (0, 0)
        })
    }
}

/// The riding Grunty's calls on the music (`pgrider.cpp`): `ccPgBgmInit`
/// as the flute's call starts, `ccPgBgmEnd(n)` as the ride ends (0 back to
/// the field's music, 1 into a dungeon or out of the field), and
/// `pgRideFlag`, which keeps `bgmChange` from switching to the battle
/// music while riding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PgBgm {
    Init,
    End(i32),
    Riding(bool),
}

/// The sound system as the runtime uses it.
pub struct Audio {
    engine: Arc<Mutex<Engine>>,
    snd: SndData,
    tables: &'static Tables,
    iso: Option<PathBuf>,
    output: Option<output::Output>,
    /// Why there is no output, when there is none.
    pub silent_reason: Option<String>,
}

impl Audio {
    /// From the banks alone, rendering only on request.
    pub fn from_snddata(snd: SndData) -> Result<Audio> {
        let tables = snd.tables();
        let se = snd.commse(tables)?;
        let mut engine = Engine::new(se, tables);
        engine.driver.set_voice(piney_data::tables::voice::of(snd.volume()));
        engine.driver.volume = snd.volume();
        let engine = Arc::new(Mutex::new(engine));
        Ok(Audio { engine, snd, tables, iso: None, output: None, silent_reason: None })
    }

    /// Headless: reads `DATA/SNDDATA.BIN` from the disc image and renders
    /// only through [`Audio::render`].
    pub fn headless(iso_path: impl AsRef<Path>) -> Result<Audio> {
        let path = iso_path.as_ref().to_path_buf();
        let snd = SndData::read(&mut Iso::open(&path)?)?;
        let mut a = Audio::from_snddata(snd)?;
        a.iso = Some(path);
        a.silent_reason = Some("headless".into());
        Ok(a)
    }

    /// Plays through the default output device; with none (or one that
    /// will not open) it stays silent without failing.
    pub fn open(iso_path: impl AsRef<Path>) -> Result<Audio> {
        let mut a = Audio::headless(iso_path)?;
        match output::Output::open(a.engine.clone()) {
            Ok(o) => {
                a.output = Some(o);
                a.silent_reason = None;
            }
            Err(e) => a.silent_reason = Some(e),
        }
        Ok(a)
    }

    pub fn is_silent(&self) -> bool {
        self.output.is_none()
    }

    /// The output device's (rate, channels), when there is one.
    pub fn output_format(&self) -> Option<(u32, u16)> {
        self.output.as_ref().map(|o| (o.rate, o.channels))
    }

    fn engine(&self) -> MutexGuard<'_, Engine> {
        match self.engine.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    fn run(&self, f: impl FnOnce(&mut Driver, &Tables, &mut Vec<Command>)) {
        let mut out = Vec::new();
        let mut e = self.engine();
        f(&mut e.driver, self.tables, &mut out);
        let mut voices = Vec::new();
        if out.iter().any(|c| matches!(c, Command::Voice(_))) {
            // Read the lines without holding the engine from the output.
            drop(e);
            for c in &out {
                if let Command::Voice(v) = c {
                    voices.push(self.read_voice(v));
                }
            }
            e = self.engine();
        }
        e.apply(&self.snd, out, voices);
    }

    /// What SEWORDS reads of a line: its file from `ofs` on, as far as the
    /// stream can go; `None` when the file is not on the disc.
    fn read_voice(&self, v: &VoiceCmd) -> Option<Vec<u8>> {
        let path = self.iso.as_ref()?;
        let ofs = u64::try_from(v.ofs).ok()?;
        voice::read_voice(&mut Iso::open(path).ok()?, v.file, ofs, seword::read_len(v.size)).ok()
    }

    /// `ccSeOn(n)`: sound effect `n` of `seData`.
    pub fn se_on(&self, n: usize) {
        if let Some(se) = self.tables.se.get(n) {
            let m = Driver::se_on(se);
            self.engine().port0.extend(m);
        }
    }

    /// `ccSeOnNote(n, note)`: sound effect `n` at another note.
    pub fn se_on_note(&self, n: usize, note: i8) {
        if let Some(se) = self.tables.se.get(n) {
            let m = Driver::se_note(se, note);
            self.engine().port0.extend(m);
        }
    }

    /// `ccSeOn3D(n, pos)`, or `ccSeOn3DNote(n, pos, note)` with a `note`:
    /// sound effect `n` at `pos` in the world, heard from the active camera
    /// `cam` (`None` when the game has none: full velocity, centred) -
    /// quieter with distance, silent beyond the row's reach, panned by its
    /// direction from the view.
    pub fn se_on_3d(&self, n: usize, cam: Option<&Listener>, pos: &se3d::V4, note: Option<i8>) {
        if let Some(se) = self.tables.se.get(n) {
            let mut e = self.engine();
            let m = e.driver.se_3d(se, cam, pos, note);
            e.port0.extend(m);
        }
    }

    /// An animation note's sound ([`se3d::spc_note`], [`se3d::enemy_note`],
    /// [`se3d::pc_note`], [`se3d::inu_note`]) at the character's `pos`.
    pub fn note_se(&self, s: NoteSe, cam: Option<&Listener>, pos: &se3d::V4) {
        self.se_on_3d(s.code, cam, pos, s.note);
    }

    /// `ccSeOn3DLoop(n, pos)`: a looping sound effect at `pos`; its id for
    /// [`Audio::se_off_loop`], or -1 when out of reach or with the eight
    /// slots taken.
    pub fn se_on_3d_loop(&self, n: usize, cam: Option<&Listener>, pos: &se3d::V4) -> i32 {
        let Some(se) = self.tables.se.get(n) else { return -1 };
        let mut e = self.engine();
        let (id, m) = e.driver.se_3d_loop(se, cam, pos);
        e.port0.extend(m);
        id
    }

    /// `ccSeOffLoop(n, id)`: end loop `id` of sound effect `n`.
    pub fn se_off_loop(&self, n: usize, id: i32) {
        if let Some(se) = self.tables.se.get(n) {
            let mut e = self.engine();
            let m = e.driver.se_off_loop(se, id);
            e.port0.extend(m);
        }
    }

    /// `ccSoundMain`'s scene sounds for this frame ([`scene::scene_sound`]):
    /// Mac Anu's canals, area 15's church music, the breeder's tune.
    pub fn scene_sound(&self, s: &scene::SceneInput) {
        self.run(|d, t, out| scene::scene_sound(d, t, s, out));
    }

    /// `tobjSeLoopStart`: TOBJ's hum ([`se3d::TOBJ_SE`]) started silent in
    /// a loop slot, unless it plays.
    pub fn tobj_se_loop_start(&self) {
        if let Some(se) = self.tables.se.get(se3d::TOBJ_SE) {
            let mut e = self.engine();
            let m = e.driver.tobj_se_loop_start(se);
            e.port0.extend(m);
        }
    }

    /// `tobjSeLoop(pos, rate)`: the hum's pan and volume for TOBJ at `pos`
    /// with transparency `rate`, heard from `cam`.
    pub fn tobj_se_loop(&self, cam: &Listener, pos: &se3d::V4, rate: se3d::F) {
        if let Some(se) = self.tables.se.get(se3d::TOBJ_SE) {
            let mut e = self.engine();
            let m = e.driver.tobj_se_loop(se, cam, pos, rate);
            e.port0.extend(m);
        }
    }

    /// The jukebox: play `Wave[no]`, fading out what plays over the next
    /// 10 frames first (so [`Audio::frame`] must be running).
    pub fn bgm(&self, no: usize) {
        if let Some(w) = self.tables.wave.get(no).copied() {
            self.run(|d, t, out| d.change(t, w, out));
        }
    }

    /// The desktop starting with `saveData.dtBgm = no`: load and play at
    /// once, as `ccSetupDesktop` does.
    pub fn desktop_bgm(&self, no: usize) {
        if let Some(w) = self.tables.wave.get(no).copied() {
            self.run(|d, t, out| d.desktop(t, w, out));
        }
    }

    pub fn bgm_stop(&self) {
        self.run(|d, _, out| d.stop(out));
    }

    /// `ccSoundFadeOut()`: the music faded out over 8 frames and stopped.
    pub fn sound_fade_out(&self) {
        self.engine().driver.sound_fade_out();
    }

    /// `saveData.mainVol`, `seVol`, `bgmVol`, each 0..256.
    pub fn set_volumes(&self, main: i32, se: i32, bgm: i32) {
        self.run(|d, _, out| d.set_volumes(main, se, bgm, out));
    }

    /// `ccSetMainVol(v)`: the master volume alone, 0..256, on the next frame.
    pub fn set_main_volume(&self, v: i32) {
        self.engine().driver.set_main_volume(v);
    }

    /// `ccSndSQLoad(n)`: load a mode's bank (the title's is
    /// [`SqContext::Title`], an area's what [`driver::setup_context`] says):
    /// all sound off, the bank, its sequences, the port volumes. Nothing
    /// plays until [`Audio::sq_play`] or [`Audio::bgm_ctrl`].
    pub fn sq_load(&self, ctx: SqContext) {
        self.run(|d, t, out| d.sq_load(t, ctx, out));
    }

    /// `ccSqPlay(n)`: sequence `n` of the loaded bank from its start, unless
    /// it is playing (0 is the title's music).
    pub fn sq_play(&self, n: i32) {
        if let Ok(n) = usize::try_from(n) {
            self.run(|d, _, out| d.sq_play(n, out));
        }
    }

    /// `ccSqStop(n)`.
    pub fn sq_stop(&self, n: i32) {
        if let Ok(n) = usize::try_from(n) {
            self.run(|d, _, out| d.sq_stop(n, out));
        }
    }

    /// `ccSqFade(n, volume, time, mode)`: sequence `n`'s port from where it
    /// is to `volume` 256ths of its table volume over `time` frames of
    /// [`Audio::frame`]; `mode` bit 1 stops the sequence at the end. On the
    /// title (`ccSceneFade`) the port is written every frame; once a mode
    /// has started the game (`ccFade`), every other frame.
    pub fn sq_fade(&self, n: i32, volume: u16, time: i32, mode: u8) {
        if let Ok(n) = usize::try_from(n) {
            self.engine().driver.sq_fade(n, volume, time, mode);
        }
    }

    /// `ccPortVolSet(port, vol)`: the SE port (0) takes the SE volume, the
    /// music ports `vol` scaled by the BGM volume.
    /// The synthesizer has ports 0-3; the game would write past them, the
    /// port ignores it.
    pub fn port_volume(&self, port: usize, vol: u16) {
        if port < 4 {
            self.run(|d, _, out| d.port_vol_set(port, vol, out));
        }
    }

    /// `ccAllSoundOff()`, which each mode's set-up calls first: fades 0 and
    /// 1 stopped, all sound off and the voice stopped on the next frame.
    pub fn all_sound_off(&self) {
        self.engine().driver.all_sound_off();
    }

    /// `ccSound::gameInterrupt()` from `ccGame::ChangeRequest`: until the
    /// next mode starts the game (the desktop does, through
    /// [`Audio::desktop_bgm`]), fades run as the title's do.
    pub fn game_interrupt(&self) {
        self.engine().driver.game_interrupt();
    }

    /// `ccSndGameOver()`: the music faded out over 20 frames.
    pub fn game_over(&self) {
        self.engine().driver.game_over();
    }

    /// `ccSndBgmCtrl()`: start the loaded bank's music as the game does once
    /// a mode's fade in has ended (see [`driver::bgm_plan`]).
    pub fn bgm_ctrl(&self, world: driver::BgmWorld) -> driver::BgmPlan {
        let mut plan = None;
        self.run(|d, _, out| plan = Some(d.bgm_ctrl(&world, out)));
        plan.expect("bgm_ctrl ran")
    }

    /// The event instruction `sound 10`: the next [`Audio::bgm_ctrl`] of the
    /// desktop, a field, a dungeon or an event bank starts nothing.
    pub fn hold_bgm(&self) {
        self.engine().driver.hold_bgm();
    }

    /// `ccSnd.gameStart = 1`, which `ccSetupGameCtrl` (0x001694ec, before
    /// its fade in), `ccSetupDesktop` and `ccSetupToppage` set: fades run as
    /// `ccFade` does, and `bgmChange` runs.
    pub fn game_start(&self) {
        self.engine().driver.game_start = true;
    }

    /// `game.inBattle` and `pgRideFlag == 1`, which the sound task's
    /// `bgmChange` reads every frame: a battle starting switches a field's
    /// or dungeon's music to its battle arrangement, its end switches back.
    pub fn set_battle(&self, in_battle: bool, riding: bool) {
        let mut e = self.engine();
        e.driver.in_battle = in_battle;
        e.driver.riding = riding;
    }

    /// `game.inBattle` alone ([`Audio::set_battle`]'s first half).
    pub fn set_in_battle(&self, in_battle: bool) {
        self.engine().driver.in_battle = in_battle;
    }

    /// `ccSndGateHack(n)`: the gate hack's menu silencing the music (0),
    /// bringing it back when cancelled (1), done (2)
    /// ([`driver::Driver::gate_hack`]).
    pub fn gate_hack(&self, n: i32) {
        self.engine().driver.gate_hack(n);
    }

    /// The riding Grunty's music ([`PgBgm`]).
    pub fn pg_bgm(&self, p: PgBgm) {
        match p {
            PgBgm::Init => self.engine().driver.pg_bgm_init(),
            PgBgm::End(n) => {
                // bgmWavCmd 0x120: channel 0's BGM.BIN track stopped.
                self.bgm_stream_stop();
                self.run(|d, _, out| d.pg_bgm_end(n, out));
            }
            PgBgm::Riding(on) => self.engine().driver.riding = on,
        }
    }

    /// Once per game frame: fades, option changes, a pending jukebox change.
    pub fn frame(&self) {
        self.run(|d, t, out| d.frame(t, out));
    }

    /// `saveData.voice` (+0x842c; true for English, `VOICE_E/`),
    /// `saveData.parodyFlag` (+0x842b) and `talkNum` by `charTbl` row
    /// (`ccSaveData` +0x220c, the extension's for 18-20), which
    /// [`Audio::voice`] reads.
    pub fn set_voice_options(&self, english: bool, parody: bool, talk_num: [i8; 21]) {
        let mut e = self.engine();
        e.driver.voice_english = english;
        e.driver.parody = parody;
        e.driver.talk_num = talk_num;
    }

    /// A message window opening: `ccEvVoiceRequest(event, msg)`. The line
    /// is sent on the next [`Audio::frame`] and plays unless channel 0 is
    /// still streaming (the previous line, if nothing stopped it). True
    /// when the event's table has a line for the message.
    pub fn voice(&self, event: i32, msg: i32) -> bool {
        let mut e = self.engine();
        if event == piney_data::sound::voice::FOOD_GROUP {
            // ccVoicePgFood's own request.
            return e.driver.food_voice_request(msg);
        }
        e.driver.voice_request(event, msg)
    }

    /// `ccEvVoiceStop()`: on the next [`Audio::frame`], stop channel 0 -
    /// the voice, or a `BGM.BIN` track.
    pub fn voice_stop(&self) {
        self.engine().driver.voice_stop();
    }

    /// `ccWordsPlay(sid, ch)`: a skill's name queued for the sound task
    /// ([`driver::Driver::words_play`]); -1 when refused.
    pub fn words_play(&self, event_running: bool, char_type: u32, char_id: i16, sid: i32, type_bit: bool) -> i32 {
        self.engine().driver.words_play(event_running, char_type, char_id, sid, type_bit)
    }

    /// `game.area` as the sound task reads it: the skill words play only in
    /// a field (1) or a dungeon (2).
    pub fn set_game_area(&self, area: i32) {
        let mut e = self.engine();
        e.driver.game_area = area;
        e.driver.in_world = true;
    }

    /// Whether a voice line is streaming now.
    pub fn voice_playing(&self) -> bool {
        self.engine().voice_playing()
    }

    /// `wavPlay` (0x0017e6e0): a `VOICE/BGM.BIN` track (0 the loop played
    /// in the Grunty race, 1 the staff roll) into the sound-data input, at
    /// `0x7fff * bgmVol >> 8`.
    pub fn bgm_stream(&self, track: usize) -> Result<()> {
        let t = *self.tables.bgm.get(track).ok_or_else(|| Error::NotFound(format!("BGM.BIN track {track}")))?;
        let path = self.iso.as_ref().ok_or_else(|| Error::NotFound("the disc image".into()))?;
        let mut disc = Iso::open(path)?;
        let entry = disc.find("VOICE/BGM.BIN")?;
        let raw = disc.read_at(&entry, t.ofs as u64, t.size as usize)?;
        if raw.len() != t.size as usize {
            return Err(Error::NotFound(format!("BGM.BIN track {track} runs past the file")));
        }
        let pcm = raw.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect();
        let mut e = self.engine();
        let vol = ((0x7fff * e.driver.bgm_vol) >> 8) as u16;
        e.spu.input_vol = (vol, vol);
        // SEWORDS's bgmPlay would refuse while a line streams; nothing that
        // plays BGM.BIN is ported yet, so the track takes the channel.
        e.input = Some(Input::Bgm(Stream { pcm, pos: 0, looped: t.loops() }));
        Ok(())
    }

    pub fn bgm_stream_stop(&self) {
        let mut e = self.engine();
        if matches!(e.input, Some(Input::Bgm(_))) {
            e.input = None;
        }
    }

    /// `audioDecStart` (demo.prg 0x0040d3e0): a PSS movie's sound
    /// (interleaved 48 kHz stereo) into the sound-data input at 0x7fff on
    /// both sides, not scaled by the BGM volume.
    pub fn movie_stream(&self, pcm: Vec<i16>) {
        let mut e = self.engine();
        e.spu.input_vol = (0x7fff, 0x7fff);
        e.input = Some(Input::Bgm(Stream { pcm, pos: 0, looped: false }));
    }

    /// A STREAM cutscene's samples (`CcspcmSetData`, interleaved 48 kHz
    /// stereo), queued after what channel 0 already streams, at
    /// `CcspcmStartPlay`'s 0x7fff. A queue that ran dry has closed the
    /// channel; the next samples open it again.
    pub fn stream_pcm(&self, pcm: Vec<i16>) {
        let mut e = self.engine();
        e.spu.input_vol = (0x7fff, 0x7fff);
        match &mut e.input {
            Some(Input::Bgm(s)) if !s.looped => s.pcm.extend(pcm),
            _ => e.input = Some(Input::Bgm(Stream { pcm, pos: 0, looped: false })),
        }
    }

    /// `audioDecReset` (demo.prg 0x0040d460): the input's volume back to 0
    /// and the movie's sound gone.
    pub fn movie_stream_stop(&self) {
        let mut e = self.engine();
        e.spu.input_vol = (0, 0);
        if matches!(e.input, Some(Input::Bgm(_))) {
            e.input = None;
        }
    }

    /// Headless rendering: interleaved 48 kHz stereo.
    pub fn render(&self, out: &mut [i16]) {
        self.engine().render(out);
    }

    /// The engine, for inspection.
    pub fn with_engine<R>(&self, f: impl FnOnce(&mut Engine) -> R) -> R {
        f(&mut self.engine())
    }

    pub fn tables(&self) -> &'static Tables {
        self.tables
    }

    pub fn snddata(&self) -> &SndData {
        &self.snd
    }
}
