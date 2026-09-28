---
number: 51
title: The title's Option is the system menu in its title mode, and the title music written up
date: 2026-09-23
area: ui, audio, test
files: crates/piney-desktop/src/dtmenu.rs, crates/piney-demo/src/seam.rs, crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, tools/test_desktop_menu_rs.py, docs/engine/title.md
---

# 51. The title's Option is the system menu in its title mode, and the title music written up

The user found the title's Option missing; since [[43]] it had been a stub.
It turns out not to be a screen of the title's own. The title agent found
that it opens the same `ccDtMenu` the desktop's START opens, in a title
mode.

## The menu's title mode

`ccSetupDemo` sets `ccGame.status` to 1; the desktop sets 2. About 20
branches in `ccDtMenu` test it:
- **The title keeps running.** `OpenMenu` puts no task to sleep, freezes
  no layer and plays no sound 16.
- **No dim.** `SystemMenu` does not dim the screen.
- **Other sounds.** OK and back play 4 and 7, not the desktop's 18 and 19.
- **No select cursor.** The chosen row is drawn in `ccSpriteColorTable[20]`
  instead, in the list and on every option page. List rows are 24 px
  apart.
- **The list.** `PlayOption` asks for list 1: Controller, Vibrate, Adjust
  Screen, Sound, Voiceover, Movie Text. The rows sit on the title's own
  `op02` panel.
- **START does nothing.** `enableReset` is 0 on the title.

`DtMenu` has a `title` flag with those branches, and the title's menu seam
now runs it (`seam::TitleMenu`). The options write the title's save.

## In the runtime

- **The menu's requests.** `Request::Menu` carries them. They go through
  the same mapping as the desktop's (`desktop::event`), so Sound changes
  the volumes live on the title.
- **Into the game.** The options reach the game in the save New Game hands
  over.
- **The leaving fade.** The title agent's write-up of the music (below)
  found that the runtime cut New Game's music fade after three of its nine
  frames. The desktop's `ccAllSoundOff` came the frame after
  `ChangeRequest`. In the game, `ccThMother` first loads `DESKTOP.PRG` from
  the disc, with the screen black, and the sound task steps the fade
  meanwhile. The session now waits 8 black frames (`LOAD_FRAMES`) before
  the desktop's setup. That is long enough for the fade; the real load
  time is not measured.

## The title music

"The music" in [the title page](../docs/engine/title.md) covers:
- what `ccSndSQLoad(7)` loads, and the IOP commands it sends;
- what `ccSqPlay`, `ccSqStop`, `ccSqFade` and `ccSetMainVol` do;
- all seven music calls in `DEMO.PRG`.

Two findings:
- **It plays once.** The sequence has no loop and ends at 45.6 s.
- **The game never learns it ended.** Nothing sets `sqStFlag`, so a
  second `ccSqPlay(0)` stays silent until a `ccSqStop(0)`. The attract loop
  stops the music before it replays.

## Checked

- **Against the game's code.** `test_desktop_menu_rs.py` runs the game's
  `ccThDtMenu` in eemu with status 1 beside the port: the list, and every
  option page entered. State, packets, sounds, info lines and requests
  match each frame. All 5 tests pass, and `test_demo_rs.py` (18),
  `test_desktop_rs.py` (20) and `test_desktop_data_rs.py` (6) still pass.
- **The title crate.** `option_sets_vibration_through_the_system_menu`:
  Vibrate OFF through the menu, the save, the request, the title's sound 4.
- **The runtime.**
  - `title_option_carries_into_the_game` sets Vibrate OFF on the title,
    backs out, starts a New Game, and finds it off in the desktop's save.
  - `setup_lines_are_voiced` now also checks that New Game's fade gets its
    nine frames before the desktop's `ccAllSoundOff`.

  `piney-game` has 8 tests, all passing; clippy (also with `trace`) and
  fmt are clean.
- **A shot.** A GPU shot shows the Option list on the title's panel, the
  chosen row green.

**Still unknown:**
- **`LOAD_FRAMES`.** The real time between `ChangeRequest` and
  `ccSetupDesktop`'s `ccAllSoundOff`.
- **All sound off.** The port's all-sound-off cuts only the voices, where
  SNDBASE also zeroes every port volume and stops the sequencers. No
  difference can be heard on the title, because a bank load always
  follows.
- **Not applied yet.** The display offset, vibration and camera scheme the
  menu sets. Nothing in the runtime uses them.
