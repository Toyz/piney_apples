//! The OPTION pages (gcmn `menu.cpp`), the eight rows of OPTION's list
//! (menu 12, `SystemMenu`): `ControllerMenu` (13, 0x0053d0e0),
//! `VibrationMenu` (14, 0x0053ea80), `ScreenMenu` (15, 0x0053f060),
//! `SoundMenu` (16, 0x0053f800), `ResetMenu` (17, 0x00540270),
//! `DataDrainDemoMenu` (18, 0x00540730), `VoiceMenu` (19, 0x00540d20) and
//! `StrwinMenu` (20, 0x005412e0). Every page goes back to OPTION the same
//! way; what they ask of the system comes out as `Request`s, and their
//! drawing is in [`disp`]. The steps are in docs/engine/field-ui.md.

mod disp;

pub use disp::{controller_menu_disp, disp_slide_bar, on_off_menu_disp, screen_menu_disp, sound_menu_disp};

use crate::Request;
use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl, SE_BACK, SE_MOVE, SE_OK};
use crate::menus::system::extract_menu;

/// The save's fields the pages set (`ccSaveData`).
pub const SCREEN_X: usize = 0x841a;
pub const SCREEN_Y: usize = 0x841c;
pub const MAIN_VOL: usize = 0x841e;
pub const SE_VOL: usize = 0x8420;
pub const BGM_VOL: usize = 0x8422;
pub const OUTPUT: usize = 0x8424;
/// `drainDemo`: Data Drain's demo on (1) or off.
pub const DRAIN_DEMO: usize = 0x8427;
pub const VIBRATION: usize = 0x8428;
pub const CAM_TYPE: usize = 0x8429;
pub const VOICE: usize = 0x842c;
pub const STR_WIN_MODE: usize = 0x8430;

/// `ccStartThread(ccThControllerMenu, 34, 2048)`: the loading task's
/// priority and stack.
pub const LOAD_TASK: (i32, i32) = (34, 2048);

/// The OPTION pages' texts, read from the executable with gcmn over it.
#[derive(Clone, Debug, Default)]
pub struct OptionTexts {
    /// `ctrlMenuStr`: A-1, A-2, B-1, B-2 (16 glyphs a row).
    pub ctrl: Vec<u8>,
    /// `ctrlMenuStrBtn`, `ctrlMenuStrMov`, `ctrlMenuStrCam`: the picture's
    /// labels (settingKanji 0, 1, 2).
    pub ctrl_btn: Vec<u8>,
    pub ctrl_mov: Vec<u8>,
    pub ctrl_cam: Vec<u8>,
    /// `ctrlMenuHelp[4]`: two lines each.
    pub ctrl_help: [[Vec<u8>; 2]; 4],
    /// `soundMenuStr`: the four rows, then Mono and Stereo.
    pub sound: Vec<u8>,
    /// `dialogOnOff` (ON, OFF) and `dialogVoice` (English, Japanese).
    pub on_off: Vec<u8>,
    pub voice: Vec<u8>,
    /// `vibrationMenuInfo`, `datadrainDemoMenuInfo`, `voiceMenuInfo`,
    /// `strwinMenuInfo`: [on, off], two lines each.
    pub vibration_info: [[Vec<u8>; 2]; 2],
    pub drain_demo_info: [[Vec<u8>; 2]; 2],
    pub voice_info: [[Vec<u8>; 2]; 2],
    pub strwin_info: [[Vec<u8>; 2]; 2],
    /// `resetMenuInfo[0]` (whole) and `resetMenuInfo[1]`'s two lines.
    pub reset_info: Vec<u8>,
    pub reset_confirm: [Vec<u8>; 2],
    /// "X :", "Y :".
    pub screen_labels: [Vec<u8>; 2],
    /// The Controller picture's CCS file and texture.
    pub control_file: String,
    pub control_tex: String,
}

impl OptionTexts {
    /// The volume's: the desktop menu's words and lines
    /// (`piney_data::tables::dtmenu`, the same the field's OPTION pages
    /// draw) and the field's own (`fieldui`).
    pub fn of(volume: piney_data::volume::Volume) -> OptionTexts {
        use crate::tables::piece;
        use piney_data::tables::sjis::encode;
        use piney_desktop::dtmenu::slots;
        let d = piney_data::tables::dtmenu::of(volume);
        let f = piney_data::tables::fieldui::of(volume);
        let two = |l: &[&str]| [piece(l, 0), piece(l, 1)];
        let pair = |l: &[&[&str]]| [two(l[0]), two(l[1])];
        let help = d.help();
        OptionTexts {
            ctrl: slots(d.ctrl()),
            ctrl_btn: slots(d.ctrl_btn()),
            ctrl_mov: slots(d.ctrl_mov()),
            ctrl_cam: slots(d.ctrl_cam()),
            ctrl_help: [two(help[0]), two(help[1]), two(help[2]), two(help[3])],
            sound: slots(d.sound()),
            on_off: slots(d.on_off()),
            voice: slots(d.voice()),
            vibration_info: pair(d.vibration_info()),
            drain_demo_info: pair(f.drain_demo_info()),
            voice_info: pair(d.voice_info()),
            strwin_info: pair(d.strwin_info()),
            reset_info: encode(d.reset_info()),
            reset_confirm: two(d.reset_info2()),
            screen_labels: [encode(d.screen_x()), encode(d.screen_y())],
            control_file: f.control_file().to_string(),
            control_tex: f.control_tex().to_string(),
        }
    }
}

/// `ccMenuCtrl`'s members only the OPTION pages use.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptionState {
    /// +0x18c: the Sound page's main, BGM and SE volumes in 32 steps and
    /// the output (0 mono, 1 stereo).
    pub sound: [i32; 4],
}

/// Where the Controller page breathed inside its handler (its `Disp` and
/// `ccBreathThread(1)`): the rest runs at the start of the next frame,
/// before the task's own loop (so no `openReqNum` is looked at meanwhile).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// Waiting on the loading task's flag (+0x30's tscb +0x14). The port
    /// has the picture with the rest of the textures, so the task's load is
    /// done by its first run, after this breath: the flag is clear next
    /// frame, the shortest wait the game can have.
    Load,
    /// The close's first breath, then its second.
    Close1,
    Close2,
}

fn breathe(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Flow {
    crate::disp::disp(m, x);
    Flow::Breathed(Cont { woke: false, cursors: false, after: After::Option(r) })
}

/// The Controller page after one of its own breaths: `Some` when it
/// breathed again (its `Disp` done), else the task's `Disp` ends the frame.
pub fn resume(m: &mut MenuCtrl, r: Resume, x: &mut Ctx) -> Option<Cont> {
    match r {
        Resume::Load => {
            // The flag clear: ccDeleteThread, the picture's texture, the
            // window in with the help.
            m.note(|| "[\"delete_thread\"]".into());
            let (file, tex) = (x.texts.option.control_file.clone(), x.texts.option.control_tex.clone());
            m.note(|| format!("[\"get_ccs\",\"{file}\"]"));
            m.note(|| format!("[\"set_tex\",\"mask\",\"{file}\",\"{tex}\"]"));
            m.menu_status = 1;
            m.exception_disp = 1;
            controller_help(m, x);
            m.proccess += 1;
            None
        }
        Resume::Close1 => match breathe(m, x, Resume::Close2) {
            Flow::Breathed(c) => Some(c),
            Flow::Done => None,
        },
        Resume::Close2 => {
            m.note(|| "[\"file_delete\"]".into());
            back(m);
            None
        }
    }
}

/// The pages' decide / back test (the inlined `push & assignPAD*`): cancel
/// first, with sound 19, else OK with 18.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    Cancel,
    Ok,
}

fn key(x: &mut Ctx) -> Option<Key> {
    if x.pushed_cancel() {
        x.se(SE_BACK);
        Some(Key::Cancel)
    } else if x.pushed_ok() {
        x.se(SE_OK);
        Some(Key::Ok)
    } else {
        None
    }
}

fn list_index(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// Back to OPTION: `menuNext = prev`, the window out, `proccess` and
/// `waitCount` 0, `InitCursol(1)` on both windows.
fn back(m: &mut MenuCtrl) {
    let prev = m.list().prev;
    m.back_to_prev(prev);
}

/// `ccKanji::Extract(settingKanji[i], s)`.
pub fn extract_setting(m: &mut MenuCtrl, i: usize, s: &[u8]) {
    if m.setting_text[i] != s {
        m.note(|| format!("[\"extract\",\"set{i}\",\"{}\"]", String::from_utf8_lossy(s)));
    }
    m.setting_text[i] = s.to_vec();
}

impl MenuCtrl {
    /// `SelectXY(x0, x1, y0, y1, lim, dx, dy)` (0x00526730): down / up
    /// (repeat bits) move the list's `sy` by `dy` within y0..=y1, right /
    /// left its `sx` by `dx` within x0..=x1, sound 17 at each; past an end
    /// it stops there with `lim` bit 2 (y) or 1 (x), else wraps to the
    /// other end. Returns 1 for a y move, 2 for an x move.
    #[allow(clippy::too_many_arguments)]
    pub fn select_xy(&mut self, x0: i16, x1: i16, y0: i16, y1: i16, lim: i32, dx: i16, dy: i16, x: &mut Ctx) -> i32 {
        let repeat = x.pad.repeat.bits();
        let li = list_index(self);
        let mut r = 0;
        if repeat & 0x4000 != 0 {
            r = 1;
            x.se(SE_MOVE);
            let l = &mut self.lists[li];
            l.sy = l.sy.wrapping_add(dy);
            if l.sy > y1 {
                l.sy = if lim & 2 != 0 { y1 } else { y0 };
            }
        } else if repeat & 0x1000 != 0 {
            r = 1;
            x.se(SE_MOVE);
            let l = &mut self.lists[li];
            l.sy = l.sy.wrapping_sub(dy);
            if l.sy < y0 {
                l.sy = if lim & 2 != 0 { y0 } else { y1 };
            }
        }
        if repeat & 0x2000 != 0 {
            r |= 2;
            x.se(SE_MOVE);
            let l = &mut self.lists[li];
            l.sx = l.sx.wrapping_add(dx);
            if l.sx > x1 {
                l.sx = if lim & 1 != 0 { x1 } else { x0 };
            }
        } else if repeat & 0x8000 != 0 {
            r |= 2;
            x.se(SE_MOVE);
            let l = &mut self.lists[li];
            l.sx = l.sx.wrapping_sub(dx);
            if l.sx < x0 {
                l.sx = if lim & 1 != 0 { x0 } else { x1 };
            }
        }
        r
    }
}

// --- Controller (menu 13) -------------------------------------------------

/// `ControllerMenu` (0x0053d0e0).
pub fn controller_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let li = list_index(m);
    match m.proccess {
        0 => {
            m.exception_disp = 0;
            m.menu_status = 3;
            let rows = x.texts.option.ctrl.clone();
            extract_menu(m, rows);
            let t = i16::from(x.save.save.u8(CAM_TYPE) as i8);
            m.lists[li].select = t;
            m.lists[li].index = t;
            m.proccess += 1;
        }
        1 => {
            // ccStartThread(ccThControllerMenu), its flag set; the loop's
            // first Disp and breath.
            m.note(|| format!("[\"start_thread\",{},{}]", LOAD_TASK.0, LOAD_TASK.1));
            return breathe(m, x, Resume::Load);
        }
        2 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(Key::Cancel) => {
                    m.msg.close();
                    m.menu_status = 3;
                    m.proccess += 1;
                }
                Some(Key::Ok) => {
                    let sel = m.lists[li].select;
                    m.lists[li].index = sel;
                    x.save.save.set_u8(CAM_TYPE, sel as u8);
                    x.req.push(Request::CameraType(i32::from(sel)));
                    controller_help(m, x);
                }
                None => {}
            }
        }
        3 if m.menu_status == 0 => {
            m.exception_disp = 0;
            return breathe(m, x, Resume::Close1);
        }
        _ => {}
    }
    Flow::Done
}

/// `ChangeInfo(ctrlMenuHelp[index]'s two lines)`, the window moved to (39,
/// 444). (The game indexes the table unchecked; the port keeps to its four
/// rows.)
fn controller_help(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = m.list().index.clamp(0, 3) as usize;
    let [a, b] = x.texts.option.ctrl_help[i].clone();
    let names = x.save.names();
    m.msg.change_info([Some(&a), Some(&b), None, None], &names);
    m.msg.set_pos(39.0, 444.0);
}

// --- The on / off pages (menus 14, 18, 19, 20) ----------------------------

/// What an on / off page sets.
struct OnOff {
    /// The save's byte.
    field: usize,
    /// The rows (`dialogOnOff`, or `dialogVoice`).
    voice_rows: bool,
    /// Voiceover: row 0 is the value 1 (English); the others: any value
    /// but 0.
    on_is_one: bool,
    /// The information texts show both lines (Data Drain) or the first.
    two_lines: bool,
}

fn on_off(menu: i16) -> OnOff {
    match menu {
        14 => OnOff { field: VIBRATION, voice_rows: false, on_is_one: false, two_lines: false },
        18 => OnOff { field: DRAIN_DEMO, voice_rows: false, on_is_one: false, two_lines: true },
        19 => OnOff { field: VOICE, voice_rows: true, on_is_one: true, two_lines: false },
        _ => OnOff { field: STR_WIN_MODE, voice_rows: false, on_is_one: false, two_lines: false },
    }
}

/// Whether the save's value stands for row 0 ("on") and for row 1 ("off").
pub(crate) fn on_off_marks(menu: i16, save: &piney_desktop::SaveState) -> (bool, bool) {
    let o = on_off(menu);
    let v = save.save.u8(o.field) as i8;
    let on = if o.on_is_one { v == 1 } else { v != 0 };
    (on, v == 0)
}

fn on_off_info(menu: i16, texts: &crate::tables::Texts) -> &[[Vec<u8>; 2]; 2] {
    let o = &texts.option;
    match menu {
        14 => &o.vibration_info,
        18 => &o.drain_demo_info,
        19 => &o.voice_info,
        _ => &o.strwin_info,
    }
}

/// `VibrationMenu` (0x0053ea80), `DataDrainDemoMenu` (0x00540730),
/// `VoiceMenu` (0x00540d20), `StrwinMenu` (0x005412e0).
fn on_off_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let li = list_index(m);
    let o = on_off(m.menu);
    if m.proccess == 0 {
        let rows = if o.voice_rows { x.texts.option.voice.clone() } else { x.texts.option.on_off.clone() };
        extract_menu(m, rows);
        let (on, _) = on_off_marks(m.menu, x.save);
        let l = &mut m.lists[li];
        l.select = if on { 0 } else { 1 };
        l.x = 6;
        l.y = 2;
        let info = on_off_info(m.menu, x.texts)[if on { 0 } else { 1 }].clone();
        let names = x.save.names();
        let second = if o.two_lines { Some(&info[1][..]) } else { None };
        m.msg.open_info([Some(&info[0]), second, None, None], &names);
        m.note(|| "[\"open_info\"]".into());
        m.exception_disp = 1;
        m.proccess += 1;
    }
    if m.proccess != 1 {
        return Flow::Done;
    }
    m.select(0, 0, false, x);
    match key(x) {
        Some(Key::Cancel) => {
            m.msg.close();
            back(m);
        }
        Some(Key::Ok) => {
            let row = usize::from(m.lists[li].select != 0);
            let v = if row == 0 { 1 } else { 0 };
            x.save.save.set_u8(o.field, v);
            if m.menu == 14 {
                x.req.push(Request::Vibration { on: row == 0 });
            }
            let info = on_off_info(m.menu, x.texts)[row].clone();
            let names = x.save.names();
            let second = if o.two_lines { Some(&info[1][..]) } else { None };
            m.msg.change_info([Some(&info[0]), second, None, None], &names);
            m.cursor.init(true);
            m.cursor_pr.init(true);
        }
        None => {}
    }
    Flow::Done
}

/// `VibrationMenu` (0x0053ea80): `vibration` (+0x8428), and
/// `ccPad::actuaterSw` with a buzz switching on ([`Request::Vibration`]).
pub fn vibration_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    on_off_menu(m, x)
}

/// `DataDrainDemoMenu` (0x00540730): `drainDemo` (+0x8427); its texts are
/// two lines.
pub fn data_drain_demo_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    on_off_menu(m, x)
}

/// `VoiceMenu` (0x00540d20): `voice` (+0x842c), 1 English, 0 Japanese.
pub fn voice_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    on_off_menu(m, x)
}

/// `StrwinMenu` (0x005412e0): `strWinMode` (+0x8430), the movies' text
/// window.
pub fn strwin_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    on_off_menu(m, x)
}

// --- Adjust Screen (menu 15) ----------------------------------------------

/// `ScreenMenu` (0x0053f060).
pub fn screen_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let li = list_index(m);
    match m.proccess {
        0 => {
            m.bg_status = 1;
            m.lists[li].sx = x.save.save.i16(SCREEN_X).wrapping_add(48);
            m.lists[li].sy = x.save.save.i16(SCREEN_Y).wrapping_add(16);
            m.exception_disp = 1;
            m.wait_count = 0;
            m.proccess += 1;
        }
        1 => {
            m.wait_count = m.wait_count.wrapping_add(1) % 30;
            m.select_xy(0, 96, 0, 32, 3, 3, 1, x);
            let l = &m.lists[li];
            let (ox, oy) = (l.sx.wrapping_sub(48), l.sy.wrapping_sub(16));
            x.req.push(Request::DisplayOffset { x: i32::from(ox), y: i32::from(oy) });
            x.save.save.set_i16(SCREEN_X, ox);
            x.save.save.set_i16(SCREEN_Y, oy);
            if x.pushed_cancel() {
                x.se(SE_BACK);
                back(m);
            }
        }
        _ => {}
    }
    Flow::Done
}

// --- Sound (menu 16) ------------------------------------------------------

/// `v * 32 / 256` as the game shifts it (toward zero).
fn to_steps(v: i16) -> i32 {
    (i32::from(v) << 5) / 256
}

/// `SoundMenu` (0x0053f800).
pub fn sound_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let li = list_index(m);
    match m.proccess {
        0 => {
            m.exception_disp = 1;
            m.lists[li].y = 4;
            let s = &x.save.save;
            m.option.sound = [
                to_steps(s.i16(MAIN_VOL)),
                to_steps(s.i16(BGM_VOL)),
                to_steps(s.i16(SE_VOL)),
                i32::from(s.i16(OUTPUT)),
            ];
            m.proccess += 1;
        }
        1 => {
            m.select(0, 0, false, x);
            let mut moved = false;
            let repeat = x.pad.repeat.bits();
            let sel = m.lists[li].select.clamp(0, 3) as usize;
            let t = &mut m.option.sound;
            if repeat & 0x8000 != 0 {
                if t[sel] > 0 {
                    t[sel] = (t[sel] - 1).max(0);
                    x.se(SE_MOVE);
                    moved = true;
                } else {
                    t[sel] = 0;
                }
            } else if repeat & 0x2000 != 0 {
                let max = match m.lists[li].select {
                    0..=2 => 32,
                    3 => 1,
                    _ => 0,
                };
                if t[sel] < max {
                    t[sel] += 1;
                    x.se(SE_MOVE);
                    moved = true;
                } else {
                    t[sel] = max;
                }
            }
            if x.pushed_cancel() {
                x.se(SE_BACK);
                back(m);
            }
            if moved {
                let t = m.option.sound;
                // (t << 8) / 32, toward zero.
                let vol = |v: i32| ((v << 8) / 32) as i16;
                let s = &mut x.save.save;
                s.set_i16(MAIN_VOL, vol(t[0]));
                s.set_i16(BGM_VOL, vol(t[1]));
                s.set_i16(SE_VOL, vol(t[2]));
                s.set_i16(OUTPUT, t[3] as i16);
                x.req.push(Request::SoundEnv {
                    main: i32::from(s.i16(MAIN_VOL)),
                    bgm: i32::from(s.i16(BGM_VOL)),
                    se: i32::from(s.i16(SE_VOL)),
                    output: i32::from(s.i16(OUTPUT)),
                });
            }
        }
        _ => {}
    }
    Flow::Done
}

// --- Title Screen (menu 17) -----------------------------------------------

/// `ResetMenu` (0x00540270): the list's OK / Cancel rows (disp 11) under
/// two questions.
pub fn reset_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let li = list_index(m);
    match m.proccess {
        0 => {
            m.bg_status = 1;
            let rows = x.texts.dialog_default.clone();
            extract_menu(m, rows);
            m.lists[li].select = 1;
            m.wait_count = 0;
            let info = x.texts.option.reset_info.clone();
            let names = x.save.names();
            m.msg.open_info([Some(&info), None, None, None], &names);
            m.note(|| "[\"open_info\"]".into());
            m.proccess += 1;
        }
        1 | 3 => {
            m.select(0, 0, false, x);
            match key(x) {
                Some(Key::Ok) => {
                    m.msg.close();
                    if m.lists[li].select == 1 {
                        back(m);
                    } else if m.proccess == 1 {
                        m.menu_status = 3;
                        m.proccess += 1;
                    } else {
                        x.req.push(Request::ChangeMode { num: 1, sf: 7 });
                        m.proccess += 1;
                    }
                }
                Some(Key::Cancel) => {
                    m.msg.close();
                    back(m);
                }
                None => {}
            }
        }
        2 if m.menu_status == 0 => {
            m.menu_status = 1;
            let [a, b] = x.texts.option.reset_confirm.clone();
            let names = x.save.names();
            m.msg.open_info([Some(&a), Some(&b), None, None], &names);
            m.note(|| "[\"open_info\"]".into());
            m.lists[li].select = 1;
            m.wait_count = 0;
            m.proccess += 1;
        }
        _ => {}
    }
    Flow::Done
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn volume_steps_round_toward_zero() {
        assert_eq!(to_steps(256), 32);
        assert_eq!(to_steps(255), 31);
        assert_eq!(to_steps(8), 1);
        assert_eq!(to_steps(7), 0);
        assert_eq!(to_steps(-9), -1);
        assert_eq!(((31 << 8) / 32) as i16, 248);
    }

    use std::sync::Arc;

    use piney_input::{Buttons, Pad};

    use crate::{CharInfo, FieldUi, World};

    /// The field UI over Infection's disc, Kite alone; `None` when the disc
    /// image is not extracted (PINEY_ISO points elsewhere).
    pub(crate) fn field_ui() -> Option<(FieldUi, World, piney_desktop::SaveState)> {
        let path = std::env::var_os("PINEY_ISO").map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso")
        });
        if !path.exists() {
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(&path).unwrap();
        let archive = Arc::new(piney_data::archive::Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let ui = FieldUi::new(&mut iso, archive).unwrap();
        let mut world = World { boss_entry: -1, party_id: [0, -1, -1], ..World::default() };
        world.game.status = 5;
        world.party[0] =
            Some(CharInfo { handle: 0x100, types: 1, name: b"Kite".to_vec(), attribute: -1, ..CharInfo::default() });
        Some((ui, world, piney_desktop::SaveState::fresh()))
    }

    #[test]
    fn texts_and_lists_from_the_disc() {
        let Some((ui, _, _)) = field_ui() else { return };
        let t = ui.texts();
        assert_eq!(t.lists[12].items, [13, 14, 15, 16, 18, 19, 20, 17]);
        assert_eq!((t.lists[13].disp, t.lists[13].x, t.lists[13].y), (4, 6, 4));
        assert_eq!((t.lists[17].disp, t.lists[17].x, t.lists[17].y), (11, 6, 2));
        let o = &t.option;
        assert!(o.ctrl.starts_with(b"A-1 "));
        assert!(o.sound.starts_with(b"Main Volume"));
        assert_eq!(o.screen_labels, [b"X :".to_vec(), b"Y :".to_vec()]);
        assert_eq!((o.control_file.as_str(), o.control_tex.as_str()), ("xcontrol", "TEX_xcontrol"));
        assert!(o.drain_demo_info[1][1].starts_with(b"drain"));
    }

    /// Each pad, then ten idle frames: the requests they made.
    fn run(
        ui: &mut FieldUi,
        world: &World,
        save: &mut piney_desktop::SaveState,
        count: &mut u32,
        pads: &[(Buttons, Buttons)],
    ) -> Vec<Request> {
        let mut out = Vec::new();
        for &(push, repeat) in pads {
            for k in 0..11 {
                *count += 1;
                let pad = if k == 0 { Pad { push, repeat, ..Pad::default() } } else { Pad::default() };
                ui.step(&pad, world, save, *count);
                out.extend(ui.take_requests());
            }
        }
        out
    }

    #[test]
    fn sound_and_vibration_through_option() {
        let Some((mut ui, world, mut save)) = field_ui() else { return };
        save.save.set_i16(MAIN_VOL, 256);
        save.save.set_u8(VIBRATION, 1);
        let mut count = 0;
        let (n, x) = (Buttons::NONE, Buttons::CROSS);
        run(&mut ui, &world, &mut save, &mut count, &[(n, n)]);
        ui.open_menu(12);
        // Sound: the main volume one step down.
        let pads = [(n, n), (n, Buttons::DOWN), (n, Buttons::DOWN), (n, Buttons::DOWN), (x, n)];
        run(&mut ui, &world, &mut save, &mut count, &pads);
        assert_eq!(ui.menu_type(), 16);
        let r = run(&mut ui, &world, &mut save, &mut count, &[(n, Buttons::LEFT), (Buttons::CIRCLE, n)]);
        assert_eq!(save.save.i16(MAIN_VOL), 248);
        assert!(r.iter().any(|q| matches!(q, Request::SoundEnv { main: 248, .. })));
        // Vibrate: off.
        let pads = [(n, Buttons::UP), (n, Buttons::UP), (x, n), (n, Buttons::DOWN), (x, n)];
        let r = run(&mut ui, &world, &mut save, &mut count, &pads);
        assert_eq!(ui.menu_type(), 14);
        assert_eq!(save.save.u8(VIBRATION), 0);
        assert!(r.contains(&Request::Vibration { on: false }));
    }
}
