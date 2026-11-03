---
number: 576
title: A town's characters' effects and sounds are taken in their frame, as a field's
date: 2026-11-03
area: world, engine
files: crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/kept_dummies.rs, docs/engine/field-game.md
started: 2026-11-03T15:39:07-07:00
took: 56m
---

# 576. A town's characters' effects and sounds are taken in their frame, as a field's

The third behaviour difference between the town's mode and the field's
(`plans/rust-cleanup.md`, "One world, one area system"): the town's mode
took what its characters' tasks had started (the walking PCs', dogs',
Grunties' and the town's own effects, sounds and chat lines, the party's
lines, and `ccSoundMain`'s scene sound) only from the tasks' second frame
(`Phase::Play(f)` with `f >= 2`) and only while the scripts had not left the
scene; the field's mode takes the battle's shows, sounds and lines every
frame, and only its minimap waits for the second frame. The game's code
sides with the field.

## The evidence

The characters call the effects and sounds themselves, in their own tasks,
from the first frame those run, with no test of the frame count or of the
event task: `effTransfer` (INF SLUS_202.67:0x001ce020) is called from
`ccRtownPC::normalMode` (INF gcmn 0x005085ec, 0x00508688) and `eventMode`
(0x00508a60) under `ccThEntryCtrl` (64), from `ccMerchan::sysopeAct`
(0x005062f0, 0x00506430), `ccFellow::Action` (0x0041df20, 0x0041e074) and
`ccPlayer::AnimCtrl` (0x0059a7f4, 0x0059a900); `ccThEffect` (80) runs after
them in the same frame and starts what they made. Kite's own transfers,
footsteps and dust were already taken every frame in the town. The second
frame is `ccThFieldDisp`'s: `ROOTTOWN0n::Draw` and its `DrawMap` first run
then, which the town and the field already agreed on for the minimap.

The scripts' leave (`ChangeRequest(6, 7)`) puts every task to sleep before
the world's tasks run that frame, so nothing new is made after it; the
gate's half for it held back only what was already taken.

## The port

`WorldMode::step` takes the characters' events, the town's own, the scene
sound and the party's lines in every frame; the minimap alone keeps the
`f >= 2` and not-left gate. Nothing in a covered scene starts an effect,
sound or line on the tasks' first frame or after the leave: the golden
shots (4548, none differ) and the piney-game suite hold their values, and
the two new tests pass on the old code too. They hold the rule from here:
`a_towns_characters_effects_and_sounds_are_taken_in_their_frame`
(`kept_dummies.rs`: Mutation's five towns from the set-up through 400
frames) and `event_2_s_characters_events_are_taken_in_their_frame`
(`world.rs`: Infection's event 2 to its scene change), each finding no
character's event left waiting after any frame.

## Checked

The town harnesses on the four volumes (20 runs) as the baseline at
b95f324; the golden shots, 4548 files, none differ; the crates' tests and
the piney-game suite (347) pass; clippy and fmt clean; too_many_arguments
142, dead_code 0.

**Still unknown:** nothing
