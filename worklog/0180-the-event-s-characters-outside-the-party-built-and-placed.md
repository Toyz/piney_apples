---
number: 180
title: "The event's characters outside the party, built and placed in the fields"
date: 2026-09-26
area: world
files: crates/piney-battle/src/evparty.rs, crates/piney-battle/src/entry.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session/tests/side_events.rs, docs/engine/field-walk.md, docs/engine/battle.md
---

# 180. The event's characters outside the party, built and placed in the fields

`field-walk.md` listed as unknown that the registered characters outside
the party are not built, "the game builds them at the origin". The side
event survey (0178) showed what that costs: side events 58, 60 and 61
name a player character who is not in the party (`entry 2 code`: Sanjuro,
Gardenia, Natsume), and the port never built them, so block 3's
`near_marker` could never hold beside them.

**What the game does.**

- `ccRegisterEventMng` (set-up pass): each entry of type 0-2 goes to the
  registry as `EntrySpc`, with the entry's `bootParam`.
- `ccSPC::Reboot` builds every registered character; one outside the
  party is built at the origin (`ccGetStartPositions` gives it nothing
  else) and has no HUD panel.
- `ccThEntryCtrl`'s set-up, `ccEntryEventMng`, places each such entry at
  its event position by marker. When the position's floor or block is
  9999 or more, Kite's position is added to it. It faces the position's
  `dirc`, and an entry `param` of 5 puts it on `ccEntryCmnd`, the list
  `pc_command` also fills.

**The port.**

- The area mode passes the VM's type 0-2 entries to
  `FieldWorld::set_spc_entries` in the set-up, logged as `entry ..`.
- `FieldWorld`'s reboot builds the registered non-members at the origin
  (`combat.add_member`). After the party is set, each entry is placed by
  `EvParty::place_entry(code, marker, param, kite)`, which reuses
  piney-battle's `entry::event_pos` (now `pub(crate)`) and `entry_cmnd`.
- The survey now prints `spc N built` for each such code. 58, 60 and 61
  build theirs.
- `event_61_natsume_in_her_dungeon`: story 18, blocks 0-1 marked run, the
  session put in area 35's dungeon and moved to event point 1's room.
  Natsume is built there. With Kite put beside her, `near_marker 0 <= 65`
  holds and her scene opens (the fade and her first three lines).

**Stale notes cleared.** The area host's header still said the walks,
`pc_command`, the puts, the entries, `hold` and the battle's instructions
were not reached. All of them are, through `EvParty`, the VM's entries
and `FieldWorld`. The header now lists them, and `field-walk.md`'s
matching unknown is gone.

`battle.md`'s unknowns were narrowed:

- `plcol` is the bracelet (`data_drain` sets it with Kite's skill 2);
  only the protect break's reading stays inferred.
- Skeith's `_ccBossSkillDamage` and `BossSkillTbl` are ported. Only
  `ccBoss02`-`04`, fought in the later volumes, remain.

**Still unknown:** Event 61 was driven only to its opening lines; the
fight and the reward after them need a player. Events 58 and 60 were not
given tests of their own. `inviteSpc`'s build at the Chaos Gate is still
not ported (no script makes one).
