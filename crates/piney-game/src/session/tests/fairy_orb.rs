//! A Fairy's Orb (TOOL 2) used from PERSONAL in a random field: its
//! `ccUseItemRequest` steps (the info window, then `WORLD_MAN::ShowMap`
//! once a menu frame until it is done, 20 frames at least) put the magic
//! portals on the field's map (`WORLD::ShowMap`: `mapFlag`).

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
