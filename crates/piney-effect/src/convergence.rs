//! The converging spells (water, fire, wind, thunder, dark):
//! `ccSkill::ConvergenceSystem` (gcmn 0x00578060), the charge controller
//! `effSkillChargeObject` (effect -18) and its pieces
//! `effSkillChargeObj` (effects 102-112).
//!
//! ```text
//! ConvergenceSystem
//!   count 0   effPtr[0] cleared; a target on the lists: its magic attack
//!             sign, else the end and the caster released; level = id -
//!             (212 water, 232 fire, 244 wind, 264 thunder, 276 dark)
//!   count 20  a target on the lists: tPos its position; level 3+: the
//!             convergence element (m_effElm); else effPtr[0] the charge
//!             controller (effSkillChargeObject(target, type, level)) and
//!             ccSeOn3D(63, tPos); no target: the end, the caster released
//!   count 45  (levels 1, 2) ccSeOn3D(64, tPos)
//!   count 40  atkCnt 1; (levels 1, 2) the controller's temp[0] 1 (the
//!             pieces may converge); the caster released
//!   otherwise level 3+: once the element is deleted, the end and (a
//!             target on the lists) ccSkillDamage; nothing else
//!             levels 1, 2: once the controller has counted a piece in
//!             (temp[0] >= 2), once (tempCnt 0): on a target on the lists
//!             the burst at its middle - effRadiateSomething and the
//!             element's effSmoke*, ccSkillDamage, a shake in the camera's
//!             range (noise 10, cameraShake(0, 2, 10, 0)), the sound
//!             (water ccSeOn3DNote(66, tPos, 52), else effSkillBreakSE) -
//!             then tempCnt = ConvergenceSystem's third table + 1; when the
//!             controller's count reaches tempCnt, the end
//! the controller -18 (life 200, on the target), second chain:
//!   until all are made (temp[3]): temp[2] on by max(1, n / 20) (n the
//!   pieces for the level, main 0x003401c0), a piece for each whole step
//!   (at 100 + 15 cnt from the target's middle, cnt / 2 frames' wait),
//!   temp[1] counting them; then over once temp[0] > n
//!   two frames before its life ends temp[0] 99 (every piece goes)
//! a piece 102-112 (life 60, velocity -40): rising by speed.z (effIV,
//!   slowed by effSR from its count 7, still from 20); once the
//!   controller's temp[0] is set and its own wait (temp[1]) has run out,
//!   towards the target's middle, velocity growing by 10 a frame;
//!   FadeInOut(20, 10); within |velocity| of the middle (or the target
//!   gone, or its life all but over) it counts itself in (the
//!   controller's temp[0] + 1) and ends
//! ```

use piney_data::volume::Volume;

use crate::drawelm::{self, DrawElm};
use crate::ee::{self, F, ONE, V4};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR};
use crate::element::{Base, Element};
use crate::fall::eff_skill_break_se;
use crate::spell::{attr, camera_shake, check_camera_shake_range, first, noise};
use crate::{CharRef, Cx, Event, debris, ring, space, thunder, tornado, vu};

/// `effSkillChargeObject`'s controller (no object).
pub const CHARGE: i16 = -18;
/// The pieces `effSkillChargeObj` makes.
pub const PIECE_FIRST: i16 = 102;
pub const PIECE_LAST: i16 = 112;

/// The convergence's tables.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvergenceTables {
    /// `ConvergenceSystem`'s three (gcmn 0x00651930, 0x00651940,
    /// 0x00651950): the burst's pieces, its smoke, and the pieces to wait
    /// for, by level.
    pub radiate: [i32; 4],
    pub smoke: [i32; 4],
    pub wait: [i32; 4],
    /// The controller's pieces by level (main 0x003401c0).
    pub pieces: [i32; 4],
    /// `effIV` (main 0x00377fb8), `effSR` (0x00377ee8): a piece's first
    /// rise and its slowing.
    pub eff_iv: F,
    pub eff_sr: F,
    /// The convergence element's pieces by level (gcmn 0x005ed570).
    pub elements: [i32; 4],
}

impl ConvergenceTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> ConvergenceTables {
        let t = piney_data::tables::effect::of(volume);
        ConvergenceTables {
            radiate: first(t.convergence_radiate()),
            smoke: first(t.convergence_smoke()),
            wait: first(t.convergence_wait()),
            pieces: first(t.convergence_pieces()),
            eff_iv: t.convergence_iv().to_bits(),
            eff_sr: t.convergence_sr().to_bits(),
            elements: first(t.convergence_elements()),
        }
    }
}

/// `tbl[level - 1]`; the game reads its stack for a level out of range.
fn by_level(tbl: &[i32; 4], level: i32) -> i32 {
    tbl.get((level - 1) as usize).copied().unwrap_or(0)
}

/// A character's middle: its position, half its height up.
fn middle(cx: &Cx, ch: CharRef) -> V4 {
    let mut p = cx.host.char_pos(ch);
    p[2] = ee::add(p[2], ee::mul(0x3f00_0000, cx.host.char_height(ch)));
    p
}

/// `ccSkill::ConvergenceSystem` (gcmn 0x00578060).
pub fn convergence_system(ctrl: &mut EffectCtrl, cx: &mut Cx, k: usize) {
    let attr = cx.spells.runs[k].attr(&cx.spells.data);
    let s = cx.spells.runs[k].clone();
    let listed = |cx: &Cx| s.target.is_some_and(|t| cx.host.check_target(t));
    match s.count {
        0 => {
            cx.spells.runs[k].eff_ptr[0] = None;
            if listed(cx) {
                tornado::eff_magic_attack_sign(cx, s.target.unwrap_or(0), attr);
            } else {
                cx.spells.runs[k].status = 1;
                s.release(cx);
            }
            let r = &mut cx.spells.runs[k];
            match attr {
                attr::WATER => r.level = r.id - 212,
                attr::FIRE => r.level = r.id - 232,
                attr::WIND => r.level = r.id - 244,
                attr::THUNDER => r.level = r.id - 264,
                attr::DARK => r.level = r.id - 276,
                _ => {}
            }
            return;
        }
        20 => {
            if let Some(t) = s.target
                && cx.host.check_target(t)
            {
                let tpos = cx.host.char_pos(t);
                cx.spells.runs[k].t_pos = tpos;
                if s.level >= 3 {
                    cx.spells.runs[k].elm = convergence_element_generate(cx, k, attr, s.level);
                    return;
                }
                let ty = cx.spells.data.skill_type(s.id);
                cx.spells.runs[k].eff_ptr[0] = eff_skill_charge_object(ctrl, cx, t, ty, s.level);
                cx.raise(Event::Sound3d { se: 63, pos: tpos });
            } else {
                cx.spells.runs[k].status = 1;
                s.release(cx);
            }
            return;
        }
        45 => {
            if s.level < 3 {
                cx.raise(Event::Sound3d { se: 64, pos: s.t_pos });
            }
            return;
        }
        40 => {
            cx.spells.runs[k].atk_cnt = 1;
            if s.level < 3
                && let Some(p) = s.eff_ptr[0]
            {
                ctrl.effects[p].temp[0] = 1;
            }
            s.release(cx);
            return;
        }
        _ => {}
    }
    if s.level >= 3 {
        if let Some(e) = s.elm
            && cx.spells.elements.get(e).is_some_and(|e| e.deleted())
        {
            cx.spells.runs[k].status = 1;
            if listed(cx) {
                crate::spell::damage(cx, &s);
            }
        }
        return;
    }
    let Some(p) = s.eff_ptr[0] else { return };
    if (ctrl.effects[p].temp[0] as i32) < 2 {
        return;
    }
    if s.temp_cnt == 0 {
        if let Some(t) = s.target
            && cx.host.check_target(t)
        {
            let c = middle(cx, t);
            let r = [ee::deg2rad(-16384), 0, 0, 0];
            let d = &cx.spells.data.convergence;
            let (nr, ns) = (by_level(&d.radiate, s.level), by_level(&d.smoke, s.level));
            const V: F = 0x41f0_0000;
            debris::eff_radiate_something(ctrl, cx, c, r, V, attr, nr);
            match attr {
                attr::WATER => debris::eff_smoke_ice(ctrl, cx, c, r, V, 0x3f00_0000, ns),
                attr::FIRE => debris::eff_smoke_sparks(ctrl, cx, c, r, V, 0x4000_0000, ns),
                attr::WIND => debris::eff_smoke_leaf(ctrl, cx, c, r, V, ONE, ns),
                attr::THUNDER => debris::eff_smoke_electric(ctrl, cx, c, r, V, ONE, ns),
                attr::DARK => debris::eff_smoke_smoke(ctrl, cx, c, r, V, 0x4000_0000, ns),
                _ => None,
            };
            crate::spell::damage(cx, &s);
            if check_camera_shake_range(cx, s.t_pos) {
                noise(cx, 10);
                camera_shake(cx, 0, 2, 10, 0);
            }
            if attr == attr::WATER {
                cx.raise(Event::Sound3dNote { se: 66, pos: s.t_pos, note: 52 });
            } else {
                eff_skill_break_se(cx, c, attr);
            }
        }
        let w = by_level(&cx.spells.data.convergence.wait, s.level);
        cx.spells.runs[k].temp_cnt = w + 1;
    }
    let s = &mut cx.spells.runs[k];
    if ctrl.effects[p].temp[0] as i32 >= s.temp_cnt {
        s.hold = false;
        s.status = 1;
    }
}

/// `effSkillChargeObject(tp, type, level)` (main 0x001d5da0): the
/// controller -18 on `tp`, life 200, its level (4 bits), the counters
/// clear.
pub fn eff_skill_charge_object(ctrl: &mut EffectCtrl, cx: &mut Cx, tp: CharRef, ty: i32, level: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, CHARGE)?;
    let pos = cx.host.char_pos(tp);
    let e = &mut ctrl.effects[i];
    e.life_time = 200;
    e.target = Some(tp);
    e.pos_t = pos;
    e.param = ty;
    e.level = (((level & 0xf) << 4) as i8) >> 4;
    e.temp[0] = 0;
    e.temp[1] = 0;
    Some(i)
}

/// The controller -18's case of the second chain (main 0x001cb170).
pub fn charge_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let n = by_level(&cx.spells.data.convergence.pieces, i32::from(ctrl.effects[i].level));
    if ctrl.effects[i].temp[3] == 0 {
        let e = &mut ctrl.effects[i];
        let before = thunder::fptosi(e.temp[2]);
        let step = if n < 20 { ONE } else { ee::div(ee::from_int(n), 0x41a0_0000) };
        e.temp[2] = ee::add(e.temp[2], step);
        let mut m = thunder::fptosi(e.temp[2]) - before;
        while m != 0 {
            let (cnt, target, param) = (ctrl.effects[i].cnt, ctrl.effects[i].target, ctrl.effects[i].param);
            // fptosi(0.033 of the distance): computed, not used.
            let f = ee::add(ee::mul(0x4170_0000, ee::from_int(i32::from(cnt))), 0x42c8_0000);
            if let Some(t) = target
                && cx.host.check_target(t)
            {
                eff_skill_charge_obj(ctrl, cx, t, f, param, i, i32::from(cnt) >> 1);
            }
            m -= 1;
            let e = &mut ctrl.effects[i];
            e.temp[1] = e.temp[1].wrapping_add(1);
            if e.temp[1] as i32 >= n {
                e.temp[3] = 1;
            }
        }
    } else if ctrl.effects[i].temp[0] as i32 > n {
        ctrl.effects[i].end_flag = true;
    }
    let e = &mut ctrl.effects[i];
    if i32::from(e.age) >= i32::from(e.life_time) - 2 {
        e.temp[0] = 99;
    }
}

/// `effSkillChargeObj(tp, f, type, cnt, n)` (main 0x001d5eb0): a piece
/// at about `f` from `tp`'s middle, 50 up, at a random bearing and tilt,
/// counting into the controller in slot `counter` (the game keeps a
/// pointer to its temp[0]), waiting `n` frames once the controller says go.
/// Water 102-105 (one of four), fire 106, wind 107-110, thunder 111, dark
/// 112 (turned to face the target).
pub fn eff_skill_charge_obj(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    tp: CharRef,
    f: F,
    ty: i32,
    counter: usize,
    n: i32,
) -> Option<usize> {
    const TENTH: F = 0x3dcc_cccd;
    let rn = cx.host.rand() >> 3;
    let size = |k: F| ee::mul(TENTH, ee::mul(k, ee::from_int((rn >> 2) % 7 + 3)));
    let (id, s) = match ty & 0xfc {
        attr::WATER => (102 + (rn & 3) as i16, size(0x3f00_0000)),
        attr::FIRE => (106, size(0x4080_0000)),
        attr::WIND => (107 + (rn & 3) as i16, ONE),
        attr::THUNDER => (111, size(0x3f00_0000)),
        attr::DARK => (112, size(0x4000_0000)),
        // Registers the callers never leave unset.
        _ => return None,
    };
    let i = ctrl.new_effect(cx, id)?;
    let c = middle(cx, tp);
    let f = ee::sub(ee::add(f, ee::from_int((rn & 0xff0) % 20)), 0x4120_0000);
    let tilt = ee::deg2rad((rn % 24576 - 9216) as i16);
    let r = ee::mul(f, ee::cosf(tilt));
    let bearing = ee::deg2rad(((rn >> 8) & 0xfc00) as i16);
    // w is the stack's leftover plus 1 in the game; the port keeps 1 (it
    // is replaced once the piece moves, and no one reads it before).
    let v = [ee::mul(r, ee::cosf(bearing)), ee::mul(r, ee::sinf(bearing)), ee::mul(f, ee::sinf(tilt)), 0];
    let mut v = ee::vadd(v, c);
    v[2] = ee::add(v[2], 0x4248_0000);
    if ty & 0x20 != 0 {
        // A turn written into the clump's lwMatrix, which its next draw
        // rebuilds.
        cx.host.rand();
    }
    if ty & 0x80 != 0 {
        let d = ee::vsub(cx.host.char_pos(tp), v);
        let a = ee::rad2deg(ee::atan2f(d[1], d[0])) as i16;
        ctrl.effects[i].rot = [0, 0, ee::deg2rad(a.wrapping_add(16384)), 0];
    }
    // Dark, wind, water: SetFogSw(clump, 1).
    let pos = cx.host.char_pos(tp);
    let eff_iv = cx.spells.data.convergence.eff_iv;
    let e = &mut ctrl.effects[i];
    e.pos = v;
    e.life_time = 60;
    e.velocity = 0xc220_0000;
    e.speed[2] = eff_iv;
    e.target = Some(tp);
    e.pos_t = pos;
    e.scale = ee::vscale(ONE_VECTOR, s);
    e.temp[0] = counter as u32;
    e.temp[1] = n as u32;
    e.temp[2] = f;
    e.param = ty;
    Some(i)
}

/// The pieces 102-112's case of the first switch (main 0x001c58f0).
pub fn piece_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let counter = ctrl.effects[i].temp[0] as usize;
    let go = ctrl.effects.get(counter).is_some_and(|c| c.temp[0] != 0);
    if ctrl.effects[i].flags == 0 && go {
        let e = &mut ctrl.effects[i];
        let wait = e.temp[1] as i32;
        e.temp[1] = wait.wrapping_sub(1) as u32;
        if wait < 0 {
            e.flags = 1;
            let target = e.target;
            let speed = match target {
                Some(t) if cx.host.check_target(t) => ee::normalize(ee::vsub(middle(cx, t), ctrl.effects[i].pos)),
                _ => [0; 4],
            };
            ctrl.effects[i].speed = speed;
        }
    }
    let sr = cx.spells.data.convergence.eff_sr;
    let e = &mut ctrl.effects[i];
    if e.flags == 0 {
        if e.cnt < 20 {
            e.pos[2] = ee::add(e.pos[2], e.speed[2]);
            if e.cnt >= 7 {
                e.speed[2] = ee::mul(e.speed[2], sr);
            }
        } else {
            e.speed = [0; 4];
        }
    } else {
        e.velocity = ee::add(e.velocity, 0x4120_0000);
        let t = ee::vscale(e.speed, e.velocity);
        e.pos = ee::vadd(e.pos, t);
        e.pos[3] = ONE;
    }
    let life = i32::from(e.life_time);
    e.fade_in_out(20, 10, life);
    Next::Draw
}

/// The pieces' case of the second chain (main 0x001cafdc): moving, the
/// piece counts itself in (the controller's temp[0] + 1) and ends when it
/// is within |velocity| of the target's middle, when the target is gone,
/// or a frame before its life ends.
pub fn piece_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    if ctrl.effects[i].flags == 0 {
        return;
    }
    let counter = ctrl.effects[i].temp[0] as usize;
    let arrive = |ctrl: &mut EffectCtrl| {
        if let Some(c) = ctrl.effects.get_mut(counter) {
            c.temp[0] = c.temp[0].wrapping_add(1);
        }
        ctrl.effects[i].end_flag = true;
    };
    match ctrl.effects[i].target {
        Some(t) if cx.host.check_target(t) => {
            let d = ee::vsub(middle(cx, t), ctrl.effects[i].pos);
            let dist = ee::sqrtf(ee::dot(d, d));
            // (double)dist < fabs((double)velocity): exact in single.
            if ee::lt(dist, ee::fabsf(ctrl.effects[i].velocity)) {
                arrive(ctrl);
            }
        }
        _ => arrive(ctrl),
    }
    let e = &ctrl.effects[i];
    if e.end_flag {
        return;
    }
    if i32::from(e.age) >= i32::from(e.life_time) - 1 {
        arrive(ctrl);
    }
}

/// `ccConvergenceElementGenerate(skill, attr, level)` (gcmn 0x004ffd10):
/// the element in the first empty slot of the manager (none: deleted).
pub fn convergence_element_generate(cx: &mut Cx, k: usize, atr: i32, level: i32) -> Option<usize> {
    let s = cx.spells.runs[k].clone();
    let e = ConvergenceElement::new(cx, &s, atr & 0xfc, level);
    cx.spells.elements.add(Element::Convergence(Box::new(e)))
}

/// One of `ccConvergenceElement`'s `ELEMENT_T`s (64 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct ConvSlot {
    /// +0x00 the piece, +0x04 `interval`, +0x10 `cp[2]` and +0x30 `time`
    /// (never set), +0x34 `status`, +0x38 `life`.
    pub elm: DrawElm,
    pub interval: i32,
    pub cp: [V4; 2],
    pub time: F,
    pub status: i32,
    pub life: i32,
}

/// `ccConvergenceElement` (gcmn effect2.cpp, 0x5d0 bytes): levels 3 and 4,
/// pieces that grow out round the target and fly in.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvergenceElement {
    pub base: Base,
    /// +0x590 `m_elmNum`, +0x5a0 `m_elements`, +0x5a4 `m_skillPtr`, +0x5a8
    /// `m_generateSEOne`, +0x5ac `m_moveSEOne`, +0x5b0 `m_shockSEOne`,
    /// +0x5c0 `m_tPos`.
    pub elm_num: i32,
    pub elements: Vec<ConvSlot>,
    pub skill: u32,
    pub generate_se_one: i32,
    pub move_se_one: i32,
    pub shock_se_one: i32,
    pub t_pos: V4,
}

impl ConvergenceElement {
    /// `new ccConvergenceElement(skill, attr, level)` (gcmn 0x004eb250).
    pub fn new(cx: &mut Cx, s: &crate::spell::Spell, atr: i32, level: i32) -> ConvergenceElement {
        let n = by_level(&cx.spells.data.convergence.elements, level);
        let base = Base { target: s.target, level, attr: atr, ..Base::default() };
        let mut elements = Vec::new();
        for _ in 0..n.max(0) {
            let elm = match atr {
                attr::WATER => {
                    let r = (cx.host.genrand() as i32 & 3).wrapping_abs() + 4;
                    DrawElm::ice(cx, r)
                }
                attr::FIRE => DrawElm::fire_ball(cx),
                attr::THUNDER => DrawElm::plasma_ball(cx),
                attr::DARK => {
                    let mut e = DrawElm::dark_bat(cx);
                    for k in 0..3 {
                        e.base.anim.scale[k] = 0x4080_0000;
                    }
                    e
                }
                // The game stores to address 0 for any other.
                _ => DrawElm::blantch(cx),
            };
            elements.push(ConvSlot { elm, interval: 0, cp: [[0; 4]; 2], time: 0, status: 0, life: 0 });
        }
        let mut e = ConvergenceElement {
            base,
            elm_num: n,
            elements,
            skill: s.key,
            generate_se_one: 0,
            move_se_one: 0,
            shock_se_one: 0,
            t_pos: s.t_pos,
        };
        if level == 3 || level == 4 {
            let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
            let tp = s.t_pos;
            let tpp = space::w2p(tp, player, bounds);
            for i in 0..e.elements.len() {
                let x = thunder::add_abs(50.0, thunder::rand_f(cx, 0x43fa_0000));
                let v = [x, 0, 0, ONE];
                let mut w = v;
                w[0] = ee::add(w[0], 0x4396_0000);
                w[2] = ee::add(w[2], 0x4396_0000);
                let tilt = ee::neg(ee::fabsf(thunder::rand_f(cx, drawelm::PI)));
                let m = vu::rot_y(&vu::UNIT, tilt);
                let m = vu::rot_z(&m, thunder::rand_f(cx, drawelm::PI));
                let p1 = ee::vadd(tpp, ee::apply(&m, v));
                let sp = space::p2w(p1, player, bounds);
                let p2 = ee::vadd(tpp, ee::apply(&m, w));
                let ep = space::p2w(p2, player, bounds);
                let slot = &mut e.elements[i];
                let a = &mut slot.elm.base.anim;
                a.sp = sp;
                a.ep = tp;
                a.pos = sp;
                for k in 0..3 {
                    a.scale[k] = 0x3f33_3333;
                }
                a.transparency = 0;
                slot.interval = (i as i32 / 3) * 5;
                slot.time = 0;
                slot.status = 0;
                let up = ee::atan2f(ee::sub(p1[2], tpp[2]), drawelm::get_dist(p1, tpp));
                let dirc = drawelm::get_dirc(p1, tpp);
                let a = &mut slot.elm.base.anim;
                a.dirc = [0, up, dirc, ONE];
                a.start_fade = 0;
                a.end_fade = ONE;
                a.fade_spd = 0x3d08_8889;
                a.transparency = 0;
                a.fade_flag = 1;
                a.fade_accel = 0x3c23_d70a;
                let (ss, es) = ([0, 0, 0, ONE], [0x4020_0000, 0x4020_0000, 0x4020_0000, ONE]);
                a.start_scale = ss;
                a.end_scale = es;
                for k in 0..3 {
                    a.scale_spd[k] = ee::div(ee::sub(es[k], ss[k]), 0x41f0_0000);
                }
                a.scale = ss;
                a.scale_flag = 1;
                a.scale_accel = [0x3c23_d70a, 0x3c23_d70a, 0x3c23_d70a, ONE];
                a.speed = ee::vscale(ee::vsub(ep, sp), 0x3e4c_cccd);
                a.sp = sp;
                a.ep = ep;
                a.pos_flag = 1;
                slot.life = 15;
            }
        }
        e
    }

    /// `ccConvergenceElement::Main` (gcmn 0x004ebc70).
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let gen_se = self.generate_se_one;
        if cx.spells.entry_check(self.skill)
            && let Some(s) = cx.spells.get(self.skill)
        {
            self.t_pos = s.t_pos;
        }
        match self.base.level {
            3 => self.level3(ctrl, cx, gen_se),
            4 => match self.base.proccess {
                0 => self.level4(cx, gen_se),
                1 => self.level4_end(ctrl, cx),
                _ => {}
            },
            _ => {}
        }
    }

    /// Inline `AnimateFade` and `AnimateScale` of a piece growing: true
    /// once both are over.
    fn grow(a: &mut crate::element::Animate) -> bool {
        let faded = if a.fade_flag == 0 {
            true
        } else {
            a.transparency = ee::add(a.transparency, a.fade_spd);
            let e = a.end_fade;
            let over = if ee::lt(e, a.start_fade) { ee::lt(a.transparency, e) } else { !ee::le(a.transparency, e) };
            if over {
                a.transparency = e;
                a.fade_flag = 0;
                true
            } else {
                a.fade_spd = ee::add(a.fade_spd, a.fade_accel);
                false
            }
        };
        let scaled = a.animate_scale();
        faded && scaled
    }

    /// Off to the target: from the piece's place, 0.1 of the way a frame at
    /// first, twice that after 15 frames.
    fn launch(&self, slot: &mut ConvSlot, life: i32) {
        let a = &mut slot.elm.base.anim;
        a.speed = ee::vscale(ee::vsub(self.t_pos, a.pos), 0x3dcc_cccd);
        a.sp = a.pos;
        a.ep = self.t_pos;
        a.pos_flag = 1;
        let twice = ee::vscale(a.speed, 0x4000_0000);
        a.accel = ee::vscale(ee::vsub(twice, a.speed), 0x3d88_8889);
        a.start_speed = a.speed;
        a.end_speed = twice;
        a.accel_flag = 1;
        slot.life = life;
    }

    /// The level-3 pass.
    fn level3(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, gen_se: i32) {
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut done = true;
        let n = self.elements.len();
        for i in 0..n {
            if self.elements[i].elm.base.del_flag != 0 {
                continue;
            }
            self.elements[i].interval -= 1;
            if self.elements[i].interval > 0 {
                done = false;
                continue;
            }
            if gen_se == 0 {
                cx.raise(Event::Sound3d { se: 63, pos: self.t_pos });
                self.generate_se_one = 1;
            }
            self.elements[i].elm.main(cx);
            let pos = self.elements[i].elm.base.anim.pos;
            match self.elements[i].status {
                0 => {
                    if Self::grow(&mut self.elements[i].elm.base.anim) {
                        if i == 0 {
                            self.move_se(cx, pos);
                        }
                        self.elements[i].status += 1;
                    }
                }
                1 => {
                    let slot = &mut self.elements[i];
                    let life = slot.life;
                    slot.life = life - 1;
                    if life < 0 || slot.elm.base.anim.animate_pos(player, bounds) {
                        slot.status += 1;
                        let mut slot = self.elements[i].clone();
                        self.launch(&mut slot, 30);
                        self.elements[i] = slot;
                    }
                }
                2 => {
                    let slot = &mut self.elements[i];
                    let life = slot.life;
                    slot.life = life - 1;
                    if life < 0 || slot.elm.base.anim.animate_pos(player, bounds) {
                        self.shock3(ctrl, cx, pos);
                        if i == n - 1 && self.base.attr == attr::THUNDER {
                            self.thunder_generator(cx);
                        }
                        self.shock_se(cx);
                        self.elements[i].elm.base.del_flag = 1;
                        if check_camera_shake_range(cx, self.t_pos) {
                            noise(cx, 20);
                            camera_shake(cx, 2, 2, 20, 2);
                        }
                    }
                }
                _ => {}
            }
            done = false;
        }
        if done {
            self.base.del_flag = 1;
        }
    }

    /// The level-4 pass.
    fn level4(&mut self, cx: &mut Cx, gen_se: i32) {
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut done = true;
        for i in 0..self.elements.len() {
            if self.elements[i].elm.base.del_flag != 0 {
                continue;
            }
            self.elements[i].elm.main(cx);
            let pos = self.elements[i].elm.base.anim.pos;
            match self.elements[i].status {
                0 => {
                    self.elements[i].interval -= 1;
                    if self.elements[i].interval <= 0 {
                        if gen_se == 0 {
                            self.generate_se_one = 1;
                            cx.raise(Event::Sound3d { se: 63, pos });
                        }
                        if Self::grow(&mut self.elements[i].elm.base.anim) {
                            // (attr 3, never: every fourth only.)
                            self.move_se(cx, pos);
                            self.elements[i].status += 1;
                        }
                    }
                }
                1 => {
                    let slot = &mut self.elements[i];
                    let life = slot.life;
                    slot.life = life - 1;
                    if life < 0 || slot.elm.base.anim.animate_pos(player, bounds) {
                        slot.status += 1;
                        let mut slot = self.elements[i].clone();
                        self.launch(&mut slot, 15);
                        self.elements[i] = slot;
                    }
                }
                2 => {
                    let slot = &mut self.elements[i];
                    let life = slot.life;
                    slot.life = life - 1;
                    if life < 0 || slot.elm.base.anim.animate_pos(player, bounds) {
                        let ty = match self.base.attr & 0xfc {
                            attr::WIND => 164,
                            attr::DARK => 165,
                            attr::SOIL => 167,
                            attr::FIRE => 162,
                            attr::WATER | attr::THUNDER => 161,
                            _ => -1,
                        };
                        if ty != -1 {
                            let r = [0, thunder::rand_f(cx, drawelm::PI), thunder::rand_f(cx, drawelm::PI), ONE];
                            ring::eff_summon_ring_element(cx, pos, r, ty);
                        }
                        self.shock_se(cx);
                        self.elements[i].status += 1;
                        if check_camera_shake_range(cx, self.t_pos) {
                            noise(cx, 20);
                            camera_shake(cx, 2, 2, 20, 2);
                        }
                    }
                }
                _ => continue,
            }
            done = false;
        }
        if done {
            self.base.proccess += 1;
        }
    }

    /// Level 4's end: the pieces drawn on, and at 30 the burst.
    fn level4_end(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        for slot in &mut self.elements {
            slot.elm.main(cx);
        }
        let c = self.base.count;
        self.base.count = c + 1;
        if c != 30 {
            return;
        }
        let tp = self.t_pos;
        self.shock4(ctrl, cx, tp);
        cx.raise(Event::Sound3d { se: 56, pos: tp });
        cx.raise(Event::Sound3d { se: 35, pos: tp });
        if self.base.attr == attr::WATER {
            cx.raise(Event::Sound3dNote { se: 66, pos: tp, note: 48 });
        }
        if check_camera_shake_range(cx, tp) {
            noise(cx, 20);
            camera_shake(cx, 2, 2, 20, 2);
        }
        self.base.del_flag = 1;
    }

    /// `MoveSE(pos)` (gcmn 0x004ec850): once.
    fn move_se(&mut self, cx: &mut Cx, pos: V4) {
        if self.move_se_one != 0 {
            return;
        }
        self.move_se_one = 1;
        if self.base.attr == attr::DARK {
            cx.raise(Event::Sound3dNote { se: 187, pos: self.t_pos, note: 72 });
        } else {
            cx.raise(Event::Sound3d { se: 64, pos });
        }
    }

    /// `ShockSE()` (gcmn 0x004ec8c0).
    fn shock_se(&mut self, cx: &mut Cx) {
        let tp = self.t_pos;
        match self.base.attr {
            attr::DARK => cx.raise(Event::Sound3dNote { se: 170, pos: tp, note: 52 }),
            attr::WATER => {
                if self.base.level == 3 {
                    if self.shock_se_one == 0 {
                        self.shock_se_one = 1;
                        cx.raise(Event::Sound3dNote { se: 66, pos: tp, note: 48 });
                        cx.raise(Event::Sound3d { se: 35, pos: tp });
                        cx.raise(Event::Sound3dNote { se: 56, pos: tp, note: 72 });
                    }
                } else {
                    cx.raise(Event::Sound3d { se: 65, pos: tp });
                }
            }
            a => eff_skill_break_se(cx, tp, a),
        }
    }

    /// `Shock3(pos)` (gcmn 0x004ec9c0): fire's rock and debris, water's
    /// ice.
    fn shock3(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4) {
        let r = [ee::deg2rad(-16384), 0, thunder::rand_f(cx, drawelm::PI), ONE];
        const V: F = 0x4234_0000;
        match self.base.attr & 0xfc {
            attr::FIRE => {
                debris::eff_smoke_rock(ctrl, cx, pos, r, V, 1, 0);
                debris::eff_radiate_something2(ctrl, cx, pos, r, V, self.base.attr, 1);
            }
            attr::WATER => {
                debris::eff_radiate_something2(ctrl, cx, pos, r, V, attr::WATER, self.base.level * 5 / 2);
            }
            _ => {}
        }
    }

    /// `Shock4(pos)` (gcmn 0x004ecad0): a big explosion growing from 1 to
    /// 5 over 15 frames and the element's debris.
    fn shock4(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4) {
        let r = [ee::deg2rad(-16384), 0, thunder::rand_f(cx, drawelm::PI), ONE];
        const V: F = 0x4234_0000;
        let grow = |cx: &mut Cx, k: Option<usize>| {
            if let Some(k) = k
                && let Some(Element::Draw(e)) = cx.spells.elements.slots[k].as_mut()
            {
                let a = &mut e.base.anim;
                let (ss, es) = ([ONE; 4], [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, ONE]);
                a.start_scale = ss;
                a.end_scale = es;
                for i in 0..3 {
                    a.scale_spd[i] = ee::div(ee::sub(es[i], ss[i]), 0x4170_0000);
                }
                a.scale = ss;
                a.scale_flag = 1;
                a.scale_accel = [0, 0, 0, ONE];
            }
        };
        let n = self.base.level * 5 / 2;
        let a = self.base.attr;
        match a {
            attr::FIRE => {
                let k = drawelm::eff_explode3(cx, pos, [0, 0, 0, ONE], 0x40a0_0000, 0);
                grow(cx, k);
            }
            attr::WATER => {
                let k = drawelm::eff_explode3(cx, pos, [0, 0, 0, ONE], 0x40a0_0000, 1);
                grow(cx, k);
                debris::eff_radiate_something(ctrl, cx, pos, r, V, a & 0xfc, n);
            }
            attr::WIND => {
                let k = drawelm::eff_explode3(cx, pos, [0, 0, 0, ONE], 0x40a0_0000, 3);
                grow(cx, k);
            }
            attr::THUNDER => {
                let k = drawelm::eff_explode3(cx, pos, [0, 0, 0, ONE], 0x40a0_0000, 2);
                grow(cx, k);
                self.thunder_generator(cx);
                debris::eff_radiate_something(ctrl, cx, pos, r, V, a & 0xfc, n);
            }
            attr::DARK => {
                let k = drawelm::eff_explode3(cx, pos, [0, 0, 0, ONE], 0x40a0_0000, 4);
                grow(cx, k);
                debris::eff_radiate_something(ctrl, cx, pos, r, V, a & 0xfc, n);
            }
            _ => {}
        }
        if check_camera_shake_range(cx, self.t_pos) {
            noise(cx, 20);
            camera_shake(cx, 2, 2, 20, 2);
        }
    }

    /// The thunder's lingering sparks: `effElementGeneratorTbl` + 0x70 at
    /// `m_tPos`, `pTexMod` 156.
    fn thunder_generator(&self, cx: &mut Cx) {
        let (param, ff) = (cx.spells.data.fall.thunder_gen, cx.spells.data.fall.thunder_ff);
        let tp = self.t_pos;
        crate::pfx::start_param(cx, param, [Some(ff[0]), Some(ff[1]), None, None], |g| {
            g.pos = tp;
            g.p_tex_mod = 156;
        });
    }
}
