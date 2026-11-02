---
number: 558
title: Tartarga's skin: CompleteChunk gives a bone or skin model its clump's node list, SetAnm resolves it to the anm's objects
date: 2026-11-02
area: render, format, decomp, volumes
files: crates/piney-world/src/chara/body.rs, docs/formats/ccs-model.md, tools/ccs.py
resolves: 549
started: 2026-11-02T03:01:09-07:00
took: 6m
---

# 558. Tartarga's skin: CompleteChunk gives a bone or skin model its clump's node list, SetAnm resolves it to the anm's objects

[[549]] posed Tartarga's skin (`MDL_cts1body` on `OBJ_cts1body`, the 49th node of `cts1`'s `CMP_t0`) through `CMP_t0`, though his PC is made from `CMP_trall`. The rule was taken from event 310's camera framing him. The game's code makes the same choice, in three steps.

1. **`ccStream::CompleteChunk`** (INF main 0x00147c20-0x00147c88). After the file is decoded, it walks the stream's clump list (+0x5c). For every node of a clump that is an object (kind 0x100) whose model (object data +0x14, then +0x2c) has `mtype & 6` (bone or skin), it stores the clump's node list (+0x10) at the model data's +0x44 and its count (+0xc) at +0x48. `Decode_Model` zeroes both first (0x0014be28). `Decode_Clump` (0x0014c200) pushes each clump onto the head of the list, so if two clumps listed one object, the first decoded would be written last.
2. **`ccAnm::SetAnm`** (INF main 0x00151950, QUA 0x00150610). Once its objects are made, either the clump's (`ApplyClump`) or its own for the animation's other objects (`ccObj::Init` with no coords, QUA 0x001502bc), it goes over every bone or skin model it met (`modelTbl`, INF 0x00382e60). It sets `ccModel`+0x24 bit 0x40 and +0x14 to the data's +0x48, and allocates +0x10 with that many entries. Entry `i` is the object made for node `i` of the +0x44 list (chunk index +0x30, then +4), or 0.
3. **`ccModel::DrawBoneType`** (INF main 0x0013f860, QUA loop 0x0013dfb8) poses slot `i` by entry `i`'s coord (`_SetLWMatrix`, or a copy of +0x40 when +0x80 is 0), with no null test.

So a skin's slots are its clump's nodes as the animation moves them, whichever clump the character was made from. `ccClump::Init` (0x0013c5e0) passes its own node array to `ccObj::Init`, then `ccModel::Init`. That is only the table of a model in the clump being set up, and SetAnm's pass replaces it for bone and skin models.

`tools/ccs.py survey` now counts the objects of bone and skin models by how many clumps list them. On every volume each is in exactly one clump, none in two, none in none: 190 objects on Infection, 194 on Mutation, Outbreak and Quarantine. The port's `Body::model_clumps` (the first clump of the file listing the object, for a model outside the body's clump) is therefore the game's rule for every file on the discs, and its fallback (the root matrix for an unresolved node) is never reached. The code is unchanged. Its comment and `docs/formats/ccs-model.md` now cite these steps, and `tartarga_is_drawn_whole_in_the_goodbyes` still holds it.

**Still unknown:** nothing.
