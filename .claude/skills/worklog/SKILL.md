---
name: worklog
description: Write an entry in the project worklog - one file per entry under worklog/, indexed by WORKLOG.md. Use whenever a unit of work finishes - a problem diagnosed, a subsystem built and verified, a decision made, a dead end ruled out, a measurement taken. Also use when the user says "worklog", "log this", "write it up", "/worklog".
---

# Worklog

The durable record of piney_apples. Code says what; the worklog says *how we
found out* and *why it is that way*. It is the primary artifact that survives
context compaction - write it for a reader who was not here, including yourself
in a later session.

## Where it lives

One entry per file:

```
worklog/0003-a-short-title.md    the entries, NNNN-slug.md
WORKLOG.md                            generated index - never edit by hand
```

One file per entry means the next number is a filename lookup rather than a read
of the whole log, an entry can be found by grepping front matter instead of
scrolling, and two entries written in the same session do not collide in a diff.

## How long it took

The `clock` skill starts a clock when work begins (`cairns start`), and
`cairns new` records how long it ran as the entry's `took:`. Do not estimate
the time yourself. If no clock was running, the entry simply records none; if
the work spanned a break, `cairns new --took 45m` gives the time spent instead.

## Writing one

Write the whole entry in one command, with the prose on stdin:

```sh
cairns new "Short title in plain words" --area disc,format --files "a.rs,b.rs" \
  --unknown "what remains open, or nothing" --body - <<'EOF'
The finding first, then the evidence.

## A sub-heading if it needs one
EOF
```

That numbers and dates the entry, writes the front matter and the `# N. Title`
heading, appends the `**Still unknown:**` line from `--unknown`, and refreshes
the index. There is no file to open and edit afterwards.

- `--body -` reads the prose from stdin. Quote the heredoc marker (`<<'EOF'`)
  so backticks and `$` in the prose arrive as written.
- The body is the prose only. Do not start it with the `# N. Title` heading -
  the command writes that, and drops a leading one if you do.
- Say what is still unknown **once**: either `--unknown "..."`, or a
  `**Still unknown:**` line at the end of the body - not both, and not neither.
  `--unknown nothing` closes the entry out. The command refuses the other two
  cases and says which it was, before anything is written.

Without `--body`, `cairns new` writes a stub with the front matter and heading,
to be filled in by editing the file.

An entry may be filed under **several areas at once** - comma separated, as
above, or by repeating `--area`. Where a piece of work genuinely sits in two,
say so; it is truer than picking whichever it was mostly.

`--area` is one or more of:

| area | what belongs there |
| --- | --- |
| `disc` | the DVD image, its file system, SYSTEM.CNF and boot |
| `format` | a container or record layout decoded |
| `decomp` | facts pulled out of the EE executable or its overlays |
| `iop` | IOP modules, the sound driver, CD streaming |
| `engine` | game loop, task system, memory, overlay loading |
| `render` | GS and VU pipeline, models, textures, animation |
| `world` | fields, dungeons, Root Towns, the area-word system |
| `battle` | combat, skills, stats, items, monsters |
| `script` | events, dialogue, the in-game desktop: mail, news, boards |
| `audio` | music, sound effects, voice |
| `video` | PSS movies and in-engine streams |
| `ui` | menus, HUD, fonts, text encoding |
| `save` | memory card data and what carries between volumes |
| `content` | what is on the disc, inventories, counts |
| `volumes` | how Infection, Mutation, Outbreak and Quarantine differ |
| `tooling` | the extractors, disassemblers and viewers under tools/ |
| `build` | repository layout and how things are run |
| `test` | harnesses and fixtures |

Other commands:

```sh
cairns next                 # the number the next entry would take
cairns index                # regenerate the index
cairns open                 # every unresolved question in the log
cairns check                # numbering sound, front matter complete, index current
cairns ls :area and open    # the entries a query matches - has word, took gt 2h, date ge ...
cairns refs 12              # where entry 12 is mentioned, in the log and in the code
cairns update               # after upgrading cairns: refresh these skills and the index
```

Read what the log already knows before starting on something: `cairns ls has
<word>` and `cairns open` find the entries and questions that touch it.

**Two entries with one number** - after merging a branch that also wrote the
next entry - `check` names them, and `cairns renumber <path>` moves one; it
lists the other places that named the number, to check by hand.

**A screenshot or capture** goes with the entry: `cairns new ... --attach
shot.png`, linking it in the body as `![what it shows](shot.png)`.

**In code**, a comment that says `see worklog N` is found and shown on entry
N's page, linking back to the line.

Run `check` before finishing. It catches a stale index, a file whose name no
longer matches its title, a missing date, and an area nobody has heard of.
`cairns check --fix` regenerates a stale index - after editing `cairns.toml`,
say - and still reports everything else.

## When to write an entry

When a unit of work concludes:

- a problem diagnosed, with the root cause - not just the symptom
- a subsystem built and verified, with what verified it
- a decision made, with the alternatives that were rejected and why
- a hypothesis tested and **disproven** - dead ends are the most valuable
  entries, because they stop the next session re-walking them
- a measurement taken - a benchmark, a size, a count - because the number is
  the thing that is hard to get again
- a claim in an earlier entry overturned

Do not write an entry for trivial edits, formatting, or anything the diff
already explains on its own.

## Entry format

The file starts with front matter the tool wrote. Do not renumber it, and do not
edit the index to match - run `cairns index`.

```markdown
---
number: 12
title: A short title in plain words
date: 2026-09-20
area: disc, format
files: src/thing.rs
supersedes: 6
---

# 12. A short title in plain words

<What was done and what was learned, in prose. Lead with the finding, not the
process. Include the concrete evidence: numbers, offsets, names, sample values,
measured timings. Show the table or the struct in a fenced block when there is
one.>

**Still unknown:** <what remains open, or "nothing" if closed out.>
```

Sub-headings inside an entry use `##` - the entry's own title is the `#`.

`**Still unknown:**` must begin a line, and it is what `cairns open` collects
across the whole log. It is the log's list of what the project does not yet
know, so it is worth writing honestly rather than leaving blank.

`nothing.` closes the entry - a note may follow the full stop:
`**Still unknown:** nothing. The test runs on Mutation only.` Do not write
`nothing about X` or `nothing new`: it reads as closed and may not be, and
`cairns new` refuses it. Either nothing is open, or say what is.

When more than one thing is open, make it a list - each item one question,
specific enough that a later entry could answer it:

```markdown
**Still unknown:**
- whether the second table is ever read
- what [[12]] left about the header's last word
```

## Pointing at another entry

Write `[[12]]`. It becomes a link to entry 12 carrying that entry's title, and
`check` fails if entry 12 does not exist. `[[12|in other words]]` supplies your
own wording.

Use it freely in prose - "as [[12]] found", "this contradicts [[6]]". A log
whose entries do not point at each other is a pile of entries.

Inside code, fenced or inline, `[[...]]` is left exactly as written.

An entry in another worklog named under `[workspace]` in `cairns.toml` is
`[[name:12]]`.

## Linking an entry to an earlier one

The log is append-only. Never rewrite or renumber an entry. Two front matter
fields carry everything that would otherwise tempt you to edit one, and both
take one or more entry numbers:

```sh
cairns new "What changed" --area disc --supersedes 6
cairns new "What it turned out to be" --area disc --resolves 6
```

**`supersedes`** - this entry overturns something an earlier one claimed. The
reader who lands on entry 6 is then told that 12 corrected it. This is the
single most valuable edge in the log, and it only exists if you record it.

**`resolves`** - this entry answers the question an earlier one left open. That
question then leaves the open-questions list, and entry 6 keeps it, struck
through, naming what closed it.

They are not interchangeable. Answering a question does not mean the entry that
asked it was wrong, and `check` will reject a `resolves` aimed at an entry that
left no question open.

**One question of a list.** A trailer written as a list is numbered on its
page. `--resolves 6.2` answers only the second; the rest stay open.

**`carries`** - this entry takes questions over without answering them: a
triage pass gathering what is open into its own list. Use it, never
`resolves`, for that - `resolves` tells every reader of the old entries their
questions were answered.

```sh
cairns new "Open questions after the triage" --area disc --carries 6,9.1,9.3 ...
```

## Pointing at code

`[[src/thing.rs#parse_header]]` links to a definition by name - prefer it to
line numbers, which move. `[[src/thing.rs:120-158]]` links to lines;
`@3fbdc65` after either pins it to a commit; `![[...]]` embeds the code in the
entry instead of linking it. `files:` takes the same forms.

## The reference

This project also keeps reference pages under `docs/` - what is true now, for
someone who wants to use it rather than read how it was found. When an entry
establishes something a reader would look up - a layout, a table, a rule -
write or correct the page too, as this project's skill for its reference
pages says, and cite the entry on it:

```sh
cairns doc cite formats/the-archive 31
```

The entry is the evidence and the page is the result. Neither replaces the
other: an entry is never edited, a page always is.

## Rules

- Prose, not bullet soup. Bullets for genuine lists only - field tables,
  enumerated options.
- No emojis anywhere in the log.
- Absolute facts over impressions. If something is a guess, label it a guess and
  say what evidence would confirm it.
- Keep numbers, names and paths exact. A wrong constant in the log is worse than
  no log.
- Record the *negative* results: the thing that turned out not to be what it
  looked like, the approach that failed, the code that turned out to be dead.
- If the entry records a behaviour, there should be a test that holds it. Say
  which test, by name, so the claim and its proof are linked.

<!-- cairns:project -->

## This project

The worklog is the narrative; `docs/` is the reference - see the `docs` skill.
A format entry says how the layout was worked out and what is still guessed;
the matching page under `docs/formats/` states the layout flatly, for someone
who only wants to write a parser. **A format entry is not finished until
`docs/formats/<name>.md` exists and the entry links to it.**

There are four games. Say which one every claim is about, with its tag:

| tag | game | boot ELF |
| --- | --- | --- |
| `INF` | .hack//Infection (USA) | `SLUS_202.67` |
| `MUT` | .hack//Mutation (USA) | `SLUS_205.62` (stripped) |
| `OUT` | .hack//Outbreak (USA) | `SLUS_205.63` (stripped) |
| `QUA` | .hack//Quarantine (USA) | `SLUS_205.64` (stripped) |

Only Infection's executable has symbols. `tools/xfer.py` carries them to the
other three as a `.syms` sidecar next to each ELF under `work/`, which every
tool loads; a name from there is a match, not a fact, so cite the VA as well
(see `docs/disc/volumes.md` for how often it is wrong).

A claim checked on one volume is a claim about that volume only, until someone
checks the others. Say so, and say it under the `volumes` area when they are
compared.

Every claim about the original game carries a locator, so it can be
re-checked:

- a virtual address in the EE executable, as `INF SLUS_202.67:0x00123456`, or
  in an overlay, as `INF GCMN.PRG:0x00401234` - overlays load at a fixed
  address, so give the VA, not the file offset
- a disc path, as `INF DATA/DATA.BIN`, and a member of an archive after `::`,
  as `INF DATA/DATA.BIN::framebuf.cmp`
- a sector on the disc, as `INF LBA 12463`, when the game addresses data by
  sector rather than by name
- a byte offset into a decoded record, as `+0x54`

Keep offsets, sizes, opcodes and paths exact. A wrong constant in the log is
worse than no log.

There were no reverse engineering tools on this machine when the project
started, so the tools are written here, under `tools/`, in Python with no
dependencies outside the standard library. The ones an entry is likely to cite:
`tools/iso.py` (the DVD image), `tools/elf.py` (the EE executable: sections,
symbols, relocations, reads by VA), `tools/disasm.py` (disassembly with exact
xrefs; `--overlay` maps a .PRG), `tools/mips.py` (the R5900 decoder),
`tools/image.py` (main plus one overlay), `tools/fdtbl.py` (the DATA.BIN
index), `tools/gzarc.py` (`DATA.BIN` and `STREAM/*.BIN`),
`tools/ccs.py` (CCSF scene files), `tools/ccstex.py` (textures to PNG), `tools/ccsmodel.py`
(models to OBJ),
`tools/dwarf1.py` (types, signatures, locals, lines from the DWARF),
`tools/demangle.py` (C++ names), `tools/eemu.py` (run static initialisers),
`tools/text.py` (mail, board, news), `tools/areas.py` (keywords and the
area generator), `tools/tables.py` (any table through its DWARF type), `tools/sound.py`
(all audio; with `scei.py`, `adpcm.py`, `wav.py`), `tools/vu.py` and `tools/vif.py`
(microcode, VIF packets), `tools/evscript.py` (event scripts), `tools/dungeon.py` and `tools/field.py`
(area generation), `tools/battle.py` (the battle rules), `tools/font.py`
(text rendering), `tools/save.py` (saves and carry-over, run in eemu),
`tools/anim.py` (animation playback), `tools/portdata.py` (the port's
data files in `work/data` read back),
`tools/xfer.py` (Infection's names carried to the
stripped volumes), `tools/docs.py` (the reference pages' index).
When an entry adds a tool, it adds it to this list.

The disc images live in `originals/` as the 7z archives they came in, and are
extracted under `work/` - neither is ever committed.

A port, when it starts, is written in Rust as a Cargo workspace under `crates/`
- never another language. The analysis tools stay Python under `tools/`.
