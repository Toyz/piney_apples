---
number: 465
title: Quarantine's field staff roll: STFROLL_VOL4 ported and checked frame by frame against the game's code, drawn on sysLayer, its music alone over field 126's empty event bank
date: 2026-10-24
area: render, audio
files: crates/piney-effect/src/stfroll.rs crates/piney-effect/examples/stfroll_probe.rs tools/test_stfroll_rs.py crates/piney-gen/src/manifest.rs crates/piney-data/src/tables/stfroll.rs crates/piney-gen/src/placement/sound.rs crates/piney-audio/src/driver.rs crates/piney-event/src/host.rs crates/piney-event/src/vm/exec.rs crates/piney-world/src/combat/mod.rs crates/piney-world/src/field_world.rs crates/piney-world/src/camera.rs crates/piney-game/src/fx.rs crates/piney-game/src/area.rs crates/piney-game/src/area_host.rs crates/piney-game/src/session/tests/ending.rs crates/piney-eemu/src/sys.rs crates/piney-eemu/examples/ee_run.rs docs/engine/stfroll.md docs/engine/sound.md
started: 2026-10-24T14:22:50-06:00
took: 1h 14m
---

# 465. Quarantine's field staff roll: STFROLL_VOL4 ported and checked frame by frame against the game's code, drawn on sysLayer, its music alone over field 126's empty event bank

## The roll

Event 314's block 3 runs `ending_kanji` in field 126. That is opcode
169, QUA main 0x001c52e8, and only Quarantine's executable has it. It
starts `STFROLL_VOL4`, a thread in gcmn (0x004f0950, priority 66) with
no symbol, and waits for it. [[464]] left it unported.

**What the code does** ([stfroll.md](../docs/engine/stfroll.md)).
- **The object.** It is 0x150 bytes, built by the constructor 0x004f0aa0
  and `Init(0)`. Each frame main (0x004f1880) runs ten functions: the
  scroll, the logo's fade, the text or logo draw, the states, the
  animations' transparency and draw, the generators, the phases, the late
  comets and the comets' frame. Once +0x108 is set the thread sets its
  tscb's +0x14 and only breathes.
- **What it shows.**
  - `TEX_hackrogo` fades in, holds 240 frames and fades out.
  - `ccBgmPlay(2)`, then 17 `ccKanji` lines of the credits scroll up
    0.55 a frame, over the `ending` scene's ring.
  - The ring's clip ends, the glows go and the book comes up.
  - Comets of particles fly Catmull-Rom paths.
  - The last page shows `TEX_hackrogo_2`, then `TEX_hack_url`.
  - After a 480-frame tail, `ccBgmStop`.
- **The layer.** The text is on its own layer 129, which takes sysLayer's
  uncentred view. The logo is a mask on sysLayer. The ring and the book
  draw on whatever `ccLayer::active` holds, which in the ending is
  sysLayer:
  - Each frame `SetActiveLayer(0)` sets it.
  - `ccLattice::Disp`'s empty path stores its caller's stale `$s0`
    there.
  - Kite's `ArmsEffect`, which would store -1, is skipped in act 14,
    where `pc_act 0 6` has put him.

  `ee_run` showed this, with `EE_PRINT` (each call with the active layer)
  and a new `EE_THEN` that starts the roll from a stub task over a save
  before 314's field part.
- **Lighting.** None of `ending`'s 82 models is lit (`mtype` bit 0).
- **The music data.** Field 126 asks the event bank, row 126.
  Quarantine's `sqDataEvent` runs to 127 rows, and row 126 is -1, so no
  bank loads. `eventPlayType[126]` is 3. The port's play-type tables were
  cut at Infection's 125 bytes, so it gave -1: a port bug.

**The port.**
- `piney_effect::stfroll` holds the rules, a function for each of the
  game's. The comets publish their place as anchors for the generators
  that sync to them.
- piney-gen's `stfroll` group gives `piney_data::tables::stfroll`: the
  generator and particle rows, the comets' random ranges and the two line
  tables.
- `Op::EndingKanji` waits on `Wait::EndingKanji`. `AreaHost` starts the
  roll (`FieldWorld::ending_start`, the player at the origin) and waits
  while it runs.
- `FxTasks::ending` runs it with the field's effects:
  `AreaFx::ending` makes it the first frame, then calls main.
- `Ending::draw` sends the kanji, the mask and the anms (unlit, sysLayer,
  the event camera).
- `AreaMode::ending_asks` applies its camera (`Camera::set_event`) and
  turns its music into `BgmStream(2)` and `BgmStreamStop`.
- The play-type tables now take a row for each row of their `SQ_LOAD`
  table, up to the next global. Only `eventPlayType` grows, 125 to 127,
  on Mutation, Outbreak and Quarantine.

**Checked.**
- `tools/test_stfroll_rs.py` runs the game's constructor, `Init(0)` and
  main in eemu, with `ending.ccs` loaded and the particle calls hooked.
  It compares them with `stfroll_probe`, frame by frame: every field, the
  lines, the comets, the generators, each draw call with its arguments,
  and the camera and music calls. All 12000 frames agree.
- `event_314_plays_through_field_126` runs the roll for 11742 frames
  under the pilot, and the event ends.
- `the_field_staff_roll_plays_alone` routes every frame's events through
  `crate::handle` from 314's start. No sequence plays at any frame of the
  roll. Track 2 streams from the roll's frame 320 to its end, then stops.
- `field_126_has_no_bank` checks the bank row.
- `staff_roll_shots` (ignored) writes twelve beats, each looked at. The
  first logo is centred. The credits scroll with the ring, and comets fly
  behind them. The book comes up in particles, with "Produced by" over
  it. The player page shows; Project .hack shows over the blue sparks.
  The URL holds to the last frame.
- The first shots had the text and logo off by (256, 192): the
  centred view of the desktop's roll had been used. sysLayer's own view
  fixed it.
- The test saves have no user name, so the player page's "User Name:"
  is blank, as it would be on the desktop's own roll.
- The first suite run failed 12 effect tests. The roll's call went
  through `Combat::fx_call`, which marks the frame's shows seen, so the
  effects' pass after it missed them. `Combat::fx_run` now runs a task
  without taking the shows, and the roll goes through it.
- The suites, clippy and fmt pass.

**Still unknown:** nothing. The roll's pixels are not compared with the
game's output, since piney-eemu draws nothing. The rules, the draw calls
and the layers that place them are.
