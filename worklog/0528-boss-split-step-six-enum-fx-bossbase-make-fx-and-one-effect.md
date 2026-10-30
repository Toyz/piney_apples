---
number: 528
title: Boss split, step six: enum Fx, BossBase::make_fx and one effect pass; on the effect side Mirrored pictures and one sync from one Source
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, crates/piney-effect/src/boss.rs, crates/piney-effect/src/lib.rs, crates/piney-game/src/fx.rs, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T12:00:53-06:00
took: 18m
---

# 528. Boss split, step six: enum Fx, BossBase::make_fx and one effect pass; on the effect side Mirrored pictures and one sync from one Source

The sixth step of section 2's boss split (`plans/rust-cleanup.md`, the
boss effects item; 523-527 before it). The effects whose `Draw` a class
runs had ten `Option<Box<_>>` fields on `Eff`. They also had seven copies
of the manager's pass, four of the maker, and four separate mirrors on
the effect side. All three are one now:

- the battle side: `Fx`, `make_fx` and `effect_pass`;
- the effect side: `Source`, `Mirrored` and `sync`.

The 288 pinned probe answers are byte for byte the same, and the boss
harnesses pass against the game.

## The battle side

- **`enum Fx`** in place of `Eff`'s ten fields (`missile`, `meteorite`,
  `needle`, `fidchell`, `bolt`, `ball`, `hell`, `macha`, `tarvos`,
  `corbenik`): `Eff.fx: Option<Fx>`.
  - Its variants box each class's rules.
  - A `From` for each, written by a small macro.
- **`BossBase::make_fx(cx, kind, fx, pos, dirc)`** puts an effect with its
  rules in the first free slot and shows it.
  - It replaces Corbenik's, Fidchell's, Macha's and Tarvos's `make_fx`s.
  - It also replaces the inline create-then-fill of Innis's missiles,
    Kyvia's meteors, bolts and ball, Magus's needles, a seed's flush and
    the star dust's thunders.
  - Each rules value is built before the slot, as the copies did; the
    needle keeps the body's heading as its `dirc`. The hell's express
    still fills its slot itself, since it starts its generators between
    the two.
- **`BossBase::effect_pass(mode, draw)`** is `ccBossEffManager::Draw`'s
  pass:
  - each enabled effect in slot order, a disabled one freed;
  - an effect with rules taken out of its slot for the class's `draw`,
    which gets the base and the slot, then put back;
  - the rest keep their lives (`Eff::draw`).
  - It replaces the seven `manager_pass`es (Innis, Magus, Kyvia,
    Fidchell, Macha, Tarvos, Corbenik).
- **`PassMode`** keeps the one way they really differed:
  - `Set`: the `Draw`'s answer sets `m_bEnabled` (Macha, Tarvos,
    Corbenik);
  - `ClearOnly`: it only clears it (Innis, Magus, Kyvia, Fidchell).
- **The other difference was not one.** The copies ran over a snapshot
  of the slot count (`for k in 0..len`) or the live count (`while k <
  len`). The slots are made all at once (1,024) by the first `create`. A
  pass that starts with none has nothing that could make one, so both
  read the same.
- **The draws Magus and Kyvia ran in their slots** (needles, meteors,
  bolts) are taken out like the others. None of them reaches the manager.

## The effect side

- **`piney_effect::boss::Source`**: the effects with their rules by their
  slot (`(i32, &Fx)`, every live boss's in character then slot order),
  Kyvia's members' places, Magus's leaves' markers and bursts.
  - piney-game builds it once (`boss_fx`, `kyvia_members`,
    `magus_shows`). That replaces `boss_spells`, `innis_flights` and
    `kyvia_shows`' effects, and Magus's needles.
- **`Mirrored`**, for a picture that mirrors a rules effect:
  - the rules it reads in an `Fx` (`rules`);
  - the slot it follows (`rules_slot`);
  - whether it runs on its own (`unmirrored`);
  - what dropping it lets go (`forget`: a missile's and a meteor's trail
    anchors);
  - its draw of the rules' state (`mirror`).
  - Innis's missile, Kyvia's meteor, bolt, ball and hell, and Magus's
    needles implement it. The four Kyvia ones (but the meteor, which
    forgets its anchors) come from one macro.
- **`BossEffects::sync(ctrl, cx, t, &Source)`** replaces `sync_spells`,
  `sync_missiles`, `sync_kyvia` and `sync_magus`, and the four `*_sync`s
  of `piney_effect::Fx`, with one `boss_sync`. Its order is explicit and
  the old one:
  1. Fidchell's spells, each picture made at first sight;
  2. Innis's missiles: those let go dropped, the rest drawn in the
     rules' order;
  3. Kyvia's members' places, then its four kinds: dropped, then drawn
     in slot order;
  4. Magus's leaves' markers, its needles likewise, then the bursts.
- **A rules slot is matched as before**: the first entry at that slot of
  the picture's own kind.

The diff is about 520 lines added and 600 removed in 28 files.

## Checks

- The pin: 288 probe calls replayed, every answer byte for byte the same.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's full suite (its boss session tests and the boss
  effects' shots), release.
- The boss harnesses against the game on their volumes (the eleven
  `test_battle_*_rs`), `test_boss_effect_rs` on all four volumes and
  `test_bosscam_rs` on Infection, Mutation and Quarantine.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 152 before, 152 after.

**Still unknown:** nothing
