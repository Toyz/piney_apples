---
number: 55
title: ccMorpher::Modify against the game: the weight's lanes and the EE rescale
date: 2026-09-23
area: render, test
files: crates/piney-data/src/model.rs, crates/piney-data/examples/morph_probe.rs, tools/test_morph_rs.py
resolves: 54
---

# 55. ccMorpher::Modify against the game: the weight's lanes and the EE rescale

[[54]] ported `ccMorpher::Modify` from the disassembly alone. This entry
runs the game's own function against the port.

## The check

`tools/test_morph_rs.py` runs `Modify` (0x0013af10) natively in eemu, with
`test_anim`'s VU0 macro and MMI support. Each run sets memory up as
`ccModel::Draw` leaves it for the call:
- the morpher and its targets (`ccModel`, index, chunk and `ccMmat` each);
- `ccDrawModelParam` with rwflag bit 0 set, so the blend is written in
  place instead of into a `GetWork` buffer (the arithmetic is the same);
- `ccSys`'s work pointer (+604) at free memory.

It compares every position with what the `morph_probe` example gives, over
302 morphers:
- one to four targets;
- weights in and outside [0, 1], negative ones included;
- positions over the whole s16 range;
- targets at the base's scale and at others;
- two cases for the 16-bit wrap and saturation.

## What the first runs found

Simple cases matched from the start, which showed the harness was right.
Two things did not:

- **Negative weights.** The weight goes into halfword lanes as
  `(r << 16 | r) << 16 | r`, where `r` holds `vftoi12`'s result `x` in its
  low word and, above it, the float's `lw` sign extension `y`. That leaves:
  - `x.lo` in the x lane;
  - `x.lo | x.hi` in the y lane;
  - `x.lo | x.hi | y.lo` in the z lane.

  For weights in [0, 8) all three are the weight. A negative weight leaves
  the y and z lanes at -1. `Model::morph` now spreads the weight the same
  way.
- **The rescale.** A target at another scale is multiplied by
  `div.s(target, base)` with `mul.s`, then truncated by `vftoi0`. In IEEE
  arithmetic the factor rounds to nearest, and a value could land one off.
  The rescale now uses `piney_data::field::ee`, the bit-exact EE float
  arithmetic `test_field_rs.py` already checks against eemu.

After both changes, all 302 morphers give equal positions.

**Still unknown:**
- **`GetWork` never ran.** The first-call path, where the blend goes into
  a work buffer from `ccDrawPacketCtrl::GetWork` and replaces the mmat's
  `vertexData`, is not run here. Its arithmetic is the same code.
- **Weights in the data.** Whether any stream uses a negative weight or one
  of 8 or more, where the lanes differ, is not surveyed.
