#!/usr/bin/env python3
"""crates/piney-data's field module against tools/field.py.

tools/test_field.py checks field.py against the game's own WORLD::Init and
WORLD::Generate; this checks the Rust port against field.py, field by field: the
field_snapshot example (built with cargo, reading the tables piney-gen writes
to work/data) answers the same requests as field.py -

  - every story area that has a field (its words through areas.py, its
    IsProtectArea record, WORLD::Init's draws), and random fields across all
    eleven field types, every weather, ground and object, some with a story
    area's number and protection: the height map bit for bit, the check,
    check2, check3 and mnt grids, every hill, every object (kind, row,
    chips, world centre, height cell, whether the game zeroes its z), every
    cover tile, the dungeon entrance, the start chip and position, WORLD::Init's
    draws and the RNG state after;
  - the EE arithmetic itself (add, sub, mul, div, compare, the conversions
    and both square roots) against tools/eemu.py's f_* functions and
    field.py's sqrtf, on random and edge-case operands;
  - what the game draws (field.py's render section): every object's z
    (WORLD::GetHeight as it was placed), GetHeight at 200 points, the vertex
    normals and lit colours over the whole map, every chip's ground tile and
    every cover, under the lights of real backgrounds; the frame-0 light of
    every BG_INFO_TABLE row read from its background file; and
    CalcObjectVertexColor on random vertices.

Skipped when the executable is not extracted or cargo is missing; the light
comparison also needs DATA.BIN. PINEY_CARGO_ROOT names another cargo
workspace to build the example in (default: this repository).
"""

import json
import os
import random
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
DATA_BIN = volume.DATA
CARGO_ROOT = os.environ.get("PINEY_CARGO_ROOT", ROOT)
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(CARGO_ROOT, "target")), "release", "examples",
                       "field_snapshot")
RANDOM_CASES = 220
FPU_CASES = 4000
# Story areas the random cases borrow: the first (entrance near the start),
# ones without an entrance, and ordinary ones.
EVENTS = (14, 28, 42, 113, 1, 23, 47, 71, 100)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data",
                    "--example", "field_snapshot"], cwd=CARGO_ROOT, check=True)


def ask(requests, *args):
    """The example's answers, one parsed JSON value per request."""
    p = subprocess.run([EXAMPLE, *args], input="\n".join(requests) + "\n", capture_output=True, text=True,
                       check=True)
    out = [json.loads(line) for line in p.stdout.splitlines()]
    if len(out) != len(requests):
        raise AssertionError(f"{len(requests)} requests, {len(out)} answers")
    return out


def first_difference(a, b, path=""):
    """Where two JSON values first differ, or None."""
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                return f"{path}.{k}: only in {'rust' if k in a else 'python'}"
            d = first_difference(a[k], b[k], f"{path}.{k}")
            if d:
                return d
        return None
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return f"{path}: {len(a)} items, python {len(b)}"
        for i, (x, y) in enumerate(zip(a, b)):
            d = first_difference(x, y, f"{path}[{i}]")
            if d:
                return d
        return None
    if isinstance(a, str) and isinstance(b, str) and len(a) > 64 and a != b:
        i = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
        if path == ".map":
            cell = i // 8
            return f"{path}: height differs at x={cell // 80} y={cell % 80}: rust {a[cell * 8:cell * 8 + 8]}, " \
                   f"python {b[cell * 8:cell * 8 + 8]}"
        return f"{path}: differs at byte {i // 2}"
    return None if a == b else f"{path}: rust {a!r}, python {b!r}"


def field_mod():
    import field
    return field


def level(data, ft, kind, idx):
    """Whether the game sets the object's z to 0 instead of asking
    WORLD::GetHeight, as each setter's code does: the entrance, the lake and
    field type 1's tree rows always; sub objects for flat & 6; key, base and
    tree objects for flat & 2. (Not part of field.py's model.)"""
    if kind in (0, 7) or (kind == 4 and ft == 1):
        return True
    name = {1: "KeyObjTABLE", 2: "SubObjTABLE", 3: "BaseObjTABLE", 4: "TreeObjTABLE"}[kind]
    flat = data.tables[name][ft][idx]["flat"]
    return bool(flat & (6 if kind == 2 else 2))


def py_snapshot(data, fl):
    """field.py's Field in the example's shape."""
    import eemu
    F = eemu.f_from_py
    chip, two, cell = F(1200.0), F(2.0), F(600.0)

    def centre(x, w):
        return eemu.f_add(eemu.f_mul(chip, eemu.f_from_int(x)),
                          eemu.f_div(eemu.f_mul(chip, eemu.f_from_int(w)), two))

    def to_int(v):
        return eemu.sx(eemu.f_to_int(v), 32)

    objects = []
    for (kind, idx, x, y, w, h, clump), z in zip(fl.objects, fl.object_z):
        wx, wy = centre(x, w), centre(y, h)
        objects.append([kind, idx, x, y, w, h, clump, wx, wy, to_int(eemu.f_div(wx, cell)),
                        to_int(eemu.f_div(wy, cell)), level(data, fl.field_type, kind, idx), z])
    return {"init_draws": fl.init_draws, "map": "".join(f"{v:08x}" for v in fl.map),
            "check": bytes(fl.check).hex(), "check3": bytes(fl.check3).hex(), "mnt": bytes(fl.mnt).hex(),
            "hills": [list(h) for h in fl.hills], "objects": objects,
            "covers": [list(c) for c in fl.covers],
            "entrance": None if fl.entrance is None else list(fl.entrance),
            "dungeon_pos": None if fl.dungeon_pos is None else list(fl.dungeon_pos),
            "start": list(fl.start), "start_pos": list(fl.start_world),
            "seed": fl.rng.seed, "randcnt": fl.rng.count}


def height_points():
    """The points the example's `render` asks GetHeight for."""
    return [(float((k * 7919) % 48000) + 0.25 * (k % 4), float((k * 104729) % 48000) + 0.5 * (k % 3))
            for k in range(200)]


def py_render(data, fl, light):
    """field.py's render section in the example's `render` shape."""
    import field
    tile_scale, cover_scale = field.F(600.0), field.F(300.0)
    normals = field.vertex_normals(fl)
    colours = field.vertex_colours(fl, light)
    tiles = []
    for x in range(40):
        for y in range(40):
            t = field.tile(fl, x, y, tile_scale, colours)
            tiles.append([t["visible"], list(t["pos"]), [z for _k, z in sorted(t["z"].items())],
                          [list(c) for _k, c in sorted(t["colours"].items())]])
    covers = []
    for c in fl.covers:
        m = field.cover_mesh(fl, c, cover_scale, colours)
        covers.append([list(m["pos"]), [z for _k, z in sorted(m["z"].items())],
                       [list(c) for _k, c in sorted(m["colours"].items())]])
    return {"dirc": list(light.dirc()), "direction": list(field.light_direction(light.dirc())),
            "ambient": list(light.ambient_rgb()), "colour": list(light.colour_rgb()),
            "normals": "".join(f"{v:08x}" for n in normals for v in n),
            "colours": "".join(f"{v:08x}" for c in colours for v in c),
            "tiles": tiles, "covers": covers,
            "heights": [fl.get_height(field.F(x), field.F(y)) for x, y in height_points()]}


def request(seed, ft, weather, ground, obj, event=0, protect=0, skip_init=0):
    return f"field {seed} {ft} {weather} {ground} {obj} {event} {int(bool(protect))} {skip_init}"


def story_cases(data):
    """(event, request, field.Field) for every story area that can be typed
    and has a field, as field.from_words builds it."""
    field = field_mod()
    out = []
    for e in data.areas.events:
        if not e["has_words"]:
            continue
        try:
            words = [data.areas.find(i, e[k]).ID for i, k in enumerate(("wordA", "wordB", "wordC"))]
        except SystemExit:              # a word no slot has: the area cannot be typed
            continue
        area, fl = field.from_words(data, *words, server=e["server"])
        if area["event"] != e["code"] or area["fieldType"] == 4:
            continue
        out.append((e["code"], request(area["fieldSeed"], area["fieldType"], area["weather"], area["ground"],
                                       area["object"], area["event"], fl.protect), fl))
    return out


def random_cases():
    """(seed, type, weather, ground, object, event, protect, skip_init):
    every field type equally often, some with a story area's number."""
    rnd = random.Random(0xF1E1D)
    cases = []
    for i in range(RANDOM_CASES):
        ft = i % 11
        event = protect = 0
        if rnd.random() < 0.25:
            event, protect = rnd.choice(EVENTS), rnd.randrange(2)
        cases.append((rnd.getrandbits(32), ft, rnd.randrange(10), rnd.randrange(3), rnd.randrange(3), event,
                      protect, int(rnd.random() < 0.1)))
    return cases


def fpu_operands(rnd):
    """A pair of float patterns, biased toward the cases that matter:
    exponent 0 and 255, signs, equal and near-equal magnitudes, far-apart
    exponents, small integers."""
    def one():
        r = rnd.random()
        if r < 0.3:
            return rnd.getrandbits(32)
        if r < 0.45:
            return rnd.choice((0, 1, 254, 255, 127, 128)) << 23 | rnd.getrandbits(23) | rnd.getrandbits(1) << 31
        if r < 0.7:
            return struct.unpack("<I", struct.pack("<f", rnd.choice((1, -1)) * rnd.randrange(1, 100000)
                                                   / rnd.choice((1, 3, 7, 1024))))[0]
        return rnd.getrandbits(1) << 31 | rnd.randrange(1, 255) << 23 | rnd.getrandbits(23)
    a = one()
    r = rnd.random()
    if r < 0.2:
        b = a ^ rnd.choice((0, 0x80000000)) ^ rnd.getrandbits(rnd.randrange(1, 6))
    elif r < 0.35:
        b = (a & 0x807FFFFF) | ((((a >> 23) & 0xFF) - rnd.randrange(20, 120)) & 0xFF) << 23
    else:
        b = one()
    return a, b


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
@unittest.skipUnless(shutil.which("cargo"), "cargo not found")
class TestFieldRust(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = field_mod().Data(ELF)
        build()

    def test_story_areas_match_python(self):
        cases = story_cases(self.data)
        got = ask([req for _e, req, _fl in cases])
        bad, objects, covers = [], 0, 0
        for (event, req, fl), rust in zip(cases, got):
            py = py_snapshot(self.data, fl.generate())
            d = first_difference(rust, py)
            if d:
                bad.append(f"story area {event} ({req}): {d}")
            objects += len(py["objects"])
            covers += len(py["covers"])
        self.assertEqual(bad, [], f"{len(bad)} of {len(cases)} differ")
        self.assertGreaterEqual(len(cases), 100)
        types = {int(req.split()[2]) for _e, req, _fl in cases}
        print(f"\n{len(cases)} story areas, 0 mismatches: field types {sorted(types)}, {objects} objects, "
              f"{covers} cover tiles", file=sys.stderr)

    def test_random_fields_match_python(self):
        field = field_mod()
        cases = random_cases()
        got = ask([request(*c) for c in cases])
        bad, seen = [], {"types": set(), "weathers": set(), "objects": 0, "hills": 0, "lakes": 0,
                         "rows": 0, "story": 0, "no_entrance": 0}
        for case, rust in zip(cases, got):
            seed, ft, weather, ground, obj, event, protect, skip = case
            fl = field.Field(self.data, seed, ft, weather, ground, obj, event, protect,
                             skip_init=bool(skip)).generate()
            py = py_snapshot(self.data, fl)
            d = first_difference(rust, py)
            if d:
                bad.append(f"{case}: {d}")
                continue
            seen["types"].add(ft)
            seen["weathers"].add(weather)
            seen["objects"] += len(py["objects"])
            seen["hills"] += len(py["hills"])
            seen["lakes"] += any(o[0] == 7 for o in py["objects"])
            seen["rows"] += ft == 1
            seen["story"] += event != 0
            seen["no_entrance"] += py["entrance"] is None
        self.assertEqual(bad, [], f"{len(bad)} of {len(cases)} differ")
        self.assertEqual(seen["types"], set(range(11)))
        self.assertEqual(seen["weathers"], set(range(10)))
        for k in ("lakes", "rows", "story", "no_entrance"):
            self.assertGreater(seen[k], 5, k)
        print(f"\n{len(cases)} random fields, 0 mismatches: all 11 field types, {seen['hills']} hills, "
              f"{seen['objects']} objects, {seen['lakes']} lakes, {seen['story']} with a story area's "
              f"number ({seen['no_entrance']} without an entrance)", file=sys.stderr)

    # (seed, type, weather, ground, object, event, protect, skip): the first
    # story area, and random fields of other types, each drawn under its own
    # background's light.
    RENDER = ((1420855, 10, 0, 1, 1, 14, 0, 0), (0x00C0FFEE, 1, 3, 1, 0, 0, 0, 0),
              (987654321, 5, 5, 0, 1, 0, 0, 0), (424242, 3, 2, 2, 2, 0, 0, 0))
    # bg_p1's light, used when DATA.BIN is not there
    LIGHT = (0x8094786E, (0x420D0EBC, 0x41F00000, 0x420D0EBC), 0xDFF0F7)

    def light_for(self, ft, weather):
        import ccs
        import field
        if not os.path.exists(DATA_BIN):
            return field.Light(*self.LIGHT)
        b = self.data.backgrounds()
        row = b["rows"][ft][weather]
        c = ccs.Ccs(ccs.load(f"{DATA_BIN}::{b['ccs'][ft].replace('?', str(weather + 1))}"))
        return field.Light.from_anime(c, row["anime"], row["light"])

    def test_render_matches_python(self):
        import field
        cases = [(c, self.light_for(c[1], c[2])) for c in self.RENDER]
        got = ask([f"{request(*c).replace('field', 'render', 1)} {light.ambient} "
                   f"{' '.join(str(r) for r in light.rotation)} {light.colour}" for c, light in cases])
        tiles = covers = 0
        for (case, light), rust in zip(cases, got):
            seed, ft, weather, ground, obj, event, protect, skip = case
            fl = field.Field(self.data, seed, ft, weather, ground, obj, event, protect,
                             skip_init=bool(skip)).generate()
            py = py_render(self.data, fl, light)
            self.assertIsNone(first_difference(rust, py), case)
            tiles += sum(t[0] for t in py["tiles"])
            covers += len(py["covers"])
        print(f"\n{len(cases)} fields rendered, 0 mismatches: {tiles} visible ground tiles, {covers} covers, "
              f"{len(cases) * 6400} vertex normals and colours, {len(cases) * 200} heights", file=sys.stderr)

    @unittest.skipUnless(os.path.exists(DATA_BIN), "DATA.BIN not present")
    def test_lights_match_python(self):
        """The frame-0 light of every BGTBL row an area can reach (clamp_weather
        keeps field types 0-4 in weathers 0-3, 5 and 6 in 0-5), Rust from
        DATA.BIN against field.py."""
        import areas
        import ccs
        import field
        b = self.data.backgrounds()
        reach = {ft: sorted({areas.clamp_weather(ft, w) for w in range(10)}) for ft in range(11)}
        rows = [(ft, w, b["rows"][ft][w]) for ft in range(11) for w in reach[ft]]
        names = [b["ccs"][ft].replace("?", str(w + 1)) for ft, w, _r in rows]
        got = ask([f"light {n} {r['anime']} {r['light']}" for n, (_ft, _w, r) in zip(names, rows)], DATA_BIN)
        for name, (ft, w, r), rust in zip(names, rows, got):
            c = ccs.Ccs(ccs.load(f"{DATA_BIN}::{name}"))
            py = field.Light.from_anime(c, r["anime"], r["light"])
            self.assertEqual(rust, {"ambient": py.ambient, "rotation": list(py.rotation), "colour": py.colour},
                             (ft, w))
        self.assertEqual(len(rows), 64)

    def test_object_colours_match_python(self):
        import field
        rnd = random.Random(0x0B1)
        light = field.Light(*self.LIGHT)
        words = [rnd.getrandbits(32) for _ in range(200)]
        got = ask([f"objcol {light.ambient} {' '.join(str(r) for r in light.rotation)} {light.colour} "
                   + " ".join(str(w) for w in words)])[0]
        normals = [w.to_bytes(4, "little") for w in words[0::2]]
        colours = [w.to_bytes(4, "little") for w in words[1::2]]
        want = [int.from_bytes(bytes(c), "little") for c in field.object_colours(normals, colours, light)]
        self.assertEqual(got, want)

    def test_fpu_matches_eemu(self):
        import eemu
        field = field_mod()
        rnd = random.Random(0xEE)
        ops = {"add": eemu.f_add, "sub": eemu.f_sub, "mul": eemu.f_mul, "div": eemu.f_div,
               "cmp": eemu.f_cmp, "from_int": lambda a, _b: eemu.f_from_int(a),
               "to_int": lambda a, _b: eemu.f_to_int(a), "sqrt": lambda a, _b: eemu.f_sqrt(a),
               "sqrtf": lambda a, _b: field.sqrtf(a)}
        queries = []
        for _ in range(FPU_CASES):
            a, b = fpu_operands(rnd)
            for op in ops:
                queries.append((op, a, b))
        got = ask([f"fpu {op} {a} {b}" for op, a, b in queries])
        bad = [(op, hex(a), hex(b), rust, ops[op](a, b)) for (op, a, b), rust in zip(queries, got)
               if rust != ops[op](a, b)]
        self.assertEqual(bad[:10], [], f"{len(bad)} of {len(queries)} differ")
        print(f"\n{len(queries)} EE operations, 0 mismatches", file=sys.stderr)


if __name__ == "__main__":
    unittest.main()
