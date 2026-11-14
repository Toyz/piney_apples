//! The entry control's NPC list (`g_entCtrl`'s fourth, one `Npcs` in every
//! area). In a town `ccEntryEventMng` (INF main 0x001b62e0, MUT 0x001cb990,
//! OUT 0x001c2060, QUA 0x001c96e0) enters the event's NPCs (types 3 and 4)
//! in its `entry[]` walk, then `ccSetMerchant(0)`, `ccSetDog`,
//! `ccSetChibiGuso` and `ccEntryRandomNpc`; `ccThEntryCtrl` (gcmn
//! 0x00431970) steps the list in that order and each NPC draws in its main.

use piney_event::ScriptSave as _;
use piney_event::ir::Op;
use piney_world::systems::npcs::Npc;

use super::*;

/// Dun Loireag (`game.town`).
const DUN_LOIREAG: u8 = 1;

/// Infection's event `event` with blocks `0..=last` run and its status 1,
/// Kite logged in to `town`, held until the town's tasks have run 30
/// frames. None without the disc.
fn logged_in(event: i32, last: usize, town: u8) -> Option<Session> {
    let mut s = story_session_with(event, |start| {
        start.state.save.update_flags(event, |f| f | ((1u64 << (last + 1)) - 1));
        start.play_ops(event, last, &[Op::StatusSet { index: 0, num: 1 }]);
        start.state.save.set_u8(offset::LAST_TOWN, town);
        let scene = piney_world::area::Scene::log_in(&mut start.state.save);
        start.at = Resume::World(Box::new(InWorld { scene, world_man: None, spcs: None }));
    })?;
    let playing = |s: &Session| matches!(&s.stage, Stage::Area(w) if w.in_town() && matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    super::area15::hold(&mut s, 128, 128, 900, playing);
    assert!(playing(&s), "event {event}: not in town {town}: {}", Mode::title(&s));
    Some(s)
}

/// Where an NPC comes in `ccEntryEventMng`'s order: the event's, the
/// merchants, the dogs, the Grunties, the walking PCs.
fn rank(n: &Npc) -> u8 {
    match n {
        Npc::Pc(p) if p.npc_num < 0 => 0,
        Npc::Merchant(_) => 1,
        Npc::Dog(_) => 2,
        Npc::Grunty(_) => 3,
        Npc::Pc(_) => 4,
    }
}

/// Infection's event 24 (ML0130), block 4 in Dun Loireag: `entry 3 82 4`.
/// Its PC is first on the list, before every merchant, dog, Grunty and
/// walking PC, so it steps and draws first; the rest in their set-up's
/// order.
#[test]
fn an_events_pc_steps_and_draws_before_the_towns_npcs() {
    let Some(s) = logged_in(24, 3, DUN_LOIREAG) else { return };
    let Stage::Area(w) = &s.stage else { unreachable!() };
    let npcs = town_cast(w.world()).npcs();
    let ranks: Vec<u8> = npcs.iter().map(rank).collect();
    let first = npcs.iter().next().expect("an NPC list");
    assert_eq!((rank(first), first.core().code()), (0, 82), "the event's PC first: {ranks:?}");
    assert!(ranks.is_sorted(), "not in ccEntryEventMng's order: {ranks:?}");
    for (k, what) in [(1, "merchants"), (2, "dogs"), (4, "walking PCs")] {
        assert!(ranks.contains(&k), "no {what}: {ranks:?}");
    }
}

/// Infection's side event 59 (S109) in story area 33's second dungeon:
/// blocks 0-16 run, the party put in the room of event point `point`
/// (`WORLD_MAN::SetEventData`'s rooms, read from the dungeon itself), held
/// until its tasks run. None without the disc.
fn in_area_33(point: i32) -> Option<Session> {
    let iso = crate::check_image("infection");
    let mut room = (0, 0);
    for pass in 0..2 {
        let mut s = story_session_with(22, |start| {
            start.state.save.update_flags(59, |f| f | ((1u64 << 17) - 1));
            let save = &mut start.state.save;
            let mut scene = piney_world::area::Scene::log_in(save);
            scene.change_scene(2, 1, 33, 1, room.0, room.1, save);
            let wm = crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), 33, false).ok();
            start.at = Resume::World(Box::new(InWorld { scene, world_man: wm, spcs: None }));
        })?;
        let playing = |s: &Session| matches!(&s.stage, Stage::Area(w) if !w.in_town() && matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 2));
        super::area15::hold(&mut s, 128, 128, 900, playing);
        assert!(playing(&s), "not in area 33's dungeon: {}", Mode::title(&s));
        if pass == 1 {
            return Some(s);
        }
        let Stage::Area(a) = &s.stage else { unreachable!() };
        let data = a.world().area().event_data(2).expect("a story dungeon's event data");
        let p = data.points.iter().find(|p| p[2] == point).expect("the event point");
        room = (p[0], p[1]);
    }
    None
}

/// Event 59's blocks 17 and 18 at event point 5 (floor 3, room 2): `entry
/// 3 87-90` puts four PCs in the dungeon, then `npc_walk_dir` walks each
/// 60 on. `rtpcCheckNote` (gcmn 0x00509160) is the PC anm's note callback
/// wherever it animates, so each step's sound comes out where the PC
/// stands (`ccSeSetParamPC`), as in a town.
#[test]
fn a_dungeons_event_pcs_sound_their_steps() {
    const PCS: [i32; 4] = [87, 88, 89, 90];
    let Some(mut s) = in_area_33(5) else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut steps = 0;
    for n in 0..3000u32 {
        let Stage::Area(a) = &s.stage else { break };
        if a.in_town() {
            break;
        }
        pad.read(&if n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        s.step(&pad);
        let events = s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        let Some(npcs) = a.world().cast().npcs() else { continue };
        let at: Vec<_> = PCS.iter().filter_map(|&c| npcs.get_npc(c)).map(|p| p.core().ch.pos).collect();
        // A step's sound has its note (`ccSeSetParamPC`); the transfer's
        // as they go (`npc_act 4`) has none.
        steps +=
            events.iter().filter(|e| matches!(e, Event::Se3d { pos, note: Some(_), .. } if at.contains(pos))).count();
    }
    assert!(steps > 0, "no step of the event's PCs sounded");
}
