//! BUGS.md, Infection's missing particle effects: Kite casts skills of
//! `skillTbl` on a Mimic, and every effect slot a cast starts must draw
//! its object on the frames it is in view (`dispSW` on after a pass of
//! `ccEffect::Main`); an element art lights his blades
//! (`StartArmsEffect`'s generators), and the particles are drawn.

use std::collections::BTreeMap;

use piney_data::volume::Volume;
use piney_effect::files::{Assets, Kind};

use super::revive::{area, field_with_mia_and_elk, step};
use super::*;

/// Mimic, `enemyTbl` row 240.
const MIMIC: i32 = 240;

/// What `FxCensus::effect_draws` calls a draw of effect `id`'s object;
/// None for an id with no object (a controller).
fn drawn_as(a: &Assets, id: i16) -> Option<String> {
    let obj = a.adrs(id, false)?;
    match a.kind(id) {
        Kind::Clump | Kind::Anm => Some(a.name(obj).to_string()),
        k if k.is_eff() => Some(format!("eff {}", a.eff_chunk(obj)?.0)),
        _ => None,
    }
}

/// One cast as the effects showed it.
#[derive(Debug, Default)]
struct Cast {
    /// Each effect id with an object: frames in view, frames drawn.
    ids: BTreeMap<i16, (u32, u32)>,
    /// The most generators along a weapon at once.
    arms: usize,
    /// The particles' draws over the cast.
    particle_draws: usize,
}

impl Cast {
    /// The ids in view on some frame and never drawn.
    fn never_drawn(&self) -> Vec<i16> {
        self.ids.iter().filter(|(_, (seen, drawn))| *seen > 0 && *drawn == 0).map(|(id, _)| *id).collect()
    }
}

/// Kite, the party and a Mimic in event 19's field; each of `sids` cast by
/// Kite (or the party's member `by`) on the Mimic in turn, until its run and its effects end. Data
/// Drain's rows (2-5) are the drain menu's, which owns the camera: asked
/// for bare they leave the event camera on. None without the disc.
fn survey(by: Option<i32>, sids: impl IntoIterator<Item = i32>) -> Option<Vec<(i32, Cast)>> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut disc = Iso::open(&iso).ok()?;
    let data = Archive::new(disc.read_path("DATA/DATA.BIN").ok()?).ok()?;
    let assets = Assets::read(&data, Volume::Inf).ok()?;
    let mut s = field_with_mia_and_elk()?;
    let w = area(&mut s).world_mut();
    let k = w.combat().kite.unwrap();
    let mut at = w.combat().scene.chars[k].pos;
    at[0] = (f32::from_bits(at[0]) + 300.0).to_bits();
    let foe = w.put_enemy(MIMIC, at, 0).expect("the Mimic");
    let me = by.map_or(Some(k), |id| w.combat().who(id)).expect("the caster");
    step(&mut s, 2);
    let mut out = Vec::new();
    for sid in sids {
        let c = area(&mut s).world_mut().combat_mut();
        if c.data.t.skill(sid).is_none_or(|p| p.name.is_empty()) || (2..=5).contains(&sid) {
            continue;
        }
        for i in [me, foe] {
            let ch = &mut c.scene.chars[i];
            (ch.hp, ch.max_hp, ch.sp, ch.max_sp) = (9999, 9999, 999, 999);
            ch.no_death = true;
        }
        c.skill_request(me, Some(foe), sid);
        let mut cast = Cast::default();
        for f in 0..900 {
            step(&mut s, 1);
            let w = area(&mut s).world();
            let census = w.fx().census();
            cast.arms = cast.arms.max(census.weapon_generators);
            cast.particle_draws += census.particle_draws;
            for &id in census.effects.iter().filter(|id| !census.effects_unseen.contains(id)) {
                let Some(name) = drawn_as(&assets, id) else { continue };
                let e = cast.ids.entry(id).or_default();
                e.0 += 1;
                e.1 += u32::from(census.effect_draws.contains(&name));
            }
            let running = w.combat().skills.borrow().runs.iter().any(|r| r.id == sid);
            if f > 30 && !running && census.effects.is_empty() {
                break;
            }
        }
        out.push((sid, cast));
    }
    Some(out)
}

/// One skill of each kind: Flame Dance (a fire art: the blades' fire),
/// Wildflower (a wood art's shock wave), Repth (a heal), Gan Rom (a
/// tornado), Yarthkins (a summons), Rue Kruz (a convergence).
#[test]
fn a_skill_of_each_kind_draws_its_effects() {
    let Some(casts) = survey(None, [9, 19, 150, 197, 205, 213]) else { return };
    assert_eq!(casts.len(), 6);
    for (sid, cast) in &casts {
        assert!(!cast.ids.is_empty(), "{sid}: no effect in view");
        assert_eq!(cast.never_drawn(), Vec::<i16>::new(), "{sid}: {:?}", cast.ids);
        assert!(cast.particle_draws > 0, "{sid}: no particle drawn");
    }
    assert!(casts[0].1.arms >= 2, "Flame Dance's blades: {:?}", casts[0].1.arms);
}

/// Every row of `skillTbl`: each effect it starts is drawn while in view.
/// `cargo test --release -p piney-game every_skills_effects_draw --
/// --ignored --nocapture` (2-3 minutes).
#[test]
#[ignore]
fn every_skills_effects_draw() {
    let Some(casts) = survey(None, 0..400) else { return };
    let mut missing = Vec::new();
    for (sid, cast) in &casts {
        println!("{sid:3} arms {} particles {} {:?}", cast.arms, cast.particle_draws, cast.ids);
        if !cast.never_drawn().is_empty() {
            missing.push((*sid, cast.never_drawn()));
        }
    }
    println!("{} skills cast", casts.len());
    assert!(missing.is_empty(), "effects never drawn: {missing:?}");
}

/// Elk (a Wavemaster) casts a fire art: `ccFellow::Action`'s
/// `StartArmsEffect` lights his staff, and its effects draw.
#[test]
fn a_members_art_lights_his_weapon() {
    let Some(casts) = survey(Some(10), [9]) else { return };
    let (_, cast) = &casts[0];
    assert_eq!(cast.never_drawn(), Vec::<i16>::new(), "{:?}", cast.ids);
    assert!(cast.arms >= 1, "Elk's staff: {}", cast.arms);
}
