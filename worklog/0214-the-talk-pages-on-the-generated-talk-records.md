---
number: 214
title: The talk pages on the generated talk records
date: 2026-09-27
area: volumes
files: crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/menus/breeder.rs, crates/piney-fieldui/src/tables.rs, crates/piney-fieldui/src/lib.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/talk.rs, crates/piney-data/src/tables
---

# 214. The talk pages on the generated talk records

The field UI no longer reads GCMN.PRG's image at run time. Its talk pages
were the last part that did: they read the people's lines, `npcTbl` and
the breeder's food table from the PRG over an empty executable. That
image has no main, so the trading PCs' lines (in main's `.sdata`) came out
as `errorData`: the six trade and shop harness failures of 0210.

## What the pages read now

- **The records** come from the `talk` group (0211) by the address the
  save and `npcTbl` hold:
  - `TalkTexts::record(p)` finds the record array that holds `p`;
  - `TalkTexts::word(p)` finds the pointer table that holds `p`;
  - `has_text(p)` tells a record with text from an empty one (the chain
    to `errorData`).
- **An NPC's base** (`Base::npc`) is its `battle::npcs` row.
- **`charTbl`'s names** are `newgame::chars`, so a later volume gets its
  own 21 rows. The fixed Infection read at DEMO.PRG 0x0040dc80 is gone.
- **The breeder:**
  - `breedTeachMsgTbl` and `pgEvoMsg` are the volume's
    (`fieldui::breed_teach_va`, `pg_evo_msg_va`);
  - `foodTbl` is a new entry (`fieldui::food`, `FOOD_PARAM`).

The generator now writes a DWARF name in capitals, such as `FOOD_PARAM`,
as `FoodParam`.

## A pointer table read as a record

An NPC past the PCs (row 80 on) whose `type` sends the talk page to its
last branch opens `msg + 12 * talkNum` as a record. `msg` there is
`pcMsg0`, a pointer table, so the game reads its words as a record:

- `emode` is the address of `pcMsg0_00`;
- `str` is the address of `pcMsg0_20`, a record array whose bytes it
  shows as text.

The shop harness checks this case. To rebuild those bytes, each record
array in `talk` now also lists the addresses its records hold
(`RecordVa`: `name`, `str`). `TalkTexts::bytes_at` lays a record array's
words (`emode`, `name`, `str`) and a pointer table's words back into
bytes. `separate` then cuts them as `ccKanjiStrSeparate` does, one byte
for 0x20-0x7f and two for anything else. `talk.rs` grows from 0.9 MB to
1.4 MB.

## Checks

The field UI harnesses all pass against the game:

| harness | cases |
| --- | --- |
| main | 54 |
| talk | 33 |
| trade | 10 |
| shop | 31 |
| personal | 17 |
| option | 12 |

The crate's own tests pass.

**Still unknown:** piney-game's session tests still stop at the world's
and the areas' loads (`The World: ... not in the image`). Those readers,
and the Python generators behind their tables (`statics.py`,
`area_tables.py`, `dungeon_tables.py`, `field_tables.py`), are next.
