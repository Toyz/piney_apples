---
number: 415
title: Moonstone leaves event 162 in the swirls: the effects' camera cone is the active camera's, ecam under an event's camera, not tcam (#67)
date: 2026-10-21
area: world, render, script
files: crates/piney-effect/src/host.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session/tests/party_leave.rs
started: 2026-10-21T09:46:15-06:00
took: 57m
---

# 415. Moonstone leaves event 162 in the swirls: the effects' camera cone is the active camera's, ecam under an event's camera, not tcam (#67)

Issue #67 (build 3e87cd5, Mutation): at the end of Moonstone's quest in
Lambda Bottomless Guffawing Raw Ore, Moonstone vanished without the
gate-out rings. The event is 162 (GETUCHOUSEKI, `eventTblS212`). Block 8,
in the dungeon's first room (event point 1) once it holds no entries,
sets the camera (`camz_set`), puts Kite and Moonstone, has Moonstone say
"Not bad. Later..." and runs `pc_act 7 5`: remote command 5, whose
`ccAI::ManualControl` (gcmn 0x00580ef0) calls `TransferOut` and
`resignParty`. Act 12 then calls `effTransfer(this)` at count 1
(`ccFellow::Action` 0x0041df20): three `CMP_x032` rings and the photons.

## The cause

The port ran all of that: effect 4 and its three effect 3 rings were
live on Moonstone's scene index for their 40 frames. None was drawn. The
effects' `Host::camera()` came from `Camera::from_field`, which took
`tcam`'s eye and view point. The game reads `cameraList[camID]`:
`ccEffect::Main`'s eye is `cameraGetPos(camID)` (main 0x00161590) and
`ccCheckCameraDeg` reads `activeCamPtr` (0x001da750, 0x001da75c), which
`changeCamera` (0x00161808) points at `cameraList[camID]`. Under an event's
camera that is `ecam` (camID 3). In event 162 the field camera behind
Kite faced away from Moonstone, so every ring failed the 67.5-degree cone
and was never drawn. The same cone and eye feed the particles, the words
pulled 1000 in front of the eye (`ccTransPosW2CZW`, also `activeCamPtr`)
and the portals. `from_field` now takes `c.active()`: `tcam` in play,
`ecam` under an event's camera or a shop's (`SetMerchantCamera`).

## What else it covers

Every effect seen through an event's camera, on all four volumes, in
towns (`town_fx`), fields and dungeons. That includes the 44 events with a
`pc_act` 4 or 5 transfer (11, 14, 15, 17, 18, 21, 22, 24, 25, 58, 59, 61,
104, 108, 109, 111-113, 115, 155-157, 159-162, 202, 205, 206, 210, 211,
217, 255-257, 259-262, 301, 304, 305, 315, 355) and the 40 with a fight
under an event camera (`battle_ready`). Infection's event 18 (issue #16)
and event 155 (issue #66) are among them.

## Check

`session::tests::party_leave::moonstone_leaves_in_the_swirls`: event 162
from Mutation's story start 104 (155, 157 and 159 brought forward,
`eventStatus[3]` 3), blocks 0-6 run, in area 65's dungeon with Moonstone,
the dungeon's portals counted closed. Block 7 takes the room again, block
8 plays, and Moonstone leaves by act 12 with `effTransfer` on him and the
three rings drawn together on 20 frames or more. Kite is alone back in
Carmina Gadelica. It fails before the fix (rings drawn on 0 frames).
`FxCensus::effect_draws` names each effect and particle draw of the step.
`host::tests::the_cone_is_the_active_cameras` checks `from_field` with
`ecam` facing away from `tcam`. The cone itself (`ccCheckCameraDeg`) was
already checked against the game in tools/test_effect_rs.py; what changed
is which camera feeds it, read from the disassembly above.

Shots: /mnt/data/claude/scratch/i67/before/f0156.png and f0176.png (no
rings, Moonstone fading out), /mnt/data/claude/scratch/i67/after-test/
leave-28.png (`moonstone_leave_shots`: the rings round him as he fades).
piney-game's suite (four threads), piney-effect's and piney-world's
tests, clippy and fmt pass.

**Still unknown:** nothing.
