//! The voice lines against the game: `voice_ee_fixture.txt` (`python3
//! tools/sound_ee.py voice-fixture ELF`: what `ccEvVoiceRequest`,
//! `ccEvVoiceStop` and `evVoicePlay` send to SEWORDS.IRX for every row of the
//! volume 1 tables, the field groups and the slot cases), `voice_fixture.txt`
//! (`python3 tools/iopemu.py voice-fixture ELF ISO`: SEWORDS.IRX itself
//! streaming lines, the samples heard), and a headless render of event 1's
//! message 1. The disc parts are skipped without the image (PINEY_ISO).

use std::path::PathBuf;

use piney_audio::Audio;
use piney_audio::driver::{Command, Driver};
use piney_audio::seword::{self, Word};
use piney_data::iso::Iso;
use piney_data::sound::INF;
use piney_data::sound::voice;

/// The IOP's name for a file as a path on the disc, as piney-gen writes
/// the tables: `cdrom0:\VOICE_E\EVVOL1_E.BIN` is `VOICE_E/EVVOL1_E.BIN`.
fn disc_path(name: &str) -> String {
    let p = name.split_once(':').map_or(name, |(_, p)| p);
    p.trim_start_matches('\\').replace('\\', "/")
}

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

fn words(cmds: &[Command]) -> Vec<String> {
    cmds.iter()
        .map(|c| match c {
            Command::Voice(v) => format!("seword 80e0 {} {} 0 {:x} {}", v.ofs, v.size, v.vol, v.file),
            Command::VoiceStop => "seword 120".to_string(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn the_driver_asks_what_the_game_asks() {
    let mut iso = iso_path().map(|p| Iso::open(p).unwrap());
    let (mut rows, mut elsewhere, mut other, mut field) = (0, 0, 0, 0);
    let (mut spoken, mut outside) = (0, 0);
    for line in include_str!("voice_ee_fixture.txt").lines().filter(|l| !l.starts_with('#')) {
        let (head, want) = line.split_once(" | ").unwrap_or((line.trim_end_matches(" |"), ""));
        // The fixture names files as the IOP opens them.
        let want: Vec<String> = want
            .split("; ")
            .filter(|s| !s.is_empty())
            .map(|w| match w.rsplit_once(' ') {
                Some((head, name)) if w.starts_with("seword 80e0") => format!("{head} {}", disc_path(name)),
                _ => w.to_string(),
            })
            .collect();
        let mut f = head.splitn(3, ' ');
        let (lang, parody, steps) = (f.next().unwrap(), f.next().unwrap(), f.next().unwrap());
        let mut d = Driver::new();
        d.voice_english = lang == "1";
        d.parody = parody == "1";
        let mut out = Vec::new();
        // A word whose row falls outside its character's table: the game
        // reads the memory next to it; the port plays nothing.
        let mut out_of_table = false;
        let mut events = Vec::new();
        for step in steps.split('+') {
            let w: Vec<&str> = step.split_whitespace().collect();
            match w[0] {
                "voice" => {
                    let (e, m) = (w[1].parse().unwrap(), w[2].parse().unwrap());
                    events.push(e);
                    d.voice_request(e, m);
                }
                "stop" => d.voice_stop(),
                "pginit" => d.pg_bgm_init(),
                "play" => d.frame(&INF, &mut out),
                "lang" => d.voice_english = w[1] == "1",
                "parody" => d.parody = w[1] == "1",
                "words" => {
                    let n = |i: usize| w[i].parse::<i32>().unwrap();
                    let (c, ty, sid, ev, bit) = (n(1), n(2), n(3), n(4), n(5));
                    let skill = piney_data::tables::voice::of(piney_data::volume::Volume::Inf).skill;
                    let len = usize::try_from(c)
                        .ok()
                        .and_then(|k| skill.get(k))
                        .map_or(0, |v| if d.voice_english { v.en.rows.len() } else { v.jp.rows.len() });
                    let row = piney_audio::driver::skill_row(c as i16, sid as i16, bit == 1);
                    let taken = ev == 0 && ty & 5 != 0 && sid < 304;
                    if taken
                        && usize::try_from(c).is_ok_and(|k| k < 18)
                        && usize::try_from(row).map_or(true, |r| r >= len)
                    {
                        out_of_table = true;
                    }
                    d.words_play(ev != 0, ty as u32, c as i16, sid, bit == 1);
                }
                "skill" => d.skill_voice_play(&mut out),
                s => panic!("step {s}"),
            }
        }
        let got = words(&out);
        if steps.contains("words") || steps == "skill" {
            if out_of_table && got.is_empty() {
                outside += 1;
                continue;
            }
            assert_eq!(got, want, "{head}");
            spoken += 1;
            continue;
        }
        if events.iter().any(|&e| e < -1) {
            // ccVoiceRequest's groups (gcmn.prg's tables).
            assert_eq!(got, want, "{head}");
            field += 1;
            continue;
        }
        if events.len() == 1 && !(0..100).contains(&events[0]) {
            // Volumes 2-4: a request for a file this disc does not have,
            // which SEWORDS cannot open. The port sends what the game sends,
            // or nothing where the game reads past a placeholder table (the
            // port stops at a table's end).
            assert!(got == want || got.is_empty(), "{head}: {got:?} / {want:?}");
            for w in &want {
                let name = w.rsplit(' ').next().unwrap();
                if let Some(iso) = &mut iso {
                    assert!(iso.find(name).is_err(), "{head}: {name} is on the disc");
                }
            }
            elsewhere += 1;
            continue;
        }
        assert_eq!(got, want, "{head}");
        if events.len() == 1 && steps.split('+').count() == 2 {
            rows += 1;
        } else {
            other += 1;
        }
    }
    // The tables' 1,292 rows (1,152 lines, 140 of -1) with Parody Mode off
    // and on, 10 events without a table in each language and event 1 once
    // more; 19 events outside 0-99 in each language; 15 slot cases and 3 of
    // `ccPgBgmInit`'s; and
    // ccVoiceRequest's groups: the 20 tables' 554 rows in both languages,
    // with and without Parody Mode (2,216), 7 groups with no case, and 3
    // frames mixing a group with an event or a stop.
    assert_eq!((rows, elsewhere, other, field), (2 * 1292 + 20 + 1, 38, 18, 2216 + 7 + 3));
    // ccWordsPlay and skillVoicePlay: each character's 304 ids in English
    // and every seventh in Japanese, and 10 gate and queue cases; the words
    // whose row the rule puts outside the table are the port's silence.
    assert_eq!(spoken + outside, 18 * (304 + 44) + 10, "spoken {spoken} outside {outside}");
    eprintln!("skill words: {spoken} as the game, {outside} outside their tables");
}

#[test]
fn the_stream_is_what_seword_plays() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let mut iso = Iso::open(path).unwrap();
    let mut n = 0;
    for line in include_str!("voice_fixture.txt").lines().filter(|l| !l.starts_with('#')) {
        let (head, want) = line.split_once(" | ").unwrap();
        let f: Vec<&str> = head.split_whitespace().collect();
        let (file, ofs, size) = (f[1], f[2].parse::<u64>().unwrap(), f[3].parse::<i32>().unwrap());
        let data = voice::read_voice(&mut iso, file, ofs, seword::read_len(size)).unwrap();
        let mut w = Word::open(data, size).expect(head);
        let mut heard = Vec::new();
        while let Some((l, r)) = w.next_sample() {
            assert_eq!(l, r, "{head}");
            heard.extend(l.to_le_bytes());
        }
        let got = format!("{} {:016x} 6fff 6fff 0 0", heard.len() / 2, fnv(&heard));
        assert_eq!(got, want, "{head}");
        n += 1;
    }
    assert_eq!(n, 115);
}

/// `x * v >> 15` after clamping to 16 bits, as the SPU2 model scales.
fn scale(x: i32, v: i32) -> i32 {
    (x.clamp(-0x8000, 0x7fff) * v) >> 15
}

#[test]
fn event_1_message_1_renders_headless() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    // Event 1's registration line in English:
    // VOICE_E/EVVOL1_E.BIN at 159,744, 441,604 bytes.
    let mut iso = Iso::open(&path).unwrap();
    let line = voice::voice_pcm(&voice::read_voice(&mut iso, "VOICE_E/EVVOL1_E.BIN", 159_744, 441_604).unwrap());
    assert_eq!(line.len(), 220_802);
    let heard = 4096 * (441_604usize.div_ceil(8192) - 1);
    assert_eq!(heard, 217_088);

    let audio = Audio::headless(&path).unwrap();
    audio.set_voice_options(true, false, [0; 21]);
    assert!(audio.voice(1, 1));
    assert!(!audio.voice_playing(), "sent on the next frame");
    audio.frame();
    assert!(audio.voice_playing());
    assert_eq!(audio.with_engine(|e| e.spu.input_vol), (0x6fff, 0x6fff));

    let mut out = vec![0i16; 2 * (heard + 4800)];
    audio.render(&mut out);
    // Core 0's input at 0x6fff, core 0's master and core 1's master at
    // 0x3fff (0x7ffe as a fixed volume); nothing else sounds.
    for (i, f) in out.as_chunks::<2>().0.iter().enumerate() {
        let want = if i < heard { scale(scale(scale(line[i] as i32, 0x6fff), 0x7ffe), 0x7ffe) as i16 } else { 0 };
        assert_eq!(*f, [want, want], "sample {i}");
    }
    let loud = out[..2 * heard].iter().filter(|s| s.unsigned_abs() > 1000).count();
    assert!(loud > 20_000, "{loud}");
    assert!(!audio.voice_playing());
    assert_eq!(audio.with_engine(|e| e.spu.input_vol), (0, 0));
}

#[test]
fn a_line_plays_until_stopped_and_refuses_another_meanwhile() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let mut iso = Iso::open(&path).unwrap();
    let first = voice::voice_pcm(&voice::read_voice(&mut iso, "VOICE/EVVOL1.BIN", 0, 248_216).unwrap());
    let audio = Audio::headless(&path).unwrap();
    audio.set_voice_options(false, false, [0; 21]);
    assert!(audio.voice(1, 0));
    audio.frame();
    let mut a = vec![0i16; 2 * 10_000];
    audio.render(&mut a);
    // The next message's line is refused: channel 0 is still streaming.
    assert!(audio.voice(1, 2));
    audio.frame();
    let mut b = vec![0i16; 2 * 10_000];
    audio.render(&mut b);
    let want = |i: usize| scale(scale(scale(first[i] as i32, 0x6fff), 0x7ffe), 0x7ffe) as i16;
    for (i, f) in a.as_chunks::<2>().0.iter().chain(b.as_chunks::<2>().0).enumerate() {
        assert_eq!(f[0], want(i), "sample {i}");
    }
    // The confirm button: ccEvVoiceStop, then silence.
    audio.voice_stop();
    assert!(audio.voice_playing());
    audio.frame();
    assert!(!audio.voice_playing());
    let mut c = vec![0i16; 2 * 1000];
    audio.render(&mut c);
    assert!(c.iter().all(|&s| s == 0));
    // Parody Mode: no voice for events 0-49, but 50-99 keep theirs.
    audio.set_voice_options(true, true, [0; 21]);
    assert!(!audio.voice(1, 1));
    assert!(audio.voice(50, 0));
}

#[test]
fn every_event_line_is_in_its_file() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    assert_eq!(event_lines(&path, piney_data::volume::Volume::Inf), 1152);
}

/// Mutation's own event lines (its second hundred's main and side events,
/// Japanese and English), each inside its file on Mutation's disc.
#[test]
fn every_mutation_event_line_is_in_its_file() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !path.exists() {
        eprintln!("skipped: no disc image");
        return;
    }
    let n = event_lines(&path, piney_data::volume::Volume::Mut);
    assert!(n > 1000, "{n} lines");
}

/// Every line of the disc's event voice tables checked against its file:
/// sector aligned, long enough, inside the file. The number of lines.
fn event_lines(path: &std::path::Path, volume: piney_data::volume::Volume) -> usize {
    let mut iso = Iso::open(path).unwrap();
    let voice = piney_data::tables::voice::of(volume);
    let mut n = 0;
    for b in voice.events.iter().flat_map(|s| [s.main, s.side, s.parody]).flatten() {
        let tables = [Some(b.jp), b.en];
        for (lang, table) in tables.iter().enumerate() {
            let Some(table) = table else { continue };
            let name = if lang == 0 {
                piney_data::tables::voice::FILES[b.file as usize]
            } else {
                voice.files_e[b.file as usize]
            };
            // A hundred the disc does not voice has no file (Infection's
            // executable has Mutation's side events' rows, whose file is on
            // Mutation's disc).
            let Ok(entry) = iso.find(name) else { continue };
            let size = entry.size as i64;
            for rows in table.iter().flatten() {
                for r in rows.iter().filter(|r| r.ofs != -1) {
                    assert!(r.ofs as i64 % 2048 == 0 && r.siz > 0, "{name} {r:?}");
                    // SEWORDS reads the last packet to the sector's end.
                    assert!(r.ofs as i64 + (r.siz as i64 + 2047) / 2048 * 2048 <= size, "{name} {r:?}");
                    assert!(r.siz > 32_768, "{name} {r:?}: a line this short would replay its start");
                    n += 1;
                }
            }
        }
    }
    n
}

/// Mutation's own event lines play from its disc: the first of its
/// events (100-199) with a line, in English and in Japanese, sent on the
/// next frame and playing.
#[test]
fn a_mutation_event_line_plays() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !path.exists() {
        eprintln!("skipped: no disc image");
        return;
    }
    let audio = Audio::headless(&path).unwrap();
    for english in [true, false] {
        audio.set_voice_options(english, false, [0; 21]);
        let asked = (100..200).flat_map(|e| (0..40).map(move |m| (e, m))).find(|&(e, m)| audio.voice(e, m));
        let (event, msg) = asked.expect("a Mutation event with a line");
        audio.frame();
        assert!(audio.voice_playing(), "event {event} message {msg}");
        audio.voice_stop();
        audio.frame();
    }
}

/// Every line of the field's voices (`ccVoiceRequest`'s groups and their
/// alternatives), the food's and the skill words', in both languages,
/// checked against its file: sector aligned, inside the file. The number
/// of lines.
fn field_lines(path: &std::path::Path, volume: piney_data::volume::Volume) -> usize {
    use piney_data::tables::voice::{FILES, FOOD, VoiceData};
    let mut iso = Iso::open(path).unwrap();
    let voice = piney_data::tables::voice::of(volume);
    let mut tables: Vec<(&str, &[VoiceData])> = Vec::new();
    for g in voice.field.iter().map(|g| (&g.table, g.alt)).chain([(&*FOOD, None)]) {
        let (t, alt) = g;
        tables.push((FILES[t.file as usize], t.jp));
        tables.push((voice.files_e[t.file as usize], t.en));
        if let Some(a) = alt {
            tables.push((voice.files_e[a.file as usize], a.rows));
        }
    }
    for s in voice.skill {
        tables.push((s.jp.file, s.jp.rows));
        tables.push((s.en.file, s.en.rows));
    }
    let mut n = 0;
    for (name, rows) in tables {
        // Fidchell's file (group -40) is on neither Infection's disc nor
        // Mutation's.
        let Ok(entry) = iso.find(name) else {
            assert!(name.ends_with("/BOSSTALK.BIN"), "{name} is not on the disc");
            continue;
        };
        let size = entry.size as i64;
        // The skill words' tables end in a (0, 0) row.
        for r in rows.iter().filter(|r| r.ofs != -1 && r.siz != 0) {
            assert!(r.ofs as i64 % 2048 == 0 && r.siz > 0, "{name} {r:?}");
            assert!(r.ofs as i64 + (r.siz as i64 + 2047) / 2048 * 2048 <= size, "{name} {r:?}");
            n += 1;
        }
    }
    n
}

#[test]
fn every_field_line_is_in_its_file() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let n = field_lines(&path, piney_data::volume::Volume::Inf);
    assert!(n > 7000, "{n} lines");
}

#[test]
fn every_mutation_field_line_is_in_its_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !path.exists() {
        eprintln!("skipped: no disc image");
        return;
    }
    let n = field_lines(&path, piney_data::volume::Volume::Mut);
    assert!(n > 7000, "{n} lines");
}

/// Mutation's English party lines for Mia: while `talkNum[1]` is set,
/// messages 3-5 of the party group (-30) come from `MIAE.BIN`, row
/// `msg - 3`; otherwise, and in Japanese, from the party file.
#[test]
fn mutation_mia_speaks_from_miae_once_talked_to() {
    use piney_data::volume::Volume;
    let voice = piney_data::tables::voice::of(Volume::Mut);
    let tables = piney_data::sound::tables_of(Volume::Mut);
    let heard = |english: bool, talk: i8, group: i32, msg: i32| {
        let mut d = Driver::new();
        d.set_voice(voice);
        d.voice_english = english;
        d.talk_num[1] = talk;
        assert!(d.voice_request(group, msg), "group {group} message {msg}");
        let mut out = Vec::new();
        d.frame(tables, &mut out);
        match out.as_slice() {
            [Command::Voice(v)] => (v.file, v.ofs),
            other => panic!("{other:?}"),
        }
    };
    let alt = voice.field.iter().find(|g| g.group == -30).unwrap().alt.unwrap();
    assert_eq!(heard(true, 1, -30, 3), ("VOICE_E/MIAE.BIN", alt.rows[0].ofs));
    assert_eq!(heard(true, 0, -30, 3).0, "VOICE_E/PARTY_E.BIN");
    assert_eq!(heard(false, 1, -30, 3).0, "VOICE/PARTY.BIN");
    // Outside the range, the party file.
    assert_eq!(heard(true, 1, -30, 6).0, "VOICE_E/PARTY_E.BIN");
    // Talk (-31) and presents (-32) read the same table further on.
    assert_eq!(heard(true, 1, -31, 2), ("VOICE_E/MIAE.BIN", alt.rows[3].ofs));
    assert_eq!(heard(true, 1, -32, 9), ("VOICE_E/MIAE.BIN", alt.rows[9].ofs));
}
