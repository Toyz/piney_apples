//! Issue #66: Mutation's event 155 (NUKE-1), Nuke Usagimaru's battle talk
//! show. Block 10 has him fight a monster under the event's cameras, then
//! turns the camera on Kite with the fight behind it ("He calls it a show,
//! but maybe he needs some help?"). The monster's weapon flashes and the
//! fight's hit marks belong on the fight, never on Kite.

use piney_draw::{Cmd, PrimKind};
use piney_input::{Buttons, Raw};

use super::side_events::{MUT_CASES, play_with};
use super::*;

/// Nuke Usagimaru's `charTbl` row.
const NUKE: i32 = 5;

/// One frame of block 10: whether the active camera has its back to Nuke
/// (the fight), whether a weapon flash shines (`ccPrimRadiate` rays), the
/// untextured strips drawn on `refLayer` (rays and trails), the effects'
/// and particles' draws, and whether a hit ring (`CMP_x037`) was drawn.
struct Shot {
    behind: bool,
    flash: bool,
    strips: usize,
    draws: usize,
    rings: bool,
}

/// Event 155 played by the side-event survey's pilot (god on) to block 10;
/// then `hold` frames of it, its messages moved on every 30 frames until
/// the camera turns to Kite and left up after, each frame recorded and
/// handed to `each`.
fn the_show(hold: u64, mut each: impl FnMut(&Session, &Frame, u64)) -> Option<Vec<Shot>> {
    let iso = crate::check_image("mutation");
    let case = MUT_CASES.iter().find(|c| c.event == 155)?;
    let mut shots = Vec::new();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    play_with("mutation", &iso, case, 12000, true, &mut |s, _, _| {
        let playing =
            |s: &Session| matches!(&s.stage, Stage::Area(a) if a.vm().and_then(|v| v.playing()) == Some((155, 10)));
        if !shots.is_empty() || !playing(s) {
            return;
        }
        let mut pad = Pad::default();
        let mut on_kite = false;
        for f in 0..hold {
            let press = !on_kite && f.is_multiple_of(30);
            pad.read(&if press { Raw { buttons: Buttons::CROSS, ..still } } else { still });
            let frame = s.step(&pad);
            s.take_events();
            let Stage::Area(a) = &s.stage else { break };
            let c = a.world().combat();
            let Some(nuke) = c.who(NUKE).map(|n| c.scene.chars[n].pos.map(f32::from_bits)) else { break };
            let cam = a.world().camera().active();
            let (eye, view) = (cam.pos.map(f32::from_bits), cam.view.map(f32::from_bits));
            let behind = (nuke[0] - eye[0]) * (view[0] - eye[0]) + (nuke[1] - eye[1]) * (view[1] - eye[1]) < 0.0;
            on_kite |= behind;
            let census = a.world().fx().census();
            let strips = (frame.cmds.iter())
                .filter(|c| matches!(c, Cmd::Prim(p) if p.kind == PrimKind::Strip && p.state.texture.is_none()))
                .count();
            shots.push(Shot {
                behind,
                flash: !c.rays.is_empty(),
                strips,
                draws: census.effect_draws.len(),
                rings: census.effect_draws.iter().any(|d| d == "CMP_x037"),
            });
            each(s, &frame, f);
        }
    })?;
    Some(shots)
}

/// Issue #66: a weapon flash's rays go out through `ccPrimPacket::makePacket`,
/// whose second pair kicks the ray's triangles only in front of the eye
/// and on the screen (`pair_continues`); the port drew every ray, so a
/// flash behind the camera came out mirrored onto Kite. The effects' cone
/// is the active camera's (`ecam`), so the fight's marks are not sent
/// either. Facing the fight, its flashes and hit rings are drawn.
#[test]
fn the_shows_flashes_and_marks_stay_on_the_fight() {
    let Some(shots) = the_show(2800, |_, _, _| {}) else { return };
    let behind: Vec<&Shot> = shots.iter().filter(|s| s.behind).collect();
    assert!(behind.len() >= 300, "the shot on Kite: {} frames", behind.len());
    let flashes = behind.iter().filter(|s| s.flash).count();
    assert!(flashes >= 10, "a weapon flash behind the camera on {flashes} frames");
    for (i, s) in behind.iter().enumerate() {
        assert_eq!((s.strips, s.draws), (0, 0), "the fight drawn behind the camera ({i})");
    }
    let facing = shots.iter().filter(|s| !s.behind);
    let (rays, rings) =
        facing.fold((0, 0), |(r, g), s| (r + usize::from(s.flash && s.strips > 0), g + usize::from(s.rings)));
    assert!(rays >= 10 && rings >= 30, "facing the fight: rays on {rays} frames, hit rings on {rings}");
}

/// Shots of [`the_shows_flashes_and_marks_stay_on_the_fight`] every 5
/// frames of the shot on Kite to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/i66).
#[test]
#[ignore]
fn the_show_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i66".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs: Option<piney_gs::Gs> = None;
    the_show(2800, |s, frame, f| {
        if f < 2400 || !f.is_multiple_of(5) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        std::fs::write(format!("{dir}/show-{f:04}.png"), piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}
