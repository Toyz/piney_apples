---
number: 468
title: The no-aid whole runs stop in their first dungeons; the pilot now fights casters, strikes the weakest foe, prefers blows to weak casts, sizes its heals and buys its stock, and Quarantine's run ends 301-303
date: 2026-10-24
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs crates/piney-game/src/session/tests/shrine.rs crates/piney-game/src/session.rs plans/quarantine-story.md
started: 2026-10-24T15:53:48-06:00
took: 56m
---

# 468. The no-aid whole runs stop in their first dungeons; the pilot now fights casters, strikes the weakest foe, prefers blows to weak casts, sizes its heals and buys its stock, and Quarantine's run ends 301-303

## The no-aid whole runs, and the pilot's ordinary fights

With no aid (`PINEY_SURVEY_AIDS=0`), every whole run stopped in its
first dungeon:
- Quarantine's in event 302's field 100, to three level-69 Liches, at
  frame 28,506;
- Outbreak's in 203's field 71, Kite alone at 50 against four foes of
  51 and 52, at 20,370;
- Mutation's in 104's field 44, to three Phoenix Queens of level 30.

Mutation's and Outbreak's runs had only ever finished with the
survey's aids: god, or the console's levels and potions.

**What the traces showed** (`PINEY_DEBUG_PILOT` on 302's and 203's own
starts, each foe now printed with its distance, whom its skill aims at,
and the walk's choice):
- **The casters were left alone.** The dungeon walk fought only foes
  within 600 of Kite. The Liches cast from 1,000 and more, and Kite
  walked on to his goal room under their spells.
- **Kite cast where he should strike.** A spell takes its menus and a
  cast, about three plain blows' time, and he cast whenever his SP
  allowed. Against 302's foes his blows did as much in that time.
- **The pilot ordered where Kite should fight.** In ordinary fights the
  members' spells were ordered first, and each order holds the pilot for
  180 frames.
- **The heals were too small.** Kite chose Repth (150 HP) first, against
  blows of 400-650.
- **The blows were spread.** The walk went at the nearest foe and the
  members at their own, so three Liches stood at three different HPs.
- **Kite stood frozen.** Getting up from a revive (`dead` 5, 78 frames)
  he could still begin an action. Its menu then put the world to sleep,
  him with it, for the action's 600 frames.
- **No heal could be bought.** The standing stock never came. Quarantine's
  towns sell no Healing Potion, only the Recovery Drink. And the shop
  lookup (`stock_row`) took the Elf (0x200) and the Recorder (0x1000)
  for the item shop. The merchants' types are 0x100 weapons, 0x400 items
  and 0x800 magic (field-ui.md), so the walk went to the Elf, whose
  menu (26) has no Buy, and stayed there.

**Menu 92.** `CheckMenuType` answers the last list's number (88, 92 from
Mutation on: `MenuCtrl::changing`) while `menu` and `menuNext` differ.
That is a window fading out to another one or to none. It is no menu of
its own. The pilot now presses nothing while it lasts, and still gives
an action up after its 600 frames.

**The pilot's changes** (`StoryPilot`, `Walker`):
- **Casters.** A foe whose skill aims at one of the party
  (`target_char`) counts as near, for the walk and for Kite's skills.
- **Focus fire** (`prey`). Out of a boss fight, the party strikes the foe
  with the least HP, of those within 1,500 or aiming at the party. The
  walk goes at it, and the target menus are walked to it.
- **Blows before casts.** A skill at a foe the blows reach must hit for
  three blows or more (`calc_battle_damage` at 100). Out of a boss fight
  Kite's own skill comes before a member's ordered spell.
- **Mends before orders.** Revives, cures and heals come before the
  fight's opening order.
- **Heals sized to the wound** (`pick_heal`). The cheapest Repth that
  mends what is missing, else the strongest.
- **No action while Kite gets up.**
- **Stock from play** (`STOCK`). In town Kite buys up to 6 Resurrects, 10
  Healing Potions or Recovery Drinks and 6 Mage's Souls, as his gold
  allows.
- **The goblins' charms come first.** While a field's golden goblins
  want Speed Charms, the stock waits for them. GOB3-2's second trip to
  the shop needs the gold, and its test failed until it did.
- **A shop out of reach is given up.** After 3,000 frames the town's stock
  is given up until the next town. Mutation's Dun Loireag's item shop
  lies out of the town walk's reach.
- **Diagnostics.** `PINEY_SURVEY_EXP` gives a survey start experience
  once, to measure a fight's level gap. The whole runs' lines carry the
  gold, levels and stock (`purse`).

**Where it stands.**
- **Quarantine, event 302 from its own start, no aid.** It was lost at
  15,500 frames. It is now done.
- **Measured, before the changes.** 302 with 10,000 experience each
  (80, 85, 75) still fell. With 20,000 each (90, 95, 85) it was done.
- **Quarantine's whole run, no aid.** It ends 301, 302 and 303 (302 at
  frame 46,800).
  - It then stalls in Fort Ouph at the gate hack (menu 62) for area 101.
  - The hack wants Virus Cores, which the aided runs took from the
    console (`cores_for_hack`). The pilot drains no common foes for them.
- **Mutation's whole run.** It now stops at a gate hack too, after
  101-103.
- **Outbreak's 203.** Kite alone at 50 still falls in field 71's dungeon:
  - a level-52 Earth Maiden casts at him from 1,900;
  - two Miasma Wyverns and a Maxwell strike him;
  - his Ol Repth heals do not keep up.

  With 20,000 experience (Kite 70) the run goes much further, into the
  dungeon's later floors, and still falls.

What the no-aid runs need next is play the aids stood in for:
- Virus Cores won by draining common foes, with the infection their
  drains raise brought down by kills (`ccExpDistributor`), not by the
  console;
- levels won by fighting more of the portals on the way, and a lone
  Kite's fights;
- then the bosses' own tactics.

**Checked.** The piney-game suite, clippy and fmt pass.

**Still unknown:** nothing. Where each no-aid run stops is above, and
the next pieces are the cores, the levels and the bosses.
