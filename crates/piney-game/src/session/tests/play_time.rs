//! `ccAddPlayTime`: every frame of the main loop adds the frame rate to
//! `saveData.playTime`.

use super::*;

/// Mac Anu from event 11's start: 300 frames standing add 300 frames'
/// vertical blanks to the save's play time.
#[test]
fn the_world_counts_play_time() {
    let Some(mut s) = story_session(11) else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let time = |s: &Session| match &s.stage {
        Stage::World(w) => Some(w.world().state().save.play_time()),
        _ => None,
    };
    let start = time(&s).expect("not in Mac Anu");
    let mut blanks = 0;
    for _ in 0..300 {
        pad.read(&still);
        s.step(&pad);
        s.take_events();
        blanks += Mode::frame_rate(&s) as i32;
    }
    assert_eq!(time(&s), Some(start + blanks));
}
