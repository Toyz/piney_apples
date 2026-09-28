---
number: 262
title: The tree cleaned of the games' dialogue
date: 2026-09-28
area: build, tooling
files: plans/release.md, tools/test_stream_rs.py, crates/piney-stream/tests/subtitle_fixture.txt
---

# 262. The tree cleaned of the games' dialogue

The repository is to be public, under MIT OR Apache-2.0. It must not carry
the games' writing. The policy, in [plans/release.md](../plans/release.md):

- **Never:** dialogue and writing in any form, whether plain, hex, in
  fixtures or as test input.
- **Allowed where the work needs them:** names and labels. That means area
  words, item and character names, menu labels, the menus' short system
  messages, and asset and symbol names.

## How it was found

The corpus is every printable run of 16 characters or more with two spaces
or more, from every file of the four discs. `DATA.BIN` and `STREAM/*.BIN`
members were inflated, and movies and raw audio skipped: 27,159 runs.
Every 24-character window at a word start that reads like a sentence is
hashed. Each tracked file's own windows are looked up in that set, and so
are the windows of its hex runs decoded.

The first pass found 207 matches in 69 files. A second pass at 16-character
windows found the short quotes the first missed. A sweep for Japanese
characters covered what an ASCII corpus cannot see.

## What was changed

- **The subtitle fixture** (`crates/piney-stream/tests/subtitle_fixture.txt`)
  held about 100 cut-scene lines, hex encoded in its `change` rows. The
  Rust test never read those rows, since it compares only the draws'
  hashes. The rows now carry each string's FNV-1a 64 (401 strings), and the
  generator writes them that way.
- **Quotes in prose.** Dialogue quoted in docs, code comments and worklogs
  became a description of what the line is. Worklog entries 18, 70, 87, 92,
  123, 134, 143 and 228 were redacted this way. It is the only edit made to
  past entries, and this entry records it. The fountain's conversation
  (`menus/fountain.rs`, `docs/engine/field-ui.md`) now goes by message
  numbers.
- **Sample text in the harnesses.** The harnesses that feed text to the
  game's code and to the port alike (`test_desktop_rs.py`,
  `test_chat_msg_rs.py`, `test_fieldui_rs.py`) now use neutral lines of the
  same lengths. `test_evscript.py` checks that a line is inlined with its
  voice without spelling the line out.

## A finding on the way

The chat-balloon harness failed on a 74-byte sample line. The game's
`ccChatMsg::OpenChat` (INF main 0x001a67c0) copies the text with a plain
`strcpy` into `data[i]`, which is 71 bytes. It then measures the source
with `ccKanjiStrWidth`, not the copy. A longer line overruns into the next
balloon's buffer. The port cuts the text at 70 bytes instead, so the two
differ only past the game's own buffer. The game's lines never get there:
the longest sample of the old set was 68 bytes. The new samples keep to
70.

## Checks

- Rescan at 24-character windows: 82 matches left. Each is a name, a
  system message, a debug string in an IOP module (`playEnv running status
  error`), or ordinary English that happens to match ("in the fields and
  dungeons").
- Rescan at 16 characters: the same kinds, and no dialogue.
- The harnesses pass: `test_desktop_rs` (message window),
  `test_chat_msg_rs`, `test_fieldui_rs` (field message) and
  `test_evscript`. So do the Rust tests over the fixture:
  `the_subtitles_are_the_games`, the desktop's `event_message_freezes_the_desktop`
  and piney-audio's voice tests.

**Still unknown:**
- Short dialogue under four words is not caught by the scan.
- Text in pictures (the games' textures) is not text to it.
- The history's blobs have not been scanned. The history holds whole data
  files, so it will not be published as it is (plans/release.md).
