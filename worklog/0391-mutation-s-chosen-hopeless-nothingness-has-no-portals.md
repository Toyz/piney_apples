---
number: 391
title: Mutation's Chosen Hopeless Nothingness has no portals: EntryGimmick's volumeNum 2, field 27 test is asked of the disc's volume
date: 2026-10-08
area: world, battle, volumes, test
files: crates/piney-battle/src/entry.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/hopeless_nothingness.rs, tools/test_battle_spawn_rs.py, docs/engine/dungeon.md, docs/README.md, BUGS.md
---

# 391. Mutation's Chosen Hopeless Nothingness has no portals: EntryGimmick's volumeNum 2, field 27 test is asked of the disc's volume

Issue #57 (Mutation, build 4cce396): the dungeon of Chosen Hopeless
Nothingness (field 27) is empty of enemies on the PS2, but the port filled
it with magic portals. The report's map shows yellow marks in the rooms.
Mutation's event 101 (`M201`) sends Kite and BlackRose back there.
BlackRose says "There are no enemies..." (message 8, block 19).

## What the game does

`WORLD_MAN::EntryGimmick` places a dungeon's gimmicks once per dungeon:
`SetItemBox`, `SetMagicCircle`, `SetIDOL`. Between the first and the
second it tests `volumeNum` and the field:

```
MUT main 0x001b7058  lui $at, 0x36 ; lw $v1, volumeNum (0x00363578)
         0x001b7060  li $v0, 2     ; bne $v1, $v0, -> SetMagicCircle
         0x001b706c  lw $v1, game.field ; li $v0, 27 ; beq -> skip it
```

Every volume has the test: INF 0x001a1f20, OUT and QUA in their
`EntryGimmick` too, always with 2 and 27. So only Mutation's field 27
dungeon goes without portals and the lake objects `SetMagicCircle` also
makes. Its boxes and idols are placed. The field branch has no such test.

The dungeon doc already had the test (entry 343, read in Infection's
code). The port had it as `entry::entry_gimmick`, with `volumeNum` a
constant 1. Two things kept it from working on Mutation:

- the constant: Infection's `volumeNum`, written in when the port was
  Infection alone;
- the runtime (`Combat::start_entries`) never called `entry_gimmick`. It
  calls `set_item_box`, `dungeon_set_magic_circle` and `set_idol` itself,
  with no test at all. Only the battle probe ran `entry_gimmick`.

## Fix

`entry::dungeon_places_portals(volume, field)` is the test, read from the
volume the spawn tables were made for (`SpawnTables::volume`). Both
`entry_gimmick` and the runtime ask it. The constant is gone.

## Checked

- **Failing before, passing after:**
  `mutations_hopeless_nothingness_has_no_portals` puts a new game on the
  first floor of field 27's dungeon. On Mutation the entry control holds
  no portal and 33 gimmicks; before the fix it held 23 portals, on floors
  0 to 3. On Infection the same dungeon keeps its portals.
- **The game's own code:** `tools/test_battle_spawn_rs.py`'s
  `entry_gimmick` check now draws field 27 half the time. With
  `PINEY_VOLUME=mutation` it matches the game's `EntryGimmick` in eemu on
  every case. With the test taken out of the port it fails on the first
  field 27 dungeon case: the port calls `DUNGEON::SetMagicCircle` and the
  game does not. Infection matches too.
- **Shots** (`hopeless_nothingness_shot`, ignored; Mutation, floor 0
  room 2, where Infection's rules put a portal, Kite walked in 40 frames):
  `/mnt/data/claude/scratch/i57/before/mut-field27-dungeon.png` has the
  portal's mark on the minimap. `after/mut-field27-dungeon.png` has none,
  as the PS2 shot.
- piney-game's suite (four threads), the tests of piney-battle and
  piney-world, clippy on the touched crates, fmt, `cairns check` and
  `tools/docs.py check` pass.

**Still unknown:** nothing for this issue. Outbreak and Quarantine run the
same test with their own `volumeNum` (3, 4), so their field 27 dungeons
keep their portals. That follows from the code; no PS2 shot of them was
compared.
