---
number: 588
title: One world, leg 11: leg 10's field changes checked against the game, the façade gone, one HostWorld
date: 2026-11-13
area: world, engine, decomp, test
files: crates/piney-world/src/world.rs, crates/piney-world/src/world/cast.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs
started: 2026-11-04T03:41:02-07:00
took: 235h 12m
---

# 588. One world, leg 11: leg 10's field changes checked against the game, the façade gone, one HostWorld

Leg 11 of the relay on the plan's "One world, one area system" item
(branch `one-world`, after [[587]]). It first checked what leg 10 changed
outside the towns, then did two of item 6's three parts.

The `took:` above is not the time worked. It includes a nine-day outage:
the weekly limit stopped the leg on 2026-11-04 after its second commit,
and it resumed on 2026-11-13 only to write this entry. The clock was left
as it ran, not backfilled.

## Leg 10's changes outside the towns, at 477cf7f

The battle pins hold: all 290 (the eleven recordings under scratch/pin)
replayed byte for byte against the worktree's own `battle_probe`. That
probe drives piney-battle alone, so ccSys.count's new start cannot reach
it. The full harness (fifteen tools on all four volumes, 60 runs) gives
every line as the baseline's.

ccSys.count (+0x358) from power-on is the game's. INF
`ccSystem::Init` (SLUS_202.67:0x0010a900) zeroes it once (0x0010a944),
`ccSystem::Ctrl` adds one (0x0010a6c4), and `main`'s loop (0x0015a804)
wakes the task thread, sleeps until the tasks are done, then calls Ctrl
(0x0015a820). A scan of main and GCMN.PRG for stores at +0x358 finds no
other store to ccSys.

The party lit by last frame's lights is the game's order too.
`ccModel::Draw` (0x0013eab0) calls `ccDrawEnv::SetLightMatrix`
(0x0013ec14 -> 0x00105900). That walks ccSys's light group through each
light's CheckRange when the draw is made, and the packet is built right
after. Kite draws inside `ccPlayer::Main` (GCMN.PRG:0x00598988, task 49)
and each member inside `ccFellow::Main` (0x0041ba90, task 50). Both run
before the entry control (64: the symbols', springs' and enemy weapons'
lights), the boss (66) and `ccThEffect` (80) move theirs.

Golden shots: all 4548 against main's goldens (world2/shots/base). 947
files differ by bytes. The 14 in field_gims_shots have the same pixels;
only the PNG encoding differs. To split the rest, the field dirs were
shot two more times: at leg 9's tip (289ebd0, tag l9tip), and at 477cf7f
with the old per-mode count put back uncommitted (l11cnt). The
comparison is pixel by pixel (scratch/world3/shotdiff.py).

- Leg 10, the count's phase (l9tip to l11a less l9tip to l11cnt): dungeon
  27, kyvia_stage 4, ocarina_lake 4, mirror_world 1, dressed_room 34,
  macha 23, field_shots 48. These come from regeneration ticks, the
  bosses' count&1 and count&3 picks, the ambient's odd frames and the
  rand() draws after them.
- Leg 10, the rest (l9tip to l11cnt): skeith 58, field_shots 22,
  field_type 2, dressed_room 2, macha 12. Every changed box sits on Kite
  or a member, with a channel delta of at most 1 to 17. Two macha shots
  lose a weapon trail while the menu sleeps the tasks. The causes are the
  lights a frame behind and no trail asleep: `ccSpcChar::ArmsEffect`'s
  Disp runs from the party's Main.
- Earlier legs (base to l9tip, never shot on the field before): breath
  20, event_11_church 19, field_type 13, field_shots 133, event_11 3,
  event_11_warp 25, macha 2. These are put down to leg 8's particle task
  after the disp, leg 7's ground attribute for ccEffPawSmoke and One
  Kite, going by those legs' notes. They were not bisected.

Found and left alone: the port lights enemies with this frame's lights.
In the game they draw at 64, before the boss and the effects move
theirs.

## Item 6: the façade's methods, then the façade

Commit 6cb113b makes the façade's town methods generic. `Cast` has
`affect`, `ask` (`world::cast::Ask`: the Flag Race's start, quit and end,
the breeder's writes), `event_entry`, `holds_events` (MUT main
0x001cb154), `seen` (`world::cast::Seen`: what the menus read) and
`map_entries` (the race's marks go in `map::Entries`). `World` has
`affect`: a battle character goes through `entry_affect`, the rest to the
cast. It also has `ask`, `seen`, `event_entry`, `holds_events`, `remove`,
`skip_fade_out` and `set_merchant_camera`.

`remove` now takes type 2 out of the party in every area. Before, a
field did nothing for it. ccEvent::Execute (0x001aa290) does not test
the area.

registNpcNum is now the world's, since every area's file list calls
ccRegisterEventMng (0x00169690, 0x001697a0, 0x001698b0).

There is one set_field_seed. SimGenerateCode (0x0019e5c0) moves randcnt
on by 13 on the one fieldrand: the place's, else the one the save
carries. The world's old version left the save's count alone.

A grown Grunty's line goes out as `Out::Chat`. The world's town
downcasts (`with_town`, `town`, `town_mut`) are gone. Tests reach the
town's cast through `<dyn Cast>::get` (`town_cast` in session.rs's
tests).

Commit ca94cdc removes the façade. `piney_world::World` is now the one
world; the town is built by `World::log_in` and `World::enter_town`. The
town's HostWorld impl is gone, and area.rs's answers every area:

- ccGame is the world's scene. The town mode had its own copy, a log-in
  model (InitScene, then ChangeArea(0, lastTown)), which was wrong for a
  town entered from a field.
- A script's ChangeArea in a town is the world's change of scene. Before,
  it pushed an event the session dropped for the town stage.
- The file list follows ccGame.area.
- The menu's noise reads WORLD_MAN.hackFlag. The town's set-up sets it
  as WORLD_MAN::GO (0x0019f8e0) does: 2, or 3 in the crisis.
- charTbl names, camCtrlType, `World::remove`, and the area's spc_item
  and clear_gate_hack apply in towns too.

`world::tests::event_2_plays_to_the_scene_change` now logs
`clear_gate_hack (gtHackFlag 0)`.

At ca94cdc piney-world's release tests all pass. The piney-game suite is
346 passed, 0 failed, 152 ignored. The town shot subset is 51 of 51
identical to l10d. The touched harnesses (thirteen tools, all four
volumes, 52 runs) give every line as leg 10's.

**Still unknown:**
- the merge of WorldMode and AreaMode into one mode (item 6's last part:
  one Stage, one menu view in place of ui_world and hud_world, one
  ccEntryEventMng flow, the town's SqLoad(2) mapping in
  WorldMode::take_events deleted)
- which earlier leg moved event_11_warp_shots against main's goldens (a
  town; its shots are timed from Kite's act 12), to be bisected before
  the goldens are made again
- whether spc_item's game-only view (the area's, now every area's) is
  enough for AddSpcItem in a town, where the town's full menu view was
  handed before
- enemies drawn under this frame's lights, where the game lights them
  before the boss (66) and ccThEffect (80) move theirs
