---
number: 574
title: A town's tasks run on while a script's stream plays, as ccEventStream breathes
date: 2026-11-03
area: script, video, world
files: crates/piney-game/src/world.rs, crates/piney-game/src/ev_host.rs, docs/engine/field-game.md
started: 2026-11-03T11:17:56-07:00
took: 3h 29m
---

# 574. A town's tasks run on while a script's stream plays, as ccEventStream breathes

The first of the behaviour differences between the town's mode and the
field's that "One world, one area system" (`plans/rust-cleanup.md`) has to
settle before the two become one: while the scripts' `stream` played, the
town's mode returned straight after the event task (`WorldMode::step`), so
none of the town's tasks ran until the stream ended; the field's mode
(`AreaMode`) runs every task and only shows the stream's frame in place of
its own. The game's code says the field's way is right.

## The evidence

`ccEventStream(num, 1)` (INF SLUS_202.67:0x001b5670, the `stream`
instruction's call at 0x001aa168) starts `ccThExecuteStream` (priority 33)
and then loops on `ccBreathThread(1)` (0x001b56f8, 0x001b571c, 0x001b5994)
until the stream task's parameter is -1: one frame at a time, every other
task scheduled as usual. Nothing in it or in what it starts puts a task to
sleep: no `ccSleepAllThread` in `ccEventStream`, `ccThExecuteStream`
(0x001695a0), `ccRequestLoadStream` (0x00198da0) or
`ccStreamLoadPlay::RequestStrPlay` (0x00198fc0), which starts the stream's
own tasks (`ccThEffectStr` 80, `ccThParticle` 98, the read, gzip and decode
threads) and waits in `WaitEnd`. MUT (0x001cacd0), OUT (0x001c1380) and QUA
(0x001c8a00) are the same: breaths, no sleep. Only the desktop's Audio
screen (`SimplePlayStream`) puts every task asleep for its movie.

So `WorldMode::step` now keeps the event task's stream frame aside, runs
the town's tasks, menus, effects and map as in any frame, and shows the
stream's frame at the end, as `AreaMode` does. A Ryu Book's cover stream
(`ccThBook`'s, stepped by the menu task) is unchanged.

## rand() while both run

The stream plays on its own copy of the game's `rand()` and hands it back
when it ends (`StreamPlayer::rand`); with the tasks running beside it, a
task's draw in that time would be lost. In the game the tasks that draw
(`ccThSpc`, `ccThAISystem` 48) come before the stream's effect tasks (80,
96, 98) in each frame. Measured over the piney-game suite and every golden
shot story (4548 shots: the towns on four volumes, the story maps, fields,
dungeons, the bosses), with the world's `rand()` read each stream frame:
no task drew while a stream played, in a town or elsewhere. The stream's
end now warns if the world's `rand()` moved since the stream took it, so a
scene where both draw shows itself.

## Checked

The town harnesses on the four volumes (`test_town02_rs`, `test_town03_rs`,
`test_grunty_rs`, `test_merchcam_rs`, `test_race_rs`, 20 runs) as the
baseline at b95f324; the golden shots, 4548 files, none differ (the town
streams in them are cutscenes where the tasks change nothing drawn); the
crates' tests and the piney-game suite pass; clippy and fmt clean;
too_many_arguments 142, dead_code 0.

**Still unknown:** nothing
