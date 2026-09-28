//! Keyboard and gamepad to what a PS2 pad reports ([`Raw`]); the game reads
//! that as `ccPad::Read` does ([`piney_input::Pad`]).
//!
//! Keyboard: arrows the D-pad, Z Cross, X Circle, A Square, S Triangle,
//! Q / W L1 / R1, 1 / 2 L2 / R2, Enter Start, Backspace Select.
//! Gamepad (a DualSense or any pad gilrs knows): the buttons where a
//! DualShock 2 has them, Create as Select, Options as Start, sticks as
//! sticks.

use std::collections::HashSet;

use gilrs::{Axis, Button, Gilrs};
use piney_input::{Buttons, Raw};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Keys held, by physical position.
#[derive(Default)]
pub struct Keyboard {
    held: HashSet<KeyCode>,
}

const KEYS: [(KeyCode, Buttons); 16] = [
    (KeyCode::ArrowUp, Buttons::UP),
    (KeyCode::ArrowDown, Buttons::DOWN),
    (KeyCode::ArrowLeft, Buttons::LEFT),
    (KeyCode::ArrowRight, Buttons::RIGHT),
    (KeyCode::KeyZ, Buttons::CROSS),
    (KeyCode::KeyX, Buttons::CIRCLE),
    (KeyCode::KeyA, Buttons::SQUARE),
    (KeyCode::KeyS, Buttons::TRIANGLE),
    (KeyCode::KeyQ, Buttons::L1),
    (KeyCode::KeyW, Buttons::R1),
    (KeyCode::Digit1, Buttons::L2),
    (KeyCode::Digit2, Buttons::R2),
    (KeyCode::Enter, Buttons::START),
    (KeyCode::Backspace, Buttons::SELECT),
    (KeyCode::KeyC, Buttons::L3),
    (KeyCode::KeyV, Buttons::R3),
];

impl Keyboard {
    pub fn key(&mut self, key: PhysicalKey, pressed: bool) {
        let PhysicalKey::Code(code) = key else { return };
        if pressed {
            self.held.insert(code);
        } else {
            self.held.remove(&code);
        }
    }

    pub fn buttons(&self) -> Buttons {
        let mut b = Buttons::NONE;
        for (k, bits) in KEYS {
            if self.held.contains(&k) {
                b |= bits;
            }
        }
        b
    }
}

const PAD: [(Button, Buttons); 16] = [
    (Button::DPadUp, Buttons::UP),
    (Button::DPadDown, Buttons::DOWN),
    (Button::DPadLeft, Buttons::LEFT),
    (Button::DPadRight, Buttons::RIGHT),
    (Button::South, Buttons::CROSS),
    (Button::East, Buttons::CIRCLE),
    (Button::West, Buttons::SQUARE),
    (Button::North, Buttons::TRIANGLE),
    (Button::LeftTrigger, Buttons::L1),
    (Button::RightTrigger, Buttons::R1),
    (Button::LeftTrigger2, Buttons::L2),
    (Button::RightTrigger2, Buttons::R2),
    (Button::Start, Buttons::START),
    (Button::Select, Buttons::SELECT),
    (Button::LeftThumb, Buttons::L3),
    (Button::RightThumb, Buttons::R3),
];

/// A modern stick as a DualShock 2 reports it. A DualShock 2's pots
/// saturate before its round gate, so a full push in any direction reads at
/// or near the ends of both axes; the game's strength (`SetAnalogStick`,
/// 255 over 80 steps past its dead zone) is built on that, and its run/walk
/// threshold sits where a round stick pushed diagonally only reaches 0.7 per
/// axis. So the stick's radius is carried to the square: the vector is
/// scaled so its larger axis is its length (at most 1). Raw values, with no
/// dead zone of our own: the game applies its own.
pub fn square_stick(x: f32, y: f32) -> (f32, f32) {
    let r = (x * x + y * y).sqrt().min(1.0);
    let m = x.abs().max(y.abs());
    if m <= 0.0 { (0.0, 0.0) } else { (x * r / m, y * r / m) }
}

/// A stick axis (-1 left or down in gilrs, 1 right or up) as the pad's byte:
/// 0 left or up, 255 right or down, 128 centred.
fn axis_byte(v: f32, flip: bool) -> u8 {
    let v = if flip { -v } else { v };
    (128.0 + v.clamp(-1.0, 1.0) * 127.5).floor().clamp(0.0, 255.0) as u8
}

/// Every gamepad gilrs knows, one per line: id, name, driver name, uuid,
/// whether it is connected.
pub fn gamepads(gilrs: &Gilrs) -> String {
    gilrs
        .gamepads()
        .map(|(id, p)| {
            format!(
                "gamepad {id}: {:?} ({:?}) uuid {:02x?} connected {}",
                p.name(),
                p.os_name(),
                p.uuid(),
                p.is_connected()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// This frame's report: the keyboard and the first connected gamepad
/// together. With `log`, what was read is described there: the gamepad
/// taken, its sticks' and triggers' values as gilrs gives them, and the
/// events drained this frame.
pub fn read(keyboard: &Keyboard, gilrs: Option<&mut Gilrs>, log: Option<&mut String>) -> Raw {
    let mut raw = Raw { buttons: keyboard.buttons(), ..Raw::default() };
    let Some(gilrs) = gilrs else { return raw };
    let mut events = 0;
    while gilrs.next_event().is_some() {
        events += 1;
    }
    if let Some((id, pad)) = gilrs.gamepads().find(|(_, p)| p.is_connected()) {
        if let Some(log) = log {
            let v = |a| pad.value(a);
            *log = format!(
                "pad {id} events {events} ls {:+.3} {:+.3} rs {:+.3} {:+.3} lz {:+.3} rz {:+.3}",
                v(Axis::LeftStickX),
                v(Axis::LeftStickY),
                v(Axis::RightStickX),
                v(Axis::RightStickY),
                v(Axis::LeftZ),
                v(Axis::RightZ)
            );
        }
        for (b, bits) in PAD {
            if pad.is_pressed(b) {
                raw.buttons |= bits;
            }
        }
        let (lx, ly) = square_stick(pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
        let (rx, ry) = square_stick(pad.value(Axis::RightStickX), pad.value(Axis::RightStickY));
        raw.lx = axis_byte(lx, false);
        raw.ly = axis_byte(ly, true);
        raw.rx = axis_byte(rx, false);
        raw.ry = axis_byte(ry, true);
    }
    raw
}

/// Buttons by name, for scripted presses: `cross`, `circle`, `square`,
/// `triangle`, `up`, `down`, `left`, `right`, `start`, `select`, `l1`,
/// `r1`, `l2`, `r2`, `l3`, `r3`.
pub fn button(name: &str) -> Option<Buttons> {
    Some(match name {
        "cross" | "x" => Buttons::CROSS,
        "circle" | "o" => Buttons::CIRCLE,
        "square" => Buttons::SQUARE,
        "triangle" => Buttons::TRIANGLE,
        "up" => Buttons::UP,
        "down" => Buttons::DOWN,
        "left" => Buttons::LEFT,
        "right" => Buttons::RIGHT,
        "start" => Buttons::START,
        "select" => Buttons::SELECT,
        "l1" => Buttons::L1,
        "r1" => Buttons::R1,
        "l2" => Buttons::L2,
        "r2" => Buttons::R2,
        "l3" => Buttons::L3,
        "r3" => Buttons::R3,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_bytes() {
        assert_eq!(axis_byte(0.0, false), 128);
        assert_eq!(axis_byte(1.0, false), 255);
        assert_eq!(axis_byte(-1.0, false), 0);
        // gilrs has up positive; the pad has up at 0.
        assert_eq!(axis_byte(1.0, true), 0);
    }

    /// A round stick pushed fully north-west reads as a DualShock 2's does:
    /// both axes at their ends, so the game sees full strength and runs.
    #[test]
    fn a_full_diagonal_reaches_the_corner() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let (x, y) = square_stick(-d, d);
        assert!((x + 1.0).abs() < 1e-5 && (y - 1.0).abs() < 1e-5, "{x} {y}");
        let raw = Raw { lx: axis_byte(x, false), ly: axis_byte(y, true), ..Raw::default() };
        assert_eq!((raw.lx, raw.ly), (0, 0));
        let mut pad = piney_input::Pad::default();
        pad.read(&raw);
        assert_eq!(pad.pow_l, 255);
        // Straight pushes and the centre are unchanged; half a push stays half.
        assert_eq!(square_stick(0.0, 1.0), (0.0, 1.0));
        assert_eq!(square_stick(0.0, 0.0), (0.0, 0.0));
        let (x, y) = square_stick(0.5 * d, 0.5 * d);
        assert!((x - 0.5).abs() < 1e-5 && (y - 0.5).abs() < 1e-5);
    }
}

/// The pad's two motors on the gamepad, through gilrs' force feedback:
/// the large motor as its strong rumble (at the power's share of full),
/// the small one as its weak rumble (on or off). Each is an effect that
/// plays until stopped.
pub struct Rumble {
    strong: gilrs::ff::Effect,
    weak: gilrs::ff::Effect,
}

impl Rumble {
    /// The first connected gamepad that takes force feedback; None without
    /// one.
    pub fn new(gilrs: &mut Gilrs) -> Option<Rumble> {
        use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
        let id = gilrs.gamepads().find(|(_, p)| p.is_connected() && p.is_ff_supported()).map(|(id, _)| id)?;
        let effect = |kind: BaseEffectType, gilrs: &mut Gilrs| {
            EffectBuilder::new()
                .add_effect(BaseEffect {
                    kind,
                    scheduling: Replay { play_for: Ticks::from_ms(1000), ..Default::default() },
                    ..Default::default()
                })
                .gamepads(&[id])
                .finish(gilrs)
                .map_err(|e| eprintln!("rumble: {e}"))
                .ok()
        };
        let strong = effect(BaseEffectType::Strong { magnitude: u16::MAX }, gilrs)?;
        let weak = effect(BaseEffectType::Weak { magnitude: u16::MAX }, gilrs)?;
        Some(Rumble { strong, weak })
    }

    /// What `scePadSetActDirect` sends: the motors as they are to run.
    pub fn set(&self, m: piney_input::actuator::Motors) {
        let r = if m.large > 0 {
            self.strong.set_gain(f32::from(m.large) / 255.0).and_then(|_| self.strong.play())
        } else {
            self.strong.stop()
        };
        let r = r.and_then(|_| if m.small { self.weak.play() } else { self.weak.stop() });
        if let Err(e) = r {
            eprintln!("rumble: {e}");
        }
    }
}

#[cfg(test)]
mod rumble_tests {
    /// By hand, with a pad connected: `cargo test -p piney-game
    /// rumble_on_the_pad -- --ignored --nocapture` rumbles it (the large
    /// motor at 160, then both, then off) and prints what gilrs reports.
    #[test]
    #[ignore]
    fn rumble_on_the_pad() {
        let mut gilrs = gilrs::Gilrs::new().expect("gilrs");
        for (id, p) in gilrs.gamepads() {
            eprintln!("{id}: {} connected {} ff {}", p.name(), p.is_connected(), p.is_ff_supported());
        }
        let r = super::Rumble::new(&mut gilrs).expect("no pad with force feedback");
        let wait = |ms| std::thread::sleep(std::time::Duration::from_millis(ms));
        r.set(piney_input::actuator::Motors { small: false, large: 160 });
        wait(500);
        r.set(piney_input::actuator::Motors { small: true, large: 255 });
        wait(500);
        r.set(piney_input::actuator::Motors::default());
        wait(200);
    }
}
