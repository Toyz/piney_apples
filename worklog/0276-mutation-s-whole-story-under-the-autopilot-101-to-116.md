---
number: 276
title: Mutation's whole story under the autopilot, 101 to 116
date: 2026-09-28
area: test, volumes
files: crates/piney-game/src/session/tests/survey.rs, plans/mutation-story.md
---

# 276. Mutation's whole story under the autopilot, 101 to 116

With Magus ported ([[274]]) and field 13's talks working ([[275]]),
`mutation_whole_story` makes one run from Mutation's new game, with no
start built for any event. It ends every story event and stops at the
desktop's staff roll. The run is 1,400,000 frames long, is traced with
`PINEY_DEBUG_PILOT`, and took 612 s:

| event | ended at frame | where |
| ---: | ---: | --- |
| 101 | 79,200 | top page |
| 102 | 80,700 | Dun Loireag |
| 103 | 84,000 | top page |
| 104 | 121,200 | field 58 |
| 105 | 159,900 | Carmina Gadelica |
| 106 | 161,700 | top page |
| 107 | 300,000 | desktop set-up (after Innis) |
| 108 | 370,500 | Carmina Gadelica (after Kyvia) |
| 109 | 384,600 | top page |
| 110 | 387,000 | Carmina Gadelica |
| 111 | 405,900 | Carmina Gadelica |
| 112 | 470,100 | top page |
| 113 | 579,900 | Carmina Gadelica |
| 114 | 618,300 | Carmina Gadelica |
| 115 | 700,200 | desktop set-up, stream 45 (after Magus) |
| 116 | 703,500 | desktop, the staff roll |

The run is under god mode, and it uses the harness aids the earlier
entries list:
- Kite's infection is held at 0 ([[268]]).
- The gate hack's Virus Cores are given ([[270]]).
- The party is held at level 60 while Kyvia's parts are up ([[273]]).

Everything else is the pilot's input through the game's menus and the
port's own rules. The pilot's goals come from the scripts' conditions,
and it takes no shortcuts through the events.

**Still unknown:**
- How the run compares with a player's play in time: the pilot's fights
  are long (Innis about 140,000 frames, the area 47 King about 170,000).
- Whether a run without the level aid can beat Kyvia 01 with a pilot that
  grinds, and at what level.
- The staff roll itself and what the desktop does after it (the save for
  Outbreak's carry-over) were not followed here.
