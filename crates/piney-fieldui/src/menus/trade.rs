//! Trading with a PC (or a party member): `TradeMenu` (gcmn 0x0054e720,
//! menu 48: what the trader offers), `TradeSubMenu` (0x0054f4b0, 49: what
//! Kite gives for it) and `TradeMenuDisp` (0x00551570), the page both
//! draw. A party member is handed what it traded for by
//! `ccMenuCtrl::AddSpcItem` ([`crate::menus::talk::add_spc_item`], which
//! Gift shares).
//!
//! PcMenu (22) drops the target (it is `cmndTargetPrev` here) and goes into
//! 48. Who trades what (each page builds the lists again every frame):
//!
//! ```text
//! the bag       saveData.itemList[0][40] (+0x30): every held slot but key
//!               items (category 15)
//! the offers    a party member (type 4): spcTradeList[id - 1][16] (+0xe3c)
//!               a trading PC (type 0x02000018, id 66-79): tpcTradeList
//!               [id - 66][3] (main 0x00347f90), four ccItemLists a trade
//!               (what it gives, then up to three it wants), each while
//!               tpcTradeListSW[id - 66][k] (+0x1e7c) is set
//!               another PC: npcTradeList[row][16] (+0x127c), row id - 30
//!               (145-153: id - 109)
//! ```
//!
//! ```text
//! TradeMenu (48)
//! proccess 0   talkTradeFlag 1; the page (disp 4). An empty bag: 20;
//!              nothing offered: 30. Unless the list's index is set: temp[8]
//!              0, drainItem[0..4] -1, "Please select an item ..."
//!              (tradeMenuHelp[0], OpenInfo). The dim; the other tasks
//!              asleep; the list shut (y, my 0)
//! proccess 1   until Check (the index 0), else 6 in waitCount
//! proccess 2   waitCount to 7: the offers (my, y at most 8), the window in,
//!              exceptionDisp 1
//! proccess 3   SelectScr (select, dy kept in temp[4], temp[5]); triangle on
//!              equipment: its status (64); cancel (19): 100; OK (18): the
//!              item in drainItem[0], its count in temp[0], into 49. The
//!              item's name and comment (DispMsg)
//! 20, 30       "There is no item to trade." (tradeMenuHelp[3]); 21, 31
//!              until Check: 100
//! 40, 41       tradeMenuHelp[4]; until Check: 100
//! proccess 100 the window and dim out, the target back (ccChangeCmndTarget
//!              (cmndTargetPrev)); with mode 1 and the tasks asleep: woken,
//!              Disp, a breath, the flips back on; back to the list (22)
//!
//! TradeSubMenu (49)
//! proccess 0   the page; unless the index is set "Select item(s) to trade
//!              with ..." (tradeMenuHelp[1])
//! proccess 1-2 as 48's, over the bag (exceptionDisp 2)
//! proccess 3   SelectScr (temp[6], temp[7]); triangle as 48's; the keys
//!              read from the pad's repeat:
//!              cancel on an item offered: one fewer (none left: out of the
//!                offer; sound 19); on another, pushed: 100
//!              OK: the trade balanced (a trading PC: each thing it wants
//!                offered in its count; else the offer's worth at least the
//!                item's, below) and OK pushed: 4 (sound 18); not balanced:
//!                one more of the item offered (a new one takes the first
//!                free of drainItem[1..4], up to three; sound 18)
//! proccess 4   the window gone: OK / Cancel (disp 11), "Trade under these
//!              conditions?" (tradeMenuHelp[2])
//! proccess 5   Select; cancel, or Cancel: 100; OK: 6
//! proccess 6   the window gone: 99 of the item after it: 40 with
//!              tradeMenuHelp[5]; the bag full, the item not held and no
//!              offered stack given up whole: 40 with tradeMenuHelp[4].
//!              Else DelItem of each offered, AddItem of the item (sound
//!              74), AddTradeCount, EntryAffect(cmndTargetPrev, 0), the
//!              trade struck off the trader's list, "You now have #G<item>
//!              #W!"
//! proccess 7   until Check: a PC: the menu shut (CloseMenu inlined); a
//!              member: 8
//! 8 - 19       a member: after 10 frames each book given it (category 12)
//!              read ("<name> used #G<book>#W.", sound 92); each piece of
//!              equipment with a price it may wear equipped when dearer
//!              than its own ("<name> equipped ..."); the rest into its
//!              bag (AddSpcItem); cmndTargetFix 0, EntryAffect 0, shut
//! 20 - 41      as 48's (40: tradeMenuHelp[waitCount], its third line when
//!              it has one)
//! proccess 100 as 48's, and 49's prev becomes 48's (back to 22)
//! ```
//!
//! The worth (TradeSubMenu's OK and the page's gauge): `temp[k] *
//! ccGetItemPrice(item k)` for each, an offered item the same as the one
//! wanted counting nothing (member 1's 14/10 counts 50000 a piece), each
//! times its rate (`ccGetItemTradeRate` over the trader's row of
//! `spcTradeRateTbl` or `npcTradeRateTbl`, 14 shorts: categories 10-14,
//! the three armour classes, the six weapons; 10 by default); a member
//! adds `friendship / 200` (at most 5) to the offered items' rates.

use piney_battle::item as bitem;
use piney_desktop::eef::{add, from_int, sub};

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::items::{self, Item};
use crate::menus::merchant::{TPC_TRADE_LIST_SW, add_trade_count};
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::menus::talk::{Added, SpcGift, add_spc_item, add_spc_item_after};
use crate::spr::{Spr, font_type, make_num, set_clm};
use crate::talk::{self, SPC_PARAM, SPC_PARAM_SIZE, Then};
use crate::window::{disp_square, disp_square_sb, disp_target, set_type};

/// `tradeMenuHelp[7]` (main 0x0033ec40): the pages' help, questions and
/// refusals, up to three lines each.
pub const TRADE_MENU_HELPS: u32 = 7;
/// `presentMenuHelp[6]` (main 0x0033ec60): [4] " equipped ", [5] " used ".
pub const PRESENT_MENU_HELPS: u32 = 6;
/// `tradeMenuStr` (main .sdata 0x00377e38): the gauge's eight glyphs and
/// "Approve", 16 glyphs a row.
/// `getItemMenuStr` (main .sdata 0x00377e0c): its first line, "You now
/// have ".
/// gcmn's literals `@13687` "#G", `@13688` "#W!", `@7962` "#W.".
pub const STR_GREEN: &[u8] = b"#G";
pub const STR_BANG: &[u8] = b"#W!";
pub const STR_DOT: &[u8] = b"#W.";
/// The save's lists: `spcTradeList[17][16]` (+0xe3c), `npcTradeList[48][16]`
/// (+0x127c), 64 bytes a row.
pub const SPC_TRADE_LIST: i64 = 0x0e3c;
pub const NPC_TRADE_LIST: i64 = 0x127c;
/// `ccSpcParam.friendship` (+0xda), which a member's rates add to.
pub const SPC_FRIENDSHIP: usize = 0xda;
/// The sounds: the trade made, a book read.
pub const SE_TRADE: i32 = 74;
pub const SE_BOOK: i32 = 92;

/// The pages' texts, read from the executable once.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    /// `tradeMenuHelp[k]`'s three lines (`ccKanjiStrSeparate` 0-2).
    pub help: Vec<[Vec<u8>; 3]>,
    /// `presentMenuHelp[k]`'s first line.
    pub present: Vec<Vec<u8>>,
    pub menu_str: Vec<u8>,
    pub get_item: Vec<u8>,
    pub green: Vec<u8>,
    pub bang: Vec<u8>,
    pub dot: Vec<u8>,
}

impl Texts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> Texts {
        use crate::tables::piece;
        use piney_data::tables::sjis::encode;
        let f = piney_data::tables::fieldui::of(volume);
        Texts {
            help: f.trade_help().iter().map(|l| [piece(l, 0), piece(l, 1), piece(l, 2)]).collect(),
            present: f.present_help().iter().map(|l| piece(l, 0)).collect(),
            menu_str: encode(f.trade_menu_str()),
            get_item: piece(f.get_item_str(), 0),
            green: STR_GREEN.to_vec(),
            bang: STR_BANG.to_vec(),
            dot: STR_DOT.to_vec(),
        }
    }
}

/// What the pages do after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// `proccess 100` after its breath: the flips back on, back to the
    /// list (49: its prev made 48's first).
    Leave { sub: bool },
    /// `AddSpcItem` after its equipment thread's breath, then the page.
    Spc { gift: SpcGift, then: SpcThen },
}

/// Where TradeSubMenu goes on after an `AddSpcItem` that breathed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpcThen {
    /// `proccess 15`'s item `k`.
    Equip(usize),
    /// `proccess 19`'s loop, after item `k`.
    Rest(usize),
}

/// The pages' tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::Leave { sub } => {
            m.still = 0;
            x.req.push(Request::Still(false));
            back(m, sub);
            None
        }
        Tail::Spc { gift, then } => {
            let tr = trader(m, x).unwrap_or_default();
            let r = add_spc_item_after(x, gift);
            match then {
                SpcThen::Equip(k) => {
                    equipped(m, x, &tr, k, r);
                    None
                }
                SpcThen::Rest(k) => talk::then(rest(m, x, &tr, k + 1)),
            }
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

/// Where a trader's offers are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Offers {
    /// 16 entries at a save offset (`spcTradeList`, `npcTradeList`).
    Save(i64),
    /// A trading PC's three trades: its row of `tpcTradeList`.
    Tpc(usize),
    /// A character of neither kind: the game reads the list from a stale
    /// register; the port offers nothing.
    #[default]
    None,
}

/// `cmndTargetPrev` as the pages read it.
#[derive(Clone, Debug, Default)]
struct Trader {
    handle: u32,
    types: u32,
    id: i32,
    name: Vec<u8>,
    offers: Offers,
}

impl Trader {
    fn trading(&self) -> bool {
        matches!(self.offers, Offers::Tpc(_))
    }
}

/// A PC's row of `npcTradeList` and `npcTradeRateTbl`.
fn npc_row(id: i32) -> i32 {
    if (145..154).contains(&id) { id - 109 } else { id - 30 }
}

/// `cmndTargetPrev`, `None` when there is none.
fn trader(m: &MenuCtrl, x: &Ctx) -> Option<Trader> {
    let prev = x.target_prev.as_ref()?;
    let base = talk::prev_base(m, x).unwrap_or_default();
    let (types, id) = (base.types, i32::from(base.id));
    let offers = if types & 4 != 0 {
        Offers::Save(SPC_TRADE_LIST + 64 * i64::from(id - 1))
    } else if types & 0x0200_0018 != 0 {
        if (66..80).contains(&id) {
            Offers::Tpc((id - 66) as usize)
        } else {
            Offers::Save(NPC_TRADE_LIST + 64 * i64::from(npc_row(id)))
        }
    } else {
        Offers::None
    };
    Some(Trader { handle: prev.handle, types, id, name: base.name, offers })
}

/// A `ccItemList` in the save (`{-1, -1, 0}` outside it).
fn save_item_at(x: &Ctx, at: i64) -> Item {
    if at < 0 || at as usize + 4 > piney_data::save::SIZE {
        return Item::NONE;
    }
    let (s, a) = (&x.save.save, at as usize);
    Item { id: s.i16(a), cat: s.u8(a + 2) as i8, num: s.u8(a + 3) as i8 }
}

/// Item `slot` of a trading PC's trade `k` (`tpcTradeList[row][k]`: what
/// it gives, then the three it wants); none past the table.
fn tpc_item(x: &Ctx, row: usize, k: i32, slot: i32) -> Item {
    let tpc = piney_data::tables::newgame::of(x.texts.volume).tpc_trade();
    let it =
        tpc.get(row).and_then(|t| t.get(usize::try_from(k).ok()?)).and_then(|t| t.get(usize::try_from(slot).ok()?));
    it.map_or(Item::NONE, |i| Item { id: i.id, cat: i.category, num: i.num })
}

/// The trader's offer `k` (a trading PC's: what trade `k` gives).
fn offer(x: &Ctx, t: &Trader, k: i32) -> Item {
    match t.offers {
        Offers::Save(at) => save_item_at(x, at + 4 * i64::from(k)),
        Offers::Tpc(row) => tpc_item(x, row, k, 0),
        Offers::None => Item::NONE,
    }
}

/// What a trading PC's trade `k` wants, `w` 0-2.
fn wanted(x: &Ctx, t: &Trader, k: i32, w: i32) -> Item {
    match t.offers {
        Offers::Tpc(row) => tpc_item(x, row, k, 1 + w),
        _ => Item::NONE,
    }
}

/// A slot the lists take: a category, not a key item, and a row.
fn listed(it: Item) -> bool {
    it.cat >= 0 && it.cat != 15 && it.id >= 0
}

/// `tpcTradeListSW[id - 66][k]`.
fn trade_open(x: &Ctx, id: i32, k: i32) -> bool {
    let at = TPC_TRADE_LIST_SW as i64 + i64::from((id - 66) * 3 + k);
    (0..piney_data::save::SIZE as i64).contains(&at) && x.save.save.u8(at as usize) != 0
}

/// The offers' list: their indices, -1 after, and how many.
fn offer_slots(x: &Ctx, t: &Trader) -> ([i16; 16], i32) {
    let mut out = [-1i16; 16];
    let mut n = 0usize;
    for k in 0..16 {
        let take = if t.trading() {
            k < 3 && listed(offer(x, t, k)) && trade_open(x, t.id, k)
        } else {
            listed(offer(x, t, k))
        };
        if take {
            out[n] = k as i16;
            n += 1;
        }
    }
    (out, n as i32)
}

/// The bag's list: its slots (key items left out), -1 after, and how many.
fn bag_slots(x: &Ctx) -> ([i16; 40], i32) {
    let mut out = [-1i16; 40];
    let mut n = 0usize;
    for k in 0..items::ITEMS {
        if listed(items::save_item(x.save, 0, k)) {
            out[n] = k as i16;
            n += 1;
        }
    }
    (out, n as i32)
}

/// A list's entry `i` (-1 past the list, as its stack array is set).
fn slot_at<const N: usize>(l: &[i16; N], i: i32) -> i32 {
    usize::try_from(i).ok().and_then(|i| l.get(i)).map_or(-1, |&v| i32::from(v))
}

/// Kite's bag slot `k` (`itemList[0][k]`; -1 reads the word before).
fn bag_item(x: &Ctx, k: i32) -> Item {
    save_item_at(x, items::ITEM_LIST as i64 + 4 * i64::from(k))
}

/// `(category << 16) | id` with the row sign-extended, as the triangle
/// button's `itemNum` is made.
fn item_num(it: Item) -> i32 {
    (i32::from(it.cat) << 16) | i32::from(it.id)
}

/// `ccGetItemPrice(cat, id)` (main 0x00178eb0): equipment +0x40 (an
/// empty slot's the row before its table), items +0x0c; 0 past the
/// categories or a table's rows.
fn item_price(x: &Ctx, cat: i32, id: i32) -> i32 {
    match cat {
        0..=9 => x.texts.pers.equip_raw.price(cat, id),
        10..=15 => crate::menus::shop::price(x, cat, id & 0xffff),
        _ => 0,
    }
}

/// An equipment row's class (+0x38): which characters may wear it.
fn equip_class(x: &Ctx, cat: i32, id: i32) -> i32 {
    i32::from(x.texts.pers.equip_raw.level(cat, id))
}

/// `ccGetItemTradeRate(tbl, cat, id)` (main 0x00178fb0): the row's rate
/// for the item (0 past the table), 10 when it has none.
fn trade_rate(x: &Ctx, row: Option<&[i16; 14]>, code: i32) -> i32 {
    let (cat, id) = (code >> 16, code & 0xffff);
    let at = |o: i32| row.and_then(|r| r.get(o as usize / 2)).map_or(0, |&v| i32::from(v));
    match cat {
        10..=14 => at(2 * (cat - 10)),
        6..=9 => match equip_class(x, cat, id) {
            1 => at(10),
            2 => at(12),
            3 => at(14),
            _ => 10,
        },
        0..=5 => at(16 + 2 * cat),
        _ => 10,
    }
}

/// `saveData.spcParam[id]` at `field`, `None` off the table.
fn spc_at(id: i32, field: usize) -> Option<usize> {
    usize::try_from(id).ok().filter(|&i| i < 18).map(|i| SPC_PARAM + SPC_PARAM_SIZE * i + field)
}

/// The worth of the trade: what the item is worth to the trader and what
/// each offered stack is, each times its rate (0x0054fdd4 - 0x005501ac).
fn values(x: &Ctx, t: &Trader, d: &[i32; 17], n: &[i32; 8]) -> (i32, [i32; 3]) {
    let price = |c: i32| item_price(x, c >> 16, c & 0xffff);
    let mut want = n[0].wrapping_mul(price(d[0]));
    let mut o = [0i32; 3];
    for k in 1..4 {
        if d[k] != d[0] {
            o[k - 1] = n[k].wrapping_mul(price(d[k]));
        }
    }
    if t.types & 4 != 0 && t.id == 1 {
        for k in 1..4 {
            if d[k] >> 16 == 14 && d[k] & 0xffff == 10 {
                o[k - 1] = n[k].wrapping_mul(50_000);
            }
        }
    }
    let mut r = [10i32; 4];
    if t.types & 4 != 0 {
        let rates = piney_data::tables::fieldui::of(x.texts.volume).spc_trade_rate();
        let row = usize::try_from(t.id - 1).ok().and_then(|i| rates.get(i));
        for k in 0..4 {
            r[k] = trade_rate(x, row, d[k]);
        }
        let f = spc_at(t.id, SPC_FRIENDSHIP).map_or(0, |a| i32::from(x.save.save.i16(a))) / 200;
        let b = if f < 6 { f } else { 5 };
        for v in r.iter_mut().skip(1) {
            *v += b;
        }
    } else if t.types & 0x0200_0018 != 0 {
        let rates = piney_data::tables::fieldui::of(x.texts.volume).npc_trade_rate();
        let row = usize::try_from(npc_row(t.id)).ok().and_then(|i| rates.get(i));
        for k in 0..4 {
            r[k] = trade_rate(x, row, d[k]);
        }
    }
    want = want.wrapping_mul(r[0]);
    for k in 0..3 {
        o[k] = o[k].wrapping_mul(r[k + 1]);
    }
    (want, o)
}

/// Whether a trading PC's trade `k` has all it wants offered, and how
/// many of the things it wants are (0x0054fc60 - 0x0054fd58).
fn wants_met(x: &Ctx, t: &Trader, k: i32, d: &[i32; 17], n: &[i32; 8]) -> (i32, i32) {
    let (mut all, mut met) = (0, 0);
    for w in 0..3 {
        let e = wanted(x, t, k, w);
        if e.cat < 0 || e.id < 0 || e.num <= 0 {
            all += 1;
            continue;
        }
        for j in 1..4 {
            if d[j] >> 16 == i32::from(e.cat) && d[j] & 0xffff == i32::from(e.id) && n[j] >= i32::from(e.num) {
                all += 1;
                met += 1;
                break;
            }
        }
    }
    (all, met)
}

/// The key the lists read from the push: cancel (sound 19), else OK (18).
fn pushed_key(x: &mut Ctx) -> u32 {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        x.save.cancel()
    } else if x.pushed_ok() {
        x.se(SE_OK);
        x.save.ok()
    } else {
        0
    }
}

fn cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// `OpenInfo(l0, l1, l2, 0, -1, -1)`.
fn open_info(m: &mut MenuCtrl, x: &Ctx, lines: [Option<&[u8]>; 3]) {
    let names = x.save.names();
    m.msg.open_info([lines[0], lines[1], lines[2], None], &names);
}

/// `ccMsg->Check(0)`.
fn check(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    push_msg_requests(x, req);
    r
}

/// `tradeMenuHelp[k]` (empty lines past the table).
fn help(x: &Ctx, k: i32) -> [Vec<u8>; 3] {
    usize::try_from(k).ok().and_then(|k| x.texts.talk.trade.help.get(k)).cloned().unwrap_or_default()
}

/// `DispMsg({0x100, name, comment}, 0)` for the item under the cursor, all
/// lines null when its category or row is negative.
fn item_help(m: &mut MenuCtrl, x: &Ctx, it: Item) {
    let names = x.save.names();
    if it.cat < 0 || it.id < 0 {
        m.msg.disp_msg(0x100, None, [None, None, None], &names);
        return;
    }
    let p = x.texts.items.item(i32::from(it.cat), i32::from(it.id)).cloned().unwrap_or_default();
    m.msg.disp_msg(0x100, Some(&p.name), [Some(&p.comment[0]), Some(&p.comment[1]), Some(&p.comment[2])], &names);
}

/// A refusal's window: the dim in, the other tasks asleep, the help's
/// lines (0x0054f020 and its copies).
fn refusal(m: &mut MenuCtrl, x: &mut Ctx, lines: [Option<&[u8]>; 3]) {
    m.bg_status = 1;
    if m.still == 0 {
        talk::sleep_all(m, x);
    }
    open_info(m, x, lines);
    m.proccess += 1;
}

/// The refusal's window closed: out (proccess 100).
fn refusal_check(m: &mut MenuCtrl, x: &mut Ctx) {
    if check(m, x) != 0 {
        m.msg.close();
        m.proccess = 100;
    }
}

/// `proccess 1`: until the help is dismissed (the index 0), else six
/// frames on.
fn first_help(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    if m.lists[i].index == 0 {
        if check(m, x) == 0 {
            return;
        }
        m.msg.close();
        m.wait_count = 0;
        m.lists[i].index = 1;
    } else {
        m.wait_count = 6;
    }
    m.proccess += 1;
}

/// `proccess 100` (0x0054f3c8, 0x00551468): the window and dim out, the
/// target back, and (after a breath when the tasks slept) back to the list.
fn leave(m: &mut MenuCtrl, x: &mut Ctx, sub: bool) -> Flow {
    let i = idx(m);
    m.menu_status = 3;
    m.bg_status = 3;
    m.lists[i].index = 0;
    m.exception_disp = 0;
    let prev = x.target_prev.clone();
    x.change_target(prev.as_ref());
    if m.mode == 1 && m.still == 1 {
        x.req.push(Request::WakeAll);
        crate::disp::disp(m, x);
        return talk::breathed(talk::Tail::Trade(Tail::Leave { sub }));
    }
    back(m, sub);
    Flow::Done
}

/// Back to the list the page came from; 49 first takes 48's prev, so it
/// goes back to the PC's list.
fn back(m: &mut MenuCtrl, sub: bool) {
    let i = idx(m);
    if sub {
        let p = m.lists[i].prev.clamp(0, 88) as usize;
        m.lists[i].prev = m.lists[p].prev;
    }
    let prev = m.lists[i].prev;
    m.back_to_prev(prev);
}

/// `TradeMenu` (gcmn 0x0054e720, menu 48).
pub fn trade_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let (_, nb) = bag_slots(x);
    let t = trader(m, x);
    let (slots, nt) = t.as_ref().map_or(([-1; 16], 0), |t| offer_slots(x, t));
    if t.is_none() {
        m.proccess = 100;
    }
    let t = t.unwrap_or_default();
    match m.proccess {
        0 => {
            m.talk.talk_trade_flag = 1;
            m.lists[i].disp = 4;
            if nb == 0 {
                m.proccess = 20;
                return Flow::Done;
            }
            if nt == 0 {
                m.proccess = 30;
                return Flow::Done;
            }
            if m.lists[i].index == 0 {
                m.talk.temp = [0; 8];
                m.talk.drain_item[..4].fill(-1);
                let h = help(x, 0);
                open_info(m, x, [Some(&h[0]), Some(&h[1]), None]);
            }
            m.bg_status = 1;
            if m.still == 0 {
                talk::sleep_all(m, x);
            }
            m.lists[i].my = 0;
            m.lists[i].y = 0;
            m.exception_disp = 0;
            m.proccess += 1;
        }
        1 => first_help(m, x),
        2 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.lists[i].my = nt as i16;
                m.lists[i].y = nt.min(8) as i16;
                m.menu_status = 1;
                m.exception_disp = 1;
                m.cursor.init(false);
                m.cursor_pr.init(false);
                m.proccess += 1;
            }
        }
        3 => return offer_keys(m, x, &t, &slots),
        20 | 30 => {
            let h = help(x, 3);
            refusal(m, x, [Some(&h[0]), None, None]);
        }
        21 | 31 | 41 => refusal_check(m, x),
        40 => {
            let h = help(x, 4);
            refusal(m, x, [Some(&h[0]), Some(&h[1]), None]);
        }
        100 => return leave(m, x, false),
        _ => {}
    }
    Flow::Done
}

/// TradeMenu's `proccess 3` (0x0054ecc4).
fn offer_keys(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader, slots: &[i16; 16]) -> Flow {
    let i = idx(m);
    m.lists[i].select = m.talk.temp[4] as i16;
    m.lists[i].dy = m.talk.temp[5] as i16;
    m.select_scr(0, 0, 0, x);
    m.talk.temp[4] = i32::from(m.lists[i].select);
    m.talk.temp[5] = i32::from(m.lists[i].dy);
    let it = offer(x, t, slot_at(slots, m.talk.temp[4]));
    if x.pad.push.bits() & 0x10 != 0 && (0..10).contains(&it.cat) {
        x.se(SE_OK);
        m.item_num = item_num(it);
        m.change_menu_to(64);
        return Flow::Done;
    }
    let key = pushed_key(x);
    if key == x.save.cancel() {
        cursors(m);
        m.proccess = 100;
    } else if key == x.save.ok() {
        cursors(m);
        m.talk.drain_item[0] = it.code();
        m.talk.temp[0] = i32::from(it.num);
        m.menu_status = 3;
        m.lists[i].index = 0;
        m.change_menu_to(49);
        return Flow::Done;
    }
    item_help(m, x, it);
    Flow::Done
}

/// `TradeSubMenu` (gcmn 0x0054f4b0, menu 49).
pub fn trade_sub_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let (bag, nb) = bag_slots(x);
    let t = trader(m, x);
    let (slots, _) = t.as_ref().map_or(([-1; 16], 0), |t| offer_slots(x, t));
    if t.is_none() {
        m.proccess = 100;
    }
    let t = t.unwrap_or_default();
    match m.proccess {
        0 => {
            m.lists[i].disp = 4;
            if m.lists[i].index == 0 {
                let h = help(x, 1);
                open_info(m, x, [Some(&h[0]), Some(&h[1]), Some(&h[2])]);
            }
            m.lists[i].my = 0;
            m.lists[i].y = 0;
            m.exception_disp = 0;
            m.proccess += 1;
        }
        1 => first_help(m, x),
        2 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 7 {
                m.lists[i].my = nb as i16;
                m.lists[i].y = nb.min(8) as i16;
                m.menu_status = 1;
                m.exception_disp = 2;
                cursors(m);
                m.proccess += 1;
            }
        }
        3 => return give_keys(m, x, &t, &bag, &slots),
        4 if m.menu_status == 0 => {
            m.exception_disp = 3;
            m.menu_status = 1;
            let l = &mut m.lists[i];
            l.disp = 11;
            l.x = 6;
            l.y = 2;
            l.select = 0;
            m.wait_count = 0;
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            cursors(m);
            let h = help(x, 2);
            open_info(m, x, [Some(&h[0]), None, None]);
            m.proccess += 1;
        }
        5 => {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.menu_status = 3;
                m.msg.close();
                cursors(m);
                m.proccess = 100;
            } else if key == x.save.ok() {
                m.msg.close();
                cursors(m);
                m.menu_status = 3;
                if m.lists[i].select == 1 {
                    m.proccess = 100;
                } else {
                    m.proccess += 1;
                }
            }
        }
        6 if m.menu_status == 0 => trade(m, x, &t, &slots),
        7 => {
            if check(m, x) == 0 {
                return Flow::Done;
            }
            m.msg.close();
            m.lists[i].index = 0;
            if t.types & 4 != 0 {
                m.bg_status = 1;
                m.wait_count = 0;
                m.proccess += 1;
            } else {
                return talk::close(m, x, Then::Nothing);
            }
        }
        8 | 12 | 17 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 9 {
                m.proccess = if m.proccess == 17 { 15 } else { 10 };
            }
        }
        10 => books(m, x, &t),
        11 | 16 => {
            if check(m, x) != 0 {
                m.msg.close();
                m.wait_count = 0;
                m.proccess += 1;
            }
        }
        15 => return equipment(m, x, &t),
        19 => return rest(m, x, &t, 1),
        20 | 30 => {
            let h = help(x, 3);
            refusal(m, x, [Some(&h[0]), None, None]);
        }
        21 | 31 | 41 => refusal_check(m, x),
        40 => {
            let h = help(x, i32::from(m.wait_count));
            if h[2].first().copied().unwrap_or(0) != 0 {
                refusal(m, x, [Some(&h[0]), Some(&h[1]), Some(&h[2])]);
            } else {
                refusal(m, x, [Some(&h[0]), Some(&h[1]), None]);
            }
            m.wait_count = 0;
        }
        100 => return leave(m, x, true),
        _ => {}
    }
    Flow::Done
}

/// TradeSubMenu's `proccess 3` (0x0054f9f8).
fn give_keys(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader, bag: &[i16; 40], slots: &[i16; 16]) -> Flow {
    let i = idx(m);
    m.lists[i].select = m.talk.temp[6] as i16;
    m.lists[i].dy = m.talk.temp[7] as i16;
    m.select_scr(0, 0, 0, x);
    m.talk.temp[6] = i32::from(m.lists[i].select);
    m.talk.temp[7] = i32::from(m.lists[i].dy);
    let it = bag_item(x, slot_at(bag, m.talk.temp[6]));
    let push = x.pad.push.bits();
    if push & 0x10 != 0 && (0..10).contains(&it.cat) {
        x.se(SE_OK);
        m.item_num = item_num(it);
        m.change_menu_to(64);
        return Flow::Done;
    }
    // The keys from the pad's repeat, OK over cancel.
    let (rep, cancel, ok) = (x.pad.repeat.bits(), x.save.cancel(), x.save.ok());
    let mut key = 0;
    if rep & cancel != 0 {
        key = cancel;
    }
    if rep & ok != 0 {
        key = ok;
    }
    if key == cancel {
        let code = it.code();
        let (d, n) = (&mut m.talk.drain_item, &mut m.talk.temp);
        let mut took = false;
        if let Some(k) = (1..4).find(|&k| d[k] == code) {
            took = true;
            n[k] -= 1;
            if n[k] <= 0 {
                d[k] = -1;
                n[k] = 0;
            }
        } else if push & cancel != 0 {
            took = true;
            m.proccess = 100;
        }
        if took {
            cursors(m);
            x.se(SE_BACK);
        }
    } else if key == ok {
        let (d, n) = (m.talk.drain_item, m.talk.temp);
        let balanced = if t.trading() {
            wants_met(x, t, slot_at(slots, n[4]), &d, &n).0 >= 3
        } else {
            let (want, o) = values(x, t, &d, &n);
            o[2].wrapping_add(o[0].wrapping_add(o[1])) >= want
        };
        if balanced {
            if push & ok != 0 {
                cursors(m);
                x.se(SE_OK);
                m.menu_status = 3;
                m.proccess += 1;
                return Flow::Done;
            }
        } else {
            offer_more(m, x, it);
        }
    }
    item_help(m, x, it);
    Flow::Done
}

/// One more of the bag's item in the offer (0x0055020c - 0x0055036c): its
/// stack while below the count carried, else the first free of three.
fn offer_more(m: &mut MenuCtrl, x: &mut Ctx, it: Item) {
    let code = it.code();
    let (d, n) = (&mut m.talk.drain_item, &mut m.talk.temp);
    let (d1, d2, d3) = (d[1], d[2], d[3]);
    let mut more = false;
    if let Some(k) = [d1, d2, d3].iter().position(|&c| c == code).map(|k| k + 1) {
        if n[k] < i32::from(it.num) {
            n[k] += 1;
            more = true;
        }
    } else if let Some(k) = [d1, d2, d3].iter().position(|&c| c == -1).map(|k| k + 1) {
        d[k] = code;
        n[k] = 1;
        more = true;
    }
    if more {
        cursors(m);
        x.se(SE_OK);
    }
}

/// TradeSubMenu's `proccess 6` (0x00550674): the trade.
fn trade(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader, slots: &[i16; 16]) {
    let i = idx(m);
    let (d, n) = (m.talk.drain_item, m.talk.temp);
    let held = |x: &Ctx, c: i32| bitem::get_item_num(&x.save.save, 0, c >> 16, c & 0xffff);
    let (h0, h1, h2, h3) = (held(x, d[0]), held(x, d[1]), held(x, d[2]), held(x, d[3]));
    if h0.wrapping_add(n[0]) >= 100 {
        m.lists[i].index = 0;
        m.wait_count = 5;
        m.proccess = 40;
        return;
    }
    // A full bag takes the item only onto its stack, or into a slot an
    // offered stack given whole leaves.
    let kept = |h: i32, k: usize| h == 0 || h - n[k] > 0;
    if bitem::get_item_slot(&x.save.save, 0) < 0 && h0 <= 0 && kept(h1, 1) && kept(h2, 2) && kept(h3, 3) {
        m.lists[i].index = 0;
        m.wait_count = 4;
        m.proccess = 40;
        return;
    }
    for k in 1..4 {
        if d[k] >= 0 {
            bitem::del_item(&mut x.save.save, 0, d[k] >> 16, d[k] & 0xffff, n[k]);
        }
    }
    let order = x.texts.talk.shop.order;
    bitem::add_item(&mut x.save.save, &order, 0, d[0] >> 16, d[0] & 0xffff, n[0]);
    x.se(SE_TRADE);
    add_trade_count(x, t.types, t.id);
    talk::affect(x, t.handle, 0);
    // The trade off the trader's list.
    let k = slot_at(slots, n[4]);
    if t.types & 4 != 0 {
        del_trade_list(x, SPC_TRADE_LIST + 64 * i64::from(t.id - 1) + 4 * i64::from(k));
    } else if t.types & 0x0200_0018 != 0 {
        if !t.trading() {
            del_trade_list(x, NPC_TRADE_LIST + 64 * i64::from(npc_row(t.id)) + 4 * i64::from(k));
        } else {
            let at = TPC_TRADE_LIST_SW as i64 + i64::from((t.id - 66) * 3 + k);
            if (0..piney_data::save::SIZE as i64).contains(&at) {
                x.save.save.set_u8(at as usize, 0);
            }
        }
    }
    // "You now have #G<item>#W!"
    let tt = &x.texts.talk.trade;
    let mut s = tt.get_item.clone();
    s.extend_from_slice(&tt.green);
    s.extend(x.texts.items.item_name(d[0] >> 16, d[0] & 0xffff));
    s.extend_from_slice(&tt.bang);
    open_info(m, x, [Some(&s), None, None]);
    m.proccess += 1;
}

/// `ccSaveData::DelSpcTradeList` / `DelNpcTradeList` (main 0x00177f40,
/// 0x00177f70): the entry emptied (`{-1, -1, 0}`).
fn del_trade_list(x: &mut Ctx, at: i64) {
    if at < 0 || at as usize + 4 > piney_data::save::SIZE {
        return;
    }
    let (s, a) = (&mut x.save.save, at as usize);
    s.set_u8(a + 2, 0xff);
    s.set_i16(a, -1);
    s.set_u8(a + 3, 0);
}

/// "<name><piece>#G<item>#W." (a member's use of what it got).
fn spc_line(x: &Ctx, t: &Trader, piece: usize, code: i32) -> Vec<u8> {
    let tt = &x.texts.talk.trade;
    let mut s = t.name.clone();
    s.extend(tt.present.get(piece).cloned().unwrap_or_default());
    s.extend_from_slice(&tt.green);
    s.extend(x.texts.items.item_name(code >> 16, code & 0xffff));
    s.extend_from_slice(&tt.dot);
    s
}

/// `proccess 10` (0x00550b2c): the first book offered, read by the member.
fn books(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader) {
    for k in 1..4 {
        let c = m.talk.drain_item[k];
        if c < 0 || c >> 16 != 12 {
            continue;
        }
        // Books go no further than their use (no breath).
        let _ = add_spc_item(m, x, t.handle, t.id, 12, c & 0xffff, m.talk.temp[k], true);
        let s = spc_line(x, t, 5, c);
        open_info(m, x, [Some(&s), None, None]);
        x.se(SE_BOOK);
        m.talk.drain_item[k] = -1;
        m.proccess += 1;
        break;
    }
    if m.proccess == 10 {
        m.proccess = 15;
    }
}

/// `proccess 15` (0x00550cf4): the first piece of equipment with a price
/// offered, equipped when the member takes it.
fn equipment(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader) -> Flow {
    let d = m.talk.drain_item;
    let pick = (1..4).find(|&k| {
        let (cat, id) = (d[k] >> 16, d[k] & 0xffff);
        d[k] >= 0 && (0..10).contains(&cat) && item_price(x, cat, id) > 0
    });
    let Some(k) = pick else {
        m.proccess = 19;
        return Flow::Done;
    };
    match add_spc_item(m, x, t.handle, t.id, d[k] >> 16, d[k] & 0xffff, m.talk.temp[k], true) {
        Added::Done(r) => {
            equipped(m, x, t, k, r);
            Flow::Done
        }
        Added::Breathed(gift) => talk::breathed(talk::Tail::Trade(Tail::Spc { gift, then: SpcThen::Equip(k) })),
    }
}

/// After `proccess 15`'s AddSpcItem: "<name> equipped ..." when it did.
fn equipped(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader, k: usize, r: i32) {
    let c = m.talk.drain_item[k];
    if r == 1 {
        let s = spc_line(x, t, 4, c);
        open_info(m, x, [Some(&s), None, None]);
        m.talk.drain_item[k] = -1;
        m.proccess += 1;
    } else {
        m.talk.drain_item[k] = -1;
    }
}

/// `proccess 19` (0x00550f1c) from offered item `from`: the rest into the
/// member's bag, the target free, the member released, the menu shut.
fn rest(m: &mut MenuCtrl, x: &mut Ctx, t: &Trader, from: usize) -> Flow {
    for k in from..4 {
        let c = m.talk.drain_item[k];
        if c < 0 {
            continue;
        }
        if let Added::Breathed(gift) = add_spc_item(m, x, t.handle, t.id, c >> 16, c & 0xffff, m.talk.temp[k], true) {
            return talk::breathed(talk::Tail::Trade(Tail::Spc { gift, then: SpcThen::Rest(k) }));
        }
    }
    x.req.push(Request::TargetFix(false));
    if let Some(p) = x.target_prev.as_ref() {
        let h = p.handle;
        talk::affect(x, h, 0);
    }
    talk::close(m, x, Then::Nothing)
}

/// The gauge's glyphs of `tradeMenuStr`'s first row: `n` of them in
/// colour `c` from `dx` (SetClm(n)).
fn gauge(s: &mut Spr, n: i32, c: usize, dx: f32, a: i32) {
    set_clm(s, n, 0, 0, 1);
    s.set_colour(c);
    s.set_alpha(a);
    s.dx = dx;
    s.dy = 92.0;
    s.make_packet(0);
}

/// `TradeMenuDisp` (gcmn 0x00551570): the page `ExceptionDisp` draws for
/// menus 48 and 49.
///
/// ```text
/// settingKanji[0]  tradeMenuStr's two rows, the trader's name, Kite's,
///                  the item wanted, the three offered (16 glyphs each)
/// (40, 24)         the trader's name frame (DispTarget); (24, 56) the
///                  item wanted and its count
/// (24, 128)        the offers, 8 rows (grey unless on 48): icon, name,
///                  count (less what is being traded)
/// (322, 24)        Kite's name; (306, 56) the three offered and counts,
///                  the one under 49's cursor in colour 22
/// (278, 128)       the bag (grey unless on 49), counts less the offer
/// (214, 56)        "Approve" lit when the trade balances, over a gauge
///                  of eight: a trading PC's wants met (2 each), else the
///                  offer's worth * 4 / the item's
/// ```
///
/// The font's shadow bit (ctrl 0x10) is set and left set, as BuyMenuDisp
/// does.
pub fn trade_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let texts = x.texts;
    let items = &texts.items;
    let a = m.alpha;
    font_type(&mut m.font, 1);
    m.font.shadow = true;
    m.font.set_colour(7);
    m.font.set_alpha(a);
    let (bag, nb) = bag_slots(x);
    let Some(t) = trader(m, x) else { return };
    let (slots, nt) = offer_slots(x, &t);
    let (d, n) = (m.talk.drain_item, m.talk.temp);
    let names = x.save.names();
    let kite = crate::disp::member_name(x, 0);
    let name_of = |c: i32| items.item_name(c >> 16, c & 0xffff);
    // settingKanji[0]'s rows.
    let mut buf = texts.talk.trade.menu_str.clone();
    str_cat(&mut buf, &t.name, 16);
    str_cat(&mut buf, &kite, 16);
    for &c in &d[..4] {
        if c < 0 {
            str_cat(&mut buf, b"", 16);
        } else {
            str_cat(&mut buf, &name_of(c), 16);
        }
    }
    m.setting_text[0] = buf;
    set_clm(&mut m.setting[0], 16, 0, 0, 1);
    m.setting[0].set_colour(7);
    m.setting[0].set_alpha(a);
    // The trader's side.
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = 40.0;
    m.win.dy = 24.0;
    disp_target(&mut m.win, piney_desktop::message::str_len(&t.name, &names), 0);
    m.setting[0].dx = 65.0;
    m.setting[0].dy = 30.0;
    m.setting[0].make_packet(2);
    set_type(&mut m.win, 1);
    m.win.dx = 24.0;
    m.win.dy = 56.0;
    disp_square(&mut m.win, 11, 2, None);
    if d[0] >= 0 {
        m.setting[0].dx = 38.0;
        m.setting[0].dy = 64.0;
        m.setting[0].make_packet(4);
        m.font.dx = 164.0;
        m.font.dy = 64.0;
        make_num(&mut m.font, 2, n[0]);
    }
    if m.exception_disp < 3 {
        m.win.set_colour(if m.exception_disp == 1 { 7 } else { 0 });
        m.win.set_alpha(a);
        m.win.dx = 24.0;
        m.win.dy = 128.0;
        disp_square_sb(&mut m.win, 12, 8, n[5], nt, None);
    }
    // Kite's side.
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = 322.0;
    m.win.dy = 24.0;
    disp_target(&mut m.win, piney_desktop::message::str_len(&kite, &names), 0);
    m.setting[0].dx = 347.0;
    m.setting[0].dy = 30.0;
    m.setting[0].make_packet(3);
    set_type(&mut m.win, 1);
    m.win.dx = 306.0;
    m.win.dy = 56.0;
    disp_square(&mut m.win, 11, 2, None);
    let cur = bag_item(x, slot_at(&bag, n[6]));
    for k in 1..4 {
        let c = d[k];
        if c < 0 {
            continue;
        }
        let on = i32::from(cur.cat) == c >> 16 && i32::from(cur.id) == c & 0xffff && m.exception_disp < 3;
        let colour = if on { 22 } else { 7 };
        m.setting[0].set_colour(colour);
        m.setting[0].set_alpha(a);
        m.font.set_colour(colour);
        m.font.set_alpha(a);
        let y = from_int(64 + 18 * (k as i32 - 1));
        m.setting[0].dx = 320.0;
        m.setting[0].dy = y;
        m.setting[0].make_packet(4 + k as i32);
        m.font.dx = 446.0;
        m.font.dy = y;
        make_num(&mut m.font, 2, n[k]);
    }
    m.setting[0].set_colour(7);
    m.setting[0].set_alpha(a);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    if m.exception_disp < 3 {
        m.win.set_colour(if m.exception_disp == 2 { 7 } else { 0 });
        m.win.set_alpha(a);
        m.win.dx = 278.0;
        m.win.dy = 128.0;
        disp_square_sb(&mut m.win, 12, 8, n[7], nb, None);
    }
    // The balance: "Approve" and its gauge.
    let bal = if t.trading() {
        let (all, met) = wants_met(x, &t, slot_at(&slots, n[4]), &d, &n);
        if all >= 3 { 4 } else { met }
    } else {
        let (want, o) = values(x, &t, &d, &n);
        if want == 0 {
            0
        } else {
            let v = o[2].wrapping_add(o[0].wrapping_add(o[1])).wrapping_mul(4).wrapping_div(want);
            if v >= 5 { 4 } else { v }
        }
    };
    let c = if m.exception_disp == 3 || bal == 4 { 7 } else { 0 };
    m.win.set_colour(c);
    m.setting[0].set_colour(c);
    m.win.set_alpha(a);
    m.setting[0].set_alpha(a);
    m.win.dx = 214.0;
    m.win.dy = 56.0;
    disp_square(&mut m.win, 4, 2, None);
    m.setting[0].dx = 228.0;
    m.setting[0].dy = 72.0;
    m.setting[0].make_packet(1);
    let s0 = &mut m.setting[0];
    match bal {
        4 => gauge(s0, 8, 6, 225.0, a),
        1..=3 => {
            let lit = 2 * bal;
            gauge(s0, lit, 6, 225.0, a);
            gauge(s0, 8 - lit, 0, from_int(225 + 8 * lit), a);
            set_clm(s0, 16, 0, 0, 1);
        }
        0 => {
            s0.set_colour(0);
            s0.set_alpha(a);
            s0.dx = 225.0;
            s0.dy = 92.0;
            s0.make_packet(0);
        }
        _ => {}
    }
    if m.exception_disp >= 3 {
        return;
    }
    // The two lists.
    font_type(&mut m.font, 1);
    m.font.shadow = true;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    set_clm(&mut m.setting[1], 16, 0, 0, 1);
    m.item_icon.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    let fr = x.frame_rate;
    let mut buf = Vec::new();
    let mut row = 0i32;
    for s2 in 0..nt {
        if s2 < n[5] {
            continue;
        }
        let it = offer(x, &t, slot_at(&slots, s2));
        let y = add(144.0, from_int(row * 20));
        if it.cat >= 0 && it.id >= 0 {
            let (cat, id) = (i32::from(it.cat), i32::from(it.id));
            str_cat(&mut buf, &items.item_name(cat, id), 16);
            let c = if m.exception_disp != 1 { 0 } else { 7 };
            m.setting[1].set_colour(c);
            m.item_icon.set_colour(c);
            m.font.set_colour(c);
            m.setting[1].set_alpha(a);
            m.setting[1].dx = 52.0;
            m.setting[1].dy = y;
            m.setting[1].make_packet(row);
            m.item_icon.set_alpha(a);
            m.item_icon.dx = 36.0;
            m.item_icon.dy = sub(y, 2.0);
            m.item_icon.make_packet(items.item_icon(cat));
            m.font.set_alpha(a);
            m.font.dx = 174.0;
            m.font.dy = from_int(144 + row * 20);
            let num = if it.code() == d[0] { i32::from(it.num) - n[0] } else { i32::from(it.num) };
            make_num(&mut m.font, 2, num);
        } else {
            str_cat(&mut buf, b"", 16);
        }
        if row == n[4] - n[5] && m.exception_disp == 1 {
            m.cursor.disp(&mut m.win, 24.0, y, 12, row, a, 3, fr);
        }
        row += 1;
        if row >= 8 {
            break;
        }
    }
    m.setting_text[1] = buf;
    set_clm(&mut m.setting[2], 16, 0, 0, 1);
    let mut buf = Vec::new();
    let mut row = 0i32;
    for s2 in 0..nb {
        if s2 < n[7] {
            continue;
        }
        let it = bag_item(x, slot_at(&bag, s2));
        let y = add(144.0, from_int(row * 20));
        if it.cat >= 0 && it.id >= 0 {
            let (cat, id) = (i32::from(it.cat), i32::from(it.id));
            str_cat(&mut buf, &items.item_name(cat, id), 16);
            let c = if m.exception_disp != 2 { 0 } else { 7 };
            m.setting[2].set_colour(c);
            m.item_icon.set_colour(c);
            m.font.set_colour(c);
            m.setting[2].set_alpha(a);
            m.setting[2].dx = 306.0;
            m.setting[2].dy = y;
            m.setting[2].make_packet(row);
            m.item_icon.set_alpha(a);
            m.item_icon.dx = 290.0;
            m.item_icon.dy = sub(y, 2.0);
            m.item_icon.make_packet(items.item_icon(cat));
            m.font.set_alpha(a);
            m.font.dx = 428.0;
            m.font.dy = from_int(144 + row * 20);
            let code = it.code();
            let num = match (1..4).find(|&k| d[k] == code) {
                Some(k) => i32::from(it.num) - n[k],
                None => i32::from(it.num),
            };
            make_num(&mut m.font, 2, num);
        } else {
            str_cat(&mut buf, b"", 16);
        }
        if row == n[6] - n[7] && m.exception_disp == 2 {
            m.cursor.disp(&mut m.win, 278.0, y, 12, row, a, 3, fr);
        }
        row += 1;
        if row >= 8 {
            break;
        }
    }
    m.setting_text[2] = buf;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_rows() {
        assert_eq!(npc_row(30), 0);
        assert_eq!(npc_row(65), 35);
        assert_eq!(npc_row(145), 36);
        assert_eq!(npc_row(153), 44);
        assert_eq!(npc_row(154), 124);
    }

    #[test]
    fn slots_past_the_list() {
        let l = [3i16, 5, -1, -1];
        assert_eq!(slot_at(&l, 1), 5);
        assert_eq!(slot_at(&l, 2), -1);
        assert_eq!(slot_at(&l, -1), -1);
        assert_eq!(slot_at(&l, 9), -1);
    }
}
