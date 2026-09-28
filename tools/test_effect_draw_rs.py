#!/usr/bin/env python3
"""crates/piney-effect's sprite draw (src/sprite.rs) against the game's own
ccEff::Draw run in eemu and the VU1 program mc_DrawEff run in tools/vu.py.

The draw_probe example draws a ccEff through the port (Eff::packet, what
the GS receives; Eff::render, the primitive put in the layer); the same
sprite goes through the game:

  - DrawMachine: main in eemu (tools/test_anim.py's machine) with what
    ccEff::Draw reads set up as the field sets it: ccSys (the scratch stack,
    the screen, ZBUF), a ccLayer whose ccView is InitCCSys's sysLayer view
    with WORLD_MAN::GO's SetFrame and ccView::SetView from a ccCam placed by
    ccCam::SetMatrix_PosTarget(eye, target), a ccDrawEnv built by its own
    constructor (Reset) and SetFog; a ccEffChunk for each EFF_ object of the
    effect files with its texture and palette built by ccTexChunk::Init and
    ccClutChunk::Init from the file's Texture and Clut chunks (their
    ccBltData cleared: resident, no upload chained), the ccEff by
    ccEff::Init(chunk, fogSw). ccDrawPacketCtrl::GetWork hands Draw a
    buffer (filled with noise first), ccDLSort::Add records the sort key.
  - The packet then goes through a VIF1 model (the tag's STMOD/STCYCL, then
    STMASK, STMOD, STROW, masked and offset UNPACKs, MSCAL) into tools/vu.py's
    Vu (VU memory filled with noise first), with the MAC flag delayed as the
    FMAC pipeline delays it (FMAND reads the flags of the upper instruction
    four pairs earlier); what XGKICK sends is decoded as the GS takes it
    (A+D registers, PACKED PRIM, FOG, ST, XYZ2).
  - SpritesAgainstGame: random cameras (eye and target), random draw
    environments (fog), and for each of thousands of cases a random EFF_
    object with random position (on screen, off screen, behind the eye,
    beyond the far plane), scale, turn, colour, transparency (negative
    too), pattern, blend type, render state and PRIM; compared bit for bit:
    the view's fview, the fog constants, whether a packet is sorted, its key
    and matrix, whether VU1 kicks, every GS register (ALPHA, TEST, TEX1,
    CLAMP, TEX0's format bits, RGBAQ, FOGCOL, ZBUF, PRIM, FOG) and each
    vertex's ST and XYZ, and the primitive the port renders against the one
    the GS draws (kind, shading, blend, alpha and depth tests, texture,
    filter, wrap, and each vertex's position, depth, texel and colour).

Skipped when the disc is not extracted or cargo is missing.
"""

import collections
import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

from test_effect_rs import (DATA, EFFECT_CCS_TBL, ELF, ISO, ONE, PROFILE, ROOT,  # noqa: E402
                            TARGET, build, eff_chunks, fb)

EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "draw_probe")
CCSYS, LAYER_ACTIVE, DRAWENV_ACTIVE = inf_va(0x003788E0), inf_va(0x003788D8), inf_va(0x003788C4)
SYS, SCRATCH, VIEW, LAYER, DRAWENV, CAM, EFF, PKT, VEC = (0x01000000, 0x01010000, 0x01020000, 0x01021000,
                                                         0x01022000, 0x01023000, 0x01024000, 0x01025000,
                                                         0x01026000)
HEAP, HEAP_END = 0x01100000, 0x01F00000
ZBUF = 0xE0
MASK32 = 0xFFFFFFFF
# TEX0's PSM, TW, TH, TCC and TFX.
TEX0_FORMAT = (0x3F << 20) | (0xF << 26) | (0xF << 30) | (1 << 34) | (3 << 35)


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & MASK32))[0]


def bits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def hx(*v):
    return " ".join("%x" % x for x in v)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


def vu1_class():
    import vu

    class Vu1(vu.Vu):
        """tools/vu.py's Vu with the MAC flag FMAND/FMOR/FMEQ read as the
        hardware gives it: the flags of the upper instruction four pairs
        earlier (the FMAC pipeline; mc_DrawEff puts its FMANDs exactly four
        pairs after the SUBs they test, with no stall between)."""

        def __init__(self, blob):
            super().__init__(blob)
            self.step = 0
            self.hist = collections.deque(maxlen=8)

        def _pair(self, lo, hi, pc):
            self.step += 1
            return super()._pair(lo, hi, pc)

        def _flags(self, dest, vals):
            super()._flags(dest, vals)
            self.hist.append((self.step, self.mac))

        def _lower(self, w, pc, vf, vi):
            self.mac = 0
            for s, mac in self.hist:
                if s <= self.step - 4:
                    self.mac = mac
            return super()._lower(w, pc, vf, vi)
    return Vu1


class DrawMachine:
    """ccEff::Draw's surroundings in eemu, as the module docstring says."""

    def __init__(self):
        import ccs
        import gzarc
        import vif
        import vu
        from image import Program
        from test_anim import machine_class
        self.vif = vif
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value     # noqa: E731
        m, sym = self.m, self.sym
        self.heap = HEAP
        for name in ("ccMalloc__FUi", "_ccMalloc__FUiUiUii", "__nw__FUi", "__nwa__FUi"):
            m.hooks[sym(name)] = self.malloc
        for name in ("ccFree__FPv", "__dl__FPv"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        self.mc = vu.Microcode(ELF)
        self.Vu1 = vu1_class()
        # The effect files, by effectCCSTbl, and their EFF_ objects.
        data = gzarc.open_bytes(DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        self.files = []
        for i in range(13):
            name = self.prog.cstr(EFFECT_CCS_TBL + 44 * i).decode()
            if not name:
                break
            self.files.append(name[:-4].lower())
        self.effs = []    # (file index, chunk index, name, fields, ccEffChunk address, (tex, clut) objects)
        for k, n in enumerate(self.files):
            c = ccs.Ccs(gzarc.inflate(data, members[n + ".cmp"]))
            tex, clut = self.textures(c)
            for j, (name, e) in enumerate(eff_chunks(c).items()):
                chunk, objs = self.eff_chunk(c, e, tex, clut)
                self.effs.append((k, j, name, e, chunk, objs))
        # ccSys, the layer and its view (InitCCSys's sysLayer view, then
        # WORLD_MAN::GO's SetFrame), the draw environment.
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(SYS + 12, 2, 512)
        m.store(SYS + 14, 2, 448)
        m.store(SYS + 20, 4, 0x3F955555)
        m.store(SYS + 0xBC8, 8, ZBUF)
        for off, val in ((0x204, 0x457FF000), (0x200, 0x457FF000), (0x1E4, 0x457FF000), (0x1E0, 0x457FF000),
                         (0x1DC, 0x41000000), (0x1FC, 0x41000000), (0x1EC, 0x49800000), (0x20C, 0x49800000),
                         (0x25C, 0x447A0000), (0x250, 0x3F800000), (0x254, 0x4D800000)):
            m.store(VIEW + off, 4, val)
        self.call("SetFrame__6ccViewFffffffff", VIEW, fargs=[0, 0, fb(512), fb(384), fb(256), fb(192), ONE, ONE])
        m.store(LAYER + 0x2C, 4, VIEW)
        m.store(LAYER_ACTIVE, 4, LAYER)
        self.call("__ct__9ccDrawEnvFv", DRAWENV)
        m.store(DRAWENV_ACTIVE, 4, DRAWENV)
        m.store(CAM + 0x0C, 4, fb(45.0))
        m.hooks[sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm, *a: PKT
        m.hooks[sym("Add__8ccDLSortFfP10_sceDmaTagP10_sceDmaTag")] = self.add
        for name in ("MakePacketLoadDataSub__9ccBltDataFP7ccLayeriP10_sceDmaTag",
                     "MakePacketLoadDataSub__9ccBltDataFP7ccLayerP10_sceDmaTag"):
            m.hooks[sym(name)] = self.upload
        self.sorted = None

    # memory ------------------------------------------------------------------
    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        assert self.heap < HEAP_END
        m.mem[a:a + n] = bytes(n)
        return a

    def vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    def rvec(self, a, n=4):
        return [self.m.load(a + 4 * i, 4) for i in range(n)]

    def call(self, name, *args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name) if isinstance(name, str) else name, list(args), limit=50_000_000)

    def add(self, m, sort, head, tail, *a):
        self.sorted = (m.f[12], head, tail)
        return 0

    def upload(self, m, *a):
        raise AssertionError("a texture upload was chained")

    # the effect files' chunks ---------------------------------------------------
    def textures(self, c):
        """The file's Texture and Clut chunks: {object: fields}."""
        tex, clut = {}, {}
        for off, t, _n, _end in c.chunks():
            if t is None:
                continue
            p = off + 8
            if t & 0xFFFF == 0x0300:
                obj, cl = struct.unpack_from("<II", c.data, p)
                q = p + (12 if c.version >= 0x92 else 8)
                flag, psm, mip, aref, tw, th = c.data[q:q + 6]
                q += 8
                pos = []
                for _ in range(mip + 1):
                    x, y, n = struct.unpack_from("<hhI", c.data, q)
                    pos.append((x, y))
                    q += 8 + 4 * n
                tex[obj] = dict(clut=cl, flag=flag, psm=psm, mip=mip, aref=aref, tw=tw, th=th, pos=pos)
            elif t & 0xFFFF == 0x0400:
                obj = struct.unpack_from("<I", c.data, p)[0]
                q = p + (8 if c.version >= 0x92 else 4)
                flag, cpsm = c.data[q], c.data[q + 1]
                x, y, n = struct.unpack_from("<hhI", c.data, q + 4)
                clut[obj] = dict(flag=flag, cpsm=cpsm, psm=19 if n > 16 else 20, pos=(x, y))
        return tex, clut

    def eff_chunk(self, c, e, textures, cluts):
        """A ccEffChunk as Decode_Eff leaves it, its texture and palette
        built by their own Init from the file's chunks as Decode_Texture and
        Decode_Clut describe them, resident (no ccBltData)."""
        m = self.m
        tobj = [o[0] for o in c.objects].index(e["tex"])
        t = textures.get(tobj)
        index = self.malloc(m, 0x40)
        objs = None
        if t is not None:
            cindex = self.malloc(m, 0x40)
            cl = cluts.get(t["clut"])
            if cl is not None:
                cdesc = self.malloc(m, 8)
                m.store(cdesc, 2, cl["flag"])
                m.store(cdesc + 2, 1, cl["cpsm"])
                m.store(cdesc + 3, 1, cl["psm"])
                m.store(cdesc + 4, 2, cl["pos"][0] & 0xFFFF)
                m.store(cdesc + 6, 2, cl["pos"][1] & 0xFFFF)
                cchunk = self.malloc(m, 0x28)
                self.call("Init__11ccClutChunkFP12ccChunkIndexP15ccClutChunkDesc", cchunk, cindex, cdesc)
                m.store(cchunk + 0x10, 4, 0)
                m.store(cindex + 0x2C, 4, cchunk)
            desc = self.malloc(m, 0x18)
            m.store(desc, 2, t["flag"] | 0x40)            # 0x40: no ccBltData
            for off, k in ((2, "psm"), (3, "tw"), (4, "th"), (5, "mip"), (6, "aref")):
                m.store(desc + off, 1, t[k])
            for i, (x, y) in enumerate(t["pos"][:4]):
                m.store(desc + 8 + 4 * i, 2, x & 0xFFFF)
                m.store(desc + 10 + 4 * i, 2, y & 0xFFFF)
            tchunk = self.malloc(m, 0x48)
            self.call("Init__10ccTexChunkFP12ccChunkIndexP12ccChunkIndexP14ccTexChunkDesc", tchunk, index, cindex,
                      desc)
            m.store(index + 0x2C, 4, tchunk)
            objs = (tobj, t["clut"])
        pats = self.malloc(m, 8 * max(1, len(e["pats"])))
        for i, (u, v, tr) in enumerate(e["pats"]):
            m.store(pats + 8 * i, 2, u)
            m.store(pats + 8 * i + 2, 2, v)
            m.store(pats + 8 * i + 4, 2, tr)
        addr = self.malloc(m, 0x40)
        m.store(addr + 0x0C, 4, index)
        m.store(addr + 0x18, 2, e["flag"])
        m.store(addr + 0x1A, 2, e["zoffs"] & 0xFFFF)
        m.store(addr + 0x1C, 2, len(e["pats"]))
        for i, v in enumerate(e["corners"]):
            m.store(addr + 0x20 + 4 * i, 4, v)
        m.store(addr + 0x30, 4, e["w"] | e["h"] << 16)
        m.store(addr + 0x34, 4, pats)
        return addr, objs

    # the frame ----------------------------------------------------------------
    def set_camera(self, eye, target):
        self.vec(VEC, [fb(v) for v in eye] + [ONE])
        self.vec(VEC + 16, [fb(v) for v in target] + [ONE])
        self.call("SetMatrix_PosTarget__5ccCamFPfPf", CAM, VEC, VEC + 16)
        self.call("SetView__6ccViewFRC5ccCamPA4_f", VIEW, CAM, 0)

    def set_fog(self, near, far, nr, fr, colour):
        self.call("SetFog__9ccDrawEnvFffffUi", DRAWENV, colour, fargs=[near, far, nr, fr])

    # one draw -----------------------------------------------------------------
    def draw(self, eff, pat, rng):
        """ccEff::Draw(pat): (why, None) when nothing is sorted - hidden
        (the transparency below 0), or the centre nearer than the view's
        near w or beyond its far one - else (why, (key, the packet's matrix,
        the GS output or None when VU1 kicks nothing))."""
        m = self.m
        m.mem[PKT:PKT + 432] = bytes(rng.getrandbits(8) for _ in range(432))
        m.mem[SCRATCH:SCRATCH + 64] = b"\xa5" * 64
        self.sorted = None
        self.call("Draw__5ccEffFUs", eff, pat)
        if self.sorted is None:
            if bytes(m.mem[SCRATCH:SCRATCH + 64]) == b"\xa5" * 64:
                return "hidden", None
            w = f32(m.load(SCRATCH + 60, 4))
            return ("near" if w < 8 else "far"), None
        key, head, tail = self.sorted
        assert (head, tail) == (PKT, PKT + 16)
        pkt = bytes(m.mem[PKT:PKT + 432])
        tag = struct.unpack_from("<Q", pkt, 16)[0]
        assert (tag & 0xFFFF, (tag >> 28) & 7) == (25, 2), hex(tag)
        u = self.Vu1(self.mc.vu1)
        for q in u.mem:
            q[:] = [rng.getrandbits(32) for _ in range(4)]
        self.vif_run(u, pkt[24:32] + pkt[32:432])
        mat = [u.mem[i][k] for i in range(4) for k in range(4)]
        gs = self.gif(u, u.kicks[0]) if u.kicks else None
        return ("drawn" if gs else "rejected by VU1"), (key, mat, gs)

    def vif_run(self, u, data):
        """VIF1 over the tag's two codes and the 25 qwords after it."""
        mode, row, mask, wl = 0, [0] * 4, 0, 1
        for c in self.vif.codes(data, cycle=(4, 4)):
            if c.cmd == 0x05:
                mode = c.imm & 3
            elif c.cmd == 0x01:
                wl = c.imm >> 8
            elif c.cmd == 0x20:
                mask = struct.unpack("<I", c.data)[0]
            elif c.cmd == 0x30:
                row = list(struct.unpack("<4I", c.data))
            elif c.is_unpack:
                assert not c.flg
                for i, vec in enumerate(self.vif.unpack(c, row, mode)):
                    q = u.mem[c.addr + i]
                    for k in range(4):
                        mk = (mask >> (8 * min(i % wl, 3) + 2 * k)) & 3 if c.mask else 0
                        if mk == 0 and vec[k] is not None:
                            q[k] = vec[k]
                        elif mk == 1:
                            q[k] = row[k]
                        elif mk == 0:
                            raise AssertionError("unpack lane with no data")
            elif c.cmd == 0x14:
                assert c.imm == 0x595
                u.run(c.imm)
            else:
                assert c.cmd in (0x00, 0x13), hex(c.cmd)

    @staticmethod
    def gif(u, addr):
        """What the GS takes from an XGKICK at `addr`: the A+D registers,
        PRIM, and each XYZ2's vertex (ST, X, Y, Z, ADC, FOG)."""
        regs, verts, st, fog, prim = {}, [], None, None, None
        q = addr
        while True:
            w = u.mem[q]
            lo, hi = w[0] | w[1] << 32, w[2] | w[3] << 32
            nloop, eop, pre, flg, nreg = lo & 0x7FFF, lo >> 15 & 1, lo >> 46 & 1, lo >> 58 & 3, (lo >> 60) or 16
            q += 1
            assert flg == 0
            if pre:
                prim = lo >> 47 & 0x7FF
            for _ in range(nloop):
                for r in range(nreg):
                    reg, d = hi >> (4 * r) & 15, u.mem[q]
                    q += 1
                    if reg == 0xE:
                        regs[d[2] & 0xFF] = d[0] | d[1] << 32
                    elif reg == 0xA:
                        fog = d[3] >> 4 & 0xFF
                    elif reg == 0x2:
                        st = (d[0], d[1])
                    elif reg == 0x5:
                        verts.append([st[0], st[1], d[0] & 0xFFFF, d[1] & 0xFFFF, d[2], d[3] >> 15 & 1, fog])
                    else:
                        raise AssertionError("GIF register %x" % reg)
            if eop:
                return dict(regs=regs, prim=prim, verts=verts)


def expected_prim(gs, stem, objs):
    """The primitive the GS draws from `gs`, as the probe prints one."""
    r, prim = gs["regs"], gs["prim"]
    test, zbuf, tex0, tex1, clamp = r[0x47], r[0x4E], r[0x06], r[0x14], r[0x08]
    tw, th = tex0 >> 26 & 15, tex0 >> 30 & 15
    rgba = r[0x01] & MASK32
    verts = [[bits((x - 0x7000) / 16.0), bits((y - 0x7200) / 16.0), z, bits(f32(s) * 2 ** tw),
              bits(f32(t) * 2 ** th), rgba] for s, t, x, y, z, adc, fog in gs["verts"]]
    tex = None
    if prim & 0x10:
        tex = [stem, objs[0], objs[1], tex0 >> 35 & 3, tex0 >> 34 & 1, tex1 >> 5 & 1, int(clamp & 3 == 1)]
    return {"kind": prim & 7, "gouraud": prim >> 3 & 1, "blend": r[0x42] if prim & 0x40 else None,
            "atest": [test >> 1 & 7, test >> 4 & 0xFF, test >> 12 & 3] if test & 1 else None,
            "depth": [test >> 17 & 3, 1 - (zbuf >> 32 & 1)], "tex": tex, "verts": verts}


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class SpritesAgainstGame(unittest.TestCase):
    CASES = int(os.environ.get("DRAW_CASES", "6000"))
    SEED = int(os.environ.get("DRAW_SEED", "1234"))

    @classmethod
    def setUpClass(cls):
        build("draw_probe")
        cls.dm = DrawMachine()

    def random_camera(self, rng):
        import math
        target = (rng.uniform(-20000, 20000), rng.uniform(-20000, 20000), rng.uniform(-100, 1500))
        d, a = rng.uniform(150, 3000), rng.uniform(-math.pi, math.pi)
        eye = (target[0] + d * math.cos(a), target[1] + d * math.sin(a), target[2] + rng.uniform(-300, 1500))
        if rng.random() < 0.05:   # straight down: sceVu0CameraMatrix's other up
            eye = (target[0], target[1], target[2] + d)
        return eye, target

    def random_eff(self, rng, eye, target):
        dm, m = self.dm, self.dm.m
        k, j, name, e, chunk, objs = rng.choice(dm.effs)
        dm.call("Init__5ccEffFP10ccEffChunki", EFF, chunk, rng.randrange(2))
        look = [t - s for s, t in zip(eye, target)]
        r = rng.random()
        if r < 0.55:      # around what the camera looks at
            pos = [t + rng.uniform(-600, 600) for t in target]
        elif r < 0.7:     # off to a side: VU1's window
            pos = [t + rng.uniform(-6000, 6000) for t in target]
        elif r < 0.8:     # at or behind the eye: the near plane
            s = rng.uniform(-0.2, 0.02)
            pos = [p + s * d + rng.uniform(-20, 20) for p, d in zip(eye, look)]
        elif r < 0.9:     # far off along the view: the far plane
            s = rng.uniform(50, 3000) / max(1.0, sum(d * d for d in look) ** 0.5 / 1000)
            pos = [p + s * d for p, d in zip(eye, look)]
        else:
            pos = [t + rng.uniform(-3000, 3000) for t in target]
        dm.vec(EFF + 0x10, [fb(v) for v in pos] + [rng.choice((ONE, 0, fb(rng.uniform(-5, 5))))])
        sc = lambda: rng.choice((1.0, rng.uniform(0.05, 4), rng.uniform(-3, 3), 0.0, rng.uniform(4, 60)))  # noqa
        m.store(EFF + 0x20, 4, fb(sc()))
        m.store(EFF + 0x24, 4, fb(sc()))
        m.store(EFF + 0x28, 4, rng.choice((0, 0x80000000, fb(rng.uniform(-7, 7)), fb(rng.uniform(-40, 40)))))
        m.store(EFF + 0x2C, 4, rng.getrandbits(32))
        m.store(EFF + 0x34, 4, fb(rng.choice((1.0, rng.uniform(-0.3, 2.6), rng.uniform(0, 1), 0.0, -1e-9))))
        if rng.random() < 0.5:
            dm.call("SetBlendType__5ccEffFi", EFF, rng.randrange(8))
        if rng.random() < 0.15:
            dm.call("SetRenderState__5ccEffF20CC_RENDER_STATE_TYPEi", EFF, rng.randrange(2), rng.randrange(2))
        if rng.random() < 0.5:     # ccEffect::InitEffect: PRIM's FGE off
            m.store(EFF + 0x62, 2, m.load(EFF + 0x62, 2) & ~0x20)
        pat = rng.randrange(len(e["pats"]))
        fields = [m.load(EFF + off, 4) for off in (0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 52, 76)]
        line = "eff " + hx(k, j, pat, *fields, m.load(EFF + 80, 8), m.load(EFF + 88, 8), m.load(EFF + 96, 2),
                           m.load(EFF + 98, 2))
        return line, pat, (k, j, name, objs)

    def test_sprites(self):
        dm, m = self.dm, self.dm.m
        rng = random.Random(self.SEED)
        lines, want = [], []
        outcomes = collections.Counter()
        for case in range(self.CASES):
            if case % 40 == 0:
                eye, target = self.random_camera(rng)
                dm.set_camera(eye, target)
                lines.append("view " + hx(*dm.rvec(VIEW + 0x50, 16), *dm.rvec(VIEW + 0xD0, 16)))
                want.append(("view", dm.rvec(VIEW + 0x10, 16)))
            if case % 40 == 0:
                r = rng.random()
                if r < 0.15:
                    fog = (0x3DCCCCCD, 0x4B7FFFFF, 0, 0, 0)       # Reset's
                elif r < 0.3:
                    fog = (fb(1000.0), fb(7500.0), 0, fb(85.0), 0x144870)    # Mac Anu's
                else:
                    d = sum((a - b) ** 2 for a, b in zip(eye, target)) ** 0.5
                    near = rng.uniform(0, 1.2 * d)
                    fog = (fb(near), fb(near + rng.uniform(50, 5000)), fb(rng.uniform(0, 100)),
                           fb(rng.uniform(0, 100)), rng.getrandbits(32))
                dm.set_fog(*fog[:4], fog[4])
                lines.append("env " + hx(0x41000000, 0x49800000, *fog, m.load(DRAWENV + 0xA8, 8), ZBUF))
                want.append(("env", dm.rvec(DRAWENV + 0xC8, 4) + [m.load(DRAWENV + 0xD8, 4)]))
            line, pat, info = self.random_eff(rng, eye, target)
            why, got = dm.draw(EFF, pat, rng)
            outcomes[why] += 1
            lines.append(line)
            want.append(("eff", got, info, line))
        answers = ask(lines)
        self.assertEqual(len(answers), len(want))
        stems = dm.files
        bad = []
        prims = 0
        for (kind, *w), a in zip(want, answers):
            if kind == "view":
                if a["fview"] != w[0]:
                    bad.append(("fview", w[0], a["fview"]))
                continue
            if kind == "env":
                if a["fog"] != w[0]:
                    bad.append(("fog", w[0], a["fog"]))
                continue
            got, (k, j, name, objs), line = w
            p = a["packet"]
            if got is None:
                if p is not None or a["prims"]:
                    bad.append((line, "the game sorts nothing", p))
                continue
            key, mat, gs = got
            if p is None:
                bad.append((line, "the port sends nothing"))
                continue
            if (p["key"], p["m"]) != (key, mat):
                bad.append((line, "key/matrix", key, mat, p["key"], p["m"]))
                continue
            if gs is None:
                if p["gs"] is not None or a["prims"]:
                    bad.append((line, "VU1 kicks nothing", p["gs"]))
                continue
            g, r = p["gs"], gs["regs"]
            if g is None:
                bad.append((line, "the port rejects a sprite VU1 draws", gs))
                continue
            game = {"alpha": r[0x42], "test": r[0x47], "tex1": r[0x14], "clamp": r[0x08],
                    "tex0fmt": r[0x06] & TEX0_FORMAT, "rgbaq": r[0x01], "fogcol": r[0x3D], "zbuf": r[0x4E],
                    "prim": gs["prim"], "fog": gs["verts"][0][6],
                    "verts": [v[:5] for v in gs["verts"]], "tex": list(objs) if objs else None}
            port = {key: g[key] for key in game}
            if game != port or r[0x34] != 0 or r[0x3F] != 0 or any(v[5] for v in gs["verts"]):
                bad.append((line, {kk: (game[kk], port[kk]) for kk in game if game[kk] != port[kk]}))
                continue
            want_prim = expected_prim(gs, stems[k], objs)
            prims += 1
            if a["prims"] != [want_prim]:
                bad.append((line, "prim", want_prim, a["prims"]))
        views = sum(1 for k, *_ in want if k == "view")
        envs = sum(1 for k, *_ in want if k == "env")
        print(f"\nsprites: {self.CASES} cases, {dict(outcomes)}, {prims} primitives compared; {views} views' "
              f"fview, {envs} draw environments' fog; {len(bad)} mismatches", file=sys.stderr)
        self.assertFalse(bad, bad[:3])
        for what in ("hidden", "near", "far", "rejected by VU1", "drawn"):
            self.assertGreater(outcomes[what], self.CASES // 100, what)


class SceneMachine:
    """The effect files decoded by the game's own ccStream::DecodeSetup in
    eemu (tools/test_stream_rs.py's Game: the allocators, the reader, the
    semaphores; texture uploads not made), and effect clumps and animations
    made as ccEffect::InitEffect makes them: new ccClump, ccCoord's
    constructor, ccClump::Init(chunk); new ccAnm, SetAnm(chunk, 0). What
    ccClump::Draw and ccAnm::Draw hand ccModel::Draw is recorded: the Obj,
    its model, its lwMatrix and the transparency (ccShadowModel::Draw and
    the Eff nodes' draws too, which no effect object reaches)."""

    def __init__(self, stems):
        import gzarc
        import test_stream_rs as ts
        self.ts = ts
        self.g = g = ts.Game()
        m = self.m = g.m
        for name in ("LoadData__9ccBltDataFv", "sceGsSetDefLoadImage"):
            m.hooks[g.sym(name)] = lambda mm, *a: 0
        data = gzarc.open_bytes(DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        self.index = {}     # stem -> {name: ccChunkIndex}
        at = ts.DATA
        for stem in stems:
            raw = gzarc.inflate(data, members[stem + ".cmp"])
            assert at + len(raw) <= ts.HEAP0
            m.mem[at:at + len(raw)] = raw
            ccs = g.malloc(m, 0x190)
            g.call("__ct__8ccStreamFP14ccStreamReader", (ccs, 0))
            m.store(ccs + 0x160, 4, at)
            g.call("DecodeSetup__8ccStreamFv", (ccs,))
            idx, n = m.load(ccs + 0x30, 4), m.load(ccs + 0x38, 4)
            self.index[stem] = {g.cstr(idx + 64 * i + 8): idx + 64 * i for i in range(n)}
            at = (at + len(raw) + 0xFFF) & ~0xFFF
        self.mark = g.heap
        layer, env = g.malloc(m, 0x100), g.malloc(m, 0x200)
        g.call("__ct__9ccDrawEnvFv", (env,))
        m.store(g.sym("active__7ccLayer"), 4, layer)
        m.store(g.sym("active__9ccDrawEnv"), 4, env)
        self.vec = g.malloc(m, 64)
        self.drawn = []
        m.hooks[g.sym("Draw__7ccModelFP16ccDrawModelParam")] = self.model_draw
        for name in ("Draw__13ccShadowModelFP16ccDrawModelParam", "Draw__5ccEffFUs", "DrawNoAnm__8ccEffObjFv"):
            m.hooks[g.sym(name)] = lambda mm, *a, name=name: self.drawn.append(name) or 0

    def model_draw(self, m, model, param, *a):
        obj = m.load(param + 0x90, 4)
        # An ExtObj copy's ccObj is named by its EXT_ index; the file's
        # object table names it OBJ_ like the object it copies.
        name = self.g.cstr(m.load(obj + 0x90, 4) + 8)
        name = "OBJ_" + name[4:] if name.startswith("EXT_") else name
        self.drawn.append([name, self.g.cstr(m.load(model, 4) + 8),
                           [m.load(obj + 4 * k, 4) for k in range(16)], m.load(param + 0x120, 4)])
        return 0

    def chunk(self, stem, name):
        return self.m.load(self.index[stem][name] + 0x2C, 4)

    def place(self, coord, pos, rot, scale, zyx):
        for k, v in enumerate(list(pos) + [1.0] + list(rot) + [0.0] + list(scale) + [1.0]):
            self.m.store(self.vec + 4 * k, 4, fb(v))
        name = "SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf" if zyx else "SetMatrix_PosRotXYZScale__7ccCoordFPfPfPf"
        self.g.call(name, (coord, self.vec, self.vec + 16, self.vec + 32))
        return [self.m.load(coord + 0x40 + 4 * k, 4) for k in range(16)]

    def clump(self, stem, name):
        g, m = self.g, self.m
        c = g.malloc(m, 0xA0)
        g.call("__ct__7ccCoordFv", (c,))
        g.call("Init__7ccClumpFP12ccClumpChunk", (c, self.chunk(stem, name)))
        g.call("SetFogSw__7ccClumpFi", (c, 0))
        return c

    def draw_clump(self, c, alpha):
        self.drawn = []
        self.m.f[12] = alpha
        self.g.call("SetTransparency__7ccClumpFf", (c,))
        self.g.call("Draw__7ccClumpFv", (c,))
        return self.drawn

    def anm(self, stem, name, steps, speed):
        g, m = self.g, self.m
        a = g.malloc(m, 0x110)
        g.call("__ct__5ccAnmFv", (a,))
        g.call("SetAnm__5ccAnmFP10ccAnmChunkUi", (a, self.chunk(stem, name), 0))
        g.call("SetFogSw__5ccAnmFi", (a, 0))
        for _ in range(steps):
            g.call("_AnimateForward__5ccAnmFUi", (a, speed))
        return a

    def draw_anm(self, a, alpha):
        self.drawn = []
        self.m.store(a + 0x88, 4, alpha)
        self.g.call("Draw__5ccAnmFv", (a,))
        return self.drawn


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ModelsAgainstGame(unittest.TestCase):
    """The effects' clumps and animations: what the port draws for a
    DrawRec::Clump / DrawRec::Anm (src/nodes.rs) against what the game's
    ccClump::Draw / ccAnm::Draw hand ccModel::Draw, for every CMP_ object of
    the effect files whose nodes are all Obj (every clump effectTbl names)
    and every ANM_ object effectTbl names (with the resistant shield's two),
    at random matrices (both of
    ccCoord's SetMatrix orders), transparencies (at and around 1/128 too)
    and animation times: the models in order, their transparency bit for
    bit, a clump node's lwMatrix bit for bit (up to the sign of zero) and an
    animated object's within the animation port's accuracy."""

    CASES = int(os.environ.get("MODEL_CASES", "10"))

    @classmethod
    def setUpClass(cls):
        build("draw_probe")

    def test_clumps_and_animations(self):
        import ccs
        import gzarc
        rng = random.Random(55)
        from image import Program
        prog = Program(ELF)
        stems = []
        for i in range(13):
            name = prog.cstr(EFFECT_CCS_TBL + 44 * i).decode()
            if not name:
                break
            stems.append(name[:-4].lower())
        rows = []
        for i in range(173):
            f, o, t = struct.unpack("<III", prog.read(inf_va(0x0033F240) + 12 * i, 12))
            if f:
                rows.append((prog.cstr(f).decode().lower().replace(".ccs", ""), prog.cstr(o).decode(), t))
        data = gzarc.open_bytes(DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        # effectTbl's animations, and the resistant shield's (gcmn
        # ccResistantShieldElement: particle's ANM_x069 and ANM_x070).
        anms = sorted({(f, o) for f, o, t in rows if t == 1} | {("particle", "ANM_x069"), ("particle", "ANM_x070")})
        # The magic portal's file (gimmickTbl[15]'s XMAGCIR.CCS, which the
        # port draws the effects' way after the effect files): its circle,
        # its idle loop and its opening, through to the opening's end,
        # where every object has faded out.
        stems.append("xmagcir")
        portal = [("xmagcir", "ANM_xmagcir1"), ("xmagcir", "ANM_xmagcir2")]
        anms += portal
        clumps = []
        for stem in stems:
            c = ccs.Ccs(gzarc.inflate(data, members[stem + ".cmp"]))
            kind = {}
            for off, t, _n, _e in c.chunks():
                if t is not None and t & 0xFFFF in (0x0100, 0x0A00, 0x0D00, 0x0E00):
                    kind.setdefault(struct.unpack_from("<I", c.data, off + 8)[0], t & 0xFFFF)
            for off, t, _n, _e in c.chunks():
                if t is not None and t & 0xFFFF == 0x0900:
                    obj, n = struct.unpack_from("<II", c.data, off + 8)
                    nodes = struct.unpack_from("<%dI" % n, c.data, off + 16)
                    if all(kind.get(x) == 0x0100 for x in nodes):
                        clumps.append((stem, c.objects[obj][0]))
        named = {(f, o) for f, o, t in rows if t == 0}
        self.assertTrue(named <= set(clumps), named - set(clumps))
        sm = SceneMachine(stems)
        lines, want = [], []
        tps = lambda: rng.choice((1.0, 0.0, 0.0078125, 0.0078126, rng.uniform(0, 1), rng.uniform(0, 1.5)))  # noqa
        for stem, name in clumps:
            c = sm.clump(stem, name)
            for _ in range(self.CASES):
                pos = [rng.uniform(-30000, 30000) for _ in range(3)]
                rot = [rng.uniform(-7, 7) for _ in range(3)]
                scale = [rng.choice((1.0, rng.uniform(-3, 3))) for _ in range(3)]
                mat = sm.place(c, pos, rot, scale, rng.random() < 0.5)
                t = fb(tps())
                want.append(("clump", stem, name, sm.draw_clump(c, t)))
                lines.append("clump %x %s %s %x" % (stems.index(stem), name, hx(*mat), t))
        for stem, name in anms:
            for _ in range(self.CASES * 3):
                # ccEffect::Main steps an animation before each draw: one
                # drawn before its first _AnimateForward (not posed yet) is
                # not a case the effects make.
                steps, speed = rng.randrange(1, 70), rng.choice((256, 256, 128, 0x180, 512))
                if (stem, name) in portal:
                    # ccMagicCircle::main steps its clip 256 a frame, the
                    # opening's 111 frames and on (held at its end).
                    steps, speed = rng.randrange(1, 180), 256
                a = sm.anm(stem, name, steps, speed)
                mat = sm.place(a, [rng.uniform(-30000, 30000) for _ in range(3)],
                               [rng.uniform(-7, 7) for _ in range(3)],
                               [rng.choice((1.0, rng.uniform(0.2, 3))) for _ in range(3)], rng.random() < 0.7)
                t = fb(tps())
                want.append(("anm", stem, name, sm.draw_anm(a, t)))
                lines.append("anm %x %s %x %x %s %x" % (stems.index(stem), name, steps, speed, hx(*mat), t))
        answers = ask(lines)
        self.assertEqual(len(answers), len(want))
        bad, counts = [], collections.Counter()
        worst = 0.0
        for (kind, stem, name, game), port, line in zip(want, answers, lines):
            counts[kind + "s"] += 1
            if stem == "xmagcir":
                counts["portal " + kind + "s"] += 1
                counts["portal models"] += len([d for d in game if not isinstance(d, str)])
            others = [d for d in game if isinstance(d, str)]
            game = [d for d in game if not isinstance(d, str)]
            if others:
                bad.append((line, "the game draws", others))
            if [(g[0], g[1], g[3]) for g in game] != [(p[0], p[1], p[3]) for p in port]:
                bad.append((line, "models", [(g[0], g[1], g[3]) for g in game], [(p[0], p[1], p[3]) for p in port]))
                continue
            for g, p in zip(game, port):
                counts[kind + " models"] += 1
                if kind == "clump":
                    if [x & 0x7FFFFFFF if x & 0x7FFFFFFF == 0 else x for x in g[2]] != \
                       [x & 0x7FFFFFFF if x & 0x7FFFFFFF == 0 else x for x in p[2]]:
                        bad.append((line, "lwMatrix", g, p))
                else:
                    d = max(abs(f32(a) - f32(b)) / max(1.0, abs(f32(a))) for a, b in zip(g[2], p[2]))
                    worst = max(worst, d)
                    if d > 1e-4:
                        bad.append((line, "lwMatrix", d, g, p))
        print(f"\nmodels: {dict(counts)}; worst animated lwMatrix difference {worst:.2e} (relative); "
              f"{len(bad)} mismatches", file=sys.stderr)
        self.assertFalse(bad, bad[:3])


if __name__ == "__main__":
    unittest.main()
