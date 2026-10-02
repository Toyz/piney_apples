//! `ccMenuCtrl` (0x240 bytes) and its task `ccThMenu` (gcmn 0x005280d0):
//! the state, opening and closing menus, and the list cursor.

use piney_desktop::SaveState;
use piney_desktop::kanji::Fonts;
use piney_desktop::message::{MsgDraw, MsgWindow};
use piney_desktop::noiz::Noiz;
use piney_input::Pad;

use crate::Request;
use crate::spr::{Obj, Packet, Spr};
use crate::tables::{MenuList, Texts};
use crate::window::Cursor;
use crate::world::World;

/// `CheckMenuType` while a menu opens or closes (`menu != menuNext`).
pub const MENU_CHANGING: i32 = 88;
/// Menus by number (the jump table at gcmn 0x006e0300, by menu + 1).
pub const MENU_PERSONAL_TOWN: i16 = 0;
pub const MENU_PERSONAL_FIELD: i16 = 1;
pub const MENU_PERSONAL_DUNGEON: i16 = 2;
pub const MENU_CHAT: i16 = 3;
pub const MENU_OPTION: i16 = 12;
/// The sounds the menus make.
pub const SE_OPEN: i32 = 16;
pub const SE_MOVE: i32 = 17;
pub const SE_OK: i32 = 18;
pub const SE_BACK: i32 = 19;
pub const SE_BUZZ: i32 = 20;
/// The tutorial menus open without sound 16 (`OpenMenu` 0x00525c54).
pub const SILENT_OPEN: [i16; 6] = [75, 78, 80, 83, 84, 85];

/// newlib's `rand()` (main 0x00133a38): a 64-bit LCG from 1, bits 32-62.
pub fn newlib_rand() -> impl FnMut() -> i32 {
    let mut next: u64 = 1;
    move || {
        next = next.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        ((next >> 32) & 0x7fff_ffff) as i32
    }
}

/// What the frame's drawing is, in `Disp`'s send order.
#[derive(Clone, Debug)]
pub enum Draw {
    /// `SendPacket` of a sprite's queue.
    Send(Vec<Packet>),
    /// A `ccKanji`'s queue, with the text last `Extract`ed into it.
    Kanji { obj: Obj, text: Vec<u8>, packets: Vec<Packet> },
    /// `ccMsg->Disp()`: the message window's cells and texts.
    Msg(Vec<MsgDraw>),
    /// `ccKanji::Disp(str, count, 1, 1)` on a menu kanji: text drawn at
    /// once (`count` glyphs, -1 all; `kt` 2 the large font).
    Text { obj: Obj, text: Vec<u8>, dx: f32, dy: f32, rgba: [u8; 4], count: i32, kt: i32 },
    /// `menuFade->SendPacket()`: an element of the menu's screen fader
    /// over the whole frame, `c0` to `c1` at `cnt` of `tcnt`.
    Fade { c0: u32, c1: u32, cnt: i16, tcnt: i16 },
    /// `ccHackMenu::Draw`'s animations on its own layer
    /// ([`crate::menus::hack`]).
    Hack(Box<crate::menus::hack::HackDraw>),
    /// `ccNoiz::Draw` on the noise's own layer
    /// ([`piney_desktop::noiz::NOIZ_LAYER`]), in the GS's order.
    Noiz(Vec<piney_draw::Cmd>),
}

/// What a handler leaves for the rest of the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Return to the task loop: its `Disp` ends the frame.
    Done,
    /// The handler called `Disp` and breathed itself (closing a menu);
    /// the rest runs next frame as `Cont`.
    Breathed(Cont),
}

/// The rest of a handler after its own breath: the tail of `CloseMenu`
/// (0x00525e20) or of its inlined copies (`still = 0` and the flips back
/// on when it woke the tasks; `firstTime = 0`), the cursors reset when
/// `ChangeMenu` closed instead, then what the handler does after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cont {
    pub woke: bool,
    pub cursors: bool,
    pub after: After,
}

/// What a handler does after its close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
    Nothing,
    /// SystemMenu's help line.
    SystemHelp,
    /// SystemMenu after `ChangeMenu`: Equipment's first-time flags, then
    /// the help line.
    SystemChanged,
    /// TargetMenu after a skill or an item is used.
    Use(UseTail),
    /// ChatMenu (and ChatMenu1 and 3, by the menu still open) after its
    /// close: the help line (ChatMenu's while its `proccess` is below 50,
    /// ChatMenu3's while it is 1).
    ChatHelp,
    /// The Chaos Gate's menus after their own breaths.
    Gate(crate::menus::gate::GateAfter),
    /// ChatMenuT's order: `n` more `Disp; Breath` passes, then the order.
    ChatT(u8),
    /// A close that may take the panels out too (GetItemMenu's).
    Panels(bool),
    /// ReplaceItemMenu after its close.
    Replace(crate::menus::getitem::ReplaceTail),
    /// The talk and shop menus' pages ([`crate::talk::tail`]).
    Talk(crate::talk::Tail),
    /// The PERSONAL pages' own frames after a breath (the
    /// handler resumes where it breathed; see [`crate::menus::personal`]).
    Pers(crate::menus::personal::Tail),
    /// `GtHackMenu`'s own frames ([`crate::menus::hack`]): it goes on
    /// before the task's loop next frame.
    Hack(crate::menus::hack::Resume),
    /// `DataDrainMenu`'s own frames ([`crate::menus::drain`]).
    Drain(crate::menus::drain::Resume),
    /// An OPTION page's own breaths (the Controller's load and close): it
    /// goes on before the task's loop next frame.
    Option(crate::menus::option::Resume),
    /// Inside `ccUseItemRequest` ([`crate::menus::useitem`]).
    Item,
    /// `StreamMenu`'s own frames ([`crate::menus::stream`]).
    Stream(crate::menus::stream::Resume),
    /// The spring's menus after a breath ([`crate::menus::fountain`]).
    Fountain(crate::menus::fountain::Tail),
}

/// The frames after TargetMenu's use (0x005326d8 on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseTail {
    /// The tasks were woken for the use: the flips back on.
    Woken,
    /// Until `ccSkillCheck(plw) < 2`; then the target dropped.
    Wait,
    /// The frames after (the first of six already breathed).
    Settle(u8),
    /// The window shut; the flips back on if it woke the tasks.
    Closed(bool),
}

impl Cont {
    pub fn close(woke: bool) -> Cont {
        Cont { woke, cursors: false, after: After::Nothing }
    }
}

/// Where the task is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boot {
    /// `new ccMenuCtrl` done; breathing until party slot 0 is filled.
    WaitParty,
    Running,
}

/// The per-frame context of the menu code.
pub struct Ctx<'a> {
    pub pad: &'a Pad,
    /// The world as the runtime gave it, with what the menus change of it
    /// this frame (the player's SP) written back.
    pub world: World,
    pub save: &'a mut SaveState,
    pub texts: &'a Texts,
    pub fonts: &'a Fonts,
    pub req: &'a mut Vec<Request>,
    /// `ccSys.count`.
    pub count: u32,
    pub frame_rate: u32,
    /// `cmndTarget` and `cmndTargetPrev` as this frame leaves them: the
    /// world's at the start, then what the menus' `ccChangeCmndTarget`
    /// calls make of them (each also a [`Request`] for the runtime).
    pub target: Option<crate::world::CharInfo>,
    pub target_prev: Option<crate::world::CharInfo>,
}

impl Ctx<'_> {
    pub fn se(&mut self, n: i32) {
        self.req.push(Request::Se(n));
    }
    /// `ccSeOnNote(n, note)`.
    pub fn se_note(&mut self, n: i32, note: i32) {
        self.req.push(Request::SeNote { n, note });
    }
    /// `ccSys.pad[0].push & saveData.assignPAD*`.
    pub fn pushed_ok(&self) -> bool {
        self.pad.push.bits() & self.save.ok() != 0
    }
    pub fn pushed_cancel(&self) -> bool {
        self.pad.push.bits() & self.save.cancel() != 0
    }
    /// `ccChangeCmndTarget(c)` (gcmn 0x005198c0): a new target, the old
    /// one kept as the previous. `c` is taken to pass `ccCheckTarget`.
    pub fn change_target(&mut self, c: Option<&crate::world::CharInfo>) {
        let cur = self.target.as_ref().map_or(0, |t| t.handle);
        let new = c.map_or(0, |t| t.handle);
        if new != cur {
            self.target_prev = self.target.take();
            self.target = c.cloned();
        }
        self.req.push(match c {
            Some(t) => Request::Target(t.handle),
            None => Request::TargetClear,
        });
    }

    /// `saveData->assignPAD[i]` (+0x8404 + 2 i): 0 act, 1 personal menu,
    /// 2 chat, 3 option, 4 map, 5 ok, 6 cancel.
    pub fn assign(&self, i: usize) -> u32 {
        self.save.save.i16(0x8404 + 2 * i) as u16 as u32
    }
}

/// `ccMenuCtrl`.
pub struct MenuCtrl {
    pub boot: Boot,
    /// +0x00 `panelBure[3]`: a party panel's shake, frames left.
    pub panel_bure: [u8; 3],
    /// +0x03 `panelFlash[3]`.
    pub panel_flash: [u8; 3],
    /// +0x06 `menu`, +0x08 `menuNext`: -1 none.
    pub menu: i16,
    pub menu_next: i16,
    /// +0x0a `menuStatus`: 0 shut, 1 opening, 2 open, 3 closing.
    pub menu_status: i16,
    /// +0x0c `panelStatus`: the party panels (same states).
    pub panel_status: i16,
    /// +0x0e `bgStatus`: the dim behind a menu.
    pub bg_status: i16,
    /// +0x10 `mapStatus`: the minimap.
    pub map_status: i16,
    /// +0x12 `drainStatus`: the bracelet's gauge.
    pub drain_status: i16,
    /// +0x14 `openReqNum`: a menu asked for, -1 none (0x1000: no dim).
    pub open_req: i16,
    /// +0x16 `mode`.
    pub mode: i16,
    /// +0x18 `still`: the other tasks sleep and their layers are frozen.
    pub still: i16,
    /// +0x1a `proccess`, +0x1c `waitCount`: the open menu's step.
    pub proccess: i16,
    pub wait_count: i16,
    /// +0x1e `exceptionDisp`: the menu draws its own page.
    pub exception_disp: i16,
    /// +0x20 `battleCnt`: the battle banner's clock.
    pub battle_cnt: i16,
    pub teach_cnt: i16,
    /// +0x24 `cursolInit`, +0x26 `cursolOff`.
    pub cursol_init: i16,
    pub cursol_off: i16,
    /// +0x28 `fade`, +0x2a `bgCol` (8 after the constructor).
    pub fade: i16,
    pub bg_col: i16,
    /// +0x2c `faceNum`.
    pub face_num: i16,
    /// +0x34 `alpha`, +0x38 `kanjiAlpha`, +0x3c `targetAlpha`, +0x40
    /// `panelAlpha` (-48 after the constructor), +0x44 `bgAlpha`, +0x48
    /// `mapAlpha`, +0x4c `cursolAlpha`, +0x50 `drainAlpha`.
    pub alpha: i32,
    pub kanji_alpha: i32,
    pub target_alpha: i32,
    pub panel_alpha: i32,
    pub bg_alpha: i32,
    pub map_alpha: i32,
    pub cursol_alpha: i32,
    pub drain_alpha: i32,
    /// +0x54 `cursolTarget`: the character the cursor is on (handle).
    pub cursol_target: u32,
    /// +0xec `reverseHead`: rows greyed, a bit each.
    pub reverse_head: u32,
    /// +0xf4 `plAttack`.
    pub pl_attack: i16,
    /// +0xf8 `firstTime`.
    pub first_time: i16,
    /// +0xfe `forbid` (the events' `menu_ban`), +0x100 `forbidChatExcept`,
    /// +0x102 `targetForbid`, +0x104 `interNoiz`.
    pub forbid: i16,
    pub forbid_chat_except: i16,
    pub target_forbid: i16,
    pub inter_noiz: i16,
    /// +0xe8 `noiz`: the screen breaking up, drawn at the end of `Disp`
    /// (the Data Drain, the gate hack, `interNoiz`).
    pub noiz: Noiz,
    /// +0x12a `mailCnt`: frames to the new-mail sound (240 at the start).
    pub mail_cnt: i16,
    /// +0x12c `protect[12]`, +0x144 `protectCnt[12]`, +0x15c
    /// `protectChar[12]`.
    pub protect: [i16; 12],
    pub protect_cnt: [i16; 12],
    pub protect_char: [u32; 12],
    /// +0x106 - +0x129 as shorts: `chatMember`, `chatAction`, `chatSkill`,
    /// `equipSpcNum`, `chatEquipStatus[2]`, `chatEquip[2]`. ChatMenu keeps
    /// each member's chat settings here as a 12-byte record per party
    /// slot (slots 1 and 2 run into `chatEquip`), and marks slot `k`
    /// unset at `equipSpcNum + 2 k`; see [`crate::menus::chat`].
    pub chat_mem: [i16; 18],
    /// +0x1f4 `subTarget[16]`: an area skill's other targets (handles; 0
    /// none; only TargetMenu's first frame clears them).
    pub sub_target: [u32; 16],
    /// +0x18c `temp[8]`: the gate's three words and their lists' scrolls;
    /// Status's member for its Items page (31).
    pub temp: [i32; 8],
    /// The word under GtNewMenuDisp's cursor (a stack slot the game keeps
    /// from frame to frame).
    pub gt_word: i32,
    /// What `WORLD_MAN::SimGenerateCode` last made of the gate's words.
    pub generated: crate::words::Generated,
    /// `rand()`: newlib's, unless the runtime gives its own.
    pub rng: Box<dyn FnMut() -> i32>,
    /// +0x238 `itemNum`: the item (or drain skill) a sub-menu is about.
    pub item_num: i32,
    /// +0x23c `trapNum`: the item a full bag is offered (ReplaceItemMenu);
    /// the box menus' trap (32: the box's `param[2]`, 33: its trap skill).
    pub trap_num: i32,
    /// +0x1f0: the items `DataDrainSubMenu` (67) has still to give from
    /// `drainItem` (the talk state's), the last first.
    pub drain_num: i32,
    /// A `ccUseItemRequest` the task is inside ([`crate::menus::useitem`]).
    pub item: Option<crate::menus::useitem::ItemRun>,
    /// +0xb8 `menuFade`: the menu's own screen fader (Gate Out, Log Out,
    /// the Ryu Books), sent by `Disp` after `ccMsg`.
    pub menu_fade: piney_demo::fade::ScFade,
    /// `GtHackMenu`'s screen (`ccHackMenu`, a local of its frames) and
    /// what its `Select` ended with.
    pub hack: Option<Box<crate::menus::hack::HackMenu>>,
    pub hack_result: i32,
    /// `DataDrainMenu`'s state ([`crate::menus::drain`]).
    pub data_drain: crate::menus::drain::DrainState,
    /// +0xf0 `streamFlag` and +0xf2 `streamNum`: `StreamMenu`'s (menu
    /// 74) members drawn (bits 0-2), no fade in (0x100), a white one
    /// (0x200), and its stream ([`crate::menus::stream`]).
    pub stream_flag: i16,
    pub stream_num: i16,
    pub stream_menu: crate::menus::stream::StreamState,
    /// `ccThBook` while a Ryu Book is read ([`crate::book::Task`]).
    pub book: Option<Box<crate::book::Task>>,
    /// `BookOfs[24]` (gcmn 0x005d3530): each book's window, which books III
    /// and VIII change as their lists open and close.
    pub book_ofs: Vec<i32>,
    /// +0x234 `dummyTarget`: a character the talk menus hold (handle).
    pub dummy_target: Option<crate::world::CharInfo>,
    /// `menuList[89]`.
    pub lists: Vec<MenuList>,
    /// The talk and shop menus' state and seam (`talkNum`, `temp`, who is
    /// spoken to).
    pub talk: crate::talk::TalkState,
    /// The sprites.
    pub win: Spr,
    pub win_pr: Spr,
    pub win_a: Spr,
    pub ene_life: Spr,
    pub target: Spr,
    pub kanji: Spr,
    pub kanji_pr: Spr,
    pub name: Spr,
    pub name_pr: Spr,
    pub setting: Vec<Spr>,
    /// +0xa0 `settingKanjiSend[8]`.
    pub setting_send: [bool; 8],
    pub font: Spr,
    pub fly: Spr,
    pub faces: Vec<Spr>,
    pub item_icon: Spr,
    pub con_icon: Spr,
    pub bg: Spr,
    pub drain: Spr,
    pub protect_spr: Spr,
    pub mask: Spr,
    /// The texts last `Extract`ed into menuKanji, menuKanjiPr, nameKanji,
    /// nameKanjiPr and the setting kanji.
    pub kanji_text: Vec<u8>,
    pub kanji_pr_text: Vec<u8>,
    pub name_text: Vec<u8>,
    pub name_pr_text: Vec<u8>,
    pub setting_text: Vec<Vec<u8>>,
    /// The face texture each `menuFace` holds (`menuFaceCcsList` row, -1
    /// none).
    pub face_tex: [i32; 4],
    /// menuWindow's and menuWindowPr's select cursors.
    pub cursor: Cursor,
    pub cursor_pr: Cursor,
    /// `ccMsg`: the event windows and the menus' help.
    pub msg: MsgWindow,
    /// The rest of a handler after its breath.
    pub cont: Option<Cont>,
    /// The frame's drawing so far.
    pub draws: Vec<Draw>,
    /// `ccChat` (0x00378a90), which the constructor makes: the chat
    /// balloons ([`crate::chat_msg`]).
    pub chat: crate::chat_msg::ChatMsg,
    /// The OPTION pages' members (the Sound page's settings, +0x18c); see
    /// [`crate::menus::option`].
    pub option: crate::menus::option::OptionState,
    /// A trace of calls for the harness (feature `trace`).
    #[cfg(feature = "trace")]
    pub trace: Vec<String>,
}

impl MenuCtrl {
    /// `ccMenuCtrl::ccMenuCtrl` (0x0051c140) and `InitMenuList`.
    pub fn new(texts: &Texts) -> MenuCtrl {
        let spr = |o, n| Spr::new(o, n);
        let mut con_icon = spr(Obj::ConIcon, 84);
        let mut item_icon = spr(Obj::ItemIcon, 40);
        let mut bg = spr(Obj::MenuBg, 8);
        for s in [&mut con_icon, &mut item_icon] {
            s.wi = 1;
        }
        bg.wu = 0;
        bg.wv = 0;
        bg.wi = 1;
        bg.su = 64;
        bg.sv = 64;
        bg.sx = 128.0;
        bg.sy = 224.0;
        let mut faces: Vec<Spr> = (0..4).map(|i| spr(Obj::MenuFace(i), 2)).collect();
        for f in faces.iter_mut() {
            f.wu = 0;
            f.wv = 0;
            f.wi = 1;
        }
        let mut rng: Box<dyn FnMut() -> i32> = Box::new(newlib_rand());
        let noiz = Noiz::new(&mut *rng);
        MenuCtrl {
            boot: Boot::WaitParty,
            panel_bure: [0; 3],
            panel_flash: [0; 3],
            menu: -1,
            menu_next: -1,
            menu_status: 0,
            panel_status: 1,
            bg_status: 0,
            map_status: 1,
            drain_status: 0,
            open_req: -1,
            mode: 0,
            still: 0,
            proccess: 0,
            wait_count: 0,
            exception_disp: 0,
            battle_cnt: 0,
            teach_cnt: 0,
            cursol_init: 0,
            cursol_off: 0,
            fade: 0,
            bg_col: 8,
            face_num: 0,
            alpha: 0,
            kanji_alpha: 0,
            target_alpha: 0,
            panel_alpha: -48,
            bg_alpha: 0,
            map_alpha: 0,
            cursol_alpha: 0,
            drain_alpha: 0,
            cursol_target: 0,
            reverse_head: 0,
            pl_attack: 0,
            first_time: 0,
            forbid: 0,
            forbid_chat_except: 0,
            target_forbid: 0,
            inter_noiz: 0,
            noiz,
            mail_cnt: 240,
            protect: [0; 12],
            protect_cnt: [0; 12],
            protect_char: [0; 12],
            chat_mem: [0; 18],
            temp: [0; 8],
            gt_word: 0,
            generated: crate::words::Generated::default(),
            rng,
            sub_target: [0; 16],
            item_num: 0,
            trap_num: 0,
            drain_num: 0,
            item: None,
            menu_fade: piney_demo::fade::ScFade::default(),
            hack: None,
            hack_result: 0,
            data_drain: crate::menus::drain::DrainState::default(),
            stream_flag: 0,
            stream_num: 0,
            stream_menu: crate::menus::stream::StreamState::default(),
            book: None,
            book_ofs: piney_data::tables::book::of(texts.volume).ofs.to_vec(),
            dummy_target: None,
            lists: texts.lists.clone(),
            talk: crate::talk::TalkState::default(),
            win: spr(Obj::MenuWindow, 192),
            win_pr: spr(Obj::MenuWindowPr, 64),
            win_a: spr(Obj::MenuWindowA, 48),
            ene_life: spr(Obj::EneLife, 32),
            target: spr(Obj::TargetCursol, 32),
            kanji: spr(Obj::MenuKanji, 24),
            kanji_pr: spr(Obj::MenuKanjiPr, 24),
            name: spr(Obj::NameKanji, 12),
            name_pr: spr(Obj::NameKanjiPr, 12),
            setting: (0..8).map(|i| spr(Obj::Setting(i), 32)).collect(),
            setting_send: [true; 8],
            font: spr(Obj::MenuFont, 256),
            fly: spr(Obj::FlyFont, 160),
            faces,
            item_icon,
            con_icon,
            bg,
            drain: spr(Obj::MenuDrain, 16),
            protect_spr: spr(Obj::MenuProtect, 16),
            mask: spr(Obj::MenuMask, 64),
            kanji_text: Vec::new(),
            kanji_pr_text: Vec::new(),
            name_text: Vec::new(),
            name_pr_text: Vec::new(),
            setting_text: vec![Vec::new(); 8],
            face_tex: [-1; 4],
            cursor: Cursor::default(),
            cursor_pr: Cursor::default(),
            msg: MsgWindow::default(),
            cont: None,
            draws: Vec::new(),
            chat: crate::chat_msg::ChatMsg::new(),
            option: Default::default(),
            #[cfg(feature = "trace")]
            trace: Vec::new(),
        }
    }

    /// Records a line of the trace (feature `trace`).
    #[allow(unused_variables)]
    pub fn note(&mut self, s: impl FnOnce() -> String) {
        #[cfg(feature = "trace")]
        self.trace.push(s());
    }

    /// Both select cursors back (`InitCursol(1)` on each window).
    pub fn cursors_init(&mut self) {
        self.cursor.init(true);
        self.cursor_pr.init(true);
    }

    /// +0x106 `chatMember`.
    pub fn chat_member(&self) -> i16 {
        self.chat_mem[0]
    }

    /// +0x10c `equipSpcNum`: the member the Equipment menu is on.
    pub fn set_equip_spc_num(&mut self, v: i16) {
        self.chat_mem[3] = v;
    }

    pub fn equip_spc_num(&self) -> i16 {
        self.chat_mem[3]
    }

    /// `CheckMenuType` (0x00526150).
    pub fn check_menu_type(&self) -> i32 {
        if self.menu == self.menu_next { i32::from(self.menu) } else { MENU_CHANGING }
    }

    /// The list of the menu open.
    pub fn list(&self) -> &MenuList {
        &self.lists[self.menu.clamp(0, 88) as usize]
    }

    pub fn list_mut(&mut self) -> &mut MenuList {
        let m = self.menu.clamp(0, 88) as usize;
        &mut self.lists[m]
    }

    /// One pass of `ccThMenu`: the frame's drawing.
    pub fn frame(&mut self, x: &mut Ctx) -> Vec<Draw> {
        self.draws.clear();
        match self.boot {
            Boot::WaitParty => {
                // Slot 0 seen, the loop's own breath ends this frame.
                if x.world.party[0].is_some() {
                    self.boot = Boot::Running;
                }
                return std::mem::take(&mut self.draws);
            }
            Boot::Running => {}
        }
        // A use no one answered goes on as one with nothing to do.
        if self.item_asked() {
            self.answer_item(Vec::new());
        }
        if let Some(c) = self.cont.take() {
            if let After::Item = c.after {
                if let Some(next) = crate::menus::useitem::run(self, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Use(t) = c.after {
                // A tail that breathes again did its own Disp.
                if let Some(next) = crate::menus::target::use_tail(self, t, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Hack(r) = c.after {
                if let Some(next) = crate::menus::hack::resume(self, r, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Drain(r) = c.after {
                if let Some(next) = crate::menus::drain::resume(self, r, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Stream(r) = c.after {
                if let Some(next) = crate::menus::stream::resume(self, r, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::ChatT(n) = c.after {
                if let Some(next) = crate::menus::tutorial::chat_tail(self, c.woke, n, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Talk(t) = c.after {
                if let Some(next) = crate::talk::tail(self, t, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Pers(t) = c.after {
                if let Some(next) = crate::menus::personal::tail(self, c, t, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else if let After::Option(r) = c.after {
                if let Some(next) = crate::menus::option::resume(self, r, x) {
                    self.cont = Some(next);
                    return std::mem::take(&mut self.draws);
                }
            } else {
                self.continue_after_breath(c, x);
            }
            crate::disp::disp(self, x);
            return std::mem::take(&mut self.draws);
        }
        if self.open_req >= 0 {
            let n = self.open_req;
            if n & 0x1000 != 0 {
                self.open_menu_nwd(n & 0xfff, x);
            } else {
                self.open_menu(n, x);
            }
        }
        if self.check_menu_type() == MENU_CHANGING {
            if self.menu_status == 0 {
                self.menu = self.menu_next;
                if self.menu != -1 {
                    self.menu_status = 1;
                    self.reverse_head = 0;
                } else {
                    if self.forbid == 0 || self.forbid_chat_except != 0 {
                        self.panel_status = 1;
                    }
                    x.req.push(Request::TargetFix(false));
                }
            }
            self.cursor.init(true);
            self.cursor_pr.init(true);
        }
        let flow = crate::menus::handler(self, x);
        if let Flow::Breathed(c) = flow {
            self.cont = Some(c);
            return std::mem::take(&mut self.draws);
        }
        // Inside ccUseItemRequest: the frame goes on with the runtime's
        // answer ([`MenuCtrl::item_frame`]).
        if self.item_asked() {
            return std::mem::take(&mut self.draws);
        }
        crate::disp::disp(self, x);
        std::mem::take(&mut self.draws)
    }

    /// The rest of a frame that stopped in `ccUseItemRequest`, the
    /// runtime's steps given ([`MenuCtrl::answer_item`]): the call to its
    /// first breath or its end.
    pub fn item_frame(&mut self, x: &mut Ctx) -> Vec<Draw> {
        match crate::menus::useitem::run(self, x) {
            Some(next) => self.cont = Some(next),
            None => crate::disp::disp(self, x),
        }
        std::mem::take(&mut self.draws)
    }

    fn continue_after_breath(&mut self, c: Cont, x: &mut Ctx) {
        // The gate's tails do their own; only its close ends as CloseMenu.
        if let After::Gate(g) = c.after
            && !matches!(g, crate::menus::gate::GateAfter::GateHelp | crate::menus::gate::GateAfter::TutorialDone)
        {
            crate::menus::gate::after(self, g, x);
            return;
        }
        self.close_tail(c.woke, x);
        if c.cursors {
            self.cursor.init(true);
            self.cursor_pr.init(true);
        }
        match c.after {
            After::Nothing => {}
            After::SystemHelp => crate::menus::system::help(self, x),
            After::SystemChanged => {
                crate::menus::system::after_change(self);
                crate::menus::system::help(self, x);
            }
            After::Use(_) | After::Talk(_) | After::Pers(_) | After::Option(_) | After::Item => {}
            After::ChatHelp => match self.menu {
                71 => crate::menus::chat_member::help1(self, x),
                73 => crate::menus::chat_member::help3(self, x),
                _ => crate::menus::chat::help(self, x),
            },
            After::Gate(g) => crate::menus::gate::after(self, g, x),
            After::ChatT(_) | After::Hack(_) | After::Drain(_) | After::Stream(_) => {}
            After::Fountain(t) => crate::menus::fountain::after(self, t, x),
            After::Panels(on) => {
                if on {
                    self.panel_status = 3;
                }
            }
            After::Replace(t) => crate::menus::getitem::replace_after(self, t, x),
        }
    }

    fn close_tail(&mut self, woke: bool, x: &mut Ctx) {
        if woke {
            self.still = 0;
            x.req.push(Request::Still(false));
        }
        self.first_time = 0;
    }

    fn sleep_all(&mut self, x: &mut Ctx) {
        x.req.push(Request::SleepAll);
        self.still = 1;
        x.req.push(Request::Still(true));
        x.req.push(Request::KeepLayers);
    }

    /// `OpenMenu(t)` (0x00525b40).
    pub fn open_menu(&mut self, t: i16, x: &mut Ctx) {
        self.open_req = -1;
        self.menu_next = t;
        self.menu = t;
        self.lists[t.clamp(0, 88) as usize].prev = -1;
        self.menu_status = 1;
        self.panel_status = 3;
        self.proccess = 0;
        self.wait_count = 0;
        self.reverse_head = 0;
        self.exception_disp = 0;
        if self.mode == 0 && self.still == 0 {
            self.sleep_all(x);
        }
        if !SILENT_OPEN.contains(&t) {
            x.se(SE_OPEN);
        }
        self.cursor.init(false);
        self.cursor_pr.init(false);
    }

    /// `OpenMenuNWD(t)` (0x00525ce0): as OpenMenu without the window's
    /// fade in (menuStatus kept) or the sound.
    pub fn open_menu_nwd(&mut self, t: i16, x: &mut Ctx) {
        self.open_req = -1;
        self.menu_next = t;
        self.menu = t;
        self.lists[t.clamp(0, 88) as usize].prev = -1;
        self.panel_status = 3;
        self.proccess = 0;
        self.wait_count = 0;
        self.reverse_head = 0;
        self.exception_disp = 0;
        if self.mode == 0 && self.still == 0 {
            self.sleep_all(x);
        }
        self.cursor.init(false);
        self.cursor_pr.init(false);
    }

    /// The first half of `CloseMenu` (0x00525e20): the window closing,
    /// every task woken, `Disp`, and the breath; [`Cont::Close`] finishes
    /// it next frame.
    pub fn close_menu(&mut self, x: &mut Ctx) -> Flow {
        self.menu_next = -1;
        self.menu_status = 3;
        self.bg_status = 3;
        let woke = self.breathe_close(x);
        Flow::Breathed(Cont::close(woke))
    }

    /// `WakeAll` if still, `Disp`, and the breath: whether it woke.
    pub fn breathe_close(&mut self, x: &mut Ctx) -> bool {
        let woke = self.still == 1;
        if woke {
            x.req.push(Request::WakeAll);
        }
        crate::disp::disp(self, x);
        woke
    }

    /// `ChangeMenu()` (0x00525f60): into the item under the cursor, when the
    /// events allow it; else the menu closes.
    pub fn change_menu(&mut self, x: &mut Ctx) -> Flow {
        let l = self.list().clone();
        self.menu_next = l.items.get(l.select.max(0) as usize).copied().unwrap_or(-1);
        let op = if self.menu_next == 10 { 14 } else { -1 };
        if x.save.check_operate(op) {
            if self.menu_next != -1 {
                let m = self.menu;
                self.lists[self.menu_next.clamp(0, 88) as usize].prev = m;
                self.proccess = 0;
                self.wait_count = 0;
            }
            self.menu_status = 3;
            self.first_time = 0;
            self.cursor.init(true);
            self.cursor_pr.init(true);
            Flow::Done
        } else {
            self.menu_next = -1;
            self.menu_status = 3;
            self.bg_status = 3;
            let woke = self.breathe_close(x);
            // The cursors' InitCursol(1) comes after the breath.
            Flow::Breathed(Cont { woke, cursors: true, after: After::Nothing })
        }
    }

    /// `ChangeMenu(m)` (0x005260d0).
    pub fn change_menu_to(&mut self, m: i16) {
        self.menu_next = m;
        let cur = self.menu;
        self.lists[m.clamp(0, 88) as usize].prev = cur;
        self.proccess = 0;
        self.wait_count = 0;
        self.menu_status = 3;
        self.first_time = 0;
        self.cursor.init(true);
        self.cursor_pr.init(true);
    }

    /// Back to the list this one came from (`menuNext = prev`), the
    /// cursors reset.
    pub fn back_to_prev(&mut self, prev: i16) {
        self.menu_next = prev;
        self.menu_status = 3;
        self.proccess = 0;
        self.wait_count = 0;
        self.cursor.init(true);
        self.cursor_pr.init(true);
    }

    /// `Select(pn, lim, af)` (0x00526170): up / down through the rows
    /// (sound 17), left / right (and L1 / R1) through `pn` pages; `lim`
    /// bit 2 stops at the ends instead of wrapping, bit 1 the pages. With
    /// `af` a move makes the help fade in again. Returns 1 for a row, 2 for
    /// a page.
    pub fn select(&mut self, pn: i32, lim: i32, af: bool, x: &mut Ctx) -> i32 {
        let repeat = x.pad.repeat.bits();
        let mut r = 0;
        let m = self.menu.clamp(0, 88) as usize;
        let old_sel = self.lists[m].select;
        let old_page = self.lists[m].page;
        if self.lists[m].y > 0 {
            if repeat & 0x1000 != 0 {
                self.lists[m].select = old_sel - 1;
                if self.lists[m].select < 0 {
                    if lim & 2 != 0 {
                        self.lists[m].select = 0;
                    } else {
                        self.lists[m].select = self.lists[m].y - 1;
                        x.se(SE_MOVE);
                        r = 1;
                    }
                } else {
                    x.se(SE_MOVE);
                    r = 1;
                }
            } else if repeat & 0x4000 != 0 {
                self.lists[m].select = old_sel + 1;
                if self.lists[m].select >= self.lists[m].y {
                    if lim & 2 != 0 {
                        self.lists[m].select = self.lists[m].y - 1;
                    } else {
                        self.lists[m].select = 0;
                        x.se(SE_MOVE);
                        r = 1;
                    }
                } else {
                    x.se(SE_MOVE);
                    r = 1;
                }
            }
        }
        let l = &mut self.lists[m];
        if l.select >= l.y {
            l.select = l.y - 1;
        }
        if l.select < 0 {
            l.select = 0;
        }
        if af && old_sel != l.select {
            self.msg.set_kanji_alpha(-32);
        }
        if pn < 2 {
            return r;
        }
        if repeat & 0x8004 != 0 {
            self.lists[m].page -= 1;
            if self.lists[m].page < 0 {
                if lim & 1 != 0 {
                    self.lists[m].page = 0;
                } else {
                    self.lists[m].page = (pn - 1) as i16;
                    self.kanji_alpha = -24;
                    x.se(SE_MOVE);
                    r |= 2;
                }
            } else {
                self.kanji_alpha = -24;
                x.se(SE_MOVE);
                r |= 2;
            }
        } else if repeat & 0x2008 != 0 {
            self.lists[m].page += 1;
            if i32::from(self.lists[m].page) >= pn {
                if lim & 1 != 0 {
                    self.lists[m].page = (pn - 1) as i16;
                } else {
                    self.lists[m].page = 0;
                    self.kanji_alpha = -24;
                    x.se(SE_MOVE);
                    r |= 2;
                }
            } else {
                self.kanji_alpha = -24;
                x.se(SE_MOVE);
                r |= 2;
            }
        }
        if af && old_page != self.lists[m].page {
            self.msg.set_kanji_alpha(-32);
        }
        r
    }

    /// `SelectScr(pn, lim, ofs)` (0x00526490): as `Select` over a list that
    /// scrolls (`dy` the first row shown of `my`, `y` shown at once).
    pub fn select_scr(&mut self, pn: i32, lim: i32, ofs: i32, x: &mut Ctx) -> i32 {
        let repeat = x.pad.repeat.bits();
        let m = self.menu.clamp(0, 88) as usize;
        let mut r = 0;
        let old_sel = self.lists[m].select;
        let old_page = self.lists[m].page;
        if repeat & 0x1000 != 0 {
            self.lists[m].select = old_sel - 1;
            x.se(SE_MOVE);
            r = 1;
        } else if repeat & 0x4000 != 0 {
            self.lists[m].select = old_sel + 1;
            x.se(SE_MOVE);
            r = 1;
        }
        {
            let l = &mut self.lists[m];
            if l.select < 0 {
                l.select = if lim & 2 != 0 { 0 } else { l.my - 1 };
            } else if l.select >= l.my {
                l.select = if lim & 2 != 0 { l.my - 1 } else { 0 };
            }
            let mut a0 = i32::from(l.select) - ofs;
            if a0 < 0 {
                a0 = 0;
            }
            if a0 < i32::from(l.dy) {
                l.dy = a0 as i16;
            } else if a0 >= i32::from(l.dy) + i32::from(l.y) {
                l.dy = (a0 - (i32::from(l.y) - 1)) as i16;
            }
        }
        if old_sel != self.lists[m].select {
            self.msg.set_kanji_alpha(-32);
        }
        if pn < 2 {
            return r;
        }
        if repeat & 0x8004 != 0 {
            self.lists[m].page -= 1;
            x.se(SE_MOVE);
            r |= 2;
        } else if repeat & 0x2008 != 0 {
            self.lists[m].page += 1;
            x.se(SE_MOVE);
            r |= 2;
        }
        {
            let l = &mut self.lists[m];
            if l.page < 0 {
                l.page = if lim & 1 != 0 { 0 } else { (pn - 1) as i16 };
            } else if i32::from(l.page) >= pn {
                l.page = if lim & 1 != 0 { (pn - 1) as i16 } else { 0 };
            }
        }
        if old_page != self.lists[m].page {
            self.msg.set_kanji_alpha(-32);
        }
        r
    }

    /// `SetPanelBure(id, bure)` (0x00522430): a panel shakes and flashes.
    pub fn set_panel_bure(&mut self, id: i32, bure: u8) {
        if (0..3).contains(&id) {
            self.panel_bure[id as usize] = bure;
        }
        // The flash's index is written unchecked (id + 3 into the struct).
        if (0..3).contains(&id) {
            self.panel_flash[id as usize] = 4;
        } else if (-3..0).contains(&id) {
            self.panel_bure[(id + 3) as usize] = 4;
        }
    }

    /// `SetProtect(pn, c)` (0x00526e60).
    pub fn set_protect(&mut self, pn: i16, c: &crate::world::CharInfo) {
        if c.condition[0] != 0 {
            return;
        }
        for k in 0..12 {
            if self.protect_cnt[k] == 0 {
                self.protect_cnt[k] = 60;
                self.protect[k] = pn;
                self.protect_char[k] = c.handle;
                return;
            }
        }
    }

    /// The event scripts' `menu_ban` (`ccEvent::MenuBan`, 0x001b2460) and
    /// `menu_clear` (`MenuClr`, 0x001b25d0), as far as the menu goes.
    pub fn menu_ban(&mut self, on: bool) {
        if on {
            self.forbid = 1;
            self.target_forbid = 1;
            self.panel_status = 0;
            self.map_status = 0;
        } else {
            self.forbid = 0;
            self.target_forbid = 0;
            self.panel_status = 1;
            self.map_status = 1;
        }
    }
}
