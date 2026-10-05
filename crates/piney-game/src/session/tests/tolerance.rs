//! Issue #49: a foe's immunity in its target window. `ccMenuCtrl::Disp`
//! names an enemy's or boss's lowest `Exdefense` bit right of its HP
//! ("Fire Tol.", `kyviaStatusStr`), and strikes it through once that
//! defence is lowered. The port left the line empty.

use piney_battle::param::elm;
use piney_fieldui::spr::{Obj, Packet};

use super::revive::{area, field_with_mia_and_elk, step};
use super::*;

/// Hell Hound, `enemyTbl` row 172: fire, `Exdefense` 0x10 (fire).
const HELL_HOUND: i32 = 172;

/// Story area 14's field, the party up, Hell Hounds put 600 ahead of Kite
/// and one of them the command target: (session, its scene index).
fn hell_hound_targeted() -> Option<(Session, usize)> {
    let mut s = field_with_mia_and_elk()?;
    let w = area(&mut s).world_mut();
    let c = w.combat_mut();
    for p in c.members.iter().map(|m| m.1).collect::<Vec<_>>() {
        c.scene.chars[p].no_death = true;
    }
    let (pos, dirc) = (c.scene.chars[c.kite.unwrap()].pos, f32::from_bits(c.kite_dirc()[2]));
    let mut at = pos;
    at[0] = (f32::from_bits(at[0]) - 600.0 * dirc.sin()).to_bits();
    at[1] = (f32::from_bits(at[1]) + 600.0 * dirc.cos()).to_bits();
    let hound = w.put_enemy(HELL_HOUND, at, 0).expect("the Hell Hounds");
    // The command target fixed on it, as the field's menus fix it.
    w.change_command_target(Some((piney_world::entry::Kind::Enemy, hound as i32)));
    w.set_target_fix(true);
    step(&mut s, 30);
    assert_eq!(area(&mut s).world().command_target(), Some(hound), "the Hell Hound targeted");
    Some((s, hound))
}

/// The hound's fire lowered by 10 for 60 seconds (`temp` and `time`, as
/// a debuff leaves them): `CalcReal`'s `real` falls below the row's.
fn lower_fire(s: &mut Session, hound: usize) {
    let c = area(s).world_mut().combat_mut();
    let f = c.scene.chars[hound].foe_state_mut().unwrap();
    f.temp[elm::FIRE] = -10;
    f.time[elm::FIRE] = 3600;
}

/// The menu's packets of `obj` in the last frame's drawing.
fn packets(s: &mut Session, obj: Obj) -> Vec<Packet> {
    let ui = area(s).ui();
    ui.draws()
        .iter()
        .flat_map(|d| match d {
            piney_fieldui::ctrl::Draw::Send(ps) | piney_fieldui::ctrl::Draw::Kanji { packets: ps, .. } => ps.clone(),
            _ => Vec::new(),
        })
        .filter(|p| p.obj == obj)
        .collect()
}

/// The immunity's line and the bar over it (row 0x1800 of `xwin`), each
/// at (145, 122).
fn tolerance_drawn(s: &mut Session) -> (bool, Option<f32>) {
    let at = |p: &Packet| (p.dx, p.dy) == (145.0, 122.0);
    let line = packets(s, Obj::NameKanji).iter().any(|p| p.code == 5 && at(p));
    let bar = packets(s, Obj::ItemIcon).iter().find(|p| p.wv == 0x1800 && at(p)).map(|p| p.sx);
    (line, bar)
}

/// The targeted Hell Hound's window names "Fire Tol." beside its HP, and
/// once its fire is lowered below its row's the line is struck through
/// (a bar 60 wide, the piece's). Before, neither was drawn.
#[test]
fn a_foes_immunity_shows_in_its_target_window() {
    let Some((mut s, hound)) = hell_hound_targeted() else { return };
    let text = area(&mut s).ui().ctrl.name_text.clone();
    let lines: Vec<&[u8]> = text.chunks(16).collect();
    assert!(
        lines.get(5).is_some_and(|l| l.starts_with(b"Fire Tol.")),
        "nameKanji: {:?}",
        String::from_utf8_lossy(&text)
    );
    assert_eq!(tolerance_drawn(&mut s), (true, None), "(the line, the bar)");
    lower_fire(&mut s, hound);
    step(&mut s, 2);
    assert_eq!(tolerance_drawn(&mut s), (true, Some(60.0)), "(the line, the bar) once lowered");
}

/// Shots of the target window (ignored; a diagnostic): the Hell Hound's,
/// its fire held and lowered, to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/issue49).
#[test]
#[ignore]
fn tolerance_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/issue49".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some((mut s, hound)) = hell_hound_targeted() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    for name in ["held", "lowered"] {
        if name == "lowered" {
            lower_fire(&mut s, hound);
        }
        let mut pad = Pad::default();
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        let mut frame = s.step(&pad);
        // A frame with the immunity's pulse at its height.
        for _ in 0..40 {
            if area(&mut s).ui().ctrl.cursol_alpha == 10 {
                break;
            }
            frame = s.step(&pad);
            s.take_events();
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(&s));
        g.render(&frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/hell-hound-{name}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        println!("{path}");
    }
}
