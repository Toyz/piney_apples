---
number: 1
title: The Infection disc and what is on it
date: 2026-09-22
area: disc, content, tooling
files: tools/iso.py, docs/disc/layout.md
---

# 1. The Infection disc and what is on it

The four games came as 7z archives in `originals/`, one ISO each. Infection's
is `Dot Hack Part 1 - Infection (USA) (En,Ja).iso`, 2,598,567,936 bytes, which
is exactly the 1,268,832 sectors the primary volume descriptor claims. It is
extracted to `work/infection/infection.iso`. 7z was used only to open the
archive; everything after that goes through our own reader.

`tools/iso.py` reads ISO 9660 directly: the primary volume descriptor at sector
16, then the directory records from the root. The disc is plain ISO 9660 with
2048-byte sectors - no Joliet, no Rock Ridge, system identifier `PLAYSTATION`,
an empty volume identifier. It lists 86 files, and when sorted by LBA they are
packed with no gaps from LBA 370 to LBA 1268821. `iso.py where IMAGE LBA` maps a
sector back to a file, which matters because the game may address data by
sector rather than by name.

`SYSTEM.CNF` is 57 bytes:

```
BOOT2 = cdrom0:\SLUS_202.67;1
VER = 1.00
VMODE = NTSC
```

By directory:

| dir | files | bytes | what |
| --- | ---: | ---: | --- |
| `/` | 4 | 20,525,222 | `SYSTEM.CNF`, the EE executable `SLUS_202.67` (19,223,388), `IOPRP243.IMG`, `OUTSIDE.BIN` |
| `DATA/` | 7 | 183,200,768 | four overlays `*.PRG`, `ICON.BIN`, `DATA.BIN` (136,665,088), `SNDDATA.BIN` |
| `MODULES/` | 12 | 447,630 | IOP modules (`*.IRX`) |
| `PSS/` | 4 | 87,769,104 | MPEG-2 movies: three logos and `OPENING.PSS` |
| `STREAM/` | 4 | 880,826,368 | `STR1`, `STRCMN`, each with an `E` twin |
| `VOICE/` | 28 | 764,319,744 | Japanese voice banks, plus `BGM.BIN` |
| `VOICE_E/` | 27 | 660,668,416 | English voice banks - the same set without `BGM.BIN` |

The US disc still carries the complete Japanese voice set alongside the English
one, which is why the dump is tagged `(En,Ja)`. `BGM.BIN` (63,014,912 bytes)
lives in `VOICE/` and has no English twin, so music is shared. The `STREAM/`
pairs split the same way: `STR1.BIN` / `STR1E.BIN` and `STRCMN.BIN` /
`STRCMNE.BIN`.

`OUTSIDE.BIN` is the last file on the disc, 1,048,576 bytes, and every byte of
it is zero. The name and the position suggest padding placed to push the real
data outward, but nothing in the code has been checked for a reference to it.

The EE executable is 19 MB because it was never stripped - that is the subject
of [[2]]. The full listing with LBAs is in
[the disc layout page](../docs/disc/layout.md).

**Still unknown:** the formats of `SNDDATA.BIN`, `ICON.BIN`, the voice banks,
`BGM.BIN` and the `STREAM/` files; whether `OUTSIDE.BIN` is read by anything;
what `IOPRP243.IMG` replaces in the IOP's ROM modules.
