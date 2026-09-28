//! What shows as a skill starts (main effect.cpp): `effSkillStart` (main
//! 0x001d35f0 by skill id, 0x001d3650 by `ccSkillParam`), which a skill's
//! request, a party member's or an enemy's attack skill and
//! `ccSkillModifyCondition` call on the caster (or the one a condition skill
//! lands on), and `effSkillStartEffect` (0x001d39d0), the Data Drain menu's
//! side effects; with the rings and circles they start (effects 82-86,
//! 0x001d2d80-0x001d3450) and the controllers -12 and -13. The cases are in
//! docs/engine/effects.md ("The start: effSkillStart").

use piney_data::volume::Volume;

use crate::ee::{self, F, V4};
use crate::effect::{EffectCtrl, Next, Obj};
use crate::{CharRef, Cx, Event, VecRef, pfx};

/// Effect ids.
pub const START: i16 = -12;
pub const START_B: i16 = -13;
pub const EXEC_RING: i16 = 82;
pub const FORCE_RING: i16 = 83;
pub const FORCE_RING2: i16 = 84;
pub const SUMMONS_CIRCLE: i16 = 85;
pub const CIRCLE: i16 = 86;

/// Sounds: a PC's or party member's skill, an enemy's.
pub const SE_START_PC: i32 = 104;
pub const SE_START_ENEMY: i32 = 159;

/// `particleGeneratorTbl` rows of the start's sparks, before the element's
/// place: a spell's or support skill's (`ccSkillCheckType` not 0), a
/// physical skill's or the normal attack's (0); each with `b` over the
/// head.
const ROW_SPELL: [usize; 2] = [164, 178];
const ROW_PHYSICAL: [usize; 2] = [157, 171];

/// The rings' CLUTs by the element's place (0 soil ... 5 dark, 6 none):
/// the `particleCcsAdrs` row each is re-coloured with (0: none), and the
/// one it replaces.
const EXEC_RING_CLUT: ([usize; 7], usize) = ([200, 198, 199, 0, 200, 201, 203], 197);
const FORCE_RING_CLUT: ([usize; 7], usize) = ([181, 0, 176, 178, 177, 179, 180], 175);
const FORCE_RING2_CLUT: ([usize; 7], usize) = ([167, 0, 162, 164, 163, 165, 166], 161);
/// The circles' sprites' CLUT (into their `ccTex` +0x3c).
const SUMMONS_CIRCLE_CLUT: [usize; 7] = [230, 0, 225, 227, 226, 228, 229];
const CIRCLE_CLUT: [usize; 7] = [130, 126, 127, 0, 128, 129, 131];

/// The rings' scales (gp tables, loaded onto `Main`'s stack): from and to
/// by `param` (0 or 1: `(from[p], to[p]) = (t[p], t[p + 2])`).
#[derive(Clone, Debug)]
pub struct Tables {
    /// 82 (main 0x00377f48): 0 -> 2, 2 -> 0.
    pub exec_ring: [F; 4],
    /// 83 (0x00377f58): 0.5 -> 2.5, 2.5 -> 0.5.
    pub force_ring: [F; 4],
    /// 84 (0x00377f68): x, y 1 -> 3 and 3 -> 1; z (+4) 0.2 -> 1.2 and 1.2
    /// -> 0.2.
    pub force_ring2: [F; 8],
    /// 85, 86 (0x00377f88): 0 -> 2, 2 -> 0; and (0x00377f98) the fades'
    /// in and out times, `FadeInOut(t[p], t[p + 2], life)`.
    pub circle: [F; 4],
    pub circle_fade: [i32; 4],
}

impl Tables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Tables {
        use crate::spell::{bits, first};
        let t = piney_data::tables::effect::of(volume);
        Tables {
            exec_ring: bits(t.exec_ring()),
            force_ring: bits(t.force_ring()),
            force_ring2: bits(t.force_ring2()),
            circle: bits(t.circle()),
            circle_fade: first(t.circle_fade()),
        }
    }
}

/// The element's place by `type & 0xfc`: soil 0, water 1, fire 2, wind 3,
/// thunder 4, dark 5, anything else 6.
pub fn element_index(ty: i32) -> usize {
    match ty & 0xfc {
        0x04 => 0,
        0x08 => 1,
        0x10 => 2,
        0x20 => 3,
        0x40 => 4,
        0x80 => 5,
        _ => 6,
    }
}

/// `_ccSkillCheckType(type)` (gcmn 0x00573c30): bit 0 (physical): 0 with
/// any of bits 10-12 (every physical skill of `skillTbl`), else -1; bit 1
/// (magic): 3 with bit 18 (the Repths), 2 with bit 16, else -2 with bit 17,
/// else 1; neither: -1.
pub fn check_type_of(ty: i32) -> i32 {
    if ty & 1 != 0 {
        if ty & 0x1c00 == 0 { -1 } else { 0 }
    } else if ty & 2 != 0 {
        if ty & 0x4_0000 != 0 {
            3
        } else if ty & 0x1_0000 != 0 {
            2
        } else if ty & 0x2_0000 == 0 {
            1
        } else {
            -2
        }
    } else {
        -1
    }
}

/// `ccSkillCheckType(sp)` (gcmn 0x00573be0): 0 for `skillTbl[1]` (the
/// normal attack), else `_ccSkillCheckType(sp->type)`.
pub fn skill_check_type(ty: i32, normal_attack: bool) -> i32 {
    if normal_attack { 0 } else { check_type_of(ty) }
}

/// `from + cnt * ((to - from) / life)`, as the rings' cases compute it.
fn grow(from: F, to: F, life: i16, cnt: i16) -> F {
    let step = ee::div(ee::sub(to, from), ee::from_int(i32::from(life)));
    ee::add(from, ee::mul(ee::from_int(i32::from(cnt)), step))
}

/// A duplicated clump's CLUT swap `ChangeClut(ccParticleAdrs(new),
/// ccParticleAdrs(old))`, as (from, to).
fn clump_clut(cx: &Cx, new: usize, old: usize) -> Option<(u32, u32)> {
    let d = &cx.spells.data;
    d.particle_adrs(cx.assets, old).zip(d.particle_adrs(cx.assets, new)).map(|(f, t)| (f.object, t.object))
}

/// The start's shared part: the sparks on `ch`, the exec ring, the
/// controller, the force rings when `rings`, the sound.
#[allow(clippy::too_many_arguments)]
fn start(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    ch: CharRef,
    row: usize,
    ty: i32,
    flags: i32,
    rings: bool,
    b: i32,
) -> Option<usize> {
    let h = cx.host.char_height(ch);
    let over = if b != 0 { h } else { 0 };
    pfx::start(cx, row + element_index(ty), |g| {
        g.offset = [0, 0, over, 0];
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::CharPos(ch));
    });
    let off = [0, 0, h, 0];
    eff_skill_exec_ring(ctrl, cx, ch, off, ty, b);
    let i = ctrl.new_effect(cx, if b != 0 { START_B } else { START });
    if let Some(i) = i {
        let pos = cx.host.char_pos(ch);
        let e = &mut ctrl.effects[i];
        e.life_time = 7;
        e.param = ty;
        e.flags = flags;
        e.offset = off;
        e.target = Some(ch);
        e.pos_t = pos;
    }
    if rings {
        eff_skill_exec_force_ring2(ctrl, cx, ch, ty, b);
        eff_skill_exec_force_ring(ctrl, cx, ch, ty, b);
    }
    let pos = cx.host.char_pos(ch);
    let t = cx.host.char_type(ch);
    if t & 0x0700_000f != 0 {
        cx.raise(Event::Sound3d { se: SE_START_PC, pos });
    } else if t & 0xe0 != 0 {
        cx.raise(Event::Sound3d { se: SE_START_ENEMY, pos });
    }
    i
}

/// `effSkillStart(ch, sp, a, b)` (main 0x001d3650) with `sp`'s `type`
/// (+0x2c) and whether `sp` is `skillTbl[1]`: the controller -12 (-13).
pub fn eff_skill_start_param(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    ch: CharRef,
    ty: i32,
    normal_attack: bool,
    a: i32,
    b: i32,
) -> Option<usize> {
    let check = skill_check_type(ty, normal_attack);
    let base = if check != 0 { ROW_SPELL } else { ROW_PHYSICAL };
    let row = base[usize::from(b != 0)];
    start(ctrl, cx, ch, row, ty, check, a == 0, b)
}

/// `effSkillStart(ch, sid, a, b)` (main 0x001d35f0):
/// `effSkillStart(ch, ccGetSkillParam(sid), a, b)`.
pub fn eff_skill_start(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, sid: i32, a: i32, b: i32) -> Option<usize> {
    let ty = cx.spells.data.skill_type(sid);
    eff_skill_start_param(ctrl, cx, ch, ty, sid == 1, a, b)
}

/// `effSkillStartEffect(ch, attr, b)` (main 0x001d39d0).
pub fn eff_skill_start_effect(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, attr: i32, b: i32) -> Option<usize> {
    start(ctrl, cx, ch, ROW_SPELL[usize::from(b != 0)], attr, 1, true, b)
}

/// A ring on `ch`'s pos: `id`, life, param `b`, re-coloured by `clut`.
#[allow(clippy::too_many_arguments)]
fn ring(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    id: i16,
    life: i16,
    ch: CharRef,
    off: Option<V4>,
    ty: i32,
    b: i32,
    clut: ([usize; 7], usize),
) -> Option<usize> {
    let i = ctrl.new_effect(cx, id)?;
    let row = clut.0[element_index(ty)];
    let swap = if row != 0 { clump_clut(cx, row, clut.1) } else { None };
    let e = &mut ctrl.effects[i];
    e.life_time = life;
    e.param = b;
    if let Some(off) = off {
        e.offset = off;
    }
    e.pos_ptr = Some(VecRef::CharPos(ch));
    if row != 0 {
        e.clut_swap = swap;
    }
    Some(i)
}

/// `effSkillExecRing(ch, off, type, b)` (main 0x001d2d80): 82.
pub fn eff_skill_exec_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, off: V4, ty: i32, b: i32) -> Option<usize> {
    ring(ctrl, cx, EXEC_RING, 20, ch, Some(off), ty, b, EXEC_RING_CLUT)
}

/// `effSkillExecForceRing(ch, type, b)` (main 0x001d2f50): 83.
pub fn eff_skill_exec_force_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, ty: i32, b: i32) -> Option<usize> {
    ring(ctrl, cx, FORCE_RING, 30, ch, None, ty, b, FORCE_RING_CLUT)
}

/// `effSkillExecForceRing2(ch, type, b)` (main 0x001d3100): 84.
pub fn eff_skill_exec_force_ring2(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, ty: i32, b: i32) -> Option<usize> {
    ring(ctrl, cx, FORCE_RING2, 30, ch, None, ty, b, FORCE_RING2_CLUT)
}

/// A circle sprite on `ch`'s pos: `id`, life 25, param `b`, offset `off`,
/// its `ccTex`'s CLUT `particleCcsAdrs[clut[e]]`.
#[allow(clippy::too_many_arguments)]
fn circle(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    id: i16,
    ch: CharRef,
    off: V4,
    ty: i32,
    b: i32,
    clut: [usize; 7],
) -> Option<usize> {
    let i = ctrl.new_effect(cx, id)?;
    let row = clut[element_index(ty)];
    let c = if row != 0 { cx.spells.data.particle_adrs(cx.assets, row) } else { None };
    let e = &mut ctrl.effects[i];
    e.life_time = 25;
    e.param = b;
    e.offset = off;
    e.pos_ptr = Some(VecRef::CharPos(ch));
    if row != 0
        && let Obj::Eff(eff) = &mut e.obj
    {
        eff.clut = c;
    }
    Some(i)
}

/// `effSkillExecSummonsCircle(ch, off, type, b)` (main 0x001d32b0): 85.
pub fn eff_skill_exec_summons_circle(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    ch: CharRef,
    off: V4,
    ty: i32,
    b: i32,
) -> Option<usize> {
    circle(ctrl, cx, SUMMONS_CIRCLE, ch, off, ty, b, SUMMONS_CIRCLE_CLUT)
}

/// `effSkillExecCircle(ch, off, type, b)` (main 0x001d3450): 86.
pub fn eff_skill_exec_circle(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    ch: CharRef,
    off: V4,
    ty: i32,
    b: i32,
) -> Option<usize> {
    circle(ctrl, cx, CIRCLE, ch, off, ty, b, CIRCLE_CLUT)
}

/// -12 and -13's case of the second chain (main 0x001cac4c).
pub fn start_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let Some(t) = e.target.filter(|&t| cx.host.check_target(t)) else {
        e.end_flag = true;
        return;
    };
    if e.age != e.life_time {
        return;
    }
    let b = i32::from(e.id == START_B);
    let (off, ty, flags) = (e.offset, e.param, e.flags);
    if flags != 0 {
        eff_skill_exec_summons_circle(ctrl, cx, t, off, ty, b);
    } else {
        eff_skill_exec_circle(ctrl, cx, t, off, ty, b);
    }
}

/// The table's (from, to) by `param`.
fn pick(t: &[F], p: i32) -> (F, F) {
    let p = p as usize;
    (t[p], t[p + 2])
}

/// 82's case of the first switch (main 0x001c4ee0).
pub fn exec_ring_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let t = cx.spells.data.skill_start.exec_ring;
    let e = &mut ctrl.effects[i];
    e.pos[2] = ee::add(e.pos[2], e.offset[2]);
    let (from, to) = pick(&t, e.param);
    let s = grow(from, to, e.life_time, e.cnt);
    e.scale[0] = s;
    e.scale[1] = s;
    e.scale[2] = s;
    let life = i32::from(e.life_time);
    e.fade_in_out(2, 15, life);
    Next::Draw
}

/// 83's case of the first switch (main 0x001c4f78).
pub fn force_ring_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let t = cx.spells.data.skill_start.force_ring;
    let e = &mut ctrl.effects[i];
    let (from, to) = pick(&t, e.param);
    let s = grow(from, to, e.life_time, e.cnt);
    e.scale[0] = s;
    e.scale[1] = s;
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 15, life);
    Next::Draw
}

/// 84's case of the first switch (main 0x001c4ffc).
pub fn force_ring2_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let t = cx.spells.data.skill_start.force_ring2;
    let e = &mut ctrl.effects[i];
    let (from, to) = pick(&t, e.param);
    let s = grow(from, to, e.life_time, e.cnt);
    e.scale[0] = s;
    e.scale[1] = s;
    let (from, to) = pick(&t[4..], e.param);
    e.scale[2] = grow(from, to, e.life_time, e.cnt);
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 15, life);
    Next::Draw
}

/// 85 and 86's case of the first switch (main 0x001c50e0): the sprite's
/// scale.
pub fn circle_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let d = &cx.spells.data.skill_start;
    let (t, fade) = (d.circle, d.circle_fade);
    let e = &mut ctrl.effects[i];
    e.pos[2] = ee::add(e.pos[2], e.offset[2]);
    let (from, to) = pick(&t, e.param);
    let s = grow(from, to, e.life_time, e.cnt);
    if let Obj::Eff(eff) = &mut e.obj {
        eff.scale_y = s;
        eff.scale_x = s;
    }
    let p = e.param as usize;
    let life = i32::from(e.life_time);
    e.fade_in_out(fade[p], fade[p + 2], life);
    Next::Draw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Camera, DrawRec};
    use crate::ee::{ONE, k};
    use crate::host::{CharState, Simple};

    #[test]
    fn the_skill_check_type() {
        assert_eq!(check_type_of(0x401), 0);
        assert_eq!(check_type_of(0x1), -1);
        assert_eq!(check_type_of(0x2 | 0x4_0000), 3);
        assert_eq!(check_type_of(0x2 | 0x1_0000), 2);
        assert_eq!(check_type_of(0x2), 1);
        assert_eq!(check_type_of(0x2 | 0x2_0000), -2);
        assert_eq!(check_type_of(0), -1);
        assert_eq!(skill_check_type(0x401, true), 0);
        assert_eq!(element_index(0x10 | 0x401), 2);
        assert_eq!(element_index(0x0c), 6);
    }

    /// A skill's start: the sparks, the controller and its three rings,
    /// then at count 7 the circle; everything gone by frame 40.
    #[test]
    fn a_skill_start_runs_its_rings_and_circle() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut r = || 0;
        let eye = [0, k(-600.0), k(300.0), ONE];
        let cam = Camera { eye, cam_pos: eye, cam_view: [0, 0, k(100.0), ONE], ..Camera::default() };
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], cam);
        host.chars.push(CharState { id: 3, pos: [0, 0, 0, ONE], dirc: [0; 4], height: k(160.0), width: k(45.0) });
        // Kite's Saber Dance (sid 6): a physical skill, so the circle 86.
        let i = fx.skill_start(&mut host, 3, 6, 0, 0).unwrap();
        assert_eq!(fx.ctrl.effects[i].id, START);
        let ids: Vec<i16> = fx.ctrl.effects.iter().filter(|e| e.status != 0).map(|e| e.id).collect();
        assert_eq!(ids, [EXEC_RING, START, FORCE_RING2, FORCE_RING]);
        assert_eq!(fx.particles.started.len(), 1);
        let mut circles = 0;
        for f in 0..40 {
            fx.step(&mut host);
            if f == 7 {
                circles = fx.ctrl.effects.iter().filter(|e| e.status != 0 && e.id == CIRCLE).count();
            }
            assert!(fx.draws().iter().all(|d| matches!(d, DrawRec::Clump { .. } | DrawRec::Eff { .. })));
        }
        assert_eq!(circles, 1);
        assert!(fx.ctrl.effects.iter().all(|e| e.status == 0));
    }
}
