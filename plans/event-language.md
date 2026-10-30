# The event language

A language for the games' event scripts that people can read and write,
so the story can be studied, changed and extended: new events, new side
quests, new areas once defs and systems land. The maintainer's plan
(2026-10-30). It comes after the Rust clean-up; it touches `piney-event`,
which is stable and fully checked.

## Where it starts

`piney-event` already has the whole pipeline, one level down:

- **IR** (`ir.rs`): an event is a number, a label, a `Script { open, blocks }`
  and two message tables (normal and Parody Mode). A block is `tags`
  (precondition settings: `set phase >= 4`, `set in_field ..`), `conds`
  (`if talked_to ..`, `if operate ..`) and `ops`. 169 op kinds, 40 condition
  kinds, 10 tag kinds; some exist only on later volumes (`volume`, the
  `noise` ops, `ending_kanji`).
- **Text form** (`text.rs`): `print_event` / `parse_event` round-trip
  exactly. It is one instruction a line with raw arguments
  (`entry_mc type=0 code=130 marker=0 param=1`), messages by index, and
  positions as numbers: an assembly language, faithful and unreadable.
- **Official scripts** (`official/`): every disc's events decoded and
  re-encoded byte for byte, in the build as `PINEY/EVENTS.EVS`.
- **VM** (`vm/`): checked against the game's own `eventSub`,
  `ccEventFlagSet`, `CheckOpen` and `Execute`.

So the new language sits on top: it compiles to the same IR, and the IR to
the same bytes. Nothing below it changes.

## The layers

```
event language (.evl)   what people write: names, structure, inline text
        |  compile / lift
text form (.evt)        the assembly level, kept for exact control and debugging
        |  parse / print
IR                      Script, Block, Op, Cond, Tag, Message
        |  encode / decode
bytecode                what the game's VM runs, in the build
```

## The one guarantee

Every event on all four discs (and fragment's, later), with both message
tables, lifts to the language and compiles back to **byte-identical**
bytecode. That test runs in the suite, over every script, on every volume.
It is what makes the language trustworthy: it can express everything the
games do, and it adds nothing the games did not have. Every lowering rule
and every piece of sugar must keep it.

## The language

The syntax is our own design (clean-room, like the rest of the port):
braces, a rich language, implicit block numbers (the maintainer's
decisions, 2026-10-30; see Decisions):

```
event 3 "TEACH-F" {
    open { done event 2; game status 5 }

    when in field 14, phase == 0, repeatable {
        operate add 14
        target Orca
    }

    when phase >= 4, talked to Orca, repeatable {
        menu off
        Orca: "Whenever you want me to do / some..."
        menu clear
    }

    once {
        menu off
        camera at (2474, 2457, 11) looking (2497, 2413, 21)
        turn Kite 0
        turn Orca 0
        wait 120
        Orca: "Before we get going, let me / ex..."
    }
}
```

What it gives over the text form:

- **Structure the game implies.** An event's `open` conditions, its
  blocks, each block's tags and conditions as one `when` clause, and
  `repeatable` as a keyword, not an instruction at the end of the body.
- **Implicit block numbers.** Blocks are numbered by their order, as the
  game numbers them; the source writes none. Where something refers to a
  block (the status ops, the replay's per-block flags), the block gets a
  name (`block intro { .. }`) and references use it; the compiler numbers
  and checks them. The lifter names a block only where something refers to
  it, and keeps empty blocks the bytes need as `empty` placeholders.
- **Names, not numbers,** resolved from the game's own tables and the
  build: characters (`Kite`, `Orca`, the PCs by name), event numbers and
  labels, areas (by their words: `Δ Bursting Polluted Sea of Sand`),
  fields, towns, markers and dummies (`DMY_marker01` from the area's own
  file, or a position), sounds and voices, BGM rows, streams, mails, items,
  key items, wallpapers, flags (`eventStatus[52]`, given names where we
  know them). Numbers stay allowed everywhere a name is; the lifter uses a
  name only where it is unambiguous.
- **Inline text.** `Speaker: "line / line"` writes a message where it is
  said; the compiler builds the message table. Questions
  (`ask Orca: "..." { yes { .. } no { .. } }` lowering to the question
  message and its branch ops), Parody Mode text beside the normal text
  (`parody: "..."`), the game's escapes (`#0` the speaker's name, `#1` the
  player's, colours, extended font codes) as named escapes. The text is
  written in Unicode and encoded to the game's ASCII plus Shift-JIS; a
  character the font cannot draw is a compile error.
- **Typed arguments.** Every op's arguments have types (character, area,
  sound, count, position, angle, flag, comparison): range-checked, named
  enums where the game has them (camera kinds, pc modes, fade kinds).
  Errors give the line, the column and what was expected.
- **Built-ins for the common shapes,** each lowering to the exact op
  sequence the games use: fades, a camera move, a party call, entering a
  character at a marker, a gate-out, giving an item with its window. Sugar
  only where the lifter can recognise it and lowering gives the same bytes;
  otherwise the plain op stays.
- **Per-volume checks.** An op or condition that a volume does not have is
  an error when compiling for that volume (`event ... for Quarantine`).
- **An escape hatch.** Any IR op can be written raw (`op entry_mc(0, 130,
  0, 1)`), so nothing is ever out of reach, and the lifter falls back to it
  where no clearer form round-trips.

## The lifter (bytecode to language)

- Walks the IR, names what it can (from the symbol tables), folds tags and
  conditions into `when`, recognises built-in shapes, puts messages inline.
- Keeps everything the bytes need: block numbers, message indices where
  the inline order would not reproduce them (an explicit `msg 12:` prefix),
  unreferenced messages, the Parody table's alignment, the label.
- Output is formatted canonically (one formatter shared with the compiler's
  `fmt`), so lifting is stable and diffs are readable.
- Lifting all four discs gives a readable corpus of the games' story,
  checked in? No: the scripts are the games' content, so the lifted text is
  made from your discs into `work/` like everything else, never committed.

## The compiler (language to bytecode)

1. Parse (a hand-written lexer and parser in Rust; precise errors).
2. Resolve names against the symbol tables of the target volume.
3. Type-check arguments, volume availability, message encoding.
4. Lower sugar and built-ins to IR ops.
5. Assign the message table (inline order, or the explicit indices).
6. Encode to bytecode through the existing `official` encoder.

Warnings, not errors: blocks that can never run, messages never said, a
`when` that contradicts itself, a character entered and never used.

## Symbols

One symbol table per volume, made from the build (no hand-typed lists
where the game has the data): `charTbl` names, the event table's labels,
area words, the field and town tables, `seTbl` and voice rows, BGM rows,
stream names, mail and item tables, wallpaper and BGM unlocks, the known
event-status names (from the docs, as constants). It lives in the
generated tables (piney-gen), so it is checked by `gen --check` like the
rest.

## Tools

- `piney-event lift` (a disc's events to `.evl` files), `build` (to the
  bytecode and into a mod or the build), `check`, `fmt`, and `explain`
  (an op's meaning and arguments, from the instruction set's docs).
- A grammar file for editors' syntax highlighting; a language server
  (completion of names, hover docs from the instruction set, errors as you
  type) is a later step.

## Mods

- A mods folder beside the build. A mod is a folder of `.evl` files and a
  small manifest (name, volume, what it replaces or adds).
- **Replace** an event by number, or **add** a new one (new numbers from a
  range the games never use); new messages, mails and items go through
  their own defs once defs and systems land.
- Load order and conflicts (two mods replacing one event) are reported, not
  guessed.
- The VM must hold up against any compiled script, not just the games':
  bounds-checked, never panicking, a bad op an error in the log. Fuzz the
  VM with compiled random scripts.
- A mod never touches the player's discs or the build's own data; the
  original scripts are always there to fall back to.

## Testing

- The round-trip over every event of every disc, both tables, byte for
  byte (the guarantee above).
- Parser and formatter: every lifted file re-formats to itself; a fuzzer
  over the parser (no panics, errors only).
- Golden tests of compile errors and warnings (the message text matters).
- The VM fuzz above.
- A few hand-written scripts run end to end in session tests (a new side
  event in a town, a replaced line), so modding is checked in play.

## Order

1. The language's grammar and the symbol tables (piney-gen).
2. The lifter, raw ops only, to the round-trip on all discs.
3. Names, `when`, inline messages, Parody, escapes; round-trip held.
4. The compiler with types and volume checks; error messages.
5. Built-ins one by one, each kept only if the round-trip holds.
6. The CLI and the formatter; docs: a language reference
   (docs/engine/event-language.md) and a short tutorial (write a side
   event).
7. Mods: the folder, the manifest, replace and add, the VM fuzz.
8. Editor support; a language server later.

## Decisions

Made by the maintainer, 2026-10-30:

- **Braces**, as drafted.
- **A rich language**: many built-ins for the games' common shapes, each
  with its lowering rule and kept only while the round-trip holds.
- **Implicit block numbers**: numbered by order, named only where
  referred to.
- **The assembly-level text form stays public**: documented beside the
  language for modders who want exact control; the CLI lifts and builds
  either form, and both round-trip.

Still open: the language's name and its file extension (`.evl` drafted).
