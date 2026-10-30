---
number: 507
title: Quarantine's ending is the game's: the desktop set-up waits out event 314's streams at rate 2, its staff_roll grants wallpapers instead of a roll, and the book's pages bend by their own morphers
date: 2026-10-29
area: script, audio, render, volumes, test
files: crates/piney-game/src/desktop.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-event/src/vm/exec.rs, crates/piney-event/src/host.rs, crates/piney-data/src/scene.rs, crates/piney-world/src/town.rs, crates/piney-world/src/field_area.rs, crates/piney-game/src/session/tests/ending.rs, crates/piney-game/src/session/tests/ending_save.rs, docs/engine/desktop.md, docs/engine/stfroll.md, docs/engine/sound.md, docs/formats/ccs.md
started: 2026-10-29T17:25:04-06:00
took: 46m
---

# 507. Quarantine's ending is the game's: the desktop set-up waits out event 314's streams at rate 2, its staff_roll grants wallpapers instead of a roll, and the book's pages bend by their own morphers

Issue #75 (build 66d17c2, Quarantine) reported four things after Aura is
stabbed: a de-sync as the party wakes up, the player's ALTIMIT theme
playing, a black screen with Infection's outro music after "Orca and I
started on our journey again", and a distorted Book of Ryu in the
credits. They have three causes. `issue_75_replay` replays the
player's pad log to each one.

## The set-up that gave up (de-sync, desktop music)

Event 314's block 2 runs in the desktop set-up's pass at phase 2. It plays
streams 89, 90, 91 and 93, about 12,084 frames at the World's rate 2.
`ccEnableThEvent` breathes until the pass settles, with no limit (QUA
main 0x001c8620, INF 0x001b52c0). `DesktopMode` gave up after 10,000
frames. At the player's log frame 47,031 (stream 93, frame 3,218) it:

- set phase 4;
- set `SetFrameRate(1)`, so stream 93's picture ran at 60 frames a second
  against its 48 kHz PCM: the de-sync;
- built the desktop, which sent `DesktopBgm(20)`. With no `sound 10` hold
  (that comes in block 4), sequence 0 played: the theme.

The area's and the town's stepped set-ups had the same 10,000-frame cap.
All three now wait as the game does. The synchronous passes of
`new_game_events` keep their bound, since a stuck pass there would hang.
In the replay after the fix, the rate stays 2 under all four streams.
No `DesktopBgm` comes until the desktop after field 126.

## Quarantine's staff_roll (the black screen)

The port ran Infection's case 147 on every volume. Each executable has
its own (INF 0x001b1ca4, MUT 0x001c7070, OUT 0x001bd430, QUA
0x001c4a88). All four first set `pauseFlag`, set `dtMenu +0x16` and call
`ccSleepAllThread`:

| volume | after the sleep |
| --- | --- |
| INF, MUT | `ccThStaffRoll`, 30 frames, rate 2, menu 8, `ccSndBgmCtrl`, a frame, wake |
| OUT | as INF, then wallpapers 57-66 with a box each and 30 frames before menu 8 |
| QUA | rate 2, wallpapers 77-86 with a box each, 30 frames, menu 8, a frame, wake: no roll, no `ccSndBgmCtrl` |

Each wallpaper box is `desktop_item`'s own: sound 74, `DispInfo` of
`strcpy(0x00278dec) + dec2sjis(id, 3) + strcat(0x00278de8)` ("Image 77
is added to the Desktop."), then the bit in `dtWallpaperList` (+0x2238).
Parody Mode skips the boxes and the menu.

Quarantine's `g_srDataGrp` is empty, so `StaffRoll::new` had no pages
and `main` returned at once. The roll never set `done`, and `busy`
waited forever over the slept, black desktop, while `ccBgmPlay(1)` (track
1, the first three volumes' outro) played.

- `piney_event` holds the four shapes as `RollCase`.
- `Host::staff_roll_sleep` puts the desktop to sleep; `begin` only starts
  the roll.
- The wallpapers share `desktop_item`'s grant.
- Outbreak's wallpapers were never given before; they are now.
- On Quarantine the desktop stays silent through the boxes and the save.
  The theme starts with block 5's `mode 3`: its `ccSetupDesktop` ends in
  `ccSndBgmCtrl`, with the hold already spent.
- Worklog [[438]]'s line on 314's hold is right: block 4's phase-0 pass.

## The book's pages (the distortion)

`ANM_book_anm2` (QUA `DATA.BIN::ending`) draws `MDL_xdhdpag_a0` for
three index entries (its own and the ExtObj copies `OBJ_xdhdpag_d` and
`_e`), and `MDL_xdhdpag_b0` for two (with `_f`). Each entry's Obj2 chunk
names its own morpher (`MPH_xdhdpag_a0`, `_d`, `_e`, `_f`).
`Decode_ExtObj` (INF 0x0014cb80) keeps the Obj2's words on the copy.
`ccObj::Draw` (0x0013f220) hands `ccModel::Draw` the object's modifier
(+0x98).

The port's `anim_draw_blended` drew by piece. Its map of worlds collapsed
the three entries into one page, and that page took the weights of every
morpher whose base model it was, summed: three page flips in one mesh,
pushed through the cover.

- `piney_data::scene::Scene::modifier` now reads the Obj2 modifiers.
- `piney_world::town::object_morph` picks the entry's own morpher.
- The anim draws (`anm_pieces`) draw every entry at its own place. For
  an anm whose entries are one per piece, the order and places are the
  same as before.
- `field_area`'s animated objects take the entry's morpher too.
- The `morphers` maps the anim draws no longer need are gone.

A survey of every Anime chunk on the four discs (each morpher it weights,
against the entries drawing that morpher's base model) checks the old
pairing. It differs from the game's only in `ending` (Outbreak's and
Quarantine's, 24 entries) and the `field_i`/`field_i1` towers
(`MPH_sfi1tow1_a00`-`a03` on one base, every disc, 28 entries). Every
other weighted entry, 222-257 a disc, names its morpher on its Obj2. So
the body's and the effects' pairing by model draws the same.

## Checks

- **`issue_75_replay`** (ignored; `PINEY_PADLOG`) replays the player's
  log. Before: rate 1 and `DesktopBgm(20)` at frame 47,031 under stream
  93, sequence 0 playing to field 126, and the desktop's roll at page 0,
  status 3 from frame 61,598 to the log's end. After: rate 2 through the
  streams, no sequence, and at 62,000 "Image 77 is added to the Desktop."
  on black, where the player's log, pressing nothing, ends. Shots:
  /mnt/data/claude/scratch/i75/before, book (the book before) and after.
- **`every_ending_plays_over_no_desktop_theme`** adds Quarantine's 314
  and presses through each ending's lines and boxes to menu 8. For each
  of `dtBgm` 15, 47 and 27 it checks:
  - no sequence from the start to menu 8;
  - a roll of more than 7,000 frames (Quarantine's 11,742 in field 126);
  - no desktop before Quarantine's roll, and no desktop roll on it;
  - wallpapers 57-66 (Outbreak) and 77-86 (Quarantine) granted;
  - the theme after menu 8's cancel: 10 frames on the first three
    volumes, 14 on Quarantine, through `mode 3`'s set-up.
- **`book_pages_morph_by_their_own_morphers`** (piney-world) checks
  frame 200 of `ANM_book_anm2`: three entries draw `MDL_xdhdpag_a0`, each
  with its Obj2's own morpher and weights, all three different.
- The piney-game suite, the piney-event, -data, -world, -effect and
  -desktop suites, clippy and fmt pass.

**Still unknown:** nothing
