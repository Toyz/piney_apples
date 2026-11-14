---
number: 584
title: One world, leg 7: the gimmicks' near fade, the NPC list at the entry control's slot, one effect host
date: 2026-11-04
area: world, render
files: crates/piney-world/src/combat/mod.rs, crates/piney-world/src/world.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/world/cast.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs
started: 2026-11-03T23:42:31-07:00
took: 55m
---

# 584. One world, leg 7: the gimmicks' near fade, the NPC list at the entry control's slot, one effect host

Leg 7 of the relay finishing "One world, one area system" (plans/rust-cleanup.md section 2), on branch `one-world` after [[583]]. It settled leg 6's open question about the gimmicks' draws from the game's code, moved the area's NPC list to the entry control's slot in the frame, and gave a town the field's effect host. Commits: 1a1c34d (gimmick draws), 0d87d81 (the NPC list's slot), cef6f41 (one effect host).

## The gimmicks' draws go through ccChar::Draw

Leg 6 left open whether the Grunty foods and the gimmicks pass through `ccChar::Draw`'s near fade. They do. `ccGimFood::main` (INF GCMN.PRG:0x00458dc4), `ccGimIdol::main` (0x00459d2c) and `ccGimSymbol::symMain` (0x0045ae48) each call `ccChar::Draw` (0x0056b1c0) on their own `ccChar`; Mutation, Outbreak and Quarantine do the same (MUT 0x0046e764, 0x0046f6cc, 0x004707e8; OUT 0x0046a094, 0x0046af7c, 0x0046c0b0; QUA 0x0035c9a4, 0x0035d88c, 0x0035e9c0). `transDist` (+0x90) is the 1 that `ccChar::ccChar` sets (0x0056af7c), and neither the `ccEntryObj` nor the `ccGimmick` constructor changes it. The size comes from the gimmick's parameter row through `SetBaseParam`. So the idol, the symbol and the food fade as Kite does when the camera comes close, and their shadow follows the same rule. They now go through `Stage::char_draw` with the entry's `setTransparency` (+0x8c), and write the result back to its `transparency` (+0x88) as the game's draw does. Before this, the idol and the symbol were drawn at 1.0 and the food at +0x88, with no near fade. `ccGimBox` calls `ccAnm::Draw` itself and keeps its own fade.

## The NPC list at the entry control's slot

`Combat::frame` is now two halves, `frame_to_entry` (from 48 through `ccThEntryCtrl`'s walk of its lists and what that walk raised) and `frame_from_entry` (66 to 98). `World::combat_frame` runs `Cast::entry_slot` between them. A field's event NPCs step there and push their starts into the battle's shows, so this frame's `ccThEffect` takes them; before, they waited a frame. A town's race task (63) and its list (gate, flags, merchants, dogs, Grunties, walking PCs, event NPCs) step there too; the battle's own walk is empty in a town, so running the race after it changes nothing. `TownCast::frame` keeps only the draws. The party is drawn under the gate's lights as the gate's last step left them, as at 49 and 50. The menu's fader the race fades on goes down through `World::step_into_with`.

## One effect host

piney-game's town mode installs `AreaFx` as the field's mode does, and town_fx.rs is gone. A town's own characters' outputs travel with the battle's shows as `Show::Cast`. The merchants' `sysopeAct` goes as a field event NPC's does, as `Show::Npc` on its handle. `AreaFx` starts all of them in `ccThEffect` and reaches a town NPC by handle through `FxWorld::npcs`. Kite's and the members' shows are no longer dropped in a town, so `Show::Rule` reaches the effects; that includes the ride's `ClearConditionEffect` on Kite. The town mode reads the shows in order as the field's mode does, through area.rs's `show_sounds`, `fx_sounds`, `party_chats` and `menu_rules`, which are now free functions both modes call. The town's draw hands its sprites and the airship's `effSmokeN` to the effects, and its sounds go into the area's calls.

These differences from the old town path all follow the game's order. The effects' `rand()` draws now come before the town's draw (96), not after it. The race task's own asks now go out at 63; before, they were appended at the end. The effects draw `genrand` from the one ccRand; the old town host returned 0 without drawing. They use the real camera rotation, and `ccCheckTarget` reads the NPC's listing.

## State

At cef6f41, piney-world (release) passes. piney-game gives 347 passed, 0 failed and 152 ignored, as the baseline. Every town and map harness run (test_town02_rs, test_town03_rs, test_world_rs, test_party_rs, test_grunty_rs, test_merchcam_rs, test_race_rs, test_evarea_rs, test_field_rt, test_evchar_rs, test_evcam_rs) gives the baseline's lines on all four volumes. Of the 51 town golden shots, 49 are unchanged and two of flag_race_shots move. In `mut-race-result` one walking NPC behind the ride is different. It does not come from the effects' draw or from the party's gate lights; most likely the effects now draw `genrand` from the one ccRand, and that moves the walking PCs' draws. In `mut-race-rankings` the whole frame is brighter. Bisecting traces it to the town's `ccEff` sprites (the lens flare and the clouds), now drawn through `AreaFx`'s sprite cache and assets instead of TownFx's own; the cause is not yet found. The full checks (every shot, gen --check, clippy) were not run. The UI's handles (item 4) and one mode and HostWorld with the façade's removal (item 5) were not started.

The scratch notes are /mnt/data/claude/scratch/world3/HANDOFF.md, "Leg 7" and "Next, in order (after leg 7)".

**Still unknown:** why the flag race's Rankings shot is brighter when the town's lens flare and clouds go through AreaFx's sprites (bisected to the sprite path, cause open); whether the walking NPC that moves in the race result shot comes from the effects' genrand draws (likely, not proven); the town's draw's effSmokeN (and a field's effSmoke) start at the next effect task where the game's ccThParticle (98) sees them in the same frame; the UI's one handle scheme (the game names a target by its ccChar; the likely port is town NPCs as battle-scene stand-ins), the rest of one frame (the town's game_ctrl, ev_hold, draws), one mode and HostWorld, and the façade's 49 methods and Deref.
