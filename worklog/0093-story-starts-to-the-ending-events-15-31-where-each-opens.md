---
number: 93
title: Story starts to the ending: events 15-31, where each opens, and what they run into
date: 2026-09-25
area: script, test
files: crates/piney-game/src/start.rs, crates/piney-game/src/main.rs, docs/engine/event-vm.md
---

# 93. Story starts to the ending: events 15-31, where each opens, and what they run into

[[82]]'s story starts stopped at event 14. Infection's main story goes on
as one chain to its ending. `--mode story:N` now reaches all of it.

## The chain

Each event from 14 to 31 has one open condition on the story, `event_done`
of the event before. 31 is `ENDING`, which also wants `game_status 2`,
the desktop. So the story is events 1-4 and 10-31, and a start at event N
replays every earlier one with `ccEventFlagSet`, as before.

## Where each opens

Each event's first block that names a place gives it:

| place | events |
|---|---|
| the desktop (`game_status 2`) | 16 17 24 25 29 31 |
| the board (`game_status 3`) | 14 20 22 23 26 30 |
| Mac Anu (`in_town 0`) | 15 18 19 21 27 28 |

- **The town starts.** They enter Mac Anu from Log in, as 11 and 13 do.
- **The party.** Logging out empties it, so these starts need none.
- **The existing places are enough.** Every event from 15 on opens on
  the desktop, the board or Mac Anu, which the starts already had.

## The replay's `virus_core`

`virus_core` at level 1 calls `WORLD_MAN::SimGenerateCode` (main
0x0019e5c0) on the area's words. That sets the field generator's `seed`
and `randcnt` and `WORLD_MAN`'s generated area, not the save, and a
start's own scene set-up makes them again. The replay host implements it
as nothing to keep. The protect bit and the protect items the instruction
also handles are the interpreter's.

## What the new starts run into

Each ran 4,000 frames with X every 25 frames.
- **Starts 15-21 and 23-30.** They open their event and run without a
  host default. Only a talk-through was tried, so a start that needs a
  walk somewhere has only begun.
- **31, the ending.** Its stream 15 plays on the desktop's set-up.
- **22 (PIRO02).** `piros_colour` and its wait (`begin` and `end`) are
  missing. More in the list below.

## Checked

- **The story-start test.** `story_starts_open_their_event` now covers all
  24 starts: each opens its event's first block, and no earlier event
  plays again.
- **The rest.** The workspace's tests, clippy, fmt and the docs check
  pass.

**Still unknown:**
- **Dun Loireag is not ported.** The town world refuses any town but Mac
  Anu (`World::enter`: "only Mac Anu (0) is ported"). Events 22-30 move
  the story there (`in_town 1`), so the story cannot be played past event
  21 until `ROOTTOWN02` is ported: its constructor (gcmn 0x004240c0, 3.9
  KB), `DrawBG`, `DrawObj`, `DrawObj2`, `DrawFloor`, `DrawMap` (3.9 KB),
  `Draw`, and its NPCs, merchants, gate and landmarks.
- **`piros_colour` is not ported** (event 22). If Piros (registry id 8) is
  loaded, a switch on `eventStatus` (+0x64f9) over 0-13 sets his
  `affectColorFix` (+0xa6), `affectColor` (+0xac) and its count (+0xaa):
  - case 0 clears them;
  - the other cases flash the screen (`EntryFlash(8, 0x602080ff, ...)`)
    and wait 5 frames;
  - some cases play SE 74 and open `DispInfo` with an event message
    picked by another `eventStatus` byte (+0x6510).

  The town's characters do not draw `affectColor` yet.
- **Not played past the opening.** The later events' fields and dungeons
  (areas 17-31), their fights and bosses.
