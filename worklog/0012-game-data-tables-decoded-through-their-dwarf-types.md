---
number: 12
title: Game data tables, decoded through their DWARF types
date: 2026-09-22
area: battle, content, tooling
files: tools/tables.py, tools/test_tables.py, docs/content/game-data.md
---

# 12. Game data tables, decoded through their DWARF types

The game's balance data - enemies, skills, items, equipment, bosses, party
characters, level-up growth - is compiled into the overlays as C arrays of
structs, and the DWARF ([[6]]) describes every one of them down to the field
names. Rather than write a dumper per table, `tools/tables.py dump` decodes
any global through its DWARF type: it walks the element struct member by
member, reads `char *` and `char[]` as Shift-JIS text, shows other pointers as
the symbol they point at, flattens nested structs (`param.elm.battleAbility.pAtk`),
extracts bit fields (counting from the least significant bit, as CodeWarrior
does), and runs the overlay's constructors first with `tools/eemu.py` ([[9]])
so start-up patches are included. Arrays declared without a length
(`ccEnemyTable enemyTbl[]`) take it from the ELF symbol's size.
`tables.py list` shows every struct array in an overlay.

## What came out

All from `INF gcmn.prg` except `charTbl` (`demo.prg`):

| table | type | rows | bytes each |
| --- | --- | ---: | ---: |
| `enemyTbl` | `ccEnemyTable` | 303 | 448 |
| `skillTbl` | `ccSkillParam` | 304 | 56 |
| `BossSkillTbl` | `ccSkillParam` | 61 | 56 |
| `itemTblE` | `ccItemParam` | 291 | 20 |
| `itemTblD` | `ccItemParam` | 72 | 20 |
| `equipmentWeapon1Tbl` .. `6Tbl` | `ccEquipmentParam` | 82, 77, 97, 75, 74, 76 | 72 |
| `equipmentHeadTbl`, `Body`, `Arm`, `Leg` | `ccEquipmentParam` | 69, 68, 67, 68 | 72 |
| `bossTbl` | `ccBossParamData` | 49 | 104 |
| `LevelUpParamTbl` | `ccLevelUpTbl` | 18 | 36 |
| `charTbl` (demo) | `ccSpcParamData` | 18 | 92 |
| `storeCondition` | `ccStoreCondition` | 18 | 112 |
| `EditDungeon` | `EDITDUNGEON` | 88 | 24 |
| `bookItemList` | `BOOKITEMDATA` | 189 | 8 |
| `spcAIParam` | `ccAIParam` | 18 | 36 |
| `_g_cinemaSkillName` | `CINEMASKILLNAME_T` | 72 | 12 |

Every enemy row carries its name, CCS model file, type, level, experience,
gold, collision height and width, HP/SP, physical and magical attack, defence,
hit and evasion, six elemental attributes (soil, water, fire, wind, thunder,
dark), two tolerances, drain/critical flags and more - 137 columns. The first
three are Razine (`E1X1`, level 0, 50 HP), Swordmanoid (`E1F1`) and Gladiator
(`E1FA`). The model names are the `DATA.BIN` file names of [[5]], so every
enemy links to its model. 279 distinct names over 303 rows; the three
highest-level rows (level 95-99) are named `San=Hi/`, `AAAAAAA` and `DDDDDDD` -
placeholders or deliberately corrupted monsters, not yet told apart.

`charTbl` is the party: Kite (`ctu1body`, level 1, 63 HP), Mia, Orca (level
50, 1,050 HP), Marlo, Sanjuro, Nuke Usagimaru, Balmung, Moonstone, Piros,
Wiseman, Elk, Natsume, Rachel, Gardenia, Terajima Ryoko, BlackRose, Mistral,
Helba - 18, as `LevelUpParamTbl` and `spcAIParam` also have 18 rows.
`skillTbl` opens `NOTHING`, `ATTACK`, `Data Drain`, `Drain Arc`, `2128
Drain`; `itemTblD` holds the spell scrolls (`Raining Rocks`, `Gaia's Spell`,
`Meteor Strike`); weapon table 1 is the twin blades (`Amateur Blades`, model
`cwdhsw02`).

`bossTbl` has 49 entries: the eight Phases (Skeith, Inis, Magus, Fidhell,
Gorre, Macha, Tarvos, Corbenik), Corbenik's later forms, Cubia and its cores,
four Gomoras four times over, and nine entries at the end whose names were
never translated (`タルボス`, `コルベニクシード`, `ヘルシーカー`, `コルベニク２`
...). Like the mail ([[9]]) and the story areas ([[11]]), the Infection disc
carries data for all four volumes.

The dumps are TSV files under `work/infection/tables/` (not committed);
`tools/test_tables.py` pins the item count, the first party member and the
first three enemies.

**Still unknown:** what the enemy table's `type`, `Exdefense`, `entry.*` and
AI fields drive; how `enemyList00`..`enemyList15` and the area's `enemyOfs`
([[11]]) pick rows; skill and item effect fields; the `msg` pointer in the
character and boss rows; whether the placeholder enemies can appear.
