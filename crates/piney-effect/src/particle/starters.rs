//! What the rest of the game starts particles with (main particle.cpp): the
//! hit spark (`ccParticleHitMark`), the heal (`ccParticleHeal`), the
//! bosses' and effects' bursts (`ccParticleExplode`, `ccParticleSetup`),
//! and the character effects of `particleEffectTbl` (`startParticleEffect`,
//! `startParticleEffect2`). The battle marks particle.cpp also holds -
//! `ccParticleCritical`, `ccParticleDying`, `ccParticleNoDamage`,
//! `ccParticleAttributeCritical`, `ccParticleAttributeGuard`, which start
//! effects as well as generators - are with the hits (hit.rs).
//!
//! ```text
//! ccParticleHitMark(pos)          generator 1 at pos
//! ccParticleHeal(ch, _)           generator 2 on ch's pos
//! ccParticleExplode(pos, v, s, t) one particle (texture 10, 155, 44 or
//!                                 202 + t for t 3-7) at pos flying v, its
//!                                 speed |v|, scale s
//! ccParticleSetup(t, pos, life, pat, gp)  one particle of texture t at pos
//! startParticleEffect(ch, num)    particleEffectTbl[num]'s generators on
//!                                 ch (following it when the row syncs)
//! startParticleEffect2(s, e, num, sw)  ... between two points, while *sw
//! ```

use super::{GenRef, Generator, one};
use crate::ee::{self, F, V4};
use crate::effect::EffectCtrl;
use crate::{CharRef, Cx, IntRef, VecRef};

/// 0.5.
const HALF: F = 0x3f00_0000;

/// `ccParticleHitMark`'s and `ccParticleHeal`'s `particleGeneratorTbl` rows.
pub const HIT_MARK_ROW: usize = 1;
pub const HEAL_ROW: usize = 2;

/// `ccParticleHitMark(pos)` (main 0x001bc420): generator 1 at `pos`.
pub fn cc_particle_hit_mark(cx: &mut Cx, pos: V4) -> GenRef {
    let mut g = cx.particles.generator(cx.assets, HIT_MARK_ROW);
    g.pos = pos;
    cx.particles.start(g)
}

/// `ccParticleHeal(target, n)` (main 0x001bc4b0): generator 2 following
/// `target`'s pos (`syncPosType` 0, `syncPos` its +0x40; `n` is not read).
pub fn cc_particle_heal(cx: &mut Cx, target: CharRef, _n: i32) -> GenRef {
    let mut g = cx.particles.generator(cx.assets, HEAL_ROW);
    g.sync_pos_type = false;
    g.sync_pos = Some(VecRef::CharPos(target));
    cx.particles.start(g)
}

/// `ccParticleSetup(t, pos, life, pat, gp)` (main 0x001c2940): one
/// particle of texture `t` (from `pat` of them) at `pos`, lifeTime `life`,
/// counted to generator `gp` (whose pCnt it does not raise); its slot.
pub fn cc_particle_setup(cx: &mut Cx, t: i32, pos: V4, life: i16, pat: i32, gp: Option<GenRef>) -> Option<usize> {
    let p = &mut *cx.particles;
    let s = p.generate_particle()?;
    let gene = gp.and_then(|r| p.gene_at(r));
    let mut slot = std::mem::take(&mut p.slots[s]);
    let mut serial = p.particle_serial;
    one::setup(&mut slot, gene.map(|at| p.gene(at)), t, pat, true, &mut *cx.host, cx.assets, &mut serial);
    p.particle_serial = serial;
    p.slots[s] = slot;
    if gene.is_none() {
        // A generator no longer on the list: the game keeps the stale
        // pointer; here the particle has none.
        p.slots[s].gene = None;
    }
    p.slots[s].pos = pos;
    p.slots[s].life_time = life;
    p.count_up();
    p.p2_num = p.p2_num.wrapping_add(1);
    Some(s)
}

/// `ccParticleExplode(pos, speed, scale, type)` (main 0x001bccd0): one
/// particle flying `speed`; its slot. Types past 7 read a texture from
/// the caller's pos pointer in the game; here they make none.
pub fn cc_particle_explode(cx: &mut Cx, pos: V4, speed: V4, scale: F, kind: i32) -> Option<usize> {
    let t = match kind {
        0 => 10,
        1 => 155,
        2 => 44,
        3..=7 => kind + 202,
        _ => return None,
    };
    let s = cc_particle_setup(cx, t, pos, -1, 0, None)?;
    let p = &mut cx.particles.slots[s];
    p.velocity = speed;
    // sqrtf, then fabs through doubles.
    p.speed = ee::fabsf(ee::sqrtf(ee::dot(speed, speed)));
    p.scale[1] = scale;
    p.scale[0] = scale;
    Some(s)
}

/// `startParticleEffect(cp, num)` (main 0x001c2560): each generator of
/// `particleEffectTbl[num]` on `cp` - following it `offset` up (plus its
/// height for `sync` 2, half of it for 3) when the row syncs, else placed
/// there once.
pub fn start_particle_effect(cx: &mut Cx, cp: CharRef, num: usize) -> Vec<GenRef> {
    let h = cx.host.char_height(cp);
    let h2 = ee::mul(HALF, h);
    let row = cx.assets.particle.effects[num];
    let mut out = Vec::new();
    for k in 0..4 {
        let Ok(gr) = usize::try_from(row.gene_num[k]) else { continue };
        let mut g = cx.particles.generator(cx.assets, gr);
        if g.g_sync {
            let mut z = row.offset[k];
            match row.sync[k] {
                2 => z = ee::add(z, h),
                3 => z = ee::add(z, h2),
                _ => {}
            }
            g.offset = [0, 0, z, 0];
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::CharPos(cp));
        } else {
            let mut at = cx.host.char_pos(cp);
            at[2] = ee::add(at[2], row.offset[k]);
            g.pos = at;
        }
        out.push(cx.particles.start(g));
    }
    out
}

/// `startParticleEffect2(s, e, num, sw)` (main 0x001c2770): each generator
/// of `particleEffectTbl[num]` between `s` and `e` (followed when the row
/// syncs, else read once), running while `sw` is not 0; each marked to
/// take its particles with it (killFlag 1).
pub fn start_particle_effect2(
    ctrl: &EffectCtrl,
    cx: &mut Cx,
    s: VecRef,
    e: VecRef,
    num: usize,
    sw: Option<IntRef>,
) -> Vec<GenRef> {
    let row = cx.assets.particle.effects[num];
    let mut out = Vec::new();
    for k in 0..4 {
        let Ok(gr) = usize::try_from(row.gene_num[k]) else { continue };
        let mut g: Generator = cx.particles.generator(cx.assets, gr);
        g.sync_sw = sw;
        if g.g_sync {
            g.sync_pos_type = false;
            g.sync_pos = Some(s);
            g.sync_pos_type2 = false;
            g.sync_pos2 = Some(e);
        } else {
            g.pos = super::resolve(&*cx.host, ctrl, s);
            g.pos2 = super::resolve(&*cx.host, ctrl, e);
        }
        if !cx.particles.check(g.sn) {
            g.kill_flag = 1;
        }
        out.push(cx.particles.start(g));
    }
    out
}
