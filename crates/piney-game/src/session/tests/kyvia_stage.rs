//! Issues #71 and #72: Mutation's event 108 from area 47's dungeon to
//! Kyvia's (Cubia's) first fight. The dungeon's last door is
//! `ChangeArea(1, 9)` with area 47's `WORLD_MAN` still in place.

use piney_input::Raw;

use super::*;

fn still() -> Raw {
    Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// A session whose every frame's events go to a headless engine as `main`
/// routes them, from its start.
struct Ear {
    s: Session,
    pad: Pad,
    audio: piney_audio::Audio,
}

impl Ear {
    fn step(&mut self, raw: &Raw) -> Vec<String> {
        self.pad.read(raw);
        let rate = Mode::frame_rate(&self.s);
        self.s.step(&self.pad);
        let events = self.s.take_events();
        let said = events
            .iter()
            .filter(|e| !matches!(e, Event::MovieAudio(_) | Event::StreamPcm(_)))
            .map(|e| format!("{e:?}"))
            .filter(|e| !e.starts_with("Se") && !e.starts_with("SceneSound"))
            .collect();
        crate::handle(events, Some(&self.audio));
        self.audio.frame();
        self.audio.render(&mut vec![0i16; 1600 * rate as usize]);
        said
    }

    /// The sequencers playing.
    fn sequences(&self) -> [bool; 3] {
        self.audio.with_engine(|e| e.seq.each_ref().map(|q| q.playing()))
    }
}

/// Event 108 brought to area 47's dungeon (floor 0, room 0) on area 47's
/// `WORLD_MAN`: blocks 0-20 played and `eventStatus[0]` 1 (block 17's
/// `set status`), as the Chaos Gate and the dungeon leave them.
fn in_area_47_dungeon() -> Option<Ear> {
    in_area_47_dungeon_of("mutation", 108, 21)
}

/// Event `n` of `disc` in area 47's dungeon, its blocks below `block`
/// (the one that makes Kyvia) played and `eventStatus[0]` 1.
fn in_area_47_dungeon_of(disc: &str, n: i32, block: u32) -> Option<Ear> {
    let iso = crate::check_image(disc);
    if !iso.exists() {
        eprintln!("{disc}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, n).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(n as usize);
    state.save.set_event_flag(n as usize, f | ((1 << block) - 1));
    state.save.set_u8(offset::EVENT_STATUS, 1);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 47, false).unwrap();
    scene.change_scene(2, 2, 47, 0, 0, 0, &mut state.save);
    let s = Session::in_world(iso.clone(), archive, None, state, Some(start.vm), scene, Some(wm)).unwrap();
    Some(Ear { s, pad: Pad::default(), audio: piney_audio::Audio::headless(&iso).unwrap() })
}

/// The stream event 108 plays at phase 2 on the way in, if any.
fn streaming(s: &Session) -> Option<String> {
    match &s.stage {
        Stage::Area(a) => a.stream_scene(),
        _ => None,
    }
}

/// Kyvia's body, once block 21 has made it.
fn kyvia(s: &Session) -> Option<usize> {
    let Stage::Area(a) = &s.stage else { return None };
    let c = a.world().combat();
    let me = c.boss_char()?;
    matches!(c.scene.chars[me].foe_state()?.boss.as_ref()?.class, piney_battle::boss::Class::Kyvia(_)).then_some(me)
}

/// Issue #72: area 47's last door is `ChangeArea(1, 9)` on area 47's
/// `WORLD_MAN` (field 10 from Outbreak on, its event 211). `GO(1)` takes
/// `eventAreaNumber` from `game.field` and the field's row (model 1)
/// makes `EVENTAREAB8`, the disc Kyvia fights on. Before the fix the port
/// kept area 47's number and model 0, and the fight was in area 47's
/// generated lava field.
#[test]
fn area_47s_door_leads_to_kyvias_disc() {
    for (disc, n, block) in [("mutation", 108, 21), ("outbreak", 211, 11)] {
        let Some(mut e) = in_area_47_dungeon_of(disc, n, block) else { continue };
        for _ in 0..200 {
            e.step(&still());
        }
        let volume = Iso::open(&e.s.iso).unwrap().volume().unwrap().number() as u32;
        let field = piney_data::dungeon::special::exit_field(47, volume).unwrap();
        e.s.go(Pending::Go(piney_data::area::Go::ChangeArea(1, field)));
        let mut made = false;
        for f in 0..8000u32 {
            // START skips the event's stream; CROSS moves its lines on.
            let raw = match streaming(&e.s) {
                Some(_) if f.is_multiple_of(30) => Raw { buttons: piney_input::Buttons::START, ..still() },
                None if f.is_multiple_of(8) => Raw { buttons: piney_input::Buttons::CROSS, ..still() },
                _ => still(),
            };
            e.step(&raw);
            if kyvia(&e.s).is_some() {
                made = true;
                break;
            }
        }
        let Stage::Area(a) = &e.s.stage else { panic!("{disc}: not in the field: {}", Mode::title(&e.s)) };
        let w = a.world();
        assert_eq!((w.scene().area, w.scene().field), (1, field), "{disc}");
        assert_eq!(w.world_man().event, field, "{disc}: eventAreaNumber");
        assert!(w.place().story::<piney_world::evarea_b8::DiscArea>().is_some(), "{disc}: not on the disc");
        assert!(made, "{disc}: event {n} made no Kyvia: {}", Mode::title(&e.s));
    }
}

/// Issue #71: leaving area 47's dungeon for field 9, `ccSetupGameCtrl`
/// fades the music out (`ccSoundFadeOut`, the scene changes) and after
/// its fade `ccAllSoundOff` stops it, before event 108's phase-2 stream
/// (`str0932`) plays. Before the fix the session's fade sent neither, and
/// the dungeon's music played on under the stream.
#[test]
fn area_47s_music_stops_before_event_108s_stream() {
    let Some(mut e) = in_area_47_dungeon() else { return };
    for _ in 0..200 {
        e.step(&still());
    }
    assert!(e.sequences()[0], "the dungeon's music is not on");
    e.s.go(Pending::Go(piney_data::area::Go::ChangeArea(1, 9)));
    let (mut said, mut streamed, mut heard) = (Vec::new(), 0u32, 0u32);
    for _ in 0..600 {
        let now = e.step(&still());
        if streaming(&e.s).is_some() {
            streamed += 1;
            heard += u32::from(e.sequences().iter().any(|&q| q));
        } else {
            said.extend(now.into_iter().filter(|e| matches!(e.as_str(), "SoundFadeOut" | "AllSoundOff")));
        }
    }
    println!("before the stream {said:?}; the stream {streamed} frames, music in {heard}");
    assert_eq!(said, ["SoundFadeOut", "AllSoundOff"]);
    assert!(streamed > 300, "the stream played {streamed} frames");
    assert_eq!(heard, 0, "the dungeon's music under the stream");
}

/// Shots of the fight's start from area 47's door to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/cubia), every 200th of 1200 frames once
/// Kyvia is made, and the mean wall time of a session step over them.
#[test]
#[ignore]
fn kyvia_stage_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/cubia".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut e) = in_area_47_dungeon() else { return };
    for _ in 0..200 {
        e.step(&still());
    }
    e.s.go(Pending::Go(piney_data::area::Go::ChangeArea(1, 9)));
    let mut f = 0u32;
    while kyvia(&e.s).is_none() && f < 8000 {
        let raw = match streaming(&e.s) {
            Some(_) if f.is_multiple_of(30) => Raw { buttons: piney_input::Buttons::START, ..still() },
            None if f.is_multiple_of(8) => Raw { buttons: piney_input::Buttons::CROSS, ..still() },
            _ => still(),
        };
        e.step(&raw);
        f += 1;
    }
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(e.s.archive.clone())).unwrap();
    let (mut stepping, mut drawing) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    for i in 0..1200u32 {
        if let Stage::Area(a) = &mut e.s.stage {
            let c = a.world_mut().combat_mut();
            for &(_, k) in &c.members {
                c.scene.chars[k].hp = c.scene.chars[k].max_hp;
            }
        }
        let raw = if i.is_multiple_of(8) { Raw { buttons: piney_input::Buttons::CROSS, ..still() } } else { still() };
        e.pad.read(&raw);
        let t = std::time::Instant::now();
        let frame = e.s.step(&e.pad);
        stepping += t.elapsed();
        crate::handle(e.s.take_events(), Some(&e.audio));
        let t = std::time::Instant::now();
        gs.set_overlay(Mode::archive(&e.s));
        gs.render(&frame);
        let (w, h) = gs.target_size();
        let picture = gs.read_back();
        drawing += t.elapsed();
        if i.is_multiple_of(200) {
            let path = format!("{dir}/fight-{i:04}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &picture)).unwrap();
            println!("{path}");
        }
    }
    let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0 / 1200.0;
    println!("a step {:.2} ms, its picture {:.2} ms", ms(stepping), ms(drawing));
}

/// Shots of Kyvia's attacks from area 47's door to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/cubia/acts): every 8th frame of the arm (3),
/// the beams (4) and the meteors (5) and of the 120 after each, through
/// the fight's first `$PINEY_FRAMES` (5000), the core's time run short.
#[test]
#[ignore]
fn kyvia_attack_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/cubia/acts".into());
    let frames = std::env::var("PINEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(5000);
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut e) = in_area_47_dungeon() else { return };
    for _ in 0..200 {
        e.step(&still());
    }
    e.s.go(Pending::Go(piney_data::area::Go::ChangeArea(1, 9)));
    let mut gs: Option<piney_gs::Gs> = None;
    let mut since = u32::MAX;
    super::kyvia_fx::fight(&mut e.s, frames, |s, frame, i, now| {
        since = if (3..=5).contains(&now.act) { 0 } else { since.saturating_add(1) };
        if since > 120 || !i.is_multiple_of(8) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/kyvia-{i:05}-act{}.png", now.act);
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}

/// Issue #72's glyphs (`o e a 0 / ¥ = * Y :`): area 47 is a hacked area,
/// and its generated field flies data bugs (`FIREFLY` hacked,
/// `EFF_sfzdigi0` trailing `field_eff`'s `TEX_sfzdigi1` glyphs). Fought
/// there, the bugs drew over the fight; on the disc none is drawn.
#[test]
fn no_data_bugs_over_kyvias_disc() {
    let Some(mut e) = in_area_47_dungeon() else { return };
    for _ in 0..200 {
        e.step(&still());
    }
    e.s.go(Pending::Go(piney_data::area::Go::ChangeArea(1, 9)));
    let glyphs = piney_data::ccs::Ccs::parse(e.s.archive.inflate_named("field_eff").unwrap())
        .unwrap()
        .find_object("TEX_sfzdigi1")
        .unwrap();
    let mut drawn = 0u32;
    super::kyvia_fx::fight(&mut e.s, 1500, |_, frame, _, _| {
        drawn += frame
            .cmds
            .iter()
            .filter(|c| {
                matches!(c, piney_draw::Cmd::Prim(p) if matches!(&p.state.texture,
                Some(t) if matches!(&t.tex, piney_draw::TexRef::Ccs { file, texture, .. }
                    if file == "field_eff" && *texture == glyphs)))
            })
            .count() as u32;
    });
    assert_eq!(drawn, 0, "the data bugs' glyphs drawn over the fight");
}
