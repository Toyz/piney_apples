---
number: 158
title: The game over: ccThGameOver, its noise and the television
date: 2026-09-26
area: world
files: crates/piney-world/src/gameover.rs, crates/piney-game/src/gameover.rs, crates/piney-game/src/area.rs, crates/piney-world/src/field_world.rs, crates/piney-audio/src/driver.rs
---

# 158. The game over: ccThGameOver, its noise and the television

Before this, a wiped party (or `compulsionGameOver`) made `ccThGameCtrl`
answer `Ctrl::GameOver` once, and the field just froze. Now the port
runs the rest, as `docs/engine/field-walk.md` "The game over" and
"`ccThGameOver` and its noise" set out.

The signal frame: the pause, the menu forbidden, the message closed. The
next frame: `ccSndGameOver` (the sound's `gameInterrupt` and a 20-step
`sq_fade` of every sequence, `Driver::game_over`) and `ccThGameOver`
started. It waits 181 frames on the frozen field, then runs
`ccGameOverNoise::Main` each frame. The noise has three phases: 51 frames
of short sampling bursts, 41 of sampling and inversion, then one long
burst while the TV switches off. The switch-off halves the picture's
height to a line (sound 90, the flash), holds three frames, then takes a
quarter of the width at a time down to a dot. `InitData` leaves the
flash armed, so it fires twice: at the second squeeze and at the line.
Only `StretchTV_Y` sends the fader's packet, so the flash shows only
while the picture squeezes.

Once `Main` answers 0, the field's tasks go. After 2 + 40 frames of
black, the "over" file's `ANM_xdggame0` (GAME OVER in red) plays on layer
129 to its end, and `ccOpenGameOverMenu`'s `ChangeRequest(1, 7)` resets
to the title.

In the port, `piney_world::gameover::GameOverNoise` is the noise
(`ccRand` for its odds, newlib's `rand` for `ccNoiz`'s bands, both
through `FieldWorld::with_rands`). `piney-game`'s `GameOverTask` is the
task. `AreaMode` starts the task on `FieldWorld::take_game_over`, runs
it before the frame is finished, and applies the view frame the noise
leaves with `gameover::squeeze`: every command of the frame (prim
vertices, model matrices and scissors) is moved into the band. Once the
field is gone, `frame_alone` draws the black frames and the clip.

Checks: `the_television_switches_off` (with `ccRand` all 0: 51 + 41
frames, the flashes at the 94th and 101st, w = h = 1, the noise at phase
4), `a_squeeze_maps_the_frame_into_the_band`, and the session test
`a_game_over_ends_at_the_title`, which forces the game over in story
area 14's field. Its sound is at frame 1, the TV is off at 282 and the
title comes at 470. `game_over_shots` (ignored) saves the noise, the
squeeze, the line and GAME OVER. They look as described.

**Still unknown:** The party under manual control (`ManualModeAI`,
`SetRemoteCmd`) and `ccClearConditionAllEnemy` are not run. Nothing
after the freeze reads them, so the player sees no difference.
`DUNGEON.fog` is not zeroed at the TV. The squeeze moves the whole
finished frame, not only what is drawn through sysLayer's view. The
noise's frame-by-frame draws were not checked against an eemu run of
`ccGameOverNoise`: the phases and the TV were read from the disassembly,
and only their counts are tested.
