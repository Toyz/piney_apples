#!/usr/bin/env python3
"""MIPS R5900 (PlayStation 2 Emotion Engine) instruction decoder.

The EE core is MIPS III plus the MIPS IV movz/movn/pref, with 128-bit GPRs,
the MMI SIMD group (opcode 0x1c), a second HI/LO pair for pipeline 1, the SA
shift register, a single-precision FPU with an accumulator, and VU0 attached
as COP2 in macro mode. Output follows GNU objdump conventions: pseudo-ops
(nop, move, li, b, beqz, negu, not ...), hex immediates for logical ops,
signed decimal for arithmetic and memory offsets.

    tools/mips.py word WORD... [--pc PC]    decode 32-bit instruction words
    tools/mips.py hex  HEXBYTES [--pc PC]   decode little-endian bytes

Also a library: `decode(word, pc)` returns an `Insn` with the mnemonic, the
operand strings and structured fields for analysis (branch target, call /
delay-slot flags, registers read and written, immediate, memory base and
offset). Words that are not EE instructions come back as `.word 0x........`
with `valid` False; decode never raises.
"""

import argparse
import sys

GPR = ("$zero", "$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3",
       "$t0", "$t1", "$t2", "$t3", "$t4", "$t5", "$t6", "$t7",
       "$s0", "$s1", "$s2", "$s3", "$s4", "$s5", "$s6", "$s7",
       "$t8", "$t9", "$k0", "$k1", "$gp", "$sp", "$fp", "$ra")
FPR = tuple(f"$f{i}" for i in range(32))
VF = tuple(f"$vf{i}" for i in range(32))
VI = tuple(f"$vi{i}" for i in range(32))
C0 = ("$Index", "$Random", "$EntryLo0", "$EntryLo1", "$Context", "$PageMask",
      "$Wired", "$c0r7", "$BadVAddr", "$Count", "$EntryHi", "$Compare",
      "$Status", "$Cause", "$EPC", "$PRId", "$Config", "$c0r17", "$c0r18",
      "$c0r19", "$c0r20", "$c0r21", "$c0r22", "$BadPAddr", "$Debug", "$Perf",
      "$c0r26", "$c0r27", "$TagLo", "$TagHi", "$ErrorEPC", "$c0r31")
# COP2 control registers as cfc2/ctc2 see them: vi0-vi15, then VU0 state.
C2 = VI[:16] + ("$Status", "$MACflag", "$ClipFlag", "$c2r19", "$R", "$I",
                "$Q", "$c2r23", "$c2r24", "$c2r25", "$TPC", "$CMSAR0",
                "$FBRST", "$VPU-STAT", "$c2r30", "$CMSAR1")

# Loads and stores: mnemonic -> (bytes, is_store).
MEM = {"lb": (1, False), "lbu": (1, False), "lh": (2, False), "lhu": (2, False),
       "lw": (4, False), "lwu": (4, False), "lwl": (4, False), "lwr": (4, False),
       "ld": (8, False), "ldl": (8, False), "ldr": (8, False), "lq": (16, False),
       "sb": (1, True), "sh": (2, True), "sw": (4, True), "swl": (4, True),
       "swr": (4, True), "sd": (8, True), "sdl": (8, True), "sdr": (8, True),
       "sq": (16, True), "lwc1": (4, False), "swc1": (4, True),
       "lqc2": (16, False), "sqc2": (16, True), "cache": (0, False),
       "pref": (0, False)}

HI = ("$hi", "$lo")
HI1 = ("$hi1", "$lo1")
# Registers an instruction touches without naming them: mnemonic -> (reads, writes).
IMPLICIT = {
    "mult": ((), HI), "multu": ((), HI), "div": ((), HI), "divu": ((), HI),
    "madd": (HI, HI), "maddu": (HI, HI),
    "mult1": ((), HI1), "multu1": ((), HI1), "div1": ((), HI1), "divu1": ((), HI1),
    "madd1": (HI1, HI1), "maddu1": (HI1, HI1),
    "mfhi": (("$hi",), ()), "mflo": (("$lo",), ()),
    "mthi": ((), ("$hi",)), "mtlo": ((), ("$lo",)),
    "mfhi1": (("$hi1",), ()), "mflo1": (("$lo1",), ()),
    "mthi1": ((), ("$hi1",)), "mtlo1": ((), ("$lo1",)),
    "pmfhi": (("$hi",), ()), "pmflo": (("$lo",), ()),
    "pmthi": ((), ("$hi",)), "pmtlo": ((), ("$lo",)),
    "pmfhl.lw": (HI, ()), "pmfhl.uw": (HI, ()), "pmfhl.slw": (HI, ()),
    "pmfhl.lh": (HI, ()), "pmfhl.sh": (HI, ()), "pmthl.lw": (HI, HI),
    "pmultw": ((), HI), "pmultuw": ((), HI), "pmulth": ((), HI),
    "pdivw": ((), HI), "pdivuw": ((), HI), "pdivbw": ((), HI),
    "pmaddw": (HI, HI), "pmadduw": (HI, HI), "pmaddh": (HI, HI),
    "pmsubw": (HI, HI), "pmsubh": (HI, HI), "phmadh": ((), HI), "phmsbh": ((), HI),
    "mfsa": (("$sa",), ()), "mtsa": ((), ("$sa",)), "mtsab": ((), ("$sa",)),
    "mtsah": ((), ("$sa",)), "qfsrv": (("$sa",), ()),
    "jal": ((), ("$ra",)), "bal": ((), ("$ra",)),
    "bltzal": ((), ("$ra",)), "bgezal": ((), ("$ra",)),
    "bltzall": ((), ("$ra",)), "bgezall": ((), ("$ra",)),
    "c.f.s": ((), ("$fcc",)), "c.eq.s": ((), ("$fcc",)),
    "c.lt.s": ((), ("$fcc",)), "c.le.s": ((), ("$fcc",)),
    "bc1f": (("$fcc",), ()), "bc1t": (("$fcc",), ()),
    "bc1fl": (("$fcc",), ()), "bc1tl": (("$fcc",), ()),
    "adda.s": ((), ("$acc",)), "suba.s": ((), ("$acc",)), "mula.s": ((), ("$acc",)),
    "madda.s": (("$acc",), ("$acc",)), "msuba.s": (("$acc",), ("$acc",)),
    "madd.s": (("$acc",), ()), "msub.s": (("$acc",), ()),
    "vwaitq": (("$Q",), ()),
}


class Insn:
    """One decoded instruction.

    `target` is the destination of a PC-relative branch or j/jal (None for
    jr/jalr). `delay` marks anything with a delay slot, `cond` a conditional
    branch, `likely` a branch-likely (slot annulled when not taken), `call`
    jal/jalr/bal/bgezal. `imm` is the immediate as printed (lui: the raw
    16 bits, not shifted). Memory ops fill `base`, `offset`, `size`, `store`.
    """
    __slots__ = ("pc", "word", "mnemonic", "operands", "target", "delay",
                 "cond", "likely", "call", "reads", "writes", "imm", "base",
                 "offset", "size", "store", "valid")

    def __init__(self, pc, word, mnemonic):
        self.pc = pc
        self.word = word
        self.mnemonic = mnemonic
        self.operands = []
        self.target = None
        self.delay = self.cond = self.likely = self.call = False
        self.reads = self.writes = ()
        self.imm = self.base = self.offset = None
        self.size = 0
        self.store = False
        self.valid = True

    @property
    def rs(self):
        return (self.word >> 21) & 31

    @property
    def rt(self):
        return (self.word >> 16) & 31

    @property
    def rd(self):
        return (self.word >> 11) & 31

    @property
    def unconditional(self):
        """j, jr, b: control never falls through past the delay slot."""
        return self.delay and not self.cond and not self.call

    @property
    def text(self):
        if not self.operands:
            return self.mnemonic
        return f"{self.mnemonic:<7} {', '.join(self.operands)}"

    def __str__(self):
        return self.text

    def __repr__(self):
        return f"<Insn 0x{self.pc:08x} {self.text}>"


def _sext16(v):
    return v - 0x10000 if v & 0x8000 else v


# ---------------------------------------------------------------------------
# Operand tokens. Each takes (word, pc, insn) and returns (text, register)
# where register is the name to record as read/written, or None.

def _dest(w):
    d = (w >> 21) & 15
    return "".join(c for c, b in zip("xyzw", (8, 4, 2, 1)) if d & b)


def _t_mem(w, pc, ins):
    off = _sext16(w & 0xffff)
    base = GPR[(w >> 21) & 31]
    ins.base, ins.offset = base, off
    ins.size, ins.store = MEM.get(ins.mnemonic, (0, False))
    return f"{off}({base})", base


def _t_br(w, pc, ins):
    t = (pc + 4 + (_sext16(w & 0xffff) << 2)) & 0xffffffff
    ins.target = t
    return f"0x{t:08x}", None


def _t_jt(w, pc, ins):
    t = ((pc + 4) & 0xf0000000) | ((w & 0x3ffffff) << 2)
    ins.target = t
    return f"0x{t:08x}", None


def _t_simm(w, pc, ins):
    ins.imm = _sext16(w & 0xffff)
    return str(ins.imm), None


def _t_uimm(w, pc, ins):
    ins.imm = w & 0xffff
    return f"0x{ins.imm:x}", None


def _t_sa(w, pc, ins):
    ins.imm = (w >> 6) & 31
    return str(ins.imm), None


def _t_code20(w, pc, ins):
    c = (w >> 6) & 0xfffff
    return (f"0x{c:x}" if c else ""), None


def _t_code10(w, pc, ins):
    hi, lo = (w >> 16) & 0x3ff, (w >> 6) & 0x3ff
    if lo:
        return f"0x{hi:x}, 0x{lo:x}", None
    return (f"0x{hi:x}" if hi else ""), None


def _t_tcode(w, pc, ins):
    c = (w >> 6) & 0x3ff
    return (f"0x{c:x}" if c else ""), None


def _t_hex5(w, pc, ins):
    # cache op / pref hint live in the rt field.
    return f"0x{(w >> 16) & 31:x}", None


def _t_pcreg(w, pc, ins):
    return str((w >> 1) & 31), None


def _vbc(w):
    return "xyzw"[w & 3]


def _t_vimm5(w, pc, ins):
    v = (w >> 6) & 31
    ins.imm = v - 32 if v & 16 else v
    return str(ins.imm), None


def _t_vimm15(w, pc, ins):
    # CMSAR0 counts 64-bit micro instructions; print the byte address in
    # VU0 micro memory, which is what the microcode listings use.
    ins.imm = ((w >> 6) & 0x7fff) * 8
    return f"0x{ins.imm:04x}", None


def _fixed(name):
    return lambda w, pc, ins: (name, name)


TOK = {
    "rs": lambda w, pc, ins: (GPR[(w >> 21) & 31],) * 2,
    "rt": lambda w, pc, ins: (GPR[(w >> 16) & 31],) * 2,
    "rd": lambda w, pc, ins: (GPR[(w >> 11) & 31],) * 2,
    "fs": lambda w, pc, ins: (FPR[(w >> 11) & 31],) * 2,
    "ft": lambda w, pc, ins: (FPR[(w >> 16) & 31],) * 2,
    "fd": lambda w, pc, ins: (FPR[(w >> 6) & 31],) * 2,
    "fcr": lambda w, pc, ins: (f"$fcr{(w >> 11) & 31}",) * 2,
    "c0": lambda w, pc, ins: (C0[(w >> 11) & 31],) * 2,
    "c2": lambda w, pc, ins: (C2[(w >> 11) & 31],) * 2,
    "vfrd": lambda w, pc, ins: (VF[(w >> 11) & 31],) * 2,
    "vft": lambda w, pc, ins: (VF[(w >> 16) & 31],) * 2,
    "vfs": lambda w, pc, ins: (VF[(w >> 11) & 31],) * 2,
    "vfd": lambda w, pc, ins: (VF[(w >> 6) & 31],) * 2,
    "vftbc": lambda w, pc, ins: (VF[(w >> 16) & 31] + _vbc(w), VF[(w >> 16) & 31]),
    "vfsf": lambda w, pc, ins: (VF[(w >> 11) & 31] + "xyzw"[(w >> 21) & 3],
                                VF[(w >> 11) & 31]),
    "vftf": lambda w, pc, ins: (VF[(w >> 16) & 31] + "xyzw"[(w >> 23) & 3],
                                VF[(w >> 16) & 31]),
    "vit": lambda w, pc, ins: (VI[(w >> 16) & 31],) * 2,
    "vis": lambda w, pc, ins: (VI[(w >> 11) & 31],) * 2,
    "vid": lambda w, pc, ins: (VI[(w >> 6) & 31],) * 2,
    "(vis)": lambda w, pc, ins: (f"({VI[(w >> 11) & 31]})", VI[(w >> 11) & 31]),
    "(vis++)": lambda w, pc, ins: (f"({VI[(w >> 11) & 31]}++)", VI[(w >> 11) & 31]),
    "(vit++)": lambda w, pc, ins: (f"({VI[(w >> 16) & 31]}++)", VI[(w >> 16) & 31]),
    "(--vis)": lambda w, pc, ins: (f"(--{VI[(w >> 11) & 31]})", VI[(w >> 11) & 31]),
    "(--vit)": lambda w, pc, ins: (f"(--{VI[(w >> 16) & 31]})", VI[(w >> 16) & 31]),
    "ACC": _fixed("$ACC"), "Q": _fixed("$Q"), "I": _fixed("$I"), "R": _fixed("$R"),
    "mem": _t_mem, "br": _t_br, "jt": _t_jt, "simm": _t_simm, "uimm": _t_uimm,
    "sa": _t_sa, "code20": _t_code20, "code10": _t_code10, "tcode": _t_tcode,
    "hex5": _t_hex5, "pcreg": _t_pcreg, "vimm5": _t_vimm5, "vimm15": _t_vimm15,
}

# Which R-type fields each token consumes; the rest must be zero.
RS, RT, RD, SA = 0x03e00000, 0x001f0000, 0x0000f800, 0x000007c0
FIELD = {"rs": RS, "rt": RT, "rd": RD, "sa": SA}
USES = {"rs": RS, "rt": RT, "rd": RD, "sa": SA, "fs": RD, "ft": RT, "fd": SA,
        "fcr": RD, "c0": RD, "c2": RD, "vfrd": RD, "code20": RS | RT | RD | SA,
        "code10": RS | RT | RD | SA, "tcode": RD | SA}
R4 = ("rs", "rt", "rd", "sa")


def E(mn, fmt="", flags="", fields=(), zero=0):
    """A table leaf: (mnemonic, tokens, must-be-zero mask, flags).

    fmt is comma-separated tokens; a trailing ! marks a register written, ~
    one both read and written. `fields` lists the R-type fields that must be
    zero unless a token uses them. flags: b conditional branch, l likely,
    j unconditional jump, c call, D append the VU dest mask to the mnemonic.
    """
    toks = []
    used = 0
    for t in fmt.split(","):
        if not t:
            continue
        mode = 0
        if t[-1] in "!~":
            mode = 1 if t[-1] == "!" else 2
            t = t[:-1]
        toks.append((TOK[t], mode))
        used |= USES.get(t, 0)
    for f in fields:
        if not FIELD[f] & used:
            zero |= FIELD[f]
    return (mn, tuple(toks), zero, flags)


def by(shift, mask, table):
    get = table.get
    return lambda w: get((w >> shift) & mask)


# ---------------------------------------------------------------------------
# Tables.

def R(mn, fmt="", flags=""):
    return E(mn, fmt, flags, R4)


SPECIAL = {
    0x00: R("sll", "rd!,rt,sa"), 0x02: R("srl", "rd!,rt,sa"), 0x03: R("sra", "rd!,rt,sa"),
    0x04: R("sllv", "rd!,rt,rs"), 0x06: R("srlv", "rd!,rt,rs"), 0x07: R("srav", "rd!,rt,rs"),
    0x08: R("jr", "rs", "j"), 0x09: R("jalr", "rd!,rs", "c"),
    0x0a: R("movz", "rd~,rs,rt"), 0x0b: R("movn", "rd~,rs,rt"),
    0x0c: R("syscall", "code20"), 0x0d: R("break", "code10"),
    0x0f: by(6, 31, {0x00: E("sync", "", "", R4), 0x10: E("sync.p", "", "", ("rs", "rt", "rd"))}),
    0x10: R("mfhi", "rd!"), 0x11: R("mthi", "rs"), 0x12: R("mflo", "rd!"), 0x13: R("mtlo", "rs"),
    0x14: R("dsllv", "rd!,rt,rs"), 0x16: R("dsrlv", "rd!,rt,rs"), 0x17: R("dsrav", "rd!,rt,rs"),
    # The EE's mult/multu also write LO to rd.
    0x18: R("mult", "rd!,rs,rt"), 0x19: R("multu", "rd!,rs,rt"),
    0x1a: R("div", "rs,rt"), 0x1b: R("divu", "rs,rt"),
    0x20: R("add", "rd!,rs,rt"), 0x21: R("addu", "rd!,rs,rt"),
    0x22: R("sub", "rd!,rs,rt"), 0x23: R("subu", "rd!,rs,rt"),
    0x24: R("and", "rd!,rs,rt"), 0x25: R("or", "rd!,rs,rt"),
    0x26: R("xor", "rd!,rs,rt"), 0x27: R("nor", "rd!,rs,rt"),
    0x28: R("mfsa", "rd!"), 0x29: R("mtsa", "rs"),
    0x2a: R("slt", "rd!,rs,rt"), 0x2b: R("sltu", "rd!,rs,rt"),
    0x2c: R("dadd", "rd!,rs,rt"), 0x2d: R("daddu", "rd!,rs,rt"),
    0x2e: R("dsub", "rd!,rs,rt"), 0x2f: R("dsubu", "rd!,rs,rt"),
    0x30: R("tge", "rs,rt,tcode"), 0x31: R("tgeu", "rs,rt,tcode"),
    0x32: R("tlt", "rs,rt,tcode"), 0x33: R("tltu", "rs,rt,tcode"),
    0x34: R("teq", "rs,rt,tcode"), 0x36: R("tne", "rs,rt,tcode"),
    0x38: R("dsll", "rd!,rt,sa"), 0x3a: R("dsrl", "rd!,rt,sa"), 0x3b: R("dsra", "rd!,rt,sa"),
    0x3c: R("dsll32", "rd!,rt,sa"), 0x3e: R("dsrl32", "rd!,rt,sa"), 0x3f: R("dsra32", "rd!,rt,sa"),
}

REGIMM = {
    0x00: E("bltz", "rs,br", "b"), 0x01: E("bgez", "rs,br", "b"),
    0x02: E("bltzl", "rs,br", "bl"), 0x03: E("bgezl", "rs,br", "bl"),
    0x08: E("tgei", "rs,simm"), 0x09: E("tgeiu", "rs,simm"),
    0x0a: E("tlti", "rs,simm"), 0x0b: E("tltiu", "rs,simm"),
    0x0c: E("teqi", "rs,simm"), 0x0e: E("tnei", "rs,simm"),
    0x10: E("bltzal", "rs,br", "bc"), 0x11: E("bgezal", "rs,br", "bc"),
    0x12: E("bltzall", "rs,br", "blc"), 0x13: E("bgezall", "rs,br", "blc"),
    0x18: E("mtsab", "rs,simm"), 0x19: E("mtsah", "rs,simm"),
}

MMI_3OP = "rd!,rs,rt"
MMI0 = {i: E(mn, MMI_3OP, "", ("rs", "rt", "rd")) for i, mn in enumerate(
    ("paddw", "psubw", "pcgtw", "pmaxw", "paddh", "psubh", "pcgth", "pmaxh",
     "paddb", "psubb", "pcgtb", None, None, None, None, None,
     "paddsw", "psubsw", "pextlw", "ppacw", "paddsh", "psubsh", "pextlh", "ppach",
     "paddsb", "psubsb", "pextlb", "ppacb", None, None, "pext5", "ppac5")) if mn}
MMI1 = {i: E(mn, MMI_3OP, "", ("rs", "rt", "rd")) for i, mn in enumerate(
    (None, "pabsw", "pceqw", "pminw", "padsbh", "pabsh", "pceqh", "pminh",
     None, None, "pceqb", None, None, None, None, None,
     "padduw", "psubuw", "pextuw", None, "padduh", "psubuh", "pextuh", None,
     "paddub", "psubub", "pextub", "qfsrv")) if mn}
MMI2 = {i: E(mn, MMI_3OP, "", ("rs", "rt", "rd")) for i, mn in enumerate(
    ("pmaddw", None, "psllvw", "psrlvw", "pmsubw", None, None, None,
     "pmfhi", "pmflo", "pinth", None, "pmultw", "pdivw", "pcpyld", None,
     "pmaddh", "phmadh", "pand", "pxor", "pmsubh", "phmsbh", None, None,
     None, None, "pexeh", "prevh", "pmulth", "pdivbw", "pexew", "prot3w")) if mn}
MMI3 = {i: E(mn, MMI_3OP, "", ("rs", "rt", "rd")) for i, mn in enumerate(
    ("pmadduw", None, None, "psravw", None, None, None, None,
     "pmthi", "pmtlo", "pinteh", None, "pmultuw", "pdivuw", "pcpyud", None,
     None, None, "por", "pnor", None, None, None, None,
     None, None, "pexch", "pcpyh", None, None, "pexcw")) if mn}
# The ones that are not rd, rs, rt.
_MMI_FMT = {
    "pabsw": "rd!,rt", "pabsh": "rd!,rt", "pext5": "rd!,rt", "ppac5": "rd!,rt",
    "pexeh": "rd!,rt", "prevh": "rd!,rt", "pexew": "rd!,rt", "prot3w": "rd!,rt",
    "pexch": "rd!,rt", "pcpyh": "rd!,rt", "pexcw": "rd!,rt",
    "pmfhi": "rd!", "pmflo": "rd!", "pmthi": "rs", "pmtlo": "rs",
    "psllvw": "rd!,rt,rs", "psrlvw": "rd!,rt,rs", "psravw": "rd!,rt,rs",
    "pdivw": "rs,rt", "pdivuw": "rs,rt", "pdivbw": "rs,rt",
}
for _t in (MMI0, MMI1, MMI2, MMI3):
    for _k, _e in _t.items():
        if _e[0] in _MMI_FMT:
            _t[_k] = E(_e[0], _MMI_FMT[_e[0]], "", ("rs", "rt", "rd"))

MMI = {
    0x00: R("madd", "rd!,rs,rt"), 0x01: R("maddu", "rd!,rs,rt"),
    0x04: R("plzcw", "rd!,rs"),
    0x08: by(6, 31, MMI0), 0x09: by(6, 31, MMI2), 0x28: by(6, 31, MMI1), 0x29: by(6, 31, MMI3),
    0x10: R("mfhi1", "rd!"), 0x11: R("mthi1", "rs"), 0x12: R("mflo1", "rd!"), 0x13: R("mtlo1", "rs"),
    0x18: R("mult1", "rd!,rs,rt"), 0x19: R("multu1", "rd!,rs,rt"),
    0x1a: R("div1", "rs,rt"), 0x1b: R("divu1", "rs,rt"),
    0x20: R("madd1", "rd!,rs,rt"), 0x21: R("maddu1", "rd!,rs,rt"),
    0x30: by(6, 31, {i: E("pmfhl." + f, "rd!", "", ("rs", "rt"))
                     for i, f in enumerate(("lw", "uw", "slw", "lh", "sh"))}),
    0x31: by(6, 31, {0: E("pmthl.lw", "rs", "", ("rt", "rd"))}),
    0x34: R("psllh", "rd!,rt,sa"), 0x36: R("psrlh", "rd!,rt,sa"), 0x37: R("psrah", "rd!,rt,sa"),
    0x3c: R("psllw", "rd!,rt,sa"), 0x3e: R("psrlw", "rd!,rt,sa"), 0x3f: R("psraw", "rd!,rt,sa"),
}

# COP0. The EE hides its breakpoint (rd 24) and performance counter (rd 25)
# registers behind the low bits of mfc0/mtc0.
_C0_DBG = {0: "bpc", 2: "iab", 3: "iabm", 4: "dab", 5: "dabm", 6: "dvb", 7: "dvbm"}
MF0 = {("mf", k): E("mf" + v, "rt!") for k, v in _C0_DBG.items()}
MF0.update({("mt", k): E("mt" + v, "rt") for k, v in _C0_DBG.items()})
MF0.update({("mf", "ps"): E("mfps", "rt!,pcreg"), ("mf", "pc"): E("mfpc", "rt!,pcreg"),
            ("mt", "ps"): E("mtps", "rt,pcreg"), ("mt", "pc"): E("mtpc", "rt,pcreg"),
            ("mf", None): E("mfc0", "rt!,c0"), ("mt", None): E("mtc0", "rt,c0")})


def _cop0_move(kind):
    def f(w):
        rd, low = (w >> 11) & 31, w & 0x7ff
        if rd == 24:
            return MF0.get((kind, low))
        if rd == 25:
            if low & 0x7c0:
                return None
            return MF0[(kind, "pc" if low & 1 else "ps")]
        return MF0[(kind, None)] if low == 0 else None
    return f


C0OPS = 0x01ffffc0
COP0 = {
    0x00: _cop0_move("mf"), 0x04: _cop0_move("mt"),
    0x08: by(16, 31, {0: E("bc0f", "br", "b"), 1: E("bc0t", "br", "b"),
                      2: E("bc0fl", "br", "bl"), 3: E("bc0tl", "br", "bl")}),
    0x10: by(0, 63, {0x01: E("tlbr", zero=C0OPS), 0x02: E("tlbwi", zero=C0OPS),
                     0x06: E("tlbwr", zero=C0OPS), 0x08: E("tlbp", zero=C0OPS),
                     0x18: E("eret", zero=C0OPS), 0x38: E("ei", zero=C0OPS),
                     0x39: E("di", zero=C0OPS)}),
}

F3 = ("rt", "rd", "sa")  # ft, fs, fd
COP1_S = {
    0x00: E("add.s", "fd!,fs,ft", "", F3), 0x01: E("sub.s", "fd!,fs,ft", "", F3),
    0x02: E("mul.s", "fd!,fs,ft", "", F3), 0x03: E("div.s", "fd!,fs,ft", "", F3),
    0x04: E("sqrt.s", "fd!,ft", "", F3),  # EE: the operand is ft, not fs
    0x05: E("abs.s", "fd!,fs", "", F3), 0x06: E("mov.s", "fd!,fs", "", F3),
    0x07: E("neg.s", "fd!,fs", "", F3), 0x16: E("rsqrt.s", "fd!,fs,ft", "", F3),
    0x18: E("adda.s", "fs,ft", "", F3), 0x19: E("suba.s", "fs,ft", "", F3),
    0x1a: E("mula.s", "fs,ft", "", F3), 0x1c: E("madd.s", "fd!,fs,ft", "", F3),
    0x1d: E("msub.s", "fd!,fs,ft", "", F3), 0x1e: E("madda.s", "fs,ft", "", F3),
    0x1f: E("msuba.s", "fs,ft", "", F3), 0x24: E("cvt.w.s", "fd!,fs", "", F3),
    0x28: E("max.s", "fd!,fs,ft", "", F3), 0x29: E("min.s", "fd!,fs,ft", "", F3),
    0x30: E("c.f.s", "fs,ft", "", F3), 0x32: E("c.eq.s", "fs,ft", "", F3),
    0x34: E("c.lt.s", "fs,ft", "", F3), 0x36: E("c.le.s", "fs,ft", "", F3),
}
COP1 = {
    0x00: E("mfc1", "rt!,fs", zero=0x7ff), 0x02: E("cfc1", "rt!,fcr", zero=0x7ff),
    0x04: E("mtc1", "rt,fs!", zero=0x7ff), 0x06: E("ctc1", "rt,fcr!", zero=0x7ff),
    0x08: by(16, 31, {0: E("bc1f", "br", "b"), 1: E("bc1t", "br", "b"),
                      2: E("bc1fl", "br", "bl"), 3: E("bc1tl", "br", "bl")}),
    0x10: by(0, 63, COP1_S),
    0x14: by(0, 63, {0x20: E("cvt.s.w", "fd!,fs", "", F3)}),
}

# COP2 macro mode. SPECIAL1 is keyed by funct; funct 0x3c-0x3f escape to
# SPECIAL2, keyed by bits 10..6 and 1..0 together.
VU1 = {}
for _i, _c in enumerate("xyzw"):
    for _base, _mn in ((0x00, "vadd"), (0x04, "vsub"), (0x08, "vmadd"), (0x0c, "vmsub"),
                       (0x10, "vmax"), (0x14, "vmini"), (0x18, "vmul")):
        VU1[_base + _i] = E(_mn + _c, "vfd!,vfs,vftbc", "D")
for _k, _mn, _src in ((0x1c, "vmulq", "Q"), (0x1d, "vmaxi", "I"), (0x1e, "vmuli", "I"),
                      (0x1f, "vminii", "I"), (0x20, "vaddq", "Q"), (0x21, "vmaddq", "Q"),
                      (0x22, "vaddi", "I"), (0x23, "vmaddi", "I"), (0x24, "vsubq", "Q"),
                      (0x25, "vmsubq", "Q"), (0x26, "vsubi", "I"), (0x27, "vmsubi", "I")):
    VU1[_k] = E(_mn, "vfd!,vfs," + _src, "D")
for _k, _mn in ((0x28, "vadd"), (0x29, "vmadd"), (0x2a, "vmul"), (0x2b, "vmax"),
                (0x2c, "vsub"), (0x2d, "vmsub"), (0x2e, "vopmsub"), (0x2f, "vmini")):
    VU1[_k] = E(_mn, "vfd!,vfs,vft", "D")
VU1.update({
    0x30: E("viadd", "vid!,vis,vit"), 0x31: E("visub", "vid!,vis,vit"),
    0x32: E("viaddi", "vit!,vis,vimm5"), 0x34: E("viand", "vid!,vis,vit"),
    0x35: E("vior", "vid!,vis,vit"), 0x38: E("vcallms", "vimm15"),
    0x39: E("vcallmsr", "vis"),
})

VU2 = {}
for _i, _c in enumerate("xyzw"):
    for _row, _mn in ((0, "vadda"), (1, "vsuba"), (2, "vmadda"), (3, "vmsuba"), (6, "vmula")):
        VU2[_row * 4 + _i] = E(_mn + _c, "ACC!,vfs,vftbc", "D")
for _i, _n in enumerate(("0", "4", "12", "15")):
    VU2[16 + _i] = E("vitof" + _n, "vft!,vfs", "D")
    VU2[20 + _i] = E("vftoi" + _n, "vft!,vfs", "D")
_VNONE = 0x01fff800  # dest, ft and fs all zero
VU2.update({
    28: E("vmulaq", "ACC!,vfs,Q", "D"), 29: E("vabs", "vft!,vfs", "D"),
    30: E("vmulai", "ACC!,vfs,I", "D"), 31: E("vclipw", "vfs,vftbc", "D"),
    32: E("vaddaq", "ACC!,vfs,Q", "D"), 33: E("vmaddaq", "ACC~,vfs,Q", "D"),
    34: E("vaddai", "ACC!,vfs,I", "D"), 35: E("vmaddai", "ACC~,vfs,I", "D"),
    36: E("vsubaq", "ACC!,vfs,Q", "D"), 37: E("vmsubaq", "ACC~,vfs,Q", "D"),
    38: E("vsubai", "ACC!,vfs,I", "D"), 39: E("vmsubai", "ACC~,vfs,I", "D"),
    40: E("vadda", "ACC!,vfs,vft", "D"), 41: E("vmadda", "ACC~,vfs,vft", "D"),
    42: E("vmula", "ACC!,vfs,vft", "D"),
    44: E("vsuba", "ACC!,vfs,vft", "D"), 45: E("vmsuba", "ACC~,vfs,vft", "D"),
    46: E("vopmula", "ACC!,vfs,vft", "D"), 47: E("vnop", zero=_VNONE),
    48: E("vmove", "vft!,vfs", "D"), 49: E("vmr32", "vft!,vfs", "D"),
    52: E("vlqi", "vft!,(vis++)~", "D"), 53: E("vsqi", "vfs,(vit++)~", "D"),
    54: E("vlqd", "vft!,(--vis)~", "D"), 55: E("vsqd", "vfs,(--vit)~", "D"),
    56: E("vdiv", "Q!,vfsf,vftf"), 57: E("vsqrt", "Q!,vftf"),
    58: E("vrsqrt", "Q!,vfsf,vftf"), 59: E("vwaitq", zero=_VNONE),
    60: E("vmtir", "vit!,vfsf"), 61: E("vmfir", "vft!,vis", "D"),
    62: E("vilwr", "vit!,(vis)", "D"), 63: E("viswr", "vit,(vis)", "D"),
    64: E("vrnext", "vft!,R~", "D"), 65: E("vrget", "vft!,R", "D"),
    66: E("vrinit", "R!,vfsf"), 67: E("vrxor", "R~,vfsf"),
})
# vmadd*/vmsub* (non-A forms) read ACC without naming it.
for _e in list(VU1.values()):
    if _e[0].startswith(("vmadd", "vmsub")):
        IMPLICIT[_e[0]] = (("$ACC",), ())
IMPLICIT["vopmsub"] = (("$ACC",), ())


def _vu(w):
    f = w & 63
    if f < 0x3c:
        return VU1.get(f)
    return VU2.get((((w >> 6) & 31) << 2) | (w & 3))


def _il(mn, fmt):
    # Bit 0 of the COP2 moves is the VU0 interlock: .i waits for a running
    # micro program to finish, .ni does not.
    return by(0, 1, {0: E(mn + ".ni", fmt, zero=0x7fe), 1: E(mn + ".i", fmt, zero=0x7fe)})


COP2 = {
    0x01: _il("qmfc2", "rt!,vfrd"), 0x02: _il("cfc2", "rt!,c2"),
    0x05: _il("qmtc2", "rt,vfrd!"), 0x06: _il("ctc2", "rt,c2!"),
    0x08: by(16, 31, {0: E("bc2f", "br", "b"), 1: E("bc2t", "br", "b"),
                      2: E("bc2fl", "br", "bl"), 3: E("bc2tl", "br", "bl")}),
}
for _k in range(0x10, 0x20):
    COP2[_k] = _vu

OPCODE = {
    0x00: by(0, 63, SPECIAL), 0x01: by(16, 31, REGIMM),
    0x02: E("j", "jt", "j"), 0x03: E("jal", "jt", "c"),
    0x04: E("beq", "rs,rt,br", "b"), 0x05: E("bne", "rs,rt,br", "b"),
    0x06: E("blez", "rs,br", "b", ("rt",)), 0x07: E("bgtz", "rs,br", "b", ("rt",)),
    0x08: E("addi", "rt!,rs,simm"), 0x09: E("addiu", "rt!,rs,simm"),
    0x0a: E("slti", "rt!,rs,simm"), 0x0b: E("sltiu", "rt!,rs,simm"),
    0x0c: E("andi", "rt!,rs,uimm"), 0x0d: E("ori", "rt!,rs,uimm"),
    0x0e: E("xori", "rt!,rs,uimm"), 0x0f: E("lui", "rt!,uimm", "", ("rs",)),
    0x10: by(21, 31, COP0), 0x11: by(21, 31, COP1), 0x12: by(21, 31, COP2),
    0x14: E("beql", "rs,rt,br", "bl"), 0x15: E("bnel", "rs,rt,br", "bl"),
    0x16: E("blezl", "rs,br", "bl", ("rt",)), 0x17: E("bgtzl", "rs,br", "bl", ("rt",)),
    0x18: E("daddi", "rt!,rs,simm"), 0x19: E("daddiu", "rt!,rs,simm"),
    0x1a: E("ldl", "rt~,mem"), 0x1b: E("ldr", "rt~,mem"),
    0x1c: by(0, 63, MMI),
    0x1e: E("lq", "rt!,mem"), 0x1f: E("sq", "rt,mem"),
    0x20: E("lb", "rt!,mem"), 0x21: E("lh", "rt!,mem"), 0x22: E("lwl", "rt~,mem"),
    0x23: E("lw", "rt!,mem"), 0x24: E("lbu", "rt!,mem"), 0x25: E("lhu", "rt!,mem"),
    0x26: E("lwr", "rt~,mem"), 0x27: E("lwu", "rt!,mem"),
    0x28: E("sb", "rt,mem"), 0x29: E("sh", "rt,mem"), 0x2a: E("swl", "rt,mem"),
    0x2b: E("sw", "rt,mem"), 0x2c: E("sdl", "rt,mem"), 0x2d: E("sdr", "rt,mem"),
    0x2e: E("swr", "rt,mem"), 0x2f: E("cache", "hex5,mem"),
    0x31: E("lwc1", "ft!,mem"), 0x33: E("pref", "hex5,mem"),
    0x36: E("lqc2", "vft!,mem"), 0x37: E("ld", "rt!,mem"),
    0x39: E("swc1", "ft,mem"), 0x3e: E("sqc2", "vft,mem"), 0x3f: E("sd", "rt,mem"),
}
_ROOT = [OPCODE.get(i) for i in range(64)]


# ---------------------------------------------------------------------------

def _word(w, pc):
    ins = Insn(pc, w, ".word")
    ins.operands = [f"0x{w:08x}"]
    ins.valid = False
    return ins


def decode(word, pc=0):
    """Decode one 32-bit little-endian-loaded instruction word at `pc`."""
    w = word & 0xffffffff
    node = _ROOT[w >> 26]
    while node is not None and not isinstance(node, tuple):
        node = node(w)
    if node is None or w & node[2]:
        return _word(w, pc)
    base, toks, _, flags = node
    ins = Insn(pc, w, base)
    ops = ins.operands
    reads = []
    writes = []
    for fn, mode in toks:
        text, reg = fn(w, pc, ins)
        if text:
            ops.append(text)
        if reg:
            if mode != 1:
                reads.append(reg)
            if mode:
                writes.append(reg)
    if flags:
        if "D" in flags:
            d = _dest(w)
            if d:
                ins.mnemonic = f"{base}.{d}"
        if "b" in flags:
            ins.delay = ins.cond = True
            ins.likely = "l" in flags
        if "j" in flags:
            ins.delay = True
        if "c" in flags:
            ins.delay = ins.call = True
            ins.cond = "b" in flags
    imp = IMPLICIT.get(base)
    if imp:
        reads.extend(imp[0])
        writes.extend(imp[1])
    if base in _PSEUDO:
        _PSEUDO[base](ins, w, reads, writes)
    ins.reads = tuple(r for r in reads if r != "$zero")
    ins.writes = tuple(r for r in writes if r != "$zero")
    return ins


# ---------------------------------------------------------------------------
# GNU objdump pseudo-ops.

def _p_move(ins, w, reads, writes):
    if (w >> 16) & 31 == 0:
        ins.mnemonic = "move"
        del ins.operands[2]


def _p_li(ins, w, reads, writes):
    if (w >> 21) & 31 == 0:
        ins.mnemonic = "li"
        del ins.operands[1]


def _p_beq(ins, w, reads, writes):
    rs, rt = (w >> 21) & 31, (w >> 16) & 31
    if rt:
        return
    if rs == 0 and ins.mnemonic == "beq":
        ins.mnemonic = "b"
        ins.cond = False
        ins.operands = ins.operands[2:]
    else:
        ins.mnemonic = {"beq": "beqz", "bne": "bnez", "beql": "beqzl", "bnel": "bnezl"}[ins.mnemonic]
        del ins.operands[1]


def _p_bgez(ins, w, reads, writes):
    if (w >> 21) & 31 == 0:
        ins.mnemonic = "b" if ins.mnemonic == "bgez" else "bal"
        ins.cond = False
        del ins.operands[0]


def _p_neg(ins, w, reads, writes):
    if (w >> 21) & 31 == 0:
        ins.mnemonic = {"sub": "neg", "subu": "negu", "dsub": "dneg", "dsubu": "dnegu"}[ins.mnemonic]
        del ins.operands[1]


def _p_not(ins, w, reads, writes):
    if (w >> 16) & 31 == 0:
        ins.mnemonic = "not"
        del ins.operands[2]


def _p_jalr(ins, w, reads, writes):
    if (w >> 11) & 31 == 31:
        del ins.operands[0]


def _p_rd0(ins, w, reads, writes):
    # EE three-operand multiplies print as the classic two-operand form
    # when rd is $zero.
    if (w >> 11) & 31 == 0:
        del ins.operands[0]


def _p_sll(ins, w, reads, writes):
    if w == 0:
        ins.mnemonic = "nop"
        ins.operands = []
        ins.imm = None


_PSEUDO = {"addu": _p_move, "daddu": _p_move, "or": _p_move,
           "addiu": _p_li, "ori": _p_li,
           "beq": _p_beq, "bne": _p_beq, "beql": _p_beq, "bnel": _p_beq,
           "bgez": _p_bgez, "bgezal": _p_bgez,
           "sub": _p_neg, "subu": _p_neg, "dsub": _p_neg, "dsubu": _p_neg,
           "nor": _p_not, "jalr": _p_jalr, "sll": _p_sll}
for _mn in ("mult", "multu", "madd", "maddu", "mult1", "multu1", "madd1", "maddu1"):
    _PSEUDO[_mn] = _p_rd0


# ---------------------------------------------------------------------------
# Address tracking: which lui values reach which addiu/ori/load/store.

# o32 caller-saved: a call leaves nothing we tracked in these.
CLOBBERED = frozenset(("$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3", "$t0", "$t1",
                       "$t2", "$t3", "$t4", "$t5", "$t6", "$t7", "$t8", "$t9", "$ra"))
MOVES = ("move",)


def _step(ins, st, out):
    """Apply one instruction to the register state {reg: value from lui}."""
    mn = ins.mnemonic
    if mn == "lui":
        st[GPR[ins.rt]] = ins.imm << 16
        return
    if ins.base is not None and ins.base in st:
        out[ins.pc] = (st[ins.base] + ins.offset) & 0xffffffff
    elif mn in ("addiu", "daddiu", "addi", "daddi") and GPR[ins.rs] in st:
        out[ins.pc] = (st[GPR[ins.rs]] + ins.imm) & 0xffffffff
    elif mn == "ori" and GPR[ins.rs] in st:
        out[ins.pc] = st[GPR[ins.rs]] | ins.imm
    for r in ins.writes:
        st.pop(r, None)
    if mn in MOVES and ins.reads and ins.reads[0] in st:
        st[ins.writes[0]] = st[ins.reads[0]]


def track_lui(insns, gp=None):
    """{pc: address} for every addiu/ori/load/store whose base register
    holds a lui value (or $gp) on all paths reaching it. Forward dataflow
    over the basic blocks of `insns` (one function), meeting by agreement."""
    n = len(insns)
    if not n:
        return {}
    index = {ins.pc: k for k, ins in enumerate(insns)}
    leaders = {0}
    for k, ins in enumerate(insns):
        if ins.delay:
            if k + 2 < n:
                leaders.add(k + 2)
            if ins.target in index and not ins.call:
                leaders.add(index[ins.target])
    order = sorted(leaders)
    end = {b: (order[i + 1] if i + 1 < len(order) else n) for i, b in enumerate(order)}
    init = {"$gp": gp} if gp is not None else {}
    state = {}

    def run(b, st, out):
        """Run block b; return [(successor, state)]."""
        edges = []
        k = b
        while k < end[b]:
            ins = insns[k]
            if ins.delay and k + 1 < n:
                before = dict(st)
                _step(ins, st, out)
                _step(insns[k + 1], st, out)
                if ins.call:
                    for r in CLOBBERED:
                        st.pop(r, None)
                    if k + 2 < n:
                        edges.append((k + 2, st))
                    return edges
                if ins.target in index:
                    edges.append((index[ins.target], dict(st)))
                if ins.cond and k + 2 < n:
                    edges.append((k + 2, before if ins.likely else st))
                return edges
            _step(ins, st, out)
            k += 1
        if k < n:
            edges.append((k, st))
        return edges

    scratch = {}
    pending = [0]
    state[0] = dict(init)
    for seed in order + [None]:
        while pending:
            b = pending.pop()
            for succ, st in run(b, dict(state[b]), scratch):
                cur = state.get(succ)
                if cur is None:
                    state[succ] = st
                    pending.append(succ)
                else:
                    merged = {r: v for r, v in cur.items() if st.get(r) == v}
                    if merged != cur:
                        state[succ] = merged
                        pending.append(succ)
        # Blocks reached only through jr (jump tables) start from scratch.
        if seed is not None and seed not in state:
            state[seed] = dict(init)
            pending.append(seed)
    out = {}
    for b in order:
        run(b, dict(state.get(b, init)), out)
    return out



def disassemble(data, pc):
    """Yield an Insn for every aligned word of `data`, starting at `pc`."""
    for i in range(0, len(data) - 3, 4):
        yield decode(int.from_bytes(data[i:i + 4], "little"), pc + i)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("word")
    p.add_argument("words", nargs="+", type=lambda s: int(s, 16))
    p.add_argument("--pc", type=lambda s: int(s, 0), default=0)
    p = sub.add_parser("hex")
    p.add_argument("bytes")
    p.add_argument("--pc", type=lambda s: int(s, 0), default=0)
    args = parser.parse_args()

    if args.cmd == "word":
        words = args.words
    else:
        raw = bytes.fromhex(args.bytes.replace(" ", ""))
        words = [int.from_bytes(raw[i:i + 4], "little") for i in range(0, len(raw) - 3, 4)]
    for i, w in enumerate(words):
        ins = decode(w, args.pc + 4 * i)
        extra = []
        if ins.target is not None:
            extra.append(f"target=0x{ins.target:08x}")
        if ins.reads:
            extra.append("r=" + ",".join(ins.reads))
        if ins.writes:
            extra.append("w=" + ",".join(ins.writes))
        tail = f"  ; {' '.join(extra)}" if extra else ""
        print(f"{ins.pc:08x}  {w:08x}  {ins.text}{tail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
