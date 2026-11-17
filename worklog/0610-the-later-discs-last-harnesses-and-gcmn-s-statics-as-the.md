---
number: 610
title: The later discs' last harnesses, and gcmn's statics as the overlay loads
date: 2026-11-17
area: battle, ui, audio, world, volumes, tooling, test
files: crates/piney-battle/src/prim.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-data/tests/sound.rs, crates/piney-desktop/tests/staffroll.rs, crates/piney-fieldui/src/disp.rs, crates/piney-world/src/dungeon/mod.rs, docs/engine/battle.md, docs/engine/dungeon.md, docs/engine/field-ui.md, docs/engine/sound.md, docs/formats/prg.md, tools/dungeon.py, tools/refscan.py, tools/sound.py, tools/test_battle.py, tools/test_battle_event_rs.py, tools/test_battle_flow_rs.py, tools/test_battle_frame_rs.py, tools/test_battle_items_rs.py, tools/test_battle_navi_rs.py, tools/test_desktop_data_rs.py, tools/test_dungeon_rs.py, tools/test_effect_skill_rs.py, tools/test_field_rt.py, tools/test_fieldui_option_rs.py, tools/test_fieldui_rs.py, tools/test_fieldui_trade_rs.py, tools/test_lights_rs.py, tools/test_sound_rs.py, tools/test_staffroll_rs.py, tools/volume.py
started: 2026-11-16T21:19:53-07:00
took: 5h 53m
---

# 610. The later discs' last harnesses, and gcmn's statics as the overlay loads

## What failed

On main (934b942), the sweep still failed in four harnesses on the later
discs:

- `test_dungeon_rs`: 2 failures each on Mutation, Outbreak and Quarantine.
- `test_enemy_weapon_rs`: 1 failure each on Outbreak and Quarantine.
- `test_sound_rs`: 1 error each on Mutation, Outbreak and Quarantine
  (`IndexError` in `sound.load_bgm`).
- `test_staffroll_rs`: 1 failure each on Mutation and Outbreak, and 1
  error on Quarantine (no `SetLayerCenter` symbol).

None of the four was a port bug. Following the weapon failure turned up
one in [[607]]'s work instead. The harnesses build the game's memory from
the PRG file, and they never run what `mwLoadOverlay` runs after the read.

## gcmn's statics as the overlay loads

`ccPrimPacket::makePacket` sets ADC on a pair that starts a run, has a
point behind the eye, or has both points off the screen.

- Infection and Mutation compare with `slti` 5633 and 4609 (INF gcmn
  0x004386dc, MUT 0x0044e07c).
- Outbreak and Quarantine read two gcmn statics and shift them left by 4
  (OUT makePacket 0x00449100 reads 0x006dc5a8 and 0x006dc5b0; QUA
  0x0033ba10 reads 0x005d4fa8 and 0x005d4fb0).

The PRG holds both words as 0. `test_enemy_weapon_rs` ran makePacket on
the file's image, so its bounds were 0, and every pair off the screen's
centre carried ADC. [[607]] read the condition icons' y the same way (OUT
`Disp` 0x00539ad8 reads 0x00720c90, QUA 0x0042c3e8 reads 0x00619690). It
found the word 0 in the PRG and no store in the code, so it moved the
icons to the screen's top on those two discs.

`mwLoadOverlay` (INF main 0x001002c0, OUT 0x001002e0) does more than read
the file:

1. `mwBload` reads the whole file to the load address. The PRG has no
   relocation table. Besides text and data it holds only its 0x40-byte
   header: magic, id, load address, text, data and bss sizes, the
   constructor table's bounds and the name. So nothing relocates the
   image.
2. It calls `FlushCache(2)`.
3. It clears `bss_size` bytes past the file.
4. `__initialize_cpp_rts` runs the static initialisers in the constructor
   table. CodeWarrior puts them at the tail of `.data`, just before the
   table.

Outbreak's and Quarantine's gcmn has four initialisers:

| OUT | QUA | writes |
| --- | --- | --- |
| 0x0073d680 | 0x0063a200 | 0x006dc5a8 = `(int)(96 + (int)(f[0x006dc600] / 2.0))`; 0x006dc5b0 = the same of f[0x006dc5f8] (QUA 0x005d4fa8 and 0x005d4fb0, from 0x005d5000 and 0x005d4ff8) |
| 0x0073d6f0 | 0x0063a270 | `ccDamUprStr`'s list root (main) |
| 0x0073d730 | 0x0063a2b0 | 0x00720c90 = 352 - [0x00720c88] (QUA 0x00619690, from 0x00619688); `menuList`'s 93 constructors |
| 0x0073d790 | 0x0063a310 | `ccSPC::Initialise` on `ccSpcManager` |

On both discs the PRG holds the three source words as 512.0, 384.0 and
32.

I ran each disc's own `mwLoadOverlay` on the disc's GCMN.PRG in eemu (the
Rust machine), then read the words:
- First the overlay window was filled with 0xa5, so any word the loader
  did not write would show.
- `mwBload` was given the file's bytes, and `FlushCache` did nothing.
- Everything else was the game's own code.

| disc | across | down | icons' y | file bytes the load changes |
| --- | --- | --- | --- | --- |
| Infection | literal (`slti` 5633) | literal (4609) | literal 320 | 0 |
| Mutation | literal (5633) | literal (4609) | literal 320 | 0 |
| Outbreak | 0x160 = 352 | 0x120 = 288 | 0x140 = 320 | 6, the three words |
| Quarantine | 0x160 = 352 | 0x120 = 288 | 0x140 = 320 | 6, the three words |

The neighbours after the load:
- Before 0x006dc5a8 is the jump table `@864` (0x004488f0 ...). The words
  at 0x006dc5ac, 0x006dc5b4 and after are 0.
- Around 0x00720c90 come 32, 0, 32, 0, then 0x140, 0, then "PERSONAL".
- Infection and Mutation have no such words: they compare with literals,
  and their three initialisers write only `.bss` and main.

So on every disc the bounds are 352 << 4 = 5632 and 288 << 4 = 4608, and
the icons' y is 320.

`tools/refscan.py` missed the stores because it read only the overlays'
text as code. It now scans the initialisers too, and finds one store for
each word, in the initialisers: OUT 0x0073d6b8 and 0x0073d6e0, and QUA
0x0063a2cc for the icons. Nothing writes the source words: only the
initialisers load 0x006dc600, 0x006dc5f8, 0x00720c88 and Quarantine's
three.

The port changes:

- **The icons.** [[607]]'s `party_icons_y` is gone. `PARTY_ICONS_Y` is
  320 on every disc, as the port had it before 607.
- **makePacket.** `pair_continues` keeps Infection's bounds on every disc,
  with the initialiser in its comment. Earlier in this work the branch had
  made them per volume (0 on Outbreak and Quarantine, which hid the weapon
  trails and the boxes' rays there). That is reverted.
- **The harnesses.** `volume.overlay_init(m, prog)` runs the overlay's
  initialisers on a harness machine and keeps their writes to the
  overlay's `.data`. The objects they build in `.bss` and main
  (`menuList`, `ccSpcManager`, the damage list) are left as each harness
  sets them up. `test_battle.GameBattle` and `test_fieldui_rs` call it.
- docs/formats/prg.md ("Loading"), docs/engine/battle.md and
  docs/engine/field-ui.md ("The condition icons' static") carry the
  proof.

## The four harnesses

- **test_enemy_weapon_rs.** As above: its machine now has the loaded
  `.data`, and its probe and the port are main's.
- **test_dungeon_rs.**
  - Its "e" rule queries asked the port for the volume's own rule but
    asked `dungeon.py` for Infection's ("alt"). From Mutation on,
    `SetDungeonTypeFromField` has the "plain" rule (MUT 0x001b1a20, OUT
    0x001a77c0, QUA 0x001aefd0; `test_dungeon.py` checks `dungeon.py`
    against each). The query now uses `Data.type_rule`, read from the
    code.
  - It asserted 88 `EditDungeon` rows. From Mutation on, the
    constructor's bound is 90.
  - The last two rows' ROOMDATA tables (areas 118 and 119, MUT gcmn
    0x006c31e0 and 0x006c3c40) have no carried name. piney-gen calls them
    `rooms_<address>`, and `dungeon.py` called them by the bare address.
    `dungeon.py` now uses piney-gen's name.
  - The fog test allowed one bit in `SetAmbient(row / 255)`, because
    `FogRow::ambient` divided in IEEE. The runtime already used the EE's
    `div.s` (`ambient_bits`). Now `FogRow::ambient` does too, and the test
    is exact. Only piney-viewer read the old value.
- **test_sound_rs.**
  - `sound.Program.words` sized tables by their carried symbols, which
    keep Infection's sizes.
    - `bgmParam` is 16 bytes on Infection and 64 from Mutation on (eight
      rows: MUT main 0x0031cce0, OUT 0x00314a10, QUA 0x00208990).
    - `bgmWavTbl` moved to main with eight tracks (MUT 0x0031e840).
    - `wavPlay` (MUT 0x00181750) indexes both by the track.
    - `sound.Program.rows` now sizes a later table as piney-gen's
      `placement::sound::extent` does.
    - `commseTbl`'s padding grew on Outbreak (0x00314af8), so only its
      first three words are read.
  - The voice rows were read by Infection's names (`evVoiceDataVol1M`
    ...), which name other data on the later discs.
    - `sound.event_tables` reads `ccEvVoiceRequest`'s tables from each
      volume's code (INF main 0x0017e810, MUT 0x00181910, OUT 0x00181620,
      QUA 0x001816e0): the lui/addiu bases before each `voiceFile` store.
    - On Infection, every table it finds starts at an `evVoiceData`
      symbol, and every array with rows is a whole VOICE_DATA object.
  - There is a fixture per disc (`sound_fixture.txt`, `_mut`, `_out`,
    `_qua`), and the Rust test checks each disc against its own.
  - New: every message every event script can open runs through the
    game's `ccEvVoiceRequest` and `evVoicePlay` (`tools/evscript.py`), in
    both languages and in Parody Mode.
    - The line they send is the table row: 1,322, 1,640, 2,592 and 3,096
      voiced lines on the four discs.
    - No message is past its event's array. So the rows the tables keep
      past an array's end are never asked for. (On Outbreak, EVVOL3SE's
      event 266 array is one row in .sdata at 0x003862f8; its script has
      one message.)
- **test_staffroll_rs.** It stored `volumeNum` 1 and kept one fixture.
  - Each disc now runs with its own number against its own fixture
    (`staffroll_fixture_mut.txt`, `_out`). The Rust test runs all three,
    and the port matched both later rolls with no change.
  - Quarantine's desktop.prg has no staff roll: no `ccThStaffRoll`
    symbol, and no `stfroll` scene name in the file. The harness checks
    that, and the Rust test checks that the generated tables hold none.
    Quarantine's ending runs its roll in the field, through
    `ending_kanji` (`RollCase`, piney-event).

## The harnesses outside the sweep

The sweep picks up a `tools/test_*_rs.py` that names `PINEY_VOLUME`. Eleven
harnesses did not name it, and the sweep read no `tools/test_*_rt.py`. All
of them already ran on the volume's disc:

| harness | how it follows the volume |
| --- | --- |
| test_battle_event_rs, _flow_rs, _items_rs, test_desktop_data_rs | `import volume` |
| test_battle_navi_rs | `import volume`. It read Mac Anu's dummies from Infection's DATA.BIN; it now uses `volume.DATA` |
| test_battle_frame_rs | test_battle_rs's game. The operation byte is saveData+0x6773 on every disc |
| test_effect_skill_rs | test_effect_rs's game |
| test_fieldui_option_rs, _trade_rs | test_fieldui_rs's game |
| test_lights_rs | test_stream_rs's game and test_world_rs's probe |
| test_field_rt | test_world_rs's game and ISO |
| test_dungeon_rt | `import volume`; it already named the variable |

None of them is specific to one volume by design. Each now names
`PINEY_VOLUME`, and `scratch/sweep/sweep.sh` also takes
`tools/test_*_rt.py`: 89 harnesses, 356 runs. Before the marks, all twelve
passed on all four discs.

## Checks

At cdf8a59 (the code of this entry):

- **The sweep.** 356 runs, none failing. On main's (304 runs):
  - These 10 now pass: `test_dungeon_rs` MUT, OUT and QUA;
    `test_enemy_weapon_rs` OUT and QUA; `test_sound_rs` MUT, OUT and QUA;
    `test_staffroll_rs` MUT and OUT.
  - `test_staffroll_rs` QUA passes too, which makes 11 of main's 11
    failures.
  - Nothing that passed on main fails.
  - The 52 new runs all pass: the 11 newly marked harnesses and
    `test_dungeon_rt`, on four discs each.
  - `test_piros_rs` and `test_stream_rs` are fixture writers with rc 0 and
    no test line, as on main.
- **Battle pins.** 291 calls, 0 differ. The probe is the worktree's.
- **Golden shots.** I shot a base today from 934b942 in its own worktree
  (removed after). It matches `world2/shots/base`: 4,550 files, 0 differ.
  - Against that base, the tip has 22 differences, all `macha_shots` from
    frame 6670 on: the condition icons go back over the panels.
  - Against the base [[607]] compared with (2d2b796, shot today), the tip
    has 0 differences in 4,550 files.
  - The per-volume makePacket bounds this branch first had would have
    changed 25 Outbreak and Quarantine shots: their weapon trails and rays
    went missing. With main's bounds those files are as in the base
    (2,732 files reshot, 0 differ).
- **Tests and tools.** Crate tests (world, data, desktop, battle, effect,
  event, audio, fieldui): 42 ok, 0 failed. The piney-game suite: 349
  passed, 0 failed, 153 ignored. `gen --check`, `syms --check` and
  `carry --check` are current. clippy (all targets and features) has 0
  findings. fmt, `cairns check` and `docs.py check` are clean.
- **Regenerations for main.** None: the tables, syms and carry are
  unchanged.

**Still unknown:**
- `test_lights_rs` compares the light group within 1e-4 for directions and
  1e-5 for colours. Compared exactly, 309 of its 400 random groups differ
  in the last bits of a direction. piney-world's `light_matrix`
  (chara/kite.rs) works in glam's f32, where the game uses EE and VU0
  arithmetic:
  - `ccDrawEnv::SetLightMatrix` (main 0x00105900);
  - `ccOmniLight::CheckRange` (0x00139830);
  - `ccDirectLight::CheckRange` (0x00139330);
  - `ccDistantLight::CheckRange` (0x00139170);
  - `ccSpotLight::CheckRange` (0x001395b0).
- `test_dungeon_rt` compares a keyed rotation's hit matrices within 1e-5
  for rotation and 0.05 for translation. The largest difference is 12,288
  ULP on the later discs and 8,192 on Infection, over 7,961 such hits per
  later disc. `piney_data::anim` interpolates keyed rotations in double
  precision, where the game uses its own single-precision arithmetic
  (`_AnimateForward`). The harness also skips one case on each disc,
  "roomless rows walked into by the door path".
- `test_fieldui_rs` with every write of gcmn's initialisers kept: three
  chat-member tests then differ on every disc.
  - `ccSPC::Initialise` (INF gcmn 0x0059f5f0) puts Kite in slot 0, sets
    the count to 1 and the other slots to -1.
  - With the damage list's root also built, the game's chat member window
    has four rows where the scenario's port has one.
  - The scenario sets `ccPartyManager`, not `ccSpcManager`. So
    `overlay_init` keeps only the `.data` writes, and it is open which
    state of `ccSpcManager` a real party in a town has there.
