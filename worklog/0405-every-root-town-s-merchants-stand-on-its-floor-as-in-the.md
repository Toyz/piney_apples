---
number: 405
title: Every Root Town's merchants stand on its floor as in the game: the merchant harness had never registered the town's mesh, and now checks every merchant in every town
date: 2026-10-08
area: world, test
files: tools/test_world_rs.py, UNKNOWNS.md
---

# 405. Every Root Town's merchants stand on its floor as in the game: the merchant harness had never registered the town's mesh, and now checks every merchant in every town

[[397]] found Carmina Gadelica's merchants 15 and 16 higher in the port
than in tools/test_world_rs.py's `MerchantRun` (z 2.0 and 1.4e-4 against
0). It asked whether ROOTTOWN03 registers its Hit chunk otherwise than
ROOTTOWN01, and compared Carmina's Event NPC alone.

## What it was

- **Not the game's set-up.** On Mutation, `setMerchant`, `ccMerchan::ccMerchan`
  and `entryObject`/`initObject` set a merchant down with
  `ccLandHitCheck(pos, 0x20000002)` (MUT gcmn 0x00520910-0x005210b0), as
  Infection does. `ccLandHitCheck`, `_ccHitCheckLM` and `collisionLM`
  differ only in addresses. Each town has one Hit chunk (`HIT_sr2town1hit`,
  `HIT_sr3flo1hit`, `HIT_sr4flo1hit`), which the port reads whole.
- **The harness had no floor.** `MerchantRun` decodes the town's Hit chunk
  and enables a `ccModelHit` on it, through `ccModelHit::HitEnable`. But
  `Pieces`, its base, stubs `HitEnable` for the static pieces. So the mesh
  was never on the list.
  - Over a grid of 357 points in Mac Anu and in Carmina, the game's
    `ccLandHitCheck` found nothing.
  - Every merchant kept its dummy's z. Where a dummy lies on the floor
    (Mac Anu, Dun Loireag) that is the same.
  - Carmina's `DMY_merchant5` and `DMY_merchant6` (z 0) are a little
    under it: the port gives 7.6e-6 and 2.0.
  - So is the fourth town's `DMY_merchant6` (z -340): the port gives -338.
- **The port was right.**

## Fix

- `MerchantRun` lets the real `HitEnable` run for the town's mesh.
- It and `TownPcsRun` store the town's stream at ROOTTOWN01's +0x1a4 as the
  disc lays it out ([[404]]).
- **`test_every_town`** (new) runs `ccSetMerchant(0)` in each of the four
  Root Towns beside `set_merchants`: every merchant's id, name, position,
  posP, headings, idle and body. Then 300 frames of `routine` and
  `ccMerchan::main` beside `step_all`, Kite walking about the town.
- `test_event_npc` compares Carmina's merchants with its Event NPC again.

## Checked

- With the mesh registered, all 23 merchants of Mutation's four towns
  match the port at the start, 15, 16 and 22 among them.
- `MerchantsAgainstGame` passes on all four volumes: `test_merchants`,
  `test_event_merchant`, `test_every_town` and, from Mutation on,
  `test_event_npc`.
- tools/test_world_rs.py passes whole on Mutation; `cairns check` and
  `tools/docs.py check` pass. No Rust changed.

**Still unknown:** nothing.
