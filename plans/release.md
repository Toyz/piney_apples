# Release: license and what the repository must not hold

Goal: the repository can be public. The code is ours and dual licensed; nothing
from the discs is in it, now or in its history.

## Done

- MIT OR Apache-2.0: `LICENSE-MIT`, `LICENSE-APACHE`, the workspace's
  `license` field (every crate inherits it), the README's License section with
  the note that the games' own material is not covered and the trademark and
  non-affiliation line.
- Dependencies checked (`cargo metadata`): all permissive; `r-efi` is MIT OR
  Apache-2.0 OR LGPL-2.1-or-later, so MIT/Apache applies.

## The audit (2026-09-27)

Method: every printable run of 16+ characters with two spaces or more from the
four discs (every file; `DATA.BIN` and `STREAM/*.BIN` members inflated; movies
and raw audio skipped): 27159 runs. Every 24-character window at a word start
that reads like a sentence (letters and punctuation, four words or more, no
double spaces) is hashed; a file matches where one of its own windows, or of
its hex runs decoded, is in the set. The scripts are in the session's scratch
(`/mnt/data/claude/scratch/leak/corpus.py`, `scan.py`); they are to become a
repository tool (below).

### The current tree: 207 matches in 69 files

- `crates/piney-stream/tests/subtitle_fixture.txt`: 100. The cut scenes'
  dialogue lines, hex encoded (`change` rows). A leak: to hash.
- Dialogue quoted in docs, worklogs, comments and tests: Bell's greeting,
  Orca's and the members' lines (worklog 123), the fountain's speech
  (`menus/fountain.rs`), Rachel's line (`docs/engine/battle.md`), "I'll check
  my e-mail first." (desktop tests), Mutation's "bad feeling..." (worklog
  228). To remove or replace by a description.
- UI messages ("Cannot be used while dead.", "Select MEMORY CARD slot.",
  "There is no item to trade.") and area-word names ("Bursting Passed Over
  Aqua Field", "Hidden Forbidden Holy Ground"): short phrases, mostly in tests
  that check what the port shows. To keep only where a test needs them, and
  there read them from the disc or compare hashes.

### History: whole data files

Paths that existed and are gone, holding disc data:

- `crates/piney-data/src/events/{inf,mut_,out,qua}.evs`: every event script
  and its messages.
- `crates/piney-data/src/main_data/*.bin`, `*.carry`: the executables' data.
- `crates/piney-data/src/{area,dungeon,field,sound,statics}/{inf,mut_,out,qua}.rs`,
  `sinit.rs`, `crates/piney-desktop/src/tables.rs`: generated tables with the
  values.
- `crates/piney-data/src/tables/*.rs` before worklog 257: the same files as
  now, but with the values in them. So the history cannot be cleaned by path
  alone: older versions of files that still exist hold data.

Not yet done: the text scan over every blob in history (all refs: branches,
stashes, the worktrees' branches).

## The policy

- **Never:** the games' dialogue and writing - anything a character says,
  mail, news and board bodies, story and cut-scene text, in any form (plain,
  hex, in fixtures, as test input). Tests that need text use neutral sample
  lines, or hashes of the game's.
- **Allowed where the work needs them:** names and labels - area words and
  keywords, item, enemy and character names, menu labels, the short system
  messages the menus show (a refusal, a prompt), asset and symbol names.
  These are labels, not writing.

## The tree cleaned (2026-09-28)

- The subtitle fixture's text is hashed (FNV-1a 64); the generator
  (`tools/test_stream_rs.py subtitles`) writes it so.
- Dialogue quoted in docs, code comments, tests and worklogs replaced by
  what it is ("Orca's line for the dead end"). Worklog entries 18, 70, 87,
  92, 123, 134, 143 and 228 were redacted so; the fountain's conversation
  in `menus/fountain.rs` and `docs/engine/field-ui.md` goes by its message
  numbers.
- The harnesses' sample lines (`test_desktop_rs.py`, `test_chat_msg_rs.py`,
  `test_fieldui_rs.py`) are neutral text of the same lengths.
- Rescanned: at 24-character windows 82 matches left, every one a name, a
  system message, a debug string in an IOP module or a coincidence of
  ordinary English; at 16 characters, the same kinds and no dialogue.
  Japanese text in the tree: titles, mail subjects, names, labels and the
  Shift-JIS table.

## To do

1. A repository tool for the audit (Rust, reading the discs through
   `piney-data`): the corpus, the scan of the tree and of every blob in
   history, and a check that fails on a match, for CI.
2. ~~Clean the tree~~ (done, above).
3. The README's "no game data" claim (plans/build-data.md step 4) once 2 holds.
4. ~~History~~ (done 2026-09-28, the safe way): the cleaned tree became a
   new single-root `main`, pushed to `github.com/Toyz/piney_apples`
   (private until the owner makes it public). The old history is kept only
   in the local branch `archive/full-history` and the verified bundles
   (`/mnt/data/claude/backup/`, `~/piney-backups/`); nothing was rewritten
   in place. Push `main` only: the worktrees' branches share the old
   history.