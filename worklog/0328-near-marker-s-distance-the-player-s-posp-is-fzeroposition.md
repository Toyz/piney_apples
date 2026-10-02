---
number: 328
title: "near_marker's distance: the player's posP is FZeroPosition with his ground height"
date: 2026-10-01
area: script, world, decomp
files: crates/piney-world/src/field_world.rs, docs/engine/field-game.md
---

# 328. near_marker's distance: the player's posP is FZeroPosition with his ground height

[[134]] ported `near_marker` outside the towns as the distance between
`W2P(marker)` and `W2P(player)`, assuming the game's `plw->posP` is the
player in that same wrapped frame. The game's code (INF
SLUS_202.67:0x001a7d9c) is
`ccGetDist(ccTransPosW2P(marker), plw.pw->posP)`, where `plw +0x20` is the
`ccPlayer*` and `+0x50` is `ccChar::posP`.

What the two sides are:
- **`ccTransPosW2P`** (gcmn 0x0059b940) is `ccPlayer::W2PPos`
  (0x0059b5a0): `w - this->pos`, x and y wrapped through
  `WORLD_MAN`'s bounds (+0x420..+0x42c) about their middle, z kept as
  `w`'s.
- **The player's `posP`** is set by `ccPlayer::ccPlayer` (0x00597b74) to
  `FZeroPosition` (main 0x002f7390: 0, 0, 0, 1), while `pos` takes
  `StartPos[n]`. After that, two things change it:
  - `CollisionTest` and `MapLoopAdjustPos` write its z: the ground height,
    as `pos.z`, at 0x0059b418;
  - `Main` writes `W2P(GetTransCenter()) + diskOffset` while
    `WORLD_MAN::GetTransMode` is set (0x0059849c). `diskOffset` (+0x2d0)
    is `pos - centre` from the constructor, so this is the carrier's move
    since the last frame, which `P2W` then adds to `pos`.

Nothing else writes the player's `posP` x and y. A scan of gcmn's 357
`ccTransPosW2P` calls finds `posP = W2P(pos)` for every other character
each frame (`ccFellow`, enemies, gimmicks, merchants, NPCs, bosses), but
none for the player.

So off a carrier the distance is `|W2P(marker) - (0, 0, z)|`, the
player's own distance, as the port computed. On a carrier the game
measures from that frame's carrier move instead. The port's Kite already
carries the game's `posP`:
- `Combat` makes it `[0, 0, z, 1]` (combat/mod.rs, combat/town.rs);
- `collision_test` writes its z;
- the trans-mode branch of `kite::motion` writes the carrier move.

`FieldWorld::player_distance` now measures to that character's `pos_p`,
and to `(0, 0, z)` when there is no Kite in the combat.

`event_17_walks_kite_to_the_portal` (block 14's `near_marker 1 <= 250`)
and the RACHEL-2 side event tests still pass, and so do the piney-world and
piney-game suites (204).

**Still unknown:** no test rides a carrier to a marker. Which areas set
`GetTransMode` and use `near_marker` there was not surveyed. The other
half of [[134]], the statue room's doorway in area 18's dungeon, is still
open.
