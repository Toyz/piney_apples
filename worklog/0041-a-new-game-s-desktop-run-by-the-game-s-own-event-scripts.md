---
number: 41
title: A new game's desktop, run by the game's own event scripts
date: 2026-09-23
area: script, ui, test
files: crates/piney-game/src/desktop.rs, crates/piney-game/src/main.rs
---

# 41. A new game's desktop, run by the game's own event scripts

[[37]] put the desktop on screen with its mail delivered by hand (`--mail`).
[[40]] built the event engine. This entry runs the two together, in the
game's own order, so a new game's desktop is what the scripts make it.

## The order

`piney-game`'s desktop mode now runs the official scripts from the disc,
unless given `--no-events`:
- **Boot.** `ccStartEvent(1, 0)` on a fresh save, as `ccThMother` does.
- **Setup.** `ccStartThEvent`, then `ccEnableThEvent(0)` and `(2)`, each
  run to its settled pass before the desktop is built, as `ccSetupDesktop`
  waits for them. Event 1's pass at phase 0 takes 59 frames, the number the
  eemu check of [[40]] measured, and the pass at phase 2 takes 2. Then
  `(4)`.
- **Every game frame.**
  1. The desktop gets the events' lock mask (`ccEvent.operate`) and a
     cleared `operateSet`.
  2. The desktop steps, recording the first operation the player tried.
  3. That goes back to the interpreter, and `ccThEvent` makes its pass.

  Both sides write the one `SaveData`.

The host answers the scripts with the desktop's status (2), the pad's
pressed buttons and the save. The scripts' windows are printed to the
terminal: a setup line counts as shown, and a line on the desktop waits for
Cross, since the desktop's own message window (`ccDtMenu`) is not ported.
Name entry keeps the new game's default name.

## What a new game does

This is played through `piney-game` with scripted presses:
- The setup prints event 1's two lines, from the disc.
- The desktop opens with mails 4, 5 and 320 delivered, NEWS marked new
  (headlines 0-3 and 37 posted), and every icon but MAILER locked:
  `operate` is 0x3f7fd.
- Choosing NEWS is refused, and event 1's block 1 says "I'll check my
  e-mail first."
- The inbox lists the three mails, newest first. Once 5 and 4 are read,
  block 2 clears the locks (`operate` 0) in the same frame, and NEWS opens
  on the posted headlines, the last being "WNC Official Statement".

`new_game_events_run_on_the_desktop` checks that sequence: delivery, the
lock, the refusal message, and the release after reading.

**Still unknown:**
- **The scripts' windows.** They need `ccDtMenu` ported to be drawn on the
  desktop.
- **Name entry.** `NameEntry_Control` is not ported.
- **Streams and overlays.** The script's movie streams and overlay loads are
  accepted and skipped.
- **Before the desktop.** What the setup screen shows during those 59
  frames, and the title and opening of `DEMO.PRG`.
