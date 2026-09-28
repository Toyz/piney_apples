---
number: 184
title: "The scenes' own sounds: Mac Anu's canals, area 15's church, the breeder's tune; and the camera's wiped-out party"
date: 2026-09-26
area: audio
files: crates/piney-audio/src/scene.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-world/src/camera.rs, docs/engine/sound.md, docs/engine/field-game.md
---

# 184. The scenes' own sounds: Mac Anu's canals, area 15's church, the breeder's tune; and the camera's wiped-out party

Two items from the docs' Unknown lists that turned out to be gaps.

## The scenes' own sounds

`sound.md` said `ccSndChangeOption`'s field cases were "read, not run,
and not ported", and did not know what `ccSnd` +0x108..+0x114 and +0x132
do. Reading the option cases led to the writers of +0x108 and +0x10c:
`bgmChurch` and `bgmBreed`. Both are called only from `ccSoundMain`
(0x00181010), the sound thread's loop, beside a third, `waterTest`.
Nothing of the three was ported, and none was in the docs.

After each frame, while `gameStart` (+0x17) is set, the loop switches on
+0x105:

- **1, Mac Anu: `waterTest`, the canals** (`seData` row 42).
  - It starts once, silent, in a free loop slot, as TOBJ's hum starts.
  - Then each frame it sends pan 60 and a volume that follows the
    camera's distance from the canal line (y 0): full within 1700, fading
    over the row's decay.
  - Two zones hold it at 48 at least: west of x -3000, and the south-west.
  - +0x132 marks it started; `ccSndBgmCtrl` clears it in Mac Anu.
- **2, area 15's bank: `bgmChurch`** (the Hidden Forbidden Holy Ground).
  - Sequence 0 starts silent.
  - Outside, port 1 follows the camera's distance to y 3500: louder as the
    church comes near, silent 3500 away.
  - Walking in (block 1) switches to sequence 1 at full, fading 0 out over
    30 frames; walking out does the reverse. +0x108 and +0x10c keep the
    state.
- **3, another town: `bgmBreed`** (Dun Loireag's Grunty breeder).
  - Kite within 1000 of `DMY_merchant6` fades the town's tune out and the
    breeder's in; beyond 1200 back.
  - +0x114 and +0x120 cache the breeder's position.

And `ccSndChangeOption`, in The World, leaves those scenes' ports alone:

| scene | ports a volume change sets |
| --- | --- |
| area 15 | port 2 only; ports 1 and 2 inside the church |
| Dun Loireag | ports 1 and 3; 2 and 3 while the breeder's tune plays |
| anywhere else | all three |

**The port.** `piney_audio::scene`:

- `SceneInput` holds the camera, Kite, `game.block`, Mac Anu, and the
  breeder's dummy.
- `scene_sound` holds the three functions.
- The pieces the driver already had are reused: `sq_fade` equals their
  inline fades, `sq_play_vol`, and the hum's loop start.
- The driver gained `water_loop`, `scene_bgm`, `church_block`, `breeder`
  and `in_world` (`game.status` 5: set on the area's arrival, cleared by
  `gameInterrupt`), and `change_option` gained the cases.
- The town and area modes send `Event::SceneSound` every frame.

Unit tests:

- `the_canals`: started silent once, then pan 60, loud near the canals,
  silent far away, nothing outside Mac Anu.
- `the_church`: silent out of reach, 255 at the church (100 * 2.56 in the
  EE's floats), then the switch in and out with the 30-frame fades.
- `the_breeder`: the tune in within 1000, staying up to 1200, out beyond.

## The camera with the party wiped out

`field-game.md` listed `cameraMain`'s party-annihilation branch as not
ported. It is five instructions at 0x00160cd0:

- With `checkPartyAnnihilation`, camera 1 in the eye view goes back to
  following, and the buttons are read that frame.
- Any other camera skips L2 and the reset button; only the reset's turn
  (and the shock absorber) runs.

`Camera::main` takes the party's state; the field passes
`combat.annihilated()`. `a_wiped_out_party_follows` checks both paths.

**Still unknown:** The canals' and the church's volumes were not compared
with the game playing, only derived from the code. What +0x110 is.
