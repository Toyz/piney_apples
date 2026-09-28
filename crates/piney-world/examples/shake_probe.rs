//! Answers `tools/test_camera_shake_rs.py`: the screen shake
//! (`piney_world::camera::Shake`: `cameraShake`, `cameraShockAbsorber`). One
//! request a line, one JSON line back with the state after it: `seed S`
//! (newlib's `rand()` state, decimal), `shake POWER CYCLE TIME DIRC`, `absorb`,
//! and `set P0 P1 P2 V0 V1 V2` (`cameraSet`'s eye and target for camera 1).

use std::io::BufRead;

use piney_battle::rand::{Rand, Rng};
use piney_world::camera::Shake;

fn main() {
    let mut shake = Shake::default();
    let mut rand = Rand::default();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| -> i64 { w[i].parse().unwrap() };
        match w.first().copied() {
            Some("seed") => rand = Rand::new(n(1) as u64),
            Some("shake") => shake.shake(n(1) as i32, n(2) as i32, n(3) as i32, n(4) as i32, &mut || rand.rand()),
            Some("absorb") => shake.absorb(),
            Some("set") => {
                let v = |i: usize| -> [u32; 4] { [n(i) as u32, n(i + 1) as u32, n(i + 2) as u32, 0x3f80_0000] };
                let (e, t) = shake.apply(v(1), v(4));
                let l = |a: [u32; 4]| a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
                println!("[[{}],[{}]]", l(e), l(t));
                continue;
            }
            _ => {}
        }
        let sf: Vec<String> =
            shake.sf.iter().map(|s| format!("[{},{},{},{},{}]", s.power, s.cycle, s.rot, s.time, s.dirc)).collect();
        let m: Vec<String> = shake.matrix.iter().flatten().map(|x| x.to_string()).collect();
        println!(
            "{{\"sf\":[{}],\"force\":{},\"cycle\":{},\"rotate\":{},\"matrix\":[{}],\"z\":{},\"osc\":{},\"rand\":{}}}",
            sf.join(","),
            shake.force,
            shake.cycle,
            shake.rotate,
            m.join(","),
            shake.offset_z,
            shake.oscillator,
            rand.0
        );
    }
}
