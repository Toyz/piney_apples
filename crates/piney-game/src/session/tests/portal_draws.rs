//! A dungeon portal's group (issue #52's follow-up): area 16's dungeon,
//! Hideous Someone's Giant. Its portals stand where the area's seed puts
//! them, and its three registered rows come from the area's rank; what an
//! opened portal gives is drawn from `ccRand`, which each scene's set-up
//! seeds anew from the frames since power-on (`ccInitRand`).

use piney_battle::entry::Kind;
use piney_event::host::PcCommand;
use piney_world::area::Scene;
use piney_world::{HOLD_FRAMES, Phase};

use super::area15::{disc, playing, wait};
use super::*;

/// The giant's first room with a portal: floor 0 (the first floor), block 1.
const ROOM: (i32, i32) = (0, 1);

/// A new game put in area 16's dungeon (`ChangeScene(2, .., 16, 0, 0, 0)`)
/// with the console `before` frames past power-on.
fn in_giants_dungeon(before: u32) -> Option<Session> {
    let (iso, archive) = disc()?;
    let mut d = Iso::open(&iso).unwrap();
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    let mut scene = Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 16, false).unwrap();
    scene.change_scene(2, scene.town, 16, 0, 0, 0, &mut state.save);
    let mut s = Session::bare(iso, archive, false, None, false);
    s.sys_frames = before;
    s.resume_in(state, None, Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs: None }))).unwrap();
    Some(s)
}

/// The world's phase as the next step begins, in a field or a town.
fn phase(s: &Session) -> Option<Phase> {
    match &s.stage {
        Stage::Area(a) => Some(a.world().phase()),
        Stage::World(w) => Some(w.world().phase()),
        _ => None,
    }
}

/// Steps until the scene's set-up reaches `ccInitRand` (the step that
/// begins at the hold's last frame): `ccSys+0x358` in that step.
fn step_to_init_rand(s: &mut Session) -> u32 {
    for _ in 0..600 {
        let at = phase(s) == Some(Phase::Hold(HOLD_FRAMES - 1));
        wait(s, 1);
        if at {
            return s.sys_frames;
        }
    }
    panic!("no set-up: {}", Mode::title(s));
}

/// Steps until the world plays in `room`.
fn play_in(s: &mut Session, room: (i32, i32)) {
    for _ in 0..600 {
        if playing(s).is_some_and(|w| (w.scene().floor, w.scene().block) == room) {
            return;
        }
        wait(s, 1);
    }
    panic!("not playing in {room:?}: {}", Mode::title(s));
}

/// What an opened portal gave.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Gift {
    /// The enemies' rows.
    Foes(Vec<i32>),
    /// A treasure box (`(ccRand() & 7) == 3`), its gimmick row.
    Box(i32),
}

/// The rows of the enemies standing in the dungeon.
fn foes(s: &Session) -> Vec<i32> {
    let Stage::Area(a) = &s.stage else { return vec![] };
    let c = a.world().combat();
    c.ctrl.list(Kind::Enemy).into_iter().filter_map(|i| c.foes.get(i)?.as_ref()).map(|e| e.ent.id).collect()
}

/// The gimmicks on the entry control's list: (object, row).
fn gimmicks(s: &Session) -> Vec<(usize, i32)> {
    let Stage::Area(a) = &s.stage else { return vec![] };
    let c = a.world().combat();
    let gims = c.ctrl.list(Kind::Gimmick).into_iter();
    gims.filter_map(|i| Some((i, c.ctrl.entry_obj(i)?.ent.id))).collect()
}

/// The giant's dungeon `before` frames past power-on: to `ROOM`, `stand`
/// frames there, then Kite put on its portal: what the portal gave.
fn giants_portal(before: u32, stand: u32) -> Option<Gift> {
    let mut s = in_giants_dungeon(before)?;
    play_in(&mut s, (0, 0));
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    assert!(a.world_mut().room_select(ROOM.0, ROOM.1));
    play_in(&mut s, ROOM);
    wait(&mut s, stand);
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    let c = a.world().combat();
    let at = c
        .ctrl
        .list(Kind::Circle)
        .into_iter()
        .find(|&i| c.ctrl.entry_obj(i).is_some_and(|o| o.obj_flag))
        .map(|i| c.scene.chars[i].pos)
        .expect("the room's portal");
    let tenth = |v: u32| (f32::from_bits(v) / 10.0).round() as i16;
    let before = gimmicks(&s);
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    a.world_mut().pc_command(PcCommand::Put { pc: 0, x: tenth(at[0]), y: tenth(at[1]), z: tenth(at[2]) });
    // ccMagicCircle::main: acts 1 and 2 take 52 frames to the gift.
    for _ in 0..200 {
        wait(&mut s, 1);
        let Stage::Area(a) = &s.stage else { unreachable!() };
        let c = a.world().combat();
        let open = c.ctrl.list(Kind::Circle).into_iter().all(|i| c.ctrl.entry_obj(i).is_none_or(|o| !o.obj_flag));
        if open || !foes(&s).is_empty() {
            break;
        }
    }
    let made: Vec<i32> = gimmicks(&s).into_iter().filter(|g| !before.contains(g)).map(|g| g.1).collect();
    Some(match made[..] {
        [id] => Gift::Box(id),
        _ => Gift::Foes(foes(&s)),
    })
}

/// The same walk to the same portal, the console a few frames further on
/// from power-on: each set-up's `ccInitRand` seeds `ccRand` from
/// `ccSys+0x358`, so the portal's draws (a box 1 in 8, the first row, the
/// count, the rows after) come out otherwise. With a generator restarting
/// at each scene, as the port had it, every walk gave rows 106 and 56.
#[test]
fn a_portals_group_moves_with_the_frames_since_power_on() {
    let mut groups = Vec::new();
    for before in [0, 1, 2, 3, 500, 12_345] {
        let Some(g) = giants_portal(before, 37) else { return };
        if let Gift::Foes(rows) = &g {
            assert!(!rows.is_empty() && rows.iter().all(|r| [106, 56, 10].contains(r)), "{before}: {g:?}");
        }
        eprintln!("{before} frames: {g:?}");
        groups.push(g);
    }
    groups.sort();
    groups.dedup();
    assert!(groups.len() >= 3, "{groups:?}");
}

/// `ccSys+0x358` as the set-ups' `ccInitRand` reads it: the step at the
/// hold's end, after the fade out (the first arrival, the town) or with
/// it drawn by the scene before (a room's door). `ccRandS` moves on by
/// it, from 0 at power-on, and both go on to the next mode.
#[test]
fn each_set_up_seeds_ccrand_from_the_frames_at_its_step() {
    let Some(mut s) = in_giants_dungeon(4321) else { return };
    let at = step_to_init_rand(&mut s);
    play_in(&mut s, (0, 0));
    let Stage::Area(a) = &mut s.stage else { unreachable!() };
    assert_eq!(a.world().rand_count(), at, "the first arrival");
    let mut rnds = 0;
    piney_battle::rand::init_rand(at, &mut rnds);
    assert_eq!(a.world().state_out().rand_s, rnds);
    // The next mode (the desktop's staff roll) draws on from the same ccRand.
    let (mut out, mut cc) = (a.world().state_out().cc, a.world().combat().cc.clone());
    for _ in 0..700 {
        assert_eq!(out.rand(), piney_battle::rand::Rng::rand(&mut cc));
    }
    assert!(a.world_mut().room_select(ROOM.0, ROOM.1));
    let at = step_to_init_rand(&mut s);
    play_in(&mut s, ROOM);
    let Stage::Area(a) = &s.stage else { unreachable!() };
    assert_eq!(a.world().rand_count(), at, "through a door");
    s.console("town 0");
    let at = step_to_init_rand(&mut s);
    let Stage::World(w) = &s.stage else { panic!("not in Mac Anu: {}", Mode::title(&s)) };
    assert_eq!(w.world().rand_count(), at, "in Mac Anu");
}
