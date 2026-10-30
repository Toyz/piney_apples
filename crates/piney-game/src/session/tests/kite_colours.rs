//! Kite's colours in a new game's town. Before the Data Drain bracelet
//! (`saveData.plcol` 0) `ccPlayer` swaps `CLT_ctu1body` for
//! `CLT_ctu1bodyc1` and the HUD shows face 18, both grey; on a later volume
//! the boot's `ccStartEvent(volumeNum, 0)` brings Infection's events
//! forward, event 11's `data_drain` among them, so a New Game there starts
//! with the bracelet's colours. The towns built straight from
//! [`crate::world::new_game_state`] once left the boot out.

use super::*;

/// `playTime` (+0x8400), which the desktop counts from its first frame.
const PLAY_TIME: std::ops::Range<usize> = 0x8400..0x8404;

const DISCS: [&str; 4] = ["infection", "mutation", "outbreak", "quarantine"];

/// The save a player's New Game hands the desktop: power on, the title,
/// NEWGAME (the event task left out, so no pass has run on it).
fn title_new_game(disc: &str) -> Option<SaveData> {
    let mut s = session_on(disc, false, Some(empty_card()))?;
    let mut pad = Pad::default();
    new_game(&mut s, &mut pad);
    Some(s.save_mut().expect("the desktop's save").clone())
}

/// [`crate::world::new_game_state`] is the title's New Game, byte for
/// byte but the play time the desktop has begun to count, on every volume:
/// on the later ones with the bracelet.
#[test]
fn the_built_new_game_is_the_title_s() {
    for disc in DISCS {
        let Some(title) = title_new_game(disc) else { continue };
        let iso = crate::check_image(disc);
        let built = crate::world::new_game_state(&mut Iso::open(&iso).unwrap()).unwrap().save;
        let differ: Vec<usize> = (0..title.bytes().len())
            .filter(|&i| !PLAY_TIME.contains(&i) && title.bytes()[i] != built.bytes()[i])
            .take(16)
            .collect();
        assert!(differ.is_empty(), "{disc}: the built save differs from the title's at {differ:#x?}");
        assert_eq!(built.u8(offset::PLCOL), u8::from(disc != "infection"), "{disc}: plcol");
    }
}

/// In Mac Anu on a new game, Kite's body palette and his HUD face are the
/// bracelet's on the later volumes and the plain ones on Infection.
#[test]
fn kite_wears_the_bracelet_s_colours_in_a_later_volume_s_town() {
    for disc in DISCS {
        let iso = crate::check_image(disc);
        if !iso.exists() {
            eprintln!("{disc}.iso not present; skipped");
            continue;
        }
        let mut d = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut d).unwrap();
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let s = Session::in_world(iso, archive, None, state, None, scene, None).unwrap();
        let Stage::World(w) = &s.stage else { panic!("{disc}: not in the town") };
        let bracelet = w.world().state().save.u8(offset::PLCOL) != 0;
        assert_eq!(bracelet, disc != "infection", "{disc}: plcol");
        assert_eq!(w.world().kite().swaps(bracelet).is_empty(), bracelet, "{disc}: CLT_ctu1bodyc1");
        assert_eq!(w.ui().ctrl.face_tex[0], if bracelet { 0 } else { 18 }, "{disc}: Kite's face");
    }
}
