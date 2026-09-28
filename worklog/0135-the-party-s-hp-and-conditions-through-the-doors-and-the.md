---
number: 135
title: The party's HP and conditions through the doors and the event scenes: storeCondition
date: 2026-09-26
area: battle
files: crates/piney-world/src/party.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/battle.md
---

# 135. The party's HP and conditions through the doors and the event scenes: storeCondition

In the port, each dungeon door, stairway and field change built the party
fresh at full HP. It also cleared their conditions and their timed buffs
and debuffs. A Kite killed in one room walked into the next one whole.
I found this while watching event 29's walk: Kite went down in room
(0, 6), wandered at 0 HP, and was back at 63 in the next room.

## The game

`ccChar::SetBaseParam(ccSpcParam *)` (gcmn 0x0056b690) is what gives the
full HP. It sets HP and SP to the maximum and clears `condition`, `tune`,
`temp` and `time`. What the game keeps across the rebuild is
`storeCondition[18]` (gcmn 0x0072fb00, `ccStoreCondition` from DWARF, 0x70
bytes, by `charTbl` row). It holds the 16 condition shorts, `speedValue`,
the `ccSpcParam`'s `temp` and `time`, HP, SP, the maximums and
`conditionNum`.

- **Storing.** `ccStoreSpcCondition` (0x0056cc40) fills the table from
  every built character. The next scene's `ccSetupGameCtrl` calls it
  first, as do `ccEvent::MenuBan` and the fountain's menu.
- **Restoring.** `ccRestoreSpcCondition` (0x0056cfc0) copies it back,
  right after `SetBaseParam` in `ccPlayer::ccPlayer` and in
  `ccFellow::Initialize`, for a character whose `partyFlag` is 1. It
  restores nothing in these cases:
  - in a town, so a town does heal the party;
  - after a log-in (`areaPrev` -1);
  - in field 13 while `eventStatus[39]` is 1.
- **Ghosts.** After the restore, both constructors turn a character with
  no HP into a ghost: SP 0, `dead` 4, `ghostFlag`, cloak 0.5. That is the
  state act 10 leaves a dead Kite in, so a ghost Kite walking at 0 HP is
  the game, not a bug.

## The port

- **The table.** `party::StoredCondition` is one row, and
  `Spcs::store[18]` travels with the registry.
- **Storing.** `FieldWorld::store_conditions` and
  `World::store_conditions` store the built party when the session leaves
  a scene (`AreaMode::leave`, the town's `change_scene`).
- **Restoring.** `Combat::restore` is set before `rebootSpcManager`'s
  constructors from `party::restores_condition`. `Combat::add_kite` and
  `add_member` then restore each character and stand it as a ghost if it
  has no HP.
- **The check.** `the_party_keeps_its_hp_through_a_door` takes Kite
  through area 26's first door twice:
  - at 17 HP and 5 SP, he is at 17 HP in the next room (SP has
    regenerated a point);
  - with no HP and `dead` 4, he comes back a ghost.

## Around the event scenes

`ccEvent::MenuBan` (main 0x001b2460) calls `ccStoreSpcCondition` and
then clears every registered character's conditions, buffs and debuffs
(`ClearCondition(ccSpcParam *)`). `MenuClr` (0x001b25c0) calls
`ccRestoreSpcCondition` and then `ccSpcChar::ConditionAdjustment` on
each, which turns a down character into a ghost and stands a revived
one up. So an event scene leaves the party exactly as it began, HP
included, even over a heal the scene gave. `FieldWorld::menu_ban_party`
now does both, through `Combat::menu_ban_conditions` and
`menu_clear_conditions`, which carries out `ConditionAdjustment`'s hit
events as the frame does. `menu_ban_keeps_the_partys_condition` poisons
Kite at 20 HP and bans. The poison is gone during the scene and HP stays
at 20. It then heals him to 55 and clears, and he is back at 20 and
poisoned.

**Still unknown:** the fountain's `FountainMenu3` pair, and
`ccClearSpcCondition` (0x0056d300), which the bosses' deaths call, are
not ported.
