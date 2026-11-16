---
number: 605
title: ccSelectTarget on Outbreak and Quarantine: the swapped stack arrays, and test_select's probe pinned to Infection's root
date: 2026-11-16
area: battle, volumes, test
files: crates/piney-world/src/event/talk.rs, crates/piney-world/examples/gamectrl_probe.rs, tools/test_gamectrl_rs.py
started: 2026-11-16T00:04:47-07:00
took: 48m
---

# 605. ccSelectTarget on Outbreak and Quarantine: the swapped stack arrays, and test_select's probe pinned to Infection's root

`tools/test_gamectrl_rs.py`'s `test_select` failed on Outbreak and
Quarantine, 2,689 of 3,000 cases, every one a `ccSortCmnd` distance one
ULP above the game's. Two causes, one in the harness and one in the port.

## The harness: the probe measured with Infection's root

`ccSortCmnd` takes `sqrtf(sceVu0InnerProduct(d, d))` on Infection and
Mutation (INF gcmn 0x00518b98, MUT 0x00536c78, `jal sqrtf`, newlib's
rounding root) and an inline `sqrt.s` on Outbreak and Quarantine (OUT gcmn
0x00531f1c, QUA 0x0042482c), which truncates. The port has had this since
d4fa65b: `talk::sort` measures with `ee::sqrtf_on(leader.volume, ..)`, and
`World` builds the leader with its own volume. But `gamectrl_probe`
built every `Leader` with `Volume::Inf` (in `inarea`, `select` and
`ctrl_frame`), so the harness compared the game's truncated root with
Infection's rounded one. The port's answers were one ULP high exactly
where rounding goes up. The probe now takes the disc's `volumeNum` as its
argument, as `shake_probe` and `cinema_probe` do, and the test passes
`volume.number()`. `ccCheckInAreaCmnd` has `sqrt.s` from Outbreak on too
(OUT gcmn 0x005334e4, QUA 0x00425df4); its 3,000 cases had passed with the
wrong root only because no distance fell on a rounding boundary.

Not a regression, so there is nothing to bisect. The test has followed
`PINEY_VOLUME` since the first commit (3fa1e9e). There `talk::sort` called
`ee::sqrtf` on every volume, so the port itself was wrong on Outbreak and
Quarantine. From d4fa65b the port was right and the probe hid it. No
commit ever had both right.

## The port: Outbreak swaps ccSelectTarget's two arrays

With the probe on the right volume, one case of 3,000 still differed on
both discs: an 18-candidate crowd in a fight, mode 1, `pri` 4, where the
game chose candidate 9 and the port 5. `ccSelectTarget` keeps the range-1
candidates in `near[8]` and the range-2 ones in `wide[8]`, two unbounded
arrays one after the other in its 144-byte frame:

| volume | `near` | `wide` | stores |
| --- | --- | --- | --- |
| INF, MUT | sp+0x50 | sp+0x70 | INF gcmn 0x00518edc / 0x00518f04, MUT 0x00536fbc / 0x00536fe4 |
| OUT, QUA | sp+0x70 | sp+0x50 | OUT gcmn 0x0053224c / 0x00532274, QUA 0x00424b5c / 0x00424b84 |

The passes read back from the same places on each volume, `near` first
and then `wide` (OUT gcmn 0x005322e0 `addiu 112`, 0x00532358 `addiu 80`).
Apart from that the two functions are the same: the filter, the clear of
both arrays, mode 2's step of `cmndTargetPriNum` and the three passes'
masks (0xe0, 0x070000ef clear, 0x0700000f). Outbreak's 48 extra bytes are
nops after branches. So on Infection a ninth candidate in range 1
overwrites `wide[0]`, but on Outbreak it runs past both arrays and is kept.
A ninth in range 2 overwrites `near[0]` instead. In the failing case nine
were in range 1 and one (candidate 5) in range 2: Infection's port lost
candidate 9 under candidate 5, which Outbreak's game does not.

`talk::select_target` now places the arrays by the leader's volume
(`Arrays::on`: slots from sp+0x50, `near` 0 and `wide` 8 on INF/MUT, the
reverse on OUT/QUA) and reads them back the same way. The unit test
`the_select_s_arrays_lie_by_the_disc` holds it: nine enemies ahead and one
to the side, `pri` 8, give the side one on Infection and the ninth ahead
on Outbreak.

## Past the frame

On each volume the array that comes second writes from its ninth entry
into the caller's frame. That is `ccThGameCtrl`'s on every disc (the only
caller: INF gcmn 0x00517f88, MUT 0x00536018, OUT 0x005312b8, QUA
0x00423bc8). That frame is 144 bytes holding only the saved `$s0`-`$s7`
and `$ra` (INF 0x00517800, OUT 0x00530b00, QUA 0x00423410). The task
never returns: it has no `lq` and no `jr $ra` on any volume, so nothing
reads those words again. The port models what `ccSelectTarget` reads back
of its own writes and nothing more. That is exact for up to 36 words past
the arrays, which is 44 candidates in one ring. Beyond that the writes
reach the 2048-byte task stack's top (`ccStartThread(ccThGameCtrl, 33,
2048)`, OUT main 0x001688cc), above the task's entry frame, which no code
on the disc addresses.

## Checks

At 06a4d87 (the fix before this entry):

- `test_gamectrl_rs` on all four volumes: every check 0 differ. Select
  3,000 (941 with a target), in-area 3,000, in-battle 2,000, frames
  104,132, conditions 102,897, game over 61,523.
- `test_world_rs` (talking through `world_probe`'s `target`) on all four:
  24 tests OK. Infection skipped 2, the same as the baseline.
- Battle pins: 291 calls, 0 differ.
- Golden shots against `world2/shots/base`: 4,550 files, 0 differ.
- Crate tests: 39 ok.
- piney-game suite: 348 passed, 0 failed, 153 ignored.
- gen --check, clippy, fmt, cairns check and docs check all clean.

The reference is docs/engine/field-walk.md ("The stack arrays", the
harness note) and field-game.md (`ccSortCmnd`'s root).

`tools/test_sound3d_rs.py` follows `PINEY_VOLUME` too, but its probe
(`se3d_probe`) always loads Infection's sound tables. On Outbreak it fails
7 tests with 2 errors. Not looked into here: this change touches no audio
code.

**Still unknown:** why `test_sound3d_rs` fails on Outbreak, whether only
because `se3d_probe` pins Infection's tables (`sound::tables_of(Volume::Inf)`)
or also from the harness's addresses.
