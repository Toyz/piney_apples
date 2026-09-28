//! The game, run from the disc image.
//!
//!     piney-game [--iso PATH] [--mode MODE]
//!     piney-game [--iso PATH] [--mode MODE] --shot OUT.png [--frames N] [--press F[-G]:BUTTON,...]
//!
//! Reads `DATA/DATA.BIN` from the disc image (default
//! `work/infection/infection.iso`) and runs one mode on the game's own
//! clock: NTSC vertical blanks at 59.94 a second, a game frame every
//! `frameRate` of them. Each frame the pad is read as `ccPad::Read` reads it
//! and the mode's draw list is drawn as the GS draws it (`piney-gs`), in the
//! game's 512 x 448 frame buffer, shown at 4:3.
//!
//! Modes: `game` (the default) runs from power-on as `ccThMother` does: the
//! title (`DEMO.PRG`: the card check, the logos, the menu), New Game into the
//! desktop, START's Title Screen back again. `desktop` starts on the ALTIMIT
//! desktop of a new game, skipping the title; `world` starts in Mac Anu on
//! a new game's save, the player named Kite, with the field's HUD and
//! menus and the event task as a new game leaves it at Log in (event 2,
//! Orca's welcome, plays, and its `scene` takes Kite to story area 14's
//! field; `--no-events` for none), `world:1` the same in Dun Loireag (the
//! save's `lastTown` 1); `field:N` in story area N's field (14,
//! the first, made from its words as event 2 makes it), where `--press
//! F:gateout` goes back to the town as the Gate Out menu does (`F:gofield`
//! from a dungeon to its field, as TransFieldMenu does), and
//! `field:N/TYPE,ROW[,HACK[,SEED]]` has another field type, weather row,
//! hack flag and seed over it (`field:14/7,5` type 7's night);
//! `dungeon:N` in its dungeon's first room, as its entrance leads in;
//! `story:N` where event N of the opening arc opens (3, 4, 10, 11, 12, 13,
//! 14), the save and the event task as the story before it leaves them
//! (each earlier event brought forward by `ccEventFlagSet`, `start.rs`): 3
//! in story area 14's field, 4 in its dungeon, 10 and 12 on the desktop,
//! 11 and 13 in Mac Anu, 14 on the top page;
//! `test-card[:FILE]` shows every texture of a `DATA.BIN` member (default
//! `xddesk01`) with a cursor the D-pad moves.
//!
//! The desktop runs the game's own event scripts from the disc beside it,
//! from the boot on, as a new game does: event 1 delivers the first mails,
//! posts the board and news, and locks every icon but the mailer until the
//! first two mails are read. The scripts' messages are printed to the
//! terminal for now. `--no-events` leaves them out; `--mail N,...` and
//! `--news N,...` deliver mails and post headlines by hand.
//!
//! The Data screen saves to a memory card kept as a directory
//! (`work/memcard/slot1` by default, `--card DIR`, `--no-card` for none),
//! in the card's own layout: `BASLUS-20267DOTHACK/` with its index and
//! `dhdata01`-`12`.
//!
//! Sound effects and music play through `piney-audio` (the game's own sound
//! driver, sequencer and synthesizer; `--mute` for none). Mode changes and
//! movies are printed until those parts exist.
//!
//! `--shot` runs N frames (default 60) with no window, pressing BUTTON on
//! frame F for each `F:BUTTON` (see `input::button`), or holding it over
//! frames F to G for `F-G:BUTTON` (`lup`, `ldown`, `lleft`, `lright` push
//! the left stick, `rup` ... the right), and writes the last frame's frame buffer as a 512 x
//! 448 PNG. `--webp OUT.webp` also writes the frames (from `--webp-from F`,
//! every `--webp-every N`th, at `--webp-quality Q`) as one animated WebP,
//! timed on the game's clock, for sharing (`webp`).
//!
//! Keyboard and pad: see `input`. Escape asks whether to quit (`quit`):
//! the game stops under the desktop menu's own confirmation window; OK
//! quits, Cancel or Escape again goes back. `--quit N` opens it at frame N
//! of a `--shot`.

mod area;
mod area_host;
mod console;
mod desktop;
mod dvd;
mod field_host;
mod fx;
mod gameover;
mod input;
mod launcher;
mod loaddisp;
mod loose;
mod mode;
mod movie;
mod piros;
mod quit;
mod session;
mod settings;
mod start;
mod story;
mod stream;
mod toppage;
mod town_fx;
mod webp;
mod world;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gilrs::Gilrs;
use piney_audio::Audio;
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_gs::{Assets, Gs, Presenter};
use piney_input::{Buttons, Pad, Raw};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::mode::{Event, Mode, TestCard};

/// NTSC vertical blanks a second.
const VBLANK_HZ: f64 = 59.94;
/// Game frames caught up at most per redraw, so a stall does not run the
/// game fast afterwards.
const MAX_CATCH_UP: u32 = 4;

struct Display {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    presenter: Presenter,
}

struct App {
    mode: Box<dyn Mode>,
    /// Held until the window's device exists.
    assets: Option<Assets>,
    gs: Option<Gs>,
    win: Option<Display>,
    display: OwnedDisplayHandle,
    instance: Option<wgpu::Instance>,
    keyboard: input::Keyboard,
    gilrs: Option<Gilrs>,
    pad: Pad,
    last: Instant,
    /// Vertical blanks not yet spent on a game frame.
    vblanks: f64,
    /// The game's sound, when there is an output device.
    audio: Option<Audio>,
    /// `--pad-log FILE`: every game frame's pad, as read and as decoded.
    pad_log: Option<std::io::BufWriter<std::fs::File>>,
    /// `--replay FILE`: a pad log's pads, one a frame, before the live
    /// pads take over.
    replay: std::collections::VecDeque<Raw>,
    frame_no: u64,
    /// The debug console (F1), and what its `story N` restarts from.
    console: console::Console,
    iso: PathBuf,
    archive: Arc<Archive>,
    card: Option<PathBuf>,
    /// The disc's volume, named in the window's title.
    volume: piney_data::volume::Volume,
    /// Pad 0's motors (`ccPad`'s queue) and the gamepad's rumble.
    actuator: piney_input::actuator::Actuator,
    rumble: Option<input::Rumble>,
    /// Ctrl held, for the console's keys.
    ctrl: bool,
    /// Escape's quit prompt while it is open (the game stands still).
    quit: Option<quit::QuitPrompt>,
    /// The prompt's OK: the window closes after this frame.
    exit: bool,
    /// How a game the launcher starts runs (its disc set when it starts).
    options: Options,
    /// No sound was asked for (`--mute`).
    mute: bool,
    /// The launcher is up: no volume is playing yet.
    launching: bool,
}

impl App {
    /// A console line: `story N` restarts the game at a story start here;
    /// the rest go to the mode.
    fn console_line(&mut self, line: &str) -> String {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.first() == Some(&"story") {
            let Some(n) = w.get(1).and_then(|s| s.parse::<i32>().ok()) else {
                return format!("story N, N one of {:?}", start::POINTS);
            };
            return match start::session(self.iso.clone(), self.archive.clone(), self.card.clone(), n) {
                Ok(s) => {
                    self.mode = Box::new(s);
                    format!("story {n}")
                }
                Err(e) => e,
            };
        }
        let answer = self.mode.console(line);
        if w.first() == Some(&"help") {
            return format!("{answer}\nstory N           the game again at event N's start");
        }
        answer
    }
}

impl App {
    /// The launcher's choice: the game again on the disc at `path`, its
    /// `DATA.BIN`, sound and fonts in place of the launcher's.
    fn boot(&mut self, path: PathBuf) {
        let options = Options { iso: path.clone(), logos_played: launcher::LOGOS.len() as i32, ..self.options.clone() };
        let booted = match boot(&options) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                return;
            }
        };
        if let Some(gs) = &mut self.gs {
            gs.set_archive(booted.archive.clone());
            gs.set_overlay(None);
        }
        // The launcher's sound goes before the game's opens.
        self.audio = None;
        if !self.mute {
            self.audio = open_audio(&path);
        }
        self.console = console::Console::new(console_fonts(&path, &booted.archive));
        self.archive = booted.archive;
        self.volume = booted.volume;
        self.mode = booted.mode;
        self.iso = path;
        self.options = options;
        self.launching = false;
        self.vblanks = 0.0;
    }

    /// Escape: the quit prompt opens over the game, or closes if open.
    /// Without the game's fonts there is no window to ask with, and Escape
    /// quits at once.
    fn toggle_quit(&mut self) {
        if self.quit.take().is_some() {
            return;
        }
        let Some(fonts) = self.console.fonts().cloned() else {
            self.exit = true;
            return;
        };
        let save = self.mode.save_copy();
        self.quit = Some(quit::QuitPrompt::open(self.volume, &self.archive, fonts, save));
        self.keyboard = input::Keyboard::default();
    }

    /// A frame with the quit prompt open: the game's picture held, the
    /// window run on the live pad and drawn over it. The first frame steps
    /// the game once on a neutral pad for the picture to hold.
    fn quit_frame(&mut self, live: &Raw) {
        let Some(q) = &mut self.quit else { return };
        let frame = if q.holding() {
            let (answer, se) = q.frame(live);
            if let Some(a) = &self.audio {
                for n in se {
                    a.se_on(n as usize);
                }
            }
            match answer {
                quit::Answer::Quit => self.exit = true,
                quit::Answer::Back => {
                    if let Some(gs) = &mut self.gs {
                        gs.render(&q.backdrop());
                    }
                    self.quit = None;
                    self.keyboard = input::Keyboard::default();
                    return;
                }
                quit::Answer::Open => {}
            }
            q.picture(self.mode.frame_rate())
        } else {
            self.pad.read(&Raw::default());
            let f = self.mode.step(&self.pad);
            handle(self.mode.take_events(), self.audio.as_ref());
            let mut picture = None;
            if let Some(gs) = &mut self.gs {
                gs.set_overlay(self.mode.archive());
                gs.render(&f);
                let (w, h) = gs.target_size();
                picture = Some((w as u16, h as u16, gs.read_back()));
            }
            q.hold(picture);
            return;
        };
        if let Some(a) = &self.audio {
            a.frame();
        }
        if let Some(gs) = &mut self.gs {
            gs.render(&frame);
        }
    }

    /// Run the game frames that are due, and draw the newest.
    fn tick(&mut self) {
        let now = Instant::now();
        self.vblanks += (now - self.last).as_secs_f64() * VBLANK_HZ;
        self.last = now;
        let rate = f64::from(self.mode.frame_rate().max(1));
        let mut steps = 0;
        while self.vblanks >= rate && steps < MAX_CATCH_UP {
            self.vblanks -= rate;
            steps += 1;
            let mut line = String::new();
            let logging = self.pad_log.is_some();
            let mut live = input::read(&self.keyboard, self.gilrs.as_mut(), logging.then_some(&mut line));
            // The console holds the game's pad neutral while it is open.
            if self.console.open {
                live = Raw::default();
            }
            // The quit prompt stops the game under it.
            if self.quit.is_some() {
                self.quit_frame(&live);
                continue;
            }
            let raw = match self.replay.pop_front() {
                Some(r) => {
                    if self.replay.is_empty() {
                        eprintln!("replay: done at frame {}; the pad is yours", self.frame_no + 1);
                    }
                    r
                }
                None => live,
            };
            self.pad.read(&raw);
            self.frame_no += 1;
            // ccSystem::Ctrl before the frame: the motors' time, then what
            // ccPad::Ctrl sends them.
            self.actuator.tick(rate as u16);
            if let Some(m) = self.actuator.ctrl() {
                if self.rumble.is_none() {
                    self.rumble = self.gilrs.as_mut().and_then(input::Rumble::new);
                }
                if let Some(r) = &self.rumble {
                    r.set(m);
                }
            }
            if let Some(log) = &mut self.pad_log {
                use std::io::Write;
                let p = &self.pad;
                let _ = writeln!(
                    log,
                    "{} {line} | bytes {} {} {} {} buttons {:04x} | l {:.3} {} r {:.3} {} direct {:04x} | {}",
                    self.frame_no,
                    raw.lx,
                    raw.ly,
                    raw.rx,
                    raw.ry,
                    raw.buttons.bits(),
                    p.dirc_l,
                    p.pow_l,
                    p.dirc_r,
                    p.pow_r,
                    p.direct.bits(),
                    self.mode.title()
                );
            }
            let frame = self.mode.step(&self.pad);
            if let Some(on) = self.mode.vibration() {
                self.actuator.sw = on;
            }
            let events = self.mode.take_events();
            for e in &events {
                if let Event::Actuate { small, power, ms } = *e {
                    self.actuator.set(small, power, ms);
                }
            }
            let boot = boot_request(&events);
            handle(events, self.audio.as_ref());
            if let Some(path) = boot {
                self.boot(path);
                break;
            }
            if let Some(a) = &self.audio {
                a.frame();
            }
            // Every frame is drawn, caught up or not: a frame can sample
            // the one before it (a stream's feedback).
            if let Some(gs) = &mut self.gs {
                gs.set_overlay(self.mode.archive());
                gs.render(&frame);
            }
        }
        if steps == MAX_CATCH_UP {
            self.vblanks = self.vblanks.min(rate);
        }
        if steps > 0
            && let Some(w) = &self.win
        {
            // Where the game is: the console's `where`.
            let title = if self.launching { "piney".to_string() } else { format!("piney - {}", self.volume.title()) };
            w.window.set_title(&title);
        }
    }

    fn present(&mut self) {
        let (Some(gs), Some(w)) = (&self.gs, &mut self.win) else { return };
        let frame = match w.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                w.surface.configure(gs.device(), &w.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let overlay = self.console.overlay(w.config.width, w.config.height);
        let commands = w.presenter.present(gs, &view, w.config.width, w.config.height, overlay.as_ref());
        gs.queue().submit([commands]);
        w.window.pre_present_notify();
        gs.queue().present(frame);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.win.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(if self.launching { "piney".to_string() } else { format!("piney - {}", self.volume.title()) })
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 720.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let instance = self.instance.get_or_insert_with(|| {
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(self.display.clone())))
        });
        let result = (|| -> Result<Display, String> {
            let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
            let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            }))
            .map_err(|e| e.to_string())?;
            let (device, queue) = pollster::block_on(
                adapter.request_device(&wgpu::DeviceDescriptor { label: Some("piney"), ..Default::default() }),
            )
            .map_err(|e| e.to_string())?;
            let size = window.inner_size();
            let mut config = surface
                .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                .ok_or("the surface is not supported by this adapter")?;
            let caps = surface.get_capabilities(&adapter);
            if let Some(f) = caps.formats.iter().find(|f| f.is_srgb()) {
                config.format = *f;
            }
            config.present_mode = wgpu::PresentMode::AutoVsync;
            if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
                config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
            }
            surface.configure(&device, &config);
            let presenter = Presenter::new(&device, config.format);
            let assets = self.assets.take().ok_or("assets already taken")?;
            self.gs = Some(Gs::new(device, queue, assets));
            Ok(Display { window: window.clone(), surface, config, presenter })
        })();
        match result {
            Ok(w) => self.win = Some(w),
            Err(e) => {
                eprintln!("no GPU: {e}");
                event_loop.exit();
                return;
            }
        }
        self.last = Instant::now();
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(gs), Some(w)) = (&self.gs, &mut self.win)
                    && size.width > 0
                    && size.height > 0
                {
                    w.config.width = size.width;
                    w.config.height = size.height;
                    w.surface.configure(gs.device(), &w.config);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = event.physical_key {
                    if pressed && matches!(code, KeyCode::F1 | KeyCode::Backquote) {
                        self.console.toggle();
                        if self.console.open {
                            let help = self.console_line("help");
                            self.console.set_commands(&help);
                        }
                        self.keyboard = input::Keyboard::default();
                        return;
                    }
                    if self.console.open {
                        if pressed
                            && let console::Key::Run(line) = self.console.key(code, event.text.as_deref(), self.ctrl)
                        {
                            let answer = self.console_line(&line);
                            self.console.say(answer);
                        }
                        return;
                    }
                }
                // Escape: the quit prompt, or back to the game from it.
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    if pressed && !event.repeat {
                        self.toggle_quit();
                    }
                    return;
                }
                self.keyboard.key(event.physical_key, event.state == ElementState::Pressed);
            }
            WindowEvent::ModifiersChanged(m) => self.ctrl = m.state().control_key(),
            WindowEvent::RedrawRequested => {
                self.tick();
                self.present();
                if self.exit {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.win {
            w.window.request_redraw();
        }
    }
}

/// What a mode asked for: sound to the audio, the rest (mode changes,
/// movies) printed until those parts exist.
fn handle(events: Vec<Event>, audio: Option<&Audio>) {
    for e in events {
        match e {
            Event::MovieAudio(pcm) => {
                if let Some(a) = audio {
                    a.movie_stream(pcm);
                }
                continue;
            }
            Event::StreamPcm(pcm) => {
                if let Some(a) = audio {
                    a.stream_pcm(pcm);
                }
                continue;
            }
            _ => {}
        }
        match (&e, audio) {
            (Event::Se(n), Some(a)) => a.se_on(*n as usize),
            (Event::SeNote { n, note }, Some(a)) => a.se_on_note(*n, *note),
            (Event::Se3d { n, pos, note, ear }, Some(a)) => a.se_on_3d(*n, ear.as_ref(), pos, *note),
            (Event::TobjSeLoopStart, Some(a)) => a.tobj_se_loop_start(),
            (Event::SoundGameOver, Some(a)) => a.game_over(),
            (Event::TobjSeLoop { pos, rate, ear }, Some(a)) => a.tobj_se_loop(ear, pos, *rate),
            (Event::DesktopBgm(n), Some(a)) => a.desktop_bgm(*n),
            (Event::Bgm(n), Some(a)) => a.bgm(*n),
            (Event::SoundFadeOut, Some(a)) => a.bgm_stop(),
            (Event::BgmStream(n), Some(a)) => {
                if let Err(e) = a.bgm_stream(*n) {
                    eprintln!("BGM.BIN track {n}: {e}");
                }
            }
            (Event::BgmStreamStop, Some(a)) => a.bgm_stream_stop(),
            (Event::SkillWords { event_running, char_type, char_id, sid, type_bit }, Some(a)) => {
                a.words_play(*event_running, *char_type, *char_id, *sid, *type_bit);
            }
            (Event::GameArea(area), Some(a)) => a.set_game_area(*area),
            (Event::SceneSound(s), Some(a)) => a.scene_sound(s),
            (Event::Volumes { main, se, bgm }, Some(a)) => a.set_volumes(*main, *se, *bgm),
            (Event::Voice { event, msg }, Some(a)) => {
                a.voice(*event, *msg);
            }
            (Event::VoiceStop, Some(a)) => a.voice_stop(),
            (Event::VoiceOptions { english, parody, talk_num }, Some(a)) => {
                a.set_voice_options(*english, *parody, *talk_num)
            }
            (Event::SqLoad(ctx), Some(a)) => a.sq_load(*ctx),
            (Event::BgmCtrl(w), Some(a)) => {
                a.bgm_ctrl(*w);
            }
            (Event::GameStart, Some(a)) => a.game_start(),
            (Event::InBattle(on), Some(a)) => a.set_in_battle(*on),
            (Event::PgBgm(p), Some(a)) => a.pg_bgm(*p),
            (Event::StreamMusic { num, size, after, game }, Some(a)) => a.stream_music(*num, *size, *after, *game),
            (Event::StreamBgm(r), Some(a)) => a.stream_bgm(*r),
            (Event::SqPlay(n), Some(a)) => a.sq_play(*n),
            (Event::SqStop(n), Some(a)) => a.sq_stop(*n),
            (Event::SqFade { seq, volume, time, mode }, Some(a)) => a.sq_fade(*seq, *volume, *time, *mode),
            (Event::MainVolume(v), Some(a)) => a.set_main_volume(*v),
            (Event::PortVolume { port, volume }, Some(a)) => a.port_volume(*port, *volume),
            (Event::HoldBgm, Some(a)) => a.hold_bgm(),
            (Event::AllSoundOff, Some(a)) => a.all_sound_off(),
            (Event::GameInterrupt, Some(a)) => a.game_interrupt(),
            (Event::MovieAudioStop, Some(a)) => a.movie_stream_stop(),
            // Nothing to hear without an output.
            (
                Event::Voice { .. }
                | Event::VoiceStop
                | Event::VoiceOptions { .. }
                | Event::SqLoad(_)
                | Event::BgmCtrl(_)
                | Event::BgmStream(_)
                | Event::BgmStreamStop
                | Event::SkillWords { .. }
                | Event::GameArea(_)
                | Event::SceneSound(_)
                | Event::GameStart
                | Event::InBattle(_)
                | Event::PgBgm(_)
                | Event::Se3d { .. }
                | Event::TobjSeLoopStart
                | Event::SoundGameOver
                | Event::TobjSeLoop { .. }
                | Event::SqPlay(_)
                | Event::SqStop(_)
                | Event::SqFade { .. }
                | Event::MainVolume(_)
                | Event::PortVolume { .. }
                | Event::HoldBgm
                | Event::AllSoundOff
                | Event::GameInterrupt
                | Event::MovieAudio(_)
                | Event::StreamPcm(_)
                | Event::StreamMusic { .. }
                | Event::StreamBgm(_)
                | Event::MovieAudioStop,
                None,
            ) => {}
            // The pad's motors are the app's.
            (Event::Actuate { .. }, _) => {}
            // The game on another disc: the loop that owns the disc does it.
            (Event::Boot(_), _) => {}
            _ => eprintln!("mode asks: {e:?}"),
        }
    }
}

/// One `--press` item: frames `from..=to` hold `buttons`, or push a stick
/// fully (x, y; 128 centred), or on frame `from` call a mode's test hook.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    from: u32,
    to: u32,
    buttons: Buttons,
    stick: Option<(Stick, u8, u8)>,
    hook: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stick {
    Left,
    Right,
}

/// The test hooks `--press F:NAME` can call (`Mode::hook`).
const HOOKS: [&str; 2] = ["gateout", "gofield"];

/// `F:BUTTON,...` or `F-G:BUTTON,...` to presses: BUTTON is a button name,
/// or `lup`, `ldown`, `lleft`, `lright` (`rup` ... for the right stick), or
/// a test hook (`gateout`, `gofield`).
/// The console's font: the game's own bitmap fonts with `xasc00`'s
/// palette; None (no console text) when they cannot be read.
fn console_fonts(iso: &std::path::Path, archive: &Archive) -> Option<piney_desktop::kanji::Fonts> {
    let volume = Iso::open(iso).ok()?.volume().ok()?;
    piney_desktop::assets::read_fonts(volume, archive).map_err(|e| eprintln!("the console's font: {e}")).ok()
}

/// A `--pad-log` file's pads back, one a frame: each frame's line has
/// `bytes LX LY RX RY buttons HEX` (the sticks' bytes and the held buttons,
/// all a `Raw` from the keyboard or a gamepad carries); other lines (the
/// gamepads' names) are skipped.
fn read_pad_log(path: &str) -> Result<std::collections::VecDeque<Raw>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut out = std::collections::VecDeque::new();
    for line in text.lines() {
        let Some(at) = line.find("| bytes ") else { continue };
        let w: Vec<&str> = line[at + 8..].split_whitespace().collect();
        let bad = || format!("{path}: a pad line I cannot read: {line}");
        if w.len() < 6 || w[4] != "buttons" {
            return Err(bad());
        }
        let byte = |i: usize| w[i].parse::<u8>().map_err(|_| bad());
        let bits = u32::from_str_radix(w[5], 16).map_err(|_| bad())?;
        out.push_back(Raw {
            buttons: Buttons(bits),
            lx: byte(0)?,
            ly: byte(1)?,
            rx: byte(2)?,
            ry: byte(3)?,
            ..Raw::default()
        });
    }
    Ok(out)
}

/// `dst` made a copy of the directory `src` (a memory card's: one level
/// of save directories and their files).
fn copy_card(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if dst.exists() {
        std::fs::remove_dir_all(dst)?;
    }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_card(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

fn parse_presses(s: &str) -> Result<Vec<Press>, String> {
    s.split(',')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (f, b) = p.split_once(':').ok_or(format!("{p}: want FRAME:BUTTON"))?;
            let (from, to) = match f.split_once('-') {
                Some((a, b)) => (a.parse(), b.parse()),
                None => (f.parse(), f.parse()),
            };
            let (from, to) = (from.map_err(|_| format!("{p}: bad frame"))?, to.map_err(|_| format!("{p}: bad frame"))?);
            let b = b.to_ascii_lowercase();
            let stick = match b.as_str() {
                "lup" => Some((Stick::Left, 128, 0)),
                "ldown" => Some((Stick::Left, 128, 255)),
                "lleft" => Some((Stick::Left, 0, 128)),
                "lright" => Some((Stick::Left, 255, 128)),
                "rup" => Some((Stick::Right, 128, 0)),
                "rdown" => Some((Stick::Right, 128, 255)),
                "rleft" => Some((Stick::Right, 0, 128)),
                "rright" => Some((Stick::Right, 255, 128)),
                _ => None,
            };
            let hook = HOOKS.iter().copied().find(|h| *h == b);
            let buttons = if stick.is_some() || hook.is_some() {
                Buttons::NONE
            } else {
                input::button(&b).ok_or(format!("{p}: unknown button"))?
            };
            Ok(Press { from, to, buttons, stick, hook })
        })
        .collect()
}

/// How the game runs, from the command line.
#[derive(Clone)]
struct Options {
    iso: PathBuf,
    /// Mails delivered and headlines posted by hand before the desktop.
    mail: Vec<usize>,
    news: Vec<usize>,
    /// The official event scripts run.
    scripts: bool,
    /// The directory standing for memory card slot 1.
    card: Option<PathBuf>,
    /// New Game's name entry runs.
    name_entry: bool,
    /// The logo movies the launcher played, which the title then skips.
    logos_played: i32,
    /// The options kept across the parts (`settings.toml`), in a window;
    /// None headless, which leaves the file alone.
    settings: Option<PathBuf>,
}

/// The disc a mode's events ask the game to start on (the launcher's).
fn boot_request(events: &[Event]) -> Option<PathBuf> {
    events.iter().find_map(|e| match e {
        Event::Boot(p) => Some(p.clone()),
        _ => None,
    })
}

/// A game started on `o.iso`: its archive, volume and mode.
struct Booted {
    archive: Arc<Archive>,
    volume: piney_data::volume::Volume,
    mode: Box<dyn Mode>,
}

fn boot(o: &Options) -> Result<Booted, String> {
    let mut disc = Iso::open(&o.iso).map_err(|e| e.to_string())?;
    let volume = piney_data::volume::of_disc(&mut disc).map_err(|e| e.to_string())?;
    let data = disc.read_path("DATA/DATA.BIN").map_err(|e| e.to_string())?;
    let archive = Arc::new(Archive::new(data).map_err(|e| e.to_string())?);
    let mut assets = Assets::new(archive.clone());
    let mode = make_mode("game", &mut assets, o)?;
    Ok(Booted { archive, volume, mode })
}

/// The sound for the disc at `path`, or None (said why).
fn open_audio(path: &std::path::Path) -> Option<Audio> {
    match Audio::open(path) {
        Ok(a) => {
            if a.is_silent() {
                eprintln!("no sound: {}", a.silent_reason.as_deref().unwrap_or("no output device"));
            }
            Some(a)
        }
        Err(e) => {
            eprintln!("no sound: {e}");
            None
        }
    }
}

fn make_mode(name: &str, assets: &mut Assets, o: &Options) -> Result<Box<dyn Mode>, String> {
    let (kind, arg) = name.split_once(':').unwrap_or((name, ""));
    match kind {
        // An archive no stream table lists, its streams played for looking
        // at them: `loose:STREAM/STRT.BIN:N` (Outbreak's unused scenes).
        "loose" => {
            let (path, n) = arg.rsplit_once(':').filter(|(_, n)| n.parse::<usize>().is_ok()).unwrap_or((arg, "0"));
            let path = if path.is_empty() { "STREAM/STRT.BIN" } else { path };
            Ok(Box::new(loose::LooseMode::new(&o.iso, path, n.parse().unwrap_or(0))?))
        }
        "game" => Ok(Box::new(session::Session::after_logos(
            o.iso.clone(),
            assets.archive().clone(),
            o.scripts,
            o.card.clone(),
            o.name_entry,
            o.logos_played,
            o.settings.clone(),
        )?)),
        "desktop" => {
            // The boot's save (ccSaveData::Init, its text from the disc),
            // the player named Kite.
            let mut disc = Iso::open(&o.iso).map_err(|e| format!("{}: {e}", o.iso.display()))?;
            let text = piney_desktop::InitText::from_disc(&mut disc).map_err(|e| format!("ccSaveData::Init: {e}"))?;
            let mut state = piney_desktop::SaveState::fresh_with(&text);
            // Until the event scripts run: mail delivered as `NewMail` does.
            for &m in &o.mail {
                state.save.new_mail(m);
            }
            // Headlines posted (`webnewsList[n]` = 1), likewise.
            for &n in &o.news {
                state.save.set_webnews(n, 1);
            }
            let archive = assets.archive().clone();
            let d = desktop::DesktopMode::new(o.iso.clone(), archive, state, o.scripts, o.card.clone(), o.name_entry)?;
            Ok(Box::new(d))
        }
        // Straight into Mac Anu on a new game's save (NewGame(0) over
        // ccSaveData::Init), the player named Kite.
        // Straight into The World on a new game's save (NewGame(0) over
        // ccSaveData::Init), the player named Kite: `world` in Mac Anu,
        // `world:N` in Root Town N (`saveData.lastTown`: 1 Dun Loireag),
        // `field:N` in story area N's field (as event 2's `area` and `scene`
        // leave it for N = 14); `field:N/TYPE,ROW[,HACK[,SEED]]` with the
        // field's type, weather row, hack flag and seed put in its
        // WORLD_MAN's place (another field's weather over the story area).
        "world" | "field" | "dungeon" => {
            let mut disc = Iso::open(&o.iso).map_err(|e| format!("{}: {e}", o.iso.display()))?;
            let mut state = world::new_game_state(&mut disc)?;
            if kind == "world" && !arg.is_empty() {
                let t: i32 = arg.parse().map_err(|_| format!("world:{arg}: want a town number"))?;
                if !piney_world::town_ported(t) {
                    return Err(format!(
                        "world:{t}: only Mac Anu (0), Dun Loireag (1) and Carmina Gade (2) are ported"
                    ));
                }
                state.save.set_u8(piney_data::save::offset::LAST_TOWN, t as u8);
            }
            // The event task as a new game brings it here (the boot, the
            // desktop's event 1 with the first mails read, the board), so
            // that event 2 opens; `--no-events` leaves it out. The fields
            // start without it.
            let vm =
                if kind == "world" && o.scripts { Some(world::new_game_events(&mut disc, &mut state)?) } else { None };
            let mut scene = piney_world::area::Scene::log_in(&mut state.save);
            let mut world_man = None;
            if kind != "world" {
                let (arg, look) = arg.split_once('/').map_or((arg, None), |(a, l)| (a, Some(l)));
                let n: i32 = arg.parse().map_err(|_| format!("{kind}:{arg}: want a story area number"))?;
                let crisis = state.save.u8(piney_data::save::offset::CRISIS) != 0;
                let mut town = scene.town;
                world_man = Some(match area::story_world_man(&mut disc, n, crisis) {
                    Ok(w) => w,
                    // Fields 1-8, the boss arenas, have no words: GO(1)
                    // keeps the WORLD_MAN of the area the party came from,
                    // area 27 (Theta, town 1) for Skeith's.
                    Err(_) if piney_world::evarea_b0::is_arena(n) => {
                        town = 1;
                        area::story_world_man(&mut disc, 27, crisis)?
                    }
                    Err(e) => return Err(e),
                });
                if let (Some(w), Some(look)) = (world_man.as_mut(), look) {
                    let v: Vec<u32> = look.split(',').filter_map(|x| x.parse().ok()).collect();
                    let [ty, row, ..] = v[..] else { return Err(format!("{kind}:{arg}/{look}: want TYPE,ROW")) };
                    w.field_type = ty;
                    w.weather = row;
                    w.hack = v.get(2).copied().unwrap_or(w.hack);
                    w.field_seed = v.get(3).copied().unwrap_or(w.field_seed);
                }
                scene.change_scene(1, town, n, -1, -1, -1, &mut state.save);
                // The dungeon as its entrance leaves it: ChangeArea(2, 0).
                if kind == "dungeon" {
                    scene.change_area(2, 0, &mut state.save);
                }
            }
            let s = session::Session::in_world(
                o.iso.clone(),
                assets.archive().clone(),
                o.card.clone(),
                state,
                vm,
                scene,
                world_man,
            )?;
            Ok(Box::new(s))
        }
        // Where event N opens, the story before it brought forward; the
        // scripts always run.
        "story" => {
            let n: i32 = arg.parse().map_err(|_| format!("story:{arg}: want an event, one of {:?}", start::POINTS))?;
            Ok(Box::new(start::session(o.iso.clone(), assets.archive().clone(), o.card.clone(), n)?))
        }
        "test-card" => {
            let file = if arg.is_empty() { "xddesk01" } else { arg };
            Ok(Box::new(TestCard::new(assets, file).ok_or(format!("{file}: not in DATA.BIN"))?))
        }
        _ => Err(format!("unknown mode {name}")),
    }
}

fn main() {
    // The disc: --iso, or a build's (--game, else the one piney-build left
    // in the port's folder), else the working tree's Infection image.
    let mut iso = PathBuf::from("work/infection/infection.iso");
    let mut iso_given = false;
    let mut game: Option<PathBuf> = None;
    let mut volume_arg: Option<piney_data::volume::Volume> = None;
    let mut card_given = false;
    let mut mode_name = String::from("game");
    let mut shot: Option<String> = None;
    let mut frames = 60u32;
    let mut presses = String::new();
    let mut mail: Vec<usize> = Vec::new();
    let mut news: Vec<usize> = Vec::new();
    let mut scripts = true;
    let mut mute = false;
    let mut pad_log: Option<String> = None;
    let mut replay: Option<String> = None;
    let mut every: Option<u32> = None;
    let mut console_lines: Option<String> = None;
    let mut quit_at: Option<u32> = None;
    let mut webp_out: Option<String> = None;
    let mut webp_every = 1u32;
    let mut webp_from = 0u32;
    let mut webp_quality = 80.0f32;
    let mut frames_given = false;
    let mut name_entry = true;
    let mut card = Some(PathBuf::from("work/memcard/slot1"));
    let mut args = std::env::args().skip(1).peekable();
    // `--shot` with no file after it: a headless run all the same.
    let mut shot_bare = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => {
                iso = args.next().map(PathBuf::from).unwrap_or(iso);
                iso_given = true;
            }
            "--game" => game = args.next().map(PathBuf::from),
            "--volume" => {
                volume_arg = match args.next().as_deref().map(str::to_ascii_lowercase).as_deref() {
                    Some("1" | "inf" | "infection") => Some(piney_data::volume::Volume::Inf),
                    Some("2" | "mut" | "mutation") => Some(piney_data::volume::Volume::Mut),
                    Some("3" | "out" | "outbreak") => Some(piney_data::volume::Volume::Out),
                    Some("4" | "qua" | "quarantine") => Some(piney_data::volume::Volume::Qua),
                    v => {
                        eprintln!("--volume wants 1-4 or inf, mut, out, qua, not {}", v.unwrap_or("nothing"));
                        std::process::exit(2);
                    }
                }
            }
            "--mode" => mode_name = args.next().unwrap_or(mode_name),
            "--shot" => match args.peek() {
                Some(n) if !n.starts_with("--") => shot = args.next(),
                _ => shot_bare = true,
            },
            "--frames" => {
                frames = args.next().and_then(|n| n.parse().ok()).unwrap_or(frames);
                frames_given = true;
            }
            "--replay" => replay = args.next(),
            "--every" => every = args.next().and_then(|n| n.parse().ok()),
            "--quit" => quit_at = args.next().and_then(|n| n.parse().ok()),
            "--webp" => webp_out = args.next(),
            "--webp-every" => webp_every = args.next().and_then(|n| n.parse().ok()).unwrap_or(1).max(1),
            "--webp-from" => webp_from = args.next().and_then(|n| n.parse().ok()).unwrap_or(0),
            "--webp-quality" => webp_quality = args.next().and_then(|n| n.parse().ok()).unwrap_or(80.0),
            "--console" => console_lines = args.next(),
            "--press" => presses = args.next().unwrap_or_default(),
            "--mail" => mail = args.next().unwrap_or_default().split(',').filter_map(|m| m.parse().ok()).collect(),
            "--no-events" => scripts = false,
            "--mute" => mute = true,
            "--dvd" => match args.peek() {
                Some(n) if !n.starts_with("--") => match n.parse::<f64>() {
                    Ok(speed) if speed > 0.0 => {
                        args.next();
                        dvd::set(speed);
                    }
                    _ => {
                        eprintln!("--dvd wants a speed (DVD 1x multiples, e.g. 3), not {n}");
                        return;
                    }
                },
                _ => dvd::set(dvd::DEFAULT_SPEED),
            },
            "--pad-log" => pad_log = args.next(),
            "--skip-name" => name_entry = false,
            "--voice" => match args.next().as_deref() {
                Some("en" | "english") => mode::set_voice_override(true),
                Some("jp" | "ja" | "japanese") => mode::set_voice_override(false),
                v => {
                    eprintln!("--voice wants en or jp, not {}", v.unwrap_or("nothing"));
                    std::process::exit(2);
                }
            },
            "--card" => {
                card = args.next().map(PathBuf::from);
                card_given = true;
            }
            "--no-card" => {
                card = None;
                card_given = true;
            }
            "--news" => news = args.next().unwrap_or_default().split(',').filter_map(|n| n.parse().ok()).collect(),
            "-h" | "--help" => {
                println!(
                    "piney-game [--iso PATH | --game DIR [--volume N]] [--mode MODE] [--no-events] [--mute] [--dvd [SPEED]] [--voice en|jp] [--card DIR | --no-card] [--mail N,...] [--news N,...] [--pad-log FILE] [--replay FILE]"
                );
                println!(
                    "--iso PATH: a disc image, or a disc of a build (DIR/outbreak.disc); --game DIR: a piney-build build (by default {}), its launcher when it holds more than one disc; --volume 1-4 (inf, mut, out, qua): that disc of the build, no launcher",
                    piney_data::pack::default_build().map_or_else(|| "none".into(), |d| d.display().to_string())
                );
                println!(
                    "piney-game [--iso PATH] [--mode MODE] --shot OUT.png [--frames N] [--press F[-G]:BUTTON,... | --replay FILE] [--every N]"
                );
                println!(
                    "--pad-log FILE: every frame's pad into FILE, the card as it was into FILE.card; --replay FILE: those pads again from a copy of that card (FILE.card-replay), then the live pad"
                );
                println!("--every N: with --shot, a picture every N frames too (OUT-FRAME.png)");
                println!("--quit N: with --shot, Escape's quit prompt opened at frame N (the game stops under it)");
                println!(
                    "--webp OUT.webp [--webp-from F] [--webp-every N] [--webp-quality Q]: the frames from F on (every Nth, lossy at Q, 80) as one animated WebP; runs headless like --shot (--shot OUT.webp does the same); a loose stream with no --frames records once through"
                );
                println!(
                    "--console \"CMD;CMD\": with --shot, the debug console's commands run first and the console shown (in the window, F1 opens it; help lists them)"
                );
                println!(
                    "modes: game (default), desktop, world[:TOWN], field:N, dungeon:N, story:N (N = 3, 4, 10-31), test-card[:FILE], loose[:PATH][:N] (an untabled stream archive, STREAM/STRT.BIN by default; START for the next)"
                );
                println!("--voice en|jp: the voices' language for this run, over the save's Options setting");
                println!(
                    "--dvd [SPEED]: loads between areas take about the PS2 disc's time (at SPEED times DVD 1x, by default {}), so the loading screens stay up",
                    dvd::DEFAULT_SPEED
                );
                return;
            }
            other => {
                eprintln!("unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    // A build of the discs (piney-build): which disc, or the launcher.
    let mut launch: Option<(PathBuf, launcher::Discs)> = None;
    let build = match &game {
        Some(dir) => Some(dir.clone()),
        None if !iso_given => piney_data::pack::default_build().filter(|d| !piney_data::pack::volumes_in(d).is_empty()),
        None => None,
    };
    if volume_arg.is_some() && build.is_none() {
        eprintln!("--volume picks a disc of a build: give --game DIR, or build one with piney-build");
        std::process::exit(2);
    }
    if let Some(dir) = &build {
        let found = piney_data::pack::volumes_in(dir);
        if found.is_empty() {
            eprintln!("{}: no build there (piney-build makes one)", dir.display());
            std::process::exit(1);
        }
        let mut discs: launcher::Discs = Default::default();
        for (v, path) in &found {
            discs[*v as usize] = Some(path.clone());
        }
        iso = match volume_arg {
            Some(v) => match discs[v as usize].clone() {
                Some(p) => p,
                None => {
                    eprintln!("{}: the build has no {}", dir.display(), v.title());
                    std::process::exit(1);
                }
            },
            // The launcher plays from Outbreak's disc when there is one.
            None => discs[piney_data::volume::Volume::Out as usize].clone().unwrap_or_else(|| found[0].1.clone()),
        };
        if volume_arg.is_none() && mode_name == "game" && found.len() > 1 {
            launch = Some((dir.clone(), discs));
        } else if volume_arg.is_none() {
            iso = found[0].1.clone();
        }
        // A build's saves live in the port's folder, not the working tree.
        if !card_given {
            card = piney_data::pack::home().map(|h| h.join("memcard").join("slot1"));
        }
    }
    // The port's data from the disc (`plans/build-data.md`): a disc image's
    // made once into the port's folder, while the executable can still be
    // read; a build's discs hold their own.
    let playing: Vec<PathBuf> = match &launch {
        Some((_, discs)) => discs.iter().flatten().cloned().collect(),
        None => vec![iso.clone()],
    };
    for path in &playing {
        match piney_build::image_data(path) {
            Ok(true) => println!("{}: the port's data made from the disc (once)", path.display()),
            Ok(false) => {}
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        let ready = Iso::open(path).map(|mut d| piney_data::pack::has_port_data(&mut d)).unwrap_or(false);
        if !ready {
            eprintln!(
                "{}: no port data of version {} (a build made before it): run piney-build again",
                path.display(),
                piney_data::pack::DATA_VERSION
            );
            std::process::exit(1);
        }
    }
    // Each volume's tables come from its disc's port data.
    for path in &playing {
        if let Ok(v) = Iso::open(path).and_then(|mut d| d.volume()) {
            piney_data::store::use_disc(v, path.clone());
        }
    }
    // The port reads the disc's data files, never its boot executable.
    piney_data::iso::deny_executable();
    let mut disc = match Iso::open(&iso) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {e}", iso.display());
            std::process::exit(1);
        }
    };
    // Which of the four discs (plans/volumes.md); the window's title names it.
    let volume = match piney_data::volume::of_disc(&mut disc) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {e}", iso.display());
            std::process::exit(1);
        }
    };
    let data = match disc.read_path("DATA/DATA.BIN") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {e}", iso.display());
            std::process::exit(1);
        }
    };
    let archive = Arc::new(Archive::new(data).expect("DATA.BIN"));
    // An empty directory is a formatted card with no save on it yet.
    if let Some(dir) = &card
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("{}: {e}; no memory card", dir.display());
        card = None;
    }
    // --pad-log keeps the card as the run starts beside the log; --replay
    // plays from a copy of it, so the replay's saves leave it as it was.
    if let (Some(path), Some(dir)) = (&pad_log, &card)
        && let Err(e) = copy_card(dir, std::path::Path::new(&format!("{path}.card")))
    {
        eprintln!("{path}.card: {e}");
    }
    let replay = match &replay {
        Some(path) => {
            let snap = PathBuf::from(format!("{path}.card"));
            if snap.is_dir() {
                let copy = PathBuf::from(format!("{path}.card-replay"));
                match copy_card(&snap, &copy) {
                    Ok(()) => card = Some(copy),
                    Err(e) => eprintln!("{}: {e}", copy.display()),
                }
            } else {
                eprintln!("{}: no card kept with the log; the current card is used", snap.display());
            }
            match read_pad_log(path) {
                Ok(r) => {
                    eprintln!("replay: {} frames from {path}", r.len());
                    r
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            }
        }
        None => std::collections::VecDeque::new(),
    };
    let mut assets = Assets::new(archive.clone());
    // A window keeps the options across the parts; a headless run leaves
    // the file alone.
    let windowed = !(shot.is_some() || shot_bare || webp_out.is_some());
    let settings = windowed.then(settings::path).flatten();
    let options = Options {
        iso: iso.clone(),
        mail,
        news,
        scripts,
        card: card.clone(),
        name_entry,
        logos_played: 0,
        settings: settings.clone(),
    };
    let launching = launch.is_some();
    let made = match &launch {
        Some((dir, discs)) => Ok(Box::new(
            launcher::LauncherMode::new(
                discs.clone(),
                Some(dir.join("launcher.txt")),
                console_fonts(&iso, &archive),
                Some((&archive, volume)),
            )
            .with_main_volume(settings.as_deref().and_then(|p| {
                settings::Settings::load(p, &piney_desktop::SaveState::fresh().save).map(|s| s.main_volume())
            })),
        ) as Box<dyn Mode>),
        None => make_mode(&mode_name, &mut assets, &options),
    };
    let mut mode = match made {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    // A shot named `.webp` is the recording; `--webp` or a bare `--shot`
    // alone still runs headless, with no PNG.
    if shot.as_deref().is_some_and(|s| s.to_ascii_lowercase().ends_with(".webp")) {
        webp_out = shot.take();
    }
    if shot.is_some() || shot_bare || webp_out.is_some() {
        let out = shot.clone().unwrap_or_default();
        let presses = match parse_presses(&presses) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        };
        let mut gs = match Gs::headless(assets) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("no GPU: {e}");
                std::process::exit(1);
            }
        };
        let mut pad = Pad::default();
        let mut replay = replay;
        if !frames_given && !replay.is_empty() {
            frames = replay.len() as u32;
        }
        // `--every`'s pictures are named after the PNG, or the WebP.
        let named = if out.is_empty() { webp_out.clone().unwrap_or_else(|| "shot".into()) } else { out.clone() };
        let stem = named.strip_suffix(".png").or_else(|| named.strip_suffix(".webp")).unwrap_or(&named).to_string();
        // A mode that ends by itself (a loose stream), with no `--frames`,
        // runs once through (up to ten minutes).
        let to_end = !frames_given && mode.ends();
        if to_end {
            frames = 60 * 60 * 10;
        }
        let mut console = console::Console::new(None);
        if console_lines.is_some() {
            console = console::Console::new(console_fonts(&iso, &archive));
            console.open = true;
        }
        let mut prompt: Option<quit::QuitPrompt> = None;
        // `--webp`: the frames kept, and the vertical blanks since the last.
        let mut recorder: Option<webp::Recorder> = None;
        let mut since = 0u32;
        let record = |gs: &Gs, f: u32, rate: u32, recorder: &mut Option<webp::Recorder>, since: &mut u32| {
            if webp_out.is_none() || f < webp_from {
                return;
            }
            *since += rate;
            if !(f - webp_from).is_multiple_of(webp_every) {
                return;
            }
            let (w, h) = gs.target_size();
            if recorder.is_none() {
                match webp::Recorder::new(w, h, webp_quality) {
                    Ok(r) => *recorder = Some(r),
                    Err(e) => {
                        eprintln!("{e}");
                        return;
                    }
                }
            }
            if let Some(r) = recorder.as_mut()
                && let Err(e) = r.add(&gs.read_back(), *since)
            {
                eprintln!("{e}");
            }
            *since = 0;
        };
        let mut ran = 0u32;
        for f in 0..frames {
            // The console's commands once the mode has run a frame.
            if f == 1
                && let Some(lines) = &console_lines
            {
                for l in lines.split(';') {
                    console.say(format!("> {l}"));
                    let answer = mode.console(l);
                    println!("console> {l}\n{answer}");
                    console.say(answer);
                }
            }
            let analog = presses.iter().any(|p| p.stick.is_some());
            let mut raw = Raw { analog, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            if let Some(r) = replay.pop_front() {
                raw = r;
            }
            for p in presses.iter().filter(|p| (p.from..=p.to).contains(&f)) {
                raw.buttons |= p.buttons;
                match p.stick {
                    Some((Stick::Left, x, y)) => (raw.lx, raw.ly) = (x, y),
                    Some((Stick::Right, x, y)) => (raw.rx, raw.ry) = (x, y),
                    None => {}
                }
                if let Some(h) = p.hook.filter(|_| f == p.from)
                    && !mode.hook(h)
                {
                    eprintln!("frame {f}: the mode has no hook {h}");
                }
            }
            // Escape's prompt from frame `quit_at`: the game's frame then
            // held, the prompt run on the presses after it.
            if quit_at == Some(f)
                && let Some(fonts) = console_fonts(&iso, &archive)
            {
                prompt = Some(quit::QuitPrompt::open(volume, &archive, fonts, mode.save_copy()));
            }
            if let Some(q) = prompt.as_mut().filter(|q| q.holding()) {
                let (answer, _) = q.frame(&raw);
                println!("frame {f}: quit prompt {answer:?}");
                let frame = q.picture(mode.frame_rate());
                gs.render(&frame);
                record(&gs, f, mode.frame_rate(), &mut recorder, &mut since);
                ran = f + 1;
                if answer != quit::Answer::Open {
                    gs.render(&q.backdrop());
                    prompt = None;
                }
                if let Some(n) = every.filter(|&n| n > 0 && (f + 1) % n == 0) {
                    let _ = n;
                    let (w, h) = gs.target_size();
                    let path = format!("{stem}-{:06}.png", f + 1);
                    if let Err(e) = std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())) {
                        eprintln!("{path}: {e}");
                    }
                }
                continue;
            }
            pad.read(&raw);
            let mut frame = mode.step(&pad);
            if to_end && mode.finished() {
                break;
            }
            if console.open {
                console.draw(&mut frame);
            }
            let events = mode.take_events();
            let boot_path = boot_request(&events);
            handle(events, None);
            // Every frame, as the window draws them.
            gs.set_overlay(mode.archive());
            gs.render(&frame);
            // The prompt opened this frame: this picture held under it.
            if let Some(q) = prompt.as_mut() {
                let (w, h) = gs.target_size();
                q.hold(Some((w as u16, h as u16, gs.read_back())));
            }
            record(&gs, f, mode.frame_rate(), &mut recorder, &mut since);
            ran = f + 1;
            if let Some(n) = every.filter(|&n| n > 0 && (f + 1) % n == 0) {
                let _ = n;
                let (w, h) = gs.target_size();
                let path = format!("{stem}-{:06}.png", f + 1);
                if let Err(e) = std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())) {
                    eprintln!("{path}: {e}");
                }
                println!("{path}: {}", mode.title());
            }
            // The launcher's choice, once its last frame is drawn: the game
            // on that disc from here.
            if let Some(path) = boot_path {
                match boot(&Options {
                    iso: path.clone(),
                    logos_played: launcher::LOGOS.len() as i32,
                    ..options.clone()
                }) {
                    Ok(b) => {
                        gs.set_archive(b.archive.clone());
                        mode = b.mode;
                        println!("launcher: {} at frame {}", b.volume.title(), f + 1);
                    }
                    Err(e) => eprintln!("{}: {e}", path.display()),
                }
            }
        }
        let full = |p: &str| std::fs::canonicalize(p).map_or_else(|_| p.to_string(), |a| a.display().to_string());
        if !out.is_empty() {
            let (w, h) = gs.target_size();
            if let Err(e) = std::fs::write(&out, piney_gs::png::encode(w, h, &gs.read_back())) {
                eprintln!("{out}: {e}");
                std::process::exit(1);
            }
            println!("{} after {ran} frames -> {}", mode.title(), full(&out));
        } else {
            println!("{} after {ran} frames", mode.title());
        }
        if let (Some(r), Some(path)) = (recorder, webp_out.as_deref()) {
            match r.finish(path, mode.frame_rate() * webp_every) {
                Ok(n) => println!("{n} frames -> {}", full(path)),
                Err(e) => eprintln!("{e}"),
            }
        }
        return;
    }

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    // No gilrs filters: its dead zone rescales the stick and can zero one
    // axis on its own; the game has a dead zone of its own.
    let gilrs = gilrs::GilrsBuilder::new()
        .with_default_filters(false)
        .build()
        .map_err(|e| eprintln!("no gamepad support: {e}"))
        .ok();
    let pad_log = pad_log.and_then(|path| match std::fs::File::create(&path) {
        Ok(f) => {
            let mut w = std::io::BufWriter::new(f);
            if let Some(g) = &gilrs {
                use std::io::Write;
                let _ = writeln!(w, "{}", input::gamepads(g));
            }
            eprintln!("pad log: {path}");
            Some(w)
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            None
        }
    });
    let audio = if mute { None } else { open_audio(&iso) };
    let mut app = App {
        mode,
        assets: Some(assets),
        gs: None,
        win: None,
        display: event_loop.owned_display_handle(),
        instance: None,
        keyboard: input::Keyboard::default(),
        gilrs,
        pad: Pad::default(),
        last: Instant::now(),
        vblanks: 0.0,
        audio,
        pad_log,
        replay,
        frame_no: 0,
        console: console::Console::new(console_fonts(&iso, &archive)),
        iso,
        archive,
        card,
        volume,
        actuator: piney_input::actuator::Actuator::default(),
        rumble: None,
        ctrl: false,
        quit: None,
        exit: false,
        options,
        mute,
        launching,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("{e}");
    }
}
