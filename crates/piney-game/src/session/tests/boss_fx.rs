//! Issue #69: what a boss's fight shows. The protect's break and its
//! return on Skeith, Innis and Gorre's brothers (the return raised by the
//! boss's own `CalcReal`), Innis's magic: its missiles, its rings and its
//! particles, and Gorre's brothers' shields.

use std::collections::BTreeMap;

use piney_battle::boss::Class;
use piney_battle::event::Event as Rule;
use piney_input::{Buttons, Raw};
use piney_world::combat::{Combat, Show};

use super::skeith::{event_30_arena, event_107_arena};
use super::*;

/// `effProtect`'s effects: the break's (19-21) and the return's (22-24).
const BREAK: std::ops::RangeInclusive<i16> = 19..=21;
const RETURN: std::ops::RangeInclusive<i16> = 22..=24;

/// What one frame of the fight left: the boss's act and gauge count, the
/// live effects, the boss effects' draws and the generators following
/// them.
#[derive(Default)]
struct Seen {
    act: i16,
    pp_count: i16,
    effects: Vec<i16>,
    draws: Vec<String>,
    anchored: usize,
    /// Innis's mirrors' shards drawn.
    shards: usize,
}

/// The fight in its arena for `frames` frames from the boss's coming, the
/// party kept alive and the event's messages moved on; `each` may change
/// the fight before the frame is seen.
fn fight(s: &mut Session, frames: u32, mut each: impl FnMut(&mut Combat, usize, u32, &Seen)) -> Vec<Seen> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut seen) = (None, Vec::new());
    for n in 0..frames + 3000 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        let f = c.scene.chars[me].foe_state();
        let now = Seen {
            act: f.and_then(|f| f.boss.as_ref()).map_or(-1, |b| b.base.act_num),
            pp_count: c.scene.param(me).map_or(0, |p| p.pp_count),
            effects: census.effects,
            draws: census.boss_draws,
            anchored: census.anchored_generators,
            shards: c.boss.as_ref().map_or(0, |r| r.shards.len()),
        };
        each(c, me, i, &now);
        seen.push(now);
    }
    seen
}

/// Kite's skill `sid` on the boss as `CalcBattleDamage` makes it (a sure
/// hit, live), the gauge one short of full and the save's `plcol` set: its
/// events shown as the frame shows a rule's. True when the protect broke.
fn break_protect(c: &mut Combat, me: usize, sid: i32) -> bool {
    let Some(k) = c.kite else { return false };
    let max = c.scene.chars[me].foe_state().map_or(0, |f| f.row.max_pp);
    if let Some(p) = c.scene.param_mut(me) {
        p.pp = max - 1;
    }
    let d = c.data.clone();
    let Some(sk) = d.t.skill(sid).cloned() else { return false };
    let env = piney_battle::chara::Env::default();
    let mut rng = piney_battle::rand::Rand::new(1);
    let mut ev = Vec::new();
    let roll = piney_battle::damage::Roll::Given(100);
    piney_battle::damage::calc_damage_in(
        &d.t,
        &mut c.scene,
        k,
        me,
        &sk,
        piney_effect::ONE,
        roll,
        &mut rng,
        &env,
        &mut ev,
    );
    let broke = ev.iter().any(|e| matches!(e, Rule::Protect { broken: 0, .. }));
    c.shows.extend(ev.into_iter().map(Show::Rule));
    broke
}

/// Two breaks of the boss's protect, each left to run out: the frames
/// each break's picture and each return's was live.
fn protect_twice(s: &mut Session, frames: u32) -> ([u32; 2], [u32; 2]) {
    let mut breaks = 0usize;
    let seen = fight(s, frames, |c, me, _, now| {
        let free = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).is_some_and(|b| b.base.lock_player == 0);
        // Each break after the last one's pictures are over.
        let quiet = !now.effects.iter().any(|e| BREAK.contains(e) || RETURN.contains(e));
        if breaks < 2
            && quiet
            && now.pp_count == 0
            && now.act == 0
            && free
            && c.scene.listed(me)
            && break_protect(c, me, 1)
        {
            breaks += 1;
        }
    });
    let (mut broke, mut back) = ([0; 2], [0; 2]);
    let mut k = 0usize;
    for w in seen.windows(2) {
        let shows = |r: &std::ops::RangeInclusive<i16>| w[1].effects.iter().any(|e| r.contains(e));
        // The count starting is a break; its end, the return.
        if w[0].pp_count == 0 && w[1].pp_count > 0 {
            k += 1;
        }
        if k == 0 || k > 2 {
            continue;
        }
        broke[k - 1] += u32::from(shows(&BREAK));
        back[k - 1] += u32::from(w[1].pp_count == 0 && shows(&RETURN));
    }
    (broke, back)
}

/// Innis's protect broken twice by Kite: each break shows its picture
/// (effect 20, ANM_xdhprom0) and each return, 300 frames on, its own
/// (effect 24, ANM_xdhprol1), which the boss's `CalcReal` raises.
#[test]
fn innis_protect_breaks_and_returns_with_pictures() {
    let Some(mut s) = event_107_arena() else { return };
    let (broke, back) = protect_twice(&mut s, 5000);
    println!("Innis: break shown {broke:?} frames, return {back:?}");
    assert!(broke.iter().all(|&n| n > 30), "a break with no picture: {broke:?}");
    assert!(back.iter().all(|&n| n > 30), "a return with no picture: {back:?}");
}

/// The same on Skeith, Infection's boss: its return is its `CalcReal`'s
/// too.
#[test]
fn skeith_protect_breaks_and_returns_with_pictures() {
    let Some(mut s) = event_30_arena() else { return };
    let (broke, back) = protect_twice(&mut s, 6000);
    println!("Skeith: break shown {broke:?} frames, return {back:?}");
    assert!(broke.iter().all(|&n| n > 30), "a break with no picture: {broke:?}");
    assert!(back.iter().all(|&n| n > 30), "a return with no picture: {back:?}");
}

/// Outbreak's event 218 brought to Gorre's arena: field 5 of town 3 as
/// area 71's dungeon leaves the game (block 9's `area 5` and scene),
/// blocks 0-9 played and `eventStatus[0]` 2.
pub(super) fn event_218_arena() -> Option<Session> {
    let iso = crate::check_image("outbreak");
    if !iso.exists() {
        eprintln!("outbreak.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 218).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(218);
    state.save.set_event_flag(218, f | ((1 << 10) - 1));
    state.save.set_u8(offset::EVENT_STATUS, 2);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 71, false).unwrap();
    let wm = crate::area::ev_area_number(&iso, 5, &wm, &state.save).unwrap();
    scene.change_scene(1, 3, 5, -1, -1, -1, &mut state.save);
    Some(Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap())
}

/// Gorre's protect is its brothers': a brother broken by Kite's blow
/// breaks the other with it (`SendMessage`, `effProtect(other, 0, 2)`),
/// so both show the break's picture; when the count runs out
/// (`CalcRealEx`) the return's picture shows on both brothers, never on
/// Gorre, whose body is not drawn.
#[test]
fn gorres_brothers_show_the_protect_break_and_return() {
    let Some(mut s) = event_218_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut broke) = (None, false);
    let (mut breaks, mut returns): (BTreeMap<u32, u32>, BTreeMap<u32, u32>) = Default::default();
    let mut brothers = Vec::new();
    let mut gorre = None;
    for n in 0..9000u32 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i > 1200 {
            break;
        }
        gorre = Some(me as u32);
        brothers = c.boss.as_ref().map_or(Vec::new(), |r| r.parts.clone());

        for &(id, t) in &census.effect_targets {
            let Some(t) = t else { continue };
            if BREAK.contains(&id) {
                *breaks.entry(t).or_default() += 1;
            }
            if RETURN.contains(&id) {
                *returns.entry(t).or_default() += 1;
            }
        }
        // Kite's blow on the first brother once it is on the lists, then
        // its hit as an affect (the brother's `Affect`, `SendMessage`).
        if !broke
            && i > 200
            && let Some(&b0) = brothers.first()
            && c.scene.listed(b0)
            && (break_protect(c, b0, 1) || break_protect(c, b0, 193))
        {
            if let Some(b) = c.scene.chars[b0].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                b.base.queued.push((1, [10, 0, 0], c.kite));
            }
            broke = true;
        }
    }
    println!("Gorre {gorre:?}, brothers {brothers:?}: breaks on {breaks:?}, returns on {returns:?}");
    assert!(broke, "no brother was broken");
    for b in &brothers {
        let b = *b as u32;
        assert!(breaks.get(&b).is_some_and(|&n| n > 30), "brother {b}: no break shown ({breaks:?})");
        assert!(returns.get(&b).is_some_and(|&n| n > 30), "brother {b}: no return shown ({returns:?})");
    }
    let g = gorre.unwrap();
    assert!(!breaks.contains_key(&g) && !returns.contains_key(&g), "a protect picture on Gorre itself");
}

/// Gorre's brothers on Gorre's `param` (worklog 431): the drain's 21 on a
/// brother (`SendMessage`'s `SetBaseParam(brother, row 4, &Gorre's)`)
/// points both at Gorre's block. A debuff on one brother (skill 163 as
/// `_ccSkillModifyCondition` writes it) is then on all three, and its time
/// runs down by Gorre's `CalcRealEx` and both brothers' `CalcReal`: three
/// a frame. Before, each brother kept its own copy, and the debuff stayed
/// on the one hit, counting down once a frame.
#[test]
fn gorres_brothers_share_its_param_after_the_drain() {
    let Some(mut s) = event_218_arena() else { return };
    let (mut drained, mut hit, mut gorre) = (false, None::<u32>, 0usize);
    let (mut shared, mut steps) = (0u32, BTreeMap::<i32, u32>::new());
    let (mut last, mut lowered) = (None::<i16>, 0u32);
    fight(&mut s, 2400, |c, me, i, _| {
        gorre = me;
        let brothers = c.boss.as_ref().map_or(Vec::new(), |r| r.parts.clone());
        let [b0, b1] = brothers[..] else { return };
        if !drained && i > 200 && c.scene.listed(b0) {
            let kite = c.kite;
            if let Some(b) = c.scene.chars[b0].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                b.base.queued.push((21, [0, 0, 0], kite));
            }
            drained = true;
        }
        let on_gorre = [b0, b1].iter().all(|&b| c.scene.param_owner(b) == me);
        shared += u32::from(on_gorre);
        if on_gorre && hit.is_none() && c.scene.listed(b1) {
            let t = c.data.t.clone();
            let mut ev = Vec::new();
            c.scene.lend(b1, |ch| piney_battle::skill::modify_condition(&t, ch, 163, &mut ev));
            hit = Some(i);
        }
        // Skill 163 lowers field 0; Gorre's and brother 0's see it.
        let time = c.scene.param(me).map(|p| p.time[0]);
        if let (Some(a), Some(b)) = (last, time)
            && a > 0
        {
            *steps.entry(i32::from(a - b)).or_default() += 1;
        }
        last = time;
        let row = c.scene.chars[b0].foe_state().map_or([0; 16], |f| f.row.elm);
        lowered += u32::from(c.scene.real(b0)[0] < row[0] && c.scene.real(me)[0] < row[0]);
    });
    println!(
        "on Gorre's block {shared} frames; the debuff from {hit:?}, its time's steps {steps:?}, lowered {lowered}"
    );
    assert!(shared > 1000, "the brothers on Gorre's block in {shared} frames");
    assert!(hit.is_some(), "no debuff");
    let threes = steps.get(&3).copied().unwrap_or(0);
    assert!(threes > 500 && threes * 10 > steps.values().sum::<u32>() * 9, "its time's steps {steps:?}");
    assert!(lowered > 500, "brother 0 and Gorre lowered in {lowered} frames");
    let _ = gorre;
}

/// Innis's class.
fn innis(c: &Combat, me: usize) -> Option<&piney_battle::boss::innis::Innis> {
    match &c.scene.chars[me].foe_state()?.boss.as_ref()?.class {
        Class::Innis(x) => Some(x),
        _ => None,
    }
}

/// `MagicSquare`'s word (`SwitchActionPattern` 19).
const MAGIC_WORD: i32 = 19;

/// Innis's fight with the gauge at half from frame 300 (the second mode's
/// run, with its magic), each magic's element forced in turn (ice,
/// lightning, fire) once its square is drawn (`EnyInt` set at count 2; the
/// missiles come at count 10 after the eye's move): each frame's draws
/// with the element of the magic then flying.
fn innis_magic(s: &mut Session, frames: u32) -> Vec<(Option<i32>, Seen)> {
    let (mut casts, mut flying) = (0, None);
    let mut elements = Vec::new();
    let seen = fight(s, frames, |c, me, i, now| {
        if i == 300 {
            let max = c.scene.chars[me].foe_state().map_or(0, |f| f.row.max_pp);
            if let Some(p) = c.scene.param_mut(me) {
                p.pp = max / 2;
            }
        }
        if let Some(Class::Innis(x)) =
            c.scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
            && x.move_flg == MAGIC_WORD
            && x.eny_flg == 0
            && x.cou == 3
        {
            x.eny_int = casts % 3;
            flying = Some(x.eny_int);
            casts += 1;
        }
        elements.push(if now.act == 4 { flying } else { None });
    });
    elements.into_iter().zip(seen).collect()
}

/// Innis's three magics show their missiles: the ice's crystals
/// (`CMP_x202a`) and, once the first lands, its hit and its explosion
/// (`ANM_ex31lhit`, `ANM_x305`); the lightning's bolts (`CMP_x500`) and the
/// WaveShock at the first's landing; the fire's fireballs (`ANM_x300`) and
/// the first's explosion. Each flies with its trails (the generators
/// following it). Before, nothing of the missiles was drawn.
#[test]
fn innis_magic_shows_its_missiles() {
    let Some(mut s) = event_107_arena() else { return };
    let seen = innis_magic(&mut s, 7000);
    let mut drawn: BTreeMap<(i32, String), u32> = BTreeMap::new();
    let mut trails: BTreeMap<i32, u32> = BTreeMap::new();
    for (el, now) in &seen {
        let Some(el) = *el else { continue };
        for d in &now.draws {
            *drawn.entry((el, d.clone())).or_default() += 1;
        }
        *trails.entry(el).or_default() += u32::from(now.anchored > 0);
    }
    println!("drawn {drawn:?}, trails {trails:?}");
    let shows = |el: i32, name: &str| drawn.get(&(el, name.to_string())).copied().unwrap_or(0);
    for (el, names) in [
        (0, ["CMP_x202a", "ANM_ex31lhit", "ANM_x305"]),
        (1, ["CMP_x500", "ANM_ex31lhit", "CMP_x500"]),
        (2, ["ANM_x300", "ANM_x305", "ANM_x300"]),
    ] {
        for n in names {
            assert!(shows(el, n) > 10, "magic {el}: {n} drawn {} frames ({drawn:?})", shows(el, n));
        }
        assert!(trails.get(&el).is_some_and(|&n| n > 30), "magic {el}: no trails ({trails:?})");
    }
}

/// One of Innis's magics as the player meets it, unforced: its frame, the
/// cinema's name row and the element of the missiles flying then.
struct Cast {
    frame: u32,
    row: i32,
    element: piney_battle::boss::innis::Element,
}

/// Innis's fight with the gauge at half from frame 300, Innis choosing
/// its magic itself (`ccRand() % 3`), until each element has come or
/// `frames` run out (70 frames past the last): each magic's first frame in
/// flight. `each` sees every frame with the casts so far.
fn innis_unforced(s: &mut Session, frames: u32, mut each: impl FnMut(&Session, &Frame, u32, &[Cast])) -> Vec<Cast> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut casts, mut flying) = (None, Vec::<Cast>::new(), false);
    for n in 0..frames + 3000 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        let all = [0, 1, 2].iter().all(|&r| casts.iter().any(|x| x.row == r));
        if i >= frames || all && casts.last().is_some_and(|c| i > c.frame + 70) {
            break;
        }
        if i == 300 {
            let max = c.scene.chars[me].foe_state().map_or(0, |f| f.row.max_pp);
            if let Some(p) = c.scene.param_mut(me) {
                p.pp = max / 2;
            }
        }
        let b = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref());
        let element = b.and_then(|b| {
            b.base.effects.slots.iter().flatten().find_map(|e| match e.kind {
                piney_battle::boss::EffKind::Missile { element, .. } => Some(element),
                _ => None,
            })
        });
        match (element, c.boss.as_ref().and_then(|r| r.cinema.name.as_ref())) {
            (Some(element), Some(name)) if !flying => {
                casts.push(Cast { frame: i, row: name.row, element });
                flying = true;
            }
            (None, _) => flying = false,
            _ => {}
        }
        each(s, &frame, i, &casts);
    }
    casts
}

/// Unforced, each of Innis's magics names its own element: `MagicSquare`
/// (MUT gcmn 0x0049a000) draws `n = |ccRand() % 3|`, turns cinema 5 + n on
/// (`TEX_ini_skl` row n: Absolute Zero, the lightning's, Demon Fire) and at
/// count 10 calls `IceAttack`, `ThunderAttack` or `BlazeAttack` by the same
/// n. Each magic's name row is its missiles' element.
#[test]
fn innis_magic_names_its_own_missiles() {
    use piney_battle::boss::innis::Element;
    let Some(mut s) = event_107_arena() else { return };
    let casts = innis_unforced(&mut s, 30000, |_, _, _, _| {});
    let seen: Vec<(u32, i32, Element)> = casts.iter().map(|c| (c.frame, c.row, c.element)).collect();
    println!("casts {seen:?}");
    for c in &casts {
        let want = match c.element {
            Element::Ice => 0,
            Element::Lightning => 1,
            Element::Blaze => 2,
        };
        assert_eq!(c.row, want, "frame {}: the name's row against {:?}", c.frame, c.element);
    }
    for r in 0..3 {
        assert!(casts.iter().any(|c| c.row == r), "row {r} never cast: {seen:?}");
    }
}

/// Shots of [`innis_magic_names_its_own_missiles`]'s fight to
/// `PINEY_SHOTS` (default /mnt/data/claude/scratch/i69/unforced): 30 and 60
/// frames into each magic's flight.
#[test]
#[ignore]
fn innis_unforced_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i69/unforced".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_107_arena() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    innis_unforced(&mut s, 30000, |s, frame, i, casts| {
        let Some(c) = casts.last().filter(|c| [c.frame + 30, c.frame + 60].contains(&i)) else { return };
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/innis-{i:05}-row{}-{:?}.png", c.row, c.element);
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}

/// Innis's images burst into rings (`CMP_x037`, models 191 and 193 of
/// xeffect) and their mirrors into shards (`CMP_ex21ef*` of x21), and its
/// fall raises a ring (192), as the rules make them.
#[test]
fn innis_images_burst_into_rings_and_shards() {
    let Some(mut s) = event_107_arena() else { return };
    let seen = innis_magic(&mut s, 3000);
    let rings = seen.iter().filter(|(_, n)| n.draws.iter().any(|d| d == "CMP_x037")).count();
    let images = seen.iter().filter(|(_, n)| n.act == 5 && n.draws.iter().any(|d| d == "CMP_x037")).count();
    let shards = seen.iter().filter(|(_, n)| n.act == 5 && n.shards > 0).count();
    println!("rings {rings} frames, {images} of the images', shards {shards} frames");
    assert!(images > 10, "the images' rings drawn {images} frames");
    assert!(rings > images, "the fall's ring never drawn");
    assert!(shards > 10, "the mirrors' shards drawn {shards} frames");
}

/// Frames of Innis's fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/i69/shots): every 10th of its magic (act 4)
/// and its images (act 5), every 4th while a protect picture is live,
/// with the gauge at half from frame 300 (the magic's run) and the
/// magics' elements in turn.
#[test]
#[ignore]
fn innis_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i69/shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_107_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut gs: Option<piney_gs::Gs> = None;
    let (mut came, mut casts) = (None, 0);
    let mut drawn: BTreeMap<String, u32> = BTreeMap::new();
    for n in 0..9000u32 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let census = a.world().fx().census();
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].hp = c.scene.chars[k].max_hp;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i == 300 {
            let max = c.scene.chars[me].foe_state().map_or(0, |f| f.row.max_pp);
            if let Some(p) = c.scene.param_mut(me) {
                p.pp = max / 2;
            }
        }
        // Each magic's element in turn, as innis_magic forces them.
        if let Some(Class::Innis(x)) =
            c.scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut()).map(|b| &mut b.class)
            && x.move_flg == MAGIC_WORD
            && x.eny_flg == 0
            && x.cou == 3
        {
            x.eny_int = casts % 3;
            casts += 1;
        }
        for d in &census.boss_draws {
            *drawn.entry(d.clone()).or_default() += 1;
        }
        let act = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map_or(-1, |b| b.base.act_num);
        let prot = census.effects.iter().any(|e| BREAK.contains(e) || RETURN.contains(e));
        let word = innis(c, me).map_or(0, |x| x.move_flg);
        if (matches!(act, 4 | 5) && i.is_multiple_of(10)) || (prot && i.is_multiple_of(4)) {
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(&s));
            g.render(&frame);
            let (w, h) = g.target_size();
            let path = format!("{dir}/innis-{i:05}-act{act}-w{word}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        }
        if i > 6000 {
            break;
        }
    }
    println!("drawn {drawn:?}");
}

/// Gorre's Kerse ends in its `FinalPhotonFlash` at the target: the sprite
/// (particle's `EFF_x000`, chunk 0) shrinking onto it, then the burst's
/// ring (`CMP_x033`). Before, `boss_make` made no picture for it.
#[test]
fn gorres_kerse_shows_its_flash() {
    let Some(mut s) = event_218_arena() else { return };
    let seen = fight(&mut s, 12000, |_, _, _, _| {});
    let kerse = |n: &&Seen| n.act == piney_battle::boss::gorre::act::KERSE;
    let acts: std::collections::BTreeSet<i16> = seen.iter().map(|n| n.act).collect();
    let flash = seen.iter().filter(kerse).filter(|n| n.draws.iter().any(|d| d == "eff 0")).count();
    let ring = seen.iter().filter(kerse).filter(|n| n.draws.iter().any(|d| d == "CMP_x033")).count();
    println!("acts {acts:?}: the flash {flash} frames, its ring {ring}");
    assert!(acts.contains(&piney_battle::boss::gorre::act::KERSE), "no Kerse: {acts:?}");
    assert!(flash >= 11, "the flash not drawn ({flash} frames)");
    assert!(ring > 10, "the burst's ring not drawn ({ring} frames)");
}

/// Kite's hits on Gorre's brothers, every 40 frames once they are on the
/// lists: row 40 struck by the normal attack (skill 1, physical), row 41
/// by skill 2 (a spell), the kinds each resists. `each` sees the frame and
/// the shields' elements on the brothers (`FxCensus::shields`).
fn shield_fight(
    s: &mut Session,
    frames: u32,
    mut each: impl FnMut(&Session, &Frame, u32, &[(u32, piney_effect::V4, piney_effect::V4)]),
) {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut came = None;
    for n in 0..frames + 3000 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let shields = a.world().fx().census().shields;
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(_) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        let brothers = c.boss.as_ref().map_or(Vec::new(), |r| r.parts.clone());
        let hit: Vec<usize> = brothers.iter().copied().filter(|&b| i.is_multiple_of(40) && c.scene.listed(b)).collect();
        for b in hit {
            let sid = if c.scene.chars[b].id() == 40 { 1 } else { 2 };
            let kite = c.kite;
            if let Some(pb) = c.scene.chars[b].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                pb.base.queued.push((1, [10, sid, 0], kite));
            }
        }
        let on: Vec<_> = shields.into_iter().filter(|(t, ..)| brothers.contains(&(*t as usize))).collect();
        each(s, &frame, i, &on);
    }
}

/// Gorre's brothers' shields as `ccBoss05Brother::Affect` reshapes them:
/// `effResistantShield`'s scale with x halved and z 1.3 times, its offset
/// 200 higher. Row 40's against the physical, 41's against spells, both
/// shown. Before, every brother's shield was the plain one, of a spell.
#[test]
fn gorres_brothers_show_their_stretched_shields() {
    let Some(mut s) = event_218_arena() else { return };
    let (mut shown, mut shaped) = (BTreeMap::new(), 0u32);
    let mut bad = Vec::new();
    shield_fight(&mut s, 3000, |s, _, _, on| {
        let Stage::Area(a) = &s.stage else { return };
        let c = a.world().combat();
        for &(t, scale, offset) in on {
            let base = c.scene.chars[t as usize].base();
            *shown.entry(c.scene.chars[t as usize].id()).or_insert(0u32) += 1;
            // effResistantShield's for a boss: (w, w, h / 2) 0.01, offset
            // (0, -w / 2, h / 2).
            let (w, h) = (f32::from_bits(base.width), f32::from_bits(base.height));
            let half_h = f32::from_bits(piney_effect::ee::mul(0x3f00_0000, base.height));
            let want_off = f32::from_bits(piney_effect::ee::add(half_h.to_bits(), 0x4348_0000));
            let ok = f32::from_bits(scale[0]) * 2.0 == f32::from_bits(scale[1])
                && (f32::from_bits(scale[2]) - h * 0.005 * 1.3).abs() < 1e-3
                && f32::from_bits(offset[2]) == want_off
                && (f32::from_bits(offset[1]) + w * 0.5).abs() < 1e-3;
            if ok {
                shaped += 1;
            } else {
                bad.push((t, scale.map(f32::from_bits), offset.map(f32::from_bits)));
            }
        }
    });
    println!("shields by brother row {shown:?}, {shaped} frames reshaped, {} not", bad.len());
    assert!(shown.contains_key(&40) && shown.contains_key(&41), "a brother showed no shield: {shown:?}");
    assert!(bad.is_empty(), "shields not reshaped: {:?}", &bad[..bad.len().min(3)]);
}

/// Shots of the brothers' shields to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a429): every 4th frame one is up.
#[test]
#[ignore]
fn gorre_shield_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a429".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_218_arena() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    shield_fight(&mut s, 2000, |s, frame, i, on| {
        if on.is_empty() || !i.is_multiple_of(4) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        std::fs::write(format!("{dir}/gorre-shield-{i:05}.png"), piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    });
}

/// Shots of Gorre's Kerse to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a422): every 2nd frame its flash or ring is
/// drawn.
#[test]
#[ignore]
fn gorre_kerse_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a422".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_218_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut gs: Option<piney_gs::Gs> = None;
    for n in 0..15000u32 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let draws = a.world().fx().census().boss_draws;
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let shown = draws.iter().any(|d| d == "eff 0" || d == "CMP_x033");
        if !(shown && n.is_multiple_of(2)) {
            continue;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(&s));
        g.render(&frame);
        let (w, h) = g.target_size();
        std::fs::write(format!("{dir}/gorre-{n:05}.png"), piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    }
}

/// `ccSaveData` as [`event_218_arena`] starts it, written to `PINEY_SAVE_OUT`
/// for piney-eemu's `ee_run` (the whole program run to Gorre's fight).
#[test]
#[ignore]
fn event_218_save_bytes() {
    let Ok(out) = std::env::var("PINEY_SAVE_OUT") else { return };
    let iso = crate::check_image("outbreak");
    if !iso.exists() {
        return;
    }
    let blocks: u32 = std::env::var("PINEY_SAVE_BLOCKS").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let extra: u64 = std::env::var("PINEY_SAVE_DONE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let start = crate::start::build(&iso, 218).unwrap();
    let mut state = start.state;
    let f = state.save.event_flag(218);
    state.save.set_event_flag(218, f | ((1 << blocks) - 1) | extra);
    state.save.set_u8(offset::EVENT_STATUS, 2);
    std::fs::write(out, state.save.bytes()).unwrap();
}

/// One frame of Gorre's fight as [`brothers_fight`] sees it: whether each
/// brother's `ccBoss::Main` draws it (`drawSW`, not out) and whether its
/// actor is drawn; the frame's `x51` models by name; whether the frame is
/// the field's (models drawn, no stream nor menu movie playing).
struct BrotherSeen {
    due: [bool; 2],
    /// Seen: on the boss camera (2) and above `ccObj::Draw`'s 1/128.
    shown: bool,
    drawn: [bool; 2],
    models: Vec<String>,
    world: bool,
    /// The waves drawn (each at a brother, none at Gorre) and the brothers'
    /// trails alive.
    waves: Vec<[u32; 4]>,
    trails: usize,
}

/// Gorre's arena for `frames` frames from the boss's making, the party
/// kept up and the event's lines moved on; `each` sees every frame.
fn brothers_fight(s: &mut Session, frames: u32, mut each: impl FnMut(&Session, &Frame, u32)) -> Vec<BrotherSeen> {
    let x51 = piney_desktop::assets::SceneFile::read(&s.archive, "x51").unwrap();
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut came, mut seen) = (None, Vec::new());
    for n in 0..frames + 3000 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { continue };
        let boss_cam = a.world().camera().cam_id == 2;
        let c = a.world_mut().combat_mut();
        for &(_, k) in &c.members {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let Some(me) = c.boss_char() else { continue };
        let i = n - *came.get_or_insert(n);
        if i >= frames {
            break;
        }
        let Some(Class::Gorre(x)) = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class)
        else {
            continue;
        };
        let bm = x.brother_me;
        // Due while drawSW holds and ccBoss::Draw drew it (not faded out
        // by the camera's distance).
        let due = bm.map(|k| {
            let b = c.scene.chars[k].foe_state().and_then(|f| f.boss.as_ref());
            b.is_some_and(|b| b.base.draw_sw != 0 && b.base.exit == 0 && b.base.drawn)
        });
        let drawn = bm.map(|k| c.cast.actors.get(&k).is_some_and(|a| a.drawn));
        let run = c.boss.as_ref().unwrap();
        let waves = run.waves.iter().map(|w| w.1).collect();
        let trails = run.afterimages.iter().filter(|a| a.clip.starts_with("ANM_ex5")).count();
        let alpha = bm.map(|k| c.cast.actors.get(&k).map_or(0.0, |a| f32::from_bits(a.alpha)));
        let shown = boss_cam && alpha.iter().all(|&t| t > 1.0 / 128.0);
        let models: Vec<String> = frame
            .cmds
            .iter()
            .filter_map(|c| match c {
                piney_draw::Cmd::Model(m) if m.file == "x51" => x51.ccs.object_name(m.model).map(str::to_string),
                _ => None,
            })
            .collect();
        let Stage::Area(a) = &s.stage else { continue };
        let drew = frame.cmds.iter().any(|c| matches!(c, piney_draw::Cmd::Model(_)));
        let world = drew && !a.streaming() && !a.movie_playing();
        each(s, &frame, i);
        seen.push(BrotherSeen { due, shown, drawn, models, world, waves, trails });
    }
    seen
}

/// Issue #73: Gorre's brothers are drawn. Each brother's `Main` is
/// `ccBoss::Main`, whose `Draw` draws its anm (`ANM_ex51*` on
/// `CMP_trall`, `ANM_ex52*` on `CMP_trall2`, the Epitaph's `ANM_ex5x*`
/// on `CMP_trallx`) while its `drawSW` holds and the camera's fade leaves
/// it shown: from Gorre's first `Main` on, every field frame that holds
/// it draws both, each its own model.
#[test]
fn gorres_brothers_are_drawn_through_the_fight() {
    let Some(mut s) = event_218_arena() else { return };
    let seen = brothers_fight(&mut s, 6000, |_, _, _| {});
    assert_eq!(seen.len(), 6000, "the fight not reached");
    // The constructor's frame: the task's first Main is the next.
    let due: Vec<&BrotherSeen> = seen[1..].iter().filter(|f| f.due.iter().any(|&d| d)).collect();
    let missed: Vec<usize> = (1..seen.len()).filter(|&i| seen[i].due != seen[i].drawn).collect();
    let bare = due.iter().filter(|f| f.world && f.shown && f.models.is_empty()).count();
    let first = |f: &BrotherSeen, b: &str| f.models.iter().any(|m| m.contains(b));
    let both = due
        .iter()
        .filter(|f| f.due == [true; 2] && f.world && f.shown)
        .all(|f| (first(f, "ex51") && first(f, "ex52")) || f.models.iter().filter(|m| m.contains("ex5x")).count() >= 2);
    println!("{} frames, {} with a brother due, {} missed, {bare} bare", seen.len(), due.len(), missed.len());
    assert!(due.len() > 5000, "the brothers drawn too seldom: {}", due.len());
    assert!(missed.is_empty(), "frames whose drawn brothers part from drawSW: {:?}", &missed[..missed.len().min(8)]);
    assert_eq!(bare, 0, "a field frame with a brother due drew no x51 model");
    assert!(both, "a frame drew one brother's model only");
    assert!(seen[1].drawn == [true; 2], "not drawn from the first Main");
    // Each brother's wave at its own place, and their trails.
    let waves = seen.iter().filter(|f| f.waves.len() == 2 && f.waves[0] != f.waves[1]).count();
    let lone = seen.iter().filter(|f| f.waves.len() == 1).count();
    let trails = seen.iter().filter(|f| f.trails > 0).count();
    println!("{waves} frames with both waves, {lone} with one; {trails} with trails");
    assert!(waves > 0 && lone == 0, "the brothers' waves");
    assert!(trails > 0, "the brothers' trails");
}

/// Shots of Gorre's fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/a437): every 100th frame of 4000 (the
/// Talk's stream 71, which draws the brothers in close-up, about 3,400).
#[test]
#[ignore]
fn gorre_brothers_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a437".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_218_arena() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    brothers_fight(&mut s, 4000, |s, frame, i| {
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        if i.is_multiple_of(100) {
            let (w, h) = g.target_size();
            std::fs::write(format!("{dir}/gorre-{i:05}.png"), piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        }
    });
}

/// Shots of the brothers' bodies from `x51` alone to `PINEY_SHOTS`
/// (default /mnt/data/claude/scratch/a440): each clump at its neutral clip
/// (`ANM_ex51nut` on `CMP_trall`, `ANM_ex52nut` on `CMP_trall2`,
/// `ANM_ex5xnut` on `CMP_trallx`), turned 0, 90, 180 and 270 degrees, once
/// under a plain white ambient and once under the arena's own lights.
#[test]
#[ignore]
fn gorre_brothers_model_shots() {
    use piney_world::draw;
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/a440".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_218_arena() else { return };
    // The arena's lights, once the fight is up.
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut arena = None;
    for n in 0..3000u32 {
        let playing = matches!(&s.stage, Stage::Area(a) if a.vm().is_some_and(|v| v.playing().is_some()));
        pad.read(&if playing && n.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && a.world().combat().boss_char().is_some()
        {
            arena = Some(a.world().place().lights().clone());
            break;
        }
    }
    let arena = arena.expect("the fight");
    let white = piney_world::town::TownLights { ambient: glam::Vec3::splat(1.0), lights: Vec::new(), fog: None };
    let file = std::rc::Rc::new(piney_desktop::assets::SceneFile::read(&s.archive, "x51").unwrap());
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap();
    let f = |x: f32| x.to_bits();
    for (clump, clip) in [("CMP_trall", "ANM_ex51nut"), ("CMP_trall2", "ANM_ex52nut"), ("CMP_trallx", "ANM_ex5xnut")] {
        let body = piney_world::chara::body::Body::of(file.clone(), clump).unwrap();
        let play = body.play(clip).unwrap();
        for (name, lights) in [("white", &white), ("arena", &arena)] {
            for turn in [0u32, 90, 180, 270] {
                let r = (turn as f32).to_radians();
                let root = piney_world::chara::body::root([0, 0, 0, f(1.0)], [0, 0, f(r), f(1.0)]);
                let eye = [0, f(-1800.0), f(400.0), f(1.0)];
                let at = [0, 0, f(400.0), f(1.0)];
                let ws = piney_data::anim::vu_mul(
                    &piney_world::camera::VIEW_SCREEN,
                    &piney_world::camera::pos_target(eye, at),
                );
                let mut layers = piney_desktop::layers::Layers::default();
                body.draw(&mut layers, draw::CHAR_LAYER, draw::screen(&ws), &play, root, 1.0, lights, false, &[]);
                let mut frame = Frame::new();
                frame.cmds = layers.flatten();
                frame.clear = piney_draw::Rgba([0x40, 0x40, 0x40, 0x80]);
                gs.render(&frame);
                let (w, h) = gs.target_size();
                let path = format!("{dir}/{clump}-{name}-{turn:03}.png");
                std::fs::write(path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            }
        }
    }
}
