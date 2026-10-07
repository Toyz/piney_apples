---
number: 389
title: Stream and desktop cameras are placed and projected in VU0's arithmetic: every view matrix is the game's bit for bit
date: 2026-10-07
area: render, video, test
files: crates/piney-desktop/src/camera.rs, crates/piney-desktop/src/view.rs, crates/piney-desktop/src/anm.rs, crates/piney-desktop/src/frames.rs, crates/piney-desktop/tests/camera.rs, crates/piney-desktop/tests/camera_fixture.txt, crates/piney-stream/src/scene.rs, crates/piney-stream/src/draw.rs, crates/piney-stream/src/lib.rs, crates/piney-stream/tests/stream.rs, tools/test_stream_rs.py, docs/engine/desktop.md, docs/engine/stream.md, UNKNOWNS.md
resolves: 388
---

# 389. Stream and desktop cameras are placed and projected in VU0's arithmetic: every view matrix is the game's bit for bit

Entry 388 left one gap. The port drew stream cameras through
`piney_desktop::camera::Camera::world_view`, a glam inverse of
`T(pos) Rz Ry Rx Rx(pi)` with glam's sine. libvu0's `_sceVu0ecossin`
makes sine as `sqrt(1 - cos^2)`, about 5e-5 off near 0. So stream 18's
steps 33 and 34 put `world_screen` 3.3e-4 of a column off the game's, and
the fixture's bound had been loosened to 4e-4. `Scene::world_view_bits`
had the game's camera arithmetic, but only the sort key and the test used
it.

## What the game does

Every view the desktop and the streams draw through comes from
`ccView::SetView` (0x001052c0). It has three callers: `PlaySceneMain`, `ccAnm::Draw`
and the field's `cameraSet`.

- **The camera.** `DecodeF_Camera` (0x0014ece0) turns the record's
  degrees into radians as `(pi deg) / 180` and calls
  `ccCam::SetMatrix_PosRotXYZ` (0x001386b0). That is the unit matrix
  turned a half turn about x, then about x, y and z with
  `sceVu0RotMatrixX/Y/Z`, moved by `sceVu0TransMatrix` (xyz only), then
  `sceVu0InversMatrix` (0x001107b0). `PlaySceneMain` places the stream's
  camera again every frame with `SetMatrix_PosRotXYZDebug` (0x001385a0).
  That call is the same, with the stream's matrix (+0xc0, the unit matrix)
  multiplied in after the half turn. A record without a fov (flag 0x100)
  keeps the camera's own.
- **The projection.** `SetView` does it all on the EE and VU0:
  - `scrz = 512 / (2 tanf((0.0174533 fov) / 2))`, with newlib's `tanf`.
  - `ccSetViewScreenClipMatrix` (0x00101f60) builds a scale-and-centre
    matrix. It holds `ax`, `ay * screenAspect`, the centre, and
    `az = (far near (zmax - zmin)) / (far - near)`.
    `cz = (-zmax near + zmin far) / (far - near)` uses a `mula.s` and a
    `madd.s`, whose product is rounded before the add.
  - It `sceVu0MulMatrix`es that with the distance matrix
    `(scrz, scrz, z -> w, w -> z)`.
  - `world_screen` is that view screen times the camera's matrix.
- **The field, towns and battle** go through `cameraSet`:
  `SetMatrix_PosTarget`, then the same `SetView`. The port already had both
  in VU0's arithmetic (`piney_world::camera::pos_target`, `VIEW_SCREEN`),
  and `tools/test_world_rs.py` and the other camera harnesses check them
  bit for bit. Nothing else builds a view through glam.

## Fix

- **`piney_desktop::camera`** is now the game's arithmetic, in float bits:
  - `Camera` holds pos, rot (radians), fov and `matrix` (`ccCam.matrix`).
  - `Camera::pos_rot_xyz` is `SetMatrix_PosRotXYZ`, `Camera::debug(m)` is
    `SetMatrix_PosRotXYZDebug`, and `Camera::decode(flag, values)` is
    `DecodeF_Camera`.
  - `view_screen`, `screen_z` and `screen_aspect` are the EE's;
    `world_screen` is `vu_mul(view_screen, matrix)`.
  - The glam `world_view` is gone.
- **`View`** is built by `View::new(projection, camera)` and `set_camera`.
  It keeps the `world_screen` that `SetView` makes. The draw reads
  `world_screen_bits()` and glam's `world_screen()` takes it from there.
- **Streams.** `Scene::world_view_bits` and `camera_bits` are gone.
  `Scene::camera` is `DecodeF_Camera`'s camera, and `Scene::view_camera` is
  `PlaySceneMain`'s debug placement. The draw, the sort key, the shadow
  view and the effects' camera (`fx_camera`'s `world_view` and
  `world_screen`) all use it.
- **The desktop's `ccAnm`.** `frames::camera_records` gives the flag and
  the values as bits, and `Anm` decodes them onto its camera. A flag-bit-0
  record now changes nothing, as in the game, and a record without a fov
  keeps the camera's.

## Checked

- **`scenes_play_as_the_game_plays_them`**: every frame's `world_screen`
  now equals the game's bit for bit. That is all 1837 frames of streams 0,
  106, 2, 15, 6 and 18. The fixture's `%.9g` reads back exact. The old
  bounds were 4e-4 for glam and 1e-5 for `world_view_bits`.
- **`tools/test_stream_rs.py cameras`** (new) runs `SetFrame`,
  `SetMatrix_PosRotXYZ` and `SetView` natively in eemu and writes
  `crates/piney-desktop/tests/camera_fixture.txt`. It covers
  `ANM_xddcamer` and 300 seeded cameras: near-zero turns, whole turns,
  records without a fov, and the default, letterbox and mail frames.
  `cameras_place_and_project_as_the_game_s` matches the camera matrix and
  `world_screen` of all 301 bit for bit.
- **`default_view_screen_is_the_ee_s`**: the ported view screen at fov 45
  equals `piney_world::camera::VIEW_SCREEN`, the field's constant (scrz
  0x441a827a, aspect 0x3f955555, az 0x4f00003f, cz 0xc4ffe07f).
- **The records.** A one-off survey covered every F_Camera record in
  Infection's streams and `DATA.BIN`: 44089 records in 1089 files. All of
  them have flag 0. No record leaves out a value, so the game's stack
  leftovers for a missing position or turn never arise.
- **Shots.** In `/mnt/data/claude/scratch/cam389`, each before/after pair
  was compared by eye and by pixel diff.
  - Stream 18 frame 33 (`s18_f33_pair.png`, `s18_f33_diff.png`): scattered
    single-pixel edge and texel changes from the sub-pixel camera move. The
    effects' streaks are the same in both.
  - The desktop after 200 frames (`desk_pair.png`, `desk_diff.png`): at
    most 5 levels of gradient rounding.
  - The title's stream 0 frame 200: 25 pixels by 1.
  - No golden image or pixel hash in the repo moved.
- piney-game's suite (four threads), the tests of piney-desktop,
  piney-stream, piney-demo and piney-toppage, clippy on the touched crates,
  fmt, `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing new from this entry. The model draws still
multiply `world_screen` by each object's `lwMatrix` in glam, where VU1
does it in its microcode; this entry covers only the view.
`docs/engine/field-walk.md`'s note on `ghoCam`'s markers is about object
matrices too. The console's own Movie 13 and 16 from the desktop (entry
388) still need a PS2 or PCSX2 capture.
