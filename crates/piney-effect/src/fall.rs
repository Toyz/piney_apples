//! The falling spells (soil, fire, thunder, dark): `ccSkill::FallSystem`
//! (gcmn 0x00577a30), the meteor `effMeteoFireBall2` (main 0x001cd4e0) and
//! its landing, and the flare rings and debris it leaves. Level 1's
//! `fsDelay` of 0 is a divide by zero, which on the EE leaves the count: a
//! meteor at 20 only. Levels 3 and up use the fall element. The rules are in
//! docs/engine/effects.md ("FallSystem").

use crate::debris;
use crate::drawelm::{self, DrawElm, PI};
use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR};
use crate::element::{Base, Element};
use crate::particle::{FfParam, GenParam};
use crate::spell::{Spell, attr, bits, camera_shake, check_camera_shake_range, first, noise};
use crate::thunder::{self, ThunderBolt, ThunderData};
use crate::{CharRef, Cx, Event, IntRef, NO_HIT, VecRef, pfx, space, vu};
use piney_data::volume::Volume;

/// ANM_x300, the fire meteor's animation: it shares the meteors' cases of
/// both passes, but no function of any volume makes it.
pub const METEOR_ANM: i16 = 0;
pub const FIRE_FLAME: i16 = 5;
pub const FIRE_METEOR: i16 = 6;
pub const FLARE_RING: i16 = 7;
pub const SOIL_METEOR: i16 = 87;
pub const DARK_METEOR: i16 = 88;
pub const THUNDER_METEOR: i16 = -14;

/// The fall's tables.
#[derive(Clone, Debug, PartialEq)]
pub struct FallTables {
    /// `FallSystemLevelTbl` (main 0x00378260): meteors by level.
    pub level_tbl: [i16; 4],
    /// `fsDelay` (main 0x00378268): frames between them.
    pub delay: [i16; 4],
    /// `effMeteoFireBall2`'s `et[4][3]` (main 0x003401d0): the smoke
    /// generators by kind (soil, fire, thunder, dark).
    pub et: [[i16; 3]; 4],
    /// `skillThunderEff` (main 0x0033fcc0): the thunder meteor's bolt.
    pub skill_thunder_eff: ThunderData,
    /// The fall element's counts by level (gcmn 0x005ed510: 1, 4, 8, 15),
    /// the dark balls' sizes (0x005ed520), the damage counts (0x005ed540,
    /// each + 5).
    pub elements: [i32; 4],
    pub dark_scale: [F; 4],
    pub hits: [i32; 4],
    /// The thunder fall's bolts by level (0x005ed550: 1, 2, 3, 8), its
    /// damage counts (0x005ed560, each + 25), and the generator it leaves
    /// (`effElementGeneratorTbl` + 0x70, 0x005ecfe0, with the force fields
    /// `effElementFFTbl` + 0x120 and + 0x100).
    pub bolts: [i32; 4],
    pub bolt_hits: [i32; 4],
    pub thunder_gen: GenParam,
    pub thunder_ff: [FfParam; 2],
}

impl FallTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> FallTables {
        let t = piney_data::tables::effect::of(volume);
        let e: [i16; 12] = first(t.fall_et());
        FallTables {
            level_tbl: first(t.fall_level()),
            delay: first(t.fall_delay()),
            et: [[e[0], e[1], e[2]], [e[3], e[4], e[5]], [e[6], e[7], e[8]], [e[9], e[10], e[11]]],
            skill_thunder_eff: ThunderData::of(&t.thunder_eff()),
            elements: first(t.fall_elements()),
            dark_scale: bits(t.fall_dark_scale()),
            hits: first(t.fall_hits()),
            bolts: first(t.fall_bolts()),
            bolt_hits: first(t.fall_bolt_hits()),
            thunder_gen: GenParam::other(t.element_generators(), t.element_generators_va(), 2),
            thunder_ff: [
                FfParam::row(t.element_ffs(), t.element_ffs_va(), 9),
                FfParam::row(t.element_ffs(), t.element_ffs_va(), 8),
            ],
        }
    }
}

/// The EE's `div`: the remainder, and the dividend itself for a divisor of
/// 0 (the hardware's answer; the compiler put no check in).
fn rem(a: i32, b: i32) -> i32 {
    if b == 0 { a } else { a.wrapping_rem(b) }
}

/// `ccSkill::FallSystem` (gcmn 0x00577a30). From Mutation on (MUT gcmn
/// 0x0059d1e0) it aims at [`Spell::aim`] and lets the caster go at count
/// 120 or at the end, whichever is first, instead of at count 0 (still
/// then when the target is gone).
pub fn fall_system(ctrl: &mut EffectCtrl, cx: &mut Cx, k: usize) {
    const START: i16 = 20;
    const RELEASE: i16 = 120;
    let attr = cx.spells.runs[k].attr(&cx.spells.data);
    let inf = cx.assets.volume == Volume::Inf;
    // From Mutation on: the caster let go early, before count 120.
    let early = |cx: &mut Cx, s: &Spell| {
        if !inf && s.count < RELEASE {
            s.release(cx);
        }
    };
    if cx.spells.runs[k].count == 0 {
        let s = cx.spells.runs[k].clone();
        match s.target {
            Some(t) if cx.host.check_target(t) => {
                crate::tornado::eff_magic_attack_sign(cx, t, attr);
                if cx.spells.data.skill_type(s.id) & 0x10 != 0 {
                    pfx::start_particle_effect(cx, t, 38);
                }
            }
            _ => {
                if !inf {
                    s.release(cx);
                }
                cx.spells.runs[k].status = 1;
            }
        }
        if inf {
            s.release(cx);
        }
        let tbl = cx.spells.data.fall.level_tbl;
        let s = &mut cx.spells.runs[k];
        match attr {
            attr::DARK => s.level = s.id - 272,
            attr::THUNDER => s.level = s.id - 256,
            attr::FIRE => s.level = s.id - 224,
            attr::SOIL => s.level = s.id - 192,
            _ => {}
        }
        s.temp_cnt = i32::from(tbl.get((s.level - 1) as usize).copied().unwrap_or(0));
    }
    let s = cx.spells.runs[k].clone();
    if s.count >= START && s.step < level_tbl(cx, s.level) {
        if s.level >= 3 {
            if s.elm.is_none() {
                let e = fall_element_generate(cx, k, attr, s.level);
                cx.spells.runs[k].elm = e;
                if let Some(e) = e
                    && let Some(b) = cx.spells.elements.base_mut(e)
                {
                    b.skill = Some(s.key);
                }
            }
        } else if rem(i32::from(s.count - START), i32::from(delay(cx, s.level))) == 0 {
            match s.target {
                Some(t) if cx.host.check_target(t) => throw(ctrl, cx, k, t, attr),
                _ => {
                    early(cx, &s);
                    cx.spells.runs[k].status = 1;
                }
            }
        }
    }
    let s = cx.spells.runs[k].clone();
    if !inf && s.count == RELEASE {
        s.release(cx);
    }
    if s.level >= 3 {
        if let Some(e) = s.elm
            && cx.spells.elements.get(e).is_some_and(|e| e.deleted())
        {
            early(cx, &s);
            let s = &mut cx.spells.runs[k];
            s.elm = None;
            s.hold = false;
            s.status = 1;
        }
        return;
    }
    if s.count < 21 {
        return;
    }
    let aim = s.aim(cx.assets.volume);
    for i in 0..8 {
        let Some(slot) = cx.spells.runs[k].eff_ptr[i] else { continue };
        if !ctrl.effects[slot].end_flag {
            continue;
        }
        let s = &mut cx.spells.runs[k];
        s.eff_ptr[i] = None;
        s.temp_cnt -= 1;
        let s = s.clone();
        cx.raise(Event::SkillDamage2 {
            spell: s.key,
            attacker: s.creator,
            target: s.target,
            pos: aim,
            ttype: s.t_type,
            sid: s.id,
        });
    }
    let s = cx.spells.runs[k].clone();
    if s.temp_cnt == 0 && s.step >= level_tbl(cx, s.level) {
        early(cx, &s);
        let s = &mut cx.spells.runs[k];
        s.hold = false;
        s.status = 1;
    }
}

fn level_tbl_of(t: &FallTables, level: i32) -> i16 {
    t.level_tbl.get((level - 1) as usize).copied().unwrap_or(0)
}

fn level_tbl(cx: &Cx, level: i32) -> i16 {
    level_tbl_of(&cx.spells.data.fall, level)
}

fn delay(cx: &Cx, level: i32) -> i16 {
    cx.spells.data.fall.delay.get((level - 1) as usize).copied().unwrap_or(0)
}

/// A meteor: `FallSystem`'s throw at count 20 (and on).
fn throw(ctrl: &mut EffectCtrl, cx: &mut Cx, k: usize, t: CharRef, attr: i32) {
    let s: Spell = cx.spells.runs[k].clone();
    let aim = s.aim(cx.assets.volume);
    let mut p = aim;
    let zz = p[2];
    let rn = cx.host.rand() >> 3;
    let mut r = cx.host.char_width(t);
    if ee::lt(r, 0x4348_0000) {
        r = ee::mul(r, 0x3fcc_cccd);
    }
    r = ee::sub(r, ee::div(ee::mul(ee::mul(0x3f4c_cccd, r), ee::from_int(rn % 101)), 0x42c8_0000));
    let a = ee::deg2rad(((rn & 0x3f00) - 4096) as i16);
    p[2] = ee::mul(r, ee::sinf(a));
    let r = ee::mul(r, ee::cosf(a));
    let a = ee::deg2rad(((rn << 8) & 0xff00) as i16);
    p[0] = ee::mul(r, ee::sinf(a));
    p[1] = ee::mul(ee::neg(r), ee::cosf(a));
    p[3] = ONE;
    let top = ee::add(p[2], ee::add(0x43fa_0000, ee::add(zz, cx.host.char_height(t))));
    p[2] = top;
    if !ee::lt(top, 0x4461_0000) {
        let (area, dungeon, field_type) = cx.host.area();
        if (field_type == 4 && dungeon == 0) || area == 2 {
            p[2] = 0x445e_8000;
        }
    }
    let m = eff_meteo_fire_ball2(ctrl, cx, p, t, attr);
    let s = &mut cx.spells.runs[k];
    let n = s.eff_num;
    if let Some(slot) = s.eff_ptr.get_mut(n as usize) {
        *slot = m;
    }
    s.eff_num = n.wrapping_add(1);
    s.step = s.step.wrapping_add(1);
    cx.raise(Event::Sound3d { se: 57, pos: aim });
}

/// `effMeteoFireBall2(p, t, atr)` (main 0x001cd4e0): the meteor.
pub fn eff_meteo_fire_ball2(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, t: CharRef, atr: i32) -> Option<usize> {
    let (p1, p2, p3) = match atr {
        attr::SOIL => (SOIL_METEOR, -1, 0),
        attr::FIRE => (FIRE_METEOR, FIRE_FLAME, 1),
        attr::THUNDER => (THUNDER_METEOR, -1, 2),
        attr::DARK => (DARK_METEOR, -1, 3),
        _ => (-1, -1, -1),
    };
    // Not on the lists: the game stores to address 0 (a deliberate crash).
    let i = ctrl.new_effect(cx, p1)?;
    let tpos = cx.host.char_pos(t);
    let e = &mut ctrl.effects[i];
    e.life_time = 300;
    e.param = atr;
    e.offset = p;
    e.target = Some(t);
    e.pos_t = tpos;
    let mut r = VF0;
    match p3 {
        1 => {
            r[0] = ee::deg2rad(16384);
            e.rot = r;
        }
        3 => {
            r[0] = ee::deg2rad(8192);
            e.rot = r;
            e.scale = ee::vscale(ONE_VECTOR, 0x4080_0000);
            // SetFogSw(clump, 1).
        }
        _ => {
            let rn = cx.host.rand() >> 3;
            r[0] = ee::deg2rad((rn & 0x7000) as i16);
            r[1] = ee::deg2rad(((rn << 4) & 0x7000) as i16);
            r[2] = ee::deg2rad(((rn << 8) & 0xf000) as i16);
            ctrl.effects[i].rot = r;
            // Soil: SetFogSw(clump, 1).
        }
    }
    let e = &mut ctrl.effects[i];
    e.speed[2] = 0xc1a0_0000;
    let mut r = ee::vadd(tpos, p);
    r[2] = p[2];
    e.pos = r;
    e.flags = 1;
    if p2 > 0
        && let Some(j) = ctrl.new_effect(cx, p2)
    {
        let f = &mut ctrl.effects[j];
        f.life_time = 300;
        f.link = Some(i);
    }
    let z0 = tpos[2];
    let mut pp = ee::vadd(tpos, p);
    pp[3] = ONE;
    let z = cx.host.land_hit_check2(pp, ee::mul(0xbf80_0000, ee::add(0x43fa_0000, pp[2])), 0x2000_0000);
    ctrl.effects[i].temp[0] = if ee::eq(z, NO_HIT) { z0 } else { z };
    let et = cx.spells.data.fall.et[p3.clamp(0, 3) as usize];
    for row in et {
        if row > 0 {
            pfx::start(cx, row as usize, |g| {
                g.sync_pos_type = false;
                g.sync_pos = Some(VecRef::EffectPos(i));
                g.sync_sw = Some(IntRef::EffectFlags(i));
            });
        }
    }
    Some(i)
}

/// Meteors 0, 6, 87 and 88's case of the first switch (main 0x001c41a0):
/// FadeInOut(10, 3); turning about y (not the dark one); at the target's
/// position plus the offset, z the offset's.
pub fn meteor_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let life = i32::from(e.life_time);
    e.fade_in_out(10, 3, life);
    if e.id != DARK_METEOR {
        e.rot[1] = ee::deg2rad((ee::rad2deg(e.rot[1]) as i32 + 1024) as i16);
    }
    if let Some(t) = e.target
        && cx.host.check_target(t)
    {
        e.pos_t = cx.host.char_pos(t);
    }
    e.pos = ee::vadd(e.pos_t, e.offset);
    e.pos[2] = e.offset[2];
    e.pos[3] = ONE;
    Next::Draw
}

/// The fire meteor's flame 5 (main 0x001c4128): FadeInOut(10, 3); with its
/// meteor's position, turn and flags, and its count once the meteor has
/// landed.
pub fn flame_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let life = i32::from(e.life_time);
    e.fade_in_out(10, 3, life);
    if let Some(l) = ctrl.effects[i].link {
        let (pos, rot, flags, cnt) = {
            let m = &ctrl.effects[l];
            (m.pos, m.rot, m.flags, m.cnt)
        };
        let e = &mut ctrl.effects[i];
        e.pos = pos;
        e.rot = rot;
        e.flags = flags;
        if flags == 0 {
            e.cnt = cnt;
            e.age = cnt;
        }
    }
    Next::Draw
}

/// Meteors 0, 6, 87 and 88's case of the second chain (main 0x001c8900):
/// the fall, and the landing.
pub fn meteor_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    if e.flags == 0 {
        return;
    }
    if ee::lt(e.temp[0], e.pos[2]) {
        e.speed[2] = ee::sub(e.speed[2], 0x404c_cccd);
        e.offset[2] = ee::add(e.offset[2], e.speed[2]);
        return;
    }
    let (pos, param, z) = (e.pos, e.param, e.temp[0]);
    eff_skill_break_se(cx, pos, param);
    let mut p = pos;
    p[2] = z;
    if check_camera_shake_range(cx, p) {
        camera_shake(cx, 0, 1, 10, 0);
    }
    if param & 0x14 != 0 {
        debris::eff_smoke_rock(ctrl, cx, p, debris::down(), 0x4234_0000, 9, -1);
    } else if param & 0x80 != 0 {
        debris::eff_dark_smoke(ctrl, cx, p, debris::down(), 0x4234_0000, 9);
    }
    let n = match param {
        attr::SOIL => 6,
        attr::FIRE => 1,
        attr::DARK => 4,
        _ => 1,
    };
    p[2] = ee::add(p[2], 0x41a0_0000);
    eff_flare_ring(ctrl, cx, p, n);
    if n == 1 {
        for row in [91, 93] {
            pfx::start(cx, row, |g| g.pos = p);
        }
    }
    let e = &mut ctrl.effects[i];
    e.flags = 0;
    let t = e.life_time.wrapping_sub(3);
    e.cnt = t;
    e.age = t;
}

/// The thunder meteor -14's case of the second chain (main 0x001c8b98):
/// at its count 20 (the value the chain's compares left), the bolt.
pub fn thunder_meteor_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    if ctrl.effects[i].cnt != 20 {
        return;
    }
    let z = ctrl.effects[i].temp[0];
    ctrl.effects[i].pos[2] = z;
    let (pos, param) = (ctrl.effects[i].pos, ctrl.effects[i].param);
    let k = thunder::new_effect2(&mut cx.spells.effect2, 0);
    let dat = cx.spells.data.fall.skill_thunder_eff;
    let bolt = ThunderBolt::new(cx, pos, 1, ONE, 1, Some(dat));
    if let Some(k) = k {
        cx.spells.effect2[k].bolt = Some(Box::new(bolt));
    }
    cx.raise(Event::Sound3d { se: 41, pos });
    eff_skill_break_se(cx, pos, param);
    if check_camera_shake_range(cx, pos) {
        camera_shake(cx, 0, 1, 10, 0);
    }
    let mut p = pos;
    p[2] = ee::add(p[2], 0x41a0_0000);
    eff_flare_ring(ctrl, cx, p, 2);
    for row in [190, 191] {
        pfx::start(cx, row, |g| g.pos = p);
    }
    let e = &mut ctrl.effects[i];
    e.cnt = e.life_time;
    e.age = e.life_time;
}

/// `effSkillBreakSE(pos, atr)` (main 0x001d80c0): the landing's sound by
/// element (soil 65, water 66, wind 67, thunder 68, dark 69, fire 35).
pub fn eff_skill_break_se(cx: &mut Cx, pos: V4, atr: i32) {
    let se = match atr & 0xfc {
        attr::FIRE => 35,
        attr::DARK => 69,
        attr::THUNDER => 68,
        attr::WIND => 67,
        attr::WATER => 66,
        attr::SOIL => 65,
        _ => return,
    };
    cx.raise(Event::Sound3d { se, pos });
}

/// `effFlareRing(p, n)` (main 0x001ccec0): the flare ring 7, life 15; for
/// n 1-6 re-coloured (CLUT 190 + n of `particleCcsAdrs` for 190).
pub fn eff_flare_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, n: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, FLARE_RING)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 15;
    e.pos = p;
    if (1..7).contains(&n) {
        let d = &cx.spells.data;
        e.clut_swap = d
            .particle_adrs(cx.assets, 190)
            .zip(d.particle_adrs(cx.assets, (190 + n) as usize))
            .map(|(from, to)| (from.object, to.object));
    }
    Some(i)
}

/// The flare ring 7's case of the first switch (main 0x001c4318): scale
/// 1 + cnt (0.9 - 0.03 cnt); FadeOut(10, lifeTime).
pub fn flare_ring_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let c = ee::from_int(i32::from(e.cnt));
    let s = ee::add(ONE, ee::mul(c, ee::sub(0x3f66_6666, ee::mul(0x3cf5_c28f, c))));
    e.scale = ee::vscale(ONE_VECTOR, s);
    let life = i32::from(e.life_time);
    e.fade_out(10, life);
    Next::Draw
}

/// `ccHitCheckLM2`'s answer for no hit: -1.0.
const NO_LM_HIT: F = 0xbf80_0000;

/// `ccFallElementGenerate(skill, attr, level)` (gcmn 0x004fef30): the
/// thunder fall for thunder, else the fall element, in the first empty
/// slot of the manager (none: deleted, None).
pub fn fall_element_generate(cx: &mut Cx, k: usize, atr: i32, level: i32) -> Option<usize> {
    let s = cx.spells.runs[k].clone();
    let e = if atr & 0xfc == attr::THUNDER {
        Element::ThunderFall(Box::new(ThunderFallElement::new(cx, &s, level)))
    } else {
        Element::Fall(Box::new(FallElement::new(cx, &s, atr & 0xfc, level)))
    };
    cx.spells.elements.add(e)
}

/// One of `ccFallElement`'s `ELEMENT_T`s (28 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct FallSlot {
    /// +0x00 the drawn element (the class's own `new`).
    pub elm: DrawElm,
    /// +0x04 `height`, +0x08 `radius`, +0x0c `rad` (the spiral's; unset
    /// from level 3), +0x10 `interval`, +0x14 `status`, +0x18 `seOn`.
    pub height: F,
    pub radius: F,
    pub rad: F,
    pub interval: i32,
    pub status: i32,
    pub se_on: i32,
}

/// `ccFallElement` (gcmn effect2.cpp, 0x5c0 bytes): the meteors of levels 3
/// and 4, falling on the target from all round it.
#[derive(Clone, Debug, PartialEq)]
pub struct FallElement {
    pub base: Base,
    /// +0x590 `m_elmNum` (`ccMoveElement`'s; its `m_drawElmTbl` stays
    /// empty).
    pub elm_num: i32,
    /// +0x5a0 `SPIRAL_RADIUS`, +0x5a4 `SPIRAL_HEIGHT` (level 2's), +0x5a8
    /// `m_hitCheck`, +0x5ac `m_velocity`, +0x5b0 `m_skillPtr` (its own),
    /// +0x5b4 `m_iceShockSEOne`.
    pub spiral_radius: F,
    pub spiral_height: F,
    pub hit_check: i32,
    pub velocity: F,
    pub skill: u32,
    pub ice_shock_se_one: i32,
    /// +0x5b8 `m_elements`.
    pub elements: Vec<FallSlot>,
}

impl FallElement {
    /// `new ccFallElement(skill, attr, level)` (gcmn 0x004e9610), levels 3
    /// and 4 (level 2's spiral is not reached by `FallSystem`).
    pub fn new(cx: &mut Cx, s: &Spell, atr: i32, level: i32) -> FallElement {
        let t = cx.spells.data.fall.clone();
        let n = t.elements.get((level - 1) as usize).copied().unwrap_or(0);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let top = space::w2p(s.t_pos, player, bounds);
        let mut base = Base { attr: atr, level, target: s.target, ..Base::default() };
        base.anim.pos = s.t_pos;
        let mut elements = Vec::new();
        for _ in 0..n.max(0) {
            let (mut elm, scale) = match atr {
                attr::FIRE => (DrawElm::fire(cx), 0x3fc0_0000),
                attr::SOIL => (DrawElm::rock(cx), 0x3fc0_0000),
                attr::DARK => (DrawElm::dark(cx), t.dark_scale.get((level - 1) as usize).copied().unwrap_or(0)),
                attr::WATER => {
                    let r = (cx.host.genrand() as i32 & 3).wrapping_abs() + 4;
                    (DrawElm::ice(cx, r), 0x4080_0000)
                }
                // The game stores to address 0 here.
                _ => (DrawElm::rock(cx), 0x3fc0_0000),
            };
            for k in 0..3 {
                elm.base.anim.scale[k] = scale;
            }
            elements.push(FallSlot { elm, height: 0, radius: 0, rad: 0, interval: 0, status: 0, se_on: 0 });
        }
        let mut e = FallElement {
            base,
            elm_num: n,
            spiral_radius: 0,
            spiral_height: 0,
            hit_check: 0,
            velocity: 0,
            skill: s.key,
            ice_shock_se_one: 0,
            elements,
        };
        if level == 3 || level == 4 {
            e.hit_check = 1;
            e.velocity = 0x4120_0000;
            let mut ang: F = 0;
            for i in 0..e.elements.len() {
                let mut tp = space::w2p(s.t_pos, player, bounds);
                let x = thunder::add_abs(100.0, thunder::rand_f(cx, 0x43c8_0000));
                let v = ee::apply(&vu::rot_z(&vu::UNIT, ang), [x, 0, 0x442a_0000, ONE]);
                tp = ee::vadd(tp, v);
                let sp = space::p2w(tp, player, bounds);
                let mut ep = tp;
                ep[2] = ee::add(ep[2], 0x447a_0000);
                if ee::eq(cx.host.hit_check_lm2(tp, &mut ep, 0x2000_0000), NO_LM_HIT) {
                    thunder::rand_f(cx, PI);
                    ep = tp;
                    ep[2] = ee::add(top[2], thunder::rand_f(cx, 0x4348_0000));
                }
                let epw = space::p2w(ep, player, bounds);
                let d = drawelm::get_dist(tp, ep);
                let tilt = ee::atan2f(ee::sub(tp[2], ep[2]), d);
                let dirc = drawelm::get_dirc(tp, ep);
                let m = vu::rot_z(&vu::rot_z(&vu::rot_y(&vu::UNIT, tilt), dirc), 0xbfc9_0fdb);
                let slot = &mut e.elements[i];
                let a = &mut slot.elm.base.anim;
                a.sp = sp;
                a.ep = epw;
                a.pos = sp;
                a.speed = ee::apply(&m, [e.velocity, 0, 0, ONE]);
                slot.elm.base.accel = ee::apply(&m, [0x4120_0000, 0, 0, ONE]);
                let a = &mut slot.elm.base.anim;
                a.dirc[1] = tilt;
                a.dirc[2] = dirc;
                slot.interval = (i as i32 >> 1) * 5;
                ang = thunder::rand_f(cx, PI);
                slot.se_on = 0;
            }
        }
        e
    }

    /// `ccFallElement::Main` (gcmn 0x004ea300).
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let cnt = self.base.count;
        self.base.count = cnt + 1;
        let atr = self.base.attr;
        match self.base.proccess {
            0 => {}
            1 => {
                let c = self.base.count;
                self.base.count = c + 1;
                if c == 15 {
                    self.base.del_flag = 1;
                }
                return;
            }
            _ => return,
        }
        let mut done = true;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        for i in 0..self.elements.len() {
            if self.elements[i].elm.base.del_flag != 0 {
                continue;
            }
            let iv = self.elements[i].interval;
            self.elements[i].interval = iv - 1;
            if iv > 0 {
                continue;
            }
            let slot = &mut self.elements[i];
            if slot.se_on == 0 {
                // Thunder (41 at a stale stack vector) never reaches here:
                // its element is the thunder fall.
                if atr != attr::WATER && atr != attr::THUNDER {
                    cx.raise(Event::Sound3d { se: 57, pos: slot.elm.base.anim.pos });
                }
                slot.se_on = 1;
            }
            let a = &mut slot.elm.base.anim;
            let ep = a.ep;
            let p = ee::vadd(space::w2p(a.pos, player, bounds), a.speed);
            a.pos = space::p2w(p, player, bounds);
            let pos = a.pos;
            a.speed = ee::vadd(a.speed, slot.elm.base.accel);
            let a = &mut slot.elm.base.anim;
            if !ee::le(drawelm::get_dist3d(VF0, a.speed), 0x4348_0000) {
                a.speed = ee::vscale(ee::normalize(a.speed), 0x4348_0000);
            }
            slot.elm.main(cx);
            if ee::le(pos[2], ep[2]) {
                self.elements[i].elm.base.del_flag = 1;
                self.shock(ctrl, cx, ep);
            }
            done = false;
        }
        if atr != attr::WATER && self.hit_now(cnt, &cx.spells.data.fall.hits.clone(), 5) {
            damage2_of(cx, self.skill);
        }
        if done {
            self.base.count = 0;
            self.base.proccess += 1;
        }
    }

    fn hit_now(&self, cnt: i32, t: &[i32; 4], plus: i32) -> bool {
        t.iter().any(|&h| cnt == h + plus)
    }

    /// `ccFallElement::Shock(pos)` (gcmn 0x004ea6c0): an explosion, the
    /// debris (`effRadiateSomething2`), a shake, the sound.
    fn shock(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4) {
        let r = [ee::deg2rad(-16384), 0, thunder::rand_f(cx, PI), ONE];
        let kind = match self.base.attr & 0xfc {
            attr::SOIL => 5,
            attr::DARK => 4,
            attr::WIND => 3,
            attr::THUNDER => 2,
            attr::WATER => 1,
            _ => 0,
        };
        drawelm::eff_explode3(cx, pos, VF0, 0x4000_0000, kind);
        debris::eff_radiate_something2(ctrl, cx, pos, r, 0x41a0_0000, self.base.attr & 0xfc, 4);
        if check_camera_shake_range(cx, pos) {
            camera_shake(cx, 2, 2, 3, 2);
        }
        if self.base.attr == attr::WATER {
            cx.raise(Event::Sound3d { se: 65, pos });
            if self.ice_shock_se_one == 0 {
                cx.raise(Event::Sound3dNote { se: 66, pos, note: 48 });
                self.ice_shock_se_one = 1;
            }
        } else {
            eff_skill_break_se(cx, pos, self.base.attr);
        }
    }
}

/// `ccSkillDamage2(creator, target, tPos, tType, &acFlag, ID)` of a spell
/// still on `SkillEntryTop`.
pub(crate) fn damage2_of(cx: &mut Cx, key: u32) {
    if !cx.spells.entry_check(key) {
        return;
    }
    let Some(s) = cx.spells.get(key).cloned() else { return };
    cx.raise(Event::SkillDamage2 {
        spell: s.key,
        attacker: s.creator,
        target: s.target,
        pos: s.t_pos,
        ttype: s.t_type,
        sid: s.id,
    });
}

/// One of `ccThunderFallElement`'s `ELEMENT_T`s (12 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct ThunderFallSlot {
    /// +0x00 the bolt, +0x04 `interval`, +0x08 `status`.
    pub elm: DrawElm,
    pub interval: i32,
    pub status: i32,
}

/// `ccThunderFallElement` (gcmn effect2.cpp, 0x5c0 bytes): the lightning of
/// levels 3 and 4.
#[derive(Clone, Debug, PartialEq)]
pub struct ThunderFallElement {
    pub base: Base,
    /// +0x590 `m_elmNum`, +0x5a0 `m_elements`, +0x5a4 `m_skillPtr` (its
    /// own), +0x5b0 `m_tPos`.
    pub elm_num: i32,
    pub elements: Vec<ThunderFallSlot>,
    pub skill: u32,
    pub t_pos: V4,
}

impl ThunderFallElement {
    /// `new ccThunderFallElement(skill, level)` (gcmn 0x004ea890): bolts
    /// from 2000 up within 300 of the target (a random bearing) down 2100
    /// (to the target when no model is hit), 5 frames apart.
    pub fn new(cx: &mut Cx, s: &Spell, level: i32) -> ThunderFallElement {
        let n = cx.spells.data.fall.bolts.get((level - 1) as usize).copied().unwrap_or(0);
        let base = Base { level, target: s.target, ..Base::default() };
        let mut e = ThunderFallElement { base, elm_num: n, elements: Vec::new(), skill: s.key, t_pos: s.t_pos };
        if level == 3 || level == 4 {
            let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
            let tp = s.t_pos;
            let tpp = space::w2p(tp, player, bounds);
            for i in 0..n.max(0) {
                let mut elm = DrawElm::thunder(cx);
                let x = thunder::rand_f(cx, 0x4396_0000);
                let r = thunder::rand_f(cx, PI);
                let v = ee::apply(&vu::rot_z(&vu::UNIT, r), [x, 0, 0x44fa_0000, ONE]);
                let p = ee::vadd(tpp, v);
                let sp = space::p2w(p, player, bounds);
                let mut q = p;
                q[2] = ee::sub(q[2], 0x4503_4000);
                let mut ep = space::p2w(q, player, bounds);
                if ee::eq(cx.host.hit_check_lm2(sp, &mut ep, 0x2000_0000), NO_LM_HIT) {
                    ep = tp;
                }
                let a = &mut elm.base.anim;
                a.sp = sp;
                a.ep = ep;
                a.pos = sp;
                for k in 0..3 {
                    a.scale[k] = 0x4000_0000;
                }
                e.elements.push(ThunderFallSlot { elm, interval: i * 5, status: 0 });
                if i & 1 != 0 {
                    cx.raise(Event::Sound3dNote { se: 41, pos: ep, note: 67 });
                }
            }
        }
        e
    }

    /// `ccThunderFallElement::Main` (gcmn 0x004eae10).
    pub fn main(&mut self, cx: &mut Cx) {
        let cnt = self.base.count;
        self.base.count = cnt + 1;
        let mut done = true;
        if cx.spells.entry_check(self.skill)
            && let Some(s) = cx.spells.get(self.skill)
        {
            self.t_pos = s.t_pos;
        }
        for i in 0..self.elements.len() {
            let slot = &mut self.elements[i];
            if slot.elm.base.del_flag != 0 {
                continue;
            }
            let iv = slot.interval;
            slot.interval = iv - 1;
            if iv >= 0 {
                done = false;
                continue;
            }
            slot.elm.main(cx);
            if slot.elm.base.end_flag != 0 && slot.status == 0 {
                let ep = slot.elm.base.anim.ep;
                self.shock(cx, ep);
                self.elements[i].status += 1;
            }
            done = false;
        }
        let hits = cx.spells.data.fall.bolt_hits;
        if hits.iter().any(|&h| cnt == h + 25) && cx.spells.entry_check(self.skill) {
            damage2_of(cx, self.skill);
            let tp = cx.spells.get(self.skill).map_or(VF0, |s| s.t_pos);
            if check_camera_shake_range(cx, tp) {
                noise(cx, 20);
                camera_shake(cx, 2, 2, 20, 2);
            }
        }
        if done {
            self.base.del_flag = 1;
            let (param, ff) = (cx.spells.data.fall.thunder_gen, cx.spells.data.fall.thunder_ff);
            let tp = self.t_pos;
            pfx::start_param(cx, param, [Some(ff[0]), Some(ff[1]), None, None], |g| {
                g.pos = tp;
                g.p_tex_mod = 156;
            });
        }
    }

    /// `ccThunderFallElement::Shock(pos)` (gcmn 0x004eb0a0).
    fn shock(&mut self, cx: &mut Cx, pos: V4) {
        if self.base.level == 4 {
            if let Some(k) = drawelm::eff_explode3(cx, pos, VF0, ONE, 2)
                && let Some(Element::Draw(e)) = cx.spells.elements.slots[k].as_mut()
            {
                let a = &mut e.base.anim;
                let (ss, es) = ([0, 0, 0, ONE], [0x4080_0000, 0x4080_0000, 0x4080_0000, ONE]);
                a.start_scale = ss;
                a.end_scale = es;
                for i in 0..3 {
                    a.scale_spd[i] = ee::div(ee::sub(es[i], ss[i]), 0x4170_0000);
                }
                a.scale = ss;
                a.scale_flag = 1;
                a.scale_accel = VF0;
                a.start_fade = ONE;
                a.end_fade = 0;
                a.fade_spd = 0xbd08_8889;
                a.transparency = ONE;
                a.fade_flag = 1;
            }
            cx.raise(Event::Sound3dNote { se: 56, pos: self.t_pos, note: 72 });
        } else {
            drawelm::eff_explode3(cx, pos, VF0, 0x4000_0000, 2);
        }
        cx.raise(Event::Sound3dNote { se: 41, pos, note: 67 });
        eff_skill_break_se(cx, pos, attr::THUNDER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_delay_throws_only_at_the_start() {
        // fsDelay[0] is 0: the EE's divide leaves the count, 0 only at 20.
        assert_eq!(rem(0, 0), 0);
        assert_eq!(rem(5, 0), 5);
        assert_eq!(rem(40, 20), 0);
    }

    /// Id 0 (ANM_x300) set up as effMeteoFireBall2 sets up a fire meteor:
    /// it falls from 400 as the meteors do and lands on its temp[0], with
    /// the fire's sound and a flare ring.
    #[test]
    fn the_meteor_animation_falls_and_lands() {
        use crate::draw::Camera;
        use crate::ee::k;
        use crate::effect::Obj;
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut r = || 0;
        let mut host = crate::host::Simple::new(&mut r, [0, 0, 0, ONE], Camera::default());
        let i = {
            let (ctrl, cx) = fx.split(&mut host);
            ctrl.new_effect(&cx, METEOR_ANM).unwrap()
        };
        assert!(matches!(fx.ctrl.effects[i].obj, Obj::Anm { .. }));
        let e = &mut fx.ctrl.effects[i];
        e.life_time = 300;
        e.param = attr::FIRE;
        e.offset = [0, 0, k(400.0), ONE];
        e.pos_t = [0, 0, 0, ONE];
        e.speed[2] = 0xc1a0_0000;
        e.flags = 1;
        e.temp[0] = 0;
        let mut landed = None;
        for f in 0..40 {
            fx.step(&mut host);
            if landed.is_none() && fx.ctrl.effects.iter().any(|e| e.status != 0 && e.id == FLARE_RING) {
                landed = Some(f);
                assert_eq!(fx.ctrl.effects[i].flags, 0);
                assert!(fx.take_events().iter().any(|e| matches!(e, Event::Sound3d { se: 35, .. })));
            }
        }
        assert!(landed.is_some_and(|f| f > 5));
    }
}
