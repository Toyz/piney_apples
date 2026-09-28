#!/usr/bin/env python3
"""Tests for tools/dwarf1.py against the retail .hack//Infection ELF.
Skipped when work/infection/disc/SLUS_202.67 is absent.
Run: python3 tools/test_dwarf1.py"""

import os
import unittest

from demangle import cdecl
import dwarf1

ELF = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "work",
                   "infection", "disc", "SLUS_202.67")


@unittest.skipUnless(os.path.exists(ELF), "needs work/infection/disc/SLUS_202.67")
class Dwarf1Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.dw = dwarf1.Dwarf(ELF)

    def test_tree(self):
        dw = self.dw
        self.assertEqual(dw.problems, [])
        self.assertEqual(len(dw.dies), 336743)
        self.assertEqual(dw.nulls, 39462)
        self.assertEqual(len(dw.cus), 4612)
        self.assertEqual(len(dw.tus), 216)
        self.assertEqual(len(dw.funcs), 4396)

    def test_overlays(self):
        dw = self.dw
        self.assertEqual(dw.overlay_names,
                         ["main", "gcmn.prg", "demo.prg", "desktop.prg", "toppage.prg"])
        # Every CU with code at the overlay address is claimed by an overlay.
        for cu in dw.cus:
            lo = cu.attrs.get(dwarf1.AT_low_pc)
            if lo is not None:
                self.assertEqual(lo >= 0x400800, dw.overlay(cu) != "main", hex(cu.off))

    def test_functions_match_symbols(self):
        dw = self.dw
        idx = dw.elf_symbols()
        for f in dw.funcs:
            lo, hi = f.attrs[dwarf1.AT_low_pc], f.attrs[dwarf1.AT_high_pc]
            names = {f.name, dw.mangled(f)}
            ok = [s for s in idx.get((dw.overlay(f), lo), [])
                  if s.name in names and s.size == hi - lo]
            self.assertTrue(ok, f"{f!r} 0x{lo:x}")

    def test_lines(self):
        dw = self.dw
        (cu,) = dw.cus_at(0x00102680)
        self.assertTrue(cu.name.endswith("syspad.cpp"))
        self.assertEqual(dw.line_for(cu, 0x00102680), (0x00102680, 214))

    def test_function(self):
        dw = self.dw
        (f,) = dw.names()["mwLoadOverlay"]
        self.assertEqual(dw.prototype(f), "int mwLoadOverlay(char *pFilePath, void *pAddress)")
        (f,) = dw.names()["Ctrl__5ccPadFv"]
        self.assertEqual(dw.prototype(f), "void ccPad::Ctrl(void)")
        self.assertTrue(dw.has_this(f))
        # a dropped (unused) parameter is restored from the mangled name
        (f,) = dw.names()["_ccMalloc__FUiUiUii"]
        self.assertIn("unsigned int /* not in DWARF */", dw.prototype(f))

    def test_type(self):
        dw = self.dw
        d = dw.aggregates("ccHeap")[0]
        self.assertEqual(d.attrs[dwarf1.AT_byte_size], 0x10)
        mems = [(c.name, dwarf1.member_offset(c), cdecl(dw.type_of(c)))
                for c in d.children if c.tag == dwarf1.TAG_member]
        self.assertEqual(mems, [("before", 0, "ccHeap *"), ("next", 4, "ccHeap *"),
                                ("size", 8, "unsigned int")])
        # bitfields: LSB-first offsets (GS PMODE.EN1 is bit 0)
        d = dw.aggregates("tGS_PMODE")[0]
        en1 = next(c for c in d.children if c.name == "EN1")
        self.assertEqual((en1.attrs[dwarf1.AT_bit_offset], en1.attrs[dwarf1.AT_bit_size]), (0, 1))

    def test_render_union_inline(self):
        dw = self.dw
        text = "\n".join(dw.render_aggregate(dw.aggregates("_reent")[0]))
        self.assertIn("union {", text)
        self.assertIn("} _new;", text)


if __name__ == "__main__":
    unittest.main()
