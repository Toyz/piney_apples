---
number: 433
title: Cubia's third fight: Quarantine's ccBossKyvia03 and its magical ball
date: 2026-10-22
area: battle, render, world, test, tooling
files: crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/magic_ball.rs, crates/piney-battle/src/boss/kyvia/thunder.rs, crates/piney-effect/src/kyvia.rs, crates/piney-world/src/combat/boss.rs, crates/piney-gen/src/manifest.rs, crates/piney-game/src/session/tests/kyvia_third.rs, tools/test_battle_kyvia_rs.py, tools/test_boss_effect_rs.py, docs/engine/boss-kyvia-third.md
started: 2026-10-22T20:49:02-06:00
took: 58m
---

# 433. Cubia's third fight: Quarantine's ccBossKyvia03 and its magical ball

The port had Kyvia's first two fights. Quarantine's third,
`ccBossKyvia03` (`bossFunc` code 14, event 308 on field 11's disc), fell
back to Skeith's code. It is now the second fight's code with its own
numbers, a third stage and a fifth attack, the magical ball, each checked
frame by frame against Quarantine's code.

## Finding the code

Quarantine's executable names none of ccBossKyvia03's functions but its
thread and constructor (`ccThKyvia03` 0x003d0a60, `__ct__` 0x003d0b30).
Mutation's carries the class (unused) with its names. The vtable the
constructor stores (0x00277f00) gives the virtual ones (`Main`, `Think`,
`Action`, `Slave`), and each named function's calls, in order against
Mutation's copy, give the rest: 23 functions, all at the addresses the
DWARF order expects. docs/engine/boss-kyvia-third.md lists them.

Quarantine's layout is Infection's DWARF with the cinema's byte at
+0x29350 (as Outbreak's second fight); `ThunderGp` sits before `MaxHP`,
moving it and the next three 4 on; the constructor clears a word DWARF
leaves out at +0x293f4, which `SwitchActionPattern` counts.

## What the third fight does

The second fight's `Main`, `SwitchDeadCheck`, `SetBPos`, `Slave`,
`EntrySlave` and `CheckDmgSmokeEff`, with:

- the constructor: `bossTbl` row 13 (`ccGetBossParam(13)`, as the
  second's), `x03`, `ANM_ex03nut0`, `OBJ_dummy4_4`, three stages, the
  pitch 0.25, `CoreInit(3)`, `SwitchHp` a fifth (again at each rise);
- the attacks by `AtkPatMode` and stage (the arm, the thunder of kind 1
  or 2, the beams, the meteors); at the last stage's meteors the next mode
  is 4, the magical ball, while the core is below 70% of `MaxHP`
  (`(double)hp < MaxHP * 0.7`), else 1; the ball goes back to mode 2. At
  that stage's third mode the turn waits for the menu, the player held
  from its 30th frame;
- the arm's camera by stage (6000, 3000, 4500), skill 76;
- the beams 1300 up, `ccRandF(0.2)` for the eye's turn, the point sinking
  8 and the eye rising 12 a frame while it swings, eased to 2500; at its
  30th frame (counted before it is raised) the start raised 180; the
  quake (20, 30, 45) as they land;
- the meteors: the player held from the first frame, stream 108, no quake
  at 60, and the later fights now wait out an annihilation or a game over
  as Outbreak's second does (the port's second had missed it);
- the thunder: each kind's bolts its own (table in the docs), set each
  frame the attack waits; the storm's camera 3500 off turned 0.6, looking
  at the disc; a quake (10, 10, 20) at 30; SE 41 at the build-up's 50th
  frame and with some strikes; skills 47 and 48; the strike's sound twice
  in every kind; no SE 41 at the disc after the storm's bolt;
- the death: the smoke 1250 up and 720 back at x 0, sinking 0.55 more and
  rising 3; the first camera at 60, 650 back; the cries unplaced.

## The magical ball

`MagicBallAtk` (act 20) holds the player, sends the core away, pulls the
camera to 5500 and colours the blur, then makes
`ccBossEffMagicalBall(40, DiscPos, blur, KyviaMagicalBallDamage)`. The
ball grows 1.5 a frame to 40, then for 150 frames runs seven plasmas
along Catmull-Rom splines from points `SetSpPoint` drew at random (every
third but the first, at each loop's end, deals the core's boss skill 72
round the ball), then bursts: skill 73, the quake, the flash, the blur
swelling to 1.06 and fading, and at its 81st frame the callback that moves
the act on. The docs give each step.

`DrawPlasma`'s spline point is a local that the legs without a spline (0
and 4) do not write, so a frame's first such plasma reads what the stack
held. The ball's `Draw` is called by the manager's `Draw` (0x00365ac0) in
`ccThBossEffect`'s task; the quadword is the top of a frame under the
manager's. The manager's last call each frame is the cinema's `Draw`,
whose `ccScFade::SendPacket` (0x0015eaa0) saves the task's `s7` there
(`sq s7, 112(sp)`); nothing in the task changes `s7`. On frames where
`ccSeOnNote` runs first, its saved return address is there instead, a
denormal VU0 reads as 0. Either way the end stands at the ball, so the port
starts each frame's point at zeros, and both harnesses write the zeros
where the game's would be.

## The port

- piney-battle: `Fight::Third` with its numbers in each step;
  `third_pattern`; `kyvia::magic_ball` (`magic_ball_atk`, `MagicalBall`
  on a `BallHost`, `ball_damage`). Kyvia's effects run in
  `Boss::effects_pass`, which the world and the probe call before the
  blur's `Main` and the boss's `Main`, as the manager's `Draw` runs before
  the boss's task: the ball's blur writes are in that frame's `Main`, and
  the camera mode its callback sets is the camera's before the boss reads
  `ResetFlg`. Every boss's distances are already the volume's (worklog
  432).
- piney-gen: `kyvia03_anims`, `kyvia04_anims`, `kyvia04_anims_ex`,
  `kyvia_skills_various`, `kyvia_gomora_lists_4` and
  `kyvia_gomora_lists_3`, read from `ListStepUp`'s third address build
  (Mutation on; Infection's has no case 3).
- piney-world: code 14 (`KYVIA_THIRD`, `x03`'s `CMP_trallex03`).
- piney-effect: `kyvia::Ball` draws the ball's `EffAnm` and publishes its
  plasmas' places; the rows' lives; `Ball::alone` for the probe.
- piney-game: Quarantine's story (301-314) with starts at 308 and 312.

## Checks

- `PINEY_VOLUME=quarantine tools/test_battle_kyvia_rs.py` runs
  `ccBossKyvia03(3)` natively against battle_probe's `kyvia 3`, with
  Quarantine's unnamed functions by address and the ball's members
  compared each frame: 40 cases, 120,477 frames, every frame agreeing,
  reaching acts 0-6, 14 and 20, both thunders and the core's three deaths.
- `test_kyvia_effects` (tools/test_boss_effect_rs.py) on Quarantine runs
  balls alone against boss_probe's `ball`: 1,036 frames of draws, sounds,
  the flash, generators and `ccRand` agree.
- `session::tests::kyvia_third::kyvia_s_third_fight_to_its_end` plays
  event 308 from field 11's disc with the console's god, the core hurried
  and struck by Kite's blows (`EntryAffect`) each 300 frames while it can
  be hit (the party alone takes long), held at the last stage until the
  ball has come. 15,949 frames: acts 0, 1, 3-6, 14 and 20, the three
  stages, the ball drawn in 518 frames with its plasmas' generators
  following them in 240, the music in 13,051, the body gone at 15,110 and
  the event ended at 15,948. A Quarantine start at 308 needed the
  volume's story (301-314) in `start::story`; Quarantine's events clear
  `eventStatus[0]` in their first block, so the disc helper takes the
  status to start from.
- piney-battle, piney-world and piney-game's suite (four threads),
  clippy, fmt, `piney-gen gen --check`.

Shots (`kyvia_third_shots`, /mnt/data/claude/scratch/a433; every frame
rendered): `kyvia3-13260-ball.png` the ball over the disc under its
cinema's name ("Chaos Gehenna"), `kyvia3-13380-ball.png` its blast and
the damage, the rest the fight every 600 frames.

**Still unknown:** nothing.
