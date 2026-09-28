---
number: 139
title: The walking PCs' texture variants and steps: changeTEX, rtpcCheckNote
date: 2026-09-26
area: render
files: crates/piney-draw/src/lib.rs, crates/piney-world/src/draw.rs, crates/piney-world/src/body.rs, crates/piney-world/src/rtownpc.rs, crates/piney-gs/src/lib.rs, crates/piney-desktop/src/soft.rs, crates/piney-game/src/world.rs, crates/piney-world/examples/world_probe.rs, crates/piney-game/src/session/tests/event22.rs, docs/engine/field-game.md
---

# 139. The walking PCs' texture variants and steps: changeTEX, rtpcCheckNote

Several of the town's walking PCs share a model file and differ only by
texture. `npcTbl`'s row names the texture: row 57, Koji, uses
`TEX_cwm3bod1v3` in `ctwm3`, which also holds `v1`, `v2` and `v4`. The
port read that name but drew every PC with the model's own texture, since
the draw list had no way to swap one. This came from the GAPS sweep.

## The game

`changeTEX(entry, clump)` (gcmn 0x005076c0) runs from
`ccRtownPC::ccRtownPC` when the name's first byte is set. It duplicates
the clump (`ccClump::Duplicate(0x2000)`), so the change is this PC's
alone, then calls `ccClump::ChangeTex(tex, mat)` with the file's
`MAT_tex` material and the named TEX chunk. The texture brings its own
palette.

## The port

- **The draw list.** `piney_draw::ModelDraw::tex_swaps` is a list of
  (MAT_, TEX_) pairs. Both renderers apply it when they look up an mmat's
  texture: piney-gs by its key, piney-desktop's software renderer in
  `material_texture`.
- **The body.** `Body::tex_swaps` belongs to the body's own clump.
  `draw_parts` passes it through the new `draw::model_swapped`, and
  attached models (the weapons) keep their own textures.
- **The PCs.** `rtownpc::load_model` sets the pair from the row. Each PC
  already has a `Body` of its own, which stands in for the game's
  duplicated clump.
- **The check.** The Mac Anu PC test now requires Koji's pair to be
  (`MAT_tex`, `TEX_cwm3bod1v3`).

## Their steps

The same PCs never read their animation notes. `rtpcCheckNote` (gcmn
0x00509160), the `funcNoteProcess` the constructor sets, handles notes 1
and 2. It calls `ccSeSetParamPC(param, pc, ccsType)` for the step's
sound, by the ground under the PC, and `ccEffPawSmoke(pc, +0x2c0)` for
the dust, on every step, walking or not.

`RtownPc`'s step now reads its notes. Each step becomes a `PcEvent::Step`
carrying the PC's position, ground attribute, feet, heading and speed.
The town plays it through `se3d::pc_note` and the town's effects' paw
smoke. `world_probe`, whose game side records no presentation, leaves
the steps out. `the_walking_pcs_have_footsteps` hears 46 steps in 300
frames of a quiet Mac Anu with Kite standing.

**Still unknown:** nothing compares the variants' or the steps' pictures
and sounds with the game's.
