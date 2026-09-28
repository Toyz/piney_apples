//! `LENSFLARE` (gcmn lensflare.cpp, 0x00503090-0x00503644): six `ccEff`s of
//! `town_z` strung on the line from a map's sun toward a point 2500 before
//! the eye, at the fractions `@973` (0.2 .. 0.8) of the way ([`points`]).
//! Dun Loireag, `ROOTTOWN04`, `EVENTAREA04`, `05`, `07` and the fields make
//! one; area 15's `EVENTAREA02` has its own copy ([`crate::evarea`]). Drawn
//! only when the sun is within 67.5 degrees of the camera's view (mode 0,
//! the towns) and no puppet show runs; mode 1 (the fields) takes the sun
//! through `P2W` (docs/engine/town02.md, "LENSFLARE").

use crate::ee::{self, F, V4};
use crate::evarea::FlareCamera;

/// The six `ccEff`s (+0x00 .. +0x14) in the order the constructor makes
/// them, of `town_z`.
pub const FLARES: [&str; 6] =
    ["EFF_sflenz_2", "EFF_sflenz_3", "EFF_sflenz_4", "EFF_sflenz_5", "EFF_sflenz_6", "EFF_sflenz_1"];
pub const FILE: &str = "town_z";
/// `@973` (gcmn 0x005ed7e0), as `EVENTAREA02`'s `@1260`: how far along from
/// the sun toward the point before the eye each flare stands.
const SCALE: [F; 6] = [0x3e4c_cccd, 0x3e99_999a, 0x3f00_0000, 0x3f0c_cccd, 0x3f40_0000, 0x3f4c_cccd];
/// -2500 (the point before the eye), 500 (its rise), -0.2 and -200.
const K_M2500: F = 0xc51c_4000;
const K500: F = 0x43fa_0000;
const K_M0_2: F = 0xbe4c_cccd;
const K_M200: F = 0xc348_0000;
/// `ccCheckCameraDeg(sun, 12288)`.
pub const VIEW_DEG: i16 = 12288;

/// Where the six flares are drawn for the sun at `sun` (w 1) and the
/// camera `cam`.
pub fn points(sun: V4, cam: &FlareCamera) -> [V4; 6] {
    std::array::from_fn(|i| {
        let m = piney_data::anim::rot_bits([cam.rot[0], cam.rot[1], cam.rot[2]]);
        let mut ahead = [0, K_M2500, 0, cam.view[3]];
        ahead = ee::apply(&m, ahead);
        ahead = ee::vadd(ahead, cam.eye);
        ahead[2] = ee::add(ahead[2], K500);
        let mut v = ee::vsub(sun, ahead);
        v[0] = ee::mul(v[0], K_M0_2);
        v[1] = ee::mul(v[1], K_M0_2);
        v[2] = 0;
        v = ee::vadd(v, ahead);
        v[2] = K_M200;
        let d = ee::vscale(ee::vsub(v, sun), SCALE[i]);
        ee::vadd(d, sun)
    })
}

/// `LENSFLARE::Draw(s, name, 0)`: the flares' places, or None when it draws
/// nothing. `sun_in_view` is `ccCheckCameraDeg(sun, 12288)`.
pub fn draw(sun: V4, cam: &FlareCamera, puppet_show: bool, sun_in_view: bool) -> Option<[V4; 6]> {
    (!puppet_show && sun_in_view).then(|| points(sun, cam))
}
