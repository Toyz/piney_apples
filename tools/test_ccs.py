#!/usr/bin/env python3
"""Tests for tools/ccs.py and tools/ccsmodel.py against PINEY_VOLUME's disc
(volume.py; Infection by default). Archive tests are skipped when the
volume's DATA.BIN or disc image is absent.

Every model of DATA.BIN is compared word for word with crates/piney-data's
reader (the model_probe example; skipped without cargo): positions, normals
and strip flags, colours, ST, skin entries and triangles. A weapon's normals
are checked against the words at the offsets the layout gives. The skinned
body is posed at frame 0 of its idle animation (tools/anim.py) and its skin
blended as VU1's mc03b_DrawModel1 does it: ccsmodel.skin_positions against
that microcode run in tools/vu.py, word for word, with the slot matrices
ccModel::DrawBoneType makes (the game's sceVu0InversMatrix2 and
sceVu0MulMatrix run in eemu) and with the node world matrices place() uses.
Run: python3 tools/test_ccs.py"""

import collections
import os
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ccs  # noqa: E402
import ccsmodel  # noqa: E402
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA, ISO, ELF = volume.DATA, volume.ISO, volume.ELF
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
PROBE = os.path.join(TARGET, "release", "examples", "model_probe")

# Per volume: ccs.survey of DATA.BIN (members, the kinds whose size field
# disagrees, models, textures), the mmats ccsmodel decodes (kinds,
# vertices, triangles) and the skin vertices by the sum of their weights
# (256 = 1.0).
SURVEY = {
    "infection": (1023, {0x0300: 4306, 0x0800: 190}, 17254, 4306),
    "mutation": (1031, {0x0300: 4333, 0x0800: 194}, 17453, 4333),
    "outbreak": (1072, {0x0300: 4389, 0x0800: 194}, 17613, 4389),
    "quarantine": (1072, {0x0300: 4390, 0x0800: 194}, 17613, 4390),
}
MMATS = {
    "infection": ({"rigid": 16407, "shadow": 3129, "bone": 2994, "skin": 185}, 5417645, 2819008),
    "mutation": ({"rigid": 16400, "shadow": 3201, "bone": 3070, "skin": 189}, 5428057, 2823591),
    "outbreak": ({"rigid": 16555, "shadow": 3202, "bone": 3070, "skin": 189}, 5450567, 2837353),
    "quarantine": ({"rigid": 16555, "shadow": 3202, "bone": 3070, "skin": 189}, 5450567, 2837345),
}
WEIGHT_SUMS = {
    "infection": {192: 15, 198: 2, 206: 3, 212: 17, 255: 73680, 256: 105541},
    "mutation": {192: 15, 198: 2, 206: 3, 212: 17, 255: 75706, 256: 108625},
}
WEIGHT_SUMS["outbreak"] = WEIGHT_SUMS["quarantine"] = WEIGHT_SUMS["mutation"]
# Each stream archive: members, frames and how the files close.
STREAMS = {
    "infection": {"STRCMN": (93, 8494, {"end": 92, "end, reset scene": 1}),
                  "STRCMNE": (93, 8494, {"end": 92, "end, reset scene": 1}),
                  "STR1": (40, 38569, {"end": 40}), "STR1E": (40, 38569, {"end": 40})},
    "mutation": {"STRCMN": (97, 8606, {"end": 96, "end, reset scene": 1}),
                 "STRCMNE": (97, 8606, {"end": 96, "end, reset scene": 1}),
                 "STR1": (40, 38569, {"end": 40}), "STR1E": (40, 38569, {"end": 40}),
                 "STR2": (56, 46520, {"end": 56}), "STR2E": (55, 46319, {"end": 55}),
                 "STRSUB": (26, 25868, {"end": 26}), "STRSUBE": (8, 6604, {"end": 8})},
    "outbreak": {"STRCMNE": (97, 8606, {"end": 96, "end, reset scene": 1}),
                 "STR1E": (40, 38569, {"end": 40}), "STR2E": (55, 46319, {"end": 55}),
                 "STR3E": (48, 44202, {"end": 48}), "STRSUBE": (26, 25868, {"end": 26}),
                 "STRT": (6, 2275, {"end": 6})},
    "quarantine": {"STRCMNE": (97, 8606, {"end": 96, "end, reset scene": 1}),
                   "STR1E": (40, 38569, {"end": 40}), "STR2E": (55, 46319, {"end": 55}),
                   "STR3E": (48, 44202, {"end": 48}), "STR4E": (81, 50596, {"end": 81}),
                   "STRSUBE": (26, 25868, {"end": 26})},
}
KIND = {"rigid": 0, "bone": 1, "skin": 2, "shadow": 3}
NONE = 0xFFFFFFFF


def stream(name):
    return f"{ISO}::STREAM/{name}.BIN"


def stream_member(name, ccs_name):
    """The member of a stream archive whose file is `ccs_name`: Outbreak's
    and Quarantine's archives call every member tmp.ccs."""
    import gzarc
    data = gzarc.open_bytes(stream(name))
    for m in gzarc.members(data):
        c = ccs.Ccs(gzarc.inflate(data, m))
        if c.name == ccs_name:
            return c
    raise KeyError(ccs_name)


def mmat_words(model, mm):
    """A ccsmodel mmat as model_probe writes a piney-data one."""
    unit = model.scale * ccsmodel.POS_UNIT

    def lst(items, fmt, conv):
        return struct.pack("<I", len(items)) + b"".join(struct.pack(fmt, *conv(t)) for t in items)

    raw = lambda xs: [round(x * 64) for x in xs]     # noqa: E731  # s8 / 64, exact
    out = struct.pack("<B3I", KIND[mm.kind], *(NONE if v is None else v for v in (mm.name, mm.material, mm.slot)))
    skin = mm.kind == "skin"
    pos = [] if skin else mm.stored
    assert skin or all(p == tuple(c * unit for c in s) for p, s in zip(mm.positions, mm.stored))
    out += lst(pos, "<3h", lambda p: p)
    out += lst(mm.normals or [], "<3b", raw)
    out += lst(mm.flags or [], "<B", lambda f: (f,))
    out += lst(mm.colours or [], "<4B", lambda c: c)
    out += lst(mm.uvs or [], "<2H", lambda uv: [round(x * 256) for x in uv])
    out += struct.pack("<I", len(mm.stored) if skin else 0)
    if skin:
        for entries, raws in zip(mm.weights, mm.stored):
            out += struct.pack("<B", len(entries))
            for (slot, w, p, n), (rslot, rw, xyz) in zip(entries, raws):
                assert (slot, w, p) == (rslot, rw / 256, tuple(c * unit for c in xyz))
                out += struct.pack("<BH3h3b", slot, rw, *xyz, *raw(n))
    out += lst(mm.triangles, "<3I", lambda t: [i & 0xFFFFFFFF for i in t])
    return out


@unittest.skipUnless(os.path.exists(DATA), "needs the volume's DATA.BIN")
class DataBinTest(unittest.TestCase):
    def test_survey(self):
        members, disagree, models, textures = SURVEY[volume.NAME]
        s = ccs.survey(DATA)
        self.assertEqual((s.members, s.clean), (members, members))
        self.assertEqual(dict(s.lost), {})
        # Only textures and skinned models run past or short of their size field.
        self.assertEqual(dict(s.disagree), disagree)
        self.assertEqual(s.counts[0x0800], models)
        self.assertEqual(s.counts[0x0300], textures)
        # Every DATA.BIN file has an empty frame section: Frame, then Top "end".
        self.assertEqual(dict(s.closes), {"end": members})
        self.assertEqual(s.frames, 0)

    def test_every_model(self):
        import gzarc
        data = gzarc.open_bytes(DATA)
        kinds = collections.Counter()
        sums = collections.Counter()
        verts = tris = 0
        rust = None
        if shutil.which("cargo"):
            subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data", "--example",
                            "model_probe"], cwd=ROOT, check=True)
            rust = subprocess.run([PROBE, DATA], capture_output=True, check=True).stdout
        q = 0
        for m in gzarc.members(data):
            c = ccs.Ccs(gzarc.inflate(data, m))
            models = list(ccsmodel.models(c))     # raises if a decode overruns
            words = struct.pack("<I", len(m.name)) + m.name.encode() + struct.pack("<I", len(models))
            for model in models:
                self.assertEqual(ccsmodel.check_model(model), [], m.name)
                words += struct.pack("<3I2H", model.offset, model.obj, ccsmodel.float_bits(model.scale),
                                     model.mtype, len(model.mmats))
                for mm in model.mmats:
                    kinds[mm.kind] += 1
                    verts += mm.vertex_count
                    tris += len(mm.triangles)
                    if mm.kind == "skin":
                        sums.update(sum(w for _, w, _ in e) for e in mm.stored)
                    words += mmat_words(model, mm)
            if rust is not None:
                self.assertEqual(rust[q:q + len(words)], words, m.name)
                q += len(words)
        if rust is not None:
            self.assertEqual(q, len(rust))
        want_kinds, want_verts, want_tris = MMATS[volume.NAME]
        self.assertEqual(dict(kinds), want_kinds)
        self.assertEqual((verts, tris), (want_verts, want_tris))
        self.assertEqual(dict(sums), WEIGHT_SUMS[volume.NAME])

    def test_weapon(self):
        c = ccs.Ccs(ccs.load(DATA + "::cw1hst00"))
        models = list(ccsmodel.models(c))
        self.assertEqual(len(models), 2)
        sword, shadow = models
        self.assertEqual((c.objects[sword.obj][0], sword.mtype, sword.scale),
                         ("MDL_cw1hst00", 1, 512.0))
        (mm,) = sword.mmats
        self.assertEqual((mm.kind, mm.vertex_count, len(mm.triangles)), ("rigid", 118, 68))
        self.assertIsNone(mm.colours)   # mtype & 1: read and dropped
        lo, hi = ccsmodel.bounds(mm.positions)
        # s16 * 512 / 4096: the raw extremes are (4, 4, -670) and (194, 48, 446).
        self.assertEqual(lo, (0.5, 0.5, -83.75))
        self.assertEqual(hi, (24.25, 6.0, 55.75))
        us = [u for u, v in mm.uvs]
        vs = [v for u, v in mm.uvs]
        self.assertTrue(0 <= min(us) and max(us) <= 1 and 0 <= min(vs) and max(vs) <= 1)
        # The normal words: after the chunk header, the model header (20),
        # the mmat header (name, material, vertex count) and the positions
        # padded to 4 bytes; s8 x, y, z / 64 and the strip flag byte.
        q = sword.offset + 8 + 20 + 12 + (6 * 118 + 3) // 4 * 4
        words = [struct.unpack_from("<3bB", c.data, q + 4 * i) for i in range(118)]
        self.assertEqual(mm.normals, [(x / 64, y / 64, z / 64) for x, y, z, _ in words])
        self.assertEqual(mm.flags, [f for *_, f in words])
        (sh,) = shadow.mmats
        self.assertEqual((sh.kind, sh.vertex_count, len(sh.triangles)), ("shadow", 17, 26))

    def test_skinned_body(self):
        import vu
        from eemu import f_to_py
        from test_anim import machine_class
        c = ccs.Ccs(ccs.load(DATA + "::cbu1body"))
        scene = ccsmodel.Scene(c)
        (body,) = [m for m in ccsmodel.models(c) if scene.name(m.obj) == "MDL_cbalbody"]
        self.assertEqual(body.mtype, 5)
        self.assertEqual([mm.kind for mm in body.mmats], ["bone"] * 21 + ["skin"])
        skin = body.mmats[-1]
        self.assertEqual(skin.vertex_count, 1137)
        self.assertEqual(sum(len(v) for v in skin.weights), 1713)
        self.assertEqual(len(skin.triangles), 611)
        world = scene.world(scene.anime_locals(scene.find("ANM_cbu1nut0")))
        owner = scene.model_owner[body.obj]
        nodes = [world.get(n, ccsmodel.IDENTITY) for n in scene.clump_of(owner)]

        # ccModel::DrawBoneType's slots: inverse(model world) * node world,
        # by the game's own routines.
        prog = volume.program(ELF)
        m = machine_class()(prog)
        sym = lambda n: prog.symbol_named(n).value      # noqa: E731
        IN, INV, OUT = 0x01800000, 0x01800100, 0x01800200

        def put(a, mat):
            for i, w in enumerate(x for col in mat for x in col):
                m.store(a + 4 * i, 4, w)

        def get(a):
            return [[m.load(a + 16 * r + 4 * k, 4) for k in range(4)] for r in range(4)]

        put(IN, world[owner])
        m.call(sym("sceVu0InversMatrix2__FPA4_fPA4_f"), [INV, IN])
        game_slots = []
        for node in nodes:
            put(IN, node)
            m.call(sym("sceVu0MulMatrix"), [OUT, INV, IN])
            game_slots.append(get(OUT))

        # mc03b_DrawModel1's first pass on one vertex at a time: the slot
        # matrices at VU1 20 + 4 * slot, the vertex count at the buffer's top,
        # then per entry its weight * 16, matrix address and (first entry)
        # entry count, its position, its normal; the blended position lands
        # 2 qwords past the first entry (sq vf22, 2(vi04)) before 0x463.
        mc = vu.Microcode(ELF)
        entry = mc.vu1.labels["mc03b_DrawModel1"]
        top = 0x120

        def vu_skin(slots):
            out = []
            for entries in skin.stored:
                u = vu.Vu(mc.vu1)
                for i, mat in enumerate(slots):
                    for r in range(4):
                        u.mem[20 + 4 * i + r] = list(mat[r])
                u.mem[top] = [1, 0, 0, 0]
                for j, (slot, weight, xyz) in enumerate(entries):
                    q = top + 1 + 3 * j
                    u.mem[q] = [weight * 16, 20 + 4 * slot, len(entries) if j == 0 else 0, 0]
                    u.mem[q + 1] = ccsmodel.bone_floats(body, xyz) + [ccsmodel.ONE]
                    u.mem[q + 2] = [0, 0, 0, 0]
                u.top = top
                u.run(entry, stop=entry + 0x30)
                out.append(u.mem[top + 3][:3])
            return out

        self.assertEqual(ccsmodel.skin_positions(body, skin, game_slots), vu_skin(game_slots))
        want = vu_skin(nodes)
        self.assertEqual(ccsmodel.skin_positions(body, skin, nodes), want)

        # place() blends with the node world matrices: what VU1 makes of
        # them, so the figure's bounds are the microcode's.
        placed = ccsmodel.place(scene, body, world)
        self.assertEqual(placed[-1][1], [tuple(f_to_py(x) for x in v) for v in want])
        lo, hi = ccsmodel.bounds(placed[-1][1])
        self.assertEqual(lo[2], min(f_to_py(v[2]) for v in want))
        self.assertEqual(hi[2], max(f_to_py(v[2]) for v in want))
        # Inside the file's BOX_bbox across (+-90, +-80) and below its top
        # (230), bone parts included.
        lo, hi = ccsmodel.bounds([v for mm, pos, nrm in placed for v in pos])
        self.assertTrue(-90 <= lo[0] and hi[0] <= 90 and -80 <= lo[1] and hi[1] <= 80 and hi[2] <= 230)


@unittest.skipUnless(os.path.exists(ISO), "needs the volume's disc image")
class StreamTest(unittest.TestCase):
    def test_survey(self):
        for name, (members, frames, closes) in STREAMS[volume.NAME].items():
            s = ccs.survey(stream(name))
            self.assertEqual((s.members, s.clean), (members, members), name)
            self.assertEqual(dict(s.lost), {}, name)
            self.assertEqual(s.frames, frames, name)
            self.assertEqual(s.unordered, 0, name)
            self.assertEqual(dict(s.closes), closes, name)
            # Frame-section chunks all read exactly their size field.
            self.assertEqual(set(s.disagree) - {0x0300, 0x0800}, set(), name)

    def test_frame_section(self):
        c = stream_member("STRCMNE", "str6100")
        tops = [(c.data[off + 8:off + 12]) for off, t, n, end in c.chunks()
                if t & 0xFFFF == 0xFF01]
        frames = [int.from_bytes(b, "little") for b in tops]
        self.assertEqual(c.frames, 398)
        self.assertEqual(frames, list(range(398)) + [0xFFFFFFFF])

    def test_cutscene_pose(self):
        # Models come from strNNNNe.tmp, frames from strNNNN.tmp.
        c = stream_member("STR1E", "str0130e")
        scene = ccsmodel.Scene(c)
        frames = ccsmodel.Scene(stream_member("STR1E", "str0130"))
        (kite,) = [m for m in ccsmodel.models(c) if scene.name(m.obj) == "MDL_ckitbod1h"]
        locals_ = scene.frame_locals(0, frames)
        nodes = scene.clump_of(scene.model_owner[kite.obj])
        self.assertTrue(all(n in locals_ for n in nodes[3:9]))   # pelvis .. toe
        placed = ccsmodel.place(scene, kite, scene.world(locals_))
        self.assertTrue(all(pos is not None for mm, pos, nrm in placed))


class StripTest(unittest.TestCase):
    def test_strips(self):
        # Two strips: 1 1 0 0 | 1 1 0. Winding alternates from each start.
        tris = ccsmodel.strip_triangles([1, 1, 0, 0, 1, 1, 0])
        self.assertEqual(tris, [(1, 0, 2), (1, 2, 3), (5, 4, 6)])


if __name__ == "__main__":
    unittest.main()
