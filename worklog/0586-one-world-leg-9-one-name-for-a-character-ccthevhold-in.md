---
number: 586
title: One world, leg 9: one name for a character, ccThEvHold in every area
date: 2026-11-04
area: world, ui, engine
files: crates/piney-world/src/world/char_id.rs, crates/piney-world/src/world.rs, crates/piney-world/src/world/cast.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/event/talk.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs
started: 2026-11-04T01:43:39-07:00
took: 51m
---

# 586. One world, leg 9: one name for a character, ccThEvHold in every area

The ninth leg of the "One world, one area system" relay (after [[585]]) gave
every character one world-level name and ran the event's hold task in every
area. The game names a character by its `ccChar` pointer: `ccCheckTarget`
(INF GCMN.PRG:0x00519920) walks `cmndPcRoot`, `cmndEneRoot` and
`cmndObjRoot` for it, and `ccCalcTagPosChar` (0x0051a500) reads its +0x40
with no `ccTransPosFW2LW` (that function's 38 callers are effects, enemies
and `cameraSet`, none of the HUD). The Chaos Gate is no special case in the
game: `ccSetChaosGate` (0x00458df0) puts it with `g_entCtrl->entryObject`
(0x00458e98), an object on the entry control's object list like any field
gimmick, and `ccChgate::main` (0x00459280) never touches the command lists.

## One name

`piney_world::world::CharId` says which list holds the character, and the
world resolves it:

| variant | list | u32 handle |
| --- | --- | --- |
| `Member(id)` | the party, by `charTbl` row (on the battle scene in every area) | `1 << 24 \| id` |
| `Scene(i)` | the battle scene's other characters | `2 << 24 \| i` |
| `Ride` | the battle's riding Grunty | `3 << 24` |
| `Object(row)` | the cast's object list, by `gimmickTbl` row (a town's gate) | `4 << 24 \| row` |
| `Npc(code)` | the cast's NPC list, by `ccEvent::GetNpc`'s code | `5 << 24 \| code` |

The ride's handle is the field's old `RIDE_HANDLE` value, now a variant, and
the NPC's is [[585]]'s `own_ref` value, which the effects now take from
`CharId::Npc`. `Targeting` (target, prev, the sorted chain, the remembered
flags), its candidates (`Cmnd::who`), the action, `Ctrl::Attack`,
`Host::skill_request` and `TalkRequest::Menu` / `Event` all hold a `CharId`.
`World::listed` (`ccCheckTarget`), `char_place`, `tag_pos`, `chat_point`,
`base_id`, `char_info`, `command_target_ref`, `target_alive`, `scene_index`
and `command_target_id` are one method each; the cast answers for its own
through `Cast::own`, which `BattleCast` leaves empty and `TownCast` fills for
its NPCs and the gate. Gone: the town mode's `handle`/`unhandle`, the field
mode's `RIDE_HANDLE`, `target_of` and `target_index`, the façade's six
lookups and `TownCast`'s.

Decisions: a scene character's place is the field's copy nearest the
player in every area (no member of a town is ever across a seam); the gate
is listed while near, as the town's candidates already had it; a member's
type flags are its scene character's, the same row the town read from the
save.

## What the one `listed` exposed

The six `mac_anu_chat_shots` (frames 2240-2340) differ from leg 8's: the
party's balloons in Mac Anu no longer show. `ccChatMsg::Disp` (INF
SLUS_202.67:0x001a63c4) calls `ccCheckTarget` on every speaker before its
tag, so an unlisted speaker gets no balloon, and the port's town never puts
Kite or his members on the battle scene's party list: a debug run found
Kite (scene 0) and Orca (member 15, scene 1) unlisted at every frame. The
game puts them there in every area: `ccPlayer::ccPlayer` (GCMN.PRG:0x00598098,
`ccEntryCmnd` unless `CheckBootStatus` & 4), `ccFellow::Initialize`
(0x0041b554) and `ccSPC::Wakeup` (0x005a04a4). The old town code answered
"the party is always listed", which hid the gap. With the party counted as
listed (an experiment, not committed) all six shots match leg 8's
byte for byte, so this is the only shot difference. A first suspect, a
walking PC's code failing `NpcRow::of` (its code is its base id, not an
`npcTbl` index), was not the cause, but `Own::name` is now optional so a
balloon needs no table row.

## ccThEvHold

`ccEvent::Execute`'s `hold` case (INF SLUS_202.67:0x001b0a00) starts
`ccThEvHold` at priority 33, when the level is at least 2 and none runs,
with no area test. A town's host now takes `hold`/`hold_end` as a field's
does (`HostWorld::hold` has no do-nothing default), and the town's frame runs
the task after the event camera, as `World::frame` does.

## Checks

Commits 9ef137e, bd58bb3, 6fe9414 on branch `one-world`, each passing the
build checks. piney-world release: 115 + 3 + 1 + 12 pass (the handle test
`handles_are_distinct_never_zero_and_round_trip` moved here); the piney-game
suite 346 passed, 0 failed, 152 ignored (leg 8's 347 less that test, and
`chat::mac_anu_players_chat` now matches on `CharId::Npc`). The town and map
harnesses (`test_town02_rs`, `test_town03_rs`, `test_world_rs`,
`test_party_rs`, `test_grunty_rs`, `test_merchcam_rs`, `test_race_rs`,
`test_evarea_rs`, `test_field_rt`, `test_evchar_rs`, `test_evcam_rs`,
`test_area_rs`, `test_gate_out_rs`) give every line as before on all four
volumes. The town shot subset: 45 of 51 identical, the six above.

Not done this leg: the rest of item 5 (one frame: one `ccThGameCtrl`, the
draws, the ride's race view, one `ccSys.count`) and item 6 (one mode, one
`HostWorld`, the façade). The differences found for them are written down in
the relay's handoff notes.

**Still unknown:**
- where the port's town should put Kite and his members on the party's command list, as `ccPlayer::ccPlayer` (GCMN.PRG:0x00598098) and `ccFellow::Initialize` (0x0041b554) do, so that their town balloons and menu-21 targets come back
- where the game increments `ccSys.count` each frame; the port keeps it three times (the world's, in `ccThGameCtrl`'s slot and never in a town, and each mode's)
- how one `ccThGameCtrl` gets the menu wait a town ends with `close_menu` and a field ends with `menu_type`, which the town mode never sets
- whether a walking PC's HUD name should come from its own row rather than `NpcRow::of(code)`, since its code is a base id
