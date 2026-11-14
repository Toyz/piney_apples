---
number: 580
title: One world, leg 3: Kite is the cast's, the lookups and party instructions settled from the game's code, one rand(), a town's outputs as one ordered stream
date: 2026-11-03
area: world, engine, decomp, audio
files: crates/piney-world/src/world.rs, crates/piney-world/src/world/cast.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/chara/party.rs, crates/piney-game/src/world.rs
started: 2026-11-03T19:43:29-07:00
took: 30m
---

# 580. One world, leg 3: Kite is the cast's, the lookups and party instructions settled from the game's code, one rand(), a town's outputs as one ordered stream

The third leg of the relay finishing "One world, one area system"
(`plans/rust-cleanup.md`, section 2), on the branch `one-world` after
[[579]]. It settled from Infection's code the questions [[579]] left
(where Kite is, the character lookups, the party instructions, the order
of a town's outputs), merged what those answers allowed onto the one
`world::World`, made one newlib `rand()`, and did W3: a town's
per-character outputs are one stream in the game's task order. The
façade (`piney_world::World` in `lib.rs`) went from 80 methods to 59.
Main gets nothing until the last leg's merge.

## What the game's code settled (INF)

**Kite.** `ccEvent::GetSpc` (`INF SLUS_202.67:0x001b2bb0`) returns
`ccSpcManager`'s `charPtr`: one `ccPlayer` in every area. The port keeps
two copies whose master differs: in a town the core's `player` (run by the
town; the battle's stand-in `combat.kite` follows it once a frame,
`Combat::mirror_leader`, before the members' frames), elsewhere the
battle's character (`player` its mirror, `sync_player` after the battle's
tasks). So `combat.who(0)` cannot serve both: the stand-in is stale after
a put or a move earlier in the same frame. The one rule is that the cast
answers for Kite: `Cast::kite_pos`, `Cast::kite_pos_p` and
`Cast::with_spc_chars` (the registry's characters as `SpcChars`, Kite the
town's player in `TownCast`, the battle's character by default). Every
lookup of a party character resolves to a `charTbl` id, and Kite (id 0,
or the scene index `combat.kite`) goes to the cast.

**The character lookups.** The game has two. `ccEvent::Execute`'s cases
59, 64, 72, 161 and 162 test `1 << type` (sllv, the low five bits): 4
`GetSpc`, 0x18 `GetNpc` (0x001b2cc0), 0x60 `GetEnemy` (0x001b2d50), else no
one. The event camera (`ccEvent::CamCtrl` at 0x001b3704, and Execute's
cases 23, 24, 27 and 30, found by mapping every call of these functions
to its case through Execute's jump table at 0x00355fd0) does the same and
otherwise calls `ccCheckTargetTypeId(1 << type, code)`
(`INF GCMN.PRG:0x005199e0`) on the command lists. That returns the
`ccChar` itself (the lists link through `ccChar` +0xbc; +0 is `base`,
`base.type` +0x08, `base.id` +0x0c; Kite's type is 7, the members' 6), so
types 0 and 1 name a listed party member. Both port modes were wrong: the
town took types 0-2 as `GetSpc` and -3 as Kite (`GetSpc(-3)` reads
`memberID[3]`, which is `num`), the field only type 2 and no enemies. Now
`World::char_pos` is Execute's lookup and the camera's is
`World::camera_char_pos`.

**trans_on / trans_off** (cases 161/162, 0x001b2248, 0x001b22f4) use
Execute's lookup in every area. The field's handled NPCs only (a port
bug: Kite, the members and enemies now too).

**near_marker's distance.** `ccEvent::CheckOpen` (0x001a7d9c) calls
`ccTransPosW2P` (`INF GCMN.PRG:0x0059b940`), which is
`ccPlayer::W2PPos` (`INF GCMN.PRG:0x0059b5a0`): it wraps through
`WORLD_MAN` +0x420..+0x42c in every area. A town's bounds are +-24000,
which `Combat::bounds(0, hits)` already gives, so one `player_distance`.

**The camera's markers.** `cam_look_marker` and `cam_pan_marker` (cases
25 and 32, from 0x001aaec0) branch on `game.area` (`ccGame` +0x14): 0 a
town's `markerEvTbl[marker]` dummy (+0x10 through `GetChunkAdrsF`), 1 and
2 the event's `evPos[16]` (eventMng +0x1c0, 0x20 bytes each, the number
at +0x04, the position at +0x10), anything else nothing. The field's was
"not ported"; it is now. `TownScene` and `impl evcam::Scene for World`
are one `EvScene { w, cast }`.

**close_menu.** The one world's clears `player.acts.pause` and the battle
Kite's PAUSE bit. Nothing in a town sets the stand-in's PAUSE
(`affect.rs` gives a character that is not a fellow RESTRAINT;
`mirror_leader` does not copy it), so it serves the town too.

**MenuBan / MenuClr.** `ccEvent::MenuBan` (0x001b2460) calls
`ccStoreSpcCondition` (0x001b24b4), `ManualModeAI` and `SetRemoteCmd` per
slot, `ccSpcConditionEffectOFF`, then `ccDeleteCmnd` and `ClearCondition`
(0x001b2580) per registered character, with no test of the area. The
town's `menu_ban_party` skipped the conditions and `MenuClr`'s two-frame
wait (a port bug); the field's body is the game's and is now the one.
`delete_fellows`, `party_remove`, `transfer_out` and `pc_mode` are the
same `Spcs` calls on both sides, differing only in where Kite is.

**The order of a town's outputs.** The task priorities (`ccThPlayer` 49,
started at `INF GCMN.PRG:0x0059789c`; `ccThFellowNN` 50; the Flag Race's
task 63, `MUT GCMN.PRG:0x005ff680`; `ccThEntryCtrl` 64; `ccThEffect` 80;
`ccThFieldDisp` 96; `ccSoundMain` 112) give the order sounds and effect
starts are made in: Kite's steps and running dust; the members'
transfers and lines; the race; the entry control's list (the gate's
`ccSeOn3D` 71/72, the race flags' 240 and box, the Administrator's
`Se(217)` and starts, the dogs, the Grunties, the walking PCs); the
effects' own sounds; the town's draw (Carmina Gade's airship smoke and
sound); the scene sounds last. The port had consumed them kind by kind
(the walking PCs, dogs, Grunties and the town's sounds before the
Administrator's `Se(217)`, Kite's steps after it, `SceneSound` before
the effects).

## The steps (each commit builds)

1. Kite is the cast's; one `char_pos`, camera lookup, `set_trans`,
   `player_distance`, `close_menu`; `EvScene`; `World::with_party` goes
   through the cast and brings the battle's `ccPartyManager` copy up to
   date after (the game has one).
2. The party instructions are the one world's: `delete_fellows`,
   `party_add` (`Cast::invite_new` builds a newcomer at a town's Chaos
   Gate, `inviteSpc` `INF GCMN.PRG:0x005a08e0`), `party_remove`,
   `transfer_out`, `set_boot_param`, `menu_ban_party`.
3. One newlib `rand()`: `crate::Rand` is `piney_battle::rand::Rand` (an
   inherent `rand()`), the town draws `combat.rand`; `TownCast.rand` and
   its copies around every party call are gone, and with them the town's
   `camera_shake`, `set_rand` and `rand_state`.
4. W3: `world::cast::Out` and `World::take_outs`. The town's eleven
   queues (PC, dog, Grunty, merchant events, gate sounds, Kite's steps
   and dust, the members' transfers and lines, the town's draw, the race)
   are one stream pushed as the frame runs; a Grunty's camera writes are
   split off and still applied after the list. piney-game's
   `WorldMode::step` takes the stream in order and plays the effects'
   sounds, then the town's own, then `SceneSound`.

The checks at each commit: `cargo check --workspace --all-targets`,
`cargo test --no-run -p piney-world -p piney-game`, `cargo test -p
piney-world`, and piney-game's town tests (`world::`, `kept_dummies`,
`town_return`, `race`, `grunty`: 46 passed). The two tests that asserted
single queues empty after a frame (`world::tests`' walk to the gate,
`kept_dummies`) now assert the whole stream is.

At the leg's end, release: `cargo test -p piney-world` 116 + 3 + 1 + 12
passed, and the piney-game suite 347 passed, 0 failed, 152 ignored, as
the baseline: no test pins the order of a town frame's sounds, so the
new order moved nothing the suite checks. The harness, golden shots and
audio pins (scratch/world2's checks) are the last leg's.

What is left, in order, is in the relay's handoff and below.

**Still unknown:**
- the town's two `ccRand`s: `town_pcs.mt` (seeded in
  `TownCast::place_entries`) and `combat.cc` (the party AI's) are one in
  the game; merging them, and the town's `save.rand_s` against the
  field's `combat.rnds`, needs `ccInitRand` (`INF SLUS_202.67:0x001d9900`)
  read for whether a town's scene resets `lastRnd` (blocks
  `state_out`, `with_live_state`, `cc`)
- one handle for a character in both modes (the town's kind and code,
  the field's scene index): blocks `use_item`, `item_step`, `chat_cmd`,
  `equip_report`, `manual_mode_ai`, `remote_cmd`, `manual_off`,
  `command_target_ref`, `target_alive`, `char_place`, `tag_pos`,
  `chat_point`, `char_info`
- the party's event instructions (`pc_command`, `npc_command`, `entry`,
  `remove`): the town's `chara::party` against piney-battle's `EvParty`,
  to be compared case by case with Execute
- one frame for both casts in the game's task order, then one mode and
  one `HostWorld`, the façade gone (W5); one effect host (W4); the town's
  `event_npcs` into the core's event entries (W6)
- whether `inviteSpc` builds a newcomer outside the towns (the port's
  field does not; the game's `ccGetChgatePosDirc` there is unread)
