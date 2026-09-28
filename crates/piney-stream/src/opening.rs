//! Mutation's opening, event 101's `stream 24`: `Func_str0710` (MUT main
//! 0x0019c140-0x0019df6c) over `str0710` (`docs/engine/stream.md`, "Stream
//! 24's effect task"). Beside the [`Ctrl`] it holds the ending's table walker
//! and puff bursts over `eventObjTbl_0710` ([`crate::ending`]) and two blocks
//! of text lines (new in Mutation, 0x00187960-0x00187d40) that fade in and
//! out on cues 700 / 709 and 710 / 719. The lines are the executable's, read
//! into the build ([`Tables::text`]); the stream's host draws them
//! ([`Str0710::text_draws`]).

use piney_world::ee::{self, F, ONE};

use crate::effect::{Ctrl, Cue, Draws, EFFECT_LAYER, FADE_WHITE, MAX_CUES, Rand};
use crate::ending::{EventKind, EventObj, PartDraw, Parts, World};

/// `StreamDemoFuncTbl`'s entry (MUT main 0x00367210, stream 24).
pub const STR0710: &str = "str0710";
/// The effect file (`str0710e`, else `str0710ep`) and its puff.
pub const EFF_FILE: &str = "str0710e";

/// The lines' layer: `ccLayer::Init(1000, sysLayer's view)`.
pub const TEXT_LAYER: i16 = 1000;
/// Each line's `ccKanji::Init(3, 8)`.
pub const KANJI_L: u32 = 3;
pub const KANJI_PACKETS: usize = 8;
/// `ccSpriteColorTable[2]` (+0x10): the lines' colour; the fade gives the
/// alpha.
pub const TEXT_COLOUR: usize = 2;

const LINES_MAX: usize = 10;
/// A line's bytes at most (`SetText` cuts it there).
const LINE_BYTES: usize = 40;
/// +0x290 and +0x294: the width per byte and the line height the cues
/// place the lines by.
const GLYPH_W: i32 = 20;
const LINE_H: i32 = 24;
/// The block's centre line on the screen: `175 - total / 4`.
const TOP: i32 = 175;
const CENTRE_X: i32 = 256;
/// The fades' steps a frame: 0.05 in, -0.05 out.
const FADE_IN: F = 0x3d4c_cccd;
const FADE_OUT: F = 0xbd4c_cccd;
/// 128.0: the alpha at 1.0.
const ALPHA_FULL: F = 0x4300_0000;

/// `Func_str0710`'s feedback, shades and the fade it starts on.
const REFLEX_1050: (F, u32) = (0x3f80_a3d7, 0x5880_8080);
const REFLEX_55: (F, u32) = (0x3f81_47ae, 0x5080_8080);
const REFLEX_X5: (F, u32) = (0x3f82_8f5c, 0x4080_8080);
const REFLEX_65: (F, u32) = (0x3f80_a3d7, 0x4080_8080);
/// Cue 55's feedback depth and its shades' (`SetShade(0, 896, 0, 7 - i,
/// 7 - i, 250.0, 0x40808080)`).
const FEEDBACK_Z_55: F = 0x447a_0000;
const SHADE_Z_55: F = 0x437a_0000;
const SHADE_COLOUR_55: u32 = 0x4080_8080;
const BLACK: u32 = 0x8000_0000;

/// What the task reads (MUT main): `eventObjTbl_0710` (0x00366e30, five
/// entries and its end) and the two blocks' texts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tables {
    /// The walker's table, as the ending's reads (no rocks here).
    pub events: crate::ending::Tables,
    /// Cue 700's and cue 710's text, each `[normal, Parody Mode]`
    /// (`saveData.parodyFlag` picks), lines split at `\n`.
    pub text: [[Vec<u8>; 2]; 2],
}

impl Tables {
    /// The volume's (`tables::stream`): none on Infection.
    pub fn read(volume: piney_data::volume::Volume) -> Tables {
        let t = piney_data::tables::stream::of(volume);
        let events =
            crate::ending::Tables { entries: crate::ending::entries_of(t.opening_events()), rock_scale: [0; 3] };
        let s = |k: usize| t.opening_text()[k].map_or_else(Vec::new, piney_data::tables::sjis::encode);
        Tables { events, text: [[s(0), s(1)], [s(2), s(3)]] }
    }
}

/// One of a block's ten line slots: `+0x00` its `ccKanji`, `+0x08` the
/// length, `+0x0c` the text, `+0x38` x, `+0x3c` y; and the sprite's alpha.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    pub text: Vec<u8>,
    pub x: i32,
    pub y: i32,
    pub alpha: u8,
}

/// A block of text lines (0x298 bytes): ten slots, the fade (+0x280 alpha,
/// +0x284 its step, +0x288 where it stops) and the lines in use (+0x28c).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLines {
    pub slots: [Line; LINES_MAX],
    pub count: usize,
    pub alpha: F,
    pub step: F,
    pub target: F,
}

impl Default for TextLines {
    /// As the task leaves it: alpha 0, the fade still, no lines.
    fn default() -> Self {
        TextLines { slots: Default::default(), count: 0, alpha: 0, step: 0, target: 0 }
    }
}

impl TextLines {
    /// `SetText(s)` (0x00187bd0): the old lines emptied; `s` split at `\n`
    /// into at most ten lines of at most 40 bytes, line `i` at (0, 24 i).
    pub fn set_text(&mut self, s: &[u8]) {
        for l in &mut self.slots[..self.count] {
            l.text.clear();
        }
        self.count = 0;
        let mut p = 0;
        for i in 0..LINES_MAX {
            let start = p;
            while p < s.len() && s[p] != b'\n' && s[p] != 0 {
                p += 1;
            }
            let n = (p - start).min(LINE_BYTES);
            let l = &mut self.slots[i];
            l.text = s[start..start + n].to_vec();
            l.x = 0;
            l.y = LINE_H * i as i32;
            if p >= s.len() || s[p] == 0 {
                self.count = i + 1;
                return;
            }
            p += 1;
        }
        self.count = LINES_MAX;
    }

    /// `SetLines(s, last)` (0x00187cf0): as [`TextLines::set_text`] with
    /// the lines given (NUL-separated in the game), `last + 1` of them.
    pub fn set_lines(&mut self, lines: &[Vec<u8>]) {
        for l in &mut self.slots[..self.count] {
            l.text.clear();
        }
        self.count = lines.len().min(LINES_MAX);
        for (i, t) in lines.iter().take(LINES_MAX).enumerate() {
            let l = &mut self.slots[i];
            l.text = t[..t.len().min(LINE_BYTES)].to_vec();
            l.x = 0;
            l.y = LINE_H * i as i32;
        }
    }

    /// Cues 700 and 710 after `SetText`: [`TextLines::place`].
    fn show(&mut self, s: &[u8]) {
        self.set_text(s);
        self.place();
    }

    /// Each line centred by its bytes (`256 - 20 n / 2 / 2`), the block
    /// about row 175; alpha 0, fading in.
    pub(crate) fn place(&mut self) {
        for l in &mut self.slots[..self.count] {
            l.x = CENTRE_X - GLYPH_W * l.text.len() as i32 / 2 / 2;
        }
        let h = self.count as i32 * LINE_H / 2 / 2;
        for (i, l) in self.slots[..self.count].iter_mut().enumerate() {
            l.y = LINE_H * i as i32 + TOP - h;
        }
        self.alpha = 0;
        self.step = FADE_IN;
        self.target = ONE;
    }

    /// Cues 709 and 719: fading out from 1.
    pub(crate) fn hide(&mut self) {
        self.alpha = ONE;
        self.step = FADE_OUT;
        self.target = 0;
    }

    /// The fade's frame (0x00187ac0): nothing while still; else the alpha
    /// stepped and held at its stop, and each line's sprite alpha
    /// `fptosi(128 alpha)`.
    pub fn fade(&mut self) {
        if ee::eq(0, self.step) {
            return;
        }
        self.alpha = ee::add(self.alpha, self.step);
        let past =
            if ee::lt(self.step, 0) { ee::lt(self.alpha, self.target) } else { !ee::le(self.alpha, self.target) };
        if past {
            self.alpha = self.target;
            self.step = 0;
        }
        let a = ee::to_int(ee::mul(ALPHA_FULL, self.alpha)) as u8;
        for l in &mut self.slots[..self.count] {
            l.alpha = a;
        }
    }

    /// The lines drawn this frame: none while the alpha is 0.
    pub fn shown(&self) -> &[Line] {
        if ee::eq(0, self.alpha) { &[] } else { &self.slots[..self.count] }
    }
}

/// A line to draw: `ccKanji::Disp(text, -1, 1.0, 1.0)` at (x, y) on
/// [`TEXT_LAYER`], colour [`TEXT_COLOUR`] at `alpha`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextDraw {
    pub text: Vec<u8>,
    pub x: i32,
    pub y: i32,
    pub alpha: u8,
}

/// `Func_str0710`'s state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str0710 {
    pub ctrl: Ctrl,
    pub event: EventObj,
    pub text: [TextLines; 2],
    tables: Tables,
    parody: bool,
    parts: Parts,
    draws: Vec<PartDraw>,
    texts: Vec<TextDraw>,
}

impl Str0710 {
    /// `Func_str0710` started: its objects (the noise bands' `Init` drawing
    /// from `rand`), the walker on its first entry, both blocks at alpha 0,
    /// and a one-frame black fade.
    pub fn new(tables: &Tables, world: &dyn World, parody: bool, rand: &mut Rand) -> Str0710 {
        let mut ctrl = Ctrl::new(Some(EFFECT_LAYER), rand);
        ctrl.fade.entry(1, BLACK, BLACK);
        Str0710 {
            ctrl,
            event: EventObj::new(&tables.events, 0, world),
            text: Default::default(),
            tables: tables.clone(),
            parody,
            parts: Parts::default(),
            draws: Vec::new(),
            texts: Vec::new(),
        }
    }

    /// The parts' draws of the last pass.
    pub fn part_draws(&self) -> &[PartDraw] {
        &self.draws
    }

    /// The text lines of the last pass, block 700's then block 710's.
    pub fn text_draws(&self) -> &[TextDraw] {
        &self.texts
    }

    /// One pass of its loop: the scene frame's changes (465 the shades
    /// off, 1050 the feedback, 1155 both off), the cues last queued first,
    /// the parts, both blocks' fades, then [`Ctrl::pass`].
    pub fn step(&mut self, world: &dyn World, cues: &[Cue], frame_now: u32, paused: bool, rand: &mut Rand) -> Draws {
        self.draws.clear();
        self.texts.clear();
        let c = &mut self.ctrl;
        match frame_now {
            465 => c.shades.count = 0,
            1050 => c.feedback.reflex(REFLEX_1050.0, REFLEX_1050.1),
            1155 => {
                c.feedback.off();
                c.shades.count = 0;
            }
            _ => {}
        }
        for &Cue { param: cue, .. } in cues[..cues.len().min(MAX_CUES)].iter().rev() {
            self.cue(cue, world, rand);
        }
        self.parts.step(world, &self.tables.events, rand, &mut self.draws);
        for t in &mut self.text {
            t.fade();
            self.texts.extend(t.shown().iter().map(|l| TextDraw {
                text: l.text.clone(),
                x: l.x,
                y: l.y,
                alpha: l.alpha,
            }));
        }
        self.ctrl.pass(paused, rand)
    }

    fn cue(&mut self, cue: u32, world: &dyn World, rand: &mut Rand) {
        let c = &mut self.ctrl;
        match cue {
            602..=610 => {
                let Some(o) = self.event.set_obj(&self.tables.events, cue, world) else { return };
                if let EventKind::Eff(e) = self.tables.events.entries[self.event.at].kind {
                    let at = world.position(o);
                    self.parts.puffs(at, &e, rand);
                }
            }
            700 => self.text[0].show(&self.tables.text[0][usize::from(self.parody)]),
            709 => self.text[0].hide(),
            710 => self.text[1].show(&self.tables.text[1][usize::from(self.parody)]),
            719 => self.text[1].hide(),
            600 => c.fade.entry(20, 0, 0x4000_0000),
            601 => c.fade.entry(20, 0x4000_0000, BLACK),
            800 => c.fade.entry(30, BLACK, 0),
            999 => c.fade.entry(1, BLACK, BLACK),
            900 => c.fade.entry(16, FADE_WHITE.1, FADE_WHITE.0),
            901..=904 => c.fade.entry(10, FADE_WHITE.1, FADE_WHITE.0),
            65 => c.feedback.reflex_timed(REFLEX_65.0, REFLEX_65.1, 0, 15),
            56 => {
                c.shades.count = 0;
                c.feedback.off();
            }
            55 => {
                c.shades.list.clear();
                for i in 0..2u8 {
                    let s = piney_desktop::noiz::Sampling::shade_own(7 - i, 7 - i, SHADE_Z_55, SHADE_COLOUR_55);
                    c.shades.list.push(s);
                }
                c.shades.count = 2;
                c.feedback.reflex(REFLEX_55.0, REFLEX_55.1);
                c.feedback.z = FEEDBACK_Z_55;
            }
            _ => match cue % 10 {
                1 => c.noise_start(rand),
                2 => c.noise_on = false,
                3 => c.reverse = true,
                4 => c.reverse = false,
                5 => c.feedback.reflex(REFLEX_X5.0, REFLEX_X5.1),
                6 => c.feedback.flag = 2,
                7 => c.fade.entry(1, FADE_WHITE.0, FADE_WHITE.1),
                8 => c.fade.entry(1, FADE_WHITE.1, FADE_WHITE.0),
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_text_splits_and_cuts() {
        let mut t = TextLines::default();
        let long = [b'a'; 45];
        let mut s = b"ab\n".to_vec();
        s.extend_from_slice(&long);
        s.extend_from_slice(b"\ncd");
        t.set_text(&s);
        assert_eq!(t.count, 3);
        assert_eq!(t.slots[0].text, b"ab");
        assert_eq!(t.slots[1].text.len(), 40);
        assert_eq!(t.slots[2].text, b"cd");
        assert_eq!(t.slots[2].y, 48);
    }

    /// Two lines of 19 and 20 bytes: x `256 - 20 n / 4`, the block about
    /// row 175 (`175 - 48 / 4`).
    #[test]
    fn show_places_and_fades_in() {
        let mut t = TextLines::default();
        t.show(b"aaaaaaaaaaaaaaaaaaa\nbbbbbbbbbbbbbbbbbbbb");
        assert_eq!((t.slots[0].x, t.slots[0].y), (256 - 95, 163));
        assert_eq!((t.slots[1].x, t.slots[1].y), (256 - 100, 187));
        assert!(t.shown().is_empty());
        t.fade();
        assert_eq!(t.slots[0].alpha, 6);
        for _ in 0..30 {
            t.fade();
        }
        assert_eq!((t.alpha, t.step, t.slots[1].alpha), (ONE, 0, 128));
        t.hide();
        for _ in 0..30 {
            t.fade();
        }
        assert_eq!((t.alpha, t.step), (0, 0));
        assert!(t.shown().is_empty());
    }
}
