//! Issue #69: what a boss's fight shows. The protect's break and its
//! return on Skeith and Innis (the return raised by the boss's own
//! `CalcReal`), and Innis's magic: its missiles, its rings and its
//! particles.

use std::collections::BTreeMap;

use piney_battle::boss::Class;
use piney_battle::event::Event as Rule;
use piney_input::{Buttons, Raw};
use piney_world::combat::{Combat, Show};

use super::skeith::{event_30_arena, event_107_arena};
use super::*;

/// `effProtect`'s effects: the break's (19-21) and the return's (22-24).
const BREAK: std::ops::RangeInclusive<i16> = 19..=21;
const RETURN: std::ops::RangeInclusive<i16> = 22..=24;

/// What one frame of the fight left: the boss's act and gauge count and
/// the live effects.
#[derive(Default)]
struct Seen {
    act: i16,
    pp_count: i16,
    effects: Vec<i16>,
}

/// The fight in its arena for `frames` frames from the boss's coming, the
/// party kept alive and the event's messages moved on; `each` may change
/// the fight before the frame is seen.
fn fight(s: &mut Session, frames: u32, mut each: impl FnMut(&mut Combat, usize, u32, &Seen)) -> Vec<Seen> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut seen) = (None, Vec::new());
    for n in 0..frames + 3000 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        let f = c.scene.chars[me].foe_state();
        let now = Seen {
            act: f.and_then(|f| f.boss.as_ref()).map_or(-1, |b| b.act_num),
            pp_count: f.map_or(0, |f| f.pp_count),
            effects: census.effects,
        };
        each(c, me, i, &now);
        seen.push(now);
    }
    seen
}

/// Kite's blow on the boss as `CalcBattleDamage` makes it (a sure hit,
/// live), the gauge one short of full and the save's `plcol` set: its
/// events shown as the frame shows a rule's. True when the protect broke.
fn break_protect(c: &mut Combat, me: usize) -> bool {
    let Some(k) = c.kite else { return false };
    if let Some(f) = c.scene.chars[me].foe_state_mut() {
        f.pp = f.row.max_pp - 1;
    }
    let d = c.data.clone();
    let Some(sk) = d.t.skill(1).cloned() else { return false };
    let env = piney_battle::chara::Env::default();
    let mut rng = piney_battle::rand::Rand::new(1);
    let mut ev = Vec::new();
    let roll = piney_battle::damage::Roll::Given(100);
    piney_battle::damage::calc_damage_in(
        &d.t,
        &mut c.scene,
        k,
        me,
        &sk,
        piney_effect::ONE,
        roll,
        &mut rng,
        &env,
        &mut ev,
    );
    let broke = ev.iter().any(|e| matches!(e, Rule::Protect { broken: 0, .. }));
    c.shows.extend(ev.into_iter().map(Show::Rule));
    broke
}

/// Two breaks of the boss's protect, each left to run out: the frames
/// each break's picture and each return's was live.
fn protect_twice(s: &mut Session, frames: u32) -> ([u32; 2], [u32; 2]) {
    let mut breaks = 0usize;
    let seen = fight(s, frames, |c, me, _, now| {
        let free = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).is_some_and(|b| b.lock_player == 0);
        // Each break after the last one's pictures are over.
        let quiet = !now.effects.iter().any(|e| BREAK.contains(e) || RETURN.contains(e));
        if breaks < 2
            && quiet
            && now.pp_count == 0
            && now.act == 0
            && free
            && c.scene.listed(me)
            && break_protect(c, me)
        {
            breaks += 1;
        }
    });
    let (mut broke, mut back) = ([0; 2], [0; 2]);
    let mut k = 0usize;
    for w in seen.windows(2) {
        let shows = |r: &std::ops::RangeInclusive<i16>| w[1].effects.iter().any(|e| r.contains(e));
        // The count starting is a break; its end, the return.
        if w[0].pp_count == 0 && w[1].pp_count > 0 {
            k += 1;
        }
        if k == 0 || k > 2 {
            continue;
        }
        broke[k - 1] += u32::from(shows(&BREAK));
        back[k - 1] += u32::from(w[1].pp_count == 0 && shows(&RETURN));
    }
    (broke, back)
}

/// Innis's protect broken twice by Kite: each break shows its picture
/// (effect 20, ANM_xdhprom0) and each return, 300 frames on, its own
/// (effect 24, ANM_xdhprol1), which the boss's `CalcReal` raises.
#[test]
fn innis_protect_breaks_and_returns_with_pictures() {
    let Some(mut s) = event_107_arena() else { return };
    let (broke, back) = protect_twice(&mut s, 5000);
    println!("Innis: break shown {broke:?} frames, return {back:?}");
    assert!(broke.iter().all(|&n| n > 30), "a break with no picture: {broke:?}");
    assert!(back.iter().all(|&n| n > 30), "a return with no picture: {back:?}");
}

/// The same on Skeith, Infection's boss: its return is its `CalcReal`'s
/// too.
#[test]
fn skeith_protect_breaks_and_returns_with_pictures() {
    let Some(mut s) = event_30_arena() else { return };
    let (broke, back) = protect_twice(&mut s, 6000);
    println!("Skeith: break shown {broke:?} frames, return {back:?}");
    assert!(broke.iter().all(|&n| n > 30), "a break with no picture: {broke:?}");
    assert!(back.iter().all(|&n| n > 30), "a return with no picture: {back:?}");
}

/// Innis's class.
fn innis(c: &Combat, me: usize) -> Option<&piney_battle::boss::innis::Innis> {
    match &c.scene.chars[me].foe_state()?.boss.as_ref()?.class {
        Class::Innis(x) => Some(x),
        _ => None,
    }
}

/// Frames of Innis's fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/i69/shots): every 10th of its magic (act 4)
/// and its images (act 5), every 4th while a protect picture is live,
/// with the gauge at half from frame 300 (the magic's run).
#[test]
#[ignore]
fn innis_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i69/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_107_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut gs: Option<piney_gs::Gs> = None;
    let mut came = None;
    let mut drawn: BTreeMap<String, u32> = BTreeMap::new();
    for n in 0..9000u32 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].hp = c.scene.chars[k].max_hp;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i == 300
            && let Some(f) = c.scene.chars[me].foe_state_mut()
        {
            f.pp = f.row.max_pp / 2;
        }
        for d in &census.boss_draws {
            *drawn.entry(d.clone()).or_default() += 1;
        }
        let act = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map_or(-1, |b| b.act_num);
        let prot = census.effects.iter().any(|e| BREAK.contains(e) || RETURN.contains(e));
        let word = innis(c, me).map_or(0, |x| x.move_flg);
        if (matches!(act, 4 | 5) && i.is_multiple_of(10)) || (prot && i.is_multiple_of(4)) {
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(&s));
            g.render(&frame);
            let (w, h) = g.target_size();
            let path = format!("{dir}/innis-{i:05}-act{act}-w{word}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        }
        if i > 6000 {
            break;
        }
    }
    println!("drawn {drawn:?}");
}
