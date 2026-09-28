---
title: Game data tables
status: partial
volumes: all
covers: INF gcmn.prg enemyTbl, skillTbl, BossSkillTbl, itemTblE, itemTblD, equipment*Tbl, bossTbl, LevelUpParamTbl, storeCondition, EditDungeon, bookItemList, spcAIParam; INF demo.prg charTbl
worklog: 12, 13, 22, 25, 67, 78, 208, 275
---

# Game data tables

The balance data compiled into the overlays. Every table here is an array of
structs whose layout the DWARF gives in full, so the field names are the
original ones; `tools/tables.py dump ELF NAME --overlay gcmn` prints any of
them. Tables are read after the overlay's constructors have run.

## Inventory

| table | overlay | VA | element | rows | size |
| --- | --- | --- | --- | ---: | ---: |
| `enemyTbl` | gcmn | 0x005f1e70 | `ccEnemyTable` | 303 | 448 |
| `skillTbl` | gcmn | 0x0061f4a0 | `ccSkillParam` | 304 | 56 |
| `BossSkillTbl` | gcmn | 0x00696ce0 | `ccSkillParam` | 61 | 56 |
| `itemTblE` | gcmn | 0x00624350 | `ccItemParam` | 291 | 20 |
| `itemTblD` | gcmn | 0x00623900 | `ccItemParam` | 72 | 20 |
| `equipmentWeapon1Tbl` | gcmn | 0x0063f560 | `ccEquipmentParam` | 82 | 72 |
| `equipmentWeapon2Tbl` | gcmn | 0x00640c70 | `ccEquipmentParam` | 77 | 72 |
| `equipmentWeapon3Tbl` | gcmn | 0x00642220 | `ccEquipmentParam` | 97 | 72 |
| `equipmentWeapon4Tbl` | gcmn | 0x00643d70 | `ccEquipmentParam` | 75 | 72 |
| `equipmentWeapon5Tbl` | gcmn | 0x00645290 | `ccEquipmentParam` | 74 | 72 |
| `equipmentWeapon6Tbl` | gcmn | 0x00646760 | `ccEquipmentParam` | 76 | 72 |
| `equipmentHeadTbl` | gcmn | 0x0063a8d0 | `ccEquipmentParam` | 69 | 72 |
| `equipmentBodyTbl` | gcmn | 0x0063bc40 | `ccEquipmentParam` | 68 | 72 |
| `equipmentArmTbl` | gcmn | 0x0063cf60 | `ccEquipmentParam` | 67 | 72 |
| `equipmentLegTbl` | gcmn | 0x0063e240 | `ccEquipmentParam` | 68 | 72 |
| `bossTbl` | gcmn | 0x006130b0 | `ccBossParamData` | 49 | 104 |
| `LevelUpParamTbl` | gcmn | 0x005d1800 | `ccLevelUpTbl` | 18 | 36 |
| `spcAIParam` | gcmn | 0x00653ca0 | `ccAIParam` | 18 | 36 |
| `storeCondition` | gcmn | 0x0072fb00 | `ccStoreCondition` | 18 | 112 |
| `EditDungeon` | gcmn | 0x00695cd0 | `EDITDUNGEON` | 88 | 24 |
| `bookItemList` | gcmn | 0x005d2f40 | `BOOKITEMDATA` | 189 | 8 |
| `_g_cinemaSkillName` | gcmn | 0x005eb040 | `CINEMASKILLNAME_T` | 72 | 12 |
| `charTbl` | demo | 0x0040dc80 | `ccSpcParamData` | 18 | 92 |

`enemyTbl`, `skillTbl` and `BossSkillTbl` are declared without a length; the
row counts come from their ELF symbol sizes (135,744, 17,024 and 3,416 bytes).

## Shared records

Enemies, bosses and party characters share a base record, shown flattened as
`base.*` (`param.base.*` in `enemyTbl`):

```
name        char *      display name
ccsname     char *      the model's CCS file, a DATA.BIN name without ".CCS"
type        s32
id          s32
level       s32
exp         s32
gold        s32
height      f32         collision
width       f32         collision
msg         pointer     the address of the row's message table
```

followed by `maxHP`, `maxSP`, and `elm`: `battleAbility` (`pAtk`, `pDef`,
`pHit`, `pEva`, `mAtk`, `mDef`, `mHit`, `mEva`), `attribute` (`soil`,
`water`, `fire`, `wind`, `thunder`, `dark`) and `tolerance` (`spirit`,
`body`). Equipment carries the same `elm` block plus `name`, `ccsname` and
`auraSW`. Print `tools/dwarf1.py type SLUS_202.67 ccEnemyTable` for the full
layouts.

What the fields drive is on the engine pages. The enemy `type` bits,
`Exdefense` (+0x64), the AI fields, `entry` and the skill and item effect
fields are in [battle](../engine/battle.md) (the skill types, hit and
damage, enemy AI, items). How an area's `enemyOfs` and the lists
`enemyList00`..`enemyList46` pick the rows it registers is in
[area words](../engine/area-words.md) and in battle's "Where the entries
come from"; story areas take column 6.

## Content

- Party (`charTbl`, 18): Kite, Mia, Orca, Marlo, Sanjuro, Nuke Usagimaru,
  Balmung, Moonstone, Piros, Wiseman, Elk, Natsume, Rachel, Gardenia, Terajima
  Ryoko, BlackRose, Mistral, Helba.
- Bosses (`bossTbl`, 49): Skeith, Inis, Magus, Fidhell, Gorre, Macha, Tarvos,
  Corbenik and later forms, Cubia and its cores, the Gomoras, and nine entries
  whose names are still Japanese.
- Enemies: 303 rows, 279 distinct names; three rows at level 95-99 are named
  `San=Hi/`, `AAAAAAA`, `DDDDDDD`.

## Other volumes

The same tables exist in every volume. `tools/battle.py` finds them from
their accessor functions, since the stripped volumes' carried names are
missing or wrong for some. For example, MUT's carried `skillTbl` points
0x11b0 bytes into the table, whose real start is `0x0064e2f0`. What changes:

- **INF to MUT:**
  - `charTbl` and `LevelUpParamTbl` go from 18 to 21 rows; Kite now starts
    at level 30.
  - `skillTbl`, 33 rows: the summons hit harder (atk 150/250/350/450 become
    250/350/450/550), the Kruz element goes from 80 to 100, and Meooow's
    cost goes from 20 to 70.
  - `BossSkillTbl` goes from 61 to 79 rows, with display names instead of
    internal ones.
  - `enemyTbl` changes in 127 rows ("Temple Night" becomes "Temple Knight")
    and `bossTbl` in 27 ("Inis" becomes "Innis").
  - Armour changes mostly as smaller pHit/pEva penalties. Weapons 1, 4 and 6
    each gain a row: "Last Betrayal", "Fate Encounter", "Ludicrous".
- **MUT to OUT:** `skillTbl` 20 rows (the Don/Zot `dmgRate`s go up),
  `enemyTbl` 84 rows, 10 equipment and 51 item name fixes, and `charTbl` 15
  rows.
- **OUT to QUA:** only `charTbl`, in 14 rows.
- `expCalcTbl`, `ccEntryRaceTbl`, `dataDrainErosionTbl` and the erosion table
  are identical in all four.

## Unknown

Nothing.
