---
number: 457
title: Corbenik's three forms play in field 8: code 7, the second form in the first's task, their parts, cameras, generators and pictures, under god to event 313's end
date: 2026-10-24
area: world
files: crates/piney-world/src/combat/corbenik.rs crates/piney-world/src/combat/boss.rs crates/piney-world/src/combat/mod.rs crates/piney-world/src/field_world.rs crates/piney-effect/src/corbenik.rs crates/piney-effect/src/boss.rs crates/piney-game/src/fx.rs crates/piney-game/src/area.rs crates/piney-gen/src/manifest.rs crates/piney-game/src/session/tests/corbenik.rs docs/engine/boss-corbenik.md
started: 2026-10-24T12:16:59-06:00
took: 11m
---

# 457. Corbenik's three forms play in field 8: code 7, the second form in the first's task, their parts, cameras, generators and pictures, under god to event 313's end

## Corbenik in the field

The rules of the three forms were checked against the game (worklogs
450-453). The world had no code 7: event 313's `entry 7 7` started
nothing. This puts the forms where the event makes them.

**What the code says.**
- `ccThBoss08` (QUA gcmn 0x003aca30) reads `saveData` +0x652f, which is
  `eventStatus[55]`. Event 313's block 18 adds 1 to it after the first
  task ends. At 1 the task plays the third form (`ccThBoss08c_Main`,
  which also sets the font's type to 0 each frame).
- Otherwise it plays the first form until its `CheckExit`. In that same
  frame it clears the menu's +0xfe, +0x100 and +0x26, calls
  `EntryFlash(30, 0x80e0ffff)`, deletes the first form (its destructor
  takes the seeds) and makes `ccBoss08_b`. After the second form's loop it
  clears the menu again and sets the task's +0x14, which `absent 7` reads.
- The third form's constructor sets its camera: `InitBossCamera(980,
  1500)`, the pitch `atan2f(500, 1000)` at +0xe0, the range 1000 at +0xd8
  and `1500 tanf(pitch)` at +0xdc, then `SetRotXLimit(1.0)`. The first
  form has no boss camera. The second form calls `InitBossCamera(200,
  1000)`.
- The parts draw at their own matrices:
  - a seed 50 up at 3x (its `PreDrawAnm` 0x003b09f0 also draws once more
    before `ccBoss::Draw`);
  - a root at 1.5x (`@1033`);
  - an eye turned XYZ, in its flash's `SetFogBlend(blend, 0xffffff)`.
- The generators, from their makers' code:
  - row, force-field rows, the `distSW` bit cleared and `pTexMod` for each
    (the drain's four, the dead's three with textures 105 and 31, the
    tera charge's 108);
  - a seed's charge is `g_energyChargeGenerator` row 0 (ff 1, 4);
  - Mutation on, the tera laser's charge glow and fall rows sit right
    after `g_generateEyeFF` (QUA 0x00515490..0x00515550).
- The pictures' draws were read for: the flush, the stars and core, the
  sparks (with their turn, now in `Pic::Spark`), Thunder3's segments, the
  tera beam, the transform's lays, the barrier at `OBJ_ex8bria` (`GetSubstAdrsF` of
  `OBJ_ex8body2`), the press images, the elements (the body's neutral clip
  one step on), the drain's root, and the laser rain's and reflected
  lasers' segments and heads. The laser rain's heads share one
  `ANM_ex31les3`, stepped once for each falling laser (now `Pic::LaserHead`),
  and its draws turn by the trail's back way, not the fall's.

**The port.**
- `combat::corbenik`: `parts`, `third_camera`, `part_matrix`,
  `corbenik_next_form` (run right after the first form's last `Main`),
  and `shown` (the x81 and xeffect pictures the field draws).
- `start_boss` takes code 7 and the save's flag.
- The world handles `CamLook`, `CamLock`, `CameraType` and `EntryFade`.
- `cam_view`'s eye view reads the active camera's type.
- The effects:
  - `piney_effect::corbenik` draws the generators, sprites, clumps,
    bursts, shields, smoke and radiates.
  - `Kept::alive` answers `ccCheckParticleGenerator` for the keys, through
    `FxTasks::boss_gens_alive` into `Combat::gens_alive`.
- piney-gen gains the generator rows. x81 joins the effects' files on
  Quarantine.

**A world bug found.** A part's `ccDeleteCmnd` reaches the world among its
boss's outs. The world took the boss off the lists for it, so after the
first wave the first form could not be targeted and the fight never
ended. The rules do the delete themselves, so the world no longer acts on
that out. `corbenik_under_god` fails without the fix.

**Checked.**
- `session::tests::corbenik::corbenik_under_god` plays forms one and two
  from the arena, then the third after block 18's scene change, to event
  313's end in 76,284 frames, with the music for nearly all of it.
- The Corbenik harnesses still pass (3 cases each).
- The shots show the seeds, their red after-images and counts, the
  waves, the second form's barrier, roots, elements and lasers, and the
  third form's eyes and reflected lasers.

**Still unknown:**
- The seed's second draw in its `PreDrawAnm` (at the last frame's
  transparency) is left out; one draw a frame.
- The lasers' blend settings are not applied: `OBJ_ex31les1`'s
  `alphaBlendTbl[1]` and its model's render state, and the head clip's
  `SetBlendType(1)`. The field draws them with the file's.
- The first press image draws at the body's last transparency in the
  game; the port uses the image's.
- The fight with no aid, and 313's town part under the pilot, are not
  played yet.
- Field 8's sky shows "BATTLE MODE ON" tiled across its top through the
  fight. It is not compared with the game.
