//! What the title shares with the rest of the game: the memory card
//! (`piney_desktop::card::MemoryCard`, the desktop's seam; [`MemCard`] is an
//! in-memory one) and the system menu (`ccThDtMenu`) the Option item opens
//! ([`TitleMenu`], the desktop's `ccDtMenu` in the title's mode; a
//! stand-in, [`StubMenu`], closes on cancel).

use piney_data::archive::Archive;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::card::{MemoryCard, PortState};
use piney_desktop::dtmenu::{DtMenu, MenuCtx};
use piney_desktop::kanji::{Fonts, Names};
use piney_desktop::message::WindowTexture;
use piney_desktop::savesys::{INDEX_SIZE, INFO_COUNT};
use piney_input::Pad;

use crate::Request;

/// A memory card kept in memory, in MEMORY CARD slot 1 (slot 2 empty):
/// formatted, with room, and the game's save directory (its index and the
/// twelve slot files) when `dir` is set. The default is such a card with no
/// Infection save: the boot check passes, the load screen says there is no
/// saved data. The runtime gives the title the card the desktop saves to
/// (`piney_desktop::card::FilesCard`) instead.
#[derive(Clone, Debug)]
pub struct MemCard {
    pub dir: bool,
    pub index: [u8; INDEX_SIZE],
    pub slots: Vec<Vec<u8>>,
    /// The slot files' size `make_dir` writes (Infection's by default).
    pub slot_size: usize,
}

impl Default for MemCard {
    fn default() -> Self {
        let slot_size = piney_data::save::SIZE;
        MemCard { dir: false, index: [0; INDEX_SIZE], slots: vec![vec![0; slot_size]; INFO_COUNT], slot_size }
    }
}

impl MemoryCard for MemCard {
    fn check_port(&mut self, port: i32) -> PortState {
        match (port, self.dir) {
            (0, true) => PortState::Ready,
            (0, false) => PortState::NoDirectory,
            _ => PortState::NoCard,
        }
    }

    fn read_index(&mut self, port: i32) -> Option<Vec<u8>> {
        (port == 0 && self.dir).then(|| self.index.to_vec())
    }

    fn write_index(&mut self, port: i32, index: &[u8; INDEX_SIZE]) -> bool {
        let ok = port == 0 && self.dir;
        if ok {
            self.index = *index;
        }
        ok
    }

    fn write_slot(&mut self, port: i32, slot: usize, data: &[u8]) -> bool {
        let ok = port == 0 && self.dir && slot < INFO_COUNT;
        if ok {
            self.slots[slot] = data.to_vec();
        }
        ok
    }

    fn format(&mut self, port: i32) -> bool {
        port == 0
    }

    fn make_dir(&mut self, port: i32) -> bool {
        if port != 0 {
            return false;
        }
        self.dir = true;
        self.index = [0; INDEX_SIZE];
        self.slots = vec![vec![0; self.slot_size]; INFO_COUNT];
        true
    }

    /// One directory: `vol` is not told apart.
    fn read_slot(&mut self, port: i32, slot: usize, _vol: i32, size: usize) -> Option<Vec<u8>> {
        if port != 0 || !self.dir {
            return None;
        }
        self.slots.get(slot).filter(|b| b.len() >= size).map(|b| b[..size].to_vec())
    }
}

/// `ccDtMenu::CheckMenuType` (0x0016a8e0) while no menu is up.
pub const MENU_NONE: i32 = -1;
/// The menu `PlayOption` opens (`openReqNum = 1`): the title's OPTION list.
pub const MENU_SETTINGS: i32 = 1;
/// `CheckMenuType` while `menu != menuNext` (changing).
pub const MENU_CHANGING: i32 = 12;

/// What the system menu gets each frame.
pub struct MenuFrame<'a> {
    pub pad: &'a Pad,
    /// The save: the options the menu sets (`vibration`, `voice`,
    /// `strWinMode`, the screen offset, the volumes, `camType`).
    pub state: &'a mut SaveState,
    /// What it asks of the game: sounds as [`Request::Se`], the rest as
    /// [`Request::Menu`].
    pub req: &'a mut Vec<Request>,
    pub names: Names,
}

/// The system menu task `ccThDtMenu` (`ccDtMenu`, dtMenu 0x003789d0), which
/// `ccSetupDemo` starts after `ccThDemo` (both priority 33, so it runs
/// after the title each frame) and which opens the options for Option.
pub trait SystemMenu {
    /// `dtMenu->openReqNum = n`.
    fn request(&mut self, n: i16);
    /// `dtMenu->dispFlag`: `ccThDemo` sets it 0 while the logos play and 1
    /// from the title on; the task draws (`Disp`, which also steps the
    /// window's alpha) only while it is set.
    fn set_display(&mut self, on: bool);
    /// `ccDtMenu::CheckMenuType()`.
    fn menu_type(&self) -> i32;
    /// One frame of `ccThDtMenu`, after the title's.
    fn frame(&mut self, x: &mut MenuFrame);
    /// `ccDtMenu::Disp` onto the title's frame, when `dispFlag` is set.
    fn disp(&mut self, _ctx: &mut Ctx, _fonts: &Fonts, _names: &Names) {}
}

/// `ccDtMenu.openReqNum` with nothing asked for.
pub const NO_REQUEST: i16 = -1;

/// The system menu as the title runs it: `piney_desktop::dtmenu`'s
/// `ccDtMenu` with `ccGame.status` 1 (`ccSetupDemo`) and `enableReset` 0,
/// so START opens nothing and the options (list 1: Controller, Vibrate,
/// Adjust Screen, Sound, Voiceover, Movie Text) open over the running
/// title, with the title's sounds and without the desktop's dim.
pub struct TitleMenu {
    pub menu: DtMenu,
    /// `dispFlag`.
    pub display: bool,
}

impl TitleMenu {
    /// `ccThDtMenu`'s `new ccDtMenu`, with the menu window's texture and
    /// the Controller's picture (`XCONTROL.CCS`) from `DATA.BIN`.
    pub fn new(volume: piney_data::volume::Volume, archive: &Archive) -> Self {
        let window = WindowTexture::read(archive);
        let control = WindowTexture::read_named(
            archive,
            piney_desktop::dtmenu::CONTROL_FILE,
            piney_desktop::dtmenu::CONTROL_TEX,
            None,
        );
        let mut menu = DtMenu::new(volume, window, control);
        menu.title = true;
        TitleMenu { menu, display: true }
    }
}

impl SystemMenu for TitleMenu {
    fn request(&mut self, n: i16) {
        self.menu.open_req = n;
    }

    fn set_display(&mut self, on: bool) {
        self.display = on;
    }

    fn menu_type(&self) -> i32 {
        self.menu.check_menu_type()
    }

    fn frame(&mut self, x: &mut MenuFrame) {
        let mut req = Vec::new();
        let mut m = MenuCtx {
            save: &mut *x.state,
            req: &mut req,
            pad: x.pad,
            enable_reset: false,
            names: x.names.clone(),
            save_sys: None,
        };
        self.menu.frame(&mut m);
        for r in req {
            x.req.push(match r {
                piney_desktop::Request::Se(se) => Request::Se(se.0),
                other => Request::Menu(other),
            });
        }
    }

    fn disp(&mut self, ctx: &mut Ctx, fonts: &Fonts, names: &Names) {
        if self.display {
            self.menu.disp(ctx, fonts, names, crate::FRAME_RATE);
        }
    }
}

/// A stand-in menu for tests without the disc: a request opens it the same
/// frame, the cancel button closes it the frame it is pushed. Nothing is
/// drawn.
#[derive(Clone, Copy, Debug)]
pub struct StubMenu {
    pub menu: i32,
    /// `openReqNum`.
    pub request: i16,
    pub display: bool,
}

impl Default for StubMenu {
    fn default() -> Self {
        StubMenu { menu: MENU_NONE, request: NO_REQUEST, display: false }
    }
}

impl SystemMenu for StubMenu {
    fn request(&mut self, n: i16) {
        self.request = n;
    }

    fn set_display(&mut self, on: bool) {
        self.display = on;
    }

    fn menu_type(&self) -> i32 {
        self.menu
    }

    fn frame(&mut self, x: &mut MenuFrame) {
        if self.request != NO_REQUEST {
            self.menu = i32::from(self.request);
            self.request = NO_REQUEST;
        } else if self.menu != MENU_NONE && x.pad.push.bits() & x.state.cancel() != 0 {
            self.menu = MENU_NONE;
        }
    }
}
