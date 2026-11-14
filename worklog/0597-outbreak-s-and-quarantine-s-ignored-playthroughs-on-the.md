---
number: 597
title: Outbreak's and Quarantine's ignored playthroughs on the unsigned food draw: no new failure; GOB3-1's extra Speed Charm is the game's own
date: 2026-11-14
area: test, volumes, battle
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs
started: 2026-11-14T15:05:33-07:00
took: 1h 45m
---

# 597. Outbreak's and Quarantine's ignored playthroughs on the unsigned food draw: no new failure; GOB3-1's extra Speed Charm is the game's own

Worklog 596 made `WORLD::SetFood`'s node draw unsigned on Outbreak and
Quarantine (`divu`, OUT gcmn 0x005d397c, QUA 0x004c63dc), which moves
every `ccRand` after a field is entered there. Every ignored piney-game
test that plays either disc was run on the branch (c2ede0f), uncapped,
one at a time; the ones that failed were run again on main c4f70b5 in a
worktree of its own. None fails because of the change, and no bot, test
or game code changed here.

## The runs

Pass on the branch (45 of the 52 runs):
`selector_icon_shots`; `replay_through_the_launcher`, `issue_73_replay`,
`player_log_replay` (issue 73's pad log), `issue_74_replay`,
`issue_75_replay` (their own logs); `event_313_save_bytes`,
`event_218_save_bytes`, `event_314_save_bytes`; Gorre's four shot tests;
`corbenik_third_under_god`, `corbenik_trace`, `corbenik_under_god`;
`staff_roll_shots`, `save_menu_shots`, `fidchell_shots`; Kyvia's
`kyvia_fourth_shots`, `kyvia_s_fourth_fight_to_its_end`, the three fx
shots, the three stage shots, `kyvia_s_third_fight_to_its_end`,
`kyvia_third_shots`; `macha_shots`; `event_310_goodbyes_shots`,
`tarvos_under_god`; `story_map_shots`, `dungeon_shots`, `event_3_shots`,
`volume_sound_survey`; `town_shots` and `pc_menu_shot` with
`PINEY_DISC` outbreak and quarantine; `search_bt_shot`;
`outbreak_side_event_survey`, `outbreak_story_survey`,
`quarantine_story_survey`, `outbreak_whole_story`.

Fail on the branch, and on main the same way:

| test | branch | main |
| --- | --- | --- |
| corbenik_fight_to_its_end | party fell at 6,006 | the same, every number |
| kyvia_fourth_under_the_pilot | 312 not past its fight at 399,937 | the same |
| machas_fight_to_its_end | party fell at 1,796 | the same |
| tarvos_fight_to_its_end | party fell at 37,103 | the same |
| event_307_from_its_start | party fell at 31,687 | party fell at 36,777 |
| event_310_from_its_start | party fell at 6,323 | party fell at 46,016 |
| quarantine_whole_story | stalled after 311 at 700,200 (field 12) | stalled after 311 at 672,600 (field 12) |

The four arena fights start in their arena and match main to the frame.
The two "from its start" runs and Quarantine's whole story cross fields,
so their frames move, but the outcome is main's: the no-aid party falls,
and the whole story stops at event 312's fourth Kyvia fight, which
`kyvia_fourth_under_the_pilot`'s own note gives as not yet won (the
gomoras' heals outpace the blows).

The surveys report the same states on both: every story event's
`done` or blocks in `outbreak_story_survey` and `quarantine_story_survey`,
all fifteen side events open after 30,000 frames without aids in
`outbreak_side_event_survey`. `outbreak_whole_story` ends event 219 on
both, at 321,600 on the branch and 313,200 on main; the events from 203
on end a few thousand frames later, as the fields' draws move.

## GOB3-1's charm

The pilot's extra charm (worklog 596) is what a player may do, with the
fight's numbers the game's own, read from Outbreak's executable:

- The goblins are `enemyTbl` row 135 (OUT gcmn 0x0061e2e0 + 0x1c0 x 135):
  `maxHP` (+0x24) 1,170, as the run shows them (`foes (1170, 1170)`), the
  same row as Infection's.
- The Speed Charm is `itemTblD[56]` (OUT 0x006501d0): skill 177, Ap Do,
  price 100 GP, the same on Infection and Quarantine.
- Ap Do's haste is x1.75 for 9,000 frames: `_ccSkillModifyCondition` (OUT
  0x00597bb0) stores 1.75 (`lui 0x3fe0`, 0x00597fb4) and takes the length
  from a table at OUT 0x007220d8 (0x2328), where Infection's has
  `li 9000` (0x005760c0).
- Every magic shop sells it: the first item of OUT's
  `magicShopItemList_01` and `_03` is 0xb0038 (category 11, item 56), as
  on Infection.

So a run that spends its charms with a goblin standing is short of
charms, not of anything the game would refuse: the goblins heal (La
Repth, skill 153 on both) and the fight is decided by `rand()`. Buying
three on the second trip is an ordinary purchase; the event's repeatable
`entry 5` block puts the goblins back at full HP, and the three fall by
frame 40,527.

**Still unknown:** nothing new; the party falling in Corbenik's, Macha's
and Tarvos's no-aid fights and event 312's fourth Kyvia fight under the
pilot are main's as well.
