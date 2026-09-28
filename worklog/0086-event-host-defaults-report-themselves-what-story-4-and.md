---
number: 86
title: Event host defaults report themselves: what story 4 and story 11 leave unported
date: 2026-09-24
area: script, test
files: crates/piney-event/src/host.rs, docs/engine/event-vm.md
---

# 86. Event host defaults report themselves: what story 4 and story 11 leave unported

The event VM was checked against the game ([[82]] and earlier). The user
then played `story:4` ("Orca gives me items and it didn't work right, no
trapped chest, nothing") and `story:11` (BlackRose never joins). Neither
fault was in the interpreter. Each was an instruction whose `Host` method
the mode's host never implemented.

## Why it was silent

- **The defaults.** `Host` has 79 methods. All but `save` have a default
  that does nothing or returns a neutral value. A host overrides only what
  its mode needs.
- **What that hid.** An instruction whose method a host had not ported ran
  its default without a trace. The scene carried on as if the step had
  happened.

## What changed

- **Defaults report.** Every default now calls `host::unported(name)`,
  which prints the method's name once to stderr: "event host: `stream` is
  not ported by this host (the default ran)". `host::take_unported()`
  returns the names since the last call, so a playthrough test can assert
  its host left nothing to the defaults.
- **The one exception.** `volume` answers 1, which is Infection's volume
  and so correct for the port. It stays quiet.

## What the story starts leave unported

Each start ran for 6,000 frames with X pressed every 25 frames. That
reaches only the branches a talk-through takes. In every mode,
`clear_gate_hack` (`ccClearGtHack`, from `ccStartThEvent`) and
`game_over` (the event task's check in status 5) ran.

| start | where it plays | also unported |
|---|---|---|
| `story:3` | field 14 | none |
| `story:4` | dungeon, then the desktop | `stream`, `remove_trap`, `remove_trap_done`, `gimmick` (dungeon); `load_overlay` (desktop) |
| `story:10`, `story:12` | desktop | `play_pass_done` |
| `story:11` | Mac Anu | `stream` |
| `story:13` | Mac Anu | `player_distance` |

- **`story:4`.** It reached the desktop at frame 4,623. Without
  `WORLD_MAN::SetEventData` every room's block fires in room 0 ([[82]]),
  so the event's camera marks were placed for rooms Kite was not in. That
  is the likely cause of the camera in the walls the user saw.
- **`story:11`.** BlackRose's stream 7 never plays in the town. The script
  gives her member address, then branches on `in_party 15`. The player's
  invite from Member Address is the join, and nothing past the missing
  stream leads to it.

## Who has what

- **Event 4 end to end.** The combat agent has SetEventData, the doors and
  area_host `stream` / `remove_trap` ready to merge. It still owes:
  - `add_spc_item`, `item_get_menu(_end)`, `remove_trap_done`, `room`,
    `prev_room`, `gimmick` and `boss`;
  - the treasure and trapped boxes;
  - the event cameras in the dungeon's rooms;
  - a `story:4` playthrough that asserts `take_unported()` is empty.
- **Event 11's town side.** A new agent has:
  - the town's `stream`, `clear_gate_hack` and `game_over`;
  - the Member Address invite and BlackRose's warp-in;
  - the party through the Chaos Gate and area_host's `party_add` /
    `party_remove`;
  - a `story:11` playthrough.

## Checked

The piney-event tests and the story-start tests pass: `event_2_plays`,
`event_3_plays` and the story starts. This entry's change does not alter
behaviour, only reports it.

**Still unknown:**
- **Not assigned yet.** The desktop's `play_pass_done` and `load_overlay`.
- **Not reached by the talk-through.** Branches it did not take may call
  other defaults: menus, fights lost, other answers.
- **A limit of the counts.** Once reported, a name stays silent until
  `take_unported()` drains it, so the counts say which defaults ran, not
  how often.
