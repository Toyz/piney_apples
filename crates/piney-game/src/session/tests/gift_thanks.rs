//! Issue #58: Gift to a party member, the menus driven as the runtime
//! drives them, on every volume's disc. The member's thanks are a record
//! of the volume's `spcMsgPresent10` (its address read from the volume's
//! tables); Infection's address read on Mutation found no record, and
//! BlackRose said `errorData` in red. Skipped when a disc is not there.
//!
//! The frame-by-frame checks against the game's own `PresentMenu` are
//! tools/test_fieldui_talk_rs.py's `GiftPages`.

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::by_id;
use piney_data::volume::Volume;
use piney_desktop::SaveState;
use piney_fieldui::talk::{Speaker, TalkTarget};
use piney_fieldui::{CharInfo, FieldUi, Request, World};
use piney_input::{Buttons, Pad};

/// BlackRose's `charTbl` row.
const BLACK_ROSE: i16 = 15;
const HANDLE: u32 = 0x300;

struct Run {
    archive: Arc<Archive>,
    ui: FieldUi,
    world: World,
    save: SaveState,
    count: u32,
}

impl Run {
    /// Kite in Mac Anu with a Health Drink, BlackRose spoken to (her menu,
    /// 21, opened as `ccThGameCtrl` opens it, 8 frames in); None without
    /// the disc.
    fn new(volume: Volume) -> Option<Run> {
        let file = piney_data::pack::disc_name(volume).trim_end_matches(".disc");
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{file}/{file}.iso"));
        if !iso.exists() {
            return None;
        }
        let mut iso = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut ui = FieldUi::new(&mut iso, archive.clone()).unwrap();
        let mut save = SaveState::fresh_with(&piney_desktop::InitText::from_disc(&mut iso).unwrap());
        piney_fieldui::menus::talk::set_spc_base_msg(&mut save, volume);
        let at = by_id::spc_param(BLACK_ROSE as usize);
        save.save.set_i32(at + 8, 6);
        save.save.set_i16(at + 0xc, BLACK_ROSE);
        for k in 0..40 {
            let at = by_id::item_list(0) + 4 * k;
            let (id, cat, num) = if k == 0 { (0, 10, 3) } else { (-1, -1, 0) };
            save.save.set_i16(at, id);
            save.save.set_u8(at + 2, cat as u8);
            save.save.set_u8(at + 3, num as u8);
        }
        let mut world = World { boss_entry: -1, party_id: [0, i32::from(BLACK_ROSE), -1], ..World::default() };
        world.game.status = 5;
        let her = CharInfo {
            handle: HANDLE,
            types: 6,
            id: BLACK_ROSE,
            name: b"BlackRose".to_vec(),
            in_view: true,
            ..CharInfo::default()
        };
        world.party[0] = Some(CharInfo { handle: 0x100, types: 1, name: b"Kite".to_vec(), ..CharInfo::default() });
        world.party[1] = Some(her.clone());
        world.target = Some(her);
        ui.talk_to(Some(TalkTarget { handle: HANDLE, who: Speaker::Spc(i32::from(BLACK_ROSE)) }));
        let mut r = Run { archive, ui, world, save, count: 0 };
        r.press(Buttons::NONE, 8);
        r.ui.ctrl.open_req = 21;
        r.ui.ctrl.mode = 1;
        r.ui.ctrl.first_time = 1;
        Some(r)
    }

    fn step(&mut self, push: Buttons) -> piney_draw::Frame {
        self.count += 1;
        let pad = Pad { push, repeat: push, ..Pad::default() };
        let frame = self.ui.step(&pad, &self.world, &mut self.save, self.count);
        for r in self.ui.take_requests() {
            // The runtime's side of `ccChangeCmndTarget`.
            match r {
                Request::Target(h) => {
                    let c = self.world.target.iter().chain(&self.world.target_prev).find(|c| c.handle == h).cloned();
                    self.world.target_prev = self.world.target.take();
                    self.world.target = c;
                }
                Request::TargetClear if self.world.target.is_some() => {
                    self.world.target_prev = self.world.target.take();
                }
                _ => {}
            }
        }
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

    /// Gift (her list's third row), the Health Drink, one, OK: the frame
    /// her thanks open on.
    fn give(&mut self) -> piney_draw::Frame {
        self.press(Buttons::NONE, 12);
        assert_eq!(self.ui.menu_type(), 21, "her menu");
        self.press(Buttons::DOWN, 6);
        self.press(Buttons::DOWN, 6);
        self.press(Buttons::CROSS, 12);
        assert_eq!(self.ui.menu_type(), 50, "Gift");
        self.press(Buttons::CROSS, 8);
        self.press(Buttons::CROSS, 25);
        self.press(Buttons::CROSS, 20)
    }
}

/// Gift on BlackRose: her thanks are her own record on every volume, not
/// `errorData`.
#[test]
fn blackroses_thanks_for_a_gift_are_hers_on_every_volume() {
    for volume in Volume::ALL {
        let Some(mut r) = Run::new(volume) else { continue };
        r.give();
        let f = piney_data::tables::fieldui::of(volume);
        let chain = r.ui.ctrl.talk.chain.clone().unwrap_or_else(|| panic!("{volume}: no thanks"));
        assert_eq!(chain.grp, -32, "{volume}: the gift's voice group");
        assert_ne!(chain.rec, f.error_data_va(), "{volume}: errorData for BlackRose's thanks");
        // One of her five thanks, by the drink's worth.
        let first = r.ui.texts().talk.word(f.spc_msg_present10_va() + 4 * BLACK_ROSE as u32);
        assert!((0..5).any(|k| chain.rec == first + 12 * k), "{volume}: {:#x} not her thanks ({first:#x})", chain.rec);
        let lines = r.ui.texts().talk.ev_msg(chain.rec).lines;
        assert!(!lines[0].is_empty() && !lines[0].starts_with(b"#R"), "{volume}: {lines:?}");
    }
}

/// The menu layer as her thanks open on Mutation, to `$PINEY_SHOTS`
/// (/mnt/data/claude/scratch/i58) as `mut-blackrose-thanks.png`.
#[test]
#[ignore]
fn blackroses_thanks_shot() {
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/i58".into());
    std::fs::create_dir_all(&dir).unwrap();
    let Some(mut r) = Run::new(Volume::Mut) else { return };
    let frame = r.give();
    let mut g = piney_gs::Gs::headless(piney_gs::Assets::new(r.archive.clone())).unwrap();
    g.render(&frame);
    let (w, h) = g.target_size();
    let path = format!("{dir}/mut-blackrose-thanks.png");
    std::fs::write(&path, piney_gs::png::encode(w, h, &g.read_back())).unwrap();
    println!("{path}");
}
