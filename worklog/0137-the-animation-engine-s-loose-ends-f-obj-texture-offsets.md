---
number: 137
title: The animation engine's loose ends: F_Obj, texture offsets, cameras, lights and frame speed
date: 2026-09-26
area: engine
files: crates/piney-data/src/anim.rs, crates/piney-world/src/foe.rs, crates/piney-data/tests/anim.rs, docs/engine/animation.md
---

# 137. The animation engine's loose ends: F_Obj, texture offsets, cameras, lights and frame speed

[The animation page](../docs/engine/animation.md) had three open items
([[32]]). A scan of every Anime chunk in DATA.BIN, counting its records by
kind and file, settled them.

## What was found

- **Texture offsets (`ccMaterial.u/v`).** The draw applies them through
  the model packet's STROW, `(u >> 4) & 0xff` in 1/256 of the texture.
  [render.md](../docs/engine/render.md) and
  [desktop.md](../docs/engine/desktop.md) already said so, and the port
  already does it (`uv_rows` in piney-desktop's `anm.rs` and piney-world's
  `pose.rs`). The 21 files that animate them are:
  - the Chaos Gate;
  - the fields' waters (`field_a`, `field_c`);
  - Aura's body and the virus cores;
  - a few enemies;
  - the desktop and the title.
- **F_Obj (0x0101).** This was a real gap. Four files use it:
  - Three enemies, `eex1`, `eii1` and `elgx`, move their body with it in
    their damage, down and magic clips: one record a frame, with no
    controller on the body.
  - The desktop's `xddesk01` has some as well.

  `ccStream::DecodeF_Obj` (main 0x0014e950) sets the object's matrix
  through `ccCoord::SetMatrix_PosRotZYXScale` (0x00138120): `T(pos) ·
  sceVu0RotMatrix(π · deg / 180) · S(scale)`. It also sets `localtp` to
  the transparency, clamped to 0..1. The port skipped these records, so
  those enemies did not flinch or fall.
- **Cameras and lights.** These records are read where they are played,
  not by the pose:
  - the streams' scenes and the desktop's clips read F_Camera;
  - `town::anim_lights` reads F_Ambient and the distant and omni
    controllers, for the towns, the gate, the story maps and the
    dungeons' special rooms;
  - the fields read frame 0's ambient and distant light.
- **`frameSpd`.** The field does change it:
  - Kite's `AnimCtrl`, by act;
  - the town player's motion;
  - the enemies' and members' motion;
  - the Administrator's act -5 (384).

## The port

- **Reading.** `piney_data::anim` reads F_Obj into `Animation::objs`, as
  `ObjRecord`: object, ExtObj target, position, degrees, scale,
  transparency, flags.
- **Posing.** `obj_poses_at(time)` gives each driven object the last
  record read by then, from frame 1, as the per-frame reader does.
  `locals_at` and `transforms_at` use it for objects no controller
  drives, and `Foe::node_alphas` takes its transparency.
- **The check.** `f_obj_records_pose_the_body` checks `ANM_eex1dmg0`. It
  has 31 records on `OBJ_eex1body` and none at frame 0. At frame 3 the
  body is 7.75 up, turned by the record's degrees, and it holds there
  through that frame and past the end.
- **Other readers.** The `other` list now counts only F_Camera and the
  lights.

**Still unknown:** four things.
- The direct (0x0605) and spot (0x0607) light controllers are not
  evaluated. The shrine's `LGT_se1_3lig1` has a position, a colour and a
  float pair (498, 500) in a field no other light kind has.
- F_Obj's flag byte and its effect-object branch are not modelled.
- Whether an F_Obj pose outlives its clip is not checked.
- F_Obj is not checked in eemu: the composition is the controllers'
  (already checked), with the decoders' radians.
