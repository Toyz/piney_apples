//! `SaveData_control` (DESKTOP.PRG, savedata.cpp; constructor 0x0040f740):
//! the Data screen, saving the game to a memory card (`docs/engine/desktop.md`,
//! "Data"). The screen asks `ccSaveSys` ([`SaveSys`]) for each step and
//! follows its `result`. It draws on three layers framed on the whole screen
//! (`SetData` 0x0040eef0): the kanji layer (127) with the cursors and the
//! question's window, layer 128 (`CurLayer`) with YES / NO, and layer 129
//! (`infoLayer`, square pixels) with the slot panel, the message and the
//! button. Texts are `ccKanji` `Init(3, 16)` with the drop shadow.

use std::rc::Rc;

use piney_data::tables::sjis::encode;
use piney_data::tables::title;
use piney_data::volume::Volume;
use piney_draw::TexRef;
use piney_input::{Buttons, Pad};

use crate::anm::Anm;
use crate::assets::{Assets, SceneFile};
use crate::card::{MemoryCard, NoCard};
use crate::eef::{add, div, from_int, lt, mul, sub, to_int};
use crate::kanji::{Kanji, dec2sjis};
use crate::layers::{KANJI_LAYER, Layers, RES_LAYER, RESK_LAYER};
use crate::mail::{AY_SQUARE, MailCtx, cursor_texture};
use crate::savesys::{INFO_SIZE, OPERATE_DESKTOP, SaveDataInfo, SaveSys, code};
use crate::sprite::Sprite;
use crate::view::LayerView;
use crate::{Request, Se};
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

/// `laysize` (x, y, hi, wid): the slot list; its x and y place the cursors.
pub const LAYSIZE: [f32; 4] = [114.0, 125.0, 175.0, 285.0];
/// `Windowsize` (x, y, hi, wid): the question's window, `HI` x `WID` tiles
/// of 8.
pub const WINDOWSIZE: [f32; 4] = [204.0, 260.0, 56.0, 160.0];
/// `Mojipos` (x, y, ...): the slot panel's first line.
pub const MOJIPOS: [f32; 4] = [177.0, 153.0, 175.0, 285.0];
/// `InfoMojipos` (x, y, ...): the message's first line.
pub const INFO_MOJIPOS: [f32; 4] = [177.0, 240.0, 56.0, 160.0];
/// `YNpos` (x, y, ...): YES / NO.
pub const YNPOS: [f32; 4] = [204.0, 260.0, 56.0, 160.0];
/// `MoveY`: the list's row pitch, 11.3.
pub const MOVE_Y: u32 = 0x4134_cccd;
/// `HI`, `WID`: the window in tiles.
pub const HI: i32 = 7;
pub const WID: i32 = 20;
/// `Slot1Max`, `Slot2Max`: save slots on a card.
pub const SLOT_MAX: i32 = 12;
/// `CurRepeat`: frames of repeat input before the first repeat, then between.
pub const REPEAT_FIRST: i16 = 30;
pub const REPEAT_NEXT: i16 = 9;
/// `CurLayer` and `infoLayer`'s priorities.
pub const CUR_LAYER: i16 = RES_LAYER;
pub const INFO_LAYER: i16 = RESK_LAYER;
/// `Back[3]`: the Data page for choosing a card slot, and for card slot 1's
/// and slot 2's lists; each is stepped once in `SetData` and drawn still.
pub const BACK_ANM: [&str; 3] = ["ANM_xddsel60", "ANM_xddsel61", "ANM_xddsel62"];
/// Sounds: the cursor moved, and the jingle for "Formatted", "Save data
/// created." and "Data saved.".
pub const SE_MOVE: Se = Se(6);
pub const SE_DONE: Se = Se(74);

/// The cells of `TEX_xddcurs1` (wu, wv in 1/16 texels; w, h).
pub mod cell {
    pub const CURSOR_BIG: (i32, i32, i32, i32) = (208, 0, 14, 11);
    pub const CURSOR_SMALL: (i32, i32, i32, i32) = (0, 0, 13, 10);
    pub const BUTTON: (i32, i32, i32, i32) = (448, 0, 16, 16);
    /// Window tiles: wv of the top, middle and bottom rows; wu of the
    /// left, middle and right columns.
    pub const WIN_ROWS: [i32; 3] = [496, 624, 752];
    pub const WIN_COLS: [i32; 3] = [0, 128, 256];
}

/// The screen's strings and `saveSysMsg`'s messages, the volume's
/// (`piney_data::tables::title`). savedata.cpp's own `STR_` pointers (main
/// 0x00378648) and SetSavePar's "Vol." (desktop.prg 0x0042f990) hold the
/// same words as the title's on all four discs.
struct Strings {
    yes: Vec<u8>,
    no: Vec<u8>,
    lv: Vec<u8>,
    alltime: Vec<u8>,
    nodeta: Vec<u8>,
    data: Vec<u8>,
    clear: Vec<u8>,
    parody: Vec<u8>,
    vol: Vec<u8>,
    /// By message number: the lines InfoWindow draws (at most four, up to
    /// the first empty one), or `None` where `saveSysMsg` is null.
    messages: Vec<Option<Vec<Vec<u8>>>>,
}

impl Strings {
    fn of(volume: Volume) -> Self {
        let messages = title::of(volume)
            .save_sys_msg
            .iter()
            .map(|m| m.map(|lines| lines.iter().take(4).take_while(|l| !l.is_empty()).map(|l| encode(l)).collect()))
            .collect();
        Strings {
            yes: encode(*title::YES),
            no: encode(*title::NO),
            lv: encode(*title::LV),
            alltime: encode(*title::ALLTIME),
            nodeta: encode(*title::NODETA),
            data: encode(*title::DATA),
            clear: encode(*title::CLEAR),
            parody: encode(*title::PARODY),
            vol: encode(*title::VOL),
            messages,
        }
    }
}

#[derive(Clone, Copy)]
struct Keys {
    push: u32,
    unpush: u32,
    repeat: u32,
    ok: u32,
    cancel: u32,
}

/// Which record SetSavePar shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rec {
    /// `&saveSys->info[i]`.
    Index(usize),
    /// `m_DammyInfoState`: the slot's record as it was before the save.
    Dummy,
}

/// `SaveData_control` (0x108 bytes).
pub struct Data {
    card: Box<dyn MemoryCard>,
    /// `saveSys`: the game's one `ccSaveSys`.
    pub save_sys: SaveSys,
    desk: Rc<SceneFile>,
    curs: (TexRef, i32),
    strings: Strings,
    /// +0x00: the question's fade-in, 0 to 128 by 8.
    pub alpha: i32,
    /// +0x04 `RecheckFlg`: a save just finished; show the record as it was
    /// while the index is read again.
    recheck_flg: bool,
    /// +0x05 `PacketFlg`: set with every question, never read.
    pub packet_flg: bool,
    /// +0x08 `NowSaveNo`, +0x18 `SubMode`, +0x28, +0x2c `Slot1Max`,
    /// `Slot2Max`: set by ResetData, never read.
    pub now_save_no: i32,
    pub sub_mode: i32,
    pub slot_max: [i32; 2],
    /// +0x0c `listNO`: the save slot under the cursor, 1-12.
    pub list_no: i32,
    /// +0x10 `TempListNo`: the slot being saved to, 0-11.
    pub temp_list_no: i32,
    /// +0x14 `SaveMode`: 0 enter, 1 choose a card slot, 2 wait for the
    /// card, 3 nothing, 4 and 5 card slot 1's and 2's list, 6 saving, 7
    /// leave, 8 reading the index again after a save.
    pub save_mode: i32,
    pushflg: i32,
    pushcnt: i32,
    /// +0x24 `Slot`: the card slot under the cursor, 0 or 1.
    pub slot: i32,
    /// +0x38 `fn`: `listNO - 1`, -1 after ResetData.
    pub file: i32,
    /// +0x3c `delcount`: frames to wait before leaving (always 0).
    pub delcount: i32,
    /// +0x40 `ProcChar`: the last message shown while saving, as a
    /// `saveSysMsg` number.
    proc_char: Option<usize>,
    /// +0x48 `dialog`: the answer under the cursor, 1 YES, 0 NO.
    pub dialog: i32,
    /// +0x4c `tempPN`: the done jingle is still to play.
    pub temp_pn: i32,
    /// +0x54 `ParFlg`: the slot panel is drawn (clear for the frame after
    /// every cursor move).
    par_flg: bool,
    repeat_count: i16,
    /// +0x84 `Back`, +0x88 `tempback` (an index into it).
    back: [Anm; 3],
    tempback: Option<usize>,
    /// +0xe8 `infoState`: the slot under the cursor.
    info_state: usize,
    /// +0xec `m_DammyInfoState`.
    dummy: [u8; INFO_SIZE],
    mask: Option<Sprite>,
    info: Option<Sprite>,
    icon: Option<Sprite>,
    cur_view: LayerView,
    info_view: LayerView,
}

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

fn se(x: &mut MailCtx, s: Se) {
    event(|| format!("[\"se\",{}]", s.0));
    x.req.push(Request::Se(s));
}

/// `ccKanji::Disp(s, -1)` on a kanji of this colour at (dx, dy).
#[allow(clippy::too_many_arguments)]
fn disp(x: &mut MailCtx, name: &str, layer: i16, view: &LayerView, dx: f32, dy: f32, rgba: [u8; 4], s: &[u8]) {
    event(|| {
        format!(
            "[\"disp\",\"{name}\",{layer},{},{},[{},{},{},{}],\"{}\"]",
            dx.to_bits(),
            dy.to_bits(),
            rgba[0],
            rgba[1],
            rgba[2],
            rgba[3],
            hex(s)
        )
    });
    let mut k = Kanji::init(3, 16);
    k.colour = rgba;
    k.dx = dx;
    k.dy = dy;
    x.ctx.disp(x.fonts, &mut k, layer, view, s, &x.names);
}

/// `ccSprite::MakePacket(0, 1)`.
fn packet(m: &mut Sprite, name: &str, view: &LayerView) {
    event(|| {
        format!(
            "[\"pkt\",\"{name}\",{},[{},{},{},{},{},{}],{},{},{},{},{}]",
            m.layer,
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
    m.make_packet(0, view);
}

/// `ccSprite::SendPacket`.
fn send(m: &mut Sprite, name: &str, layers: &mut Layers) {
    event(|| format!("[\"send\",\"{name}\"]"));
    m.send(layers);
}

impl Data {
    /// The constructor (0x0040f740), which `Desktop_control`'s builds once;
    /// no card until the runtime gives one.
    pub fn new(assets: &Assets) -> piney_data::Result<Self> {
        let mut d = Data {
            card: Box::new(NoCard),
            save_sys: SaveSys::new(assets.volume),
            desk: assets.desk.clone(),
            curs: cursor_texture(&assets.desk),
            strings: Strings::of(assets.volume),
            alpha: 0,
            recheck_flg: false,
            packet_flg: false,
            now_save_no: 0,
            sub_mode: 0,
            slot_max: [SLOT_MAX; 2],
            list_no: 1,
            temp_list_no: 0,
            save_mode: 0,
            pushflg: 0,
            pushcnt: 0,
            slot: 0,
            file: -1,
            delcount: 0,
            proc_char: None,
            dialog: 0,
            temp_pn: 1,
            par_flg: true,
            repeat_count: REPEAT_FIRST,
            back: [Anm::new(), Anm::new(), Anm::new()],
            tempback: None,
            info_state: 0,
            dummy: [0; INFO_SIZE],
            mask: None,
            info: None,
            icon: None,
            cur_view: full_frame(1.0),
            info_view: full_frame(AY_SQUARE),
        };
        d.reset_data();
        Ok(d)
    }

    /// One frame of `ccThSaveSys` when another screen (the save menu after
    /// the staff roll) started it: `MainProccess` over this screen's card.
    pub fn save_task(&mut self, save: &piney_data::save::SaveData) {
        self.save_sys.main_proccess(self.card.as_mut(), save);
    }

    /// The memory card the task talks to.
    pub fn set_card(&mut self, card: Box<dyn MemoryCard>) {
        self.card = card;
    }

    /// The card given, for the runtime to reach.
    pub fn card_mut(&mut self) -> &mut dyn MemoryCard {
        self.card.as_mut()
    }

    /// Where the title screen's load left `ccSaveSys` (`port`, `fileNum`),
    /// which the game keeps from boot: the card slot cursor starts on
    /// `port`, the save list on `file`. For before the first visit.
    pub fn set_card_position(&mut self, port: i32, file: i32) {
        self.save_sys.port = port;
        self.save_sys.file_num = file;
        self.reset_data();
    }

    /// ChooseMode entering (0x004013c0): `saveSys->StartReq(3)`, then
    /// `SetData(ccsc, kanjilayer)`.
    pub fn enter(&mut self, x: &mut MailCtx) {
        self.save_sys.start_req(OPERATE_DESKTOP);
        self.set_data(x);
    }

    /// ChooseMode leaving: `saveSys->EndReq()`.
    pub fn end_req(&mut self) {
        self.save_sys.end_req();
    }

    /// `SetData(ccsc, layer)` (0x0040eef0): the masks, the layers' frames,
    /// the three `Back` pages (stepped once).
    fn set_data(&mut self, x: &mut MailCtx) {
        let (tex, h) = self.curs.clone();
        let (wu, wv, w, ht) = cell::CURSOR_BIG;
        let mk = |layer: i16, n: usize| {
            let mut m = Sprite::mask(layer, tex.clone(), h, n);
            m.cell(wu, wv, w, ht);
            m
        };
        self.mask = Some(mk(KANJI_LAYER, 10));
        self.info = Some(mk(KANJI_LAYER, (WID * HI) as usize));
        self.icon = Some(mk(INFO_LAYER, 3));
        x.views.kanji = full_frame(1.0);
        self.info_view = full_frame(AY_SQUARE);
        self.cur_view = full_frame(1.0);
        for (a, name) in self.back.iter_mut().zip(BACK_ANM) {
            a.set(&self.desk, name);
            a.forward();
        }
    }

    /// `DelData` (0x0040f5e0).
    fn del_data(&mut self) {
        self.mask = None;
        self.info = None;
        self.icon = None;
        self.back = [Anm::new(), Anm::new(), Anm::new()];
        self.packet_flg = false;
    }

    /// `ResetData` (0x0040fab0): the cursors back, the card slot cursor on
    /// the port last used.
    fn reset_data(&mut self) {
        self.file = -1;
        self.now_save_no = 0;
        self.list_no = 1;
        self.save_mode = 0;
        self.pushflg = 0;
        self.pushcnt = 0;
        self.slot = self.save_sys.port;
        self.sub_mode = 0;
        self.slot_max = [SLOT_MAX; 2];
        self.delcount = 0;
    }

    fn keys(pad: &Pad, x: &MailCtx) -> Keys {
        Keys {
            push: pad.push.bits(),
            unpush: pad.unpush.bits(),
            repeat: pad.repeat.bits(),
            ok: x.save.ok(),
            cancel: x.save.cancel(),
        }
    }

    /// One frame: the card task's `MainProccess`, taken to run before the
    /// desktop's task (priority 20 against 33; not traced), then
    /// `MainSaveData_control` (0x0040ede0, jump table 0x0042f8d0). Returns
    /// -1 to leave.
    pub fn main(&mut self, x: &mut MailCtx, pad: &Pad) -> i32 {
        if self.save_sys.running {
            self.save_sys.main_proccess(self.card.as_mut(), &x.save.save);
        }
        let k = Self::keys(pad, x);
        match self.save_mode {
            0 => {
                self.save_mode = 1;
                self.save_sys.slot_select_req();
            }
            1 => self.slot_select(x, k),
            2 => self.slot_state_data(x, k),
            4 | 5 => self.slot_control(x, k),
            6 => self.save_data(x, k),
            7 => {
                if self.delcount < 0 {
                    self.delcount += 1;
                } else {
                    self.reset_data();
                    self.del_data();
                    return -1;
                }
            }
            8 => self.re_check_data(x, k),
            _ => {}
        }
        0
    }

    /// `CurRepeat(key)`, inlined everywhere: a push at once, then held
    /// (the repeat bits) every 30th frame, then every 9th.
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

    fn message(&self, pn: u32) -> Option<usize> {
        let n = SaveSys::message_number(pn);
        self.strings.messages[n].is_some().then_some(n)
    }

    /// `SlotSelect` (0x004106e0): choosing MEMORY CARD slot 1 or 2.
    fn slot_select(&mut self, x: &mut MailCtx, k: Keys) {
        if self.save_mode == 1 {
            let px = to_int(sub(LAYSIZE[0], 2.0));
            let py = to_int(add(14.0, add(LAYSIZE[1], from_int(36 * self.slot))));
            self.time_alpha_cur_draw(x, px, py, false);
        }
        if self.cur_repeat(k, Buttons::UP.bits()) {
            self.slot -= 1;
            se(x, SE_MOVE);
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            self.slot += 1;
            se(x, SE_MOVE);
        } else if k.push & k.ok != 0 {
            self.save_mode = 2;
            self.list_no = self.save_sys.file_num + 1;
            self.dialog = 0;
            se(x, Se::OPEN);
            self.save_sys.load_info_req(self.slot);
        } else if k.push & k.cancel != 0 {
            // No sound: ChooseMode plays 7 as the screen closes next frame.
            self.save_mode = 7;
        }
        if self.slot < 0 {
            self.slot = 1;
        }
        if self.slot >= 2 {
            self.slot = 0;
        }
        let pn = self.save_sys.result;
        if let Some(m) = self.message(pn) {
            self.info_window(x, Some(m), pn);
        }
    }

    /// `SlotStateData` (0x00410a00): waiting on the card, and its
    /// messages and questions.
    fn slot_state_data(&mut self, x: &mut MailCtx, k: Keys) {
        let pn = self.save_sys.result;
        if pn == code::BACK {
            self.save_mode = 1;
            self.dialog = 0;
            self.save_sys.slot_select_req();
        }
        if pn == code::RELOAD {
            self.save_mode = 2;
            self.list_no = 1;
            self.dialog = 0;
            self.save_sys.load_info_req(self.slot);
        }
        if pn == code::DONE {
            self.save_mode = if self.slot == 0 { 4 } else { 5 };
            self.dialog = 0;
            self.save_sys.save_select_req();
        }
        if pn & (code::ACK | code::ERROR) != 0 {
            for done in [34, 38] {
                if self.temp_pn != 0 && pn & code::MESSAGE == done {
                    se(x, SE_DONE);
                    self.temp_pn = 0;
                }
            }
            if k.push & (k.ok | k.cancel) != 0 {
                se(x, Se::OPEN);
                self.save_sys.next_proccess(0);
                self.alpha = 0;
                self.temp_pn = 1;
            }
        }
        if pn & code::QUESTION != 0 {
            self.packet_flg = true;
            self.dia_window(x, k);
            if k.push & k.cancel != 0 {
                self.dialog = 0;
                se(x, Se::CLOSE);
                self.save_sys.next_proccess(self.dialog);
            } else if k.push & k.ok != 0 {
                self.alpha = 0;
                se(x, Se::OPEN);
                self.save_sys.next_proccess(self.dialog);
            }
        }
        if let Some(m) = self.message(pn) {
            self.info_window(x, Some(m), pn);
        }
    }

    /// `Slot_control` (0x00410c50): the twelve save slots of a card.
    fn slot_control(&mut self, x: &mut MailCtx, k: Keys) {
        let pn = self.save_sys.result;
        if pn == code::BACK {
            self.save_mode = 1;
            self.dialog = 0;
            self.save_sys.slot_select_req();
        }
        if pn == code::RELOAD || pn == code::DONE {
            self.save_mode = 2;
            self.list_no = 1;
            self.dialog = 0;
            self.save_sys.load_info_req(self.slot);
        }
        if pn & (code::ACK | code::ERROR) != 0 && k.push & k.ok != 0 {
            self.alpha = 0;
            se(x, Se::OPEN);
            self.save_sys.next_proccess(0);
        }
        if pn & code::QUESTION != 0 {
            self.packet_flg = true;
            self.dia_window(x, k);
            if k.push & k.cancel != 0 {
                self.dialog = 0;
                se(x, Se::CLOSE);
                self.save_sys.next_proccess(self.dialog);
            } else if k.push & k.ok != 0 {
                se(x, Se::OPEN);
                self.save_sys.next_proccess(self.dialog);
            }
        }
        if let Some(m) = self.message(pn) {
            self.info_window(x, Some(m), pn);
        }
        if pn != code::WORKING && pn != code::SAVE_SELECT {
            return;
        }
        self.list_move(x, k);
        self.file = self.list_no - 1;
        self.info_state = self.file.clamp(0, SLOT_MAX - 1) as usize;
        if self.par_flg {
            if self.save_mode != 1 {
                self.set_save_par(x, Rec::Index(self.info_state), self.file);
            }
        } else {
            self.par_flg = true;
        }
        if k.push & k.cancel != 0 {
            self.list_no = 1;
            self.save_mode = 1;
            se(x, Se::CLOSE);
            self.save_sys.slot_select_req();
        } else if k.push & k.ok != 0 {
            self.tempback = Some(if self.slot == 0 { 1 } else { 2 });
            self.file = self.list_no - 1;
            self.temp_list_no = self.file;
            self.save_mode = 6;
            self.dialog = 0;
            self.alpha = 0;
            se(x, Se::OPEN);
            self.save_sys.save_data_req(self.file);
        }
    }

    /// `SaveData` (0x00411720): the save's questions and messages.
    fn save_data(&mut self, x: &mut MailCtx, k: Keys) {
        let pn = self.save_sys.result;
        if pn == code::BACK {
            self.save_mode = 1;
            self.dialog = 0;
            self.save_sys.slot_select_req();
        }
        if pn == code::DONE {
            self.pushflg = 0;
            self.pushcnt = 0;
            self.recheck_flg = true;
            self.save_mode = 8;
            self.dialog = 0;
            self.save_sys.load_info_req(self.slot);
        }
        if pn & (code::ACK | code::ERROR) != 0 {
            if self.temp_pn != 0 && pn & code::MESSAGE == 28 {
                se(x, SE_DONE);
                self.temp_pn = 0;
            }
            if k.push & (k.ok | k.cancel) != 0 {
                self.alpha = 0;
                self.temp_pn = 1;
                se(x, Se::OPEN);
                self.save_sys.next_proccess(0);
                self.keep_record();
            }
        }
        if pn & code::QUESTION != 0 {
            self.packet_flg = true;
            self.dia_window(x, k);
            if k.push & k.cancel != 0 {
                self.dialog = 0;
                se(x, Se::CLOSE);
                self.save_sys.next_proccess(self.dialog);
                self.keep_record();
            } else if k.push & k.ok != 0 {
                se(x, Se::OPEN);
                self.save_sys.next_proccess(self.dialog);
                self.keep_record();
            }
            self.set_save_par(x, Rec::Index(self.info_state), self.file);
        }
        if let Some(m) = self.message(pn) {
            self.proc_char = Some(m);
            self.info_window(x, Some(m), pn);
        }
        if self.recheck_flg {
            self.set_save_par(x, Rec::Dummy, self.temp_list_no);
            self.info_window(x, self.proc_char, 0);
            self.recheck_flg = false;
        }
        if pn == code::WORKING {
            self.set_save_par(x, Rec::Index(self.info_state), self.file);
        }
    }

    /// `m_DammyInfoState = *infoState`.
    fn keep_record(&mut self) {
        let at = INFO_SIZE * self.info_state;
        self.dummy.copy_from_slice(&self.save_sys.info[at..at + INFO_SIZE]);
    }

    /// `ReCheckData` (0x0040fa50): after a save, the record as it was and
    /// the last message while the index is read again, then the list.
    fn re_check_data(&mut self, x: &mut MailCtx, k: Keys) {
        self.set_save_par(x, Rec::Dummy, self.temp_list_no);
        self.info_window(x, self.proc_char, 0);
        self.slot_state_data(x, k);
    }

    /// `ListMove` (0x00411ae0): the save slot cursor, wrapping over 1-12.
    fn list_move(&mut self, x: &mut MailCtx, k: Keys) {
        if self.cur_repeat(k, k.cancel) {
            self.pushflg = 0;
            self.pushcnt = 0;
        }
        if self.cur_repeat(k, Buttons::UP.bits()) {
            self.list_no -= 1;
            if self.list_no <= 0 {
                self.list_no = SLOT_MAX;
            }
            se(x, SE_MOVE);
            self.par_flg = false;
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            self.list_no += 1;
            se(x, SE_MOVE);
            if self.list_no > SLOT_MAX {
                self.list_no = 1;
            }
            self.par_flg = false;
        }
    }

    /// `DiaWindow` (0x0040fc30): YES / NO in a window fading in. Up moves
    /// to YES, down to NO, each wrapping.
    fn dia_window(&mut self, x: &mut MailCtx, k: Keys) {
        let a = self.alpha as u8;
        if self.cur_repeat(k, Buttons::UP.bits()) {
            se(x, SE_MOVE);
            self.dialog += 1;
        } else if self.cur_repeat(k, Buttons::DOWN.bits()) {
            se(x, SE_MOVE);
            self.dialog -= 1;
        }
        if self.dialog >= 2 {
            self.dialog = 0;
        }
        if self.dialog < 0 {
            self.dialog = 1;
        }
        let mut rgba = SPRITE_COLOR_TABLE[7];
        rgba[3] = a;
        let view = self.cur_view;
        let (yes_y, no_y) = (add(16.0, YNPOS[1]), add(32.0, YNPOS[1]));
        let tx = add(50.0, YNPOS[0]);
        let sel = |s: &[u8]| [b"#G".as_slice(), s].concat();
        let (yes, no, cur_y) = if self.dialog == 0 {
            (self.strings.yes.clone(), sel(&self.strings.no), no_y)
        } else {
            (sel(&self.strings.yes), self.strings.no.clone(), yes_y)
        };
        disp(x, "diakanji[1]", CUR_LAYER, &view, tx, yes_y, rgba, &yes);
        disp(x, "diakanji[0]", CUR_LAYER, &view, tx, no_y, rgba, &no);
        let cx = to_int(add(35.0, YNPOS[0]));
        let cy = to_int(add(2.0, cur_y));
        self.time_alpha_cur_draw(x, cx, cy, true);
        if let Some(m) = self.info.as_mut() {
            m.colour[3] = a;
        }
        self.res_window(x);
        if let Some(m) = self.info.as_mut() {
            send(m, "info", &mut x.ctx.layers);
        }
        self.alpha += 8;
        if self.alpha >= 128 {
            self.alpha = 128;
        }
    }

    /// `ResWindow` (0x00412040): the question's window, `WID` x `HI` tiles
    /// of 8 at `Windowsize`, on the kanji layer.
    fn res_window(&mut self, x: &mut MailCtx) {
        let view = x.views.kanji;
        let Some(m) = self.info.as_mut() else { return };
        m.sx = 8.0;
        m.su = 8;
        m.sy = 8.0;
        m.sv = 8;
        let (w, h) = (WINDOWSIZE[3], WINDOWSIZE[2]);
        let edge = sub(w, 8.0);
        let tile = |m: &mut Sprite, col: i32, row: usize, px: i32, py: i32| {
            m.wu = if !lt(from_int(px), edge) { cell::WIN_COLS[2] } else { col };
            m.wv = cell::WIN_ROWS[row];
            m.wi = 1;
            m.dx = add(WINDOWSIZE[0], from_int(px));
            m.dy = add(from_int(py), WINDOWSIZE[1]);
            packet(m, "info", &view);
        };
        let side = |px: i32| if px < 8 { cell::WIN_COLS[0] } else { cell::WIN_COLS[1] };
        let mut px = 0;
        while lt(from_int(px), w) {
            tile(m, side(px), 0, px, 0);
            px += 8;
        }
        let mut py = 8;
        while lt(from_int(py), sub(h, 8.0)) {
            let mut px = 0;
            while lt(from_int(px), w) {
                tile(m, if px == 0 { cell::WIN_COLS[0] } else { cell::WIN_COLS[1] }, 1, px, py);
                px += 8;
            }
            py += 8;
        }
        let mut px = 0;
        while lt(from_int(px), w) {
            tile(m, side(px), 2, px, py);
            px += 8;
        }
    }

    /// `DrawCur` (0x00410110): the list cursor at `listNO`'s row.
    fn draw_cur(&mut self, x: &mut MailCtx) {
        let move_y = f32::from_bits(MOVE_Y);
        let row = add(LAYSIZE[1], mul(move_y, from_int(self.list_no)));
        let py = to_int(add(3.0, add(move_y, row)));
        self.time_alpha_cur_draw(x, to_int(LAYSIZE[0]), py, false);
    }

    /// `TimeAlphaCurDraw(x, y, Modeflg)` (0x004101b0): the big cursor
    /// blinking (held at 128 while a message or question is up or the card
    /// is busy), or the small one (the question's) no brighter than the
    /// window.
    fn time_alpha_cur_draw(&mut self, x: &mut MailCtx, px: i32, py: i32, small: bool) {
        let (mut t, half, full) = if x.frame_rate == 2 {
            ((x.count & 0x7f) as i32, 64, 128)
        } else {
            ((x.count.wrapping_mul(5) % 160) as i32, 80, 160)
        };
        if t >= half {
            t = full - t;
        }
        let (a, c) = if small {
            let top = add(48.0, from_int(t));
            (if lt(from_int(self.alpha), top) { self.alpha } else { to_int(top) }, cell::CURSOR_SMALL)
        } else {
            let r = self.save_sys.result;
            if r & code::QUESTION != 0 || r & (code::ACK | code::ERROR) != 0 || r & code::BUSY != 0 {
                t = 80;
            }
            (to_int(add(48.0, from_int(t))), cell::CURSOR_BIG)
        };
        let view = x.views.kanji;
        let Some(m) = self.mask.as_mut() else { return };
        m.colour[3] = a as u8;
        m.cell(c.0, c.1, c.2, c.3);
        m.dx = px as f32;
        m.dy = py as f32;
        packet(m, "mask", &view);
        send(m, "mask", &mut x.ctx.layers);
        m.colour[3] = 0x80;
    }

    /// `DispButton(x, y)` (0x00410430): the button to acknowledge a
    /// message, pulsing about its centre.
    fn disp_button(&mut self, x: &mut MailCtx, bx: i32, by: i32) {
        let (c, half, full) = if x.frame_rate == 2 { (x.count % 24, 12.0, 24.0) } else { (x.count % 48, 24.0, 48.0) };
        let mut c = from_int(c as i32);
        if !lt(c, half) {
            c = sub(full, c);
        }
        let bn = to_int(add(16.0, div(mul(112.0, c), half)));
        let s = add(div(sub(128.0, from_int(bn)), 600.0), f32::from_bits(0x3f66_6666));
        // 0.9 * s in double precision (fptodp, dpmul, dptofp).
        let scale = (f64::from_bits(0x3fec_cccc_cccc_cccd) * f64::from(s)) as f32;
        let view = self.info_view;
        let Some(m) = self.icon.as_mut() else { return };
        let (wu, wv, w, h) = cell::BUTTON;
        m.cell(wu, wv, w, h);
        m.dx = bx as f32;
        m.dy = by as f32;
        m.cx = -8.0;
        m.cy = -8.0;
        m.sx = mul(m.sx, scale);
        m.sy = mul(m.sy, scale);
        m.cx = mul(m.cx, scale);
        m.cy = mul(m.cy, scale);
        packet(m, "icon", &view);
        m.cx = 0.0;
        m.cy = 0.0;
        send(m, "icon", &mut x.ctx.layers);
    }

    /// `InfoWindow(msg, pn)` (0x00411d80): the message's lines (at most
    /// four, up to an empty one) at `InfoMojipos`, 16 apart; the button
    /// under them when it waits for one.
    fn info_window(&mut self, x: &mut MailCtx, msg: Option<usize>, pn: u32) {
        let view = self.info_view;
        let lines = msg.and_then(|m| self.strings.messages[m].clone()).unwrap_or_default();
        for (i, l) in lines.iter().enumerate() {
            let dy = add(INFO_MOJIPOS[1], from_int(16 * i as i32));
            disp(x, &format!("infokanji[{i}]"), INFO_LAYER, &view, INFO_MOJIPOS[0], dy, SPRITE_COLOR_TABLE[7], l);
        }
        let n = lines.len() as i32;
        if pn & (code::ACK | code::ERROR) != 0 {
            let bx = (f64::from(INFO_MOJIPOS[0]) + 120.0) as i32;
            let by = to_int(add(16.0, add(INFO_MOJIPOS[1], from_int(16 * n))));
            self.disp_button(x, bx, by);
        }
        // With a card operation running (26, 27, 32, 33, 36, 37 and not
        // 0xb000) the game also draws a ccMenuWindow page cursor at the
        // button's place. The port's card calls complete at once, so the
        // screen never sees those results.
    }

    /// `SetSavePar(info, no)` (0x00410f80): the slot panel: "DataNN", the
    /// clear or parody line, "Lv. LL", the name and "Time HHH:MM:SS" at
    /// `Mojipos`, 16 apart, in yellow for a save with clear data, red in
    /// parody mode; an empty slot shows "Unused" on the second line.
    fn set_save_par(&mut self, x: &mut MailCtx, rec: Rec, no: i32) {
        let (r, name) = self.record(rec);
        let pt = r.playtime;
        let hour = pt / 216_000;
        let rest = pt.wrapping_sub(hour.wrapping_mul(216_000));
        let minute = rest / 3600;
        let second = rest.wrapping_sub(minute.wrapping_mul(3600)) / 60;
        let st = &self.strings;
        let par = [st.data.as_slice(), &dec2sjis(no + 1, 2, 2)].concat();
        let (line, colour) = if i32::from(r.clear_flag) >= self.save_sys.volume.number() {
            ([st.vol.as_slice(), &dec2sjis(i32::from(r.clear_flag), 1, 0), &st.clear].concat(), SPRITE_COLOR_TABLE[6])
        } else if r.parody_flag != 0 {
            (st.parody.clone(), SPRITE_COLOR_TABLE[18])
        } else {
            (Vec::new(), SPRITE_COLOR_TABLE[7])
        };
        let view = self.info_view;
        let (mx, my) = (MOJIPOS[0], MOJIPOS[1]);
        if r.status != 0 {
            let lv = [st.lv.as_slice(), &dec2sjis(i32::from(r.level), 2, 2)].concat();
            let time = [
                st.alltime.as_slice(),
                b" ",
                &dec2sjis(hour, 3, 1),
                b":",
                &dec2sjis(minute, 2, 2),
                b":",
                &dec2sjis(second, 2, 2),
            ]
            .concat();
            disp(x, "kanji[3]", INFO_LAYER, &view, mx, my, colour, &par);
            disp(x, "kanji[4]", INFO_LAYER, &view, mx, add(16.0, my), colour, &line);
            disp(x, "kanji[0]", INFO_LAYER, &view, mx, add(32.0, my), colour, &lv);
            disp(x, "kanji[1]", INFO_LAYER, &view, mx, add(48.0, my), colour, &name);
            disp(x, "kanji[2]", INFO_LAYER, &view, mx, add(64.0, my), colour, &time);
        } else {
            let nodeta = st.nodeta.clone();
            disp(x, "kanji[2]", INFO_LAYER, &view, mx, add(16.0, my), SPRITE_COLOR_TABLE[7], &nodeta);
        }
    }

    /// A record and its name as `Disp` reads it: up to the NUL, which in
    /// the index may lie past the record.
    fn record(&self, rec: Rec) -> (SaveDataInfo, Vec<u8>) {
        let bytes: &[u8] = match rec {
            Rec::Index(i) => &self.save_sys.info[INFO_SIZE * i..],
            Rec::Dummy => &self.dummy,
        };
        let name = &bytes[4..];
        let name = &name[..name.iter().position(|&c| c == 0).unwrap_or(name.len())];
        (SaveDataInfo::from_bytes(bytes), name.to_vec())
    }

    /// `DrawBack` (0x0040fb00, jump table 0x0042f950): the list cursor and
    /// the page behind.
    pub fn draw_back(&mut self, x: &mut MailCtx) {
        match self.save_mode {
            0..=2 | 7 => self.draw_page(x, 0),
            4 => {
                self.draw_cur(x);
                self.draw_page(x, 1);
            }
            5 => {
                self.draw_cur(x);
                self.draw_page(x, 2);
            }
            6 | 8 => {
                self.draw_cur(x);
                if let Some(t) = self.tempback {
                    self.draw_page(x, t);
                }
            }
            _ => {}
        }
    }

    fn draw_page(&self, x: &mut MailCtx, i: usize) {
        event(|| format!("[\"anm\",{i}]"));
        self.back[i].draw(x.ctx);
    }
}

/// `SetFrame(0, 0, 512, 448, 256, 224, 1, ay)`: the whole screen.
fn full_frame(ay: f32) -> LayerView {
    LayerView::frame(0.0, 0.0, 512.0, 448.0, 1.0, ay)
}

/// A record of the screen's text, packets, sends, page draws and sounds, in
/// call order, as JSON arrays, for `tools/test_desktop_data_rs.py`.
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

/// Hooks for `tools/test_desktop_data_rs.py`: the panel, the cursors and
/// the button on given states.
#[cfg(feature = "trace")]
pub mod probe {
    use super::*;

    impl Data {
        /// The masks, as SetData leaves them.
        pub fn probe_set_data(&mut self, x: &mut MailCtx) {
            self.set_data(x);
        }

        /// SetSavePar on a record put in slot `slot` of the index.
        pub fn probe_set_save_par(&mut self, x: &mut MailCtx, rec: &[u8; INFO_SIZE], slot: usize, no: i32) {
            self.save_sys.info[INFO_SIZE * slot..INFO_SIZE * (slot + 1)].copy_from_slice(rec);
            self.set_save_par(x, Rec::Index(slot), no);
        }

        /// DrawCur at `list_no`.
        pub fn probe_draw_cur(&mut self, x: &mut MailCtx, list_no: i32, result: u32) {
            self.list_no = list_no;
            self.save_sys.result = result;
            self.draw_cur(x);
        }

        /// TimeAlphaCurDraw(px, py, small) with `alpha` and `result`.
        pub fn probe_cursor(&mut self, x: &mut MailCtx, px: i32, py: i32, small: bool, alpha: i32, result: u32) {
            self.alpha = alpha;
            self.save_sys.result = result;
            self.time_alpha_cur_draw(x, px, py, small);
        }

        /// DispButton(bx, by).
        pub fn probe_button(&mut self, x: &mut MailCtx, bx: i32, by: i32) {
            self.disp_button(x, bx, by);
        }

        /// (SaveMode, Slot, listNO, dialog, alpha, tempPN).
        pub fn probe_state(&self) -> [i32; 6] {
            [self.save_mode, self.slot, self.list_no, self.dialog, self.alpha, self.temp_pn]
        }
    }
}
