//! The menus' handlers, called by the task once a frame by the menu open
//! (`ccThMenu`'s jump table at gcmn 0x006e0300, by `CheckMenuType() + 1`).
//!
//! | menu | handler | what |
//! | ---: | --- | --- |
//! | 0, 1, 2 | `SystemMenu` | PERSONAL: town, field, dungeon (triangle) |
//! | 3 | `ChatMenu` | CHAT: the party's orders (square) |
//! | 71 - 73 | `ChatMenu1`, `ChatMenu2`, `ChatMenu3` | CHAT's member: its orders, skill, target ([`chat_member`]) |
//! | 4 | `SkillMenu` | Skills (PERSONAL) |
//! | 5 | `ItemMenu` | Items (PERSONAL) |
//! | 6, 7 | `ImportantItemMenu`, `ThrowItemMenu` | Key Items, Discard Item |
//! | 8, 31, 64 | `StatusMenu`, `ItemStatusMenu`, `EquipStatusMenu` | Status, a member's items, a piece's status |
//! | 9, 68 - 70 | `PartyMenu`, `PartyInMenu`, `PartyOutMenu`, `PartyDisbandMenu` | PARTY: Add, Remove, Disband |
//! | 10, 11 | `GateoutMenu`, `LogoutMenu` | Gate Out, Log Out |
//! | 63 | `EquipmentMenu` | Equipment |
//! | 86, 87 | `TransFieldMenu`, `AreaInfoMenu` | back to the field; Area Information |
//! | 12 | `SystemMenu` | OPTION (START) |
//! | 28 | `GateMenu` | the Chaos Gate |
//! | 57 - 61 | `GtRandomMenu`, `GtNewMenu`, `GtListMenu`, `GtRecordMenu`, `GtTownMenu` | its pages |
//! | 62 | `GtHackMenu` | the gate hack, a protected area's cores ([`hack`]) |
//! | 21, 23, 47, 50 | `SpcMenu`, `NpcMenu`, `TalkMenu`, `PresentMenu` | talking, Gift ([`talk`]) |
//! | 22, 24, 25, 26 | `PcMenu`, `VenderMenu`, `RecorderMenu`, `FairyshopMenu` | the lists the action button opens on a PC and the merchants ([`merchant`]) |
//! | 27, 56 | `BreederMenu`, `BreedingMenu` | the Grunty breeders ([`breeder`]) |
//! | 44 - 46 | `NorainuMenu`, `OtonainuMenu`, `InuMenu` | Dun Loireag's stray dogs and Grunties ([`inu`]) |
//! | 48, 49 | `TradeMenu`, `TradeSubMenu` | trading with a PC ([`trade`]) |
//! | 51, 52, 54, 55 | `SellMenu`, `BuyMenu`, `ItemDepositMenu`, `ItemDrawMenu` | the shops and Elf's Haven ([`shop`]) |
//! | 53 | `RecordMenu` | the Recorder's Save ([`record`]) |
//! | 13 - 20 | `ControllerMenu`, `VibrationMenu`, `ScreenMenu`, `SoundMenu`, `ResetMenu`, `DataDrainDemoMenu`, `VoiceMenu`, `StrwinMenu` | the OPTION pages |
//! | 29, 30, 32, 33, 38, 67 | `GetItemMenu`, `ReplaceItemMenu`, `ItemBoxMenu`, `TrapBoxMenu`, `ItemIdolMenu`, `DataDrainSubMenu` | an item got, the boxes and idols ([`getitem`]) |
//! | 34 - 37, 39, 43 | `ItemObjMenu`, `TrapObjMenu`, `SymbolMenu`, `VirusMenu`, `TimeIdolMenu`, `FoodMenu` | the breakables, symbols, virus crystals, the Zeit statue and the Grunty foods ([`objects`]) |
//! | 65 | `TargetMenu` | a skill's or an item's target, and its use |
//! | 66, 67 | `DataDrainMenu`, `DataDrainSubMenu` | Data Drain ([`drain`]) and its drops |
//! | 75 | `PersonalMenuT` | PERSONAL, the party tutorial (event 2) |
//! | 76 | `PartyMenuT` | PARTY, the party tutorial |
//! | 77 | `PartyInMenuT` | Add to party, the party tutorial |
//!
//! Every other number is listed in `docs/engine/field-ui.md`; the port
//! closes a menu it has no handler for (and says so with
//! [`crate::Request::Unported`]).

pub mod breeder;
pub mod chat;
pub mod chat_member;
pub mod drain;
pub mod equip;
pub mod fountain;
pub mod gate;
pub mod getitem;
pub mod hack;
pub mod inu;
pub mod item;
pub mod keyitem;
pub mod leave;
pub mod merchant;
pub mod objects;
pub mod option;
pub mod party;
pub mod party_menus;
pub mod personal;
pub mod record;
pub mod shop;
pub mod skill;
pub mod status;
pub mod stream;
pub mod system;
pub mod talk;
pub mod target;
pub mod trade;
pub mod tutorial;
pub mod useitem;

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl};

/// `ccEvent::CheckOperate(n, flag)` (0x001b32f0): operation `n` allowed;
/// flag 0 records the first one asked in `operateSet`.
pub fn check_operate(x: &mut Ctx, n: i32, flag: i32) -> bool {
    if n < 0 {
        return true;
    }
    if flag == 0 && x.save.operate_set < 0 {
        x.save.operate_set = n as i16;
    }
    let bit = (1i32.wrapping_shl(n as u32)) as i64 as u64;
    !(x.save.operate != 0 && x.save.operate & bit != 0)
}

/// The handler of the menu open.
pub fn handler(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let t = m.check_menu_type();
    match t {
        -1 | 88 => Flow::Done,
        0..=2 | 12 => system::system_menu(m, x),
        3 => chat::chat_menu(m, x),
        71 => chat_member::chat_menu1(m, x),
        72 => chat_member::chat_menu2(m, x),
        73 => chat_member::chat_menu3(m, x),
        4 => skill::skill_menu(m, x),
        5 => item::item_menu(m, x),
        6 => keyitem::important_item_menu(m, x),
        7 => keyitem::throw_item_menu(m, x),
        8 => status::status_menu(m, x),
        9 => party_menus::party_menu(m, x),
        10 => leave::gateout_menu(m, x),
        11 => leave::logout_menu(m, x),
        28 => gate::gate_menu(m, x),
        29 => getitem::get_item_menu(m, x),
        30 => getitem::replace_item_menu(m, x),
        31 => status::item_status_menu(m, x),
        32 => getitem::item_box_menu(m, x),
        33 => getitem::trap_box_menu(m, x),
        38 => getitem::item_idol_menu(m, x),
        34 => objects::item_obj_menu(m, x),
        35 => objects::trap_obj_menu(m, x),
        36 => objects::symbol_menu(m, x),
        37 => objects::virus_menu(m, x),
        43 => objects::food_menu(m, x),
        39 => objects::time_idol_menu(m, x),
        40 => fountain::fountain_menu(m, x),
        41 => fountain::fountain_menu2(m, x),
        42 => fountain::fountain_menu3(m, x),
        67 => getitem::data_drain_sub_menu(m, x),
        57 => gate::random_menu(m, x),
        58 => gate::new_menu(m, x),
        59 => gate::list_menu(m, x),
        60 => gate::record_menu(m, x),
        61 => gate::town_menu(m, x),
        62 => hack::gt_hack_menu(m, x),
        66 => drain::data_drain_menu(m, x),
        74 => stream::stream_menu(m, x),
        21 => talk::spc_menu(m, x),
        22 => merchant::pc_menu(m, x),
        23 => talk::npc_menu(m, x),
        24 => merchant::merchant_menu(m, x, true),
        25 | 26 => merchant::merchant_menu(m, x, false),
        27 => breeder::breeder_menu(m, x),
        44 => inu::norainu_menu(m, x),
        45 => inu::otonainu_menu(m, x),
        46 => inu::inu_menu(m, x),
        47 => talk::talk_menu(m, x),
        48 => trade::trade_menu(m, x),
        49 => trade::trade_sub_menu(m, x),
        50 => talk::present_menu(m, x),
        51 => shop::sell_menu(m, x),
        52 => shop::buy_menu(m, x),
        53 => record::record_menu(m, x),
        54 => shop::item_deposit_menu(m, x),
        55 => shop::item_draw_menu(m, x),
        56 => breeder::breeding_menu(m, x),
        63 => equip::equipment_menu(m, x),
        64 => status::equip_status_menu(m, x),
        13 => option::controller_menu(m, x),
        14 => option::vibration_menu(m, x),
        15 => option::screen_menu(m, x),
        16 => option::sound_menu(m, x),
        17 => option::reset_menu(m, x),
        18 => option::data_drain_demo_menu(m, x),
        19 => option::voice_menu(m, x),
        20 => option::strwin_menu(m, x),
        65 => target::target_menu(m, x),
        68 => party_menus::party_in_menu(m, x),
        69 => party_menus::party_out_menu(m, x),
        70 => party_menus::party_disband_menu(m, x),
        78 => gate::gate_menu_t(m, x),
        79 => gate::new_menu_t(m, x),
        75 => tutorial::personal_menu_t(m, x),
        76 => tutorial::party_menu_t(m, x),
        77 => tutorial::party_in_menu_t(m, x),
        80 => tutorial::personal_menu_ts(m, x),
        81 => tutorial::skill_menu_t(m, x),
        82 => tutorial::target_menu_t(m, x),
        83 => tutorial::chat_menu_t(m, x),
        // The table's case for 84 has no break: ItemBoxMenuT runs twice a
        // frame there (85's case follows it).
        84 => {
            getitem::item_box_menu_t(m, x);
            getitem::item_box_menu_t(m, x)
        }
        85 => getitem::item_box_menu_t(m, x),
        86 => leave::trans_field_menu(m, x),
        87 => leave::area_info_menu(m, x),
        _ => unported(m, t as i16, x),
    }
}

/// `ExceptionDisp` (0x00522460): the open menu's own page, by menu.
pub fn exception_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    match m.menu {
        3 | 83 => chat::chat_menu_disp(m, x),
        4 | 72 | 81 => skill::skill_menu_disp(m, x),
        5 | 30 => item::item_menu_disp(m, x),
        6 => keyitem::important_item_menu_disp(m, x),
        7 => keyitem::throw_item_menu_disp(m, x),
        8 => status::status_menu_disp(m, x),
        31 => status::item_status_menu_disp(m, x),
        63 => equip::equipment_menu_disp(m, x),
        64 => status::equip_status_menu_disp(m, x),
        57 | 58 | 79 | 87 => gate::new_menu_disp(m, x),
        59 => gate::list_menu_disp(m, x),
        60 => gate::record_menu_disp(m, x),
        61 => gate::town_menu_disp(m, x),
        62 => hack::gt_hack_menu_disp(m, x),
        68 | 77 => party::party_in_menu_disp(m, x),
        48 | 49 => trade::trade_menu_disp(m, x),
        50 => talk::present_menu_disp(m, x),
        51 => shop::sell_menu_disp(m, x),
        52 => shop::buy_menu_disp(m, x),
        53 => record::record_menu_disp(m, x),
        54 => shop::item_deposit_menu_disp(m, x),
        55 => shop::item_draw_menu_disp(m, x),
        46 => inu::inu_menu_disp(m, x),
        56 => breeder::breeding_menu_disp(m, x),
        13 => option::controller_menu_disp(m, x),
        14 | 18 | 19 | 20 => option::on_off_menu_disp(m, x),
        15 => option::screen_menu_disp(m, x),
        16 => option::sound_menu_disp(m, x),
        41 => fountain::fountain_menu_disp2(m, x),
        42 => fountain::fountain_menu_disp3(m, x),
        _ => {}
    }
}

/// A menu the port has no handler for: shut at once.
pub(crate) fn unported(m: &mut MenuCtrl, t: i16, x: &mut Ctx) -> Flow {
    x.req.push(Request::Unported(t));
    m.close_menu(x)
}
