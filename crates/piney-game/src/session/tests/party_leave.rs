//! Members who leave by an event's `pc_act 5` outside a town: event 18's
//! end in area 19's dungeon (issue #16).

use std::path::{Path, PathBuf};

use piney_event::host::PcCommand;
use piney_event::state::ScriptSave as _;

use super::survey::{StoryPilot, event_flag};
use super::town_return::{back_to_town, in_town, to_the_field};
use super::*;

/// Mia's and Elk's `charTbl` rows.
const MIA: i32 = 1;
const ELK: i32 = 10;

/// Event 18 from story 18's start, blocks 0-17 run (Mia's and Elk's
/// addresses given, area 19's gate hacked, its dungeon entered,
/// `eventStatus[9]` 1), Elk and Mia in the party, in the room of event
/// point 1 (blocks 18-20), taken there as `room_point` goes.
fn event_18_at_point_1(iso: &Path) -> Option<Session> {
    let mut s = story_session_on("infection", 18, |start| {
        let save = &mut start.state.save;
        save.update_flags(18, |f| f | ((1 << 18) - 1));
        save.set_u8(offset::EVENT_STATUS + 9, 1);
        for at in [offset::PARTY_MEMBER_FLAG, offset::PARTY_MEMBER_CALL] {
            save.set_i32(at, save.i32(at) | 1 << MIA | 1 << ELK);
        }
        let mut scene = piney_world::area::Scene::log_in(save);
        scene.change_scene(2, 0, 19, 0, 0, 0, save);
        let wm = crate::area::story_world_man(&mut Iso::open(iso).unwrap(), 19, false).ok();
        let spcs = Some(crate::start::party_of(&[ELK, MIA]));
        start.at = Resume::World(Box::new(InWorld { scene, world_man: wm, spcs }));
    })?;
    let mut pad = Pad::default();
    let (mut room, mut away) = (false, false);
    for f in 0..2400u64 {
        pad.read(&story_player(&s, f));
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let playing = matches!(a.world().phase(), piney_world::Phase::Play(k) if k > 10);
        away |= room && !playing;
        if !playing {
            continue;
        }
        if room {
            if away {
                return Some(s);
            }
            continue;
        }
        let Some(p) = a.vm().and_then(|vm| vm.mng.point(1)) else { continue };
        let (floor, block) = (i32::from(p.floor), i32::from(p.block));
        if (floor, block) == (a.world().scene().floor, a.world().scene().block) {
            return Some(s);
        }
        if let Some(vm) = a.vm_mut() {
            vm.disable();
        }
        room = a.world_mut().room_select(floor, block);
    }
    panic!("event point 1's room never played: {}", Mode::title(&s));
}

/// The party and its registered ids where `s` is.
fn party(s: &Session) -> ([i32; 3], Vec<i32>) {
    let spcs = match &s.stage {
        Stage::World(w) => w.world().spcs(),
        Stage::Area(a) => a.world().spcs(),
        _ => panic!("not in The World: {}", Mode::title(s)),
    };
    (spcs.party(), spcs.registry.iter().map(|r| r.id).filter(|&id| id >= 0).collect())
}

/// Where the room's magic portal stands while it is switched on.
fn portal(w: &piney_world::field_world::FieldWorld) -> Option<[u32; 4]> {
    let c = w.combat();
    c.ctrl
        .list(piney_battle::entry::Kind::Circle)
        .into_iter()
        .find(|&i| c.ctrl.entry_obj(i).is_some_and(|o| o.obj_flag))
        .map(|i| c.scene.chars[i].pos)
}

/// Issue #16: event 18 played to its end. Block 20's `pc_act 1 5` and
/// `pc_act 10 5` are remote command 5 (`ccAI::ManualControl`, gcmn
/// 0x00580ef0), whose `resignParty` (0x0059d150) takes each out of his slot
/// at once, so back in Mac Anu, in a field and in Mac Anu again Kite is
/// alone. The field's frames dropped the call: Mia went by her exit, but
/// Elk's came after `scene 0 0`, so he kept his slot with no registry entry
/// (a face with no name in town, nobody in the field).
#[test]
fn event_18_leaves_kite_alone() {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let Some(mut s) = event_18_at_point_1(&iso) else { return };
    assert_eq!(party(&s).0, [0, ELK, MIA]);
    s.console("god");
    let (mut pad, mut pilot) = (Pad::default(), StoryPilot::default());
    for f in 0..8000u64 {
        // Block 20 waits on the room's magic portal and its Data Bug
        // (`no_active`): once block 19's scene is over, Kite put on the
        // portal, the Data Bug's protect broken for the pilot's Data
        // Drain, and what it leaves felled.
        let flags = event_flag(&mut s, 18).unwrap_or(0);
        if flags & 1 << 19 != 0 && f.is_multiple_of(30) {
            if let Stage::Area(a) = &mut s.stage
                && let Some(at) = portal(a.world())
            {
                let tenth = |v: u32| (f32::from_bits(v) / 10.0).round() as i16;
                a.world_mut().pc_command(PcCommand::Put { pc: 0, x: tenth(at[0]), y: tenth(at[1]), z: tenth(at[2]) });
            }
            s.console("infection 0");
            s.console("protect");
            s.console("kill");
        }
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if in_town(&s).is_some() {
            break;
        }
    }
    assert!(in_town(&s).is_some(), "never back in Mac Anu: {}", Mode::title(&s));
    assert_eq!(party(&s), ([0, -1, -1], vec![0]), "back in Mac Anu");
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(w.ui().ctrl.face_tex[1..3], [-1, -1], "the faces in Mac Anu");
    to_the_field(&mut s, &iso);
    assert_eq!(party(&s), ([0, -1, -1], vec![0]), "in the field");
    back_to_town(&mut s);
    assert_eq!(party(&s), ([0, -1, -1], vec![0]), "in Mac Anu again");
}

/// Piros along in a field (worklog 370): `ccSndSQLoad(3)` asks
/// `checkPartyMenberNum(8)` over `ccPartyManager`'s ids, which the scene
/// change keeps (the members' destructors clear the slots' characters, not
/// the ids), and plays `piroshi`, the twelfth field table, whatever the
/// field. Area 19's field (model 0) with Kite and Piros.
#[test]
fn piros_along_plays_his_field_music() {
    use crate::piros::PIROS;
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let Some(mut s) = story_session_on("infection", 18, |start| {
        let save = &mut start.state.save;
        for at in [offset::PARTY_MEMBER_FLAG, offset::PARTY_MEMBER_CALL] {
            save.set_i32(at, save.i32(at) | 1 << PIROS);
        }
        let mut scene = piney_world::area::Scene::log_in(save);
        scene.change_scene(1, 0, 19, -1, -1, -1, save);
        let wm = crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), 19, false).ok();
        let spcs = Some(crate::start::party_of(&[PIROS]));
        start.at = Resume::World(Box::new(InWorld { scene, world_man: wm, spcs }));
    }) else {
        return;
    };
    let mut pad = Pad::default();
    let mut loads = Vec::new();
    for f in 0..120u64 {
        pad.read(&story_player(&s, f));
        s.step(&pad);
        loads.extend(s.take_events().into_iter().filter_map(|e| match e {
            Event::SqLoad(c) => Some(c),
            _ => None,
        }));
    }
    assert!(
        matches!(loads[..], [piney_audio::SqContext::Field { piros: true, .. }]),
        "the field's bank with Piros along: {loads:?}"
    );
    let tables = piney_data::sound::tables_of(piney_data::volume::Volume::Inf);
    assert!(loads[0].pick(tables).row.is_some(), "piroshi names no bank");
}
