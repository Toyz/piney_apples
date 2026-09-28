#!/usr/bin/env python3
"""crates/piney-data's area module against the game's own code run in
tools/eemu.py.

The area_probe example answers the same questions as Infection's functions
run on a scratch WORLD_MAN (tools/test_areas.py's set-up):

  - WORLD_MAN::SimGenerateCode (0x0019e5c0) over every story area with an
    address, its words typed on each of the five servers, with and without
    the crisis byte; areas 71 and 47 under their special conditions (the
    save flag set, volumeNum forced to 3 and 4); and random keyword triples
    on random servers. Compared: every field it writes - WORLD_MAN.A/B/C,
    fieldSeed, dungeonSeed[3], bgnum, fieldtype, eventAreaNumber, timeSym,
    dungeonLevelNum, dungeonRoomNum, dungeonType[0..1] (from
    SetDungeonTypeFromField), the merged WORDPARAM's ID and nine attributes,
    and the globals seed and randcnt;
  - WORLD_MAN::IsProtectArea (0x0019ceb0) for every area number -1 .. 130;
  - WORLD_MAN::GetEventAreaInfo(int) (0x0019d3b0) for the same numbers;
  - WORLD_MAN::GetWordParamFromEvCode (0x001a29a0) for every row's code and
    each part, and the instruction 118 path it feeds (`area N`: those IDs
    through SimGenerateCode) for every code on every server;
  - WORLD_MAN::GetWordParamID (0x001a3370) for every keyword's text.

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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from volume import ELF, ISO, ROOT, va as inf_va  # noqa: E402  # PINEY_VOLUME's disc
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "area_probe")

WM, WP, GAME, SAVE, STR = 0x01800000, 0x01801000, 0x01802000, 0x01810000, 0x01820000
SEED, RANDCNT = inf_va(0x00377D20), inf_va(0x00378A80)  # the RNG's globals
CRISIS = 0x6772                                # saveData.crisis
ATTRS = ("fieldType", "dungeonSize", "weather", "ground", "object", "areaLevel", "enemyOfs", "itemOfs",
         "circleOfs")


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v


class Game:
    """The volume's WORLD_MAN functions in eemu."""

    def __init__(self):
        import test_anim
        from image import Program
        self.p = Program(ELF)
        self.m = test_anim.machine_class()(self.p)
        self.sym = lambda n: self.p.symbol_named(n).value
        m = self.m
        m.store(self.sym("worldman"), 4, WM)
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("saveData"), 4, SAVE)
        self.vol = self.sym("volumeNum")
        self.volume = m.load(self.vol, 4)

    def clear(self, base, n):
        self.m.mem[base:base + n] = bytes(n)

    def rd(self, va):
        return s32(self.m.load(va, 4))

    def generate(self, a, b, c, server, flag71=False, crisis=False, volume=None):
        m = self.m
        self.clear(WM, 0x4E0)
        self.clear(WP, 0x30)
        self.clear(SAVE, 0x8530)
        self.clear(GAME, 0x88)
        m.store(GAME + 0x1C, 4, server)
        m.store(WM + 0x158, 4, WP)
        m.store(SAVE + 0x5BC0, 8, (1 << 62) if flag71 else 0)
        m.store(SAVE + CRISIS, 1, 1 if crisis else 0)
        m.store(self.vol, 4, self.volume if volume is None else volume)
        m.store(SEED, 4, 0)
        m.store(RANDCNT, 4, 0)
        m.call(self.sym("SimGenerateCode__9WORLD_MANFiii"), (WM, a, b, c))
        out = {k: self.rd(WP + 0xC + 4 * i) for i, k in enumerate(ATTRS)}
        out.update(code=self.rd(WP + 4), a=self.rd(WM + 0x14C), b=self.rd(WM + 0x150), c=self.rd(WM + 0x154),
                   fieldSeed=m.load(WM + 0x44, 4), dungeonSeed=[m.load(WM + 0x48 + 4 * i, 4) for i in range(3)],
                   event=self.rd(WM + 0x120), timeSym=self.rd(WM + 0x134), levelMax=self.rd(WM + 0x138),
                   roomMax=self.rd(WM + 0x13C), seed=m.load(SEED, 4), randcnt=m.load(RANDCNT, 4),
                   dungeonType=[self.rd(WM + 0x34), self.rd(WM + 0x38)])
        # WORLD_MAN's own copies of the field type and weather.
        out["wmFieldType"], out["wmWeather"] = self.rd(WM + 0x10), self.rd(WM + 0xC)
        return out

    def go(self, a, b, c, server):
        """WORLD_MAN::SetGenerateCode(a, b, c): what it asks of ccGame."""
        m = self.m
        self.generate(a, b, c, server)
        calls = []

        def change_area(mm, this, area, n, *rest):
            calls.append({"area": [s32(area), s32(n)]})
            return 0

        def change_scene(mm, this, area, town, field, *rest):
            # The last three arguments are in $t0 .. $t2, which this hook does
            # not see; SetGenerateCode passes 0 for all three.
            calls.append({"scene": [s32(area), s32(town), s32(field), 0, 0, 0]})
            return 0
        hooks = {self.sym("ChangeArea__6ccGameFii"): change_area,
                 self.sym("ChangeScene__6ccGameFiiiiii"): change_scene}
        before = {k: m.hooks.get(k) for k in hooks}
        m.hooks.update(hooks)
        try:
            self.clear(WM, 0x4E0)
            self.clear(WP, 0x30)
            m.store(WM + 0x158, 4, WP)
            m.store(GAME + 0x1C, 4, server)
            m.call(self.sym("SetGenerateCode__9WORLD_MANFiii"), (WM, a, b, c))
        finally:
            for k, v in before.items():
                if v is None:
                    del m.hooks[k]
                else:
                    m.hooks[k] = v
        assert len(calls) == 1, calls
        return calls[0]

    def protect(self, n):
        self.m.store(WM + 0x120, 4, n & 0xFFFFFFFF)
        return self.m.call(self.sym("IsProtectArea__9WORLD_MANFv"), (WM,))

    def info_row(self, n):
        va = self.m.call(self.sym("GetEventAreaInfo__9WORLD_MANFi"), (WM, n & 0xFFFFFFFF))
        return -1 if va == 0 else (va - self.sym("eventAreaInfo")) // 0x54

    def from_ev(self, n, part):
        va = self.m.call(self.sym("GetWordParamFromEvCode__9WORLD_MANFii"), (WM, n & 0xFFFFFFFF, part))
        return None if va == 0 else self.rd(va + 4)

    def word_id(self, text):
        raw = text.encode("latin-1") + b"\0"
        self.m.mem[STR:STR + len(raw)] = raw
        return s32(self.m.call(self.sym("GetWordParamID__9WORLD_MANFPc"), (WM, STR)))


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data", "--example",
                    "area_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "game files or cargo not present")
class TestAreaRs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import areas
        build()
        cls.game = Game()
        cls.data = areas.Data(ELF)

    def compare(self, cases):
        """cases: (a, b, c, server, flag71, crisis, volume); every field
        SimGenerateCode writes, the port against the game."""
        lines = [f"gen {a} {b} {c} {s} {int(f)} {int(k)} {self.game.volume if v is None else v}"
                 for a, b, c, s, f, k, v in cases]
        got = ask(lines)
        bad = 0
        for case, rs in zip(cases, got):
            game = self.game.generate(*case)
            self.assertEqual(game.pop("wmFieldType"), game["fieldType"])
            self.assertEqual(game.pop("wmWeather"), game["weather"])
            if rs != game:
                bad += 1
                diff = {k: (game[k], rs.get(k)) for k in game if rs.get(k) != game[k]}
                self.fail(f"{case}: {diff}")
        return len(cases), bad

    def story_words(self):
        """(code, server, [a, b, c]) of every story area whose words are all
        keywords."""
        out = []
        for e in self.data.events:
            if not e["has_words"]:
                continue
            try:
                ids = [self.data.find(i, e[k]).ID for i, k in enumerate(("wordA", "wordB", "wordC"))]
            except SystemExit:
                continue
            out.append((e["code"], e["server"], ids))
        return out

    def test_sim_generate_code(self):
        cases = []
        for code, server, (a, b, c) in self.story_words():
            for s in range(5):
                cases.append((a, b, c, s, False, False, None))
            cases.append((a, b, c, server, False, True, None))
            if code in (47, 71):
                cases += [(a, b, c, server, True, False, None), (a, b, c, server, True, False, 3),
                          (a, b, c, server, False, False, 4), (a, b, c, server, True, True, 4)]
        rnd = random.Random(11)
        slots = [[w for w in self.data.words if w.slot == s] for s in range(3)]
        for _ in range(400):
            a, b, c = (rnd.choice(s).ID for s in slots)
            cases.append((a, b, c, rnd.randrange(5), rnd.random() < 0.2, rnd.random() < 0.3, None))
        n, bad = self.compare(cases)
        print(f"\nSimGenerateCode: {n} cases, {bad} mismatches", file=sys.stderr)

    def test_protect_and_info(self):
        numbers = list(range(-1, 131))
        got = ask([f"protect {n}" for n in numbers] + [f"info {n}" for n in numbers])
        for n, r in zip(numbers, got[:len(numbers)]):
            self.assertEqual(r["protect"], self.game.protect(n), f"IsProtectArea {n}")
        for n, r in zip(numbers, got[len(numbers):]):
            self.assertEqual(r["row"], self.game.info_row(n), f"GetEventAreaInfo {n}")
        protected = sum(r["protect"] for r in got[:len(numbers)])
        print(f"\nIsProtectArea, GetEventAreaInfo: {len(numbers)} area numbers each, 0 mismatches, "
              f"{protected} protected", file=sys.stderr)

    def test_instruction_118(self):
        codes = sorted({e["code"] for e in self.data.events})
        parts = [(n, p) for n in codes for p in range(3)]
        got = ask([f"fromev {n} {p}" for n, p in parts])
        for (n, p), r in zip(parts, got):
            if not any(e["code"] == n and e["has_words"] for e in self.data.events):
                continue    # the game reads through NULL for a row with no address
            self.assertEqual(r["id"], self.game.from_ev(n, p), f"GetWordParamFromEvCode({n}, {p})")
        cases = [(n, s) for n in codes if n not in range(1, 14) for s in range(5)]
        got = ask([f"ev {n} {s} 0" for n, s in cases])
        generated = 0
        for (n, s), r in zip(cases, got):
            ids = [self.game.from_ev(n, p) for p in range(3)]
            if None in ids:
                self.assertIn("missing", r, n)
                continue
            game = self.game.generate(*ids, s)
            game.pop("wmFieldType")
            game.pop("wmWeather")
            self.assertEqual(r["generated"], game, (n, s))
            generated += 1
        print(f"\ninstruction 118: {len(parts)} word lookups, {generated} areas generated, 0 mismatches",
              file=sys.stderr)

    def test_set_generate_code(self):
        """Where a Chaos Gate's words send the party: the field of the area
        they make (story or random), or its dungeon where the field type has
        no field."""
        cases = [(a, b, c, server) for _, server, (a, b, c) in self.story_words()]
        rnd = random.Random(12)
        slots = [[w for w in self.data.words if w.slot == s] for s in range(3)]
        for _ in range(300):
            cases.append(tuple(rnd.choice(s).ID for s in slots) + (rnd.randrange(5),))
        got = ask([f"go {a} {b} {c} {s}" for a, b, c, s in cases])
        kinds = {}
        for case, r in zip(cases, got):
            game = self.game.go(*case)
            self.assertEqual(r, game, case)
            key = next(iter(game)) + ("" if next(iter(game.values()))[0] == 1 else " dungeon")
            kinds[key] = kinds.get(key, 0) + 1
        self.assertGreater(kinds.get("area dungeon", 0) + kinds.get("scene dungeon", 0), 0)
        print(f"\nSetGenerateCode: {len(cases)} word triples, 0 mismatches, {kinds}", file=sys.stderr)

    def test_word_id(self):
        texts = [w.text for w in self.data.words] + ["Nothing Here", ""]
        got = ask([f"wordid {t}" for t in texts])
        for t, r in zip(texts, got):
            self.assertEqual(r["id"], self.game.word_id(t), t)


if __name__ == "__main__":
    unittest.main()
