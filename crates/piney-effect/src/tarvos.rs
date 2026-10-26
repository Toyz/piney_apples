//! The pictures of Tarvos's effects (QUA gcmn bosseff.cpp): the spike
//! rain's clumps (`ccBossEffSpikeRain::Draw` 0x00373d60), the phantom's
//! sprite and its lights' generators (`ccBossEffPhantom`, 0x0037a4b0 on),
//! the transfer's, the eruption's and the explosion's generators, the
//! lights' bursts and the guard's shield. The rules own every step and
//! `ccRand` (`piney_battle::boss::tarvos`); this draws what they say.

use piney_battle::boss::tarvos::Pic;

use crate::boss::{Tables, ff_row, gen_row, va};
use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4, VF0};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::particle::GenRef;
use crate::{CharRef, Cx, VecRef, debris, drawelm, shield, vu};

const QUARTER: F = 0x3e80_0000;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// The phantom's sprite's colour (`ccEff` +0x2c, 0x80 = 1).
const SPRITE_COLOUR: u32 = 0x80e0_ffff;
/// The lights' generators' `pTexMod`: 109 and the light's number's low
/// three bits.
const LIGHT_TEX: i16 = 109;
/// The anchors the lights' generators follow (`syncPos`, the light's
/// +0x10): one per light.
const ANCHOR: u32 = 0x0710_0000;

/// The phantom as the effects keep it: its sprite, each light's generator
/// and the lights' states as last told.
#[derive(Clone, Debug, Default)]
pub struct Kept {
    sprite: Option<Box<Eff>>,
    gens: [Option<GenRef>; 14],
    states: [i32; 14],
}

/// What a picture of Tarvos's asks of the effects. `me` is the boss (the
/// shield's character).
pub fn calls(ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables, kept: &mut Kept, me: CharRef, p: &Pic) {
    match p {
        // The spike at a quarter: Rx(-pi/2) Rx(pitch) Rz(heading), then
        // its place, at its transparency.
        &Pic::Spike { pos, pitch, heading, transparency } => {
            let mut m = vu::UNIT;
            (m[0][0], m[1][1], m[2][2]) = (QUARTER, QUARTER, QUARTER);
            let m = vu::rot_x(&m, NEG_HALF_PI);
            let m = vu::rot_x(&m, pitch);
            let m = vu::rot_z(&m, heading);
            let m = vu::trans(&m, pos);
            if let Some(obj) = cx.assets.find("xeffect", "CMP_ex71body2") {
                cx.draws.push(DrawRec::Clump {
                    obj,
                    matrix: m,
                    alpha: transparency,
                    layer: EFFECT_LAYER,
                    clut: None,
                    fog: true,
                });
            }
        }
        Pic::PhantomStart => phantom_start(cx, t, kept),
        Pic::PhantomLights(lights) => phantom_lights(ctrl, cx, t, kept, lights),
        // `ccEff::Draw(pos, 0)` of particle's `EFF_x000` on `CLT_x000c7`:
        // `size` each way, no depth test, 0x80e0ffff.
        &Pic::PhantomSprite { pos, size } => {
            if kept.sprite.is_none() {
                kept.sprite = crate::boss::particle_eff(cx, "EFF_x000").map(|mut e| {
                    e.clut = cx.assets.find("particle", "CLT_x000c7");
                    e
                });
            }
            if let Some(e) = kept.sprite.as_mut() {
                e.scale_x = size;
                e.scale_y = size;
                e.test &= !(1 << 16);
                e.color = SPRITE_COLOUR;
                e.pos[..3].copy_from_slice(&pos[..3]);
                cx.draws.push(DrawRec::Eff { eff: e.clone(), pat: 0, layer: EFFECT_LAYER });
            }
        }
        // `effExplode3(pos, (0, 0, 0, 1), 1, kind)`, grown 1 to 4 over 15
        // frames (`SetScaleAnm(1, 4, 15)`).
        &Pic::Burst { pos, kind } => {
            if let Some(k) = drawelm::eff_explode3(cx, pos, [0, 0, 0, ONE], ONE, kind)
                && let Some(b) = cx.spells.elements.base_mut(k)
            {
                b.anim.set_scale_anm_f(ONE, 0x4080_0000, 0x4170_0000);
            }
        }
        // `effRadiateSomething2(pos, (0, -pi/2, 0, 1), 45, 4, 16)`.
        &Pic::Radiate { pos } => {
            let r = [0, ee::deg2rad(-16384), 0, ONE];
            debris::eff_radiate_something2(ctrl, cx, pos, r, 0x4234_0000, 4, 16);
        }
        &Pic::Explode { pos } => crate::boss::explode2(cx, t, pos),
        &Pic::TransEffect { pos } => trans_effect(cx, t, pos),
        &Pic::Eruption { pos } => eruption(cx, t, pos),
        // `effResistantShield(this, magic, -1)`, its scale times 0.7 and
        // its offset 80 up.
        &Pic::Shield { magic } => {
            if let Some(k) = shield::eff_resistant_shield(cx, me, magic, -1)
                && let Some(b) = cx.spells.elements.base_mut(k)
            {
                b.anim.scale = ee::vscale(b.anim.scale, 0x3f33_3333);
                b.anim.offset[2] = ee::add(b.anim.offset[2], 0x42a0_0000);
            }
        }
        // The waves and the fog: the field's.
        Pic::Wave { .. } | Pic::Fog { .. } => {}
    }
}

/// `ccBossEffTransEffectCreate(pos)` (0x0037db00): `g_transEffGen` at
/// `pos`; then `MagicSquareGenerator` row 6 (ff 8, 7) made, row 5 (ff 6,
/// 10, 9) made and started with `pTexMod` 114, row 6 started with 31.
fn trans_effect(cx: &mut Cx, t: &Tables, pos: V4) {
    if let Some(mut g) = t.generator(cx, va::TRANS_EFF, [va::TRANS_EFF_FF, 0, 0, 0]) {
        g.pos = pos;
        cx.particles.start(g);
    }
    let ff = |rows: &[u32]| std::array::from_fn(|k| rows.get(k).map_or(0, |&r| ff_row(va::MAGIC_SQUARE_FF, r)));
    let six = t.generator(cx, gen_row(va::MAGIC_SQUARE, 6), ff(&[8, 7]));
    let five = t.generator(cx, gen_row(va::MAGIC_SQUARE, 5), ff(&[6, 10, 9]));
    for (g, tex) in [(five, 114), (six, 31)] {
        if let Some(mut g) = g {
            g.pos = pos;
            g.p_tex_mod = tex;
            cx.particles.start(g);
        }
    }
}

/// `ccBossEffEruptionCreate(pos, 102, 100, 10)` (0x0037b7b0):
/// `g_eruptionGenerator` (both its force fields) at `pos`, no offset,
/// `pTexMod` 102.
fn eruption(cx: &mut Cx, t: &Tables, pos: V4) {
    if let Some(mut g) = t.generator(cx, va::ERUPTION, [va::ERUPTION_FF, va::ERUPTION_FF + 0x20, 0, 0]) {
        g.pos = pos;
        g.offset = VF0;
        g.p_tex_mod = 102;
        cx.particles.start(g);
    }
}

/// The constructor's generators: `g_phantomGen` row 0 for each light,
/// following it (`syncPos` its place), `pTexMod` 109 + its low bits,
/// paused; the first eight going.
fn phantom_start(cx: &mut Cx, t: &Tables, kept: &mut Kept) {
    *kept = Kept::default();
    for k in 0..14 {
        let Some(mut g) = t.generator(cx, va::PHANTOM, [va::PHANTOM_FF, 0, 0, 0]) else { continue };
        g.p_tex_mod = LIGHT_TEX + (k as i16 & 7);
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::Anchor(ANCHOR + k as u32));
        g.pause = k >= 8;
        kept.gens[k] = Some(cx.particles.start(g));
    }
}

/// The lights this frame: each anchor where its light is; all of them
/// set to fall (`ChangePtclGenerator`: the old generators deleted, row 1
/// on lights 0-6, `pSyncFlag` on, `pTexMod` 109 + 2k's low bits); a light
/// burst, its generator deleted.
fn phantom_lights(ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables, kept: &mut Kept, lights: &[(V4, i32); 14]) {
    for (k, &(pos, _)) in lights.iter().enumerate() {
        ctrl.anchors.insert(ANCHOR + k as u32, pos);
    }
    let change = lights.iter().zip(kept.states).any(|(&(_, s), was)| s == 3 && was == 0);
    if change {
        for g in kept.gens.iter_mut() {
            if let Some(r) = g.take() {
                if let Some(x) = cx.particles.get_mut(r) {
                    x.pause = false;
                }
                cx.particles.particle_delete(r);
            }
        }
        for k in 0..7 {
            let Some(mut g) = t.generator(cx, gen_row(va::PHANTOM, 1), [va::PHANTOM_FF, 0, 0, 0]) else { continue };
            g.p_sync = true;
            g.p_tex_mod = LIGHT_TEX + ((2 * k) as i16 & 7);
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::Anchor(ANCHOR + k as u32));
            kept.gens[k] = Some(cx.particles.start(g));
        }
    }
    for (k, &(_, s)) in lights.iter().enumerate() {
        if s == 5
            && kept.states[k] != 5
            && let Some(r) = kept.gens[k].take()
        {
            cx.particles.particle_delete(r);
        }
        kept.states[k] = s;
    }
}
