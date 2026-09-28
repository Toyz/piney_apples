//! The party panels along the bottom of the screen and the condition
//! icons (`FacePanelDisp` gcmn 0x00522eb0, `ConditionIconDisp` 0x00522910).
//!
//! A panel is 170 wide: the face (96 x 96, `xwin_f00::TEX_xwin_fNN`) at
//! its left, the name in its frame (`DispTarget(n, 1)`), and HP over SP,
//! each a label cell, a 56-unit bar (green, blue; the empty rest red) and
//! the numbers in `fontTex`'s type 2 digits coloured by how full the gauge
//! is.

use piney_desktop::eef::{add, from_int, sub};

use crate::ctrl::{Ctx, MenuCtrl};
use crate::disp::now_max_colour;
use crate::spr::{font_type, make_num};
use crate::window::disp_target;
use crate::world::CharInfo;

/// `FacePanelDisp(x, y, a, cnum, id, hp, mhp, sp, msp, fcol, fcol2)`:
/// party slot `cnum`'s panel at (x, y); `fcol` tints the face, `fcol2`
/// (the flash) is added to the frame, the bars and the face.
#[allow(clippy::too_many_arguments)]
pub fn face_panel(
    m: &mut MenuCtrl,
    x: &mut Ctx,
    px: f32,
    py: f32,
    a: i32,
    cnum: usize,
    c: &CharInfo,
    fcol: u32,
    fcol2: u32,
) {
    let (hp, mhp, sp, msp) = (i32::from(c.hp), i32::from(c.max_hp).max(1), i32::from(c.sp), i32::from(c.max_sp).max(1));
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win_pr.set_rgb_u32(fcol2.wrapping_add(0x8080_8080));
    m.win_pr.set_alpha(a);
    let names = x.save.names();
    let n = piney_desktop::message::str_len(&c.name, &names);
    m.name_pr.set_colour(7);
    m.name_pr.set_alpha(a);
    m.name_pr.dx = add(50.0, px);
    m.name_pr.dy = add(6.0, py);
    m.name_pr.make_packet(cnum as i32 + 2);
    let w = &mut m.win_pr;
    w.dx = px;
    w.dy = py;
    disp_target(w, n, 1);
    // The gauges' frames: mirrored ends, then the middles and the plain ends.
    w.set_grid(8, 16, 8.0, 25.0, 1088, 2048, 2);
    w.flip = true;
    let f20 = add(74.0, px);
    let f23 = add(24.0, py);
    let f21 = add(6.0, f23);
    w.dx = f20;
    w.dy = f21;
    w.make_packet(1);
    let f27 = add(10.0, f20);
    let f24 = add(52.0, py);
    let f22 = add(6.0, f24);
    w.dx = f27;
    w.dy = f22;
    w.make_packet(1);
    w.flip = false;
    w.sx = 80.0;
    w.sy = 25.0;
    let f28 = add(8.0, f20);
    w.dx = f28;
    w.dy = f21;
    w.make_packet(0);
    w.sx = 60.0;
    w.sy = 25.0;
    let f27b = add(8.0, f27);
    w.dx = f27b;
    w.dy = f22;
    w.make_packet(0);
    w.sx = 8.0;
    w.sy = 25.0;
    w.dx = add(80.0, f28);
    w.dy = f21;
    w.make_packet(1);
    w.dx = add(60.0, f27b);
    w.dy = f22;
    w.make_packet(1);
    // HP and SP labels.
    w.set_grid(84, 16, 84.0, 16.0, 0, 2048, 1);
    let lx = add(6.0, f20);
    w.dx = lx;
    w.dy = f23;
    w.make_packet(0);
    w.dx = lx;
    w.dy = f24;
    w.make_packet(1);
    // The bars.
    let bx = sub(sub(add(84.0, lx), 56.0), 2.0);
    for (now, max, y, col) in [(hp, mhp, add(4.0, f23), 0x8050_8030u32), (sp, msp, add(4.0, f24), 0x8080_5030u32)] {
        w.set_grid(8, 8, 0.0, 8.0, 1344, 2048, 1);
        let fill = (now * 56) / max;
        w.sx = from_int(fill);
        w.sy = 8.0;
        w.dx = bx;
        w.dy = y;
        w.set_rgb_u32(fcol2.wrapping_add(col));
        w.set_alpha(a);
        w.make_packet(0);
        if fill < 56 {
            w.sx = from_int(56 - fill);
            w.sy = 8.0;
            w.set_colour(18);
            w.set_alpha(a);
            w.make_packet(0);
        }
    }
    w.set_colour(7);
    // The numbers.
    let f = &mut m.font;
    font_type(f, 2);
    let nx = add(16.0, lx);
    for (now, max, y) in [(hp, mhp, add(38.0, py)), (sp, msp, add(66.0, py))] {
        f.set_colour(now_max_colour(now, max));
        f.set_alpha(a);
        let n = if max >= 1000 {
            4
        } else if max >= 100 {
            3
        } else {
            2
        };
        f.dx = sub(nx, piney_desktop::eef::mul(10.0, from_int(n - 2)));
        f.dy = y;
        make_num(f, n, now);
        f.make_str(&x.texts.slash);
        make_num(f, n, max);
    }
    // The face.
    let face = &mut m.faces[cnum];
    face.set_rgb_u32(fcol.wrapping_add(fcol2));
    face.set_alpha(a);
    face.dx = px;
    face.dy = py;
    face.su = 96;
    face.sv = 96;
    face.sx = 96.0;
    face.sy = 96.0;
    face.make_packet(0);
}

/// `ConditionIconDisp(c, x, y, a)`: up to 21 icons, five a row upward from
/// (x, y): poison, curse, paralysis, sleep, confusion, charm, speed, the
/// six battle abilities' ups and downs, the six elements', regeneration.
pub fn condition_icons(m: &mut MenuCtrl, c: &CharInfo, x: f32, y: f32, a: i32) {
    let s = &mut m.con_icon;
    s.set_grid(32, 32, 32.0, 32.0, 0, 0, 4);
    s.set_colour(7);
    s.set_alpha(a);
    let cond = &c.condition;
    let mut n = 0;
    for k in 0..21 {
        let icon: i32 = match k {
            0 => sel(cond[13] != 0, 0),
            1 => sel(cond[8] != 0, 6),
            2 => sel(cond[14] != 0, 1),
            3 => sel(cond[9] != 0, 5),
            4 => sel(cond[10] != 0, 4),
            5 => sel(cond[11] != 0, 3),
            6 => {
                if cond[15] == 0 {
                    -1
                } else if c.speed_value < 1.0 {
                    2
                } else if c.speed_value > 1.0 {
                    22
                } else {
                    -1
                }
            }
            7..=12 => {
                let v = c.ability[[0, 1, 2, 4, 5, 6][k - 7]];
                updown(v, (k - 7) as i32 + 7)
            }
            13..=18 => updown(c.elements[k - 13], (k - 13) as i32 + 13),
            19 => sel(cond[12] != 0, 20),
            _ => sel(cond[7] != 0, 21),
        };
        if icon == -1 {
            continue;
        }
        s.dx = add(x, from_int((n % 5) << 5));
        s.dy = add(y, from_int(-(n / 5) << 5));
        s.make_packet(icon);
        n += 1;
    }
}

fn sel(on: bool, icon: i32) -> i32 {
    if on { icon } else { -1 }
}

/// A buff's icon: `down` below 0, `down + 16` above.
fn updown(v: i16, down: i32) -> i32 {
    if v < 0 {
        down
    } else if v > 0 {
        down + 16
    } else {
        -1
    }
}
