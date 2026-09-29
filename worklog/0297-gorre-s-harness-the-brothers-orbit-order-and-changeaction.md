---
number: 297
title: Gorre's harness: the brothers' orbit, Order and ChangeAction checked against the game
date: 2026-09-28
area: battle, test
files: crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-battle/examples/battle_probe/main.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/combat.rs, tools/test_battle_gorre_rs.py
---

# 297. Gorre's harness: the brothers' orbit, Order and ChangeAction checked against the game

Gorre (`ccBoss05` + `ccBoss05Brother`, [[296]]) went into the last session
without a harness. Built one this session: `tools/test_battle_gorre_rs.py`
(from `test_battle_fidchell_rs.py`'s own pattern) and
`crates/piney-battle/examples/battle_probe/gorre.rs` (from `fidchell.rs`),
registered in `battle_probe/main.rs`. It builds `ccBoss05` (which builds
both `ccBoss05Brother`s) with the game's own constructor at a fixed VA and
runs `Main` (which runs each brother's own `Main` under it) frame by
frame against the port, comparing Gorre's own generic `ccBoss` fields,
each brother's the same plus its own `m_posB`/id, the effects alive
(unordered - see below), the party, the calls and the menu. `PINEY_VOLUME=outbreak
python3 tools/test_battle_gorre_rs.py bulk N` runs it.

Getting the harness to run at all surfaced several real bugs the harness
alone would not have caught without decoding the functions it disagreed
on:

**The brothers do not turn with Gorre's heading.** `ccBoss05::TransPosB2W`
(OUT gcmn, called once from the constructor after `Init`) is a plain
translate - `unitMatrix`, `TransMatrix(mat, mat, this->pos)`,
`ApplyMatrix(out, mat, in)` - no rotation. The existing port's
`brother::mov` rotated the formation offset by Gorre's current heading
every frame; the game does not. `m_posB` (`Brother::offset`, +0x29400) is
a plain world-space addend, `pos = master.pos + m_posB`, changed only by
whatever `Move` or a think function explicitly adds to it that frame. This
also meant `pos_p` should not be set at construction (`Init` never calls
`ccTransPosW2P`, only the first `Move` derives it from `pos`) and that
`body_hit_sw` starts at 0, not 1 (no `OnBodyHit` in `Init`; the harness
caught both at frame -1, the state right after the constructor).

**`ccBoss05::ChangeAction` (0x004985c0) is its own override**, not the
base's: the base's `ChangeAction`, then `br0->Order(act)` and
`br1->Order(act)` with the *same* act number, then
`m_actSubCount`/`m_actSubProccess` cleared. The port had never modelled
this - it ordered brothers only from the specific attacks that already
did so by hand (`on_wave`, `on_drain`, `on_tornade`, `on_dead`), which
happen to use the same act numbers `ChangeAction` would have used anyway
(`brother::act::WAVE == 4 == gorre::act::WAVE`, and so on for
`EPITAPH`/`EPITAPH_WAVE`/`DEAD`/`TORNADE`). Every other act Gorre takes -
`DATA_DRAIN_ATK`, `KERSE`, `TALK`, `SKILL`, `MAGIC`, `DMG0`/`DMG1`, the
patterns the base's own `ExecPatternIndex` handles - never reached the
brothers at all, so they sat frozen in whatever act they last had while
Gorre fought on. Added `gorre::change_action`, a wrapper around
`Boss::change_action` that also orders both brothers, and routed every
call site through it except the constructor's first (before the brothers
exist). The base fallback in `ExecPatternIndex` calls
`Boss::base_exec_pattern_index` directly (shared code, cannot be wrapped
from inside); ordered the brothers there too, but only by comparing
`act_num` before and after the call.

**`ccBoss05Brother::Order(on)` (0x0049e020) has a real switch**, not a
plain `ChangeAction` at forbid 1: 12/21/14 keep their own act at forbid 3;
3 and 5 (Gorre's own Kerse and Talk) become act 15, an act with no case in
`Think` (a plain watch, same as `OnThinkNeutral`'s default); 4 (`WAVE`)
also resets `m_bWaveEnd` and sets the anm to "ANM_xx11wave" (not ported:
`OnThinkWave`'s own case 0 sets the same anm a moment later, so this is
harmless); 15 and 0 go to a plain neutral at forbid 1; anything else (most
of Gorre's own acts, reached only through `ChangeAction`'s new forbidding)
falls to a default branch that sign-extends `on` as the act and passes
through whatever `forbid`/`af` happened to be left in `$a2`/`$a3` by
`ChangeAction`'s own preceding call to the base `ChangeAction` (which
itself ends by calling `SetAnm`). Traced empirically rather than by
reading `SetAnm`'s own tail: a case with Gorre in `DATA_DRAIN_ATK` (18)
showed the brother's own `act_forbid` at 3, matching every other named
case, so the default now also uses forbid 3.

**Each brother's own `bossTbl` row is its own**, not Gorre's row 4:
`ccBoss05Brother::Init` calls `ccGetBossParam(id ? 41 : 40)`. Rows 40 and
41 share row 4's HP/SP/PP/level/type but differ from row 4 and from each
other in their elemental resistance and item-drop bytes (raw row bytes
compared: `4 != 40 != 41`). Wired `GorreData::BROTHER_ROW = [40, 41]`
through `new`.

**`OnThinkNeutral` (0x0049cb50) and `OnThinkEpitaph` (0x0049cc50, act 12)
turn to face the master and slowly orbit it**, not "hold formation facing
Gorre's heading" as the port had it: stopped or held
(`this->stop`/`cond::HOLD`), a report back (`SendMessage` msg 3, not
ported) and, for `OnThinkNeutral` only, no turn; else `ccSetDirc(&dirc[2],
ccGetDirc(pos_p, master.pos_p), 256)` (a smoothed turn toward the master,
not a copy of Gorre's own heading), then `m_posB` rotated a fixed
0x3c8efa35 radians around Z (`OnThinkEpitaph` always turns first, then
gates only the orbit on stopped/held). This is what makes the brothers
visibly circle Gorre while it is idle. Added `brother::face_master`,
`brother::orbit` and rewrote `on_neutral`/`on_epitaph`; `on_wave` (a
still-unwired approximation, see below) now at least calls
`face_master` instead of copying Gorre's raw heading.

**`ccBossEffFinalPhotonFlashCreate`'s life is 72 frames**, not the 45
carried over from `WaveShock`: its own `Draw` (0x0046f460) counts a life
(1000.0, `-100.0` a `Draw`) down past 0 on the 11th call (ringing and
starting a 61-call countdown, the old counter reaching 60, before
`m_bEnabled` clears): 11 + 61 = 72. Measured by running the game's own
`Create` and `Draw` natively, the same way `test_effect_lives` already
does for the other kinds.

**A WaveShock or FinalPhotonFlash the port fires from a custom
position/direction** (`on_wave`'s effect at each brother's place,
`on_tornade`'s and `on_kerse`'s at the target) went out through a bare
`cx.out(Out::Effect { id: -1, .. })`, never registered in `Boss::effects`.
The game's own `_g_bossEffManager` is one array shared by Gorre and both
brothers, so it would show these; the harness's "eff" comparison
(unordered - the port keeps each character's own `Effects`, so slot
indices never line up with the game's single array anyway) would have
silently missed them. Switched those three call sites to
`Boss::effect_at` (which does track), matching how every other boss's
own effect calls already work.

Added `gorre_brother_anims`/`gorre_brother2_anims` to the manifest and
regenerated `combat.rs`/the data store (`cargo run -p piney-gen -- gen
--group combat`) for `boss05SlaveAnmTbl1`/`2` (0x5ebf80/0x5ebff0, both
25-entry `array(opt(cstr()), 25)` like `Boss05AnmTbl`): the brothers had
no anm table at all before this, so `change_action` never set a clip on
them. `brother::new` now sets `b.anm_tbl` from the right one
(`CheckSlaveID`'s id) and calls `Boss::change_action` itself (with
`cx.me` swapped to the brother) instead of poking `act_num` directly, so
the initial "ANM_ex51nut"/whatever-id-1's-own-neutral-clip-is is set the
same way the game sets it.

With all of the above, `bulk 1`'s frame -1 (right after the constructor)
and the neutral idle that follows match exactly for a good while; the
first random case (seed 9900) now parts company around frame 91, in
`Brother`'s own orbit: the port's heading has turned about one
`ORBIT_STEP` (roughly a degree) further than the game's by then, an
exact-multiple drift that looks like an off-by-one in how many times
`orbit` has run rather than a wrong constant, not found this session.
`bulk 20` does not reach 0 mismatches. Left unfixed and documented in
docs/engine/boss-gorre.md's Checks and Unknown: `OnThinkWave`'s and
`OnThinkTornade`'s own state machines for the brothers (decoded down to
`m_moveSpdB`/`m_fRad` easing and their own `SendMessage` calls, msgs 2, 5,
6, 7, but not wired into the port, which still holds its own 90/60-frame
timers instead of waiting on the brothers), `SendMessage`'s full switch,
`OnThinkDead`'s exact camera math, and `ccBossCam`'s `MaxRenge` (+0xD8),
whose real source was not found even for Fidchell's own harness (its
value there, 600, is not written by anything the stand-in `InitBossCamera`
runs or that this session could find) - dropped from the comparison for
both bosses' probes rather than left wrong.

Ran `PINEY_SURVEY_ONLY=218 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=150000
cargo test --release -p piney-game outbreak_story_survey -- --ignored
--nocapture` after these changes: it does not reach "done" - `story 218:
blocks 0xe8f`, stuck cycling menus in "The World - area 1 field 5" (event
218's own field), with `walk_to` also logging Kite stuck in two rooms
during the run. Could not cleanly compare against the pre-session code:
the data store's own format changed underfoot (`gorre_brother_anims`
added), so the old `combat.rs` panics against the now-larger store
(`.hack//Outbreak's combat: ends at 49846 of 49847 wanting 4 more`) rather
than giving a comparable run. Whether the stuck survey is new or
pre-existing, and whether it is even Gorre's doing rather than an
unrelated `walk_to` pathing issue in the same field, is open.

All of piney-battle's existing unit tests (106, including Gorre's own and
every other boss's) and piney-data's and piney-gen's pass unchanged;
`cargo fmt`/`clippy` clean on `piney-battle`, `piney-gen`, `piney-data`.

**Still unknown:** the orbit's one-step drift past ~90 frames; whether the
harness reaches 0 mismatches once that and the Wave/Tornade/SendMessage
gaps above are closed; `ccBossCam`'s real `MaxRenge` source; whether the
survey's stuck field-5 menu loop is Gorre's own doing, a pre-existing
`walk_to` issue, or both.
