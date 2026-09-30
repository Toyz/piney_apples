---
number: 305
title: Outbreak's 206 in the survey's budget and its whole story from the new game
date: 2026-09-29
area: script, test, volumes
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session.rs, crates/piney-game/src/toppage.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/geom.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, tools/test_battle.py, docs/engine/battle.md, plans/outbreak-story.md, GAPS.md
---

# 305. Outbreak's 206 in the survey's budget and its whole story from the new game

After [[293]] event 206 was the one Outbreak event left, and the one run
from the new game (`outbreak_whole_story`) had not been made. Both now
finish: 206 from its start in 82,000 frames, the whole run to 219's staff
roll in 463,200.

## Why 206 was slow

- **Field 13 first.** Block 2 puts enemy 72, a Data Bug (`type` 0x40,
  20,650 HP), in field 13's story map. Since [[299]] its HP stops at half
  and only Data Drain ends it. It roamed out of Kite's reach while the
  members filled its gauge, and the pilot only walked to a broken boss in a
  fight, so the run never left field 13.
- **The dungeon of field 72 is the game's.** Its rooms are shut
  (`SetDoor`) until no foe is active. Each holds Maxwell, Forest Hsien and
  two Napylons (OUT `enemyTbl` row 289: 2,130 HP; Ola Repth, skill
  154: 400 to each foe within 600 of one under half its HP, 40 of 1,045 SP
  a cast). A drained Napylon becomes an Astro Prince (row 288, 1,000 HP).
- **The heal rules are exact.** `tools/test_battle_enemy_ai_rs.py` had not
  run on Outbreak: OUT has no `cmndTarget` name, so `GameBattle.sym` now
  falls back to Infection's name carried (`volume.va`). With it, 300 cases
  each of `select_attack`, `skill_list`, `skill_target`, `start_affect`,
  `routine`, `init_skills`, and `test_battle_rs.py`'s `recovery` and
  `area_recovery`: 0 mismatches. The game makes this fight this hard for
  a lone Kite at the new game's level 50.
- **The pilot wasted it.** It took each page's first skill (Gale of
  Swords, `atk` 60 at `dmgRate` 25) over stronger ones (Thunder Dance 80,
  Juk Kruz 100). It tried Gale beyond its 350 reach, and 11% of the frames
  went to the target menu's "no target". The walker went at the
  first-listed foe, not the nearest, and walked into walls when chasing.
  Goal-room foes past 600 were left standing, with Kite idle in 203's and
  206's goal rooms.
- **The goal room kills a lone Kite.** After the Data Bug's drain its
  drained form, Comad Goo (row 61, Drill Thrust twice), took Kite's 963 HP
  in one blow; god's per-frame heal cannot catch that, and the game went
  to the title.

## The changes

- The pilot walks to a field's Data Bug, fight or not; when found stopped
  it uses `path_to` over the field's or story map's hits.
- Kite's skill is the strongest (`atk` by `dmgRate`, over the foe's
  resistance to its element) whose reach holds the nearest foe. It comes
  from the page the foe is weaker to, else the other. "No target" gives
  the action up.
- The walker goes at the nearest foe. A chase found stopped goes round by
  a room waypoint. The goal room's foes are fought at any distance.
- `LONE_LEVEL` (75): Kite alone in a dungeon where the story wants him
  alone (`Want::Alone`: 204, 206) is raised through the console's `exp`, a
  harness aid like the boss levels. At 75 the lone rooms go in two thirds
  of the frames and he lives through the goal room.
- The whole run: the top page quits to the desktop when the story wants
  only it (`Want::Leave { desktop }`: 208's news). A wanted member, the
  party full (210's Piros after 209's Balmung and Wiseman), has the party
  disbanded first.
- **ccGetDist on Outbreak.** The only enemy-harness mismatches on Outbreak
  were distances one unit in the last place long. OUT main 0x001e6e70
  takes `sqrt.s` (truncating) inline where INF and MUT call `sqrtf` (QUA
  0x001ee540 too). `enemy_ai::get_dist_on(volume, ..)` now serves
  `checkEnemy`, `ccSearchNearPerson`, `checkSkillRange`, `selectTarget`
  and the escape hold. On Outbreak every enemy AI and motion check now
  matches in 300 cases except `main`, 2 of 300. INF and MUT stay at 0.

## The whole run

It starts from Outbreak's own new game: the title's New Game, then
`ccSetupNewGame` at the first Log in. Kite is at level 50 with its gear,
and the members are at `InitSpcParam`'s levels. Mutation's carry-over
(`ConvGame`) is not ported. 201's `call_lock` keeps Kite alone to 204.
Events end at 9,900 (201), 14,100, 77,700, 81,600, 84,300, 156,600 (206),
232,200, 233,700, 275,700, 298,800 (210), 320,400, 323,400, 338,400,
365,100, 387,900, 390,000, 391,500, 461,100 (218) and 463,200 (219).

Tests: `outbreak_story_survey` at 150,000 frames, all 19 done.
`outbreak_whole_story` passes. `mutation_whole_story` passes (116 at
613,800, from 751,800). The piney-game suite passes (165), and
piney-battle's with the new `outbreak_distance_truncates`. Clippy and fmt
are clean.

**Still unknown:**
- The 2 Outbreak `main` motion cases left differ in a hit's number (8 to
  9) and in the order a weapon's hits land; the damage paths' own `sqrtf`s
  (damage, flow, drain, follow) were not checked for OUT's `sqrt.s`.
- Whether a Kite carried over from Mutation (`ConvGame`) would get through
  206 without `LONE_LEVEL`; the run was only made from Outbreak's new game.
- Whether the goal room's any-distance fights stall where a foe cannot be
  reached; none did in either story.
