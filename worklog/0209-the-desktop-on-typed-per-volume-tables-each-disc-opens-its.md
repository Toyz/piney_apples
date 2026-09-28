---
number: 209
title: The desktop on typed per-volume tables; each disc opens its own story
date: 2026-09-26
area: volumes
files: tools/static.py, tools/static_manifest.py, crates/piney-data/src/tables, crates/piney-desktop/src, crates/piney-demo/src/dialog.rs, crates/piney-fieldui/src/chat_msg.rs, crates/piney-game/src/desktop.rs, crates/piney-event/src/vm/mod.rs, crates/piney-event/src/host.rs, plans/volumes.md
---

# 209. The desktop on typed per-volume tables; each disc opens its own story

The second system rebuilt after 0207. The desktop now reads only
generated tables. Mutation's New Game, which showed an empty window over
a black setup, now opens Mutation's own story.

## The desktop's tables

Five new groups join the title's (0208):

| group | what |
| --- | --- |
| `desktop` | walls, music, movies, news; the mailer's and Audio screen's words |
| `mail` | the mails, replies and their links, normal and parody |
| `kanji` | the glyph trims (with the three halfwords past each), `ccSpriteColorTable`, `alphaBlendTbl` |
| `staffroll` | `g_srDataGrp`, `SR_CCS_NAME`, the eight `SR_*_TIME`s (Quarantine's desktop has none) |
| `nameentry` | the entry's texts, the 21 refused names, `HiraBlock` and `EigoBlock` |

These readers moved onto the tables:

- the Data screen, the mailer, the Audio screen;
- the staff roll (`staffroll::Tables` is the generated struct);
- name entry: `NameEntry::of(volume, ..)`, no program image;
- the colour table everywhere it was used.

The desktop's own `tables.rs` and `tools/desktop_tables.py` are deleted.
The grids are rows of text (`char[][14]`). A row past a table reads as
NULs. The game reads the tables' neighbours there, but the cursor never
reaches one with anything but NULs to show.

The Data screen draws the title's words: savedata.cpp's own `STR_`
pointers hold the same text on all four discs (checked in the generator's
output). The generator gives no `write` to a row of only pointers.

## Text width moved to the renderer

`str_width` needs the volume's glyph trims, so it takes the `Fonts`. The
message window no longer measures:

- an information line is drawn with `centred`, and the renderer places it
  at `256 - width / 2` (`message::text_dx`);
- the probes print the same position, so the harness sees what it saw
  before;
- chat balloons measure with the field UI's fonts;
- the load screen measures with the volume's glyphs.

## The volume, not 1

`volumeNum` was the constant 1 in five places. It is now the disc's:

- `ccSaveSys` takes the volume, which picks the "no data" and "clear data"
  messages and decides which clear data counts;
- the Audio screen's lock line and the news page's animation follow the
  volume;
- the save menu's clear colour follows it too.

## Each disc's New Game

Event numbers are one tree across the four discs:

- the main stories: M1 0-49, M2 100-149, M3 200-249, M4 300-349;
- the side stories: S1 50-99, S2 150-199, S3 250-299, S4 350-399;
- the mail: ML 400-499.

The port ran Infection's boot on every disc, which went wrong in three
ways:

- **The boot.** `ccThMother` calls `ccStartEvent(volumeNum, 0)`: 1, 2, 3
  or 4 (read at each disc's call site). The port always passed 1, so on
  Mutation event 0 counted as done and Infection's leftover opening (event
  1) ran. Mutation's copy of that event has no messages; its `evMsgTbl`
  group 0 is all zeros and nothing writes it. That was the empty window.
  `boot` now passes the disc's volume, which marks event 100 done on
  Mutation.
- **The pass order.** Each disc's `ccThEvent` walks its own main story
  just before the side story of the same number: Infection M1 then S1-S4,
  Mutation S1 M2 S2-S4, Outbreak S1 S2 M3 S3 S4, Quarantine S1-S3 M4 S4,
  then ML. The port walked Infection's order and never reached 100-149.
  The later volumes also stop the story stages once the phase goes
  negative; the ML stage does not check. `story_stages` holds the order,
  with a test of all four.
- **`volumeNum` in the scripts.** The event host's `volume()` defaulted to
  1 and no host answered it. The volume is a property of the disc's
  scripts, so `Library` carries it and the VM reads it from there.
  `Host::volume` is gone.

On Mutation the setup's first pass now opens M201: name entry, then
streams 23 and 24, the first town and the first mails
(`mutation_s_setup_opens_its_own_story`).

## Checks

The Infection suites that compare with the game (the event VM, the desktop,
menu, save menu, Data, name entry, demo and chat harnesses) all pass.
Against the previous commit, piney-game's failures went from 106 to 99,
with none new. Seven desktop paths pass again:

- power-on to the desktop;
- load from the card;
- the title's option;
- the ending's save;
- Black Rose's mail;
- the new-game events;
- the title reset.

The 99 left are the field UI, top page, streams and story announcements,
which still read the executable's data until they are rebuilt.

**Still unknown:** Mutation's `ccThEvent` also ends a pass on a second
flag beside `compulsionGameOver`: main 0x0038bd44, which has no name. The
port has no host answer for it. The card still uses Infection's directory
and slot size on every disc. Streams fail on the later discs (and on
Infection) until the stream tables are rebuilt, so Mutation's opening
movies are skipped. The field UI's record menu still reads `saveSysMsg`
through `piney_data::sinit`, and the story announcements read
`getItemMenuStr` from the image. Both go with the field UI's rebuild.
