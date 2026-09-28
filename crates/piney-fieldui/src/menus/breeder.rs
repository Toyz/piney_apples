//! The Grunty breeders (`npcTbl` row 10, "Grunt Shop", type 0x8000000):
//! `BreederMenu` (gcmn 0x00543930, menu 27: Talk and the three "About"
//! pages), and Give Food: `BreedingMenu` (0x0054bdd0, 56) with
//! `BreedingMenuDisp` (0x0054cd70), which a Grunty's own menus open
//! (`OtonainuMenu`, `InuMenu`, 45 and 46, [`super::inu`]: they drop the
//! target and `ChangeMenu`), so the Grunty fed is `cmndTargetPrev`, a
//! `ccPGuso` (gcmn pgbreed.cpp). Its state is the world's ([`Grunty`],
//! [`crate::World::grunty`]); the one write the menu makes to it is a
//! request ([`TalkReq::GruntyGrowth`]).
//!
//! ```text
//! BreederMenu
//! proccess 0  cmndTargetFix; no target: CloseMenu. The rows: the list's
//!             first item's name (Talk) and breederMenuStr's three lines
//!             (About Grunties, About Food, About Breeding); the first
//!             time the breeder stops, faces Kite (EntryAffect 14) and
//!             greets (base->msg[game.server]), the cursor on Talk;
//!             talkNum 3 with key item 49, else 1 (Talk's line); the
//!             minimap out; SetMerchantCamera
//! proccess 1  Select; cmndTarget gone: changeCamera(1), the minimap
//!             back, CloseMenu, ccMsg->Close - then the keys, next frame
//!             cancel (19): EntryAffect 0, changeCamera(1), the minimap
//!             back, CloseMenu, ccMsg->Close
//!             OK (18): Talk: ChangeMenu, ccMsg->Close; the others: the
//!             window out, 2
//! proccess 2  EntryAffect 15; breedTeachMsgTbl[row - 1] (town 1), else
//!             breedTeachMsgTbl2's (no target: as cancel); 3
//! proccess 3  Check(1) (emode 1 chains): the list again (window in,
//!             proccess 0, firstTime 0)
//!
//! BreedingMenu (cmndTargetPrev: the Grunty)
//! proccess 0  the foods held (key items 26-41), 8 rows at most; none:
//!             the Grunty the target again (ccChangeCmndTarget(prev)),
//!             back to the Grunty's list
//! proccess 1  SelectScr; cancel: the same; OK: the count
//! proccess 2  itemNum the food; up / down one, left / right ten (1 ..
//!             held); cancel: 1; OK: the window out, 3
//! proccess 3  the window gone: "Give N #Gfood#W." (breedingMenuHelp[1]),
//!             OK / Cancel (disp 11, the row kept in sx)
//! proccess 4  Select; OK on OK: EntryAffect(pg, plw, 19, food - 26, N)
//!             (it eats) and DelItem(0, 15, food, N); 5
//! proccess 5  the window out; 6
//! proccess 6  the window gone: the Grunty idle (growthNum 0): the foods
//!             again (0); else 10
//! proccess 10 by growthNum: 0 the foods again (11); 1 eating: wait; 2
//!             growing up: pgEvoMsg at 0 frames, its second record at 60,
//!             shut at 120; 3 its line (base->msg[msgNum], voice
//!             ccCheckVoiceGrp(id), msgNum): 12; 5 gone off: index 1, 20
//! proccess 12 Check(1): index 1; growthNum 4 (the menu's write); the
//!             Grunty staying (exist): 13; key item 49 held: 20; else the
//!             tasks asleep, the dim: 14
//! proccess 13 until growthNum 5: 20
//! proccess 14 ten frames: "You now have #G<key item 49>#W!" (and
//!             getItemMenuStr[8]), AddItem(0, 15, 49, 1), sound 74
//! proccess 15 Check(0): Close, the tasks woken (a breath), the dim out;
//!             20
//! proccess 20 menuFade->EntryFade(10, 0, 0x80000000): to black in 10
//! proccess 21 CheckFade done: EntryAffect(pg, plw, 11)
//! proccess 22 ContinueFade(10, 0): back
//! proccess 23 CheckFade done: DeleteFade; index 0: the Grunty the target
//!             again, back to the list; else EntryAffect(pg, 0), the
//!             minimap back, cursolOff 0, CloseMenu
//! ```

use piney_battle::item as bitem;
use piney_data::tables::fieldui;
use piney_desktop::eef::from_int;

use crate::Request;
use crate::ctrl::{Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_MOVE, SE_OK};
use crate::menus::personal;
use crate::menus::system::{extract_menu, push_msg_requests, str_cat};
use crate::spr::{Spr, font_type, make_num, set_clm};
use crate::talk::{self, Base, TalkReq, Then};
use crate::window::{disp_square, disp_square_sb};

/// gcmn's literal "STATUS" (`@15433`).
pub const STR_STATUS: &[u8] = b"STATUS";
/// `saveData.impItemList[320]` (+0xcfc): Kite's key items' counts; the
/// foods are 26-41, 49 the breeder's gift.
pub const IMP_ITEM_LIST: usize = 0x0cfc;
pub const FOODS: std::ops::Range<i32> = 26..42;
pub const FOOD_CAT: i32 = 15;
pub const PG_GIFT: i32 = 49;
/// `saveData.growth[5]` (+0x2194, `GROWTH_PARAM`, 0x18 bytes, by server):
/// level, size, smell, crooked, cruel, iq, pure ...
pub const SAVE_GROWTH: usize = 0x2194;

/// The pages' texts, read from the executable once.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    pub breeder_rows: [Vec<u8>; 3],
    pub status_rows: Vec<u8>,
    /// `breedingMenuHelp[0]`'s line: "There is no food." (InuMenu's).
    pub no_food: Vec<u8>,
    pub help: [Vec<u8>; 3],
    pub get_item: [Vec<u8>; 2],
    pub status: Vec<u8>,
}

impl Texts {
    /// The volume's (`piney_data::tables::fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> Texts {
        use crate::tables::piece;
        let f = piney_data::tables::fieldui::of(volume);
        let rows = f.breeder_str();
        let help = f.breeding_help();
        Texts {
            breeder_rows: [piece(rows, 0), piece(rows, 1), piece(rows, 2)],
            status_rows: piney_data::tables::sjis::encode(f.breeding_str()),
            no_food: piece(help[0], 0),
            help: [piece(help[1], 0), piece(help[1], 1), piece(help[1], 2)],
            get_item: [piece(f.get_item_str(), 0), piece(f.get_item_str(), 8)],
            status: STR_STATUS.to_vec(),
        }
    }
}

/// A Grunty (`ccPGuso`, gcmn pgbreed.cpp) as `BreedingMenu` reads it
/// through `cmndTargetPrev`: the world's, each frame. Another character
/// there reads as an idle Grunty (the game reads its bytes at the same
/// offsets).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Grunty {
    /// The character's handle (as `cmndTargetPrev`).
    pub handle: u32,
    /// Its base: `npcTbl[row]` (Little Grunty, Grunty the Kid ...).
    pub row: i32,
    /// +0x2ae `growthNum`: 0 idle, 1 eating, 2 growing up, 3 it speaks, 4
    /// done (the menu's), 5 gone off (runAway).
    pub growth: i8,
    /// +0x2af `msgNum`: which of its lines it says (growth 3).
    pub msg: i8,
    /// +0x305 `exist`.
    pub exist: u8,
    /// Its record's level (+0x2d0), size (+0x2d2) and food line (+0x2e4):
    /// what `dogAction`'s line (15) reads.
    pub level: i16,
    pub size: i16,
    pub food_num: i32,
    /// +0x306 `chatFlag`, +0x307 `foodMode`: InuMenu's writes.
    pub chat_flag: u8,
    pub food_mode: u8,
}

/// What the pages do after a breath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// BreederMenu after its close for a lost target: ccMsg->Close, then
    /// the keys with this frame's pad.
    Keys,
    /// BreederMenu's "About" page after its close for a lost target:
    /// ccMsg->Close, proccess 3.
    TeachClosed,
    /// BreedingMenu after the key item's window: the tasks woken, the
    /// dim out, the fade.
    Woken,
}

/// The pages' tails.
pub fn tail(m: &mut MenuCtrl, t: Tail, x: &mut Ctx) -> Option<Cont> {
    match t {
        Tail::Keys => {
            m.msg.close();
            talk::then(breeder_keys(m, x))
        }
        Tail::TeachClosed => {
            m.msg.close();
            m.proccess += 1;
            None
        }
        Tail::Woken => {
            m.still = 0;
            x.req.push(Request::Still(false));
            m.bg_status = 3;
            m.proccess = 20;
            None
        }
    }
}

fn idx(m: &MenuCtrl) -> usize {
    m.menu.clamp(0, 88) as usize
}

fn cursors(m: &mut MenuCtrl) {
    m.cursor.init(true);
    m.cursor_pr.init(true);
}

/// The key the pages read: cancel (sound 19), else OK (18), else 0.
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

/// `EntryAffect(cmndTarget, plw, n)`.
fn affect_target(x: &mut Ctx, n: i16) {
    if let Some(t) = x.target.as_ref() {
        let target = t.handle;
        talk::affect(x, target, n);
    }
}

/// `saveData.impItemList[k]`.
fn imp(x: &Ctx, k: i32) -> i32 {
    i32::from(x.save.save.u8(IMP_ITEM_LIST + k as usize) as i8)
}

/// `ccMsg->Check(0)`.
fn check(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let mut req = Vec::new();
    let r = m.msg.check(x.pad.push.bits(), x.pad.repeat.bits(), x.save.ok(), &mut req);
    push_msg_requests(x, req);
    r
}

/// `BreederMenu` (gcmn 0x00543930, menu 27).
pub fn breeder_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    match m.proccess {
        0 => {
            x.req.push(Request::TargetFix(true));
            if x.target.is_none() {
                return talk::close(m, x, Then::Nothing);
            }
            let first = m.lists[i].items.first().copied().unwrap_or(-1);
            let name = m.lists.get(first.max(0) as usize).map(|l| l.name.clone()).unwrap_or_default();
            let mut buf = Vec::new();
            str_cat(&mut buf, &name, 16);
            for r in x.texts.talk.breeder.breeder_rows.clone() {
                str_cat(&mut buf, &r, 16);
            }
            extract_menu(m, buf);
            if m.first_time != 0 {
                if let Some(base) = talk::target_base(m, x)
                    && base.msg != 0
                {
                    affect_target(x, 14);
                    let server = x.world.game.server;
                    let rec = x.texts.talk.word(base.msg.wrapping_add(4 * server as u32));
                    talk::open_record(m, x, rec, &base.name, -1, -1);
                }
                m.lists[i].select = 0;
            }
            m.talk.talk_num = if bitem::get_item_num(&x.save.save, 0, FOOD_CAT, PG_GIFT) > 0 { 3 } else { 1 };
            m.map_status = 3;
            talk::req(x, TalkReq::MerchantCamera);
            m.proccess += 1;
            Flow::Done
        }
        1 => {
            m.select(0, 0, false, x);
            if x.target.is_none() {
                talk::req(x, TalkReq::Camera(1));
                m.map_status = 1;
                return talk::close(m, x, Then::Breeder(Tail::Keys));
            }
            breeder_keys(m, x)
        }
        2 => {
            if x.target.is_none() {
                talk::req(x, TalkReq::Camera(1));
                m.map_status = 1;
                return talk::close(m, x, Then::Breeder(Tail::TeachClosed));
            }
            affect_target(x, 15);
            let base = talk::target_base(m, x).unwrap_or_default();
            // `breedTeachMsgTbl[3]` in town 1, `breedTeachMsgTbl2[3]`
            // elsewhere: the "About" pages' records.
            let t = fieldui::of(x.texts.volume);
            let tbl = if x.world.game.town == 1 { t.breed_teach_va() } else { t.breed_teach2_va() };
            let sel = i32::from(m.lists[i].select);
            let rec = x.texts.talk.word(tbl.wrapping_add((4 * (sel - 1)) as u32));
            talk::open_record(m, x, rec, &base.name, -1, -1);
            m.proccess += 1;
            Flow::Done
        }
        3 => {
            if talk::msg_check(m, x) != 0 {
                m.menu_status = 1;
                m.proccess = 0;
                m.first_time = 0;
            }
            Flow::Done
        }
        _ => Flow::Done,
    }
}

/// BreederMenu's keys (0x00543c80 on).
fn breeder_keys(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let key = pushed_key(x);
    if key == x.save.cancel() {
        affect_target(x, 0);
        talk::req(x, TalkReq::Camera(1));
        m.map_status = 1;
        return talk::close(m, x, Then::MsgClose);
    }
    if key == x.save.ok() {
        if m.lists[i].select != 0 {
            m.menu_status = 3;
            m.proccess += 1;
            return Flow::Done;
        }
        let flow = m.change_menu(x);
        m.msg.close();
        return flow;
    }
    Flow::Done
}

/// `cmndTargetPrev` as the Grunty: the world's when it is this character
/// (another reads as an idle Grunty: the game reads that character's
/// bytes at the Grunty's offsets).
fn grunty(x: &Ctx) -> Grunty {
    let h = x.target_prev.as_ref().map_or(0, |t| t.handle);
    match x.world.grunty {
        Some(g) if g.handle == h && h != 0 => g,
        _ => Grunty { handle: h, ..Grunty::default() },
    }
}

/// Back to the list the page came from: the Grunty the target again
/// (`ccChangeCmndTarget(cmndTargetPrev)`), `menuNext = prev`.
fn back(m: &mut MenuCtrl, x: &mut Ctx) {
    let prev = x.target_prev.clone();
    x.change_target(prev.as_ref());
    let i = idx(m);
    m.menu_next = m.lists[i].prev;
    m.menu_status = 3;
    m.proccess = 0;
    m.wait_count = 0;
    cursors(m);
}

/// The count (`waitCount`, 1 .. `most`), as the shops'.
fn count_keys(m: &mut MenuCtrl, x: &mut Ctx, most: i32) {
    let rep = x.pad.repeat.bits();
    let most = most as i16;
    if rep & 0x1000 != 0 {
        m.wait_count += 1;
        if most < m.wait_count {
            m.wait_count = most;
        } else {
            x.se(SE_MOVE);
        }
    } else if rep & 0x4000 != 0 {
        m.wait_count -= 1;
        if m.wait_count > 0 {
            x.se(SE_MOVE);
        } else {
            m.wait_count = 1;
        }
    } else if rep & 0x8000 != 0 {
        if m.wait_count < most {
            m.wait_count += 10;
            x.se(SE_MOVE);
            if most < m.wait_count {
                m.wait_count = most;
            }
        } else {
            m.wait_count = most;
        }
    } else if rep & 0x2000 != 0 {
        if m.wait_count >= 2 {
            m.wait_count -= 10;
            x.se(SE_MOVE);
            if m.wait_count <= 0 {
                m.wait_count = 1;
            }
        } else {
            m.wait_count = 1;
        }
    }
}

/// `OpenInfo(l0, l1, 0, 0, -1, -1)`.
fn open_info(m: &mut MenuCtrl, x: &Ctx, l0: &[u8], l1: Option<&[u8]>) {
    let names = x.save.names();
    m.msg.open_info([Some(l0), l1, None, None], &names);
}

/// `ccMsg->Open(rec, 0, -1, -1)` / `Change(rec, 0, -1, -1)`: a record with
/// no name.
fn record_nameless(m: &mut MenuCtrl, x: &Ctx, rec: u32, change: bool) {
    let r = x.texts.talk.ev_msg(rec);
    let names = x.save.names();
    let lines: Vec<&[u8]> = r.lines.iter().map(|l| &l[..]).collect();
    if change {
        m.msg.change_record(r.emode, None, &lines, &names);
    } else {
        m.msg.open_record(r.emode, None, &lines, &names);
    }
    m.talk.chain = None;
}

/// `BreedingMenu` (gcmn 0x0054bdd0, menu 56).
pub fn breeding_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let pg = grunty(x);
    match m.proccess {
        0 => {
            let held = FOODS.filter(|&k| imp(x, k) > 0).count() as i16;
            let l = &mut m.lists[i];
            l.index = 0;
            l.select = 0;
            l.dy = 0;
            l.my = held;
            if l.my > 0 {
                l.y = l.my;
                if l.y >= 9 {
                    l.y = 8;
                }
                m.exception_disp = 1;
                m.proccess += 1;
            } else {
                back(m, x);
            }
        }
        1 => {
            m.select_scr(0, 0, 0, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                cursors(m);
                back(m, x);
            } else if key == x.save.ok() {
                m.exception_disp = 2;
                m.wait_count = 1;
                cursors(m);
                m.proccess += 1;
            }
        }
        2 => {
            let sel = i32::from(m.lists[i].select);
            let mut most = 0;
            let mut seen = 0;
            for k in FOODS {
                let n = imp(x, k);
                if n <= 0 {
                    continue;
                }
                if seen == sel {
                    most = n;
                    m.item_num = (FOOD_CAT << 16) | k;
                    break;
                }
                seen += 1;
            }
            count_keys(m, x, most);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.exception_disp = 1;
                cursors(m);
                m.proccess = 1;
            } else if key == x.save.ok() {
                m.menu_status = 3;
                cursors(m);
                m.proccess += 1;
            }
        }
        3 if m.menu_status == 0 => {
            let t = &x.texts.talk.breeder;
            let mut s = t.help[0].clone();
            s.extend(piney_desktop::kanji::dec2sjis(i32::from(m.wait_count), 16, 0));
            s.extend_from_slice(&x.texts.talk.talk.green);
            s.extend(x.texts.items.item_name(m.item_num >> 16, m.item_num & 0xffff));
            s.extend_from_slice(&x.texts.talk.talk.end);
            open_info(m, x, &s, None);
            let d = x.texts.dialog_default.clone();
            extract_menu(m, d);
            let l = &mut m.lists[i];
            l.x = 6;
            l.y = 2;
            l.sx = l.select;
            l.select = 0;
            l.disp = 11;
            m.exception_disp = 0;
            m.menu_status = 1;
            cursors(m);
            m.proccess += 1;
        }
        4 => {
            m.select(0, 0, false, x);
            let key = pushed_key(x);
            if key == x.save.cancel() {
                m.msg.close();
                cursors(m);
                m.proccess += 1;
            } else if key == x.save.ok() {
                m.msg.close();
                if m.lists[i].select == 0 {
                    let (cat, id, n) = (m.item_num >> 16, m.item_num & 0xffff, i32::from(m.wait_count));
                    let food = i32::from((id - 26) as i16);
                    talk::req(x, TalkReq::Feed { target: pg.handle, food, num: n });
                    bitem::del_item(&mut x.save.save, 0, cat, id, n);
                }
                cursors(m);
                m.proccess += 1;
            }
        }
        5 => {
            m.menu_status = 3;
            m.proccess += 1;
        }
        6 if m.menu_status == 0 => {
            if pg.growth != 0 {
                m.wait_count = 0;
                m.proccess = 10;
            } else {
                m.lists[i].disp = 4;
                m.menu_status = 1;
                cursors(m);
                m.proccess = 0;
            }
        }
        10 => match pg.growth {
            0 => m.proccess += 1,
            2 => {
                match m.wait_count {
                    // `pgEvoMsg[2]`: the two records of a Grunty growing up.
                    0 => record_nameless(m, x, fieldui::of(x.texts.volume).pg_evo_msg_va(), false),
                    60 => record_nameless(m, x, fieldui::of(x.texts.volume).pg_evo_msg_va() + 12, true),
                    120 => m.msg.close(),
                    _ => {}
                }
                m.wait_count += 1;
            }
            3 => {
                let base = Base::npc(x.texts.volume, pg.row).unwrap_or_default();
                let n = i32::from(pg.msg);
                let rec = base.msg.wrapping_add((12 * n) as u32);
                let grp = crate::menus::talk::check_voice_grp(&x.texts.talk.talk, i32::from(base.id));
                talk::open_record(m, x, rec, &base.name, grp, n);
                m.proccess += 2;
            }
            5 => {
                m.lists[i].index = 1;
                m.proccess = 20;
            }
            _ => {}
        },
        11 => {
            m.lists[i].disp = 4;
            m.menu_status = 1;
            cursors(m);
            m.proccess = 0;
        }
        12 => {
            if talk::msg_check(m, x) != 0 {
                m.lists[i].index = 1;
                if pg.exist != 0 {
                    m.proccess += 1;
                } else if bitem::get_item_num(&x.save.save, 0, FOOD_CAT, PG_GIFT) > 0 {
                    m.proccess = 20;
                } else {
                    m.bg_status = 1;
                    if m.still == 0 {
                        talk::sleep_all(m, x);
                    }
                    m.wait_count = 0;
                    m.proccess += 2;
                }
                talk::req(x, TalkReq::GruntyGrowth { target: pg.handle, growth: 4 });
            }
        }
        13 => {
            if pg.growth == 5 {
                m.proccess = 20;
            }
        }
        14 => {
            let w = m.wait_count;
            m.wait_count += 1;
            if w >= 9 {
                let t = x.texts.talk.breeder.clone();
                let mut s = t.get_item[0].clone();
                s.extend_from_slice(&x.texts.gi_green);
                s.extend(x.texts.items.item_name(FOOD_CAT, PG_GIFT));
                s.extend_from_slice(&x.texts.gi_bang);
                open_info(m, x, &s, Some(&t.get_item[1]));
                let order = x.texts.talk.shop.order;
                bitem::add_item(&mut x.save.save, &order, 0, FOOD_CAT, PG_GIFT, 1);
                x.se(74);
                m.proccess += 1;
            }
        }
        15 => {
            if check(m, x) != 0 {
                m.msg.close();
                x.req.push(Request::WakeAll);
                crate::disp::disp(m, x);
                return talk::breathed(talk::Tail::Breeder(Tail::Woken));
            }
        }
        20 => {
            // menuFade->EntryFade(10, 0, 0x80000000): drawn by Disp.
            m.fade = m.menu_fade.entry_fade(10, 0, 0x8000_0000) as i16;
            m.proccess += 1;
        }
        21 => {
            if !personal::check_fade(m, i32::from(m.fade)) {
                talk::affect(x, pg.handle, 11);
                m.proccess += 1;
            }
        }
        22 => {
            personal::continue_fade(m, i32::from(m.fade), 10, 0);
            m.proccess += 1;
        }
        23 => {
            let id = i32::from(m.fade);
            if !personal::check_fade(m, id) {
                personal::delete_fade(m, id);
                if m.lists[i].index == 0 {
                    back(m, x);
                } else {
                    talk::affect(x, pg.handle, 0);
                    m.map_status = 1;
                    m.cursol_off = 0;
                    return talk::close(m, x, Then::Nothing);
                }
            }
        }
        _ => {}
    }
    Flow::Done
}

/// `ccFont::MakeSignedNum(n, v)` (main 0x0015cdd0): `sdec2str(n, v)` (main
/// 0x0015cfc0), `n + 1` cells: the sign (a space for 0 and up) before
/// the first digit shown, the leading zeros but the last blank; a value
/// wider than `n` digits keeps its last `n` with the sign first.
pub fn make_signed_num(s: &mut Spr, n: i32, v: i32) {
    let n = if n >= 127 { 126 } else { n.max(0) } as usize;
    let (sign, mut v) = if v < 0 { (b'-', -i64::from(v)) } else { (b' ', i64::from(v)) };
    let mut buf = vec![b'0'; n + 1];
    for k in (1..=n).rev() {
        buf[k] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    if v != 0 {
        buf[0] = sign;
    } else {
        let mut k = 0;
        while k < n && buf[k] == b'0' {
            buf[k] = b' ';
            k += 1;
        }
        if k > 0 {
            buf[k - 1] = sign;
        }
    }
    s.make_str(&buf);
}

/// `BreedingMenuDisp` (gcmn 0x0054cd70): the foods held (name and count,
/// 8 rows from `dy`), the STATUS window (the six stats of the server's
/// Grunty, `growth[game.server]`; while counting, what the food would
/// make of them, coloured by the food's sign), and the count's window.
pub fn breeding_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let a = m.alpha;
    let fr = x.frame_rate;
    let c = if m.exception_disp == 1 { 7 } else { 0 };
    m.win.set_colour(c);
    m.kanji.set_colour(c);
    m.font.set_colour(c);
    m.win.set_alpha(a);
    m.kanji.set_alpha(a);
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    m.font.set_alpha(a);
    font_type(&mut m.font, 1);
    m.win.dx = 39.0;
    m.win.dy = 36.0;
    let title = if l.title.is_empty() { None } else { Some(&l.title[..]) };
    if l.y < l.my {
        disp_square_sb(&mut m.win, 11, i32::from(l.y), i32::from(l.dy), i32::from(l.my), title);
    } else {
        disp_square(&mut m.win, 11, i32::from(l.y), title);
    }
    // The foods from dy: name, count; the cursor's food.
    let mut buf = Vec::new();
    let (mut seen, mut row) = (0i32, 0i32);
    let mut food = -1;
    for k in FOODS {
        let n = imp(x, k);
        if n <= 0 {
            continue;
        }
        if seen >= i32::from(l.dy) {
            str_cat(&mut buf, &x.texts.items.item_name(FOOD_CAT, k), 16);
            let y = row * 20 + 52;
            if row == i32::from(l.select) - i32::from(l.dy) {
                let df = if m.exception_disp == 2 { 2 } else { 3 };
                m.cursor.disp(&mut m.win, 39.0, from_int(y), 11, row, a, df, fr);
                food = k;
            }
            m.font.dx = 175.0;
            m.font.dy = from_int(y);
            make_num(&mut m.font, 3, n);
            m.kanji.dx = 53.0;
            m.kanji.dy = from_int(y);
            m.kanji.make_packet(row);
            row += 1;
            if row >= 8 {
                break;
            }
        }
        seen += 1;
    }
    extract_menu(m, buf);
    // STATUS.
    m.win.set_colour(7);
    m.win.set_alpha(a);
    m.win.dx = 336.0;
    m.win.dy = 36.0;
    let status = x.texts.talk.breeder.status.clone();
    disp_square(&mut m.win, 9, 6, Some(&status));
    let s0 = &mut m.setting[0];
    s0.set_colour(7);
    s0.set_alpha(a);
    set_clm(s0, 16, 0, 0, 1);
    m.setting_text[0] = x.texts.talk.breeder.status_rows.clone();
    for k in 0..6 {
        let s0 = &mut m.setting[0];
        s0.dx = 350.0;
        s0.dy = from_int(k * 20 + 52);
        s0.make_packet(k);
    }
    let server = x.world.game.server.clamp(0, 4) as usize;
    let g = |k: usize| i32::from(x.save.save.i16(SAVE_GROWTH + 24 * server + 2 * k));
    let rows = [52.0, 72.0, 92.0, 112.0, 132.0, 152.0];
    if m.exception_disp == 2 {
        // `foodTbl[food - 26]`: what the food does to the six stats.
        let f = usize::try_from(food - 26).ok().and_then(|k| fieldui::of(x.texts.volume).food().get(k));
        for (j, &y) in rows.iter().enumerate() {
            let d = f.map_or(0, |f| i32::from([f.size, f.smell, f.crooked, f.cruel, f.iq, f.pure][j]));
            m.font.set_colour(if d < 0 {
                23
            } else if d > 0 {
                20
            } else {
                7
            });
            m.font.set_alpha(a);
            m.font.dx = 434.0;
            m.font.dy = y;
            make_signed_num(&mut m.font, 3, d.wrapping_mul(i32::from(m.wait_count)).wrapping_add(g(j + 1)));
        }
    } else {
        m.font.set_colour(7);
        m.font.set_alpha(a);
        for (j, &y) in rows.iter().enumerate() {
            m.font.dx = 434.0;
            m.font.dy = y;
            make_signed_num(&mut m.font, 3, g(j + 1));
        }
    }
    if m.exception_disp != 2 {
        return;
    }
    // The count's window beside the row.
    let s1 = (i32::from(l.select) - i32::from(l.dy)) * 20 + 54;
    m.win_pr.set_colour(7);
    m.win_pr.set_alpha(a);
    m.win_pr.dx = 257.0;
    m.win_pr.dy = from_int(s1 - 16);
    disp_square(&mut m.win_pr, 2, 1, None);
    let sn = i32::from(l.select) - i32::from(l.dy);
    m.cursor_pr.disp(&mut m.win_pr, 257.0, from_int(s1), 2, sn, a, 6, fr);
    m.font.set_colour(22);
    m.font.set_alpha(a);
    m.font.dx = 277.0;
    m.font.dy = from_int(s1);
    make_num(&mut m.font, 2, i32::from(m.wait_count));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spr::Obj;

    fn text(n: i32, v: i32) -> Vec<u8> {
        let mut s = Spr::new(Obj::MenuFont, 4);
        make_signed_num(&mut s, n, v);
        s.queue[0].text.clone().unwrap_or_default()
    }

    #[test]
    fn signed_numbers() {
        // sdec2str: n + 1 cells, the sign before the first digit shown.
        assert_eq!(text(3, 5), b"   5");
        assert_eq!(text(3, 0), b"   0");
        assert_eq!(text(3, -12), b" -12");
        assert_eq!(text(3, 120), b" 120");
        assert_eq!(text(3, -999), b"-999");
        assert_eq!(text(3, 1234), b" 234");
        assert_eq!(text(3, -1234), b"-234");
    }
}
