//! Kite's acts and animation: `ccPlayer::AnimCtrl` (gcmn 0x005993c0) for a
//! player with no skill, target, enemy or condition - standing, walking,
//! running, the idle fidget and the arrival fade-in. Every act is one
//! animation of `playerAnimTbl` (gcmn 0x006f0560), cut to frame 0 when the
//! act changes (nothing blends). Walking and running play faster with the
//! lean: `frameSpd = 256 (k speedRate speedValue)`, k 1.1 and 1.375.

use piney_data::anim::{Animation, Ticks};

use crate::ee::{self, F, div, from_int, mul};

/// `playerAnimTbl`: the animation of each act (26 entries of 21 bytes).
pub const PLAYER_ANIM_TBL: [&str; 26] = [
    "ANM_ctu1nut0",
    "ANM_ctu1nut1",
    "ANM_ctu1nut2",
    "ANM_ctu1nut3",
    "ANM_ctu1nut4",
    "ANM_ctu1run0",
    "ANM_ctu1wal0",
    "ANM_ctu1dmg0",
    "ANM_ctu1dmg1",
    "ANM_ctu1dwn0",
    "ANM_ctu1dwn0",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1atc0",
    "ANM_ctu1atc1",
    "ANM_ctu1mag0",
    "ANM_ctu1mag1",
    "ANM_ctu1ski2",
    "ANM_ctu1ski3",
    "ANM_ctu1ski4",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1hac4b",
    "ANM_ctu1atc0",
];

/// The acts the town reaches.
pub mod act {
    /// Field idle (`nut0`).
    pub const IDLE_FIELD: i16 = 0;
    pub const FIDGET_FIELD: i16 = 1;
    /// Town idle (`nut2`, looping).
    pub const IDLE: i16 = 2;
    /// The fidget after 451 frames standing (`nut3`, once), then `nut4`.
    pub const FIDGET: i16 = 3;
    pub const FIDGET_LOOP: i16 = 4;
    pub const RUN: i16 = 5;
    pub const WALK: i16 = 6;
    /// Leaving through a gate (`ccSpcChar::TransferOut`): `nut2` with the
    /// fade-out, then out of sight.
    pub const LEAVE: i16 = 12;
    /// Arriving at a town or gate: `nut2` with the fade-in.
    pub const ARRIVE: i16 = 13;
    /// Out of sight (`SetBootStatus` bit 1, the departure's end).
    pub const HIDDEN: i16 = 14;
}

/// Frames standing still before the fidget: `reactCnt` reaching 451.
pub const FIDGET_AFTER: i16 = 451;

/// 1.375 and 1.1: the run and walk animations' speed per `speedRate`.
const RUN_SPD: F = 0x3fb0_0000;
const WALK_SPD: F = 0x3f8c_cccd;
/// 256.0.
const K256: F = 0x4380_0000;

/// What `AnimCtrl` reads of the rest of the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct ActInput {
    /// `game->area`: 0 in a town.
    pub area: i32,
    /// `checkCameraType()`.
    pub cam_type: i32,
    /// `worldman->warpFlag` (+0x168): arriving by warp (shorter fade).
    pub warp: bool,
    /// `game->inBattle`.
    pub in_battle: i32,
    /// The AI in manual mode (`ai->manualSW`): no fidget, and act 0 turns
    /// into act 1.
    pub manual: bool,
}

/// Something `AnimCtrl` asks for beside the acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActEvent {
    /// `effTransfer(this)`: the arrival effect, at `actCnt` 30.
    Transfer,
    /// `effWarpTransfer(this)`.
    WarpTransfer,
    /// The arrival ended: `ccEntryCmnd(this)` (not under manual control)
    /// and `bodyHit.HitEnable()`.
    Arrived,
    /// The departure (act 12) began: `effTransfer(this)` with
    /// `bodyHit.HitDisable()`.
    Leaving,
    /// `ccPlayer::CheckNote` (gcmn 0x0059c300) on a footstep note (events
    /// 1 and 2): `ccSeSetParamSPC(param, this)`, the step's sound.
    Step(u32),
}

/// The act state (`ccSpcChar` members) and the `ccAnm` time.
#[derive(Clone, Debug, PartialEq)]
pub struct Acts {
    /// +0xee `actNum`, +0xf0 `actNumOld`.
    pub act: i16,
    pub act_old: i16,
    /// +0xf4 `anmFlag`: the last `_AnimateForward`'s result.
    pub anm_flag: i16,
    /// +0xf8 `actCnt`, +0xfa `reactCnt`, +0x100 `transferLag`.
    pub act_cnt: i16,
    pub react_cnt: i16,
    pub transfer_lag: i16,
    /// +0x124 `walkRunCnt`.
    pub walk_run_cnt: i32,
    /// +0xe0 bits: 0 `pauseSW`, 2 `restraintSW`, 4 `stopFlag`, 6 `ghostFlag`.
    pub pause: bool,
    pub restraint: bool,
    pub stop_flag: bool,
    pub ghost: bool,
    /// +0xfe `attack`.
    pub attack: i16,
    /// +0x110 `cloak`: the arrival's fade.
    pub cloak: F,
    /// The `ccAnm`: which animation of the table plays (by act), its time
    /// (`frameNow * 256 + frameCnt`), `frameSpd`, and the time the pose was
    /// last evaluated at.
    pub anim: usize,
    pub time: Ticks,
    pub frame_spd: u16,
    pub posed: Ticks,
}

impl Acts {
    /// As `ccPlayer::ccPlayer` leaves them in a town: act 13 with
    /// `transferLag` 1, `restraintSW` set (the act is not 2), `stopFlag`
    /// set, `cloak` 1, the animation set at frame 0.
    pub fn arriving() -> Self {
        Acts {
            act: act::ARRIVE,
            act_old: 0,
            anm_flag: 0,
            act_cnt: 0,
            react_cnt: 0,
            transfer_lag: 1,
            walk_run_cnt: 0,
            pause: false,
            restraint: true,
            stop_flag: true,
            ghost: false,
            attack: 0,
            cloak: ee::ONE,
            anim: act::ARRIVE as usize,
            time: 0,
            frame_spd: 256,
            posed: 0,
        }
    }
}

/// `ccPlayer::AnimCtrl` for the town's acts. `move_flag`/`run_flag` are the
/// player's (+0xe0 bits 3 and 5; the pause branch clears them), `speed_rate`
/// and `speed_value` its +0x108 and +0x28. `anims[act]` is the act's
/// animation; `rand` is newlib's `rand()`.
#[allow(clippy::too_many_arguments)]
pub fn anim_ctrl(
    a: &mut Acts,
    move_flag: &mut bool,
    run_flag: &mut bool,
    speed_rate: F,
    speed_value: F,
    input: &ActInput,
    anims: &[&Animation],
    rand: &mut dyn FnMut() -> i32,
    events: &mut Vec<ActEvent>,
) {
    // Paused while moving: back to standing.
    if a.pause && (a.act == act::WALK || a.act == act::RUN) {
        a.act = if input.area == 0 { act::IDLE } else { act::IDLE_FIELD };
        a.restraint = false;
        a.attack = 0;
        a.stop_flag = true;
        *move_flag = false;
        *run_flag = false;
    }
    // The fidget: 451 frames standing, never under manual control.
    if input.cam_type != 1 && !input.manual && a.act < 4 && input.in_battle != 1 {
        a.react_cnt = a.react_cnt.wrapping_add(1);
        if a.react_cnt >= FIDGET_AFTER {
            a.react_cnt = (rand() % 60) as i16;
            match a.act {
                act::IDLE => a.act = act::FIDGET,
                act::IDLE_FIELD => a.act = act::FIDGET_FIELD,
                _ => {}
            }
        }
    } else {
        a.react_cnt = 0;
    }
    // The last animation ended (jump table @1749, gcmn 0x006f07e0).
    if a.anm_flag != 0 {
        match a.act {
            0 | 2 | 5 | 9..=11 => {}
            1 => {
                a.act = act::IDLE;
                a.restraint = false;
            }
            3 => {
                a.act = act::FIDGET_LOOP;
                a.restraint = false;
            }
            12 => {
                a.act = 14;
                a.restraint = true;
                *move_flag = false;
            }
            13 => {
                a.act = act::IDLE;
                a.act_old = act::IDLE;
                a.restraint = false;
                events.push(ActEvent::Arrived);
            }
            15..=21 => {
                if *move_flag {
                    a.act = act::RUN;
                }
                if a.stop_flag {
                    a.act = act::IDLE_FIELD;
                }
                a.restraint = false;
            }
            24 => a.act = 23,
            _ => {
                a.act = act::IDLE_FIELD;
                a.restraint = false;
            }
        }
    }
    // Starting and stopping.
    if a.stop_flag && *move_flag {
        a.stop_flag = false;
        a.act = act::WALK;
    } else if !a.stop_flag && !*move_flag {
        a.stop_flag = true;
        *run_flag = false;
        if a.act == act::WALK || a.act == act::RUN {
            a.act = if input.area == 0 { act::IDLE } else { act::IDLE_FIELD };
        }
    }
    if !a.stop_flag && *move_flag {
        if *run_flag {
            if a.act == act::WALK {
                a.act = act::RUN;
            }
        } else if a.act == act::RUN {
            a.act = act::WALK;
        }
    }
    a.walk_run_cnt = if *move_flag { a.walk_run_cnt + 1 } else { 0 };
    if input.manual && a.act == act::IDLE_FIELD {
        a.act = act::FIDGET_FIELD;
    }
    if a.act_old != a.act {
        // ccAnm::SetAnm: frame 0, frameSpd kept.
        a.anim = a.act as usize;
        a.time = 0;
        a.posed = 0;
    }
    let spd = |k: F| ee::to_int(mul(K256, mul(mul(k, speed_rate), speed_value))) as u16;
    a.frame_spd = match a.act {
        act::RUN => spd(RUN_SPD),
        act::WALK => spd(WALK_SPD),
        _ => 256,
    };
    // _AnimateForward, then NoteProcess: the steps' notes.
    let (f, notes) = anims[a.anim].forward_notes(a.time, Ticks::from(a.frame_spd));
    events.extend(notes.into_iter().filter(|n| matches!(n.0, 1 | 2)).map(|n| ActEvent::Step(n.1)));
    if let Some(t) = f.pose_at {
        a.posed = t;
    }
    a.time = f.time;
    a.anm_flag = i16::from(f.ended);
    if a.act == act::ARRIVE {
        let (eff, s, e) = if input.warp { (0, 20, 40) } else { (30, 50, 70) };
        let mut cloak = 0;
        if a.act_cnt == eff {
            events.push(if input.warp { ActEvent::WarpTransfer } else { ActEvent::Transfer });
        } else if a.act_cnt > e {
            a.anm_flag = 1;
            cloak = if a.ghost { 0x3f00_0000 } else { ee::ONE };
        } else if a.act_cnt > s {
            cloak = div(from_int(i32::from(a.act_cnt - s)), from_int(i32::from(e - s)));
            if !ee::le(cloak, ee::ONE) {
                cloak = ee::ONE;
            }
            if a.ghost {
                cloak = mul(cloak, 0x3f00_0000);
            }
        }
        a.cloak = if a.act_cnt < s { 0 } else { cloak };
    } else if a.act == act::LEAVE {
        // The departure: the effect and the body off the list at actCnt 1,
        // cloak 1 until 21, then (41 - actCnt) / 20 down to 0; done at 141.
        let mut cloak = 0;
        if a.act_cnt == 1 {
            events.push(ActEvent::Transfer);
            events.push(ActEvent::Leaving);
        } else if a.act_cnt >= 141 {
            a.anm_flag = 1;
        } else if a.act_cnt >= 22 {
            cloak = div(from_int(41 - i32::from(a.act_cnt)), 0x41a0_0000);
            if ee::lt(cloak, 0) {
                cloak = 0;
            }
        }
        a.cloak = if a.act_cnt < 22 { ee::ONE } else { cloak };
    }
    a.act_old = a.act;
    if a.transfer_lag > 0 {
        a.transfer_lag -= 1;
    } else {
        a.act_cnt = a.act_cnt.wrapping_add(1);
    }
}
