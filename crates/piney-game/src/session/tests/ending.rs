//! Quarantine's ending (event 314, "Ending"): its desktop block's `area
//! 126`, `scene 1 0 126` and `mode 5` into field 126, the ending's empty
//! map, where block 3 hides Kite and runs the staff roll.

use super::*;

/// Event 314 from its story start on the desktop under the pilot, through
/// field 126 (`eventAreaNumber` 126 from Quarantine's instruction 118, the
/// map with nothing in it, the party's panels out, Kite hidden) and back to
/// the desktop for the event's end.
#[test]
fn event_314_plays_through_field_126() {
    let Some(mut s) = story_session_on("quarantine", 314, |_| {}) else { return };
    let (mut pad, mut pilot) = (Pad::default(), super::survey::StoryPilot::default());
    let (mut field, mut ended) = (None, None);
    for f in 0..60_000u64 {
        let raw = pilot.next(&s, f);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && a.world().scene().field == 126
            && field.is_none()
        {
            let w = a.world();
            let empty = matches!(w.place(), piney_world::field_world::Place::Story(m)
                if m.as_any().is::<piney_world::evarea_end::EndArea>());
            field = Some((f, w.world_man().event, empty));
        }
        if super::survey::event_flag(&mut s, 314).is_some_and(|x| x & 3 << 62 != 0) {
            ended = Some(f);
            break;
        }
    }
    assert_eq!(field.map(|(_, n, empty)| (n, empty)), Some((126, true)), "field 126: {}", Mode::title(&s));
    assert!(ended.is_some(), "event 314 did not end: {}", Mode::title(&s));
}
