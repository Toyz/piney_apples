//! The riding Grunty through the session: a random field of Dun Loireag's
//! server (event 25's start, the party at the gate), the Grunty Flute and a grown Grunty in
//! the save; PERSONAL, Key Items, the flute (`ccUseItemRequest(plw, plw,
//! 0xf0031, kind)`); the fade out, the party asleep, the Grunty and Kite on
//! it; the stick; the cancel button and the dismount (the fade, the members
//! placed about Kite, the menu shut).

use piney_input::{Buttons, Raw};
use piney_world::area::{Scene, WorldMan, kind};
use piney_world::field_world::Place;

use super::*;

/// The flute (key item 49) and a grown Grunty in pen 1 of `server`.
fn give_flute(save: &mut piney_data::save::SaveData, server: i32) {
    save.set_u8(0x0cfc + 49, 1);
    save.set_i16(piney_battle::ride::SAVE_GROWTH + 24 * server as usize + 0xe + 2, 1);
}

/// The first word triple in the tables' order that makes a random area on
/// `server` whose field is not a lake's (or the special types).
fn field_words(server: i32) -> Option<[i32; 3]> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut disc = Iso::open(&iso).ok()?;
    let t = crate::area::area_tables(&mut disc).unwrap();
    let words = |slot: usize| t.words.iter().filter(|w| w.slot == slot).map(|w| w.id).collect::<Vec<_>>();
    let (a, b, c) = (words(0), words(1), words(2));
    for &z in &c {
        for &y in &b {
            for &x in &a {
                let Some(code) = piney_data::area::sim_generate_code(t, x, y, z, server, false) else { continue };
                let ft = code.attrs.field_type;
                if code.event != 0 || matches!(ft, 2 | 3 | 4 | 8 | 9 | 10) {
                    continue;
                }
                return Some([x, y, z]);
            }
        }
    }
    None
}

/// Event 25's start logged in, the area of a random field made and the
/// party put in it as the gate leaves it, with the flute and a Grunty.
pub(super) fn in_field() -> Option<(Session, i32)> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut disc = Iso::open(&iso).unwrap();
    let tables = crate::area::area_tables(&mut disc).unwrap();
    let mut server = 0;
    let s = story_session_with(25, |start| {
        let save = &mut start.state.save;
        // Dun Loireag's gate (game.server 1, where the Grunties grow).
        save.set_u8(offset::LAST_TOWN, 1);
        let mut scene = Scene::log_in(save);
        server = scene.server;
        give_flute(save, scene.server);
        let words = field_words(scene.server).expect("a field's words");
        let (wm, _) = WorldMan::set_generate_code(tables, words, scene.server, save).expect("the words");
        scene.change_scene(1, 1, 0, -1, -1, -1, save);
        scene.change_area(kind::FIELD, 0, save);
        start.at = crate::session::Resume::World(Box::new(crate::session::InWorld {
            scene,
            world_man: Some(wm),
            spcs: Some(crate::start::party(25)),
        }));
    })?;
    Some((s, server))
}

pub(super) fn raw(buttons: Buttons, ly: u8) -> Raw {
    Raw { buttons, analog: true, lx: 128, ly, rx: 128, ry: 128, ..Raw::default() }
}

pub(super) fn area(s: &Session) -> &crate::area::AreaMode {
    match &s.stage {
        Stage::Area(a) => a,
        _ => panic!("not in the field: {}", Mode::title(s)),
    }
}

/// What the ride's check reads each frame.
#[derive(Debug, Default, Clone, Copy)]
struct Look {
    riding: bool,
    obj: bool,
    asleep: bool,
    main_on: bool,
    ride_pos: [f32; 3],
    kite_pos: [f32; 3],
    kite_drawn: bool,
    act: i16,
    menu: i32,
    forbid: i16,
}

fn look(s: &Session) -> Look {
    let Stage::Area(a) = &s.stage else { return Look::default() };
    let w = a.world();
    let c = w.combat();
    let Some(k) = c.kite else { return Look::default() };
    let r = &c.ride;
    let f = |v: [u32; 4]| [f32::from_bits(v[0]), f32::from_bits(v[1]), f32::from_bits(v[2])];
    Look {
        riding: r.flag,
        obj: r.obj.is_some(),
        asleep: r.asleep,
        main_on: r.main_on,
        ride_pos: r.obj.as_ref().map_or([0.0; 3], |o| f(o.ride.pos)),
        kite_pos: f(c.scene.chars[k].pos),
        kite_drawn: c.cast.get(k).is_some_and(|a| a.drawn),
        act: r.obj.as_ref().map_or(-1, |o| o.ride.act),
        menu: a.ui().menu_type(),
        forbid: a.ui().ctrl.forbid,
    }
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// From the field's start to the flute blown: PERSONAL (Triangle), its
/// third row (Key Items), the flute, OK. The frame the ride's start takes
/// the menu, and the session's events so far.
fn blow_flute(s: &mut Session, pad: &mut Pad, events: &mut Vec<Event>) -> u32 {
    let mut f = 0u32;
    loop {
        f += 1;
        assert!(f < 3000, "the flute was never blown: {}", Mode::title(s));
        let ready = match &s.stage {
            Stage::Area(a) => matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 90),
            _ => false,
        };
        let b = if !ready || !f.is_multiple_of(8) {
            Buttons::NONE
        } else {
            let a = area(s);
            let c = &a.ui().ctrl;
            match (a.ui().menu_type(), c.proccess) {
                (-1, _) => Buttons::TRIANGLE,
                (1, 1) if c.list().select < 2 => Buttons::DOWN,
                (1, 1) => Buttons::CROSS,
                (6, 1) => Buttons::CROSS,
                _ => Buttons::NONE,
            }
        };
        pad.read(&raw(b, 128));
        s.step(pad);
        events.extend(s.take_events());
        if look(s).riding {
            return f;
        }
        if std::env::var("RIDE_DEBUG").is_ok() && f.is_multiple_of(8) && ready {
            let a = area(s);
            let c = &a.ui().ctrl;
            let wc = a.world().combat();
            eprintln!(
                "{f} menu {} proccess {} select {} page {} wait {} inb {} b {:?}",
                a.ui().menu_type(),
                c.proccess,
                c.list().select,
                c.list().page,
                c.wait_count,
                wc.battle.in_battle,
                b
            );
        }
    }
}

#[test]
fn the_grunty_flute_rides_and_dismounts() {
    let Some((mut s, server)) = in_field() else { return };
    let mut pad = Pad::default();
    let mut events = Vec::new();
    // The field plays; Kite stands where the gate left him.
    blow_flute(&mut s, &mut pad, &mut events);
    let kind = 2 * server + 1 - 2;
    let calls = area(&s).calls().iter().map(|c| c.1.clone()).collect::<Vec<_>>();
    assert!(calls.contains(&format!("pucciguso {kind}")), "{calls:?}");
    let before = look(&s);
    assert!(before.riding && !before.obj && !before.asleep);
    assert_eq!((before.menu, before.forbid), (6, 1), "the key items menu held in the call, the menu banned");
    assert!(events.iter().any(|e| matches!(e, Event::PgBgm(piney_audio::PgBgm::Init))));
    // The fade out (10 frames), then the party asleep and the Grunty.
    let step = |s: &mut Session, pad: &mut Pad, b: Buttons, ly: u8, events: &mut Vec<Event>| {
        pad.read(&raw(b, ly));
        s.step(pad);
        events.extend(s.take_events());
        look(s)
    };
    let mut l = before;
    for _ in 0..12 {
        l = step(&mut s, &mut pad, Buttons::NONE, 128, &mut events);
    }
    assert!(l.asleep && l.obj && l.main_on, "{l:?}");
    assert!(!l.kite_drawn, "Kite's own model is not drawn while he rides");
    assert!(dist(l.ride_pos, before.kite_pos) < 1.0, "the Grunty comes where Kite stood: {l:?} {before:?}");
    // The fade back in (15), the ride's music.
    for _ in 0..20 {
        l = step(&mut s, &mut pad, Buttons::NONE, 128, &mut events);
    }
    assert!(events.iter().any(|e| matches!(e, Event::BgmStream(0))));
    {
        let r = area(&s).world().combat().ride.obj.as_ref().unwrap();
        assert_eq!(r.ride.kind, kind);
        let pg = format!("ANM_cdg{}", if kind > 0 { kind + 1 } else { 0 });
        assert!(r.pg.anim_name().is_some_and(|n| n.starts_with(&pg)), "{:?}", r.pg.anim_name());
        assert!(r.kite.anim_name().is_some_and(|n| n.starts_with("ANM_ctu1dg")));
    }
    // The stick up: it walks, then runs, Kite carried along.
    let start = l.ride_pos;
    let mut acts = Vec::new();
    for _ in 0..90 {
        l = step(&mut s, &mut pad, Buttons::NONE, 0, &mut events);
        acts.push(l.act);
        assert_eq!(l.kite_pos, l.ride_pos, "plw.pos is the ride's");
    }
    assert!(dist(start, l.ride_pos) > 600.0, "it did not go far: {start:?} {l:?}");
    assert!(acts.contains(&piney_battle::ride::act::RUN), "{acts:?}");
    // Let go; it slows to a stand.
    for _ in 0..40 {
        l = step(&mut s, &mut pad, Buttons::NONE, 128, &mut events);
    }
    assert_eq!(l.act, piney_battle::ride::act::IDLE, "{l:?}");
    let stood = l.ride_pos;
    // Circle, the cancel button: the dismount's fade out with Main (10),
    // then the party awake and placed, the fade in (15).
    l = step(&mut s, &mut pad, Buttons::CIRCLE, 128, &mut events);
    assert!(l.riding && l.main_on);
    for _ in 0..12 {
        l = step(&mut s, &mut pad, Buttons::NONE, 128, &mut events);
    }
    assert!(!l.asleep && !l.main_on && l.riding, "{l:?}");
    {
        let w = area(&s).world();
        let c = w.combat();
        let k = c.kite.unwrap();
        for &(id, m) in &c.members {
            if m == k {
                continue;
            }
            let p = c.scene.chars[m].pos.map(f32::from_bits);
            let d = dist([p[0], p[1], p[2]], l.kite_pos);
            assert!((d - 150.0).abs() < 1.0, "member {id} is {d} from Kite");
        }
        // Where it stood, but for the last of its eased move through the
        // fade's Mains.
        assert!(dist(l.kite_pos, stood) < 10.0, "{:?} {stood:?}", l.kite_pos);
    }
    for _ in 0..20 {
        l = step(&mut s, &mut pad, Buttons::NONE, 128, &mut events);
    }
    assert!(!l.riding && !l.obj, "{l:?}");
    assert!(events.iter().any(|e| matches!(e, Event::PgBgm(piney_audio::PgBgm::End(0)))));
    // The menu shuts after the call; the ban is off; Kite walks again.
    for _ in 0..30 {
        l = step(&mut s, &mut pad, Buttons::NONE, 128, &mut events);
    }
    assert_eq!((l.menu, l.forbid), (-1, 0), "{l:?}");
    assert!(l.kite_drawn);
    let here = l.kite_pos;
    for _ in 0..30 {
        l = step(&mut s, &mut pad, Buttons::NONE, 0, &mut events);
    }
    assert!(dist(here, l.kite_pos) > 50.0, "Kite does not walk after the ride");
    let _ = Place::Field;
}

/// Pictures of the ride: the Grunty come, standing with Kite on it, running
/// (its dust), the fade of the dismount and the party after it.
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game ride_shots --
/// --ignored --nocapture` (default `/mnt/data/claude/scratch/ride/shots`).
#[test]
#[ignore]
fn ride_shots() {
    let Some((mut s, _)) = in_field() else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/ride/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let archive = match &s.stage {
        Stage::Area(_) | Stage::World(_) => Mode::archive(&s),
        _ => None,
    };
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut disc = Iso::open(&iso).unwrap();
    let data = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive.unwrap_or(data))).unwrap();
    let mut pad = Pad::default();
    let mut events = Vec::new();
    let mut shot = |s: &Session, frame: &_, name: &str| {
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let (w, h) = gs.target_size();
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
        println!("{path}: {}", Mode::title(s));
    };
    blow_flute(&mut s, &mut pad, &mut events);
    let frames = |s: &mut Session, pad: &mut Pad, n: u32, b: Buttons, ly: u8| {
        let mut f = None;
        for _ in 0..n {
            pad.read(&raw(b, ly));
            f = Some(s.step(pad));
            s.take_events();
        }
        f.unwrap()
    };
    let f = frames(&mut s, &mut pad, 6, Buttons::NONE, 128);
    shot(&s, &f, "1-fading");
    let f = frames(&mut s, &mut pad, 40, Buttons::NONE, 128);
    shot(&s, &f, "2-riding");
    let f = frames(&mut s, &mut pad, 70, Buttons::NONE, 0);
    shot(&s, &f, "3-running");
    let f = frames(&mut s, &mut pad, 50, Buttons::NONE, 128);
    shot(&s, &f, "4-stood");
    frames(&mut s, &mut pad, 1, Buttons::CIRCLE, 128);
    let f = frames(&mut s, &mut pad, 5, Buttons::NONE, 128);
    shot(&s, &f, "5-dismounting");
    let f = frames(&mut s, &mut pad, 40, Buttons::NONE, 128);
    shot(&s, &f, "6-dismounted");
}
