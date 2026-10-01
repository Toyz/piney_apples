//! Dun Loireag on the disc: the town entered from a save whose `lastTown`
//! is 1, Kite's arrival at its start, walking, its collision, the Chaos
//! Gate, the merchants and walking PCs of town 1, its sky, clouds and lens
//! flare. Skipped without `work/infection/infection.iso`.
//! `tools/test_town02_rs.py` checks the town's draw against the game's own
//! code frame by frame.

use std::path::Path;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_data::save::offset;
use piney_draw::Cmd;
use piney_input::{Pad, Raw};
use piney_world::hit::{FLOOR, HitModel, Hits, LAND_MASK, WALL};
use piney_world::motion::act;
use piney_world::{FADE_FRAMES, HOLD_FRAMES, Phase, SaveState, World, ee};

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

/// A new game's save (`SaveState::fresh`, `ccSaveData::NewGame(0)`) whose
/// last town is Dun Loireag.
fn in_dun_loireag(iso: &mut Iso) -> SaveState {
    let tables = piney_demo::newgame::NewGameTables::of(iso.volume().unwrap());
    let mut state = SaveState::fresh();
    piney_demo::newgame::new_game(&mut state.save, 0, &tables, piney_demo::newgame::SAVE_VA);
    state.save.set_u8(offset::LAST_TOWN, 1);
    state
}

fn step(world: &mut World, pad: &mut Pad, raw: Raw) -> piney_draw::Frame {
    pad.read(&raw);
    world.step(pad)
}

/// `HIT_sr2town1hit`, town02's one Hit chunk: its floors and walls, and
/// the ground under the start.
#[test]
fn dun_loireag_collision_mesh() {
    let Some((_, archive)) = disc() else { return };
    for stem in ["town02", "town02d"] {
        let c = Ccs::parse(archive.inflate_named(stem).unwrap()).unwrap();
        let models = HitModel::read(piney_data::volume::Volume::Inf, &c).unwrap();
        assert_eq!(models.len(), 1, "{stem}");
        assert_eq!(c.object_name(models[0].object), Some("HIT_sr2town1hit"));
        let polys = &models[0].polys;
        let floors = polys.iter().filter(|p| p.att & FLOOR != 0).count();
        let walls = polys.iter().filter(|p| p.att & WALL != 0).count();
        assert_eq!((polys.len(), floors, walls), (1356, 354, 1002), "{stem}");
        let mut hits = Hits::new(piney_data::volume::Volume::Inf, models);
        let (start, _) = piney_world::start_position(1);
        // The start stands on the gate plaza's floor, at 0 to the float.
        let z = ee::f(hits.land(start, LAND_MASK));
        assert!(hits.nearest.att & FLOOR != 0 && z.abs() < 0.001, "{z}");
    }
}

/// `ccSetupGameCtrl` into Dun Loireag: the start (0, 3500, 0) facing
/// south, the arrival, the Chaos Gate at `DMY_gate`, the six merchants of
/// town 1 and the walking PCs; the town draws its sky, clouds and lens
/// flare; then Kite runs.
#[test]
fn arrival_in_dun_loireag_then_walking() {
    let Some((mut iso, archive)) = disc() else { return };
    let save = in_dun_loireag(&mut iso);
    let mut world = World::enter(&mut iso, archive, save).unwrap();
    assert_eq!(world.town().base.no, 1);
    assert_eq!(world.town().base.file.stem, "town02");
    assert_eq!(world.clear_colour(), Some([0x80, 0xc0, 0xf0]));
    let mut pad = Pad::default();
    for _ in 0..FADE_FRAMES + HOLD_FRAMES {
        step(&mut world, &mut pad, Raw::default());
    }
    assert_eq!(world.phase(), Phase::Play(0));
    let p = world.player();
    assert_eq!((p.acts.act, ee::f(p.body.pos[0]), ee::f(p.body.pos[1])), (act::ARRIVE, 0.0, 3500.0));
    // The gate at DMY_gate, 700 north of the start.
    assert_eq!(world.gate().pos[..3], [0, ee::k(4200.0), 0]);
    let mut sprites = 0;
    for i in 0..120 {
        let f = step(&mut world, &mut pad, Raw::default());
        if i > 2 {
            assert!(f.cmds.iter().any(|c| matches!(c, Cmd::Model(_))), "the town draws");
        }
        sprites += world.take_town_sprites().len();
    }
    assert_eq!(world.player().acts.act, act::IDLE);
    assert!(sprites > 0, "no clouds or flares drawn");
    // ccSetMerchant(0) in town 1: npcTbl rows 5-10.
    let ids: Vec<i32> = world.merchants().iter().map(piney_world::entry::Npc::code).collect();
    assert_eq!(ids, [5, 6, 7, 8, 9, 10]);
    assert_eq!(world.pcs().len(), 15);
    let d = world.town().class::<piney_world::town02::DunLoireag>().unwrap();
    assert_eq!(d.clouds.len(), 25);
    // Stick up: he runs south and stays on the ground.
    let start = world.player().body.pos;
    for _ in 0..40 {
        step(&mut world, &mut pad, Raw { ly: 0, ..Raw::default() });
        assert_eq!(world.player().acts.act, act::RUN);
    }
    let p = world.player().body.pos;
    assert!(ee::f(p[1]) < ee::f(start[1]) - 500.0, "ran to {:?}", p.map(ee::f));
    // Let go: he stops at once.
    step(&mut world, &mut pad, Raw::default());
    assert_eq!(world.player().acts.act, act::IDLE);
}

/// `markerEvTbl`'s markers in town02: the gate (0) and the event markers
/// its file has.
#[test]
fn dun_loireag_markers() {
    let Some((mut iso, archive)) = disc() else { return };
    let save = in_dun_loireag(&mut iso);
    let world = World::enter(&mut iso, archive, save).unwrap();
    let (gate, _) = world.marker_bits(0).unwrap();
    assert_eq!(gate, world.gate().pos);
    // DMY_marker_ev01-06, 20 and 21, DMY_marker71 and DMY_marker30.
    let found: Vec<i16> = (0..33).filter(|&n| world.marker_bits(n).is_some()).collect();
    assert_eq!(found, [0, 1, 2, 3, 4, 5, 6, 20, 21, 31, 32]);
}
