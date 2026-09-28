#!/usr/bin/env python3
"""A small EE interpreter, for running static initialisers over the image.

Some tables are only complete after start-up: an overlay's `__sinit_<file>`
functions copy structs into place (the mail replies in desktop.prg are one
case), and `mwLoadOverlay` runs them through `__initialize_cpp_rts` right
after loading. This runs the same constructor table over a copy of memory, so
tools can read tables as the game sees them.

    tools/eemu.py sinit ELF --overlay NAME       run the overlay's ctor table,
                                                 report what changed
    tools/eemu.py call  ELF FUNC [--overlay NAME] [ARG...]

It is an interpreter with 128-bit registers and flat memory from 0 to
32 MB, covering what compiled game code uses: ALU and shifts, loads and
stores of every width including lq/sq and lwc1/swc1, branches, j/jal/jr/jalr,
mult/div on both EE pipelines, and the COP1 single-precision FPU. Anything
else stops the run with the pc and the instruction, rather than guessing. The
C string and memory functions are run in Python (HLE), because the game's
versions are MMI-vectorised.

The FPU follows the EE, not IEEE 754 (EE Core User's Manual, FPU chapter):
an exponent field of 0 is zero whatever the mantissa, on input and output (no
denormals); an exponent field of 255 is an ordinary number (no infinities or
NaNs); a result too large becomes +/-0x7fffffff, the EE's Fmax, and one too
small becomes a zero of the result's sign; division by zero gives +/-Fmax;
sqrt.s takes the magnitude of a negative operand; the only rounding mode is
towards zero. Each result here is the exact result truncated to 24 bits.
Real hardware is known to differ from that in the last bit for some add and
multiply operands (its adder keeps few guard bits, its multiplier is not
exact); that is not modelled. The f_* functions implement the arithmetic on
raw 32-bit patterns so reimplementations can share it; madd.s/msub.s round
the product before the add.
"""

import argparse
import math
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mips  # noqa: E402
from image import Program  # noqa: E402

RAM = 32 << 20
M64 = (1 << 64) - 1
M128 = (1 << 128) - 1
STACK_TOP = RAM - 0x1000
RETURN_SENTINEL = 0xFFFFFFF0
ANNUL = object()


class Stop(Exception):
    pass


def sx(v, bits):
    v &= (1 << bits) - 1
    return v - (1 << bits) if v >> (bits - 1) else v


# EE single-precision arithmetic on raw u32 patterns ---------------------------

FMAX = 0x7FFFFFFF
C_BIT = 1 << 23            # FCR31 condition bit, what bc1t/bc1f test


def _unpack(v):
    """(sign, exponent field, mantissa with the hidden bit); exponent 0 is a
    zero whatever the mantissa says."""
    e = (v >> 23) & 0xFF
    return v >> 31 & 1, e, ((v & 0x7FFFFF) | 0x800000) if e else 0


def _round(sign, n, e2):
    """sign, exact value n * 2**e2 (n >= 0) -> EE float, truncating."""
    if n == 0:
        return sign << 31
    length = n.bit_length()
    m = n >> (length - 24) if length >= 24 else n << (24 - length)
    exp = e2 + length - 24 + 150
    if exp > 255:
        return sign << 31 | FMAX
    if exp < 1:
        return sign << 31
    return sign << 31 | exp << 23 | (m & 0x7FFFFF)


def f_value(v):
    """Exact (numerator, exponent) of an EE float, as a signed n * 2**e."""
    s, e, m = _unpack(v)
    return (-m if s else m), e - 150


def f_add(a, b):
    (na, ea), (nb, eb) = f_value(a), f_value(b)
    if na == 0 and nb == 0:
        return a & b & 0x80000000        # -0 only when both are -0
    if na == 0:
        return _round(int(nb < 0), abs(nb), eb)
    if nb == 0:
        return _round(int(na < 0), abs(na), ea)
    e = min(ea, eb)
    n = (na << (ea - e)) + (nb << (eb - e))
    return _round(int(n < 0), abs(n), e)


def f_sub(a, b):
    return f_add(a, b ^ 0x80000000)


def f_mul(a, b):
    sa, ea, ma = _unpack(a)
    sb, eb, mb = _unpack(b)
    return _round(sa ^ sb, ma * mb, ea + eb - 300)


def f_div(a, b):
    sa, ea, ma = _unpack(a)
    sb, eb, mb = _unpack(b)
    if mb == 0:
        return (sa ^ sb) << 31 | FMAX
    return _round(sa ^ sb, (ma << 60) // mb, ea - eb - 60)


def _sqrt_parts(v):
    """isqrt of |v| as (root, exponent): |v| ~ root**2 * 4**exponent."""
    _, e, m = _unpack(v)
    t = 60 + ((e - 150 - 60) & 1)
    return math.isqrt(m << t), (e - 150 - t) // 2


def f_sqrt(v):
    r, e = _sqrt_parts(v)
    return _round(0, r, e)


def f_rsqrt(a, b):
    """a / sqrt(b), rounded once: floor(A / sqrt(B)) = isqrt(A * A // B)."""
    sa, ea, ma = _unpack(a)
    _, eb, mb = _unpack(b)
    if mb == 0:
        return sa << 31 | FMAX
    t = 60 + ((eb - 150 - 60) & 1)
    num = ma << 60
    q = math.isqrt(num * num // (mb << t))
    return _round(sa, q, ea - 150 - 60 - (eb - 150 - t) // 2)


def f_from_int(i):
    """cvt.s.w"""
    i = sx(i, 32)
    return _round(int(i < 0), abs(i), 0)


def f_to_int(v):
    """cvt.w.s: truncate, saturating at the int range."""
    n, e = f_value(v)
    mag = abs(n) << e if e >= 0 else abs(n) >> -e
    if n < 0:
        return 0x80000000 if mag > 0x80000000 else (-mag) & 0xFFFFFFFF
    return 0x7FFFFFFF if mag > 0x7FFFFFFF else mag


def f_cmp(a, b):
    """-1, 0, 1 comparing exact values (both zeros equal)."""
    (na, ea), (nb, eb) = f_value(a), f_value(b)
    e = min(ea, eb)
    x, y = na << (ea - e), nb << (eb - e)
    return (x > y) - (x < y)


def f_to_py(v):
    """An EE float as a Python float (exponent 255 overflows to inf)."""
    n, e = f_value(v)
    try:
        return float(n) * 2.0 ** e
    except OverflowError:
        return float("inf") if n > 0 else float("-inf")


def f_from_py(x):
    """A Python float as IEEE single bits (for constants)."""
    return struct.unpack("<I", struct.pack("<f", x))[0]


class Machine:
    def __init__(self, program):
        self.p = program
        self.mem = bytearray(RAM)
        elf = program.elf
        for seg in elf.segments:
            if seg.type == 1 and seg.filesz:
                self.mem[seg.vaddr:seg.vaddr + seg.filesz] = elf.data[seg.offset:seg.offset + seg.filesz]
        ov = program.overlay
        if ov is not None:
            self.mem[ov.base:ov.base + len(ov.data)] = ov.data
        self.pristine = bytes(self.mem)
        # The C library is hand-vectorised with MMI (strcmp uses pcpyld, pceqb
        # and friends); running it adds nothing, so these are done in Python.
        self.hooks = {}
        for name, impl in HLE.items():
            sym = program.symbol_named(name)
            if sym is not None:
                self.hooks[sym.value] = impl
        self.r = [0] * 32
        self.f = [0] * 32
        self.hi = self.lo = 0
        self.hi1 = self.lo1 = 0     # the EE's second multiply/divide pipeline
        self.fcr31 = 0
        self.acc = 0                # FPU accumulator (adda.s, madd.s...)
        self.steps = 0

    # memory -----------------------------------------------------------------
    def load(self, addr, n, signed=False):
        addr &= 0xFFFFFFFF
        if addr + n > RAM:
            raise Stop(f"read of {n} bytes at 0x{addr:08x} is outside RAM")
        return int.from_bytes(self.mem[addr:addr + n], "little", signed=signed)

    def store(self, addr, n, value):
        addr &= 0xFFFFFFFF
        if addr + n > RAM:
            raise Stop(f"write of {n} bytes at 0x{addr:08x} is outside RAM")
        self.mem[addr:addr + n] = (value & ((1 << (8 * n)) - 1)).to_bytes(n, "little")

    # registers --------------------------------------------------------------
    def get(self, i):
        return self.r[i]

    def set(self, i, v, bits=64):
        if i:
            self.r[i] = v & ((1 << bits) - 1) if bits == 128 else (
                (self.r[i] & ~M64) | (v & M64))

    def set32(self, i, v):
        self.set(i, sx(v, 32) & M64)

    # execution --------------------------------------------------------------
    def call(self, addr, args=(), limit=5_000_000):
        self.r = [0] * 32
        self.r[28] = self.p.gp
        self.r[29] = STACK_TOP
        self.r[31] = RETURN_SENTINEL
        # the EE ABI passes eight integer arguments in registers: a0-a3, t0-t3
        for i, a in enumerate(args[:8]):
            self.set32(4 + i, a)
        pc = addr
        npc = pc + 4
        self.steps = 0
        while pc != RETURN_SENTINEL:
            self.steps += 1
            if self.steps > limit:
                raise Stop(f"gave up after {limit} steps at 0x{pc:08x}")
            hook = self.hooks.get(pc)
            if hook is not None:
                self.set32(2, hook(self, *(self.r[4 + i] & 0xFFFFFFFF for i in range(4))))
                pc = self.r[31] & 0xFFFFFFFF
                npc = pc + 4
                continue
            w = self.load(pc, 4)
            target = self.exec(pc, w)
            if target is ANNUL:
                # A branch-likely that is not taken skips its delay slot.
                pc, npc = npc + 4, npc + 8
            else:
                pc, npc = npc, (target if target is not None else npc + 4)
        return self.r[2] & 0xFFFFFFFF

    def exec(self, pc, w):
        """Execute one word; return the branch target to take after the delay
        slot, or None."""
        op = w >> 26
        rs = (w >> 21) & 31
        rt = (w >> 16) & 31
        rd = (w >> 11) & 31
        sa = (w >> 6) & 31
        imm = w & 0xFFFF
        simm = sx(imm, 16)
        R = self.r
        a32 = lambda i: sx(R[i], 32)            # noqa: E731
        u32 = lambda i: R[i] & 0xFFFFFFFF       # noqa: E731
        a64 = lambda i: sx(R[i], 64)            # noqa: E731
        ea = (a32(rs) + simm) & 0xFFFFFFFF

        if op == 0:  # SPECIAL
            fn = w & 63
            if fn == 0x00:
                self.set32(rd, u32(rt) << sa)
            elif fn == 0x02:
                self.set32(rd, u32(rt) >> sa)
            elif fn == 0x03:
                self.set32(rd, a32(rt) >> sa)
            elif fn == 0x04:
                self.set32(rd, u32(rt) << (R[rs] & 31))
            elif fn == 0x06:
                self.set32(rd, u32(rt) >> (R[rs] & 31))
            elif fn == 0x07:
                self.set32(rd, a32(rt) >> (R[rs] & 31))
            elif fn == 0x08:
                return u32(rs)
            elif fn == 0x09:
                self.set(rd, pc + 8)
                return u32(rs)
            elif fn == 0x0a:
                if R[rt] & M64 == 0:
                    self.set(rd, R[rs])
            elif fn == 0x0b:
                if R[rt] & M64:
                    self.set(rd, R[rs])
            elif fn == 0x0f:
                pass  # sync
            elif fn == 0x10:
                self.set(rd, self.hi)
            elif fn == 0x12:
                self.set(rd, self.lo)
            elif fn == 0x11:
                self.hi = R[rs]
            elif fn == 0x13:
                self.lo = R[rs]
            elif fn in (0x18, 0x19):
                if fn == 0x18:
                    v = a32(rs) * a32(rt)
                else:
                    v = u32(rs) * u32(rt)
                self.lo = sx(v, 32) & M64
                self.hi = sx(v >> 32, 32) & M64
                self.set(rd, self.lo)
            elif fn in (0x1a, 0x1b):
                d = a32(rt) if fn == 0x1a else u32(rt)
                n = a32(rs) if fn == 0x1a else u32(rs)
                if d:
                    q = abs(n) // abs(d) * (1 if (n < 0) == (d < 0) else -1)
                    self.lo = sx(q, 32) & M64
                    self.hi = sx(n - q * d, 32) & M64
            elif fn == 0x21 or fn == 0x20:
                self.set32(rd, a32(rs) + a32(rt))
            elif fn == 0x23 or fn == 0x22:
                self.set32(rd, a32(rs) - a32(rt))
            elif fn == 0x24:
                self.set(rd, R[rs] & R[rt])
            elif fn == 0x25:
                self.set(rd, R[rs] | R[rt])
            elif fn == 0x26:
                self.set(rd, R[rs] ^ R[rt])
            elif fn == 0x27:
                self.set(rd, ~(R[rs] | R[rt]))
            elif fn == 0x2a:
                self.set(rd, int(a64(rs) < a64(rt)))
            elif fn == 0x2b:
                self.set(rd, int((R[rs] & M64) < (R[rt] & M64)))
            elif fn == 0x14:
                self.set(rd, (R[rt] & M64) << (R[rs] & 63))
            elif fn == 0x16:
                self.set(rd, (R[rt] & M64) >> (R[rs] & 63))
            elif fn == 0x17:
                self.set(rd, a64(rt) >> (R[rs] & 63))
            elif fn == 0x2d or fn == 0x2c:
                self.set(rd, R[rs] + R[rt])
            elif fn == 0x2f or fn == 0x2e:
                self.set(rd, R[rs] - R[rt])
            elif fn == 0x38:
                self.set(rd, (R[rt] & M64) << sa)
            elif fn == 0x3a:
                self.set(rd, (R[rt] & M64) >> sa)
            elif fn == 0x3b:
                self.set(rd, a64(rt) >> sa)
            elif fn == 0x3c:
                self.set(rd, (R[rt] & M64) << (sa + 32))
            elif fn == 0x3e:
                self.set(rd, (R[rt] & M64) >> (sa + 32))
            elif fn == 0x3f:
                self.set(rd, a64(rt) >> (sa + 32))
            else:
                self.bad(pc, w)
            return None
        if op == 1:  # REGIMM
            cond = {0: a64(rs) < 0, 1: a64(rs) >= 0, 2: a64(rs) < 0, 3: a64(rs) >= 0,
                    0x10: a64(rs) < 0, 0x11: a64(rs) >= 0}.get(rt)
            if cond is None:
                self.bad(pc, w)
            if rt in (0x10, 0x11):
                self.set(31, pc + 8)
            return self.branch(pc, simm, cond, likely=rt in (2, 3))
        if op in (2, 3):
            if op == 3:
                self.set(31, pc + 8)
            return ((pc + 4) & 0xF0000000) | ((w & 0x3FFFFFF) << 2)
        if op in (4, 5, 6, 7, 0x14, 0x15, 0x16, 0x17):
            base = op & 3
            cond = [R[rs] & M64 == R[rt] & M64, R[rs] & M64 != R[rt] & M64,
                    a64(rs) <= 0, a64(rs) > 0][base]
            return self.branch(pc, simm, cond, likely=op >= 0x14)
        if op == 0x08 or op == 0x09:
            self.set32(rt, a32(rs) + simm)
        elif op == 0x0a:
            self.set(rt, int(a64(rs) < simm))
        elif op == 0x0b:
            self.set(rt, int((R[rs] & M64) < (simm & M64)))
        elif op == 0x0c:
            self.set(rt, R[rs] & imm)
        elif op == 0x0d:
            self.set(rt, (R[rs] & M64) | imm)
        elif op == 0x0e:
            self.set(rt, (R[rs] & M64) ^ imm)
        elif op == 0x0f:
            self.set32(rt, imm << 16)
        elif op == 0x18 or op == 0x19:
            self.set(rt, a64(rs) + simm)
        elif op == 0x1c:  # MMI: only the pipeline-1 multiply/divide and moves
            fn = w & 63
            if fn in (0x18, 0x19):
                v = a32(rs) * a32(rt) if fn == 0x18 else u32(rs) * u32(rt)
                self.lo1 = sx(v, 32) & M64
                self.hi1 = sx(v >> 32, 32) & M64
                self.set(rd, self.lo1)
            elif fn in (0x1a, 0x1b):
                d = a32(rt) if fn == 0x1a else u32(rt)
                n = a32(rs) if fn == 0x1a else u32(rs)
                if d:
                    q = abs(n) // abs(d) * (1 if (n < 0) == (d < 0) else -1)
                    self.lo1 = sx(q, 32) & M64
                    self.hi1 = sx(n - q * d, 32) & M64
            elif fn == 0x10:
                self.set(rd, self.hi1)
            elif fn == 0x12:
                self.set(rd, self.lo1)
            elif fn == 0x11:
                self.hi1 = R[rs]
            elif fn == 0x13:
                self.lo1 = R[rs]
            else:
                self.bad(pc, w)
        elif op == 0x11:  # COP1
            fmt = rs
            if fmt == 0x00:
                self.set32(rt, self.f[rd])
            elif fmt == 0x02:
                self.set32(rt, self.fcr31 if rd == 31 else 0x2e30 if rd == 0 else 0)
            elif fmt == 0x04:
                self.f[rd] = u32(rt)
            elif fmt == 0x06:
                if rd == 31:
                    self.fcr31 = u32(rt)
            elif fmt == 0x08:
                if rt > 3:
                    self.bad(pc, w)
                cond = bool(self.fcr31 & C_BIT) == bool(rt & 1)
                return self.branch(pc, simm, cond, likely=rt >= 2)
            elif fmt == 0x10:
                self.cop1_s(pc, w, rt, rd, sa)
            elif fmt == 0x14 and (w & 63) == 0x20:
                self.f[sa] = f_from_int(self.f[rd])
            else:
                self.bad(pc, w)
        elif op == 0x20:
            self.set(rt, self.load(ea, 1, True) & M64)
        elif op == 0x24:
            self.set(rt, self.load(ea, 1))
        elif op == 0x21:
            self.set(rt, self.load(ea, 2, True) & M64)
        elif op == 0x25:
            self.set(rt, self.load(ea, 2))
        elif op == 0x23:
            self.set32(rt, self.load(ea, 4))
        elif op == 0x27:
            self.set(rt, self.load(ea, 4))
        elif op == 0x37:
            self.set(rt, self.load(ea, 8))
        elif op == 0x1e:
            self.set(rt, self.load(ea & ~15, 16), bits=128)
        elif op == 0x28:
            self.store(ea, 1, R[rt])
        elif op == 0x29:
            self.store(ea, 2, R[rt])
        elif op == 0x2b:
            self.store(ea, 4, R[rt])
        elif op == 0x3f:
            self.store(ea, 8, R[rt])
        elif op == 0x1f:
            self.store(ea & ~15, 16, R[rt])
        elif op == 0x31:
            self.f[rt] = self.load(ea, 4)
        elif op == 0x39:
            self.store(ea, 4, self.f[rt])
        elif op == 0x2f:
            pass  # cache
        elif op == 0x33:
            pass  # pref
        else:
            self.bad(pc, w)
        return None

    def cop1_s(self, pc, w, ft, fs, fd):
        """COP1 S-format: ft, fs, fd are the rt, rd, sa fields."""
        fn = w & 63
        F = self.f
        a, b = F[fs], F[ft]
        if fn == 0x00:
            F[fd] = f_add(a, b)
        elif fn == 0x01:
            F[fd] = f_sub(a, b)
        elif fn == 0x02:
            F[fd] = f_mul(a, b)
        elif fn == 0x03:
            F[fd] = f_div(a, b)
        elif fn == 0x04:
            F[fd] = f_sqrt(b)            # the EE takes sqrt.s's operand from ft
        elif fn == 0x05:
            F[fd] = a & 0x7FFFFFFF
        elif fn == 0x06:
            F[fd] = a
        elif fn == 0x07:
            F[fd] = a ^ 0x80000000
        elif fn == 0x16:
            F[fd] = f_rsqrt(a, b)
        elif fn == 0x18:
            self.acc = f_add(a, b)
        elif fn == 0x19:
            self.acc = f_sub(a, b)
        elif fn == 0x1a:
            self.acc = f_mul(a, b)
        elif fn == 0x1c:
            F[fd] = f_add(self.acc, f_mul(a, b))
        elif fn == 0x1d:
            F[fd] = f_sub(self.acc, f_mul(a, b))
        elif fn == 0x1e:
            self.acc = f_add(self.acc, f_mul(a, b))
        elif fn == 0x1f:
            self.acc = f_sub(self.acc, f_mul(a, b))
        elif fn == 0x24:
            F[fd] = f_to_int(a)
        elif fn == 0x28:
            F[fd] = a if f_cmp(a, b) >= 0 else b
        elif fn == 0x29:
            F[fd] = a if f_cmp(a, b) <= 0 else b
        elif fn in (0x30, 0x32, 0x34, 0x36):
            c = f_cmp(a, b)
            cond = {0x30: False, 0x32: c == 0, 0x34: c < 0, 0x36: c <= 0}[fn]
            self.fcr31 = (self.fcr31 | C_BIT) if cond else (self.fcr31 & ~C_BIT)
        else:
            self.bad(pc, w)

    def branch(self, pc, simm, cond, likely=False):
        if cond:
            return (pc + 4 + (simm << 2)) & 0xFFFFFFFF
        return ANNUL if likely else None

    def bad(self, pc, w):
        ins = mips.decode(w, pc)
        where = self.p.name_at(pc) if hasattr(self.p, "name_at") else ""
        raise Stop(f"0x{pc:08x} {where}: {ins.text if hasattr(ins, 'text') else ins} "
                   f"is not interpreted")

    def changed(self):
        """[(start, end)] byte ranges that differ from the loaded image,
        leaving out the stack."""
        out = []
        a = self.mem
        b = self.pristine
        i = 0
        n = STACK_TOP - 0x100000
        while i < n:
            if a[i] != b[i]:
                j = i
                while j < n and a[j] != b[j]:
                    j += 1
                out.append((i, j))
                i = j
            else:
                i += 1
        return out


def _cstr(m, a):
    end = m.mem.index(0, a)
    return bytes(m.mem[a:end])


def _strcmp(m, a, b, *_):
    x, y = _cstr(m, a), _cstr(m, b)
    for i in range(min(len(x), len(y)) + 1):
        cx = x[i] if i < len(x) else 0
        cy = y[i] if i < len(y) else 0
        if cx != cy:
            return cx - cy
    return 0


def _strncmp(m, a, b, n, *_):
    x, y = _cstr(m, a)[:n], _cstr(m, b)[:n]
    return 0 if x == y else (1 if x > y else -1)


def _strlen(m, a, *_):
    return len(_cstr(m, a))


def _strcpy(m, d, s, *_):
    v = _cstr(m, s) + b"\0"
    m.mem[d:d + len(v)] = v
    return d


def _strcat(m, d, s, *_):
    return _strcpy(m, d + len(_cstr(m, d)), s) and d


def _memcpy(m, d, s, n, *_):
    m.mem[d:d + n] = bytes(m.mem[s:s + n])
    return d


def _memset(m, d, c, n, *_):
    m.mem[d:d + n] = bytes([c & 0xFF]) * n
    return d


def _memcmp(m, a, b, n, *_):
    x, y = bytes(m.mem[a:a + n]), bytes(m.mem[b:b + n])
    return 0 if x == y else (1 if x > y else -1)


HLE = {"strcmp": _strcmp, "strncmp": _strncmp, "strlen": _strlen, "strcpy": _strcpy,
       "strcat": _strcat, "memcpy": _memcpy, "memmove": _memcpy, "memset": _memset,
       "memcmp": _memcmp}


def run_ctors(program):
    """A Machine with the overlay's constructor table already run, as
    mwLoadOverlay -> __initialize_cpp_rts does."""
    m = Machine(program)
    ov = program.overlay
    if ov is None:
        raise ValueError("run_ctors needs an overlay")
    for va in range(ov.ctor_start, ov.ctor_end, 4):
        m.call(m.load(va, 4))
    return m


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("sinit")
    p.add_argument("elf")
    p.add_argument("--overlay", required=True)
    p = sub.add_parser("call")
    p.add_argument("elf")
    p.add_argument("func")
    p.add_argument("args", nargs="*", type=lambda s: int(s, 0))
    p.add_argument("--overlay")
    args = parser.parse_args()

    prog = Program(args.elf, args.overlay)
    if args.cmd == "sinit":
        m = Machine(prog)
        ov = prog.overlay
        for va in range(ov.ctor_start, ov.ctor_end, 4):
            fn = m.load(va, 4)
            m.call(fn)
            print(f"ran {prog.name_at(fn)} in {m.steps} steps")
        spans = m.changed()
        total = sum(e - s for s, e in spans)
        print(f"{len(spans)} changed ranges, {total} bytes")
        byobj = {}
        for s, e in spans:
            hit = prog.symbol_at(s)
            key = hit[0].name if hit else f"0x{s:08x}"
            byobj[key] = byobj.get(key, 0) + (e - s)
        for k, v in sorted(byobj.items(), key=lambda kv: -kv[1])[:30]:
            print(f"  {v:8d}  {k}")
    else:
        sym = prog.symbol_named(args.func)
        m = Machine(prog)
        v = m.call(sym.value if sym else int(args.func, 0), args.args)
        print(f"returned 0x{v:08x} after {m.steps} steps")
    return 0


if __name__ == "__main__":
    sys.exit(main())
