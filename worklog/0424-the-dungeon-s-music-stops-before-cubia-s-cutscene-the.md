---
number: 424
title: The dungeon's music stops before Cubia's cutscene: the session's scene change sends ccSetupGameCtrl's ccSoundFadeOut, and the next set-up its ccAllSoundOff (or ccEvVoiceStop), before the event's phase-2 streams (#71)
date: 2026-10-22
area: audio, world, test
files: crates/piney-game/src/session.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/area.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/kyvia_stage.rs, docs/engine/field-game.md
started: 2026-10-22T16:00:32-06:00
took: 12m
---

# 424. The dungeon's music stops before Cubia's cutscene: the session's scene change sends ccSetupGameCtrl's ccSoundFadeOut, and the next set-up its ccAllSoundOff (or ccEvVoiceStop), before the event's phase-2 streams (#71)

Issue #71 (ThreePendant, build 20db4ca): before Cubia's first fight the
dungeon's music played on under the cutscene's own.

## The cause

`ccSetupGameCtrl` (MUT main 0x00168ae0, the same on every volume) makes
both sound calls of a scene change by `CheckSceneReplace`:
`ccSoundFadeOut` at its start for a new scene (0x00168bb4), and after
its ten-frame fade `ccAllSoundOff` for a new scene (0x00168d0c), else
`ccEvVoiceStop` (0x00168d1c). Only then does the event task's phase-2
pass run, where event 108 plays `stream 30` (`str0932`) on the way into
field 9.

In the port the session draws that fade over the old scene (`leaving`)
and builds the next one with `faded` set, which starts its set-up held
black. The session sent `GameInterrupt` only, and `FieldWorld::enter`
with `faded` skipped the fade out and with it both calls. So nothing
stopped the dungeon's sequencer until the field's `ccSndSQLoad`, after
the 586-frame stream. Every change the session makes into a field or a
dungeon was affected (a town's set-up sent both); most have no
phase-2 stream, so the bank load cut the music a frame or two late.
Skeith's arena (event 30's streams 14 and 17 from area 27's dungeon) had
the same fault.

## The fix

- `Session::leave` sends `GameInterrupt` and, when the change replaces
  the scene (`Scene::replaced_by`, `CheckSceneReplace` on the scene the
  change makes; a gate's words always do), `SoundFadeOut`.
- `field_world::sound_off`: the set-up's call after the fade, held or
  drawn, `AllSoundOff` for a new scene and `EvVoiceStop` (a new
  `Request`, routed as `Event::VoiceStop`) for a room or floor of the
  same dungeon. The town's `skip_fade_out` drops the `SoundFadeOut` its
  constructor queued, as the session sent it.

## Checks

`kyvia_stage::area_47s_music_stops_before_event_108s_stream` routes every
frame's events into a headless engine through `crate::handle` from the
session's start in area 47's dungeon: the dungeon's music plays, the
door's change sends `SoundFadeOut` and then `AllSoundOff`, and no
sequencer plays in the 586 frames of `str0932`. Before the fix: neither
event, and music in all 586 frames.

**Still unknown:** nothing
