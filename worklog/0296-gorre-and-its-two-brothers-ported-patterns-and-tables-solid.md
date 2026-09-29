---
number: 296
title: Gorre and its two brothers ported: patterns and tables solid, per-brother camera math approximated
date: 2026-09-28
area: battle, decomp
files: crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss/gorre/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/pack.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/fx.rs, docs/engine/boss-gorre.md
---

# 296. Gorre and its two brothers ported: patterns and tables solid, per-brother camera math approximated

Gorre (`ccBoss05`, boss05.cpp, `bossTbl` row 4, file `x51`) is Outbreak's
second phase boss, event 218 in field 5. Read from OUT gcmn.prg
0x00497010-0x0049e590 via `tools/disasm.py` (INF's clean listing plus
`tools/voldiff.py diff ... --volume OUT --overlay gcmn` for the codegen
delta) and `tools/dwarf1.py fn` for locals and source line ranges. 33
functions on `ccBoss05` (~21 KB) plus 15 on `ccBoss05Brother` (~9 KB), as
the task named them; INF's `syms.txt` (`work/infection/syms.txt`) gave
the class list and confirms the byte counts almost exactly (33 methods
summing near 21 KB, 15 summing to 8,956 bytes).

## What Gorre turned out to be

Unlike Fidchell (`ccBoss04`, a single character), Gorre fights with two
`ccBoss05Brother`s, each its own `ccChar` beside it. `ccBoss05::Main`
(OUT 0x00498110) is its own override, not the base's: it inlines the
centre's distance and heading itself, calls its own `CalcRealEx(0)`
(the base's per-character `CalcReal` plus a protect-gauge broadcast -
`effProtect`/`SetProtect` - to both brothers every frame), and drives
`Think` and `Slave` through the same indirect-vtable-call pattern the
base's generic `Main` uses (confirmed by matching the vtable slot offsets
against `CalcTargetInfo`/`Move`/`PreDrawAnm`'s known positions in
`ccBoss::Main`'s own disassembly).

The two brothers are not simulated inline the way Innis's three slaves
are (`innis::slave_main` writing directly into their characters from a
single call); each is a full `Boss` of its own, exactly like Kyvia's core
and gomoras (docs/engine/boss-kyvia.md's `Tree`).
`crates/piney-battle/src/boss/kyvia.rs`'s
`Part<T>`, `plain_boss`, `new_char`, `set_base_param` and `head` are all
reused directly rather than re-copied into `gorre.rs`; `Part<T>` gained
`Clone`, `Default`, `PartialEq`, `Eq` derives (previously only `Debug`)
so `Gorre`'s own struct, which holds `[Part<Brother>; 2]` as scratch for
the span of one frame, can derive the same set the rest of `Boss` does.
`ccBoss05::ExecPatternIndex` (0x004978d0) turned out to share Fidchell's
exact shape for the `-1` wrap, the `move_spd`/`move_vector` reset and
patterns 9-11's target selection; unlike Fidchell, though, its fallback
(patterns 2-8, 12) could call `Boss::base_exec_pattern_index` directly
rather than duplicating it, because Gorre's own `act` module happens to
assign the same numbers to `WANDER` (8), `DASH_CENTER` (7),
`EPITAPH`/`EPITAPH_NEUTRAL` (12), `ESCAPE` (16) and `CHASE` (18) that the
base module does - checked by hand against every pattern the base
fallback can reach.

`ccBoss05::Affect` (0x00498100) is 8 bytes: `jr $ra` and nothing else.
Gorre itself never takes a direct hit; the fight's win condition is
"both brothers' HP at 0", read here by Gorre's own frame polling
`cx.scene.chars[brother_me].hp` each frame rather than the brothers'
`SendMessage(this, msg)` callback the game uses (`SendMessage`,
0x0049b680-0x0049c228, 2984 bytes - the largest function in the class,
and the one this session had least time to read in full: only msg 1 and
4's outcomes, a hit and a death, were confirmed from its branch
structure). Once both are down Gorre goes to its Epitaph table
(`OnThinkDrain`); since nothing in what was read ever sets Gorre's own
act to `DEAD` from there (its `Affect` is empty, so no hit can do it
either), a grace period was added (`Gorre::epitaph_frames`,
`EPITAPH_GRACE` = 300) after which Gorre is taken as finished and exits,
the way Fidchell's own dead effect (`begin_dead_effect`, `EffKind::Dead`)
already does. This is a guess standing in for whatever `SendMessage`'s
msg 21 (which touches `boss05SuperActTbl` and a stat reset, per its
`touches:` line) actually does; the first survey run without it looped
in field 5's menus for the full 150,000-frame budget without the story
ever completing, and with it `outbreak_story_survey` reaches "story 218:
done" (and, since nothing else in Outbreak was blocked, the staff roll)
in 72.99 s wall time.

## Tables

`tools/disasm.py range ... --data` over `boss05NormalActTbl` (0x005ec060,
39 words to its `-1`), `boss05SuperActTbl` (0x005ec100, 47 words) and
`boss05EpitaphActTbl` (0x005ec1c0) gave the real pattern sequences;
`@1261` (0x005ec228, 12 bytes: 157, 158, 160) and `@1777` (0x005ec250, 16
bytes: 157, 158, 159, 160) are `OnThinkSkill`'s and `OnThinkMagic`'s
spells. `@1672`/`@2074` (200, 212, 232, 264, each 16 bytes) were found but
not placed in any function read closely enough to say what they time;
they are not in the manifest. `crates/piney-gen/src/manifest.rs` gained
`gorre_anims`, `gorre_normal`, `gorre_super`, `gorre_epitaph`,
`gorre_skills`, `gorre_magic_skills`; `cargo run --release -p piney-gen --
gen --group combat` regenerated `crates/piney-data/src/tables/combat.rs`
clean, and `DATA_VERSION` (`crates/piney-data/src/pack.rs`) moved 17 to
18.

## What is not ported

Brother's own `OnThinkWave`, `OnThinkEpitaphWave` and `OnThinkTornade`
(1668, 1476, 1356 bytes - real per-brother position and camera math past
their locals, which is as far as `dwarf1.py fn` was read for them) are
approximated as holding the brother's formation place, facing Gorre's
target. `Order`'s special `on` values beyond a plain `ChangeAction`, and
all but two of `SendMessage`'s message codes, are not ported.
`ccBossEffFinalPhotonFlashCreate` (`OnThinkKerse`) is a new `EffKind`
with a placeholder 45-frame life and no picture
(`crates/piney-game/src/fx.rs` returns `None` for it, as Innis's and
Kyvia's own unfinished effects do). None of this was checked against the
game running in eemu - the task's harness
(`tools/test_battle_fidchell_rs.py` and `battle_probe`'s `fidchell`
example) was not built for Gorre in this session; `docs/engine/boss-gorre.md`
is filed `partial` rather than `solid` for exactly this reason.

## What was verified

`cargo test -p piney-battle --lib`: 106 passed (105 before this entry),
including four new `boss::gorre::tests` (`stands_behind_kite_with_two_brothers_beside_it`,
`runs_its_first_pattern_without_panicking`, `both_brothers_down_drains_gorre`,
`both_brothers_down_ends_the_fight`) built on the same `Fight` harness
Fidchell's own tests use, against the real `combat.bin` tables this
environment has for Outbreak (they do not skip). `cargo clippy` and
`cargo fmt --check` are clean on `piney-battle`, `piney-world`,
`piney-game`, `piney-gen` and `piney-data`.
`PINEY_SURVEY_ONLY=218 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000
cargo test --release -p piney-game outbreak_story_survey -- --ignored
--nocapture` reports `story 218: done`. The pilot aids
`levels_for_boss` and `approach_roamer` (`crates/piney-game/src/session/tests/survey.rs`)
were extended to Gorre the same way they already covered Fidchell
(`GORRE_LEVEL` = 75; Gorre added to `approach_roamer`'s boss codes) -
without the level aid the first survey attempt could not be told apart
from a genuine stall, since both look like a fight that does not end.

**Still unknown:** `SendMessage`'s full switch and Brother's own
per-brother formation/camera math (see "What is not ported" above); an
eemu-backed harness comparing the port frame by frame against the game's
own `ccBoss05`/`ccBoss05Brother`, as Fidchell got
(`tools/test_battle_fidchell_rs.py`), was not attempted this session.
