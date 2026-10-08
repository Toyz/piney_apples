//! From Mutation on, the Event NPC (`npcTbl` 175 + town, flags 0x10000000)
//! stands in the town while ITEM COMPLETE's status (`eventStatus[52]`) is
//! set. Spoken to, it lists Talk and Item List (menu 90); Item List (91)
//! registers what Kite holds, wears and keeps at Elf's Haven, and once
//! every item is in it gives the desktop items and the NPC goes.

use std::path::PathBuf;

use piney_data::save::{by_id, ext, registry_row};
use piney_input::{Buttons, Raw};
use piney_world::entry::Kind;

use super::*;

/// The Event NPC's type bit.
const EVENT_NPC: u32 = 0x1000_0000;
/// `eventStatus[52]`: ITEM COMPLETE's.
const ITEM_COMPLETE: usize = offset::EVENT_STATUS + 52;
const MENU_LIST: i16 = 90;
const MENU_ITEMS: i16 = 91;

fn still(buttons: Buttons) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// A new Mutation game logged in to Dun Loireag with ITEM COMPLETE's status
/// set, Kite's bag holding `bag` (category, id), and the registry full but
/// `missing` when `full`. None without the disc.
fn dun_loireag(bag: &[(i8, i16)], full: bool, missing: &[(i32, i32)]) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !iso.exists() {
        eprintln!("mutation.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    let s = &mut state.save;
    s.set_u8(offset::LAST_TOWN, 1);
    s.set_u8(ITEM_COMPLETE, 1);
    let at = by_id::item_list(0);
    for k in 0..40 {
        let (cat, id) = bag.get(k).copied().unwrap_or((-1, -1));
        s.set_i16(at + 4 * k, id);
        s.set_u8(at + 4 * k + 2, cat as u8);
        s.set_u8(at + 4 * k + 3, u8::from(cat >= 0));
    }
    if full {
        for row in 0..16 {
            for w in 0..4 {
                s.set_i32(ext::ITEM_REGISTRY + 16 * row + 4 * w, -1);
            }
        }
        for &(cat, id) in missing {
            let at = ext::ITEM_REGISTRY + 16 * registry_row(cat).unwrap() + 4 * (id / 32) as usize;
            let v = s.i32(at) as u32 & !(1u32 << (id % 32));
            s.set_i32(at, v as i32);
        }
    }
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    Some(Session::in_world(iso, archive, None, state, None, scene, None).unwrap())
}

fn world(s: &Session) -> &crate::world::WorldMode {
    match &s.stage {
        Stage::World(w) => w,
        _ => panic!("left the town: {}", Mode::title(s)),
    }
}

/// The menu open and its step.
fn menu(s: &Session) -> (i16, i16, i16) {
    let c = &world(s).ui().ctrl;
    (c.menu, c.menu_status, c.proccess)
}

fn press(s: &mut Session, pad: &mut Pad, b: Buttons) {
    pad.read(&still(b));
    s.step(pad);
    s.take_events();
}

/// Kite walks to the Event NPC round the town's walls and speaks to it;
/// false if its list (90) did not open.
fn speak_to_event_npc(s: &mut Session, pad: &mut Pad) -> bool {
    let mut walk = None;
    for f in 0..4000u32 {
        let (raw, open) = {
            let w = world(s);
            let world = w.world();
            let placed = matches!(world.phase(), piney_world::Phase::Play(n) if n >= 60);
            let npc = world.merchants().iter().find(|m| m.flags & EVENT_NPC != 0);
            let open = menu(s) == (MENU_LIST, 2, 1);
            let raw = match npc {
                _ if !placed || open => still(Buttons::NONE),
                Some(m) if world.command_target() == Some((Kind::Npc, m.id)) => {
                    still(if f % 20 == 0 { Buttons::CROSS } else { Buttons::NONE })
                }
                Some(m) => {
                    let q = m.ch.pos.map(f32::from_bits);
                    let w = walk.get_or_insert_with(|| super::town_walk::TownWalker::new([q[0], q[1]], 300.0));
                    w.stick(world, f).unwrap_or_else(|| {
                        let kite = world.player().body.pos.map(f32::from_bits);
                        let cam = f32::from_bits(world.camera().rot()[2]);
                        super::stick_toward(cam, (q[0] - kite[0]).atan2(-(q[1] - kite[1])))
                    })
                }
                None => panic!("no Event NPC in Dun Loireag"),
            };
            (raw, open)
        };
        if open {
            return true;
        }
        pad.read(&raw);
        s.step(pad);
        s.take_events();
    }
    false
}

/// From the NPC's list, Item List chosen; then Cross every 12 frames until
/// `done` holds.
fn item_list(s: &mut Session, pad: &mut Pad, done: impl Fn(&Session) -> bool) {
    press(s, pad, Buttons::DOWN);
    for _ in 0..8 {
        press(s, pad, Buttons::NONE);
    }
    press(s, pad, Buttons::CROSS);
    for k in 0..3000u32 {
        if done(s) {
            return;
        }
        press(s, pad, if k % 12 == 11 { Buttons::CROSS } else { Buttons::NONE });
    }
    panic!("Item List did not get there: menu {:?}", menu(s));
}

/// Item List registers what is new in Kite's bag and what he wears, shows
/// its two windows and the groups with the count, and cancel goes back to
/// the NPC's list. Before the port the list (90) shut at once.
#[test]
fn the_event_npc_registers_the_bag() {
    let bag = [(0, 3), (10, 5), (14, 2), (11, 8), (0, 70)];
    let Some(mut s) = dun_loireag(&bag, false, &[]) else { return };
    let mut pad = Pad::default();
    assert!(speak_to_event_npc(&mut s, &mut pad), "the Event NPC's list did not open");
    item_list(&mut s, &mut pad, |s| menu(s) == (MENU_ITEMS, 2, 41));
    let save = &world(&s).world().state().save;
    for (cat, id) in [(0, 3), (-2, 5), (14, 2), (11, 8)] {
        assert!(save.registered(cat, id), "({cat}, {id}) not registered");
    }
    // Past the weapons' listed ids: left out.
    assert!(!save.registered(0, 70), "(0, 70) registered");
    let c = &world(&s).ui().ctrl;
    assert!(c.talk.temp[0] >= 4 && c.talk.temp[1] == 758, "the count: {:?}", &c.talk.temp[..2]);
    press(&mut s, &mut pad, Buttons::CIRCLE);
    for _ in 0..40 {
        press(&mut s, &mut pad, Buttons::NONE);
    }
    assert_eq!(menu(&s).0, MENU_LIST, "not back on the NPC's list");
    assert_eq!(world(&s).world().state().save.u8(ITEM_COMPLETE), 1, "ITEM COMPLETE's status cleared");
}

/// The last item comes in from the bag: the NPC's word, wallpaper 56, BGM
/// 51 and movies 90-96 on the desktop, ITEM COMPLETE's status cleared, the
/// menu shut, and the NPC gone from the town.
#[test]
fn item_complete_gives_the_desktop_items_and_the_npc_goes() {
    let Some(mut s) = dun_loireag(&[(1, 7)], true, &[(1, 7)]) else { return };
    let mut pad = Pad::default();
    assert!(speak_to_event_npc(&mut s, &mut pad), "the Event NPC's list did not open");
    item_list(&mut s, &mut pad, |s| world(s).ui().ctrl.menu == -1);
    let save = &world(&s).world().state().save;
    let bit = |at: usize, n: usize| save.i32(at + 4 * (n / 32)) as u32 & 1 << (n % 32) != 0;
    assert!(bit(offset::DT_WALLPAPER_LIST, 55), "wallpaper 56");
    assert!(bit(offset::DT_BGM_LIST, 51), "BGM 51");
    assert!((89..=95).all(|n| bit(offset::DT_STR_LIST, n)), "movies 90-96");
    assert_eq!(save.u8(ITEM_COMPLETE), 0, "ITEM COMPLETE's status");
    for _ in 0..200 {
        if world(&s).world().merchants().iter().all(|m| m.flags & EVENT_NPC == 0) {
            return;
        }
        press(&mut s, &mut pad, Buttons::NONE);
    }
    panic!("the Event NPC is still in the town");
}
