//! Not the game's: the debug console. F1 (or the ` key) opens a line typed on
//! the keyboard; Enter hands it to the mode ([`crate::mode::Mode::console`])
//! and keeps the answer; Escape or F1 again closes it, while the game runs on
//! with its pad held neutral. It draws in the game's `ef8x16` font on a dark
//! band ([`Console::overlay`], or [`Console::draw`] under `--shot`). The keys
//! are the usual line-editing ones, Up / Down for the lines before, Page Up /
//! Down to scroll, Tab to complete; `help` lists the commands. Answers wider
//! than the window wrap at their spaces. A scroll bar on the right takes the
//! mouse: the wheel, a drag of the thumb, a click on the track for a page.

use std::cell::Cell;

use piney_desktop::anm::Ctx;
use piney_desktop::kanji::{Fonts, Kanji, Names};
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_gs::Overlay;

/// Answer lines kept.
const LOG_KEEP: usize = 1000;
/// Answer lines `--shot` shows.
const SHOT_LINES: usize = 8;
/// The longest line taken.
const LINE_MAX: usize = 120;
/// Lines typed that the history file keeps.
const HISTORY_KEEP: usize = 200;
/// `--shot`'s band: drawn over everything (the font layer is 240, the
/// menu's 242), its lines' spacing in logical units.
const LAYER: i16 = 250;
const LINE_H: f32 = 18.0;
/// The overlay's line height and margin, in window pixels at scale 1.
const PX_LINE: u32 = 18;
const PX_MARGIN: u32 = 6;
/// A help line's second column at least, in pixels at scale 1 (further
/// right when a command is wider), and the gap after the widest command.
const HELP_COLUMN: u32 = 140;
const HELP_GAP: u32 = 12;
/// The scroll bar's width and its thumb's least height, at scale 1; the
/// rows one wheel notch scrolls.
const BAR_W: u32 = 10;
const THUMB_MIN: u32 = 16;
const WHEEL_ROWS: i32 = 3;

/// Text colours: an answer, a line typed, the prompt, the band's rule.
const ANSWER: [u8; 3] = [0xd8, 0xd8, 0xd8];
const TYPED: [u8; 3] = [0xff, 0xdc, 0x78];
const PROMPT: [u8; 3] = [0xff, 0xff, 0xff];
const RULE: [u8; 4] = [0x70, 0x78, 0xa0, 0xff];
const BAND: [u8; 4] = [0x0c, 0x0c, 0x14, 0xd8];
const TRACK: [u8; 4] = [0x20, 0x22, 0x30, 0xff];
const THUMB: [u8; 4] = [0x70, 0x78, 0xa0, 0xff];
const THUMB_HELD: [u8; 4] = [0xb0, 0xb8, 0xe0, 0xff];

/// A piece of a drawn row: its x, its text, its colour.
type Piece = (u32, Vec<u8>, [u8; 3]);

/// Where the last overlay put the scroll bar, in window pixels, and the
/// rows it scrolls over.
#[derive(Clone, Copy, Default)]
struct Bar {
    x: u32,
    w: u32,
    top: u32,
    bottom: u32,
    /// The answers' rows, and how many the band shows.
    total: usize,
    shown: usize,
}

impl Bar {
    fn max_scroll(&self) -> usize {
        self.total.saturating_sub(self.shown)
    }

    /// The thumb's top and height for `scroll`; None when all rows fit.
    fn thumb(&self, scroll: usize, scale: u32) -> Option<(u32, u32)> {
        let max = self.max_scroll();
        if max == 0 || self.bottom <= self.top {
            return None;
        }
        let track = self.bottom - self.top;
        let h = ((u64::from(track) * self.shown as u64 / self.total as u64) as u32).clamp(THUMB_MIN * scale, track);
        let travel = u64::from(track - h);
        let top = self.top + (travel * (max - scroll.min(max)) as u64 / max as u64) as u32;
        Some((top, h))
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && x < f64::from(self.x + self.w)
            && y >= f64::from(self.top)
            && y < f64::from(self.bottom)
    }
}

#[derive(Default)]
pub struct Console {
    pub open: bool,
    line: String,
    /// Where the cursor is in `line`, in bytes (the line is ASCII).
    cursor: usize,
    log: Vec<String>,
    /// Rows (answers as wrapped) scrolled back from the newest.
    scroll: usize,
    /// The scroll bar as the last overlay drew it.
    bar: Cell<Bar>,
    /// The overlay's scale (2 on a tall window).
    scale: Cell<u32>,
    /// A drag of the thumb: where in the thumb it was taken.
    drag: Option<f64>,
    history: Vec<String>,
    /// Where Up and Down are in `history` (its length: the new line).
    back: usize,
    /// Where `history` is kept between runs ([`Console::keep_history`]).
    history_file: Option<std::path::PathBuf>,
    /// The commands Tab completes.
    commands: Vec<String>,
    fonts: Option<Fonts>,
}

/// What a key did to the console.
pub enum Key {
    /// Nothing for the caller.
    None,
    /// Enter on a line: run it.
    Run(String),
}

impl Console {
    /// The game's fonts the console draws with.
    pub fn fonts(&self) -> Option<&Fonts> {
        self.fonts.as_ref()
    }

    pub fn new(fonts: Option<Fonts>) -> Self {
        Console { fonts, ..Console::default() }
    }

    /// Another disc's fonts; the answers and the lines typed stay.
    pub fn set_fonts(&mut self, fonts: Option<Fonts>) {
        self.fonts = fonts;
    }

    /// The lines typed kept in `file`: those it holds are read now (Up
    /// reaches them), and each new one is written back.
    pub fn keep_history(&mut self, file: std::path::PathBuf) {
        if let Ok(text) = std::fs::read_to_string(&file) {
            let mut kept: Vec<String> = text.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect();
            kept.append(&mut self.history);
            self.history = kept;
            self.back = self.history.len();
        }
        self.history_file = Some(file);
    }

    fn save_history(&self) {
        let Some(file) = &self.history_file else { return };
        let from = self.history.len().saturating_sub(HISTORY_KEEP);
        let mut text = self.history[from..].join("\n");
        text.push('\n');
        let _ = std::fs::write(file, text);
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// The commands Tab completes: the first word of each `help` line that
    /// names one (a command, two spaces, what it does).
    pub fn set_commands(&mut self, help: &str) {
        let mut c: Vec<String> = help
            .lines()
            .filter(|l| l.contains("  "))
            .filter_map(|l| l.split_whitespace().next())
            .map(str::to_string)
            .collect();
        c.sort();
        c.dedup();
        self.commands = c;
    }

    /// A key pressed while open, with the text it types and whether Ctrl is
    /// held.
    pub fn key(&mut self, code: winit::keyboard::KeyCode, text: Option<&str>, ctrl: bool) -> Key {
        use winit::keyboard::KeyCode as K;
        match code {
            K::Escape => self.open = false,
            K::Enter | K::NumpadEnter => {
                let line = std::mem::take(&mut self.line);
                self.cursor = 0;
                self.scroll = 0;
                if line.trim().is_empty() {
                    return Key::None;
                }
                if self.history.last() != Some(&line) {
                    self.history.push(line.clone());
                    self.save_history();
                }
                self.back = self.history.len();
                self.say(format!("> {line}"));
                return Key::Run(line);
            }
            K::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.line.remove(self.cursor);
            }
            K::Delete if self.cursor < self.line.len() => {
                self.line.remove(self.cursor);
            }
            K::ArrowLeft => self.cursor = self.cursor.saturating_sub(1),
            K::ArrowRight => self.cursor = (self.cursor + 1).min(self.line.len()),
            K::Home => self.cursor = 0,
            K::End => self.cursor = self.line.len(),
            K::KeyA if ctrl => self.cursor = 0,
            K::KeyE if ctrl => self.cursor = self.line.len(),
            K::KeyU if ctrl => {
                self.line.clear();
                self.cursor = 0;
            }
            K::KeyL if ctrl => {
                self.log.clear();
                self.scroll = 0;
            }
            K::ArrowUp if self.back > 0 => {
                self.back -= 1;
                self.line = self.history[self.back].clone();
                self.cursor = self.line.len();
            }
            K::ArrowDown if self.back < self.history.len() => {
                self.back += 1;
                self.line = self.history.get(self.back).cloned().unwrap_or_default();
                self.cursor = self.line.len();
            }
            K::PageUp => self.scroll_by(8),
            K::PageDown => self.scroll_by(-8),
            K::Tab => self.complete(),
            _ if !ctrl => {
                for c in text.unwrap_or("").chars() {
                    if (' '..='~').contains(&c) && c != '`' && self.line.len() < LINE_MAX {
                        self.line.insert(self.cursor, c);
                        self.cursor += 1;
                    }
                }
            }
            _ => {}
        }
        Key::None
    }

    /// Scrolled back `n` rows (forward for negative `n`), within the answers.
    fn scroll_by(&mut self, n: i32) {
        let bar = self.bar.get();
        let max = if bar.total == 0 { self.log.len().saturating_sub(1) } else { bar.max_scroll() };
        self.scroll = (self.scroll as i64 + i64::from(n)).clamp(0, max as i64) as usize;
    }

    /// The mouse wheel over the open console: `notches` up scrolls back.
    pub fn wheel(&mut self, notches: f32) {
        if self.open {
            self.scroll_by((notches * WHEEL_ROWS as f32).round() as i32);
        }
    }

    /// The left button pressed at (x, y): on the thumb a drag starts, on the
    /// track above or below it a page goes by. True when the bar took it.
    pub fn press(&mut self, x: f64, y: f64) -> bool {
        let bar = self.bar.get();
        if !self.open || !bar.contains(x, y) {
            return false;
        }
        let Some((top, h)) = bar.thumb(self.scroll, self.scale.get()) else { return true };
        let page = bar.shown.max(1) as i32;
        if y < f64::from(top) {
            self.scroll_by(page);
        } else if y >= f64::from(top + h) {
            self.scroll_by(-page);
        } else {
            self.drag = Some(y - f64::from(top));
        }
        true
    }

    /// The mouse moved to `y`: a drag moves the thumb with it.
    pub fn motion(&mut self, y: f64) {
        let Some(grab) = self.drag else { return };
        let bar = self.bar.get();
        let Some((_, h)) = bar.thumb(self.scroll, self.scale.get()) else { return };
        let travel = f64::from(bar.bottom - bar.top - h);
        if travel <= 0.0 {
            return;
        }
        let at = ((y - grab - f64::from(bar.top)) / travel).clamp(0.0, 1.0);
        let max = bar.max_scroll();
        self.scroll = max - (at * max as f64).round() as usize;
    }

    /// The left button let go.
    pub fn release(&mut self) {
        self.drag = None;
    }

    /// Tab: the first word completed from the commands, as far as they
    /// agree; with more than one left, they are listed.
    fn complete(&mut self) {
        if self.line.contains(' ') {
            return;
        }
        let found: Vec<String> = self.commands.iter().filter(|c| c.starts_with(&self.line)).cloned().collect();
        let Some(first) = found.first() else { return };
        let mut common = first.clone();
        for c in &found[1..] {
            let n = common.bytes().zip(c.bytes()).take_while(|(a, b)| a == b).count();
            common.truncate(n);
        }
        if found.len() == 1 {
            self.line = format!("{common} ");
        } else {
            if common.len() == self.line.len() {
                self.say(found.join("  "));
            }
            self.line = common;
        }
        self.cursor = self.line.len();
    }

    /// An answer: each of its lines kept, the oldest dropped.
    pub fn say(&mut self, text: String) {
        for l in text.lines() {
            self.log.push(l.to_string());
        }
        let n = self.log.len().saturating_sub(LOG_KEEP);
        self.log.drain(..n);
    }

    /// The console at the window's size, `width` x `height` pixels: a band
    /// over the top two fifths of the window, the answers that fit (scrolled
    /// back by Page Up), the line being typed with its cursor. None when it
    /// is closed or there is no font.
    pub fn overlay(&self, width: u32, height: u32) -> Option<Overlay> {
        let fonts = self.fonts.as_ref().filter(|_| self.open && width > 0 && height > 0)?;
        let scale = if height >= 1400 { 2 } else { 1 };
        let lh = PX_LINE * scale;
        let bh = (height * 2 / 5).max(lh * 3 + 2 * PX_MARGIN).min(height);
        let mut o = Overlay { width, height: bh, rgba: vec![0; (width * bh * 4) as usize] };
        for px in o.rgba.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&BAND);
        }
        for x in 0..width {
            let at = (((bh - 1) * width + x) * 4) as usize;
            o.rgba[at..at + 4].copy_from_slice(&RULE);
        }
        let rows = ((bh - 2 * PX_MARGIN) / lh).saturating_sub(1) as usize;
        let bar_w = BAR_W * scale;
        let lines = self.wrapped(fonts, width.saturating_sub(bar_w + PX_MARGIN), scale);
        let bar = Bar {
            x: width.saturating_sub(bar_w + PX_MARGIN / 2),
            w: bar_w,
            top: PX_MARGIN,
            bottom: PX_MARGIN + lh * rows as u32,
            total: lines.len(),
            shown: rows,
        };
        self.bar.set(bar);
        self.scale.set(scale);
        if let Some((top, h)) = bar.thumb(self.scroll, scale) {
            let held = if self.drag.is_some() { THUMB_HELD } else { THUMB };
            for y in bar.top..bar.bottom {
                let c = if (top..top + h).contains(&y) { held } else { TRACK };
                for x in bar.x..bar.x + bar.w {
                    put(&mut o, x, y, c);
                }
            }
        }
        let end = lines.len().saturating_sub(self.scroll);
        let start = end.saturating_sub(rows);
        // The newest line just above the prompt, as Quake's.
        let first = rows - (end - start);
        for (i, row) in lines[start..end].iter().enumerate() {
            let y = PX_MARGIN + lh * (first + i) as u32;
            for &(x, ref bytes, colour) in row {
                text(fonts, &mut o, x, y, bytes, colour, scale);
            }
        }
        let y = PX_MARGIN + lh * rows as u32;
        let note = if self.scroll > 0 { format!("   (scrolled back {})", self.scroll) } else { String::new() };
        // The line typed, scrolled so that the cursor stays in the window.
        let room = width.saturating_sub(2 * PX_MARGIN + 4 * scale);
        let from = (0..self.cursor)
            .find(|&i| text_width(fonts, format!("] {}", &self.line[i..self.cursor]).as_bytes(), scale) <= room)
            .unwrap_or(self.cursor);
        let prompt = format!("] {}{note}", &self.line[from..]);
        text(fonts, &mut o, PX_MARGIN, y, prompt.as_bytes(), PROMPT, scale);
        // The cursor: a bar after the text before it.
        let x = PX_MARGIN + text_width(fonts, format!("] {}", &self.line[from..self.cursor]).as_bytes(), scale);
        for dy in 0..lh.saturating_sub(2) {
            for dx in 0..scale.max(1) * 2 {
                put(&mut o, x + dx, y + dy + 1, [0xff, 0xff, 0xff, 0xff]);
            }
        }
        Some(o)
    }

    /// The answers as rows `width` pixels wide: each row its pieces (x, text,
    /// colour). A line wraps at its spaces, a word wider than the row at
    /// its letters; a help line's second column (after a run of spaces)
    /// starts past the widest command, up to half the row, and wraps under
    /// itself.
    fn wrapped(&self, fonts: &Fonts, width: u32, scale: u32) -> Vec<Vec<Piece>> {
        let right = width.saturating_sub(PX_MARGIN);
        let head = |l: &str| l.find("  ").filter(|_| !l.starts_with("> "));
        let widest =
            self.log.iter().filter_map(|l| head(l).map(|at| text_width(fonts, &l.as_bytes()[..at], scale))).max();
        let column = PX_MARGIN + (widest.unwrap_or(0) + HELP_GAP * scale).max(HELP_COLUMN * scale).min(width / 2);
        let mut out = Vec::new();
        for l in &self.log {
            let colour = if l.starts_with("> ") { TYPED } else { ANSWER };
            let split = head(l).filter(|&at| {
                right > column + 100 * scale && PX_MARGIN + text_width(fonts, &l.as_bytes()[..at], scale) < column
            });
            match split {
                Some(at) => {
                    let mut pieces = wrap(fonts, l[at..].trim_start().as_bytes(), right - column, scale).into_iter();
                    let head = (PX_MARGIN, l.as_bytes()[..at].to_vec(), TYPED);
                    out.push(match pieces.next() {
                        Some(p) => vec![head, (column, p, colour)],
                        None => vec![head],
                    });
                    out.extend(pieces.map(|p| vec![(column, p, colour)]));
                }
                None => {
                    let pieces = wrap(fonts, l.as_bytes(), right - PX_MARGIN, scale);
                    if pieces.is_empty() {
                        out.push(Vec::new());
                    }
                    out.extend(pieces.into_iter().map(|p| vec![(PX_MARGIN, p, colour)]));
                }
            }
        }
        out
    }

    /// `--shot`: the band, the last answers and the line, drawn into
    /// `frame` in the game's font at its logical size.
    pub fn draw(&self, frame: &mut Frame) {
        let Some(fonts) = &self.fonts else { return };
        let mut ctx = Ctx::new(View::default());
        ctx.uploads = std::mem::take(&mut frame.uploads);
        let log = &self.log[self.log.len().saturating_sub(SHOT_LINES)..];
        let lines = log.len() + 1;
        let h = LINE_H * lines as f32 + 8.0;
        piney_desktop::fade::draw_rect(&mut ctx, LAYER, [0.0, 0.0, 512.0, h], 0x6000_0000, 0x6000_0000, 0, 1);
        let view = piney_desktop::message::menu_view();
        let names = Names::default();
        let prompt = format!("] {}_", self.line);
        for (i, text) in log.iter().chain(std::iter::once(&prompt)).enumerate() {
            let mut k = Kanji::init(3, 96);
            k.dx = 8.0;
            k.dy = 4.0 + LINE_H * i as f32;
            k.colour = if i == lines - 1 { [128, 128, 64, 128] } else { [128, 128, 128, 128] };
            ctx.disp(fonts, &mut k, LAYER, &view, text.as_bytes(), &names);
        }
        let m = ctx.finish();
        frame.uploads = m.uploads;
        frame.cmds.extend(m.cmds);
    }
}

/// A character's advance, as `Extract` packs `ef8x16` proportionally: the
/// glyph's 8 texels less its trim (`englishFontOfsS`), one texel apart.
fn advance(fonts: &Fonts, c: u8) -> u32 {
    match fonts.small_ascii(c) {
        Some((_, trim)) if (0..8).contains(&trim) => (8 - trim) as u32 + 1,
        _ => 8,
    }
}

fn text_width(fonts: &Fonts, s: &[u8], scale: u32) -> u32 {
    s.iter().map(|&c| advance(fonts, c) * scale).sum()
}

/// `s` in pieces no wider than `room` pixels: broken at its spaces (which
/// are dropped at a break), a word wider than `room` at its letters.
fn wrap(fonts: &Fonts, s: &[u8], room: u32, scale: u32) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut row: Vec<u8> = Vec::new();
    let mut w = 0;
    for word in s.split(|&c| c == b' ') {
        let ww = text_width(fonts, word, scale);
        let space = if row.is_empty() { 0 } else { advance(fonts, b' ') * scale };
        if !row.is_empty() && w + space + ww > room {
            out.push(std::mem::take(&mut row));
            w = 0;
        }
        if !row.is_empty() {
            row.push(b' ');
            w += space;
        }
        for &c in word {
            let a = advance(fonts, c) * scale;
            if !row.is_empty() && w + a > room {
                out.push(std::mem::take(&mut row));
                w = 0;
            }
            row.push(c);
            w += a;
        }
    }
    if !row.is_empty() || out.is_empty() && !s.is_empty() {
        out.push(row);
    }
    out
}

/// `s` at (x, y) in `colour`: each glyph's texels through `xasc00`'s
/// palette (white and its black-to-white ramp), tinted, opaque; the
/// clear index left alone. Glyphs past the right edge are dropped.
fn text(fonts: &Fonts, o: &mut Overlay, mut x: u32, y: u32, s: &[u8], colour: [u8; 3], scale: u32) {
    for &c in s {
        let c = if (0x20..0x80).contains(&c) { c } else { b'?' };
        let adv = advance(fonts, c) * scale;
        if x + adv >= o.width {
            return;
        }
        if let Some((rows, _)) = fonts.small_ascii(c) {
            // Left-aligned in its cell: the texels the advance covers.
            let w = adv / scale;
            for (r, row) in rows.iter().enumerate() {
                for (j, &i) in row.iter().enumerate() {
                    if i == 0 || j as u32 >= w {
                        continue;
                    }
                    let p = fonts.palette.get(usize::from(i)).map_or([255; 4], |c| c.0);
                    let lum = u32::from(p[0].max(p[1]).max(p[2]));
                    let px = colour.map(|k| (u32::from(k) * lum / 255) as u8);
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let xx = x + j as u32 * scale + sx;
                            put(o, xx, y + r as u32 * scale + sy, [px[0], px[1], px[2], 0xff]);
                        }
                    }
                }
            }
        }
        x += adv;
    }
}

fn put(o: &mut Overlay, x: u32, y: u32, px: [u8; 4]) {
    if x < o.width && y < o.height {
        let at = ((y * o.width + x) * 4) as usize;
        o.rgba[at..at + 4].copy_from_slice(&px);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts() -> Option<Fonts> {
        let iso = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let mut disc = piney_data::iso::Iso::open(&iso).ok()?;
        let archive = piney_data::archive::Archive::new(disc.read_path("DATA/DATA.BIN").ok()?).ok()?;
        piney_desktop::assets::read_fonts(disc.volume().ok()?, &archive).ok()
    }

    /// Issue #27: an answer wider than the window wraps at its spaces into
    /// rows that each fit, losing no word; a help line's second column
    /// wraps under itself; a word wider than a row breaks at its letters.
    #[test]
    fn long_answers_wrap() {
        let Some(f) = fonts() else { return };
        let mut c = Console::new(Some(f));
        c.open = true;
        let long = "import_card: copied 3 saves from C:/Users/someone/Documents/PCSX2/memcards/Mcd001.ps2 into the build's card, the old card kept in backup-1";
        c.say(long.into());
        c.say("pad_log [FILE|stop]  write this run since power-on (pads, console commands, the card it began with) for a bug report".into());
        c.say("deflicker [on|off]  the console's deflicker".into());
        c.say("x".repeat(400));
        let fonts = c.fonts().unwrap();
        let width = 640;
        let rows = c.wrapped(fonts, width, 1);
        for row in &rows {
            for (x, t, _) in row {
                assert!(
                    x + text_width(fonts, t, 1) <= width - PX_MARGIN,
                    "a row past the edge: {:?}",
                    String::from_utf8_lossy(t)
                );
            }
        }
        let first: Vec<String> = rows
            .iter()
            .take_while(|r| r.len() == 1 && r[0].0 == PX_MARGIN)
            .map(|r| String::from_utf8_lossy(&r[0].1).into_owned())
            .collect();
        assert!(first.len() > 1, "the long answer did not wrap");
        assert_eq!(first.join(" "), long);
        let help = rows.iter().position(|r| r.len() == 2).expect("the help line's two columns");
        let (head, column) = (&rows[help][0], rows[help][1].0);
        assert!(head.0 + text_width(fonts, &head.1, 1) < column, "the command runs into its description");
        assert!(rows[help + 1].len() == 1 && rows[help + 1][0].0 == column, "the second column wraps under itself");
        assert_eq!(rows.iter().filter(|r| r.len() == 2).count(), 2, "both help lines in two columns");
        let xs: usize = rows.iter().rev().take_while(|r| r[0].1.iter().all(|&b| b == b'x')).map(|r| r[0].1.len()).sum();
        assert_eq!(xs, 400);
        assert!(c.overlay(width, 480).is_some());
    }

    /// The lines typed come back after a disc change (new fonts) and from
    /// the history file of an earlier run, the newest last.
    #[test]
    fn the_history_is_kept() {
        let dir = std::env::temp_dir().join(format!("piney-console-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("console_history.txt");
        let _ = std::fs::remove_file(&file);
        let mut c = Console::new(None);
        c.keep_history(file.clone());
        c.open = true;
        for l in ["gold 100", "where"] {
            c.line = l.into();
            c.key(winit::keyboard::KeyCode::Enter, None, false);
        }
        c.set_fonts(None);
        c.key(winit::keyboard::KeyCode::ArrowUp, None, false);
        assert_eq!(c.line, "where");
        let mut later = Console::new(None);
        later.keep_history(file.clone());
        later.key(winit::keyboard::KeyCode::ArrowUp, None, false);
        later.key(winit::keyboard::KeyCode::ArrowUp, None, false);
        assert_eq!(later.line, "gold 100");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Tab learns its commands from the help lines alone (a command, two
    /// spaces, what it does): the title's "no console commands here" is
    /// none. Every one of the window's own help lines is one.
    #[test]
    fn tab_completes_the_commands() {
        let mut c = Console::new(None);
        c.set_commands(&format!("no console commands here\n{}", crate::APP_HELP));
        assert_eq!(
            c.commands,
            [
                "deflicker",
                "fps_cap",
                "hud_scale",
                "import_card",
                "pad_log",
                "render_scale",
                "story",
                "version",
                "vsync"
            ]
        );
        assert!(crate::APP_HELP.lines().all(|l| l.contains("  ")));
        c.line = "ve".into();
        c.cursor = 2;
        c.complete();
        assert_eq!(c.line, "version ");
    }

    /// The scroll bar: the wheel scrolls back three rows a notch, the thumb
    /// dragged to the top shows the first rows, a click under the thumb
    /// pages forward, and the scroll stays within the answers.
    #[test]
    fn the_scroll_bar_scrolls() {
        let Some(f) = fonts() else { return };
        let mut c = Console::new(Some(f));
        c.open = true;
        for i in 0..100 {
            c.say(format!("line {i}"));
        }
        let (w, h) = (640, 480);
        c.overlay(w, h).unwrap();
        let bar = c.bar.get();
        assert_eq!(bar.total, 100);
        let max = bar.max_scroll();
        assert!(max > 0 && bar.shown > 0);
        let (top, th) = bar.thumb(0, 1).expect("a thumb");
        assert_eq!(top + th, bar.bottom, "the thumb at the bottom shows the newest");
        c.wheel(1.0);
        assert_eq!(c.scroll, 3);
        c.wheel(-5.0);
        assert_eq!(c.scroll, 0);
        // Drag the thumb to the top.
        let x = f64::from(bar.x + 1);
        assert!(c.press(x, f64::from(top + 2)));
        c.motion(f64::from(bar.top) - 50.0);
        c.release();
        assert_eq!(c.scroll, max);
        c.overlay(w, h).unwrap();
        // A click under the thumb: a page forward.
        let (top, th) = c.bar.get().thumb(c.scroll, 1).unwrap();
        assert_eq!(top, bar.top);
        assert!(c.press(x, f64::from(top + th + 4)));
        c.release();
        assert_eq!(c.scroll, max - bar.shown);
        // Off the bar the mouse is not the console's.
        assert!(!c.press(10.0, 10.0));
        for _ in 0..50 {
            c.wheel(1.0);
        }
        assert_eq!(c.scroll, max);
    }

    /// By hand: `cargo test -p piney-game console_picture -- --ignored`
    /// writes the console over a grey window, 1280 x 960, to
    /// `PINEY_SHOTS` (default `/mnt/data/claude/scratch/shots`).
    #[test]
    #[ignore]
    fn console_picture() {
        let iso = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        let mut disc = piney_data::iso::Iso::open(&iso).unwrap();
        let archive = piney_data::archive::Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let fonts = piney_desktop::assets::read_fonts(disc.volume().unwrap(), &archive).unwrap();
        let mut c = Console::new(Some(fonts));
        c.open = true;
        for i in 0..30 {
            c.say(format!("an earlier answer {i}"));
        }
        c.say("help".into());
        c.say("item CAT ID [N]   an item to Kite (15 key items)\ngod               the party at full HP and SP each frame".into());
        c.say("> where".into());
        c.say("The World - area 1 field 27 - play 1138 - Kite at (24600, 24600, 0) act 2".into());
        c.say("import_card: copied 3 saves from C:/Users/someone/Documents/PCSX2/memcards/Mcd001.ps2 into the build's card; the card it replaced is kept in backup-1 beside it, and the slots' records were rebuilt".into());
        c.line = "gold 1000".into();
        c.cursor = 4;
        let o = c.overlay(1280, 960).unwrap();
        let mut img: Vec<u8> = [0x50u8, 0x60, 0x40, 0xff].repeat(1280 * 960);
        for y in 0..o.height as usize {
            for x in 0..1280 {
                let s = &o.rgba[(y * 1280 + x) * 4..][..4];
                let d = &mut img[(y * 1280 + x) * 4..][..4];
                let a = u32::from(s[3]);
                for k in 0..3 {
                    d[k] = ((u32::from(s[k]) * a + u32::from(d[k]) * (255 - a)) / 255) as u8;
                }
            }
        }
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/shots".into());
        std::fs::write(format!("{dir}/console.png"), piney_desktop::soft::png(1280, 960, &img)).unwrap();
    }
}
