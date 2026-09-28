//! The chat balloons (`ccChatMsg`, `piney_fieldui::chat_msg`) in play:
//! Mac Anu's walking players (`ccRtownPC`) saying their lines over their
//! heads.

use super::*;

/// Mac Anu from event 11's start, standing, until `each` has seen enough;
/// every frame `each` gets the session and the frame.
fn mac_anu(frames: u32, mut each: impl FnMut(&Session, u32, &Frame) -> bool) {
    let Some(mut s) = story_session(11) else { return };
    let mut pad = Pad::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    for f in 0..frames {
        pad.read(&still);
        let frame = s.step(&pad);
        s.take_events();
        if each(&s, f, &frame) {
            return;
        }
    }
}

/// A walking player opens a balloon within a minute: its text one of the
/// players' lines, over one of Mac Anu's walking PCs (`handle` kind 2).
#[test]
fn mac_anu_players_chat() {
    let mut seen = None;
    mac_anu(3600, |s, f, _| {
        let Stage::World(w) = &s.stage else { return false };
        let c = &w.ui().ctrl.chat;
        if let Some(slot) = c.slots.iter().find(|s| s.cf > 0 && s.who >> 24 == 2) {
            seen = Some((f, slot.who, String::from_utf8_lossy(&slot.text).into_owned()));
            return true;
        }
        false
    });
    if story_session(11).is_none() {
        return;
    }
    let (f, who, text) = seen.expect("no walking player spoke in a minute");
    println!("frame {f}: PC {:x}: {text}", who & 0xff_ffff);
    assert!(!text.is_empty());
}

/// Shots of Mac Anu while balloons are up, into `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/chat): `cargo test --release -p piney-game
/// mac_anu_chat_shots -- --ignored --nocapture`.
#[test]
#[ignore]
fn mac_anu_chat_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/chat".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs: Option<piney_gs::Gs> = None;
    let mut taken = 0;
    mac_anu(3600, |s, f, frame| {
        let Stage::World(w) = &s.stage else { return false };
        if w.ui().chat_speakers().is_empty() || f % 20 != 0 {
            return false;
        }
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (wd, ht) = g.target_size();
        let path = format!("{dir}/chat-{f:04}.png");
        std::fs::write(&path, piney_gs::png::encode(wd, ht, &g.read_back())).unwrap();
        println!("{path}");
        taken += 1;
        taken >= 6
    });
}

/// A party member speaks in a fight: after event 3's lessons, at its east
/// portal with the goblins out and Kite attacking them, a member's lines
/// (`ccAI::ChatMessage*`, opened by `ChatMessageSender` through
/// `ccChatMsg::OpenChat`) come up in balloons over him (`handle` kind 1,
/// his id) while the battle is on.
#[test]
fn a_member_speaks_in_a_fight() {
    let Some((mut s, mut f)) = story_to_field(0) else { return };
    let mut log = Vec::new();
    story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
    let mut said: Vec<(u32, String, bool)> = Vec::new();
    walk_to_east_portal(&mut s, 1500, true, |s| {
        let Stage::Area(a) = &s.stage else { return };
        let fight = a.world().combat().battle.in_battle != 0;
        for slot in a.ui().ctrl.chat.slots.iter().filter(|x| x.cf > 0 && x.who >> 24 == 1) {
            let line = (slot.who & 0xff_ffff, String::from_utf8_lossy(&slot.text).into_owned(), fight);
            if !said.contains(&line) {
                said.push(line);
            }
        }
    });
    for (who, text, fight) in &said {
        println!("member {who} (in battle {fight}): {text}");
    }
    let (who, text, _) = said.iter().find(|l| l.2).expect("no member spoke in the fight");
    assert!(*who != 0 && !text.is_empty());
}

/// A shot of [`a_member_speaks_in_a_fight`]'s first balloon in the fight,
/// 20 frames after it opens, into `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/chat): `cargo test --release -p piney-game
/// member_chat_shot -- --ignored --nocapture`.
#[test]
#[ignore]
fn member_chat_shot() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/chat".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some((mut s, mut f)) = story_to_field(0) else { return };
    let mut log = Vec::new();
    story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
    let mut gs: Option<piney_gs::Gs> = None;
    let mut n = 0u64;
    let mut open = None;
    walk_to_east_portal_frames(&mut s, 1500, true, |s, frame| {
        n += 1;
        let Stage::Area(a) = &s.stage else { return };
        if open.is_none()
            && a.world().combat().battle.in_battle != 0
            && a.ui().ctrl.chat.slots.iter().any(|x| x.cf > 0 && x.who >> 24 == 1 && x.who & 0xff_ffff != 0)
        {
            open = Some(n);
        }
        if open.is_some_and(|o| n == o + 20) {
            let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
            g.set_overlay(Mode::archive(s));
            g.render(frame);
            let (wd, ht) = g.target_size();
            let path = format!("{dir}/member-{n:04}.png");
            std::fs::write(&path, piney_gs::png::encode(wd, ht, &g.read_back())).unwrap();
            println!("{path}");
        }
    });
    assert!(open.is_some(), "no member spoke in the fight");
}

/// What [`chat_member_heal`] saw: the menus in the order they opened,
/// the battle, the order, the member's cast and his words.
#[derive(Default)]
struct HealRun {
    menus: Vec<i32>,
    fought: bool,
    ordered: bool,
    cast: bool,
    words: bool,
}

/// After event 3's lessons, at the east portal with the goblins out, Kite
/// opens CHAT (square), the Members page, the first member (71),
/// Designate Skill (72), the recovery page's Repth (given him, with the SP
/// for it) and Kite as its target (73); then until the member casts it.
/// `each` sees every frame drawn.
fn chat_member_heal(mut each: impl FnMut(&Session, &Frame)) -> Option<HealRun> {
    const REPTH: i16 = 150;
    let (mut s, mut f) = story_to_field(0)?;
    let mut log = Vec::new();
    story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
    // The member (party slot 1), Repth in his skill list and SP for it.
    let (member_id, member) = {
        let Stage::Area(a) = &mut s.stage else { panic!("left the area") };
        let id = a.world().party()[1];
        assert!(id > 0, "a member in slot 1: {:?}", a.world().party());
        let save = &mut a.world_mut().state_mut().save;
        let at = piney_fieldui::items::SKILL_LIST + 40 * id as usize;
        if !(0..20).any(|k| save.i16(at + 2 * k) == REPTH) {
            let free = (0..20).find(|&k| save.i16(at + 2 * k) < 0).unwrap();
            save.set_i16(at + 2 * free, REPTH);
        }
        let c = a.world_mut().combat_mut();
        let i = c.who(id).unwrap();
        c.scene.chars[i].sp = c.scene.chars[i].max_sp.max(60);
        (id, i)
    };
    let mut pad = Pad::default();
    let goal = [28600.0f32, 24600.0];
    let mut run = HealRun::default();
    // Each menu is left up 16 frames before its keys (for the shots).
    let (mut since, mut left) = (0u32, 0u32);
    for i in 0..4000u64 {
        let raw = {
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let w = a.world();
            let c = w.combat();
            let ui = a.ui();
            let m = &ui.ctrl;
            let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
            let t = ui.menu_type();
            if run.menus.last() != Some(&t) {
                run.menus.push(t);
                since = 0;
            }
            since += 1;
            let press =
                |b: Buttons| if i.is_multiple_of(8) && since > 16 { Raw { buttons: b, ..still } } else { still };
            let go_to = |row: i16| match m.list().select {
                s if s == row => Buttons::CROSS,
                s if s < row => Buttons::DOWN,
                _ => Buttons::UP,
            };
            run.fought |= c.battle.in_battle != 0;
            run.cast |= run.ordered && c.scene.chars[member].skill_id == REPTH;
            match t {
                -1 if run.fought && !run.ordered => press(Buttons::SQUARE),
                -1 => {
                    let k = c.kite.unwrap();
                    let p = c.scene.chars[k].pos.map(f32::from_bits);
                    let (dx, dy) = (goal[0] - p[0], goal[1] - p[1]);
                    let cam_z = f32::from_bits(w.camera().rot()[2]);
                    if dx * dx + dy * dy < 1500.0 * 1500.0 { still } else { stick_toward(cam_z, dx.atan2(-dy)) }
                }
                // CHAT: over to the Members page, the first member.
                3 if m.list().page != 2 => press(Buttons::RIGHT),
                3 => press(go_to(0)),
                71 => press(go_to(0)),
                // The recovery page, Repth.
                72 if m.list().page != 2 => press(Buttons::RIGHT),
                72 => {
                    let list = piney_fieldui::items::skill_list(&ui.texts().items, w.state(), member_id as usize, 2);
                    let row = list.iter().position(|&x| x == REPTH).unwrap_or(0) as i16;
                    press(go_to(row))
                }
                // Kite, the first of the party standing.
                73 => press(go_to(0)),
                _ => still,
            }
        };
        pad.read(&raw);
        let frame = s.step(&pad);
        for e in s.take_events() {
            if let Event::SkillWords { char_id, sid, .. } = e {
                run.words |= run.ordered && sid == i32::from(REPTH) && char_id != 0;
            }
        }
        each(&s, &frame);
        {
            let Stage::Area(a) = &s.stage else { panic!() };
            let m = &a.ui().ctrl;
            run.ordered |= m.menu == 73 && m.menu_next == -1;
        }
        // A second after the cast (the balloon still up), done.
        if run.cast && run.words {
            left += 1;
            if left > 60 {
                break;
            }
        }
    }
    Some(run)
}

/// CHAT's member menus in a fight ([`chat_member_heal`]): the order goes
/// to the member (`RequestChatCmd(member, 5, Kite, 150)`) and he casts
/// Repth (`skillID` 150 on his character, his `ccWordsPlay`).
#[test]
fn chat_a_member_to_heal_in_a_fight() {
    let Some(run) = chat_member_heal(|_, _| {}) else { return };
    println!("menus {:?}", run.menus);
    assert!(run.fought, "the battle mode came on");
    for n in [3, 71, 72, 73] {
        assert!(run.menus.contains(&n), "menu {n} opened: {:?}", run.menus);
    }
    assert!(run.ordered, "the order given from 73");
    assert!(run.cast, "the member cast Repth");
    assert!(run.words, "the member named Repth (ccWordsPlay)");
}

/// Shots of [`chat_member_heal`]'s menus, each 12 frames after it opens
/// (71, 72, 73), and of the order's balloon, into `PINEY_SHOTS` (default
/// /mnt/data/claude/scratch/chat_member): `cargo test --release -p
/// piney-game chat_member_shots -- --ignored --nocapture`.
#[test]
#[ignore]
fn chat_member_shots() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/chat_member".into());
    std::fs::create_dir_all(&dir).unwrap();
    let mut gs: Option<piney_gs::Gs> = None;
    let (mut open, mut since, mut taken, mut ordered) = (-1, 0u32, Vec::new(), false);
    let run = chat_member_heal(|s, frame| {
        let Stage::Area(a) = &s.stage else { return };
        let m = &a.ui().ctrl;
        // 0: from the order on (the balloon up).
        ordered |= m.menu == 73 && m.menu_next == -1;
        let t = if ordered { 0 } else { a.ui().menu_type() };
        if t != open {
            (open, since) = (t, 0);
        }
        since += 1;
        let want = matches!(t, 71..=73) && since == 12 || t == 0 && since == 20;
        if !want || taken.contains(&t) {
            return;
        }
        taken.push(t);
        let g = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
        g.set_overlay(Mode::archive(s));
        g.render(frame);
        let (wd, ht) = g.target_size();
        let name = if t == 0 { "order".to_string() } else { format!("menu{t}") };
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, piney_gs::png::encode(wd, ht, &g.read_back())).unwrap();
        println!("{path}");
    });
    if run.is_some() {
        assert_eq!(taken.len(), 4, "shots taken: {taken:?}");
    }
}
