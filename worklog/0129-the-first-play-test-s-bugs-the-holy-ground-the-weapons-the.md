---
number: 129
title: The first play-test's bugs: the holy ground, the weapons, the fades, the transfers
date: 2026-09-25
area: engine, render, world, save, tooling, test
files: crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/main.rs, crates/piney-game/src/town_fx.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/body.rs, crates/piney-world/src/chara.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/town_party.rs, crates/piney-world/src/field_world.rs, BUGS.md
---

# 129. The first play-test's bugs: the holy ground, the weapons, the fades, the transfers

The first long play of the port through the tutorial left a list of
bugs in `BUGS.md`. This entry covers the ones fixed so far.

## Hidden Forbidden Holy Ground stayed black

After the Chaos Gate's warp from Mac Anu, the holy ground stayed black
for about three minutes. The tests got through because they allowed 30000
frames.

- **Cause.** `ccGame::ChangeArea` goes through `ChangeRequest`, which
  calls `ccDisableThEvent`. The town's own scene changes (the gate's
  `GoToArea`, `ChangeArea`) did not disable the event task. It stayed at
  its play pass, and in the next area it ran event 11's block 20 (phase
  4 or later, the arrival talk) during the hold. Then the set-up's pass 0
  could not finish while the talk waited on a message, and it held for
  `PASS_FRAMES`.
- **Fix.** The town disables the task on both requests. Block 18 (the
  party's `pc_mode`, phase 0) now runs in its own pass as well. The
  arrival test wants the set-up done within 200 frames.

## The weapons

BlackRose had no sword. Only Kite (always the Amateur Blades) and the
walking PCs had weapons.

- **What the game does.** `ccPlayer::ccPlayer` calls
  `ccSpcChar::EquipWeapon` (gcmn 0x0059d650). The file comes from
  `ccGetJobWeaponParam(job, weapon)`'s `ccsname` (`spcParam` +0xd8 and
  +0xd0). By job, the models go on the hands (+0x12c `OBJ_t0 l hand`,
  +0x130 `OBJ_t0 r hand`):
  - job 0: `MDL_<file>r` and `MDL_<file>l`
  - job 3: `MDL_<file>` on the left hand
  - other jobs: `MDL_<file>` on the right hand
- **Port.** `Body::equip_weapon` and `body::weapon_of` do this. Members
  get it in fields and towns, and Kite reads his equipped blades.

## The fades froze nothing

Walking into the church showed Kite already standing in the nave, in
front of the statue, before stream 8 started.

- **What the game does.** `ChangeRequest(6, 7)` calls
  `ccLayer::OffFlipExcept` (main 0x00108830). Every layer except the
  fade's gets its off-flip count raised, and `ccLayer::AddAll` resends
  that layer's previous list. So the fade darkens the picture from the
  frame before the request. The tasks keep running but are not seen, and
  the door's `ChangeBlock` has already moved Kite into block 1.
- **Port.** The session keeps the last frame of The World and draws its
  fade over that frame.

## The transfers

Towns dropped `effTransfer`, so the gate's leave and every arrival had no
rings.

- **Towns.** `town_fx::TownFx` now runs a `ccEffectCtrl(0)` for every
  town. It starts transfers for Kite (from `ccPlayer::AnimCtrl`'s acts 12
  and 13), the members (`ccFellow::Action`) and the walking PCs
  (`PcEvent::Transfer`). It steps the effects on the town's `rand()`,
  plays their `ccSeOn3D`, and draws them.
- **Fields.** A member's transfers are now started as Kite's were.
- **Streams.** The event and Data Drain streams get `StreamEffects` (see
  the stream entries), so stream 7 shows Kite's arrival rings.

## Play time and replays

- **Play time.** `main` calls `ccAddPlayTime` (main 0x00167740) every
  frame: `saveData.playTime` plus the frame rate, up to 0x0cdfe5c4. The
  session now adds it on the desktop, the board and The World. The only
  thing that stops it, `gameCntStop`, is set only inside a card write.
- **Replays.** `--pad-log FILE` also keeps the card as it was at the start
  (`FILE.card`). `--replay FILE` plays the log's pads back from a copy of
  that card, in the window or headless (`--shot`, `--every N`). The same
  log gives the same frames.

## Checked

- The event 11 shots, and new ones: `event_11_church_shots` (the church
  from the door, optionally from a card save, with buttons mashed or
  attacking at the door) and `event_11_warp_shots` (Mac Anu's leave).
- `play_time::the_world_counts_play_time`.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **The black screen in the church.** It came with Kite in act 15 (an
  attack) during the Balmung and BlackRose part. It is not reproduced:
  plain play, X or START mashed, attacking at the door and the play-test's
  own card save all play through. A pad log of the run is needed.
- **A merchant's `sysopeAct` transfer** is not started.
- **The stream effects** are drawn but not yet checked against the game
  (`tools/test_effect_str_rs.py` is still to be written).
