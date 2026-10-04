//! The spring, the boss room's warning and the radiator (`ccGimEtc`, gcmn
//! gmetc.cpp, 0x00456220-0x00458218): `gimmickTbl` rows 19 (`WARNING`; with
//! `param[2]` 0 the event's `radiator`, rays at a character's right hand), 20
//! (the lakes' Spring of Myst, `FOUNTAIN`) and 21 (`ENTRANCE`). The
//! functions are in docs/engine/battle.md ("The spring and the boss room's
//! warning"). Affect 11 is `FountainMenu3`'s `EntryAffect(fountain, plw, 11)`.

use crate::chara::Char;
use crate::enemy_ai::{EntryParam, rand_f};
use crate::entry::{Cx, EntryObj, Out, SAVE_FOUNTAIN, delete_cmnd, gimmick_char, rand_s};
use crate::geom::{self, F, HALF_PI, ONE, PI, TWO_PI, V4, add, deg2rad, div, le, lt, mul, sub};
use crate::gimmick::Class;
use crate::prim::{self, GIM_RAD_INFO, OmniLight, Radiate};
use crate::world::{AnmSlot, World};
use piney_data::libm;
use piney_data::save::SaveData;

// `ccEntryGimEtc` (INF gcmn 0x00456450): rows 19-21.
/// `ccDestGimEtc` (0x004564a0): the lights out and freed.
pub const DEST_ETC: u32 = 0x0045_64a0;

/// The spring's clumps and clips (`XGWATER0.CCS`), by `spiritType`.
pub const SPRING_MODEL: [(&str, &str); 2] = [("CMP_trall1", "ANM_xgwater1a"), ("CMP_trall2", "ANM_xgwater2a")];

/// `saveData.fountainCount` (+0x676c): the next of the 100 fountains used.
pub const SAVE_FOUNTAIN_COUNT: usize = 0x676c;

/// `ccGimEtc`'s members (+0x1e0 on).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etc {
    /// +0x1e0 `effsw`: the row's particles run while it is not 0.
    pub effsw: i32,
    /// +0x1e4 `spiritFlag` (the spring drawn), `spiritType` (area level 4
    /// on: `CMP_trall2`).
    pub spirit_flag: bool,
    pub spirit_type: bool,
    /// +0x1f0 `scale`, +0x200 `vpos`, +0x210 `radCnt`, +0x214
    /// `windowOfs`.
    pub scale: V4,
    pub vpos: V4,
    pub rad_cnt: F,
    pub window_ofs: F,
    /// +0x220 `scale0`, +0x230 `bounceCnt`, +0x234 `bounceDeg`, +0x23c
    /// `bounceOfs`.
    pub scale0: V4,
    pub bounce_cnt: i32,
    pub bounce_deg: [i16; 4],
    pub bounce_ofs: F,
    /// +0x240 `scale1`, +0x250 `springCnt`, +0x254 `springDec`, +0x258
    /// `springZoom`.
    pub scale1: V4,
    pub spring_cnt: F,
    pub spring_dec: F,
    pub spring_zoom: F,
    /// +0x25c `lgtFlag`: the lights in the group; +0x260 `lgt[2]`.
    pub lgt_flag: bool,
    pub lights: [OmniLight; 2],
    /// +0x268 `gimRad`: the radiator's rays (row 19, `param[2]` 0).
    pub gim_rad: Option<Box<Radiate>>,
}

impl Default for Etc {
    /// As the constructor leaves rows 19 and 21 (the members zero but
    /// `effsw`).
    fn default() -> Self {
        Etc {
            effsw: 1,
            spirit_flag: false,
            spirit_type: false,
            scale: [0; 4],
            vpos: [0; 4],
            rad_cnt: 0,
            window_ofs: 0,
            scale0: [0; 4],
            bounce_cnt: 0,
            bounce_deg: [0; 4],
            bounce_ofs: 0,
            scale1: [0; 4],
            spring_cnt: 0,
            spring_dec: 0,
            spring_zoom: 0,
            lgt_flag: false,
            lights: [OmniLight::default(); 2],
            gim_rad: None,
        }
    }
}

/// `param[3]` as the event's `radiator` sets it, the rays' user (a
/// `ccSpcChar *`): here its scene index plus 1, 0 for none (null).
pub fn rad_user_param(user: Option<usize>) -> i32 {
    user.map_or(0, |u| u as i32 + 1)
}

/// The user [`rad_user_param`] names.
fn rad_user(param: i32) -> Option<usize> {
    usize::try_from(param).ok()?.checked_sub(1)
}

/// `new ccGimEtc(ent)` (gcmn 0x00456540), see the module's header.
pub fn etc_new(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
    let mut ch = gimmick_char(cx.st, ent.id);
    ch.ent_root = ent.ent_root as u32;
    ch.pos = ent.pos;
    ch.pos_p = cx.world.w2p(ent.pos);
    let mut o = EntryObj { ent: *ent, dirc: ent.dirc, gim_id: ent.id, ..EntryObj::default() };
    o.act_num = 0;
    o.act_cnt = 0;
    let mut etc = Etc::default();
    match o.gim_id {
        // deleteCmnd(1): not listed yet, so only cmndFlag. Row 19 with
        // param[2] 0: new ccGimRadiator(gimRadInfo, param[3]), then
        // setGimRadPos(param[3]).
        19 => {
            o.cmnd_flag = true;
            if ent.param[2] == 0 {
                let mut rad = Radiate::new(GIM_RAD_INFO, rad_user(ent.param[3]));
                set_gim_rad_pos(cx.world, &mut rad, &mut ch.pos, &mut o.dirc);
                etc.gim_rad = Some(Box::new(rad));
            }
        }
        21 => o.cmnd_flag = true,
        20 => init_fountain(cx, who, &mut etc, ch.pos),
        _ => {}
    }
    o.class = Class::Etc(Box::new(etc));
    // initFountain's: rows 19 and 21 have none.
    if o.gim_id == 20 {
        o.dest_func = DEST_ETC;
    }
    (ch, o)
}

/// `ccGimEtc::initFountain(ent)` (gcmn 0x00456ba0).
fn init_fountain(cx: &mut Cx, who: usize, etc: &mut Etc, pos: V4) {
    etc.spirit_flag = false;
    etc.scale = [ONE; 4];
    etc.scale0 = [ONE; 4];
    etc.scale1 = [ONE; 4];
    etc.vpos = pos;
    etc.rad_cnt = 0;
    etc.window_ofs = 0;
    etc.bounce_cnt = 1;
    etc.bounce_ofs = 0;
    for d in &mut etc.bounce_deg {
        *d = rand_s(cx.rnds);
    }
    etc.spring_zoom = 0;
    etc.spring_dec = 0;
    etc.spring_cnt = 0;
    etc.spirit_type = cx.game.area_level >= 4;
    let (_, anm) = SPRING_MODEL[usize::from(etc.spirit_type)];
    cx.world.anim_set(who, AnmSlot::Main, anm);
}

/// `ccGimEtc::main()` (gcmn 0x00456760): true to delete it.
pub fn main(cx: &mut Cx, who: usize, o: &mut EntryObj) -> bool {
    if o.freeze_flag {
        return false;
    }
    let Class::Etc(mut etc) = std::mem::replace(&mut o.class, Class::None) else { return false };
    if o.init_flag {
        if etc.effsw == 1 {
            let pos = cx.scene.chars[who].pos;
            match o.gim_id {
                19 if o.ent.param[2] != 0 => cx.out.push(Out::EtcEffect { who, row: 19, pos }),
                20 | 21 => cx.out.push(Out::EtcEffect { who, row: o.gim_id, pos }),
                _ => {}
            }
        }
        o.init_flag = false;
    }
    let q = cx.world.w2p(cx.scene.chars[who].pos);
    cx.scene.chars[who].pos_p = q;
    cx.scene.chars[who].pos = cx.world.p2w(q);
    let r = match o.gim_id {
        19 if o.ent.param[2] == 0 => ctrl_gim_radiator(cx, who, o, &mut etc),
        20 => ctrl_fountain(cx, who, o, &mut etc),
        _ => false,
    };
    o.class = Class::Etc(etc);
    r
}

/// `ccGimEtc::setGimRadPos(user)` (gcmn 0x00456950): the rays and the
/// object at the user's right hand (`objHandR`'s `lwMatrix` translation),
/// both turned as the hand is (`ccSetMat2Rot` of its rotation; the rays'
/// x, y and z, the object's `dirc` whole).
fn set_gim_rad_pos(w: &mut dyn World, rad: &mut Radiate, pos: &mut V4, dirc: &mut V4) {
    let Some(mut m) = rad.user.and_then(|u| w.hand_r(u)) else { return };
    rad.pos = m[3];
    *pos = m[3];
    m[3] = [0, 0, 0, m[3][3]];
    let rot = crate::breath::mat2rot(&m);
    rad.rot[..3].copy_from_slice(&rot[..3]);
    *dirc = rot;
}

/// `ccGimEtc::ctrlGimRadiator()` (gcmn 0x00456a30): the first frame the
/// rays start (`radFlag`); each frame they follow the user's hand, and go
/// with the object once the user's AI leaves manual mode (the event let
/// go); else `ccPrimRadiate::main` ([`prim::radiator_ctrl`]).
fn ctrl_gim_radiator(cx: &mut Cx, who: usize, o: &mut EntryObj, etc: &mut Etc) -> bool {
    let Some(rad) = etc.gim_rad.as_deref_mut() else { return false };
    if o.act_num == 0 {
        rad.rad_flag = true;
        o.act_num += 1;
    }
    let mut out = Vec::new();
    let gone = match rad.user {
        Some(u) => {
            set_gim_rad_pos(cx.world, rad, &mut cx.scene.chars[who].pos, &mut o.dirc);
            !cx.world.control_mode(u)
        }
        None => false,
    };
    if gone {
        // ~ccPrimRadiate: delLight.
        rad.del_light(&mut out);
        etc.gim_rad = None;
    } else {
        prim::main(rad, cx, &mut out, prim::radiator_ctrl);
    }
    cx.out.extend(out.into_iter().map(|r| Out::Radiate { who, out: r }));
    gone
}

/// Whether the object took affect 11 this frame (its `affectFlag` taken
/// down).
fn affect11(cx: &Cx, who: usize, o: &mut EntryObj) -> bool {
    if o.affect_flag {
        o.affect_flag = false;
        return cx.scene.chars[who].affect.ty == 11;
    }
    false
}

/// The next state, its count back to 0.
fn next(o: &mut EntryObj) {
    o.act_num += 1;
    o.act_cnt = 0;
}

/// A spring of `cnt` wobbles, `zoom` its depth over `frames` frames.
fn spring(etc: &mut Etc, cnt: F, zoom: F, frames: F) {
    etc.spring_cnt = cnt;
    etc.spring_zoom = div(zoom, etc.spring_cnt);
    etc.spring_dec = div(etc.spring_cnt, frames);
}

/// An angle past pi taken back by 2 pi.
fn wrap(v: F) -> F {
    if le(v, PI) { v } else { sub(v, TWO_PI) }
}

/// `ccGimEtc::ctrlFountain()` (gcmn 0x00456fb0), see the module's header.
fn ctrl_fountain(cx: &mut Cx, who: usize, o: &mut EntryObj, etc: &mut Etc) -> bool {
    match o.act_num {
        0 | 5 | 7 | 9 => {
            o.act_cnt += 1;
            if affect11(cx, who, o) {
                next(o);
            }
        }
        1 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            match c {
                0 => {
                    for se in [85, 216, 217] {
                        cx.out.push(Out::Se2d(se));
                    }
                }
                10 => cx.out.push(Out::Flash { t0: 30, t1: Some(30), colour: 0x80ff_ffff }),
                60 => next(o),
                _ => {}
            }
        }
        2 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                o.transparency = 0;
                o.set_transparency = 0;
                o.fade_flag = 1;
                o.fade_cnt = 10;
                let d = geom::rad2deg(o.pl_dirc).wrapping_add(i16::MIN);
                o.dirc[2] = deg2rad(d);
                cx.scene.chars[who].pos[2] = geom::k(-400.0);
                spring(etc, geom::k(6.0), geom::k(0.7), geom::k(90.0));
                etc.rad_cnt = 0;
                etc.spirit_flag = true;
                cx.out.push(Out::Se2d(215));
                let rgb = if etc.spirit_type { 0x0080_80ff } else { 0x00ff_8080 };
                for l in &mut etc.lights {
                    l.far_start = 0;
                    l.far_end = geom::k(1500.0);
                    l.far_end2 = geom::k(2_250_000.0);
                    l.rgb = rgb;
                }
                // lgtFlag down (the lights stay in the group until the
                // next draw adds them again).
                etc.lgt_flag = false;
            } else if c == 90 {
                next(o);
            }
            etc.rad_cnt = add(etc.rad_cnt, 0x3d0e_fa35);
            let s = libm::sinf(mul(geom::k(2.0), etc.rad_cnt));
            let to = add(geom::k(150.0), mul(geom::k(230.0), s));
            geom::set_dist(&mut cx.scene.chars[who].pos[2], to, 0x3dcc_cccd);
            let s = libm::sinf(etc.rad_cnt);
            o.dirc[2] = wrap(add(o.dirc[2], mul(0x3f19_999a, s)));
        }
        3 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                etc.bounce_cnt = 1;
            }
            if etc.spring_cnt == 0 && cx.cc.rand() & 7 == 0 {
                let n = cx.cc.rand().wrapping_add(1) & 3;
                spring(etc, geom::from_int(n), geom::k(0.2), geom::k(60.0));
            }
            geom::set_dist(&mut cx.scene.chars[who].pos[2], geom::k(150.0), 0x3dcc_cccd);
            o.dirc[2] = geom::set_dirc(o.dirc[2], o.pl_dirc, 128);
            if affect11(cx, who, o) {
                next(o);
            }
        }
        4 | 8 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                spring(etc, geom::k(7.0), geom::k(0.5), geom::k(90.0));
            } else if c == 30 {
                next(o);
            }
        }
        6 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                spring(etc, geom::k(7.0), geom::k(0.5), geom::k(90.0));
                cx.out.push(Out::Se2d(104));
            }
            let n = o.act_cnt;
            if n < 30 && n & 3 == 0 {
                let mut p = etc.vpos;
                p[2] = add(p[2], add(geom::k(-100.0), mul(geom::k(10.0), geom::from_int(n))));
                cx.out.push(Out::TrapRemoved { pos: p });
            }
            if n < 31 {
                o.dirc[2] = wrap(add(o.dirc[2], 0x3ed6_7750));
            }
            if n == 80 {
                next(o);
            }
        }
        10 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            match c {
                0 => {
                    etc.effsw = 0;
                    spring(etc, geom::k(3.0), geom::k(0.9), geom::k(180.0));
                }
                30 => {
                    o.transparency = ONE;
                    o.set_transparency = ONE;
                    o.fade_flag = 2;
                    o.fade_cnt = 60;
                    spring(etc, geom::k(7.0), geom::k(0.7), geom::k(90.0));
                    cx.out.push(Out::Se2d(215));
                }
                90 => {
                    cx.out.push(Out::Flash { t0: 10, t1: None, colour: 0x80ff_ffff });
                    next(o);
                }
                _ => {}
            }
            let z = &mut cx.scene.chars[who].pos[2];
            if o.act_cnt < 30 {
                geom::set_dist(z, geom::k(-200.0), 0x3ca3_d70a);
            } else {
                geom::set_dist(z, geom::k(1800.0), 0x3da3_d70a);
                etc.bounce_cnt = 0;
                for k in 0..3 {
                    geom::set_dist(&mut etc.scale0[k], 0, 0x3df5_c28f);
                }
            }
            let n = o.act_cnt;
            if (6..60).contains(&n) && n & 3 == 2 {
                cx.out.push(Out::OpenBox { pos: cx.scene.chars[who].pos });
            }
        }
        11 => {
            set_fountain(cx.save, cx.game.server, cx.game.area_code);
            delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
            if etc.lgt_flag {
                cx.out.push(Out::EtcLights { who, lights: None });
                etc.lgt_flag = false;
            }
            return true;
        }
        _ => {}
    }
    tail(cx, who, o, etc);
    false
}

/// Every state's end (see the module's header).
fn tail(cx: &mut Cx, who: usize, o: &mut EntryObj, etc: &mut Etc) {
    let pos = cx.scene.chars[who].pos;
    if o.act_num < 10 && o.act_cnt & 3 == 0 {
        let m = geom::rot_matrix_z(&geom::unit_matrix(), rand_f(cx.cc, PI));
        let mut v = geom::VF0;
        v[0] = rand_f(cx.cc, geom::k(100.0));
        let v = geom::apply_matrix(&m, v);
        let p = [add(pos[0], v[0]), add(pos[1], v[1]), geom::k(-50.0), pos[3]];
        cx.out.push(Out::DustRingAt { pos: p, s: geom::k(3.0), r: geom::k(30.0), n: 4, life: 35, tex: 110 });
    }
    etc.vpos = pos;
    if (3..11).contains(&o.act_num) {
        geom::set_dist(&mut etc.window_ofs, geom::k(150.0), 0x3d23_d70a);
    } else {
        geom::set_dist(&mut etc.window_ofs, 0, 0x3c23_d70a);
    }
    etc.vpos[2] = add(etc.vpos[2], etc.window_ofs);
    if etc.bounce_cnt != 0 {
        let steps: [(i16, i16); 4] = [(0xff, 837), (0xff, 1110), (0xff, 564), (0x7f, 1110)];
        let mut vals = [0; 4];
        for (k, (mask, step)) in steps.into_iter().enumerate() {
            let r = rand_s(cx.rnds) & mask;
            etc.bounce_deg[k] = etc.bounce_deg[k].wrapping_add(r + step);
            vals[k] = deg2rad(etc.bounce_deg[k]);
            if k == 0 {
                etc.scale0[0] = add(0x3f99_999a, mul(0x3e61_47ae, libm::cosf(vals[0])));
            } else if k == 1 {
                etc.scale0[1] = add(0x3f8c_cccd, mul(0x3e6b_851f, libm::cosf(vals[1])));
            } else if k == 2 {
                etc.scale0[2] = add(ONE, mul(0x3df5_c28f, libm::sinf(vals[2])));
            } else {
                etc.bounce_ofs = mul(geom::k(20.0), libm::sinf(vals[3]));
                etc.vpos[2] = add(etc.vpos[2], etc.bounce_ofs);
            }
        }
    }
    if etc.spring_cnt != 0 {
        if lt(etc.spring_cnt, 0x3c23_d70a) {
            etc.spring_cnt = 0;
        } else {
            etc.spring_cnt = sub(etc.spring_cnt, etc.spring_dec);
            let a = add(HALF_PI, mul(TWO_PI, etc.spring_cnt));
            let d = mul(etc.spring_zoom, etc.spring_cnt);
            etc.scale1[0] = add(ONE, mul(d, libm::cosf(a)));
            let a = mul(TWO_PI, etc.spring_cnt);
            let d = mul(etc.spring_zoom, etc.spring_cnt);
            etc.scale1[2] = add(ONE, mul(d, libm::sinf(a)));
        }
    }
    for k in 0..3 {
        etc.scale[k] = mul(etc.scale0[k], etc.scale1[k]);
    }
    if !o.disp_sw || !etc.spirit_flag {
        return;
    }
    o.disp_sw = cx.world.camera_deg(pos, 0x3000);
    if !o.disp_sw {
        return;
    }
    o.anm_flag = i32::from(cx.world.anim_forward(who, AnmSlot::Main, 256));
    cx.out.push(Out::EtcDraw { who, pos: etc.vpos, dirc: o.dirc, scale: etc.scale });
    let s = libm::sinf(deg2rad(etc.bounce_deg[3]));
    let up = add(ONE, s);
    // (1 + sin) + 0.5 in double (fptodp, dpadd, dptofp).
    let intensity = ((f64::from(f32::from_bits(up)) + 0.5) as f32).to_bits();
    let far = add(geom::k(1000.0), mul(geom::k(1000.0), up));
    for (k, l) in etc.lights.iter_mut().enumerate() {
        let mut p = etc.vpos;
        p[2] = if k == 0 { sub(p[2], geom::k(30.0)) } else { add(p[2], geom::k(100.0)) };
        l.intensity = intensity;
        l.far_end = far;
        l.far_end2 = mul(far, far);
        l.matrix = geom::trans_matrix(&geom::unit_matrix(), p);
        l.mat_calc = true;
    }
    cx.out.push(Out::EtcLights { who, lights: Some(etc.lights) });
    etc.lgt_flag = true;
}

/// `ccSaveData::SetFountain(code)` (main 0x00178350): `code | server << 29`
/// among the 100 fountains used, at `fountainCount` (+0x676c, back to 0
/// at 100) when it is not there yet.
pub fn set_fountain(save: &mut SaveData, server: i32, code: i32) {
    let key = code | server.wrapping_shl(29);
    if (0..100).any(|k| save.i32(SAVE_FOUNTAIN + 4 * k) == key) {
        return;
    }
    let n = save.i16(SAVE_FOUNTAIN_COUNT);
    save.set_i32(SAVE_FOUNTAIN + 4 * n as usize, key);
    save.set_i16(SAVE_FOUNTAIN_COUNT, n.wrapping_add(1));
    if save.i16(SAVE_FOUNTAIN_COUNT) >= 100 {
        save.set_i16(SAVE_FOUNTAIN_COUNT, 0);
    }
}
