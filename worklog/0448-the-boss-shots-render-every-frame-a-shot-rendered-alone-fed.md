---
number: 448
title: The boss shots render every frame: a shot rendered alone fed the blur the last shot's picture, so Tarvos's target window wore Balmung's from 277 frames before; Tarvos's 30000 HP reads 0000 through cheatHpStr, and the phantom's unwritten view is drawn for one frame
date: 2026-10-24
area: test, render, battle
files: crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/arena.rs, crates/piney-game/src/session/tests/fidchell.rs, crates/piney-game/src/session/tests/magus_fx.rs, crates/piney-game/src/session/tests/kyvia_fx.rs, docs/engine/boss-tarvos.md
started: 2026-10-24T05:25:18-06:00
took: 40m
---

# 448. The boss shots render every frame: a shot rendered alone fed the blur the last shot's picture, so Tarvos's target window wore Balmung's from 277 frames before; Tarvos's 30000 HP reads 0000 through cheatHpStr, and the phantom's unwritten view is drawn for one frame

Worklog 447's shot at frame 14,677 (`tarvos-14677-act25.png`) showed
Tarvos's target window with a second, fainter window under it: HP digits
over its own, and an SP row of 235/235, Balmung's SP. That looked like two
target windows in one frame, or the window not cleared when the target
changed.

## What it was

The port's HUD draws one target window a frame. `ccMenuCtrl::Disp` takes
one target (the command target, else the previous one), and its SP row is
only for a PC (`type & 7`). Tracing the run (`PINEY_SHOTS_AT=14640-14700`,
which shoots every frame of the span and prints the command target) gave
these facts:
- the command target was Tarvos from frame 14,640 through 14,700;
- frames 14,644-14,659 drew Tarvos's window alone;
- the faint window came up at 14,660 and was gone by 14,670.

The source was the shot helper. `ccBossBlur` draws a `ccBufferSampling`
feedback of the last displayed frame, `PreviousFrame`, which holds that
frame's HUD. The headless renderer drew only the frames it shot, so its
"last frame" was the previous shot: frame 14,400, where the pilot had
Balmung's window up for a heal. That stale picture was fed back into
14,660 and again into 14,677.

The game draws every frame. A target change there leaves a fading echo of
the old window for a few frames (the blur's colour 0x48: about 0.56 a
frame), never one from 277 frames before. So the game can show a doubled
window only briefly, after a real change. The shot above was not the
game's, and not the port's runtime either. No field-UI case was needed:
the target window's numbers and rows are already checked against the
game's code (`tools/test_fieldui_rs.py`, `test_target`).

## The fix

`session::tests::Shots` renders every frame and writes only the frames
asked for, as the blur tests already did. These now use it:
- the arena's god shots (Macha, Tarvos);
- `fidchell_shots`;
- `magus_flash_shots` and `magus_shots`;
- the Kyvia push, gomora and fight shots.

With it, frame 14,677 shows Tarvos's window alone. Only the changing HP
digit echoes for a frame or two, as the game's feedback does. The Tarvos
run takes 403 s instead of 70.

## Tarvos's HP as shown

8888 is not Tarvos's maximum. `bossTbl` rows 6, 42 and 43 all hold maxHP
30000. From 10,000 the window shows each value's last four digits, each
digit through `cheatHpStr` ("01*#4>67%:"), so 30000 reads `0000` and
27,760 reads `7760`. The 8888 was the stale window's digits over the
zeros.

## The phantom's view

The view `ccBossEffPhantom`'s constructor never writes (worklog 447) is
drawn. In the frame the phantom is made, the constructor's `SetMode(5)`
and `SetFreeCamPosView` reach the boss camera. After `Move`, `CamMain`
copies the locked eye and view to camera 2. The phantom's first `Draw`,
which sets the view anew, comes only in the next frame's manager pass.

So one frame looks from the target's place toward (0, 0, z): the
pointer's float is under 1e-37 and its sign word 0, and `z` is the upper
half's first word. The value stays open: it is what the kernel's
`StartThread` left in the boss thread's first context, and no disc code
writes it.

**Still unknown:** the upper half of `$s1` in the boss's thread (taken
as 0), now known to show for one frame.
