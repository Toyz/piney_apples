"""tools/tables.py on real tables of PINEY_VOLUME's executable (volume.py;
Infection by default), skipped when it is absent. A later volume has no
DWARF: tables.py takes Infection's types and the volume's own placement
and layout of the table (its `.syms` sidecar, volume.carried_rows,
which takes a count the code states from the code).
Run: PINEY_VOLUME=<volume> python3 tools/test_tables_rt.py"""

import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)

ELF = volume.ELF
# charTbl's rows: 18 characters on Infection, 21 from Mutation on (the
# extension's Tsukasa, Subaru and Sora); Kite's level at the start.
CHARACTERS = ["Kite", "Mia", "Orca", "Marlo", "Sanjuro", "Nuke Usagimaru", "Balmung", "Moonstone", "Piros",
              "Wiseman", "Elk", "Natsume", "Rachel", "Gardenia", "Terajima Ryoko", "BlackRose", "Mistral",
              "Helba"]
LATER = CHARACTERS + ["Tsukasa", "Subaru", "Sora"]
CHARS = {"infection": (CHARACTERS, "1"), "mutation": (LATER, "30"), "outbreak": (LATER, "50"),
         "quarantine": (LATER, "70")}


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
        self.assertEqual((rows[0]["name"], rows[-1]["name"]), ("Virus Core A", "M:Long Arm"))

    def test_characters(self):
        names, level = CHARS[volume.NAME]
        # A later volume's count is the one NewGame's loop states
        # (volume.WALKERS), the manifest's (piney-gen).
        rows = dump("charTbl", "demo")
        self.assertEqual([r["base.name"] for r in rows], names)
        self.assertEqual((rows[0]["base.ccsname"], rows[0]["base.level"], rows[0]["maxHP"] != ""),
                         ("ctu1body", level, True))

    def test_enemies(self):
        rows = dump("enemyTbl", "gcmn", limit=3)
        self.assertEqual([r["param.base.name"] for r in rows],
                         ["Razine", "Swordmanoid", "Gladiator"])


if __name__ == "__main__":
    unittest.main()
