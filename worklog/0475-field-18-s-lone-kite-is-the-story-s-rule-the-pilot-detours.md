---
number: 475
title: Field 18's lone Kite is the story's rule; the pilot detours to grind before it, reads the gate's area levels, casts charms, sells its spare items, and farms Virus Cores where foes give them
date: 2026-10-25
area: test, battle
files: crates/piney-game/src/session.rs crates/piney-game/src/session/tests/survey.rs crates/piney-game/src/session/tests/shrine.rs
started: 2026-10-25T10:28:33-06:00
took: 53m
---

# 475. Field 18's lone Kite is the story's rule; the pilot detours to grind before it, reads the gate's area levels, casts charms, sells its spare items, and farms Virus Cores where foes give them

## Field 18's lone Kite

[[471]]'s Infection run ended in field 18, with Kite alone at level 7
against three level-7 foes. That is the story's rule. The event's block
that unbars area 18 asks for an empty party (`NotInParty { pc: -1 }`
beside its `DelAreaCode`), which the pilot reads as `Want::Alone`.
Members cannot be called in a field, so a player grinds before the
stretch, with the members, and then goes alone.

## Detours

The gate shows each area's level: `ccAnalyzeEnemyList` of its rank, the
highest of the three enemies registered from it (`Words::analyze_enemies`,
`WorldMan::enemy_rank`). The pilot reads it too (`StoryPilot::need`). It
takes the field and the first three floors of the dungeon, cached by
(server, area). Infection's story areas read:
- 14: 1-4;
- 15: 6;
- 17: 3-4;
- 18: 7-9;
- 19: 10-11;
- 21: 14;
- 22: 4-6;
- 23: 7-8;
- 24 and 25: 21-23;
- 26: 12-13;
- 16 and 27: 30.

These match the foes met.

`plan_detour`, in town, sends the party out of the story's way:
- **A lone stretch.** It goes when the story wants Kite alone for an
  area he is under by `LONE_OVER` (4) levels.
- **A gate hack.** It goes when the next area is behind a hack and its
  Virus Cores are short.
- **Where to.** The pick is an area of the gate's list (not the next
  one, not behind a hack of its own, not barren). For a lone stretch it
  is the strongest within two of Kite's level; for a hack, one whose
  foes give a short core.
- **How.** The town walk takes the detour (`Errands::detour`) after its
  shop: free slots filled, no disband.
- **There.** The pilot grinds while the detour holds
  (`detour_holds`), counting the level shown before its foes are met.
  A detour field with no portals sends it into its dungeon, else the
  area is marked barren and another is picked. A detour is re-picked
  once Kite is more than two levels over it.

Infection picks area 15, finds it barren (an event's field), then
grinds in area 29 and on.

## The pilot's fights and the town

- **Charms.** Out of a boss fight, when Kite's own skills give nothing,
  he casts his strongest spell charm (`itemTbl` category 11, no SP)
  whose sure hit is three blows' or more and in reach. The target menu
  is walked to the prey (`Action::Item { target: -1 }`).
- **Sales** (`spare`, `GateGoal::Sell`, `SellMenu` 51). In town Kite
  sells:
  - the equipment he carries;
  - the odds of category 13 (all but the Fairy's Orb);
  - the gifts of 14;
  - all but three of each charm (Speed Charms kept).

  It is sold to the nearest vendor (`VenderMenu`'s 0x100, 0x400 or
  0x800), down to what he keeps. The Sell page with nothing left over
  is backed out of. The first sale, Mac Anu's: five Fortune Wires for
  25 GP.
- **Shops out of reach.** A sale out of reach is given up as a buy is
  (`SHOP_TRIES`).
- **Field portals out of reach.** A field portal the walk does not
  reach in 6,000 frames is passed by (`PORTAL_TRIES`): area 30's stood
  behind a cliff.
- **Casters.** A field foe chased for 1,800 frames without losing HP
  is let be (`field_hopeless`): Quarantine's Dark Maidens keep their
  distance.
- **A grinding that costs a member** ends there (`no_grind`, the detour
  marked barren): QUA field 100's grinding brought groups of Dark
  Maidens on a party at their level.

## Virus Cores

- **The ask.** The hack the gate will ask (`GateMenu`: an area of this
  server not yet in `protectArea` whose protect asks any core) gives the
  cores the next area is short of (`cores_short`).
- **Which foes.** A foe whose drain list (`enemyTbl.item`, category 15)
  holds a short core counts as drainable once its protect breaks, while
  Kite's infection is under 60%. The drain aims at it.
- **Where.** The farm's areas are those whose registered enemies give
  one (`area_cores`).

Quarantine picks area 90 (level 54) for 101's cores (F, H, J short).
Its foes, 16 levels under the party, fall before their protect breaks,
so no core came in 395,000 frames there.

## Where the chain stands

**Infection.** The earlier runs of this piece ended events 1-4 and
10-16, then went to the detour:
- the first (before the shown-level fix) stood in a barren detour;
- the later ones found the detour's portals but levelled slowly: Kite
  went from 9 to 10 in 140,000 frames at area 30.

The latest run, with the down rule, is slower again: event 14 ends at
449,700 (288,600 before). The stall rule was raised for these runs
(`PINEY_SURVEY_STALL` 400,000).

Quarantine's ends 301-303, then farms cores in area 90 without a drain.

**Checked.** The piney-game suite, clippy and fmt pass.

**Still unknown:** nothing. Next are the drains at level (blows that
break a protect without the kill), Infection's rerun past field 18, and
the carry into Mutation.
