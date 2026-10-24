#!/usr/bin/env python3
"""crates/piney-effect's staff roll (piney_effect::stfroll) against
Quarantine's own STFROLL_VOL4 run in eemu.

PINEY_VOLUME=quarantine is required (the roll is Quarantine's alone). The
game's object (0x150 bytes) is built by its constructor (QUA gcmn 0x004f0aa0)
and Init(0) (0x004f0d40), then its main (0x004f1880) runs frame by frame on
tools/test_effect_rs.py's EffectMachine, with `ending` served from DATA.BIN.
Recorded, not run: ccKanji::Disp (the kanji, its dx and dy, the text),
ccSprite::SetTex and MakePacket (the logo's texture and transparency), ccAnm's
SetAnm, _AnimateForward (tools/anim.py's playback) and Draw (its matrix and
transparency), ccBgmPlay and ccBgmStop, the camera calls. The particle
generators are made by their own constructor; started, killed and deleted are
recorded, and ccCheckParticleGenerator answers that every started one is on
the list, as the probe's particles, which no frame runs, do. genrand() (behind
ccRand and ccRandF) is a 32-bit LCG from 0x12345678 on both sides.

Each frame the object's fields, its lines, comets and generators, the draws,
the asks and genrand's word are compared with stfroll_probe's.

    PINEY_VOLUME=quarantine python3 tools/test_stfroll_rs.py            the tests
    PINEY_VOLUME=quarantine python3 tools/test_stfroll_rs.py run N      N frames, the first mismatch
"""

import json
import os
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
import test_effect_rs as base  # noqa: E402
from test_effect_rs import ISO, ONE, fb  # noqa: E402

TARGET = base.TARGET
EXAMPLE = os.path.join(TARGET, "release", "examples", "stfroll_probe")
# Quarantine's staff roll (gcmn, no symbols): its object's constructor,
# Init and main, and its two line tables.
CTOR, INIT, MAIN = 0x004F0AA0, 0x004F0D40, 0x004F1880
CREDITS, END_PAGE = 0x005CF1C0, 0x005CFC20
KANJI_SIZE = 0xE8


def build():
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "piney-effect", "--example", "stfroll_probe"],
                   cwd=base.ROOT, check=True)


def probe(frames):
    p = subprocess.run([EXAMPLE, ISO], input=f"roll {frames}\n", capture_output=True, text=True, cwd=base.ROOT)
    if p.returncode:
        raise RuntimeError(p.stderr[-2000:])
    return [json.loads(line) for line in p.stdout.splitlines()]


class Roll:
    """The game's roll in eemu."""

    def __init__(self):
        import anim
        import ccs
        import gzarc
        em = base.EffectMachine(town=False)
        self.em, m = em, em.m
        data = gzarc.open_bytes(base.DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        c = ccs.Ccs(gzarc.inflate(data, members["ending.cmp"]))
        em.files.append("ending")
        em.ccs["ending"] = c
        for _, a in anim.animations(c):
            em.anims.setdefault(c.objects[a.object][0], a)
        sym = em.sym
        self.word = 0x12345678

        def genrand(mm, *a):
            self.word = (self.word * 1664525 + 1013904223) & 0xFFFFFFFF
            return self.word
        m.hooks[sym("genrand__Fv")] = genrand
        self.draws, self.asks = [], []
        self.anm_name, self.anm_time = {}, {}

        def anm_ct(mm, a, *r):
            for base_ in (a, a + 0x40):
                for i in range(16):
                    mm.store(base_ + 4 * i, 4, ONE if i % 5 == 0 else 0)
            mm.store(a + 0x84, 4, ONE)
            mm.store(a + 0x88, 4, ONE)
            mm.store(a + 0x9C, 2, 256)
            mm.store(a + 0xAC, 4, 0)
            return a

        def set_anm(mm, a, stream, name, chg, *r):
            from eemu import _cstr
            n = _cstr(mm, name).decode()
            self.anm_name[a] = n
            self.anm_time[a] = 0
            mm.store(a + 0x98, 4, 0)
            mm.store(a + 0xAC, 4, 1 if n in em.anims else 0)
            return 0

        def forward(mm, a, spd, *r):
            f = em.anims[self.anm_name[a]].forward(self.anm_time[a], spd & 0xFFFF)
            self.anm_time[a] = f.time
            mm.store(a + 0x98, 4, f.time >> 8)
            mm.store(a + 0x9E, 2, f.time & 0xFF)
            return int(f.ended)

        def anm_draw(mm, a, *r):
            self.draws.append(["anm", self.anm_name[a], self.anm_time[a], em.rvec(a + 0x40, 16), mm.load(a + 0x88, 4)])
            return 0
        m.hooks[sym("__ct__5ccAnmFv")] = anm_ct
        m.hooks[sym("SetAnm__5ccAnmFP8ccStreamPcUi")] = set_anm
        m.hooks[sym("_AnimateForward__5ccAnmFUi")] = forward
        m.hooks[sym("Draw__5ccAnmFv")] = anm_draw
        # The text and the logo.
        self.tex = {}
        nop = lambda mm, *a: 0                                   # noqa: E731
        m.hooks[sym("__ct__8ccSpriteFv")] = lambda mm, a, *r: a
        m.hooks[sym("Init__7ccKanjiFii")] = nop
        m.hooks[sym("__ct__6ccMaskFii")] = lambda mm, a, *r: a
        m.hooks[sym("SendPacket__8ccSpriteFv")] = nop

        def set_tex(mm, a, ccs_, chunk, *r):
            from eemu import _cstr
            self.tex[a] = _cstr(mm, chunk).decode()
            return 0

        def disp(mm, a, s, c, *r):
            from eemu import _cstr
            k = (a - mm.load(self.obj + 0x44, 4)) // KANJI_SIZE
            self.draws.append(["line", k, mm.load(a + 0x40, 4), mm.load(a + 0x44, 4),
                               _cstr(mm, s).decode("shift_jis")])
            return 0

        def make_packet(mm, a, *r):
            self.draws.append(["logo", self.tex.get(a, ""), mm.load(a + 0x30, 4)])
            return 0
        m.hooks[sym("SetTex__8ccSpriteFPcPci")] = set_tex
        m.hooks[sym("Disp__7ccKanjiFPciff")] = disp
        m.hooks[sym("MakePacket__8ccSpriteFii")] = make_packet
        # The music and the camera.
        m.hooks[sym("ccBgmPlay__Fi")] = lambda mm, n, *r: self.asks.append(["bgm", n]) or 0
        m.hooks[sym("ccBgmStop__Fv")] = lambda mm, *r: self.asks.append(["bgmstop"]) or 0
        self.cam_pos = None
        m.hooks[sym("changeCamera__Fi")] = nop

        def set_pos(mm, p, n, *r):
            self.cam_pos = em.rvec(p)
            return 0

        def set_view(mm, p, n, *r):
            self.asks.append(["camera", self.cam_pos, em.rvec(p)])
            return 0
        m.hooks[sym("cameraSetPos__FPfi")] = set_pos
        m.hooks[sym("cameraSetView__FPfi")] = set_view
        # The generators: started, killed, deleted.
        self.started, self.kill, self.comet_serial = [], {}, {}

        def start(mm, g, *r):
            self.started.append(g)
            self.kill[g] = 0
            sync = mm.load(g + 0x18, 4)
            if sync and sync - 0x70 not in self.comet_serial:
                self.comet_serial[sync - 0x70] = len(self.comet_serial) + 1
            return 0

        def kill(n):
            def f(mm, g, *r):
                self.kill[g] = n
                return 0
            return f
        m.hooks[sym("startParticleGenerator__FP19ccParticleGenerator")] = start
        m.hooks[sym("ccCheckParticleGenerator__FP19ccParticleGenerator")] = lambda mm, g, *r: int(g in self.kill)
        m.hooks[sym("ParticleKill__19ccParticleGeneratorFv")] = kill(2)
        m.hooks[sym("ParticleDelete__19ccParticleGeneratorFv")] = kill(3)
        self.obj = em.malloc(m, 0x150)
        em.call(CTOR, self.obj)
        em.call(INIT, self.obj, 0)

    # reading the object ---------------------------------------------------------------
    def u(self, off, n=4):
        return self.em.m.load(self.obj + off, n)

    def s32(self, off):
        v = self.u(off)
        return v - (1 << 32) if v & 0x80000000 else v

    def vec(self, at, size_off):
        n, data = self.u(size_off), self.u(size_off + 4)
        return [self.em.m.load(data + 4 * k, 4) for k in range(n)]

    def gen_index(self, g):
        return self.started.index(g) if g in self.started else -1

    def state(self, frame):
        em, m = self.em, self.em.m

        def s32(v):
            return v - (1 << 32) if v & 0x80000000 else v
        last = self.u(0x24)
        state = [self.s32(0x118), self.s32(0x11C), self.s32(0x100), self.u(0x114, 1), self.u(0x108, 1),
                 self.u(0x28), self.s32(0x2C), self.s32(0x30), self.s32(0x34), self.u(0x120), self.u(0x124),
                 self.s32(0x128), self.s32(0x12C), self.s32(0x130), self.s32(0x134), self.s32(0x138),
                 self.s32(0x13C), self.s32(0x140), self.s32(0x10C), self.s32(0x110), self.comet_serial.get(last, 0)]
        vecs = [em.rvec(self.obj + 0x70), em.rvec(self.obj + 0x80), em.rvec(self.obj + 0x50),
                em.rvec(self.obj + 0x60), em.rvec(last + 0x70) if last else [0, 0, 0, ONE]]
        anms = []
        for k in range(2):
            a = self.u(0x48) + 0x110 * k
            anms.append([self.anm_time.get(a, 0), m.load(a + 0x9C, 2), int(m.load(a + 0xAC, 4) != 0),
                         m.load(a + 0x88, 4)])
        lines = []
        for r in self.vec(0, 4):
            end = r >= END_PAGE
            lines.append([int(end), (r - (END_PAGE if end else CREDITS)) // 12, m.load(r + 4, 4)])
        comets = []
        for c in self.vec(0x0C, 0x10):
            subs = [self.gen_index(g) for g in [m.load(m.load(c + 0x6C, 4) + 4 * k, 4)
                                                for k in range(m.load(c + 0x68, 4))]]
            comets.append([self.comet_serial.get(c, 0), em.rvec(c, 16), m.load(c + 0x40, 4), s32(m.load(c + 0x44, 4)),
                           m.load(c + 0x48, 4), s32(m.load(c + 0x4C, 4)), s32(m.load(c + 0x50, 4)),
                           s32(m.load(c + 0x54, 4)), s32(m.load(c + 0x58, 4)), s32(m.load(c + 0x5C, 4)),
                           self.gen_index(m.load(c + 0x60, 4)), subs, em.rvec(c + 0x70)])
        gens = []
        for k, g in enumerate(self.started):
            ffs = []
            for i in range(4):
                f = m.load(g + 0x34 + 4 * i, 4)
                ffs.append(m.load(f, 4) if f else 0)
            sync = m.load(g + 0x18, 4)
            gens.append([k, m.load(g, 4), ffs, self.comet_serial.get(sync - 0x70, 0) if sync else 0,
                         (m.load(g + 4, 1) >> 4) & 1, em.rvec(g + 0x60), self.kill[g]])
        out = {"frame": frame, "state": state, "vecs": vecs, "anms": anms, "lines": lines, "comets": comets,
               "gens": gens, "draws": self.draws, "asks": self.asks, "genrand": self.word}
        self.draws, self.asks = [], []
        return out

    def frame(self):
        self.em.call(MAIN, self.obj)


def first_mismatch(frames):
    want = probe(frames)
    roll = Roll()
    for f in range(frames + 1):
        if f:
            roll.frame()
        got = json.loads(json.dumps(roll.state(f)))
        port = want[f]
        if got != port:
            for k in got:
                if got[k] != port.get(k):
                    return f, k, port.get(k), got[k]
    return None


@unittest.skipUnless(volume.NAME == "quarantine", "the staff roll is Quarantine's: PINEY_VOLUME=quarantine")
class StfRollAgainstGame(unittest.TestCase):
    FRAMES = int(os.environ.get("PINEY_STFROLL_FRAMES", "3000"))

    @classmethod
    def setUpClass(cls):
        build()

    def test_every_frame(self):
        bad = first_mismatch(self.FRAMES)
        self.assertIsNone(bad, f"frame {bad and bad[0]}: {bad and bad[1]}\nport {bad and bad[2]}\ngame {bad and bad[3]}")


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "run":
        build()
        print(first_mismatch(int(sys.argv[2])) or "all frames agree")
    else:
        unittest.main()
