//! `MailList_control` (DESKTOP.PRG, mailer.cpp; constructor 0x004085a0): the
//! inbox, reading a mail, and replying (`docs/engine/desktop.md`, "Mail"). Its
//! layout constants are the class's own: the list frame `laysize` (167, 70,
//! 318 x 250), the scroll bar `scrsize`, the reply window `reLaysize` (90,
//! 270, 224 x 80) and the info window `ressize` (224 x 72). Texts are
//! `ccKanji` `Init(3, 16)` in `ccSpriteColorTable[7]`; icons, cursors and
//! window tiles are cells of `TEX_xddcurs1` (64 x 64).

use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_data::tables::desktop;
use piney_data::tables::sjis::encode;
use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::anm::{Anm, Ctx, Instance, draw_model};
use crate::assets::{Assets, SceneFile};
use crate::content::{self, Mail, ReMail};
use crate::kanji::{Fonts, Kanji, Names};
use crate::layers::{KANJI_LAYER, RES_LAYER, RESK_LAYER};
use crate::save::{MAIL_SLOTS, MailState, SaveState};
use crate::sprite::Sprite;
use crate::view::LayerView;
use crate::{Request, Se};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

/// `Numremit`: rows the list and a body show.
pub const ROWS: i32 = 14;
/// `shiftRemit`, `UpRemit`: where the list starts scrolling down / up.
pub const SHIFT_REMIT: i32 = 9;
pub const UP_REMIT: i32 = 4;
/// Row pitch and first row of the list and a body.
pub const ROW_H: i32 = 17;
pub const ROW_Y: i32 = 48;
/// `laysize` (x, y, hi, wid): the list's frame on the kanji layer.
pub const LAYSIZE: [f32; 4] = [167.0, 70.0, 250.0, 318.0];
/// `scrsize` (x, y, hi, wid): the scroll bar's travel and width.
pub const SCRSIZE: [f32; 4] = [5.0, 0.0, 224.0, 4.0];
/// `ressize` (x, y, hi, wid): the info windows' size.
pub const RESSIZE: [f32; 4] = [90.0, 230.0, 72.0, 224.0];
/// `reLaysize` (x, y, hi, wid): the reply window's frame.
pub const RELAYSIZE: [f32; 4] = [90.0, 270.0, 80.0, 224.0];
/// `ccView::SetFrame`'s `ay` for every mailer frame: 6/7, square pixels.
pub const AY_SQUARE: f32 = 6.0 / 7.0;
/// Where `InfoWindow` and `InfoWindowEnd` put their window.
pub const INFO_POS: [f32; 2] = [160.0, 50.0];
pub const INFO_END_POS: [f32; 2] = [200.0, 150.0];
/// `CurRepeat`: frames of repeat input before the first repeat, then between.
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
/// The unread icon's pulse (`IconScale` in `WriteTitle`): step and top.
pub const ICON_SCALE_STEP: u32 = 0x3c44_9ba6;
pub const ICON_SCALE_TOP: u32 = 0x3fa6_6666;
/// `ANM_xddphot0`, the photo frame, and the object whose position and
/// transparency the sender's photo takes.
pub const PHOT_ANM: &str = "ANM_xddphot0";
pub const PHOT_OBJ: &str = "OBJ_xddphoto";

/// The cells of `TEX_xddcurs1` (wu, wv in 1/16 texels; w, h).
pub mod cell {
    pub const CURSOR_BIG: (i32, i32, i32, i32) = (208, 0, 14, 11);
    pub const CURSOR_SMALL: (i32, i32, i32, i32) = (0, 0, 13, 10);
    pub const UNREAD: (i32, i32, i32, i32) = (0, 208, 13, 10);
    pub const READ: (i32, i32, i32, i32) = (192, 208, 13, 10);
    pub const SCROLL: (i32, i32, i32, i32) = (0, 368, 7, 7);
    /// Window tiles: wv of the top, middle and bottom rows; wu of the
    /// left, middle and right columns.
    pub const WIN_ROWS: [i32; 3] = [496, 624, 752];
    pub const WIN_COLS: [i32; 3] = [0, 128, 256];
}

/// The mailer's strings, read once.
struct Strings {
    yes: Vec<u8>,
    no: Vec<u8>,
    nores: Vec<u8>,
    send_comp: Vec<u8>,
    sel_remail: Vec<u8>,
    send_mail: Vec<u8>,
    newmail: Vec<u8>,
    mail: Vec<Vec<u8>>,
}

impl Strings {
    /// The mailer's `STR_` texts, alike on the four volumes
    /// (`piney_data::tables::desktop`'s `MAIL_`); `mail` is "You have ",
    /// " new mail.", "No new mail.", " new mails.".
    fn new() -> Self {
        Strings {
            yes: encode(*desktop::MAIL_YES),
            no: encode(*desktop::MAIL_NO),
            nores: encode(*desktop::MAIL_NORES),
            send_comp: encode(*desktop::MAIL_SEND_COMP),
            sel_remail: encode(*desktop::MAIL_SEL_REMAIL),
            send_mail: encode(*desktop::MAIL_SEND_MAIL),
            newmail: encode(*desktop::MAIL_NEWMAIL),
            mail: desktop::MAIL_COUNT.iter().map(|s| encode(s)).collect(),
        }
    }
}

/// A mail's `reFlg` and `mailFlg` as the loaded table holds them: the file
/// has every `mailFlg` 0; the desktop sets them as mails are read and
/// replied to, and the table is fresh each time the overlay loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flags {
    re_flg: i32,
    mail_flg: i32,
}

/// The pad as the mailer tests it.
#[derive(Clone, Copy)]
struct Keys {
    direct: u32,
    push: u32,
    unpush: u32,
    repeat: u32,
    ok: u32,
    cancel: u32,
}

/// `MailList_control`.
pub struct Mailer {
    desk: Rc<SceneFile>,
    strings: Strings,
    parody: bool,
    /// The volume's own mails ([`content::mail_count`]); the port's extras
    /// follow them in `table`.
    count: usize,
    /// The table, in RAM.
    table: Vec<Mail>,
    flags: Vec<Flags>,
    /// `maillist`: mail numbers, newest first.
    pub list: Vec<usize>,
    /// `NewMailNum` (+0xa2): unread mails listed.
    pub new_mail_num: i16,
    /// +0x128 `mailNO`: the cursor; +0x88 `MinMax`: the top row.
    pub mail_no: i32,
    pub min_max: i32,
    reflg: i32,
    pushflg: i32,
    pushcnt: i32,
    repeat_count: i16,
    change_phot: bool,
    icon_scale: f32,
    icon_sflg: bool,
    /// +0x11c: 0 enter, 1 list, 2 reading, 3 a reply mail, 4 leave.
    pub mail_mode: i32,
    pre_mode: i32,
    resub_mode: i32,
    res_mode: i32,
    pre_res_mode: i32,
    pre_pos: i32,
    re_pos: i32,
    pre_res_pos: i32,
    redel: i32,
    res_no: i32,
    check_no: i32,
    lock: i32,
    alpha: i32,
    damyflg: i32,
    resw_rdel: bool,
    resw_del: bool,
    /// +0xec: InfoWindow's frame count (never reset).
    pub counter: i32,
    /// The mail open, as a table index.
    mail: Option<usize>,
    /// SetMailData's text is live (+0xf8, +0x10c).
    mail_text: bool,
    /// PreviewRes's text (+0x104 / +0x110) and the copy kept after a send
    /// (+0x17c / +0x180).
    res_text: bool,
    tmp_res_text: bool,
    /// ResWindow_Control's four kanji (+0x100) exist.
    res_kanji: bool,
    res_kanji_alpha: u8,
    info_alpha: u8,
    /// The sender's photo (`NowPhot`): a `PhotName` index and where.
    now_phot: Option<(usize, Vec3)>,
    /// `Mailview`: its own ANM_xddphot0.
    mailview: Anm,
    /// The desktop's `mask` (kanji layer) and the mailer's `resmask`.
    pub mask: Sprite,
    resmask: Sprite,
}

impl Mailer {
    /// The constructor (0x004085a0) and the table as the overlay loads it.
    pub fn new(assets: &Assets, save: &SaveState) -> piney_data::Result<Self> {
        let parody = save.parody();
        let count = content::mail_count(assets.volume);
        let mut table: Vec<_> = (0..count).map(|i| content::mail(assets.volume, i, parody)).collect();
        // The port's own mail past the game's (crate::extras).
        table.extend((count..MAIL_SLOTS).map(|n| crate::extras::mail(n).unwrap_or_else(|| crate::extras::empty(n))));
        let flags = table.iter().map(|m| Flags { re_flg: m.re_flg, mail_flg: m.mail_flg }).collect();
        let curs = cursor_texture(&assets.desk);
        Ok(Mailer {
            desk: assets.desk.clone(),
            strings: Strings::new(),
            parody,
            count,
            table,
            flags,
            list: Vec::new(),
            new_mail_num: 0,
            mail_no: 0,
            min_max: 0,
            reflg: 0,
            pushflg: 0,
            pushcnt: 0,
            repeat_count: REPEAT_FIRST,
            change_phot: false,
            icon_scale: 1.0,
            icon_sflg: true,
            mail_mode: 0,
            pre_mode: 5,
            resub_mode: 8,
            res_mode: 8,
            pre_res_mode: 16,
            pre_pos: 0,
            re_pos: 0,
            pre_res_pos: 0,
            redel: 0,
            res_no: 0,
            check_no: 0,
            lock: 0,
            alpha: 0,
            damyflg: 0,
            resw_rdel: false,
            resw_del: false,
            counter: 0,
            mail: None,
            mail_text: false,
            res_text: false,
            tmp_res_text: false,
            res_kanji: false,
            res_kanji_alpha: 128,
            info_alpha: 128,
            now_phot: None,
            mailview: Anm::new(),
            mask: Sprite::mask(KANJI_LAYER, curs.0.clone(), curs.1, 64),
            resmask: Sprite::mask(RES_LAYER, curs.0, curs.1, 300),
        })
    }

    pub fn parody(&self) -> bool {
        self.parody
    }

    /// `AddMailList` (0x00409800), from `Desktop_control::AddAllList`: the
    /// inbox from `mailOrderList`, newest first. Returns 1 when a mail was
    /// never seen before (the new-mail notice).
    pub fn add_mail_list(&mut self, save: &mut SaveState) -> i32 {
        let mut ret = 0;
        for i in 0..self.count.min(MAIL_SLOTS) {
            let m = save.save.mail_order(i);
            if m < 0 {
                break;
            }
            let m = m as usize;
            let s = MailState::from_byte(save.save.check_mail(m));
            match s {
                MailState::None => continue,
                MailState::New | MailState::Unread => {
                    self.new_mail_num += 1;
                    if s == MailState::New {
                        ret = 1;
                    }
                }
                MailState::Read | MailState::RepliedOne | MailState::RepliedTwo => {
                    if let Some(f) = self.flags.get_mut(m) {
                        *f = Flags { re_flg: 0, mail_flg: 1 };
                    }
                }
                MailState::Other(_) => {}
            }
            if m < self.table.len() {
                self.list.insert(0, m);
            }
        }
        ret
    }

    /// `SetBack(ccsc, layer, mask)` (0x00408e00): the list's frame on the
    /// kanji layer, and the photo frame's animation started.
    pub fn set_back(&mut self, views: &mut Views) {
        views.kanji = frame(LAYSIZE);
        self.mailview.set(&self.desk, PHOT_ANM);
        self.mailview.forward();
    }

    /// `Resetmail` (0x00409710).
    fn reset(&mut self) {
        self.mail_mode = 0;
        self.pre_res_mode = 16;
        self.res_mode = 8;
        self.resub_mode = 8;
        self.reflg = 0;
        self.mail_no = 0;
        self.pushflg = 0;
        self.pushcnt = 0;
        self.pre_res_pos = 0;
        self.lock = 0;
        self.res_no = 0;
        self.check_no = 0;
        self.re_pos = 0;
        self.redel = 0;
        self.pre_pos = 0;
        self.min_max = 0;
        self.alpha = 0;
        self.mailview = Anm::new();
        self.now_phot = None;
    }

    fn keys(pad: &Pad, save: &SaveState) -> Keys {
        Keys {
            direct: pad.direct.bits(),
            push: pad.push.bits(),
            unpush: pad.unpush.bits(),
            repeat: pad.repeat.bits(),
            ok: save.ok(),
            cancel: save.cancel(),
        }
    }

    /// `MainMailer` (0x00408350). Returns -1 to leave.
    pub fn main(&mut self, x: &mut MailCtx, pad: &Pad) -> i32 {
        let k = Self::keys(pad, x.save);
        match self.mail_mode {
            0 => {
                if self.list.is_empty() {
                    if k.push & k.cancel != 0 {
                        self.mail_mode = 4;
                    }
                    return 0;
                }
                x.req.push(Request::Se(Se(5)));
                self.set_phot();
                self.mail_mode = 1;
                self.mail_list_frame(x, k);
            }
            1 => self.mail_list_frame(x, k),
            2 => self.preview_mail(x, k),
            3 => self.reply_mail(x, k),
            4 => {
                self.reset();
                return -1;
            }
            _ => {}
        }
        0
    }

    /// `MailList` (0x00409a20): the list's frame.
    fn mail_list_frame(&mut self, x: &mut MailCtx, k: Keys) {
        self.move_line(x, k);
        if self.change_phot {
            self.set_phot();
        }
        self.set_length_list(x, self.min_max);
        self.write_title(x, self.min_max);
        if self.reflg == 1 && k.push & k.ok != 0 {
            x.req.push(Request::Se(Se::OPEN));
            let m = self.list[self.mail_no as usize];
            self.pushflg = 0;
            self.pushcnt = 0;
            self.mail = Some(m);
            // FlgCheck (0x0040d180).
            match self.flags[m].re_flg {
                0 => self.mail_mode = 2,
                1 => self.mail_mode = 3,
                _ => {}
            }
        } else if k.push & k.cancel != 0 {
            self.mail_mode = 4;
        }
    }

    /// `CurRepeat(key)` (0x00409230).
    fn cur_repeat(&mut self, k: Keys, key: u32) -> bool {
        if k.push & key != 0 {
            self.pushflg = 1;
            return true;
        }
        if k.unpush & key != 0 {
            self.repeat_count = REPEAT_FIRST;
            self.pushflg = 0;
            return false;
        }
        if k.repeat & key != 0 {
            self.pushcnt += 1;
            if self.pushcnt == i32::from(self.repeat_count) {
                self.repeat_count = REPEAT_NEXT;
                self.pushcnt = 2;
                return true;
            }
        }
        false
    }

    /// The gate both MoveLines start with: OK selects only after it has
    /// been let go once.
    fn gate(&mut self, k: Keys) {
        if self.reflg == 0 && k.direct & k.ok == 0 {
            self.reflg = 1;
        } else if k.push & k.cancel != 0 {
            self.pushflg = 0;
            self.pushcnt = 0;
        }
    }

    /// `MoveLine()` (0x0040ca10): the list cursor and scroll.
    fn move_line(&mut self, x: &mut MailCtx, k: Keys) {
        let n = self.list.len() as i32;
        self.change_phot = false;
        let mut d = 0;
        self.gate(k);
        if self.cur_repeat(k, Buttons::UP.bits()) {
            d = 2;
            x.req.push(Request::Se(Se(6)));
            self.mail_no -= 1;
            self.change_phot = true;
            if self.mail_no < 0 {
                self.mail_no = n - 1;
                self.min_max = (self.mail_no - ROWS + 1).max(0);
            }
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            x.req.push(Request::Se(Se(6)));
            self.mail_no += 1;
            d = 1;
            self.change_phot = true;
            if self.mail_no > n - 1 {
                self.mail_no = 0;
                self.min_max = 0;
            }
        }
        let below = n - (self.min_max + ROWS);
        if self.mail_no - self.min_max > SHIFT_REMIT && d == 1 && below > 0 {
            self.min_max += 1;
        }
        if d == 2 && self.min_max > 0 && self.mail_no - UP_REMIT < self.min_max {
            self.min_max -= 1;
        }
    }

    /// `MoveLine(pos, line)` (0x0040cc10): a body's scroll.
    fn move_body(&mut self, x: &mut MailCtx, k: Keys, which: Scroll, line: i32) {
        self.gate(k);
        let mut pos = self.scroll(which);
        if self.cur_repeat(k, Buttons::UP.bits()) {
            pos -= 1;
            if pos < 0 {
                pos = 0;
            } else {
                x.req.push(Request::Se(Se(6)));
            }
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            pos += 1;
            if line < pos + ROWS {
                pos -= 1;
            } else {
                x.req.push(Request::Se(Se(6)));
            }
            if pos < 0 {
                pos = 0;
            }
        }
        *self.scroll_mut(which) = pos;
    }

    fn scroll(&self, which: Scroll) -> i32 {
        match which {
            Scroll::Pre => self.pre_pos,
            Scroll::Re => self.re_pos,
            Scroll::PreRes => self.pre_res_pos,
        }
    }

    fn scroll_mut(&mut self, which: Scroll) -> &mut i32 {
        match which {
            Scroll::Pre => &mut self.pre_pos,
            Scroll::Re => &mut self.re_pos,
            Scroll::PreRes => &mut self.pre_res_pos,
        }
    }

    /// `SetPhot` (0x00409350): the selected mail's sender photo,
    /// `CMP_xddphot{fromNO + 1}`, at `OBJ_xddphoto`'s position in the frame
    /// animation.
    fn set_phot(&mut self) {
        let Some(&m) = self.list.get(self.mail_no.max(0) as usize) else { return };
        let from = self.table[m].from_no.max(0) as usize;
        self.now_phot = Some((from, self.phot_pos()));
    }

    /// `SetResPhot` (0x00409540): the player's photo, `CMP_xddphot01`.
    fn set_res_phot(&mut self) {
        self.now_phot = Some((0, self.phot_pos()));
    }

    fn phot_pos(&self) -> Vec3 {
        let obj = self.desk.ccs.find_object(PHOT_OBJ);
        self.mailview
            .instances_all()
            .into_iter()
            .find(|i| Some(i.0) == obj)
            .map_or(Vec3::ZERO, |(_, m, _)| m.w_axis.truncate())
    }

    /// `SetLength(pos)` (0x0040c620): the list's scroll bar.
    fn set_length_list(&mut self, x: &mut MailCtx, pos: i32) {
        let n = self.list.len() as i32;
        let at_end = self.mail_no == n - 1;
        self.scroll_bar(x, n, pos, at_end);
    }

    /// `SetLength(pos, pos2, mail)` (0x0040c770 / 0x0040c8c0): a body's.
    fn set_length_body(&mut self, x: &mut MailCtx, pos: i32, pos2: i32, line: i32) {
        self.scroll_bar(x, line, pos2, pos + ROWS >= line);
    }

    fn scroll_bar(&mut self, x: &mut MailCtx, count: i32, pos: i32, at_end: bool) {
        use crate::eef::{add, div, from_int, le, mul, sub};
        let travel = SCRSIZE[2];
        let mut len = mul(travel, div(from_int(ROWS << 4), from_int(count << 4)));
        if !le(len, travel) {
            len = travel;
        }
        let (wu, wv, su, sv) = cell::SCROLL;
        let m = &mut self.mask;
        m.wu = wu;
        m.wv = wv;
        m.wi = 1;
        m.su = su;
        m.sv = sv;
        m.sx = SCRSIZE[3];
        m.sy = len;
        m.dx = sub(sub(LAYSIZE[3], SCRSIZE[3]), 3.0);
        let rest = sub(travel, len);
        let step = div(rest, from_int(count - ROWS));
        m.dy = if at_end { add(52.0, rest) } else { add(52.0, mul(step, from_int(pos))) };
        m.make_packet(0, &x.views.kanji);
    }

    /// `WriteTitle(top)` (0x0040a3f0): the list's rows, icons, count line
    /// and cursor.
    fn write_title(&mut self, x: &mut MailCtx, top: i32) {
        use crate::eef::{add, from_int, le, lt, mul, sub};
        let step = f32::from_bits(ICON_SCALE_STEP);
        if self.icon_sflg {
            self.icon_scale = add(self.icon_scale, step);
            if !le(self.icon_scale, f32::from_bits(ICON_SCALE_TOP)) {
                self.icon_scale = f32::from_bits(ICON_SCALE_TOP);
                self.icon_sflg = false;
            }
        } else {
            self.icon_scale = sub(self.icon_scale, step);
            if lt(self.icon_scale, 1.0) {
                self.icon_scale = 1.0;
                self.icon_sflg = true;
            }
        }
        let mut sel_row = 0;
        let s = self.icon_scale;
        let view = x.views.kanji;
        for (r, &m) in self.list.iter().skip(top.max(0) as usize).take(ROWS as usize).enumerate() {
            let r = r as i32;
            let y = (ROW_Y + ROW_H * r) as f32;
            let mail = &self.table[m];
            let (mut title, mut from) = (Vec::new(), Vec::new());
            if self.mail_no == top + r {
                title.extend_from_slice(b"#G");
                from.extend_from_slice(b"#G");
                sel_row = r;
            }
            title.extend_from_slice(&mail.title);
            from.extend_from_slice(&mail.from);
            x.text(KANJI_LAYER, &view, 139.0, y, &title, true);
            x.text(KANJI_LAYER, &view, 20.0, y, &from, true);
            let mk = &mut self.mask;
            if self.flags[m].mail_flg == 0 {
                let (wu, wv, w, h) = cell::UNREAD;
                mk.cell(wu, wv, w, h);
                mk.sx = mul(13.0, s);
                mk.sy = mul(10.0, s);
                mk.cx = mul(-6.0, s);
                mk.cy = mul(-17.0, s);
                mk.dx = 8.0;
                mk.dy = add(add(48.0, mul(9.0, sub(s, 1.0))), from_int(ROW_H * (r + 1)));
                mk.make_packet(0, &view);
                mk.cx = 0.0;
                mk.cy = 0.0;
            } else {
                let (wu, wv, w, h) = cell::READ;
                mk.cell(wu, wv, w, h);
                mk.dx = 0.0;
                mk.dy = y;
                mk.make_packet(0, &view);
            }
        }
        let st = &self.strings;
        let line = if self.new_mail_num > 0 {
            let mut l = st.mail[0].clone();
            l.extend_from_slice(self.new_mail_num.to_string().as_bytes());
            l.extend_from_slice(&st.mail[if self.new_mail_num == 1 { 1 } else { 3 }]);
            l
        } else {
            st.mail[2].clone()
        };
        let mut k = Kanji::init(3, 16);
        k.dx = 20.0;
        k.dy = 3.0;
        x.disp(&mut k, KANJI_LAYER, &view, &line);
        self.time_alpha_cur_draw(x, 0, 53 + ROW_H * sel_row, false, false);
    }

    /// `TimeAlphaCurDraw(x, y, small, mask)` (0x00409020): the blinking
    /// cursor, sent with whatever the sprite has queued.
    fn time_alpha_cur_draw(&mut self, x: &mut MailCtx, px: i32, py: i32, small: bool, res: bool) {
        let a = cursor_alpha(x.count, x.frame_rate);
        let view = if res { x.views.res } else { x.views.kanji };
        let m = if res { &mut self.resmask } else { &mut self.mask };
        m.colour[3] = a;
        let (wu, wv, w, h) = if small { cell::CURSOR_SMALL } else { cell::CURSOR_BIG };
        m.cell(wu, wv, w, h);
        m.dx = px as f32;
        m.dy = py as f32;
        m.make_packet(0, &view);
        m.send(&mut x.ctx.layers);
        m.colour[3] = 0x80;
    }

    /// `SetMailData(mail)` (0x00409b60).
    fn set_mail_data(&mut self) {
        self.mail_text = true;
    }

    /// `WriteMail(pos, mail)` (0x0040a150): a body from line `pos`, the
    /// title and sender (no shadow), and the read / unread icon.
    fn write_mail(&mut self, x: &mut MailCtx, pos: i32, m: usize) {
        let view = x.views.kanji;
        let mail = &self.table[m];
        for (r, line) in mail.lines.iter().skip(pos.max(0) as usize).take(ROWS as usize).enumerate() {
            x.text(KANJI_LAYER, &view, 8.0, (ROW_Y + ROW_H * r as i32) as f32, line, true);
        }
        x.text(KANJI_LAYER, &view, 140.0, 24.0, &mail.title, false);
        x.text(KANJI_LAYER, &view, 22.0, 24.0, &mail.from, false);
        let (wu, wv, w, h) = if self.flags[m].mail_flg == 0 { cell::UNREAD } else { cell::READ };
        self.mask.cell(wu, wv, w, h);
        self.mask.dx = 0.0;
        self.mask.dy = 27.0;
        self.mask.make_packet(0, &view);
    }

    /// `PreviewMail(mail)` (0x0040aac0): reading a mail that takes no reply.
    fn preview_mail(&mut self, x: &mut MailCtx, k: Keys) {
        let Some(m) = self.mail else { return };
        let line = self.table[m].lines.len() as i32;
        if self.pre_mode == 5 {
            self.set_mail_data();
            self.pre_mode = 6;
        }
        match self.pre_mode {
            6 => {
                self.move_body(x, k, Scroll::Pre, line);
                self.set_length_body(x, self.pre_pos, self.pre_pos, line);
                self.write_mail(x, self.pre_pos, m);
                if k.push & k.cancel != 0 {
                    self.pre_mode = 7;
                    x.req.push(Request::Se(Se::CLOSE));
                }
            }
            7 => {
                self.set_length_body(x, self.pre_pos, 0, line);
                if self.redel < 3 {
                    self.redel += 1;
                    return;
                }
                self.redel = 0;
                self.pre_pos = 0;
                let s = x.save.mail(m);
                if s != MailState::RepliedOne && s != MailState::RepliedTwo {
                    x.save.set_mail(m, MailState::Read);
                }
                if self.flags[m].mail_flg == 0 {
                    self.new_mail_num -= 1;
                }
                self.flags[m].mail_flg = 1;
                self.mail_text = false;
                self.pre_mode = 5;
                self.mail_mode = 1;
            }
            _ => {}
        }
    }

    /// `ReplyMail(mail)` (0x0040acb0): a mail that waits for a reply.
    fn reply_mail(&mut self, x: &mut MailCtx, k: Keys) {
        let Some(m) = self.mail else { return };
        match self.pre_mode {
            5 => {
                self.pre_mode = 6;
                self.set_mail_data();
                self.alpha = 0;
                x.views.res = frame(RELAYSIZE);
                self.sub_reply_mail(x, k, m);
            }
            6 => self.sub_reply_mail(x, k, m),
            7 => {
                if self.redel != 1 {
                    self.redel = 1;
                    return;
                }
                if self.lock == 1 {
                    self.reply_mail_del(m);
                } else if k.push & k.ok != 0 {
                    self.set_phot();
                    self.pushflg = 0;
                    self.pushcnt = 0;
                    self.reply_mail_del(m);
                } else {
                    if self.alpha == 0 {
                        x.req.push(Request::Se(Se(15)));
                    }
                    self.alpha = (self.alpha + 5).min(128);
                    self.set_a_info(self.alpha);
                    self.info_window_end(x);
                }
            }
            _ => {}
        }
    }

    /// `ReplyMailDel(mail)` (0x0040ae80). "Don't reply" (lock 1) leaves the
    /// mail unread and waiting.
    fn reply_mail_del(&mut self, m: usize) {
        self.redel = 0;
        self.re_pos = 0;
        self.mail_mode = 1;
        if self.lock != 1 {
            if self.flags[m].mail_flg == 0 {
                self.new_mail_num -= 1;
            }
            self.flags[m] = Flags { re_flg: 0, mail_flg: 1 };
        }
        self.resub_mode = 8;
        self.lock = 0;
        self.mail_text = false;
        self.pre_mode = 5;
    }

    /// `SubReplyMail(mail)` (0x0040af20).
    fn sub_reply_mail(&mut self, x: &mut MailCtx, k: Keys, m: usize) {
        let line = self.table[m].lines.len() as i32;
        let mut s0 = -1;
        match self.resub_mode {
            8 | 9 => {
                if self.resub_mode == 8 {
                    self.move_body(x, k, Scroll::Re, line);
                }
                self.damyflg = 1;
                if self.resub_mode == 9 {
                    s0 = self.res_window_control(x, k, m);
                }
                match s0 {
                    0 | 1 => self.resub_mode = 13,
                    2 => self.resub_mode = 8,
                    5 => self.pre_mode = 7,
                    _ => {}
                }
                self.set_length_body(x, self.re_pos, self.re_pos, line);
                self.write_mail(x, self.re_pos, m);
                if self.resub_mode == 8 && k.push & (k.ok | k.cancel) != 0 && s0 != 2 {
                    x.req.push(Request::Se(Se::OPEN));
                    self.resub_mode = 9;
                }
            }
            13 => {
                s0 = self.res_window_control(x, k, m);
                if self.damyflg == 1 {
                    self.set_length_body(x, self.re_pos, 0, line);
                    self.damyflg = 0;
                }
                if s0 == 2 {
                    self.resub_mode = 9;
                } else if k.push & k.ok != 0 {
                    self.resub_mode = 14;
                }
            }
            14 => {
                s0 = self.res_window_control(x, k, m);
                if s0 == 1 {
                    self.resub_mode = 13;
                }
                if s0 == 3 {
                    self.pre_mode = 7;
                }
            }
            _ => {}
        }
    }

    fn chosen(&self, m: usize) -> Option<ReMail> {
        let mail = &self.table[m];
        match self.res_no {
            0 => mail.one_res.clone(),
            1 => mail.two_res.clone(),
            _ => None,
        }
    }

    /// `ResWindow_Control(mail)` (0x0040b1a0): the reply chooser, its
    /// preview and the YES / NO confirmation. -1 while running.
    fn res_window_control(&mut self, x: &mut MailCtx, k: Keys, m: usize) -> i32 {
        let ret = self.res_window_state(x, k, m);
        if ret != -1 {
            return ret;
        }
        if k.unpush & (k.ok | k.cancel) != 0 {
            self.lock = 2;
        }
        -1
    }

    fn res_window_state(&mut self, x: &mut MailCtx, k: Keys, m: usize) -> i32 {
        let chosen = self.chosen(m);
        match self.res_mode {
            8 | 10 => {
                if self.res_mode == 8 {
                    self.res_kanji = true;
                    self.alpha = 0;
                    self.res_mode = 10;
                    self.resw_rdel = false;
                    self.resw_del = false;
                }
                self.alpha += 8;
                if self.alpha >= 128 {
                    self.alpha = 128;
                    let freed = self.tmp_res_text;
                    self.tmp_res_text = false;
                    if !freed {
                        self.res_mode = 9;
                    }
                }
                self.set_a(self.alpha);
                self.res_window(x, Some(RELAYSIZE), 0);
                self.reply_list(x, m);
            }
            9 => {
                if self.cur_repeat(k, Buttons::UP.bits()) {
                    self.res_no -= 1;
                    x.req.push(Request::Se(Se(6)));
                }
                if self.cur_repeat(k, Buttons::DOWN.bits()) {
                    self.res_no += 1;
                    x.req.push(Request::Se(Se(6)));
                }
                if self.res_no >= 3 {
                    self.res_no = 0;
                }
                if self.res_no < 0 {
                    self.res_no = 2;
                }
                if self.lock > 0 && k.push & k.ok != 0 {
                    self.pushflg = 0;
                    self.pushcnt = 0;
                    x.req.push(Request::Se(Se::OPEN));
                    if self.res_no == 0 || self.res_no == 1 {
                        self.set_res_phot();
                        self.res_mode = 13;
                        return self.res_no;
                    }
                    self.res_mode = 15;
                    self.lock = 1;
                    return 4;
                }
                if self.lock > 0 && k.push & k.cancel != 0 {
                    x.req.push(Request::Se(Se::CLOSE));
                    self.alpha = 0;
                    self.res_mode = 10;
                    self.lock = 1;
                    return 2;
                }
                self.res_window(x, Some(RELAYSIZE), 0);
                self.reply_list(x, m);
            }
            13 => {
                if let Some(re) = &chosen {
                    self.preview_res(x, k, re, m, -1);
                }
                if k.push & k.ok != 0 {
                    x.req.push(Request::Se(Se::OPEN));
                    self.res_mode = 11;
                    self.alpha = 0;
                } else if k.push & k.cancel != 0 {
                    x.req.push(Request::Se(Se::CLOSE));
                    self.resw_rdel = true;
                    self.alpha = 0;
                    self.res_mode = 10;
                    self.lock = 1;
                    self.set_phot();
                    if let Some(re) = &chosen {
                        self.preview_res(x, k, re, m, -1);
                    }
                    self.resw_rdel = false;
                    return 2;
                }
            }
            11 => {
                self.alpha += 8;
                if self.alpha >= 128 {
                    self.alpha = 128;
                    self.res_mode = 14;
                }
                self.set_a(self.alpha);
                self.res_window(x, None, 0);
                self.check_list(x);
                if let Some(re) = &chosen {
                    self.preview_res(x, k, re, m, -1);
                }
            }
            14 => {
                if self.cur_repeat(k, Buttons::UP.bits()) {
                    self.check_no += 1;
                    x.req.push(Request::Se(Se(6)));
                } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
                    self.check_no -= 1;
                    x.req.push(Request::Se(Se(6)));
                }
                if self.check_no >= 2 {
                    self.check_no = 0;
                }
                if self.check_no < 0 {
                    self.check_no = 1;
                }
                if k.push & k.ok != 0 {
                    x.req.push(Request::Se(Se::OPEN));
                    if self.check_no == 1 {
                        if let Some(re) = &chosen {
                            self.preview_res(x, k, re, m, 1);
                            x.save.set_mail(
                                m,
                                if self.res_no == 0 { MailState::RepliedOne } else { MailState::RepliedTwo },
                            );
                            self.preview_res(x, k, re, m, 1);
                        }
                        self.alpha = 0;
                        self.res_mode = 15;
                        self.resw_del = true;
                    } else {
                        if let Some(re) = &chosen {
                            self.preview_res(x, k, re, m, 0);
                        }
                        self.res_mode = 13;
                        return 1;
                    }
                } else if k.push & k.cancel != 0 {
                    if let Some(re) = &chosen {
                        self.preview_res(x, k, re, m, -1);
                    }
                    x.req.push(Request::Se(Se::CLOSE));
                    self.res_mode = 13;
                    return 1;
                }
                if self.res_mode != 15 {
                    if let Some(re) = &chosen {
                        self.preview_res(x, k, re, m, -1);
                    }
                    self.res_window(x, None, 0);
                    self.check_list(x);
                }
            }
            15 => {
                self.tmp_res_text = false;
                let mut s0 = if self.lock == 1 { 3 } else { 0 };
                self.res_mode = 8;
                self.res_no = 0;
                self.check_no = 0;
                if self.resw_del {
                    s0 = 1;
                }
                self.res_kanji = false;
                return match s0 {
                    0 => 2,
                    1 => 3,
                    _ => 5,
                };
            }
            _ => {}
        }
        -1
    }

    /// `PreviewRes(re, arg)` (0x0040bbf0): the chosen reply's text.
    fn preview_res(&mut self, x: &mut MailCtx, k: Keys, re: &ReMail, m: usize, arg: i32) {
        let line = re.lines.len() as i32;
        match self.pre_res_mode {
            16 | 17 => {
                if self.pre_res_mode == 16 {
                    if self.tmp_res_text {
                        return;
                    }
                    self.res_text = true;
                    self.pre_res_mode = 17;
                }
                self.move_body(x, k, Scroll::PreRes, line);
                let s2 = if self.pre_res_pos > 0 { self.pre_res_pos } else { 0 };
                self.set_length_body(x, self.pre_res_pos, s2, line);
                if self.res_mode == 13 {
                    if k.push & k.ok != 0 {
                        self.pre_res_mode = 18;
                    } else if k.push & k.cancel != 0 {
                        self.pre_res_pos = 0;
                        self.pre_res_mode = 19;
                    }
                }
                if self.pre_res_mode != 19 {
                    self.write_res(x, re);
                }
            }
            18 => {
                if self.res_mode == 14 {
                    if k.push & k.ok != 0 {
                        if arg == 1 {
                            self.pre_res_mode = 19;
                        }
                        if arg == 0 {
                            self.pre_res_mode = 17;
                        }
                    } else if k.push & k.cancel != 0 {
                        self.pre_res_mode = 17;
                    }
                }
                if arg == -1 || arg == 0 {
                    self.write_res(x, re);
                    self.set_length_body(x, self.pre_res_pos, 0, line);
                }
            }
            19 => {
                let _ = m;
                self.tmp_res_text = self.res_text;
                self.res_text = false;
                self.pre_res_mode = 16;
            }
            _ => {}
        }
    }

    /// PreviewRes's text: the body at (20, 48 + 17 r), title (139, 22),
    /// sender (20, 22).
    fn write_res(&mut self, x: &mut MailCtx, re: &ReMail) {
        let view = x.views.kanji;
        for (r, l) in re.lines.iter().skip(self.pre_res_pos.max(0) as usize).take(ROWS as usize).enumerate() {
            x.text(KANJI_LAYER, &view, 20.0, (ROW_Y + ROW_H * r as i32) as f32, l, true);
        }
        x.text(KANJI_LAYER, &view, 139.0, 22.0, &re.title, false);
        x.text(KANJI_LAYER, &view, 20.0, 22.0, &re.from, false);
    }

    /// `SetA(kanji, a)` (0x0040c1c0): the reply window's alpha.
    fn set_a(&mut self, a: i32) {
        self.resmask.colour[3] = a as u8;
        if self.res_mode == 10 || self.res_mode == 11 {
            self.res_kanji_alpha = a as u8;
        }
    }

    /// `SetA(InfoKanji, a)`.
    pub fn set_a_info(&mut self, a: i32) {
        self.resmask.colour[3] = a as u8;
        self.info_alpha = a as u8;
    }

    /// `ReplyList(mail, n)` (0x0040cd80): the two replies, "Don't reply",
    /// the heading, and the small cursor on the reply window.
    fn reply_list(&mut self, x: &mut MailCtx, m: usize) {
        let view = x.views.resk;
        let a = self.res_kanji_alpha;
        let mail = &self.table[m];
        let one = mail.one_res.as_ref().map(|r| r.title.clone()).unwrap_or_default();
        let two = mail.two_res.as_ref().map(|r| r.title.clone()).unwrap_or_default();
        x.text_a(RESK_LAYER, &view, 24.0, 22.0, &one, a);
        x.text_a(RESK_LAYER, &view, 24.0, 40.0, &two, a);
        x.text_a(RESK_LAYER, &view, 24.0, 58.0, &self.strings.nores.clone(), a);
        x.text_a(RESK_LAYER, &view, 10.0, 5.0, &self.strings.sel_remail.clone(), a);
        self.time_alpha_cur_draw(x, 8, 26 + ROW_H * self.res_no, true, true);
    }

    /// `CheckList(n)` (0x0040ced0): "Send e-mail?", YES, NO.
    fn check_list(&mut self, x: &mut MailCtx) {
        let view = x.views.resk;
        let a = self.res_kanji_alpha;
        x.text_a(RESK_LAYER, &view, 10.0, 10.0, &self.strings.send_mail.clone(), a);
        x.text_a(RESK_LAYER, &view, 24.0, 28.0, &self.strings.yes.clone(), a);
        x.text_a(RESK_LAYER, &view, 24.0, 46.0, &self.strings.no.clone(), a);
        self.time_alpha_cur_draw(x, 8, 49 - ROW_H * self.check_no, true, true);
    }

    /// `ResWindow(rect, y0)` (0x0040c250): a window of 8 x 8 tiles on the
    /// reply layer.
    fn res_window(&mut self, x: &mut MailCtx, rect: Option<[f32; 4]>, y0: i32) {
        let view = x.views.res;
        let (h, w) = match rect {
            Some(r) => (r[2], r[3]),
            None => (RESSIZE[2], RESSIZE[3]),
        };
        let m = &mut self.resmask;
        m.sx = 8.0;
        m.su = 8;
        m.sy = 8.0;
        m.sv = 8;
        m.wi = 1;
        let tile = |m: &mut Sprite, col: i32, row: usize, px: i32, py: i32| {
            m.wu = col;
            m.wv = cell::WIN_ROWS[row];
            m.dx = px as f32;
            m.dy = py as f32;
            m.make_packet(0, &view);
        };
        let col = |px: i32, middle_left: bool| {
            let mut c = if (middle_left && px == 0) || (!middle_left && px < 8) {
                cell::WIN_COLS[0]
            } else {
                cell::WIN_COLS[1]
            };
            if (px as f32) >= w - 8.0 {
                c = cell::WIN_COLS[2];
            }
            c
        };
        let mut px = 0;
        while px as f32 <= w {
            tile(m, col(px, false), 0, px, y0);
            px += 8;
        }
        let mut py = 8;
        while (py as f32) < h - 8.0 {
            let mut px = 0;
            while px as f32 <= w {
                tile(m, col(px, true), 1, px, py + y0);
                px += 8;
            }
            py += 8;
        }
        let mut px = 0;
        while px as f32 <= w {
            tile(m, col(px, false), 2, px, (h - 8.0 + y0 as f32) as i32);
            px += 8;
        }
    }

    /// `InfoWindow` (0x0040cfe0): "You have new mail." in a window at
    /// (160, 50). Returns the frame count.
    pub fn info_window(&mut self, x: &mut MailCtx) -> i32 {
        x.views.res = LayerView::frame(INFO_POS[0], INFO_POS[1], RESSIZE[3], RESSIZE[2], 1.0, AY_SQUARE);
        let view = x.views.res;
        x.text_a(RES_LAYER, &view, 48.0, 28.0, &self.strings.newmail.clone(), self.info_alpha);
        self.res_window(x, None, 0);
        self.resmask.send(&mut x.ctx.layers);
        self.counter += 1;
        self.counter
    }

    /// `InfoWindowEnd` (0x0040d0c0): "Reply sent." at (200, 150).
    fn info_window_end(&mut self, x: &mut MailCtx) {
        x.views.res = LayerView::frame(INFO_END_POS[0], INFO_END_POS[1], RESSIZE[3], RESSIZE[2], 1.0, AY_SQUARE);
        let view = x.views.res;
        x.text_a(RES_LAYER, &view, 72.0, 30.0, &self.strings.send_comp.clone(), self.info_alpha);
        self.res_window(x, None, 0);
    }

    /// `MailList_control::DrawBack` (0x00408f40): the photo with the frame
    /// object's transparency, the frame animation, then the two masks.
    pub fn draw_back(&mut self, x: &mut MailCtx) {
        if let Some((phot, pos)) = self.now_phot {
            let tp = self.phot_alpha();
            self.draw_photo(x.ctx, phot, pos, tp);
        }
        if self.mailview.is_set() {
            self.mailview.forward();
            self.mailview.draw(x.ctx);
        }
        self.mask.send(&mut x.ctx.layers);
        self.resmask.send(&mut x.ctx.layers);
    }

    fn phot_alpha(&self) -> f32 {
        let obj = self.desk.ccs.find_object(PHOT_OBJ);
        self.mailview.instances_all().into_iter().find(|i| Some(i.0) == obj).map_or(1.0, |i| i.2)
    }

    /// `ccClump::Draw` of `CMP_xddphotNN` placed at `pos`.
    fn draw_photo(&self, ctx: &mut Ctx, phot: usize, pos: Vec3, tp: f32) {
        let file = &self.desk;
        let name = format!("CMP_xddphot{:02}", phot + 1);
        let Some(clump) = file.ccs.find_object(&name) else { return };
        let Some((_, nodes)) = file.scene.clumps.iter().find(|(c, _)| *c == clump) else { return };
        let view = ctx.view.clone();
        let rows = std::collections::HashMap::new();
        for &node in nodes {
            let Some(&model) = file.obj_model.get(&node) else { continue };
            let inst = Instance { object: node, model, world: Mat4::from_translation(pos), alpha: tp };
            draw_model(ctx, &view, file, &inst, 1.0, &rows);
        }
    }

    /// The mail open, for tests and the report.
    pub fn open_mail(&self) -> Option<usize> {
        self.mail
    }

    /// A reply mail's steps (PreMode, ResubMode, ResMode, ResNo, and
    /// checkNo: the confirmation's YES 1, NO 0), for tests and the report.
    pub fn reply_state(&self) -> (i32, i32, i32, i32, i32) {
        (self.pre_mode, self.resub_mode, self.res_mode, self.res_no, self.check_no)
    }
}

#[derive(Clone, Copy)]
enum Scroll {
    Pre,
    Re,
    PreRes,
}

/// `TimeAlphaCurDraw`'s blink: `ccSys.count` in, alpha out.
pub fn cursor_alpha(count: u32, frame_rate: u32) -> u8 {
    let (t, half) =
        if frame_rate == 2 { ((count & 0x7f) as i32, 64) } else { ((count.wrapping_mul(5) % 160) as i32, 80) };
    let t = if t < half { t } else { 2 * half - t };
    (48.0 + t as f32) as u8
}

/// `laysize`-style (x, y, hi, wid) to a `SetFrame(x, y, wid, hi, wid / 2,
/// hi / 2, 1, 6/7)` view.
pub fn frame(r: [f32; 4]) -> LayerView {
    LayerView::frame(r[0], r[1], r[3], r[2], 1.0, AY_SQUARE)
}

/// `xddesk01::TEX_xddcurs1` and its height.
pub(crate) fn cursor_texture(desk: &SceneFile) -> (TexRef, i32) {
    let tex = desk.ccs.find_object("TEX_xddcurs1").unwrap_or(0);
    let clut = desk.ccs.find_object("CLT_xddcurs1").unwrap_or(0);
    (TexRef::Ccs { file: desk.stem.clone(), texture: tex, clut }, 64)
}

/// The views of the layers the mailer draws on.
#[derive(Clone, Copy, Debug)]
pub struct Views {
    /// Layer 127, `Desktop_control.kanjilayer`.
    pub kanji: LayerView,
    /// Layer 128, `resLayer`.
    pub res: LayerView,
    /// Layer 129, `reskLayer`.
    pub resk: LayerView,
}

impl Default for Views {
    fn default() -> Self {
        Views { kanji: LayerView::default_layer(), res: frame(RELAYSIZE), resk: frame(RELAYSIZE) }
    }
}

/// What one mailer call needs from its caller.
pub struct MailCtx<'a> {
    pub ctx: &'a mut Ctx,
    pub fonts: &'a Fonts,
    pub names: Names,
    pub save: &'a mut SaveState,
    pub req: &'a mut Vec<Request>,
    pub views: &'a mut Views,
    /// `ccSys.count`: frames since boot.
    pub count: u32,
    pub frame_rate: u32,
}

impl MailCtx<'_> {
    /// A kanji in `ccSpriteColorTable[7]`, sent at (dx, dy).
    pub(crate) fn text(&mut self, layer: i16, view: &LayerView, dx: f32, dy: f32, s: &[u8], shadow: bool) {
        let mut k = Kanji::init(3, 16);
        k.colour = SPRITE_COLOR_TABLE[7];
        k.shadow = shadow;
        k.dx = dx;
        k.dy = dy;
        self.ctx.disp(self.fonts, &mut k, layer, view, s, &self.names);
    }

    /// A kanji in another colour of `ccSpriteColorTable` (its RGB; the
    /// alpha stays 0x80), with the drop shadow.
    pub(crate) fn text_rgb(&mut self, layer: i16, view: &LayerView, dx: f32, dy: f32, s: &[u8], rgba: [u8; 4]) {
        let mut k = Kanji::init(3, 16);
        k.colour = [rgba[0], rgba[1], rgba[2], 0x80];
        k.dx = dx;
        k.dy = dy;
        self.ctx.disp(self.fonts, &mut k, layer, view, s, &self.names);
    }

    fn text_a(&mut self, layer: i16, view: &LayerView, dx: f32, dy: f32, s: &[u8], a: u8) {
        let mut k = Kanji::init(3, 16);
        k.colour = SPRITE_COLOR_TABLE[7];
        k.colour[3] = a;
        k.dx = dx;
        k.dy = dy;
        self.ctx.disp(self.fonts, &mut k, layer, view, s, &self.names);
    }

    fn disp(&mut self, k: &mut Kanji, layer: i16, view: &LayerView, s: &[u8]) {
        self.ctx.disp(self.fonts, k, layer, view, s, &self.names);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TimeAlphaCurDraw's closed form, checked in eemu by the mail
    /// investigation (count 0..299 and the wrap, both frame rates).
    #[test]
    fn cursor_blinks_like_the_game() {
        let one: Vec<u8> = (0..32).map(|c| cursor_alpha(c, 1)).collect();
        assert_eq!(&one[..4], &[48, 53, 58, 63]);
        assert_eq!(one[16], 128);
        assert_eq!(one[31], 53);
        assert_eq!(cursor_alpha(64, 2), 112);
        assert_eq!(cursor_alpha(0, 2), 48);
    }
}

/// Hooks for `tools/test_desktop_rs.py`: the mailer's cursor, scroll and
/// scroll-bar logic on a list of `n` mails, without drawing.
#[cfg(feature = "trace")]
pub mod probe {
    use super::*;

    /// The fields MoveLine and CurRepeat touch.
    pub type ListState = (i32, i32, i32, i32, i32, i16);

    impl Mailer {
        pub fn probe_list(&mut self, n: usize) {
            self.list = (0..n).collect();
        }

        /// (mailNO, MinMax, reflg, pushflg, pushcnt, RepeatCount).
        pub fn probe_state(&self) -> ListState {
            (self.mail_no, self.min_max, self.reflg, self.pushflg, self.pushcnt, self.repeat_count)
        }

        pub fn probe_set(&mut self, s: ListState) {
            (self.mail_no, self.min_max, self.reflg, self.pushflg, self.pushcnt, self.repeat_count) = s;
        }

        /// A fresh inbox (the table's flags as loaded).
        pub fn probe_reset_inbox(&mut self) {
            self.list.clear();
            self.new_mail_num = 0;
            for (f, m) in self.flags.iter_mut().zip(&self.table) {
                *f = Flags { re_flg: m.re_flg, mail_flg: m.mail_flg };
            }
        }

        /// (reFlg, mailFlg) of mail `m` in RAM.
        pub fn probe_flags(&self, m: usize) -> (i32, i32) {
            (self.flags[m].re_flg, self.flags[m].mail_flg)
        }

        pub fn probe_pre_pos(&self) -> i32 {
            self.pre_pos
        }

        pub fn probe_set_pre_pos(&mut self, p: i32) {
            self.pre_pos = p;
        }

        /// One MoveLine() with the pad (direct, push, unpush, repeat).
        pub fn probe_move_list(&mut self, x: &mut MailCtx, pad: [u32; 4]) {
            let k = Keys {
                direct: pad[0],
                push: pad[1],
                unpush: pad[2],
                repeat: pad[3],
                ok: x.save.ok(),
                cancel: x.save.cancel(),
            };
            self.move_line(x, k);
        }

        /// One MoveLine(PrePos, line).
        pub fn probe_move_body(&mut self, x: &mut MailCtx, pad: [u32; 4], line: i32) {
            let k = Keys {
                direct: pad[0],
                push: pad[1],
                unpush: pad[2],
                repeat: pad[3],
                ok: x.save.ok(),
                cancel: x.save.cancel(),
            };
            self.move_body(x, k, Scroll::Pre, line);
        }

        /// SetLength(MinMax) on the list: the scroll bar's (dy, sy).
        pub fn probe_length(&mut self, x: &mut MailCtx, top: i32) -> (f32, f32) {
            self.set_length_list(x, top);
            let _ = self.mask.queued_len();
            (self.mask.dy, self.mask.sy)
        }
    }
}
