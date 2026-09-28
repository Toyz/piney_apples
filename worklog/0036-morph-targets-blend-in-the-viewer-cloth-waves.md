---
number: 36
title: Morph targets blend in the viewer: cloth waves
date: 2026-09-23
area: render, test
files: crates/piney-viewer/src/mesh.rs
---

# 36. Morph targets blend in the viewer: cloth waves

[[35]] left morphs out: the viewer played rigid and clump animations but did
not apply F_Morpher records.

Each Morpher chunk names a base model. When a job's own animation has a
morph track for that morpher, playback does the following at every game
frame:
- takes the last F_Morpher record at or before the frame (its targets and
  weights);
- blends each of the base's mmats with the matching mmat of every target
  through `anim::morph`, the integer `ccMorpher::Modify` of [[32]];
- places the result instead of the stored positions.

A file with morph animations now plays the first one by default, the way a
file with bone models always has.

`xp_flag` is a flag whose cloth is two morph targets. It waves: 63,546
bytes of a 480 x 360 shot change between 0 and 0.5 seconds, and
`morph_cloth_plays` checks that its vertices move between game frames 1
and 15.

**Still unknown:** which of the 11,086 F_Morpher records are faces and which
are cloth (the viewer does not need to know); texture-offset animation, which
waits on how the draw applies `ccMaterial.u/v`.
