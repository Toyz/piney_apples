---
number: 174
title: "The church's black screen: a field set-up's windows on the set-up's own screen"
date: 2026-09-26
area: ui
files: crates/piney-desktop/src/setup.rs, crates/piney-desktop/src/kanji.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/message.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/event11.rs, crates/piney-stream/examples/stream_time.rs, docs/engine/event-vm.md, BUGS.md
---

# 174. The church's black screen: a field set-up's windows on the set-up's own screen

Story 11, in the church on the Hidden Forbidden Holy Ground: Balmung's
and BlackRose's streams played, and then the screen stayed black. The
title read "loading 1" with Kite in act 13, just before Data Drain.

**What the church does.** Event 11's block 22 plays streams 8 and 9 in
the set-up's pass at phase 0. Block 24 runs in the pass at phase 2, after
the field's files load and before the field's tasks start. It opens four
`info_now` windows ("The book. / Open the book." and the three after
it), then plays streams 10-12 and gives Data Drain. A window in a pass
below phase 4 is not the menu's `ccMsg`. The event makes its own window
on the set-up's black screen (`docs/engine/event-vm.md`, "Instructions
that take frames").

**The bug.** The field host sent those windows to the field UI. The
field UI neither steps nor draws until play, so the load's hold showed
black. The pass then waited on a button for a window nobody could see.
Its voice requests also sat unsent until play.

**The fix.** The desktop's set-up already had the right screen
(`piney_desktop::setup::SetupScreen`). `SetupScreen::with` now builds it
from fonts and a window texture already read, and the field UI keeps one.
While `set_setup` is on, the windows and announcements open, are checked
and close there. The area turns it on for its passes (`Setup::Pass`),
draws the screen while a window is up (a stream the scripts play still
wins), and passes the window's voice and sounds on. The church now shows
the four lines, white on black with no frame, and the book's streams
follow.

`event_11_church_ends_with_blackrose_out` now asserts that a textured
draw (the glyphs) is in the frame while the load holds on
`message_open 11 24`. It fails with the set-up screen switched off and
passes with it on. Of Infection's scripts, this is the only field set-up
with windows: events 10 and 31 have them in the desktop's set-up, which
already had its screen.

**On the way:** `stream_time` (piney-stream's examples) times each step
and GPU draw of a stream, for the Skeith lag that BUGS.md lists. It can
also write a range of frames (`--shots`) and print a frame's draw
commands (`--dump`). Stream 5's only slow draw is frame 1834, at 151 ms.

**Still unknown:** Whether the game's `info_now` on the set-up screen
places its lines exactly where `SetupScreen` does was not compared with a
picture of the game. The tail of the last window before stream 10 was
not timed against the game.
