//! Issue #76: an Antidote or a Restorative used from PERSONAL in a field:
//! its `ccUseItemRequest` (gcmn 0x0057aa80) asks `ccItemSkillRequest(plw,
//! plw, 178 or 179, 0)`, stype 1, whose `_ccSkillRequest` calls
//! `effSkillStart(plw, sid, 1, 0)` (gcmn 0x00572b58) and so `ccSeOn3D(104,
//! pos)` (main 0x001d3970).

use piney_input::{Buttons, Raw};

use super::ride::{area, in_field, raw};
use super::*;

/// Kite's first item, `id` of category `cat`, used on him from
/// PERSONAL, Items; the sounds asked for from the use on, and the energy
/// heard over 120 frames with every event routed but SE `drop`.
fn sounds_of_use(cat: u8, id: i16, drop: Option<usize>) -> Option<(Vec<usize>, i64)> {
    let (mut s, _) = in_field()?;
    let audio = piney_audio::Audio::headless(&s.iso).ok()?;
    let mut buf = vec![0i16; 2 * 800];
    let mut energy = 0i64;
    let mut pad = Pad::default();
    let (mut given, mut used, mut heard) = (false, None, Vec::new());
    for f in 0..2000u32 {
        let ready = match &s.stage {
            Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
            _ => false,
        };
        if ready && !given {
            let Stage::Area(a) = &mut s.stage else { unreachable!() };
            let save = &mut a.world_mut().state_mut().save;
            save.set_i16(offset::ITEM_LIST, id);
            save.set_u8(offset::ITEM_LIST + 2, cat);
            save.set_u8(offset::ITEM_LIST + 3, 1);
            given = true;
        }
        let b = if !ready || !given || !f.is_multiple_of(8) || used.is_some() {
            Buttons::NONE
        } else {
            let a = area(&s);
            let c = &a.ui().ctrl;
            match (a.ui().menu_type(), c.proccess) {
                (-1, _) => Buttons::TRIANGLE,
                (1, 1) if c.list().select < 1 => Buttons::DOWN,
                _ => Buttons::CROSS,
            }
        };
        pad.read(&raw(b, 128));
        s.step(&pad);
        let events = s.take_events();
        let routed: Vec<Event> =
            events.iter().filter(|e| !matches!(e, Event::Se3d { n, .. } if Some(*n) == drop)).cloned().collect();
        crate::handle(routed, Some(&audio));
        audio.frame();
        audio.render(&mut buf);
        if given && used.is_none() {
            let a = area(&s);
            if a.world().state().save.u8(offset::ITEM_LIST + 3) == 0 {
                used = Some(f);
            }
        }
        if used.is_some() {
            energy += buf.iter().map(|&x| i64::from(x).abs()).sum::<i64>();
            heard.extend(events.iter().filter_map(|e| match e {
                Event::Se3d { n, .. } => Some(*n),
                Event::Se(n) => usize::try_from(*n).ok(),
                _ => None,
            }));
        }
        if used.is_some_and(|u| f > u + 120) {
            break;
        }
    }
    used.map(|_| (heard, energy))
}

/// Issue #76: the Antidote's and the Restorative's one sound, the skill
/// start's 104 (`effCure` and `effSanity` make none, main 0x001cef50 and
/// 0x001cf0d0), asked for and heard over the field.
#[test]
fn the_cures_sound_the_skill_start() {
    for (id, what) in [(3, "antidote"), (4, "restorative")] {
        let Some((heard, energy)) = sounds_of_use(10, id, None) else { return };
        let Some((_, without)) = sounds_of_use(10, id, Some(104)) else { return };
        assert!(heard.contains(&104), "{what}: no effSkillStart sound: {heard:?}");
        assert!(energy > without, "{what}: 104 not heard ({energy} against {without})");
    }
}

/// The same from power-on: a player's log (`PINEY_PADLOG`, row
/// `PINEY_LAUNCHER_ROW`) replayed through the launcher to `PINEY_FROM`, in a
/// field or dungeon, then an Antidote given and used there; the sound's
/// energy over the 120 frames after, with every event routed from power-on,
/// and again with SE 104 held back.
/// Whether the item list holds item `id` of category `cat`.
fn holds(save: &piney_data::save::SaveData, cat: u8, id: i16) -> bool {
    (0..40).any(|k| {
        let at = offset::ITEM_LIST + 4 * k;
        save.i16(at) == id && save.u8(at + 2) == cat && save.u8(at + 3) > 0
    })
}

#[test]
#[ignore]
fn an_antidote_sounds_from_power_on() {
    let Some(log) = std::env::var_os("PINEY_PADLOG") else { return };
    let log = log.into_string().unwrap();
    let from = std::env::var("PINEY_FROM").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(7400);
    // PINEY_ITEM=ID of category 10 (3 the Antidote), PINEY_DROP=SE.
    let item = std::env::var("PINEY_ITEM").ok().and_then(|v| v.parse::<i16>().ok()).unwrap_or(3);
    let held = std::env::var("PINEY_DROP").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(104);
    let run = |drop: Option<usize>| {
        use std::cell::Cell;
        let (given, used) = (Cell::new(false), Cell::new(None::<u32>));
        let (mut energy, mut heard) = (0i64, Vec::new());
        let mut buf = vec![0i16; 2 * 800];
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let pad_of = |f: u32, mode: &mut dyn Mode, raw: Raw| {
            if f < from || used.get().is_some_and(|u| f > u + 120) {
                return raw;
            }
            let Some(s) = mode.session() else { return raw };
            let Stage::Area(a) = &mut s.stage else { return raw };
            if !given.get() {
                let save = &mut a.world_mut().state_mut().save;
                save.set_i16(offset::ITEM_LIST, item);
                save.set_u8(offset::ITEM_LIST + 2, 10);
                save.set_u8(offset::ITEM_LIST + 3, 1);
                given.set(true);
            }
            if used.get().is_some() || !f.is_multiple_of(8) {
                return still;
            }
            let c = &a.ui().ctrl;
            // PERSONAL (2) at its Items row, the list (5) at its top.
            let b = match (a.ui().menu_type(), c.list().select) {
                (-1, _) => Buttons::TRIANGLE,
                (2, s) if s < 1 => Buttons::DOWN,
                (2, s) if s > 1 => Buttons::UP,
                (5, s) if s > 0 => Buttons::UP,
                _ => Buttons::CROSS,
            };
            Raw { buttons: b, ..still }
        };
        let routed = |e: &Event| !matches!(e, Event::Se3d { n, .. } if Some(*n) == drop);
        crate::launcher::replay_pad_log_with(&log, pad_of, routed, |f, mode, events, audio, _| {
            audio.render(&mut buf);
            if given.get() && used.get().is_none() {
                let gone = mode.session().is_some_and(|s| match &s.stage {
                    Stage::Area(a) => !holds(&a.world().state().save, 10, item),
                    _ => false,
                });
                if gone {
                    used.set(Some(f));
                }
            }
            if used.get().is_some_and(|u| f <= u + 120) {
                energy += buf.iter().map(|&x| i64::from(x).abs()).sum::<i64>();
                heard.extend(events.iter().filter_map(|e| match e {
                    Event::Se3d { n, .. } => Some(*n),
                    Event::Se(n) => usize::try_from(*n).ok(),
                    _ => None,
                }));
            }
        });
        (used.get(), heard, energy)
    };
    let (used, heard, energy) = run(None);
    let (_, _, without) = run(Some(held));
    println!("used at {used:?}: {heard:?} energy {energy}, without {held} {without}");
}
