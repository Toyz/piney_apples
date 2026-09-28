---
number: 189
title: "The ML events open in Infection: twelve of the members' mail events, Black Rose's run on the desktop"
date: 2026-09-26
area: script
files: crates/piney-game/src/desktop.rs, docs/engine/events.md
---

# 189. The ML events open in Infection: twelve of the members' mail events, Black Rose's run on the desktop

`events.md` listed "whether the ML events open in Infection" as unknown.
The ML group (401-455) is on Infection's disc, all 55 scripts. Each is a
member's mail thread:

- It opens once a story event is done, on the desktop (`game_status` 2).
- Its blocks wait for the desktop's set-up (`phase >= 4`).
- Each block sends the next mail as the member's friendship passes a
  mark (50, 75, 100, ...) and adds 10 to it.
- The reply to the last mail (`mail_5` or `mail_6`) picks the branch.

Twelve open on Infection's events alone:

| member | events | after |
| --- | --- | --- |
| Black Rose | 401, 405 | 14, 23 |
| Mistral | 407, 410 | 18, 31 |
| Piros | 417, 421 | 15, 31 |
| Sanjuro | 422, 426 | 58, then 31 |
| Gardenia | 427, 431 | 59, then 31 |
| Natsume | 432, 436 | 61, then 31 |

The rest wait on events 102, 113, 155-162, 210, 219, 262, 304 and 310, of
the later volumes.

The walk (`event-vm.md`, the gate) reaches them each pass after S1 to S4,
never in Parody Mode. A first test had only events 0-14 done: event 15
then opened, its block 2 held the pass, and 401 was never reached. The
game's `eventSub` walk, which the port's VM follows (the VM fixture
replays it), does the same.

`black_rose_writes_when_friendly` (piney-game) marks Infection's events
0-99 done, sets Black Rose's friendship (row 15) and runs the desktop:

- at 50, mail 85 comes: state 1, then 2 once the mailer icon has seen it;
- at 49, nothing comes.

**Still unknown:** Whether a player of Infection alone reaches 50 with
Black Rose by event 14's time. Friendship grows with time in the party
(+1 each time the member's `partyTime` reaches a multiple of 7,560) and
by the scripts' own additions; no play of the game was measured.
