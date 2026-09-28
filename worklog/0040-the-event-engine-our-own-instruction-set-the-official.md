---
number: 40
title: The event engine: our own instruction set, the official scripts loaded without loss, the interpreter checked against the game
date: 2026-09-23
area: script, decomp, test, build
files: crates/piney-event, tools/test_event_vm.py, tools/evscript.py, docs/engine/event-vm.md, docs/engine/events.md
---

# 40. The event engine: our own instruction set, the official scripts loaded without loss, the interpreter checked against the game

[[18]] decoded the story's scripts and [[24]] read them on every volume.
To see the desktop as a new game sees it, the scripts have to run: they
deliver the first mails, lock the desktop's other icons until those are
read, and post the board and news.

The user also asked that the scripts could later be replaced by our own
clean-room ones. So the engine is built in three layers, and the official
bytecode is only one way in. A helper agent built it; I re-ran its checks.
The reference is [the event interpreter page](../docs/engine/event-vm.md).

## Layers

- **`ir`: our own instruction set.** A script is its open conditions and
  blocks of (tags, conditions, instructions).
  - It has 169 instructions, 40 conditions and 10 tags, each an enum variant
    with named fields (evscript.py's names).
  - It has no opcode numbers and no offsets.
  - `text` prints and parses a readable form, and the two round-trip exactly.
    This is the format a clean-room script set would be written in. A
    made-up script runs through the whole engine in `tests/clean_room.rs`.
- **`official`: the adapter for the game's scripts.** It finds `Execute`,
  `CheckOpen`, `eventTbl` and the message tables through the code, with no
  symbols, on all four volumes, and maps opcodes per volume.
  - Every script on every disc goes bytecode to IR to identical bytecode:
    192, 196, 199 and 206 scripts.
  - All 192 Infection events round-trip through the text form, with 2,369
    messages.
  - The scripts and messages are the game's content. They are read from
    the disc at run time. `examples/export` writes their text form only
    under `work/`.
- **`vm`: the interpreter.** It is `CheckOpen`, `SetCurrentOpen`,
  `Execute` and `eventSub` at levels 0-2, and `ccThEvent`'s passes by phase.
  - Instructions that span frames keep the game's own frame counts.
  - Every effect goes through a `Host` trait of named, typed calls:
    messages, mail, modes, sound, the field's cameras and characters. A
    `LogHost` records them.
  - The saved state is the shared `SaveData`. The ccSaveData functions the
    scripts call (`AddItem` with its category sort, `DelItem`, `AddSkill`,
    `AddFriendship`, `SetGateList`, `SetAreaBan`) are ported.
  - The rest of `ccEvent` is `EventMng`, with its offsets documented.

## Checked

`tools/test_event_vm.py` runs the game's own event and save code in eemu,
and `tests/vm.rs` compares the Rust against the fixture it writes. The
fixture holds numbers only; I grepped it and it has no script text. All
match:
- **Bookkeeping.** `ccEventFlagSet` for every script from the boot save
  and three random saves: 768 runs, every save byte and the event-manager
  registers.
- **Starting events.** `ccStartEvent` on every volume, and
  `ccStartThEvent`'s 63-to-62 rule.
- **Conditions.** 1,536 `CheckOpen` walks, and each of 2,059 conditions in
  8 random worlds.
- **Playing.** 576 `eventSub` runs, and every block played at level 2:
  2,616 blocks over 169,146 frames, matching frame counts, saves and
  registers.
- **A new game reaching the desktop.** 152 frames under the game's own
  `ccThEvent`. The phase matches every frame, every call outside the
  event code matches in kind and frame, and three save snapshots match.

Deliberate mistakes planted in the Rust are caught.

**What a new game does, as the checks show it.** At boot `ccStartEvent(1,
0)` marks event 0 done. When the desktop's phase-0 pass runs, event 1's
block 0:
- plays from frame 2 to frame 59;
- locks every desktop operation but the mailer;
- delivers mails 4, 5 and 320;
- posts board thread 62's first seven messages and news 0-3 and 37.

Trying a locked icon replays a message (block 1). Reading mails 4 and 5
releases the locks, posts 53 more board messages (not the 57 [[18]]
counted), and closes the event.

On integration:
- the crate's tests pass;
- `test_event_vm.py` and `test_evscript.py` are OK;
- clippy and fmt are clean.

The agent also corrected seven instruction descriptions in `evscript.py`,
such as `wait`, which waits count + 1 frames.

**Still unknown:**
- **The later volumes' own semantics.** The VM runs Infection's rules for
  every dialect, including the changed cases on the events page and the
  order their event task walks story events.
- **Story areas.** The table of story areas' gate words and servers is not
  loaded from the executable yet; the host supplies it.
- **`piros_colour`.** Its frame shape is inferred; it needs Piros present.
- **The field side of the Host.** Markers, distances and entries come from
  whoever ports the field game.
