#!/usr/bin/env python3
"""tools/save.py: each volume's own memory-card code run in tools/eemu.py
against a card kept in Python. Infection always (when extracted); the other
volumes, and the carry-over chain between them, when extracted with their
`.syms` sidecar (piney-gen syms)."""

import os
import struct
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ROOT = os.path.dirname(HERE)
VOLUMES = [("infection", "SLUS_202.67", "BASLUS-20267DOTHACK", "INFECTION"),
           ("mutation", "SLUS_205.62", "BASLUS-20562DOTHACK", "MUTATION"),
           ("outbreak", "SLUS_205.63", "BASLUS-20563DOTHACK", "OUTBREAK"),
           ("quarantine", "SLUS_205.64", "BASLUS-20564DOTHACK", "QUARANTINE")]


def elf(i):
    path = os.path.join(ROOT, "work", VOLUMES[i][0], "disc", VOLUMES[i][1])
    ok = os.path.exists(path) and (i == 0 or os.path.exists(path + ".syms"))
    return path if ok else None


def fullwidth(text):
    return "．ｈａｃｋ　" + "".join(chr(ord(c) + 0xfee0) for c in text)


class TestCard(unittest.TestCase):
    """MakeDir, then one save: the files, the slot record and its checksum."""

    def check_volume(self, i):
        import save
        path = elf(i)
        if path is None:
            self.skipTest(f"{VOLUMES[i][0]} not extracted")
        g, made, result = save.run_card(path, slot=2)
        self.assertEqual((made, result), (0, 0x101c))
        d = "/" + VOLUMES[i][2]
        size = 0x8530 if i == 0 else 0x8530 + 0x854
        ext = "ico" if i == 0 else "icn"
        files = {p: len(v) for p, v in g.card.files.items()}
        expect = {f"{d}{d}": 336, f"{d}/icon.sys": 964}
        expect.update({f"{d}/dhdata{k:02d}": size for k in range(1, 13)})
        self.assertEqual({p: n for p, n in files.items() if "icon" not in p or p.endswith("sys")},
                         expect)
        self.assertEqual(sorted(p for p in files if p.endswith(ext)),
                         [f"{d}/icon{i + 1}{k}.{ext}" for k in range(3)])
        info = save.icon_sys(bytes(g.card.files[f"{d}/icon.sys"]))
        self.assertEqual((info["magic"], info["title"], info["title_break"]),
                         ("PS2D", fullwidth(VOLUMES[i][3]), 10))
        self.assertEqual(info["list"], f"icon{i + 1}0.{ext}")
        # Slot 2 is dhdata03; its record holds the file's 16-bit byte sum.
        data = bytes(g.card.files[f"{d}/dhdata03"])
        self.assertEqual(data, g.save_bytes())
        record = bytes(g.card.files[f"{d}{d}"][2 * 28:3 * 28])
        self.assertEqual(record[:8], bytes([1, 1, 0, 0]) + b"Kite")
        self.assertEqual(struct.unpack_from("<H", record, 0x16)[0], sum(data) & 0xffff)
        # And the volume's own load reads it back.
        g2 = save.Game(path, save.Card(g.card.files))
        self.assertEqual(g2.request("LoadInfoReq__9ccSaveSysFi", 0) & 0xfff, 1)
        g2.request("LoadDataReq__9ccSaveSysFi", 2)
        loaded = g2.save_bytes()
        pads = {0x221e, 0x221f, 0x7486, 0x7487, 0x852c, 0x852d, 0x852e, 0x852f}
        self.assertEqual([k for k in range(len(data)) if loaded[k] != data[k]
                          and k not in pads and not (i and k - 0x8530 in (0x49b, 0x751, 0x752, 0x753))],
                         [])

    def test_infection(self):
        self.check_volume(0)

    def test_mutation(self):
        self.check_volume(1)

    def test_outbreak(self):
        self.check_volume(2)

    def test_quarantine(self):
        self.check_volume(3)


class TestCarry(unittest.TestCase):
    """Loading the previous volume's save: which file, how many bytes, where
    they land, and what ccStartEventConvert and ConvGame then change."""

    def test_extension(self):
        import save
        from image import Program
        for i in range(4):
            path = elf(i)
            if path is None:
                continue
            with self.subTest(volume=VOLUMES[i][0]):
                ext = save.find_extension(Program(path))
                self.assertEqual(ext and ext[1], None if i == 0 else 0x854)

    def test_paths(self):
        import save
        seen = 0
        for i in range(4):
            path = elf(i)
            if path is None:
                continue
            seen += 1
            with self.subTest(volume=VOLUMES[i][0]):
                index, data, size = save.learn_previous(path, 3)
                prev = VOLUMES[max(i - 1, 0)][2]     # Infection "carries" from itself
                self.assertEqual((index, data), (f"/{prev}/{prev}", f"/{prev}/dhdata04"))
                self.assertEqual(size, 0x8530 if i <= 1 else 0x8d84)
        if not seen:
            self.skipTest("no volume extracted")

    def test_mutation_from_infection(self):
        import save
        path = elf(1)
        if path is None:
            self.skipTest("mutation not extracted")
        out = save.carry(path).splitlines()
        self.assertIn("  saveData+0x0000..0x221e  <- file 0x0000..0x221e", out)
        self.assertIn("  saveData+0x7488..0x852c  <- file 0x7488..0x852c", out)
        self.assertIn("  ext+0x0000..0x0854  not written", out)
        self.assertIn("ccStartEventConvert: +0x581f (eventFlag[100] byte 7)", out)

    def test_outbreak_from_mutation(self):
        import save
        path = elf(2)
        if path is None:
            self.skipTest("outbreak not extracted")
        out = save.carry(path).splitlines()
        self.assertIn("  ext+0x0000..0x049b  <- file 0x8530..0x89cb", out)
        self.assertIn("  ext+0x0754..0x0854  <- file 0x8c84..0x8d84", out)
        self.assertIn("ccStartEventConvert: +0x5b3f (eventFlag[200] byte 7)", out)

    def test_chain(self):
        # Each volume's code writes a save that the next volume's code loads.
        import save
        for i in range(1, 4):
            if elf(i) is None or elf(i - 1) is None:
                continue
            with self.subTest(volume=VOLUMES[i][0]):
                out = save.carry(elf(i), prev=elf(i - 1))
                self.assertTrue(out.endswith("(padding aside): True"), out)


if __name__ == "__main__":
    unittest.main()
