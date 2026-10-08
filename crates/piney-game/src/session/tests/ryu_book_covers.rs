//! Issue #61: Mutation's Ryu Books played other streams for their covers
//! (Book I the Altimit boot, II the gate hack, III to VI the drains). The
//! town played stream 112 + the book, Infection's numbering; `ccThBook`
//! adds 118 from Mutation on (`book::cover_stream`), whose tables put six
//! streams before the covers. From the second book on `ccThBook` also
//! puts the book's palette on the backdrop (`str8800e`'s `MAT_clut`).

use piney_data::volume::Volume;
use piney_input::Buttons;
use piney_stream::file::StreamFile;

use super::fairy_orb::{BookRead, menus, read_ryu_book};
use super::ride::raw;
use super::*;
use crate::stream::tests::work_iso;

fn in_town(s: &Session) -> bool {
    matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30))
}

/// A new game's town on `v`, playing.
fn town(v: Volume) -> Option<Session> {
    use crate::session::area15::{hold, start};
    let iso = work_iso(v)?;
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut s = start(&iso, &archive, None);
    hold(&mut s, 128, 128, 900, in_town);
    Some(s)
}

/// Book `book` read (its cover, then its pages), then the book and the
/// menus closed with circle: what it showed, and the palette swaps the
/// backdrop's (`str8800e`'s) models drew with.
fn read_and_close(s: &mut Session, v: Volume, book: usize) -> (BookRead, Vec<Vec<(u32, u32)>>) {
    let mut palettes: Vec<Vec<(u32, u32)>> = Vec::new();
    let r = read_ryu_book(s, book, in_town, |_, frame| {
        for c in &frame.cmds {
            if let piney_draw::Cmd::Model(m) = c
                && m.file == "str8800e"
                && !palettes.contains(&m.clut_swaps)
            {
                palettes.push(m.clut_swaps.clone());
            }
        }
    });
    assert!(r.cover && r.pages, "{v:?} book {}: (the cover, the pages) ({}, {})", book + 1, r.cover, r.pages);
    let mut pad = Pad::default();
    for f in 0..900u32 {
        if menus(s).is_none_or(|ui| ui.ctrl.book.is_none() && ui.menu_type() == -1) {
            return (r, palettes);
        }
        pad.read(&raw(if f % 10 == 9 { Buttons::CIRCLE } else { Buttons::NONE }, 128));
        s.step(&pad);
        s.take_events();
    }
    panic!("{v:?} book {}: the menus never closed", book + 1);
}

/// The cover each book plays is its own (`str8801`-`str8808`): the
/// volume's first cover + the book, 112 on Infection and 118 on the
/// later volumes, as their `ccThBook`s add.
#[test]
fn each_ryu_book_plays_its_own_cover() {
    for v in Volume::ALL {
        if work_iso(v).is_none() {
            continue;
        }
        let first = if v == Volume::Inf { 112 } else { 118 };
        for book in 0..8 {
            let mut s = town(v).unwrap();
            let (r, _) = read_and_close(&mut s, v, book);
            assert_eq!(r.streams, [first + book], "{v:?} book {}: the cover's streams", book + 1);
            let def = piney_stream::table::Def::read(v, first + book, false).unwrap();
            assert_eq!(def.header.name.trim_end_matches('E'), format!("str880{}", book + 1), "{v:?}");
        }
    }
}

/// `ccThBook`'s palette: Book I draws the backdrop on its own palette,
/// Book III on `CLT_x800bac1c2` (`stream_cluts[2]`), and Book I read
/// after it in the same town keeps that one, the town's `str8800e`
/// being the same loaded file.
#[test]
fn a_ryu_books_cover_takes_the_books_palette() {
    for v in [Volume::Inf, Volume::Mut] {
        let Some(mut s) = town(v) else { continue };
        let backdrop = StreamFile::read(&s.archive, "str8800e").unwrap();
        let id = |name: &str| backdrop.sf.ccs.find_object(name).unwrap();
        let book3 = vec![(id("CLT_x800bac1"), id("CLT_x800bac1c2"))];
        assert_eq!(read_and_close(&mut s, v, 0).1, [Vec::new()], "{v:?} Book I first");
        assert_eq!(read_and_close(&mut s, v, 2).1, std::slice::from_ref(&book3), "{v:?} Book III");
        assert_eq!(read_and_close(&mut s, v, 0).1, [book3], "{v:?} Book I after Book III");
    }
}

/// Shots of Mutation's covers (ignored; a diagnostic): Books I and III
/// at their stream's frame 90, to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/i61).
#[test]
#[ignore]
fn ryu_book_cover_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i61".into());
    std::fs::create_dir_all(&dir).unwrap();
    for book in [0, 2] {
        let Some(mut s) = town(Volume::Mut) else { return };
        let mut shot = None;
        read_ryu_book(&mut s, book, in_town, |s, frame| {
            if let Stage::World(w) = &s.stage
                && shot.is_none()
                && w.stream_frame() == Some(90)
            {
                let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
                g.set_overlay(Mode::archive(s));
                g.render(frame);
                shot = Some((g.target_size(), g.read_back(), w.stream_num()));
            }
        });
        let Some(((w, h), rgba, num)) = shot else { panic!("book {}: no frame 90", book + 1) };
        let path = format!("{dir}/mut-book{}-stream{}.png", book + 1, num.unwrap_or(0));
        std::fs::write(&path, piney_gs::png::encode(w, h, &rgba)).unwrap();
        println!("{path}");
    }
}
