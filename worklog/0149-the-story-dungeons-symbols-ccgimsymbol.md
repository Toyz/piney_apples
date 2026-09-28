---
number: 149
title: The story dungeons' symbols: ccGimSymbol
date: 2026-09-26
area: battle
files: crates/piney-battle/src/gimmick.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/world.rs, crates/piney-effect/src/gimmick.rs, crates/piney-effect/src/lib.rs, crates/piney-world/src/foe.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/field_ambient.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/fx/ambient.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/field-ui.md
---

# 149. The story dungeons' symbols: ccGimSymbol

A GAPS item the menus work left open. Infection's story dungeons
(event areas 17, 18, 20 and 24; event 17's is the Data Drain lesson's)
have symbols: type-3 kind-4 rows that `EntryGimmick` makes as gimmick row
17. Touching one opens `SymbolMenu` (36), which names a skill and casts
it on Kite. The port made them as plain stand-ins, so the menu named
skill 0 and nothing was cast. There was no fire, no light and no model.

## The game

- **The constructor** (`ccGimSymbol::ccGimSymbol`, gcmn 0x0045a0d0).
  - `trapNum` (+0x7c) is `symbolSkillTbl[ccRand() & 15]` (0x006aa280:
    skills 296-303 and 187-192).
  - An omni light, `ccLight(4, 1)`: white, full to 1000, nothing past
    1000000.
  - Row 17 puts the light 160 above the symbol, turns its body on, and
    plays `ANM_xgsymbol` on `CMP_xgsymbo0` (`XGSYMBOL.CCS`).
  - It makes 40 fires (`initFire`: `ccEff`s of `EFF_x008`) and runs them 20
    frames ahead (`advanceFire`).
  - Row 18 (the lakes') only puts its light 60 up.
- **`symMain`** (0x0045ab00).
  - Every frame until spent, the light goes into the group at 1 +
    `ccRandF(0.2)`.
  - Waiting, a fire lights every other frame. Affect 11 (the menu's close)
    starts the cast.
  - The cast's first frame takes it off the command lists, plays sound
    35 and `effUseSymbol` at the light, and calls
    `ccItemSkillRequest(this, opener, trapNum, 0)`.
  - Each cast frame lights a fire and throws a spark (`invokeEff`:
    `ccParticleExplode` along three `ccRand` turns).
  - After 30 frames the symbol is spent and its light leaves the group.
  - In view, the model and the fires are drawn, the fires on layer 3.
- **A fire** (`ccSymFire::main`, 0x00459e50; `setFire`, 0x0045a600).
  - It is lit at the light, moved by `ccRandF(10)` along a `ccRand`
    heading and `ccRandF(10)` up.
  - It rises 0.8 a frame at scale 1.2, brightening by 3, until its clip
    ends. Then it rises 1.6, darkening by 3, until the clip ends again.
    `fade` turns a byte below 0 into 255.
  - The fires take the draws from `ccRand`, and only while a slot is free,
    so their lifetimes matter to the generator's sequence.

## The port

- **piney-battle.** `gimmick::symbol_new` and `symbol_main` (a `Symbol`
  class beside the box's and the idol's), with the fires.
  - New outputs: `Out::SymbolLight`, `SymbolFire`, `UseSymbol` and
    `SymbolSpark`.
  - The fires' `patNum` comes from a new `World::eff_pat_num` (the
    world reads it from the gimmick's file).
- **piney-world.** It loads row 17's look (the model).
  - `Combat::symbol_lights` puts the light at the head of the group the
    characters are lit by (priority 1).
  - The fires draw through the field's ambient sprite path, which now
    takes a colour (`Sprite::colour`).
- **piney-effect.** `eff_use_symbol` (generators 123 and 124) and
  `Effects::particle_explode`.

`event_17s_symbol_casts_its_skill` starts in event 17's dungeon, in floor
0's room 2, where the symbol stands. The table's skill is set, the light
joins the group and 40 fires burn. Kite put before it opens SymbolMenu,
whose close casts the skill: act 0, 1, then 2 with the light out and
`symbolCount` 1. `symbol_shot` (ignored) takes its picture.

**Still unknown:** row 18's `objMain` (the lakes' symbol) is not ported,
and neither the fires nor the light are compared with the game frame by
frame. The picture from a mid-dungeon start shows the fire column but not
the room, which such starts leave undrawn.
