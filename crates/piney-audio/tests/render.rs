//! Headless renders: a sound effect, a few seconds of the desktop's music,
//! of the jukebox and of the title's, and a `BGM.BIN` stream must come out non-silent and
//! sane - no clipping runs, no DC, sound where the game plays it and quiet
//! where it does not. Skipped without the disc image; set PINEY_ISO to point
//! at it elsewhere. Set PINEY_AUDIO_WAV to a directory to keep the renders.

use std::path::PathBuf;

use piney_audio::driver::{AreaMusic, BgmWorld, SqContext, setup_context};
use piney_audio::{Audio, wav};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

/// Render `seconds`, running a game frame every 800 samples.
fn run(audio: &Audio, seconds: f64) -> Vec<i16> {
    let mut pcm = vec![0i16; (seconds * 48_000.0) as usize * 2];
    for chunk in pcm.chunks_mut(1600) {
        audio.frame();
        audio.render(chunk);
    }
    pcm
}

struct Stats {
    rms: f64,
    peak: i32,
    clipped: usize,
    dc: f64,
}

fn stats(pcm: &[i16]) -> Stats {
    let n = pcm.len().max(1) as f64;
    let sum: f64 = pcm.iter().map(|&s| s as f64).sum();
    let sq: f64 = pcm.iter().map(|&s| (s as f64).powi(2)).sum();
    Stats {
        rms: (sq / n).sqrt(),
        peak: pcm.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0),
        clipped: pcm.iter().filter(|&&s| s == i16::MAX || s == i16::MIN).count(),
        dc: sum / n,
    }
}

fn keep(name: &str, pcm: &[i16]) {
    if let Some(dir) = std::env::var_os("PINEY_AUDIO_WAV") {
        let path = PathBuf::from(dir).join(name);
        wav::write(&path, pcm, 48_000, 2).unwrap();
    }
}

fn sane(what: &str, s: &Stats) {
    assert!(s.rms > 100.0, "{what}: silent (rms {:.1})", s.rms);
    assert!(s.peak < 32767 || s.clipped < 64, "{what}: clips {} samples", s.clipped);
    assert!(s.dc.abs() < s.rms * 0.5, "{what}: DC {:.1} against rms {:.1}", s.dc, s.rms);
}

#[test]
fn a_sound_effect() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    // Silence before anything plays.
    let quiet = run(&audio, 0.2);
    assert_eq!(stats(&quiet).peak, 0);
    // SE 4, the jukebox's selection sound.
    audio.se_on(4);
    let pcm = run(&audio, 1.0);
    keep("se4.wav", &pcm);
    let s = stats(&pcm);
    sane("SE 4", &s);
    // It ends: the last tenth of 3 s is quiet.
    let tail = run(&audio, 3.0);
    let t = stats(&tail[tail.len() * 9 / 10..]);
    assert!(t.rms < s.rms / 20.0, "SE 4 still sounds: {:.1} against {:.1}", t.rms, s.rms);
    // Every sound effect sounds, except those for which MODHSYN itself
    // starts no voice (their note is outside every split of the program):
    // the fixture's first tick is empty for them.
    let none: Vec<usize> = include_str!("se_fixture.txt")
        .lines()
        .filter_map(|l| l.strip_prefix("se "))
        .filter_map(|l| l.strip_suffix(" 256 -1 0 ").or_else(|| l.strip_suffix(" 256 -1 0")))
        .map(|n| n.parse().unwrap())
        .collect();
    let mut silent = Vec::new();
    for n in 0..audio.tables().se.len() {
        let a = Audio::headless(&iso).unwrap();
        a.se_on(n);
        if stats(&run(&a, 0.5)).peak == 0 {
            silent.push(n);
        }
    }
    assert_eq!(silent, none, "silent sound effects");
    assert_eq!(none, [11, 12, 14, 37, 54, 83, 211]);
}

/// The RMS of the left and the right channel.
fn sides(pcm: &[i16]) -> (f64, f64) {
    let rms = |k: usize| {
        let s: f64 = pcm.iter().skip(k).step_by(2).map(|&x| (x as f64).powi(2)).sum();
        (s / (pcm.len() / 2).max(1) as f64).sqrt()
    };
    (rms(0), rms(1))
}

#[test]
fn a_positioned_sound_effect() {
    use piney_audio::Listener;
    use piney_audio::se3d;
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let k = |x: f32| x.to_bits();
    // A camera at the origin looking along +x; SE 4 (velocity 112, decay
    // 256: heard to 5,500 away).
    let cam = Listener { pos: [0, 0, 0, k(1.0)], view: [k(100.0), 0, 0, k(1.0)], kind: 0 };
    let play = |pos: [u32; 4]| {
        let a = Audio::headless(&iso).unwrap();
        a.se_on_3d(4, Some(&cam), &pos, None);
        run(&a, 0.5)
    };
    // A quarter turn to the left sounds on the left, to the right on the
    // right; ahead in the middle.
    let left = play([k(0.01), k(500.0), 0, 0]);
    keep("se4_left.wav", &left);
    sane("SE 4 to the left", &stats(&left));
    let (l, r) = sides(&left);
    assert!(l > 20.0 * r.max(1.0), "left: {l:.1} {r:.1}");
    let (l2, r2) = sides(&play([k(0.01), k(-500.0), 0, 0]));
    assert!(r2 > 20.0 * l2.max(1.0), "right: {l2:.1} {r2:.1}");
    let (la, ra) = sides(&play([k(500.0), 0, 0, 0]));
    assert!((la - ra).abs() < 0.2 * la, "ahead: {la:.1} {ra:.1}");
    // Farther is quieter (velocity 78 at 4,800 against 112 near), and out of
    // reach silent.
    let far = sides(&play([k(4800.0), 0, 0, 0]));
    assert!(far.0 < 0.9 * la && far.0 > 0.2 * la, "far: {:.1} near: {la:.1}", far.0);
    assert_eq!(stats(&play([k(6000.0), 0, 0, 0])).peak, 0);
    // No camera: full velocity, centred.
    let a = Audio::headless(&iso).unwrap();
    a.se_on_3d(4, None, &[0; 4], None);
    let (ln, rn) = sides(&run(&a, 0.5));
    assert!((ln - rn).abs() < 0.2 * ln && ln > la * 0.9, "no camera: {ln:.1} {rn:.1}");
    // A footstep note: Kite on a ground with no row of its own.
    let a = Audio::headless(&iso).unwrap();
    let step = se3d::spc_note(0, 0, 0).unwrap();
    a.note_se(step, Some(&cam), &[k(300.0), 0, 0, 0]);
    sane("Kite's footstep", &stats(&run(&a, 0.5)));
    // A loop takes slot 0 and sounds until its note off.
    let a = Audio::headless(&iso).unwrap();
    let id = a.se_on_3d_loop(230, Some(&cam), &[k(300.0), 0, 0, 0]);
    assert_eq!(id, 0);
    assert_eq!(a.se_on_3d_loop(230, Some(&cam), &[k(300.0), 0, 0, 0]), 1);
    let on = stats(&run(&a, 1.0));
    sane("the loop", &on);
    a.se_off_loop(230, 0);
    a.se_off_loop(230, 1);
    let off = run(&a, 3.0);
    let t = stats(&off[off.len() * 9 / 10..]);
    assert!(t.rms < on.rms / 20.0, "the loop still sounds: {:.1} against {:.1}", t.rms, on.rms);
    assert_eq!(a.with_engine(|e| e.driver.loop_id), [-1; 8]);
}

#[test]
fn the_desktop_music() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    // A new save's desktop plays "Original 01" (Wave 50); BGM 01 (Wave 0)
    // is a busier track.
    for no in [50usize, 0, 19] {
        audio.desktop_bgm(no);
        let pcm = run(&audio, 6.0);
        keep(&format!("desktop{no}.wav"), &pcm);
        sane(&format!("Wave[{no}]"), &stats(&pcm));
    }
    // The jukebox: fade out, load, play.
    audio.bgm(46);
    let pcm = run(&audio, 5.0);
    keep("jukebox46.wav", &pcm);
    sane("Wave[46] from the jukebox", &stats(&pcm[48_000..]));
    // bgmVol 0 silences the music; only the reverb's tail is left, dying.
    audio.set_volumes(256, 256, 0);
    let tail: Vec<f64> = (0..8).map(|_| stats(&run(&audio, 0.5)).rms).collect();
    eprintln!("bgmVol 0: {tail:?}");
    assert!(tail[7] < 20.0 && tail[7] < tail[0], "bgmVol 0 still plays: {tail:?}");
    audio.bgm_stop();
}

#[test]
fn a_streamed_track() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    audio.bgm_stream(0).unwrap();
    let pcm = run(&audio, 3.0);
    keep("stream0.wav", &pcm);
    sane("BGM.BIN track 0", &stats(&pcm));
}

#[test]
fn the_title_music() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    // ccSetupDemo: ccAllSoundOff, ccSndSQLoad(7); ccThDemo: ccSqStop(0),
    // then (past the logos) ccSetMainVol(mainVol), ccSqPlay(0).
    audio.all_sound_off();
    audio.sq_load(SqContext::Title);
    audio.sq_stop(0);
    let quiet = run(&audio, 0.2);
    assert_eq!(stats(&quiet).peak, 0, "the bank loads silent");
    audio.set_main_volume(256);
    audio.sq_play(0);
    let pcm = run(&audio, 6.0);
    keep("title.wav", &pcm);
    let s = stats(&pcm);
    sane("the title", &s);
    // New Game: ccSqFade(0, 0, 8, 3) - on the title ccSceneFade takes port
    // 1 to 0 over 8 frames and stops the sequence.
    audio.sq_fade(0, 0, 8, 3);
    let _ = run(&audio, 10.0 / 60.0);
    assert!(!audio.with_engine(|e| e.seq[0].playing()), "the fade stops sequence 0");
    assert_eq!(audio.with_engine(|e| e.driver.port_vol[1]), 0);
    let tail: Vec<f64> = (0..6).map(|_| stats(&run(&audio, 0.5)).rms).collect();
    assert!(tail[5] < 20.0 && tail[5] < s.rms / 50.0, "the title music goes on: {tail:?}");
}

#[test]
fn mac_anu_music() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    // ccSetupGameCtrl: ccSoundFadeOut, ccAllSoundOff, ccSndSQLoad(2) with
    // Mac Anu's row (town 0, no crisis); the fade in's end: ccSndBgmCtrl's
    // town case, sequence 0 as ccSqPlay(0).
    audio.all_sound_off();
    audio.sq_load(SqContext::Town { row: 0 });
    let quiet = run(&audio, 0.2);
    assert_eq!(stats(&quiet).peak, 0, "the bank loads silent");
    audio.sq_play(0);
    let pcm = run(&audio, 6.0);
    keep("mac_anu.wav", &pcm);
    sane("Mac Anu", &stats(&pcm));
    // The crisis town's row has a bank of its own.
    let crisis = Audio::headless(&iso).unwrap();
    crisis.all_sound_off();
    crisis.sq_load(SqContext::Town { row: 5 });
    assert!(crisis.with_engine(|e| e.driver.sq_num) >= 1);
}

#[test]
fn field_music_and_battle() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    // Story area 14, "Bursting Passed Over Aqua Field" on Delta: field type
    // 10, weather 0, a new scene. ccSetupGameCtrl: ccAllSoundOff,
    // ccSndSQLoad(3), gameStart before the fade in, ccSndBgmCtrl after it.
    let area =
        AreaMusic { area: 1, scene_replaced: true, field: 14, field_type: 10, special_room: -1, ..Default::default() };
    let ctx = setup_context(&area).unwrap();
    assert_eq!(ctx, SqContext::Field { field_type: 10, bg: 0, piros: false });
    audio.all_sound_off();
    audio.sq_load(ctx);
    let quiet = run(&audio, 0.2);
    assert_eq!(stats(&quiet).peak, 0, "the bank loads silent");
    assert_eq!(audio.with_engine(|e| e.driver.sq_num), 2, "the field music and its battle arrangement");
    audio.game_start();
    let plan = audio.bgm_ctrl(BgmWorld { scene_replaced: true, ..Default::default() });
    assert_eq!(plan.play, &[0]);
    let pcm = run(&audio, 6.0);
    keep("field_aqua.wav", &pcm);
    sane("Aqua Field", &stats(&pcm));
    // A battle: bgmChange has SNDBASE start sequence 1 where sequence 0 is
    // and fades 0 out over 30 frames; its end brings 0 back.
    audio.set_battle(true, false);
    let pcm = run(&audio, 1.0);
    audio.with_engine(|e| {
        assert!(e.seq[1].playing() && !e.seq[0].playing(), "the battle music replaces the field's");
        assert_eq!(e.driver.port_vol[1], 0);
        assert_eq!(e.driver.port_vol[2], e.driver.sqtbl[1].vol);
    });
    keep("field_aqua_battle.wav", &pcm);
    sane("Aqua Field's battle", &stats(&pcm[pcm.len() / 2..]));
    audio.set_battle(false, false);
    let _ = run(&audio, 1.0);
    audio.with_engine(|e| assert!(e.seq[0].playing() && !e.seq[1].playing(), "the field music is back"));
}

/// `ccAllSoundOff` with a field's music playing, as the desktop's set-up
/// sends it before its event pass (event 4's Skeith streams): SNDBASE's
/// `allSoundOff` (0x140) stops every sequencer, not only the voices
/// sounding, so the area's music does not play on under the streams.
#[test]
fn all_sound_off_stops_the_music() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    let area =
        AreaMusic { area: 1, scene_replaced: true, field: 14, field_type: 10, special_room: -1, ..Default::default() };
    audio.all_sound_off();
    audio.sq_load(setup_context(&area).unwrap());
    run(&audio, 0.2);
    audio.game_start();
    audio.bgm_ctrl(BgmWorld { scene_replaced: true, ..Default::default() });
    sane("Aqua Field", &stats(&run(&audio, 2.0)));
    audio.game_interrupt();
    audio.all_sound_off();
    let pcm = run(&audio, 2.0);
    audio.with_engine(|e| assert!(e.seq.iter().all(|s| !s.playing()), "a sequencer still plays"));
    // The voices' releases end within the first half second.
    assert_eq!(stats(&pcm[pcm.len() / 2..]).peak, 0, "the music plays on");
}

/// One town to the next as the session sends it (the console's `town`, a
/// gate between towns): Mac Anu's music playing, then `ChangeRequest`'s
/// `gameInterrupt`, the next set-up's `ccAllSoundOff` a few frames on, its
/// `ccSndSQLoad(2)` with Dun Loireag's row, `gameStart` and `ccSndBgmCtrl`:
/// Dun Loireag's music plays.
#[test]
fn town_to_town_music() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let audio = Audio::headless(&iso).unwrap();
    audio.all_sound_off();
    audio.sq_load(SqContext::Town { row: 0 });
    run(&audio, 0.2);
    audio.game_start();
    audio.bgm_ctrl(BgmWorld { town: 0, ..Default::default() });
    sane("Mac Anu", &stats(&run(&audio, 2.0)));
    audio.game_interrupt();
    run(&audio, 10.0 / 60.0);
    audio.all_sound_off();
    run(&audio, 4.0 / 60.0);
    audio.sq_load(SqContext::Town { row: 1 });
    audio.game_start();
    run(&audio, 11.0 / 60.0);
    let plan = audio.bgm_ctrl(BgmWorld { town: 1, ..Default::default() });
    eprintln!("plan {plan:?}");
    let pcm = run(&audio, 4.0);
    keep("dun_loireag_after_mac_anu.wav", &pcm);
    audio.with_engine(|e| {
        eprintln!("ports {:?} playing {:?}", e.driver.port_vol, e.seq.iter().map(|s| s.playing()).collect::<Vec<_>>())
    });
    sane("Dun Loireag after Mac Anu", &stats(&pcm[pcm.len() / 2..]));
}

/// Every Root Town's music: `ccSndSQLoad(2)` with each town's row (0 Mac
/// Anu .. 4 Lia Fail) loads a bank, and `ccSndBgmCtrl` sounds it.
#[test]
fn every_town_has_music() {
    let Some(iso) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    for town in 0..5 {
        let audio = Audio::headless(&iso).unwrap();
        audio.all_sound_off();
        audio.sq_load(SqContext::Town { row: town });
        run(&audio, 0.2);
        assert!(audio.with_engine(|e| e.driver.sq_num) >= 1, "town {town}: no bank");
        audio.game_start();
        audio.bgm_ctrl(BgmWorld { town: town.into(), ..Default::default() });
        sane(&format!("town {town}"), &stats(&run(&audio, 3.0)));
    }
}

/// Every volume's disc sounds: a sound effect from the common bank, and a
/// town's music (Mac Anu's bank and `ccSndBgmCtrl`), headless.
#[test]
fn every_volume_sounds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    for disc in
        ["infection/infection.iso", "mutation/mutation.iso", "outbreak/outbreak.iso", "quarantine/quarantine.iso"]
    {
        let iso = root.join(disc);
        if !iso.exists() {
            eprintln!("skipped: no {disc}");
            continue;
        }
        let audio = Audio::headless(&iso).unwrap();
        audio.se_on(4);
        let se = stats(&run(&audio, 1.0));
        eprintln!("{disc}: se rms {:.0}", se.rms);
        sane(&format!("{disc} se"), &se);
        let audio = Audio::headless(&iso).unwrap();
        audio.all_sound_off();
        audio.sq_load(SqContext::Town { row: 0 });
        run(&audio, 0.2);
        audio.game_start();
        audio.bgm_ctrl(BgmWorld { town: 0, ..Default::default() });
        let music = stats(&run(&audio, 3.0));
        eprintln!("{disc}: town rms {:.0}", music.rms);
        sane(&format!("{disc} town music"), &music);
    }
}
