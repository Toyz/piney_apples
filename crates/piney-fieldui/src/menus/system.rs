//! `SystemMenu` (gcmn 0x00528810): the PERSONAL lists (menus 0, 1, 2:
//! town, field, dungeon; the triangle button) and OPTION (12, START). Gate
//! Out is greyed in battle or while the events hold operation 14; Skills,
//! Items and Key Items while the player is down, where OK on them says
//! "Cannot be used while dead.". The steps are in docs/engine/field-ui.md.

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::check_operate;

/// `ccKanjiStrcat(buf, s, n)` (main 0x0015eed0): `s` as `n` glyphs,
/// padded with spaces. `#0` / `#1` (the names) are left out here; `#x`
/// colour codes are kept and not counted; `%x` counts one; a Shift-JIS
/// character becomes its `%x` code (`ccGetExtendedCode`) or else `_` for
/// its first byte alone.
pub fn str_cat(buf: &mut Vec<u8>, s: &[u8], n: usize) {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let (mut i, mut t) = (0usize, 0usize);
    while t < n {
        let c = at(i);
        if c == b'#' && (at(i + 1) == b'0' || at(i + 1) == b'1') {
            i += 2;
            continue;
        }
        if c == 0 {
            buf.push(b' ');
            t += 1;
            continue;
        }
        if c == b'#' {
            buf.extend_from_slice(&[c, at(i + 1)]);
            i += 2;
            continue;
        }
        if c == b'%' {
            buf.extend_from_slice(&[c, at(i + 1)]);
            i += 2;
            t += 1;
            continue;
        }
        if (0x20..0x80).contains(&c) {
            buf.push(c);
            i += 1;
            t += 1;
            continue;
        }
        let e = piney_desktop::kanji::extended_code(u16::from_be_bytes([c, at(i + 1)]));
        if (e >> 8) as u8 == b'%' {
            buf.extend_from_slice(&e.to_be_bytes());
            i += 2;
        } else {
            buf.push(b'_');
            i += 1;
        }
        t += 1;
    }
}

/// The rows of a list: each item's name, 16 glyphs a row.
pub fn item_rows(m: &MenuCtrl) -> Vec<u8> {
    let l = m.list();
    let mut buf = Vec::new();
    for &it in l.items.iter().take(l.y.max(0) as usize) {
        let name = m.lists.get(it.max(0) as usize).map(|x| x.name.clone()).unwrap_or_default();
        str_cat(&mut buf, &name, 16);
    }
    buf
}

/// `ccKanji::Extract(menuKanji, s)`.
pub fn extract_menu(m: &mut MenuCtrl, s: Vec<u8>) {
    m.note(|| format!("[\"extract\",\"kanji\",\"{}\"]", String::from_utf8_lossy(&s)));
    m.kanji_text = s;
}

pub fn system_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            let rows = item_rows(m);
            extract_menu(m, rows);
            x.req.push(Request::TargetFix(true));
            x.change_target(None);
            m.bg_status = 1;
            let g = x.world.game;
            match m.menu {
                1 => {
                    if g.in_battle != 0 || !check_operate(x, 14, 1) {
                        m.reverse_head |= 128;
                    }
                }
                2 if (g.dungeon_type == 8 || g.dungeon_type == 9) && (g.in_battle != 0 || !check_operate(x, 14, 1)) => {
                    m.reverse_head |= 128;
                }
                _ => {}
            }
            if m.menu != 12 && x.world.player_dead {
                m.reverse_head |= 7;
            }
            let l = m.list_mut();
            if l.select >= l.y {
                l.select = l.y - 1;
            }
            if l.select < 0 {
                l.select = 0;
            }
            m.proccess += 1;
        }
        1 => {
            m.select(0, 0, true, x);
            let key = if x.pushed_cancel() {
                x.se(SE_BACK);
                Some(false)
            } else if x.pushed_ok() {
                x.se(SE_OK);
                Some(true)
            } else {
                None
            };
            match key {
                Some(false) => {
                    let prev = m.list().prev;
                    if prev == -1 {
                        m.menu_next = -1;
                        m.menu_status = 3;
                        m.bg_status = 3;
                        let woke = m.breathe_close(x);
                        return Flow::Breathed(Cont { woke, cursors: false, after: After::SystemHelp });
                    }
                    m.back_to_prev(prev);
                }
                Some(true) => {
                    let sel = m.list().select;
                    if m.menu != 12 && (0..=2).contains(&sel) && x.world.player_dead {
                        m.menu_status = 3;
                        m.proccess += 1;
                    } else {
                        let menu = m.menu;
                        let flow = m.change_menu(x);
                        if let Flow::Breathed(c) = flow {
                            let after = if menu == 12 { After::SystemHelp } else { After::SystemChanged };
                            return Flow::Breathed(Cont { after, ..c });
                        }
                        if menu != 12 {
                            after_change(m);
                        }
                    }
                }
                None => {}
            }
        }
        2 => {
            if m.menu_status == 0 {
                let names = x.save.names();
                let info = x.texts.dead_info.clone();
                m.msg.open_info([Some(&info), None, None, None], &names);
                m.note(|| "[\"open_info\"]".into());
                m.proccess += 1;
            }
        }
        3 => {
            let ok = x.save.ok();
            let mut req = Vec::new();
            let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
            push_msg_requests(x, req);
            if r != 0 {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        4 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 11 {
                m.menu_status = 1;
                m.proccess = 1;
            }
        }
        _ => {}
    }
    help(m, x);
    Flow::Done
}

/// The desktop message's requests as the field's.
pub fn push_msg_requests(x: &mut Ctx, req: Vec<piney_desktop::Request>) {
    for r in req {
        match r {
            piney_desktop::Request::Se(s) => x.se(s.0),
            piney_desktop::Request::VoiceStop => x.req.push(Request::VoiceStop),
            _ => {}
        }
    }
}

/// After `ChangeMenu` from PERSONAL: Equipment starts at the first member.
pub fn after_change(m: &mut MenuCtrl) {
    if m.list().select == 5 {
        m.first_time = 1;
        m.set_equip_spc_num(0);
    }
}

/// The list's help line: `DispMsg({0x100, lines}, 0)` while `proccess` is
/// below 2 (0x00528d68).
pub fn help(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.proccess >= 2 {
        return;
    }
    let mut a1 = i32::from(m.list().select);
    let lines = match m.menu {
        12 => {
            a1 = a1.clamp(0, 7);
            x.texts.option_help.get(a1 as usize)
        }
        0 => {
            if a1 >= 7 {
                a1 = 8;
            }
            x.texts.personal_help.get(a1 as usize)
        }
        1 => {
            if a1 == 6 {
                a1 = 9;
            } else if a1 >= 7 {
                a1 = 7;
            }
            x.texts.personal_help.get(a1 as usize)
        }
        2 => {
            if a1 >= 6 {
                a1 = 9;
            }
            x.texts.personal_help.get(a1 as usize)
        }
        _ => None,
    };
    let Some(lines) = lines.cloned() else { return };
    if lines[0].is_empty() && lines[1].is_empty() {
        return;
    }
    let names = x.save.names();
    m.msg.disp_msg(0x100, None, [Some(&lines[0]), Some(&lines[1]), Some(&lines[2])], &names);
}
