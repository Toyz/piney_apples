#!/usr/bin/env python3
"""crates/piney-effect's hits, fly fonts and stacked numbers against the
game's own code run in eemu (tools/test_effect_particle_rs.py's ParticleMachine).

The hit_probe example runs the port on the requests this sends it; the same
requests go through the game's functions:

  - HitMachine: tools/test_effect_particle_rs.py's ParticleMachine (the
    EffectMachine with the game's own particle system: ccParticleCtrl built
    by its constructor, effect.cpp's static generators, the real
    ccParticleHitMark, ccParticleSetup and generators) in a field, plus what
    the hit code reaches for: ccCheckTarget, ccCheckObjectSize,
    checkPartyMenberNum, ccConditionIconNum and CheckCharAttribute answered
    per character; ccClump::Duplicate a no-op and ChangeClut recorded;
    sysLayer's view with a world-to-screen matrix, camID and the active
    camera's type; the global `font` a real ccFont (ccSprite's constructor,
    SetPrim(160, 0)) on a layer with the menu layer's view, with a packet
    buffer, so ccSprite::MakePacketStr runs as it is and the quads it
    writes are read back.
  - HitsAgainstGame: random hits (ccHitMarkDisp between random characters,
    a character on itself too) with frames of ccEffectCtrl::Main until
    every effect has ended - each frame ccEffectCtrl::Main then
    ccParticleCtrl::Main, as ccThEffect and ccThParticle run: every slot's
    whole state, the effects' and the particles' draws, the sounds, every
    generator's and every live particle's whole state (the hit marks'
    generators, the photons), the control's counts and rand's state.
  - ProtectAgainstGame: effProtect, ccParticleAttributeGuard (with the
    guard list), ccParticleAttributeCritical, ccParticleCritical,
    ccParticleDying and ccParticleNoDamage (their generators, words and
    camera shakes) over sizes, kinds, elements and repeats, frames until
    done.
  - FlyFontsAgainstGame: ccEntryFlyFontNum and the ccEntryFlyFontNew*
    functions at random with ccMenuCtrl::Disp's calls (ccCtrlFlyFont,
    CtrlAll, DrawAll) frame by frame, paused or not: every fly-font entry,
    every ccDamUprStr node and line, flyFont's fields and the quads
    MakePacketStr wrote (the drawing), each frame.

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

from test_effect_rs import ELF, EFFWORK, ISO, ONE, PROFILE, TARGET, ask, build, compare, fb, hexs  # noqa: E402
from test_effect_particle_rs import P2NUM, SERIALS, ParticleMachine, diff as particle_diff  # noqa: E402

EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "hit_probe")

# gp globals and fixed objects (INF main, gcmn.prg).
FONT_G, FONT_TEX, SYSLAYER_VIEW = inf_va(0x00378954), inf_va(0x00378960), inf_va(0x00379B3C)
ROOT, FLY_CTRL, GUARD = inf_va(0x00378C80), inf_va(0x0072EBD0), inf_va(0x003FEEB0)
PARTICLE_GENERATOR_TBL = inf_va(0x003402F0)
CAMID, ACTIVE_CAM = inf_va(0x0037897C), inf_va(0x0037896C)
# hitPhotonDummyG's serial number (effect.cpp's third static generator) and
# the photon's texture id.
HIT_PHOTON_SN, PHOTON_TEX = 2, 102
# Scratch for the harness's own objects.
VIEW3D, FONT, FLAYER, FVIEW, PKT = 0x01E00000, 0x01E01000, 0x01E02000, 0x01E03000, 0x01E10000
# The menu layer's view (SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)):
# x 16 + 28672.5, y 16 + 29184.5.
MENU_VIEW = (16.0, 16.0, 28672.5, 29184.5)


def s32(v):
    return v - (1 << 32) if v & 0x80000000 else v


def s16(v):
    return v - 0x10000 if v & 0x8000 else v


def cstr(m, a, n):
    out = []
    for k in range(n):
        b = m.load(a + k, 1)
        if b == 0:
            break
        out.append(b)
    return out


def world_screen(rng, eye, look, focal=None):
    """A perspective world-to-screen matrix (columns) as floats: x and y
    around (2048, 2048), w the depth along the view, z 65536 w - 4194304."""
    f = [look[i] - eye[i] for i in range(3)]
    n = math.sqrt(sum(x * x for x in f)) or 1.0
    f = [x / n for x in f]
    up = [0.0, 0.0, 1.0]
    r = [f[1] * up[2] - f[2] * up[1], f[2] * up[0] - f[0] * up[2], f[0] * up[1] - f[1] * up[0]]
    n = math.sqrt(sum(x * x for x in r)) or 1.0
    r = [x / n for x in r]
    u = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]]
    focal = focal or rng.uniform(250, 700)
    dot = lambda a, b: sum(a[i] * b[i] for i in range(3))            # noqa: E731
    w = f + [-dot(f, eye)]
    x = [focal * r[i] + 2048 * f[i] for i in range(3)] + [-focal * dot(r, eye) + 2048 * w[3]]
    y = [-focal * u[i] + 2048 * f[i] for i in range(3)] + [focal * dot(u, eye) + 2048 * w[3]]
    z = [65536 * f[i] for i in range(3)] + [65536 * w[3] - 4194304]
    cols = [[x[k], y[k], z[k], w[k]] for k in range(4)]
    return [c for col in cols for c in col]


class HitMachine(ParticleMachine):
    """ParticleMachine with the hits', fly fonts' and stacked numbers'
    surroundings, as the module docstring says."""

    def __init__(self, town=False, seed=1):
        super().__init__(town=town, seed=seed)
        m, sym = self.m, self.sym
        self.alive, self.size, self.slot_of, self.icons, self.attr = {}, {}, {}, {}, {}
        m.hooks[sym("ccCheckTarget__FP6ccChar")] = \
            lambda mm, ch, *a: 1 if self.alive.get(self.char_of.get(ch)) else 0
        m.hooks[sym("ccCheckObjectSize__FP6ccChar")] = \
            lambda mm, ch, *a: self.size.get(self.char_of.get(ch), 0) & 0xFFFFFFFF
        m.hooks[sym("checkPartyMenberNum__Fi")] = lambda mm, cid, *a: self.slot_of.get(s16(cid & 0xFFFF), -1) & 0xFFFFFFFF
        m.hooks[sym("ccConditionIconNum__FP6ccChar")] = \
            lambda mm, ch, *a: self.icons.get(self.char_of.get(ch), 0) & 0xFFFFFFFF
        m.hooks[sym("CheckCharAttribute__6ccCharFi")] = \
            lambda mm, ch, *a: self.attr.get(self.char_of.get(ch), -1) & 0xFFFFFFFF
        m.hooks[sym("cameraShake__Fiiii")] = \
            lambda mm, a, b, c, d: self.events.append(["shake", s32(a), s32(b), s32(c), s32(d)]) or 0
        self.draw_eff = sym("Draw__5ccEffFUs")
        # sysLayer's view, the camera's id and type.
        m.store(SYSLAYER_VIEW, 4, VIEW3D)
        m.store(CAMID, 2, 0)
        # ccDamUprStr's list, empty; the fly fonts, free.
        m.store(ROOT, 4, ROOT)
        m.store(ROOT + 4, 4, ROOT)
        self.call("ccInitFlyFont__Fv")
        # flyFont: a ccFont of 160 packets on a layer with the menu's view.
        self.call("__ct__8ccSpriteFv", FONT)
        self.call("SetPrim__8ccSpriteFii", FONT, 160, 0)
        m.store(FONT + 0x2C, 4, PKT)
        m.store(FONT + 0xA8, 4, FLAYER)
        m.store(FONT + 0xB0, 8, 8 << 30)
        m.store(FLAYER + 0x2C, 4, FVIEW)
        sx, sy, ox, oy = MENU_VIEW
        for k, v in enumerate([sx, 0, 0, 0, 0, sy, 0, 0, 0, 0, 1.0, 0, ox, oy, 0, 1.0]):
            m.store(FVIEW + 0x190 + 4 * k, 4, fb(v))
        m.store(FONT_G, 4, FONT)
        m.store(FONT_TEX, 4, 0x01E0F000)

    # the world ------------------------------------------------------------
    def char(self, cid, pos, height, width, dirc, kind, slot, icons, size, alive, attribute=-1):
        if cid in self.chars:
            ch = self.chars[cid]
            self.vec(ch + 0x40, [fb(v) for v in pos] + [ONE])
            self.vec(ch + 0x60, [fb(v) for v in dirc] + [0])
            param = ch + 0x100
            self.m.store(param + 0x18, 4, fb(height))
            self.m.store(param + 0x1C, 4, fb(width))
        else:
            ch = self.add_char(cid, pos, height, width, dirc)
        param = ch + 0x100
        self.m.store(param + 0x08, 4, kind & 0xFFFFFFFF)
        self.m.store(param + 0x0C, 2, cid & 0xFFFF)
        self.alive[cid], self.size[cid], self.slot_of[cid], self.icons[cid] = alive, size, slot, icons
        self.attr[cid] = attribute

    def screen(self, mat):
        for k, v in enumerate(mat):
            self.m.store(VIEW3D + 0xD0 + 4 * k, 4, v)

    def camid(self, cid, ctype):
        self.m.store(CAMID, 2, cid & 0xFFFF)
        self.m.store(0x01023000 + 0x5C, 4, ctype & 0xFFFFFFFF)

    # what is compared -------------------------------------------------------
    def slot(self, i, base=EFFWORK, n=500):
        d = super().slot(i, base, n)
        eff = self.m.load(base + 0xC0 * i + 0x8C, 4)
        # ParticleMachine records ChangeClut as (new, old); a slot's swap is
        # (from, to).
        c = self.cluts.get(eff)
        d["clut"] = [c[1], c[0]] if c and d["obj"] and d["obj"][0] == "clump" else None
        is_eff = d["obj"] is not None and d["obj"][0] == "eff"
        d["efftest"] = self.m.load(eff + 0x58, 8) if is_eff else None
        # The sprite's own palette: ccEff +0x3c (ccTex.clutChunk), by name.
        d["effclut"] = self.names.get(self.m.load(eff + 0x3C, 4)) if is_eff else None
        return d

    def who(self, ch):
        return self.char_of.get(ch, -1 if ch == 0 else ch)

    def font(self):
        m = self.m
        return [s32(m.load(FONT + 4, 4)), m.load(FONT + 0x30, 4), m.load(FONT + 0x34, 4), m.load(FONT + 0x38, 4),
                m.load(FONT + 0x40, 4), m.load(FONT + 0x44, 4), m.load(FONT + 0x48, 4), m.load(FONT + 0x4C, 4),
                s32(m.load(FONT + 0x50, 4)), s32(m.load(FONT + 0x54, 4)), s32(m.load(FONT + 0x58, 4)),
                s32(m.load(FONT + 0x5C, 4)), s32(m.load(FONT + 0x60, 4)),
                [m.load(FONT + 0x68 + 4 * k, 4) for k in range(4)], s32(m.load(FONT + 0x20, 4))]

    def extras(self):
        """The events since the last answer, the particle system's whole
        state, the guard list and flyFont's fields."""
        m = self.m
        out = {"events": self.events, "gens": self.gen_list(), "parts": self.part_list(),
               "ctrl": [m.load(self.pc + 12, 2, True), m.load(self.pc + 14, 2, True), m.load(self.pc + 20, 4),
                        m.load(SERIALS[0], 4), m.load(SERIALS[1], 4), m.load(P2NUM, 2)],
               "guard": [self.who(m.load(GUARD + 4 * k, 4)) for k in range(20)], "font": self.font()}
        self.events = []
        return out

    def frame(self):
        """ccThEffect's frame, then ccThParticle's: the effects' sprites are
        recorded as EffectMachine records them, the particles' with their
        CLUT (ParticleMachine's)."""
        m = self.m
        self.draws, self.events = [], []
        m.hooks[self.draw_eff] = self.eff_draw
        self.call("Main__12ccEffectCtrlFv", self.effc)
        draws, self.draws = self.draws, []
        m.hooks[self.draw_eff] = self.eff_draw2
        self.call("Main__14ccParticleCtrlFv", self.pc)
        pdraws, self.draws = self.draws, []
        out = {"slots": self.live(), "draws": draws, "pdraws": pdraws}
        out.update(self.extras())
        out["rand"] = self.rand.s
        return out

    def fly_state(self):
        m = self.m
        entries = []
        for k in range(16):
            e = FLY_CTRL + 0x40 * k
            entries.append([s32(m.load(e, 4)), s32(m.load(e + 4, 4)), s32(m.load(e + 8, 4)), self.rvec(e + 0x10),
                            cstr(m, e + 0x20, 16), s32(m.load(e + 0x30, 4)), m.load(e + 0x34, 4),
                            m.load(e + 0x38, 4)])
        return entries

    def dam_state(self):
        m = self.m
        nodes = []
        n = m.load(ROOT + 4, 4)
        while n != ROOT:
            u = n + 0xC
            lines = []
            for k in range(16):
                a = u + 0x2C * k
                lines.append([cstr(m, a, 16), s32(m.load(a + 0x10, 4)), s32(m.load(a + 0x14, 4)),
                              s16(m.load(a + 0x18, 2)), s16(m.load(a + 0x1A, 2)), s16(m.load(a + 0x1C, 2)),
                              s16(m.load(a + 0x1E, 2)), s16(m.load(a + 0x20, 2)), m.load(a + 0x24, 4),
                              m.load(a + 0x28, 4)])
            nodes.append([self.who(m.load(n + 8, 4)), lines, s16(m.load(u + 0x2C0, 2)), s16(m.load(u + 0x2C2, 2)),
                          s16(m.load(u + 0x2C4, 2)), s16(m.load(u + 0x2C6, 2)), m.load(u + 0x2C8, 1)])
            n = m.load(n + 4, 4)
        return nodes

    def quads(self):
        """The SPRITEs MakePacketStr wrote since the last send (six qwords
        from +176 each: TEX0 RGBAQ UV XYZ2 UV XYZ2), then the send."""
        m = self.m
        n = m.load(FONT + 0x20, 4)
        out = []
        for k in range(n):
            q = PKT + 176 + 96 * k
            w = [m.load(q + 16 * j + 4 * i, 4) for j in range(6) for i in range(4)]
            out.append([w[4:8], s32(w[8]), s32(w[9]), s32(w[12]), s32(w[13]), s32(w[16]), s32(w[17]),
                        s32(w[20]), s32(w[21])])
        m.store(FONT + 0x20, 4, 0)
        return out

    def fly(self, pause, over):
        """ccMenuCtrl::Disp's numbers (gcmn 0x0052170c-0x00521754)."""
        self.call("ccCtrlFlyFont__Fi", int(pause))
        if not pause:
            self.call("CtrlAll__11ccDamUprStrFv")
            if not over:
                self.call("DrawAll__11ccDamUprStrFv")
        out = {"fly": self.fly_state(), "dam": self.dam_state(), "quads": self.quads()}
        out.update(self.extras())
        return out


class Script:
    """The same requests to the game (now) and the probe (at the end)."""

    def __init__(self, test, seed, town=False):
        self.test = test
        self.hm = HitMachine(town=town, seed=seed)
        self.lines = ["reset %x %x" % (int(town), seed)]
        self.want = []

    def put(self, line, answer=None):
        self.lines.append(line)
        self.want.append((line, answer if answer is not None else {}))

    def player(self, pos):
        self.hm.set_player(pos)
        self.put("player " + hexs(*map(fb, pos)))

    def camera(self, eye, look, rng):
        self.hm.set_camera(eye, eye, look)
        self.put("camera " + hexs(*map(fb, eye + eye + look)))
        mat = [fb(v) for v in world_screen(rng, eye, look)]
        self.hm.screen(mat)
        self.put("screen " + hexs(*mat))

    def camid(self, cid, ctype):
        self.hm.camid(cid, ctype)
        self.put("camid %x %x" % (cid & 0xFFFFFFFF, ctype & 0xFFFFFFFF))

    def char(self, cid, pos, height, width, dirc, kind=0, slot=-1, icons=0, size=0, alive=True, attribute=-1):
        self.hm.char(cid, pos, height, width, dirc, kind, slot, icons, size, alive, attribute)
        self.put("char %x " % cid + hexs(*map(fb, list(pos) + [height, width] + list(dirc)))
                 + " %x %x %x %x %x %x" % (kind & 0xFFFFFFFF, slot & 0xFFFFFFFF, icons, size & 0xFFFFFFFF, int(alive),
                                           attribute & 0xFFFFFFFF))

    def ch(self, cid):
        return self.hm.chars[cid] if cid is not None else 0

    def hit(self, a, b):
        self.hm.call("ccHitMarkDisp__FP6ccCharP6ccChar", self.ch(a), self.ch(b))
        self.put("hit %x %x" % (a, b), self.hm.extras())

    def protect(self, c, broken, kind):
        p = self.hm.call("effProtect__FP6ccCharii", self.ch(c), broken, kind & 0xFFFFFFFF)
        ans = {"slot": -1 if p == 0 else (p - EFFWORK) // 0xC0}
        ans.update(self.hm.extras())
        self.put("protect %x %x %x" % (c, broken, kind & 0xFFFFFFFF), ans)

    WORDS = {"critical": "ccParticleAttributeCritical__FP6ccChar", "crit": "ccParticleCritical__FP6ccChar",
             "dying": "ccParticleDying__FP6ccChar", "nodamage": "ccParticleNoDamage__FP6ccChar"}

    def critical(self, c, what="critical"):
        before = [self.hm.m.load(EFFWORK + 0xC0 * i + 0x6E, 2) for i in range(500)]
        self.hm.call(self.WORDS[what], self.ch(c))
        made = [i for i in range(500) if not before[i] and self.hm.m.load(EFFWORK + 0xC0 * i + 0x6E, 2)]
        ans = {"slot": made[0] if made else -1}
        ans.update(self.hm.extras())
        self.put("%s %x" % (what, c), ans)

    def guard(self, c, att):
        before = [self.hm.m.load(EFFWORK + 0xC0 * i + 0x6E, 2) for i in range(500)]
        self.hm.call("ccParticleAttributeGuard__FP6ccCharP6ccChar", self.ch(c), self.ch(att))
        made = [i for i in range(500) if not before[i] and self.hm.m.load(EFFWORK + 0xC0 * i + 0x6E, 2)]
        ans = {"slot": made[0] if made else -1}
        ans.update(self.hm.extras())
        self.put("guard %x %x" % (c, att), ans)

    def flynum(self, color, num, pos, sx, sy, ofs):
        pa = self.hm.malloc(self.hm.m, 16)
        self.hm.vec(pa, [fb(v) for v in pos] + [ONE])
        before = [self.hm.m.load(FLY_CTRL + 0x40 * k + 4, 4) for k in range(16)]
        self.hm.call("ccEntryFlyFontNum__FiiPfffi", color, num & 0xFFFFFFFF, pa, ofs & 0xFFFFFFFF,
                     fargs=(fb(sx), fb(sy)))
        after = [self.hm.m.load(FLY_CTRL + 0x40 * k + 4, 4) for k in range(16)]
        slot = next((k for k in range(16) if before[k] == 0 and after[k] != 0), -1)
        ans = {"slot": slot, "fly": self.hm.fly_state()}
        ans.update(self.hm.extras())
        self.put("flynum %x %x " % (color, num & 0xFFFFFFFF) + hexs(*map(fb, pos)) + " "
                 + hexs(fb(sx), fb(sy)) + " %x" % (ofs & 0xFFFFFFFF), ans)

    def fly_entry(self, kind, what, value=0, cid=None):
        pa = self.hm.malloc(self.hm.m, 16)
        ch = self.ch(cid)
        name = {"new": "ccEntryFlyFontNew__FiiPfP6ccCharff", "exp": "ccEntryFlyFontNewExp__FiiPfP6ccChar",
                "level": "ccEntryFlyFontNewLevelDown__FiPfP6ccChar", "miss": "ccEntryFlyFontNewMiss__FPfP6ccChar"}
        c = (cid if cid is not None else -1) & 0xFFFFFFFF
        if what == "new":
            node = self.hm.call(name[what], kind, value & 0xFFFFFFFF, pa, ch, fargs=(ONE, ONE))
            line = "flynew %x %x %x" % (kind, value & 0xFFFFFFFF, c)
        elif what == "exp":
            node = self.hm.call(name[what], kind, value & 0xFFFFFFFF, pa, ch)
            line = "flyexp %x %x %x" % (kind, value & 0xFFFFFFFF, c)
        elif what == "level":
            node = self.hm.call(name[what], kind, pa, ch)
            line = "flylevel %x %x" % (kind, c)
        else:
            node = self.hm.call(name[what], pa, ch)
            line = "flymiss %x" % c
        dam = self.hm.dam_state()
        # The node's index in the list (the functions but New return void).
        idx = next((k for k, n in enumerate(dam) if n[0] == (cid if cid is not None else -1)), -1)
        del node
        ans = {"node": idx, "dam": dam}
        ans.update(self.hm.extras())
        self.put(line, ans)

    def frame(self):
        self.put("frame", self.hm.frame())

    def fly(self, pause=False, over=False):
        self.put("fly %x %x" % (int(pause), int(over)), self.hm.fly(pause, over))

    def check(self, what):
        got = [g for g in ask(self.lines, EXAMPLE)]
        self.test.assertEqual(len(got), len(self.want) + 1)
        for (line, want), port in zip(self.want, got[1:]):
            d = particle_diff({k: want[k] for k in ("gens", "parts", "pdraws") if k in want}, port,
                              "%s: %s" % (what, line))
            if d:
                self.test.fail(d)
            compare(self.test, want, port, "%s: %s" % (what, line))
        return self.want


def live_slots(hm):
    return sum(1 for i in range(500) if hm.m.load(EFFWORK + 0xC0 * i + 0x6E, 2))


def angle(rng):
    return [0.0, 0.0, rng.uniform(-math.pi, math.pi)]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class HitsAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build("hit_probe")

    def run_hits(self, seed, hits=12, extra_frames=20):
        rng = random.Random(seed)
        sc = Script(self, seed)
        player = [rng.uniform(-3000, 3000), rng.uniform(-3000, 3000), rng.uniform(0, 400)]
        sc.player(player)
        eye = [player[0] + rng.uniform(-900, 900), player[1] + rng.uniform(-900, 900), player[2] + 500]
        sc.camera(eye, player, rng)
        chars = list(range(1, 7))

        def place(c, alive=True):
            pos = [player[0] + rng.uniform(-600, 600), player[1] + rng.uniform(-600, 600),
                   player[2] + rng.uniform(-50, 50)]
            sc.char(c, pos, rng.uniform(40, 400), rng.uniform(5, 120), angle(rng), alive=alive)
        for c in chars:
            place(c)
        frames = 0
        for k in range(hits):
            a, b = rng.choice(chars), rng.choice(chars)
            if rng.random() < 0.15:
                b = a
            if rng.random() < 0.15:
                place(a)
            if rng.random() < 0.05:
                place(rng.choice([a, b]), alive=False)
            sc.hit(a, b)
            if not sc.hm.alive[a] or not sc.hm.alive[b]:
                place(a)
                place(b)
            for _ in range(rng.randrange(0, 4)):
                sc.frame()
                frames += 1
        while live_slots(sc.hm) and frames < 400:
            sc.frame()
            frames += 1
        for _ in range(extra_frames):
            sc.frame()
            frames += 1
        want = sc.check("hits seed %d" % seed)
        rings = sum(1 for _, w in want for d in w.get("draws", []) if d[0] == "clump" and d[1] == "CMP_x037")
        # The photons and the hit marks' generators, each once however many
        # frames it lives.
        parts = len({p["sn"] for _, w in want for p in w.get("parts", [])
                     if p["texID"] == PHOTON_TEX and p["gene"] == HIT_PHOTON_SN})
        marks = len({g["sn"] for _, w in want for g in w.get("gens", [])
                     if g["param"] == PARTICLE_GENERATOR_TBL + 0x38})
        self.pstates += sum(len(w.get("parts", [])) for _, w in want)
        self.pdraws += sum(len(w.get("pdraws", [])) for _, w in want)
        return frames, rings, parts, marks

    def test_random_hits(self):
        total = [0, 0, 0, 0]
        self.pstates = self.pdraws = 0
        for seed in range(40):
            for k, v in enumerate(self.run_hits(seed)):
                total[k] += v
        print("hits: 40 cases, %d frames, %d ring draws, %d photons, %d hit marks" % tuple(total), file=sys.stderr)
        print("hits: %d particle states, %d particle draws" % (self.pstates, self.pdraws), file=sys.stderr)
        self.assertGreater(total[1], 500)
        self.assertGreater(total[2], 1000)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ProtectAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build("hit_probe")

    def run_case(self, seed):
        rng = random.Random(seed)
        sc = Script(self, seed)
        player = [rng.uniform(-3000, 3000), rng.uniform(-3000, 3000), 0.0]
        sc.player(player)
        far = 900 if rng.random() < 0.5 else 2400
        eye = [player[0] + rng.uniform(-far, far), player[1] + rng.uniform(-far, far), 500.0]
        look = player if rng.random() < 0.8 else [2 * eye[0] - player[0], 2 * eye[1] - player[1], 0.0]
        sc.camera(eye, look, rng)
        for c in range(1, 6):
            pos = [player[0] + rng.uniform(-500, 500), player[1] + rng.uniform(-500, 500), 0.0]
            sc.char(c, pos, rng.uniform(80, 300), 40.0, angle(rng), size=rng.choice([1, 3, 4, 1, 3, 4, 2]),
                    alive=rng.random() < 0.9, attribute=rng.randrange(-2, 10))
        frames = 0
        for step in range(14):
            c, att = rng.randrange(1, 6), rng.randrange(1, 6)
            r = rng.random()
            if r < 0.2:
                sc.critical(c)
            elif r < 0.4:
                sc.critical(c, rng.choice(["crit", "dying", "nodamage"]))
            elif r < 0.7:
                kind = rng.choice([-1, 0, 1, 2])
                if kind == -1 and sc.hm.size[c] not in (1, 3, 4):
                    kind = 0
                sc.protect(c, rng.choice([0, 1]), kind)
            else:
                if sc.hm.size[c] not in (1, 3, 4):
                    continue
                sc.guard(c, att if rng.random() < 0.85 else c)
            for _ in range(rng.randrange(0, 25)):
                sc.frame()
                frames += 1
        while live_slots(sc.hm) and frames < 600:
            sc.frame()
            frames += 1
        sc.guard(1, 2) if sc.hm.size[1] in (1, 3, 4) else None
        sc.frame()
        want = sc.check("protect seed %d" % seed)
        anms = sum(1 for _, w in want for d in w.get("draws", []) if d[0] == "anm")
        words = sum(1 for _, w in want for d in w.get("draws", []) if d[0] == "eff" and d[-1] == 60)
        sounds = sum(len(w.get("events", [])) for _, w in want)
        return frames + 1, anms, words, sounds

    def test_protect_guard_and_critical(self):
        total = [0, 0, 0, 0]
        for seed in range(30):
            for k, v in enumerate(self.run_case(seed)):
                total[k] += v
        print("protect/guard/critical: 30 cases, %d frames, %d animation draws, %d critical draws, %d sounds and "
              "shakes" % tuple(total), file=sys.stderr)
        self.assertGreater(total[1], 500)
        self.assertGreater(total[2], 300)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class FlyFontsAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build("hit_probe")

    def value(self, rng):
        r = rng.random()
        if r < 0.1:
            return -1
        if r < 0.15:
            return rng.randrange(-99999, 0)
        if r < 0.2:
            return rng.randrange(10000, 200000)
        return rng.choice([rng.randrange(0, 10), rng.randrange(0, 100), rng.randrange(0, 1000),
                           rng.randrange(0, 10000)])

    def run_case(self, seed, frames=90):
        rng = random.Random(seed)
        sc = Script(self, seed)
        player = [rng.uniform(-3000, 3000), rng.uniform(-3000, 3000), rng.uniform(0, 300)]
        sc.player(player)
        eye = [player[0] + rng.uniform(-1200, 1200), player[1] + rng.uniform(-1200, 1200), player[2] + 450]
        sc.camera(eye, player, rng)
        if rng.random() < 0.3:
            sc.camid(1, 1)
        chars = list(range(1, 8))
        for c in chars:
            pos = [player[0] + rng.uniform(-1500, 1500), player[1] + rng.uniform(-1500, 1500),
                   player[2] + rng.uniform(-80, 80)]
            member = rng.random() < 0.35
            sc.char(c, pos, rng.uniform(60, 300), 40.0, angle(rng), kind=2 if member else rng.choice([0, 0x20, 0x80]),
                    slot=rng.randrange(-1, 3) if member else -1, icons=rng.randrange(0, 12))
        quads = 0
        for f in range(frames):
            for _ in range(rng.choice([0, 0, 0, 1, 1, 2, 5])):
                r = rng.random()
                cid = rng.choice(chars + [None]) if rng.random() < 0.97 else None
                if r < 0.12:
                    pos = [player[0] + rng.uniform(-1500, 1500), player[1] + rng.uniform(-1500, 1500),
                           player[2] + rng.uniform(-100, 300)]
                    sx = rng.choice([1.0, 1.3, 1.6, rng.uniform(0.5, 2.5)])
                    sy = rng.choice([sx, rng.uniform(0.5, 2.5)])
                    sc.flynum(rng.randrange(0, 24), self.value(rng), pos, sx, sy, rng.randrange(-40, 41))
                elif r < 0.75:
                    sc.fly_entry(rng.choice([2, 3, 5, 20, 19, 23, 0]), "new", self.value(rng), cid)
                elif r < 0.85:
                    sc.fly_entry(rng.choice([19, 23, 3]), "exp", self.value(rng), cid)
                elif r < 0.92:
                    sc.fly_entry(rng.choice([23, 2]), "level", 0, cid)
                else:
                    sc.fly_entry(0, "miss", 0, cid)
            if rng.random() < 0.08:
                c = rng.choice(chars)
                sc.char(c, [player[0] + rng.uniform(-1500, 1500), player[1] + rng.uniform(-1500, 1500), player[2]],
                        rng.uniform(60, 300), 40.0, angle(rng), kind=sc.hm.m.load(sc.hm.chars[c] + 0x108, 4),
                        slot=sc.hm.slot_of[c], icons=rng.randrange(0, 12), alive=rng.random() < 0.7)
            if rng.random() < 0.05:
                e2 = [player[0] + rng.uniform(-1200, 1200), player[1] + rng.uniform(-1200, 1200), player[2] + 450]
                sc.camera(e2, player, rng)
            pause = rng.random() < 0.1
            sc.fly(pause, rng.random() < 0.05)
            quads += len(sc.want[-1][1]["quads"])
        sc.check("fly fonts seed %d" % seed)
        entries = sum(1 for line, _ in sc.want if line.startswith("fly"))
        return frames, quads, entries

    def test_numbers(self):
        total = [0, 0, 0]
        for seed in range(60):
            for k, v in enumerate(self.run_case(seed)):
                total[k] += v
        print("fly fonts: 60 cases, %d frames, %d quads, %d calls and frames" % tuple(total), file=sys.stderr)
        self.assertGreater(total[1], 5000)


if __name__ == "__main__":
    unittest.main()
