---
number: 579
title: One world, leg 2: the town's logic beside its cast, the façade down to 80 methods through the one world, the markers one
date: 2026-11-03
area: world, build
started: 2026-11-03T19:28:12-07:00
files: crates/piney-world/src/lib.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/world.rs, crates/piney-world/src/event/mod.rs, crates/piney-world/src/combat/town.rs, crates/piney-game/src/world.rs
took: 14m
---

# 579. One world, leg 2: the town's logic beside its cast, the façade down to 80 methods through the one world, the markers one

The second leg of the relay finishing "One world, one area system"
(`plans/rust-cleanup.md`, section 2), on the branch `one-world` after
[[578]]. It shrank the town's façade (`piney_world::World` in `lib.rs`)
from 132 methods to 80 and found what blocks the rest: where Kite is.
Main gets nothing until the last leg's merge.

## The steps (each commit builds)

1. The town's logic, `impl TownCast` (its frame, `ccThGameCtrl`, the
   entries, the event camera's `TownScene`), moved out of `lib.rs` into
   `town/cast.rs` beside the cast. `lib.rs` is now the façade and the
   free items (`tasks`, `town_start`, `event_kind`, `load_char_tbl`,
   `CharInfo`, `Request`, `Phase`, `Rand`).
2. The façade derefs to the one world (`Deref<Target = world::World>`),
   so the 30 methods it had with the one world's own bodies are gone. A
   step only: the deref goes with the façade. Path calls such as
   `World::state(self)` in piney-game's `impl HostWorld for World` then
   resolve to the trait's method and recurse (rustc warns); they are
   written `piney_world::world::World::state(self)`.
3. `Combat::member` (combat/town.rs) was a copy of `Combat::who`: one
   `who`. With it the façade's `set_affect_colour` and `spc_loaded` were
   the one world's, and the town's `command_target` (by kind and code)
   is the one world's `command_target_code` (21 callers renamed;
   `command_target` stays the scene index). What read only the core
   moved onto the one world: the rand count, `asleep`, the sorted
   command list, the area's fog and clear colour, Kite (`kite`), the
   event camera, `set_spcs`, `present_other`, `fellow_affect`, the asked
   transfers.
4. The markers are the one world's: `marker_dummy`, `marker_bits`,
   `event_marker`, `marker` (`ccEvMarker2Pos`, `INF SLUS_202.67:0x001b1aa4`)
   read any area's marker file with the rotations `AreaCore::written`
   holds. Only a town's `setMerchant` writes that list, so a field reads
   exactly what the field's own `marker` read before.

## What blocks the remaining 80

Most of the methods the two worlds both have differ first in where Kite
is. The town reads `world.player` for Kite (codes 0 and -3); the field
reads the battle's character (`combat.who(0)`, `combat.kite`). The
town's combat also holds a Kite stand-in (`combat.kite` is set in
combat/town.rs, member 0) that the item rules use. `char_pos`,
`set_trans`, `pc_command`, `target_alive`, `command_target_ref`,
`player_distance` and the event camera's scene all turn on this. The
next leg settles one rule first: either a `Cast` answer for Kite's
place, heading and near fade, or, if the town's stand-in follows
`player`, `combat.who(0)` for both.

Other differences found and not yet decided from the game's code:

- `char_pos` takes event types 0-2 as a party character in the town
  (`event_kind`) but only type 2 in the field (`1 << ty & 4`, as
  `set_trans` tests in both).
- The event camera's field scene has no markers ("not ported"); the
  town's reads them. Once `char_pos` is one, `impl evcam::Scene for
  World` with the markers serves both and `TownScene` goes.
- `player_distance` (`ccEvent::CheckOpen` near_marker, `INF
  SLUS_202.67:0x001a7d9c`): the town's `chara::char::w2p` against the
  field's `kite::w2p_pos` through the map's bounds and Kite's `pos_p`.
- `close_menu`: the one world's also clears the PAUSE flag on
  `combat.kite`, which in a town is the stand-in.
- `use_item` and `item_step` take the target as the two modes' handles
  name it (kind and code against a scene index), so they merge with the
  modes (W5).
- The party instructions (`delete_fellows`, `party_remove`,
  `transfer_out`, `manual_mode_ai`, `remote_cmd`, `menu_ban_party`,
  `set_boot_param`) read no town cast state. They work on `world.combat`
  through `TownParty::with_chars`, and the field's versions work on it
  through the battle's API: two ports of the same `ccParty` and `ccAI`
  functions, to be compared and made one.
- The town's outputs (W3) are taken in two orders: effect starts
  (merchants before the race) and sounds (the merchant's `Se(217)` after
  the town's). One stream read in two passes keeps both orders only if a
  merchant's start and sound are separate items; otherwise the game's
  task order decides, and that may move audio pins.

The relay's handoff (/mnt/data/claude/scratch/world3/HANDOFF.md) lists
the façade's remaining methods and these findings.

## Checked

Every commit passed the branch's build checks (`cargo check --workspace
--all-targets`; `cargo test --no-run -p piney-world -p piney-game`).
The release runs at the leg's end: `cargo test -p piney-world` passes
(116 + 3 + 1 + 12), the piney-game suite 347 passed, 0 failed, 152
ignored, as before the leg (run uncapped, as [[578]] explains).

**Still unknown:**
- where Kite's place, heading and near fade come from in the one world (the town's `player` against the battle's character), and whether the town's Kite stand-in in `combat` follows `player`
- whether the game's character position for events takes event types 0-2 or only type 2 as a party character
- whether the town's `ccTransPosW2P` for near_marker goes through map bounds as the field's does
- the 80 methods left on the town's façade (`piney_world::World`), with `World::with_town`, `town`, `town_mut`
- the town's per-character outputs as one stream, one effect host, one mode and one `HostWorld` (W3 to W5)
- `TownCast.rand` and `combat.rand` still two copies of one stream with two `Rand` types (W6)
