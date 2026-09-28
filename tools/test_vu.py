#!/usr/bin/env python3
"""Tests for tools/vu.py and tools/vif.py.

    python3 tools/test_vu.py [-v]

Instruction vectors are hand-assembled from the VU User's Manual field
layouts with the helpers below; the ones marked "blob" are words taken from
the VU1 microcode in INF SLUS_202.67 and agree with what the helpers build.
Tests over the real microcode are skipped when work/infection/disc/SLUS_202.67
is absent.
"""

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402
import vu  # noqa: E402
import vif  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
XYZW, XYZ, XYW, XZW, XY, X, Y, Z, W = 15, 14, 13, 11, 12, 8, 4, 2, 1


def up(op, dest=XYZW, ft=0, fs=0, fd=0):
    return (dest << 21) | (ft << 16) | (fs << 11) | (fd << 6) | op


def up2(ext, dest=XYZW, ft=0, fs=0):
    return (dest << 21) | (ft << 16) | (fs << 11) | ((ext >> 2) << 6) | 0x3c | (ext & 3)


def lo(op7, dest=0, t=0, s=0, low11=0):
    return (op7 << 25) | (dest << 21) | (t << 16) | (s << 11) | (low11 & 0x7ff)


def lo40(sub, dest=0, t=0, s=0, d=0):
    return (0x40 << 25) | (dest << 21) | (t << 16) | (s << 11) | (d << 6) | sub


def lo2(ext, dest=0, t=0, s=0):
    return (0x40 << 25) | (dest << 21) | (t << 16) | (s << 11) | ((ext >> 2) << 6) | 0x3c | (ext & 3)


UPPER = {
    0x000002ff: "nop",                                              # blob
    up(0x00, XYZW, 3, 2, 1): "addx.xyzw   vf01, vf02, vf03x",
    up(0x07, XYZ, 3, 2, 1): "subw.xyz    vf01, vf02, vf03w",
    up(0x0a, XYZW, 3, 2, 1): "maddz.xyzw  vf01, vf02, vf03z",
    up(0x0d, X, 3, 2, 1): "msuby.x     vf01, vf02, vf03y",
    up(0x10, XYZ, 0, 21, 21): "maxx.xyz    vf21, vf21, vf00x",
    up(0x15, W, 15, 18, 18): "miniy.w     vf18, vf18, vf15y",
    up(0x1b, XYZ, 3, 2, 1): "mulw.xyz    vf01, vf02, vf03w",
    up(0x1c, XYZ, 0, 20, 20): "mulq.xyz    vf20, vf20, Q",
    up(0x1d, XYZ, 0, 2, 1): "maxi.xyz    vf01, vf02, I",
    up(0x1e, XYZ, 0, 9, 9): "muli.xyz    vf09, vf09, I",
    up(0x1f, XYZ, 0, 21, 21): "minii.xyz   vf21, vf21, I",
    up(0x21, XYZW, 0, 21, 21): "maddq.xyzw  vf21, vf21, Q",
    up(0x23, XYZW, 0, 12, 21): "maddi.xyzw  vf21, vf12, I",
    up(0x26, XY, 0, 2, 1): "subi.xy     vf01, vf02, I",
    up(0x27, W, 0, 2, 1): "msubi.w     vf01, vf02, I",
    up(0x28, XYZW, 3, 2, 1): "add.xyzw    vf01, vf02, vf03",
    up(0x29, XYZ, 7, 7, 17): "madd.xyz    vf17, vf07, vf07",
    up(0x2a, XYZ, 17, 5, 5): "mul.xyz     vf05, vf05, vf17",
    up(0x2b, XYZW, 3, 2, 1): "max.xyzw    vf01, vf02, vf03",
    up(0x2c, W, 9, 9, 9): "sub.w       vf09, vf09, vf09",
    up(0x2d, XYZ, 3, 2, 1): "msub.xyz    vf01, vf02, vf03",
    up(0x2e, XYZ, 3, 2, 1): "opmsub.xyz  vf01, vf02, vf03",
    up(0x2f, XYZW, 3, 2, 1): "mini.xyzw   vf01, vf02, vf03",
    up2(0x00, XYZW, 0, 20): "addax.xyzw  ACC, vf20, vf00x",
    up2(0x07, W, 16, 20): "subaw.w     ACC, vf20, vf16w",
    up2(0x08, XYZW, 20, 1): "maddax.xyzw ACC, vf01, vf20x",
    up2(0x0e, XYZ, 20, 29): "msubaz.xyz  ACC, vf29, vf20z",
    up2(0x10, XYZW, 21, 21): "itof0.xyzw  vf21, vf21",
    up2(0x11, XYZ, 1, 2): "itof4.xyz   vf01, vf02",
    up2(0x12, XYZ, 20, 20): "itof12.xyz  vf20, vf20",
    up2(0x13, XYZ, 1, 2): "itof15.xyz  vf01, vf02",
    up2(0x14, XYZW, 21, 21): "ftoi0.xyzw  vf21, vf21",
    up2(0x15, XYZ, 20, 20): "ftoi4.xyz   vf20, vf20",
    up2(0x16, XY, 1, 2): "ftoi12.xy   vf01, vf02",
    up2(0x17, X, 1, 2): "ftoi15.x    vf01, vf02",
    up2(0x19, XYZW, 20, 1): "mulay.xyzw  ACC, vf01, vf20y",
    up2(0x1c, XYZ, 0, 2): "mulaq.xyz   ACC, vf02, Q",
    up2(0x1d, XYZ, 1, 2): "abs.xyz     vf01, vf02",
    up2(0x1e, XYZ, 0, 2): "mulai.xyz   ACC, vf02, I",
    up2(0x1f, XYZ, 2, 1): "clipw.xyz   vf01, vf02w",
    up2(0x20, XYZ, 0, 2): "addaq.xyz   ACC, vf02, Q",
    up2(0x21, XYZ, 0, 2): "maddaq.xyz  ACC, vf02, Q",
    up2(0x22, XYZ, 0, 2): "addai.xyz   ACC, vf02, I",
    up2(0x23, XYZ, 0, 2): "maddai.xyz  ACC, vf02, I",
    up2(0x24, XYZ, 0, 2): "subaq.xyz   ACC, vf02, Q",
    up2(0x25, XYZ, 0, 2): "msubaq.xyz  ACC, vf02, Q",
    up2(0x26, XYZ, 0, 2): "subai.xyz   ACC, vf02, I",
    up2(0x27, XYZ, 0, 2): "msubai.xyz  ACC, vf02, I",
    up2(0x28, XYZW, 3, 2): "adda.xyzw   ACC, vf02, vf03",
    up2(0x29, XYZ, 6, 6): "madda.xyz   ACC, vf06, vf06",
    up2(0x2a, XYZ, 5, 5): "mula.xyz    ACC, vf05, vf05",
    up2(0x2c, XYW, 20, 13): "suba.xyw    ACC, vf13, vf20",
    up2(0x2d, XYZ, 3, 2): "msuba.xyz   ACC, vf02, vf03",
    up2(0x2e, XYZ, 3, 2): "opmula.xyz  ACC, vf02, vf03",
}

LOWER = {
    0x8000033c: "nop",                                              # blob
    lo(0x00, XYZW, 1, 0, 8): "lq.xyzw     vf01, 8(vi00)",
    lo(0x00, XZW, 18, 2, 0): "lq.xzw      vf18, 0(vi02)",
    0x03e1c807: "sq.xyzw     vf25, 7(vi01)",                        # blob
    lo(0x01, XYZ, 4, 22, -3): "sq.xyz      vf22, -3(vi04)",
    0x08211000: "ilw.w       vi01, 0(vi02)",                        # blob
    lo(0x05, W, 1, 4, 3): "isw.w       vi01, 3(vi04)",
    0x11e107ff: "iaddiu      vi01, vi00, 0x7fff",                   # blob
    0x100e0277: "iaddiu      vi14, vi00, 0x277",                    # blob
    lo(0x09, 0xf, 12, 12, 0x731): "isubiu      vi12, vi12, 0x7f31",
    lo(0x10) | 0x0000c0: "fceq        vi01, 0x0000c0",
    lo(0x11) | 0x123456: "fcset       0x123456",
    lo(0x12) | 0x000fff: "fcand       vi01, 0x000fff",
    lo(0x13) | 0xfff000: "fcor        vi01, 0xfff000",
    lo(0x14, 0, 1, 0, 0x020): "fseq        vi01, 0x020",
    lo(0x15, 1, 0, 0, 0x001): "fsset       0x801",
    0x2c010002: "fsand       vi01, 0x002",                          # blob
    lo(0x17, 0, 3, 0, 0x080): "fsor        vi03, 0x080",
    lo(0x18, 0, 1, 13): "fmeq        vi01, vi13",
    0x34016800: "fmand       vi01, vi13",                           # blob
    lo(0x1b, 0, 1, 2): "fmor        vi01, vi02",
    lo(0x1c, 0, 5): "fcget       vi05",
    lo(0x2c, 0, 0, 11, 5): "ibltz       vi11, 0x106",
    lo(0x2d, 0, 0, 11, 5): "ibgtz       vi11, 0x106",
    lo(0x2e, 0, 0, 11, 5): "iblez       vi11, 0x106",
    lo(0x2f, 0, 0, 11, 5): "ibgez       vi11, 0x106",
    lo(0x28, 0, 6, 0, 0x7fe): "ibeq        vi06, vi00, 0x0ff",
    lo(0x25, 0, 15, 14): "jalr        vi15, vi14",
    lo40(0x30, 0, 7, 8, 9): "iadd        vi09, vi08, vi07",
    lo40(0x31, 0, 9, 12, 1): "isub        vi01, vi12, vi09",
    lo40(0x32, 0, 1, 1, 0x1f): "iaddi       vi01, vi01, -1",
    lo40(0x34, 0, 1, 8, 9): "iand        vi09, vi08, vi01",
    lo40(0x35, 0, 10, 11, 11): "ior         vi11, vi11, vi10",
    lo2(0x30, W, 21, 16): "move.w      vf21, vf16",
    lo2(0x31, XYZW, 1, 2): "mr32.xyzw   vf01, vf02",
    lo2(0x34, XYZW, 20, 2): "lqi.xyzw    vf20, (vi02++)",
    0x81e4937d: "sqi.xyzw    vf18, (vi04++)",                       # blob
    lo2(0x36, XYZW, 1, 2): "lqd.xyzw    vf01, (--vi02)",
    lo2(0x37, XYZW, 3, 2): "sqd.xyzw    vf02, (--vi03)",
    0x81f403bc: "div         Q, vf00w, vf20w",                      # blob
    lo2(0x38, 0b0110, 17, 17): "div         Q, vf17z, vf17y",
    lo2(0x39, 0b0100, 5): "sqrt        Q, vf05y",
    lo2(0x3a, 0b1001, 2, 1): "rsqrt       Q, vf01y, vf02z",
    0x800003bf: "waitq",                                            # blob
    0x804f83fc: "mtir        vi15, vf16z",                          # blob
    lo2(0x3d, XYZW, 3, 4): "mfir.xyzw   vf03, vi04",
    lo2(0x3e, X, 5, 6): "ilwr.x      vi05, (vi06)",
    lo2(0x3f, Y, 5, 6): "iswr.y      vi05, (vi06)",
    lo2(0x40, XYZW, 7): "rnext.xyzw  vf07, R",
    lo2(0x41, X, 7): "rget.x      vf07, R",
    lo2(0x42, 0b0010, 0, 8): "rinit       R, vf08z",
    lo2(0x43, 0b0011, 0, 8): "rxor        R, vf08w",
    0x8111067c: "mfp.x       vf17, P",                              # blob
    0x800206bc: "xtop        vi02",                                 # blob
    lo2(0x69, 0, 1): "xitop       vi01",
    0x80002efc: "xgkick      vi05",                                 # blob
    lo2(0x70, 0, 0, 3): "esadd       P, vf03",
    lo2(0x71, 0, 0, 3): "ersadd      P, vf03",
    lo2(0x72, 0, 0, 3): "eleng       P, vf03",
    lo2(0x73, 0, 0, 3): "erleng      P, vf03",
    lo2(0x74, 0, 0, 3): "eatanxy     P, vf03",
    lo2(0x75, 0, 0, 3): "eatanxz     P, vf03",
    lo2(0x76, 0, 0, 3): "esum        P, vf03",
    lo2(0x78, 0b0001, 0, 3): "esqrt       P, vf03y",
    0x80008fbd: "ersqrt      P, vf17x",                             # blob
    lo2(0x7a, 0b0011, 0, 3): "ercpr       P, vf03w",
    lo2(0x7b): "waitp",
    lo2(0x7c, 0, 0, 3): "esin        P, vf03x",
    lo2(0x7d, 0, 0, 3): "eatan       P, vf03x",
    lo2(0x7e, 0, 0, 3): "eexp        P, vf03x",
}


class TestDecode(unittest.TestCase):
    def test_upper(self):
        for w, text in UPPER.items():
            with self.subTest(w=hex(w)):
                op = vu.upper(w)
                self.assertTrue(op.valid, text)
                self.assertEqual(op.text, text)

    def test_lower(self):
        for w, text in LOWER.items():
            with self.subTest(w=hex(w)):
                op = vu.lower(w, 0x100)
                self.assertTrue(op.valid, text)
                self.assertEqual(op.text, text)

    def test_branches(self):
        # blob words: targets are pc + 1 + imm11
        self.assertEqual(vu.lower(0x520307dc, 0x5e).target, 0x3b)   # ibne back
        self.assertEqual(vu.lower(0x40000009, 0x27a).target, 0x284)  # b
        b = vu.lower(0x420f005a, 0x1ef)                               # bal vi15
        self.assertEqual((b.mnemonic, b.target, b.kind), ("bal", 0x24a, "call"))
        self.assertEqual(vu.lower(0x48007000).kind, "jr")

    def test_flags_and_loi(self):
        p = vu.decode(0x40000000, 0x81c73c69, 0x13)                   # blob
        self.assertEqual(p.flags, "I")
        self.assertEqual(p.upper.text, "madd.xyz    vf17, vf07, vf07")
        self.assertEqual((p.lower.mnemonic, p.lower.imm), ("loi", 2.0))
        p = vu.decode(0x8000033c, 0x400002ff)
        self.assertTrue(p.end)
        self.assertEqual(p.text(0), "nop[E] nop")
        for bit, f in zip(range(31, 26, -1), "IEMDT"):
            self.assertEqual(vu.decode(0x8000033c, (1 << bit) | 0x2ff).flags, f)

    def test_invalid(self):
        self.assertFalse(vu.upper(0x000002ff | (1 << 25)).valid)
        self.assertFalse(vu.upper(up(0x30)).valid)
        self.assertFalse(vu.upper(up2(0x2b)).valid)
        self.assertFalse(vu.lower(0x7e000000).valid)
        self.assertFalse(vu.lower(lo40(0x33)).valid)
        self.assertFalse(vu.lower(lo2(0x44)).valid)
        self.assertFalse(vu.lower(lo(0x20, 0, 1)).valid)   # b with a register
        self.assertFalse(vu.lower(lo2(0x3b, 0, 1)).valid)  # waitq with a register


class TestVif(unittest.TestCase):
    def test_codes(self):
        words = [0x01000104, 0x05000001, 0x6530c00b] + [0] * 48 + \
                [0x20000000, 0x3f3f3f3f, 0x30000000, 1, 2, 3, 4, 0x14000026]
        data = struct.pack(f"<{len(words)}I", *words)
        got = [(v.name, v.text) for v in vif.codes(data)]
        self.assertEqual(got[0][1], "STCYCL cl=4 wl=1")
        self.assertEqual(got[1][1], "STMOD offset")
        self.assertEqual(got[2][1], "UNPACK V2-16 usn flg addr=0x00b num=48 (192 bytes)")
        self.assertEqual(got[3][1], "STMASK 0x3f3f3f3f pppd/pppd/pppd/pppd")
        self.assertTrue(got[4][1].startswith("STROW 0x00000001"))
        self.assertEqual(got[5][1], "MSCAL 0x0026")

    def test_fill_mode(self):
        # STCYCL cl=1 wl=4: only one of every four written vectors is read
        data = struct.pack("<II", 0x01000401, 0x6c080000) + bytes(32)
        v = list(vif.codes(data))[1]
        self.assertEqual(len(v.data), 32)

    def test_mpg_alignment(self):
        data = struct.pack("<IIII", 0, 0x4a010010, 0x8000033c, 0x000002ff)
        v = list(vif.codes(data))[1]
        self.assertEqual((v.text, v.data[:4]), ("MPG 1 instructions at 0x010", b"\x3c\x03\x00\x80"))

    def test_unpack_values(self):
        data = struct.pack("<I", 0x6e01c00a) + bytes([0x80, 0x40, 0x20, 0xff])
        v = list(vif.codes(data))[0]
        self.assertEqual(vif.unpack(v), [[0x80, 0x40, 0x20, 0xff]])
        data = struct.pack("<I", 0x69018009) + struct.pack("<3h", -1, 2, -4096) + bytes(2)
        self.assertEqual(vif.unpack(list(vif.codes(data))[0])[0][:3],
                         [0xffffffff, 2, 0xfffff000])

    def test_giftag(self):
        q = struct.pack("<QQ", 0x4000000000008000 | 48, 0x512a)
        self.assertEqual(vif.giftag(q).text,
                         "GIFtag nloop=48 eop PACKED regs=FOG,ST,RGBAQ,XYZ2")
        hi = ((0x4000_4000 | (0x5c << 15)) << 32) | 0x8000 | 3
        t = vif.giftag(struct.pack("<QQ", hi, 0x512a))
        self.assertTrue(t.pre)
        self.assertEqual(vif.prim_text(t.prim), "TRISTRIP IIP TME ABE")

    def test_dmatag(self):
        t = vif.dmatag(struct.pack("<QII", 0x0000_1234_3000_0012, 0x05000000, 0x69308009))
        self.assertEqual(t.text, "REF qwc=18 addr=0x00001234")
        self.assertEqual(t.vif, struct.pack("<II", 0x05000000, 0x69308009))


@unittest.skipUnless(os.path.exists(ELF), "needs work/infection/disc/SLUS_202.67")
class TestMicrocode(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mc = vu.Microcode(ELF)

    def test_blobs(self):
        mc = self.mc
        self.assertEqual((len(mc.vu1.pairs), len(mc.vu0.pairs)), (1881, 90))
        self.assertEqual([(a, n) for a, n, _ in mc.vu1.mpgs],
                         [(i * 0x100, 256) for i in range(7)] + [(0x700, 89)])
        self.assertEqual(mc.vu1.range, (inf_va(0x001dac80), inf_va(0x001de790)))
        self.assertEqual(mc.vu0.mpgs, [(0, 90, inf_va(0x001de7a0))])

    def test_labels(self):
        v1, v0 = self.mc.vu1.labels, self.mc.vu0.labels
        self.assertEqual((len(v1), len(self.mc.vu1.globals)), (109, 24))
        self.assertEqual((len(v0), len(self.mc.vu0.globals)), (5, 2))
        for name, addr in (("mc_SetMatrix", 0), ("mc_SetObjParam", 0x22),
                           ("mc01_Start00", 0x26), ("mc_DrawTriFast", 0x63),
                           ("mc_DrawTriSFast", 0xfe), ("mc_DrawTriL", 0x154),
                           ("mc_DrawTriLC", 0x181), ("mc_ScissorTriPolyXYZ", 0x1b4),
                           ("mcstp_clipZ", 0x277), ("mcstp_clipX", 0x281),
                           ("mc03_SetParam", 0x40f), ("mc_DrawShadow3", 0x641)):
            self.assertEqual(v1[name], addr, name)
        self.assertEqual(v0["mc0_CheckBoundingBox"], 0x1f)

    def test_ee_references_agree(self):
        refs = self.mc.ee_refs()
        self.assertEqual(len(refs), 22)
        for va, name, byte in refs:
            _, a = self.mc.find(name)
            self.assertEqual(byte, a * 8, name)
        b = self.mc.vu1
        self.assertEqual(len(b.relocs), 10)
        for a, (rtype, sym) in b.relocs.items():
            self.assertEqual((rtype, b.pairs[a].lower.mnemonic), (123, "iaddiu"))
            self.assertEqual(b.pairs[a].lower.imm, b.labels[sym])

    def test_decode_sanity(self):
        r = vu.check(self.mc)
        v1, v0 = r["vu1"], r["vu0"]
        self.assertEqual((v1["invalid"], v0["invalid"]), (0, 0))
        self.assertEqual((v1["branches"], v1["to_label"], v1["outside"]), (110, 110, 0))
        self.assertEqual((v0["branches"], v0["to_label"], v0["outside"]), (4, 4, 0))
        self.assertEqual((v1["ends"], v1["ends_after_xgkick"], v0["ends"]), (25, 17, 1))
        self.assertEqual(v1["entries_not_ending"], {})
        self.assertEqual(v0["entries_not_ending"], {})
        self.assertEqual(v1["unreached_local"], ["mc_Cos", "mc_Sin"])
        self.assertEqual(v1["unreferenced_entries"], ["mc_DrawTri", "mc_DrawTriC", "mc_DrawTriS"])

    def test_model_packet(self):
        walk = vu.model_packet(ELF, 100, 0)
        seq = []
        for _, tag, segs in walk:
            st = vif.Stream(segs)
            seq += [v.text.split(" (")[0] for v in vif.codes(st.data) if v.cmd]
        self.assertEqual(seq[:16], [
            "BASE 0x0ec", "OFFSET 0x18a", "FLUSHE", "STMOD normal",
            "UNPACK V4-32 usn flg addr=0x000 num=1",
            "STMOD normal", "UNPACK V3-16 flg addr=0x009 num=48",
            "STMASK 0x3f3f3f3f pppd/pppd/pppd/pppd",
            "STMOD normal", "UNPACK V4-8 mask flg addr=0x009 num=48",
            "STMOD normal", "UNPACK V4-8 usn flg addr=0x00a num=48",
            "STMOD offset", "UNPACK V2-16 usn flg addr=0x00b num=48",
            "ITOP 0x276", "MSCAL 0x0026"])
        self.assertEqual([s for s in seq if s.startswith(("MSCAL", "ITOP"))],
                         ["ITOP 0x276", "MSCAL 0x0026", "ITOP 0x0ec", "MSCAL 0x002e",
                          "ITOP 0x276", "MSCAL 0x002e"])
        self.assertEqual(vif.DMA_ID[walk[-1][1].id], "RET")
        lit = [v.text.split(" (")[0] for _, _, segs in vu.model_packet(ELF, 10, 1)
               for v in vif.codes(vif.Stream(segs).data) if v.cmd]
        self.assertIn("UNPACK V4-8 flg addr=0x00a num=10", lit)
        self.assertIn("MSCAL 0x0141", lit)


if __name__ == "__main__":
    unittest.main()
