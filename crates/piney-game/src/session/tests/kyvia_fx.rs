//! Kyvia's fights: the meteors (`ANM_x300`) with their trails, the beams'
//! particles following their ends, the core's auras following it, the
//! second fight's thunderbolts (`CMP_x012`), the arm, the gomoras' pushes
//! on each other and their shapes, and the stick's turn of the camera.

use std::collections::BTreeMap;
use std::path::PathBuf;

use piney_battle::boss::Class;
use piney_input::{Buttons, Raw};
use piney_world::camera::id::BATTLE;
use piney_world::combat::Combat;

use super::*;

/// Event `n` of `disc` brought to Kyvia's disc, field `field` of town 2
/// (`EVENTAREAB8`), as area 47's dungeon leads there: blocks below `block`
/// played and `eventStatus[0]` 1 (block `block` makes Kyvia).
fn on_the_disc(disc: &str, n: i32, block: u32, field: i32) -> Option<Session> {
    on_the_disc_in(disc, n, block, (2, 47), field, 1)
}

/// [`on_the_disc`] from story area `area`'s dungeon of town `town`, with
/// `eventStatus[0]` `status` (Quarantine's events clear it in their first
/// block and set it after their fight).
pub(super) fn on_the_disc_in(
    disc: &str,
    n: i32,
    block: u32,
    (town, area): (i32, i32),
    field: i32,
    status: u8,
) -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
    if !iso.exists() {
        eprintln!("{disc}.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, n).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(n as usize);
    state.save.set_event_flag(n as usize, f | ((1 << block) - 1));
    state.save.set_u8(offset::EVENT_STATUS, status);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, area, false).unwrap();
    let wm = crate::area::ev_area_number(&iso, field, &wm, &state.save).unwrap();
    scene.change_scene(1, town, field, -1, -1, -1, &mut state.save);
    Some(Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap())
}

/// Mutation's event 108 on field 9's disc (block 21 makes Kyvia 01).
pub(super) fn event_108_disc() -> Option<Session> {
    on_the_disc("mutation", 108, 21, 9)
}

/// Outbreak's event 211 on field 10's disc (block 11 makes Kyvia 02).
pub(super) fn event_211_disc() -> Option<Session> {
    on_the_disc("outbreak", 211, 11, 10)
}

/// Quarantine's event 308 on field 11's disc in town 4, from area 102's
/// dungeon (block 6 makes Kyvia 03; block 8 sets `eventStatus[0]` once
/// it is gone).
pub(super) fn event_308_disc() -> Option<Session> {
    on_the_disc_in("quarantine", 308, 6, (4, 102), 11, 0)
}

/// What one frame of the fight showed: the body's act, the boss effects'
/// draws, the generators following the fight's moving places (the beams'
/// ends, a gomora's aura, the meteors) and those following the core.
#[derive(Default)]
pub(super) struct Seen {
    pub(super) act: i16,
    draws: Vec<String>,
    anchored: usize,
    on_core: usize,
    /// `camID` (the boss camera's 2 throughout every Kyvia fight).
    pub(super) cam_id: i16,
}

/// The core's character.
fn core_of(c: &Combat, me: usize) -> Option<usize> {
    match &c.scene.chars[me].foe_state()?.boss.as_ref()?.class {
        Class::Kyvia(x) => Some(x.core),
        _ => None,
    }
}

/// The fight for `frames` frames from the body's coming, the party kept
/// up, the event's lines moved on, and each time the core is out its
/// time run short (`NowDmgtime` to `Dmgtime`), so the body strikes in
/// turn: the arm, the beams, the meteors (and in the second fight the
/// thunder). `each` sees every frame.
pub(super) fn fight(s: &mut Session, frames: u32, each: impl FnMut(&Session, &Frame, u32, &Seen)) -> Vec<Seen> {
    fight_leaning(s, frames, |_| 128, true, each)
}

/// [`fight`] with the right stick's x at `rx(i)` on the fight's frame `i`
/// (128 centred; leant, its y a step down, as straight sideways reads as
/// angle 0), the core's time cut short only with `hurry`.
fn fight_leaning(
    s: &mut Session,
    frames: u32,
    rx: impl Fn(u32) -> u8,
    hurry: bool,
    mut each: impl FnMut(&Session, &Frame, u32, &Seen),
) -> Vec<Seen> {
    let mut pad = Pad::default();
    let (mut came, mut seen) = (None, Vec::new());
    for n in 0..frames + 6000 {
        let lean = came.map_or(128, |c| rx(n - c));
        let ry = if lean == 128 { 128 } else { 129 };
        let still = Raw { analog: true, lx: 128, ly: 128, rx: lean, ry, ..Raw::default() };
        let talking = match &s.stage {
            Stage::Area(a) => a.vm().is_some_and(|v| v.playing().is_some()) || a.ui().ctrl.check_menu_type() != -1,
            _ => false,
        };
        pad.read(&if talking && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let cam_id = a.world().camera().cam_id;
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let Some(core) = core_of(c, me) else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        if hurry
            && c.scene.listed(core)
            && let Some(Class::KyviaCore(x)) =
                c.scene.chars[core].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
        {
            x.now_dmg_time = x.now_dmg_time.max(x.dmg_time - 1);
        }
        let now = Seen {
            act: c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map_or(-1, |b| b.act_num),
            draws: census.boss_draws,
            anchored: census.anchored_generators,
            on_core: census.char_generators.iter().filter(|&&g| g as usize == core).count(),
            cam_id,
        };
        each(s, &frame, i, &now);
        seen.push(now);
    }
    seen
}

/// Frames each boss effect draw showed in, and the frames with generators
/// following the fight's places and the core.
fn tally(seen: &[Seen]) -> (BTreeMap<String, u32>, u32, u32) {
    let mut drawn: BTreeMap<String, u32> = BTreeMap::new();
    for now in seen {
        let mut names = now.draws.clone();
        names.dedup();
        for d in names {
            *drawn.entry(d).or_default() += 1;
        }
    }
    let anchored = seen.iter().filter(|n| n.anchored > 0).count() as u32;
    let on_core = seen.iter().filter(|n| n.on_core > 0).count() as u32;
    (drawn, anchored, on_core)
}

/// Mutation's first fight: the meteors fall drawn (`ANM_x300`) with their
/// trails, the beams' and the gomoras' generators follow their places,
/// and the core's auras follow the core.
#[test]
fn kyvia_shows_its_meteors_beams_and_auras() {
    let Some(mut s) = event_108_disc() else { return };
    let seen = fight(&mut s, 12000, |_, _, _, _| {});
    let (drawn, anchored, on_core) = tally(&seen);
    let acts: std::collections::BTreeSet<i16> = seen.iter().map(|n| n.act).collect();
    println!("acts {acts:?}, drawn {drawn:?}, anchored {anchored} frames, on the core {on_core} frames");
    assert!(drawn.get("ANM_x300").is_some_and(|&n| n > 10), "the meteors not drawn: {drawn:?}");
    assert!(anchored > 100, "no generator followed the fight's places ({anchored} frames)");
    assert!(on_core > 100, "no aura followed the core ({on_core} frames)");
    assert!(seen.iter().all(|n| n.cam_id == BATTLE), "camID left the boss camera");
}

/// Outbreak's second fight: its thunder's bolts are drawn (`CMP_x012`),
/// and its meteors.
#[test]
fn kyvia_s_second_fight_shows_its_thunderbolts() {
    let Some(mut s) = event_211_disc() else { return };
    let seen = fight(&mut s, 16000, |_, _, _, _| {});
    let (drawn, anchored, on_core) = tally(&seen);
    let acts: std::collections::BTreeSet<i16> = seen.iter().map(|n| n.act).collect();
    println!("acts {acts:?}, drawn {drawn:?}, anchored {anchored} frames, on the core {on_core} frames");
    assert!(acts.contains(&6), "no thunder: {acts:?}");
    assert!(drawn.get("CMP_x012").is_some_and(|&n| n > 30), "the bolts not drawn: {drawn:?}");
    assert!(on_core > 100, "no aura followed the core ({on_core} frames)");
    assert!(seen.iter().all(|n| n.cam_id == BATTLE), "camID left the boss camera");
}

/// Kyvia's arm in Legion's Reach (act 3, #72): `HandAtk` draws `anmw`
/// (`ANM_ex0batc0`, the arm's `MDL_ex0bbod1`) with `DrawParts` each frame
/// of its sweep, and every `AFtiming`-th frame enters an after-image of
/// it, so the arm shows more than once. Before, neither was drawn.
#[test]
fn kyvias_arm_sweeps_with_its_after_images() {
    for (disc, s) in [("x01", event_108_disc()), ("x02", event_211_disc())] {
        let Some(mut s) = s else { continue };
        let ccs = piney_data::ccs::Ccs::parse(s.archive.inflate_named(disc).unwrap()).unwrap();
        let arm = ccs.find_object("MDL_ex0bbod1").unwrap();
        let (mut shown, mut doubled) = (0u32, 0u32);
        fight(&mut s, 6000, |_, frame, _, now| {
            if now.act != 3 {
                return;
            }
            let n = (frame.cmds.iter())
                .filter(|c| matches!(c, piney_draw::Cmd::Model(m) if m.file == disc && m.model == arm))
                .count();
            shown += u32::from(n > 0);
            doubled += u32::from(n > 1);
        });
        println!("{disc}: the arm in {shown} frames, with after-images in {doubled}");
        assert!(shown > 30, "{disc}: the arm drawn in {shown} frames of act 3");
        assert!(doubled > 10, "{disc}: its after-images in {doubled} frames");
    }
}

/// Each gomora's draws of `MDL_ex01gom1` this frame: the morph targets it
/// was bent toward, by name.
fn gomora_morphs(frame: &Frame, disc: &str, ccs: &piney_data::ccs::Ccs, base: u32) -> Vec<Vec<(String, f32)>> {
    (frame.cmds.iter())
        .filter_map(|c| match c {
            piney_draw::Cmd::Model(m) if m.file == disc && m.model == base => {
                Some((m.morph.iter()).map(|&(t, w)| (ccs.object_name(t).unwrap_or("?").to_string(), w)).collect())
            }
            _ => None,
        })
        .collect()
}

/// The gomoras' shapes (worklog 430): every gomora draws its clip's
/// objects (`CMP_trall3`'s), and `MPH_ex01gom1` bends `MDL_ex01gom1`
/// through `MDL_ex01gom2`, `3` and `4` in turn as `ANM_ex0gnut0` plays,
/// toward the `atc` shapes in its attacks; each by its own clip, so
/// gomoras out together differ. Before, every gomora drew the bare base.
#[test]
fn kyvias_gomoras_bend_through_their_shapes() {
    for (disc, s) in [("x01", event_108_disc()), ("x02", event_211_disc())] {
        let Some(mut s) = s else { continue };
        let ccs = piney_data::ccs::Ccs::parse(s.archive.inflate_named(disc).unwrap()).unwrap();
        let base = ccs.find_object("MDL_ex01gom1").unwrap();
        let (mut targets, mut drawn, mut apart) = (BTreeMap::<String, u32>::new(), 0u32, 0u32);
        fight_leaning(
            &mut s,
            6000,
            |_| 128,
            false,
            |_, frame, _, _| {
                let ms = gomora_morphs(frame, disc, &ccs, base);
                drawn += u32::from(!ms.is_empty());
                apart += u32::from(ms.iter().any(|m| m != &ms[0]));
                for m in &ms {
                    assert!(m.iter().all(|(_, w)| (0.0..=1.0).contains(w)), "{disc}: {m:?}");
                    for (t, w) in m {
                        if *w > 0.0 {
                            *targets.entry(t.clone()).or_default() += 1;
                        }
                    }
                }
            },
        );
        println!("{disc}: gomoras drawn in {drawn} frames, bent apart in {apart}: {targets:?}");
        for t in ["MDL_ex01gom2", "MDL_ex01gom3", "MDL_ex01gom4"] {
            assert!(targets.get(t).copied().unwrap_or(0) > 100, "{disc}: {t} in {targets:?}");
        }
        assert!(targets.keys().any(|t| t.starts_with("MDL_ex01atc")), "{disc}: no attack's shape");
        assert!(apart > 100, "{disc}: gomoras bent alike but in {apart} frames");
    }
}

/// The gomoras' `bodyHit`s.
fn gomora_hits(s: &Session) -> Vec<piney_battle::world::CharHit> {
    let Stage::Area(a) = &s.stage else { return Vec::new() };
    let c = a.world().combat();
    let Some(core) = c.boss_char().and_then(|me| core_of(c, me)) else { return Vec::new() };
    let Some(Class::KyviaCore(x)) = c.scene.chars[core].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class)
    else {
        return Vec::new();
    };
    (x.gomoras.iter())
        .filter_map(|&g| match &c.scene.chars[g].foe_state()?.boss.as_ref()?.class {
            Class::Gomora(gx) => Some(gx.hit),
            _ => None,
        })
        .collect()
}

/// The gomoras push each other apart (`kyviaGomora::Move`'s
/// `CollisionDetection`): out of the core their bodies are in the field's
/// list (kind and mask 128) and a gomora run into another is put back out
/// of it. Before, no gomora had a body in the list and none was pushed.
#[test]
fn kyvias_gomoras_push_each_other_apart() {
    for (disc, s) in [("mutation", event_108_disc()), ("outbreak", event_211_disc())] {
        let Some(mut s) = s else { continue };
        let (mut listed, mut pushed) = (0u32, 0u32);
        fight_leaning(
            &mut s,
            6000,
            |_| 128,
            false,
            |s, _, _, _| {
                let gs = gomora_hits(s);
                listed += u32::from(gs.iter().any(|h| h.sw && h.kind == 0x80 && h.mask == 0x80));
                pushed += u32::from(gs.iter().any(|h| h.offset != [0; 4]));
            },
        );
        println!("{disc}: gomoras' bodies listed in {listed} frames, one pushed in {pushed}");
        assert!(listed > 1000, "{disc}: the gomoras' bodies listed in {listed} frames");
        assert!(pushed > 0, "{disc}: no gomora pushed");
    }
}

/// Kyvia's camera turns about Kite with the right stick (`ZrotControlFlg`,
/// set by the constructor): leant right then left, `ZrotCont` goes to +0.5
/// and back to -0.5, except while an attack's `SetMode(3)` holds the pad
/// (`ExLock` bit 4). Before, the stick did not turn it.
#[test]
fn the_stick_turns_kyvias_camera() {
    let Some(mut s) = event_108_disc() else { return };
    let (mut most, mut least, mut held) = (0f32, 0f32, 0u32);
    let mut last = None;
    fight_leaning(
        &mut s,
        3000,
        |i| if (i / 300) % 2 == 0 { 255 } else { 0 },
        true,
        |s, _, _, _| {
            let Stage::Area(a) = &s.stage else { return };
            let Some(cam) = a.world().combat().boss.as_ref().and_then(|r| r.cam.as_ref()) else { return };
            let z = f32::from_bits(cam.zrot_cont);
            (most, least) = (most.max(z), least.min(z));
            if cam.ex_lock & 0x10 != 0 && last == Some(cam.zrot_cont) {
                held += 1;
            }
            last = Some(cam.zrot_cont);
            assert!(cam.zrot_control, "the constructor's ZrotControlFlg");
        },
    );
    println!("ZrotCont from {least} to {most}, held through {held} frames of mode 3");
    assert_eq!((least, most), (-0.5, 0.5));
    assert!(held > 100, "the pad read while mode 3 held it ({held} frames unmoved)");
}

/// Shots to `PINEY_SHOTS` (default /mnt/data/claude/scratch/a427): the
/// first fight with the stick leant right then left (the eye turned about
/// Kite), every 40th frame, and frames a gomora was pushed.
#[test]
#[ignore]
fn kyvia_push_and_turn_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a427".into());
    let mut shots = Shots::new(&dir);
    let Some(mut s) = event_108_disc() else { return };
    let mut pushes = 0u32;
    fight_leaning(
        &mut s,
        2400,
        |i| if (i / 600) % 2 == 0 { 255 } else { 0 },
        false,
        |s, frame, i, now| {
            let pushed = gomora_hits(s).iter().any(|h| h.offset != [0; 4]);
            pushes += u32::from(pushed);
            let Stage::Area(a) = &s.stage else { return };
            let z = a.world().combat().boss.as_ref().and_then(|r| r.cam.as_ref()).map_or(0, |c| c.zrot_cont);
            let tag = if pushed && pushes.is_multiple_of(25) {
                Some("push")
            } else if i.is_multiple_of(40) {
                Some("turn")
            } else {
                None
            };
            let path = tag.map(|tag| format!("{dir}/kyvia-{tag}-{i:05}-act{}-z{:+.2}.png", now.act, f32::from_bits(z)));
            shots.frame(s, frame, path);
        },
    );
}

/// Shots to `PINEY_SHOTS` (default /mnt/data/claude/scratch/a430) of both
/// fights, every 50th frame the gomoras are drawn, named by the shapes
/// the first drawn is bent toward.
#[test]
#[ignore]
fn kyvia_gomora_shape_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a430".into());
    std::fs::create_dir_all(&dir).unwrap();
    for (disc, s) in [("x01", event_108_disc()), ("x02", event_211_disc())] {
        let Some(mut s) = s else { continue };
        let ccs = piney_data::ccs::Ccs::parse(s.archive.inflate_named(disc).unwrap()).unwrap();
        let base = ccs.find_object("MDL_ex01gom1").unwrap();
        let mut shots = Shots::new(&dir);
        fight_leaning(
            &mut s,
            3000,
            |_| 128,
            false,
            |s, frame, i, now| {
                let ms = gomora_morphs(frame, disc, &ccs, base);
                let path = (!ms.is_empty() && i.is_multiple_of(50)).then(|| {
                    let shape: Vec<String> = ms[0]
                        .iter()
                        .map(|(t, w)| format!("{}{:.0}", t.trim_start_matches("MDL_ex01"), w * 100.0))
                        .collect();
                    format!("{dir}/gomora-{disc}-{i:05}-act{}-{}.png", now.act, shape.join("-"))
                });
                shots.frame(s, frame, path);
            },
        );
    }
}

/// Shots of both fights to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a420): every 6th frame a meteor or a bolt is
/// drawn, or the beams' generators run, through each fight's first 4000.
#[test]
#[ignore]
fn kyvia_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a420".into());
    std::fs::create_dir_all(&dir).unwrap();
    for (name, s) in [("mut", event_108_disc()), ("out", event_211_disc())] {
        let Some(mut s) = s else { continue };
        let mut shots = Shots::new(&dir);
        fight(&mut s, 4000, |s, frame, i, now| {
            let shown = now.draws.iter().any(|d| d == "ANM_x300" || d == "CMP_x012") || now.anchored > 0;
            let path = (shown && i.is_multiple_of(6)).then(|| format!("{dir}/kyvia-{name}-{i:05}-act{}.png", now.act));
            shots.frame(s, frame, path);
        });
    }
}

/// Quarantine's event 312 on field 12's disc in town 0, from area 108's
/// dungeon (block 10 makes Kyvia 04).
pub(super) fn event_312_disc() -> Option<Session> {
    on_the_disc_in("quarantine", 312, 10, (0, 108), 12, 0)
}
