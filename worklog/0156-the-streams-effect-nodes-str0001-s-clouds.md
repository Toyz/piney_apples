---
number: 156
title: "The streams' effect nodes: str0001's clouds"
date: 2026-09-26
area: render
files: crates/piney-stream/src/scene.rs, crates/piney-stream/src/lib.rs, crates/piney-stream/tests/stream.rs, crates/piney-stream/tests/stream_fixture.txt, tools/test_stream_rs.py, docs/engine/stream.md, docs/engine/animation.md
---

# 156. The streams' effect nodes: str0001's clouds

A GAPS item: the streams' effect objects. The port's scene kept an effect
node only for its place in the draw list's layer order, and drew nothing
for it.

## What there is

A survey of every stream file found Eff nodes in four streams and Particle
nodes in none:

- **str0001** (stream 2, "Epitaph of the Twilight"): 181 nodes
  `EFF_srzsmo00` to `EFF_srzsmo180`, all `EFF_srzsmo1` of `str0001e`, each
  with an F_Obj record every one of its 2,041 frames. They are the clouds
  the relic stands among. Without them the relic stood on a bare dark
  floor.
- **str0580**: 15 smoke nodes. Their records keep them at the origin, so
  they never start (stream.md already noted they draw nothing).
- **str0570** and **str9104**: one Eff node each, with no records.

## The game's code

- `ccEffObj::Init`, `StartAnm`, `EndAnm`, `Animate` and `DrawNoAnm`, and
  `DecodeF_Obj`'s effect branch (stream.md, "Effect nodes").
- A node's "part" bit comes from its Obj2 flags. `Decode_Eff` copies them
  into the Eff chunk's +0x10.
- With the part bit, a stopped node restarts wherever a record puts it
  off the origin. The clouds loop that way.
- A record also sets the Eff's scale, turn and transparency, and a
  translation-only matrix.

## The port

- `piney_stream::scene::EffNode` holds each node's state: its parent, its
  Eff chunk and `patNum`, the part bit, and what the records set.
  `Scene::new` builds them in the ended state.
- `f_obj_eff` is the record's branch.
- `Scene::reset` ends them, as `ResetScene` does.
- `Scene::eff_world` gives `DrawNoAnm`'s `lwMatrix`.
- `Stream::draw_effect_nodes` draws each running node through piney-effect's
  `Eff`. The template for each chunk is read into the effects' assets once.
  This happens when the player gives the stream its effects, which
  piney-game always does.

## Checks

- **The fixture.** `tools/test_stream_rs.py fixture` now lets `DrawNoAnm`
  run in eemu and records each `ccEff::Draw(pattern)`: the pattern, the
  place `DrawNoAnm` set, the scale, the turn and the transparency, as a
  hash of the sorted records a step. Stream 2's run grew from 60 to 200
  steps so that nodes stop and restart. That is up to 181 draws a step,
  and the port's nodes match on every step.
- **Nothing else moved.** No other scene of the fixture draws an effect,
  and its other lines are unchanged.
- **Pictures.** `stream_shot --stream 2 --frame 900` with `--fx` shows
  the white cloud sea under the relic. Without it, the bare floor.

**Still unknown:** the clouds were not compared with the game's own
picture. The effect nodes of `ccAnm` clumps in other files (effects.md)
are still not drawn; no such file was found to have one.
