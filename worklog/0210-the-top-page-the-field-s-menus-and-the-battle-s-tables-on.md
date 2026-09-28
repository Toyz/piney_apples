---
number: 210
title: The top page, the field's menus and the battle's tables on generated tables
date: 2026-09-26
area: volumes
files: tools/static.py, tools/static_manifest.py, tools/carry.py, crates/piney-data/src/tables, crates/piney-toppage/src, crates/piney-fieldui/src, crates/piney-battle/src, crates/piney-world/src/foe.rs, crates/piney-desktop/src/name_entry.rs
---

# 210. The top page, the field's menus and the battle's tables on generated tables

The rebuild after 0209 goes on toward Infection booting end to end again,
one volume at a time. Three more systems read only generated tables: the
top page, the field's menus and the battle's parameters. The field's menus
now load on every disc (`FieldUi::new` failed on the executable's data
since 0207).

## The new groups

| group | what |
| --- | --- |
| `toppage` | the board's 63 threads and their posts, normal and parody; its words; the idle animation by volume |
| `game` | `ccGame::ChangeScene`'s server of each town |
| `fieldui` | the 89 menu lists, every page's help and words, the Chaos Gate's texts, the item lists the boxes draw from, the shops' stock, the trade rates, the party's message tables, the error record |
| `battle` | skills, items, equipment, enemies, bosses, level-ups, the enemy lists, `gimmickTbl`, `npcTbl`, the party's AI, the default item lists, the item icons |
| `party_chat` | the 78 tables of the party's chat lines |

Findings along the way:

- **The server of a town.** Infection's `ChangeScene` reads it from
  `@1489` (`0 1 2 3 4 0 1 2`). Mutation, Outbreak and Quarantine dropped
  the table: their server is the town number itself.
- **An empty equipment slot.** With nothing worn (-1) the game reads the
  0x48 bytes before an equipment table. Those are now a typed row per table
  (`*_before`), not a raw span of the executable.
- **An enemy's animation names.** They are `char[][30]` tables of up to
  16 names (`TextRows`). An enemy now names its table by its row, not by an
  address.
- **Shared structs.** Every struct built from the DWARF is written once, in
  `tables/types.rs`. Two layouts under one name stop the generator (the
  enemies' skills and `skillTbl`'s now share one description layout).
- **Uniform access.** Every group has its struct, `of(volume)` and a
  method per field, whether a value is shared or the volume's own.

The readers now take these tables:

- the top page's `Assets`;
- the field UI's `Texts` and every page's texts: items, words, hack,
  drain, record, personal, equipment, party, option, objects, fountain,
  talk, trade, shop and breeder;
- `piney_battle::Tables::of(volume)`;
- the enemy motion data and the world's `EnemyLook`;
- the new-game setup (`setup_new_game(state, volume, ..)`).

Short gcmn literals ("WORD", "Lv.", "#G", ":") are the readers' own
constants. The desktop menu's option words are the field's too, so they are
written once, in `dtmenu`.

## Finding a table on the later discs

The carry misses some tables and misplaces others. Two cases found here:

- Outbreak's `gateWordListMsg`: the volume edited it, so no place matches
  every pointer.
- Outbreak's `shopStr`: the carry placed it wrongly in `.sdata`.

The generator now weighs several candidates for each entry:

- the carry's place;
- **by code**: Infection's instructions round each place that builds the
  entry's address (`lui` pairs and `$gp`-relative accesses), matched by
  opcode in the volume's code at windows of 8, 5 and 3 instructions;
- **by neighbour**: where Infection's nearest globals went, shifted by
  their distance (a translation unit's `.sdata` keeps its order).

Each candidate that reads as the layout is scored by how much of its value
matches Infection's. The best wins, the carry on a tie. The narrowest code
window alone gave false unique answers (one address for a dozen entries),
which the scoring discards.

In `tools/carry.py`, `by_pointers` also takes a table most of whose
pointers match, when one place clearly wins. This added
`gateWordListMsg` on all three later discs and changed nothing already
carried.

## Kite's face in name entry

Name entry drew `TEX_xwin_f18` on every disc. That is Infection's
`ccFacePanel` picture: Kite before the bracelet. Mutation, Outbreak and
Quarantine use `TEX_xwin_f00`, red Kite with the bracelet. It is now the
volume's (`nameentry::face_tex`).

## Checks

These harnesses pass against the game:

- the field UI's main, talk, personal and option suites;
- name entry;
- the battle's.

The battle crate's tests and the event VM's pass. piney-game has 97
failures, as before this work; they have moved on to the world's and the
areas' tables (`The World: ... not in the image`).

**Still unknown:** Six trade and shop harness cases fail: a trading PC's
line shows `errorData`. The trading PCs' `npcTbl.msg` points into main's
`.sdata` (0x00378208 on), and the talk pages still read their records
through GCMN.PRG's image at run time, which has no main. The records and
the pointer tables they are reached through need a generated index by
address, which the save's stored pointers (`base.msg`) require. The world,
the areas, the streams and the story announcements still read the
executable.
