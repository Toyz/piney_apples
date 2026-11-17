---
number: 609
title: The desktop, the top page and the eemu harness on the later discs
date: 2026-11-16
area: ui, volumes, tooling, test
files: crates/piney-desktop/src/name_entry.rs, crates/piney-desktop/src/data.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/nameentry.rs, crates/piney-data/src/tables/sjis.rs, tools/volume.py, tools/test_desktop_rs.py, tools/test_desktop_name_rs.py, tools/test_desktop_savemenu_rs.py, tools/test_toppage_rs.py, tools/test_eemu_rs.py, tools/test_stream_rs.py
started: 2026-11-16T14:30:53-07:00
took: 6h 46m
---

# 609. The desktop, the top page and the eemu harness on the later discs

## What failed

On main (18bdc71), the 76-harness sweep left these failures in this area:

- `test_desktop_rs`: 3 errors each on Outbreak and Quarantine.
- `test_desktop_name_rs`: 2 failures each on Outbreak and Quarantine.
- `test_desktop_savemenu_rs`: 2 failures on Quarantine.
- `test_toppage_rs`: rc 1 with no output on Mutation, Outbreak and
  Quarantine.
- `test_eemu_rs`: 1 error on every disc.

`test_desktop_data_rs` is not in the sweep, because it names no
`PINEY_VOLUME` itself. It failed 3 times on Quarantine. Four of the
causes were in the harnesses and three were in the port.

## The harnesses

- **Outbreak's own vector insert.** `AddMailList`, `AddWallList` and
  the audio lists call `vector_pod<Ui>::insert`. The harness hooks that
  call with a model. Infection and Mutation call main's insert (INF
  0x0010d760). Outbreak's and Quarantine's desktop overlay has its own
  stripped copy (OUT desktop.prg 0x00414260, QUA 0x00306b60). That copy
  reports the length error with `fprintf` and `abort` instead of
  throwing. So nothing was hooked, and the audio lists ran into main's
  allocator (the `mfc0` in `DIntr`). `volume.vector_inserts` now finds
  the inserts by name, or by their code: the call target that loads
  "vector::insert length error" before its first `jr $ra`.
- **@1489 is on no later disc.** The top page harness read each town's
  server at `inf_va(0x00306dc0)`. Only Infection has that table: its 32
  bytes are in no later executable or overlay. From Mutation on,
  `ChangeScene` has no `lui`/`lq` copy and stores the town into the
  server unconditionally (MUT main 0x0016756c, OUT 0x001667bc, QUA
  0x0016664c). The carry is right not to carry it. The harness now reads
  the table from `ChangeScene`'s own code, and where there is none it
  expects the town. The port has three copies of `ChangeScene`: in
  piney-world, in piney-game's field host and in piney-toppage. All
  three apply Infection's table. That gives a different result from
  "server = town" only for a town of -1 or of 5 and above, and no disc
  ever passes one:
  - every `scene` instruction in the four discs' exported scripts gives
    town -2 or 0-4 (counted for each disc);
  - the gate menu's `ChangeArea(0, n)` towns are 0-4;
  - only `ChangeScene` writes `lastTown`, and only from those towns.
- **Quarantine's overwrite scenario.** `test_desktop_savemenu_rs` made
  the warning with a record whose clear flag was the volume's number
  plus one. On Quarantine that is 5. `CheckRightInfo` refuses it (clear
  flag 0-4, QUA main 0x00170dbc, alike on every disc), so the whole
  index read as broken. The menu never closed and result 0x1030 never
  came. The scenario now gives the save its own flag, `min(VOL, 3)`,
  and gives the warning's record that flag plus one. Nothing changes on
  the first three discs; Quarantine gets 3 and 4.
- **test_eemu_rs.**
  - On Infection, its demo driver loaded `test_demo_rs.TITLE1`, which
    [[519]] renamed to `TITLE` (the volume's title).
  - On the later discs it could not import `test_toppage_rs` (see
    above). After that fix, three more problems showed:
    - Its fuzz took `inf_va(0x00400800)`. That is the base of the
      overlay window, a constant; it is now `program.overlay_space[0]`.
    - Its demo driver called the fades at Infection's addresses. It now
      calls them by name.
    - Its stream driver read `str6100` by name from STRCMN.BIN.
      Outbreak and Quarantine hold no Japanese archive, and every gzip
      member in their archives is named `tmp.ccs`, so only the tables
      can find a file there.
  - The new `test_stream_rs.scene_by_table` reads the first stream from
    106 on whose header archive is on the disc. It reads it as
    `ReadThread` does: the sector is `lsn + ceil(header ofs / 2048) +
    ceil(file ofs / 2048)`, it reads the file's size, then inflates the
    gzip. The archive for each header type comes from `ccCdInit`:
    STRCMNE.BIN's record plus 0x24 per type. That record is at ccCd
    +0x108 on Infection, +0x12c on Mutation and +0x150 on Outbreak (OUT
    0x00158654).
  - The result is `str6100` as stream 112 on Outbreak, because stream
    106 is in STR4E.BIN, which only Quarantine has. On Quarantine it is
    `str9802` as stream 106. Infection and Mutation still read
    `str6100` by name.

## The port

| what | Infection | from | the code |
| --- | --- | --- | --- |
| the grid after the start pages | `SetHira` (MainAct 2); its `PlayHira` sends NowMode 2 on to `SetEigo` two frames later | OUT: `SetEigo` (MainAct 6) at once | `PlayStartInfoMsg`, desktop.prg OUT 0x0042a708, QUA 0x0031b0e8 |
| the character names `CheckName` refuses | 21 | MUT: 24 (tsukasa, subaru and sora added) | its loop's `slti`, MUT 0x0042bf90 |
| `m_Blockbuf` with the cursor off the grid (rows -3 to -1 and 18 to 20) | NULs | OUT: the bytes before `EigoBlock`, which are `@2319`'s last pointers ("G", "", "G"; QUA "6", 0x90 0x76 "6", "6") | `SetBufBlock`, `EigoBlock + 14 NO` |
| a card message on the Data screen | closes on decide or cancel | QUA: decide only | `SlotStateData`, QUA 0x003128f8; OUT 0x0041fff4, 0x00420000 |

The generated tables had the refused names as "alike on every volume",
taken from Infection's 21. piney-gen now reads the count from
`CheckName`'s loop. The rows around `EigoBlock` are two generated
entries, `eigo_before` and `eigo_past`, read past the table's ends in
the same way that `english_font_ofs_s_past` reads past its font. The
bytes 0x90 0x76 added one character to `sjis.rs`. The port's `Texts`
holds the grid rows that `SetBufBlock` can reach, -3 to 20 (the
cursor's x is 0-14 and its y -1 to 6). Those edge rows are never drawn,
because only rows 0-17 are, and `CutInBlock` runs only on the grid. The
member's value is still the game's.

`HiraBlock`'s neighbouring rows from -3 to 20 all start with a NUL on
Infection and Mutation, the only two discs that reach it, so its rows
stay as they were. `test_desktop_name_rs` now types the three new names
from Mutation on and checks for `Err` 3.

I also compared the shape of every desktop and top page function
between Outbreak and Quarantine. Only `SlotStateData` differs, besides
the staff roll, which Quarantine's desktop does not have. Among main's
save and card functions, only `ccSaveData::NewGame` differs, and
`test_save_init_rs` covers it.

## Checks

At be19db8 (the code of this entry):

- **Harnesses in this area.** All OK on all four discs:
  - from the sweep: `test_desktop_rs`, `test_desktop_name_rs`,
    `test_desktop_savemenu_rs`, `test_desktop_menu_rs`,
    `test_desktop_announce_rs`, `test_toppage_rs`, `test_demo_rs`
    (Infection skips 1) and `test_eemu_rs`;
  - run apart: `test_desktop_data_rs`.
- **The sweep against main's.** 304 runs each. 11 fail here, 23 on main.
  These 12 now pass: `test_desktop_rs` OUT and QUA,
  `test_desktop_name_rs` OUT and QUA, `test_desktop_savemenu_rs` QUA,
  `test_toppage_rs` MUT, OUT and QUA, and `test_eemu_rs` on all four.
  Nothing that passed on main fails. The other failures are
  `test_dungeon_rs`, `test_enemy_weapon_rs`, `test_sound_rs` and
  `test_staffroll_rs` on the later discs, as on main.
- **Battle pins.** 291 calls, 0 differ. The probe is the worktree's, at
  be19db8.
- **Golden shots.** I shot a base from 18bdc71 today in its own
  worktree. It matches `world2/shots/base` (4,550 files, 0 differ). The
  tip, shot today: 4,550 files, 0 differ.
- **Tests and tools.** Crate tests: 45 ok, 0 failed. The piney-game
  suite: 349 passed, 0 failed, 153 ignored. `gen --check`, `syms
  --check` and `carry --check` are current on every volume. clippy
  (all targets and features), fmt, `cairns check` and `docs.py check`
  are clean.
- **Merge.** After the merge, main's `work/` needs `piney-gen gen`,
  which rewrites `nameentry.bin` for every volume. Nothing else in the
  tables, the syms or the carry changes.

**Still unknown:**
- `test_toppage_rs` checks the game's scene after Log in against the
  table it reads from the code, but it does not compare the port's
  (`toppage_probe` reports only the request). The port's server is the
  same for towns 0-4 on every disc, as shown above.
- The later discs' failures that remain in other areas (`test_dungeon_rs`
  2, `test_enemy_weapon_rs` 1, `test_sound_rs` 1, `test_staffroll_rs` 1)
  are not this entry's.
