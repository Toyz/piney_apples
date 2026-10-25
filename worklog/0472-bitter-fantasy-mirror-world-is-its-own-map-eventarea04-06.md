---
number: 472
title: Bitter Fantasy Mirror World is its own map: EVENTAREA04-06 ported, so field 66 has its ground and area 66's last door reaches the arena and its fight (#74)
date: 2026-10-25
area: world, test
files: crates/piney-world/src/evarea_sky.rs, crates/piney-world/src/story_map.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/session/tests/mirror_world.rs, crates/piney-game/src/launcher.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/outbreak_end.rs, docs/engine/evarea.md, GAPS.md, UNKNOWNS.md
started: 2026-10-25T08:56:27-06:00
took: 36m
---

# 472. Bitter Fantasy Mirror World is its own map: EVENTAREA04-06 ported, so field 66 has its ground and area 66's last door reaches the arena and its fight (#74)

Issue #74 (shugo21-bit, build 24205f6, Mutation; also seen on
Quarantine): the post-game quest at Θ Bitter Fantasy Mirror World
arrives in a field with no ground, only an oasis and black void. The
dungeon's bottom leads back to a field instead of the boss arena. There
the Sprite Ocarina is grey and Gate Out does nothing.

## The cause

Side event 163 (open once event 115 is done, on every volume) gives
area 66's words. Fields 66, 67 and 91 have rows of model 1 on all four
volumes. `GO(1)`'s switch makes `EVENTAREA04` for 66, `EVENTAREA05` for
67 and `EVENTAREA06` for 91. The port had none of the three, so
`story_map::build` answered None and `FieldWorld::enter` generated a
field from the rows' type 255:
- **The ground.** Field 66 was a generated field of a type with no
  ground table: no ground drawn.
- **The door.** Area 66's last door is right: `GotoNextRoom`'s -255,
  `ChangeArea(1, 67)`. But field 67 was another generated desert field.

Event 163's block 2 still ran there: a portal of enemy 262 at (0, 0,
500) and `battle_ready`. `battle_ready`'s `AddOperate(14)` makes
PERSONAL grey Gate Out until the event ends, as the game means: the
arena's only way out is the fight. The ocarina is grey in any field.

## The classes

`piney_world::evarea_sky::SkyMap` holds all three as one shape, a
`Class` each (MUT gcmn 0x00417050, 0x00417e10, 0x00419e30; the same code
on OUT and QUA, tables the same on all four). Each has:
- the static models and objects of its tables (piney-gen's, all type 0);
- the `EA_BGNAME` sky clumps on `bgLayer[k]` (04's `bg[2..4]` fogged);
- one `MoveTexture` row;
- the light animation's one light, the fog and `bgColor`;
- 25 `CLOUD`s `Init`ed at the first `Draw` (type 0, or 3 for 06);
- for 04 and 05 a `LENSFLARE` at `DMY_dummy5`.

05's models go on floorLayer. `se2_6` holds both 04's and 05's pieces;
each `STATICMODEL` enables only its own Hit chunk, so each map collides
with its own floor.

The starts write the dummy's chunk first:
- 04: from the dungeon (-8000, 5800, 920) facing 0, else (4600, -4700,
  910) facing pi;
- 05: `DMY_dummy3` at y 9000, z 800;
- 06: `DMY_marker02` or `01`.

`WORLD_MAN::Enter` (MUT main 0x001b2b80): 66 to its dungeon, 67
nothing, 91 to its dungeon from block 0 or -1. 91's other branch
(`EVENTAREA07::ChangeBlock(0)` on the 06 map) is never taken: no door or
script gives field 91 another block. `SetCharPosition` stands 66's party
as field 16's. The tables are in docs/engine/evarea.md.

## The player's log

`issue_74_replay` replays the log through the launcher (it now hands
each frame's picture to the callback):
- **Before the fix** it plays the bug. Field 66 at frame 8,289 is a
  generated field (/mnt/data/claude/scratch/i74/replay/f08689.png: black
  where the ground should be, as the player's shot). The dungeon's three
  floors follow, then field 67, again generated, at 17,752
  (/mnt/data/claude/scratch/i74/replay/f18152.png). The fight there runs
  to 27,659, when the player quits to the title.
- **After the fix** field 66 is the ruined city in the clouds
  (/mnt/data/claude/scratch/i74/after/f08689.png). The player's steps
  were for the other field, so the log goes elsewhere from there.

The arena from area 66's door: /mnt/data/claude/scratch/i74/arena/arena-0200.png
(the causeway to the round floor), arena-0600.png (enemy 262 out of its
portal).

## Checks

- `evarea_sky::tests::the_sky_maps` (all four volumes, each class): every
  row in the file, the clumps, one light, the own-models collision, the
  scrolled material, a first draw with its clouds.
- `evarea_sky::tests::the_starts`.
- `session::tests::mirror_world::fields_66_and_91_are_their_maps`
  (Mutation, Quarantine 66; Outbreak 91): the class, the start on the
  floor, and the floor's door into the dungeon.
- `session::tests::mirror_world::area_66s_last_door_leads_to_the_arena`
  (Mutation, Outbreak, Quarantine): from area 66's dungeon the door's
  `ChangeArea(1, 67)` makes `EVENTAREA05`. Kite put on the portal, enemy
  262 comes out. Beaten, it ends event 163: block 6 played, the event
  closed, the party in town.
- Both session tests fail with the classes taken out of `build`.
- `mirror_world_shots` (ignored) writes the arena's shots.
- The piney-game suite (327 passed), piney-world and piney-data, clippy
  and fmt pass.

**Still unknown:** nothing about these maps. Two things the game leaves
to runtime state are not modelled, as for every story map:
- `fieldrand`'s state at the clouds' `Init`: whatever the last
  generator or `WORLD_MAN::Init`'s count seed left. Each map starts its
  own from 0.
- `EA_moveTex04`-`06` live in gcmn's data and keep their scroll across
  visits; each map starts its scroll at 0 (docs/engine/evarea.md,
  Unknown).
