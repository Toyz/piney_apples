---
number: 202
title: "The carry made reliable and every reader through it: Mutation boots from its title into the desktop's setup"
date: 2026-09-26
area: volumes
files: tools/main_data.py, crates/piney-data/src/exe.rs, crates/piney-data/src/main_data, crates/piney-battle/src/tables.rs, crates/piney-battle/src/party_chat.rs, crates/piney-stream/src/table.rs, crates/piney-stream/src/effect.rs, crates/piney-stream/src/ending.rs, crates/piney-effect/src/particle/tables.rs, crates/piney-effect/src/spell.rs, crates/piney-effect/src/files.rs, crates/piney-demo/src/names.rs, plans/volumes.md
---

# 202. The carry made reliable and every reader through it: Mutation boots from its title into the desktop's setup

After 0201 Mutation's title worked. Its New Game led into streams and
the desktop, which stopped at readers still using Infection's addresses
directly. This entry covers four things: the carry, the readers, the
streams and titles, and a check of the whole source.

## The carry

Four changes to `tools/main_data.py`'s carry, each forced by a wrong or
missing address:

- **Votes at a global's start come first.** Mutation's `skillTbl`
  (Infection 0x0061f4a0) had two votes each for 0x0064e2f0 and
  0x0064f4a0. Code that indexes into a table votes for "its start",
  computed from Infection's offset, and the table grew in Mutation. The
  carry now counts only the votes at a global's own start when there
  are any. 0x0064e2f0 is right: its rows read "NOTHING", "ATTACK",
  "Data Drain".
- **The code outranks a name.** Where a unanimous vote disagrees with
  the name `xfer.py` carried, the vote wins (96 cases on Mutation).
  Most are a run of `$gp` variables off by 4, after a variable Mutation
  inserted; one is `skillTbl`, which the plan had flagged. Each case is
  printed.
- **An overlay's copy of a main global.** Outbreak keeps a
  `tpcTradeList` in GCMN.PRG as well as main's. GCMN's votes for it now
  go under the overlay's key, and `Image::carried` tries the loaded
  overlay's key first.
- **Content, and neighbours.** Globals the other passes miss, of 8 bytes
  or more with no pointer in them (Infection's relocations say which),
  are carried where their bytes occur exactly once in the volume's
  section. In `carried`, an address between two carried rows that lie
  the same distance apart in both executables maps by its offset from
  the lower. That covers a run of local arrays, such as the tornado's
  `@1868` .. `@1872`.

## The readers

- The first sweep (0200) wrapped every direct `image.read(CONST ...)`.
- This one did the indirect uses by hand:
  - constants passed to helpers (the battle tables' `rows`, `shorts`,
    `ints`, `item_lists`) or chosen first (`if english { STREAM_TBL_E }
    else { STREAM_TBL }`);
  - the chat tables, keyed by Infection's address but read at the
    volume's.
- Helpers that only ever get an Infection address now carry inside:
  `read_f32s`, `read_i16s`, `read_rows`. So do `GenParam::read` and
  `FfParam::read`, which keep Infection's address as the row's identity
  (`p.va - GENERATOR_TBL`).
- Reads of literal addresses (`exe.u32(0x0037_7f10)`) are wrapped the
  same way.

## Streams and the title

- The stream archive table has six types: CMN, STR1, STR2, STRSUB, STR3,
  STR4, the order the later `ccCdInit`s list them.
- Each disc's opening movie is its own: `opening.pss`, `opening2.pss`,
  `opening3.pss`, `opening4.pss`.
- `eventObjTbl_0580` and `_0581` are carried separately. The port had
  read them as one run.

## Checking the whole source

A script checks every Infection data address the crates name (constants
and literals, 867 once other volumes' own values are left out) against
each volume's carry, using the same lookup `Image::carried` does. It
misses:

| volume | missed |
| --- | ---: |
| Mutation | 8 |
| Outbreak | 44 |
| Quarantine | 43 |

Mutation's 8 are each referenced by one function Mutation changed, where
no vote can align:

- `ccGame::ChangeScene`'s `@1489`;
- `ccEvent::Execute`'s `@5293`-`@5295`;
- `InitSpcParam`'s default item lists;
- `ConvergenceSystem`'s `@3328`;
- the name entry's `STR_INFO_MSG_0`.

Mutation now boots through its logos, its title (with its own opening
stream 21), New Game, its opening stream 2, and into the desktop's setup.
The desktop's events then stay in their first phase. Infection's tests
all pass unchanged.

**Still unknown:** Why Mutation's desktop setup stalls. Finders for the
8 misses. Outbreak's and Quarantine's 44 misses, where their compiler's
changes keep votes from aligning.
