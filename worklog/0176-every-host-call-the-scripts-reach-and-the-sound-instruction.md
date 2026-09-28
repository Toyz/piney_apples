---
number: 176
title: "Every host call the scripts reach, and the sound instruction's eleven cases"
date: 2026-09-26
area: script
files: crates/piney-game/src/mode.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/main.rs, crates/piney-game/src/stream.rs, crates/piney-audio/src/lib.rs, crates/piney-event/examples/host_calls.rs, docs/engine/sound.md, docs/engine/events.md
---

# 176. Every host call the scripts reach, and the sound instruction's eleven cases

The question: does any event instruction Infection runs still fall to a
host's default?

**The check.** `host_calls` (piney-event's examples) plays every block of
every script on the disc at level 2, on a host that ports nothing. Each
`Host` method a block reaches then falls to its default and names itself.
It prints one line per block: the block's own tags, its conditions and
the methods it reached.

A script over it (under `work/`) works out where each block runs:

- The sticky tags, in walk order, and the conditions give the place:
  status 2 or 3 is the desktop's host, status 5 in a town the town's,
  status 5 in a field or dungeon the area's. A block with only a town,
  field or dungeon tag is status 5.
- It keeps only the events Infection walks (`eventSub`'s ranges) and drops
  those whose open conditions need a later volume's event.
- It compares each block's methods with that host's `impl Host`.

Nothing is missing: every method a walked block reaches is one its place's
host implements.

**Inside the hosts.** Implemented did not mean done. All three hosts
answered `sound` (`ccSndEvRequest`) for command 7 alone. Infection's
scripts ask for commands 0, 2, 4, 7, 8, 9 and 10. Command 4 alone accounts
for 38 sound effects, and the announcement chime (74) is 30 of them.
Command 10, which holds the next area's music, was dropped as well.

The switch (0x0017de80, jump table 0x0034d380) has eleven cases:

| cmd | what |
| --- | --- |
| 0 | `ccSqPlay(p0)` |
| 2 | `ccSqFade(p0, p1, p2, 3)` |
| 4 | `ccSeOn(p0)`, or `ccSeOnNote` at `p1`'s low byte floored at 0 when `p1` is not -1 |
| 7 | `ccEvVoiceStop` |
| 8 | `ccPortVolSet(p0, p1)` |
| 10 | `ccSnd +0x138 = 1` |
| 1, 3, 5, 6, 9 | nothing |

Each case was read against the driver's `sq_play`, `sq_fade`,
`port_vol_set`, `se_on`, `se_note` and `hold_bgm`, whose rules already
match the game's own functions. `mode::sound_request` maps a request to
the session's sound events, with two new ones, `PortVolume` and `HoldBgm`
(`Audio::port_volume`, `Audio::hold_bgm`). The three hosts use it, and
`sound.md` has the table. `the_scripts_sound_requests` checks the uses
the scripts make.

**Stale notes.**

- The town host's "the next area is not ported" was wrong: the session
  builds the `area` instruction's story area as the scene changes.
- The stream player's "stream 5's hit marks and near clip are not drawn"
  was also wrong. With the stream demo's effects set, the stream draws
  the marks and transfers itself and its scene takes the clip. The
  requests are only reports.

Both notes now say so.

**Asked on the way:** are members who cannot be called greyed out in
PARTY > Add? Not in Infection. `PartyInMenuDisp` (gcmn 0x0053b6e0) reads
only `partyMemberFlag` (+0x2220) and draws every row alike.
`PartyInMenu` tests `partyMemberCall` (+0x2224, at 0x0053af3c) only after
the choice, which leads to "There is no response to the Flash Mail." The
port does the same.

**Still unknown:** Whether the SE port's volume is ever below full when
`sound 8 0 256` runs was not looked for. The host-call check plays blocks
in isolation, so a method reached only through state an earlier block
sets (a question's answer, a wait's result) may not show. The places come
from tags and conditions, not from running the story.
