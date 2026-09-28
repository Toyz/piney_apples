---
number: 200
title: "The later volumes' save: the extension, Init and NewGame checked against their own code; Mutation reaches its title"
date: 2026-09-26
area: volumes
files: crates/piney-data/src/save.rs, crates/piney-data/src/save/init.rs, crates/piney-data/src/sinit.rs, crates/piney-data/src/exe.rs, crates/piney-data/src/volume.rs, crates/piney-demo/src/newgame.rs, crates/piney-demo/src/names.rs, tools/sinit_tables.py, tools/main_data.py, tools/test_save_init_rs.py, docs/formats/save.md, plans/volumes.md
---

# 200. The later volumes' save: the extension, Init and NewGame checked against their own code; Mutation reaches its title

0199 carried Infection's addresses. Mutation's boot then stopped at the
new game's tables. From Mutation on there are 21 characters, and the last
three live in a 0x854-byte extension beside `ccSaveData` (save.md,
"The extension").

## The record

`SaveData` now always holds the extension after its 0x8530 bytes, as a
later volume's slot file does. On Infection it stays zero and is never
written: `bytes()` and `sum()` still cover `ccSaveData` only, and
`slot_bytes(volume)` gives each volume's file. `save::ext` names the
extension's members. `save::by_id` routes a character's records as the
accessors do (ids 18-20 to the extension). `Image` remembers its volume,
and `InitText` carries it, so `SaveData::boot(text)` knows whose `Init`
to run.

## Init

The first comparison ran Infection's and Mutation's boot records in eemu.
They differed in 14 bytes of `ccSaveData` plus the extension. That hides
every zero write, so the second run was each `Init` alone on
0xAA-filled and 0x55-filled records. It found five differences (save.md
lists them):

- the 21 characters' lists through the accessors;
- the talk counts through their setter;
- the trade counts through a new (kind, code) setter;
- `partyTime` and `spcPresent` through setters for ids 0-19;
- new blocks cleared at +0x8432 and +0x8462, and the extension's tail.

Three things the disassembly showed:

- The setters count from character 1, so the game's loop from 0 writes
  the word before each array: the last entry of `enemyKillArea`.
- The trade count setter moves NPC codes 181, 180, 182, 183, 121 and 120
  into the extension at +0x74b. save.md had guessed those six bytes were
  sound settings.
- Outbreak and Quarantine run the same `Init`.

`test_save_init_rs.py`'s `InitLaterVolumes` runs each later volume's own
`Init` (flags 0 and 1; zeroed, 0xff and random records and extensions).
It compares every byte with the port's: 0 differences on MUT, OUT and
QUA.

## NewGame

- `charTbl` has 21 rows. The later trade tables hold 20 characters' and
  54 NPCs' lists. The carried symbol sizes are Infection's, so each
  table's base is carried and the rows are counted by the volume
  (`Volume::characters`).
- A Parody Kite's level is 20, 50, 70 or 90 by `volumeNum`.
- `SetDefaultWord` is one routine on every disc, keyed on `volumeNum`.
  Infection's copy, run in eemu with 1-4, gives each volume's bits.
- `NewGame(0)` gives Kite important items, more with each volume.
  Quarantine also sets four news entries and clears seven mails.

Outbreak and Quarantine moved `charTbl` from DEMO.PRG into main. The
carry now looks for an overlay's global in the volume's main when its
overlay lacks it.

`NewGameLaterVolumes` compares the title's part of a new game with the
game's for all three volumes, with and without Parody: 0 differences.
That covers the boot, `NewGame(1)`, the flag, `NewGame(0)`, the name
pointers and both parts of the record.

## The rest of the boot so far

- **Static initializers.** `saveSysMsg` is filled by
  `__sinit_sdmng.cpp`, so the port had held Infection's table.
  `tools/sinit_tables.py` runs each volume's own initializer in eemu and
  writes `piney_data::sinit`. Infection's generated table equals the old
  one.
- **Direct reads.** Every direct read of an Infection address constant
  (218 in 50 files) now goes through `at`, applied mechanically. The
  desktop's readers that pass constants through closures and helpers
  were done by hand.
- **The title's scene.** It is `title1`-`title4` by volume (`SetVolCcs`).

Mutation now boots to its title: its copyright line and memory-card
question. The logo and menus do not draw yet. Infection's own overlay
switches eight opening functions on the volume (the control's +8) and
holds every volume's animation names. The port took only volume 1's.

**Still unknown:** The later titles' menus (more items: the previous
volume's save) are not ported. The desktop's tables (mail, news,
wallpaper, music) are read with Infection's row counts. What reads the
blocks at +0x8432 and +0x8462, and the extension's last 0x100 bytes, is
not known.
