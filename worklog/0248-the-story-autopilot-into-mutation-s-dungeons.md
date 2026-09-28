---
number: 248
title: The story autopilot into Mutation's dungeons
date: 2026-09-27
area: test
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-world/src/lib.rs
---

# 248. The story autopilot into Mutation's dungeons

Event 104 (M204) now plays to its end under the autopilot, from its
start in Dun Loireag. The path goes through the board, Carmina Gade, Elk,
the party calls, the gate hack, field 44, its dungeon, three floors of
fights, and event point 1. Before this entry the autopilot read the board,
logged in, and then warped to a stale marked area. It knew no way to find
what the story wanted next.

## What the story wants, from the scripts

`story_wants(vm, save)` reads the volume's main events (100 x (volume - 1)
to +49). It skips any event whose flag has bit 62 or 63 set, and any whose
`event_done` open conditions fail. For each block not yet played it walks
the precondition settings as the event walk does (`Vm::set_current`):
- **Scene settings** (`in_town`, `in_field`, `in_dungeon`, `scene`) and
  `game_status` other than 5 replace one another.
- **`status` / `status_range`** decide whether the block can open at all.
- **`block_done`** is an earlier block that must have played.

A block that passes gives its wants:
- its scene's town (`Town`);
- its area, from the field or dungeon (`Area`, `Dungeon`);
- its `in_point` (`Point`);
- its conditions' `gate_words` area and `in_party` members.

Blocks that want `not_in_party` or an answer are refusals and give
nothing. Event 104 shows why the settings matter: `in_town 2` and
`status 0 == 1` are block 2's settings, not its conditions. The first
reader looked only at the conditions and missed both.

## The town

`gate_player` picks one goal, in this order:
1. an event target NPC present here (`World::event_targets`, walk and talk);
2. with mail or a board post unread (board states 1 and 7 count), log out;
3. a wanted town not this one, by its row in the gate's Towns list;
4. a wanted member missing from the party who has an address
   (`partyMemberFlag`) and answers calls (`partyMemberCall`), while a slot
   is free. The call goes PERSONAL, Party, Add, the member's face, OK;
5. a wanted area, either its row in this server's Word List
   (`gateOrderList`) or the town on the server that lists it;
6. the marks, as before.

On the top page the autopilot now reads every new post, thread by thread,
before it logs in. A protected area opens the gate hack (menu 62). The
autopilot turns each slot's cores up to its `protect` row's count, moves to
the next slot, and presses OK.

## The field

A field whose dungeon the story wants: Kite walks straight at the
entrance (`dungeon_pos`) while he is more than 2500 away. Closer in, he
follows `entrance_path`. It is a breadth-first search over a 100-unit
grid 3000 either side of the entrance. The target is the ground whose
attribute has bit 0x80000; field 44's door cells are round the middle, at
the end of a walled way from the south. A step between cells is allowed
when:
- no wall crosses it at 30 or 95 above the ground, 100 either side of the
  line;
- the ground rises or falls less than 120;
- a diagonal has both of its sides open too.

Walls stop `Hits::line` only from their front. Each check therefore runs
both ways; the first planner went through the back of the entrance's west
wall. The path is planned again whenever a 60-frame check finds Kite has
not moved. The field's hit models are only loaded near the player, so
nothing can be planned from the far side of the map.

## The dungeon

`walk_to`'s body is now `Walker::step`, one frame at a time, and
`walk_to` loops over it; the shrine and dressing tests pass unchanged
(19). The story's walker (`Walker::wary`) makes two changes:
- it fights only enemies within 600 of Kite;
- it leaves magic portals shut unless it is stuck in the room.

Only event entries shut a room's doors (`ccEntryEventMng`'s
`CloseDoor`), so a portal can be left alone. The goal is the room of the
event point the story wants (`EventMng::point`, from `SetEventData`).

## The fights

At level 30, attacks alone lost. Area 44's Red Scissors (level 29, 1210
HP) has pDef 585 against Kite's pAtk 250 and regains HP. The party was
wiped two rooms in. `StoryPilot` now drives the menus, one action at a
time, in this order:
1. **The fight's start.** CHAT's Skill Usage goes to both members: Magic!
   when the nearest foe's pDef is above its mDef, else Skills!.
2. **Someone is down.** A member with Rip Maen (180) and the SP for it is
   told to use it (CHAT, Members, Designate Skill, the recovery page, the
   target). Failing that, Kite uses a Resurrect. Failing that, Kite gives
   a Mage's Soul to the member who knows Rip Maen but lacks the SP.
3. **Someone is low** (under 45% of his HP in a fight, 70% out of one).
   The first of these that applies: a member's heal (La Repth); Kite's
   Repth (Skills, the recovery page, the target); a Healing Potion; First
   Aid!, at most every ten seconds.
4. **Out of a fight,** anyone under a third of his SP gets a Mage's Soul.
5. **In a fight,** a member is told to cast his spell of the element the
   nearest foe resists least, when that is under 100 (Elk's Vak Rom
   against the crabs' fire 0). Otherwise Kite uses the first skill he has
   the SP for from the magic or attack page.

The target menus list the party in order. A revive lists the fallen
(condition not 0 and not 5), anything else the members in condition 0.
A row past the list backs out.

Several menus hold a message that only OK closes:
- the target menu with no target left (menu 65, proccess 2, and 73's
  proccess 10);
- the PERSONAL menu's "Cannot be used while dead." (menus 0 - 2,
  proccess 2 - 3).

Cancel did nothing there, and Kite's death with menu 2 open froze the
dungeon for good. When no action is under way, the pilot now backs out of
any of these menus. In a field or dungeon it is the only thing that opens
them.

`PINEY_DEBUG_PILOT` prints the actions, the dungeon goal, and every 50
frames the place, the pad, the party and the foes. The survey line now
says whether the event finished (`done`) or which blocks played.

## The survey (20000 frames a start)

- **102, 110** finish.
- **104** is in its dungeon at 20000 frames (blocks 0, 1, 3, 9). Given
  70000 frames it finishes: floor 2's room 3 and event point 1, then on
  into event 105's field 58.
- **105, 109, 112 - 115** go round the party call: menu 68 again and
  again.
- **107, 111** stop at the gate hack (menu 62).
- **103** waits in its town, **106** on the desktop, **108** on the top
  page.
- **116** plays its staff roll, as before.

**Still unknown:** Why the party calls of 105, 109, 112 and 113 loop at
menu 68 (a member refusing, or none to call). Why 107's and 111's gate
hacks do not pass (the cores, or the protect row). What holds 103 in its
town. The level a player carrying Infection's clear data would bring to
event 104, next to the start's 30.
