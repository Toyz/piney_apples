//! The characters' light group as each draw of a frame saw it.
//! `ccModel::Draw` (INF main 0x0013eab0) lights a model through
//! `SetLightMatrix` (0x0013ec14) at the call, so a draw sees each light as
//! the tasks before it left it: the party (49, 50) the last frame's, the
//! entry control's objects (64) the walk's order ([`WalkLights`]), the
//! NPC list the gimmicks', weapons' and the cast's this frame, the boss
//! (`ccThBossEffect` then its own task, both 66) all of them. A frame
//! whose tasks sleep draws as the last awake one did.

use std::borrow::Cow;
use std::collections::BTreeMap;

use piney_battle::prim::OmniLight;
use piney_data::light::Light;

use super::{World, omni_light, symbol_light};
use crate::combat::lights::{GimmickLights, WalkLights};
use crate::town::TownLights;

/// Everything that moves in the group, as one moment of the frame had it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LightSources {
    gimmicks: GimmickLights,
    /// The enemies' weapons' flashes, by (enemy, weapon).
    flashes: BTreeMap<(usize, usize), OmniLight>,
    /// The cast's own (a Root Town's Chaos Gate's), after the map's.
    own: Vec<Light>,
    /// The boss's effects' (`ccBossEffLight`, the spells').
    boss_fx: Vec<Light>,
    /// The boss's own (Macha's light ball).
    boss: Option<OmniLight>,
}

impl LightSources {
    /// The group: the map's lights with these (`ccLightGrp::AddGrp` puts
    /// a priority-1 omni light before the map's, the cast's and the
    /// effects' after).
    pub(crate) fn group<'a>(&self, map: &'a TownLights) -> Cow<'a, TownLights> {
        let g = &self.gimmicks;
        if *self == LightSources::default() {
            return Cow::Borrowed(map);
        }
        let mut l = map.clone();
        for &(p, intensity) in g.symbols.values().rev() {
            l.lights.insert(0, symbol_light(p, intensity));
        }
        // The spring's, the rays' and the flashes': `ccLight(4, 1)`, their
        // colour (`ccSetColor`'s low byte red) and fall-off as their frame
        // left it.
        for o in g.springs.values().flatten().chain(g.rays.values()).chain(self.flashes.values()).rev() {
            l.lights.insert(0, omni_light(o));
        }
        l.lights.extend(self.own.iter().copied());
        l.lights.extend(self.boss_fx.iter().copied());
        if let Some(o) = &self.boss {
            l.lights.insert(0, omni_light(o));
        }
        Cow::Owned(l)
    }

    /// These with the weapons' flashes of the enemies `ran` as `after` has
    /// them.
    fn with_flashes_of(&self, after: &LightSources, ran: &[usize]) -> LightSources {
        let mut s = self.clone();
        s.flashes.retain(|(e, _), _| !ran.contains(e));
        s.flashes.extend(after.flashes.iter().filter(|((e, _), _)| ran.contains(e)).map(|(k, o)| (*k, *o)));
        s
    }
}

/// One frame's draws' lights: the group before its tasks ran and as they
/// left it, and the order the entry control's walk lit its own by.
pub(crate) struct FrameLights<'a> {
    map: &'a TownLights,
    before: &'a LightSources,
    after: LightSources,
    walk: &'a WalkLights,
}

impl<'a> FrameLights<'a> {
    pub(crate) fn new(w: &'a World, before: &'a LightSources) -> FrameLights<'a> {
        FrameLights { map: &w.area.core().lights, before, after: w.light_sources(), walk: &w.combat.walk_lights }
    }

    /// The party's and the ride's (49, 50): before everything that moves
    /// a light.
    pub(crate) fn party(&self) -> Cow<'a, TownLights> {
        self.before.group(self.map)
    }

    /// Enemy `who`'s (`dispEnemy`): the flashes of the enemies the walk
    /// ran before it as they left them, the rest as before the tasks.
    pub(crate) fn enemy(&self, who: usize) -> Cow<'a, TownLights> {
        self.before.with_flashes_of(&self.after, self.walk.enemies_before(who)).group(self.map)
    }

    /// A circle's or a gimmick's (the walk after the enemies): every
    /// weapon's flashes as this frame left them, the gimmicks' as the walk
    /// had them at its draw, the cast's and the boss's as before.
    pub(crate) fn walk_object(&self, who: usize) -> Cow<'a, TownLights> {
        let s = LightSources {
            gimmicks: self.walk.drawn.get(&who).unwrap_or(&self.after.gimmicks).clone(),
            flashes: self.after.flashes.clone(),
            ..self.before.clone()
        };
        s.group(self.map)
    }

    /// The NPC list's (after the walk's other lists): the walk's and the
    /// cast's lights as this frame left them, the boss's as before.
    pub(crate) fn npc_list(&self) -> CastLights<'a> {
        let s = LightSources { boss_fx: self.before.boss_fx.clone(), boss: self.before.boss, ..self.after.clone() };
        CastLights { map: self.map, sources: s }
    }

    /// The boss's (66, after `ccThBossEffect`): as the frame left them.
    pub(crate) fn boss(&self) -> Cow<'a, TownLights> {
        self.after.group(self.map)
    }
}

/// The group a cast's draws on the NPC list see.
pub struct CastLights<'a> {
    map: &'a TownLights,
    sources: LightSources,
}

impl CastLights<'_> {
    pub fn group(&self) -> Cow<'_, TownLights> {
        self.sources.group(self.map)
    }

    /// The group with the cast's own lights as `own` instead (a draw its
    /// own object makes before it moves one of them).
    pub fn with_own(&self, own: &[Light]) -> TownLights {
        LightSources { own: own.to_vec(), ..self.sources.clone() }.group(self.map).into_owned()
    }
}

impl World {
    /// The group's moving parts as they stand now.
    pub(crate) fn light_sources(&self) -> LightSources {
        let mut own = Vec::new();
        self.cast.lights(&mut own);
        LightSources {
            gimmicks: self.combat.gimmick_lights.clone(),
            flashes: self.combat.weapons.lights.clone(),
            own,
            boss_fx: self.fx.lights(),
            boss: self.combat.boss.as_ref().and_then(|r| r.light),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flash(intensity: u32) -> OmniLight {
        OmniLight { intensity, ..OmniLight::default() }
    }

    /// An enemy sees the flashes of the enemies before it on the walk as
    /// this frame moved them (one put out, one moved), its own and later
    /// ones' as the last frame left them.
    #[test]
    fn an_enemy_sees_only_the_flashes_moved_before_it() {
        let walk = WalkLights { enemies: vec![4, 1, 7], ..WalkLights::default() };
        let before = LightSources {
            flashes: [((4, 0), flash(1)), ((4, 1), flash(2)), ((1, 0), flash(3)), ((7, 0), flash(4))].into(),
            ..LightSources::default()
        };
        let after = LightSources {
            flashes: [((4, 0), flash(5)), ((1, 0), flash(6)), ((7, 0), flash(7)), ((7, 1), flash(8))].into(),
            ..LightSources::default()
        };
        let seen = |who| before.with_flashes_of(&after, walk.enemies_before(who)).flashes;
        assert_eq!(seen(4), before.flashes);
        assert_eq!(seen(1), [((4, 0), flash(5)), ((1, 0), flash(3)), ((7, 0), flash(4))].into());
        assert_eq!(seen(7), [((4, 0), flash(5)), ((1, 0), flash(6)), ((7, 0), flash(4))].into());
    }
}
