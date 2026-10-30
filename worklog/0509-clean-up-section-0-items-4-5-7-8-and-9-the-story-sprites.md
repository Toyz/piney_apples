---
number: 509
title: Clean-up section 0, items 4, 5, 7, 8 and 9: the story sprites keyed by file, store groups read once, eemu's bus owned by its system, the field ambient statics carried in the save, the executable guard per disc, the events shared
date: 2026-10-29
area: engine, world, script, tooling, test
files: crates/piney-game/src/fx.rs, crates/piney-data/src/store.rs, crates/piney-eemu/src/cpu.rs, crates/piney-eemu/src/sys.rs, crates/piney-desktop/src/save.rs, crates/piney-world/src/field_ambient.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/field_world.rs, crates/piney-data/src/iso.rs, crates/piney-game/src/main.rs, crates/piney-event/src/official/mod.rs, plans/rust-cleanup.md
started: 2026-10-29T20:16:06-06:00
took: 16m
---

# 509. Clean-up section 0, items 4, 5, 7, 8 and 9: the story sprites keyed by file, store groups read once, eemu's bus owned by its system, the field ambient statics carried in the save, the executable guard per disc, the events shared

Section 0 of `plans/rust-cleanup.md`, items 4, 5, 7, 8 and 9: state that
was process-wide, or a cache that could hand out the wrong thing, is now
owned or keyed correctly. No game behaviour changes.

## The story maps' sprite cache (item 4)

`AreaFx::story_eff` cached a map's `ccEff`s by (effect, flare, fog). The
town's cache has the file in its key; this one did not. The key is now
(file, effect, flare, fog). Nothing collided in practice:

- an `AreaFx` lives for one area;
- the arena fireflies' names (`EFF_se1_5fir1` and so on) are unique per
  stage file.

So this is a latent fix only.

## `store::group` (item 5)

The old code took the lock, read the group without it, then inserted
with `or_insert`. Two threads could both read and leak the group, and
the loser got back a pointer to its own copy. Now each (volume, name)
has a leaked `OnceLock` slot, found under the map's lock and filled
under its own. A group is read once and every caller gets the same
`&'static`. Different groups still read in parallel.
`a_group_is_read_once` races eight threads on Mutation's field group
and checks they agree.

## eemu's bus (item 7)

The `BUS` thread-local is gone. `Mem` carries `bus: Option<&mut dyn Bus>`:

- `Sys` owns its `IoBus` and passes it in for every run, `w32` and
  `write`.
- `r32` and `r8` read through it as before (`IoBus::get` behind the
  `RefCell`).
- The harness machines (`machine.rs`, `python.rs`) pass `None`. As before
  (no bus was ever set on their threads), an access outside RAM faults.

`cpu::load` now reads RAM alone, and `Mem::load` goes to the bus outside
it. `each_system_has_its_own_bus` writes two `Sys`' scratchpads on one
thread, then drops one.

## `KEPT` and `DENY_EXECUTABLE` (item 8)

`WORLD`'s ambient drawing statics (rain, thunder, steam, the balloon's
cloth) outlive every field: they are function statics in the game.
`KEPT` held them in a process-wide mutex, set by `FieldArea`'s `Drop`.
That made every field after the first in a test process depend on the
fields before it.

They now ride in `SaveState::field_ambient`, an `Option<[u32; 17]>`.
`field_ambient::kept_words` and `kept_statics` lay them out, and None
means the executable's start values (`tcnt2` 750, `steam_step` 0.5).
`FieldArea::with_world` takes them, and `FieldWorld::state_out` writes
them back. `carry_memory` keeps them across a soft reset, as the game's
memory does. A dungeon in between leaves them as they were.
`kept_statics_round_trip` covers the layout.

The executable guard is now `Iso::without_executable`. The game opens
its discs through `open_disc` (piney-game's 36 opens outside its tests).
Tools, checks and piney-gen's own opens still read the executable, which
the generator needs. `a_disc_without_its_executable` covers it.

piney-game's suite plays the check images, and Quarantine's has no port
data, so its events come from the executable. The first suite run showed
it: seven tests failed on `SLUS_205.64: the port does not read the boot
executable`. The old global never caught this, because only `main` set
it and the tests never call `main`. `open_disc` therefore leaves the
disc as it is under `cfg!(test)`, which matches the old split exactly.

## `PARSED` (item 9)

`official::events` returns `Arc<[Event]>` from a
`OnceLock<Arc<[Event]>>` per volume. A failed read is no longer kept,
so the next call tries again. Callers that only read share the slice.
The two that edit or move events (`desktop::boot` with its extras, and
`Library::new`) copy them.

## Checks

- piney-game's suite, and the tests of piney-data, -eemu, -event,
  -desktop, -world and -fieldui.
- `test_eemu_rs.py`.
- `test_map_rs.py` and `test_field_ambient_rs.py`, which open fields
  through `FieldArea::new`, now from the start values every time.

**Still unknown:** nothing
