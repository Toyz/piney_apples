#!/usr/bin/env python3
"""crates/piney-world's field (area 1) against the game's own code run in
tools/eemu.py (the Rust eemu_rs.so when present).

The field_probe example builds a field as WORLD_MAN::GO(1) does
(piney_world::field_area: story area 14's, from fieldSeed 1420855) and
answers the requests below; the game's side is the same field laid out in
the interpreter's memory the way WORLD::Generate leaves it - the FIELD's
height map and hidden chips, the chip meshes, the FOBJECT and FOBJECT2
objects - with one ccModelHit per Hit chunk of field_p, decoded by the
game's own ccStream::Decode_Hit, as the file's models carry them:

  - WORLD_MAN::SetCharPosition (main 0x001a1190) in a field, for every
    field type, from a town and from the dungeon, with the party members'
    places, and the members' places in a dungeon;
  - ccLandHitCheck (gcmn 0x00571e00) with game.area 1 at random points of
    the field, on and around its objects (their hits placed as their draw
    places them) and on the bare height map (WORLD_MAN::GetHeight ->
    WORLD::GetHeight): the ground's z, the result count, the nearest
    result's attribute and point, and checkHitResultAttlibute's answer;
  - WORLD::DrawObject (gcmn 0x005a8320), then WORLD::Draw's pass over the
    entrance and key objects, for players all over the field and across
    its wrap: each object's place (FOBJECT::Draw 0x005b0e90 and
    FOBJECT2::Draw 0x005b13f0: ccTransPosW2P then P2W), whether drawn, its
    fade, its anm's steps, and the ccModelHit list left behind - order and
    each hit's matrix - with ccClump/ccAnm's HitEnable, HitDisable and
    SetHitMatrix acting on the shared per-model ccModelHits;
  - the frame: cameraMain, ccPlayer::Main over the field (its ground, the
    objects' hits, the map wrap and WORLD::AddCenter, the camera's
    avoidObstacle over the height map, cameraSet's ccTransPosFW2LW, the
    dungeon entrance's WORLD_MAN::Enter), then the object passes, frame by
    frame from Kite's arrival over scripted and random pads: everything
    tools/test_world_rs.py compares for the town, plus Enter, the centre
    and the hit list.

What runs in Python in place of the game: as tools/test_world_rs.py (the AI,
conditions, effects, the anm's playback by tools/anim.py), and here the
objects' clumps and anms themselves - their Draw, SetTransparency and
_AnimateForward are recorded (Kite's ccAnm::Draw still goes to the town
harness's hook), their HitEnable, HitDisable and SetHitMatrix act on the
ccModelHits of the object's models (the probe's association of object to Hit
chunk) as ccModelHit::HitEnable (0x00153760) and HitDisable (0x001537e0)
do, on the game's own list globals (a hook cannot call back into the
interpreter), the matrix being the object's coordinate as the game's
ccCoord::SetMatrix_PosRotZYX sets it.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import math
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import test_world_rs as tw  # noqa: E402
from test_world_rs import GAME, ONE, PLAYER, STREAM, WM, fb, hexs  # noqa: E402

ROOT = tw.ROOT
ISO, ELF = tw.ISO, tw.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "field_probe")
# Story area 14 (Bursting Passed Over Aqua Field) as WORLD_MAN keeps it.
AREA14 = (1420855, 10, 0, 1, 1, 14)
SIZE_F = 48000.0
SIZE = fb(SIZE_F)


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "field_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class FieldGame(tw.Field):
    """The game's field code in eemu over a generated field."""

    def __init__(self, info):
        # The town harness's machine, globals and hooks; its hit list
        # (field_p's first Hit chunk at the identity) emptied again.
        super().__init__("field_p")
        m, sym = self.m, self.sym
        self.call("initHitCheck1__Fv")
        del m.hooks[sym("SetMatrix_PosRotZYX__7ccCoordFPfPf")]
        self.info = info
        self.recorded = []
        self.decode_hits()
        self.lay_out()
        self.field_hooks()

    # --- the field's ccModelHits --------------------------------------------
    def decode_hits(self):
        """One ccModelHit per Hit chunk of field_p (ccModel +0x3c), each
        decoded by ccStream::Decode_Hit, in the probe's order."""
        m, c = self.m, self.town
        by_object = {}
        for off, t, n, end in c.chunks():
            if t is None or t & 0xFFFF != 0x0B00:
                continue
            obj = struct.unpack_from("<I", c.data, off + 8)[0]
            self.data, self.cur_read = c.data, off + 8
            m.store(STREAM + 0x84, 4, 0)
            self.call("Decode_Hit__8ccStreamFv", STREAM)
            by_object[obj] = m.load(m.load(STREAM + 0x84, 4) + 16, 4)
        self.mh = []
        for obj, parent in self.info["hits"]:
            mh = self.malloc(m, 0xA0)
            self.call("__ct__10ccModelHitFv", mh)
            m.store(mh + 12, 4, by_object[obj])
            self.vec(mh + 0x10, [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE])
            self.mh.append(mh)
        self.mh_index = {a: i for i, a in enumerate(self.mh)}

    # --- WORLD and FIELD as Generate leaves them -----------------------------
    def lay_out(self):
        m = self.m
        info = self.info
        self.world = w = self.malloc(m, 0x61E0)
        self.fieldmem = fd = self.malloc(m, 0xCFD40)
        m.store(w + 0x6168, 4, fd)
        for k, v in enumerate(info["map"]):
            m.store(fd + 0xAF02C + 4 * k, 4, v)
        for k, v in enumerate(info["check3"]):
            m.store(fd + 0xCF0B0 + k, 1, v)
        for off, v in ((0xAF01C, 0), (0xAF020, 0), (0xAF024, SIZE), (0xAF028, SIZE)):
            m.store(fd + off, 4, v)
        meshes = self.malloc(m, 0x60 * 1600)
        for x in range(40):
            for y in range(40):
                a = meshes + 0x60 * (40 * x + y)
                m.store(a + 0x44, 4, x)
                m.store(a + 0x48, 4, y)
                m.store(w + 0x590 + 4 * (40 * x + y), 4, a)
        self.objs = []          # (struct address, clump or anm address, FOBJECT2)
        self.owner = {}         # clump/anm -> object index
        n2 = 0
        for k, (kind, two, cx, cy, wp, anm, hits) in enumerate(info["objects"]):
            if two:
                o = self.malloc(m, 0x80)
                m.store(o, 4, kind)
                self.vec(o + 0x40, wp)
                m.store(w + 0x5090 + 4 * n2, 4, o)
                n2 += 1
            else:
                o = self.malloc(m, 0x50)
                self.vec(o + 0x30, wp)
                m.store(w + 0x1E90 + 4 * (40 * cx + cy), 4, o)
            if anm:
                a = self.malloc(m, 0x110)
                m.store(a + 0xAC, 4, 1)
                m.store(a + 0x9C, 2, 256)
                m.store(o + (0x78 if two else 0x4C), 4, a)
            else:
                a = self.malloc(m, 0xB0)
                m.store(o + (0x70 if two else 0x48), 4, a)
            self.owner[a] = k
            self.objs.append((o, a, bool(two)))
        m.store(w + 0x18, 4, n2)

    def field_hooks(self):
        m, sym = self.m, self.sym

        def rec(kind, before):
            def h(mm, a, *rest):
                if a in self.owner:
                    self.recorded.append((kind, self.owner[a], mm.f[12]))
                    return 0
                # Anyone else's (Kite's ccAnm::Draw): the town harness's hook.
                return before(mm, a, *rest) if before else 0
            return h
        for name, kind in (("SetTransparency__7ccClumpFf", "alpha"), ("Draw__7ccClumpFv", "draw"),
                           ("Draw__5ccAnmFv", "draw"), ("_AnimateForward__5ccAnmFUi", "fwd")):
            m.hooks[sym(name)] = rec(kind, m.hooks.get(sym(name)))
        # Kite's anm (ccPlayer::Main) goes through tools/anim.py as before.
        forward = tw_forward = m.hooks.get(sym("_AnimateForward__5ccAnmFUi"))

        def fwd(mm, anm, spd, *rest):
            if anm in self.owner:
                self.recorded.append(("fwd", self.owner[anm], 0))
                return 0
            return self.kite_forward(mm, anm, spd)
        self.kite_forward = self.town_forward
        m.hooks[sym("_AnimateForward__5ccAnmFUi")] = fwd
        del forward, tw_forward

        def hits_of(a):
            return self.info["objects"][self.owner[a]][6]

        # A hook cannot call back into the interpreter, so the list's two
        # operations are done here as ccModelHit::HitEnable (0x00153760) and
        # HitDisable (0x001537e0) do them, on the game's own globals.
        top, tail, num = self.sym("ccModelHitTop"), self.sym("ccModelHitTail"), self.sym("ccModelHitEntryNum")

        def mh_enable(mh):
            m.store(mh + 8, 4, 0)
            if m.load(mh, 4):
                return
            m.store(mh, 4, 1)
            m.store(mh + 4, 4, 0)
            if not m.load(top, 4):
                m.store(top, 4, mh)
            if m.load(tail, 4):
                m.store(m.load(tail, 4) + 4, 4, mh)
            m.store(tail, 4, mh)
            m.store(num, 4, m.load(num, 4) + 1)

        def mh_disable(mh):
            if m.load(mh, 4) != 1:
                return
            m.store(mh + 0x96, 2, 0)
            m.store(mh, 4, 0)
            if m.load(top, 4) == mh:
                m.store(top, 4, m.load(mh + 4, 4))
                if not m.load(mh + 4, 4):
                    m.store(tail, 4, 0)
            else:
                prev = m.load(top, 4)
                while m.load(prev + 4, 4) != mh:
                    prev = m.load(prev + 4, 4)
                m.store(prev + 4, 4, m.load(mh + 4, 4))
                if m.load(tail, 4) == mh:
                    m.store(tail, 4, prev)
            m.store(mh + 4, 4, 0)
            m.store(num, 4, m.load(num, 4) - 1)
            m.store(mh + 0x96, 2, 0)

        def enable(mm, a, *rest):
            for h in hits_of(a):
                mh_enable(self.mh[h])
            return 0

        def disable(mm, a, *rest):
            for h in hits_of(a):
                mh_disable(self.mh[h])
            return 0

        def set_matrix(mm, a, *rest):
            # The coordinate has no parent: its world matrix is its local
            # one (+0x40), which SetMatrix_PosRotZYX wrote.
            local = self.rvec(a + 0x40, 16)
            for h in hits_of(a):
                self.vec(self.mh[h] + 0x10, local)
            return 0
        for cls in ("7ccClump", "5ccAnm"):
            m.hooks[sym("HitEnable__%sFv" % cls)] = enable
            m.hooks[sym("HitDisable__%sFv" % cls)] = disable
        m.hooks[sym("SetHitMatrix__7ccClumpFP7ccCoord")] = set_matrix
        m.hooks[sym("SetHitMatrix__5ccAnmFv")] = set_matrix
        self.entered = []
        m.hooks[sym("Enter__9WORLD_MANFPf")] = lambda mm, *a: self.entered.append(1) or 0

    def clear_hits(self):
        """initHitCheck1 (the list's globals emptied) and every hit off it,
        as a new set-up leaves them: between runs a hit still marked on
        the list would not be put back."""
        for mh in self.mh:
            self.m.store(mh, 4, 0)
            self.m.store(mh + 4, 4, 0)
        self.call("initHitCheck1__Fv")

    def town_forward(self, mm, anm, spd):
        f = self.anims[self.cur].forward(self.time, spd & 0xFFFFFFFF)
        self.time = f.time
        return int(f.ended)

    # --- the area's globals ---------------------------------------------------
    def area(self, center=None):
        m = self.m
        m.store(GAME + 0x14, 4, 1)
        m.store(GAME + 0x18, 4, 0)
        m.store(GAME + 0x24, 4, 14)
        m.store(WM + 0x8, 4, 1)
        m.store(WM + 0x10, 4, 10)
        m.store(WM + 0x124, 4, 0)
        m.store(WM + 0x15C, 4, self.info["def_se"])
        for off, v in ((0x420, 0), (0x424, 0), (0x428, SIZE), (0x42C, SIZE)):
            m.store(WM + off, 4, v)
        m.store(WM + 0x434, 4, self.world)
        if center is not None:
            m.store(self.world + 0x6140, 4, center[0])
            m.store(self.world + 0x6144, 4, center[1])
            m.store(WM + 0x80, 4, center[0])
            m.store(WM + 0x84, 4, center[1])

    def hit_list(self):
        """The ccModelHit list: (hit index, translation) in order."""
        m = self.m
        out = []
        a = m.load(self.sym("ccModelHitTop"), 4)
        while a:
            out.append([self.mh_index[a], self.rvec(a + 0x10 + 0x30)])
            a = m.load(a + 4, 4)
        return out

    def draw_passes(self):
        """ccThFieldDisp's object passes: WORLD::DrawObject, then WORLD::Draw's
        loop over fobj2 calling FOBJECT2::Draw for types 0 and 1 (0x005a9df4),
        then DrawMesh's for type 6 (0x005a85a0)."""
        m = self.m
        self.call("DrawObject__5WORLDFv", self.world)
        n = m.load(self.world + 0x18, 4)
        for types in ((0, 1), (6,)):
            for i in range(n):
                o = m.load(self.world + 0x5090 + 4 * i, 4)
                if m.load(o, 4) in types:
                    self.call("Draw__8FOBJECT2Fv", o)

    def object_state(self, k):
        m = self.m
        o, a, two = self.objs[k]
        pos = self.rvec(o + (0x20 if two else 0x10))
        return pos, (m.load(o + 0x14, 4) if two else None)

    def start(self, pos, dircz, scheme, mode, seed):
        super().start(pos, dircz, scheme, mode, seed)
        self.area(center=pos[:2])
        self.entered = []

    def frame(self):
        self.events = []
        self.entered = []
        self.call("cameraMain__Fv")
        self.call("Main__8ccPlayerFv", PLAYER)
        s = self.state()
        self.draw_passes()
        s["enter"] = int(bool(self.entered))
        s["center"] = [self.m.load(self.world + 0x6140, 4), self.m.load(self.world + 0x6144, 4)]
        s["list"] = self.hit_list()
        return s


def qualifies(att):
    """checkHitResultAttlibute takes a result's ground-type nibbles unless
    it is shaded (0x40000) or of type 0xe000e0; with none that qualify it
    takes them from a stale register ($a2: an address inside the hit data,
    which depends on where the game's heap put it), so only the other bits
    are compared then."""
    return att & 0x40000 != 0x40000 and att & 0xF0F0F0 != 0xE000E0


def attribute_key(attribute, nearest_att):
    return attribute if qualifies(nearest_att) else attribute & 0xFF0F0F0F


def field_info():
    return ask(["new " + hexs(*AREA14)])[0]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class FieldAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.info = field_info()

    def test_char_position(self):
        """WORLD_MAN::SetCharPosition's field cases: from a town the start,
        back from the dungeon beside its entrance by field type, and the
        party members' places round the leader (ccGetStartPositions asks
        for all three); in a dungeon, the members round its position."""
        f = FieldGame(self.info)
        m = f.m
        rng = random.Random(3)
        cases = []
        for ft in range(11):
            for prev in (0, 2, 1):
                s = [fb(rng.uniform(0, 48000)), fb(rng.uniform(0, 48000)), fb(rng.choice([0.0, 12.5]))]
                d = [fb(rng.uniform(0, 48000)), fb(rng.uniform(0, 48000)), fb(rng.choice([0.0, -3.25]))]
                cases.append((prev, ft, *s, *d))
        got = ask(["new " + hexs(*AREA14)] + ["charpos " + hexs(*c) for c in cases])[1:]
        p = tw.ARGS
        bad = 0
        for c, g in zip(cases, got):
            f.area()
            m.store(GAME + 0x18, 4, c[0])
            m.store(WM + 0x10, 4, c[1])
            f.vec(WM + 0x450, [c[2], c[3], c[4], ONE])
            f.vec(WM + 0x460, [c[5], c[6], c[7], ONE])
            # ccGetStartPositions(1, 2, 3): every member's place asked for.
            members = [tw.SCRATCH + 0xB000 + 0x40 * k for k in range(3)]
            for k, a in enumerate(members):
                m.store(a, 4, fb(k + 1.0))
            f.call("SetCharPosition__9WORLD_MANFPfPfPfPf", WM, p, *members)
            want = [m.load(p + 4 * i, 4) for i in (1, 2, 3, 4)]
            want.append([[m.load(a + 4 * i, 4) for i in (1, 2, 3, 4)] for a in members])
            if want != g:
                bad += 1
                if bad < 5:
                    print("charpos", c, [hex(v) for v in want], [hex(v) for v in g])
        self.assertEqual(bad, 0)
        # In a dungeon: the members round WORLD_MAN.position by its facing.
        cases = [(fb(rng.uniform(0, 48000)), fb(rng.uniform(0, 48000)), fb(rng.choice([0.0, 100.0])),
                  fb(rng.uniform(-3.2, 3.2))) for _ in range(40)]
        got = ask(["new " + hexs(*AREA14)] + ["dgpos " + hexs(*c) for c in cases])[1:]
        for c, g in zip(cases, got):
            f.area()
            m.store(WM + 0x8, 4, 2)
            f.vec(WM + 0x20, list(c))
            members = [tw.SCRATCH + 0xB000 + 0x40 * k for k in range(3)]
            for k, a in enumerate(members):
                m.store(a, 4, fb(k + 1.0))
            f.call("SetCharPosition__9WORLD_MANFPfPfPfPf", WM, p, *members)
            want = [[m.load(a + 4 * i, 4) for i in (1, 2, 3, 4)] for a in members]
            if want != g:
                bad += 1
                if bad < 5:
                    print("dgpos", c, want, g)
        self.assertEqual(bad, 0)

    def test_land(self):
        """ccLandHitCheck over the field: every object's hits placed where
        its wp is, then points on them, beside them and across the bare
        ground, and after some the list emptied again."""
        f = FieldGame(self.info)
        m = f.m
        rng = random.Random(9)
        objs = self.info["objects"]
        lines = ["new " + hexs(*AREA14)]
        f.area()
        placed = [k for k, o in enumerate(objs) if o[6]]
        for k in placed:
            lines.append("place %x" % k)
        # A hit belongs to a model, so it stands at the last object of that
        # model placed: the points go round those.
        last = sorted(set({h: k for k in placed for h in objs[k][6]}.values()))
        pts = []
        for i in range(1500):
            if i % 3 == 0:
                x, y = rng.uniform(0, 48000), rng.uniform(0, 48000)
                z = rng.choice([0.0, 50.0, 200.0, rng.uniform(-100, 900)])
            else:
                # On and round an object: from above its top down to its foot.
                o = objs[rng.choice(last)]
                r = rng.choice([400, 900, 1500])
                x, y = f32(o[4][0]) + rng.uniform(-r, r), f32(o[4][1]) + rng.uniform(-r, r)
                z = f32(o[4][2]) + rng.choice([rng.uniform(0, 1500), rng.uniform(100, 600), 0.0])
            pts.append((fb(x), fb(y), fb(z)))
        lines += ["land " + hexs(*p) for p in pts]
        got = ask(lines)
        got = got[1 + len(placed):]
        # The game's list as the probe's places left it.
        for k in placed:
            o, a, two = f.objs[k]
            f.vec(a + 0x40, [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0] + objs[k][4])
            f.m.hooks[f.sym("SetHitMatrix__7ccClumpFP7ccCoord")](m, a)
            f.m.hooks[f.sym("HitEnable__7ccClumpFv")](m, a)
        bad = hits = 0
        for p, g in zip(pts, got):
            f.vec(tw.ARGS, list(p) + [ONE])
            f.call("ccLandHitCheck__FPfUi", tw.ARGS, 0x20000001)
            z = m.f[0]
            num = m.load(f.sym("hitResultNum"), 4)
            near = f.sym("hitResultNearest")
            att = m.load(near + 0x48, 4)
            cp = f.rvec(near)
            attribute = f.call("checkHitResultAttlibute__Fv") & 0xFFFFFFFF
            want = {"z": z, "num": num, "att": att, "cp": cp, "attribute": attribute_key(attribute, att)}
            g["attribute"] = attribute_key(g["attribute"], g["att"])
            hits += att != self.info["def_se"] | 0x20000F0F
            if want != g:
                bad += 1
                if bad < 5:
                    print("land", [hex(v) for v in p], want, g)
        self.assertEqual(bad, 0)
        print("\nccLandHitCheck: %d points, %d on an object's hit, 0 mismatches" % (len(pts), hits),
              file=sys.stderr)
        # The hit meshes are mostly walls: few points land on one (the
        # entrance's floor and steps, the tops of plinths and rocks).
        self.assertGreater(hits, 20)

    def test_draw_object(self):
        """DrawObject and the entrance and key pass for players all over the
        field and across its edges, one after another."""
        f = FieldGame(self.info)
        m = f.m
        rng = random.Random(4)
        pts = []
        for i in range(300):
            if i % 4 == 0:
                x, y = rng.choice([rng.uniform(0, 1500), rng.uniform(46500, 48000)]), rng.uniform(0, 48000)
            else:
                x, y = rng.uniform(0, 48000), rng.uniform(0, 48000)
            pts.append((fb(x), fb(y), fb(rng.uniform(0, 300))))
        got = ask(["new " + hexs(*AREA14)] + ["step " + hexs(*p) for p in pts])[1:]
        f.area()
        f.clear_hits()
        k = next(i for i, o in enumerate(self.info["objects"]) if o[0] == 0)
        o, a, two = f.objs[k]
        # SetDungeonEnter's FOBJECT2::HitEnable at the entrance's wp.
        f.call("HitEnable__8FOBJECT2Fv", o)
        bad = 0
        for p, g in zip(pts, got):
            f.area(center=p[:2])
            f.vec(PLAYER + 0x40, list(p) + [ONE])
            f.recorded = []
            f.draw_passes()
            fwd = {}
            alpha = {}
            drawn = []
            for kind, obj, val in f.recorded:
                if kind == "fwd":
                    fwd[obj] = fwd.get(obj, 0) + 1
                elif kind == "alpha":
                    alpha[obj] = val
                elif kind == "draw":
                    drawn.append(obj)
            want_list = f.hit_list()
            if want_list != g["list"] or drawn != g["drawn"]:
                bad += 1
                if bad < 5:
                    print("draw", [hex(v) for v in p], "game", drawn, want_list[:6], "port", g["drawn"],
                          g["list"][:6])
                continue
            for obj in drawn:
                pos, disp = f.object_state(obj)
                go = g["objects"][obj]
                ga = f.m.load(f.objs[obj][1] + 0x88, 4) if self.info["objects"][obj][5] else alpha.get(obj)
                if pos != go[0] or ga != go[2] or (disp is not None and disp != go[3]):
                    bad += 1
                    if bad < 5:
                        print("object", obj, [hex(v) for v in p], pos, go[0], hex(ga or 0), hex(go[2]))
        self.assertEqual(bad, 0)

    def run_frames(self, f, pos, dircz, scheme, mode, seed, pads, label):
        lines = ["new " + hexs(*AREA14), "start " + hexs(*pos, dircz, scheme, mode, seed)]
        for d, p, pl, dl, pr, dr, pw in pads:
            lines.append("pad " + hexs(d, p, pl, dl, pr, dr, *pw))
        got = ask(lines)[2:]
        f.clear_hits()
        k = next(i for i, o in enumerate(self.info["objects"]) if o[0] == 0)
        f.call("HitEnable__8FOBJECT2Fv", f.objs[k][0])
        f.start(pos, dircz, scheme, mode, seed)
        stats = {"moved": 0, "acts": set(), "enter": 0, "listed": 0, "wrapped": 0}
        last = [f32(v) for v in pos[:2]]
        for i, ((d, p, pl, dl, pr, dr, pw), g) in enumerate(zip(pads, got)):
            f.pad(d, p, pl, dl, pr, dr, pw)
            want = f.frame()
            now = [f32(v) for v in want["pos"][:2]]
            stats["wrapped"] += any(abs(a - b) > SIZE_F / 2 for a, b in zip(now, last))
            last = now
            stats["acts"].add(want["act"])
            stats["moved"] += want["move_flag"]
            stats["enter"] += want["enter"]
            stats["listed"] = max(stats["listed"], len(want["list"]))
            if not g.pop("qualified"):
                want["attribute"] &= 0xFF0F0F0F
                g["attribute"] &= 0xFF0F0F0F
            diff = [k for k in want if want[k] != g[k]]
            if diff:
                self.fail("%s frame %d: %s\n game %s\n port %s" % (
                    label, i, diff, {k: want[k] for k in diff}, {k: g[k] for k in diff}))
        return stats

    def test_frames(self):
        """Kite arriving at the field's start and walking it: to the dungeon
        entrance (Enter), round its objects, and random pads."""
        f = FieldGame(self.info)
        rng = random.Random(12)
        start = (fb(24600.0), fb(24600.0), 0)
        s = self.run_frames(f, start, 0, 0, 3, 1, tw.script_pads("walk", 320, rng), "walk")
        self.assertIn(5, s["acts"])
        # Toward the entrance at (17400, 24000), the camera looking along -y:
        # the stick right and a touch up (a stick exactly across reads as
        # straight down: SetAnalogStick tests y before atan2f). Enter from
        # about frame 370, then down its steps.
        pads = []
        for i in range(420):
            lx, ly = (128, 128) if i < 80 else (255, 120)
            dl, pl = tw.stick(lx, ly)
            pads.append((0, 0, pl, dl, 0, 0, [0] * 12))
        s = self.run_frames(f, start, 0, 0, 3, 2, pads, "entrance")
        self.assertGreater(s["enter"], 0)
        for k in range(3):
            pads = tw.script_pads("walk", 60, rng) + tw.script_pads("random", 300, rng)
            x, y = rng.uniform(2000, 46000), rng.uniform(2000, 46000)
            self.run_frames(f, (fb(x), fb(y), 0), fb(rng.uniform(-3, 3)), rng.randrange(4), 3,
                            rng.randrange(1000), pads, "random %d" % k)

    def test_frames_wrap(self):
        """Running off the field's edge onto its other side: W2MPos, the
        re-landing, WORLD::AddCenter's wrap, the objects over the edge."""
        f = FieldGame(self.info)
        # The camera looks along -y: left and a touch down runs +x, right and
        # a touch up -x (straight across reads as down), down +y, up -y.
        for k, (x, y, lx, ly) in enumerate(((47700.0, 20000.0, 0, 136), (300.0, 30000.0, 255, 120),
                                            (20000.0, 47700.0, 128, 255), (30000.0, 250.0, 128, 0))):
            pads = []
            for i in range(200):
                dl, pl = tw.stick(lx, ly) if i >= 80 else (0, 0)
                pads.append((0, 0, pl, dl, 0, 0, [0] * 12))
            s = self.run_frames(f, (fb(x), fb(y), 0), 0, 0, 3, 5 + k, pads, "wrap %d" % k)
            self.assertGreater(s["moved"], 50)
            self.assertGreater(s["wrapped"], 0, "wrap %d never crossed the edge" % k)


if __name__ == "__main__":
    unittest.main()
