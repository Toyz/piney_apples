---
number: 431
title: Gorre's brothers share its param as the game does: one block that Gorre owns and the brothers read and write through it after the drain's 21, so a buff on one brother is on all three and runs out three times as fast, checked frame by frame against the game's code
date: 2026-10-22
area: battle, test, tooling
files: crates/piney-battle/src/chara.rs, crates/piney-battle/src/scene.rs, crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/message.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/damage.rs, crates/piney-battle/src/skill.rs, crates/piney-battle/src/flow.rs, crates/piney-battle/src/item.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/boss_fx.rs, tools/test_battle_gorre_rs.py, docs/engine/boss-gorre.md
started: 2026-10-22T19:10:52-06:00
took: 35m
---

# 431. Gorre's brothers share its param as the game does: one block that Gorre owns and the brothers read and write through it after the drain's 21, so a buff on one brother is on all three and runs out three times as fast, checked frame by frame against the game's code

boss-gorre.md listed as Unknown that after the drain's 21 the game's
brothers share Gorre's `param`, while the port wrote the same values to
three copies (worklogs 303, 412, 429). Now the port keeps one block, which
the brothers read and write through Gorre.

## What the game does

`ccChar.param` (+4, also read as `personality`) points at a
`ccBossParam`: `real`, `temp`, `time` (0x20 each), `PP`, `PPcount`,
`PPrestore`. Gorre's and each brother's own is at +0xe0.
`SetBaseParam(data, param)` (OUT gcmn 0x0058d160) stores both pointers
and sets the block from the row:

- `real` the row's stats, `temp` and `time` cleared;
- `PP`, `PPcount` and `PPrestore` 0;
- HP and SP full, the conditions cleared.

The constructor passes Gorre's own block (row 4), and `Init` each
brother's (rows 40 and 41, 0x004ae9f8, 0x004aea20).

Msg 0's 21 (`SendMessage`, 0x004ae140) calls `SetBaseParam(brother, row
4, &Gorre's param)` for both. After that:

- Gorre's block is set from row 4 twice. Its own `PPrestore` goes to 0
  too.
- `PP` -1 and `PPcount` 0 are written through Gorre, then each brother.
- Each frame, Gorre's `CalcRealEx` (0x004a9650) and each brother's
  `CalcReal` (0x0058d270, from `ccBoss::Main`) run `time` down on the one
  block: three a frame.
- A buff or debuff on one brother is on all three.
  `_ccSkillModifyCondition` (0x00597bb0) writes `temp` and `time` at
  `tp->personality` + 0x20 and + 0x40.

Every read and write of the block in boss05.cpp goes through the pointer:

| where | what |
| --- | --- |
| `Main` 0x004aa458 | Gorre's `PPcount` up while the party is held |
| `Main` 0x004aa520 | the count written to both brothers' `param` |
| `OnThinkWave` 0x004ad0e0 | the count up in the Wave |
| brother `Main` 0x004aefe4 | the brother's count up while Gorre holds the party |
| `SendMessage` 0x004add10-0x004ade18 | the shared break: the struck brother's count to Gorre's, a fresh break (300) zeroes the other's `PP` and sets its count, then each takes the struck brother's `GetPP()` |
| `SendMessage` 0x004adf80 | Gorre's `GetPP()` against half its `GetMaxPP()` for the Super table |
| `SendMessage` 0x004ae140 | 21 |
| `CalcRealEx` 0x004a9684 | Gorre's own block, as `CalcReal` |

## What the copies hid

Two things the copies got wrong, both after 21:

- Gorre's `PPrestore`, `temp` and `time` were kept; the game resets
  them. (`PP` is -1 from then on, so `PPrestore` is never read again.)
- A buff or debuff on a brother stayed on that brother and counted down
  once a frame. In the game it is on all three and lasts a third as long:
  a debuff's 1,800 frames run out in 600.

## The port

- piney-battle:
  - `chara::Param` is the block. `Foe::param` is `ParamAt::Own(Param)`,
    or `ParamAt::Of(owner)` for a character pointing at another's.
  - `Scene::param`, `param_mut`, `real` and `temp_time` resolve through
    the owner. `share_param`, `own_param` and `param_owner` set and read
    the pointer.
  - `Scene::lend` (and `lend_in`) hands the owner's block to a sharer for
    code that takes one character: `CalcReal`, `CalcBattleDamage`,
    `ModifyCondition`, the cures, a book, `DispConditionEffect`.
  - `Scene::view` gives a copy for reading: the AI's
    `SkillDamageValue`, `CheckTargetConditionBySkill`, the conditions'
    rolls, kept entries.
  - `Char::real`, `temp_time` and `param` on a sharer outside these stop
    the port. It is a bug in the port, not a state of the game, and it
    caught one: `disp_condition` (piney-world) now lends.
  - `kyvia::set_base_param_of` is `SetBaseParam` with another's block;
    21 uses it.
  - `gorre/message.rs`: `share_protect` reads and writes in the game's
    order, through each character's `param`.
  - Gorre's and the brothers' count steps and every other boss's gauge
    reads go through `Scene::param`.
- piney-game: the target window's buffs, Exdefense and break count read
  through `Scene`.

## Checks

- `tools/test_battle_gorre_rs.py`:
  - For Gorre and each brother, every frame: whose block `param` points
    at (its own +0xe0 or Gorre's) and the whole block.
  - Script kind 12 writes a buff or debuff (field, value, time) through
    a brother's `param`, before and after 21.
  - `bulk 100` from 9900, 20000 and 40000 agree: 951,176 frames, 24,121
    of them with both brothers on Gorre's block, 18,951 with a buff
    running on it. Quarantine's gcmn, `bulk 30`: 95,818 frames (726
    shared) agree.
  - With three copies instead, 3 of the first 30 cases part: first
    Gorre's `PPrestore` (game 0, port 2), then, the reset aside, brother
    0's `temp[11]` and `time[11]` (game -8 and 537, port 0 and 0) after a
    debuff on brother 1.
- `session::tests::boss_fx::gorres_brothers_share_its_param_after_the_drain`
  (Outbreak's event 218 in the arena) drains a brother and debuffs the
  other with skill 163:
  - both brothers on Gorre's block for 2,198 frames;
  - the debuff's time falls 3 a frame through all 600 frames;
  - brother 0 and Gorre lowered in 599.
- `scene::tests::a_sharer_reads_and_writes_its_owners_block`.
- The other bosses' harnesses (Skeith, Innis, Magus, Fidchell, Kyvia)
  and test_battle_rs, items, drain, party AI and enemy AI: their tests
  pass.
- piney-battle, piney-world and piney-fieldui tests, piney-game's suite
  (four threads), clippy, fmt.

**Still unknown:** nothing.
