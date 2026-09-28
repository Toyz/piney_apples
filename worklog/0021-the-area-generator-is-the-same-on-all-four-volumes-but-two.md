---
number: 21
title: The area generator is the same on all four volumes, but two story areas moved into data
date: 2026-09-22
area: world, volumes, test
files: tools/areas.py, tools/test_areas.py, tools/elf.py, docs/engine/area-words.md
---

# 21. The area generator is the same on all four volumes, but two story areas moved into data

[[20]] ended with the question of whether anything beyond the containers
changed in the later volumes. The area generator of [[11]] is the first
system to be checked: with the names carried over, each volume's own
`WORLD_MAN::SimGenerateCode` can run in `tools/eemu.py` against
`tools/areas.py` reading that volume's tables. The reference is updated in
[the area keywords page](../docs/engine/area-words.md#other-volumes).

## A first check of the carried names

`areas.py` needed no change to read the other volumes: `word_a1`..`word_c4`,
`eventAreaInfo`, `dungeonData` and the rest all arrived as `data` names. One
of them is a free independent check of [[20]]'s data pass: `volumeNum` reads
1, 2, 3 and 4 in the four executables (`INF SLUS_202.67:0x0034bbf8`,
`MUT SLUS_205.62:0x00363578`, `OUT SLUS_205.63:0x0035bd08`,
`QUA SLUS_205.64:0x002509a8`).

## The one mismatch

With the harness of [[11]], generalised to take any executable, every story
area and 100 random keyword triples matched on INF and MUT. OUT and QUA
matched on everything except area 47 (`Merciless Grieving Furnace`, Λ):
the game gave `enemyOfs` 42 where `areas.py` gave 78.

Infection writes the two special story areas inline in `SimGenerateCode`
(`0x0019ec98`):
- area 71 with bit 62 of `saveData+0x5bc0` set gets enemy and item offsets
  of 119;
- area 47 with `volumeNum >= 3` gets 78 and 122.

Outbreak's `SimGenerateCode` has neither. Instead `ccGetEventAreaInfo`
(`OUT SLUS_205.63:0x001af500`) tests the same two conditions and returns a
different `EVENTAREA_INFO`: 0x00324e20 for area 71, 0x00324e80 for area 47.
Each is a copy of the table's record. Area 71's has enemy and item 119.
Area 47's changes only `item`, to 122, and keeps the table's `enemy` of 42.
So the 78 in Infection's code was never used by any shipped game: Infection's
`volumeNum` is 1, and by volume 3 the value comes from the record.

Mutation already has the substitutes (`MUT SLUS_205.62:0x001b9330`, records
0x0032ea00 and 0x0032ea60), which is why it matched: at volume 2 neither the
inline nor the substitute path fires for area 47. `WORLD_MAN::GetEventAreaInfo`
(`MUT 0x001b1f10`) returns the same substitutes. `ccRegisterDifficultyEnemy`
reads a story area's `enemy` through it, so from Mutation on, area 71's enemy
rank under the save flag also comes from the substitute. That is read from
the code; the enemy registration has not been run on any volume.

`areas.py` now:
- reads `volumeNum` and uses it as the default volume;
- finds the substitute records by scanning `ccGetEventAreaInfo` for
  `li $v1, CODE` followed by an address built in `$v0`;
- routes both `generate` and `enemies` through one `event_info` lookup that
  applies them under the game's conditions;
- keeps Infection's inline values for an executable that has no
  substitutes.

## Checked

Each volume's own `SimGenerateCode` against `areas.generate`, in eemu, over:
- every story area with an address;
- areas 71 and 47 again, with the save flag set, with `volumeNum` forced to
  3, and with it forced to 4;
- 100 random triples.

INF gave 218 cases (its area 94 cannot be typed), MUT, OUT and QUA 219 each:
**0 mismatches**.

`tools/test_areas.py` now does the same:
- Infection with 60 random triples;
- every other extracted volume with a `.syms` sidecar with 20, also
  asserting its `volumeNum` and its substitute records.

## What else changed in the tables

The 305 keyword IDs and the 113 addressed story areas are the same set on
every disc. The text changes, though:
- MUT: keyword 304 `Beginning` becomes `Darkside`, and `Howling` becomes
  `Barking`, which makes area 94 typeable for the first time.
- OUT: `Evil Eyed` becomes `Evil-eyed`, `Sun Colored` `Sun-colored`, and
  `Grave Stone` `Gravestone`.
- QUA: `Vengeful` becomes `Vindictive`.

Mutation also fills in the Σ and Ω story areas that Infection left unset:
- 19 `type` and 19 `bgnum` values go from 255 to a value;
- area 76's `type` goes from 9 to 7;
- `enemy` changes in 18 areas and `item` in 16.

This fits the servers opening in later volumes (inference). Mutation also
adds an area-126 record with no address.

`tools/elf.py` now closes the file it reads, which the new test's four
executables turned from a quiet leak into a warning.

**Still unknown:** whether `ccRegisterDifficultyEnemy`, `DUNGEON::Generate`
and `WORLD::Generate` also match on the other volumes (only
`SimGenerateCode` has been run there); what the save flag at
`saveData+0x5bc0` bit 62 records; why area 47 gets a different item offset
from volume 3; what area 126 is.
