---
number: 435
title: Kyvia's thunder reads the boss camera: no reachable code changes camID from 2 in its fights
date: 2026-10-22
area: battle, world, test
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/kyvia/thunder.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/session/tests/kyvia_fx.rs, docs/engine/boss-kyvia-second.md
started: 2026-10-22T23:22:24-06:00
took: 23m
---

# 435. Kyvia's thunder reads the boss camera: no reachable code changes camID from 2 in its fights


The second fight's page left one question open. The storm thunder (kind
2) takes the camera's eye, view and turn with `cameraGetPos`,
`cameraGetView` and `cameraGetRot` of `camID`, not of camera 2. The port
took them as the boss camera's, but nothing showed `camID` could not
change mid-fight.

## What sets camID

Kyvia's `HandAtk` and `ThunderboltAtk` read `camID` (MUT gcmn 0x004e0790
and on; every fight's copy does the same). Each Kyvia constructor calls
`ccBoss::InitBossCamera` (QUA gcmn 0x003622f0), which makes the
`ccBossCam` and calls `changeCamera(2)`. The disassembler's scan finds
every `jal changeCamera` on Mutation, Outbreak and Quarantine:

- `ccThCamera`'s start, before the scene plays.
- The event's `camera_end` (49), `camera_end_reset` (50) and its camera
  task (`ccThCameraExecute`).
- `EVENTAREA03::Draw` and the other bosses. Also `ccBoss::OffBossCamera`,
  `BeginDeadEffect` and `ccBossCam`'s destructor, which run after the
  fight.
- The merchants' menus (Vender, Recorder, Fairyshop, Breeder) with
  `SetMerchantCamera`, `SetFountainCamera` with `FountainMenu3`, the
  dogs, and `GateHackingOut`.

Outbreak's and Quarantine's symbol files call the four merchant menus
`StrwinMenu`, `SpcMenu`, `PcMenu` and `NpcMenu`, and `FountainMenu3`
`ItemIdolMenu`, one function off. Each has Mutation's merchant menu's
body: `SetMerchantCamera` (camera 3), then `changeCamera(1)` on each way
out.

## None of them runs in a Kyvia fight

- **Menus.** Each opens only on a target whose flags pick it (the menu
  buttons' table). On the disc the targets are the party and Kyvia's
  parts. The entries the events make in their town blocks go with those
  scenes. A party member's menu (`SpcMenu`, 21; Mutation's names it
  right) changes no camera.
- **Event blocks.** A block's settings hold for the blocks after it, so
  each of events 108, 211, 308 and 312's blocks with a camera
  instruction was read with the settings it inherits. Every one is held
  to another place (`in_town`, `in_dungeon`, `in_point`), or to a
  `status[0]` value that only the block after `if absent` sets (108's 23,
  211's 13, 308's 8, 312's 12). So none plays while Kyvia is in the
  field.
- **Result.** `camID` is 2 from the constructor to the death, so the
  thunder reads the boss camera. Its eye and view are what `ccBossCam`'s
  `CamMain` and `SetMode` write to camera 2. Its turn is what
  `cameraGetRot` works out from them.

## The port

- piney-battle: `CamView::active_pos` and `active_view` are
  `cameraGetPos(camID)` and `cameraGetView(camID)`. Kyvia's hand (its
  `tes_pos`) and the storm read them, as the game's code does.
- piney-world: `cam_view` fills them from the active camera.
- battle_probe: they come from the case, as the harness's camera hooks
  answer.

## Checks

- **Session tests.** `kyvia_shows_its_meteors_beams_and_auras` (event
  108, 12,000 frames) and `kyvia_s_second_fight_shows_its_thunderbolts`
  (211, 16,000) now check `camID` on every frame of the fight. So do
  `kyvia_s_third_fight_to_its_end` (308, to the body's death) and
  `kyvia_s_fourth_fight_to_its_end` (312). It is 2 throughout.
- **Harness.** test_battle_kyvia_rs.py's units pass on all three volumes
  and on the fourth fight, and Outbreak's `bulk 10` (33,686 frames)
  agrees.
- **Suites.** piney-battle, piney-world and piney-game's suite (four
  threads) pass, as do clippy and fmt.

**Still unknown:** nothing.
