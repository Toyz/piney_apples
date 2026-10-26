//! The skill words against the game: `words_fixture_{inf,mut,out,qua}.txt`
//! (`python3 tools/sound_ee.py words-fixture ELF`) is what `ccWordsPlay`
//! and `skillVoicePlay` send to SEWORDS.IRX for every `charTbl` character
//! and skill id in both languages, each followed by one of Kite's words.
//! The driver must send the same, and hold its queue where the game's does.

use piney_audio::driver::{Command, Driver};
use piney_data::volume::Volume;

/// The IOP's name for a file as a path on the disc.
fn disc_path(name: &str) -> String {
    let p = name.split_once(':').map_or(name, |(_, p)| p);
    p.trim_start_matches('\\').replace('\\', "/")
}

/// One fixture's lines against a driver with the volume's tables.
fn check(volume: Volume, fixture: &str) {
    let tables = piney_data::tables::voice::of(volume);
    let mut names = std::collections::HashMap::new();
    let kite_bit = |lang: &str| {
        let head = format!("{lang} 0 10 ");
        let line = fixture.lines().find(|l| l.starts_with(&head)).expect("Kite's skill 10");
        line.split(' ').nth(3) == Some("1")
    };
    let mut n = 0;
    for line in fixture.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<&str> = line.split(' ').collect();
        if w[0] == "file" {
            names.insert((w[1], w[2]), disc_path(w.get(3).copied().unwrap_or("")));
            continue;
        }
        let (lang, c, sid, bit): (&str, i16, i32, bool) =
            (w[0], w[1].parse().unwrap(), w[2].parse().unwrap(), w[3] == "1");
        let mut d = Driver::new();
        d.volume = volume;
        d.set_voice(tables);
        d.voice_english = lang == "1";
        let mut out = Vec::new();
        assert_eq!(d.words_play(false, 6, c, sid, bit), 0);
        d.skill_voice_play(&mut out);
        assert_eq!(d.words_play(false, 7, 0, 10, kite_bit(lang)), 0);
        d.skill_voice_play(&mut out);
        let got: Vec<(String, i32, i32)> = out
            .iter()
            .filter_map(|c| match c {
                Command::Voice(v) => Some((v.file.to_string(), v.ofs, v.size)),
                _ => None,
            })
            .collect();
        if w[4] == "held" {
            assert!(got.is_empty(), "{volume:?} {line}: {got:?}");
        } else {
            let want = (names[&(w[0], w[1])].clone(), w[4].parse().unwrap(), w[5].parse().unwrap());
            assert_eq!(got.len(), 2, "{volume:?} {line}: {got:?}");
            assert_eq!(got[0], want, "{volume:?} {line}");
        }
        n += 1;
    }
    assert!(n > 10_000, "{volume:?}: {n} words");
}

#[test]
fn infections_words_are_the_games() {
    check(Volume::Inf, include_str!("words_fixture_inf.txt"));
}

/// Tsukasa (18) and Subaru (19) hold the queue (NULL files); Sora's (20)
/// name lies past the arrays and is empty.
#[test]
fn mutations_words_are_the_games() {
    check(Volume::Mut, include_str!("words_fixture_mut.txt"));
}

#[test]
fn outbreaks_words_are_the_games() {
    check(Volume::Out, include_str!("words_fixture_out.txt"));
}

#[test]
fn quarantines_words_are_the_games() {
    check(Volume::Qua, include_str!("words_fixture_qua.txt"));
}
