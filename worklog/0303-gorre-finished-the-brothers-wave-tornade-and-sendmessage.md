---
number: 303
title: Gorre finished: the brothers' Wave, Tornade and SendMessage ported, the harness at 0 mismatches, event 218 done
date: 2026-09-28
area: battle
files: crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss/gorre/message.rs, crates/piney-battle/src/boss/gorre/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/combat.rs, crates/piney-data/src/pack.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/session/tests/survey.rs, tools/test_battle_gorre_rs.py, docs/engine/boss-gorre.md, plans/outbreak-story.md
---

# 303. Gorre finished: the brothers' Wave, Tornade and SendMessage ported, the harness at 0 mismatches, event 218 done

Gorre ([[296]], [[297]]) went into this session with its harness failing
every case (`bulk 20` from 9900: 20 of 20, all near frame 91). Everything
was re-read from Outbreak's own listing this time (`tools/disasm.py fn
work/outbreak/disc/SLUS_205.63 ... --overlay gcmn`): the doc's "OUT"
addresses had been Infection's, and Outbreak's code is not Infection's
(`SendMessage` is 3108 bytes at OUT gcmn 0x004adbb0, not 2984).

## What the game does

**Gorre is a controller.** It is never on the command lists (no
`ccEntryCmnd` in `ccBoss05::ccBoss05`, OUT gcmn 0x004a9130), its `Affect`
(0x004aa200) is empty, and its `Main` (0x004aa210) never calls
`PreDrawAnm` or `Draw` (nor does its task, `ccThBoss05`, 0x004a9050):
its body is animated and drawn only in the Kerse's step 2, at
`m_talkPos`. The two brothers are the fight.

**`SendMessage` (OUT gcmn 0x004adbb0, jump table `@2268` 0x006dfe10).**
Msg 0 is the brothers' `Affect` (0x004b07c0) report, by the brother's
affect type: a hit takes the three's one HP (under `cheatHP` a tenth,
never under half, and the broken protect shared: the struck brother's
count is Gorre's, a fresh break of 300 breaks the other's, its gauge
Gorre's and the other's); at 0 `OffBossCamera` (0x004700a0), the party
held, the music out, Gorre's Dead. 7 and 9 heal the three, 13 is the
Drain (`ChangeAction(11, 2)`, `cheatHP` off), 21 gives both brothers row
4's stats and Gorre's own `param` at 6000 HP. Msg 2 (the Wave home), 4
(a death), 5, 6, 7 (the Tornade faded, over, 120 frames in) act once both
brothers' `m_sendMsgFlag` are set; msg 3 (a held brother) holds the
other; msg 1 has a case and no sender. The death ends the fight: Gorre's
Dead only places the brothers and the camera; each brother's dead effect,
once out of the manager (`IsValidAdrs`, one pass after it disables),
sends msg 4, and Gorre's `OnExit` follows.

**The brothers' acts** (Think 0x004af020 has cases for 0, 4, 12, 13, 14,
21 only: in any other act, Gorre's own ordered along, a brother does
nothing; the old port orbited, which was the frame-91 drift). The Wave
(0x004afe10) and the Epitaph's (0x004af300): out past 500, 30 still, half
a turn at 5 degrees a frame from `m_prevPosB`, the strike clip with the
wave (`anmw`) drawn from its frame 40, skill 18 at 55, home within 300 and
msg 2. The Tornade (0x004af8d0): fade out (msg 5), in, then 211 frames of
a quickening turn (msg 7 at 120, msg 6 after); msg 6's flags are never
cleared, so the next Tornade ends at the first brother's report. Below
frame 40 of the strike, `OnThinkWave` tests a register it never set there
(`$s0`, the caller's `this`): the clip's end alone ends the step.

**Gorre's acts, all re-read**: the Kerse is seven steps (`@1644`,
0x006dfdf0) with the brothers fading out, Gorre's clip and fade into the
`FinalPhotonFlash`, the second brother's skill 19, and `OffDraw` of Gorre
at its end; the Talk's skill 20 is the second brother's; the Skill takes
`@1777` (157, 158 the first brother's), the Magic `@1672`, the Tornade's
spell `@2074`; pattern 10 skips its word (no rand); pattern 9's pick is a
member standing, not one down; the base's `ChangeAction` is Gorre's
through the vtable (the brothers ordered on every base pattern). Chase,
Escape, Wander, Dash and Data Drain were each off in several places.

**The tables** hold pattern 10's operand as a -1, so reading "up to the
first -1" cut all three; and Outbreak's Super table runs four words past
its symbol's 188 bytes (Infection's). `piney-gen`'s manifest now reads
each through its closing -1 as patterns and operands
(`gorre_patterns`); a Gorre run off the end of its cut table sat in its
neutral for good (`word` 0 past the end, the base's neutral, the index on
by one each frame).

**`MaxRenge`**: `ccBossCam::ccBossCam` (0x00472090) sets 1000.0;
Fidchell's constructor alone sets its 600 after `InitBossCamera`
(`sw 0x4416_0000, 216($v1)` at 0x004a3aa0); Gorre's does not. Gorre's
port no longer sends `CamMaxRange`; the harness's stand-in camera now
writes the constructor's 1000 and compares it.

## The port and the checks

`gorre/brother.rs` rewritten (acts as `Msg` and `Wave` enums, the
brother's members, `Move` with `m_posB`'s own step), `gorre/message.rs`
new (`SendMessage`), `gorre.rs`'s acts re-ported; `Effects::valid` for
`IsValidAdrs`; `Out::BossCamOff` (the world's `boss_cam_off`); the world
draws Gorre's body only in the Kerse (`gorre::body`). Data version 19.
The harness now scripts affects on both brothers (hits, drains, heals,
the 21), compares the lists, `MaxRenge` and the gauges through each
character's `param` pointer (the 21 moves the brothers onto Gorre's),
hooks `effResistantShield`, and ignores an effect's manager slot in the
calls (one array for three in the game). `bulk 20` from 9900: 0 of 20;
`bulk 100` from 9900, 20000, 40000: 0; its own tests pass;
`cargo test -p piney-battle --lib` 110.

## Event 218

The field-5 menu loop was Gorre's: the cut tables left Gorre frozen in
its neutral, and with Gorre unlisted the pilot never saw a drainable
foe (its candidates were the listed foes). Now Data Drain's candidates
take in Gorre's brothers, and the pilot fights the brother whose gauge a
physical hit fills (row 41's physical PP defence 2000, row 40's 9990).
`PINEY_SURVEY_ONLY=218 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000
cargo test --release -p piney-game outbreak_story_survey -- --ignored
--nocapture`: `story 218: done` (the gauge broken near frame 98,000, a
brother drained, the three down near 107,000).

**Still unknown:** `effResistantShield`'s vector tweak on the brothers'
shield (shown, not shaped); after msg 0's 21 the port keeps three gauges
where the game shares Gorre's one (equal, and unchanged in the fight);
the `FinalPhotonFlash`'s picture.
