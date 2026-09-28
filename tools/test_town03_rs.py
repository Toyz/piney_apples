"""crates/piney-world's Carmina Gade (piney_world::town03, Mutation's
ROOTTOWN03 with its AIRSHIP) against Mutation's own code run in
tools/eemu.py (the Rust eemu_rs.so when present). It runs on Mutation
whatever PINEY_VOLUME says: Carmina Gade is Mutation's.

The town03_probe example builds the town and answers the requests below;
the game's side is ROOTTOWN03 built by its own constructor in the
interpreter (MUT gcmn 0x0043b980), its STATICMODELs, clumps, water ccAnms
and AIRSHIP made by their own code over town03's objects as
tools/test_world_rs.py's Pieces serves them, and run through:
  - the set-up: SetFog's arguments and ccSys.bgColor, the files it asks
    for, each STATICMODEL's row, type, model and position, the lights put
    in the group (the distant one also given to WORLD_MAN), the water's
    times after its first step, the airship as its constructor leaves it;
  - the draw: ROOTTOWN03::Draw (0x0043e400) frame after frame, through the
    airship's first leg, its turn, its wait and into the next leg, in
    town03 and in town03d (crisis): every piece in order - ccAnm::Draw with
    its time, layer and matrix (water 0, the airship), ccClump::Draw with
    its layer (the four background clumps, the crisis clumps), ccModel::Draw
    with its model, matrix and layer, DrawWithOutFog, the frame-buffer copy,
    DrawMap - and after it the airship (leg, turn, wait, t, puff, place,
    heading, way, ends), each effSmokeN's and ccSeOn3D's arguments,
    fieldrand's seed, DrawBG's scrolls and its offset in MAT_sr3dat1_2 and
    _3, the water's scroll.
What runs in Python in place of the game: tools/test_world_rs.py's (the anm
playback by tools/anim.py), the pieces' own drawing (ccModel::Draw,
ccClump::Draw, ccAnm::Draw recorded), DrawWithOutFog (recorded), the light
group (AddGrp recorded), waterUVModifi2 (recorded), effSmokeN and ccSeOn3D
(recorded), the map's sprites and the file's streams.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

os.environ["PINEY_VOLUME"] = "mutation"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_world_rs as tw  # noqa: E402
from test_world_rs import ONE, WM, fb, hexs  # noqa: E402
from volume import va as inf_va  # noqa: E402

ROOT = tw.ROOT
ISO, ELF = tw.ISO, tw.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "town03_probe")
LAYER_ACTIVE = tw.LAYER_ACTIVE
# WORLD_MAN's layers (+0x494 bgLayer[10], objLayer, obj2Layer, floorLayer,
# effLayer, charLayer, refLayer) and their priorities as GO makes them.
LAYERS = [(0x494 + 4 * k, p) for k, p in enumerate((-100, -90, -80, -70, -60, -50, -45, -40, -35, -30))] + [
    (0x4BC, 0), (0x4C0, -10), (0x4C4, -20), (0x4C8, 20), (0x4CC, 10), (0x4D0, 30)]
# The area generator's seed and count.
SEED, RANDCNT = inf_va(0x00377D20), inf_va(0x00378A80)
# ROOTTOWN03's vtable and the airship's fields (+0x04 leg, +0x0c turn, +0x10
# wait, +0x20 place, +0x30 heading, +0x40 way, +0x90 from, +0xa0 to, +0xb0 t,
# +0xb8 puff, +0xbc its ccAnm).
RT03_VTABLE = 0x00388F60


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "town03_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class TownGame(tw.Pieces):
    """ROOTTOWN03 as Mutation's code builds and draws it."""

    def __init__(self, crisis=False, seed=13):
        stem = "town03d" if crisis else "town03"
        super().__init__((stem,))
        m, sym = self.m, self.sym
        self.log = []
        rec = self.log.append
        self.streams = {}

        def ccs_adrs(mm, name, *a):
            from eemu import _cstr
            s = _cstr(mm, name).decode()
            if s not in self.streams:
                self.streams[s] = self.malloc(mm, 0x100)
            rec(("ccs", s))
            return self.streams[s]
        m.hooks[sym("GetCCSAdrs__8ccStreamFPCc")] = ccs_adrs

        def fog(mm, env, color, *a):
            rec(("fog", [mm.f[12], mm.f[13], mm.f[14], mm.f[15], color & 0xFFFFFFFF]))
            return 0
        m.hooks[sym("SetFog__9ccDrawEnvFffffUi")] = fog

        def clump_init(mm, clump, chunk, *a):
            self.names[clump] = self.names[chunk]
            return 0
        m.hooks[sym("Init__7ccClumpFP12ccClumpChunk")] = clump_init
        nop = lambda mm, *a: 0              # noqa: E731
        for name in ("SetLightEnv__5ccAnmF11ccAnmOption", "GetAmbient__5ccAnmFPf", "SetAmbient__9ccDrawEnvFPf",
                     "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff", "__ct__16ccDrawPacketCtrlFv",
                     "SetTex__8ccSpriteFPcPci", "Duplicate__5ccObjFUi",
                     "Init__10ccTexChunkFP12ccChunkIndexP12ccChunkIndexP14ccTexChunkDesc",
                     "ChangeTex__7ccModelFP10ccTexChunkP10ccTexChunk"):
            m.hooks[sym(name)] = nop
        m.hooks[sym("__ct__6ccMaskFii")] = lambda mm, a, *r: a
        self.subst = {}

        def subst(mm, anm, name, *a):
            from eemu import _cstr
            s = _cstr(mm, name).decode()
            if s not in self.subst:
                self.subst[s] = self.malloc(mm, 0x100)
                self.names[self.subst[s]] = s
            return self.subst[s]
        m.hooks[sym("GetSubstAdrsF__5ccAnmFPCcb")] = subst
        m.hooks[sym("AddGrp__10ccLightGrpFP7ccLight")] = lambda mm, g, l, *a: rec(("light", self.names[l])) or 0
        m.hooks[sym("SetLightDirection__9WORLD_MANFP14ccDistantLight")] = (
            lambda mm, w, l, *a: rec(("dirlight", self.names[l])) or 0)
        # The draws.
        del m.hooks[sym("SetActiveLayer__9WORLD_MANFi")]
        self.layer_prio = {}
        for off, prio in LAYERS:
            la = self.malloc(m, 0x40)
            m.store(WM + off, 4, la)
            self.layer_prio[la] = prio
        self.mats = {}

        def mat_subst(mm, stream, name, *a):
            from eemu import _cstr
            s = _cstr(mm, name).decode()
            if s not in self.mats:
                mat = self.mats[s] = self.malloc(mm, 0x40)
                mm.store(mat + 12, 4, self.malloc(mm, 0x40))     # its chunk: no crop
            return self.mats[s]
        m.hooks[sym("GetSubstAdrsF__8ccStreamFPCci")] = mat_subst
        self.draws = []
        lay = lambda mm: self.layer_prio.get(mm.load(LAYER_ACTIVE, 4))          # noqa: E731
        d = self.draws.append
        m.hooks[sym("Draw__7ccModelFP16ccDrawModelParam")] = lambda mm, model, param, *a: d(
            ("model", model, self.rvec(mm.load(param + 144, 4), 16), lay(mm))) or 0
        m.hooks[sym("DrawWithOutFog__11STATICMODELFv")] = lambda mm, sm, *a: d(("nofog", sm, lay(mm))) or 0
        m.hooks[sym("Draw__7ccClumpFv")] = lambda mm, cl, *a: d(("clump", cl, self.rvec(cl + 0x40, 16), lay(mm))) or 0
        m.hooks[sym("Draw__5ccAnmFv")] = lambda mm, anm, *a: d(
            ("anm", anm, self.anm[anm][1], self.rvec(anm + 0x40, 16), lay(mm))) or 0
        m.hooks[sym("waterUVModifi2__FP5ccObjPff")] = lambda mm, *a: d(("wateruv",)) or 0
        m.hooks[sym("MakePacketDrawBuffTrans__7ccLayerFP10ccTexChunk")] = lambda mm, layer, tex, *a: d(
            ("copy", self.layer_prio.get(layer))) or 0
        m.hooks[sym("DrawMap__10ROOTTOWN03Fv")] = lambda mm, *a: d(("map",)) or 0

        def smoke(mm, pos, v, life, kind, *a):
            d(("smoke", self.rvec(pos, 3), self.rvec(v, 3), mm.f[12], life, kind, mm.r[8] & 0xFFFF,
               mm.r[9] & 0xFFFF))
            return 1
        m.hooks[sym("effSmokeN__FPfPffiiUsUs")] = smoke
        m.hooks[sym("ccSeOn3D__FiPf")] = lambda mm, n, pos, *a: d(("se", n, self.rvec(pos, 3))) or 0
        m.store(tw.SAVE + 0x6772, 1, int(crisis))
        m.store(SEED, 4, seed)
        m.store(RANDCNT, 4, 0)
        for name, n in (("u$1875", 4), ("init$1876", 1), ("v$1474", 8)):
            m.mem[sym(name):sym(name) + n] = bytes(n)
        self.obj = self.malloc(m, 0x200)
        self.log.clear()
        self.call("__ct__10ROOTTOWN03Fv", self.obj)
        o = self.obj
        assert m.load(o + 0x1B0, 4) == RT03_VTABLE
        self.water = [m.load(o + 0x1BC + 4 * k, 4) for k in range(3)]
        self.model_row = {m.load(m.load(o + 0xA4 + 4 * k, 4) + 4, 4): k for k in range(34)}
        self.sm_row = {m.load(o + 0xA4 + 4 * k, 4): k for k in range(34)}
        clumps = {0x1C8 + 4 * k: ["back", k] for k in range(4)}
        clumps.update({0x74 + 4 * k: ["crisis", k] for k in range(3)})
        self.clumps = {m.load(o + off, 4): v for off, v in clumps.items() if m.load(o + off, 4)}
        self.ship = m.load(sym("town03Ship"), 4)
        self.ship_anm = m.load(self.ship + 0xBC, 4)

    def ship_state(self):
        m, s = self.m, self.ship
        return {"leg": m.load(s + 4, 4), "turn": m.load(s + 0xC, 4, True), "wait": m.load(s + 0x10, 4, True),
                "t": m.load(s + 0xB0, 4), "puff": m.load(s + 0xB8, 4, True), "pos": self.rvec(s + 0x20),
                "dirc": self.rvec(s + 0x30), "way": self.rvec(s + 0x40), "from": self.rvec(s + 0x90),
                "to": self.rvec(s + 0xA0)}

    def state(self):
        """The constructor's work as the probe's `new` reports it."""
        m, o = self.m, self.obj
        models = []
        for k in range(34):
            sm = m.load(o + 0xA4 + 4 * k, 4)
            models.append([k, m.load(sm, 4), self.names[m.load(sm + 4, 4)], self.rvec(sm + 0x10, 3)])
        fog = [e[1] for e in self.log if e[0] == "fog"]
        return {"models": models, "fog": fog[-1], "clear": m.load(tw.SYS + 0x18, 4) & 0xFFFFFF,
                "water": [self.anm[a][1] for a in self.water], "ship": self.ship_state()}

    def draw(self, eye):
        """One ROOTTOWN03::Draw: its pieces as the probe's `draw` lists them,
        with the state after it; and each piece's layer and matrix to check
        apart."""
        m, o = self.m, self.obj
        self.eye = list(eye)
        self.draws.clear()
        self.call("Draw__10ROOTTOWN03Fv", o)
        pieces, extra, events, root = [], [], [], None
        for e in self.draws:
            if e[0] == "anm" and e[1] in self.water:
                pieces.append(["water", self.water.index(e[1]), e[2]])
                extra.append((e[4], e[3]))
            elif e[0] == "anm" and e[1] == self.ship_anm:
                pieces.append(["ship", e[2]])
                extra.append((e[4], e[3]))
                root = e[3]
            elif e[0] == "copy":
                pieces.append(["copy"])
                extra.append((e[1], None))
            elif e[0] == "clump":
                pieces.append(list(self.clumps[e[1]]))
                extra.append((e[3], e[2]))
            elif e[0] == "model":
                row = self.model_row[e[1]]
                mat = e[2]
                assert mat[:12] == [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0] and mat[15] == ONE, mat
                pieces.append(["model", row, mat[12:15]])
                extra.append((e[3], None))
            elif e[0] == "nofog":
                pieces.append(["nofog", self.sm_row[e[1]]])
                extra.append((e[2], None))
            elif e[0] == "map":
                pieces.append(["map"])
                extra.append((None, None))
            elif e[0] == "smoke":
                events.append(["smoke", *e[1:]])
            elif e[0] == "se":
                events.append(["se", e[1], e[2]])
        bg = m.load(self.mats["MAT_sr3dat1_2"] + 20, 2) if "MAT_sr3dat1_2" in self.mats else 0
        if "MAT_sr3dat1_3" in self.mats:
            assert m.load(self.mats["MAT_sr3dat1_3"] + 20, 2) == bg == m.load(self.mats["MAT_sr3dat1_3"] + 22, 2)
        v = self.sym("v$1474")
        want = {"pieces": pieces, "ship": self.ship_state(), "root": root, "events": events,
                "seed": m.load(SEED, 4), "scroll": [m.load(v, 4), m.load(v + 4, 4)], "bg": bg,
                "u": m.load(self.sym("u$1875"), 4)}
        return want, extra


def eyes(rng, n):
    """Frames of a camera about the town (the water's reach is 32000)."""
    out = []
    for i in range(n):
        if i % 200 == 0:
            base = (rng.uniform(-9000, 9000), rng.uniform(-9000, 9000))
        e = (base[0] + rng.uniform(-900, 900), base[1] + rng.uniform(-900, 900), rng.uniform(100, 1500))
        if i % 500 == 499:
            e = (rng.uniform(-60000, 60000), rng.uniform(-60000, 60000), 500.0)
        out.append([fb(v) for v in e] + [ONE])
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs Mutation's extracted disc and cargo")
class TownAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_setup(self):
        """The constructor: its file, fog and background colour, the static
        models, the lights, the water, the airship."""
        for crisis in (0, 1):
            g = TownGame(bool(crisis))
            port = ask(["new %x d" % crisis])[0]
            want = g.state()
            log = list(g.log)
            self.assertEqual([e[1] for e in log if e[0] == "ccs"], ["town03d" if crisis else "town03"])
            self.assertEqual(want["fog"], [fb(3000.0), fb(15000.0), 0, fb(85.0), 0x0014140C])
            self.assertEqual(want["fog"], port["fog"])
            self.assertEqual(want["clear"], port["clear"])
            self.assertEqual(want["models"], [[r[0], r[1], r[2].strip('"'), r[3]] for r in port["models"]])
            self.assertEqual(want["water"], port["water"])
            self.assertEqual(want["ship"], port["ship"])
            lights = [e[1] for e in log if e[0] == "light"]
            self.assertEqual(lights, ["LGT_sr3lig1"] + ["LGT_omni%02d" % k for k in range(1, 12)])
            self.assertEqual([e[1] for e in log if e[0] == "dirlight"], ["LGT_sr3lig1"])
            self.assertEqual(len(port["lights"]), 12)

    def run_draws(self, crisis, frames, seed):
        g = TownGame(bool(crisis), seed)
        got = ask(["new %x %x" % (crisis, seed)] + ["draw " + hexs(*eye[:3]) for eye in frames])[1:]
        self.assertEqual(len(got), len(frames))
        seen = {}
        for i, (eye, port) in enumerate(zip(frames, got)):
            want, extra = g.draw(eye)
            self.assertEqual(want, port, "crisis %d frame %d eye %s" % (crisis, i, [f32(v) for v in eye]))
            for p, (layer, _mat) in zip(want["pieces"], extra):
                seen[p[0]] = seen.get(p[0], 0) + 1
                expect = {"water": 0, "ship": 0, "copy": 0, "model": -10, "nofog": -10}
                if p[0] == "back":
                    self.assertEqual(layer, (-100, -90, -80, -70)[p[1]], p)
                elif p[0] == "crisis":
                    self.assertEqual(layer, (-60, -50, -45)[p[1]], p)
                elif p[0] in expect:
                    self.assertEqual(layer, expect[p[0]], p)
            for e in want["events"]:
                seen[e[0]] = seen.get(e[0], 0) + 1
            seen["leg%d" % want["ship"]["leg"]] = 1
        return seen

    def test_draw(self):
        """ROOTTOWN03::Draw frame after frame, the camera about the town:
        the pieces in order, the airship through its first leg, its turn,
        its wait and into the next leg, the puffs and the sounds."""
        rng = random.Random(31)
        seen = self.run_draws(0, eyes(rng, 2700), 13)
        print("\nROOTTOWN03::Draw: 2700 frames, %s" % seen, file=sys.stderr)
        for k in ("water", "ship", "copy", "back", "model", "nofog", "map", "smoke", "se", "leg0", "leg1"):
            self.assertGreater(seen.get(k, 0), 0, k)

    def test_draw_crisis(self):
        """The same in town03d, whose DrawBG adds the three crisis clumps,
        from another fieldrand seed."""
        rng = random.Random(32)
        seen = self.run_draws(1, eyes(rng, 200), 1_234_567)
        self.assertEqual(seen.get("crisis"), 3 * 200)


if __name__ == "__main__":
    unittest.main()
