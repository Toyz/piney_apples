//! Issue #65: Kite left standing in Carmina Gadelica. Nothing the town
//! draws piles up: after the scene settles, the frame's commands and the
//! GS's vertices stay within what the first idle stretch drew, on every
//! later volume, with the pens' grown Grunties, the race's mail and the
//! Event NPC's status set (the town at its fullest). The panic the player
//! met is a lost device (worklog 408), not a frame grown too big.

use super::*;

/// Carmina Gadelica, `game.town` 2.
const TOWN: u8 = 2;
/// The mail that tells of the Flag Race.
const RACE_MAIL: usize = 324;
/// ITEM COMPLETE's status: the Event NPC stands.
const ITEM_COMPLETE: usize = offset::EVENT_STATUS + 52;

/// A new game on `disc` logged in to Carmina Gadelica, its three pens
/// holding grown Grunties, the race's mail read and the Event NPC there.
fn in_carmina_gadelica(disc: &str) -> Option<Session> {
    let iso = crate::check_image(disc);
    if !iso.exists() {
        eprintln!("{disc}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    // growth[server]: a grown Grunty in each pen, the last raised grown.
    let growth = 0x2194 + 24 * usize::from(TOWN);
    for k in 0..3 {
        state.save.set_i16(growth + 0xe + 2 * k, 1);
    }
    state.save.set_i16(growth, 4);
    state.save.set_mail(RACE_MAIL, 2);
    state.save.set_u8(ITEM_COMPLETE, 1);
    state.save.set_u8(offset::LAST_TOWN, TOWN);
    let scene = piney_world::area::Scene::log_in(&mut state.save);
    Some(Session::in_world(iso, archive, None, state, None, scene, None).unwrap())
}

/// The most commands and vertices a stretch of `frames` idle frames drew.
fn most(s: &mut Session, gs: &mut piney_gs::Gs, frames: u32) -> (usize, usize) {
    let pad = Pad::default();
    (0..frames).fold((0, 0), |(cmds, verts), _| {
        let f = s.step(&pad);
        s.take_events();
        gs.set_overlay(Mode::archive(s));
        gs.render(&f);
        (cmds.max(f.cmds.len()), verts.max(gs.vertices()))
    })
}

#[test]
fn an_idle_town_draws_no_more_as_time_goes_on() {
    for disc in ["mutation", "outbreak", "quarantine"] {
        let Some(mut s) = in_carmina_gadelica(disc) else { continue };
        let mut gs = match piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())) {
            Ok(gs) => gs,
            Err(e) => return eprintln!("no GPU ({e}); skipped"),
        };
        // The log-in's fade and the town's set-up, then the airship's
        // first legs and the walking PCs' first rounds.
        most(&mut s, &mut gs, 600);
        assert!(Mode::title(&s).contains("Carmina Gadelica - play"), "{}", Mode::title(&s));
        // The airship's puffs come and go at random: a little slack. One
        // more sprite a frame would be 3000 commands and 18,000 vertices.
        let first = most(&mut s, &mut gs, 3000);
        let later = most(&mut s, &mut gs, 3000);
        eprintln!("{disc}: first {first:?}, later {later:?}");
        let grew = later.0 > first.0 + 10 || later.1 > first.1 + first.1 / 50;
        assert!(!grew, "{disc}: the frame grew: {first:?} then {later:?}");
        assert!(gs.vertices() > 50_000, "{disc}: the town drawn");
    }
}
