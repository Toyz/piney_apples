//! The game's pad reading: `ccPad::Read` (`INF SLUS_202.67:0x00102d40`), once
//! per frame, for a pad already connected (`ccPad::Ctrl`'s state 0x40): the
//! active-low buttons inverted into [`Buttons`] (the SCE layout), the left
//! stick pressing the D-pad in analog mode, and `push`, `unpush` and `repeat`
//! (15 frames' wait) from the last frame's `direct`. Two quirks are kept: a
//! stick with raw y exactly 128 points down, and the up-right eighth presses
//! right alone (docs/engine/overview.md, "The pad").

use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

pub mod actuator;

/// Button bits as `ccPad.direct` holds them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Buttons(pub u32);

impl Buttons {
    pub const NONE: Buttons = Buttons(0);
    pub const L2: Buttons = Buttons(0x0001);
    pub const R2: Buttons = Buttons(0x0002);
    pub const L1: Buttons = Buttons(0x0004);
    pub const R1: Buttons = Buttons(0x0008);
    pub const TRIANGLE: Buttons = Buttons(0x0010);
    pub const CIRCLE: Buttons = Buttons(0x0020);
    pub const CROSS: Buttons = Buttons(0x0040);
    pub const SQUARE: Buttons = Buttons(0x0080);
    pub const SELECT: Buttons = Buttons(0x0100);
    pub const L3: Buttons = Buttons(0x0200);
    pub const R3: Buttons = Buttons(0x0400);
    pub const START: Buttons = Buttons(0x0800);
    pub const UP: Buttons = Buttons(0x1000);
    pub const RIGHT: Buttons = Buttons(0x2000);
    pub const DOWN: Buttons = Buttons(0x4000);
    pub const LEFT: Buttons = Buttons(0x8000);
    /// The D-pad bits, which the stick can also set.
    pub const DPAD: Buttons = Buttons(0xf000);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Every bit of `other` is set.
    pub const fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0
    }

    /// Any bit of `other` is set.
    pub const fn intersects(self, other: Buttons) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for Buttons {
    type Output = Buttons;
    fn bitor(self, other: Buttons) -> Buttons {
        Buttons(self.0 | other.0)
    }
}

impl BitOrAssign for Buttons {
    fn bitor_assign(&mut self, other: Buttons) {
        self.0 |= other.0;
    }
}

impl BitAnd for Buttons {
    type Output = Buttons;
    fn bitand(self, other: Buttons) -> Buttons {
        Buttons(self.0 & other.0)
    }
}

impl Not for Buttons {
    type Output = Buttons;
    fn not(self) -> Buttons {
        Buttons(!self.0 & 0xffff)
    }
}

/// Frames a combination is held before `repeat` fires every frame
/// (`ccPad.repeatCnt` counts to 15).
pub const REPEAT_DELAY: u8 = 15;

/// Stick offsets within this of the centre (128) count as none.
pub const DEAD_ZONE: i32 = 48;

/// The centre of a stick axis.
pub const CENTRE: i32 = 128;

/// What the pad reports for one frame (`scePadRead`'s bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Raw {
    /// Held buttons, active high.
    pub buttons: Buttons,
    /// A DualShock in analog mode (pad id 7; 5, the analog joystick, reads
    /// the same): the sticks are read.
    pub analog: bool,
    /// Left and right sticks, 0 left / up to 255 right / down, 128 centred.
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    /// A DualShock 2 in pressure mode (pad id 0x79) also reports how hard
    /// each of twelve buttons is pressed, 0-255, in the order right, left,
    /// up, down, triangle, circle, cross, square, L1, R1, L2, R2. None for a
    /// pad without (`ccPad::Read` then zeroes `pow`).
    pub pressure: Option<[u8; 12]>,
}

impl Default for Raw {
    fn default() -> Self {
        Raw { buttons: Buttons::NONE, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, pressure: None }
    }
}

/// Indices into [`Pad::pow`] (`ccPad.pow`), the pressure mode's order.
pub mod pressure {
    pub const L1: usize = 8;
    pub const R1: usize = 9;
    pub const L2: usize = 10;
    pub const R2: usize = 11;
}

/// `ccPad`'s per-frame state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pad {
    /// Held.
    pub direct: Buttons,
    /// Pressed this frame.
    pub push: Buttons,
    /// Released this frame.
    pub unpush: Buttons,
    /// Pressed this frame, or held long enough to repeat.
    pub repeat: Buttons,
    pub repeat_cnt: u8,
    /// The left stick's (or, when it is centred, the D-pad's) direction in
    /// radians, 0 down, pi/2 left, pi up, -pi/2 right; and its strength,
    /// 0-255 (255 for the D-pad).
    pub dirc_l: f32,
    pub pow_l: u8,
    /// The right stick's.
    pub dirc_r: f32,
    pub pow_r: u8,
    /// `ccPad.pow`: each button's pressure (see [`Raw::pressure`]), all 0
    /// for a pad that does not report it. The field camera turns and zooms
    /// by L1, R1 and R2's pressure, and at a fixed rate when it reads 0.
    pub pow: [u8; 12],
}

/// `dircTbl` (0x003484a0): the D-pad's direction by bits 12-15 (up, right,
/// down, left); [`NO_DIRECTION`] for none or an impossible combination.
const DIRC_TBL: [u32; 16] = [
    0x4080_0000, // none
    0x4049_0fdb, // up: pi
    0xbfc9_0fdb, // right: -pi/2
    0xc016_cbe4, // up right: -3pi/4
    0x0000_0000, // down: 0
    0x4080_0000,
    0xbf49_0fdb, // down right: -pi/4
    0x4080_0000,
    0x3fc9_0fdb, // left: pi/2
    0x4016_cbe4, // up left: 3pi/4
    0x4080_0000,
    0x4080_0000,
    0x3f49_0fdb, // down left: pi/4
    0x4080_0000,
    0x4080_0000,
    0x4080_0000,
];

/// `dircTbl`'s "no direction", 4.0.
const NO_DIRECTION: f32 = 4.0;

/// The stick's eighths: above each bound (in turn), the D-pad bits it
/// presses; below the last, up. Bounds are the odd multiples of pi/8 as the
/// code has them.
const EIGHTHS: [(u32, Buttons); 8] = [
    (0x402f_ede0, Buttons::UP),     // > 7pi/8
    (0x3ffb_53d2, Buttons(0x9000)), // up left
    (0x3f96_cbe4, Buttons::LEFT),   // > 3pi/8
    (0x3ec9_0fdb, Buttons(0xc000)), // down left
    (0xbec9_0fdb, Buttons::DOWN),   // > -pi/8
    (0xbf96_cbe4, Buttons(0x6000)), // down right
    (0xbffb_53d2, Buttons::RIGHT),  // > -5pi/8
    (0xc02f_ede0, Buttons::RIGHT),  // > -7pi/8: not up right
];

/// `SetAnalogStick` (0x00102390): a stick's direction and strength, or None
/// inside the dead zone.
pub fn stick(x: u8, y: u8) -> Option<(f32, u8)> {
    let axis = |v: u8| {
        let d = i32::from(v) - CENTRE;
        if d > DEAD_ZONE {
            d - DEAD_ZONE
        } else if d < -DEAD_ZONE {
            d + DEAD_ZONE
        } else {
            0
        }
    };
    let (ax, ay) = (axis(x), axis(y));
    let len = ((ax * ax + ay * ay) as f32).sqrt();
    // 255 over the 80 steps past the dead zone, truncated (`fptosi`).
    let pow = ((255.0 * len / 80.0) as i32).min(255);
    if pow == 0 {
        return None;
    }
    let (dx, dy) = (i32::from(x) - CENTRE, i32::from(y) - CENTRE);
    let dirc = if dy == 0 {
        0.0
    } else {
        let a = (-dx as f32).atan2(dy as f32);
        if a < -std::f32::consts::PI { a + 2.0 * std::f32::consts::PI } else { a }
    };
    Some((dirc, pow as u8))
}

impl Pad {
    /// `ccPad::Read` for one frame's report.
    pub fn read(&mut self, raw: &Raw) {
        let mut now = raw.buttons & Buttons(0xffff);
        let dpad_dirc = || f32::from_bits(DIRC_TBL[((now.0 >> 12) & 0xf) as usize]);
        if raw.analog {
            match stick(raw.lx, raw.ly) {
                None => {
                    let d = dpad_dirc();
                    (self.dirc_l, self.pow_l) = if d == NO_DIRECTION { (0.0, 0) } else { (d, 255) };
                }
                Some((d, p)) => {
                    (self.dirc_l, self.pow_l) = (d, p);
                    if !now.intersects(Buttons::DPAD) {
                        now |= EIGHTHS
                            .iter()
                            .find(|(bound, _)| d > f32::from_bits(*bound))
                            .map_or(Buttons::UP, |&(_, bits)| bits);
                    }
                }
            }
            (self.dirc_r, self.pow_r) = stick(raw.rx, raw.ry).unwrap_or((0.0, 0));
        } else {
            let d = dpad_dirc();
            (self.dirc_l, self.pow_l) = if d == NO_DIRECTION { (0.0, 0) } else { (d, 255) };
            (self.dirc_r, self.pow_r) = (0.0, 0);
        }
        self.pow = raw.pressure.unwrap_or([0; 12]);
        self.push = now & !self.direct;
        self.unpush = self.direct & !now;
        if !now.is_empty() && now == self.direct {
            if self.repeat_cnt < REPEAT_DELAY {
                self.repeat_cnt += 1;
                self.repeat = Buttons::NONE;
            } else {
                self.repeat = now;
            }
        } else {
            self.repeat = now;
            self.repeat_cnt = 0;
        }
        self.direct = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pad: &mut Pad, buttons: Buttons) {
        pad.read(&Raw { buttons, ..Raw::default() });
    }

    #[test]
    fn push_release_and_repeat() {
        let mut pad = Pad::default();
        frame(&mut pad, Buttons::CROSS);
        assert_eq!((pad.push, pad.repeat, pad.direct), (Buttons::CROSS, Buttons::CROSS, Buttons::CROSS));
        // 15 quiet frames, then every frame.
        for _ in 0..REPEAT_DELAY {
            frame(&mut pad, Buttons::CROSS);
            assert_eq!((pad.push, pad.repeat), (Buttons::NONE, Buttons::NONE));
        }
        frame(&mut pad, Buttons::CROSS);
        assert_eq!(pad.repeat, Buttons::CROSS);
        frame(&mut pad, Buttons::CROSS);
        assert_eq!(pad.repeat, Buttons::CROSS);
        frame(&mut pad, Buttons::NONE);
        assert_eq!((pad.unpush, pad.repeat, pad.repeat_cnt), (Buttons::CROSS, Buttons::NONE, 0));
    }

    #[test]
    fn stick_presses_the_dpad() {
        let at = |x, y| {
            let mut pad = Pad::default();
            pad.read(&Raw { lx: x, ly: y, ..Raw::default() });
            pad.direct
        };
        assert_eq!(at(128, 255), Buttons::DOWN);
        assert_eq!(at(128, 0), Buttons::UP);
        assert_eq!(at(0, 127), Buttons::LEFT);
        assert_eq!(at(255, 129), Buttons::RIGHT);
        assert_eq!(at(0, 0), Buttons(0x9000));
        assert_eq!(at(0, 255), Buttons(0xc000));
        assert_eq!(at(255, 255), Buttons(0x6000));
        // The game's quirks: up right is right; y exactly centred is down.
        assert_eq!(at(255, 0), Buttons::RIGHT);
        assert_eq!(at(255, 128), Buttons::DOWN);
        // Inside the dead zone nothing; a held D-pad wins over the stick.
        assert_eq!(at(170, 90), Buttons::NONE);
        let mut pad = Pad::default();
        pad.read(&Raw { buttons: Buttons::LEFT, lx: 255, ly: 128, ..Raw::default() });
        assert_eq!(pad.direct, Buttons::LEFT);
    }

    #[test]
    fn dead_zone_and_strength() {
        assert_eq!(stick(128 + 48, 128), None);
        assert_eq!(stick(128 + 49, 128).map(|s| s.1), Some(3));
        // 79 steps past the dead zone on one axis; a diagonal reaches 255.
        assert_eq!(stick(255, 128).map(|s| s.1), Some(251));
        assert_eq!(stick(255, 255).map(|s| s.1), Some(255));
        assert_eq!(stick(128, 255).map(|s| s.0), Some(0.0));
        assert_eq!(stick(0, 128 + 64).map(|s| s.0 > 0.0), Some(true));
    }
}
