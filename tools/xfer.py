#!/usr/bin/env python3
"""Comparing code between volumes: a function's words with every
address-bearing field masked (`normalise`), the addresses its code forms
(`addresses`), and its opcode sequence (`shape`), over one section of an
executable (`Side`). The tools that read a later volume's code by what it
does use these (voldiff.py, font.py and others).

The symbol transfer that writes the stripped volumes' `<elf>.syms`
sidecars, which grew up here, is now `piney-gen syms` (crates/piney-gen);
its first passes compare code with the same masks.
"""

import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from image import Program  # noqa: E402

GP = 28
# I-type ops whose 16-bit immediate can be the low half of an address.
LO_OPS = {0x09, 0x19, 0x0d,                                     # addiu daddiu ori
          0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x37,  # loads
          0x1a, 0x1b, 0x1e, 0x31, 0x36,
          0x28, 0x29, 0x2a, 0x2b, 0x2e, 0x3f, 0x2c, 0x2d, 0x1f,  # stores
          0x39, 0x3e}
LOADS = {0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x37, 0x1a, 0x1b, 0x1e}


def sext16(v):
    return v - 0x10000 if v & 0x8000 else v


def normalise(words):
    """Mask every field that holds an absolute address or a $gp offset."""
    out = []
    hi = set()                  # registers holding a lui'd upper half
    for w in words:
        op = w >> 26
        rs = (w >> 21) & 31
        rt = (w >> 16) & 31
        if op in (2, 3):
            out.append(w & 0xFC000000)
            continue
        if op == 0x0F:
            out.append(w & 0xFFFF0000)
            hi.add(rt)
            continue
        if op in LO_OPS and (rs == GP or rs in hi):
            out.append(w & 0xFFFF0000)
            if op in LOADS:
                hi.discard(rt)
            continue
        out.append(w)
        if op == 0:
            funct = w & 63
            rd = (w >> 11) & 31
            if funct in (0x21, 0x2D, 0x25) and (rs in hi or rt in hi):
                hi.add(rd)
            elif funct not in (0x08, 0x09):
                hi.discard(rd)
        elif 0x08 <= op <= 0x0F or op in LOADS or op == 0x19:
            hi.discard(rt)
    return out


def addresses(words, va, gp):
    """{instruction index: absolute address it forms}, for lo halves paired
    with a lui and for $gp-relative accesses."""
    out = {}
    hi = {}
    for i, w in enumerate(words):
        op = w >> 26
        rs = (w >> 21) & 31
        rt = (w >> 16) & 31
        if op == 0x0F:
            hi[rt] = (w & 0xFFFF) << 16
            continue
        if op in LO_OPS:
            if rs == GP and gp:
                out[i] = (gp + sext16(w & 0xFFFF)) & 0xFFFFFFFF
            elif rs in hi:
                imm = w & 0xFFFF
                out[i] = (hi[rs] | imm) if op == 0x0D else (hi[rs] + sext16(imm)) & 0xFFFFFFFF
            if op in LOADS or (op in (0x09, 0x0D, 0x19) and rt != rs):
                hi.pop(rt, None)
            continue
        if op == 0 and ((w >> 11) & 31) in hi and (w & 63) not in (0x08, 0x09):
            hi.pop((w >> 11) & 31)
    return out


class Side:
    """One section of one volume: main, or main + an overlay."""

    def __init__(self, elf, overlay):
        self.p = Program(elf, overlay)
        self.overlay = overlay
        if overlay:
            ov = self.p.overlay
            self.lo, self.hi = ov.text, ov.text + ov.text_size + ov.data_size
            self.code_hi = ov.text + ov.text_size
        else:
            m = self.p.main_seg
            self.lo, self.hi = m.vaddr, m.vaddr + m.filesz
            self.code_hi = self.hi
        raw = self.p.read(self.lo, self.hi - self.lo)
        self.words = struct.unpack(f"<{len(raw) // 4}I", raw[:len(raw) // 4 * 4])

    def word_at(self, va):
        return self.words[(va - self.lo) >> 2]

    def run(self, va, n):
        i = (va - self.lo) >> 2
        return list(self.words[i:i + n]) if 0 <= i and i + n <= len(self.words) else None

    def owns(self, va):
        return self.lo <= va < self.hi


def shape(words):
    """The opcode sequence, nops dropped: what survives register allocation
    and scheduling changes between compiler settings."""
    out = []
    for w in words:
        if w == 0:
            continue
        op = w >> 26
        if op == 0:
            out.append(w & 63)
        elif op == 0x11:
            out.append(0x1100 | ((w >> 21) & 31) << 6 | (w & 63))
        elif op == 0x1C:
            out.append(0x1C00 | ((w >> 6) & 31) << 6 | (w & 63))
        elif op == 1:
            out.append(0x100 | (w >> 16) & 31)
        else:
            out.append(0x200 | op)
    return out
