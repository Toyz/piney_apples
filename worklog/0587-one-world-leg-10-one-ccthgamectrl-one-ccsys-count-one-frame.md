---
number: 587
title: One world, leg 10: one ccThGameCtrl, one ccSys.count, one frame for a town
date: 2026-11-04
area: world, engine, ui, test
files: crates/piney-world/src/world.rs, crates/piney-world/src/world/cast.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/event/talk.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/mode.rs
started: 2026-11-04T02:36:19-07:00
took: 1h 4m
---

# 587. One world, leg 10: one ccThGameCtrl, one ccSys.count, one frame for a town

The tenth leg of the "One world, one area system" relay (after [[586]])
made a Root Town step through the one world's set-up and frame: one
`ccThGameCtrl`, one `ccSys.count`, one task-ordered draw. It also found that
the party's missing balloons in Mac Anu, which [[586]] blamed on the town
never listing its party, are the game's own behaviour.

## The party's command lists: no change

[[586]] measured Kite and Orca off `scene.pc_list` at every balloon of
`mac_anu_chat_shots` and supposed the town built them unlisted. Traced frame
by frame, the town's constructors list them as the field's do
(`add_kite` / `add_member` with `bootParam & 4` clear: `pc_list [0, 1]` at
`Play(1)`), and event 11's first play pass then runs `menu_ban true`.
`ccEvent::MenuBan` (INF SLUS_202.67:0x001b2460) walks `ccSpcManager` and
calls `ccDeleteCmnd` on each built character (0x001b2570) with no area
test, and the first play pass comes after the set-up's `ccSPC::Reboot`
(event-vm.md: phase 4 is asked just before F0). Orca's AI line at 2230 then
has no balloon because `ccChatMsg::Disp` (0x001a63c4) closes a slot whose
speaker fails `ccCheckTarget` (GCMN.PRG:0x00519920, a pure walk of the three
lists). So the six shots of [[586]] are right and leg 8's party balloons
came from the old "party always" rule. The town shot baseline is now leg
9's `l9a`. Members are targetable in a town as in the game:
`piros_menu_shot` opens menu 21 at frame 86.

## One `ccThGameCtrl`

The game has one task for every area (GCMN.PRG:0x00517800).
`World::game_ctrl(pad, cast)` builds the candidates from the battle's three
lists and then `Cast::cmnds` (a town's listed NPCs, then the Chaos Gate
while Kite is near, its posP `ccTransPosW2P` of its place). The eye view is
`checkCameraType()`, `activeCamPtr->type`, so it reads the active camera
in every area (the field read `tcam`). The town mode now sets the menu type
as the field's does (one `menu_view`). An action on a cast NPC gives its
line or its shop through `Cast::npc`. The town's `Targeting::step` wrapper,
its `Town` host, `talk::Step`, `Battle::alone`, `Input::town` and the
`Input` fields only that wrapper read (`held`, `dead`, `skill_one`,
`player_ok`) are gone; the targeting tests keep them as test helpers. In
a town Kite is now off the list under `menu_ban` like everywhere else, so
no target is chosen then, as the game's `ccCheckTarget(plw)` before
`ccSelectTarget` (0x00517f10) decides.

## One `ccSys.count`

`ccSystem::Ctrl` (main 0x0010a6bc: `lw`/`sw 856($s1)`) adds one to
`ccSys+0x358` every frame from power-on in every mode. The port kept it
three times: the world's (+1 in `ccThGameCtrl`'s slot, never in a town),
`WorldMode`'s and `AreaMode`'s. Now the session's `sys_frames` is the only
counter, handed to every stage each frame (`Mode::set_sys_frames`, which
the desktop already had) and by the world modes to `World::set_count`; the
menus read `World::count()`. In fields the battle's `Env.count` and
`MapView.odd` now carry the power-on phase (regeneration ticks, the
bosses' `count & 1` / `& 3` picks, the fires' odd frames).
`mutation_skill_from_the_menus` failed on it: Staccatto hit the other
monster, the command target, and the test watched only the first; it now
sums every monster's HP.

## One frame and one set-up

`TownCast`'s own phases and frame are gone; a town runs
`World::step_into_with` and `World::frame`. Settled from the game:

- `ccSPC::Reboot` is `ccSetupGameCtrl`'s in every area, at the set-up's end
  before F0. The town built its party at F0; it now does so with the field,
  so F0's menu party check sees it. `ccEntryEventMng`'s town part (event
  NPCs, merchants, walking PCs) is the cast's `placed`, run by
  `entry_setup` at F0 when awake.
- The ride's slot (49) reads the Flag Race (63) as it last left it:
  `Cast::race_ride`.
- The characters draw in their tasks' order. The ride, Kite and the members
  (49, 50) draw before anything that moves a light (the gate at 64, the
  effects at 80), so they take the light group as the last frame left it.
  Their weapon trails go out from their `Main`: none asleep, and not Kite's
  while his task sleeps on a town's Grunty. The entry control's characters
  and the cast's (`Cast::draw`: the race's HUD, the gate, the flags, the
  merchants, dogs, Grunties, walking PCs and NPCs) come after, under this
  frame's lights. The gate's two lights join the group through
  `Cast::lights`.
- `World::map_view` gives a town's systems what the town's own `MapView`
  gave: only `field/ambient.rs` reads `eye1`, `rot`, `rot2` and `odd`, and
  only `lake_sky` reads `room`. `no_active` and `room_clear` are true over
  a town's empty lists.

Tests that encoded the old F0 reboot now expect Kite built at `Play(0)`,
arriving (`tests/world.rs`, `tests/dun_loireag.rs`), and Piros's first
sampled `actType` is `ccAI`'s constructor's 95 before 96, 99 and his walk.
The façade's `add_npc`, `npcs`, `npc_mut`, `grunties`, `dogs` and `gate`,
which no game code read, are gone; tests read the town's cast through
`<dyn Cast>::get`.

## State

Commits on `one-world`: 3e39323 (one `ccThGameCtrl`), 9855a4d (one
`ccSys.count`), 756b489 (one frame and set-up), 6e19e23 (façade getters),
then this entry. Each builds. `cargo test --release -p piney-world` passes
(115 + 3 + 1 + 12); the piney-game suite 346 passed, 0 failed, 152 ignored;
the town shot subset (`l10d`) is identical to `l9a`, 51 of 51.
The harnesses for what changed (the towns' five, test_world_rs,
test_party_rs, test_evarea_rs, test_field_rt, test_evchar_rs,
test_evcam_rs, test_area_rs, test_gate_out_rs and the rest of leg 9's list)
give every one of their 52 lines as leg 9's on all four volumes.

**Still unknown:**
- Item 6 is not done: one mode (`WorldMode` and `AreaMode`), one
  `HostWorld`, and the façade with its `Deref`. The façade still forwards
  about 24 town methods whose game users (the race's menu requests, the
  breeder's feeding, the merchant camera, shop stock, event entries) need
  generic requests to the cast; its `take_requests` maps the world's
  `SqLoad` and `GameStart` to the town mode's `SqLoad(2)` and `GameStart`
  until the modes are one. The town mode's own `ccGame` model
  (`FieldState.game`) against the world's scene is the first difference to
  settle.
- The field's pixels under this leg's rules were not shot: the event NPCs
  now draw after the party, no trails go out asleep, the party takes the
  effects' lights a frame late, and `ccSys.count`'s power-on phase moves
  the field's odd-frame and count-based picks. The last leg's full shots
  and goldens must show them.
- Whether a field's enemies (64) should also take some lights as they were
  before the frame's entry list moved them; the port lights everything
  past the party with the frame's final group.
- A mode driven without a session keeps `ccSys.count` at 0.
