---
number: 281
title: Mutation's attack spells: the ccSkill's request-time aim and the later release
date: 2026-09-28
area: battle, render, volumes, test
files: crates/piney-effect/src/spell.rs, crates/piney-effect/src/convergence.rs, crates/piney-effect/src/fall.rs, crates/piney-effect/src/upheaval.rs, crates/piney-effect/src/summons.rs, crates/piney-effect/src/summoned.rs, crates/piney-effect/src/files.rs, crates/piney-effect/examples/spell_probe.rs, tools/test_effect_spell_rs.py, docs/engine/effects.md
---

# 281. Mutation's attack spells: the ccSkill's request-time aim and the later release

`tools/test_effect_spell_rs.py` had only run on Infection. On Mutation's
disc the game's `ccSkill` died at frame 0, because the harness built it
with Infection's layout. MUT's `_ccSkillRequest` (gcmn 0x00598000) news
192 bytes, not 176; OUT's and QUA's do too. A vector at +0x70 moves
`creator` and everything after it 0x10 on: creator +0x80, target +0x84,
effPtr +0x94, `m_effElm` +0xb4.

## The new field

- The request copies the target's position into +0x60 (`tPos`) and into
  +0x70 when the target is on the lists (MUT 0x005981d4). The
  constructor (MUT 0x00598730) leaves +0x70 alone.
- `ccSkill::Main` (MUT 0x00598980) is Infection's up to the offsets. It
  refreshes +0x60 each frame and never touches +0x70.
- So +0x70 is where the target stood when the spell was asked for. The
  port keeps it as `Spell::t_pos_req`; `Spell::aim(volume)` answers
  `tPos` on INF and +0x70 from MUT on.

The elements still read +0x60. A diff of all 190 gcmn functions named
`*Element*` (INF against MUT, registers and addresses dropped) shows only
the skill's creator and target offsets, except two elements (below).

## The systems

Each MUT system differs from INF's in two ways: what INF aims at `tPos`
MUT aims at +0x70, and the caster is released later.

| system | MUT gcmn | aimed at +0x70 | released (INF) |
| --- | --- | --- | --- |
| Fall | 0x0059d1e0 | the meteor's height, sound 57, `ccSkillDamage2` | at 0 only with no target; else at 120, or at the end or a lost target before 120 (always at 0) |
| Tornado | 0x0059ccf0 | smoke, rings, their sound, the hits round the place, the shake's range | after 75, with the end (at 30) |
| Convergence | 0x0059d930 | count 20 writes the target's pos to +0x70 (INF: `tPos`); sounds 63, 64, the shake's range, note 66 | at 93, or at the end before 93 (at 40) |
| Upheaval | 0x0059e170 | the ground generator, the pillars, `ccSkillDamage2`, note 56, the shake's range | at 93, or at the end before 93 (at 35) |
| Summons | 0x0059ea10 | sounds 62, 290's lock-on, 289's drills, `ccSkillDamage2` | at 60; 100 from level 3 and for 289, 290 (at the end) |

The convergence's burst, smoke and `effSkillBreakSE` stay at the target's
middle, read from the target as on INF.

## Main and two elements

A diff of every main `eff*` function, and of the rest of INF's effect
code (main 0x001bc000-0x001dc000), leaves these beside the three already
in the tree (the pieces' aim, `effSkillChargeObj`'s new argument, the
drill's `endFlag`):
- `effSkillChargeObject` (MUT main 0x001eb650) sets the controller's
  `posT` to the target's middle (INF: its feet). The pieces start round
  it. Each piece's own `posT` is still the target's feet, and MUT's pieces
  fly to that.
- `effTCDrillMissile` (MUT main 0x001edcf0) builds each drill's place
  from `tPos` in a local. INF built it in the caller's `cPos`, whose z
  gathered `tPos.z` each time round. Each drill now lives 600 frames (INF:
  the row's -1).
- `ccTreeSummonsElement`'s roots: INF's loop never moves past
  `m_rootsTbl[0]` (the `addiu $s0, 16` is missing). MUT's (gcmn
  0x00511108) places all eight 100 round the tree through
  `ccTransPosW2P` and `ccTransPosP2W`.
- `ccThunderSummonsElement::Main` (MUT gcmn 0x0050e0e8) plays the hit's
  four sounds once even when the shake is out of the camera's range. INF
  plays them only in range.

## The harness

- The `ccSkill` is `volume.SKILL_SIZE` and its fields go through
  `volume.skill_at`. From MUT on the request sets +0x70 and the skill's
  state reports it as `tPosReq`; the probe prints the same.
- The elements are read with INF's DWARF. MUT has no .debug section, and
  its element classes keep INF's layout.
- The zeroing hooks (locals the game reads before writing) are per
  volume, and each decodes the `move` it replaces. OUT's and QUA's
  recompiled frames keep the tree upheaval's throw at sp+112, the ice
  `Delete` throws at sp+80 and the goblin's rings at sp+256.

## Results

`python3 tools/test_effect_spell_rs.py bulk 20` (the release probe), all
102 attack spell ids, 20 random casts each (2,040 casts a volume):

| system | INF frames | MUT frames |
| --- | ---: | ---: |
| tornado | 37,777 | 37,777 |
| fall | 32,457 | 32,456 |
| convergence | 44,484 | 44,439 |
| upheaval | 44,942 | 44,942 |
| summons | 87,626 | 85,148 |
| all | 247,286 | 244,762 |

0 mismatches on both. The summons ran in six shards of the same cases.
Before the ports, MUT parted at frame 0 (the harness's layout), then at
the pieces' `posT` (convergence), then at 289's drills' life.
`cargo test --release -p piney-effect` passes; clippy is clean.

## Outbreak and Quarantine

Run with `PINEY_VOLUME=outbreak` or `quarantine`, 3 casts each (before
the last two fixes below), the two discs gave the same answers:
- tornado: 0 mismatches (5,614 frames).
- fall, convergence, summons: parted by one ulp in a matrix or an
  element's `m_dirc`. That is the recompiled float code (`sqrt.s` and the
  like), not chased.
- upheaval: parted at 219's ice `Delete`, whose frame keeps the throws'
  leftovers elsewhere. The hooks are now per volume (above).
- 220: OUT's and QUA's `_Level4` raises 42 spikes a wave (OUT gcmn
  0x005067e0, QUA 0x003f90f0), not 43. Ported; not rerun on OUT or QUA.

**Still unknown:** why MUT moved the releases (93, 120, 60 and 100).
piney-game's `AreaFx::spell` requests a spell at its first system call,
so its +0x70 is the target's place at count 0, not at `_ccSkillRequest`;
a runtime that asks earlier would need to pass it. OUT's and QUA's
one-ulp float differences, and whether the per-volume hooks and the
42-spike wave make upheaval agree there (not rerun).
