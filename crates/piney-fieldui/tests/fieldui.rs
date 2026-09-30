//! The field UI against Infection's disc: the tables it reads, and the
//! PERSONAL, Skills, Items, TARGET and CHAT menus driven by pad input the
//! way the runtime drives them. Skipped when the disc image is not
//! extracted; set PINEY_ISO to point at it elsewhere.
//!
//! The frame-by-frame checks against the game's own code (every state,
//! packet, text and sound) are in tools/test_fieldui_rs.py.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::SaveState;
use piney_fieldui::items::{self, Item};
use piney_fieldui::{CharInfo, FieldUi, Request, World};
use piney_input::{Buttons, Pad};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn member(handle: u32, id: i16, sp: i16, name: &[u8]) -> CharInfo {
    CharInfo {
        handle,
        types: 1,
        id,
        name: name.to_vec(),
        hp: 100,
        sp,
        max_hp: 100,
        max_sp: 80,
        attribute: -1,
        width: 40.0,
        in_view: true,
        ..CharInfo::default()
    }
}

struct Run {
    ui: FieldUi,
    world: World,
    save: SaveState,
    count: u32,
    requests: Vec<Request>,
}

impl Run {
    /// Kite and Orca in a field battle, a goblin in reach.
    fn new() -> Option<Run> {
        let mut iso = Iso::open(iso_path()?).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let ui = FieldUi::new(&mut iso, archive).unwrap();
        let mut world = World { boss_entry: -1, party_id: [0, 2, -1], ..World::default() };
        world.game.status = 5;
        world.game.area = 1;
        world.game.in_battle = 1;
        world.party[0] = Some(member(0x100, 0, 60, b"Kite"));
        world.party[1] = Some(member(0x101, 2, 50, b"Orca"));
        let goblin = CharInfo {
            handle: 0x300,
            types: 0x20,
            id: 10,
            name: b"Goblin".to_vec(),
            hp: 40,
            max_hp: 80,
            cmnd_dist: 300.0,
            width: 50.0,
            in_view: true,
            bar_res: 1,
            attribute: -1,
            ..CharInfo::default()
        };
        world.sorted.push(goblin.clone());
        world.ene_chain.push(goblin);
        let mut save = SaveState::fresh();
        // Saber Dance and Tiger Claws; three Health Drinks.
        for k in 0..20 {
            let v = [6i16, 7].get(k).copied().unwrap_or(-1);
            save.save.set_i16(items::SKILL_LIST + 2 * k, v);
        }
        for k in 0..40 {
            let at = items::ITEM_LIST + 4 * k;
            let (id, cat, num) = if k == 0 { (0, 10, 3) } else { (-1, -1, 0) };
            save.save.set_i16(at, id);
            save.save.set_u8(at + 2, cat as u8);
            save.save.set_u8(at + 3, num as u8);
        }
        Some(Run { ui, world, save, count: 0, requests: Vec::new() })
    }

    fn step(&mut self, push: Buttons, repeat: Buttons) -> piney_draw::Frame {
        self.count += 1;
        let pad = Pad { push, repeat, ..Pad::default() };
        let f = self.ui.step(&pad, &self.world, &mut self.save, self.count);
        for r in self.ui.take_requests() {
            // What the runtime does with the target changes.
            match r {
                Request::Target(h) => {
                    let c = self.world.sorted.iter().chain(self.world.party.iter().flatten()).find(|c| c.handle == h);
                    self.world.target_prev = self.world.target.take();
                    self.world.target = c.cloned();
                }
                Request::TargetClear if self.world.target.is_some() => {
                    self.world.target_prev = self.world.target.take();
                }
                _ => {}
            }
            self.requests.push(r);
        }
        f
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.step(Buttons::NONE, Buttons::NONE);
        }
    }

    fn press(&mut self, b: Buttons) {
        self.step(b, Buttons::NONE);
        self.idle(12);
    }

    fn menu(&self) -> i32 {
        self.ui.menu_type()
    }
}

#[test]
fn tables_from_the_disc() {
    let Some(r) = Run::new() else { return };
    let t = r.ui.texts();
    assert_eq!(t.lists.len(), 89);
    // PERSONAL in a field: Skills, Items, Key Items, Discard, Status,
    // Equipment, Area Information, Gate Out.
    assert_eq!(t.lists[1].items, [4, 5, 6, 7, 8, 63, 87, 10]);
    // The menus the events open.
    assert_eq!(t.lists[75].items, [4, 5, 6, 7, 8, 63, 76, 11]);
    assert_eq!(t.lists[80].items[0], 81);
    let s = &t.items;
    assert_eq!(s.skills.len(), piney_battle::tables::SKILLS);
    assert_eq!((s.skills[2].cost, s.skills[2].target_type, s.skills[2].kind), (10, 0xe0, 2));
    assert_eq!(s.skills[180].target_type & 3, 2);
    assert_eq!(s.pages, [-1, 11, 12, 14, -2]);
    assert_eq!(s.items.len(), 16);
    assert_eq!(s.item(10, 0).map(|p| p.skill), Some(150));
    assert_eq!(s.item_icon(10), 14);
    assert_eq!(t.skill_tags.len(), 6);
    assert_eq!(t.item_help.len(), 10);
    assert_eq!(t.virus_col.len(), 6);
    assert_eq!(t.bt_chat_act, [0, 16, 17, 18, 15, 13, 14, 19, 7, 8, 9, 10]);
    assert_eq!(t.town_chat_act, [9, 12]);
    assert_eq!(t.char_names.len(), 18);
    assert_eq!(t.char_names[2], b"Orca");
    assert!(t.tutorial.contains_key(&2) && t.tutorial_parody.contains_key(&4));
}

#[test]
fn a_skill_on_the_goblin() {
    let Some(mut r) = Run::new() else { return };
    r.idle(2);
    r.ui.open_menu(1);
    r.idle(12);
    assert_eq!(r.menu(), 1);
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 4, "Skills");
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 65, "TARGET");
    assert_eq!(r.world.target.as_ref().map(|c| c.handle), Some(0x300));
    r.step(Buttons::CROSS, Buttons::NONE);
    assert!(r.requests.contains(&Request::Skill { target: 0x300, skill: 6 }));
    r.idle(20);
    assert_eq!(r.menu(), -1, "shut after the skill");
    assert!(r.requests.contains(&Request::WakeAll));
}

/// An area skill (`type` 0x6000) chosen on the goblin at (1000, 0): the
/// small squares (`subTarget`, drawn at 0x0051f3d8) its other targets
/// get. Beside Kite at the origin stands a goblin at (0, 300), beside the
/// target one at (1000, 300); a third, far off, moves out of reach after.
fn area_marks(skill: i16) -> Option<[Vec<(f32, f32)>; 2]> {
    use piney_fieldui::ctrl::Draw;
    use piney_fieldui::spr::Obj;
    let mut r = Run::new()?;
    for k in 0..20 {
        r.save.save.set_i16(items::SKILL_LIST + 2 * k, if k == 0 { skill } else { -1 });
    }
    let goblin = |handle: u32, x: f32, y: f32, tag: (i32, i32)| CharInfo {
        handle,
        types: 0x20,
        width: 50.0,
        pos_p: [x, y, 0.0],
        tag: Some(tag),
        ..CharInfo::default()
    };
    let t = &mut r.world.ene_chain[0];
    t.pos_p = [1000.0, 0.0, 0.0];
    t.tag = Some((200, 150));
    r.world.sorted[0] = t.clone();
    r.world.ene_chain.push(goblin(0x301, 0.0, 300.0, (100, 150)));
    r.world.ene_chain.push(goblin(0x302, 1000.0, 300.0, (300, 150)));
    let marks = |r: &Run| -> Vec<(f32, f32)> {
        let packets = r.ui.draws().iter().filter_map(|d| match d {
            Draw::Send(p) => Some(p),
            _ => None,
        });
        packets
            .flatten()
            .filter(|p| p.obj == Obj::TargetCursol && p.su == 24 && p.rot == 0.0)
            .map(|p| (p.dx, p.dy))
            .collect()
    };
    r.idle(2);
    r.ui.open_menu(1);
    r.idle(12);
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 4, "Skills");
    // The skill's page.
    for _ in 0..6 {
        let l = r.ui.ctrl.list();
        if items::skill_list(&r.ui.texts().items, &r.save, 0, i32::from(l.page))[0] == skill {
            break;
        }
        r.step(Buttons::NONE, Buttons::RIGHT);
        r.idle(4);
    }
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 65, "TARGET");
    assert_eq!(r.world.target.as_ref().map(|c| c.handle), Some(0x300));
    let near = marks(&r);
    for c in &mut r.world.ene_chain[1..] {
        c.pos_p[1] = 3000.0;
    }
    r.idle(1);
    Some([near, marks(&r)])
}

/// Issue #6. Tiger Claws (skill 7, type 0x2801, range 400) strikes about
/// Kite (bit 0x2000): the goblin beside him is marked. An attack spell
/// (skill 193, type 0xc106, range 400) strikes about its target (bit
/// 0x4000 alone): the goblin beside the target. Out of reach, no mark.
#[test]
fn an_area_skill_marks_its_other_targets() {
    let Some([art, gone]) = area_marks(7) else { return };
    assert_eq!(art, [(100.0, 150.0)], "about Kite");
    assert_eq!(gone, []);
    let Some([spell, gone]) = area_marks(193) else { return };
    assert_eq!(spell, [(300.0, 150.0)], "about the target");
    assert_eq!(gone, []);
}

#[test]
fn a_health_drink_on_orca() {
    let Some(mut r) = Run::new() else { return };
    r.idle(2);
    r.ui.open_menu(1);
    r.idle(12);
    r.step(Buttons::NONE, Buttons::DOWN);
    r.idle(4);
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 5, "Items");
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 65, "TARGET");
    r.step(Buttons::NONE, Buttons::DOWN);
    r.idle(4);
    r.step(Buttons::CROSS, Buttons::NONE);
    assert!(r.requests.contains(&Request::UseItem { target: 0x101, code: 0xa0000 }));
    assert_eq!(items::save_item(&r.save, 0, 0), Item { id: 0, cat: 10, num: 2 });
}

#[test]
fn chat_orders_the_party() {
    let Some(mut r) = Run::new() else { return };
    r.idle(2);
    r.ui.open_menu(3);
    r.idle(12);
    assert_eq!(r.menu(), 3);
    r.step(Buttons::CROSS, Buttons::NONE);
    assert!(r.requests.contains(&Request::ChatCmd { member: 0x101, cmd: 0 }));
    assert!(r.requests.iter().any(|q| matches!(q, Request::OpenChat(t) if !t.is_empty())));
    r.idle(8);
    assert_eq!(r.menu(), -1);
}

#[test]
fn frames_draw_the_hud() {
    let Some(mut r) = Run::new() else { return };
    r.idle(2);
    r.ui.open_menu(1);
    r.idle(6);
    let f = r.step(Buttons::NONE, Buttons::NONE);
    assert!(!f.cmds.is_empty(), "the menu layer draws");
}

#[test]
fn gate_and_item_tables_from_the_disc() {
    let Some(r) = Run::new() else { return };
    let t = r.ui.texts();
    // The twelve word tables, 305 words.
    assert_eq!(t.words.words.len(), 305);
    assert!(!t.words.events.is_empty());
    assert_eq!(t.gate_help.len(), 5);
    assert_eq!(t.get_item_str[0], b"You now have ");
    assert_eq!(t.area_items.box_list.len(), 30);
    assert!(t.area_items.box_list.iter().chain(&t.area_items.suka).all(|l| l.len() == 130));
    assert_eq!(t.status_class.len(), 6);
}

/// Kite and Orca in Mac Anu at the Chaos Gate, three words of each part
/// held.
fn at_the_gate(r: &mut Run) {
    let gate = CharInfo { handle: 0x500, types: 0x2000, id: 16, name: b"Chaos Gate".to_vec(), ..CharInfo::default() };
    r.world.game.area = 0;
    r.world.game.in_battle = 0;
    r.world.sorted.clear();
    r.world.target = Some(gate);
    r.world.area_codes = [(-1, -1); 16];
    r.save.save.set_i32(0x523c, 0x0C00_6006);
}

#[test]
fn the_gate_warps_the_party() {
    let Some(mut r) = Run::new() else { return };
    at_the_gate(&mut r);
    r.idle(2);
    r.ui.ctrl.first_time = 1;
    r.ui.open_menu(28);
    r.idle(12);
    assert_eq!(r.menu(), 28);
    assert!(r.requests.contains(&Request::Affect { target: 0x500, kind: 11 }));
    // Random, then Warp.
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 57);
    r.press(Buttons::CROSS);
    r.idle(220);
    assert!(r.requests.contains(&Request::TransferOut(0x100)));
    assert!(r.requests.contains(&Request::TransferOut(0x101)));
    let ids = r.requests.iter().find_map(|q| if let Request::GoToArea(w) = q { Some(*w) } else { None });
    let ids = ids.expect("the area change");
    let words = &r.ui.texts().words;
    for (slot, id) in ids.iter().enumerate() {
        assert_eq!(words.part(*id), slot as i32, "word {id} in part {slot}");
    }
    assert!(r.requests.contains(&Request::DeleteNoPartyMember));
}

#[test]
fn the_party_tutorial_adds_orca() {
    let Some(mut r) = Run::new() else { return };
    r.world.party[1] = None;
    r.world.party_id = [0, -1, -1];
    r.world.game.in_battle = 0;
    r.save.save.set_i32(0x2220, 0b101);
    r.idle(2);
    r.ui.open_menu(75);
    r.idle(12);
    r.press(Buttons::TRIANGLE);
    for _ in 0..6 {
        r.step(Buttons::NONE, Buttons::DOWN);
        r.idle(4);
    }
    r.press(Buttons::CROSS);
    assert_eq!(r.menu(), 76);
    for _ in 0..40 {
        if r.menu() == -1 {
            break;
        }
        r.step(Buttons::CROSS, Buttons::NONE);
        r.idle(8);
        if r.requests.contains(&Request::AddMember(2)) && r.world.party[1].is_none() {
            // What the runtime does: Orca made and in slot 1.
            r.world.party[1] = Some(member(0x101, 2, 50, b"Orca"));
            r.world.party_id[1] = 2;
        }
    }
    assert!(r.requests.contains(&Request::AddMember(2)));
    assert_eq!(r.menu(), -1);
}

#[test]
fn the_box_tutorial_gives_its_item() {
    let Some(mut r) = Run::new() else { return };
    r.world.game.in_battle = 0;
    r.world.sorted.clear();
    r.world.target = Some(CharInfo { handle: 0x600, types: 0x4000, item: 0xa0001, ..CharInfo::default() });
    r.idle(2);
    r.ui.open_menu(84);
    r.idle(12);
    for _ in 0..20 {
        if r.menu() == -1 {
            break;
        }
        r.step(Buttons::CROSS, Buttons::NONE);
        r.idle(8);
    }
    assert!(r.requests.contains(&Request::Affect { target: 0x600, kind: 11 }));
    assert_eq!(items::save_item(&r.save, 0, 1), Item { id: 1, cat: 10, num: 1 });
    assert_eq!(r.save.save.i16(0x7440), 1, "itemBoxCount");
}
