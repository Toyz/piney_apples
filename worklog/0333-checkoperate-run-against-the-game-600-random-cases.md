---
number: 333
title: "CheckOperate run against the game: 600 random cases"
date: 2026-10-01
area: script, test
files: tools/test_event_vm.py, crates/piney-event/tests/vm.rs, crates/piney-event/tests/vm_fixture.txt, docs/engine/field-game.md
---

# 333. CheckOperate run against the game: 600 random cases

[[90]] noted that `ccEvent::CheckOperate(9)`, the talk rule, had only been
checked in pieces. The target test was the interpreter's, but the function
as a whole had not been run against the game.

`CheckOperate(num, flag)` (INF SLUS_202.67:0x001b32f0, 224 bytes) is a
leaf:
- `num < 0` answers 1;
- with `flag` 0 and `operateSet` (+0x778, s16) below 0, it stores `num`
  there;
- the answer is 0 when `operate` (+0x770, 64 bits) is not 0 and has the
  bit `1 << num`. The shift is a 32-bit `sllv` and sign-extends, so `num`
  31 tests bits 31-63 and `num` 32 tests bit 0;
- for `num` 9 with `cmndTarget` set, it stores `operateTarget` (+0x780)
  and answers 0 when one of `target[16]` (+0x40, s16 type and code) has
  `1 << type` (again 32-bit) among `cmndTarget->base` +8's type flags and
  its code equal to `base` +0xc. Empty targets (-1, -1) test bit 31 and
  code -1.

`tools/test_event_vm.py` now writes 600 `operate` lines (seed 9). Each
starts from `ccEvent::Init` with random registers:
- `operate` (0 in 30% of cases);
- `operateSet` (-1 in half);
- sixteen targets (60% empty);
- `cmndTarget` either NULL or a character whose type flags are random,
  given one live target's bit and code in 40% of cases;
- `num` from -1, 9, 9 and 0-63, with `flag` 0 or 1.

Each line records the game's answer, `operateSet` after, and whether
`operateTarget` was set. They were appended to `vm_fixture.txt`; the rest
of the fixture is unchanged by the harness edit.

`check_operate_matches_the_game` (piney-event, tests/vm.rs) replays them
with `Vm::check_operate` on an `OperateHost` and needs no disc. All 600
match. Moving the type bit by one in the port fails it.

**Still unknown:** nothing about `CheckOperate`. The rest of [[90]]'s list
(members in town, the gate's invite warp-in) is unchanged.
