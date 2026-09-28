//! `ccParticleForceField::Calc(p)` (main 0x001bcde0): one force field of a
//! particle's generator acting on the particle, every frame before it moves,
//! by `forceType` (the jump table at 0x003741f0, 17 cases) and `calcType`.
//! For type 1 a negative `force.x` brings the speed to a stop and answers 1
//! (the particle fades out); `fieldType` 3 ends a particle within two frames'
//! flight of the generator. The cases are in docs/engine/particles.md.

use super::dmath;
use super::one::Particle;
use super::tables::FfParam;
use super::{Generator, normal2angle};
use crate::ee::{self, F, ONE, V4};
use crate::vu;

/// `oneVector` (main 0x0033ef80, particle.cpp's own): (1, 1, 1, 0).
const ONE_VECTOR: V4 = [ONE, ONE, ONE, 0];
/// 0.01 and -0.01 (force 16).
const HUNDREDTH: F = 0x3c23_d70a;

/// The squash `forceType` 6 and 9 apply: (w, 1, 1, 0) for w below 1,
/// else (1, 1 / w, 1, 0).
fn squash(w: F) -> V4 {
    let mut t = ONE_VECTOR;
    if ee::lt(w, ONE) {
        t[0] = w;
    } else {
        t[1] = ee::div(ONE, w);
    }
    t
}

fn vmul(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| ee::mul(a[i], b[i]))
}

/// `Calc(p)`: true when the particle should fade out now (`Main` sets
/// its `lifeTime` -1 and `fadeFlag` 2).
pub fn calc(ff: &FfParam, p: &mut Particle, gene: Option<&Generator>) -> bool {
    use ee::{add, div, from_int, mul, sub};
    if ff.field_type > 3 {
        return false;
    }
    let f = ff.force;
    let one = ff.calc_type == 1;
    match ff.force_type {
        0 => {
            if one {
                p.pos = ee::vadd(p.pos, ee::vscale(p.dirc, f[0]));
            } else {
                p.pos = ee::vadd(p.pos, f);
            }
        }
        1 => {
            if one {
                p.speed = add(p.speed, f[0]);
                p.velocity = ee::vadd(p.velocity, ee::vscale(p.dirc, f[0]));
            } else {
                p.velocity = ee::vadd(p.velocity, f);
            }
        }
        2 => {
            if one {
                p.rotate[1] = p.rotate[1].wrapping_add(ff.rotate);
                let a = ee::deg2rad(p.rotate[1] as i16);
                p.offset[0] = mul(ff.radius, ee::cosf(a));
                p.offset[1] = mul(ff.radius, ee::sinf(a));
            } else {
                let centre = if p.sync_flag { [0; 4] } else { gene.map_or([0; 4], |g| g.pos) };
                let mut t = ee::vsub(centre, p.pos);
                t[2] = 0;
                p.rotate[1] = p.rotate[1].wrapping_add(ff.rotate);
                let a = ee::rad2deg(ee::atan2f(t[1], t[0]));
                let a = ee::deg2rad(p.rotate[1].wrapping_add(a as u16) as i16);
                let d = ee::sqrtf(ee::dot(t, t));
                p.offset[0] = add(t[0], mul(d, ee::cosf(a)));
                p.offset[1] = add(t[1], mul(d, ee::sinf(a)));
            }
        }
        3 => {
            if p.style == 1 {
                for (r, &f) in p.rot.iter_mut().zip(&f).take(3) {
                    let d = i32::from(ee::rad2deg(*r)) + i32::from(ee::rad2deg(f));
                    *r = ee::deg2rad(d as i16);
                }
            }
        }
        4 => p.pos = ee::vadd(p.pos, f),
        6..=8 => {
            let mut s = from_int(i32::from(p.cnt));
            if !ee::le(s, f[2]) {
                s = f[2];
            }
            s = add(div(mul(s, sub(f[1], f[0])), f[2]), f[0]);
            match ff.force_type {
                6 => {
                    p.scale = ee::vscale(ONE_VECTOR, s);
                    p.scale = ee::vscale(p.scale, p.size);
                    p.scale = vmul(p.scale, squash(f[3]));
                    p.scale[3] = ONE;
                }
                7 => p.scale[0] = mul(p.size, s),
                _ => p.scale[1] = mul(p.size, s),
            }
        }
        9 => {
            p.scale = vmul(f, squash(f[3]));
            p.scale[3] = ONE;
        }
        10 => {
            // A force.w that is not 0 leaves the scale the caller's f20,
            // which no table row does; the port takes 1.
            let mut s = ONE;
            if ee::to_int(f[3]) == 0 {
                let cnt = i32::from(p.cnt);
                let h = ee::to_int(f[2]) / 2;
                let (q, mut r) = if h == 0 { (0, cnt) } else { (cnt / h, cnt % h) };
                if q & 1 != 0 {
                    r = h - r;
                }
                s = add(f[0], div(mul(from_int(r), sub(f[1], f[0])), f[2]));
            }
            p.scale = ee::vscale(ONE_VECTOR, s);
            p.scale[3] = ONE;
        }
        11 => p.rotate[3] = p.rotate[3].wrapping_add(ff.rotate),
        12 => p.rotate[3] = ff.rotate,
        13 => {
            p.rot = f;
            p.rot[3] = ONE;
        }
        14 => {
            if let Some(g) = gene {
                let t = ee::normalize(ee::vsub(g.pos2, g.pos));
                p.pos = ee::vscale_xyz(t, p.temp);
                p.pos[3] = ONE;
            }
        }
        15 => {
            let i = ee::to_int;
            let n = i(add(f[3], add(f[2], add(f[0], f[1]))));
            let a = i(f[0]);
            let b = a.wrapping_add(i(f[1]));
            let c = b.wrapping_add(i(f[2]));
            let s = if n == 0 { i32::from(p.cnt) } else { i32::from(p.cnt) % n };
            p.transparency = if i(f[0]) != 0 && s < a {
                div(from_int(s), f[0])
            } else if i(f[1]) != 0 && s < b {
                ONE
            } else if i(f[2]) != 0 && s < c {
                div(sub(f[2], from_int(s - b)), f[2])
            } else {
                0
            };
        }
        16 => {
            if let (true, Some(g)) = (one, gene) {
                let line = ee::vsub(g.pos2, g.pos);
                let d = ee::sqrtf(ee::dot(line, line));
                let axis = ee::normalize(line);
                let ang = normal2angle(axis);
                let m = vu::rot_z(&vu::rot_x(&vu::UNIT, ang[0]), ang[2]);
                let r = g.param.map_or(0, |p| p.g_radius);
                let mut t = [0, 0, 0, ONE];
                t[0] = mul(HUNDREDTH, mul(from_int(i32::from(p.ofst_r)), r));
                t[1] = mul(HUNDREDTH | 0x8000_0000, mul(d, from_int(i32::from(p.ofst_d))));
                let t = ee::apply(&m, t);
                p.rotate[1] = p.rotate[1].wrapping_add(ff.rotate);
                let q = dmath::set_quaternion(axis, ee::deg2rad(p.rotate[1] as i16));
                p.pos = ee::apply(&dmath::quaternion_to_matrix(q), t);
                p.pos[3] = ONE;
            }
        }
        _ => {}
    }
    if ff.force_type == 1 {
        if ee::lt(f[0], 0) && ee::le(p.speed, 0) {
            p.speed = 0;
            p.velocity = [0; 4];
            return true;
        }
        if ff.field_type == 3
            && let Some(g) = gene
        {
            let t = ee::vsub(p.pos, g.pos);
            let d = ee::sqrtf(ee::dot(t, t));
            let v = ee::sqrtf(ee::dot(p.velocity, p.velocity));
            if ee::le(d, mul(0x4000_0000, v)) {
                p.end_flag = true;
            }
        }
    }
    false
}
