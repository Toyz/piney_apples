---
number: 321
title: ccSoundFadeOut fades the music over 8 frames instead of cutting it
date: 2026-10-01
area: audio
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-game/src/main.rs
resolves: 58
---

# 321. ccSoundFadeOut fades the music over 8 frames instead of cutting it

[[58]] left this open: the port mapped `Event::SoundFadeOut` to
`Audio::bgm_stop`, which stops every sequence at once. The game's
`ccSoundFadeOut` fades. Every mode change that asks for it (the desktop's,
the board's, the towns' and the fields' leaving) cut the music short.

`ccSoundFadeOut` (INF SLUS_202.67:0x0017ae60) does the following for each
loaded sequence: `sqNum` at `ccSnd` +0x64, with the SQTBLs at +0x70, +0x7c
and +0x88, as `midi port`, `hd port`, `vol`. On the sequence's fade
(+0x94 + 20 x the midi port) it sets:
- the switch to 3;
- the time (+0xa0) to 8;
- the volume (+0xa4) to `hdSynPortVol[hd port] << 8`;
- the rate (+0x9c) to that over 8, rounded toward zero (`sra 3` with the
  +7 adjust);
- the end (+0x98) to 0.

That is `ccSqFade(n, 0, 8, 3)` written inline. Switch bit 1 stops the
sequence when its fade ends.

`Driver::sound_fade_out` calls `sq_fade(sq, 0, 8, 3)` for each loaded
sequence; its rate `(now - end) / t` is the same truncating division.
`Audio::sound_fade_out` exposes it, and `main` routes `SoundFadeOut` there.
`bgm_stop` stays for what really stops at once.

The test is `a_sound_fade_out_fades_before_it_stops` (piney-audio, driver):
two sequences at volume 200. After the call, the first frame's port 1 is
between 0 and 200. Both sequences stop after frame 1, with their ports at 0.
The piney-audio and piney-game suites pass.

**Still unknown:** nothing about this call. Whether a mode's set-up can start
its own bank before the 8 frames are up (cutting the fade) follows each
mode's frame order and was not checked per mode.
