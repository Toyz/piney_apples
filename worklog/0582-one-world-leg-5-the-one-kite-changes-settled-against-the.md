---
number: 582
title: One world, leg 5: the One Kite changes settled against the game, one ride, Kite's Player view gone
date: 2026-11-03
area: world, engine, test, volumes
files: crates/piney-world/src/combat/ride.rs, crates/piney-world/src/world/ride.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/world.rs, crates/piney-world/src/town/cast.rs, crates/piney-game/src/session/tests/town_shots.rs
started: 2026-11-03T20:59:13-07:00
took: 31m
---

# 582. One world, leg 5: the One Kite changes settled against the game, one ride, Kite's Player view gone

Leg 5 of the relay finishing "One world, one area system" (plans/rust-cleanup.md section 2), on branch `one-world` after [[581]]. It first settled the three behaviour changes [[581]] left unverified, each from the game's code, then made the ride one and took Kite's one-way Player view out of the World. Commits: 8a0e503 (pc_menu_shot), be6671f (one ride), 4a506f8 (the view gone).

## The golden shots: the new Kite is the game's

19 of 50 town images moved with One Kite, every difference on Kite's body. An experiment drew town0-inf (INF) with the draw call varied: fog/blend off, the play's `hidden` cleared, `time` set to `posed`. Every variant gave the branch's pixels exactly, so the draw's treatment is not what moved. The state drawn is: Kite idles on animation 14 (fading in on arrival, alpha 0x3dcccccc up to 1.0), and the idle's phase (`posed`) differs from the old town Player's. The game has one ccPlayer drawn by one `ccChar::Draw` (INF GCMN.PRG:0x0056b1c0) in every area, and the battle's Kite matches the game's animation and time frame by frame in a town (tools/test_evchar_rs.py, OK again at 4a506f8). So the branch's shots are the new goldens; the last leg regenerates them.

On the way: `ccChar::Draw`'s shadow alpha (drawEnv +0xa4) is `setTransparency` (+0x8c) when the near fade hides (sp+108) and `transparency` (+0x88) otherwise (INF GCMN.PRG:0x0056b4e4-0x0056b54c). The town's `Char::fade` does this; every battle draw of an actor (world.rs's actor draws, town/party.rs `draw_members`) uses the actor's alpha for both. They differ when the camera comes within 1.25 r of a character.

## The walking PCs: the shot test's scenario, not the code

All the town harnesses on all four volumes give the baseline's lines (b95f324): test_town02_rs, test_town03_rs, test_grunty_rs, test_merchcam_rs, test_race_rs, test_world_rs (TownPcsAgainstGame: 11970 act-5 frames, 546 pushes by Kite, routes, chats) and test_party_rs. The PCs match the game on the one ccRand, so `pc_menu_shot` chasing walking PC 39 as it walked off was the scenario's fault. It now steers to the nearest PC standing in a chat group (`thinkType` 1) and opens PcMenu (22) on NPC 62 at play 535.

## The ride's slot, and one ride

MUT `ccPuccigusoStart` (SLUS_205.62 GCMN.PRG:0x0052d6f0) starts `ccThPucciguso` at priority 49 in a town as in a field (0x0052d8a0, `li a1, 49`). `ccStartThread` (INF SLUS_202.67:0x00159fd0) appends the new task at `tscbEnd`, so tasks of one priority run in start order: the ride runs after `ccThPlayer` (started at Reboot) and before `ccThFellowNN` (50). That is the field's slot; the town's had run after all the battle's tasks.

MUT's `ccThPucciguso` (0x0052d540) is one task for both areas. Its tcb +0x18 (a town) sets the object's pause bit (+0xe0 bit 0) after the constructor. It runs 30 `Main`s, then one more, then each frame checks the end before `Main`: in a field the pad button, in a town the race over (0x005ff760), both only while `pgRideFlag`. Then it calls `ccPuccigusoExit`, clears `ccSnd` +0x139 and deletes itself.

So town/ride.rs (TownRide, TownStage, its own slot) is gone. Combat's `Riding` runs the ride in every area at the game's slot. In a town the object is paused at its creation; from the 32nd frame the race's end makes `combat::ride::frame` return true, and `ride::exit_town` (MUT 0x0052d960) runs there and wakes Kite. `World::ride_start_town` (MUT 0x0052d6f0) starts it: `pgRideFlag`, `pgR`, `ccSpcSleep(1)` on Kite alone with his condition effect cleared, the kind's file, the task. The race reads `pgRideFlag`.

Two further changes, both the game's. A town now sets `pgRideFlag` (0x0052d764), which the old port never did. `Main`'s write of plw is now seen by its later fades: `ccGetCameraTransparency` goes through `ccTransPosW2P`, and that reads `ccPlayer::W2PPos`. The ride shots are unchanged by both. flag_race_shots differ from base only as [[581]]'s One Kite shots do; breeder_race_shot and ride_shots are as base.

## The Player view

`world.player`, `sync_player` and `World::player()` are gone. Readers use `World::kite_pos`, `kite_dirc`, `kite_act`, `kite_paused`, `kite_drawn`, `kite_listed`, `kite_moving`, `kite_size`, `kite_spc` and `kite_char`, all on the battle's Kite. Before Reboot builds him, `kite_pos` and `kite_dirc` give the start. One bug went with the view: the façade's `item_step` Pause wrote only the view, and now sets his pause. The arrival tests (tests/world.rs, tests/dun_loireag.rs) now see Kite built on the frame after Play(0) and arriving there.

## State

At 4a506f8, piney-world (release) passes, and piney-game gives 347 passed, 0 failed, 152 ignored, as the baseline. The town shot subset (50 files) is identical to [[581]]'s k1, and evchar is OK. The full checks (every harness and shot, gen --check, clippy) were not run. The scratch notes are /mnt/data/claude/scratch/world3/HANDOFF.md, whose "Next, in order (after leg 5)" carries what follows.

**Still unknown:**
- the seven probes (world_probe, story_map_probe, merchcam_probe, field_probe, evcam_probe, dungeon_probe, bosscam_probe) still drive the town's second port of ccPlayer::Main through `piney_world::tasks`; moved onto `Combat::frame`, does each harness still pass, so that `Player::main` and its movement and animation code can go
- one shadow rule for every `ccChar::Draw` port (hidden: `setTransparency`), and which golden shots it moves
- W5 addressing (GetSpc / GetNpc / GetEnemy by type and code), EvParty finding a town's NPCs and one `pc_command`; one frame, mode and HostWorld; the façade's 53 methods and its Deref; W4 one effect host (a town drops `Show::Rule`, the ride's ClearConditionEffect on Kite among them)
- the full checks on the branch, and the regenerated golden shots
