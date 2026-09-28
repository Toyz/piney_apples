---
number: 30
title: The dungeon generator ported to Rust, with its tables as generated data
date: 2026-09-23
area: world, build, test
files: tools/dungeon_tables.py, tools/test_dungeon_rs.py, crates/piney-data/src/dungeon, crates/piney-data/tests/dungeon.rs, crates/piney-data/examples/dungeon_snapshot.rs, docs/engine/dungeon.md
---

# 30. The dungeon generator ported to Rust, with its tables as generated data

[[19]] reproduced `DUNGEON::Generate` in `tools/dungeon.py` and checked it
against the game in eemu. This entry carries it into the port as
`piney_data::dungeon`, following the pattern of [[27]]: the engine's tables
are extracted once into generated Rust, and asset data is read from the disc
at run time. A helper agent did the port; I re-ran its checks.

## The tables

`tools/dungeon_tables.py` reads through `dungeon.py`'s `Data`, which finds
each table through the code that uses it. It writes
`crates/piney-data/src/dungeon/inf.rs` (425 KB, exempt from rustfmt, with a
`--check` mode). It holds:
- the 31 `ROOM_INFO` tables (713 rows), reached through `MakeFloor`'s four
  jump tables. Each row has a model name, an exit mask (`Exits(NORTH |
  UP)`) and a `Rotation`.
- `symroom`, `DungeonName`/`DungeonName2` and `dungeonData`.
- the eight dummy-name patterns `SetAllGim` searches, read from its code and
  asserted equal to `dungeon.py`'s.
- the dungeon-type rule as tables, and a `SaveFlag` enum: Infection's
  `saveData+0x6772` test, and Mutation's bit 62 of `+0x5ec8` ([[25]]).
- the 88 hand-made story dungeons (`EditDungeon`), with 2,794 `ROOMDATA` and
  657 `GIMMICKDATA` rows.

The story layouts are level data rather than engine rules, and make up
264 KB of the file. The user was asked and chose to commit them as port
data, like the placement tables.

The file holds no player-facing text.

## The Rust

- **`generate(&Tables, &Params, &DummySource)`.** Takes the seed, dungeon
  type, `levelMax`/`roomMax`, server, volume, first keyword, field type and
  area code. It returns every floor:
  - the 80 x 80 realmap;
  - the rooms, with position, size, exits and the chosen model, rotation,
    minimap code and dummy rolls;
  - the stairs, restarts and start positions;
  - the Gott statue room, the kept gimmicks and the final RNG.
- **`GenerateError::NoDownStairs`.** Covers the case where the game would
  loop forever.
- **Dummy counts.** Each room model's item-box and `ps` dummies are counted
  from the dungeon's CCS file (`Dummies::load`), following ExtObj as
  `ccMatchIndex` does. The counts are not a table, because they are asset
  data.
- **`story(&Tables, event, index)` and `real_map`.** These are
  `MakeRealMap`, for the story dungeons.

## Checked

`tools/test_dungeon_rs.py` builds an example binary
(`examples/dungeon_snapshot`) and compares it with `dungeon.py` field by
field:
- 360 random dungeons: all 10 types, servers 0-4, volumes 1-4, word 131,
  lake starts, and the five cases `test_dungeon.py` runs in the game;
  1,293 floors, 12,351 rooms, 742 restarts, 339 statue rooms and 15,829 kept
  gimmicks, with **0 mismatches**;
- all 88 story layouts;
- the type rule on 624 inputs per rule;
- the dummy counts of all 18 dungeon CCS files;
- that the generated file is current.

A deliberately changed stair threshold broke 83 of the 360 cases, so the
comparison bites. On integration I re-ran it: 5 tests, OK. The six Rust unit
tests (fixed seeds, zero dummy counts, numbers only) pass.

**Still unknown:** the story dungeons' room models (`MakeRoom(ROOMDATA*)`);
the item boxes, circles and idols `WORLD_MAN::EntryGimmick` places later; the
start positions of rooms without a lake entry (`OBJ_0ppp`); the other
volumes' tables (the tool already runs on Mutation, and each volume would be
one more generated file).
