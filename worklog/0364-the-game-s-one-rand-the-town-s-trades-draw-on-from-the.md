---
number: 364
title: The game's one rand(): the town's trades draw on from the field, not from a stream of their own
date: 2026-10-03
area: engine, world
files: crates/piney-desktop/src/save.rs, crates/piney-desktop/src/lib.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/session/tests/town_return.rs, docs/engine/field-ui.md
---

# 364. The game's one rand(): the town's trades draw on from the field, not from a stream of their own

A player reported that several town PCs at once offered Hell's Gate,
Dante's Blades and Masterblades. Another found that the offers change
each time a town is entered, from the lists in `feTbl`, bounded by the
servers open. The question was whether the port's random numbers were
right.

## The pick is the game's

`SetTradeItemTown` (INF SLUS_202.67:0x001767a0) was read again against
`piney_battle::restock`; the rule is now in
[field-ui.md](../docs/engine/field-ui.md). The port had it right:
- **The toss and the category.** An odd `rand()`, then `rand() % 10`.
- **The tables.** The jump table at 0x0034d080 maps categories 0-9 to
  `feTbl1`-`feTbl6`, `feTblH`, `feTblA`, `feTblG`, `feTblB`. That is the
  order of the `feTbl` pointers the port reads (gcmn 0x0069a260).
- **Kite's row.** The weapons key on Kite's weapon (`equipment[4]`,
  read at `this` +0x7558) in `feTbl1`; armour on Kite's piece in its own
  table.
- **The limit.** Index 10 + server of `f_limitTbl` (15 + server for
  category 2).
- **The move.** `rand() % 4 - 1`, clamped to 0 and the limit inclusive.

So an offer is never luck of the table: it is Kite's own row moved by
-1 to +2. From a new game (Amateur Blades, row 0), a simulation of the
first four restocks put picks on 28 to 31 of the 48 rows, every blade
among rows 0-2 (Amateur, Steel, Phantom). Hell's Gate is row 18,
between Masterblades (17) and Dante's Blades (19). A Kite holding blades
of that class gets those three from every PC whose toss came up
category 0, and the other weapon kinds of the same rank from the rest.
That fits the report.

## The random numbers were not the game's

The game has one `rand()`, newlib's (main 0x00133a38, state at
`*_impure_ptr + 168`). It starts at 1 at power-on, and everything that
draws a random number moves it on. `srand` has one caller, the staff
roll (`ccThStaffRollCtrl`, desktop.prg 0x0041270c). The soft reset runs
inside `ccThMother`, with no new boot, so it does not restart the
generator. `ccInitRand` (main 0x001d9900) seeds the other generator, the
Mersenne Twister `ccRand`, with 0x1100; it does not touch this one.

The port kept separate streams, each from 1:
- the session's, used only by the restocks;
- every town `World`'s;
- every field's and dungeon's `Combat`;
- every stream played (event, Data Drain, gate hack, book, title,
  Audio screen).

So the restocks drew the same numbers on every boot, however the game
had been played: the n-th restock of a session always made the same
picks. The simulation above, four restocks of a new game's save, made
28, 30, 28 and 31, the same on every run.

## Fix

- **One state.** `SaveState` gets `rand`: the newlib state, 1 for a new
  state. `SaveState` is what every mode is handed and hands back.
- **The modes.** The town (`World::enter`) and the field or dungeon
  (`FieldWorld::enter`, its `Combat`) start from it. `World::state_out`
  and `FieldWorld::state_out` return the state with the stream as they
  left it, and `WorldMode::leave` and `AreaMode::leave` use them.
- **The restock.** It draws from `state.rand` and writes it back. The
  session's own stream is gone.
- **Streams.** Every `StreamPlayer` (event, drain, gate hack, start)
  takes the mode's state and starts its `Rand` there. When it ends, the
  mode takes the stream's `rand` back: `FieldWorld::set_rand`,
  `World::set_rand`, the desktop's or the title's state.
- **The staff roll.** Its `srand` reseeds the one stream; when it ends,
  the desktop's state takes its stream.
- **The soft reset.** RESET and TITLE carry the old mode's `rand` into
  the new title's state.

Within a mode, the port's draws are not the game's draw for draw:
not every caller of `rand()` is ported, and the port's may come in
another order. What now holds is the game's shape: one stream, moved
on by play, so a town's offers depend on what came before.

## Tests

- **The new test.** `the_town_restocks_from_the_rand_the_field_left`
  (piney-game) leaves field 14 for Mac Anu twice with the field's
  `rand()` set to 7, and once with it set to another value. Each NPC's
  slot-15 pick is equal for the two 7s and differs for the other. With
  the restock on a fixed seed, as before, the second assertion fails.
- **Pinned scenarios.** `drain_with`'s fixture (`a_scroll_on_a_foe_is_
  cast_by_kite` and the drain tests) and `field_with_mia_and_elk`
  (`the_fallen_keep_no_condition_marker`) now set the area's `rand()` to
  1 when they hand over. Their fights had depended on the area starting
  from 1.
- **Suites.** piney-game (223), piney-world, piney-desktop and
  piney-stream pass, as does `tools/test_world_rs.py`.

**Still unknown:** nothing.
