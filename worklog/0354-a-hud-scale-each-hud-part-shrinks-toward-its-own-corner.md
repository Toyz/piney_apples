---
number: 354
title: "A HUD scale: each HUD part shrinks toward its own corner"
date: 2026-10-02
area: ui, render
files: crates/piney-fieldui/src/spr.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/render.rs, crates/piney-desktop/src/sprite.rs, crates/piney-desktop/src/layers.rs, crates/piney-game/src/area.rs, crates/piney-game/src/main.rs
---

# 354. A HUD scale: each HUD part shrinks toward its own corner

Not the game's. The user expected render scale to make the HUD smaller,
as a PC game's does at a higher resolution. Render scale adds detail
([[350]]) to a picture that always fills the window, so a separate
`hud_scale` (0.5 to 1; console `hud_scale`, `--hud-scale`) now does it.

## Anchors

One sprite queue can carry several HUD parts. `MenuFont`, for one, holds
the party panels' HP and SP digits and the target window's numbers in a
single send. So an anchor has to come from the code that draws each part,
not from the queue. Each packet now carries `spr::Anchor`, a point per axis
in the menu's 512 x 448 space, or none. `Disp` sets it on every sprite
before each part:

- party panels: bottom left;
- target window and new-mail mark: top left;
- bracelet gauge: top right;
- battle band: its height alone, toward the top;
- battle announcement: the centre.

Everything else keeps its size: the menus, the message window,
world-anchored marks (enemy bars, the target cursor, balloons, damage
numbers) and the dim. The renderer scales a packet's position (its offset
folded in) and its size about the anchor. At 1 it leaves the packet
untouched, so the game's frames, and the 65 harness tests, are unchanged.

## The minimap and two seams

The field's minimap (`piney_world::map`) draws whole into the layers.
`Layers::mark` and `map_since` let `area.rs` shrink just its commands
toward the top right: vertices, scissors (the inclusive ends widened to
whole pixels) and texture coordinates.

At 0.6 a light line crossed the map. 0.5 and 0.75 were clean, and the
line also showed at render scale 1. A dump of the map's primitives found
it. The terrain is four tiles, and each stops 1/16 pixel short of the
next (the top tile ends at 121.625, the bottom one starts at 121.6875). On
whole-pixel positions that gap never covers a pixel's centre. Shrunk to
fractional ones it can, and the sky showed through. A shrunk sprite's far
edges are now taken from the next tile's start (+1/16 before scaling), so
neighbouring tiles share one edge. Clean at 0.55, 0.6 and 0.8.

A shrunk cell's texture coordinates are pulled in by half a texel
(`Sprite::uv_inset`, `sprite::inset_uv`). A bilinear sample at a
fractional edge then stays inside its cell of the atlas.

`the_hud_scale_shrinks_the_panels_into_their_corner` (piney-fieldui)
checks the frame at 0.5: the same primitives in the same order, every
panel primitive half as wide and moved toward the corner, and the frame
back to the game's at 1. A field shot at render scale 2 with HUD scale
1.0 and 0.6 shows the panels in the bottom left and the map in the top
right.

**Still unknown:** the menus, the message window and the towns' own
displays keep their size; which corner each menu should keep is a choice
not yet made.
