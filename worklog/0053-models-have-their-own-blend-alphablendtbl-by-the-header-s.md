---
number: 53
title: Models have their own blend: alphaBlendTbl by the header's blend type, and the title's glows are additive
date: 2026-09-23
area: render, test
files: crates/piney-desktop/src/anm.rs, crates/piney-desktop/src/assets.rs, crates/piney-demo/src/draw.rs, crates/piney-stream/src/draw.rs, crates/piney-gs/src/lib.rs
---

# 53. Models have their own blend: alphaBlendTbl by the header's blend type, and the title's glows are additive

The user found the title's selection still wrong after [[47]] and [[51]]:
- behind the selected icon, a hard-edged dark square;
- behind its text, a dark box.

The GPU and the CPU GS model drew the same thing, so the fault was in the
draw list, not the renderer.

## Finding it

In `ccSetMaterialPacket` (0x0013e6c0), ALPHA_1 is
`ccDrawModelParam.alpha` (+0x138). `ccModel::Draw` copies it from the model
itself, `ccModel.alpha` (+0x28), and its TEST from `ccModel.test` (+0x30).
`ccModel::Init` (0x0013a440) sets both:

- `alpha = alphaBlendTbl[blendType]` (0x00348580). `blendType` is the model
  header's flag & 3, which `Decode_Model` keeps at `ccModelChunk` +0x4c.
- Blend type 0: `SetRenderState(1, 1)`. The alpha test is GEQUAL with AFAIL
  FB_ONLY, as the port already drew every model.
- Any other type: `SetRenderState(1, 0)` (0x0013ae60), ATST NEVER with AFAIL
  FB_ONLY. Every pixel's colour is drawn and no Z is written. It also sets
  `ccModel.flag` 0x20.
- `ccModel::Draw` (0x0013ed70) puts every mmat of a model with flag 0x20
  into the sorted group, whatever its transparency.

`alphaBlendTbl`'s first four entries:

| type | ALPHA | blend |
| --- | --- | --- |
| 0 | 0x44 | `(Cs - Cd) As + Cd` |
| 1 | 0x48 | `Cs As + Cd`, additive |
| 2 | 0x42 | `Cd - Cs As`, subtractive |
| 3 | 0x09 | `Cd As + Cs` |

The ports drew every model with type 0's state. In `title1`, 16 of the 97
models are type 1: `MDL_xdt_lig_*`, the selection glows, and
`MDL_xdt_dig_*`. Mixed instead of added, their black surrounds darkened the
screen, which made the square and the box. `xddesk01` has no blended model.
In stream 2's frame 822, `MDL_ngon02` (the figure's glowing runes) is
additive over `MDL_ngon01`.

## The fix

- **The model's blend type.** `ModelInfo` carries it.
- **The state.** `model_state(blend_type)` gives that type's ALPHA and
  TEST.
- **The three draw paths.** The desktop's, the title's and the streams' use
  that state, and put a blended model's mmats in the sorted group.
- **The GPU.** It draws NEVER with FB_ONLY as a single pass with Z writes
  off.

## Checked

- **Unit tests.** `model_state_by_blend_type` checks the four ALPHA values
  and the two tests. `never_fb_only_writes_no_z` (`piney-gs`) draws two
  additive sprites that add, and a farther opaque one after them that still
  lands.
- **Against the game's code.** `test_demo_rs.py` (18), `test_desktop_rs.py`
  (20) and `test_desktop_menu_rs.py` (5) still pass; the workspace's tests
  pass, and clippy and fmt are clean.
- **Shots.** The title menu, on both the GPU and the CPU GS model, now
  shows a soft glow around the selected item and white text, with no dark
  boxes.

**Still unknown:**
- **No game frame to compare.** The glows and stream 2's figure are checked
  against the code, not against a frame from the console.
- **Not checked against the game.** The comparison scripts do not compare
  which group (opaque or sorted) a mmat lands in. The blended models' move
  into the sorted group is from the disassembly alone.
- **The other blend types.** Types 2 and 3 are in the table but not seen
  in `title1` or `xddesk01`. The streams have not been surveyed for them.
