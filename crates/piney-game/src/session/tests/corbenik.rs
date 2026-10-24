//! Corbenik (`ccBoss08`, Quarantine's event 313 in field 8's arena): its
//! first two forms in one task, then the third once the save says so;
//! under god from the arena with shots. docs/engine/boss-corbenik.md.

use piney_battle::boss::Class;
use piney_battle::boss::corbenik::seed;

use super::arena::{EVENT_313, EVENT_313_THIRD, arena_session, god_shots_for};
use super::*;

fn corbenik(c: &Class) -> bool {
    matches!(c, Class::Corbenik(_) | Class::CorbenikB(_) | Class::CorbenikC(_))
}

/// Forms one and two under god from the arena to event 313's end (the
/// third made after the scene change): shots in `PINEY_SHOTS`, else the
/// scratch's `qua/corbenik`.
#[test]
#[ignore]
fn corbenik_under_god() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/corbenik".into());
    let Some(mut s) = arena_session(&EVENT_313, &[15, 6]) else { return };
    let n = std::env::var("PINEY_CORBENIK_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(240_000);
    let (p, music) = god_shots_for(&mut s, 313, corbenik, &dir, "corbenik", n);
    if n < 240_000 {
        return;
    }
    eprintln!("acts {:?} hurt {:?} ended {:?} music {music} of {}", p.acts, p.hurt, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 313 did not end: {}", Mode::title(&s));
}

/// The third form alone under god (block 19 on).
#[test]
#[ignore]
fn corbenik_third_under_god() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/qua/corbenik3".into());
    let Some(mut s) = arena_session(&EVENT_313_THIRD, &[15, 6]) else { return };
    let (p, music) = god_shots_for(&mut s, 313, corbenik, &dir, "corbenik3", 120_000);
    eprintln!("acts {:?} hurt {:?} ended {:?} music {music} of {}", p.acts, p.hurt, p.ended_at, p.frames);
    assert!(p.ended_at.is_some(), "event 313 did not end: {}", Mode::title(&s));
}

/// A diagnostic: the first frames of the fight, the boss's HP, the lists
/// and what the pilot does every two seconds (`PINEY_CORBENIK_FRAMES`).
#[test]
#[ignore]
fn corbenik_trace() {
    let n = std::env::var("PINEY_CORBENIK_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let Some(mut s) = arena_session(&EVENT_313, &[15, 6]) else { return };
    s.console("god");
    super::arena::play(&mut s, 313, corbenik, n, |s, _, f| {
        s.take_events();
        if !f.is_multiple_of(600) {
            return;
        }
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        let Some(me) = c.boss_char() else { return };
        let ch = &c.scene.chars[me];
        let act = ch.foe_state().and_then(|f| f.boss.as_ref()).map(|b| (b.act_num, b.exit, b.draw_sw));
        eprintln!(
            "F {f} hp {}/{} act {act:?} listed {} enemies {:?} target {:?} | {}",
            ch.hp,
            ch.max_hp,
            c.scene.listed(me),
            c.enemies(),
            a.world().command_target_code(),
            super::survey::fighters(a)
        );
    });
}

/// The HUD reads `eventMng.bossEntry` (7 once 313's block 15 enters
/// Corbenik), which keeps the battle banner off through the fight.
#[test]
fn the_banner_stays_off_through_the_fight() {
    let Some(mut s) = arena_session(&EVENT_313, &[15, 6]) else { return };
    let mut seen = None;
    super::arena::play(&mut s, 313, corbenik, 400, |s, _, _| {
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && a.world().combat().boss_char().is_some()
        {
            seen = Some(a.hud_world().boss_entry);
        }
    });
    assert_eq!(seen, Some(7), "the HUD's bossEntry once Corbenik is in");
}

/// Corbenik's three forms fought to the end of event 313 with no aid: no
/// god, Kite with BlackRose and Balmung as the story leaves them, the
/// pilot's own tactics. `PINEY_CORBENIK_GOD` (the console's god),
/// `PINEY_CORBENIK_EXP` (that much experience to each member) and
/// `PINEY_CORBENIK_TRACE` (the party's HP as it changes) are diagnostics.
#[test]
#[ignore]
fn corbenik_fight_to_its_end() {
    let Some(mut s) = arena_session(&EVENT_313, &[15, 6]) else { return };
    if std::env::var_os("PINEY_CORBENIK_GOD").is_some() {
        s.console("god");
    }
    let trace = std::env::var_os("PINEY_CORBENIK_TRACE").is_some();
    let exp = std::env::var("PINEY_CORBENIK_EXP").ok();
    let mut forms = Vec::new();
    let mut last: Vec<i16> = Vec::new();
    // The seeds broken and burst, and each one's act last frame.
    let mut seeds = (0, 0);
    let mut seed_acts = [0i16; 8];
    let p = super::arena::play(&mut s, 313, corbenik, 240_000, |s, _, f| {
        s.take_events();
        if f == 40
            && let Some(n) = &exp
        {
            eprintln!("{}", s.console(&format!("exp {n}")));
        }
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        let Some(me) = c.boss_char() else { return };
        let Some(b) = c.scene.chars[me].foe_state().and_then(|x| x.boss.as_ref()) else { return };
        if let Class::Corbenik(x) = &b.class {
            for (i, &k) in x.seed_me.iter().enumerate() {
                let Some(sb) = c.scene.chars[k].foe_state().and_then(|x| x.boss.as_ref()) else { continue };
                let now = sb.act_num;
                if now != seed_acts[i] {
                    match now {
                        seed::act::FIRE => seeds.1 += 1,
                        seed::act::DEAD if seed_acts[i] == seed::act::COUNT_DOWN => seeds.0 += 1,
                        _ => {}
                    }
                    seed_acts[i] = now;
                }
            }
        }
        let hp: Vec<i16> = c.members.iter().map(|&(_, k)| c.scene.chars[k].hp).collect();
        if trace && f == 40 {
            let k = &c.data.t.boss_skills[31];
            eprintln!(
                "boss skill 31 {} atk {} hit {} reach {} range {} ty {} rate {}",
                String::from_utf8_lossy(&k.name),
                k.atk,
                k.hit,
                f32::from_bits(k.trigger_range),
                f32::from_bits(k.target_range),
                k.ty,
                k.dmg_rate
            );
        }
        if trace && hp != last {
            eprintln!("F {f} act {} {} party {hp:?} boss {}", b.act_num, b.act_proccess, c.scene.chars[me].hp);
            let at = |k: usize| c.scene.chars[k].pos.map(f32::from_bits);
            let kite = c.members.first().map(|&(_, k)| at(k)).unwrap_or_default();
            let d = |q: [f32; 4]| (q[0] - kite[0]).hypot(q[1] - kite[1]) as i32;
            if let Class::Corbenik(x) = &b.class {
                let seeds: Vec<String> = (x.seed_me.iter())
                    .filter_map(|&k| {
                        let sb = c.scene.chars[k].foe_state()?.boss.as_ref()?;
                        (sb.exit == 0).then(|| format!("{}:{} d{}", sb.act_num, c.scene.chars[k].hp, d(at(k))))
                    })
                    .collect();
                let mates: Vec<i32> = c.members.iter().skip(1).map(|&(_, k)| d(at(k))).collect();
                eprintln!("  boss d{} mates {mates:?} seeds {seeds:?}", d(at(me)));
            }
        }
        last = hp;
        let form = match b.class {
            Class::Corbenik(_) => 1,
            Class::CorbenikB(_) => 2,
            Class::CorbenikC(_) => 3,
            _ => return,
        };
        if forms.last().map(|&(_, n, _)| n) != Some(form) {
            eprintln!("F {f} form {form} hp {}", c.scene.chars[me].hp);
            forms.push((f, form, c.scene.chars[me].hp));
        }
        if let Some(l) = forms.last_mut() {
            l.2 = l.2.min(c.scene.chars[me].hp);
        }
    });
    eprintln!(
        "levels {:?} forms (frame, form, lowest HP) {forms:?} seeds (broken, burst) {seeds:?} acts {:?} hurt {:?} over {:?} ended {:?} frames {}",
        p.levels, p.acts, p.hurt, p.over_at, p.ended_at, p.frames
    );
    assert!(p.over_at.is_none(), "the party fell at {:?}: {}", p.over_at, Mode::title(&s));
    assert!(p.ended_at.is_some(), "event 313 did not end: {}", Mode::title(&s));
}

/// Event 313's town part from its own story start (Lia Fail) under the
/// pilot: BlackRose and another member called before her "Are you
/// ready?" (her block for a party of two refuses), both answers Yes,
/// and on through block 14 (`area 8`, the scene to field 8's arena).
#[test]
fn event_313_answers_blackrose() {
    let Some(mut s) = story_session_on("quarantine", 313, |_| {}) else { return };
    let (mut pad, mut pilot) = (Pad::default(), super::survey::StoryPilot::default());
    let (mut at, mut party) = (None, [0, -1, -1]);
    for f in 0..20_000u64 {
        let raw = pilot.next(&s, f);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if super::survey::event_flag(&mut s, 313).is_some_and(|x| x & 1 << 14 != 0) {
            at = Some(f);
            break;
        }
        if let Stage::World(w) = &s.stage {
            party = w.world().party();
        }
    }
    let flags = super::survey::event_flag(&mut s, 313).unwrap_or(0);
    eprintln!("block 14 at {at:?}, party {party:?}");
    assert!(at.is_some(), "block 14 not played (blocks {:#x}): {}", flags & 0xffff, Mode::title(&s));
    assert!(party.contains(&15) && party[1..].iter().any(|&m| m != -1 && m != 15), "the party asked: {party:?}");
}
