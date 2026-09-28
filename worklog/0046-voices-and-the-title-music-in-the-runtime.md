---
number: 46
title: Voices and the title music in the runtime
date: 2026-09-23
area: audio, test
files: crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/main.rs, crates/piney-game/src/mode.rs
---

# 46. Voices and the title music in the runtime

After [[45]] the game was silent in two places: message windows had no
voice-over, and the title had no music. A helper agent ported both into
`piney-audio` (commit 088f107), checked against the game's own code; the
details are in [the voice page](../docs/formats/voice.md) and
[the sound page](../docs/engine/sound.md). This entry puts them in the
runtime.

## What the runtime now asks for

Each mode hands its requests to `piney-game`, which passes them to the
sound driver:
- **Voice lines.** Every message window the scripts open asks for
  `ccEvVoiceRequest(event, msg)`. The camera tutorial's prompts ask with
  event 3, as the game does. The save's Voiceover option (+0x842c) and
  parody flag (+0x842b) are read at each request, so changing the option
  in OPTION takes effect on the next line.
- **Stopping a line.** `ccEvVoiceStop` comes from three places: a camera
  tutorial prompt closing, the scripts' sound command 7, and the desktop's
  own message windows.
- **The title's music.** `ccSetupDemo` turns all sound off and loads the
  title's bank (`ccSndSQLoad(7)`). The title's `ccSqPlay`, `ccSqStop`,
  `ccSqFade` and `ccSetMainVol` now reach it; before, they were only
  printed.
- **Changing mode.** `ccGame::ChangeRequest` calls
  `ccSound::gameInterrupt`, and each mode's setup starts with
  `ccAllSoundOff`.

## Timing

With the logos and the intro stream counted as played, the title's music
starts at frame 74, when `LogoMain` calls `ccSqPlay(0)`. New Game fades it
over 8 frames. The first setup line's voice (event 1, message 1) is asked
for two frames after the change to the desktop.

## Also: texel bleed at sprite edges

Commit 63635fb fixed the text glitches seen when moving the cursor.
Sprite UVs sat exactly on texel boundaries, and the GPU sampled the
neighbouring glyph or icon there. A 1/512 texel bias now makes it pick the
texel the GS picks. Text areas now match the CPU GS model.

## Checked

`setup_lines_are_voiced` runs the session from power-on with a headless
`Audio`, doing what the window loop does each frame: handle the requests,
`frame()`, then render one frame of samples. Two checks:
- The title's music sounds in more than 90 of the 95 frames between
  `ccSqPlay(0)` and New Game's fade.
- Channel 0 streams event 1's lines for more than 60 frames of the setup.

`piney-game` has 5 tests, all passing; clippy and fmt are clean.

**Still unknown:**
- **Movies and streams.** The logos, `OPENING.PSS` and the intro stream
  still count as played. Their audio, and the title music's real start
  after them, wait for the movie and stream ports.
- **The field's voices.** Voices from `ccVoiceRequest` (negative ids) and
  the per-character voices have no caller until the field runs.
- **The lit icons.** Against the CPU GS model, the title's lit icons still
  differ. This may be the "transparency" the user saw; the title agent is
  checking the lighting.
