---
number: 488
title: Innis drained under the survey's pilot reaches its Epitaph: the first drain at a break, the item drop with it
date: 2026-10-25
area: battle
files: crates/piney-game/src/session/tests/survey.rs, UNKNOWNS.md
resolves: 286.1
started: 2026-10-25T18:47:48-06:00
took: 15m
---

# 488. Innis drained under the survey's pilot reaches its Epitaph: the first drain at a break, the item drop with it

Play question [[268]] (286.1): in `mutation_story_survey`'s run of event
107, the drains at Innis's breaks gave item drops and no Epitaph, where
`innis_drained_through_the_menus` gets the Epitaph. [[274]] suspected
the queued-affect bug it fixed (the drain's 21 lost).

**Re-run.** Event 107 under the survey's pilot (`PINEY_SURVEY_ONLY=107
PINEY_SURVEY_GOD=1`, 400,000 frames). It ends `story 107: done` again.
The pilot's per-frame line (`fighters`) now also prints a boss's act and
its `epitaph` flag. That is a diagnostic line in survey.rs; the pilot is
unchanged.
- Innis's protect broke 80,237 frames in, at 28,393/30,000 HP, and the
  pilot drained once (menu 66 at 80,300).
- By 80,700 Innis was in its Epitaph: act 12, `epitaph` 1, HP 3000/3000.
  The fight went on in the Epitaph's acts (12, 13) to Innis's end.
- The other drains of the run (frames 22,000-40,200 and 193,500 on) were
  common foes' and Gr=go&!'s, not Innis's.

**Why the item drop is right.** The item window (menu 29) after the
drain is the game's own. `DataDrainMenu` step 0 (gcmn
0x00533010-0x00533470; `piney_battle::drain`) gives a boss its row's
`item[0]` into `drainItem`, and the item menu hands it out. So the drop
and the Epitaph come together.

**What it was.** The run of [[268]] predates [[274]]. There the target
menu's `EntryAffect(13)` waited in `Boss::queued` while the world was
paused, and the drain's 21 was lost, so Innis never entered the Epitaph.

**Still unknown:** nothing
