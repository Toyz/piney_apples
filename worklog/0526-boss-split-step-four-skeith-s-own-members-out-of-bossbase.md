---
number: 526
title: Boss split, step four: Skeith's own members out of BossBase into Class::Skeith, the brother's and Gorre's borrowed ones their own
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-world/src/combat/boss.rs, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T11:40:21-06:00
took: 11m
---

# 526. Boss split, step four: Skeith's own members out of BossBase into Class::Skeith, the brother's and Gorre's borrowed ones their own

The fourth step of section 2's boss split (`plans/rust-cleanup.md`;
523-525 before it). `BossBase` held `ccBoss01`'s members for every boss.
The ones only Skeith reads are now its class's own, and the two other
classes that borrowed them have their own. The 288 pinned probe answers
are byte for byte the same.

## Skeith's class

- **`Class::Skeith(Box<Skeith>)`**. `Skeith` holds:
  - the spell's force and ice effects (`eff_force`, `eff_ice_break`);
  - the death's effect and its two fades (`eff_dead`, `animate`,
    `animate_dead`);
  - the stage effect it holds (`stage_eff_id`);
  - the wave's ring clip (`anm_wave`).
  - Its `BossClass::main` is `ccBoss01::Main`.
  - `Class`'s `Default` is a default `Skeith`, as the unit variant was.
- **Skeith's rules take it beside the base.** The acts that read those
  members take `s: &mut Skeith`:
  - `think`, `action`, and the wave, epitaph wave, chase, dash and magic
    acts.
- **`skeith_main`** is the building blocks with its own `PreDrawAnm`
  inline: the wave's speed 512 in its acts, then `forward_and_draw`.
- **`base_main`** (the Innis images' and Corbenik's roots' `ccBoss::Main`)
  now ends in `forward_and_draw`. Their wave speed was never read, and
  they no longer have a wave clip.
- **Its pattern code** reads none of those members. So it is
  `impl Patterns for SkeithPatterns`, a unit struct. Skeith's acts,
  `ccBoss01::Affect` and Gorre's fallback all use it.

## What stays on the base, and why

- `patMode`: Gorre reads and writes the base's (its message's Super
  switch, its constructor's mode 2). `ccBoss01::Affect` also does, and
  Corbenik's eyes and roots keep that affect as their default, on their
  own bases.
- `m_actDirc`: six classes read it on the base (Magus, Macha, Tarvos,
  Gorre, Corbenik's first and second forms).
- The field's comment says so.

## The borrowers

- **A Gorre brother** kept its wave clip and its death's effect on its
  base. They are now `Brother::anm_w` and `Brother::eff_dead`.
  - `anm_w` starts as `Anm::new()` and is set to the wave's clip in the
    constructor, as the base's was.
  - The strike takes the brother's members to forward it.
- **Gorre** kept its spells' stage effect on the base. It is now
  `Gorre::stage_eff_id`, starting at 0 as the base's did.

## The bare `ccBoss`

`Class::Plain` (a part between its master's takes) is now its own unit
`Bare`:

- Its default `main` is `ccBoss::Main` with no `Think`. It was Skeith's.
  `Boss::main` only runs on the fight's master, so nothing reaches it.
- Its `affect` is still `ccBoss01::Affect`, as it is for Corbenik's eyes
  and roots.

## Outside the rules

- **piney-world's boss draw:**
  - It tests `Class::Skeith(_)` for the cross's trail.
  - It reads the wave's time from Skeith's class. Any other master's base
    wave was never set (time 0), so that arm is 0 now.
- **The Infection probe** reads the wave and the fade from Skeith's
  class. For a boss that is not Skeith it reads a default `Skeith`.

## Checks

- The pin: 288 probe calls replayed, every answer byte for byte the same.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's full suite (its boss session tests), release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 152 before, 152 after.

**Still unknown:** nothing
