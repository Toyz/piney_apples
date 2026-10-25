---
number: 482
title: A lake's statues and flowers already take its palettes: EntryObject's ChangeClut is the file's clut_swaps
date: 2026-10-25
area: render, world, test
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/dungeon_area/dress.rs, docs/engine/dungeon.md
resolves: 283.3
started: 2026-10-25T17:20:55-06:00
took: 2m
---

# 482. A lake's statues and flowers already take its palettes: EntryObject's ChangeClut is the file's clut_swaps

[[157]] and [[161]] left `EntryObject`'s palettes for the lakes' statues
and flowers "not ported", and [[283]] carried it as a play question: lake
dungeons by evening and night. The code says the port already draws them
so.

`DUNGEON::EntryObject(dummy, f, i, name, num)` (INF gcmn 0x005c6e80) ends,
for types 8 and 9 with `WORLD_MAN::GetBG()` not 0 (0x005c7030), with
`ccClump::ChangeClut(+0x24c[k], +0x14c[k])` over the pairs until the first
null (0x005c704c-0x005c707c). Those are the two lists `SetClutList`
(0x005c1440) fills from the type's `CLT_` names and their "c1"-"c3" twins,
the same pairs `SetRoom` gives the room's anm. `ccModel::ChangeClut` writes
the palette into the material, and a file's materials are one set for all
its models (`ccMmat::Init` takes the chunk's object, as [[480]] found for
the lake's water). So after either call, every model of the dungeon's file
that uses one of those palettes draws swapped.

The port builds those pairs once (`DungeonArea::clut_swaps`, from
`clut_list(dtype, clutType, GetBG)`, stopping at the first missing name)
and passes them to every draw of the file: the room, the waters, the
doors, the walls, and the statue and flower clumps (`draw_clump`). Under
`GetBG` 0 the list is empty, as `SetClutList` is not called.

`a_lakes_statues_take_its_palettes` builds a field-type-4 dungeon under
`GetBG` 0 to 3. At 0 nothing is swapped. At 1-3 it is a lake (8 or 9),
and all eight clumps `SetObject` can place (`CMP_o_statue_l0_` to `l3_`,
`CMP_o_flower_l0_` to `l3_`) have a palette on the list. No code changed;
the docs' "not ported" was stale.

The other steps of the clock: piney-world tests, clippy and fmt clean.

**Still unknown:** nothing
