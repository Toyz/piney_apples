//! A Data Bug through the session: Infection's row 115 (type 0x40, base
//! form 113's `EET1` and clips, its second model from `EETX`) put as an
//! event's enemy entry puts one, in front of Kite in a random field of Dun
//! Loireag's server, and fought there.

use piney_input::Raw;
use piney_world::combat::Look;

use super::*;

/// The row, and its HP's half: `affectEnemy` never takes a virus-flagged
/// enemy below it.
const BUG: i32 = 115;
const HALF: i16 = 20169 / 2;

/// The Data Bug is made with its own row's look (not its base form's,
/// which shares its clips), drawn with both models, and the console's
/// `kill` (a hit of 9999 from Kite, every 20 frames) leaves it standing
/// at half its HP.
#[test]
fn a_data_bug_is_drawn_and_not_beaten_by_damage() {
    let Some((mut s, _)) = super::ride::in_field() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut bug, mut drawn, mut hits, mut hp) = (None, 0, 0, Vec::new());
    for _ in 0..1200u32 {
        pad.read(&still);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let w = a.world_mut();
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 30);
        let c = w.combat_mut();
        let Some(k) = c.kite else { continue };
        c.scene.chars[k].hp = c.scene.chars[k].max_hp;
        let kite = c.scene.chars[k].pos;
        let Some(who) = bug else {
            if playing {
                let at = [(f32::from_bits(kite[0]) + 600.0).to_bits(), kite[1], kite[2], kite[3]];
                bug = Some(w.put_enemy(BUG, at, 0).expect("the Data Bug made"));
            }
            continue;
        };
        let actor = c.cast.get(who).expect("the Data Bug's actor");
        assert_eq!(actor.look, Look::Enemy(BUG));
        assert!(actor.ch.anim_name().is_some_and(|n| n.starts_with("ANM_eet1")), "{:?}", actor.ch.anim_name());
        if actor.drawn && actor.matrix.is_some() && actor.second.is_some() && actor.second_draw.is_some() {
            drawn += 1;
        }
        assert_ne!(c.scene.chars[who].spc_char.enemy_flags & piney_battle::chara::enemy_flag::VIRUS, 0);
        hp.push(c.scene.chars[who].hp);
        if hp.len().is_multiple_of(20) && drawn > 0 {
            a.kill_enemies();
            hits += 1;
        }
    }
    println!("drawn {drawn} frames, {hits} hits, hp {:?}", hp.iter().step_by(60).collect::<Vec<_>>());
    assert!(drawn > 100, "the Data Bug was drawn on {drawn} frames");
    assert!(hits >= 12, "only {hits} hits landed");
    assert!(hp.iter().all(|&h| h >= HALF), "its HP fell below half: {hp:?}");
    assert_eq!(hp.last(), Some(&HALF));
}
