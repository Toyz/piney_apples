---
number: 508
title: Clean-up section 0, items 1-3: every caller names its volume's tables (the later volumes' thunder is sound 235), one guarded ccGetDircChg, and the bosses' HP kept in a short as the game keeps it
date: 2026-10-29
area: world, battle, volumes, test
files: crates/piney-data/src/field/mod.rs, crates/piney-data/src/field/generate.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-data/src/sound/mod.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/field_ambient.rs, crates/piney-viewer/src/main.rs, crates/piney-battle/src/geom.rs, crates/piney-world/src/ai.rs, crates/piney-world/src/rtownpc.rs, crates/piney-battle/src/boss.rs, tools/test_field_rs.py, tools/test_field_ambient_rs.py, tools/test_dungeon_rs.py, docs/engine/field.md, plans/rust-cleanup.md
started: 2026-10-29T19:15:38-06:00
took: 1h
---

# 508. Clean-up section 0, items 1-3: every caller names its volume's tables (the later volumes' thunder is sound 235), one guarded ccGetDircChg, and the bosses' HP kept in a short as the game keeps it

Section 0 of `plans/rust-cleanup.md`, items 1-3: the `INF` table statics
out, one guarded `ccGetDircChg`, and the bosses' HP arithmetic in one
place. One behaviour fix came out of the checks: Mutation's, Outbreak's
and Quarantine's thunder is sound 235, not 236. The clock also ran over
the start of the other section-0 items (worklog 509, 510), coded
alongside.

## Every caller names its volume

`field::INF`, `dungeon::INF` and `sound::INF` are gone. Every caller now
calls `tables_of(volume)`:

- `FieldArea::with_world` and `field_ambient::field_file` take the
  world's volume.
- The viewer detects its disc's volume and keeps each view's tables
  (`dungeon_params` names the disc's `volumeNum`, not 1).
- The probes take the volume from their harness: `field_snapshot`,
  `dungeon_snapshot` and `ambient_probe` get `volumeNum` from
  `tools/volume.py`'s `number()`.
- Tests that mean Infection say `tables_of(Volume::Inf)` through a local
  `inf()`.

**Did the later volumes' fields go wrong?** No. The tables differ only in
`volumeNum` and `sqrt` (`later_volumes_differ_only_in_the_square_root`
compares the Debug of all four). The generator's two `ccGetDist` tests
never part between the two roots: every point it measures is on the
600-unit lattice at z 0, and no lattice distance comes within an ulp of
8000 or 7000. `both_square_roots_test_alike` checks all 161 x 161
offsets.

`test_field_rs.py`, against the game's own `WORLD::Init` and `Generate`
through `field.py`:

| run | story areas | random fields | renders |
| --- | --- | --- | --- |
| OUT, the volume's tables | 107, 0 mismatches | 220, 0 | 4, 0 |
| QUA, the volume's tables | 107, 0 | 220, 0 | 4, 0 |
| OUT and QUA, Infection's tables (the old code) | 107, 0 | 220, 0 | 4, 0 |

So the static was wrong in kind but gave the same fields.

**The thunder.** `test_field_ambient_rs.py` passes on Infection. On
Outbreak and Quarantine it failed in 16 storm cases (types 7-10,
weathers 6 and 7): the port's `ccSeOn(236)` against the game's 235. The
old code sent 236 on every volume, so it would have failed the same way;
the harness had not been run there before. Each volume's
`WORLD::DrawRain`:

| volume | `li $a0` before `ccSeOn` |
| --- | --- |
| INF | 236 (gcmn 0x005a8e44) |
| MUT | 235 (0x005d43d4) |
| OUT | 235 (0x005d1c90) |
| QUA | 235 (0x004c46f0) |

`thunder_se(volume)` now gives the volume's. The harness expects
`se:235` past Infection, and `docs/engine/field.md` says so. After the
fix all three volumes pass.

## `ccGetDircChg`

There were three copies (`piney-battle` `geom.rs`, `piney-world`
`ai.rs`, `rtownpc.rs`). Only `ai.rs` guarded a zero rate. With rate 0
the game traps: `__divdi3` divides by zero on purpose. The port's
answer is the whole step, as for any rate of 0x10000 or more. Now
`geom.rs` has the one guarded copy, and the world re-exports it.
`dirc_chg_quirks` checks rate 0.

## Boss HP

The plan guessed the game adds in `int` and clamps there. The code says
otherwise. Each `Affect` adds or subtracts in `int` and keeps the result
in a `short` local before the clamp: `addu`/`subu`, then
`dsll32`/`dsra32` by 16, a sign extension from bit 15. The sites:

- INF `ccBoss::Affect`: heal 0x0045c9f0, hit 0x0045c90c, `cheatHP`
  0x0045c8c8.
- QUA `ccBoss06`: 0x003a4594, 0x003a45f8.
- `ccBoss08`: 0x003ad230.
- `ccBoss08_b`: hit 0x003b2df0, heal 0x003b2f40.
- `ccBoss08KillerEye`: 0x003bf784, 0x003bf7d0.
- `ccBoss08Seed`: 0x003b04b0, 0x003b0500.

So `i16` wrapping is the game's arithmetic. The `wrapping_*` copies
(Kyvia, Gorre, Magus, the leaves) were right. The plain `+`/`-` ones
(base, Innis, Macha, Corbenik's four) wrapped the same way in release
but panicked in debug. `boss::heal_hp` and `boss::hit_hp` (cheat
divisor 10, or 5 for the Killer Eye and seeds) now serve all eleven
sites.

## Checks

- piney-game's suite, and the tests of piney-data, -world, -battle,
  -audio and -viewer.
- `test_field_rs` on INF, OUT and QUA; `test_field_ambient_rs` on all
  three; `test_dungeon_rs`; `test_map_rs`.
- The boss harnesses: base, Innis, Macha, Corbenik (three), Gorre,
  Kyvia, Magus.

`test_field_rt.py` errors before any frame: its game side now returns
an `arms` key (from `test_world_rs`) that `field_probe` has never
printed. That came before this work; it is open below.

**Still unknown:** Why test_field_rt.py's game side returns an arms key that field_probe never prints (it errors before any frame, at HEAD too); and test_battle_corbenik_rs.py case 4 frame 622 (camview 1 against the port's 2), which fails the same at HEAD 2a88a71.
