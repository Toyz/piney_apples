#!/usr/bin/env python3
"""Tests for tools/mips.py (and tools/image.py when the game files are there).

    python3 tools/test_mips.py [-v]

Base-ISA vectors were assembled with llvm-mc (mipsel, mips4) and its
disassembly agrees; R5900 vectors are hand-assembled from the EE Core
manual's field layouts or taken from libvu0 routines in the retail ELF,
whose listings match Sony's published source.
"""

import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mips  # noqa: E402

BASE = {
    0x27bdffe0: "addiu   $sp, $sp, -32",
    0xffbf0010: "sd      $ra, 16($sp)",
    0x0080282d: "move    $a1, $a0",
    0x00802825: "move    $a1, $a0",
    0x3c020038: "lui     $v0, 0x38",
    0x34038000: "li      $v1, 0x8000",
    0x2403ffff: "li      $v1, -1",
    0x306200ff: "andi    $v0, $v1, 0xff",
    0x8f828df0: "lw      $v0, -29200($gp)",
    0x90860043: "lbu     $a2, 67($a0)",
    0xa2000041: "sb      $zero, 65($s0)",
    0xdfbf0010: "ld      $ra, 16($sp)",
    0x68820007: "ldl     $v0, 7($a0)",
    0x6c820000: "ldr     $v0, 0($a0)",
    0xb0820007: "sdl     $v0, 7($a0)",
    0x9c820004: "lwu     $v0, 4($a0)",
    0xc4800008: "lwc1    $f0, 8($a0)",
    0xe7acfffc: "swc1    $f12, -4($sp)",
    0x000317fc: "dsll32  $v0, $v1, 31",
    0x000310ff: "dsra32  $v0, $v1, 3",
    0x00831014: "dsllv   $v0, $v1, $a0",
    0x00031100: "sll     $v0, $v1, 4",
    0x0064100a: "movz    $v0, $v1, $a0",
    0x0064100b: "movn    $v0, $v1, $a0",
    0x00031023: "negu    $v0, $v1",
    0x00601027: "not     $v0, $v1",
    0x2c620064: "sltiu   $v0, $v1, 100",
    0x38621234: "xori    $v0, $v1, 0x1234",
    0x0062001a: "div     $v1, $v0",
    0x0062001b: "divu    $v1, $v0",
    0x00001010: "mfhi    $v0",
    0x00400011: "mthi    $v0",
    0x03e00008: "jr      $ra",
    0x0320f809: "jalr    $t9",
    0x03201009: "jalr    $v0, $t9",
    0x00000000: "nop",
    0x0000000c: "syscall",
    0x0007000d: "break   0x7",
    0x004301f4: "teq     $v0, $v1, 0x7",
    0x00400036: "tne     $v0, $zero",
    0x044c0005: "teqi    $v0, 5",
    0xcc800040: "pref    0x0, 64($a0)",
    0xbc870000: "cache   0x7, 0($a0)",
    0x44820000: "mtc1    $v0, $f0",
    0x4442f800: "cfc1    $v0, $fcr31",
    0x46020800: "add.s   $f0, $f1, $f2",
    0x46000806: "mov.s   $f0, $f1",
    0x46010032: "c.eq.s  $f0, $f1",
    0x6462fff8: "daddiu  $v0, $v1, -8",
    0x0003107a: "dsrl    $v0, $v1, 1",
}

EE = {
    # 128-bit loads/stores, three-operand multiplies, pipeline 1, SA
    0x7fb30030: "sq      $s3, 48($sp)",
    0x7bb30030: "lq      $s3, 48($sp)",
    0x00631818: "mult    $v1, $v1, $v1",
    0x00640018: "mult    $v1, $a0",
    0x70851018: "mult1   $v0, $a0, $a1",
    0x7123001a: "div1    $t1, $v1",
    0x70001812: "mflo1   $v1",
    0x70850000: "madd    $a0, $a1",
    0x70851000: "madd    $v0, $a0, $a1",
    0x70801004: "plzcw   $v0, $a0",
    0x00001028: "mfsa    $v0",
    0x00400029: "mtsa    $v0",
    0x04980003: "mtsab   $a0, 3",
    0x0000040f: "sync.p",
    0x0000000f: "sync",
    # MMI0-3 and the shifts
    0x70851008: "paddw   $v0, $a0, $a1",
    0x70851488: "pextlw  $v0, $a0, $a1",
    0x70431b89: "pcpyld  $v1, $v0, $v1",
    0x708516e8: "qfsrv   $v0, $a0, $a1",
    0x708514e9: "pnor    $v0, $a0, $a1",
    0x700517a9: "pexcw   $v0, $a1",
    0x700a46e9: "pcpyh   $t0, $t2",
    0x70001030: "pmfhl.lw $v0",
    0x70001070: "pmfhl.uw $v0",
    0x700510f4: "psllh   $v0, $a1, 3",
    0x710b41e8: "pminh   $t0, $t0, $t3",
    0x70000229: "pmthi   $zero",
    # COP0
    0x40116000: "mfc0    $s1, $Status",
    0x4002c803: "mfpc    $v0, 1",
    0x42000038: "ei",
    0x42000039: "di",
    0x42000018: "eret",
    # EE FPU
    0x4617d01c: "madd.s  $f0, $f26, $f23",
    0x4618b01a: "mula.s  $f22, $f24",
    0x46000818: "adda.s  $f1, $f0",
    0x46000834: "c.lt.s  $f1, $f0",
    0x46010004: "sqrt.s  $f0, $f1",
    0x46020816: "rsqrt.s $f0, $f1, $f2",
    0x46020828: "max.s   $f0, $f1, $f2",
    0x46000824: "cvt.w.s $f0, $f1",
    0x46800820: "cvt.s.w $f0, $f1",
    # COP2 moves and VU0 macro mode (sceVu0Normalize, hand-assembled rest)
    0xd8a40000: "lqc2    $vf4, 0($a1)",
    0xf8860000: "sqc2    $vf6, 0($a0)",
    0x48a20801: "qmtc2.i $v0, $vf1",
    0x48220800: "qmfc2.ni $v0, $vf1",
    0x4844e000: "cfc2.ni $a0, $FBRST",
    0x4bc4216a: "vmul.xyz $vf5, $vf4, $vf4",
    0x4b052941: "vaddy.x $vf5, $vf5, $vf5y",
    0x4a0503bd: "vsqrt   $Q, $vf5x",
    0x4a0003bf: "vwaitq",
    0x4b000160: "vaddq.x $vf5, $vf0, $Q",
    0x4a0002ff: "vnop",
    0x4a6503bc: "vdiv    $Q, $vf0w, $vf5x",
    0x4bc0219c: "vmulq.xyz $vf6, $vf4, $Q",
    0x4bc31068: "vadd.xyz $vf1, $vf2, $vf3",
    0x4be31040: "vaddx.xyzw $vf1, $vf2, $vf3x",
    0x4be821bc: "vmulax.xyzw $ACC, $vf4, $vf8x",
    0x4bc209ff: "vclipw.xyz $vf1, $vf2w",
    0x4be1137c: "vlqi.xyzw $vf1, ($vi2++)",
    0x4b0113fe: "vilwr.x $vi1, ($vi2)",
    0x4b0113fd: "vmfir.x $vf1, $vi2",
    0x4a0117f2: "viaddi  $vi1, $vi2, -1",
    0x4a0000b8: "vcallms 0x0010",
    0x4a26333d: "vmr32.w $vf6, $vf6",
}

NOT_EE = [
    0x00ff00ff,  # dsra32 with rs set: the 128-bit constant in _copyRefImage
    0x4c000000,  # COP1X
    0x46200000,  # add.d: no doubles on the EE
    0xc1111111,  # ll
    0xd4000000,  # ldc1
    0x0000001c,  # dmult
    0x40000003,  # mfc0 with a MIPS32 select
    0x70000002,  # hole in the MMI table
]


class Decode(unittest.TestCase):
    def check(self, table):
        for w, want in table.items():
            with self.subTest(word=f"{w:08x}"):
                self.assertEqual(mips.decode(w, 0x100000).text, want)

    def test_base(self):
        self.check(BASE)

    def test_ee(self):
        self.check(EE)

    def test_word_fallback(self):
        for w in NOT_EE:
            ins = mips.decode(w, 0)
            self.assertFalse(ins.valid, f"{w:08x} -> {ins.text}")
            self.assertEqual(ins.text, f".word   0x{w:08x}")

    def test_never_raises(self):
        rng = random.Random(5900)
        for _ in range(100000):
            mips.decode(rng.getrandbits(32), rng.getrandbits(30) << 2)

    def test_branches(self):
        ins = mips.decode(0x10c300ef, 0x102698)  # beq $a2, $v1
        self.assertEqual(ins.target, 0x102a58)
        self.assertTrue(ins.delay and ins.cond and not ins.call)
        ins = mips.decode(0x10000135, 0x102700)  # b
        self.assertEqual((ins.mnemonic, ins.target), ("b", 0x102bd8))
        self.assertTrue(ins.unconditional)
        ins = mips.decode(0x0c04b436, 0x102740)  # jal
        self.assertEqual(ins.target, 0x12d0d8)
        self.assertTrue(ins.call and ins.delay and not ins.cond)
        self.assertIn("$ra", ins.writes)
        ins = mips.decode(0x5045fffb, 0x11e6c8)  # beql
        self.assertTrue(ins.likely and ins.cond)
        self.assertEqual(mips.decode(0x4901fff9, 0x13ea1c).target, 0x13ea04)  # bc2t
        self.assertEqual(mips.decode(0x03e00008, 0).target, None)

    def test_fields(self):
        ins = mips.decode(0x8f828df0, 0)  # lw $v0, -29200($gp)
        self.assertEqual((ins.base, ins.offset, ins.size, ins.store), ("$gp", -29200, 4, False))
        self.assertEqual((ins.reads, ins.writes), (("$gp",), ("$v0",)))
        ins = mips.decode(0x7fb30030, 0)  # sq
        self.assertEqual((ins.size, ins.store), (16, True))
        self.assertEqual(mips.decode(0x3c020038, 0).imm, 0x38)
        self.assertEqual(mips.decode(0x2403ffff, 0).imm, -1)
        ins = mips.decode(0x70851018, 0)  # mult1
        self.assertEqual(set(ins.writes), {"$v0", "$hi1", "$lo1"})
        ins = mips.decode(0x4be821bc, 0)  # vmulax
        self.assertEqual((ins.reads, ins.writes), (("$vf4", "$vf8"), ("$ACC",)))

    def test_track_lui(self):
        words = [0x3c040010,   # lui   $a0, 0x10
                 0x24843210,   # addiu $a0, $a0, 0x3210
                 0x3c030037,   # lui   $v1, 0x37
                 0x0c000000,   # jal   (clobbers $v1 after the slot)
                 0x8c6288e0,   # lw    $v0, -0x7720($v1)   in the slot: still known
                 0x8c620000,   # lw    $v0, 0($v1)         after the call: unknown
                 0x8f828df0]   # lw    $v0, -29200($gp)
        insns = [mips.decode(w, 0x1000 + 4 * i) for i, w in enumerate(words)]
        got = mips.track_lui(insns, gp=0x37faf0)
        self.assertEqual(got, {0x1004: 0x103210, 0x1010: 0x3688e0, 0x1018: 0x3788e0})


ELF = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "work", "infection",
                   "disc", "SLUS_202.67")


@unittest.skipUnless(os.path.exists(ELF), "game files not extracted")
class Image(unittest.TestCase):
    def test_overlay_mapping(self):
        from image import Program
        p = Program(ELF, "gcmn")
        s = p.symbol_named("ccRegisterEnemyList__Fiii")
        self.assertEqual(s.value, 0x0042ed70)
        # sll $v1, $a0, 3 opens it; the header is loaded at 0x00400800 too
        self.assertEqual(p.u32(s.value), 0x000418c0)
        self.assertEqual(p.read(0x00400800, 4), b"MWo3")
        self.assertIsNone(Program(ELF).symbol_named("ccRegisterEnemyList__Fiii"))
        with self.assertRaises(KeyError):
            Program(ELF).read(0x0042ed70, 4)

    def test_relocs(self):
        from image import Program
        p = Program(ELF, "toppage")
        hi, = p.relocs_at(0x00400888)
        lo, = p.relocs_at(0x0040088c)
        self.assertEqual((hi.type, lo.type), (5, 6))
        self.assertEqual(lo.addr, p.symbol_named("toppageFileList").value)
        call, = Program(ELF).relocs_at(0x00102740)
        self.assertEqual(call.symbol.name, "scePadInfoMode")


if __name__ == "__main__":
    unittest.main()
