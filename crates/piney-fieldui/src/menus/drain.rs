//! Data Drain (gcmn menu.cpp): `DataDrainMenu` (66, 0x00532ae0), the menu
//! TargetMenu opens when Kite drains (`itemNum` +0x238 the skill: 2 Data
//! Drain, 3 Drain Arc, 4 2128 Drain, 5 Drain Heart; the target
//! `cmndTarget`). The rules are `piney_battle::drain`'s, which the runtime
//! runs on its world ([`Request::DataDrain`], [`Request::DrainSideEffect`])
//! and answers with [`crate::FieldUi::drain_drops`] and
//! [`crate::FieldUi::drain_side_effect`]. `waitCount` is only zeroed at 0's
//! start. The steps are in docs/engine/field-ui.md (Data Drain).

use piney_event::ScriptSave;

use crate::ctrl::{After, Cont, Ctx, Flow, MenuCtrl};
use crate::menus::personal::{check, check_fade, delete_fade};
use crate::{Noise, Request};

/// `saveData.drainDemo`, the infection.
pub const DRAIN_DEMO: usize = 0x8427;
pub const EROSION: usize = 0x676e;

/// The texts Data Drain shows.
#[derive(Clone, Debug, Default)]
pub struct DrainTexts {
    /// `dataDrainWarn`'s pieces (34).
    pub warn: Vec<Vec<u8>>,
    /// `dataDrainEvolutionStr`: each page's two lines, by pointer (44).
    pub evolution: Vec<[Vec<u8>; 2]>,
}

impl DrainTexts {
    /// The volume's `dataDrainWarn` (34 pieces) and
    /// `dataDrainEvolutionStr` (44, two lines each).
    pub fn of(volume: piney_data::volume::Volume) -> DrainTexts {
        let t = piney_data::tables::fieldui::of(volume);
        DrainTexts {
            warn: t.drain_warn().iter().map(|l| piney_data::tables::sjis::encode(l)).collect(),
            evolution: t
                .drain_evolution()
                .iter()
                .map(|l| [crate::tables::piece(l, 0), crate::tables::piece(l, 1)])
                .collect(),
        }
    }

    fn warn(&self, k: usize) -> Vec<u8> {
        self.warn.get(k).cloned().unwrap_or_default()
    }
}

/// What step 10's side effect came to (the runtime's answer).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Side {
    /// 0-30 (`menuList[66]` +0x10).
    pub id: i32,
    /// Kite loses a level at 11's frame 30 (+0x0c).
    pub level_down: bool,
    /// Effect 29's item lost (`trapNum` +0x23c, `category << 16 | id`),
    /// -1 none.
    pub lost: i32,
}

/// Data Drain's state in `ccMenuCtrl` and its handler's locals.
#[derive(Clone, Debug, Default)]
pub struct DrainState {
    /// Step 0's fade (a local of the handler).
    pub fade: i32,
    /// The side effect, once the runtime answered.
    pub side: Option<Side>,
    /// Step 20's pages: the stage's first pointer, and the next page.
    pub pages: Option<(usize, usize)>,
    /// Step 0's movie has ended ([`crate::FieldUi::drain_movie_done`]).
    pub movie_done: bool,
}

/// Where `DataDrainMenu`'s own frames resume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// Step 0's fade: after its `Disp; Breath`, `CheckFade`.
    Fade,
    /// `ccBreathThread(2)` after it: frames left.
    Black(u8),
    /// The movie's `ccBreathThread(2)` after its start: frames left.
    Movie(u8),
    /// The movie playing: `ccBreathThread(1)` until it has ended.
    MovieWait,
    /// The three frames after the rules.
    Settle(u8),
    /// A step's two frames before its window (2 and 11): `Disp; Breath`
    /// done so far.
    Pause { step: i16, done: u8 },
    /// Step 20's pages: `Disp; Breath` until `Check`, then the next page.
    Pages,
}

fn breathe(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Flow {
    crate::disp::disp(m, x);
    wait(r)
}

fn wait(r: Resume) -> Flow {
    Flow::Breathed(Cont { woke: false, cursors: false, after: After::Drain(r) })
}

fn noise(m: &mut MenuCtrl, x: &mut Ctx, n: Noise) {
    crate::disp::noiz(m, x, n);
}

/// The world asleep and its layers kept (the windows of 2 and 11):
/// `bgStatus` 1, `ccSleepAllThread`, `still` 1 and the flips off, each
/// layer's `flipFlag` down one.
fn freeze(m: &mut MenuCtrl, x: &mut Ctx) {
    m.bg_status = 1;
    x.req.push(Request::SleepAll);
    m.still = 1;
    x.req.push(Request::Still(true));
    x.req.push(Request::KeepLayers);
}

fn open_info(m: &mut MenuCtrl, x: &Ctx, l0: &[u8], l1: Option<&[u8]>) {
    let names = x.save.names();
    m.msg.open_info([Some(l0), l1, None, None], &names);
}

/// The infection over two, as `sra 1` rounds it.
fn half_erosion(x: &Ctx) -> i32 {
    i32::from(x.save.save.i16(EROSION)) / 2
}

/// Step 1's roll (also run at the end of step 0).
fn roll(m: &mut MenuCtrl, x: &mut Ctx) {
    let half = half_erosion(x);
    let r = m.rng.rand() % 100;
    if half < r {
        m.proccess += 1;
    } else {
        m.proccess = 10;
    }
}

/// `DataDrainMenu` (66).
pub fn data_drain_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    match m.proccess {
        0 => {
            m.data_drain = DrainState { fade: m.menu_fade.entry_fade(25, 0, 0x8000_0000), ..DrainState::default() };
            m.wait_count = 0;
            fade_body(m, x)
        }
        1 => {
            roll(m, x);
            Flow::Done
        }
        2 => {
            m.wait_count += 1;
            let wc = m.wait_count;
            if wc < 46 {
                if wc % 8 == 0 {
                    let r = m.rng.rand() % 4 + 3;
                    noise(m, x, Noise::Rn(r));
                    noise(m, x, Noise::Bs(r));
                } else {
                    noise(m, x, Noise::Br(10));
                }
                return Flow::Done;
            }
            noise(m, x, Noise::Set(0, 0, 0));
            breathe(m, x, Resume::Pause { step: 2, done: 1 })
        }
        3 => {
            if check(m, x) {
                m.msg.close();
                m.proccess = 20;
            }
            Flow::Done
        }
        10 => {
            // ccMenuCtrl's side effect: the runtime's rules on the party.
            x.req.push(Request::DrainSideEffect);
            noise(m, x, Noise::Set(8, 8, 8));
            m.proccess += 1;
            Flow::Done
        }
        11 => {
            let old = m.wait_count;
            m.wait_count += 1;
            if old >= 61 {
                noise(m, x, Noise::Set(0, 0, 0));
                return breathe(m, x, Resume::Pause { step: 11, done: 1 });
            }
            let wc = m.wait_count;
            if wc == 30 {
                noise(m, x, Noise::Set(3, 3, 3));
                if m.data_drain.side.is_some_and(|s| s.level_down) {
                    x.req.push(Request::DrainLevelDown);
                }
            } else if wc == 10 || wc == 18 || wc == 45 {
                let r = m.rng.rand() % 5 + 5;
                noise(m, x, Noise::Set(r, r, r));
            }
            Flow::Done
        }
        12 => {
            if check(m, x) {
                m.msg.close();
                if m.data_drain.side.is_some_and(|s| s.id == 30) {
                    x.req.push(Request::GameOver);
                }
                m.proccess = 20;
            }
            Flow::Done
        }
        20 => evolve(m, x),
        _ => Flow::Done,
    }
}

/// Step 0's fade: noise, the count, `Disp; Breath`.
fn fade_body(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if m.wait_count % 6 == 0 {
        let r = m.rng.rand() % 3 + 3;
        noise(m, x, Noise::Rn(r));
        noise(m, x, Noise::Bs(r));
    }
    noise(m, x, Noise::Br(10));
    m.wait_count += 1;
    breathe(m, x, Resume::Fade)
}

/// After one of `DataDrainMenu`'s own breaths: `Some` when it breathed
/// again, else the task's `Disp` ends the frame.
pub fn resume(m: &mut MenuCtrl, r: Resume, x: &mut Ctx) -> Option<Cont> {
    match run(m, x, r) {
        Flow::Breathed(c) => Some(c),
        Flow::Done => None,
    }
}

/// The drain movies (streams): an enemy's by its `ObjectSize` (4 small,
/// 3 middle, 1 large), any other target's by its `base->id` (0-7, then any
/// other), and the one with no target.
struct Movies {
    small: i32,
    middle: i32,
    large: i32,
    by_id: [i32; 8],
    other: i32,
}

/// Infection's; from Mutation on the enemies' and the fallbacks move to
/// 115-117 and ids 5-7 get streams of their own (MUT 0x005515f8 -
/// 0x005516ec, the same in Outbreak and Quarantine).
const INF_MOVIES: Movies =
    Movies { small: 109, middle: 110, large: 111, by_id: [18, 37, 44, 67, 69, 111, 111, 100], other: 111 };
const LATER_MOVIES: Movies =
    Movies { small: 115, middle: 116, large: 117, by_id: [18, 37, 44, 67, 69, 96, 100, 106], other: 117 };

/// Step 0's movie (0x00532ce4 - 0x00532e8c): an enemy's by its size, only
/// with `drainDemo` on; any other target's by its `base->id` whatever
/// `drainDemo` says (0 is Skeith's 18); the small enemy's when `cmndTarget`
/// is gone ([`Movies`]).
fn movie(x: &Ctx) -> Option<i32> {
    let m = if x.texts.volume == piney_data::volume::Volume::Inf { &INF_MOVIES } else { &LATER_MOVIES };
    let Some(t) = x.target.as_ref() else { return Some(m.small) };
    if t.is(0x60) {
        if x.save.save.u8(DRAIN_DEMO) == 0 {
            return None;
        }
        return match t.object_size {
            4 => Some(m.small),
            3 => Some(m.middle),
            1 => Some(m.large),
            // No enemy row has another size; the game would play the
            // stream a register last held.
            _ => None,
        };
    }
    Some(usize::try_from(t.id).ok().and_then(|k| m.by_id.get(k)).copied().unwrap_or(m.other))
}

/// The movie's loop (0x00532f60 - 0x00532fe4): `ccBreathThread(1)` until
/// the stream's task has ended, then the rules in the same frame.
fn movie_wait(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    if m.data_drain.movie_done {
        return rules(m, x);
    }
    wait(Resume::MovieWait)
}

/// Step 0 after the movie (0x00533010 on): the rules on the world (a
/// boss's `EntryAffect(21)`, the infection, the drops, `ccDeleteCmnd`),
/// answered with the drops; the targets cleared, the world back, the
/// flash, the tasks woken, three frames.
fn rules(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let target = x.target.as_ref().map_or(0, |t| t.handle);
    x.req.push(Request::DataDrain { target, sid: m.item_num });
    // cmndTarget and cmndTargetPrev cleared as they are.
    x.target = None;
    x.target_prev = None;
    x.req.push(Request::TargetsCleared);
    x.req.push(Request::WorldHidden(false));
    m.menu_fade.entry_flash(20, 0x70c0_a020);
    x.req.push(Request::WakeAll);
    breathe(m, x, Resume::Settle(1))
}

fn run(m: &mut MenuCtrl, x: &mut Ctx, r: Resume) -> Flow {
    match r {
        Resume::Fade => {
            if check_fade(m, m.data_drain.fade) {
                return fade_body(m, x);
            }
            if m.still == 1 {
                m.still = 0;
                x.req.push(Request::Still(false));
            }
            x.req.push(Request::WorldHidden(true));
            wait(Resume::Black(1))
        }
        Resume::Black(1) => wait(Resume::Black(0)),
        Resume::Black(_) => {
            delete_fade(m, m.data_drain.fade);
            // A boss takes EntryAffect(21) from Kite before the movie.
            if let Some(t) = x.target.as_ref().filter(|t| t.is(0x80)) {
                x.req.push(Request::Affect { target: t.handle, kind: 21 });
            }
            match movie(x) {
                Some(num) => {
                    x.req.push(Request::DrainMovie(num));
                    m.data_drain.movie_done = false;
                    wait(Resume::Movie(1))
                }
                None => rules(m, x),
            }
        }
        Resume::Movie(1) => wait(Resume::Movie(0)),
        Resume::Movie(_) => {
            // The stream is the runtime's and has started: an enemy is
            // drawn into it (ccThDrainEnemy).
            if let Some(t) = x.target.as_ref().filter(|t| t.is(0x60)) {
                x.req.push(Request::DrainEnemy(t.handle));
            }
            movie_wait(m, x)
        }
        Resume::MovieWait => movie_wait(m, x),
        Resume::Settle(n) if n < 3 => breathe(m, x, Resume::Settle(n + 1)),
        Resume::Settle(_) => {
            roll(m, x);
            Flow::Done
        }
        Resume::Pause { step, done: 1 } => breathe(m, x, Resume::Pause { step, done: 2 }),
        Resume::Pause { step, .. } => {
            let t = &x.texts.drain;
            if step == 2 {
                let l0 = t.warn(0);
                open_info(m, x, &l0, None);
            } else {
                let side = m.data_drain.side.unwrap_or(Side { id: 0, level_down: false, lost: -1 });
                let title = t.warn(1);
                if side.id != 29 {
                    let line = t.warn((side.id + 2).clamp(0, 33) as usize);
                    open_info(m, x, &title, Some(&line));
                } else if side.lost >= 0 {
                    let mut line = x.texts.items.item_name(side.lost >> 16, side.lost & 0xffff);
                    line.extend_from_slice(&t.warn(31));
                    open_info(m, x, &title, Some(&line));
                    x.se(79);
                    m.menu_fade.entry_flash(6, 0x8000_ffff);
                } else {
                    let line = t.warn(33);
                    open_info(m, x, &title, Some(&line));
                }
            }
            freeze(m, x);
            m.proccess += 1;
            Flow::Done
        }
        Resume::Pages => {
            if !check(m, x) {
                return breathe(m, x, Resume::Pages);
            }
            let Some((first, next)) = m.data_drain.pages else { return Flow::Done };
            change_info(m, x, first + next);
            if next + 1 < 5 {
                m.data_drain.pages = Some((first, next + 1));
                return breathe(m, x, Resume::Pages);
            }
            m.msg.close();
            m.data_drain.pages = None;
            to_items(m, x);
            Flow::Done
        }
    }
}

fn change_info(m: &mut MenuCtrl, x: &Ctx, k: usize) {
    let names = x.save.names();
    let [l0, l1] = x.texts.drain.evolution.get(k).cloned().unwrap_or_default();
    m.msg.change_info([Some(&l0), Some(&l1), None, None], &names);
}

/// Step 20: the drain count and the bracelet's growth, its pages, then the
/// drops through 67.
fn evolve(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let (stage, item) = piney_battle::drain::evolution(&mut x.save.save, x.texts.volume.number());
    if let Some(code) = item {
        x.save.save.add_item(0, (code >> 16) as i16, (code & 0xffff) as i16, 1);
    }
    let Some(stage) = stage else {
        to_items(m, x);
        return Flow::Done;
    };
    let first = 4 * (stage.max(1) as usize - 1);
    change_info(m, x, first);
    x.se(74);
    m.data_drain.pages = Some((first, 1));
    breathe(m, x, Resume::Pages)
}

/// `cmndTargetFix = 0; ChangeMenu(67)`.
fn to_items(m: &mut MenuCtrl, x: &mut Ctx) {
    x.req.push(Request::TargetFix(false));
    m.change_menu_to(67);
}
