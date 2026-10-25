---
number: 476
title: The pilot buys and wears a better weapon, walks lone Kite past field 18's foes, grinds where kills pay and holds the members back for drains; Infection reaches area 18's dungeon
date: 2026-10-25
area: test, battle
files: crates/piney-game/src/session.rs crates/piney-game/src/session/tests/survey.rs crates/piney-game/src/session/tests/town_walk.rs
started: 2026-10-25T11:28:46-06:00
took: 3h 2m
---

# 476. The pilot buys and wears a better weapon, walks lone Kite past field 18's foes, grinds where kills pay and holds the members back for drains; Infection reaches area 18's dungeon

All runs below are `cargo test --release`, the whole story from a new
game with no aids (`PINEY_SURVEY_AIDS=0`).

## Holding the members back for a drain

A Virus Core comes from Data Drain on a foe whose protect is broken, and
a common foe's protect breaks only on a hit that does not kill it
([[475]]). At a level advantage the party killed every core foe first.
The CHAT menu's Strategy page has an order for this. Its fourth row is
`btInChatAct[11]` = 10: strategy 3, `skillMask` 4. Under strategy 3 a
member only aims (`Reconnoiter`) and heals; it never strikes. The order
also goes into `saveData.tactics` (+0x6773), so each later fight starts
under it.

So when a foe whose drain gives a short core is up with its protect
whole (`StoryPilot::holds_back`), the pilot orders Strategy row 3, and
Kite fights with plain blows only (`choose` returns none). With no such
foe up, it orders row 0 again (7, a new game's). Skills! is not ordered
while the hold is on: in Infection's table, command 0 resets a commanded
strategy 3 to 0.

The hold was tried in Quarantine's area 90 farm:
- the order goes out at frame 59,675;
- Data Drain is tried 25 times.

Two fixes came out of that run:
- The Napylons stood 300 off Kite, where the walk after foes stops.
  Nothing was in his reach, so the pilot waited for 400,000 frames. With
  the hold on and no command target, Kite now walks up to the nearest
  held foe.
- A hold that breaks no protect in 3,600 frames (`HOLD_TRIES`) is given
  up for that field. A farm detour there is marked barren.

## A better weapon

Infection's Kite starts with 1000 GP and a 20-attack blade (0:0).
Mac Anu's weapon shop sells 0:3, 40 attack, for 975. The pilot buys it
before anything else (`shop_weapon`). The pick is the most physical
attack over the one worn, of a level his job may wear (`job_weapon`),
at a price his gold covers. Out of a fight Kite then puts it on
(`Action::Equip`). That goes through Equipment (menu 63): page 0, down
into the bag's pieces, the piece's row, OK, and back out.
`spare` keeps that weapon out of the sales. The old blade is sold.

## Town walks

After the weapon shop, Kite stood still for 300,000 frames. Three causes:
- The wide planner found a way, but he stopped against something that
  the hits do not hold.
- The town's walker (`TownWalker`) closes the cells round each person
  standing. BlackRose, just invited, stood at his side, so his own cells
  were closed and no way was found.
- Walking straight at the gate then held him at a wall.

The pilot's town walk now uses `TownWalker`, with these changes:
- people within 250 of Kite are not planned round;
- a stop closes the place 250 ahead of him;
- with no way round everyone, it plans round only the places he
  stopped at, and then round nothing;
- the town walker runs (`run_toward`).

## Grinding

- Grinding is weighed by Kite's level, not the lowest member's. A third
  member who joined at 3 had held Infection's area 30 for some 300,000
  frames.
- A lone stretch's detour must pay: an area within two levels of Kite,
  either way. Infection's area 18 rates 9, so Kite alone wanted 13, and
  every other area open then rates 6 at most. One level from 9 to 13 at
  ten under is 1 exp a kill (`expCalcTbl`), so earlier runs ground there
  until they stalled. With no such area, the story goes on.
- A new grinding detour: when Kite is under the story area's rating plus
  `GRIND_OVER`, go to an area rated above it, within two of him.
  Infection's area 15 (6) was picked over 17 (4). Its field has no
  portals, so it was judged barren and the pilot went back to 17.
- A field is judged to have no portals only after 600 frames in it. At
  frame 27 they had not come in yet.
- Grinding waits while an event plays. Event 14's lines at field 17's
  entrance had held Kite there for 100,000 frames.
- Party falls: grinding in an area ends at two falls, or at one fall
  with no Resurrect carried. Before, it ended at any fall; with three
  falls, Mutation's field 44 wiped the party.
- A walk that is stopped twice in a row sidesteps for 45 frames.
- A detour the gate does not take him to within 6,000 frames in town is
  given up. Mutation's area 58 gets event 155's lines each time it is
  asked for.

Tried and reverted:
- Capping the portals walked to at 12,000. Level 3 came at 106,200
  against 90,000 without the cap.
- Running in the fields. The stick's circle lean is a walk on the
  diagonals (`pow_l` under 230 past the dead zones), and running
  covered field 17 at some 27 a frame against 7. But levels came far
  slower: level 4 at 381,300 against 135,000. Why is not known.

## Lone Kite

In a stretch the story wants him alone for (`Want::Alone`, no member
in the party), Kite passes the foes by (`lone_here`):
- no walk after them;
- no blows or attack skills;
- heals still allowed.
Before, three level-7 foes killed him at level 9 in field 18. Now he
reaches area 18's dungeon.

Every unaided fight now tracks the worst second of damage, as boss
fights did. A heal comes once HP is within 1.25 times that.

## Where each run of the chain stops

| Volume (run) | Events ended | Where it stops |
|---|---|---|
| Infection (42) | 1-4 and 10-16 | Event 16 at 706,200. Game over at 723,389 in area 18's dungeon: Kite alone at 9, against a Shield Man (9) and a Bee Army (7), with no potions and 25 GP |
| Mutation (7) | 101-104 | Event 104 at 264,600. A farm detour to area 58 for Virus Core 1, which the gate never takes him to (event 155's lines), is given up. Then the run stalls at 664,800 in Carmina Gadelica's gate hack menu (62) |
| Outbreak (11) | 201, 202 | Game over at 24,209 in dungeon 71: Kite alone at 50, Earth Maiden and Maxwell |
| Quarantine (23) | 301-303 | Area 90 farm with the hold, then stalled in Fort Ouph at menu 62 (the hack) |

Infection's gold stays at 25 from the weapon on. No sale ran in run 41.
Golden goblins are the volume's gold source.

The other steps of the clock:
- piney-game suite: 331 passed;
- clippy and fmt clean.

**Still unknown:**
- Why Kite running in the fields gave levels so much slower.
- Whether the hold yields cores. Quarantine's 25 Data Drains were not
  counted against the cores held.
- Why the gate hack's menu (62) holds Kite in Mutation and Quarantine.
  It may be the cores still short with no farm left, or something else.
- Why Infection's gold never rises after the weapon.
