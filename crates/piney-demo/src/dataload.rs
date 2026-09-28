//! The load screen: `DataLoad_Control` (DEMO.PRG, DataControl.cpp;
//! `Main_Control` 0x004037f0), which `PlayDataLoad` runs a frame at a time once
//! the Load panel is open, over the `ccSaveSys` task
//! ([`piney_desktop::savesys::SaveSys`]): the card slot, the index, the save
//! list, then "Load this data?" and the load. Each step reads
//! `saveSys->result` once, at its start; the pushes are `ccSys` +0x2d0 against
//! the save's `assignPADok` / `assignPADcancel`. Layout from
//! `Data_Control::Init` (0x00402e40) on layer 129 (docs/engine/title.md).

use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_desktop::anm::Ctx;
use piney_desktop::eef::{add, div, from_int, lt, mul, sub, to_int};
use piney_desktop::kanji::{Kanji, Names, dec2sjis};
use piney_desktop::savesys::{INFO_SIZE, SaveDataInfo, SaveSys, code};
use piney_desktop::sprite::Sprite;
use piney_desktop::view::LayerView;
use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::Request;
use crate::dialog::{DialogAssets, KANJI_L, KANJI_PACKETS, LAYER_AY};
use crate::opening::{REPEAT_FIRST, REPEAT_NEXT, REPEAT_RESTART, se};

/// `Data_Control.m_Mask` bits: what `Main` does next frame.
pub mod mask {
    /// Ok and cancel are not read.
    pub const NO_DECIDE: i32 = 1;
    /// Up and down move `m_dialog` (the yes / no), not `m_CurNo`.
    pub const DIALOG: i32 = 2;
    /// No se for ok and cancel.
    pub const QUIET_DECIDE: i32 = 4;
    /// No se for a move.
    pub const QUIET_MOVE: i32 = 8;
    /// Up and down move nothing.
    pub const FREEZE: i32 = 0x10;
}

/// The layer `Data_Control::Init` makes for the load screen
/// (`m_pri` 0: `ccLayer::Init(129, NULL)`).
pub const LOAD_LAYER: i16 = 129;
/// `BootMem_Control::Init`'s (`ccLayer::Init(130, NULL)`).
pub const BOOT_LAYER: i16 = 130;
/// The jingle for "Load complete.".
pub const SE_DONE: i32 = 74;
/// `saveSysMsg` numbers `InfoMessage` shows with no sounds from `Main`
/// (a card operation running).
pub const QUIET_MESSAGES: [u32; 8] = [19, 20, 21, 22, 32, 33, 36, 37];
/// `TimeAlphaCurDraw`'s cells of `TEX_xgtcur00` (wu, wv, w, h): the
/// question's small cursor (mode 1) and the lists' big one (mode 0);
/// `DispButton`'s button.
pub const CURSOR_SMALL: (i32, i32, i32, i32) = (0, 0, 13, 10);
pub const CURSOR_BIG: (i32, i32, i32, i32) = (208, 0, 14, 11);
pub const BUTTON: (i32, i32, i32, i32) = (448, 0, 16, 16);
/// `SetLoadPar`: a clear save's line in `ccSpriteColorTable[6]`, a parody
/// save's in `[18]`, the rest `[7]`.
pub const COLOUR_CLEAR: usize = 6;
pub const COLOUR_PARODY: usize = 18;
pub const COLOUR_TEXT: usize = 7;
/// `@1613`'s filler between the slot number and "PARODY": two SJIS
/// full-width spaces.
pub const WIDE_SPACES: [u8; 4] = [0x81, 0x40, 0x81, 0x40];

/// A trace event, built only when the `trace` feature records it.
fn event(f: impl FnOnce() -> String) {
    #[cfg(feature = "trace")]
    trace::push(f());
    #[cfg(not(feature = "trace"))]
    let _ = f;
}

fn hex(s: &[u8]) -> String {
    s.iter().map(|b| format!("{b:02x}")).collect()
}

/// What a frame of the screen draws into.
pub struct Draw<'a> {
    pub ctx: &'a mut Ctx,
    pub assets: Option<&'a DialogAssets>,
    /// `ccSys.count`.
    pub count: u32,
    pub frame_rate: u32,
}

/// The pad as the steps read it.
#[derive(Clone, Copy, Debug)]
pub struct Keys<'a> {
    pub pad: &'a Pad,
    pub ok: u32,
    pub cancel: u32,
}

impl Keys<'_> {
    fn push(&self) -> u32 {
        self.pad.push.bits()
    }
}

/// `Data_Control` (0x8c bytes, DWARF), as far as the title uses it.
#[derive(Clone, Debug)]
pub struct DataControl {
    /// +0x00 `m_tempPN`: set when a message is acknowledged; "Load
    /// complete." plays the jingle only while it is set. Nothing
    /// initialises it (the object is `new`ed); taken as 0.
    pub temp_pn: i32,
    /// +0x04 `m_PrevFlg`: the previous volume's saves (volumes 2-4).
    pub prev_flg: i16,
    /// +0x08 `m_LoadFlg`: a save was loaded.
    pub load_flg: bool,
    /// +0x0c `m_Mask`, see [`mask`].
    pub mask: i32,
    /// +0x14 `m_SlotProc`.
    pub slot_proc: i32,
    /// +0x18 `m_Max`, +0x1a `m_Min`: `m_CurNo`'s range (it wraps).
    pub max: i16,
    pub min: i16,
    /// +0x1c `m_SW`: 1 ok, -1 cancel this frame (through `CurRepeat`).
    pub sw: i32,
    /// +0x20 `m_SlotNO`, +0x24 `m_DataNO`.
    pub slot_no: i32,
    pub data_no: i32,
    /// +0x28 `m_CurNo`, +0x2a `m_dialog` (1 yes, 0 no).
    pub cur_no: i16,
    pub dialog: i16,
    /// +0x2c .. +0x30: `CurRepeat`'s count.
    pub push_flg: i16,
    pub push_cnt: i16,
    pub repeat_count: i16,
    /// +0x34 `m_hi`, the line pitch, and the places (+0x58 ..).
    pub hi: i32,
    pub slot_x: i32,
    pub slot_y: i32,
    pub data_x: i32,
    pub data_y: i32,
    pub info_x: i32,
    pub info_y: i32,
    pub par_x: i32,
    pub par_y: i32,
    /// +0x78 `m_CurXY`.
    pub cur_xy: (i32, i32),
    /// +0x80 `m_SelInitFlg`: the next `Slot_Select` asks `SlotSelectReq`.
    pub sel_init: bool,
    /// +0x84 `m_infoState`: which record of `saveSys->info` the panel
    /// shows.
    pub info_state: usize,
    /// The layer (`m_KanjiLayer`'s priority).
    pub layer: i16,
    /// `m_KanjiPar`'s colour, as `SetLoadPar` last set it.
    pub par_colour: [u8; 4],
    /// `m_Cur` (a `ccMask`), made on the first draw.
    pub cur: Option<Sprite>,
}

impl DataControl {
    /// The fields the constructors and `Init`s set, on `layer`.
    pub fn new(layer: i16) -> Self {
        DataControl {
            temp_pn: 0,
            prev_flg: 0,
            load_flg: false,
            mask: 0,
            slot_proc: 0,
            max: 1,
            min: 0,
            sw: 0,
            slot_no: 0,
            data_no: 0,
            cur_no: 0,
            dialog: 0,
            push_flg: 0,
            push_cnt: 0,
            repeat_count: REPEAT_FIRST,
            hi: 17,
            slot_x: 0,
            slot_y: 0,
            data_x: 0,
            data_y: 0,
            info_x: 0,
            info_y: 0,
            par_x: 0,
            par_y: 0,
            cur_xy: (0, 0),
            sel_init: false,
            info_state: 0,
            layer,
            par_colour: SPRITE_COLOR_TABLE[COLOUR_TEXT],
            cur: None,
        }
    }

    /// `CurRepeat(bits)` (0x00403710): a push, or a held button's repeat
    /// (not for ok and cancel) after 30 frames and then every 7.
    pub fn cur_repeat(&mut self, pad: &Pad, bits: u32, ok: u32, cancel: u32) -> bool {
        if pad.push.bits() & bits != 0 {
            self.push_flg = 1;
            return true;
        }
        if pad.unpush.bits() & bits != 0 {
            self.repeat_count = REPEAT_FIRST;
            self.push_flg = 0;
            return false;
        }
        if pad.repeat.bits() & bits != 0 {
            if (ok | cancel) & bits != 0 {
                return false;
            }
            self.push_cnt += 1;
            if self.push_cnt == self.repeat_count {
                self.repeat_count = REPEAT_NEXT;
                self.push_cnt = REPEAT_RESTART;
                return true;
            }
        }
        false
    }

    /// `Data_Control::Main` (0x00400bc0): up / down move the cursor or the
    /// yes / no, ok and cancel into `m_SW`, with se 6 for a move, 4 for
    /// ok and 7 for cancel unless the mask quiets them; the mask is used
    /// up.
    pub fn main(&mut self, pad: &Pad, ok: u32, cancel: u32, req: &mut Vec<Request>) {
        self.sw = 0;
        let quiet = self.mask & mask::QUIET_MOVE != 0;
        let (up, down) = (Buttons::UP.bits(), Buttons::DOWN.bits());
        let moved = |req: &mut Vec<Request>| {
            if !quiet {
                event(|| format!("[\"se\",{}]", se::DATA_CURSOR));
                req.push(Request::Se(se::DATA_CURSOR));
            }
        };
        if self.mask & mask::DIALOG != 0 {
            if self.cur_repeat(pad, up, ok, cancel) {
                self.dialog += 1;
                moved(req);
            } else if self.cur_repeat(pad, down, ok, cancel) {
                self.dialog -= 1;
                moved(req);
            }
        } else if self.mask & mask::FREEZE == 0 {
            if self.cur_repeat(pad, up, ok, cancel) {
                self.cur_no -= 1;
                moved(req);
            } else if self.cur_repeat(pad, down, ok, cancel) {
                self.cur_no += 1;
                moved(req);
            }
        }
        if self.mask & mask::NO_DECIDE == 0 {
            let loud = self.mask & mask::QUIET_DECIDE == 0;
            let sound = |n: i32, req: &mut Vec<Request>| {
                if loud {
                    event(|| format!("[\"se\",{n}]"));
                    req.push(Request::Se(n));
                }
            };
            if self.cur_repeat(pad, ok, ok, cancel) {
                self.sw = 1;
                sound(se::DECIDE, req);
            } else if self.cur_repeat(pad, cancel, ok, cancel) {
                self.sw = -1;
                sound(se::DATA_CANCEL, req);
            }
        }
        self.mask = 0;
        if self.cur_no > self.max {
            self.cur_no = self.min;
        }
        if self.cur_no < self.min {
            self.cur_no = self.max;
        }
        if self.dialog >= 2 {
            self.dialog = 0;
        }
        if self.dialog < 0 {
            self.dialog = 1;
        }
    }

    fn view() -> LayerView {
        LayerView::frame(0.0, 0.0, 512.0, 384.0, 1.0, f32::from_bits(LAYER_AY))
    }

    /// `ccKanji::Disp(s, -1)` of kanji `name`, placed at (dx, dy) in
    /// `rgba`.
    fn disp(&self, d: &mut Draw, name: &str, dx: i32, dy: i32, rgba: [u8; 4], s: &[u8]) {
        let (x, y) = (from_int(dx), from_int(dy));
        event(|| {
            format!(
                "[\"disp\",\"{name}\",{},{},{},[{},{},{},{}],\"{}\"]",
                self.layer,
                x.to_bits(),
                y.to_bits(),
                rgba[0],
                rgba[1],
                rgba[2],
                rgba[3],
                hex(s)
            )
        });
        let Some(a) = d.assets else { return };
        let mut k = Kanji::init(KANJI_L, KANJI_PACKETS);
        k.colour = rgba;
        k.dx = x;
        k.dy = y;
        d.ctx.disp(&a.fonts, &mut k, self.layer, &Self::view(), s, &Names::default());
    }

    /// `m_Cur->MakePacket(0, 1)`, with the event.
    fn packet(&mut self, d: &mut Draw) {
        let layer = self.layer;
        let m = self.cur_sprite(d);
        event(|| {
            format!(
                "[\"pkt\",\"cur\",{},[{},{},{},{},{},{}],{},{},{},{},{}]",
                layer,
                m.dx.to_bits(),
                m.dy.to_bits(),
                m.sx.to_bits(),
                m.sy.to_bits(),
                m.cx.to_bits(),
                m.cy.to_bits(),
                m.su,
                m.sv,
                m.wu,
                m.wv,
                m.colour[3]
            )
        });
        if d.assets.is_some() {
            m.make_packet(0, &Self::view());
        }
    }

    /// `m_Cur`: the `ccMask(10, 0)` on `TEX_xgtcur00` (a stand-in texture
    /// when there are no assets, never drawn).
    fn cur_sprite(&mut self, d: &Draw) -> &mut Sprite {
        let layer = self.layer;
        self.cur.get_or_insert_with(|| match d.assets {
            Some(a) => Sprite::mask(layer, a.cursor.clone(), a.cursor_h, 10),
            None => Sprite::mask(layer, TexRef::Ccs { file: String::new(), texture: 0, clut: 0 }, 0, 10),
        })
    }

    /// `m_Cur->SendPacket()`.
    fn send(&mut self, d: &mut Draw) {
        if d.assets.is_some()
            && let Some(m) = self.cur.as_mut()
        {
            m.send(&mut d.ctx.layers);
        }
    }

    /// `TimeAlphaCurDraw(x, y, Modeflg)` (0x00401c40): the question's small
    /// cursor (1), or the lists' big one (0), which stays at alpha 128
    /// while a question or message is up; else it blinks, 48 to 128.
    pub fn time_alpha_cur_draw(&mut self, d: &mut Draw, x: i32, y: i32, small: bool, result: u32) {
        let (mut t, half, full) = if d.frame_rate == 2 {
            ((d.count & 0x7f) as i32, 64, 128)
        } else {
            ((d.count.wrapping_mul(4) % 160) as i32, 80, 160)
        };
        if t >= half {
            t = full - t;
        }
        let c = if small {
            CURSOR_SMALL
        } else {
            if result & code::QUESTION != 0 || result & (code::ACK | code::ERROR) != 0 {
                t = 80;
            }
            CURSOR_BIG
        };
        let alpha = to_int(add(48.0, from_int(t)));
        let m = self.cur_sprite(d);
        m.colour[3] = alpha as u8;
        m.cell(c.0, c.1, c.2, c.3);
        m.dx = from_int(x);
        m.dy = from_int(y);
        self.packet(d);
        self.send(d);
        self.cur_sprite(d).colour[3] = 0x80;
    }

    /// `DispButton(dx, dy)` (0x00401990): the button to acknowledge a
    /// message, pulsing about its centre.
    pub fn disp_button(&mut self, d: &mut Draw, bx: i32, by: i32) {
        let (c, half, full) = if d.frame_rate == 2 { (d.count % 24, 12.0, 24.0) } else { (d.count % 48, 24.0, 48.0) };
        let mut c = from_int(c as i32);
        if !lt(c, half) {
            c = sub(full, c);
        }
        let bn = to_int(add(16.0, div(mul(112.0, c), half)));
        let s = add(div(sub(128.0, from_int(bn)), 600.0), f32::from_bits(0x3f66_6666));
        // 0.9 * s in double precision (fptodp, dpmul, dptofp).
        let scale = (f64::from_bits(0x3fec_cccc_cccc_cccd) * f64::from(s)) as f32;
        let m = self.cur_sprite(d);
        let (wu, wv, w, h) = BUTTON;
        m.cell(wu, wv, w, h);
        m.dx = from_int(bx);
        m.dy = from_int(by);
        m.cx = -8.0;
        m.cy = -8.0;
        m.sx = mul(m.sx, scale);
        m.sy = mul(m.sy, scale);
        m.cx = mul(m.cx, scale);
        m.cy = mul(m.cy, scale);
        self.packet(d);
        let m = self.cur_sprite(d);
        m.cx = 0.0;
        m.cy = 0.0;
        self.send(d);
    }

    /// `InfoMessage(pn, FLG)` (0x00401e80): `saveSysMsg[pn & 0xfff]`'s
    /// lines (four, five with FLG, up to an empty one) at (Info_X, Info_Y),
    /// a line apart, and the button under them for a message to
    /// acknowledge; a card operation's messages quiet `Main`'s sounds.
    pub fn info_message(&mut self, d: &mut Draw, pn: u32, flg: bool) {
        let m = pn & 0xfff;
        if QUIET_MESSAGES.contains(&m) {
            self.mask |= mask::QUIET_MOVE | mask::QUIET_DECIDE;
        }
        let ack = pn & (code::ACK | code::ERROR) != 0;
        if ack {
            self.mask |= mask::QUIET_MOVE;
        }
        let max = if flg { 5 } else { 4 };
        let lines: Vec<Vec<u8>> = d.assets.and_then(|a| a.message(pn)).map(|l| l.to_vec()).unwrap_or_default();
        let mut i = 0;
        for line in lines.iter().take(max) {
            let y = self.info_y + self.hi * i;
            self.disp(d, &format!("info[{i}]"), self.info_x, y, SPRITE_COLOR_TABLE[COLOUR_TEXT], line);
            i += 1;
        }
        if ack {
            // (int)(Info_X + 7.5 * m_hi) in double; the line after the last.
            let bx = (f64::from(self.info_x) + 7.5 * f64::from(self.hi)) as i32;
            let by = self.hi + self.info_y + self.hi * i;
            self.disp_button(d, bx, by);
        }
    }

    /// `YesNoDialogue(dialog, FLG)` (0x004020f0): YES and NO at lines 3
    /// and 4 of the message (5 and 6 with FLG), the chosen one "#G", the
    /// small cursor before it.
    pub fn yes_no_dialogue(&mut self, d: &mut Draw, dialog: i16, flg: bool) {
        let (yes_row, no_row) = if flg { (5, 6) } else { (3, 4) };
        let (yes, no, row) = match d.assets {
            Some(a) => {
                let mut chosen = a.highlight.clone();
                if dialog == 0 {
                    chosen.extend_from_slice(&a.no);
                    (a.yes.clone(), chosen, no_row)
                } else {
                    chosen.extend_from_slice(&a.yes);
                    (chosen, a.no.clone(), yes_row)
                }
            }
            None => (Vec::new(), Vec::new(), if dialog == 0 { no_row } else { yes_row }),
        };
        let white = SPRITE_COLOR_TABLE[COLOUR_TEXT];
        let y = |r: i32| self.info_y + self.hi * r;
        self.disp(d, "dia[1]", self.info_x, y(yes_row), white, &yes);
        self.disp(d, "dia[0]", self.info_x, y(no_row), white, &no);
        self.cur_xy = (self.info_x - 12, y(row) + 3);
        let (x, cy) = self.cur_xy;
        self.time_alpha_cur_draw(d, x, cy, true, 0);
    }

    /// Back to choosing the card slot: the result 0 branch every step
    /// shares.
    fn back_to_slots(&mut self, sys: &mut SaveSys) {
        self.slot_proc = 0;
        self.dialog = 0;
        self.cur_no = sys.port as i16;
        self.max = 1;
        self.min = 0;
        sys.slot_select_req();
    }

    /// Read the slot's index again (`LoadInfoReq`; `LoadInfoPrevReq` for
    /// the previous volume, never in Infection).
    fn reload(&mut self, sys: &mut SaveSys) {
        self.slot_proc = 1;
        self.dialog = 0;
        sys.load_info_req(self.slot_no);
    }

    /// The question branch every step shares: cancel is no, ok answers
    /// `m_dialog`, then `YesNoDialogue(m_dialog, 0)`.
    fn question(&mut self, d: &mut Draw, k: Keys, sys: &mut SaveSys) {
        self.mask |= mask::DIALOG;
        if k.push() & k.cancel != 0 {
            self.dialog = 0;
            sys.next_proccess(0);
        } else if k.push() & k.ok != 0 {
            sys.next_proccess(i32::from(self.dialog));
        }
        self.yes_no_dialogue(d, self.dialog, false);
    }
}

/// `DataLoad_Control` (0x90 bytes).
#[derive(Clone, Debug)]
pub struct DataLoad {
    pub data: DataControl,
    /// +0x8c `m_DataAct` (0; nothing reads it).
    pub data_act: i32,
}

impl Default for DataLoad {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLoad {
    /// The constructor (0x004038f0) as far as `Init` needs a `ccSaveSys`;
    /// [`DataLoad::init`] is the rest.
    pub fn new() -> Self {
        let mut data = DataControl::new(LOAD_LAYER);
        data.hi = 17;
        data.slot_x = 75;
        data.slot_y = 17 * 7;
        data.data_x = 75;
        data.data_y = 17 * 6;
        data.info_x = 165;
        // (int)(m_hi * 13.5) in double.
        data.info_y = (17.0f64 * 13.5) as i32;
        data.par_x = 165;
        data.par_y = 17 * 6;
        DataLoad { data, data_act: 0 }
    }

    /// `NextDataLoad_Control`'s constructor (0x00403af0) as far as `Init`
    /// needs a `ccSaveSys`: the same control ([`DataLoad::init_next`] is
    /// the rest).
    pub fn new_next() -> Self {
        let mut d = Self::new();
        d.data.prev_flg = 1;
        d
    }

    /// The rest of `NextDataLoad_Control`'s constructor: `Init`, then
    /// `m_PrevFlg` 1, the previous volume's saves (volumes 2-4).
    pub fn init_next(&mut self, sys: &mut SaveSys) {
        self.init(sys);
        self.data.prev_flg = 1;
    }

    /// `Data_Control::Init` (0x00402e40), which the constructor calls
    /// (`ccOpening_Control::Init` builds the control before its
    /// `saveSys->StartReq(1)`): the cursor on the card slot and save the
    /// `ccSaveSys` last used, and `SlotSelectReq`.
    pub fn init(&mut self, sys: &mut SaveSys) {
        let d = &mut self.data;
        d.cur_xy = (0, 0);
        d.prev_flg = 0;
        d.load_flg = false;
        d.cur_no = sys.port as i16;
        d.data_no = sys.file_num;
        d.max = 1;
        d.min = 0;
        d.dialog = 0;
        d.push_flg = 0;
        d.push_cnt = 0;
        d.repeat_count = REPEAT_FIRST;
        d.slot_proc = 0;
        d.sel_init = true;
        sys.slot_select_req();
    }

    /// `Main_Control` (0x004037f0): -1 cancelled from the slot choice, 1 a
    /// save loaded, else 0.
    pub fn main_control(&mut self, d: &mut Draw, k: Keys, sys: &mut SaveSys, req: &mut Vec<Request>) -> i32 {
        self.data.main(k.pad, k.ok, k.cancel, req);
        match self.data.slot_proc {
            0 => {
                self.slot_select(d, sys);
                if self.data.sw == -1 {
                    return -1;
                }
            }
            1 => {
                self.slot_state_data(d, k, sys, req);
            }
            2 => {
                self.write_data_list(d, sys);
                self.data_select(d, k, sys);
            }
            3 => {
                self.write_data_list(d, sys);
                self.load_data(d, k, sys, req);
                if self.data.load_flg {
                    return 1;
                }
            }
            _ => {}
        }
        0
    }

    /// `Slot_Select` (0x00400e20): "MEMORY CARD" / "slot 1" and "slot 2",
    /// the chosen one "#G"; ok reads that card's index.
    fn slot_select(&mut self, dr: &mut Draw, sys: &mut SaveSys) {
        let d = &mut self.data;
        if d.sel_init {
            sys.slot_select_req();
            d.sel_init = false;
        }
        let s = sys.result;
        let white = SPRITE_COLOR_TABLE[COLOUR_TEXT];
        for i in 0..2i32 {
            let mut card = Vec::new();
            let mut slot = Vec::new();
            if let Some(a) = dr.assets {
                if i32::from(d.cur_no) == i {
                    card.extend_from_slice(&a.highlight);
                    slot.extend_from_slice(&a.highlight);
                }
                card.extend_from_slice(&a.memorycard);
                slot.extend_from_slice(if i == 0 { &a.slot1 } else { &a.slot2 });
            }
            let y = d.slot_y + i * 3 * d.hi;
            d.disp(dr, &format!("mc[{i}]"), d.slot_x - 7, y - d.hi, white, &card);
            d.disp(dr, &format!("slot[{i}]"), d.slot_x, y, white, &slot);
        }
        d.cur_xy = (d.slot_x - 12, d.slot_y + i32::from(d.cur_no) * 3 * d.hi + 3);
        let (x, y) = d.cur_xy;
        d.time_alpha_cur_draw(dr, x, y, false, sys.result);
        if d.sw == 1 {
            d.slot_proc = 1;
            d.slot_no = i32::from(d.cur_no);
            d.cur_no = 0;
            d.max = 11;
            d.min = 0;
            sys.load_info_req(d.slot_no);
        }
        d.info_message(dr, s, false);
    }

    /// `SlotStateData` (0x00401170): waits on the index read; 1 goes on
    /// to the save list (`LoadSelectReq`).
    fn slot_state_data(&mut self, dr: &mut Draw, k: Keys, sys: &mut SaveSys, req: &mut Vec<Request>) {
        let d = &mut self.data;
        let s = sys.result;
        if s == 0 {
            d.back_to_slots(sys);
        }
        if s == code::RELOAD {
            d.cur_no = 0;
            d.reload(sys);
        }
        if s == code::DONE {
            d.slot_proc = 2;
            d.dialog = 0;
            sys.load_select_req();
        }
        if s & (code::ACK | code::ERROR) != 0 {
            d.mask |= mask::DIALOG;
            if d.temp_pn != 0 && s & code::MESSAGE == 23 {
                event(|| format!("[\"se\",{SE_DONE}]"));
                req.push(Request::Se(SE_DONE));
                d.temp_pn = 0;
            }
            if k.push() & (k.ok | k.cancel) != 0 {
                sys.next_proccess(0);
                d.temp_pn = 1;
            }
        }
        if s & code::QUESTION != 0 {
            d.question(dr, k, sys);
        }
        d.info_message(dr, s, false);
    }

    /// `Data_Select` (0x00401390): the twelve saves; ok on a used one asks
    /// to load it (`LoadDataReq`), cancel goes back to the card slots.
    fn data_select(&mut self, dr: &mut Draw, k: Keys, sys: &mut SaveSys) {
        let d = &mut self.data;
        let s = sys.result;
        if s == 0 {
            d.back_to_slots(sys);
        }
        if s == code::RELOAD || s == code::DONE {
            d.reload(sys);
        }
        if s & (code::ACK | code::ERROR) != 0 {
            d.mask |= mask::DIALOG;
            if k.push() & k.ok != 0 {
                if matches!(s & code::MESSAGE, 5 | 6) {
                    d.slot_proc = 1;
                }
                sys.next_proccess(0);
            }
        }
        if s & code::QUESTION != 0 {
            d.question(dr, k, sys);
        }
        d.info_message(dr, s, false);
        if s == code::DONE || s == piney_desktop::savesys::LOAD_SELECT {
            d.data_no = i32::from(d.cur_no);
            d.info_state = d.data_no.clamp(0, 11) as usize;
            if k.push() & k.cancel != 0 {
                d.dialog = 0;
                d.cur_no = sys.port as i16;
                d.slot_proc = 0;
                sys.slot_select_req();
                d.max = 1;
                d.min = 0;
            } else if k.push() & k.ok != 0 {
                d.mask |= mask::FREEZE;
                if d.prev_flg == 0 && sys.record(d.info_state).status != 0 {
                    d.data_no = i32::from(d.cur_no);
                    d.slot_proc = 3;
                    d.dialog = 0;
                    sys.load_data_req(d.data_no);
                }
            }
            if s != code::DONE && d.slot_proc != 0 {
                let (st, no) = (d.info_state, d.data_no);
                Self::set_load_par(d, dr, sys, st, no);
            }
        }
    }

    /// `LoadData` (0x00401750): "Load this data?", the load, and "Load
    /// complete." acknowledged sets `m_LoadFlg`.
    fn load_data(&mut self, dr: &mut Draw, k: Keys, sys: &mut SaveSys, req: &mut Vec<Request>) {
        let d = &mut self.data;
        d.mask |= mask::FREEZE;
        let s = sys.result;
        if s == 0 {
            d.back_to_slots(sys);
        }
        if s == code::DONE {
            d.slot_proc = 2;
            if d.dialog != 0 {
                d.cur_no = 0;
                d.dialog = 0;
            }
        }
        if s & (code::ACK | code::ERROR) != 0 {
            if d.temp_pn != 0 && s & code::MESSAGE == 23 {
                event(|| format!("[\"se\",{SE_DONE}]"));
                req.push(Request::Se(SE_DONE));
                d.temp_pn = 0;
            }
            if k.push() & (k.ok | k.cancel) != 0 {
                if s & code::MESSAGE == 23 {
                    d.load_flg = true;
                }
                d.temp_pn = 1;
                sys.next_proccess(0);
            }
            d.mask |= mask::DIALOG;
        }
        if s & code::QUESTION != 0 {
            d.question(dr, k, sys);
            let (st, no) = (d.info_state, d.data_no);
            Self::set_load_par(d, dr, sys, st, no);
        }
        d.info_message(dr, s, false);
        if s == code::WORKING {
            let (st, no) = (d.info_state, d.data_no);
            Self::set_load_par(d, dr, sys, st, no);
        }
    }

    /// `WriteDataList` (0x00402360): "Data01" .. "Data12", the one under
    /// the cursor "#G", and the big cursor; nothing while no card is in
    /// (messages 5 and 6).
    fn write_data_list(&mut self, dr: &mut Draw, sys: &SaveSys) {
        let d = &mut self.data;
        if matches!(sys.result & code::MESSAGE, 5 | 6) {
            return;
        }
        let white = SPRITE_COLOR_TABLE[COLOUR_TEXT];
        for s in 0..12i32 {
            let mut text = Vec::new();
            if let Some(a) = dr.assets {
                if s == i32::from(d.cur_no) {
                    text.extend_from_slice(&a.highlight);
                }
                text.extend_from_slice(&a.data);
                text.extend_from_slice(&dec2sjis(s + 1, 2, 2));
            }
            d.disp(dr, &format!("data[{s}]"), d.data_x, d.data_y + d.hi * s, white, &text);
        }
        d.cur_xy = (d.data_x - 14, d.data_y + d.hi * i32::from(d.cur_no) + 3);
        let (x, y) = d.cur_xy;
        d.time_alpha_cur_draw(dr, x, y, false, sys.result);
    }

    /// `SetLoadPar(info, no)` (0x00402570): the save's panel - "DataNN"
    /// (with "Vol.N CLEAR" in yellow for a cleared save, the parody mark in
    /// red), "Lv.", the name, the play time; "no data" for an empty slot.
    fn set_load_par(d: &mut DataControl, dr: &mut Draw, sys: &SaveSys, state: usize, no: i32) {
        let bytes = &sys.info[INFO_SIZE * state..];
        let r = SaveDataInfo::from_bytes(bytes);
        let name = &bytes[4..];
        let name = &name[..name.iter().position(|&c| c == 0).unwrap_or(name.len())];
        let pt = r.playtime;
        let hour = pt / 216_000;
        let rest = pt.wrapping_sub(hour.wrapping_mul(216_000));
        let minute = rest / 3600;
        let second = rest.wrapping_sub(minute.wrapping_mul(3600)) / 60;
        let Some(a) = dr.assets else {
            // Only the colour matters without the texts.
            return;
        };
        let num = dec2sjis(no + 1, 2, 2);
        let (line, colour) = if i32::from(r.clear_flag) >= sys.volume.number() {
            let l =
                [a.data.as_slice(), &num, b"  ", &a.vol, &dec2sjis(i32::from(r.clear_flag), 1, 0), &a.clear].concat();
            (l, SPRITE_COLOR_TABLE[COLOUR_CLEAR])
        } else if r.parody_flag != 0 {
            ([a.data.as_slice(), &num, &WIDE_SPACES, &a.parody].concat(), SPRITE_COLOR_TABLE[COLOUR_PARODY])
        } else {
            ([a.data.as_slice(), &num].concat(), SPRITE_COLOR_TABLE[COLOUR_TEXT])
        };
        d.par_colour = colour;
        if r.status != 0 {
            let lv = [a.lv.as_slice(), &dec2sjis(i32::from(r.level), 2, 2)].concat();
            let time = [
                a.alltime.as_slice(),
                b" ",
                &dec2sjis(hour, 3, 1),
                b":",
                &dec2sjis(minute, 2, 2),
                b":",
                &dec2sjis(second, 2, 2),
            ]
            .concat();
            d.disp(dr, "par[3]", d.par_x, d.par_y, colour, &line);
            d.disp(dr, "par[0]", d.par_x, d.par_y + d.hi, colour, &lv);
            d.disp(dr, "par[1]", d.par_x, d.par_y + 2 * d.hi, colour, name);
            d.disp(dr, "par[2]", d.par_x, d.par_y + 3 * d.hi, colour, &time);
        } else {
            let white = SPRITE_COLOR_TABLE[COLOUR_TEXT];
            d.par_colour = white;
            d.disp(dr, "par[0]", d.par_x, d.par_y + d.hi, white, &a.nodeta);
        }
    }
}

/// A record of the screen's texts, cursor packets and sounds in call
/// order, as JSON arrays, for `tools/test_demo_rs.py`.
#[cfg(feature = "trace")]
pub mod trace {
    use std::cell::RefCell;

    thread_local! {
        static LOG: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    }

    pub fn start() {
        LOG.with(|l| *l.borrow_mut() = Some(Vec::new()));
    }

    pub fn stop() -> Vec<String> {
        LOG.with(|l| l.borrow_mut().take().unwrap_or_default())
    }

    pub(super) fn push(e: String) {
        LOG.with(|l| {
            if let Some(v) = l.borrow_mut().as_mut() {
                v.push(e);
            }
        });
    }
}
