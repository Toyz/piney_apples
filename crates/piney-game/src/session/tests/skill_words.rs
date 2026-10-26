//! BUGS.md, the members' skill words not heard: what `skillVoicePlay`
//! (main 0x0017e350) does with each party character's every skill. A word
//! that holds the queue (a NULL file, a row of -1) is tried every frame
//! and silences every later word, Kite's too.

use piney_data::volume::Volume;

/// What the sound task makes of one queued word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Word {
    /// A line of the character's file is sent.
    Line,
    /// A name no file has is sent: nothing is heard, the queue empties.
    Unheard,
    /// Nothing is sent and the queue holds.
    Held,
}

fn word(vol: Volume, v: &piney_data::tables::voice::Voice, ty: i32, c: i16, sid: i32, en: bool) -> Word {
    let Some(sv) = v.skill.get(c as usize) else { return Word::Held };
    let w = if en { &sv.en } else { &sv.jp };
    let Some(file) = w.file else { return Word::Held };
    let at = i64::from(w.row0) + i64::from(piney_audio::driver::skill_row(vol, c, sid as i16, ty & 1 != 0, en));
    match usize::try_from(at).ok().and_then(|k| v.skill_memory.get(k)) {
        Some(r) if r.ofs != -1 && !file.is_empty() => Word::Line,
        Some(r) if r.ofs != -1 => Word::Unheard,
        _ => Word::Held,
    }
}

/// Every `charTbl` character `ccWordsPlay` keeps (`base->type & 5`) and
/// every named skill, in both languages: a line, but for Mutation's
/// Tsukasa (18) and Subaru (19), whose file pointers are NULL (the game
/// holds the queue), and its Sora (20), whose name lies past the arrays
/// and is no file (the game empties the queue, nothing heard). Outbreak
/// and Quarantine give all three their files.
#[test]
fn every_party_characters_skill_words() {
    for vol in [Volume::Inf, Volume::Mut, Volume::Out, Volume::Qua] {
        let v = piney_data::tables::voice::of(vol);
        let t = piney_battle::tables::Tables::of(vol);
        let party: Vec<i16> =
            (t.chars.iter().enumerate()).filter(|(_, c)| c.param.base.ty & 5 != 0).map(|(i, _)| i as i16).collect();
        assert_eq!(party.len(), if vol == Volume::Inf { 18 } else { 21 }, "{vol:?}");
        for &c in &party {
            let want = match (vol, c) {
                (Volume::Mut, 18 | 19) => Word::Held,
                (Volume::Mut, 20) => Word::Unheard,
                _ => Word::Line,
            };
            for en in [false, true] {
                for sid in 6..304 {
                    let Some(p) = t.skill(sid).filter(|p| !p.name.is_empty()) else { continue };
                    assert_eq!(
                        word(vol, v, p.ty, c, sid, en),
                        want,
                        "{vol:?}: character {c}, skill {sid}, English {en}"
                    );
                }
            }
        }
    }
}
