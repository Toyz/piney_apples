//! Issue #60: Mutation's event 101 (M201) trades Kite's Book of Law for the
//! Book of Absolute in Dun Loireag (town 1): block 25 registers `entry 4 29`
//! at marker 6, and block 27 has the Shop Keeper (`npcTbl` row 29, the
//! Administrator's `ccMerchan`) speak. `ccEntryEventMng` makes a type 4
//! through `ccSetMerchant(29)` and puts it at `markerEvTbl[6]`'s dummy; the
//! port's town refused type 4, so no one stood there.

use piney_data::volume::Volume;
use piney_event::ScriptSave as _;
use piney_event::ir::Op;
use piney_world::merchant::Merchant;

use super::area15::{hold, hold_frame};
use super::*;

/// The event, its arrival block, the Shop Keeper's row and marker.
const EVENT: i32 = 101;
const ARRIVAL: usize = 25;
const SHOP_KEEPER: i32 = 29;
const MARKER: i16 = 6;
/// `charTbl` row 15.
const BLACK_ROSE: i32 = 15;

/// Mutation's event 101 as block 24 leaves it (blocks 0-24 run,
/// `eventStatus[0]` 1), Kite and BlackRose logged in to Dun Loireag, held
/// until the town's tasks have run 30 frames. None without the disc.
fn arrived() -> Option<Session> {
    let mut s = story_session_on("mutation", EVENT, |start| {
        start.state.save.update_flags(EVENT, |f| f | ((1u64 << ARRIVAL) - 1));
        start.play_ops(EVENT, 24, &[Op::StatusSet { index: 0, num: 1 }]);
        start.state.save.set_u8(offset::LAST_TOWN, 1);
        let scene = piney_world::area::Scene::log_in(&mut start.state.save);
        let spcs = Some(crate::start::party_of(&[BLACK_ROSE]));
        start.at = Resume::World(Box::new(InWorld { scene, world_man: None, spcs }));
    })?;
    let playing = |s: &Session| matches!(&s.stage, Stage::World(w) if matches!(w.world().phase(), piney_world::Phase::Play(n) if n > 30));
    hold(&mut s, 128, 128, 900, playing);
    assert!(playing(&s), "not in Dun Loireag: {}", Mode::title(&s));
    Some(s)
}

fn shop_keeper(s: &Session) -> Option<&Merchant> {
    let Stage::World(w) = &s.stage else { return None };
    w.world().merchants().iter().find(|m| m.id == SHOP_KEEPER)
}

/// The Shop Keeper stands at marker 6, on the NPC list before the town's
/// merchants, with row 29's clump and the Administrator's clips, faces
/// Kite and draws while block 27 has him speak.
#[test]
fn mutations_shop_keeper_stands_at_marker_6() {
    let Some(mut s) = arrived() else { return };
    let Stage::World(w) = &s.stage else { unreachable!() };
    let world = w.world();
    assert_eq!(world.town().base.no, 1);
    let (pos, rot) = world.marker_dummy(MARKER).expect("marker 6 in Dun Loireag's file");
    let m = shop_keeper(&s).expect("entry 4 29 not placed");
    assert_eq!((m.name.as_str(), m.flags), ("Administrator", piney_world::npc::TYPE_SYSOPE));
    assert_eq!(world.merchants()[0].id, SHOP_KEEPER, "made before ccSetMerchant(0)'s");
    // At the marker's dummy, its rotation copied whole; then block 27's
    // `npc_face 29 2 0 0` turned him to Kite at once.
    let kite = world.player().body.pos;
    let face = piney_world::ee::deg2rad(piney_world::event::dirc_to(m.ch.pos, kite));
    assert_eq!((m.ch.pos, m.ch.dirc), (pos, [rot[0], rot[1], face, rot[3]]));
    assert_eq!(m.anm_tbl, piney_world::merchant::ANM_TBL[0]);
    let vm = w.vm().expect("the event task");
    assert!(blocks(vm).contains(&(EVENT, ARRIVAL as i32)), "block 25 did not run");
    // Block 27: faced toward Kite, the camera on him, his first line.
    hold_frame(&mut s, 128, 128, 150);
    let m = shop_keeper(&s).expect("still there while he speaks");
    assert!(m.frame.drawn, "not drawn: {:?}", m.frame);
}

/// The scene of the issue's screenshots, block 27's first line held, to
/// `$PINEY_SHOTS` (/mnt/data/claude/scratch/i60) as `mut-shop-keeper.png`.
#[test]
#[ignore]
fn shop_keeper_shot() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i60".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = arrived() else { return };
    let frame = hold_frame(&mut s, 128, 128, 150);
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    g.set_overlay(Mode::archive(&s));
    g.render(&frame);
    let (w, h) = g.target_size();
    let path = format!("{dir}/mut-shop-keeper.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    println!("{path}: {:?}", shop_keeper(&s).map(|m| m.frame));
}

/// Where a block's scene setting (kept from the blocks before, as
/// `SetCurrentOpen` keeps it) puts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Area {
    Unset,
    Town,
    Outside,
}

/// Every `entry 3|4` in every volume's scripts, with the area its block is
/// set in: (volume, event, block, area, type, code).
fn npc_entries() -> Vec<(Volume, u16, usize, Area, i16, i16)> {
    use piney_event::ir::Tag;
    let mut out = Vec::new();
    for volume in Volume::ALL {
        let file = piney_data::pack::disc_name(volume).trim_end_matches(".disc");
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{file}/{file}.iso"));
        if !iso.exists() {
            eprintln!("{file}.iso not present; skipped");
            continue;
        }
        let events = piney_event::official::events(&mut Iso::open(&iso).unwrap()).unwrap();
        for e in &events {
            let mut area = Area::Unset;
            for (b, block) in e.script.blocks.iter().enumerate() {
                for t in &block.tags {
                    area = match *t {
                        Tag::InTown { .. } | Tag::Scene { area: 0, .. } => Area::Town,
                        Tag::InField { .. } | Tag::InDungeon { .. } | Tag::InPoint { .. } | Tag::Scene { .. } => {
                            Area::Outside
                        }
                        Tag::GameStatus { status } if status != 5 => Area::Unset,
                        _ => area,
                    };
                }
                for op in &block.ops {
                    if let Op::Entry { ty: ty @ (3 | 4), code, .. } = *op {
                        out.push((volume, e.number, b, area, ty, code));
                    }
                }
            }
        }
    }
    out
}

/// Every event NPC entry on every volume makes a class: the town's
/// `entry 4 29` (seven on each volume, Mutation's 101 among them) and
/// Outbreak's `entry 3 29`s (event 209, in a town and outside) as well as
/// the PCs. Before, the towns refused type 4 and type 3 of a merchant's row.
#[test]
fn every_event_npc_entry_makes_a_class() {
    let all = npc_entries();
    let mut before = std::collections::BTreeMap::new();
    let mut after = std::collections::BTreeMap::new();
    for &(volume, n, b, area, ty, code) in &all {
        let row = piney_world::npc::NpcRow::of(volume, code as usize).unwrap();
        let old = match (area, ty) {
            (Area::Outside, 3) => row.func == Some(piney_data::tables::types::EntryFunc::RtownPC),
            (Area::Outside, _) => piney_world::merchant::anm_tbl(i32::from(code), 0).is_some(),
            (_, 3) => row.flags & piney_world::npc::TYPE_PC != 0,
            _ => false,
        };
        if !old {
            *before.entry((volume, area)).or_insert(0) += 1;
            eprintln!("BEFORE {volume:?} event {n} block {b} {area:?} entry {ty} {code}");
        }
        if row.event_class(ty).is_none() {
            *after.entry((volume, area)).or_insert(0) += 1;
        }
    }
    eprintln!("TOTAL {} before {before:?} after {after:?}", all.len());
    assert!(after.is_empty(), "{after:?}");
}
