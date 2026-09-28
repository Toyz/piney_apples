//! The EE's single-precision arithmetic on `f32` values (truncating, no
//! denormals or infinities): `piney_data::anim::ee`, which tools/eemu.py
//! models, for the desktop's float code to come out bit for bit.

use piney_data::anim::ee;

pub fn add(a: f32, b: f32) -> f32 {
    f32::from_bits(ee::add(a.to_bits(), b.to_bits()))
}

pub fn sub(a: f32, b: f32) -> f32 {
    f32::from_bits(ee::sub(a.to_bits(), b.to_bits()))
}

pub fn mul(a: f32, b: f32) -> f32 {
    f32::from_bits(ee::mul(a.to_bits(), b.to_bits()))
}

pub fn div(a: f32, b: f32) -> f32 {
    f32::from_bits(ee::div(a.to_bits(), b.to_bits()))
}

/// cvt.s.w
pub fn from_int(i: i32) -> f32 {
    f32::from_bits(ee::from_int(i))
}

/// cvt.w.s / fptosi: truncate.
pub fn to_int(v: f32) -> i32 {
    ee::to_int(v.to_bits())
}

/// c.le.s, c.lt.s on the EE's values.
pub fn le(a: f32, b: f32) -> bool {
    ee::to_f64(a.to_bits()) <= ee::to_f64(b.to_bits())
}

pub fn lt(a: f32, b: f32) -> bool {
    ee::to_f64(a.to_bits()) < ee::to_f64(b.to_bits())
}
