---
number: 425
title: Cubia's attacks show their names: SetupSkillName reads x01 to x04 in fields 9 to 12 whatever the row says, so Legion's Reach, Ark Bullet and Megiddo Flame come from the field's file (#72)
date: 2026-10-22
area: battle, render, test
files: crates/piney-world/src/combat/boss.rs, docs/engine/boss-kyvia.md
started: 2026-10-22T16:07:32-06:00
took: 8m
---

# 425. Cubia's attacks show their names: SetupSkillName reads x01 to x04 in fields 9 to 12 whatever the row says, so Legion's Reach, Ark Bullet and Megiddo Flame come from the field's file (#72)

Issue #72 also said Cubia's attacks showed no names. The cinema's bars
came down for each attack, empty.

## What the game does

`SwitchActionPattern` (MUT gcmn 0x004d48a0) turns the cinema on with 33
(`HandAtk`), 32 (`LightAtk`) and 34 (`MegidFlame`); Outbreak's second
fight with 36, 37, 38 and 46 (the thunder). `_g_cinemaSkillName` (MUT
0x00619320) gives those rows `TEX_kbi_skl` rows 0, 1, 2 (and 1, 0, 2, 8)
but no file. `SetupSkillName` (MUT gcmn 0x00480540, OUT 0x0047bed0)
takes the file from `game.field` in fields 9 to 12 (x01 to x04) whatever
the row says, and the row's own file elsewhere. So on the disc the names
are x01's `TEX_kbi_skl`: "Legion's Reach", "Ark Bullet", "Megiddo Flame".

## The port

`BossLook::load` built the cinema's names only from rows naming the
boss's own file, and applied the field's file only to the skill rows
(`OnCinemaMode` with a skill, Fidchell's). Kyvia's rows name none, so it
had no names. Both now go through one `row_file`: x0N in fields 9-12,
else the row's file.

## Checks

`combat::boss::tests::kyvias_attacks_have_their_names_on_the_disc`:
Mutation's Kyvia in field 9 has cinema 32-34 at `TEX_kbi_skl` rows 0-2;
Outbreak's in field 10 has 36, 37, 38 and 46 at rows 1, 0, 2 and 8.
Before the fix 32 had none. The shots of
`kyvia_stage::kyvia_attack_shots` show each title over its attack
(/mnt/data/claude/scratch/cubia/acts/kyvia-00456-act3.png, Legion's
Reach).

**Still unknown:** nothing
