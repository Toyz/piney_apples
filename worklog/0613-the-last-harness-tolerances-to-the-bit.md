---
number: 613
title: The last harness tolerances to the bit
date: 2026-11-17
area: render, test, volumes, tooling
files: crates/piney-data/src/shadow.rs, crates/piney-data/examples/shadow_probe.rs, crates/piney-data/examples/model_probe.rs, crates/piney-desktop/src/shadow.rs, crates/piney-desktop/src/dtmenu.rs, tools/test_shadow_rs.py, tools/test_desktop_menu_rs.py, tools/test_ccs.py, tools/ccsmodel.py, tools/vu.py, docs/engine/shadow.md, docs/engine/desktop.md, docs/engine/render.md, docs/formats/ccs-model.md
started: 2026-11-17T21:34:51-07:00
took: 41m
---

# 613. The last harness tolerances to the bit

Three harnesses still compared within a tolerance: the shadow directions
(`test_shadow_rs`, a relative 1e-6), the desktop menu's sprite places
(`test_desktop_menu_rs` and `test_desktop_savemenu_rs`, 0.01) and the
CCS tools' normals and posed body (`test_ccs`, 0.05 and 0.5). Each now
compares bits, against the game's own code, on all four discs.

## Shadow directions

`SetShadowNormal2` (INF 0x001400e0, MUT 0x00140160, OUT 0x0013e8b0, QUA
0x0013e670; Outbreak's reorders registers only) keeps a direction's
integer cross product and sends VU1 `vitof12` of it (a `cvt.s.w` and a
divide by 4096) times `1 / 4096` (`sceVu0DivVector`: `vdiv` Q, `vmulq`),
w 0. The harness worked out `i / 4096 / 4096` in doubles. It now takes
the EE's ops on the port's integers and compares the stream's bits with
both. Of the 65,013 direction lanes on Infection, 8 pass 2^24 (where
`cvt.s.w` truncates); none of them differed from the nearest float, so
the tolerance hid no difference in the data.

The port's merge test was the host's floats: `unit` is now
`sceVu0ITOF0Vector` and `vu0::normalize`, the cosine `vu0::dot`, and
the merge `ee::lt(0x3f7f0000, cos)` (the game's `c.le.s` falls through to
a merge). `shadow::direction` gives the floats VU1 is sent, which
`shadow_probe` prints and `shadow::volume`'s lit test now uses in place
of `n as f32 / 2^24`.

## The menu cursor

Every inexact packet was the select cursor's wing (`DispSelectCursol`
0x001b7910, code 10): 5,112 places a run off by a few ULPs. The port
eased the wings in the host's rounding; the game truncates. The target
is `x + (a / 10 - 16)` above alpha 96 (the test is a double compare on
the int, `_dpfgt`) else `x + (a / 12 - 4 - 16)`; each wing `w[k-1] + (t
- w[k-1]) / ((10 - k) * (3 - rate))`, the lead `w0 + (t - w0) / (4.5 /
rate)`; the bars at `6 + x`, `y - 2`, `16 + x` and `(16 + x) + w`.
`SelectCursor::disp` now runs those through `eef`. The same ops on all
four discs (Outbreak's and Quarantine's allocate registers otherwise).

The harness's model was inexact too: its `MakePacket` hook moved the
sprite's x by `dx + sx` in doubles, and `MakePacketStr`'s by `sx *
len`. The game's `MakePacketStr` adds `sx` per glyph with `add.s`
(0x0015c070) and on a newline puts x back and adds `sy` to y
(0x0015b550); the hook now does that, for an unturned sprite (it asserts
ctrl & 2 clear). The port's trace prints places and sizes as float
bits; `same_pk` is equality.

## The CCS tools

The Python readers are still the harnesses' model of the data (anim,
test_anim, test_shadow_rs, test_demo_rs, test_foe_rs and the effect
harnesses import them), so they stay. What changed:

- **Every model against piney-data, word for word.** The new
  `model_probe` example writes each model `model::models` decodes;
  `test_every_model` compares positions, normals with strip flags,
  colours, ST, skin entries and triangles of every model of the volume's
  DATA.BIN with ccsmodel's (which now keeps the stored s16 triples,
  `Mmat.stored`). The weights' range check (0.7 to 1.01) became the exact
  count of skin vertices by weight sum (192, 198, 206, 212, 255, 256).
- **The weapon's normals** are compared with the words at the offsets the
  layout gives (`s8 / 64` and the strip flag), not by their length.
- **The posed body.** ccsmodel's posing was its own double-precision
  model (true sines, T R S in doubles). It now takes tools/anim.py's poses
  (`Pose.bits`, the first keys), composes worlds with `vu_mul`, turns an
  F_Obj record into `T * sceVu0RotMatrix(pi * deg / 180) * S`, and blends a
  skin as VU1 does. Reading `DrawBoneType` and the microcode: slot `i`'s
  matrix is `sceVu0InversMatrix2(model world) * node world` at VU1 qword
  `20 + 4i`; `Decode_Mmat02`'s positions are `ITOF4` then `* (scale /
  256)` (so not `s16 * scale / 4096` to the bit) and each entry carries
  `weight * 16` and its matrix address; `mc03b_DrawModel1` (0x433) is the
  skin program, summing from (0, 0, 0) the matrix times the position
  (`mulax`/`madday`/`maddaz`/`maddw` with w 1) times `itof12(weight *
  16)`, and stores the blend at `2(vi04)` (0x45d). `mc03m` is the bone
  program, its one matrix multiplied into the screen matrix first.
  `ccsmodel.skin_positions` is that blend; `test_skinned_body` runs the
  microcode itself in tools/vu.py (`Vu.run` gained a stop address) on
  every vertex of `MDL_cbalbody` at frame 0 of `ANM_cbu1nut0`, with the
  slots the game's own `sceVu0InversMatrix2` and `sceVu0MulMatrix` make
  in eemu and with the node worlds `place()` uses, and compares every
  word. The bounds are then the microcode's, asserted exactly; the box
  check keeps the file's `BOX_bbox` across and below its top.
- **Every volume.** `test_ccs` follows PINEY_VOLUME, with each disc's
  survey, mmat and weight counts and its own stream archives (Outbreak's
  and Quarantine's name every member `tmp.ccs`, so a member is found by
  its file's own name).

## Other tolerances

A grep of every harness for `assertAlmostEqual`, `delta=`, `places=`,
`isclose` and `abs(a - b) <` found one more: `test_shadow_rs`'s
`VolumeAgainstGame` counts each draw's shadow buffer pixel by pixel and
allows 8 pixels or 1% of a draw, and 1% of draws past that. It is not
exact: on Infection 183 of 600 comparisons differ by one to six pixels
(175 on Mutation). `shadow::volume` works in glam's host floats, not
VU1's arithmetic. Left for its own piece; shadow.md says so. The rest are
random input ranges, printed diagnostics (`test_world_rs`'s worst
element, then `bad == 0`) or exact recoveries (`round(v / unit)` of an
exact product). `test_desktop_menu_rs` logged `SetFrame`'s arguments
rounded to six places (never compared); they are bits now.

## Checked

- `test_shadow_rs` (decode: 1,957 / 1,964 / 1,965 / 1,965 distinct
  mmats, every direction's bits), `test_desktop_menu_rs`,
  `test_desktop_savemenu_rs`, `test_ccs` (8 tests): OK on all four
  volumes. `test_anim` OK on all four; `test_vu` and `test_dungeon` OK on
  Infection (`test_vu` fails on the later discs on main too: its blob
  range and unreferenced-entry list are Infection's).
- Crate tests 780 passed, 0 failed; the piney-game suite 349 / 0 / 153;
  clippy on all targets and features clean; fmt; `piney-gen gen`, `syms`
  and `carry --check` current; `tools/docs.py check`.
- The sweep, the battle pins and the golden shots run on this commit
  after it was made; their results are with the hand-off.

**Still unknown:** the shadow volume's pixels are not yet compared
exactly: making them so means porting `ccShadowModel::Draw`'s matrices
and VU1's `mc_DrawShadow1`-`3` and `mc_DrawShadowClip` (0x5d2-0x758) in
the EE's and VU's arithmetic, and deciding shadow.md's open path question
on the way. The bone and skin packets' VIF codes (how `Decode_Mmat02`'s
halfwords are unpacked) are still unread; `mc03b`'s entry layout is
known from the microcode.
