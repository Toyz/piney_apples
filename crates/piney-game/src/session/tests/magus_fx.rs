//! Magus's pictures in its fight (Mutation's event 115): the leaves'
//! markers following them, their bursts (`OBJ_ex31exp1`), the needles
//! (`OBJ_ex71body2`), the laser's shocks where their beams meet the
//! ground with the thunder's particles, the dropped leaves hidden on the
//! body, and its flashes and the grow's looping sound.

use std::collections::{BTreeMap, BTreeSet};

use piney_battle::boss::Class;
use piney_battle::boss::magus::act;
use piney_input::{Buttons, Raw};

use super::skeith::event_115_session;
use super::*;

/// What one frame of the fight showed.
#[derive(Default)]
struct Seen {
    act: i16,
    draws: Vec<String>,
    anchored: usize,
    particles: usize,
    /// The body's objects hidden, the leaves dropped, the shocks on the
    /// ground.
    hidden: usize,
    dropped: usize,
    shocks: usize,
}

/// The fight for `frames` frames from Magus's making, the party kept up,
/// the event's lines moved on. When Magus stands with its leaves on the
/// body the laser is reserved (at most once in 3000 frames), and every
/// 1500 frames its damage count is run out (`m_dmgCount` 0: the needles),
/// so its rules take each turn.
/// `each` sees every frame.
fn fight(s: &mut Session, frames: u32, each: impl FnMut(&Session, &Frame, u32, &Seen)) -> Vec<Seen> {
    fight_heard(s, frames, None, each)
}

/// [`fight`] with every frame's events sent through `crate::handle` to
/// `audio`, from the session's start.
fn fight_heard(
    s: &mut Session,
    frames: u32,
    audio: Option<&piney_audio::Audio>,
    mut each: impl FnMut(&Session, &Frame, u32, &Seen),
) -> Vec<Seen> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut seen, mut laser_at) = (None, Vec::new(), 0);
    for n in 0..frames + 6000 {
        let talking = match &s.stage {
            Stage::Area(a) => a.vm().is_some_and(|v| v.playing().is_some()) || a.ui().ctrl.check_menu_type() != -1,
            _ => false,
        };
        pad.read(&if talking && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        let events = s.take_events();
        if audio.is_some() {
            crate::handle(events, audio);
        }
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss.as_ref().filter(|r| r.code == 2 && !r.exit).map(|r| r.me) else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        let hidden = c.cast.actors.get(&me).map_or(0, |a| a.ch.play.hidden.len());
        let leaves = match c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class) {
            Some(Class::Magus(x)) => x.leaves.clone(),
            _ => Vec::new(),
        };
        let dropped = leaves
            .iter()
            .filter(|&&l| {
                matches!(c.scene.chars[l].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class),
                    Some(Class::MagusLeaf(x)) if x.drop != 0)
            })
            .count();
        let Some(b) = c.scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut()) else { continue };
        let neutral = b.act_num == act::NEUTRAL;
        let Class::Magus(x) = &mut b.class else { continue };
        if neutral && dropped == 0 && i >= laser_at {
            x.reserve_laser = 1;
            laser_at = i + 3000;
        }
        if i % 1500 == 1499 {
            x.dmg_count = 0;
        }
        let shocks = if b.act_num == act::LASER { x.shock_at.iter().flatten().count() } else { 0 };
        let now = Seen {
            act: b.act_num,
            draws: census.boss_draws,
            anchored: census.anchored_generators,
            particles: census.particles,
            hidden,
            dropped,
            shocks,
        };
        each(s, &frame, i, &now);
        seen.push(now);
    }
    seen
}

/// Frames each boss effect draw showed in.
fn tally(seen: &[Seen]) -> BTreeMap<String, u32> {
    let mut drawn: BTreeMap<String, u32> = BTreeMap::new();
    for now in seen {
        let names: BTreeSet<&String> = now.draws.iter().collect();
        for d in names {
            *drawn.entry(d.clone()).or_default() += 1;
        }
    }
    drawn
}

/// The fight shows the leaves' markers, bursts, the needles, the laser's
/// shocks on the ground with their thunder, and hides its dropped leaves.
#[test]
fn magus_shows_its_leaves_needles_and_laser() {
    let Some(mut s) = event_115_session() else { return };
    let seen = fight(&mut s, 9000, |_, _, _, _| {});
    let drawn = tally(&seen);
    let acts: BTreeSet<i16> = seen.iter().map(|n| n.act).collect();
    let count = |f: &dyn Fn(&Seen) -> bool| seen.iter().filter(|n| f(n)).count();
    let anchored = count(&|n| n.anchored > 0);
    let hidden = count(&|n| n.dropped > 0 && n.hidden > 0);
    let unhidden = count(&|n| n.dropped > 0 && n.hidden == 0 && n.act != act::LEAF_GROW);
    let shocks = count(&|n| n.shocks > 0);
    let laser_eff = count(&|n| n.act == act::LASER && n.draws.iter().any(|d| d.starts_with("eff ")));
    let thunder = count(&|n| n.act == act::LASER && n.particles > 0);
    println!(
        "acts {acts:?}, drawn {drawn:?}, markers {anchored}, hidden {hidden} (not {unhidden}), shocks {shocks} \
         (drawn {laser_eff}), thunder {thunder}"
    );
    assert!(anchored > 100, "no marker followed a leaf ({anchored} frames)");
    assert!(hidden > 100 && unhidden == 0, "dropped leaves not hidden: {hidden} hidden, {unhidden} not");
    assert!(drawn.get("OBJ_ex71body2").is_some_and(|&n| n > 10), "no needles: {drawn:?}");
    assert!(drawn.get("OBJ_ex31exp1").is_some_and(|&n| n > 10), "no leaf burst: {drawn:?}");
    assert!(acts.contains(&act::LASER), "no laser: {acts:?}");
    assert!(shocks > 10 && laser_eff > 10, "the shocks not on the ground: {shocks} frames, {laser_eff} drawn");
    assert!(thunder > 30, "no thunder in the laser ({thunder} frames)");
}

/// The fader's element of Magus's flashes (colour 0x80e0ffff, from the
/// colour with alpha 0): its status and count.
fn magus_flash(s: &Session) -> Option<(i16, i16)> {
    let Stage::Area(a) = &s.stage else { return None };
    (a.fade_def().elems.iter())
        .find(|e| e.status != 0 && (e.col0 == 0x80e0_ffff || e.col1 == 0x80e0_ffff))
        .map(|e| (e.status, e.tcnt))
}

/// Magus's flashes and its grow's loop as the game sends them, every
/// frame's events through `crate::handle` from the session's start. The
/// rise's `EntryFlash2(60, 60)` fades in then out (status 5, then 3 over
/// 60); the laser's `EntryFlash3(80, 80, 20)` in, held 20, out (9, 5, 3).
/// The grow's SE 230 takes a loop slot (`ccSeOn3DLoop`) until its end's
/// `ccSeOffLoop`. Before, each flash was one fade out and the loop played
/// once, taking no slot.
#[test]
fn magus_flashes_and_its_grows_loop() {
    let Some(mut s) = event_115_session() else { return };
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    let audio = piney_audio::Audio::headless(&iso).unwrap();
    let (mut phases, mut growing, mut held, mut stray) = (BTreeSet::new(), 0u32, 0u32, 0u32);
    let mut grows = 0u32;
    let mut last = -1;
    fight_heard(&mut s, 9000, Some(&audio), |s, _, _, now| {
        if let Some(p) = magus_flash(s) {
            phases.insert(p);
        }
        let looped = audio.with_engine(|e| e.driver.loop_id.iter().any(|&i| i != -1));
        if now.act == act::LEAF_GROW {
            growing += 1;
            held += u32::from(looped);
        } else {
            stray += u32::from(looped);
        }
        grows += u32::from(now.act == act::LEAF_GROW && last != act::LEAF_GROW);
        last = now.act;
    });
    println!("flash phases {phases:?}; {grows} grows, the loop on {held} of {growing} frames, {stray} outside");
    assert!(phases.contains(&(5, 60)) && phases.contains(&(3, 60)), "the rise's flash: {phases:?}");
    assert!(
        phases.contains(&(9, 80)) && phases.contains(&(5, 20)) && phases.contains(&(3, 80)),
        "the laser's flash: {phases:?}"
    );
    assert!(grows > 0 && held + 2 * grows >= growing, "the loop on {held} of {growing} grow frames");
    assert_eq!(stray, 0, "the loop left on after the grow");
}

/// Shots of Magus's flashes to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a428): every 8th frame one of them is on the
/// fader, named by its status and count.
#[test]
#[ignore]
fn magus_flash_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a428".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_115_session() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    fight(&mut s, 9000, |s, frame, i, now| {
        let Some((status, t)) = magus_flash(s).filter(|_| i.is_multiple_of(8)) else { return };
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/magus-{i:05}-act{}-status{status}-t{t}.png", now.act);
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}

/// Shots of the fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a421): every 6th frame a needle, a burst or a
/// shock is drawn, through its first 9000.
#[test]
#[ignore]
fn magus_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a421".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_115_session() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    fight(&mut s, 9000, |s, frame, i, now| {
        let shown = !now.draws.is_empty() || now.shocks > 0;
        if !(shown && i.is_multiple_of(6)) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/magus-{i:05}-act{}.png", now.act);
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}
