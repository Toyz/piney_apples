---
number: 226
title: Mutation's saves, its voices, and the recall's black fades
date: 2026-09-27
area: volumes
files: crates/piney-desktop/src/card.rs, crates/piney-desktop/src/savesys.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/syms.rs, crates/piney-data/src/tables/voice.rs, crates/piney-audio/src/driver.rs, crates/piney-data/src/sound/voice.rs, crates/piney-stream/src/draw.rs, crates/piney-stream/src/scene.rs, crates/piney-desktop/src/layers.rs, tools/sound_tables.py, tools/test_desktop_data_rs.py, tools/test_desktop_savemenu_rs.py
---

# 226. Mutation's saves, its voices, and the recall's black fades

This entry covers three pieces of Mutation work: the memory card, which
the port knew only in Infection's form; the event voices, which Mutation
never played; and the new game's recall, whose pages go black between
topics (reported from play).

## The memory card by volume

- **Save directories.** `mcDirName` holds the four volumes' save
  directories, indexed by `volumeNum - 1`. Infection's executable names
  the later three with placeholders. The table is now generated
  (`title::mc_dir_names`). `FilesCard` and the probes' cards use the
  disc's own directory, `BASLUS-20562DOTHACK` on Mutation.
- **Slot files.** From Mutation on, a slot file is the record followed by
  the extension, 0x8d84 bytes (`savesys::slot_size`).
  - `MainProccess` copies the extension member by member, as it does the
    record. The copy leaves two padding runs, which a load keeps:
    ext+0x49b and ext+0x751..0x753 (`EXT_KEPT`).
  - The index's sum covers the whole file.
  - `DataRead` takes the volume whose directory it reads.
- **The previous volume's saves.** The title reads these from directory
  `volumeNum - 1` at 0x8530 bytes. The card supports that read, but the
  title's import of that data (the rest of the 4096 bytes Mutation's
  `MainProccess` grew) is not ported yet.
- **The harnesses.**
  - They take the directory and size from the executable
    (`volume.card_dir`, `SLOT_SIZE`, `number`).
  - They place the extension beside the record, with its `$gp` pointer
    set (`test_save_init_rs.place_save`).
  - The clear-data scenarios use the disc's volume number.
  - The save menus' machine has a controller in port 1. Mutation's
    `SaveMenu` keeps its input hold after a save only while the game is
    paused or `ccSys.pad[0].status` is set.

The Data screen and save menu harnesses pass on Mutation and Infection.

## The event voices

Mutation's events are 100-199. The port's `ccEvVoiceRequest` knew only
Infection's 0-99, and the voice tables for the later volumes were never
generated, so every Mutation event was silent. The tables are now
piney-gen's `voice` group (`tables/voice.rs`), and the Python generator
no longer writes them.

- **`files`, `files_e`.** These are `evVoiceFile(E)`. The file list is
  the same on every disc. Mutation adds `BOSSTALK.BIN` (19).
- **`events`.** Each disc's own `ccEvVoiceRequest` is decoded as the code
  stands, since the discs differ:
  - The function groups by `event / 100`. Each `lui`/`addiu` table base
    before a `voiceFile` store belongs to that store's file, Japanese
    first.
  - The file names the hundred and the kind: 6 + 3 (v - 1) main, + 1
    side, + 2 parody. A base is its table less 4 times the first event.
  - A row is a line (whole-sector offset, positive size) or none
    (-1, -1). An array ends at the first row that is neither, so padding
    and placeholder words drop out.
  - Infection's volume 2 has main, side and parody tables, but no
    English. Mutation's has main and side in both languages; a main event
    in Parody Mode has no voice there. Infection's tables come out as the
    old ones did, row for row.
- **The driver** (`voice_request`) dispatches by section:
  - The main events take the parody table in Parody Mode, or no voice
    where there is none.
  - The side events take the side table.
  - English applies when the code has an English table.
  - `voiceFile` is set whenever the branch runs.
- **Checks.**
  - Every Mutation event line lies in its file on Mutation's disc.
  - A Mutation line plays headless in both languages.
  - On Infection, requests for volume 2's side events now match the game.
    The game names a file that disc lacks. Where the game reads past a
    placeholder table, the port sends nothing.

The field voices (`ccVoiceRequest`'s groups), the skill words and the
food voices are still Infection's tables only.

## The recall's black fades

Mutation's new game plays stream 23 (`str0695E`) after the name entry:
Helba's recall of Infection. Its 27 pages are planes in front of the
camera.

- The pages cross-fade through a black board (`MDL_blackbord_a01`) 1 unit
  in front of them, and through a white one for flashes.
- The port drew the board first, and the page behind it failed the
  depth test, so each fade went black: the page vanished at once and came
  back as a cut.
- Both are translucent models without a bounding box. `ccModel::Draw`
  keys such a model `M[2][3] / M[3][3]` of `world_screen * lw`, and
  `ccDLSort` draws ascending keys. For planes square to the view, the key
  is the camera's tilt over the distance.
- The port built the stream camera in f32, so the tilt was noise.

The camera is now VU0-exact: `Scene::world_view_bits` is
`SetMatrix_PosRotXYZDebug` with the stream's unit matrix (+0xc0). It is
checked bit for bit against the game's own function in eemu. The key is
taken from it (`exact_key`), and sorted keys now treat -0 as 0. The camera
is pitched 2 ulps short of 90 degrees, so the tilt is real, and the key
still orders the board first. By the game's own arithmetic the board
draws first, and the Z buffer is 32 bits, so nothing saturates.

**Still unknown:** Whether the PS2 shows these fades black too. Every
step checked (camera, key, sort, depth test, clear) matches the game,
yet the page layout suggests smooth fades were meant. The field, skill
and food voices of Mutation, and its `BOSSTALK.BIN`, are not generated.
The title's import of the previous volume's saves is not ported.
