---
number: 246
title: What the save's open fields hold
date: 2026-09-27
area: save
files: docs/formats/save.md
---

# 246. What the save's open fields hold

Asked for: more of the docs' open questions. `docs/formats/save.md` is now
solid, with a section on what its open fields hold.

## Already on other pages

Most of the page's questions had answers on the engine pages:
- `eventFlag` bits (events);
- `eventStatus` (the status ops);
- `partyMemberCall` (the `call_*` ops, and the Members menu, which
  refuses a clear bit);
- `townMoveFlag` (Other Servers);
- `drainDemo` (its option menu);
- `growth` (the Grunties);
- the Time Idol ranks (the field objects and the board).

Two Init values needed a lookup:
- **`tactics` 7** is Operation Wonder Battle. The field holds the CHAT
  order 7-10, and `ccSpcSetOperation` maps it to `partyStrategy` 0-3.
- **`partyMemberCall` 0x3fffe** means every member (bits 1-17) answers a
  call.

## Found here

- **`eventEntry[160][6]`.** A bit per event-placed gimmick of each field
  (`game.field`, 1-160, up to 192 entries).
  - `CheckEventEntry` tests the bit.
  - `ClearEventEntry` clears it, through `ccEntryCtrl::deleteEventEntry`,
    when a box is opened, an object broken, a Virus Core taken or an idol
    used.
- **`hyProccess[8][4]`.** Each Ryu Book's payout level:
  - `BOOK::GetBookItem` runs the book's `GetBookNNItem` per level;
  - each gives a reward only above the stored level, and stores the level
    once the item is added.

## Never read

A scan of every instruction in the executables and overlays, for the
offsets as immediates next to a `lui 1` and through the extension's
pointer, found the same on Mutation, Outbreak and Quarantine:
- the blocks at +0x8432 and +0x8462: zeroed by `Init`, copied by
  `ccSaveSys::MainProccess`, nothing else;
- the extension's +0x754: cleared and copied, nothing else.

The page's note that gcmn reads +0x845e had no source and the scan finds no
such read, so it is gone.

**Still unknown:** Nothing on the page. What each `eventStatus` index
counts belongs to the scripts that use it.
