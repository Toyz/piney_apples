---
number: 335
title: "vBank+0x08 is never read: SEWORDS' wordPlay takes the offset, size, volume and name"
date: 2026-10-01
area: audio, iop
files: docs/formats/voice.md
---

# 335. vBank+0x08 is never read: SEWORDS' wordPlay takes the offset, size, volume and name

[[95]] noted that `skillVoicePlay` leaves `vBank+0x08` as `evVoicePlay` last
wrote it (0), while the port sends no such word. Whether that word matters
depends on the IOP side.

`evVoicePlay` (INF SLUS_202.67:0x0017eca0) sends `vBank` as command 0x80e0
through `sewordCmd`. Its fields are `ofs`, `siz`, 0, 0x6fff6fff and the file
name. SEWORDS.IRX's `bgmFunc` hands the RPC buffer for 0x80e0 straight to
`wordPlay` (SEWORDS 0x05a4), which reads:
- +0x00, the offset (`lseek`, 0x3108);
- +0x04, the size (0x30d8);
- +0x0c, the volume (`BgmSetVolume`, 0x3384);
- +0x10, the name (`open` and the debug prints).

No load of +0x08 is made from any of `wordPlay`'s eight copies of the
pointer. The word is never read, so the port's not sending it changes
nothing. [the voice page](../docs/formats/voice.md) says so.

**Still unknown:** nothing about +0x08. The rest of [[95]]'s list is
unchanged.
