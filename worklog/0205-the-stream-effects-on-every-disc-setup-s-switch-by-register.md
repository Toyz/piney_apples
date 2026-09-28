---
number: 205
title: "The stream effects on every disc: Setup's switch by register, the effects' literal tables carried, the carry's sequence pass"
date: 2026-09-26
area: volumes
files: tools/main_data.py, crates/piney-data/src/main_data, crates/piney-effect/src/particle/tables.rs, crates/piney-effect/src/drawelm.rs, crates/piney-effect/src/summoned.rs, crates/piney-effect/src/upheaval.rs, crates/piney-effect/tests/volumes.rs
---

# 205. The stream effects on every disc: Setup's switch by register, the effects' literal tables carried, the carry's sequence pass

After 0204 the three later discs reached their desktops, but every stream's
effects failed to build ("the stream's effects: ..."). A new test,
`piney-effect/tests/volumes.rs`, builds the tables on each disc part by
part. The failures were in three places.

## `ccParticle::Setup`'s switch

The port reads the switch on a texture id out of the function's code: its
bias, its bound, its jump table and each case's row and CLUT. The old
reader matched Infection's exact words.

Outbreak and Quarantine recompiled `Setup`:

- the switch sits one instruction later;
- the id is in `s2` rather than `s0`;
- the row is in `s1` rather than `s2`;
- the CLUT is in `s0` rather than `s3`.

The reader now finds the `addi a0, id, -first` / `sltiu at, a0, n` pair in
the function's first 0x200 bytes. It reads each case's two assignments
(`li`, `move r, zero`, `move r, id`) by register: the row's register and
the CLUT's are the ones the first case assigns. All four volumes decode to
the same map: ids 91-244, where 91-100 are the default and 101 draws row 0
with CLUT 101 (`the_setup_switch_on_every_volume`).

## Literal tables behind helpers

The upheaval, summoned-creature and element-draw tables read literal
addresses through local helpers (`names`, `ints`, `arr`), which did not
carry. The 0202 sweep had wrapped only literal reads made directly. The
helpers now carry. `read_f32s` and `read_i16s` already did.

## The carry's sequence pass

Mutation's `ConvergenceSystem` grew by 224 bytes, so no alignment reached
its `@3328`. The name pass had put `@3329` where `@3328` is: each compile
numbers its labels anew.

`by_sequence` takes each pair of same-named functions and lists the data
addresses each builds, in instruction order, leaving out bare `lui`s and
high halves alone. It uses a pair only when:

- the two lists are equally long;
- every named global the carry already holds sits at its carried place,
  with at least one there.

A global still uncarried then goes to the address in the same place, on
unanimous votes. A compiler label (`@123`) the name pass carried doesn't
count as an anchor, and the order may move it ("the code wins"). A global
with no pointers in it moves only where the volume's bytes are
Infection's. That keeps out places where the tracker paired a `lui` with
the wrong low half (`@1871` built at 0x0034fff0, zeros).

On Mutation it moved 11 labels. Among them are `@1308`, `@7305` and their
runs, where the names had landed on the neighbouring strings
(`CMP_sr1dat1_1` for `_2`, `ANM_sd5ae0_a` for `sd1`). It carried 286
globals besides: vtables, `.sdata` statics and `@3328`-`@3330`.

The misses the source still names: Mutation 6, Outbreak 29, Quarantine
28.

All three later discs now go through New Game into the desktop with
their streams, effects included, and no error printed.

**Still unknown:** Mutation's 6 remaining misses:

- `@1489` in `ChangeScene`;
- `@5293`-`@5295` in `ccEvent::Execute`;
- `InitSpcParam`'s two default item lists.

Outbreak's and Quarantine's 29 and 28.
