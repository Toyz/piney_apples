---
number: 396
title: A walking PC's second box is its own line: the later volumes' talk records are measured by their own shape, and PcMenu gives Sieg, Kaz and the sign PCs Talk
date: 2026-10-08
area: ui, volumes, build, test
files: crates/piney-gen/src/talk.rs, crates/piney-data/src/pack.rs, crates/piney-fieldui/src/menus/merchant.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/pc_talk.rs, tools/test_fieldui_shop_rs.py, docs/engine/field-ui.md, docs/README.md, UNKNOWNS.md, BUGS.md
---

# 396. A walking PC's second box is its own line: the later volumes' talk records are measured by their own shape, and PcMenu gives Sieg, Kaz and the sign PCs Talk

Issue #63 (Mutation, build 4cce396): Oborozukiyo, walking in Mac Anu at
night, said "I totally forgot about the fanfic Mutsuki asked me about!",
and her second box was the red `___b_Z_[_W_f_[_^____`. Nijukata, in a
shop, did the same. The report says it happens to any town PC with more
than one box.

## What it was

The red line is `errorData` again, as in [[392]]. This time the address
was right; the record behind it was missing.

A walking PC's lines are `pcMsgN` (48 words, gcmn): the greeting, the line
after three talks, an empty one, then three lines for each story stage
(`talkNum[0]` 0-6, set by the events' `talk_num 0 N`). `TalkMenu` opens
`msg[(talkNum[0] + 1) * 3 + talkNum]`; `Check(1)` follows a record of
emode 1 with the next one in memory (`rec + 12`). Mutation's `TalkMenu`
and `PcMenu` read them as Infection's do (MUT gcmn 0x0056cc00,
0x005609e0).

Many of Mutation's lines at stories 3-6 are two records where
Infection's were one: Oborozukiyo's line 14 is "I totally forgot about
the..." (emode 1) and "Oh well. It's not the first time!".

## Cause

The port looks records up in the build's talk tables (`PINEY/TABLES/
talk.bin`), which `piney-gen` makes from the objects the talk pages can
reach. For a later volume it took each of Infection's objects where the
carry put it, with Infection's length:

- `pcMsg1_14` is 12 bytes on Infection and 24 on Mutation. The carry
  names it at MUT 0x006610e0 with Infection's size, so the array held one
  record and `rec + 12` was in no array: `errorData`.
- Some carried records landed between Mutation's records (the carry's
  "between two rows" guess): an array at 0x00660e18, 8 bytes into a
  record, holding nothing. It hid the real record at 0x00660e20 (Wing's
  line 13), so some first boxes were `errorData` too.

On each of Mutation, Outbreak and Quarantine, 34 of the 36 walking PCs
(rows 30-65) had a line with an `errorData` box: 152 boxes, 150 of them
second boxes. Over every row with the PC flag and every line of its
table, reached or not, 284 of 3,348 boxes were. Infection's had none.

## Fix

- **The talk tables** (`piney-gen` talk.rs). Infection's closure is as
  before (each object is the one its DWARF declares). A later volume's is
  now built by shape:
  - each address reached is an object of its own shape (records while
    they have an emode, and the record an emode-1 record chains to);
  - where the carry puts one of Infection's objects at that very
    address, its kind stands and its length is a floor (`spcMsgPresent10`
    has five records of emode 0, which no shape finds);
  - objects that overlap in step are merged.
  
  Mutation's talk table grows from 157,362 to 170,640 bytes. Infection's
  is the same byte for byte. `DATA_VERSION` is 28.
- **`PcMenu`** (MUT gcmn 0x00560bf4). From Mutation on Sieg and Kaz (120,
  121) and the four sign PCs (180-183: Mimiru, Bear, Crim, A-20) get the
  walking PCs' greeting, `rand() % 3` and two rows. The port gave them
  Infection's "any other id": the table as the record and no Talk. Their
  trade counts were already the extension's.

## Checked

- **Reproduced and fixed.** In piney-game, `pc_talk` drives `FieldUi` as
  the runtime does: `PcMenu` opened on the PC, Talk, OK through every box.
  - `every_walking_pcs_every_box_is_a_line_on_every_volume`: every
    talking PC at every story stage, three talks each, on every volume.
    Before the fix 152 of 1,229 boxes on Mutation were `errorData`
    (rec 0x36f968).
  - `oborozukiyos_second_box_is_hers`: the fanfic line's second box is
    "Oh well. It's not the first time!", and Nijukata's two-box lines at
    story 6 are lines.
  - Sieg, Kaz and the sign PCs have one line of empty text, which the
    game shows as an empty box.
- **Shots** (`oborozukiyo_shot`, ignored):
  - `/mnt/data/claude/scratch/i63/before/mut-oborozukiyo-second-box.png`
    shows the report's red line under her name;
  - `/mnt/data/claude/scratch/i63/mut-oborozukiyo-second-box.png` shows
    "Oh well. It's not the first time! / You have to think positive!".
- **Against the game's own `PcMenu` and `TalkMenu`**
  (tools/test_fieldui_shop_rs.py, eemu, frame by frame):
  - `test_pc_talk_later_story`: Oborozukiyo at stories 3 and 4, Nijukata
    at 5 and 6, rows 30 and 45. From Mutation on also Sieg at 5 and Bear
    at 6.
  - `test_later_pcs`: rows 120, 121 and 180-183 greeted and listed.
  - Before the fix the first failed at Oborozukiyo's story 3, frame 136
    (the window's emode 4, `errorData`'s, against 1). The second failed
    at once without the `PcMenu` change.
  - After it `TalkPages` and `MerchantLists` match on all four volumes,
    but for Quarantine's merchant lists (below).
- **The other talk pages.** tools/test_fieldui_talk_rs.py now matches
  whole (41 cases) on Mutation, Outbreak and Quarantine. Its Outbreak and
  Quarantine `test_admin` and `test_breeder_about`, which [[392]] left
  differing in the window's mode on a chained record, were this cut too.
  So were the shop harness's Outbreak and Quarantine `test_other_talk`.
  The shop harness is whole on Outbreak.
- piney-game's suite (four threads), the tests of piney-fieldui,
  piney-gen and piney-data, `piney-gen gen --check`, clippy on the
  touched crates, fmt, `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing of #63. Two shop-harness differences remain,
both from before this entry and not about talk:
- Quarantine's Buy pages (`test_buy`, `test_buy_cancels`,
  `test_random_buy`, frame 40) and merchant lists (`test_merchant_pages`,
  `test_random_lists`).
- Mutation's three Recorder cases ([[392]]'s 10-frame hold).
