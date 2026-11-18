//! `libvu0`'s macro-mode routines on raw float bits, as the game's code runs
//! them on VU0: every lane through the EE's FPU model ([`crate::field::ee`]:
//! no denormals, no infinities, truncated), a multiply-add (`vmula`,
//! `vmadda`) rounding its product before the add, as eemu runs them. A
//! matrix is four stored rows (`sceVu0FMATRIX`), each a column of the maths.

use crate::field::ee::{add, div, mul, sqrt, sub};

/// A float as bits.
pub type F = u32;
/// A `sceVu0FVECTOR`: x, y, z, w as bits.
pub type V4 = [F; 4];
/// A `sceVu0FMATRIX`: four stored rows.
pub type M4 = [V4; 4];

pub const ONE: F = 0x3f80_0000;
/// `sceVu0UnitMatrix` (main 0x00110990).
pub const UNIT: M4 = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// A matrix's float bits as stored rows (each a column of the maths).
pub fn of_mat4(m: glam::Mat4) -> M4 {
    m.to_cols_array_2d().map(|c| c.map(f32::to_bits))
}

/// Stored rows of float bits as a matrix, the bits kept.
pub fn to_mat4(m: &M4) -> glam::Mat4 {
    glam::Mat4::from_cols_array_2d(&m.map(|c| c.map(f32::from_bits)))
}

/// `sceVu0MulMatrix` (main 0x00110698): each stored row of `b` through
/// `a`'s rows, ((a0 x + a1 y) + a2 z) + a3 w.
pub use crate::anim::vu_mul as mul_matrix;

/// `sceVu0AddVector` (0x00110888): every lane.
pub fn vadd(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| add(a[i], b[i]))
}

/// `sceVu0SubVector` (0x001108a0): every lane.
pub fn vsub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| sub(a[i], b[i]))
}

/// `sceVu0MulVector` (0x001108b8): every lane, `a` times `b`.
pub fn vmul(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| mul(a[i], b[i]))
}

/// `sceVu0ScaleVector` (0x001108d0): every lane times `s`.
pub fn vscale(a: V4, s: F) -> V4 {
    a.map(|x| mul(x, s))
}

/// `sceVu0ScaleVectorXYZ` (0x00111200): x, y, z times `s`, w kept.
pub fn vscale_xyz(a: V4, s: F) -> V4 {
    [mul(a[0], s), mul(a[1], s), mul(a[2], s), a[3]]
}

/// `sceVu0InnerProduct` (0x00110700): `vmul.xyz`, then x + y, then + z.
pub fn dot(a: V4, b: V4) -> F {
    add(add(mul(a[0], b[0]), mul(a[1], b[1])), mul(a[2], b[2]))
}

/// `sceVu0Normalize` (0x00110728): Q = sqrt(x x + y y + z z), then 1 / Q,
/// x, y, z times that; w 0.
pub fn normalize(a: V4) -> V4 {
    let q = sqrt(dot(a, a));
    let r = div(ONE, add(0, q));
    [mul(a[0], r), mul(a[1], r), mul(a[2], r), 0]
}

/// `sceVu0OuterProduct` (0x001106e0): `vopmula` then `vopmsub`, each
/// product rounded; w 0.
pub fn cross(a: V4, b: V4) -> V4 {
    [
        sub(mul(a[1], b[2]), mul(b[1], a[2])),
        sub(mul(a[2], b[0]), mul(b[2], a[0])),
        sub(mul(a[0], b[1]), mul(b[0], a[1])),
        0,
    ]
}

/// `sceVu0ApplyMatrix` (0x00110668): the stored rows `m` times `v`,
/// ((r0 x + r1 y) + r2 z) + r3 w.
pub fn apply(m: &M4, v: V4) -> V4 {
    std::array::from_fn(|i| {
        let acc = mul(m[0][i], v[0]);
        let acc = add(acc, mul(m[1][i], v[1]));
        let acc = add(acc, mul(m[2][i], v[2]));
        add(acc, mul(m[3][i], v[3]))
    })
}

/// `sceVu0TransposeMatrix` (0x00110768): the words moved, nothing computed.
pub fn transpose(m: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| m[c][r]))
}

/// `sceVu0TransMatrix(out, m, t)` (0x001108e8): `m` with `t`'s x, y, z
/// added to its translation (`vadd.xyz`), w kept.
pub fn trans_matrix(m: &M4, t: V4) -> M4 {
    let mut out = *m;
    for k in 0..3 {
        out[3][k] = add(m[3][k], t[k]);
    }
    out
}

/// `sceVu0InversMatrix` (main 0x001107b0): the transposed rotation and the
/// translation taken back through it, `-(r0 t.x + r1 t.y + r2 t.z)` as VU0
/// multiply-adds, w kept.
pub fn invers(m: &M4) -> M4 {
    let t = m[3];
    let rows: [V4; 3] = std::array::from_fn(|i| [m[0][i], m[1][i], m[2][i], 0]);
    let mut out = [rows[0], rows[1], rows[2], [0, 0, 0, t[3]]];
    for k in 0..3 {
        let acc = mul(rows[0][k], t[0]);
        let acc = add(acc, mul(rows[1][k], t[1]));
        let v = add(acc, mul(rows[2][k], t[2]));
        out[3][k] = sub(0, v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn k(x: f32) -> F {
        x.to_bits()
    }

    fn f(x: F) -> f32 {
        f32::from_bits(x)
    }

    #[test]
    fn vectors() {
        let a = [k(1.0), k(2.0), k(3.0), ONE];
        let b = [k(4.0), k(5.0), k(6.0), ONE];
        assert_eq!(f(dot(a, b)), 32.0);
        assert_eq!(cross(a, b).map(f), [-3.0, 6.0, -3.0, 0.0]);
        let n = normalize([k(3.0), k(4.0), 0, 0]);
        assert!((f(n[0]) - 0.6).abs() < 1e-6 && (f(n[1]) - 0.8).abs() < 1e-6);
        assert_eq!(vmul(a, b).map(f), [4.0, 10.0, 18.0, 1.0]);
    }

    /// A translation's inverse takes it back; the transpose is its own
    /// inverse; `TransMatrix` adds to the place.
    #[test]
    fn matrices() {
        let m = trans_matrix(&UNIT, [k(5.0), k(-7.0), k(2.0), 0]);
        assert_eq!(m[3].map(f), [5.0, -7.0, 2.0, 1.0]);
        let back = apply(&invers(&m), [k(5.0), k(-7.0), k(2.0), ONE]);
        assert_eq!(back.map(f)[..3], [0.0; 3]);
        let r = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]];
        assert_eq!(transpose(&transpose(&r)), r);
        assert_eq!(transpose(&r)[0], [1, 5, 9, 13]);
    }
}
