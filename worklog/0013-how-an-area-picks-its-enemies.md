---
number: 13
title: How an area picks its enemies
date: 2026-09-22
area: battle, world, decomp
files: tools/areas.py, tools/test_areas.py, docs/engine/area-words.md
---

# 13. How an area picks its enemies

[[11]] left open how an area's `areaLevel` and `enemyOfs` choose enemies from
the table of [[12]]. Two functions do it, and `tools/areas.py gen` now prints
the result alongside the area.

`ccRegisterDifficultyEnemy` (`INF SLUS_202.67:0x001b7020`) runs only when
`ccGame.area` is 1 or 2 - a field or a dungeon - and computes three arguments:

- `lsv`, the server, `ccGame.server`.
- `ltp`, a list type: 6 when `ccGame.field` is positive - taken here to mean a
  story area is loaded, which is an inference about how `field` is set - and
  otherwise `WORLD_MAN::GetFieldAttrb()` (`0x001a3830`).
- `lrk`, a rank: `25 * (areaLevel - 1) + 10 + enemyOfs`, floored at 0. In a
  story area whose record sets `enemy`, that value is the rank instead. In a
  dungeon (`area` 2), `ccGame.floor + 1` is added.

So the five area levels start at ranks 10, 35, 60, 85 and 110, and a word's
`enemyOfs` (-10 to +10 in the tables) or the dice (-9 to +10) shift it.

`GetFieldAttrb` maps field type and weather to a list type with a jump table.
Rather than transcribe it, it was run in `tools/eemu.py` for all 110 inputs:

| fieldType | weather 0-3 | 4-5 | 6-9 |
| ---: | ---: | ---: | ---: |
| 0-3 | 2 | 2 | 2 |
| 4 | 3 | 3 | 4 |
| 5, 6 | 1 | 1 | 1 |
| 7 | 0 | 0 | 0 |
| 8 | 5 | 5 | 4 |
| 9 | 3 | 1 | 4 |
| 10 | 3 | 5 | 4 |

`ccRegisterEnemyList(lsv, ltp, lrk)` (`INF gcmn.prg:0x0042ed70`,
`entctrl.cpp`) takes `ccEnemyListInfo[lsv][ltp]` - 5 servers by 7 types of
`ccEnemyList { int *list; int num; }`, each pointing at one of
`enemyList00` .. `enemyList46`, all 130 entries long - and registers
`ccRegisterEnemyRange` consecutive entries from index `lrk`, staying on the
last entry once it reaches the end. `ccInitRegisterEnemy` sets that range to
3 (`gcmn.prg:0x0042e8b4`). Each registered entry is an index into `enemyTbl`,
whose `entry.exist` field (`+0x68`) it sets to 1. Entries with base type 64
go through `ccEntryRaceTbl` and a drain table first; that path is not
modelled.

The first story area, `Bursting Passed Over Aqua Field` on Δ, is list 6 at
rank 0: Goblin, Mad Grass, Disco Knife - Goblins being the first enemies of
the game. `Hidden Forbidden Holy Ground` is rank 5 (Chicken Hand, Sword of
Chaos, Cadet Valkyrie); `Expansive Haunted Sea of Sand`'s first dungeon floor
is rank 12 (Swordmanoid, Magical Goblin, Chicken Hand).

`tools/test_areas.py` re-derives the `GetFieldAttrb` table through the
interpreter and pins the first area's three enemies.

**Still unknown:** the race/drain expansion for type-64 entries; what
`ccGame.field` holds exactly; how the three registered enemies become
encounters (magic portals, `circleOfs`); how `itemOfs` picks treasure.
