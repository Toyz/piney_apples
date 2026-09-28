//! What PERSONAL's and OPTION's pages share: the pad's OK / cancel as the
//! handlers read it, the item and dialog texts, the menu fader
//! (`ccScFade`, `menuFade`), and the frames a handler resumes after it
//! breathed itself ([`Tail`]).
//!
//! A handler that calls `Disp` and `ccBreathThread(1)` in the middle of
//! its work (Gate Out's and Log Out's fades, a Ryu Book, a closing menu)
//! returns [`Flow::Breathed`] with an [`After::Pers`] tail; the task runs
//! [`tail`] at the start of the next frame, before anything else, and the
//! task loop's own `Disp` only when the handler finally returns.

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_OK};
use crate::menus::system::push_msg_requests;

/// The frames after a handler's own breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// Key Items, a Ryu Book (0x005300a0 on): after `WakeAll` the tasks
    /// run a frame; then the flips back on for one more.
    BookWoken(i32),
    /// Asleep and frozen again (as `OpenMenu` does it, the noise, still
    /// and game layers kept live), the fade back in.
    BookSleep(i32),
    /// Until `CheckFade` of the book's fade ends.
    BookFade(i32),
    /// A close inlined in a handler (`still = 0`, the flips back on when it
    /// woke the tasks, `firstTime = 0`) with nothing after.
    Closed,
    /// Gate Out and Log Out: until the fade to black ends, then what
    /// follows (see [`crate::menus::leave`]).
    Leave(crate::menus::leave::LeaveTail),
}

/// Runs a tail; `Some` when it breathed again (it drew the frame itself),
/// `None` when the handler returned (the task's `Disp` ends the frame).
pub fn tail(m: &mut MenuCtrl, c: Cont, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::BookWoken(id) => {
            m.still = 0;
            x.req.push(Request::Still(false));
            crate::disp::disp(m, x);
            Some(pers(Tail::BookSleep(id)))
        }
        Tail::BookSleep(id) => {
            x.req.push(Request::SleepAll);
            m.still = 1;
            x.req.push(Request::Still(true));
            x.req.push(Request::KeepLayers);
            continue_fade(m, id, 10, 0);
            crate::menus::keyitem::book_fade(m, id, x)
        }
        Tail::BookFade(id) => crate::menus::keyitem::book_fade(m, id, x),
        Tail::Closed => {
            if c.woke {
                m.still = 0;
                x.req.push(Request::Still(false));
            }
            m.first_time = 0;
            None
        }
        Tail::Leave(l) => crate::menus::leave::tail(m, c, l, x),
    }
}

/// A continuation that breathed without waking anything.
pub fn pers(t: Tail) -> Cont {
    Cont { woke: false, cursors: false, after: After::Pers(t) }
}

/// The inlined `CloseMenu`: `menuNext -1`, the window and the dim out,
/// the tasks woken, `Disp`, the breath; the rest next frame.
pub fn close(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    m.menu_next = -1;
    m.menu_status = 3;
    m.bg_status = 3;
    let woke = m.breathe_close(x);
    Flow::Breathed(Cont { woke, cursors: false, after: After::Pers(Tail::Closed) })
}

/// The pad's decision as the handlers read it: cancel (sound 19) first,
/// then OK (18): `Some(false)` cancel, `Some(true)` OK.
pub fn key(x: &mut Ctx) -> Option<bool> {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        Some(false)
    } else if x.pushed_ok() {
        x.se(SE_OK);
        Some(true)
    } else {
        None
    }
}

/// `ccMsg->Check(0)` from a handler, its sounds passed on.
pub fn check(m: &mut MenuCtrl, x: &mut Ctx) -> bool {
    let ok = x.save.ok();
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), ok, &mut req);
    push_msg_requests(x, req);
    r != 0
}

/// `ccMsg->OpenInfo(sep(s, 0), sep(s, 1) or null, 0, 0)`: a two-line
/// refusal, the second line left out when it is empty.
pub fn open_info2(m: &mut MenuCtrl, x: &Ctx, lines: &[Vec<u8>; 2]) {
    let names = x.save.names();
    let second = if lines[1].is_empty() { None } else { Some(&lines[1][..]) };
    m.msg.open_info([Some(&lines[0]), second, None, None], &names);
}

/// `menuFade->ContinueFade(n, t, c)` (main 0x00160490): element `n` ramps
/// again from where it is to `c` over `t`.
pub fn continue_fade(m: &mut MenuCtrl, n: i32, t: i16, c: u32) {
    if let Some(e) = usize::try_from(n).ok().and_then(|n| m.menu_fade.elm.get_mut(n)) {
        e.status = piney_demo::fade::DRAW;
        e.tcnt = t;
        e.cnt = 0;
        e.col0 = e.col1;
        e.col1 = c;
    }
}

/// `menuFade->DeleteFade(n)` (main 0x00160510).
pub fn delete_fade(m: &mut MenuCtrl, n: i32) {
    if let Some(e) = usize::try_from(n).ok().and_then(|n| m.menu_fade.elm.get_mut(n)) {
        e.status = 0;
    }
}

/// `menuFade->CheckFade(n)` (main 0x001604d0).
pub fn check_fade(m: &MenuCtrl, n: i32) -> bool {
    usize::try_from(n).is_ok_and(|n| m.menu_fade.check(n))
}

/// The texts PERSONAL's pages read (pointers in main's .sdata, read from
/// the disc at run time).
#[derive(Clone, Debug, Default)]
pub struct PersTexts {
    /// `impItemMenuTag`: Key Items' four tabs.
    pub imp_tags: Vec<Vec<u8>>,
    /// `puccigusoCallWarnStr`: "There are no Grunties you can call ...".
    pub grunty_warn: [Vec<u8>; 2],
    /// `shopStr`'s first line: the count window's labels ("Discard" is
    /// cell 6).
    pub shop_str: Vec<u8>,
    /// `inBattleMenuWarn`, `gateoutMenuInfo`, `logoutMenuInfo`.
    pub in_battle_warn: Vec<u8>,
    pub gateout_info: Vec<u8>,
    pub logout_info: Vec<u8>,
    /// `statusMenuHelp[75]` (0x0033e9e0 + 0x20): a help of three lines for
    /// each cell of the Status grid, 15 rows a column.
    pub status_help: Vec<[Vec<u8>; 3]>,
    /// `statusBeffStr[5]`: each added effect's help (lines 0-2) and name
    /// (line 3).
    pub beff_help: Vec<[Vec<u8>; 3]>,
    pub beff_name: Vec<Vec<u8>>,
    /// The battle tables (equipment, skills, characters) the Equipment
    /// page's stats come from: `piney_battle::Tables` (set by
    /// [`crate::FieldUi::new`]).
    pub battle: piney_battle::Tables,
    /// Equipment's: the equipment tables as `ccGetEquipParam` reads them,
    /// `equipChangeStr` (each page's labels), `equipMenuWarn` (three lines
    /// each) and `addItemCategoryTbl` (the order `AddItem` sorts in).
    pub equip_raw: crate::menus::equip::EquipRaw,
    pub equip_change: Vec<Vec<u8>>,
    pub equip_warn: Vec<[Vec<u8>; 3]>,
    pub category_order: piney_battle::item::CategoryOrder,
    /// PARTY's (menus 9, 68-70).
    pub party: crate::menus::party_menus::PartyTexts,
}

impl PersTexts {
    /// The volume's (`piney_data::tables::fieldui`), with the battle's
    /// tables the Equipment page reads.
    pub fn of(volume: piney_data::volume::Volume, battle: piney_battle::Tables) -> PersTexts {
        use crate::tables::piece;
        use piney_data::tables::sjis::encode;
        let f = piney_data::tables::fieldui::of(volume);
        let three = |l: &[&str]| [piece(l, 0), piece(l, 1), piece(l, 2)];
        let order = f.category_order();
        PersTexts {
            imp_tags: f.imp_tags().iter().map(|l| encode(l)).collect(),
            grunty_warn: [piece(f.grunty_warn(), 0), piece(f.grunty_warn(), 1)],
            shop_str: encode(f.shop_str()),
            in_battle_warn: encode(f.in_battle_warn()),
            gateout_info: encode(f.gateout_info()),
            logout_info: encode(f.logout_info()),
            status_help: f.status_help().iter().map(|l| three(l)).collect(),
            beff_help: f.status_beff().iter().map(|l| three(l)).collect(),
            beff_name: f.status_beff().iter().map(|l| piece(l, 3)).collect(),
            equip_raw: crate::menus::equip::EquipRaw::of(volume, &battle),
            battle,
            equip_change: f.equip_change().iter().map(|l| encode(l)).collect(),
            equip_warn: f.equip_warn().iter().map(|l| three(l)).collect(),
            category_order: std::array::from_fn(|k| (order[k][0], order[k][1])),
            party: crate::menus::party_menus::PartyTexts::of(volume),
        }
    }
}
