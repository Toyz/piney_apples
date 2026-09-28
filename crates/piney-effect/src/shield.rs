//! The resistant shield (gcmn effect2.cpp): `effResistantShield(ch,
//! magic, n)` (gcmn 0x00501ab0), which `ccEnemy::affectEnemy` and the
//! bosses' `Affect` call when a blow meets an immunity (Exdefense), and the
//! element it makes, `ccResistantShieldElement` (0x1a0 bytes; Main gcmn
//! 0x00501970), run by `ccEffectElementManager`.
//!
//! ```text
//! effResistantShield(ch, magic, n)
//!   an enemy (base->type & 0x60): the size s by n, or
//!     ccCheckObjectSize(ch) when n < 0: 1 -> 7, 3 -> 4, 4 -> 2, else 0;
//!     scale (s/2, s/2, s/2, 1)
//!   a boss (& 0x80): s by n itself (no size asked); s > 0: (s, s, s, 1),
//!     else (width, width, height / 2, 1) * 0.01, all four lanes
//!   anyone else: nothing
//!   effResistantShield(ch, magic, scale) (gcmn 0x00501cb0):
//! effResistantShield(ch, magic, scale)
//!   ch off the lists or down (condition.dead not 0 or 1), or its
//!     affectPerson (+0x98) on the lists and down: nothing
//!   new ccResistantShieldElement: the element's defaults; m_anmShield a
//!     ccAnm of particle's ANM_x070 (magic 0 or below) or ANM_x069;
//!     m_targetChar ch; into the manager's first empty slot (none: deleted,
//!     nothing more)
//!   m_offset (0, -width / 2, height / 2, 1), m_scale scale, m_dirc (0, 0,
//!   ccGetDirc(ch->posP, affectPerson->posP), 1); ccSeOn3D(99, ch's pos)
//! Main (0x00501970)
//!   the target off the lists or down: m_delFlag; nothing drawn
//!   m_pos = ccTransPosP2W(target->posP + RotMatrix(m_dirc) m_offset)
//!   the animation a step (its end: m_delFlag after the draw); its matrix
//!   SetMatrix_PosRotZYXScale(m_pos, m_dirc, m_scale); drawn (its
//!   transparency the ccCoord constructor's 1) on the effect layer
//! ```
//!
//! `ccGetDirc(a, b)` (main 0x001d9ce0) is the heading from `a` to `b`:
//! `atan2f(b.y - a.y, b.x - a.x) + pi / 2`, brought into -pi..pi.

use piney_world::pose::Play;

use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4};
use crate::effect::EFFECT_LAYER;
use crate::element::{Base, Element};
use crate::files::ObjRef;
use crate::{CharRef, Cx, Event, space, vu};

/// `ccSeOn3D(99, ch's pos)`.
pub const SE_SHIELD: i32 = 99;
/// The animations: magic, and the rest.
pub const ANM_MAGIC: &str = "ANM_x069";
pub const ANM_PHYSICAL: &str = "ANM_x070";

const HALF: F = 0x3f00_0000;
/// 0.01.
const HUNDREDTH: F = 0x3c23_d70a;

/// `ccResistantShieldElement` (0x1a0 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct ResistantShield {
    pub base: Base,
    /// +0x190 `m_anmShield`: its object and playback.
    pub anm: Option<(ObjRef, Play)>,
    /// +0x194 `m_targetChar`.
    pub target: CharRef,
}

/// `ccGetDirc(a, b)` (main 0x001d9ce0).
pub fn get_dirc(a: V4, b: V4) -> F {
    const PI: F = 0x4049_0fdb;
    const HALF_PI: F = 0x3fc9_0fdb;
    const TWO_PI: F = 0x40c9_0fdb;
    const NEG_PI: F = 0xc049_0fdb;
    let x = ee::sub(b[0], a[0]);
    let y = ee::sub(b[1], a[1]);
    let mut r = ee::add(HALF_PI, ee::atan2f(y, x));
    if !ee::le(r, PI) {
        r = ee::sub(r, TWO_PI);
    }
    if ee::lt(r, NEG_PI) {
        r = ee::add(r, TWO_PI);
    }
    r
}

/// The shield's size by `n`: 1 -> 7, 3 -> 4, 4 -> 2, else 0.
fn size_of(n: i32) -> F {
    match n {
        1 => 0x40e0_0000,
        3 => 0x4080_0000,
        4 => 0x4000_0000,
        _ => 0,
    }
}

/// `effResistantShield(ch, magic, n)` (gcmn 0x00501ab0): the element's
/// slot in the manager.
pub fn eff_resistant_shield(cx: &mut Cx, ch: CharRef, magic: i32, n: i32) -> Option<usize> {
    let t = cx.host.char_type(ch);
    let scale = if t & 0x60 != 0 {
        let n = if n < 0 { cx.host.object_size(ch) } else { n };
        let v = ee::mul(HALF, size_of(n));
        [v, v, v, ONE]
    } else if t & 0x80 != 0 {
        let s = size_of(n);
        if !ee::le(s, 0) {
            [s, s, s, ONE]
        } else {
            let w = cx.host.char_width(ch);
            let h = ee::mul(HALF, cx.host.char_height(ch));
            ee::vscale([w, w, h, ONE], HUNDREDTH)
        }
    } else {
        return None;
    };
    eff_resistant_shield_scale(cx, ch, magic, scale)
}

/// Standing: on the lists and `condition.dead` 0 or 1.
fn standing(cx: &Cx, c: CharRef) -> bool {
    cx.host.check_target(c) && matches!(cx.host.char_dead(c), 0 | 1)
}

/// `effResistantShield(ch, magic, scale)` (gcmn 0x00501cb0): the element's
/// slot in the manager.
pub fn eff_resistant_shield_scale(cx: &mut Cx, ch: CharRef, magic: i32, scale: V4) -> Option<usize> {
    if !standing(cx, ch) {
        return None;
    }
    let by = cx.host.affect_person(ch);
    if let Some(a) = by
        && cx.host.check_target(a)
        && !matches!(cx.host.char_dead(a), 0 | 1)
    {
        return None;
    }
    let name = if magic <= 0 { ANM_PHYSICAL } else { ANM_MAGIC };
    let anm = cx.assets.find("particle", name).and_then(|o| {
        let anim = cx.assets.files[o.file].anim(name)?;
        Some((o, Play { anim, time: 0, posed: 0, frame_spd: 256 }))
    });
    let mut base = Base::default();
    let w = cx.host.char_width(ch);
    let h = cx.host.char_height(ch);
    base.anim.offset = [0, ee::sub(0, ee::mul(HALF, w)), ee::mul(HALF, h), ONE];
    base.anim.scale = scale;
    // An absent affectPerson is a null pointer the game reads through.
    let from = cx.host.char_pos_p(ch);
    let to = by.map_or([0; 4], |a| cx.host.char_pos_p(a));
    base.anim.dirc = [0, 0, get_dirc(from, to), ONE];
    let e = ResistantShield { base, anm, target: ch };
    let slot = cx.spells.elements.add(Element::Shield(Box::new(e)))?;
    let pos = cx.host.char_pos(ch);
    cx.raise(Event::Sound3d { se: SE_SHIELD, pos });
    Some(slot)
}

impl ResistantShield {
    /// `ccResistantShieldElement::Main` (gcmn 0x00501970).
    pub fn main(&mut self, cx: &mut Cx) {
        let t = self.target;
        if !standing(cx, t) {
            self.base.del_flag = 1;
            return;
        }
        let a = &mut self.base.anim;
        let m = vu::rot_zyx(&vu::UNIT, a.dirc);
        let p = ee::vadd(cx.host.char_pos_p(t), ee::apply(&m, a.offset));
        a.pos = space::p2w(p, cx.host.player_pos(), cx.host.bounds());
        let mut done = false;
        if let Some((obj, play)) = &mut self.anm {
            done = play.forward(&cx.assets.files[obj.file]);
            let matrix = vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale);
            cx.draws.push(DrawRec::Anm { obj: *obj, play: play.clone(), matrix, alpha: ONE, layer: EFFECT_LAYER });
        }
        if done {
            self.base.del_flag = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Host;
    use crate::draw::Camera;
    use crate::ee::k;

    /// A boss (1) struck by Kite (0).
    struct Two;

    impl Host for Two {
        fn rand(&mut self) -> i32 {
            0
        }
        fn player_pos(&self) -> V4 {
            [0, 0, 0, ONE]
        }
        fn camera(&self) -> Camera {
            Camera::default()
        }
        fn char_pos(&self, c: CharRef) -> V4 {
            [0, if c == 1 { k(200.0) } else { 0 }, 0, ONE]
        }
        fn char_dirc(&self, _c: CharRef) -> V4 {
            [0; 4]
        }
        fn char_height(&self, _c: CharRef) -> F {
            k(120.0)
        }
        fn char_width(&self, _c: CharRef) -> F {
            k(40.0)
        }
        fn char_type(&self, c: CharRef) -> i32 {
            if c == 1 { 0x80 } else { 0x7 }
        }
        fn affect_person(&self, c: CharRef) -> Option<CharRef> {
            (c == 1).then_some(0)
        }
    }

    /// The shield: an element facing Kite (a half turn: he is at -y), its
    /// animation drawn each frame until it ends and the element goes.
    #[test]
    fn a_shield_faces_its_striker_and_plays_out() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut host = Two;
        assert_eq!(fx.resistant_shield(&mut host, 0, 1, -1), None);
        let slot = fx.resistant_shield(&mut host, 1, 0, -1).unwrap();
        let Some(Element::Shield(s)) = fx.spells.elements.get(slot) else { panic!() };
        // (width, width, height / 2) * 0.01, and the heading from the foe to Kite.
        assert_eq!(s.base.anim.scale[0], ee::mul(k(40.0), HUNDREDTH));
        assert_eq!(s.base.anim.dirc[2], get_dirc(host.char_pos_p(1), host.char_pos_p(0)));
        assert_eq!(s.anm.as_ref().map(|(o, _)| fx.assets.name(*o)), Some(ANM_PHYSICAL));
        let mut frames = 0;
        while fx.spells.elements.get(slot).is_some_and(|e| !e.deleted()) && frames < 500 {
            fx.step(&mut host);
            assert!(fx.draws().iter().any(|d| matches!(d, DrawRec::Anm { .. })));
            frames += 1;
        }
        assert!(frames > 1 && frames < 500);
        fx.step(&mut host);
        assert!(fx.spells.elements.get(slot).is_none());
    }
}
