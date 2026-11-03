//! `DrawEffect`'s smoke over field type 7's fires (`WORLD` +0x358, +0x360,
//! which `SetSpecialObj`, gcmn 0x005aae90 from `WORLD_MAN::EntryGimmick`,
//! finds): each fire brought to the player's side (and kept so); within
//! 6500 of the eye on even frames, one `effSmoke` thrown 10 out along a
//! direction turned by three `ccRand()` angles about x, then y, then z.

use piney_data::Result;

use super::{BuildCx, Hook, MapEvent, MapSystem, Others, Smoke, StepCx};
use crate::ee::{self, ONE, V4};
use crate::field::ambient::Env;

/// The fire nodes `SetSpecialObj` looks for in each chip's animated
/// `FOBJECT`, in its order.
const FIRES: [&str; 2] = ["OBJ_roc1fire", "OBJ_roc3fire"];

/// The fires' places.
#[derive(Clone, Debug)]
pub struct Fires {
    pub places: Vec<V4>,
}

impl Fires {
    /// The fires as `SetSpecialObj` finds them. The game's `EntryGimmick`
    /// is not ported in a field; they are found here as the field is made,
    /// the animations at their first frame.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Fires> {
        let places = cx.field.and_then(|f| f.field).map_or_else(Vec::new, |a| a.object_nodes(&FIRES));
        Ok(Fires { places })
    }
}

impl MapSystem for Fires {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let mut out = Vec::new();
        {
            let env = Env::of(cx);
            for fire in &mut self.places {
                *fire = env.p2w(env.w2p(*fire));
                let d = piney_battle::enemy_ai::get_dist_on(env.volume, env.eye, *fire);
                if !ee::lt(d, 0x45cb_2000) || env.odd {
                    continue;
                }
                out.push(*fire);
            }
        }
        let Some(cc) = cx.cc.as_deref_mut() else { return };
        let mut next = || cc.rand() as u32;
        for fire in out {
            let unit = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
            let a = ee::deg2rad(next() as u16 as i16);
            let mut m = piney_data::anim::rot_x_bits_of(unit, a);
            let b = ee::deg2rad(next() as u16 as i16);
            m = piney_data::anim::rot_y_bits_of(m, b);
            let c = ee::deg2rad(next() as u16 as i16);
            m = piney_data::anim::rot_z_bits_of(m, c);
            let off = ee::apply(&m, [0x4120_0000, 0, 0, ONE]);
            let mut pos = fire;
            for k in 0..3 {
                pos[k] = ee::add(pos[k], off[k]);
            }
            let mut v = ee::apply(&m, [ONE, 0, 0, ONE]);
            v[2] = ee::add(v[2], 0x4020_0000);
            let s = Smoke { pos, v, s: ONE, life: 8, t: 208, fade_in: 512, fade_out: 32 };
            cx.events.push(MapEvent::Smoke(s));
        }
    }
}
