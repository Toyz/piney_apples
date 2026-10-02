---
number: 320
title: "New Game's name entry was silent: ccSetupDesktop's ccAllSoundOff comes after the phase-0 pass"
date: 2026-10-01
area: audio, script
files: crates/piney-game/src/desktop.rs, crates/piney-game/src/session.rs, crates/piney-game/src/launcher.rs
---

# 320. New Game's name entry was silent: ccSetupDesktop's ccAllSoundOff comes after the phase-0 pass

Reported several times since 2026-09-28 (INF), and shown in a player's video:
New Game's name entry had no keyboard sounds, and its confirm was silent
too. Sound came back on the desktop.

## Why the tests missed it

A test of 2026-10-01 measured the name entry's sounds through `main`'s
routing into a headless engine, and heard all of them. But it reached the
setup through the session tests' `new_game`, which drops the events of the
frames it presses through. Among those were the title's `GameInterrupt`
and the setup's `AllSoundOff`, so the engine never heard the latter.

The player's `pad_log` was replayed as `main` runs it:
- the launcher on the build's discs;
- `Event::Boot`, then `Session::after_logos` with a fresh engine.

That showed every sound of the setup at peak 0, then loud again at the
desktop. The diagnostic is `replay_through_the_launcher` (`PINEY_PADLOG=FILE`).

## The cause

SNDBASE's `allSoundOff` (INF MODULES/SNDBASE.IRX:0x2e5c, command 0x140)
does four things on each of the four ports:
- `sceHSyn_AllNoteOff`;
- `sceHSyn_AllSoundOff`;
- `sceHSyn_SetVolume(port, 0)`, port 0 included;
- `sqAllStop`.

The sound-effect port stays at 0 until a bank load sets it back to `seVol`.

`ccSetupDesktop` (INF SLUS_202.67:0x00168320) runs, in order:
1. the event task's phase-0 pass (0x001683dc-0x001683e8);
2. `DESKTOP.PRG`'s load;
3. `ccAllSoundOff` (0x0016847c);
4. the pass at phase 2 (`ccEnableThEvent(2)`, 0x00168488);
5. `ccSndChangeData` with the desktop's bank (0x001684c4).

The name entry runs in event 1's phase-0 pass, on the title's ports.

The port pushed `AllSoundOff` as the setup was entered, before the phase-0
pass. Now it is pushed as that pass ends, before `enable(2)`. A desktop
entered without the scripts still gets it at once.

## Tests

- `the_name_entry_is_heard`: power-on, the title, New Game (every event
  routed), the name entry's presses. Each sound asked for (18, 6, 4, 7) must
  be heard over the 12 frames after it. It fails with the old order ("sound
  [18] asked for, none heard") and passes with the new one.
- The player's log, replayed: every sound of the setup heard, peaks from
  5,901 to 29,330.

The piney-game suite passes (202).

**Still unknown:** the other session tests that reach the desktop through
`new_game` still drop those frames' events. They test no sound, so nothing
else rests on it.
