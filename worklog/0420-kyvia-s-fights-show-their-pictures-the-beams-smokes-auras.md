---
number: 420
title: Kyvia's fights show their pictures: the beams, smokes, auras and their generators as the fight keeps them, the meteors falling with their trails and bursts, and Outbreak's thunderbolts
date: 2026-10-21
area: battle, render, test, tooling
files: crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-battle/src/boss/kyvia/thunder.rs, crates/piney-effect/src/kyvia.rs, crates/piney-effect/src/boss.rs, crates/piney-game/src/fx.rs, crates/piney-gen/src/manifest.rs, tools/test_battle_kyvia_rs.py, tools/test_boss_effect_rs.py, crates/piney-game/src/session/tests/kyvia_fx.rs, docs/engine/boss-kyvia.md, docs/engine/boss-kyvia-second.md
started: 2026-10-21T11:37:02-06:00
took: 40m
---

# 420. Kyvia's fights show their pictures: the beams, smokes, auras and their generators as the fight keeps them, the meteors falling with their trails and bursts, and Outbreak's thunderbolts

Worklog 411 left Kyvia's own pictures undrawn on both discs. Its rules
named every particle generator (`Out::KyviaParticles`, a table and row),
but nothing in the runtime started one. The meteors'
(`EffKind::Meteorite`) and Outbreak's thunderbolts' (`EffKind::Thunderbolt`)
`Create`s made no picture. A player saw the fight's attacks land with
nothing in the air: no light gathering, no beams, no smoke, no meteors,
no bolts, no auras about the core or the gomoras.

## What the game draws

Mutation's code (kyvia01.cpp, kyviacore.cpp, kyviagomora.cpp,
bosseff.cpp); Outbreak's copy is the same, its second fight's layout 0x10
further on. Every site is in docs/engine/boss-kyvia.md, "The pictures":

- **The generators.** About thirty `new ccParticleGenerator` sites over
  five tables: `KyviaGenerator` (28 rows on Infection, 30 from Mutation:
  the symbol's DWARF size is Infection's), `KyviaDmgGenerator`,
  `CoreGenerator`, `GomoraGenerator` and `GomoraAuraGenerator`, each with
  its force fields. Each site has its own rows and force fields (row 5
  twice, with ff 1 and ff 8). It either stands at a place or follows one
  (`syncPos`, `syncPosType` 0):
  - the core's place, with the auras' offset (0, 0, 250);
  - `DedDmgVec`;
  - a beam's head or tail (`LBpos`, `LBposSub`);
  - a gomora's `AuraPos`.
  Some sites keep the pointer, so the fight can kill it later (the beams'
  ends before their bursts, the smokes at the death, a grown aura's last).
  The core pauses its other attribute's aura (+5 bit 0, `pauseFlag`) and
  frees it again. A gomora's attack clears its pointer without a kill.
  Kyvia 02's death smoke and the thunder's member smoke have distSW off.
- **The meteors** (`ccBossEffMeteoriteMissile` 0x0047aa90, `Draw`
  0x0047b330):
  - Each meteor is particle's `ANM_x300`, drawn at transparency 0.5 and
    scale 3 and never stepped (its first frame throughout).
  - Two trails follow each meteor's place: `MissileSmokeGenerator` 6 (5)
    and 7 (6).
  - At each landing they are deleted (`ParticleDelete`) and SE 35 plays.
  - The first's landing starts `BurstSmokeGenerator` 8, 9 and 11 and
    throws `effSmokeRock(now, rot, 75, 0, 20, -1)`, facing away from
    camera 2.
- **The bolts** (`ccBossEffThunderbolt`, OUT 0x00477410, `Draw`
  0x004778b0):
  - Each segment is particle's `CMP_x012` on `CLT_x012c1` (mode 1),
    drawn with the stroke's own matrix.
  - On its first `Draw` a disc bolt writes `MissileSmokeGenerator` row 8
    (`gLife` its time, `gRadius` max(350, range + 100)) and starts it.
  - At its end it starts `BurstSmokeGenerator` 10.
  - `ThunderboltAtk` writes row 17 (`gLife` the thunder's time, `gRadius`
    50, `pGenRate` 20) before each member's smoke.

## In the port

- piney-battle `kyvia`:
  - `Gen` names each start by its maker, and `rows()` gives its table,
    row, force fields and distSW.
  - `Place`/`Follow` say where it stands, `Held` who keeps it.
  - `Out::KyviaGen` (`GenOp::Kill`, `Pause`, `Forget`) is emitted where
    the game kills, pauses or clears a kept generator: the beams, the
    smokes, the core's auras, a gomora's aura.
  - `Meteorite::make` and `fly` (with `MeteorCall`) and
    `thunder::Bolt::new` and `stroke` are pure steps on a `ccRand`
    source, so a manager can run them alone.
- piney-effect `kyvia`:
  - `start` and `gen_op` make and keep the fight's generators, the rows
    the game writes applied to the copy (`Tables::generator_edited`).
  - `Meteorite` and `Bolt` are drawn after the boss's task from the
    rules' flights (`BossEffects::sync_kyvia`), or run alone
    (`Make::MeteoriteAlone`, `BoltAlone`).
- piney-game:
  - `fx.rs` starts them and makes the meteors' and bolts' pictures at
    their `Create`.
  - `kyvia_shows` publishes the followed places as anchors each frame.
- piney-gen: the effect group's `boss_kyvia`, `boss_kyvia_ff`,
  `boss_kyvia_dmg(_ff)`, `boss_core(_ff)`, `boss_gomora(_ff)` and
  `boss_gomora_aura(_ff)`, with `KyviaGenerator`'s rows counted to the
  next table. Data version 33.
- The session tests' Kyvia disc (`event_108_disc`) was a generated
  field: area 47's `WORLD_MAN` with its own event number, so the disc
  (`EVENTAREAB8`) was never built and `DiscPos` was the origin. It is
  now `ev_area_number(9)`, as for Gorre's arena.
  `event_108_ends_with_kyvia` still ends the event.

## Checks

- `tools/test_battle_kyvia_rs.py` now records each start in full from
  the game: the force fields' rows, distSW, the place or what it follows
  (by member) and its offset, and who keeps it. It also records each
  `ParticleKill` of a kept generator (by member), the kept pointers
  (the body's light and beams, a gomora's aura) and the core's auras'
  `pauseFlag`. battle_probe prints the same from the rules' outs, its kills
  only for generators it keeps, as the game's pointers are.
  - Mutation: bulk 120 (161,184 frames) and 60 (84,657) agree.
  - Outbreak's second fight: 60 (180,064) and 40 (129,363) agree, the three
    thunders reached.
  - Outbreak's copy of the first: 30 (38,224) agree.
- `tools/test_boss_effect_rs.py`'s `test_kyvia_effects` runs the game's
  `ccBossEffMeteoriteMissileCreate` (pos, range 500, seven, no callback,
  no camera; camera 2 the test's camera) and, on Outbreak,
  `ccBossEffThunderboltCreate` for each kind's disc bolt (with the
  caller's offset) and a member's. Each runs with the manager's pass
  frame by frame against boss_probe's `meteorite` and `bolt`: the draws
  (the fireballs' `ccAnm` and frame, the segments' clump, matrix and
  palette), the generators (rows, force fields, flags, place, what they
  follow: the meteors' `now`), kills and deletes, the rocks' effect
  slots, the sounds, `rand()` and `genrand()`.
  - Mutation: 4 cases, 472 frames, agree.
  - Outbreak: 16 cases, 1,696 frames, agree.
  - Skeith's, Fidchell's and Innis's cases still agree.
- `session::tests::kyvia_fx`:
  - `kyvia_shows_its_meteors_beams_and_auras` (Mutation's event 108 on
    the disc, the core's time cut short so the body strikes in turn):
    `ANM_x300` drawn 384 frames, generators following the beams'
    places 1,164 frames, the auras on the core throughout.
  - `kyvia_s_second_fight_shows_its_thunderbolts` (Outbreak's event 211
    on field 10's disc): `CMP_x012` drawn 3,458 frames, the thunder (act
    6) reached.
  - Before, the fight started none of its generators, and `boss_make`
    made no picture for either effect.
- piney-battle, piney-effect, piney-world tests, piney-game's suite (four
  threads), clippy, fmt, `piney-gen gen --check`.

Shots (`kyvia_shots`, /mnt/data/claude/scratch/a420):

- `kyvia-mut-00702-act4.png`: the beams leaving with their light and
  smoke.
- `kyvia-mut-01170-act5.png`: a meteor's fall and burst over the disc.
- `kyvia-out-00702-act6.png`: the thunder's bolts on Kite and the disc.

**Still unknown:** the gomoras' pushes, the pad's sway and the arm's picture (docs/engine/boss-kyvia.md, worklog 272) are other parts of the fight than its pictures and stay open; the blur (ccBossBlur) is drawn for no boss (worklog 117)
