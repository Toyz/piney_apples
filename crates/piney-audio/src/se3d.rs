//! The positioned sound effects (`sndlib.cpp`, `INF SLUS_202.67`): a
//! sound effect at a point of the world, its velocity from the distance to
//! the camera (`calcVel`) and its pan from the direction of the point
//! against the camera's (`calcPan`); and the sounds of the characters'
//! animation notes (`ccSeSetParamSPC`, `ccSeSetParamPC`,
//! `ccSeSetParamEnemy`, `ccSeSetParamInu`), which pick a sound effect from
//! setbl.cpp's tables ([`crate::setbl`]) and play it at the character.
//!
//! Floats are raw `u32` bit patterns computed with the EE's rules
//! ([`piney_data::field::ee`], [`piney_data::libm`]), so the velocity and
//! pan are the game's to the bit. `tools/test_sound3d_rs.py` checks every
//! function here against the game's own code run in eemu.

use std::cmp::Ordering;

use piney_data::field::ee::{add, cmp, div, from_int, lt, mul, sub, to_int};
use piney_data::libm::{atan2f, sinf, sqrtf};
use piney_data::sound::SeTbl;

use crate::setbl;

/// A float's bits.
pub type F = u32;
/// A `float[4]`: x and y on the ground, z up.
pub type V4 = [F; 4];

const F_5500: F = 0x45ab_e000;
const F_256: F = 0x4380_0000;
const F_10: F = 0x4120_0000;
const F_100: F = 0x42c8_0000;
const F_63: F = 0x427c_0000;
const PI: F = 0x4049_0fdb;
const HALF_TURN: F = 0x4700_0000;

/// What `calcVel` and `calcPan` read of the active camera
/// (`activeCamPtr`, 0x0037896c, a `CAMERA`). The field's is
/// `piney_world::camera::Camera::active()`: its `pos`, `view` and `kind`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Listener {
    /// `CAMERA.pos` (+0x00): the eye. x, y and z are read.
    pub pos: V4,
    /// `CAMERA.view` (+0x10): the point it looks at. x and y are read.
    pub view: V4,
    /// `CAMERA.type` (+0x5c, `checkCameraType()`): 1 is the eye view.
    pub kind: i32,
}

/// An `SE_NT` row of setbl.cpp (gcmn.prg): `code` the `seData` row, -1 for
/// none; `note` the note to play it at, 0 for the row's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeNt {
    pub code: i32,
    pub note: i8,
    /// Read by nothing.
    pub velocity: i8,
}

/// The call a note's sound ends in: `ccSeOn3D(code, pos)` when `note` is
/// `None`, `ccSeOn3DNote(code, pos, note)` otherwise; `pos` the
/// character's (`ccChar.pos`, +0x40).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteSe {
    pub code: usize,
    pub note: Option<i8>,
}

/// libgcc's `__fixsfsi` (`fptosi`, 0x00129c18): truncate, saturate at the
/// int range; an exponent of 0 is 0.
pub(crate) fn fptosi(v: F) -> i32 {
    if (v >> 23) & 0xff == 0 { 0 } else { to_int(v) }
}

/// `RAD2DEG` (0x001dab50): radians to the game's 16-bit angle (32768 is
/// pi), `(short)(int)(32768 (pi + r) / pi - 32768)`.
pub fn rad2deg(r: F) -> i16 {
    fptosi(sub(div(mul(HALF_TURN, add(PI, r)), PI), HALF_TURN)) as i16
}

/// `DEG2RAD` (0x001dabb0): `pi * s / 32768`.
pub fn deg2rad(s: i16) -> F {
    div(mul(PI, from_int(i32::from(s))), HALF_TURN)
}

/// `calcVel(pos, se)` (0x0017a370): 127 with no camera; else, `d` the
/// distance from the camera's eye (`sqrtf` of the three squares),
/// `(int)(velocity * (max(decay * 5500 / 256 - d, 0) / 10) / 100)`, -1
/// when that is 0 or less, at most 127.
pub fn calc_vel(se: &SeTbl, cam: Option<&Listener>, pos: &V4) -> i32 {
    let Some(c) = cam else { return 127 };
    let x = sub(pos[0], c.pos[0]);
    let y = sub(pos[1], c.pos[1]);
    let z = sub(pos[2], c.pos[2]);
    // mul.s, mul.s, adda.s, madd.s: the product rounded before the add.
    let dist = sqrtf(add(add(mul(x, x), mul(y, y)), mul(z, z)));
    let reach = div(mul(F_5500, from_int(i32::from(se.decay))), F_256);
    let mut per = sub(reach, dist);
    if lt(per, 0) {
        per = 0;
    }
    let v = fptosi(div(mul(from_int(i32::from(se.velocity)), div(per, F_10)), F_100));
    if v <= 0 { -1 } else { v.min(127) }
}

/// `calcPan(pos)` (0x0017a4c0), as (pan, cdeg): `cdeg` the angle of the
/// point seen from the eye less the angle of the view (`atan2f` of the x-y
/// offsets, each through [`rad2deg`]), in 16-bit units; pan `(int)(63 - 63
/// sin(cdeg))`, 0 to the left, 126 to the right. (63, 0) with no camera,
/// and in the eye view (type 1) when the point's angle is exactly 0.
pub fn calc_pan(cam: Option<&Listener>, pos: &V4) -> (i32, u16) {
    let Some(c) = cam else { return (63, 0) };
    let deg = atan2f(sub(pos[1], c.pos[1]), sub(pos[0], c.pos[0]));
    if cmp(0, deg) == Ordering::Equal && c.kind == 1 {
        return (63, 0);
    }
    let deg2 = rad2deg(deg);
    let cam_deg = rad2deg(atan2f(sub(c.view[1], c.pos[1]), sub(c.view[0], c.pos[0])));
    let cdeg = (i32::from(deg2) - i32::from(cam_deg)) as u16;
    let s = sinf(deg2rad(cdeg as i16));
    (fptosi(sub(F_63, mul(F_63, s))), cdeg)
}

/// Whether `cdeg` is behind the camera: 16385 to 49151 (past a quarter
/// turn either way).
pub fn behind(cdeg: u16) -> bool {
    (16385..0xc000).contains(&cdeg)
}

/// What the 3D forms send to port 0, in order: the program change
/// (`sceMSIn_PutMsg`); `F9 01 ch pan 00`, the pan of the next note on
/// (`sceMSIn_PutHsMsg`); behind the camera `F9 02 ch 120 63`, its bend
/// (8184, 8 below the centre); then the note on `FD 10 ch note id
/// velocity 00`.
fn messages(se: &SeTbl, pan: i32, cdeg: u16, note: u8, id: u8, vel: i32) -> Vec<u8> {
    let ch = se.ch as u8;
    let mut m = vec![0xc0 | ch, (se.prog as u8) & 0x7f, 0xf9, 1, ch, pan as u8, 0];
    if behind(cdeg) {
        m.extend([0xf9, 2, ch, 120, 63]);
    }
    m.extend([0xfd, 0x10, ch, note, id, vel as u8, 0]);
    m
}

/// `ccSeOn3D(n, pos)` (0x00179d90), or with `note` `ccSeOn3DNote(n, pos,
/// note)` (0x00179f50): the velocity [`calc_vel`] at most the row's, the
/// pan [`calc_pan`], the note on with id 0. Nothing when the point is out
/// of reach (velocity -1), or for a note below 0. The velocity is also
/// `ccSound.testVelocity` (+0x148) in `ccSeOn3D`, which nothing reads.
pub fn se_on_3d(se: &SeTbl, cam: Option<&Listener>, pos: &V4, note: Option<i8>) -> Vec<u8> {
    if note.is_some_and(|k| k < 0) {
        return Vec::new();
    }
    let vel = calc_vel(se, cam, pos).min(i32::from(se.velocity));
    if vel < 0 {
        return Vec::new();
    }
    let (pan, cdeg) = calc_pan(cam, pos);
    messages(se, pan, cdeg, note.unwrap_or(se.note) as u8, 0, vel)
}

/// `ccSeOn3DLoop(n, pos)` (0x0017a140): a looping effect. Out of reach
/// (-1 from [`calc_vel`], whose velocity is not capped by the row's here)
/// or with the eight slots of `ccSound.loopID` (+0x65) all taken, nothing
/// and -1; else the first free slot (-1) takes its own index, which is the
/// note on's id and what is returned.
pub fn se_on_3d_loop(loop_id: &mut [i8; 8], se: &SeTbl, cam: Option<&Listener>, pos: &V4) -> (i32, Vec<u8>) {
    let vel = calc_vel(se, cam, pos);
    if vel == -1 {
        return (-1, Vec::new());
    }
    let (pan, cdeg) = calc_pan(cam, pos);
    let Some(id) = loop_id.iter().position(|&x| x == -1) else { return (-1, Vec::new()) };
    loop_id[id] = id as i8;
    (id as i32, messages(se, pan, cdeg, se.note as u8, id as u8, vel))
}

/// `ccSeOffLoop(n, id)` (0x0017a620): `FD 10 ch note id 00 00`, the note
/// off of loop `id`, and the slot freed. The game frees any `id` without a
/// check (writing outside `loopID` past 7); here only 0-7 are freed.
pub fn se_off_loop(loop_id: &mut [i8; 8], se: &SeTbl, id: i32) -> Vec<u8> {
    if let Some(s) = usize::try_from(id).ok().and_then(|i| loop_id.get_mut(i)) {
        *s = -1;
    }
    vec![0xfd, 0x10, se.ch as u8, se.note as u8, id as u8, 0, 0]
}

/// The `seData` row of TOBJ's hum (`seData+0x160`): program 31 on channel
/// 15, note 60, velocity 64, decay 128.
pub const TOBJ_SE: usize = 44;

const F_15000: F = 0x466a_6000;

/// `tobjSeLoopStart(pos)` (0x0017bf20), from `TOBJ::Init` for Δ's airship
/// and Λ's whale: nothing when `ccSnd +0x133` is already set or the eight
/// slots of `loopID` are taken; else the first free slot takes its index
/// and the loop starts silent - the program change (`sceMSIn_PutMsg`),
/// the note on `FD 10 ch note id 127 00`, then its expression to 0 (`FD
/// 00 ch note id 00 00`). The slot is returned, for `looptest`; `pos` is
/// not read.
pub fn tobj_se_loop_start(loop_id: &mut [i8; 8], se: &SeTbl) -> Option<(i32, Vec<u8>)> {
    let id = loop_id.iter().position(|&x| x == -1)?;
    loop_id[id] = id as i8;
    let (ch, note, id8) = (se.ch as u8, se.note as u8, id as u8);
    let m = vec![0xc0 | ch, (se.prog as u8) & 0x7f, 0xfd, 0x10, ch, note, id8, 127, 0, 0xfd, 0, ch, note, id8, 0, 0];
    Some((id as i32, m))
}

/// `tobjSeLoop(pos, rate)` (0x0017c0d0), from `TOBJ::Draw` while its
/// transparency `rate` is not 0: loop `id`'s pan and volume for the point.
/// The volume is `calcVel`'s with a reach of `decay * 15000 / 256` (not
/// 5500), clamped to 0..127 (never -1) and uncapped by the row, then
/// `(int)(vol * rate)`. Sent: `FD 01 ch note id pan 00`; behind the
/// camera `FD 02 ch note id 120 63` (its bend 8 below the centre); `FD 00
/// ch note id vol 00`. The caller checks `ccSnd +0x133`.
pub fn tobj_se_loop(se: &SeTbl, cam: &Listener, pos: &V4, rate: F, id: i32) -> Vec<u8> {
    let x = sub(pos[0], cam.pos[0]);
    let y = sub(pos[1], cam.pos[1]);
    let z = sub(pos[2], cam.pos[2]);
    let dist = sqrtf(add(add(mul(x, x), mul(y, y)), mul(z, z)));
    let reach = div(mul(F_15000, from_int(i32::from(se.decay))), F_256);
    let mut per = sub(reach, dist);
    if lt(per, 0) {
        per = 0;
    }
    let vol = fptosi(div(mul(from_int(i32::from(se.velocity)), div(per, F_10)), F_100)).clamp(0, 127);
    let vol = fptosi(mul(from_int(vol), rate));
    let (pan, cdeg) = calc_pan(Some(cam), pos);
    let (ch, note, id) = (se.ch as u8, se.note as u8, id as u8);
    let mut m = vec![0xfd, 1, ch, note, id, pan as u8, 0];
    if behind(cdeg) {
        m.extend([0xfd, 2, ch, note, id, 120, 63]);
    }
    m.extend([0xfd, 0, ch, note, id, vol as u8, 0]);
    m
}

/// `seHitAttr(attr)` (0x0017a7f0): the footstep's `seData` row for the
/// ground (`ccChar.hitAttribute`, masked with 0x00f0f0f0); 202 for any
/// other ground.
pub fn se_hit_attr(attr: u32) -> usize {
    match attr & 0x00f0_f0f0 {
        0x0080_80f0 => 50,
        0x00b0_6010 => 49,
        0x00c0_d000 => 55,
        0x0030_4050 | 0x0040_5060 | 0x20f0 | 0x2040 | 0x3060 | 0x5030 | 0x6040 | 0x4080 => 24,
        0x0090_b0c0 => 27,
        0x0060_b0d0 => 26,
        0x0070_c0e0 => 25,
        0x00c0_c0c0 | 0x00e0_e0e0 => 28,
        0x00d0_d0d0 | 0x0060_6060 => 22,
        0x0040_4040 | 0x0050_5050 => 23,
        0xc000 => 21,
        _ => 202,
    }
}

/// A footstep: the ground's row ([`se_hit_attr`]), at note 67 on a ground
/// with no row of its own (202), else at `note`.
fn step(hit_attribute: u32, note: i8) -> NoteSe {
    let code = se_hit_attr(hit_attribute);
    NoteSe { code, note: Some(if code == 202 { 67 } else { note }) }
}

/// Row `param` of the table starting at row `start` of [`setbl::ROWS`]; a
/// param past the table's end reads on into what follows it, as the game
/// does (`None` past the end of setbl.cpp's data).
fn row(start: u16, param: u32) -> Option<SeNt> {
    let i = usize::from(start).checked_add(usize::try_from(param).ok()?)?;
    setbl::ROWS.get(i).copied()
}

/// A row's call: none for code -1 (or below); `ccSeOn3DNote` when the note
/// is not 0, else `ccSeOn3D`.
fn row_se(r: SeNt) -> Option<NoteSe> {
    let code = usize::try_from(r.code).ok()?;
    Some(NoteSe { code, note: (r.note != 0).then_some(r.note) })
}

/// `ccSeSetParamSPC(param, ch)` (0x0017aa20), a party member's note 1 or 2
/// (Kite's `ccPlayer::CheckNote`, `ccFellow::CheckNote`,
/// `ccSkillCheckNote`): row `param` of `spcSeTbl[id]` (`id` the
/// character's `ccCharBaseParam.id`, +0xc); nothing for a NULL table
/// (id 18) or a row of code -1. Param 0 is the footstep
/// (`ccSeOnPCStep`, 0x0017a6b0): the ground's row ([`se_hit_attr`] of
/// `ccChar.hitAttribute`) at a note by `id` - 60 for 0 (Kite), 62 for 1, 58
/// for 2, 3 and 8, 64 for 6, 61 for 10, 63 for 15, 61 for the rest - or 67
/// on a ground with no row. Any other param plays the row. `None` for an
/// `id` outside the table's 19 (which the game would read past).
pub fn spc_note(param: u32, id: i16, hit_attribute: u32) -> Option<NoteSe> {
    let start = (*setbl::SPC.get(usize::try_from(id).ok()?)?)?;
    let r = row(start, param)?;
    if r.code < 0 {
        return None;
    }
    if param != 0 {
        return row_se(r);
    }
    let note = match id {
        15 => 63,
        10 => 61,
        6 => 64,
        2 | 3 | 8 => 58,
        1 => 62,
        0 => 60,
        _ => 61,
    };
    Some(step(hit_attribute, note))
}

/// `ccSeSetParamPC(param, ch, ccstype)` (0x0017aaf0), a town PC's note
/// (`rtpcCheckNote`): param 0 only, the footstep - the ground's row at 60
/// for `ccstype` 0, 65 for 1, 58 for 2, 61 otherwise, or 67 on a ground
/// with no row.
pub fn pc_note(param: u32, ccstype: i32, hit_attribute: u32) -> Option<NoteSe> {
    if param != 0 {
        return None;
    }
    let note = match ccstype {
        0 => 60,
        1 => 65,
        2 => 58,
        _ => 61,
    };
    Some(step(hit_attribute, note))
}

/// `ccSeSetParamEnemy(param, ch, category)` (0x0017abd0), an enemy's notes
/// 1 and 2 (`ccEnemyCheckNote`): row `param` of `enemySeTbl[category]`
/// (the race's `seCategory`); nothing for a row of code -1. `None` for a
/// category outside the table's 19.
pub fn enemy_note(param: u32, category: i32) -> Option<NoteSe> {
    let start = (*setbl::ENEMY.get(usize::try_from(category).ok()?)?)?;
    row_se(row(start, param)?)
}

/// `ccSeSetParamInu(param, ch)` (0x0017ac70), a town dog's or a Grunty's
/// note (`inuCheckNote`, `ccPuccigusoCheckNote`): row `param` of
/// `inuSeData`, nothing for code -1; params 0 and 1 are footsteps, the
/// ground's row at 60 (67 on a ground with no row).
pub fn inu_note(param: u32, hit_attribute: u32) -> Option<NoteSe> {
    let r = row(*setbl::INU, param)?;
    if r.code < 0 {
        return None;
    }
    if param < 2 {
        return Some(step(hit_attribute, 60));
    }
    row_se(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(x: f32) -> F {
        x.to_bits()
    }

    fn se(velocity: i8, decay: i16) -> SeTbl {
        SeTbl { prog: 5, port: 0, ch: 2, note: 60, velocity, dummy: 127, decay }
    }

    #[test]
    fn no_camera_is_full_and_centred() {
        let s = se(96, 256);
        assert_eq!(calc_vel(&s, None, &[0; 4]), 127);
        assert_eq!(calc_pan(None, &[0; 4]), (63, 0));
        assert_eq!(se_on_3d(&s, None, &[0; 4], None), [0xc2, 5, 0xf9, 1, 2, 63, 0, 0xfd, 0x10, 2, 60, 0, 96, 0]);
    }

    #[test]
    fn velocity_falls_with_distance() {
        let s = se(127, 256);
        let cam = Listener { pos: [0, 0, 0, k(1.0)], view: [k(100.0), 0, 0, k(1.0)], kind: 0 };
        // At the eye: 127 * 5500 / 1000, capped.
        assert_eq!(calc_vel(&s, Some(&cam), &[0; 4]), 127);
        // 5000 of 5500 away: 127 * 50 / 100 = 63.5.
        assert_eq!(calc_vel(&s, Some(&cam), &[k(3000.0), k(4000.0), 0, 0]), 63);
        assert_eq!(calc_vel(&s, Some(&cam), &[k(6000.0), 0, 0, 0]), -1);
        assert!(se_on_3d(&s, Some(&cam), &[k(6000.0), 0, 0, 0], None).is_empty());
    }

    #[test]
    fn pan_follows_the_side() {
        let cam = Listener { pos: [0, 0, 0, 0], view: [k(100.0), 0, 0, 0], kind: 0 };
        // Ahead: the centre; a quarter turn to the left (counter-clockwise):
        // hard left; to the right: hard right; behind flags the bend.
        assert_eq!(calc_pan(Some(&cam), &[k(50.0), 0, 0, 0]), (63, 0));
        let left = calc_pan(Some(&cam), &[k(0.001), k(50.0), 0, 0]);
        assert!(left.0 <= 1, "{left:?}");
        let right = calc_pan(Some(&cam), &[k(0.001), k(-50.0), 0, 0]);
        assert!(right.0 >= 125, "{right:?}");
        let back = calc_pan(Some(&cam), &[k(-50.0), k(-1.0), 0, 0]);
        assert!(behind(back.1), "{back:?}");
        let m = se_on_3d(&se(96, 256), Some(&cam), &[k(-50.0), k(-1.0), 0, 0], None);
        assert_eq!(&m[7..12], &[0xf9, 2, 2, 120, 63]);
    }

    #[test]
    fn loops_take_and_free_slots() {
        let s = se(96, 256);
        let mut ids = [-1i8; 8];
        let (a, m) = se_on_3d_loop(&mut ids, &s, None, &[0; 4]);
        assert_eq!((a, m[m.len() - 2]), (0, 127));
        assert_eq!(se_on_3d_loop(&mut ids, &s, None, &[0; 4]).0, 1);
        assert_eq!(se_off_loop(&mut ids, &s, 0), [0xfd, 0x10, 2, 60, 0, 0, 0]);
        assert_eq!(se_on_3d_loop(&mut ids, &s, None, &[0; 4]).0, 0);
        for _ in 0..6 {
            se_on_3d_loop(&mut ids, &s, None, &[0; 4]);
        }
        assert_eq!(se_on_3d_loop(&mut ids, &s, None, &[0; 4]), (-1, Vec::new()));
    }

    #[test]
    fn tobj_hum_starts_silent_and_follows_distance_and_fade() {
        let s = SeTbl { prog: 31, port: 0, ch: 15, note: 60, velocity: 64, dummy: 127, decay: 128 };
        let mut ids = [0, -1, -1, -1, -1, -1, -1, -1];
        let (id, m) = tobj_se_loop_start(&mut ids, &s).unwrap();
        assert_eq!(id, 1);
        assert_eq!(m, [0xcf, 31, 0xfd, 0x10, 15, 60, 1, 127, 0, 0xfd, 0, 15, 60, 1, 0, 0]);
        let cam = Listener { pos: [0, 0, 0, 0], view: [k(100.0), 0, 0, 0], kind: 0 };
        // 3,000 of a reach of 7,500 away, ahead: 64 * 450 / 100 = 288,
        // clamped to 127, then 80% of it.
        let m = tobj_se_loop(&s, &cam, &[k(3000.0), 0, 0, 0], k(0.8), id);
        assert_eq!(m, [0xfd, 1, 15, 60, 1, 63, 0, 0xfd, 0, 15, 60, 1, 101, 0]);
        // Beyond the reach: silent, never -1.
        let m = tobj_se_loop(&s, &cam, &[k(8000.0), 0, 0, 0], k(1.0), id);
        assert_eq!(m[m.len() - 2], 0);
    }

    #[test]
    fn notes_pick_rows() {
        // Kite on a ground with no row: 202 at 67; on 0x008080f0, row 50 at 60.
        assert_eq!(spc_note(0, 0, 0), Some(NoteSe { code: 202, note: Some(67) }));
        assert_eq!(spc_note(0, 0, 0x0080_80f0), Some(NoteSe { code: 50, note: Some(60) }));
        assert_eq!(spc_note(2, 0, 0), Some(NoteSe { code: 30, note: None }));
        assert_eq!(spc_note(1, 0, 0), None);
        assert_eq!(spc_note(0, 18, 0), None);
        // cateWarrior (category 0) row 9: 169 at 64.
        assert_eq!(enemy_note(9, 0), Some(NoteSe { code: 169, note: Some(64) }));
        assert_eq!(enemy_note(0, 0), None);
        assert_eq!(inu_note(5, 0), Some(NoteSe { code: 78, note: None }));
        assert_eq!(inu_note(1, 0xc000), Some(NoteSe { code: 21, note: Some(60) }));
        assert_eq!(pc_note(0, 1, 0xc000), Some(NoteSe { code: 21, note: Some(65) }));
        assert_eq!(pc_note(1, 1, 0xc000), None);
    }
}
