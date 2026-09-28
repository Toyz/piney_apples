---
number: 150
title: The travellers' hum: tobjSeLoopStart and tobjSeLoop
date: 2026-09-26
area: world
files: crates/piney-stream/src/draw.rs, docs/engine/stream.md, crates/piney-audio/src/se3d.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-world/src/field_ambient.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/field_area.rs, crates/piney-data/src/field/render.rs, crates/piney-game/src/area.rs, tools/test_sound3d_rs.py, tools/test_field_ambient_rs.py, docs/engine/sound.md, docs/engine/field.md
---

# 150. The travellers' hum: tobjSeLoopStart and tobjSeLoop

Half of a GAPS field item. Δ's airship and Λ's whale, the servers'
travellers that now and then cross a field (TOBJ), hum as they pass. The
port drew them in silence.

## What the game does

Two functions of main, called only by TOBJ, speak to the synthesizer
directly rather than through `ccSeOn3DLoop`:

- **`tobjSeLoopStart(pos)`** (0x0017bf20), from `TOBJ::Init` for servers
  0 and 2. Unless `ccSnd +0x133` is already set, it takes the first free
  `loopID` slot, keeps it in `looptest` (0x003789dc), and starts
  `seData[44]` (program 31, channel 15, note 60) with a note on at 127
  followed at once by expression 0: the loop runs silent. `+0x133` is set.
- **`tobjSeLoop(pos, rate)`** (0x0017c0d0), from `TOBJ::Draw` while the
  traveller's transparency is not 0, sets the loop's pan and volume. The
  volume is `calcVel`'s arithmetic with a reach of 15,000 per 256 of decay
  (7,500 here) instead of 5,500, clamped to 0-127 (never -1), then scaled
  by the transparency, so the hum fades with the picture. Behind the
  camera it also bends the note as `ccSeOn3D` does.
- **The end.** Nothing turns the note off. `+0x133` is cleared by
  `initBeforeLoad`, so the next `ccSndSQLoad` (whose sound off silences
  the note) stops the updates, and by the desktop's `ccSndChangeData`.
- Ω's sleigh (server 4) calls `tobjSeLoop` too, but no hum was started for
  it, so it stays silent.

## The port

- **piney-audio.** `se3d::tobj_se_loop_start` and `se3d::tobj_se_loop`
  build the messages. `Driver` gains `tobj_loop` (`+0x133`, cleared in
  `sq_load` and `desktop`) and `looptest`. `Audio::tobj_se_loop_start` and
  `Audio::tobj_se_loop` send them on port 0.
- **piney-world.** `Ambient::generate` marks `tobj_se_start` for servers 0
  and 2. `Tobj::draw` emits `Op::SeLoop(pos, alpha)` before its far test,
  as `Draw` calls it. The field runtime sends `Op::SeLoopStart` once, on
  the first drawn frame, which comes after the area's `ccSndSQLoad` as in
  `ccSetupGameCtrl`.
- **piney-game.** The ops become `Event::TobjSeLoopStart` and
  `Event::TobjSeLoop`, heard from the active camera.

## A crash found on the way

A 300-frame run of the ambient sweep panicked in `cell_height`.
`FIREFLY2::SetBasePosition2` asks the height at its rotated offset (the
game's slip). The harness's camera turn had drifted to -8.4 radians, where
the game's sine no longer holds (matrix entries near -25), so the offset
landed 58,000 away. `FIELD::GetHeight` wraps only once and read past the
map. The game reads whatever memory lies there; the port indexed out of
bounds. A real camera's turn is a 16-bit angle, so play stays within -pi
to pi. Still, `cell_height` now gives 0 outside the map instead of
panicking, and the harness keeps its turns within -pi to pi.

## Checks

- `tools/test_sound3d_rs.py`'s new `test_tobj`: 1,510 calls of
  `tobjSeLoopStart` and `tobjSeLoop` mixed with `ccSeOn3DLoop`, random
  cameras, points and transparencies, run in eemu against the probe: 0
  mismatches. `test_loop` still has 0 in 1,506.
- `tools/test_field_ambient_rs.py`: TOBJ's calls are now recorded (the
  starts after `Generate`, each frame's `tobjSeLoop` with its place and
  rate) instead of stubbed. 124 cases of 1,000 frames: 0 mismatches,
  with 8,463 calls of `tobjSeLoop` among them.
- `the_airship_hums` (piney-world): a type 10 field of seed 1 on Δ has
  the airship; the start is asked; 120 frames each send one hum, the rate
  rising by 0.01 from 0.01 to 0.8.
- `tobj_hum_starts_silent_and_follows_distance_and_fade` (piney-audio).

## Also: the streams' fog by depth

The streams' models still took one fog value at their centre. They now
carry `piney_draw::DepthFog` like every other model since worklog 0145,
so VU1's per-vertex fog is drawn in the streams too (stream.md).

**Still unknown:** the hum was not listened to against the game. TOBJ's
shadow (`ccShadowModel`) is still not drawn.
