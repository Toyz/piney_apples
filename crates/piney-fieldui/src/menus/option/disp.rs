//! The OPTION pages' own drawing, from `ExceptionDisp` (0x00522460) early in
//! `Disp` (menuFont is `font` then): `ControllerMenuDisp` (0x0053d550),
//! `VibrationMenuDisp` (0x0053ed90) and its three twins,
//! `ScreenMenuDisp` (0x0053f250), `SoundMenuDisp` (0x0053fbc0), with
//! `ccMenuWindow::DispSlideBar` (main 0x001b97f0). The positions, cells and
//! colours are in docs/engine/field-ui.md (the OPTION pages).

use piney_desktop::eef::{add, from_int, sub};

use crate::ctrl::{Ctx, MenuCtrl};
use crate::menus::option::{CAM_TYPE, OUTPUT, extract_setting, on_off_marks};
use crate::spr::{Spr, font_type, make_signed_num, set_clm};
use crate::window::{disp_square, set_type};

fn list_index(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// `ccMenuWindow::DispSlideBar(w, n, max)` (main 0x001b97f0): at (dx, dy)
/// the trough's three cells (11, 12 `w` wide, 13), the knob (29) at `n` of
/// `max` along it, and the filled part in colour 22 from the fill cell;
/// colour 7 and grid 1 after.
pub fn disp_slide_bar(s: &mut Spr, w: i32, n: i32, max: i32) {
    let (x0, y0) = (s.dx, s.dy);
    set_type(s, 1);
    s.sx = from_int(s.su);
    s.sy = from_int(s.sv);
    s.make_packet(11);
    s.sx = from_int(s.su.wrapping_mul(w));
    s.sy = from_int(s.sv);
    s.make_packet(12);
    s.sx = from_int(s.su);
    s.sy = from_int(s.sv);
    s.make_packet(13);
    let fill = if max == 0 { 0 } else { n.wrapping_mul(s.su.wrapping_mul(w).wrapping_add(18)) / max };
    let x1 = add(6.0, x0);
    s.dx = add(sub(x1, 5.0), from_int(fill));
    s.dy = add(6.0, y0);
    s.make_packet(29);
    s.set_colour(22);
    s.wu = 1344;
    s.wv = 2048;
    s.wi = 1;
    s.sx = from_int(fill);
    s.sy = 8.0;
    s.su = 8;
    s.sv = 8;
    s.dx = x1;
    s.dy = add(4.0, y0);
    s.make_packet(0);
    s.set_colour(7);
    set_type(s, 1);
}

fn at(s: &mut Spr, x: f32, y: f32, code: i32) {
    s.dx = x;
    s.dy = y;
    s.make_packet(code);
}

// --- Controller -----------------------------------------------------------

/// `ControllerMenuDisp` (0x0053d550).
pub fn controller_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let l = m.lists[list_index(m)].clone();
    let alpha = m.alpha;
    let fr = x.frame_rate;
    let o = &x.texts.option;
    let cam = x.save.save.u8(CAM_TYPE) as i8;
    // The list.
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    m.win.dx = 39.0;
    m.win.dy = 56.0;
    let title = if l.title.is_empty() { None } else { Some(&l.title[..]) };
    disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), title);
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    let f20 = add(56.0, 16.0);
    for r in 0..i32::from(l.y) {
        let y = add(f20, from_int(20 * r));
        if i32::from(l.select) == r {
            m.cursor.disp(&mut m.win, 39.0, y, i32::from(l.x), i32::from(l.select), alpha, 3, fr);
        }
        m.kanji.set_colour(if i32::from(l.index) == r { 6 } else { 7 });
        m.kanji.set_alpha(m.kanji_alpha);
        m.kanji.dx = 53.0;
        m.kanji.dy = y;
        m.kanji.make_packet(r);
    }
    // The picture.
    let k = &mut m.mask;
    k.set_colour(7);
    k.set_alpha(alpha);
    (k.wu, k.wv, k.wi, k.su, k.sv) = (0, 0, 1, 256, 128);
    (k.sx, k.sy) = (256.0, 128.0);
    at(k, 88.0, 168.0, 0);
    // The buttons.
    let (mut f21, mut f20) = (320.0f32, 128.0f32);
    m.win.set_colour(0);
    m.win.set_alpha(alpha);
    set_type(&mut m.win, 1);
    m.win.dx = f21;
    m.win.dy = f20;
    disp_square(&mut m.win, 9, 4, None);
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    extract_setting(m, 0, &o.ctrl_btn);
    let s0 = &mut m.setting[0];
    set_clm(s0, 16, 0, 0, 1);
    s0.set_colour(7);
    s0.set_alpha(alpha);
    set_type(&mut m.win, 2);
    f21 = add(f21, 6.0);
    f20 = add(f20, 16.0);
    let f22 = add(16.0, f21);
    for (k, (button, label)) in [(17, 0), (18, 1), (16, 3), (19, 2)].into_iter().enumerate() {
        if k > 0 {
            f20 = add(f20, 20.0);
        }
        at(&mut m.win, f21, f20, button);
        at(&mut m.setting[0], f22, f20, label);
    }
    // START and SELECT: their cells, labels and the two buttons' shapes.
    (f21, f20) = (179.0, 300.0);
    m.win.set_colour(0);
    m.win.set_alpha(alpha);
    set_type(&mut m.win, 1);
    m.win.dx = f21;
    m.win.dy = f20;
    disp_square(&mut m.win, 10, 2, None);
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    set_type(&mut m.win, 2);
    f21 = add(f21, 8.0);
    f20 = add(f20, 13.0);
    at(&mut m.win, f21, f20, 8);
    let f22 = add(22.0, f21);
    at(&mut m.setting[0], f22, sub(f20, 6.0), 4);
    let w = &mut m.win;
    (w.wu, w.wv, w.wi, w.su, w.sv, w.sx, w.sy) = (1792, 3776, 1, 39, 12, 39.0, 12.0);
    let f23 = add(6.0, f21);
    at(w, f23, add(10.0, f20), 0);
    set_type(&mut m.win, 2);
    f20 = add(f20, 27.0);
    at(&mut m.win, f21, f20, 9);
    at(&mut m.setting[0], f22, sub(f20, 6.0), 5);
    let w = &mut m.win;
    (w.wu, w.wv, w.wi, w.su, w.sv, w.sx, w.sy) = (1024, 3776, 1, 48, 12, 48.0, 12.0);
    at(w, f23, add(10.0, f20), 0);
    // The left stick.
    (f21, f20) = (39.0, 300.0);
    m.win.set_colour(0);
    m.win.set_alpha(alpha);
    set_type(&mut m.win, 1);
    m.win.dx = f21;
    m.win.dy = f20;
    disp_square(&mut m.win, 8, 2, None);
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    extract_setting(m, 1, &o.ctrl_mov);
    let s1 = &mut m.setting[1];
    set_clm(s1, 16, 0, 0, 1);
    s1.set_colour(7);
    s1.set_alpha(alpha);
    set_type(&mut m.win, 3);
    f21 = add(f21, 8.0);
    f20 = add(f20, 16.0);
    at(&mut m.win, f21, f20, 0);
    f21 = add(f21, 24.0);
    at(&mut m.setting[1], f21, f20, 0);
    f20 = add(f20, 20.0);
    at(&mut m.setting[1], f21, f20, 1);
    // The right stick.
    extract_setting(m, 2, &o.ctrl_cam);
    let s2 = &mut m.setting[2];
    set_clm(s2, 16, 0, 0, 1);
    s2.set_colour(7);
    s2.set_alpha(alpha);
    (f21, f20) = (347.0, 300.0);
    set_type(&mut m.win, 1);
    m.win.dx = f21;
    m.win.dy = f20;
    disp_square(&mut m.win, 7, 2, None);
    set_type(&mut m.win, 3);
    f21 = add(f21, 12.0);
    f20 = add(f20, 16.0);
    at(&mut m.win, f21, f20, 1);
    f21 = add(f21, 24.0);
    match cam {
        0 | 1 => {
            at(&mut m.setting[2], f21, f20, 0);
            f20 = add(f20, 20.0);
            at(&mut m.setting[2], f21, f20, 1);
        }
        2 | 3 => {
            f20 = add(f20, 10.0);
            at(&mut m.setting[2], f21, f20, 2);
        }
        _ => {}
    }
    // The shoulder buttons.
    (f21, f20) = (165.0, 56.0);
    set_type(&mut m.win, 1);
    m.win.dx = f21;
    m.win.dy = f20;
    disp_square(&mut m.win, 20, 2, None);
    set_type(&mut m.win, 2);
    f21 = add(f21, 14.0);
    f20 = add(f20, 16.0);
    m.win.dx = f21;
    m.win.dy = f20;
    m.win.make_packet(32);
    m.win.make_packet(33);
    let f22 = add(32.0, f21);
    m.setting[2].dx = f22;
    m.setting[2].dy = f20;
    match cam {
        0 | 1 => m.setting[2].make_packet(2),
        2 | 3 => m.setting[2].make_packet(4),
        _ => {}
    }
    f20 = add(f20, 20.0);
    m.win.dx = f21;
    m.win.dy = f20;
    m.win.make_packet(34);
    m.win.make_packet(35);
    m.setting[2].dx = f22;
    m.setting[2].dy = f20;
    if (0..=3).contains(&cam) {
        m.setting[2].make_packet(3);
    }
    f20 = add(56.0, 16.0);
    m.win.dx = 323.0;
    m.win.dy = f20;
    m.win.make_packet(36);
    m.win.make_packet(37);
    match cam {
        // Mutation moved this label from 369 to 375, as on the desktop's
        // page.
        0 | 1 => at(&mut m.setting[2], if x.texts.words.volume > 1 { 375.0 } else { 369.0 }, f20, 2),
        2 | 3 => at(&mut m.setting[2], 355.0, f20, 0),
        _ => {}
    }
    f20 = add(f20, 20.0);
    m.win.dx = 323.0;
    m.win.dy = f20;
    m.win.make_packet(38);
    m.win.make_packet(39);
    m.setting[2].dx = 355.0;
    m.setting[2].dy = f20;
    match cam {
        0 | 1 => m.setting[2].make_packet(4),
        2 | 3 => m.setting[2].make_packet(1),
        _ => {}
    }
    // The scheme's arrows: A-2 and B-2 turned upside down (0x40), the left
    // ones mirrored (0x20).
    set_type(&mut m.win, 1);
    let w = &mut m.win;
    let (lx, rx, y0, y1, upside) = match cam {
        0 => (165.0, 355.0, 60.0, 76.0, false),
        1 => (165.0, 355.0, 56.0, 72.0, true),
        2 => (345.0, 375.0, 290.0, 306.0, false),
        3 => (345.0, 375.0, 286.0, 302.0, true),
        _ => return,
    };
    let (top, bottom) = if upside { (32, 14) } else { (14, 32) };
    w.flip_v = upside;
    w.flip = true;
    at(w, lx, y0, top);
    at(w, lx, y1, bottom);
    w.flip = false;
    at(w, rx, y0, top);
    at(w, rx, y1, bottom);
    w.flip_v = false;
}

// --- The on / off pages ---------------------------------------------------

/// `VibrationMenuDisp` (0x0053ed90), `DataDrainDemoMenuDisp` (0x00540a50),
/// `VoiceMenuDisp` (0x00541010), `StrwinMenuDisp` (0x005415d0): the list's
/// window and its two rows, the save's in colour 6.
pub fn on_off_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let l = m.lists[list_index(m)].clone();
    let (on, off) = on_off_marks(m.menu, x.save);
    m.win.dx = 200.0;
    m.win.dy = 248.0;
    disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), None);
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    let alpha = m.alpha;
    for r in 0..i32::from(l.y) {
        let y = from_int(20 * r + 264);
        if i32::from(l.select) == r {
            m.cursor.disp(&mut m.win, 200.0, y, i32::from(l.x), i32::from(l.select), alpha, 3, x.frame_rate);
        }
        let marked = if r == 0 { on } else { off };
        m.kanji.set_colour(if marked { 6 } else { 7 });
        m.kanji.set_alpha(m.kanji_alpha);
        m.kanji.dx = 214.0;
        m.kanji.dy = y;
        m.kanji.make_packet(r);
    }
}

// --- Adjust Screen --------------------------------------------------------

/// Adjust Screen's marks (sx, sy, dx, dy): the four corners' two bars each,
/// and the centre's four.
const SCREEN_MARKS: [(f32, f32, f32, f32); 12] = [
    (10.0, 80.0, 0.0, 0.0),
    (70.0, 10.0, 10.0, 0.0),
    (10.0, 80.0, 0.0, 368.0),
    (70.0, 10.0, 10.0, 438.0),
    (10.0, 80.0, 502.0, 0.0),
    (70.0, 10.0, 432.0, 0.0),
    (10.0, 80.0, 502.0, 368.0),
    (70.0, 10.0, 432.0, 438.0),
    (10.0, 48.0, 251.0, 112.0),
    (10.0, 48.0, 251.0, 288.0),
    (48.0, 10.0, 128.0, 219.0),
    (48.0, 10.0, 336.0, 219.0),
];

/// `ScreenMenuDisp` (0x0053f250).
pub fn screen_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let l = m.lists[list_index(m)].clone();
    let alpha = m.alpha;
    set_type(&mut m.win, 1);
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    m.win.dx = 200.0;
    m.win.dy = 188.0;
    disp_square(&mut m.win, 6, 2, None);
    // `font` is menuFont inside Disp.
    let f = &mut m.font;
    f.set_colour(7);
    font_type(f, 1);
    f.shadow = true;
    f.set_alpha(alpha);
    let [lx, ly] = &x.texts.option.screen_labels;
    f.dx = 216.0;
    f.dy = 204.0;
    f.make_str(lx);
    make_signed_num(f, 3, i64::from(i32::from(l.sx) / 3 - 16));
    f.dx = 216.0;
    f.dy = 224.0;
    f.make_str(ly);
    make_signed_num(f, 3, i64::from(i32::from(l.sy) - 16));
    // The marks.
    let a = &mut m.win_a;
    a.set_colour(10);
    let mut v = i32::from(m.wait_count);
    if v >= 16 {
        v = 30 - v;
    }
    a.set_alpha(((v << 7) / 15 + 16).min(alpha));
    (a.wu, a.wv, a.wi, a.su, a.sv) = (1344, 2064, 1, 1, 1);
    (a.cx, a.cy) = (0.0, 0.0);
    for (sx, sy, dx, dy) in SCREEN_MARKS {
        a.sx = sx;
        a.sy = sy;
        at(a, dx, dy, 0);
    }
}

// --- Sound ----------------------------------------------------------------

/// `SoundMenuDisp` (0x0053fbc0).
pub fn sound_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let l = m.lists[list_index(m)].clone();
    let alpha = m.alpha;
    let fr = x.frame_rate;
    let t = m.option.sound;
    set_type(&mut m.win, 1);
    m.win.set_colour(7);
    m.win.set_alpha(alpha);
    let (f20, mut f21) = (152.0f32, 96.0f32);
    m.win.dx = f20;
    m.win.dy = f21;
    let title = if l.title.is_empty() { None } else { Some(&l.title[..]) };
    disp_square(&mut m.win, 12, 10, title);
    let f20 = add(f20, 14.0);
    let rows = x.texts.option.sound.clone();
    crate::menus::system::extract_menu(m, rows);
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    m.kanji.set_colour(7);
    m.kanji.set_alpha(alpha);
    f21 = add(f21, 20.0);
    let f22 = add(12.0, f20);
    let sel = i32::from(l.select);
    for (r, value) in t.iter().take(3).enumerate() {
        let r = r as i32;
        if r > 0 {
            f21 = add(f21, 50.0);
        }
        m.win.dx = f22;
        m.win.dy = add(16.0, f21);
        disp_slide_bar(&mut m.win, 8, *value, 32);
        m.kanji.dx = f20;
        m.kanji.dy = f21;
        if sel == r {
            m.cursor.disp(&mut m.win, sub(f20, 14.0), f21, 12, sel, alpha, 3, fr);
        }
        m.kanji.make_packet(r);
    }
    // Output: Mono and Stereo.
    f21 = add(f21, 48.0);
    m.kanji.dx = f20;
    m.kanji.dy = f21;
    m.kanji.set_colour(7);
    m.kanji.set_alpha(alpha);
    m.kanji.make_packet(3);
    let output = x.save.save.i16(OUTPUT);
    let y = add(16.0, f21);
    for (k, code) in [(0i16, 4), (1, 5)] {
        let dx = if k == 0 { f22 } else { add(72.0, f22) };
        if output == k {
            m.kanji.set_colour(6);
            if sel == 3 {
                m.cursor.disp(&mut m.win, sub(dx, 14.0), y, 4, sel + i32::from(output), alpha, 3, fr);
            }
        } else {
            m.kanji.set_colour(7);
        }
        m.kanji.set_alpha(alpha);
        m.kanji.dx = dx;
        m.kanji.dy = y;
        m.kanji.make_packet(code);
    }
}
