---
number: 469
title: Infection with no aid falls in event 14's dungeon at level 2; the pilot begins to grind: the portals on the way opened while the party is below its foes, and out and back for more
date: 2026-10-24
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs crates/piney-game/src/session/tests/shrine.rs crates/piney-game/src/session/tests/town_return.rs
started: 2026-10-24T16:59:19-06:00
took: 54m
---

# 469. Infection with no aid falls in event 14's dungeon at level 2; the pilot begins to grind: the portals on the way opened while the party is below its foes, and out and back for more

## Infection with no aid

The chain the maintainer asked for (each volume finished by the pilot with
no aids, each clear carried into the next by `ConvGame`) starts with
Infection. [[360]] did not end Infection without aids: with them all off
the game was over at frame 40,564 in area 17's dungeon (event 14), with
Kite and BlackRose still level 1. Only the `levels` setting, which levels
the party by the console, took it to event 28.

With the pilot of [[468]] and no aid, the run ends events 1-4 and 10-13.
The game is then over at frame 42,827 in the same dungeon. Kite and
BlackRose, both level 2, fall to event 14's Data Bug: level 5, with its
HP held up to 20,065 until a drain. The pilot fought only what stood in
its way, so nothing levelled the party.

## Grinding, first part

A player fights for his levels on the way. The pilot now does too
(`StoryPilot::grinding`):
- **When.** It grinds while the weakest of the party is below the
  strongest common foe met in this field or its dungeon by three levels
  (`GRIND_OVER`), and everyone stands with 70% of his HP or more.
- **Not while led.** It does not grind while a guest 20 levels above the
  rest leads (Orca at 50 through Infection's tutorial), nor while the
  events wait on someone there. In the tutorial the detours left a
  lesson waiting for ever.
- **In a dungeon.** The walk goes first to a room of this floor (else of
  another) whose magic portal still stands (`portal_room`), and opens
  the portals on its way (`Walker::grind`).
- **In a field.** Kite walks to the nearest portal and opens it
  (`grind_field`).
- **None left.** It goes out and back in, by a member's ocarina from a
  dungeon and by the gate from a field, for the portals anew. That is
  only from an area whose portals it has seen. An exit that has not come
  in 1,500 frames drops the area's grinding (`grind_exit`, `no_grind`).
- **Stock.** It now begins with Health Drinks (100 GP, 150 HP) while
  Kite's HP is under 400: a level-1 party's gold went on Resurrects.

**A menu-change wait the later volumes lacked.** Two places waited for a
menu changing by `CheckMenuType`'s 88, Infection's answer:
- the pilot's gate-out;
- `town_return`'s gate-out.

From Mutation on the answer is 92, so they pressed through the change.
Both now wait while `menu` and `menuNext` differ.

**Where it stands.** Infection's run ends events 1-4 and 10-13, then the
game is over at frame 64,058 in event 14's dungeon. The party is still
level 2: Kite does open field 17's portals.
- Infection's kills give no gold, so after the first 1,000 the stock
  runs dry.
- A kill of a level-3 or level-4 foe gives a level-1 member 80-100 of the
  1,000 experience a level takes.
- The portals of one visit bring about two levels.
- The party seldom stands above 70% of its HP for long, so the grinding
  stops and the story's walk takes over. The way out and back for the
  portals anew has not yet come.

**Checked.** The piney-game suite, clippy and fmt pass.

**Still unknown:** nothing. The next parts are the party's mending
between fights (nothing comes back on its own, and the Health Drinks run
out), the round trips for the portals, and then the Virus Cores.
