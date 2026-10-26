---
number: 490
title: npc_act -3, row 139's drain, ported and checked against the game; the Administrator's -5 already was, now checked too
date: 2026-10-25
area: world
files: crates/piney-world/src/rtownpc.rs, crates/piney-game/src/world.rs, crates/piney-game/src/town_fx.rs, crates/piney-world/examples/world_probe.rs, tools/test_world_rs.py
resolves: 291.2
started: 2026-10-25T19:10:39-06:00
took: 14m
---

# 490. npc_act -3, row 139's drain, ported and checked against the game; the Administrator's -5 already was, now checked too

Play question [[18]]/[[65]]/[[98]]/[[116]] (291.2): `npc_act -3` (row
139's drain, with `effVirusCrystal`) and `-5` were said not to be ported.
Only Mutation's event 114 uses them, alternating
`npc_act 139 -3` with `npc_act 29 -5`.

**-3, row 139's drain** (`ccRtownPC::eventMode`, INF gcmn 0x00508760,
MUT 0x00524d50; the case at MUT 0x00525264; Infection has it too).
Every row but 139 stands. Row 139 (`+0x124`) steps by `evActProcess`:
- 0: transrate 1, `anmTbl[5]`;
- 1: `anmTbl[6]`, then as 2;
- 2: `ccSeOn(79)`, `effAfterDrain(pc, 0)`, `ccSeOn(60)`, then
  `effVirusCrystal` and `effRemoveTrap(pos, 103, 121)` at a copy of its
  place;
- 3: transrate less 0.02 a frame; below 0 it is 0, the step 4 and
  `actCnt` 0.

The port had `-3 => {}`. Now `RtownPc::event_mode` runs the steps, and a
new `PcEvent::Drained { pos }` gives the world sounds 79 and 60. It also
gives the town's effects `Start::AfterDrain`, `VirusCrystal` and
`RemoveTrap`, in the game's order.

**-5, the Administrator's force rings** (`ccMerchan::sysopeAct`, INF
gcmn 0x00506170) was ported already ([[132]]: `Merchant::sysope_act`,
`SysopEvent::Vanish`). The docs' Unknown was stale.

Checks against the game:
- `TownPcsAgainstGame.test_frames` gains a run that adds row 139 (its
  stem now loaded), shows it (act 3), then drains it (act -3).
  - The harness records `effAfterDrain`, and `effVirusCrystal` and
    `effRemoveTrap` by value, since the case passes a copy of its place.
  - The probe gives `["drained"]` for the new event.
  - Frame by frame, position, clip, time, transparency, act, step and
    events agree, on Infection and on Mutation (`PINEY_VOLUME=mutation`).
    One drain in each run.
- `MerchantsAgainstGame.test_event_merchant` gains an `act` op on both
  sides (`+0x1f4` = N, `+0x1f8` = 0). The Administrator is set to act 2
  at frame 500 and to act -5 at 520, and the run asserts he reaches its
  step 2. The force rings and `ccSeOn` are hooked and recorded. It passes
  on both volumes.

Docs: field-game.md (the -3 table and the checks; its Unknown line
gone), events.md and effects.md (their lines gone), UNKNOWNS.md.

**Still unknown:** nothing
