//! Issue #50: Ryu Book III's "Online" for a person in town. `SetCharInfo`
//! (gcmn 0x0040bcd0) marks a person (`npcTbl` 30-65) or another player
//! (66-79) online when an entry on `g_entCtrl`'s NPC list has its row as
//! `entParam.id`; the port's town handed the book no list.

use piney_data::save::offset;
use piney_data::tables::sjis::encode;
use piney_data::volume::Volume;
use piney_input::{Buttons, Raw};

use super::fairy_orb::{menus, read_a_ryu_book};
use super::*;

/// Book III's row of character `id`: members 1-17, people 30-65, players
/// 66-79 (`SetCharInfo`'s order).
fn row_of(id: i32) -> usize {
    match id {
        1..=17 => (id - 1) as usize,
        30..=65 => (17 + id - 30) as usize,
        _ => (53 + id - 66) as usize,
    }
}

/// The open book's page.
fn book(s: &Session) -> Option<&piney_fieldui::book::Book> {
    menus(s)?.ctrl.book.as_ref()?.book.as_ref()
}

/// Mac Anu, one of its walking PCs (`pc`) and one person not in town
/// (`away`) the only people met, Book III open on `pc`'s sub-window:
/// (session, pc, away, the last frame).
fn stare_at_a_walking_pc() -> Option<(Session, i32, i32, Frame)> {
    use crate::session::area15::{disc, hold, start};
    let (iso, archive) = disc()?;
    let mut s = start(&iso, &archive, None);
    let in_town = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    hold(&mut s, 128, 128, 900, in_town);
    let Stage::World(w) = &mut s.stage else { unreachable!() };
    let here: Vec<i32> = w.world().all_npcs().map(|n| n.code()).filter(|c| (30..80).contains(c)).collect();
    let pc = *here.iter().find(|&&c| c >= 66).or(here.first()).expect("no walking PC in Mac Anu");
    let away = (30..80).find(|c| !here.contains(c)).expect("everyone in town");
    // Met: these two only (`pcTradeCount`, -1 never met).
    let save = &mut w.world_mut().state_mut().save;
    for id in 30..80 {
        save.set_u8(offset::PC_TRADE_COUNT + id - 13, if id == pc as usize || id == away as usize { 1 } else { 0xff });
    }
    save.set_i32(offset::PARTY_MEMBER_FLAG, 0);
    assert_eq!(read_a_ryu_book(&mut s, 2, in_town), (true, true), "(the cover, the pages)");
    let mut pad = Pad::default();
    let mut frame = Frame::new();
    for f in 0..600u32 {
        let b = book(&s).expect("the book shut");
        if b.is_sub_win_open != 0 {
            for _ in 0..10 {
                pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
                frame = s.step(&pad);
                s.take_events();
            }
            return Some((s, pc, away, frame));
        }
        // Onto the list, along it to the PC's row, its sub-window.
        let want = match (b.now as usize).cmp(&row_of(pc)) {
            _ if b.sel != b.sel_max => Buttons::DOWN,
            std::cmp::Ordering::Less => Buttons::DOWN,
            std::cmp::Ordering::Greater => Buttons::UP,
            std::cmp::Ordering::Equal => Buttons::CROSS,
        };
        let buttons = if f.is_multiple_of(8) { want } else { Buttons::NONE };
        pad.read(&Raw { buttons, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        frame = s.step(&pad);
        s.take_events();
    }
    panic!("the PC's sub-window never opened");
}

/// A walking PC in Mac Anu is "Online" in Ryu Book III, and its
/// sub-window draws the word; someone met but not in town is not. Before,
/// no one was ever online.
#[test]
fn a_walking_pc_in_town_is_online_in_ryu_book_iii() {
    let Some((s, pc, away, _)) = stare_at_a_walking_pc() else { return };
    let b = book(&s).unwrap();
    assert_eq!(b.now as usize, row_of(pc), "the sub-window open on the PC");
    assert_eq!(b.char_data[row_of(pc)].is_online, 1, "the PC ({pc}) in town");
    assert_eq!(b.char_data[row_of(away)].is_online, 0, "the person ({away}) not in town");
    let on_line = encode(piney_data::tables::book::of(Volume::Inf).on_line());
    let drawn = menus(&s)
        .unwrap()
        .draws()
        .iter()
        .any(|d| matches!(d, piney_fieldui::ctrl::Draw::Text { text, .. } if *text == on_line));
    assert!(drawn, "\"Online\" drawn in the sub-window");
}

/// A shot of the PC's sub-window (ignored; a diagnostic), to
/// `$PINEY_SHOTS` (/mnt/data/claude/scratch/issue50).
#[test]
#[ignore]
fn ryu_book_online_shot() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/issue50".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some((s, pc, _, frame)) = stare_at_a_walking_pc() else { return };
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    g.set_overlay(Mode::archive(&s));
    g.render(&frame);
    let (w, h) = g.target_size();
    let path = format!("{dir}/book3-pc{pc}.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    println!("{path}");
}
