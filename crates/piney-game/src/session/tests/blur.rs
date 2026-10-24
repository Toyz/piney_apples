//! The boss effect manager's blur (`ccBossBlur`, worklog 432): every
//! fight's feedback of the last frame on the manager's layer, as each
//! boss's constructor and acts set it.

use std::collections::BTreeSet;

use piney_battle::boss::blur::Reflex;
use piney_draw::{Cmd, TexRef};
use piney_input::{Buttons, Raw};

use super::*;

/// A fight's name and how its session is reached.
type Arena = (&'static str, fn() -> Option<Session>);

/// What a frame of a fight drew of the blur: its reflex, and whether the
/// frame carries its strip (a primitive textured with the last frame).
struct Seen {
    reflex: Option<Reflex>,
    drawn: bool,
    /// The frame draws the field (not a stream's).
    world: bool,
}

/// The feedback strip in a frame's commands.
fn feedback_drawn(frame: &Frame) -> bool {
    frame.cmds.iter().any(|c| match c {
        Cmd::Prim(p) => p.state.texture.as_ref().is_some_and(|t| t.tex == TexRef::PreviousFrame),
        _ => false,
    })
}

/// `frames` frames from the boss's making, the party kept up and the
/// event's lines moved on; `each` sees every frame.
fn fight(s: &mut Session, frames: u32, mut each: impl FnMut(&Session, &Frame)) -> Vec<Seen> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut seen) = (None, Vec::new());
    for n in 0..frames + 16000 {
        let talking = match &s.stage {
            Stage::Area(a) => a.vm().is_some_and(|v| v.playing().is_some()) || a.ui().ctrl.check_menu_type() != -1,
            _ => false,
        };
        pad.read(&if talking && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(run) = c.boss.as_ref() else { continue };
        let reflex = run.reflex;
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        each(s, &frame);
        // A stream's frame (a cutscene over the fight) draws its own scene.
        let world = frame.cmds.iter().any(|c| matches!(c, Cmd::Model(m) if !m.file.starts_with("str")));
        seen.push(Seen { reflex, drawn: feedback_drawn(&frame), world });
    }
    seen
}

/// Each fight's blur over 1500 frames: drawn every frame its reflex is
/// (from the boss's second frame: `Main` runs before the boss's), with
/// the reflexes the constructors and acts give. Skeith's and Fidchell's
/// `ccBoss03BlurParamCB` swings the scale about 1 and the turn about 0 in
/// 0x48808080; Innis's is turned 0.02 at 1.01; Magus's 1.01; Kyvia's
/// 1.0 at its colours; Gorre's, whose `m_exit` is the heap's word, none
/// with the port's (docs/engine/heap.md).
#[test]
fn every_fight_shows_its_blur() {
    let fights: [Arena; 7] = [
        ("skeith", super::skeith::event_30_arena),
        ("innis", super::skeith::event_107_arena),
        ("magus", super::skeith::event_115_session),
        ("kyvia", super::kyvia_fx::event_108_disc),
        ("kyvia2", super::kyvia_fx::event_211_disc),
        ("fidchell", super::fidchell::event_207_arena),
        ("gorre", super::boss_fx::event_218_arena),
    ];
    for (name, make) in fights {
        let Some(mut s) = make() else { continue };
        let seen = fight(&mut s, 1500, |_, _| {});
        let shown: Vec<&Reflex> = seen.iter().filter_map(|n| n.reflex.as_ref()).collect();
        let drawn = seen.iter().filter(|n| n.drawn).count();
        let missed = seen.iter().filter(|n| n.reflex.is_some() && n.world && !n.drawn).count();
        let abgr: BTreeSet<u32> = shown.iter().map(|r| r.abgr).collect();
        let scales: BTreeSet<u32> = shown.iter().map(|r| r.scale).collect();
        let turns: BTreeSet<u32> = shown.iter().map(|r| r.turn).collect();
        println!(
            "{name}: {} frames, the blur in {}, drawn in {drawn}; colours {abgr:x?}, {} scales, {} turns",
            seen.len(),
            shown.len(),
            scales.len(),
            turns.len()
        );
        assert!(seen.len() > 1000, "{name}: the fight not reached");
        assert_eq!(missed, 0, "{name}: a reflex not drawn");
        match name {
            "gorre" => assert!(shown.is_empty(), "{name}: HEAP_EXIT is not 0"),
            "skeith" | "fidchell" => {
                assert!(shown.len() + 2 >= seen.len(), "{name}: {} of {}", shown.len(), seen.len());
                assert_eq!(abgr, BTreeSet::from([0x4880_8080]));
                assert!(scales.len() > 50 && turns.len() > 50, "{name}: the callback's swing");
            }
            "innis" => {
                assert!(shown.iter().all(|r| r.scale == 0x3f81_47ae && r.turn == 0x3ca3_d70a));
            }
            "magus" => {
                assert!(shown.iter().all(|r| r.scale == 0x3f81_47ae && r.turn == 0));
                assert!(abgr.contains(&0x4880_8080));
            }
            _ => {
                assert!(shown.len() + 2 >= seen.len(), "{name}: {} of {}", shown.len(), seen.len());
                assert!(abgr.contains(&0x3080_8080), "{name}: DefaultARGB");
            }
        }
    }
}

/// Gorre's fight on the other word: the histories whose manager lands on
/// one of the cutscene mesh's 0 words (docs/engine/heap.md) leave
/// `m_exit` 0, and nothing in the fight writes the blur. Disabled, its
/// scale eases from 1.01 to 1.0 by a thirtieth of the gap a frame and
/// stays; its turn starts at 0, so it never arrives and never exits.
#[test]
fn gorre_on_a_zero_word_shows_the_managers_own_blur() {
    let Some(mut s) = super::boss_fx::event_218_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut shown) = (None, Vec::new());
    for n in 0..20000u32 {
        let talking = match &s.stage {
            Stage::Area(a) => a.vm().is_some_and(|v| v.playing().is_some()) || a.ui().ctrl.check_menu_type() != -1,
            _ => false,
        };
        pad.read(&if talking && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let c = a.world_mut().combat_mut();
        let Some(run) = c.boss.as_ref() else {
            c.heap_exit = 0;
            continue;
        };
        if n - *came.get_or_insert(n) >= 120 {
            break;
        }
        shown.push(run.reflex);
    }
    assert_eq!(shown.len(), 120, "the fight not reached");
    // From the boss's second frame: Main runs before the boss's.
    let shown: Vec<Reflex> = shown.into_iter().skip(1).map(|r| r.expect("a reflex every frame")).collect();
    assert!(shown.iter().all(|r| r.abgr == 0x4080_8080 && r.turn == 0));
    let scales: Vec<f32> = shown.iter().map(|r| f32::from_bits(r.scale)).collect();
    assert!(scales[0] > 1.0 && scales[0] < 1.01, "{}", scales[0]);
    assert!(scales.windows(2).all(|w| w[1] <= w[0]), "the scale only falls");
    assert!(scales[40..].iter().all(|&x| x == 1.0), "held at 1.0");
}

/// Shots to `PINEY_SHOTS` (default /mnt/data/claude/scratch/a432): each
/// fight with its blur, every 150th frame.
#[test]
#[ignore]
fn blur_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a432".into());
    std::fs::create_dir_all(&dir).unwrap();
    let fights: [Arena; 5] = [
        ("skeith", super::skeith::event_30_arena),
        ("innis", super::skeith::event_107_arena),
        ("magus", super::skeith::event_115_session),
        ("kyvia", super::kyvia_fx::event_108_disc),
        ("fidchell", super::fidchell::event_207_arena),
    ];
    for (name, make) in fights {
        let Some(mut s) = make() else { continue };
        let mut gs: Option<piney_gs::Gs> = None;
        let mut i = 0u32;
        // Every frame rendered: the feedback reads the frame before.
        fight(&mut s, 900, |s, frame| {
            i += 1;
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            if !i.is_multiple_of(150) {
                return;
            }
            let (w, h) = g.target_size();
            let path = format!("{dir}/blur-{name}-{i:04}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        });
    }
}

/// `ccSaveData` before Quarantine's event 313's fight, written to
/// `PINEY_SAVE_OUT` for piney-eemu's `ee_run` (the whole program run to
/// `ccBoss08`'s first frame): the story to 313, its blocks 0-14 played
/// (`PINEY_SAVE_BLOCKS`) and `eventStatus[0]` 1 (block 15's). The party
/// is `ee_run`'s to invite.
#[test]
#[ignore]
fn event_313_save_bytes() {
    let Ok(out) = std::env::var("PINEY_SAVE_OUT") else { return };
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/quarantine/quarantine.iso");
    if !iso.exists() {
        return;
    }
    let blocks: u32 = std::env::var("PINEY_SAVE_BLOCKS").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    let (mut state, _) = crate::start::state_at(&iso, 313).unwrap();
    let f = state.save.event_flag(313);
    state.save.set_event_flag(313, f | ((1 << blocks) - 1));
    state.save.set_u8(offset::EVENT_STATUS, 1);
    std::fs::write(out, state.save.bytes()).unwrap();
}
