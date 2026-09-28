#!/usr/bin/env python3
"""tools/tables.py on real tables; skipped when the executable is absent."""

import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ELF = os.path.join(os.path.dirname(HERE), "work", "infection", "disc", "SLUS_202.67")


def dump(name, overlay, limit=None):
    cmd = [sys.executable, os.path.join(HERE, "tables.py"), "dump", ELF, name,
           "--overlay", overlay, "--tsv"]
    if limit:
        cmd += ["--limit", str(limit)]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True).stdout
    lines = out.splitlines()
    cols = lines[0].split("\t")
    return [dict(zip(cols, ln.split("\t"))) for ln in lines[1:]]


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
class TestTables(unittest.TestCase):
    def test_items(self):
        rows = dump("itemTblE", "gcmn")
        self.assertEqual(len(rows), 291)
        self.assertEqual(rows[0]["name"], "Virus Core A")

    def test_characters(self):
        rows = dump("charTbl", "demo")
        self.assertEqual(len(rows), 18)
        self.assertEqual((rows[0]["base.name"], rows[0]["base.ccsname"], rows[0]["maxHP"]),
                         ("Kite", "ctu1body", "63"))

    def test_enemies(self):
        rows = dump("enemyTbl", "gcmn", limit=3)
        self.assertEqual([r["param.base.name"] for r in rows],
                         ["Razine", "Swordmanoid", "Gladiator"])


if __name__ == "__main__":
    unittest.main()
