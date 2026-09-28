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
(m2a 0x00105ec0: the axis from EigenVector, the angle an atan2, so at most
half a turn). ccAnmCtrlRot_Set (0x00146f30) keeps a matrix W, starting at
the first key's, multiplies each passed segment's rotation on the left, and
returns axis_angle(axis, angle * t / duration) * W: a constant-speed turn
about one axis between keys, the short way round. A single-value rotation
is sceVu0RotMatrix of degrees * pi / 180, not quantised. The final hold
segment of a rotation is the last key's delta over a duration of
(last - N) * 256 wrapped to u32, so it creeps by angle * t / 2^32 - kept.
sceVu0RotMatrix is reproduced here in the EE's arithmetic (rot_bits); the
axis-angle work is done in double, where the game uses float, sinf, cosf
and a double atan2 - so keyed rotations match it to 1e-5 plus 1e-7 per key
passed (the game's running matrix drifts), not bit for bit.

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
import math
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


def rot_matrix(rad):
    """rot_bits as rows of Python floats (math orientation)."""
    m = rot_bits(rad)
    return [[f_to_py(m[c][r]) for c in range(3)] for r in range(3)]


def mul3(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def transpose3(a):
    return [[a[j][i] for j in range(3)] for i in range(3)]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def normalize(v):
    n = math.sqrt(sum(c * c for c in v))
    return [c / n for c in v] if n else [0.0, 0.0, 0.0]


def axis_angle(m):
    """m2a: (axis, angle) of a rotation matrix, the way EigenVector,
    EigenVec2Mat and m2a find them; None where EigenVector gives up."""
    a = [[m[i][j] - (1.0 if i == j else 0.0) for j in range(3)] for i in range(3)]
    # EigenVector works on the stored (column-major) rows: math columns
    col = [[a[r][c] for r in range(3)] for c in range(3)]
    best, row = -1.0, 0
    for i in range(3):
        for j in range(3):
            r1, r2, c1, c2 = (i + 1) % 3, (i + 2) % 3, (j + 1) % 3, (j + 2) % 3
            cof = abs(col[r1][c1] * col[r2][c2] - col[r1][c2] * col[r2][c1])
            if cof > best:
                best, row = cof, i
    u = normalize(cross(col[(row + 1) % 3], col[(row + 2) % 3]))
    if best < f_to_py(EIGEN_MIN):
        return None
    w = normalize(col[(row + 1) % 3])
    x = normalize(cross(u, w))
    f = [u, w, x]
    local = mul3(f, mul3(m, transpose3(f)))
    return u, math.atan2(local[2][1], local[1][1])


def axis_matrix(axis, angle):
    """a2m: c I + s [u]x + (1 - c) u u^T."""
    if axis is None:
        return [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    x, y, z = axis
    c, s = math.cos(angle), math.sin(angle)
    t = 1.0 - c
    return [[c + t * x * x, t * x * y - s * z, t * x * z + s * y],
            [t * x * y + s * z, c + t * y * y, t * y * z - s * x],
            [t * x * z - s * y, t * y * z + s * x, c + t * z * z]]


def key_radians(deg):
    """A key's degrees (float bits) quantised to s16 turns, in radians (bits)."""
    return [f_mul(FROM_S16, f_from_int(s16(f_to_int(f_mul(TO_S16, d))))) for d in deg]


def const_radians(deg):
    return [f_div(f_mul(PI, d), DEG180) for d in deg]


class RotKeyed:
    """ccAnmCtrlRot_SetCtrl's segments and ccAnmCtrlRot_Set's walk."""

    def __init__(self, keys, frames):
        mats = [rot_matrix(key_radians(v)) for _, v in keys]
        self.base = mats[0]
        t = (keys[0][0] << 8) & M32
        segs = []
        if t:
            segs.append((t, None, 0.0))
        aa = (None, 0.0)
        for i in range(1, len(keys)):
            t_new = (keys[i][0] << 8) & M32
            found = axis_angle(mul3(mats[i], transpose3(mats[i - 1])))
            aa = found if found else (None, 0.0)
            segs.append(((t_new - t) & M32, aa[0], aa[1]))
            t = t_new
        end = (frames << 8) & M32
        if t < end:
            segs.append(((t - end) & M32, aa[0], aa[1]))
        self.segs = segs

    def passed(self, time):
        """How many segments the walk to `time` multiplies in."""
        t, i = time, 0
        while i < len(self.segs) - 1 and self.segs[i][0] < t:
            t = (t - self.segs[i][0]) & M32
            i += 1
        return i

    def at(self, time):
        w = self.base
        t = time
        i = 0
        while i < len(self.segs) - 1 and self.segs[i][0] < t:
            dur, axis, angle = self.segs[i]
            w = mul3(axis_matrix(axis, angle), w)
            t = (t - dur) & M32
            i += 1
        if not self.segs:
            return w
        dur, axis, angle = self.segs[i]
        frac = f_to_py(f_div(u2f(t), u2f(dur))) if dur else 0.0
        return mul3(axis_matrix(axis, angle * frac), w)


class Const:
    def __init__(self, value):
        self.value = value

    def at(self, time):
        return self.value


class RotConst(Const):
    """A single-value rotation: sceVu0RotMatrix of the degrees in radians."""

    def __init__(self, deg):
        self.bits = rot_bits(const_radians(deg))
        super().__init__([[f_to_py(self.bits[c][r]) for c in range(3)] for r in range(3)])


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


class Pose:
    __slots__ = ("pos", "rot", "scale", "alpha")

    def __init__(self, pos, rot, scale, alpha):
        self.pos = pos          # 3 float bit patterns
        self.rot = rot          # 3x3 rows, Python floats
        self.scale = scale      # 3 float bit patterns
        self.alpha = alpha      # float bit pattern

    def matrix(self):
        """T * R * S as 4x4 rows."""
        p = [f_to_py(v) for v in self.pos]
        s = [f_to_py(v) for v in self.scale]
        return [[self.rot[i][0] * s[0], self.rot[i][1] * s[1], self.rot[i][2] * s[2], p[i]]
                for i in range(3)] + [[0.0, 0.0, 0.0, 1.0]]


class Track:
    """One 0x0102 record: object is the record's, target what it drives
    (through ExtObj chunks, as ccGetExternalIndex follows them)."""

    def __init__(self, obj, target, flags, pos, rot, scale, alpha):
        self.object, self.target, self.flags = obj, target, flags
        self.pos, self.rot, self.scale, self.alpha = pos, rot, scale, alpha

    def at(self, time):
        return Pose(self.pos.at(time) if self.pos else [0, 0, 0],
                    self.rot.at(time) if self.rot else IDENTITY,
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
