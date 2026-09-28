//! The enemies' fire breath ([`piney_battle::breath::Breath`]) driven line by
//! line for `tools/test_enemy_breath_rs.py`, which runs the game's own
//! `ccEnemyBreath` in eemu and compares: `new KIND SMOKE SEED MTI`
//! (`initBreath`), `set PARAMHEX DISP TRANS CNT M..` (`setBreath` with the
//! node's matrix), `ctrl NL L.. NW W..` (`ctrlBreath` with the ground's and
//! walls' answers in order). Each line answers the breath as JSON (and the
//! matrix after `set`, the outputs and ccRand's state after `ctrl`). W2P is
//! (x + 0.5, y - 0.25, z, w) and P2W its inverse, as the harness hooks them.

use std::collections::VecDeque;
use std::io::{BufRead, Write};

use piney_battle::breath::{BrInfo, BrParam, BrPart, Breath, BreathOut, BreathWorld};
use piney_battle::geom::{self, F, M4, V4};
use piney_battle::rand::Genrand;

struct World {
    land: VecDeque<F>,
    line: VecDeque<F>,
}

impl BreathWorld for World {
    fn line(&mut self, _a: V4, _b: V4, _mask: u32) -> F {
        self.line.pop_front().expect("a wall's answer")
    }

    fn land(&mut self, _p: V4, _mask: u32) -> F {
        self.land.pop_front().expect("a ground's answer")
    }

    fn w2p(&mut self, p: V4) -> V4 {
        [geom::add(p[0], 0x3f00_0000), geom::add(p[1], 0xbe80_0000), p[2], p[3]]
    }

    fn p2w(&mut self, p: V4) -> V4 {
        [geom::sub(p[0], 0x3f00_0000), geom::sub(p[1], 0xbe80_0000), p[2], p[3]]
    }
}

fn hexw(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn list<T: ToString>(v: impl IntoIterator<Item = T>) -> String {
    let parts: Vec<String> = v.into_iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn part_vec(p: &BrPart) -> Vec<i64> {
    let mut v = vec![
        i64::from(p.active),
        i64::from(p.landed),
        i64::from(p.wall),
        i64::from(p.life),
        i64::from(p.state),
        i64::from(p.count),
        i64::from(p.pattern),
        i64::from(p.alpha),
    ];
    v.extend(p.rot.map(i64::from));
    v.extend(p.vel.map(i64::from));
    v.push(i64::from(p.phase));
    v.extend(p.pos.map(i64::from));
    v.extend(p.scale.map(i64::from));
    v.extend([i64::from(p.rotate), i64::from(p.eff_alpha)]);
    v
}

fn breath_json(b: &Breath) -> String {
    let parts: Vec<String> = b.parts.iter().map(|p| list(part_vec(p))).collect();
    format!("[{},[{}]]", b.count, parts.join(","))
}

/// The harness's ccRand fingerprint: FNV-1a over the words' bytes.
fn cc_state(cc: &Genrand) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for w in cc.mt {
        for byte in w.to_le_bytes() {
            h = (h ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("[{},{}]", cc.mti, h)
}

fn out_json(o: &BreathOut) -> String {
    match *o {
        BreathOut::Draw { part, pos, scale, rotate, alpha, pattern } => {
            format!("[\"draw\",{part},{},{},{rotate},{alpha},{pattern}]", list(pos), list(scale))
        }
        BreathOut::Smoke { pos, vel, size, n, kind, fade_in, fade_out } => {
            format!("[\"smoke\",{},{},{size},{n},{kind},{fade_in},{fade_out}]", list(pos), list(vel))
        }
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    let mut breath = Breath::new(BrInfo::default());
    let mut cc = Genrand::default();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let t: Vec<&str> = line.split_whitespace().collect();
        let reply = match t[0] {
            "new" => {
                let info = BrInfo { kind: t[1].parse().unwrap(), smoke: t[2].parse().unwrap(), ..BrInfo::default() };
                breath = Breath::new(info);
                cc = Genrand::seeded(t[3].parse().unwrap());
                cc.mti = t[4].parse().unwrap();
                format!("{{\"b\":{}}}", breath_json(&breath))
            }
            "set" => {
                let param = BrParam::read(&unhex(t[1]));
                let disp = t[2] != "0";
                let trans = hexw(t[3]);
                let cnt: i16 = t[4].parse().unwrap();
                let w: Vec<u32> = t[5..21].iter().map(|s| hexw(s)).collect();
                let mut node: M4 = std::array::from_fn(|r| std::array::from_fn(|c| w[4 * r + c]));
                breath.set(&param, disp, trans, cnt, &mut node);
                format!("{{\"b\":{},\"node\":{}}}", breath_json(&breath), list(node.iter().flatten()))
            }
            "ctrl" => {
                let nl: usize = t[1].parse().unwrap();
                let land = t[2..2 + nl].iter().map(|s| hexw(s)).collect();
                let nw: usize = t[2 + nl].parse().unwrap();
                let walls = t[3 + nl..3 + nl + nw].iter().map(|s| hexw(s)).collect();
                let mut w = World { land, line: walls };
                let mut outs = Vec::new();
                breath.ctrl(&mut w, &mut cc, &mut outs);
                let o: Vec<String> = outs.iter().map(out_json).collect();
                format!("{{\"b\":{},\"out\":[{}],\"cc\":{}}}", breath_json(&breath), o.join(","), cc_state(&cc))
            }
            _ => "{}".to_string(),
        };
        writeln!(out, "{reply}").unwrap();
        out.flush().unwrap();
    }
}
