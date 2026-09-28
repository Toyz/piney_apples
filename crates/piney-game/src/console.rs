//! Not the game's: the debug console. F1 (or the ` key) opens a line typed on
//! the keyboard; Enter hands it to the mode ([`crate::mode::Mode::console`])
//! and keeps the answer; Escape or F1 again closes it, while the game runs on
//! with its pad held neutral. It draws in the game's `ef8x16` font on a dark
//! band ([`Console::overlay`], or [`Console::draw`] under `--shot`). The keys
//! are the usual line-editing ones, Up / Down for the lines before, Page Up /
//! Down to scroll, Tab to complete; `help` lists the commands.

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
/// `--shot`'s band: drawn over everything (the font layer is 240, the
/// menu's 242), its lines' spacing in logical units.
const LAYER: i16 = 250;
const LINE_H: f32 = 18.0;
/// The overlay's line height and margin, in window pixels at scale 1.
const PX_LINE: u32 = 18;
const PX_MARGIN: u32 = 6;
/// A help line's second column, in pixels at scale 1.
const HELP_COLUMN: u32 = 140;

/// Text colours: an answer, a line typed, the prompt, the band's rule.
const ANSWER: [u8; 3] = [0xd8, 0xd8, 0xd8];
const TYPED: [u8; 3] = [0xff, 0xdc, 0x78];
const PROMPT: [u8; 3] = [0xff, 0xff, 0xff];
const RULE: [u8; 4] = [0x70, 0x78, 0xa0, 0xff];
const BAND: [u8; 4] = [0x0c, 0x0c, 0x14, 0xd8];

#[derive(Default)]
pub struct Console {
    pub open: bool,
    line: String,
    /// Where the cursor is in `line`, in bytes (the line is ASCII).
    cursor: usize,
    log: Vec<String>,
    /// Answer lines scrolled back from the newest.
    scroll: usize,
    history: Vec<String>,
    /// Where Up and Down are in `history` (its length: the new line).
    back: usize,
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

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// The commands Tab completes (each the first word of a `help` line).
    pub fn set_commands(&mut self, help: &str) {
        let mut c: Vec<String> = help.lines().filter_map(|l| l.split_whitespace().next()).map(str::to_string).collect();
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
            K::PageUp => self.scroll = (self.scroll + 8).min(self.log.len().saturating_sub(1)),
            K::PageDown => self.scroll = self.scroll.saturating_sub(8),
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
        let end = self.log.len().saturating_sub(self.scroll);
        let start = end.saturating_sub(rows);
        // The newest line just above the prompt, as Quake's.
        let first = rows - (end - start);
        for (i, l) in self.log[start..end].iter().enumerate() {
            let colour = if l.starts_with("> ") { TYPED } else { ANSWER };
            let y = PX_MARGIN + lh * (first + i) as u32;
            // A help line's two columns (a run of spaces between them) at a
            // column of their own: the font is proportional.
            match l.find("  ").filter(|_| !l.starts_with("> ")) {
                Some(at) => {
                    text(fonts, &mut o, PX_MARGIN, y, &l.as_bytes()[..at], TYPED, scale);
                    let rest = l[at..].trim_start();
                    text(fonts, &mut o, PX_MARGIN + HELP_COLUMN * scale, y, rest.as_bytes(), colour, scale);
                }
                None => text(fonts, &mut o, PX_MARGIN, y, l.as_bytes(), colour, scale),
            }
        }
        let y = PX_MARGIN + lh * rows as u32;
        let prompt = if self.scroll > 0 {
            format!("] {}   (scrolled back {})", self.line, self.scroll)
        } else {
            format!("] {}", self.line)
        };
        text(fonts, &mut o, PX_MARGIN, y, prompt.as_bytes(), PROMPT, scale);
        // The cursor: a bar after the text before it.
        let x = PX_MARGIN + text_width(fonts, format!("] {}", &self.line[..self.cursor]).as_bytes(), scale);
        for dy in 0..lh.saturating_sub(2) {
            for dx in 0..scale.max(1) * 2 {
                put(&mut o, x + dx, y + dy + 1, [0xff, 0xff, 0xff, 0xff]);
            }
        }
        Some(o)
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
        c.say("help".into());
        c.say("item CAT ID [N]   an item to Kite (15 key items)\ngod               the party at full HP and SP each frame".into());
        c.say("> where".into());
        c.say("The World - area 1 field 27 - play 1138 - Kite at (24600, 24600, 0) act 2".into());
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
