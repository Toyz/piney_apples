---
number: 454
title: Field 8's last stage: EVENTAREAB0's NextStage and the clump Corbenik's lasers hide
date: 2026-10-24
area: world
files: crates/piney-world/src/evarea_b0.rs crates/piney-world/src/story_map.rs crates/piney-world/src/field_world.rs crates/piney-world/src/combat/boss.rs docs/engine/evarea.md
---

# 454. Field 8's last stage: EVENTAREAB0's NextStage and the clump Corbenik's lasers hide

## Field 8's last stage and the lasers' clump

Corbenik's third form calls QUA gcmn 0x0030aa40 to clear and set
`worldman->eventArea` +0x2bc. The question (worklog 453) was what that
is. The arena is field 8's `EVENTAREAB0`; Quarantine's class differs
from Infection's in its last stage.

**What the code says.**
- The constructor (0x00308b90) loads `se4_8` beside `se4_7` on every arena:
  scene at +0x2a8, `ANM_se4_8bac1a` at +0x2ac, `CMP_se4_8f01` at +0x2b0.
  `stageSW` (+0x2b4) starts 0. On field 8 with `saveData[0x652f]` set it
  runs `NextStage` (0x0030aa70).
- `NextStage`, field 8 and `stageSW` 0: deletes 50 STATICMODELs and 25
  STATICOBJECTs, `DelGrp`s the lights from the last down to 1 (light 0
  stays), sets `stageSW` and +0x2bc, `HitEnable`s the clump and the clip
  and sets the clip's hit matrix.
- `Draw` with `stageSW` (0x0030a968): the clip goes forward and is drawn at
  the origin on objLayer; the clump `CMP_se4_8f01` on floorLayer only while
  +0x2bc; nothing else.
- 0x0030aa40 sets +0x2bc = (a1 == 1). The tera laser (step 3, count 29)
  and the reflected laser (count 15) clear it under their dark stage
  effect and set it again as the effect ends. So the floor clump vanishes
  while the laser plays; hits are untouched.

**The port.** `evarea_b0::Arena` loads `LastStage` for field 8;
`next_stage()` and `set_clump()`; `select()` gives `Piece::LastAnm` and
`LastClump`. `story_map::build` runs `next_stage` on Quarantine when the
save byte is set. The fight's `Pic::ArenaClump` goes through
`BossRun.arena_clump` to the arena in `FieldWorld`. Test
`corbenik_last_stage` checks the stage switch, lights, hits and pieces
from the Quarantine archive.

**Still unknown:**
- The clump and clip as drawn are not compared with the game's frames
  (no shot yet); the third form's world side, which reaches this in
  play, is not wired yet.
