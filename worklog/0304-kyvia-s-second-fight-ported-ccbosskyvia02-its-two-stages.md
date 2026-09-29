---
number: 304
title: Kyvia's second fight ported: ccBossKyvia02, its two stages and thunderbolts; event 211 done
date: 2026-09-29
area: battle, volumes
files: crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/thunder.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-battle/src/boss/kyvia/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/combat.rs, crates/piney-data/src/pack.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/fx.rs, tools/test_battle_kyvia_rs.py, docs/engine/boss-kyvia-second.md, docs/engine/boss-kyvia.md, plans/outbreak-story.md
---

# 304. Kyvia's second fight ported: ccBossKyvia02, its two stages and thunderbolts; event 211 done

Outbreak's event 211 stopped at field 10's `entry type=7 code=13`. OUT
`bossFunc` (gcmn 0x00616d60) entry 13 is 0x004d5870, a task that makes
`new ccBossKyvia02(2)` (0x29740 bytes, the string "Kyvia02") and runs its
`Main`. So row 13 is Kyvia's second fight, not another boss: the plan's
"Cubia, the second Kyvia" was half right. Cubia is the English
`bossTbl`'s name for Kyvia (rows 12 and 13, "Cubia Core" for 32-39).

## Finding the code

OUT's sidecar names only the constructor. The class's functions follow
Infection's order (its syms list `ccBossKyvia02`'s 23); the prologues
between 0x004d5830 and 0x004de150, the jal targets into them and the
vtable (0x00385560: +0x18, +0x1c Main 0x004d7300, +0x8c, +0x90, +0x94)
place each, and the same for OUT's `ccBossKyvia01` (Main 0x004d0780).
The disc's `Move` and `IsMove` are 0x00419c80 and 0x00419d10 (called from
`CheckDiscMove`), `KyviaGenerator` 0x0061d0d0 (0x690 below
`KyviaDmgGenerator`, as in Mutation).

Diffing OUT's `ccBossKyvia01` against OUT's `ccBossKyvia02` (the same
compiler) with member offsets mapped between the layouts gave the change
list: Main, Think's switch but act 6, SwitchDeadCheck, SetBPos, Slave,
EntrySlave and DeadKyviaVoice are the same code; the rest differ in
constants and a few steps; ThunderboltAtk, ThunderDmg and ThunderSound
are new, with `ccBossEffThunderbolt` (OUT gcmn 0x00477410-0x00477ef0).
OUT's layout is Infection's with a byte at +0x29350 (the cinema flag),
the next three pointers 4 on and the rest 0x10. Details are in
docs/engine/boss-kyvia-second.md.

## First, the first fight on Outbreak

The harness took a fight (`PINEY_KYVIA_FIGHT`) and Outbreak's unnamed
addresses. Outbreak's copy of the first fight agreed with the port on
its first three cases, so OUT's core and gomoras at level 1 are
Mutation's.

## The port

`kyvia::Fight` (First, Second) picks the numbers where the code
differs; `kyvia::new(cx, fight)`. `kyvia::thunder` holds act 6 and the
bolt, whose `Draw` draws `ccRand` each frame (a stroke: `ccRand`, two
`ccRandF(pi)` unused, the turn, three offsets; two more a segment), so
the kyvia manager pass runs it as it runs the meteors. Found on the
way:

- The ride's camera looks at the disc itself in the second fight
  (`SetFreeCamPosView(eye, DiscPos)`), 150 over it in the first.
- `DmgGp[k]` past four reads `DeadGp` and then `CamRotX`: with
  `SwitchHp` a seventh, the fifth smoke fills `DeadGp` and there is no
  sixth.
- `SetCoreBaseParam` at level 2 adds 3000 to both rows' `maxHP` each
  call while the core is dead. The port added it once, to the HP only,
  so a later change of attribute would read the row without it. Now
  `Core::row_hp` carries it, and the harness restores `bossTbl` for each
  case (the game's table had kept the first case's 3000).
- The gomoras' lists by level and death: `AllGomoraList_2` (two steps)
  in the manifest (`kyvia_gomora_lists_2`, also `kyvia02_anims`),
  `ListStepUp(n)` taking step n where it is not null. `DATA_VERSION`
  19 to 20.

## Checks

`PINEY_VOLUME=outbreak python3 tools/test_battle_kyvia_rs.py bulk`: the
first case after the port agreed; the next failed only on `bossTbl`'s
kept HP (above). After that, 100 cases (320,004 frames, seed 9500) and,
on the final code, 30 more (85,575 frames, seed 9600) agree: the body's
acts 0-6 and 14, thunders 0-2, the core's two deaths. Mutation's first
fight, 25 cases, still agrees. Cases of the second fight run up to 6000
frames with hits to 1200 so the core dies twice.

`PINEY_SURVEY_ONLY=211 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000
cargo test --release -p piney-game outbreak_story_survey -- --ignored
--nocapture`: `story 211: done`. The fight starts near frame 11,650; the
first core (2000 HP) is down near 16,500, the second (5000) near 22,000.
The pilot needed nothing new: Kyvia's parts were already its focus.

**Still unknown:** kind 2's camera reads `cameraGetPos`/`Rot`/`View` of
`camID`, taken as the boss camera's; the bolts' picture (`CMP_x012` by
`Bolt::segments`) is not drawn; whether a retried fight meets `bossTbl`'s
rows as the disc has them or with the core's added HP.
