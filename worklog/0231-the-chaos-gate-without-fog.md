---
number: 231
title: The Chaos Gate without fog
date: 2026-09-27
area: render
files: crates/piney-world/src/gate.rs
---

# 231. The Chaos Gate without fog

Reported from play in Mutation's Dun Loireag: a light blue translucent
square stood behind the Chaos Gate's ring.

## Finding it

The town was drawn from the far side of the gate with kinds of pieces
hidden in turn (a throwaway switch, since removed):
- The hazy patches around the platform are the town's 25 `CLOUD` puffs
  (`EFF_srzsmo1`). Alone they draw as soft round puffs, since the
  texture's alpha falls to 0 at the corners.
- The square went only with the gate hidden, and then only with the fog
  off. The viewer, which draws `CHGATE` unfogged, shows a clean oval.

## The cause

`ccChgate` sets both of its anms (the gate `CMP_xmgtwav0` and the circle
`CMP_xmgtcir0`) to `SetFogSw(0)`. The port drew them with the town's depth
fog. The portal's plane blends additively with a black border. The fog
mixed the town's fog colour (Dun Loireag's 0xf0c080, a light blue) into
that border, so the whole square showed. The GS mixes fog before
blending, as piney-gs does. Nothing was wrong in the blend itself; the
fog was never meant to be on.

`Gate::draw` now draws both anms with no fog. Infection's towns use the
same gate, so they had the same square. The town's own `SetFogSw(0)`
pieces (the sky, the cloud layers, the sun, the water) were already
unfogged.

**Still unknown:** Whether other gimmicks or objects that set
`SetFogSw(0)` are still drawn fogged; only the gate was checked.
