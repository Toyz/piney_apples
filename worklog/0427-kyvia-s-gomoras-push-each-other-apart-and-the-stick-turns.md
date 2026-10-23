---
number: 427
title: Kyvia's gomoras push each other apart and the stick turns its camera: kyviaGomora::Move's CollisionDetection in the hits' list, ZrotControlFlg and ExLock in Pad_Control, checked against the game's code on both fights
date: 2026-10-22
area: battle, world, test, tooling
files: crates/piney-battle/src/world.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, crates/piney-world/src/hit.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/bosscam.rs, crates/piney-world/examples/bosscam_probe.rs, crates/piney-game/src/session/tests/kyvia_fx.rs, tools/test_battle_kyvia_rs.py, tools/test_bosscam_rs.py, docs/engine/boss-kyvia.md, docs/engine/boss.md
started: 2026-10-22T17:27:52-06:00
took: 35m
---

# 427. Kyvia's gomoras push each other apart and the stick turns its camera: kyviaGomora::Move's CollisionDetection in the hits' list, ZrotControlFlg and ExLock in Pad_Control, checked against the game's code on both fights

Worklog 272 left two parts of Kyvia's first fight unported, and
boss-kyvia.md listed them as Unknown: the gomoras' pushes on each other
and "the pad's sway" (`bossCam` +0x139). In the port the gomoras flew
through each other, and the right stick did nothing to the camera.

## What the game does

**The pushes.** Mutation's kyviagomora.cpp; Outbreak's copy is the same
code.

- When a gomora is fully out (`Think` step 2, MUT gcmn 0x00500f9c), its
  `bodyHit` (+0x240) takes:
  - its row's width and height as radius and height;
  - kind and mask 0x80 (bosses), `mask2` 0 (no ground);
  - then `HitEnable`.
- The dash takes it out and puts it back (`Action` +0x2f8, +0x884). Act
  14 takes it out (+0xab4).
- `kyviaGomora::Move` (0x00500850) moves the gomora as `ccBoss::Move`
  does, then puts the hit 100 below it and calls `CollisionDetection`
  (main 0x001534f0): the push out of each other listed body of a boss
  kind, `r1 + r2 - d` along the line between the centres, summed in
  list order.
- When pushed, the gomora and its `posP` move by the whole push. This
  keeps its height, which `ccBoss::Move` drops. The hit then stands at
  the gomora's own place, not 100 below.
- The body's hit (radius 400 in both constructors, kind 0x80) is listed
  from its constructor. The core's never is (`bodyHitSW` 0).

**The sway** is `ZrotControlFlg` (+0x139; Infection's +0x129), which
Kyvia's constructor sets. `Pad_Control` (MUT 0x00475cf0, INF
0x004602a0) then turns `ZrotCont` after the zoom and resets, held to
+-0.5:

- the right stick's `sinf(dirc) * pow` past 1, times 0.00023: added in
  schemes 0 and 2, taken off in 1 and 3;
- in schemes A, without the stick, L1 and R1 by 0.015, plus 0.0008 per
  step of pressure past 155 (0.08 more at no pressure).

Mutation's `SetMode` (0x00475ae0) also keeps `ExLock` (+0x138): mode 3
sets bit 4 and mode 4 clears it; mode 5 sets bit 0 and mode 6 clears it.
`Pad_Control` reads nothing while either is set, so between an attack's
mode 3 and its mode 4 the stick neither zooms nor turns. The port read
the pad there.

`tools/test_bosscam_rs.py` had only run on Infection. On Mutation it
found two more differences in `CamMain` (0x004755a0):

- the eye is raised by +0xdc times `1 - t` (0 for every boss but
  Quarantine's `ccBoss08_c`);
- `CamPos.w` is set to 1.

It also gave Mutation's layout (0x150 bytes): +0xdc pushes `Xrot` to
`count` 4 further, and `RotZ` on 0x10 further.

## The port

- piney-battle:
  - `world::char_push` is the bodies' half of `CollisionDetection`.
    piney-world's `Hits::collision_detection` now uses it as well.
  - `boss::PartHits` (`switch`, `collide`, `sync`) and `NoHits` reach the
    list from `Cx::part_hits`.
  - `Gomora::hit` replaces `hit_on` and the dropped `Out::PartHit`.
  - `gomora::move_` pushes.
  - `kyvia::BODY_RADIUS`.
- piney-world:
  - `combat::boss::PartList` answers `PartHits` from the field's hits.
  - The body's entry has radius 400 and moves with the body's push, for
    the gomoras that move after it.
  - A part's body leaves the list with its boss.
  - `BossCam` keeps `ex_lock`, `zrot_control` (`Out::CamSway`) and
    `lift` and `later` (Mutation's `CamMain`); `pad_control` honours
    `ExLock` and turns `zrot_cont` (`zrot_turn`).

## Checks

- `tools/test_battle_kyvia_rs.py` now runs the hits' list natively
  (`SetHitSW`, `HitEnable`, `HitDisable`, `CollisionDetection`), from an
  empty list each case, with no ground (`ccModelHitCheckQ` -1.0).
  - battle_probe keeps its own list of the body's and the gomoras'
    bodies.
  - Compared each frame: every gomora's `bodyHit` (switch, masks, kind,
    radius, height, place, push) and the camera's `ExLock` and
    `ZrotControlFlg`.
  - Mutation bulk 120 from 9300 (161,184 frames, 3,761 gomora-frames
    pushed) agrees.
  - Outbreak's second fight, 60 cases (180,064 frames, 1,912 pushed),
    agrees, and so do 30 of its first (41,061, 808).
- `tools/test_bosscam_rs.py` runs on any volume (`PINEY_VOLUME`, the
  layout by volume). The new `test_sway` runs `ZrotControlFlg` on all
  four schemes with runs of the stick leant and of L1 and R1 at
  pressures. On Mutation and Outbreak, `SetMode` 3 and 4 come and go
  too.
  - `ZrotCont`, `ExLock`, the camera's members, the cameras and the
    matrices agree on Infection, Mutation and Outbreak.
  - Both clamps are reached, and both modes.
- `session::tests::kyvia_fx`:
  - `kyvias_gomoras_push_each_other_apart`: over 6,000 frames of each
    fight, the gomoras' bodies are listed in 5,255 (Mutation) and 5,260
    (Outbreak) frames, and pushed in 1,567 and 271.
  - `the_stick_turns_kyvias_camera` (stick right then left in 300-frame
    runs): `ZrotCont` goes to -0.5 and +0.5, and holds while mode 3 holds
    the pad.
  - Before, nothing was listed and `ZrotCont` stayed 0.
- piney-battle and piney-world tests, piney-game's suite (four threads),
  clippy, fmt.

The pad's stick read straight sideways (y 128) gives angle 0 (piney-input's
`stick`, the game's), so the session test leans it a step down.

Shots (`kyvia_push_and_turn_shots`, /mnt/data/claude/scratch/a427):

- `kyvia-turn-00520-act0-z-0.50.png` and `kyvia-turn-01040-act0-z+0.50.png`:
  the eye turned either way about Kite.
- `kyvia-push-*.png`: frames in which a gomora was pushed.

**Still unknown:** nothing. Which of CMP_ex01gom2-4 each gomora draws stays open under 272.
