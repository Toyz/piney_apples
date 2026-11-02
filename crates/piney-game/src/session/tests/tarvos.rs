//! Tarvos (`ccBoss07`, Quarantine's event 310 in field 7's arena): the
//! fight under god from the arena, its acts, pictures and music; and with
//! no aid from the arena and from the event's own start under the story
//! autopilot. docs/engine/boss-tarvos.md.

use piney_battle::boss::tarvos::act;
use piney_battle::boss::{Boss, defs};

use super::arena::{EVENT_310, arena_session, god_shots, play};
use super::*;

fn tarvos(b: &Boss) -> bool {
    b.def == &defs::TARVOS
}

/// The fight under god from the arena to event 310's end: its patterns'
/// acts met (the spike rain, the wave, the sacrifice, the phantom, the
/// transfer, the skills and spells), the drain and the Epitaph, the music
/// through most of it; shots in `PINEY_SHOTS`, else the scratch's
/// `qua/tarvos`.
#[test]
#[ignore]
fn tarvos_under_god() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/tarvos".into());
    let Some(mut s) = arena_session(&EVENT_310, &[15, 6]) else { return };
    let (p, music) = god_shots(&mut s, 310, tarvos, &dir, "tarvos");
    eprintln!("acts {:?} hurt {:?} ended {:?} music {music} of {}", p.acts, p.hurt, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 310 did not end: {}", Mode::title(&s));
    for a in [act::NEUTRAL, act::EPITAPH, act::DEAD] {
        assert!(p.acts.contains(&a), "act {a} met");
    }
    assert!(music * 2 > p.frames, "the fight's music through most of it");
}

/// Tarvos fought to the end of event 310 with no aid: no god, Kite with
/// BlackRose and Balmung as the story leaves them, the pilot's own
/// tactics. `PINEY_TARVOS_GOD` (the console's god) is a diagnostic.
#[test]
#[ignore]
fn tarvos_fight_to_its_end() {
    let Some(mut s) = arena_session(&EVENT_310, &[15, 6]) else { return };
    if std::env::var_os("PINEY_TARVOS_GOD").is_some() {
        s.console("god");
    }
    let p = play(&mut s, 310, tarvos, 60_000, |s, _, _| {
        s.take_events();
    });
    eprintln!(
        "levels {:?} acts {:?} hurt {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.hurt, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 310 did not end: {}", Mode::title(&s));
}

/// Event 310 from its own story start (Lia Fail, area 102's dungeon, the
/// arena) with no aid.
#[test]
#[ignore]
fn event_310_from_its_start() {
    let Some(mut s) = story_session_on("quarantine", 310, |_| {}) else { return };
    let p = play(&mut s, 310, tarvos, 400_000, |s, _, _| {
        s.take_events();
    });
    eprintln!(
        "levels {:?} acts {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 310 did not end: {}", Mode::title(&s));
}

/// Tarvos's guard (`SetDefMode` 0 or 1) while it fights.
fn tarvos_guard(b: &piney_battle::boss::Boss) -> Option<i32> {
    b.systems.get::<piney_battle::boss::tarvos::Tarvos>().map(|x| x.def_mode)
}

/// Issue #79: Tarvos drained through the menus. Its guard's
/// `SetDefMode` keeps its base on `bossTbl` row 42 or 43; the drain's
/// `EntryAffect(21)` runs its `Affect` at once (QUA gcmn 0x0047f2e0), whose
/// `SetDefMode(2)` puts row 6 back before `DataDrainMenu` reads
/// `base->id`: stream 100, its own movie with its body, not 117 (a large
/// enemy's, with no one in it).
#[test]
fn tarvos_drained_plays_its_own_movie() {
    use piney_input::{Buttons, Raw};
    let Some(mut s) = arena_session(&EVENT_310, &[15, 6]) else { return };
    let mut pad = Pad::default();
    let (mut drain_menu, mut prepared, mut guard_at_menu) = (false, false, None);
    for i in 0..30000u64 {
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let press = |b: Buttons| if i.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
        if !matches!(s.stage, Stage::Area(_)) {
            pad.read(&press(Buttons::CROSS));
            s.step(&pad);
            s.take_events();
            continue;
        }
        let raw = {
            let Stage::Area(a) = &mut s.stage else { unreachable!() };
            if !prepared {
                let save = &mut a.world_mut().state_mut().save;
                let has = (0..20).any(|k| save.i16(piney_fieldui::items::SKILL_LIST + 2 * k) == 2);
                if !has {
                    let free = (0..20).find(|&k| save.i16(piney_fieldui::items::SKILL_LIST + 2 * k) < 0).unwrap();
                    save.set_i16(piney_fieldui::items::SKILL_LIST + 2 * free, 2);
                }
                save.set_u8(offset::PLCOL, 1);
                prepared = true;
            }
            {
                let c = a.world_mut().combat_mut();
                if let Some(k) = c.kite {
                    let ch = &mut c.scene.chars[k];
                    ch.max_hp = 9999;
                    ch.hp = 9999;
                    ch.sp = ch.sp.max(100);
                }
                // Enough hits would have broken its protect.
                if let Some(me) = c.boss_char()
                    && c.scene.chars[me]
                        .foe_state()
                        .and_then(|f| f.boss.as_ref())
                        .is_some_and(|b| tarvos_guard(b).is_some_and(|d| d < 2))
                    && let Some(p) = c.scene.param_mut(me)
                {
                    p.pp_count = p.pp_count.max(5);
                }
            }
            let w = a.world();
            let c = w.combat();
            let ui = a.ui();
            let m = &ui.ctrl;
            let boss = c
                .boss_char()
                .and_then(|me| c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| (me, b)));
            if m.menu == 66 && !drain_menu {
                drain_menu = true;
                guard_at_menu = boss.and_then(|(_, b)| tarvos_guard(b));
            }
            if a.calls().iter().any(|(_, c)| c.starts_with("drain_movie")) {
                break;
            }
            let go_to = |row: usize| match m.list().select.max(0) as usize {
                s if s == row => Buttons::CROSS,
                s if s < row => Buttons::DOWN,
                _ => Buttons::UP,
            };
            let playing = a.vm().is_some_and(|v| v.playing().is_some());
            match (ui.menu_type(), boss) {
                (-1, _) if playing => press(Buttons::CROSS),
                (-1, Some((me, b))) if b.base.lock_player == 0 && m.forbid == 0 => {
                    let k = c.kite.unwrap();
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let q = c.scene.chars[me].pos.map(f32::from_bits);
                    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                    if dx * dx + dy * dy < 900.0 * 900.0 && b.base.erase_target == 0 {
                        press(Buttons::TRIANGLE)
                    } else {
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        stick_toward(cam_z, dx.atan2(-dy))
                    }
                }
                (-1, _) => still,
                (0..=2, _) => press(go_to(m.list().items.iter().position(|&x| x == 4).unwrap_or(0))),
                (4, _) if m.list().page < 5 => press(Buttons::RIGHT),
                (4, _) => press(go_to(0)),
                (65, Some((me, _)))
                    if w.targeting().target != Some((piney_world::event::entry::Kind::Enemy, me as i32)) =>
                {
                    press(Buttons::DOWN)
                }
                _ => press(Buttons::CROSS),
            }
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    let Stage::Area(a) = &s.stage else { panic!("not in the arena: {}", Mode::title(&s)) };
    let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
    assert!(drain_menu, "menu 66 never ran");
    assert!(guard_at_menu.is_some_and(|d| d < 2), "Tarvos was not guarding at the drain: {guard_at_menu:?}");
    let movie = calls.iter().find(|c| c.starts_with("drain_movie"));
    assert_eq!(movie, Some(&"drain_movie 100"), "Tarvos's own movie");
}

/// Event 310's blocks 0-12 done (Tarvos drained and dead), back in Lia
/// Fail (town 4): blocks 13 and 14, the NPCs' goodbyes.
fn lia_fail_after_tarvos() -> Option<Session> {
    let iso = crate::check_image("quarantine");
    if !iso.exists() {
        return None;
    }
    let mut d = piney_data::iso::Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 310).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(310);
    state.save.set_event_flag(310, f | 0x1fff);
    state.save.set_u8(offset::EVENT_STATUS, 1);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    scene.change_scene(0, 4, -1, -1, -1, -1, &mut state.save);
    let spcs = Some(crate::start::party_of(&[15, 6]));
    let at = Resume::World(Box::new(InWorld { scene, world_man: None, spcs }));
    Some(Session::resume(iso, archive, None, state, start.vm, at).unwrap())
}

/// Issue #80: Tartarga (`cts1`) drawn whole in event 310's goodbyes. His
/// skin hangs on `OBJ_cts1body`, a node of `CMP_t0` (49 nodes), not of the
/// `CMP_trall` his town PC is made from (`OBJ_trall` alone); posed through
/// that one node, the whole body fell to his feet, out of the shot that
/// frames him as he speaks (block 14's `camz_set 40 -466 3 77 -433 9`).
#[test]
fn tartarga_is_drawn_whole_in_the_goodbyes() {
    let Some(mut s) = lia_fail_after_tarvos() else { return };
    let mut pad = Pad::default();
    let mut seen = 0;
    for n in 0..760u32 {
        let b = if n.is_multiple_of(12) { piney_input::Buttons::CROSS } else { piney_input::Buttons::NONE };
        pad.read(&piney_input::Raw {
            buttons: b,
            analog: true,
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            ..Default::default()
        });
        let frame = s.step(&pad);
        s.take_events();
        if n < 570 {
            continue;
        }
        for c in &frame.cmds {
            let piney_draw::Cmd::Model(m) = c else { continue };
            if m.file != "cts1" || m.mmats.len() < 2 {
                continue;
            }
            assert_eq!(m.nodes.len(), 49, "frame {n}: CMP_t0's nodes");
            let on_screen = m.nodes.iter().any(|k| {
                let q = glam::Mat4::from_cols_array_2d(k) * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
                let (x, y) = (q.x / q.w, q.y / q.w);
                q.w > 0.0 && (0.0..512.0).contains(&x) && (0.0..448.0).contains(&y)
            });
            assert!(on_screen, "frame {n}: Tartarga out of his shot");
            seen += 1;
        }
    }
    assert!(seen > 0, "Tartarga's skin never drawn");
}

/// Shots of event 310's block 14 every 30 frames to `PINEY_SHOTS`
/// (/mnt/data/claude/scratch/bugs/80/block14).
#[test]
#[ignore]
fn event_310_goodbyes_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/bugs/80/block14".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = lia_fail_after_tarvos() else { return };
    let mut pad = Pad::default();
    let mut gs: Option<piney_gs::Gs> = None;
    for n in 0..900u32 {
        let b = if n.is_multiple_of(12) { piney_input::Buttons::CROSS } else { piney_input::Buttons::NONE };
        pad.read(&piney_input::Raw {
            buttons: b,
            analog: true,
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            ..Default::default()
        });
        let frame = s.step(&pad);
        s.take_events();
        if n.is_multiple_of(30) {
            let gs =
                gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            std::fs::write(format!("{dir}/f{n:05}.png"), piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
        }
    }
}
