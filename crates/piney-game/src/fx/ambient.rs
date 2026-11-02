//! A field's weather and ambient pictures as the effects draw them
//! (`piney_world::field::ambient`): each `ccEff` of `WORLD::Draw`, `DrawEffect`
//! and a dungeon's `DrawEff` through the one sprite cache ([`super::sprites`]),
//! and the steam's and fires' `effSmoke` puffs. The game calls `effSmoke` from
//! `WORLD::Draw` (96) before `ccThParticle` (98); here a puff starts at the next
//! frame's first effect task, a frame later, its `rand()` draws after that
//! task's shows'.

use piney_effect::Effects;
use piney_effect::draw::Camera as FxCamera;
use piney_world::field::ambient::Op;

use super::{AreaFx, BattleHost};

/// The frame's sprites on their layers, and with `fresh` its smoke puffs
/// queued for the next effect task.
pub(super) fn draw(
    fx: &mut AreaFx,
    ops: &[Op],
    fresh: bool,
    camera: &piney_world::camera::Camera,
    ctx: &mut piney_desktop::anm::Ctx,
) {
    let cam = FxCamera::from_field(camera).fogged(fx.fog);
    for op in ops {
        match op {
            Op::Sprite(s) => fx.sprites.draw_one(&mut fx.fx.assets, &fx.archive, s, &cam, &mut ctx.layers),
            Op::Smoke { .. } if fresh => fx.ambient_smoke.push(op.clone()),
            _ => {}
        }
    }
}

/// `effSmoke(pos, v, s, life, t, in, out)` for each queued puff, in order.
pub(super) fn start_smoke(fx: &mut Effects, host: &mut BattleHost, queued: &mut Vec<Op>) {
    for op in std::mem::take(queued) {
        if let Op::Smoke { pos, v, s, life, t, fade_in, fade_out } = op {
            fx.smoke(host, pos, v, s, life, t, fade_in as i16, fade_out as i16);
        }
    }
}
