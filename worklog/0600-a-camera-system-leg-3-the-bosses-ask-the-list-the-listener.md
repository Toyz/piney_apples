---
number: 600
title: A camera system, leg 3: the bosses ask the list, the listener per sound, the ambient's unread camera values
date: 2026-11-15
area: engine, battle, world, audio, test
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/camera.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/stage.rs, crates/piney-game/src/area.rs, crates/piney-world/src/field/ambient.rs, crates/piney-world/src/sky/firefly.rs, tools/test_battle_boss_rs.py, tools/test_listener_rs.py
started: 2026-11-14T22:00:37-07:00
took: 4h 32m
---

# 600. A camera system, leg 3: the bosses ask the list, the listener per sound, the ambient's unread camera values

Leg 3 of the camera relay (plans/rust-cleanup.md section 2, "A camera
system"; branch `camera-system`, after worklog 599). It did step 5 (the
bosses ask the list), the listener per sound, the ambient's unread camera
values and step 7 (the cinema is `piney_world::combat::cinema`, the boss
effect manager's, not a camera). Step 6 is left (the relay's handoff has
it).

## The bosses ask the list (step 5)

A boss's rules wrote the cameras through deferred `Out`s the world applied
after the frame, and read a per-frame snapshot (`CamView`). Now they call
camera.cpp at the game's point. `boss::Cams` (the list, the shake and the
scheme) is lent to the task in `BossEnv`, the way its `ccRand` is. An
`Affect` gets it too: Gorre's brothers' reports end in `OffBossCamera`
inside their `Affect`. The boss camera's own calls (`SetMode`,
`SetFreeCamPosView`, `QuakeCam`, `CheckMoveCamera`, its members) are on
`BossBase::cam`. `CamView`, `CamFade` and the twenty-one camera `Out`s are
gone.

What the game settled:

- `ccBoss::Main` (INF gcmn 0x0045c270) calls `CamMain` (0x0045c52c) right
  after the virtual `Move`, while `bossCam` and `bossCamSW` hold. Most
  classes' Mains call `ccBoss::Main`. The own copies put it after their
  lock: `ccBoss02` (MUT 0x00496008), Kyvia 01-03 (0x004d4700), Kyvia 04
  (QUA 0x003dbc28). Gorre's comes after its `Move` (OUT 0x004aa514). It is
  a `Step::CamMain` in each def's order.
- `InitBossCamera(z, y)` (0x0045e750) is called inside each class's
  constructor, at its point, so the constructors make the camera and what
  follows writes it live. `CamDef` and the world's replay of constructor
  `Out`s are gone.
- `cmn_bossCam` is read only by four effects and `WaveFuncCB`; everything
  else is `this->bossCam`.
- A gomora's `QuakeCam` goes to `DammyBossCam` (+0x29400).
  `kyviaCore::CoreInit` copies the core's +0x293d8 there (MUT gcmn
  0x004f8614), and that is the fight's camera.

Differences the live camera exposed, each fixed:

- `ccBoss::Draw`'s fade read the camera from before the frame's `CamMain`.
- `cameraShake`'s `rand()` and `checkCameraShakeRange` came after the
  frame.
- StarDust's `cameraSetRot(rot, 3)` was dropped.
- Kyvia's fourth `ThunderboltAtk` picked its range wrongly. The game picks
  it by `DiscLV` (QUA gcmn 0x003e4960): 1800 past Kite at level 1, 2000 at
  2 and 3. The EX turn (0x003e4b70) takes 2000 at 1, 2 and 4. The port had
  1800 at level 3. The native `CamMain` in the harness found it at frame
  3223 of a fourth-fight case.

**The harnesses.** The eleven boss harnesses and battle_probe now share
one shape:

- The case lays camID, the scheme and tcam, bcam and ecam into the game's
  own CAMERAs.
- camera.cpp and the boss camera run natively, with the pad at rest.
- After the constructor, the case may put its draw camera into the active
  one and clear `bossCamSW`.
- Every frame compares camID, the three cameras, the shake's slots and the
  `ccBossCam`.

The stand-ins went with the old shape: the cams dicts, the scripted turn
and shake answers, and the FakeCam/Cams emulations. Tarvos's phantom keeps
one: its constructor hands `SetFreeCamPosView` a stack word it never
wrote, which both sides take as zeros. Gorre's `MaxRenge`, "the mechanism
not yet found" under the old zeroed camera, is the real constructor's
1000.

## The listener per sound

`ccSeOn3D` pans at the call (calcPan reads `activeCamPtr` then). The battle's
show stream now carries `Show::Ear` marks at three points:

- as its tasks start;
- after Kite's `cameraSet` (the end of `CameraPosSet`, before his notes);
- after the boss's task.

A boss's own 3D sounds each follow an `Out::Ear` of the list at the call.
piney-game pans each sound by the last mark before it
(`piney_world::combat::heard`). Before this it took the camera as the
frame ended for every sound.

tools/test_listener_rs.py runs test_bosscam_rs's game: `cameraMain`,
`ccPlayer::Main`, then `CamMain` with the boss stepping. Neither side's
Kite plays its clips' notes there, so a footstep is made where the note
would come, and the game's calcPan runs on the camera as it stands then.
The port's listener for the same sound gives the same calcPan on all four
volumes. On about 40% of the frames, the camera as the frame ends would
pan it otherwise (343 of 800 on Outbreak). With `heard` made to ignore the
marks, the harness fails at scheme 0 frame 5 (calcPan 4063233 against
4128768).

## The ambient's unread camera values

`WORLD::Init`'s snow and `Generate`'s fireflies read no camera. The calls
of SNOW's constructor (INF gcmn 0x00503650) and its `calcPos`/`calcPos2`
are `GetCenter`, `GetHeight`, `fieldrand` and the effect's set-up.
`SetBasePosition` (0x005b49f0) calls `GetHeight` alone. So `Env::at`,
which filled the eyes and the turn with the centre and zeros, goes: a
`Spot` (the centre and the heights) makes the flakes, and the field's
ground the roamers' bases. A `FIREFLY2`'s `Fly` holds the camera and
ground a type 1 roams by as `Option<Roam>`, since the arena's type 0 reads
none of them.

## Checks
On 1942027, from 2026-11-14 into 2026-11-15:

- harness, four volumes each: the twenty-three of leg 1 and
  test_listener_rs, 96 of 96.
- The eleven boss harnesses on their volumes (Skeith's on all four,
  Kyvia's fights on theirs): 15 of 18 pass. The other three are
  test_battle_kyvia_rs's test_core_wave on Outbreak, Quarantine and
  Quarantine's fourth fight ("the core never waved"), which fail the same
  way at d6840dc. Their frames all compare equal.
- golden shots: 4550 files, 0 differ against the base shot at b1a9d90
  (2026-11-14). Main, shot again on 2026-11-15, also shows 0 differ.
- battle pins (this worktree's probe at 1942027): 78 of 290 calls differ,
  and every one is a boss-frame request. The request now carries the
  cameras, so the pinned ones run out of tokens (rc 101). Every other call
  is byte-identical. The pins need recording again once this lands.
- crate tests 42 ok, 0 failed.
- piney-game suite, uncapped: 348 passed, 0 failed, 152 ignored.
- gen --check, clippy, fmt, cairns and docs are clean.
- test_cinema_rs (step 7's) passes on Infection. Elsewhere it fails at its
  first cinema 2, before the move too: its probe names cinema 2 as
  Infection's table does.

**Still unknown:** what OnThinkPhantom's $s1 holds as the phantom's unwritten view (both sides take zeros); and the NPC list's sounds after its own camera writes at 64, which step 6 marks
