---
number: 416
title: Event 155's weapon flashes behind the camera stay off Kite: a ray's outer pair kicks only where makePacket would, in front of the eye and on the screen (#66)
date: 2026-10-21
area: battle, render, script
files: crates/piney-battle/src/prim.rs, crates/piney-world/src/field_world.rs, crates/piney-battle/examples/weapon_probe.rs, tools/test_enemy_weapon_rs.py, crates/piney-game/src/session/tests/show_fight.rs, crates/piney-game/src/session/tests/side_events.rs
started: 2026-10-21T10:43:42-06:00
took: 9m
---

# 416. Event 155's weapon flashes behind the camera stay off Kite: a ray's outer pair kicks only where makePacket would, in front of the eye and on the screen (#66)

Issue #66 (build 3e87cd5, Mutation): while a PC fights a monster in a
cutscene, magenta strike marks showed on Kite's waist. The line in the
shot is event 155's (NUKE-1, `eventTblS205`, Nuke Usagimaru's battle talk
show), block 10, message 17. Block 9 enters Nuke (`entry 2 5 1 1`) and
monster 284 at event point 3 (`battle_ready`); block 10's `pc_act 5 1`
lets his AI fight it while `cam_follow_char` watches, then `camera 2 0`
turns the event camera onto Kite from in front (distance 23), its back
to the fight about 2,000 behind it. The clock for this entry is the
finishing only; the shared investigation is in [[415]]'s.

## The cause

Reproduced headless: the stars came with each hit of the monster's
weapon, untextured gouraud strips on `refLayer`. They are its flash
(`ccEnemyWeaponRad`, a `ccPrimRadiate` plate of 8 rays), which
`ccEnemyWeaponCtrl::ctrl` runs whether or not the enemy is shown.
`ccPrimRadiate::disp` (gcmn 0x00439df0) puts each ray's four points
through `sceVu0RotTransPers` and hands them to
`ccPrimPacket::makePacket` (0x00438580) as two pairs: the inner with
`cont` 0, the outer with 1. makePacket clears `cont` when either point's
z is below 0 (behind the eye) or both lie more than 5632 across or 4608
down from 0x8000, and writes ADC (0x8000) for such a pair: no triangle.
The port's `draw_rays` drew every ray. Behind the camera each point's
divide by a negative w mirrors it through the eye to the screen's
middle, its z (about -21,300,000) passes the GREATER test as a word, and
the flash came out over Kite. Now `draw_rays` kicks a ray only when
`piney_battle::prim::pair_continues` holds for its outer pair, the rule
the trails already used, moved beside the rays as one function.
[[415]]'s camera fix also keeps the fight's hit marks and words (pulled
in front of the eye) from being sent while the event camera faces away.

## What else it covers

Every weapon flash of every race (97 tables, 225 rows) and the boxes',
idols' and radiators' rays, on all four volumes, in play as well as in
events: a flash or a box's light behind the camera no longer lands on
the screen.

## Check

`MakePacketAgainstGame` in tools/test_enemy_weapon_rs.py runs the game's
makePacket natively on 4,000 pairs (behind the eye, the bounds 5632/5633
and 4608/4609, the word's ends) and compares the ADC it writes with the
port's (`weapon_probe`'s `cont`): 0 mismatches; dropping the z test fails
it. `session::tests::show_fight::the_shows_flashes_and_marks_stay_on_the_fight`:
event 155 by the survey pilot (god on) to block 10, its messages moved
on to the shot on Kite, held 2,800 frames. On 300 frames or more the
camera's back is to the fight and a flash shines on 10 or more of them;
on none is a strip or an effect drawn. Facing the fight, its rays and
hit rings are drawn (10 and 30 frames or more). Without the rays fix it
fails (8 strips behind the camera), without [[415]]'s (17 effect draws).
`mutation_side_event_survey` (ignored) plays Mutation's 155 and 162.

Shots: before, /mnt/data/claude/scratch/i66/words-before/w02455.png and
/mnt/data/claude/scratch/i66/crop2630.png (the stars on Kite, as in the
issue); after, /mnt/data/claude/scratch/i66/after-show/show-2630.png
(`the_show_shots`; no magenta on any of its 80 frames). piney-game's suite
(four threads), piney-battle's and piney-world's tests, the weapon
harness, clippy and fmt pass.

**Still unknown:** nothing.
