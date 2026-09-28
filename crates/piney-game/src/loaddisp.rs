//! `ccLoadDisp` (main 0x0019ac00-0x0019c430): the loading display between
//! areas. `ccFileListLoad` puts it up (`ccLoadDispInit`, 0x0019ad70)
//! whenever the new scene's file list has a file the old one lacked
//! (`loadCheck`, 0x00165530), unless one is already up:
//!
//! ```text
//! a town                                   the town's card and the server's
//! a field from a town, no stream playing   the area's name and the server's
//!   (the gate hack's stream 107 is one; field 8 has none)
//! a type 4 field's dungeon from a town,    the area's name and the server's
//!   no stream playing
//! a field from its dungeon, or a dungeon   the "NOW LOADING" animation only
//! ```
//!
//! Its task (`ccLoadDispTh`, 0x0019c290, priority 17) draws, each frame
//! until `ccSnd.gameStart`, the card fading in (`allDisp(0)`: alpha up by 8
//! to 128) and `xdl_load`'s `ANM_xdl_lod1` (with its camera) on the world's
//! layer; then, with a card, 30 frames at full (`allDisp(1)`) and 17 fading
//! out by 8 (`allDisp(2)`), without the animation. The port's loads take no
//! time, so the card mostly shows those 47 frames, over the new area's
//! fade in, as the game's does after its load; `--dvd` holds the new scene
//! for about the disc's time first (`crate::dvd`).
//!
//! The card (`allDisp`, 0x0019ba50): `xdl_tit`'s title in two cells (256 x
//! 128 at (56, 128), 144 x 128 from V 128 at (312, 128)); the texts in
//! `ccKanji` type 2 (ef12x20), white: the town's two lines (`townNameTbl`)
//! centred at y 168 and 196, or the area's three words centred at y 182;
//! the server's symbol (`serverNameTbl`) at (364, 40) and "Server" 24 to its
//! right, in the server's colour (`serverDisp`).

use std::rc::Rc;

use piney_data::archive::Archive;
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_data::tables::loaddisp;
use piney_data::tables::sjis::encode;
use piney_data::volume::Volume;
use piney_data::world::{Server, Town};
use piney_desktop::anm::{Anm, Ctx};
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::{Fonts, Kanji, Kt, Names, str_width};
use piney_desktop::message::{WindowTexture, menu_view};
use piney_desktop::sprite::Sprite;
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_world::area::Scene;

/// The loading screen's files and animation.
const LOAD_FILE: &str = "xdl_load";
const LOAD_ANM: &str = "ANM_xdl_lod1";
const TITLE_FILE: &str = "xdl_tit";
const TITLE_TEX: &str = "TEX_xdl_tit";

/// Frames the card holds at full, then fades over, once the area has
/// started.
const HOLD: u32 = 30;
const FADE: u32 = 17;

/// The layers: the animation under the card (the world's layer 0), the
/// card's own (`ccLayer::Init(255)`) above everything but the fade.
const ANM_LAYER: i16 = 0;
const CARD_LAYER: i16 = 253;

/// `ccLoadDisp +0x170`: which card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 0: the town's card.
    Town,
    /// 1: the area's name.
    Field,
    /// 2: the animation only.
    Plain,
}

/// `ccLoadDispInit`'s choice for the scene being entered (`scene`, whose
/// `area_prev` is the one left): `field_type` is `WORLD_MAN::GetFieldType()`
/// and `stream` whether a stream plays (`strPtr`, the gate hack's).
pub fn kind(scene: &Scene, field_type: i32, stream: bool) -> Option<Kind> {
    match scene.area {
        0 => Some(Kind::Town),
        1 if scene.area_prev != 0 => (scene.area_prev == 2).then_some(Kind::Plain),
        1 if stream || scene.field == 8 => None,
        1 => Some(Kind::Field),
        2 if scene.area_prev == 0 && field_type == 4 && scene.dungeon == 0 && !stream => Some(Kind::Field),
        2 => Some(Kind::Plain),
        _ => None,
    }
}

/// A line of the card: text at (x, y) in a colour of `ccSpriteColorTable`.
#[derive(Clone, Debug)]
struct Line {
    text: Vec<u8>,
    x: f32,
    y: f32,
    colour: usize,
}

/// The loading display while it is up.
pub struct LoadDisp {
    kind: Kind,
    /// +0x178: the card's alpha.
    alpha: i32,
    /// Frames since the area started (`gameStart`), once it has.
    after: Option<u32>,
    anm: Option<Anm>,
    title: Option<WindowTexture>,
    lines: Vec<Line>,
    fonts: Option<Fonts>,
}

impl LoadDisp {
    /// `ccLoadDispInit` with its set-ups (`setupInit`, `townSetup`,
    /// `fieldSetup`, `serverSetup`). `words` are the area's three keywords
    /// (`WORLD_MAN.A`, `B`, `C` through `GetWordParamPtr`).
    /// Its words are the volume's (`piney_data::tables::loaddisp`: `townNameTbl`
    /// main 0x00311730, `serverNameTbl` 0x00311770, "Server" 0x00353f98, the
    /// words' separator 0x00353fa0).
    pub fn new(kind: Kind, archive: &Archive, volume: Volume, scene: &Scene, words: [&str; 3]) -> LoadDisp {
        let fonts = piney_desktop::assets::read_fonts(volume, archive).ok();
        let anm = SceneFile::read(archive, LOAD_FILE).ok().map(|f| {
            let mut a = Anm::new();
            a.set(&Rc::new(f), LOAD_ANM);
            a
        });
        let title =
            if kind == Kind::Plain { None } else { WindowTexture::read_named(archive, TITLE_FILE, TITLE_TEX, None) };
        let names = Names::default();
        // The widths need only the glyphs, not `CLT_xasc00`.
        let bare;
        let metrics = match &fonts {
            Some(f) => f,
            None => {
                bare = Fonts::of(volume, Vec::new());
                &bare
            }
        };
        let centred = |text: Vec<u8>, y: f32| {
            let w = str_width(metrics, &text, Kt::LargeProportional, &names);
            Line { x: 256.0 - w as f32 / 2.0, y, text, colour: 7 }
        };
        let mut lines = Vec::new();
        match kind {
            Kind::Town => {
                let card = Town::from_index(scene.town.clamp(0, 4)).unwrap_or(Town::MacAnu).card();
                lines.push(centred(encode(card.name), 168.0));
                lines.push(centred(encode(card.sub), 196.0));
            }
            Kind::Field => {
                // The words' separator (0x00353fa0): one space.
                let sep = b" ";
                let mut text: Vec<u8> = Vec::new();
                for (k, w) in words.iter().enumerate() {
                    if k > 0 {
                        text.extend_from_slice(sep);
                    }
                    text.extend(w.chars().map(|c| c as u8));
                }
                lines.push(centred(text, 182.0));
            }
            Kind::Plain => {}
        }
        if kind != Kind::Plain {
            // serverDisp's colours by server: Δ 2, Θ 1, Λ 4, Σ 6, Ω 5.
            let colour = match scene.server {
                0 => 2,
                1 => 1,
                2 => 4,
                3 => 6,
                4 => 5,
                _ => 7,
            };
            let server = Server::from_index(scene.server.clamp(0, 4)).unwrap_or(Server::Delta);
            lines.push(Line { text: encode(server.symbol()), x: 364.0, y: 40.0, colour });
            lines.push(Line { text: encode(*loaddisp::SERVER), x: 388.0, y: 40.0, colour });
        }
        LoadDisp { kind, alpha: 0, after: None, anm, title, lines, fonts }
    }

    /// One frame of `ccLoadDispTh`: `started` once the new area has set
    /// `ccSnd.gameStart`. False when the display is done.
    pub fn frame(&mut self, started: bool) -> bool {
        match self.after {
            None if !started => {
                // allDisp(0), then the animation forward.
                self.alpha = (self.alpha + 8).min(128);
                if let Some(a) = &mut self.anm {
                    a.forward();
                }
                true
            }
            None => {
                if self.kind == Kind::Plain {
                    return false;
                }
                self.after = Some(0);
                self.alpha = 128;
                true
            }
            Some(n) if n + 1 < HOLD => {
                self.after = Some(n + 1);
                self.alpha = 128;
                true
            }
            Some(n) if n + 1 < HOLD + FADE => {
                self.after = Some(n + 1);
                self.alpha = (self.alpha - 8).max(0);
                true
            }
            Some(_) => false,
        }
    }

    /// The display over `frame`: the animation while loading, the card.
    pub fn draw(&self, frame: &mut Frame) {
        let mut ctx = Ctx::new(View::default());
        ctx.uploads = std::mem::take(&mut frame.uploads);
        if self.after.is_none()
            && let Some(a) = &self.anm
        {
            let mut v = View::default();
            if let Some(cam) = a.camera {
                v.set_camera(&cam);
            }
            a.draw_on(&mut ctx, ANM_LAYER, &v);
        }
        if self.kind != Kind::Plain {
            let view = menu_view();
            let a = self.alpha.clamp(0, 255) as u8;
            // allDisp's order: the texts (townDisp or fieldDisp, then
            // serverDisp: ccKanji::Disp sends at once), then the title's
            // SendPacketS, which the layer's prepending draws under them.
            if let Some(fonts) = &self.fonts {
                let names = Names::default();
                for l in &self.lines {
                    let mut k = Kanji::init(3, 10);
                    k.kt = Kt::LargeProportional;
                    let c = SPRITE_COLOR_TABLE[l.colour.min(SPRITE_COLOR_TABLE.len() - 1)];
                    k.colour = [c[0], c[1], c[2], a];
                    k.dx = l.x;
                    k.dy = l.y;
                    ctx.disp(fonts, &mut k, CARD_LAYER, &view, &l.text, &names);
                }
            }
            if let Some(t) = &self.title {
                let mut s = Sprite::mask(CARD_LAYER, t.tex.clone(), t.tex_h, 2);
                s.colour = [128, 128, 128, a];
                (s.wu, s.wv, s.wi, s.su, s.sv, s.sx, s.sy, s.dx, s.dy) = (0, 0, 1, 256, 128, 256.0, 128.0, 56.0, 128.0);
                s.make_packet(0, &view);
                (s.wu, s.wv, s.wi, s.su, s.sv, s.sx, s.sy, s.dx, s.dy) =
                    (0, 2048, 1, 144, 128, 144.0, 128.0, 312.0, 128.0);
                s.make_packet(0, &view);
                s.send(&mut ctx.layers);
            }
        }
        let f = ctx.finish();
        frame.uploads = f.uploads;
        frame.cmds.extend(f.cmds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(area: i32, area_prev: i32) -> Scene {
        let mut s = Scene::init();
        s.area = area;
        s.area_prev = area_prev;
        s.field = 14;
        s.dungeon = -1;
        s
    }

    fn display(kind: Kind) -> LoadDisp {
        LoadDisp { kind, alpha: 0, after: None, anm: None, title: None, lines: Vec::new(), fonts: None }
    }

    /// `ccLoadDispTh`: the card fades in by 8 while the scene loads, holds
    /// 30 frames at 128 once it has started and fades out over 17; the
    /// plain display ends as the scene starts.
    #[test]
    fn its_frames() {
        let mut d = display(Kind::Town);
        for k in 1..=20 {
            assert!(d.frame(false));
            assert_eq!(d.alpha, (8 * k).min(128));
        }
        let mut alphas = Vec::new();
        while d.frame(true) {
            alphas.push(d.alpha);
        }
        assert_eq!(alphas.len(), 47);
        assert!(alphas[..30].iter().all(|&a| a == 128));
        assert_eq!(&alphas[30..], &[120, 112, 104, 96, 88, 80, 72, 64, 56, 48, 40, 32, 24, 16, 8, 0, 0]);
        let mut p = display(Kind::Plain);
        assert!(p.frame(false));
        assert!(!p.frame(true));
    }

    /// `ccLoadDispInit`'s cases.
    #[test]
    fn which_card() {
        assert_eq!(kind(&scene(0, 1), 0, false), Some(Kind::Town));
        assert_eq!(kind(&scene(1, 0), 0, false), Some(Kind::Field));
        assert_eq!(kind(&scene(1, 0), 0, true), None, "the gate hack's stream");
        assert_eq!(kind(&scene(1, 2), 0, false), Some(Kind::Plain));
        assert_eq!(kind(&scene(2, 1), 0, false), Some(Kind::Plain));
        let mut d = scene(2, 0);
        d.dungeon = 0;
        assert_eq!(kind(&d, 4, false), Some(Kind::Field));
        let mut f8 = scene(1, 0);
        f8.field = 8;
        assert_eq!(kind(&f8, 0, false), None);
    }
}
