---
title: The desktop text - mail, board, news
status: partial
volumes: INF
covers: INF desktop.prg MailTbl, MailTblp, ReMail, ReMailp, HtmlTbl; INF toppage.prg bbsThreadTbl, bbsThreadTblP, bbsMsgTbl, bbsMsgTblP; INF SLUS_202.67:0x0015e380 ccKanji::Disp
worklog: 9, 18, 23, 37, 58
---

# The desktop text - mail, board, news

The in-game desktop's mail, bulletin board and news headlines are C tables
compiled into the `desktop.prg` and `toppage.prg` [overlays](../formats/prg.md).
Every table has a Parody Mode twin, used when `ccSaveData.parodyFlag`
(+0x842b) is non-zero.

## Tables

All little-endian; pointers are VAs in the overlay.

```
ccMailData          0x48 bytes     MailTbl 0x0041dfd0, MailTblp 0x00425280
  +0x00  s32       NO              mail number, equals the index
  +0x04  s32       reFlg
  +0x08  s32       mailFlg
  +0x0c  char *    title
  +0x10  char *    from
  +0x14  s32       fromNO          sender id: the photo CMP_xddphot{fromNO + 1}
  +0x18  s32       line            lines in the body
  +0x1c  char *    sentence        `line` NUL-terminated strings, back to back
  +0x20  ccReMailData oneRes       first reply the player can send
  +0x34  ccReMailData twoRes       second reply

ccReMailData        0x14 bytes     ReMail 0x0041c8d0 [294], ReMailp 0x00423b80 [294]
  +0x00  s32       mailFlg
  +0x04  char *    title
  +0x08  char *    from
  +0x0c  s32       line
  +0x10  char *    sentence

ccBBSThreadList     0x0c bytes     bbsThreadTbl 0x00406450, bbsThreadTblP 0x00408200
  +0x00  char *    title
  +0x04  ccBBSMsgList * msgList
  +0x08  s32       msgNum

ccBBSMsgList        0x14 bytes     bbsMsgTbl 0x004049a0 [341], bbsMsgTblP 0x00406750 [341]
  +0x00  s32       dateindex       the date cell, 0-5 plain, 0 in parody
  +0x04  char *    title
  +0x08  char *    transname       the poster
  +0x0c  s32       maxLines
  +0x10  char *    message         `maxLines` strings, back to back

HtmlData            0x1c bytes     HtmlTbl 0x0041c120 [70]
  +0x00  s32       No
  +0x04  s32       flg
  +0x08  char *    title           the headline
  +0x0c  char *    ccsName         CCS file holding the page image
  +0x10  char *    chunk           texture name in it
  +0x14  ccFileList * html
  +0x18  s32       hight           page height in pixels (spelled so)
```

`MailTbl` and `MailTblp` hold 326 entries each; `bbsThreadTbl` 63 threads over
all 341 posts. How the board shows them, and which the save posts, is on
[the top page](toppage.md#the-board).

## Start-up copies

`oneRes` and `twoRes` are filled at load time: `__sinit_mailtbl.cpp`
(`0x00461c30`) and `__sinit_mailtblp.cpp` copy `ReMail[k]` / `ReMailp[k]`
into them, and `mwLoadOverlay` runs them. Read the tables after running the
overlay's constructor table (`tools/eemu.py`), not from the file. 147 mails
have both replies set afterwards.

## Text encoding and escapes

Strings are single-byte ASCII mixed with Shift-JIS pairs. `ccKanji::Disp`
(`0x0015e380`) handles:

| bytes | meaning |
| --- | --- |
| `#0` | `ccSaveData.plName` (+0x00): the player character's name |
| `#1` | `ccSaveData.plRealName` (+0x18): the player's own name |
| `#R` | colour `ccSpriteColorTable`+0x90, (128, 56, 56) |
| `#G` | colour +0xa0, (72, 128, 72) |
| `#B` | colour +0x88, (48, 80, 128) |
| `#Y` | colour +0x30, (128, 128, 0) |
| `#W` | the string's starting colour |
| `%` + byte | an extended font code, passed through to the draw pass |
| 0x20-0x7f | ASCII |
| Shift-JIS pair | `ccGetExtendedCode` (what it maps, and how `%` codes are drawn: [font](font.md)); if the result's high byte is `%` it becomes that two-byte code, otherwise `_` |

Colours are GS values, 0x80 = full intensity, alpha 0x80.

## Notes

In the US build the plain tables are English and the parody tables Japanese.
The parody tables are a separately written comedy script, not a Japanese copy
of the normal one: event dialogue differs in content (worklog 18), though the
mail entries compared so far agree.
The English area addresses still use the Shift-JIS `Δ` (`83 a2`).

Corrupted-text mails use `#` and `%` as noise characters; the renderer treats
them as escapes regardless.

## Unknown

- `reFlg`, `mailFlg` (their writes are ported: [desktop](desktop.md)),
  `HtmlData.flg`.
- Which mails and posts the game actually delivers in Infection.
