//! The World on the disc: Mac Anu entered as a new game, Kite's arrival,
//! walking, the walls and the camera. Skipped without
//! `work/infection/infection.iso`. `tools/test_world_rs.py` checks the same
//! code against the game's own functions frame by frame.

use std::path::Path;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_draw::Cmd;
use piney_input::{Pad, Raw};
use piney_world::hit::{FLOOR, HitModel, Hits, LAND_MASK, WALL};
use piney_world::motion::act;
use piney_world::{FADE_FRAMES, HOLD_FRAMES, Phase, Request, SaveState, World, ee};

const ISO: &str = "../../work/infection/infection.iso";

fn disc() -> Option<(Iso, Arc<Archive>)> {
    if !Path::new(ISO).exists() {
        eprintln!("skipped: no {ISO}");
        return None;
    }
    let mut iso = Iso::open(ISO).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    Some((iso, archive))
}

/// A new game's save: `SaveState::fresh`, then `ccSaveData::NewGame(0)`
/// with the disc's `charTbl`.
fn new_game(iso: &mut Iso) -> SaveState {
    let tables = piney_demo::newgame::NewGameTables::of(iso.volume().unwrap());
    let mut state = SaveState::fresh();
    piney_demo::newgame::new_game(&mut state.save, 0, &tables, piney_demo::newgame::SAVE_VA);
    state
}

fn step(world: &mut World, pad: &mut Pad, raw: Raw) -> piney_draw::Frame {
    pad.read(&raw);
    world.step(pad)
}

#[test]
fn mac_anu_collision_mesh() {
    let Some((_, archive)) = disc() else { return };
    for stem in ["town01", "town01d"] {
        let c = Ccs::parse(archive.inflate_named(stem).unwrap()).unwrap();
        let models = HitModel::read(piney_data::volume::Volume::Inf, &c).unwrap();
        assert_eq!(models.len(), 1, "{stem}");
        let polys = &models[0].polys;
        assert_eq!(polys.len(), 828);
        let floors = polys.iter().filter(|p| p.att & FLOOR != 0).count();
        let walls = polys.iter().filter(|p| p.att & WALL != 0).count();
        assert_eq!((floors, walls), (301, 527), "{stem}");
        assert_eq!(c.object_name(models[0].object), Some("HIT_sr1town1hit"));
        assert_eq!(c.object_name(models[0].parent), Some("MDL_floor_02"));
        // The gate plaza is at 600; a point off the mesh keeps its height.
        let mut hits = Hits::new(piney_data::volume::Volume::Inf, models);
        assert_eq!(ee::f(hits.land(piney_world::START_POS, LAND_MASK)), 600.0);
        let off = [ee::k(20000.0), ee::k(20000.0), ee::k(77.0), ee::ONE];
        assert_eq!(ee::f(hits.land(off, LAND_MASK)), 77.0);
    }
}

#[test]
fn arrival_then_walking() {
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    assert_eq!(world.frame_rate(), 2);
    let mut pad = Pad::default();
    // The fade out and the hold draw only the fader.
    for k in 0..FADE_FRAMES + HOLD_FRAMES {
        let f = step(&mut world, &mut pad, Raw::default());
        assert!(f.cmds.iter().all(|c| matches!(c, Cmd::Prim(_))), "frame {k}");
    }
    assert_eq!(world.phase(), Phase::Play(0));
    let reqs = world.take_requests();
    assert!(reqs.contains(&Request::SoundFadeOut) && reqs.contains(&Request::SqLoad(2)), "{reqs:?}");
    // The arrival: act 13, Kite faded in, then standing in town (act 2).
    let p = world.player();
    assert_eq!((p.acts.act, ee::f(p.body.pos[1]), ee::f(p.body.pos[2])), (act::ARRIVE, 5600.0, 600.0));
    let mut arrived = None;
    for i in 0..120 {
        let f = step(&mut world, &mut pad, Raw::default());
        if i > 2 {
            assert!(f.cmds.iter().any(|c| matches!(c, Cmd::Model(_))), "the town draws");
        }
        if arrived.is_none() && world.player().acts.act == act::IDLE {
            arrived = Some(i);
        }
    }
    // F0 sets up, F1 is the first act-13 frame; it ends on its 74th.
    assert_eq!(arrived, Some(74));
    assert!(world.take_requests().contains(&Request::Transfer));
    assert_eq!(ee::f(world.player().acts.cloak), 1.0);
    // The camera sits behind him: north of him, looking south.
    let cam = world.camera().tcam.pos;
    assert!(ee::f(cam[1]) > 6400.0 && ee::f(cam[2]) > 700.0, "{:?}", cam.map(ee::f));
    // Stick up: he runs south, down the stairs, the camera following.
    for _ in 0..70 {
        step(&mut world, &mut pad, Raw { ly: 0, ..Raw::default() });
        assert_eq!(world.player().acts.act, act::RUN);
    }
    let p = world.player();
    let (y, z) = (ee::f(p.body.pos[1]), ee::f(p.body.pos[2]));
    assert!(y < 4000.0 && z < 350.0, "ran to ({y}, {z})");
    assert!(ee::f(world.camera().tcam.pos[1]) > y);
    // Let go: he stops at once and stands.
    step(&mut world, &mut pad, Raw::default());
    let p = world.player();
    assert_eq!((p.acts.act, p.body.move_flag), (act::IDLE, false));
}

#[test]
fn walls_hold_him() {
    let Some((_, archive)) = disc() else { return };
    let c = Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap();
    let mut hits =
        Hits::new(piney_data::volume::Volume::Inf, HitModel::read(piney_data::volume::Volume::Inf, &c).unwrap());
    // Running east from the gate plaza for long enough to cross the town:
    // the plaza's walls stop him well short.
    let mut pos = piney_world::START_POS;
    let mut body = piney_world::hit::Body::default();
    for _ in 0..200 {
        let mv = [ee::k(27.5), 0, 0, ee::ONE];
        let (_, m) = hits.hit_check(&mut body, pos, mv, ee::k(45.0), ee::k(27.5), true);
        pos[0] = ee::add(pos[0], m[0]);
        pos[1] = ee::add(pos[1], m[1]);
        pos[2] = hits.land(pos, LAND_MASK);
    }
    assert!(ee::f(pos[0]) < 1500.0, "walked through to {:?}", pos.map(ee::f));
}

#[test]
fn markers_and_event_commands() {
    use piney_event::host::PcCommand;
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    // markerEvTbl[0] is DMY_gate, where the Chaos Gate stands.
    let (gate, _) = world.marker_bits(0).unwrap();
    assert_eq!(gate, world.gate().pos);
    // Every Mac Anu event marker the table names.
    let found: Vec<i16> = (0..33).filter(|&n| world.marker_bits(n).is_some()).collect();
    assert!(found.len() > 10, "{found:?}");
    assert!(world.marker_bits(3).is_some(), "event 2 puts Orca at marker 3");
    // pc_turn and pc_put_marker on Kite, at once.
    assert!(world.pc_command(PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 }));
    assert_eq!(world.player().body.dirc[2], ee::deg2rad(-32768));
    assert!(world.pc_command(PcCommand::PutMarker { pc: 0, marker: 3 }));
    assert_eq!(world.player().body.pos, world.marker_bits(3).unwrap().0);
    // A gradual turn needs the AI: declined.
    assert!(!world.pc_command(PcCommand::Turn { pc: 0, dirc: 0, chg: 8 }));

    // Event 2's `entry 1 2 3 5`, registered, then built and put by the
    // town's set-up: Orca at marker 3, facing its heading, on the party's
    // command list.
    assert!(world.entry(1, 2, 3, 5));
    let mut pad = Pad::default();
    for _ in 0..(FADE_FRAMES + HOLD_FRAMES + 1) {
        step(&mut world, &mut pad, Raw::default());
    }
    let (at, rot) = world.marker_bits(3).unwrap();
    let party = world.town_party();
    let orca = party.rec(2).unwrap();
    assert_eq!((orca.listed, party.flags(2)), (true, 6));
    assert_eq!(orca.pos, at);
    assert_eq!(orca.dirc[2], ee::deg2rad(ee::rad2deg(rot)));
    assert_eq!(party.actor(2).unwrap().ch.anim_name(), Some(party.combat.data.mt.clip(2, orca.act)));
    assert_eq!(world.char_pos(1, 2).unwrap().0, at);
    // pc_face Kite at Orca: Kite's heading is ccGetDirc's.
    assert!(world.pc_command(PcCommand::Face { pc: 0, ty: 1, code: 2, chg: 0 }));
    // pc_put on Orca, then remove.
    assert!(world.pc_command(PcCommand::Put { pc: 2, x: 10, y: 0, z: -5 }));
    assert_eq!(world.town_party().rec(2).unwrap().pos[0], ee::from_int(100));
    world.remove(1, 2);
    assert_eq!(world.town_party().members().count(), 1);
    world.remove(2, 2);
    assert_eq!(world.town_party().members().count(), 0);
}

/// Kite put in front of the Recorder's booth and turned to it (`pc_put`,
/// `pc_face`): the Recorder is the command target, and X opens its shop
/// with its greeting; Kite is held until the menu closes.
#[test]
fn talking_to_the_recorder() {
    use piney_event::host::PcCommand;
    use piney_input::Buttons;
    use piney_world::entry::Kind;
    use piney_world::talk::{Shop, TalkRequest};
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    let mut pad = Pad::default();
    for _ in 0..(FADE_FRAMES + HOLD_FRAMES + 100) {
        step(&mut world, &mut pad, Raw::default());
    }
    assert!(world.take_talk().is_empty());
    assert!(world.pc_command(PcCommand::Put { pc: 0, x: -70, y: 360, z: 30 }));
    assert!(world.pc_command(PcCommand::Face { pc: 0, ty: 3, code: 4, chg: 0 }));
    for _ in 0..3 {
        step(&mut world, &mut pad, Raw::default());
    }
    assert_eq!(world.command_target(), Some((Kind::Npc, 4)));
    // cmndSortRoot: the Recorder nearest, 150 away, straight ahead.
    let (kind, code, dist, _) = world.command_sorted()[0];
    assert_eq!((kind, code, ee::f(dist)), (Kind::Npc, 4, 150.0));
    // The target cursor over it and the HUD's names.
    let (x, y, on) = world.tag_pos(Kind::Npc, 4, 0.45, 0).unwrap();
    eprintln!("recorder tag {x} {y} {on}; kite {:?}", world.tag_pos(Kind::Spc, 0, 0.45, 0));
    assert_eq!(on, 1);
    assert!((0..512).contains(&x) && (0..448).contains(&y));
    assert_eq!(world.char_info(Kind::Npc, 4).unwrap().name, b"Recorder");
    let kite = world.char_info(Kind::Spc, 0).unwrap();
    assert_eq!((kite.name.as_slice(), kite.flags), (&b"Kite"[..], 7));
    step(&mut world, &mut pad, Raw { buttons: Buttons::CROSS, ..Raw::default() });
    assert_eq!(world.take_talk(), [TalkRequest::Shop { npc: 4, shop: Shop::Recorder, msg: 0x0063_1a50, line: 0 }]);
    assert!(world.player().acts.pause);
    world.close_menu();
    assert!(!world.player().acts.pause);
    // Asleep (a menu's ccSleepAllThread), the stick moves nobody.
    world.set_asleep(true);
    let at = world.player().body.pos;
    for _ in 0..10 {
        step(&mut world, &mut pad, Raw { ly: 0, ..Raw::default() });
    }
    assert_eq!(world.player().body.pos, at);
    world.set_asleep(false);
    for _ in 0..10 {
        step(&mut world, &mut pad, Raw { ly: 0, ..Raw::default() });
    }
    assert_ne!(world.player().body.pos, at);
}

/// The entry control's set-up: an event's town PC and the Administrator
/// first (each at its marker), the five merchants, then the walking PCs
/// `ccRegisterRandomNpc` chose with Kite and the three entries counted (12):
/// `entry 4 40` makes nothing but counts all the same. Bodies go on the
/// character list in that order, Kite's after them once he has arrived.
#[test]
fn the_entry_controls_set_up() {
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    assert!(world.entry(3, 40, 3, 0));
    assert!(world.entry(4, 29, 5, 0));
    assert!(!world.entry(4, 40, 5, 0), "ccSetMerchant makes only 29 and 158");
    assert!(world.merchants().is_empty() && world.pcs().is_empty());
    let mut pad = Pad::default();
    for _ in 0..(FADE_FRAMES + HOLD_FRAMES + 1) {
        step(&mut world, &mut pad, Raw::default());
    }
    let (at, rot) = world.marker_dummy(3).unwrap();
    let ids: Vec<i32> = world.merchants().iter().map(|m| m.id).collect();
    assert_eq!(ids, [29, 0, 1, 2, 3, 4]);
    assert_eq!((world.merchants()[0].ch.pos, world.merchants()[0].ch.dirc), world.marker_dummy(5).unwrap());
    assert_eq!(world.pcs().len(), 13);
    assert_eq!((world.pcs()[0].row.row, world.pcs()[0].char.pos, world.pcs()[0].char.dirc), (40, at, rot));
    let kinds: Vec<u32> = world.town().base.hits.chars.iter().map(|b| b.kind).collect();
    assert_eq!(kinds, [2; 19]);
    for _ in 0..100 {
        step(&mut world, &mut pad, Raw::default());
    }
    assert_eq!(world.town().base.hits.chars.last().map(|b| b.kind), Some(7));
}

/// Event 2 ("MG0020 TEACH_T") as it drives the characters through the
/// World: the phase 0 pass (`entry 2 2 3 5`, `pc_mode -3 6`), the town's
/// set-up (Kite out of sight under manual control, Orca standing at marker
/// 3), then the play pass (`menu_ban`, Orca facing Kite, `pc_act 0 3`, Kite
/// turning to Orca at 64, Orca joining). `tools/test_evchar_rs.py` checks
/// the same code frame by frame against the game.
#[test]
fn event_two_characters() {
    use piney_event::host::PcCommand;
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    let mut pad = Pad::default();
    assert!(world.entry(2, 2, 3, 5));
    assert!(world.pc_command(PcCommand::Mode { pc: -3, param: 6 }));
    let r = world.spcs().registry;
    assert_eq!((r[0].id, r[0].boot_param, r[1].id, r[1].boot_param), (0, 6, 2, 5));
    assert_eq!(world.party(), [0, -1, -1]);
    for _ in 0..(FADE_FRAMES + HOLD_FRAMES + 1) {
        step(&mut world, &mut pad, Raw::default());
    }
    // The town's set-up: Kite out of sight (act 14, cloak 0) under manual
    // control and off the command list; Orca standing at marker 3 facing
    // its heading, manual, listed, his body on the character list.
    let p = world.player();
    assert_eq!((p.acts.act, p.acts.cloak, p.manual(), p.listed), (act::HIDDEN, 0, true, false));
    let (at, rot) = world.marker_bits(3).unwrap();
    let orca = world.town_party().rec(2).unwrap();
    let ai = orca.ai.as_ref().unwrap();
    assert_eq!((orca.act, ai.manual_sw, orca.listed), (2, true, true));
    assert_eq!(orca.pos, at);
    assert_eq!(ai.g_deg, ee::rad2deg(rot) as u16);
    assert!(orca.hit.sw);
    // The play pass.
    world.menu_ban_party(true);
    let orca = world.town_party().rec(2).unwrap();
    assert!(!orca.listed && !orca.trans_dist);
    assert!(world.pc_command(PcCommand::Face { pc: 2, ty: 2, code: 0, chg: 0 }));
    for _ in 0..80 {
        step(&mut world, &mut pad, Raw::default());
        assert!(!world.player().drawn);
    }
    assert!(world.pc_command(PcCommand::Act { pc: 0, act: 3 }));
    for _ in 0..80 {
        step(&mut world, &mut pad, Raw::default());
    }
    let p = world.player();
    assert_eq!((p.acts.act, p.acts.cloak, p.manual()), (act::IDLE, ee::ONE, true));
    assert!(p.drawn && p.hit_body.sw);
    // Turning to Orca a quarter of the way a frame.
    let before = p.body.dirc[2];
    assert!(world.pc_command(PcCommand::Face { pc: 0, ty: 2, code: 2, chg: 64 }));
    let want = piney_world::event::dirc_to(world.player().body.pos, world.town_party().rec(2).unwrap().pos) as u16;
    assert_eq!(world.player().ai.as_ref().unwrap().g_deg, want);
    for _ in 0..60 {
        step(&mut world, &mut pad, Raw::default());
    }
    let turned = world.player().body.dirc[2];
    assert_ne!(turned, before);
    // It stops 8 short: once ccSetDirc's step is 1, RAD2DEG(DEG2RAD(s + 1))
    // truncates back to s (the game does the same, checked in eemu).
    assert_eq!(ee::rad2deg(turned), want as i16 + 8);
    assert!(world.player().ai.as_ref().unwrap().remote_flag);
    // The pad does nothing under manual control.
    let pos = world.player().body.pos;
    let raw = Raw { ly: 0, ..Raw::default() };
    for _ in 0..10 {
        step(&mut world, &mut pad, raw);
    }
    assert_eq!(world.player().body.pos, pos);
    // Orca joins (the menu's ccParty::AddMember).
    assert_eq!(world.party_add(2), 1);
    assert_eq!(world.party(), [0, 2, -1]);
    assert_eq!(world.town_party().rec(2).unwrap().party_flag, 1);
    // The second menu_ban, now with Orca in slot 1.
    world.menu_ban_party(true);
    // The end: both turned at once.
    assert!(world.pc_command(PcCommand::Turn { pc: 0, dirc: -32768, chg: 0 }));
    assert!(world.pc_command(PcCommand::Turn { pc: 2, dirc: 28672, chg: 0 }));
    assert_eq!(world.player().body.dirc[2], ee::deg2rad(-32768));
    assert_eq!(world.town_party().rec(2).unwrap().dirc[2], ee::deg2rad(28672));
    // menu_clear hands Kite back to the pad; Orca stays as menu_ban found
    // him (manual).
    world.menu_ban_party(false);
    let orca = world.town_party().rec(2).unwrap();
    assert!(!world.player().manual() && orca.ai.as_ref().is_some_and(|a| a.manual_sw));
    assert!(world.player().listed && orca.listed);
}

/// A registered character's menu (Talk / Trade / Gift) in a town: the
/// menu's `EntryAffect(14)` on Orca reaches his `ccFellow::Influence`,
/// whose `ccAI::Greeting` sets his talkFlag and his gDeg toward Kite
/// (`RAD2DEG(atan2f(d.x, -d.y))`), so his AI stands facing Kite; the
/// menu's close (0) clears the flag.
#[test]
fn a_member_spoken_to_stands_facing_kite() {
    use piney_world::entry::Kind;
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    let mut pad = Pad::default();
    assert!(world.entry(2, 2, 3, 5));
    // The town's set-up, then a few frames of his Main (posP, which
    // Greeting measures from, is his place in Kite's frame as the last
    // Main left it).
    for _ in 0..(FADE_FRAMES + HOLD_FRAMES + 10) {
        step(&mut world, &mut pad, Raw::default());
    }
    let talking = |w: &World| w.town_party().rec(2).unwrap().ai.unwrap().talk_flag;
    assert!(!talking(&world));
    world.affect(Kind::Spc, 2, 14);
    step(&mut world, &mut pad, Raw::default());
    let orca = world.town_party().rec(2).unwrap();
    assert!(talking(&world));
    let want = piney_world::event::dirc_to(orca.pos, world.player().body.pos) as u16;
    assert_eq!(orca.ai.unwrap().g_deg, want);
    let pos = orca.pos;
    for _ in 0..30 {
        step(&mut world, &mut pad, Raw::default());
    }
    assert!(talking(&world));
    assert_eq!(world.town_party().rec(2).unwrap().pos, pos);
    world.affect(Kind::Spc, 2, 0);
    step(&mut world, &mut pad, Raw::default());
    assert!(!talking(&world));
}

/// A timed buff Kite brought from a field times out in a town: his
/// stand-in's `CalcReal(dead)` each frame (`ccPlayer::Main`, gcmn
/// 0x005983c8) counts `ccSpcParam.time` down, the buff's `temp` cleared at
/// 0, and the record goes back into the save.
#[test]
fn kites_buff_times_out_in_town() {
    use piney_battle::param::SpcParam;
    let Some((mut iso, archive)) = disc() else { return };
    let mut save = new_game(&mut iso);
    let mut p = SpcParam::from_save(&save.save, 0);
    (p.temp[0], p.time[0]) = (10, 60);
    p.store(&mut save.save, 0);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    let mut pad = Pad::default();
    let buff = |w: &World| {
        let p = SpcParam::from_save(&w.state().save, 0);
        (p.temp[0], p.time[0])
    };
    // The town's tasks start after the fade.
    let mut frames = 0;
    while buff(&world).1 == 60 {
        step(&mut world, &mut pad, Raw::default());
        frames += 1;
        assert!(frames < 400, "the timer never ran");
    }
    let (temp, time) = buff(&world);
    assert_eq!((temp, time), (10, 59));
    for _ in 0..time {
        step(&mut world, &mut pad, Raw::default());
    }
    assert_eq!(buff(&world), (0, 0));
}

/// Kite's first normal attack and his run played one frame a step through
/// `Play::forward_notes`: the notes `NoteProcess` hands on, as the game's
/// own `_AnimateForward` / `NoteProcess` give them (`tools/test_anim.py`
/// checks the model against them).
#[test]
fn kites_notes() {
    let Some((_, archive)) = disc() else { return };
    let file = piney_desktop::assets::SceneFile::read(&archive, "ctu1body").unwrap();
    let run = |name: &str, steps: u32| {
        let mut play = piney_world::pose::Play::new(&file, name).unwrap();
        let mut out = Vec::new();
        for i in 1..=steps {
            let (ended, notes) = play.forward_notes(&file);
            if !notes.is_empty() || ended {
                out.push((i, play.time >> 8, ended, notes));
            }
            if ended {
                break;
            }
        }
        out
    };
    // The hit (0x8005) lands on frame 5, handed on before the swing's sound
    // that precedes it in the file.
    assert_eq!(
        run("ANM_ctu1atc0", 40),
        vec![
            (1, 1, false, vec![(2, 0)]),
            (5, 5, false, vec![(0x8005, 1), (2, 2)]),
            (7, 7, false, vec![(2, 7)]),
            (20, 20, true, vec![]),
        ]
    );
    // The run loops over frames 1..20: footsteps on 7 and 17 every pass.
    assert_eq!(
        run("ANM_ctu1run0", 40),
        vec![
            (7, 7, false, vec![(2, 0)]),
            (17, 17, false, vec![(2, 0)]),
            (27, 7, false, vec![(2, 0)]),
            (37, 17, false, vec![(2, 0)]),
        ]
    );
}

/// `pc_walk_pos` on Kite in Mac Anu (events 21 and 30 walk their
/// characters in a town): his AI under manual control walks him by
/// `MoveP2P` to the goal, stops within 50 of it and ends the command.
#[test]
fn kite_walks_to_an_event_goal() {
    use piney_event::host::PcCommand;
    let Some((mut iso, archive)) = disc() else { return };
    let save = new_game(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    let mut pad = Pad::default();
    for _ in 0..200 {
        step(&mut world, &mut pad, Raw::default());
    }
    let start = world.player().body.pos;
    let tenth = |v: u32| (ee::f(v) / 10.0).round() as i16;
    let (x, y, z) = (tenth(start[0]), tenth(start[1]) - 30, tenth(start[2]));
    assert!(world.pc_command(PcCommand::WalkPos { pc: 0, x, y, z, run: false }));
    let goal = [f32::from(x) * 10.0, f32::from(y) * 10.0];
    let dist = |w: &World| {
        let p = w.player().body.pos;
        ((ee::f(p[0]) - goal[0]).powi(2) + (ee::f(p[1]) - goal[1]).powi(2)).sqrt()
    };
    let before = dist(&world);
    let mut arrived = None;
    for f in 0..300 {
        step(&mut world, &mut pad, Raw::default());
        let ai = world.player().ai.as_ref().unwrap();
        if ai.remote_cmd == 0 {
            arrived = Some(f);
            break;
        }
    }
    let f = arrived.expect("the walk never ended");
    assert!(before > 250.0, "{before}");
    // MoveP2P measures the ground distance less the body's radius, which
    // running widens (45 standing, 72.5 on the run): he stops once that is
    // under 50, about 107 short of the goal here.
    let after = dist(&world);
    assert!(after < 125.0 && before - after > 150.0, "{after} from the goal after {f} frames");
    assert!(world.player().ai.as_ref().unwrap().remote_flag);
    assert!(!world.player().body.move_flag);
}

/// Mac Anu after the arrival with the save's Controller setting (`camType`)
/// `cam`, then `raw` held for 20 frames: the town camera's turn (`deg[0]`),
/// tilt (`deg[1]`) and distance before and after.
fn camera_under(iso: &mut Iso, archive: &Arc<Archive>, cam: u8, raw: Raw) -> ([i16; 2], f32, [i16; 2], f32) {
    let mut save = new_game(iso);
    save.save.set_u8(piney_data::save::offset::CAM_TYPE, cam);
    let mut world = World::enter(iso, archive.clone(), save).unwrap();
    assert_eq!(world.camera().scheme, piney_world::camera::Scheme::new(i32::from(cam)));
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    for _ in 0..FADE_FRAMES + HOLD_FRAMES + 150 {
        step(&mut world, &mut pad, still);
    }
    let t = &world.camera().tcam;
    let before = (t.deg, ee::f(t.dist));
    for _ in 0..20 {
        step(&mut world, &mut pad, Raw { analog: true, ..raw });
    }
    let t = &world.camera().tcam;
    (before.0, before.1, t.deg, ee::f(t.dist))
}

/// The Controller setting drives the camera as `setCameraCtrlType` sets
/// it up (0 A-1, 1 A-2, 2 B-1, 3 B-2):
/// - R1: A turns the camera (A-2 the other way from A-1) and keeps its
///   distance; B zooms in and does not turn.
/// - The right stick pushed up: A zooms; B tilts.
#[test]
fn the_controller_scheme_drives_the_camera() {
    let Some((mut iso, archive)) = disc() else { return };
    let r1 = Raw { lx: 128, ly: 128, rx: 128, ry: 128, buttons: piney_input::Buttons::R1, ..Raw::default() };
    let up = Raw { lx: 128, ly: 128, rx: 128, ry: 0, ..Raw::default() };
    let turn = |(b, _, a, _): ([i16; 2], f32, [i16; 2], f32)| a[0].wrapping_sub(b[0]);
    let zoom = |(_, b, _, a): ([i16; 2], f32, [i16; 2], f32)| a - b;
    let tilt = |(b, _, a, _): ([i16; 2], f32, [i16; 2], f32)| a[1].wrapping_sub(b[1]);

    let a1 = camera_under(&mut iso, &archive, 0, r1);
    let a2 = camera_under(&mut iso, &archive, 1, r1);
    let b1 = camera_under(&mut iso, &archive, 2, r1);
    assert!(turn(a1) != 0 && zoom(a1) == 0.0, "A-1, R1 turns: {a1:?}");
    assert!(turn(a2).signum() == -turn(a1).signum(), "A-2 turns the other way: {a1:?} {a2:?}");
    assert!(turn(b1) == 0 && zoom(b1) < 0.0, "B-1, R1 zooms in: {b1:?}");

    let a1 = camera_under(&mut iso, &archive, 0, up);
    let b1 = camera_under(&mut iso, &archive, 2, up);
    assert!(zoom(a1) != 0.0 && tilt(a1) == 0, "A-1, the stick up zooms: {a1:?}");
    assert!(tilt(b1) != 0, "B-1, the stick up tilts: {b1:?}");
}
