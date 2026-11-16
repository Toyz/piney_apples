---
number: 607
title: The field UI on the later discs: the carried names put Outbreak's menu handlers four over, and ten behaviours of Mutation, Outbreak and Quarantine
date: 2026-11-16
area: ui, volumes, tooling, test
files: crates/piney-gen/src/xfer.rs, crates/piney-gen/src/manifest.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/menus/keyitem.rs, crates/piney-fieldui/src/menus/skill.rs, crates/piney-fieldui/src/menus/shop.rs, crates/piney-fieldui/src/menus/status.rs, crates/piney-fieldui/src/menus/system.rs, crates/piney-fieldui/src/menus/option.rs, crates/piney-desktop/src/savesys.rs, tools/test_fieldui_rs.py, tools/test_fieldui_personal_rs.py
started: 2026-11-16T05:01:07-07:00
took: 4h 44m
---

# 607. The field UI on the later discs: the carried names put Outbreak's menu handlers four over, and ten behaviours of Mutation, Outbreak and Quarantine

## Why the field UI failed from Mutation on

The 76-harness sweep on main (2d2b796) had `test_fieldui_rs` failing 8 on
Outbreak and 9 on Quarantine, `test_fieldui_shop_rs` 1 and 6,
`test_fieldui_personal_rs` 4 on each later disc, and (not in the sweep, as
it names no `PINEY_VOLUME` itself) `test_fieldui_option_rs` 2 on each.
Three causes were in the tools (the carried names, the harness's own
spans, `inf_va` around ground codes), and ten behaviours in the port.

## The carried names put Outbreak's menus four over

Outbreak's and Quarantine's executables are stripped; `piney-gen syms`
carries Infection's names to them. Its call pass pairs a function's calls
by aligning the two bodies. `ccThMenu`'s switch has some ninety cases, each
`jal Handler; nop; b end; nop`, all alike once addresses are masked, so on
Outbreak and Quarantine (whose table has four more entries, 94 against
Infection's 90) the alignment paired a long run of them four cases off:
about sixty handlers got the name of the one four menus before.
`GtHackMenu` (OUT gcmn 0x00581bb0, the function just before `ccThGtHackMenu`, which it starts) was filed as `GtNewMenu`,
`RecordMenu` (0x00573090) as `TradeSubMenu`, `FountainMenu3` as
`InuMenu`. The data pass then named globals from the wrong bodies:
`dialogVoice` sat on `cmndTarget` (OUT main 0x00386fa8).

That reached the build. The generator reads the gate's refusal (the
bracelet's story areas, [[228]]) from the code of `GtNewMenu`; on
Outbreak and Quarantine it read `GtHackMenu`, found no `eventStatus` test
and gave none, so those discs' warp menus never refused.

The fix pairs switch cases by entry: where two paired bodies have as many
`jr` tables (built by `lui`/`addiu` before the `jr`, sized by the
`sltiu`), each case's first call before its first branch is paired with
the other's at the same entry, and those calls leave the alignment's
pairs. Mutation's sidecar is unchanged; Outbreak's 280 lines and
Quarantine's 281 change (every one looked at a correction: the handlers
by the switch, the globals they build); the carry changes 157 and 158
rows; of the generated tables only `fieldui`'s gate refusal changes
(Outbreak and Quarantine now have Mutation's: `eventStatus[40]`, 16 areas,
Kite's line).

## The harness

- `GT_HACK`, `DRAIN` and `FountainMenu3`'s span came by name and were
  wrong or empty on Outbreak and Quarantine; with the names right they are
  taken by name on every disc.
- `DISP_NOIZ`, the part of `Disp` whose `SetNoiz` calls count as the
  menu's own, started at Infection's 0x00522220 carried by offset, which
  fell past Outbreak's `Disp`; it was then left empty with a comment that
  no Outbreak scenario made noise (three did). It is now found in the
  code: after the menu window's `ccSprite::Trans` up to `ccNoiz::Draw` (OUT
  0x0053b028, QUA 0x0042d938; Mutation's start moved from 0x00540370 to
  0x005403c8, leaving out a chat `Disp` and three `SendPacket`s).

## The port: what the later discs do

Each is the game's code on that disc; the port had Infection's (or
Mutation's) on every disc.

| what | INF | from | the code |
| --- | --- | --- | --- |
| a party panel's condition icons start at y | 320 | OUT: a static the disc holds as 0, read and never written | INF gcmn 0x00520a9c; OUT 0x00539ad8 reads 0x00720c90, QUA 0x0042c3e8 reads 0x00619690 |
| a barred Data Drain's help | `skillMenuHelp[2]`, Kite's promise | QUA: `[3]`, "Cannot use Data Drain." (`volumeNum >= 4`, the test on every disc; the row exists from OUT) | QUA 0x00439070, OUT 0x00546760 |
| `BuyMenu`'s stock count stops at | a code <= 0 (`blez`) | MUT: < 0 (`bltz`), so Quarantine's weapon shop sells item 0 | INF 0x005565a8, MUT 0x00575728, OUT 0x0057141c, QUA 0x00463d5c |
| the hold after "saved" with no controller | 10 | MUT: 0 unless `game.pauseFlag`; OUT: `LoadInfoReq` sets it, `SlotSelectReq` and `EndReq` clear it, so 10 again | OUT main 0x00171090, 0x00171054, 0x0017102c |
| `ParameterDisp`'s tenth of a value below 0 | cut toward 0 | MUT: 9 lower first (-25 is -3) | INF 0x00524188, MUT 0x005424fc |
| the Grunty Flute | a random adult Grunty | MUT: a list of the callable ones to choose from (proccess 20, 30, 31, 35) | MUT 0x0054e7ac - 0x0054ec9c |
| PERSONAL's row 6 on fields 8 and 13 | open | MUT: greyed, OK buzzes (sound 20) | MUT 0x00546ec4, 0x00546f50 |
| Discard after a throw | the list refitted next frame | MUT: at once (`SetItemList` with the count) | MUT 0x00550054 |
| the Data Drain demo page's information | two lines | MUT: the first | INF 0x0054080c, MUT 0x0055f390, OUT 0x0055aad0 |

The Grunty Flute's list is the port's new piece: its names are
`ccGetNpcParam(145 + n)` for the slots `ccPgAdultCheck` answers, in the
key items' window as disp 7 (`select` and `my` kept in `temp[0]`,
`temp[1]`), its help the `$gp` word Mutation added after
`puccigusoCallWarnStr` (the generator's `grunty_select`, read where
`ImportantItemMenu` reads it; none on Infection). Cancel (31) puts the
list back; OK (35) puts it back and calls the chosen Grunty as
Infection's random call did. `test_fieldui_personal_rs` gained a choice
and a cancel with two Grunties.

## The inf_va audit

Every `inf_va(...)` in `tools/test_*_rs.py` (344 distinct literals, five
expressions) was checked against Infection's symbols: what it names, at
what offset, in which overlay (the demo, desktop and top page harnesses
bind their own). The ones that are not addresses are ground codes, the hit
attributes `seHitAttr` and the dust and footstep code compare with
`hitAttribute & 0x00f0f0f0`, which happen to fall inside big data symbols
(0x00304050 is `enemyCCSTbl` +5664): `test_battle_spawn_rs` (eight lines),
`test_foe_rs` (one) and `test_effect_particle_rs` (two: 0x0060B0D0,
0x0070C0E0, 0x00304050, 0x00606060, 0x00405060, 0x00123456). They are
plain constants now; the three pass on all four volumes, as before on
Infection. `test_stream_rs`'s `DATA = inf_va(0x00400000)` (`mt` +3072)
was never used and is gone. The five expressions carry real addresses
(destructors, vtables, hooked code).

## Checks

At 85680d9 (the code of this entry):

- The field UI harnesses on all four volumes: `test_fieldui_rs` OK (5
  skipped on each: Infection's four Mutation-on tests and the gate
  refusal; from Mutation the five lesson tests, whose events are empty
  there, `evMsgTblM1`), `test_fieldui_personal_rs`, `_shop_rs` (Infection
  skips its Mutation-on talkers), `_talk_rs`, `_trade_rs`, `_option_rs`
  and `test_chat_msg_rs` all OK.
- The 76-harness sweep against main's: 304 runs each; failing runs 31 on
  main, 27 here. Newly passing: `test_fieldui_rs` OUT and QUA,
  `_personal_rs` MUT, OUT and QUA, `_shop_rs` OUT and QUA,
  `test_desktop_savemenu_rs` OUT (the pause flag; Quarantine's goes from
  3 failures to 2). Nothing that passed on main fails: the sweep ran at
  7135751, where `test_effect_draw_rs` (4) and `test_eemu_rs` (2 more on
  Infection) broke on `test_stream_rs`'s `DATA`, which they import; with
  85680d9 they are back to main's results (`test_effect_draw_rs` OK on
  all four, `test_eemu_rs` main's one error).
- Battle pins: 291 calls, 0 differ.
- Golden shots against a base shot today from 2d2b796 (itself equal to
  `world2/shots/base`, 4,550 files, 0 differ): 4,550 files, 22 differ, all
  `macha_shots` (Quarantine) from frame 6670 on. Each difference lies in
  x 173-289, y 0-368: BlackRose's (and later the others') condition icons
  move from over the panel to the screen's top, the change the game's own
  `Disp` draws in `test_fieldui_rs` (`test_low_hp`, `test_boosts`).
- Crate tests 45 ok; the piney-game suite 348 passed, 0 failed, 153
  ignored (the ride tests now OK the flute's list); `gen --check`, `syms
  --check` and `carry --check` on all three later volumes current; clippy,
  fmt, `cairns check`, `docs.py check` clean.

**Still unknown:**
- why `test_desktop_savemenu_rs` still fails twice on Quarantine (the
  overwrite's menu does not close; no scenario reaches result 0x1030)
- `ccThMother`'s pause while the controller is out (OUT main 0x00167148
  reads `pauseFlag` and `pad[0].status`) is not ported, so the port's
  `pauseFlag` lives only in `SaveSys`
