---
number: 144
title: The target cursor turned and the drain gauge shaded: TRF and TG sprites
date: 2026-09-26
area: ui
files: crates/piney-desktop/src/sprite.rs, crates/piney-fieldui/src/render.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/field-ui.md
---

# 144. The target cursor turned and the drain gauge shaded: TRF and TG sprites

A GAPS drawing item. The menus' packets were right, and the harness
compares them with the game. The drawing of two sprite types was not:

- **The target cursor** (`targetCursol`, `SetPrim` TRF) drew as a square
  where the game draws a diamond. Its middle cell turns by pi/4.
- **The bracelet's gauge** (`menuDrain`, TG) drew each cell in its first
  corner's colour. The game shades the virus cells between two colour
  pairs.

## The game

`ccSprite::MakePacketStr` (main 0x0015aed0) makes both as a textured
TRIANGLE_STRIP of four corners, where TF makes a SPRITE.

- **The strip.** The corners are (top left, bottom left, top right,
  bottom right), with the UV of each. Mirroring swaps the pairs as the
  sprite does.
- **The rotated types (bit 0x2).** The corner is `RotZ(-rot)` of (cx, cy)
  plus (dx, dy). The edges are the turned (sx, 0) and (0, sy). Each
  corner goes through `ApplyLayerScreenMatrix`. A cell is dropped only
  when all four corners are off the screen. `dx` and `dy` move on by the
  turned first edge, and a shadow moves the corner by the view's scales.
- **The Gouraud types (bit 0x4).** They send an RGBAQ before each UV and
  XYZ2, from the colour word pairs at +0x68, +0x78, +0x88 and +0x98 in
  corner order. Each alpha is multiplied by `transp`.

The minimap's second sprites had the rotated path already (piney-world
`map::sprite`, bit-exact through the VU's `cossin`).

## The port

`piney_desktop::sprite::Sprite` gains `rot` and `vcol`. `make_packet`
draws either kind as the strip (`make_strip`), and TF stays the SPRITE it
was. `piney-fieldui`'s render sets `rot` for `targetCursol` and `vcol`
from the packet.

- **Unit tests.** `a_rotated_cell_is_a_turned_strip` checks the diamond's
  corners and the turned step. `a_gouraud_cell_keeps_its_corners` checks
  the colours by corner.
- **The picture.** `target_cursor_shot` (ignored) walks Kite up to event
  17's Administrator and saves the frames with him targeted. The diamond
  and its four corner marks are drawn as on the console.

**Still unknown:** the menu's turn uses `f32` sine and cosine, not the
VU's `cossin` (a sixteenth of a pixel at most). A rotated cell's
off-screen test in the game reads a width and height left over from an
earlier unrotated cell; the port takes 0.
