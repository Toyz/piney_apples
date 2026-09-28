---
number: 82
title: Story start points, the way out of the dungeon to the desktop, and the board's announcements
date: 2026-09-24
area: script, ui, test
files: crates/piney-game/src/start.rs, crates/piney-game/src/session.rs, crates/piney-game/src/main.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/story.rs, crates/piney-game/src/toppage.rs, crates/piney-event/src/vm/exec.rs, tools/test_desktop_announce_rs.py, docs/engine/event-vm.md, docs/engine/desktop.md
---

# 82. Story start points, the way out of the dungeon to the desktop, and the board's announcements

A survey of the opening arc (events 3 and 4, then 10-12) listed what the
port still lacked. Three items did not touch the combat work under way,
and an agent has done them:
- start points in the story;
- the scripts' mode changes to the desktop;
- the desktop's announcements.

## Starting later in the story

`piney-game --mode story:N` starts where event N opens, for N = 3, 4, 10,
11, 12, 13 and 14.
- **How the state is built.** Not by setting bytes. It starts from a new
  game's save and the boot's event task, then replays each earlier story
  event with the VM's own `ccEventFlagSet`, in order: 1, 2, 3, 4, 10, 11,
  12, 13. Log in's `ccSetupNewGame` is applied after event 1.
- **The party.** It is not in the save, so it is rebuilt from the party
  instructions event 2 ran.
- **Where each start is:**
  - 3: field 14;
  - 4: its dungeon's first room;
  - 10 and 12: the desktop;
  - 11 and 13: Mac Anu;
  - 14: the top page.

`story:3` is compared with event 2 actually played: the scene, the area
data, both party managers, and the flags and bits 60 frames on.

## Leaving The World for the desktop

- **Mode 3 from the scripts.** Event 4's end asks for mode 3 (the desktop)
  from the dungeon, then again from the desktop itself. The session
  refused both. It now takes mode 3 from a field, a dungeon, Mac Anu and
  the desktop. From the desktop it sets the desktop up again on the same
  event task.
- **The frame rate.** `ccSetupDesktop` keeps the frame rate of the mode
  before it, 2 when entered from The World, until its `SetFrameRate(1)`
  after the phase-2 pass (0x001684dc). So streams 4-6 (Orca's fall), which
  play on the desktop's setup screen, run at 30 frames a second as in the
  game.
- **A pending change.** A pass that asks for another mode abandons the
  set-up, as `ccSetupDesktop` does.

## The desktop's announcements

- **The texts.** `DispInfo`'s windows open on the desktop, the board and
  the setup screen:
  - the gate address ("#B" and the server, the three words, then "is added
    to the Word List");
  - the member address, with the name from `spcNameList`;
  - the desktop item.

  The texts come from the executable, DEMO.PRG and GCMN.PRG.
- **The words.** The desktop's host now answers `story_area`, so the
  board's `gate_add` adds the area's real words on its real server.
- **A VM fix.** The eemu check found it: the game's `desktop_item` puts no
  bound on the id, so an id past a list's bits sets a bit in the next
  list. The VM now does the same, within the save.

## Checked

- **Announcements against the game.** `tools/test_desktop_announce_rs.py`
  runs the game's `Execute` with `DispInfo` for 159 cases:
  - `gate_add_msg` for all 126 area codes;
  - members 1-17;
  - 16 desktop items.

  The Rust side replays them from a fixture: 0 mismatches in the lines,
  the frames, the sounds and the changed save bytes.
- **Runtime tests** (the port's own behaviour):
  - every story start plays event N's first block, and no earlier event
    replays;
  - `story_3` equals event 2 played;
  - event 4's end reaches the desktop, plays streams 4, 5 and 6 at rate 2,
    and opens event 10;
  - mode 3 from the town;
  - reading the board's posts adds areas 17, 28 and 29 with their windows.
- **Re-run in a clean worktree.** The announcement, desktop, top page,
  event VM, field host and demo suites pass, with the workspace's tests,
  clippy, fmt and the docs check.
- **Shots.**
  - Each story start.
  - Stream 5 on the desktop's setup screen.
  - The board's "Expansive Haunted Sea of Sand is added to the Word
    List".

**Still unknown:**
- **`WORLD_MAN::SetEventData` is not ported.** In `story:4` event 4's room
  blocks all fire in the first room. It is with the combat agent's event 4
  work.
- **Mac Anu's gate-address window** (event 11's `gate_add_msg 15`) is
  empty: the field UI composes no gate lines yet.
- **Not modelled:**
  - streams 4-6's subtitles;
  - event 10's `sound 8`;
  - the load time between the mode change and the desktop's set-up.
- **What a replayed start lacks.** It has no experience or items from the
  earlier fights, and mails and posts sit at the state
  `ccEventFlagSet` leaves.
- **`story:12` starts on the desktop.** In the game its first block would
  run in the first set-up after event 11, probably the top page.
