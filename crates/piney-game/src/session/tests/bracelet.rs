//! Issue #40: event 30's block 18 in Chosen Hopeless Nothingness (field
//! 27, town 1): `radiator 0 2 0` makes gimmick row 19 with its rays
//! (`ccGimRadiator`) at Kite's right hand, the bracelet shining as he says
//! "The bracelet... it's shining...".

use std::path::PathBuf;

use piney_input::Raw;

use super::*;

/// BlackRose's `charTbl` row, whom the event wants along.
const BLACK_ROSE: i32 = 15;

/// Event 30 brought to Chosen Hopeless Nothingness: blocks 0-16 played,
/// `eventStatus[0]` 4, the session entering field 27 from Mac Anu
/// (area 27's `WORLD_MAN`) with BlackRose in the party.
fn area_27() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 30).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(30);
    state.save.set_event_flag(30, f | ((1 << 17) - 1));
    state.save.set_u8(offset::EVENT_STATUS, 4);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 27, false).unwrap();
    scene.change_scene(1, 1, 27, -1, -1, -1, &mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap();
    let Stage::Area(a) = &mut s.stage else { panic!("not in area 27") };
    let sp = a.world_mut().spcs_mut();
    let i = sp.entry_spc(BLACK_ROSE) as usize;
    sp.registry[i].party_flag = 1;
    sp.member_id[1] = BLACK_ROSE;
    sp.member_char[1] = Some(BLACK_ROSE);
    sp.num += 1;
    Some(s)
}

/// What a frame of the bracelet scene shows: the rays drawn (their count),
/// their light in the group, Kite's right hand and the rays' first point.
struct Look {
    rays: usize,
    lit: bool,
    hand: Option<[f32; 3]>,
    ray0: Option<[f32; 3]>,
    window: bool,
}

/// The session from [`area_27`] stepped until event 30's message 11 has
/// been up for `after` frames, each frame's [`Look`] given to `each`.
fn play(mut each: impl FnMut(&Session, u32, &Look, &Frame)) -> Option<Session> {
    let mut s = area_27()?;
    let mut pad = Pad::default();
    let mut window_at = None;
    for f in 0..3000u32 {
        // The window closed on CROSS once it has been up 40 frames.
        let push = window_at.is_some_and(|w| f == w + 40);
        let b = if push { piney_input::Buttons::CROSS } else { piney_input::Buttons::NONE };
        pad.read(&Raw { buttons: b, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { break };
        let w = a.world();
        let c = w.combat();
        let hand = c.kite.and_then(|k| c.cast.get(k)).and_then(|a| a.hand_now()).map(|m| m[3]);
        // Message 11's window: opened, not closed since.
        let calls = a.calls();
        let window = calls
            .iter()
            .rposition(|(_, c)| c.starts_with("message_open 30 11 "))
            .is_some_and(|i| !calls[i..].iter().any(|(_, c)| c == "message_close"));
        if window && window_at.is_none() {
            window_at = Some(f);
        }
        let look = Look {
            rays: c.rays.iter().map(Vec::len).sum(),
            lit: !c.rad_lights.is_empty(),
            hand: hand.map(|p| [p[0], p[1], p[2]].map(f32::from_bits)),
            ray0: c
                .rays
                .first()
                .and_then(|r| r.first())
                .map(|v| [v[0].pos[0], v[0].pos[1], v[0].pos[2]].map(f32::from_bits)),
            window,
        };
        each(&s, f, &look, &frame);
        if window_at.is_some_and(|w| f > w + 120) {
            break;
        }
    }
    Some(s)
}

/// The radiator's rays shine at Kite's right hand from the instruction
/// until the event lets him go: 32 rays a frame, their light in the
/// group, near the hand, and still there while message 11 is up.
#[test]
fn the_bracelet_shines_in_chosen_hopeless_nothingness() {
    let mut shining = 0u32;
    let mut with_window = 0u32;
    let mut lit = 0u32;
    let mut far = Vec::new();
    let Some(s) = play(|_, f, look, _| {
        if look.rays > 0 {
            shining += 1;
            assert_eq!(look.rays, 32, "frame {f}: the radiator's plate has 32 rays");
            with_window += u32::from(look.window);
            lit += u32::from(look.lit);
            if let (Some(h), Some(r)) = (look.hand, look.ray0) {
                let d = ((h[0] - r[0]).powi(2) + (h[1] - r[1]).powi(2) + (h[2] - r[2]).powi(2)).sqrt();
                if d > 30.0 {
                    far.push((f, d));
                }
            }
        }
    }) else {
        return;
    };
    let Stage::Area(a) = &s.stage else { panic!("left the area: {}", Mode::title(&s)) };
    let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
    println!("shining {shining} frames, {with_window} with the window, lit {lit}");
    assert!(calls.iter().any(|c| c.starts_with("gimmick Radiator")), "no radiator: {calls:?}");
    assert!(with_window > 0, "the bracelet did not shine under message 11 ({shining} frames)");
    assert_eq!(lit, shining, "the rays' light was not in the group every frame they drew");
    assert!(far.is_empty(), "the rays strayed from the hand: {far:?}");
}

/// Shots of the bracelet scene: every 10th frame while the rays shine to
/// `$PINEY_SHOTS` (/mnt/data/claude/scratch/bracelet), and a list of the
/// frames they start and stop with the event's last calls then.
#[test]
#[ignore]
fn bracelet_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/bracelet".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs: Option<piney_gs::Gs> = None;
    let mut was = false;
    play(|s, f, look, frame| {
        let on = look.rays > 0;
        if on != was {
            println!("frame {f}: rays {} (window {})", look.rays, look.window);
            if let Stage::Area(a) = &s.stage {
                for (n, c) in a.calls().iter().rev().take(12).rev() {
                    println!("  {n}: {c}");
                }
            }
            was = on;
        }
        if !f.is_multiple_of(10) || (!on && !look.window) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/bracelet-{f:04}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        println!("{path}");
    });
}
