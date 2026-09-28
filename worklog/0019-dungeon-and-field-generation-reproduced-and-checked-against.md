---
number: 19
title: Dungeon and field generation, reproduced and checked against the game code
date: 2026-09-22
area: world, decomp, tooling, test
files: tools/dungeon.py, tools/field.py, tools/test_dungeon.py, tools/test_field.py, tools/eemu.py, docs/engine/dungeon.md, docs/engine/field.md
---

# 19. Dungeon and field generation, reproduced and checked against the game code

[[11]] took three keywords to an area's `fieldSeed`, `dungeonSeed[3]` and
attributes. This entry takes those seeds to the actual dungeon layouts and
field terrain. `tools/dungeon.py` and `tools/field.py` reproduce the game's
generators, and both are checked the way [[11]] was: by running the game's
own code in `tools/eemu.py` and comparing every field it writes. The
algorithms are on [the dungeon page](../docs/engine/dungeon.md) and
[the field page](../docs/engine/field.md).

## Dungeons

`WORLD_MAN::GO` (`INF SLUS_202.67:0x0019f8e0`) seeds the area RNG with
`dungeonSeed[game.dungeon]` and resets `randcnt`, and `DUNGEON::Generate`
(`INF gcmn.prg:0x005c12a0`) grows each floor from a medium, small or large
room at the map centre: rooms spawn through their exits, each new room
drawing its exit count and sides, until the floor has
`fieldrand(5) + roomMax` rooms; a floor that stalls, or ends with fewer than
two dead ends, starts over without rewinding the RNG. The down stairs go in a
dead-end room that passes a 9% roll, the up stairs in room 0, and a dead-end
medium room on the last floor becomes the Gott statue room. Every room then
picks a model from a table by its exits and size, and every item-box dummy in
that model rolls to be kept - which means each floor's layout depends on how
many item-box dummies the earlier floors' models happen to contain.
`dungeon.py` counts them from the models themselves, following the ExtObj
indirection of [[10]].

`WORLD_MAN::SetDungeonTypeFromField` (`0x0019cf50`) picks the dungeon type,
and so the CCS set (sd1 ... sda), from the field type, with overrides for
some story areas. Story dungeons are not generated at all: `MakeRealMap` lays
them out from `ROOMDATA` tables selected through `EditDungeon`.

Checked in eemu, with only asset loading and drawing stubbed: **300 random
dungeons, 0 mismatches** - 150 from random keyword triples on all five
servers, 150 from raw inputs across volumes 1-4 including the forced-four-floor
word 131 - covering 1,047 floors, 10,952 rooms, 759 floor restarts, all ten
dungeon types and 297 statue rooms. Every floor's map, stairs, room models,
rotations, positions, minimap codes, start orientations, statue fields, every
kept gimmick, and the final RNG state and count are compared. All 88
`EditDungeon` layouts over 10 floors, 0 mismatches; all 480 inputs to
`SetDungeonTypeFromField`, 0 mismatches. The item-box counts fed to the game
code come from `dungeon.py`'s own counting, so the counting itself is only
checked through the layouts it produces.

## Fields

`WORLD::Init` burns a fixed number of draws on snow and smoke (measured for
all 110 field type and weather combinations), then `WORLD::Generate`
(`INF gcmn.prg:0x005a6da0`) lays white noise from `fieldrand(64)`, raises one
large and three to eight small fractal hills (a 5x5 grid of random keys,
spline-interpolated), places the dungeon entrance and a lake, scatters key,
sub, base and tree objects with per-object placement tests, lays ground cover
and picks the start. The hills are floating-point, so `tools/eemu.py` now
interprets the EE FPU - including its non-IEEE rules: no denormals, no
infinities or NaNs, saturation on overflow, truncation toward zero, and
`madd` rounding the product first.

Checked in eemu against the real `WORLD::Generate`, comparing the height map
bit for bit, the four chip grids, every object, every cover tile, the start
position and the RNG state: the first story area identical, the unit tests
over five field types, and a bulk run of **150 cases, 0 mismatches** - 105
story areas and 45 random keyword triples, every field type but 4 (which has
no field), 15,171 objects in all.

## The first story area, generated

`Bursting Passed Over Aqua Field` ([[11]], event 14): its dungeon is the
hand-made D0001, 2 floors of 5 rooms; its field, from `fieldSeed` 1420855,
is type 10 with 6 hills (heights 0-766), a lake, 107 objects, 1,550 cover
tiles, and the dungeon entrance within 8,000 units of the start as that area
requires. The outputs are in `work/infection/areas/`.

**Still unknown:** the treasure, magic circles and idols placed afterwards
by `WORLD_MAN::EntryGimmick` on the entry thread - by then per-frame code has
drawn from the same RNG, so they are probably not derivable from the seed;
story-dungeon room models; object heights; why dungeon types 8 and 9 pick a
down-stairs room on their single floor; `EVENTAREA_INFO.protect[]` beyond
index 1; `sd4a.cmp`, named by `DungeonName2` but not in `DATA.BIN`.
