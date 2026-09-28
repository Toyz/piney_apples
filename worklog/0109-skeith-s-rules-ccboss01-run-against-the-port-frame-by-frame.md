---
number: 109
title: Skeith's rules: ccBoss01 run against the port frame by frame, and the effects' lives measured
date: 2026-09-25
area: battle, test
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/affect.rs, crates/piney-battle/src/chara.rs, crates/piney-battle/src/tables.rs, crates/piney-battle/src/lib.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-battle/examples/battle_probe/main.rs, crates/piney-data/src/anim.rs, tools/test_battle_boss_rs.py, tools/annotate.py, docs/engine/boss.md
---

# 109. Skeith's rules: ccBoss01 run against the port frame by frame, and the effects' lives measured

The tutorial ends with Skeith (event 30). Nothing of `ccBoss` was ported.
This entry is its rules, the first of three parts:

1. the rules;
2. the arena, the boss's entry and its picture;
3. the member drain's menu and the ending.

## Reading the listings

Skeith is about 60 functions of `ccBoss` and `ccBoss01`, with members
past `0x8000` (the class is `0x296b0` bytes).

**`tools/annotate.py`** is new. It disassembles a function as
`tools/disasm.py` does and follows the registers that hold `this`: `$a0`
on entry, `move` copies, and stack slots it was saved to. It names each
load and store from the DWARF layout, with base classes and members
flattened, and follows `lui $at` / `addu $at, this, $at` for the large
offsets. Vtable loads are named by their slot. It is an aid for reading,
not a decompiler: it follows registers in listing order, not along the
flow.

## The port

**`piney_battle::boss`**:

- **`SkeithData`**: the three pattern tables, the anim table and
  `BossSkillTbl`, read from `GCMN.PRG`.
- **`Boss`**: every `ccBoss`/`ccBoss01` field the rules use, its two
  `Anm`s, the two `ccAnimateObject` fades and the effect manager.
- **`Boss::main`** is `ccBoss01::Main`, the manager's pass first.
- **What it asks of the rest of the game** comes back as `Out`: sounds,
  flashes, effects, menu state, the arena's layer, the stage effect, the
  cinema.

The rest of the crate:

- **`Foe.boss`** carries the state.
- **`AffectFunc::Boss`** sends an affect on the boss straight to
  `boss::entry`, as `bossAffectFunc` does. `AffectCtx.boss` brings the
  tables and clip lengths.
- **`piney_data::anim::forward_clip`** is `_AnimateForward` over a bare
  frame count, split out of `Animation::forward` so the boss's clips can
  use it. `tables::game_image` reads `GCMN.PRG` over the executable.

The page is [docs/engine/boss.md](../docs/engine/boss.md).

## The check

**`tools/test_battle_boss_rs.py`** builds Skeith with the game's
constructor over a party of one to three. It runs `ccBoss01::Main` natively
for 300 to 1500 frames, under a script:

- Kite's hits (type 1, -1 to 400);
- the protect gauge set;
- the drain's 13 and 21;
- in half the drains, hits every 2-12 frames after, which kill;
- menus 0, 5 and 74 opening and closing.

`battle_probe boss` plays the same case, and every frame is compared:

- acts, patterns and waits, movement, position and target;
- HP, the gauge, the flags;
- both animations' clips and frames;
- the effects, the party's HP and hold;
- the calls: sounds, flashes, effects, cinema, stage, fly fonts, hit
  marks, skills;
- `ccMenu`'s forbid, cursor and stream-menu words;
- `rand()` and `ccRand`'s index.

`EntryAffect` and `checkPartyAnnihilation` run natively, so the hits go
through the game's own gate. 300 random cases match. Over 30 of them the
acts reached are:

- 0, 3-8, 12, 13, 14 (death), 16 and 18;
- 11 is passed in a frame.

Return (17) is in none of Skeith's tables.

What the first runs caught:

- **The fades start opaque.** The constructor sets both
  `ccAnimateObject`s' transparency (and scale) to 1.0. The port had 0.
- **`ClearCondition` at death is the plain one.** `ccChar::ClearCondition()`
  clears the conditions only. The port's `affect::clear_condition` also
  zeroes the buffs and the enemy's `PP` (the record overload). At
  Skeith's death that wiped a gauge the game keeps.
  `affect::clear_conditions` is the plain one now.
- **Two harness faults.**
  - `ccEntryCmnd` appends after `cmndEneLast`. The last case's pointer
    was left over, so a new Skeith linked to itself, stayed off the list,
    and `ccCheckTarget` refused every hit. The harness now resets the
    three `Last` pointers.
  - GameBattle's `checkPartyAnnihilation` stand-in answered instead of
    the game.

## The effects' lives

Skeith waits on its effects' `m_bEnabled` in the wave and the magic, so
how long each lives is timing. The port's lives were guesses.
**`test_effect_lives`** now runs each real `ccBossEff*Create` with Skeith's
arguments (the stand-in unhooked) and the `Draw` of what it put in the
manager, until the flag clears:

| effect | was | is |
| --- | --- | --- |
| WaveShock | 60 | 45: its `ccAnm` to the end of `ANM_ex31lhit` |
| MagicSquare | 150 | 91: it makes a `ccBossEffLight(pos, 0, 80, 10)` and returns -1 |
| ForceGenerator | life + 8 num | life + 10 ((num - 1) / 2) + 3: photon k waits (k / 2) * 10 |
| AutoSamonRing | 30 + 3 n | 20: the ring fades out whatever the model |
| IceBreak | 124 | 123: a flash, 61, 61 |
| Dead | 120 | 120 |

The ForceGenerator formula was measured at num 1, 2, 3, 4, 5 and 8 over
lives 10-100. The third int does not matter. The listings agree:

- the BrightMagicSquare constructor's delay is `(k / 2) * 10`;
- IceBreak's `Draw` counts to 60 twice;
- the ring's `Draw` fades `Transparency` by `Tpoint`.

The magic is now 115 frames shorter at the ring alone.

## Checked

- The harness's unit tests (6 cases and the effect lives).
- `bulk 300` matches.
- The battle, Kite, fellow, items, drain, event and anim suites pass.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **The arena.** It is not known which `EVENTAREAB0` area event 30
  loads. `SwitchLayer` and `DMY_center01` are not ported.
- **The runtime.** The boss entry (type 7) is not in the field's runtime,
  nor `hold 7` or `present`/`absent 7`, so Skeith cannot yet be met in
  play.
- **The picture.** Not ported:
  - `CMP_trall` and its animations;
  - the afterimages and the cross's trail;
  - `ccBossCam`, the stage fader and the reversed layer;
  - the effects' pictures.
- **Menu 74** (`StreamMenu`), which the member drain opens, is not
  ported. The rules only set its request words.
- **The effects' particles** draw from the random generators in the
  game. The harness's stand-ins do not, and neither does the port. How
  much that moves Skeith's `ccRand` draws (pattern 10) in play is not
  measured.
