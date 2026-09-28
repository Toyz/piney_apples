---
number: 204
title: "The later volumes' New Game to the desktop: the carry's pointers, each volume's mail, the streams' archives, the bound constants"
date: 2026-09-26
area: volumes
files: tools/main_data.py, tools/sinit_tables.py, tools/desktop_tables.py, crates/piney-data/src/sinit.rs, crates/piney-desktop/src/content.rs, crates/piney-desktop/src/mail.rs, crates/piney-desktop/src/tables.rs, crates/piney-stream/src/table.rs, crates/piney-stream/src/load.rs, crates/piney-stream/src/subtitle.rs, crates/piney-game/src/main.rs, crates/piney-fieldui/src, crates/piney-world/src, crates/piney-battle/src, plans/volumes.md
---

# 204. The later volumes' New Game to the desktop: the carry's pointers, each volume's mail, the streams' archives, the bound constants

After 0203 Mutation had sound, but its New Game stopped in the desktop's
setup:

- an empty message window;
- name entry reading address 0;
- the desktop failing to build.

Outbreak and Quarantine got no further, and their streams did not open.
All three now reach their desktops, each with its own mail. The window's
title names the disc's game.

## The name entry's strings

`STR_INFO_MSG_0` is a `static const char *` from a header that every
desktop translation unit repeats (ten copies on Infection). Only the
unnamed `__sinit_NameEntry.cpp` builds its address, so no vote reached
it.

The carry has a new pass, `by_pointers`, for an uncarried global made of
data pointers (Infection's word relocations, none into a function) and
zeros. It carries the global to where the volume holds words that point
into the same section at the same bytes. If several places match, or
Infection repeats the content, it takes the place as far from the carried
global below as on Infection. A place two globals claim is dropped.

On Mutation this carried 343 globals: every `STR_*` run, the book's
strings and the `pcMsg` tables. The runs land at Infection's layout plus
0x12fbc and 0x12fc0, matching what the code's votes say for
`STR_STR_MODE`, `STR_NORES` and `STR_CLEAR`.

The misses the source still names fall from 8 to 7 on Mutation, 44 to 29
on Outbreak and 43 to 28 on Quarantine.

## The mail

`ReMail`'s only user is also a static initializer: `__sinit_mailtbl.cpp`,
which links each mail's two replies. Every volume's desktop.prg lists
the same 15 initializers in the same order. So the later volumes'
unnamed ones sit where Infection's do in the overlay's constructor list.

`tools/sinit_tables.py` now runs each volume's `__sinit_mailtbl.cpp` and
`__sinit_mailtblp.cpp` in eemu over a `ReMail` of marker words.

- `MailTbl` comes from the carry.
- `ReMail` is the run of addresses the initializer builds right below
  `MailTbl`.
- `MailTbl` runs up to `ReMailp`.

| | mails | replies |
| --- | ---: | ---: |
| Infection | 326 | 294 |
| the later volumes | 375 | 364 |

Infection's generated links equal the table `tools/desktop_tables.py`
had made, which is dropped. `piney_data::sinit::mail_links` gives each
volume's `ReMail` address, its rows and the links, and
`content::mail_count` the mails.

Mutation's carried name `MailTbl` (0x0042dfd0) is wrong. `AddMailList`
builds 0x00431660, and the code-over-names rule already carries that.

## The streams

- **The archive types.** The later `ccCdInit`s store each archive's name
  at +0x54 + 36 × type. The order is CMN, STR1, STR2, STR3, STR4, then
  `STRSUB` as type 5, on all three discs. Types 3 and 4 are volumes 3
  and 4's streams (50-74, 75-111) and 5 the shared ones (126-133). The
  port had `STRSUB` at 3, so Outbreak's title stream read `STRSUBE`
  instead of `STR3E`.
- **The member names.** Outbreak's and Quarantine's archives name every
  gzip member `tmp.ccs`. Such a member is renamed to the table's name, so
  the scene archive finds it by stem. The CRC covers only the inflated
  data.
- **The subtitles.** `read_table` picked `EV_STR_MSG_TBL` or its parody
  twin with an `if` and read it without `at`. On Quarantine the address
  is overlay space: its overlays load at 0x00301b00, not 0x00400000. On
  the other two it read garbage.

## Constants bound before a read

The first sweeps wrapped direct reads (`image.read(CONST, ...)`). A scan
of every use of an Infection-address constant found the other shapes.

- `let at = NPC_TBL + ROW * row; img.u32(at)`, at these sites:
  - `NPC_TBL` (twice), `SKILL_TBL`, `CHAR_TBL`, `ENEMY_LIST_INFO` (twice);
  - `EVENT_AREA_INFO`, `MENU_ELEMENT_DATA`, `GIMMICK_TBL` (twice);
  - `FELLOW_ANIM_TBL`, `THREAD_TBL`;
  - the trade and friendship rate tables, `TPC_TRADE_LIST`;
  - the breeder's and the presents' message tables, `FOOD_TBL`;
  - `ERROR_DATA` (three places);
  - `CHECK_VOICE_GRP`, the special names' list.
- Constants handed to helpers that did not carry:
  - the item box lists, the shop stock;
  - the map's icon tables, the Grunties' camera;
  - the option menu's texts, the announcements' pieces;
  - the load screen's server names, the town PCs' names.
- The breath params, read at the address the enemy's motion passes.
- The equipment tables, read as one span, now take each table's own
  carried base. The Twin Blades' and the Heavy Axes' tables grew by a row
  from Mutation on.

On Infection `at` is the identity, and every test passes unchanged.

**Still unknown:** Outbreak's and Quarantine's `ccParticle::Setup`
switch, which the port decodes from the function body. Both recompiled
it, so the stream effects fail there. The remaining carry misses. Whether
other readers take an Infection constant through a shape the scan does
not see (a struct field holding one, read later).
