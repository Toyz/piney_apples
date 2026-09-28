//! `ccConditionEffect` (main particle.cpp, 0x20 bytes): the particles about a
//! character under a condition (poison, sleep, the stat changes...).
//! `ccChar::DispConditionEffect` makes one with `setConditionEffect`
//! (0x001c24a0; constructor 0x001c13f0) for the condition number (its +0x30)
//! and ends it with `killConditionEffect` (0x001c24f0: the particles fade
//! out) or `deleteConditionEffect` (0x001c2520: they end at once). The rows
//! by condition are in docs/engine/particles.md ("ccConditionEffect").

use super::{GenRef, Generator};
use crate::ee::{self, F, V4};
use crate::effect::EffectCtrl;
use crate::{CharRef, Cx, VecRef};

/// A `ccConditionEffect`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConditionEffect {
    /// +0x00 `gene[5]`.
    pub gene: [Option<GenRef>; 5],
    /// +0x14 `eff` (a slot of the effects), +0x18 `effSN` (its serial
    /// number then).
    pub eff: Option<usize>,
    pub eff_sn: u32,
    /// +0x1c `now`: the condition; 0 or -1 when ended.
    pub now: i32,
}

/// Which of the effect helpers' functions the condition's effect is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ability {
    /// `effAbilityDown(cp, num)` (num 0-18).
    Down,
    /// `effAbilityUp(cp, num)` (num 20-34).
    Up,
}

/// 0.5, 50.0.
const HALF: F = 0x3f00_0000;
const FIFTY: F = 0x4248_0000;

/// A generator of `row` (pTexMod `tex` unless -1, force fields `ff` given)
/// following `cp` `z` up, marked and started, as each of the constructor's
/// are.
fn make(cx: &mut Cx, row: usize, tex: i16, ff: [Option<usize>; 2], cp: CharRef, z: F) -> GenRef {
    let t = &cx.assets.particle;
    let mut g: Generator = if ff.iter().any(Option::is_some) {
        let ffs = [ff[0].map(|i| t.force_fields[i]), ff[1].map(|i| t.force_fields[i]), None, None];
        Generator::new(Some(t.generators[row]), ffs, &t.force_fields, &mut cx.particles.generator_serial)
    } else {
        cx.particles.generator(cx.assets, row)
    };
    if tex != -1 {
        g.p_tex_mod = tex;
    }
    g.offset = [0, 0, z, 0];
    g.sync_pos_type = false;
    g.sync_pos = Some(VecRef::CharPos(cp));
    if !cx.particles.check(g.sn) {
        g.kill_flag = 1;
    }
    cx.particles.start(g)
}

/// `new ccConditionEffect(cp, num)` (main 0x001c13f0) =
/// `setConditionEffect(cp)` with `num` its +0x30; `ability` runs the
/// effect helpers' `effAbilityDown` / `effAbilityUp(cp, num)` and answers
/// the effect's slot.
pub fn set_condition_effect(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    cp: CharRef,
    num: i32,
    ability: impl FnOnce(&mut EffectCtrl, &mut Cx, CharRef, i32, Ability) -> Option<usize>,
) -> ConditionEffect {
    let mut ep = ConditionEffect { now: num, ..Default::default() };
    let h = cx.host.char_height(cp);
    let h2 = ee::add(0, ee::mul(HALF, h));
    let h1 = ee::add(0, h);
    let mut default_rows = false;
    match num {
        0 | 3 | 5 => {
            let (row, t0, t1) = match num {
                0 => (197, -1, -1),
                3 => (197, 17, 144),
                _ => (199, -1, 145),
            };
            ep.gene[0] = Some(make(cx, row, t0, [None; 2], cp, h1));
            ep.gene[1] = Some(make(cx, 198, t1, [None; 2], cp, h2));
        }
        1 => {
            ep.gene[0] = Some(make(cx, 198, 158, [None; 2], cp, h2));
            ep.gene[1] = Some(make(cx, 198, 143, [None; 2], cp, h2));
        }
        2 | 4 | 6 | 22 => {
            let (row, t, t4) = match num {
                2 => (201, 16, 145),
                4 => (201, 18, 145),
                6 => (201, 36, 147),
                _ => (202, 16, 145),
            };
            ep.gene[0] = Some(make(cx, 200, t, [None; 2], cp, h1));
            for k in 0..3 {
                ep.gene[1 + k] = Some(make(cx, row, t, [None; 2], cp, h1));
            }
            ep.gene[4] = Some(make(cx, 198, t4, [None; 2], cp, h2));
        }
        13..=18 | 29..=34 => {
            let (a, b) = if num <= 18 { (31, 204) } else { (37, 203) };
            let (t0, t1) = match (num - 13) % 16 {
                0 => (107, 202),
                1 => (0, 198),
                2 => (102, 199),
                3 => (104, 43),
                4 => (103, 200),
                _ => (105, 201),
            };
            ep.gene[0] = Some(make(cx, a, t0, [None; 2], cp, h2));
            ep.gene[1] = Some(make(cx, b, t1, [None; 2], cp, h2));
        }
        20 | 21 => {
            let t = if num == 20 { 119 } else { 118 };
            ep.gene[4] = Some(make(cx, 205, t, [None; 2], cp, h1));
            default_rows = true;
        }
        -1 => {
            for k in 0..2 {
                ep.gene[k] = Some(make(cx, 209, -1, [Some(105 + k), Some(107 + k)], cp, h1));
            }
            ep.gene[2] = Some(make(cx, 210, -1, [None; 2], cp, ee::add(0, ee::add(FIFTY, h))));
        }
        _ => default_rows = true,
    }
    if default_rows {
        let row = usize::try_from(num).ok().and_then(|n| cx.assets.particle.effects.get(n)).copied();
        if let Some(row) = row {
            for k in 0..4 {
                let Ok(gr) = usize::try_from(row.gene_num[k]) else { continue };
                let mut g = cx.particles.generator(cx.assets, gr);
                if g.g_sync {
                    let mut z = row.offset[k];
                    if row.sync[k] == 2 {
                        z = ee::add(z, h);
                    }
                    g.offset = [0, 0, z, 0];
                    g.sync_pos_type = false;
                    g.sync_pos = Some(VecRef::CharPos(cp));
                } else {
                    let mut at: V4 = cx.host.char_pos(cp);
                    at[2] = ee::add(at[2], row.offset[k]);
                    g.pos = at;
                }
                if !cx.particles.check(g.sn) {
                    g.kill_flag = 1;
                }
                ep.gene[k] = Some(cx.particles.start(g));
            }
        }
    }
    let which = match num {
        0..=18 => Some(Ability::Down),
        20..=34 => Some(Ability::Up),
        _ => None,
    };
    if let Some(a) = which {
        ep.eff = ability(ctrl, cx, cp, num, a);
        if let Some(e) = ep.eff {
            ep.eff_sn = ctrl.effects[e].sn;
        }
    }
    ep
}

/// `~ccConditionEffect()` (main 0x001c2360) with `now` as set.
fn end(ctrl: &mut EffectCtrl, cx: &mut Cx, ep: &mut ConditionEffect) {
    for g in &mut ep.gene {
        if let Some(r) = g.take() {
            match ep.now {
                0 => cx.particles.particle_kill(r),
                -1 => cx.particles.particle_delete(r),
                _ => {}
            }
        }
    }
    if let Some(e) = ep.eff {
        let e = &mut ctrl.effects[e];
        if e.sn == ep.eff_sn {
            e.end_flag = true;
        }
    }
}

/// `killConditionEffect(ep)` (main 0x001c24f0): `now` 0, then deleted -
/// the generators killed (their particles fade out), the effect ended.
pub fn kill_condition_effect(ctrl: &mut EffectCtrl, cx: &mut Cx, mut ep: ConditionEffect) {
    ep.now = 0;
    end(ctrl, cx, &mut ep);
}

/// `deleteConditionEffect(ep)` (main 0x001c2520): `now` -1, then deleted -
/// the generators deleted (their particles end at once).
pub fn delete_condition_effect(ctrl: &mut EffectCtrl, cx: &mut Cx, mut ep: ConditionEffect) {
    ep.now = -1;
    end(ctrl, cx, &mut ep);
}
