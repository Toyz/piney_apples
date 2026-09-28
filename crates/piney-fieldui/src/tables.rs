//! The menu's tables and texts: the volume's generated tables
//! (`piney_data::tables::fieldui`), as the Shift-JIS the menus draw.

use std::collections::HashMap;

use piney_data::tables::fieldui::{self, FieldMenuElement};
use piney_data::tables::sjis::encode;
use piney_data::volume::Volume;
use piney_event::ir::Message;

/// `menuList[89]`.
pub const LISTS: usize = 89;
/// `menuFaceCcsList`'s rows: row 18 is Kite before the bracelet.
pub const MENU_FACES: usize = 19;
/// gcmn's literals the Chaos Gate pages draw with: "#B", " ", sixteen
/// spaces, "."; and the item windows': "#G", "#W!", "#W.", "#W?".
pub const GT_HASH_B: &[u8] = b"#B";
pub const GT_SPACE: &[u8] = b" ";
pub const GT_SPACES16: &[u8] = b"                ";
pub const GT_DOT: &[u8] = b".";
pub const GI_GREEN: &[u8] = b"#G";
pub const GI_BANG: &[u8] = b"#W!";
pub const GI_DOT: &[u8] = b"#W.";
pub const GI_QUESTION: &[u8] = b"#W?";
/// gcmn's short words: the HP separator, the off-screen enemy tag, "WORD",
/// "STATUS", "Lv.", LevelDisp's "/1000" and "GP", `STATUS_UP_STOP`.
pub const SLASH: &[u8] = b"/";
pub const ENEMY: &[u8] = b"ENEMY";
pub const GT_WORD: &[u8] = b"WORD";
pub const GT_STATUS: &[u8] = b"STATUS";
pub const GT_LV: &[u8] = b"Lv.";
pub const EXP_MAX: &[u8] = b"/1000";
pub const GP: &[u8] = b"GP";
pub const STATUS_UP_STOP: &[u8] = b".";

/// One `ccMenuList` (0x20 bytes) as `InitMenuList` and the menus leave it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuList {
    /// `element->str`: the list's name as other lists show it.
    pub name: Vec<u8>,
    /// `element->strT`: the window's title.
    pub title: Vec<u8>,
    /// `data`: the items (menu numbers, -1 filled by the menu).
    pub items: Vec<i16>,
    /// +0x08 `prev`: the list to go back to, -1 none.
    pub prev: i16,
    /// +0x0a `select`.
    pub select: i16,
    /// +0x0c `page`, +0x0e `pageNum`.
    pub page: i16,
    pub page_num: i16,
    /// +0x10 `index`.
    pub index: i16,
    /// +0x12 `disp`: how Disp draws the window.
    pub disp: i16,
    /// +0x14 `x`: the window's width in cells.
    pub x: i16,
    /// +0x16 `y`: the rows shown.
    pub y: i16,
    /// +0x18 `sx`, +0x1a `sy`.
    pub sx: i16,
    pub sy: i16,
    /// +0x1c `dy`: the first row shown; +0x1e `my`: the rows in all.
    pub dy: i16,
    pub my: i16,
}

/// Everything the menus read from the executable.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    /// The disc's volume (`volumeNum`).
    pub volume: Volume,
    pub lists: Vec<MenuList>,
    /// Two lines per entry.
    pub personal_help: Vec<[Vec<u8>; 3]>,
    pub option_help: Vec<[Vec<u8>; 3]>,
    pub help: Vec<Vec<u8>>,
    pub dead_info: Vec<u8>,
    pub new_mail: Vec<u8>,
    pub kyvia_status: Vec<Vec<u8>>,
    /// `cheatHpStr`: the digits the target window uses for 5-digit HP.
    pub cheat_hp: Vec<u8>,
    pub slash: Vec<u8>,
    pub enemy: Vec<u8>,
    pub panel_flash: [u32; 5],
    /// (file, texture) per face.
    pub faces: Vec<(String, String)>,
    /// The event message tables the tutorial menus read (`evMsgTblM1`,
    /// `evMsgTblM1p` in Parody Mode): events 2, 3 and 4.
    pub tutorial: HashMap<u16, Vec<Message>>,
    pub tutorial_parody: HashMap<u16, Vec<Message>>,
    /// `getItemMenuStr`'s pieces for the member announcement.
    pub announce: crate::message::AnnounceTexts,
    /// `charTbl`'s names by row (`spcParam[pc].base.name`), row 0 empty
    /// (Kite's is `plName`).
    pub char_names: Vec<Vec<u8>>,
    /// The skill and item tables.
    pub items: crate::items::ItemTables,
    /// `skillMenuTag` / `itemMenuTag`: the pages' tab names.
    pub skill_tags: Vec<Vec<u8>>,
    pub item_tags: Vec<Vec<u8>>,
    /// `skillMenuHelp[3]`, `itemMenuHelp[10]`: two lines each.
    pub skill_help: Vec<[Vec<u8>; 2]>,
    pub item_help: Vec<[Vec<u8>; 2]>,
    /// `targetMenuInfo` ("Select target you wish to use ", "."),
    /// `targetMenuWarn`, `targetMenuDrainWarn` (two lines).
    pub target_info: [Vec<u8>; 2],
    pub target_warn: Vec<u8>,
    pub target_drain_warn: [Vec<u8>; 2],
    /// `dialogDefault`: the OK / Cancel rows.
    pub dialog_default: Vec<u8>,
    /// `virusCol` (gcmn 0x00651570): the bracelet gauge's two colour pairs
    /// by `erosion / 17`, r g b each.
    pub virus_col: Vec<[i32; 12]>,
    /// CHAT: `chatMenuTag`'s tabs, `chatMenuStr`'s orders (0-7 in battle,
    /// 8-11 strategies, 12-15 and 18 a member's own, 16-17 in town),
    /// `chatActionStr`'s pieces (11), `chatMenuHelp[19]` (three lines
    /// each), `chatMenuWarn[6]` (two lines each), `partyMenuWarn[5]` (two).
    pub chat_tags: Vec<Vec<u8>>,
    pub chat_str: Vec<Vec<u8>>,
    pub chat_action: Vec<Vec<u8>>,
    pub chat_help: Vec<[Vec<u8>; 3]>,
    pub chat_warn: Vec<[Vec<u8>; 2]>,
    pub party_warn: Vec<[Vec<u8>; 2]>,
    /// `btInChatAct[12]` (gcmn 0x006516d0): the orders' and strategies'
    /// command numbers; `townChatAct[2]` (0x00378258) the town's.
    pub bt_chat_act: Vec<i32>,
    pub town_chat_act: Vec<i32>,
    /// The Chaos Gate's keywords and story areas.
    pub words: crate::words::Words,
    /// The gate hack's texts and file.
    pub hack: crate::menus::hack::HackTexts,
    /// Data Drain's texts.
    pub drain: crate::menus::drain::DrainTexts,
    /// `gateMenuHelp[5]`, `gtTownMenuHelp[5]` (three lines each),
    /// `gtNewInfo[3]` (two), `gtNewStr1[12]` (the attributes and parts).
    pub gate_help: Vec<[Vec<u8>; 3]>,
    pub gt_town_help: Vec<[Vec<u8>; 3]>,
    pub gt_new_info: Vec<[Vec<u8>; 2]>,
    pub gt_new_str1: Vec<Vec<u8>>,
    /// `gtNewStr0`'s pieces (Warp, Cancel, the labels, the elements),
    /// `gtTownMenuStr`'s servers, `serverStr`'s symbols.
    pub gt_new_str0: Vec<Vec<u8>>,
    pub gt_town_str: Vec<Vec<u8>>,
    pub server_str: Vec<Vec<u8>>,
    /// `gtMenuInfo` ("Warp to"), `gtTownMenuWarn`, `gtListMenuWarn`.
    pub gt_menu_info: Vec<u8>,
    pub gt_town_warn: Vec<u8>,
    pub gt_list_warn: Vec<u8>,
    /// `gateWordListMsg[133]`: each story area's note in the Word List
    /// (an event record: up to three NUL-ended lines).
    pub gate_word_list_msg: Vec<[Vec<u8>; 3]>,
    /// From Mutation on, the story areas the warp menus refuse while the
    /// bracelet is off, and Kite's line ([`GateRefusal`]).
    pub gate_refusal: Option<GateRefusal>,
    /// gcmn's literals: "#B", " ", "WORD", sixteen spaces, "STATUS", "Lv.",
    /// the Area Information row, ".".
    pub gt_hash_b: Vec<u8>,
    pub gt_space: Vec<u8>,
    pub gt_word: Vec<u8>,
    pub gt_spaces16: Vec<u8>,
    pub gt_status: Vec<u8>,
    pub gt_lv: Vec<u8>,
    pub gt_area_info: Vec<u8>,
    pub gt_dot: Vec<u8>,
    /// `ccGetGtNewColor`'s colours (gcmn 0x00651718).
    pub gt_new_colours: [i32; 3],
    /// `statusMenuStr[6]` (0x0033e9e0): the Status page's label rows, 16
    /// characters a label (row 0: Level, EXP, Money, Class).
    pub status_menu: Vec<Vec<u8>>,
    /// `statusClassStr`'s lines: the classes' names.
    pub status_class: Vec<Vec<u8>>,
    /// gcmn's "/1000" and "GP" (LevelDisp's EXP and Money).
    pub exp_max: Vec<u8>,
    pub gp: Vec<u8>,
    /// `partyInMenuInfo[2]`: "Add ... to your party." around the name,
    /// then the call's two lines.
    pub party_in_info: [[Vec<u8>; 2]; 2],
    /// `getItemMenuStr`'s lines ("You now have ", ...).
    pub get_item_str: Vec<Vec<u8>>,
    /// `getTrapMenuStr`'s pieces (`ccKanjiStrSeparate`): "Set off trap!",
    /// then the trap's line by kind (1 the explosion, 2 the poison gas, 3
    /// the cursed gas), 4 "But nothing happened."
    pub trap_menu: Vec<Vec<u8>>,
    /// The item uses' windows (`ccUseItemRequest`): `trapDischargeStr`,
    /// `showMapInfo`, `statusUpStr`'s pieces (0 "increased by ", 1
    /// "decreased by ", 2 on the stats' names), `installWarnStr`'s three
    /// lines and `epitaphStr0X`'s first piece.
    pub use_texts: UseTexts,
    /// gcmn's "#G", "#W!", "#W." and "#W?" around an item's name.
    pub gi_green: Vec<u8>,
    pub gi_bang: Vec<u8>,
    pub gi_dot: Vec<u8>,
    pub gi_question: Vec<u8>,
    /// The item lists the boxes draw from (`AreaItem`).
    pub area_items: AreaItems,
    /// The talk and shop menus' executable and texts.
    pub talk: crate::talk::TalkTexts,
    /// PERSONAL's pages' texts.
    pub pers: crate::menus::personal::PersTexts,
    /// The OPTION pages' texts.
    pub option: crate::menus::option::OptionTexts,
    /// The field objects' menus' texts.
    pub objects: crate::menus::objects::ObjTexts,
    pub fountain: crate::menus::fountain::FountainTexts,
}

/// `AreaItem`'s lists, 130 item codes each: `ItemBoxList`,
/// `DangerItemBoxList`, `areaItemList` and `idolItemList` by server and
/// element (server * 6 + element), `SukaItemBoxList` and
/// `idolSubItemList` by server.
#[derive(Clone, Debug, Default)]
pub struct AreaItems {
    pub box_list: Vec<Vec<i32>>,
    pub danger: Vec<Vec<i32>>,
    pub area: Vec<Vec<i32>>,
    pub idol: Vec<Vec<i32>>,
    pub suka: Vec<Vec<i32>>,
    pub idol_sub: Vec<Vec<i32>>,
}

impl AreaItems {
    pub fn of(volume: Volume) -> AreaItems {
        let t = fieldui::of(volume);
        let lists = |l: &[&[i32]]| l.iter().map(|x| x.to_vec()).collect();
        AreaItems {
            box_list: lists(t.item_box_list()),
            danger: lists(t.danger_item_box_list()),
            area: lists(t.area_item_list()),
            idol: lists(t.idol_item_list()),
            suka: lists(t.suka_item_box_list()),
            idol_sub: lists(t.idol_sub_item_list()),
        }
    }
}

/// The gate's refusal (`piney_data::tables::fieldui::GateRefusal`): while
/// `saveData.eventStatus[status]` is set, a warp to one of `areas` (by
/// `GetEventAreaNumber`) shows the record instead.
#[derive(Clone, Debug, Default)]
pub struct GateRefusal {
    pub status: usize,
    pub areas: Vec<i32>,
    pub mode: i32,
    pub name: Option<Vec<u8>>,
    pub lines: [Vec<u8>; 3],
}

/// Piece `k` of a text's lines, empty past its end.
pub fn piece(lines: &[&str], k: usize) -> Vec<u8> {
    lines.get(k).map_or_else(Vec::new, |l| encode(l))
}

fn two(lines: &[&str]) -> [Vec<u8>; 2] {
    [piece(lines, 0), piece(lines, 1)]
}

fn three(lines: &[&str]) -> [Vec<u8>; 3] {
    [piece(lines, 0), piece(lines, 1), piece(lines, 2)]
}

fn all(lines: &[&str]) -> Vec<Vec<u8>> {
    lines.iter().map(|l| encode(l)).collect()
}

fn text(s: Option<&str>) -> Vec<u8> {
    s.map_or_else(Vec::new, encode)
}

fn list(e: &FieldMenuElement) -> MenuList {
    let mut l = MenuList { name: text(e.name), title: text(e.title), prev: -1, ..MenuList::default() };
    if let Some(d) = e.data {
        l.disp = d.disp;
        l.x = d.width;
        l.y = d.items.len() as i16;
        l.items = d.items.to_vec();
    }
    l
}

impl Texts {
    /// The volume's.
    pub fn read(volume: Volume) -> piney_data::Result<Texts> {
        let t = fieldui::of(volume);
        let dialog = piney_data::tables::dtmenu::of(volume).dialog();
        let battle = piney_battle::Tables::of(volume);
        Ok(Texts {
            volume,
            lists: t.elements().iter().map(list).collect(),
            personal_help: t.personal_help().iter().map(|l| three(l)).collect(),
            option_help: t.option_help().iter().map(|l| three(l)).collect(),
            help: t.help().iter().map(|&h| text(h)).collect(),
            dead_info: encode(t.dead_info()),
            new_mail: encode(t.new_mail()),
            kyvia_status: all(t.kyvia_status()),
            cheat_hp: encode(t.cheat_hp()),
            slash: SLASH.to_vec(),
            enemy: ENEMY.to_vec(),
            panel_flash: t.panel_flash().try_into().unwrap_or_default(),
            faces: t
                .faces()
                .iter()
                .map(|f| (f.file.unwrap_or_default().to_string(), f.texture.unwrap_or_default().to_string()))
                .collect(),
            tutorial: HashMap::new(),
            tutorial_parody: HashMap::new(),
            // `charTbl`'s names as `ccSaveData::NewGame` copies them into
            // `spcParam[i].base.name` (20 bytes at most); row 0 (Kite) is the
            // save's `plName` and left empty here.
            char_names: std::iter::once(Vec::new())
                .chain(piney_data::tables::newgame::of(volume).chars().iter().skip(1).map(|c| {
                    let mut n = c.base.name.map(encode).unwrap_or_default();
                    n.truncate(20);
                    n
                }))
                .collect(),
            announce: crate::message::AnnounceTexts {
                you_now_have: piece(t.get_item_str(), 0),
                member_tail: piece(t.get_item_str(), 6),
            },
            items: crate::items::ItemTables::of(volume, &battle),
            skill_tags: all(t.skill_tags()),
            item_tags: all(t.item_tags()),
            skill_help: t.skill_help().iter().map(|l| two(l)).collect(),
            item_help: t.item_help().iter().map(|l| two(l)).collect(),
            target_info: two(t.target_info()),
            target_warn: encode(t.target_warn()),
            target_drain_warn: two(t.target_drain_warn()),
            dialog_default: piney_desktop::dtmenu::slots(dialog),
            virus_col: t.virus_col().to_vec(),
            chat_tags: all(t.chat_tags()),
            chat_str: all(t.chat_str()),
            chat_action: all(t.chat_action()),
            chat_help: t.chat_help().iter().map(|l| three(l)).collect(),
            chat_warn: t.chat_warn().iter().map(|l| two(l.unwrap_or_default())).collect(),
            party_warn: t.party_warn().iter().map(|l| two(l)).collect(),
            bt_chat_act: t.bt_chat_act().to_vec(),
            town_chat_act: t.town_chat_act().to_vec(),
            words: crate::words::Words::of(volume, &battle),
            hack: crate::menus::hack::HackTexts::of(volume),
            drain: crate::menus::drain::DrainTexts::of(volume),
            gate_help: t.gate_help().iter().map(|l| three(l)).collect(),
            gt_town_help: t.gt_town_help().iter().map(|l| three(l)).collect(),
            gt_new_info: t.gt_new_info().iter().map(|l| two(l)).collect(),
            gt_new_str1: t.gt_new_str1().iter().map(|&s| text(s)).collect(),
            gt_new_str0: all(t.gt_new_str0()),
            gt_town_str: all(t.gt_town_str()),
            server_str: all(t.server_str()),
            gt_menu_info: encode(t.gt_menu_info()),
            gt_town_warn: encode(t.gt_town_warn()),
            gt_list_warn: encode(t.gt_list_warn()),
            gate_word_list_msg: t.gate_word_list_msg().iter().map(|l| three(l.unwrap_or_default())).collect(),
            gate_refusal: t.gate_refusal().map(|r| GateRefusal {
                status: r.status as usize,
                areas: r.areas.to_vec(),
                mode: r.msg.emode,
                name: r.msg.name.map(encode),
                lines: three(r.msg.str.unwrap_or_default()),
            }),
            gt_hash_b: GT_HASH_B.to_vec(),
            gt_space: GT_SPACE.to_vec(),
            gt_word: GT_WORD.to_vec(),
            gt_spaces16: GT_SPACES16.to_vec(),
            gt_status: GT_STATUS.to_vec(),
            gt_lv: GT_LV.to_vec(),
            gt_area_info: encode(t.gt_area_info()),
            gt_dot: GT_DOT.to_vec(),
            gt_new_colours: t.gt_new_colours().try_into().unwrap_or_default(),
            status_menu: t.status_menu().iter().map(|s| encode(s)).collect(),
            status_class: all(t.status_class()),
            exp_max: EXP_MAX.to_vec(),
            gp: GP.to_vec(),
            party_in_info: [two(t.party_in_info()[0]), two(t.party_in_info()[1])],
            get_item_str: all(t.get_item_str()),
            trap_menu: all(t.trap_menu()),
            use_texts: UseTexts::of(volume),
            gi_green: GI_GREEN.to_vec(),
            gi_bang: GI_BANG.to_vec(),
            gi_dot: GI_DOT.to_vec(),
            gi_question: GI_QUESTION.to_vec(),
            area_items: AreaItems::of(volume),
            talk: crate::talk::TalkTexts::read(volume)?,
            pers: crate::menus::personal::PersTexts::of(volume, battle),
            option: crate::menus::option::OptionTexts::of(volume),
            objects: crate::menus::objects::ObjTexts::of(volume),
            fountain: crate::menus::fountain::FountainTexts::of(volume),
        })
    }
}

/// The windows an item's use opens ([`piney_battle::item::Info`]).
#[derive(Clone, Debug, Default)]
pub struct UseTexts {
    pub trap_discharge: Vec<u8>,
    pub show_map: Vec<u8>,
    /// `statusUpStr`'s pieces 0-17 (0 "increased by ", 1 "decreased by
    /// ", 2-17 the stats' names).
    pub status_up: Vec<Vec<u8>>,
    /// `STATUS_UP_STOP` (gcmn 0x006e1fb8): the line's end.
    pub status_up_stop: Vec<u8>,
    pub install_warn: [Vec<u8>; 3],
    pub epitaph_unknown: Vec<u8>,
    /// The important items' epitaphs and notes (`ccEpitaphMsg`).
    pub epitaphs: Vec<EpitaphText>,
}

impl UseTexts {
    pub fn of(volume: Volume) -> Self {
        let t = fieldui::of(volume);
        UseTexts {
            trap_discharge: encode(t.trap_discharge()),
            show_map: encode(t.show_map()),
            status_up: all(t.status_up()),
            status_up_stop: STATUS_UP_STOP.to_vec(),
            install_warn: three(t.install_warn()),
            epitaph_unknown: encode(t.epitaph_unknown()),
            epitaphs: epitaphs(t),
        }
    }

    /// Item `item`'s pages, the parody mode's with `parody` (none for an
    /// item with no epitaph).
    pub fn epitaph(&self, item: i32, parody: bool) -> &[[Vec<u8>; 3]] {
        self.epitaphs.iter().find(|e| e.0 == item).map_or(&[][..], |e| if parody { &e.2[..] } else { &e.1[..] })
    }
}

/// An item's epitaph: the item, its pages (three lines each) and the
/// parody mode's.
pub type EpitaphText = (i32, Vec<[Vec<u8>; 3]>, Vec<[Vec<u8>; 3]>);

/// The build's epitaph tables by item (the use code's `ccEpitaphMsg`
/// calls: 287's table serves both modes).
fn epitaphs(t: &fieldui::FieldUi) -> Vec<EpitaphText> {
    let pages = |p: &[&[&str]]| p.iter().map(|l| three(l)).collect::<Vec<_>>();
    [
        (42, t.epitaph_00(), t.epitaph_00p()),
        (43, t.epitaph_01(), t.epitaph_01p()),
        (44, t.epitaph_02(), t.epitaph_02p()),
        (45, t.epitaph_03(), t.epitaph_03p()),
        (46, t.epitaph_04(), t.epitaph_04p()),
        (48, t.epitaph_10(), t.epitaph_10p()),
        (68, t.epitaph_11(), t.epitaph_11p()),
        (287, t.epitaph_m0(), t.epitaph_m0()),
        (288, t.epitaph_m1(), t.epitaph_m1()),
        (289, t.epitaph_m2(), t.epitaph_m2()),
        (290, t.epitaph_m3(), t.epitaph_m3()),
    ]
    .into_iter()
    .map(|(id, n, p)| (id, pages(n), pages(p)))
    .collect()
}

/// The tutorial events whose messages the menus open.
pub const TUTORIAL_EVENTS: [u16; 3] = [2, 3, 4];

impl Texts {
    /// The tutorial events' message tables from the boot executable.
    pub fn read_tutorial(&mut self, events: &[piney_event::ir::Event]) -> Result<(), String> {
        for e in TUTORIAL_EVENTS {
            let ev = events.iter().find(|x| x.number == e).ok_or(format!("no event {e}"))?;
            self.tutorial.insert(e, ev.messages.clone());
            self.tutorial_parody.insert(e, ev.parody.clone());
        }
        Ok(())
    }
}
