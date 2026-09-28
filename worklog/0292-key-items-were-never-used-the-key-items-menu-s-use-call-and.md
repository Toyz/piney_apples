---
number: 292
title: Key items were never used: the Key Items menu's use call, and the epitaphs' pages
date: 2026-09-28
area: ui, battle, volumes
files: crates/piney-fieldui/src/menus/keyitem.rs, crates/piney-fieldui/src/menus/useitem.rs, crates/piney-gen/src/manifest.rs, crates/piney-battle/src/item.rs
---

# 292. Key items were never used: the Key Items menu's use call, and the epitaphs' pages

The triage of the open questions (UNKNOWNS.md, [[288]]) listed "an epitaph
or a book used does nothing" as a play gap. The port logged "item step not
carried out" for `Step::Epitaph`. The cause was wider than that step.

## The menu's call

`ImportantItemMenu` (menu 6) calls `ccUseItemRequest` at its proccess 2,
then waits seven frames at proccess 12 before its list answers again. The
port's menu pushed `Request::UseItem` without holding the menu task in the
call (`useitem::call`, which starts an `ItemRun`). The runtime answers a
use only while the task waits in one (`FieldUi::item_asked`), so the
request fell through `AreaMode::menu_request` unanswered. No key item did
anything when used from the menu, not only the epitaphs.

The menu now calls it as the Items menu does. A new `Resume::KeyItem` puts
it back at proccess 12. The Grunty Flute and the Ryu Books keep their own
paths.

## `ccEpitaphMsg`

`ccEpitaphMsg(strs, pages)` (INF gcmn 0x0057c3e0) runs inside the use:
- 8 frames of `Disp` and a breath;
- for each page: `ccKanjiStrSeparate` 0, 1 and 2 of the page's text,
  each a null line when empty, into `ChangeInfo(l0, l1, l2, 0, -1, -1)`;
  `ccMessage` +0x1e and +0x34 (`window_status`, `window_alpha`) zeroed, so
  the lines show without the window; 5 frames; then frames until OK is
  pushed;
- `Close`, and 8 frames more.

The use-item runner does this as `Wait::Epitaph`, with the pages in
`ItemRun::epitaph`. `Step::Epitaph` now names the item and the parody
mode, not Infection's table address.

## The pages in the build

The tables are `char *[pages]`, each page three NUL-ended lines:
`epitaphStr00` (item 42, 2 pages), `01` (43, 1), `02` (44, 3), `03` (45,
2), `04` (46, 3), `10` (48, 3), `11` (68, 4), each followed by its parody
table, and `M0`-`M3` (287-290, one table for both modes). They are now in
the `fieldui` group as `epitaph_00` .. `epitaph_m3`, placed on each volume
by the carry (only `00`, `01`, `03` and `M3` carry names past Infection).
Outbreak's and Quarantine's parody tables are empty. Data version 15.

`an_epitaph_reads_its_pages` gives item 42 and reads it from the Key
Items: both pages show in order, then the list answers again. It fails
without the menu's call. `tools/test_battle_items_rs.py` still matches
the game; its probe gives the `ccEpitaphMsg` call as Infection's address
for the item.

**Still unknown:**
- The Ryu Books (273-280): `ccStartThread(ccThBook, 35, 0x1000)` and the
  steps after it (`BookStart`, `CloseChat`, `WaitBook`) are not ported, so
  reading one still does nothing.
- Whether the key items other than the epitaphs, the flute and the books
  have uses whose steps the runtime does not carry out: none was tried.
