---
number: 392
title: A member's thanks for a gift are the volume's: Mutation's Gift weighs the gift against what the member has, and its thanks come from the volume's own tables
date: 2026-10-08
area: ui, volumes, save, test
files: crates/piney-fieldui/src/menus/talk.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/items.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/talk.rs, crates/piney-data/src/tables/fieldui.rs, crates/piney-data/src/pack.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/gift_thanks.rs, tools/test_fieldui_talk_rs.py, tools/test_fieldui_shop_rs.py, tools/test_fieldui_trade_rs.py, docs/engine/field-ui.md, UNKNOWNS.md, BUGS.md
---

# 392. A member's thanks for a gift are the volume's: Mutation's Gift weighs the gift against what the member has, and its thanks come from the volume's own tables

Issue #58 (Mutation, build 4cce396): "trading" BlackRose in Chosen
Hopeless Nothingness. Her window showed `___b_Z_[_W_f_[_^____` in red,
but her voice was right. The report asked whether her lines were Kite's.

## What it was

The window is `errorData`, the game's record for a missing line:
`#R` and "メッセージデータがありません" (no message data) in Shift-JIS,
then `#W`. The English font draws each kana's two bytes as `_` and a
letter, so "ッセージデータ" reads `_b_Z_[_W_f_[_^`.

The "trade" was Gift (`PresentMenu`, menu 50). Trade (`TradeSubMenu`)
opens no window in the member's name: Mutation's calls only `OpenInfo`.
A gift's thanks open with voice group -32, line `5 id + n`, which the
port worked out from the id. That is why the voice was right.

## Cause

The port's `PresentMenu` read the thanks from `spcMsgPresent10` and
`spcMsgPresent11` at Infection's gcmn addresses (0x00638800,
0x00638890), written in as constants. Mutation has no table there, so
every member's thanks were `errorData` on the later volumes. The tables'
own addresses (`fieldui::spc_msg_present10_va`) were already generated.

## What the game does from Mutation on

Mutation's `PresentMenu` (MUT gcmn 0x00572a90) is not Infection's. It
weighs the gift before giving it, and gives it after the thanks:

- proccess 4 only keeps the count (`temp[0]`) and the item.
- proccess 6 (0x00573660) works out the worth. It is the price at a gift
  rate of its own (MUT main 0x0035f620; Trade keeps `spcTradeRateTbl`,
  0x0035c480, as on Infection).
  - A gift the member wants (16 a member, MUT main 0x0035f850) is worth
    50000, unless it is equipment the member carries or wears. Mia
    wants 1/66, 1/69 and Aromatic Grass, so Infection's special case for
    the grass is now a row of this table.
  - Other equipment is worth its value less the member's dearest of its
    kind, carried or worn (a weapon: the job's weapon). Its bands are
    -2000, 0, 500 and 1000 (gcmn 0x00682100).
  - Anything else, and the wanted gifts, use 100, 1500, 3000 and 6000.
  - `talkNum` comes through the getter (MUT main 0x0017abf0).
- proccess 8 (0x00573cfc) gives it after ten frames: `spcPresent`
  through the adder (0x0017ae30), `AddSpcItem` and its breath,
  `DelItem`, then the same line as Infection's.

Outbreak and Quarantine run the same code with their own bands (Outbreak
-3000, 0, 600, 1300 and 100, 2000, 4000, 8000; Quarantine -3000, 0, 1000,
3000 and 100, 5000, 10000, 20000).

## Fix

- **Tables.** `piney-gen` finds the three new tables
  (`fieldui::gift_wants`, `gift_bands`, `gift_rates`) in each later
  volume's weighing code (`gift_tables`). It looks for:
  - the `lui/addiu $fp` of the bands, with `addiu $fp, $fp, 16` after it;
  - the `sll 6` row added to the wants just after that load;
  - the rates added to `ccGetItemTradeRate`'s `$a0` just before it.

  A first try found the code from the place that loads
  `spcMsgPresent10`. That failed on Outbreak, because the carry puts
  Infection's 0x00638800 at Outbreak's `presentMenuHelp`. `DATA_VERSION`
  is 25.
- **The menu.** `GiftOrder` picks Infection's order or the later one.
  `weighed_worth` is proccess 6. `PresentTail::Gave` is proccess 8's
  breath. The thanks read the volume's `spc_msg_present10_va` and
  `spc_msg_present11_va`.
- **Members 18-20** (Tsukasa, Subaru, Sora) keep their records in the
  save's extension. These now go through `by_id`, as the later volumes'
  getters do: `SetSpcBaseMsg` (1-20 from Mutation on, MUT 0x001777d0),
  the member's base, `talkNum` (SpcMenu, TalkMenu, Gift), `spcPresent`,
  the skill list, and the bag (`items::save_item`, `del_item`).
- **The talk tables.** The generator had given every table by
  character Infection's 18 rows. From Mutation on there are 21, so
  members 18-20's thanks were `errorData` too.

## Checked

- **Reproduced and fixed.**
  `blackroses_thanks_for_a_gift_are_hers_on_every_volume` (piney-game)
  drives `FieldUi` as the runtime does: BlackRose spoken to, Gift, a
  Health Drink, OK. It fails before the fix on Mutation (`errorData`,
  rec 0x36f968) and passes on all four volumes.
- **Shots** (`blackroses_thanks_shot`, ignored):
  - `/mnt/data/claude/scratch/i58/before/mut-blackrose-thanks.png`
    shows the report's red `___b_Z_[_W_f_[_^___`;
  - `after/mut-blackrose-thanks.png` shows "Yeah, I'll take it. /
    Thanks."
- **Against the game's own `PresentMenu`** (tools/test_fieldui_talk_rs.py,
  `GiftPages`, eemu, frame by frame):
  - The harness now runs on every volume. It reads charTbl's 21 names,
    and the probe runs `set_spc_base_msg` with the disc's volume (it had
    used Infection's, so every later greeting was `errorData` there too).
  - Before the fix seven of the eight gift cases failed on Mutation,
    Outbreak and Quarantine. After it they pass, with new cases:
    - `test_gift_every_member`: members 1-17;
    - `test_gift_wanted`: Mia's wanted weapon, carried, worn, and the
      grass;
    - `test_gift_later_members`: 18-20, through the extension.
  - Mutation's whole talk harness matches (36 cases). So does
    Infection's (two later-volume cases skipped).
- **Member trades** (tools/test_fieldui_trade_rs.py) match on all four
  volumes. The shop and trade harnesses now read `ccMenuCtrl` through
  `volume.menu_at`. Before that, nine of Mutation's ten trade cases
  "failed" on Infection's offset of `drainItem`.
- piney-game's suite (four threads), the tests of piney-fieldui,
  piney-gen and piney-data, `piney-gen gen --check`, clippy on the
  touched crates, fmt, `cairns check` and `tools/docs.py check` pass.
  tools/test_fieldui_rs.py matches on Infection and Mutation.

**Still unknown:** two things the harnesses showed, not about gifts.
- On Outbreak and Quarantine the talk harness's `test_admin` and
  `test_breeder_about` differ in the message window's mode (1 against 4)
  on a chained record. The shop harness's Outbreak `test_other_talk` and
  Quarantine Buy pages, merchant lists and `test_other_talk` differ.
  None of these were read.
- Mutation's Recorder: `RecordMenu` drops the 10-frame hold after a
  save's result unless `game+0x7c` or `ccSys+0x26c` is set (MUT gcmn
  0x00577a8c, 0x00577af0, 0x00577b54). The port keeps Infection's hold,
  and three Recorder cases differ by it. What `ccSys+0x26c` is has not
  been found.
