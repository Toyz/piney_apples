//! The tornado spells' effects (main effect.cpp): the magic attack sign every
//! attack spell starts on its target (`effMagicAttackSign` 0x001d1090), the
//! smoke (`effSkillTornadeSmoke` 0x001d48b0, generator 196) and the rings
//! (`effSkillTornadeRingsPos` 0x001d3d80) of `TornadoSystem`, and the rings'
//! motion (effects 89, 90 and 91; first switch 0x001c5198, 0x001c5320,
//! 0x001c5568), whose growth counts are not clamped past 30. The rules are in
//! docs/engine/effects.md ("TornadoSystem").

use piney_data::volume::Volume;

use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR, Obj};
use crate::element::{Base, Element};
use crate::particle::FfParam;
use crate::spell::{attr, camera_shake, check_camera_shake_range, first, noise};
use crate::{CharRef, Cx, Event, VecRef, pfx, ring, space, thunder};

pub const RING_A: i16 = 89;
pub const RING_B: i16 = 90;
pub const RING_C: i16 = 91;
/// `particleGeneratorTbl`'s row of the tornado smoke.
pub const SMOKE_ROW: usize = 196;

/// The rings' interpolation tables: `ccEffect::Main`'s function statics.
#[derive(Clone, Debug, PartialEq)]
pub struct RingTables {
    /// Effect 89 (main 0x0033ff50..0x0033ff90): scale x/y from `a` to `b`,
    /// z from `c` to `d`, by param.
    pub a89: [[F; 4]; 4],
    /// Effect 90 (0x0033ff90..0x00340030): the same four and the rise, by
    /// flags (0-1) and param; the turn (gp 0x00377fa8) by flags.
    pub t90: [[[F; 4]; 2]; 5],
    pub turn90: [i16; 2],
    /// Effect 91 (0x00340030..0x00340170), by flags (0-3) and param; the
    /// turn (gp 0x00377fb0).
    pub t91: [[[F; 4]; 4]; 5],
    pub turn91: [i16; 4],
    /// `skillTornadeSmokePFF1..3` +0x04 (u16), +0x18 (f32) by level - 1,
    /// and `particleGeneratorTbl[191 + level]` +0x08 `gRadius`, +0x14
    /// `pIV`: what the flying objects take from the tornado.
    pub pff2_04: [u16; 4],
    pub pff1_18: [F; 4],
    pub g_radius: [F; 4],
    pub p_iv: [F; 4],
    /// `skillTornadeSmokePFF1..3[level - 1]`: the smoke generator's force
    /// fields.
    pub smoke_ff: [[FfParam; 3]; 4],
}

impl RingTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> RingTables {
        let t = piney_data::tables::effect::of(volume);
        let b4 = |r: &[f32; 4]| r.map(f32::to_bits);
        let smoke = [t.tornado_smoke1(), t.tornado_smoke2(), t.tornado_smoke3()];
        let smoke_va = [t.tornado_smoke1_va(), t.tornado_smoke2_va(), t.tornado_smoke3_va()];
        let row = |l: usize| t.generators()[192 + l];
        RingTables {
            a89: std::array::from_fn(|k| b4(&t.ring89()[k])),
            t90: std::array::from_fn(|k| t.ring90()[k].map(|r| b4(&r))),
            turn90: first(t.turn90()),
            t91: std::array::from_fn(|k| t.ring91()[k].map(|r| b4(&r))),
            turn91: first(t.turn91()),
            pff2_04: std::array::from_fn(|l| smoke[1][l].rotate),
            pff1_18: std::array::from_fn(|l| smoke[0][l].force[2].to_bits()),
            g_radius: std::array::from_fn(|l| row(l).g_radius.to_bits()),
            p_iv: std::array::from_fn(|l| row(l).p_iv.to_bits()),
            smoke_ff: std::array::from_fn(|l| std::array::from_fn(|k| FfParam::row(smoke[k], smoke_va[k], l))),
        }
    }
}

/// `effMagicAttackSign(tp, atr)` (main 0x001d1090): false for an element
/// with no sign.
pub fn eff_magic_attack_sign(cx: &mut Cx, tp: CharRef, atr: i32) -> bool {
    let (p1, p2) = match atr {
        attr::SOIL => (136, 140),
        attr::WATER => (130, 137),
        attr::FIRE => (131, 139),
        attr::WIND => (133, 138),
        attr::THUNDER => (132, 140),
        attr::DARK => (134, 141),
        _ => return false,
    };
    pfx::start(cx, p1, |g| {
        g.offset = [0, 0, 0x41a0_0000, 0];
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::CharPos(tp));
    });
    pfx::start(cx, p2, |g| {
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::CharPos(tp));
    });
    true
}

/// `effSkillTornadeSmoke(tpos, offset, atr, level)` (main 0x001d48b0).
pub fn eff_skill_tornade_smoke(cx: &mut Cx, tpos: V4, offset: V4, atr: i32, _level: i32) {
    let c = match atr & 0xfc {
        attr::THUNDER => 11,
        attr::WIND => 112,
        attr::FIRE => 8,
        attr::WATER => 110,
        attr::SOIL => 114,
        _ => 1,
    };
    let player = cx.host.player_pos();
    let bounds = cx.host.bounds();
    let tp = space::w2p(tpos, player, bounds);
    let mut p = space::p2w(ee::vadd(tp, offset), player, bounds);
    p[2] = ee::add(p[2], 0x4220_0000);
    pfx::start(cx, SMOKE_ROW, |g| {
        g.pos = p;
        g.p_tex_mod = c;
    });
}

/// The first free slot, `InitEffect(id, 0)`, as the ring spawners find
/// it.
#[allow(clippy::too_many_arguments)]
fn ring(
    ctrl: &mut EffectCtrl,
    cx: &Cx,
    id: i16,
    tpos: V4,
    offset: Option<V4>,
    level: i32,
    flags: i32,
    clut: (u16, u16),
) -> Option<usize> {
    let i = ctrl.new_effect(cx, id)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 60;
    e.param = level - 1;
    if id != RING_A {
        e.flags = flags;
    }
    e.temp[0] = 1;
    if let Some(o) = offset {
        e.pos = tpos;
        e.pos_t = tpos;
        e.offset = o;
    } else {
        e.pos_t = tpos;
        e.pos = tpos;
    }
    if clut.0 != 0 {
        // Duplicate(0x3800), ChangeClut(ccParticleAdrs(new),
        // ccParticleAdrs(old)).
        let d = &cx.spells.data;
        e.clut_swap = d
            .particle_adrs(cx.assets, usize::from(clut.1))
            .zip(d.particle_adrs(cx.assets, usize::from(clut.0)))
            .map(|(from, to)| (from.object, to.object));
    }
    Some(i)
}

/// `effSkillTornadeRingsPos(tpos, atr, level)` (main 0x001d3d80): the last
/// ring made.
pub fn eff_skill_tornade_rings_pos(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    tpos: V4,
    atr: i32,
    level: i32,
) -> Option<usize> {
    rings_pos(ctrl, cx, tpos, None, atr, level)
}

/// `effSkillTornadeRingsPos(tpos, offset, atr, level)` (main 0x001d4350),
/// the tornado element's: the rings at `ccTransPosFW2LW(tpos)` moved by
/// `offset`, no tornado generator, the thunder or objects with the offset.
pub fn eff_skill_tornade_rings_pos_offset(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    tpos: V4,
    offset: V4,
    atr: i32,
    level: i32,
) -> Option<usize> {
    let p = space::fw2lw(tpos, cx.host.player_pos(), cx.host.bounds());
    rings_pos(ctrl, cx, p, Some(offset), atr, level)
}

fn rings_pos(ctrl: &mut EffectCtrl, cx: &mut Cx, tpos: V4, offset: Option<V4>, atr: i32, level: i32) -> Option<usize> {
    // The CLUTs (particleCcsAdrs indices) of rings 89, 90, 91 and the
    // tornado generator's pTexMod, by element.
    let (c89, c90, c91, tex) = match atr & 0xfc {
        attr::SOIL => (181, 223, 216, 114),
        attr::WATER => (0, 0, 0, 110),
        attr::FIRE => (176, 218, 211, 151),
        attr::WIND => (178, 220, 213, 153),
        attr::THUNDER => (177, 219, 212, 152),
        // Registers the callers never leave unset.
        _ => (0, 0, 0, 0),
    };
    let mut last = ring(ctrl, cx, RING_A, tpos, offset, level, 0, (c89, 175));
    for i in 0..2 {
        last = ring(ctrl, cx, RING_B, tpos, offset, level, i, (c90, 217));
    }
    for i in 0..4 {
        last = ring(ctrl, cx, RING_C, tpos, offset, level, i, (c91, 210));
    }
    if level >= 2 {
        if offset.is_none() {
            let f = cx.spells.data.rings.smoke_ff[(level - 1).clamp(0, 3) as usize];
            let ff = [Some(f[0]), Some(f[1]), Some(f[2]), None];
            pfx::start_ff(cx, (level + 191) as usize, ff, |g| {
                g.pos = tpos;
                g.p_tex_mod = tex;
            });
        }
        if atr & 0x40 != 0 {
            thunder::eff_skill_tornade_thunder_pos(ctrl, cx, tpos, offset, level);
            return last;
        }
        let t = &cx.spells.data.rings;
        let l = (level - 1).clamp(0, 3) as usize;
        let p: V4 = [
            ee::deg2rad((t.pff2_04[l] << 1) as i16),
            ee::mul(0x3e80_0000, t.p_iv[l]),
            ee::mul(0x3f40_0000, t.pff1_18[l]),
            t.g_radius[l],
        ];
        eff_skill_tornade_object_pos(ctrl, cx, tpos, offset, p, atr, level);
    }
    last
}

/// Effects 89, 90 and 91's case of the first switch (main 0x001c5198,
/// 0x001c5320, 0x001c5568).
pub fn ring_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    const TIME: i32 = 30;
    let t = &cx.spells.data.rings;
    let e = &mut ctrl.effects[i];
    let base = if e.temp[0] != 0 { e.pos_t } else { e.pos };
    e.pos = ee::vadd(base, e.offset);
    let p = (e.param & 3) as usize;
    let f = (e.flags & 3) as usize;
    let cnt = ee::from_int(i32::from(e.cnt));
    let lerp = |a: F, b: F| ee::add(a, ee::mul(cnt, ee::div(ee::sub(b, a), ee::from_int(TIME))));
    let (a, b, c, d) = match e.id {
        RING_A => {
            e.pos[3] = ONE;
            let turn = i32::from(ee::rad2deg(e.rot[2])) + 16384 / TIME;
            e.rot[2] = ee::deg2rad(turn as i16);
            (t.a89[0][p], t.a89[1][p], t.a89[2][p], t.a89[3][p])
        }
        _ => {
            let n = i32::from(e.cnt).min(TIME);
            let (rise, turn, tab) = if e.id == RING_B {
                let f = f & 1;
                (t.t90[4][f][p], t.turn90[f], [t.t90[0][f][p], t.t90[1][f][p], t.t90[2][f][p], t.t90[3][f][p]])
            } else {
                (t.t91[4][f][p], t.turn91[f], [t.t91[0][f][p], t.t91[1][f][p], t.t91[2][f][p], t.t91[3][f][p]])
            };
            e.pos[2] = ee::add(e.pos[2], ee::div(ee::mul(ee::from_int(n), rise), ee::from_int(TIME)));
            e.pos[3] = ONE;
            let turn = i32::from(ee::rad2deg(e.rot[2])) + i32::from(turn) / TIME;
            e.rot[2] = ee::deg2rad(turn as i16);
            (tab[0], tab[1], tab[2], tab[3])
        }
    };
    let s = lerp(a, b);
    e.scale[0] = s;
    e.scale[1] = s;
    e.scale[2] = lerp(c, d);
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 25, life);
    Next::Draw
}

/// The flying objects' controller (no object).
pub const OBJECT_POS: i16 = -16;
/// The flying objects: soil 92-95 (CMP_x102a-d), water 96-99
/// (CMP_x202a-d), fire 100 (EFF_x008), wind 101 (EFF_x402).
pub const OBJ_FIRST: i16 = 92;
pub const OBJ_LAST: i16 = 101;

/// `effSkillTornadeObjectPos(tpos, v, atr, level)` (main 0x001d5560; with
/// an offset 0x001d5680): the controller -16, life 30, `speed` the
/// tornado's (the objects' turn and rise, in w their spread).
pub fn eff_skill_tornade_object_pos(
    ctrl: &mut EffectCtrl,
    cx: &Cx,
    tpos: V4,
    offset: Option<V4>,
    v: V4,
    atr: i32,
    level: i32,
) -> Option<usize> {
    let i = ctrl.new_effect(cx, OBJECT_POS)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 30;
    e.level = ((level << 28) >> 28) as i8;
    e.param = atr;
    e.speed = v;
    e.pos_t = tpos;
    if let Some(o) = offset {
        e.offset = o;
    }
    Some(i)
}

/// Effect -16's case of the second chain (main 0x001cacf8): `velocity` on
/// by 8, 14, 20 or 26 (by level) over the life; an object for each whole
/// unit it passes.
pub fn object_pos_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let before = thunder::fptosi(e.velocity);
    let k = match e.level {
        1 => 0x4100_0000,
        2 => 0x4160_0000,
        3 => 0x41a0_0000,
        4 => 0x41d0_0000,
        _ => 0,
    };
    if k != 0 {
        e.velocity = ee::add(e.velocity, ee::div(k, ee::from_int(i32::from(e.life_time))));
    }
    let mut n = thunder::fptosi(e.velocity).wrapping_sub(before);
    let (pos_t, offset, speed, param) = (e.pos_t, e.offset, e.speed, e.param);
    while n != 0 {
        eff_skill_tornade_obj_pos(ctrl, cx, pos_t, offset, speed, param);
        n = n.wrapping_sub(1);
    }
}

/// `effSkillTornadeObjPos(tpos, offset, v, atr)` (main 0x001d57b0): one
/// flying object, life 35, turning out from the tornado.
pub fn eff_skill_tornade_obj_pos(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    tpos: V4,
    offset: V4,
    v: V4,
    atr: i32,
) -> Option<usize> {
    const TENTH: F = 0x3dcc_cccd;
    let rn = cx.host.rand() >> 3;
    let p1 = rn & 3;
    let a = atr & 0xfc;
    let size = || ee::from_int((rn >> 2) % 7 + 3);
    let (en, s) = match a {
        attr::WIND => (101, ONE),
        attr::FIRE => (100, ee::mul(TENTH, ee::mul(0x4080_0000, size()))),
        attr::WATER => (96 + p1 as i16, ee::mul(TENTH, size())),
        attr::SOIL => (92 + p1 as i16, ee::mul(TENTH, ee::mul(0x3f00_0000, size()))),
        // Registers the callers never leave unset.
        _ => (0, 0),
    };
    let i = ctrl.new_effect(cx, en)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 35;
    e.speed = v;
    e.pos_t = tpos;
    e.velocity = ee::mul(TENTH, ee::mul(v[3], ee::from_int((rn & 0xff0) % 7 + 1)));
    e.rot_speed[2] = (rn & 0xf000) as u16;
    e.scale = ee::vscale(ONE_VECTOR, s);
    if atr & 0x30 != 0 {
        e.tex_anm_pat = p1 as u16;
        let r = cx.host.rand();
        if let Obj::Eff(eff) = &mut ctrl.effects[i].obj {
            eff.rotate = ee::deg2rad(((r >> 3) & 0xf100) as i16);
        }
    } else {
        e.rot_speed[0] = ((rn >> 4) & 0xf000) as u16;
        e.rot_speed[1] = ((rn >> 8) & 0xf000) as u16;
    }
    // Water and soil: SetFogSw(clump, 1).
    ctrl.effects[i].offset = offset;
    Some(i)
}

/// Effects 92-101's case of the first switch (main 0x001c580c; 100 and 101
/// at 0x001c5800 first take the sprite's scale from `scale.x`): outward by
/// `speed.y` a frame, round by `speed.x * speed.w / velocity`, up by
/// `speed.z`, facing along; FadeInOut(5, 10, lifeTime).
pub fn obj_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    if e.id >= 100
        && let Obj::Eff(eff) = &mut e.obj
    {
        eff.scale_x = e.scale[0];
        eff.scale_y = e.scale[0];
    }
    e.velocity = ee::add(e.velocity, e.speed[1]);
    let turn = ee::rad2deg(ee::div(ee::mul(e.speed[0], e.speed[3]), e.velocity));
    e.rot_speed[2] = e.rot_speed[2].wrapping_add(turn as u16);
    let a = ee::deg2rad(e.rot_speed[2] as i16);
    e.offset[0] = ee::mul(e.velocity, ee::cosf(a));
    e.offset[1] = ee::mul(e.velocity, ee::sinf(a));
    e.offset[2] = ee::add(e.offset[2], e.speed[2]);
    e.pos = ee::vadd(e.pos_t, e.offset);
    e.rot[2] = ee::neg(a);
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 10, life);
    Next::Draw
}

/// `ccTornadeElement` (gcmn effect2.cpp, 0x210 bytes): the tornados of
/// levels 3 and 4, three (four) at once round the target.
#[derive(Clone, Debug, PartialEq)]
pub struct TornadeElement {
    pub base: Base,
    /// +0x190 `m_skillPtr` (its own; `ccEffectElement`'s stays clear).
    pub skill: u32,
    /// +0x1a0 `m_offsets[4]`: the tornados' places round the target (300
    /// out, a third of a turn apart; the fourth (0, 0, 0, 1)).
    pub offsets: [V4; 4],
    /// +0x1e0 `m_index`: the next tornado's rings.
    pub index: i32,
    /// +0x1f0 `m_tPos`: the target's position, from the skill while it
    /// runs.
    pub t_pos: V4,
    /// +0x200 `m_skillType`: the low half of the row's `type`.
    pub skill_type: i16,
}

/// The tornado element's timing (gcmn 0x005ed760.. for level 3,
/// 0x005ed790.. for 4): the smoke's counts, the rings' counts, the rings'
/// sounds.
#[derive(Clone, Debug, PartialEq)]
pub struct TornadeTables {
    pub start_1: [[i32; 4]; 2],
    pub start_2: [[i32; 4]; 2],
    pub sndcode: [[i32; 4]; 2],
}

impl TornadeTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> TornadeTables {
        let t = piney_data::tables::effect::of(volume);
        TornadeTables {
            start_1: [first(t.tornade_start1()), first(t.tornade_start1b())],
            start_2: [first(t.tornade_start2()), first(t.tornade_start2b())],
            sndcode: [first(t.tornade_sndcode()), first(t.tornade_sndcodeb())],
        }
    }
}

impl TornadeElement {
    /// `new ccTornadeElement(skillPtr, level)` (gcmn 0x004ff0d0).
    pub fn new(cx: &Cx, skill: &crate::spell::Spell, level: i32) -> TornadeElement {
        let base = Base { level, target: skill.target, ..Base::default() };
        let radius = 0x4396_0000;
        let mut r: F = 0;
        let mut offsets = [VF0; 4];
        for o in offsets.iter_mut().take(3) {
            o[0] = ee::mul(radius, ee::cosf(r));
            o[1] = ee::mul(radius, ee::sinf(r));
            r = ee::add(r, 0x4006_0a92);
            if !ee::le(r, ee::PI) {
                r = ee::sub(r, 0x40c9_0fdb);
            }
        }
        let _ = cx;
        TornadeElement {
            base,
            skill: skill.key,
            offsets,
            index: 0,
            t_pos: skill.t_pos,
            skill_type: skill.skill_type as i16,
        }
    }

    /// `ccTornadeElement::Main` (gcmn 0x004ff3f0): the target's position
    /// while the skill runs (deleted once its caster is off the lists),
    /// then the level's step.
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        if cx.spells.entry_check(self.skill) {
            let s = cx.spells.get(self.skill).cloned().unwrap_or_else(|| crate::spell::Spell::new(0, 0));
            self.t_pos = s.t_pos;
            if !s.creator.is_some_and(|c| cx.host.check_target(c)) {
                self.base.del_flag = 1;
                return;
            }
        }
        self.level(ctrl, cx);
    }

    /// `_Level3` (gcmn 0x004ff4a0) and `_Level4` (0x004ff8b0): the smoke and
    /// rings at the counts of `START_1` and `START_2`, `ccSkillDamage` every 5
    /// frames, the shake at `START_2[0]` + 10, then `m_delFlag`. The counts are
    /// in docs/engine/effects.md ("TornadoSystem").
    fn level(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let l4 = self.base.level != 3;
        let t = &cx.spells.data.tornade;
        let k = usize::from(l4);
        let (start_1, start_2, sndcode) = (t.start_1[k], t.start_2[k], t.sndcode[k]);
        let blur = start_2[0] + 10;
        let tc1 = if l4 { start_2[3] } else { start_2[1] } + 5;
        let tc9 = tc1 + 40;
        let count = self.base.count;
        self.base.count = count.wrapping_add(1);
        let live = cx.spells.entry_check(self.skill);
        let skill = cx.spells.get(self.skill).cloned();
        let (creator, target) = match (&skill, live) {
            (Some(s), true) => (s.creator, s.target),
            _ => (None, None),
        };
        let t_pos = self.t_pos;
        let player = cx.host.player_pos();
        let bounds = cx.host.bounds();
        let n = if l4 { 4 } else { 3 };
        let index = start_1[..n].iter().position(|&c| c == count);
        let smoke = if l4 { index == Some(3) } else { index.is_some() };
        if let (true, Some(i)) = (smoke, index) {
            let (atr, level) = if l4 {
                (skill.as_ref().map_or(0, |s| data_type(cx, s)) & 0xfc, 3)
            } else {
                (i32::from(self.skill_type) & 0xfc, self.base.level)
            };
            eff_skill_tornade_smoke(cx, t_pos, self.offsets[i], atr, level);
            let p = ee::vadd(t_pos, self.offsets[(self.index & 3) as usize]);
            ring::eff_summon_ring_element(cx, p, self.base.anim.dirc, 195);
        }
        let index = start_2[..n].iter().position(|&c| c == count);
        if let Some(i) = index {
            let (snd, level) = if l4 && i == 3 {
                (sndcode[3], 4)
            } else if l4 {
                (sndcode[1], 2)
            } else {
                (0, 2)
            };
            let snd = if l4 { snd } else { sndcode[((self.base.level - 1) & 3) as usize] };
            let off = self.offsets[(self.index & 3) as usize];
            let p = space::p2w(ee::vadd(space::w2p(t_pos, player, bounds), off), player, bounds);
            cx.raise(Event::Sound3d { se: snd, pos: p });
            let atr = skill.as_ref().map_or(0, |s| data_type(cx, s)) & 0xfc;
            self.index = self.index.wrapping_add(1);
            eff_skill_tornade_rings_pos_offset(ctrl, cx, t_pos, off, atr, level);
        }
        if count >= tc1 && count <= tc9 && (count - tc1) % 5 == 0 {
            if live && let Some(s) = &skill {
                match target {
                    Some(tg) if cx.host.check_target(tg) => {
                        cx.raise(Event::SkillDamage { spell: s.key, attacker: creator, target: Some(tg), sid: s.id });
                    }
                    _ => cx.raise(Event::SkillDamageAt { attacker: creator, pos: s.t_pos, ttype: s.t_type, sid: s.id }),
                }
            }
        } else if count > tc9 {
            self.base.del_flag = 1;
        }
        if count == blur && check_camera_shake_range(cx, t_pos) {
            noise(cx, 20);
            camera_shake(cx, if l4 { 0 } else { 2 }, 2, 20, 2);
        }
    }
}

/// `skill->param->type` (read even once the skill is gone).
fn data_type(cx: &Cx, s: &crate::spell::Spell) -> i32 {
    cx.spells.data.skill_type(s.id)
}

/// `ccSkillTornadeElementsGenerate(skill, level)` (gcmn 0x00500bd0): the
/// element in the first empty slot (None: deleted, the generator answers
/// 0).
pub fn tornade_elements_generate(cx: &mut Cx, k: usize, level: i32) -> Option<usize> {
    let s = cx.spells.runs[k].clone();
    let e = TornadeElement::new(cx, &s, level);
    cx.spells.elements.add(Element::Tornade(Box::new(e)))
}
