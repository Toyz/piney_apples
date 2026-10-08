//! A member's status: `StatusMenu` (gcmn 0x00535620, menu 8) with
//! `StatusMenuDisp` (0x005362d0); the member's items, `ItemStatusMenu`
//! (0x00545e70, menu 31) with `ItemStatusMenuDisp` (0x005462a0); an
//! equipment piece's, `EquipStatusMenu` (0x00539430, menu 64) with
//! `EquipStatusMenuDisp` (0x00539ca0); and the boxes those pages and
//! Equipment (63) share: `ParameterDisp` (0x00523e50), `SkillDisp`
//! (0x00524e20) and `BeffDisp` (0x005251f0; 0x00525150 on `ccCondition`s).
//! The grid and the steps are in docs/engine/field-ui.md (PERSONAL's pages).

use piney_desktop::eef::{add, from_int, mul, sub};

use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_MOVE, SE_OK};
use crate::items::{self, ITEMS, Item};
use crate::menus::personal::key;
use crate::menus::system::str_cat;
use crate::spr::{font_type, make_signed_num, set_clm};
use crate::window::{disp_button, disp_line_h, disp_line_v, disp_square, disp_target, set_type};
use crate::world::CharInfo;

/// `saveData.spcParam[id]` (+0x7488, 0xdc bytes).
pub const SPC_PARAM: usize = 0x7488;
pub const SPC_SIZE: usize = 0xdc;
/// `ccSpcParam` members.
pub const SPC_REAL: usize = 0x48;
pub const SPC_EQUIPMENT: usize = 0xc8;
pub const SPC_JOB: usize = 0xd8;

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// The `page`-th filled party slot and its character; none (the loop's
/// last slot) as the game leaves it.
pub fn page_member(x: &Ctx, page: i32) -> Option<(usize, CharInfo)> {
    let mut n = 0;
    for (slot, c) in x.world.party.iter().enumerate() {
        if let Some(c) = c {
            if n == page {
                return Some((slot, c.clone()));
            }
            n += 1;
        }
    }
    None
}

/// The member a Status page is on (`list.page` reset to 0, the player,
/// when it names none).
fn status_member(m: &mut MenuCtrl, x: &Ctx) -> (usize, CharInfo) {
    let i = idx(m);
    match page_member(x, i32::from(m.lists[i].page)) {
        Some(v) => v,
        None => {
            m.lists[i].page = 0;
            (0, x.world.player().cloned().unwrap_or_default())
        }
    }
}

/// `ccSpcParam` shorts at `off` of member `id`.
pub fn spc_i16(x: &Ctx, id: i32, off: usize) -> i16 {
    x.save.save.i16(SPC_PARAM + SPC_SIZE * id.clamp(0, 17) as usize + off)
}

/// The member's `real` block (`ccCharParamElement`, 16 shorts).
pub fn spc_real(x: &Ctx, id: i32) -> [i16; 16] {
    std::array::from_fn(|k| spc_i16(x, id, SPC_REAL + 2 * k))
}

/// `saveData.skillList[id]` (20 shorts).
pub fn skill_list(x: &Ctx, id: i32) -> [i16; 20] {
    std::array::from_fn(|k| items::save_skill(x.save, id.clamp(0, 17) as usize, k))
}

/// The cursor moves of Status (0x00535800 - 0x00535cd8): whether it moved.
fn status_cursor(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let i = idx(m);
    let r = x.pad.repeat.bits();
    let (right, left, up, down) = (r & 0x2000 != 0, r & 0x8000 != 0, r & 0x1000 != 0, r & 0x4000 != 0);
    let l = &mut m.lists[i];
    let mut moved = 0;
    let mut se = false;
    if l.sx == 0 && l.sy < 5 {
        if right {
            se = true;
            l.sx += 1;
            l.sy = if l.sy < 3 { 8 } else { l.sy + 6 };
            moved += 1;
        } else if up {
            l.sy -= 1;
            if l.sy < 0 {
                l.sy = 0;
            } else {
                se = true;
                moved += 1;
            }
        } else if down {
            se = true;
            l.sy += 1;
            if l.sy >= 5 {
                l.sy = 11;
            }
            moved += 1;
        }
    } else if l.sx > 0 && l.sy < 8 {
        if right {
            l.sx += 1;
            if l.sx >= 3 {
                l.sx = 2;
            } else {
                se = true;
                moved += 1;
            }
        } else if left {
            l.sx -= 1;
            se = true;
            if l.sx <= 0 {
                l.sx = 0;
                l.sy = 0;
            }
            moved += 1;
        } else if up {
            l.sy -= 1;
            if l.sy < 0 {
                l.sy = 0;
            } else {
                se = true;
                moved += 1;
            }
        } else if down {
            se = true;
            l.sy += 1;
            moved += 1;
        }
    } else if l.sx > 0 && l.sy < 11 {
        if right {
            l.sx += 1;
            if l.sx >= 3 {
                l.sx = 2;
            } else {
                se = true;
                moved += 1;
            }
        } else if left {
            se = true;
            l.sx -= 1;
            if l.sx <= 0 {
                l.sx = 0;
                l.sy -= 6;
            }
            moved += 1;
        } else if up {
            se = true;
            l.sy -= 1;
            moved += 1;
        } else if down {
            se = true;
            l.sy += 1;
            if l.sy >= 11 {
                l.sx += 1;
                l.sy = 11;
            }
            moved += 1;
        }
    } else if l.sy >= 11 {
        if right {
            l.sx += 1;
            if l.sx >= 4 {
                l.sx = 3;
            } else {
                se = true;
                moved += 1;
            }
        } else if left {
            l.sx -= 1;
            if l.sx < 0 {
                l.sx = 0;
            } else {
                se = true;
                moved += 1;
            }
        } else if up {
            se = true;
            l.sy -= 1;
            if l.sy < 11 {
                if l.sx < 2 {
                    l.sx = 0;
                    l.sy = 4;
                } else {
                    l.sx -= 1;
                    l.sy = 10;
                }
            }
            moved += 1;
        } else if down {
            l.sy += 1;
            if l.sy >= 15 {
                l.sy = 14;
            } else {
                se = true;
                moved += 1;
            }
        }
    }
    if se {
        x.se(SE_MOVE);
    }
    moved
}

/// The help of a Status row: `statusMenuHelp[sx * 15 + sy]`, a skill's own
/// help, an added effect's (`statusBeffStr`) among those the character has.
fn status_help(m: &mut MenuCtrl, x: &Ctx, c: &CharInfo) {
    let l = m.lists[idx(m)].clone();
    let (sx, sy) = (i32::from(l.sx), i32::from(l.sy));
    let t = &x.texts.pers;
    let mut help = t.status_help.get((sx * 15 + sy).max(0) as usize).cloned().unwrap_or_default();
    if sx > 0 && sy < 11 {
        if sy < 8 {
            let want = (sx - 1) * 8 + sy;
            let mut n = 0;
            let mut found = -1;
            for s in skill_list(x, i32::from(c.id)) {
                if s < 0 {
                    continue;
                }
                if n == want {
                    found = i32::from(s);
                    break;
                }
                n += 1;
            }
            if found >= 0
                && let Some(p) = x.texts.items.skill(found)
            {
                help = p.help.clone();
            }
        } else {
            let want = (sy - 8) + (sx - 1) * 3;
            let mut n = 0;
            for k in 0..5 {
                if c.condition[2 + k] != 0 {
                    if n == want {
                        help = t.beff_help[k].clone();
                    }
                    n += 1;
                }
            }
        }
    }
    let names = x.save.names();
    let third = if sx > 0 && sy < 11 { Some(&help[2][..]) } else { None };
    m.msg.change(0x100, None, [Some(&help[0]), Some(&help[1]), third], &names);
    m.msg.set_pos(39.0, 334.0);
}

/// `StatusMenu` (menu 8).
pub fn status_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let (_, mut c) = status_member(m, x);
    let mut moved = 0;
    match m.proccess {
        0 => {
            m.exception_disp = 1;
            moved = 1;
            m.proccess += 1;
        }
        1 => {}
        _ => return Flow::Done,
    }
    let num = x.world.party_num();
    if num >= 2 {
        let r = x.pad.repeat.bits();
        if r & 0x4 != 0 {
            m.lists[i].page -= 1;
            moved += 1;
            x.se(SE_MOVE);
        } else if r & 0x8 != 0 {
            m.lists[i].page += 1;
            moved += 1;
            x.se(SE_MOVE);
        }
        let l = &mut m.lists[i];
        if l.page < 0 {
            l.page = num - 1;
        } else if l.page >= num {
            l.page = 0;
        }
        if let Some((_, cc)) = page_member(x, i32::from(m.lists[i].page)) {
            c = cc;
        }
    }
    moved += status_cursor(m, x);
    if x.pushed_cancel() {
        x.se(crate::ctrl::SE_BACK);
        m.msg.close();
        let prev = m.lists[i].prev;
        m.back_to_prev(prev);
    }
    let push = x.pad.push.bits();
    let l = m.lists[i].clone();
    if push & 0x10 != 0 {
        if l.sx == 0 && l.sy < 5 {
            x.se(SE_OK);
            let id = i32::from(c.id);
            let eq = |k: usize| i32::from(spc_i16(x, id, SPC_EQUIPMENT + 2 * k));
            m.item_num = match l.sy {
                0 => {
                    let job = i32::from(spc_i16(x, id, SPC_JOB));
                    let base = if (0..6).contains(&job) { job << 16 } else { m.item_num };
                    base | eq(4)
                }
                1 => eq(0) | 0x6_0000,
                2 => eq(1) | 0x7_0000,
                3 => eq(2) | 0x8_0000,
                _ => eq(3) | 0x9_0000,
            };
            m.msg.close();
            m.change_menu_to(64);
            return Flow::Done;
        }
    } else if push & 0x80 != 0 {
        x.se(SE_OK);
        m.temp[0] = i32::from(l.page);
        m.msg.close();
        m.change_menu_to(31);
        return Flow::Done;
    }
    if moved != 0 {
        status_help(m, x, &c);
    }
    Flow::Done
}

/// `StatusMenuDisp`.
pub fn status_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let (slot, c) = status_member(m, x);
    let l = m.lists[idx(m)].clone();
    let id = i32::from(c.id);
    let a = m.alpha;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    for (dx, dy, f) in [(39.0, 16.0, 0), (214.0, 32.0, 1), (46.0, 260.0, 2), (46.0, 150.0, 3), (228.0, 182.0, 4)] {
        m.win.dx = dx;
        m.win.dy = dy;
        match f {
            0 => disp_square(&mut m.win, 29, 16, None),
            1 => disp_line_v(&mut m.win, 10),
            2 => disp_line_h(&mut m.win, 28),
            3 => disp_line_h(&mut m.win, 10),
            _ => disp_line_h(&mut m.win, 15),
        }
    }
    m.item_icon.set_colour(7);
    m.item_icon.set_alpha(a);
    crate::panel::face_panel(m, x, 39.0, 0.0, a, slot, &c, 0x8080_8080, 0);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.shadow = true;
    set_type(&mut m.win, 1);
    m.win.set_colour(7);
    m.win.set_alpha(a);
    crate::menus::party::level_disp(m, x, 40.0, 80.0, id, false);
    // The equipment: its label and the five pieces' names.
    let t = &x.texts;
    let mut buf = Vec::new();
    str_cat(&mut buf, t.status_menu.get(4).map_or(&[][..], |v| &v[..]), 16);
    let eq = |k: usize| i32::from(spc_i16(x, id, SPC_EQUIPMENT + 2 * k));
    let job = i32::from(spc_i16(x, id, SPC_JOB));
    let bt = &t.pers.battle;
    let name = |p: Option<&piney_battle::param::EquipParam>| p.map(|p| p.name.clone()).unwrap_or_default();
    if eq(4) != -1 {
        str_cat(&mut buf, &name(bt.job_weapon(job, eq(4))), 16);
    } else {
        str_cat(&mut buf, b"", 16);
    }
    for (k, cat) in [(0, 6), (1, 7), (2, 8), (3, 9)] {
        if eq(k) != -1 {
            str_cat(&mut buf, &name(bt.equip(cat, eq(k))), 16);
        } else {
            str_cat(&mut buf, b"", 16);
        }
    }
    m.setting_text[6] = buf;
    let k6 = &mut m.setting[6];
    set_clm(k6, 16, 0, 0, 1);
    k6.set_colour(17);
    k6.set_alpha(a);
    k6.dx = 50.0;
    k6.dy = 162.0;
    k6.make_packet(0);
    k6.set_colour(7);
    k6.set_alpha(a);
    for k in 0..5 {
        k6.dx = 66.0;
        k6.dy = from_int(k * 17 + 180);
        k6.make_packet(k + 1);
    }
    let ic = &mut m.item_icon;
    ic.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    let weapon_icon = match job {
        1 => 3,
        2 => 5,
        3 => 6,
        4 => 7,
        5 => 1,
        _ => 0,
    };
    for (code, dy) in [(weapon_icon, 178.0), (9, 195.0), (12, 212.0), (11, 229.0), (13, 246.0)] {
        ic.dx = 50.0;
        ic.dy = dy;
        ic.make_packet(code);
    }
    // Skills, added effects, parameters.
    let skills: Vec<(i16, usize)> = skill_list(x, id).iter().map(|&s| (s, 7)).collect();
    skill_disp(m, x, 220.0, 30.0, &skills);
    let beff: [i16; 5] = std::array::from_fn(|k| c.condition[2 + k]);
    beff_disp(m, x, 220.0, 196.0, Some(&beff), None);
    let real = spc_real(x, id);
    param_disp(m, x, 40.0, 270.0, &real, &real);
    m.win.dx = 256.0;
    m.win.dy = 16.0;
    disp_button(&mut m.win, 2, x.count, x.frame_rate);
    m.setting_text[7] = t.status_menu.get(5).cloned().unwrap_or_default();
    let k7 = &mut m.setting[7];
    k7.set_colour(7);
    k7.set_alpha(m.kanji_alpha);
    set_clm(k7, 16, 0, 0, 1);
    k7.dx = 270.0;
    k7.dy = 8.0;
    k7.make_packet(0);
    let (mut cx, mut cy, mut cw) = (256, 8, 40);
    if x.world.party_num() >= 2 {
        cy = 200;
        for (dx, dy, b) in [(24.0, 200.0, 4), (24.0, 224.0, 13), (488.0, 200.0, 6), (488.0, 224.0, 12)] {
            m.win.dx = dx;
            m.win.dy = dy;
            disp_button(&mut m.win, b, x.count, x.frame_rate);
        }
        cx = 488;
    }
    let (sx, sy) = (i32::from(l.sx), i32::from(l.sy));
    if sx == 0 && sy < 5 {
        (cx, cy, cw) = (40, sy * 17 + 179, 10);
    } else if sx > 0 && sy < 8 {
        (cx, cy, cw) = ((sx - 1) * 126 + 221, sy * 17 + 47, 7);
    } else if sx > 0 && sy < 11 {
        (cx, cy, cw) = ((sx - 1) * 126 + 221, (sy - 8) * 17 + 213, 7);
    } else if sy >= 11 {
        if sx >= 2 {
            (cx, cy, cw) = ((sx - 2) * 90 + 292, (sy - 11) * 17 + 287, 4);
        } else {
            (cx, cy, cw) = (sx * 126 + 40, (sy - 11) * 17 + 287, 6);
        }
    }
    let fr = x.frame_rate;
    m.cursor.disp(&mut m.win, from_int(cx), from_int(cy), cw, sy + sx * 10, a, 3, fr);
}

/// `ccGetParamColor(a, b)` (0x0056a990): 20 when `a` rose over `b`, 23
/// when it fell, else 7 (both at most 990).
pub fn param_colour(a: i16, b: i16) -> usize {
    let a = i32::from(a).min(990);
    let b = i32::from(b).min(990);
    if b < a {
        20
    } else if a < b {
        23
    } else {
        7
    }
}

/// `ParameterDisp(x, y, btr, bto, atr, ato, tor, too)`: the physical and
/// magical abilities, the elements and the tolerances of `now` (a
/// `ccCharParamElement`, shown a tenth), each coloured against `was`.
pub fn param_disp(m: &mut MenuCtrl, x: &Ctx, px: f32, py: f32, now: &[i16; 16], was: &[i16; 16]) {
    let mut f24 = add(10.0, px);
    let f20 = add(18.0, py);
    m.setting_text[1] = x.texts.status_menu.get(1).cloned().unwrap_or_default();
    let ka = m.kanji_alpha;
    let k = &mut m.setting[1];
    k.set_colour(17);
    k.set_alpha(ka);
    set_clm(k, 16, 0, 0, 1);
    k.dx = f24;
    k.dy = py;
    k.make_packet(0);
    let f22 = add(80.0, add(46.0, f24));
    k.dx = f22;
    k.dy = py;
    k.make_packet(1);
    k.dx = add(84.0, add(46.0, f22));
    k.dy = py;
    k.make_packet(6);
    k.set_colour(7);
    k.set_alpha(ka);
    let rows = |r: usize| match r {
        0 => f20,
        1 => add(17.0, f20),
        2 => add(34.0, f20),
        _ => add(51.0, f20),
    };
    let labels = |m: &mut MenuCtrl, x0: f32| {
        let k = &mut m.setting[1];
        for s in 0..4 {
            k.dx = sub(x0, 1.0);
            k.dy = add(f20, mul(17.0, from_int(s)));
            k.make_packet(s + 2);
        }
    };
    let num = |m: &mut MenuCtrl, x0: f32, r: usize, v: i16, c: i16| {
        let f = &mut m.font;
        f.set_colour(param_colour(v, c));
        f.dx = x0;
        f.dy = rows(r);
        make_signed_num(f, 3, i64::from(i32::from(v) / 10));
    };
    labels(m, f24);
    f24 = add(f24, 48.0);
    m.font.set_alpha(m.alpha);
    for r in 0..4 {
        num(m, f24, r, now[r], was[r]);
    }
    f24 = add(f24, 78.0);
    labels(m, f24);
    f24 = add(f24, 48.0);
    for r in 0..4 {
        num(m, f24, r, now[4 + r], was[4 + r]);
    }
    f24 = add(f24, 82.0);
    let ic = &mut m.item_icon;
    ic.set_grid(14, 16, 14.0, 16.0, 0, 0x1400, 9);
    for r in 0..4 {
        ic.dx = f24;
        ic.dy = rows(r);
        ic.make_packet(r as i32);
    }
    f24 = add(f24, 8.0);
    for r in 0..4 {
        num(m, f24, r, now[8 + r], was[8 + r]);
    }
    f24 = add(f24, 84.0);
    let ic = &mut m.item_icon;
    for r in 0..4 {
        ic.dx = f24;
        ic.dy = rows(r);
        ic.make_packet(4 + r as i32);
    }
    f24 = add(f24, 8.0);
    for r in 0..4 {
        num(m, f24, r, now[12 + r], was[12 + r]);
    }
}

/// `SkillDisp(x, y, sklist)`: "Skills:" and up to sixteen skills, eight a
/// column, each in its colour (`sklist` pairs of skill and colour).
pub fn skill_disp(m: &mut MenuCtrl, x: &Ctx, px: f32, py: f32, list: &[(i16, usize)]) {
    let f21 = add(px, 10.0);
    m.setting_text[2] = x.texts.status_menu.get(2).cloned().unwrap_or_default();
    let ka = m.kanji_alpha;
    let k = &mut m.setting[2];
    k.set_colour(17);
    k.set_alpha(ka);
    set_clm(k, 16, 0, 0, 1);
    k.dx = f21;
    k.dy = py;
    k.make_packet(0);
    let f20 = add(py, 18.0);
    set_clm(&mut m.setting[3], 16, 0, 0, 1);
    set_clm(&mut m.setting[4], 16, 0, 0, 1);
    let mut buf = Vec::new();
    let (mut row, mut kanji) = (0i32, 3usize);
    for &(s, colour) in list.iter().take(20) {
        if s < 0 {
            continue;
        }
        let name = x.texts.items.skill(i32::from(s)).map(|p| p.name.clone()).unwrap_or_default();
        str_cat(&mut buf, &name, 16);
        let k = &mut m.setting[kanji];
        k.set_colour(colour.min(23));
        k.set_alpha(ka);
        k.dx = add(f21, from_int((kanji as i32 - 3) * 126));
        k.dy = add(f20, from_int((row % 8) * 17));
        k.make_packet(row);
        row += 1;
        if row >= 8 {
            m.setting_text[kanji] = std::mem::take(&mut buf);
            row = 0;
            kanji += 1;
            if kanji >= 5 {
                break;
            }
        }
    }
    if row != 0 {
        m.setting_text[kanji] = buf;
    }
}

/// `BeffDisp(x, y, cono, con)` (the `ccBattleEffect` form): "Added
/// Effects:" and the names of the effects either has; colour 20 for one
/// only `con` has, 23 for one only `cono` has, else 7.
pub fn beff_disp(m: &mut MenuCtrl, x: &Ctx, px: f32, py: f32, cono: Option<&[i16; 5]>, con: Option<&[i16; 5]>) {
    let f21 = add(px, 10.0);
    let mut buf = x.texts.status_menu.get(3).cloned().unwrap_or_default();
    let ka = m.kanji_alpha;
    let k = &mut m.setting[5];
    k.set_colour(17);
    k.set_alpha(ka);
    set_clm(k, 16, 0, 0, 1);
    k.dx = f21;
    k.dy = py;
    k.make_packet(0);
    let f20 = add(py, 18.0);
    set_clm(k, 16, 0, 0, 1);
    let old = |i: usize| cono.map_or(0, |c| c[i]);
    let mut n = 0i32;
    for i in 0..5 {
        let has_new = con.is_some_and(|c| c[i] != 0);
        if !has_new && old(i) == 0 {
            continue;
        }
        let name = x.texts.pers.beff_name.get(i).cloned().unwrap_or_default();
        str_cat(&mut buf, &name, 16);
        let colour = if old(i) == 0 {
            20
        } else if con.is_none() || has_new {
            7
        } else {
            23
        };
        let k = &mut m.setting[5];
        k.set_colour(colour);
        k.set_alpha(ka);
        k.dx = add(f21, from_int((n / 3) * 126));
        k.dy = add(f20, from_int((n % 3) * 17));
        k.make_packet(n + 1);
        n += 1;
    }
    m.setting_text[5] = buf;
}

// --- 31: the member's items ------------------------------------------------

/// The member `temp[0]` names (the `temp[0]`-th filled party slot).
pub fn items_member(m: &MenuCtrl, x: &Ctx) -> Option<(usize, CharInfo)> {
    page_member(x, m.temp[0])
}

fn member_item_list(m: &mut MenuCtrl, x: &Ctx, pc: usize) -> [Item; ITEMS] {
    let i = idx(m);
    let list = items::item_list(&x.texts.items, x.save, pc, i32::from(m.lists[i].page));
    let n = list.iter().filter(|it| it.cat >= 0).count() as i16;
    items::fit_list(&mut m.lists[i], n);
    list
}

/// `ItemStatusMenu` (menu 31).
pub fn item_status_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let who = items_member(m, x);
    match m.proccess {
        0 => {
            let Some((_, c)) = who else {
                let prev = m.lists[i].prev;
                m.back_to_prev(prev);
                return Flow::Done;
            };
            m.exception_disp = 1;
            m.bg_status = 1;
            let l = &mut m.lists[i];
            l.page_num = 5;
            if l.page >= l.page_num {
                l.page = l.page_num - 1;
            }
            if l.page < 0 {
                l.page = 0;
            }
            member_item_list(m, x, c.id.clamp(0, 17) as usize);
            m.proccess += 1;
        }
        1 => {
            let pc = who.map_or(0, |(_, c)| c.id.clamp(0, 17) as usize);
            let old_page = m.lists[i].page;
            let pn = i32::from(m.lists[i].page_num);
            m.select_scr(pn, 0, 0, x);
            if old_page != m.lists[i].page {
                m.kanji_alpha = -24;
            }
            let list = member_item_list(m, x, pc);
            let it = list.get(m.lists[i].select.max(0) as usize).copied().unwrap_or(Item::NONE);
            if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&it.cat) {
                x.se(SE_OK);
                m.item_num = (i32::from(it.cat) << 16) | i32::from(it.id);
                m.change_menu_to(64);
                return Flow::Done;
            }
            if key(x) == Some(false) {
                let prev = m.lists[i].prev;
                m.back_to_prev(prev);
                return Flow::Done;
            }
            let names = x.save.names();
            match x.texts.items.item(i32::from(it.cat), i32::from(it.id)).filter(|_| it.cat >= 0 && it.id >= 0) {
                Some(p) => {
                    let p = p.clone();
                    m.msg.disp_msg(
                        0x100,
                        Some(&p.name),
                        [Some(&p.comment[0]), Some(&p.comment[1]), Some(&p.comment[2])],
                        &names,
                    );
                }
                None => m.msg.disp_msg(0x100, None, [None, None, None], &names),
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `ItemStatusMenuDisp`: the member's Items page and their name in a frame
/// over it (nameKanji's party row).
pub fn item_status_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    crate::menus::item::item_menu_disp(m, x);
    let (slot, c) = items_member(m, x).unwrap_or((2, CharInfo::default()));
    let a = m.alpha;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = 87.0;
    m.win.dy = 28.0;
    let n = piney_desktop::message::str_len(&c.name, &x.save.names());
    disp_target(&mut m.win, n, 0);
    let k = &mut m.name;
    k.set_colour(7);
    k.set_alpha(a);
    k.dx = 112.0;
    k.dy = 33.0;
    k.make_packet(slot as i32 + 2);
}

// --- 64: a piece of equipment ----------------------------------------------

/// The cursor of EquipStatus (0x005394a4 - 0x005398e4).
fn equip_status_cursor(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let i = idx(m);
    let r = x.pad.repeat.bits();
    let (right, left, up, down) = (r & 0x2000 != 0, r & 0x8000 != 0, r & 0x1000 != 0, r & 0x4000 != 0);
    let l = &mut m.lists[i];
    let mut moved = 0;
    let mut se = false;
    if l.sx == 0 && l.sy == 0 {
        if right {
            l.sx += 1;
            l.sy = 0;
            se = true;
            moved += 1;
        } else if down {
            se = true;
            l.sy = 6;
            moved += 1;
        }
    } else if l.sx > 0 && l.sy < 3 {
        if right {
            l.sx += 1;
            if l.sx >= 3 {
                l.sx = 2;
            } else {
                se = true;
                moved += 1;
            }
        } else if left {
            l.sx -= 1;
            if l.sx <= 0 {
                l.sx = 0;
                l.sy = 0;
            }
            se = true;
            moved += 1;
        } else if up {
            l.sy -= 1;
            if l.sy < 0 {
                l.sy = 0;
            } else {
                se = true;
                moved += 1;
            }
        } else if down {
            se = true;
            l.sy += 1;
            moved += 1;
        }
    } else if l.sx > 0 && l.sy < 6 {
        if right {
            l.sx += 1;
            if l.sx >= 3 {
                l.sx = 2;
            } else {
                se = true;
                moved += 1;
            }
        } else if left {
            l.sx -= 1;
            if l.sx <= 0 {
                l.sx = 0;
                l.sy = 0;
            }
            se = true;
            moved += 1;
        } else if up {
            se = true;
            l.sy -= 1;
            moved += 1;
        } else if down {
            se = true;
            l.sy += 1;
            if l.sy >= 6 {
                l.sx += 1;
                l.sy = 6;
            }
            moved += 1;
        }
    } else if l.sy >= 6 {
        if right {
            l.sx += 1;
            if l.sx >= 4 {
                l.sx = 3;
            } else {
                se = true;
                moved += 1;
            }
        } else if left {
            l.sx -= 1;
            if l.sx < 0 {
                l.sx = 0;
            } else {
                se = true;
                moved += 1;
            }
        } else if up {
            se = true;
            l.sy -= 1;
            if l.sy < 6 {
                if l.sx < 2 {
                    l.sx = 0;
                    l.sy = 0;
                } else {
                    l.sx -= 1;
                    l.sy = 5;
                }
            }
            moved += 1;
        } else if down {
            l.sy += 1;
            if l.sy >= 10 {
                l.sy = 9;
            } else {
                se = true;
                moved += 1;
            }
        }
    }
    if se {
        x.se(SE_MOVE);
    }
    moved
}

/// `ccGetEquipParam(code)` (0x005710d0): `code >> 16` the category.
pub fn equip_by_code(x: &Ctx, code: i32) -> Option<piney_battle::param::EquipParam> {
    x.texts.pers.battle.equip(code >> 16, code & 0xffff).cloned()
}

/// `EquipStatusMenu` (menu 64).
pub fn equip_status_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let mut moved = 0;
    match m.proccess {
        0 => {
            m.exception_disp = 1;
            m.lists[i].sx = 0;
            m.lists[i].sy = 0;
            moved = 1;
            m.proccess += 1;
        }
        1 => {}
        _ => return Flow::Done,
    }
    moved += equip_status_cursor(m, x);
    if x.pushed_cancel() {
        x.se(crate::ctrl::SE_BACK);
        m.msg.close();
        let prev = m.lists[i].prev;
        m.back_to_prev(prev);
    }
    if moved == 0 {
        return Flow::Done;
    }
    let l = m.lists[i].clone();
    let (sx, sy) = (i32::from(l.sx), i32::from(l.sy));
    let names = x.save.names();
    if sx == 0 && sy == 0 {
        let (cat, id) = (m.item_num >> 16, m.item_num & 0xffff);
        match x.texts.items.item(cat, id).filter(|_| cat >= 0) {
            Some(p) => {
                let p = p.clone();
                m.msg.change(0x100, Some(&p.name), [Some(&p.comment[0]), Some(&p.comment[1]), None], &names);
            }
            None => m.msg.change(0x100, None, [None, None, None], &names),
        }
        return Flow::Done;
    }
    let t = &x.texts.pers;
    let mut help = t.status_help.get((5 + sx * 15 + sy).max(0) as usize).cloned().unwrap_or_default();
    if sx > 0 && sy < 6 {
        let ep = equip_by_code(x, m.item_num).unwrap_or_default();
        if sy < 3 {
            if sx == 1 {
                let s = i32::from(ep.skill_id[sy as usize]);
                if s >= 0
                    && let Some(p) = x.texts.items.skill(s)
                {
                    help = p.help.clone();
                }
            }
        } else {
            let want = (sy - 3) + (sx - 1) * 3;
            let mut n = 0;
            for k in 0..5 {
                if ep.beff[k] != 0 {
                    if n == want {
                        help = t.beff_help[k].clone();
                    }
                    n += 1;
                }
            }
        }
    }
    let third = if sx > 0 && sy < 6 { Some(&help[2][..]) } else { None };
    m.msg.change(0x100, None, [Some(&help[0]), Some(&help[1]), third], &names);
    Flow::Done
}

/// `EquipStatusMenuDisp`.
pub fn equip_status_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let l = m.lists[idx(m)].clone();
    let a = m.alpha;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    for (dx, dy, f) in [(39.0, 58.0, 0), (214.0, 70.0, 1), (46.0, 217.0, 2), (228.0, 139.0, 3)] {
        m.win.dx = dx;
        m.win.dy = dy;
        match f {
            0 => disp_square(&mut m.win, 29, 12, None),
            1 => disp_line_v(&mut m.win, 6),
            2 => disp_line_h(&mut m.win, 28),
            _ => disp_line_h(&mut m.win, 15),
        }
    }
    m.item_icon.set_colour(7);
    m.item_icon.set_alpha(a);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.shadow = true;
    set_type(&mut m.win, 1);
    m.win.set_colour(7);
    m.win.set_alpha(a);
    let ep = equip_by_code(x, m.item_num).unwrap_or_default();
    let mut buf = Vec::new();
    str_cat(&mut buf, x.texts.status_menu.get(4).map_or(&[][..], |v| &v[..]), 16);
    str_cat(&mut buf, &ep.name, 16);
    m.setting_text[6] = buf;
    let k6 = &mut m.setting[6];
    set_clm(k6, 16, 0, 0, 1);
    k6.set_colour(17);
    k6.set_alpha(a);
    k6.dx = 50.0;
    k6.dy = 72.0;
    k6.make_packet(0);
    k6.set_colour(7);
    k6.set_alpha(a);
    k6.dx = 66.0;
    k6.dy = 90.0;
    k6.make_packet(1);
    let ic = &mut m.item_icon;
    ic.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    ic.dx = 50.0;
    ic.dy = 88.0;
    ic.make_packet(x.texts.items.item_icon(m.item_num >> 16));
    let mut skills = vec![(-1i16, 7usize); 20];
    for (k, s) in skills.iter_mut().take(3).enumerate() {
        s.0 = ep.skill_id[k];
    }
    skill_disp(m, x, 220.0, 72.0, &skills);
    beff_disp(m, x, 220.0, 153.0, Some(&ep.beff), None);
    param_disp(m, x, 40.0, 227.0, &ep.elm, &ep.elm);
    let (sx, sy) = (i32::from(l.sx), i32::from(l.sy));
    let (mut cx, mut cy, mut cw) = (220, 153, 40);
    if sx == 0 && sy == 0 {
        (cx, cy, cw) = (40, 89, 10);
    } else if sx > 0 && sy < 3 {
        (cx, cy, cw) = ((sx - 1) * 126 + 221, sy * 17 + 89, 7);
    } else if sx > 0 && sy < 6 {
        (cx, cy, cw) = ((sx - 1) * 126 + 221, (sy - 3) * 17 + 170, 7);
    } else if sy >= 6 {
        if sx >= 2 {
            (cx, cy, cw) = ((sx - 2) * 90 + 292, (sy - 6) * 17 + 244, 4);
        } else {
            (cx, cy, cw) = (sx * 126 + 40, (sy - 6) * 17 + 244, 6);
        }
    }
    let fr = x.frame_rate;
    m.cursor.disp(&mut m.win, from_int(cx), from_int(cy), cw, sy + sx * 10, a, 3, fr);
}
