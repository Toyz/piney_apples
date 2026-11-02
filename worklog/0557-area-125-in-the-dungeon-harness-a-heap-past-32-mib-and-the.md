---
number: 557
title: Area 125 in the dungeon harness: a heap past 32 MiB, and the doors past the room array's last row
date: 2026-11-02
area: world, test, tooling, volumes
files: tools/test_dungeon_rt.py, tools/eemu.py, crates/piney-eemu/src/cpu.rs, crates/piney-eemu/src/python.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-world/src/field/world.rs
resolves: 550
started: 2026-11-02T01:30:35-07:00
took: 1h 26m
---

# 557. Area 125 in the dungeon harness: a heap past 32 MiB, and the doors past the room array's last row

[[550]] left area 125's fifteen floors out of `tools/test_dungeon_rt.py`. The case now runs on Mutation, Outbreak and Quarantine, and the harness passes on all four volumes. The heap had to grow, and the port was wrong about one Draw on B15.

## The heap

The harness's `ccMalloc` hook is a bump allocator over 0x01100000-0x01F00000, and its frees do nothing. Handing freed blocks out again was tried and rejected: it changes what the game's code reads of a deleted block, and Quarantine's field 14 hit models then differ. On Quarantine, area 125's `make` alone (constructor, `Generate` over fifteen floors) ran from its base at 0x016C5EC0 to 0x02438B10. That is about 12.4 MiB, against the 8.2 MiB left below the stack.

Both machines can now grow past the console's 32 MiB. `tools/eemu.py`'s `Machine.grow(size)` extends its bytearray. piney-eemu checks every access against its RAM slice's length instead of the `RAM` constant (`fits` in `cpu.rs`), and its Python binding has `grow`, which makes `mem` a new bytearray holding the old bytes. Its `mem` setter now takes any bytearray of 32 MiB or more. `test_eemu_rs.py`'s `test_api` checks the two machines alike past 32 MiB: an `sw` there, a harness store, and the fault at the new end.

The harness grows its machine to 64 MiB. Its heap goes on at 0x02000000 once the window below the stack is full, and only area 125 gets that far. The first run then failed on later cases (field 14's `OBJ_w_9e20_` word 14, off by 299; a lake's word 13), because the cases share one `Game`. Every case resets the heap to its base, so a pointer a global keeps lands on the same block when the next case is made again. Area 125's late blocks sat above 32 MiB, where no later case allocates, so the globals kept pointing at them. Without the area-125 case the same three tests passed with `grow` in place. A case after one whose heap went high now gets a fresh machine (`went_high`).

## The doors past the last row

With the heap in place, one comparison failed. On area 125's floor 14 (B15), after `GotoNextRoom` from room 0 at cell (32, 20), a Draw at (59362.9, 1376.7), a cell of no room, counted 2 doors drawn in the game and 0 in the port.

`here` for a cell of no room is 15. `MoveDoor` (QUA gcmn 0x004e8258) and `Draw` (0x004e9a94) both test `room[level][here]` for non-null, so with `here` 15 they read `room[level + 1][0]`. That is another floor's slot, never built while the party is on this one, except past the array's last row:

| volume | array | the word after the last row | null? |
| --- | --- | --- | --- |
| INF | `room[10][15]` +0x2f230 | `rotate[0][0]` +0x2f488 | when floor 0's room 0 is not turned |
| MUT, OUT, QUA | `room[15][15]` +0x438 | animIdx pointer +0x7bc | never |

Only B15 of area 125 reaches the later volumes' last row. There `MoveDoor` steps and draws the built room's doors, and `Draw` hands the animIdx block to `ccAnm::Draw` as the room. Its +0xfc word is floor 2 room 1's `ROOM_INFO` pointer, and its +0x94 `ccAnime` is floor 1 room 3's `ROOM_INFO` row. The port does not reproduce that garbage draw.

`DungeonArea::room_slot_taken` reads the slot as the game does. `shown` and `draw` draw the doors whenever the slot is taken, and the room only when it is the one built. The frame's `move_door` now runs for a cell of no room too, taking `Option<usize>`, and stores `doorAnm` before the test as the game does. `SetDoor(level, 15)` is left out: `CloseStart` runs only after `CloseDoor` has shut the party in its room. Test: `dungeon::tests::past_the_last_row_a_roomless_cell_still_draws_the_doors` (B15 draws the doors, B10 none).

The harness on every volume: INF, MUT, OUT and QUA all OK, with 11,856 `DUNGEON::Draw` comparisons on the later volumes and 9,536 on Infection.

**Still unknown:** nothing.
