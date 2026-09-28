---
number: 35
title: The viewer plays animations as the game steps them
date: 2026-09-23
area: render, test
files: crates/piney-viewer/src/mesh.rs, crates/piney-viewer/src/main.rs, crates/piney-viewer/src/render.rs, crates/piney-data/src/scene.rs
---

# 35. The viewer plays animations as the game steps them

With playback worked out ([[32]]), the viewer no longer stops at frame 0.
The user had asked why `--anime` did nothing to Mac Anu's flags and ships.

- **What moves.** When the viewer builds a scene, each model posed by an
  animation remembers how:
  - a town object's instance remembers its one controller, under its row's
    root;
  - bone and skin models remember the whole clump the chosen Anime chunk
    poses.

  It also keeps the vertex spans each mmat wrote.
- **How often.** The viewer counts game frames at 29.97 a second
  (`frameRate` 2, as in towns and fields). On each new frame it poses only
  those spans again and sends only them to the GPU.
- **How each animation runs.** Each animation steps one frame (256 ticks)
  per game frame from 0. It loops or holds its last frame as its chunk
  says (`Animation::looped`). ExtObj copies stay separate instances, through
  a matrix form of `Scene::controller_worlds`.
- **Controls.** Space or P (Options on a pad) pauses. `--time SECONDS` poses
  a `--shot`.

## Checked

`town_objects_play` builds Mac Anu and poses it at game frame 45. More than
100 vertices move, all stay finite, and posing frame 0 again restores every
vertex exactly. Two shots of town01, 2 seconds apart, differ where the
flags and boats are.

**Still unknown:** the morph blends (F_Morpher: cloth that waves by morph
targets, such as `xp_flag`) are not applied yet; texture-offset animation
waits on how the draw applies `ccMaterial.u/v`; the transparency channel
is read but not drawn.
