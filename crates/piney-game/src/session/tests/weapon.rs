//! The enemies' weapon trails through the session: after event 3, the
//! sword goblin of the portal east of the start (`egn1`) fighting Orca
//! and Kite, its sword (`OBJ_egn1swor`: four edges, a spline cell between
//! each two, the strip packet) swung at them.

use super::*;

/// Event 3 played through (its goblin, the skill and chat lessons), then
/// Kite walked to the east portal and left standing while Orca fights its
/// goblin: each frame the goblin's trail is drawn, (frame, pairs in the
/// strips); `each` sees the session, the frame drawn and the pairs after
/// every step. None without the disc.
fn goblin_trail(mut each: impl FnMut(&Session, &piney_draw::Frame, usize)) -> Option<Vec<(u64, usize)>> {
    let (mut s, mut f) = story_to_field(0)?;
    let mut log = Vec::new();
    story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
    let mut seen = Vec::new();
    let mut n = 0u64;
    walk_to_east_portal_frames(&mut s, 900, false, |s, frame| {
        n += 1;
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        let pairs: usize = c.trails.iter().map(|t| t.polys.len()).sum();
        if pairs > 0 {
            seen.push((n, pairs));
        }
        each(s, frame, pairs);
    });
    Some(seen)
}

/// The goblin swings: its trail is drawn on the frames it swings, the
/// strip through several cells (the spline cells between its frames' too).
#[test]
fn goblin_swing_draws_a_trail() {
    let Some(seen) = goblin_trail(|_, _, _| {}) else { return };
    println!("trail frames {seen:?}");
    assert!(!seen.is_empty(), "the goblin never swung");
    let most = seen.iter().map(|&(_, n)| n).max().unwrap_or(0);
    // Four edges make three runs of the cells, each a pair a cell.
    assert!(most >= 3 * 4, "the trail stayed short: {seen:?}");
}

/// Pictures of the goblin's swing: the frames its trail is longest, each
/// whole (`swing-N.png`) and cut about the trail, twice the size
/// (`swing-N-near.png`).
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game weapon_shots --
/// --ignored --nocapture` (default `/mnt/data/claude/scratch/weapon/shots`).
#[test]
#[ignore]
fn weapon_shots() {
    use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/weapon/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let Ok(mut disc) = Iso::open(&iso) else { return };
    let data = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut gs: Option<piney_gs::Gs> = None;
    let mut n = 0;
    goblin_trail(|s, frame, pairs| {
        if pairs < 12 || n >= 6 {
            return;
        }
        let gs = gs.get_or_insert_with(|| {
            piney_gs::Gs::headless(piney_gs::Assets::new(Mode::archive(s).unwrap_or(data.clone()))).unwrap()
        });
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let (w, h) = gs.target_size();
        let px = gs.read_back();
        let path = format!("{dir}/swing-{n}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &px)).unwrap();
        println!("{path}: {pairs} pairs");
        // The trail's points on the screen, and about them.
        let Stage::Area(a) = &s.stage else { return };
        let ws = a.world().camera().world_screen;
        let (sx, sy) = (w as f32 / 512.0, h as f32 / 448.0);
        let pts: Vec<(f32, f32)> = a
            .world()
            .combat()
            .trails
            .iter()
            .flat_map(|t| t.polys.iter().flat_map(|q| q.pos))
            .map(|p| {
                let g = piney_world::ee::rot_trans_pers(&ws, p);
                ((g[0] - XYOFFSET_X) as f32 / 16.0 * sx, (g[1] - XYOFFSET_Y) as f32 / 16.0 * sy)
            })
            .collect();
        let lo = |f: fn(&(f32, f32)) -> f32, m: f32| (pts.iter().map(f).fold(f32::MAX, f32::min) - m).max(0.0) as u32;
        let hi = |f: fn(&(f32, f32)) -> f32, m: f32, e: u32| {
            (pts.iter().map(f).fold(f32::MIN, f32::max) + m).min(e as f32 - 1.0) as u32
        };
        let (x0, y0, x1, y1) = (lo(|p| p.0, 64.0), lo(|p| p.1, 64.0), hi(|p| p.0, 64.0, w), hi(|p| p.1, 64.0, h));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (cw, ch) = ((x1 - x0) * 2, (y1 - y0) * 2);
        let mut cut = Vec::with_capacity((cw * ch * 4) as usize);
        for y in 0..ch {
            for x in 0..cw {
                let k = (((y0 + y / 2) * w + x0 + x / 2) * 4) as usize;
                cut.extend_from_slice(&px[k..k + 4]);
            }
        }
        let path = format!("{dir}/swing-{n}-near.png");
        std::fs::write(&path, piney_gs::png::encode(cw, ch, &cut)).unwrap();
        println!("{path}");
        n += 1;
    });
}

/// The party's weapon trails (`ccSpcChar::ArmsEffect`): as Orca's heavy
/// blade (job 1) swings at the goblin, `trajectorySW` on, his trail
/// (`ccLattice(2, 0)`, white with no aura) is sent once 16 rows have been
/// laid; Kite, standing, lays none.
#[test]
fn orcas_swings_leave_trails() {
    let mut orca_frames = 0;
    let mut kite_frames = 0;
    let seen = goblin_trail(|s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        for (&who, actor) in &c.cast.actors {
            let Some(w) = actor.weapon.as_ref() else { continue };
            if w.strips.is_empty() {
                continue;
            }
            assert!(w.strips.iter().all(|(st, _)| st.len() >= 4 && st.len() % 2 == 0));
            if Some(who) == c.kite {
                kite_frames += 1;
            } else {
                assert_eq!(w.job, 1, "Orca's heavy blade");
                assert!(w.strips.iter().all(|(st, _)| st[0].rgba[..3] == [0xf0; 3]));
                orca_frames += 1;
            }
        }
    });
    if seen.is_none() {
        return;
    }
    println!("Orca's trail frames {orca_frames}, Kite's {kite_frames}");
    assert!(orca_frames > 0, "Orca's swings left no trail");
    assert_eq!(kite_frames, 0, "Kite stood still");
}

/// Pictures of Orca's trail: the frames it is longest, whole
/// (`orca-N.png`) and cut about the trail, twice the size
/// (`orca-N-near.png`).
/// `PINEY_SHOTS=DIR cargo test --release -p piney-game party_trail_shots
/// -- --ignored --nocapture` (default `/mnt/data/claude/scratch/weapon/shots`).
#[test]
#[ignore]
fn party_trail_shots() {
    use piney_desktop::view::{XYOFFSET_X, XYOFFSET_Y};
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/weapon/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let Ok(mut disc) = Iso::open(&iso) else { return };
    let data = Arc::new(Archive::new(disc.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut gs: Option<piney_gs::Gs> = None;
    let mut n = 0;
    goblin_trail(|s, frame, _| {
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        let pts: Vec<piney_world::ee::V4> = c
            .cast
            .actors
            .values()
            .filter_map(|x| x.weapon.as_ref())
            .flat_map(|w| w.strips.iter().flat_map(|(st, _)| st.iter().map(|v| v.pos)))
            .collect();
        if pts.len() < 12 || n >= 6 {
            return;
        }
        let gs = gs.get_or_insert_with(|| {
            piney_gs::Gs::headless(piney_gs::Assets::new(Mode::archive(s).unwrap_or(data.clone()))).unwrap()
        });
        gs.set_overlay(Mode::archive(s));
        gs.render(frame);
        let (w, h) = gs.target_size();
        let px = gs.read_back();
        let path = format!("{dir}/orca-{n}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &px)).unwrap();
        println!("{path}: {} vertices", pts.len());
        let ws = a.world().camera().world_screen;
        let (sx, sy) = (w as f32 / 512.0, h as f32 / 448.0);
        let pts: Vec<(f32, f32)> = pts
            .iter()
            .map(|&p| {
                let g = piney_world::ee::rot_trans_pers(&ws, p);
                ((g[0] - XYOFFSET_X) as f32 / 16.0 * sx, (g[1] - XYOFFSET_Y) as f32 / 16.0 * sy)
            })
            .collect();
        let lo = |f: fn(&(f32, f32)) -> f32, m: f32| (pts.iter().map(f).fold(f32::MAX, f32::min) - m).max(0.0) as u32;
        let hi = |f: fn(&(f32, f32)) -> f32, m: f32, e: u32| {
            (pts.iter().map(f).fold(f32::MIN, f32::max) + m).min(e as f32 - 1.0) as u32
        };
        let (x0, y0, x1, y1) = (lo(|p| p.0, 64.0), lo(|p| p.1, 64.0), hi(|p| p.0, 64.0, w), hi(|p| p.1, 64.0, h));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (cw, ch) = ((x1 - x0) * 2, (y1 - y0) * 2);
        let mut cut = Vec::with_capacity((cw * ch * 4) as usize);
        for y in 0..ch {
            for x in 0..cw {
                let k = (((y0 + y / 2) * w + x0 + x / 2) * 4) as usize;
                cut.extend_from_slice(&px[k..k + 4]);
            }
        }
        let path = format!("{dir}/orca-{n}-near.png");
        std::fs::write(&path, piney_gs::png::encode(cw, ch, &cut)).unwrap();
        println!("{path}");
        n += 1;
    });
}
