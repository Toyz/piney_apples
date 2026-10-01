---
number: 314
title: "Outbreak's battle harnesses at 0: a menu word, swapped names, every foe type held, and sqrt.s beyond the battle"
date: 2026-09-30
area: battle, volumes
files: tools/volume.py, tools/test_battle.py, tools/test_battle_flow_rs.py, tools/test_battle_items_rs.py, tools/test_ride_rs.py, crates/piney-battle/src/damage.rs, crates/piney-battle/src/skill.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-data/src/libm.rs, crates/piney-gen/src/manifest.rs, crates/piney-fieldui/src/menus/target.rs, crates/piney-world/src/lib.rs, crates/piney-effect/src/effect.rs, crates/piney-audio/src/se3d.rs, docs/engine/battle.md
---

# 314. Outbreak's battle harnesses at 0: a menu word, swapped names, every foe type held, and sqrt.s beyond the battle

[[308]] left these Outbreak (OUT) harness mismatches:
- drain: `drain` 176, `side_effect` 1;
- flow: `area_item` 35, `skill_main` 4;
- enemy motion: `main` 2 of 300;
- the ride harness would not start;
- items: the map items of category 13.

Every check below is now at 0 mismatches on Infection, Mutation and Outbreak, 100 cases each:
- drain: `drain`, `side_effect`, `evolution`;
- flow: `skill_main`, `note_affect`, `skill_check`, `attack_note`, `condition_adjustment`, `level_absent`, `area_item`;
- items: every check;
- enemy motion: `main`.

The agent doing this work stopped at the last step (an account limit). The results above were re-run afterwards, before the commit.

## The causes

- **A menu word.** `ccMenuCtrl` grows a word after `dummyTarget` from
  Outbreak on. `itemNum` (the Data Drain skill) is at +0x240 (OUT gcmn
  0x0054ca44; MUT +0x23c), with `trapNum` after it. The drain harness now
  reads it there; which of the 176 cases it accounts for was not counted
  separately.
- **Swapped names.** The carry pairs Infection's note tables for important
  items 288 and 289 the wrong way round on Outbreak, which rewrote both
  notes. Two places now find each table by the branch `ccUseItemRequest` takes for the item (OUT gcmn
  0x0059e93c for 288, 0x0059e964 for 289):
  - the build: `piney-gen`'s `item_pages`, a manifest change, data version 21;
  - the items harness: `note_table`.

  Outbreak's gcmn rows also misplace two of main's globals,
  `showMapInfo` and `trapDischargeStr` (0x00386168, 0x00386170). The harness
  now looks main's globals up by main's carry (`tools/volume.py`); the
  category-13 map items are among what this fixed.
- **Every foe type held.** From Outbreak on, an area aimed at a foe takes in
  every foe type, not only the damage ([[307]]). It applies to `ccSkillHold` (OUT 0x00597200,
  point 0x00597478) and `ccSkillModifyCondition` (0x00597684, point
  0x005979dc) too, with an enemy's own area attack taking the attacker in.
  `damage::side_types` is now the one rule for all of them. The test is
  `an_area_hold_takes_in_every_foe_type_from_outbreak_on`. docs/engine/battle.md
  records `skill_main` and the motion harness's `main` at 0 on OUT with it.
- **sqrt.s beyond the battle.** Every Infection function that calls `sqrtf`
  (70, main and gcmn) has the truncating `sqrt.s` inline on Outbreak, and
  most of the drawing code's `(float)sqrt((double)x)` became `sqrt.s` too.
  Examples: `STATICOBJECT::Draw` (OUT 0x005f7fb8), `CLOUD`, `FIREFLY`, `FOBJECT`,
  `waterUVModifi2`, `ccMerchan::main` and `ccChgate::main`. `BIRD::Move`, `TOBJ::Move`,
  `STATICMODEL::Draw` and `CheckFrontObstacleF` keep the double.
  `piney_data::libm::sqrtf_on` and `dsqrt_on` match on the volume. piney-world,
  piney-effect, piney-audio and piney-fieldui (the target menu's `flat_dist`)
  take their roots through them. Infection and Mutation compute exactly
  what they did.

## The ride harness

`AwakeDistantLight` has no carried name on Outbreak: it is a leaf too short to place.
`volume.found` now finds it as the call `DrawPG` makes, checked by its
opcodes. So `tools/test_ride_rs.py` starts on Outbreak. It passes on Infection.

On Mutation and Outbreak it fails 6 checks, with 2 errors. The riding Grunty
changed from Mutation on:
- `ccPucciguso` is 0x80 bytes longer;
- `ControlMove` grows from 1288 bytes (INF) to 2260 (MUT) and 2516 (OUT), with new
  `sinf`/`cosf`/`fabs` physics.

The port's ride is still Infection's (docs/engine/battle.md).

## Checks

The piney-battle, piney-world, piney-effect, piney-audio, piney-fieldui,
piney-gen, piney-data and piney-game suites pass (piney-game 189). `gen` was
re-run: `sjis.rs` gains six kanji from the newly read note tables.

**Still unknown:**
- The later volumes' riding Grunty (`ccPucciguso`'s new members and `ControlMove`'s
  physics) is not ported; the ride harness fails on MUT and OUT.
- `outbreak_whole_story` and `mutation_whole_story` were not re-run after
  the hold rule and the roots changed.
