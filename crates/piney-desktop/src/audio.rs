//! `Audio_control` (DESKTOP.PRG, audio.cpp; constructor 0x00405b70): the Audio
//! screen, a menu of Sound Mode (the desktop music) and Movie Mode (the movies
//! unlocked), and a list for each (`docs/engine/desktop.md`, "Audio"). The
//! frame `laysize` (130, 131, 153 x 305) is on the kanji layer, the scroll bar
//! `scrsize` on `ScrLayer` (129) framed by `Tmpsize`; text is `ccKanji` with
//! the drop shadow in `ccSpriteColorTable[7]`.

use piney_data::tables::desktop;
use piney_data::tables::sjis::encode;
use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::anm::Ctx;
use crate::assets::Assets;
use crate::content::{self, StrData, WaveData};
use crate::kanji::dec2sjis;
use crate::layers::{KANJI_LAYER, RESK_LAYER};
use crate::mail::{MailCtx, cursor_alpha, cursor_texture, frame};
use crate::save::SaveState;
use crate::sprite::Sprite;
use crate::view::LayerView;
use crate::{Request, Se, fade};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

/// `Numremit`: rows a list shows.
pub const ROWS: i32 = 10;
/// `shiftRemit`, `UpRemit`: where a list starts scrolling down / up.
pub const SHIFT_REMIT: i32 = 7;
pub const UP_REMIT: i32 = 2;
/// Row pitch and first row.
pub const ROW_H: i32 = 17;
pub const ROW_Y: i32 = 5;
pub const TEXT_X: f32 = 32.0;
/// The two info lines (`Infokanji[2]`, `[3]`).
pub const INFO: [(f32, f32); 2] = [(165.0, 140.0), (165.0, 157.0)];
/// The menu's two items and their cursor.
pub const MENU: [(f32, f32); 2] = [(34.0, 60.0), (34.0, 95.0)];
pub const MENU_CURSOR: [(i32, i32); 2] = [(21, 65), (21, 100)];
/// The list cursor's x, its y past its row's, and its y with no row
/// selected.
pub const CURSOR_X: i32 = 20;
pub const CURSOR_DY: i32 = 6;
pub const CURSOR_NONE: i32 = 1;
/// `laysize` (x, y, hi, wid) (`ControlDataSet` 0x004062f0:
/// `SetFrame(130, 131, 305, 153, 152.5, 76.5, 1, 6/7)`).
pub const LAYSIZE: [f32; 4] = [130.0, 131.0, 153.0, 305.0];
/// `scrsize` (x, y, hi, wid): the scroll bar; x is never read.
pub const SCRSIZE: [f32; 4] = [246.0, 8.0, 138.0, 5.0];
/// `Tmpsize` (x, y, hi, wid): `ScrLayer`'s frame.
pub const TMPSIZE: [f32; 4] = [130.0, 131.0, 153.0, 155.0];
/// `ScrLayer`'s priority (`Init(129, 0)` in `SetData` 0x00405ee0).
pub const SCR_LAYER: i16 = RESK_LAYER;
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
/// `MAX_WAVE_NUM`: `Wave[50]`, "Original 01", is always listed.
pub const MAX_WAVE_NUM: usize = 50;
/// The movie's fades: `EntryFlash3(30, 1, 1, 0x80000000, ...)` before and
/// `EntryFlash(30, 0x80000000, ...)` after (`StartStream`, mode 5).
pub const MOVIE_FADE: u32 = 30;
/// The sound for Movie Mode refused in parody mode.
pub const SE_REFUSED: Se = Se(20);

pub mod cell {
    pub const CURSOR: (i32, i32, i32, i32) = (208, 0, 14, 11);
    pub const SCROLL: (i32, i32, i32, i32) = (0, 368, 7, 7);
}

#[derive(Clone, Copy)]
struct Keys {
    push: u32,
    unpush: u32,
    repeat: u32,
    ok: u32,
    cancel: u32,
}

/// The screen's strings, alike on the four volumes
/// (`piney_data::tables::desktop`'s `AUDIO_`). `lock`'s lines: 0 "#YClear
/// the game#W", 1 "for #YVol." 2 "#W movies.", 3-4 "Watch movies " /
/// "you've obtained.", 5-6 "Change desktop " / "music.", 7 "???", 8
/// "#YClear the game#W.", 9-10 "Cannot be used in" / "Parody Mode.".
struct Strings {
    snd_mode: Vec<u8>,
    str_mode: Vec<u8>,
    lock: Vec<Vec<u8>>,
}

impl Strings {
    fn new() -> Self {
        Strings {
            snd_mode: encode(*desktop::AUDIO_SND_MODE),
            str_mode: encode(*desktop::AUDIO_STR_MODE),
            lock: desktop::AUDIO_LOCK.iter().map(|s| encode(s)).collect(),
        }
    }
}

/// A fading element of the movie's (`ccScFade`), and its frames drawn.
#[derive(Clone, Copy, Debug)]
struct Fade {
    c0: u32,
    c1: u32,
    cnt: u32,
}

/// `Audio_control`.
pub struct Audio {
    curs: (TexRef, i32),
    strings: Strings,
    /// The disc's `volumeNum`: Movie Mode opens once it is cleared.
    volume_num: i32,
    waves: Vec<WaveData>,
    streams: Vec<StrData>,
    /// `WaveList`: `Wave` indices.
    pub wave_list: Vec<usize>,
    /// `StrList` and `StrNumList`: `Stream` indices and their volumes.
    pub str_list: Vec<(usize, i32)>,
    pushflg: i32,
    pushcnt: i32,
    /// +0x2c `ClearFlg`: `saveData.clearFlag` as the screen opened.
    clear_flg: i32,
    /// +0x30, +0x44: the cursor and the top row of the list up.
    pub list_no: i32,
    pub min_max: i32,
    /// +0x40: 0 enter, 1 music, 2 the menu, 4 movies, 5 a movie, 6 leave.
    pub mode: i32,
    kill_flg: i8,
    /// +0x4c `NowNum`: the music playing, as a `Wave` index.
    pub now_num: usize,
    /// +0x50 `PreviewCount`: frames since a movie was chosen, -1 none.
    pub preview_count: i32,
    repeat_count: i16,
    /// +0x90 `AccsMode`: the menu's cursor, 1 Sound, 2 Movie; kept.
    pub accs_mode: i32,
    /// +0xb4 `TempStr`: the movie under the cursor as last drawn.
    temp_str: Option<usize>,
    /// +0xbc `loadFlag`: `BgmRead` runs.
    load_flag: bool,
    mask: Option<Sprite>,
    scrmask: Option<Sprite>,
    scr_view: LayerView,
    fade: Option<Fade>,
}

impl Audio {
    /// The constructor (0x00405b70) and the tables as the overlay loads
    /// them.
    pub fn new(assets: &Assets, save: &SaveState) -> piney_data::Result<Self> {
        let waves = content::waves(assets.volume);
        let streams = content::streams(assets.volume);
        Ok(Audio {
            curs: cursor_texture(&assets.desk),
            strings: Strings::new(),
            volume_num: assets.volume.number(),
            waves,
            streams,
            wave_list: Vec::new(),
            str_list: Vec::new(),
            pushflg: 0,
            pushcnt: 0,
            clear_flg: 0,
            list_no: 0,
            min_max: 0,
            mode: 0,
            kill_flg: 0,
            now_num: save.dt_bgm(),
            preview_count: -1,
            repeat_count: REPEAT_FIRST,
            accs_mode: 1,
            temp_str: None,
            load_flag: false,
            mask: None,
            scrmask: None,
            scr_view: scr_frame(),
            fade: None,
        })
    }

    /// `AddWaveList` (0x00406450) and `AddStrList` (0x004065b0), from
    /// `AddAllList`: `Wave[i]` for each bit i < 50 of `dtBgmList` set, then
    /// `Wave[50]`; `Stream[i]` for each bit of `dtStrList`, with its volume.
    /// From Mutation on the waves after the original (51 on) are listed by
    /// their bits too: the bound is a global equal to the table's rows (52
    /// on each later disc).
    pub fn add_lists(&mut self, save: &SaveState) -> piney_data::Result<()> {
        let bound =
            if self.volume_num == piney_data::volume::Volume::Inf.number() { MAX_WAVE_NUM } else { self.waves.len() };
        for i in (0..self.waves.len().min(bound)).filter(|&i| i != MAX_WAVE_NUM) {
            if save.bgm_unlocked(i) {
                self.wave_list.push(i);
            }
        }
        if MAX_WAVE_NUM < self.waves.len() {
            self.wave_list.push(MAX_WAVE_NUM);
        }
        for i in 0..self.streams.len() {
            if save.movie_unlocked(i) {
                self.str_list.push((i, content::stream_volume(i)));
            }
        }
        Ok(())
    }

    /// `ControlDataSet(kanjilayer)` (0x004062f0), as ChooseMode enters: the
    /// frame on the kanji layer (kept after leaving) and `ClearFlg`.
    pub fn control_data_set(&mut self, x: &mut MailCtx) {
        x.views.kanji = frame(LAYSIZE);
        self.clear_flg = i32::from(x.save.save.u8(piney_data::save::offset::CLEAR_FLAG) as i8);
    }

    /// `MainAudio_control` (0x00405700). Returns -1 to leave.
    pub fn main(&mut self, x: &mut MailCtx, pad: &Pad) -> i32 {
        let k = Keys {
            push: pad.push.bits(),
            unpush: pad.unpush.bits(),
            repeat: pad.repeat.bits(),
            ok: x.save.ok(),
            cancel: x.save.cancel(),
        };
        match self.mode {
            0 => {
                self.set_data();
                self.mode = 2;
            }
            1 => {
                if self.wave_list.is_empty() {
                    self.empty(x, k);
                    return 0;
                }
                self.wave_frame(x, k);
            }
            2 => {
                let r = self.select_mode(x, k);
                if r != 0 {
                    self.mode = r;
                }
            }
            4 => {
                if self.str_list.is_empty() {
                    self.empty(x, k);
                    return 0;
                }
                self.str_frame(x, k);
            }
            5 => {
                if self.preview_count == 30 {
                    self.draw_back(x);
                    // ccSndMoviePlayer(0, NowNum), SetFrameRate(2), every
                    // task asleep, SimplePlayStream, then back.
                    if let Some(s) = self.temp_str {
                        x.req.push(Request::Movie { stream: self.streams[s].str_num, bgm: self.now_num });
                    }
                    self.fade = Some(Fade { c0: 0x8000_0000, c1: 0, cnt: 0 });
                }
                self.str_frame(x, k);
                self.preview_count -= 1;
                if self.preview_count == -1 {
                    self.mode = 4;
                    self.pushflg = 0;
                    self.pushcnt = 0;
                    x.req.push(Request::EnableReset(true));
                }
            }
            6 => {
                if self.kill_flg == 0 {
                    self.kill_flg += 1;
                } else {
                    // DelData (0x00406370), ResetData (0x00405ea0).
                    self.mask = None;
                    self.scrmask = None;
                    self.pushflg = 0;
                    self.min_max = 0;
                    self.pushcnt = 0;
                    self.list_no = 0;
                    self.mode = 0;
                    self.kill_flg = 0;
                    return -1;
                }
            }
            _ => {}
        }
        self.mode
    }

    /// An empty list waits for cancel, back to the menu.
    fn empty(&mut self, x: &mut MailCtx, k: Keys) {
        if k.push & k.cancel != 0 {
            self.mode = 2;
            x.req.push(Request::Se(Se::CLOSE));
        }
    }

    /// `SetData` (0x00405ee0): the kanji, the masks (two packets each),
    /// `ScrLayer`.
    fn set_data(&mut self) {
        let (tex, h) = self.curs.clone();
        self.mask = Some(Sprite::mask(KANJI_LAYER, tex.clone(), h, 2));
        self.scrmask = Some(Sprite::mask(SCR_LAYER, tex, h, 2));
        self.scr_view = scr_frame();
    }

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

    /// `SelectMode` (0x00406770): the menu. Returns the mode chosen.
    fn select_mode(&mut self, x: &mut MailCtx, k: Keys) -> i32 {
        let mut r = 0;
        if self.cur_repeat(k, Buttons::UP.bits()) {
            self.accs_mode -= 1;
            x.req.push(Request::Se(Se(6)));
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            self.accs_mode += 1;
            x.req.push(Request::Se(Se(6)));
        }
        if self.accs_mode <= 0 {
            self.accs_mode = 2;
        }
        if self.accs_mode >= 3 {
            self.accs_mode = 1;
        }
        if k.push & k.ok != 0 {
            if self.accs_mode == 2 && x.save.parody() {
                x.req.push(Request::Se(SE_REFUSED));
            } else {
                r = if self.accs_mode == 2 { 4 } else { 1 };
                self.list_no = 0;
                self.min_max = 0;
                x.req.push(Request::Se(Se::OPEN));
            }
        } else if k.push & k.cancel != 0 {
            r = 6;
        }
        let view = x.views.kanji;
        let st = &self.strings;
        let (snd, strm) = if self.accs_mode == 1 {
            ([b"#G".as_slice(), &st.snd_mode].concat(), st.str_mode.clone())
        } else if self.accs_mode == 2 {
            (st.snd_mode.clone(), [b"#G".as_slice(), &st.str_mode].concat())
        } else {
            return r;
        };
        x.text(KANJI_LAYER, &view, MENU[0].0, MENU[0].1, &snd, true);
        x.text(KANJI_LAYER, &view, MENU[1].0, MENU[1].1, &strm, true);
        let (cx, cy) = MENU_CURSOR[(self.accs_mode - 1) as usize];
        self.time_alpha_cur_draw(x, cx, cy);
        let st = &self.strings;
        let lines: [Vec<u8>; 2] = if self.accs_mode == 1 {
            [st.lock[5].clone(), st.lock[6].clone()]
        } else if self.clear_now(x.save) >= self.volume_num {
            [st.lock[3].clone(), st.lock[4].clone()]
        } else if x.save.parody() {
            [st.lock[9].clone(), st.lock[10].clone()]
        } else {
            [st.lock[0].clone(), [st.lock[1].as_slice(), &dec2sjis(self.volume_num, 2, 0), &st.lock[2]].concat()]
        };
        x.text(KANJI_LAYER, &view, INFO[0].0, INFO[0].1, &lines[0], true);
        x.text(KANJI_LAYER, &view, INFO[1].0, INFO[1].1, &lines[1], true);
        r
    }

    /// `saveData.clearFlag`, which the menu reads each frame.
    fn clear_now(&self, save: &SaveState) -> i32 {
        i32::from(save.save.u8(piney_data::save::offset::CLEAR_FLAG) as i8)
    }

    /// `ListMove(listNO, MinMax, Number)` (0x00408050).
    fn list_move(&mut self, x: &mut MailCtx, k: Keys, n: i32) {
        if k.push & k.cancel != 0 {
            self.pushflg = 0;
            self.pushcnt = 0;
        }
        if self.load_flag {
            return;
        }
        let mut d = 0;
        if self.cur_repeat(k, Buttons::UP.bits()) {
            x.req.push(Request::Se(Se(6)));
            self.list_no -= 1;
            d = 2;
            if self.list_no < 0 {
                self.list_no = n - 1;
                self.min_max = (n - ROWS).max(0);
            }
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            x.req.push(Request::Se(Se(6)));
            self.list_no += 1;
            d = 1;
            if self.list_no > n - 1 {
                self.list_no = 0;
                self.min_max = 0;
            }
        }
        if self.list_no - self.min_max > SHIFT_REMIT && d == 1 && n - (self.min_max + ROWS) > 0 {
            self.min_max += 1;
        }
        if d == 2 && self.min_max > 0 && self.list_no - UP_REMIT < self.min_max {
            self.min_max -= 1;
        }
    }

    /// `SetLength(MinMax, Number)` (0x00407730): the bar's (dx, dy, sx, sy).
    fn length_packet(&self, min_max: i32, n: i32) -> [f32; 4] {
        use crate::eef::{add, div, from_int, le, mul, sub};
        let travel = SCRSIZE[2];
        let mut len = mul(travel, div(from_int(ROWS << 4), from_int(n << 4)));
        if !le(len, travel) {
            len = travel;
        }
        let rest = sub(travel, len);
        let step = div(rest, from_int(n - ROWS));
        let dy =
            if self.list_no == n - 1 { add(SCRSIZE[1], rest) } else { add(SCRSIZE[1], mul(step, from_int(min_max))) };
        [sub(sub(TMPSIZE[3], SCRSIZE[3]), 3.0), dy, SCRSIZE[3], len]
    }

    fn set_length(&mut self, min_max: i32, n: i32) {
        let [dx, dy, sx, sy] = self.length_packet(min_max, n);
        let view = self.scr_view;
        let Some(m) = self.scrmask.as_mut() else { return };
        let (wu, wv, su, sv) = cell::SCROLL;
        m.wu = wu;
        m.wv = wv;
        m.wi = 1;
        m.su = su;
        m.sv = sv;
        m.sx = sx;
        m.sy = sy;
        m.dx = dx;
        m.dy = dy;
        m.make_packet(0, &view);
    }

    /// `Wavelist` (0x00407250): the music list's frame.
    fn wave_frame(&mut self, x: &mut MailCtx, k: Keys) {
        let n = self.wave_list.len() as i32;
        self.list_move(x, k, n);
        self.set_length(self.min_max, n);
        self.write_list(x);
        self.change_weve(x, k);
    }

    /// `WriteList` (0x00407db0): the music rows, the selected one's
    /// comment, the cursor.
    fn write_list(&mut self, x: &mut MailCtx) {
        let view = x.views.kanji;
        let mut cur_y = CURSOR_NONE;
        let top = self.min_max.max(0) as usize;
        for (r, &w) in self.wave_list.iter().skip(top).take(ROWS as usize).enumerate() {
            let r = r as i32;
            let y = (ROW_Y + ROW_H * r) as f32;
            let wave = &self.waves[w];
            if self.list_no == self.min_max + r {
                let s = [b"#G".as_slice(), &wave.title].concat();
                x.text(KANJI_LAYER, &view, TEXT_X, y, &s, true);
                x.text(KANJI_LAYER, &view, INFO[0].0, INFO[0].1, &wave.comment, true);
                x.text(KANJI_LAYER, &view, INFO[1].0, INFO[1].1, &wave.comment2, true);
                cur_y = ROW_Y + ROW_H * r + CURSOR_DY;
            } else {
                x.text(KANJI_LAYER, &view, TEXT_X, y, &wave.title, true);
            }
        }
        self.time_alpha_cur_draw(x, CURSOR_X, cur_y);
    }

    /// `ChangeWeve` (0x004072c0): OK on a piece not playing starts
    /// `BgmRead` (`ccSndChangeData(&Wave[n], NowNum)`) and writes `dtBgm`;
    /// the next frame finds it done (the port's load is immediate).
    fn change_weve(&mut self, x: &mut MailCtx, k: Keys) {
        if self.load_flag {
            self.load_flag = false;
            x.req.push(Request::EnableReset(true));
            return;
        }
        if k.push & k.ok != 0 {
            x.req.push(Request::Se(Se::OPEN));
            let Some(&w) = self.wave_list.get(self.list_no as usize) else { return };
            let n = self.waves[w].no.max(0) as usize;
            if n == self.now_num {
                return;
            }
            x.req.push(Request::ChangeBgm { wave: n, old: self.now_num });
            self.load_flag = true;
            self.now_num = n;
            x.save.set_dt_bgm(n as i8);
            x.req.push(Request::EnableReset(false));
        } else if k.push & k.cancel != 0 {
            self.mode = 2;
            x.req.push(Request::Se(Se::CLOSE));
        }
    }

    /// `Strlist` (0x00406ef0): the movie list's frame.
    fn str_frame(&mut self, x: &mut MailCtx, k: Keys) {
        let n = self.str_list.len() as i32;
        if self.mode == 4 {
            self.start_stream(x, k);
            if self.preview_count == -1 {
                self.list_move(x, k, n);
            }
        }
        self.set_length(self.min_max, n);
        self.write_str_list(x);
    }

    /// `StartStream` (0x00406f90): OK on a movie the save may watch fades
    /// out over 30 frames, then mode 5 plays it.
    fn start_stream(&mut self, x: &mut MailCtx, k: Keys) {
        if self.preview_count == -1 && k.push & k.ok != 0 {
            let vol = self.str_list.get(self.list_no as usize).map_or(i32::MAX, |s| s.1);
            if vol <= self.clear_flg {
                self.preview_count = 0;
                self.fade = Some(Fade { c0: 0, c1: 0x8000_0000, cnt: 0 });
                x.req.push(Request::EnableReset(false));
            }
        } else if self.preview_count == -1 && k.push & k.cancel != 0 {
            self.mode = 2;
            x.req.push(Request::Se(Se::CLOSE));
        }
        if self.preview_count >= 0 {
            self.preview_count += 1;
        }
        if self.preview_count == 30 {
            self.mode = 5;
        }
    }

    /// `WriteStrList(listNO, MinMax)` (0x00407860): the movie rows (grey,
    /// `ccSpriteColorTable[0]`, while the save's clear flag is below the
    /// movie's volume), the selected one's comment or "???", the cursor.
    fn write_str_list(&mut self, x: &mut MailCtx) {
        let view = x.views.kanji;
        let mut cur_y = CURSOR_NONE;
        let top = self.min_max.max(0) as usize;
        let rows: Vec<(usize, i32)> = self.str_list.iter().skip(top).take(ROWS as usize).copied().collect();
        for (r, (s, vol)) in rows.into_iter().enumerate() {
            let r = r as i32;
            let y = (ROW_Y + ROW_H * r) as f32;
            let strm = &self.streams[s];
            let playable = vol <= self.clear_flg;
            if self.list_no == self.min_max + r {
                let name = [b"#G".as_slice(), &strm.name].concat();
                x.text(KANJI_LAYER, &view, TEXT_X, y, &name, true);
                self.temp_str = Some(s);
                let (a, b) = if playable {
                    (strm.comment.clone(), strm.comment2.clone())
                } else {
                    (self.strings.lock[7].clone(), self.strings.lock[8].clone())
                };
                x.text(KANJI_LAYER, &view, INFO[0].0, INFO[0].1, &a, true);
                x.text(KANJI_LAYER, &view, INFO[1].0, INFO[1].1, &b, true);
                cur_y = ROW_Y + ROW_H * r + CURSOR_DY;
            } else if playable {
                x.text(KANJI_LAYER, &view, TEXT_X, y, &strm.name, true);
            } else {
                x.text_rgb(KANJI_LAYER, &view, TEXT_X, y, &strm.name, SPRITE_COLOR_TABLE[0]);
            }
        }
        self.time_alpha_cur_draw(x, CURSOR_X, cur_y);
    }

    /// `TimeAlphaCurDraw(x, y, 0)` (0x00407470).
    fn time_alpha_cur_draw(&mut self, x: &mut MailCtx, px: i32, py: i32) {
        let a = cursor_alpha(x.count, x.frame_rate);
        let view = x.views.kanji;
        let Some(m) = self.mask.as_mut() else { return };
        m.colour[3] = a;
        let (wu, wv, w, h) = cell::CURSOR;
        m.cell(wu, wv, w, h);
        m.dx = px as f32;
        m.dy = py as f32;
        m.make_packet(0, &view);
        m.send(&mut x.ctx.layers);
        m.colour[3] = 0x80;
    }

    /// `DrawBack` (0x004076e0): both masks.
    pub fn draw_back(&mut self, x: &mut MailCtx) {
        if let Some(m) = self.mask.as_mut() {
            m.send(&mut x.ctx.layers);
        }
        if let Some(m) = self.scrmask.as_mut() {
            m.send(&mut x.ctx.layers);
        }
    }

    /// The movie's fade, if one is up, drawn by `ccScFade` on the font layer.
    pub fn draw_fade(&mut self, ctx: &mut Ctx) {
        if let Some(f) = self.fade.as_mut() {
            fade::draw_colours(ctx, f.c0, f.c1, f.cnt, MOVIE_FADE);
            f.cnt += 1;
            if f.c1 == 0 && f.cnt > MOVIE_FADE {
                self.fade = None;
            }
        }
    }

    /// A piece of music's `Wave` entry.
    pub fn wave(&self, i: usize) -> Option<&WaveData> {
        self.waves.get(i)
    }
}

/// `ScrLayer`'s `SetFrame(130, 131, 155, 153, 77.5, 76.5, 1, 1)`.
fn scr_frame() -> LayerView {
    LayerView::frame(TMPSIZE[0], TMPSIZE[1], TMPSIZE[3], TMPSIZE[2], 1.0, 1.0)
}

/// Hooks for `tools/test_desktop_rs.py`: the list cursor, the scroll bar
/// and the list builds on given states, without drawing.
#[cfg(feature = "trace")]
pub mod probe {
    use super::*;

    impl Audio {
        /// A fresh cursor, `BgmRead` running or not.
        pub fn probe_reset(&mut self, loading: bool) {
            (self.list_no, self.min_max, self.pushflg, self.pushcnt, self.repeat_count) = (0, 0, 0, 0, REPEAT_FIRST);
            self.load_flag = loading;
        }

        /// ListMove over `n` entries with the pad (direct, push, unpush,
        /// repeat): (listNO, MinMax, pushflg, pushcnt, RepeatCount) after.
        pub fn probe_move(&mut self, x: &mut MailCtx, pad: [u32; 4], n: i32) -> (i32, i32, i32, i32, i16) {
            let k = Keys { push: pad[1], unpush: pad[2], repeat: pad[3], ok: x.save.ok(), cancel: x.save.cancel() };
            self.list_move(x, k, n);
            (self.list_no, self.min_max, self.pushflg, self.pushcnt, self.repeat_count)
        }

        /// SetLength(MinMax, Number) with the cursor at `list_no`: (dx, dy,
        /// sx, sy).
        pub fn probe_length(&mut self, list_no: i32, min_max: i32, n: i32) -> [f32; 4] {
            self.list_no = list_no;
            self.length_packet(min_max, n)
        }

        /// AddWaveList and AddStrList over fresh lists.
        pub fn probe_add(&mut self, save: &SaveState) -> (Vec<usize>, Vec<(usize, i32)>) {
            self.wave_list.clear();
            self.str_list.clear();
            self.add_lists(save).unwrap();
            (self.wave_list.clone(), self.str_list.clone())
        }
    }
}
