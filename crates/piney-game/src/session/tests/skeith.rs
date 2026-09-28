//! Skeith in its arena: field 1 (`EVENTAREAB0`, `se1_5`) as area 27's last
//! door leaves the game - area 27's `WORLD_MAN`, town 1 - with event 30's
//! `entry 7 0` for the entry control's set-up.

use std::collections::BTreeSet;
use std::path::PathBuf;

use piney_input::{Buttons, Raw};
use piney_world::field_world::FxCensus;

use super::*;

/// The arena with the boss's entry, and the disc's archive.
fn arena() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        eprintln!("infection.iso not present; skipped");
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut state = crate::world::new_game_state(&mut d).unwrap();
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 27, false).unwrap();
    scene.change_scene(1, 1, 1, -1, -1, -1, &mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, None, scene, Some(wm)).unwrap();
    let Stage::Area(a) = &mut s.stage else { panic!("not in the area") };
    a.world_mut().set_event_entries(Vec::new(), vec![[7, 0, 0, 0]], Vec::new());
    Some(s)
}

/// The boss's act and HP each frame of `frames`, Kite standing still
/// (and kept alive).
fn run(s: &mut Session, frames: u32, each: impl FnMut(&Session, u32, &Frame)) -> Vec<(i16, i16)> {
    run_forcing(s, frames, None, None, each)
}

/// [`run`], the boss put into act `force.1` (`ChangeAction`'s act,
/// process and count) after frame `force.0`, and Kite walking on the left
/// stick at `walk.0`, `walk.1` from frame `walk.2` to before `walk.3`.
fn run_forcing(
    s: &mut Session,
    frames: u32,
    force: Option<(u32, i16)>,
    walk: Option<(u8, u8, u32, u32)>,
    mut each: impl FnMut(&Session, u32, &Frame),
) -> Vec<(i16, i16)> {
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut seen = Vec::new();
    for i in 0..frames {
        match walk {
            Some((lx, ly, from, to)) if (from..to).contains(&i) => pad.read(&Raw { lx, ly, ..still }),
            _ => pad.read(&still),
        }
        let frame = s.step(&pad);
        s.take_events();
        each(s, i, &frame);
        let Stage::Area(a) = &mut s.stage else { continue };
        let c = a.world_mut().combat_mut();
        if let Some((at, act)) = force
            && i == at
            && let Some(me) = c.boss_char()
        {
            // ChangeAction(act, .., 1): its clip from Boss01AnmTbl.
            let clip = c.data.skeith.as_ref().and_then(|d| d.anm_tbl.get(act as usize).cloned().flatten());
            let clips = c.boss.as_ref().map(|r| r.clips.clone());
            if let Some(b) = c.scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                b.act_num = act;
                b.act_proccess = 0;
                b.act_count = 0;
                if let (Some(clip), Some(clips)) = (clip, clips) {
                    let (frames, looping) = clips.get(&clip).copied().unwrap_or((0, false));
                    b.anm.clip = Some(clip);
                    b.anm.time = 0;
                    b.anm.frames = frames;
                    b.anm.looping = looping;
                }
            }
        }
        if let Some(k) = c.kite {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        if let Some(me) = c.boss_char()
            && let Some(b) = c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref())
        {
            seen.push((b.act_num, c.scene.chars[me].hp));
        }
    }
    seen
}

/// The boss is made at the set-up in the arena, 500 below Kite, and runs
/// its patterns: more than one act over 900 frames, the event host told
/// it is there.
///
/// The boss camera has camera 2: its eye behind Kite, 200 up, and it looks
/// the way from Kite to the boss but for the frames it turns to catch up
/// (or a hit throws him, or a quake shakes it).
#[test]
fn skeith_fights_in_its_arena() {
    let Some(mut s) = arena() else { return };
    let (mut errors, mut heights) = (Vec::new(), Vec::new());
    let seen = run(&mut s, 900, |s, _, _| {
        let Stage::Area(a) = &s.stage else { return };
        let (cam, c) = (a.world().camera(), a.world().combat());
        let (Some(k), Some(me)) = (c.kite, c.boss_char()) else { return };
        assert_eq!(cam.cam_id, piney_world::camera::id::BATTLE);
        let f = f32::from_bits;
        let (kp, bp, b) = (c.scene.chars[k].pos, c.scene.chars[me].pos, &cam.bcam);
        let look = (f(b.view[1]) - f(b.pos[1])).atan2(f(b.view[0]) - f(b.pos[0]));
        let to_boss = (f(bp[1]) - f(kp[1])).atan2(f(bp[0]) - f(kp[0]));
        let d = (look - to_boss + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        errors.push(d.abs());
        heights.push(f(b.pos[2]) - f(kp[2]));
    });
    assert!(!seen.is_empty(), "no boss");
    let acts: BTreeSet<i16> = seen.iter().map(|x| x.0).collect();
    assert!(acts.len() > 1, "the boss stood in act {acts:?}");
    let Stage::Area(a) = &s.stage else { panic!() };
    assert_eq!(a.world().boss_task(), Some(0));
    let near = errors.iter().filter(|e| **e < 0.35).count();
    println!("acts {acts:?}, HP {:?}, facing the boss {near} of {} frames", seen.last(), errors.len());
    assert!(near * 10 >= errors.len() * 8, "the camera looked away: {near} of {}", errors.len());
    // 200 over Kite as he was in the boss's task: other than that a hit
    // threw him or a quake shook it.
    let level = heights.iter().filter(|h| (**h - 200.0).abs() < 1.0).count();
    assert!(level * 10 >= heights.len() * 8, "eye heights {heights:?}");
}

/// Every 30th frame of the fight to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/skeith).
#[test]
#[ignore]
fn skeith_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/skeith".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = arena() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    run(&mut s, 1800, |s, i, frame| {
        // Every 30th frame, and every 4th while the boss dashes, chases or
        // swings the wave.
        let busy = match &s.stage {
            Stage::Area(a) => {
                let c = a.world().combat();
                c.boss_char()
                    .and_then(|me| c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()))
                    .is_some_and(|b| matches!(b.act_num, 4 | 6 | 7 | 18))
            }
            _ => false,
        };
        if i % 30 != 29 && !(busy && i % 4 == 0) {
            return;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (w, h) = g.target_size();
        let path = format!("{dir}/skeith-{i:04}.png");
        std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        println!("{path}");
    });
}

/// Shots of one of the boss's acts: at frame `$PINEY_AT` (default 80) the
/// boss is put into act `$PINEY_ACT` (default 6, the magic; 3 the cross, 4
/// the wave) as
/// `ChangeAction` would, and every `$PINEY_EVERY`th frame (default 6) of
/// the next `$PINEY_FRAMES` (default 480) goes to `$PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/skeith-act), named by frame. `$PINEY_WALK`
/// (`LX,LY,FROM,TO`) holds the left stick there from frame FROM to before
/// TO (Kite walks off toward the fireflies, which move and show only
/// within 1000 of him).
#[test]
#[ignore]
fn skeith_act_shots() {
    let env = |k: &str, d: u32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/skeith-act".into());
    std::fs::create_dir_all(&dir).unwrap();
    let (act, every, frames) = (env("PINEY_ACT", 6) as i16, env("PINEY_EVERY", 6), env("PINEY_FRAMES", 480));
    let at = env("PINEY_AT", 80);
    let walk = std::env::var("PINEY_WALK").ok().and_then(|v| {
        let w: Vec<u32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (w.len() == 4).then(|| (w[0] as u8, w[1] as u8, w[2], w[3]))
    });
    let Some(mut s) = arena() else { return };
    let mut gs: Option<piney_gs::Gs> = None;
    run_forcing(&mut s, at + frames, Some((at - 1, act)), walk, |s, i, frame| {
        if i >= at && (i - at) % every == 0 {
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            let (w, h) = g.target_size();
            let path = format!("{dir}/act{act}-{i:04}.png");
            std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
        }
    });
}

/// Event 30's task with the story brought to the arena, the session in
/// field 1 (town 1, area 27's `WORLD_MAN`): blocks 0-19 played and
/// `eventStatus[0]` 4, as the board, the mail, Dun Loireag and the
/// dungeon leave them.
fn event_30_arena() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    if !iso.exists() {
        return None;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 30).unwrap();
    let mut state = start.state;
    // The event as the player brings it to the arena: the board, the mail,
    // Dun Loireag, the Chaos Gate and the dungeon (blocks 0-19) played.
    let f = state.save.event_flag(30);
    state.save.set_event_flag(30, f | ((1 << 20) - 1));
    // Block 11's status_set 0 4 (Dun Loireag, BlackRose along).
    state.save.set_u8(offset::EVENT_STATUS, 4);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 27, false).unwrap();
    scene.change_scene(1, 1, 1, -1, -1, -1, &mut state.save);
    Some(Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap())
}

/// Event 30 with the story brought to it, in the arena: its block 20 makes
/// the boss (`entry 7 0`, `battle_ready`, `save_party`) at the set-up.
/// Kite is kept alive; the drain's two affects (13, then 21) and then
/// heavy hits are put on the boss as Kite's frame would, and the fight
/// runs to its end: the Epitaph's patterns, death, the exit, and block
/// 22's `if absent 7` - `area_ban`, `gate_unmark`, `mode 3`. The death
/// clears the party's conditions (`ccClearSpcCondition`): Kite, poisoned
/// and with a stat raised in his record before it, has neither after, and
/// `noDeathFlag`. Then the desktop's event 31 (ENDING): stream 15, the
/// closing lines, the staff roll.
#[test]
fn event_30_ends_with_skeith() {
    let Some(mut s) = event_30_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut acts = BTreeSet::new();
    let (mut made, mut drained, mut exited, mut left) = (false, false, false, false);
    let mut calls = Vec::new();
    let mut cams = BTreeSet::new();
    let mut since = 0u32;
    let mut cleared = None;
    for _ in 0..8000u32 {
        pad.read(&still);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else {
            left = true;
            break;
        };
        // Before the fight's end: Kite poisoned for long, and his record's
        // physical attack raised for as long (temp, with its timer).
        if since == 300 {
            let c = a.world_mut().combat_mut();
            let k = c.kite.unwrap();
            c.scene.chars[k].cond[piney_battle::param::cond::POISON] = 30000;
            let mut p = piney_battle::param::SpcParam::from_save(&a.world().state().save, 0);
            p.temp[piney_battle::param::elm::P_ATK] = 5;
            p.time[piney_battle::param::elm::P_ATK] = 30000;
            p.store(a.save_mut(), 0);
        }
        if cleared.is_none()
            && let Some(me) = a.world().combat().boss.as_ref().map(|b| b.me)
            && a.world().combat().scene.chars[me].hp == 0
        {
            let c = a.world().combat();
            let k = &c.scene.chars[c.kite.unwrap()];
            let p = piney_battle::param::SpcParam::from_save(&a.world().state().save, 0);
            cleared = Some((k.cond[piney_battle::param::cond::POISON], p.temp, k.no_death));
        }
        calls = a.calls().iter().map(|(_, c)| c.clone()).collect();
        let cam_id = a.world().camera().cam_id;
        let c = a.world_mut().combat_mut();
        if let Some(k) = c.kite {
            let ch = &mut c.scene.chars[k];
            // A new game's Kite would fall to one blow: a stout one.
            ch.max_hp = 9999;
            ch.hp = 9999;
        }
        let Some(me) = c.boss.as_ref().map(|b| b.me) else { continue };
        made = true;
        since += 1;
        let kite = c.kite;
        let Some(b) = c.scene.chars[me].foe_state_mut().and_then(|f| f.boss.as_mut()) else { continue };
        acts.insert(b.act_num);
        exited |= b.exit != 0;
        // The boss camera (2) through the fight, the dead camera (3) from
        // BeginDeadEffect.
        cams.insert((b.act_num == 14 || b.exit != 0, cam_id));
        // The menus open only with the player free (LockPlayer forbids
        // them): the drain in the boss's neutral stance.
        if since >= 400 && !drained && b.act_num == 0 && b.lock_player == 0 {
            // The drain: the target menu's hold (13), then its 21.
            b.queued.push((13, [0; 3], kite));
            b.queued.push((21, [0; 3], kite));
            drained = true;
        }
        if drained && since.is_multiple_of(20) && b.exit == 0 && b.lock_player == 0 && b.epitaph != 0 {
            b.queued.push((1, [800, 0, 0], kite));
        }
    }
    calls.dedup();
    println!("acts {acts:?}\ncalls {calls:?}\nleft the area: {left}");
    assert!(made, "block 20 made no boss");
    assert!(drained);
    assert!(acts.contains(&12), "no Epitaph: {acts:?}");
    assert!(acts.contains(&14), "never died: {acts:?}");
    assert!(exited, "never exited");
    let (poison, temp, no_death) = cleared.expect("the boss's HP never reached 0");
    assert_eq!((poison, temp, no_death), (0, [0; 16], true), "the party's conditions after the death");
    use piney_world::camera::id;
    assert!(cams.contains(&(false, id::BATTLE)) && cams.contains(&(true, id::EVENT)), "cameras {cams:?}");
    assert!(!cams.contains(&(false, id::FIELD)), "the field camera in the fight: {cams:?}");
    // Block 22: area_ban and gate_unmark on the save, then mode 3 (the
    // desktop) takes the session out of the area.
    assert!(left, "the event did not end the area");
    println!("now: {}", Mode::title(&s));
    assert!(!matches!(s.stage, Stage::Area(_)));
    // The desktop: event 31 (ENDING) - stream 15, the closing lines, the
    // staff roll.
    let mut titles = Vec::new();
    for i in 0..30000u32 {
        let b = if i % 30 == 0 { Buttons::CROSS } else { Buttons::NONE };
        pad.read(&Raw { buttons: b, ..still });
        s.step(&pad);
        s.take_events();
        let t = Mode::title(&s);
        let key: String = t
            .split(" - ")
            .filter(|p| !p.starts_with("frame"))
            .map(|p| p.split(" frame").next().unwrap_or(p))
            .collect::<Vec<_>>()
            .join(" - ");
        if titles.last().is_none_or(|(_, k): &(u32, String)| *k != key) {
            println!("{i}: {t}");
            titles.push((i, key));
        }
    }
    assert!(titles.iter().any(|(_, k)| k.contains("stream 15")), "no ending movie: {titles:?}");
    assert!(titles.iter().any(|(_, k)| k.contains("staff roll")), "no staff roll: {titles:?}");
}

/// The stick pushed toward heading `h` (radians, the field's) with the
/// camera turned `cam_z`: the nearest of 64 directions.
fn stick_toward(cam_z: f32, h: f32) -> Raw {
    use std::f32::consts::PI;
    let want = (cam_z - PI - h).rem_euclid(2.0 * PI);
    let mut best = (f32::MAX, Raw::default());
    for k in 0..64 {
        let a = k as f32 * PI / 32.0;
        let (lx, ly) = ((128.0 + 127.0 * a.cos()) as u8, (128.0 + 127.0 * a.sin()) as u8);
        let raw = Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() };
        let mut p = Pad::default();
        p.read(&raw);
        let d = (p.dirc_l.rem_euclid(2.0 * PI) - want).abs();
        let d = d.min(2.0 * PI - d);
        if d < best.0 {
            best = (d, raw);
        }
    }
    best.1
}

/// Skeith drained through the menus, as a player does it: in event 30's
/// arena, Kite (with the bracelet and Data Drain, stout) walks up to the
/// boss while it is free and its protect is broken, opens the menu, and
/// PERSONAL, Skills, the Data Drain page, Skeith: menu 66 runs, the boss
/// takes the drain's 13 and 21, and its Epitaph (4500 HP) begins.
#[test]
fn skeith_drained_through_the_menus() {
    let Some(mut s) = event_30_arena() else { return };
    {
        let Stage::Area(a) = &mut s.stage else { panic!("not in the area") };
        let save = &mut a.world_mut().state_mut().save;
        let free = (0..20).find(|&k| save.i16(piney_fieldui::items::SKILL_LIST + 2 * k) < 0).unwrap();
        save.set_i16(piney_fieldui::items::SKILL_LIST + 2 * free, 2);
        save.set_u8(offset::PLCOL, 1);
        save.set_u8(piney_fieldui::menus::drain::DRAIN_DEMO, 0);
    }
    let mut pad = Pad::default();
    let (mut drain_menu, mut epitaph, mut menus) = (false, false, BTreeSet::new());
    for i in 0..9000u64 {
        let raw = {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area: {}", Mode::title(&s)) };
            {
                let c = a.world_mut().combat_mut();
                if let Some(k) = c.kite {
                    let ch = &mut c.scene.chars[k];
                    ch.max_hp = 9999;
                    ch.hp = 9999;
                }
                // Enough hits would have broken its protect.
                if let Some(me) = c.boss_char()
                    && let Some(f) = c.scene.chars[me].foe_state_mut()
                    && f.boss.as_ref().is_some_and(|b| b.epitaph == 0)
                {
                    f.pp_count = f.pp_count.max(5);
                }
            }
            let w = a.world();
            let c = w.combat();
            let ui = a.ui();
            let m = &ui.ctrl;
            menus.insert(m.menu);
            drain_menu |= m.menu == 66;
            let boss = c
                .boss_char()
                .and_then(|me| c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| (me, b)));
            if let Some((_, b)) = boss {
                epitaph |= b.epitaph != 0;
            }
            if epitaph {
                break;
            }
            let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let press = |b: Buttons| if i.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
            let go_to = |row: usize| match m.list().select.max(0) as usize {
                s if s == row => Buttons::CROSS,
                s if s < row => Buttons::DOWN,
                _ => Buttons::UP,
            };
            match (ui.menu_type(), boss) {
                (-1, Some((me, b))) if b.lock_player == 0 && m.forbid == 0 => {
                    let k = c.kite.unwrap();
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let q = c.scene.chars[me].pos.map(f32::from_bits);
                    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                    if dx * dx + dy * dy < 600.0 * 600.0 {
                        press(Buttons::TRIANGLE)
                    } else {
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        stick_toward(cam_z, dx.atan2(-dy))
                    }
                }
                (-1, _) => still,
                (0..=2, _) => press(go_to(m.list().items.iter().position(|&x| x == 4).unwrap_or(0))),
                (4, _) if m.list().page < 5 => press(Buttons::RIGHT),
                (4, _) => press(go_to(0)),
                _ => press(Buttons::CROSS),
            }
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    println!("menus {menus:?}");
    assert!(drain_menu, "menu 66 never ran");
    assert!(epitaph, "the drain did not bring the Epitaph");
}

/// The boss's body is on the collision list: Kite, walking at it, stops
/// against it (their two radii apart) instead of passing through.
#[test]
fn kite_bumps_into_skeith() {
    let Some(mut s) = arena() else { return };
    let mut pad = Pad::default();
    let mut nearest = f32::MAX;
    let mut radii = 0.0f32;
    for _ in 0..600u32 {
        let raw = {
            let Stage::Area(a) = &mut s.stage else { panic!() };
            let c = a.world_mut().combat_mut();
            let (Some(k), Some(me)) = (c.kite, c.boss_char()) else {
                pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
                s.step(&pad);
                s.take_events();
                continue;
            };
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
            let p = c.scene.chars[k].pos.map(f32::from_bits);
            let q = c.scene.chars[me].pos.map(f32::from_bits);
            let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
            nearest = nearest.min((dx * dx + dy * dy).sqrt());
            radii = f32::from_bits(c.scene.chars[me].base().width) + f32::from_bits(c.scene.chars[k].base().width);
            let w = a.world();
            stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
        };
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
    }
    println!("nearest {nearest}, radii {radii}");
    assert!(nearest > radii * 0.8, "Kite came within {nearest} of the boss (radii {radii})");
}

/// Skeith's Data Drain on a member: with its gauge past half and a hit,
/// the Super table's `15` (act 5) locks the player and at its count 45
/// opens menu 74 (`StreamMenu`) on stream 20 with the member's slot; the
/// stream plays with the member drawn into it, the menu closes, and the
/// boss goes on (its affect 13 on the member, the next pattern).
#[test]
fn skeith_drains_a_member() {
    let Some(mut s) = event_30_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let (mut menu74, mut after, mut acts) = (false, false, BTreeSet::new());
    let mut hit = false;
    for _ in 0..12000u32 {
        pad.read(&still);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
        let open = a.ui().ctrl.menu == 74;
        menu74 |= open;
        let c = a.world_mut().combat_mut();
        if let Some(k) = c.kite {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let kite = c.kite;
        let Some(me) = c.boss_char() else { continue };
        let Some(f) = c.scene.chars[me].foe_state_mut() else { continue };
        let Some(b) = f.boss.as_mut() else { continue };
        acts.insert(b.act_num);
        if !hit && b.act_num == 0 && b.lock_player == 0 {
            // The gauge past half, then a hit: the Super table.
            f.pp = f.row.max_pp;
            if let Some(b) = f.boss.as_mut() {
                b.queued.push((1, [10, 0, 0], kite));
            }
            hit = true;
            continue;
        }
        if menu74 && !open && b.act_num != 5 {
            after = true;
            break;
        }
    }
    let Stage::Area(a) = &s.stage else { panic!() };
    let calls: Vec<&str> = a.calls().iter().map(|(_, c)| c.as_str()).collect();
    println!(
        "acts {acts:?} calls {:?}",
        calls.iter().filter(|c| c.contains("movie") || c.contains("party")).collect::<Vec<_>>()
    );
    assert!(acts.contains(&5), "no drain act: {acts:?}");
    assert!(menu74, "menu 74 never opened");
    assert!(calls.iter().any(|c| c.starts_with("drain_movie 20")), "stream 20 not played");
    assert!(after, "the fight did not go on after the member drain");
}

/// Frames of the member drain's stream (menu 74) to `PINEY_SHOTS`
/// (default /mnt/data/claude/scratch/skeith): every 20th while it plays.
#[test]
#[ignore]
fn member_drain_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/skeith".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut s) = event_30_arena() else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut gs: Option<piney_gs::Gs> = None;
    let (mut hit, mut n) = (false, 0u32);
    for _ in 0..12000u32 {
        pad.read(&still);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { break };
        if a.ui().ctrl.menu == 74 {
            n += 1;
            if n % 20 == 1 {
                let g =
                    gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
                g.set_overlay(Mode::archive(&s));
                g.render(&frame);
                let (w, h) = g.target_size();
                let path = format!("{dir}/member-{n:04}.png");
                std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
                println!("{path}");
            }
            continue;
        } else if n > 0 {
            break;
        }
        let c = a.world_mut().combat_mut();
        if let Some(k) = c.kite {
            c.scene.chars[k].max_hp = 9999;
            c.scene.chars[k].hp = 9999;
        }
        let kite = c.kite;
        let Some(me) = c.boss_char() else { continue };
        let Some(f) = c.scene.chars[me].foe_state_mut() else { continue };
        if !hit && f.boss.as_ref().is_some_and(|b| b.act_num == 0 && b.lock_player == 0) {
            f.pp = f.row.max_pp;
            if let Some(b) = f.boss.as_mut() {
                b.queued.push((1, [10, 0, 0], kite));
            }
            hit = true;
        }
    }
    assert!(n > 0, "menu 74 never opened");
}

/// One of Data Drain's side effects, forced on a drain of Skeith: the save's
/// infection before step 10 (its row of `dataDrainErosionTbl`), the
/// effect's two draws (`rand() & 15`, the resistance roll), and Kite's exp
/// and level then (a level lost when the exp does not cover the loss).
#[derive(Clone, Copy)]
struct Side {
    erosion: i16,
    draws: [i32; 2],
    kite_exp: Option<(i16, i16)>,
}

/// Skeith drained through the menus as [`skeith_drained_through_the_menus`]
/// does it, the menu's roll made to go to the side effect (its `rand` 0)
/// and the side effect forced (`side`). The drain runs to its growth step:
/// `each` sees every frame of steps 10 to 12 with the step and count, the
/// effects' census and the frame. The last `drain_side_effect` id.
fn drain_with_side(side: Side, mut each: impl FnMut(&mut Session, i16, i16, &FxCensus, &Frame)) -> Option<i32> {
    let mut s = event_30_arena()?;
    {
        let Stage::Area(a) = &mut s.stage else { panic!("not in the area") };
        let save = &mut a.world_mut().state_mut().save;
        let free = (0..20).find(|&k| save.i16(piney_fieldui::items::SKILL_LIST + 2 * k) < 0).unwrap();
        save.set_i16(piney_fieldui::items::SKILL_LIST + 2 * free, 2);
        save.set_u8(offset::PLCOL, 1);
        save.set_u8(piney_fieldui::menus::drain::DRAIN_DEMO, 0);
    }
    let mut pad = Pad::default();
    let (mut in_drain, mut forced, mut id) = (false, false, None);
    for i in 0..12000u64 {
        let raw = {
            let Stage::Area(a) = &mut s.stage else { panic!("left the area: {}", Mode::title(&s)) };
            let menu = a.ui().ctrl.menu;
            if menu == 66 && !in_drain {
                in_drain = true;
                a.ui_mut().ctrl.rng = Box::new(|| 0);
            }
            if in_drain && menu != 66 {
                break;
            }
            if in_drain && a.ui().ctrl.proccess == 10 && !forced {
                forced = true;
                let w = a.world_mut();
                w.state_mut().save.set_i16(piney_battle::exp::SAVE_EROSION, side.erosion);
                // Kite's experience and level into his record in the save,
                // which the battle's frame reads its copy from.
                if let Some((exp, level)) = side.kite_exp {
                    let mut p = piney_battle::param::SpcParam::from_save(&w.state().save, 0);
                    p.base.exp = exp;
                    p.base.level = level;
                    p.store(&mut w.state_mut().save, 0);
                }
                let c = w.combat_mut();
                c.force_side_draws = Some(side.draws);
                if let (Some((exp, level)), Some(k)) = (side.kite_exp, c.kite)
                    && let Some(p) = c.scene.chars[k].spc_mut()
                {
                    p.base.exp = exp;
                    p.base.level = level;
                }
            }
            {
                let c = a.world_mut().combat_mut();
                if !in_drain && let Some(k) = c.kite {
                    let ch = &mut c.scene.chars[k];
                    ch.max_hp = 9999;
                    ch.hp = 9999;
                }
                if let Some(me) = c.boss_char()
                    && let Some(f) = c.scene.chars[me].foe_state_mut()
                    && f.boss.as_ref().is_some_and(|b| b.epitaph == 0)
                {
                    f.pp_count = f.pp_count.max(5);
                }
            }
            let w = a.world();
            let c = w.combat();
            let ui = a.ui();
            let m = &ui.ctrl;
            let boss = c
                .boss_char()
                .and_then(|me| c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| (me, b)));
            let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let press = |b: Buttons| if i.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
            let go_to = |row: usize| match m.list().select.max(0) as usize {
                s if s == row => Buttons::CROSS,
                s if s < row => Buttons::DOWN,
                _ => Buttons::UP,
            };
            match (ui.menu_type(), boss) {
                _ if in_drain => press(Buttons::CROSS),
                (-1, Some((me, b))) if b.lock_player == 0 && m.forbid == 0 => {
                    let k = c.kite.unwrap();
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let q = c.scene.chars[me].pos.map(f32::from_bits);
                    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                    if dx * dx + dy * dy < 600.0 * 600.0 {
                        press(Buttons::TRIANGLE)
                    } else {
                        let cam_z = f32::from_bits(w.camera().rot()[2]);
                        stick_toward(cam_z, dx.atan2(-dy))
                    }
                }
                (-1, _) => still,
                (0..=2, _) => press(go_to(m.list().items.iter().position(|&x| x == 4).unwrap_or(0))),
                (4, _) if m.list().page < 5 => press(Buttons::RIGHT),
                (4, _) => press(go_to(0)),
                _ => press(Buttons::CROSS),
            }
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &mut s.stage else { break };
        if let Some((_, l)) = a.calls().iter().rev().find(|c| c.1.starts_with("drain_side_effect ")) {
            id = l.trim_start_matches("drain_side_effect ").parse().ok();
        }
        let (menu, step, count) = (a.ui().ctrl.menu, a.ui().ctrl.proccess, a.ui().ctrl.wait_count);
        if menu == 66 && (10..=12).contains(&step) {
            let census = a.world_mut().fx_mut().census();
            each(&mut s, step, count, &census, &frame);
        }
    }
    assert!(in_drain, "menu 66 never ran");
    id
}

/// `flyFont`'s codes for MISS, EXP and LEVEL DOWN (`ccEntryFlyFontNewMiss`,
/// `...Exp`, `...LevelDown`).
const FF_MISS: &[u8] = b"+,--";
const FF_EXP: &[u8] = b"./0";
const FF_LEVEL_DOWN: &[u8] = b"1.2.1 3456";

/// Data Drain's side effects on screen: each kind forced on a drain of
/// Skeith, what DataDrainMenu's step 10 starts comes up in the effects
/// during step 11 - effSkillStartEffect's ring (-13) on whoever it takes,
/// effHeal's (-19) for the heal, the MISS over a resisted condition, the
/// exp lost and effAfterDrain's wave (12), and at count 30 LEVEL DOWN.
#[test]
fn drain_side_effects_show() {
    struct Case {
        id: i32,
        side: Side,
        effects: &'static [i16],
        words: &'static [&'static [u8]],
    }
    let cases = [
        // 0: everyone healed.
        Case { id: 0, side: Side { erosion: 0, draws: [0, 1000], kite_exp: None }, effects: &[-19], words: &[] },
        // 1: Kite's pAtk down.
        Case { id: 1, side: Side { erosion: 0, draws: [2, 1000], kite_exp: None }, effects: &[-13], words: &[] },
        // 9: Kite slowed.
        Case { id: 9, side: Side { erosion: 0, draws: [10, 1000], kite_exp: None }, effects: &[-13], words: &[] },
        // 16: every member slowed.
        Case { id: 16, side: Side { erosion: 25, draws: [12, 1000], kite_exp: None }, effects: &[-13], words: &[] },
        // 21: every member's HP halved.
        Case { id: 21, side: Side { erosion: 25, draws: [13, 1000], kite_exp: None }, effects: &[-13], words: &[] },
        // 25: 600 exp lost from 100 at level 5: a level down.
        Case {
            id: 25,
            side: Side { erosion: 50, draws: [15, 1000], kite_exp: Some((100, 5)) },
            effects: &[12],
            words: &[FF_EXP, FF_LEVEL_DOWN],
        },
        // 28: everyone left with 1 HP and 1 SP.
        Case { id: 28, side: Side { erosion: 75, draws: [15, 1000], kite_exp: None }, effects: &[-13], words: &[] },
    ];
    if event_30_arena().is_none() {
        return;
    }
    for case in &cases {
        let (mut effects, mut words) = (BTreeSet::new(), Vec::new());
        let id = drain_with_side(case.side, |_, step, _, census, _| {
            if step == 11 {
                effects.extend(census.effects.iter().copied());
                words.extend(census.fly_fonts.iter().cloned());
            }
        });
        println!("side effect {id:?}: effects {effects:?}, words {:?}", words.len());
        assert_eq!(id, Some(case.id), "the forced side effect");
        for e in case.effects {
            assert!(effects.contains(e), "side effect {}: effect {e} never live ({effects:?})", case.id);
        }
        for w in case.words {
            assert!(
                words.iter().any(|t| t.ends_with(w)),
                "side effect {}: {:?} never shown",
                case.id,
                String::from_utf8_lossy(w)
            );
        }
    }
}

/// A condition resisted: effect 9 (Kite slowed) with the roll at 0, below
/// Kite's body tolerance - MISS over Kite as the ring starts.
#[test]
fn drain_side_effect_resisted_shows_miss() {
    let side = Side { erosion: 0, draws: [10, 0], kite_exp: None };
    let mut words = Vec::new();
    let mut tolerance = None;
    let Some(id) = drain_with_side(side, |s, step, _, census, _| {
        if step == 11 {
            words.extend(census.fly_fonts.iter().cloned());
        }
        if tolerance.is_none()
            && let Stage::Area(a) = &s.stage
            && let Some(k) = a.world().combat().kite
        {
            tolerance = Some(a.world().combat().scene.chars[k].real()[piney_battle::param::elm::BODY]);
        }
    }) else {
        return;
    };
    assert_eq!(id, 9);
    // Kite's body tolerance in the arena is 40.
    assert!(tolerance.is_some_and(|t| t > 0), "Kite resists nothing: {tolerance:?}");
    assert!(words.iter().any(|t| t.ends_with(FF_MISS)), "no MISS over Kite (tolerance {tolerance:?})");
}

/// Frames of Data Drain's side effects to `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/drain_side): for each kind, step 11's frames 2, 8,
/// 16 and 24 and the frame after count 30 (the level lost), as
/// `side-ID-FRAME.png` (FRAME counted from step 11's first).
#[test]
#[ignore]
fn drain_side_effect_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/drain_side".into());
    std::fs::create_dir_all(&dir).unwrap();
    let sides = [
        Side { erosion: 0, draws: [0, 1000], kite_exp: None },
        Side { erosion: 0, draws: [2, 1000], kite_exp: None },
        Side { erosion: 0, draws: [10, 0], kite_exp: None },
        Side { erosion: 25, draws: [12, 1000], kite_exp: None },
        Side { erosion: 25, draws: [13, 1000], kite_exp: None },
        Side { erosion: 50, draws: [15, 1000], kite_exp: Some((100, 5)) },
        Side { erosion: 75, draws: [15, 1000], kite_exp: None },
        Side { erosion: 100, draws: [14, 1000], kite_exp: None },
    ];
    for side in sides {
        // A renderer per drain, fed every frame from step 10 on: the drain's
        // noise reads the last frame drawn.
        let mut gs: Option<piney_gs::Gs> = None;
        let (mut shots, mut n) = (Vec::new(), 0);
        let id = drain_with_side(side, |s, step, count, _, frame| {
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            if step != 11 {
                return;
            }
            n += 1;
            if [2, 8, 16, 24].contains(&n) || count == 31 {
                let (w, h) = g.target_size();
                shots.push((n, piney_gs::png::encode(w, h, &g.read_back())));
            }
        });
        let Some(id) = id else { return };
        for (count, png) in shots {
            let path = format!("{dir}/side-{id:02}-{count:02}.png");
            std::fs::write(&path, png).unwrap();
            println!("{path}");
        }
    }
}
