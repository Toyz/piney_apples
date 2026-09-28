---
number: 113
title: "Skeith in the field: the boss entry, its task each frame, and what it asks of the area"
date: 2026-09-25
area: battle, world
files: crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/skeith.rs, docs/engine/boss.md
---

# 113. Skeith in the field: the boss entry, its task each frame, and what it asks of the area

[[109]]'s rules and [[112]]'s arena, put together: event 30's `entry 7 0`
now makes Skeith in field 1, and he fights.

## The runtime

- **The entry.** `ccEntryEventMng` hands type 7 to `ccBossEntryStart(0)`.
  In the port, `FieldWorld::entry_setup` sees an `entry 7 0` among the
  event's entries after the entry control's set-up. It loads the boss's
  look (`x11`'s `CMP_trall` and every clip of `x11` and `xeffect`), and
  `Combat::start_boss` builds `ccBoss01` over the battle's scene:
  - the boss is a `Char::foe` of `bossTbl` row 0, its affects going to
    `AffectFunc::Boss`;
  - its centre is the arena's `DMY_center01`, and it stands 500 below Kite,
    facing his way;
  - `BattleData` now reads Skeith's tables from `GCMN.PRG`.
- **The frame.** `Combat::frame` runs the boss's task after the entry
  control, in the game's task order (64, then 66, then the effects at 80).
  The actor takes the boss's clip and frame, place and heading,
  `setTransparency`, and whether it is drawn. The boss joins the enemies
  Kite's frame targets and the HUD lists.
- **Affects from other frames.** The game calls `bossAffectFunc` from
  inside Kite's and the members' frames. Their contexts in the port build
  their own `AffectCtx` without the boss's tables. `boss::entry` with no
  `BossEnv` now queues the affect (type, params, who) on the boss, and
  `boss::apply_queued` runs them in order at the start of its task, the
  same frame, before `Main`. Two hits in one frame keep their order.
- **What it asks.**
  - `Out::Skill` now carries its target and runs through
    `Combat::item_skill`.
  - Hit marks and damage numbers become rule events for the effects.
  - Sounds (`ccSeOn3D`, `ccSeOn3DNote`, `ccSeOn`), `scFadeDef` flashes and
    `EVENTAREAB0::SwitchLayer` reach the area mode (`boss_calls`).
  - A `DeleteCmnd` takes it off the enemies' list.
  - Everything else is shown as `Show::Boss` and waits for its reader.
- **The event.** The area host's `boss()` answers `present 7` / `absent 7`:
  task parameter 0 while it fights, 1 once it has exited.

## Checked

- **`session::skeith::skeith_fights_in_its_arena`.** Field 1 is set up as
  area 27's door leaves it (area 27's `WORLD_MAN`, town 1), with
  `entry 7 0`. Over 900 frames the boss exists and runs its patterns:
  neutral, the cross, the chase. The host reports it fighting.
- **`skeith_shots`** (ignored) writes every 30th frame. The shots show:
  - Skeith in the arena, targetable as "Skeith" with his HP;
  - BATTLE MODE ON;
  - the cross swung at a new game's Kite, who does not survive it at
    level 1.
- **`session::skeith::event_30_ends_with_skeith`.** Event 30 plays end to
  end in the arena: the VM of `--mode story:30` with blocks 0-19 marked
  played and `eventStatus[0]` 4. Block 20's accumulated settings need
  both, carried from block 3's `block_done 1` and block 12's status 4.
  - Block 20 runs `battle_ready`, block 21 (phase 2) plays streams 14 and
    17, then `entry 7 0` makes the boss.
  - Kite has 9999 HP. The drain's 13 and 21 are put on the boss while
    the player is free (the menus cannot open under `LockPlayer`), then
    hits of 800.
  - The Epitaph (act 12) follows, then death (act 14) and the exit.
  - Block 22's `if absent 7` runs, and its `mode 3` takes the session to
    the desktop.
  - The desktop runs event 31 (ENDING):
    - stream 15, about 5160 frames;
    - the closing lines, pressed through;
    - the desktop with block 1's `staff_roll`, about 7230 frames;
    - then the desktop again, the game cleared.
  - The desktop's title now says when the staff roll runs.
  - An earlier run drained Skeith in the middle of his magic, which the
    menus cannot do. That left `lockPlayer` set and the party held for
    good. A level 1 Kite dying makes the party count as annihilated, which
    leaves a -1 wait in force and the boss standing.
- **`session::skeith::skeith_drained_through_the_menus`.** In the same
  arena, Kite gets the bracelet and Data Drain and the boss's protect is
  broken. Kite walks up while the boss is free and plays the menus:
  PERSONAL, Skills, the Data Drain page, the target page (65), menu 66.
  The boss takes the drain and its Epitaph begins.
- **The boss's menu locks.** `LockPlayer`'s `MenuForbid` and the cursor
  now reach `ccMenu`'s `forbid`, `forbidChatExcept` and `cursolOff`.
- **The body hit.** `bodyHit` is on the collision list at the boss's
  place:
  - `ccBoss::Move`'s `SetHitSW(bodyHitSW)`;
  - its `CollisionDetection` answers the rules' `collide`;
  - its copy follows the boss;
  - it leaves the list at the exit.

  `kite_bumps_into_skeith` walks Kite at the boss for 600 frames, and he
  stops 200 away instead of passing through.
- **Menu 74.** `StreamMenu` (gcmn 0x00535340) is ported
  (`piney_fieldui::menus::stream`):
  - the fade, then the stream;
  - `ccThStrParty` (0x0056ab00, `piney_world::foe::StrParty`): each
    flagged member's `CMP_trall` in its `spcAnmTbl` pose at the stream's
    `OBJ_dmy_spcN` (`piney_stream::scene::Scene::world_named`, new);
  - the menu's close.

  The boss's `StreamMenu` output opens it. `skeith_drains_a_member` gets
  the gauge past half and lands a hit, so the Super table's drain (act 5)
  opens menu 74. Stream 20 plays with the member drawn
  (`str_party 0x1`), the menu closes and the boss goes on.
  `member_drain_shots` (ignored) shows Kite hanging in stream 20 while
  Skeith drains him.
- **The picture.**
  - `ccBoss::Draw`'s blend: `ccChar::Draw`'s hit flash and condition
    tints, but none while off the command lists. The boss's
    `conditionNum` now starts at `ccChar::ccChar`'s -1; with 0 Skeith
    wore the first condition's purple.
  - The after-images (`EntryAfterImage`, `ccBossAfterImageEx::Draw`
    0x0045b980): the pose when entered, white (`m_eai` colour -1 at 100),
    from 1.0 by -0.05 a draw on effLayer.
  - The wave's `DrawParts` (`Out::DrawWave`, new): `xeffect`'s
    `ANM_xx11wave` objects at the boss on objLayer.

  - The stage fader (`InitStageEffect` 0x0045eac0, `BeginStageEffect`,
    `EndStageEffect`): `scStageFade` on `stageLayer`, priority 2, above
    the stage and under the characters. `EntryFlash3(t0, t1, t2, rgba)`
    for Begin. End deletes a running element and, with `t`, flashes out
    from `rgba`.

  The shots show the dash's white trail, the wave's burst under Skeith and
  its damage on Kite. The magic dims the arena, and Skeith hides for the
  IceBreak (`OffDraw`).
- **The rest.** The rules' harness still matches with the target added
  to `Out::Skill`. The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **The pictures.** Not drawn:
  - the cross's trail (`DrawCross`);
  - the six effects;
  - the reversed layer (`ccBufferReverce`) and the quake (`QuakeCam`,
    with the boss camera).
- **`ccBossCam`** and the dead camera are not ported: the field's own
  camera follows Kite.
