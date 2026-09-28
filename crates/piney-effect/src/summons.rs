//! The summoning spells: `ccSkill::SummonsSystem` (gcmn 0x00579000; the damage
//! at 52 and the end at 62, 289 at 150 and 190, 290 at 115 and 125), the
//! summoning ring `effSummonsRing` (effect 56) with its shock waves (64, 65)
//! and fragments (66-78), the summoned creature `effSummonsElement` (57-63,
//! 164, 165), 289's drills (168, `effTCDrillMissile`), and the lock-on of
//! skill 290 (`effSBLockon`: the controller -26 and its mark 166, count 167,
//! range rings 172 and flare 171). Levels 3 and up are `crate::summoned`'s. The
//! cases are in docs/engine/effects.md ("SummonsSystem").

use piney_data::volume::Volume;

use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR, Obj};
use crate::fall;
use crate::fall::eff_skill_break_se;
use crate::hit::EFF_SBL_LAYER;
use crate::spell::{Spell, attr, camera_shake, check_camera_shake_range, noise};
use crate::{CharRef, Cx, Event, debris, pfx, summoned, tornado, upheaval, vu};

pub const RING: i16 = 56;
pub const CREATURE_FIRST: i16 = 57;
pub const CREATURE_LAST: i16 = 63;
pub const CREATURE_EXTRA: [i16; 2] = [164, 165];
pub const WAVE: i16 = 64;
pub const WAVE_OUTER: i16 = 65;
pub const FRAGMENT_FIRST: i16 = 66;
pub const FRAGMENT_LAST: i16 = 78;
pub const LOCKON: i16 = -26;
pub const LOCKON_MARK: i16 = 166;
pub const STACK_NUM: i16 = 167;
pub const HUGE_FLARE_RING: i16 = 171;
pub const STACK_RANGE_RING: i16 = 172;

/// The shock wave 64's tables (main 0x0033ff10-0x0033ff40): by wave, its
/// width from and towards, its height from and towards.
#[derive(Clone, Debug, PartialEq)]
pub struct SummonsTables {
    pub wave: [[F; 3]; 4],
}

impl SummonsTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> SummonsTables {
        let t = piney_data::tables::effect::of(volume);
        SummonsTables { wave: std::array::from_fn(|k| t.summons_wave()[k].map(f32::to_bits)) }
    }
}

/// The end's release: the caster of its own skill, when it is on the
/// lists.
fn release_listed(cx: &mut Cx, s: &Spell) {
    if let Some(c) = s.creator
        && cx.host.check_target(c)
        && (s.stype == 0 || s.stype == 2)
    {
        cx.raise(Event::SkillRelease { ch: c });
    }
}

fn damage2(cx: &mut Cx, s: &Spell) {
    cx.raise(Event::SkillDamage2 {
        spell: s.key,
        attacker: s.creator,
        target: s.target,
        pos: s.t_pos,
        ttype: s.t_type,
        sid: s.id,
    });
}

/// `ccSkill::SummonsSystem` (gcmn 0x00579000).
pub fn summons_system(ctrl: &mut EffectCtrl, cx: &mut Cx, k: usize) {
    let attr = cx.spells.runs[k].attr(&cx.spells.data);
    let s = cx.spells.runs[k].clone();
    let (hit, end) = match s.id {
        289 => (150, 190),
        290 => (115, 125),
        _ => (52, 62),
    };
    let listed = |cx: &Cx, c: Option<CharRef>| c.is_some_and(|c| cx.host.check_target(c));
    if s.count == 0 {
        if listed(cx, s.target) {
            cx.raise(Event::Sound3d { se: 62, pos: s.t_pos });
            tornado::eff_magic_attack_sign(cx, s.target.unwrap_or(0), attr);
        } else {
            cx.spells.runs[k].status = 1;
            s.release(cx);
        }
        let r = &mut cx.spells.runs[k];
        r.level = match attr {
            attr::SOIL => r.id - 204,
            attr::WATER => r.id - 220,
            attr::FIRE => r.id - 236,
            attr::WIND => r.id - 252,
            attr::THUNDER => r.id - 268,
            attr::DARK => r.id - 284,
            _ if r.id >= 291 => r.id - 290,
            _ => 1,
        };
    } else if s.count == 30 {
        cx.raise(Event::Sound3d { se: 62, pos: s.t_pos });
        let mut ty = 0;
        if s.id == 289 {
            ty = 1;
        }
        if s.id == 290 {
            ty = 2;
            let mut p = s.t_pos;
            if let Some(t) = s.target
                && cx.host.check_target(t)
            {
                p[2] = ee::add(p[2], ee::mul(0x3f00_0000, cx.host.char_height(t)));
            }
            eff_sb_lockon(ctrl, cx, p);
        }
        eff_summons_ring(ctrl, cx, s.c_pos, attr, s.level, ty);
        eff_summons_element(ctrl, cx, s.c_pos, s.c_height, s.c_dirc[2], attr, ty);
        if s.id == 289 {
            let n = if listed(cx, s.target) {
                match cx.host.object_size(s.target.unwrap_or(0)) {
                    1 => 2,
                    3 => 1,
                    _ => 0,
                }
            } else {
                0
            };
            let mut cp = s.c_pos;
            eff_tc_drill_missile(ctrl, cx, &mut cp, s.t_pos, n);
            cx.spells.runs[k].c_pos = cp;
        }
        if s.level >= 3 && s.id != 289 && s.id != 290 {
            cx.spells.runs[k].elm = summoned::summons_element_generate(cx, k, attr, s.level);
        }
    }
    let s = cx.spells.runs[k].clone();
    if s.id == 289 || s.id == 290 {
        if s.count == hit {
            damage2(cx, &s);
        }
        if s.count == end {
            let r = &mut cx.spells.runs[k];
            r.hold = false;
            r.status = 1;
            release_listed(cx, &s);
        }
        return;
    }
    if s.count == hit {
        cx.raise(Event::Sound3d { se: 35, pos: s.c_pos });
        match attr {
            attr::SOIL => cx.raise(Event::Sound3d { se: 56, pos: s.c_pos }),
            attr::WATER => cx.raise(Event::Sound3dNote { se: 66, pos: s.c_pos, note: 48 }),
            attr::DARK => cx.raise(Event::Sound3d { se: 69, pos: s.c_pos }),
            _ => {}
        }
    }
    if s.level >= 3 {
        if let Some(e) = s.elm
            && cx.spells.elements.get(e).is_some_and(|e| e.deleted())
        {
            cx.spells.runs[k].hold = false;
            release_listed(cx, &s);
            cx.spells.runs[k].status = 1;
        }
        return;
    }
    if s.count == hit {
        damage2(cx, &s);
        if check_camera_shake_range(cx, s.c_pos) {
            if s.level == 1 {
                camera_shake(cx, 0, 2, 20, 0);
            } else {
                camera_shake(cx, 2, 2, 30, 2);
            }
            noise(cx, 20);
        }
    }
    if s.count == end {
        let r = &mut cx.spells.runs[k];
        r.hold = false;
        r.status = 1;
        release_listed(cx, &s);
    }
}

/// The CLUT swap `ccClump::Duplicate`, `ChangeClut(ccParticleAdrs(new),
/// ccParticleAdrs(old))` gives an effect's clump.
fn clump_clut(cx: &Cx, new: usize, old: usize) -> Option<(u32, u32)> {
    let d = &cx.spells.data;
    d.particle_adrs(cx.assets, old).zip(d.particle_adrs(cx.assets, new)).map(|(f, t)| (f.object, t.object))
}

/// `effSummonsRing(p, atr, level, type)` (main 0x001d1aa0): the ring 56,
/// life 40, re-coloured by element (161 made 167 soil, 162 fire, 164 wind,
/// 163 thunder, 165 dark, 166 none), with generator 150-156 at `p`.
pub fn eff_summons_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, atr: i32, level: i32, ty: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, RING)?;
    let (row, clut) = match atr {
        attr::SOIL => (0, 167),
        attr::WATER => (1, 0),
        attr::FIRE => (2, 162),
        attr::WIND => (3, 164),
        attr::THUNDER => (4, 163),
        attr::DARK => (5, 165),
        _ => (6, 166),
    };
    let swap = if clut != 0 { clump_clut(cx, clut, 161) } else { None };
    let e = &mut ctrl.effects[i];
    e.life_time = 40;
    e.param = atr;
    e.level = (((level & 0xf) << 4) as i8) >> 4;
    e.pos = p;
    e.flags = ty;
    if clut != 0 {
        e.clut_swap = swap;
    }
    pfx::start(cx, 150 + row, |g| g.pos = p);
    Some(i)
}

/// The ring 56's case of the first switch (main 0x001c468c).
pub fn ring_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    e.scale[1] = 0x4026_6666;
    e.scale[0] = 0x4026_6666;
    let d = ee::div(ee::sub(0x400c_cccd, 0x3f00_0000), ee::from_int(i32::from(e.life_time)));
    e.scale[2] = ee::add(0x3f00_0000, ee::mul(ee::from_int(i32::from(e.cnt)), d));
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(819));
    let life = i32::from(e.life_time);
    e.fade_in_out(3, 5, life);
    Next::Draw
}

/// The ring 56's case of the second chain (main 0x001ca650).
pub fn ring_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &ctrl.effects[i];
    let (pos, param, level, flags, cnt) = (e.pos, e.param, i32::from(e.level), e.flags, e.cnt);
    if cnt == 12 {
        eff_summons_shock_wave(ctrl, cx, pos, param, level);
    }
    if flags == 0 && cnt == 22 {
        let mut r = VF0;
        r[0] = ee::deg2rad(-16384);
        eff_smmons_fragment(ctrl, cx, pos, r, 0x4220_0000, level * 2 + 6, param);
    }
}

/// `effSummonsElement(p, height, dirc, atr, type)` (main 0x001d1d30): the
/// creature (soil 57, water 58, fire 59, wind 60, thunder 61, dark 62; no
/// element 63, or 164 for type 1, 165 for type 2), life 65, from half the
/// caster's height to 50 above that, facing `dirc`.
pub fn eff_summons_element(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    p: V4,
    height: F,
    dirc: F,
    atr: i32,
    ty: i32,
) -> Option<usize> {
    let id = match atr {
        attr::SOIL => 57,
        attr::WATER => 58,
        attr::FIRE => 59,
        attr::WIND => 60,
        attr::THUNDER => 61,
        attr::DARK => 62,
        _ => match ty {
            1 => 164,
            2 => 165,
            _ => 63,
        },
    };
    let i = ctrl.new_effect(cx, id)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 65;
    e.pos = p;
    let h = ee::mul(0x3f00_0000, height);
    // z and w are the stack's leftovers in the game; nothing reads them.
    e.offset = [ee::add(h, p[2]), ee::add(0x4248_0000, h), 0, 0];
    e.rot_speed[2] = ee::rad2deg(dirc) as u16;
    Some(i)
}

/// The creatures' case of the first switch (main 0x001c4738): turning in
/// over the first 20 frames, rising over the first 10; FadeInOut(10, 15).
pub fn creature_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let cnt = i32::from(e.cnt);
    let a = if cnt < 20 { (i32::from(e.rot_speed[2]) + ((20 - cnt) << 15) / 20) as i16 } else { e.rot_speed[2] as i16 };
    e.rot[2] = ee::deg2rad(a);
    let life = i32::from(e.life_time);
    e.fade_in_out(10, 15, life);
    let cnt = i32::from(e.cnt);
    e.pos[2] = if cnt < 10 {
        ee::add(e.offset[0], ee::div(ee::mul(e.offset[1], ee::from_int(cnt)), ee::from_int(10)))
    } else {
        ee::add(e.offset[0], e.offset[1])
    };
    Next::Draw
}

/// `effSummonsShockWave(p, atr, level)` (main 0x001d1f10): three waves 64
/// (life 20, `param` their number, 217 made 223 soil, 218 fire, 220 wind,
/// 219 thunder, 221 dark, 222 none) and one 65 (life 40, 210 made 216 soil,
/// 211 fire, 213 wind, 212 thunder, 214 dark, 215 none). The last.
pub fn eff_summons_shock_wave(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, atr: i32, level: i32) -> Option<usize> {
    let pick = |base: usize| -> usize {
        match atr {
            attr::SOIL => base + 6,
            attr::WATER => 0,
            attr::FIRE => base + 1,
            attr::WIND => base + 3,
            attr::THUNDER => base + 2,
            attr::DARK => base + 4,
            _ => base + 5,
        }
    };
    for n in 0..3 {
        let Some(i) = ctrl.new_effect(cx, WAVE) else { continue };
        let clut = pick(217);
        let swap = if clut != 0 { clump_clut(cx, clut, 217) } else { None };
        let e = &mut ctrl.effects[i];
        e.life_time = 20;
        e.param = n;
        e.level = (((level & 0xf) << 4) as i8) >> 4;
        e.pos = p;
        if clut != 0 {
            e.clut_swap = swap;
        }
    }
    let i = ctrl.new_effect(cx, WAVE_OUTER)?;
    let clut = pick(210);
    let swap = if clut != 0 { clump_clut(cx, clut, 210) } else { None };
    let e = &mut ctrl.effects[i];
    e.life_time = 40;
    e.pos = p;
    if clut != 0 {
        e.clut_swap = swap;
    }
    Some(i)
}

/// `x + level (y - x) / 5`: where a wave's growth stops.
fn towards(x: F, y: F, level: F) -> F {
    ee::add(x, ee::div(ee::mul(level, ee::sub(y, x)), 0x40a0_0000))
}

/// `from + cnt (to - from) / life`, held at `stop`.
fn grow(from: F, to: F, life: i16, cnt: i16, stop: F) -> F {
    let v =
        ee::add(from, ee::mul(ee::from_int(i32::from(cnt)), ee::div(ee::sub(to, from), ee::from_int(i32::from(life)))));
    if ee::le(v, stop) { v } else { stop }
}

/// The waves 64's case of the first switch (main 0x001c4818): widening and
/// rising with the tables, by level; turning (the second the other way);
/// FadeInOut(5, 10).
pub fn wave_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let t = &cx.spells.data.summons.wave;
    let e = &mut ctrl.effects[i];
    let n = (e.param as usize).min(2);
    let level = ee::from_int(i32::from(e.level));
    let wide = towards(t[0][n], t[1][n], level);
    let high = towards(t[2][n], t[3][n], level);
    e.scale[0] = grow(t[0][n], t[1][n], e.life_time, e.cnt, wide);
    e.scale[1] = e.scale[0];
    e.scale[2] = grow(t[2][n], t[3][n], e.life_time, e.cnt, high);
    let turn: i16 = if e.param == 1 { -819 } else { 819 };
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(turn));
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 10, life);
    Next::Draw
}

/// The outer wave 65's case of the first switch (main 0x001c4a28): as the
/// waves, 0.5 towards 2.2 wide and 0 towards 9 high (its level is never
/// set: it stays 0.5 wide and flat).
pub fn wave_outer_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let level = ee::from_int(i32::from(e.level));
    let wide = towards(0x3f00_0000, 0x400c_cccd, level);
    let high = towards(0, 0x4110_0000, level);
    e.scale[0] = grow(0x3f00_0000, 0x400c_cccd, e.life_time, e.cnt, wide);
    e.scale[1] = e.scale[0];
    e.scale[2] = grow(0, 0x4110_0000, e.life_time, e.cnt, high);
    e.rot[2] = ee::deg2rad(ee::rad2deg(e.rot[2]).wrapping_add(819));
    let life = i32::from(e.life_time);
    e.fade_in_out(5, 10, life);
    Next::Draw
}

/// (0, -1, 0) turned by a random tilt up to 45 degrees off straight up,
/// then by `r`.
fn upward(cx: &mut Cx, r: V4) -> (i32, V4) {
    let rn = cx.host.rand() >> 3;
    let a = [0, ee::deg2rad(((rn << 8) & 0xfc00) as i16), ee::deg2rad((6144 - (rn & 0xfc0)) as i16), 0];
    let d = ee::apply(&vu::rot_zyx(&vu::UNIT, a), [0, 0xbf80_0000, 0, ONE]);
    (rn, ee::apply(&vu::rot_zyx(&vu::UNIT, r), d))
}

/// `(100 - (rn >> 8) % 50) / 100`.
fn share(rn: i32) -> F {
    let h = 0x42c8_0000;
    ee::div(ee::sub(h, ee::from_int((rn >> 8) % 50)), h)
}

/// `effSmmonsFragment(p, r, v, n, atr)` (main 0x001d22b0): n pieces thrown
/// up, life 45: rocks 66-69 (soil) and ice 70-73 (water) three tenths the
/// size, fire 74 (CLUT 154 made 155), a leaf 75 (its pattern count cut to
/// 0-3, turned at random), thunder 76, dark 77, none 78. The last.
#[allow(clippy::too_many_arguments)]
pub fn eff_smmons_fragment(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, n: i32, atr: i32) -> Option<usize> {
    let mut last = None;
    for _ in 0..n {
        let (rn, d) = upward(cx, r);
        let mut pats = 0;
        let id = match atr {
            attr::SOIL => 66 + ((cx.host.rand() >> 3) % 4) as i16,
            attr::WATER => 70 + ((cx.host.rand() >> 3) % 4) as i16,
            attr::FIRE => 74,
            attr::WIND => {
                pats = (cx.host.rand() >> 3) % 4;
                75
            }
            attr::THUNDER => 76,
            attr::DARK => 77,
            _ => 78,
        };
        last = ctrl.new_effect(cx, id);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.life_time = 45;
        e.pos = p;
        if atr == attr::FIRE {
            upheaval::eff_change_clut(cx, &mut ctrl.effects[k], 155, 154);
        }
        if atr == attr::WIND
            && let Obj::Eff(f) = &mut ctrl.effects[k].obj
        {
            f.pat_num = pats as u16;
        }
        let e = &mut ctrl.effects[k];
        e.speed = ee::vscale(d, ee::mul(v, share(rn)));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0x3c0) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0x3c0) as u16;
        if atr == attr::SOIL || atr == attr::WATER {
            e.scale = ee::vscale(ONE_VECTOR, 0x3e99_999a);
        }
        if atr == attr::WIND {
            let a = ee::deg2rad(((cx.host.rand() >> 3) & 0xf800) as i16);
            if let Obj::Eff(f) = &mut ctrl.effects[k].obj {
                f.rotate = a;
            }
        }
        // Water and soil: SetFogSw(clump, 1).
    }
    last
}

/// The fragments 66-78's case of the second chain (main 0x001ca6e8): the
/// rocks' landing and bounce, 0.75 a bounce.
pub fn fragment_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    debris::bounce_post(ctrl, cx, i, 0x3f40_0000, 0xbf40_0000);
}

/// `effSBLockon(p)` (main 0x001d8190): the controller -26, life 80.
pub fn eff_sb_lockon(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4) -> Option<usize> {
    let i = ctrl.new_effect(cx, LOCKON)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 80;
    e.pos = p;
    Some(i)
}

/// A sprite drawn on `effSBL` with no depth test (`SetRenderState(0, 0)`).
fn sbl_sprite(ctrl: &mut EffectCtrl, cx: &mut Cx, id: i16, life: i16, p: V4) -> Option<usize> {
    let i = ctrl.new_effect(cx, id)?;
    let e = &mut ctrl.effects[i];
    e.life_time = life;
    e.pos = p;
    if let Obj::Eff(f) = &mut e.obj {
        f.test &= !(1 << 16);
    }
    e.layer = Some(EFF_SBL_LAYER);
    Some(i)
}

/// `effSBLockonMark(p)` (main 0x001d8250): the mark 166, life 90.
pub fn eff_sb_lockon_mark(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4) -> Option<usize> {
    sbl_sprite(ctrl, cx, LOCKON_MARK, 90, p)
}

/// `effSBStackNum(p)` (main 0x001d8330): the count 167, life 60.
pub fn eff_sb_stack_num(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4) -> Option<usize> {
    sbl_sprite(ctrl, cx, STACK_NUM, 60, p)
}

/// `effStackRangeRing(p, n)` (main 0x001cd1e0): the range ring 172 growing
/// to 1 + (n - 1) / 2 (temp[0]; n 0 or above 8: 1), life 9 (20 for n 8).
pub fn eff_stack_range_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, n: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, STACK_RANGE_RING)?;
    let e = &mut ctrl.effects[i];
    e.pos = p;
    e.temp[0] = ONE;
    e.temp[1] = 0;
    e.param = n;
    let mut life = 9;
    if (1..=8).contains(&n) {
        e.temp[0] = [ONE, 0x3fc0_0000, 0x4000_0000, 0x4020_0000, 0x4040_0000, 0x4060_0000, 0x4080_0000, 0x4090_0000]
            [(n - 1) as usize];
        if n == 8 {
            life += 11;
        }
    }
    e.life_time = life;
    Some(i)
}

/// `effHugeFlareRing(p, n)` (main 0x001cd050): the flare ring 171, life 20;
/// for n 1-6 its CLUT 190 made 190 + n.
pub fn eff_huge_flare_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, n: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, HUGE_FLARE_RING)?;
    let swap = if (1..7).contains(&n) { clump_clut(cx, (190 + n) as usize, 190) } else { None };
    let e = &mut ctrl.effects[i];
    e.life_time = 20;
    e.pos = p;
    if (1..7).contains(&n) {
        e.clut_swap = swap;
    }
    Some(i)
}

/// The lock-on -26's case of the second chain (main 0x001cbd5c). Its
/// temp[0] names the count 167 (the slot + 1; the game keeps the pointer).
pub fn lockon_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    const COUNT: i16 = 30;
    const BLAST: i16 = COUNT + 50;
    let (pos, cnt) = (ctrl.effects[i].pos, ctrl.effects[i].cnt);
    if cnt == 0 {
        eff_sb_lockon_mark(ctrl, cx, pos);
        cx.raise(Event::Sound3d { se: 102, pos });
    }
    if cnt == COUNT {
        let n = eff_sb_stack_num(ctrl, cx, pos);
        ctrl.effects[i].temp[0] = n.map_or(0, |n| n as u32 + 1);
        cx.raise(Event::Sound3d { se: 103, pos });
        let f = ctrl.effects[i].flags;
        eff_stack_range_ring(ctrl, cx, pos, f + 1);
    }
    let cnt = ctrl.effects[i].cnt;
    if cnt > COUNT && (i32::from(cnt) - i32::from(COUNT) - 1) % 5 == 4 {
        if let Some(n) = (ctrl.effects[i].temp[0] as usize).checked_sub(1) {
            let e = &mut ctrl.effects[n];
            e.tex_anm_pat = e.tex_anm_pat.wrapping_add(1);
        }
        ctrl.effects[i].flags += 1;
        let f = ctrl.effects[i].flags;
        if f < 8 {
            cx.raise(Event::Sound3d { se: 103, pos });
            eff_stack_range_ring(ctrl, cx, pos, f + 1);
        }
    }
    if ctrl.effects[i].cnt == BLAST {
        eff_skill_break_se(cx, pos, attr::FIRE);
        noise(cx, 50);
        if check_camera_shake_range(cx, pos) {
            camera_shake(cx, 2, 1, 50, 2);
        }
        let mut r = VF0;
        r[0] = ee::deg2rad(-16384);
        debris::eff_smoke_rock(ctrl, cx, pos, r, 0x425c_0000, 12, 0);
        eff_huge_flare_ring(ctrl, cx, pos, 1);
        let mut q = pos;
        q[2] = ee::add(q[2], 0x41a0_0000);
        pfx::start(cx, 242, |g| g.pos = q);
        pfx::start(cx, 243, |g| g.pos = pos);
    }
}

/// The mark 166's case of the first switch (main 0x001c6830): shrinking
/// from 32/15 to 2 over 15 frames, then from 21 its pattern steps to the
/// last; FadeInOut(8, 10).
pub fn mark_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let cnt = i32::from(e.cnt);
    if let Obj::Eff(f) = &mut e.obj {
        if cnt < 15 {
            let s = ee::div(ee::mul(0x4000_0000, ee::mul(0x4180_0000, ee::from_int(15 - cnt))), ee::from_int(15));
            f.scale_y = s;
            f.scale_x = s;
        } else if cnt > 20 {
            f.scale_y = 0x4000_0000;
            f.scale_x = 0x4000_0000;
            e.tex_anm_pat = e.tex_anm_pat.wrapping_add(1);
            if e.tex_anm_pat >= f.pat_num {
                e.tex_anm_pat = (f.pat_num as i16).wrapping_sub(1) as u16;
            }
        }
    }
    let life = i32::from(e.life_time);
    e.fade_in_out(8, 10, life);
    Next::Draw
}

/// The count 167's case of the first switch (main 0x001c6904): size 2,
/// its pattern held at the last; FadeOut(10).
pub fn stack_num_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    if let Obj::Eff(f) = &mut e.obj {
        f.scale_y = 0x4000_0000;
        f.scale_x = 0x4000_0000;
        if e.tex_anm_pat >= f.pat_num {
            e.tex_anm_pat = (f.pat_num as i16).wrapping_sub(1) as u16;
        }
    }
    let life = i32::from(e.life_time);
    e.fade_out(10, life);
    Next::Draw
}

/// The flare ring 171's case of the first switch (main 0x001c4390): scale
/// 1 + cnt (2.9 - 0.03 cnt); FadeOut(10).
pub fn huge_flare_ring_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let c = ee::from_int(i32::from(e.cnt));
    let s = ee::add(ONE, ee::mul(c, ee::sub(0x4039_999a, ee::mul(0x3cf5_c28f, c))));
    e.scale = ee::vscale(ONE_VECTOR, s);
    let life = i32::from(e.life_time);
    e.fade_out(10, life);
    Next::Draw
}

/// The range ring 172's case of the first switch (main 0x001c4408): its
/// size (temp[1]) three tenths of the way to temp[0] a frame; FadeOut(6).
pub fn range_ring_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let s = ee::add(e.temp[1], ee::mul(ee::sub(e.temp[0], e.temp[1]), 0x3e99_999a));
    e.temp[1] = s;
    e.scale = ee::vscale(ONE_VECTOR, s);
    let life = i32::from(e.life_time);
    e.fade_out(6, life);
    Next::Draw
}

/// The drill missile.
pub const DRILL: i16 = 168;

/// A drill missile's own: +0xa0 and +0xa4 (`temp[0]`, `temp[1]`) its two
/// `ccAnm`s, the nut (ANM_x703nut0) and the drill (ANM_x703atc0), drawn by
/// `texAnmPat`; and the object its row gave it, kept while one of them is
/// drawn in its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Drill {
    pub anm: [Option<Obj>; 2],
    pub own: Option<Obj>,
}

/// A `ccAnm` of X703.CCS as an effect draws it.
fn x703_anm(cx: &Cx, name: &str) -> Option<Obj> {
    let obj = cx.assets.find("x703", name)?;
    let play = piney_world::pose::Play::new(&cx.assets.files[obj.file], name)?;
    Some(Obj::Anm { obj, play, matrix: vu::UNIT })
}

/// `ccLandHitCheck(pos, 0x20000001)`.
fn land(cx: &mut Cx, pos: V4) -> F {
    cx.host.land_hit_check(pos, 0x2000_0001)
}

/// `effTCDrillMissile(cPos, tPos, n)` (main 0x001d8410): five drills 168,
/// a fifth of a turn apart round `tPos` from the caster's bearing, 300,
/// 400 or 500 out (n % 3), each on the land there (or 300 down), 96.5
/// under it, heading round; each waits 1-21 frames by its serial number.
/// The game builds each place in `cPos` itself, which the spell keeps:
/// its z gathers `tPos.z` each time round. The last.
pub fn eff_tc_drill_missile(ctrl: &mut EffectCtrl, cx: &mut Cx, cp: &mut V4, tp: V4, n: i32) -> Option<usize> {
    let d = ee::vsub(tp, *cp);
    let bearing = ee::rad2deg(ee::atan2f(d[1], d[0]));
    let k = (n % 3) as usize;
    let radius = [0x4396_0000, 0x43c8_0000, 0x43fa_0000][k];
    let mut last = None;
    for i in 0..5 {
        let Some(s) = ctrl.free_slot() else { continue };
        ctrl.init_effect(cx, s, DRILL);
        last = Some(s);
        let step = ((i << 16) / 5) as i16;
        let turn = ee::deg2rad((i32::from(bearing) + i32::from(step) + 16384) as i16);
        let a = ee::deg2rad((i32::from(bearing) + i32::from(step) + 32767 + 1) as i16);
        cp[0] = ee::mul(radius, ee::cosf(a));
        cp[1] = ee::mul(radius, ee::sinf(a));
        *cp = ee::vadd(*cp, tp);
        cp[2] = ee::add(cp[2], 0x4396_0000);
        cp[3] = ONE;
        let z = cp[2];
        let g = land(cx, *cp);
        cp[2] = g;
        if ee::eq(z, g) {
            cp[2] = ee::sub(z, 0x4396_0000);
        }
        cp[2] = ee::add(cp[2], 0xc2c1_0000);
        let e = &mut ctrl.effects[s];
        e.pos = *cp;
        e.pos_t = tp;
        e.rot = [0, 0, turn, 0];
        e.scale[3] = radius;
        e.temp[3] = (e.sn % 5) * 5 + 1;
        e.param = i32::from(ee::rad2deg(a));
        e.zyx_flag = false;
        let own = std::mem::replace(&mut e.obj, Obj::None);
        e.obj = own.clone();
        let anm = [x703_anm(cx, "ANM_x703nut0"), x703_anm(cx, "ANM_x703atc0")];
        cx.spells.drills.insert(s, Drill { anm, own: Some(own) });
        ctrl.effects[s].temp[2] = i as u32;
    }
    last
}

/// `(RAD2DEG(a) + d)` as a short, back to radians.
fn nudge(a: F, d: i32) -> F {
    ee::deg2rad((i32::from(ee::rad2deg(a)) + d) as i16)
}

/// The drill's case of the first switch (main 0x001c6950, its states by
/// `flags`; jump table 0x00374c80).
pub fn drill_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let state = ctrl.effects[i].flags;
    if !(0..10).contains(&state) {
        return Next::End;
    }
    let cnt = i32::from(ctrl.effects[i].cnt);
    match state {
        0 => {
            if cnt >= ctrl.effects[i].temp[3] as i32 {
                let e = &mut ctrl.effects[i];
                e.flags = 1;
                e.offset[3] = ee::add(0x4316_0000, e.pos[2]);
                e.speed[2] = 0x4220_0000;
                e.tex_anm_pat = 1;
                let pos = e.pos;
                let mut r = VF0;
                r[0] = ee::deg2rad(-16384);
                let mut p = pos;
                p[2] = ee::sub(ee::sub(e.offset[3], 0x4256_0000), 0x41f0_0000);
                eff_smmons_fragment(ctrl, cx, p, r, 0x4220_0000, 3, attr::SOIL);
                let mut p = pos;
                p[2] = ee::add(p[2], 0x4316_0000);
                cx.raise(Event::Sound3d { se: 39, pos: p });
                if check_camera_shake_range(cx, p) {
                    camera_shake(cx, 0, 2, 8, 0);
                }
                pfx::start(cx, 241, |g| g.pos = p);
            }
        }
        1 => {
            let e = &mut ctrl.effects[i];
            e.pos[2] = ee::add(e.pos[2], e.speed[2]);
            e.speed[2] = ee::sub(e.speed[2], 0x4060_0000);
            if ee::le(e.pos[2], e.offset[3]) && ee::lt(e.speed[2], 0) {
                e.flags += 1;
                e.tex_anm_pat = 0;
                e.pos[2] = e.offset[3];
            }
        }
        2 => {
            if cnt >= 40 {
                let e = &mut ctrl.effects[i];
                e.flags += 1;
                e.cnt = 0;
            }
        }
        3 => {
            let e = &mut ctrl.effects[i];
            if cnt >= 5 {
                e.flags += 1;
                e.cnt = 0;
            } else {
                e.rot[2] = nudge(e.rot[2], -3276);
            }
        }
        4 => {
            if cnt >= 40 {
                let e = &mut ctrl.effects[i];
                e.flags += 1;
                e.cnt = 0;
            } else {
                let e = &mut ctrl.effects[i];
                e.param = e.param.wrapping_add(256);
                let a = ee::deg2rad(e.param as i16);
                let z = e.pos[2];
                let r = e.scale[3];
                e.pos = e.pos_t;
                e.pos[0] = ee::add(e.pos[0], ee::mul(r, ee::cosf(a)));
                e.pos[1] = ee::add(e.pos[1], ee::mul(r, ee::sinf(a)));
                e.pos[2] = ee::add(e.pos[2], r);
                e.pos[3] = ONE;
                let p = e.pos;
                let g = land(cx, p);
                let e = &mut ctrl.effects[i];
                let far = f64::from(f32::from_bits(ee::sub(g, z))).abs() > 100.0;
                if ee::eq(g, e.pos[2]) || far {
                    e.pos[2] = ee::sub(e.pos[2], r);
                } else {
                    e.pos[2] = g;
                }
                e.pos[2] = ee::add(e.pos[2], 0x4256_0000);
                e.rot[2] = ee::deg2rad((e.param + 32767 + 1) as i16);
                if e.temp[2] == 0 && cnt % 25 == 1 {
                    let p = e.pos;
                    cx.raise(Event::Sound3d { se: 100, pos: p });
                }
            }
        }
        5 => {
            let e = &mut ctrl.effects[i];
            if cnt >= 5 {
                e.flags += 1;
                e.cnt = 0;
                e.rot[2] = ee::deg2rad((e.param + 32767 + 16385) as i16);
                e.temp[3] = (e.sn % 5) * 10 + 3;
            } else {
                e.rot[2] = nudge(e.rot[2], 3276);
            }
        }
        6 => {
            if cnt >= ctrl.effects[i].temp[3] as i32 {
                let e = &mut ctrl.effects[i];
                e.flags += 1;
                e.cnt = 0;
                e.tex_anm_pat = 1;
                e.speed[2] = 0x41f0_0000;
                let t = ee::mul(0x4000_0000, ee::div(e.speed[2], 0x4060_0000));
                let v = ee::div(ee::sub(e.scale[3], 0x41c8_0000), t);
                let a = ee::deg2rad((e.param + 32767 + 1) as i16);
                e.speed[0] = ee::mul(v, ee::cosf(a));
                e.speed[1] = ee::mul(v, ee::sinf(a));
                e.speed[3] = 0;
                e.offset[0] = v;
                let pt = e.pos_t;
                let g = land(cx, pt);
                let e = &mut ctrl.effects[i];
                e.offset[3] = g;
                let p = e.pos;
                cx.raise(Event::Sound3d { se: 101, pos: p });
            }
        }
        7 => {
            let e = &mut ctrl.effects[i];
            let old = e.pos;
            e.pos = ee::vadd(e.pos, e.speed);
            let d = ee::vsub(e.pos, old);
            let len = ee::sqrtf(ee::dot(d, d));
            let up = ee::rad2deg(asinf(ee::div(d[2], len)));
            e.rot[0] = ee::deg2rad((16384 - i32::from(up)) as i16);
            e.speed[2] = ee::sub(e.speed[2], 0x4060_0000);
            let down = ee::le(e.pos[2], e.offset[3]) || ee::le(e.pos[2], 0xc3fa_0000);
            if down && ee::lt(e.speed[2], 0) {
                e.flags += 1;
                e.tex_anm_pat = 0;
                e.pos[2] = e.offset[3];
                e.cnt = 0;
                let p = e.pos;
                fall::eff_skill_break_se(cx, p, attr::FIRE);
                noise(cx, 30);
                if check_camera_shake_range(cx, p) {
                    camera_shake(cx, 2, 1, 10, 2);
                }
                let mut r = VF0;
                r[0] = ee::deg2rad(-16384);
                debris::eff_smoke_rock(ctrl, cx, p, r, 0x4248_0000, 3, 0);
                let mut q = p;
                q[2] = ee::add(q[2], 0x41a0_0000);
                fall::eff_flare_ring(ctrl, cx, q, 1);
                pfx::start(cx, 239, |g| g.pos = q);
                pfx::start(cx, 240, |g| g.pos = p);
            }
        }
        _ => {
            ctrl.effects[i].flags += 1;
            if let Some(d) = cx.spells.drills.get_mut(&i) {
                d.anm = [None, None];
            }
            return Next::End;
        }
    }
    // The one it draws, by texAnmPat, in its object's place for the draw.
    let pat = usize::from(ctrl.effects[i].tex_anm_pat.min(1));
    if let Some(d) = cx.spells.drills.get_mut(&i) {
        let a = d.anm[pat].take().unwrap_or(Obj::None);
        ctrl.effects[i].obj = a;
    }
    Next::Draw
}

/// The drill's case of the second chain: its `ccAnm` back, its row's
/// object in place again.
pub fn drill_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let pat = usize::from(ctrl.effects[i].tex_anm_pat.min(1));
    if let Some(d) = cx.spells.drills.get_mut(&i) {
        let own = d.own.clone().unwrap_or(Obj::None);
        let a = std::mem::replace(&mut ctrl.effects[i].obj, own);
        if !matches!(a, Obj::None) {
            d.anm[pat] = Some(a);
        }
    }
}

/// newlib's `__ieee754_asinf` (main 0x00123f78) as the EE runs it.
pub fn asinf(x: F) -> F {
    use ee::{add, div, mul, sub};
    const PIO2_HI: F = 0x3fc9_0fda;
    const PIO2_LO: F = 0x33a2_2168;
    const PIO4_HI: F = 0x3f49_0fdb;
    const HUGE: F = 0x7149_f2ca;
    const PS: [F; 6] = [0x3e2a_aaab, 0xbea6_b090, 0x3e4e_0aa8, 0xbd24_1146, 0x3a4f_7f04, 0x3811_ef08];
    const QS: [F; 4] = [0xc019_d139, 0x4001_572d, 0xbf30_3361, 0x3d9d_c62e];
    // p(t) and q(t) as the code evaluates them, step for step.
    let pq = |t: F| {
        let mut p = mul(t, PS[5]);
        let mut q = mul(t, QS[3]);
        p = add(p, PS[4]);
        q = add(q, QS[2]);
        p = mul(t, p);
        q = mul(t, q);
        p = add(p, PS[3]);
        q = add(q, QS[1]);
        p = mul(t, p);
        q = mul(t, q);
        p = add(p, PS[2]);
        q = add(q, QS[0]);
        p = mul(t, p);
        q = mul(t, q);
        p = add(p, PS[1]);
        let q = add(q, ONE);
        p = mul(t, p);
        p = add(p, PS[0]);
        (mul(t, p), q)
    };
    let ix = x & 0x7fff_ffff;
    if ix == 0x3f80_0000 {
        return add(mul(x, PIO2_HI), mul(x, PIO2_LO));
    }
    if ix > 0x3f80_0000 {
        let d = sub(x, x);
        return div(d, d);
    }
    if ix < 0x3f00_0000 {
        if ix >= 0x3200_0000 {
            let t = mul(x, x);
            let (p, q) = pq(t);
            return add(x, mul(x, div(p, q)));
        }
        if ee::lt(ONE, add(x, HUGE)) {
            return x;
        }
    }
    let w = sub(ONE, ee::fabsf(x));
    let t = mul(w, 0x3f00_0000);
    let (p, q) = pq(t);
    let s = ee::sqrtf(t);
    let r = if ix > 0x3f79_9999 {
        let w = div(p, q);
        let a = add(s, mul(s, w));
        sub(PIO2_HI, sub(add(a, a), PIO2_LO))
    } else {
        let df = s & 0xffff_f000;
        let c = sub(t, mul(df, df));
        let sd = add(s, df);
        let r = div(p, q);
        let s2 = add(s, s);
        let d2 = add(df, df);
        let c = div(c, sd);
        let q = sub(PIO4_HI, d2);
        let p2 = mul(s2, r);
        let c2 = add(c, c);
        let p = sub(p2, sub(PIO2_LO, c2));
        sub(PIO4_HI, sub(p, q))
    };
    if (x as i32) > 0 { r } else { ee::neg(r) }
}
