---
number: 474
title: Root Towns' scroll statics live in the executable's data and carry across visits, log ins and resets, with the arena's, dungeons' and field's; test_evarea_rs passes on all four volumes
date: 2026-10-25
area: world, test
files: crates/piney-desktop/src/save.rs, crates/piney-desktop/src/lib.rs, crates/piney-world/src/town.rs, crates/piney-world/src/town01.rs, crates/piney-world/src/town02.rs, crates/piney-world/src/town03.rs, crates/piney-world/src/town04.rs, crates/piney-world/src/town05.rs, crates/piney-world/src/town06.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/story_map.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/evarea_b0.rs, crates/piney-world/src/evarea_sky.rs, crates/piney-world/examples/evarea_probe.rs, crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/session/tests/mirror_world.rs, tools/test_evarea_rs.py, docs/engine/root-towns.md, docs/engine/town02.md, UNKNOWNS.md
started: 2026-10-25T09:59:11-06:00
took: 28m
---

# 474. Root Towns' scroll statics live in the executable's data and carry across visits, log ins and resets, with the arena's, dungeons' and field's; test_evarea_rs passes on all four volumes

Two items left after worklog 473: `tools/test_evarea_rs.py`'s
`test_frames` error, and the Root Towns' scroll statics, which the port
started at 0 with each visit.

## test_evarea_rs

On a clean worktree of HEAD^ (23bc0d9, its own target), `test_frames`
errors the same way, KeyError 'arms'. It predates worklogs 472 and 473.
Worklog 378 (86c2571) added `arms` (`ArmsEffect` / `ClearArmsEffect`
from `ccPlayer::Main`) to the shared world harness's state and to
`world_probe`, but not to `evarea_probe`. That probe now reports it as
`world_probe` does.

The harness then ran on Infection and Mutation but not on Outbreak or
Quarantine. Three fixes:
- **Stripped vtables.** Their executables name no `EVENTAREA02` or `B0`
  vtable. Where none is named, the check is that the constructor stored
  one.
- **Unnamed functions.** `EVENTAREAB0::Draw` is called through its
  vtable (+0x08). `SwitchLayer` is found as the function just before
  `Draw` that opens on `lw $v1, n($a0)`. Its field is +0x2b8 on
  Outbreak, not Infection's +0x2b4.
- **A port bug.** On Outbreak, frame 52 of a random walk gave Kite's
  `transparency` 3 ulps off. `evarea_probe` built its player and camera
  without the disc's volume, so it took Infection's double `sqrt` where
  Outbreak's code has `sqrt.s`. It now builds them as `world_probe`
  does.

The harness passes in full on all four volumes (7 tests each).

## The scroll statics

Each Root Town's `DrawBG` (and `Draw`, for its water) scrolls by
function statics:
- Mac Anu: `v$1334` (two), `u$1841`.
- Dun Loireag: `v$1346` (two), `v2$1347`, `u$1777`.
- Carmina Gade: `v$1474` (two), `u$1875`.
- Fort Ouph: `v$1269` (two).
- Lia Fail: `v$1244` (two), `v2$1245`.
- Quarantine's town 5: three words at gp 0x00279980.

Infection's symbols place every one in `main`'s small data
(0x00378168-0x00378b98), 0 in the image. Outbreak's draws reach the same
kind of gp words (0x00386e98-0x00386ee8). They are not gcmn's data, so
GCMN.PRG's load at log in leaves them, and `ccThMother`'s soft reset
boots nothing. Each scroll goes on from its town's last visit until
power-off.

The same holds for the other draws' statics the port models:
- `EVENTAREAB0::DrawBG`'s `v$1597`;
- `DUNGEON::DrawWater`'s `u$8862`;
- the lakes' `DrawBG` `v$8917`;
- `WORLD::DrawBG`'s sky `v$4519`. Its first use is guarded by
  `init$4520`, also in main, so it too is 0 once.

**The port.** `SaveState::scroll_statics` holds the 22 slots, named in
`town::scroll_statics`:
- **Towns.** `RootTown::take_statics` and `put_statics`; `World::enter`
  hands them in and `state_out` takes them back.
- **The rest.** The arena (`StoryMap::take_statics`), `DungeonArea` and
  `FieldArea` take them as they are made, and `FieldWorld::state_out`
  hands them back.
- **Soft reset.** `SaveState::carry_memory` carries them through, with
  `rand()`, `fieldrand` and gcmn's scrolls (until log in).

The desktop's `Stage::Setup` now boxes its state, which grew past
clippy's size limit.

A sky map's draw while the tasks slept, before its first step, indexed
the clouds a step makes and then put back. It now draws the pieces
before putting the state back. `the_sky_maps` checks it.

## Checks

- `session::tests::mirror_world::dun_loireags_scrolls_carry_across_visits`:
  Dun Loireag, its words to field 66, Gate Out back. The field carried the
  town's `u$1777`. The town back goes on from it, as does `v$1346`, and
  log in leaves the statics. It fails with the town not taking them
  (`u$1777 restarted: 0.46 then 0`).
- `save::tests::a_soft_reset_carries_memory` and
  `evarea_b0::tests::the_arenas_scroll_is_the_executables`.
- `test_evarea_rs` on all four volumes; `test_town02_rs` and
  `test_town03_rs`.
- The piney-game suite (331 passed); piney-world, piney-data,
  piney-fieldui and piney-desktop; clippy and fmt.

**Still unknown:** `ccSys.count` at `WORLD_MAN::Init` on a console: the
frames GCMN.PRG's read takes, which only the drive decides (worklog 473).
The field's other function statics (`field_ambient::Statics`, TOBJ's
cloth) still carry over in a process-wide store rather than the state.
They behave the same within a run.
