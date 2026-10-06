//! `ccMenuCtrl::Disp` (gcmn 0x0051cfb0), once a frame from the task: the
//! whole field HUD on the menu layer. In order: the battle band's clock,
//! the target, the menu's fades and page, the dim, the minimap's alpha, the
//! bracelet's gauge, `nameKanji`, the target's cursor and window, the party
//! panels, the enemies' life bars, the banners, `ccMsg->Disp`, then every
//! sprite's `SendPacket` in a fixed order. The details are in
//! docs/engine/field-ui.md (the HUD).

use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_desktop::eef::{add, div, from_int, mul, sub, to_int};

use crate::ctrl::{Ctx, Draw, MenuCtrl};
use crate::menus::system::str_cat;
use crate::spr::{Anchor, Obj, Spr, font_type, make_num, set_clm};
use crate::window::{self, disp_button, disp_square, disp_square_sb, disp_square_tag, disp_target, set_type};
use crate::world::CharInfo;
use crate::{Noise, Request};
use piney_data::volume::Volume;

/// `ccGetNowMaxColor(n, m)` (0x0056a9f0): a colour index by how full a
/// gauge is: 2 below 20 %, 23 below 40 %, 6 below 60 %, else 22.
pub fn now_max_colour(n: i32, m: i32) -> usize {
    let p = if m != 0 { (n * 100) / m } else { 0 };
    if p < 20 {
        2
    } else if p < 40 {
        23
    } else if p < 60 {
        6
    } else {
        22
    }
}

/// A `ccSpriteColorTable` entry as the 0x00BBGGRR word the code adds to.
fn table_u32(i: usize) -> u32 {
    let c = SPRITE_COLOR_TABLE[i];
    u32::from(c[0]) | (u32::from(c[1]) << 8) | (u32::from(c[2]) << 16)
}

/// Sends a sprite's queue.
fn send(m: &mut MenuCtrl, obj: Obj) {
    let (packets, text) = {
        let (s, text) = sprite_mut(m, obj);
        (s.take(), text)
    };
    if matches!(obj, Obj::MenuKanji | Obj::MenuKanjiPr | Obj::NameKanji | Obj::NameKanjiPr | Obj::Setting(_)) {
        m.draws.push(Draw::Kanji { obj, text, packets });
    } else {
        m.draws.push(Draw::Send(packets));
    }
}

/// Not the game's: the HUD scale's anchor ([`crate::spr::Anchor`]) for
/// every packet the menu's sprites make from now on.
fn set_anchor(m: &mut MenuCtrl, a: Anchor) {
    for s in [
        &mut m.win,
        &mut m.win_pr,
        &mut m.win_a,
        &mut m.ene_life,
        &mut m.target,
        &mut m.kanji,
        &mut m.kanji_pr,
        &mut m.name,
        &mut m.name_pr,
        &mut m.font,
        &mut m.fly,
        &mut m.item_icon,
        &mut m.con_icon,
        &mut m.bg,
        &mut m.drain,
        &mut m.protect_spr,
        &mut m.mask,
    ] {
        s.anchor = a;
    }
    for s in m.setting.iter_mut().chain(m.faces.iter_mut()) {
        s.anchor = a;
    }
}

fn sprite_mut(m: &mut MenuCtrl, obj: Obj) -> (&mut Spr, Vec<u8>) {
    match obj {
        Obj::MenuWindow => (&mut m.win, Vec::new()),
        Obj::MenuWindowPr => (&mut m.win_pr, Vec::new()),
        Obj::MenuWindowA => (&mut m.win_a, Vec::new()),
        Obj::EneLife => (&mut m.ene_life, Vec::new()),
        Obj::TargetCursol => (&mut m.target, Vec::new()),
        Obj::MenuKanji => (&mut m.kanji, m.kanji_text.clone()),
        Obj::MenuKanjiPr => (&mut m.kanji_pr, m.kanji_pr_text.clone()),
        Obj::NameKanji => (&mut m.name, m.name_text.clone()),
        Obj::NameKanjiPr => (&mut m.name_pr, m.name_pr_text.clone()),
        Obj::Setting(i) => {
            let t = m.setting_text[i as usize].clone();
            (&mut m.setting[i as usize], t)
        }
        Obj::MenuFont => (&mut m.font, Vec::new()),
        Obj::FlyFont => (&mut m.fly, Vec::new()),
        Obj::MenuFace(i) => (&mut m.faces[i as usize], Vec::new()),
        Obj::ItemIcon => (&mut m.item_icon, Vec::new()),
        Obj::ConIcon => (&mut m.con_icon, Vec::new()),
        Obj::MenuBg => (&mut m.bg, Vec::new()),
        Obj::MenuDrain => (&mut m.drain, Vec::new()),
        Obj::MenuProtect => (&mut m.protect_spr, Vec::new()),
        Obj::ChatWindow => (&mut m.chat.window, Vec::new()),
        // The gate hack's own two are never sent from here, nor the chat's
        // texts (drawn at once), nor ccDfComp's or a book's (their own
        // tasks').
        Obj::MenuMask
        | Obj::MenuIcon
        | Obj::HackMask
        | Obj::SysFont
        | Obj::ChatKanji(_)
        | Obj::DfComp
        | Obj::BookBg
        | Obj::BookWin
        | Obj::BookButton
        | Obj::BookTitle
        | Obj::BookMsg(_) => (&mut m.mask, Vec::new()),
    }
}

/// A status's fade: `status` 1 in by `step_in` to `top` (then 2), 2 held,
/// 3 out by 28 to 0 (then 0), 0 nothing.
fn fade(status: &mut i16, alpha: &mut i32, step_in: i32, top: i32) {
    match *status {
        0 => *alpha = 0,
        1 => {
            *alpha += step_in;
            if *alpha >= top {
                *alpha = top;
                *status = 2;
            }
        }
        2 => *alpha = top,
        3 => {
            *alpha -= 28;
            if *alpha <= 0 {
                *alpha = 0;
                *status = 0;
            }
        }
        _ => {}
    }
}

/// `ccMenuCtrl::Disp`.
pub fn disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let menu_idx = m.menu.clamp(0, 88) as usize;
    let game = x.world.game;
    // The battle band's clock.
    let mut blink = 0;
    let t = m.check_menu_type();
    if game.in_battle == 1 && game.in_battle_cnt == 0 && m.menu_status == 0 && (t == -1 || t == 88) {
        m.battle_cnt += 1;
        if m.battle_cnt >= 49 {
            m.battle_cnt = 0;
        }
        blink = i32::from(m.battle_cnt);
        if blink >= 25 {
            blink -= 24;
        }
        blink = if blink >= 12 { ((24 - blink) * 96) / 12 } else { (blink * 96) / 12 };
    }
    // The target.
    let target: Option<CharInfo> = if x.target.is_some() {
        x.target.clone()
    } else if x.target_prev.is_some() {
        x.target_prev.clone()
    } else {
        m.dummy_target.clone()
    };
    // The window.
    match m.menu_status {
        0 => {
            m.alpha = 0;
            m.kanji_alpha = 0;
        }
        1 => {
            m.alpha += 24;
            if m.alpha >= 128 {
                m.alpha = 128;
                m.menu_status = 2;
            }
            m.kanji_alpha += 24;
            if m.kanji_alpha >= 128 {
                m.kanji_alpha = 128;
            }
        }
        2 => {
            m.alpha = 128;
            m.kanji_alpha += 24;
            if m.kanji_alpha >= 128 {
                m.kanji_alpha = 128;
            }
        }
        3 => {
            m.alpha -= 28;
            if m.alpha <= 0 {
                m.alpha = 0;
                m.exception_disp = 0;
                m.menu_status = 0;
            }
            m.kanji_alpha -= 28;
            if m.kanji_alpha <= 0 {
                m.kanji_alpha = 0;
            }
        }
        _ => {}
    }
    m.win.set_alpha(m.alpha);
    set_type(&mut m.win, 1);
    // Not the game's: the HUD scale's anchors, by what each part draws.
    set_anchor(m, Anchor::NONE);
    if m.exception_disp != 0 {
        crate::menus::exception_disp(m, x);
    }
    if m.alpha != 0 {
        list_window(m, x, menu_idx, target.as_ref());
    }
    // The dim.
    fade(&mut m.bg_status, &mut m.bg_alpha, 15, 104);
    if m.bg_alpha != 0 {
        let c = m.bg_col.clamp(0, 23) as usize;
        m.bg.set_colour(c);
        m.bg.set_alpha(m.bg_alpha);
        for k in 0..8 {
            m.bg.dx = from_int((k % 4) << 7);
            m.bg.dy = from_int((k / 4) * 224);
            m.bg.make_packet(0);
        }
    }
    // The minimap.
    fade(&mut m.map_status, &mut m.map_alpha, 15, 128);
    x.req.push(Request::MapAlpha(div(from_int(m.map_alpha), 128.0)));
    // The bracelet's gauge.
    fade(&mut m.drain_status, &mut m.drain_alpha, 15, 128);
    if x.save.save.bytes()[0x64f8 + 40] != 0 {
        m.drain_alpha = 0;
    }
    if m.drain_alpha != 0 {
        set_anchor(m, Anchor::TOP_RIGHT);
        drain_gauge(m, x);
        set_anchor(m, Anchor::NONE);
    }
    name_texts(m, x, target.as_ref());
    target_cursor(m, x, target.as_ref());
    if m.target_alpha != 0
        && let Some(t) = target.as_ref()
    {
        set_anchor(m, Anchor::TOP_LEFT);
        target_window(m, x, t);
    }
    set_anchor(m, Anchor::BOTTOM_LEFT);
    panels(m, x);
    set_anchor(m, Anchor::NONE);
    if !x.world.game_over {
        enemy_bars(m, x);
    }
    protect_marks(m, x);
    set_anchor(m, Anchor::TOP_BAND);
    banner(m, x, blink);
    set_anchor(m, Anchor::TOP_LEFT);
    new_mail(m, x);
    set_anchor(m, Anchor::NONE);
    // ccMsg->Disp.
    let draws = m.msg.disp(x.frame_rate);
    m.draws.push(Draw::Msg(draws));
    // menuFade->SendPacket: each element up, then counted on.
    for (c0, c1, cnt, tcnt) in m.menu_fade.advance() {
        m.draws.push(Draw::Fade { c0, c1, cnt, tcnt });
    }
    // The sends, in Disp's order (each put at the front of the layer).
    for o in [Obj::MenuKanjiPr, Obj::NameKanjiPr, Obj::MenuFont, Obj::MenuWindowPr, Obj::MenuKanji, Obj::NameKanji] {
        send(m, o);
    }
    for i in 0..8 {
        if m.setting_send[i] {
            send(m, Obj::Setting(i as u8));
        }
        m.setting_send[i] = true;
    }
    send(m, Obj::ItemIcon);
    send(m, Obj::MenuWindowA);
    for i in 0..3 {
        if x.world.party[i].is_some() {
            send(m, Obj::MenuFace(i as u8));
        }
    }
    for o in
        [Obj::MenuFace(3), Obj::MenuWindow, Obj::TargetCursol, Obj::ConIcon, Obj::MenuMask, Obj::MenuDrain, Obj::MenuBg]
    {
        send(m, o);
    }
    // ccChat->Disp(still): the chat balloons.
    let player = x.world.party[0].as_ref().map(|c| c.handle);
    m.chat.disp(m.still != 0, &x.world.chat_at, player, &mut m.draws);
    for o in [Obj::FlyFont, Obj::MenuProtect, Obj::EneLife] {
        send(m, o);
    }
    inter_noiz(m, x);
    let cmds = m.noiz.draw();
    if !cmds.is_empty() {
        m.draws.push(Draw::Noiz(cmds));
    }
}

/// A call on the menu's noise (`ccNoiz`, +0xe8): traced, and sound 94
/// when `SetNoiz` switches anything on.
pub fn noiz(m: &mut MenuCtrl, x: &mut Ctx, n: Noise) {
    m.note(|| match n {
        Noise::Set(a, b, c) => format!("[\"noiz\",{a},{b},{c}]"),
        Noise::Rn(v) => format!("[\"noiz_rn\",{v}]"),
        Noise::Bs(v) => format!("[\"noiz_bs\",{v}]"),
        Noise::Br(v) => format!("[\"noiz_br\",{v}]"),
    });
    match n {
        Noise::Set(a, b, c) => {
            if m.noiz.set_noiz(a, b, c, &mut || m.rng.rand()) {
                x.se(piney_desktop::noiz::NOIZ_SE);
            }
        }
        Noise::Rn(v) => m.noiz.set_rn(v, &mut || m.rng.rand()),
        Noise::Bs(v) => m.noiz.set_bs(v),
        Noise::Br(v) => m.noiz.set_br(v),
    }
}

/// `Disp`'s `interNoiz` (INF 0x00522220 - 0x005223e4; MUT 0x005403c8 -
/// 0x005406fc, the same on Outbreak and Quarantine), before `ccNoiz::Draw`:
/// 2 is one `SetNoiz(12, 12, 12)` burst; Infection's 3 in town 4 and, from
/// Mutation on, the event levels 3 - 5 make their own noise; otherwise, on
/// a Grunty or with no menu, ban or event, a one-in-four chance every 64th
/// frame, and in a menu but Data Drain's (66) and the gate hack's (62) the
/// noise off. The rules are in docs/engine/field-ui.md (the noise).
fn inter_noiz(m: &mut MenuCtrl, x: &mut Ctx) {
    let level = m.inter_noiz;
    if level == 0 {
        return;
    }
    if level == 2 {
        noiz(m, x, Noise::Set(12, 12, 12));
        m.inter_noiz = 0;
        return;
    }
    let g = x.world.game;
    let count = x.count;
    if x.texts.volume == Volume::Inf {
        if level == 3 && g.area == 0 && g.town == 4 {
            if count & 0xf == 0 {
                let a = m.rng.rand() % 10 + 4;
                let c = m.rng.rand() % 10 + 4;
                noiz(m, x, Noise::Set(a, 255, c));
            }
            return;
        }
    } else {
        let random = |m: &mut MenuCtrl| {
            let a = m.rng.rand() % 10 + 10;
            let b = m.rng.rand() % 10 + 15;
            let c = m.rng.rand() % 10 + 4;
            Noise::Set(a, b, c)
        };
        match level {
            3 => {
                if count & 0x3f == 0 && (count & 0x7f == 0 || m.rng.rand() & 1 != 0) {
                    let n = random(m);
                    noiz(m, x, n);
                }
                return;
            }
            4 => {
                if count & 0xf == 0 {
                    noiz(m, x, Noise::Set(255, 255, 255));
                }
                return;
            }
            5 => {
                if count & 0x1f == 0 {
                    let n =
                        if count & 0x5f == 0 || m.rng.rand() & 3 == 0 { random(m) } else { Noise::Set(255, 255, 255) };
                    noiz(m, x, n);
                    x.req.push(Request::CameraShake { power: 2, cycle: 1, time: 30, dirc: 2 });
                }
                if count & 7 == 0 {
                    x.se(SE_QUAKE);
                }
                return;
            }
            _ => {}
        }
    }
    let free = m.menu_status == 0 && m.check_menu_type() == -1 && m.forbid == 0 && x.world.event_status == 0;
    if x.world.pg_ride || free {
        if x.count & 0x3f == 0 && m.rng.rand() & 3 == 0 {
            let n = m.rng.rand() % 9 + 4;
            noiz(m, x, Noise::Set(n, n, n));
        }
    } else if m.menu != 66 && m.menu != 62 {
        noiz(m, x, Noise::Set(0, 0, 0));
    }
}

/// Level 5's rumble (`ccSeOn(258)` every 8th frame).
const SE_QUAKE: i32 = 258;

/// `saveData.erosion` (+0x676e): the bracelet's infection, 0-100.
pub const EROSION: usize = 0x676e;

/// The bracelet's gauge (0x0051dcf4 - 0x0051e788): its frame, its bar, at
/// erosion 60 on the pulsing warning (another cell from 80), and the four
/// Gouraud cells of the virus, their corners between `virusCol`'s two
/// colour pairs by a 49-frame swing.
fn drain_gauge(m: &mut MenuCtrl, x: &mut Ctx) {
    let a = m.drain_alpha;
    let erosion = i32::from(x.save.save.i16(EROSION));
    let flat = |i: usize| {
        let c = SPRITE_COLOR_TABLE[i];
        Some([[c[0], c[1], c[2]]; 4])
    };
    let d = &mut m.drain;
    d.set_grid(48, 144, 144.0, 208.0, 1280, 0, 1);
    d.set_colour(0);
    d.vcol = flat(0);
    d.set_alpha(a);
    d.dx = 344.0;
    d.dy = 56.0;
    d.make_packet(0);
    d.set_grid(128, 32, 128.0, 32.0, 0, 2304, 1);
    d.set_colour(2);
    d.vcol = flat(2);
    d.set_alpha(a);
    d.dx = 352.0;
    d.dy = 64.0;
    d.make_packet(0);
    if erosion >= 60 {
        let mut v = (x.count % 25) as i32;
        if v >= 13 {
            v = 24 - v;
        }
        d.sx = 112.0;
        d.sy = 32.0;
        d.su = 112;
        d.sv = 32;
        d.cx = -56.0;
        d.cy = -16.0;
        let f = add(1.0, div(from_int(v), 40.0));
        d.sx = mul(d.sx, f);
        d.sy = mul(d.sy, f);
        d.cx = mul(d.cx, f);
        d.cy = mul(d.cy, f);
        d.dx = 416.0;
        d.dy = 280.0;
        d.set_colour(7);
        d.vcol = flat(7);
        d.set_alpha(a / 3 + (2 * a * v) / 36);
        d.wu = 2048;
        d.wv = if erosion < 80 { 1280 } else { 1792 };
        d.wi = 1;
        d.make_packet(0);
        d.cx = 0.0;
        d.cy = 0.0;
    }
    d.set_grid(40, 72, 40.0, 72.0, 0, 0, 2);
    let mut v = (x.count % 49) as i32;
    if v >= 25 {
        v = 48 - v;
    }
    let row = x.texts.virus_col.get((erosion / 17).clamp(0, 5) as usize).copied().unwrap_or([0; 12]);
    let lerp = |i: usize| row[6 + i] + ((row[i] - row[6 + i]) * v) / 24;
    // r | g << 8 | b << 16 as the code ORs them, each sign-extended.
    let word = |r: i32, g: i32, b: i32| {
        let w = i64::from(r) | (i64::from(g) << 8) | (i64::from(b) << 16);
        [w as u8, (w >> 8) as u8, (w >> 16) as u8]
    };
    let xc = word(lerp(0), lerp(1), lerp(2));
    let yc = word(lerp(3), lerp(4), lerp(5));
    for (code, dx, dy, corner) in
        [(0, 376.0, 112.0, 3), (1, 416.0, 112.0, 1), (2, 376.0, 184.0, 2), (3, 416.0, 184.0, 0)]
    {
        let mut c = [yc; 4];
        c[corner] = xc;
        d.rgb = c[0];
        d.vcol = Some(c);
        d.set_alpha(a);
        d.dx = dx;
        d.dy = dy;
        d.make_packet(code);
    }
    d.vcol = None;
}

/// The list's window and rows (0x0051d2e4 - 0x0051d9ac).
fn list_window(m: &mut MenuCtrl, x: &mut Ctx, idx: usize, target: Option<&CharInfo>) {
    let l = m.lists[idx].clone();
    let title = if l.title.is_empty() { None } else { Some(&l.title[..]) };
    // Where the rows go (x, y) and the cursor's width.
    let (mut rx, mut ry, mut w) = (0i32, 0i32, 0i32);
    match l.disp {
        7 | 8 => {
            m.win.dx = 39.0;
            m.win.dy = 96.0;
            (rx, ry, w) = (53, 112, i32::from(l.x));
            if l.y < l.my {
                disp_square_sb(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.dy), i32::from(l.my), title);
            } else {
                disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), title);
            }
        }
        6 => {
            let mut h = i32::from(l.y);
            ry = 130;
            m.win.dx = 39.0;
            m.win.dy = 114.0;
            if let Some(t) = target {
                if t.is(7) {
                    m.win.dy = 129.0;
                    h = i32::from(l.y) + 1;
                    ry = 169;
                } else if t.is(0xe0) {
                    m.win.dy = 129.0;
                    ry = 149;
                }
            }
            if l.y < l.my {
                disp_square_sb(&mut m.win, i32::from(l.x), h, i32::from(l.dy), i32::from(l.my), None);
            } else {
                disp_square(&mut m.win, i32::from(l.x), h, None);
            }
            (rx, w) = (53, i32::from(l.x));
        }
        9 => {
            m.win.dx = 39.0;
            m.win.dy = 96.0;
            disp_square_tag(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.page), i32::from(l.page_num), 8);
            (rx, ry, w) = (39, 112, i32::from(l.x));
            if l.y < l.my {
                m.win.dx = 39.0;
                m.win.dy = 96.0;
                window::disp_scroll_bar(&mut m.win, i32::from(l.x), i32::from(l.y), i32::from(l.dy), i32::from(l.my));
            }
            m.win.dx = 55.0;
            m.win.dy = 64.0;
            disp_button(&mut m.win, 4, x.count, x.frame_rate);
            m.win.dx = from_int((i32::from(l.x) + 2) * 14 + 23);
            m.win.dy = 64.0;
            disp_button(&mut m.win, 6, x.count, x.frame_rate);
            m.win.set_colour(7);
            m.win.set_alpha(m.alpha);
            m.win.cx = 0.0;
            m.win.cy = 0.0;
            set_clm(&mut m.kanji, 16, 0, 0, 1);
            m.kanji.set_colour(7);
            m.kanji.set_alpha(m.kanji_alpha);
            m.kanji.dx = from_int((i32::from(l.page) + 2) * 14 + 39);
            m.kanji.dy = 84.0;
            m.kanji.make_packet(0);
            let a = m.alpha;
            let fr = x.frame_rate;
            m.cursor.disp(
                &mut m.win,
                39.0,
                from_int(i32::from(l.select) * 20 + 112),
                w,
                i32::from(l.select) + (i32::from(l.page) << 8),
                a,
                3,
                fr,
            );
        }
        11 => {
            let wx = 256 - 7 * (i32::from(l.x) + 2);
            m.win.dx = from_int(wx);
            m.win.dy = 248.0;
            disp_square(&mut m.win, i32::from(l.x), i32::from(l.y), None);
            (rx, ry, w) = (wx + 14, 264, i32::from(l.x));
        }
        _ => {}
    }
    if l.disp == 4 || l.disp == 9 {
        return;
    }
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    let mut sel = i32::from(l.select);
    if l.disp != 11 && l.y < l.my {
        sel -= i32::from(l.dy);
    }
    let a = m.alpha;
    let fr = x.frame_rate;
    for r in 0..i32::from(l.y) {
        if sel == r {
            m.cursor.disp(&mut m.win, from_int(rx - 14), from_int(ry + 20 * r), w, i32::from(l.select), a, 3, fr);
        }
        if m.reverse_head & (1 << r) != 0 {
            m.kanji.set_colour(0);
        } else {
            m.kanji.set_colour(7);
        }
        m.kanji.set_alpha(m.kanji_alpha);
        let kx = if l.disp == 8 || l.disp == 10 { rx + 14 } else { rx };
        m.kanji.dx = from_int(kx);
        m.kanji.dy = from_int(ry + 20 * r);
        m.kanji.make_packet(r);
    }
}

/// The name of a party member for texts: `spcParam[pc].base.name`.
pub fn member_name(x: &Ctx, pc: i32) -> Vec<u8> {
    if pc == 0 {
        return x.save.save.name().to_vec();
    }
    x.texts.char_names.get(pc.max(0) as usize).cloned().unwrap_or_default()
}

/// nameKanji's text: the target's name, its action, the party's names,
/// then the town's face name or a foe's immunity ("Fire Tol."), then "You
/// have new mail." (0x0051e78c - 0x0051eca0), extracted into nameKanji
/// and nameKanjiPr every frame.
fn name_texts(m: &mut MenuCtrl, x: &mut Ctx, target: Option<&CharInfo>) {
    let mut buf = Vec::new();
    match target {
        Some(t) => str_cat(&mut buf, &t.name, 16),
        None => str_cat(&mut buf, b"", 16),
    }
    match target {
        Some(t) => {
            let h = if t.is(0x0f00_1f1f) {
                0
            } else if t.is(0xe0) {
                1
            } else if t.is(0x3_0000) {
                3
            } else {
                2
            };
            let s = x.texts.help.get(h).cloned().unwrap_or_default();
            str_cat(&mut buf, &s, 16);
        }
        None => str_cat(&mut buf, b"", 16),
    }
    for p in &x.world.party {
        match p {
            Some(c) => str_cat(&mut buf, &c.name, 16),
            None => str_cat(&mut buf, b"", 16),
        }
    }
    if x.world.game.area == 0 {
        let n = member_name(x, i32::from(m.face_num));
        str_cat(&mut buf, &n, 16);
    } else {
        let piece = target.and_then(CharInfo::tolerance).and_then(|(k, _)| x.texts.kyvia_status.get(k));
        str_cat(&mut buf, piece.map_or(&b""[..], |p| p), 16);
    }
    buf.extend_from_slice(&x.texts.new_mail);
    let differs = buf != m.name_text;
    if differs {
        m.note(|| format!("[\"extract\",\"name\",\"{}\"]", String::from_utf8_lossy(&buf)));
    }
    m.name_text = buf.clone();
    m.name_pr_text = buf;
    set_clm(&mut m.name, 16, 0, 0, 1);
    set_clm(&mut m.name_pr, 16, 0, 0, 1);
}

/// The target cursor and targetAlpha (0x0051ecbc - 0x0051f610); every
/// path ends with subTarget cleared.
fn target_cursor(m: &mut MenuCtrl, x: &mut Ctx, target: Option<&CharInfo>) {
    let Some(t) = target.filter(|_| m.target_forbid == 0) else {
        m.target_alpha = 0;
        m.cursol_target = 0;
        m.sub_target = [0; 16];
        return;
    };
    let is_cmnd = x.target.as_ref().is_some_and(|c| c.handle == t.handle)
        || m.dummy_target.as_ref().is_some_and(|c| c.handle == t.handle);
    if !is_cmnd {
        m.target_alpha -= 28;
        if m.target_alpha < 0 {
            m.target_alpha = 0;
        }
        m.cursol_target = 0;
        m.sub_target = [0; 16];
        return;
    }
    if t.is(0x0fbf_dfff) && m.cursol_off == 0 {
        let enemy = t.is(0xe0);
        if enemy {
            m.target.set_colour(2);
        } else if t.is(0x0f00_1f1f) {
            m.target.set_colour(17);
        } else {
            m.target.set_colour(6);
        }
        if m.pl_attack != 0 && enemy && m.check_menu_type() == -1 {
            m.cursol_alpha = (m.cursol_alpha + 2) & 31;
        } else {
            m.cursol_alpha = (m.cursol_alpha + 1) & 31;
        }
        let cmnd = x.target.as_ref().map_or(0, |c| c.handle);
        if m.cursol_target != cmnd {
            m.cursol_target = cmnd;
            m.cursol_alpha = 0;
            m.cursol_init = 1;
        } else if m.cursol_init != 0 && m.cursol_alpha == 0 {
            m.cursol_init = 0;
        }
        let ca = m.cursol_alpha;
        let s4 = 31 - ca;
        let a = if ca < 8 {
            ca << 4
        } else if ca >= 12 {
            s4 * 6
        } else {
            128
        };
        m.target.set_alpha(a);
        let mut f20 = from_int(ca);
        f20 = if m.cursol_init != 0 && ca < 8 { mul(sub(9.0, f20), 10.0) } else { add(div(sub(32.0, f20), 3.0), 8.0) };
        f20 = div(f20, 16.0);
        if let Some((mut px, mut py)) = t.tag {
            px = px.clamp(16, 496);
            py = py.clamp(16, 440);
            let c = &mut m.target;
            c.set_grid(14, 16, 14.0, 16.0, 1584, 2048, 4);
            c.cx = 0.0;
            c.cy = 0.0;
            c.rot = 0.0;
            for (code, ox, oy) in [(3, px - 14 - s4, py - 16 - s4), (0, px + s4, py - 16 - s4)] {
                c.dx = from_int(ox);
                c.dy = from_int(oy);
                c.make_packet(code);
            }
            for (code, ox, oy) in [(2, px - 14 - s4, py + s4), (1, px + s4, py + s4)] {
                c.dx = from_int(ox);
                c.dy = from_int(oy);
                c.make_packet(code);
            }
            if m.pl_attack != 0 && enemy && m.check_menu_type() == -1 {
                let c = &mut m.target;
                c.set_grid(72, 32, 72.0, 32.0, 2912, 2048, 1);
                c.cx = -36.0;
                c.cy = -16.0;
                c.rot = 0.0;
                // The pulse again from the diamond's scale (`$f20`), not
                // from cursolAlpha (0x0051f21c): it barely pulses.
                let s = div(add(div(sub(32.0, f20), 3.5), 8.0), 16.0);
                c.sx = mul(c.sx, s);
                c.sy = mul(c.sy, s);
                c.cx = mul(c.cx, s);
                c.cy = mul(c.cy, s);
                c.dx = from_int(px);
                c.dy = from_int(py);
                c.make_packet(0);
                f20 = s;
            } else {
                let c = &mut m.target;
                c.set_grid(24, 24, 32.0, 32.0, 2464, 2048, 1);
                c.cx = -16.0;
                c.cy = -16.0;
                c.rot = std::f32::consts::FRAC_PI_4;
                c.sx = mul(c.sx, f20);
                c.sy = mul(c.sy, f20);
                c.cx = mul(c.cx, f20);
                c.cy = mul(c.cy, f20);
                c.dx = from_int(px);
                c.dy = from_int(py);
                c.make_packet(0);
            }
        }
        // An area skill's other targets (subTarget, filled each frame by
        // TargetMenu or ChatMenu3): the diamond's cell square, smaller,
        // at each one's tag point (0.45 of its height), x held to 0..512
        // and y to 16..448 (0x0051f3d8 - 0x0051f588).
        let c = &mut m.target;
        c.set_grid(24, 24, 20.0, 20.0, 2464, 2048, 1);
        c.cx = -10.0;
        c.cy = -10.0;
        c.rot = 0.0;
        c.sx = mul(c.sx, f20);
        c.sy = mul(c.sy, f20);
        c.cx = mul(c.cx, f20);
        c.cy = mul(c.cy, f20);
        let w = &x.world;
        let chains = || w.pc_chain.iter().chain(&w.ene_chain).chain(&w.obj_chain);
        for h in m.sub_target.into_iter().filter(|&h| h != 0) {
            let Some((px, py)) = chains().find(|c| c.handle == h).and_then(|c| c.tag) else { continue };
            c.dx = from_int(px.clamp(0, 512));
            c.dy = from_int(py.clamp(16, 448));
            c.make_packet(0);
        }
    }
    m.target_alpha += 24;
    if m.target_alpha >= 129 {
        m.target_alpha = 128;
    }
    m.sub_target = [0; 16];
}

/// The target's window: its frame and name, the action button, its HP
/// and SP, condition icons (0x0051f614 - 0x0052063c).
fn target_window(m: &mut MenuCtrl, x: &mut Ctx, t: &CharInfo) {
    let ta = m.target_alpha;
    m.win.set_colour(7);
    m.win.set_alpha(ta);
    m.win.dx = 35.0;
    m.win.dy = 96.0;
    let names = x.save.names();
    let sn = piney_desktop::message::str_len(&t.name, &names);
    disp_target(&mut m.win, sn, 0);
    m.name.set_colour(7);
    m.name.set_alpha(ta);
    m.name.dx = 60.0;
    m.name.dy = 102.0;
    m.name.make_packet(0);
    if m.check_menu_type() == -1 && m.pl_attack == 0 && x.world.player_skill == 0 && !x.world.player_attacking {
        if t.is(6) {
            (m.win.dx, m.win.dy, m.name.dx, m.name.dy) = (47.0, 176.0, 59.0, 170.0);
        } else if t.is(0xe0) {
            (m.win.dx, m.win.dy, m.name.dx, m.name.dy) = (47.0, 156.0, 59.0, 150.0);
        } else {
            (m.win.dx, m.win.dy, m.name.dx, m.name.dy) = (47.0, 132.0, 59.0, 126.0);
        }
        let act = x.assign(0);
        let b = if act & 0x10 != 0 {
            1
        } else if act & 0x80 != 0 {
            2
        } else if act & 0x40 != 0 {
            3
        } else {
            0
        };
        disp_button(&mut m.win, b, x.count, x.frame_rate);
        m.name.make_packet(1);
    }
    if !t.is(0xe7) {
        return;
    }
    // HP.
    m.win.set_colour(7);
    m.win.set_alpha(ta);
    m.win.set_grid(84, 16, 84.0, 16.0, 0, 2048, 1);
    m.win.dx = 59.0;
    m.win.dy = 120.0;
    m.win.make_packet(0);
    let hp = i32::from(t.hp);
    let mhp = i32::from(t.max_hp).max(1);
    bars(&mut m.win, hp, mhp, 124.0, [48, 128, 80], ta);
    font_type(&mut m.font, 2);
    m.font.set_colour(22);
    m.font.set_alpha(ta);
    if mhp >= 10000 {
        let d = |v: i32| -> Vec<u8> {
            let v = v % 10000;
            let digits = [v / 1000, (v % 1000) / 100, (v % 100) / 10, v % 10];
            digits.iter().map(|&k| x.texts.cheat_hp.get(k as usize).copied().unwrap_or(b'0')).collect()
        };
        m.font.dx = 55.0;
        m.font.dy = 132.0;
        m.font.make_str(&d(hp));
        m.font.make_str(&x.texts.slash);
        m.font.make_str(&d(i32::from(t.max_hp)));
    } else {
        let n = digits(mhp);
        m.font.dx = from_int(75 - (n - 2) * 10);
        m.font.dy = 132.0;
        make_num(&mut m.font, n, hp);
        m.font.make_str(&x.texts.slash);
        make_num(&mut m.font, n, i32::from(t.max_hp));
    }
    if t.is(7) {
        m.win.set_colour(7);
        m.win.set_alpha(ta);
        m.win.set_grid(84, 16, 84.0, 16.0, 0, 2048, 1);
        m.win.dx = 59.0;
        m.win.dy = 142.0;
        m.win.make_packet(1);
        let sp = i32::from(t.sp);
        let msp = i32::from(t.max_sp).max(1);
        bars(&mut m.win, sp, msp, 146.0, [48, 80, 128], ta);
        let n = digits(msp);
        m.font.dx = from_int(75 - (n - 2) * 10);
        m.font.dy = 154.0;
        make_num(&mut m.font, n, sp);
        m.font.make_str(&x.texts.slash);
        make_num(&mut m.font, n, i32::from(t.max_sp));
    } else if t.is(0xe0) && t.attribute >= 0 {
        let c = &mut m.item_icon;
        c.set_grid(18, 20, 18.0, 20.0, 896, 0x1500, 4);
        c.set_colour(7);
        c.set_alpha(ta);
        c.dx = 37.0;
        c.dy = 122.0;
        c.make_packet(t.attribute - 2);
    }
    m.win.set_colour(7);
    crate::panel::condition_icons(m, t, 39.0, 63.0, ta);
    if x.world.game.area != 0 && t.is(0xe0) && t.exdefense != 0 {
        tolerance(m, t, ta);
    }
}

/// A foe's immunity right of its HP (0x005202e4 - 0x0052063c): nameKanji's
/// sixth line in colour 23, pulsing with the cursor, and struck through
/// (a bar the text's width) once the defence is lowered.
fn tolerance(m: &mut MenuCtrl, t: &CharInfo, ta: i32) {
    let c = m.cursol_alpha;
    let pulse = match c {
        ..4 => c << 5,
        24.. => (31 - c) << 4,
        _ => 128,
    };
    m.name.set_colour(23);
    m.name.set_alpha(pulse.min(ta));
    m.name.dx = 145.0;
    m.name.dy = 122.0;
    m.name.make_packet(5);
    let Some((k, true)) = t.tolerance() else { return };
    // The pieces' widths: "Physical Tol." 88 ... "Darkness Tol." 94.
    const WIDTH: [f32; 8] = [88.0, 68.0, 72.0, 72.0, 60.0, 64.0, 86.0, 94.0];
    let s = &mut m.item_icon;
    s.set_grid(72, 16, WIDTH[k], 16.0, 0, 0x1800, 1);
    s.set_colour(7);
    s.set_alpha(ta);
    s.dx = 145.0;
    s.dy = 122.0;
    s.make_packet(0);
}

/// The digits of a gauge's maximum: 4 from 1000, 3 from 100, else 2.
fn digits(max: i32) -> i32 {
    if max >= 1000 {
        4
    } else if max >= 100 {
        3
    } else {
        2
    }
}

/// A gauge's fill and its empty rest: 56 units wide at (85, y).
fn bars(s: &mut Spr, now: i32, max: i32, y: f32, rgb: [u8; 3], a: i32) {
    s.set_grid(8, 8, 0.0, 8.0, 1344, 2048, 1);
    let w = (now * 56) / max;
    s.sx = from_int(w);
    s.sy = 8.0;
    s.dx = 85.0;
    s.dy = y;
    s.set_rgb_u32(u32::from(rgb[0]) | (u32::from(rgb[1]) << 8) | (u32::from(rgb[2]) << 16));
    s.set_alpha(a);
    s.make_packet(0);
    if w != 56 {
        s.sx = from_int(56 - w);
        s.sy = 8.0;
        s.set_colour(18);
        s.set_alpha(a);
        s.make_packet(0);
    }
}

/// The party panels (0x00520640 - 0x00520ad0).
fn panels(m: &mut MenuCtrl, x: &mut Ctx) {
    match m.panel_status {
        0 => m.panel_alpha = 0,
        1 => {
            m.panel_alpha += 24;
            if m.panel_alpha >= 128 {
                m.panel_alpha = 128;
                m.panel_status = 2;
            }
        }
        2 => m.panel_alpha = 128,
        3 => {
            m.panel_alpha -= 28;
            if m.panel_alpha <= 0 {
                m.panel_alpha = 0;
                m.panel_status = 0;
            }
        }
        _ => {}
    }
    for i in 0..3 {
        if x.world.party[i].is_none() {
            continue;
        }
        m.panel_bure[i] = m.panel_bure[i].saturating_sub(1);
        m.panel_flash[i] = m.panel_flash[i].saturating_sub(1);
    }
    if m.panel_alpha <= 0 {
        return;
    }
    let pa = m.panel_alpha;
    m.name.set_colour(7);
    m.name.set_alpha(pa);
    m.name_pr.set_colour(7);
    m.name_pr.set_alpha(pa);
    font_type(&mut m.font, 2);
    let mut beeped = false;
    for i in 0..3usize {
        let Some(c) = x.world.party[i].clone() else { continue };
        let mut fcol: u32 = 0x8080_8080;
        if c.condition[0] != 0 {
            // Down: the face goes dim, from its colour's green.
            let g = i32::from(m.faces[i].rgb[1]) - 16;
            let v = g.max(16) as u32;
            fcol = (v << 8) | 128 | (v << 16);
        } else if now_max_colour(i32::from(c.hp), i32::from(c.max_hp)) == 2 {
            let mut k = (x.count % 28) as i32;
            if k == 0 && !beeped {
                x.se(95);
                beeped = true;
            }
            if k >= 14 {
                k = 28 - k;
            }
            let v = ((k * 112) / 14 + 16) as u32;
            fcol = (v << 8) | 128 | (v << 16);
        }
        let b = i32::from(m.panel_bure[i]);
        let mut shake = 0;
        if b != 0 {
            shake = b.min(16);
            if b & 1 != 0 {
                shake = -shake;
            }
        }
        let fl = m.panel_flash[i] as usize;
        let flash = if fl != 0 { x.texts.panel_flash.get(fl).copied().unwrap_or(0) } else { 0 };
        let px = from_int(170 * i as i32);
        let py = from_int(shake + 352);
        crate::panel::face_panel(m, x, px, py, pa, i, &c, fcol, flash);
        crate::panel::condition_icons(m, &c, from_int(170 * i as i32 + 12), from_int(shake + 320), pa);
    }
}

/// The enemies' life bars over them, or "ENEMY" and an arrow at the
/// screen's edge (0x00520ae4 - 0x00521440).
fn enemy_bars(m: &mut MenuCtrl, x: &mut Ctx) {
    let fly = &mut m.fly;
    font_type(fly, 2);
    fly.set_colour(2);
    let mut v = (x.count % 30) as i32;
    if v >= 16 {
        v = 30 - v;
    }
    fly.set_alpha((v * 80) / 15 + 80);
    if m.target_forbid != 0 {
        return;
    }
    let list: Vec<CharInfo> =
        x.world.sorted.iter().filter(|c| c.is(0xe0) && c.condition[0] == 0).take(12).cloned().collect();
    for c in list.iter().rev() {
        if c.bar_res == 1 {
            let (px, py) = (c.bar.0.clamp(26, 486), c.bar.1.clamp(90, 436));
            let d = c.cmnd_dist;
            let s4 = if d < 400.0 {
                128
            } else if d >= 2200.0 {
                16
            } else {
                to_int(mul(112.0, div(sub(2200.0, d), 1800.0))) + 16
            };
            let hp = i32::from(c.hp);
            let mhp = i32::from(c.max_hp).max(1);
            let w = (hp * 50) / mhp;
            let a = (s4 * 4) / 5;
            let e = &mut m.ene_life;
            e.set_grid(8, 8, 0.0, 6.0, 1344, 2048, 1);
            e.sx = from_int(w);
            e.sy = 6.0;
            e.dx = from_int(px - 20);
            e.dy = from_int(py);
            e.set_rgb_u32(32 | (128 << 8) | (64 << 16));
            e.set_alpha(a);
            e.make_packet(0);
            if w != 50 {
                e.sx = from_int(50 - w);
                e.sy = 6.0;
                e.set_colour(18);
                e.set_alpha(a);
                e.make_packet(0);
            }
            if c.pp != 0 && c.pp < 240 {
                let py2 = c.bar.1.clamp(94, 436);
                let p = &mut m.protect_spr;
                p.set_grid(128, 40, 96.0, 32.0, 0, 3072, 1);
                p.set_colour(7);
                p.set_alpha(s4);
                p.dx = from_int(c.bar.0 - 48);
                p.dy = from_int(py2 + 8);
                p.make_packet(0);
            }
            continue;
        }
        let (mut px, mut py) = if c.bar_res == 0 { c.arrow } else { c.bar };
        px = px.clamp(36, 476);
        py = py.clamp(4, 336);
        let fly = &mut m.fly;
        fly.dx = from_int(px - 25);
        fly.dy = from_int(py);
        let enemy = x.texts.enemy.clone();
        fly.make_str(&enemy);
        let e = &mut m.ene_life;
        e.set_grid(24, 24, 68.0, 24.0, 2464, 2048, 1);
        e.dx = from_int(px - 33);
        e.dy = from_int(py - 6);
        e.set_colour(2);
        e.set_alpha(80);
        e.make_packet(0);
    }
}

/// The protect marks over characters `SetProtect` named (0x00521448 -
/// 0x0052170c).
fn protect_marks(m: &mut MenuCtrl, x: &mut Ctx) {
    for k in 0..12 {
        if m.protect_cnt[k] == 0 {
            continue;
        }
        m.protect_cnt[k] -= 1;
        if m.forbid != 0 {
            continue;
        }
        let who = m.protect_char[k];
        let found = x.world.sorted.iter().chain(x.world.party.iter().flatten()).find(|c| c.handle == who).cloned();
        let Some(c) = found.filter(|c| c.condition[0] == 0) else {
            m.protect_cnt[k] = 0;
            continue;
        };
        if c.bar_res != 1 {
            continue;
        }
        let d = c.cmnd_dist;
        let a = if d < 400.0 {
            128
        } else if d >= 2200.0 {
            16
        } else {
            to_int(mul(112.0, div(sub(2200.0, d), 1800.0))) + 16
        };
        let py = c.bar.1.clamp(94, 436);
        let p = &mut m.protect_spr;
        p.set_grid(128, 40, 128.0, 40.0, 2048, 0, 1);
        p.set_alpha(a);
        p.dx = from_int(c.bar.0 - 64);
        p.dy = from_int(py + 8);
        p.make_packet(i32::from(m.protect[k]));
    }
}

/// The battle banner as a fight starts, and the band while one waits
/// (0x00521754 - 0x00521dc0).
fn banner(m: &mut MenuCtrl, x: &mut Ctx, blink: i32) {
    let g = x.world.game;
    if g.in_battle == 0 || m.forbid != 0 || x.world.event_status != 0 || x.world.boss_entry >= 0 {
        return;
    }
    let c = g.in_battle_cnt;
    if c == 0 {
        if g.in_battle != 1 {
            return;
        }
        let e = &mut m.ene_life;
        e.set_colour(2);
        e.set_alpha(blink);
        e.set_grid(192, 32, 192.0, 32.0, 0, 2560, 1);
        e.cx = -92.0;
        e.cy = -16.0;
        let bc = from_int(i32::from(m.battle_cnt));
        for base in [128.0, 384.0, 640.0] {
            e.dx = sub(base, div(mul(256.0, bc), 48.0));
            e.dy = 16.0;
            e.make_packet(0);
        }
        e.set_colour(7);
        e.cx = 0.0;
        e.cy = 0.0;
        return;
    }
    if c < 16 {
        return;
    }
    let s4 = if c >= 46 {
        ((60 - c) * 96) / 15
    } else if c >= 26 {
        96
    } else {
        ((c - 15) * 96) / 10
    };
    let mut f1 = 0.0f32;
    if c >= 46 {
        let mut f2 = 0.5f32;
        for _ in 0..(c - 45) {
            f2 = add(f2, div(f2, 2.75));
            f1 = add(f1, f2);
        }
    }
    let e = &mut m.ene_life;
    // Not the game's: the announcement shrinks about the screen's centre.
    e.anchor = Anchor::CENTRE;
    let mut code = 0;
    if g.in_battle == 1 {
        e.set_colour(2);
    } else if g.in_battle == 2 {
        e.set_colour(17);
        code = 1;
    }
    e.set_alpha(s4);
    e.set_grid(144, 32, 144.0, 32.0, 0, 2560, 1);
    e.cx = -92.0;
    e.cy = -8.0;
    e.sx = mul(e.sx, 1.8);
    e.sy = mul(e.sy, 2.0);
    e.cx = mul(e.cx, 1.8);
    e.cy = mul(e.cy, 2.0);
    let x0 = sub(256.0, f1);
    e.dx = x0;
    e.dy = 176.0;
    e.make_packet(0);
    e.set_grid(48, 32, 48.0, 32.0, 2304, 2560, 2);
    e.cx = 52.0;
    e.cy = -8.0;
    e.sx = mul(e.sx, 1.8);
    e.sy = mul(e.sy, 2.0);
    e.cx = mul(e.cx, 1.8);
    e.cy = mul(e.cy, 2.0);
    e.dx = x0;
    e.dy = 176.0;
    e.make_packet(code);
    e.set_colour(7);
    e.cx = 0.0;
    e.cy = 0.0;
}

/// "You have new mail." in a town, 240 frames after arriving with unread
/// mail, with sound 3 (0x00521dc4 - 0x00522054).
fn new_mail(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.check_menu_type() != -1 || x.world.game.area != 0 || m.forbid != 0 {
        return;
    }
    let any = (0..512).any(|i| {
        let s = x.save.save.mail(i);
        s == 1 || s == 2
    });
    if !any {
        return;
    }
    if m.mail_cnt != 0 {
        m.mail_cnt -= 1;
        if m.mail_cnt == 0 {
            x.se(3);
        }
    }
    if m.mail_cnt != 0 {
        return;
    }
    set_type(&mut m.win, 1);
    m.win.set_colour(5);
    let mut a0 = (x.count % 30) as i32;
    if a0 >= 15 {
        a0 = 30 - a0;
    }
    let a = (a0 * 96) / 15 + 32;
    m.win.set_alpha(a);
    m.win.sx = 14.0;
    m.win.sy = 16.0;
    m.win.dx = 16.0;
    m.win.dy = 16.0;
    m.win.make_packet(30);
    m.win.make_packet(31);
    m.name.set_colour(5);
    m.name.set_alpha(a);
    m.name.dx = 44.0;
    m.name.dy = 16.0;
    m.name.make_packet(6);
    m.name.dx = 164.0;
    m.name.dy = 16.0;
    m.name.make_packet(7);
}

/// `ExceptionDisp` needs this.
pub fn table(i: usize) -> u32 {
    table_u32(i)
}
