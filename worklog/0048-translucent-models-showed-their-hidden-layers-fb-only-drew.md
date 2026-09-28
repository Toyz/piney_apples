---
number: 48
title: Translucent models showed their hidden layers: FB_ONLY drew colour before Z
date: 2026-09-23
area: render, test
files: crates/piney-gs/src/lib.rs, crates/piney-gs/src/convert.rs
---

# 48. Translucent models showed their hidden layers: FB_ONLY drew colour before Z

The user reported that transparency on the desktop looked wrong. [[47]]
ruled out the title's lighting. The cause was in the GPU renderer: the way
it drew an alpha test whose failing pixels keep their colour but not their
Z (AFAIL = FB_ONLY).

## Finding it

Earlier comparisons ran the GPU and the CPU GS model
(`piney_desktop::soft`) in separate runs. This time one desktop frame went
through both renderers:
- **Frame 545.** The frame is taken after the cursor has moved. 1,520
  pixels differed, all in the icon column: the icons were near white on the
  GPU and translucent on the CPU.
- **Narrowing it down.** Drawing one model at a time left six icon models,
  all translucent (mmat alpha 0.75, 0.5, 0.2).
- **One pixel of one icon.** On the CPU, the icon's nearer layer (Z 5591)
  drew 0xff at alpha 96 (191 over black), and a farther layer (Z 5587)
  failed the Z test. The GPU gave 239: that farther layer blended on top.

## The cause

The desktop's models draw with the alpha test GEQUAL (the mmat's
reference, usually 0) and FB_ONLY, with Z writes on.
- **The GS.** It writes each pixel's Z as it draws. A model's nearer layer
  therefore hides the layers drawn after it, back faces included.
- **The GPU.** It split every FB_ONLY draw into two passes: first colour for
  the whole draw with Z writes off, then a Z-only pass. In the colour pass
  nothing had written Z yet, so every layer of the model passed the test
  and blended in.

## The fix

- **Pass order.** An FB_ONLY draw now draws its passing pixels first, with
  colour and Z, in drawing order as the GS does. A second pass adds the
  failing pixels' colour without Z (a new `Params` bit, 19, discards the
  passing pixels there).
- **No split when nothing can fail.** A test that cannot fail (ALWAYS, or
  GEQUAL 0) is drawn as one pass.

The result differs from the GS only where a failing pixel meets a passing
one drawn after it in the same draw.

## Checked

- **One desktop frame through both renderers.** Pixels that differ between
  the GPU and the CPU GS model, before and after the fix:

  | frame | before | after |
  | --- | ---: | ---: |
  | 100 | not taken | 2 |
  | 400 | not taken | 18 |
  | 545 | 1,520 | 87 |

  The remaining pixels are single pixels on triangle edges and one row at
  the selection bar.
- **The GS tests.** `fb_only_hides_later_farther_layers` draws a nearer
  sprite, then a farther one at alpha 0x60, in one draw. The farther one is
  hidden: 0xbf over black, as one layer gives. `piney-gs` has 13 tests,
  all passing, and clippy and fmt are clean.

**Still unknown:**
- **The remaining pixels.** Edge rasterisation (the GS's fill rules against
  the GPU's) and the selection-bar row are not explained.
- **Failing-pixel order.** No test covers a draw where failing and passing
  pixels of the same draw overlap. Where they do, the blend order differs
  from the GS.
- **The CPU model is a model.** Both renderers are ours; neither has been
  compared with a real GS frame capture.
