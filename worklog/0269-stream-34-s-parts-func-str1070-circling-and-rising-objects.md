---
number: 269
title: Stream 34's parts: Func_str1070 circling and rising objects
date: 2026-09-28
area: video, volumes
files: crates/piney-stream/src/mutation.rs, crates/piney-gen/src/manifest.rs, crates/piney-stream/src/lib.rs
---

# 269. Stream 34's parts: Func_str1070 circling and rising objects

The last of Mutation's thirteen new effect functions ([[267]] left it).
`Func_str1070` (MUT main 0x001a6af0) keeps the usual `ccStrEffectCtrl`
and a part group, as stream 15's tasks do, with its own part class: one
of six `str1070e` objects (`OBJ_xpart00`-`05`) circling the scene's origin
while it rises and spins. The creator is `ccPartCreate` (vtable
0x003889a0, `Ctrl` 0x001a6510), the part's `Ctrl` is 0x001a63e0.

What the build reads for it (piney-gen, Mutation's only):

| entry | MUT main | what |
| --- | --- | --- |
| `part_params_1070` | 0x00321c00 | seven creator blocks, 0x50 bytes: 14 floats (base and random pairs), rate, random rate, life |
| `part_models_1070` | 0x00366f70 | ten rows: chunk, scale |
| `part_chunks_1070` | 0x00321e30 | the six chunk names |

The generator finds them from the code: the blocks as the addresses each
new creator is given (after `sw $zero, 56($v0)`), the model table through
the creator class's vtable (slot 3, its `Ctrl`), the names as the first
address built after the effect file's `GetCCSAdrs`. Outbreak's
`Func_str1070` is other code (4.9 KB against 6.8 KB) and the patterns do
not match it, so the entries are Mutation's only for now.

The timeline, by scene frame: 260 blocks 0 and 1 (a burst of 16 parts,
then one every other frame for 300 frames), 261 two shades, 411 blocks 2
and 3, 505 one shade and blocks 4 and 5, 636 two shades and 4 and 5
again, 781 two shades and block 6 (a burst of 64), 932 the shades off;
each creators' frame empties the group first. The part draws `rand()`
ten times as it is made (docs/engine/stream.md has the order), and each
frame turns its angle, rises, spins about y and z, and is placed at
`(r sin a, r cos a, 0)` plus its place. The unit test
`mutation::tests::str1070_bursts_then_streams` holds frame 260's 16 and
frame 261's 17.

The part is gone once above 200 and out of view (`CheckBoundingBox`). The
port keeps every part (it takes the models to have no Bbox, as the
ending's rocks have none); unchecked.

**Still unknown:**
- Whether `OBJ_xpart00`-`05`'s models carry a Bbox; if they do, the game
  drops parts the port keeps.
- Outbreak's and Quarantine's `Func_str1070`.
