---
number: 217
title: The World off the executable image
date: 2026-09-27
area: volumes
files: crates/piney-battle/src/blocks.rs, crates/piney-battle/src/entry.rs, crates/piney-world/src/npc.rs, crates/piney-world/src/rtownpc.rs, crates/piney-world/src/grunty.rs, crates/piney-world/src/map, crates/piney-world/src/lib.rs, crates/piney-gen/src/manifest.rs, crates/piney-game/src/start.rs
---

# 217. The World off the executable image

Mutation's boot reached the desktop (0216) but stopped entering The World.
Every reader named in 0216's list used Infection's addresses in GCMN.PRG,
and on Mutation those addresses hold other data. For example, the spawn
tables read `ccEnemyListInfo` there and followed a pointer to 0x28820050.

Nothing in `piney-world` builds an image any more. `load_exe`, `World::exe`
and `FieldWorld::exe` are gone.

## The spawn tables and the enemies' blocks

- **`SpawnTables::of(volume)`** comes from `battle::of(volume)`:
  - the gimmick rows and the `npcTbl` rows, each a `SpawnRow` of its base,
    its constructor and its `esize`;
  - the enemy lists.

  The constructor is `entry.func` as a generated enum, `EntryFunc`
  (`GimBox`, `RtownMerchant` and the others). The generator names it from
  the symbol.
- **The HUD names.** A generated base has no name pointer, so `Base.label`
  carries the text, and the HUD shows an enemy's, gimmick's or NPC's name
  from it.
- **The gimmick looks** (the portal, the boxes, the idols, the spring) take
  their file, palette and size from the gimmick rows.
- **The races' weapon trails and dust.** `races.rs` held 503 pointers into
  156 blocks (`e1x1WpInfo`, `ezc1DustInfo`...), each a named global at
  offset 0.
  - They are now names: `blocks::InfoRef { table, row }`.
  - The breaths' `ehkBreathInfo`, `elBrInfo`, `ehkBrParam` and `elBrParam`
    are names too, with a row.
  - The turtles' footsteps use `eet1DustInfo` rows 8 to 11.
  - The `combat` group generates every block by name. `named_blocks` finds
    them by their Infection symbols and places them per volume. It also
    generates `footObjTbl`.
  - The world takes typed rows (`WpInfo::of`, `blocks::br_info`), no
    longer reading bytes at an address.
  - The battle probe prints each block's own address (`InfoRef::va`), so
    the harnesses compare the same numbers.
- **The generator's `write()`** lost the outer offset of an array within an
  array: an inner `k` shadowed the outer one. It now names each loop's
  variables by depth.

## The towns and the maps

The `world` group has grown by these tables, all four volumes':

| entry | game |
| --- | --- |
| `rtpc_*` | the walking PCs' files, weapons, clips and chat lines (as text), `markPosTbl`, `merchanNum` |
| `pg_*` | the Grunty's cameras, places and lines |
| `breeder_cam_*` | the Grunt Shop breeders' views |
| `town_icons` | `RT01ICONPOS` to `RT05ICONPOS`, by name |
| `key_icons` .. `tree_icons` | each field type's map icons, as many as the volume's symbol holds |
| `fp_dungeon`, `field_minimap`, `level_str` | the field map's dungeon icon and texture, the dungeon map's floors |

The readers that use them:

- `NpcRow::of(volume, row)` reads the battle's `npcs`.
- The walking PCs (`rtownpc::Tables::of`), the Grunties, the dogs, the
  merchants and the field NPCs take the volume.
- `breeder_view(volume, server)` gives None outside servers 1-4, where the
  game reads the words beside the tables. Only servers 1-4 have Grunt Shops.
- The field, town and dungeon maps.

A walking PC's chat is `PcEvent::Chat(text)`. The world harness now records
the text the game's `OpenChat` got, not its address.

## Dead reads removed

These read main from an empty image and could never succeed:

- `item::category_order` is now `fieldui::category_order`.
- The desktop's unused `Assets.elf`, `prg` and `image`, and
  `content::separated`.
- `Tables::game_image`.

The generated `tables/mod.rs` allows `approx_constant` and `identity_op`:
the numbers are the game's.

## Mutation's story starts

`start::story(volume)` is Infection's 1-31, or Mutation's M201 to its ending
(101-116). `--mode story:N` takes Mutation's events 102-116:

- each event's first located block puts M204 and M205 on the board, M215 in
  Carmina Gade (`in_town 2`, `start::log_in_town`), and the rest on the
  desktop;
- `mutation_story_survey` runs them as `story_survey` runs Infection's.

## Checks

- piney-game: 125 passed, 1 failed, 60 ignored. The failure is
  `mutation_story_starts_open_their_event`: 102-114 open where they should,
  and 115 stops at "town 2: only Mac Anu (0) and Dun Loireag (1) are
  ported".
- `mutation_the_world_and_into_the_town` enters the town with no reader
  failing.
- The harnesses pass against the game:

  | harness | cases |
  | --- | --- |
  | spawn | 6 |
  | enemy motion | 5 |
  | dust | 1 |
  | breath | 1 (24 subtests) |
  | weapon | 1 |
  | foe | 4 |
  | map | 6 |
  | merchant camera | 3 |
  | grunty | 2 (15 subtests) |
  | world | 18 |
  | items | 5 |

- The spawn and enemy motion harnesses had failed since 0210. The port keeps
  an enemy's `anmTbl` as its row plus one, and the harnesses now translate
  the game's pointer to that.

**Still unknown:** Carmina Gade. Town 2 is new in Mutation: its scene
file, its lights and sky, its map (`RT03ICONPOS`, texture and scale), its
walking PCs' marker rows and its merchants (`merchant::TOWN_ROWS` has
Infection's two towns). What M201's fields and dungeons need past area
loading is not yet run, nor are the gate hacks and Data Bugs of Mutation's
story, which `start::fights` knows only for Infection.
