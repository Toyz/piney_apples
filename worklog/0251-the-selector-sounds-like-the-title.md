---
number: 251
title: The selector sounds like the title
date: 2026-09-27
area: ui
files: crates/piney-game/src/launcher.rs, README.md
---

# 251. The selector sounds like the title

Asked for from play: the launcher's game selection should use the sounds
of one of the boot screens.

## What was there

The launcher asked for the title screen's cursor, OK and refusal sounds
(6, 4 and 20), but no sound bank was loaded, so nothing played. The
selector's own scene (`STRT.BIN`'s `trial_v2st`) carries no sound either.
Stepped for 600 frames it asks for nothing but a movie-audio stop and a
voice stop: no stream PCM and no music.

## What it does now

Once the logos are over (or a push stops them), the launcher does for the
selector what the title does after its logos (`LogoMain`, LogoAct 4):
1. all sound off;
2. the title's bank loaded (`SqContext::Title`) from the launcher's disc,
   which is Outbreak's when the build has it;
3. the main volume set, taken from a new save's `mainVol`;
4. sequence 0, the title's music, started.

The cursor, OK and refusal sounds then come from that bank. Choosing a
part replaces the sound with that disc's, as before.

`selector_sounds_like_the_title` checks both sides against headless
audio:
- the logos render only silence;
- after a push skips them, the selector is heard on more than 100 of the
  next 250 frames, with two cursor moves in them.

## The third logo

Asked for from play as well: the Project .hack logo (`LOGO_H.PSS`) now
plays in the launcher too, after Bandai's and CyberConnect2's, and
`launcher::LOGOS` has all three. The chosen part's title is given
`logos_played` 3 (worklog 249), so it opens on its own opening movie
(Mutation's `OPENING2.PSS` when checked headless).

**Still unknown:** Whether the unused selector was meant to have music of
its own. No code for it survives on the demo discs or on Outbreak.
