---
number: 385
title: The whole stories after the draws moved: Infection, Mutation and Outbreak end, and GOB3-2's pilot goes back for Speed Charms when they run out
date: 2026-10-06
area: test
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs, plans/outbreak-side-events.md
---

# 385. The whole stories after the draws moved: Infection, Mutation and Outbreak end, and GOB3-2's pilot goes back for Speed Charms when they run out

Three commits changed which caller gets which number from the game's one
`rand()`:
- 05b691e (#48): a level 3 or 4 spell's damage draws where the game's
  `ccThEffect` does.
- a80585a (#52): the menus draw from the area's generator.
- 3726576 (worklog 384): a room's new menu draws its noise before
  `rebootSpcManager`.

The autopilot's runs ride on those numbers, so each was run again at
3726576, one at a time (release, the survey's aids).

## The whole stories

| run | end | before (worklog 375) |
| --- | --- | --- |
| `infection_whole_story` | event 31 at 399,900 | 408,900 (417,900 at 383) |
| `mutation_whole_story` | event 116 at 497,100 | 497,100 |
| `outbreak_whole_story` | event 219 at 414,000 | 423,600 |

None stalled. Mutation's and Outbreak's were run again after the pilot
change below, with the same ends.

## Outbreak's side events

`outbreak_side_event_survey` with `PINEY_SURVEY_GOD=1`: at its own 30,000
frames 19 finish. Worklog 375's count was at 150,000. At 150,000, 251
(GOB3-2) stayed open beside the known 254.

**The cause.** `PINEY_DEBUG_PILOT` on 251 alone:
- The pilot weighed the three goblins (row 136) once, on first sight:
  three Speed Charms. It went to Fort Ouph for them and came back.
- The goblins heal each other. By frame 30,000 the charms were gone
  with all three still standing; one fell near 36,000, one near 85,000.
- Unhasted, Kite could not catch the last one, which fled about the
  field: 61 HP from 90,000 to 105,000, then healed to 661 by 145,000.

The goblins flee and heal by the game's rules (worklogs 373, 374). Only
the pilot's arithmetic ran short. In worklog 374's run 251 ended at
99,879, near the edge already.

**The fix.** `StoryPilot::provision` weighs again when Kite is out of
charms and unhasted with golden goblins still up, at their full HP (the
repeatable `entry 5` puts them back). Short of that, he takes another trip
to the shop. 251 now ends at 48,471, after a second trip at 30,500. Under
`PINEY_DEBUG_PILOT` a `PROVISION` line reports each weighing.

**The survey again** (`PINEY_SURVEY_FRAMES=150000`): 25 of 26 finish.
Each event's frame is in plans/outbreak-side-events.md. 254 (GOB3-5) is
still open, for the game's own reasons (worklog 374).

## Checked

- `gob3_2_golden_goblins_fall_after_a_second_shop`: 251 ends within
  60,000 frames. With the old `provision` it fails.
- `gob3_1_golden_goblin_is_run_down` and
  `gob3_4_golden_goblins_fall_after_a_shop` pass.
- piney-game suite (253 passed, four threads), clippy, fmt.

**Still unknown:** nothing.
