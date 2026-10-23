//! Kyvia's fourth fight (`ccBossKyvia04`, Quarantine's event 312 on field
//! 12's disc): played from the body's coming through the four places, the
//! EX turn and the rebirth to its death and the event on, the party held
//! up by the console's god, with Hell's express drawn and the music heard.
//! docs/engine/boss-kyvia-fourth.md.

use std::collections::{BTreeMap, BTreeSet};

use piney_battle::boss::Class;
use piney_input::{Buttons, Raw};

use super::kyvia_fx::event_312_disc;
use super::survey::{event_flag, levels_for_boss};
use super::*;

/// What the fight showed: the body's acts, the places reached, the boss
/// effects' draws by frames, whether the EX turn came and the frame the
/// body died and event 312 went on past its fight (block 17 done: Lia
/// Fail, where its next scene waits for the player).
#[derive(Default)]
struct Played {
    acts: BTreeSet<i16>,
    places: BTreeSet<i32>,
    drawn: BTreeMap<String, u32>,
    ex: bool,
    frames: u32,
    boss_gone_at: Option<u32>,
    on_at: Option<u32>,
    /// Frames of the fight with a sequence (the music) playing.
    music: u32,
}

/// Event 312 from its fight on, `frames` at most: the lines moved on, the
/// god's party kept at full HP and levelled for the parts, Kite set at the
/// next place while the body waits for it, the core hurried and worn down;
/// `each` sees every frame of the fight.
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
        if p.on_at.is_none() && event_flag(s, 312).is_some_and(|x| x & 1 << 17 != 0) {
            p.on_at = Some(n);
            break;
        }
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        let Some(run) = c.boss.as_ref() else { continue };
        if run.exit {
            p.boss_gone_at.get_or_insert(n);
            continue;
        }
        let me = run.me;
        let kite = c.kite;
        // While the body waits for Kite (`StFlg` 1), Kite stands at the
        // next place (`CoreAppearPos[CoreLvCount]`), as a player walks
        // there past the opened gate.
        let (core, place) = match c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
            Some(Class::Kyvia(x)) => {
                p.ex |= x.fourth.as_ref().is_some_and(|f| f.fight_flg != 0);
                let place = x.fourth.as_ref().filter(|f| x.st_flg == 1 && f.rising == 0).and_then(|f| {
                    usize::try_from(f.core_lv_count)
                        .ok()
                        .and_then(|k| f.core_appear_pos.get(k))
                        .map(|&at| (f.core_lv_count, at))
                });
                (Some(x.core), place)
            }
            _ => (None, None),
        };
        if let (Some(k), Some((lv, at))) = (kite, place) {
            c.scene.chars[k].pos = at;
            p.places.insert(lv);
        }
        if std::env::var_os("PINEY_DEBUG_KYVIA").is_some() && n.is_multiple_of(100) {
            let b = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref());
            let st = b.and_then(|b| match &b.class {
                Class::Kyvia(x) => Some((x.st_flg, x.disc_lv, x.fourth.as_ref().map(|f| (f.core_lv_count, f.rising)))),
                _ => None,
            });
            let hp = core.map(|k| (c.scene.chars[k].hp, c.scene.listed(k)));
            eprintln!("KYVIA {n} act {:?} st {st:?} core {hp:?}", b.map(|b| (b.act_num, b.act_proccess)));
        }
        // The core's time cut short (its attack turn sooner), and while it
        // can be hit a blow of an eighth of its MaxHP from Kite each 300
        // frames (`EntryAffect` 1, as the console's `kill`).
        let blow = core.filter(|&k| c.scene.listed(k) && n.is_multiple_of(300) && c.scene.chars[k].hp > 0);
        if let Some(core) = core
            && let Some(Class::KyviaCore(x)) =
                c.scene.chars[core].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
        {
            x.now_dmg_time = x.now_dmg_time.max(x.dmg_time - 1);
        }
        if let Some(core) = blow {
            let d = (c.scene.chars[core].max_hp / 8).max(1);
            a.world_mut().entry_affect(core, kite, 1, [d, 0, 0]);
        }
        let c = a.world_mut().combat_mut();
        if let Some(b) = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()) {
            p.acts.insert(b.act_num);
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

/// The fourth fight to its end: the four places reached, the EX turn's
/// attacks (Hell's express, the big megid, the skill, the magical ball),
/// the rebirth, Hell's express drawn (xeffect's `ANM_ex31lhit`), the
/// body's death, and event 312 on to Lia Fail.
#[test]
#[ignore]
fn kyvia_s_fourth_fight_to_its_end() {
    let Some(mut s) = event_312_disc() else { return };
    let p = play(&mut s, 60_000, |_, _, _| {});
    println!(
        "{} frames: acts {:?}, places {:?}, ex {}, drawn {:?}, music {} frames, boss gone at {:?}, event on at {:?}",
        p.frames, p.acts, p.places, p.ex, p.drawn, p.music, p.boss_gone_at, p.on_at
    );
    for act in [21, 22, 23, 24, 14] {
        assert!(p.acts.contains(&act), "act {act} not seen: {:?}", p.acts);
    }
    assert_eq!(p.places, BTreeSet::from([0, 1, 2, 3]));
    assert!(p.ex, "the EX turn never came");
    assert!(p.drawn.get("ANM_ex31lhit").is_some_and(|&n| n > 30), "Hell's express not drawn: {:?}", p.drawn);
    assert!(p.music > 5000, "no music in the fight ({} frames)", p.music);
    assert!(p.boss_gone_at.is_some(), "the body never died");
    assert!(p.on_at.is_some(), "event 312 never went on past its fight");
}

/// Shots of the fourth fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a434): every frame rendered, each 30th of
/// Hell's express and each 600th else saved.
#[test]
#[ignore]
fn kyvia_fourth_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a434".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_312_disc() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    play(&mut s, 80_000, |s, frame, n| {
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let hell = match &s.stage {
            Stage::Area(a) => a.world().fx().census().boss_draws.iter().any(|d| d == "ANM_ex31lhit"),
            _ => false,
        };
        if (hell && n.is_multiple_of(30)) || n.is_multiple_of(600) {
            let (w, h) = g.target_size();
            let path = format!("{dir}/kyvia4-{n:05}{}.png", if hell { "-hell" } else { "" });
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        }
    });
}
