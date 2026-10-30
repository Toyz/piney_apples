//! Kyvia's third fight (`ccBossKyvia03`, Quarantine's event 308 on field
//! 11's disc): played from the body's coming to its death and the event
//! on, the party held up by the console's god, with the magical ball and
//! its plasmas drawn and the fight's music heard.
//! docs/engine/boss-kyvia-third.md.

use std::collections::{BTreeMap, BTreeSet};

use piney_battle::boss::Class;
use piney_input::{Buttons, Raw};

use super::kyvia_fx::event_308_disc;
use super::survey::{event_flag, levels_for_boss};
use super::*;

/// What the fight showed: the body's acts, the disc's stages, the boss
/// effects' draws by frames, the frames the ball's plasmas were followed,
/// and whether event 308 ended (its done or closed bit).
#[derive(Default)]
struct Played {
    acts: BTreeSet<i16>,
    stages: BTreeSet<i16>,
    drawn: BTreeMap<String, u32>,
    plasma: u32,
    frames: u32,
    boss_gone_at: Option<u32>,
    ended_at: Option<u32>,
    /// Frames of the fight with `camID` off the boss camera's 2.
    off_cam: u32,
    /// Frames of the fight with a sequence (the music) playing, every
    /// frame's events routed to a headless engine as `main` routes them.
    music: u32,
}

/// Event 308 from its fight on, `frames` at most: the lines moved on,
/// the god's party kept at full HP and levelled for the parts, the core
/// hurried and worn down; `each` sees every frame of the fight.
fn play(s: &mut Session, frames: u32, mut each: impl FnMut(&Session, &Frame, u32)) -> Played {
    let mut pad = Pad::default();
    let mut p = Played::default();
    let audio = piney_audio::Audio::headless(&s.iso).ok();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    s.console("god");
    for n in 0..frames {
        if n.is_multiple_of(30) {
            levels_for_boss(s);
        }
        let talking = match &s.stage {
            Stage::Area(a) => a.vm().is_some_and(|v| v.playing().is_some()) || a.ui().ctrl.check_menu_type() != -1,
            _ => false,
        };
        pad.read(&if talking && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let rate = Mode::frame_rate(s);
        let frame = s.step(&pad);
        crate::handle(s.take_events(), audio.as_ref());
        let music = audio.as_ref().is_some_and(|a| {
            a.frame();
            a.render(&mut vec![0i16; 1600 * rate as usize]);
            a.with_engine(|e| e.seq.iter().any(|q| q.playing()))
        });
        p.frames = n + 1;
        if p.ended_at.is_none() && event_flag(s, 308).is_some_and(|x| x & 3 << 62 != 0) {
            p.ended_at = Some(n);
            break;
        }
        if std::env::var_os("PINEY_DEBUG_KYVIA").is_some() && n.is_multiple_of(100) {
            let what = match &s.stage {
                Stage::Area(a) => {
                    let c = a.world().combat();
                    let b = c.boss.as_ref().map(|r| {
                        let bb = c.scene.chars[r.me].foe_state().and_then(|f| f.boss.as_ref());
                        let core = r.parts.first().map(|&k| (c.scene.chars[k].hp, c.scene.listed(k)));
                        (r.code, r.exit, bb.map(|b| (b.base.act_num, b.base.act_proccess, b.base.exit)), core)
                    });
                    format!("{} boss {b:?} flag {:x?}", Mode::title(s), event_flag(s, 308))
                }
                _ => Mode::title(s),
            };
            eprintln!("KYVIA {n} {what}");
        }
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let cam_id = a.world().camera().cam_id;
        let c = a.world_mut().combat_mut();
        let Some(run) = c.boss.as_ref() else { continue };
        if run.exit {
            p.boss_gone_at.get_or_insert(n);
            continue;
        }
        let me = run.me;
        p.off_cam += u32::from(cam_id != piney_world::camera::id::BATTLE);
        // The core's time cut short (its attack turn sooner), and while it
        // can be hit a blow of an eighth of its MaxHP from Kite each 300
        // frames (`EntryAffect` 1, as the console's `kill`), for the
        // party's (the god's party alone takes long).
        let core = match c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
            Some(Class::Kyvia(x)) => Some(x.core),
            _ => None,
        };
        // At the last stage the blows wait below 60% of MaxHP until the
        // magical ball has come (it needs the core below 70%).
        let last = match c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
            Some(Class::Kyvia(x)) => x.disc_lv == x.disc_max_lv,
            _ => false,
        };
        let wait = |k: usize| {
            let ch = &c.scene.chars[k];
            last && !p.acts.contains(&20) && i32::from(ch.hp) * 10 < i32::from(ch.max_hp) * 6
        };
        let blow = core.filter(|&k| c.scene.listed(k) && n.is_multiple_of(300) && c.scene.chars[k].hp > 0 && !wait(k));
        if let Some(core) = core
            && let Some(Class::KyviaCore(x)) =
                c.scene.chars[core].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
        {
            x.now_dmg_time = x.now_dmg_time.max(x.dmg_time - 1);
        }
        let kite = c.kite;
        if let Some(core) = blow {
            let d = (c.scene.chars[core].max_hp / 8).max(1);
            a.world_mut().entry_affect(core, kite, 1, [d, 0, 0]);
        }
        let c = a.world_mut().combat_mut();
        if let Some(b) = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()) {
            p.acts.insert(b.base.act_num);
            if let Class::Kyvia(x) = &b.class {
                p.stages.insert(x.disc_lv);
            }
            if b.base.effects.slots.iter().flatten().any(|e| e.ball.is_some()) && census.anchored_generators > 0 {
                p.plasma += 1;
            }
        }
        p.music += u32::from(music);
        let mut names = census.boss_draws.clone();
        names.dedup();
        for d in names {
            *p.drawn.entry(d).or_default() += 1;
        }
        each(s, &frame, n);
    }
    p
}

/// The third fight to its end: every attack seen (the arm, the beams, the
/// meteors, the thunder, the magical ball) over the disc's three stages,
/// the ball drawn (xeffect's `ANM_ex31exp3`) with its plasmas' generators
/// following their places, the body's death, and event 308 ended.
#[test]
#[ignore]
fn kyvia_s_third_fight_to_its_end() {
    let Some(mut s) = event_308_disc() else { return };
    let p = play(&mut s, 120_000, |_, _, _| {});
    println!(
        "{} frames: acts {:?}, stages {:?}, drawn {:?}, plasma {} frames, music {} frames, boss gone at {:?}, \
         event ended at {:?}",
        p.frames, p.acts, p.stages, p.drawn, p.plasma, p.music, p.boss_gone_at, p.ended_at
    );
    for act in [3, 4, 5, 6, 20, 14] {
        assert!(p.acts.contains(&act), "act {act} not seen: {:?}", p.acts);
    }
    assert_eq!(p.stages, BTreeSet::from([1, 2, 3]));
    assert!(p.drawn.get("ANM_ex31exp3").is_some_and(|&n| n > 100), "the ball not drawn: {:?}", p.drawn);
    assert!(p.plasma > 50, "the plasmas not followed ({} frames)", p.plasma);
    assert!(p.music > 5000, "no music in the fight ({} frames)", p.music);
    assert!(p.boss_gone_at.is_some(), "the body never died");
    assert_eq!(p.off_cam, 0, "camID left the boss camera");
    assert!(p.ended_at.is_some(), "event 308 never ended");
}

/// Shots of the third fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a433): every frame rendered, each 60th of the
/// ball and each 600th else saved.
#[test]
#[ignore]
fn kyvia_third_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a433".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_308_disc() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    play(&mut s, 60_000, |s, frame, n| {
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let ball = match &s.stage {
            Stage::Area(a) => a.world().combat().boss.as_ref().is_some_and(|r| {
                a.world().combat().scene.chars[r.me]
                    .foe_state()
                    .and_then(|f| f.boss.as_ref())
                    .is_some_and(|b| b.base.effects.slots.iter().flatten().any(|e| e.ball.is_some()))
            }),
            _ => false,
        };
        if (ball && n.is_multiple_of(60)) || n.is_multiple_of(600) {
            let (w, h) = g.target_size();
            let path = format!("{dir}/kyvia3-{n:05}{}.png", if ball { "-ball" } else { "" });
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        }
    });
}
