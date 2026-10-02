---
number: 346
title: A lone slot file imports with its index record
date: 2026-10-01
area: save, tooling
files: crates/piney-desktop/src/card.rs, crates/piney-game/src/main.rs, README.md
---

# 346. A lone slot file imports with its index record

The user's repro save for [[345]] is one slot file,
`work/savecards/repro/dhdata12` (0x8530 bytes, INF). Put on the card, the
load screen showed the slot unused. That is the game's rule. A slot is used
only through its record in the save directory's index file
(`BASLUS-20267DOTHACK/BASLUS-20267DOTHACK`, twelve 28-byte
`ccSaveDataInfo`s; [the save page](../docs/formats/save.md)): status 1, and
a sum the load checks. A slot file copied in without that record reads as
empty. `--import-card` ([[338]]) took only whole save directories, so it
refused the file.

`card::import` now also takes a lone slot file (`dhdata01`-`dhdata12`, or a
folder holding only such files). `import_slots` writes each into its
volume's directory and rebuilds its record as `ccSaveSys`'s save does
(`SaveSys::write`, 0x00173cf8): status 1, `level`, `clearFlag`,
`parodyFlag`, the name from `plName`, the 16-bit sum of the file's bytes,
`playTime`. A missing directory is first made as `make_dir` makes it,
twelve zeroed slots and a zeroed index. An existing one is copied to the
`backup-` folder first, and the other slots' records are kept.

The size picks the volume. 0x8530 is Infection's. 0x8d84 is any later
volume's, and also an Infection slot as a PCSX2 export can hold it
([[338]]), so it takes the volume of the running disc (the console) or
`--volume` (the command line) and is refused without one.

`a_lone_slot_imports_with_its_record` checks the record (`check_right_info`
passes, name, sum), the other slots left empty, the slot read back, the
0x8d84 case with and without a volume, and a size of no save. The user's
`dhdata12` imported into an empty card: slot 12 used, Kite, level 11,
clear 0, sum 0xd4f7 (the file's own), 6.28 hours.

**Still unknown:** nothing.
