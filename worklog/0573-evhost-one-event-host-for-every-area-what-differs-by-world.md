---
number: 573
title: EvHost: one event host for every area, what differs by world its HostWorld
date: 2026-11-03
area: script, build
files: crates/piney-game/src/area.rs, crates/piney-game/src/ev_host.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/main.rs, crates/piney-game/src/world.rs, docs/engine/evarea.md, docs/engine/event-vm.md, docs/engine/field-game.md, docs/engine/field-walk.md, docs/engine/stfroll.md, plans/rust-cleanup.md
started: 2026-11-03T08:58:25-07:00
took: 2h 18m
---

# 573. EvHost: one event host for every area, what differs by world its HostWorld

Section 2's `EvHost` item (`plans/rust-cleanup.md`), the second step of
"One world, one area system": the event scripts had two hosts,
`field_host.rs`'s `FieldHost` over the town's world and `area_host.rs`'s
`AreaHost` over the field world, each a whole `impl Host` (29 methods the
same line for line, the rest differing in what they asked of their world),
and two stream holders (`TownStream` in `FieldState`, `AreaStream` in the
area mode). Now `ev_host.rs` has one `impl Host for EvHost<W: HostWorld>`
and one `EvStream`; what differs between the worlds is each world's
`HostWorld` (in `world.rs` for the town's, `area.rs` for the field's).
`field_host.rs` keeps what the host holds besides the world (`FieldState`,
`ScFade`, `CcGame`, now with `game_of`); `area_host.rs` is gone.

## What a world answers

`HostWorld` is the world's side of the instructions, with no area's name:

- the save and its copy out, `ccGame` as the scripts read it (the town's
  own `ccGame`; an area's scene with the scripts' `cntStop`), the change of
  area and of scene (the town's goes to the session and moves its
  `ccGame` and `lastTown` at once; an area's `ChangeArea` is its world's,
  its `ChangeScene` the session's), the stream's `ccGame` and file list,
  the stream's hand-back of `rand()`, `game.server`, the first play pass's
  menu noise (`hackFlag` 3: the crisis byte in a town, `WORLD_MAN.hack`
  elsewhere);
- the characters (`party`, where one stands, `charTbl` names, markers, the
  command target and `ccCheckTarget`, `near_marker`'s distance, the event
  camera, `pc_*`, `npc_*`, `trans_*`, Piros's tint, `menu_ban`'s camera
  and party part, `party_add` / `party_remove`, `remove`, `AddSpcItem`'s
  view of a party character);
- with defaults that are a town's answers, since its entry control has no
  enemies, portals, boss or rooms: `compulsionGameOver`, `ccClearGtHack`,
  `spcConditionEffectFlag`, `entry_present`, `boss`, `no_active`,
  `no_entries`, `remove_trap`, the gimmicks, `room`, `ShowMap` (1 at once,
  as `WORLD_MAN::ShowMap` in a town), the event positions, `enemy_put`,
  `hold`, `battle_ready` and the waits `player_skill`, `gate_hack_anim`,
  `ending_kanji`. The field world answers all of them from the battle.

`teach_camera1..3`'s pad test is the host's (`teach_input`), handed the
pad's answers by the mode that has them (the area's); the town's mode hands
none and the parts answer at once, as before.

## What changed in what the scripts see

Nothing in the game state: the harnesses and shots are the baseline's. In
the call log, which tests read by prefix: a town's `trans` that finds no
one says `(not done)` as the area's did (was `(not placed)`); a stream that
ends logs `stream done` in a town too; a town's `room`, `enemy_put` and
`show_map` now log as an area's (they did nothing there, and still do). The
town's host no longer runs `piney-event`'s defaults for the battle's
conditions and instructions (`entry_present`, `boss`, `no_active`,
`no_entries`, `teach_input`, `camera_type`, `remove_trap_done`, `gimmick`,
`show_map`, `room`, `hold`, `enemy_put`, `begin` / `end` / `busy` of the
battle's waits), so they no longer count as unported there; their answers
are the defaults' values.

## Checked

The full harness on the four volumes (60 runs, every town, story-map,
field, dungeon, party, gate-out and area harness) as the baseline at
75b66cb; the golden shots, 4548 files, none differ; the crates' tests and
the piney-game suite (event 11's and 22's "no unported default ran" among
them) pass; gen --check, clippy, fmt clean; too_many_arguments 142,
dead_code 0.

## Left of the item

The worlds are still two (`piney_world::World`, `FieldWorld`), so
`HostWorld` has two impls; with one world it has one, and `EvHost` stays.

**Still unknown:** nothing
