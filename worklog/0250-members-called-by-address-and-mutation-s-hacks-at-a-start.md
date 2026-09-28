---
number: 250
title: Members called by address, and Mutation's hacks at a start
date: 2026-09-27
area: world
files: crates/piney-world/src/party.rs, crates/piney-game/src/start.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/field-game.md, docs/engine/field-walk.md
---

# 250. Members called by address, and Mutation's hacks at a start

The Mutation survey after 248 looped at the party call (menu 68) in five
starts and stopped at the gate hack in two. Both led back to the port.

## Calling a member by address

In a town, PERSONAL, Party, Add called the member and showed his greeting,
but he never joined. `ccThPartyAdd` asks `ccParty::AddMember`, and the
port's `party_add` only took a character an event had registered. A
member called by address is not registered: the game's `inviteSpc` (gcmn
0x005a08e0) makes one, and that branch was not ported. The field-walk
page had it as a known gap: "no event script makes one". The party menu
does, though, and every player calls members that way.

What `inviteSpc` does for an unregistered id, from the disassembly:
1. `EntrySpc`, which gives nothing when the registry's five slots are
   full.
2. The character's file is queued (`spcTempFileList` type 9,
   `ccLoadFLAddOne`, then `GetCCSAdrs` into the registry's +0x20).
3. The character table's equipment is copied into the registry (+0x10 -
   +0x16), then `ChangeWeapon`.
4. The character stands at `StartPos[slot]`. `ccGetChgatePosDirc` copies
   the Chaos Gate's +0x40 position and +0x60 heading. The angle is the
   heading plus `inviteOffsetTbl[slot]` (gcmn 0x00654130: 0, 0x2000,
   0xe000, 0x4000, 0xa000 in 16-bit turns). It goes through `DEG2RAD`
   and then `fptosi` before `sinf` and `cosf`, so only whole radians
   reach them. x gains 150 sin, y loses 150 cos, and he faces half a turn
   from the gate.
5. The registry's `partyFlag` is set to 1, `ccSpcStart[id](slot, 0)`
   builds the character, and a new `ccAI` takes `ChangeMode(1)`.

`World::party_add` now does this through `invite_new` before
`AddMember`: `entry_spc`, `build_spc`, and the gate placement with the
same truncation. The equipment copy is not ported, because the port builds
the character from the save. A field or dungeon still cannot build one.
The docs say both.

## The hacks at a start

A story start replays the events before it, but not the fights. Infection's
starts are given the cores for its three listed hacks (`HACKS`: events 18,
25, 30). Mutation's areas 44 and 45 need cores that no replay gives, for
example two of core 1 for area 45 at event 105. `fights` now tops up the
cores for every protected area that the start's own event goes to
(`goes`: the fields of its `in_field`, `in_dungeon` and field `scene`
settings, and its `gate_words` areas), unless the area is already hacked
(`protectArea`). Infection's list still applies as before.

## The autopilot

- **Members.** An `in_party` condition is a want only when the same event
  also refuses without that member (`not_in_party` names him). Event 105's
  blocks 5 - 13 are lines for whichever member came along. Taking them as
  wants had the autopilot calling nine members in turn.
- **The party before a warp.** Going out to an area, the free slots are
  filled first (`companion`) with members who have an address and answer
  calls. One who knows Rip Maen comes first, then any healer.
- **Field 45's entrance** holds the door under a roof: the ramp runs down
  west from the middle to the doorway's floor at z -465. `land` finds only
  the highest floor, so no door cell was found. The planner now keeps
  every floor that a line straight down meets (the height map's ground
  where no model floor is). It walks floor to floor, no more than 120 up
  or down. A door is any floor with bit 0x80000.
- **A room whose doors stay shut.** Once the walker has put Kite in the
  same room and he is still stuck there, it fights every foe in the room,
  not only those within 600. Its magic portals may then be opened too. An
  event's entries hold the doors (`CloseDoor`) until they fall.
- **`PINEY_SURVEY_GOD`** runs a survey with the console's god: the party is
  kept at full HP and SP in the fields and dungeons. Without it, 105's
  floor 2 (two Great Sled Dogs at 1330 HP, a Phalanx and a Hysteria, level
  31 - 32) beats the level 26 - 30 party the start brings. The foes' HP
  that climbs mid-fight is the game's own: the enemies cast Regene, and
  `setSkillRate` weights it up 1.5 times.

## The survey (60000 frames a start, `PINEY_SURVEY_GOD`)

- **Finish:** 102, 104, 105, 109, 110, 114. 105 runs from the board
  through Carmina Gade, the calls, area 45's hack and its three floors to
  the desktop.
- **In their dungeons at the end:** 107 (field 46) and 112 (field 49)
  circle CHAT's member menu (71); 111 (48), 113 (50) and 115 (52) are
  still walking.
- **Waiting:** 103 in its town, 106 on the desktop, 108 on the top page.
- **116** plays its staff roll.

**Still unknown:** Why CHAT's member menu (71) loops in 107 and 112. The level and equipment a player would bring to
Mutation's dungeons, against the start's. Whether `inviteSpc` in a field
or dungeon is ever reached.
