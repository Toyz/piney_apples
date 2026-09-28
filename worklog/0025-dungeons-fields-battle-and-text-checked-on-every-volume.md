---
number: 25
title: Dungeons, fields, battle and text checked on every volume
date: 2026-09-23
area: world, battle, ui, volumes, test
files: tools/dungeon.py, tools/field.py, tools/battle.py, tools/font.py, tools/test_dungeon.py, tools/test_field.py, tools/test_battle.py, tools/test_font.py, docs/engine/dungeon.md, docs/engine/field.md, docs/engine/battle.md, docs/engine/font.md, docs/content/game-data.md, docs/disc/volumes.md
---

# 25. Dungeons, fields, battle and text checked on every volume

[[21]] ran the area generator on all four volumes. This entry does the same
for the dungeon and field generators of [[19]], the battle rules of [[22]]
and the text rendering of [[23]]: each volume's own code in eemu against the
Python reimplementation. A helper agent did the work, one system at a time.
I re-ran the whole suite and read two of the differences myself. The results
are in the "Other volumes" sections of the dungeon, field, battle and text
pages, and summarised on [the four discs](../docs/disc/volumes.md#systems-across-volumes).

## First, the harnesses

None of the four harnesses ran on the other volumes as they stood. Each
failed for reasons that were not about the game:

- **Structure layouts.** Mutation shrank `DUNGEON` from 0xd3910 to 0xce0
  bytes by moving its per-floor arrays into `new[]` allocations. The size
  was read from `WORLD_MAN::GO`'s allocation, and the new pointer offsets
  from the constructor. `ccSkill`'s creator and target moved by 0x10. The
  field and text structures did not change.
- **Wrong or missing carried names.** MUT's `skillTbl` points 0x11b0 bytes
  into the table, and its `MailTbl` points into code. `skillTbl` is missing
  on OUT and QUA, and `enemyTbl` on MUT. The tools now find every table
  through the code that uses it:
  - `battle.py` through each table's accessor;
  - `dungeon.py` through `MakeFloor`'s jump tables;
  - `font.py` through `Extract`'s and `Disp`'s address loads.
  On Infection this reproduces the symbol table's addresses and DWARF sizes,
  and tests assert it.
- **Emulator reach.** OUT's `WORLD::GetHeight` reached `sceVu0OuterProduct`
  on stack garbage, and eemu does not interpret `lqc2`. The height feeds
  only z, which is not compared, so it is now stubbed on every volume.

## Then, the results

0 mismatches everywhere:

| system | per volume (MUT, OUT, QUA) |
| --- | --- |
| dungeons | 120 random dungeons (about 400 floors, 4,100 rooms), 90 hand-made layouts, 12,288 dungeon-type inputs |
| fields | 110 Init pairs, 33 random fields, 112 story areas |
| battle | 1,000 cases of each of 13 checks |
| text | the full `font.py check`: about 19,700-20,000 strings, 3,150 `Disp` calls, 65,536 extended codes |

Real differences, each now read from the executable rather than keyed on
the volume:

- **Dungeons (MUT on):**
  - The floor count is a field, 15 for area 125 instead of the constant 10.
  - `EditDungeon` grows to 90 entries, adding areas 118 and 119.
  - `SetDungeonTypeFromField` uses [[21]]'s substitute records. It swaps the
    `saveData+0x6772` test for bit 62 of `saveData+0x5ec8`, which sends story
    areas down the random-area path with plain types.
- **Fields:**
  - OUT and QUA compute distances with `sqrt.s` rather than `sqrtf`. No case
    differs.
  - From MUT on, area 100 is no longer protected.
- **Battle (MUT on):**
  - Exdefense now keeps only the bits whose defence is not lowered, rather
    than all or nothing (MUT `gcmn:0x005933f8`).
  - An area skill no longer stops when its aimed target is dead (read, not
    run).
  - There are 21 party members.
  - The tables are rebalanced: stronger summons, 127 enemy rows changed,
    Kite starting at level 30, name fixes.
- **Text (OUT on):** `ccKanjiStrlen` counts every `%x` pair as a glyph. I read
  the branch myself:
  - `INF SLUS_202.67:0x0015f1d0` adds 1 only for `%#` (and `%%`);
  - `OUT SLUS_205.63:0x0015e260` adds 1 for any other pair.
- **Unchanged:** `ChooseRoomSize`, `FOOT`, `MakeRoom`, `SetAllGim`, every
  field constant and object table, the fonts, the trims, the colours,
  `expCalcTbl` and the erosion table.

Each test file gained a `TestOtherVolumes` class that runs a few cases per
extracted volume and skips otherwise. The full suite, 14 files, passes with
each file under two minutes.

**Still unknown:** what the save bit at `saveData+0x5ec8` bit 62 and
`DUNGEON.ishack` mean; `MakeFloor`'s story path with more than 10 floors
(read, not modelled); `ccSkillDamage`'s area path; the Data Drain drop roll;
whether hardware `sqrt.s` rounds the way eemu models it.
