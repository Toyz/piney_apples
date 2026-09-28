---
number: 47
title: The title's lit icons against VU1: the directional term was doubled
date: 2026-09-23
area: render, test
files: crates/piney-gs/src/convert.rs, crates/piney-draw/src/lib.rs, tools/vu.py, tools/test_demo_rs.py
---

# 47. The title's lit icons against VU1: the directional term was doubled

The user found the title's icons wrong: grey, dull sides, and a selected
icon that looked broken. In [[43]] they were the one part of the title that
differed from the CPU GS model, and that model draws without light. So the
title agent checked the lighting against the game's own VU1 code instead.

## How it was checked

`test_icon_lighting` in `tools/test_demo_rs.py` runs the port's title to
the menu. At two frames it takes each lit object the port draws (12
objects, 1,964 vertices) with the port's `lwMatrix`, and runs it through
the game's own code:
- `Init`'s light: the `ccLight` constructor, `ccOmniLight::Init`,
  `ccSetColor`, the matrix, `AddGrp`;
- `ccDrawEnv::SetLightMatrix` and `ccSetMatrixPacket` in eemu, with VU0
  in macro mode;
- VU1's `mc_SetMatrix`, `mc_SetObjParam`, `mc02_Start00` / `01` and
  `mc_DrawTriL`, on a new micro-mode interpreter (`tools/vu.py`'s `Vu`).

## What it found

- **The setup was right.** The port's `Lights` equalled VU1's registers
  after `mc_SetMatrix`: the directions (vf05-07), the doubled colours
  (vf09-11) and the ambient (vf12).
- **The formula was not.** Only the vertex colours differed: 585 of 1,964,
  all on faces the light reaches.
- **The cause.** VU1 dots the raw s8 normal, whose length is 64, with the
  direction, and scales the ambient by 128. Against the unit normal
  `piney-gs` uses, the doubled colours therefore count half. `mmat`
  counted them whole, so every lit face was twice as bright in its
  directional term. The fix is a factor of 0.5, and the doc on
  `piney_draw::Lights` now gives both forms.

With the fix all 1,964 vertices match VU1 exactly, none off by one.

## What the user saw

The bug made lit faces brighter, not darker. The dull sides are how the
game lights them:
- The one white omni light sits at (-8500, 0, 6500); the icons are at
  x = -22400.
- A face turned away from the light gets only the ambient, 0.35 of its
  texture.
- The selected icon's texture is white, so mid-spin it shows as a grey-white
  bar under the same light.

No title animation carries a light or an ambient, `title1` has no light
chunk, and `cc3d` is the active environment whenever the title draws, so
nothing else lights the icons.

## Checked

- `test_demo_rs.py`: 16 tests OK, `test_icon_lighting` among them.
- `piney-gs`: 12 tests.
- `piney-demo` and `piney-game` tests pass; clippy and fmt are clean.

**Still unknown:**
- **Other lit models.** The desktop's and the field's lit models, if any,
  have not been checked against VU1 the same way. Only the title builds
  `Lights` so far.
- **The "transparency" report.** It is not reproduced beyond this. After
  63635fb and this fix, the GPU matches the CPU GS model on the desktop's
  text and the title's unlit parts.
