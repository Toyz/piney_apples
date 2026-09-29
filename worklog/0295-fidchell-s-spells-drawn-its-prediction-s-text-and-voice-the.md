---
number: 295
title: Fidchell's spells drawn, its prediction's text and voice, the skills' names
date: 2026-09-28
area: render, audio, battle, volumes
files: crates/piney-effect/src/fidchell.rs, crates/piney-effect/src/boss.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/examples/boss_probe.rs, crates/piney-battle/src/boss/fidchell/eff.rs, crates/piney-battle/src/boss/fidchell.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/field_world.rs, crates/piney-desktop/src/anm.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/fidchell.rs, crates/piney-gen/src/manifest.rs, tools/test_boss_effect_rs.py, docs/engine/boss-fidchell.md, docs/engine/boss.md
---

# 295. Fidchell's spells drawn, its prediction's text and voice, the skills' names

What [[294]] left named but not shown now shows and sounds: the four
spells' pictures, the prediction's text and voice, a skill's name in the
cinema, and a boss's `effSkillStart`. The reference is
[docs/engine/boss-fidchell.md](../docs/engine/boss-fidchell.md) ("The
spells' pictures", "The prediction's text, its voice and the skills'
names").

## One simulation, two drivers

The spells' rules draw every `ccRand` (the Fidchell harness runs them
natively and counts it), so the pictures must not draw again. The rules
(`fidchell::eff`) now keep what each `Draw` drew: a meteor moved this
`Draw`, a bolt's ten points, how many segments it drew, their
transparency and each sprite's pattern draw, a tower's place and
transparency before it moved. They run on a small `eff::Host` (`ccRand`,
the frame, `ccGetDist`, the calls). In the fight the boss's `Cx` is that
host and piney-effect's new `fidchell` draws each spell from the rules'
state after the boss's task (`BossEffects::sync_spells`). For the
harness, piney-effect's manager runs the same rules itself on its own
generators (`Make::MeteoSworm` etc.), making the calls beside in the
game's order. The pictures are the game's: a meteor is particle's
`ANM_x300` and the glow `CMP_x100` turning pi/30 a `Draw`; a segment is
`CMP_x012` stretched to its length over 900 and laid along it, then an
`EFF_x011` sprite; a tower is `CMP_x101b` ("abcd"[1] in its
constructor: always b). The swarm's light is an omni light at the last
meteor, full to nothing at 500.

## ccSetColor has an HSV half

Outbreak's disc failed Skeith's own magic square in the boss effect
harness (frame 75, the light's colour 0.984 grey against the port's
black). Outbreak's `ccBossEffLight::Draw` turns its count into an
unsigned with an inline `cvt.w.s` where Infection and Mutation call
`fptoui` (0 for a negative): a count fallen below 0 keeps its high bits,
and `ccSetColor` (OUT main 0x00136f90) reads a colour with an alpha byte
as HSV (hue in sixths of the low byte, saturation, value). 0xfffb0000 is
value 251, saturation 0: grey. The meteors' light, 0x80ffffff, is hue
sector 5 at full value: red with 250/256 of the way to magenta, blue
0.023. Both are ported (`boss::set_color`, `to_uint` by volume), and the
light's modes 1 (the thunders' square: a `ccRandF(0.5)` tilt drawn and
lost, grey and orange by turns, its light moved one `Draw` in three) and 2
(the meteors': a tilt of -1.5393804, the turn falling behind after 15,
pulsing red), with magic squares 1 and 2 (their SEs by `game.field`,
rows 2 and 6, 5).

## The text is on the screen

`m_predTxt`'s `pos` and `dirc` are written in step 2 and never read
anywhere in boss04.cpp: `ccAnm::Draw` places `ANM_ex41txtN` by the
animation's own `F_Camera` records, on layer 254 (sysLayer's view) with
`SetRenderState(CCRS_ZENABLE, 0)`. The world draws it with the desktop's
`ccAnm` (`zenable` added) set when `Pic::Text` first comes. The voice is
`ccEvVoiceRequest(-40, n)`, which the audio driver already routes to the
field voice groups (-40 is Fidchell's).

## A skill's name

OUT gcmn 0x00472550 is a switch on `game.field` (+0x24) and the skill:
fields 2, 4, 5, 6, 7 and 8 each have a table of `CINEMASKILLNAME_T` rows
(field 4: 157, 158, 160, 203, 219, 227, 259, rows 8-14 of x41's
`TEX_fid_skl`). piney-gen finds it as the first call of the function after
`CinemaOn__19ccBossEffCinemaFadeFi` (Outbreak's `CinemaOn` with a skill;
on Infection and Mutation `CinemaOff` follows, so nothing) and runs it in
eemu for fields 0-31 and skills under 512: `cinema_skill_rows` in the
combat group (data version 17). `SetupSkillName` is Infection's, with its
`x01`-`x04` for fields 9-12.

## Checks

`tools/test_boss_effect_rs.py` now hooks `genrand` (behind `ccRand` and
`ccRandF`) with a 32-bit LCG on both sides instead of a constant `ccRand`,
and `ccSeOn`; it builds no particle control, so the probe has no free
particle either (the tower's `effSmoke` first differed there: the port's
particle took a pattern `rand()` the game's never reached).
`test_fidchells_spells` (Outbreak): meteo 192, storm 312, tower 436,
square 1 288 (fields 4 and 2), square 2 184 frames agree on every draw,
sound, generator, light, slot, `rand` and `genrand` count. The Skeith
cases pass on Outbreak and Infection. The Skeith, Innis, Magus, Kyvia and
Fidchell battle harnesses pass. With the pictures' own `rand()` draws now
made in the field, the story pilot still ends Outbreak's 207 (120,000
frames) and Mutation's 108 (150,000, the gomoras' `effSkillStart`) done.

`session::tests::fidchell::fidchell_spells_show` brings event 207 to block
9 (area 73's `WORLD_MAN`, field 4 of town 3) and forces the prediction,
each spell and a skill from act 0: the text shows 420 frames, voice (-40,
1) and (-40, 0) are asked, every spell's objects are drawn (`ANM_x300`,
`CMP_x100`, `CMP_x012`, `EFF_x011`, `CMP_x101b`, `CMP_x201d`, `CMP_x033`),
`effSkillStart` makes its rings at Fidchell, and the skill's cinema has
its name. `fidchell_shots` wrote the frames to
/mnt/data/claude/scratch/fidchell-fx: the text mid-screen under "Oracle:
Ice Storm", the meteors falling and landing round Kite, the zig-zag bolts
from the sky, the towers rising round him seen from above, the ice's
photons, the skill's name. The first run forced acts in the middle of
others (camera 3 left on from the prediction), which put the tower camera
inside the rocks; forcing only from act 0 does not.

**Still unknown:**
- In the fight a storm's magic square is made after the rules' pass, so
  its first `Draw` is a frame late, and a picture's own `ccRand` (a mode
  1 light's) comes in the effects' pass before the boss's, not in slot
  order: the field's `genrand` order differs from the game's there.
- The prediction's text and the skills' names are not run against the
  game's own drawing frame by frame; `effSkillStart`'s end with the boss
  is still the rules' guess of [[294]].
- Quarantine's `ccBossEffLight` and `ccSetColor` are taken to be
  Outbreak's (its `cvt.w.s` is seen); unchecked.
