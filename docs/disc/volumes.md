---
title: The four discs
status: solid
volumes: all
covers: SYSTEM.CNF and every file on each disc, INF SLUS_202.67, MUT SLUS_205.62, OUT SLUS_205.63, QUA SLUS_205.64, QUA DATA/KFED.BIN, QUA DATA/KFAED.BIN, IOPRP243.IMG, OUT STREAM/STRT.BIN, OUT/QUA VOICE2
worklog: 20, 21, 23, 24, 25, 239
---

# The four discs

What changes from volume to volume, and how the stripped executables of
volumes 2 to 4 get their names. Each disc is described file by file on its
own page: [Infection](layout.md), [Mutation](mutation.md),
[Outbreak](outbreak.md), [Quarantine](quarantine.md). Everything here is
measured against Infection's.

## Volumes

| | INF | MUT | OUT | QUA |
| --- | --- | --- | --- | --- |
| game | .hack//Infection | .hack//Mutation | .hack//Outbreak | .hack//Quarantine |
| boot ELF (`SYSTEM.CNF`) | `SLUS_202.67` | `SLUS_205.62` | `SLUS_205.63` | `SLUS_205.64` |
| sectors | 1,268,832 | 1,830,448 | 1,859,600 | 2,176,912 |
| files | 86 | 93 | 96 | 100 |
| boot ELF bytes | 19,223,388 | 2,669,392 | 2,649,808 | 1,546,192 |
| symbols | all, with DWARF | none | none | none |
| main section | 0x00100000 + 0x278880 | + 0x28b800 | + 0x286b80 | + 0x179480 |
| `$gp` | 0x0037faf0 | 0x00392a70 | 0x0038ddf0 | 0x00280770 |
| overlay window | 0x00400800 | 0x00413800 | 0x0040f200 | 0x00301b00 |
| heap section | 0x00730600 | 0x00774b00 | 0x00773780 | 0x00670300 |
| `DATA.BIN` bytes | 136,665,088 | 138,172,416 | 142,372,864 | 142,372,864 |
| `DATA.BIN` records | 1,023 | 1,031 | 1,072 | 1,072 |
| `categoryFDTbl` | 0x002fb750 | 0x00310fd0 | 0x00308700 | 0x001fc680 |
| `cateCDOfsTbl` | 0x002fb7a0 | 0x00311020 | 0x00308750 | 0x001fc6d0 |

All four `SYSTEM.CNF` files read `VER = 1.00`, `VMODE = NTSC`. Each
executable carries the string `MW MIPS C Compiler (2.4.1.01)`. The three
stripped ones keep Infection's section layout by position (main, the four
overlay sections, heap) with the names blanked; `tools/image.py` maps them by
that order.

The four overlays (`GCMN.PRG`, `DEMO.PRG`, `DESKTOP.PRG`, `TOPPAGE.PRG`) are
the same [PRG format](../formats/prg.md) on every disc, loaded at that
volume's window. `DATA.BIN` is the same [archive](../formats/data-bin.md) on
every disc; `tools/fdtbl.py check` passes every record of all four.

## Files that differ

| directory | INF | MUT | OUT | QUA |
| --- | ---: | ---: | ---: | ---: |
| root | 4 | 4 | 4 | 4 |
| `DATA` | 7 | 7 | 7 | 9 |
| `MODULES` | 12 | 12 | 10 | 10 |
| `PSS` | 4 | 4 | 4 | 4 |
| `STREAM` | 4 | 8 | 6 | 6 |
| `VOICE` | 28 | 29 | 13 | 13 |
| `VOICE_E` | 27 | 29 | 13 | 12 |
| `VOICE2` | - | - | 21 | 21 |
| `VOICE2_E` | - | - | 18 | 21 |

- `STREAM`: INF has `STR1`, `STR1E`, `STRCMN`, `STRCMNE`. MUT adds `STR2`,
  `STR2E`, `STRSUB`, `STRSUBE`. OUT and QUA keep only the English (`E`)
  archives: OUT `STR1E`-`STR3E`, `STRCMNE`, `STRSUBE` and `STRT`; QUA
  `STR1E`-`STR4E`, `STRCMNE`, `STRSUBE`.
- `MODULES`: OUT and QUA drop `CDVDFSV.IRX` and `CDVDMAN.IRX`, which no
  volume loads: every disc's CD drivers are `IOPRP243.IMG`'s ([the IOP
  image](layout.md#the-iop-image)).
- `DATA`: QUA adds `KFED.BIN` (688,000 bytes) and `KFAED.BIN` (35,200).
- `VOICE2`: the characters' skill words, moved out of `VOICE` on OUT and
  QUA, with three new characters' ([the Outbreak DVD](outbreak.md#voice2)).
- `STREAM/STRT.BIN` (OUT only): six scenes no code opens, demo discs'
  screens ([the Outbreak DVD](outbreak.md#strtbin)).
- `IOPRP243.IMG`, `DATA/ICON.BIN`, the logo movies, `OUTSIDE.BIN` and eight
  of the modules are byte-identical on all four discs. Each disc's page
  says, file by file, which other discs hold the same bytes.

## The font tables in Quarantine

Infection keeps its bitmap fonts in the executable, between the last code
and the first data: `kf14x16` (385,280 bytes), `kf20x20` (688,000),
`kfa20x20` (35,200), `kfa14x16` (19,712), `ef12x20` (13,440), `ef8x16`
(7,168), 1,148,800 bytes in all. From the end of the code to the first named
data object is 1,166,168 bytes in INF, 1,122,628 in MUT and 1,175,028 in
OUT, but only 36,868 in QUA.

MUT and OUT embed all six byte-identical to INF. QUA embeds only `ef12x20`
and `ef8x16` - the two that INF's code uses - and ships `KFED.BIN` and
`KFAED.BIN`, the same sizes and sheet layout as `kf20x20` and `kfa20x20`. In
INF the four kanji sheets are unreferenced. KFED's bytes differ (0.2% equal)
because the palette indexes changed (background 1 in INF, 0 in QUA) and the
glyphs were re-shaded; most of the ink falls in the same places. See
[text rendering](../engine/font.md#the-kanji-fonts).

## DATA.BIN across volumes

Records per category, from each executable's own tables:

| category | INF | MUT | OUT | QUA |
| --- | ---: | ---: | ---: | ---: |
| desktop | 136 | 136 | 177 | 177 |
| spc | 18 | 21 | 21 | 21 |
| pc | 37 | 38 | 38 | 38 |
| town | 14 | 15 | 15 | 15 |
| event | 50 | 53 | 53 | 53 |
| the other 15 | same | same | same | same |

- MUT drops `pc/CBUZ.CCS` and adds 9 records: `CTBUZ` and `CTWMK` (pc),
  `CHA3BODY`, `CTU4BODY`, `CWM5BODY` (spc), `TOWN06`, and `HANABIRA1`,
  `X7478CAM`, `X7484CAM` (event). Of the 1,022 records in both INF and MUT,
  992 inflate to the same size.
- OUT adds 41 desktop records to MUT's (`ENDING`, `XDDWAL56` onward, ...) and
  drops none.
- OUT and QUA have the same 1,072 records. Five members differ: four desktop
  images (`XDDN_050`, `XDDN_400`, `XDDN_490`, `XDDN_690`, same size, different
  content) and `boss/X11.CCS` (310,052 bytes in OUT, 326,660 in QUA).

## Names for the stripped executables

`piney-gen syms` writes each later volume's `<elf>.syms` (`tools/xfer.py
match` did until 2026-09-27, byte for byte the same), and
`tools/image.py` loads it automatically for an executable with no symbol
table, so `disasm.py`, `fdtbl.py` and the rest work on every volume by
Infection's names. One symbol per line, tab-separated:

```
section  va(8 hex)  size  FUNC|OBJECT  name  how
```

`section` is `main` or an overlay name. `how` says which pass named it:

| how | meaning |
| --- | --- |
| `exact` | the body equals Infection's with address fields masked, uniquely |
| `call` | the target of a call, or a function address taken, from a paired body |
| `order` | between two paired neighbours, aligned by the shape of its code |
| `data` | the global a paired body addresses at the same instruction, or in the same place in the order of the addresses it builds |
| `pointer` | a global a carried global points at, where Infection's relocation says it points (a unanimous vote also moves a `data` name) |
| `content` | a global found by what it holds: its numbers equal, its pointers to the same strings; not when two of Infection's hold the same |
| `layout` | between two carried globals whose words agree and which sit the same distance apart in both, a global at its offset whose words agree (90%), or a `data` name moved there because they agree better; or two `data` names of one size that agree better each at the other's place |
| `ref` | a function that builds a carried global's (or a string's) address, where only one function does on each side |

`FUNC` sizes are the destination's own (to the next entry point, less
padding), except for `exact`, whose size is Infection's.

| | MUT | OUT | QUA |
| --- | ---: | ---: | ---: |
| `exact` | 3,518 | 1,029 | 1,026 |
| `call` | 604 | 500 | 499 |
| `order` | 1,214 | 3,331 | 3,311 |
| `data` | 4,329 | 3,262 | 3,248 |
| `pointer` | 1,093 | 1,343 | 1,333 |
| `content` | 1,662 | 1,533 | 1,505 |
| `layout` | 173 | 140 | 136 |
| `ref` | 3 | 72 | 70 |
| total | 12,596 | 11,210 | 11,128 |

The last four passes were added for the per-volume data
([plans/volumes.md](../../plans/volumes.md)): Outbreak's and Quarantine's
recompiled `EVENTAREA` and `ROOTTOWN` constructors carried no names, so
their tables were found by content and the constructors by the tables and
strings they build. The order is content, pointer, layout, content again,
ref, pointer again: what a global holds outranks a table's pointer to it,
since a table can keep Infection's layout while rows are added to what it
points at (Mutation's `pcMsg4`). The pointer pass does not take a
pointer's bytes for text (`0x006f2d98` reads "\x98-o").

The data pass pairs a function's globals by the order its code builds
them, and a reordered function misleads it. The layout pass found, on all
three: `BGTBL` and `BGTBL2` swapped (so every `BG_TBL_x` the pointer pass
reached through them was `BG_TBL_x2`), `BgMatName` unnamed, the Grunties'
voice tables (`gusoponTbl` to `aquaTblE`) each on its neighbour, and
`tradeMenuStr`, `evs209Tbl` and eleven stream tables elsewhere. Each moved
name was checked: Infection's bytes at its new place, not its old one.
In `.bss` nothing tells, so no span is anchored or filled there: a first
try placed main's `SkillDamageValueAttributeCritical` 8 bytes off, which
`test_battle.py`'s run of Mutation's `ccSkillDamageValue` caught.

Infection has 4,998 functions of 6 or more instructions across main and the
overlays. MUT's code is mostly Infection's instruction for instruction. OUT
and QUA were compiled differently:
- saved registers are allocated differently;
- branch delay slots Infection fills are often left as nops;
- `fptoui` is inlined.
So there only about a fifth match exactly, and link order does most of the
work.

Precision, measured by carrying names back: MUT's names onto Infection give
9,512 symbols, 7 wrong (0.07%). OUT's names onto Infection give 7,927, 18
wrong (0.23%). The wrong `order` names sit on a neighbouring function; the
wrong `data` names on a different object, or inside a larger one. Two functions with identical bodies (`_kill_r`,
`__sigtramp`) cannot be told apart.

## Systems across volumes

Each system below was checked by running every volume's own code in
`tools/eemu.py` against the Python reimplementation, with the differences
read from each executable rather than keyed on the volume:

| system | changed in later volumes | page |
| --- | --- | --- |
| area generator | areas 71 and 47 become substitute records; keyword and story-area table edits | [area keywords](../engine/area-words.md#other-volumes) |
| dungeons | floor count a field (15 for area 125), 90 hand-made dungeons, a new save bit in the type rule | [dungeons](../engine/dungeon.md#other-volumes) |
| fields | `sqrt.s` in OUT/QUA; area 100 unprotected | [fields](../engine/field.md#other-volumes) |
| battle | Exdefense rule, 21 party members, table rebalancing | [battle](../engine/battle.md#other-volumes) |
| text | `ccKanjiStrlen` counts every `%x` in OUT/QUA; more mails and replies | [text rendering](../engine/font.md#other-volumes) |
| event scripts | opcodes 168, 169, a new operand for 99, new events per volume | [events](../engine/events.md#other-volumes) |
| saves | a 0x854-byte extension for the three new characters; carry-over | [save](../formats/save.md#carrying-a-save-forward) |

This work also found carried names that are wrong or missing:
- MUT's `MailTbl` points into `desktop.prg`'s code.
- MUT's `skillTbl` points 0x11b0 bytes into the table.
- OUT's and QUA's `skillTbl`, and MUT's `enemyTbl`, are not named at all.

The tools find these tables from the code that uses them instead.

## What the later volumes' code adds

`tools/voldiff.py gaps ELF` measures, section by section, the text the
carried names do not cover, and lists the stretches between two named
functions; `tools/voldiff.py look ELF SECTION LO HI` prints the strings a
stretch materialises and the named functions it calls, which tells new
code from code compiled too differently to be matched. On Infection itself
only main's embedded fonts show.

| | INF | MUT | OUT | QUA |
| --- | ---: | ---: | ---: | ---: |
| main text | 0x275968 | 0x288868 | 0x283be4 | 0x1764e4 |
| main unnamed, less the fonts | 0x160 | 0x19ed0 | 0x1541c | 0x1ca88 |
| `GCMN.PRG` text | 0x1d0fc0 | 0x1ec240 | 0x1ee340 | 0x1f2dc0 |
| `GCMN.PRG` unnamed | 0x78 (0.0%) | 0x112c0 (3.5%) | 0x4be00 (15.4%) | 0x510d0 (16.2%) |
| `DESKTOP.PRG` unnamed | 0.1% | 0.1% | 1.4% | 1.4% |
| `TOPPAGE.PRG`, `DEMO.PRG` unnamed | 0.6%, 0.4% | 0.9%, 0.4% | 1.4%, 0.6% | 1.4%, 0.4% |
| stretches | 13 | 80 | 173 | 170 |

(Measured 2026-09-27; the carry has named more since the first count.)
Every stretch is listed, with its neighbours, strings and calls, in [the
later volumes' unnamed code](unnamed-code.md).

"Less the fonts" leaves out the one stretch from `DEG2RAD` to the static
constructors: the bitmap fonts (in QUA, which moved two of them to
`KFED.BIN` and `KFAED.BIN`, whatever took their place, 0x86ac4 bytes).

The stretches of 8 KB or more, and what they are:

| volume | where | bytes | what (by its strings and calls) |
| --- | --- | ---: | --- |
| MUT, OUT, QUA | main, before `ccInitStreamDemoThread` | 0xffb4 / 0x10584 / 0x17ce4 | the new streams' effect functions (`Func_str0710e`, `str1070e`, `str1430e`: `ccRasterNoize`, `ccBufferSampling`, `ccBufferReverce`, fades) |
| MUT | main, after `ccEvent::Execute` | 0x3490 | the event engine's new instructions (overlay loads, `SimGenerateCode`, `RoomSelect`, party add and delete, gate list, items) |
| MUT, OUT | main, after `Func_str9001` | 0x332c / 0x2778 | stream `str9001`'s effects (`OBJ_se1_6flo1`) |
| MUT, OUT, QUA | gcmn, after `BIRD::Move` | 0x3cf8 / 0x3c94 / 0x85b4 | the Grunty race (`PG_RACE`, `town06`, the `xp_` cups, flags and count); QUA adds its staff roll (`STFROLL_VOL4`) and the hacking logos |
| MUT, OUT, QUA | gcmn, after `AreaInfoMenu` | 0x59c4 / 0x5b20 / 0x5b20 | a new menu (messages, `OpenInfo`, number input) |
| OUT, QUA | gcmn, after `ccBoss08KillerEye::OnThinkDead` | 0x25da0 | Kyvia's fights (`kyviaCore`, `Kyvia01`-`04`, the `ex0`-`ex4` models, the boss camera and particles) |
| OUT, QUA | gcmn, after `ccFellowCheckNote` | 0x10ec0 | the Root Towns 02-05 (`town02`-`05` static models, their water and lights) and the party of 12 and more (`SPC_01`-`SPC_12`, `expulsionSpc`) |
| OUT, QUA | gcmn, after `EVENTAREA::EVENTAREA` | 0xaf60 | the other event areas (`se1_1`, `se1_2`, `se2_1`, `se2_6`: their static models, animations, lens flares) |
| OUT, QUA | gcmn, after `ccEnemyG::actEscapeGold` | 0xa6cc | the enemy races recompiled (their constructors, acts, breath), with `ccEnemyL`'s type 3 (`OBJ_elg1body`) |

MUT's code is Infection's with these additions. OUT and QUA were compiled
differently, so part of what shows there is Infection's code under other
register choices (the enemy races are); the Kyvia fights, the towns and
the event areas are new.

## Settled

What this page once left open:
- **`KFED.BIN` / `KFAED.BIN`.** Event opcode 169 (Quarantine's ending)
  reads both and frees them after the staff roll (`STFROLL_VOL4`); no
  instruction reads them in between. The staff roll draws its text with
  the executable's own fonts ([the Quarantine DVD](quarantine.md#kfedbin-and-kfaedbin)).
- **`STRT.BIN`.** Six scenes no code opens: the Infection and Mutation
  demo discs' title screens, a "February 2003" teaser and a four-part
  selector ([the Outbreak DVD](outbreak.md#strtbin)).
- **`VOICE2`.** The characters' skill-word banks: Infection's 18, moved,
  and Tsukasa's, Subaru's and Sora's ([the Outbreak DVD](outbreak.md#voice2)).
- **The CD modules.** They did not move into `IOPRP243.IMG`; the image
  already carried newer ones on every disc. Infection's and Mutation's
  `MODULES/CDVD*.IRX` (1.04) were never loaded; the image's (2.26) are
  ([the IOP image](layout.md#the-iop-image)).
- **The stretches.** All 423 are listed with what can be read of them
  ([the later volumes' unnamed code](unnamed-code.md)).

## Unknown

- The names of the functions in the unnamed stretches: none survive to
  carry. They are read from their strings and calls, and from their code
  where a system is ported.
