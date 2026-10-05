//! A Fairy's Orb (TOOL 2) used from PERSONAL in a random field: its
//! `ccUseItemRequest` steps (the info window, then `WORLD_MAN::ShowMap`
//! once a menu frame until it is done, 20 frames at least) put the magic
//! portals on the field's map (`WORLD::ShowMap`: `mapFlag`). In a random
//! dungeon's fight the orb builds the room under Kite again with its doors
//! shut, as they were.

use piney_input::Buttons;
use piney_world::field_world::Place;

use super::ride::{area, in_field, raw};
use super::*;

fn map_flag(s: &Session) -> bool {
    match &s.stage {
        Stage::Area(a) => match a.world().place() {
            Place::Field(f) => f.map.as_ref().is_some_and(|m| m.map_flag),
            _ => false,
        },
        _ => false,
    }
}

#[test]
fn a_fairys_orb_shows_the_portals() {
    let Some((mut s, _)) = in_field() else { return };
    let mut pad = Pad::default();
    let mut given = false;
    let mut used_at = None;
    for f in 0..3000u32 {
        let ready = match &s.stage {
            Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
            _ => false,
        };
        // Kite's first item: one Fairy's Orb (id 2, TOOL, 1).
        if ready && !given {
            let Stage::Area(a) = &mut s.stage else { unreachable!() };
            let save = &mut a.world_mut().state_mut().save;
            save.set_i16(offset::ITEM_LIST, 2);
            save.set_u8(offset::ITEM_LIST + 2, 13);
            save.set_u8(offset::ITEM_LIST + 3, 1);
            given = true;
            assert!(!map_flag(&s), "the portals before the orb");
        }
        let b = if !ready || !given || !f.is_multiple_of(8) || used_at.is_some() {
            Buttons::NONE
        } else {
            let a = area(&s);
            let c = &a.ui().ctrl;
            // PERSONAL, its second row (Items), the orb, and on.
            match (a.ui().menu_type(), c.proccess) {
                (-1, _) => Buttons::TRIANGLE,
                (1, 1) if c.list().select < 1 => Buttons::DOWN,
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        s.take_events();
        if used_at.is_none() && map_flag(&s) {
            used_at = Some(f);
        }
        if let Some(u) = used_at
            && f > u + 60
        {
            break;
        }
    }
    let u = used_at.expect("the orb never showed the portals");
    let a = area(&s);
    let save = &a.world().state().save;
    assert_eq!(save.u8(offset::ITEM_LIST + 3), 0, "the orb used up (frame {u})");
}

/// An important item's epitaph read from the Key Items (menu 6): item 42,
/// `ccEpitaphMsg`'s pages in the information window one after another as
/// OK is pushed, each page's first line as the build's `epitaph_00` has
/// it, and the Key Items' list answering again after the last (its
/// proccess 12, then 1).
#[test]
fn an_epitaph_reads_its_pages() {
    let Some((mut s, _)) = in_field() else { return };
    let pages = piney_fieldui::tables::UseTexts::of(piney_data::volume::Volume::Inf).epitaph(42, false).to_vec();
    assert_eq!(pages.len(), 2);
    let mut pad = Pad::default();
    let (mut given, mut seen) = (false, Vec::new());
    for f in 0..3000u32 {
        let ready = match &s.stage {
            Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
            _ => false,
        };
        if ready && !given {
            let Stage::Area(a) = &mut s.stage else { unreachable!() };
            a.world_mut().state_mut().save.set_u8(offset::IMP_ITEM_LIST + 42, 1);
            given = true;
        }
        let a = area(&s);
        let c = &a.ui().ctrl;
        if let Some(l) = c.msg.line(0)
            && let Some(k) = pages.iter().position(|p| p[0] == l)
            && !seen.contains(&k)
        {
            seen.push(k);
        }
        if seen.len() == pages.len() && a.ui().menu_type() == 6 && c.proccess == 1 {
            break;
        }
        let b = if !ready || !given || !f.is_multiple_of(8) {
            Buttons::NONE
        } else {
            // PERSONAL, Key Items, the item, and OK through its pages.
            match (a.ui().menu_type(), c.proccess) {
                (-1, _) if seen.is_empty() => Buttons::TRIANGLE,
                (1, 1) => {
                    let l = c.list();
                    match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == 6) {
                        Some(r) if (l.select as usize) < r => Buttons::DOWN,
                        _ => Buttons::CROSS,
                    }
                }
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        s.take_events();
    }
    assert_eq!(seen, vec![0, 1], "the pages shown");
    let c = &area(&s).ui().ctrl;
    assert_eq!((area(&s).ui().menu_type(), c.proccess), (6, 1), "the list did not answer again");
}

/// What a frame of [`an_orb_in_a_dungeon_fight`] holds: the field camera
/// (`camID`, `tcam`'s type, eye, target, angles and distance), the room
/// built with its doors (their count, `doorFlag`) and `inBattle`.
#[derive(Debug, PartialEq)]
struct Held {
    cam: (i16, i32, [u32; 4], [u32; 4], [i16; 2], u32),
    room: Option<(usize, usize)>,
    doors: (usize, bool),
    fight: i32,
}

fn held(s: &Session) -> Held {
    let w = area(s).world();
    let (c, t) = (w.camera(), &w.camera().tcam);
    let (room, doors) = match w.place() {
        Place::Dungeon(d) => (d.room_at, (d.doors.len(), d.door.door_flag)),
        _ => (None, (0, false)),
    };
    Held { cam: (c.cam_id, t.kind, t.pos, t.view, t.deg, t.dist), room, doors, fight: w.combat().battle.in_battle }
}

/// Kite walks from a random dungeon's entrance until a fight starts, then
/// PERSONAL opens: with `orb` a Fairy's Orb is used from Items, else the
/// menu is shut. From the menu's close he backs away for 150 frames and
/// stands; what each of those 240 frames held, and whether the room was
/// deleted while the menu was up (`DUNGEON::ShowMap`).
fn fight_then_back(orb: bool) -> Option<(Vec<Held>, bool)> {
    let words = super::dressing::random_areas(0, &[0]).into_iter().next()?;
    let mut s = super::dressing::in_random_dungeon(0, words)?;
    s.console("god");
    let (mut pad, mut walker) = (Pad::default(), super::shrine::Walker::default());
    let (mut to, mut fight, mut opened, mut shut) = (None, false, false, None);
    let (mut seen, mut vanished, mut up) = (Vec::new(), false, false);
    for f in 0..4000u64 {
        if to.is_none()
            && let Stage::Area(a) = &s.stage
            && let Place::Dungeon(d) = a.world().place()
        {
            to = Some((0, d.floors[0].down));
        }
        fight |= to.is_some() && area(&s).world().combat().battle.in_battle != 0;
        let r = match (to, fight, shut) {
            (Some(t), false, _) => {
                let r = walker.step(&s, t, f).expect("out of the dungeon before a fight");
                walker.put(&mut s);
                r
            }
            (_, true, None) => {
                let a = area(&s);
                let c = &a.ui().ctrl;
                if !opened {
                    // Kite's first item: one Fairy's Orb, in both runs.
                    let Stage::Area(a) = &mut s.stage else { unreachable!() };
                    let save = &mut a.world_mut().state_mut().save;
                    save.set_i16(offset::ITEM_LIST, 2);
                    save.set_u8(offset::ITEM_LIST + 2, 13);
                    save.set_u8(offset::ITEM_LIST + 3, 1);
                    opened = true;
                    raw(Buttons::TRIANGLE, 128)
                } else if !f.is_multiple_of(8) || a.world().state().save.u8(offset::ITEM_LIST + 3) == 0 {
                    raw(Buttons::NONE, 128)
                } else {
                    match (a.ui().menu_type(), c.proccess, orb) {
                        (_, _, false) => raw(Buttons::CIRCLE, 128),
                        (2, 1, true) if c.list().select < 1 => raw(Buttons::DOWN, 128),
                        _ => raw(Buttons::CROSS, 128),
                    }
                }
            }
            (_, true, Some(from)) => raw(Buttons::NONE, if f - from < 150 { 255 } else { 128 }),
            (None, false, _) => raw(Buttons::NONE, 128),
        };
        pad.read(&r);
        s.step(&pad);
        s.take_events();
        let a = area(&s);
        if let Place::Dungeon(d) = a.world().place() {
            vanished |= opened && shut.is_none() && d.room_at.is_none();
        }
        up |= opened && a.ui().menu_type() != -1;
        if up && shut.is_none() && a.ui().menu_type() == -1 {
            shut = Some(f + 1);
        }
        if shut.is_some() {
            seen.push(held(&s));
            if seen.len() == 240 {
                let used = a.world().state().save.u8(offset::ITEM_LIST + 3) == 0;
                assert_eq!(used, orb, "the orb used");
                return Some((seen, vanished));
            }
        }
    }
    panic!("no fight, or the menu never shut (orb {orb})");
}

/// A Fairy's Orb used as a fight starts in a random dungeon's room with
/// its portal: `DUNGEON::ShowMap` (gcmn 0x005cf260) builds the room under
/// Kite again with `SetRoom`, whose `SetDoor` (0x005c7c30) keeps the doors
/// shut while `ccCheckActiveObject(f, i)` finds a foe or a portal there.
/// Backing into the door he came through, Kite and the camera then do on
/// every frame what they do when the menu is only opened and shut.
#[test]
fn an_orb_in_a_dungeon_fight() {
    let Some((plain, shown)) = fight_then_back(false) else { return };
    let (used, vanished) = fight_then_back(true).expect("the second run");
    assert!(!shown && vanished, "the room deleted only by the orb");
    assert_eq!(plain[0].fight, 1, "the fight on as the menu shuts");
    assert!(!plain[0].doors.1 && plain[0].doors.0 > 0, "the doors shut: {:?}", plain[0]);
    // The walk reaches the door: a shut leaf pulls the camera in.
    let near = |h: &Held| {
        let (eye, at) = (h.cam.2.map(f32::from_bits), h.cam.3.map(f32::from_bits));
        (0..3).map(|i| (eye[i] - at[i]).powi(2)).sum::<f32>().sqrt()
    };
    assert!(plain.iter().any(|h| near(h) < 300.0), "the camera never met the door");
    // The camera first, then the rest.
    for same in [|a: &Held, b: &Held| a.cam == b.cam, |a: &Held, b: &Held| a == b] {
        if let Some(k) = (0..plain.len()).find(|&k| !same(&plain[k], &used[k])) {
            panic!("frame {k} after the menu:\nwithout {:?}\nwith    {:?}", plain[k], used[k]);
        }
    }
}

/// Issue #8: a Speed Charm (category 11, id 56) Kite uses on
/// himself from PERSONAL casts skill 177 through him: his act 18 (the
/// spell's cast) plays and the skill ends with it. Without his
/// `targetChar` the act never came and skill 177 held him still.
#[test]
fn a_speed_charm_on_kite_ends() {
    let Some((mut s, _)) = in_field() else { return };
    let kite = |s: &Session| {
        let c = area(s).world().combat();
        let ch = &c.scene.chars[c.kite?];
        Some((ch.skill_id, ch.spc_char.act_num))
    };
    let mut pad = Pad::default();
    let (mut given, mut cast_at, mut acts) = (false, None, Vec::new());
    for f in 0..3000u32 {
        let ready = match &s.stage {
            Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
            _ => false,
        };
        if ready && !given {
            let Stage::Area(a) = &mut s.stage else { unreachable!() };
            // Kite's only item: one Speed Charm.
            let save = &mut a.world_mut().state_mut().save;
            for k in 0..40 {
                let (id, cat, num) = if k == 0 { (56, 11, 1) } else { (-1, 0xff, 0) };
                save.set_i16(offset::ITEM_LIST + 4 * k, id);
                save.set_u8(offset::ITEM_LIST + 4 * k + 2, cat);
                save.set_u8(offset::ITEM_LIST + 4 * k + 3, num);
            }
            given = true;
        }
        let b = if !ready || !given || !f.is_multiple_of(8) || cast_at.is_some() {
            Buttons::NONE
        } else {
            let a = area(&s);
            let c = &a.ui().ctrl;
            // PERSONAL, Items, the charms' page, the charm, Kite.
            let page = a.ui().texts().items.pages.iter().position(|&p| p == 11).expect("a page of charms");
            match (a.ui().menu_type(), c.proccess) {
                (-1, _) => Buttons::TRIANGLE,
                (1, 1) if c.list().select < 1 => Buttons::DOWN,
                (5, 1) if c.lists[5].page as usize != page => Buttons::RIGHT,
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        s.take_events();
        let Some((sid, act)) = kite(&s) else { continue };
        if cast_at.is_none() && sid == 177 {
            cast_at = Some(f);
        }
        if let Some(u) = cast_at {
            acts.push(act);
            if f > u + 300 {
                break;
            }
        }
    }
    let u = cast_at.expect("the charm was never used");
    assert!(acts.contains(&18), "Kite never cast it (from frame {u})");
    assert_eq!(kite(&s).map(|k| k.0), Some(0), "skill 177 still on Kite 300 frames on");
}

/// The menus and the save of the town or the area `s` is in.
pub(super) fn menus(s: &Session) -> Option<&piney_fieldui::FieldUi> {
    match &s.stage {
        Stage::Area(a) => Some(a.ui()),
        Stage::World(w) => Some(w.ui()),
        _ => None,
    }
}

fn save_mut(s: &mut Session) -> Option<&mut piney_data::save::SaveData> {
    match &mut s.stage {
        Stage::Area(a) => Some(&mut a.world_mut().state_mut().save),
        Stage::World(w) => Some(&mut w.world_mut().state_mut().save),
        _ => None,
    }
}

/// Issue #11: the Book of Law (key item 60) read from PERSONAL's Key
/// Items once `ready`: `ccUseItemRequest` (gcmn 0x0057c02c) opens
/// `installWarnStr`'s three lines, 8 frames on `ccSeOnNote(196, 52)`, and
/// after OK the list answers again. What the window's first line and the
/// sounds showed.
fn read_the_book_of_law(s: &mut Session, ready: impl Fn(&Session) -> bool) -> (bool, bool) {
    let warn = piney_fieldui::tables::UseTexts::of(piney_data::volume::Volume::Inf).install_warn;
    let mut pad = Pad::default();
    let (mut given, mut shown, mut sound) = (false, false, false);
    for f in 0..3000u32 {
        let now = ready(s);
        if now && !given {
            let save = save_mut(s).expect("a town or an area");
            for id in 0..piney_fieldui::menus::keyitem::IMP_ITEMS {
                save.set_u8(offset::IMP_ITEM_LIST + id, u8::from(id == 60));
            }
            given = true;
        }
        let ui = menus(s).expect("a town or an area");
        let c = &ui.ctrl;
        shown |= c.msg.line(0).is_some_and(|l| warn[0] == l);
        if shown && ui.menu_type() == 6 && c.proccess == 1 {
            break;
        }
        let b = if !now || !given || !f.is_multiple_of(8) {
            Buttons::NONE
        } else {
            // PERSONAL, Key Items, the book, and OK past the window.
            match (ui.menu_type(), c.proccess) {
                (-1, _) if !shown => Buttons::TRIANGLE,
                (0 | 1, 1) => {
                    let l = c.list();
                    match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == 6) {
                        Some(r) if (l.select as usize) < r => Buttons::DOWN,
                        _ => Buttons::CROSS,
                    }
                }
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        sound |= s.take_events().iter().any(|e| matches!(e, Event::SeNote { n: 196, note: 52 }));
    }
    (shown, sound)
}

/// Issue #11 where the book is got, in Mac Anu: before, nothing was
/// carried out for an item used in a town, so no window and no sound.
#[test]
fn the_book_of_law_warns_in_town() {
    use crate::session::area15::{disc, hold, start};
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    let in_town = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    hold(&mut s, 128, 128, 900, in_town);
    assert_eq!(read_the_book_of_law(&mut s, in_town), (true, true), "(the window, the sound)");
}

/// Issue #11 in a field: the window came, the sound (a step the menu
/// handed the world, which played none) did not.
#[test]
fn the_book_of_law_warns_in_a_field() {
    let Some((mut s, _)) = in_field() else { return };
    let ready = |s: &Session| match &s.stage {
        Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
        _ => false,
    };
    assert_eq!(read_the_book_of_law(&mut s, ready), (true, true), "(the window, the sound)");
}

/// A book (category 12, row 0: physical attack +10) read from PERSONAL's
/// Items in Mac Anu: the use now runs on the town party's scene, and
/// Kite's record in the save carries the rise once the menu is shut.
#[test]
fn a_book_read_in_town_raises_the_stat() {
    use crate::session::area15::{disc, hold, start};
    use piney_battle::param::{SpcParam, elm};
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    let in_town = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    hold(&mut s, 128, 128, 900, in_town);
    let save = save_mut(&mut s).expect("the town");
    let before = SpcParam::from_save(save, 0).elm[elm::P_ATK];
    for k in 0..40 {
        let (id, cat, num) = if k == 0 { (0, 12, 1) } else { (-1, 0xff, 0) };
        save.set_i16(offset::ITEM_LIST + 4 * k, id);
        save.set_u8(offset::ITEM_LIST + 4 * k + 2, cat);
        save.set_u8(offset::ITEM_LIST + 4 * k + 3, num);
    }
    let mut pad = Pad::default();
    let mut opened = false;
    for f in 0..3000u32 {
        let used = save_mut(&mut s).is_some_and(|v| v.u8(offset::ITEM_LIST + 3) == 0);
        let ui = menus(&s).expect("the town");
        let c = &ui.ctrl;
        if used && ui.menu_type() == -1 {
            break;
        }
        let page = ui.texts().items.pages.iter().position(|&p| p == 12).expect("a page of books");
        let b = if !f.is_multiple_of(8) {
            Buttons::NONE
        } else {
            // PERSONAL, Items, the books' page, the book, OK past the
            // window (the menu then shuts).
            match (ui.menu_type(), c.proccess) {
                (-1, _) if !opened => Buttons::TRIANGLE,
                (-1, _) => Buttons::NONE,
                (_, _) if used => Buttons::CROSS,
                (0 | 1, 1) => {
                    let l = c.list();
                    match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == 5) {
                        Some(r) if (l.select as usize) < r => Buttons::DOWN,
                        _ => Buttons::CROSS,
                    }
                }
                (5, 1) if c.lists[5].page as usize != page => Buttons::RIGHT,
                _ => Buttons::CROSS,
            }
        };
        opened |= ui.menu_type() != -1;
        pad.read(&raw(b, 128));
        s.step(&pad);
        s.take_events();
    }
    for _ in 0..4 {
        pad.read(&raw(Buttons::NONE, 128));
        s.step(&pad);
        s.take_events();
    }
    let save = save_mut(&mut s).expect("the town");
    assert_eq!(save.u8(offset::ITEM_LIST + 3), 0, "the book was not used");
    assert_eq!(SpcParam::from_save(save, 0).elm[elm::P_ATK], before + 10, "Kite's physical attack");
}

/// A Ryu Book (key item 273 + `book`) read from PERSONAL's Key Items once
/// `ready`: `ccThBook` fades to black, plays the cover stream (112 +
/// `book`), and opens the book's pages once the stream ends. Whether the
/// cover played and the pages opened. In a town the cover's frame must
/// move on every game frame it plays (#21: it stood still every other
/// one, the town's frame between).
pub(super) fn read_a_ryu_book(s: &mut Session, book: usize, ready: impl Fn(&Session) -> bool) -> (bool, bool) {
    let mut pad = Pad::default();
    let (mut given, mut cover, mut pages) = (false, false, false);
    let (mut last, mut still) = (None, 0);
    for f in 0..6000u32 {
        let now = ready(s);
        if now && !given {
            let save = save_mut(s).expect("a town or an area");
            for id in 0..piney_fieldui::menus::keyitem::IMP_ITEMS {
                save.set_u8(offset::IMP_ITEM_LIST + id, u8::from(id == 273 + book));
            }
            // The books' page (3) is there only with the bracelet.
            save.set_u8(offset::PLCOL, 1);
            given = true;
        }
        cover |= match &s.stage {
            Stage::World(w) => w.streaming(),
            Stage::Area(a) => a.movie_playing(),
            _ => false,
        };
        if let Stage::World(w) = &s.stage {
            let now = w.stream_frame();
            if now.is_some() && now == last && now != Some(0) {
                still += 1;
            }
            last = now;
        }
        let ui = menus(s).expect("a town or an area");
        pages |= ui.ctrl.book.as_ref().is_some_and(|t| t.book.is_some());
        if pages {
            break;
        }
        let opened = ui.ctrl.book.is_some();
        let c = &ui.ctrl;
        let b = if !now || !given || opened || !f.is_multiple_of(8) {
            Buttons::NONE
        } else {
            // PERSONAL, Key Items, its page 3 (the books), the book.
            match (ui.menu_type(), c.proccess) {
                (-1, _) => Buttons::TRIANGLE,
                (6, 1) if c.list().page < 3 => Buttons::RIGHT,
                (0 | 1, 1) => {
                    let l = c.list();
                    match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == 6) {
                        Some(r) if (l.select as usize) < r => Buttons::DOWN,
                        _ => Buttons::CROSS,
                    }
                }
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        s.take_events();
    }
    assert_eq!(still, 0, "the cover stood still for {still} frames");
    (cover, pages)
}

/// Ryu Book I read in Mac Anu: its cover stream plays over the town and
/// the pages open after it. Before, nothing played the cover and the book
/// waited for it for good.
#[test]
fn a_ryu_book_opens_in_town() {
    use crate::session::area15::{disc, hold, start};
    let Some((iso, archive)) = disc() else { return };
    let mut s = start(&iso, &archive, None);
    let in_town = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    hold(&mut s, 128, 128, 900, in_town);
    assert_eq!(read_a_ryu_book(&mut s, 0, in_town), (true, true), "(the cover, the pages)");
}

/// Issue #23: Ryu Books IV to VIII open in Mac Anu and close again on
/// cancel. Their pages were not ported: the window stayed empty and,
/// with no page's keys running its opening wait down, cancel never
/// closed the book.
#[test]
fn the_later_ryu_books_open_and_close() {
    use crate::session::area15::{disc, hold, start};
    let Some((iso, archive)) = disc() else { return };
    for book in 3..8 {
        let mut s = start(&iso, &archive, None);
        let in_town = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
        hold(&mut s, 128, 128, 900, in_town);
        assert_eq!(read_a_ryu_book(&mut s, book, in_town), (true, true), "book {}: (the cover, the pages)", book + 1);
        let mut pad = Pad::default();
        let mut closed = None;
        for f in 0..600u32 {
            let open = menus(&s).is_some_and(|ui| ui.ctrl.book.is_some());
            if !open {
                closed = Some(f);
                break;
            }
            let b = if f % 10 == 9 { Buttons::CIRCLE } else { Buttons::NONE };
            pad.read(&raw(b, 128));
            s.step(&pad);
            s.take_events();
        }
        assert!(closed.is_some(), "book {} never closed", book + 1);
    }
}

/// In a field the Key Items refuse a Ryu Book (`ImportantItemMenu`'s
/// help 8 when `area` is not 0): no cover, no pages.
#[test]
fn a_ryu_book_is_refused_in_a_field() {
    let Some((mut s, _)) = in_field() else { return };
    let ready = |s: &Session| match &s.stage {
        Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
        _ => false,
    };
    assert_eq!(read_a_ryu_book(&mut s, 0, ready), (false, false), "(the cover, the pages)");
}
