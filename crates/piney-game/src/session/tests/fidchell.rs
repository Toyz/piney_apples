//! Fidchell in its arena: Outbreak's event 207 at its block 9 (field 4 of
//! town 3, as area 73's dungeon leaves the game), the prediction and each
//! spell and a skill forced in turn; what shows and sounds.

use std::path::PathBuf;

use piney_battle::boss::Class;
use piney_battle::boss::fidchell::act;
use piney_input::Raw;

use super::*;

/// Event 207 brought to Fidchell's arena: blocks 0-8 played and
/// `eventStatus[0]` 1, the session in field 4 of town 3 with area 73's
/// `WORLD_MAN`.
fn event_207_arena() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/outbreak/outbreak.iso");
    if !iso.exists() {
        eprintln!("outbreak.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 207).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(207);
    state.save.set_event_flag(207, f | ((1 << 9) - 1));
    state.save.set_u8(offset::EVENT_STATUS, 1);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 73, false).unwrap();
    scene.change_scene(1, 3, 4, -1, -1, -1, &mut state.save);
    Some(Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap())
}

/// The forcing: from so many frames after Fidchell comes, at its first
/// frame in act 0, the act it is put into (its process and count 0), with
/// the spell for act 5.
const SCHEDULE: [(u32, i16, i32); 6] = [
    (60, act::PREDICTION, -1),
    (500, act::EXEC_PREDICTION, 0),
    (1000, act::EXEC_PREDICTION, 2),
    (1500, act::EXEC_PREDICTION, 3),
    (2000, act::EXEC_PREDICTION, 1),
    (2600, act::SKILL, -1),
];
/// The frames run after Fidchell comes, and the most before it does.
const FRAMES: u32 = 4000;
const ARRIVAL: u32 = 3000;

/// What a frame showed of the boss.
#[derive(Default)]
struct Seen {
    act: i16,
    text: bool,
    named: bool,
    draws: Vec<String>,
    events: Vec<Event>,
    effects: Vec<i16>,
}

/// [`SCHEDULE`] run until [`FRAMES`] frames after Fidchell comes, Kite
/// still and kept alive; `each` sees every frame from its coming, counted
/// from it.
fn run(s: &mut Session, mut each: impl FnMut(&Session, u32, &Frame, &Seen)) {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut came = None;
    let mut next = 0;
    for n in 0..ARRIVAL + FRAMES {
        pad.read(&still);
        let frame = s.step(&pad);
        let events = s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        if let Some(k) = c.kite {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        if came.is_none() && c.boss_char().is_some() {
            came = Some(n);
        }
        let Some(i) = came.map(|c| n - c) else { continue };
        if i >= FRAMES {
            break;
        }
        let mut seen = Seen { draws: census.boss_draws, events, effects: census.effects, ..Seen::default() };
        if let Some(r) = c.boss.as_ref() {
            seen.text = r.text_shown;
            seen.named = r.cinema.draw_name;
        }
        if let Some(me) = c.boss_char()
            && let Some(b) = c.scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut())
        {
            seen.act = b.act_num;
            if let Some(&(_, act, pred)) = SCHEDULE.get(next).filter(|x| i >= x.0 && b.act_num == act::NEUTRAL) {
                next += 1;
                b.act_num = act;
                b.act_proccess = 0;
                b.act_count = 0;
                if let Class::Fidchell(x) = &mut b.class
                    && pred >= 0
                {
                    x.pred_id = pred;
                }
            }
        }
        each(s, i, &frame, &seen);
    }
}

/// The prediction's text drawn and its voice asked (field voice group
/// -40); each spell's pictures (the meteors' fire and glow, the bolts,
/// the towers, the ice's pieces); the skill's start and its cinema name.
#[test]
fn fidchell_spells_show() {
    let Some(mut s) = event_207_arena() else { return };
    let (mut text, mut voices, mut named, mut frames) = (0, Vec::new(), 0, 0);
    let mut drawn: std::collections::BTreeMap<String, u32> = Default::default();
    let mut skill_start = 0;
    run(&mut s, |_, _, _, seen| {
        frames += 1;
        text += u32::from(seen.text);
        named += u32::from(seen.named && seen.act == act::SKILL);
        for e in &seen.events {
            if let Event::Voice { event, msg } = e {
                voices.push((*event, *msg));
            }
        }
        for d in &seen.draws {
            *drawn.entry(d.clone()).or_default() += 1;
        }
        // effSkillStart's controller (-12) with its rings (82-84).
        skill_start += u32::from(seen.effects.contains(&-12) && seen.effects.contains(&82));
    });
    println!(
        "{frames} frames: text {text}, voices {voices:?}, named {named}, skill start {skill_start}, drawn {drawn:?}"
    );
    assert_eq!(frames, FRAMES, "Fidchell came too late");
    assert!(text > 100, "the prediction's text showed {text} frames");
    assert!(voices.iter().any(|v| v.0 == -40), "no prediction voice: {voices:?}");
    for d in ["ANM_x300", "CMP_x100", "CMP_x012", "CMP_x101b", "CMP_x201d"] {
        assert!(drawn.get(d).is_some_and(|&n| n > 0), "{d} never drawn: {drawn:?}");
    }
    assert!(skill_start > 0, "no effSkillStart");
    assert!(named > 0, "the skill's cinema had no name");
}

/// Frames of [`fidchell_spells_show`]'s run to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/fidchell-fx): every 8th after each forcing
/// for 480 frames, every 60th otherwise.
#[test]
#[ignore]
fn fidchell_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/fidchell-fx".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_207_arena() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    run(&mut s, |s, i, frame, seen| {
        let busy = SCHEDULE.iter().any(|x| (x.0..x.0 + 480).contains(&i));
        if !(busy && i % 8 == 0 || i % 60 == 59) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/fid-{i:04}-act{}.png", seen.act);
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}
