---
number: 374
title: GOB3-2 to GOB3-4 are won after a trip to the magic shop; GOB3-5's goblins evade blows and shake off spells
date: 2026-10-04
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs, crates/piney-game/src/session.rs, crates/piney-game/src/start.rs, crates/piney-event/src/vm/mod.rs, plans/outbreak-side-events.md
---

# 374. GOB3-2 to GOB3-4 are won after a trip to the magic shop; GOB3-5's goblins evade blows and shake off spells

Worklog 373 left GOB3-2 to GOB3-5 (Outbreak 251-254) unrun under the new
pilot. The plan still marked 251-253 done from the old HP aid.

## The first run

`outbreak_side_event_survey`, `PINEY_SURVEY_GOD=1
PINEY_SURVEY_FRAMES=150000 PINEY_SURVEY_ONLY=251,252,253,254`, at 268db68:

| event | row | result |
| --- | --- | --- |
| 251 | 136, Jonue T | done at 99,879 |
| 252 | 142, Zyan T | done at 14,319 |
| 253 | 149, Albert T | open |
| 254 | 156, Martina T | open |

The four scripts are GOB3-1's (`tools/evscript.py dis`): `entry 5` of
three goblins of the row in fields 79-82, Kite alone, block 8's
`no_active`. The rows differ (the port's tables, OUT `enemyTbl`):

| row | HP | pDef | pEva | mDef | mEva | spirit | Exdefense |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 135 | 1,170 | 700 | 121 | 1,070 | 990 | 900 | 0 |
| 136 | 1,310 | 785 | 586 | 1,185 | 990 | 1,000 | 0 |
| 142 | 1,450 | 800 | 151 | 1,300 | 9,990 | 1,000 | 0 |
| 149 | 1,590 | 1,950 | 760 | 9,990 | 9,990 | 0 | 0 |
| 156 | 1,730 | 9,990 | 9,990 | 485 | 181 | 350 | 2 |

`tools/battle.py damage` on OUT's code, Kite at 90: the plain blow does
292, 260, 256, 105 and nothing (a miss but for the sure rolls).

## GOB3-4: too many blows for two Speed Charms

Row 149 takes 68-105 a blow. Spells and debuffs all miss (mEva 9,990).
Kite has two Speed Charms. Their 18,000 frames of haste landed 37 blows;
after that, one blow in 8,000 frames. 4,770 HP takes about 50. Given four
more charms (a console item, for the measure only), it ended at 23,247.

A player would buy them: every server's magic shop sells the Speed Charm
(100 GP) and the six Banes (`magicShopItemList`). Kite has 10,000 GP.

The pilot does that now:
- **Weighing** (`StoryPilot::provision`). In a field the event waits to
  clear, once Kite can act, the goblins whose blows land: their HP over
  a sure blow, at 15 blows a charm, and one more. 250 wants 2 (Kite has
  them), 251-253 want 3 to 5.
- **Out.** Gate Out refuses in a fight (`GateoutMenu`'s `inBattle`
  warning), so Kite runs from the nearest foe first (`out_of_battle`).
- **The shop** (`GateGoal::Shop`, `gate_player`). The merchant of the town
  whose stock has the item (`stock_row`: the list by its type's 0x100 or
  0x800 bit, on this server), Buy (52), the row, the count (`waitCount`)
  up to what Kite lacks and can pay for, OK, OK.
- **Back** by the Word List. The event's `entry 5` block is repeatable,
  so the goblins are there again.

Two harness fixes came with it:
- The survey placed Kite in the field with blocks 0-4 marked run. Their
  gate words (`gate_add_msg 81`, `gate_mark`, `town_move`) were never
  played, so the Word List had no way back. `place` now plays a skipped
  block's gate and town instructions (`Start::play_ops`, through
  `Vm::execute_now`, now public).
- The repeatable blocks stay unmarked, as `repeatable` keeps a block's
  bit clear. Marked, block 2 (`in_party` -1: `add_area_code 81`) never
  barred the area, and the pilot took two members along.

The rush ran into a wall on one way back (Kite still for 30,000 frames at
28,167). A rush that moves under 60 in 60 frames now takes
`walk_after`'s path round the wall for 180 frames.

## GOB3-5: the game's rules, not the port

Row 156 cannot be beaten the way the others are:
- **Blows.** pEva 9,990: `CalcBattleDamage`'s roll misses below 95, and
  the sure rolls do about 20 against pDef 9,990.
- **Spells** do 0 while its mDef is not below 485 (Exdefense 2, worklog
  307). Beast's Bane (Dek Vorma, mDef -100 for 1,800 frames) lifts that,
  and its hit is 95% against mEva 181.
- **The hold.** A spell holds its target (`ccSkillHold`). A volume-3
  goblin held out of its attack act runs. Once 500 from where it was
  caught it shakes the hold off (`thinkGold`'s `goldEscPos`): off the
  lists for a frame. A targeted spell then ends (`ccSkill::Main`). An
  untargeted one (0x4000, Juk Zot) goes on at the last place, and the
  goblin is out of its 300 by then. Measured: caught at 1,146, free at
  1,176, the cast over at 1,190, no damage.
- **In its attack act** a goblin stands. A Juk Zot cast at one in act 6
  landed: 340, 49 frames after the hold.

The pilot was tried that way, with 30 Beast's Banes given: Bane and spell
only at a goblin in its attack act, waiting 1,400 off for one. Over
60,000 frames, 8 of 21 spells landed. The goblins' heals put back 400 to
800 at a time (710 to 1,510 in one). None fell. The code was not kept.

The port is the game's code here. On OUT (`PINEY_VOLUME=outbreak`,
`bulk 100`), 0 mismatches each:
- `tools/test_battle_rs.py`: damage, exdefense, hold, condition_success,
  affect and the rest;
- `tools/test_battle_flow_rs.py`: skill_main and note_affect.

Worklog 373 had checked thinkGold and moveGold.

## The whole table again

The placement change touches every case, so the whole survey ran again:
23 of 26 finish. 250 at 13,551 (as in worklog 373), 251-253 at 16,911,
20,199 and 31,095 with a shop trip each, 254 open. 260 (SANJYURO-4) and
261 (MOON-2) do not finish. They do not at 268db68 either, with the old
placement, in a worktree. They were done on 2026-09-30, so they broke
since; that is left to the next entry.

## Checks

- `gob3_4_golden_goblins_fall_after_a_shop` (new): 253 with a shop trip,
  ended within 40,000 frames. `gob3_1_golden_goblin_is_run_down` passes
  as before.
- piney-game suite: 234 passed, 82 ignored, four threads. Clippy on
  piney-game and piney-event clean; `cargo fmt --check` clean.

**Still unknown:** how a player beats GOB3-5's goblins (row 156) alone.
Their heals outpace Bane-then-untargeted-spell, the only damage Kite has
from shop items within reach. Needed: a player's pad log or a capture of
a win, or a pilot that crosses to server 4's shop for the summon scrolls
(Yarthkins and the like: atk 250, hit 990, splash 500), not tried. Also
what broke 260 and 261 since 2026-09-30 (worklog 375 looks).
