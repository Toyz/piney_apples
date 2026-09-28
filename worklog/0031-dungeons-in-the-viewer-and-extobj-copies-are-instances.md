---
number: 31
title: Dungeons in the viewer, and ExtObj copies are instances
date: 2026-09-23
area: world, render, format
files: crates/piney-data/src/scene.rs, crates/piney-viewer/src/mesh.rs, crates/piney-viewer/src/main.rs, crates/piney-viewer/src/render.rs
---

# 31. Dungeons in the viewer, and ExtObj copies are instances

With the generator ported ([[30]]), the viewer can build a dungeon floor:
`piney-viewer --dungeon SEED[,TYPE[,SIZE]] [--floor N]` generates it as the
game does and assembles each room from the type's scene file (`sd1` for type
0). In the window, Left/Right change floor, A changes the dungeon type and M
rolls a new seed from the game's RNG. The user ran it and asked where the
walls were.

## Rooms are animations of shared pieces

A room's model (`ANM_sd1l4n40`) is an animation: its object controllers
place kit pieces - floor tiles, wall segments, ceilings - in room space. It
places the same piece many times: 195 controllers, but only 25 distinct
pieces, one floor tile 20 times. Each placement is an ExtObj (0x0a00: `u32
object, u32 parent, u32 target`) whose target is the shared piece and whose
parent is the room's root object (`OBJ_sd1l4n40`).

The frame-0 reader of [[26]] - like `tools/ccsmodel.py`'s `anime_locals`,
which it copied - mapped every controller to its ExtObj target and kept the
first. So each piece was drawn once, and most of every room was missing.

`piney_data::scene` now keeps ExtObj parents (`Scene::ext_parent`):
- `anime_controllers0` returns every controller;
- `controller_worlds` gives each its own world through its own parent
  chain.

The viewer draws one instance per controller: that floor went from 205
drawn models to 975, and rooms come out whole. Town objects ([[27]]) use the
same path and are unchanged.

`tools/ccsmodel.py obj --anime` still collapses copies. Its OBJ export of
such files is missing pieces.

## What is still wrong

**Placement.** The rooms do not join up.

The generator's room data is right: room 1's centre is (30000, 39000) for
cells (32, 44) of size 16, and the Rust matches `dungeon.py` there
([[30]]). But the pieces disagree with the rotation convention. I measured
how far each room's pieces reach on each side, in room space, against the
exits the map gives it.
- Room 2 (medium, `ANM_sd1m2n01`, rotation −π/2, exits N and E) reaches out
  on its local S and E. That fits a standard counter-clockwise rotation.
- Room 7 (large, `ANM_sd1l2n31`, the same rotation and exits) reaches out on
  its local N and W. That fits only the opposite sign.

Both cannot hold. So `rotate`, the centre, or the tables' row order is
applied in a way not yet read.

- **`DUNGEON::SetAnmObject`** (`INF gcmn.prg:0x005c3ad0`) places a room's
  animated objects as `TransMatrix(pos) * RotMatrix((0, 0, rotate)) * M`.
  So rotation is applied, in radians.
- **The gaps.** Rooms stop short of each other (a large room's pieces span
  about ±5,476 of its ±6,000). Doors are separate: `SetDoor`, `OpenDoor`,
  `MoveDoor`, not read.
- **`SetRoom`** (0x005c1ca0) builds only the rooms around the player. It
  also sets each type's fog and ambient light from a table at
  `DUNGEON+0x144` (0x24-byte rows: fog at +0xc/+0x10/+0x14, ambient
  RGB/255 at +0x18..+0x20). That is why the viewer's dungeons are so dark:
  their pieces' baked colours are about 35/128, and the game lifts them
  with that ambient, which the viewer does not apply yet.

## Also

- `B` cycles an exposure of 1x, 2x or 4x (`--bright` for shots), to inspect
  dark scenes. At 1x the viewer draws what the GS would write, lighting
  aside.
- The window now asks for an opaque surface. The compositor had been
  showing the desktop through the frame's alpha.

**Still unknown:** the rotation and placement rule that makes rooms join;
the door models and where they go; the per-type fog and ambient (and the
room lights from `LGT_` objects `SetRoom` picks up); whether rooms off the
player's neighbourhood are ever drawn together as the viewer draws them.
