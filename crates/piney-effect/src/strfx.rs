//! The stream demo's effects: `ccEffectCtrl(1)` (`effcStr`, 50 slots, run by
//! `ccThEffectStr` 0x001c2f70 at priority 80 with `MainStr`), what the
//! streams' effect tasks start in it (`effHitMarkStr` 0x001cc620,
//! `effTransferStr` 0x001ce220), and `ccEffect::MainStr` (0x001d8810): as
//! `Main`, with its own cases (the field's 131, 3, -23 and the arrival) and a
//! draw with no `ccTransPosFW2LW`, camera cone or distance fade. See
//! docs/engine/effects.md ("The streams' effects").

use crate::ee::{self, F, ONE, V4};
use crate::effect::{EffectCtrl, Next};
use crate::{Cx, VecRef, arrival, hit};

pub const HIT_MARK: i16 = 0;
pub const HIT_RING: i16 = 1;
pub const TRANSFER_RING: i16 = 2;
pub const HIT_PHOTON: i16 = -1;
pub const TRANSFER: i16 = -2;

/// `particle`'s palettes the stream's hit marks draw through.
const FILE: &str = "particle";
const MARK_CLUT: &str = "CLT_x007c1";
const RING_CLUT: &str = "CLT_x037";
const RING_CLUT_HIT: &str = "CLT_x037c1";

/// `effHitMarkStr(pos, rot, layer)` (main 0x001cc620): the spark, the ring
/// and the photons; the ring's slot.
pub fn eff_hit_mark_str(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4, rot: V4, layer: Option<i16>) -> Option<usize> {
    if let Some(i) = ctrl.new_effect(cx, HIT_MARK) {
        let clut = cx.assets.find(FILE, MARK_CLUT);
        let e = &mut ctrl.effects[i];
        e.layer = layer.or(e.layer);
        e.pos = pos;
        if let crate::effect::Obj::Eff(eff) = &mut e.obj {
            eff.clut = clut;
        }
    }
    let ring = ctrl.new_effect(cx, HIT_RING);
    if let Some(i) = ring {
        let swap = cx.assets.find(FILE, RING_CLUT).zip(cx.assets.find(FILE, RING_CLUT_HIT));
        let e = &mut ctrl.effects[i];
        e.layer = layer.or(e.layer);
        e.life_time = 15;
        e.pos = pos;
        e.rot = rot;
        e.zyx_flag = false;
        e.clut_swap = swap.map(|(a, b)| (a.object, b.object));
    }
    if let Some(i) = ctrl.new_effect(cx, HIT_PHOTON) {
        let e = &mut ctrl.effects[i];
        e.layer = layer.or(e.layer);
        e.pos = pos;
        e.rot = rot;
    }
    ring
}

/// `effTransferStr(pos, height, layer)` (main 0x001ce220).
pub fn eff_transfer_str(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4, height: F, layer: Option<i16>) -> Option<usize> {
    let i = ctrl.new_effect(cx, TRANSFER)?;
    let e = &mut ctrl.effects[i];
    e.layer = layer.or(e.layer);
    e.life_time = 50;
    e.pos = pos;
    e.offset[2] = height;
    Some(i)
}

/// `MainStr`'s first switch.
pub fn pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    match ctrl.effects[i].id {
        HIT_RING => hit::ring_pre(ctrl, cx, i),
        TRANSFER_RING => arrival::ring_pre(ctrl, cx, i),
        _ => Next::Draw,
    }
}

/// `MainStr`'s second chain.
pub fn post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    match ctrl.effects[i].id {
        HIT_PHOTON => hit::photon_post_str(ctrl, cx, i),
        TRANSFER_RING => arrival::ring_post(ctrl, cx, i),
        TRANSFER => transfer_post(ctrl, cx, i),
        _ => {}
    }
}

/// Effect -2's case (main 0x001d92d0).
fn transfer_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let (cnt, layer, height, pos) = {
        let e = &ctrl.effects[i];
        (e.cnt, e.layer, e.offset[2], e.pos)
    };
    match cnt {
        0 | 3 | 6 => {
            let h = ee::add(0x41a0_0000, height);
            let Some(k) = ctrl.new_effect(cx, TRANSFER_RING) else { return };
            let turn = ee::deg2rad((cx.host.rand() & 0x3f00) as i16);
            let e = &mut ctrl.effects[k];
            if layer.is_some() {
                e.layer = layer;
            }
            e.life_time = 40;
            e.pos_ptr = Some(VecRef::EffectPos(i));
            e.offset[2] = h;
            e.scale = [0, 0, 0, ONE];
            e.rot = [0, 0, turn, ONE];
        }
        10 => {
            let mut g = cx.particles.generator(cx.assets, arrival::GENERATOR_ROW);
            let mut at = pos;
            at[2] = ee::add(at[2], height);
            g.pos = at;
            g.layer = layer;
            // +0x04's strFlag (bits 6-7) 1.
            g.str_flag = 1;
            cx.particles.start(g);
        }
        _ => {}
    }
}
