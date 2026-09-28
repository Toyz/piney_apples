#!/usr/bin/env python3
"""The shadow volumes against the game: crates/piney-data's ShadowMesh::build
against DecodeShadowModel, and crates/piney-desktop's shadow::volume
against ccShadowModel::Draw and the VU1 programs it runs.

DecodeAgainstGame. DecodeShadowModel (main 0x00140920) reads a shadow mmat
(positions and a triangle list), works out each triangle's direction
(SetShadowNormal2, the directions within 0.99609375 of one another merged),
the edges between faces (SetShadowWork2), refuses the models it cannot
shadow, and writes the VIF stream ccShadowModel::Draw sends each frame: the
directions (UNPACK V4-32 to VU1 0x212), the triangles in batches of 24 for
mc_DrawShadow2 and the edges in batches of 24 for mc_DrawShadow3. This runs
the game's function in eemu on every shadow mmat in DATA.BIN, its stream
reader (ccRingBufferTh::ReadS32/ReadS16/ReadPadding) and the heap hooked,
reads the stream back, and compares with the shadow_probe example: whether
the model is refused, each direction (as the floats the stream carries),
and each triangle and edge in order - its vertex positions and the
directions on its sides (an open edge's second side is -1 - its face, which
the stream carries as 529 - face).

VolumeAgainstGame. ccShadowModel::Draw (0x001412b0) runs in eemu with a
real ccView (SetFrame, a camera placed by ccCam::SetMatrix_PosTarget and
ccView::SetView), a random world matrix (turned, tilted, now and then
mirrored), a random light (mostly downward), length and buffer size, and a
shadow mmat decoded as above; the packet it adds goes through a VIF1 model
(STCYCL, STMASK and STROW with masked unpacks, BASE/OFFSET and the double
buffer's TOPS, MSCAL and MSCNT) into tools/vu.py's Vu (the MAC flag four
pairs late as test_effect_draw_rs has it, plus the status flag's sticky bits
for FSSET/FSAND), and what each XGKICK sends is read as the GS takes it
(TRIANGLE, TRISTRIP and TRIFAN with ADC; the blend of each). The
shadow_volume_probe example gives the port's polygons for the same draw.
Both are counted into the buffer the way the GS would (+1 for the add
blend, -1 for the subtract, where the Z beats a flat scene depth: 0, which
every face passes, and the draw's median Z) and the counts compared pixel
by pixel. The game cuts polygons at the buffer's edges before VU1's ftoi4
saturates Z at 0x7fffffff, and the port leaves that to the pixel; polygons
drawn without the cut (the whole polygon inside the GS's range) saturate at
their vertices instead, so a volume reaching past the near plane can differ
by a few pixels near the camera: up to 1% of the comparisons may.

Skipped when the disc is not extracted or cargo is missing.
PINEY_CARGO_ROOT names another cargo workspace to build the examples in;
PINEY_SHADOW_CASES sets the number of draws (default 300),
PINEY_SHADOW_VERBOSE prints each, PINEY_SHADOW_DUMP=N the polygons of draw N.
"""

import math
import os
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
DATA = volume.DATA
CARGO_ROOT = os.environ.get("PINEY_CARGO_ROOT", ROOT)
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(CARGO_ROOT, "target"))
EXAMPLE = os.path.join(TARGET, "release", "examples", "shadow_probe")
VOLUME = os.path.join(TARGET, "release", "examples", "shadow_volume_probe")

CHUNK, INDEX, READER, MINMAX = 0x01800000, 0x01801000, 0x01802000, 0x01803000
HEAP, HEAP_END = 0x01900000, 0x01F00000
NORMALS = 0x212
DRAW2, DRAW3 = 0x5EF, 0x641


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data",
                    "--example", "shadow_probe"], cwd=CARGO_ROOT, check=True)
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-desktop",
                    "--example", "shadow_volume_probe"], cwd=CARGO_ROOT, check=True)


def shadow_mmats():
    """(file, model offset, positions, triangles, vertex scale) of every
    shadow mmat, the positions as stored (s16)."""
    import ccs
    import ccsmodel
    import gzarc
    data = gzarc.open_bytes(DATA)
    out = []
    for mm in gzarc.members(data):
        try:
            c = ccs.Ccs(gzarc.inflate(data, mm))
        except Exception:
            continue
        for m in ccsmodel.models(c):
            if not m.mtype & 8:
                continue
            unit = m.scale * ccsmodel.POS_UNIT
            for s in m.mmats:
                pos = [tuple(round(v / unit) for v in p) for p in s.positions]
                out.append((mm.name, m.offset, pos, [tuple(t) for t in s.triangles], m.scale))
    return out


class Decoder:
    """DecodeShadowModel in eemu."""

    def __init__(self, prog=None, m=None):
        from image import Program
        from test_anim import machine_class
        self.prog = prog or Program(ELF)
        self.m = m or machine_class()(self.prog)
        sym = lambda n: self.prog.symbol_named(n).value     # noqa: E731
        self.fn = sym("DecodeShadowModel__FP11ccMmatChunkP12ccChunkIndexP14ccStreamReaderR14ccVertexMinMum")
        m = self.m
        for name in ("ccMalloc__FUi", "__nw__FUi", "__nwa__FUi"):
            m.hooks[sym(name)] = self.malloc
        for name in ("__dl__FPv", "__dla__FPv", "ccPrtChunkName__FP12ccChunkIndex"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        m.hooks[sym("ReadS32__14ccRingBufferThFv")] = lambda mm, *a: self.read(4) & 0xFFFFFFFF
        m.hooks[sym("ReadS16__14ccRingBufferThFv")] = lambda mm, *a: self.read(2) & 0xFFFF
        m.hooks[sym("ReadPadding__14ccRingBufferThFi")] = self.pad

    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        assert self.heap < HEAP_END
        m.mem[a:a + n] = bytes(n)
        return a

    def read(self, n):
        v = int.from_bytes(self.body[self.q:self.q + n], "little", signed=True)
        self.q += n
        return v

    def pad(self, m, _r, n, *a):
        self.q = (self.q + n - 1) // n * n
        return 0

    def decode(self, pos, tris):
        """None when refused, else (normals as float bits, [(3 positions,
        face addr)], [(2 positions, 2 face addrs)])."""
        self.body = struct.pack("<ii", len(pos), 3 * len(tris))
        self.body += b"".join(struct.pack("<3h", *p) for p in pos)
        self.body += bytes(-len(self.body) % 4)
        self.body += b"".join(struct.pack("<3i", *t) for t in tris)
        self.q, self.heap = 0, HEAP
        self.raw = None
        m = self.m
        m.mem[CHUNK:CHUNK + 0x80] = bytes(0x80)
        for i, v in enumerate((0x7FFFFFFF, -0x80000000) * 3):
            m.store(MINMAX + 4 * i, 4, v & 0xFFFFFFFF)
        refused = m.call(self.fn, [CHUNK, INDEX, READER, MINMAX], limit=200_000_000) & 0xFFFFFFFF
        if refused:
            return None
        addr, size = m.load(CHUNK + 0x40, 4), m.load(CHUNK + 0x44, 4)
        assert m.load(CHUNK + 0x10, 2) >= 1
        self.raw = (addr, size, m.load(CHUNK + 0x10, 2))
        return self.stream(bytes(m.mem[addr:addr + size]))

    @staticmethod
    def stream(d):
        normals, tris, edges = None, [], []
        q, pending = 0, []
        while q + 4 <= len(d):
            w = struct.unpack_from("<I", d, q)[0]
            q += 4
            cmd, imm, num = w >> 24 & 0x7F, w & 0xFFFF, w >> 16 & 0xFF
            if cmd in (0x00, 0x01, 0x02, 0x03, 0x10, 0x11, 0x13, 0x17):
                continue
            if cmd == 0x20:
                q += 4
            elif cmd == 0x30:
                q += 16
            elif cmd == 0x14:
                vecs = [v for v in pending if v[0] != 0]
                if imm == DRAW2:
                    words = [x for v in vecs for x in v[1]]
                    for k in range(0, len(words), 12):
                        t = words[k:k + 12]
                        tris.append(([tuple(t[i:i + 3]) for i in (0, 4, 8)], t[3]))
                else:
                    assert imm == DRAW3, hex(imm)
                    words = [x for v in vecs for x in v[1]]
                    for k in range(0, len(words), 8):
                        e = words[k:k + 8]
                        edges.append(([tuple(e[0:3]), tuple(e[4:7])], (e[3], e[7])))
                pending = []
            elif cmd >= 0x60:
                vn, vl = cmd >> 2 & 3, cmd & 3
                size = {0: 4, 1: 2, 2: 1}[vl] * (vn + 1)
                n = num or 256
                raw = d[q:q + size * n]
                q += (size * n + 3) // 4 * 4
                addr = imm & 0x3FF
                if vl == 0:
                    vals = list(struct.unpack("<%dI" % (n * (vn + 1)), raw))
                else:
                    vals = list(struct.unpack("<%dh" % (n * (vn + 1)), raw))
                if addr == NORMALS and vl == 0:
                    normals = [tuple(vals[4 * i:4 * i + 4]) for i in range(n)]
                else:
                    pending.append((addr, vals))
            else:
                raise AssertionError("VIF code %08x" % w)
        return normals, tris, edges


def f32(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(DATA) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DecodeAgainstGame(unittest.TestCase):
    def test_every_shadow_mmat(self):
        build()
        mmats = shadow_mmats()
        reqs = []
        for _f, _o, pos, tris, _s in mmats:
            reqs.append(" ".join(str(v) for v in [len(pos), len(tris)] + [c for p in pos for c in p] +
                                 [i for t in tris for i in t]))
        p = subprocess.run([EXAMPLE], input="\n".join(reqs) + "\n", capture_output=True, text=True, check=True)
        answers = p.stdout.splitlines()
        self.assertEqual(len(answers), len(mmats))
        dec = Decoder()
        seen, refused, merged, open_edges = set(), 0, 0, 0
        for (name, off, pos, tris, _scale), ans in zip(mmats, answers):
            key = (tuple(pos), tuple(tris))
            if key in seen:
                continue
            seen.add(key)
            got = dec.decode(pos, tris)
            where = "%s model 0x%x" % (name, off)
            if got is None:
                refused += 1
                self.assertEqual(ans, "R", where)
                continue
            self.assertNotEqual(ans, "R", where)
            v = [int(x) for x in ans.split()]
            nn = v[0]
            normals = [v[1 + 3 * i:4 + 3 * i] for i in range(nn)]
            k = 1 + 3 * nn
            nt = v[k]
            ptris = [v[k + 1 + 4 * i:k + 5 + 4 * i] for i in range(nt)]
            k += 1 + 4 * nt
            ne = v[k]
            pedges = [v[k + 1 + 4 * i:k + 5 + 4 * i] for i in range(ne)]
            gn, gt, ge = got
            self.assertEqual(len(gn), nn, where)
            merged += len(tris) - nn
            for (x, y, z, w), n in zip(gn, normals):
                for g, i in zip((x, y, z), n):
                    want = float(i) / 4096.0 / 4096.0
                    self.assertAlmostEqual(f32(g), want, delta=abs(want) * 1e-6 + 1e-12, msg=where)
                self.assertEqual(w, 0, where)
            self.assertEqual(len(gt), nt, where)
            for (ps, face), t in zip(gt, ptris):
                self.assertEqual(ps, [pos[t[i]] for i in range(3)], where)
                self.assertEqual(face, 530 + t[3], where)
            self.assertEqual(len(ge), ne, where)
            for (ps, faces), e in zip(ge, pedges):
                self.assertEqual(ps, [pos[e[0]], pos[e[1]]], where)
                self.assertEqual(faces, (530 + e[2], 530 + e[3]), where)
                open_edges += e[3] < 0
        print("\n%d distinct shadow mmats: %d refused, %d triangles merged into shared directions, %d open edges"
              % (len(seen), refused, merged, open_edges))

# ---------------------------------------------------------------------------
# the volume: ccShadowModel::Draw and VU1

CCSYS, DRAWENV_ACTIVE = inf_va(0x003788E0), inf_va(0x003788C4)
SYS, SCRATCH, VIEW, DRAWENV, CAM, PKT, VEC = (0x01000000, 0x01010000, 0x01020000, 0x01022000, 0x01023000,
                                             0x01024000, 0x01026000)
MODEL, BOX, CHUNKREF, PARAM, LW, WORK = 0x01027000, 0x01027100, 0x01027200, 0x01027300, 0x01027400, 0x01028000
ONE = 0x3F800000
BUF = 256
ADD, SUB = 0x48, 0x42


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def vu1_class():
    """test_effect_draw_rs's Vu1 (the MAC flag four pairs late) with the
    status flag's sticky sign and zero bits (FSSET, FSAND) and the address a
    program stopped at (for MSCNT)."""
    from test_effect_draw_rs import vu1_class as base

    class Vu1(base()):
        def __init__(self, blob):
            super().__init__(blob)
            self.status = 0
            self.sent = []

        def _flags(self, dest, vals):
            super()._flags(dest, vals)
            for k in range(4):
                if dest & (8 >> k):
                    v = vals[k] & 0xFFFFFFFF
                    if v >> 31:
                        self.status |= 0x82
                    if v & 0x7FFFFFFF == 0:
                        self.status |= 0x41

        def _lower(self, w, pc, vf, vi):
            op7 = w >> 25
            if op7 in (0x15, 0x16):
                imm = ((w >> 10) & 0x800) | (w & 0x7FF)
                if op7 == 0x15:
                    self.status = (self.status & 0x3F) | (imm & 0xFC0)
                else:
                    self.vi[(w >> 16) & 15] = self.status & imm
                return None
            n = len(self.kicks)
            r = super()._lower(w, pc, vf, vi)
            if len(self.kicks) > n:
                # What the GS takes, read as the kick sends it: the
                # program reuses the buffers after.
                self.sent.append(gif_triangles(self, self.kicks[-1]))
            return r

        def run(self, start, limit=2_000_000):
            pc, branch, ending, steps = start, None, False, 0
            import eemu
            self._e = eemu
            while True:
                steps += 1
                assert steps < limit
                pair = self.blob.pairs[pc]
                target = self._pair(pair.lo, pair.hi, pc)
                if ending:
                    self.resume = branch if branch is not None else pc + 1
                    return
                ending = bool(pair.hi >> 30 & 1)
                if branch is not None:
                    pc, branch = branch, None
                else:
                    pc += 1
                if target is not None:
                    branch = target
    return Vu1


def vif_run(u, data):
    """VIF1 over `data` (bytes) into VU `u`: unpacks (masked, with the
    double buffer's TOPS), STROW/STMASK/STCYCL, BASE/OFFSET, MSCAL, MSCNT."""
    q, row, mask, cl, wl = 0, [0] * 4, 0, 1, 1
    base = offset = dbf = 0
    tops = 0
    while q + 4 <= len(data):
        w = struct.unpack_from("<I", data, q)[0]
        q += 4
        cmd, imm, num = w >> 24 & 0x7F, w & 0xFFFF, w >> 16 & 0xFF
        if cmd in (0x00, 0x05, 0x06, 0x07, 0x10, 0x11, 0x13):
            continue
        if cmd == 0x01:
            cl, wl = imm & 0xFF, imm >> 8
        elif cmd == 0x02:
            offset, dbf, tops = imm & 0x3FF, 0, base
        elif cmd == 0x03:
            base = imm & 0x3FF
            tops = base + (offset if dbf else 0)
        elif cmd in (0x14, 0x17):
            u.top = tops
            dbf ^= 1
            tops = base + (offset if dbf else 0)
            u.run(imm if cmd == 0x14 else u.resume)
        elif cmd == 0x20:
            mask = struct.unpack_from("<I", data, q)[0]
            q += 4
        elif cmd == 0x30:
            row = list(struct.unpack_from("<4I", data, q))
            q += 16
        elif cmd >= 0x60:
            vn, vl, masked = cmd >> 2 & 3, cmd & 3, cmd & 0x10
            assert cl == wl
            size = {0: 4, 1: 2, 2: 1}[vl] * (vn + 1)
            n = num or 256
            raw = data[q:q + size * n]
            q += (size * n + 3) // 4 * 4
            addr = (imm & 0x3FF) + (tops if imm & 0x8000 else 0)
            fmt = {0: "I", 1: "H" if imm & 0x4000 else "h", 2: "B" if imm & 0x4000 else "b"}[vl]
            vals = struct.unpack("<%d%s" % (n * (vn + 1), fmt), raw)
            for i in range(n):
                dst = u.mem[(addr + i) % len(u.mem)]
                for k in range(4):
                    mk = (mask >> (8 * min(i % wl, 3) + 2 * k)) & 3 if masked else 0
                    if mk == 1:
                        dst[k] = row[k]
                    elif mk == 0 and k <= vn:
                        dst[k] = vals[i * (vn + 1) + k] & 0xFFFFFFFF
        else:
            raise AssertionError("VIF code %08x" % w)


def gif_triangles(u, addr):
    """The triangles the GS draws from the GIF packet at `addr`: (blend,
    [(x, y, z)] in GS pixels)."""
    out, verts, prim, alpha = [], [], 0, None
    q = addr
    while True:
        wd = u.mem[q % len(u.mem)]
        lo, hi = wd[0] | wd[1] << 32, wd[2] | wd[3] << 32
        nloop, eop, pre, flg, nreg = lo & 0x7FFF, lo >> 15 & 1, lo >> 46 & 1, lo >> 58 & 3, (lo >> 60) or 16
        q += 1
        assert flg == 0
        if pre:
            prim, verts = lo >> 47 & 7, []
        for _ in range(nloop):
            for r in range(nreg):
                reg, d = hi >> (4 * r) & 15, u.mem[q % len(u.mem)]
                q += 1
                if reg == 0xE:
                    if d[2] & 0xFF == 0x43:
                        alpha = d[0] | d[1] << 32
                elif reg == 0x5:
                    v = ((d[0] & 0xFFFF) / 16.0, (d[1] & 0xFFFF) / 16.0, d[2] & 0xFFFFFFFF)
                    verts.append(v)
                    adc = d[3] >> 15 & 1
                    if len(verts) >= 3 and not adc:
                        if prim == 3 and len(verts) % 3 == 0:
                            out.append((alpha, verts[-3:]))
                        elif prim == 4:
                            out.append((alpha, verts[-3:]))
                        elif prim == 5:
                            out.append((alpha, [verts[0], verts[-2], verts[-1]]))
                elif reg == 0x1:
                    pass
                else:
                    raise AssertionError("GIF register %x" % reg)
        if eop:
            return out


def raster(tris, depth, size=BUF):
    """Count +1 / -1 per pixel for the triangles whose Z at the pixel's
    corner beats `depth` (GS Z, larger nearer: ZTST GREATER). Z past VU1's
    ftoi4 saturates; the game cuts polygons at the buffer's edges before
    that, so it is the pixel's Z that saturates."""
    import numpy as np
    count = np.zeros((size, size), dtype=np.int32)
    # A hair past the pixel's corner, so that a corner on an edge two
    # triangles share falls in one of them (the GS's fill rule does as much).
    ys, xs = np.mgrid[0:size, 0:size].astype(np.float64)
    xs, ys = xs + 1 / 1024, ys + 1 / 2048
    for sign, (a, b, c) in tris:
        x0 = max(0, int(np.floor(min(a[0], b[0], c[0]))))
        x1 = min(size - 1, int(np.ceil(max(a[0], b[0], c[0]))))
        y0 = max(0, int(np.floor(min(a[1], b[1], c[1]))))
        y1 = min(size - 1, int(np.ceil(max(a[1], b[1], c[1]))))
        if x1 < x0 or y1 < y0:
            continue
        px, py = xs[y0:y1 + 1, x0:x1 + 1], ys[y0:y1 + 1, x0:x1 + 1]
        area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        if area == 0:
            continue
        w0 = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) / area
        w1 = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) / area
        w2 = 1 - w0 - w1
        inside = (w0 >= 0) & (w1 >= 0) & (w2 >= 0)
        z = np.minimum(w0 * a[2] + w1 * b[2] + w2 * c[2], 2147483647)
        inside &= z > depth[y0:y1 + 1, x0:x1 + 1]
        count[y0:y1 + 1, x0:x1 + 1] += np.where(inside, sign, 0)
    return count


class DrawMachine:
    """ccShadowModel::Draw in eemu, its packet through VIF1 and VU1."""

    def __init__(self):
        import vu
        from image import Program
        from test_anim import machine_class
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        self.dec = Decoder(self.prog, self.m)
        m, sym = self.m, lambda n: self.prog.symbol_named(n).value
        self.sym = sym
        self.mc = vu.Microcode(ELF)
        self.Vu1 = vu1_class()
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(SYS + 12, 2, 512)
        m.store(SYS + 14, 2, 448)
        m.store(SYS + 20, 4, 0x3F955555)
        for off, val in ((0x204, 0x457FF000), (0x200, 0x457FF000), (0x1E4, 0x457FF000), (0x1E0, 0x457FF000),
                         (0x1DC, 0x41000000), (0x1FC, 0x41000000), (0x1EC, 0x49800000), (0x20C, 0x49800000),
                         (0x25C, 0x447A0000), (0x250, 0x3F800000), (0x254, 0x4D800000)):
            m.store(VIEW + off, 4, val)
        self.call("SetFrame__6ccViewFffffffff", VIEW, fargs=[0, 0, fb(512), fb(384), fb(256), fb(192), ONE, ONE])
        m.store(DRAWENV_ACTIVE, 4, DRAWENV)
        m.store(CAM + 0x0C, 4, fb(45.0))
        m.hooks[sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm, *a: WORK
        m.hooks[sym("AddPacket__14ccShadowPacketFiPvPv")] = self.add
        m.hooks[sym("_ccCheckBoundingBoxEx__FPA4_fP6ccViewPA4_fi")] = lambda mm, *a: 1

    def call(self, name, *args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name), list(args), limit=50_000_000)

    def add(self, m, pkt, alpha, head, tail, *a):
        self.added = (alpha, head, tail)
        return 0

    def vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    def words(self, a, n):
        return [self.m.load(a + 4 * i, 4) for i in range(n)]

    def draw(self, pos, tris, scale, lw, eye, target, light, length, tw):
        """(view matrices and clip for the probe, the game's triangles as
        (+1 / -1, three (x, y, z) in buffer pixels))."""
        m = self.m
        if self.dec.decode(pos, tris) is None:
            return None
        vif, size, normals = self.dec.raw
        self.vec(VEC, [fb(v) for v in eye] + [ONE])
        self.vec(VEC + 16, [fb(v) for v in target] + [ONE])
        self.call("SetMatrix_PosTarget__5ccCamFPfPf", CAM, VEC, VEC + 16)
        self.call("SetView__6ccViewFRC5ccCamPA4_f", VIEW, CAM, 0)
        self.call("GetScreenClip__6ccViewFPf", VIEW, VEC + 32)
        view = dict(world_screen=self.words(VIEW + 0xD0, 16), view_screen=self.words(VIEW + 0x90, 16),
                    near=m.load(VIEW + 0x1DC, 4), far=m.load(VIEW + 0x1EC, 4), clip=self.words(VEC + 32, 4))
        # the model: +4 its box, +8 the chunk holding the stream, +0xc scale
        m.store(MODEL + 4, 4, BOX)
        m.store(MODEL + 8, 4, CHUNKREF)
        m.store(MODEL + 12, 4, fb(scale))
        m.store(CHUNKREF + 0xA, 2, normals)
        m.store(CHUNKREF + 0x30, 4, vif)
        m.store(CHUNKREF + 0x34, 4, size)
        self.vec(LW, [fb(v) for v in lw])
        m.store(PARAM + 0x90, 4, LW)
        m.store(DRAWENV + 0x8C, 4, PKT)
        self.vec(DRAWENV + 0x90, [fb(v) for v in light] + [0])
        m.store(DRAWENV + 0xA0, 4, fb(length))
        m.store(DRAWENV + 0xA4, 1, 0x80)
        m.store(PKT + 0x15C, 1, 1)
        m.store(PKT + 0x3C, 4, VIEW)
        m.store(PKT + 0x15A, 1, tw)
        m.store(PKT + 0x15B, 1, tw)
        self.added = None
        self.call("Draw__13ccShadowModelFP16ccDrawModelParam", MODEL, PARAM)
        if self.added is None:
            return view, []
        alpha, head, tail = self.added
        assert head == WORK and tail == WORK + 336, (hex(head), hex(tail))
        pkt = bytes(m.mem[WORK:WORK + 352])
        data = pkt[8:16] + pkt[16:320] + pkt[328:336] + bytes(m.mem[vif:vif + size]) + pkt[344:352]
        u = self.Vu1(self.mc.vu1)
        vif_run(u, data)
        off = (4096 - (1 << tw)) / 2
        out = []
        for tris in u.sent:
            for blend, vs in tris:
                sign = {ADD: 1, SUB: -1}[blend & 0xFF]
                out.append((sign, [(x - off, y - off, z) for x, y, z in vs]))
        return view, out


def poly_triangles(line):
    """The probe's polygons as fans: (+1 / -1, three (x, y, z))."""
    v = line.split()
    n, k, out = int(v[0]), 1, []
    for _ in range(n):
        front, nv = int(v[k]), int(v[k + 1])
        k += 2
        # VU1's ftoi4: 1/16 pixel, truncated.
        pts = [tuple(math.floor(f32(int(v[k + 3 * i + j], 16)) * 16) / 16 for j in range(3)) for i in range(nv)]
        k += 3 * nv
        for i in range(1, nv - 1):
            out.append((1 if front else -1, [pts[0], pts[i], pts[i + 1]]))
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(DATA) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class VolumeAgainstGame(unittest.TestCase):
    CASES = int(os.environ.get("PINEY_SHADOW_CASES", "300"))

    def test_volumes(self):
        import random
        import numpy as np
        build()
        rng = random.Random(7)
        mmats = shadow_mmats()
        seen, distinct = set(), []
        for _f, _o, pos, tris, sc in mmats:
            key = (tuple(pos), tuple(tris), sc)
            if key not in seen:
                seen.add(key)
                distinct.append((pos, tris, sc))
        dm = DrawMachine()
        reqs, games = [], []
        for case in range(self.CASES):
            pos, tris, scale = rng.choice(distinct)
            a, tilt = rng.uniform(-math.pi, math.pi), rng.uniform(-0.3, 0.3)
            ca, sa, ct, st = math.cos(a), math.sin(a), math.cos(tilt), math.sin(tilt)
            mirror = -1.0 if rng.random() < 0.1 else 1.0
            t = [rng.uniform(-200, 200), rng.uniform(-200, 200), rng.uniform(-50, 100)]
            lw = [mirror * ca, mirror * sa, 0, 0, -sa * ct, ca * ct, st, 0, sa * st, -ca * st, ct, 0] + t + [1]
            dist = rng.choice([150, 400, 900, 1600])
            yaw, pitch = rng.uniform(-math.pi, math.pi), rng.uniform(0.1, 1.2)
            target = [t[0] + rng.uniform(-50, 50), t[1] + rng.uniform(-50, 50), t[2] + rng.uniform(0, 80)]
            eye = [target[0] + dist * math.cos(yaw) * math.cos(pitch), target[1] + dist * math.sin(yaw) * math.cos(pitch),
                   target[2] + dist * math.sin(pitch)]
            lx, ly = rng.uniform(-0.8, 0.8), rng.uniform(-0.8, 0.8)
            lz = -math.sqrt(max(0.05, 1 - lx * lx - ly * ly)) if rng.random() < 0.85 else rng.uniform(-1, 1)
            ln = math.sqrt(lx * lx + ly * ly + lz * lz)
            light = [lx / ln, ly / ln, lz / ln]
            length = rng.choice([150.0, 300.0, 450.0, rng.uniform(20, 900)])
            tw = rng.choice([8, 8, 7])
            got = dm.draw(pos, tris, scale, lw, eye, target, light, length, tw)
            if got is None:
                continue
            view, game = got
            hx = lambda x: "%08x" % (x & 0xFFFFFFFF)  # noqa: E731
            req = [len(pos), len(tris)] + [c for p in pos for c in p] + [i for tr in tris for i in tr]
            req = " ".join(map(str, req)) + " " + " ".join(
                [hx(fb(scale))] + [hx(fb(v)) for v in lw] + [hx(v) for v in view["world_screen"]] +
                [hx(view["near"]), hx(view["far"])] +
                [hx(v) for v in view["clip"]] + [hx(fb(v)) for v in light] + ["00000000", hx(fb(length))] +
                ["1", str(1 << tw), str(1 << tw)])
            reqs.append(req)
            games.append((case, tw, game))
        p = subprocess.run([VOLUME], input="\n".join(reqs) + "\n", capture_output=True, text=True, check=True)
        answers = p.stdout.splitlines()
        self.assertEqual(len(answers), len(reqs))
        bad, total, drawn = 0, 0, 0
        for (case, tw, game), ans in zip(games, answers):
            port = poly_triangles(ans)
            size = 1 << tw
            if os.environ.get("PINEY_SHADOW_DUMP") == str(case):
                v = ans.split()
                k = 1
                for _ in range(int(v[0])):
                    fr, nv = int(v[k]), int(v[k + 1])
                    k += 2
                    print("poly %+d" % (1 if fr else -1), " ".join("(%.1f %.1f %.4g)" % tuple(
                        f32(int(v[k + 3 * i + j], 16)) for j in range(3)) for i in range(nv)))
                    k += 3 * nv
                for name, tl in (("game", game), ("port", port)):
                    print(name)
                    for sg, vs in sorted(tl, key=lambda t: (t[0], sorted((round(v[0]), round(v[1])) for v in t[1]))):
                        print("  %+d" % sg, " ".join("(%.2f %.2f %.0f)" % v for v in vs))
            drawn += bool(game)
            for depth in (0.0, None):
                if depth is None:
                    zs = [v[2] for _s, vs in game for v in vs] or [0.0]
                    depth = np.full((size, size), float(np.median(zs)))
                else:
                    depth = np.full((size, size), depth)
                g, r = raster(game, depth, size), raster(port, depth, size)
                if os.environ.get("PINEY_SHADOW_DUMP") == str(case) and (g != r).any():
                    ys_, xs_ = np.nonzero(g != r)
                    py, px = int(ys_[0]), int(xs_[0])
                    print("pixel", px, py, "game", int(g[py, px]), "port", int(r[py, px]))
                    for name, tl in (("game", game), ("port", port)):
                        for t in tl:
                            c = raster([t], depth, size)
                            if c[py, px]:
                                print(" ", name, t[0], " ".join("(%.2f %.2f %.4g)" % v for v in t[1]))
                diff = int((g != r).sum())
                total += 1
                if os.environ.get("PINEY_SHADOW_VERBOSE"):
                    print("case %d tw %d: game %d tris, port %d; shadowed %d / %d, %d differ" % (
                        case, tw, len(game), len(port), int((g > 0).sum()), int((r > 0).sum()), diff))
                if diff > max(8, int(0.01 * ((g != 0) | (r != 0)).sum())):
                    bad += 1
                    print("case %d: %d pixels differ (game %d shadowed, port %d)" % (case, diff, int((g > 0).sum()),
                                                                                       int((r > 0).sum())))
        print("\n%d draws (%d with a volume), %d comparisons, %d off" % (len(games), drawn, total, bad))
        self.assertLessEqual(100 * bad, total)


if __name__ == "__main__":
    unittest.main()
