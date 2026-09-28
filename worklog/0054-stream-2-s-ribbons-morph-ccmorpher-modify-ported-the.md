---
number: 54
title: Stream 2's ribbons morph: ccMorpher::Modify ported, the tendrils around the figure
date: 2026-09-23
area: render, test
files: crates/piney-data/src/model.rs, crates/piney-draw/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-desktop/src/soft.rs, crates/piney-stream/src/draw.rs, docs/engine/stream.md
---

# 54. Stream 2's ribbons morph: ccMorpher::Modify ported, the tendrils around the figure

The user remembered animated lines in event 1's first cutscene (stream 2),
where the port drew four straight brown bands that only moved with the
figure. [[52]] had left `F_Morpher` decoded but not drawn.

## Finding it

- **Where the ribbons come from.** `OBJ_ribbon_a00`-`d00` draw models from
  the preload file `str0001e`. That file has one frame, so nothing in their
  own records changes their shape.
- **The morphers.** Stream 2 holds four morphers, `MPH_ribbon_a00`-`d00`.
  Each blends two targets, `MDL_ribbon_*39` and `*40`, with weights such as
  0.4 and 0.6 that the `F_Morpher` records change frame by frame. Each
  ribbon's Obj2 names its morpher as its modifier.
- **The rest of the stream's effects.** `Func_str0001` (3,400 bytes) directs
  them: `ccRasterNoize`, `ccBufferSampling` feedback, `ccBufferReverce`,
  fades and fog, cued by the stream's notes. It does not touch the ribbons,
  and it is still not ported.

## `ccMorpher::Modify` (0x0013af10)

`ccModel::Draw` calls its modifier for each rigid mmat before the packets.
Modify writes the blended positions to a work buffer that stands in for the
mmat's own:
1. **Rescale.** A target whose vertex scale differs is brought to the base
   model's, once and in place: `trunc(v * target / base)`.
2. **Weights.** Each weight becomes 1/4096 fixed point (`vftoi12`),
   replicated into halfword lanes.
3. **The blend.** Per vertex and axis, in the EE's 16-bit SIMD:
   - the target's difference from the base (`psubh`, wrapping);
   - times the weight, summed in 32 bits over the targets (`pmaddh`
     through HI/LO);
   - shifted down 12 (`psraw`, a floor);
   - added back to the base with 16-bit saturation (`paddsh`).

`Model::morph` in `piney-data` does the same.
- `ModelDraw` gained a `morph` list (target model, weight).
- The stream draw fills it from the node's modifier, its targets resolved
  to models of the drawn model's file.
- The GPU renderer and the CPU GS model both draw the blended positions.

## The result

At frame 822 the four ribbons are no longer straight bands. They are thin
tendrils winding around the figure, and by frame 850 they have moved.

## Checked

- **The arithmetic.** `morph_blends_as_the_ee_does` covers the floor of
  the shift, a target at another scale, and the 16-bit wrap and saturation:
  -32000 toward 32000 at weight 1 wraps to -1536 and saturates at -32768.
- **The stream.** `ribbons_are_morphed`: at stream 2's frame 822 exactly
  four model draws carry two targets whose weights sum to 1, and each
  blend moves the base positions.
- **The rest.** The workspace's tests pass, and clippy and fmt are clean.

**Still unknown:**
- **Not against a run of the game.** `Modify` itself is ported from the
  disassembly. The EE's lane-by-lane result is not compared with a run of
  the function.
- **Still not drawn.** `Func_str0001`'s effects: raster noise, feedback,
  the reversed buffer, fades and fog.
- **Other callers.** The desktop and title files have no morphers; town or
  field models that do will need the same modifier path.
