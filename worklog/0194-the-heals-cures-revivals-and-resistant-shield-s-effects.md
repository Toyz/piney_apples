---
number: 194
title: "The heals', cures', revivals' and resistant shield's effects started in play; the party panels shake and the protect marks show"
date: 2026-09-26
area: battle
files: crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, docs/engine/effects.md
---

# 194. The heals', cures', revivals' and resistant shield's effects started in play; the party panels shake and the protect marks show

The same check as 0193's, on piney-battle's rule events
(`piney_battle::event::Event`), found nine variants read nowhere in
piney-world or piney-game. The combat hands them on as `Show::Rule`, and
piney-game's `fx::event` handled ten of the variants with `_ => {}` for the
rest.

Commit c3e15bb had ported and tested the effects for these events in
piney-effect: `effHealSkill`, `effCure`, `effSanity`, `effResurrect` and
`effResistantShield`, with eemu checks and shots. It also wrote
`effects.md`'s table of which event starts which. The runtime never got
the calls. So in play:

- a heal (Repth, a member's heal, the healing items through
  `ccSkillRecovery`) showed no light;
- an Antidote, a Restorative or a Resurrect showed nothing;
- an Exdefense immunity showed no shield.

**Now `fx::event`:**

- `HealSkill { on, sid }` calls `fx.heal_skill`, `Cure`, `Sanity` and
  `Resurrect` call their effects, and `ResistantShield { on, magic }`
  calls `fx.resistant_shield(.., magic, -1)`.
- `a_heal_shows_its_light`: Kite casts Repth on himself in event 3's
  field; effect 130 (`CMP_x060`) and the controller -19 of the rising
  lights come up.

**The menu's two.** `PanelBure { slot, n }` (`ccMenuCtrl::SetPanelBure`:
a party panel shakes as its member is hit) and `SetProtect { state, on }`
(the protect marks over a character whose protect a hit broke or which
regained it). The field UI had both (`MenuCtrl::set_panel_bure`,
`set_protect`) but no caller. `AreaMode::menu_rules` now reads them from
the frame's shows, the rules inside a Kite's, member's or enemy's output
included, and gives `SetProtect` the character's `CharInfo`.

The sweep found three other effect sets ported but never started:

- `effSkillStart`: `Show::SkillStart` is pushed and read by nobody.
  `kite::Out::SkillStart`, `fellow::Out::SkillStart` and
  `enemy_ai::Out::SkillStart` are dropped.
- `effPhysicalSkillHitShockWave`: `flow::MainOut::shock_waves` is counted
  and never read.
- `ccChar::DispConditionEffect`, and with it `effAbilityUp` and
  `effAbilityDown`.

These are the next worklog's.

**Still unknown:** The panel's shake and the protect marks were not
compared with the game's pictures.
