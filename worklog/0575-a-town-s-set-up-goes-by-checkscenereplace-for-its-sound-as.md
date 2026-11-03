---
number: 575
title: A town's set-up goes by CheckSceneReplace for its sound, as every area's
date: 2026-11-03
area: audio, world
files: crates/piney-world/src/lib.rs, crates/piney-world/src/area/mod.rs, crates/piney-world/src/field/world.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/kept_dummies.rs, docs/engine/field-game.md
started: 2026-11-03T14:46:47-07:00
took: 52m
---

# 575. A town's set-up goes by CheckSceneReplace for its sound, as every area's

The second behaviour difference between the town's mode and the field's
(`plans/rust-cleanup.md`, "One world, one area system"): the town's set-up
always sent `SoundFadeOut` at its start and `AllSoundOff` after its fade,
always asked for its bank (`ccSndSQLoad(2)`), and told `ccSndBgmCtrl` the
scene was never replaced; the field world goes by `ccGame::CheckSceneReplace`
for all of them. The game's `ccSetupGameCtrl` has one path for every area,
so the town now does as the field does.

## The evidence

`ccSetupGameCtrl` (INF SLUS_202.67:0x00168960) tests `CheckSceneReplace`
inline before it does anything else: `game` +0x14 against +0x18 (area),
+0x20 against +0x38 (town), +0x24 against +0x3c (field), +0x28 against
+0x40 (dungeon); `ccSoundFadeOut` (0x00168a34) only when one differs. After
the fade the same test picks `ccAllSoundOff` (0x00168b8c) or
`ccEvVoiceStop` (0x00168b9c). Neither branch reads the area. The town's
`ccSndSQLoad(2)` (0x00169094, on the area-0 path chosen at 0x00168fc4) sits
behind the same four comparisons. MUT (0x00168bb4, 0x00168d0c, 0x00168d1c),
OUT (0x00167e78, 0x00167fe8, 0x00167ff8) and QUA (0x00167d08, 0x00167e78,
0x00167e88) have the same three calls in the same places.

A town set up again without the scene being replaced is an event's
`scene` to the town it is in ([[298]], [[406]]): there the game fades
nothing, stops only the voices and keeps the bank playing.

## The port

`Scene::sound_off` (`area/mod.rs`) is the after-fade choice for every area
(it was the field world's private `sound_off`). The town's `World` now holds
its `Scene` (`World::enter_after` takes it, the session passes its own;
`World::enter` and `WorldMode::enter` make the log-in scene), sends
`SoundFadeOut` only for a replaced scene and `scene.sound_off()` after the
fade and when the session already drew it (`skip_fade_out`). The town's
mode turns `SqLoad` into an event through `piney_audio::setup_context`, as
the area's mode does (area 0 loads only when replaced), and hands
`ccSndBgmCtrl` `scene.changed()`.

`a_town_set_up_again_keeps_its_sound` (session tests, `kept_dummies.rs`)
sends Mutation's Carmina Gadelica to itself and then to Mac Anu: the first
asks only `VoiceStop`; the second `SoundFadeOut`, `AllSoundOff` and a bank,
no `VoiceStop`. It failed before the change.

## Checked

The town, area, party and gate-out harnesses on the four volumes (32 runs)
as the baseline at b95f324; the golden shots, 4548 files, none differ; the
crates' tests and the piney-game suite pass (345 with the new test); clippy
and fmt clean; too_many_arguments 142, dead_code 0.

**Still unknown:** nothing
