---
number: 235
title: Mutation's interNoiz and the lessons it lacks
date: 2026-09-27
area: ui
files: crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md
---

# 235. Mutation's interNoiz and the lessons it lacks

`tools/test_fieldui_rs.py` failed 9 of 55 tests on Mutation. It now passes
on Mutation (5 skipped) and on Infection (1 skipped, as before).

## The lessons: 8 failures

The five tutorial tests failed, and so did three that open the lesson's
item box (menu 84): the 99-items and full-bag tests. Each failed at the
same place. As the menu opened its first line, the game had four lines of
text and the port one.

The tutorial menus (75 - 85) open their lines from `evMsgTblM1[2..4]`:
- On Infection, these are the lesson events' messages (Orca's lines).
- On Mutation, `evMsgTbl[0]` points at a group of 50 entries that are all
  zero in the executable. Mutation's events 1 - 49 are Infection's
  scripts left in the executable. The generated `mut_.evs` has no
  messages for any of them; Mutation's story starts at event 50.
  - Mutation's `TOPPAGE.PRG` has none of the lesson's text either.

So on Mutation a tutorial menu reads its record through NULL. The game's
four lines were whatever the harness had at address 0. The scripts that
open these menus are events 2 - 4, which cannot run there.

The harness's changes:
- `has_lessons()` reads `evMsgTbl[0][2..4]`.
- Without lessons, the five tutorial tests skip.
- The three item-box tests use the action button's box (menu 32) instead
  of 84. Both hand the item to `GetItemMenu`, so the 99-items and
  full-bag flows are still checked on Mutation.

## interNoiz: 1 failure

Mutation rewrote `Disp`'s `interNoiz` (MUT 0x005403c8 - 0x005406fc).
Outbreak's and Quarantine's are the same code. The event `noise level=N`
(from Mutation on) sets N + 2, so there are three new levels:
- **3:** on every 64th frame, always on the 128th and otherwise when
  `rand() & 1`, `SetNoiz(rand() % 10 + 10, rand() % 10 + 15, rand() % 10
  + 4)`. It no longer depends on town 4.
- **4:** every 16th frame, `SetNoiz(255, 255, 255)`.
- **5:** every 32nd frame, a noise and `cameraShake(2, 1, 30, 2)`. The
  noise is level 3's random one when bit 6 of the count is clear or
  `rand() & 3` is 0, else full. Every 8th frame, also sound 258.

Levels 1 and 2 are Infection's.

In the scripts, Mutation's event 301 (M401) uses level 3. Outbreak's and
Quarantine's events 301 and 313 use level 5.

The first failure was the harness's own, though. It counted a `SetNoiz`
as Disp's only if the call returned into Infection's interNoiz range,
translated to the other volume. Mutation's block is longer, so level 1's
call was left out. The range now ends at Disp's own `ccNoiz::Draw` call.

The port's changes:
- `disp::inter_noiz` takes Mutation's branch on volumes after Infection.
- It asks for the shake through the new `Request::CameraShake`. The area
  sends it to `FieldWorld::camera_shake`. The town sends it to the new
  `World::camera_shake`, which draws the town's `rand()`.
- Camera 1 applies the shake in both.

The harness adds five cases on the later volumes: levels 3 (rand 0 and 5),
4, and 5 (rand 0 and 3). A `cameraShake` hook logs the game's shake.

**Still unknown:** Whether a Mutation save can ever make events 2 - 4
run; the scripts' conditions need events 1 and 2 done.
