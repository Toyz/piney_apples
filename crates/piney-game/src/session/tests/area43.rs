//! Mutation's area 43, `EVENTAREA03`, on a new game (the area not marked
//! on the server): the camera's fly-over, Kite's line, and the way back to
//! Dun Loireag.

use piney_input::{Buttons, Raw};

use super::*;
use crate::session::area15::{hold, playing, start};

/// The area's words go to the story map, not a generated field; camera 3
/// flies over it until Kite speaks (frame 140); OK closes his line, and
/// the party is sent to town 1.
#[test]
fn area_43_flies_over_and_sends_the_party_back() {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !iso.exists() {
        return;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut s = start(&iso, &archive, Some(43));
    hold(&mut s, 128, 128, 400, |s| playing(s).is_some());
    let w = playing(&s).unwrap();
    assert_eq!((w.scene().area, w.scene().field), (1, 43));
    let a = w.place().story::<piney_world::evarea03::Area43>().expect("EVENTAREA03");
    assert!(!a.flag, "area 43 marked on a new game");
    let calls = |s: &Session| match &s.stage {
        Stage::Area(a) => a.calls().iter().map(|(_, c)| c.clone()).collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    hold(&mut s, 128, 128, 400, |s| calls(s).iter().any(|c| c == "message_open story"));
    let w = playing(&s).unwrap();
    assert_eq!(w.camera().cam_id, piney_world::camera::id::EVENT);
    assert!(calls(&s).iter().any(|c| c == "menu_ban true"));
    assert_eq!(w.place().story::<piney_world::evarea03::Area43>().unwrap().cnt, 141);
    // OK until the line closes and the town comes.
    let mut pad = Pad::default();
    for f in 0..600u32 {
        if let Stage::World(w) = &s.stage {
            assert_eq!(w.town_number(), 1, "{}", Mode::title(&s));
            return;
        }
        let b = if f % 24 == 0 { Buttons::CROSS } else { Buttons::NONE };
        pad.read(&Raw { buttons: b, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
    panic!("not back in town: {}", Mode::title(&s));
}
