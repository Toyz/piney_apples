//! Back in a town from a field: the party's faces, the members' kit and
//! the trade lists as `ccSetupGameCtrl` leaves them.

use std::path::Path;

use piney_data::volume::Volume;

use crate::session::area15::{disc, hold, playing, start};

use super::*;

/// Orca's `charTbl` row.
const ORCA: i32 = 2;

/// The town's world once its set-up is well over.
fn in_town(s: &Session) -> Option<&crate::world::WorldMode> {
    match &s.stage {
        Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30) => Some(w),
        _ => None,
    }
}

/// PERSONAL, Party, Add, member `pc`'s face, OK, the greeting closed: the
/// pad for the menus as they stand (`None` once done).
fn invite_pad(w: &crate::world::WorldMode, f: u64, pc: i32) -> Option<Raw> {
    let ui = w.ui();
    let c = &ui.ctrl;
    let still = |b: Buttons| Raw { buttons: b, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let every = |n: u64, b: Buttons| still(if f.is_multiple_of(n) { b } else { Buttons::NONE });
    let go_to_item = |item: i16| {
        let l = c.list();
        match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == item) {
            Some(r) if r as i16 == l.select => Buttons::CROSS,
            Some(r) if (r as i16) > l.select => Buttons::DOWN,
            Some(_) => Buttons::UP,
            None => Buttons::NONE,
        }
    };
    Some(match (ui.menu_type(), c.proccess) {
        (-1, _) if w.world().party().contains(&pc) => return None,
        (-1, _) => every(30, Buttons::TRIANGLE),
        (0, 1) => every(8, go_to_item(9)),
        (9, 1) => every(8, go_to_item(68)),
        (68, 2) if i32::from(c.face_num) == pc => every(8, Buttons::CROSS),
        (68, 2) => every(8, Buttons::DOWN),
        (68, 5) => every(8, Buttons::CROSS),
        (68, p) if p >= 20 => every(24, Buttons::CROSS),
        (0 | 9, _) if w.world().party().contains(&pc) => every(8, Buttons::CIRCLE),
        _ => still(Buttons::NONE),
    })
}

/// Member `pc` called by the Party menu in `s`'s town (his address and
/// Kite's leave to call set first).
fn invite(s: &mut Session, pc: i32) {
    let Stage::World(w) = &mut s.stage else { panic!("not in a town: {}", Mode::title(s)) };
    let save = &mut w.world_mut().state_mut().save;
    for at in [offset::PARTY_MEMBER_FLAG, offset::PARTY_MEMBER_CALL] {
        save.set_i32(at, save.i32(at) | 1 << pc);
    }
    let mut pad = Pad::default();
    for f in 1..4000u64 {
        let Stage::World(w) = &s.stage else { panic!("left the town: {}", Mode::title(s)) };
        let Some(raw) = invite_pad(w, f, pc) else { return };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    panic!("{pc} never joined: {}", Mode::title(s));
}

/// From the town to story area 14's field, until its fade in.
fn to_the_field(s: &mut Session, iso: &Path) {
    let mut d = Iso::open(iso).unwrap();
    s.world_man = Some(crate::area::story_world_man(&mut d, 14, false).unwrap());
    s.go(Pending::Go(piney_data::area::Go::ChangeArea(1, 14)));
    hold(s, 128, 128, 900, |s| playing(s).is_some());
}

/// Back to Mac Anu by the console's `town 0` (a Gate Out), until the
/// town's set-up is over.
fn back_to_town(s: &mut Session) {
    s.console("town 0");
    hold(s, 128, 128, 900, |s| in_town(s).is_some());
}

/// Issue #3: Orca called by the Party menu in Mac Anu, taken to a field
/// and back: every member's menu face in each (Kite's own before the
/// bracelet, row 18), which the status page and the panels draw
/// (`ccCheckMenuFaceNameParty` in each scene's `ccMenuCtrl`).
#[test]
fn faces_come_back_from_a_field() {
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    hold(&mut s, 128, 128, 900, |s| in_town(s).is_some());
    invite(&mut s, ORCA);
    let faces = |party: [i32; 3], tex: [i32; 4]| {
        for (slot, &id) in party.iter().enumerate() {
            assert_eq!(tex[slot], if id == 0 { 18 } else { id }, "slot {slot} of {party:?}");
        }
    };
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(w.world().party(), [0, ORCA, -1]);
    faces(w.world().party(), w.ui().ctrl.face_tex);
    to_the_field(&mut s, &iso);
    let Stage::Area(a) = &s.stage else { unreachable!() };
    faces(a.world().party(), a.ui().ctrl.face_tex);
    back_to_town(&mut s);
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(w.world().party(), [0, ORCA, -1]);
    faces(w.world().party(), w.ui().ctrl.face_tex);
}

/// Orca's `spcDefaultItemList` row: (category, id, count).
fn kit(v: Volume) -> Vec<(i32, i32, i32)> {
    let rows = piney_data::tables::battle::of(v).spc_default_items();
    rows[ORCA as usize]
        .iter()
        .filter(|e| e.category >= 0)
        .map(|e| (i32::from(e.category), i32::from(e.id), i32::from(e.num)))
        .collect()
}

/// Issue #4: Orca's kit used up in a field (his address known), and his
/// gold; back in Mac Anu `SetSpcItemTown` has bought it back up to its
/// counts with his own gold, not Kite's.
#[test]
fn the_kit_comes_back_in_town() {
    use piney_battle::item::{del_item, get_item_num};
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, Some(14));
    hold(&mut s, 128, 128, 900, |s| playing(s).is_some());
    let kit = kit(Volume::Inf);
    assert!(!kit.is_empty());
    let gold = piney_data::save::by_id::spc_param(ORCA as usize) + offset::SPC_GOLD;
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    let save = a.save_mut();
    let kite_gold = save.i32(piney_data::save::by_id::spc_param(0) + offset::SPC_GOLD);
    for at in [offset::PARTY_MEMBER_FLAG, offset::PARTY_MEMBER_CALL] {
        save.set_i32(at, save.i32(at) | 1 << ORCA);
    }
    for &(cat, id, _) in &kit {
        del_item(save, ORCA, cat, id, 99);
        assert_eq!(get_item_num(save, ORCA, cat, id), 0);
    }
    save.set_i32(gold, 5000);
    back_to_town(&mut s);
    let Stage::World(w) = &s.stage else { unreachable!() };
    let save = &w.world().state().save;
    let mut cost = 0;
    for &(cat, id, n) in &kit {
        assert_eq!(get_item_num(save, ORCA, cat, id), n, "item {cat}/{id}");
        cost += n * piney_battle::restock::item_price(Volume::Inf, cat, id);
    }
    assert_eq!(save.i32(gold), (5000 - cost).max(0), "Orca's gold");
    assert_eq!(save.i32(piney_data::save::by_id::spc_param(0) + offset::SPC_GOLD), kite_gold, "Kite's gold");
}

/// Issue #9: Orca's Speed Charm and an NPC's restocking trade taken (the
/// trade empties the slot, `DelSpcTradeList`); back in Mac Anu
/// `SetTradeItemTown` has both back, and out to a field again after a
/// second trade, again.
#[test]
fn trades_come_back_each_scene() {
    use piney_data::save::by_id::{npc_trade_list, spc_trade_list};
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, Some(14));
    hold(&mut s, 128, 128, 900, |s| playing(s).is_some());
    let t = piney_data::tables::newgame::of(Volume::Inf);
    let charm = t.spc_trade()[ORCA as usize - 1][0];
    assert_ne!(charm.sw, 0, "Orca's first trade restocks");
    let (npc, slot, trade) = t
        .npc_trade()
        .iter()
        .enumerate()
        .find_map(|(k, l)| l.iter().take(15).position(|e| e.sw != 0).map(|j| (k, j, l[j].lst)))
        .expect("an NPC's restocking trade");
    let spots = [spc_trade_list(ORCA as usize - 1), npc_trade_list(npc) + 4 * slot];
    let take = |save: &mut SaveData| {
        for at in spots {
            save.set_i16(at, -1);
            save.set_u8(at + 2, 0xff);
            save.set_u8(at + 3, 0);
        }
    };
    let held = |save: &SaveData| spots.map(|at| (save.i16(at), save.u8(at + 2) as i8, save.u8(at + 3) as i8));
    let want = [(charm.lst.id, charm.lst.category, charm.lst.num), (trade.id, trade.category, trade.num)];
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    take(a.save_mut());
    back_to_town(&mut s);
    let Stage::World(w) = &mut s.stage else { unreachable!() };
    assert_eq!(held(&w.world().state().save), want, "in Mac Anu");
    take(&mut w.world_mut().state_mut().save);
    to_the_field(&mut s, &iso);
    let Stage::Area(a) = &s.stage else { unreachable!() };
    assert_eq!(held(&a.world().state().save), want, "in the field");
}

/// Mistral's `charTbl` row.
const MISTRAL: i32 = 16;

/// Issue #2, as the report shows it: Mistral in the party (with
/// BlackRose) runs off through Mac Anu to a shop, and Kite goes up to her
/// and presses the action button. Her menu (`SpcMenu`, 21) sends
/// `EntryAffect` 14 (`ccAI::Greeting`: talkFlag, gDeg); she stops where it
/// found her and turns to face Kite, and stays stopped through Talk (47).
#[test]
fn mistral_spoken_to_while_running_stands_facing_kite() {
    use piney_world::entry::Kind;
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    hold(&mut s, 128, 128, 900, |s| in_town(s).is_some());
    invite(&mut s, 15);
    invite(&mut s, MISTRAL);
    // Her place, heading, act and flags, the heading from her to Kite, the menu.
    let mistral = |s: &Session| {
        let Stage::World(w) = &s.stage else { panic!("left the town: {}", Mode::title(s)) };
        let tp = w.world().town_party();
        let k = tp.member(MISTRAL).expect("Mistral in town");
        let (at, dirc) = tp.place(MISTRAL).unwrap();
        let sp = tp.combat.crew.spc.get(&k).copied().unwrap_or_default();
        let kite = w.world().player().body.pos;
        let (dx, dy) =
            (f32::from_bits(kite[0]) - f32::from_bits(at[0]), f32::from_bits(kite[1]) - f32::from_bits(at[1]));
        let moving = sp.move_flag || sp.run_flag;
        (at, f32::from_bits(dirc[2]), sp.act_num, moving, dx.atan2(-dy), w.ui().ctrl.menu)
    };
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut pad = Pad::default();
    let mut opened = None;
    for f in 0..6000u64 {
        let (at, _, act, _, _, menu) = mistral(&s);
        if menu == 21 {
            opened = Some((f, at));
            break;
        }
        let Stage::World(w) = &s.stage else { unreachable!() };
        let world = w.world();
        let raw = if f < 300 || act != piney_battle::fellow::act::RUN {
            still
        } else if world.command_target() == Some((Kind::Spc, MISTRAL)) {
            Raw { buttons: if f % 30 == 0 { Buttons::CROSS } else { Buttons::NONE }, ..still }
        } else {
            let kite = world.player().body.pos;
            let (dx, dy) =
                (f32::from_bits(at[0]) - f32::from_bits(kite[0]), f32::from_bits(at[1]) - f32::from_bits(kite[1]));
            stick_toward(f32::from_bits(world.camera().rot()[2]), dx.atan2(-dy))
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    let (f0, at0) = opened.expect("Mistral's menu never opened");
    let mut step = |s: &mut Session, b: Buttons| {
        pad.read(&Raw { buttons: b, ..still });
        s.step(&pad);
        s.take_events();
    };
    for f in 0..40 {
        step(&mut s, Buttons::NONE);
        let (at, _, act, _, _, _) = mistral(&s);
        let d = (0..2).map(|k| (f32::from_bits(at[k]) - f32::from_bits(at0[k])).powi(2)).sum::<f32>().sqrt();
        assert!(d < 1.0, "Mistral ran {d:.0} on, {f} frames into her menu (act {act}, opened at {f0})");
    }
    let (_, dirc, act, _, to_kite, _) = mistral(&s);
    assert!((dirc - to_kite).abs() < 0.01, "Mistral faces {dirc:.3}, Kite is at {to_kite:.3} (act {act})");
    // Talk: she stays stopped while it is up.
    step(&mut s, Buttons::CROSS);
    for f in 0..300 {
        step(&mut s, Buttons::NONE);
        let (_, _, act, moving, _, menu) = mistral(&s);
        assert!(!moving && act != piney_battle::fellow::act::RUN, "Mistral moves {f} frames into Talk (act {act})");
        assert!(menu == 47 || f < 10, "not in her Talk page: menu {menu}");
    }
}

/// The CHAT menu's Operation outlasts the scene (issue #7's path): with
/// "Union Battle" in the save (operation 8), Orca's AI in the next field
/// starts with strategy 1, the `partyStrategy` the town's `ccThSpc` last
/// set (`ccAI::ccAI` reads the global before the field's `ccThSpc` runs).
#[test]
fn the_operation_holds_from_a_field_s_start() {
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    hold(&mut s, 128, 128, 900, |s| in_town(s).is_some());
    invite(&mut s, ORCA);
    let Stage::World(w) = &mut s.stage else { unreachable!() };
    w.world_mut().state_mut().save.set_u8(piney_battle::frame::SAVE_OPERATION, 8);
    let mut pad = Pad::default();
    for _ in 0..5 {
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
    to_the_field(&mut s, &iso);
    let Stage::Area(a) = &s.stage else { unreachable!() };
    let c = a.world().combat();
    let orca = c.who(ORCA).expect("Orca in the field");
    let ai = &c.crew.ais[&orca];
    assert_eq!((ai.strategy, ai.strategy_cmd), (1, 1), "Orca's strategy as the field starts");
}
