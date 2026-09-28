---
number: 188
title: "The fields' tables for all four volumes; xfer's layout pass and the misnamed tables it found"
date: 2026-09-26
area: volumes
files: tools/xfer.py, tools/field_tables.py, crates/piney-data/src/field/mod.rs, crates/piney-data/src/field/mut_.rs, crates/piney-data/src/field/out.rs, crates/piney-data/src/field/qua.rs, crates/piney-data/src/dungeon/out.rs, crates/piney-data/src/dungeon/qua.rs, crates/piney-data/src/volume.rs, docs/disc/volumes.md, docs/engine/field.md, docs/engine/battle.md, plans/volumes.md, GAPS.md
---

# 188. The fields' tables for all four volumes; xfer's layout pass and the misnamed tables it found

The field generator was the last of phase 0's generators besides the
sound's that ran on Infection only. It failed differently on each volume.

## Mutation: a name the carry got wrong

`field.py` reads `BgMatName`, which Mutation's `.syms` did not have. The
bytes were there: Infection lays out `SmallMeshName`, `BgMatName`,
`BgMatName2`, `BGTBL` and `BGTBL2` 0x60 apart, and Mutation keeps that
layout. But Mutation's `WORLD::DrawBG` builds `BgMatName2` before
`BgMatName`, and `WORLD::Init` builds `BGTBL2` before `BGTBL`. The data
pass pairs a function's globals by the order its code builds them, so it
named `BGTBL` and `BGTBL2` the wrong way round and left `BgMatName`
without a name. The pointer pass then followed the misnamed `BGTBL`, so
every `BG_TBL_x` sat on `BG_TBL_x2`. Checked by content: `BGTBL`'s row
counts (4, 8, 4, 4, 8, 6, 6, 8, 8, 8, 8) are at 0x00688f70, not 0x00688fd0.

**xfer's layout pass** (pass 8):

- Two data-pass names of one size near each other swap when each one's
  words agree better at the other's place.
- Two carried globals that sit the same distance apart in both volumes,
  and whose words agree, bound a span. The globals between them take
  their offsets when their words agree: unnamed ones get a name, and a
  data-pass name moves.

The first try was too trusting in three ways:

- It anchored a span on a wrongly named global, and moved `BGTBL2` onto a
  table Mutation added.
- In `.bss`, where no words can be compared, it placed main's
  `SkillDamageValueAttributeCritical` 8 bytes off. `test_battle.py`'s run
  of Mutation's `ccSkillDamageValue` caught that.
- It did not see main's globals that the data pass files under an
  overlay.

Now:

- Only globals whose words agree anchor a span, so `.bss` is left alone.
- Every name in main's window counts as inside a span.

**The other passes, fixed on the way:**

- The pointer pass took a pointer's bytes for text: 0x006f2d98 reads
  "\x98-o". It then refused Infection's `BG_TBL_H` because Mutation's
  pointer does not read as text.
- With that fixed, a table that keeps Infection's layout while rows are
  added to what it points at voted wrongly. `pcMsg4` in Mutation points
  row 14 at another message than "Whoohoo! ... server is open!". So the
  content pass now runs first, and what a global holds outranks a
  pointer to it.
- The content pass named `pcMsg7_24` where `pcMsg7_04` is, and
  `enemyList46` on `enemyList36`, because in Infection those pairs hold
  the same bytes. Two of Infection's globals with one pattern are now
  both left for the other passes.

**What moved.** Every move on all three volumes was checked, with
Infection's bytes at the new place and not the old:

- `BGTBL` and `BGTBL2`, and the `BG_TBL_x` reached through them;
- `BgMatName`;
- the Grunties' voice tables, `gusoponTbl` to `aquaTblE`, each of which
  sat on its neighbour;
- `tradeMenuStr` in Mutation, `evs209Tbl`, and `str1070Tbl`.

A few right names were lost where they were only right by luck:

- `str0820Tbl`: `streamTbl` and `streamTblE` point different ways, so
  the vote is not unanimous.
- Some of Mutation's `pcMsg` rows, whose text changed.

Counts are in volumes.md. The generated tables did not change:

- area, main data and statics are current;
- `dungeon/out.rs` and `qua.rs` only renamed a fog table to
  `dungeonFog_hacked`, as Infection names that row.

The tool tests pass: `test_battle` (Mutation's check included),
`test_areas`, `test_evscript`, `test_save`, `test_dungeon`, `test_field`
and `test_font`.

## Outbreak and Quarantine: the compiler

`field_tables.py` reads `WORLD::Generate`'s constants from its code, and
Outbreak's compiler moved them:

- `this` is in `$s2`, not `$s0`: taken from the prologue's `move`.
- The objects' percentage is in `$f1`, not `$f20`: taken from the first
  `lui`/`mtc1` pair after the divides.
- A nop follows the `SetDungeonEnter` call's delay slot, so the
  no-entrance branches target one word later.
- The hill loops' counters are in `$s3` and `$s4` the other way round:
  taken from each loop's decrement. The first loop makes the large hills.

`water()` finds `WORLD.effccs`'s store by the same `this` register.

## The tables

`piney_data::field` now has `mut_`, `out` and `qua` and
`field::tables_of(v)`. The four are equal except for the square root:
Outbreak and Quarantine use `Sqrt::Fpu`. `each_volume_has_its_tables`
compares the four `Debug` forms with that one field aside. The run time
still uses `field::INF`: the game refuses other discs until phase 4.

## Also

The uncited-functions audit (0186) was run again in full. Of the 484
functions left once the SDK, destructors and later-volume code are
filtered out, one was an unread Infection path: `ccRegisterRegularGimmick`,
which registers the gimmick rows each place loads.

- `ccRegisterGimmick` sets the row's +0x28.
- `initEntryCCS` gives those rows their file (+0x38).
- A town registers row 16. A field registers its seven rows and its field
  type's. A dungeon registers its eleven, its dungeon type's, then its
  attribute's and row 20 for field type 4.

The port reads a gimmick's file when it makes one, which is what the
registration gives in the game. This is written up in battle.md. Also
from that list:

- `BreakMirror` is `ccBoss02Slave`'s (Innis, Mutation).
- `ccWaterSummonsElement::Level4` is ported; the audit missed it because
  of its short name.

**Still unknown:** The sound's voices across volumes. Each disc's
`ccEvVoiceRequest` has real tables only for its own volume's group.
Mutation's group 1 indexes words at 0x0038ace8 that are not a table, and
the carried `evVoiceDataVol1M` sits there with Infection's size.
`EvVoice` needs groups per volume, read from the code (plans/volumes.md,
"The sound across volumes").
