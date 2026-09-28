//! `ccRingElement` (gcmn effect2.cpp): a ring clump of particle.ccs,
//! re-coloured by mode, that grows and fades; `effSummonRingElement` (gcmn
//! 0x004ffe20) starts one in the element manager. Every spell family's
//! shock rings are these.
//!
//! ```text
//! effSummonRingElement(pos, dirc, type)
//!   new ccRingElement(pos, dirc, (1, 1, 1, 1), type), EnyFlg 1, into the
//!   first empty slot of the manager (none: left alone, never run)
//! ccRingElement(VP, VR, VS, Mode) (0x004fbbe0)
//!   the element's defaults (a ccDrawElement), EnyFlg 0, Flg 0,
//!   Transparency 1, Tpoint 0.05; SetModel(Mode): the clump and CLUT names;
//!   the clump of particle.ccs, fog off, and with a CLUT name duplicated
//!   and re-coloured; Pos VP, Rot VR, Scale VS, Spoint 0.5 in all four lanes
//! ccDrawElement::Main (0x004ea670): m_life counts down to deletion (-1:
//!   never), then Draw
//! Draw (0x004fbf00): with EnyFlg the scale grows by Spoint and the ring
//!   is drawn at Transparency, which falls by Tpoint; below 0 m_delFlag;
//!   the matrix from Pos, Rot (x, y, z) and Scale
//! ```

use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4};
use crate::effect::EFFECT_LAYER;
use crate::element::{Base, Element};
use crate::files::ObjRef;
use crate::{Cx, vu};

/// `ccRingElement` (0x200 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct RingElement {
    pub base: Base,
    /// +0x190 `Pos`, +0x1a0 `Rot`, +0x1b0 `Scale`, +0x1c0 `Spoint`.
    pub pos: V4,
    pub rot: V4,
    pub scale: V4,
    pub spoint: V4,
    /// +0x1d0 `CMPSTR`, +0x1d4 `CLUTSTR`, +0x1d8 `CLUT`: `SetModel`'s names.
    pub cmp_str: Option<&'static str>,
    pub clut_str: Option<&'static str>,
    pub clut_name: Option<&'static str>,
    /// +0x1dc `EnyFlg`, +0x1e0 `Flg`, +0x1e8 `Transparency`, +0x1ec
    /// `Tpoint`.
    pub eny_flg: u8,
    pub flg: i32,
    pub transparency: F,
    pub tpoint: F,
    /// +0x1f4 `EffCmp`, with its CLUT swap (from, to: the objects of
    /// particle.ccs).
    pub eff_cmp: Option<ObjRef>,
    pub clut: Option<(u32, u32)>,
    /// The clump's transparency as last set (`ccClump::Init` leaves 1).
    pub drawn_alpha: F,
}

/// `SetModel(Mode)` (gcmn 0x004fb3f0): the clump, the CLUT it becomes, the
/// CLUT it replaces.
pub fn set_model(mode: i32) -> (Option<&'static str>, Option<&'static str>, Option<&'static str>) {
    const C30: [&str; 7] =
        ["CLT_x030", "CLT_x030c1", "CLT_x030c2", "CLT_x030c3", "CLT_x030c4", "CLT_x030c5", "CLT_x030c6"];
    const C31: [&str; 7] =
        ["CLT_x031", "CLT_x031c1", "CLT_x031c2", "CLT_x031c3", "CLT_x031c4", "CLT_x031c5", "CLT_x031c6"];
    const C33: [&str; 7] =
        ["CLT_x033", "CLT_x033c1", "CLT_x033c2", "CLT_x033c3", "CLT_x033c4", "CLT_x033c5", "CLT_x033c6"];
    const C37: [&str; 7] =
        ["CLT_x037", "CLT_x037c1", "CLT_x037c2", "CLT_x037c3", "CLT_x037c4", "CLT_x037c5", "CLT_x037c6"];
    const C03: [&str; 7] =
        ["CLT_x003", "CLT_x003c1", "CLT_x003c2", "CLT_x003c3", "CLT_x003c4", "CLT_x003c5", "CLT_x003c6"];
    let (cmp, tbl, k): (&'static str, &[&'static str; 7], i32) = match mode {
        32 => ("CMP_x030", &C30, 0),
        33 => ("CMP_x031", &C31, 0),
        34 => return (Some("CMP_x032"), None, None),
        35 => ("CMP_x033", &C33, 0),
        42 => ("CMP_x037", &C37, 0),
        43 => ("CMP_x038", &C03, 0),
        161..=167 => ("CMP_x030", &C30, mode - 161),
        168..=174 => ("CMP_x031", &C31, mode - 168),
        175..=181 => ("CMP_x033", &C33, mode - 175),
        190..=196 => ("CMP_x037", &C37, mode - 190),
        197..=203 => ("CMP_x038", &C03, mode - 197),
        _ => return (None, None, None),
    };
    (Some(cmp), Some(tbl[k as usize]), Some(tbl[0]))
}

impl RingElement {
    /// `new ccRingElement(VP, VR, VS, Mode)` (gcmn 0x004fbbe0).
    pub fn new(cx: &Cx, vp: V4, vr: V4, vs: V4, mode: i32) -> RingElement {
        let (cmp_str, clut_str, clut_name) = set_model(mode);
        let find = |n: Option<&str>| n.and_then(|n| cx.assets.find("particle", n));
        let eff_cmp = find(cmp_str);
        let clut = if clut_str.is_some() {
            find(clut_name).zip(find(clut_str)).map(|(f, t)| (f.object, t.object))
        } else {
            None
        };
        RingElement {
            base: Base::default(),
            pos: vp,
            rot: vr,
            scale: vs,
            spoint: [0x3f00_0000; 4],
            cmp_str,
            clut_str,
            clut_name,
            eny_flg: 0,
            flg: 0,
            transparency: ONE,
            tpoint: 0x3d4c_cccd,
            eff_cmp,
            clut,
            drawn_alpha: ONE,
        }
    }

    /// `ccDrawElement::Main` (gcmn 0x004ea670), then `Draw` (0x004fbf00).
    pub fn main(&mut self, cx: &mut Cx) {
        let life = self.base.life;
        if life >= 0 {
            self.base.life = life - 1;
            if life == 0 {
                self.base.del_flag = 1;
            }
        }
        if self.eny_flg != 0 {
            let t = self.transparency;
            self.scale = ee::vadd(self.scale, self.spoint);
            self.drawn_alpha = t;
            let t = ee::sub(t, self.tpoint);
            if ee::lt(t, 0) {
                self.base.del_flag = 1;
            }
            self.transparency = t;
        }
        let matrix = vu::pos_rot_xyz_scale(self.pos, self.rot, self.scale);
        if let Some(obj) = self.eff_cmp {
            cx.draws.push(DrawRec::Clump {
                obj,
                matrix,
                alpha: self.drawn_alpha,
                layer: EFFECT_LAYER,
                clut: self.clut,
            });
        }
    }
}

/// `effSummonRingElement(pos, dirc, type)` (gcmn 0x004ffe20).
pub fn eff_summon_ring_element(cx: &mut Cx, pos: V4, dirc: V4, ty: i32) -> Option<usize> {
    let mut r = RingElement::new(cx, pos, dirc, [ONE; 4], ty);
    r.eny_flg = 1;
    cx.spells.elements.add(Element::Ring(Box::new(r)))
}
