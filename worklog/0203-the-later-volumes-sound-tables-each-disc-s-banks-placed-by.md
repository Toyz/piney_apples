---
number: 203
title: "The later volumes' sound tables: each disc's banks placed by its own executable"
date: 2026-09-26
area: volumes
files: tools/sound_tables.py, crates/piney-data/src/sound/mod.rs, crates/piney-data/src/sound/mut_.rs, crates/piney-data/src/sound/out.rs, crates/piney-data/src/sound/qua.rs, crates/piney-data/tests/sound.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/src/stream.rs, plans/volumes.md, docs/engine/sound.md
---

# 203. The later volumes' sound tables: each disc's banks placed by its own executable

After 0202 Mutation booted, but it had no sound at all, the opening movie
included. `SndData::read` placed every disc's `SNDDATA.BIN` with
Infection's loading tables. On Mutation the first bank past Infection's
layout ran off the file ("read of 20 bytes at 0x6260 runs past the end"),
`Audio::open` failed, and the game went on silent.

## The tables

`tools/sound_tables.py` now writes the tables for all four volumes
(`gen`), and `piney_data::sound::tables_of` picks them. A later volume's
names come from `tools/xfer.py` with Infection's sizes, and several tables
grew:

| table | Infection | the later volumes |
| --- | ---: | ---: |
| `sqDataTown` | 10 | 12 |
| `sqDataEvent` | 123 | 127 |
| `sqVolTblEvent` | 123 | 126 |
| `seData` | 237 | 259 |
| BGM tracks | 2 | 8 |

So a table's rows on a later volume come from that volume's own layout
(`extent`):

- if the next global is the same one as on Infection, the table grows by
  however much the gap to it grew;
- if Infection's neighbour sits elsewhere, the table runs to the next
  global;
- if Infection's neighbour has no name, the table keeps Infection's size.

Infection's output is unchanged.

## The port

`SndData` keeps the tables that placed it, and `SndData::read` takes the
disc's volume. `Audio::from_snddata` builds the engine from those tables.
`stream_ctrl` takes the tables for its `seData[42]` note-off.

`later_volumes_tables_place_their_discs` checks each of the three later
discs:

- every bank's header, sequences and samples load;
- the common SE bank loads;
- every BGM track lies inside `BGM.BIN`;
- every jukebox row names a bank.

Headless renders of the desktop music and a sound effect play on all
three. The game on Mutation no longer prints "no sound" and plays its
opening movie with sound.

## Left out

- **The voices.** The later volumes' voice tables are empty for now: the
  event voices, the field's voices, the skill words. Each volume's code
  picks them from its own groups (plans/volumes.md, "The sound across
  volumes"), and several of their tables carried no name.
- **`setbl.cpp`**, the animation notes' sounds, is still Infection's.
  Mutation's rows are Infection's, laid out the same. Outbreak and
  Quarantine changed them.

**Still unknown:** Each later volume's `ccEvVoiceRequest` groups and
`ccVoiceRequest` cases. Outbreak's and Quarantine's `setbl.cpp` rows and
their changed `ccSeSetParam*`. Whether `ccSndStreamCtrl`'s stream numbers
changed with the later volumes' streams.
