---
number: 9
title: The desktop text: mail, board and news, in English and parody
date: 2026-09-22
area: script, ui, tooling, decomp
files: tools/eemu.py, tools/text.py, docs/engine/text.md
---

# 9. The desktop text: mail, board and news, in English and parody

The in-game operating system's text is compiled into the overlays as C
tables, and the DWARF ([[6]]) names every struct. `tools/text.py` dumps it;
the full layouts are on [the text page](../docs/engine/text.md).

| table | overlay | type | entries |
| --- | --- | --- | ---: |
| `MailTbl` / `MailTblp` | desktop | `ccMailData`, 0x48 | 326 each |
| `ReMail` / `ReMailp` | desktop | `ccReMailData`, 0x14 | 294 each |
| `bbsThreadTbl` / `bbsThreadTblP` | toppage | `ccBBSThreadList`, 0xc | 63 threads |
| `bbsMsgTbl` / `bbsMsgTblP` | toppage | `ccBBSMsgList`, 0x14 | 341 posts |
| `HtmlTbl` | desktop | `HtmlData`, 0x1c | 70 news pages |

## The twin tables are Parody Mode, and they are Japanese

Every table has a twin with a `p` or `P` suffix. `AddMailList__16MailList_controlFv`
(`INF desktop.prg:0x00409800`) loads a byte at `saveData + 0x842b` and uses
`MailTblp` when it is non-zero, `MailTbl` when it is zero. The DWARF names
that byte `ccSaveData.parodyFlag` - the next one is `voice`. So the twins are
Parody Mode's text, not a language switch.

In the US build the plain tables are English and every parody table is
Japanese Shift-JIS. Mail 0 is `Power of the Bracelet` / `腕輪の力`, mail 5
`Thank You` / `登録完了のお知らせ`, and the Japanese bodies I read are
ordinary translations of the English ones, not jokes; whether other parody
entries differ in content cannot be told from this disc, which has no
Japanese normal-mode text to compare with. The US text path sends Shift-JIS
through `ccGetExtendedCode` and draws `_` for anything it cannot map, so
Parody Mode in the US Infection would presumably show mostly underscores.
That is an inference from the code; nobody has set the flag and looked.

## Mail replies only exist after start-up

In the executable, `MailTbl[n].oneRes` and `.twoRes` - the two replies the
player can send - are mostly zero. `__sinit_mailtbl.cpp`
(`INF desktop.prg:0x00461c30`, 16,472 bytes of straight-line code) copies
`ReMail[k]` into them field by field, moving each word through an FPU
register (`lwc1`/`swc1`), and `mwLoadOverlay` runs it through
`__initialize_cpp_rts` when the overlay loads ([[4]]).

Rather than hand-map 294 copies, `tools/eemu.py` is a small EE interpreter -
128-bit registers, integer ALU, every load and store width including `lq`/`sq`
and `lwc1`/`swc1`, branches with delay slots and branch-likely annulling,
calls - that runs an overlay's constructor table over a copy of memory, the
way `__initialize_cpp_rts` does. Anything it does not interpret stops the run
with the pc. It runs every constructor in all four overlays without stopping.
What changes:

| overlay | bytes changed | where |
| --- | ---: | --- |
| desktop | 6,369 | `MailTbl` 2,923, `MailTblp` 2,914, `InfoMsg` 267, `StrMsg` 152, `ResetMsg` 112 |
| gcmn | 380 | `menuList` 267, `ccSpcManager` 88, a few statics |
| demo | 124 | `InfoMsg`, `StrMsg`, `ResetMsg` |
| toppage | 0 | |

After that, 147 of the 326 mails carry two replies (`Thanks` / `There's no
need.` on mail 8, for instance).

## The escapes

`ccKanji::Disp` (`INF SLUS_202.67:0x0015e380`, `font.cpp`) expands a string
into an 82-byte buffer in a first pass and draws it in a second:

| bytes | meaning |
| --- | --- |
| `#0` | the text of `ccSaveData.plName` (+0x00, 24 bytes): the character's name, Kite by default |
| `#1` | the text of `ccSaveData.plRealName` (+0x18): the player's own name, as typed into the desktop |
| `#R` `#G` `#B` `#Y` | colour from `ccSpriteColorTable` +0x90, +0xa0, +0x88, +0x30: (128,56,56), (72,128,72), (48,80,128), (128,128,0) in GS units |
| `#W` | back to the colour the string started with |
| `%` + byte | copied through as a two-byte extended font code |
| 0x20-0x7f | ASCII |
| a Shift-JIS pair | `ccGetExtendedCode`; kept as `%`+code when it maps, drawn as `_` when not |

In the English mail `#0` appears 322 times and `#1` never; colours are mostly
`#Y` (185) and `#B` (104, the area addresses: `#BΔ Boundless Corrupted Fort
Walls`). The `Δ` is Shift-JIS `83 a2`, so even the English text relies on the
extended-code path for it. The deliberately corrupted mails (`Ple%:*(` and
the sender `Au]$`) contain `#` and `%` as noise, which the renderer will read
as escapes like any other.

## What is in them

Mail senders in the English table, top of the list: BlackRose 45, Gardenia
25, Sanjuro 24, Piros 24, Natsume 23, Wiseman 22, Mistral 21, Moonstone 19,
Rachel 18, Marlo 18, Nuke Usagimaru 17, Balmung 12, Terajima Ryoko 12,
`Au]$` 10, CC Corporation 8, Helba 6, Lios 6, `ROY@BANDAI` 3. Several of those
characters belong to later volumes, so the table may carry mail for more than
Infection; which entries Infection can actually send is not checked.

The news pages are images: each `HtmlData` names a CCS file and texture
(`xddn_010::TEX_xddn_010` for "New Transportation System Operational"), and
only the headline is text.

**Still unknown:** the meaning of `mailFlg` and `reFlg`; how the game decides
which mails a volume delivers; what `ccGetExtendedCode` maps and how the `%`
codes are drawn; the BBS `dateindex`; whether Parody Mode can be switched on
in the US Infection at all.
