---
number: 370
title: Skeith's fight plays his own theme; a boss's blow shows its damage once
date: 2026-10-04
area: audio, battle
files: crates/piney-game/src/area.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session/tests/skeith.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/party_leave.rs, docs/engine/sound.md, docs/engine/boss.md, UNKNOWNS.md
resolves: 112
---

# 370. Skeith's fight plays his own theme; a boss's blow shows its damage once

Two player reports from Skeith's fight, the last of Infection.

## The music (#43)

The fight played Chosen Hopeless Nothingness's field music, the
Wasteland night battle arrangement, not Skeith's theme.

`ccSetupGameCtrl` picks a field's bank at main 0x00169160-0x001691a8:
`GetEventAreaInfo(game.field)`, and with `game.field` not 0 and that
row's `model` 1, `ccSndSQLoad(5)` (the event bank, row `game.field`);
else `ccSndSQLoad(3)`. The row is looked up by `game.field` itself.
The port took the model from its `WORLD_MAN` instead. The two agree
everywhere but in the arena: area 27's last door goes to field 1 with
area 27's `WORLD_MAN` still in place ([[112]]), whose model is 0. So the
port loaded area 27's field bank (`Field { field_type: 7, bg: 2 }`), as
[[112]]'s open question feared. Event 30 has no `sound` instruction in
blocks 20-22: the bank is all the music there is.

The area mode now reads the model from `AreaTables::event_area_info`
(`GetEventAreaInfo`, with Mutation's substitutes) for `scene.field`. The
same request passed `piros: false` always. It now asks the party's
slots for character 8, as `checkPartyMenberNum(8)` does over
`ccPartyManager`'s ids, so Piros's field music (`piroshi`) plays with him
along. `ccPartyManager` keeps its ids (+0xc) across the scene change:
the members' destructors (`ccFellow`'s at gcmn 0x0041adfc) clear a slot's
character pointer, not its id. So the party carried in is the one the
game reads.

`skeith_plays_its_own_music` enters field 1 from area 27's dungeon
(floor 4, room 3) on area 27's `WORLD_MAN`. The only load is
`Event { field: 1, area_prev: 2 }`, whose row exists and is not area 27's
field row. Before the fix the load was the field bank.

## The numbers (#44)

Every blow of Skeith's showed its number twice over the member hit, the
second line just under the first. HP dropped once.

The boss's blows go through `ccBossSkillDamage`'s `EntryAffect` on the
member inside the boss's task. The port's `boss_frame` returns those
events, and `Combat::frame` pushed each as a show, then handed it to
`consequence`, which shows whatever it does not consume. So a FlyFont
went to the effects twice and `ccDamUprStr::AddStr` put up two lines. A
box's trap 0 (`invokeTrap`'s `EntryAffect`) went the same way. Every
other source hands its events to `consequence` alone. Both pre-pushes
are gone.

So it was every boss, not Skeith only: Innis, Magus, Kyvia, Fidchell and
Gorre's own blows, one target or several, and trap 0's damage. Their
hit marks and Kite's rumble were doubled too. A normal enemy's attack
and a skill a boss runs through `ccItemSkillRequest` were not; those
reach `consequence` alone.

`FxCensus` gains `new_fly_fonts`: the lines put up this frame (alphaCnt
23), with the character under them. `skeith_hits_show_their_damage_once`
plays event 30 into the fight, Kite kept alive. Each frame he loses HP,
there must be exactly one new line over him, the HP lost. Before the
fix all 26 blows of 9000 frames had two lines. After it, the 3 blows by
frame 4000 have one each. `a_trapped_box_opened_as_it_is_goes_off` now
checks trap 0's number the same way: two lines before, one after.

`piros_along_plays_his_field_music` enters area 19's field (model 0)
with Piros in the party: the load is `Field { piros: true }`, whose row
exists. It fails with `piros` false.

The game suite passes (229, 82 ignored), and piney-world's.

**Still unknown:** nothing.
