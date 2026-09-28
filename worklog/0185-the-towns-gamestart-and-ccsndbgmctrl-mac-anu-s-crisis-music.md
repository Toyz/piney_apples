---
number: 185
title: "The towns' gameStart and ccSndBgmCtrl: Mac Anu's crisis music and Dun Loireag's second sequence"
date: 2026-09-26
area: audio
files: crates/piney-world/src/lib.rs, crates/piney-game/src/world.rs, docs/engine/sound.md
---

# 185. The towns' gameStart and ccSndBgmCtrl: Mac Anu's crisis music and Dun Loireag's second sequence

Wiring the scene sounds (0184) into the towns showed that the port's
towns never told the sound driver the game had started. `ccSetupGameCtrl`
sets `ccSnd.gameStart` just before the fade in (0x001694ec) for a town as
for a field; the town mode had no such request. Without it the sound
task in a town ran `ccSceneFade` instead of `ccFade` and `bgmChange`, and
`ccSoundMain`'s scene sounds (Mac Anu's canals) would never run.

The town's `ccSndBgmCtrl` was also its own shortcut: `ccSqPlay(2)` in
the crisis, else `ccSqPlay(0)`. The game's town case (0x0017b064, read
again) does this:

- In Mac Anu (town type 0) it sets +0x105 = 1 and +0x132 = 0, then starts
  sequence 2 when the crisis byte is 1 and the bank has three sequences.
- In types 1 and 3 it starts sequence 2 whenever the bank has three.
- In every town it then starts sequence 0 (0x0017b354).

So the shortcut missed sequence 0 under Mac Anu's crisis music, and Dun
Loireag's sequence 2 altogether. `sound.md` and the driver's `bgm_plan`
already said so. Only the town mode did not go through them.

**Fix.**

- piney-world's `Request` gained `GameStart`, which the town asks for
  with its bank and `enableReset` as the black hold ends.
- piney-game's town maps it to `Event::GameStart`, and `BgmCtrl` to
  `Event::BgmCtrl` with the town (`lastTown`), the crisis byte being 1,
  and `dtBgm`, so `Driver::bgm_ctrl` carries out `bgm_plan`'s town case.
  That case also resets +0x105 and the canals' flag (0184).
- `mac_anu_starts_its_sound`: `GameStart`, then the town's `BgmCtrl`
  (town 0, no crisis), and the scene sounds' inputs every frame.

**Still unknown:** Nothing new; Dun Loireag's sequence 2 was not listened
to.
