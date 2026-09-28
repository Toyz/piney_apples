//! `ccKanji`: proportional text rasterised into a sprite's own 4-bit texture
//! and drawn as one SPRITE per run (`docs/engine/font.md`). A port of
//! `tools/font.py` (`expand`, `extract`, `make_packet_str`, `Kanji.disp`),
//! which `font.py check` holds to the game's own functions run in eemu.
//!
//! The two bitmap fonts are read from the executable at run time; the glyph
//! trims and the escape colours are engine tables (`tables.rs`).

use piney_draw::{Blend, DrawState, Prim, PrimKind, Rgba, TexRef, TexState, Upload, UploadFormat, Vertex};

use crate::view::{CULL_X1, CULL_Y1, LayerView, XYOFFSET_X, XYOFFSET_Y};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_data::volume::Volume;

/// Bytes a row of a ccKanji texture: 128 texels at 4 bits.
pub const TEX_W: usize = 64;
/// Texels across a ccKanji texture.
pub const TEX_TEXELS: u16 = 128;
/// Glyphs in each bitmap font.
pub const GLYPHS: usize = 112;
/// The drop shadow's colour (ctrl 0x10 in MakePacketStr).
pub const SHADOW: [u8; 3] = [16, 16, 16];
/// `ccKanji::Init`'s ctrl: textured (1) with the drop shadow (0x10).
pub const CTRL_KANJI: u32 = 0x11;

/// One of the two bitmap fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Face {
    cw: usize,
    ch: usize,
    per_row: usize,
    /// Bytes a bitmap row.
    stride: usize,
    small: bool,
}

const SMALL: Face = Face { cw: 8, ch: 16, per_row: 16, stride: 64, small: true };
const LARGE: Face = Face { cw: 12, ch: 20, per_row: 10, stride: 96, small: false };

/// ccKanji.kt: which font, and whether glyphs pack proportionally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kt {
    /// ef8x16, proportional: chat, WORLD, the mailer, the board, menus.
    SmallProportional = 0,
    /// ef8x16, fixed cells.
    SmallFixed = 1,
    /// ef12x20, proportional: dialogue, loading screens.
    LargeProportional = 2,
    /// ef12x20, fixed: name entry, the staff roll.
    LargeFixed = 3,
}

impl Kt {
    fn face(self) -> Face {
        match self {
            Kt::SmallProportional | Kt::SmallFixed => SMALL,
            _ => LARGE,
        }
    }

    fn proportional(self) -> bool {
        matches!(self, Kt::SmallProportional | Kt::LargeProportional)
    }

    fn small(self) -> bool {
        matches!(self, Kt::SmallProportional | Kt::SmallFixed)
    }
}

/// The two bitmap fonts out of the executable, and `CLT_xasc00`.
#[derive(Clone)]
pub struct Fonts {
    small: Vec<u8>,
    large: Vec<u8>,
    /// `englishFontOfsS` and `englishFontOfsL` with the three halfwords
    /// past each that %X-%Z read (the volume's next global).
    trims_s: Vec<i16>,
    trims_l: Vec<i16>,
    /// `CLT_xasc00`: 0 clear, 1-3 white, 4-15 a black-to-white ramp.
    pub palette: Vec<Rgba>,
}

/// Glyph rows read from each bitmap: 9 rows of 16, enough for glyph 130
/// (%Z), which the game reads past the 112 drawn glyphs.
const BITMAP_GLYPH_ROWS: usize = 9;

impl Fonts {
    /// The volume's (`piney_data::tables::fonts`): `ef8x16` with the rows
    /// past its end the game reads, and `ef12x20`, [`BITMAP_GLYPH_ROWS`]
    /// glyph rows each.
    pub fn of(volume: Volume, palette: Vec<Rgba>) -> Self {
        use piney_data::tables::{fonts, kanji};
        let small = [*fonts::EF8X16, fonts::of(volume).ef8x16_past].concat();
        let large = fonts::EF12X20.to_vec();
        let trims_s = [*kanji::ENGLISH_FONT_OFS_S, kanji::of(volume).english_font_ofs_s_past].concat();
        let trims_l = [*kanji::ENGLISH_FONT_OFS_L, *kanji::ENGLISH_FONT_OFS_L_PAST].concat();
        debug_assert_eq!(small.len(), BITMAP_GLYPH_ROWS * SMALL.ch * SMALL.stride);
        debug_assert_eq!(large.len(), BITMAP_GLYPH_ROWS * LARGE.ch * LARGE.stride);
        Fonts { small, large, trims_s, trims_l, palette }
    }

    fn bitmap(&self, f: Face) -> &[u8] {
        if f.small { &self.small } else { &self.large }
    }

    /// Byte offset of glyph g's top row in its bitmap.
    fn glyph_src(f: Face, g: usize) -> usize {
        (g >> 4) * f.stride * f.ch + (g & 15) * (f.cw / 2)
    }

    fn byte(&self, f: Face, at: usize) -> u8 {
        self.bitmap(f).get(at).copied().unwrap_or(0)
    }
}

impl Fonts {
    /// Glyph `g`'s trim for `kt`.
    fn trim(&self, kt: Kt, g: usize) -> i32 {
        let t = if kt.small() { &self.trims_s } else { &self.trims_l };
        t.get(g).copied().unwrap_or(0) as i32
    }

    /// An ASCII character (0x20-0x7f) of `ef8x16`, for the port's own text
    /// drawn outside the game's windows (the console): its 16 rows of 8
    /// palette indices (each byte two texels, the low nibble on the left)
    /// and its proportional width (`englishFontOfsS`); None for any other
    /// byte.
    pub fn small_ascii(&self, c: u8) -> Option<([[u8; 8]; 16], i32)> {
        if !(0x20..0x80).contains(&c) {
            return None;
        }
        let g = usize::from(c - 0x20);
        let src = Fonts::glyph_src(SMALL, g);
        let mut rows = [[0u8; 8]; 16];
        for (r, row) in rows.iter_mut().enumerate() {
            for j in 0..4 {
                let b = self.byte(SMALL, src + r * SMALL.stride + j);
                row[2 * j] = b & 15;
                row[2 * j + 1] = b >> 4;
            }
        }
        Some((rows, self.trim(Kt::SmallProportional, g)))
    }
}

/// C integer division, truncating towards zero.
fn c_div(a: i32, b: i32) -> i32 {
    a / b
}

/// `ccGetExtendedCode` (0x0015d390): ten Shift-JIS codes to `%x`, anything
/// else unchanged.
pub fn extended_code(code: u16) -> u16 {
    match code {
        0x83a2 => 0x2530,
        0x83a9 => 0x2531,
        0x83b0 => 0x2532,
        0x83b6 => 0x2533,
        0x83a6 => 0x2534,
        0x819b => 0x2535,
        0x81a2 => 0x2536,
        0x81a0 => 0x2537,
        0x817e => 0x2538,
        0x8165 => 0x2544,
        c => c,
    }
}

/// The glyph a `%x` pair draws, or None.
pub fn percent_glyph(d: u8) -> Option<usize> {
    match d {
        0x30..=0x39 => Some(d as usize + 47),
        0x41..=0x5a => Some(d as usize + 40),
        0x25 | 0x23 => Some(d as usize - 0x20),
        _ => None,
    }
}

/// The string with three NULs after it: the game reads a byte or two past a
/// trailing escape or lead byte.
fn pad(raw: &[u8]) -> Vec<u8> {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    let mut s = raw[..end].to_vec();
    s.extend_from_slice(&[0, 0, 0]);
    s
}

/// The two names `#0` and `#1` expand to (`ccSaveData.plName`, `plRealName`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    pub name: Vec<u8>,
    pub real: Vec<u8>,
}

impl Default for Names {
    fn default() -> Self {
        Names { name: b"Kite".to_vec(), real: b"Player".to_vec() }
    }
}

/// `ccKanji::Disp`'s first pass: names in, Shift-JIS mapped, other bytes
/// outside 0x20-0x7f to `_` one byte at a time.
pub fn expand(raw: &[u8], names: &Names) -> Vec<u8> {
    let s = pad(raw);
    let mut out = Vec::new();
    let mut i = 0;
    while s[i] != 0 {
        let c = s[i];
        if c == 0x23 && s[i + 1] == 0x30 {
            out.extend_from_slice(&names.name);
            i += 2;
        } else if c == 0x23 && s[i + 1] == 0x31 {
            out.extend_from_slice(&names.real);
            i += 2;
        } else if c == 0x23 || c == 0x25 {
            out.extend_from_slice(&s[i..i + 2]);
            i += 2;
        } else if (0x20..0x80).contains(&c) {
            out.push(c);
            i += 1;
        } else {
            let e = extended_code(u16::from(c) << 8 | u16::from(s[i + 1]));
            if e >> 8 == 0x25 {
                out.extend_from_slice(&[0x25, e as u8]);
                i += 2;
            } else {
                out.push(0x5f);
                i += 1;
            }
        }
    }
    out
}

/// `dec2sjis(n, buf, width, mode)` (0x0015f8e0), which writes ASCII
/// digits: the low `width` digits of `n`, leading zeros dropped (mode 0),
/// as spaces (1) or kept (2); a lone 0 is written as "0".
pub fn dec2sjis(n: i32, width: i32, mode: i32) -> Vec<u8> {
    let mut digits = [0i32; 16];
    let mut v = n;
    for d in digits.iter_mut().rev() {
        if v != 0 {
            *d = v % 10;
            v /= 10;
        }
    }
    let mut out = Vec::new();
    if width <= 0 {
        return out;
    }
    let mut mode = mode;
    for (i, &d) in digits.iter().enumerate().skip((16 - width).max(0) as usize) {
        if d != 0 || mode == 2 {
            out.push((48 + d) as u8);
            mode = 2;
        } else if i == 15 {
            out.push(b'0');
        } else if mode == 1 {
            out.push(b' ');
        }
    }
    out
}

/// `ccKanjiStrSeparate(m, n)` (0x0015f860): the offset of line `n` in
/// NUL-separated lines; a byte outside 0x20-0x7f takes the next with it.
pub fn str_separate(buf: &[u8], n: usize) -> usize {
    let at = |i: usize| buf.get(i).copied().unwrap_or(0);
    let mut i = 0;
    for _ in 0..n {
        loop {
            let c = at(i);
            i += 1;
            if c == 0 {
                break;
            }
            if !(0x20..0x80).contains(&c) {
                i += 1;
            }
        }
    }
    i
}

/// `ccKanjiStrWidth(m, kt)` (0x0015f2a0) in pixels.
pub fn str_width(fonts: &Fonts, raw: &[u8], kt: Kt, names: &Names) -> i32 {
    let adv = |g: usize| -> i32 {
        match kt {
            Kt::SmallFixed => 8,
            Kt::SmallProportional => 8 - fonts.trim(Kt::SmallProportional, g),
            Kt::LargeFixed => 12,
            Kt::LargeProportional => 12 - fonts.trim(Kt::LargeProportional, g),
        }
    };
    let s = pad(raw);
    let mut w = 0;
    let mut i = 0;
    while s[i] != 0 {
        let c = s[i];
        if c == 0x23 {
            if s[i + 1] == 0x30 {
                w += str_width(fonts, &names.name, kt, names);
            } else if s[i + 1] == 0x31 {
                w += str_width(fonts, &names.real, kt, names);
            }
            i += 2;
        } else if c == 0x25 {
            match kt {
                Kt::SmallFixed => w += 8,
                Kt::LargeFixed => w += 12,
                _ => {
                    if let Some(g) = percent_glyph(s[i + 1]) {
                        w += adv(g);
                    }
                }
            }
            i += 2;
        } else if (0x20..0x80).contains(&c) {
            w += adv(c as usize - 0x20);
            i += 1;
        } else {
            let e = extended_code(u16::from(c) << 8 | u16::from(s[i + 1]));
            if e >> 8 == 0x25 {
                let lo = e as u8;
                match kt {
                    Kt::SmallFixed => w += 8,
                    Kt::LargeFixed => w += 12,
                    _ if (0x30..=0x39).contains(&lo) => w += adv(lo as usize + 47),
                    // The quirk: the pair's own trail byte is tested.
                    _ if (0x41..=0x5a).contains(&s[i + 1]) => w += adv(lo as usize + 40),
                    _ => {}
                }
                i += 2;
            } else {
                w += adv(63);
                i += 1;
            }
        }
    }
    w
}

/// `ccKanji::Extract` (0x0015d490) over an expanded string: rasterise into
/// `tex` (128 x th texels, 4-bit, VRAM order). Returns the glyphs extracted
/// (ccKanji.clm).
pub fn extract(fonts: &Fonts, strn: &[u8], kt: Kt, th: usize, tex: &mut [u8]) -> usize {
    let f = kt.face();
    let rows = f.ch as i32;
    let lines = th as i32 / rows;
    let cwb = (f.cw / 2) as i32;
    let per_row = 128 / f.cw;
    let base = if f.small { 0 } else { 8 * TEX_W as i32 };
    let tw = TEX_W as i32;
    let len = tex.len() as i32;
    let s = pad(strn);
    let (mut i, mut c, mut l, mut aflp, mut clm) = (0usize, 0i32, 0i32, 0i32, 0usize);
    let mut last_face: Option<Face> = None;

    let row0 = |l: i32| base + (lines - l) * rows * tw - tw;
    let tail = |tex: &mut [u8], c: i32, l: i32, aflp: i32, face: Face| {
        // The rest of the texture row, filled with byte 0 of each row of
        // glyph 0.
        let dst = row0(l) + c * cwb - c_div(aflp, 2);
        let n = c_div(aflp, 2) + 64 - c * cwb;
        if dst - (rows - 1) * tw < 0 || dst + n > len {
            return;
        }
        for r in 0..rows {
            let fill = fonts.byte(face, r as usize * face.stride);
            let d = dst - r * tw;
            for j in 0..n {
                tex[(d + j) as usize] = fill;
            }
        }
    };

    while s[i] != 0 {
        let code = extended_code(u16::from(s[i]) << 8 | u16::from(s[i + 1]));
        let (hi, lo) = ((code >> 8) as u8, code as u8);
        let g;
        if hi == 0x23 {
            i += 2;
            continue;
        } else if hi == 0x25 {
            i += 2;
            match percent_glyph(lo) {
                Some(x) => g = x,
                None => continue,
            }
        } else if (0x20..0x80).contains(&hi) {
            g = hi as usize - 0x20;
            i += 1;
        } else {
            g = 63;
            i += 1;
        }
        last_face = Some(f);
        let afi = fonts.trim(kt, g);
        let src = Fonts::glyph_src(f, g);
        if kt.proportional() {
            let dst = row0(l) + c * cwb - c_div(aflp, 2);
            let afo = c_div(f.cw as i32 + 1 - afi, 2);
            // Only %X-%Z, whose trims come from past the table, can put dst
            // outside the texture; the game writes there, this does not.
            // The first glyph of the last line starts at byte 0 (texture row
            // 0 is the bottom line's bottom row).
            let wild = dst - (rows - 1) * tw < 0 || dst + afo > len;
            if !wild {
                for r in 0..rows {
                    let d = dst - r * tw;
                    let sr = |j: i32| fonts.byte(f, src + r as usize * f.stride + j as usize);
                    if aflp & 1 != 0 {
                        for j in 0..afo.max(0) {
                            let b = sr(j);
                            // Half a byte to the left: at byte 0 of the
                            // texture the game writes the byte before it.
                            if d + j >= 1 {
                                let k = (d + j - 1) as usize;
                                tex[k] = (tex[k] & 0x0f) | ((b << 4) & 0xf0);
                            }
                            tex[(d + j) as usize] = b >> 4;
                        }
                    } else {
                        for j in 0..afo.max(0) {
                            tex[(d + j) as usize] = sr(j);
                        }
                    }
                }
            }
            aflp += afi;
        } else {
            let dst = row0(l) + c * cwb;
            let afo = c_div(afi, 4);
            for r in 0..rows {
                let sr = src + r as usize * f.stride;
                let d = dst - r * tw;
                for j in 0..afo.max(0) {
                    if d + j < len {
                        tex[(d + j) as usize] = 0;
                    }
                }
                for j in afo.max(0)..cwb {
                    tex[(d + j) as usize] = fonts.byte(f, (sr as i32 + j - afo) as usize);
                }
            }
        }
        clm += 1;
        c += 1;
        if c >= per_row as i32 {
            if kt.proportional() {
                tail(tex, c, l, aflp, f);
            }
            aflp = 0;
            c = 0;
            l += 1;
            if l >= lines {
                return clm;
            }
        }
    }
    if c != 0
        && kt.proportional()
        && let Some(face) = last_face
    {
        tail(tex, c, l, aflp, face);
    }
    clm
}

/// One SPRITE MakePacketStr emits, in GS 12.4 units (x, y include the
/// primitive offset; u, v are texels x 16).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quad {
    pub rgba: [u8; 4],
    pub uv0: (i32, i32),
    pub xy0: (i32, i32),
    pub uv1: (i32, i32),
    pub xy1: (i32, i32),
    pub shadow: bool,
}

/// A `ccKanji`: its texture and the ccSprite fields Disp and MakePacketStr
/// use.
#[derive(Clone, Debug)]
pub struct Kanji {
    pub kt: Kt,
    /// Texture rows.
    pub th: usize,
    pub colour: [u8; 4],
    /// ccSprite.transp: multiplies the alpha.
    pub transp: f32,
    pub dx: f32,
    pub dy: f32,
    pub cx: f32,
    pub cy: f32,
    pub packet_max: usize,
    pub alpha_blend: usize,
    /// ctrl 0x10: the drop shadow (on after `Init`).
    pub shadow: bool,
    /// The rasterised text, 64 bytes a row.
    pub tex: Vec<u8>,
    pub clm: usize,
    // Per-run sprite fields.
    sx: f32,
    sy: f32,
    su: i32,
    sv: i32,
    wu: i32,
    wv: i32,
    packet_num: usize,
}

impl Kanji {
    /// `ccKanji::Init(l, m)` (0x0015d0f0): a `16 << l` row texture (l at
    /// most 3) and `m` packets.
    pub fn init(l: u32, m: usize) -> Self {
        Kanji::new(Kt::SmallProportional, 16 << l.min(3), m)
    }

    pub fn new(kt: Kt, th: usize, packet_max: usize) -> Self {
        Kanji {
            kt,
            th,
            colour: [128, 128, 128, 128],
            transp: 1.0,
            dx: 0.0,
            dy: 0.0,
            cx: 0.0,
            cy: 0.0,
            packet_max,
            alpha_blend: 0,
            shadow: CTRL_KANJI & 0x10 != 0,
            tex: vec![0; TEX_W * th],
            clm: 0,
            sx: 0.0,
            sy: 0.0,
            su: 0,
            sv: 0,
            wu: 0,
            wv: 0,
            packet_num: 0,
        }
    }

    /// ccSprite::MakePacketStr(str, t) in raw mode for the one cell a run
    /// draws (code 0): the shadow then the text, each culled against the
    /// screen; moves dx on by sx.
    fn run_packet(&mut self, screen: &LayerView, out: &mut Vec<Quad>) {
        let (mut x, mut y) = screen.apply(self.dx + self.cx, self.dy + self.cy);
        let w = crate::eef::to_int(crate::eef::mul(self.sx, screen.sx));
        let h = crate::eef::to_int(crate::eef::mul(self.sy, screen.sy));
        let [r, g, b, a] = self.colour;
        let a = crate::eef::to_int(crate::eef::mul(crate::eef::from_int(i32::from(a)), self.transp)) as u8;
        let main = [r, g, b, a];
        let shade = [SHADOW[0], SHADOW[1], SHADOW[2], a];
        let (su16, sv16, th16) = (self.su * 16, self.sv * 16, self.th as i32 * 16);
        for pass in 0..2 {
            if self.packet_num >= self.packet_max {
                return;
            }
            let col = if pass == 0 {
                if !self.shadow {
                    continue;
                }
                x = crate::eef::to_int(crate::eef::add(crate::eef::from_int(x), screen.sx));
                y = crate::eef::to_int(crate::eef::add(crate::eef::from_int(y), screen.sy));
                shade
            } else {
                x = crate::eef::to_int(crate::eef::sub(crate::eef::from_int(x), screen.sx));
                y = crate::eef::to_int(crate::eef::sub(crate::eef::from_int(y), screen.sy));
                main
            };
            if x > CULL_X1 || x + w < XYOFFSET_X || y > CULL_Y1 || y + h < XYOFFSET_Y {
                continue;
            }
            let u0 = self.wu;
            let v0 = th16 - self.wv - 1;
            let (u1, v1) = (u0 + su16, v0 - (sv16 - 1));
            out.push(Quad {
                rgba: col,
                uv0: (u0, v0),
                xy0: (x, y),
                uv1: (u1, v1),
                xy1: (x + w, y + h),
                shadow: pass == 0,
            });
            self.packet_num += 1;
        }
        self.dx += self.sx;
    }

    /// `ccKanji::Disp(str, c)` (0x0015e380): draw at most `c` glyphs (all
    /// when negative). Rasterises into [`Kanji::tex`] and returns the quads.
    pub fn disp(&mut self, fonts: &Fonts, raw: &[u8], c: i32, screen: &LayerView, names: &Names) -> Vec<Quad> {
        if c == 0 {
            return Vec::new();
        }
        let strn = expand(raw, names);
        let mut tex = std::mem::take(&mut self.tex);
        self.clm = extract(fonts, &strn, self.kt, self.th, &mut tex);
        self.tex = tex;
        let c = if c < 0 || c as usize > self.clm { self.clm } else { c as usize };
        let kt = self.kt;
        let f = kt.face();
        let (cw, ch, per_row) = (f.cw as i32, f.ch as i32, f.per_row);
        let adv = |g: usize| -> i32 {
            match kt {
                Kt::LargeProportional | Kt::SmallProportional => cw - fonts.trim(kt, g),
                _ => cw,
            }
        };
        let start = self.colour;
        let mut quads = Vec::new();
        let s = pad(&strn);
        let (mut i, mut glyphs, mut run, mut row_used, mut run_u, mut run_w) =
            (0usize, 0usize, 0usize, 0usize, 0i32, 0i32);
        let mut pending = 0u8;
        loop {
            let flush;
            let mut end = false;
            let chr = s[i];
            if chr == 0 || glyphs >= c {
                end = true;
                flush = run != 0;
            } else if chr == 0x23 {
                pending = s[i + 1];
                flush = run != 0;
                i += 2;
            } else if chr == 0x25 {
                let g = percent_glyph(s[i + 1]);
                i += 2;
                let Some(g) = g else { continue };
                run_w += adv(g);
                glyphs += 1;
                run += 1;
                flush = row_used + run >= per_row;
            } else {
                glyphs += 1;
                run += 1;
                flush = row_used + run >= per_row;
                run_w += adv(if (0x20..0x80).contains(&chr) { chr as usize - 0x20 } else { 63 });
                i += 1;
            }
            if flush {
                match kt {
                    Kt::LargeProportional => {
                        self.sx = run_w as f32;
                        self.su = run_w - 1;
                    }
                    Kt::SmallProportional => {
                        self.sx = (run_w - 1) as f32;
                        self.su = run_w;
                    }
                    _ => {
                        self.sx = run_w as f32;
                        self.su = run_w;
                    }
                }
                self.sy = ch as f32;
                self.sv = ch;
                self.wu = run_u * 16;
                self.wv = c_div(glyphs as i32 - 1, per_row as i32) * ch * 16;
                run_u += run_w;
                run_w = 0;
                self.run_packet(screen, &mut quads);
                if kt == Kt::SmallProportional {
                    self.dx += 1.0;
                }
                row_used += run;
                if row_used >= per_row {
                    row_used = 0;
                    run_u = 0;
                }
                run = 0;
            }
            if end {
                break;
            }
            if pending != 0 {
                let alpha = self.colour[3];
                let colour = match pending {
                    b'W' => Some(start),
                    b'R' => Some(SPRITE_COLOR_TABLE[18]),
                    b'G' => Some(SPRITE_COLOR_TABLE[20]),
                    b'B' => Some(SPRITE_COLOR_TABLE[17]),
                    b'Y' => Some(SPRITE_COLOR_TABLE[6]),
                    _ => None,
                };
                if let Some(col) = colour {
                    self.colour = col;
                }
                self.colour[3] = alpha;
                pending = 0;
            }
        }
        self.colour = start;
        self.packet_num = 0;
        quads
    }

    /// The texture as an upload with id `id`.
    pub fn upload(&self, id: u32, fonts: &Fonts) -> Upload {
        Upload {
            id,
            width: TEX_TEXELS,
            height: self.th as u16,
            format: UploadFormat::Psmt4,
            pixels: self.tex.clone(),
            clut: fonts.palette.clone(),
        }
    }
}

/// A MakePacketStr quad as a GS SPRITE: the text texture sampled MODULATE
/// with its alpha, blended by `blend`.
pub fn quad_prim(q: &Quad, screen: &LayerView, tex: u32, blend: Blend) -> Prim {
    let px = |x: i32| (x - XYOFFSET_X) as f32 / 16.0;
    let py = |y: i32| (y - XYOFFSET_Y) as f32 / 16.0;
    let rgba = Rgba(q.rgba);
    let v = |x: i32, y: i32, u: i32, vv: i32| Vertex {
        x: px(x),
        y: py(y),
        z: 0,
        u: u as f32 / 16.0,
        v: vv as f32 / 16.0,
        rgba,
    };
    let mut state = DrawState::sprite(blend, Some(TexState::modulate(TexRef::Upload(tex))));
    state.scissor = screen.scissor;
    Prim {
        kind: PrimKind::Sprite,
        gouraud: false,
        state,
        verts: vec![v(q.xy0.0, q.xy0.1, q.uv0.0, q.uv0.1), v(q.xy1.0, q.xy1.1, q.uv1.0, q.uv1.1)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separate_skips_lines() {
        let buf = b"ab\0c\x83\xa2\0d\0";
        assert_eq!(str_separate(buf, 0), 0);
        assert_eq!(str_separate(buf, 1), 3);
        assert_eq!(str_separate(buf, 2), 7);
    }

    #[test]
    fn expand_maps_names_and_sjis() {
        let n = Names::default();
        assert_eq!(expand(b"#0 \x83\xa2x\x88\x9f", &n), b"Kite %0x__".to_vec());
    }

    #[test]
    fn width_of_space_and_i() {
        let n = Names::default();
        let f = Fonts::of(Volume::Inf, Vec::new());
        assert_eq!(str_width(&f, b" ", Kt::SmallProportional, &n), 7);
        assert_eq!(str_width(&f, b"i", Kt::SmallProportional, &n), 3);
        assert_eq!(str_width(&f, b"i", Kt::LargeProportional, &n), 5);
    }
}
