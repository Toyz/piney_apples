---
number: 284
title: Open questions after the triage: the effects and the streams' effect tasks
date: 2026-09-28
area: render, video
files: UNKNOWNS.md
resolves: 53, 57, 79, 83, 100, 105, 113, 115, 116, 117, 119, 121, 122, 124, 125, 126, 128, 131, 133, 136, 141, 153, 156, 170, 172, 175, 195, 197, 234, 279, 280, 281
---

# 284. Open questions after the triage: the effects and the streams' effect tasks

This entry closes the open questions of 32 entries about the effects: the
effect system (`piney-effect`), particles, the bosses' pictures and the
in-engine streams' effect tasks. The triage of 2026-09-28 (UNKNOWNS.md)
split every entry's open paragraph into single questions and checked each
one against the later log, the docs and the code. The answered ones are
listed below with their evidence. What is still open is restated at the end,
with the entry that asked it, the play questions first. Open questions of
these entries that belong to another subsystem are restated in that
subsystem's entry: [[282]], [[283]], [[285]], [[286]], [[287]], [[288]],
[[289]] and [[291]].

## Answered

- [[57]]: stream 2's fog — answered by [[127]], [[145]]
- [[57]]: the table's 21 other effect tasks — answered by [[85]], [[121]],
  [[122]], [[124]], [[125]], [[126]], [[133]] (all of Infection's)
- [[79]]: `piney-effect` not called by the runtime — answered by [[80]],
  [[81]]
- [[79]]: `effSkillStart` and `effSkillStartEffect` — answered by [[83]],
  [[195]]
- [[79]]: the exec rings and the physical skills' shock wave — answered by
  [[83]], [[195]]
- [[79]]: heals, cures, Sanity, Resurrect, stat changes, the resistant
  shield — answered by [[83]], [[194]]
- [[79]]: opening boxes, removing traps, the breakables' fragments —
  answered by [[98]], [[116]], [[140]]
- [[79]]: the in-engine streams' effects — answered by [[131]]
- [[83]]: the new starters not wired into `piney-game`'s `fx.rs` — answered
  by [[194]], [[195]]
- [[83]]: the breakables' crush and fragment effects, the bosses' own deaths
  — answered by [[116]], [[140]]; Skeith's death effects by GAPS.md "Stale
  lines" (`BeginDeadEffect`, `ccBossEffDead`)
- [[105]]: `Func_str8000`, the drain streams' effect task — answered by
  [[125]]
- [[113]]: the cross's trail, the six effects, the reversed layer and the
  quake — answered by [[115]], [[117]]
- [[113]]: `ccBossCam` and the dead camera — answered by [[115]]
  (`tools/test_bosscam_rs.py`)
- [[115]]: `DrawCross`'s trail, the six effects and the reversed layer —
  answered by [[117]]
- [[116]]: the idol's dust ring — answered by [[118]] (an idol opening)
- [[116]]: no shot of a barrel smashed or a trap going off — answered by
  [[148]] (a breakable broken in play)
- [[117]]: Skeith's cinema — answered by [[128]] (`tools/test_cinema_rs.py`)
- [[117]]: the fields' type-1 fireflies (`SetBasePosition2`) — answered by
  [[130]] / `crates/piney-world/src/firefly.rs`
  (`tools/test_field_ambient_rs.py`)
- [[119]]: `effSmoke`'s other callers — answered by [[190]]
- [[121]]: `Func_str0090`'s and `Func_str0110`'s fog — answered by [[127]]
- [[121]]: the effect tasks of streams 7-9 and 11-16 and the str7xxx-str9xxx
  scenes — answered by [[122]], [[124]], [[125]], [[126]], [[133]]
- [[122]]: what the marks and the transfer look like in a stream
  (`ccThEffectStr`) — answered by [[131]] (`tools/test_effect_str_rs.py`)
- [[122]]: stream 7's fog — answered by [[127]]
- [[122]]: the tasks of streams 11-16 and the str7xxx-str9xxx scenes —
  answered by [[124]], [[125]], [[126]], [[133]]
- [[124]], [[126]], [[131]]: stream 15's `Func_str0580` and `Func_str0581` —
  answered by [[133]]
- [[124]]: the str7100, str8000, str8800 and str9xxx tasks — answered by
  [[125]], [[126]]
- [[124]]: the marks and transfers (`ccThEffectStr`) — answered by [[131]]
- [[124]]: the fog of streams 13 and 16 — answered by [[127]]
- [[125]]: stream 20's fog — answered by [[127]]
- [[125]]: `Func_str7100`, `Func_str8800`, `Func_str0580` and `0581` —
  answered by [[126]], [[133]]
- [[128]]: the cinema's other names (the other bosses' rows, x01-x04) —
  answered by `crates/piney-world/src/cinema.rs` (all 72 rows of
  `_g_cinemaSkillName`; `tools/test_cinema_rs.py`)
- [[133]]: the shades on the GPU (`REGION_REPEAT` as a plain repeat) —
  answered by [[146]]
- [[133]]: VU1's clip at `divZ` for stream 15 — answered by [[153]]
- [[136]]: `piney-gs` clipping at a fixed 1000, not `divZ` — answered by
  [[153]]
- [[195]]: the party's weapon trails (`ccSpcChar::ArmsEffect`) and
  `StartArmsEffect`'s particles — answered by [[197]]
- [[280]]: Mutation's convergence aim, `effSkillChargeObj`'s new argument,
  the drill's `endFlag` — answered by [[281]]
  (`tools/test_effect_spell_rs.py` on MUT, 0 mismatches)

**Still unknown:**
- (play) [[79]], [[231]]: `SetFogSw` and `SetShadowSw` on the spell pieces
  (and on other objects that turn fog off; only the gate was fixed), and the
  drill's `ccDrawEnv` settings, are not modelled (docs/engine/effects.md) —
  spells and objects fogged or shadowed where the game turns it off.
- (play) [[79]]: the explosions' palette swaps are kept on the element but
  not drawn (docs/engine/effects.md) — the spell explosions' colours.
- (play) docs/engine/effects.md: `ccEntryChangeCLUT` swaps every texture on
  the palette, where the game swaps per material — enemies whose
  palette-swapped texture shares a palette.
- (play) [[170]]: the enemies' weapon flashes (and the boxes') put a light
  in the scene's light group in the game; the port does not — characters not
  lit by the flashes.
- (play) [[266]]: Innis's pictures are not drawn: the missiles,
  `SamonRing`s, particle generators, blur, shield, the mirrors' shards; the
  `MagicSquare` for n 1 and 2 (docs/engine/boss-innis.md) — Mutation's first
  boss.
- (play) [[274]]: Magus's pictures are named, not drawn: the leaves'
  markers, charges and bursts, the laser's shocks and thunder, the needles,
  the smoke; the body's dropped leaves are not hidden
  (docs/engine/boss-magus.md) — Mutation's Magus.
- (play) [[280]]: Innis's ice missile and Fidchell's `IceBreak` are not
  ported, so nothing throws their rocks — Mutation's and Outbreak's bosses.
- (volumes) [[280]], [[281]]: Outbreak's and Quarantine's `sqrt.s` in the
  distance fade and their one-ulp float differences; whether the per-volume
  hooks and the 42-spike wave make upheaval agree there; not rerun.
- [[17]]: the effect program beyond its entry point (docs/engine/render.md
  Unknown).
- [[79]]: undefined behaviour taken one way: odd sizes or drain types, stack
  leftovers as 0, the goblin summons' read (docs/engine/effects.md Unknown).
- [[83]]: `effResistantShield` with no `affectPerson` reads through a null
  pointer (game UB; docs/engine/effects.md Unknown).
- [[83]]: `effHeal`'s return: the game returns 0, the port its slot; no
  caller uses it.
- [[79]]: a sprite's fog: `Prim` has no fog, and the effects' own sprites
  never fog (docs/engine/effects.md).
- [[100]]: whether `ccAnm::Draw` of a fog-off clump (the portal) takes the
  field's lights; the port draws it unlit.
- [[156]]: effect nodes in other files' `ccAnm` clumps; none found
  (docs/engine/effects.md Unknown).
- [[170]]: what a stream's decoded dummy chunk holds at `+0x10` and `+0x20`;
  the port takes the file's place, w 1, degrees as radians.
- [[280]]: what `ccEffect::Main` ids 0, 113 and -11..-8 were for; nothing
  makes them.
- [[280]]: what a real PS2 keeps at address 0, which the -11..-8 case would
  overwrite.
- [[281]]: why Mutation moved the spells' releases (93, 120, 60, 100).
- [[281]]: `AreaFx::spell` requests a spell at its first system call, so
  `+0x70` is the target's place at count 0; a runtime asking earlier must
  pass it.
- [[238]]: whether any skill's animation draws (no clump; nothing draws
  `+0x80`).
- [[279]]: whether `effAbilityDown`'s icon rows include a staff; the rows
  are not rendered one by one.
- [[279]]: whether a killed foe's condition effect fades or ends at once
  (`DispConditionEffect` against the game's delete order).
- [[266]]: which clump the game draws the Epitaph's `ANM_ex2x*` clips on.
- [[272]]: which of `CMP_ex01gom2`-`4` each gomora draws
  (docs/engine/boss-kyvia.md Unknown).
- [[117]], [[128]]: `ccBossBlur` is not ported; whether Skeith's fight blurs
  depends on the heap (docs/engine/boss.md "Not described yet").
- [[127]]: what `SetFog(0, 0, ...)`'s divisions by zero leave in
  `fogA`/`fogB` (stream 4); taken as no fog. [[267]] meets the same in
  `Func_str0932`.
- [[131]]: `strEffectStopFlag` is never set by anything the port runs; its
  setter in the game is not named.
- [[53]]: blend types 2 and 3: the streams are not surveyed for them.
- [[55]]: whether any stream uses a morph weight that is negative or 8 or
  more.
- [[57]]: the rand state when stream 2 starts; the port matches only from a
  known state (docs/engine/stream.md Unknown).
- [[104]]: a noise band's copy past the draw buffer reads 0, as in the
  stream.
- [[130]]: the field's smoke puffs start a frame late (docs/engine/field.md
  Unknown).
- [[141]]: Data Drain step 10's effects start at the next effect task, so
  their `rand()` draws come a frame late (docs/engine/field-ui.md "Not yet
  known or not ported").
- [[165]]: the effects' shakes are applied after the frame, not inside the
  effect task (only the `rand()` order within the frame).
- [[172]]: the run-time's bursts draw `ccRand` after the entry control's
  frame (ordering).
- [[160]]: the console's choice between the direct and the clipped shadow
  path (a pipelined sticky sign); they differ only past Z 0x7fffffff
  (docs/engine/shadow.md).
- [[118]], [[119]]: the runners' dust: no shot of it, and the feet's pose is
  the port's stance at the start against the game's last draw.
- [[175]]: whether the window showed Skeith's lag worse than the headless
  run; not measured in the window.
- [[175]]: the drain's erasing of the figure is inferred from the rules.
- [[53]]: the glows and stream 2's figure are checked against the code, not
  a console frame.
- [[57]]: stream 2's effect task: the packets are checked, not the pixels
  and timing.
- [[105]]: the drained enemy's look in the movie; its lights are worked out
  at its matrix.
- [[117]]: the trail's projection, the reversed sprite and the light on the
  characters' shading, checked by eye only.
- [[133]], [[136]]: stream 15's parts and opening against a game frame; the
  draws and the draw state are checked.
- [[136]]: the other streams' light pictures (3, 5, 7, 8, 10, 11, 16)
  (docs/engine/stream.md Unknown).
- [[153]]: a close-up of streams 5 or 15 against the game; str0581's `divZ`
  3000 and stream 5's 500 not looked at frame by frame
  (docs/engine/stream.md Unknown).
- [[156]]: str0001's clouds against the game's picture.
- [[141]]: the Data Drain side effects' pixels.
- [[149]], [[155]]: the symbols' fires and light frame by frame, and the
  cast on a lake symbol and its puffs on screen.
- [[234]]: the symbol room's darkness against a picture of the real game;
  the light values are harness-checked.
- [[162]]: the growing-up particles and the foods' look against the console.
- [[172]]: the flames against a game picture; the sprite's turn (`+0x28`)
  stays 0.
- [[172]]: the first enemies' weapon trails and dust, not looked at after
  the fix.
- [[195]]: the skill starts and shock waves against the game's pictures in
  play.
- [[197]]: the weapon trails and particles against the game's pictures.
- [[197]]: orange specks far off in the Flame Dance shots come from none of
  the weapon generators; which effect draws them is unknown.
- [[197]]: a member's `StartArmsEffect` is not seen in play; the hands'
  matrices are the port's pose.
- [[83]]: the new particle generator rows are seen only in shots
  (docs/engine/effects.md Unknown).
