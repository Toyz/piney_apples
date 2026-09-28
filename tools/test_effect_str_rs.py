#!/usr/bin/env python3
"""crates/piney-effect's stream demo effects (`strfx`: `ccEffectCtrl(1)`'s
`MainStr`, `effHitMarkStr`, `effTransferStr`, and the second particle
system's `ccParticle::MainStr`) against the game's own code run in eemu.

The hit_probe example runs the port on the requests this sends it
(`strreset`, `strhit`, `strtransfer`, `frame`); the same requests go
through the game's functions:

  - StrMachine: tools/test_effect_hit_rs.py's HitMachine, plus the stream
    demo's two systems as `ccThEffectStr` and the stream's set-up make
    them: `ccEffectCtrl(1)` built by its constructor (`effcStr`, the 50
    slots of `effStrWork`, `effectStrTbl`'s objects and the hit marks'
    palettes) and a `ccParticleCtrl(1)` (`particleSystemStr`, the 150
    particles of `particlesStr`) with `particleThreadStrFlag` set, so every
    generator started goes to it, as while a stream plays.
  - StreamEffectsAgainstGame: random hit marks (`effHitMarkStr` at random
    places, turns and layers) and transfers (`effTransferStr` with random
    heights), with frames between; each frame `ccEffectCtrl::MainStr` then
    the second system's `ccParticleCtrl::Main`, as `ccThEffectStr` (80) and
    `ccThParticle` (98) run them: every live slot's whole state, the
    effects' and the particles' draws, every generator's and live
    particle's state, the control's counts and rand's state.

Skipped when the disc is not extracted or cargo is missing.
"""

import math
import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

from test_effect_rs import ELF, ISO, LAYER_ACTIVE, ONE, Rand, ask, build, compare, fb, hexs  # noqa: E402
from test_effect_particle_rs import diff as particle_diff  # noqa: E402
from test_effect_spell_rs import Genrand  # noqa: E402
from test_effect_hit_rs import EXAMPLE, HitMachine  # noqa: E402

# INF main's stream-demo globals and arrays.
EFFC_STR, EFF_STR_WORK, STR_SLOTS = inf_va(0x00378AD4), inf_va(0x003FC930), 50
PARTICLE_SYSTEM_STR, PARTICLE_THREAD_STR_FLAG = inf_va(0x00378AA4), inf_va(0x00378AB0)
PARTICLES_STR, STR_PARTICLES = inf_va(0x003DDF60), 150
# The layer the streams' tasks start their effects on (STR_EFF_LAYER).
STR_LAYER = 20


class StrMachine(HitMachine):
    """HitMachine with the stream demo's systems, as the module docstring
    says."""

    def __init__(self, seed=1):
        super().__init__(town=False, seed=seed)
        m = self.m
        self.effc_str = self.malloc(m, 0x2C0)
        self.call("__ct__12ccEffectCtrlFi", self.effc_str, 1)
        m.store(EFFC_STR, 4, self.effc_str)
        self.pcs = self.malloc(m, 0x18)
        self.call("__ct__14ccParticleCtrlFi", self.pcs, 1)
        m.store(PARTICLE_SYSTEM_STR, 4, self.pcs)
        m.store(PARTICLE_THREAD_STR_FLAG, 2, 1)
        # Every generator's list, count and serial is the second system's.
        self.pc = self.pcs
        # The probe's `strreset SEED` seeds its rand and genrand afresh.
        self.rand = Rand(seed)
        m.hooks[self.sym("rand")] = self.rand.next
        self.genrand = Genrand(seed)
        m.hooks[self.sym("genrand__Fv")] = self.genrand.next
        self.str_layer = self.layer_of(STR_LAYER)

    def live(self, base=EFF_STR_WORK, n=STR_SLOTS):
        return super().live(base, n)

    def part_list(self, base=PARTICLES_STR, n=STR_PARTICLES):
        return super().part_list(base, n)

    def vref(self, p):
        if p and EFF_STR_WORK <= p < EFF_STR_WORK + 0xC0 * STR_SLOTS:
            k, off = divmod(p - EFF_STR_WORK, 0xC0)
            return [{0: "effpos", 0x20: "effrot", 0x50: "effposT"}[off], k]
        return super().vref(p)

    def layer_ptr(self, pri):
        return 0 if pri is None else self.layer_of(pri)

    def hit(self, pos, rot, layer):
        pa, ra = self.malloc(self.m, 16), self.malloc(self.m, 16)
        self.vec(pa, [fb(v) for v in pos] + [ONE])
        self.vec(ra, [fb(v) for v in rot] + [0])
        self.call("effHitMarkStr__FPfPfP7ccLayer", pa, ra, self.layer_ptr(layer))
        return self.extras()

    def transfer(self, pos, height, layer):
        pa = self.malloc(self.m, 16)
        self.vec(pa, [fb(v) for v in pos] + [ONE])
        self.call("effTransferStr__FPffP7ccLayer", pa, self.layer_ptr(layer), fargs=(fb(height),))
        return self.extras()

    def frame(self):
        """ccThEffectStr's frame, then ccThParticle's second system: the
        effects' sprites recorded as EffectMachine records them, the
        particles' with their CLUT; an effect with no layer draws on the
        active one, the streams' effect layer."""
        m = self.m
        self.draws, self.events = [], []
        m.store(LAYER_ACTIVE, 4, self.str_layer)
        m.hooks[self.draw_eff] = self.eff_draw
        self.call("MainStr__12ccEffectCtrlFv", self.effc_str)
        draws, self.draws = self.draws, []
        m.hooks[self.draw_eff] = self.eff_draw2
        self.call("Main__14ccParticleCtrlFv", self.pcs)
        pdraws, self.draws = self.draws, []
        out = {"slots": self.live(), "draws": draws, "pdraws": pdraws}
        out.update(self.extras())
        out["rand"] = self.rand.s
        return out


class Script:
    """The same requests to the game (now) and the probe (at the end)."""

    def __init__(self, test, seed):
        self.test = test
        self.sm = StrMachine(seed=seed)
        self.lines = ["reset 0 %x" % seed, "strreset %x" % seed]
        self.want = [("strreset", {})]

    def put(self, line, answer):
        self.lines.append(line)
        self.want.append((line, answer))

    def hit(self, pos, rot, layer):
        lay = 0xFFFFFFFF if layer is None else layer & 0xFFFFFFFF
        self.put("strhit " + hexs(*map(fb, list(pos) + list(rot))) + " %x" % lay, self.sm.hit(pos, rot, layer))

    def transfer(self, pos, height, layer):
        lay = 0xFFFFFFFF if layer is None else layer & 0xFFFFFFFF
        self.put("strtransfer " + hexs(*map(fb, list(pos) + [height])) + " %x" % lay,
                 self.sm.transfer(pos, height, layer))

    def frame(self):
        self.put("frame", self.sm.frame())

    def check(self, what):
        got = list(ask(self.lines, EXAMPLE))
        # One answer a line: the reset's and each request's.
        self.test.assertEqual(len(got), len(self.want) + 1)
        for (line, want), port in zip(self.want, got[1:]):
            d = particle_diff({k: want[k] for k in ("gens", "parts", "pdraws") if k in want}, port,
                              "%s: %s" % (what, line))
            if d:
                self.test.fail(d)
            compare(self.test, want, port, "%s: %s" % (what, line))
        return self.want


def busy(sm):
    m = sm.m
    slots = any(m.load(EFF_STR_WORK + 0xC0 * i + 0x6E, 2) for i in range(STR_SLOTS))
    return slots or m.load(sm.pcs, 4) != 0 or bool(sm.part_list())


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class StreamEffectsAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build("hit_probe")

    def run_case(self, seed, starts=8):
        rng = random.Random(seed)
        sc = Script(self, seed)
        frames = 0
        centre = [rng.uniform(-3000, 3000), rng.uniform(-3000, 3000), rng.uniform(-100, 300)]
        for _ in range(starts):
            pos = [centre[k] + rng.uniform(-400, 400) for k in range(3)]
            layer = rng.choice([STR_LAYER, STR_LAYER, 60, None])
            if rng.random() < 0.6:
                rot = [rng.uniform(-math.pi, math.pi) for _ in range(3)]
                sc.hit(pos, rot, layer)
            else:
                sc.transfer(pos, rng.choice([160.0, 210.0, rng.uniform(0, 300)]), layer)
            for _ in range(rng.randrange(0, 12)):
                sc.frame()
                frames += 1
        while busy(sc.sm) and frames < 400:
            sc.frame()
            frames += 1
        for _ in range(5):
            sc.frame()
            frames += 1
        want = sc.check("stream effects seed %d" % seed)
        draws = sum(len(w.get("draws", [])) for _, w in want)
        pdraws = sum(len(w.get("pdraws", [])) for _, w in want)
        parts = sum(len(w.get("parts", [])) for _, w in want)
        return frames, draws, pdraws, parts

    def test_hit_marks_and_transfers(self):
        total = [0, 0, 0, 0]
        for seed in range(30):
            for k, v in enumerate(self.run_case(seed)):
                total[k] += v
        print("stream effects: 30 cases, %d frames, %d effect draws, %d particle draws, %d particle states"
              % tuple(total), file=sys.stderr)
        self.assertGreater(total[1], 500)
        self.assertGreater(total[2], 500)


if __name__ == "__main__":
    unittest.main()
