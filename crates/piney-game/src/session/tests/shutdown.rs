//! Issue #70: Mutation's system shutdown after Innis. Lios's forced
//! shutdown empties Carmina Gadelica alone: event 107's block 23 (status
//! 3, back in town 2) and event 108's block 6 (status 1, the next visit)
//! run `regist_npc16`, `eventMng.registNpcNum = 16`. `ccRegisterEventMng`
//! (MUT main 0x001cc480) adds one per town NPC entry and hands the count,
//! with `ccSpcManager.registryNum`, to `ccRegisterRandomNpc` (0x001ccdf0),
//! which draws `16 - reserved - fewer` walkers: none. Other towns walk.

use piney_event::ScriptSave as _;
use piney_event::ir::Op;

use super::*;

/// `charTbl` row 15.
const BLACK_ROSE: i32 = 15;
/// Carmina Gadelica and Mac Anu (`game.town`).
const CARMINA_GADELICA: u8 = 2;
const MAC_ANU: u8 = 0;

/// Mutation's event `event` with blocks `0..=last` run and its status at
/// `status`, Kite and BlackRose logged in to `town`, held until the town's
/// tasks have run 30 frames. None without the disc.
fn logged_in(event: i32, last: usize, status: i16, town: u8) -> Option<Session> {
    let mut s = story_session_on("mutation", event, |start| {
        start.state.save.update_flags(event, |f| f | ((1u64 << (last + 1)) - 1));
        start.play_ops(event, last, &[Op::StatusSet { index: 0, num: status }]);
        start.state.save.set_u8(offset::LAST_TOWN, town);
        let scene = piney_world::area::Scene::log_in(&mut start.state.save);
        let spcs = Some(crate::start::party_of(&[BLACK_ROSE]));
        start.at = Resume::World(Box::new(InWorld { scene, world_man: None, spcs }));
    })?;
    let mut pad = Pad::default();
    let playing = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    for _ in 0..900 {
        if playing(&s) {
            break;
        }
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
    assert!(playing(&s), "event {event}: not in town {town}: {}", Mode::title(&s));
    Some(s)
}

/// The rows of the town's PCs (walking and the events'), sorted.
fn pc_rows(s: &Session) -> Vec<usize> {
    let Stage::World(w) = &s.stage else { panic!("not in a town: {}", Mode::title(s)) };
    let mut rows: Vec<usize> = w.world().pcs().iter().map(|p| p.row.row).collect();
    rows.sort_unstable();
    rows
}

/// Back in Carmina Gadelica after Innis ("Where's everyone else?"):
/// no PC walks, the Administrator (row 29) stands at marker 5.
#[test]
fn carmina_gadelica_is_empty_after_innis() {
    let Some(s) = logged_in(107, 22, 3, CARMINA_GADELICA) else { return };
    let Stage::World(w) = &s.stage else { unreachable!() };
    // The registry's 2 (Kite, BlackRose) are added at the draw.
    assert_eq!(w.vm().map(|vm| vm.mng.regist_npc_num), Some(17), "16 and the Administrator");
    assert!(w.world().merchants().iter().any(|m| m.id == 29), "the Administrator");
    assert_eq!(pc_rows(&s), Vec::<usize>::new(), "PCs in Carmina Gadelica after the shutdown");
}

/// The next visit (event 108, status 1): the six PCs the event puts at
/// markers 7-12 (rows 122-127) and no walker.
#[test]
fn carmina_gadelica_has_only_the_events_pcs_after_the_shutdown() {
    let Some(s) = logged_in(108, 1, 1, CARMINA_GADELICA) else { return };
    assert_eq!(pc_rows(&s), (122..=127).collect::<Vec<usize>>());
}

/// Mac Anu at the same time has no `regist_npc16`: its walkers are drawn
/// as on any visit, 16 less Kite and BlackRose (server 0 has none fewer).
#[test]
fn mac_anu_walks_after_the_shutdown() {
    let Some(s) = logged_in(108, 1, 1, MAC_ANU) else { return };
    let Stage::World(w) = &s.stage else { unreachable!() };
    assert_eq!(w.vm().map(|vm| vm.mng.regist_npc_num), Some(0));
    assert_eq!(pc_rows(&s).len(), 14, "Mac Anu's walkers: {:?}", pc_rows(&s));
}

/// Shots of event 107's tour of the empty town (block 24's camera
/// moves), every 100 frames, into `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/i70): `cargo test --release -p piney-game
/// shutdown_shots -- --ignored --nocapture`.
#[test]
#[ignore]
fn shutdown_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i70".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = logged_in(107, 22, 3, CARMINA_GADELICA) else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    let mut pad = Pad::default();
    for f in 1..=600u32 {
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        let frame = s.step(&pad);
        s.take_events();
        if f % 100 != 0 {
            continue;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(&s));
        g.render(&frame);
        let (wd, ht) = g.target_size();
        let path = format!("{dir}/carmina-{f:03}.png");
        std::fs::write(&path, piney_gs::png::encode(wd, ht, &g.read_back())).unwrap();
        println!("{path} ({} PCs)", pc_rows(&s).len());
    }
}
