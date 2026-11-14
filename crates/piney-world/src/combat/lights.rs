//! The lights `ccThEntryCtrl`'s objects hold in the characters' light
//! group, and the order its walk lit its own draws in. `ccModel::Draw`
//! (INF main 0x0013eab0) lights a model through `SetLightMatrix`
//! (0x0013ec14) when the draw is made, so each object of the walk is lit by
//! the group as the objects before it left it: an enemy draws in
//! `dispEnemy` (gcmn 0x00432f34) before its `exclusive()` moves its
//! weapon's flashes (0x00432f48); the enemies' list is walked before the
//! circles', the gimmicks' and the NPCs' (0x00431970).

use std::collections::BTreeMap;

use piney_battle::prim::OmniLight;

use crate::ee::{F, V4};

/// The gimmicks' lights in the group (`ccLightGrp::AddGrp` / `DelGrp`), by
/// scene index, as their last frame left them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GimmickLights {
    /// The symbols' omni lights (`ccGimSymbol` +0x704): place and
    /// intensity.
    pub symbols: BTreeMap<usize, (V4, F)>,
    /// The spring's two omni lights (`ccGimEtc` +0x260).
    pub springs: BTreeMap<usize, [OmniLight; 2]>,
    /// The rays' omni lights (`ccPrimRadiate` +0x58: the boxes', the
    /// idols', the radiator's), as their last `setLight` left them.
    pub rays: BTreeMap<usize, OmniLight>,
}

/// What the last walk of the entry control's lists lit its draws by.
#[derive(Clone, Debug, Default)]
pub struct WalkLights {
    /// The enemies' list as the walk took it: each enemy is lit by the
    /// weapons' flashes of the enemies before it as this frame left them.
    pub enemies: Vec<usize>,
    /// Each circle's and gimmick's draw: the gimmicks' lights as the walk
    /// had left them at it (the objects before it this frame's, its own as
    /// its frame had set them so far, the rest the last frame's).
    pub drawn: BTreeMap<usize, GimmickLights>,
}

impl WalkLights {
    /// The enemies the walk ran before enemy `who`.
    pub fn enemies_before(&self, who: usize) -> &[usize] {
        let n = self.enemies.iter().position(|&e| e == who).unwrap_or(self.enemies.len());
        &self.enemies[..n]
    }
}
