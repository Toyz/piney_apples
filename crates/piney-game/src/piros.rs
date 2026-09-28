//! The event instruction `piros_colour` (137; `ccEvent::Execute`'s case at
//! main 0x001b0c1c), which event 22 uses on Piros as his colour changes:
//! flashes (`scFadeDef`'s `EntryFlash`), sound 74, an information line, and
//! the affect tint of his `ccChar` (`affectColorFix` +0xa6, `affectColorRate`
//! +0xaa, `affectColor` +0xac), with breaths between. Only playing, with Piros
//! (`charTbl` row 8) registered; it switches on `eventStatus[1]` (0-13). The
//! cases and the pulse are in docs/engine/event-vm.md.

use std::collections::VecDeque;

use piney_desktop::eef::{div, from_int, mul};

/// Piros's `charTbl` row.
pub const PIROS: i32 = 8;
/// `eventStatus[1]` in the save (the message number, +0x6510, is the
/// interpreter's to read).
pub const STATUS: usize = 0x64f9;

/// `affectColorFix`, `affectColorRate`, `affectColor`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tint {
    pub fix: i16,
    pub rate: i16,
    pub colour: u32,
}

/// What a frame of the sequence does, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// `ccSeOn(n)`.
    Se(i32),
    /// `ccEvent::DispInfo(line, 0, 0)`.
    Info(Vec<u8>),
    /// `scFadeDef->EntryFlash(count, colour, 0, 0, 512, 384)`.
    Flash { count: i16, colour: u32 },
    /// Piros's tint.
    Tint(Tint),
}

/// The instruction's frames: each part runs after the breath before it.
#[derive(Clone, Debug, Default)]
pub struct Sequence {
    /// (frames to wait first, what runs then).
    parts: VecDeque<(u32, Vec<Action>)>,
    left: u32,
    fresh: bool,
}

const WHITE: u32 = 0x60ff_ffff;

fn set(fix: i16, rate: i16, colour: u32) -> Action {
    Action::Tint(Tint { fix, rate, colour })
}

/// `fptosi(65 k / n)` as the EE works it.
fn ramp(k: i32, n: i32) -> i16 {
    div(mul(65.0, from_int(k)), from_int(n)) as i32 as i16
}

impl Sequence {
    /// The instruction on status `status` with operand `code` and the
    /// message line `line`; `piros` whether Piros is loaded. The first part
    /// (no wait) is what runs in the instruction's own frame.
    pub fn new(status: i8, code: i16, line: &[u8], piros: bool) -> Sequence {
        let mut parts: VecDeque<(u32, Vec<Action>)> = VecDeque::new();
        let mut now: Vec<Action> = Vec::new();
        if piros {
            let told = || vec![Action::Se(74), Action::Info(line.to_vec())];
            let steady = |mut first: Vec<Action>, flash: u32, colour: u32, parts: &mut VecDeque<_>| {
                first.push(Action::Flash { count: 8, colour: flash });
                first.push(set(1, 65, colour));
                parts.push_back((0, first));
                parts.push_back((5, vec![set(1, 65, colour)]));
            };
            // The pulse; `steady_after` the tint back at 65 when it ends
            // (11-13 do, 7 does not).
            let pulse = |mut first: Vec<Action>, colour: u32, steady_after: bool, parts: &mut VecDeque<_>| {
                first.push(Action::Flash { count: 8, colour: WHITE });
                let mut rates = Vec::new();
                for r in 0..13 {
                    let n = r + 1;
                    let ks: Vec<i32> = if r % 2 == 1 { (0..=n).collect() } else { (0..=n).rev().collect() };
                    rates.extend(ks.into_iter().map(|k| ramp(k, n)));
                }
                // Each set, then a breath of one.
                let mut wait = 0;
                for rate in rates {
                    first.push(set(1, rate, colour));
                    parts.push_back((wait, std::mem::take(&mut first)));
                    wait = 1;
                }
                parts.push_back((1, if steady_after { vec![set(1, 65, colour)] } else { Vec::new() }));
            };
            match status {
                0 | 10 => now.push(set(0, 0, 0)),
                1 => steady(Vec::new(), 0x6020_80ff, 0x0020_80ff, &mut parts),
                2 | 8 => now.push(set(1, 65, 0x0020_80ff)),
                3 => steady(told(), 0x6080_80ff, 0x0080_80ff, &mut parts),
                4 => now.push(set(1, 65, 0x0080_80ff)),
                5 => steady(told(), 0x6000_ffff, 0x0040_c8ff, &mut parts),
                6 => now.push(set(1, 65, 0x0040_c8ff)),
                7 => pulse(told(), 0x0020_80ff, false, &mut parts),
                11 => pulse(Vec::new(), 0x0020_80ff, true, &mut parts),
                12 => pulse(Vec::new(), 0x0080_80ff, true, &mut parts),
                13 => pulse(Vec::new(), 0x0040_c8ff, true, &mut parts),
                9 => match code {
                    0 => {
                        now = told();
                        now.push(Action::Flash { count: 8, colour: WHITE });
                    }
                    1..=4 => {
                        let colour = [0x0020_80ff, 0x0080_80ff, 0x0020_4070, 0x0000_00ff][code as usize - 1];
                        let mut first = vec![Action::Flash { count: 8, colour: WHITE }];
                        let ks = (0..=6).rev().chain(0..=6);
                        let mut wait = 0;
                        for k in ks {
                            first.push(set(1, ramp(k, 6), colour));
                            parts.push_back((wait, std::mem::take(&mut first)));
                            wait = 1;
                        }
                        parts.push_back((1, Vec::new()));
                    }
                    5 => {
                        now.push(Action::Flash { count: 20, colour: 0x80ff_ffff });
                        now.push(set(0, 0, 0));
                    }
                    // Not in the scripts: the game pulses in a colour its
                    // stack last held.
                    _ => {}
                },
                _ => {}
            }
        }
        if parts.is_empty() {
            parts.push_back((0, now));
        }
        Sequence { parts, left: 0, fresh: true }
    }

    /// The first part: what the instruction does in its own frame.
    pub fn start(&mut self) -> Vec<Action> {
        let (_, first) = self.parts.pop_front().unwrap_or_default();
        self.left = self.parts.front().map_or(0, |p| p.0);
        first
    }

    /// The interpreter's `busy` poll: nothing in the instruction's own frame
    /// (its breath has just begun); after, a frame of the breath each call,
    /// and the part after it when it ends. False once the last part ran.
    pub fn tick(&mut self) -> (Vec<Action>, bool) {
        if std::mem::take(&mut self.fresh) {
            return (Vec::new(), !self.parts.is_empty());
        }
        let mut out = Vec::new();
        self.left = self.left.saturating_sub(1);
        while self.left == 0 {
            let Some((_, acts)) = self.parts.pop_front() else { return (out, false) };
            out.extend(acts);
            match self.parts.front() {
                Some(p) => self.left = p.0,
                None => return (out, false),
            }
        }
        (out, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("piros_fixture.txt");

    fn fnv(b: &[u8]) -> u32 {
        let mut h: u32 = 0x811c_9dc5;
        for &c in b {
            h = (h ^ u32::from(c)).wrapping_mul(0x0100_0193);
        }
        h
    }

    /// The sequence as `tools/test_piros_rs.py` records the game's: each
    /// flash, sound and line, and each breath with the tint after it; then
    /// the tint at the end. The lines are the fixture's own (by message).
    fn record(status: i8, code: i16, msg: usize, piros: bool, lines: &[Vec<u8>]) -> String {
        let mut s = Sequence::new(status, code, &lines[msg], piros);
        let mut tint = Tint::default();
        let mut log = Vec::new();
        let apply = |acts: Vec<Action>, log: &mut Vec<String>, tint: &mut Tint| {
            for a in acts {
                match a {
                    Action::Se(n) => log.push(format!("se {n}")),
                    Action::Info(l) => log.push(format!("info {}:{} -1 -1", l.len(), fnv(&l))),
                    Action::Flash { count, colour } => {
                        log.push(format!("flash {count} {colour:08x} 00000000 00000000 44000000 43c00000"))
                    }
                    Action::Tint(t) => *tint = t,
                }
            }
        };
        apply(s.start(), &mut log, &mut tint);
        let mut breath = s.left;
        let (acts, mut busy) = s.tick();
        apply(acts, &mut log, &mut tint);
        while busy {
            // A breath of `breath` frames: the tint during it, then the part.
            log.push(format!("breath {breath} {} {} {:08x}", tint.fix, tint.rate, tint.colour));
            let mut acts = Vec::new();
            for _ in 0..breath {
                let (a, b) = s.tick();
                acts = a;
                busy = b;
            }
            breath = s.left;
            apply(acts, &mut log, &mut tint);
        }
        format!("{} | end {} {} {:08x}", log.join(" ; "), tint.fix, tint.rate, tint.colour)
    }

    /// Every record of `tools/test_piros_rs.py`'s run of the game's case:
    /// statuses 0-14 with messages 0-3, status 9's operands 1-5, and no
    /// Piros.
    #[test]
    fn piros_colour_is_the_games() {
        let mut n = 0;
        for l in FIXTURE.lines().filter(|l| l.starts_with("piros ")) {
            let (head, body) = l.split_once(" | ").unwrap();
            let h: Vec<&str> = head.split(' ').collect();
            let (status, msg, piros): (i8, usize, bool) = (h[1].parse().unwrap(), h[2].parse().unwrap(), h[3] == "1");
            let code: i16 = h.get(4).map_or(0, |c| c.parse().unwrap());
            // The lines are event 22's own ([`event_22_lines_are_the_games`]
            // compares them); here they are placeholders, the info records
            // compared without their text.
            let lines: Vec<Vec<u8>> = (0..4).map(|k| vec![b'x'; k + 1]).collect();
            let norm = |s: &str| {
                s.split(" ; ").map(|p| if p.starts_with("info ") { "info" } else { p }).collect::<Vec<_>>().join(" ; ")
            };
            let got = record(status, code, msg, piros, &lines);
            assert_eq!(norm(&got), norm(body), "status {status} operand {code} message {msg} piros {piros}");
            n += 1;
        }
        assert_eq!(n, 66);
    }

    /// The information line: the first line of event 22's messages 0-3
    /// (Parody Mode off), as the game's `ccKanjiStrSeparate` gave them to
    /// `DispInfo` (the fixture's lengths and hashes).
    #[test]
    fn event_22_lines_are_the_games() {
        let iso = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = piney_data::iso::Iso::open(&iso).unwrap();
        let lib =
            piney_event::vm::Library::new(disc.volume().unwrap(), piney_event::official::events(&mut disc).unwrap());
        for l in FIXTURE.lines().filter(|l| l.starts_with("piros 3 ")) {
            let (head, body) = l.split_once(" | ").unwrap();
            let h: Vec<&str> = head.split(' ').collect();
            let Some(info) = body.split(" ; ").find(|p| p.starts_with("info ")) else { continue };
            let msg: i32 = h[2].parse().unwrap();
            let line = lib.message(22, msg, false).and_then(|m| m.lines.first()).map(|t| t.as_bytes().to_vec());
            let line = line.unwrap_or_default();
            let want = info.split(' ').nth(1).unwrap();
            assert_eq!(format!("{}:{}", line.len(), fnv(&line)), want, "message {msg}");
        }
    }
}
