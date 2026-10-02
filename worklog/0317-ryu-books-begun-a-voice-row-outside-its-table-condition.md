---
number: 317
title: Ryu Books begun, a voice row outside its table, condition effects on the flute and on a foe struck with the party gone
date: 2026-10-01
area: ui, audio, battle
files: crates/piney-fieldui/src/book/mod.rs, crates/piney-fieldui/src/book/pages.rs, crates/piney-data/src/tables/book.rs, crates/piney-audio/src/driver.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs
resolves: 300
---

# 317. Ryu Books begun, a voice row outside its table, condition effects on the flute and on a foe struck with the party gone

This was the first chunk of the sweep of open questions left by the week's fixes. The agent working
on it stopped partway (an account limit). The rest was finished by hand to
a state that builds and passes, and is recorded here as it stands. All addresses are INF.

## A skill's word outside its table ([[300]])

`skillVoicePlay` (main 0x0017e350) reads its row from the character's table
with no bound. A row past the table reads whatever lies there in the
executable's memory, sends it, and empties the queue.

The port kept the word queued, so after one such skill nobody spoke again,
Kite included.

The port now holds the memory the game reads from (`skill_memory`: every
table and the rows within reach, ids 0-303, as `VOICE_DATA`). A row
outside a table sends what lies there and empties the queue. A row of -1
still leaves the queue for the next frame. Test:
`a_word_outside_its_table_does_not_stick`.

## Condition effects ([[310]]'s three paths)

- **The Grunty Flute.** `ccClearConditionAllEnemy` (gcmn 0x0042e4f0)
  clears the enemies on the list, in its order, with `objFlag` set and
  `freezeFlag` clear: the ones `ccCheckActiveEnemy` counts. Each one's
  condition effect now ends. Test: `the_flute_ends_the_foes_sparks`.
- **A foe struck by a foe with none of the party left.** `affectEnemy`'s
  `selectTarget()` on the struck foe clears its target (gcmn
  0x00436d78-0x00436d84). The `ClearConditionEffect` it raises is the
  struck foe's own. The port handed it to the striker's frame, ending the
  wrong foe's effect, so the struck one kept its sparks. Outputs from a
  rule run outside an enemy's own frame are now kept per enemy and flushed
  after the frame's own. Test: `a_blow_with_the_party_down_ends_the_sparks`.
- **The `EnemyRetarget` flush, and the attack cursor's scale (gcmn
  0x0051f21c):** not settled in this chunk.

## The Ryu Books, in part

What is in:
- `ccThBook`'s steps in the item use (crates/piney-fieldui/src/book):
  - the fade to black;
  - the cover stream, stream 112 + the book;
  - the book's pages opened once the stream ends.
- `Request::BookStream(Some(book))` and `(None)` are now answered in fields
  (the drain movie's player, `AreaMode::start_movie`) and in towns (the
  scripts' stream player, stepped a menu frame at a time). Before, nothing
  played the cover, and a book would have waited for it forever.
- The `BOOK` class's tables are in the build (`piney-data` `tables/book.rs`,
  the manifest; data version 22).
- Pages, rows and reward checks are ported for books 1-3 (`type` 0-2:
  `Disp01`-`03`, `PadControl`, `CheckItemGet`). `PadControl` is also
  ported for types 5 and 6.

What is not:
- Books 4-8 (`type` 3-7): their `DispNN` and `CheckItemGetNN`, and the
  reward tables `T40` on.
- The cover's palette from the second book on (`bookItem`'s
  `stream_cluts`).
- A session test that reads a Ryu Book end to end.

**Still unknown:**
- Books 4-8's pages and rewards (`BOOK::Draw`, `PadControl`,
  `CheckItemGet` for types 3-7; INF gcmn 0x0040c570-0x00410400) are not
  ported.
- The second book on: the cover's palette is not swapped.
- No test reads a book through.
- The `EnemyRetarget` flush and the attack cursor's scale (0x0051f21c) were
  not checked.
