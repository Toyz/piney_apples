//! The bulletin board (bbs.cpp, toppage.prg 0x00401730-0x0040495c):
//! `ccThBBSCtrl` with its three pages, the thread and post objects built
//! from the save, and the scroll bars (`docs/engine/toppage.md`).
//!
//! The board is built by `Init` (0x00401b60), both when the top page starts
//! and each time the player enters it. Its pages, by `m_iDrawState`:
//!
//! ```text
//! 0  the thread list        DrawThreadPage (0x00402b30)
//! 1  a thread's posts       DrawMessagePage (0x00403060), with SelectMessage
//!                           (0x004038d0) or ShowMessage (0x00403b30)
//! 2  the player's own post  DrawWritingMsgPage (0x00402530): typed out
//! 3  leave                  DrawBBS sets m_exit and the top page takes over
//! ```
//!
//! `saveData.bbsList[t][p]` (+0x28e4, byte `48 t + p`) holds each post's
//! state: 0 not posted, 1 new, 3 read, 7 the player's own post waiting to
//! be written out. The event scripts post with `bbs_post` (1) and
//! `bbs_post7` (7).

use std::rc::Rc;

use piney_data::save::offset;
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::{Fonts, Names};
use piney_desktop::sprite::Sprite;
use piney_draw::TexRef;
use piney_input::Buttons;

use crate::Request;
use crate::content::{MsgList, ThreadList};
use crate::draw::{ACTIVE_LAYER, Anim, BBS_LAYER, Cell, Text, active_view, bbs_view, packet, send};
use crate::util::{KeyRepeat, ScrollBar, line_length_no_color, str_range_copy};

/// Bytes a thread takes in `bbsList` (`[128][48]`).
pub const BBS_ROW: usize = 48;
/// `bbsList`'s size: 128 threads of 48 posts.
pub const BBS_SIZE: usize = 128 * BBS_ROW;

/// A post's state in `bbsList`.
pub const POST_NONE: i8 = 0;
pub const POST_NEW: i8 = 1;
pub const POST_READ: i8 = 3;
/// `bbs_post7`: the player's own post, written out on the next visit.
pub const POST_WRITE: i8 = 7;

/// The board's animations in `xdttopen0`: the thread list's and the posts'
/// backgrounds (`Init` 0x00401e6c, 0x00401ea4).
pub const ANM_THREAD_PAGE: &str = "ANM_xdtbbsa1";
pub const ANM_MSG_PAGE: &str = "ANM_xdtbbsa0";
/// The mask's texture (`Init` 0x00401cc8): the NEW mark, the tree's
/// branches, the post icon, the scroll bars and arrows, and a column of
/// dates.
pub const MASK_TEX: &str = "TEX_xdtbbsp1";
/// `new ccMask(96, 0)`.
pub const MASK_PACKETS: usize = 96;

/// The thread the Time Idol ranking is posted in, and its post
/// (`ccBBSMsgObj::Init` 0x00404404): lines 7 + 5 i and 8 + 5 i of that
/// post show rank i's time and player from `saveData.timeIdolRankStr`.
pub const TIME_IDOL_THREAD: i32 = 29;
pub const TIME_IDOL_POST: i32 = 1;
/// `saveData.timeIdolRankStr[5][2][20]` (+0x6775): rank i's player at
/// `+0x6775 + 40 i`, time at `+0x6789 + 40 i`.
pub const TIME_IDOL_RANKS: usize = 0x6775;

/// `ccSpriteColorTable[4]` for the row under the cursor, `[7]` the rest.
pub const C_SELECTED: usize = 4;
pub const C_NORMAL: usize = 7;

/// The sound effects the board plays.
pub const SE_MOVE: i32 = 6;
pub const SE_OPEN: i32 = 4;
pub const SE_BACK: i32 = 7;

/// `m_scrollMark.m_blinkInterval`: the "more below" arrow blinks every
/// 32 frames.
pub const BLINK_INTERVAL: i32 = 30;

/// `bbsList[t][p]` as the game reads it (`lb`, flat index `48 t + p`).
pub fn bbs_state(save: &SaveState, t: i32, p: i32) -> i8 {
    match flat(t, p) {
        Some(i) => save.save.u8(offset::BBS_LIST + i) as i8,
        None => 0,
    }
}

/// `bbsList[t][p] = v`.
pub fn set_bbs_state(save: &mut SaveState, t: i32, p: i32, v: i8) {
    if let Some(i) = flat(t, p) {
        save.save.set_u8(offset::BBS_LIST + i, v as u8);
    }
}

fn flat(t: i32, p: i32) -> Option<usize> {
    let i = (t as i64) * BBS_ROW as i64 + p as i64;
    (0..BBS_SIZE as i64).contains(&i).then_some(i as usize)
}

/// `ccSaveData::CheckWriteBbs(t, p)` (main 0x00177fa0): the first post in
/// state 7, threads 0-127 by posts 0-47; (-1, -1) for none.
pub fn check_write_bbs(save: &SaveState) -> (i32, i32) {
    for t in 0..128 {
        for p in 0..BBS_ROW as i32 {
            if bbs_state(save, t, p) == POST_WRITE {
                return (t, p);
            }
        }
    }
    (-1, -1)
}

/// `ccThBBSCtrl::CheckNewMessage` (0x00402260): whether any post of the 63
/// threads is new (state 1).
pub fn check_new_message(tbl: &[ThreadList], save: &SaveState) -> bool {
    tbl.iter().enumerate().any(|(t, th)| (0..th.msgs.len()).any(|p| bbs_state(save, t as i32, p as i32) == POST_NEW))
}

/// `ccBBSMsgObj` (0x480 bytes): one post as the board shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgObj {
    /// `m_logicalindex`: the post's index in its thread's table.
    pub logical: i32,
    /// `m_read`: 1 new (a 7 counts as 1), 3 read.
    pub read: i32,
    /// `m_MaxMsgTbl`: `maxLines + 3`, at most 10 (nothing reads it).
    pub max_msg_tbl: i32,
    pub dateindex: i32,
    pub title: Vec<u8>,
    pub max_lines: i32,
    /// `m_sentenceTbl[128]`: the title, "Author: " and the poster, an
    /// empty line (a null), then the body.
    pub sentences: Vec<Option<Vec<u8>>>,
}

/// The C string at `at` in the save (to its NUL or the save's end).
fn save_cstr(save: &SaveState, at: usize) -> Vec<u8> {
    let b = save.save.bytes();
    let end = b[at.min(b.len())..].iter().position(|&c| c == 0).map_or(b.len(), |n| at + n);
    b[at.min(end)..end].to_vec()
}

impl MsgObj {
    /// `ccBBSMsgObj::Init(parent, logic, list, state)` (0x00404310).
    pub fn new(thread: i32, logic: i32, list: &MsgList, state: i32, save: &SaveState, labels: &Labels) -> Self {
        let mut sentences: Vec<Option<Vec<u8>>> = vec![None; 128];
        sentences[0] = Some(list.title.clone());
        let mut trans = labels.author.clone();
        trans.extend_from_slice(&list.transname);
        sentences[1] = Some(trans);
        for (i, l) in list.lines.iter().enumerate() {
            if let Some(s) = sentences.get_mut(3 + i) {
                *s = Some(l.clone());
            }
        }
        let mut m = MsgObj {
            logical: logic,
            read: state,
            max_msg_tbl: (list.max_lines + 3).min(10),
            dateindex: list.dateindex,
            title: list.title.clone(),
            max_lines: list.max_lines,
            sentences,
        };
        if thread == TIME_IDOL_THREAD && logic == TIME_IDOL_POST {
            m.setup_time_idol(save, labels);
        }
        m
    }

    /// `_SetupTimeIdolMessage` (0x00404490): the five ranks over lines
    /// 7 + 5 i (the time) and 8 + 5 i (the player).
    fn setup_time_idol(&mut self, save: &SaveState, labels: &Labels) {
        for i in 0..5 {
            let mut time = labels.time.clone();
            time.extend(save_cstr(save, TIME_IDOL_RANKS + 20 + 40 * i));
            let mut name = labels.player.clone();
            name.extend(save_cstr(save, TIME_IDOL_RANKS + 40 * i));
            self.sentences[7 + 5 * i] = Some(time);
            self.sentences[8 + 5 * i] = Some(name);
        }
    }

    /// `GetLineLengthNoColor(line)`: body line `line`'s length without
    /// colour escapes (a null line counts 0).
    pub fn line_length(&self, line: i32) -> i32 {
        match usize::try_from(3 + line).ok().and_then(|i| self.sentences.get(i)) {
            Some(Some(s)) => line_length_no_color(s),
            _ => 0,
        }
    }

    fn sentence(&self, i: i32) -> Option<&[u8]> {
        usize::try_from(i).ok().and_then(|i| self.sentences.get(i)).and_then(|s| s.as_deref())
    }
}

/// `ccBBSThreadObj` (0x210 bytes): a thread with its posted posts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadObj {
    /// `m_logicalindex`: the thread's index in the table.
    pub logical: i32,
    pub title: Vec<u8>,
    /// `m_msgTbl[..m_maxVisibles]`: the posts in table order.
    pub msgs: Vec<MsgObj>,
    /// `m_read`: 1 when a post is new, else 3.
    pub read: i32,
}

impl ThreadObj {
    /// `Init(th, logic)` (0x00404050): `CreateViewTbl`, then
    /// `m_read = BbsCheck()`.
    fn new(th: &ThreadList, logic: i32, save: &SaveState, labels: &Labels) -> Self {
        let mut msgs = Vec::new();
        for (p, list) in th.msgs.iter().enumerate() {
            let mut s = i32::from(bbs_state(save, logic, p as i32));
            if s == 0 {
                continue;
            }
            if s == i32::from(POST_WRITE) {
                s = i32::from(POST_NEW);
            }
            msgs.push(MsgObj::new(logic, p as i32, list, s, save, labels));
        }
        let mut t = ThreadObj { logical: logic, title: th.title.clone(), msgs, read: 0 };
        t.read = t.bbs_check(save);
        t
    }

    /// `BbsCheck` (0x004041f0): 1 when a shown post is new in the save,
    /// else 3.
    pub fn bbs_check(&self, save: &SaveState) -> i32 {
        if self.msgs.iter().any(|m| bbs_state(save, self.logical, m.logical) == POST_NEW) { 1 } else { 3 }
    }

    /// `~ccBBSThreadObj` (0x00403f70): every post read here is written back
    /// as read.
    fn release(&self, save: &mut SaveState) {
        for m in &self.msgs {
            if m.read == i32::from(POST_READ) {
                set_bbs_state(save, self.logical, m.logical, POST_READ);
            }
        }
    }
}

/// `ccMsgPage`: a thread's posts.
#[derive(Clone)]
pub struct MsgPage {
    /// The post's scroll bar and the list's.
    pub sb_low: ScrollBar,
    pub sb_high: ScrollBar,
    pub anm: Anim,
    pub k_no_msg: Text,
    pub k_thread: Text,
    pub k_title: Vec<Text>,
    pub k_msg: Vec<Text>,
    /// The post under the cursor.
    pub index: i32,
    /// The first post shown, and whether the thread's title row has
    /// scrolled away (0 or 1).
    pub th_start: i32,
    pub th_extra: i32,
    /// 0 choosing a post, 1 reading it.
    pub i_draw_state: i32,
    /// The first line of the post shown, and one past the last.
    pub msg_start: i32,
    pub msg_last: i32,
}

/// `ccThreadPage`: the thread list.
#[derive(Clone)]
pub struct ThreadPage {
    pub anm: Anim,
    pub sb: ScrollBar,
    pub kanji: Vec<Text>,
    /// The thread under the cursor and the first shown.
    pub index: i32,
    pub start_line: i32,
}

/// `ccWriteMsgPage`: the player's own post, typed out.
#[derive(Clone)]
pub struct WriteMsgPage {
    /// The thread and post (indexes into [`Bbs::threads`] and its posts).
    pub thread: Option<usize>,
    pub msg: Option<usize>,
    pub k_new_msg: Text,
    /// Their `bbsList` indexes.
    pub th_index: i32,
    pub msg_index: i32,
    pub frame_cnt: i32,
    /// The line being typed (3 the first body line) and its bytes shown
    /// (two a step).
    pub line_index: i32,
    pub str_cnt: i32,
    /// The typed line's length without colour escapes.
    pub no_col_length: i32,
    pub b_draw_end: i32,
    pub start_line: i32,
    /// A post was written this visit: leaving its thread looks for
    /// another.
    pub b_write: i32,
}

/// `ccScrollMark`: the blinking "more below" arrow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollMark {
    pub blink_flag: i32,
    pub blink_interval: i32,
    pub count: i32,
    /// `m_setMask`: the board's mask once `Init` has run.
    pub set_mask: bool,
}

/// What the board reads from the disc.
pub struct BbsAssets {
    pub file: Rc<SceneFile>,
    /// `bbsThreadTbl` and `bbsThreadTblP` with their posts.
    pub tables: [Vec<ThreadList>; 2],
    /// `TEX_xdtbbsp1` and its height.
    pub mask_tex: Option<(TexRef, i32)>,
    /// The writing page's label (@1476) and the empty thread's line
    /// (@1793), read from the overlay.
    pub new_msg: Vec<u8>,
    pub no_msg: Vec<u8>,
    pub labels: Labels,
}

/// The labels a post's lines are made with, read from the overlay:
/// `"Author: "` (@2039, before the poster), and the Time Idol ranking's
/// `"Time:   "` (@2065) and `"Player: "` (@2066).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labels {
    pub author: Vec<u8>,
    pub time: Vec<u8>,
    pub player: Vec<u8>,
}

impl BbsAssets {
    /// `CheckThreadData` (0x00402220): the parody table when
    /// `saveData.parodyFlag` is set.
    pub fn threads(&self, save: &SaveState) -> &[ThreadList] {
        &self.tables[usize::from(save.parody())]
    }
}

/// What a board frame needs of the rest of the game.
pub struct BbsEnv<'a> {
    pub ctx: &'a mut Ctx,
    pub fonts: &'a Fonts,
    pub names: &'a Names,
    pub save: &'a mut SaveState,
    pub push: u32,
    pub pad: &'a piney_input::Pad,
    pub keys: &'a mut KeyRepeat,
    /// `ccDtMenu::CheckMenuType()`: -1 when no menu is open.
    pub menu_type: i32,
    pub req: &'a mut Vec<Request>,
}

impl BbsEnv<'_> {
    fn ok(&self) -> bool {
        self.push & self.save.ok() != 0 && self.menu_type == -1
    }

    fn cancel(&self) -> bool {
        self.push & self.save.cancel() != 0 && self.menu_type == -1
    }

    fn repeat(&mut self, key: Buttons) -> bool {
        self.keys.is_key_repeat(self.pad, key.bits(), 6)
    }

    fn se(&mut self, n: i32) {
        self.req.push(Request::Se(n));
    }
}

/// `ccThBBSCtrl` (0x850 bytes).
pub struct Bbs {
    pub thread_page: ThreadPage,
    pub msg_page: MsgPage,
    pub write: WriteMsgPage,
    /// `m_iDrawState`.
    pub draw_state: i32,
    /// `m_ThreadTbl[..m_thNum]`.
    pub threads: Vec<ThreadObj>,
    /// `m_maskBbs`.
    pub mask: Option<Sprite>,
    pub scroll_mark: ScrollMark,
    pub initialized: bool,
    /// Set by page 3: the top page takes the board down.
    pub exit: bool,
}

fn colour(k: &mut Text, c: usize) {
    k.k.colour = SPRITE_COLOR_TABLE[c];
}

fn texts(prefix: &'static [&'static str]) -> Vec<Text> {
    prefix.iter().map(|l| Text::new(l)).collect()
}

const TH_LABELS: [&str; 18] = [
    "th0", "th1", "th2", "th3", "th4", "th5", "th6", "th7", "th8", "th9", "th10", "th11", "th12", "th13", "th14",
    "th15", "th16", "th17",
];
const TITLE_LABELS: [&str; 7] = ["title0", "title1", "title2", "title3", "title4", "title5", "title6"];
const MSG_LABELS: [&str; 10] = ["msg0", "msg1", "msg2", "msg3", "msg4", "msg5", "msg6", "msg7", "msg8", "msg9"];

/// The message kanji's `SetPrim(32, 0)` (0x00401e3c): 32 packets, and the
/// type-0 sprite's ctrl 1, which drops `Init`'s drop shadow.
fn msg_text(label: &'static str) -> Text {
    let mut t = Text::new(label);
    t.k.packet_max = 32;
    t.k.shadow = false;
    t
}

impl Default for Bbs {
    fn default() -> Self {
        Self::new()
    }
}

impl Bbs {
    /// `ccThBBSCtrl::ccThBBSCtrl` (0x00401830): the pages, their counters at
    /// 0, the writing page reset, nothing built.
    pub fn new() -> Self {
        Bbs {
            thread_page: ThreadPage {
                anm: Anim::new("thpage"),
                sb: ScrollBar::default(),
                kanji: Vec::new(),
                index: 0,
                start_line: 0,
            },
            msg_page: MsgPage {
                sb_low: ScrollBar::default(),
                sb_high: ScrollBar::default(),
                anm: Anim::new("msgpage"),
                k_no_msg: Text::new("nomsg"),
                k_thread: Text::new("thread"),
                k_title: Vec::new(),
                k_msg: Vec::new(),
                index: 0,
                th_start: 0,
                th_extra: 0,
                i_draw_state: 0,
                msg_start: 0,
                msg_last: 0,
            },
            write: WriteMsgPage {
                thread: None,
                msg: None,
                k_new_msg: Text::new("newmsg"),
                th_index: -1,
                msg_index: -1,
                frame_cnt: 0,
                line_index: 3,
                str_cnt: 0,
                no_col_length: -1,
                b_draw_end: 0,
                start_line: 0,
                b_write: 0,
            },
            draw_state: 0,
            threads: Vec::new(),
            mask: None,
            scroll_mark: ScrollMark::default(),
            initialized: false,
            exit: false,
        }
    }

    /// `Init(tc)` (0x00401b60): the board built again from the save.
    pub fn init(&mut self, a: &BbsAssets, save: &mut SaveState) {
        self.release(save);
        self.thread_page.index = 0;
        self.msg_page.index = 0;
        self.thread_page.sb.init(62, 329, 18);
        self.msg_page.sb_high.init(62, 133, 7);
        self.msg_page.sb_low.init(225, 165, 10);
        // new ccLayer; Init(127, NULL); SetFrame(0, 0, 512, 448, 256, 224, 1, 6/7).
        #[cfg(feature = "trace")]
        crate::trace::record("layer", "bbs", || {
            let f = [0.0f32, 0.0, 512.0, 448.0, 256.0, 224.0, 1.0, 6.0 / 7.0];
            let v: Vec<String> = f.iter().map(|x| format!("{:08x}", x.to_bits())).collect();
            format!("{BBS_LAYER} {}", v.join(" "))
        });
        // new ccMask(96, 0); SetTex("xdttopen0", "TEX_xdtbbsp1", 1).
        #[cfg(feature = "trace")]
        crate::trace::record("tex", "mask", || format!("{} {MASK_TEX} 1", a.file.stem));
        self.mask = a.mask_tex.clone().map(|(tex, h)| Sprite::mask(BBS_LAYER, tex, h, MASK_PACKETS));
        self.thread_page.kanji = texts(&TH_LABELS);
        self.msg_page.k_title = texts(&TITLE_LABELS);
        self.msg_page.k_msg = MSG_LABELS.iter().map(|l| msg_text(l)).collect();
        self.thread_page.anm.set(&a.file, ANM_THREAD_PAGE);
        self.thread_page.anm.step();
        self.msg_page.anm.set(&a.file, ANM_MSG_PAGE);
        self.msg_page.anm.step();
        self.draw_state = 0;
        self.threads.clear();
        self.create_read_table(a, save);
        self.init_writing_msg_page(save);
        self.scroll_mark.set_mask = true;
        self.scroll_mark.blink_interval = BLINK_INTERVAL;
        self.scroll_mark.count = 0;
        self.exit = false;
        self.initialized = true;
    }

    /// `Release` (0x00401f60): the mask, the layer, the threads (writing
    /// back what was read) and the page kanji go.
    pub fn release(&mut self, save: &mut SaveState) {
        if self.initialized {
            self.mask = None;
            for t in &self.threads {
                t.release(save);
            }
            self.threads.clear();
            self.thread_page.kanji.clear();
            self.msg_page.k_title.clear();
            self.msg_page.k_msg.clear();
        }
        self.scroll_mark.set_mask = false;
        self.initialized = false;
    }

    /// `_CreateReadTable` (0x004020e0): a thread object for each thread
    /// with a post in any state.
    fn create_read_table(&mut self, a: &BbsAssets, save: &SaveState) {
        self.threads.clear();
        for (t, th) in a.threads(save).iter().enumerate() {
            if (0..th.msgs.len()).any(|p| bbs_state(save, t as i32, p as i32) != POST_NONE) {
                self.threads.push(ThreadObj::new(th, t as i32, save, &a.labels));
            }
        }
    }

    /// `m_thNum`.
    pub fn th_num(&self) -> i32 {
        self.threads.len() as i32
    }

    /// `InitWritingMsgPage` (0x004023d0): the writing page reset; the first
    /// post waiting to be written (state 7) opens it, else the thread list.
    pub fn init_writing_msg_page(&mut self, save: &SaveState) {
        let w = &mut self.write;
        (w.thread, w.msg, w.th_index, w.msg_index) = (None, None, -1, -1);
        (w.frame_cnt, w.line_index, w.str_cnt, w.no_col_length) = (0, 3, 0, -1);
        (w.b_draw_end, w.start_line, w.b_write) = (0, 0, 0);
        let (t, p) = check_write_bbs(save);
        if t == -1 {
            self.init_thread_page(save);
            return;
        }
        let ti = self.threads.iter().position(|th| th.logical == t);
        let mi = ti.and_then(|i| self.threads[i].msgs.iter().position(|m| m.logical == p));
        let (Some(ti), Some(mi)) = (ti, mi) else {
            // A post in state 7 outside the tables: the game would follow a
            // null thread; the port shows the thread list.
            self.init_thread_page(save);
            return;
        };
        let w = &mut self.write;
        (w.thread, w.msg, w.th_index, w.msg_index) = (Some(ti), Some(mi), t, p);
        w.no_col_length = self.threads[ti].msgs[mi].line_length(0);
        w.b_write = 1;
        self.draw_state = 2;
    }

    /// `InitThreadPage` (0x00402a90): after a written post, look for the
    /// next; else the thread list, every thread's new mark taken again.
    pub fn init_thread_page(&mut self, save: &SaveState) {
        if self.write.b_write != 0 {
            self.init_writing_msg_page(save);
            self.write.b_write = 0;
        } else {
            self.draw_state = 0;
            for i in 0..self.threads.len() {
                let r = self.threads[i].bbs_check(save);
                self.threads[i].read = r;
            }
        }
    }

    /// `InitMessagePage` (0x00403050).
    pub fn init_message_page(&mut self) {
        self.draw_state = 1;
    }

    /// `ExitWritingMsgPage` (0x004029b0): the post is read; its thread's
    /// page opens on it.
    fn exit_writing_msg_page(&mut self, save: &mut SaveState) {
        set_bbs_state(save, self.write.th_index, self.write.msg_index, POST_READ);
        self.init_message_page();
        let mut ti = 0;
        while ti < self.threads.len() && self.threads[ti].logical != self.write.th_index {
            ti += 1;
        }
        self.thread_page.index = ti as i32;
        // The posts searched are the last thread looked at.
        let Some(th) = self.threads.get(ti.min(self.threads.len().saturating_sub(1))) else { return };
        let mi = th.msgs.iter().position(|m| m.logical == self.write.msg_index).unwrap_or(th.msgs.len());
        self.msg_page.index = mi as i32;
    }

    /// `DrawBBS` (0x00402320): the page, then the mask's packets.
    pub fn draw(&mut self, a: &BbsAssets, x: &mut BbsEnv) {
        match self.draw_state {
            0 => self.draw_thread_page(x),
            1 => self.draw_message_page(a, x),
            2 => self.draw_writing_msg_page(a, x),
            3 => {
                self.exit = true;
                return;
            }
            _ => {}
        }
        if let Some(m) = self.mask.as_mut() {
            send(m, x.ctx);
        }
    }

    fn pkt(&mut self, c: Cell) {
        if let Some(m) = self.mask.as_mut() {
            packet(m, &bbs_view(), c);
        }
    }

    /// The scroll bar's box: 5 wide at x 472.
    fn bar(&mut self, sb: ScrollBar) {
        self.pkt(Cell {
            wu: 256,
            wv: 256,
            su: 8,
            sv: 5,
            sx: 5.0,
            sy: sb.b_length as f32,
            dx: 472.0,
            dy: (sb.b_pos + sb.s_pos) as f32,
        });
    }

    /// The NEW mark at (x, y).
    fn new_mark(&mut self, x: f32, y: f32) {
        self.pkt(Cell { wu: 0, wv: 0, su: 32, sv: 16, sx: 32.0, sy: 16.0, dx: x, dy: y });
    }

    /// A post's date: row `dateindex` of the date column, after the title.
    fn date(&mut self, dateindex: i32, x: i32, y: f32) {
        self.pkt(Cell {
            wu: 512,
            wv: dateindex.wrapping_shl(8),
            su: 64,
            sv: 16,
            sx: 64.0,
            sy: 16.0,
            dx: x as f32,
            dy: y,
        });
    }

    /// The blinking "more below" arrow at y, then the blink's count.
    fn more_below(&mut self, y: f32) {
        if self.scroll_mark.blink_flag != 0 && self.scroll_mark.set_mask {
            self.pkt(Cell { wu: 0, wv: 1024, su: 16, sv: 16, sx: 16.0, sy: 16.0, dx: 240.0, dy: y });
        }
        let old = self.scroll_mark.count;
        self.scroll_mark.count += 1;
        if self.scroll_mark.blink_interval < old {
            self.scroll_mark.blink_flag ^= 1;
            self.scroll_mark.count = 0;
        }
    }

    /// Back to the thread list from a thread (cancel), or on to the next
    /// post to write out.
    fn leave_thread(&mut self, x: &mut BbsEnv) {
        let p = &mut self.msg_page;
        (p.th_start, p.th_extra, p.index, p.i_draw_state, p.msg_start, p.msg_last) = (0, 0, 0, 0, 0, 0);
        if self.write.b_write != 0 {
            self.init_writing_msg_page(x.save);
            self.write.b_write = 0;
        } else {
            self.draw_state = 0;
            for i in 0..self.threads.len() {
                let r = self.threads[i].bbs_check(x.save);
                self.threads[i].read = r;
            }
        }
    }

    /// `DrawThreadPage` (0x00402b30).
    fn draw_thread_page(&mut self, x: &mut BbsEnv) {
        self.thread_page.anm.draw(x.ctx);
        let th_num = self.th_num();
        if th_num == 0 {
            if x.cancel() {
                x.se(SE_BACK);
                self.draw_state = 3;
            }
            return;
        }
        let view = bbs_view();
        let tp = &mut self.thread_page;
        let mut s1 = tp.start_line;
        if tp.index < s1 {
            s1 = tp.index;
        }
        if tp.index >= s1 + 18 {
            s1 = tp.index - 17;
        }
        tp.start_line = s1;
        let s6 = (s1 + 18).min(th_num);
        if let Some(k) = tp.kanji.get_mut((tp.index - s1) as usize) {
            colour(k, C_SELECTED);
        }
        let mut y = 65;
        for s0 in s1..s6 {
            if self.threads[s0 as usize].bbs_check(x.save) == 1 {
                self.new_mark(50.0, y as f32);
            }
            let title = self.threads[s0 as usize].title.clone();
            if let Some(k) = self.thread_page.kanji.get_mut((s0 - s1) as usize) {
                (k.k.dx, k.k.dy) = (100.0, y as f32);
                k.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &title);
                colour(k, C_NORMAL);
            }
            y += 18;
        }
        self.thread_page.sb.set_range(0, th_num);
        self.thread_page.sb.set_pos(self.thread_page.start_line);
        let sb = self.thread_page.sb;
        self.bar(sb);
        if th_num - s6 > 0 {
            self.more_below(390.0);
        }
        if x.ok() {
            x.se(SE_OPEN);
            self.init_message_page();
            return;
        }
        if x.cancel() {
            x.se(SE_BACK);
            self.draw_state = 3;
            return;
        }
        if x.repeat(Buttons::UP) {
            x.se(SE_MOVE);
            self.thread_page.index -= 1;
            if self.thread_page.index < 0 {
                self.thread_page.index = th_num - 1;
            }
        }
        if x.repeat(Buttons::DOWN) {
            x.se(SE_MOVE);
            self.thread_page.index += 1;
            if th_num - 1 < self.thread_page.index {
                self.thread_page.index = 0;
            }
        }
    }

    /// `DrawMessagePage` (0x00403060): the thread's title and its posts as a
    /// tree, then choosing (`SelectMessage`) or reading (`ShowMessage`).
    fn draw_message_page(&mut self, a: &BbsAssets, x: &mut BbsEnv) {
        self.msg_page.anm.draw(x.ctx);
        let view = bbs_view();
        let Some(ti) = usize::try_from(self.thread_page.index).ok().filter(|&i| i < self.threads.len()) else {
            return;
        };
        let visibles = self.threads[ti].msgs.len() as i32;
        if visibles == 0 {
            let k = &mut self.msg_page.k_no_msg;
            (k.k.dx, k.k.dy) = (151.0, 100.0);
            k.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &a.no_msg);
            if x.cancel() {
                x.se(SE_BACK);
                self.leave_thread(x);
            }
            return;
        }
        let p = &mut self.msg_page;
        let pos = if p.th_extra != 0 { p.th_start + 1 } else { 0 };
        p.sb_high.set_range(0, visibles + 1);
        p.sb_high.set_pos(pos);
        let sb = p.sb_high;
        self.bar(sb);
        let p = &mut self.msg_page;
        let mut s2 = p.th_extra;
        let mut s3 = p.th_start;
        if p.index < s3 {
            s3 = p.index;
        }
        if p.index >= s3 + 6 + s2 {
            s3 = p.index - 6 - s2;
            s2 += 1;
            if s2 >= 2 {
                s2 = 1;
                s3 += 1;
            }
        }
        p.th_start = s3;
        p.th_extra = s2;
        let s7 = (s2 + s3 + 6).min(visibles);
        if s2 == 0 {
            let title = self.threads[ti].title.clone();
            let k = &mut self.msg_page.k_thread;
            (k.k.dx, k.k.dy) = (80.0, 65.0);
            k.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &title);
        }
        if s2 == 0 || s2 == 1 {
            // The tree's root.
            let y = 16 - s2 * 16 + 65;
            self.pkt(Cell { wu: 0, wv: 256, su: 16, sv: 16, sx: 16.0, sy: 16.0, dx: 80.0, dy: y as f32 });
            if s2 == 0 && self.threads[ti].bbs_check(x.save) == 1 {
                self.new_mark(48.0, 65.0);
            }
        }
        let mut y = if s2 == 0 { 97 } else { 81 };
        let sel = (self.msg_page.index - s3) as usize;
        if let Some(k) = self.msg_page.k_title.get_mut(sel) {
            colour(k, C_SELECTED);
        }
        let th_logical = self.threads[ti].logical;
        for (row, s) in (s3..s7).enumerate() {
            let (title, logical, dateindex) = {
                let m = &self.threads[ti].msgs[s as usize];
                (m.title.clone(), m.logical, m.dateindex)
            };
            if let Some(k) = self.msg_page.k_title.get_mut(row) {
                (k.k.dx, k.k.dy) = (112.0, y as f32);
                k.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &title);
                colour(k, C_NORMAL);
            }
            if bbs_state(x.save, th_logical, logical) == POST_NEW {
                self.new_mark(48.0, y as f32);
            }
            // The branch: the last post's corner, the others' tee.
            let wv = if s == visibles - 1 { 768 } else { 512 };
            self.pkt(Cell { wu: 0, wv, su: 16, sv: 16, sx: 16.0, sy: 16.0, dx: 80.0, dy: y as f32 });
            // The post icon.
            self.pkt(Cell { wu: 256, wv: 512, su: 16, sv: 16, sx: 16.0, sy: 16.0, dx: 96.0, dy: y as f32 });
            self.date(dateindex, title.len() as i32 * 8 + 126, y as f32);
            y += 16;
        }
        match self.msg_page.i_draw_state {
            1 => self.show_message(x, ti),
            0 => {
                if visibles - s7 > 0 {
                    self.more_below(195.0);
                }
                self.select_message(x, ti);
            }
            _ => {}
        }
    }

    /// `SelectMessage` (0x004038d0).
    fn select_message(&mut self, x: &mut BbsEnv, ti: usize) {
        let visibles = self.threads[ti].msgs.len() as i32;
        if x.repeat(Buttons::UP) {
            self.msg_page.index -= 1;
            x.se(SE_MOVE);
        }
        if x.repeat(Buttons::DOWN) {
            self.msg_page.index += 1;
            x.se(SE_MOVE);
        }
        let p = &mut self.msg_page;
        if p.index < 0 {
            p.th_extra -= 1;
            p.index = 0;
            if p.th_extra == -1 {
                p.index = visibles - 1;
                if p.th_start != 0 {
                    p.th_extra = 1;
                }
            }
        }
        if visibles - 1 < p.index {
            p.index = 0;
            p.th_extra = 0;
        }
        if p.th_extra < 0 {
            p.th_extra = 0;
        }
        if p.th_extra >= 2 {
            p.th_extra = 1;
        }
        if x.ok() {
            self.msg_page.i_draw_state = 1;
            x.se(SE_OPEN);
            return;
        }
        if x.cancel() {
            x.se(SE_BACK);
            self.leave_thread(x);
        }
    }

    /// `ShowMessage` (0x00403b30): the post under the cursor, 10 lines at a
    /// time from `msgStart`.
    fn show_message(&mut self, x: &mut BbsEnv, ti: usize) {
        let Some(mi) = usize::try_from(self.msg_page.index).ok().filter(|&i| i < self.threads[ti].msgs.len()) else {
            return;
        };
        let view = bbs_view();
        let (max_lines, dateindex, title_len) = {
            let m = &self.threads[ti].msgs[mi];
            (m.max_lines, m.dateindex, m.title.len() as i32)
        };
        let p = &mut self.msg_page;
        if max_lines - 7 < p.msg_start {
            p.msg_start = max_lines - 7;
        }
        if p.msg_start < 0 {
            p.msg_start = 0;
        }
        p.msg_last = p.msg_start + 10;
        if max_lines + 3 < p.msg_last {
            p.msg_last = max_lines + 3;
        }
        p.sb_low.set_range(0, max_lines + 3);
        p.sb_low.set_pos(p.msg_start);
        let (sb, start, last) = (p.sb_low, p.msg_start, p.msg_last);
        self.bar(sb);
        if max_lines + 3 - last > 0 {
            self.more_below(395.0);
        }
        if start == 0 {
            self.date(dateindex, title_len * 8 + 54, 216.0);
        }
        let mut y = 216;
        let mut k = 0usize;
        for s4 in start..last {
            if let Some(s) = self.threads[ti].msgs[mi].sentence(s4).map(<[u8]>::to_vec) {
                if let Some(t) = self.msg_page.k_msg.get_mut(k) {
                    (t.k.dx, t.k.dy) = (48.0, y as f32);
                    t.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &s);
                }
                k += 1;
            }
            y += 18;
        }
        if x.repeat(Buttons::UP) {
            self.msg_page.msg_start -= 1;
            if self.msg_page.msg_start < 0 {
                self.msg_page.msg_start = 0;
            }
        }
        if x.repeat(Buttons::DOWN) {
            self.msg_page.msg_start += 1;
        }
        if x.cancel() {
            x.se(SE_BACK);
            let p = &mut self.msg_page;
            (p.msg_start, p.msg_last, p.i_draw_state) = (0, 0, 0);
            let th = self.threads[ti].logical;
            let m = &mut self.threads[ti].msgs[mi];
            // SetSaveState(3): the post is read, in the save at once.
            m.read = i32::from(POST_READ);
            set_bbs_state(x.save, th, m.logical, POST_READ);
        }
    }

    /// `DrawWritingMsgPage` (0x00402530): the player's post typed out two
    /// bytes every six frames, a line after another; OK shows the rest, and
    /// once all is shown opens the thread on it.
    fn draw_writing_msg_page(&mut self, a: &BbsAssets, x: &mut BbsEnv) {
        let (Some(ti), Some(mi)) = (self.write.thread, self.write.msg) else { return };
        let view = bbs_view();
        self.msg_page.anm.draw(x.ctx);
        {
            let k = &mut self.write.k_new_msg;
            (k.k.dx, k.k.dy) = (100.0, 100.0);
            k.disp(x.ctx, x.fonts, x.names, ACTIVE_LAYER, &active_view(), &a.new_msg);
        }
        let (max_lines, dateindex, title_len) = {
            let m = &self.threads[ti].msgs[mi];
            (m.max_lines, m.dateindex, m.title.len() as i32)
        };
        let w = &self.write;
        let (mut frame_cnt, mut fp, mut s5, mut ncl, mut end, mut s6) =
            (w.frame_cnt, w.str_cnt, w.line_index, w.no_col_length, w.b_draw_end, w.start_line);
        if s5 < s6 {
            s6 = s5;
        }
        if end == 0 && s6 + 9 < s5 {
            s6 += 1;
        }
        let s7 = if s6 + 10 < s5 { s6 + 10 } else { s5 };
        if s6 == 0 {
            self.date(dateindex, title_len * 8 + 54, 216.0);
        }
        self.msg_page.sb_low.set_range(0, max_lines + 3);
        self.msg_page.sb_low.set_pos(s6);
        let sb = self.msg_page.sb_low;
        self.bar(sb);
        let mut y = 216;
        let mut k = 0usize;
        for s1 in s6..s7 {
            if let Some(s) = self.threads[ti].msgs[mi].sentence(s1).map(<[u8]>::to_vec) {
                if let Some(t) = self.msg_page.k_msg.get_mut(k) {
                    (t.k.dx, t.k.dy) = (40.0, y as f32);
                    t.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &s);
                }
                k += 1;
            }
            y += 18;
        }
        if s5 < max_lines + 3
            && let Some(s) = self.threads[ti].msgs[mi].sentence(s5)
        {
            let buf = str_range_copy(s, 0, fp);
            if let Some(t) = self.msg_page.k_msg.get_mut(k) {
                (t.k.dx, t.k.dy) = (40.0, y as f32);
                t.disp(x.ctx, x.fonts, x.names, BBS_LAYER, &view, &buf);
            }
        }
        frame_cnt += 1;
        if frame_cnt >= 6 {
            fp += 1;
            frame_cnt = 0;
        }
        if (ncl >> 1) < fp {
            s5 += 1;
            ncl = -1;
            if s5 < max_lines + 3 {
                ncl = self.threads[ti].msgs[mi].line_length(s5 - 3);
            }
            fp = 0;
        }
        if max_lines + 2 < s5 {
            end = 1;
            s5 = max_lines + 3;
        }
        if x.ok() {
            if end != 0 {
                self.exit_writing_msg_page(x.save);
            } else {
                s5 = max_lines + 3;
            }
        }
        let w = &mut self.write;
        (w.frame_cnt, w.str_cnt, w.line_index, w.no_col_length, w.b_draw_end, w.start_line) =
            (frame_cnt, fp, s5, ncl, end, s6);
    }
}
