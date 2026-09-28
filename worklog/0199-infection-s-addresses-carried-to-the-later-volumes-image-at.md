---
number: 199
title: "Infection's addresses carried to the later volumes: Image::at; the first reads of Mutation's boot"
date: 2026-09-26
area: volumes
files: tools/main_data.py, tools/voldiff.py, crates/piney-data/src/exe.rs, crates/piney-data/src/main_data, crates/piney-data/src/save/init.rs, crates/piney-desktop/src/kanji.rs, crates/piney-demo/src/dialog.rs, crates/piney-game/src/main.rs, plans/volumes.md
---

# 199. Infection's addresses carried to the later volumes: Image::at; the first reads of Mutation's boot

Phase 4 (each volume boots to its Root Town) needs to see where a later
disc stops. The game refused anything but Infection. `PINEY_TRY_VOLUME=1`
now lets it start one anyway.

Mutation stopped at once, in `ccSaveData::Init`. The port read
`timeIdolRankDefStr` at Infection's 0x0033eb90. Phase 1 had put each
volume's main data in piney-data, but every reader still used Infection's
addresses: in Mutation's bytes 0x0033eb90 holds something else, and its
first word led nowhere. About 540 such constants sit in 80 files.

## The carry

`tools/main_data.py gen` now also writes `<volume>.carry` for MUT, OUT and
QUA. Each is a sorted list of (section, Infection's address, its size,
the volume's address). The rows come from three sources:

- **Names:** every function and global whose name `xfer.py` carried and
  that is unique in its section on both sides. That is about 10,400 rows a
  volume.
- **Votes:** the globals whose names repeat, string literals' `@1114` and
  the jump tables, by what the paired functions build at their aligned
  instructions. This is `xfer.propagate`'s votes, where they agree, about
  2,000 more rows. The title's "#G" (demo `@1114`, 0x0040eba8) lands on
  Mutation's 0x00421ce8, which holds "#G".
- **Content:** the two bitmap fonts, which have no size in Infection's
  symbols. They are byte for byte the same on the four discs.

`Image` holds the carry for the volume `Image::main` was made for, and the
overlay's id from the PRG header (`Overlay::parse`, `with`).
`Image::at(inf_va)` finds the row holding the address and returns the
volume's address at the same offset. On Infection it returns the address
unchanged. An address nothing holds comes back as 0, which no read finds.
`PINEY_CARRY_TRACE` names each such miss.

Only constants go through `at`. A pointer read from the image is already
the volume's. `a_later_volume_carries_infections_addresses` checks:

- `timeIdolRankDefStr` goes to Mutation's 0x00353b70, and an offset into
  it moves with it;
- gcmn's `latticeAttributeColorTable` is found through the overlay;
- an address below main's first global finds nothing.

Now carried: the save's initial strings, the fonts, and the title's
memory-card dialog texts (`STR_*`, the two literals, `saveSysMsg`'s
entries).

## Where Mutation's boot stands

It passes `ccSaveData::Init` and the fonts, then reaches the new game
tables (`NewGameTables::read`). These cannot just be carried:

- Mutation's `charTbl` has 21 rows.
- Its `NewGame` fetches each record through an accessor that sends ids
  18-20 to the save's 0x854-byte extension (save.md, "The extension").

`voldiff.py diff NAME [--volume V]` shows a function's disassembly,
Infection's against the volume's, with the addresses and branch labels
normalised so the two line up. It is the way to read the changed
functions.

**Still unknown:** Mutation's save is not modelled yet: the extension,
the accessors, and `Init`'s, `NewGame`'s and `InitSpcParam`'s changes
(all "changed" in 0198's audit). The tables the port took from running a
static initializer in eemu (`saveSysMsg`, `MAIL_REPLIES`) are carried
entry by entry, not re-derived for each volume. A table a later volume
grew is read with Infection's length.
