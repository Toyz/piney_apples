---
number: 268
title: Event 107 under the autopilot: to Innis and past the breeder
date: 2026-09-28
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session.rs
---

# 268. Event 107 under the autopilot: to Innis and past the breeder

With Innis ported ([[266]]), `mutation_story_survey` for 107 now ends
`story 107: done` (400,000 frames, the fight about 146,000 of them). Three
things stood in the way, each the pilot's, not the port's.

## Walking into Data Drain's reach

Innis roams the arena where Skeith came to Kite, and the target menu (65)
finds no one past Data Drain's reach (2000 plus the boss's width). The
pilot cast from where it stood, so each protect break went by. Now, in a
fight, with a boss's protect broken (`pp_count`, the break's frames; `pp`
is the gauge that fills to it) and Kite knowing Data Drain, the pilot
walks straight at the boss until within 1200 (`approach_boss`), then
drains. In the run the protect first broke about 80,000 frames into the
fight (Innis lv 99 against a lv 31 party: its HP from 30000 down at about
0.3 a frame), and the pilot drained at ten breaks before Innis fell.

## Data Drain's game over

The first run went from the arena to the title. The drain's side effect
roll (`rand() % 100 <= infection / 2`) picked effect 30, which
`DataDrainMenu` step 12 turns into `Request::GameOver`: the game's own
rule, at the infection the story's start gives. `PINEY_SURVEY_GOD` (and
`mutation_whole_story`) now hold Kite's infection at 0 through the
console, as god holds HP: a harness aid, not the game's.

## The breeder in Carmina Gadelica

After the fight, block 23 in town makes the Grunty breeder (NPC 16) an
event target (`add_target 13 16`) for a repeatable line (block 25). The
town pilot marked an NPC spoken to only when the talk's block banned the
menus; the breeder opens his own menu (27) instead, which nothing closed,
so the pilot stood in it. Now a talk counts once the NPC's own menu opens
(21-27, 44-46), and `gate_player` backs out of a shop's or breeder's menu
(CIRCLE). Then block 26 wants the top page (`game_status 3`), and the
pilot's log-out and Quit ([[265]]) end the event (block 29, `end_event`).

## The Innis session tests

`event_107_ends_with_innis` and `innis_drained_through_the_menus` (in
`session/tests/skeith.rs`) never ran when they were written: block 19's
`message 4` waits for a button, and both held the pad still, so the event
task sat in its phase-4 pass (op 15) for the whole run and block 20 never
came. They press CROSS while a block plays now, and pass. `Vm::playing_op`
(event, block, instruction) was added to find that.

## Also

`mutation_whole_story` now passes event 104: the Gate Out from a field the
story is done with ([[265]]) takes it back to town at frame 117,600, where
it used to stand in field 58.

**Still unknown:**
- In the survey the drains at Innis's breaks gave item drops (menu 29)
  and no Epitaph, where `innis_drained_through_the_menus` gets the Epitaph;
  why the two differ is not looked into (the boss still fell and the
  event went on).
- Whether a party at the level a player would bring breaks Innis's protect
  sooner; the pilot's party is the start's.
