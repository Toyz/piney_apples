---
number: 434
title: Cubia's fourth fight: Quarantine's ccBossKyvia04, its places, EX turn, hell's express and rebirth
date: 2026-10-22
area: battle, render, world, test, tooling
files: crates/piney-battle/src/boss/kyvia/fourth.rs, crates/piney-battle/src/boss/kyvia/hell.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-effect/src/kyvia.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/evarea_b8.rs, crates/piney-game/src/session/tests/kyvia_fourth.rs, tools/test_battle_kyvia_rs.py, tools/test_boss_effect_rs.py, docs/engine/boss-kyvia-fourth.md
started: 2026-10-22T21:47:15-06:00
took: 1h 34m
---

# 434. Cubia's fourth fight: Quarantine's ccBossKyvia04, its places, EX turn, hell's express and rebirth


The port fell back to Skeith's code for `bossFunc` code 15, Cubia's
fourth fight: Quarantine's `ccBossKyvia04` (event 312, field 12's disc).
It is now ported from Quarantine's code: the four places and their gates,
the body's EX turn, hell's express, the rebirth and the death. Each is
checked frame by frame against that code, and the event is played from
the fight to the next scene.

## Finding the code

Quarantine names only `ccThKyvia04` (0x003d9b70) and the constructor
(0x003d9c40). The vtable at 0x00277fa0 gives `Main`, `Think`, `Action`
and `Slave`. Each other function's calls, in order against Mutation's
named copy, give the rest: 41 functions and the hell's express class
(`HellSpecialExpress`: `Clear`, `Init`, `HellStartScene`, `Draw`,
`HellSpecialExpressCreate`). docs/engine/boss-kyvia-fourth.md lists them.
The layout is Infection's DWARF as it stands (0x29890), plus the
rebirth's two words at +0x29614 and +0x29618. `Kyvia04AnmTbl` and
`_EX` have 25 entries from Mutation on (act 24's `ANM_ex04dmg0`).

## What the fourth fight does

- `Think`: on the first frame, the places' lights and walls. `DiscPos`
  is Kite's place. While the body waits, Kite within 250 of the next
  place starts the count: at 1 the flash and the player held, at 20 the
  party gathered there (`TogetherCharPos`), at 30 the core risen at that
  place and the place behind closed. As a core falls, the next place's
  gate opens: field 12's `body[n - 1]` taken out of the hit list
  (0x0030c620) and put back at its tail on closing (0x0030c690).
- The places' turns: the arm (44) or the ball (42), the thunder of the
  place's kind (50), then the thunder, the ball or hell's express (62).
- At the last place `KyviaScene` wakes the body (the rumble, the camera
  rising before it, `ANM_ex04act0`). `KyviaExOn` then starts the EX turn:
  row 15's core in its EX mode (state 10, at (0, -700, 900)), its gomoras
  at (190, -2154, 950).
- The EX turn's attacks: before the rebirth the thunder and the skills
  (`Skill_Atk`, five of `Skill_KYVIA_VARIOUS` on `KyviaRndTarget`'s
  pick); after it the ball, the skills, the beams, hell's express and
  `BIGMegidFlame` (stream 111, boss skill 55 on Kite).
- Hell's express: three souls on a spline from high over the place,
  the camera on the first, the burst (`ANM_ex31lhit` with
  `CLT_exx004c1`), `Kyvia_HellE_Damage` (80% of the target's HP, 40%
  of the others', at least 400).
- `DeadCheck`: the core's fourth death in the EX turn is the rebirth
  (act 24: the light, the cries, stream 85, row 15's `maxHP` written 9999,
  `KyviaExOn` again). Its next death is the body's (act 14: the boss's
  gauge on Quarantine's `dummyTarget`, filling with the core's HP, the
  cries, the camera, `DeadKyviaRipus`'s lights).

## Two words the code never writes

`Skill_Atk`'s eye vector writes only y and z. Its x word is
`KyviaSwitchActionPattern`'s `s0` (2), saved by `ccBoss::ChangeAction`'s
prologue (0x00360ea8). 2 is a denormal, which EE and VU0 read as 0. In the
EX ball the same word is the work pointer that `__ieee754_atan2f`
(0x001225f8) saves for `ccGetDirc`: a heap address, a float near 1e-38.
It shows only when `ccRandF(0.15)` gives exactly 0 with the core at x 0,
and no view matrix keeps a 1e-38. The port takes 0, and the harness
writes 0 there in that case. With a poisoned stack, every other word the
fight reads is written by its code first.

## The rows across fights

The rebirth's 9999 and the cores' added `maxHP` live in `bossTbl`
(gcmn's data). A game over is `ChangeRequest(1, 7)`, the soft reset to
the title, and DEMO.PRG takes the overlay window. Play goes on only
through `ChangeRequest(5)`, which loads GCMN.PRG from the disc again. So a
retried fight meets the disc's rows. Between events 308 and 312, event
311's blocks run on the desktop (`game_status` 2, mails 77 and 79, area
108 opened), so GCMN.PRG is loaded again there too: this fight's third
place meets rows 36 and 37 as the disc has them. The port's
`Core::row_hp` and `row15_hp`, kept on the fight, are exact. This answers
the open question in the second and third fights' docs.

## The port

- piney-battle: `Fight::Fourth` (code 15, row 15). `kyvia::fourth` has
  the places, gates, scene, EX turn, attacks, rebirth and death.
  `kyvia::hell` has `Hell` on a `HellHost`. The core and the gomoras gain
  their EX modes (`KyviaExModeOn`, `EndFadeEX`, `ActionStart_K`,
  `CheckCountGomoraEX`), and the thunder and the ball take the fourth
  fight's numbers.
- piney-gen: `kyvia04_anims` and `_ex` to 25 entries from Mutation on.
- piney-world: code 15 (`CMP_trallex04`), `Out::CamEye`,
  `Out::FieldBodyHit` (`DiscArea::body_hit`), the fade elements the ball
  waits on, `Out::DummyTarget`.
- piney-effect: `kyvia::Hell` (the burst and its palette swap, the souls'
  anchors), `Hell::alone`, and boss_probe's `hell`.
- piney-game: hell's express's picture, the menu's `dummyTarget`, and
  `session::tests::kyvia_fourth`.
- tools: eemu's `call` takes `sp` (the boss task's stack).
  test_battle_kyvia_rs.py gains fight 4: Kite walked to each place, the
  gates, hell's express's members, the menu's seventh entry, the fade
  elements, and the stack poison.

## Checks

- `PINEY_VOLUME=quarantine PINEY_KYVIA_FIGHT=4 tools/test_battle_kyvia_rs.py`
  runs `ccBossKyvia04(4)` natively against battle_probe's `kyvia 4`. The
  unit cases and `bulk 25` (163,336 frames) agree on every frame. They
  reach acts 0-4, 6, 14 and 20-24, the thunders and the gomoras.
- `test_kyvia_effects` (tools/test_boss_effect_rs.py) runs hell's express
  alone at each place (`KyviaLV` 0-3) against boss_probe's `hell`. The
  Create (generators, ccRand, the palette swap) and 3,224 frames of draws,
  sounds, generators following the souls, kills and its end all agree.
- `session::tests::kyvia_fourth::kyvia_s_fourth_fight_to_its_end` plays
  event 312 with the console's god. While the body waits, Kite is set at
  each place; the core is hurried and struck by Kite's blows every 300
  frames. In 26,925 frames it reaches the four places, the EX turn, acts
  0-4, 6, 14 and 20-24, hell's express drawn in 176 frames, the music in
  15,182, the body gone at 23,004, and the event on to Lia Fail (block 17)
  at 26,924.
- The other volumes' Kyvia bulks and units, the third fight's, every
  volume's boss effects, eemu_rs, piney-battle, piney-world, piney-effect
  and piney-game's suite (four threads), clippy, fmt, and
  `piney-gen gen --check`.

Shots (`kyvia_fourth_shots`, /mnt/data/claude/scratch/a434; every frame
rendered): `kyvia4-16050-hell.png` and `kyvia4-20550-hell.png` show hell's
express bursting under its name ("Sodom's Curse"). `kyvia4-12000.png` is
the second place's fight, `kyvia4-22800.png` the body's death with the
boss's gauge, and the rest the fight every 600 frames.

**Still unknown:** nothing.
