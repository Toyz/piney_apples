---
number: 332
title: "The Mono option reaches the synthesizer: ccSetOutputMode and sdCommand 3"
date: 2026-10-01
area: audio, ui
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/main.rs, crates/piney-game/src/session.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/desktop.rs, docs/engine/sound.md
---

# 332. The Mono option reaches the synthesizer: ccSetOutputMode and sdCommand 3

The sound page's Unknown asked which `sceHSyn_SetOutputMode` value the
stereo/mono option sends. Following it showed that the port dropped the
option entirely. Addresses are INF SLUS_202.67 unless marked SNDBASE.

The chain:
1. The OPTION Sound page's Output row (desktop and The World) writes
   `saveData.output` (+0x8424): 0 Mono, 1 Stereo. A new save copies
   `ccSnd.outputMode` (+0x0c), which the `ccSound` constructor sets to 1
   (0x00180ee4).
2. `ccSaveData::SetSoundEnv` calls `ccSetOutputMode(output)` (0x00178658).
3. `ccSetOutputMode` (0x001795f0) ignores the mode already in force and
   any value past 1. Otherwise it stores the mode and queues
   `sdRemote[2] = 3` with `sdData[2] = m`.
4. The next `sdCommand`, case 3, sends `ccSndCmd(0x100, m)` (0x00181870).
5. SNDBASE's `ccSoundFunc` passes `m` to `sceHSyn_SetOutputMode`
   (SNDBASE 0x0d68). 0 is mono: both pan gains 128.

So Mono sends 0 and Stereo 1. `piney-audio`'s synthesizer already had
`Synth::mono`, the pan law for mode 0. Nothing set it: the menus' `SoundEnv`
request carried `output`, but `Event::Volumes` left it out.

The port now carries it the game's way:
- `Event::Volumes` has `output`, from both menus and from a loaded save;
- `Audio::set_output_mode` calls `Driver::set_output_mode`, which is
  `ccSetOutputMode`;
- the driver's task, at `sdCommand`'s place (after the master volume,
  before the reverb), sends `Command::OutputMode(m)`;
- the engine sets `synth.mono = m == 0`.

The test is `the_output_mode_goes_out_at_the_next_sd_command` (piney-audio,
driver). Stereo again, or 2, sends nothing. Mono goes out once, at the next
frame. `tools/sound_ee.py` does not log `ccSndCmd(0x100)`, so the driver
fixture is unchanged.

## Also: ccSeOffLoop's ids

The same Unknown list held `ccSeOffLoop` with an id outside 0-7, which
would write other `ccSound` bytes (-1 into `sqNum` for -1). Its three
callers in main and the four overlays are:
- `ccSound::strSeEnd` (0x00183ff8);
- gcmn `ccBoss03::Affect` (0x00489e58);
- gcmn `ccBoss03::OnThinkLeafGrow` (0x0048da7c).

Each passes an id kept from `ccSeOn3DLoop` (0-7, or -1 when no slot was
free) and skips -1. None can pass one outside 0-7.

**Still unknown:** the mono pan law was not heard against the game.
