---
number: 206
title: "The carry's votes checked against content: a name whose table reads like Infection's stays"
date: 2026-09-26
area: volumes
files: tools/main_data.py, crates/piney-data/src/main_data
---

# 206. The carry's votes checked against content: a name whose table reads like Infection's stays

Since 0202 a unanimous vote from paired code outranked a name
`tools/xfer.py` carried: "the code wins". That was right for Mutation's
`skillTbl` and `MailTbl`. A look at the others printed for the three
later volumes found most of them wrong: the tracker had paired a `lui`
with the wrong low half, or the functions only partly aligned.

| global | named (right) | built (wrong) |
| --- | --- | --- |
| `typeAplayType`, `typeCplayType` | Infection's bytes | another play-type table's |
| `enemyCCSTbl` | its CCS names | 0x11111111 words |
| `str0305TblE` | `str0305E` | `Confused` (a word table) |
| `itemShopItemList` | the shop's lists | zeros |
| `bullMessages` | its lines | the next table's |
| `BGTBL`, `D0891_room` | Infection's layout | `BGTBL2`, an item list |

Several of these override places were the same address on two or three
discs, which no real move would give.

A vote now overrides a name only when it scores at least as high. The
score is how many of the global's words read as Infection's at each place,
over its first 0x400 bytes: a pointer (Infection's word relocations)
compares by the bytes it points at, and a pointer to untexted data counts
for neither. A table the later volumes changed a little still reads more
like Infection's at its own place.

The overrides left:

- `skillTbl`, `MailTbl`: checked right before;
- `idolItemList44`: neither place reads like Infection's;
- Outbreak's `mt`: zeros at both.

The sequence pass (0205) applies the same test to the labels it moves.

The misses the source still names: Mutation 5, Outbreak 23, Quarantine
28. All three later discs still go through New Game into the desktop
with no error printed.

**Still unknown:** Where Mutation's and Outbreak's `idolItemList44` is.
The remaining misses.
