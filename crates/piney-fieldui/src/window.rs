//! `ccMenuWindow` (menuwin.cpp, main executable 0x001b7860 - 0x001bc2f0):
//! the window frames, buttons, scroll bars and the select cursor the menus
//! draw with, each a run of `MakePacket` calls on a sprite over
//! `xwindow::TEX_xwindo00`'s cell grids.
//!
//! The frames are small byte programs `DispLine` interprets (0x001b8260):
//! a cell code, a width in cells after codes 6, 1, 7, 21, 102 and 112, and
//! 0 followed by a count to skip cells; -1 ends a row.

use piney_desktop::eef::{add, div, from_int, mul, sub};

use crate::spr::Spr;

/// `ccMenuWindow::SetType(t)` (0x001b8150): the cell grid.
pub fn set_type(s: &mut Spr, t: i32) {
    match t {
        0 => s.set_grid(9, 16, 9.0, 16.0, 0, 512, 28),
        2 => s.set_grid(16, 16, 16.0, 16.0, 0, 768, 16),
        3 => s.set_grid(24, 24, 24.0, 24.0, 2048, 1024, 2),
        _ => s.set_grid(14, 16, 14.0, 16.0, 0, 0, 18),
    }
}

/// Row programs of the frames (`@1332`, `@1335`, `@1336`): top, middle,
/// bottom, each a corner, a stretched edge `w` cells wide, a corner.
pub const TOP: [i8; 5] = [2, 6, 0, 3, -1];
pub const MIDDLE: [i8; 5] = [8, 1, 0, 9, -1];
pub const BOTTOM: [i8; 5] = [4, 7, 0, 5, -1];

/// `DispLine(mode, n, prog)` (0x001b8260). `title` is mode 0's string.
pub fn disp_line(s: &mut Spr, mode: i32, n: i32, prog: &[i8]) {
    let (x0, y0) = (s.dx, s.dy);
    let mut k = 0usize;
    let next = |k: &mut usize| -> i8 {
        let b = prog.get(*k).copied().unwrap_or(-1);
        *k += 1;
        b
    };
    match mode {
        1..=4 => {
            // The wide code of each row and the row's height.
            let (wide, h) = match mode {
                1 => (6, s.sv as f32),
                2 => (7, s.sv as f32),
                3 => (1, from_int(s.sv + 20 * (n - 1))),
                _ => (21, s.sv as f32),
            };
            loop {
                let c = next(&mut k);
                if c == -1 {
                    break;
                }
                if c == 0 {
                    let m = next(&mut k);
                    s.dx = add(s.dx, from_int(s.su * i32::from(m)));
                    continue;
                }
                if c == wide {
                    let m = next(&mut k);
                    s.sx = from_int(s.su * i32::from(m));
                } else {
                    s.sx = from_int(s.su);
                }
                s.sy = h;
                s.make_packet(i32::from(c));
            }
            s.dx = x0;
            s.dy = add(y0, h);
        }
        6 => {
            loop {
                let c = next(&mut k);
                if c == -1 {
                    break;
                }
                if c == 0 {
                    let m = next(&mut k);
                    s.dx = add(s.dx, from_int(s.su * i32::from(m)));
                    continue;
                }
                let c = c as u8 as i32;
                if c >= 110 {
                    (s.wu, s.wv, s.wi, s.su, s.sv) = (2816, 1088, 5, 14, 16);
                } else if c >= 100 {
                    (s.wu, s.wv, s.wi, s.su, s.sv) = (2816, 768, 5, 14, 20);
                } else {
                    (s.wu, s.wv, s.wi, s.su, s.sv) = (0, 0, 18, 14, 16);
                }
                if c == 102 || c == 112 || c == 6 {
                    let m = next(&mut k);
                    s.sx = from_int(s.su * i32::from(m));
                } else {
                    s.sx = from_int(s.su);
                }
                s.sy = from_int(s.sv);
                let code = if c >= 110 {
                    c - 110
                } else if c >= 100 {
                    c - 100
                } else {
                    c
                };
                s.make_packet(code);
            }
            s.dx = x0;
            s.dy = add(y0, from_int(s.sv));
            set_type(s, 1);
        }
        5 => {
            // The name header: two rows of the name grid.
            (s.su, s.sv, s.wu, s.wv, s.wi) = (14, 16, 2816, 1344, 4);
            s.sx = from_int(s.su);
            s.sy = from_int(s.sv);
            s.make_packet(1);
            s.sx = from_int(s.su * n);
            s.sy = from_int(s.sv);
            s.make_packet(2);
            s.sx = from_int(s.su);
            s.sy = from_int(s.sv);
            s.make_packet(3);
            s.dx = x0;
            s.dy = add(s.dy, from_int(s.sv));
            s.sx = from_int(s.su);
            s.sy = from_int(s.sv);
            s.make_packet(4);
            s.sx = from_int(s.su * n);
            s.sy = from_int(s.sv);
            s.make_packet(5);
            s.sx = from_int(s.su);
            s.sy = from_int(s.sv);
            s.make_packet(6);
            s.dx = x0;
            s.dy = add(s.dy, from_int(s.sv));
            set_type(s, 1);
        }
        _ => {}
    }
}

/// `DispLine(0, n, title)`: a title in grid 0's letters over the frame's
/// top edge, from 6 left and 8 up.
pub fn disp_title(s: &mut Spr, title: &[u8]) {
    let (x0, y0) = (s.dx, s.dy);
    set_type(s, 0);
    s.dx = sub(s.dx, 6.0);
    s.dy = sub(s.dy, 8.0);
    s.make_packet(26);
    s.make_packet(38);
    for &c in title.iter().take_while(|&&c| c != 0) {
        let c = c as i8 as i32;
        if (48..58).contains(&c) {
            s.make_packet(c - 20);
        } else if (65..91).contains(&c) {
            s.make_packet(c - 65);
        } else {
            s.make_packet(38);
        }
    }
    s.make_packet(38);
    s.make_packet(27);
    s.dx = x0;
    s.dy = y0;
    set_type(s, 1);
}

fn row(template: [i8; 5], w: i32) -> [i8; 5] {
    let mut r = template;
    r[2] = w as i8;
    r
}

/// `DispSquare(w, h, title)` (0x001b8b70): the frame `w` cells wide inside,
/// `h` rows high; the title over it.
pub fn disp_square(s: &mut Spr, w: i32, h: i32, title: Option<&[u8]>) {
    set_type(s, 1);
    let (x0, y0) = (s.dx, s.dy);
    disp_line(s, 1, 0, &row(TOP, w));
    if h > 0 {
        disp_line(s, 3, h, &row(MIDDLE, w));
    }
    disp_line(s, 2, 0, &row(BOTTOM, w));
    if let Some(t) = title {
        s.dx = x0;
        s.dy = y0;
        disp_title(s, t);
    }
}

/// `DispSquareName(w, h)` (0x001b8c90): the speech frame with a name
/// header above.
pub fn disp_square_name(s: &mut Spr, w: i32, h: i32) {
    set_type(s, 1);
    let (x0, y0) = (s.dx, s.dy);
    s.dy = add(s.dy, 20.0);
    if h > 0 {
        disp_line(s, 3, h + 1, &row(MIDDLE, w));
    }
    disp_line(s, 2, 0, &row(BOTTOM, w));
    s.dx = x0;
    s.dy = y0;
    disp_line(s, 5, w, &[]);
}

/// `DispLineH(n)` (0x001b9a80): a horizontal rule `n` cells long between
/// its two ends (cells 19, 21 stretched, 20).
pub fn disp_line_h(s: &mut Spr, n: i32) {
    let (x0, y0) = (s.dx, s.dy);
    set_type(s, 1);
    let w = mul(14.0, from_int(n));
    let cell = |s: &mut Spr, code: i32, sx: f32, dx: f32| {
        s.sx = sx;
        s.sy = 16.0;
        s.dx = dx;
        s.dy = y0;
        s.make_packet(code);
    };
    cell(s, 19, 14.0, x0);
    let x1 = add(14.0, x0);
    cell(s, 21, w, x1);
    cell(s, 20, 14.0, add(x1, w));
}

/// `DispLineV(n)` (0x001b9b90): a vertical rule `n` rows long between its
/// two ends (cells 10, 22 stretched, 18).
pub fn disp_line_v(s: &mut Spr, n: i32) {
    let (x0, y0) = (s.dx, s.dy);
    set_type(s, 1);
    let h = from_int(n * 20);
    let cell = |s: &mut Spr, code: i32, sy: f32, dy: f32| {
        s.sx = 14.0;
        s.sy = sy;
        s.dx = x0;
        s.dy = dy;
        s.make_packet(code);
    };
    cell(s, 10, 16.0, y0);
    let y1 = add(16.0, y0);
    cell(s, 22, h, y1);
    cell(s, 18, 16.0, add(y1, h));
}

/// `DispSquareSB(w, h, n0, n1, _, title)` (0x001b9360): a frame one cell
/// wider with a scroll bar in it.
pub fn disp_square_sb(s: &mut Spr, w: i32, h: i32, dy: i32, my: i32, title: Option<&[u8]>) {
    set_type(s, 1);
    let (x0, y0) = (s.dx, s.dy);
    disp_line(s, 1, 0, &row(TOP, w + 1));
    if h > 0 {
        disp_line(s, 3, h, &row(MIDDLE, w + 1));
    }
    disp_line(s, 2, 0, &row(BOTTOM, w + 1));
    if let Some(t) = title {
        s.dx = x0;
        s.dy = y0;
        disp_title(s, t);
    }
    s.dx = x0;
    s.dy = y0;
    disp_scroll_bar(s, w + 1, h, dy, my);
}

/// `DispScrollBar(w, h, n, m)` (0x001b94d0): the trough and the thumb
/// (`n` of `m` rows scrolled, `h` shown) at the frame's right.
pub fn disp_scroll_bar(s: &mut Spr, w: i32, h: i32, n: i32, m: i32) {
    let x = add(s.dx, from_int((w + 1) * s.su));
    let y = s.dy;
    (s.su, s.sv, s.wu, s.wv, s.wi) = (8, 16, 3360, 0, 3);
    let mut len = from_int(20 * (h - 1));
    if len < 0.0 {
        len = 0.0;
    }
    let cell = |s: &mut Spr, code: i32, sy: f32, dy: f32| {
        s.sx = 8.0;
        s.sy = sy;
        s.dx = x;
        s.dy = dy;
        s.make_packet(code);
    };
    let top = add(8.0, y);
    cell(s, 3, 12.0, top);
    let mid = add(20.0, y);
    cell(s, 4, len, mid);
    let bottom = add(mid, len);
    cell(s, 5, 12.0, bottom);
    if h >= m {
        cell(s, 0, 12.0, top);
        cell(s, 1, add(0.5, len), mid);
        cell(s, 2, 12.0, bottom);
    } else {
        let y2 = add(y, div(mul(from_int(n), len), from_int(m)));
        let thumb = div(mul(from_int(h), len), from_int(m));
        cell(s, 0, 12.0, add(8.0, y2));
        let y3 = add(20.0, y2);
        cell(s, 1, add(0.5, thumb), y3);
        cell(s, 2, 12.0, add(y3, thumb));
    }
}

/// `DispSquareTag(w, h, n, m, sn)` (0x001b8d90): a frame with a row of `m`
/// tabs over it, tab `n` in front, `sn` cells wide.
pub fn disp_square_tag(s: &mut Spr, w: i32, h: i32, n: i32, m: i32, sn: i32) {
    set_type(s, 1);
    // Two rows of tab cells (the tab grid's codes 100-104 over the upper
    // row, 110-113 the lower), as the game builds them on its stack.
    let mut up: Vec<i8> = vec![0, 1];
    let mut lo: Vec<i8> = vec![2];
    let mut used = 1;
    for i in 0..m {
        if i < n {
            up.push(if i == 0 { 100 } else { 101 });
            lo.extend([6, 1]);
            used += 1;
        } else if i == n {
            up.push(if i == 0 { 100 } else { 101 });
            lo.push(111);
            used += 1;
            up.extend([102, sn as i8]);
            lo.extend([112, sn as i8]);
            used += sn;
            if i == m - 1 {
                up.push(104);
            } else {
                up.push(103);
            }
            lo.push(113);
            used += 1;
        } else if i == m - 1 {
            up.push(104);
            lo.extend([6, 1]);
            used += 1;
        } else {
            up.push(103);
            lo.extend([6, 1]);
            used += 1;
        }
    }
    up.push(-1);
    lo.extend([6, (w + 1 - used) as i8, 3, -1]);
    s.dy = sub(s.dy, 20.0);
    disp_line(s, 6, 0, &up);
    disp_line(s, 6, 0, &lo);
    s.sx = 14.0;
    s.sy = 16.0;
    if h > 0 {
        disp_line(s, 3, h, &row(MIDDLE, w));
    }
    disp_line(s, 2, 0, &row(BOTTOM, w));
}

/// `DispButton(bn)` (0x001b9ca0): a pad button's picture at (dx, dy),
/// pulsing with `ccSys.count`: scale `1 + (128 - b) / 300`, b rising from
/// 16 to 128 and back over 48 frames (24 at frame rate 2).
pub fn disp_button(s: &mut Spr, bn: i32, count: u32, frame_rate: u32) {
    let (half, full, m) = if frame_rate == 2 { (12.0, 24.0, 24) } else { (24.0, 48.0, 48) };
    let mut t = from_int((count % m) as i32);
    if t >= half {
        t = sub(full, t);
    }
    let b = piney_desktop::eef::to_int(add(16.0, div(mul(112.0, t), half)));
    let scale = add(div(sub(128.0, from_int(b)), 300.0), 1.0);
    let (x, y) = (s.dx, s.dy);
    let cell = |s: &mut Spr, ty: i32, cx: f32, cy: f32, code: i32| {
        s.dx = x;
        s.dy = y;
        set_type(s, ty);
        s.cx = cx;
        s.cy = cy;
        s.sx = mul(s.sx, scale);
        s.sy = mul(s.sy, scale);
        s.cx = mul(s.cx, scale);
        s.cy = mul(s.cy, scale);
        s.make_packet(code);
    };
    match bn {
        4..=7 => {
            let base = 32 + 2 * (bn - 4);
            cell(s, 2, -16.0, -8.0, base);
            cell(s, 2, 0.0, -8.0, base + 1);
        }
        0..=3 => cell(s, 2, -8.0, -8.0, 16 + bn),
        8 | 9 => cell(s, 2, -8.0, -8.0, bn),
        10..=13 => cell(s, 1, -7.0, -8.0, 24 + (bn - 10)),
        _ => {}
    }
    s.cx = 0.0;
    s.cy = 0.0;
}

/// `DispTarget(sn, df)` (0x001ba660): the target name's frame at (dx, dy),
/// `sn` glyphs; df 1 the party panel's frame.
pub fn disp_target(s: &mut Spr, sn: i32, df: i32) {
    if df == 0 {
        (s.wu, s.wv, s.wi, s.su, s.sv) = (0, 1536, 6, 16, 24);
        s.sx = 16.0;
        s.sy = 24.0;
        s.make_packet(0);
        let mut rest = 130.0;
        if sn != 0 {
            s.sx = 16.0;
            s.sy = 24.0;
            s.make_packet(3);
            let w = sub(mul(8.0, from_int(sn - 1)), 10.0);
            if w > 0.0 {
                s.sx = w;
                s.sy = 24.0;
                s.make_packet(4);
            }
            s.sx = 16.0;
            s.sy = 24.0;
            s.make_packet(5);
            rest = sub(rest, add(32.0, w));
        }
        if rest > 0.0 {
            s.sx = rest;
            s.sy = 24.0;
            s.make_packet(1);
        }
        s.sx = 16.0;
        s.sy = 24.0;
        s.make_packet(2);
        return;
    }
    (s.wu, s.wv, s.wi, s.su, s.sv) = (1024, 3072, 2, 90, 44);
    s.sx = 90.0;
    s.sy = 44.0;
    let x = s.dx;
    let y = s.dy;
    s.dy = add(44.0, s.dy);
    s.make_packet(1);
    s.dx = x;
    s.dy = y;
    s.make_packet(0);
    (s.wu, s.wv, s.wi, s.su, s.sv) = (0, 1536, 6, 16, 24);
    s.dx = sub(add(90.0, x), 40.0);
    s.dy = y;
    let mut rest = 100.0;
    if sn != 0 {
        let w = sub(mul(8.0, from_int(sn - 1)), 4.0);
        if w > 0.0 {
            s.sx = w;
            s.sy = 24.0;
            s.make_packet(4);
            rest = sub(rest, w);
        }
    }
    s.sx = 16.0;
    s.sy = 24.0;
    s.make_packet(5);
    if rest > 0.0 {
        s.sx = rest;
        s.sy = 24.0;
        s.make_packet(1);
    }
    disp_target_tail(s);
}

/// The party frame's right end (`DispTarget` df 1, 0x001ba9a4 on).
fn disp_target_tail(s: &mut Spr) {
    (s.wu, s.wv, s.wi, s.su, s.sv) = (640, 1536, 1, 8, 24);
    s.sx = 8.0;
    s.sy = 24.0;
    s.make_packet(0);
}

/// The select cursor's state in a `ccMenuWindow` (+0xd0 on).
#[derive(Clone, Debug)]
pub struct Cursor {
    /// +0xd0 `cursolSelNum`: -2 before the first draw, -1 after `InitCursol(1)`.
    pub sel: i32,
    /// +0xd2 `cursolIndex`: the bar that belongs to the selection.
    pub idx: usize,
    pub a: [i16; 12],
    pub w: [f32; 12],
    pub x: [f32; 12],
    pub y: [f32; 12],
    pub wx: [f32; 6],
    pub wy: [f32; 6],
    /// +0x1ac, +0x1ae: the page cursor's alpha and step.
    pub page_a: i16,
    pub page_da: i16,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            sel: -2,
            idx: 0,
            a: [0; 12],
            w: [0.0; 12],
            x: [0.0; 12],
            y: [0.0; 12],
            wx: [0.0; 6],
            wy: [0.0; 6],
            page_a: 0,
            page_da: 0,
        }
    }
}

impl Cursor {
    /// `InitCursol(f)` (0x001b78b0).
    pub fn init(&mut self, f: bool) {
        self.sel = if f { -1 } else { -2 };
        self.idx = 0;
        self.a = [0; 12];
        self.page_a = 0;
        self.page_da = 0;
    }

    /// `DispSelectCursol(x, y, w, sn, a, df)` (0x001b7910) on `s`: the
    /// fading bars (df & 2), the wing easing after them (df & 1), and the
    /// pointer (df & 4).
    #[allow(clippy::too_many_arguments)]
    pub fn disp(&mut self, s: &mut Spr, x: f32, y: f32, w: i32, sn: i32, alpha: i32, df: i32, frame_rate: u32) {
        let rate = frame_rate as i32;
        if self.sel == -2 {
            for k in 0..6 {
                self.wx[k] = sub(x, 16.0);
                self.wy[k] = sub(y, 18.0);
            }
        }
        if sn != self.sel {
            self.sel = sn;
            let mut m: i16 = 255;
            for j in 0..12 {
                if self.a[j] < m {
                    self.idx = j;
                    m = self.a[j];
                }
            }
            self.a[self.idx] = 128;
            self.w[self.idx] = from_int((w - 1) * 14);
            self.x[self.idx] = add(6.0, x);
            self.y[self.idx] = sub(y, 2.0);
        }
        (s.wu, s.wv, s.wi, s.su, s.sv) = (1536, 1536, 3, 16, 20);
        for k in 0..12 {
            if self.a[k] == 0 {
                continue;
            }
            if k == self.idx {
                if self.a[k] < 48 {
                    self.a[k] = 96;
                } else {
                    let mut st = i32::from(32 - self.a[k]) / 16;
                    st /= 3 - rate;
                    if st >= 0 {
                        st = -1;
                    }
                    self.a[k] += st as i16;
                }
            } else {
                let mut st = i32::from(self.a[k]) / 8;
                st /= 3 - rate;
                if st < 4 {
                    st = 4;
                }
                self.a[k] -= st as i16;
                if self.a[k] < 8 {
                    self.a[k] = 0;
                    continue;
                }
            }
            if i32::from(self.a[k]) > alpha {
                self.a[k] = alpha as i16;
            }
            if df & 2 != 0 {
                s.set_alpha(i32::from(self.a[k]));
                s.sx = 16.0;
                s.sy = 20.0;
                s.dx = self.x[k];
                s.dy = self.y[k];
                s.make_packet(0);
                s.sx = self.w[k];
                s.sy = 20.0;
                s.dx = add(16.0, self.x[k]);
                s.dy = self.y[k];
                s.make_packet(1);
                s.sx = 16.0;
                s.sy = 20.0;
                s.dx = add(add(16.0, self.x[k]), self.w[k]);
                s.dy = self.y[k];
                s.make_packet(2);
            }
        }
        if df & 1 != 0 {
            s.set_alpha(alpha);
            set_type(s, 2);
            s.sx = 20.0;
            s.sy = 20.0;
            let cur = from_int(i32::from(self.a[self.idx]));
            let off = if f64::from(cur) > 96.0 { div(cur, 10.0) } else { sub(div(cur, 12.0), 4.0) };
            let tx = add(x, sub(off, 16.0));
            let ty = add(y, sub(off, 18.0));
            let fr = from_int(rate);
            for k in (0..6).rev() {
                if k > 0 {
                    let d = mul(sub(10.0, from_int(k as i32)), sub(3.0, fr));
                    self.wx[k] = add(self.wx[k - 1], div(sub(tx, self.wx[k - 1]), d));
                    self.wy[k] = add(self.wy[k - 1], div(sub(ty, self.wy[k - 1]), d));
                    let g = (128 - 8 * k as i32) as u32;
                    s.set_rgb_u32(g | (g << 8) | (g << 16));
                    s.set_alpha(alpha.min(48 - 8 * k as i32));
                } else {
                    self.wx[0] = add(self.wx[0], div(sub(tx, self.wx[0]), div(4.5, fr)));
                    self.wy[0] = add(self.wy[0], div(sub(ty, self.wy[0]), div(4.5, fr)));
                    s.set_colour(7);
                    s.set_alpha(alpha);
                }
                s.dx = self.wx[k];
                s.dy = self.wy[k];
                s.make_packet(10);
            }
        }
        if df & 4 != 0 {
            s.set_alpha(alpha);
            s.dx = sub(x, 10.0);
            s.dy = add(8.0, y);
            set_type(s, 1);
            let f = add(div(from_int(i32::from(self.a[self.idx])), 96.0), 1.0);
            s.cx = -7.0;
            s.cy = -8.0;
            s.sx = mul(s.sx, f);
            s.sy = mul(s.sy, f);
            s.cx = mul(s.cx, f);
            s.cy = mul(s.cy, f);
            s.make_packet(26);
            s.cx = 0.0;
            s.cy = 0.0;
        }
    }
}

/// The divider rows `DispSquareW2` draws over its frame (`@1408`, `@1411`,
/// `@1412`, main 0x00377ed0 - 0x00377ee0): past `w + 1` cells a T piece
/// (codes 10, 22 stretched down, 18), then `h + 1` cells skipped.
pub const DIVIDER_TOP: [i8; 6] = [0, 0, 10, 0, 0, -1];
pub const DIVIDER_MIDDLE: [i8; 6] = [0, 0, 22, 0, 0, -1];
pub const DIVIDER_BOTTOM: [i8; 6] = [0, 0, 18, 0, 0, -1];

/// `DispSquareW2(w, h, n, title)` (0x001b9120): a frame `w + h + 1` cells
/// wide inside and `n` rows high, divided after `w` cells; the title over
/// it.
pub fn disp_square_w2(s: &mut Spr, w: i32, h: i32, n: i32, title: Option<&[u8]>) {
    let (x0, y0) = (s.dx, s.dy);
    set_type(s, 1);
    let total = w + h + 1;
    disp_line(s, 1, 0, &row(TOP, total));
    if n > 0 {
        disp_line(s, 3, n, &row(MIDDLE, total));
    }
    disp_line(s, 2, 0, &row(BOTTOM, total));
    if let Some(t) = title {
        s.dx = x0;
        s.dy = y0;
        disp_title(s, t);
    }
    s.dx = x0;
    s.dy = y0;
    let divider = |mut p: [i8; 6]| {
        p[1] = (w + 1) as i8;
        p[4] = (h + 1) as i8;
        p
    };
    disp_line(s, 1, 0, &divider(DIVIDER_TOP));
    if n > 0 {
        disp_line(s, 3, n, &divider(DIVIDER_MIDDLE));
    }
    disp_line(s, 2, 0, &divider(DIVIDER_BOTTOM));
}
