---
number: 483
title: The game over's squeeze reaches only sysLayer's view: the maps, menus and font keep their place, and a dungeon's fog zero only keeps the clear black
date: 2026-10-25
area: world
files: crates/piney-desktop/src/layers.rs, crates/piney-desktop/src/anm.rs, crates/piney-desktop/src/fade.rs, crates/piney-game/src/gameover.rs, crates/piney-game/src/area.rs, crates/piney-world/src/draw.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs
resolves: 283.4
started: 2026-10-25T17:23:07-06:00
took: 10m
---

# 483. The game over's squeeze reaches only sysLayer's view: the maps, menus and font keep their place, and a dungeon's fog zero only keeps the clear black

Play question [[158]] (283.4): at the game over's television the port
squeezed the whole finished frame into the band, and never zeroed
`DUNGEON.fog`. The code settles both.

**Which layers squeeze.** `ccLayer::Init(pri, view)` (main 0x00108260)
keeps the view it is given; with 0 it makes a view of its own with the
default frame. `ccLayer::AddAll` (0x00108570) starts each layer's packet
with its own view's scissor (+0x220). So the TV's `SetFrame` on sysLayer's
view reaches only the layers made with that view. The 81 calls split:

- sysLayer's view: `WORLD_MAN::GO`'s field layers, `ccEffectCtrl` (60),
  `ccNoiz` (241), the bosses' layers (1, 2, 200, 241, Fidchell's 254),
  the flag race's (100, 200).
- Their own: the maps (`mapLayer` 50 in `DUNGEON`, `ROOTTOWN0n` and MUT's
  `WORLD::Init`; Dun Loireag's and Carmina Gade's 40 and 45), the hack
  menu (127, 128), `ccThGameOver`'s 129, the font (240), `ccMenuCtrl` and
  the events' windows (242), `BOOK` (243) and `ccThMother`'s 254
  (`ccGame.layer`).

The port now squeezes per layer: `Layers::flatten_with` hands each command
its priority and whether it is the older layer's, and
`gameover::through_sys_view` picks. Two layers share 254, so `ccGame`'s
fades (`FADE_LAYER`) now go to the older layer (`fade::draw_older`), as
the game's order already had it: the mother made it at boot, before any
boss. In `game_over_shots` the status pane, the minimap and its label
now stay whole and in place while the field shrinks to the line.

**The fog.** `NOISE` phase 3 (gcmn 0x00516ed4) writes `ccSys.bgColor` 0
and the RGBAQ of both clear packets (+0xdd0, +0xce0). In a dungeon
(`game` +0x14 = 2) it also zeroes `dungeon[game.dungeon]` +0x50, the fog.
`DUNGEON::Draw` (0x005cee68) copies `fog` into `bgColor` only when they
differ. With both 0 the clear stays black. `fog`'s other readers are the
constructor and `SetRoom` (found by every +0x50 access in DUNGEON's
functions), and a paused field builds no room. So the zero's only effect
is a black clear to the end. The port's game over already latches that
(`Step::black`), so nothing changes; the comment in `AreaMode::step` now
says why. docs/engine/dungeon.md said `Draw` also sets the GS fog colour
from `fog`; it does not (that is `SetRoom`'s `SetFog`), and is fixed.

Checks: `the_squeeze_leaves_the_layers_with_views_of_their_own` (every
priority above, older and newer 254), `flatten_with_names_each_layer`, the
old `a_squeeze_maps_the_frame_into_the_band`, and `game_over_shots`
looked at (frames 276 and 283).

**Still unknown:** nothing
