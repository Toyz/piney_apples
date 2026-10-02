---
number: 348
title: "A spell holds its targets for its whole run: the effects' copy of holdFlag was never set"
date: 2026-10-02
area: battle, render
files: crates/piney-effect/src/spell.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/session/tests/spell_hold.rs
---

# 348. A spell holds its targets for its whole run: the effects' copy of holdFlag was never set

The user's video (`work/video_of_bugs/20261002-0643-11.2822681.mp4`, B3 of
a dungeon, a Water Witch): its ice spell lands on Kite and he runs on under
it. In the game a spell or art used against a character holds it where it
stands while the skill runs. `_ccSkillRequest` (INF gcmn 0x00572860) sets
the new `ccSkill`'s `holdFlag` (+0x0d bit 0) for anything but the normal
attacks (sids below 6), buffs, debuffs and heals. Each frame
`ccSkill::Main` (0x005731d0) then runs `ccSkillHold` (0x005752b0), which
gives the targets `EntryAffect(5)`, condition `hold`. Kite's
`ControlMove`, `ccFellow`'s and the enemies' frames all stop on it. The
battle crate had all of this.

An attack spell's run lives twice in the port: the battle crate's
`SkillRun` and the effects crate's `Spell`, the copy its element system
(Fall, Tornado, Convergence, Upheaval, Summons) runs on. The systems clear
`holdFlag` at their own ends. So the world takes `hold` back from the
effects' copy after each system frame (`combat/mod.rs`, `r.hold =
out.hold`). But the copy was made by `Spell::new` (`hold` false) and
`Spell::sync` did not carry the flag. The first system frame handed back
false, and the hold lasted one frame. A probe showed it: Kite's `hold`
condition 1 on frame 0, then 0, with the run's flag false from frame 1.
Arts were not affected: they run no system. A probe of Kite's art 6 on a
Mimic held it for its whole run until the art's note 6.

`SpellRun` now carries `hold`, and `Spell::sync` sets it before the system
runs. With the fix, a foe's Convergence (213) on Kite holds him for 48
frames, until the system lets go.

Tests (`session/tests/spell_hold.rs`):

- `a_foes_spell_holds_kite`: Kite runs with the stick held, then is held
  still under the spell for its whole hold, then runs again.
- `kites_spell_and_art_hold_the_foe`: the foe's `hold` is 1 every frame
  of Kite's spell and of his art.

Both fail with the flag left out of the sync ("held only 0 frames").

**Still unknown:** nothing.
