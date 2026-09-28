---
number: 84
title: Area 15's story map: EVENTAREA02, the holy ground and the church, its door and lens flare, checked against the game
date: 2026-09-24
area: world, render, test
files: crates/piney-world/src/evarea.rs, crates/piney-world/examples/evarea_probe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/map/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, tools/test_evarea_rs.py, docs/engine/evarea.md
---

# 84. Area 15's story map: EVENTAREA02, the holy ground and the church, its door and lens flare, checked against the game

Event 11, where Kite reads the Book of Twilight, ends in story area 15,
Hidden Forbidden Holy Ground. The survey of the opening arc listed its
map as a blocker: area 15 is not a generated field but a hand-built one,
`EVENTAREA02` over the scene file `se1_2`, and the port built a field
from its words instead. An agent has now ported the map, its church and
the door between them. docs/engine/evarea.md has the detail.

## What happens now

- **Arriving.** `WORLD_MAN::GO(1)` takes its story-map branch when the
  story area's `EVENTAREA_INFO.model` is set: `eventAreaFlag`, the
  -48000..48000 bounds, and a new `EVENTAREA02`. Kite arrives at
  `DMY_marker01`, at the foot of the bridge up to the church, from the
  Chaos Gate's words (from `--mode story:11` too) or with
  `--mode field:15`. The event sound bank (5) is loaded.
- **The map.** `ChangeBlock` builds a block from its tables: static
  models, the lanterns' animation, background clumps, the lights of an
  animation, the fog, and the start place. The draw follows
  `EVENTAREA02::Draw`, layer by layer: the scrolling clouds and water,
  the mountains without fog, the lanterns and their reflection under the
  church's floor (`revAnm`).
- **The ground.** Each block's collision is its floor model's one Hit
  chunk, enabled as the `STATICMODEL` is built. On a story map
  `ccLandHitCheck` never falls back on a height map, and
  `WORLD_MAN::GetHeight` is 0.
- **The door.** `WORLD_MAN::Enter` on area 15 changes the block
  (`ChangeBlock`) and then the scene's block. The set-up after it keeps
  the map: `WORLD_MAN::Quit` spares `eventmap` for area 15 when the next
  area is a field. Kite then stands at once, at `DMY_marker01_2` inside
  the church and at `DMY_marker02` outside it: `ccPlayer::ccPlayer`
  plays the arrival only coming from a town.
- **The lens flare.** Six `ccEff`s of `town_z`, drawn in the church
  toward the holy ground's sun, over everything on `effLayer`, through
  piney-effect's sprite draw.
- **The battle.** The field's frame (the battle's tasks, the effects) runs
  over the map as over a field. `EntryGimmick` places nothing on a story
  map, so only the events bring enemies there.

## How it was found

- **SetActiveLayer's jump table.** It is not in order: 3 is `effLayer`
  and 4 `floorLayer`. The first draw check put the flare and the clouds
  on `floorLayer`; the game's own draw in eemu put them at priority 20.
- **The lights.** `ChangeBlock` deletes `lightList[lightNum - 1 .. 1]` and
  never index 0, which looked like a light leaking into the church. It
  does not: deleting the light animation destroys its lights, and
  `ccLight::~ccLight` takes each out of the group.
- **The door's ground.** The doorway floor that carries the Enter bit is
  half the doorway (x 0..190). The bridge's walls lead Kite onto it.

## Checked

`tools/test_evarea_rs.py` builds `EVENTAREA02` with its own constructor in
eemu, over `se1_2` as `Pieces` serves it, beside the `evarea_probe`
example:

- **The set-up.** The constructor, then the church, the holy ground and
  the church again: the models, the static object, the clumps, the
  lights, the hits enabled, the fog, `eventStartPos`, `revAnm` and the
  flares' `Init` and render state.
- **The draw.** 662 frames of `EVENTAREA02::Draw` in both blocks, with
  every piece in order with its layer and matrix, the animations' times,
  the flares' positions and `MoveTexture`'s offsets. The lens flare alone
  is checked for 400 random camera states.
- **The ground.** `ccLandHitCheck` at 1,500 points of each block.
- **The door.** `WORLD_MAN::Enter` from game block -1, 1, 0, 1 and 0,
  with the `ChangeScene` it asks, then `SetCharPosition`.
- **The walk.** 3,000 frames of camera and player over both blocks: up
  the bridge into the church's door, random pads, the nave and out of its
  door, with `Enter` on the same frames. None differ.
- **In the runtime.** Three `piney-game` session tests: `--mode field:15`
  through both doors; the gate's words from Mac Anu; and the same from
  `--mode story:11`, with the event task's passes ending in play.
- **The rest.** The workspace's tests, clippy, fmt, the docs check and
  `tools/test_field_rt.py` pass. Re-run in a clean worktree on top of
  [[83]]: the story map, field, world, map and dungeon suites.

**Still unknown:**
- The models are drawn without `SetFog`'s fog, which the game applies per
  vertex.
- `EA_moveTex02`'s scroll restarts with a new map after the town, where
  the game's carries on.
- The other `EVENTAREA` classes are not ported: their areas still get
  generated fields.
- Event 11's instructions in the map are the events' work.
