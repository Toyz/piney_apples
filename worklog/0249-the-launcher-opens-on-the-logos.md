---
number: 249
title: The launcher opens on the logos
date: 2026-09-27
area: ui
files: crates/piney-game/src/launcher.rs, crates/piney-game/src/movie.rs, crates/piney-game/src/session.rs, crates/piney-game/src/main.rs, crates/piney-demo/src/lib.rs, README.md
---

# 249. The launcher opens on the logos

Asked for from play: the Bandai and CyberConnect2 intro movies should come
before the launcher's game selection, as they come before a disc's title.

## What changed

- **The launcher** now plays `PSS/LOGO_B.PSS` and then `PSS/LOGO_C.PSS`
  before it sets up the selector. These are the first two of the title's
  `LOGO_MOVIES`, and the port's list is `launcher::LOGOS`. They come from
  Outbreak's disc when the build has it, else from the first disc it has.
- **As the title plays them.** They are silent, one picture every two
  frames, and the launcher's frame rate is 1 while they play (2 for the
  selector's stream). Cross, START or circle stops them once twelve
  pictures are decoded. As `ccDecodeMpeg`'s -1 does in `LogoMain`, one
  push ends both logos at once.
- **When a movie will not open,** it counts as played, as in the title.
- **The selector is set up** (its stream opened and its sound asked for)
  only once the logos are over, so nothing of it starts under them.
- **The movie player moved.** The title's `Playing` is now in
  `crate::movie`, which the session and the launcher share.
  `step_stopped_by` takes the stopping buttons; the launcher has no save
  to read OK and cancel from.

## Not showing the logos twice

The chosen part's title would have played Bandai's and CyberConnect2's
logos again on its first boot. `piney_demo::Config` gains
`logos_played`, the port's own field, which the title's first boot puts
into `m_LogoAct`. The launcher's boot passes 2, through
`Session::after_logos` and `Options::logos_played`, so the title starts
at `LOGO_H.PSS` (the Project .hack logo) and then plays the part's own
opening. A soft reset skips the logos as before, and a disc started on
its own still plays all four.

## Checked

A headless run from the build shows the frames in turn:
- frame 100: the Bandai logo;
- frame 450: CyberConnect2's;
- frame 1200: the selector, its rows at rest;
- a cross at frame 1250 starts Infection at frame 1276, and its title
  opens on the Project .hack logo (frame 1400).

The piney-game suite (134) and piney-demo's pass.

**Still unknown:** Whether the four discs' `LOGO_B.PSS` and `LOGO_C.PSS`
are the same file. The build's dedup would say; the launcher plays
Outbreak's either way.
