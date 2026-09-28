# Event scripts in a format of our own

Status: design only. Nothing here is built yet. Written 2026-09-26.

The story runs on the game's event scripts. Today they reach the port in
two ways:

- **From the disc:** `piney_event::official` reads the `short` arrays and
  the message tables out of the boot executable.
- **In the IR's text form:** `piney_event::text`, which maps one to one
  onto the bytecode.

Both are faithful, and both are hard to work in. This page proposes a
source format for writing, reading and changing scripts. It compiles to the
existing IR, so the interpreter, the hosts and the saves stay as they are.

## What is wrong with what we have

The text form is the bytecode with names on the fields:

```
block
  set phase phase=0 comp=eq
  if status index=8 num=2 comp=eq
  message msg=15
  pc_face pc=0 type=2 code=15 chg=256
```

- **Numbers everywhere.** `pc=15`, `type=2 code=15`, `index=8`,
  `area=15` and `msg=15` each need a table to read. The table is a
  different one each time: `charTbl`, target types, `eventStatus`, story
  areas, the message table.
- **Text is somewhere else.** A block says `message msg=15`, and the line
  itself sits in a `messages` table at the end of the file. You cannot read
  a scene top to bottom.
- **Position is identity.**
  - A block's index is its run bit in the save (`eventFlag[N]` bit b).
    Inserting a block renumbers every later one and breaks every save.
  - A message's index is its voice line.
- **Preconditions are invisible state.** `set` lines stick until the next
  `set`, so a block's real conditions depend on every block above it.
- **Phases are magic numbers.** `phase=0 comp=eq`, `phase=2`,
  `phase=4 comp=ge` stand for "the set-up before the files load", "the
  set-up after" and "play". The set-up passes also behave differently: no
  menu task, windows on the set-up screen. Nothing in the text says so.
- **No checking.** A typo in a number is a valid script. Nothing warns
  about a 63rd block, a window in a set-up pass, or an instruction the
  target volume lacks.
- **Four dialects.** Mutation to Quarantine add and change instructions
  (`noise` level, `grunty_mail`, `ending_kanji`, changed `mode`, `area`,
  `fade`). A script does not say which volume it is for.

## Goals

- Read a scene top to bottom: who says what, what the camera does, and
  when the block runs.
- Names in place of numbers wherever the number has a meaning we know.
  Raw numbers stay allowed wherever we do not.
- Stable identities for everything the save or the voice files key on.
- Checks before run time: names resolve, identities are unique, the
  volume has the instruction, and the phase rules hold.
- **Exact:** importing a disc's scripts and compiling them back gives the
  same IR, script for script and message for message. It is checked the
  way `official` is checked today, on all four volumes.
- Diffs that show intent. A reordered block or an edited line is a small
  diff.

## Non-goals

- **No new semantics.** The interpreter stays Infection's `eventSub`,
  `CheckOpen` and `Execute`, one frame per `Vm::frame`. There are no
  jumps, loops, variables or functions at run time. Anything the source
  offers beyond the IR is sugar that lowers away at compile time.
- **No change to the hosts** (`piney_event::host::Host`) or to the saved
  state (`ccSaveData`).
- **No disc content in the repository.** Imported scripts, their text and
  name tables derived from the disc stay under `work/`, as the IR export
  does today. The committed parts are the compiler, the grammar, a
  clean-room example set and hand-written name tables.

## Invariants the format must keep

1. **Event numbers.** `eventFlag[N]`, `event_done`, `end_event grp` and
   the volume offsets (101, 201, 301 open on `event_done 100·(v−1)`) all
   key on N.
2. **Block bits.** Block b's run bit is bit b (b < 62). Bit 62 is done and
   bit 63 closed. Every block carries a fixed number, written in the
   source or pinned by a lock file.
3. **Message ids.** The voice lookup is `(event, msg)`. A message keeps
   its id, and the id stays when its text changes.
4. **Walk order.** Blocks are tried in number order, and events in
   `eventSub`'s order (0-49 on volume 1 only, 50-99, 150-199, 250-299,
   350-399, then 400-499 behind the gate rule). Source order is for
   people; the compiler emits by number.
5. **Walk levels.** `lv` 0 walks, 1 replays the bookkeeping, 2 plays.
   Which instructions run at 1 (`[flag]`, `[split]`, `[replay]` on
   `docs/engine/events.md`) is fixed per instruction. The source cannot
   change it, and the checker shows it.
6. **Sticky preconditions.** Tags (`SetCurrentOpen`) persist in walk
   order. The source may scope them lexically, but the lowered tags must
   give every block the same effective preconditions.

## The source format

One file per event, `NNN-slug.evs`, UTF-8. Indentation is two spaces;
significant or not is an open question, below. A made-up example in the
style of the story's events:

```
event 900 "TEST 01"  volume=inf
  opens when event_done(899) and game.status == world

  status progress = 8          # names eventStatus[8] inside this file

  in scene holy_ground.church            # area 1 town 0 field 15 block 1

    on setup                               # phase == 0
      block 0 arrival
        stream 8

    on load                                # phase == 2 (set-up screen)
      block 1 the_voice
        info_now #0 "A voice." / "Listen."
        stream 10
        item_add Kite key_item(12) 1
        data_drain

    on play                                # phase >= 4
      block 2 talk_to_rose  repeat
        when talked_to(BlackRose) and progress == 2
        menu_ban
        camera look BlackRose height=10 rot=(5632, 28672) dist=60
        face Kite -> BlackRose chg=64
        say Kite       #1 "Hey..."
        say BlackRose  #2 "Not yet." / "Come back later."
        progress = 3
        camera end
        menu_clear
```

What each piece lowers to:

| Source | IR |
| --- | --- |
| `opens when A and B` | the open conditions |
| `in scene X`, `in town T`, `in field ...`, `in dungeon ...`, `in room ...`, `at point N` | a `scene` / `in_town` / ... tag, emitted where the scope starts and restored where it ends |
| `on setup` / `on load` / `on play` | `phase == 0` / `phase == 2` / `phase >= 4` tags |
| `on status(N) == v`, `on game.status == desktop` | the other tags (settings 2-11) |
| `block N name` | a block with fixed number N; `name` is for people and lints |
| `repeat` | `repeatable` |
| `when ...` | the block's conditions |
| `say WHO #id "..." / "..."` | a `message` op, and message `id` with the name and up to three lines |
| `info`, `info_now`, `ask` | `info`, `info_now`, and `message` with `emode` 3 (a question) |
| `progress = 3`, `progress += 1` | `status_set`, `status_add`, `status_sub` |
| `face`, `walk`, `put`, `turn` | `pc_*` / `npc_*` by what the name resolves to |
| `camera look ...`, `camera path ...` | the `cam_*` and `camz_*` families |
| anything else | the op by its IR name, with named fields, as in the text form |

Rules:

- **Every IR op stays writable as it is.** The sugar is optional. An
  imported script can come out as plain IR lines inside the new structure,
  and be raised by hand one block at a time.
- **Numbers or names.** A field takes a number or a name that resolves to
  the right kind: `pc=15` and `BlackRose` are the same. A name of the wrong
  kind is an error, such as a story area where a character belongs.
- **Messages are inline.** `#id` is optional in a new event, where the
  compiler allocates ids and pins them in the lock file. It is required on
  import, and the checker keeps ids unique.
  - Parody Mode's text goes with the line: `parody "..." / "..."`.
  - `emode`'s high byte, whose meaning is not known, is kept as
    `mode=0x2` on the line.
- **Scopes are lexical.** The compiler tracks the tag state in walk order
  and emits a tag only when the effective precondition changes.
  - Imported scripts restate tags that did not change, and some set tags
    no block uses.
  - To round-trip exactly, the importer marks those with `restate` on the
    scope line. The formatter can drop the marks, which changes the bytes
    but not the behaviour; the equivalence check below proves that.

## Names

Symbol tables are TOML, one per kind: characters, target types, story
areas, towns, fields and dungeons, items by category, mail, news, board
threads and posts, desktop wallpapers, BGM and movies, streams, operations
(0-28), menus, remote actions (`pc_act`), affects, sound commands and
markers per area.

- **Hand-written tables** are committed. They hold what the docs already
  establish: `charTbl` 0-17 by the names the port uses, target types
  2/3/4/5/6/7/20, game statuses, phases and comparisons.
- **Derived tables** are generated from the disc into `work/`. They hold
  item and area names, mail and news titles, and stream names from
  `ccsTbl`.
- **Unknown numbering stays numeric.** `docs/engine/events.md` lists what
  is not understood yet: streams, markers, menus, sound commands,
  operations and action codes. A name for those is added only when the
  meaning is established.
- **`eventStatus` slots** are named per file (`status progress = 8`),
  because each event uses them its own way. A shared table can name the
  slots several events share.

## Identity and the lock file

`events/lock/NNN.toml` pins what the source leaves implicit:

- block numbers for blocks written without one;
- message ids for lines written without one;
- the voice binding of each message id.

It is committed next to the source. The checker fails when:

- a pinned block disappears (its save bit would be reused);
- two blocks share a number;
- a number reaches 62;
- a message id is reused.

Retiring a block is explicit: `retired 7`. The bit stays reserved.

## Dialects

The header's `volume=` picks the instruction set: Infection's 166, plus
Mutation's `noise` level and `grunty_mail`, and Quarantine's
`ending_kanji`. The checker also knows the same-length changes: `mode`,
`area`, `fade`, `fade_more`, `call_lock`'s wider mask, and the save
extension for characters 18-20. It warns when a script depends on
behaviour that differs between its volume and another one it is also
built for. The `volume` condition stays an ordinary condition.

## Checks

Errors:

- a name does not resolve, or resolves to the wrong kind;
- a block number collides, is out of range, or disagrees with the lock;
- the target volume lacks an instruction;
- a comparison is not `==`, `>=` or `<=`, unless marked `raw`: the game
  fails any other, and imports keep them.

Warnings:

- **A window or a menu in a set-up pass.** It shows on the set-up screen,
  with no menu task. That is legal and the game does it (event 11's church
  set-up). The warning says which screen it lands on, because the port
  once drew nothing there.
- A `wait` or a camera move in a set-up pass, where nothing but the
  set-up screen is drawn.
- A block no walk can reach: its sticky preconditions contradict its
  conditions.
- A `repeat` block with no condition that ever turns false (it runs every
  frame).
- Instructions that run at replay (`[flag]`) mixed with play-only ones in
  a block used to fast-forward a later volume.

## Tools

A future crate, `piney-evs`, with a CLI:

- **`evs import`**: disc to IR (`official`), IR to source. It raises the
  scopes from tags, names from tables and messages inline. The output goes
  to `work/`.
- **`evs build`**: source to IR, then IR to the text form or to the
  game's shorts for a round-trip check. Nothing depends on the shorts at
  run time.
- **`evs check`**: the checks above.
- **`evs fmt`**: the canonical layout.
- **`evs diff`**: the IR-level difference between two sources, for
  reviewing a change's real effect.

At run time `piney-event` takes scripts from a source: the disc (today)
or compiled `.evs` files. It can also take an overlay, where compiled
files replace single events and the disc supplies the rest.

## Verification

- **Exact round trip.** Import every script of all four volumes (INF 192,
  MUT 196, OUT 199, QUA 206), build them back, and require IR equality
  with `official`, messages and parody tables included. This mirrors the
  check `official` passes today.
- **Behavioural equivalence** for canonicalised sources: drive the old and
  new IR through `CheckOpen`, `SetCurrentOpen` and `Execute` with
  `LogHost`. Use the walk states `tools/test_event_vm.py` builds, and
  require the same host calls and flag words.
- **Save compatibility.** A save made on the disc's scripts continues on
  compiled ones. The story tests (`--mode story:N`) run on both and must
  give the same block logs.

## Plan

1. This page.
2. The hand-written name tables and the naming rules.
3. Grammar, parser and printer for the source. Lower to IR, and pass the
   round trip on the imported set with every block written as plain IR.
4. The importer's raising: scopes, names, inline messages and `restate`
   marks. Round trip again.
5. The checker and the formatter.
6. The runtime source switch and the save-compatibility runs.
7. The later volumes' dialect checks. Moving saves between volumes
   (`ConvGame`) stays a separate piece of work.

## Open questions

- **Indentation-significant or explicit `end`s?** Write three real events
  both ways, the church, a dungeon set-up and a desktop mail chain, and
  pick the one that diffs and reads better.
- **Are `#id`s visible in the source,** or only in the lock file with a
  generated anchor? Visible ids make voice work obvious and clutter every
  line.
- **Japanese text:** UTF-8 in the source, converted to Shift-JIS by the
  compiler, with an error on anything unmappable. The game's bytes that are
  not valid Shift-JIS keep the `\xNN` escapes of the text form.
- **Should camera paths** (`camz_point` ×N, `camz_speed`, `camz_path`)
  become one `camera path { ... }` construct, or stay a run of lines?
- **Where do translations go?** Per language beside the source
  (`text/<lang>/NNN.toml`, keyed by message id), or inline alternates.
- **How much sugar before it hides the game?** The rule so far: sugar
  only where the lowering is one to one and the checker can show the IR it
  became.
