---
number: 390
title: The Ryu Books open their own covers on every volume: each ccThBook's stream base, the covers' palettes, the later volumes' 140 streams and their books' pages
date: 2026-10-08
area: video, ui, volumes, test
files: crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/book.rs, crates/piney-data/src/pack.rs, crates/piney-stream/src/table.rs, crates/piney-stream/src/subtitle.rs, crates/piney-stream/src/file.rs, crates/piney-stream/src/draw.rs, crates/piney-stream/tests/stream.rs, crates/piney-stream/tests/subtitles.rs, crates/piney-fieldui/src/book/mod.rs, crates/piney-fieldui/src/book/pages.rs, crates/piney-fieldui/src/menus/merchant.rs, crates/piney-fieldui/src/menus/drain.rs, crates/piney-desktop/src/message.rs, crates/piney-game/src/world.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/session/tests/ryu_book_covers.rs, crates/piney-audio/tests/stream.rs, tools/test_stream_rs.py, tools/test_fieldui_rs.py, docs/engine/stream.md, docs/engine/field-ui.md, UNKNOWNS.md
resolves: 317, 347
---

# 390. The Ryu Books open their own covers on every volume: each ccThBook's stream base, the covers' palettes, the later volumes' 140 streams and their books' pages

Issue #61 (Mutation): every Ryu Book opened on the wrong movie. Book I
played the Altimit boot, Book II the gate hack, Book III a broken drain,
Books IV-VI the drains. Books VII and VIII played covers, but Books I's and
II's. The books themselves worked.

## Cause

The town played stream 112 + the book. That is Infection's numbering, and
it has been in the port since the books were begun (entry 317, commit
27f3df6). Entry 388 (10dab1e) only gave the cover the town's files.

From Mutation on, `streamTbl`, `streamTblE` and `strSndTbl` have 140 rows,
not 134. Six new streams (`str9802` to `str9934`) take 106-111, and every
later stream moves up six:

| stream | Infection | Mutation on |
| --- | --- | --- |
| `str6100` (the Altimit boot) | 106 | 112 |
| `str7000` (the gate) | 107 | 113 |
| `str8000` (Mutation's JP table: `strdummy`) | 108 | 114 |
| the drains `str8100`-`str8300` | 109-111 | 115-117 |
| the covers `str8801`-`str8808` | 112-119 | 118-125 |
| `STRSUB`'s `str3001`-`str3450` | 120-132 | 126-138 |

Each volume's `ccThBook` says the same in its code: `streamNum =
tsm->arg + 112` on Infection (gcmn 0x0041a9e0), `+ 118` on Mutation
(0x0042e510), Outbreak (0x00429f40) and Quarantine (0x0031c850). The
game's audio switch (`piney_audio::stream::stream_ctrl`) already knew
118-125, and the gate already knew 113.

## Fix

- **The cover's number** comes from the code. `piney-gen` reads
  `book::cover_stream` out of each volume's `ccThBook`: the `addiu` after
  its `lw $v0, 0x14(rs)`. It gives 112, 118, 118, 118. The book task
  (`piney_fieldui::book::Task`, the port of `ccThBook`) asks for
  `Cover::of(t, page)`. That is `cover_stream + page`, plus from the second
  book on its palette. The town plays the number it is given.
- **The tables' length.** `stream_lists`, `strSndTbl` and the subtitles
  are read for 134 streams on Infection and 140 later (`later(134, 140)`).
  `piney_stream::table::COUNT` became `count(volume)`. Before this,
  events 359 (RYOKO-03) and 360 (MIA) could not play streams 134-138 on
  the later volumes: `Def::read` refused them.
  - `ccEventStream` reads `evStrMsgTbl[num]` unchecked, and the generator
    reads it the same way. Quarantine's table has lines for 134-138.
    Mutation's keeps Infection's 136 slots, so its 136-139 read
    `evStrMsgTblp`'s first rows, which are null.
  - `DATA_VERSION` is 24.
- **The cover's palette** (an open question of entry 317). Once its stream
  is set up, `ccThBook` does `GetSubstAdrsF(str8800e, "MAT_clut")
  ->tex.clutChunk = GetChunkAdrsF(stream_cluts[book])` for every book past
  the first. `MAT_clut` is the book model's material, so Book III's cover
  is green.
  - The town's resident `str8800e` keeps the change. Book I read after
    Book III shows Book III's colours until the town's files go.
  - `StreamFile::change_clut` turns that into the model draw's
    `clut_swaps`. `WorldMode::cover_palette` keeps it for the town visit.
  - The page's own background (`BOOK::bg`, `ccSprite::SetTex`) copies the
    texture's own palette, so it does not change.
- **The later volumes' book pages** (an open question of entry 347).
  Running every `test_book_*` with `PINEY_VOLUME` showed Books III, IV, V
  and VIII differing on Mutation. Mutation's book.cpp, which Outbreak and
  Quarantine keep with a few changes, is ported (docs/engine/field-ui.md,
  "The later volumes' books"):
  - **`SetCharInfo`** (MUT 0x0041f320) makes 76 rows. Members 1-20 come
    first; 18-20 read the save's extension through `by_id`. Then the
    people and the players, then NPCs 181, 180, 182, 183, 121 and 120,
    with the extension's trade lists 48-53.
  - **Trade counts.** `CheckTradeCount` and `AddTradeCount` (MUT main
    0x00179ec0, 0x0017a150) became `merchant::count_at`. They use
    `by_id::trade_count` from Mutation on, which counts no one else on
    entry 0. This also mends the later volumes' trades with members
    18-20 and those NPCs.
  - The counts 17/20 and 67/76 (Book V's sum, its pad bounds, Book III's
    `/76`) come from `Env::members` and `Env::chars`.
  - The list buttons move to x 71, and Book IV's sub-window button to 55.
  - Book IV writes `BookHelp40.emode` (0x100, and 0x1100 in the
    sub-window), and the sub-window's last line gets a leading space.
    `ccMessage::Disp` (MUT 0x001b98a0) starts a speech line with 0x1000 at
    4 instead of 28 (`MsgWindow::narrow`).
  - Bosses 203-206 show "Unknown" for their first skill. That is a new
    text (`book::unknown`) after `bookNothing`.
  - Book VIII: with all nine Grunties met, `countOver` colours the names,
    and from Outbreak on the counts too. Outbreak's page then shows the
    counter's stop, and no help while row 0's reward is up (`BOOK`
    +0x5158). That reward closes the help window first, and Outbreak's
    third line is null.
  - Cancel on the food page's total goes back to the food row on Mutation
    and Quarantine, and to the first row on Infection and Outbreak.
  - **`CheckBookLimit`**: from Outbreak on the cap tables have a fourth
    volume, and `countStopMsg` has a fourth line ("in Vol. 4."). The
    generator reads four rows there. Quarantine's counters past the cap
    set `countOver` but keep their value.
- **`drain::movies(volume)`** exposes `DataDrainMenu`'s streams, so a test
  can name them without copying the table.

## Checked

- **Failing before, passing after:**
  - `each_ryu_book_plays_its_own_cover`: all eight books on all four
    volumes. Under the old numbering it failed on Mutation's Book II,
    whose gate stream never ended.
  - `a_ryu_books_cover_takes_the_books_palette`: Book I, then III, then I
    in one town, on Infection and Mutation.
  - `later_volumes_tables_match_the_game` (piney-stream): 2 x 140 streams
    on each later disc against `stream_tables_mut/out/qua.txt`, from the
    game's `ccStreamInit` and `RequestStrPlay` in eemu (the new
    `tools/test_stream_rs.py tables`).
  - `the_later_volumes_subtitles_past_stream_133`.
  - `every_hash_object_resolves_where_its_stream_plays`: now on all four
    volumes (Inf 18 desktop / 5 field / 8 town streams; Mut 55/6/8; Out
    76/8/8; Qua 102/11/8), each cover named `str880N`. Streams 134-139 open
    on the later discs.
- **The music fixtures.** `tools/test_stream_rs.py music` was rerun on the
  three later volumes with all 140 streams, and the first cover and the
  gate at their own numbers. The port matches all 1459 scenarios on each.
  Infection's fixture is unchanged.
- **The field UI harness** (`tools/test_fieldui_rs.py`) now loads on every
  volume.
  - The harness places the save's extension (`place_save`), gives the
    members charTbl's names (`Scenario.named`), and on Outbreak and
    Quarantine tolerates `GtHackMenu` and the noise block having no name.
  - New cases: `test_book_3_later`, `_4_later`, `_5_later`, `_8_later`,
    `test_book_limits` (every counter past its cap) and `test_book_all`
    (Books III and IV complete).
  - All 16 book cases match on all four volumes (the four `_later` ones
    skipped on Infection). The probe's book stream is now the game's
    number. Before this, `test_book_1` on Mutation would have shown 112
    against the game's 118.
  - The whole harness: Infection and Mutation match on all 74 cases.
- **Shots** in `/mnt/data/claude/scratch/i61`, Mutation, stream frame 90.
  - `before/mut-book1-stream112.png`: the Altimit boot text.
  - `before/mut-book3-stream114.png`: Kite in a drain, as in the report.
  - `after/mut-book1-stream118.png`: Book I's red cover.
  - `after/mut-book3-stream120.png`: Book III's green cover.
- piney-game's suite (four threads), the tests of piney-fieldui,
  piney-stream, piney-audio, piney-desktop, piney-data and piney-gen,
  `piney-gen gen --check`, clippy on the touched crates, fmt, `cairns check`
  and `tools/docs.py check` pass.

**Still unknown:** the field UI harness on Outbreak and Quarantine, outside
the books. 7 (OUT) and 8 (QUA) of its other cases differ:
- three of noise, whose `Disp` block it does not find there (`DISP_NOIZ`
  and `GT_HACK` are left empty);
- `test_gate_refusal`;
- `test_chat_member_refusals` and `test_low_hp` (condition icons);
- `test_spring_throw`;
- Quarantine's `test_skill_refusals`.

These are those menus' ports on the volumes after Mutation, not read here.
The message window's `mode` for a frame after a book closes (entry 347)
stays open. Not checked: whether the swap lands before the cover's first
frame is drawn or one frame later. The port applies it from the first.
