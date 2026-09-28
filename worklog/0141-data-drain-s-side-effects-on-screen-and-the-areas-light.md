---
number: 141
title: "Data Drain's side effects on screen, and the areas' light group checked"
date: 2026-09-26
area: render, test
files: crates/piney-battle/src/drain.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session/tests/skeith.rs, crates/piney-world/examples/world_probe.rs, tools/test_battle_drain_rs.py, tools/test_lights_rs.py, docs/engine/field-ui.md, docs/engine/field-game.md, docs/engine/effects.md, docs/engine/battle.md
---

# 141. Data Drain's side effects on screen, and the areas' light group checked

Data Drain's side effects were rules only. The party lost HP or exp, or
took a condition, but the screen showed only the heals. This entry starts
everything `DataDrainMenu` starts around a side effect. It then answers
the question [[136]] left: whether the towns' and fields' light groups
have the order the streams' needed.

## What step 10 starts

`DataDrainMenu` step 10 (gcmn 0x005337a4-0x0053476c) starts these in
order, each character's in turn (Kite, or the party's living members):

- **Effect 0.** `effHeal(ch, 1)` on each.
- **1-6** (Kite's stats down). `effSkillStartEffect(Kite, 0, 1)`.
- **7-20** (the conditions). `ccEntryFlyFontNewMiss(pos, ch)` for a
  resisted one, then `effSkillStartEffect(ch, 0, 1)` either way.
- **21, 22, 28** (HP, SP halved; 1 HP and SP).
  `effSkillStartEffect(ch, 0, 1)` on each.
- **23-27** (exp lost). `ccEntryFlyFontNewExp(23, -loss, pos, Kite)`,
  then `effAfterDrain(Kite, 0)`.
- **Step 11's frame 30.** When the loss takes a level,
  `ccEntryFlyFontNewLevelDown(23, pos, plw)`.

The world runs through step 11, so all of this plays before the window
opens and the world sleeps.

## The port

- **The rules.** `piney_battle::drain::SideEffect` keeps `starts`, a list
  of `SideStart` in the game's order: `Heal`, `Miss`, `Effect`, `Exp` and
  `AfterDrain`. It replaces the three separate lists. `missed()`,
  `shown()` and `healed()` still give them.
- **The world.** `Combat::drain_side_effect` turns each start into a
  `Show::DrainSide`. `kite_level_down` adds `Show::DrainLevelDown`.
- **The effects.** piney-game's `fx.rs` starts each through the effects
  already ported: `fx.heal`, `fly_font_miss`, `skill_start_effect`,
  `fly_font_exp`, `after_drain` and `fly_font_level_down`.

## Checked

- **The order.** `tools/test_battle_drain_rs.py`'s side-effect check now
  also compares the starts in the order the game calls them, with their
  arguments. 3000 random cases match.
  - Swapping the exp's number and the wave in the port fails 68 of 1000.
- **The screen.** piney-game's `drain_side_effects_show` drains Skeith
  through the menus with each kind of side effect forced.
  - The menu's roll is made to go to a side effect.
    `Combat::force_side_draws` gives the effect's two draws.
  - The effects' census (`FieldFx::census`) shows each start during step
    11: `effSkillStartEffect`'s ring -13 and rings 82-85, the heal's -19
    and 130, the wave 12, and the words MISS, -600EXP and LEVEL DOWN.
  - `drain_side_effect_resisted_shows_miss` checks the MISS over Kite. His
    body tolerance in the arena is 40, so a roll of 0 is resisted.
- **Shots.** `drain_side_effect_shots` (ignored; `PINEY_SHOTS`) writes
  frames of step 11 for effects 0, 1, 9 (resisted), 16, 21, 25 (with a
  level lost), 28 and 29.
  - A first run drew a MISS into effect 16's frames. The drain's noise
    reads the last frame drawn, and the run had drawn only some frames,
    so the last one was the previous drain's.
  - The shots now use one renderer per drain, fed every frame from step
    10 on.

## The light group

- **The game's rules.**
  - `ccCreateLight` (0x001389b0) gives a distant light priority -1 and
    the other kinds 0.
  - `ccLightGrp::AddGrp` (0x00139060) puts a light after the last one of
    greater or equal priority.
  - `ccDrawEnv::SetLightMatrix` (0x00105900) keeps three (priority,
    slot) records. A light above the lowest record's priority is asked
    `CheckRange`, and on an answer takes its place by priority and the
    pushed-out record's slot.
  - In a descending group the first three that answer take slots 2, 1
    and 0.
- **The port already had it.** `chara::light_matrix` sorts the lights
  stably by descending priority. Every area builds its lights with
  `ccCreateLight`'s priorities: the towns, fields, dungeons, event areas,
  the gate's omni and the boss's `ccLight(4, 1)`. So the towns and fields
  were right, unlike the streams in [[136]].
- **The check.** It had never been compared with the game.
  `tools/test_lights_rs.py` builds groups with the game's own
  `ccCreateLight` and `AddGrp` in eemu, then runs `SetLightMatrix` at a
  point. world_probe's new `lights` answers the same group through
  `light_matrix`.
  - 5000 random groups of distant, omni and boss lights, some out of
    reach or dark, match in slots, directions and colours.
  - With the port's sort taken out, 131 of 400 fail.
  - A town's order (the distant light named first) comes out with the
    omni lights in all three slots.
- **A trap.** The ctor sets `matCalcSW`, so `CheckRange` first copies a
  light's `lwMatrix` over its matrix. The check turns it off to hold the
  position and direction it sets.

**Still unknown:**
- **The side effects' timing.** The game starts step 10's effects inside
  the menu task. The port starts them at the field's next effect task, so
  their `rand()` draws come a frame later.
- **Their look.** The effects' pixels are not compared with the game's
  pictures.
- **The lights' values.** The areas' light positions and colours at frame
  1 of their animations are not checked against the game, only the group
  logic.
- **`ccDrawEnv` +0x88.** `SetLightMatrix` walks a second group there
  first, when there is one. What fills it is not traced, and the port has
  none.
