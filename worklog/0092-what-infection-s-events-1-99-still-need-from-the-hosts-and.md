---
number: 92
title: What Infection's events 1-99 still need from the hosts, and the town's trans, frame rate and overlay
date: 2026-09-25
area: script, test
files: crates/piney-world/src/event.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/world.rs
---

# 92. What Infection's events 1-99 still need from the hosts, and the town's trans, frame rate and overlay

The story starts only reach events 3-14 ([[82]]). To see what the rest
of Infection needs, the host methods its scripts can reach were counted
statically.

## How

- **The scripts.** `tools/evscript.py dump` gives every script. Events
  1-99 are Infection's own: groups M1 and S1, the main story and the side
  events. They use 124 distinct instructions.
- **Instructions to host methods.** Each instruction's `Host` calls come
  from `piney-event`'s `vm/exec.rs`.
- **What is still missing.** A method counts when it still runs the
  trait's default in a host.

This over-counts. An instruction counts for every host, even when its
block only ever runs in one place (a dungeon's `room` in the town's list,
say).

## Where Helba is

- **In Infection** (events 1-99), she only appears by name: event 24's
  Apeiron warns of her by name.
- **In the later volumes.** She is placed (`entry` type 2 code 17) and
  talks only in events 101 and up, the M2 to M4 scripts, which are
  Mutation and later.

## What remains, by host

- **The town** (`field_host`):
  - `add_spc_item`, and `item_get_menu` / `item_get_menu_end` (the
    item-get window). These are in 12 or more events; the combat agent is
    writing the field's, and the town will follow them.
  - `piros_colour` (event 22).
  - `room`, which is only in dungeons.
- **The field and dungeon** (`area_host`, the combat agent's): the same
  item methods, and:
  - `npc` (8 events);
  - `party_remove` (5 events, event 11 among them);
  - `room`, `trans` and `piros_colour`.
- **Not a gap.** `generate_area` runs only when a story start replays
  `virus_core`; while playing, the instruction does nothing.

## Done here, for the town

- **`trans_off` / `trans_on`** (cases 161 and 162, main 0x001b2248 and
  0x001b22f4): the character's `transDist` (+0x90), the near fade. The
  type's bit picks the list: 2 a party member, 3-4 an NPC, 5-6 an enemy
  (none in a town). They are `World::set_trans`, used on NPCs by events
  14, 27 and 29.
- **`frame_rate`.** The scripts' `SetFrameRate` is kept by the town host,
  and the town mode runs at it (event 31).
- **`overlay`.** Nothing to load, as on the desktop ([[91]]).

## Checked

The workspace's tests, clippy, fmt and the docs check pass.

**Still unknown:**
- **`piros_colour` is not ported.** Inside `ccEvent::Execute` (from main
  0x001b0c1c) it is a 14-way switch on `eventStatus[1]`. Each case is
  `ccScFade::EntryFlash`, a wait by `ccBreathThread`, SE 74, and
  `DispInfo` with a line of a `ccKanjiStrSeparate` list.
- **`trans` is not run in a playthrough yet.** Events 14, 27 and 29 are
  beyond the story starts.
- **No story starts past event 14.** Nothing yet plays the story beyond
  it, so these counts are the only view of it.
