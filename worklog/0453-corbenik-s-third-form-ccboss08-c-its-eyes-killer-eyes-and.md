---
number: 453
title: Corbenik's third form (ccBoss08_c), its eyes, killer eyes and lasers are ported from Quarantine's code and agree with it frame by frame
date: 2026-10-24
area: battle, test, tooling
files: crates/piney-battle/src/boss/corbenik/third.rs, crates/piney-battle/src/boss/corbenik/eye.rs, crates/piney-battle/src/boss/corbenik/laser.rs, crates/piney-battle/src/boss/corbenik.rs, crates/piney-battle/src/boss/corbenik/eff.rs, crates/piney-battle/src/boss/corbenik/first.rs, crates/piney-battle/src/boss/corbenik/second.rs, crates/piney-battle/src/boss/corbenik/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/geom.rs, crates/piney-battle/examples/battle_probe/corbenik_c.rs, crates/piney-battle/examples/battle_probe/corbenik.rs, crates/piney-gen/src/manifest.rs, crates/piney-world/src/foe.rs, crates/piney-world/src/combat/boss.rs, tools/test_battle_corbenik_c_rs.py, tools/test_battle_corbenik_rs.py, docs/engine/boss-corbenik.md
started: 2026-10-24T09:06:33-06:00
took: 1h 2m
---

# 453. Corbenik's third form (ccBoss08_c), its eyes, killer eyes and lasers are ported from Quarantine's code and agree with it frame by frame

Event 313's second fight plays the third form. `ccThBoss08` reads `saveData` byte 0x652f, and at 1 it runs `ccThBoss08c_Main`. That task builds `ccBoss08_c` (QUA gcmn 0x003b9070-0x003be910), which carries:

- sixteen eyes (`ccBoss08Eye`);
- three killer eyes (`ccBoss08KillerEye`);
- its own effects: the tera laser (`ccBossEffTeraLaser`, with a charge and that charge's `ccBossEffEnergyGrow2` sparks), the reflected laser (`ccBossEffRefrectionLaser`) and the eye birth.

The port is in `crates/piney-battle/src/boss/corbenik/third.rs`, `eye.rs` and `laser.rs`. docs/engine/boss-corbenik.md has the form.

## What it does

The body is `bossTbl` row 9. Its sixteen eyes ride `x81`'s `DMY_dummy1`, each on an offset of its own, and follow the body's acts. Its patterns are:

- the wave;
- the eye entry, which throws out the killer eyes;
- the laser: the eyes fly out to three rings round the target, then `16 / n` lasers bounce off 3-5 eyes into each of the n members alive;
- the tera laser, which falls from 3000 up onto the target;
- the data drain;
- the slave attacks, which call one killer eye's skill.

At half its HP the first blow there shows stream 88 and turns the table.

A killer eye (rows 45, 46 and 47) fades in, is thrown, bounces, then wanders and presses its member on its own table.

## What its code does that one would not guess

- The eyes' dummy is `ApplyMatrix(anm, dmyEye->pos)`: the body clip's world matrix as its last draw left it. `ccBoss::Draw` draws the clip only while its transparency (the camera's fade times `setTransparency`) is 0.05 or more. The body's own eye centre offset is therefore a frame behind, and its `Slave` reads the frame's draw.
- `ccBoss::Draw`'s camera fade is now in the rules (`Boss::draw_fade`, `CamView::fade`, `geom::camera_transparency` moved from piney-world). The third form reads it back: the eyes' fades start from the faded value.
- Quarantine's `ccBoss::ExecPatternIndex` (0x00360630) makes the base's word 7 the wander (act 8, not the 20 the first two forms' copies had), and any unknown word the neutral with forbid 1. All three forms now share one base (`corbenik::base_exec_pattern_index`). The first two forms' tables never reach the difference.
- When a 17 (the eye entry) is passed over because a killer eye is still in, the next unknown word goes to the base from the 17's own index. The base reads the 17 again, takes the neutral, and the next call starts after it.
- From the body's first pause of its slaves (laser, tera laser, drain, slave attack), a killer eye's next pattern is 2 -1, which holds it in its neutral for good. After that it acts only when a slave attack calls it.
- `ResetRefrectLaserPos` never steps past the first eye: only eye 0 goes back to where it was.
- A reflected laser runs 200 a draw.
- The QUA-only mode-0 table is Infection's `boss08CNormalActTbl`. Quarantine's table of that name is a new one after it. The generator reads it from `SetPatternMode`'s last `lui`/`addiu`, and the killer eyes' three tables by Quarantine's own names, because the carry put them where Infection's shorter tables were.

## Checked

`tools/test_battle_corbenik_c_rs.py` runs `ccBoss08_c` natively:

- its eyes, killer eyes, lasers, the eye birth and the sparks run natively;
- `ccBoss::Draw` and `SetMatrix_PosRotZYX` run natively, with a stand-in active camera at the case's eye, sometimes far enough to fade;
- the WaveShock and the rings are stand-ins.

Seven cases (seeds 10300-10306), then 30 (seeds 10400-10429, 92,944 frames) and 24 more with the camera's fade (seeds 10500-10523) agree. They reach the body's acts 0-6, 8, 14, 20 and 21 and the killer eyes' 0, 3, 4, 8 and 14.

The harness found three port bugs, now fixed:

- the laser's SE 61;
- the reflected laser's speed;
- the base's word 7.

The stand-in `SetAnm` now keeps the chunk at +0x94 as the game's does, because a killer eye's clip restarts from it. The first and second forms' harnesses still agree (10 cases each).

**Still unknown:** the arena clump the lasers switch off (`eventArea` +0x2bc), and the world's side of all three forms: the parts as actors, the pictures, both first forms in one task, and `gen_alive` on the particle system.
