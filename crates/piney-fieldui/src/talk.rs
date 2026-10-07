//! The seam of the menus the action button opens (gcmn `menu.cpp`: 21-27
//! and their pages 47-56): who is spoken to, what the menus ask of the
//! world, and the state `ccMenuCtrl` keeps for them. The menus read the
//! character spoken to through `cmndTarget` (or `cmndTargetPrev` once a
//! shop page has dropped it): for a town NPC its `npcTbl` row (gcmn
//! 0x00619460), which the runtime names with [`crate::FieldUi::talk_to`]
//! when `ccThGameCtrl` opens the menu. The lines are the volume's talk
//! records (`piney_data::tables::talk`). See docs/engine/field-ui.md.

use piney_data::tables::types::EvMsgData;
use piney_data::tables::{battle, fieldui, sjis};
use piney_data::volume::Volume;

use piney_desktop::staffroll::CcRand;

use crate::ctrl::{Cont, Ctx, MenuCtrl};
use crate::{Request, menus};

/// `saveData.spcParam[18]` (+0x7488, 0xdc bytes each), `ccSpcParam` with
/// its `ccCharBaseParam` first.
pub const SPC_PARAM: usize = 0x7488;
pub const SPC_PARAM_SIZE: usize = 0xdc;

/// Who a target is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    /// `npcTbl[row]`: a merchant, a walking PC, an administrator, a
    /// Grunty breeder.
    Npc(i32),
    /// A party member, `saveData.spcParam[id]` (`SpcMenu`, 21).
    Spc(i32),
}

/// The character the action button spoke to, as `ccThGameCtrl` leaves it
/// for the menus: `handle` is the one the runtime gives it in
/// [`crate::World`] (`cmndTarget`, later `cmndTargetPrev`), `who` its base
/// parameters. (`game.server`, which picks a merchant's greeting and stock,
/// is [`crate::Game::server`].)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TalkTarget {
    pub handle: u32,
    pub who: Speaker,
}

/// `ccCharBaseParam` (0x24 bytes) as the talk menus read it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Base {
    /// +0x00 `name`.
    pub name: Vec<u8>,
    /// +0x08 `type`: 0x08 a walking PC, 0x10 an administrator, 0x100 the
    /// weapon shop, 0x200 Elf's Haven, 0x400 the item shop, 0x800 the magic
    /// shop, 0x1000 the Recorder, 0x8000000 a Grunty breeder.
    pub types: u32,
    /// +0x0c `id`: the `npcTbl` row.
    pub id: i16,
    pub level: i16,
    pub exp: i16,
    pub gold: i32,
    /// +0x20 `msg`: the address of the character's line table.
    pub msg: u32,
}

impl Base {
    /// `npcTbl[row].param.base` (`battle::npcs`).
    pub fn npc(volume: Volume, row: i32) -> Option<Base> {
        let b = &battle::of(volume).npcs().get(usize::try_from(row).ok()?)?.param.base;
        Some(Base {
            name: b.name.map(sjis::encode).unwrap_or_default(),
            types: b.kind as u32,
            id: b.id,
            level: b.level,
            exp: b.exp,
            gold: b.gold,
            msg: b.msg,
        })
    }
}

/// A `ccEvMsgData` (0xc bytes: `emode`, `name`, `str`) as
/// `ccMessage::Open` / `Change` take it: `str`'s first three lines
/// (`ccKanjiStrSeparate`). A null record, or one without text, is
/// `errorData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvMsg {
    pub emode: i32,
    pub lines: [Vec<u8>; 3],
}

/// The talk menus' texts: the volume's talk records, and each page's
/// words.
#[derive(Clone, Debug, Default)]
pub struct TalkTexts {
    pub volume: Volume,
    pub merchant: menus::merchant::Texts,
    pub talk: menus::talk::Texts,
    pub trade: menus::trade::Texts,
    pub shop: menus::shop::Texts,
    pub record: menus::record::Texts,
    pub breeder: menus::breeder::Texts,
}

impl TalkTexts {
    pub fn read(volume: Volume) -> piney_data::Result<TalkTexts> {
        Ok(TalkTexts {
            volume,
            merchant: menus::merchant::Texts::default(),
            talk: menus::talk::Texts::of(volume),
            trade: menus::trade::Texts::of(volume),
            shop: menus::shop::Texts::of(volume),
            record: menus::record::Texts::of(volume),
            breeder: menus::breeder::Texts::of(volume),
        })
    }

    /// The record at `p` (a `ccEvMsgData *`).
    /// A null record, or one without text, is `errorData`
    /// (`ccMessage::Open`).
    pub fn ev_msg(&self, p: u32) -> EvMsg {
        use crate::tables::piece;
        let t = fieldui::of(self.volume);
        let error = t.error_data();
        let r = match self.record(p) {
            Some(r) if p != t.error_data_va() && r.str.is_some() => r,
            Some(_) => &error,
            // A pointer table read as a record (an NPC past the PCs, whose
            // `msg` the talk page takes for its record): `emode` is its first
            // word, `str` its third, and the text what lies there.
            None if p != 0 && p != t.error_data_va() && self.word(p.wrapping_add(8)) != 0 => {
                let raw = self.bytes_at(self.word(p.wrapping_add(8)), 512);
                return EvMsg { emode: self.word(p) as i32, lines: [0, 1, 2].map(|k| separate(&raw, k)) };
            }
            None => &error,
        };
        let l = r.str.unwrap_or_default();
        EvMsg { emode: r.emode, lines: [piece(l, 0), piece(l, 1), piece(l, 2)] }
    }

    /// Up to `n` of the bytes the game reads at `va`, as far as the volume's
    /// record arrays (each record's `emode` and the addresses it holds) and
    /// pointer tables tell them.
    fn bytes_at(&self, va: u32, n: usize) -> Vec<u8> {
        let talk = piney_data::tables::talk::of(self.volume);
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let at = va.wrapping_add(out.len() as u32);
            let arrays = talk.records();
            let word = match arrays.partition_point(|a| a.va <= at).checked_sub(1).map(|i| &arrays[i]) {
                Some(a) if ((at - a.va) / 12) < a.records.len() as u32 => {
                    let k = ((at - a.va) / 12) as usize;
                    let (r, v) = (&a.records[k], &a.addresses[k]);
                    [r.emode as u32, v.name, v.str][((at - a.va) % 12 / 4) as usize]
                }
                _ if self.in_table(at) => self.word(at & !3),
                _ => break,
            };
            out.extend_from_slice(&word.to_le_bytes()[(at % 4) as usize..]);
        }
        out.truncate(n);
        out
    }

    /// Whether one of the volume's pointer tables holds `p`.
    fn in_table(&self, p: u32) -> bool {
        let tables = piney_data::tables::talk::of(self.volume).pointers();
        tables
            .partition_point(|t| t.va <= p)
            .checked_sub(1)
            .is_some_and(|i| p - tables[i].va < 4 * tables[i].words.len() as u32)
    }

    /// The record at `p`, where one of the volume's record arrays holds it.
    pub fn record(&self, p: u32) -> Option<&'static EvMsgData> {
        let arrays = piney_data::tables::talk::of(self.volume).records();
        let a = &arrays[arrays.partition_point(|a| a.va <= p).checked_sub(1)?];
        let off = p - a.va;
        if !off.is_multiple_of(12) {
            return None;
        }
        a.records.get((off / 12) as usize)
    }

    /// Whether the record at `p` has text (`str` set).
    pub fn has_text(&self, p: u32) -> bool {
        match self.record(p) {
            Some(r) => r.str.is_some(),
            None => self.word(p.wrapping_add(8)) != 0,
        }
    }

    /// The word at `p` in one of the volume's pointer tables, 0 where none
    /// holds it.
    pub fn word(&self, p: u32) -> u32 {
        let tables = piney_data::tables::talk::of(self.volume).pointers();
        let Some(i) = tables.partition_point(|t| t.va <= p).checked_sub(1) else { return 0 };
        let off = p - tables[i].va;
        if !off.is_multiple_of(4) {
            return 0;
        }
        tables[i].words.get((off / 4) as usize).copied().unwrap_or(0)
    }
}

/// `ccKanjiStrSeparate(s, k)`: the `k`th NUL-ended piece of a text, a byte
/// from 0x20 to 0x7f one character and any other the first of two.
pub fn separate(raw: &[u8], k: usize) -> Vec<u8> {
    let mut at = 0usize;
    for _ in 0..k {
        while at < raw.len() && raw[at] != 0 {
            at += if (0x20..0x80).contains(&raw[at]) { 1 } else { 2 };
        }
        at += 1;
    }
    let start = at.min(raw.len());
    let end = raw[start..].iter().position(|&c| c == 0).map_or(raw.len(), |e| start + e);
    raw[start..end].to_vec()
}

/// The state `ccMenuCtrl` keeps for these menus, and the seam's.
pub struct TalkState {
    /// +0xf6 `talkNum`: which line a PC says (PcMenu, TalkMenu).
    pub talk_num: i16,
    /// +0xfa `talkTradeFlag`, +0xfc `talkLoopCnt`.
    pub talk_trade_flag: i16,
    pub talk_loop_cnt: i16,
    /// +0x18c `temp[8]`: the pages' scratch (counts, prices, slots).
    pub temp: [i32; 8],
    /// +0x1ac `drainItem[17]`: the trade pages' items (`category << 16 |
    /// id`, -1 none; what the trader gives, then up to three offered), and
    /// Data Drain's drops (the aimed target's, then the area's), which
    /// `DataDrainSubMenu` hands out.
    pub drain_item: [i32; 17],
    /// Who is spoken to ([`crate::FieldUi::talk_to`]).
    pub target: Option<TalkTarget>,
    /// The record `ccMsg` last opened from a table (`ccMessage` +0x44), its
    /// name (+0x3c) and voice (+0x190 group, +0x194 message): `Check`
    /// chains an emode-1 record to the next one in memory.
    pub chain: Option<Chain>,
    /// `saveSys` and the memory cards (the Recorder's Save).
    pub record: menus::record::State,
    /// `ccRand()` (NorainuMenu's `talkNum`). The menus' `rand()` is
    /// [`crate::ctrl::MenuRand`].
    cc: MenuCc,
}

/// `ccRand()` as the menus draw it.
pub enum MenuCc {
    /// The game's Mersenne Twister: [`crate::FieldUi`] loads it from
    /// `SaveState::cc` (the town's or the area's, lent by the host) before
    /// the task's frame and stores it back after.
    Game(CcRand),
    /// A harness's numbers in its place; `SaveState::cc` is left alone.
    Fixed(Box<dyn FnMut() -> i32 + Send>),
}

impl Default for TalkState {
    fn default() -> Self {
        TalkState {
            talk_num: 0,
            talk_trade_flag: 0,
            talk_loop_cnt: 0,
            temp: [0; 8],
            drain_item: [0; 17],
            target: None,
            chain: None,
            record: Default::default(),
            cc: MenuCc::Game(CcRand::default()),
        }
    }
}

impl TalkState {
    /// `ccRand()`.
    pub fn cc_rand(&mut self) -> i32 {
        match &mut self.cc {
            MenuCc::Game(c) => c.rand(),
            MenuCc::Fixed(f) => f(),
        }
    }

    /// A harness's numbers for `ccRand()`.
    pub fn set_cc_rand(&mut self, f: Box<dyn FnMut() -> i32 + Send>) {
        self.cc = MenuCc::Fixed(f);
    }

    /// The game's `ccRand` at `state` (a fixed source keeps its numbers).
    pub fn load_cc(&mut self, state: &CcRand) {
        if let MenuCc::Game(c) = &mut self.cc {
            c.clone_from(state);
        }
    }

    /// The game's `ccRand` as the menus left it, into `state`.
    pub fn store_cc(&self, state: &mut CcRand) {
        if let MenuCc::Game(c) = &self.cc {
            state.clone_from(c);
        }
    }

    /// What outlives a `ccMenuCtrl` ([`crate::MenuCtrl::keep`]): `saveSys`
    /// with the cards, and `ccRand`. The members start afresh.
    pub fn keep(&mut self, old: TalkState) {
        self.record = old.record;
        self.cc = old.cc;
    }
}

/// A `ccEvMsgData` in a table, as `ccMsg` holds it for chaining.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chain {
    pub rec: u32,
    pub name: Vec<u8>,
    pub grp: i32,
    pub msg: i32,
}

/// `ccMsg->Open(ccEvMsgData *rec, name, grp, msg)` (main 0x001a5790): the
/// speech window afresh with a table's record (`errorData` for none), its
/// voice asked for when `grp` is not -1.
pub fn open_record(m: &mut MenuCtrl, x: &mut Ctx, rec: u32, name: &[u8], grp: i32, msg: i32) {
    let r = x.texts.talk.ev_msg(rec);
    if grp != -1 {
        x.req.push(Request::VoiceRequest { grp, msg });
    }
    let names = x.save.names();
    let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
    m.msg.open_record(r.emode, Some(name), &lines, &names);
    let err = fieldui::of(x.texts.volume).error_data_va();
    let rec = if rec == 0 || !x.texts.talk.has_text(rec) { err } else { rec };
    m.talk.chain = Some(Chain { rec, name: name.to_vec(), grp, msg });
    m.note(|| format!("[\"msg_record\",{rec}]"));
}

/// `ccMsg->Open(ccMsgData *m, name, -1, -1)` (main 0x001a53e0) with a
/// record the menu built: `str[1..3]` the lines.
pub fn open_data(m: &mut MenuCtrl, x: &Ctx, emode: i32, name: &[u8], lines: [&[u8]; 3]) {
    let names = x.save.names();
    m.msg.open(emode, Some(name), [Some(lines[0]), Some(lines[1]), Some(lines[2])], &names);
    m.talk.chain = None;
}

/// `ccMsg->Check(1)` (main 0x001a5960): as `Check(0)`, and a record of
/// emode 1 whose close count runs out is followed by the next record of
/// its table (`Change(rec + 1, name, grp, msg + 1)`, the voice with it).
pub fn msg_check(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let w = m.msg.wait_close_cnt;
    let chains = m.msg.emode & 0xff == 1 && w != 0 && (w - 1).min(2) <= 0;
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    crate::menus::system::push_msg_requests(x, req);
    if chains && let Some(c) = m.talk.chain.as_mut() {
        if c.grp != -1 {
            c.msg += 1;
        }
        c.rec = c.rec.wrapping_add(12);
        if !x.texts.talk.has_text(c.rec) {
            c.rec = piney_data::tables::fieldui::of(x.texts.volume).error_data_va();
        }
        let c = c.clone();
        let r = x.texts.talk.ev_msg(c.rec);
        if c.grp != -1 {
            x.req.push(Request::VoiceRequest { grp: c.grp, msg: c.msg });
        }
        let names = x.save.names();
        let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
        m.msg.change_record(r.emode, Some(&c.name), &lines, &names);
    }
    r
}

/// The base parameters of the character with this handle (the talk
/// target, when it is the one the seam named).
pub fn base_of(m: &MenuCtrl, x: &Ctx, handle: u32) -> Option<Base> {
    let t = m.talk.target.filter(|t| t.handle == handle)?;
    match t.who {
        // A Grunty's base moves as it grows (Little Grunty to the Kid to its
        // grown kind, `adultSetup`): the game reads `base` live, so its row
        // now, not the one it was spoken to as.
        Speaker::Npc(row) => {
            let row = x.world.grunty.as_ref().filter(|g| g.handle == handle).map_or(row, |g| g.row);
            Base::npc(x.texts.volume, row)
        }
        Speaker::Spc(id) => {
            // spcParam[id].base: the save's record; its name is charTbl's
            // (the save keeps a pointer).
            let at = SPC_PARAM + SPC_PARAM_SIZE * id.clamp(0, 17) as usize;
            let s = &x.save.save;
            Some(Base {
                name: crate::disp::member_name(x, id),
                types: s.i32(at + 8) as u32,
                id: s.i16(at + 0x0c),
                level: s.i16(at + 0x0e),
                exp: s.i16(at + 0x10),
                gold: s.i32(at + 0x14),
                msg: s.i32(at + 0x20) as u32,
            })
        }
    }
}

/// `cmndTarget->base`: `None` when there is no target (or the seam did not
/// name it).
pub fn target_base(m: &MenuCtrl, x: &Ctx) -> Option<Base> {
    let h = x.target.as_ref()?.handle;
    base_of(m, x, h)
}

/// `cmndTargetPrev->base`.
pub fn prev_base(m: &MenuCtrl, x: &Ctx) -> Option<Base> {
    let h = x.target_prev.as_ref()?.handle;
    base_of(m, x, h)
}

/// What these menus ask of the rest of the game.
#[derive(Clone, Debug, PartialEq)]
pub enum TalkReq {
    /// `changeCamera(n)` (main 0x00161ff0): 1 the player's camera back.
    Camera(i32),
    /// `ccMenuCtrl::SetMerchantCamera()` (gcmn 0x005269d0): the camera
    /// turned to the merchant (`changeCamera(3)`; a breeder's fixed view,
    /// else 500 in front of `cmndTarget` at its height plus 100, facing
    /// it).
    MerchantCamera,
    /// `ccUseItemRequest(spc, spc, code, 0)` (gcmn 0x0057aa80): a party
    /// member reads a book it was given in a trade (`AddSpcItem`).
    SpcUseItem { spc: u32, code: i32 },
    /// `ccSpcMessagePresentOtherFellow(c)` (gcmn): the other party members
    /// remark on the gift member `c` was given.
    PresentOther(u32),
    /// `ccChar::EntryAffect(grunty, plw, 19, food, num, 0)`: the Grunty
    /// eats `num` of food `food` (key item 26 + food).
    Feed { target: u32, food: i32, num: i32 },
    /// `pg->growthNum = v`: the menu's write to the Grunty (4: its talk
    /// is over).
    GruntyGrowth { target: u32, growth: i8 },
    /// InuMenu's writes to the Grunty: `foodMode` (+0x307) and, with Give
    /// Food, `chatFlag` (+0x306).
    GruntyFood { target: u32, food_mode: u8, chat_flag: Option<u8> },
    /// `ccMenuCtrl::SetFountainCamera()` (gcmn 0x00526c60): the event
    /// camera on the spring (`cmndTargetPrev`) from behind Kite.
    FountainCamera,
    /// `FountainMenu3`'s hold of the party (true, step 2:
    /// `ccStoreSpcCondition`; each member manual, remote command 0, off
    /// the command lists, its conditions cleared, the others' +0x110 0;
    /// `ccSpcConditionEffectOFF`; `ccClearConditionAllEnemy`) and its
    /// release (false, step 14: each member on the lists, its condition
    /// restored and adjusted, skill 0, manual off, the others' +0x110 1
    /// unless down; `ccSpcConditionEffectON`).
    FountainParty(bool),
}

impl TalkReq {
    /// The request as `tools/test_fieldui_*` logs it.
    pub fn trace(&self) -> String {
        match self {
            TalkReq::Camera(n) => format!("[\"camera\",{n}]"),
            TalkReq::MerchantCamera => "[\"merchant_camera\"]".into(),
            TalkReq::SpcUseItem { spc, code } => format!("[\"use_item\",{spc},{code}]"),
            TalkReq::PresentOther(c) => format!("[\"present_other\",{c}]"),
            TalkReq::Feed { target, food, num } => format!("[\"affect\",{target},19,{food},{num}]"),
            TalkReq::GruntyGrowth { target, growth } => format!("[\"pg_growth\",{target},{growth}]"),
            TalkReq::GruntyFood { target, food_mode, chat_flag } => {
                format!("[\"pg_food\",{target},{food_mode},{}]", chat_flag.map_or(-1, i32::from))
            }
            TalkReq::FountainCamera => "[\"fountain_camera\"]".into(),
            TalkReq::FountainParty(on) => format!("[\"fountain_party\",{}]", u8::from(*on)),
        }
    }
}

/// Pushes a [`TalkReq`].
pub fn req(x: &mut Ctx, r: TalkReq) {
    x.req.push(Request::Talk(r));
}

/// `ccSleepAllThread(); StillOn()` as the pages do it inline: every task
/// but the menu's asleep, the other layers frozen but the noise, still and
/// game layers.
pub fn sleep_all(m: &mut MenuCtrl, x: &mut Ctx) {
    x.req.push(Request::SleepAll);
    m.still = 1;
    x.req.push(Request::Still(true));
    x.req.push(Request::KeepLayers);
}

/// `ccChar::EntryAffect(c, plw, kind, 0, 0, 0)`: 14 as a talk opens (the
/// NPC stops and faces Kite), 15 as it speaks, 0 as it ends.
pub fn affect(x: &mut Ctx, target: u32, kind: i16) {
    x.req.push(Request::Affect { target, kind });
}

/// The inlined `CloseMenu` the pages use: the window and dim closing, the
/// tasks woken, `Disp`, the breath. `then` runs next frame after the
/// close's own tail (`still = 0` and the flips when it woke, `firstTime =
/// 0`).
pub fn close(m: &mut MenuCtrl, x: &mut Ctx, then: Then) -> crate::ctrl::Flow {
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    breathed(Tail::Closed { woke, then })
}

/// A page breathed: `t` runs next frame.
pub fn breathed(t: Tail) -> crate::ctrl::Flow {
    crate::ctrl::Flow::Breathed(Cont { woke: false, cursors: false, after: crate::ctrl::After::Talk(t) })
}

/// The rest of a page after a breath, by page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// After [`close`]'s breath: its tail, then the page's.
    Closed {
        woke: bool,
        then: Then,
    },
    /// A page's own continuation (it runs its close's tail itself).
    Merchant(menus::merchant::Tail),
    Talk(menus::talk::Tail),
    Trade(menus::trade::Tail),
    Shop(menus::shop::Tail),
    Record(menus::record::Tail),
    Breeder(menus::breeder::Tail),
    Inu(menus::inu::Tail),
}

/// What a page does after its close's tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    Nothing,
    /// `ccMsg->Close()`.
    MsgClose,
    Merchant(menus::merchant::Tail),
    Talk(menus::talk::Tail),
    Trade(menus::trade::Tail),
    Shop(menus::shop::Tail),
    Record(menus::record::Tail),
    Breeder(menus::breeder::Tail),
    Inu(menus::inu::Tail),
}

/// Runs a [`Tail`] (the frame after the breath). `Some` when it breathed
/// again (its own `Disp` done), `None` when it returned to the task loop,
/// whose `Disp` ends the frame.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::Closed { woke, then } => {
            if woke {
                m.still = 0;
                x.req.push(Request::Still(false));
            }
            m.first_time = 0;
            match then {
                Then::Nothing => None,
                Then::MsgClose => {
                    m.msg.close();
                    None
                }
                Then::Merchant(t) => menus::merchant::tail(m, t, x),
                Then::Talk(t) => menus::talk::tail(m, t, x),
                Then::Trade(t) => menus::trade::tail(m, t, x),
                Then::Shop(t) => menus::shop::tail(m, t, x),
                Then::Record(t) => menus::record::tail(m, t, x),
                Then::Breeder(t) => menus::breeder::tail(m, t, x),
                Then::Inu(t) => menus::inu::tail(m, t, x),
            }
        }
        Tail::Merchant(t) => menus::merchant::tail(m, t, x),
        Tail::Talk(t) => menus::talk::tail(m, t, x),
        Tail::Trade(t) => menus::trade::tail(m, t, x),
        Tail::Shop(t) => menus::shop::tail(m, t, x),
        Tail::Record(t) => menus::record::tail(m, t, x),
        Tail::Breeder(t) => menus::breeder::tail(m, t, x),
        Tail::Inu(t) => menus::inu::tail(m, t, x),
    }
}

/// A `Flow` that continues a page next frame.
pub fn then(flow: crate::ctrl::Flow) -> Option<Cont> {
    match flow {
        crate::ctrl::Flow::Breathed(c) => Some(c),
        crate::ctrl::Flow::Done => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cc_rand_is_the_saves() {
        // The menus draw on from the state the host lent, and hand it back.
        let mut lent = CcRand::seeded(4352);
        let mut want = lent.clone();
        let mut s = TalkState::default();
        s.load_cc(&lent);
        assert_eq!([s.cc_rand(), s.cc_rand()], [want.rand(), want.rand()]);
        s.store_cc(&mut lent);
        assert_eq!(lent, want);
        s.set_cc_rand(Box::new(|| 7));
        s.load_cc(&CcRand::default());
        assert_eq!(s.cc_rand(), 7);
        s.store_cc(&mut lent);
        assert_eq!(lent, want);
    }
}
