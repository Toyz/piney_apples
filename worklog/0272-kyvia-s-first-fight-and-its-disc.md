---
number: 272
title: Kyvia's first fight and its disc
date: 2026-09-28
area: battle, world, test
files: crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-world/src/evarea_b8.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/bosscam.rs, tools/test_battle_kyvia_rs.py
---

# 272. Kyvia's first fight and its disc

Mutation's event 108 now fights Kyvia on the disc of field 9 and runs to
its end. Block 21's `entry type=7 code=12` makes `ccBossKyvia01(1)` with
its core and five gomoras (MUT gcmn 0x004d2a80, 0x004f8a30, 0x004ff220),
and block 23's `absent type=7 code=12` follows once the core dies and the
body falls. The disc is `EVENTAREAB8` (0x0041d200), which `GO(1)` makes for
fields 9-12. The references are
[docs/engine/boss-kyvia.md](../docs/engine/boss-kyvia.md) and the
`EVENTAREAB8` section of [docs/engine/evarea.md](../docs/engine/evarea.md).

## How it was taken apart

Of the 98 functions (74.4 KB) the fight shares with Kyvia's later ones,
the body's 18, the core's 46 and the gomoras' 32 were lifted to a
straight-line form (a register flow over each function's blocks, the
DWARF names of Infection's layouts on Mutation's offsets) and ported from
that, checked against the listings wherever a count or a float mattered.
The layouts match Infection's DWARF except one byte Mutation adds at the
body's +0x295f8: its cinema flag.

The lifted form hid one pattern. Where the source had `actCount++ == 40`,
the compiler loads the counter, stores it raised, and compares the old
register; the lifted form printed the store and then the compare, which
reads as the new value. Eleven places do this (the body's death at 60,
`LightAtk` at 40, 30, 10 and 30, `MegidFlame` at 100, the core's wave at
25 and its death at 30, `CheckCountAtkTime`'s `NowDmgtime++ < Dmgtime`,
`CheckCountGomora`'s `GsCount++ == GsRimit`, a gomora's `AtkWait-- == 0`).
The harness found the first; a scan of the listings for a raised counter
compared through its old register found the rest. The second gomora comes
out 31 frames after the first, not 30.

Two of the game's own slips are kept. The core's constructor sets each
gomora's `LifeFlg` from `&GomoraFlg[i] != 0`, an address, so always 1.
`SetCoreState(8)` tests `CoreState != 8 || CoreState != 6`, which always
holds. The blur's fields are words at +0x24 (`m_enabled`), +0x28
(`m_exit`), +0x0c (`m_scale`) and +0x1c (`m_abgr`), not the bytes and
offsets first assumed.

## The shape of the port

The core and each gomora are a character and a `Boss` of their own
(`AffectFunc::Boss`), so the party's hits reach them through the same
queue as any boss's, and the field runs the queued affects of all seven
before the body's frame. The body's frame takes the parts out of their
characters (`Tree`), runs the body, the core (`Slave`) and the gomoras,
and puts them back. The tables are six new entries of the combat group
(the three animation tables, `Skill_VARIOUS`, `Skill_DOWNER`, and
`AllGomoraList_1[0]`, which is its only list: 3 3 3 2 0, 0 1 1 1 2,
3 3 0 1 2); the data version is 12.

The boss camera needed Mutation's modes 3 and 4 (`SetTransfer`,
0x00476380): `HandAtk` waits on `ResetFlg`, and without the ease the
fight stood still in its first attack.

## Checks

`tools/test_battle_kyvia_rs.py` runs the game's constructor and `Main`
natively against battle_probe's `kyvia`, comparing every member the port
keeps for the body, the core and each gomora, the calls, the camera, the
blur, the effects, the party and both generators each frame. 410 cases,
561,061 frames, agree, and every act of all three classes is reached.
The harness stands in for the skills the gomoras ask for:
`ccSkillRequestParam` sets the caster's `skillID` and `skillStatus` as
`_ccSkillRequest` does, because the port's request does and a gomora
with a skill running does not move. `HitEnable`/`HitDisable` only set the
flag: the native list of hits looped once a gomora's hit was enabled
twice.

`event_108_ends_with_kyvia` (piney-game's session tests) puts the save at
block 21 on field 9 and runs to `eventStatus[0]` 2: the disc stops, the
core rises, Kite's hits (queued on the core every 20 frames while it is on
the lists) bring the body's flinch and `HandAtk`, the core dies, the body
dies and exits. `boss::kyvia::tests` and the disc's own tests hold the
rest.

## The survey

`PINEY_SURVEY_ONLY=108 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000`
ends `story 108: blocks 0xe0083; 446 places; last: The World - area 2
field 47`. The run enters area 47's dungeon (block 17) and from frame
13,950 to the end fights a Hackberry King there whose HP keeps coming
back (536, then 718 of 1410), so it never reaches field 9: blocks 21-23
never run and Kyvia is never made. Run earlier the same day, before
[[270]]'s pilot changes were committed, the same survey reached field 9 and
waited at block 23 (`blocks 0x6e0083`). The stop is in the pilot's dungeon
walk, not in the fight.

**Still unknown:**
- Why the Hackberry King in area 47's dungeon recovers under the pilot's
  fight, and whether the game's would.
- The gomoras' pushes on each other, the pad's sway, the arm's picture and
  the particles are not ported; which of `CMP_ex01gom2`-`4` each gomora
  draws is not known.
- `EVENTAREAB8` is held by unit tests, not compared with the game's `Draw`.
- Kyvia's later fights (levels 2-5, the EX mode) are not ported.
