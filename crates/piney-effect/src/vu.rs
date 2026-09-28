//! Sony's `libvu0` matrix routines as the effects call them, on raw float
//! bits so that matrices come out as the game builds them: the unit matrix,
//! `sceVu0RotMatrixX/Y/Z`, `sceVu0TransMatrix`, `sceVu0MulMatrix`, and
//! `ccCoord::SetMatrix_PosRotXYZScale` / `SetMatrix_PosRotZYXScale` (main
//! 0x00138050, 0x00138120) that place an effect's clump or animation.
//!
//! Matrices are stored columns (`m[column][row]`), as the EE keeps them.

use piney_data::anim::ee;
use piney_data::anim::vu_mul;

use crate::ee::{F, ONE, V4};

pub type M4 = [V4; 4];

/// `sceVu0UnitMatrix`.
pub const UNIT: M4 = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

const HALF_PI: F = 0x3fc9_0fdb;
const SIN: [F; 4] = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];

/// `_sceVu0ecossin` (0x001109b8) as `sceVu0RotMatrixX/Y/Z` use it: cos an
/// odd polynomial in pi/2 - |angle|, sin +-sqrt(1 - cos^2); (sin, cos).
pub fn cossin(angle: F) -> (F, F) {
    let neg = ee::negative(angle);
    let x = if neg { ee::add(HALF_PI, angle) } else { ee::sub(HALF_PI, angle) };
    let x2 = ee::mul(x, x);
    let mut p = SIN.map(|k| ee::mul(ee::mul(k, x), x2));
    for v in &mut p[..3] {
        *v = ee::mul(*v, x2);
    }
    let mut r = ee::add(x, p[3]);
    for v in &mut p[..2] {
        *v = ee::mul(*v, x2);
    }
    r = ee::add(r, p[2]);
    p[0] = ee::mul(p[0], x2);
    r = ee::add(ee::add(r, p[1]), p[0]);
    let q = ee::sqrt(ee::sub(ONE, ee::mul(r, r)));
    (if neg { ee::sub(0, q) } else { ee::add(0, q) }, r)
}

fn z(v: F) -> F {
    ee::add(0, v)
}

fn n(v: F) -> F {
    ee::sub(0, v)
}

/// `sceVu0RotMatrixX(out, m, rx)`: `Rx * m`.
pub fn rot_x(m: &M4, a: F) -> M4 {
    let (s, c) = cossin(a);
    vu_mul(&[[ONE, 0, 0, 0], [0, z(c), z(s), 0], [0, n(s), z(c), 0], [0, 0, 0, ONE]], m)
}

/// `sceVu0RotMatrixY(out, m, ry)`: `Ry * m`.
pub fn rot_y(m: &M4, a: F) -> M4 {
    let (s, c) = cossin(a);
    vu_mul(&[[z(c), 0, n(s), 0], [0, ONE, 0, 0], [z(s), 0, z(c), 0], [0, 0, 0, ONE]], m)
}

/// `sceVu0RotMatrixZ(out, m, rz)`: `Rz * m`.
pub fn rot_z(m: &M4, a: F) -> M4 {
    let (s, c) = cossin(a);
    vu_mul(&[[z(c), z(s), 0, 0], [n(s), z(c), 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]], m)
}

/// `sceVu0RotMatrix(out, m, rot)`: z, then y, then x.
pub fn rot_zyx(m: &M4, rot: V4) -> M4 {
    rot_x(&rot_y(&rot_z(m, rot[2]), rot[1]), rot[0])
}

/// `sceVu0TransMatrix(out, m, tv)`: the translation column's x, y and z
/// plus `tv`'s.
pub fn trans(m: &M4, tv: V4) -> M4 {
    let mut o = *m;
    for k in 0..3 {
        o[3][k] = ee::add(o[3][k], tv[k]);
    }
    o
}

/// `sceVu0MulMatrix(out, a, b)`: `a * b`.
pub fn mul(a: &M4, b: &M4) -> M4 {
    vu_mul(a, b)
}

/// The unit matrix with `scale`'s x, y and z on the diagonal.
fn scaled(scale: V4) -> M4 {
    let mut m = UNIT;
    m[0][0] = scale[0];
    m[1][1] = scale[1];
    m[2][2] = scale[2];
    m
}

/// `ccCoord::SetMatrix_PosRotXYZScale(pos, rot, scale)` (main 0x00138050):
/// the scale, then x, y and z turns on the left, then the position.
pub fn pos_rot_xyz_scale(pos: V4, rot: V4, scale: V4) -> M4 {
    let m = rot_z(&rot_y(&rot_x(&scaled(scale), rot[0]), rot[1]), rot[2]);
    trans(&m, pos)
}

/// `ccCoord::SetMatrix_PosRotZYXScale(pos, rot, scale)` (main 0x00138120):
/// the scale, then `sceVu0RotMatrix` (z, y, x), then the position.
pub fn pos_rot_zyx_scale(pos: V4, rot: V4, scale: V4) -> M4 {
    trans(&rot_zyx(&scaled(scale), rot), pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quarter_turn_about_z() {
        let f = crate::ee::f;
        let m = rot_z(&UNIT, crate::ee::k(std::f32::consts::FRAC_PI_2));
        assert!(f(m[0][0]).abs() < 1e-6 && (f(m[0][1]) - 1.0).abs() < 1e-6);
        let t = pos_rot_xyz_scale([ONE, 0, 0, ONE], [0; 4], [0x4000_0000, ONE, ONE, ONE]);
        assert_eq!(t[0][0], 0x4000_0000);
        assert_eq!(t[3], [ONE, 0, 0, ONE]);
    }
}
