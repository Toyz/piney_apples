---
number: 267
title: Mutation's stream effect tasks: the opening's text and eleven more
date: 2026-09-28
area: video, volumes
files: crates/piney-stream/src/opening.rs, crates/piney-stream/src/mutation.rs, crates/piney-stream/src/effect.rs, crates/piney-gen/src/manifest.rs
---

# 267. Mutation's stream effect tasks: the opening's text and eleven more

Mutation's `StreamDemoFuncTbl` (MUT main 0x00367210) has 40 rows, not 23:
Infection's 22, then 18 of its own on 13 functions (Outbreak has the same
names). A stream whose scene names none plays bare: its noise, feedback,
fades, flashes, transfers and texts missing. Twelve of the thirteen are
ported now (`crates/piney-stream/src/opening.rs`, `mutation.rs`);
`Func_str1070` is not.

## The opening (stream 24)

`Func_str0710` (MUT main 0x0019c140) is `Func_str0581`'s kind: the
`ccStrEffectCtrl`, the ending's table walker and puff bursts over its own
`eventObjTbl_0710` (0x00366e30, five rows), plus two blocks of text lines,
a class new in Mutation (constructor 0x00187960, `SetText` 0x00187bd0,
fade 0x00187ac0). Cues 700 and 710 put a text in a block, centre each line
by its bytes and fade the block in by 0.05 a frame; 709 and 719 fade it
out. The texts sit in the executable (normal and Parody Mode's); the
build reads them (`opening_text`), never the repository. `SetObj` and
`ccEffPart0580::Create` are Infection's code instruction for instruction.

## The others

Most are Infection's kind of task with their own fog, shades, fades,
feedback and transfers (MUT `charHeightTbl` rows 0, 6, 15 and 17);
`Func_str0880`'s hit marks turn by a four-row table of its own
(0x00366ed0), which the build reads (`hit_rot_0880`); `Func_str9204`
serves `str9204`-`str9206` with `Func_str9000`'s letterbox;
`str9201` and `str9301` run `Func_str9101`, already ported. The table of
cues is in docs/engine/stream.md, "Mutation's other effect tasks". The
generator finds each function by its row (`demo_func`), then its tables
by the code that builds them, so Outbreak and Quarantine read theirs
too; Infection's table has none of these rows.

Two things found on the way:
- `Func_str0932`'s `SetFog(0, 0, 0, 0, 0)` at frame 1700 divides 0 by 0.
  The EE gives the largest float, so the fog is held at its near value
  (none); the port's `f32` gave NaN, full fog. `SceneFog::with_rates`
  follows the EE now.
- `Func_str1040`'s four texts are stream 31's own subtitle records 16-19
  (`evStrMsgTbl[31]` + 192), already in the build.

Checked by the unit tests `opening::tests` and `mutation::tests` (the
line placing, the fades, the shades' depths); nothing runs these tasks
against the game's code yet.

**Still unknown:**
- `Func_str1070` (a part system of its own, creator 0x001a6510, part
  `Ctrl` 0x001a63e0, parameter blocks at 0x00321c00) is not ported.
- `str7100`, `str8800` and `str0300` changed on Mutation and still run
  Infection's.
- Outbreak's and Quarantine's versions of these functions are not compared.
- No harness compares these tasks with the game's code frame by frame.
