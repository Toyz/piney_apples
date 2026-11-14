---
number: 583
title: One world, leg 6: the probes on the one Kite, one Kite reader, one ccChar::Draw fade, a town's party and NPC instructions the world's
date: 2026-11-03
area: world, battle
files: crates/piney-world/examples/common/kite_run.rs, crates/piney-world/src/world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/chara/char.rs, crates/piney-battle/src/evparty.rs
started: 2026-11-03T21:30:57-07:00
took: 2h 11m
---

# 583. One world, leg 6: the probes on the one Kite, one Kite reader, one ccChar::Draw fade, a town's party and NPC instructions the world's

Leg 6 of the relay finishing "One world, one area system" (plans/rust-cleanup.md section 2), on branch `one-world` after [[582]]. It moved the last users of the town's second port of `ccPlayer::Main` onto the battle's Kite and deleted that port, folded Kite's ten readers into one, made `ccChar::Draw`'s fade one rule, and gave a town the world's event party and NPC instructions. Commits: 9d185d8 (probes, old port gone), 09e2567 (one Kite reader), ef90099 (one ccChar::Draw fade), 631deac (party instructions), 59a7da0 (NPC instructions).

## The probes on the one Kite

world_probe, story_map_probe, merchcam_probe, field_probe, evcam_probe, dungeon_probe and bosscam_probe drove `piney_world::tasks`, the town's own `Player::main`. Each now runs Kite as evchar_probe does: `Combat::new`, `add_kite` for the area (0 a town, 1 a field or story map, 2 a dungeon; arriving, or standing in the dungeon), then per frame `cameraMain` and `Combat::frame`. The shared code is examples/common/kite_run.rs, which the probes include by `#[path]`. One thing had to be set that the old port never read: the party. Without it `party.annihilated` holds, and Kite's `motion` clears his move flags before `ControlMove` can run (frame 0's `speed_rate` was 1 against the game's 0).

Then chara/player.rs went (`Player`, `main`, `control_move`, `manual_move`, `pad_lever_power`, `Frame`, a third `ccGetCameraTransparency`), with lib.rs `tasks`, chara::motion's `Acts` and `anim_ctrl`, and chara::ai's `brains`, `manual_control` and `move_p2p`. world_probe's `move` request and test_world_rs's `test_control_move` went with `control_move`: the one `ControlMove` is the battle's, which tools/test_battle_kite_rs.py checks over every `ctrlType`.

## A bug the move found

evcam's `test_follow` differed by 3 ulp in Kite's transparency at a close camera. `ccChar::Draw` inside `ccPlayer::Main` measures the near fade through `ccGetCameraTransparency`, whose `ccTransPosW2P` reads plw's position, which is where he stands after this frame's move. The battle's `Stage` measured from his position at the frame's start. `Stage::draw` (Kite's) now takes his position first. evcam and evchar pass after it.

## One way to read Kite

`World::kite()` returns `world::Kite`: `Starting { pos, dirc }` before `ccSPC::Reboot` builds him (the area's start), or `Built(combat::KiteRef)`, his character, `ccSpcChar` record, actor and listing. `pos()` and `dirc()` answer for both; `built()` gives the rest. `Combat::kite_ref()` makes the `KiteRef`; `Combat::kite_pos` and `kite_dirc` went with the ten World getters. The old `World::kite()` (his model files) is `kite_model()`.

## One ccChar::Draw fade

`chara::char::CharFade::new(set_transparency, fade, hide)` is the draw's rule (INF GCMN.PRG:0x0056b1c0): the transparency, drawn unless under 0.05 and not hidden, and the shadow's alpha, `setTransparency` while the near fade hides and the transparency otherwise (0x0056b4e4-0x0056b54c). `Char::fade`, the enemies' `char_draw_parts`, the Chaos Gate's draw and `Stage::char_draw` (Kite, the members, the ride's Kite) all use it. Actors keep the shadow's alpha apart from the body's (`Actor::shadow`), and the world's and the town's draws use it. `ccGetCameraTransparency` is one function now, piney_battle::geom's; chara::char's and `Stage`'s only put their points through `ccTransPosW2P` first.

## The party's and NPCs' instructions

A town's event party instructions are `World::pc_command` (`EvParty`), as everywhere. `EvParty` finds a town's NPCs through `npc_pos`, a `GetNpc` on the NPCs the battle's entry control does not hold, which the world answers from its cast. The town's second ports (`pc_turn`, `pc_face`, the remote walks, `pc_command`, `Spcs::turned`) went. Where the two disagreed, `EvParty` is the game's (battle_probe-checked): `pc_put` of -3 names no one, because the game reads `memberID[0]` as the pointer and Kite's id 0 is null; the town had put Kite. `pc_face` of type 0 or 1 does nothing. evchar_probe's turn and face now go through `EvParty` in a town, and tools/test_evchar_rs.py passes (39 scenarios, 12330 frames). The NPC instructions were the same code twice; `World::npc_command` serves a town, taking a marker from the town's `markerEvTbl` dummy there and from the event positions elsewhere.

## State

At 59a7da0 piney-world (release) passes, and piney-game gives 347 passed, 0 failed, 152 ignored, as the baseline. Every town and map harness (harness.sh's fifteen, with test_evcam_rs and test_bosscam_rs) gives the baseline's lines on all four volumes; test_world_rs has one test fewer, `test_control_move`. test_evchar_rs and test_battle_event_rs pass. The town shot subset (51 files) is identical to [[582]]'s. The full checks (every shot, gen --check, clippy) were not run. The scratch notes are /mnt/data/claude/scratch/world3/HANDOFF.md, whose "Next, in order (after leg 6)" carries what follows: one frame with the town's NPC list at the battle's entry-control slot (64, before the effects at 80), then a town on the battle's effect host so it takes `Show::Rule`, then the UI's handles, one mode and HostWorld, and the façade (51 methods).

**Still unknown:** a Root Town still runs its NPC list (merchants, dogs, Grunties, walking PCs, event NPCs) in TownCast::frame after the battle's tasks rather than at their entry-control slot (64), and with no battle effect host (NoFx) it drops Show::Rule and the battle's other shows (the ride's ClearConditionEffect on Kite among them): one frame first, then TownFx's starters into AreaFx; the UI's handles (a town's `2<<24|code` against a field's scene index) and the twelve per-target façade methods they keep apart; one mode and HostWorld; the façade's 51 methods and its Deref; whether ccGimFood's and the gimmicks' draws pass ccChar::Draw's near fade (their shadows take their entry's transparency); the golden shots' regeneration and the docs' FieldWorld and chara::player names (last leg).
