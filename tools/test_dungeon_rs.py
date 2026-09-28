#!/usr/bin/env python3
"""crates/piney-data's dungeon module against tools/dungeon.py.

tools/test_dungeon.py checks dungeon.py against the game's own code; this
checks the Rust port against dungeon.py, field by field: the
dungeon_snapshot example (built with cargo, reading the tables piney-gen
writes to work/data) answers the same requests as dungeon.py -

  - random dungeons across all ten types, servers 0-4, volumes 1-4, word
    131, lake dungeons entered from field type 4, dungeonData sizes and raw
    ones: every floor's map, stairs, restarts and starts, every room's
    place, size, exits, table row, pick, model, rotation, minimap code,
    statue flag and dummy rolls, the kept gimmicks and the final RNG;
  - every EditDungeon entry through MakeRealMap;
  - SetDungeonTypeFromField over field types, story areas, flags and both
    save-flag rules;
  - the dummy counts of every dungeon CCS file;
  - the fog and ambient row: the DUNGEON constructor's fog block run in
    tools/eemu.py for every clutType 0-5, texType, dungeon type and
    background 0-3 (the table, row, packed fog colour and ambient it leaves),
    and WORLD_MAN::SetDungeonTexClut for servers 0-5 and every field type.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
DATA_BIN = volume.DATA
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "dungeon_snapshot")
# Scratch addresses for the interpreter: a DUNGEON and a WORLD_MAN.
DG, WM, GAME = 0x01000000, 0x01200000, 0x01202000
RANDOM_CASES = 360
EVENTS = (0, -1, 120, 66, 91, 16, 18, 21, 14, 17, 47, 71, 99)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data",
                    "--example", "dungeon_snapshot"], cwd=ROOT, check=True)


def ask(requests, *args):
    """The example's answers, one parsed JSON value per request."""
    p = subprocess.run([EXAMPLE, DATA_BIN, *args], input="\n".join(requests) + "\n",
                       capture_output=True, text=True, check=True)
    out = [json.loads(line) for line in p.stdout.splitlines()]
    if len(out) != len(requests):
        raise AssertionError(f"{len(requests)} requests, {len(out)} answers")
    return out


def run_range(m, start, stop, regs, limit=100_000):
    """Run the machine from start until pc reaches stop, from zeroed
    registers bar $gp, $sp and `regs`."""
    import eemu
    m.r = [0] * 32
    m.r[28] = m.p.gp
    m.r[29] = eemu.STACK_TOP
    m.r[31] = eemu.RETURN_SENTINEL
    for reg, value in regs.items():
        m.set32(reg, value)
    pc, npc = start, start + 4
    for _ in range(limit):
        if pc == stop:
            return
        hook = m.hooks.get(pc)
        if hook is not None:
            m.set32(2, hook(m, *(m.r[4 + i] & 0xFFFFFFFF for i in range(4))))
            pc = m.r[31] & 0xFFFFFFFF
            npc = pc + 4
            continue
        target = m.exec(pc, m.load(pc, 4))
        if target is eemu.ANNUL:
            pc, npc = npc + 4, npc + 8
        else:
            pc, npc = npc, (target if target is not None else npc + 4)
    raise AssertionError(f"no stop at 0x{stop:08x} after {limit} steps")


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
        i = next(i for i, (x, y) in enumerate(zip(a, b)) if x != y) if len(a) == len(b) else 0
        cell = i // 2 // 3
        return f"{path}: hex differs at byte {i // 2} (x={cell // 80} y={cell % 80} field {i // 2 % 3})"
    return None if a == b else f"{path}: rust {a!r}, python {b!r}"


def random_cases(data):
    """(seed, type, levelMax, roomMax, server, volume, word A, field type,
    code) - the cases tools/test_dungeon.py runs in the game, then random
    ones, every type equally often."""
    cases = [(12345, 1, 3, 7, 0, 1, 0, 0, 0), (0x1234567, 1, 2, 5, 0, 1, 131, 0, 0),
             (0x7654321, 8, 2, 5, 0, 1, 131, 0, 0), (0x2468ACE, 8, 3, 7, 0, 1, 0, 4, 0),
             (0x1357BDF, 9, 3, 7, 0, 1, 0, 4, 0)]
    rnd = random.Random(0xD1E5)
    for i in range(RANDOM_CASES - len(cases)):
        dtype = i % 10
        if rnd.random() < 0.7:
            levels, rooms = data.areas.dungeon[rnd.randrange(1, 11)]
        else:
            levels, rooms = rnd.randrange(1, 11), rnd.randrange(2, 13)
        word_a = 131 if rnd.random() < 0.12 else rnd.choice((0, 1, 7, 130, 132, 200))
        field_type, code = rnd.randrange(11), rnd.randrange(3)
        if dtype in dungeon_mod().LAKE_TYPES and rnd.random() < 0.5:
            field_type, code = 4, 0
        cases.append((rnd.getrandbits(32), dtype, levels, rooms, rnd.randrange(5), rnd.randrange(1, 5),
                      word_a, field_type, code))
    return cases


def dungeon_mod():
    import dungeon
    return dungeon


def py_random(data, case):
    """dungeon.py's Generator in the example's shape."""
    dungeon = dungeon_mod()
    seed, dtype, levels, rooms, server, volume, word_a, field_type, code = case
    g = dungeon.Generator(data, seed, dtype, levels, rooms, server, volume, word_a,
                          field_type=field_type, code=code)
    try:
        g.generate()
    except RuntimeError:
        return {"error": "no down stairs", "floor": len(g.floors)}
    names = [p[0] for p in dungeon.GIM_PATTERNS]
    floors = []
    for fl in g.floors:
        grid = bytearray()
        for i in range(dungeon.MAP * dungeon.MAP):
            grid += bytes((fl.d[i], fl.next[i], fl.here[i]))
        start = [None if s is None else {"pos": None if s[0] is None else [s[0], s[1]], "w": s[3]}
                 for s in fl.start]
        rooms_out = []
        for r in fl.rooms:
            model = None
            if r["model"] is not None:
                model = {"table": r["table"], "k": r["k"], "r": r["r"], "name": r["model"],
                         "rotate": r["rotate"], "minimap": r["minimap"], "statue": r["sym"],
                         "gims": [[names.index(pat), keep] for pat, _kind, keep in r["gims"]]}
            rooms_out.append({"index": r["index"], "x": r["x"], "y": r["y"], "size": r["size"],
                              "direc": r["direc"], "pos": list(r["pos"]), "model": model})
        floors.append({"level": fl.level, "up": fl.up, "down": fl.down, "room_num": len(fl.rooms),
                       "retries": fl.retries, "start": start, "map": grid.hex(), "rooms": rooms_out})
    return {"level_max": g.level_max, "seed": g.rng.seed, "randcnt": g.rng.count,
            "sym": [g.sym_flag, g.sym_floor, g.sym_block, g.lake_flag],
            "gims": [list(x) for x in g.gims],
            "floorRoomNum": g.floors[-1].room_num if g.floors else 0, "floors": floors}


def py_story(data, event, index):
    dungeon = dungeon_mod()
    ed = dungeon.Edited(data, event, index)
    floors = []
    for fl in ed.floors:
        grid = b"".join(bytes((fl.d[c], fl.next[c], fl.here[c])) for c in range(6400))
        floors.append({"level": fl.level, "up": fl.up, "down": fl.down,
                       "rooms": [r["index"] for r in fl.rooms], "map": grid.hex()})
    return {"slot": ed.slot, "name": ed.name, "gimmicks": len(ed.gimdata), "floors": floors}


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(DATA_BIN), "game files not present")
@unittest.skipUnless(shutil.which("cargo"), "cargo not found")
class TestDungeonRust(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = dungeon_mod().Data(ELF)
        build()

    def test_random_dungeons_match_python(self):
        cases = random_cases(self.data)
        got = ask([f"random {' '.join(str(v) for v in c)}" for c in cases])
        bad = []
        seen = {"types": set(), "servers": set(), "volumes": set(), "word131": 0, "entry": 0,
                "floors": 0, "rooms": 0, "restarts": 0, "statues": 0, "gims": 0}
        for case, rust in zip(cases, got):
            py = py_random(self.data, case)
            d = first_difference(rust, py)
            if d:
                bad.append(f"{case}: {d}")
                continue
            seen["types"].add(case[1])
            seen["servers"].add(case[4])
            seen["volumes"].add(case[5])
            seen["word131"] += case[6] == 131 and case[1] not in (8, 9)
            seen["entry"] += case[1] in (8, 9) and case[7] == 4 and case[8] == 0
            if "error" in py:
                continue
            seen["floors"] += len(py["floors"])
            seen["rooms"] += sum(len(f["rooms"]) for f in py["floors"])
            seen["restarts"] += sum(f["retries"] for f in py["floors"])
            seen["statues"] += py["sym"][0] or py["sym"][3]
            seen["gims"] += len(py["gims"])
        self.assertEqual(bad, [], f"{len(bad)} of {len(cases)} differ")
        self.assertEqual(seen["types"], set(range(10)))
        self.assertEqual(seen["servers"], set(range(5)))
        self.assertEqual(seen["volumes"], {1, 2, 3, 4})
        for k in ("word131", "entry", "restarts", "statues"):
            self.assertGreater(seen[k], 10, k)
        print(f"\n{len(cases)} random dungeons, 0 mismatches: {seen['floors']} floors, "
              f"{seen['rooms']} rooms, {seen['restarts']} restarts, {seen['statues']} statue rooms, "
              f"{seen['gims']} kept gimmicks, {seen['word131']} word-131 and {seen['entry']} "
              f"lake-entry cases", file=sys.stderr)

    def test_story_dungeons_match_python(self):
        import struct
        p = self.data.p
        base = p.symbol_named("EditDungeon").value
        keys = [struct.unpack("<2i", p.read(base + 24 * i, 8)) for i in range(self.data.edit_count)]
        got = ask([f"story {ev} {idx}" for ev, idx in keys])
        for (ev, idx), rust in zip(keys, got):
            self.assertIsNone(first_difference(rust, py_story(self.data, ev, idx)), (ev, idx))
        self.assertEqual(len(keys), 88)

    def test_dungeon_types_match_python(self):
        dungeon = dungeon_mod()
        plain = ",".join(f"{e}={t}" for e, t in dungeon.PLAIN_FIXED_TYPES.items())
        queries = [(rule, ft, ev, flag, save) for rule in ("e", "plain") for ft in range(13)
                   for ev in EVENTS for flag in (0, 3) for save in (0, 1)]
        got = ask([f"type {' '.join(str(v) for v in q)}" for q in queries], "--plain-fixed", plain)
        for (rule, ft, ev, flag, save), rust in zip(queries, got):
            want = dungeon.dungeon_type(ft, ev, flag, bool(save), "alt" if rule == "e" else "plain")
            self.assertEqual(rust, want, (rule, ft, ev, flag, save))

    def test_dummy_counts_match_python(self):
        names = sorted(set(self.data.dungeon_name) | set(self.data.dungeon_name2))
        got = ask([f"dummies {n}" for n in names])
        present = 0
        for name, rust in zip(names, got):
            try:
                py = self.data.gim_counts(name)
            except StopIteration:           # sd4a: named, not in DATA.BIN
                self.assertIsNone(rust, name)
                continue
            present += 1
            self.assertEqual(rust, py, name)
        self.assertEqual(present, 18)

    def test_fog_rule_matches_game(self):
        """The constructor's fog block, from `sw $zero, 0x50($s0)` (fog = 0)
        to the `sw` of the packed fog colour, against Tables::fog_table."""
        import eemu
        p = self.data.p
        va, words, _addrs = self.data._code("__ct__7DUNGEONFi")
        fog_stores = [va + 4 * k for k, w in enumerate(words)
                      if w >> 26 == 0x2B and (w >> 21) & 31 == 16 and w & 0xFFFF == 0x50]
        self.assertEqual(len(fog_stores), 2)
        m = eemu.Machine(p)
        m.store(p.symbol_named("worldman").value, 4, WM)
        cases = [(clut, tex, dtype, bg) for clut in range(6) for tex in (0, 1)
                 for dtype in range(10) for bg in range(4)]
        got = ask([f"fog {dtype} {clut} {tex} {bg}" for clut, tex, dtype, bg in cases])
        tables = set()
        for (clut, tex, dtype, bg), rust in zip(cases, got):
            m.mem[DG:DG + 0x200] = bytes(0x200)
            m.mem[WM:WM + 0x4E0] = bytes(0x4E0)
            m.store(DG + 0x10, 4, dtype)
            m.store(WM + 0x140, 4, clut)
            m.store(WM + 0x144, 4, tex)
            m.store(WM + 0x0C, 4, bg)
            run_range(m, fog_stores[0], fog_stores[1] + 4, {16: DG})
            where = (clut, tex, dtype, bg)
            self.assertEqual(rust["table"], m.load(DG + 0x144, 4), where)
            self.assertEqual(rust["index"], m.load(DG + 0x148, 4), where)
            self.assertEqual(rust["fog"], m.load(DG + 0x50, 4), where)
            # SetAmbient's argument, row / 255 on the stack: the EE divides
            # towards zero, IEEE to nearest, so allow the last bit.
            game = [m.load(eemu.STACK_TOP + 96 + 4 * i, 4) for i in range(3)]
            for a, b in zip(rust["ambient"], game):
                self.assertLessEqual(abs(a - b), 1, where)
            tables.add(rust["table"])
        self.assertEqual(len(tables), 13)
        print(f"\n{len(cases)} fog selections, 0 mismatches over {len(tables)} tables", file=sys.stderr)

    def test_tex_clut_matches_game(self):
        import eemu
        p = self.data.p
        m = eemu.Machine(p)
        m.store(p.symbol_named("worldman").value, 4, WM)
        m.store(p.symbol_named("game").value, 4, GAME)
        fn = p.symbol_named("SetDungeonTexClut__9WORLD_MANFv").value
        cases = [(server, ft) for server in range(6) for ft in range(13)]
        got = ask([f"texclut {server} {ft}" for server, ft in cases])
        for (server, ft), rust in zip(cases, got):
            m.mem[WM:WM + 0x4E0] = bytes(0x4E0)
            m.store(WM + 0x10, 4, ft)
            m.store(GAME + 0x1C, 4, server)
            m.call(fn, (WM,))
            self.assertEqual(rust, [m.load(WM + 0x140, 4), m.load(WM + 0x144, 4)], (server, ft))


if __name__ == "__main__":
    unittest.main()
