---
number: 220
title: Mutation's story areas and its new characters' save
date: 2026-09-27
area: volumes
files: crates/piney-data/src/area/mod.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-world/src/area.rs, crates/piney-game/src/area.rs, crates/piney-data/src/save.rs, tools/test_area_rs.py
---

# 220. Mutation's story areas and its new characters' save

`voldiff ported --volume MUT` counts 296 changed functions among those the
port covers. That count needs care:

- **Mis-carried names.** `ccThEvHold`, `dogAction` and `ccPGuso::adultMain`
  carried their names to functions 5 to 27 times Infection's size. They
  are other functions.
- **Doubled functions.** Many functions roughly doubled because Mutation
  inlines accessors with a branch for characters 18-20.

Two changes behind them matter to the story and are ported here.

## Characters 18-20

Mutation adds Tsukasa (18), Subaru (19) and Sora (20). Their save records
live in a 0x854-byte extension that `ccSaveData`'s constructor allocates.
The port already modelled it (`save::ext`, `save::by_id`), but ten sites
still indexed `ccSaveData`'s arrays by id directly. Past id 17 those
indices land on other members. For example, Sora's `spcParam` would cover
`lastTown`. The sites were:

- the party's `real` and `tune`;
- the HUD's members;
- the story's member announcement;
- the companions' party time (which also skipped ids above 17);
- `ChangeEquipment`;
- the character info;
- experience;
- `SpcParam::from_save` and `store`;
- the party's velocity;
- the party AI's skill and item lists;
- `AddItem`'s list.

They all use `by_id` now, as Mutation's own accessors do. Ids below 18 map
to the same offsets as before.

## The story areas

From Mutation on, `GetEventAreaInfo(n)`, `GetEventAreaInfo()`,
`ccGetEventAreaInfo`, `IsProtectArea`, `GetWordParamFromEvCode` and
`SetDungeonTypeFromField` change in three ways:

- **127 rows, not 126.** Mutation's 127th row is area 126, which the port
  never found (`AreaTables::search`).
- **Two substitute records.** Area 71's replaces the table's while
  `eventFlag[217]` has bit 62 (the same flag Infection's `SimGenerateCode`
  reads). Area 47's replaces the table's from volume 3 on. The generated
  tables already carried both records (`AreaTables::substitute`).
  `SimGenerateCode` no longer writes these two cases inline: Infection's
  enemy offset 78 for area 47 on volume 3 becomes the record's 42.
- **The dungeon types.** They come from the volume's rule
  (`AreaTables::dungeon_rule`). From Mutation on its save flag is
  `eventFlag[314]` bit 62 (+0x5ec8), which sends a story area down the
  random path. Infection's flag is the crisis byte. `SetDungeonTypeFromField`
  used to read Infection's rule on every disc. The flag is now read from
  the save (`area::dungeon_flag`); a story start with no save to read
  gives it the crisis on Infection and leaves it unset after.

## Checks

`tools/test_area_rs.py` now runs on `PINEY_VOLUME`'s code: the file paths
come from `volume`, and the random number generator's globals go through
`va()`. It passes on both Infection and Mutation, 5 tests each:

- `SimGenerateCode` over every story area's words on every server, with
  and without the flags, and 400 random triples;
- `IsProtectArea` and `GetEventAreaInfo` for areas -1 to 130;
- instruction 118;
- `SetGenerateCode`;
- the word IDs.

piney-data, piney-world and piney-game pass. A new test checks
Mutation's 127th row and the substitutes.

**Still unknown:** On Outbreak and Quarantine `GetEventAreaInfo(0)`
returns NULL where the port finds row 0. That waits for their turn. Most
of the other changed functions are still to be read one by one.
