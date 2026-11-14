---
number: 581
title: One world, leg 4: one ccRand and one Kite in every area
date: 2026-11-03
area: world, engine, test
started: 2026-11-03T20:14:26-07:00
took: 43m
files: crates/piney-world/src/mt.rs, crates/piney-world/src/world.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/combat/town.rs, crates/piney-world/src/combat/stage.rs, crates/piney-battle/src/evparty.rs, crates/piney-world/examples/evchar_probe.rs
---

# 581. One world, leg 4: one ccRand and one Kite in every area

The fourth leg of the relay finishing "One world, one area system"
(`plans/rust-cleanup.md`, section 2), on the branch `one-world` after
[[580]]. A Root Town now has the game's one `ccRand` and the game's one
Kite: he is the battle's character in every area, built by the field's
`ccSPC::Reboot` and run by the battle's tasks, and the town's stand-in, its
mirror and its own party frame are gone. Checked frame by frame against
Infection's code in eemu. Main gets nothing until the last leg's merge.

## One ccRand

`ccRand` (`INF SLUS_202.67:0x001d9a10`) is one global generator; the port
had three types for it (`crate::mt::Mt`, the battle's `Genrand`, the
save's `CcRand`) and in a town two live copies: the walking PCs'
(`TownPcs.mt`, seeded by `Mt::init(rand_count)` when the entries were
placed, also drawn by the Grunties) and `Combat::cc` (the party AI's,
unseeded in a town, and the town map draw's). `ccInitRand` (0x001d9900)
seeds `ccRand` with 4352 and draws it and `ccRandS` in turn `ccSys+0x358`
times; `lastRnd` (main 0x00378aec) is never reset, so it is moved on, not
restarted. Now `Combat::cc` is the one generator everywhere:
`World::init_rand` reseeds it and moves the one `combat.rnds`,
`World::town_core` seeds both as a count of 0 does, and the one `state_out`
writes `combat.rnds` (the town kept `save.rand_s` and lost any `ccRandS`
draw of its party). The walking PCs and the Grunties draw it through
`NpcCtx.cc` and `GruntyCtx.cc`; `FieldNpcs::with_cc`'s lending and `Mt` are
gone, `mt.rs` keeps only the conversions to the save's form. The façade's
`cc`, `state_out` and `with_live_state` went with it.

## One Kite

[[580]] found `ccEvent::GetSpc` (0x001b2bb0) returns the one `ccPlayer`.
The town now builds Kite with `World::reboot` (`Combat::add_kite`, area 0;
`ccPlayer::ccPlayer`'s arrival rules already knew a town) and runs, at his
task's place, what the field runs: `cameraMain` with his heading, the
leavers' ends, `Combat::frame` (`ccThSpc` 48, `ccThPlayer` 49 =
`kite::main`, `ccThFellowNN` 50), then the view's sync. Gone:
`Combat::add_leader`, `mirror_leader`, `town_frame`, `TownParty`,
`TownChars`, `TownFrame`, the town's own Kite arms, `TownCast::reboot` and
its Kite build, and the Cast's `kite_pos`, `kite_pos_p` and
`with_spc_chars` (the world lends the battle's characters itself). The
town's navigation is `Combat::town_nav` (`TownNavData`), which every
`Stage` carries. `Combat::kite_asleep` is `ccSpcSleep(1)` in a town (the
Grunty ride). Writes to Kite go to the battle's character
(`World::put_kite`, `World::set_pause`, the ride's Sleep and Wakeup through
the lent `SpcRef`). He is drawn as his actor with the members, and his
shows become the town's outputs (`TownCast::party_outs`: transfers, steps'
`ccSeOn3D`, the running dust at `Char::feet`, now shared with the field).

The party entries `ccEntryEventMng` places in a town go to the marker's
dummy (`markerEvTbl`; MUT main 0x001cba88: 99 and up stays from Mutation
on): `EvParty::place_entry` is area-aware and `World::ev_party` supplies the
town's markers (it passed none). A town `entry` of Kite before the set-up
is queued for Reboot, as the members' are; `GetSpc(0)` has no one before it.
A leaver's end (`ccThFellowNN`, gcmn 0x0041ed60) drops it off the command
and character lists in every area, as the town did (the field only forgot
it).

Behaviour that moved, by the game's code: `ControlMove` reads the command
target in a town too (the town passed none), `ccSys.count` is the world's
frame count, the ride's slot runs after the battle's tasks.

## How it was checked

`tools/test_evchar_rs.py` runs Kite and Orca through the game's own
functions in eemu (6 tests, 32 scenarios, 10020 frames, 262 instructions:
pc_act, pc_mode, pc_turn, pc_face, menu_ban, menu_clear, party_add,
party_remove, pad walks) and compares every frame: position, heading,
speeds, flags, acts, cloak, transparencies, animation and time, AI fields,
the camera, the registry, party and character list. `evchar_probe` now runs
the battle's Kite through `Combat::frame`; the harness passes. Its first run
failed on Orca's exit (the probe had lost the leaver's end), which is how the
field's incomplete `fellows_gone` was found. The town tests read Kite's
record (`combat().member_rec(0)`, the `kite` helper in piney-world
tests/world.rs) where the view lacks a field; `markers_and_event_commands`
now builds the town before turning Kite. piney-world 114 + 3 + 1 + 12 pass;
the piney-game suite 347 passed, 0 failed, 152 ignored, as before.

The town golden shots (scratch only) differ from main's on Kite alone: 19
of 50 files, every change inside the box round his body (his pose, blades
and the arms' sparkles), since he is drawn through the actor path. The
ignored `pc_menu_shot` no longer finds a walking PC to talk to: it chases PC
39, which walks away, its path moved with the one `ccRand`.

**Still unknown:**
- whether the town shots' new Kite (the actor's draw) or main's (`Kite::draw` with the town Player's `posed`) is the right picture; evchar says the pose state is the game's
- `world.player` is still a one-way view of Kite (`sync_player`); its readers and the town's second port of `ccPlayer::Main` (`Player::main`, `control_move`, `ai::brains`, `motion::anim_ctrl`, now driven only by seven probes) are to go
- the town's Grunty ride and the field's (`town/ride.rs` against `combat/ride.rs`) are two ports of `ccPucciguso`
- one addressing scheme (W5), the party instructions (`TownCast::pc_command` against `World::pc_command`: EvParty's GetNpc does not see a town's NPCs), one frame, one mode, one effect host
- `pc_menu_shot`'s scenario
