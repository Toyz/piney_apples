//! Not the game's: the port's launcher for a build of several discs
//! (`piney-build`, `piney_data::pack`). It plays the four-part selector left
//! unused on Outbreak's disc (`STREAM/STRT.BIN`'s `trial_v2st`,
//! docs/disc/outbreak.md), which the port drives: the chosen row glows, a
//! volume the build lacks is dimmed, the demo's labels hidden; without
//! Outbreak, a list in the game's font. Before it, the title's three logo
//! movies ([`LOGOS`]); under it, the disc's copyright and the port's credit
//! ([`PORTED_BY`]).

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::volume::Volume;
use piney_desktop::anm::{Anm, Ctx};
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::{Fonts, Kanji, Names};
use piney_draw::Frame;
use piney_input::{Buttons, Pad};

use crate::mode::{Event, Mode};
use crate::movie::{Played, Playing};
use crate::stream::StreamPlayer;

/// The logo movies played before the choice: the title's three
/// (`piney_demo::names::LOGO_MOVIES`), Bandai's, CyberConnect2's and
/// Project .hack's.
pub const LOGOS: [&str; 3] =
    [piney_demo::names::LOGO_MOVIES[0].0, piney_demo::names::LOGO_MOVIES[1].0, piney_demo::names::LOGO_MOVIES[2].0];

/// The selector's scene in `STREAM/STRT.BIN`.
const SELECTOR: &str = "trial_v2st";
const STRT: &str = "STREAM/STRT.BIN";
/// The title screen's sounds (its bank, loaded once the logos are over):
/// the cursor, OK, refused.
const SE_MOVE: i32 = 6;
const SE_OK: i32 = 4;
const SE_REFUSED: i32 = 20;
/// Frames between the choice and the game.
const CHOSEN_FRAMES: u32 = 24;
/// The launcher's frames (two vblanks each) an icon takes to turn once:
/// the title's 120 at one vblank.
const ICON_TURN: u32 = 60;
/// Where the credit's text starts: its right end 24 in from the screen's
/// (the font's type 2 half-width cells are 7.5 across here), on the
/// copyright's second line.
const CREDIT_X: f32 = 512.0 - 24.0 - 7.5 * PORTED_BY.len() as f32;
const CREDIT_Y: f32 = 424.0;
/// A dimmed row's transparency.
const DIM: f32 = 0.3;
/// The chosen row's glows at their brightest.
const GLOW: f32 = 0.3;
/// The highlight group's place from its row's: the row's text model is
/// centred (x -12300 to 12300) and the highlight's starts at its origin (x
/// -4795.3 to 19800), so the two texts line up 7,504.7 units left of the
/// row's origin.
const HIGHLIGHT_DX: f32 = -12300.0 + 4795.3;
const OK: Buttons = Buttons(Buttons::CROSS.0 | Buttons::START.0);

/// The port's credit, bottom right under the rows.
pub const PORTED_BY: &str = "Ported by helba";

/// The title's copyright line: `OBJ_xdt_cop_00_` of `ANM_xdt_ne09` (the
/// title's "digits and copyright behind", `m_back[2]`), drawn as the title
/// draws it, through `ANM_xdtcam00`, with the animation's other objects
/// (the digits) off.
struct Copyright {
    file: Rc<SceneFile>,
    anm: Anm,
    camera: Option<piney_desktop::camera::Camera>,
}

impl Copyright {
    fn read(archive: &piney_data::archive::Archive, volume: Volume) -> Option<Copyright> {
        let file = Rc::new(SceneFile::read(archive, piney_demo::names::title_file(volume)).ok()?);
        let cop = file.ccs.find_object("OBJ_xdt_cop_00_")?;
        let mut anm = Anm::new();
        anm.set(&file, "ANM_xdt_ne09");
        anm.forward();
        for (obj, _, _) in anm.instances_all() {
            anm.set_disp(obj, obj == cop);
        }
        let mut cam = Anm::new();
        cam.set(&file, piney_demo::names::CAMERA);
        cam.forward();
        Some(Copyright { file, anm, camera: cam.camera })
    }
}

/// Which build disc is which volume, None where the build lacks it.
pub type Discs = [Option<PathBuf>; 4];

pub struct LauncherMode {
    discs: Discs,
    /// The logo playing, and the next to play (all played: [`LOGOS`]'s
    /// length).
    logo: Option<Playing>,
    next_logo: usize,
    /// Where the last choice is kept (`launcher.txt` in the build).
    memo: Option<PathBuf>,
    cursor: usize,
    selector: Option<StreamPlayer>,
    /// The fallback list's font, and the credit's.
    fonts: Option<Fonts>,
    copyright: Option<Copyright>,
    /// The kept options' main volume, for the title's music (a new save's
    /// without them).
    main_volume: Option<i16>,
    /// The opening is over and the rows answer.
    menu: bool,
    /// The row chosen and the frames until its game starts.
    chosen: Option<(usize, u32)>,
    frame: u32,
    events: Vec<Event>,
}

impl LauncherMode {
    /// A launcher over `discs`; `memo` keeps the last choice. The
    /// selector plays from Outbreak's disc when the build has it, else
    /// `fonts` draw a list.
    /// `archive`: the launcher's disc's `DATA.BIN` and volume, for the
    /// title's copyright line.
    pub fn new(
        discs: Discs,
        memo: Option<PathBuf>,
        fonts: Option<Fonts>,
        archive: Option<(&piney_data::archive::Archive, Volume)>,
    ) -> LauncherMode {
        let first = discs.iter().position(Option::is_some).unwrap_or(0);
        let remembered = memo
            .as_ref()
            .and_then(|m| std::fs::read_to_string(m).ok())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|&r| r < 4 && discs[r].is_some());
        LauncherMode {
            discs,
            logo: None,
            next_logo: 0,
            memo,
            cursor: remembered.unwrap_or(first),
            selector: None,
            fonts,
            copyright: archive.and_then(|(a, v)| Copyright::read(a, v)),
            main_volume: None,
            menu: false,
            chosen: None,
            frame: 0,
            events: Vec::new(),
        }
    }

    /// The main volume the kept options give, if any.
    pub fn with_main_volume(mut self, v: Option<i16>) -> Self {
        self.main_volume = v;
        self
    }

    /// The logos' disc: Outbreak's, else the first the build has.
    fn logo_disc(&self) -> Option<&Path> {
        self.discs[Volume::Out as usize].as_deref().or_else(|| self.discs.iter().flatten().next().map(PathBuf::as_path))
    }

    /// A frame of the logos while they last: the next one opened as the
    /// last ends, a push stopping them all. None once they are over, and
    /// then the selector (or the list) set up.
    fn logos(&mut self, pad: &Pad) -> Option<Frame> {
        while self.next_logo < LOGOS.len() || self.logo.is_some() {
            if let Some(p) = &mut self.logo {
                let stop = (OK | Buttons::CIRCLE).bits();
                match p.step_stopped_by(pad, stop) {
                    Played::Showing(frame) => return Some(frame),
                    Played::Ended => {}
                    Played::Skipped => self.next_logo = LOGOS.len(),
                }
                self.logo = None;
                self.events.push(Event::MovieAudioStop);
                continue;
            }
            let path = LOGOS[self.next_logo];
            self.next_logo += 1;
            let Some(disc) = self.logo_disc().map(Path::to_path_buf) else { continue };
            match Playing::open(&disc, path, false, &mut self.events) {
                Ok(p) => self.logo = Some(p),
                // Counted as played.
                Err(e) => eprintln!("launcher: {e}"),
            }
        }
        if self.selector.is_none() && !self.menu {
            // The selector's scene has no sound of its own: the title's
            // bank (its cursor, OK and refusal sounds), and its music as
            // the title starts it after the logos (`LogoMain`'s LogoAct
            // 4: the main volume, sequence 0), at a new save's volume.
            let main = self
                .main_volume
                .unwrap_or_else(|| piney_desktop::SaveState::fresh().save.i16(piney_data::save::offset::MAIN_VOL));
            self.events.extend([
                Event::AllSoundOff,
                Event::SqLoad(piney_audio::SqContext::Title),
                Event::MainVolume(i32::from(main)),
                Event::SqPlay(0),
            ]);
            self.selector =
                self.discs[Volume::Out as usize].clone().and_then(|out| match selector(&out, &mut self.events) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        eprintln!("launcher: {e}; a list instead");
                        None
                    }
                });
            self.menu = self.selector.is_none();
        }
        None
    }

    /// The next row up or down with a disc, if any.
    fn next(&self, down: bool) -> Option<usize> {
        let rows: Vec<usize> = if down { (self.cursor + 1..4).collect() } else { (0..self.cursor).rev().collect() };
        rows.into_iter().find(|&r| self.discs[r].is_some())
    }

    fn input(&mut self, pad: &Pad) {
        if !self.menu || self.chosen.is_some() {
            return;
        }
        let moved = if pad.repeat.contains(Buttons::UP) {
            self.next(false)
        } else if pad.repeat.contains(Buttons::DOWN) {
            self.next(true)
        } else {
            None
        };
        if let Some(r) = moved {
            self.cursor = r;
            self.events.push(Event::Se(SE_MOVE));
        }
        if pad.push.intersects(OK) {
            if self.discs[self.cursor].is_some() {
                self.events.push(Event::Se(SE_OK));
                self.chosen = Some((self.cursor, CHOSEN_FRAMES));
            } else {
                self.events.push(Event::Se(SE_REFUSED));
            }
        }
    }

    /// Each row's objects drawn as the choice has them. The scene has two
    /// of most objects: the opening's (`tristr00`, animated: the rows
    /// `men_X1` and their icons) and a still set at the origin, never shown
    /// by the scene, which the demo's code would have brought out. Of the
    /// still set the port uses each row's highlight, `men_X0` (the row's
    /// text) with its two glows `lig_X0` and `lig_X1`: put on the chosen
    /// row, pulsing. A row the build lacks is dimmed.
    fn dress(&mut self) {
        let menu = self.menu;
        let cursor = self.chosen.map_or(self.cursor, |(r, _)| r);
        let flash = self.chosen.is_some();
        let pulse = 0.7 + 0.3 * (self.frame as f32 * 0.12).sin();
        let have: Vec<bool> = self.discs.iter().map(Option::is_some).collect();
        let frame = self.frame;
        let Some(scene) = self.selector.as_mut().and_then(|p| p.stream_mut().scene_mut()) else { return };
        let first = |scene: &piney_stream::scene::Scene, name: &str| scene.nodes_named(name).first().copied();
        for (row, &have) in have.iter().enumerate() {
            let lit = menu && row == cursor;
            // The row's own text at full; the glows faint, pulsing, and
            // flashing once the row is chosen.
            let (text, glow) = if !lit {
                (0.0, 0.0)
            } else if flash {
                (1.0, if (frame / 2).is_multiple_of(2) { 0.6 } else { 0.1 })
            } else {
                (1.0, GLOW * pulse)
            };
            let Some(anim) = first(scene, &format!("OBJ_xdt_men_{row}1_")) else { continue };
            let mut at = scene.world(anim);
            at[3][0] = (f32::from_bits(at[3][0]) + HIGHLIGHT_DX).to_bits();
            if let Some(group) = first(scene, &format!("OBJ_xdt_men_{row}0_")) {
                scene.place(group, lit.then_some(at));
                scene.set_alpha(group, Some(text));
            }
            for k in 0..2 {
                if let Some(g) = first(scene, &format!("OBJ_xdt_lig_{row}{k}_")) {
                    scene.set_alpha(g, Some(glow));
                }
            }
            let dim = (!have).then_some(DIM);
            scene.set_alpha(anim, dim);
            for k in 0..3 {
                if let Some(i) = first(scene, &format!("OBJ_xdt_ico_{row}{k}_")) {
                    scene.set_alpha(i, dim);
                }
            }
            // The icon turns once the rows answer, as the title's do
            // (`ANM_xdt_ic00`: its inner object a turn about y every 120
            // frames at 60 a second).
            if menu && let Some(i) = first(scene, &format!("OBJ_xdt_ico_{row}1_")) {
                let turn = (frame % ICON_TURN) as f32 / ICON_TURN as f32 * std::f32::consts::TAU;
                scene.turn_y(i, turn.to_bits());
            }
        }
    }

    /// The copyright line and the port's credit over `frame`, their
    /// uploads numbered after the frame's own.
    fn credits(&self, frame: &mut Frame) {
        let mut view = piney_desktop::view::View::default();
        if let Some(cam) = self.copyright.as_ref().and_then(|c| c.camera) {
            view.set_camera(&cam);
        }
        let mut ctx = Ctx::new(view);
        ctx.uploads = std::mem::take(&mut frame.uploads);
        if let Some(c) = &self.copyright {
            piney_demo::draw::draw_anm(&mut ctx, &c.file, &c.anm);
        }
        if let Some(fonts) = &self.fonts {
            let view = piney_desktop::message::menu_view();
            let mut k = Kanji::init(2, 96);
            k.dx = CREDIT_X;
            k.dy = CREDIT_Y;
            k.colour = [128, 128, 128, 128];
            ctx.disp(fonts, &mut k, 0, &view, PORTED_BY.as_bytes(), &Names::default());
        }
        let m = ctx.finish();
        frame.uploads = m.uploads;
        frame.cmds.extend(m.cmds);
    }

    /// Without the selector: the volumes as a list in the game's font.
    fn list(&self) -> Frame {
        let mut f = Frame::new();
        let Some(fonts) = &self.fonts else { return f };
        let mut ctx = piney_desktop::anm::Ctx::new(piney_desktop::view::View::default());
        let view = piney_desktop::message::menu_view();
        let names = Names::default();
        let line = |ctx: &mut piney_desktop::anm::Ctx, y: f32, text: &str, colour: [u8; 4]| {
            let mut k = Kanji::init(2, 96);
            k.dx = 128.0;
            k.dy = y;
            k.colour = colour;
            ctx.disp(fonts, &mut k, 0, &view, text.as_bytes(), &names);
        };
        line(&mut ctx, 120.0, "Choose a game", [128, 128, 128, 128]);
        for (row, v) in Volume::ALL.iter().enumerate() {
            let chosen = row == self.chosen.map_or(self.cursor, |(r, _)| r);
            let colour = if self.discs[row].is_none() {
                [48, 48, 48, 128]
            } else if chosen {
                [128, 128, 40, 128]
            } else {
                [128, 128, 128, 128]
            };
            let mark = if chosen { "> " } else { "  " };
            line(&mut ctx, 170.0 + 28.0 * row as f32, &format!("{mark}{}", v.title()), colour);
        }
        f = ctx.finish();
        f.clear = piney_draw::Rgba::BLACK;
        f
    }
}

/// The selector's stream, from Outbreak's disc at `out`: held on its last
/// frame.
fn selector(out: &Path, events: &mut Vec<Event>) -> Result<StreamPlayer, String> {
    let mut disc = Iso::open(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let raw = disc.read_path(STRT).map_err(|e| format!("{STRT}: {e}"))?;
    let entries = piney_stream::load::scan(&raw).map_err(|e| format!("{STRT}: {e}"))?;
    let stream = piney_stream::load::loose_streams(entries)
        .into_iter()
        .find(|s| s.iter().any(|e| e.name == SELECTOR))
        .ok_or_else(|| format!("{STRT}: no {SELECTOR}"))?;
    let mut p = StreamPlayer::loose(out, STRT, stream, events)?;
    p.stream_mut().hold_end(true);
    Ok(p)
}

impl Mode for LauncherMode {
    fn step(&mut self, pad: &Pad) -> Frame {
        if let Some(frame) = self.logos(pad) {
            return frame;
        }
        self.frame += 1;
        if let Some((row, left)) = self.chosen {
            if left == 0 {
                if let Some(path) = self.discs[row].clone() {
                    if let Some(m) = &self.memo {
                        let _ = std::fs::write(m, format!("{row}\n"));
                    }
                    self.events.push(Event::Boot(path));
                }
                self.chosen = None;
                self.menu = false;
            } else {
                self.chosen = Some((row, left - 1));
            }
        }
        // A push during the opening goes to the rows.
        if !self.menu
            && let Some(p) = self.selector.as_mut()
            && pad.push.intersects(OK | Buttons::CIRCLE)
        {
            p.stream_mut().fast_forward();
        }
        self.input(pad);
        let Some(p) = self.selector.as_mut() else { return self.list() };
        if p.stream_mut().scene_mut().is_some_and(|s| s.ended()) {
            self.menu = true;
        }
        self.dress();
        let Some(p) = self.selector.as_mut() else { return Frame::new() };
        // The stream sees no buttons: nothing skips it.
        let mut frame = p.step(&Pad::default(), &mut self.events).unwrap_or_default();
        self.credits(&mut frame);
        frame
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// The logos at the title's rate (1), the selector at its stream's.
    fn frame_rate(&self) -> u32 {
        if self.logo.is_some() { 1 } else { 2 }
    }

    fn archive(&self) -> Option<Arc<Archive>> {
        self.selector.as_ref().map(StreamPlayer::archive)
    }

    fn title(&self) -> String {
        if self.logo.is_some() {
            return format!("launcher - logo {}", self.next_logo);
        }
        let row = self.chosen.map_or(self.cursor, |(r, _)| r);
        format!("launcher - {} - frame {}", Volume::ALL[row].title(), self.frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The logos are silent; once they are over (a push stops them) the
    /// selector sounds as the title does: its music from the title's bank,
    /// and the cursor's sound over it.
    #[test]
    fn selector_sounds_like_the_title() {
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/outbreak/outbreak.iso");
        if !out.exists() {
            eprintln!("outbreak.iso not present; skipped");
            return;
        }
        let audio = piney_audio::Audio::headless(&out).unwrap();
        let mut l =
            LauncherMode::new([Some(out.clone()), Some(out.clone()), Some(out.clone()), None], None, None, None);
        let mut pad = Pad::default();
        let mut buf = vec![0i16; 2 * 800];
        let (mut logo_heard, mut menu_heard) = (0, 0);
        for f in 0..400u32 {
            let b = match f {
                40 | 120 => Buttons::CROSS,
                200 | 240 => Buttons::UP,
                _ => Buttons::NONE,
            };
            pad.read(&piney_input::Raw { buttons: b, ..piney_input::Raw::default() });
            let before = l.logo.is_some();
            l.step(&pad);
            let logo = before && l.logo.is_some();
            crate::handle(l.take_events(), Some(&audio));
            audio.frame();
            audio.render(&mut buf);
            let loud = buf.iter().any(|&x| x != 0);
            if logo {
                logo_heard += u32::from(loud);
            } else if f > 150 {
                menu_heard += u32::from(loud);
            }
        }
        assert_eq!(logo_heard, 0, "the logos are silent");
        assert!(l.menu, "the rows answer");
        assert!(menu_heard > 100, "the selector heard for {menu_heard} frames");
    }

    /// The rows' icons turning, shot headless every 5 frames of a turn
    /// once the rows answer, to hold against the title's
    /// (`PINEY_SHOTS=DIR`, default `/mnt/data/claude/scratch/icons`).
    #[test]
    #[ignore]
    fn selector_icon_shots() {
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/outbreak/outbreak.iso");
        if !out.exists() {
            return;
        }
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/icons".into());
        std::fs::create_dir_all(&dir).unwrap();
        let mut l =
            LauncherMode::new([Some(out.clone()), Some(out.clone()), Some(out.clone()), Some(out)], None, None, None);
        let mut pad = Pad::default();
        let mut gs = None;
        let mut shots = 0;
        for f in 0..2000u32 {
            let b = if f.is_multiple_of(20) && !l.menu { Buttons::CROSS } else { Buttons::NONE };
            pad.read(&piney_input::Raw { buttons: b, ..piney_input::Raw::default() });
            let frame = l.step(&pad);
            l.take_events();
            if !l.menu || !l.frame.is_multiple_of(5) {
                continue;
            }
            let gs =
                gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(l.archive().unwrap())).unwrap());
            gs.set_overlay(l.archive());
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/l-{:03}.png", l.frame % ICON_TURN);
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            shots += 1;
            if shots == 12 {
                break;
            }
        }
        assert_eq!(shots, 12);
    }

    /// Infection's menu sound effects by number, each played alone on a
    /// fresh headless audio: which are heard (a diagnostic).
    #[test]
    #[ignore]
    fn menu_sounds_heard() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            return;
        }
        for n in [2i32, 4, 6, 7, 17, 18, 19] {
            let audio = piney_audio::Audio::headless(&iso).unwrap();
            crate::handle(vec![Event::Se(n)], Some(&audio));
            let mut buf = vec![0i16; 2 * 800];
            let mut peak = 0i32;
            for _ in 0..30 {
                audio.frame();
                audio.render(&mut buf);
                peak = peak.max(buf.iter().map(|&x| i32::from(x).abs()).max().unwrap_or(0));
            }
            eprintln!("Se({n}): peak {peak}");
        }
    }

    /// Once the opening is over the four rows stand one under the other,
    /// each with a still highlight to bring onto it.
    #[test]
    fn selector_rows_at_the_end() {
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/outbreak/outbreak.iso");
        if !out.exists() {
            eprintln!("outbreak.iso not present; skipped");
            return;
        }
        let mut events = Vec::new();
        let mut p = selector(&out, &mut events).unwrap();
        // The scene starts on the first frame.
        p.step(&Pad::default(), &mut events);
        p.stream_mut().fast_forward();
        assert!(p.stream_mut().scene_mut().unwrap().ended());
        let scene = p.stream_mut().scene_mut().unwrap();
        // Each row: the opening's animated line, one under the other, and a
        // still highlight group to bring onto it.
        let mut ys = Vec::new();
        for row in 0..4 {
            let anim = scene.nodes_named(&format!("OBJ_xdt_men_{row}1_"));
            assert_eq!(anim.len(), 2, "row {row}: the opening's and the still copy");
            ys.push(f32::from_bits(scene.world(anim[0])[3][1]));
            assert_eq!(scene.nodes_named(&format!("OBJ_xdt_men_{row}0_")).len(), 1);
            assert_eq!(scene.nodes_named(&format!("OBJ_xdt_lig_{row}1_")).len(), 1);
        }
        assert_eq!(ys, vec![9000.0, 4500.0, 0.0, -4500.0]);
    }
}
