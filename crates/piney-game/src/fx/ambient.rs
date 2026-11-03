//! The smoke puffs an area's `Draw` starts (`effSmoke`: a field's steam
//! and fires, the enemies' breaths). The game calls `effSmoke` from
//! `WORLD::Draw` (96) before `ccThParticle` (98); here a puff starts at the
//! next frame's first effect task, a frame later, its `rand()` draws after
//! that task's shows'.

use piney_effect::Effects;
use piney_world::systems::Smoke;

use super::BattleHost;

/// `effSmoke(pos, v, s, life, t, in, out)` for each queued puff, in order.
pub(super) fn start_smoke(fx: &mut Effects, host: &mut BattleHost, queued: &mut Vec<Smoke>) {
    for Smoke { pos, v, s, life, t, fade_in, fade_out } in std::mem::take(queued) {
        fx.smoke(host, pos, v, s, life, t, fade_in as i16, fade_out as i16);
    }
}
