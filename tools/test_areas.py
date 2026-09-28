#!/usr/bin/env python3
"""tools/areas.py against the game's own WORLD_MAN::SimGenerateCode.

The game code is run in tools/eemu.py with a scratch WORLD_MAN, and every
field it writes is compared with areas.generate(). Infection in full; the
other three volumes, when extracted, each against their own executable
(through the names piney-gen syms carries). Skipped when an executable is
not present.
"""

import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
OTHERS = [os.path.join(ROOT, "work", v, "disc", e) for v, e in
          (("mutation", "SLUS_205.62"), ("outbreak", "SLUS_205.63"),
           ("quarantine", "SLUS_205.64"))]

WM, WP, GAME, SAVE = 0x01800000, 0x01801000, 0x01802000, 0x01810000


def run_game(eemu, areas, prog, a, b, c, server, flag71=False, volume=None):
    """WORLD_MAN::SimGenerateCode in eemu: the fields it writes."""
    sym = prog.symbol_named
    m = eemu.Machine(prog)
    m.store(sym("worldman").value, 4, WM)
    m.store(sym("game").value, 4, GAME)
    m.store(sym("saveData").value, 4, SAVE)
    m.store(GAME + 0x1c, 4, server)          # ccGame.server
    m.store(WM + 0x158, 4, WP)               # WORLD_MAN.wordparam
    if flag71:
        m.store(SAVE + 0x5bc0 + 4, 4, 0x40000000)    # bit 62 of the u64
    if volume is not None:
        m.store(sym("volumeNum").value, 4, volume)
    m.call(sym("SimGenerateCode__9WORLD_MANFiii").value, (WM, a, b, c))

    def rd(va):
        return struct.unpack("<i", bytes(m.mem[va:va + 4]))[0]
    out = {k: rd(WP + 0xc + 4 * i) for i, k in enumerate(areas.FIELDS[2:])}
    out.update(code=rd(WP + 4), event=rd(WM + 0x120), fieldSeed=rd(WM + 0x44) & 0xFFFFFFFF,
               levelMax=rd(WM + 0x138), roomMax=rd(WM + 0x13c), timeSym=rd(WM + 0x134))
    return out


def check_volume(test, eemu, areas, prog, data, randoms):
    """Every story area, events 71 and 47 under their special conditions, and
    `randoms` random keyword triples."""
    def check(a, b, c, server, flag71=False, volume=None):
        game = run_game(eemu, areas, prog, a.ID, b.ID, c.ID, server, flag71, volume)
        py = areas.generate(data, a.ID, b.ID, c.ID, server, volume, flag71)
        for k, v in game.items():
            test.assertEqual(v, py[k], f"{prog.path}: {a.text} {b.text} {c.text} "
                                       f"server {server} flag71 {flag71} volume {volume}: {k}")
    for e in data.events:
        if not e["has_words"]:
            continue
        try:
            words = [data.find(i, e[k]) for i, k in enumerate(("wordA", "wordB", "wordC"))]
        except SystemExit:
            continue    # e.g. Howling, not a keyword in this volume
        check(*words, e["server"])
        if e["code"] in (47, 71):
            check(*words, e["server"], flag71=True)
            check(*words, e["server"], flag71=True, volume=3)
            check(*words, e["server"], volume=4)
    rnd = random.Random(7)
    slots = [[w for w in data.words if w.slot == s] for s in range(3)]
    for _ in range(randoms):
        check(*(rnd.choice(s) for s in slots), rnd.randrange(5))


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
class TestAreas(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import areas
        import eemu
        from image import Program
        cls.areas = areas
        cls.eemu = eemu
        cls.prog = Program(ELF)
        cls.data = areas.Data(ELF)

    def test_first_area(self):
        r = self.areas.generate(self.data, "Bursting", "Passed Over", "Aqua Field", 0)
        self.assertEqual(r["code"], 13026)
        self.assertEqual(r["event"], 14)

    def test_counts(self):
        self.assertEqual(len(self.data.words), 305)
        self.assertEqual(len(self.data.events), 126)

    def test_areas_match_game(self):
        check_volume(self, self.eemu, self.areas, self.prog, self.data, 60)

    def test_field_attrib_matches_game(self):
        """areas.FIELD_ATTRIB against WORLD_MAN::GetFieldAttrb run in eemu."""
        fn = self.prog.symbol_named("GetFieldAttrb__9WORLD_MANFv").value
        for ft in range(11):
            for w in range(10):
                m = self.eemu.Machine(self.prog)
                m.store(WM + 0x158, 4, WP)
                m.store(WP + 0xc, 4, ft)
                m.store(WP + 0x14, 4, w)
                self.assertEqual(m.call(fn, (WM,)), self.areas.FIELD_ATTRIB[ft][w], (ft, w))

    def test_first_area_enemies(self):
        r = self.areas.generate(self.data, "Bursting", "Passed Over", "Aqua Field", 0)
        e = self.areas.enemies(self.data, r, 0)
        self.assertEqual((e["listType"], e["rank"]), (6, 0))
        self.assertEqual([n for _, n in e["enemies"]], ["Goblin", "Mad Grass", "Disco Knife"])

    def test_no_substitutes(self):
        self.assertEqual(self.data.volume, 1)
        self.assertEqual(self.data.substitutes, {})


class TestOtherVolumes(unittest.TestCase):
    """Mutation, Outbreak and Quarantine, each against its own
    SimGenerateCode. From Mutation on, events 71 and 47 come from substitute
    records in ccGetEventAreaInfo; event 47's differ from Infection's inline
    values."""

    def test_volumes(self):
        import areas
        import eemu
        from image import Program
        present = [p for p in OTHERS if os.path.exists(p) and os.path.exists(p + ".syms")]
        if not present:
            self.skipTest("no other volume extracted with a .syms sidecar")
        for path in present:
            with self.subTest(path=path):
                prog = Program(path)
                data = areas.Data(path)
                self.assertEqual(data.volume, OTHERS.index(path) + 2)
                self.assertEqual({k: (e["enemy"], e["item"]) for k, e in data.substitutes.items()},
                                 {71: (119, 119), 47: (42, 122)})
                check_volume(self, eemu, areas, prog, data, 20)


if __name__ == "__main__":
    unittest.main()
