---
number: 387
title: CONVERT carries a part's clear data into the next: the later titles read the previous part's saves on the same card, and ConvGame makes them theirs
date: 2026-10-07
area: save, ui, test
files: crates/piney-desktop/src/savesys.rs, crates/piney-desktop/src/card.rs, crates/piney-demo/src/dataload.rs, crates/piney-demo/src/card.rs, crates/piney-demo/src/dialog.rs, crates/piney-demo/src/newgame.rs, crates/piney-demo/src/lib.rs, crates/piney-demo/src/seam.rs, crates/piney-demo/examples/demo_probe.rs, crates/piney-data/src/save.rs, crates/piney-data/src/pack.rs, crates/piney-data/src/tables/title.rs, crates/piney-gen/src/manifest.rs, crates/piney-event/src/state.rs, crates/piney-event/src/vm/mod.rs, crates/piney-fieldui/examples/newgame_probe.rs, crates/piney-desktop/examples/desktop_probe.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/convert.rs, crates/piney-game/src/session/tests/ending_save.rs, tools/test_demo_rs.py, tools/test_save_init_rs.py, docs/engine/title.md, docs/formats/save.md, README.md, UNKNOWNS.md
resolves: 43, 201, 226
---

# 387. CONVERT carries a part's clear data into the next: the later titles read the previous part's saves on the same card, and ConvGame makes them theirs

Issue #55 (ThreePendant): Mutation's CONVERT did not see Infection's
cleared save. It said "There is no .hack//MUTATION saved data." both
with and without a Mutation save on the card, and after the Infection
file was pasted into Mutation's folder.

## How the game converts

The title's fourth item opens `NextDataLoad_Control`: `Data_Control` with
`m_PrevFlg` 1 (docs/engine/title.md, "CONVERT").

- **The index.** With the flag, `Slot_Select` and the re-reads call
  `LoadInfoPrevReq` (MUT 0x00171b50), not `LoadInfoReq`. It sets
  `operate` 2, which lets the card lack the played disc's own directory.
  Proccess 4 (MUT 0x00172514) reads `mcDirName[volumeNum - 2]`'s index
  into `infoPrev`. On Mutation that is Infection's `BASLUS-20267DOTHACK`,
  on the same card. Without it the message names Infection (0x103b).
- **The list.** Ok takes a record only when its `clearFlag` is at least
  `volumeNum - 1` (`LoadDataPrevReq`, proccess 7). `SetLoadPar` shows
  such a save as "Vol.N CLEAR" and any other used one as "No Data Flag".
- **The load.** "Load this data?" (0x8012) yes is proccess 9. It reads
  the previous part's slot at its own size (Infection 0x8530, the others
  0x8d84), checks the sum against `infoPrev` and copies it in member by
  member, as proccess 8 does.
- **The carry.** `ccThDemo` then calls `ccStartEventConvert` (event
  100·(volumeNum−1) done) and `ConvGame`. `ConvGame` resets every
  character not in `partyMemberFlag` to `charTbl`, with empty items and
  skills, then runs `LoadGame` and `InitTradeItem` and sets `newGameFlag`
  2.
- `InitInfo` clears `infoPrev`'s records with `info`'s, on every disc.

## Cause

None of this was ported.

- `DataControl` ignored `m_PrevFlg`: CONVERT ran Mutation's own LOAD
  (`LoadInfoReq`, `operate` 1). Without a Mutation directory that is
  0x103c, the message in the screenshot. With one it listed Mutation's
  saves, and ok did nothing.
- `SaveSys` had no proccess 4, 7 or 9 and no `infoPrev`. The card could
  read only its own disc's index.
- `Request::ConvGame` reached only the session's "title asks" log.

## Fix

- **`piney_desktop`.** `MemoryCard::read_index(port, vol)` is
  `ReadSys`'s volume argument; `FilesCard` reads `<dir(vol)>/<dir(vol)>`.
  `SaveSys` gains `info_prev`, `load_info_prev_req`,
  `load_data_prev_req` and proccess 4, 7 and 9. One `read_index` and one
  `read_data` serve both loads, by volume. `prev_volume` clamps as the
  game does.
- **`piney_demo::dataload`.** `load_info` picks the request by
  `prev_flg`. `Data_Select` takes a cleared record only, and `SetLoadPar`
  draws CONVERT's panel and "No Data Flag" (`STR_NOFLG_DETA`, a new
  `title` table entry; data version 23).
- **The carry.** `newgame::conv_game` and `start_event_convert`, applied
  in the title when `Main` returns 4, as `load_game` is for 3. The session
  treats `Request::ConvGame` as `LoadGame` (settings, volumes, display
  offset). `NewGame`'s row copy and `InitTradeItem` became functions both
  share. The VM's unused copy of `ccStartEventConvert` went, and the done
  and closed bits are `piney_data::save::EVENT_DONE`, `EVENT_CLOSED`.
- **Quarantine.** Its `MainProccess` leaves "Load complete." as 0x1017,
  not 0x2017 (`savesys::load_done`). Its recompiled `Data_Control` plays
  the cancel sound where a step acts on cancel, not in `Main`, and
  acknowledges messages with ok only (`dataload::CancelSound`). Both
  touch its LOAD as much as its CONVERT.

The port keeps one card for all four parts (`memcard/slot1` in the
port's folder), as the console has one memory card, so nothing about
where saves live changed. The player's own steps: save after
Infection's staff roll into `BASLUS-20267DOTHACK`, leave it there, and
choose CONVERT on Mutation's title.

## Checked

- **`tools/test_demo_rs.py`'s `test_data_load_prev`**, new: CONVERT
  frame by frame against Mutation's, Outbreak's and Quarantine's own
  code (`NextDataLoad_Control::Main_Control`, `ccSaveSys` native, `ccMcard`
  answered from the card's files by the volume each call names). Cards
  hold the volume before's saves, cleared or not, with good and bad sums,
  missing and short files, and with and without the disc's own folder.
  It compares state, texts, packets, sounds and the whole slot after.
  The first Mutation run caught `InitInfo`'s `infoPrev` clear. The first
  Quarantine run caught 0x1017 and the moved cancel sound.
- `test_data_load` (LOAD) now runs on every disc with `PINEY_VOLUME`
  (the probe's `load` takes the disc's volume, the harness the files and
  the slot with its extension). It passes on all four. The whole harness
  passes on Infection. `test_boot_mem_card` passes on Quarantine. The
  probe had not compiled since the title strings became `LazyLock`s.
- **`tools/test_save_init_rs.py`'s `ConvGameLaterVolumes`**:
  `ccStartEventConvert` and `ConvGame` on six random records and
  extensions per later disc, party members none, all or some, against
  the port's, every byte of both.
- **`convert_carries_infection_s_clear_data_into_mutation`**: Mutation
  from power-on to CONVERT on a card with Infection's saves (one cleared,
  one not), without and with a Mutation save. The list holds Infection's
  records, ok on the uncleared one does nothing, and the cleared one
  carries over. Name, Kite's level, a member's level, the clear flag and
  play time are kept; a non-member is back to `charTbl` with its items
  emptied; event 100 is done and `newGameFlag` is 2. With the old
  `LoadInfoReq` the test never reaches the list.
- **`the_ending_s_save_converts_into_mutation`**: Infection's event 31
  played to its save menus, the clear data saved to an empty card, then
  Mutation's title on that card. CONVERT lists it as Vol.1 CLEAR and
  carries the name, level and play time. Mutation's setup runs with its
  scripts (no name entry), and the desktop plays on the carried name.
- **`outbreak_and_quarantine_convert_the_part_before`**: Mutation's
  cleared save into Outbreak (event 200) and Outbreak's into Quarantine
  (event 300).
- **`mutation_s_convert_reads_infection_s_saves`**,
  **`later_converts_read_the_whole_slot`**: the card and `SaveSys`
  alone, the 0x103b message, the extension kept or read.
- piney-game's suite (four threads), the suites of piney-data,
  piney-desktop, piney-demo, piney-event, piney-fieldui and piney-gen,
  clippy, fmt, `cairns check` and `tools/docs.py check` pass.

**Still unknown:** nothing new from the console. The tests start from
the port's own Infection saves and from the game's code on random cards.
A save made on a PS2 and carried by a PS2 has not been compared, which
needs a real memory card's Infection clear data and the Mutation slot a
console makes from it (PCSX2 would do).
