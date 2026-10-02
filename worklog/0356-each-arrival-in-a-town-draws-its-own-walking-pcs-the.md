---
number: 356
title: "Each arrival in a town draws its own walking PCs: the session counts frames from power-on"
date: 2026-10-02
area: world, engine
files: crates/piney-game/src/session.rs, crates/piney-game/src/world.rs, docs/engine/field-game.md
---

# 356. Each arrival in a town draws its own walking PCs: the session counts frames from power-on

Issue #32: the PCs walking a Root Town were the same on every visit. In
the game they change each time the party comes back.

A town's set-up runs `ccInitRand` (INF SLUS_202.67:0x001d9900). It seeds
the game's MT19937 with 4352 and draws it `ccSys+0x358` times.
`ccRegisterRandomNpc` then picks the walking PCs' `npcTbl` rows from that
generator ([[65]], `docs/engine/field-game.md`). The port had the count
as an input, `World::set_rand_count`, but nothing ever set it. So every
town drew from a generator drawn 0 times, and picked the same rows.

The count is a frame counter. In `ccSystem::Ctrl` (0x0010a6bc), after the
wait on `sceGsSyncV` for the frame rate, the game adds one to `ccSys+0x358`
once per game frame. `ccSystem`'s set-up (0x0010a944) zeroes it at power-on.
`Session` now keeps the same count (`sys_frames`, one a `step` from power-on).
It hands the count to every town it sets up: on Log in, and on each
`enter_world` into a town. The port's frames cannot equal the game's (its
load times differ), but two arrivals now differ as they do in the game.

`each_arrival_in_town_has_its_own_pcs` goes from event 3's field to Mac
Anu, then Dun Loireag, then Mac Anu again, by the console's `town`. It
compares the rows of the walking PCs on the two arrivals. With the count
left at 0 the rows are the same and the test fails.

**Still unknown:** nothing.
