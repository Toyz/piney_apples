#!/usr/bin/env python3
"""The shadow volumes against the game: crates/piney-data's ShadowMesh::build
against DecodeShadowModel, and crates/piney-desktop's shadow::draw and kicks
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
the model is refused, each direction (the float bits the stream carries,
against the port's and against SetShadowNormal2's vitof12 and divide by
4096 of the port's integer cross product), and each triangle and edge in
order - its vertex positions and the directions on its sides (an open
edge's second side is -1 - its face, which the stream carries as 529 -
face).

VolumeAgainstGame. ccShadowModel::Draw (0x001412b0) runs in eemu with a
real ccView (SetFrame, a camera placed by ccCam::SetMatrix_PosTarget and
ccView::SetView), a random world matrix (turned, tilted, now and then
mirrored; one draw in eight looking away from the model), a random light
(mostly downward), length and buffer size, and a shadow mmat decoded as
above, its box made by the game's ccBbox_SetBox from what the decoder
found. Its box check (_ccCheckBoundingBoxEx) runs mc0_CheckBoundingBoxShadow
in tools/vu.py's VU0, and the packet it adds goes through a VIF1 model
(STCYCL, STMASK and STROW with masked unpacks, BASE/OFFSET and the double
buffer's TOPS, MSCAL and MSCNT) into tools/vu.py's VU1; both VUs are kept
from one draw to the next, as on the console. The flags reach FMAND and
FSAND four pairs after the instruction that set them, FSSET's write among
them in order (flagged). What each XGKICK sends is read when kicked. The
shadow_volume_probe example runs the port on the same draws, carrying its
own VU0 vf21 and VU1 memory, and the two are compared exactly: the box,
whether the check lets the draw on and VU0's vf21 after it, every word of
every kick; then each side's GS model counts the buffer (+1 for the add
blend, -1 for the subtract; a pixel whose top-left corner is inside, top
and left edges in; Z the plane's, truncated, GREATER than a flat depth of
0 and of the draw's median) and every pixel is compared. The models with
open edges are drawn eight times more, after the random ones.

Skipped when the disc is not extracted or cargo is missing.
PINEY_CARGO_ROOT names another cargo workspace to build the examples in;
PINEY_SHADOW_CASES sets the number of random draws (default 300),
PINEY_SHADOW_VERBOSE prints each, PINEY_SHADOW_DUMP=N the kicks of draw N.
"""

import collections
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
ONE, F4096 = 0x3F800000, 0x45800000
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
            for s in m.mmats:
                out.append((mm.name, m.offset, list(s.stored), [tuple(t) for t in s.triangles], m.scale))
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
        # Decode_Model's start (main 0x0014bddc).
        for i, v in enumerate((0x10000, -0x10000) * 3):
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


def direction(i):
    """SetShadowNormal2's float for an integer cross product lane: vitof12
    (cvt.s.w, a divide by 4096), then sceVu0DivVector by 4096 (vdiv's Q =
    1 / 4096, vmulq), in the EE's arithmetic."""
    from eemu import f_div, f_from_int, f_mul
    return f_mul(f_div(f_from_int(i), F4096), f_div(ONE, F4096))


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
            normals = [v[1 + 6 * i:7 + 6 * i] for i in range(nn)]
            k = 1 + 6 * nn
            nt = v[k]
            ptris = [v[k + 1 + 4 * i:k + 5 + 4 * i] for i in range(nt)]
            k += 1 + 4 * nt
            ne = v[k]
            pedges = [v[k + 1 + 4 * i:k + 5 + 4 * i] for i in range(ne)]
            gn, gt, ge = got
            self.assertEqual(len(gn), nn, where)
            merged += len(tris) - nn
            for (x, y, z, w), n in zip(gn, normals):
                self.assertEqual([x, y, z], [direction(i) for i in n[:3]], where)
                self.assertEqual([x, y, z], n[3:], where)
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
# ccObj::Draw's owner: the object, its parent, its own model and box, the
# model's mmat list and mmat, the active layer.
OBJ, PARENT, OWN, OWNBOX, MMATS, MMAT, LAYER = (0x01029000, 0x01029100, 0x01029200, 0x01029300, 0x01029400,
                                               0x01029500, 0x01029600)
ACTIVE_LAYER = inf_va(0x003788D8)
BUF = 256
ADD = 0x48  # alphaBlendTbl[1]


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def flagged(base):
    """tools/vu.py's Vu with the MAC and status flags read as the code
    reads them: a flag-setting upper instruction's MAC flag and status bits
    reach FMAND and FSAND four pairs later (every test in the shadow, model
    and box programs stands four pairs after what it tests), and FSSET's
    write goes in among them in order, so the FSAND after a guard band
    test sees that test's subtractions only. Also the GIF packet of each
    XGKICK, read when kicked, and the address a program stopped at."""

    class Flagged(base):
        def __init__(self, blob):
            super().__init__(blob)
            self.step = 0
            self.pending = collections.deque()
            self.status = 0
            self.mac_seen = 0
            self.sent = []

        def _pair(self, lo, hi, pc):
            self.step += 1
            return super()._pair(lo, hi, pc)

        def _flags(self, dest, vals):
            mac = 0
            for k in range(4):
                if dest & (8 >> k):
                    v = vals[k] & 0xFFFFFFFF
                    if not v & 0x7FFFFFFF:
                        mac |= 1 << (3 - k)
                    if v & 0x80000000:
                        mac |= 1 << (7 - k)
            self.pending.append((self.step, "mac", mac))

        def _settle(self):
            while self.pending and self.pending[0][0] <= self.step - 4:
                _, kind, v = self.pending.popleft()
                if kind == "mac":
                    self.mac_seen = v
                    s, z = int(bool(v & 0xF0)), int(bool(v & 0x0F))
                    self.status = (self.status & ~0x3) | s << 1 | z | s << 7 | z << 6
                else:
                    self.status = (self.status & 0x3F) | (v & 0xFC0)

        def _lower(self, w, pc, vf, vi):
            self._settle()
            self.mac = self.mac_seen
            op7 = w >> 25
            if op7 in (0x15, 0x16):
                imm = ((w >> 10) & 0x800) | (w & 0x7FF)
                if op7 == 0x15:
                    self.pending.append((self.step, "set", imm))
                else:
                    r = (w >> 16) & 15
                    if r:
                        self.vi[r] = self.status & imm
                return None
            n = len(self.kicks)
            r = super()._lower(w, pc, vf, vi)
            if len(self.kicks) > n:
                self.sent.append(gif_packet(self, self.kicks[-1]))
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
    return Flagged


def gif_packet(u, addr):
    """The GIF packet (PACKED) at VU1 `addr`, tag by tag to EOP, as words."""
    out, q, size = [], addr, len(u.mem)
    while True:
        tag = list(u.mem[q % size])
        assert tag[1] >> 26 & 3 == 0
        out.append(tag)
        q += 1
        for _ in range((tag[0] & 0x7FFF) * ((tag[1] >> 28) or 16)):
            out.append(list(u.mem[q % size]))
            q += 1
        if tag[0] & 0x8000:
            return out


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


def gs_triangles(kicks, offset, add=ADD):
    """The triangles the GS draws of the kicks (PATH1, PACKED): a PRE tag
    sets PRIM and starts the vertices afresh, A+D to ALPHA_2 sets the blend,
    an XYZ2 without ADC draws a TRIANGLE's third vertex, a TRISTRIP's last
    three, a TRIFAN's first and last two. (+1 / -1, [(x, y)] in sixteenths
    of the buffer's pixels, [z])."""
    out, prim, alpha, verts = [], 0, 0, []
    for kick in kicks:
        q = 0
        while q < len(kick):
            tag = kick[q]
            q += 1
            nreg, regs = (tag[1] >> 28) or 16, tag[2] | tag[3] << 32
            if tag[1] >> 14 & 1:
                prim, verts = tag[1] >> 15 & 0x7FF, []
            for _ in range(tag[0] & 0x7FFF):
                for r in range(nreg):
                    d = kick[q]
                    q += 1
                    reg = regs >> (4 * r) & 0xF
                    if reg == 0xE and d[2] & 0xFF == 0x43:
                        alpha = d[0] | d[1] << 32
                    elif reg == 0x5:
                        verts.append(((d[0] & 0xFFFF) - offset, (d[1] & 0xFFFF) - offset, d[2]))
                        n = len(verts)
                        c = None
                        if prim & 7 == 3 and n % 3 == 0 or prim & 7 == 4 and n >= 3:
                            c = verts[-3:]
                        elif prim & 7 == 5 and n >= 3:
                            c = [verts[0], verts[-2], verts[-1]]
                        if c and not d[3] & 0x8000:
                            out.append((1 if alpha == add else -1, c))
            if tag[0] & 0x8000:
                break
    return out


def raster(tris, depth, size=BUF):
    """Count +1 / -1 per buffer pixel as the repo models the GS: a pixel
    whose top-left corner lies inside a triangle, or on a top or a left
    edge of it, with the plane's Z there (truncated) above `depth` (ZTST
    GREATER, unsigned). Edge functions in integers; Z exactly (doubles,
    and integers where they cannot decide)."""
    import numpy as np
    count = np.zeros((size, size), dtype=np.int64)
    for sign, vs in tris:
        (ax, ay, az), (bx, by, bz), (cx, cy, cz) = vs
        area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
        if area < 0:
            (bx, by, bz), (cx, cy, cz) = (cx, cy, cz), (bx, by, bz)
            area = -area
        if area == 0:
            continue
        x0, y0 = max(0, -((-min(ax, bx, cx)) // 16)), max(0, -((-min(ay, by, cy)) // 16))
        x1, y1 = min(size - 1, max(ax, bx, cx) // 16), min(size - 1, max(ay, by, cy) // 16)
        if x1 < x0 or y1 < y0:
            continue
        ys, xs = np.mgrid[y0:y1 + 1, x0:x1 + 1].astype(np.int64) * 16
        ok = np.ones(xs.shape, dtype=bool)
        es = []
        for (px, py), (qx, qy) in (((bx, by), (cx, cy)), ((cx, cy), (ax, ay)), ((ax, ay), (bx, by))):
            e = (qx - px) * (ys - py) - (qy - py) * (xs - px)
            top_left = (qy - py) < 0 or ((qy - py) == 0 and (qx - px) > 0)
            ok &= (e > 0) | ((e == 0) if top_left else False)
            es.append(e)
        if not ok.any():
            continue
        zf = (es[0] * float(az) + es[1] * float(bz) + es[2] * float(cz)) / float(area)
        passes = zf >= depth + 1
        near = ok & (np.abs(zf - (depth + 1)) < 1e-3)
        for j, i in zip(*np.nonzero(near)):
            s = int(es[0][j, i]) * az + int(es[1][j, i]) * bz + int(es[2][j, i]) * cz
            passes[j, i] = s // area > depth
        count[y0:y1 + 1, x0:x1 + 1] += np.where(ok & passes, sign, 0)
    return count


def vu0_machine_class():
    """A machine with VU0's integer registers and data memory, which
    ccCoord::_SetLWMatrix walks its parent chain in: eemu_rs's Vu0Machine,
    else test_stream_rs's."""
    try:
        import eemu_rs
        return eemu_rs.Vu0Machine
    except ImportError:
        from test_stream_rs import machine_class
        return machine_class()


class DrawMachine:
    """ccShadowModel::Draw in eemu, its box check in VU0 and its packet
    through VIF1 and VU1, both kept from one draw to the next."""

    def __init__(self):
        import vu
        from image import Program
        self.prog = Program(ELF, "gcmn")
        self.m = vu0_machine_class()(self.prog)
        self.dec = Decoder(self.prog, self.m)
        m, sym = self.m, lambda n: self.prog.symbol_named(n).value
        self.sym = sym
        self.mc = vu.Microcode(ELF)
        self.vu1 = flagged(vu.Vu)(self.mc.vu1)
        self.vu0 = flagged(vu.Vu)(self.mc.vu0)
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
        m.hooks[sym("_ccCheckBoundingBoxEx__FPA4_fP6ccViewPA4_fi")] = self.check
        m.hooks[sym("ccSetMatrixPacket__FP16ccDrawModelParam")] = lambda mm, *a: 0

    def call(self, name, *args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name), list(args), limit=50_000_000)

    def add(self, m, pkt, alpha, head, tail, *a):
        self.added = (alpha, head, tail)
        return 0

    def check(self, m, box, view, mat, prog, *a):
        """_ccCheckBoundingBoxEx (0x0013e9e0): vf13-vf16 the view's +0x1f0,
        +0x200, +0x1d0, +0x1e0, vf1-vf4 the matrix, vf18-vf20 the three
        vectors, then the VU0 program at CMSAR0; v0 is its vi01. vf21 is
        VU0's one register: as the EE's macro code (_SetLWMatrix) left it
        going in, the program's coming out."""
        u = self.vu0
        u.vf[21] = list(m.vf[21])
        if prog & 0xFFFF == 0:
            self.stale = list(u.vf[21])
        for r, a in ((13, view + 496), (14, view + 512), (15, view + 464), (16, view + 480),
                     (1, mat), (2, mat + 16), (3, mat + 32), (4, mat + 48), (18, box), (19, box + 16),
                     (20, box + 32)):
            u.vf[r] = self.words(a, 4)
        u.run(prog & 0xFFFF)
        self.vf21 = list(u.vf[21])
        m.vf[21] = list(u.vf[21])
        return u.vi[1]

    def owner_draw(self, lw, box, tp):
        """ccObj::Draw (0x0013f220) of an object at local matrix `lw` under a
        parent whose world is the unit, the shadow model MODEL its +0x9c:
        its _SetLWMatrix, then, with `box` (a rigid model with the shadow
        model's box) and `tp` above 1/128, ccModel::Draw's check of its
        model, then the shadow. ccSetMatrixPacket says no, so the model is not drawn; an
        empty model (+0x22 0) returns at once."""
        m = self.m
        for a in (OBJ, PARENT, OWN, OWNBOX, MMATS, MMAT):
            m.mem[a:a + 0x100] = bytes(0x100)
        unit = [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE]
        self.vec(PARENT, unit)
        self.vec(OBJ + 0x40, [fb(v) for v in lw])
        for off, size, val in ((0x80, 4, PARENT), (0x84, 4, fb(tp)), (0x8D, 1, 1), (0x94, 4, OWN), (0x9C, 4, MODEL),
                               (0xA0, 1, 6), (0xA2, 1, 3)):
            m.store(OBJ + off, size, val)
        if box:
            box = self.words(BOX, 8)  # the shadow model's own box
            self.vec(OWNBOX, box + [0, 0, 0, ONE])
            m.store(OWN + 4, 4, OWNBOX)
            m.store(OWN + 8, 4, MMATS)
            m.store(OWN + 0x22, 2, 1)
            m.store(MMATS, 4, MMAT)
            m.store(MMAT + 16, 4, ONE)
        m.store(ACTIVE_LAYER, 4, LAYER)
        m.store(LAYER + 0x2C, 4, VIEW)
        self.call("Draw__5ccObjFf", OBJ, fargs=[ONE])

    def vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    def words(self, a, n):
        return [self.m.load(a + 4 * i, 4) for i in range(n)]

    def draw(self, pos, tris, scale, lw, eye, target, light, length, tw, owner=None):
        """(the view for the probe, the box, whether a packet was added,
        VU0's vf21 after the check, the kicks VU1 sent); None when refused.
        With `owner`, through ccObj::Draw (owner_draw)."""
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
        # The model: +4 its box (ccBbox_SetBox of what the decoder found),
        # +8 the chunk holding the stream, +0xc the scale.
        m.f[12] = fb(scale)
        m.call(self.sym("ccBbox_SetBox__FP6ccBboxP14ccVertexMinMumf"), [BOX, MINMAX], limit=100_000)
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
        self.added, self.vf21 = None, None
        if owner is None:
            self.call("Draw__13ccShadowModelFP16ccDrawModelParam", MODEL, PARAM)
        else:
            self.owner_draw(lw, *owner)
        box = self.words(BOX, 8)
        if self.added is None:
            return view, box, False, self.vf21, []
        alpha, head, tail = self.added
        assert head == WORK and tail == WORK + 336, (hex(head), hex(tail))
        pkt = bytes(m.mem[WORK:WORK + 352])
        data = pkt[8:16] + pkt[16:320] + pkt[328:336] + bytes(m.mem[vif:vif + size]) + pkt[344:352]
        self.vu1.sent = []
        vif_run(self.vu1, data)
        return view, box, True, self.vf21, self.vu1.sent


def has_open_edge(ans):
    """Whether shadow_probe's answer has an edge open on one side."""
    v = [int(x) for x in ans.split()]
    k = 1 + 6 * v[0]
    k += 1 + 4 * v[k]
    return any(v[k + 4 + 4 * i] < 0 for i in range(v[k]))


def placements(rng, n):
    """Random draws: a model, its world matrix (turned, tilted, now and then
    mirrored), a camera, a light (mostly downward), a length and a buffer
    size; one in eight stands off the screen or behind the camera."""
    for case in range(n):
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
        if rng.random() < 0.125:
            # Look away: the model off to a side or behind.
            turn = rng.uniform(0.6, math.pi)
            dx, dy = target[0] - eye[0], target[1] - eye[1]
            c, s = math.cos(turn), math.sin(turn)
            target = [eye[0] + c * dx - s * dy, eye[1] + s * dx + c * dy, target[2]]
        lx, ly = rng.uniform(-0.8, 0.8), rng.uniform(-0.8, 0.8)
        lz = -math.sqrt(max(0.05, 1 - lx * lx - ly * ly)) if rng.random() < 0.85 else rng.uniform(-1, 1)
        ln = math.sqrt(lx * lx + ly * ly + lz * lz)
        light = [lx / ln, ly / ln, lz / ln]
        length = rng.choice([150.0, 300.0, 450.0, rng.uniform(20, 900)])
        tw = rng.choice([8, 8, 7])
        yield case, lw, eye, target, light, length, tw


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(DATA) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class VolumeAgainstGame(unittest.TestCase):
    CASES = int(os.environ.get("PINEY_SHADOW_CASES", "300"))

    def test_volumes(self):
        import random
        import statistics
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
        # Then the models with open edges (whose second face's flag is read
        # below the directions, where the clip path's buffers are), each a
        # few times.
        probe = subprocess.run([EXAMPLE], input="\n".join(
            " ".join(str(v) for v in [len(p), len(t)] + [c for q in p for c in q] + [i for x in t for i in x])
            for p, t, _s in distinct) + "\n", capture_output=True, text=True, check=True).stdout.splitlines()
        open_models = [d for d, ans in zip(distinct, probe) if ans != "R" and has_open_edge(ans)]
        models = [None] * self.CASES + [m for m in open_models for _ in range(8)]
        dm = DrawMachine()
        reqs, games = [], []
        for (case, lw, eye, target, light, length, tw), model in zip(placements(rng, len(models)), models):
            pos, tris, scale = model or rng.choice(distinct)
            got = dm.draw(pos, tris, scale, lw, eye, target, light, length, tw)
            if got is None:
                continue
            view, box, drawn, vf21, kicks = got
            off = (4096 - (1 << tw)) * 8
            tris_g = gs_triangles(kicks, off)
            zs = [v[2] for _s, vs in tris_g for v in vs]
            depths = [0, statistics.median_low(zs) if zs else 0]
            hx = lambda x: "%08x" % (x & 0xFFFFFFFF)  # noqa: E731
            req = [len(pos), len(tris)] + [c for p in pos for c in p] + [i for tr in tris for i in tr]
            req = " ".join(map(str, req)) + " " + " ".join(
                [hx(fb(scale))] + [hx(fb(v)) for v in lw] + [hx(v) for v in view["world_screen"]] +
                [hx(v) for v in view["view_screen"]] + [hx(view["near"]), hx(view["far"])] +
                [hx(v) for v in view["clip"]] + [hx(fb(v)) for v in light] + ["00000000", hx(fb(length))] +
                ["1", str(tw), str(tw)] + [str(d) for d in depths])
            reqs.append(req)
            games.append((case, tw, box, drawn, vf21, kicks, tris_g, depths))
        p = subprocess.run([VOLUME], input="\n".join(reqs) + "\n", capture_output=True, text=True, check=True)
        answers = p.stdout.splitlines()
        self.assertEqual(len(answers), len(reqs))
        culled = clipped = words = pixels = 0
        for (case, tw, box, drawn, vf21, kicks, tris_g, depths), ans in zip(games, answers):
            v = ans.split()
            where = "case %d" % case
            self.assertEqual([int(x, 16) for x in v[:8]], box, where + ": the box")
            self.assertEqual(int(v[8]), int(drawn), where + ": the check")
            self.assertEqual([int(x, 16) for x in v[9:13]], vf21, where + ": VU0's vf21")
            k, port = 14, []
            for _ in range(int(v[13])):
                n = int(v[k])
                port.append([[int(x, 16) for x in v[k + 1 + 4 * i:k + 5 + 4 * i]] for i in range(n)])
                k += 1 + 4 * n
            if os.environ.get("PINEY_SHADOW_DUMP") == str(case):
                for name, ks in (("game", kicks), ("port", port)):
                    print(name)
                    for kk in ks:
                        print("  kick", " ".join("%08x" % x for q in kk for x in q))
            self.assertEqual(len(port), len(kicks), where + ": the kicks")
            for i, (g, q) in enumerate(zip(kicks, port)):
                self.assertEqual(q, g, where + ": kick %d" % i)
                words += 4 * len(g)
            culled += not drawn
            clipped += sum(1 for kk in kicks if kk[0][1] == 0x1122C000) // 2
            size = 1 << tw
            for depth in depths:
                g = raster(tris_g, depth, size)
                r = np.zeros((size, size), dtype=np.int64)
                for j in range(int(v[k])):
                    i, c = v[k + 1 + j].split(":")
                    r.flat[int(i)] = int(c)
                k += 1 + int(v[k])
                diff = int((g != r).sum())
                if os.environ.get("PINEY_SHADOW_VERBOSE"):
                    print("case %d tw %d depth %d: %d triangles, %d pixels counted, %d differ"
                          % (case, tw, depth, len(tris_g), int((g != 0).sum()), diff))
                self.assertEqual(diff, 0, where + ": depth %d" % depth)
                pixels += size * size
        print("\n%d draws (%d of models with open edges; %d culled, %d clipped polygons): %d kicked words and "
              "%d pixels the same" % (len(games), len(models) - self.CASES, culled, clipped, words, pixels))



@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(DATA) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class OwnerAgainstGame(unittest.TestCase):
    """ccObj::Draw's real sequence before the shadow's check (the object's
    _SetLWMatrix, its model's ccModel::Draw) against the port's Prelude:
    with the model seen its check's last corner is vf21 word for word; else
    vf21 is the qword VU0's stack holds above the object, its parent's
    address over $a0's sign and upper half, which the port reads as 0."""
    CASES = int(os.environ.get("PINEY_SHADOW_OWNERS", "90"))

    def test_owner_sequences(self):
        import random
        build()
        rng = random.Random(11)
        distinct = {(tuple(p), tuple(t), sc): (p, t, sc) for _f, _o, p, t, sc in shadow_mmats()}
        distinct = list(distinct.values())
        dm = DrawMachine()
        reqs, games = [], []
        kinds = ("seen", "empty", "faded")
        for (case, lw, eye, target, light, length, tw), _ in zip(placements(rng, self.CASES), range(self.CASES)):
            kind = kinds[case % 3]
            pos, tris, scale = rng.choice(distinct)
            owner = (kind != "empty", 0.005 if kind == "faded" else 1.0)
            got = dm.draw(pos, tris, scale, lw, eye, target, light, length, tw, owner=owner)
            if got is None:
                continue
            view, box, drawn, vf21, kicks = got
            hx = lambda x: "%08x" % (x & 0xFFFFFFFF)  # noqa: E731
            req = [len(pos), len(tris)] + [c for p in pos for c in p] + [i for tr in tris for i in tr]
            prelude = ["2"] + [str(w) for w in box[4:8]] if kind == "seen" else ["1"]
            req = " ".join(map(str, req)) + " " + " ".join(
                [hx(fb(scale))] + [hx(fb(v)) for v in lw] + [hx(v) for v in view["world_screen"]] +
                [hx(v) for v in view["view_screen"]] + [hx(view["near"]), hx(view["far"])] +
                [hx(v) for v in view["clip"]] + [hx(fb(v)) for v in light] + ["00000000", hx(fb(length))] +
                ["1", str(tw), str(tw), "0", "0"] + prelude)
            reqs.append(req)
            games.append((case, kind, box, drawn, vf21, list(dm.stale), kicks))
        p = subprocess.run([VOLUME], input="\n".join(reqs) + "\n", capture_output=True, text=True, check=True)
        answers = p.stdout.splitlines()
        self.assertEqual(len(answers), len(reqs))
        counts = {k: 0 for k in kinds}
        for (case, kind, box, drawn, vf21, stale, kicks), ans in zip(games, answers):
            v = ans.split()
            where = "case %d (%s)" % (case, kind)
            if kind == "seen":
                self.assertNotEqual(stale[:2], [PARENT, 0], where)
                self.assertEqual([int(x, 16) for x in v[9:13]], vf21, where + ": VU0's vf21")
            else:
                self.assertEqual(stale[:2], [PARENT, 0], where + ": the stack's qword")
            self.assertEqual([int(x, 16) for x in v[:8]], box, where + ": the box")
            self.assertEqual(int(v[8]), int(drawn), where + ": the check")
            k, port = 14, []
            for _ in range(int(v[13])):
                n = int(v[k])
                port.append([[int(x, 16) for x in v[k + 1 + 4 * i:k + 5 + 4 * i]] for i in range(n)])
                k += 1 + 4 * n
            self.assertEqual(port, kicks, where + ": the kicks")
            counts[kind] += 1
        print("\nccObj::Draw before the shadow: %s, the same" % ", ".join("%d %s" % (n, k) for k, n in counts.items()))


if __name__ == "__main__":
    unittest.main()
