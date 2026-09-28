#!/usr/bin/env python3
"""tools/anim.py against the game's own animation code.

The game's functions run in tools/eemu.py, extended here (VuMachine) with
the VU0 macro instructions, lqc2/sqc2, ldl/ldr and the MMI instructions the
animation and matrix code reaches. VU0 arithmetic uses eemu's FPU model:
every product and sum truncated, a multiply-add rounding its product first.
ccMalloc and operator new are a bump allocator, operator delete and ccFree
do nothing, and ccRingBufferTh::OpenData points the reader at data already
in memory; everything else is the game's code.

- Controllers: ccAnmChunk::ConvLCNum2ALCNum compiles an Anime chunk (read
  through the game's ccRingBufferTh), ccAnm::InitAnmCtrlWork builds each
  object controller's work, and ccAnm::ResetAnmCtrlWork + SetAnmCtrlWork
  evaluate it at a time. SetAnmCtrlWork's scratch keeps the position,
  rotation matrix, scale matrix and float it composed; anim.py must give the
  same position, scale and transparency bit for bit, the same single-value
  rotation matrix bit for bit, and keyed rotations to within 1e-5 plus
  1e-7 per key passed, per matrix element (the game goes through sinf,
  cosf, a double atan2 and float matrix products that anim.py does in
  double, and its running matrix drifts a little at every key). Material
  records' texture offsets, and what they write into a ccMaterial, bit for
  bit. Stepping to a time in several calls must give what one call does.
- Playback: ccAnm::_AnimateForward over a whole animation, with its own
  DecodeFrameChunk: the frame counter, the return value, every object's
  matrix and every morpher's targets and weights after each step, against
  Animation.forward / poses / morph_weights.
- Morphing: ccMorpher::Modify on real base and target models against
  anim.morph.
- Notes: SetAnm's state, then for each random step (0, a frame, odd
  values, several frames, past the end) ccAnm::_AnimateForward and
  ccAnm::NoteProcess with a hooked funcNoteProcess recording each note it
  is handed: the frame counter, the return value and every note's event,
  param, time and object, in order, against Animation.forward /
  passed_notes. A second NoteProcess must hand on the same notes.

    python3 tools/test_anim.py                  the unit tests
    python3 tools/test_anim.py bulk N [SEED]    N random animations, with counts
    python3 tools/test_anim.py notes [N] [SEED] N random animations with F_Note
                                                records (all by default), with counts
    python3 tools/test_anim.py fixture          expectations for the Rust test
                                                (crates/piney-data/tests/anim_fixture.txt)
"""

import collections
import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
DATA = volume.DATA
ISO = volume.ISO

M32 = 0xFFFFFFFF
M64 = (1 << 64) - 1
M128 = (1 << 128) - 1
ONE = 0x3F800000
# Keyed rotations: the game's running matrix picks up float error at every
# key it multiplies in; anim.py works in double.
ROT_TOLERANCE, ROT_PER_KEY = 1e-5, 1e-7

SYS, RB, CHUNK, DATAPTR, NUM = 0x01000000, 0x01000400, 0x01000800, 0x01000840, 0x01000844
TBL, ANM, STREAM, IDX = 0x01001000, 0x01003000, 0x01003400, 0x01004000
MORPH_PARAM, SCRATCH, SCRATCH_END = 0x01008000, 0x01010000, 0x01080000
NOTE_FUNC = 0x01000900      # funcNoteProcess: a hook records each note
DATA_BUF, DATA_END = 0x01100000, 0x01200000
HEAP, HEAP_END = 0x01200000, 0x01F00000


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= M32
    return v - (1 << 32) if v & 0x80000000 else v


def halves(v):
    return [(v >> (16 * i)) & 0xFFFF for i in range(8)]


def words(v):
    return [(v >> (32 * i)) & M32 for i in range(4)]


def from_halves(h):
    return sum((x & 0xFFFF) << (16 * i) for i, x in enumerate(h))


def from_words(w):
    return sum((x & M32) << (32 * i) for i, x in enumerate(w))


def machine_class():
    """The machine the harnesses run the game's code on: crates/piney-eemu's
    VuMachine (the eemu_rs module, identical instruction for instruction and
    checked so by tools/test_eemu_rs.py) when it is built into tools/, else
    the Python VuMachine below. PINEY_EEMU=py forces the Python one."""
    import os as _os
    if _os.environ.get("PINEY_EEMU") != "py":
        try:
            import eemu_rs
            return eemu_rs.VuMachine
        except ImportError:
            import sys as _sys
            print("eemu_rs is not built (cargo build --release -p piney-eemu --features python, then copy "
                  "target/release/libpiney_eemu.so to tools/eemu_rs.so); using eemu.py", file=_sys.stderr)
    return _python_machine_class()


def _python_machine_class():
    import eemu
    import mips
    from eemu import Stop, f_add, f_div, f_from_int, f_mul, f_sqrt, f_sub, f_to_int

    class VuMachine(eemu.Machine):
        """eemu.Machine plus VU0 macro mode, lqc2/sqc2, ldl/ldr and some MMI."""

        MMI = {0x08, 0x09, 0x28, 0x29, 0x34, 0x36, 0x37, 0x3C, 0x3E, 0x3F}

        def __init__(self, program):
            super().__init__(program)
            self.vf = [[0, 0, 0, 0] for _ in range(32)]
            self.vf[0] = [0, 0, 0, ONE]
            self.vacc = [0, 0, 0, 0]
            self.q = 0
            self._names = {}

        def ea(self, w):
            return (eemu.sx(self.r[(w >> 21) & 31], 32) + eemu.sx(w & 0xFFFF, 16)) & M32

        def exec(self, pc, w):
            op = w >> 26
            if op == 0x12:
                return self.cop2(pc, w)
            if op == 0x36:                               # lqc2
                v = self.load(self.ea(w) & ~15, 16)
                self.vset((w >> 16) & 31, 0xF, words(v))
                return None
            if op == 0x3E:                               # sqc2
                self.store(self.ea(w) & ~15, 16, from_words(self.vf[(w >> 16) & 31]))
                return None
            if op in (0x1A, 0x1B):                       # ldl, ldr
                rt, ea = (w >> 16) & 31, self.ea(w)
                k = ea & 7
                dw = self.load(ea & ~7, 8)
                old = self.r[rt] & M64
                if op == 0x1A:
                    sh = 8 * (7 - k)
                    v = ((dw << sh) & M64) | (old & ((1 << sh) - 1))
                else:
                    sh = 8 * k
                    v = (dw >> sh) | (old & ((M64 << (64 - sh)) & M64 if sh else 0))
                self.set(rt, v)
                return None
            if op == 0x1C and (w & 63) in self.MMI:
                return self.mmi(pc, w)
            return super().exec(pc, w)

        def vset(self, i, mask, vals):
            if i:
                for k in range(4):
                    if mask & (8 >> k):
                        self.vf[i][k] = vals[k] & M32

        def cop2(self, pc, w):
            rs, rt, rd = (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31
            if rs == 0x01:                               # qmfc2
                self.set(rt, from_words(self.vf[rd]), bits=128)
                return None
            if rs == 0x05:                               # qmtc2
                self.vset(rd, 0xF, words(self.r[rt]))
                return None
            if not rs & 0x10:
                self.bad(pc, w)
            dest = (w >> 21) & 0xF
            ft, fs, fd = rt, rd, (w >> 6) & 31
            a, b = list(self.vf[fs]), list(self.vf[ft])
            bcv, qv = [b[w & 3]] * 4, [self.q] * 4

            def lanes(fn, x, y):
                return [fn(x[k], y[k]) for k in range(4)]

            def arith(kind, y):
                if kind == "add":
                    return lanes(f_add, a, y)
                if kind == "sub":
                    return lanes(f_sub, a, y)
                if kind == "mul":
                    return lanes(f_mul, a, y)
                if kind == "madd":
                    return lanes(f_add, self.vacc, lanes(f_mul, a, y))
                if kind == "msub":
                    return lanes(f_sub, self.vacc, lanes(f_mul, a, y))
                if kind == "max":
                    return lanes(lambda x, z: x if eemu.f_cmp(x, z) >= 0 else z, a, y)
                return lanes(lambda x, z: x if eemu.f_cmp(x, z) <= 0 else z, a, y)

            funct = w & 63
            if funct < 0x3C:
                if funct < 0x1C:
                    self.vset(fd, dest, arith(("add", "sub", "madd", "msub", "max", "mini", "mul")[funct >> 2], bcv))
                elif funct in (0x1C, 0x20, 0x21, 0x24, 0x25):
                    self.vset(fd, dest, arith({0x1C: "mul", 0x20: "add", 0x21: "madd", 0x24: "sub",
                                               0x25: "msub"}[funct], qv))
                elif funct in (0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2F):
                    self.vset(fd, dest, arith({0x28: "add", 0x29: "madd", 0x2A: "mul", 0x2B: "max",
                                               0x2C: "sub", 0x2D: "msub", 0x2F: "mini"}[funct], b))
                elif funct == 0x2E:                      # vopmsub
                    prod = [f_mul(a[1], b[2]), f_mul(a[2], b[0]), f_mul(a[0], b[1])]
                    self.vset(fd, dest & 0xE, [f_sub(self.vacc[k], prod[k]) for k in range(3)] + [0])
                else:
                    raise Stop(f"0x{pc:08x}: {mips.decode(w, pc).text} is not interpreted")
                return None
            code = ((w >> 6) & 31) << 2 | (w & 3)
            acc_ops = {0x1C: ("mul", qv), 0x20: ("add", qv), 0x21: ("madd", qv), 0x24: ("sub", qv),
                       0x25: ("msub", qv), 0x28: ("add", b), 0x29: ("madd", b), 0x2A: ("mul", b),
                       0x2C: ("sub", b), 0x2D: ("msub", b)}
            if code < 0x10 or 0x18 <= code <= 0x1B or code in acc_ops:
                if code in acc_ops:
                    kind, y = acc_ops[code]
                else:
                    kind, y = {0: "add", 1: "sub", 2: "madd", 3: "msub", 6: "mul"}[code >> 2], bcv
                res = arith(kind, y)
                for k in range(4):
                    if dest & (8 >> k):
                        self.vacc[k] = res[k]
            elif 0x10 <= code <= 0x13:                   # vitof0/4/12/15
                sh = (0, 4, 12, 15)[code & 3]
                res = [f_from_int(x) for x in a]
                self.vset(ft, dest, [f_div(x, f_from_int(1 << sh)) for x in res] if sh else res)
            elif 0x14 <= code <= 0x17:                   # vftoi0/4/12/15
                sh = (0, 4, 12, 15)[code & 3]
                self.vset(ft, dest, [f_to_int(f_mul(x, f_from_int(1 << sh)) if sh else x) for x in a])
            elif code == 0x1D:                           # vabs
                self.vset(ft, dest, [x & 0x7FFFFFFF for x in a])
            elif code == 0x2E:                           # vopmula
                self.vacc[:3] = [f_mul(a[1], b[2]), f_mul(a[2], b[0]), f_mul(a[0], b[1])]
            elif code == 0x30:                           # vmove
                self.vset(ft, dest, a)
            elif code == 0x31:                           # vmr32
                self.vset(ft, dest, [a[1], a[2], a[3], a[0]])
            elif code == 0x38:                           # vdiv
                self.q = f_div(a[(w >> 21) & 3], b[(w >> 23) & 3])
            elif code == 0x39:                           # vsqrt
                self.q = f_sqrt(b[(w >> 23) & 3])
            elif code == 0x3A:                           # vrsqrt
                self.q = eemu.f_rsqrt(a[(w >> 21) & 3], b[(w >> 23) & 3])
            elif code in (0x2F, 0x3B):                   # vnop, vwaitq
                pass
            else:
                raise Stop(f"0x{pc:08x}: {mips.decode(w, pc).text} is not interpreted")
            return None

        def mmi(self, pc, w):
            rs, rt, rd, sa = (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31, (w >> 6) & 31
            n = self._names.get(w)
            if n is None:
                n = self._names[w] = mips.decode(w, pc).mnemonic.split(".")[0]
            A, B = self.r[rs] & M128, self.r[rt] & M128
            hi, lo = self.hi | (self.hi1 << 64), self.lo | (self.lo1 << 64)
            if n == "pextlw":
                a, b = words(A), words(B)
                v = from_words([b[0], a[0], b[1], a[1]])
            elif n == "pextuw":
                a, b = words(A), words(B)
                v = from_words([b[2], a[2], b[3], a[3]])
            elif n == "pcpyld":
                v = (B & M64) | ((A & M64) << 64)
            elif n == "pcpyud":
                v = (A >> 64) | ((B >> 64) << 64)
            elif n == "pextlh":
                a, b = halves(A), halves(B)
                v = from_halves([x for i in range(4) for x in (b[i], a[i])])
            elif n == "psubh":
                a, b = halves(A), halves(B)
                v = from_halves([a[i] - b[i] for i in range(8)])
            elif n == "paddsh":
                a, b = halves(A), halves(B)
                v = from_halves([max(-0x8000, min(0x7FFF, s16(a[i]) + s16(b[i]))) for i in range(8)])
            elif n == "ppach":
                a, b = halves(A), halves(B)
                v = from_halves([b[0], b[2], b[4], b[6], a[0], a[2], a[4], a[6]])
            elif n == "psraw":
                v = from_words([s32(x) >> sa for x in words(B)])
            elif n in ("pmthi", "pmtlo"):
                hi, lo = (A, lo) if n == "pmthi" else (hi, A)
                self.hi, self.hi1, self.lo, self.lo1 = hi & M64, hi >> 64, lo & M64, lo >> 64
                return None
            elif n == "pmaddh":
                a, b = [s16(x) for x in halves(A)], [s16(x) for x in halves(B)]
                H, L = [s32(x) for x in words(hi)], [s32(x) for x in words(lo)]
                for k, (acc, j) in enumerate(((L, 0), (L, 1), (H, 0), (H, 1), (L, 2), (L, 3), (H, 2), (H, 3))):
                    acc[j] += a[k] * b[k]
                hi, lo = from_words(H), from_words(L)
                self.hi, self.hi1, self.lo, self.lo1 = hi & M64, hi >> 64, lo & M64, lo >> 64
                v = from_words([L[0], H[0], L[2], H[2]])
            else:
                raise Stop(f"0x{pc:08x}: {mips.decode(w, pc).text} is not interpreted")
            if rd:
                self.r[rd] = v & M128
            return None

    return VuMachine


class GameAnim:
    """The game's animation code over scratch structures."""

    def __init__(self, elf=ELF):
        import eemu
        from image import Program
        self.eemu = eemu
        self.prog = Program(elf)
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value      # noqa: E731
        m = self.m
        for name in ("ccMalloc__FUi", "__nw__FUi", "ccMemalign__FUiUi"):
            m.hooks[self.sym(name)] = self.malloc
        for name in ("ccFree__FPv", "__dl__FPv"):
            m.hooks[self.sym(name)] = lambda m, *a: 0
        m.hooks[self.sym("OpenData__14ccRingBufferThFPv")] = self.open_data
        m.hooks[self.sym("GetWork__16ccDrawPacketCtrlFi")] = self.malloc_packet
        m.store(self.sym("ccSys"), 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        self.heap = HEAP
        self.fn = {k: self.sym(v) for k, v in (
            ("conv", "ConvLCNum2ALCNum__10ccAnmChunkFP14ccStreamReaderRPUcP16ccAnmIndexCntTblRUiUs"),
            ("init", "InitAnmCtrlWork__5ccAnmFP9ccAnmCtrlP10ccAnmIndex"),
            ("reset", "ResetAnmCtrlWork__5ccAnmFP13ccAnmCtrlWork"),
            ("set", "SetAnmCtrlWork__5ccAnmFP13ccAnmCtrlWorki"),
            ("forward", "_AnimateForward__5ccAnmFUi"),
            ("notes", "NoteProcess__5ccAnmFv"),
            ("modify", "Modify__9ccMorpherFP16ccDrawModelParam"))}

    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        if self.heap > HEAP_END:
            raise self.eemu.Stop("heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def malloc_packet(self, m, this, n, *_):
        return self.malloc(m, n)

    def open_data(self, m, reader, adrs, *_):
        m.store(reader + 36, 4, adrs)
        m.store(reader + 40, 4, HEAP_END)
        return 0

    def zero(self, a, n):
        self.m.mem[a:a + n] = bytes(n)

    def load(self, data, off):
        """Compile the Anime chunk at off as MakeAnimeIndex does; returns
        [(object, ctrl address or 0)] in the table's order."""
        m = self.m
        self.heap = HEAP
        _, frames, nwords = struct.unpack_from("<3I", data, off + 8)
        body = data[off + 20:off + 20 + 4 * nwords]
        assert DATA_BUF + len(body) <= DATA_END
        m.mem[DATA_BUF:DATA_BUF + len(body)] = body
        self.zero(RB, 64)
        self.open_data(m, RB, DATA_BUF)
        self.zero(CHUNK, 48)
        out = self.malloc(m, 4 * nwords + 64)
        m.store(CHUNK + 16, 4, frames)
        m.store(CHUNK + 20, 4, nwords)
        m.store(CHUNK + 24, 4, out)
        m.store(CHUNK + 36, 4, 0x80000000)
        m.store(DATAPTR, 4, out)
        m.store(NUM, 4, 0)
        m.call(self.fn["conv"], (CHUNK, RB, DATAPTR, TBL, NUM, 0), limit=2_000_000_000)
        self.frames = frames
        return [((lc - 1) >> 2, ctrl) for lc, ctrl in
                (struct.unpack_from("<II", m.mem, TBL + 8 * i) for i in range(m.load(NUM, 4)))]

    def new_anm(self, speed=256):
        self.zero(ANM, 0x110)
        self.m.store(ANM + 156, 2, speed)

    def work(self, ctrl, index=None):
        """InitAnmCtrlWork for ctrl driving a fresh ccCoord; returns
        (work, coord). The work goes on ANM's list."""
        m = self.m
        coord = self.malloc(m, 0x100)
        if index is None:
            index = self.malloc(m, 16)
        m.store(index + 4, 4, coord)          # ccAnmIndex.subst: the object
        m.store(index + 8, 2, 0x100)
        m.call(self.fn["init"], (ANM, ctrl, index))
        return m.load(ANM + 260, 4), coord

    def reset(self, work):
        self.m.call(self.fn["reset"], (ANM, work))

    def step(self, work, dt):
        """SetAnmCtrlWork(work, dt); returns SetAnmCtrlWork's scratch:
        (position, rotation matrix, scale matrix, float) as bits."""
        m = self.m
        m.call(self.fn["set"], (ANM, work, dt), limit=200_000_000)
        return (list(struct.unpack_from("<3I", m.mem, SCRATCH)),
                list(struct.unpack_from("<16I", m.mem, SCRATCH + 144)),
                list(struct.unpack_from("<16I", m.mem, SCRATCH + 80)),
                struct.unpack_from("<I", m.mem, SCRATCH + 208)[0])

    def playback(self, table, names, speed):
        """Set up ANM for _AnimateForward as SetAnm leaves it: one ccAnmIndex
        per table entry, object controllers on fresh ccCoords, morphers and
        models as scratch objects. Returns {obj: coord}, {morpher obj:
        address}, {model address: obj}."""
        m = self.m
        self.new_anm(speed)
        self.zero(STREAM, 0x200)
        coords, morphers, models = {}, {}, {}
        for i, (obj, ctrl) in enumerate(table):
            index = IDX + 12 * i
            self.zero(index, 12)
            m.store(index + 10, 1, 1)
            if ctrl and m.load(ctrl, 2) == 0x0102:
                coords[obj] = self.work(ctrl, index)[1]
            elif names(obj).startswith("MPH_"):
                a = self.malloc(m, 0x94)
                m.store(index + 4, 4, a)
                morphers[obj] = a
            elif names(obj).startswith("MDL_"):
                a = self.malloc(m, 0x40)
                m.store(index + 4, 4, a)
                models[a] = obj
        m.store(ANM + 148, 4, CHUNK)
        m.store(ANM + 172, 4, IDX)
        m.store(ANM + 248, 4, STREAM)
        self.open_data(m, ANM + 176, m.load(CHUNK + 24, 4))
        return coords, morphers, models

    def forward(self, dt):
        m = self.m
        ended = m.call(self.fn["forward"], (ANM, dt), limit=500_000_000)
        return (m.load(ANM + 152, 4) << 8) | m.load(ANM + 158, 2), ended

    def modify(self, base, base_scale, targets):
        """ccMorpher::Modify over one mmat. base: [(x, y, z)]; targets:
        [(positions, vertexScale bits, weight bits)]."""
        m = self.m
        self.heap = HEAP
        pack = lambda ps: b"".join(struct.pack("<3h", *p) for p in ps) + bytes(8)   # noqa: E731
        mph = self.malloc(m, 0x94)
        m.store(mph + 16, 2, len(targets))
        for i, (pos, scale, weight) in enumerate(targets):
            model = self.malloc(m, 0x40)
            index = self.malloc(m, 0x40)
            chunk = self.malloc(m, 0x60)
            mmat = self.malloc(m, 0x40)
            verts = self.malloc(m, 6 * len(pos) + 8)
            m.mem[verts:verts + 6 * len(pos) + 8] = pack(pos)
            m.store(model, 4, index)
            m.store(index + 44, 4, chunk)
            m.store(chunk + 64, 4, scale)
            m.store(model + 8, 4, mmat)
            m.store(model + 12, 4, scale)
            m.store(model + 34, 2, 1)
            m.store(mmat + 4, 4, len(pos))
            m.store(mmat + 16, 4, verts)
            m.store(mph + 20 + 8 * i, 4, model)
            m.store(mph + 24 + 8 * i, 4, weight)
        src = self.malloc(m, 6 * len(base) + 8)
        m.mem[src:src + 6 * len(base) + 8] = pack(base)
        layer = self.malloc(m, 0x40)
        self.zero(MORPH_PARAM, 0x1A0)
        m.store(MORPH_PARAM + 348, 4, layer)
        m.store(MORPH_PARAM + 364, 4, base_scale)
        m.store(MORPH_PARAM + 384, 4, len(base))
        m.store(MORPH_PARAM + 388, 4, src)
        m.call(self.fn["modify"], (mph, MORPH_PARAM), limit=500_000_000)
        out = m.load(MORPH_PARAM + 388, 4)
        return [struct.unpack_from("<3h", m.mem, out + 6 * i) for i in range(len(base))]


# data ---------------------------------------------------------------------------

_members = None


def members():
    """{member name: inflated bytes} from DATA.BIN, loaded once."""
    global _members
    if _members is None:
        import gzarc
        spec = DATA if os.path.exists(DATA) else f"{ISO}::DATA/DATA.BIN"
        data = gzarc.open_bytes(spec)
        _members = {m.name: m for m in gzarc.members(data)}
        _members = {"_raw": data, **_members}
    return _members


def member(name):
    import ccs
    import gzarc
    ms = members()
    return ccs.Ccs(gzarc.inflate(ms["_raw"], ms[name]))


def animes(c):
    import anim
    import ccsmodel
    scene = ccsmodel.Scene(c)
    return scene, anim.animations(c, scene.ext)


def find(c, name):
    scene, anims = animes(c)
    for off, a in anims:
        if c.objects[a.object][0] == name:
            return scene, off, a
    raise KeyError(name)


# checks ---------------------------------------------------------------------------

def check_controllers(g, c, off, a, rng, times=6, max_tracks=12, counts=None):
    """Compare every (sampled) object track at random times; returns a list of
    failures and fills counts."""
    import anim
    from eemu import f_to_py
    counts = counts if counts is not None else collections.Counter()
    table = g.load(c.data, off)
    ctrls = {obj: ctrl for obj, ctrl in table if ctrl and g.m.load(ctrl, 2) == 0x0102}
    last = max(0, (a.frames - 1) * 256)
    tracks = a.tracks if len(a.tracks) <= max_tracks else rng.sample(a.tracks, max_tracks)
    bad = []
    g.new_anm()
    for tr in tracks:
        ctrl = ctrls.get(tr.object)
        if ctrl is None:
            bad.append(("no controller", tr.object))
            continue
        work, _ = g.work(ctrl)
        ts = sorted({1, max(1, last)} | {rng.randint(1, max(1, last)) for _ in range(times)})
        for t in ts:
            g.reset(work)
            pos, rot, scale, alpha = g.step(work, t)
            pose = tr.at(min(t, last))
            counts["cases"] += 1
            where = (c.objects[a.object][0], c.objects[tr.object][0], t)
            if pos != pose.pos:
                bad.append(("position",) + where)
            if [scale[0], scale[5], scale[10]] != pose.scale:
                bad.append(("scale",) + where)
            if alpha != pose.alpha:
                bad.append(("transparency",) + where)
            if isinstance(tr.rot, anim.RotConst):
                counts["single-value rotations"] += 1
                if [rot[4 * k + r] for k in range(3) for r in range(3)] != \
                        [tr.rot.bits[k][r] for k in range(3) for r in range(3)]:
                    bad.append(("rotation bits",) + where)
            elif isinstance(tr.rot, anim.RotKeyed):
                counts["keyed rotations"] += 1
                d = max(abs(f_to_py(rot[4 * k + r]) - pose.rot[r][k]) for r in range(3) for k in range(3))
                counts["worst keyed rotation 1e-9"] = max(counts["worst keyed rotation 1e-9"], int(d * 1e9))
                if d > ROT_TOLERANCE + ROT_PER_KEY * tr.rot.passed(min(t, last)):
                    bad.append(("rotation", d) + where)
        # several steps land where one does
        g.reset(work)
        prev = 0
        for t in ts:
            got = g.step(work, t - prev)
            prev = t
        g.reset(work)
        counts["multi-step cases"] += 1
        if got != g.step(work, ts[-1]):
            bad.append(("multi-step differs", c.objects[tr.object][0]))
    # material records: the offsets in SetAnmCtrlWork's scratch, and what it
    # writes into a ccMaterial through the index's ccAnmModifyMat list
    mats = {obj: ctrl for obj, ctrl in table if ctrl and g.m.load(ctrl, 2) == 0x0202}
    m = g.m
    for mat, _, _ in a.materials:
        ctrl = mats.get(mat)
        if ctrl is None:
            bad.append(("no material controller", mat))
            continue
        material, chunk, lst, arr, index = (g.malloc(m, 0x20) for _ in range(5))
        crop = (rng.randint(0, 0xFFFF), rng.randint(0, 0xFFFF))
        m.store(material + 12, 4, chunk)
        m.store(chunk + 20, 2, crop[0])
        m.store(chunk + 22, 2, crop[1])
        m.store(arr, 4, material)
        m.store(lst, 4, arr)
        m.store(lst + 4, 2, 1)
        m.store(index + 4, 4, lst)
        m.call(g.fn["init"], (ANM, ctrl, index))
        work = m.load(ANM + 260, 4)
        for t in sorted({1, max(1, last)} | {rng.randint(1, max(1, last)) for _ in range(times)}):
            g.reset(work)
            m.call(g.fn["set"], (ANM, work, t))
            counts["material cases"] += 1
            (_, u, v), = [x for x in a.uv(t) if x[0] == mat][:1]
            got = [m.load(SCRATCH, 4), m.load(SCRATCH + 4, 4), m.load(material + 20, 2), m.load(material + 22, 2)]
            if got != [u, v] + anim.material_offsets(u, v, *crop):
                bad.append(("material", c.objects[mat][0], t, got, [u, v] + anim.material_offsets(u, v, *crop)))
    return bad


def check_playback(g, c, scene, off, a, speed, steps, counts=None):
    """_AnimateForward `steps` times at `speed`; compares the frame counter,
    the return value, object matrices and morpher targets each step."""
    import anim
    from eemu import f_to_py
    counts = counts if counts is not None else collections.Counter()
    name = lambda o: c.objects[o][0] if o < len(c.objects) else ""       # noqa: E731
    table = g.load(c.data, off)
    coords, morphers, models = g.playback(table, name, speed)
    bad = []
    time = 0
    for _ in range(steps):
        got_time, got_end = g.forward(speed)
        step = a.forward(time, speed)
        time = step.time
        counts["steps"] += 1
        counts["ends"] += int(step.ended)
        counts["loops"] += int(a.loops and step.time == 0)
        if (got_time, got_end) != (step.time, int(step.ended)):
            bad.append(("time", time, got_time, got_end, step))
            break
        if step.pose_at is None:
            continue
        for tr, pose in a.poses(step.pose_at):
            coord = coords.get(tr.object)
            if coord is None:
                continue
            mat = struct.unpack_from("<16I", g.m.mem, coord + 64)
            counts["object matrices"] += 1
            if list(mat[12:15]) != pose.pos:
                bad.append(("matrix position", name(tr.object), step.pose_at))
            want = pose.matrix()
            d = max(abs(f_to_py(mat[4 * k + r]) - want[r][k]) for r in range(3) for k in range(3))
            scale = max(1.0, *(abs(f_to_py(s)) for s in pose.scale))
            keys = tr.rot.passed(step.pose_at) if isinstance(tr.rot, anim.RotKeyed) else 0
            if d > (ROT_TOLERANCE + ROT_PER_KEY * keys) * scale:
                bad.append(("matrix", name(tr.object), step.pose_at, d))
        want = a.morph_weights(step.pose_at >> 8)
        for mph, addr in morphers.items():
            n = g.m.load(addr + 16, 2)
            got = [(models[g.m.load(addr + 20 + 8 * i, 4)], g.m.load(addr + 24 + 8 * i, 4)) for i in range(n)]
            counts["morph states"] += bool(step.pose_at >> 8)
            if (step.pose_at >> 8) and got != [tuple(p) for p in want.get(mph, [])]:
                bad.append(("morph", name(mph), step.pose_at >> 8, got[:2], want.get(mph, [])[:2]))
    return bad


def note_steps(rng, a, n):
    """Random _AnimateForward steps for a note check: 0, one frame, odd
    values inside a frame, several frames at once, and some past the end."""
    last = max(1, (a.frames - 1) * 256)
    out = []
    for _ in range(n):
        k = rng.random()
        if k < 0.1:
            out.append(0)
        elif k < 0.4:
            out.append(256)
        elif k < 0.6:
            out.append(rng.randint(1, 600))
        elif k < 0.8:
            out.append(256 * rng.randint(2, 12) + rng.choice([0, 0, rng.randint(1, 255)]))
        elif k < 0.95:
            out.append(rng.randint(1, 0xFFFF))
        else:
            out.append(rng.randint(last // 2, 2 * last))
    return out


def check_notes(g, c, off, a, steps, counts=None, rng=None):
    """SetAnm's state, then _AnimateForward(step) and NoteProcess for each
    step, NoteProcess's callback recording every note it is handed; compares
    the frame counter, the return value and each note (event, param, time,
    object) in order with Animation.forward / passed_notes. With rng, a
    play-once animation that has ended is restarted half the time, the way
    _AnimateForward restarts a loop (ResetAnmCtrlWork, the reader rewound,
    frame 0; the note list left alone)."""
    counts = counts if counts is not None else collections.Counter()
    m = g.m
    name = lambda o: c.objects[o][0] if o < len(c.objects) else ""       # noqa: E731
    table = g.load(c.data, off)
    g.playback(table, name, 256)
    # every index entry an instance of its own, so a note's +0x10 names its
    # object; entries playback left empty get a scratch block
    inst = {}
    for i, (obj, _) in enumerate(table):
        sub = m.load(IDX + 12 * i + 4, 4)
        if not sub:
            sub = g.malloc(m, 0x100)
            m.store(IDX + 12 * i + 4, 4, sub)
        inst[sub] = obj
    got = []
    m.hooks[NOTE_FUNC] = lambda m, note, *_: got.append(
        (m.load(note + 4, 4), m.load(note + 8, 4), m.load(note + 12, 4),
         inst.get(m.load(note + 16, 4), -1))) or 0
    m.store(ANM + 164, 4, NOTE_FUNC)
    bad = []
    time = 0
    for i, step in enumerate(steps):
        got_time, got_end = g.forward(step)
        got.clear()
        m.call(g.fn["notes"], (ANM,))
        first = list(got)
        if i % 5 == 0:
            got.clear()
            m.call(g.fn["notes"], (ANM,))
            counts["repeated NoteProcess"] += 1
            if got != first:
                bad.append(("second NoteProcess differs", time, step, first, list(got)))
        want_f = a.forward(time, step)
        old = time >> 8
        want = [(e, p, old if f == old + 1 else f, obj) for f, obj, e, p in a.passed_notes(time, step)]
        _, pairs = a.forward_notes(time, step)
        counts["note steps"] += 1
        counts["notes"] += len(first)
        counts["steps with several notes"] += len(first) > 1
        counts["zero steps"] += step == 0
        counts["loops"] += int(a.loops and want_f.time == 0 and step > 0)
        counts["ends"] += int(want_f.ended)
        if (got_time, got_end) != (want_f.time, int(want_f.ended)):
            bad.append(("time", time, step, got_time, got_end, want_f))
            break
        if first != want or pairs != [(e, p) for e, p, _, _ in want]:
            bad.append(("notes", time, step, first, want))
        time = want_f.time
        if rng is not None and want_f.ended and rng.random() < 0.5:
            work = m.load(ANM + 260, 4)
            while work:
                m.call(g.fn["reset"], (ANM, work))
                work = m.load(work, 4)
            g.open_data(m, ANM + 176, m.load(CHUNK + 24, 4))
            m.store(ANM + 152, 4, 0)
            m.store(ANM + 158, 2, 0)
            counts["restarts"] += 1
            time = 0
    return bad


def notes_pool():
    """[(member, offset)] of every Anime chunk with F_Note records."""
    import ccs
    import gzarc
    ms = members()
    pool = []
    for name, mem in ms.items():
        if name == "_raw":
            continue
        c = ccs.Ccs(gzarc.inflate(ms["_raw"], mem))
        for off, t, _, _ in c.chunks():
            if t is None:
                break
            if t & 0xFFFF == 0x0700:
                p, end = off + 20, off + 20 + 4 * struct.unpack_from("<I", c.data, off + 16)[0]
                while p < end:
                    kind, words = struct.unpack_from("<II", c.data, p)
                    if kind & 0xFFFF == 0x0108:
                        pool.append((name, off))
                        break
                    p += 8 + 4 * words
    return pool


def raw_positions(model):
    """ccsmodel.Model positions back to stored s16s, per mmat."""
    unit = model.scale / 4096
    return [[tuple(int(round(v / unit)) for v in p) for p in mm.positions] for mm in model.mmats]


def morph_cases(c, scene, a, limit=3):
    """(base mmat, base scale, [(target mmat, target scale, weight)]) from
    F_Morpher records; the base model from the Morpher chunk (0x1900)."""
    import ccsmodel
    from anim import bits
    base_of = {}
    for off, t, n, end in c.chunks():
        if t is None:
            break
        if t & 0xFFFF == 0x1900:
            mph, base = struct.unpack_from("<II", c.data, off + 8)
            base_of[mph] = base
    models = {m.obj: m for m in ccsmodel.models(c)}
    out = []
    for mph, recs in a.morphs.items():
        base = models.get(base_of.get(mph))
        if base is None:
            continue
        for frame, pairs in recs[::max(1, len(recs) // limit)][:limit]:
            if not pairs or any(t not in models for t, _ in pairs):
                continue
            for k, mm in enumerate(raw_positions(base)):
                tgts = [(raw_positions(models[t])[k], bits(models[t].scale), w) for t, w in pairs]
                if all(len(p) == len(mm) for p, _, _ in tgts):
                    out.append((mph, frame, k, mm, bits(base.scale), tgts))
    return out


# unit tests ---------------------------------------------------------------------------

HAVE = os.path.exists(ELF) and (os.path.exists(DATA) or os.path.exists(ISO))
CONTROLLER_CASES = [
    ("town01.cmp", "ANM_sr1fla1_a"),        # a town's flags
    ("town01.cmp", "ANM_sr1shi1_a"),        # its ship
    ("xp_flag.cmp", "ANM_xpflnut0"),        # the square's flag, with morphing
    ("field_d.cmp", "ANM_sfd4lak1a"),
    ("chgate.cmp", "ANM_xmgtcir2"),         # keys sharing a frame
    ("eldl.cmp", "ANM_eldlmag0"),           # keys going back to frame 0
    ("cbu3body.cmp", "ANM_cbu3nut2"),       # both, on a character
    ("xdhhack.cmp", "ANM_xdhfram0"),        # keys past the last frame
    ("xdl_load.cmp", "ANM_xdl_lod0"),       # texture offsets
    ("xdl_load.cmp", "ANM_xdl_lod1"),
]

NOTE_CASES = [
    ("ctu1body.cmp", "ANM_ctu1atc0"),      # Kite's normal attacks
    ("ctu1body.cmp", "ANM_ctu1atc1"),
    ("ctu1body.cmp", "ANM_ctu1run0"),      # his run, a loop with footsteps
    ("cbu4body.cmp", "ANM_cbu4atc0"),      # Orca's attack
    ("cbu4body.cmp", "ANM_cbu4run0"),
    ("egn1.cmp", "ANM_egn1atc0"),          # the goblin's attacks
    ("egn1.cmp", "ANM_egn1atc1"),
    ("ecm1.cmp", "ANM_ecm1mag0"),          # a note at frame 0
    ("cbu0ski2.cmp", "ANM_cbu0ski2"),      # frames with three notes
]


@unittest.skipUnless(HAVE, "needs the Infection executable and DATA.BIN")
class AnimTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.g = GameAnim()

    def test_controllers(self):
        rng = random.Random(5)
        counts = collections.Counter()
        for fname, anm in CONTROLLER_CASES:
            c = member(fname)
            try:
                scene, off, a = find(c, anm)
            except KeyError:
                continue
            bad = check_controllers(self.g, c, off, a, rng, times=4, max_tracks=6, counts=counts)
            self.assertEqual(bad, [], f"{fname} {anm}")
        self.assertGreater(counts["cases"], 50)
        self.assertGreater(counts["material cases"], 10)
        self.assertGreater(counts["keyed rotations"], 10)
        self.assertGreater(counts["single-value rotations"], 10)

    def test_playback(self):
        # a loop with morphing (xp_flag, the flag in the towns' squares), a
        # play-once animation with notes, at several speeds
        counts = collections.Counter()
        for fname, anm, speed, steps in (("xp_flag.cmp", "ANM_xpflnut0", 256, 60),
                                         ("xp_flag.cmp", "ANM_xpflnut0", 700, 30),
                                         ("ekx1.cmp", "ANM_ekx1atc0", 100, 130)):
            c = member(fname)
            scene, off, a = find(c, anm)
            self.assertEqual(check_playback(self.g, c, scene, off, a, speed, steps, counts), [], f"{fname} {anm}")
        self.assertEqual(counts["steps"], 220)
        self.assertGreaterEqual(counts["loops"], 2)
        self.assertGreater(counts["ends"], 5)
        self.assertGreater(counts["object matrices"], 500)
        self.assertGreater(counts["morph states"], 100)

    def test_notes(self):
        # Kite's attacks and run (a loop), Orca's attack and run, the
        # goblin's attacks, a note at frame 0 (never handed on) and frames
        # with several notes
        rng = random.Random(7)
        counts = collections.Counter()
        for fname, anm in NOTE_CASES:
            c = member(fname)
            scene, off, a = find(c, anm)
            self.assertTrue(a.notes, anm)
            bad = check_notes(self.g, c, off, a, note_steps(rng, a, 120), counts, rng)
            self.assertEqual(bad, [], f"{fname} {anm}")
        self.assertEqual(counts["note steps"], 120 * len(NOTE_CASES))
        self.assertGreater(counts["notes"], 100)
        self.assertGreater(counts["steps with several notes"], 10)
        self.assertGreater(counts["loops"], 5)
        self.assertGreater(counts["ends"], 20)
        self.assertGreater(counts["restarts"], 20)

    def test_morph(self):
        import anim
        rng = random.Random(3)
        n = 0
        for fname, anm in (("xp_flag.cmp", "ANM_xpflnut0"), ("ekx1.cmp", "ANM_ekx1dwn0")):
            c = member(fname)
            scene, off, a = find(c, anm)
            for mph, frame, k, base, scale, tgts in morph_cases(c, scene, a):
                if len(base) > 120:
                    idx = sorted(rng.sample(range(len(base)), 120))
                    base = [base[i] for i in idx]
                    tgts = [([p[i] for i in idx], s, w) for p, s, w in tgts]
                self.assertEqual(self.g.modify(base, scale, tgts), anim.morph(base, scale, tgts))
                n += 1
        self.assertGreater(n, 2)

    def test_morph_synthetic(self):
        # rescaled targets, weights past 1, saturation, wrapping differences
        import anim
        from anim import bits
        rng = random.Random(9)
        base = [tuple(rng.randint(-32768, 32767) for _ in range(3)) for _ in range(40)]
        tgts = [([tuple(rng.randint(-32768, 32767) for _ in range(3)) for _ in base],
                 bits(rng.choice([1.0, 2.0, 0.37])), bits(rng.uniform(-3, 9))) for _ in range(3)]
        self.assertEqual(self.g.modify(base, bits(1.0), tgts), anim.morph(base, bits(1.0), tgts))


# bulk and fixture ---------------------------------------------------------------------------

def bulk(n, seed):
    import anim
    import ccs
    import ccsmodel
    import gzarc
    rng = random.Random(seed)
    g = GameAnim()
    ms = members()
    pool = []
    for name, m in ms.items():
        if name == "_raw":
            continue
        c = ccs.Ccs(gzarc.inflate(ms["_raw"], m))
        for off, t, _, _ in c.chunks():
            if t is None:
                break
            if t & 0xFFFF == 0x0700:
                body = c.data[off + 20:off + 20 + 4 * struct.unpack_from("<I", c.data, off + 16)[0]]
                if b"\x02\x01\xcc\xcc" in body:
                    pool.append((name, off))
    print(f"{len(pool)} animations with object records", flush=True)
    counts = collections.Counter()
    fails = 0
    for name, off in rng.sample(pool, min(n, len(pool))):
        c = member(name)
        scene = ccsmodel.Scene(c)
        a = anim.Animation(c, off, scene.ext)
        bad = check_controllers(g, c, off, a, rng, counts=counts)
        counts["animations"] += 1
        if bad:
            fails += 1
            print(name, c.objects[a.object][0], bad[:4])
        if a.morphs or a.loops:
            counts["playbacks"] += 1
            speed = rng.choice([256, 256, 100, 384, 700])
            bad = check_playback(g, c, scene, off, a, speed, min(400, 2 * a.frames * 256 // speed + 3), counts)
            if bad:
                fails += 1
                print(name, c.objects[a.object][0], "playback", bad[:4])
        print(f"{name} {c.objects[a.object][0]}: {dict(counts)}", flush=True)
    print(f"{fails} failing, {dict(counts)}")
    return 1 if fails else 0


def notes_bulk(n, seed):
    """check_notes over n random animations with F_Note records (all of
    them when n is None), random steps each."""
    import anim
    import ccsmodel
    rng = random.Random(seed)
    g = GameAnim()
    pool = notes_pool()
    print(f"{len(pool)} animations with F_Note records", flush=True)
    counts = collections.Counter()
    fails = 0
    picked = pool if n is None or n >= len(pool) else rng.sample(pool, n)
    for name, off in picked:
        c = member(name)
        scene = ccsmodel.Scene(c)
        a = anim.Animation(c, off, scene.ext)
        counts["animations"] += 1
        counts["looping animations"] += a.loops
        counts["records"] += len(a.notes)
        steps = note_steps(rng, a, max(30, min(200, 3 * a.frames)))
        bad = check_notes(g, c, off, a, steps, counts, rng)
        if bad:
            fails += 1
            print(name, c.objects[a.object][0], bad[:3], flush=True)
    print(f"{fails} failing, {dict(counts)}")
    return 1 if fails else 0


FIXTURE_CASES = CONTROLLER_CASES + [("cha2body.cmp", "ANM_cha2nut3"), ("ekx1.cmp", "ANM_ekx1atc0"),
                                     ("town01.cmp", "ANM_sr1sim1_a")]


def fixture():
    """Lines for crates/piney-data/tests/anim_fixture.txt: evaluated values
    only (the Rust test reads the disc for everything else)."""
    import anim
    rng = random.Random(11)
    print("# tools/test_anim.py fixture: anim.py's values, checked against the game by the same tool.")
    print("# pose MEMBER ANIME OBJECT TARGET TICKS pos.xyz scale.xyz alpha (float bits) rot 3x3 rows")
    print("# morph MEMBER ANIME MORPHER FRAME TARGET:WEIGHT...")
    print("# blend MEMBER ANIME MORPHER FRAME MMAT FNV1A-64 of the blended s16 positions")
    print("# forward MEMBER ANIME SPEED STEPS times ... (0x80000000 | time when the step ended)")
    print("# uv MEMBER ANIME MATERIAL TICKS u v (float bits) ccMaterial u v for crops 0x1234 0xfedc")
    print("# notes MEMBER ANIME TICKS STEP TICKS' ENDED EVENT:PARAM... (hex; what NoteProcess hands on, in order)")
    for fname, anm in FIXTURE_CASES:
        c = member(fname)
        scene, off, a = find(c, anm)
        last = max(0, (a.frames - 1) * 256)
        tracks = a.tracks if len(a.tracks) <= 6 else rng.sample(a.tracks, 6)
        for tr in tracks:
            for t in sorted({0, 1, last} | {rng.randint(0, last) for _ in range(3)}):
                p = tr.at(t)
                vals = " ".join(f"{v:08x}" for v in p.pos + p.scale + [p.alpha])
                rot = " ".join(f"{x:.9g}" for row in p.rot for x in row)
                print(f"pose {fname} {anm} {tr.object} {tr.target} {t} {vals} {rot}")
        for mat, _, _ in a.materials:
            for t in sorted({0, 1, last} | {rng.randint(0, last) for _ in range(4)}):
                (_, u, v), = [x for x in a.uv(t) if x[0] == mat][:1]
                offs = anim.material_offsets(u, v, 0x1234, 0xFEDC)
                print(f"uv {fname} {anm} {mat} {t} {u:08x} {v:08x} {offs[0]:04x} {offs[1]:04x}")
        for mph, recs in a.morphs.items():
            used = [f for f, pairs in recs if pairs]
            for frame in sorted({0, a.frames - 1} | set(rng.sample(used, min(4, len(used))))):
                w = a.morph_weights(frame).get(mph, [])
                print(f"morph {fname} {anm} {mph} {frame} " + " ".join(f"{t}:{v:08x}" for t, v in w))
        for mph, frame, k, base, scale, tgts in morph_cases(c, scene, a, limit=2)[:4]:
            h = 0xCBF29CE484222325
            for v in anim.morph(base, scale, tgts):
                for x in v:
                    for byte in struct.pack("<h", x):
                        h = ((h ^ byte) * 0x100000001B3) & M64
            print(f"blend {fname} {anm} {mph} {frame} {k} {h:016x}")
        for speed in (256, 700):
            time, out = 0, []
            for _ in range(min(100, 2 * a.frames * 256 // speed + 3)):
                st = a.forward(time, speed)
                time = st.time
                out.append(time | (0x80000000 if st.ended else 0))
            print(f"forward {fname} {anm} {speed} {len(out)} " + " ".join(f"{t:x}" for t in out))
    for fname, anm in NOTE_CASES:
        c = member(fname)
        scene, off, a = find(c, anm)
        time = 0
        for step in note_steps(rng, a, 40):
            st, notes = a.forward_notes(time, step)
            print(f"notes {fname} {anm} {time:x} {step:x} {st.time:x} {int(st.ended)} "
                  + " ".join(f"{e:x}:{p:x}" for e, p in notes))
            time = 0 if st.ended and rng.random() < 0.5 else st.time
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        sys.exit(bulk(int(sys.argv[2]) if len(sys.argv) > 2 else 20,
                      int(sys.argv[3]) if len(sys.argv) > 3 else 1))
    if len(sys.argv) > 1 and sys.argv[1] == "notes":
        sys.exit(notes_bulk(int(sys.argv[2]) if len(sys.argv) > 2 else None,
                            int(sys.argv[3]) if len(sys.argv) > 3 else 1))
    if len(sys.argv) > 1 and sys.argv[1] == "fixture":
        sys.exit(fixture())
    unittest.main()
