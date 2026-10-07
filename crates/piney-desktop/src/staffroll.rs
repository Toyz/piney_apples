//! The staff roll: `ccThStaffRoll` (desktop.prg 0x004125c0), the task the
//! event instruction `staff_roll` runs on the desktop at the story's end, and
//! its `ccThStaffRollCtrl` (5092 bytes) of 19 `ccThStaffRollLine`s (260 bytes
//! each), on layer 200 with BGM.BIN track 1. Each page of `g_srDataGrp`
//! (0x0042c500) runs `_Random`, `_Fix`, `_End` or `_BGOnly` by `status`
//! (+0x1390). The pages, scene file and timings are the volume's
//! (`piney_data::tables::staffroll`). [`StaffRoll`] records each frame's draws
//! ([`Draw`]); [`StaffRoll::render`] turns them into primitives.

use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_data::tables::sjis::encode;

use crate::eef::{add, div, from_int, le, lt, mul, sub};

/// The volume's staff roll: `g_srDataGrp`, `SR_CCS_NAME` and the
/// `SR_*_TIME`s (`piney_data::tables::staffroll::of`).
pub type Tables = piney_data::tables::staffroll::StaffRoll;
/// `ccSpriteColorTable`'s entries for the text and the back shadow.
const TEXT_COLOUR: usize = 7;
const SHADOW_COLOUR: usize = 15;
/// The layer the controller makes (`ccLayer::Init(200, 0)`).
pub const LAYER: i16 = 200;
/// Lines on a page.
pub const LINES: usize = 19;
/// Characters on a line.
pub const CHARS: usize = 44;

/// A credit as a line builds it: its column (+0x00, -1 centred, -2 against
/// the right) and its text (+0x08, with `#0` / `#1` for the save's names).
struct Credit {
    col: i32,
    code: Vec<u8>,
}

/// `ccTransCode2Name(dst, src, n)` (0x00412410): `src` into at most `n & !1`
/// bytes; `#0` is the save's name (+0), `#1` its second (+24), another `#x`
/// a 0 byte and only the `#` passed over.
pub fn code_to_name(src: &[u8], n: usize, name0: &[u8], name1: &[u8]) -> Vec<u8> {
    let m = n & !1;
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    let mut out = Vec::new();
    let mut i = 0;
    while i < m || out.len() < m {
        let c = at(i);
        if c == 0 {
            break;
        }
        if c == b'#' {
            match at(i + 1) {
                b'0' | b'1' => {
                    let name = if at(i + 1) == b'0' { name0 } else { name1 };
                    for &b in name {
                        if out.len() >= m || b == 0 {
                            break;
                        }
                        out.push(b);
                    }
                    i += 2;
                }
                _ => {
                    out.push(0);
                    i += 1;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// A credit on a line: `SetLink`'s node (528 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// +0x1d8: the column times 12.
    pub x: i32,
    /// +0x1dc: the line's y.
    pub y: i32,
    /// +0x1e0: the name as placed (45 bytes, 0-padded).
    pub text: [u8; 45],
    /// The credit's `code`, for `Except`'s `strlen`.
    pub code_len: usize,
}

/// `ccThStaffRollLine`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// +0x0c: what `Draw` shows.
    pub buf: [u8; 45],
    /// +0x39: the credits composed into 44 columns.
    pub text: [u8; 45],
    /// +0x66: the characters settled (from `text`, or a random one frozen).
    pub fixed: [u8; 45],
    /// +0x93: which of the 44 have settled; the 45th byte is +0xbf's first,
    /// which `StopChar` writes when every one had settled already.
    pub stopped: [u8; 45],
    /// +0xec.
    pub index: i32,
    /// +0xf0, +0xf4: the text's and the picture's alpha as last set.
    pub alpha: u32,
    pub bg: u32,
    /// +0xf8, +0xfc.
    pub x: i32,
    pub y: i32,
    /// +0x100: the credits linked, in order.
    pub nodes: Vec<Node>,
}

impl Line {
    fn new(index: i32) -> Line {
        Line {
            buf: [0; 45],
            text: [0; 45],
            fixed: [0; 45],
            stopped: [0; 45],
            index,
            alpha: 1.0f32.to_bits(),
            bg: 0,
            x: -262,
            y: index * 20 - 192,
            nodes: Vec::new(),
        }
    }

    /// `Init` (0x00413b10).
    fn init(&mut self) {
        self.alpha = 1.0f32.to_bits();
        self.nodes.clear();
        self.buf = [0; 45];
        self.text = [0; 45];
        self.fixed = [0; 45];
        self.stopped = [0; 45];
    }

    fn all_stopped(&self) -> bool {
        self.stopped[..CHARS].iter().all(|&b| b != 0)
    }

    /// `StopChar(n)` (0x00413d50).
    fn stop_char(&mut self, n: i32, rand: &mut CRand) {
        for _ in 0..n.max(0) {
            let mut k = None;
            for _ in 0..CHARS {
                let r = (rand.rand() % CHARS as i32).unsigned_abs() as usize;
                if self.stopped[r] != 1 {
                    k = Some(r);
                    break;
                }
            }
            let k = k.unwrap_or_else(|| self.stopped[..CHARS].iter().position(|&b| b == 0).unwrap_or(CHARS));
            self.stopped[k] = 1;
        }
    }

    /// `Build` (0x00413e50).
    fn build(&mut self, credits: &[Credit], names: (&[u8], &[u8])) {
        self.text = [0; 45];
        if self.nodes.is_empty() {
            return;
        }
        for (node, c) in self.nodes.iter_mut().zip(credits) {
            let mut name = code_to_name(&c.code, CHARS, names.0, names.1);
            name.truncate(CHARS);
            node.text = [0; 45];
            let len = name.iter().position(|&b| b == 0).unwrap_or(name.len()) as i32;
            let col = match c.col {
                -1 => (41 - len) / 2,
                -2 => 42 - len,
                v if v <= 0 => 1,
                v => v,
            };
            if col < 42 {
                let n = if col + len >= 43 { 42 - col } else { len };
                // A centred name over 41 characters starts before the line (the
                // game writes before the buffer); those bytes are dropped.
                for i in 0..n.max(0) {
                    let b = name.get(i as usize).copied().unwrap_or(0);
                    if let Some(t) = usize::try_from(col + i).ok().and_then(|k| self.text.get_mut(k)) {
                        *t = b;
                    }
                    if let Some(t) = node.text.get_mut(i as usize) {
                        *t = b;
                    }
                }
            }
            node.x = col * 12;
            node.y = self.y;
        }
        self.text[CHARS] = 0;
        self.fixed = self.text;
    }

    /// `Except` (0x00414100).
    fn except(&mut self) {
        for node in &self.nodes {
            let v = node.x;
            if v <= 0 || v >= 42 {
                continue;
            }
            // v * 0x2aaaaaab >> 32 is v / 6, and the sra by 1 after it halves
            // that: the column.
            let s = v / 12;
            let len = node.code_len as i32;
            let n = if s + len >= 45 { 44 - s } else { len };
            for i in s..s + n {
                let i = i as usize;
                if i < 45 && self.text[i] != b' ' {
                    self.buf[i] = b' ';
                }
            }
        }
    }
}

/// newlib's `rand` (main 0x00133a38), which `StopChar` draws from and the
/// controller seeds (`srand`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CRand(pub u64);

impl CRand {
    pub fn rand(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (self.0 >> 32 & 0x7fff_ffff) as i32
    }
}

/// `ccRand` (main 0x001d9a10): the game's Mersenne Twister, `genrand`
/// (0x001d9620), which the random characters draw from. The same generator
/// as `piney_world::mt::Mt`; the desktop crate keeps its own copy.
#[derive(Clone, PartialEq, Eq)]
pub struct CcRand {
    pub mt: Box<[u32; 624]>,
    pub mti: usize,
}

impl std::fmt::Debug for CcRand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CcRand {{ mti: {} }}", self.mti)
    }
}

impl Default for CcRand {
    /// As a fresh boot leaves it (`mti` 625): the first draw seeds with
    /// 4352.
    fn default() -> Self {
        CcRand { mt: Box::new([0; 624]), mti: 625 }
    }
}

impl CcRand {
    /// `sgenrand(seed)`.
    pub fn seeded(seed: u32) -> CcRand {
        let mut mt = Box::new([0u32; 624]);
        let mut s = seed;
        for w in mt.iter_mut() {
            *w = s & 0xffff_0000;
            s = s.wrapping_mul(69069).wrapping_add(1);
            *w |= (s & 0xffff_0000) >> 16;
            s = s.wrapping_mul(69069).wrapping_add(1);
        }
        CcRand { mt, mti: 624 }
    }

    /// `genrand` as a signed int.
    pub fn rand(&mut self) -> i32 {
        const MAG01: [u32; 2] = [0, 0x9908_b0df];
        if self.mti >= 624 {
            if self.mti == 625 {
                *self = CcRand::seeded(0x1100);
            }
            let mt = &mut self.mt;
            for k in 0..624 {
                let y = (mt[k] & 0x8000_0000) | (mt[(k + 1) % 624] & 0x7fff_ffff);
                mt[k] = mt[(k + 397) % 624] ^ (y >> 1) ^ MAG01[(y & 1) as usize];
            }
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y as i32
    }
}

/// A draw the controller made this frame, as the game's calls take it.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    /// `ccView::SetFrame(0, 0, 512, 384, 256, 192, sx, sy)` on the layer's
    /// view (float bits).
    Frame { sx: u32, sy: u32 },
    /// `SetLayerCenter(256, 192)`.
    Centre,
    /// `mask.SetTex(SR_CCS_NAME, tex, 1)`: the page's picture.
    Tex(String),
    /// `ccKanji::Disp(text, -1, 1, 1)` of a line's sprite (`line`, `node`
    /// None) or a credit's (`node`), at (x, y) with `transp` alpha, in
    /// colour 7 (the text) or 15 (the back shadow).
    Text { line: usize, node: Option<(usize, bool)>, x: u32, y: u32, alpha: u32, text: Vec<u8>, shadow: bool },
    /// The page's picture: the mask's `MakePacket(0, flag)` then
    /// `SendPacket`, with its fields. The flag is 1 in every phase: `_Fix`
    /// and `_End` leave the 1 they stored in `wi` (+0x60) in `$a2`.
    Picture { alpha: u32, dx: u32, dy: u32, sx: u32, sy: u32, su: i32, sv: i32, flag: i32 },
}

/// `ccThStaffRollCtrl`.
#[derive(Clone, Debug)]
pub struct StaffRoll {
    pub tables: &'static Tables,
    pub lines: Vec<Line>,
    /// +0x1390, +0x1394, +0x1398, +0x139c.
    pub status: i32,
    pub count: i32,
    pub last: i32,
    pub sub: i32,
    /// +0x13a0, +0x13a4, +0x13a8.
    pub page: i32,
    pub pages: i32,
    pub done: bool,
    /// +0x13cc, +0x13d0: the view's scale (float bits).
    pub sx: u32,
    pub sy: u32,
    /// +0x13d4, +0x13d8: the text's and the picture's alpha.
    pub alpha: u32,
    pub bg: u32,
    /// +0x13dc, +0x13e0.
    pub stop_every: i32,
    pub stop_n: i32,
    /// The page's picture (`mask.SetTex(SR_CCS_NAME, tex, 1)`).
    pub tex: String,
    pub rand: CRand,
    pub cc_rand: CcRand,
    /// The save's two names for `#0` and `#1`.
    pub names: (Vec<u8>, Vec<u8>),
    /// This frame's draws.
    pub draws: Vec<Draw>,
}

fn f(v: u32) -> f32 {
    f32::from_bits(v)
}

impl StaffRoll {
    /// `ccThStaffRollCtrl()` (0x004126d0): `seed` is `ccSys+0x358` (the
    /// frame count `srand` takes), `cc_rand` the game's `ccRand` as it is.
    pub fn new(tables: &'static Tables, seed: u32, cc_rand: CcRand, names: (Vec<u8>, Vec<u8>)) -> StaffRoll {
        let pages = tables.pages.len() as i32;
        let t = tables.rnd_fadein;
        let mut s = StaffRoll {
            tables,
            lines: (0..LINES as i32).map(Line::new).collect(),
            status: 0,
            count: 0,
            last: 0,
            sub: 0,
            page: 0,
            pages,
            done: false,
            sx: 1.0f32.to_bits(),
            sy: 1.0f32.to_bits(),
            alpha: 0,
            bg: 0,
            stop_every: 0,
            stop_n: 0,
            tex: String::new(),
            rand: CRand(u64::from(seed)),
            cc_rand,
            names,
            draws: vec![Draw::Frame { sx: 1.0f32.to_bits(), sy: 1.0f32.to_bits() }],
        };
        s.change_status(3);
        s.change_page(0);
        s.done = false;
        s.stop_n = if t != 0 { (44 / t).max(1) } else { 1 };
        s.stop_every = t / 44;
        s
    }

    fn change_status(&mut self, st: i32) {
        self.status = st;
        self.count = 0;
        self.last = 0;
        self.sub = 0;
    }

    /// `ChangePage(p)` (0x00412b70).
    fn change_page(&mut self, p: i32) {
        let s = if p == 0 || p == self.pages - 1 { 1.0f32 } else { 5.0 };
        self.sx = s.to_bits();
        self.sy = s.to_bits();
        self.draws.push(Draw::Frame { sx: self.sx, sy: self.sy });
        self.draws.push(Draw::Centre);
        self.alpha = 0;
        if p >= self.pages {
            self.done = true;
            return;
        }
        let page = &self.tables.pages[p as usize];
        self.tex = page.bg_name.unwrap_or_default().to_string();
        self.draws.push(Draw::Tex(self.tex.clone()));
        self.bg = 0;
        self.page = p;
        match page.data {
            Some(_) => {
                self.change_status(0);
                self.build_page(p);
            }
            None => self.change_status(3),
        }
    }

    /// `BuildPage(p)` (0x00412d10).
    fn build_page(&mut self, p: i32) {
        for l in &mut self.lines {
            l.init();
        }
        let credits = self.tables.pages[p as usize].data.unwrap_or_default();
        let mut per_line: Vec<Vec<Credit>> = (0..LINES).map(|_| Vec::new()).collect();
        for c in credits {
            // +0x04: the line, -1 the ninth, -2 the eighteenth.
            let i = match c.y {
                -1 => 9,
                -2 => 18,
                v => v,
            };
            let Some(line) = usize::try_from(i).ok().filter(|&i| i < LINES) else { continue };
            let code = encode(c.str.unwrap_or_default());
            self.lines[line].nodes.push(Node { x: 0, y: 0, text: [0; 45], code_len: code.len() });
            per_line[line].push(Credit { col: c.x, code });
        }
        let names = (self.names.0.clone(), self.names.1.clone());
        for (l, cs) in self.lines.iter_mut().zip(&per_line) {
            l.build(cs, (&names.0, &names.1));
        }
    }

    /// `Main` (0x00412a80): one frame.
    pub fn main(&mut self) {
        self.draws.clear();
        if self.page >= self.pages {
            return;
        }
        match self.status {
            0 => self.random(),
            1 => self.fix(),
            2 => self.end(),
            3 => self.bg_only(),
            _ => self.change_status(0),
        }
    }

    fn draw_line(&mut self, i: usize, src: Option<&[u8; 45]>) {
        let l = &mut self.lines[i];
        if let Some(src) = src {
            l.buf = *src;
            for (k, &r) in src.iter().enumerate().take(CHARS) {
                if l.stopped[k] == 0 {
                    continue;
                }
                let c = l.fixed[k];
                if c != 0 && c != b' ' {
                    l.buf[k] = c;
                } else {
                    l.fixed[k] = r;
                    l.buf[k] = r;
                }
            }
        }
        let text = l.buf[..l.buf.iter().position(|&b| b == 0).unwrap_or(45)].to_vec();
        self.draws.push(Draw::Text {
            line: i,
            node: None,
            x: from_int(l.x).to_bits(),
            y: from_int(l.y).to_bits(),
            alpha: l.alpha,
            text,
            shadow: false,
        });
    }

    fn draw_fix(&mut self, i: usize) {
        let l = &self.lines[i];
        for (k, n) in l.nodes.iter().enumerate() {
            let text = n.text[..n.text.iter().position(|&b| b == 0).unwrap_or(45)].to_vec();
            self.draws.push(Draw::Text {
                line: i,
                node: Some((k, false)),
                x: from_int(n.x - 262).to_bits(),
                y: from_int(n.y).to_bits(),
                alpha: l.alpha,
                text,
                shadow: false,
            });
        }
    }

    fn draw_back_shadow(&mut self, i: usize) {
        let l = &self.lines[i];
        let a = mul(0.5, f(l.bg)).to_bits();
        for (k, n) in l.nodes.iter().enumerate() {
            let text = n.text[..n.text.iter().position(|&b| b == 0).unwrap_or(45)].to_vec();
            self.draws.push(Draw::Text {
                line: i,
                node: Some((k, true)),
                x: from_int(n.x - 261).to_bits(),
                y: from_int(n.y + 1).to_bits(),
                alpha: a,
                text,
                shadow: true,
            });
        }
    }

    fn picture(&mut self, alpha: u32, last_page_strip: bool, flag: i32) {
        let (dx, dy, sx, sy, su, sv) = if last_page_strip {
            (-256.0f32, -7.0f32, 512.0f32, 20.0f32, 512, 20)
        } else {
            (-256.0, -192.0, 512.0, 384.0, 512, 512)
        };
        self.draws.push(Draw::Picture {
            alpha,
            dx: dx.to_bits(),
            dy: dy.to_bits(),
            sx: sx.to_bits(),
            sy: sy.to_bits(),
            su,
            sv,
            flag,
        });
    }

    /// `_Random` (0x00412e50).
    fn random(&mut self) {
        let t = from_int(self.tables.rnd_fadein);
        let mut a = f(self.alpha);
        let mut sx = sub(f(self.sx), div(5.0, t));
        let mut sy = sub(f(self.sy), div(5.0, t));
        if lt(sx, 1.0) {
            sx = 1.0;
        }
        if lt(sy, 1.0) {
            sy = 1.0;
        }
        self.sx = sx.to_bits();
        self.sy = sy.to_bits();
        a = add(a, div(1.0, t));
        if !le(a, 1.0) {
            a = 1.0;
        }
        self.alpha = a.to_bits();
        self.draws.push(Draw::Frame { sx: self.sx, sy: self.sy });
        for i in 0..LINES {
            let mut buf = [0u8; 45];
            for b in buf.iter_mut().take(CHARS) {
                let mut c = (self.cc_rand.rand().wrapping_abs() % 94 + 33) as u8;
                if c == b'#' {
                    c = b'$';
                }
                if c == b'%' {
                    c = b'&';
                }
                *b = c;
            }
            self.lines[i].alpha = self.alpha;
            self.draw_line(i, Some(&buf));
        }
        let mut all = true;
        if self.stop_every == self.count - self.last {
            self.last = self.count;
            for l in &mut self.lines {
                if !l.all_stopped() {
                    all = false;
                    l.stop_char(self.stop_n, &mut self.rand);
                }
            }
        } else if self.tables.rnd_fadein < self.count + self.stop_every {
            for l in &mut self.lines {
                if !l.all_stopped() {
                    l.stop_char(CHARS as i32, &mut self.rand);
                }
            }
            all = true;
        }
        self.count += 1;
        if all && f(self.alpha) == 1.0 {
            self.change_status(1);
            for l in &mut self.lines {
                l.except();
            }
        }
    }

    /// `_Fix` (0x00413210).
    fn fix(&mut self) {
        let a = f(self.alpha);
        let b = f(self.bg);
        for i in 0..LINES {
            self.lines[i].alpha = 1.0f32.to_bits();
            self.draw_fix(i);
            self.lines[i].alpha = a.to_bits();
            self.draw_line(i, None);
        }
        for i in 0..LINES {
            self.lines[i].bg = b.to_bits();
            self.draw_back_shadow(i);
        }
        self.picture(b.to_bits(), false, 1);
        if a == 0.0 && b == 1.0 {
            self.change_status(2);
            self.alpha = 1.0f32.to_bits();
            return;
        }
        let mut a = sub(a, div(1.0, from_int(self.tables.rnd_fadeout)));
        if lt(a, 0.0) {
            a = 0.0;
        }
        self.alpha = a.to_bits();
        let mut b = add(b, div(1.0, from_int(self.tables.bg_fadein)));
        if !le(b, 1.0) {
            b = 1.0;
        }
        self.bg = b.to_bits();
    }

    /// `_End` (0x00413410).
    fn end(&mut self) {
        let mut a = f(self.alpha);
        if self.sub == 0 {
            self.draws.push(Draw::Frame { sx: self.sx, sy: self.sy });
            let c = self.count;
            self.count = c + 1;
            if c >= self.tables.bg_fix {
                let step = div(1.0, from_int(self.tables.bg_fadeout));
                let mut sx = sub(f(self.sx), step);
                let mut sy = sub(f(self.sy), step);
                self.sx = sx.to_bits();
                self.sy = sy.to_bits();
                if lt(sx, 0.0) {
                    sx = 0.0;
                    self.sx = sx.to_bits();
                }
                if lt(sy, 0.0) {
                    sy = 0.0;
                    self.sy = sy.to_bits();
                }
                a = sub(a, step);
                if lt(a, 0.0) {
                    self.change_page(self.page + 1);
                    return;
                }
                self.alpha = a.to_bits();
                let mut b = sub(f(self.bg), step);
                if lt(b, 0.0) {
                    b = 0.0;
                }
                self.bg = b.to_bits();
            }
        }
        for i in 0..LINES {
            self.lines[i].alpha = a.to_bits();
            self.draw_fix(i);
        }
        let b = self.bg;
        for i in 0..LINES {
            self.lines[i].bg = b;
            self.draw_back_shadow(i);
        }
        self.picture(b, false, 1);
    }

    /// `_BGOnly` (0x00413690).
    fn bg_only(&mut self) {
        let mut b = f(self.bg);
        match self.sub {
            0 => {
                b = add(b, div(1.0, from_int(self.tables.onlybg_fadein)));
                if !le(b, 1.0) {
                    b = 1.0;
                    self.sub += 1;
                }
            }
            1 => {
                let c = self.count;
                self.count = c + 1;
                if c == self.tables.onlybg_fix {
                    self.sub += 1;
                }
            }
            2 => {
                b = sub(b, div(1.0, from_int(self.tables.onlybg_fadeout)));
                if lt(b, 0.0) {
                    self.change_page(self.page + 1);
                    return;
                }
            }
            _ => {}
        }
        let last = self.page == self.pages - 1;
        self.picture(self.bg, last, 1);
        self.bg = b.to_bits();
    }
}

/// The page's picture as `mask.SetTex(SR_CCS_NAME, tex, 1)` finds it: the
/// texture and its height.
pub fn picture_tex(archive: &piney_data::archive::Archive, ccs: &str, tex: &str) -> Option<(piney_draw::TexRef, i32)> {
    let file = ccs.to_ascii_lowercase();
    let c = piney_data::ccs::Ccs::parse(archive.inflate_named(&file).ok()?).ok()?;
    let texture = c.find_object(tex)?;
    let (textures, _) = piney_data::texture::read(&c).ok()?;
    let t = textures.iter().find(|t| t.object == texture)?;
    Some((piney_draw::TexRef::Ccs { file, texture, clut: t.clut }, 1 << t.th))
}

impl StaffRoll {
    /// The layer's view as the controller left it: `SetFrame(0, 0, 512,
    /// 384, 256, 192, sx, sy)` with the centre `ChangePage` set.
    pub fn view(&self) -> crate::view::LayerView {
        crate::view::LayerView::frame_centred(0.0, 0.0, 512.0, 384.0, f(self.sx), f(self.sy), 256.0, 192.0)
    }

    /// This frame's [`Draw`]s as primitives on [`LAYER`], in the game's
    /// order (each send prepended to the layer): the lines' text through
    /// `ccKanji` (the staff roll's font, `LargeFixed`), the page's picture
    /// through its mask.
    pub fn render(
        &self,
        ctx: &mut crate::anm::Ctx,
        fonts: &crate::kanji::Fonts,
        names: &crate::kanji::Names,
        picture: Option<&(piney_draw::TexRef, i32)>,
    ) {
        let view = self.view();
        for d in &self.draws {
            match d {
                Draw::Text { x, y, alpha, text, shadow, .. } => {
                    let mut k = crate::kanji::Kanji::init(3, 16);
                    k.kt = crate::kanji::Kt::LargeFixed;
                    // ccSprite takes an entry's low three bytes.
                    let c = SPRITE_COLOR_TABLE[if *shadow { SHADOW_COLOUR } else { TEXT_COLOUR }];
                    k.colour = [c[0], c[1], c[2], 0x80];
                    k.transp = f(*alpha);
                    k.dx = f(*x);
                    k.dy = f(*y);
                    ctx.disp(fonts, &mut k, LAYER, &view, text, names);
                }
                Draw::Picture { alpha, dx, dy, sx, sy, su, sv, .. } => {
                    let Some((tex, tex_h)) = picture else { continue };
                    let mut m = crate::sprite::Sprite::mask(LAYER, tex.clone(), *tex_h, 1);
                    m.transp = f(*alpha);
                    m.dx = f(*dx);
                    m.dy = f(*dy);
                    m.sx = f(*sx);
                    m.sy = f(*sy);
                    m.su = *su;
                    m.sv = *sv;
                    m.make_packet(0, &view);
                    m.send(&mut ctx.layers);
                }
                Draw::Frame { .. } | Draw::Centre | Draw::Tex(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_replace_their_codes() {
        let got = code_to_name(b"Player  #0 (#1)", 44, b"Kite", b"Tanaka");
        assert_eq!(got, b"Player  Kite (Tanaka)".to_vec());
        // Another `#x` writes a 0 and passes over the `#` only.
        assert_eq!(code_to_name(b"a#xb", 44, b"", b""), vec![b'a', 0, b'x', b'b']);
        // At most n & !1 bytes out.
        assert_eq!(code_to_name(b"abcdefg", 5, b"", b"").len(), 4);
    }

    #[test]
    fn crand_is_newlibs() {
        let mut r = CRand(1);
        let a = r.rand();
        let b = r.rand();
        assert_eq!(a, (6_364_136_223_846_793_006u64 >> 32 & 0x7fff_ffff) as i32);
        assert_ne!(a, b);
    }
}
