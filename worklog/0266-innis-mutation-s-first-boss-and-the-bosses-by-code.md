---
number: 266
title: Innis, Mutation's first boss, and the bosses by code
date: 2026-09-28
area: battle, test, volumes
files: crates/piney-battle/src/boss/innis.rs, crates/piney-battle/src/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/bosscam.rs, crates/piney-world/src/field_world.rs, crates/piney-gen/src/manifest.rs, tools/test_battle_innis_rs.py, docs/engine/boss-innis.md
---

# 266. Innis, Mutation's first boss, and the bosses by code

Innis (`ccBoss02`, its three `ccBoss02Slave` images and their
`BreakMirror`s) is ported from MUT gcmn (0x004941b0-0x0049da1c,
0x00533d90-0x00534a7c), with Infection's DWARF for the names and layouts.
Its missiles' `Draw`s (MUT 0x00478d50, 0x00479930, 0x0047a220) are ported
as rules too, because the first one calls `MagicDamage`. The reference is
[docs/engine/boss-innis.md](../docs/engine/boss-innis.md).

## The bosses by code

`ccBossEntryStart(code)` starts `bossFunc[code]` (MUT 0x006192c0). The
field made Skeith for any `entry 7`. Now `FieldWorld::start_boss(code)`
takes the entry's code: 0 Skeith, 1 Innis, and nothing for the others.
`Boss.class` (`Class::Skeith`, `Class::Innis`) picks the `Main`, and the
area host reports `eventMng.bossEntry` as that code. Each boss's tables
come from the build: `innis_pattern`, `innis_epitaph`, `innis_anims`,
`innis_various_skills`, `innis_downer_skills`, `innis_monster_anims`,
`innis_ring_models` and `cinema_skill_names` join the `combat` group
(`DATA_VERSION` 10). The cinema's names are no longer Skeith's one row:
`_g_cinemaSkillName` gives Innis rows 5-8 and 70 (x21's `TEX_ini_skl`).

## What had to be found

- `DmgCount` starts at 0, not maxHP: the constructor reads maxHP before
  `SetBaseParam`. The first `DmgCheckCount` therefore sees no damage.
- `SwitchActionPattern`'s jump table (MUT 0x006e0c20) is not in word
  order: 2 is `Pattern06`, 3 `Pattern02`, 8 and 9 `Pattern05`, 10
  `Pattern11`, 13 `Pattern08`.
- `SelectTarget`'s type 14 never picks; `RndTarget` draws 0-14.
- Act 5 writes `ccMenu.forbid` itself, and `Main`'s reserved-forbid check
  in the same frame reads that write.
- `SkillAttack` passes `SetFreeCamPosView` a view whose x and y are an
  uninitialised stack slot. It holds `SetCameraEyes`' saved `$ra`
  (0x00496fe4), which the FPU reads as zero.
- `BreakMirror::Mbreak` never resets its matrix between shards, so the
  shards' turns accumulate. `MainMirror` calls `SetData` again when done,
  another 300 `ccRand` and 60 `rand()`.
- Mutation's `ccBossCam` moved its fields. +0xe0 is the pitch `SkillAttack`
  raises, and `CheckMoveCamera` reads +0xec. `SetMode` 5 and 6 and
  `SetFreeCamPosView` are ported into `BossCam`.

## Checks

`tools/test_battle_innis_rs.py` (`PINEY_VOLUME=mutation`) runs Mutation's
own `ccBoss02` constructor and `Main` in eemu against battle_probe's
`innis` request. It compares every frame over random cases. Its two tests
(`test_effect_lives`, six cases 7100-7105) pass. `bulk 120 9100` matched
all 120 cases, 228,066 frames. The acts reached were 0-6, 12, 13 and 14.
One bulk failure was the harness's: it did not reset its shard count
before the constructor. Skeith's harness (`bulk 40`),
`test_bosscam_rs.py`, `test_boss_effect_rs.py` and `test_cinema_rs.py`
still pass.

The Rust tests are in `boss::innis::tests`:
`stands_behind_kite_and_waits_first`,
`drained_to_the_epitaph_then_dies_and_exits`,
`the_magic_lands_and_lets_the_party_go` and
`the_images_break_and_come_back`. The session tests
`event_107_ends_with_innis` and `innis_drained_through_the_menus` are in
`session/tests/skeith.rs` (session.rs, which declares the test modules,
was not mine to edit). They could not be built here: the tree's
piney-stream did not compile at the time (`opening_events`, work in
progress elsewhere).

## Event 107 under the autopilot

`PINEY_SURVEY_ONLY=107 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000`
ends with `story 107: blocks 0xff89b; 4321 places; last: The World - area
1 field 2`. That is block 17 played (Innis made) and block 20 not. The
fight runs: Innis's HP falls and its protect breaks several times, with
windows of about 300 frames. The pilot tries Data Drain at each break,
but menu 65 finds no target and shows its warning. Kite stands near
(-154, 402) casting while Innis roams. At the first break Innis was 2515
away, past Data Drain's reach (`triggerRange` 2000 plus the width). For
most of its actions Innis is also off the command list (`EntryFlg`, as the
game does it). Skeith came to Kite; Innis does not, and the pilot does
not walk to it. The autopilot was left alone as asked.

**Still unknown:** whether the autopilot, once it walks Kite to Innis
during a break, finishes event 107 (and whether the two session tests
pass once the tree builds). The pictures of the missiles, rings,
particles, blur, shield and shards, and the MagicSquare for n 1 and 2, are
not drawn. Which clump the game draws the Epitaph's `ANM_ex2x*` clips on
is not checked. No case reached act 7 on Innis itself.
