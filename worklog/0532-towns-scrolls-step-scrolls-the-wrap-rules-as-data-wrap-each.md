---
number: 532
title: Towns' scrolls: step_scrolls, the wrap rules as data (Wrap), each water one WaterSpec
date: 2026-10-30
area: world
files: crates/piney-world/src/town/, plans/rust-cleanup.md
started: 2026-10-30T13:42:26-06:00
took: 11m
---

# 532. Towns' scrolls: step_scrolls, the wrap rules as data (Wrap), each water one WaterSpec

The first part of section 2's towns item (`plans/rust-cleanup.md`): the
scrolls. Each Root Town stepped its `DrawBG` scrolls and its water by a
copied loop. One rule a kind now, the rules as data. Nothing they compute
changed.

## The rules

`town::Wrap` is how a scroll comes back once a step takes it to 1:

- `Past`: above 1, back to 0. Every town's `DrawBG` scrolls.
- `AtOrPast`: at 1 or above, back to 0 (`!lt`). Dun Loireag's and
  Carmina Gadelica's water.
- `Less`: above 1, less 1. Mac Anu's water.

`Wrap::step(u, step)` adds, then wraps.

- **`step_scrolls(scrolls, steps)`** is `DrawBG`'s: each scroll by its
  step, `Past`. It replaces six loops (Mac Anu, Dun Loireag, Carmina
  Gadelica, Fort Ouph, Lia Fail, the relic city). Some stepped every
  scroll before wrapping any, some each in turn; the scrolls do not read
  each other, so the order is the same. Dun Loireag's third scroll is
  `Wrap::Past.step` on its own.
- **`WaterSpec { step, wrap }`**: each town's water is one constant
  (`WATER`), 0.005 a frame with its wrap, and `WATER.step(u)` in place of
  three copies.

The diff is about 70 lines added and 65 removed (the rules are new code;
the towns lose theirs).

## What is left of the item

- `Sky`: Dun Loireag's and Fort Ouph's clouds and lens flare (the started
  flag, the clouds, `patNum`, the rng, the flares, the sun, their
  sprites) as one member.
- `Base::passes` and the statics slots.

## Checks

- piney-world's and piney-game's tests, release.
- Against the game: `test_town02_rs` and `test_town03_rs` on all four
  volumes, `test_world_rs` on Infection.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** nothing
