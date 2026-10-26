---
number: 498
title: The console's Tab completes arguments too: a command's on/off and stop words from its help line, and paths for PATH and FILE
date: 2026-10-25
area: tooling
files: crates/piney-game/src/console.rs, BUGS.md
started: 2026-10-25T23:19:55-06:00
took: 9m
---

# 498. The console's Tab completes arguments too: a command's on/off and stop words from its help line, and paths for PATH and FILE

BUGS.md: "the console's (F1) Tab is broken" (user report, 2026-10-01).
Which case the report meant was never told, so every path Tab can take
is now covered.

**What it did.** `Console::complete` completed only a line's first word,
from the first words of the `help` lines. Once the line held a space it
returned at once. So no argument completed: not `import_card`'s path,
`pad_log`'s file or `stop`, nor the `on`/`off` of `deflicker` and `vsync`.

**What it does now.** `set_commands` keeps each help line's usage (the
part before the two spaces): the command and its arguments as written.
Tab completes the word that ends the line:
- **the first word:** from the commands, as before;
- **a later word, by its position:** from that command's argument in its
  usage line:
  - the lower-case alternatives of `[on|off]`, `[FILE|stop]`;
  - for `PATH` or `FILE`, the files and folders on the disk, from the
    folder before the word's last `/` (the current one without);
  - placeholders such as `N` or `ID` offer nothing.
- **with one candidate:** the word is finished with a space, but a folder
  keeps its `/` so Tab can go on into it.
- **with several:** the word is completed as far as they agree; if that
  adds nothing, they are listed.
- **a word past the usage:** nothing.

The help lines are the window's (APP_HELP) and the mode's (the session's
list), as before.

**Checked.** New `tab_completes_the_arguments`:
- `deflicker of` becomes `deflicker off `;
- `vsync o` lists `off  on`;
- `pad_log st` becomes `pad_log stop `;
- `import_card sr` becomes `src/`, and `src/conso` becomes
  `src/console.rs `;
- a number or a word past the usage is left alone.

`tab_completes_the_commands` still passes, as does piney-game's suite
(336).

Docs: BUGS.md.

**Still unknown:** nothing
