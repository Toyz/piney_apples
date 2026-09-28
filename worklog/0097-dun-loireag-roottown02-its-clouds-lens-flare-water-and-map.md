---
number: 97
title: Dun Loireag: ROOTTOWN02, its clouds, lens flare, water and map, and the town's game for town 1
date: 2026-09-25
area: world, render, test
files: crates/piney-world/src/town.rs, crates/piney-world/src/town02.rs, crates/piney-world/src/cloud.rs, crates/piney-world/src/lensflare.rs, crates/piney-world/src/map/town.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/town_fx.rs, crates/piney-game/src/world.rs, crates/piney-gs/src/lib.rs, crates/piney-draw/src/lib.rs, tools/test_town02_rs.py, tools/test_map_rs.py, docs/engine/town02.md
---

# 97. Dun Loireag: ROOTTOWN02, its clouds, lens flare, water and map, and the town's game for town 1

[[93]] found the story stuck at event 21: events 20-30 send the party to
Dun Loireag (`game.town` 1) and the port's town refused every town but
Mac Anu. INF's `ROOTTOWN02` is now ported, checked against the game's own
code in eemu, and the session enters it from a save, through the Chaos
Gate's Other Servers and from the scripts' `scene`. The reference is
`docs/engine/town02.md`.

## The class

`ROOTTOWN02::ROOTTOWN02` (gcmn 0x004240c0) differs from Mac Anu's in more
than its tables. It loads `town02` (`town02d` in crisis) and `town_z`,
sets `SetFog(1500, 10000, 0, 75, 0x00f0c080)` and `ccSys.bgColor` to the
same pale blue, and builds four clumps (`CMP_sr2bac1` the sky,
`CMP_sr2clo_1_1` and `_1_2` two cloud layers, `CMP_sr2sun1` the sun), a
`LENSFLARE` of `town_z`, 25 `CLOUD`s, and three `ccAnm`s of
`ANM_sr2wat1a` from town02 itself, rooted at `@988` (0, 4200, 0). Its
`Draw` (0x004266a0) has no clip rules; it spreads its pieces over five
layers: the sky and sun on `bgLayer[0]` and `[1]` (-100, -90), waters 1
and 2, the cloud layers, the flares and the clouds on the effect layer
(20), everything else on objLayer. The objects' static tables hold 20 model
rows and nine animations (eight windmills and the balloons); the balloon
row names a clump, `CMP_sr2bal1`, that town02 does not have, which is why
`tools/statics.py` matched `RT_OBJTABLE02` to no scene.

`piney_world::town::Town` now holds what every class builds the same way
(`Common`) and a `Class` for the rest, so the World, the party code and
the event code keep reading `town.hits`, `town.file` and `town.lights`.

## CLOUD and LENSFLARE

`CLOUD` (gcmn 0x00504380-0x00504d94) is a smoke sprite drawn from the area
generator's `fieldrand`, the global `seed` the fields reseed. Type 0,
Dun Loireag's, gathers within 2500 of the player at the first `Draw`,
drifts east 5-19 a frame and is sent back about him past 5000. Its
`EFF_srzsmo1` has 101 patterns; the pattern is `fieldrand(patNum)` at each
`SetPos`. All four types are ported.

`LENSFLARE::Draw` (0x00503360) is area 15's `DrawLensFlare` ([[84]]) with
two tests added: nothing during a puppet show, and in mode 0 nothing
unless `ccCheckCameraDeg(sun, 12288)`. The flare math now lives in
`lensflare::points`, which `evarea` calls.

## The water and the frame-buffer copy

Both towns do the same thing for water 0, which the port had never drawn:
`waterUVModifi2` (gcmn 0x005025d0) projects the water model's stored,
unbent vertices through the view's `world_screen` and writes their screen
fractions as texture coordinates. `ccLayer::MakePacketDrawBuffTrans`
(main 0x00108e30) then queues a context-2 sprite that copies the draw
buffer, point-sampled, into the 128 x 128 texture the constructor swapped
into water 0. Queued at the front of objLayer after water 0 was sent, it
runs after every opaque town model and just before water 0. The effect is
a refraction: the morph moves the picture of what lies under the water.
Two details needed the game run beside the port. The `fptoui` it calls is
libgcc's soft-float one: 0 for a negative value, where the inline one
wraps. And `ccObj::CheckBoundingBox` copies the local matrix into the
world matrix when the object has no parent, so a probe object's world
matrix is its local one.

To draw it, `piney-draw`'s `VertexEdits` gained `st` (the rewritten
coordinates) and `tex` (a texture in place of the materials', here
`TexRef::FrameBuffer`). `piney-gs` makes the copy when the model's command
starts, which falls at the same place in the draw order. The port copies
the whole frame, not a 128 x 128 point-sampled one, so its refraction is
sharper.

## The town's game

Everything that differs by town was found by the town number in the code.
The start is (0, 3500, 0) facing south (`WORLD_MAN::SetCharPosition`);
`party.rs` now builds Kite at the World's start rather than Mac Anu's. The
gate is at `DMY_gate` (0, 4200, 0). The collision is `HIT_sr2town1hit`,
with 1,356 triangles: 354 floors and 1,002 walls. The merchants are rows
5-10: Weapon Shop, Elf's Haven, Item Shop, Magic Shop, Recorder and the
Grunt Shop breeder at `DMY_merchant6`. The walking PCs' rows are chosen
without regard to the town; their landmarks are `markPosTbl[k][1]` over
`naviMapTown2`. The event markers are ev01-06, 20, 21, 71 and 30. The music
is `sqDataTown` row 1 and the server is 1. `ROOTTOWN02::DrawMap`
(0x00425760) puts the arrow on the font layer in screen pixels and dims a
sign to alpha 64 while the arrow's point is on its balloon. Town 1 also
calls `ccSetDog` (four dogs, npcTbl 141-144) and `ccSetChibiGuso` before
the walking PCs; neither is ported.

## The session

- **Entering.** `World::enter` builds the town of the save's `lastTown`
  (0 or 1). `--mode world:1` starts there. The session's change of scene
  accepts either town. The field UI gets server 1, and the frame is
  cleared to `bgColor`.
- **Events 20-22.** Event 20's replayed `town_move 1` puts Dun Loireag on
  the gate's list. So `--mode story:22` goes from the board through Log in
  to Mac Anu, and then to Dun Loireag through the gate. There event 22's
  `in_town 1` blocks take over from its Mac Anu ones: block 13 plays,
  since Piros is not in the party.

## Checked

`tools/test_town02_rs.py` builds `ROOTTOWN02` with its own constructor in
eemu (over town02 and town02d).

- **The constructor.** Its fog, background colour, files, rows, roots,
  lights, water times and flares match the port's.
- **`Draw`.** It ran for 700 frames with the player wandering the town and
  beyond and the camera turned every way, including at the sun and in the
  eye view and puppet shows. 200 more frames ran in crisis from another
  seed. Every piece matched, in order with its layer. After each frame so
  did `SetUV`'s value, the scrolled materials' offsets, all 25 clouds'
  state, `fieldrand`'s seed and the scrolls. That covered 5,940 cloud
  draws and 2,052 flares; clouds were sent back in 7 of the frames.
- **`waterUVModifi2`.** It ran on Mac Anu's water (4,714 vertices, reach
  9000) and Dun Loireag's (330 vertices, reach 32000), at 300 camera
  places each. Every coordinate matched, including the calls out of reach.
- **The minimap.** `tools/test_map_rs.py`'s new `test_dun_loireag` ran
  `ROOTTOWN02::DrawMap` for 191 frames: 2,992 packets, 168 of them dimmed,
  none differing.
- **The Rust tests.** `piney-world`'s `tests/dun_loireag.rs` checks the
  collision counts, the markers and the arrival, then runs Kite. In
  `piney-game`, `session::dun_loireag` has two tests. One warps Mac Anu to
  Dun Loireag and back through the gate as a player picks it. The other
  plays `story:22` as above.
- **Pictures.** `dun_loireag_shots` (ignored) takes them. The shots show
  the gate, the bridges, the Weapon Shop and the lens flare toward the sun
  in the south-west. The clouds show as pale squares of mist. Their texture
  is a square of alpha about 0.35 with a narrow clear border, and most of
  it fails the alpha test (AFAIL FB_ONLY), so it draws by colour.
- **The old suites.** `test_world_rs.py`, `test_map_rs.py`,
  `test_field_rt.py`, `test_fieldui_rs.py`, `test_evarea_rs.py` and the
  workspace's tests pass.

**Still unknown:**
- `ccDog` and the chibi Grunties (`ccSetDog`, `ccSetChibiGuso`, `setDog`)
  are not ported. Town 1 has neither, and the walking PCs do not collide
  with them.
- The copy for water 0 is the whole frame. The game's is 128 x 128,
  point-sampled; modelling that needs a scaled frame-buffer copy in
  `piney-gs`.
- Water 0's place for `waterUVModifi2`'s reach is taken as its root, since
  the animation has no record for the object. Only the function itself was
  checked, not what the object's coordinate holds in the running game.
- `fieldrand`'s state on entering the town. In the game it is whatever
  `WORLD_MAN::Init` (seeded from the frame count) or the last field or
  dungeon left. The port starts each town at 13. The scrolls' statics
  (`u$1777`, `v$1346`, `v2$1347`) likewise start at 0 with each town
  rather than carrying across visits.
- What the clouds look like on the console: a square of mist drawn by
  colour past a failing alpha test is what the data and `ccEff`'s blend
  state say, but no capture confirms it.
- `ROOTTOWN03`-`05` and their
  `DrawMap`s are not ported.
