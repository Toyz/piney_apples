#!/usr/bin/env python3
"""Anime chunks (0x0700) evaluated at any time, the way ccAnm plays them.

    tools/anim.py list  FILE                  every Anime chunk: frames, end, tracks
    tools/anim.py pose  FILE ANM_x FRAME      each object track's local transform
    tools/anim.py morph FILE ANM_x FRAME      the F_Morpher weights in force

FILE is an inflated .ccs, or ARCHIVE::MEMBER (tools/ccs.py load). FRAME may
be fractional. tools/test_anim.py checks all of this against the game's own
code run in tools/eemu.py.

The chunk is u32 object, u32 frame count N, u32 data words, then sub-chunks
laid out like the frame section's: Top 0, the controller records, Tops 1 ..
N-1 each followed by that frame's records, and an end Top: -1 plays once,
-2 loops. ccAnmChunk::ConvLCNum2ALCNum (0x00144e90) compiles the controller
records once, at load; they all sit under Top 0:

  0x0102 object   u32 object, u32 flags, then controllers picked by flag bits
                  0-2 position, 3-5 rotation (degrees), 6-8 scale, 9-11 a
                  float (the object's local transparency, ccCoord+0x88)
  0x0202 material u32 material, u32 flags, float controllers U (bits 0-2) and
                  V (3-5): texture offsets (ConvALCNum_FsetMatCtrl 0x00144d00)
  0x0603 0x0605 0x0607 0x0609  light controllers (not evaluated here)

Per-frame records follow Tops: 0x1901 F_Morpher, 0x0108 F_Note, 0x0601
F_Ambient, 0x0502 F_Camera and, in four files, 0x0101 F_Obj; F_Morpher
and F_Note are evaluated here.

A controller kind is 0 absent, 1 one value, 2 keyed: u32 n, then n x
(u32 frame, value); no other kind occurs. SetCtrl turns keys into a start
value plus segments of (duration, delta) in 1/256 frame units - the first
key's value, a hold segment up to the first key when it is not at frame 0,
one segment per later key, and a final hold to the end - and Set
(ccAnmCtrlFVec3_Set 0x00146890, ccAnmCtrlFloat_Set 0x00146530) walks them:
while a segment's duration is below the remaining time, add its delta and
step on; then value = acc + delta * (t / duration). All of it in EE single
precision (eemu's f_* functions): the deltas are key minus running sum, so
the sum at each key is what the runtime reaches. Durations are u32
differences, so keys that go back in time (a few hundred controllers)
make a segment of about 2^32 that is never passed, and keys sharing a
frame a segment of 0 that is always passed.

Rotations are not interpolated as angles. ccAnmCtrlRot_SetCtrl (0x001470c0)
quantises each key's degrees to 1/65536 turn (s16(int(deg * 182.044)), so
they wrap), builds the matrix Rx * Ry * Rz (sceVu0RotMatrix, whose sine and
cosine come from _sceVu0ecossin: a polynomial for cos and sqrt(1 - cos^2)
for sin, coarse to ~5e-4 near 0), and stores for each key the rotation from
the previous key's matrix to its own, R_i * R_{i-1}^T, as an axis and angle
(m2a 0x00105ec0: the axis from EigenVector, the angle a double atan2, so at
most half a turn). ccAnmCtrlRot_Set (0x00146f30) keeps a matrix W, starting
at the first key's, multiplies each passed segment's turn (a2m 0x00105f80)
on the left, and returns a2m(axis, angle * t / duration) * W: a
constant-speed turn about one axis between keys, the short way round. A
single-value rotation is sceVu0RotMatrix of degrees * pi / 180, not
quantised. The final hold segment of a rotation is the last key's delta
over a duration of (last - N) * 256 wrapped to u32, so it creeps by
angle * t / 2^32 - kept. All of it is reproduced here in the game's own
arithmetic: the EE's and VU0's float operations, newlib's sinf and cosf,
and the double atan / atan2, whose soft float steps round as Python's
floats do. docs/engine/animation.md has the steps.

The object's local matrix is T(position) * R * S(scale) (SetAnmCtrlWork
0x00150670: sceVu0MulMatrix(R, S) then sceVu0TransMatrix). Absent
controllers give position 0, identity, scale 1, transparency 1. A record
drives its object, or through ExtObj chunks the object they stand for
(ccGetExternalIndex 0x00101a50); several records can drive one object, each
its own instance of it.

Time is u32 in 1/256 frames (ccAnm frameNow * 256 + frameCnt; frameSpd,
the step per call, defaults to 256 = one frame). _AnimateForward (0x00152270)
clamps it to (N - 1) * 256; on reaching the last frame a -2 chunk resets to
frame 0, so a loop is N - 1 frames long and shows frame N - 1 in place of
frame 0 from the second pass on (Animation.forward). The game never
evaluates time 0 (a zero step skips SetAnmCtrlWork); here time 0 is the
first keys' values.

F_Morpher (0x1901) records after Top f set that morpher's targets and
weights (u32 morpher, u16 count, pad, count x (u32 target model, f32
weight)) from frame f on; nothing is interpolated between frames. The
game's reader starts at frame 1's records, so frame 0's are never applied.
ccMorpher::Modify (0x0013af10) blends in integers: each weight becomes
int(w * 4096) as an s16 (see morph_lanes for weights outside [0, 8)), and
every position component is base + ((sum of s16(target - base) * w) >> 12),
the sum in 32 bits, the add saturating to s16. A target stored at a
different vertexScale is first rescaled to the base's:
int(t * (target scale / base scale)). Normals are not morphed.

F_Note (0x0108) records are u32 object, u32 event, u32 param after their
frame's Top. ConvLCNum2ALCNum drops frame 0's (and Top 1): the frame data
the anm reads starts with frame 1's records. _AnimateForward (0x00152270)
first frees the anm's note list (ccAnm.noteRoot +0xa0, ccAnmNote::DelAll
0x00147ff0); only when the step changes frameNow does DecodeFrameChunk
(0x0014e1b0) read on, from where the last read stopped (just past Top
old + 1) to the first Top past the new frame, so the records of frames
old + 1 .. new, including the last frame's before the end Top. Each
F_Note makes a ccAnmNote (DecodeF_Note 0x0014f830: +4 event, +8 param,
+0xc time, +0x10 the object's instance, +0x14 its ccstag) pushed on the
list's head, and ccAnm::NoteProcess (0x00152210) calls funcNoteProcess
(+0xa4) on each from the head: so the notes go out in reverse file order,
the last frame's first and a frame's last record first (passed_notes,
forward_notes). A zero step, a step inside a frame, and steps held at a
play-once animation's last frame leave the list empty; SetAnm and
NoteProcess leave it alone. The note's time is the frame before the
first frame read (old) for that frame's notes, else the note's frame.

Material records give ccMaterial.u/v = s16(int(offset * 4096)) - the
Material chunk's cropU/cropV (material_offsets); how the draw uses them is
not traced here.
"""


import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from eemu import f_add, f_cmp, f_div, f_from_int, f_mul, f_sqrt, f_sub, f_to_int, f_to_py  # noqa: E402

M32 = 0xFFFFFFFF
ONE = 0x3F800000
PI = 0x40490FDB             # pi as the game's constant
DEG180 = 0x43340000
TO_S16 = 0x43360B61         # 182.04444 = 32768 / 180
FROM_S16 = 0x38C90FDB       # pi / 32768
EIGEN_MIN = 0x2D2FEBFF      # EigenVector gives up below ~1e-11

OBJECT, MATERIAL, MORPHER, NOTE, TOP = 0x0102, 0x0202, 0x1901, 0x0108, 0xFF01
END_ONCE, END_LOOP = 0xFFFFFFFF, 0xFFFFFFFE


def bits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def u2f(x):
    """A u32 to float as the compiled code does it: cvt.s.w, halving and
    doubling when the top bit is set."""
    if x < 0x80000000:
        return f_from_int(x)
    h = f_from_int((x >> 1) | (x & 1))
    return f_add(h, h)


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


# controllers ------------------------------------------------------------------

class Keyed:
    """A keyed float or float[3] controller, compiled as
    ccAnmCtrlFVec3_SetCtrl / ccAnmCtrlFloat_SetCtrl do."""

    def __init__(self, keys, frames):
        width = len(keys[0][1])
        f0, v0 = keys[0]
        self.base = list(v0)
        t = (f0 << 8) & M32
        segs = []
        if t:
            segs.append((t, [0] * width))
        prev = list(v0)
        for frame, v in keys[1:]:
            dur = (((frame << 8) & M32) - t) & M32
            t = (t + dur) & M32
            delta = [f_sub(v[k], prev[k]) for k in range(width)]
            prev = [f_add(prev[k], delta[k]) for k in range(width)]
            segs.append((dur, delta))
        segs.append(((((frames << 8) & M32) - t) & M32, [0] * width))
        self.segs = segs

    def at(self, time):
        acc = list(self.base)
        t = time
        i = 0
        while i < len(self.segs) - 1 and self.segs[i][0] < t:
            dur, delta = self.segs[i]
            acc = [f_add(a, d) for a, d in zip(acc, delta)]
            t = (t - dur) & M32
            i += 1
        dur, delta = self.segs[i]
        # 0/0 only at time 0 before a zero-length first segment, which the
        # game never evaluates
        frac = f_div(u2f(t), u2f(dur)) if dur else 0
        return [f_add(a, f_mul(d, frac)) for a, d in zip(acc, delta)]


HALF_PI = 0x3FC90FDB
SIN = (0x362E9C14, 0xB94FB21F, 0x3C08873E, 0xBE2AAAA4)   # S5432 (0x002f7520)


def vu_cossin(angle):
    """(sin, cos) of an angle, as sceVu0RotMatrixX/Y/Z get them from
    _sceVu0ecossin (0x001109b8): cos is an odd polynomial in pi/2 - |angle|,
    sin is +-sqrt(1 - cos^2) - coarse near 0 and wrong outside +-pi."""
    neg = bool(angle & 0x80000000) and angle & 0x7F800000 != 0
    x = f_add(HALF_PI, angle) if neg else f_sub(HALF_PI, angle)
    x2 = f_mul(x, x)
    p = [f_mul(f_mul(k, x), x2) for k in SIN]
    p[0], p[1], p[2] = f_mul(p[0], x2), f_mul(p[1], x2), f_mul(p[2], x2)
    r = f_add(x, p[3])
    p[0], p[1] = f_mul(p[0], x2), f_mul(p[1], x2)
    r = f_add(r, p[2])
    p[0] = f_mul(p[0], x2)
    r = f_add(f_add(r, p[1]), p[0])
    q = f_sqrt(f_sub(ONE, f_mul(r, r)))
    return (f_sub(0, q) if neg else f_add(0, q)), r


def vu_mul(cols, m):
    """sceVu0MulMatrix-style product of the four columns `cols` (the
    rotation) with each stored column of m: ((a*x + b*y) + c*z) + d*w."""
    out = []
    for col in m:
        acc = [f_mul(cols[0][k], col[0]) for k in range(4)]
        acc = [f_add(acc[k], f_mul(cols[1][k], col[1])) for k in range(4)]
        acc = [f_add(acc[k], f_mul(cols[2][k], col[2])) for k in range(4)]
        out.append([f_add(acc[k], f_mul(cols[3][k], col[3])) for k in range(4)])
    return out


def rot_bits(rad):
    """sceVu0RotMatrix(unit, rad) as stored columns of float bits: Rz, then
    Ry, then Rx multiplied on the left, so the matrix is Rx * Ry * Rz."""
    x, y, z = rad
    m = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]
    s, c = vu_cossin(z)
    m = vu_mul([[f_add(0, c), f_add(0, s), 0, 0], [f_sub(0, s), f_add(0, c), 0, 0],
                [0, 0, ONE, 0], [0, 0, 0, ONE]], m)
    s, c = vu_cossin(y)
    m = vu_mul([[f_add(0, c), 0, f_sub(0, s), 0], [0, ONE, 0, 0],
                [f_add(0, s), 0, f_add(0, c), 0], [0, 0, 0, ONE]], m)
    s, c = vu_cossin(x)
    m = vu_mul([[ONE, 0, 0, 0], [0, f_add(0, c), f_add(0, s), 0],
                [0, f_sub(0, s), f_add(0, c), 0], [0, 0, 0, ONE]], m)
    return m


def transpose4(m):
    """sceVu0TransposeMatrix: the words moved."""
    return [[m[c][r] for c in range(4)] for r in range(4)]


def vu_scale(v, s):
    """sceVu0ScaleVector: every lane times s."""
    return [f_mul(x, s) for x in v]


def vu_cross(a, b):
    """sceVu0OuterProduct: vopmula then vopmsub, each product rounded; w 0."""
    return [f_sub(f_mul(a[1], b[2]), f_mul(b[1], a[2])), f_sub(f_mul(a[2], b[0]), f_mul(b[2], a[0])),
            f_sub(f_mul(a[0], b[1]), f_mul(b[0], a[1])), 0]


def vu_normalize(a):
    """sceVu0Normalize: x, y, z times 1 / sqrt(x x + y y + z z); w 0."""
    q = f_sqrt(f_add(f_add(f_mul(a[0], a[0]), f_mul(a[1], a[1])), f_mul(a[2], a[2])))
    r = f_div(ONE, f_add(0, q))
    return [f_mul(a[0], r), f_mul(a[1], r), f_mul(a[2], r), 0]


# The game's libm as the rotation code calls it: newlib's float sinf and
# cosf in the EE's arithmetic, and the double atan / atan2, whose soft
# float steps (dpadd, dpmul, ...) round as IEEE doubles (Python floats) do.
# crates/piney-data/src/libm.rs has the same, with the addresses.

def _kernel_sinf(x, y, iy):
    s1, s2, s3, s4, s5, s6 = 0xBE2AAAAB, 0x3C088889, 0xB9500D01, 0x3638EF1B, 0xB2D72F34, 0x2F2EC9D3
    if x & 0x7FFFFFFF < 0x32000000 and f_to_int(x) == 0:
        return x
    z = f_mul(x, x)
    v = f_mul(z, x)
    r = f_add(f_mul(z, s6), s5)
    for c in (s4, s3, s2):
        r = f_add(f_mul(z, r), c)
    if not iy:
        return f_add(x, f_mul(v, f_add(f_mul(z, r), s1)))
    t = f_sub(f_mul(z, f_sub(f_mul(y, 0x3F000000), f_mul(v, r))), y)
    return f_sub(x, f_sub(t, f_mul(v, s1)))


def _kernel_cosf(x, y):
    cs = (0x3D2AAAAB, 0xBAB60B61, 0x37D00D01, 0xB493F27C, 0x310F74F6, 0xAD47D74E)
    ix = x & 0x7FFFFFFF
    if ix < 0x32000000 and f_to_int(x) == 0:
        return ONE
    z = f_mul(x, x)
    r = f_add(f_mul(z, cs[5]), cs[4])
    for c in (cs[3], cs[2], cs[1], cs[0]):
        r = f_add(f_mul(z, r), c)
    r = f_mul(z, r)
    if ix < 0x3E99999A:
        return f_sub(ONE, f_sub(f_mul(z, 0x3F000000), f_sub(f_mul(z, r), f_mul(x, y))))
    qx = 0x3E900000 if ix > 0x3F480000 else ix - 0x01000000
    hz = f_sub(f_mul(z, 0x3F000000), qx)
    return f_sub(f_sub(ONE, qx), f_sub(hz, f_sub(f_mul(z, r), f_mul(x, y))))


def _rem_pio2f(x):
    """(n, head, tail) for |x| up to the medium range (2^7 pi/2); a2m's
    angles stay within pi."""
    pio2_1, pio2_1t, pio2_2, pio2_2t = 0x3FC90F80, 0x37354443, 0x37354400, 0x2E85A308
    ix = x & 0x7FFFFFFF
    neg = bool(x & 0x80000000)
    if ix <= 0x3F490FD8:
        return 0, x, 0
    if ix < 0x4016CBE4:
        op = f_add if neg else f_sub
        z = op(x, pio2_1)
        hi, lo = (pio2_1, pio2_1t) if ix & 0xFFFFFFF0 != 0x3FC90FD0 else (pio2_2, pio2_2t)
        if hi == pio2_2:
            z = op(z, pio2_2)
        y0 = op(z, lo)
        return (-1 if neg else 1), y0, op(f_sub(z, y0), lo)
    if ix > 0x43490F80:
        raise ValueError(f"rem_pio2f: {x:08x} past the range a2m reaches")
    pio2_3, pio2_3t = 0x2E85A300, 0x248D3132
    n = f_to_int(f_add(f_mul(ix, 0x3F22F984), 0x3F000000))
    fn = f_from_int(n)
    r = f_sub(ix, f_mul(fn, pio2_1))
    w = f_mul(fn, pio2_1t)
    y0 = f_sub(r, w)
    if not (n < 32 and ix & 0xFFFFFF00 != NPIO2_HW[n - 1]):
        j = ix >> 23
        if j - (y0 >> 23 & 0xFF) > 8:
            t = r
            w = f_mul(fn, pio2_2)
            r = f_sub(t, w)
            w = f_sub(f_mul(fn, pio2_2t), f_sub(f_sub(t, r), w))
            y0 = f_sub(r, w)
            if j - (y0 >> 23 & 0xFF) > 25:
                t = r
                w = f_mul(fn, pio2_3)
                r = f_sub(t, w)
                w = f_sub(f_mul(fn, pio2_3t), f_sub(f_sub(t, r), w))
                y0 = f_sub(r, w)
    y1 = f_sub(f_sub(r, y0), w)
    return (-n, y0 ^ 0x80000000, y1 ^ 0x80000000) if neg else (n, y0, y1)


# npio2_hw (0x00349a38): the high bits of n * pi/2, n = 1..32.
NPIO2_HW = [0x3FC90F00, 0x40490F00, 0x4096CB00, 0x40C90F00, 0x40FB5300, 0x4116CB00, 0x412FED00, 0x41490F00,
            0x41623100, 0x417B5300, 0x418A3A00, 0x4196CB00, 0x41A35C00, 0x41AFED00, 0x41BC7E00, 0x41C90F00,
            0x41D5A000, 0x41E23100, 0x41EEC200, 0x41FB5300, 0x4203F200, 0x420A3A00, 0x42108300, 0x4216CB00,
            0x421D1400, 0x42235C00, 0x4229A500, 0x422FED00, 0x42363600, 0x423C7E00, 0x4242C700, 0x42490F00]


def sinf(x):
    if x & 0x7FFFFFFF <= 0x3F490FD8:
        return _kernel_sinf(x, 0, False)
    n, y0, y1 = _rem_pio2f(x)
    return [lambda: _kernel_sinf(y0, y1, True), lambda: _kernel_cosf(y0, y1),
            lambda: _kernel_sinf(y0, y1, True) ^ 0x80000000, lambda: _kernel_cosf(y0, y1) ^ 0x80000000][n & 3]()


def cosf(x):
    if x & 0x7FFFFFFF <= 0x3F490FD8:
        return _kernel_cosf(x, 0)
    n, y0, y1 = _rem_pio2f(x)
    return [lambda: _kernel_cosf(y0, y1), lambda: _kernel_sinf(y0, y1, True) ^ 0x80000000,
            lambda: _kernel_cosf(y0, y1) ^ 0x80000000, lambda: _kernel_sinf(y0, y1, True)][n & 3]()


def _d(bits_):
    return struct.unpack("<d", struct.pack("<Q", bits_))[0]


def _hi(x):
    b = struct.unpack("<Q", struct.pack("<d", x))[0]
    return b >> 32, b & M32


ATANHI_D = [_d(b) for b in (0x3FDDAC670561BB4F, 0x3FE921FB54442D18, 0x3FEF730BD281F69B, 0x3FF921FB54442D18)]
ATANLO_D = [_d(b) for b in (0x3C7A2B7F222F65E2, 0x3C81A62633145C07, 0x3C7007887AF0CBBD, 0x3C91A62633145C07)]
AT_D = [_d(b) for b in (0x3FD555555555550D, 0xBFC999999998EBC4, 0x3FC24924920083FF, 0xBFBC71C6FE231671,
                        0x3FB745CDC54C206E, 0xBFB3B0F2AF749A6D, 0x3FB10D66A0D03D51, 0xBFADDE2D52DEFD9A,
                        0x3FA97B4B24760DEB, 0xBFA2B4442C6A6C2F, 0x3F90AD3AE322DA11)]


def atan(x):
    """The double atan (main 0x001269d0), finite arguments."""
    hx, _ = _hi(x)
    ix = hx & 0x7FFFFFFF
    neg = hx & 0x80000000
    if ix >= 0x44100000:
        return (0.0 - ATANHI_D[3]) - ATANLO_D[3] if neg else ATANHI_D[3] + ATANLO_D[3]
    if ix < 0x3FDC0000:
        if ix < 0x3E200000 and 1e300 + x > 1.0:
            return x
        i = -1
    else:
        x = abs(x)
        if ix < 0x3FF30000:
            i, x = (0, (x + x - 1.0) / (x + 2.0)) if ix < 0x3FE60000 else (1, (x - 1.0) / (x + 1.0))
        elif ix < 0x40038000:
            i, x = 2, (x - 1.5) / (x * 1.5 + 1.0)
        else:
            i, x = 3, -1.0 / x
    z = x * x
    w = z * z
    a = AT_D
    s1 = z * (a[0] + w * (a[2] + w * (a[4] + w * (a[6] + w * (a[8] + w * a[10])))))
    s2 = w * (a[1] + w * (a[3] + w * (a[5] + w * (a[7] + w * a[9]))))
    if i < 0:
        return x - x * (s1 + s2)
    z = ATANHI_D[i] - ((x * (s1 + s2) - ATANLO_D[i]) - x)
    return 0.0 - z if neg else z


def atan2(y, x):
    """__ieee754_atan2 (0x00122b00), double, for the finite, not both
    zero arguments m2a gives it."""
    pi, pi_lo, pi_o_2 = _d(0x400921FB54442D18), _d(0x3CA1A62633145C07), _d(0x3FF921FB54442D18)
    hx, lx = _hi(x)
    hy, ly = _hi(y)
    ix, iy = hx & 0x7FFFFFFF, hy & 0x7FFFFFFF
    if hx == 0x3FF00000 and lx == 0:
        return atan(y)
    m = (hy >> 31) | ((hx >> 30) & 2)
    if iy | ly == 0:
        return y if m < 2 else (pi if m == 2 else -pi)
    if ix | lx == 0:
        return -pi_o_2 if hy >> 31 else pi_o_2
    k = (iy - ix) >> 20
    if k > 60:
        z = pi_o_2
    elif hx >> 31 and k < -60:
        z = 0.0
    else:
        z = atan(abs(y / x))
    return [z, -z, pi - (z - pi_lo), (z - pi_lo) - pi][m]


def _wide(v):
    return struct.unpack("<f", struct.pack("<I", v))[0]


def _narrow(x):
    """dptofp: to the nearest float."""
    return struct.unpack("<I", struct.pack("<f", x))[0]


def eigen_vector(m):
    """EigenVector (main 0x00105be0) on the stored rows of m - I: the row i
    whose cofactor (i+1, j+1)(i+2, j+2) - (i+1, j+2)(i+2, j+1) is largest in
    magnitude (mula.s / msub.s, fabs through doubles: the sign cleared),
    the next row w (w lane 1) and the normalised cross product of the two
    rows after it; None where the largest is below EIGEN_MIN."""
    a = [list(r) for r in m]
    for i in range(3):
        a[i][i] = f_sub(a[i][i], ONE)
    best, row = 0xBF800000, 0
    for i in range(3):
        for j in range(3):
            r1, r2, c1, c2 = (i + 1) % 3, (i + 2) % 3, (j + 1) % 3, (j + 2) % 3
            cof = f_sub(f_mul(a[r1][c1], a[r2][c2]), f_mul(a[r1][c2], a[r2][c1])) & 0x7FFFFFFF
            if f_cmp(cof, best) > 0:
                best, row = cof, i
    w = a[(row + 1) % 3][:3] + [ONE]
    v = a[(row + 2) % 3][:3] + [ONE]
    axis = vu_normalize(vu_cross(w, v))
    return None if f_cmp(best, EIGEN_MIN) < 0 else (axis, w)


def eigen_frame(axis, w):
    """EigenVec2Mat (0x00105e00): stored columns the axis, w and their cross
    product, each normalised again."""
    a, b = vu_normalize(axis), vu_normalize(w)
    c = vu_normalize(vu_cross(a, b))
    return [[a[r], b[r], c[r], 0] for r in range(3)] + [[0, 0, 0, ONE]]


def m2a(m):
    """m2a (0x00105ec0): (axis x, y, z, angle), the angle the double atan2 of
    F * (m * F^T)'s stored row 1, z over y; all zeros where EigenVector
    gives up."""
    found = eigen_vector(m)
    if found is None:
        return [0, 0, 0, 0]
    axis, w = found
    f = eigen_frame(axis, w)
    local = vu_mul(f, vu_mul(m, transpose4(f)))
    return axis[:3] + [_narrow(atan2(_wide(local[1][2]), _wide(local[1][1])))]


def a2m(aa):
    """a2m (0x00105f80): c I + s [u]x + (1 - c) u u^T from sinf / cosf, each
    lane as the code forms it (stored rows, row 3 and lane w the unit
    matrix's)."""
    s = vu_scale(aa, sinf(aa[3]))
    c = cosf(aa[3])
    t = f_sub(ONE, c)
    out = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]
    for r in range(3):
        outer = vu_scale(vu_scale(aa, aa[r]), t)
        for k in range(3):
            d = (r + 3 - k) % 3
            turn = c if d == 0 else (s[3 - r - k] ^ 0x80000000 if d == 1 else s[3 - r - k])
            out[r][k] = f_add(turn, outer[k])
    return out


def key_radians(deg):
    """A key's degrees (float bits) quantised to s16 turns, in radians (bits)."""
    return [f_mul(FROM_S16, f_from_int(s16(f_to_int(f_mul(TO_S16, d))))) for d in deg]


def const_radians(deg):
    return [f_div(f_mul(PI, d), DEG180) for d in deg]


class RotKeyed:
    """ccAnmCtrlRot_SetCtrl's segments (duration, turn as m2a stores it, and
    the running matrix ccAnmCtrlRot_Set holds on reaching it) and _Set's
    walk; matrices as stored rows of float bits."""

    def __init__(self, keys, frames):
        mats = [rot_bits(key_radians(v)) for _, v in keys]
        t = (keys[0][0] << 8) & M32
        turns = []
        if t:
            turns.append((t, [0, 0, 0, 0]))
        last = [0, 0, 0, 0]
        for i in range(1, len(keys)):
            t_new = (keys[i][0] << 8) & M32
            last = m2a(vu_mul(mats[i], transpose4(mats[i - 1])))
            turns.append(((t_new - t) & M32, last))
            t = t_new
        end = (frames << 8) & M32
        if t < end:
            turns.append(((t - end) & M32, last))
        self.segs = []
        w = mats[0]
        for dur, turn in turns:
            self.segs.append((dur, turn, w))
            w = vu_mul(a2m(turn), w)
        self.base = mats[0]

    def passed(self, time):
        """How many segments the walk to `time` multiplies in."""
        t, i = time, 0
        while i < len(self.segs) - 1 and self.segs[i][0] < t:
            t = (t - self.segs[i][0]) & M32
            i += 1
        return i

    def bits(self, time):
        """The matrix at `time`, stored rows of float bits."""
        t, i = time, 0
        while i < len(self.segs) - 1 and self.segs[i][0] < t:
            t = (t - self.segs[i][0]) & M32
            i += 1
        if not self.segs:
            return self.base
        dur, turn, start = self.segs[i]
        # 0/0 only at time 0 before a zero-length first segment, which the
        # game never evaluates
        frac = f_div(u2f(t), u2f(dur)) if dur else 0
        return vu_mul(a2m(turn[:3] + [f_mul(turn[3], frac)]), start)

    def at(self, time):
        m = self.bits(time)
        return [[f_to_py(m[c][r]) for c in range(3)] for r in range(3)]


class Const:
    def __init__(self, value):
        self.value = value

    def at(self, time):
        return self.value


class RotConst:
    """A single-value rotation: sceVu0RotMatrix of the degrees in radians."""

    def __init__(self, deg):
        self.matrix = rot_bits(const_radians(deg))

    def bits(self, time):
        return self.matrix

    def at(self, time):
        return [[f_to_py(self.matrix[c][r]) for c in range(3)] for r in range(3)]


def read_ctrl(d, q, kind, width, frames, rot=False):
    """(controller or None, next offset) for one controller of a record."""
    if kind == 1:
        v = list(struct.unpack_from(f"<{width}I", d, q))
        if rot:
            return RotConst(v), q + 4 * width
        return Const(v), q + 4 * width
    if kind == 2:
        n = struct.unpack_from("<I", d, q)[0]
        keys = []
        for i in range(n):
            p = q + 4 + (4 + 4 * width) * i
            keys.append((struct.unpack_from("<I", d, p)[0],
                         list(struct.unpack_from(f"<{width}I", d, p + 4))))
        ctrl = RotKeyed(keys, frames) if rot else Keyed(keys, frames)
        return ctrl, q + 4 + (4 + 4 * width) * n
    if kind == 0:
        return None, q
    raise ValueError(f"controller kind {kind}")


IDENTITY = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
UNIT = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]


class Pose:
    __slots__ = ("pos", "rot", "scale", "alpha", "rot_bits")

    def __init__(self, pos, rot_bits, scale, alpha):
        self.pos = pos              # 3 float bit patterns
        self.rot_bits = rot_bits    # the rotation's stored rows, float bits
        self.rot = [[f_to_py(rot_bits[c][r]) for c in range(3)] for r in range(3)]   # 3x3 rows, floats
        self.scale = scale          # 3 float bit patterns
        self.alpha = alpha          # float bit pattern

    def bits(self):
        """SetAnmCtrlWork's local matrix as stored rows of float bits:
        sceVu0MulMatrix(R, S), then sceVu0TransMatrix adding the position."""
        s = self.scale
        m = vu_mul(self.rot_bits, [[s[0], 0, 0, 0], [0, s[1], 0, 0], [0, 0, s[2], 0], [0, 0, 0, ONE]])
        m[3] = [f_add(m[3][k], self.pos[k]) for k in range(3)] + [m[3][3]]
        return m

    def matrix(self):
        """T * R * S as 4x4 rows of floats (bits() read as floats)."""
        m = self.bits()
        return [[f_to_py(m[c][r]) for c in range(4)] for r in range(4)]


class Track:
    """One 0x0102 record: object is the record's, target what it drives
    (through ExtObj chunks, as ccGetExternalIndex follows them)."""

    def __init__(self, obj, target, flags, pos, rot, scale, alpha):
        self.object, self.target, self.flags = obj, target, flags
        self.pos, self.rot, self.scale, self.alpha = pos, rot, scale, alpha

    def at(self, time):
        return Pose(self.pos.at(time) if self.pos else [0, 0, 0],
                    self.rot.bits(time) if self.rot else UNIT,
                    self.scale.at(time) if self.scale else [ONE] * 3,
                    self.alpha.at(time)[0] if self.alpha else ONE)


class Forward:
    __slots__ = ("time", "pose_at", "ended")

    def __init__(self, time, pose_at, ended):
        self.time, self.pose_at, self.ended = time, pose_at, ended

    def __repr__(self):
        return f"Forward(time={self.time}, pose_at={self.pose_at}, ended={self.ended})"


class Animation:
    def __init__(self, c, off, ext=None):
        d = c.data
        ext = ext or {}
        self.object, self.frames, words = struct.unpack_from("<3I", d, off + 8)
        self.tracks, self.materials, self.morphs, self.other = [], [], {}, {}
        self.notes = []         # F_Note records: (frame, object, event, param) in file order
        self.end = None
        p, end = off + 20, off + 20 + 4 * words
        frame = 0
        while p < end:
            t, n = struct.unpack_from("<II", d, p)
            kind = t & 0xFFFF
            q = p + 8
            if kind == TOP:
                frame = struct.unpack_from("<I", d, q)[0]
                if frame >= 0xFFFFFF00:
                    self.end = frame
            elif kind == OBJECT:
                obj, flags = struct.unpack_from("<II", d, q)
                q += 8
                pos, q = read_ctrl(d, q, flags & 7, 3, self.frames)
                rot, q = read_ctrl(d, q, flags >> 3 & 7, 3, self.frames, rot=True)
                scale, q = read_ctrl(d, q, flags >> 6 & 7, 3, self.frames)
                alpha, q = read_ctrl(d, q, flags >> 9 & 7, 1, self.frames)
                self.tracks.append(Track(obj, resolve(ext, obj), flags, pos, rot, scale, alpha))
            elif kind == MATERIAL:
                mat, flags = struct.unpack_from("<II", d, q)
                q += 8
                u, q = read_ctrl(d, q, flags & 7, 1, self.frames)
                v, q = read_ctrl(d, q, flags >> 3 & 7, 1, self.frames)
                self.materials.append((mat, u, v))
            elif kind == MORPHER:
                mph, count = struct.unpack_from("<II", d, q)
                count &= 0xFFFF
                pairs = [struct.unpack_from("<II", d, q + 8 + 8 * i) for i in range(count)]
                self.morphs.setdefault(mph, []).append((frame, pairs))
            elif kind == NOTE:
                self.notes.append((frame,) + struct.unpack_from("<3I", d, q))
            else:
                self.other[kind] = self.other.get(kind, 0) + 1
            p += 8 + 4 * n
        if p != end:
            raise ValueError(f"anime sub-chunks overrun at 0x{p:x}")

    @property
    def loops(self):
        return self.end == END_LOOP

    def clamp(self, time):
        return min(time, ((self.frames - 1) << 8) & M32) if self.frames else 0

    def forward(self, time, step):
        """One ccAnm::_AnimateForward(step) from time: the time after, the
        time the pose was evaluated at (None for a step of 0) and whether a
        play-once animation has ended (its return value)."""
        last = ((self.frames - 1) << 8) & M32
        new = (time + step) & M32
        ended = False
        if last < new:
            step = (step - (new - last)) & M32
            new, ended = last, True
        pose_at = new if step else None
        if new >> 8 != time >> 8:
            # DecodeFrameChunk runs the records of the frames passed and
            # meets the end Top after the last frame's
            ended = False
            if new >> 8 >= self.frames - 1:
                if self.loops:
                    new = 0
                else:
                    ended = True
        return Forward(new, pose_at, ended)

    def passed_notes(self, time, step):
        """The F_Note records one _AnimateForward(step) from time leaves on
        the anm's note list, in the order NoteProcess hands them on: the
        records of frames old + 1 .. new (frame 0's never), last first. Empty
        when the step does not change the frame."""
        last = ((self.frames - 1) << 8) & M32
        new = min((time + step) & M32, last)
        old, new = time >> 8, new >> 8
        return [n for n in reversed(self.notes) if old < n[0] <= new]

    def forward_notes(self, time, step):
        """forward(time, step) and the (event, param) of each note it
        passed, as NoteProcess hands them on."""
        return self.forward(time, step), [(e, p) for _, _, e, p in self.passed_notes(time, step)]

    def looped(self, elapsed):
        """The time shown after `elapsed` ticks of continuous play, for steps
        that divide the loop: a loop repeats frames 1 .. N-1."""
        last = max(0, (self.frames - 1) << 8)
        if not self.loops or last == 0:
            return min(elapsed, last)
        return (elapsed - 1) % last + 1 if elapsed else 0

    def poses(self, time):
        """[(track, Pose)] at time (1/256 frames)."""
        t = self.clamp(time)
        return [(tr, tr.at(t)) for tr in self.tracks]

    def morph_weights(self, frame):
        """{morpher: [(target model, weight bits)]} from the last F_Morpher
        record at or before frame."""
        out = {}
        for mph, recs in self.morphs.items():
            for f, pairs in recs:
                if f <= frame:
                    out[mph] = pairs
        return out

    def uv(self, time):
        """[(material, u bits, v bits)] texture offsets (default 0)."""
        t = self.clamp(time)
        return [(m, u.at(t)[0] if u else 0, v.at(t)[0] if v else 0) for m, u, v in self.materials]


def material_offsets(u, v, crop_u, crop_v):
    """What SetAnmCtrlWork stores in ccMaterial.u and .v for texture offsets
    u, v (float bits) and the Material chunk's cropU/cropV: s16(int(x * 4096))
    minus the crop, in 16 bits."""
    return [(s16(f_to_int(f_mul(x, 0x45800000))) - s16(crop)) & 0xFFFF
            for x, crop in ((u, crop_u), (v, crop_v))]


def resolve(ext, obj):
    """Follow ExtObj targets to the object they stand for."""
    seen = 0
    while obj in ext and ext[obj] != obj and seen < 64:
        obj = ext[obj]
        seen += 1
    return obj


def morph_lanes(weight):
    """The three per-axis weights Modify multiplies by: vftoi12 of the weight
    in a word whose upper half is the sign-extension of the float's bits, the
    word ORed with itself shifted 16 and 32, and its first three halfwords.
    For weights in [0, 8) - every weight on the disc - all three are
    int(w * 4096); a negative weight leaves y and z at -1."""
    x = f_to_int(f_mul(weight, 0x45800000)) & M32
    v = (0xFFFFFFFF << 32 if weight & 0x80000000 else 0) | x
    t = (v | v << 16 | v << 32) & 0xFFFFFFFFFFFFFFFF
    return [s16(t >> (16 * k)) for k in range(3)]


def morph(base, base_scale, targets):
    """ccMorpher::Modify on stored positions.

    base: [(x, y, z)] s16; base_scale: the base model's vertexScale bits;
    targets: [(positions, vertexScale bits, weight bits)]. Returns the
    blended s16 positions."""
    prepared = []
    for pos, scale, weight in targets:
        if f_cmp(scale, base_scale):
            ratio = f_div(scale, base_scale)
            pos = [tuple(s16(f_to_int(f_mul(ratio, f_from_int(c)))) for c in p) for p in pos]
        prepared.append((pos, morph_lanes(weight)))
    out = []
    for i, v in enumerate(base):
        res = []
        for k in range(3):
            acc = 0
            for pos, w in prepared:
                acc += s16(pos[i][k] - v[k]) * w[k]
            acc = (acc + 0x80000000) % (1 << 32) - 0x80000000       # 32-bit lanes
            res.append(max(-0x8000, min(0x7FFF, s16(acc >> 12) + v[k])))
        out.append(tuple(res))
    return out


def animations(c, ext=None):
    """[(offset, Animation)] for every Anime chunk of a tools/ccs.py Ccs."""
    out = []
    for off, t, n, end in c.chunks():
        if t is None:
            break
        if t & 0xFFFF == 0x0700:
            out.append((off, Animation(c, off, ext)))
    return out


def main():
    import ccs
    import ccsmodel
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("list").add_argument("file")
    for name in ("pose", "morph"):
        p = sub.add_parser(name)
        p.add_argument("file")
        p.add_argument("anime")
        p.add_argument("frame", type=float)
    args = parser.parse_args()
    c = ccs.Ccs(ccs.load(args.file))
    scene = ccsmodel.Scene(c)
    name = lambda o: c.objects[o][0] if o < len(c.objects) else f"#{o}"   # noqa: E731
    anims = animations(c, scene.ext)
    if args.cmd == "list":
        for off, a in anims:
            end = {END_ONCE: "once", END_LOOP: "loop"}.get(a.end, a.end)
            print(f"0x{off:08x} {name(a.object):24} {a.frames:5} frames {end:5} "
                  f"{len(a.tracks):3} objects {len(a.materials)} materials "
                  f"{len(a.morphs)} morphers {len(a.notes)} notes  "
                  f"other {dict((hex(k), v) for k, v in a.other.items())}")
        return 0
    a = next((a for _, a in anims if name(a.object) == args.anime), None)
    if a is None:
        print(f"no Anime chunk {args.anime}", file=sys.stderr)
        return 1
    time = int(args.frame * 256)
    if args.cmd == "pose":
        for tr, pose in a.poses(time):
            p = [round(f_to_py(v), 4) for v in pose.pos]
            s = [round(f_to_py(v), 4) for v in pose.scale]
            r = [round(x, 4) for row in pose.rot for x in row]
            print(f"{name(tr.target):28} pos {p} scale {s} alpha {f_to_py(pose.alpha):.3f} rot {r}")
    else:
        for mph, pairs in a.morph_weights(int(args.frame)).items():
            print(name(mph), [(name(t), round(f_to_py(w), 4)) for t, w in pairs])
    return 0


if __name__ == "__main__":
    sys.exit(main())
