//! Quarantine's town 5 (`town06`, `ROOTTOWN06`): event 301 played from its
//! start under the story autopilot, with no aid. Event 301's block 3 sets
//! `eventStatus[51]`, so the gate's town 4 lands in town 5, where blocks 7
//! and 8 play; block 8's `scene` takes the party on to Lia Fail.
//! docs/engine/root-towns.md.

use super::survey::{StoryPilot, event_flag};
use super::*;

#[test]
fn event_301_goes_through_town_5() {
    let Some(mut s) = story_session_on("quarantine", 301, |_| {}) else { return };
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let mut towns: Vec<i32> = Vec::new();
    let mut ended = None;
    for f in 0..20_000u64 {
        let raw = pilot.next(&s, f);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::World(w) = &s.stage {
            let t = w.world().town().base.no;
            if towns.last() != Some(&t) {
                towns.push(t);
            }
        }
        if event_flag(&mut s, 301).is_some_and(|x| x & 3 << 62 != 0) {
            ended = Some(f);
            break;
        }
    }
    eprintln!("towns {towns:?}, event 301 ended at {ended:?}");
    assert!(ended.is_some(), "event 301 did not end: towns {towns:?}, {}", Mode::title(&s));
    // Fort Ouph (Elk), town 5 (Helba's warp), Lia Fail (the meeting).
    let at = |t: i32| towns.iter().position(|&x| x == t);
    assert!(at(3) < at(5) && at(5) < at(4), "towns {towns:?}");
}
