---
number: 152
title: The field's depth shades, and model Z at the GS's scale
date: 2026-09-26
area: render
files: crates/piney-draw/src/lib.rs, crates/piney-gs/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-desktop/src/soft.rs, crates/piney-desktop/src/noiz.rs, crates/piney-desktop/src/layers.rs, crates/piney-world/src/field_area.rs, docs/engine/field.md, docs/engine/render.md
---

# 152. The field's depth shades, and model Z at the GS's scale

A GAPS drawing item: `WORLD_MAN`'s two `ccBufferSampling`s, which every
`WORLD::Draw` sends first. field.md said they made "a depth shade from
the z buffer" and that the renderer could not read Z as a texture. Neither
was right. They soften the distance.

## What the game draws

`WORLD_MAN::WORLD_MAN` calls the seven-argument `SetShade(page, x, y, tw,
th, z, colour)` (main 0x00106c70): `(0, 896, 0, 8, 7, 3000, 0x48808080)`
and `(0, 896, 0, 7, 6, 6000, 0x48808080)`. That sets `texuse` (+0x2e) to 1,
a texture of their own at x 896 of buffer 0. `MakePacket` was run in eemu
(test_stream_rs's machine, sysLayer's view) and its GIF packets decoded.
For each shade it sends:

1. A context-2 strip that draws the whole 512 x 448 picture being drawn
   into 256 x 128 (or 128 x 64) at that address: bilinear, no tests, no
   blend, Z masked.
2. That texture drawn back over the view's box: bilinear, CLAMP,
   `(Cs - Cd) As + Cd` at 0x48, alpha NEVER with FB_ONLY, and Z GEQUAL at
   the Z of depth 3,000 (or 6,000), with no Z write.

Every pixel farther than 3,000 thus takes 56% of a quarter-size copy of
the frame, and beyond 6,000 again of an eighth-size one. Both go on
sysLayer, which draws after the field's objLayer. `ccLayer::Add` (0x00108930)
keeps the root list in descending priority with a new layer after the
equal ones, and `AddAll` reverses it, so of two layers of one priority the
newer draws first. sysLayer is made at start-up and objLayer by `GO`, both
priority 0. The sky, the ground and the objects soften; the characters
and effects, on later layers, stay sharp. `MakePacket` prepends, so the
3,000 shade draws before the 6,000 one.

## The port

- **piney-draw.** New `TexRef::ScaledFrame { width, height }`: the frame as
  the command finds it, resampled as the context-2 strip would.
- **piney-gs.** The command copies the whole frame, as for `FrameBuffer`.
  Under a new `Params::SCALED` bit the shader reads that copy through the
  smaller grid: each of the four bilinear taps of the small texture is
  itself a bilinear read of the frame at that texel's centre. No extra
  pass is needed.
- **Software renderer.** It builds the small image the same way.
- **piney-desktop.** `Sampling::shade_own(tw, th, z, colour)` gives the
  draw-back strip. `Layers::older` holds an older layer's groups, drawn
  after everything else of that priority.
- **piney-world.** `FieldArea::draw` puts the two strips on `older(0)`.

## Model Z was a sixteenth of the GS's

The first shots softened the ground right at Kite's feet. VU1 divides by
w and converts the whole vector with `ftoi4`, so a model's GS Z is 16 z/w
(render.md). The port took z/w. The strips' Z and the effects' sprites
(`rot_trans_pers`, piney-effect's `fixed`) are already at the GS's scale,
so all of those compared as nearer than any model. `piney_draw::MODEL_Z_SCALE`
(16) now scales model depth in piney-gs and in the software renderer. Models
against models are unchanged. The fix also makes effect sprites, the
blades' trails and the rays hide behind the models in front of them, as
the game's do.

## Checks

- `the_field_shades_draw_their_own_texture_back`: `shade_own`'s strips
  against the game's packets from eemu, for both shades. The corners, Z
  (0x00ae43a3, 0x0056e1d9), UVs and colour are equal.
- `a_scaled_frame_reads_through_the_smaller_copy` (piney-gs, on the GPU):
  a black/red split frame read through a half-width copy gives black, one
  pixel halfway, then red.
- `Layers`' order test covers `older`.
- Field 14 shots with and without the shades: before the Z fix 95% of the
  near ground changed; after it, nothing below the horizon band changes,
  and the far hills, the horn, the sky and the distant ground soften. A
  dungeon fight shot (`target_cursor_shot`) draws as before, cursor
  included.
- All 585 workspace tests pass.

**Still unknown:** no picture of the game's own field was compared. The
GS's bilinear weights (a few fraction bits) and its reads past the frame
are not modelled. Model Z past `ftoi4`'s signed range (w below about 16)
is clamped at 2^32 here, where the VU saturates it.
