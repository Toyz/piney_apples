//! The party's pages: the members who can join (`PartyInMenuDisp`, gcmn
//! 0x0053b6e0, for PARTY's Add (68) and the tutorial's (77)) and a
//! member's level box (`LevelDisp`, 0x00523a30).
//!
//! ```text
//! PartyInMenuDisp   the names of the members of partyMemberFlag not in
//!                   the party, rows dy to dy + y, into menuKanji; the
//!                   chosen member's face (menuFace[3]) and panel at
//!                   (220, 96) with full HP and SP from spcParam; the
//!                   level box under it at (220, 192)
//! LevelDisp         Level, EXP, Money, Class (statusMenuStr[0]) and the
//!                   class's name in settingKanji[0], the numbers in
//!                   fontTex's type 1 digits: "n/1000", "nGP"
//! ```

use piney_desktop::eef::add;

use crate::ctrl::{Ctx, MenuCtrl};
use crate::disp::member_name;
use crate::menus::system::str_cat;
use crate::spr::{font_type, make_num, set_clm};
use crate::window::{disp_square, set_type};
use crate::world::CharInfo;

/// `saveData.partyMemberFlag` (+0x2220): a bit per member who can join.
pub const PARTY_MEMBER_FLAG: usize = 0x2220;
/// `saveData.spcParam[18]` (+0x7488, 0xdc bytes each).
pub const SPC_PARAM: usize = 0x7488;
pub const SPC_PARAM_SIZE: usize = 0xdc;

/// What the menus read of `spcParam[id]` (`ccGetCharParam(id)`, gcmn
/// 0x00570cc0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpcParam {
    /// +0x0e `level`.
    pub level: i16,
    /// +0x10: the experience toward the next level (of 1000).
    pub exp: i16,
    /// +0x14 `gold`.
    pub money: i32,
    /// +0x24, +0x26: HP and SP.
    pub hp: i16,
    pub sp: i16,
    /// +0xd8: the class (`statusClassStr`'s line).
    pub class: i16,
}

/// `ccGetCharParam(id)`: the save's record of member `id`.
pub fn spc(x: &Ctx, id: i32) -> SpcParam {
    let at = SPC_PARAM + SPC_PARAM_SIZE * id.clamp(0, 17) as usize;
    let s = &x.save.save;
    SpcParam {
        level: s.i16(at + 0x0e),
        exp: s.i16(at + 0x10),
        money: s.i32(at + 0x14),
        hp: s.i16(at + 0x24),
        sp: s.i16(at + 0x26),
        class: s.i16(at + 0xd8),
    }
}

/// `ccCheckMenuFaceName(n)` (0x0056a8f0): `menuFaceCcsList`'s row for
/// member `n` (Kite's own is row 18 until the bracelet's colour is set).
pub fn menu_face_row(x: &Ctx, n: i32) -> i32 {
    if n == 0 && x.save.save.u8(piney_data::save::offset::PLCOL) == 0 { 18 } else { n }
}

/// `partyMemberFlag` bit `k` set and member `k` not in the party.
pub fn can_join(x: &Ctx, k: i32) -> bool {
    let flag = x.save.save.i32(PARTY_MEMBER_FLAG) as u32;
    flag & (1 << k) != 0 && x.world.member_slot(k) < 0
}

/// `PartyInMenuDisp` (menus 68 and 77).
pub fn party_in_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = m.menu.clamp(0, 88) as usize;
    let (dy, y) = (i32::from(m.lists[i].dy), i32::from(m.lists[i].y));
    let mut buf = Vec::new();
    let mut row = 0;
    for k in 0..18 {
        if !can_join(x, k) {
            continue;
        }
        if row >= dy {
            if y < row - dy {
                break;
            }
            str_cat(&mut buf, &member_name(x, k), 16);
        }
        row += 1;
    }
    crate::menus::system::extract_menu(m, buf);
    let face = i32::from(m.face_num);
    m.face_tex[3] = menu_face_row(x, face);
    let p = spc(x, face);
    font_type(&mut m.font, 2);
    m.font.set_colour(22);
    m.font.set_alpha(m.alpha);
    let c = CharInfo {
        id: face as i16,
        name: member_name(x, face),
        hp: p.hp,
        max_hp: p.hp,
        sp: p.sp,
        max_sp: p.sp,
        ..CharInfo::default()
    };
    let a = m.alpha;
    crate::panel::face_panel(m, x, 220.0, 96.0, a, 3, &c, 0x8080_8080, 0);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.shadow = true;
    set_type(&mut m.win, 1);
    m.win.set_colour(7);
    m.win.set_alpha(a);
    level_disp(m, x, 220.0, 192.0, face, true);
}

/// `LevelDisp(x, y, id, frame)`: member `id`'s level box at (x, y), its
/// frame (`DispSquare(11, 3)`) when `frame`.
pub fn level_disp(m: &mut MenuCtrl, x: &mut Ctx, px: f32, py: f32, id: i32, frame: bool) {
    let p = spc(x, id);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(m.alpha);
    m.font.shadow = true;
    let mut buf = x.texts.status_menu.first().cloned().unwrap_or_default();
    let class = x.texts.status_class.get(p.class.max(0) as usize).cloned().unwrap_or_default();
    str_cat(&mut buf, &class, 16);
    m.setting_text[0] = buf;
    let k = &mut m.setting[0];
    set_clm(k, 16, 0, 0, 1);
    k.set_colour(7);
    k.set_alpha(m.kanji_alpha);
    if frame {
        m.win.dx = px;
        m.win.dy = py;
        disp_square(&mut m.win, 11, 3, None);
    }
    let x0 = add(px, 10.0);
    let mut y0 = add(py, 8.0);
    let k = &mut m.setting[0];
    k.dx = x0;
    k.dy = y0;
    k.make_packet(0);
    let nx = add(52.0, x0);
    let f = &mut m.font;
    f.dx = nx;
    f.dy = y0;
    make_num(f, if p.level < 10 { 1 } else { 2 }, i32::from(p.level));
    y0 = add(y0, 17.0);
    let k = &mut m.setting[0];
    k.dx = x0;
    k.dy = y0;
    k.make_packet(1);
    let f = &mut m.font;
    f.dx = nx;
    f.dy = y0;
    let n = if p.exp >= 100 {
        3
    } else if p.exp >= 10 {
        2
    } else {
        1
    };
    make_num(f, n, i32::from(p.exp));
    f.make_str(&x.texts.exp_max);
    y0 = add(y0, 17.0);
    let k = &mut m.setting[0];
    k.dx = x0;
    k.dy = y0;
    k.make_packet(2);
    let f = &mut m.font;
    f.dx = nx;
    f.dy = y0;
    let mut n = 1;
    let mut v = p.money;
    while v != 0 {
        v /= 10;
        if v != 0 {
            n += 1;
        }
    }
    make_num(f, n, p.money);
    f.make_str(&x.texts.gp);
    y0 = add(y0, 17.0);
    let k = &mut m.setting[0];
    k.dx = x0;
    k.dy = y0;
    k.make_packet(3);
    k.dx = add(56.0, x0);
    k.dy = y0;
    k.make_packet(4);
}
