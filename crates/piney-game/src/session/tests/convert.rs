//! Issue #55: the later parts' CONVERT, the previous part's clear data
//! carried into a new game. Mutation's title reads Infection's directory on
//! the same card (`LoadInfoPrevReq`), takes only a save that cleared
//! Infection, and leaves for the desktop on it after `ccStartEventConvert`
//! and `ConvGame`.

use std::path::Path;

use piney_data::save::{EVENT_DONE, SaveData, by_id, offset};
use piney_data::volume::Volume;
use piney_desktop::card::{FilesCard, MemoryCard, PortState};
use piney_desktop::savesys::{LOAD_PREV_QUESTION, LOAD_SELECT, SaveSys, code, load_done};
use piney_input::Buttons;

use super::*;

/// `save` into slot `file` of `volume`'s directory on the card `dir`, as
/// that volume's save menu writes it (the directory made when missing,
/// the index record, then the slot file).
pub(super) fn save_to(dir: &Path, volume: Volume, file: i32, save: &SaveData) {
    let mut card = FilesCard::slot1(volume, dir);
    if card.check_port(0) != PortState::Ready {
        assert!(card.make_dir(0));
    }
    let mut sys = SaveSys::new(volume);
    sys.info.copy_from_slice(&card.read_index(0, volume.number()).unwrap());
    (sys.file_num, sys.proccess, sys.result) = (file, 12, code::WORKING);
    sys.main_proccess(&mut card, save);
    assert_eq!(sys.result, 0x101c, "{volume}'s save");
}

/// `volume`'s title on the card `dir` from power-on to CONVERT's list of
/// MEMORY CARD slot 1 (the menu's fourth item, OK, then slot 1), the
/// desktop's event scripts run with `scripts`. None without the disc.
pub(super) fn convert_list(dir: &Path, volume: Volume, scripts: bool) -> Option<(Session, Pad)> {
    let disc = piney_data::pack::disc_name(volume).trim_end_matches(".disc");
    let mut s = session_on(disc, scripts, Some(dir.to_path_buf()))?;
    let mut pad = Pad::default();
    to_menu(&mut s, &mut pad, drop);
    run(&mut s, &mut pad, 0..40, &[]);
    for _ in 0..4 {
        if title(&s).unwrap().demo.opening().nut_cur_no == 3 {
            break;
        }
        tap(&mut s, &mut pad, Buttons::DOWN);
        run(&mut s, &mut pad, 0..8, &[]);
    }
    assert_eq!(title(&s).unwrap().demo.opening().nut_cur_no, 3, "CONVERT is the fourth item");
    tap(&mut s, &mut pad, Buttons::CROSS);
    until(&mut s, &mut pad, 100, |t| t.demo.opening().next_sw != 0);
    tap(&mut s, &mut pad, Buttons::CROSS);
    until(&mut s, &mut pad, 20, |t| t.demo.save_sys().result == LOAD_SELECT);
    Some((s, pad))
}

/// From CONVERT's list with the cursor on a cleared save: OK, YES (NO is
/// chosen first), OK on "Load complete.", then out of the title. The
/// desktop's save as it starts.
pub(super) fn convert_chosen(s: &mut Session, pad: &mut Pad) -> SaveData {
    tap(s, pad, Buttons::CROSS);
    until(s, pad, 20, |t| t.demo.save_sys().result == LOAD_PREV_QUESTION);
    tap(s, pad, Buttons::UP);
    tap(s, pad, Buttons::CROSS);
    until(s, pad, 20, |t| t.demo.save_sys().result == load_done(t.demo.save_sys().volume));
    tap(s, pad, Buttons::CROSS);
    let mut n = 0;
    while title(s).is_some() {
        press(s, pad, Buttons::NONE);
        n += 1;
        assert!(n < 400, "CONVERT does not leave the title: {}", Mode::title(s));
    }
    assert!(matches!(s.stage, Stage::Desktop(_)), "not on the desktop: {}", Mode::title(s));
    s.save_mut().expect("the desktop's save").clone()
}

/// The player's case: Infection's saves on the card (one cleared, one
/// not) with or without a Mutation save there too. CONVERT lists
/// Infection's (not "There is no .hack//MUTATION saved data."), refuses
/// the save that has not cleared Infection ("No Data Flag"), and carries
/// the cleared one: the name, Kite's level, a party member's own level,
/// the clear flag and play time kept; a character not in the party back
/// to Mutation's `charTbl` with empty lists; event 100 done and
/// `newGameFlag` 2. Its extension is the title's new game's.
#[test]
fn convert_carries_infection_s_clear_data_into_mutation() {
    for with_mutation_save in [false, true] {
        let dir = empty_card();
        let mut cleared = SaveState::fresh().save;
        cleared.bytes_mut()[..5].copy_from_slice(b"Haru\0");
        cleared.set_u8(offset::CLEAR_FLAG, 1);
        cleared.set_i32(offset::PLAY_TIME, 4_000_000);
        cleared.set_i16(by_id::spc_param(0) + offset::SPC_LEVEL, 42);
        cleared.set_i32(offset::PARTY_MEMBER_FLAG, 1 << 1);
        cleared.set_i16(by_id::spc_param(1) + offset::SPC_LEVEL, 33);
        cleared.set_i16(by_id::spc_param(2) + offset::SPC_LEVEL, 77);
        cleared.set_i16(by_id::item_list(2), 5);
        let mut playing = cleared.clone();
        playing.set_u8(offset::CLEAR_FLAG, 0);
        save_to(&dir, Volume::Inf, 0, &cleared);
        save_to(&dir, Volume::Inf, 1, &playing);
        if with_mutation_save {
            let mut own = SaveData::new();
            own.bytes_mut()[..5].copy_from_slice(b"Mute\0");
            save_to(&dir, Volume::Mut, 0, &own);
        }
        let Some((mut s, mut pad)) = convert_list(&dir, Volume::Mut, false) else { return };
        let sys = title(&s).unwrap().demo.save_sys();
        let (first, second) = (sys.record_prev(0), sys.record_prev(1));
        assert_eq!((first.status, first.clear_flag, first.name()), (1, 1, &b"Haru"[..]));
        assert_eq!((second.status, second.clear_flag), (1, 0));
        // The second save: OK does nothing; back up to the first.
        tap(&mut s, &mut pad, Buttons::DOWN);
        tap(&mut s, &mut pad, Buttons::CROSS);
        run(&mut s, &mut pad, 0..10, &[]);
        assert_eq!(title(&s).unwrap().demo.save_sys().result, LOAD_SELECT, "a save that has not cleared Infection");
        tap(&mut s, &mut pad, Buttons::UP);
        let save = convert_chosen(&mut s, &mut pad);

        assert_eq!(save.cstr(offset::PL_NAME, 24), b"Haru");
        assert_eq!(save.clear_flag(), 1);
        assert!((4_000_000..4_000_000 + 120).contains(&save.play_time()), "{}", save.play_time());
        assert_eq!(save.event_flag(100) & EVENT_DONE, EVENT_DONE, "ccStartEventConvert");
        assert_eq!(save.u8(offset::NEW_GAME_FLAG), 2, "ConvGame");
        assert_eq!(save.i16(by_id::spc_param(0) + offset::SPC_LEVEL), 42, "Kite carried");
        assert_eq!(save.i16(by_id::spc_param(1) + offset::SPC_LEVEL), 33, "a party member carried");
        let row = &piney_data::tables::newgame::of(Volume::Mut).chars[2];
        assert_eq!(save.i16(by_id::spc_param(2) + offset::SPC_LEVEL), row.base.level, "back to charTbl");
        assert_eq!(save.i16(by_id::item_list(2)), -1, "its items emptied");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Outbreak's CONVERT carries Mutation's clear data and Quarantine's
/// Outbreak's, by the same code: the save that cleared the volume before
/// (clear flag 2, then 3) is taken, and event 200, then 300, is done.
#[test]
fn outbreak_and_quarantine_convert_the_part_before() {
    for (volume, prev) in [(Volume::Out, Volume::Mut), (Volume::Qua, Volume::Out)] {
        let dir = empty_card();
        let mut cleared = SaveState::fresh().save;
        cleared.bytes_mut()[..5].copy_from_slice(b"Haru\0");
        cleared.set_u8(offset::CLEAR_FLAG, prev.number() as u8);
        save_to(&dir, prev, 3, &cleared);
        let Some((mut s, mut pad)) = convert_list(&dir, volume, false) else { continue };
        let rec = title(&s).unwrap().demo.save_sys().record_prev(3);
        assert_eq!((rec.status, rec.clear_flag), (1, prev.number() as i8), "{volume}");
        for _ in 0..3 {
            tap(&mut s, &mut pad, Buttons::DOWN);
        }
        let save = convert_chosen(&mut s, &mut pad);
        assert_eq!(save.cstr(offset::PL_NAME, 24), b"Haru", "{volume}");
        let event = 100 * (volume.number() as usize - 1);
        assert_eq!(save.event_flag(event) & EVENT_DONE, EVENT_DONE, "{volume}: event {event}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
