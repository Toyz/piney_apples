//! The battle rules against Infection's disc: the tables read from it, and
//! a few worked cases the docs quote (docs/engine/battle.md). Skipped when
//! the disc image is not extracted; set PINEY_ISO to point at it
//! elsewhere.
//!
//! The rules themselves are checked against the game's own code by
//! tools/test_battle_rs.py.

use std::path::PathBuf;

use piney_battle::chara::{self, Char, Env};
use piney_battle::damage::{self, Roll};
use piney_battle::exp;
use piney_battle::param::{Base, SpcParam, elm, ty};
use piney_battle::tables::{self, Tables};
use piney_data::iso::Iso;

fn tables() -> Option<Tables> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    if !p.exists() {
        return None;
    }
    let mut iso = Iso::open(p).unwrap();
    Some(Tables::read(&mut iso).unwrap())
}

fn kite(t: &Tables) -> Char {
    let p: SpcParam = t.chars[0].param;
    let mut c = Char::pc(p);
    let mut ev = Vec::new();
    chara::calc_real(t, &mut c, 1, &Env::default(), &mut ev);
    c
}

#[test]
fn table_sizes() {
    let Some(t) = tables() else { return };
    assert_eq!(t.skills.len(), tables::SKILLS);
    assert_eq!(t.enemies.len(), tables::ENEMIES);
    assert_eq!(t.bosses.len(), tables::BOSSES);
    assert_eq!(t.items.iter().map(Vec::len).collect::<Vec<_>>(), [24, 72, 34, 3, 22, 291]);
    assert_eq!(t.weapons.iter().map(Vec::len).collect::<Vec<_>>(), [82, 77, 97, 75, 74, 76]);
    assert_eq!(t.chars.len(), 18);
    assert_eq!(t.level_up.len(), 18);
    // Every row of the enemy and boss tables is its own id.
    assert!(t.enemies.iter().enumerate().all(|(i, e)| i32::from(e.param.base.id) == i as i32));
    assert!(t.bosses.iter().enumerate().all(|(i, b)| i32::from(b.base.id) == i as i32));
    // Kite is type 7, the others 6; enemies 0x20, bosses 0x80.
    assert_eq!(t.chars[0].param.base.ty, 7);
    assert!(t.chars[1..].iter().all(|c| c.param.base.ty == 6));
    assert!(t.enemies.iter().all(|e| e.param.base.ty & ty::FOE != 0));
    assert_eq!(t.exp_calc, [1, 2, 3, 4, 6, 8, 13, 28, 40, 50, 60, 70, 80, 100, 130, 170, 220, 280, 350, 430, 520]);
    assert_eq!(t.race_num.iter().sum::<i32>(), tables::ENEMIES as i32);
    assert_eq!(t.erosion, [7, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 20]);
    assert_eq!(t.area_items.iter().map(Vec::len).collect::<Vec<_>>(), [30, 30, 5, 30, 30, 5]);
}

#[test]
fn kite_starts_with_his_gear() {
    let Some(t) = tables() else { return };
    let k = kite(&t);
    assert_eq!(k.real()[..8], [35, 54, 43, 73, 24, 54, 36, 66]);
}

#[test]
fn kite_at_level_ten() {
    let Some(t) = tables() else { return };
    let mut k = kite(&t);
    while k.level() < 10 {
        k.spc_mut().unwrap().base.exp = 1000;
        chara::check_level_up(&t, &mut k);
    }
    let p = k.spc().unwrap();
    assert_eq!((p.base.level, p.max_hp, p.max_sp), (10, 225, 40));
    assert_eq!(p.elm[..4], [60, 50, 150, 150]);
    assert_eq!(exp::exp_for_kill(&t, 5, 5, true), 60);
}

#[test]
fn kite_hits_a_goblin() {
    let Some(t) = tables() else { return };
    let k = kite(&t);
    let row = t.enemies.iter().find(|e| e.param.name == b"Goblin").unwrap().param.clone();
    let sk = t.skill(1).unwrap().clone();
    let mut hits = Vec::new();
    for r in 0..=100 {
        let mut g = Char::foe(row.clone());
        let mut none = || 0;
        let d = damage::calc_battle_damage(
            &t,
            &k,
            &mut g,
            &sk,
            damage::F_ONE,
            Roll::Quiet(r),
            true,
            &mut none,
            &Env::default(),
            &mut Vec::new(),
        );
        if d.dmg >= 0 {
            hits.push(d.dmg);
        }
    }
    assert_eq!(hits.len(), 55);
    assert_eq!((*hits.iter().min().unwrap(), *hits.iter().max().unwrap()), (8, 16));
}

#[test]
fn a_trap_never_misses() {
    let Some(t) = tables() else { return };
    let trap = Char::other(Base { ty: 0x100, ..Base::default() }, [0; 16]);
    let row = t.enemies[1].param.clone();
    let mut g = Char::foe(row);
    let mut sk = t.skill(1).unwrap().clone();
    sk.ty = 1;
    let mut rng = piney_battle::Rand::default();
    let d = damage::calc_battle_damage(
        &t,
        &trap,
        &mut g,
        &sk,
        damage::F_ONE,
        Roll::Draw,
        true,
        &mut rng,
        &Env::default(),
        &mut Vec::new(),
    );
    assert!(d.dmg >= 1);
    assert_eq!(d.hit, Some(100));
    let _ = elm::P_ATK;
}
