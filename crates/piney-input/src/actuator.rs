//! The pad's motors (`ccPad` +0x28..+0x3f, main): what the game asks of them
//! and when it sends it. `SetActuater(small, power, ms)` (0x00102bf0) queues up
//! to three entries timed in vblanks, `ccSystem::Ctrl` (0x0010a740) counts the
//! top one down each frame, and `ccPad::Ctrl` (0x00102a50) sends a marked top
//! with 6 vblanks or more left (or both motors off when the queue empties).
//! The callers are `ccPlayer::DamageActuate` and the Vibration menus. The
//! rules are in docs/engine/overview.md ("The pad").

/// What the pad's two motors do: the small one on or off, the large one's
/// power (`scePadSetActDirect`'s two bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Motors {
    pub small: bool,
    pub large: u8,
}

/// A queued rumble (`ccPad` +0x30 + 4 k): vblanks left, marked to send,
/// the small motor, the large motor's power.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Entry {
    time: u16,
    send: bool,
    small: bool,
    power: u8,
}

/// `ccPad::Ctrl`'s sending state (+0x2e) once the pad is ready.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    #[default]
    Idle,
    Sent,
}

/// A pad's motor queue and what was last sent to the motors.
#[derive(Clone, Debug, Default)]
pub struct Actuator {
    /// `ccPad::actuaterSw`: the Vibration option (the save's `vibration`).
    pub sw: bool,
    /// +0x30: entry 0 is the idle one; 1..=`num` the queue, the top last.
    entries: [Entry; 4],
    /// +0x2f.
    num: usize,
    state: State,
}

impl Actuator {
    /// `ccPad::SetActuater(small, power, ms)`.
    pub fn set(&mut self, small: bool, power: u8, ms: i32) {
        if !self.sw {
            return;
        }
        let t = (3 * ms + 25) / 50;
        let mut top = self.num;
        let mut k = self.num;
        while k > 0 {
            if t < i32::from(self.entries[k].time) {
                self.entries[k].time -= t as u16;
            } else {
                for j in k..top {
                    self.entries[j] = self.entries[j + 1];
                }
                top -= 1;
            }
            k -= 1;
        }
        if top < 3 {
            self.entries[top].send = true;
            top += 1;
            self.entries[top] = Entry { time: t.clamp(0, i32::from(u16::MAX)) as u16, send: true, small, power };
            self.num = top;
        } else {
            self.num = top;
        }
    }

    /// `ccSystem::Ctrl`'s part: the finished entries off the top, then the
    /// top's time less `frame_rate` vblanks.
    pub fn tick(&mut self, frame_rate: u16) {
        while self.num > 0 && self.entries[self.num].time == 0 {
            self.num -= 1;
        }
        if self.num > 0 {
            let e = &mut self.entries[self.num];
            e.time = e.time.saturating_sub(frame_rate);
        }
    }

    /// `ccPad::Ctrl` once a frame: the motors it sends this frame, if any.
    pub fn ctrl(&mut self) -> Option<Motors> {
        if self.state == State::Sent {
            self.state = State::Idle;
            return None;
        }
        if self.num == 0 {
            if !self.entries[0].send {
                return None;
            }
            self.entries[0].send = false;
            self.state = State::Sent;
            return Some(Motors::default());
        }
        let e = &mut self.entries[self.num];
        if e.time < 6 || !e.send {
            return None;
        }
        e.send = false;
        self.state = State::Sent;
        Some(Motors { small: e.small, large: e.power })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hit's 100 ms at 30 frames a second: 6 vblanks, sent at once,
    /// then off once they are spent (two frames of 2, the third finds it
    /// empty and sends the idle entry's stop).
    #[test]
    fn a_hit_rumbles_then_stops() {
        let mut a = Actuator { sw: true, ..Actuator::default() };
        a.set(true, 128, 100);
        assert_eq!(a.ctrl(), Some(Motors { small: true, large: 128 }));
        let mut sent = Vec::new();
        for _ in 0..8 {
            a.tick(2);
            if let Some(m) = a.ctrl() {
                sent.push(m);
            }
        }
        assert_eq!(sent, [Motors::default()]);
    }

    #[test]
    fn off_does_nothing() {
        let mut a = Actuator::default();
        a.set(true, 255, 200);
        assert_eq!(a.ctrl(), None);
    }

    /// A second rumble over a longer first: the first loses the second's
    /// time and is sent again once the second is spent.
    #[test]
    fn a_shorter_rumble_over_a_longer_one() {
        let mut a = Actuator { sw: true, ..Actuator::default() };
        a.set(true, 64, 1000);
        assert_eq!(a.ctrl(), Some(Motors { small: true, large: 64 }));
        a.tick(1);
        a.ctrl();
        a.set(false, 255, 200);
        let mut sent = Vec::new();
        for _ in 0..80 {
            if let Some(m) = a.ctrl() {
                sent.push(m);
            }
            a.tick(1);
        }
        assert_eq!(sent, [Motors { small: false, large: 255 }, Motors { small: true, large: 64 }, Motors::default()]);
    }
}
