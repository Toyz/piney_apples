---
number: 615
title: Each disc's own new game and CONVERT on the desktop
date: 2026-11-19
area: script, test, volumes, engine
files: tools/test_event_vm.py, crates/piney-game/src/desktop.rs, crates/piney-game/src/desktop/openings.rs, crates/piney-game/src/world.rs, crates/piney-game/tests/opening_fixture.txt, crates/piney-game/tests/opening_fixture_mut.txt, crates/piney-game/tests/opening_fixture_out.txt, crates/piney-game/tests/opening_fixture_qua.txt, docs/engine/event-vm.md, docs/engine/desktop.md, docs/formats/save.md
started: 2026-11-19T02:04:08-07:00
took: 6h 34m
---

# 615. Each disc's own new game and CONVERT on the desktop

Every disc's title has New Game; from Mutation on it also has CONVERT, the
previous volume's cleared save carried over. Both start from the same boot,
and each disc opens its own story on the desktop: event 1, 101, 201 or 301.
`test_event_vm`'s `test_new_game_desktop` played Infection's opening on
every disc (the boot's `ccStartEvent(1, 0)`, then mails 4, 5 and 320). That
is why it failed on the three later discs. It now runs each disc's own new
game and CONVERT. The port's boot was wrong on the later discs: its replay
of the earlier stories ran without the story areas.

## Each disc's new game, from its code

- **The boot.** `ccThMother` calls `ccStartEvent(volumeNum, 0)` on
  `Init(1)`'s save (INF main 0x00167c14, MUT 0x00167d94, OUT 0x0016701c,
  QUA 0x00166eac). It counts `clearFlag` up to `volumeNum - 1` and replays
  the earlier main stories at level 1 with `ccEventFlagSet` (0-49; 100-149
  from Outbreak; 200-249 on Quarantine). With flag 0 it marks events 62,
  163 and 263 done, then event `100 * (volumeNum - 1)` (MUT 0x001ca9d0:
  `+0x56e8`, `+0x5a10`, `+0x5d30`, then `+0x54f8 + 8 n`). Outbreak's also
  sets `crisis`.
- **New Game.** `ccSetupDemo`'s `NewGame(1)`, then `NewGame(0)` (ccThDemo
  return 2).
- **CONVERT.** `NewGame(1)`. Then `LoadDataPrevReq` (MUT 0x00171c00)
  overwrites the record with the previous volume's slot; on Mutation the
  extension keeps `NewGame(1)`'s. Then ccThDemo's return 4:
  `ccStartEventConvert` (MUT 0x001cac50, OUT 0x001c12f0, QUA 0x001c8970)
  marks event 100, 200 or 300 done again, and `ConvGame` resets the
  characters outside the party, runs `LoadGame` and `InitTradeItem`, and
  sets `newGameFlag` 2. Name entry then returns at once.
- **The desktop.** `ChangeRequest(3, 7)`. The opening's header waits for
  event `100 * (volumeNum - 1)`, and it plays in the setup's pass at phase
  0:
  - INF event 1 (`eventTblM101` 0x00317ec0): setup lines, name entry, mails
    4, 5 and 320. Every operation but the mailer is held until mails 4 and 5
    are read; block 2 then releases them and closes the event.
  - MUT event 101 (`eventTblM201` 0x003360a0): block 0 has no setting.
    Name entry (+0x50), streams 23 and 24, mail 374 (+0x70), `grunty_mail`
    (+0x80), mail 25 (+0x82). Nothing is held. Blocks 2-11 mail a saved
    member and answer replies. The event goes on in Dun Loireag.
  - OUT event 201 (`eventTblM301` 0x003340b0): mails 48 (+0x42) and 323
    (+0x50). Block 1 holds operation 0 while `eventStatus[0]` is 0; block 3
    answers a held one ("Let's read the news."). News 19 read: block 2
    releases it and sets `eventStatus[0]` to 1.
  - QUA event 301 (`eventTblM401` 0x002306d0): block 0 posts news 59, 60
    and 27. Block 1 has name entry, streams 77 and 78, mails 74, 75 and 372,
    and holds every operation but the mailer. Mail 74 read: block 3 opens
    the Omega server. Mails 74 and 75 read: block 4 releases the
    operations.
- **`grunty_mail`** (case 168, MUT main 0x001c77a8). Unless
  `mailList[324]` is set, it sends mail 324 once some town 1-4 has a grown
  Grunty in each of its three pens (`ccPgAdultCheck`, MUT gcmn 0x0052d310,
  reading only the save). GCMN.PRG is in by then: the opening's
  `overlay 3`.

## The harness

`GameEvents` runs a given executable now. It takes the save extension
(the `$gp` pointer to `EXT`) and, for the openings, GCMN.PRG with its
static initialisers. It reads name entry's three DESKTOP.PRG calls from
`Execute`'s case 156; they were Infection's addresses. `boot()` is
`ccThMother`'s part.

- `title_new_game` and `title_convert` run `NewGame`,
  `ccStartEventConvert` and `ConvGame` with DEMO.PRG in, in a machine
  where the save sits at `piney_demo::newgame::SAVE_VA` (character 0's
  name pointer).
- The load runs through `tools/save.py`'s `Game`: this volume's own
  `LoadInfoPrevReq` and `LoadDataPrevReq` read the card, giving "Load
  complete." (0x2017, Quarantine's 0x1017).
- `OPENINGS` holds each disc's expectations, read from its script with
  the offsets cited: the mails, `CheckOperate`'s answers to the player's
  actions, the block bits, closed or not, and the `eventStatus` it sets.

The previous volume's save:
- **Mutation.** The user's cleared Infection save
  (`work/savecards/BASLUS-20267DOTHACK/dhdata01`: level 34, `clearFlag`
  1, events 0-31 and 50-62 done). As exported it does not load: the file
  is 0x8d84 bytes and the index sums all of them, so Mutation's
  `LoadDataPrevReq` answers 0x203f, the sum (as Infection's own load
  would). Its first 0x8530 bytes are the `ccSaveData`. Infection's
  `LoadGame` points the names at the save, and Infection's own
  `InitInfo`, `MakeDir` and `SaveDataReq` write the card. Its index
  record says level 34, clear 1.
- **Outbreak and Quarantine.** No cleared Mutation or Outbreak save
  exists. Each earlier disc carries the user's save on with its own code:
  its boot, its CONVERT of the card before, then
  `ccStartEvent(volumeNum + 1, 0)` (its main story replayed, `clearFlag`
  up to `volumeNum`), then its `SaveDataReq`.

What it shows:
- **New Game.** The pass settles in frame 59 on Infection and frame 5 on
  the others. New mails, all state 1: INF 4, 5, 320; MUT 374, 25; OUT 48,
  323; QUA 74, 75, 372.
- **`CheckOperate`** with the player's actions: INF (3 0) (1 1) (3 1);
  MUT (3 1) (1 1) (0 1); OUT (0 0) (1 1) (0 1); QUA (3 0) (1 1) (3 1).
- **Block bits at the end.** INF 0x5, closed; MUT 0x1; OUT 0x5 with
  `eventStatus[0]` 1; QUA 0x1b with `eventStatus[51]` 1.
- **CONVERT on Mutation** gives 374, 324, 25, 30 and blocks 0x401. The
  user's pens hold grown Grunty, and block 10 (+0x17c `member_saved 11`)
  mails Natsume's 30.
- **CONVERT on Outbreak** first sends 119-121 from side events 159-161.
  They open on Infection's 58, 60 and 61 and Mutation's 103 and 108, and
  `ccThEvent` runs side events before the main story. Then the opening's
  own 48 and 323.

The Infection fixture (`vm_fixture.txt`) regenerates byte for byte the
same.

## The port

`desktop::openings::openings_match_the_game` (new) builds each start with
the port's own boot, `NewGame` or `start_event_convert` and `conv_game`.
It plays the desktop through the Bridge, which answers the story areas
and `grunty_mail`, and compares with `test_event_vm.py openings`' numbers:
- `tests/opening_fixture*.txt`, committed, for New Game;
- `work/analysis/openings/convert_*.txt` for CONVERT, kept out because it
  holds the user's save.
The start is compared as a hash, since it holds `charTbl`'s rows. Then
every phase, every call and `CheckOperate` answer with its frame, and the
slot at three points.

Infection matched at once. Mutation's New Game did not: its start
differed in `impItemList` 12 and 14, the gate lists and order, and the
word list. `desktop::boot` ran `ccStartEvent` with a `HostState` that had
no story areas, so the replayed `gate_add` put every area on server 0
with no words, and `virus_core` took no protect items. Every later
disc's new game in the port, the session's, `--mode world`'s and the
story starts', began with Infection's (and Mutation's) gates and words
wrong. `boot` now reads `Announcements::from_disc` and `boot_with` hands
the story areas to the Bridge. With that, all seven runs match: New Game
on four discs and CONVERT on three.

`world::new_game_events` (`--mode world` with the scripts) waited for
event 1 to close after reading mails 4 and 5, so on a later disc it gave
up ("event 1 does not close"). It now reads what the disc's opening
waits for: `opening_reads` finds the block whose `del_operate` undoes one
of the opening's `add_operate`s and takes its `mail_got` and `news_read`
conditions. It then plays until no operation is held. Infection is
unchanged. `the_later_openings_reach_the_world` takes all three later
discs to Log in.

## Checks

- `test_event_vm` on all four discs: Infection OK (CONVERT skipped: its
  title never asks for it), Mutation, Outbreak and Quarantine OK.
- The sweep (103 harnesses, 412 runs) on 4f03e60, the code of this entry:
  412 of 412 rc 0, and against `summary-main.txt` the one change is
  `test_event_vm` on the later discs, FAILED before and OK now.
  `test_piros_rs` and `test_stream_rs` print no unittest line, as on main.
  A sweep harness replaced the worktree's `tools/eemu_rs.so` symlink with
  a build; the symlink was put back, main's file untouched.
- Battle pins: 291 calls, 0 differ (probe wt-newgame at 4f03e60).
- Golden shots: a same-day base from a clean worktree of main ea71d93
  (`newgame-base`) equals `world2/shots/base` (4,550 files, 0 differ); the
  tip (`newgame-tip`) against it: 4,550 files, 0 differ. No shot starts a
  later disc's new game through the boot's replay.
- Crate tests 785 passed, 0 failed; the piney-game suite 351 / 0 / 153
  (349 and the two new tests); clippy on all targets and features clean;
  fmt; `piney-gen gen`, `syms`, `carry --check` current; `cairns check`;
  `tools/docs.py check`.

**Still unknown:**
- No cleared Mutation or Outbreak save exists, so Outbreak's and Quarantine's CONVERT run on the user's Infection save carried on by each earlier disc's own code, its story brought forward by `ccStartEvent(volumeNum + 1, 0)` instead of played. Their openings' own mails, answers and blocks do not depend on it, but what else the carried save opens does: Quarantine's first desktop sends 119-121 again because the chain never plays Outbreak's side events 159-161. A real cleared Mutation or Outbreak save would settle it.
- What wrote the user's 0x8d84-byte slots into Infection's directory (worklog 338). Neither Infection's own load nor Mutation's CONVERT takes them as they are.
