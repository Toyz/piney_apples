---
number: 236
title: The story autopilot through the gate and the mail
date: 2026-09-27
area: test
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/world.rs, crates/piney-game/src/toppage.rs
---

# 236. The story autopilot through the gate and the mail

`mutation_story_survey` starts each of Mutation's story points (102-116)
and drives it with the tests' `story_player`. Every start from 102 to 114
stopped in its town at events phase 5. The player only answered windows
and menus; Mutation's story asks it to go places. Event 102, for
instance:
- **block 3** needs mail 32 read;
- **block 4** marks area 43 and plays on entering its field;
- **block 5**, back in Dun Loireag, plays stream 27 and sends mail 33.

Event 103 then needs mail 33 read (`mail_got`: `mailList` 4 - 6, read
on the desktop), unlocks town 2, and marks area 44 there.

## What the autopilot does now

- **To the gate** (`gate_player`, in a town, no window waiting). Kite walks
  to the Chaos Gate (gimmick 16) and speaks to it, with one of three
  goals:
  - **A story area** the events marked (`gateListMark[server]`) that the
    town's Word List (`gateOrderList[server]`) holds: menu 28, Word List
    (59), its row, Warp.
  - **Log out**, with a mail unread (`mailList` 1 or 2). PERSONAL (menu 0),
    Log Out (11), OK. The gate's own menu has no Log Out; its items are
    57 - 61.
  - **Another town**, whose server has a marked area: menu 28, Towns
    (61), its row in `townMoveFlag`'s list, OK.
- **Streams in a town.** The town's scripts can play a stream in the
  set-up pass (stream 27 at phase 0). Cancel and START skip it, as in the
  fields. `WorldMode::streaming` tells.
- **The top page.** A stream over the board is skipped, and an event
  block's windows take OK. With a mail unread it picks Log out
  (`CMD_QUIT`) to the desktop, whose player reads the mail; else Log in.
  Without this it logged back in, and the town logged out again, forever.
- `WorldMode` gained `server()` and `town_number()`; `TopPageMode` gained
  test-only `streaming()` and `event_playing()`.
- The survey takes `PINEY_SURVEY_ONLY`, `PINEY_SURVEY_FRAMES` and
  `PINEY_SURVEY_CALLS` (the last host calls when a start ends).

## The survey now (16000 frames)

- **102** plays through field 43, the stream, the log out, the mail, the
  log in, and the Towns list to town 2. It stops in event 103's block 4
  there (NPC talks to find).
- **103** reaches the same place.
- **106** logs out to the desktop.
- **107 - 109, 112, 113, 115** go to the gate again and again: the Word
  List (59) and a message (88), then back.
- **111** stops in menu 62.
- **104, 105, 110, 114** still wait in Dun Loireag; neither an area, a
  mail nor a town is marked for them.

The Infection tests that share `story_player` pass unchanged (131).

**Still unknown:** Why the warps of 107 - 115 come back to menu 88 (a
refusal, a party the area wants, or the wrong row). What the waiting
starts need: an NPC to speak to (`add_target`), which the autopilot does
not look for yet.
