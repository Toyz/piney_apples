#!/usr/bin/env python3
"""Models out of CCSF files, as Wavefront OBJ - see docs/formats/ccs.md.

    tools/ccsmodel.py ls    FILE                  every model chunk and its mmats
    tools/ccsmodel.py obj   FILE OUT.obj [--model NAME ...] [--colours] [--mtl]
                            [--anime ANM_x | --frame N [--frames-from FILE2]]
    tools/ccsmodel.py check ARCHIVE [...]         decode every model, report problems

FILE is an inflated .ccs, or ARCHIVE::MEMBER (see tools/ccs.py).

The model chunk is what ccStream::Decode_Model (0x0014bce0) reads: a header,
then per "mmat" (a run of vertices sharing one material) a few header words
and vertex arrays in one of four layouts, picked by the model's mtype:

    mtype & 4   Decode_Mmat02 (0x0014a600), drawn by ccModel::DrawBoneType.
                With offsetNum 0 ("bone"): every vertex rides one clump node.
                Otherwise ("skin"): each vertex is 1+ weighted entries, one
                per clump node, each holding the position in that node's
                space.
    mtype & 8   DecodeShadowModel (0x00140920): positions and a triangle
                list, for the shadow volume.
    otherwise   Decode_Mmat01 (0x0014b890) ("rigid"): positions in the
                owning object's space.

Positions are s16 * vertexScale / 4096: Decode_Mmat02 turns them into floats
with ITOF4 and a vertexScale / 256 multiply; ccBbox_SetBox uses the same
factor for every type's bounding box. Normals are s8 * 1/64 (their length is
64). The normal word's fourth byte is the GS strip flag: 1 starts a strip
(no triangle), 0 closes a triangle with the two vertices before it; strips
run on across the 48-vertex VU batches because the GS keeps its vertex queue.
Colours are RGBA with 0x80 = 1.0. ST is u16 S, T with 256 = one texture
repeat (the model packet's STROW offset wraps at 256); T = 0 is the first row
stored in the texture chunk, which tools/ccstex.py puts at the bottom of the
PNG, so OBJ v = T / 256 as it is.

The GS does not cull, and strips are exported with alternating winding
counted from each strip's start - for most models that faces the stored
normals; some are authored the other way round.

Skinned and bone vertices only mean something once clump nodes are posed:
--anime takes frame 0 of an Anime chunk (the controller layout ccAnmChunk::
ConvLCNum2ALCNum reads), --frame takes frame N of the frame section (the F_Obj
records ccStream::DecodeF_Obj reads). A node's local matrix is
T(pos) * Rx * Ry * Rz * S(scale) (ccCoord::SetMatrix_PosRotZYXScale, and the
same product in ccAnm::SetAnmCtrlWork), its world matrix its parent's (the Obj
chunk's parent) times that. Rigid models are placed by their object's world
matrix when a pose is given, and left in their own space otherwise.
"""

import argparse
import math
import pathlib
import re
import struct
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import ccs  # noqa: E402

POS_UNIT = 1 / 4096     # times vertexScale
NORMAL_UNIT = 1 / 64
ST_UNIT = 1 / 256


class Mmat:
    """One material's worth of a model, as the decoder leaves it.

    kind: "rigid", "bone", "skin" or "shadow". positions/normals are None for
    "skin", whose weights hold per vertex a list of (slot, weight, position,
    normal) in the slot's node space; slot is the clump node for "bone".
    """
    __slots__ = ("kind", "name", "material", "slot", "positions", "normals",
                 "flags", "colours", "uvs", "weights", "triangles")

    def __init__(self, kind):
        self.kind = kind
        self.name = self.material = self.slot = None
        self.positions = self.normals = self.flags = None
        self.colours = self.uvs = self.weights = None
        self.triangles = []

    @property
    def vertex_count(self):
        return len(self.weights if self.kind == "skin" else self.positions)


class Model:
    __slots__ = ("offset", "end", "obj", "scale", "mtype", "flag", "zoffs", "mmats")


def strip_triangles(flags):
    """Triangles the GS draws for a strip-flagged vertex run.

    Every vertex with flag 0 closes a triangle with the two before it;
    winding alternates from the start of each strip (the first of the two
    flag-1 vertices that open it)."""
    tris = []
    start = 0
    for i, f in enumerate(flags):
        if f:
            if i == 0 or not flags[i - 1]:
                start = i
            continue
        if i < 2:
            continue
        if (i - start) % 2:
            tris.append((i - 2, i - 1, i))
        else:
            tris.append((i - 1, i - 2, i))
    return tris


def _positions(d, q, n, unit):
    out = []
    for i in range(n):
        x, y, z = struct.unpack_from("<3h", d, q + 6 * i)
        out.append((x * unit, y * unit, z * unit))
    return out, ccs._align4(q + 6 * n)


def _normals(d, q, n):
    normals, flags = [], []
    for i in range(n):
        x, y, z, f = struct.unpack_from("<3bB", d, q + 4 * i)
        normals.append((x * NORMAL_UNIT, y * NORMAL_UNIT, z * NORMAL_UNIT))
        flags.append(f)
    return normals, flags, q + 4 * n


def _uvs(d, q, n):
    out = []
    for i in range(n):
        s, t = struct.unpack_from("<HH", d, q + 4 * i)
        out.append((s * ST_UNIT, t * ST_UNIT))
    return out, q + 4 * n


def decode_model(c, p):
    """Decode the model chunk at p. Raises if the result ends anywhere but
    where Ccs.model_end says."""
    d = c.data
    m = Model()
    m.offset = p
    q = p + 8
    m.obj, m.scale, mtype, count, m.flag, m.zoffs = struct.unpack_from("<IfHHHh", d, q)
    if c.version < 0x100:
        mtype &= 0xFF
    m.mtype = mtype
    unit = m.scale * POS_UNIT
    q += 20
    m.mmats = []
    for _ in range(count):
        if mtype & 2:
            name, material, vn, _bones, on = struct.unpack_from("<5I", d, q)
            q += 20
        elif mtype & 4:
            name = None
            material, vn, on = struct.unpack_from("<3I", d, q)
            q += 12
        elif mtype & 8:
            name = material = None
            vn = on = 0
        else:
            name, material, vn = struct.unpack_from("<3I", d, q)
            on = 0
            q += 12
        if mtype & 4:
            if on:
                mm = Mmat("skin")
                entries = []
                for k in range(on):
                    x, y, z, w = struct.unpack_from("<3hH", d, q + 8 * k)
                    entries.append(((x * unit, y * unit, z * unit), w))
                q += 8 * on
                normals, flags, q = _normals(d, q, on)
                mm.weights, mm.flags = [], []
                vertex = []
                for (pos, w), nrm, f in zip(entries, normals, flags):
                    # Low 9 bits weight (256 = 1.0), bit 9 last entry of the
                    # vertex, top 6 bits the clump node (0x0014ab5c).
                    vertex.append((w >> 10, (w & 0x1FF) / 256, pos, nrm))
                    if w & 0x200:
                        mm.weights.append(vertex)
                        mm.flags.append(f)
                        vertex = []
                mm.uvs, q = _uvs(d, q, len(mm.weights))
            else:
                mm = Mmat("bone")
                mm.slot = struct.unpack_from("<I", d, q)[0]
                mm.positions, q = _positions(d, q + 4, vn, unit)
                mm.normals, mm.flags, q = _normals(d, q, vn)
                mm.uvs, q = _uvs(d, q, vn)
            mm.triangles = strip_triangles(mm.flags)
        elif mtype & 8:
            mm = Mmat("shadow")
            vnum, inum = struct.unpack_from("<ii", d, q)
            q += 8
            mm.positions = []
            if vnum:
                mm.positions, q = _positions(d, q, vnum, unit)
                for k in range(int(inum / 3)):
                    mm.triangles.append(struct.unpack_from("<3i", d, q + 12 * k))
                q += 12 * int(inum / 3)
        else:
            mm = Mmat("rigid")
            mm.positions, q = _positions(d, q, vn, unit)
            if not mtype & 0x40:
                mm.normals, mm.flags, q = _normals(d, q, vn)
            if not mtype & 0x200:
                # Read either way; with mtype & 1 (lit) the decoder drops them.
                cols = [tuple(d[q + 4 * i:q + 4 * i + 4]) for i in range(vn)]
                if not mtype & 1:
                    mm.colours = cols
                q += 4 * vn
            if not mtype & 0x400:
                mm.uvs, q = _uvs(d, q, vn)
            if mm.flags is not None:
                mm.triangles = strip_triangles(mm.flags)
        mm.name = name
        mm.material = material
        m.mmats.append(mm)
    m.end = q
    if q != c.model_end(p):
        raise ValueError(f"model at 0x{p:x} decodes to 0x{q:x}, "
                         f"walk says 0x{c.model_end(p):x}")
    return m


def models(c):
    for off, t, n, end in c.chunks():
        if t is None:
            break
        if t & 0xFFFF == 0x0800:
            yield decode_model(c, off)


# ---------------------------------------------------------------------------
# scene: objects, clumps, materials, poses


class Scene:
    """What a model needs from the rest of the file: Obj parents and models
    (Decode_Obj 0x0014ca40), clump node lists (Decode_Clump 0x0014c200),
    material textures (Decode_Material 0x0014d300), Anime chunks, and the
    ExtObj map (Decode_ExtObj 0x0014cb80: u32 object, u32 parent, u32 target)
    from an animation's copy of an object to the object it drives."""

    def __init__(self, c):
        self.c = c
        d = c.data
        self.parent = {}        # obj -> parent obj (0 = none)
        self.model_owner = {}   # model obj -> obj
        self.clumps = []        # [(clump obj, [node objs])]
        self.texture = {}       # material obj -> texture obj
        self.anime = {}         # anime obj -> chunk offset
        self.ext = {}           # ExtObj obj -> target obj
        self.frame_section = None
        for off, t, n, end in c.chunks():
            if t is None:
                break
            kind = t & 0xFFFF
            q = off + 8
            if self.frame_section is not None:
                continue
            if kind == 0x0100:
                obj, parent, model = struct.unpack_from("<3I", d, q)
                # A shadow model word follows from version 0x96 on.
                shadow = struct.unpack_from("<I", d, q + 12)[0] if c.version >= 0x96 else 0
                self.parent[obj] = parent
                for m in (model, shadow):
                    if m:
                        self.model_owner.setdefault(m, obj)
            elif kind == 0x0900:
                obj, count = struct.unpack_from("<IH", d, q)
                self.clumps.append((obj, list(struct.unpack_from(f"<{count}I", d, q + 8))))
            elif kind == 0x0200:
                obj, tex = struct.unpack_from("<II", d, q)
                self.texture[obj] = tex
            elif kind == 0x0A00:
                obj, _parent, target = struct.unpack_from("<3I", d, q)
                self.ext[obj] = target
            elif kind == 0x0700:
                self.anime[struct.unpack_from("<I", d, q)[0]] = off
            elif kind == 0x0005:
                self.frame_section = end

    def name(self, obj):
        return self.c.objects[obj][0] if obj and obj < len(self.c.objects) else None

    def find(self, name):
        for i, (n, _) in enumerate(self.c.objects):
            if n == name:
                return i
        raise KeyError(name)

    def clump_of(self, obj):
        for clump, nodes in self.clumps:
            if obj in nodes:
                return nodes
        return None

    def anime_locals(self, anime_obj):
        """{obj: (pos, rot_degrees, scale)} at frame 0 of an Anime chunk.

        The chunk is u32 object, u32 frame count, u32 data words, then
        sub-chunks like the frame section's. Obj controllers (0x0102) are
        u32 object, u32 flags, then position, rotation (degrees) and scale
        controllers picked by flag bits 0-2, 3-5, 6-8: 1 is one value, 2 a
        u32 key count and (u32 frame, value) keys, which hold their first
        value before the first key (ccAnmCtrlFVec3_SetCtrl 0x00146be0,
        ccAnmCtrlRot_SetCtrl 0x001470c0)."""
        d = self.c.data
        off = self.anime[anime_obj]
        words = struct.unpack_from("<I", d, off + 16)[0]
        p, end = off + 20, off + 20 + 4 * words
        out = {}
        while p < end:
            t, n = struct.unpack_from("<II", d, p)
            if t & 0xFFFF == 0x0102:
                obj, flags = struct.unpack_from("<II", d, p + 8)
                q = p + 16
                vals = []
                for shift, default in ((0, (0.0, 0.0, 0.0)), (3, (0.0, 0.0, 0.0)),
                                       (6, (1.0, 1.0, 1.0))):
                    kind = flags >> shift & 7
                    if kind == 1:
                        vals.append(struct.unpack_from("<3f", d, q))
                        q += 12
                    elif kind == 2:
                        keys = struct.unpack_from("<I", d, q)[0]
                        vals.append(struct.unpack_from("<3f", d, q + 8))
                        q += 4 + 16 * keys
                    else:
                        vals.append(default)
                out.setdefault(self.ext.get(obj, obj), tuple(vals))
            p += 8 + 4 * n
        if p != end:
            raise ValueError(f"anime sub-chunks overrun at 0x{p:x}")
        return out

    def frame_locals(self, frame, source=None):
        """{obj: (pos, rot_degrees, scale)} after the frame section's frames
        0..frame (F_Obj records, CCSTRM_FSET_OBJ: myID, flag, t, r, s,
        transparency, dispSW).

        source is another Scene to take the frames from: a cutscene is a
        strNNNNe.tmp holding the models and a strNNNN.tmp holding the
        frames, whose ExtObj targets name objects in files marked '#' in its
        file table. Objects are matched on (name, source path)."""
        source = source or self
        d = source.c.data
        mine = {self.key(o): o for o in self.parent}
        out = {}
        for off, t, n, end in source.c.chunks():
            if t is None:
                break
            if off < source.frame_section:
                continue
            kind = t & 0xFFFF
            if kind == 0xFF01:
                if struct.unpack_from("<I", d, off + 8)[0] > frame:
                    break
            elif kind == 0x0101:
                obj = struct.unpack_from("<I", d, off + 8)[0]
                obj = source.ext.get(obj, obj)
                if source is not self:
                    obj = mine.get(source.key(obj))
                    if obj is None:
                        continue
                v = struct.unpack_from("<9f", d, off + 16)
                out[obj] = (v[0:3], v[3:6], v[6:9])
        return out

    def key(self, obj):
        name, f = self.c.objects[obj]
        return name, self.c.files[f][1:]

    def world(self, locals_):
        """{obj: 4x4 world matrix} for every object with a parent chain."""
        cache = {}

        def w(obj):
            if obj in cache:
                return cache[obj]
            pos, rot, scale = locals_.get(obj, ((0, 0, 0), (0, 0, 0), (1, 1, 1)))
            m = local_matrix(pos, rot, scale)
            parent = self.parent.get(obj, 0)
            if parent and parent != obj:
                m = matmul(w(parent), m)
            cache[obj] = m
            return m

        for obj in set(self.parent) | set(locals_):
            w(obj)
        return cache


def local_matrix(pos, rot_degrees, scale):
    rx, ry, rz = (math.radians(a) for a in rot_degrees)
    m = matmul(rot_x(rx), matmul(rot_y(ry), rot_z(rz)))
    m = matmul(m, [[scale[0], 0, 0, 0], [0, scale[1], 0, 0], [0, 0, scale[2], 0],
                   [0, 0, 0, 1]])
    for i in range(3):
        m[i][3] = pos[i]
    return m


def rot_x(a):
    c, s = math.cos(a), math.sin(a)
    return [[1, 0, 0, 0], [0, c, -s, 0], [0, s, c, 0], [0, 0, 0, 1]]


def rot_y(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, 0, s, 0], [0, 1, 0, 0], [-s, 0, c, 0], [0, 0, 0, 1]]


def rot_z(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, -s, 0, 0], [s, c, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]


def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def apply(m, p, w=1.0):
    return tuple(m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3] * w
                 for i in range(3))


IDENTITY = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]


def place(scene, model, world):
    """[(mmat, positions, normals)] with positions in world space when a pose
    (world) is given; None for mmats that cannot be placed."""
    owner = scene.model_owner.get(model.obj)
    nodes = scene.clump_of(owner) if owner else None
    out = []
    for mm in model.mmats:
        if mm.kind in ("rigid", "shadow"):
            m = world.get(owner, IDENTITY) if world is not None else IDENTITY
            pos = [apply(m, v) for v in mm.positions]
            nrm = [_unit(apply(m, n, 0.0)) for n in mm.normals] if mm.normals else None
            out.append((mm, pos, nrm))
            continue
        if world is None or nodes is None:
            out.append((mm, None, None))
            continue
        # ccModel::DrawBoneType (0x0013f860): slot i is the i-th node of the
        # model's clump; its matrix is inverse(model world) * node world, and
        # the model world goes back on in the VU, so world = node world * p.
        if mm.kind == "bone":
            m = world.get(nodes[mm.slot], IDENTITY) if mm.slot < len(nodes) else IDENTITY
            out.append((mm, [apply(m, v) for v in mm.positions],
                        [_unit(apply(m, n, 0.0)) for n in mm.normals]))
            continue
        pos, nrm = [], []
        for entries in mm.weights:
            acc = [0.0, 0.0, 0.0]
            nacc = [0.0, 0.0, 0.0]
            for slot, weight, p, n in entries:
                m = world.get(nodes[slot], IDENTITY) if slot < len(nodes) else IDENTITY
                wp = apply(m, p)
                wn = apply(m, n, 0.0)
                for i in range(3):
                    acc[i] += weight * wp[i]
                    nacc[i] += weight * wn[i]
            pos.append(tuple(acc))
            nrm.append(_unit(nacc))
        out.append((mm, pos, nrm))
    return out


def _unit(v):
    n = math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
    return tuple(x / n for x in v) if n else tuple(v)


# ---------------------------------------------------------------------------
# OBJ


def write_obj(path, scene, chosen, world, colours=False, mtl=False):
    """Write the chosen models; returns (vertices, triangles, skipped mmats)."""
    path = pathlib.Path(path)
    lines = [f"# {scene.c.name} - tools/ccsmodel.py"]
    mtl_lines = []
    if mtl:
        lines.append(f"mtllib {path.with_suffix('.mtl').name}")
    base = 1
    nverts = ntris = skipped = 0
    materials = set()
    for model in chosen:
        for k, (mm, pos, nrm) in enumerate(place(scene, model, world)):
            if pos is None:
                skipped += 1
                continue
            label = _token(f"{scene.name(model.obj)}.{k}.{mm.kind}")
            lines.append(f"o {label}")
            mat = scene.name(mm.material)
            if mtl and mat:
                safe = _token(mat)
                lines.append(f"usemtl {safe}")
                if safe not in materials:
                    materials.add(safe)
                    mtl_lines += [f"newmtl {safe}", "Kd 1 1 1"]
                    tex = scene.name(scene.texture.get(mm.material))
                    if tex:
                        mtl_lines.append(f"map_Kd {tex}.png")
            for i, v in enumerate(pos):
                col = ""
                if colours and mm.colours:
                    r, g, b, _ = mm.colours[i]
                    col = " " + " ".join(f"{min(x / 128, 1.0):.4f}" for x in (r, g, b))
                lines.append(f"v {v[0]:.6f} {v[1]:.6f} {v[2]:.6f}{col}")
            if mm.uvs:
                lines += [f"vt {u:.6f} {v:.6f}" for u, v in mm.uvs]
            if nrm:
                lines += [f"vn {n[0]:.5f} {n[1]:.5f} {n[2]:.5f}" for n in nrm]
            for tri in mm.triangles:
                a, b, c = (base + i for i in tri)
                if mm.uvs and nrm:
                    lines.append(f"f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}")
                elif mm.uvs:
                    lines.append(f"f {a}/{a} {b}/{b} {c}/{c}")
                elif nrm:
                    lines.append(f"f {a}//{a} {b}//{b} {c}//{c}")
                else:
                    lines.append(f"f {a} {b} {c}")
            base += len(pos)
            nverts += len(pos)
            ntris += len(mm.triangles)
    path.write_text("\n".join(lines) + "\n")
    if mtl:
        path.with_suffix(".mtl").write_text("\n".join(mtl_lines) + "\n")
    return nverts, ntris, skipped


# ---------------------------------------------------------------------------
# checks


def _token(name):
    # OBJ names end at whitespace and '#' starts a comment ("MAT_material #2").
    return re.sub(r"[^A-Za-z0-9_.-]", "_", name)


def check_model(model):
    """Problems a sane decode should not have: out-of-range indices, NaNs,
    strips that do not open with two flag-1 vertices, skin weights that do
    not sum to about 1."""
    problems = []
    for k, mm in enumerate(model.mmats):
        n = mm.vertex_count
        for tri in mm.triangles:
            if not all(0 <= i < n for i in tri):
                problems.append(f"mmat {k}: triangle {tri} outside {n} vertices")
                break
        if mm.positions and any(math.isnan(x) for v in mm.positions for x in v):
            problems.append(f"mmat {k}: NaN position")
        if mm.flags and n and mm.flags[:2] != [1, 1]:
            problems.append(f"mmat {k}: strip flags open {mm.flags[:2]}")
        if mm.weights:
            for entries in mm.weights:
                total = sum(w for _, w, _, _ in entries)
                if not 0.7 <= total <= 1.01:
                    problems.append(f"mmat {k}: weights sum to {total}")
                    break
    return problems


def bounds(points):
    xs, ys, zs = zip(*points)
    return (min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs))


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("ls").add_argument("file")
    p = sub.add_parser("obj")
    p.add_argument("file")
    p.add_argument("out")
    p.add_argument("--model", action="append", default=[],
                   help="MDL_ name to export (repeatable); default every model")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--anime", help="pose clump nodes at frame 0 of this ANM_ chunk")
    g.add_argument("--frame", type=int, help="pose objects at this frame of the frame section")
    p.add_argument("--frames-from", metavar="FILE",
                   help="with --frame: take the frames from this file (strNNNN.tmp "
                        "for the models in strNNNNe.tmp)")
    p.add_argument("--colours", action="store_true", help="write vertex colours as v x y z r g b")
    p.add_argument("--mtl", action="store_true",
                   help="write a .mtl naming TEX_*.png (tools/ccstex.py png) per material")
    p = sub.add_parser("check")
    p.add_argument("archives", nargs="+")
    args = parser.parse_args()

    if args.cmd == "check":
        import collections

        import gzarc
        status = 0
        for spec in args.archives:
            data = gzarc.open_bytes(spec)
            kinds = collections.Counter()
            verts = tris = nmodels = 0
            problems = []
            for m in gzarc.members(data):
                c = ccs.Ccs(gzarc.inflate(data, m))
                for model in models(c):
                    nmodels += 1
                    for mm in model.mmats:
                        kinds[mm.kind] += 1
                        verts += mm.vertex_count
                        tris += len(mm.triangles)
                    for pr in check_model(model):
                        problems.append(f"{m.name} {c.objects[model.obj][0]}: {pr}")
            print(f"{spec}: {nmodels} models, mmats {dict(kinds)}, {verts} vertices, "
                  f"{tris} triangles, {len(problems)} problems")
            for pr in problems[:20]:
                print("  " + pr)
            status |= bool(problems)
        return status

    c = ccs.Ccs(ccs.load(args.file))
    scene = Scene(c)
    if args.cmd == "ls":
        for model in models(c):
            owner = scene.name(scene.model_owner.get(model.obj))
            print(f"0x{model.offset:08x}  {scene.name(model.obj) or '-':30}"
                  f" mtype 0x{model.mtype:03x}  scale {model.scale:g}  flag {model.flag}"
                  f"  zoffs {model.zoffs}  {len(model.mmats)} mmats  owner {owner}")
            for k, mm in enumerate(model.mmats):
                extra = f" slot {mm.slot}" if mm.slot is not None else ""
                if mm.kind == "skin":
                    entries = sum(len(e) for e in mm.weights)
                    extra = f" {entries} weighted entries"
                print(f"    {k:3d} {mm.kind:6} {mm.vertex_count:6d} vertices "
                      f"{len(mm.triangles):6d} triangles  {scene.name(mm.material) or ''}{extra}")
        return 0

    chosen = [m for m in models(c) if not args.model or scene.name(m.obj) in args.model]
    world = None
    if args.anime:
        world = scene.world(scene.anime_locals(scene.find(args.anime)))
    elif args.frame is not None:
        source = Scene(ccs.Ccs(ccs.load(args.frames_from))) if args.frames_from else None
        world = scene.world(scene.frame_locals(args.frame, source))
    nv, nt, skipped = write_obj(args.out, scene, chosen, world, args.colours, args.mtl)
    print(f"{len(chosen)} models, {nv} vertices, {nt} triangles -> {args.out}")
    if skipped:
        print(f"  {skipped} bone/skin mmats left out: they need --anime or --frame")
    return 0


if __name__ == "__main__":
    sys.exit(main())
