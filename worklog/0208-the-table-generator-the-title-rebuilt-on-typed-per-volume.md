---
number: 208
title: "The table generator: the title rebuilt on typed per-volume tables"
date: 2026-09-26
area: volumes
files: tools/static.py, tools/static_manifest.py, crates/piney-data/src/tables, crates/piney-data/src/world.rs, crates/piney-demo/src/newgame.rs, crates/piney-demo/src/dialog.rs, crates/piney-desktop/src/dtmenu.rs, crates/piney-desktop/src/kanji.rs, crates/piney-game/src/loaddisp.rs, plans/volumes.md
---

# 208. The table generator: the title rebuilt on typed per-volume tables

After 0207 nothing read the executable's data. This entry builds the
generator `plans/volumes.md` describes, and rebuilds the first system on
it: everything the title reads.

## The generator

`tools/static.py` writes `crates/piney-data/src/tables` from the manifest
`tools/static_manifest.py`. The plan's "The generator" section has the
rules. Most of them came from the owner reviewing the first output:

- **The game's own types.** The first cut kept a character's row as 92
  raw bytes. Rows are now structs built from Infection's DWARF: a
  character's row is `SpcParamData { base: CharBaseParam { name, level,
  gold, height, .. }, elm: CharParamElement { battle_ability, attribute,
  tolerance }, equipment, velocity, job, friendship }`, and a trade is a
  `TradeList { lst: ItemList { id, category, num }, sw }`. Each such
  struct lays itself into the save (`write`).
- **Text as text.** Strings are Rust `&str`, checked to encode back to the
  game's Shift-JIS byte for byte, with no terminators or escapes: a
  server's symbol reads `"Δ"`, a menu row `"セーブ選択"`. A copy's limit
  ("at most 20 bytes, a NUL if it fits") is the reader's
  (`newgame::copied`). The game's fixed-width menu strings are their words
  (`["OK", "Cancel"]`); `dtmenu::slots` rebuilds the slot text the drawing
  code copies.
- **Enums for closed sets.** `piney_data::world` holds `Server` and
  `Town`. The load screen's town cards and server symbols are methods on
  them (`Town::MacAnu.card()`, `Server::Delta.symbol()`).
- **Written once.** A value the four volumes hold alike is one module
  item. Only what differs is per volume, behind `of(volume)`. A whole row
  some volumes share is written once and named by its place. Floats print
  as plain decimals. A trade list's trailing empty slots are left out.

## The title's tables

| group | what | differs by volume |
| --- | --- | --- |
| `title` | `timeIdolRankDefStr`, the card dialog's words, `saveSysMsg` (from each volume's own initializer in eemu) | the card messages' wording |
| `fonts` | `ef8x16`, `ef12x20` | the two glyph rows read past `ef8x16`'s end |
| `loaddisp` | the towns' cards, the servers' symbols, "Server" | nothing |
| `newgame` | `charTbl`, the default trade lists, where the name lists are | the characters, the lists |
| `dtmenu` | `dtMenuElementData`, the option texts, the save menu's | the clear-data question |

The readers now take these tables:

- `InitText::of`, `DialogAssets::read`, `Fonts::of`;
- `LoadDisp::new`, `NewGameTables::of` and `new_game`, which writes each
  typed member into the save's `ccSpcParam`;
- `DtMenu::new`, `SaveTexts::of`.

The desktop's and the top page's assets carry the disc's volume. Tests and
probes no longer read `SLUS_202.67`.

All nine of piney-demo's title tests pass again (they failed after 0207),
and the save and desktop suites pass.

**Still unknown:** The rest of the rebuild: the desktop's own tables, the
top page, the field UI, the world, the battle, the effects and the
streams. `msg` in a character's row is an address (its message table's),
kept as the volume holds it until what reads it is rebuilt.
