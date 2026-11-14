//! The smoke puffs an area's `Draw` starts (`effSmoke`: a field's steam
//! and fires, the enemies' breaths; `effSmokeN`: a town's airship). The
//! game calls them from `WORLD::Draw` / `ROOTTOWN0n::Draw` (96); here they
//! start as `ccThParticle` (98) begins, after the shows the tasks between
//! raised and before its step, so its `rand()` draws keep their place.

use piney_effect::Effects;
use piney_world::systems::{MapEvent, Smoke};

use super::BattleHost;

/// `effSmoke(pos, v, s, life, t, in, out)` or `effSmokeN` for each queued
/// puff, in order.
pub(super) fn start_smoke(fx: &mut Effects, host: &mut BattleHost, queued: &mut Vec<MapEvent>) {
    for e in std::mem::take(queued) {
        match e {
            MapEvent::Smoke(Smoke { pos, v, s, life, t, fade_in, fade_out }) => {
                fx.smoke(host, pos, v, s, life, t, fade_in as i16, fade_out as i16);
            }
            MapEvent::SmokeN { pos, v, scale, life, kind, fade } => {
                fx.smoke_n(host, pos, v, scale, life, kind, fade.0, fade.1);
            }
            _ => {}
        }
    }
}
