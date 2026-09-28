#!/usr/bin/env python3
"""Tests for tools/demangle.py. Run: python3 tools/test_demangle.py"""

import unittest

from demangle import cdecl, demangle, parse

# Real names from SLUS_202.67 (.hack//Infection), with what they must become.
CASES = {
    "ccMalloc__FUi": "ccMalloc(unsigned int)",
    "InitFirst__6ccHeapFv": "ccHeap::InitFirst(void)",
    "SearchFree__10ccHeapFreeFUi": "ccHeapFree::SearchFree(unsigned int)",
    "_ccMalloc__FUiUiUii": "_ccMalloc(unsigned int, unsigned int, unsigned int, int)",
    "ccPrtChunkName__FP12ccChunkIndex": "ccPrtChunkName(ccChunkIndex *)",
    "ccCmpChunkPathName__FPCcPCcP12ccChunkIndexi":
        "ccCmpChunkPathName(const char *, const char *, ccChunkIndex *, int)",
    "ccSetViewScreenClipMatrix__FPA4_fPA4_fPA4_ffffffffff":
        "ccSetViewScreenClipMatrix(float (*)[4], float (*)[4], float (*)[4], float, "
        "float, float, float, float, float, float, float, float)",
    "ccRotate__FRff": "ccRotate(float &, float)",
    "SetAnalogStick__FPfPUcii": "SetAnalogStick(float *, unsigned char *, int, int)",
    "ccPadThread__FP6ccTscb": "ccPadThread(ccTscb *)",
    "__ct__13ccFrameBufferFv": "ccFrameBuffer::ccFrameBuffer(void)",
    "GetFreeWork__12ccDrawPacketFiRP15ccDrawPacketTag":
        "ccDrawPacket::GetFreeWork(int, ccDrawPacketTag *&)",
    "ccMatchIndex__FRC17ccSearchChunkNamePC12ccChunkIndex":
        "ccMatchIndex(const ccSearchChunkName &, const ccChunkIndex *)",
    "__ct__Q218ccBossEffRockTower7TOWER_TFv":
        "ccBossEffRockTower::TOWER_T::TOWER_T(void)",
    "CalcSplinePoint__20ccDarkSummonsElementFPQ220ccDarkSummonsElement6BALL_T":
        "ccDarkSummonsElement::CalcSplinePoint(ccDarkSummonsElement::BALL_T *)",
    "__nw__FUi": "operator new(unsigned int)",
    "__dl__FPv": "operator delete(void *)",
    "Spline__18HellSpecialExpressFPfPfPfPfPff":
        "HellSpecialExpress::Spline(float *, float *, float *, float *, float *, float)",
    # templates, spelled inline with the length covering the arguments
    "at__Q23std38__vector_pod<Ui,Q23std13allocator<Ui>>FUi":
        "std::__vector_pod<unsigned int, std::allocator<unsigned int>>::at(unsigned int)",
    "__init__id__Q23std8ctype<w>": "std::ctype<wchar_t>::__init__id",
    # data members and compiler-generated objects
    "heapTop__6ccHeap": "ccHeap::heapTop",
    "__vt__13ccSysMenuItem": "vtable for ccSysMenuItem",
    "__RTTI__Q23std9exception": "RTTI for std::exception",
    "what__Q23std9exceptionCFv": "std::exception::what(void) const",
    "__dt__Q23std9exceptionFv": "std::exception::~exception(void)",
    # u_long128 mangles as a length-1 name with no characters
    "SetDataAdrs__9ccBltDataFP1": "ccBltData::SetDataAdrs(u_long128 *)",
    "videoDecCreate__FP8VideoDecPUciP1P1iP9TimeStampi":
        "videoDecCreate(VideoDec *, unsigned char *, int, u_long128 *, u_long128 *, "
        "int, TimeStamp *, int)",
    # a genuine one-letter class still works
    "think__8ccEnemy1Fv": "ccEnemy1::think(void)",
}

SYNTHETIC = {
    "f__FPFiPc_v": "f(void (*)(int, char *))",
    "g__FPCPCc": "g(const char *const *)",
    "h__FM5ClassFi_v": "h(void (Class::*)(int))",
    "x__Fie": "x(int, ...)",
    "__opi__5ClassCFv": "Class::operator int(void) const",
    "__apl__5ClassFRC5Class": "Class::operator+=(const Class &)",
    "__vc__5ClassFi": "Class::operator[](int)",
    "k__FiT1N21": "k(int, int, int, int)",
    "foo___Fv": "foo_(void)",
    "q__FSc": "q(signed char)",
    "r__FRA3_i": "r(int (&)[3])",
}

UNMANGLED = ["main", "sceGsSyncV", "__sinit_system.cpp", "@1000", "$L10",
             "__exception_table_end__", "_impure_ptr", "__divdi3", ".p__sinit_menu.cpp"]


class DemangleTest(unittest.TestCase):
    def test_real_names(self):
        for m, want in CASES.items():
            with self.subTest(m=m):
                self.assertEqual(demangle(m), want)

    def test_synthetic(self):
        for m, want in SYNTHETIC.items():
            with self.subTest(m=m):
                self.assertEqual(demangle(m), want)

    def test_unmangled_pass_through(self):
        for n in UNMANGLED:
            with self.subTest(n=n):
                self.assertEqual(demangle(n), n)
                self.assertIsNone(parse(n))

    def test_parts(self):
        s = parse("GetFreeWork__12ccDrawPacketFiRP15ccDrawPacketTag")
        self.assertEqual(s.classname, "ccDrawPacket")
        self.assertEqual(s.name, "GetFreeWork")
        self.assertEqual(s.qualname, "ccDrawPacket::GetFreeWork")
        self.assertTrue(s.is_func)
        self.assertEqual([cdecl(t) for t in s.params], ["int", "ccDrawPacketTag *&"])
        s = parse("heapTop__6ccHeap")
        self.assertFalse(s.is_func)

    def test_cdecl(self):
        f = ("func", ("base", "int"), [("base", "char")], False)
        self.assertEqual(cdecl(("ptr", f), "fp"), "int (*fp)(char)")
        self.assertEqual(cdecl(("array", 4, ("ptr", f)), "tbl"), "int (*tbl[4])(char)")
        self.assertEqual(cdecl(("cv", "const", ("ptr", ("base", "char"))), "p"),
                         "char *const p")
        self.assertEqual(cdecl(("ptr", ("array", 4, ("base", "float")))), "float (*)[4]")
        self.assertEqual(cdecl(("array", 2, ("array", 3, ("base", "int"))), "m"),
                         "int m[2][3]")


if __name__ == "__main__":
    unittest.main()
