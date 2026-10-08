//! Issue #63: a walking PC's Talk, the menus driven as the runtime drives
//! them, on every volume's disc. Mutation's PCs' tables grew to 24 lines,
//! many of two boxes; the talk records the port read were Infection's
//! shapes carried over, so a second box (`Check(1)`'s next record) and
//! some first ones were `errorData` in red. Skipped when a disc is not
//! there. The frame-by-frame checks against the game's own `PcMenu` and
//! `TalkMenu` are tools/test_fieldui_shop_rs.py's `TalkPages`.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::by_id;
use piney_data::tables::{battle, fieldui, sjis};
use piney_data::volume::Volume;
use piney_desktop::SaveState;
use piney_fieldui::talk::{Speaker, TalkTarget};
use piney_fieldui::{CharInfo, FieldUi, World};
use piney_input::{Buttons, Pad};

/// Oborozukiyo's and Nijukata's `npcTbl` rows.
const OBOROZUKIYO: i32 = 61;
const NIJUKATA: i32 = 51;
const HANDLE: u32 = 0x300;

struct Run {
    archive: Arc<Archive>,
    ui: FieldUi,
    world: World,
    save: SaveState,
    count: u32,
}

/// The disc's archive, field UI and a fresh save; None without the disc.
fn open(volume: Volume) -> Option<(Arc<Archive>, FieldUi, SaveState)> {
    let file = piney_data::pack::disc_name(volume).trim_end_matches(".disc");
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{file}/{file}.iso"));
    if !iso.exists() {
        return None;
    }
    let mut iso = Iso::open(&iso).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let ui = FieldUi::new(&mut iso, archive.clone()).unwrap();
    let save = SaveState::fresh_with(&piney_desktop::InitText::from_disc(&mut iso).unwrap());
    Some((archive, ui, save))
}

impl Run {
    /// Kite alone in Mac Anu, no one spoken to yet.
    fn new((archive, ui, save): (Arc<Archive>, FieldUi, SaveState)) -> Run {
        let mut world = World { boss_entry: -1, party_id: [0, -1, -1], ..World::default() };
        world.game.status = 5;
        world.party[0] = Some(CharInfo { handle: 0x100, types: 1, name: b"Kite".to_vec(), ..CharInfo::default() });
        let mut r = Run { archive, ui, world, save, count: 0 };
        r.press(Buttons::NONE, 8);
        r
    }

    /// PC `row` spoken to at story `story` (`talkNum[0]`): PcMenu, 22,
    /// opened as `ccThGameCtrl` opens it.
    fn speak(&mut self, volume: Volume, row: i32, story: u8) {
        self.save.save.set_u8(by_id::talk_num(0), story);
        let name = battle::of(volume).npcs()[row as usize].param.base.name.map(sjis::encode).unwrap_or_default();
        self.world.target =
            Some(CharInfo { handle: HANDLE, types: 8, id: row as i16, name, in_view: true, ..CharInfo::default() });
        self.ui.talk_to(Some(TalkTarget { handle: HANDLE, who: Speaker::Npc(row) }));
        self.ui.ctrl.open_req = 22;
        self.ui.ctrl.mode = 1;
        self.ui.ctrl.first_time = 1;
        self.press(Buttons::NONE, 12);
        assert_eq!(self.ui.menu_type(), 22, "{volume}: row {row}'s list");
    }

    /// Cancel out of the list.
    fn leave(&mut self) {
        let cancel = Buttons(self.save.cancel());
        self.press(cancel, 30);
        assert_ne!(self.ui.menu_type(), 22, "the list closed");
    }

    fn step(&mut self, push: Buttons) -> piney_draw::Frame {
        self.count += 1;
        let pad = Pad { push, repeat: push, ..Pad::default() };
        let frame = self.ui.step(&pad, &self.world, &mut self.save, self.count);
        self.ui.take_requests();
        frame
    }

    /// `b` pushed, then `n` frames at rest: the last frame.
    fn press(&mut self, b: Buttons, n: usize) -> piney_draw::Frame {
        let mut f = self.step(b);
        for _ in 0..n {
            f = self.step(Buttons::NONE);
        }
        f
    }

    /// Talk from the list, and OK through its boxes until the list is back:
    /// the record of each box, in order.
    fn talk(&mut self) -> Vec<u32> {
        let mut boxes = Vec::new();
        self.press(Buttons::CROSS, 12);
        for _ in 0..8 {
            if self.ui.menu_type() != 47 {
                break;
            }
            let rec = self.ui.ctrl.talk.chain.as_ref().map_or(0, |c| c.rec);
            if boxes.last() != Some(&rec) {
                boxes.push(rec);
            }
            self.press(Buttons::CROSS, 12);
        }
        assert_eq!(self.ui.menu_type(), 22, "the list after Talk");
        self.press(Buttons::NONE, 12);
        boxes
    }
}

/// Whether the record at `rec` is a line of the PC's, not `errorData` (Sieg,
/// Kaz and the sign PCs have lines of empty text, as on the disc).
fn is_line(ui: &FieldUi, volume: Volume, rec: u32) -> bool {
    rec != fieldui::of(volume).error_data_va() && !ui.texts().talk.ev_msg(rec).lines[0].starts_with(b"#R")
}

/// Every walking PC's Talk at every story stage, three times round its
/// lines: every box is one of its lines, on every volume. Before the fix 34
/// of Mutation's 36 walking PCs had a box of `errorData` (152 boxes).
#[test]
fn every_walking_pcs_every_box_is_a_line_on_every_volume() {
    for volume in Volume::ALL {
        let Some(disc) = open(volume) else { continue };
        let mut r = Run::new(disc);
        let rows: Vec<i32> = battle::of(volume)
            .npcs()
            .iter()
            .enumerate()
            .filter(|(_, n)| n.param.base.kind & 8 != 0 && n.param.base.msg != 0)
            .map(|(k, _)| k as i32)
            .filter(|&k| (30..66).contains(&k) || (volume != Volume::Inf && matches!(k, 120 | 121 | 180..=183)))
            .collect();
        let mut bad = Vec::new();
        let mut seen = 0;
        for &row in &rows {
            for story in 0..=6 {
                r.speak(volume, row, story);
                for k in 0..3 {
                    for (b, rec) in r.talk().into_iter().enumerate() {
                        seen += 1;
                        if !is_line(&r.ui, volume, rec) {
                            bad.push(format!("row {row} story {story} talk {k} box {b}: {rec:#x}"));
                        }
                    }
                }
                r.leave();
            }
        }
        assert!(seen > rows.len() * 21, "{volume}: {seen} boxes");
        assert!(bad.is_empty(), "{volume}: {} of {seen} boxes not a line:\n{}", bad.len(), bad.join("\n"));
    }
}

/// The report's PCs: Oborozukiyo's "I totally forgot about the fanfic..."
/// (story 3, line 14) and its second box, and Nijukata's two-box lines.
#[test]
fn oborozukiyos_second_box_is_hers() {
    let Some(disc) = open(Volume::Mut) else { return };
    let mut r = Run::new(disc);
    r.speak(Volume::Mut, OBOROZUKIYO, 3);
    let boxes: Vec<u32> = (0..3).flat_map(|_| r.talk()).collect();
    let texts = |rec: u32| r.ui.texts().talk.ev_msg(rec).lines[0].clone();
    let at = boxes.iter().position(|&b| texts(b).starts_with(b"I totally forgot about the")).expect("the fanfic line");
    assert_eq!(texts(boxes[at + 1]), b"Oh well. It's not the first time!".to_vec());
    r.leave();
    r.speak(Volume::Mut, NIJUKATA, 6);
    let boxes: Vec<u32> = (0..3).flat_map(|_| r.talk()).collect();
    assert!(boxes.len() > 3, "Nijukata's lines of two boxes: {boxes:x?}");
    assert!(boxes.iter().all(|&b| is_line(&r.ui, Volume::Mut, b)), "{boxes:x?}");
}

/// The menu layer at Oborozukiyo's second box on Mutation, to
/// `$PINEY_SHOTS` (/mnt/data/claude/scratch/i63) as
/// `mut-oborozukiyo-second-box.png`.
#[test]
#[ignore]
fn oborozukiyo_shot() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i63".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(disc) = open(Volume::Mut) else { return };
    let mut r = Run::new(disc);
    r.speak(Volume::Mut, OBOROZUKIYO, 3);
    // Talk until the fanfic line is up, then OK to its second box.
    for _ in 0..3 {
        r.press(Buttons::CROSS, 12);
        let first = r.ui.ctrl.talk.chain.as_ref().map_or(0, |c| c.rec);
        if r.ui.texts().talk.ev_msg(first).lines[0].starts_with(b"I totally forgot") {
            break;
        }
        while r.ui.menu_type() == 47 {
            r.press(Buttons::CROSS, 12);
        }
        r.press(Buttons::NONE, 12);
    }
    // The first box typed out, OK, and the second typed out.
    let first = r.ui.ctrl.talk.chain.as_ref().map_or(0, |c| c.rec);
    r.press(Buttons::NONE, 90);
    r.press(Buttons::CROSS, 0);
    for _ in 0..30 {
        if r.ui.ctrl.talk.chain.as_ref().map_or(0, |c| c.rec) != first {
            break;
        }
        r.press(Buttons::NONE, 0);
    }
    let frame = r.press(Buttons::NONE, 90);
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(r.archive.clone())).unwrap();
    g.render(&frame);
    let (w, h) = g.target_size();
    let path = format!("{dir}/mut-oborozukiyo-second-box.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    println!("{path}");
}
