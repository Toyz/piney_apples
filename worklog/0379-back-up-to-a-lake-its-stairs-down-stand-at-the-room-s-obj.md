---
number: 379
title: Back up to a lake: its stairs down stand at the room's OBJ_0ppp, so the Sprite Ocarina and the stairs below it no longer leave the party in the void
date: 2026-10-05
area: world, test
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-game/src/session/tests/ocarina.rs, crates/piney-game/src/session/tests/dressing.rs, crates/piney-game/src/session.rs, tools/test_dungeon_rt.py, docs/engine/dungeon.md, docs/engine/field-ui.md, UNKNOWNS.md
resolves: 191
---

# 379. Back up to a lake: its stairs down stand at the room's OBJ_0ppp, so the Sprite Ocarina and the stairs below it no longer leave the party in the void

Issue #51 (ThreePendant, build 86c2571): "When you exit a Forest/Jungle
dungeon (possibly with Sprite Ocarina) the map fails to load". The
screenshot shows Kite in a dark blue void with no ground, sky or models.
The map in the corner is a small patch of sand texture with a "DOWN"
marker.

## Which exit

The "forest" areas are the lakes ([[362]]): field type 4, no field,
dungeon 0 the lake itself (type 8, the forest scenery) and dungeon 1
below it (type 2). The Sprite Ocarina is refused in the lake
(`sprite_refusal`: dungeon types 8 and 9). Below it, menu 86's
`WORLD_MAN::GoField` sends the party back up, `ChangeScene(2, -2, -2, 0,
0, lastRoom)`. The second dungeon's floor-0 stairs up do the same.
`GO(2)`'s last branch then builds `lastRoom` and puts the party at the
lake's `startpos[1][0]`, the room's stairs down ([dungeon.md](../docs/engine/dungeon.md)).

Played headless (`ocarina.rs`), both ways ended with Kite at (0, 0, 0),
far from the lake's room, which was built where it belongs; the map
showed only a patch of its texture. That matches the screenshot
(before: /mnt/data/claude/scratch/issue51/before/lake-0270.png).

## The cause

`DungeonArea::start_of` gave the lake types the room's centre for the up
stairs and skipped their down stairs. The comment said "not checked
against the game". So a lake's `startpos[1]` stayed at zero, and
`come_back` put the party there. This dates from the first commit of the
public history (3fa1e9e); there was no recent regression to bisect.

`DUNGEON::MakeRoom` (gcmn 0x005ba1d0), read:

- **Up stairs** (`exits & 0x10`): types 8 and 9 branch to 0x005badf4.
  They copy the room's centre with z 0 and w 1.0 (the constant @2918).
  The heading is turned (0 to pi, pi to 0, pi/2 to -pi/2, -pi/2 to pi/2)
  only when `GetFieldType()` is 4, the dungeon's number (+0x0) is 0, the
  floor is 0 and the room is 0.
- **Down stairs** (`exits & 0x20`, at 0x005bafac): no type test. Every
  type, the lakes too, takes the `OBJ_0ppp` dummy and the turned heading
  into `startpos[1][f]` (+0x30630).

So the port also had the lake arrival's heading wrong: 1.0 where the
game turns it.

## The fix

`start_of` takes the room's (floor, room). For a lake it only special-cases
the up stairs, with the heading rule above. The down stairs go through
the dummy like every other type.

## Checked

- **Against the game.** `tools/test_dungeon_rt.py` runs the game's code in
  eemu:
  - `test_lake_dressing` and `test_lake_night` now compare `startpos` and
    `GetStartPosition`, over four lakes of type 8 (by day, by night and
    hacked) and two of type 9.
    Before the fix they failed on both halves: w 1.0 against pi, and
    `startpos[1]` zero against the dummy.
  - The new `test_lake_stairs` runs the game's `Enter` on the lake's
    stairs down (`ChangeScene` into dungeon 1, `lastRoom`, `entryFlag[1]`
    cleared). It runs `GO(2)`'s branch through a stub that enters main
    0x001a0cac with `$s1` set and stops at 0x001a0d60, comparing the
    position, `dungeonback` and the room's hits with the probe's new
    `back` (`come_back`). It also runs dungeon 1's stairs up and
    `GoField` in both dungeons; the lake's `GoField` does nothing.
  - Unfixed, it fails at `GO(2)`: the probe at (0, 0, 0), the game at the
    dummy. That answers [[191]]'s "not compared with the game's run".
  - The whole harness passes (11 tests). Its `KiteInDungeon` had been
    failing since [[378]], which added `arms` to the game's frame record
    but not to `dungeon_probe`'s; the probe now prints it.
- **The session** (`session/tests/ocarina.rs`, a Δ lake from
  `dressing::lake_areas`):
  - `the_sprite_ocarina_below_a_lake_returns_to_it` goes down the
    stairs from the room with them, then plays the ocarina from PERSONAL's
    Items with OK on "Return to the field.".
  - `the_stairs_up_below_a_lake_return_to_it` does the same by the stairs
    up.
  - Both check the lake's room left is built, Kite stands at its
    `startpos[1]` and inside that room. Both fail before the fix (Kite at
    0, 0).
  - `a_type_4_area_goes_down_into_its_second_dungeon_and_back`
    (piney-world) now also checks that `come_back` stands in the room.
- **The other exits** were not affected:
  - `the_ocarina_and_the_stairs_out_of_a_dungeon_reach_its_field`: Δ's
    random dungeons of types 0-3 reach their fields by both ways, the
    party 900 from the entrance, before the fix and after.
  - `gate_out_from_a_lake_reaches_the_town`: Gate Out from the lake and
    from below it reaches Mac Anu.
  - The Fairy's Orb changes no scene; the ocarina is the only warp item.
- **Shots.** `ocarina_lake_shots` (ignored).
  - Before: /mnt/data/claude/scratch/issue51/before/lake-0270.png, the
    void and the corner of the map, as in the issue.
  - After: /mnt/data/claude/scratch/issue51/after/lake-0270.png, the
    lake's forest, Kite at its stairs down.
- The suite passes (244).

**Still unknown:** the heading Kite now has on arriving at a lake from
town (turned as `MakeRoom` turns it) was checked against the game's
code, not against a picture. Comparing it would take a PS2 capture of a
lake's arrival. [[343]]'s question, whether `SetItemBox` places an opened
box again on the way back up, stays open.
