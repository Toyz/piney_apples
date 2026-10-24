---
number: 461
title: Corbenik without aid: the pilot breaks or runs from the seeds, and the start's party still falls in the first form, whose four-blow magic kills the member it aims at at any level
date: 2026-10-24
area: test, battle
files: crates/piney-game/src/session/tests/corbenik.rs crates/piney-game/src/session/tests/survey.rs GAPS.md UNKNOWNS.md plans/quarantine-story.md
started: 2026-10-24T13:13:36-06:00
took: 18m
---

# 461. Corbenik without aid: the pilot breaks or runs from the seeds, and the start's party still falls in the first form, whose four-blow magic kills the member it aims at at any level

## Event 313's three forms with no aid

`corbenik_fight_to_its_end` starts in field 8's arena with Kite and the
two strongest members who answer calls at 313: BlackRose (15) and
Balmung (6). The party is as the start leaves it, levels 70, 70 and 75.
There is no god and no experience, and the pilot fights with its own
tactics. The fight is not won.

**What kills the party, from the code and the tables.**
- *The magic* (act 20, at about 600 frames into the fight, then once
  each pattern round). Skill 252 is PhaJuk Zot: wind, atk 130, hit 150,
  four blows 20 frames apart, splash at half within 300. Corbenik's mAtk
  is 990. `tools/battle.py damage` gives 693-1,005 a blow on Balmung at
  75, and 498-843 at 99. Four blows on the member it aims at are
  2,800-3,800. That is more than any member's HP at any level: 1,827 for
  Kite at 99, 2,030 for the other two.
  - The party is locked through it (`lock_player`), so no one can heal
    between the blows.
  - The port's blows match the formula exactly: the first is the
    maximum (1,005 at 75, 843 at 99).
  - At the start's levels the half blows on the other two also add up
    past their HP. In the first run all three fell in the first cast.
- *The seeds* (act 21 throws them, 150 frames to their burst). Each
  burst is boss skill 31: atk 600, hit 990, physical, 800 around. It sets
  off every other seed within the same 800, so they come in chains of
  hits two frames apart, 200-500 each.
  - A seed has 250 HP, pDef 2000 and mDef 9990, so no spell touches it.
  - Kite's blow does 68-81 to a seed at level 70, and Balmung's
    112-136.
- *The wave* (act 6) hits for about 580.

**The pilot's new tactics** (`StoryPilot`, survey.rs):
- `focus` takes the counting seed nearest Kite as the part to strike,
  after Kyvia's gomora and Gorre's brother.
- `flee_seeds` runs Kite from the nearest seed with under 40 frames
  left, while it is within 950.

**What it comes to.**
- **The start's party, no aid.** It falls at 9,938 frames. The first
  form is at 7,373 of 10,000 HP. 3 seeds were broken and 64 burst.
- **Diagnostic, `PINEY_CORBENIK_EXP`.**
  - 10,000 experience each: the party falls at 8,102.
  - 20,000 each: the party falls at 10,966.
  - 30,000 each (all three at 99): the party falls at 7,862. Before the
    seed tactics it fell at 6,139.
  - With the seed tactics the start's party no longer falls in the
    first cast. The draws changed, so the magic's first target took the
    blows alone.

A win needs more than levels. Every cast kills one member, so the
pilot has to revive at once (Resurrect, a member's Rip Maen). Kite
should fight from beyond the splash of whoever is aimed at. The party
should take Ap Juka (wind +100) and Ap Vorma (mDef +50) before a cast.
The members' blows should go to the seeds as they land. And it needs
experience gained by play, the step [[444]] named.

`PINEY_CORBENIK_TRACE` prints the party's HP as it changes, with the
boss's act, each live seed's act, HP and distance, and boss skill 31's
row.

**Still unknown:** whether the pilot beats Corbenik with no aid, and at
what level and tactics. Not reached: the start's party falls in the first
form at 9,938 frames, and so does one raised to 99.
