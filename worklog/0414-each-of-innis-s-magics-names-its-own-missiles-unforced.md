---
number: 414
title: Each of Innis's magics names its own missiles, unforced: Absolute Zero throws ice, Thunder Trance lightning, Demon Fire fire (#69)
date: 2026-10-21
area: battle, test
files: crates/piney-game/src/session/tests/boss_fx.rs
started: 2026-10-21T09:36:27-06:00
took: 9m
---

# 414. Each of Innis's magics names its own missiles, unforced: Absolute Zero throws ice, Thunder Trance lightning, Demon Fire fire (#69)

[[411]]'s shots paired titles and missiles that do not go together
("Demon Fire" over ice crystals, "Absolute Zero" over fireballs). That
came from its test forcing each magic's element after the square was
drawn: the title is the square's, the missiles the forced element's. In
an unforced fight they match, in the game and in the port.

## What the game does

`ccBoss02::MagicSquare` (MUT gcmn 0x0049a000) at count 2 draws
`n = abs(ccRand() % 3)` (0x0049a2ec) into `EnyInt` (+0x29378), turns
cinema 5 + n on (`OnCinemaMode` 5, 6, 7 at 0x0049a360, 0x0049a380,
0x0049a3a0) and makes `MagicSquare(pos, n)`. At count 10 after the eye's
move the same `EnyInt` picks `IceAttack` (0), `ThunderAttack` (1) or
`BlazeAttack` (2) (0x0049a6f0-0x0049a748). Cinema 5-7 are x21's
`TEX_ini_skl` rows 0-2 (`_g_cinemaSkillName`): Absolute Zero, Thunder
Trance, Demon Fire. The port's `innis::magic_square` follows it (one
`eny_int` for the cinema, the square and the attack), as
tools/test_battle_innis_rs.py already compares.

## Check

`session::tests::boss_fx::innis_magic_names_its_own_missiles`: Innis's
fight on Mutation's event 107, the gauge at half from frame 300 and
nothing forced, run until each element has come. At each magic's first
frame in flight it records the cinema name's row and the element of the
missiles in the boss's slots, and asserts the row is the element's. The
run gives fire (row 2) at frames 486, 2267, 5132 and 6253, lightning (row
1) at 3379, ice (row 0) at 8041: all match.

`innis_unforced_shots` (ignored) writes 30 and 60 frames into each
flight to /mnt/data/claude/scratch/i69/unforced:
`innis-00516-row2-Blaze.png` (Demon Fire, fireballs),
`innis-03409-row1-Lightning.png` (Thunder Trance, bolts),
`innis-08071-row0-Ice.png` (Absolute Zero, the ice's burst). No port
change; piney-game's suite (four threads), clippy and fmt pass.

**Still unknown:** nothing.
