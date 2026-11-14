---
number: 585
title: One world, leg 8: the race shots' drift is the game's paw-smoke ground check, particles after the area's draw, one set of member commands, the effects reach own characters through the cast
date: 2026-11-04
area: world, engine, test
files: crates/piney-world/src/world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/world/cast.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/world.rs, crates/piney-effect/src/lib.rs
started: 2026-11-04T00:38:55-07:00
took: 1h 4m
---

# 585. One world, leg 8: the race shots' drift is the game's paw-smoke ground check, particles after the area's draw, one set of member commands, the effects reach own characters through the cast

Leg 8 of the relay finishing "One world, one area system" (plans/rust-cleanup.md section 2), on branch `one-world` after [[584]]. It settled leg 7's two shot differences from the game's code, put `ccThParticle` after the area's draw, and began the one handle scheme. Commits: 58ba3e6 (particles after the draw), ca43dff (one set of party-member commands), 9ec57b9 (effects reach an area's own characters through the cast).

## The race shots: the game's paw-smoke ground check

Leg 7 found two Flag Race shots changed: the rankings frame hazier by about 30 per channel, and one walking PC moved in the result frame. Neither is a regression. The sprite path is not the cause. With the shared `Assets` and with a fresh copy, every flare and cloud sprite gives the same `Eff` and the same GS packet, and the camera at the draw equals the camera after the step.

Leg 6 (3229f6b) and leg 7 were traced against each other frame by frame through `flag_race_shots`. Up to frame 205, the only difference in newlib `rand()` draws is five of Kite's `ccEffPawSmoke` calls that leg 7 no longer draws for. `ccEffPawSmoke` (INF main 0x001ce640; MUT 0x001e3ef0, OUT 0x001db1c0, QUA 0x001e2840) calls `ccLandHitCheck(pos, 0x20000000)` and `checkHitResultAttlibute` (INF 0x001ce7ac, 0x001ce7b4) and returns on water and grass before its two `rand()`s, in any area. Leg 7's effect host reads the ground there; the old town host answered 0, so a town raised dust everywhere. From those five draws the scene drifts: dog 3 picks a different act at its waypoint (frame 210), walking PC 0 touches a dog (1790) and draws `ccRand` three more times (1909), and Kite on the ride differs from 2079. The clouds respawn about the player from `fieldrand`, so the rankings frame has four or five clouds in view where leg 6 had two. Main's golden has the haze too (mean RGB 62.8, 78.5, 72.4; leg 7 64.8, 80.3, 74.7; legs 4 to 6 44.1, 60.5, 52.3). Legs 4 to 6 had lost it by the same kind of drift. No effect draws `genrand` in this scene, so leg 7's guess for the result shot was wrong.

## ccThParticle after the area's draw

The smoke a field's `WORLD::Draw` or a town's `ROOTTOWN0n::Draw` starts (96: steam, fires, the breaths' bursts, the airship's `effSmokeN`) waited for the next frame's effect task; in the game `ccThParticle` (98) moves it in the same frame. `Combat::frame_from_entry` no longer ends with the particle task. `World::frame` and the town's frame call `World::particle_task` after the area's draw while awake, and the queued smoke starts at the head of that task, after the shows raised since 80. `Combat::frame`, which the probes call, still runs the particle task last. The particles' draws go out after the area's draw (`Effects::draw_particles`, `FieldFx::draw_particles`). The town shot subset is unchanged by this.

## One handle scheme, the first two parts

A town's menus resolve a party member's handle to its scene index and call the one world's `chat_cmd`, `equip_report`, `manual_off`, `manual_mode_ai`, `remote_cmd`, `use_item` and `item_step`, as a field's menus do. The town's copies are gone: seven on the façade and four on the town's cast. They gave the AI an empty `Scene` and dropped its shows, and a town's item use carried only the pause step. It now carries every step the game's does.

The effects name an area's own character (a town's walking PC, merchant, dog, Grunty or event NPC) by its `ccEvent::GetNpc` code under one tag (`cast::own_ref`), and find it with `Cast::npc` on the cast the tasks lend. That replaces leg 7's per-frame snapshot of positions (`FxNpc`, `Cast::fx_npcs`) and its per-kind tags.

## State

piney-world (release) passes. The piney-game suite gives 347 passed, 0 failed and 152 ignored, as the baseline. The harnesses (test_town02_rs, test_town03_rs, test_world_rs, test_party_rs, test_grunty_rs, test_merchcam_rs, test_race_rs, test_evarea_rs, test_field_rt, test_evchar_rs, test_evcam_rs) on all four volumes print every line as leg 7's (world2/harness-l8). The town shot subset (51 files, world2/shots/l8b) is identical to leg 7's.

The scratch notes are /mnt/data/claude/scratch/world3/HANDOFF.md, "Leg 8" and "Next, in order (after leg 8)".

**Still unknown:**

- The menus' and the HUD's handle still means two things. In a field it is `2 << 24 |` scene index, in a town `kind << 24 |` code, and the targeting's `(Kind, i32)` holds an index in a field and a code in a town. The façade's per-target methods (`char_place`, `tag_pos`, `chat_point`, `char_info`, `command_target_ref`, `target_alive`) wait on one name for both. How the game's `ccCheckTarget` and `CamCtrl` name the Chaos Gate has to be read from the game before the gate gets one.
- One frame (the town's `game_ctrl` against the world's, `ccThEvHold`, the draws), one mode and one `HostWorld`, and the façade (42 methods) with its `Deref` are not done.
- The field shots after the particle change are not run; the full checks and the goldens are the last leg's.
