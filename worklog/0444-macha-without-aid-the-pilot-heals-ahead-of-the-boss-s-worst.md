---
number: 444
title: Macha without aid: the pilot heals ahead of the boss's worst second, cures charm and confusion, refills SP, runs from the wave and casts at range, and the party the start leaves still falls before the drain
date: 2026-10-24
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/macha.rs, crates/piney-game/src/session.rs
took: 45m
---

# 444. Macha without aid: the pilot heals ahead of the boss's worst second, cures charm and confusion, refills SP, runs from the wave and casts at range, and the party the start leaves still falls before the drain

The brief asks for each boss to be won with no aid: no god, no
experience from the console, no moved Kite. [[443]]'s Macha ends event
307 only under god. Without it the party falls. This entry gives the
pilot the tactics a player uses against Macha and measures what is
still missing. The fight is not won yet.

## The fight

`machas_fight_to_its_end` starts in the arena with Kite and the two
strongest who answer calls at 307's start: BlackRose (15, level 70) and
Balmung (6, 75). Kite is level 70. The others who answer are 8 and 9,
both 65.

What Macha does to them, all from her code ([[443]]):

- **Magic** (act 24): a Pha spell, atk 120-130, three blows ten frames
  apart, about 1,200 to each member in its area.
- **Wave** (act 4): boss skill 22, reach 900. It hits for 300-550,
  drains SP to 0 (`DRAIN_SP` 100) and confuses.
- **Data Drain attack** (act 23): every condition at once (curse,
  sleep, confusion, charm, poison, paralysis, slow).
- **Charm** (act 5): skill 159 on each member.

Confused and charmed members strike the party for 400-600 a blow; one
such blow felled Kite with his last 469. The gauge is 25,000
(`maxPP`, physical and magic PP defence 5,000 each), and the party fills
it at about one a frame. Under god the break comes at about 14,000
frames.

## What the pilot learned

All of it is in `StoryPilot` (survey.rs), so every volume's runs get it.

1. **Mends before the chase.** A heal, cure, revive or soul is chosen
   before Kite walks after a roaming boss. Before this he ran after
   Macha's escapes with a member at 34%.
2. **Heals ahead of the worst second.** Against a boss, someone is low
   under 60% of his HP (45% elsewhere), or under 5/4 of the most a
   member has lost in one second of this fight (a Pha spell's three
   blows).
3. **Kite's own heal.** His cheapest heal he has the SP for, Repth
   first; Quarantine's Kite knows Pha Repth. Then a Healing Potion, then
   the cheapest item that casts a heal (the Recovery Drink).
4. **Cures.** Someone charmed, confused or asleep gets Kite's Rip Synk,
   else a Restorative.
5. **SP in a boss fight.** Someone under a sixth of his SP gets a Mage's
   Soul.
6. **Shorter waits.** After Kite's own attack the next action may come
   at once; after an item or his own heal, 90 frames; after a member's
   order, 180 as before. While waiting against a boss he still chases
   and strikes, since nothing else moves him in the arena.
7. **No dead actions.** While Kite is held, asleep, paralysed, confused
   or charmed, no action begins and one not yet open is dropped. A
   held Kite's menus never open, and the action used to block every
   other for 600 frames.
8. **Macha is a roamer.** He walks to her as to Fidchell and Gorre, out
   of battle mode too (her escapes take her past its 2,200). He keeps
   turning to her until she is his command target, since she floats
   out of his facing.
9. **Range.** Against a boss, Kite's skills reach as far as their
   `triggerRange`, not 600, and a skill in reach is cast before a walk.
   Against a boss whose two defences are equal (Macha: 6000 and 6000)
   he takes the strongest skill of either page, here MeRai Kruz or
   MeAni Kruz (atk 100, rate 100, reach 2000).
10. **The wave's tell.** While Macha's act is the wave and he is within
    1,000, Kite runs from her. Its blow comes 55 frames on and reaches
    900.

## What it comes to

- **The start's party, no aid.** It has fallen at every frame from
  1,796 to 33,130, the run changing with each tactic. The gauge never
  broke; the most was 19,762.
- **Diagnostic, 10,000 experience each** (`PINEY_MACHA_EXP`: 80, 80,
  85). The gauge breaks at about 26,000 frames. Kite's Data Drain lands
  and Macha becomes the Epitaph. She falls to 3,981 of 6,000, and the
  party falls at 32,649.
- **Diagnostic, 20,000 each** (90, 90, 95), on an earlier pilot: the
  party fell at 24,647, just short of the break.
- **From 307's own start** (`event_307_from_its_start`, no aid). An
  earlier pilot got to the arena with Kite 72, Balmung 75 and member 9
  (69), and fell at 41,585. The final one falls at 26,023 in area 107's
  dungeon, before the arena. Its two level-80 Lich Lords hold the party,
  which the replayed start leaves at its new game's levels.

Two things are short. The party's levels: the start replays no fights,
so no experience. And the pilot: one menu action a second, with
everyone clustered in Macha's areas. A pilot that grinds as a player
does (fights for its levels in the dungeon before the arena), and stands
apart from the members, is the next step. Stat aids are not.

`PINEY_TRACE_BOSS`, `PINEY_TRACE_HITS` and `PINEY_TRACE_KITE` print the
fight, each hit on the party with its conditions, and Kite and the pilot
each frame. `StoryPilot::doing` and `fighters` are shared with the
traces.

**Still unknown:** whether the start's party can beat Macha at all
without grinding; what level a player brings to 307; and whether the
final pilot's faster attacks cost the dungeon fights before the arena
(the from-start run now falls in 107, where an earlier one got through).
