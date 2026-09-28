---
number: 58
title: The World's top page (TOPPAGE.PRG): log in, the board, log out, and the hand-off to the field game
date: 2026-09-23
area: ui, script, test
files: crates/piney-toppage, crates/piney-game/src/toppage.rs, crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, tools/test_toppage_rs.py, docs/engine/toppage.md
---

# 58. The World's top page (TOPPAGE.PRG): log in, the board, log out, and the hand-off to the field game

The desktop's THE WORLD icon asks for mode 4, `ccSetupToppage` with
`TOPPAGE.PRG`. Since [[44]] the runtime had answered by entering the desktop
again. A helper agent ported the page as `piney-toppage`; I put it in the
runtime. The reference is [the top page](../docs/engine/toppage.md).

## What the page is

`TOPPAGE.PRG` is small: `toppage.cpp` (14 functions) and `bbs.cpp` (35),
plus the board's tables. Each frame runs, in order:
- **The system menu.** The desktop's own `ccThDtMenu`. Its status tests
  compare against 1 (the title), so on the page (status 3) it behaves as on
  the desktop.
- **The page's task.**
  - A 151-frame log-in animation (`ANM_xdttopst`). Cancel skips it, except
    on frames 1 and 60.
  - A menu of LOG IN, BOARD and QUIT, with a NEW mark when a post is new.
    The event scripts lock the commands as operations 6-8 and 25-27.
  - The board: the threads and posts the scripts posted (`bbsList`: 1 new,
    3 read, 7 the player's own, typed out two bytes every 6 frames), and
    the Time Idol ranking in thread 29.
- **The fader and the menu**, drawn on top.

**Leaving the page.** Each command fades to black over 31 frames:
- QUIT asks for mode 3, the desktop.
- LOG IN asks, in one frame:
  - `ChangeRequest(5, 8)`: `ccSetupNewGame`, GCMN.PRG;
  - `ChangeArea(0, lastTown)`: area 0 (a town) and `saveData.lastTown`,
    with the town's server from `@1489`;
  - `ChangeRequest(6, 7)`.

There is no server or town choice on the page itself: the town is
whatever the scripts last set with `last_town`.

**Copied exactly.** Three things the game does are ported as they are:
- `_ChangeMode` reads the idle animation's table at `volumeNum - 1`
  through a stack offset, so Infection shows `ANM_xdttop1a`.
- The board's scroll bars divide 0 by 0 when a list is exactly one page
  long, and the data has such lists. The port uses the EE's result as
  PCSX2 models it.
- The key-repeat counters are main-executable globals, so they carry over
  between visits.

## In the runtime

- **The session.**
  - THE WORLD now enters `TopPageMode`, and QUIT goes back to the desktop.
  - LOG IN records the area and town. Mode 5/6 (the field game) is not
    ported yet, so it enters the desktop again for now.
- **The event task.** It runs over the page as on the desktop:
  - the passes at phases 0 and 2 with `ccGame.status` 3, then the page and
    phase 4, then the event task before the page each frame;
  - the bridge gained a top-page target and a per-mode status, 2 on the
    desktop and 3 on the board;
  - a script asking for another mode during the passes abandons the page.
- **The page's requests.** They map to the runtime's events:
  - `SqLoad(0)` to `sqDataToppage`, whose sequence 0 is the board's music;
  - the system menu's requests as on the desktop.

## Checked

- **Against the game's code.** `tools/test_toppage_rs.py` runs
  TOPPAGE.PRG's constructor and `Main` natively in eemu beside the port,
  frame by frame, over 6 scenario tests. Each frame it compares:
  - every call: animation steps, text draws with their bytes, mask cells,
    views, fader entries;
  - the sounds and requests;
  - the full state of both controllers, the scroll bars, every thread and
    post, the whole `bbsList` and the key-repeat globals;
  - the scene fields after LOG IN.

  The runs cover a fresh save, event 1's posts, the player's own posts, the
  Time Idol post, the longest and the one-page thread, a parody save with
  every thread, locked commands, the system menu over the board, and six
  random 900-frame runs. Two deliberate mutations were caught. I re-ran it
  in a clean worktree and all 6 pass.
- **The crate.** `piney-toppage` has 6 unit tests and 6 disc tests.
- **The runtime.** `the_world_top_page_and_back` goes from a new game's
  desktop to THE WORLD, past the log-in animation, QUIT back to the desktop,
  THE WORLD again, and LOG IN: the area (0, the save's last town) is kept.
  The workspace's tests pass, and clippy (also with `trace`) and fmt are
  clean.
- **Shots.** The CPU GS model draws the page (the logo, LOG IN / BOARD /
  QUIT) and the board with event 1's NEW posts.

**Still unknown:**
- **The first frame.** Whether the task's first `Main` runs in the frame
  it starts; the port draws nothing for one frame.
- **The fader under the menu.** How it behaves while the system menu
  freezes the layers.
- **Fading out.** `SoundFadeOut` maps to the runtime's immediate stop of
  every sequence; the game's `ccSoundFadeOut` fades over 8 frames.
- **The setup passes.** They run at once, not a frame at a time, and draw
  nothing.
- **The field game.** Mode 5/6 is next.
