---
number: 397
title: Mutation's walking PCs: the later class, its SEARCH PCs and the Flag Race's hiding, and the Event NPC
date: 2026-10-08
area: world, volumes, build, test
files: crates/piney-world/src/rtownpc.rs, crates/piney-world/src/merchant.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/event.rs, crates/piney-world/src/race.rs, crates/piney-world/src/talk.rs, crates/piney-world/src/town.rs, crates/piney-world/examples/world_probe.rs, crates/piney-world/examples/merchcam_probe.rs, crates/piney-gen/src/rtownpc.rs, crates/piney-gen/src/race.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/lib.rs, crates/piney-data/src/tables/world.rs, crates/piney-data/src/pack.rs, crates/piney-game/src/world.rs, crates/piney-game/src/town_fx.rs, crates/piney-game/src/session/tests/side_events.rs, tools/test_world_rs.py, tools/test_grunty_rs.py, docs/engine/field-game.md, docs/engine/flag-race.md, plans/outbreak-side-events.md, UNKNOWNS.md
resolves: 393
---

# 397. Mutation's walking PCs: the later class, its SEARCH PCs and the Flag Race's hiding, and the Event NPC

[[393]] left two later-volume pieces open: Mutation's `ccRtownPC` and the
"Event NPC" that `ccSetMerchant(0)` makes from Mutation on. Both are
ported here, against the game's code on all four volumes. Outbreak's and
Quarantine's class is recompiled (other registers, the same shape).

## What Mutation's class does differently

- **Who walks** (`ccRegisterRandomNpc`, MUT main 0x001ccdf0). [[393]] did
  not list this one. The draw is no longer Infection's:
  - server 1 and 3 have 2 fewer PCs, 2 has 6 fewer, 4 has 10 fewer;
  - the first three slots draw `|ccRand() % 50|`, the rest draw `% 56`;
  - rows 50-55 of the 56 are 180, 181, 183, 182, 120 and 121 (MUT main
    0x0032ef10): the sign PCs, Sieg and Kaz. Each is open once its event
    is done (300, 358, 315), else taken.

  Before the fix the harness's `test_select` differed on Mutation from
  the fourth slot on.
- **The model.**
  - `rtpcCcsName` has 50 files: `ctwmk` (Kaz) is new, and `cbuz` is now
    `ctbuz`.
  - [[393]]'s "texture tables" are the TV PCs' weapon tables: rows 159-179
    read 0x0061c250 and 180 on read 0x0061c280. The generator had carried
    Infection's `tvpcWeaponName` to the second table, so rows 159-167
    took the sign PCs' weapons.
  - Type 35 holds its weapon in the left hand. Type 49 holds one in the
    right, as 48.
  - The body is kind 8 (merchants' 16). Kite's `HitCheck`, the same code,
    tests kind 2, so it no longer flags a PC he touches.
- **The Flag Race** (`ccSnd +0x13a`).
  - A PC placed in an even slot of 4 or more gets +0x2d7.
  - While the race runs, such a walker goes after its move to act 9
    (MUT gcmn 0x00524320): off the character and command lists, hidden.
  - It walks again when the race ends.
- **Event mode.** `npc_act 3`'s fade-in starts with `effTransfer`.
- **The SEARCH PCs** (`entry 3 row marker -1`, rows 159-167, Outbreak's
  events 267-275).
  - The set-up (MUT gcmn 0x00521e50) reads the row's stage from
    `eventStatus[row - 117]`, the status the events step (BT's is
    index 48).
  - `markPos` comes from `search_marks[row - 159][stage]` (0x0061c890).
  - Below stage 5 the PC starts at landmark `markPos[0]` and routes on.
    From 5 it stands at dummy `search_dummies[markPos[0]]` (0x0061c8c0),
    facing the dummy's way for the first three. The marker is not used.
  - The constructor makes a TV PC glimmer (0.3) and walk at 3.5.
  - It runs `searchMode` (0x005243f0) in place of `normalMode`. The
    walks, shops and gate are the same, with these differences:
    - it walks with `anmTbl[1]`;
    - it stays off the character list and collides with nothing;
    - it greets no one and makes no step sounds;
    - its `transrate` is `0.3 sin`, at least 0.1, of an angle turning 256
      a frame.
  - `npc_act 2` turns it to Kite and brightens it to 0.8.
  - `npc_act 5` takes it off the lists and calls `effOpenBox` at its
    place. At stage 5 it also calls `effVirusCrystal` and
    `ccSeOn3DNote(167, pos, 40)`. It then fades through `routine` (30
    frames) and hides after 33.
  - `npc_act 6` hides it.

## The Event NPC

- **When it appears.** `ccSetMerchant(0)` (MUT gcmn 0x00520910) makes row
  175 + town after the town's merchants while `eventStatus[52]`
  (+0x652c) is set.
- **What sets the byte.** Event 317 "ITEM COMPLETE": it opens once 315 is
  done (a Quarantine event, on every disc) and sets the status when mail
  328 is read, in the desktop.
- **The row.** Flags 0x10000000, the town's `CTR1`-`CTR4`.
- **Where it stands** (`setMerchant`, 0x005207b4).
  - 175, 176 and 178 stand at `DMY_marker_ev04`.
  - 177 stands at `DMY_marker_ev01`, its rotation z written to
    `DEG2RAD(24576)`.
  - 179 stands at `DMY_marker_ev02`, x and y 0, z `DEG2RAD(-24576)`.
  - The writes go into the town's dummies, so markers 1 and 2 read them
    for the rest of the visit (`Base::written`, `World::marker_dummy`).
- **Its acts** (0x00521a00): the breeders' three. Once the status is clear
  it goes: idle, `effTransfer`, `deleteCmnd(1)`, a fade by 0.02. Then
  `main` returns 1 and the entry control deletes it.
- **Its affect** (0x00521990): 15 faces Kite from the talk clip's start, 14
  only turns it, 0 faces its way again.
- **Its talk.** Its action button opens menu 90 (MUT gcmn 0x00536aa0), so
  [[394]]'s "who opens 90" is answered. Menus 90 and 91 are not ported,
  and the port shuts 90 at once.
- **A deleted merchant's body.** `~ccCharHit` takes it off the character
  list. The port had left the Administrator's there after his act 5; now
  `Merchant::step` takes it off.

## Fix

- **piney-gen** (`rtownpc.rs`) finds the tables per volume through the
  code:
  - the constructor's `addiu -159` / `-180` give the weapon tables, and its
    `addiu $v0, $a0, -159` gives the SEARCH marks;
  - the set-up's next table is the dummies;
  - `ccRegisterRandomNpc`'s two moduli and its table give the extra rows;
  - `rtpcCcsName` and `rtpcAnmTbl` run to the null.

  New entries: `sign_weapon_names`, `search_marks`, `search_dummies`,
  `walk_extra_rows`. `DATA_VERSION` is 29.
- **piney-world**:
  - `rtownpc::Pool` and `register_random_npc`;
  - `pc_kind`;
  - `Origin` and `Search::set_up` with `RtownPc::place_as`;
  - `search_mode`, act 9, `TownPcs::race_hold` (set by the race's
    `ccPgBgmInit` and its end) and `event_status`;
  - `merchant::body_kind`, `EVENT_NPC`, `event_npc_act`,
    `event_npc_influence`, `dummy_of` / `dummy_turn`;
  - `talk::menu_for` gives 0x10000000 menu 90.
- **The town's Kite** takes the disc's volume (`player.volume`), as the
  fields' did. On Outbreak and Quarantine his camera fade used
  Infection's square root (1 ulp at frame 493 of the harness).
- **piney-game** shows the SEARCH PC's going (`effOpenBox`, the crystal
  and its sound).

## Checked

- **Against the game's code** (tools/test_world_rs.py, eemu, beside
  `world_probe`):
  - `TownPcsAgainstGame` matches on all four volumes. The harness reads
    the later class's offsets, and its new cases are:
    - `test_select` with servers 0-4 and every mix of the three events;
    - `test_frames` adds five SEARCH PCs at stages 0-5 in Mac Anu (spoken
      to, going with and without the crystal, hidden), the Flag Race on
      and off, server 4, and the extra rows open;
    - `test_search_dummies` sets up stage 5 in Dun Loireag and Fort Ouph,
      where the turned dummies are.
  - `MerchantsAgainstGame.test_event_npc` (Mutation on): the Event NPC in
    Mac Anu and Carmina Gadelica, spoken to, then going and deleted.
    `test_merchants` and `test_event_merchant` now also pass on Outbreak
    and Quarantine.
- **Harness fixes.**
  - The module loads on Outbreak and Quarantine. ROOTTOWN01's vtable is
    found through its constructor (+0x1b0 there), and the unnamed
    `AwakeDistantLight` is left to run.
  - Each PC's clips are read from its own file (Mutation's files share
    clip names).
  - The PCs' dummies carry their rotation.
  - tools/test_grunty_rs.py now matches on all four volumes, which
    answers [[395]]'s open point on the grunty harness. It finds the
    Grunty's `dogAction` as the function before `dogAction2`.
- **Played** (piney-game, Outbreak):
  - `search_bt_walks_from_its_stages_landmark`: SEARCH BT starts at its
    stage-0 landmark in Carmina Gadelica, walks at 3.5, glimmers, and is
    off the character list.
  - `search_bt_meets_its_npc_in_six_towns` (it was `..._at_six_gates`):
    the pilot catches BT in each town and the event ends at frame 12,543.
  - Shot (`search_bt_shot`, ignored): `/mnt/data/claude/scratch/i63/
    out-search-bt.png`, BT translucent beside Kite.
- piney-world's tests (`mutations_pool`, `search_marks`), piney-game's
  suite (four threads), clippy, fmt, `piney-gen gen --check`, `cairns
  check` and `tools/docs.py check` pass.

**Still unknown:**
- Menus 90 and 91 (the Event NPC's list and its item registry, MUT gcmn
  0x0058d470 and 0x0058d9f0, page 0x0058edc0) are not ported. They can be
  done in the repo with a talk-harness case.
- Two other classes of tools/test_world_rs.py fail on Mutation, from before
  this work and not read:
  - `PropsAgainstGame.test_town_draw` stops on a syscall in Mutation's
    `ROOTTOWN01::Draw`;
  - `WorldAgainstGame.test_frames_random` differs in Kite's camera at frame
    88 of "town01 random 0".
- In Carmina Gadelica merchants 15 and 16 land higher in the port than in
  `MerchantRun` (z 2.0 and 1.4e-4 against 0). That harness registers the
  town's Hit chunk as ROOTTOWN01 does; whether ROOTTOWN03 registers it
  otherwise is not read. `test_event_npc` compares the Event NPC alone
  there.
