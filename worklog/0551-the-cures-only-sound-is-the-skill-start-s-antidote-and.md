---
number: 551
title: The cures' only sound is the skill start's: Antidote and Restorative play 104 as the game does
date: 2026-11-02
area: audio, battle
files: crates/piney-game/src/session/tests/cure_sound.rs, crates/piney-game/src/launcher.rs
started: 2026-11-02T00:01:00-07:00
took: 12m
---

# 551. The cures' only sound is the skill start's: Antidote and Restorative play 104 as the game does

Issue #76: "There is no sound played when Antidotes/Restoratives are
invoked." The game's code gives them one sound, and the port plays it.

## The game

An Antidote is item R3 (code 0x000a0003: category 10, id
3; `itemTblR` row 3, skill 178), a Restorative R4 (skill 179). In
`ccUseItemRequest` (INF gcmn 0x0057aa80; QUA 0x0048f970, the same switch)
category 10 ids 18 to 23 are the drinks and elixirs with their own
`effHeal`; every other id goes to `ccItemSkillRequest(cp, tp, skill, 0)`,
whose 0 is stype 1 (0x005727a8). `_ccSkillRequest` (0x00572860) skips the
target's `effSkillStart` for 178-180 but, for stype 1, calls
`effSkillStart(cp, sid, 1, 0)` (0x00572b58); that ends in `ccSeOn3D(104,
pos)` for a party member (main 0x001d3970; 159 for a foe). The cure itself,
`ccSkill::RecoverySystem` (0x00577070), calls `effCure` or `effSanity` and
`EntryAffect(16)`: neither effect (0x001cef50, 0x001cf0d0) nor their second
chain (0x001c9348) nor `ccPlayer::Influence`'s 16 (0x0059ac50) calls ccSeOn.
The SE row 104 (program 60, channel 0, note 60, velocity 127) is the same on
all four volumes.

## The port

Used from PERSONAL, Items in a field (`in_field`, Infection), both ask for
the menu's 18 and 104, and 104 is heard: rendered with every event routed,
120 frames carry 437M of energy, 196M with 104 held back
(`the_cures_sound_the_skill_start`). From power-on through the launcher on
Quarantine (the #79 log to its dungeon, an Antidote given and used there,
`an_antidote_sounds_from_power_on`, `replay_pad_log_with`): 104 is asked
for and adds 37M over the music; in the same place a Health Drink's 104 adds
22M and its two 75s 41M. So the cures sound as the game makes them sound:
the skill start's chime alone, where a drink also rings the heal's 75.

`replay_pad_log_with` is `replay_pad_log` with a pad of the test's choosing
for any frame and a filter on the events routed to the engine.

**Still unknown:** what the reporter heard as silence: the port asks for and plays the game's one sound for these items (104); whether 104's level against the music matches a PS2's is not measured, and the reporter's case (where, which volume, on whom) is not in the report
