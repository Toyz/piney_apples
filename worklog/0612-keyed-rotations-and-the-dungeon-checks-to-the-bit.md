---
number: 612
title: Keyed rotations and the dungeon checks to the bit
date: 2026-11-17
area: render, world, test, volumes
files: crates/piney-data/src/anim.rs, crates/piney-data/src/libm.rs, crates/piney-data/src/field/ee.rs, crates/piney-data/src/vu0.rs, crates/piney-data/src/light.rs, crates/piney-data/tests/anim.rs, crates/piney-data/tests/anim_fixture.txt, crates/piney-world/src/chara/pose.rs, crates/piney-world/src/chara/foe.rs, crates/piney-world/src/chara/body.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-desktop/src/anm.rs, crates/piney-effect/src/nodes.rs, tools/anim.py, tools/test_anim.py, tools/test_dungeon_rt.py, tools/test_foe_rs.py, tools/test_world_rs.py, tools/test_effect_draw_rs.py, docs/engine/animation.md, docs/engine/dungeon.md, UNKNOWNS.md
resolves: 285.13, 610.2
started: 2026-11-17T13:22:19-07:00
took: 8h 9m
---

# 612. Keyed rotations and the dungeon checks to the bit

`tools/test_dungeon_rt.py` compared the doors' swinging leaves within 1e-5
and 0.05 (up to 12,288 ULPs apart on the later discs), and had five skip
paths. `piney_data::anim` turned keyed rotations in double precision; the
game does its own float arithmetic, with one double `atan2`. The port now
runs every op the game runs, and the harness compares every word with
nothing skipped.

## The game's ops

A keyed rotation (`ccAnmCtrlRot_SetCtrl` main 0x001470c0, `_Set` 0x00146f30,
`_Init` 0x00146e30; INF addresses):

- **Keys.** `s16(int(deg * 182.044))` then `* pi/32768` (`mul.s`), the
  key's matrix `sceVu0RotMatrix`. Infection and Mutation truncate through
  `fptosi`, Outbreak and Quarantine through `cvt.w.s` (OUT 0x00145c04).
  Every keyed rotation key on the four discs (31,384 to 32,861
  controllers) is within ±1,800 degrees and none has exponent 255, so the
  two agree.
- **The step between keys** is `sceVu0MulMatrix(R_i, R_{i-1}^T)`, turned
  into (axis, angle) by `m2a` (0x00105ec0):
  - `EigenVector` (0x00105be0): `sub.s` 1 off the diagonal; the cofactors
    `(i+1, j+1)(i+2, j+2) - (i+1, j+2)(i+2, j+1)` by `mula.s`/`msub.s`;
    their `fabs` through `fptodp`, `fabs`, `dptofp` (the sign cleared); the
    first strictly largest wins (`c.le.s`); the axis
    `sceVu0Normalize(sceVu0OuterProduct(row i+1, row i+2))`; below
    0x2d2febff `m2a` stores zeros.
  - `EigenVec2Mat` (0x00105e00): the frame of the axis, row i+1 and their
    cross product, each `sceVu0Normalize`d again.
  - The angle: `atan2(L[1][2], L[1][1])` of `L = F (m F^T)`, newlib's
    double `atan2` (`__ieee754_atan2` 0x00122b00, `atan` 0x001269d0) on the
    soft float routines (`dpadd`, `dpsub`, `dpmul`, `dpdiv`, `dpcmp`),
    which round as IEEE doubles do; `dptofp` to the nearest float.
- **`a2m`** (0x00105f80): `sinf`, `cosf` of the angle; the cross terms
  `sceVu0ScaleVector(aa, s)` (`neg.s` on one side), the outer rows
  `(aa * aa[r]) * (1 - c)`, added to `c` or the cross term.
- **Playback.** The work holds the running matrix and the time into the
  current segment. Each call passes the segments strictly shorter than
  the time (`a2m(turn) * W`), then returns `a2m(axis, angle * frac) * W`,
  `frac` as for the floats.

By disc: Mutation's routines are Infection's at other addresses (the
double `atan`/`atan2` differ only in their constants' addresses, the
constants the same). Outbreak's and Quarantine's are recompiled with the
same operations in the same order. The libvu0 routines, `sinf` and `cosf`
are the same on all four.

## The port

- `piney_data::anim`: `eigen_vector`, `eigen_frame`, `m2a`, `a2m` and
  `RotKeyed` in the EE's and VU0's arithmetic; each segment keeps the
  running matrix it starts from, so a pose costs one `a2m` and one
  product. `Pose::bits` is `SetAnmCtrlWork`'s local matrix (`R * S`, then
  the position), `Pose::matrix` reads it. The old duplicate `anim::ee`
  module is gone: `anim::ee` re-exports `field::ee`, which gained
  `from_u32`, `eq`, `negative` and `to_f64`.
- `piney_data::libm`: the double `atan` and `atan2`, from the game's code.
- `piney_data::light`: a keyed light rotation the same (no light of any
  disc has one).
- `vu0::of_mat4` / `to_mat4`.
- `chara::pose::Play::instances` / `worlds`, `foe::Model::worlds`,
  `piney_desktop::anm`: the world matrices are `_SetLWMatrix`'s VU0
  products from the anm's own matrix down; `body::root` is
  `SetMatrix_PosRotZYX` to the bit (`town::pos_rot_zyx`); a node's
  transparency is the anm's times its own by `mul.s`. The effects' anm
  already composed in VU0 arithmetic and now takes `Pose::bits`.
- `tools/anim.py`: the same model in Python (newlib's `sinf`/`cosf`
  transcribed, the double `atan2` on Python floats); `tools/test_anim.py`
  compares the rotation and the object matrices word for word, and
  `anim_fixture.txt` holds the 16 words of each local matrix, which
  `tests/anim.rs` compares exactly.

## The skip paths

- **`skipped (port error)`, `special dungeons skipped (port error)`,
  `leaf dungeons skipped (port error)`.** The probe errs on none of them
  on any disc. They are now assertions.
- **`skipped (lake or special area)`.** `cases()` never made one; the
  lakes and the story areas with branches of their own had tests of their
  own that did not walk them. `test_dungeons` now walks two lakes and the
  15 story dungeons (`walk_cases`, each in a game of its own) through
  every check. That found a port bug: a lake's special objects
  (`OBJ_0ps0*`-`OBJ_0ps3*`) stand at the dummy plus its turn of an offset
  of their own (`sceVu0ApplyMatrix` w 0, `sceVu0AddVector`, gcmn 0x005bc0b0,
  0x005bc3c0, 0x005bc6c8, 0x005bc9d0: (0, -400, 0), (-200, -100, 150),
  (0, 0, 150), (0, -150, 0)); the port took the dummy's own place
  (`gim_place`). `test_draw_eff` now runs the story dungeons too, and the
  other tests' silent `continue`s on these conditions are assertions.
- **`roomless rows walked into by the door path (skipped)`.** Area 67's
  dungeon has a type-34 row behind a door that no `GotoNextRoom` branch
  takes; `SetRoom` would read through its null anm. Play never builds that
  dungeon: only `GO(2)` makes one, for `eventAreaNumber`; that is 67 only
  after `GO(1)` on field 67 (66 and 67 share words and server, and
  `ccCheckEventAreaNum` takes 66, the first row); `Enter` returns at once
  on field 67 (INF 0x0019de6c, MUT 0x001b2cb4, OUT 0x001a8b24, QUA
  0x001b0334); no event script on the four discs has a `scene` into area 2.
  `test_area_67_dungeon_is_never_built` checks all three on each volume;
  the walk asserts this row is the only such one.

## Checked

- **The animation model against the game** (`tools/test_anim.py`, each
  volume's own code): the unit tests on all four, and `bulk 100000 1 1000`,
  every animation with object records and every track of each, at random
  times and in several steps: 3,938, 3,984, 4,038 and 4,038 animations;
  250,986, 262,052, 262,800 and 262,800 keyed-rotation cases (2.1 to 2.25
  million segments passed); 2.6 to 2.8 million object matrices in
  playback. 0 failing. `tests/anim.rs` against the new fixture: exact.
- **`test_dungeon_rt`** on all four volumes, OK, every word compared and
  no skip: Infection 41 dungeons (24 + 2 lakes + 15 story), 1,157
  `SetRoom`s, 2,081,550 hit models identical, 1,260 `gimPos` slots, 4,100
  door and 233 stairs `GotoNextRoom`s, 79 `DrawEff` rooms (story ones
  now), 120 event-script scenes, none into a dungeon; Mutation 42
  dungeons, 2,564,771 hit models, 1,508 slots.
- **The other exact checks**: `test_foe_rs` (2,220 nodes posed, 552 of the
  magic circle, every word and transparency), `test_world_rs` (Kite's
  hands, 200 frames, worst difference 0), `test_effect_draw_rs` (every
  animated `lwMatrix` to the bit), and `test_effect_rs`, `test_evchar_rs`,
  `test_grunty_rs`, `test_boss_effect_rs`, `test_stfroll_rs`,
  `test_lights_rs`, `test_desktop_rs`, `test_toppage_rs`, `test_demo_rs`:
  OK on all four volumes. No harness compares a pose within a tolerance
  any more (`test_desktop_menu_rs`'s 0.01 is the menu's sprite places, not
  an animation).
- **The sweep** on 6fdd576 (the code of this commit): 356 runs, all rc 0,
  each result as `summary-main.txt`'s. The machine's out-of-memory storm
  (another program's) killed it at run 243 at about 19:02; the 113 runs
  left were run on the same tree.
- **Pins**: 291 calls, 0 differ.
- **Golden shots**: a base from a clean 58151c4 equals `world2/shots/base`
  (4,550 files, 0 differ). The tip differs in 4,033: 3,513 pictures and
  the 520 frame-hash files. A picture differs in 318 pixels at the median
  (p90 1,142), on the animated models: Kite, the NPCs, the enemies,
  animated props (the Chaos Gate's ring), their shadows and lights. Their
  roots are now `sceVu0RotMatrix`'s (its coarse sine), their bones the
  game's keyed rotations and VU0 products. Where the camera follows a
  node the whole view moves a little (the largest, the gate-hack arrival
  at act 24, 22,704 pixels). Side-by-side diffs of that one, the PC menu,
  a town, a field fight and a dungeon room show nothing else changed.
- `cargo test --release --workspace` (piney-game 349 passed, 0 failed,
  153 ignored), clippy (all targets and features) clean, fmt, `piney-gen
  gen|syms|carry --check`, `cairns check`, `tools/docs.py check`.

The clock ran on through the storm (about 8 minutes down, 19:02 to 19:10);
`took` is the clock's.

**Still unknown:** nothing.
