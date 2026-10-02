---
number: 338
title: "Saves from PCSX2: a PS2 card image reader, the import, and Infection slots of 0x8d84"
date: 2026-10-01
area: save, format, tooling
files: crates/piney-data/src/ps2card.rs, crates/piney-desktop/src/card.rs, crates/piney-desktop/src/savesys.rs, crates/piney-game/src/main.rs, docs/formats/save.md, README.md
---

# 338. Saves from PCSX2: a PS2 card image reader, the import, and Infection slots of 0x8d84

Players asked to bring their PCSX2 saves over. The port's card is a
directory laid out as the PS2 card is: `<dir>/icon.sys`, the icons, the
index `<dir>/<dir>` and `dhdata01`-`12` ([the save page](../docs/formats/save.md)).
So a save only has to be taken off PCSX2's card and its files copied
across.

## PCSX2's cards

PCSX2 keeps a card as one of two things:
- a folder card: a directory per save, with PCSX2's own `_pcsx2_index` (a
  YAML-like list of each file's order and times) beside the game's files;
- a card image (`Mcd001.ps2`): the PS2 memory card's file system, 8 MB of
  528-byte pages (512 bytes of data and 16 of ECC).

The folder form needs copying only. The image needs a reader.
`piney_data::ps2card` (new) reads it:
- the superblock: magic "Sony PS2 Memory Card Format ", page size,
  pages a cluster, `alloc_offset`, the root's cluster, `ifc_list`;
- the FAT through its indirect clusters;
- the 512-byte directory entries (the root's own "." gives its count);
- a file through its FAT chain.

ECC is assumed when the image is a power-of-two count of 528-byte pages.
`a_save_directory_reads_back` builds a card with and without ECC (a file
over two clusters, an empty one) and reads the directory back. No real
`.ps2` image was at hand. The reader follows the format as published, and
an image from a real card is still to be tried.

## The import

`piney_desktop::card::import(src, card)` finds the four volumes' save
directories (`own_dir_name` of each, from `mcDirName`) in any of three
sources:
- a `.ps2` image;
- a folder card;
- one exported save directory.

It copies each found directory whole, leaving out `_pcsx2_*`. A directory
it replaces is moved to a fresh `backup-<secs>` beside the card. It is
reached through `piney-game --import-card PATH`, which imports and exits,
and through the console's `import_card PATH`. `a_pcsx2_save_imports` checks
a folder card, the backup, a lone save directory and a source with no save.
The user's export (`work/savecards`, not in the repository) imports its 17
files.

## Infection slots of 0x8d84

The user's export holds Infection's directory (`BASLUS-20267DOTHACK`, the
`icon10/11/12.ico` icons), but its twelve slot files are 0x8d84 (36,228)
bytes. That is the later volumes' size; Infection's own `MakeDir` makes
them 0x8530 (`li 0x8530`, INF SLUS_202.67:0x001660d4). Slots 1 and 12 hold
a cleared game (level 34, `clearFlag` 1). The index record's sum covers the
whole 0x8d84 bytes (slot 1: 0x599, against 0xc31b over the first 0x8530).
The first 0x8530 bytes are the same `ccSaveData`.

Infection's own load (and the port's) summed 0x8530 bytes and would refuse
the slot. At the user's request the port now takes either size. The card's
`read_slot` hands over the whole file, and the load accepts the sum over the
volume's size or, for a longer Infection file, over 0x8d84. It copies the
0x8530-byte record as before. `an_infection_slot_loads_at_either_size`
covers both sizes, a wrong sum, and (when `work/savecards` is present) the
user's slot 1.

**Still unknown:** what wrote 0x8d84-byte slots into Infection's directory
(a later volume, a save tool, or another release); and whether the reader
takes a real PCSX2 `.ps2` image as it takes the built one.
