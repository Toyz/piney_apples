#!/usr/bin/env python3
"""Field terrain - how WORLD::Generate builds a field area from fieldSeed.

    tools/field.py gen  ELF A B C [--server N] [--volume N] [--png FILE]
                        [--objects]           the field three words make: chip map,
                                              objects, heightmap PNG
    tools/field.py seed ELF SEED FIELDTYPE WEATHER GROUND OBJECT [--event N]
                        [--png FILE] [--objects]
                                              a field from raw inputs

The field is an 80 x 80 grid of heights (FIELD.map, one cell 600 units) under
a 40 x 40 grid of "chips" (1200 units) that objects occupy. WORLD_MAN::GO
(0x0019f8e0) seeds the RNG with fieldSeed, then WORLD::Init (gcmn 0x005a4cd0)
draws from it for weather particles before WORLD::Generate (0x005a6da0):

  FIELD::MakeField(0)    0x005adc60  every height fieldrand(64), white noise
  (story areas)                      flatten the centre, start there
  WORLD::MakeHill        0x005a6b00  1 large hill (20-29 cells) and 3/5/8 small
                                     ones (10-19) by `ground`; each a 5 x 5 key
                                     grid (fieldrand 128 on the rim, 768 inside)
                                     through FRACTAL2's natural spline, written
                                     over the noise where no hill is yet
  WORLD::SetDungeonEnter 0x005abd20  the dungeon entrance
  WORLD::SetLake         0x005ac7e0  a lake for field types 2, 3, 8, 9, 10
  FIELD::InitQuad        0x005aee30  quads from the heights (CheckAreaHeight
                                     reads these, so later flattening is not
                                     seen by the objects placed after it)
  WORLD::SetKeyObject    0x005ac120  20 key, 30 sub, 35 base, 35 tree objects
  ...SubObject, BaseObject, TreeObject(_B)   (fewer for types 7 and 1, times
                                     0.8/0.9/1.0 by `object`); each at random
                                     chips clear of others, 300 failures at most
  WORLD::SetCover        0x005aba50  ground cover on every free chip
  (start position)                   the first free 2 x 2 chips with fieldrand(100) >= 96

Objects flagged 2 flatten 3w x 3h height cells from twice their chip
position (FIELD::Flat, 0x005ae810). Heights are EE floats and are computed
with tools/eemu.py's f_* arithmetic, the same model the interpreter uses.

The render section is what the game draws of it (WORLD::DrawMesh 0x005a8570):

  get_height             WORLD::GetHeight (0x005aa520), each object's z as it is
                         placed: a segment cast into the cell's two triangles
  tile                   FIELD_MESH::RelocateMesh / SetMESH2: per chip the ground
                         model (BaseMeshName) at the chip's centre, its listed
                         vertices' z and RGB rewritten from the cells under
                         them; none on a hidden chip
  cover_mesh             FCOVER::SetPosition / FIELD::SetSmallMESH: a cover at
                         the chip's centre +-300 by quadrant, z 2.5, rewritten
                         likewise from its cell
  vertex_normals,        InitQuad's faces, CalcQuadVertexNormal, CalcVertexColor:
  vertex_colours         FIELD.color, 128 lit by the background's light (Light,
                         frame 0 of its animation)
  object_colours         CalcObjectVertexColor: the same lighting on the object
                         models' vertex colours, done in WORLD::Init
  Data.backgrounds       the background files, clumps, lights and fog
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import areas  # noqa: E402
import eemu  # noqa: E402
from eemu import f_add, f_sub, f_mul, f_div, f_from_int, f_to_int, f_cmp  # noqa: E402
from image import Program  # noqa: E402

N = 80              # height cells per side
C = 40              # chips per side
F = eemu.f_from_py
ZERO, ONE, TWO = 0, F(1.0), F(2.0)
TABLES = ("EnterObjTABLE", "LakeObjTABLE", "KeyObjTABLE", "SubObjTABLE", "BaseObjTABLE",
          "TreeObjTABLE")
# Field types that get a lake, and the story areas that get no dungeon
# entrance (WORLD::Generate 0x005a7024, 0x005a7140); these and the other
# constants below are the same in all four volumes' code.
LAKE_FIELDS = (10, 2, 3, 9, 8)
NO_ENTRANCE = (113, 112, 111, 110, 109, 82, 81, 80, 79, 78, 57, 56, 55, 54, 53, 42, 41, 40, 39, 28)
OBJECT_NAMES = {0: "entrance", 1: "key", 2: "sub", 3: "base", 4: "tree", 7: "lake"}
# fieldrand calls WORLD::Init makes before Generate, by field type: weather
# particles (100 SNOW of 7 calls, or 200 of 8 when GetWeather() is 5) for the
# snow fields 5 and 6, 16 SNOW of 7 for type 0, smoke (1 + 5 x 4) for 2, 3,
# 5 and 6. Measured by running WORLD::Init in tools/eemu.py for every field
# type and weather, the same on all four volumes; tools/test_field.py
# re-derives it.
INIT_SNOW = {0: 112, 5: 700, 6: 700}
INIT_SMOKE = (2, 3, 5, 6)

# What the game draws (see "render" below).
CELL_UNITS, CHIP_UNITS = F(600.0), F(1200.0)
MINUS_ONE = F(-1.0)
FIXED_ONE = F(4096.0)       # a model's s16 positions are value * 4096 / vertexScale
# WORLD::GetHeight: a segment from z 2500 down to -1200; -500 on a hidden
# chip. Its two triangles per cell, in cell-local x, y (the constant at gcmn
# 0x00658a30): corners take the heights of cells (0, 1), (0, 0), (1, 1) and
# (1, 0), (1, 1), (0, 0) from the cell.
RAY_TOP, RAY_BOTTOM, HIDDEN_HEIGHT = F(2500.0), F(-1200.0), F(-500.0)
GROUND_TRIANGLES = (((0, F(600.0)), (0, 0), (F(600.0), F(600.0))),
                    ((F(600.0), 0), (F(600.0), F(600.0)), (0, 0)))
EDGE_SLACK = 0xBA83126F     # collisionLP's -0.001
LIGHT_VECTOR = (0, 0, MINUS_ONE, 0)   # lightVector (main 0x002fb100)
COL255 = 0x3B808081         # col255to1Vector (main 0x002f7370), 1/255
BASE_COLOUR = F(128.0)      # FIELD's constructor starts every colour at 128
COLOUR_MAX = F(255.0)
NORMAL_UNIT = F(64.0)       # a model's s8 normals are value * 64


def init_draws(field_type, weather):
    n = INIT_SNOW.get(field_type, 0)
    if field_type in (5, 6) and weather >= 4:
        n = 1600            # GetWeather() == 5: 200 heavier flakes
    if field_type in INIT_SMOKE:
        n += 21
    return n


def sqrtf(v):
    """__ieee754_sqrtf (main 0x00124ab0): newlib's bit-by-bit square root,
    rounding to nearest; exponent 0 comes back unchanged."""
    if not v & 0x7F800000:
        return v
    if v & 0x80000000:
        return eemu.FMAX            # NaN in IEEE; the EE's 0/0 gives +Fmax
    ix = (v >> 23) - 127
    m = ((v & 0x7FFFFF) | 0x800000) << (ix & 1)
    ix >>= 1
    m <<= 1
    q = s = 0
    r = 0x1000000
    while r:
        t = s + r
        if not m < t:
            m -= t
            s = t + r
            q += r
        m <<= 1
        r >>= 1
    if m:
        q += q & 1
    return ((q >> 1) + 0x3F000000 + (ix << 23)) & 0xFFFFFFFF


class Data:
    def __init__(self, elf_path):
        self.areas = areas.Data(elf_path)
        p = self.p = Program(elf_path, "gcmn")

        def cstr(va):
            return p.cstr(va).decode() if va else None
        self.tables = {}
        for name in TABLES:
            base = p.symbol_named(name).value
            per_type = []
            for ft in range(11):
                ptr, num = struct.unpack("<Ii", p.read(base + 8 * ft, 8))
                rows = []
                for k in range(num):
                    a, c, t, w, h, rot, flat = struct.unpack("<IIiiiii", p.read(ptr + 28 * k, 28))
                    rows.append({"anm": cstr(a), "clump": cstr(c), "type": t, "w": w, "h": h,
                                 "rot": rot, "flat": flat})
                per_type.append(rows)
            self.tables[name] = per_type
        base = p.symbol_named("SmallMeshName").value
        self.small_mesh = []
        for ft in range(11):
            ptr, num = struct.unpack("<Ii", p.read(base + 8 * ft, 8))
            self.small_mesh.append([cstr(p.u32(ptr + 4 * k)) for k in range(num)])
        # ccGetDist's square root: a call of newlib's sqrtf, which rounds to
        # nearest (INF, MUT), or the FPU's sqrt.s inline (OUT 0x001e6ec0, QUA
        # 0x001ee540), which eemu truncates like every EE FPU result.
        f = p.symbol_named("ccGetDist__FPfPf")
        words = struct.unpack(f"<{f.size // 4}I", p.read(f.value, f.size // 4 * 4))
        self.sqrt = (eemu.f_sqrt if any(w & 0xFFE0003F == 0x46000004 for w in words)
                     else sqrtf)

    def backgrounds(self):
        """What WORLD::Init loads for the background and WORLD::DrawBG draws,
        read lazily (only Infection's symbols are known to name all of it):

          backccs[ft]      the CCS file, its '?' replaced by '1' + weather
                           (WORLD.bgNum, which is the area's weather);
                           backccs2 when WORLD.isHacked is not 2
          BGTBL[ft][w]     BG_INFO_TABLE rows {p, clumpname0-3, lgname,
                           anmname, fr, fg, fb, near, far, per}: the sky,
                           two cloud layers and the mountains, the light,
                           the animation that sets it and the ambient, and
                           the fog (colour 0-255, near, far, percent);
                           BGTBL2 when isHacked is not 2
          BgMatName[ft][w] the material whose V DrawBG scrolls (BgMatName2)
          aurora           field type 6's extra clump per weather, from the
                           name table WORLD::Init copies to its stack
        """
        if getattr(self, "_backgrounds", None) is not None:
            return self._backgrounds
        p = self.p

        def cstr(va):
            return p.cstr(va).decode() if va else None

        def strings(name):
            sym = p.symbol_named(name)
            return [cstr(p.u32(sym.value + 4 * i)) for i in range(sym.size // 4)]

        def per_type(name, row):
            base = p.symbol_named(name).value
            out = []
            for ft in range(11):
                ptr, num = struct.unpack("<Ii", p.read(base + 8 * ft, 8))
                out.append([row(ptr, k) for k in range(num)])
            return out

        def bg_row(ptr, k):
            w = struct.unpack("<7I6I", p.read(ptr + 52 * k, 52))
            return {"p": cstr(w[0]), "clumps": [cstr(v) for v in w[1:5]], "light": cstr(w[5]),
                    "anime": cstr(w[6]), "fog": list(w[7:13])}

        out = {"ccs": strings("backccs"), "ccs2": strings("backccs2"),
               "rows": per_type("BGTBL", bg_row), "rows2": per_type("BGTBL2", bg_row),
               "material": per_type("BgMatName", lambda ptr, k: cstr(p.u32(ptr + 4 * k))),
               "material2": per_type("BgMatName2", lambda ptr, k: cstr(p.u32(ptr + 4 * k)))}
        # WORLD::Init: for field type 6, `lq`/`ld` a 24-byte table of clump
        # names to its stack and loads the one for the weather.
        init = p.symbol_named("Init__5WORLDFv")
        words = struct.unpack(f"<{init.size // 4}I", p.read(init.value, init.size // 4 * 4))
        import xfer
        aurora = None
        for _i, va in sorted(xfer.addresses(words, init.value, p.gp).items()):
            try:
                first = p.u32(va)
                if first and p.cstr(first).startswith(b"CMP_") and "aur" in cstr(first):
                    aurora = [cstr(p.u32(va + 4 * k)) for k in range(6)]
                    break
            except KeyError:
                continue
        out["aurora"] = aurora
        self._backgrounds = out
        return out


class Fractal:
    """FRACTAL2 (fractal.cpp): a size x size hill from a 5 x 5 key grid,
    interpolated by rows then columns with a natural cubic spline."""

    KEYS, LOW, HIGH = 5, 128, 768

    def __init__(self, size):
        self.size = size
        self.height = [0] * (size * size)
        self.keys = [dict(val=0, a=0, b=0, c=0, d=0, span=0, frame=0) for _ in range(self.KEYS)]

    def init_spline(self):
        """FRACTAL2::InitNaturalSpline (0x005b3d50), operation for operation."""
        n, keys = self.KEYS, self.keys
        step = self.size // (n - 1)
        self.now_frame = self.now_key = 0
        val = [k["val"] for k in keys] + [keys[n - 1]["val"]] * 2
        for i in range(n):
            keys[i]["frame"] = step * i
            if i == n - 1:
                keys[i]["frame"] = self.size
        for i in range(n):
            if i < n - 1:
                keys[i]["span"] = f_from_int(keys[i + 1]["frame"] - keys[i]["frame"])
            else:
                keys[i]["span"] = F(10.0)
        for i in range(n):
            f4, f5 = val[i], val[i + 1]
            if i == 0:
                f2 = f_sub(f5, f4)
            else:
                span, prev = keys[i]["span"], keys[i - 1]["span"]
                acc = f_mul(span, f_sub(f4, val[i - 1]))
                f1 = f_add(acc, f_mul(prev, f_sub(f5, f4)))
                f2 = f_div(f1, f_add(prev, span))
            if i == n - 1:
                f3 = f_sub(val[i + 2], f5)
            else:
                nxt, span = keys[i + 1]["span"], keys[i]["span"]
                acc = f_mul(span, f_sub(val[i + 2], f5))
                f1 = f_add(acc, f_mul(nxt, f_sub(f5, f4)))
                f3 = f_div(f1, f_add(span, nxt))
            acc = f_mul(TWO, f4)
            f0 = f_sub(acc, f_mul(TWO, f5))
            keys[i]["a"] = f_add(f3, f_add(f2, f0))
            acc = f_add(f_mul(F(-3.0), f4), f_mul(F(3.0), f5))
            keys[i]["b"] = f_sub(f_sub(acc, f_mul(TWO, f2)), f3)
            keys[i]["c"] = f2
            keys[i]["d"] = f4

    def step(self):
        """One frame along the spline: nowFrame++, move to the next key once
        past this one's span, evaluate d + t(c + t(b + t a)) as the game
        orders it."""
        self.now_frame += 1
        self.advance()
        k = self.keys[self.now_key]
        t = f_div(f_from_int(self.now_frame - k["frame"]), k["span"])
        t2 = f_mul(t, t)
        t3 = f_mul(t2, t)
        acc = f_add(f_mul(t3, k["a"]), f_mul(t2, k["b"]))
        return f_add(k["d"], f_add(acc, f_mul(t, k["c"])))

    def advance(self):
        k = self.keys[self.now_key]
        if not f_cmp(f_from_int(self.now_frame), f_add(f_from_int(k["frame"]), k["span"])) < 0:
            self.now_key += 1

    def generate(self, rand):
        """FRACTAL2::Generate (0x005b4070)."""
        n, size = self.KEYS, self.size
        rows = [0] * (n * size)
        for r in range(n):
            for k in range(n):
                edge = k in (0, n - 1) or r in (0, n - 1)
                self.keys[k]["val"] = f_from_int(rand(self.LOW if edge else self.HIGH))
            self.init_spline()
            for s in range(size):
                rows[r * size + s] = self.step()
        for col in range(size):
            for k in range(n):
                self.keys[k]["val"] = rows[k * size + col]
            self.init_spline()
            self.advance()
            for s in range(size):
                h = self.step()
                if f_cmp(h, ZERO) < 0:
                    h = ZERO
                self.height[col * size + s] = h


class Field:
    def __init__(self, data, seed, field_type, weather, ground, obj, event=0, protect=0,
                 skip_init=False):
        self.data = data
        self.rng = areas.Rng(seed)
        self.field_type, self.weather, self.ground, self.object = field_type, weather, ground, obj
        self.event = event              # WORLD_MAN.eventAreaNumber, and ccGame.field
        self.protect = protect          # EVENTAREA_INFO.protect[1] (IsProtectArea)
        self.map = [0] * (N * N)        # FIELD.map[x][y] as EE float bits
        self.check = bytearray(2 * C * C)   # FIELD.check then check2, contiguous
        self.check3 = bytearray(C * C)
        self.mnt = bytearray(C * C)
        self.objects = []               # (type, idx, x, y, w, h, model)
        self.object_z = []              # each object's FOBJECT(2).wp z, as EE float bits
        self.covers = []
        self.hills = []
        self.start = None
        self.entrance = None
        self.dungeon_pos = None
        self.init_draws = 0 if skip_init else init_draws(field_type, weather)
        for _ in range(self.init_draws):
            self.rng.next()

    def rand(self, n):
        return self.rng.next() % n

    # FIELD ------------------------------------------------------------------
    def set_height(self, x, y, z):
        """FIELD::SetHeight: wraps at 80, clamps below 0."""
        if x >= N:
            x -= N
        if y >= N:
            y -= N
        self.map[x * N + y] = ZERO if f_cmp(z, ZERO) < 0 else z

    def flat(self, x, y, w, h):
        """FIELD::Flat: zero the height cells [2x, 2x+w) x [2y, 2y+h)."""
        for j in range(2 * y, 2 * y + h):
            for i in range(2 * x, 2 * x + w):
                self.map[(i - N if i >= N else i) * N + (j - N if j >= N else j)] = ZERO

    def hide(self, x, y, w, h):
        for j in range(y, y + h):
            for i in range(x, x + w):
                self.check3[i * C + j] = 1

    def check_area(self, x, y, x2, y2, grid=None):
        grid = self.check if grid is None else grid
        return any(grid[i * C + j] for j in range(y, y2) for i in range(x, x2))

    def check_mount(self, x, y, x2, y2):
        return self.check_area(x, y, x2, y2, self.mnt)

    def check_area_height(self, x, y, w, h):
        """FIELD::CheckAreaHeight over the quads of the last InitQuad: 1 when
        the corner heights span more than 64."""
        hi, lo = F(-65535.0), F(65535.0)
        q = self.quads

        def at(i, j):
            return q[(i - N if i >= N else i) * N + (j - N if j >= N else j)]
        for j in range(2 * y, 2 * (y + h)):
            for i in range(2 * x, 2 * (x + w)):
                for z in (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1)):
                    if not f_cmp(z, hi) <= 0:
                        hi = z
                    if f_cmp(z, lo) < 0:
                        lo = z
        return 0 if f_cmp(f_sub(hi, lo), F(64.0)) <= 0 else 1

    def init_quad(self):
        self.quads = list(self.map)

    def set_position(self, x, y, w, h):
        """FOBJECT(2)::SetPosition2's footprint: check[x..x+w)[y..y+h) = 1;
        the object's world centre and its cell (mx, my)."""
        for j in range(y, y + h):
            for i in range(x, x + w):
                self.check[i * C + j] = 1
        chip = F(1200.0)
        wx = f_add(f_mul(chip, f_from_int(x)), f_div(f_mul(chip, f_from_int(w)), TWO))
        wy = f_add(f_mul(chip, f_from_int(y)), f_div(f_mul(chip, f_from_int(h)), TWO))
        return (wx, wy), (f_to_int(f_div(wx, F(600.0))), f_to_int(f_div(wy, F(600.0))))

    def cell_height(self, x, y):
        """FIELD::GetHeight (0x005adb80): map[x][y], wrapping 80 down to 0
        and -1 up to 78 (the game adds 79)."""
        if x >= N:
            x -= N
        if x < 0:
            x += N - 1
        if y >= N:
            y -= N
        if y < 0:
            y += N - 1
        return self.map[x * N + y]

    def get_height(self, x, y):
        """WORLD::GetHeight (gcmn 0x005aa520): the ground under world (x, y),
        as float bits - a vertical segment from z 2500 to -1200 cast at the
        point's offset in its 600-unit cell against the cell's two triangles
        (collisionLP), -500 on a hidden chip, -1 if both miss."""
        cx = eemu.sx(f_to_int(f_div(x, CELL_UNITS)), 32)
        cy = eemu.sx(f_to_int(f_div(y, CELL_UNITS)), 32)
        if self.check3[int(cx / 2) * C + int(cy / 2)] == 1:
            return HIDDEN_HEIGHT
        fx, fy = fmod_float(x, CELL_UNITS), fmod_float(y, CELL_UNITS)
        h = self.cell_height
        tris = ((GROUND_TRIANGLES[0], (h(cx, cy + 1), h(cx, cy), h(cx + 1, cy + 1))),
                (GROUND_TRIANGLES[1], (h(cx + 1, cy), h(cx + 1, cy + 1), h(cx, cy))))
        for corners, zs in tris:
            a, b, c = [(cx_, cy_, z, ONE) for (cx_, cy_), z in zip(corners, zs)]
            n = vu_normalize(triangle_normal(a, b, c) + (0,))
            p0 = (fx, fy, RAY_TOP, 0)
            p1 = (fx, fy, RAY_BOTTOM, 0)
            hit = collision_lp(p0, p1, a, b, c, n)
            if hit is not None:
                return hit[2]
        return MINUS_ONE

    # WORLD ------------------------------------------------------------------
    def make_hill(self, size):
        """WORLD::MakeHill: 1 when the hill found a place in 100 tries."""
        fr = Fractal(size)
        fr.generate(self.rand)
        for _ in range(100):
            x = self.rand(N - size)
            y = self.rand(N - size)
            cx, cy, half = x // 2, y // 2, size // 2
            if self.check_mount(cx, cy, cx + half, cy + half):
                continue
            for i in range(size):
                for j in range(size):
                    self.set_height(i + cx * 2, j + cy * 2, fr.height[j * size + i])
                    self.mnt[(i // 2 + cx) * C + j // 2 + cy] = 1
            self.hills.append((x, y, size))
            return 1
        return 0

    def dist(self, a, b):
        """ccGetDist (main 0x001d9dd0): |a - b| over x, y, z. The difference
        and the inner product are VU0 macro code, modelled here with the FPU
        rules (an inference); the square root is the executable's own
        (Data.sqrt)."""
        d = [f_sub(a[i], b[i]) for i in range(3)]
        s = f_add(f_add(f_mul(d[0], d[0]), f_mul(d[1], d[1])), f_mul(d[2], d[2]))
        return self.data.sqrt(s)

    def set_dungeon_enter(self, info):
        w, h = info["w"], info["h"]
        while True:
            x = self.rand(C - w)
            y = self.rand(C - h)
            if self.check_mount(x, y, x + w, y + h) or self.check_area(x, y, x + w, y + h):
                continue
            if self.event:
                chip = F(1200.0)
                pos = (f_add(f_mul(chip, f_from_int(x)), f_div(f_mul(chip, f_from_int(w)), TWO)),
                       f_add(f_mul(chip, f_from_int(y)), f_div(f_mul(chip, f_from_int(h)), TWO)), 0)
                near = f_cmp(self.dist(self.start_pos or (0, 0, 0), pos), F(8000.0)) < 0
                # The first story area wants its dungeon near the start; the
                # rest want it at least 8000 away.
                if near != (self.event == 14):
                    continue
            break
        if info["flat"] & 2:
            self.flat(x, y, 3 * w, 3 * h)
            self.hide(x, y, w, h)
        wp, cell = self.set_position(x, y, w, h)
        self.entrance = (x, y)
        self.dungeon_pos = (wp[0], wp[1], 0)
        self.objects.append((0, 0, x, y, w, h, info["clump"]))
        self.object_z.append(ZERO)          # SetDungeonEnter stores 0, asks no height

    def set_lake(self, table):
        idx = self.rand(len(table))
        info = table[idx]
        while True:
            x = self.rand(37)
            y = self.rand(37)
            if not (self.check_mount(x, y, x + 3, y + 3) or self.check_area(x, y, x + 3, y + 3)):
                break
        self.hide(x + 1, y + 1, 1, 1)
        self.flat(x, y, 9, 9)
        for j in range(y, y + 3):
            for i in range(x, x + 3):
                self.check[i * C + j] = 1
        self.set_position(x + 1, y + 1, info["w"], info["h"])
        self.objects.append((7, idx, x + 1, y + 1, info["w"], info["h"], info["clump"]))
        self.object_z.append(ZERO)          # GetHeight's answer, then 0 over it

    def set_objects(self, kind, table, loop):
        """SetKeyObject / SetSubObject / SetBaseObject / SetTreeObject: the
        same loop, kinds 1-4; sub objects hide before flattening and also
        for flag 4."""
        fails = 0
        for _ in range(loop):
            idx = self.rand(len(table))
            info = table[idx]
            w, h, fl = info["w"], info["h"], info["flat"]
            while True:
                x = self.rand(C - w)
                y = self.rand(C - h)
                if fl & 6:
                    bad = self.check_mount(x, y, x + w, y + h) or self.check_area(x, y, x + w, y + h)
                elif fl & 8:
                    bad = (self.check_mount(x, y, x + w, y + h) or self.check_area_height(x, y, w, h)
                           or self.check_area(x, y, x + w, y + h))
                else:
                    bad = self.check_area(x, y, x + w, y + h)
                if not bad:
                    break
                fails += 1
                if fails == 300:
                    return
            if kind == 2:
                if fl & 6:
                    self.hide(x, y, w, h)
                    self.flat(x, y, 3 * w, 3 * h)
            elif fl & 2:
                self.flat(x, y, 3 * w, 3 * h)
                self.hide(x, y, w, h)
            (wx, wy), _cell = self.set_position(x, y, w, h)
            self.objects.append((kind, idx, x, y, w, h, info["clump"]))
            # WORLD::GetHeight at the centre, on the heights as they are now;
            # 0 instead where the flags levelled the ground (sub objects for
            # flag 2 or 4, the others for 2).
            level = fl & (6 if kind == 2 else 2)
            self.object_z.append(ZERO if level else self.get_height(wx, wy))

    def set_tree_rows(self, table, loop):
        """WORLD::SetTreeObject_B (field type 1): rows of 3-5 pieces, two end
        pieces and middle pieces from the table."""
        fails = 0
        for _ in range(loop):
            length = self.rand(3) + 3
            while True:
                x = self.rand(C - length)
                y = self.rand(39)
                x2 = x + length
                if not (self.check_mount(x, y, x2, y + 1) or self.check_area(x, y, x2, y + 1)):
                    break
                fails += 1
                if fails == 300:
                    return
            self.flat(x, y, 3 * length, 3)
            self.hide(x, y, length, 1)
            pieces = [(x, self.rand(2))]
            for s in range(length - 2):
                pieces.append((x + s + 1, self.rand(2) + 2))
            pieces.append((x2 - 1, 4))
            for px, idx in pieces:
                info = table[idx]
                self.set_position(px, y, info["w"], info["h"])
                self.objects.append((4, idx, px, y, info["w"], info["h"], info["clump"]))
                self.object_z.append(ZERO)

    def set_cover(self):
        """WORLD::SetCover: ground cover on every chip not hidden."""
        meshes = self.data.small_mesh[self.field_type]
        for i in range(C):
            for j in range(C):
                if self.check[C * C + i * C + j] == 1 or self.check3[i * C + j]:
                    continue
                t = self.rand(4)
                self.covers.append((i, j, t, meshes[self.rand(len(meshes))]))

    def world_pos(self, x, y):
        """FIELD::CalcWorldMeshPosition: the centre of chip (x, y)."""
        return (f_add(F(600.0), f_mul(F(1200.0), f_from_int(x))),
                f_add(F(600.0), f_mul(F(1200.0), f_from_int(y))), 0)

    def find_start(self):
        chk = self.check
        while True:
            for y in range(C):
                for x in range(C):
                    at = x * C + y
                    if chk[at] or chk[at + C] or chk[at + 1] or chk[at + C + 1]:
                        continue
                    if self.rand(100) < 96:
                        continue
                    pos = self.world_pos(x, y)
                    if not (self.event and self.protect):
                        return (x, y), pos
                    if not f_cmp(self.dist(pos, self.dungeon_pos or (0, 0, 0)), F(7000.0)) <= 0:
                        return (x, y), pos

    def generate(self):
        """WORLD::Generate up to the start position."""
        d = self.data
        ft = self.field_type
        for i in range(N):
            for j in range(N):
                self.map[i * N + j] = f_from_int(self.rand(64) & 0xFF)
        self.start_pos = None
        if self.event:
            for a in (-1, 0, 1):
                for b in (-1, 0, 1):
                    self.check[(20 + b) * C + 20 + a] = 1
                    self.mnt[(20 + b) * C + 20 + a] = 1
            self.flat(20, 20, 3, 3)
            self.start_pos = self.world_pos(20, 20)
        small, large = {0: (3, 1), 1: (5, 1), 2: (8, 1)}[self.ground]
        while large:
            if self.make_hill(self.rand(10) + 20) == 1:
                large -= 1
        while small:
            if self.make_hill(self.rand(10) + 10) == 1:
                small -= 1
        if self.event not in NO_ENTRANCE:
            self.set_dungeon_enter(d.tables["EnterObjTABLE"][ft][0])
        if ft in LAKE_FIELDS:
            self.set_lake(d.tables["LakeObjTABLE"][ft])
        key, base, sub, tree = 20, 35, 30, 35
        if ft == 7:
            key, base, tree = key // 2, base // 3, tree // 2
        if ft == 1:
            tree //= 6
        percent = {0: F(0.8), 1: F(0.9)}.get(self.object, ONE)
        key, sub, base, tree = (f_to_int(f_mul(f_from_int(n), percent)) for n in (key, sub, base, tree))
        self.init_quad()
        self.set_objects(1, d.tables["KeyObjTABLE"][ft], key)
        self.set_objects(2, d.tables["SubObjTABLE"][ft], sub)
        self.set_objects(3, d.tables["BaseObjTABLE"][ft], base)
        if ft == 1:
            self.set_tree_rows(d.tables["TreeObjTABLE"][ft], tree)
        else:
            self.set_objects(4, d.tables["TreeObjTABLE"][ft], tree)
        self.init_quad()
        self.set_cover()
        self.start, pos = self.find_start()
        if self.event:
            # A story area starts at the flattened centre; the scan above
            # still ran and drew from the RNG.
            self.start, pos = (20, 20), self.world_pos(20, 20)
        self.start_world = pos
        return self


# render ---------------------------------------------------------------------
#
# How the game draws a field, from WORLD::DrawMesh (gcmn 0x005a8570) and what
# builds its pieces. The VU0 macro code the field uses (sceVu0Normalize,
# InnerProduct, OuterProduct, Add/Sub/Scale/DivVector, ApplyMatrix and the
# RotMatrix family) is modelled with the FPU's rules, each operation truncated
# as the EE's floating-point units do: an inference, like ccGetDist's.

def vu_add(a, b):
    return tuple(f_add(x, y) for x, y in zip(a, b))


def vu_sub(a, b):
    return tuple(f_sub(x, y) for x, y in zip(a, b))


def vu_inner(a, b):
    """sceVu0InnerProduct: vmul.xyz, then x + y, then + z."""
    return f_add(f_add(f_mul(a[0], b[0]), f_mul(a[1], b[1])), f_mul(a[2], b[2]))


def vu_normalize(v):
    """sceVu0Normalize: Q = sqrt(x*x + y*y + z*z), then 1 / Q, xyz * that;
    w becomes 0."""
    q = f_add(0, eemu.f_sqrt(vu_inner(v, v)))
    r = f_div(ONE, q)
    return (f_mul(v[0], r), f_mul(v[1], r), f_mul(v[2], r), 0)


def vu_outer(a, b):
    """sceVu0OuterProduct: opmula then opmsub, each product rounded; w 0."""
    return (f_sub(f_mul(a[1], b[2]), f_mul(b[1], a[2])),
            f_sub(f_mul(a[2], b[0]), f_mul(b[2], a[0])),
            f_sub(f_mul(a[0], b[1]), f_mul(b[0], a[1])), 0)


def vu_scale(v, k):
    return tuple(f_mul(c, k) for c in v)


def vu_div(v, k):
    """sceVu0DivVector: Q = 1 / k, then every component times Q."""
    q = f_div(ONE, k)
    return tuple(f_mul(c, q) for c in v)


def vu_apply(m, v):
    """sceVu0ApplyMatrix: columns m[0..3], ((c0 x + c1 y) + c2 z) + c3 w."""
    acc = [f_mul(m[0][k], v[0]) for k in range(4)]
    acc = [f_add(acc[k], f_mul(m[1][k], v[1])) for k in range(4)]
    acc = [f_add(acc[k], f_mul(m[2][k], v[2])) for k in range(4)]
    return tuple(f_add(acc[k], f_mul(m[3][k], v[3])) for k in range(4))


UNIT_MATRIX = ((ONE, 0, 0, 0), (0, ONE, 0, 0), (0, 0, ONE, 0), (0, 0, 0, ONE))


def vu_rot(m, axis, angle):
    """sceVu0RotMatrixX/Y/Z(m, angle): the axis rotation times m, its sine
    and cosine from _sceVu0ecossin (tools/anim.py's model)."""
    import anim
    s, c = anim.vu_cossin(angle)
    cs, sn, ns = f_add(0, c), f_add(0, s), f_sub(0, s)
    rot = {0: [[ONE, 0, 0, 0], [0, cs, sn, 0], [0, ns, cs, 0], [0, 0, 0, ONE]],
           1: [[cs, 0, ns, 0], [0, ONE, 0, 0], [sn, 0, cs, 0], [0, 0, 0, ONE]],
           2: [[cs, sn, 0, 0], [ns, cs, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]}[axis]
    return anim.vu_mul(rot, [list(col) for col in m])


def triangle_normal(a, b, c):
    """The unnormalised normal InitQuad and GetHeight compute with the FPU's
    mula/msub (each product rounded first) for corners a, b, c."""
    f9, f8 = f_sub(b[1], a[1]), f_sub(c[2], b[2])
    f2, f5 = f_sub(b[2], a[2]), f_sub(c[1], b[1])
    nx = f_sub(f_mul(f9, f8), f_mul(f2, f5))
    f4, f1 = f_sub(c[0], b[0]), f_sub(b[0], a[0])
    ny = f_sub(f_mul(f2, f4), f_mul(f1, f8))
    nz = f_sub(f_mul(f1, f5), f_mul(f9, f4))
    return (nx, ny, nz)


def collision_lp(p0, p1, a, b, c, n):
    """collisionLP (main 0x00152ee0): where segment p0-p1 meets triangle
    a, b, c with normal n, or None. p0 must be on n's side, p1 not; each
    edge test lets the point lie 0.001 outside."""
    d = vu_normalize(vu_sub(p1, p0))
    k = f_mul(MINUS_ONE, vu_inner(n, a))
    s0 = f_add(k, vu_inner(n, p0))
    s1 = f_add(k, vu_inner(n, p1))
    if f_cmp(s0, 0) < 0 or not f_cmp(s1, 0) <= 0:
        return None
    nd = vu_inner(n, d)
    if f_cmp(0, nd) == 0:
        return None
    for e0, e1 in ((b, a), (c, b), (a, c)):
        side = vu_outer(vu_sub(e0, e1), d)
        to = vu_normalize(vu_sub(p0, e1))
        if f_cmp(vu_inner(side, to), EDGE_SLACK) < 0:
            return None
    v = vu_div(vu_scale(d, f_mul(MINUS_ONE, s0)), nd)
    hit = vu_add(p0, v)
    return (hit[0], hit[1], hit[2], ONE)


def fmod_float(x, m):
    """fmod(x, m) as the game gets it, through double and back (fptodp, fmod,
    dptofp): exact, and exactly a float again for a float x and m = 600."""
    nx, ex = eemu.f_value(x)
    nm, em = eemu.f_value(m)
    e = min(ex, em)
    r = (abs(nx) << (ex - e)) % (abs(nm) << (em - e))
    out = eemu._round(int(nx < 0), r, e)
    n2, e2 = eemu.f_value(out)
    if r and abs(n2) * 2 ** e2 != r * 2 ** e:
        raise ValueError(f"fmod({x:#010x}, {m:#010x}) is not a float")
    return out


def fixed(h, scale):
    """A height as the s16 SetMESH2 and SetSmallMESH store: fptosi(4096 *
    (h / vertexScale)), low 16 bits."""
    return eemu.sx(f_to_int(f_mul(FIXED_ONE, f_div(h, scale))) & 0xFFFF, 16)


def colour_byte(v):
    """fptoui into a byte (sb keeps the low 8 bits)."""
    return f_to_int(v) & 0xFF


# the meshes' vertex tables -----------------------------------------------------

MESH_FIELD, MESH_OBJ, MESH_MODEL, MESH_MMAT = 0x01000000, 0x01100000, 0x01100100, 0x01100200
MESH_VERTS, MESH_COLS = 0x01101000, 0x01102000
MESH_SENTINEL = 0x7777


def mesh_tables(data):
    """{'tile': [(vertex, dx, dy)], 'tile_colour': [...], 'cover': [...],
    'cover_colour': [...]}: which height cell FIELD_MESH::SetMESH2 (gcmn
    0x005af780, chip (x, y): cells (2x + dx, 2y + dy)) and FIELD::SetSmallMESH
    (0x005ade80, cell (x, y): cells (x + dx, y + dy)) write into each vertex's
    z and colour - read by running both in tools/eemu.py over a FIELD whose
    every height and colour names its own cell."""
    if getattr(data, "_mesh_tables", None) is None:
        data._mesh_tables = _run_mesh_tables(data.p)
    return data._mesh_tables


def _run_mesh_tables(p):
    m = eemu.Machine(p)
    sym = lambda n: p.symbol_named(n).value   # noqa: E731
    fld = MESH_FIELD
    for x in range(N):
        for y in range(N):
            m.store(fld + 0xAF02C + 4 * (x * N + y), 4, f_from_int(x * N + y))
            m.store(fld + 0xB5430 + 16 * (x * N + y), 4, f_from_int(x))
            m.store(fld + 0xB5430 + 16 * (x * N + y) + 4, 4, f_from_int(y))

    def reset():
        m.store(MESH_MODEL + 8, 4, MESH_MMAT)
        m.store(MESH_MODEL + 12, 4, FIXED_ONE)          # vertexScale 4096: s16 = height
        m.store(MESH_MMAT + 16, 4, MESH_VERTS)
        m.store(MESH_MMAT + 24, 4, MESH_COLS)
        for k in range(256):
            m.store(MESH_VERTS + 6 * k + 4, 2, MESH_SENTINEL)
            m.store(MESH_COLS + 4 * k, 4, 0x77777777)

    def read(ox, oy):
        z, col = [], []
        for k in range(256):
            v = m.load(MESH_VERTS + 6 * k + 4, 2)
            if v != MESH_SENTINEL:
                z.append((k, v // N - ox, v % N - oy))
            r, g = m.load(MESH_COLS + 4 * k, 1), m.load(MESH_COLS + 4 * k + 1, 1)
            if m.load(MESH_COLS + 4 * k, 4) != 0x77777777:
                col.append((k, r - ox, g - oy))
        return z, col

    reset()
    m.store(MESH_OBJ, 4, fld)                           # FIELD_MESH.parent
    m.store(MESH_OBJ + 76, 4, MESH_MODEL)               # FIELD_MESH.model
    m.call(sym("SetMESH2__10FIELD_MESHFiiii"), (MESH_OBJ, 10, 10, 2, 2), limit=10_000_000)
    tile, tile_colour = read(20, 20)
    reset()
    m.call(sym("SetSmallMESH__5FIELDFiiP7ccModel"), (fld, 21, 21, MESH_MODEL), limit=10_000_000)
    cover, cover_colour = read(21, 21)
    return {"tile": tile, "tile_colour": tile_colour, "cover": cover, "cover_colour": cover_colour}


# the ground ---------------------------------------------------------------------

def quad_faces(fl):
    """FIELD::InitQuad's two face normals per quad [x][y] (normalised), from
    the heights now: face 0 of corners (x, y+1), (x, y), (x+1, y+1), face 1
    of (x+1, y), (x+1, y+1), (x, y+1)."""
    faces = []
    for x in range(N):
        for y in range(N):
            px, px1 = f_mul(CELL_UNITS, f_from_int(x)), f_mul(CELL_UNITS, f_from_int(x + 1))
            py, py1 = f_mul(CELL_UNITS, f_from_int(y)), f_mul(CELL_UNITS, f_from_int(y + 1))
            h = fl.cell_height
            p0, p1 = (px, py, h(x, y)), (px1, py, h(x + 1, y))
            p2, p3 = (px, py1, h(x, y + 1)), (px1, py1, h(x + 1, y + 1))
            faces.append((vu_normalize(triangle_normal(p2, p0, p3) + (ONE,)),
                          vu_normalize(triangle_normal(p1, p3, p2) + (ONE,))))
    return faces


def vertex_normals(fl):
    """FIELD::CalcQuadVertexNormal (0x005aea20): per vertex [x][y] the
    normalised mean of six faces: both of quads (x, y) and (x-1, y-1), face 1
    of (x-1, y) and face 0 of (x+1, y-1) - the game's choice, where (x, y-1)
    would be the neighbour."""
    faces = quad_faces(fl)

    def q(x, y):
        return faces[(x % N) * N + y % N]
    out = []
    for x in range(N):
        for y in range(N):
            s = vu_add(q(x, y)[0], q(x, y)[1])
            s = vu_add(s, q(x - 1, y - 1)[0])
            s = vu_add(s, q(x - 1, y - 1)[1])
            s = vu_add(s, q(x - 1, y)[1])
            s = vu_add(s, q(x + 1, y - 1)[0])
            out.append(vu_normalize(vu_div(s, F(6.0))))
    return out


class Light:
    """A field's light: the BG_INFO_TABLE row's LGT_ light as the background
    animation's frame 0 sets it (F_DistantLight: rotation in degrees and a
    packed RGB colour, intensity 1), and its F_Ambient colour."""

    def __init__(self, ambient, rotation, colour):
        self.ambient = ambient          # packed 0x00BBGGRR
        self.rotation = rotation        # three float bits, degrees
        self.colour = colour            # packed 0x00BBGGRR

    def ambient_rgb(self):
        """ccAnm::GetAmbient (main 0x001528e0): each byte / 255."""
        return tuple(f_div(f_from_int((self.ambient >> (8 * i)) & 0xFF), COLOUR_MAX) for i in range(3))

    def colour_rgb(self):
        """ccSetColor (main 0x00138b60) with intensity 1: each byte times
        1/255."""
        return tuple(f_mul(f_from_int((self.colour >> (8 * i)) & 0xFF), COL255) for i in range(3))

    def dirc(self):
        """ccDistantLight::GetDirc: lightVector through the light's matrix,
        sceVu0RotMatrix of the rotation in radians (DecodeF_DistantLight:
        degrees * pi / 180)."""
        import anim
        rad = [f_div(f_mul(0x40490FDB, d), F(180.0)) for d in self.rotation]
        return vu_apply(anim.rot_bits(rad), LIGHT_VECTOR)

    @classmethod
    def from_anime(cls, c, anime, light):
        """Frame 0 of Anime `anime` in CCS file c (tools/ccs.Ccs): its
        F_Ambient (0x0601) and the F_DistantLight (0x0603) of object `light`."""
        d = c.data
        names = [o[0] if isinstance(o, tuple) else getattr(o, "name", o) for o in c.objects]
        for off, t, n, _end in c.chunks():
            if t is None:
                break
            if t & 0xFFFF != 0x0700 or names[struct.unpack_from("<I", d, off + 8)[0]] != anime:
                continue
            w = struct.unpack_from(f"<{n}I", d, off + 8)
            q, top, amb, found = 3, None, None, None
            while q < len(w):
                kind, cnt = w[q] & 0xFFFF, w[q + 1]
                body = w[q + 2:q + 2 + cnt]
                if kind == 0xFF01:
                    top = body[0]
                    if top != 0:
                        break
                elif kind == 0x0601:
                    amb = body[0]
                elif kind == 0x0603 and names[body[0]] == light:
                    if body[1] & 0x20:
                        raise ValueError(f"{light}: an intensity other than 1 is not modelled")
                    found = (tuple(body[2:5]), body[5])
                q += 2 + cnt
            if amb is None or found is None:
                raise KeyError(f"{anime}: no frame-0 ambient or light {light}")
            if found[1] >> 24:
                raise ValueError(f"{light}: an HSV colour is not modelled")
            return cls(amb, found[0], found[1])
        raise KeyError(anime)


def light_direction(dirc):
    """The direction Lambert (gcmn 0x005af2d0) lights along: lightVector
    rotated by Rz * Ry * Rx of dirc's components taken as angles - the
    game's own reading of its direction vector - and normalised. Lambert
    builds it again for every vertex; it depends on the light alone."""
    m = UNIT_MATRIX
    for axis in range(3):
        m = vu_rot(m, axis, dirc[axis])
    return vu_normalize(vu_apply(m, LIGHT_VECTOR))


def lambert(direction, n, colour, ambient, light_colour):
    """Lambert's colour: each channel colour * (ambient + max(0, -n.L) *
    light colour), clamped to 0-255."""
    dot = f_mul(vu_inner(n, direction), MINUS_ONE)
    if f_cmp(dot, 0) < 0:
        dot = 0
    out = []
    for i in range(3):
        v = f_mul(colour[i], f_add(ambient[i], f_mul(dot, light_colour[i])))
        if f_cmp(v, 0) < 0:
            v = 0
        elif not f_cmp(v, COLOUR_MAX) <= 0:
            v = COLOUR_MAX
        out.append(v)
    return tuple(out)


def vertex_colours(fl, light):
    """FIELD::CalcVertexColor (0x005aed50), as WORLD::Generate runs it after
    the objects: FIELD.color[x][y] (RGB float bits, from 128) lit by the light
    at each vertex normal."""
    direction = light_direction(light.dirc())
    amb, lc = light.ambient_rgb(), light.colour_rgb()
    base = (BASE_COLOUR,) * 3
    return [lambert(direction, n, base, amb, lc) for n in vertex_normals(fl)]


def cell_colour(colours, x, y):
    """FIELD::GetColor: wraps both ways by 80."""
    return colours[(x % N) * N + y % N]


def tile(fl, mx, my, scale, colours=None):
    """FIELD_MESH::RelocateMesh (0x005b0aa0) and SetMESH2 for chip (mx, my):
    whether it is drawn (not on a hidden chip), its translation (the chip's
    centre), and the ground model's vertices it rewrites - {vertex: s16 z}
    from the heights (vertexScale `scale`), {vertex: (r, g, b)} from the
    colours when given."""
    t = mesh_tables(fl.data)
    pos = (f_add(CELL_UNITS, f_mul(CHIP_UNITS, f_from_int(mx))),
           f_add(CELL_UNITS, f_mul(CHIP_UNITS, f_from_int(my))), 0)
    z = {k: fixed(fl.cell_height(2 * mx + dx, 2 * my + dy), scale) for k, dx, dy in t["tile"]}
    col = None
    if colours is not None:
        col = {k: tuple(colour_byte(c) for c in cell_colour(colours, 2 * mx + dx, 2 * my + dy))
               for k, dx, dy in t["tile_colour"]}
    return {"visible": fl.check3[mx * C + my] != 1, "pos": pos, "z": z, "colours": col}


def cover_offset(t):
    """FCOVER::SetPosition (0x005b1890): the tile's offset from its chip's
    centre by quadrant t, and its z."""
    s = F(300.0)
    return ((f_sub(0, s) if t in (0, 2) else s), (f_sub(0, s) if t in (0, 1) else s), F(2.5))


def cover_mesh(fl, cover, scale, colours=None):
    """A cover tile as WORLD::SetCover places it: position (chip centre plus
    the quadrant offset, as WORLD::DrawMesh adds them) and the vertices
    FIELD::SetSmallMESH rewrites for cell (2i + 1 + sx, 2j + 1 + sy)."""
    i, j, t, mesh = cover
    sx, sy = t & 1, t >> 1
    ox, oy, oz = cover_offset(t)
    pos = (f_add(ox, f_add(CELL_UNITS, f_mul(CHIP_UNITS, f_from_int(i)))),
           f_add(oy, f_add(CELL_UNITS, f_mul(CHIP_UNITS, f_from_int(j)))), oz)
    cx, cy = 2 * i + 1 + sx, 2 * j + 1 + sy
    tb = mesh_tables(fl.data)
    z = {k: fixed(fl.cell_height(cx + dx, cy + dy), scale) for k, dx, dy in tb["cover"]}
    col = None
    if colours is not None:
        col = {k: tuple(colour_byte(c) for c in cell_colour(colours, cx + dx, cy + dy))
               for k, dx, dy in tb["cover_colour"]}
    return {"mesh": mesh, "pos": pos, "z": z, "colours": col}


def object_colours(normals, colours, light):
    """CalcObjectVertexColor (gcmn 0x005a4560): what WORLD::Init does to each
    rigid model of the field's object clumps - every vertex colour lit like
    the ground, from its own RGB bytes and its s8 normal / 64; fptosi back to
    bytes."""
    direction = light_direction(light.dirc())
    amb, lc = light.ambient_rgb(), light.colour_rgb()
    out = []
    for n, c in zip(normals, colours):
        nf = tuple(f_div(f_from_int(eemu.sx(v, 8)), NORMAL_UNIT) for v in n[:3])
        lit = lambert(direction, nf, tuple(f_from_int(v) for v in c[:3]), amb, lc)
        out.append(tuple(f_to_int(v) & 0xFF for v in lit) + tuple(c[3:]))
    return out


# output -----------------------------------------------------------------------

def heights(fl):
    return [eemu.f_to_py(v) for v in fl.map]


def write_png(fl, path, scale=4, overlay=False):
    """The heightmap in grey (x across, y down, white the highest); with
    overlay, objects outlined in colour and the start chip checkered."""
    import png
    hs = heights(fl)
    top = max(hs) or 1.0
    colours = {0: (255, 64, 64), 1: (255, 200, 0), 2: (0, 200, 255), 3: (160, 255, 120),
               4: (0, 150, 0), 7: (60, 90, 255)}
    size = N * scale
    img = bytearray(size * size * 4)
    for y in range(size):
        for x in range(size):
            v = int(255 * hs[(x // scale) * N + y // scale] / top)
            img[(y * size + x) * 4:(y * size + x) * 4 + 4] = bytes((v, v, v, 255))
    if not overlay:
        png.write_rgba(path, size, size, bytes(img))
        return
    for kind, _idx, cx, cy, w, h, _m in fl.objects:
        c = colours.get(kind, (255, 0, 255))
        x0, y0, x1, y1 = cx * 2 * scale, cy * 2 * scale, (cx + w) * 2 * scale - 1, (cy + h) * 2 * scale - 1
        for x in range(x0, x1 + 1):
            for y in (y0, y1):
                if 0 <= x < size and 0 <= y < size:
                    img[(y * size + x) * 4:(y * size + x) * 4 + 3] = bytes(c)
        for y in range(y0, y1 + 1):
            for x in (x0, x1):
                if 0 <= x < size and 0 <= y < size:
                    img[(y * size + x) * 4:(y * size + x) * 4 + 3] = bytes(c)
    if fl.start:
        sx, sy = fl.start
        for x in range(sx * 2 * scale, (sx + 2) * 2 * scale):
            for y in range(sy * 2 * scale, (sy + 2) * 2 * scale):
                if (x + y) % 2:
                    img[(y * size + x) * 4:(y * size + x) * 4 + 3] = b"\xff\x00\xff"
    png.write_rgba(path, size, size, bytes(img))


def chip_map(fl):
    """40 x 40 chips: E entrance, L lake, K key, S sub, B base, T tree, @
    start, ^ hill, . free."""
    grid = [["." if not fl.mnt[i * C + j] else "^" for i in range(C)] for j in range(C)]
    letters = {0: "E", 1: "K", 2: "S", 3: "B", 4: "T", 7: "L"}
    for kind, _idx, x, y, w, h, _m in fl.objects:
        for i in range(x, min(x + w, C)):
            for j in range(y, min(y + h, C)):
                grid[j][i] = letters.get(kind, "?")
    if fl.start:
        grid[fl.start[1]][fl.start[0]] = "@"
    return ["".join(row) for row in grid]


def describe(fl, objects=False):
    hs = heights(fl)
    out = [f"fieldType {fl.field_type}, weather {fl.weather}, ground {fl.ground}, "
           f"object {fl.object}; {fl.init_draws} draws before Generate",
           f"heights {min(hs):.1f} .. {max(hs):.1f}; {len(fl.hills)} hills "
           + ", ".join(f"{s}@({x},{y})" for x, y, s in fl.hills),
           f"dungeon entrance at chip {fl.entrance}, start at chip {fl.start}, "
           f"{len(fl.objects)} objects, {len(fl.covers)} cover tiles",
           f"seed after {fl.rng.seed} (randcnt {fl.rng.count})", ""]
    out.extend(chip_map(fl))
    if objects:
        for kind, idx, x, y, w, h, model in fl.objects:
            out.append(f"  {OBJECT_NAMES.get(kind, kind):8} {idx:2}  chip ({x:2},{y:2}) {w}x{h}  {model}")
    return "\n".join(out)


def from_words(data, a, b, c, server=0, volume=None, flag71=False):
    """The area three words make and its Field; `volume` defaults to the
    executable's own volumeNum. `protect` is IsProtectArea's: the story
    area's EVENTAREA_INFO, or its substitute record where the game uses one."""
    area = areas.generate(data.areas, a, b, c, server, volume, flag71)
    event = data.areas.event_info(area["event"], area["volume"], flag71) if area["event"] else None
    protect = event["protect"][1] if event else 0
    return area, Field(data, area["fieldSeed"], area["fieldType"], area["weather"],
                       area["ground"], area["object"], area["event"], protect)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("gen")
    p.add_argument("elf")
    p.add_argument("a")
    p.add_argument("b")
    p.add_argument("c")
    p.add_argument("--server", type=int, default=0)
    p.add_argument("--volume", type=int, default=None,
                   help="volumeNum (default: the executable's own)")
    p2 = sub.add_parser("seed")
    p2.add_argument("elf")
    p2.add_argument("seed", type=lambda s: int(s, 0))
    for k in ("field_type", "weather", "ground", "object"):
        p2.add_argument(k, type=int)
    p2.add_argument("--event", type=int, default=0)
    for q in (p, p2):
        q.add_argument("--png", help="write the heightmap here, and FILE_objects.png with "
                                     "the objects drawn over it")
        q.add_argument("--objects", action="store_true", help="list every object")
    args = parser.parse_args()

    data = Data(args.elf)
    if args.cmd == "gen":
        def key(s):
            return int(s) if s.isdigit() else s
        area, fl = from_words(data, key(args.a), key(args.b), key(args.c), args.server, args.volume)
        print(f"{' '.join(area['words'])}: code {area['code']}, event {area['event']}, "
              f"fieldSeed {area['fieldSeed']}")
        if area["fieldType"] == 4:
            print("field type 4 has no field: the gate leads straight into the dungeon")
            return 0
    else:
        fl = Field(data, args.seed, args.field_type, args.weather, args.ground, args.object,
                   args.event)
    print(describe(fl.generate(), args.objects))
    if args.png:
        write_png(fl, args.png)
        over = os.path.splitext(args.png)[0] + "_objects.png"
        write_png(fl, over, overlay=True)
        print(f"wrote {args.png} and {over}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
