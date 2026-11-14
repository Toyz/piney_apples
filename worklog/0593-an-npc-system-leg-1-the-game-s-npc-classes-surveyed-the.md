---
number: 593
title: An NPC system, leg 1: the game's NPC classes surveyed, the design, the angle helpers and ccEntryObj once, a Core for the merchant and the dog
date: 2026-11-14
area: world, decomp, engine
files: crates/piney-battle/src/geom.rs, crates/piney-world/src/chara/entry_obj.rs, crates/piney-world/src/chara/npc.rs, crates/piney-world/src/town/merchant.rs, crates/piney-world/src/town/dog.rs, crates/piney-world/src/town/pc.rs
started: 2026-11-14T06:26:58-07:00
took: 1h 24m
---

# 593. An NPC system, leg 1: the game's NPC classes surveyed, the design, the angle helpers and ccEntryObj once, a Core for the merchant and the dog

The first leg of plans/rust-cleanup.md's "An NPC system", on branch
`npc-system`. The design and the steps left are in the relay's handoff
(scratch/npc/HANDOFF.md); this entry keeps what the game's code settled.

## What the game's NPCs share

INF's DWARF (tools/dwarf1.py `type`) gives the hierarchy. `ccChar` (0xe0)
holds the base (`npcTbl`'s `ccNpcParam`), place, heading, transparencies
and `affectType` (+0x9c); `ccEntryObj` (0x1d0) the entry control's flags
(+0xe0 objFlag .. ccs2Flag), plDist, plDirc, the fade and the event's turn
(fadeFlag, fadeCnt, grotDeg, grotSpd), entParam, bodyHit and the list links
(+0x1c0, +0x1c4), with `routine` (INF GCMN.PRG:0x0042fa60) and one virtual,
`main`; `ccGimmick` (0x1e0) adds gimId, actNum, actCnt, anmFlag. The four
NPC classes are `ccMerchan` (0x210), `ccRtownPC` (0x2f0), `ccDog` (0x240)
and `ccPGuso` (0x330). Each declares its own `actNum`, `actProcess`,
`actCnt` (the merchant keeps ccGimmick's `actCnt` at +0x1d8) and the same
`bodyHitFlag`/`bodyHitCnt` pair, and each `main` updates the pair with the
same code around `CollisionDetection` (gcmn 0x00153470), taking the push or
not. Which class a row makes is data: `npcTbl[row].entry.func`
(`ccEntryRtownMerchant`, `ccEntryRtownPC`, `ccEntryDog`,
`ccEntryChibiGuso`, the port's generated `EntryFunc`).

There is one NPC list, `g_entCtrl`'s fourth, appended by `entryNpc` and
walked by `ccThEntryCtrl` (gcmn 0x00431970) after the enemies, circles and
gimmicks; each NPC draws in its own `main`. In a town `ccEntryEventMng`
(INF SLUS_202.67:0x001b62e0) enters the event's NPCs first (types 3 and 4,
0x001b67c4 and 0x001b67dc), then `ccSetMerchant(0)` (0x001b6c40),
`ccSetChaosGate`, and by `game.town` (0x001b6c50-0x001b6cc8): town 0
`ccEntryRandomNpc`; 1 `ccSetDog`, `ccSetChibiGuso`, `ccEntryRandomNpc`; 2
and 3 `ccSetChibiGuso`, `ccEntryRandomNpc`; 4 `ccSetChibiGuso`. The port
steps and draws its town NPCs by kind instead, so an event's PC runs after
the merchants, dogs and Grunties rather than first; the one list (a later
leg) puts them in the game's order.

`rtpcCheckNote` (gcmn 0x00509160) is the PC anm's note callback, set by its
constructor (the only reference, 0x00506a80), so a PC's steps sound
wherever it animates. The port's field path keeps only a PC's `Transfer`
and drops its `Step`, `Vanish`, `Drained` and `Chat` events; the one event
channel (a later leg) carries them in every area.

## ccGetDircChg's modes

`ccGetDircChg` (INF SLUS_202.67:0x001d9eb0) masks the mode with 0xf0000 and
compares the result with 2, 1 and 0 (0x001d9f20-0x001d9f40). Only 0 can
match, so a mode with any high bit takes the whole step, capped at 24576.
The merchants' own copy (`dirc_chg`) had branches for 0x10000 (nothing under
1024) and 0x20000 (the step over 16) that the game cannot take; no caller
reached them (grotSpd is a sign-extended short, the merchants pass 64 and
128), so dropping them changed nothing. The geom test `dirc_chg_quirks`
holds it.

## What changed

- The angle helpers are once in `piney_battle::geom`: `get_dirc`
  (0x001d9ce0), `get_dirc0` (0x001d9c50), `set_dirc`, `get_dirc_chg`,
  `pi_limit` and `camera_deg` (`ccCheckCameraDeg`, 0x001da710, on points
  through the caller's W2P). Copies went from town/pc.rs, town/merchant.rs,
  chara/ai.rs, combat/stage.rs, systems/flare.rs, event/mod.rs,
  piney-battle's enemy_ai.rs and piney-effect's shield.rs, drawelm.rs and
  effect.rs (all the same newlib `atan2f`, so the same bits).
- One `chara::entry_obj::EntryObj` with one `routine`; the walking PC's
  inline copy of the fields and of `routine` went (its `deleteCmnd(0)` is
  the shared one's for entRoot -1).
- `chara::npc::Core` (row, `Char`, `EntryObj`, `affect_type`, `Acts`,
  `Touch` with its one `collide`), held by `Merchant` and `Dog`; the
  merchant's id, name, flags and message table are its row's.

Each step was behaviour-neutral. Checked on the tip: the towns', fields',
story maps' and world harnesses (15 tools, four volumes) 60/60; the golden
shots against the morning's base, 4550 files, none differ; the battle pins,
290 calls, none differ; the piney-game suite 346 passed, 152 ignored; the
crate tests, gen, clippy and fmt clean.

**Still unknown:**
- which id the events and menus find a grown Grunty by: the port names it by its first row while `ccPGuso` changes its base as it grows, and `GetNpc` matches the base's +0x0c
- whether any NPC effect reads `char_dirc`, which piney-game's effect host answers only for raw scene indices (a town NPC's handle gives zero)
