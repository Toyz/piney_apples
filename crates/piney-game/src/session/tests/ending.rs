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
    let (mut field, mut ended, mut rolled) = (None, None, 0u32);
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
            let empty = matches!(w.place(), piney_world::field::world::Place::Story(m)
                if (&**m as &dyn std::any::Any).is::<piney_world::story_map::ending::EndArea>());
            field = Some((f, w.world_man().event, empty));
        }
        if let Stage::Area(a) = &s.stage {
            rolled += u32::from(a.world().fx().ending_running());
        }
        if super::survey::event_flag(&mut s, 314).is_some_and(|x| x & 3 << 62 != 0) {
            ended = Some(f);
            break;
        }
    }
    assert_eq!(field.map(|(_, n, empty)| (n, empty)), Some((126, true)), "field 126: {}", Mode::title(&s));
    assert!(ended.is_some(), "event 314 did not end: {}", Mode::title(&s));
    println!("field 126 at {:?}, ended at {ended:?}, rolled {rolled} frames", field.map(|f| f.0));
    assert!(rolled > 11_000, "the staff roll ran {rolled} frames");
}

/// `ccSaveData` before event 314's field part, written to `PINEY_SAVE_OUT`
/// for piney-eemu's `ee_run` (the game run into field 126 for block 3's
/// `ending_kanji`): the story to 314, its blocks 0-2 played.
#[test]
#[ignore]
fn event_314_save_bytes() {
    let Ok(out) = std::env::var("PINEY_SAVE_OUT") else { return };
    let iso = crate::check_image("quarantine");
    if !iso.exists() {
        return;
    }
    let (mut state, _) = crate::start::state_at(&iso, 314).unwrap();
    let f = state.save.event_flag(314);
    state.save.set_event_flag(314, f | 0b111);
    std::fs::write(out, state.save.bytes()).unwrap();
}

/// Event 314 played under the pilot to the first frame of its staff roll
/// (`ending_kanji`'s `STFROLL_VOL4`), each frame handed to `each`.
fn play_to_roll(mut each: impl FnMut(&mut Session, Frame)) -> Option<(Session, Pad)> {
    let mut s = story_session_on("quarantine", 314, |_| {})?;
    let (mut pad, mut pilot) = (Pad::default(), super::survey::StoryPilot::default());
    for f in 0..30_000u64 {
        if let Stage::Area(a) = &s.stage
            && a.world().fx().ending_running()
        {
            return Some((s, pad));
        }
        let raw = pilot.next(&s, f);
        pad.read(&raw);
        let frame = s.step(&pad);
        each(&mut s, frame);
    }
    panic!("no staff roll: {}", Mode::title(&s));
}

/// The staff roll's beats to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/stfroll/shots), by the roll's frame: the
/// first logo, the credits with the ring, the comets, the book, the last
/// logos and the last frame.
#[test]
#[ignore]
fn staff_roll_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/stfroll/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some((mut s, mut pad)) = play_to_roll(|s, _| drop(s.take_events())) else { return };
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let beats = [160u32, 600, 3000, 8100, 8300, 9700, 9900, 10500, 11150, 11300, 11650, 11741];
    pad.read(&piney_input::Raw::default());
    for f in 1..=11_742u32 {
        let frame = s.step(&pad);
        s.take_events();
        if beats.contains(&f) {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/roll_{f:05}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}");
        }
    }
}

/// Event 314's staff roll heard through `crate::handle` from the story's
/// start: nothing over the first logo, then from `Init(1)` (its frame 320)
/// `ccBgmPlay(2)`'s `BGM.BIN` track 2 to the roll's end, whose `ccBgmStop`
/// stops it, with no sequence under it at any frame.
#[test]
fn the_field_staff_roll_plays_alone() {
    let mut audio: Option<piney_audio::Audio> = None;
    let mut hear = |s: &mut Session, rate: u32| {
        let a = audio.get_or_insert_with(|| piney_audio::Audio::headless(&s.iso).unwrap());
        crate::handle(s.take_events(), Some(a));
        a.frame();
        a.render(&mut vec![0i16; 1600 * rate as usize]);
    };
    let Some((mut s, mut pad)) = play_to_roll(|s, _| hear(s, Mode::frame_rate(s))) else { return };
    let audio = audio.take().unwrap();
    pad.read(&piney_input::Raw::default());
    let sequences = |a: &piney_audio::Audio| a.with_engine(|e| e.seq.each_ref().map(|q| q.playing()));
    let (mut f, mut streamed) = (0u32, 0u32);
    while let Stage::Area(a) = &s.stage
        && a.world().fx().ending_running()
    {
        let rate = Mode::frame_rate(&s);
        s.step(&pad);
        crate::handle(s.take_events(), Some(&audio));
        audio.frame();
        audio.render(&mut vec![0i16; 1600 * rate as usize]);
        f += 1;
        assert_eq!(sequences(&audio), [false; 3], "a sequence under the roll at its frame {f}");
        streamed += u32::from(audio.bgm_streaming());
    }
    println!("the roll ran {f} frames, BGM.BIN streamed {streamed}");
    assert_eq!((f, streamed), (11_742, 11_742 - 320), "the roll's frames and BGM.BIN's");
    assert!(!audio.bgm_streaming(), "the roll's music past its end");
}

/// Issue #75: the player's log replayed through the launcher from the
/// last fight's end: each change of the title's shape, the frame rate,
/// every music event, the sequencers and the BGM stream every 300 frames,
/// and shots at `PINEY_SHOT_AT` (frames, comma-separated) or every
/// `PINEY_EVERY` frames to `$PINEY_SHOTS` (/mnt/data/claude/scratch/i75/
/// replay). A diagnostic: `PINEY_PADLOG=FILE cargo test --release -p
/// piney-game issue_75_replay -- --ignored --nocapture`.
#[test]
#[ignore]
fn issue_75_replay() {
    let Some(log) = std::env::var_os("PINEY_PADLOG") else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i75/replay".into());
    std::fs::create_dir_all(&dir).unwrap();
    let from = std::env::var("PINEY_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(36_000u32);
    let shoot_at: Vec<u32> = std::env::var("PINEY_SHOT_AT")
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_default();
    let every = std::env::var("PINEY_EVERY").ok().and_then(|v| v.parse::<u32>().ok());
    let mut gs: Option<piney_gs::Gs> = None;
    let (mut last, mut buf) = (String::new(), vec![0i16; 2 * 1600]);
    crate::launcher::replay_pad_log(&log.into_string().unwrap(), |f, mode, events, audio, frame| {
        let rate = mode.frame_rate();
        audio.render(&mut buf[..1600 * rate.min(2) as usize]);
        if f < from {
            return;
        }
        let title = mode.title();
        let shape: String = title.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect();
        if shape != last {
            println!("frame {f}: rate {rate} | {title}");
            last = shape;
        }
        for e in events {
            if matches!(
                e,
                Event::DesktopBgm(_)
                    | Event::Bgm(_)
                    | Event::BgmStream(_)
                    | Event::BgmStreamStop
                    | Event::SqLoad(_)
                    | Event::SqPlay(_)
                    | Event::SqStop(_)
                    | Event::BgmCtrl(_)
                    | Event::HoldBgm
                    | Event::AllSoundOff
                    | Event::StreamBgm(_)
            ) {
                println!("frame {f}: {e:?} | {title}");
            }
        }
        if f.is_multiple_of(300) {
            let seq = audio.with_engine(|e| e.seq.each_ref().map(|q| q.playing()));
            let roll = mode.session().and_then(|s| match &s.stage {
                Stage::Desktop(d) => d.desktop().and_then(|d| d.staff_roll()).map(|r| (r.page, r.status, r.done)),
                _ => None,
            });
            println!(
                "frame {f}: rate {rate}, sequences {seq:?}, bgm stream {}, roll {roll:?} | {title}",
                audio.bgm_streaming()
            );
        }
        if shoot_at.contains(&f) || every.is_some_and(|n| f.is_multiple_of(n)) {
            let overlay = mode.archive();
            let Some(s) = mode.session() else { return };
            let archive = s.archive.clone();
            let gs = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap());
            gs.set_overlay(overlay);
            gs.render(frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/f{f:05}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("  {path}");
        }
    });
}
