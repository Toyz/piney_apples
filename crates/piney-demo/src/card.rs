//! The memory-card check at boot: `BootMem_Control` (DEMO.PRG,
//! DataControl.cpp; `Main_Control` 0x00403c00, `BootCheck` 0x00403d40) and the
//! part of `ccSaveSys` it asks (`BootCheckReq` 0x00171a60, `BootCheckProccess`
//! 0x00171a80, `NextProccess` 0x00173f90), on the one `ccSaveSys`
//! ([`piney_desktop::savesys::SaveSys`]) over the runtime's [`MemoryCard`]. No
//! card or no room asks a YES / NO question (0x8028, 0x8029) and keeps
//! checking, so a card put in ends it. The question draws on layer 130; see
//! docs/engine/title.md ("The memory-card check").

use piney_data::save::SaveData;
use piney_desktop::card::{MemoryCard, PortState};
use piney_desktop::savesys::SaveSys;

use crate::dataload::{BOOT_LAYER, CancelSound, DataControl, Draw, Keys, mask};
use crate::opening::Env;

/// `ccSaveSys.result` values the boot check sees.
pub mod result {
    /// `BootCheckReq`: the task is checking.
    pub const BUSY: u32 = 4;
    /// A slot passed, or the player said to start anyway.
    pub const DONE: u32 = 1;
    /// Bit 0x8000: a yes/no question is up.
    pub const QUESTION: u32 = 0x8000;
    /// Message 0x28: no memory card inserted; start anyway?
    pub const NO_CARD: u32 = 0x8028;
    /// Message 0x29: not enough space on the card; start anyway?
    pub const NO_ROOM: u32 = 0x8029;
}

/// `ccSaveSys.proccess` for the boot check.
pub const PROCCESS_BOOT_CHECK: i32 = 16;

/// Frames `ccThSaveSys` takes to answer a `BootCheckReq`: the
/// `ccBreathThread` in `BootCheckProccess`'s loop, then `CheckPort` on each
/// port with its `sceMcSync` waits. Not measured: one breath and one port.
pub const BOOT_CHECK_FRAMES: u32 = 2;

/// `BootCheckProccess`'s round while a question is up: a breath before
/// each port and before the answer.
pub const RECHECK_FRAMES: u32 = 3;

/// `BootCheckReq` (0x00171a60).
pub fn boot_check_req(sys: &mut SaveSys) {
    sys.proccess = PROCCESS_BOOT_CHECK;
    sys.result = result::BUSY;
}

/// The `ccThSaveSys` task, one frame (taken to run before the title's):
/// `BootCheckProccess` while proccess is 16, else `MainProccess`
/// ([`SaveSys::main_proccess_load`], which may load a slot into `save`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveSysTask {
    /// Frames until `BootCheckProccess`'s next answer; None before its
    /// first.
    pub wait: Option<u32>,
}

impl SaveSysTask {
    pub fn frame(&mut self, sys: &mut SaveSys, card: &mut dyn MemoryCard, save: &mut SaveData) {
        if sys.proccess != PROCCESS_BOOT_CHECK {
            self.wait = None;
            if sys.running {
                sys.main_proccess_load(card, save);
            }
            return;
        }
        if sys.result == result::DONE {
            return;
        }
        let w = self.wait.get_or_insert(BOOT_CHECK_FRAMES);
        *w -= 1;
        if *w > 0 {
            return;
        }
        let passes = |p: PortState| !matches!(p, PortState::NoCard | PortState::NotPs2 | PortState::Full);
        let s1 = card.check_port(0);
        let s2 = if passes(s1) { s1 } else { card.check_port(1) };
        if passes(s2) {
            sys.result = result::DONE;
            sys.proccess = 1;
            self.wait = None;
            return;
        }
        sys.result = if s1 == PortState::Full || s2 == PortState::Full { result::NO_ROOM } else { result::NO_CARD };
        self.wait = Some(RECHECK_FRAMES);
    }
}

/// `BootMem_Control` (0x94 bytes).
#[derive(Clone, Debug)]
pub struct BootMem {
    pub data: DataControl,
    /// +0x8c.
    pub state: i32,
    /// +0x90: -1 until a question has been asked.
    pub result: i32,
    /// The question up this frame, for inspection.
    pub shown: Option<u32>,
}

impl Default for BootMem {
    fn default() -> Self {
        Self::new()
    }
}

impl BootMem {
    /// The constructor (0x004042b0) and `BootMem_Control::Init`
    /// (0x00403e60).
    pub fn new() -> Self {
        let mut data = DataControl::new(BOOT_LAYER);
        data.hi = 17;
        data.info_x = 85;
        // (int)(m_hi * 7.5) in double.
        data.info_y = (17.0f64 * 7.5) as i32;
        BootMem { data, state: 0, result: -1, shown: None }
    }

    /// `Main_Control` (0x00403c00).
    pub fn main_control(&mut self, env: &mut Env) -> i32 {
        self.shown = None;
        self.data.cancel_sound = CancelSound::of(env.sys.volume);
        match self.state {
            0 => {
                boot_check_req(env.sys);
                self.state += 1;
            }
            1 => {
                self.data.mask |= mask::DIALOG | mask::QUIET_MOVE | mask::NO_DECIDE;
                self.boot_check(env);
                self.state += 1;
            }
            2 => {
                self.data.mask |= mask::DIALOG | mask::QUIET_MOVE | mask::NO_DECIDE;
                self.boot_check(env);
                self.data.main(env.pad, env.ok, env.cancel, env.req);
            }
            3 => return if self.result == -1 { 1 } else { 2 },
            _ => {}
        }
        self.result
    }

    /// `BootCheck` (0x00403d40).
    fn boot_check(&mut self, env: &mut Env) {
        let s = env.sys.result;
        if s == result::DONE {
            self.state += 1;
        }
        let keys = Keys { pad: env.pad, ok: env.ok, cancel: env.cancel };
        let mut d = Draw { ctx: env.ctx, assets: env.dialog, count: env.count, frame_rate: crate::FRAME_RATE };
        if s & result::QUESTION != 0 {
            self.result = 0;
            self.data.mask &= !(mask::QUIET_MOVE | mask::NO_DECIDE);
            let push = keys.pad.push.bits();
            if push & keys.cancel != 0 {
                self.data.step_cancel_sound(env.req);
                self.data.dialog = 0;
                env.sys.next_proccess(0);
            } else if push & keys.ok != 0 {
                env.sys.next_proccess(i32::from(self.data.dialog));
            }
            self.data.yes_no_dialogue(&mut d, self.data.dialog, true);
            self.shown = Some(s);
        }
        self.data.info_message(&mut d, s, true);
    }
}
