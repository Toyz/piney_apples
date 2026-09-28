//! The scenes' own sounds: what `ccSoundMain` (main 0x00181010) does after
//! each frame of its loop while `ccSnd.gameStart` (+0x17) is set, by `ccSnd
//! +0x105` ([`Driver::scene_mode`]): 1 Mac Anu's canals (`waterTest`
//! 0x0017bac0), 2 area 15's church music (`bgmChurch` 0x0017c3c0), 3 the
//! Grunty breeder's tune (`bgmBreed` 0x0017c6d0). The distances are
//! `ccGetDist` (main 0x001d9dd0): on the ground, the third lane zeroed, in the
//! EE's arithmetic. See docs/engine/sound.md ("The scenes' own sounds").

use piney_data::field::ee::{add, div, from_int, lt, mul, sub};
use piney_data::libm::sqrtf;
use piney_data::sound::Tables;

use crate::driver::{Command, Driver};
use crate::se3d::{self, V4};
use crate::stream::sq_play_vol;

/// What the scenes' sounds read of the game each frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneInput {
    /// The active camera's position (`activeCamPtr +0x00`); `None` with no
    /// camera (`activeCamPtr` not positive), when `waterTest` does
    /// nothing.
    pub camera: Option<V4>,
    /// Kite's position (`plw +0x40`).
    pub kite: V4,
    /// `game.block` (+0x30): 1 inside area 15's church.
    pub block: i32,
    /// A town is built (`WORLD_MAN +0x430`) and `GetTownType()` is 0: Mac
    /// Anu.
    pub mac_anu: bool,
    /// The town scene's `DMY_merchant6` (the Grunty breeder), which
    /// `bgmBreed` reads once.
    pub breeder: Option<V4>,
}

/// `seData` row 42: the canals (`seData+0x150`).
pub const WATER_SE: usize = 42;

const F_0: u32 = 0;
const F_10: u32 = 0x4120_0000;
const F_35: u32 = 0x420c_0000;
const F_100: u32 = 0x42c8_0000;
const F_256: u32 = 0x4380_0000;
const F_1000: u32 = 0x447a_0000;
const F_1200: u32 = 0x4496_0000;
const F_1700: u32 = 0x44d4_8000;
const F_3000: u32 = 0x453b_8000;
const F_3500: u32 = 0x455a_c000;
const F_M3000: u32 = 0xc53b_8000;
const F_M6300: u32 = 0xc5c4_e000;
const F_2_56: u32 = 0x4023_d70a;

/// `ccGetDist(a, b)`: `sqrtf` of the ground part of `b - a`.
fn get_dist(a: V4, b: V4) -> u32 {
    let x = sub(b[0], a[0]);
    let y = sub(b[1], a[1]);
    sqrtf(add(add(mul(x, x), mul(y, y)), mul(0, 0)))
}

/// A float's value truncated toward zero (`fptosi`).
fn to_int(v: u32) -> i32 {
    se3d::fptosi(v)
}

/// One frame of `ccSoundMain`'s scene sounds.
pub fn scene_sound(d: &mut Driver, tables: &Tables, s: &SceneInput, out: &mut Vec<Command>) {
    if !d.game_start {
        return;
    }
    match d.scene_mode {
        1 => water_test(d, tables, s, out),
        2 => bgm_church(d, s, out),
        3 => bgm_breed(d, s, out),
        _ => {}
    }
}

/// `waterTest` (0x0017bac0), in Mac Anu. The volume is the camera's ground
/// distance from y = 0: full within 1700, then fading out over `decay * 3000
/// / 256` more, with a floor of 48 in the west and south and never above the
/// row's velocity. The first frame starts the loop silent in a free slot
/// ([`se3d::tobj_se_loop_start`]) and sets `ccSnd +0x132`; every later one
/// sends `FD 01 ch note id 60 00` and `FD 00 ch note id vol 00`.
fn water_test(d: &mut Driver, tables: &Tables, s: &SceneInput, out: &mut Vec<Command>) {
    let Some(cam) = s.camera else { return };
    if !s.mac_anu {
        return;
    }
    let Some(se) = tables.se.get(WATER_SE) else { return };
    let dist = get_dist([0, 0, 0x3f80_0000, 0], [0, cam[1], 0, cam[3]]);
    let reach = div(mul(F_3000, from_int(i32::from(se.decay))), F_256);
    let near = lt(dist, F_1700);
    let past = if near { F_0 } else { sub(dist, F_1700) };
    let mut f = sub(reach, past);
    if lt(f, F_0) {
        f = F_0;
    }
    let f = div(mul(from_int(i32::from(se.velocity)), div(f, F_10)), F_100);
    let mut vol = to_int(f);
    if vol <= 0 {
        vol = 0;
    }
    if vol >= 128 {
        vol = 127;
    }
    if lt(cam[0], F_M3000) && !near && vol < 48 {
        vol = 48;
    }
    if lt(cam[1], F_M6300) && lt(cam[0], F_0) && !near {
        vol = 48;
    }
    if i32::from(se.velocity) < vol {
        vol = i32::from(se.velocity);
    }
    let (ch, note) = (se.ch as u8, se.note as u8);
    if !d.water_loop {
        let Some((id, m)) = se3d::tobj_se_loop_start(&mut d.loop_id, se) else { return };
        out.push(Command::Msg(m));
        d.water_loop = true;
        d.looptest = id;
        return;
    }
    let id = d.looptest as u8;
    out.push(Command::Msg(vec![0xfd, 1, ch, note, id, 60, 0, 0xfd, 0, ch, note, id, vol as u8, 0]));
}

/// `bgmChurch` (0x0017c3c0), with area 15's event bank. The first frame
/// starts sequence 0 silent (`ccSqPlayVol(0, 0)`) and tells SNDBASE play
/// type 0. Entering the church (block 1): port 2 at full, SNDBASE's
/// `bgmChange`, sequence 1 on, sequence 0 fading out over 30 frames and
/// stopping; leaving it, `bgmChange`, sequence 0 on, sequence 1 fading out.
/// Outside, port 1's volume is the camera's ground distance from y 3500
/// (x and z zeroed): `max(3500 - d, 0) / 35 * 2.56`, under `bgmVol`, sent
/// when it changes.
fn bgm_church(d: &mut Driver, s: &SceneInput, out: &mut Vec<Command>) {
    let block = s.block;
    if !d.scene_bgm {
        sq_play_vol(d, 0, 0, out);
        d.scene_bgm = true;
        d.church_block = block;
        out.push(Command::PlayType(0));
    }
    if d.church_block != block {
        if block == 1 {
            let v = ((256 * d.bgm_vol) >> 8) as u16;
            d.port_vol[2] = v;
            out.push(Command::PortVolume(2, v));
            out.push(Command::BgmChange);
            d.sq_status[1] = 1;
            d.sq_fade(0, 0, 30, 3);
        } else {
            out.push(Command::BgmChange);
            d.sq_status[0] = 1;
            d.sq_fade(1, 0, 30, 3);
        }
    }
    if block != 1 {
        let cam = s.camera.unwrap_or([0, 0, 0, 0]);
        let dist = get_dist([0, F_3500, 0, 0], [0, cam[1], 0, cam[3]]);
        let mut f = sub(F_3500, dist);
        if lt(f, F_0) {
            f = F_0;
        }
        let v = (to_int(mul(F_2_56, div(f, F_35))) as u32 & 0xffff) as i32;
        let v = ((v * d.bgm_vol) >> 8) as u16;
        if d.port_vol[1] != v {
            d.port_vol[1] = v;
            out.push(Command::PortVolume(1, v));
        }
    }
    d.church_block = block;
}

/// `bgmBreed` (0x0017c6d0), in a town other than Mac Anu: the breeder's
/// position is read once (`ccSnd +0x114`, +0x120). Kite coming within 1000
/// of it fades sequence 0 out (and stops it) and brings sequence 1 in from
/// silence over 30 frames; going beyond 1200 does the reverse.
fn bgm_breed(d: &mut Driver, s: &SceneInput, out: &mut Vec<Command>) {
    if d.breeder.is_none() {
        d.breeder = s.breeder;
    }
    let Some(at) = d.breeder else { return };
    let dist = get_dist(at, s.kite);
    if !d.scene_bgm {
        if !lt(dist, F_1000) {
            return;
        }
        d.sq_fade(0, 0, 30, 3);
        sq_play_vol(d, 1, 0, out);
        d.sq_fade(1, 256, 30, 1);
        d.scene_bgm = true;
    } else {
        if !lt(F_1200, dist) {
            return;
        }
        d.sq_fade(1, 0, 30, 3);
        sq_play_vol(d, 0, 0, out);
        d.sq_fade(0, 256, 30, 1);
        d.scene_bgm = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f32) -> u32 {
        v.to_bits()
    }

    fn driver(mode: i8) -> Driver {
        let mut d = Driver::new();
        d.game_start = true;
        d.scene_mode = mode;
        d.sq_num = 2;
        d.sqtbl[0].vol = 200;
        d.sqtbl[1].vol = 180;
        d
    }

    fn cam(x: f32, y: f32) -> Option<V4> {
        Some([f(x), f(y), f(500.0), f(1.0)])
    }

    /// Mac Anu's canals: started silent once, then pan 60 and a volume by
    /// the camera's distance from y 0.
    #[test]
    fn the_canals() {
        let t = &piney_data::sound::INF;
        let mut d = driver(1);
        let mut out = Vec::new();
        let at = |y| SceneInput { camera: cam(0.0, y), mac_anu: true, ..SceneInput::default() };
        scene_sound(&mut d, t, &at(0.0), &mut out);
        assert!(d.water_loop);
        let Command::Msg(m) = &out[0] else { panic!("{out:?}") };
        assert_eq!(&m[2..4], &[0xfd, 0x10], "the note on, silent");
        let vol = |d: &mut Driver, y: f32| {
            let mut out = Vec::new();
            scene_sound(d, t, &at(y), &mut out);
            let Some(Command::Msg(m)) = out.first() else { panic!("{out:?}") };
            assert_eq!(m[5], 60);
            m[12]
        };
        let se = t.se[WATER_SE];
        let near = vol(&mut d, 1000.0);
        assert!(near > 0 && i32::from(near) <= i32::from(se.velocity));
        let far = vol(&mut d, 100_000.0);
        assert_eq!(far, 0);
        // Not Mac Anu: nothing.
        let mut out = Vec::new();
        scene_sound(&mut d, t, &SceneInput { camera: cam(0.0, 0.0), ..SceneInput::default() }, &mut out);
        assert!(out.is_empty());
    }

    /// Area 15: the approach's music by the camera's distance from the
    /// church, then the church's own inside.
    #[test]
    fn the_church() {
        let t = &piney_data::sound::INF;
        let mut d = driver(2);
        let at = |y, block| SceneInput { camera: cam(0.0, y), block, ..SceneInput::default() };
        let mut out = Vec::new();
        scene_sound(&mut d, t, &at(-10_000.0, 0), &mut out);
        assert!(d.scene_bgm);
        assert!(out.contains(&Command::PlayType(0)));
        assert_eq!(d.port_vol[1], 0, "out of reach: silent");
        let mut out = Vec::new();
        scene_sound(&mut d, t, &at(3500.0, 0), &mut out);
        // 100 * 2.56 in the EE's floats is just under 256.
        assert!(out.contains(&Command::PortVolume(1, 255)), "{out:?}");
        let mut out = Vec::new();
        scene_sound(&mut d, t, &at(3500.0, 1), &mut out);
        assert!(out.contains(&Command::BgmChange));
        assert_eq!((d.sq_status[1], d.fade[0].sw, d.fade[0].time), (1, 3, 30));
        let mut out = Vec::new();
        scene_sound(&mut d, t, &at(3500.0, 0), &mut out);
        assert!(out.contains(&Command::BgmChange));
        assert_eq!((d.sq_status[0], d.fade[1].sw), (1, 3));
    }

    /// Dun Loireag's breeder: within 1000 the tune in, beyond 1200 out.
    #[test]
    fn the_breeder() {
        let t = &piney_data::sound::INF;
        let mut d = driver(3);
        let breeder = Some([f(1000.0), f(1000.0), 0, f(1.0)]);
        let at = |x: f32| SceneInput { kite: [f(x), f(1000.0), 0, f(1.0)], breeder, ..SceneInput::default() };
        let mut out = Vec::new();
        scene_sound(&mut d, t, &at(-500.0), &mut out);
        assert!(!d.scene_bgm && out.is_empty());
        scene_sound(&mut d, t, &at(100.0), &mut out);
        assert!(d.scene_bgm);
        assert_eq!((d.fade[0].sw, d.fade[1].sw, d.fade[1].end_vol), (3, 1, 180));
        // Between 1000 and 1200 it stays.
        let mut out = Vec::new();
        scene_sound(&mut d, t, &at(-100.0), &mut out);
        assert!(d.scene_bgm && out.is_empty());
        scene_sound(&mut d, t, &at(-300.0), &mut out);
        assert!(!d.scene_bgm);
        assert_eq!((d.fade[1].sw, d.fade[0].sw), (3, 1));
    }

    #[test]
    fn ground_distance_ignores_height() {
        let d = get_dist([f(0.0), f(3500.0), f(99.0), 0], [f(0.0), f(500.0), f(-7.0), 0]);
        assert_eq!(f32::from_bits(d), 3000.0);
    }
}
