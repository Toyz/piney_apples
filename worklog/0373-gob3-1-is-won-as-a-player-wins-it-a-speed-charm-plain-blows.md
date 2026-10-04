---
number: 373
title: GOB3-1 is won as a player wins it: a Speed Charm, plain blows, the healer first
date: 2026-10-04
area: battle
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs, plans/outbreak-side-events.md, worklog/0372-enemies-revive-their-fallen-an-enemy-s-affect-flag-is-its.md
---

# 373. GOB3-1 is won as a player wins it: a Speed Charm, plain blows, the healer first

Worklog 372 left `gob3_1_golden_goblin_is_run_down` passing on a test aid,
`gold_within_a_blow`, which held a golden goblin's HP at 250. That aid is
gone. The port's goblins were checked against the game's code first, and
then the pilot was taught to play them.

## What GOB3-1 asks

Event 250 (`SB0020 GOB3-1`, OUT `eventTbl` S300): block 5 sets Kite in
field 78 and enters `type=5 code=135 param=3`, three of row 135. Block 7's
info line is "Defeat the golden goblins running in the field". Block 8
waits on `no_active` and gives item 6/62. Block 4 makes Kite go alone.
Nothing is to be caught, trapped or spoken to: all three must fall.

Row 135 (OUT `enemyTbl`, "Stehoney T"), read from the port's tables:
- level 40, 1,170 HP, 405 SP, `goldVolume` 3;
- pDef 700, pEva 121, mDef 1,070, mEva 990;
- spirit tolerance 900, body 1,000;
- `atkRangeA` 1300, `B` 2000, `C` 4000 (`area`, `territory`, `viewRange`
  50000 by `checkGold`), `maxSpd` 22;
- two plain attacks of reach 300, and `mag0` 153, La Repth: 20 SP, 150 HP
  to every goblin within 600 of its target.

## The port is the game here

- **The SP.** `ccSkill::HealingSystem` pays the cost of any stype-0
  creator: INF gcmn 0x00576ea0, and OUT 0x00598ec0 is the same code. A
  goblin pays 20 a cast and gets back `maxSP / 50` (8) every 60 frames.
  It is whole again in 150 frames, which is why SP read 385-405 all
  along.
- **The flight and break-free.** OUT's `thinkGold` (0x00458a50),
  `moveGold` (0x004594a0) and `checkCrisisRate` (0x00444010) have
  Infection's instructions, reordered. `actEscapeGold` (0x00459ab0) lacks
  only Infection's dead branches after the dropped `ccRandF(pi/10)`.
  Infection's code is what `test_battle_spawn_rs.py` compares on gold
  goblins (worklog 372). Its harness cannot run on Outbreak:
  `ccThEntryCtrl`'s loop address (INF 0x00431b08) has no OUT symbol.
- **The rest, on OUT's code** (`PINEY_VOLUME=outbreak
  tools/test_battle_enemy_ai_rs.py bulk 100`), 0 mismatches each: think,
  interrupt, routine, check_enemy, select_attack, skill_list, skill_target,
  start_affect.

No port bug.

## Why a lone Kite could not finish

- **One blow a catch.** An art holds its target, and a held goblin that a
  blow reaches breaks free (`thinkGold`). The normal attack holds nothing.
  But `interruptThink` makes a goblin flinch at the first blow unless it is
  in its attack act. On a goblin of `goldVolume` 3, act 7 sets `goldDisHold`
  on its first frame (`moveGold`). That takes it off the command lists for
  a frame and sends it running. One hit lands: 292 from Kite at 90 (302 for
  Thunder Dance's first).
- **It outruns him.** Fleeing, it runs at `maxSpd * (1 + 1.6 c^2)`, with
  `c = crisisRate = 1 - distance / 1300` (`goldParam[2]` 160). That is 39
  at 400 and up to 57 beside him. Kite runs about 26 a frame, 46 with
  haste. In a fight he must be within about 150 plus both widths to take it
  as his target (`ccCheckTargetRange`).
- **The heal outpaces the hits.** One or two La Repths (150 each) came
  within 100 frames of each hit. Hits came every 500-2,000 frames.
- **Magic misses.** Juk Kruz at mEva 990 missed (-1). Paralysis and slow
  (body) never take; sleep, charm and curse (spirit) take one time in five.
  Kite carries none of them. A golden goblin has no protect to break
  (`max_pp` -1), so there is no Data Drain.

## How the pilot wins now

It does what the game lets a player do (`StoryPilot`, survey.rs):
- **A Speed Charm** (`itemTblD` 56, Ap Do, x1.75 for 9000 frames). Kite
  has two. `haste_for_gold` uses one whenever a goblin of `goldVolume` 2
  up lives and he is not hasted. The item action finds its own Items page
  now (`item_page`; it had always used page 0).
- **The plain attack** at such a goblin (`choose` gives no skill). The
  button goes every other frame, because the goblin is in reach for a
  frame or two.
- **Whom to go for** (`approach_foe`). First the goblin casting its heal:
  in its attack act it does not flinch and backs off at no more than 22.
  Then the one it last ran at, while that stands. Then the weakest one
  standing. While all run, Kite stands still. A goblin flees only while he
  is within `atkRangeB`, and from a stand it speeds up slowly
  (`goldAccel` is `c^2`), so it can be rushed then.
- **The stick at the rim** (`run_toward`). `stick_toward`'s circle of
  radius 127 leaves some leans under 231 after the per-axis dead zone
  (48). `ControlMove` walks him then (7 a frame hasted).

Each step was measured on the run:
- the plain blow alone: 6 hits in 14,000 frames;
- with the haste: 1 (Kite turned between whichever goblin was nearest);
- holding to one goblin: 0;
- waiting for a goblin to stand, then rushing it at the rim: 27 hits.

All three fell, at 7,210, 8,038 and 13,338. The event ended at 13,551.

## Checks

- `gob3_1_golden_goblin_is_run_down` passes with no aid but the survey's
  usual ones (god, Kite at 90). Its budget went from 14,000 to 18,000
  frames for margin.
- The piney-game suite passes (233 passed, 82 ignored, four threads).
  Only piney-game's tests changed. Clippy on piney-game is clean, and so
  is `cargo fmt --check`.
- `PINEY_SURVEY_HITS` now logs SP too.

**Still unknown:** whether the game on a PS2 plays out the same way. That
needs a capture or a player's pad log of GOB3-1 in field 78. This entry
reads the game's code; it was not seen on hardware. GOB3-2 to GOB3-5 (251-254)
were not run again under the new pilot. That needs the long Outbreak side
survey (`outbreak_side_event_survey`, `PINEY_SURVEY_ONLY=251,252,253,254`).
