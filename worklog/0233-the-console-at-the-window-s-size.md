---
number: 233
title: The console at the window's size
date: 2026-09-27
area: ui
files: crates/piney-game/src/console.rs, crates/piney-game/src/main.rs, crates/piney-game/src/session.rs, crates/piney-game/src/area.rs, crates/piney-gs/src/lib.rs, crates/piney-desktop/src/kanji.rs
---

# 233. The console at the window's size

Reported from play: the console worked, but its text was far too large
and there was no way to move around the line or the log. The window's
title was the long `Mode::title` line.

## Why it was large

The console drew into the GS frame, 512 x 448, which the presenter then
stretched to the window. Each 8 x 16 glyph came out two or three times
its size, blurred by the scaling. The glyphs were also cut off on the
right. The port read `ef8x16`'s trim as the glyph's width. It is really
the texels to remove, and a glyph is left-aligned in its cell, so the
advance is 8 less the trim.

## The console now

- **Drawn at window pixels.** `piney_gs::Presenter` gained an overlay: an
  RGBA picture laid over the top of the swapchain after the frame's blit,
  with a nearest sampler. `Console::overlay(width, height)` builds it:
  - a band of 2/5 of the window's height;
  - a rule under the band;
  - the log, the newest line just above the prompt;
  - the prompt, with a cursor bar.
  The glyphs come from the game's own `ef8x16` small font
  (`Fonts::small_ascii`), at 1:1, or 2:1 on windows 1400 pixels high or
  more. A help line's second column starts at a fixed x, since the font
  is proportional. `Console::draw` still draws into the GS frame, but only
  for `--shot`.
- **Keys.**
  - Editing: Left, Right, Home, End, Backspace, Delete, Ctrl+A, Ctrl+E,
    Ctrl+U (clear the line), Ctrl+L (clear the log).
  - History: Up, Down.
  - Scrollback: PageUp, PageDown, over a log of 1000 lines.
  - Tab: completes a command, from the names in `help`.
  - Escape: closes the console.
  The app follows Ctrl through `ModifiersChanged`.
- **Window title.** Now `piney - <volume title>`.

## God-mode commands

| command       | what it does |
|---------------|--------------|
| `god`         | Toggles god mode. While on, the session calls `heal_party` after each area frame. |
| `kill`        | `AreaMode::kill_enemies`: each enemy standing takes `EntryAffect(1, 9999)` from Kite, so it falls with its own exp and drops. |
| `protect`     | `AreaMode::break_protects`: each enemy's `ppCount` becomes 1800 (60 s), so Data Drain takes it. |
| `infection N` | The bracelet's infection (save +0x676e), clamped to 0..=100. With no N, it shows the value. |

The new test `the_console_cheats` walks into event 3's fight and checks
the four commands:
- `protect` and `kill` report the goblins, and they fall;
- under `god`, Kite's HP, set to 1, is full again 30 frames later;
- `infection 150` stores 100.

`console_picture` (ignored) writes the console over a grey window to the
shots folder, to look at it by hand.

**Still unknown:** The game's log lines (stderr) do not reach the
console. Teeing them would need the process's stderr redirected into a
pipe the app reads. Nothing is scrolled horizontally, so a line wider
than the window is cut off.
