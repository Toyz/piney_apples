---
number: 344
title: "sound 8 0 256: five uses, and port 0 always takes the SE option"
date: 2026-10-01
area: audio, script
files: docs/engine/sound.md
---

# 344. sound 8 0 256: five uses, and port 0 always takes the SE option

[[176]] left open "whether the SE port's volume is ever below full when
`sound 8 0 256` runs". The answer does not change what the instruction
does. `ccPortVolSet(port, vol)` (INF main 0x0017ad70) gives port 0 `seVol`
whatever `vol` is, so `sound 8 0 256` puts the SE port back to the
player's SE option from any value. The driver's `port_vol_set`
(`crates/piney-audio/src/driver.rs`) does the same (`0 => se_vol`).

The uses, from `tools/evscript.py dump` of each volume's executable:

| volume | tables with `sound 8` | all |
| --- | --- | --- |
| INF | M110, M131, M216, M319, M414 | `8 0 256` |
| MUT | the same five | `8 0 256` |
| OUT | the same five, and S410 | `8 0 256` |
| QUA | the same five, and S410 | `8 0 256` |

M110's is the first instruction of its block 0. M131, M216, M319 and M414
each run it right after their ending streams and `overlay 1`. S410 (Mia's
event 360) runs it after stream 138 and `call_on 1`. All but M110's
follow a stream. What lowers port 0 before it would be one of
`ccPortVolSet`'s 47 calls (`ccSndChangeOption`,
`ccSqPlay`, `ccSqPlayVol`, `ccSound::ccFade`, `ccSceneFade`,
`ccSndSQLoad`, `ccSndChangeData`). None of those calls was followed to
port 0 here, since each lands on `seVol` the same way.

`sound.md` now lists the six places.

**Still unknown:** nothing.
