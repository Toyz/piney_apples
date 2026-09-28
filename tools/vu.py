#!/usr/bin/env python3
"""VU0/VU1 micro-mode disassembler, and the microcode embedded in the EE ELF.

A micro instruction is 64 bits: the lower word (load/store, integer, branch,
EFU, XGKICK) at +0 and the upper word (FMAC vector ops) at +4, both issued
together. The upper word's top five bits are the I, E, M, D and T flags; with
I set the lower word is not an instruction but a float for the I register
(shown as `loi`). E ends the program after the next pair; branches have one
delay slot. Micro-addresses count 64-bit pairs, as MSCAL and the branch
offsets do.

    tools/vu.py pair   LOWER UPPER [--pc N]                 decode one pair
    tools/vu.py blobs  ELF                                  the microcode blobs and their MPG uploads
    tools/vu.py labels ELF [--check]                        label -> micro-address map
    tools/vu.py dis    ELF [LABEL|MICROADDR] [--vu0] [--end N]
    tools/vu.py dump   ELF OUTDIR                           write every listing to OUTDIR
    tools/vu.py check  ELF                                  decode sanity over all microcode
    tools/vu.py packet ELF VERTICES MTYPE                   a rigid mmat's DMA/VIF list, as built

In INF SLUS_202.67 the microcode is assembled into `main`: the VU1 code is
a VIF stream of MPG uploads between `ccInitDL_dsmS` and `ccInitDL_dsmE`, the
VU0 code a DMA chain at `mcVu0_Top`. The assembler left a local symbol
`_$<label>` at the EE address of every micro label, and `.vif.N`/`.dma.N`
at the VIF codes and DMA tags; EE code names micro-addresses through
value-0 symbols (`mc_DrawTri`) with HI16/LO16 pairs holding the *byte*
address in micro memory. `labels --check` cross-checks the three.

Also a library: `decode(lower, upper, pc)` returns a `Pair` with an `Op` for
each half; `Microcode(elf_path)` finds the blobs, `.vu1`/`.vu0` are `Blob`s
with `.pairs`, `.labels`, and `.listing(start, end)`; `Vu(blob)` runs a
blob's micro programs on its own data memory (the instructions the model
programs use; tools/test_demo_rs.py runs the lit path with it).
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import vif  # noqa: E402

VF = tuple(f"vf{i:02d}" for i in range(32))
VI = tuple(f"vi{i:02d}" for i in range(32))
BC = "xyzw"


def _dest(w):
    d = (w >> 21) & 15
    return "".join(c for i, c in enumerate(BC) if d & (8 >> i))


def _sext(v, bits):
    return v - (1 << bits) if v & (1 << (bits - 1)) else v


class Op:
    """One half of a pair. `target` is a branch destination (micro-address),
    `kind` one of None, "branch" (conditional), "jump" (b), "call" (bal/jalr),
    "jr" (register jump). `valid` is False for encodings that are not
    instructions or have nonzero must-be-zero fields."""
    __slots__ = ("mnemonic", "operands", "target", "kind", "valid", "imm")

    def __init__(self, mnemonic, operands=(), valid=True):
        self.mnemonic = mnemonic
        self.operands = list(operands)
        self.target = None
        self.kind = None
        self.valid = valid
        self.imm = None

    @property
    def text(self):
        if not self.operands:
            return self.mnemonic
        return f"{self.mnemonic:<11} {', '.join(self.operands)}"

    def __str__(self):
        return self.text


def _bad(w):
    return Op(f".word 0x{w:08x}", valid=False)


# ---------------------------------------------------------------------------
# upper

_UBC = {0x00: "add", 0x04: "sub", 0x08: "madd", 0x0c: "msub", 0x10: "max",
        0x14: "mini", 0x18: "mul"}
_UQI = {0x1c: "mulq", 0x1d: "maxi", 0x1e: "muli", 0x1f: "minii", 0x20: "addq",
        0x21: "maddq", 0x22: "addi", 0x23: "maddi", 0x24: "subq", 0x25: "msubq",
        0x26: "subi", 0x27: "msubi"}
_U3 = {0x28: "add", 0x29: "madd", 0x2a: "mul", 0x2b: "max", 0x2c: "sub",
       0x2d: "msub", 0x2e: "opmsub", 0x2f: "mini"}
# 11-bit opcodes behind 0x3c-0x3f: bits 10..6 then 1..0.
_UABC = {0x00: "adda", 0x04: "suba", 0x08: "madda", 0x0c: "msuba", 0x18: "mula"}
_UAQI = {0x1c: "mulaq", 0x1e: "mulai", 0x20: "addaq", 0x21: "maddaq", 0x22: "addai",
         0x23: "maddai", 0x24: "subaq", 0x25: "msubaq", 0x26: "subai", 0x27: "msubai"}
_UA3 = {0x28: "adda", 0x29: "madda", 0x2a: "mula", 0x2c: "suba", 0x2d: "msuba",
        0x2e: "opmula"}
FLAGS = "IEMDT"


def upper(w):
    """Decode an upper word, flags excluded."""
    if w & 0x06000000:  # bits 25-26 are not used
        return _bad(w)
    op = w & 63
    dest = _dest(w)
    ft, fs, fd = VF[(w >> 16) & 31], VF[(w >> 11) & 31], VF[(w >> 6) & 31]
    if op < 0x1c:
        base, bc = op & ~3, BC[op & 3]
        return Op(f"{_UBC[base]}{bc}.{dest}", (fd, fs, ft + bc))
    if op in _UQI:
        src = "Q" if _UQI[op].endswith("q") else "I"
        return Op(f"{_UQI[op]}.{dest}", (fd, fs, src))
    if op in _U3:
        return Op(f"{_U3[op]}.{dest}", (fd, fs, ft))
    if op < 0x3c:
        return _bad(w)
    ext = (((w >> 6) & 31) << 2) | (w & 3)
    if ext < 0x10 or 0x18 <= ext < 0x1c:
        base, bc = ext & ~3, BC[ext & 3]
        return Op(f"{_UABC[base]}{bc}.{dest}", ("ACC", fs, ft + bc))
    if 0x10 <= ext < 0x18:
        kind = "itof" if ext < 0x14 else "ftoi"
        return Op(f"{kind}{(0, 4, 12, 15)[ext & 3]}.{dest}", (ft, fs))
    if ext in _UAQI:
        src = "Q" if _UAQI[ext].endswith("q") else "I"
        return Op(f"{_UAQI[ext]}.{dest}", ("ACC", fs, src))
    if ext in _UA3:
        return Op(f"{_UA3[ext]}.{dest}", ("ACC", fs, ft))
    if ext == 0x1d:
        return Op(f"abs.{dest}", (ft, fs))
    if ext == 0x1f:
        return Op(f"clipw.{dest}", (fs, ft + "w"))
    if ext == 0x2f:
        return Op("nop", valid=not (w & 0x01fff800))
    return _bad(w)


# ---------------------------------------------------------------------------
# lower

def _fsf(w):
    return BC[(w >> 21) & 3]


def _ftf(w):
    return BC[(w >> 23) & 3]


def _branch(op, pc, w):
    if pc is not None:
        op.target = pc + 1 + _sext(w & 0x7ff, 11)
        op.operands.append(f"0x{op.target:03x}")
    else:
        op.operands.append(str(_sext(w & 0x7ff, 11)))
    return op


def lower(w, pc=None):
    """Decode a lower word. `pc` (a micro-address) resolves branch targets."""
    op7 = w >> 25
    dest = _dest(w)
    it = VI[(w >> 16) & 31]
    is_ = VI[(w >> 11) & 31]
    ft, fs = VF[(w >> 16) & 31], VF[(w >> 11) & 31]
    imm11 = _sext(w & 0x7ff, 11)
    F_DEST, F_T, F_S, F_LOW11 = 0x01e00000, 0x001f0000, 0x0000f800, 0x7ff

    def must(mask, op):
        if w & mask:
            op.valid = False
            op.mnemonic += "?"
        return op

    if op7 == 0x00:
        return Op(f"lq.{dest}", (ft, f"{imm11}({is_})"))
    if op7 == 0x01:
        return Op(f"sq.{dest}", (fs, f"{imm11}({it})"))
    if op7 == 0x04:
        return Op(f"ilw.{dest}", (it, f"{imm11}({is_})"))
    if op7 == 0x05:
        return Op(f"isw.{dest}", (it, f"{imm11}({is_})"))
    if op7 in (0x08, 0x09):
        imm = (((w >> 21) & 15) << 11) | (w & 0x7ff)
        o = Op(("iaddiu", "isubiu")[op7 - 8], (it, is_, f"0x{imm:x}"))
        o.imm = imm
        return o
    if 0x10 <= op7 <= 0x13:
        name = ("fceq", "fcset", "fcand", "fcor")[op7 - 0x10]
        args = [f"0x{w & 0xffffff:06x}"]
        if op7 != 0x11:
            args.insert(0, "vi01")
        return must(0x01000000, Op(name, args))
    if 0x14 <= op7 <= 0x17:
        imm12 = (((w >> 21) & 1) << 11) | (w & 0x7ff)
        name = ("fseq", "fsset", "fsand", "fsor")[op7 - 0x14]
        args = [f"0x{imm12:03x}"]
        if op7 != 0x15:
            args.insert(0, it)
            return must(0x01c00000 | F_S, Op(name, args))
        return must(0x01c00000 | F_S | F_T, Op(name, args))
    if op7 in (0x18, 0x1a, 0x1b):
        name = {0x18: "fmeq", 0x1a: "fmand", 0x1b: "fmor"}[op7]
        return must(F_DEST | F_LOW11, Op(name, (it, is_)))
    if op7 == 0x1c:
        return must(F_DEST | F_S | F_LOW11, Op("fcget", (it,)))
    if op7 == 0x20:
        o = _branch(Op("b"), pc, w)
        o.kind = "jump"
        return must(F_DEST | F_T | F_S, o)
    if op7 == 0x21:
        o = _branch(Op("bal", (it,)), pc, w)
        o.kind = "call"
        return must(F_DEST | F_S, o)
    if op7 == 0x24:
        o = Op("jr", (is_,))
        o.kind = "jr"
        return must(F_DEST | F_T | F_LOW11, o)
    if op7 == 0x25:
        o = Op("jalr", (it, is_))
        o.kind = "call"
        return must(F_DEST | F_LOW11, o)
    if op7 in (0x28, 0x29):
        o = _branch(Op(("ibeq", "ibne")[op7 - 0x28], (it, is_)), pc, w)
        o.kind = "branch"
        return must(F_DEST, o)
    if 0x2c <= op7 <= 0x2f:
        o = _branch(Op(("ibltz", "ibgtz", "iblez", "ibgez")[op7 - 0x2c], (is_,)), pc, w)
        o.kind = "branch"
        return must(F_DEST | F_T, o)
    if op7 != 0x40:
        return _bad(w)

    sub = w & 63
    id_ = VI[(w >> 6) & 31]
    if sub in (0x30, 0x31, 0x34, 0x35):
        name = {0x30: "iadd", 0x31: "isub", 0x34: "iand", 0x35: "ior"}[sub]
        return must(F_DEST, Op(name, (id_, is_, it)))
    if sub == 0x32:
        o = Op("iaddi", (it, is_, str(_sext((w >> 6) & 31, 5))))
        o.imm = _sext((w >> 6) & 31, 5)
        return must(F_DEST, o)
    if sub < 0x3c:
        return _bad(w)
    ext = (((w >> 6) & 31) << 2) | (w & 3)
    if ext == 0x30:
        if w == 0x8000033c:
            return Op("nop")
        return Op(f"move.{dest}", (ft, fs))
    if ext == 0x31:
        return Op(f"mr32.{dest}", (ft, fs))
    if ext == 0x34:
        return Op(f"lqi.{dest}", (ft, f"({is_}++)"))
    if ext == 0x35:
        return Op(f"sqi.{dest}", (fs, f"({it}++)"))
    if ext == 0x36:
        return Op(f"lqd.{dest}", (ft, f"(--{is_})"))
    if ext == 0x37:
        return Op(f"sqd.{dest}", (fs, f"(--{it})"))
    if ext == 0x38:
        return Op("div", ("Q", fs + _fsf(w), ft + _ftf(w)))
    if ext == 0x39:
        return must(0x00600000 | F_S, Op("sqrt", ("Q", ft + _ftf(w))))
    if ext == 0x3a:
        return Op("rsqrt", ("Q", fs + _fsf(w), ft + _ftf(w)))
    if ext == 0x3b:
        return must(F_DEST | F_T | F_S, Op("waitq"))
    if ext == 0x3c:
        return must(0x01800000, Op("mtir", (it, fs + _fsf(w))))
    if ext == 0x3d:
        return Op(f"mfir.{dest}", (ft, is_))
    if ext == 0x3e:
        return Op(f"ilwr.{dest}", (it, f"({is_})"))
    if ext == 0x3f:
        return Op(f"iswr.{dest}", (it, f"({is_})"))
    if ext in (0x40, 0x41):
        return must(F_S, Op(f"{('rnext', 'rget')[ext - 0x40]}.{dest}", (ft, "R")))
    if ext in (0x42, 0x43):
        return must(0x01800000 | F_T, Op(("rinit", "rxor")[ext - 0x42], ("R", fs + _fsf(w))))
    if ext == 0x64:
        return must(F_S, Op(f"mfp.{dest}", (ft, "P")))
    if ext in (0x68, 0x69):
        return must(F_DEST | F_S, Op(("xtop", "xitop")[ext - 0x68], (it,)))
    if ext == 0x6c:
        return must(F_DEST | F_T, Op("xgkick", (is_,)))
    efu_v = {0x70: "esadd", 0x71: "ersadd", 0x72: "eleng", 0x73: "erleng",
             0x74: "eatanxy", 0x75: "eatanxz", 0x76: "esum"}
    efu_s = {0x78: "esqrt", 0x79: "ersqrt", 0x7a: "ercpr", 0x7c: "esin",
             0x7d: "eatan", 0x7e: "eexp"}
    if ext in efu_v:
        return must(F_T, Op(efu_v[ext], ("P", fs)))
    if ext in efu_s:
        return must(0x01800000 | F_T, Op(efu_s[ext], ("P", fs + _fsf(w))))
    if ext == 0x7b:
        return must(F_DEST | F_T | F_S, Op("waitp"))
    return _bad(w)


# ---------------------------------------------------------------------------
# pairs

class Pair:
    __slots__ = ("addr", "va", "lo", "hi", "upper", "lower", "flags")

    def __init__(self, addr, va, lo, hi):
        self.addr, self.va, self.lo, self.hi = addr, va, lo, hi
        self.flags = "".join(f for i, f in enumerate(FLAGS) if hi >> (31 - i) & 1)
        self.upper = upper(hi & 0x07ffffff)
        if "I" in self.flags:
            f = struct.unpack("<f", struct.pack("<I", lo))[0]
            self.lower = Op("loi", (f"0x{lo:08x}",))
            self.lower.imm = f
        else:
            self.lower = lower(lo, addr)

    @property
    def valid(self):
        return self.upper.valid and self.lower.valid

    @property
    def end(self):
        return "E" in self.flags

    def text(self, width=40):
        u = self.upper.text + (f"[{self.flags}]" if self.flags else "")
        lo = self.lower.text
        if self.lower.mnemonic == "loi":
            lo += f"  # {self.lower.imm:g}"
        return f"{u:<{width}} {lo}"


def decode(lo, hi, pc=None):
    return Pair(pc, None, lo, hi)


# ---------------------------------------------------------------------------
# the blobs in the ELF

VU1_WORDS, VU0_WORDS = 2048, 512  # 16 KiB and 4 KiB of micro memory


class Blob:
    """One VU's microcode as uploaded: micro-address -> Pair, plus labels.
    `mpgs` is [(micro_addr, count, ee_va_of_first_pair)]; `vif` the VIF codes
    that carried it."""

    def __init__(self, name, unit, size):
        self.name = name
        self.unit = unit
        self.size = size
        self.pairs = {}
        self.mpgs = []
        self.vif = []
        self.labels = {}        # name -> micro-address
        self.globals = set()    # names the EE refers to (entry points)
        self.at = {}            # micro-address -> [names]
        self.relocs = {}        # micro-address -> (type, symbol name)
        self.range = None       # (first, end) EE addresses of the upload

    def load(self, stream):
        for v in vif.codes(stream.data):
            self.vif.append((stream.va(v.off), v))
            if v.cmd != 0x4a:
                continue
            n = len(v.data) // 8
            first = stream.va(v.off + v.size - len(v.data))
            self.mpgs.append((v.imm, n, first))
            for i in range(n):
                lo, hi = struct.unpack_from("<II", v.data, i * 8)
                a = v.imm + i
                self.pairs[a] = Pair(a, first + i * 8, lo, hi)

    def addr_of_va(self, va):
        for a, n, first in self.mpgs:
            if first <= va < first + n * 8 and (va - first) % 8 == 0:
                return a + (va - first) // 8
        return None

    def add_label(self, name, addr):
        self.labels[name] = addr
        self.at.setdefault(addr, []).append(name)

    def label_for(self, addr):
        names = self.at.get(addr)
        if not names:
            return None
        # Prefer an entry point, then the first defined.
        return next((n for n in names if n in self.globals), names[0])

    def programs(self):
        """(name, start, end) for every entry point, running to the next
        entry point or the end of the uploaded code."""
        starts = sorted({self.labels[n] for n in self.globals})
        top = max(self.pairs) + 1
        out = []
        for i, s in enumerate(starts):
            e = starts[i + 1] if i + 1 < len(starts) else top
            names = [n for n in self.at[s] if n in self.globals]
            out.append((" / ".join(names), s, e))
        return out

    def listing(self, start=None, end=None, width=40):
        start = min(self.pairs) if start is None else start
        end = max(self.pairs) + 1 if end is None else end
        out = []
        for a in range(start, end):
            p = self.pairs.get(a)
            if p is None:
                continue
            for n in self.at.get(a, ()):
                tag = "  ; entry" if n in self.globals else ""
                out.append(f"{n}:{tag}")
            txt = p.text(width)
            t = p.lower.target
            if t is not None and self.label_for(t):
                txt = txt.replace(f"0x{t:03x}", self.label_for(t))
            note = []
            if a in self.relocs:
                rt, sym = self.relocs[a]
                if sym in self.labels:
                    ok = "" if p.lower.imm == self.labels[sym] else " MISMATCH"
                    note.append(f"R_MIPS_DVP_U15_S3 {sym} = 0x{self.labels[sym]:03x}{ok}")
                else:
                    note.append(f"reloc {rt} {sym}")
            if p.lower.kind == "jr":
                note.append("return / indirect")
            line = f"  {a:04x} {p.va:08x}  {txt}"
            if note:
                line += "    # " + "; ".join(note)
            out.append(line)
            if p.end:
                out.append("                    (E: ends after the next pair)")
        return out


class Microcode:
    """The VU1 and VU0 microcode of main, with labels from the symbol table."""

    def __init__(self, elf_path):
        from elf import Elf
        self.elf = elf = Elf(elf_path)
        sym = {}
        for s in elf.symbols:
            if s.name and s.name not in sym and (s.value or s.shndx == 0):
                sym[s.name] = s
        self.sym = sym
        self.vu1 = Blob("vu1", 1, VU1_WORDS)
        self.vu0 = Blob("vu0", 0, VU0_WORDS)
        if "ccInitDL_dsmS" in sym:
            s, e = sym["ccInitDL_dsmS"].value, sym["ccInitDL_dsmE"].value
            self.vu1.load(vif.Stream([(s, elf.read(s, e - s))]))
            self.vu1.range = (s, e)
        if "mcVu0_Top" in sym:
            segs, end = [], None
            for tva, tag, ss in vif.chain(elf.read, sym["mcVu0_Top"].value):
                segs += ss
                end = tva + 16 + tag.qwc * 16
            self.vu0.load(vif.Stream(segs))
            self.vu0.range = (sym["mcVu0_Top"].value, end)
        self._labels()

    def blob_of_va(self, va):
        for b in (self.vu1, self.vu0):
            if b.pairs and b.addr_of_va(va) is not None:
                return b
        return None

    def _labels(self):
        elf = self.elf
        # Value-0 symbols with the label's name: GLOBAL ones are what EE code
        # references, so they are the entry points.
        zero = {}
        for s in elf.symbols:
            if s.value == 0 and s.name.startswith("mc"):
                zero[s.name] = s
        for s in elf.symbols:
            if not s.name.startswith("_$"):
                continue
            b = self.blob_of_va(s.value)
            if b is None:
                continue
            name = s.name[2:]
            b.add_label(name, b.addr_of_va(s.value))
            z = zero.get(name)
            if z is not None and z.bind == 1:
                b.globals.add(name)
        for b in (self.vu1, self.vu0):
            for va, rtype, s in elf.relocs("main"):
                a = b.addr_of_va(va)
                if a is not None:
                    b.relocs[a] = (rtype, s.name)

    def ee_refs(self):
        """(ee_va_of_lui, name, micro byte address) for every HI16/LO16 pair
        in main whose symbol is a value-0 micro label."""
        from image import Program
        prog = Program(self.elf.path)
        out = []
        for r in prog.relocs:
            if r.type == 5 and r.symbol.value == 0 and r.symbol.shndx == 0 \
                    and r.symbol.name.startswith("mc"):
                out.append((r.offset, r.symbol.name, r.addr))
        return out

    def find(self, what):
        """(blob, micro-address) for a label name or a number (VU1 unless
        the label lives in VU0)."""
        for b in (self.vu1, self.vu0):
            if what in b.labels:
                return b, b.labels[what]
        try:
            return None, int(what, 0)
        except ValueError:
            raise SystemExit(f"no micro label {what!r}")


# ---------------------------------------------------------------------------
# a rigid mmat's DMA list, built by the game's own code

MODEL_TAGGERS = ("ccModelDmaTag_SetTag__FP10_sceDmaTagUiUs",
                 "ccModelDmaTag_SetVertex__FP10_sceDmaTagUiUsP16ccModelDmaVertex",
                 "ccModelDmaTag_SetNormal__FP10_sceDmaTagUiUsP16ccModelDmaNormal",
                 "ccModelDmaTag_SetColor__FP10_sceDmaTagUiUsP15ccModelDmaColor",
                 "ccModelDmaTag_SetST__FP10_sceDmaTagUiUsP12ccModelDmaST")
# Where the fake mmat arrays sit in the emulated RAM: vertex, normal, colour, ST.
_ARRAYS = (0x01100000, 0x01200000, 0x01300000, 0x01400000)
_LIST = 0x01000000


def model_packet(elf_path, vertex_num, mtype):
    """Run the five ccModelDmaTag_Set* functions the way
    ccModelDmaData_SetDmaTag (INF SLUS_202.67:0x0013de90) does, for a mmat of
    `vertex_num` vertices and model type `mtype`, and return the DMA list
    bytes, its address and the `(tag_va, DmaTag, segments)` walk. The list
    ends in a RET tag: ccSetModelPacket CALLs it each frame."""
    import eemu
    from image import Program
    prog = Program(elf_path)
    m = eemu.Machine(prog)
    m.call(prog.symbol_named(MODEL_TAGGERS[0]).value, (_LIST, vertex_num, mtype))
    for name, data in zip(MODEL_TAGGERS[1:], _ARRAYS):
        m.call(prog.symbol_named(name).value, (_LIST, vertex_num, mtype, data))

    def read(va, n):  # the image is loaded into the machine's RAM too
        return bytes(m.mem[va:va + n])
    walk = list(vif.chain(read, _LIST))
    return walk


def packet_listing(walk):
    out = []
    for tva, tag, segs in walk:
        out.append(f"{tva:08x}  DMAtag {tag.text}")
        st = vif.Stream(segs)
        for v in vif.codes(st.data):
            line = f"   {st.va(v.off):08x}  {v.word:08x}  {v.text}"
            if v.data and v.cmd != 0x4a:
                line += f"   data @0x{st.va(v.off + v.size - len(v.data)):08x}"
            out.append(line)
            if v.is_unpack and v.vn == 3 and v.vl == 0 and v.num == 1 and len(v.data) == 16:
                out.append(f"              {vif.giftag(v.data).text}")
    return out


# ---------------------------------------------------------------------------
# running micro programs

_ONE = 0x3F800000
_LANES = (8, 4, 2, 1)  # dest bits of x, y, z, w


class Vu:
    """A micro-mode VU running a `Blob`'s pairs: enough of the instruction
    set for the model programs (`mc_SetMatrix`, `mc_SetObjParam`, the lit
    and unlit rigid prologues and triangle loops).

    Sequential, one pair at a time: both halves of a pair read the
    registers as they stood before it, then both write. There is no
    pipeline: the hardware stalls an FMAC or integer read on a result still
    in flight, which sequential order gives; Q and P are read as if DIV and
    the EFU had finished (the programs wait with WAITP, or put DIV seven
    pairs ahead of its MULQ). Floats go through tools/eemu.py's FPU model
    (a multiply-add rounds its product first). The MAC flag keeps only the
    zero and sign bits. `mem` is qwords of four 32-bit words; `kicks` the
    addresses XGKICK sent. E ends a program after the next pair."""

    def __init__(self, blob, qwords=1024):
        self.blob = blob
        self.mem = [[0, 0, 0, 0] for _ in range(qwords)]
        self.vf = [[0, 0, 0, 0] for _ in range(32)]
        self.vf[0] = [0, 0, 0, _ONE]
        self.vi = [0] * 16
        self.acc = [0, 0, 0, 0]
        self.q = self.p = self.i = 0
        self.mac = 0
        self.top = 0
        self.kicks = []

    def run(self, start, limit=100_000):
        import eemu
        self._e = eemu
        pc, branch, ending, steps = start, None, False, 0
        while True:
            steps += 1
            if steps > limit:
                raise RuntimeError(f"VU gave up at 0x{pc:03x}")
            pair = self.blob.pairs.get(pc)
            if pair is None:
                raise RuntimeError(f"VU ran off the code at 0x{pc:03x}")
            target = self._pair(pair.lo, pair.hi, pc)
            if ending:
                return
            ending = bool(pair.hi >> 30 & 1)
            if branch is not None:
                pc, branch = branch, None
            else:
                pc += 1
            if target is not None:
                branch = target

    # one pair ---------------------------------------------------------------
    def _pair(self, lo, hi, pc):
        vf0 = [list(v) for v in self.vf]
        vi0 = list(self.vi)
        acc0 = list(self.acc)
        self._upper(hi & 0x07FFFFFF, vf0, acc0)
        if hi >> 31 & 1:  # I: the lower word is a float for I
            self.i = lo
            return None
        return self._lower(lo, pc, vf0, vi0)

    def _setf(self, r, dest, vals):
        if r == 0:
            return
        for k in range(4):
            if dest & _LANES[k]:
                self.vf[r][k] = vals[k] & 0xFFFFFFFF

    def _flags(self, dest, vals):
        mac = 0
        for k in range(4):
            if dest & _LANES[k]:
                v = vals[k] & 0xFFFFFFFF
                if not v & 0x7FFFFFFF:
                    mac |= 1 << (3 - k)
                if v & 0x80000000:
                    mac |= 1 << (7 - k)
        self.mac = mac

    def _upper(self, w, vf, acc):
        e = self._e
        op = w & 63
        dest = (w >> 21) & 15
        ft, fs, fd = (w >> 16) & 31, (w >> 11) & 31, (w >> 6) & 31
        a, b = vf[fs], vf[ft]

        def lanes(fn, x, y):
            return [fn(x[k], y[k]) for k in range(4)]

        def fmax(x, y):
            return x if e.f_cmp(x, y) >= 0 else y

        def fmin(x, y):
            return x if e.f_cmp(x, y) <= 0 else y

        def madd(x, y):
            return lanes(e.f_add, acc, lanes(e.f_mul, x, y))

        def msub(x, y):
            return lanes(e.f_sub, acc, lanes(e.f_mul, x, y))

        kinds = {"add": lambda x, y: lanes(e.f_add, x, y), "sub": lambda x, y: lanes(e.f_sub, x, y),
                 "mul": lambda x, y: lanes(e.f_mul, x, y), "max": lambda x, y: lanes(fmax, x, y),
                 "mini": lambda x, y: lanes(fmin, x, y), "madd": madd, "msub": msub}
        if op < 0x3C:
            if op < 0x1C:
                name, y = _UBC[op & ~3], [b[op & 3]] * 4
            elif op in _UQI:
                name = _UQI[op]
                y = [self.q if name.endswith("q") else self.i] * 4
                name = name[:-1]
            elif op in _U3 and op != 0x2E:
                name, y = _U3[op], b
            elif op == 0x2E:  # opmsub
                prod = [e.f_mul(a[1], b[2]), e.f_mul(a[2], b[0]), e.f_mul(a[0], b[1]), 0]
                res = [e.f_sub(acc[k], prod[k]) for k in range(3)] + [0]
                self._setf(fd, dest & 14, res)
                self._flags(dest & 14, res)
                return
            else:
                raise RuntimeError(f"VU upper 0x{w:08x} is not interpreted")
            res = kinds[name](a, y)
            self._setf(fd, dest, res)
            self._flags(dest, res)
            return
        ext = (((w >> 6) & 31) << 2) | (w & 3)
        if ext == 0x2F:  # nop
            return
        if ext < 0x10 or 0x18 <= ext < 0x1C:
            name, y = _UABC[ext & ~3], [b[ext & 3]] * 4
        elif ext in _UAQI:
            name = _UAQI[ext]
            y = [self.q if name.endswith("q") else self.i] * 4
            name = name[:-1]
        elif ext in _UA3 and ext != 0x2E:
            name, y = _UA3[ext], b
        elif ext == 0x2E:  # opmula
            res = [e.f_mul(a[1], b[2]), e.f_mul(a[2], b[0]), e.f_mul(a[0], b[1]), self.acc[3]]
            for k in range(3):
                if dest & _LANES[k]:
                    self.acc[k] = res[k]
            return
        elif 0x10 <= ext < 0x14:  # itof0/4/12/15
            sh = (0, 4, 12, 15)[ext & 3]
            res = [e.f_from_int(e.sx(x, 32)) for x in a]
            if sh:
                res = [e.f_div(x, e.f_from_int(1 << sh)) for x in res]
            self._setf(ft, dest, res)
            return
        elif 0x14 <= ext < 0x18:  # ftoi0/4/12/15
            sh = (0, 4, 12, 15)[ext & 3]
            res = [e.f_to_int(e.f_mul(x, e.f_from_int(1 << sh)) if sh else x) for x in a]
            self._setf(ft, dest, res)
            return
        elif ext == 0x1D:  # abs
            self._setf(ft, dest, [x & 0x7FFFFFFF for x in a])
            return
        else:
            raise RuntimeError(f"VU upper 0x{w:08x} is not interpreted")
        # the ACC forms: name is adda/suba/madda/msuba/mula
        res = kinds[{"adda": "add", "suba": "sub", "madda": "madd", "msuba": "msub",
                     "mula": "mul"}[name]](a, y)
        for k in range(4):
            if dest & _LANES[k]:
                self.acc[k] = res[k] & 0xFFFFFFFF
        self._flags(dest, res)

    def _lower(self, w, pc, vf, vi):
        e = self._e
        op7 = w >> 25
        dest = (w >> 21) & 15
        it, is_ = (w >> 16) & 15, (w >> 11) & 15
        ft, fs = (w >> 16) & 31, (w >> 11) & 31
        imm11 = _sext(w & 0x7FF, 11)
        size = len(self.mem)

        def seti(r, v):
            if r:
                self.vi[r] = v & 0xFFFF

        def target():
            return pc + 1 + imm11

        def s16(v):
            return _sext(v & 0xFFFF, 16)

        if op7 == 0x00:  # lq
            self._setf(ft, dest, self.mem[(vi[is_] + imm11) % size])
            return None
        if op7 == 0x01:  # sq
            q = self.mem[(vi[it] + imm11) % size]
            for k in range(4):
                if dest & _LANES[k]:
                    q[k] = vf[fs][k]
            return None
        if op7 == 0x04:  # ilw
            q = self.mem[(vi[is_] + imm11) % size]
            k = next(k for k in range(4) if dest & _LANES[k])
            seti(it, q[k])
            return None
        if op7 == 0x05:  # isw
            q = self.mem[(vi[is_] + imm11) % size]
            for k in range(4):
                if dest & _LANES[k]:
                    q[k] = vi[it] & 0xFFFF
            return None
        if op7 in (0x08, 0x09):  # iaddiu / isubiu
            imm = (((w >> 21) & 15) << 11) | (w & 0x7FF)
            seti(it, vi[is_] + imm if op7 == 0x08 else vi[is_] - imm)
            return None
        if op7 in (0x18, 0x1A, 0x1B):  # fmeq / fmand / fmor
            m = vi[is_] & 0xFFFF
            seti(it, {0x18: int(m == self.mac), 0x1A: m & self.mac, 0x1B: m | self.mac}[op7])
            return None
        if op7 == 0x20:  # b
            return target()
        if op7 == 0x21:  # bal
            seti(it, pc + 2)
            return target()
        if op7 == 0x24:  # jr
            return vi[is_]
        if op7 == 0x25:  # jalr
            seti(it, pc + 2)
            return vi[is_]
        if op7 in (0x28, 0x29):  # ibeq / ibne
            eq = (vi[it] & 0xFFFF) == (vi[is_] & 0xFFFF)
            return target() if eq == (op7 == 0x28) else None
        if 0x2C <= op7 <= 0x2F:
            v = s16(vi[is_])
            taken = (v < 0, v > 0, v <= 0, v >= 0)[op7 - 0x2C]
            return target() if taken else None
        if op7 != 0x40:
            raise RuntimeError(f"VU lower 0x{w:08x} at 0x{pc:03x} is not interpreted")
        sub = w & 63
        id_ = (w >> 6) & 15
        if sub in (0x30, 0x31, 0x34, 0x35):
            x, y = vi[is_], vi[it]
            seti(id_, {0x30: x + y, 0x31: x - y, 0x34: x & y, 0x35: x | y}[sub])
            return None
        if sub == 0x32:  # iaddi
            seti(it, vi[is_] + _sext((w >> 6) & 31, 5))
            return None
        ext = (((w >> 6) & 31) << 2) | (w & 3)
        fsf, ftf = (w >> 21) & 3, (w >> 23) & 3
        if ext == 0x30:  # move
            self._setf(ft, dest, vf[fs])
        elif ext == 0x31:  # mr32
            self._setf(ft, dest, vf[fs][1:] + vf[fs][:1])
        elif ext == 0x34:  # lqi
            self._setf(ft, dest, self.mem[vi[is_] % size])
            seti(is_, vi[is_] + 1)
        elif ext == 0x35:  # sqi
            q = self.mem[vi[it] % size]
            for k in range(4):
                if dest & _LANES[k]:
                    q[k] = vf[fs][k]
            seti(it, vi[it] + 1)
        elif ext == 0x38:  # div
            self.q = e.f_div(vf[fs][fsf], vf[ft][ftf])
        elif ext == 0x39:  # sqrt
            self.q = e.f_sqrt(vf[ft][ftf])
        elif ext == 0x3A:  # rsqrt
            self.q = e.f_rsqrt(vf[fs][fsf], vf[ft][ftf])
        elif ext in (0x3B, 0x7B):  # waitq / waitp
            pass
        elif ext == 0x3C:  # mtir
            seti(it, vf[fs][fsf])
        elif ext == 0x3D:  # mfir
            self._setf(ft, dest, [s16(vi[is_]) & 0xFFFFFFFF] * 4)
        elif ext == 0x3E:  # ilwr
            q = self.mem[vi[is_] % size]
            k = next(k for k in range(4) if dest & _LANES[k])
            seti(it, q[k])
        elif ext == 0x3F:  # iswr
            q = self.mem[vi[is_] % size]
            for k in range(4):
                if dest & _LANES[k]:
                    q[k] = vi[it] & 0xFFFF
        elif ext == 0x64:  # mfp
            self._setf(ft, dest, [self.p] * 4)
        elif ext in (0x68, 0x69):  # xtop / xitop
            seti(it, self.top)
        elif ext == 0x6C:  # xgkick
            self.kicks.append(vi[is_])
        elif ext == 0x78:  # esqrt
            self.p = e.f_sqrt(vf[fs][fsf])
        elif ext == 0x79:  # ersqrt
            self.p = e.f_rsqrt(_ONE, vf[fs][fsf])
        elif ext == 0x7A:  # ercpr
            self.p = e.f_div(_ONE, vf[fs][fsf])
        else:
            raise RuntimeError(f"VU lower 0x{w:08x} at 0x{pc:03x} is not interpreted")
        return None


# ---------------------------------------------------------------------------
# checks

def flow(blob, start):
    """Follow every path from `start`: returns the E-bit pairs reached, the
    jr pairs reached (a hand-off to code chosen at run time) and the
    addresses where a path ran off the code or into an invalid pair."""
    seen, ends, hand, stuck = set(), set(), set(), set()
    work = [start]
    while work:
        a = work.pop()
        if a in seen:
            continue
        seen.add(a)
        p = blob.pairs.get(a)
        if p is None or not p.valid:
            stuck.add(a)
            continue
        if p.end:
            ends.add(a)
            continue
        k = p.lower.kind
        if k == "jr":
            hand.add(a)
        elif k == "jump":
            work.append(p.lower.target)
        else:  # branch or call: the target, and on through the delay slot
            if p.lower.target is not None:
                work.append(p.lower.target)
            work.append(a + 1)
    return ends, hand, stuck


def check(mc):
    """Counts that should hold if the decoder and the label map are right."""
    refs = {name for _, name, _ in mc.ee_refs()}
    res = {}
    for b in (mc.vu1, mc.vu0):
        pairs = [b.pairs[a] for a in sorted(b.pairs)]
        bad = [p for p in pairs if not p.valid]
        targets = [p.lower.target for p in pairs if p.lower.target is not None]
        outside = [t for t in targets if t not in b.pairs]
        labelled = [t for t in targets if t in b.at]
        ends = [p.addr for p in pairs if p.end]
        kicked = [a for a in ends if any(
            b.pairs.get(x) and b.pairs[x].lower.mnemonic == "xgkick" for x in (a - 2, a - 1, a))]
        local = set(b.labels) - b.globals
        # every local label should be reached by some branch or a DVP reloc
        reached = set(targets) | {b.labels[s] for _, s in b.relocs.values() if s in b.labels}
        unreached = sorted(n for n in local if b.labels[n] not in reached)
        # entry points neither the EE nor a branch names: dead programs
        dead = sorted(n for n in b.globals if n not in refs and b.labels[n] not in reached)
        stuck = {}
        for n in sorted(b.globals):
            e, h, st = flow(b, b.labels[n])
            if st or not (e or h):
                stuck[n] = sorted(st)
        res[b.name] = dict(pairs=len(pairs), invalid=len(bad), branches=len(targets),
                           outside=len(outside), to_label=len(labelled), ends=len(ends),
                           ends_after_xgkick=len(kicked), labels=len(b.labels),
                           entries=len(b.globals), unreached_local=unreached,
                           unreferenced_entries=dead, entries_not_ending=stuck,
                           invalid_at=[p.addr for p in bad][:10])
    return res


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    num = lambda s: int(s, 0)  # noqa: E731
    p = sub.add_parser("pair")
    p.add_argument("lower", type=num)
    p.add_argument("upper", type=num)
    p.add_argument("--pc", type=num, default=0)
    p = sub.add_parser("blobs")
    p.add_argument("elf")
    p = sub.add_parser("labels")
    p.add_argument("elf")
    p.add_argument("--check", action="store_true")
    p = sub.add_parser("dis")
    p.add_argument("elf")
    p.add_argument("what", nargs="?")
    p.add_argument("--vu0", action="store_true")
    p.add_argument("--end", type=num, help="micro-address to stop at")
    p = sub.add_parser("dump")
    p.add_argument("elf")
    p.add_argument("outdir")
    p = sub.add_parser("check")
    p.add_argument("elf")
    p = sub.add_parser("packet")
    p.add_argument("elf")
    p.add_argument("vertices", type=num)
    p.add_argument("mtype", type=num)
    args = parser.parse_args()

    if args.cmd == "pair":
        print(Pair(args.pc, 0, args.lower, args.upper).text())
        return 0
    if args.cmd == "packet":
        print("\n".join(packet_listing(model_packet(args.elf, args.vertices, args.mtype))))
        return 0
    mc = Microcode(args.elf)
    if args.cmd == "blobs":
        for b in (mc.vu1, mc.vu0):
            s, e = b.range
            print(f"VU{b.unit}: EE 0x{s:08x}-0x{e:08x}, {len(b.pairs)} pairs "
                  f"(micro 0x000-0x{max(b.pairs):03x} of 0x{b.size:03x}), "
                  f"{len(b.labels)} labels, {len(b.globals)} entry points")
            for va, v in b.vif:
                print(f"  {va:08x}  {v.text}")
    elif args.cmd == "labels":
        from image import Program
        prog = Program(args.elf)
        users = {}
        for va, name, _ in mc.ee_refs():
            f = prog.function_at(va)
            users.setdefault(name, []).append(f"{f.name if f else '?'}@0x{va:08x}")
        for b in (mc.vu1, mc.vu0):
            print(f"# VU{b.unit}")
            for n, a in sorted(b.labels.items(), key=lambda kv: (kv[1], kv[0])):
                kind = "entry" if n in b.globals else "local"
                used = "  <- " + ", ".join(users[n]) if n in users else ""
                print(f"  0x{a:03x}  byte 0x{a * 8:04x}  EE 0x{b.pairs[a].va:08x}  {kind:5}  {n}{used}")
        if args.check:
            refs = mc.ee_refs()
            bad = 0
            for va, name, addr in refs:
                b, a = mc.find(name)
                ok = addr == a * 8
                bad += not ok
                if not ok:
                    print(f"  MISMATCH 0x{va:08x} {name}: EE says 0x{addr:x}, label 0x{a * 8:x}")
            print(f"# EE references: {len(refs)} HI16 sites to "
                  f"{len({n for _, n, _ in refs})} labels, {bad} mismatches")
            n = sum(len(b.relocs) for b in (mc.vu1, mc.vu0))
            bad = 0
            for b in (mc.vu1, mc.vu0):
                for a, (rt, sym) in b.relocs.items():
                    if b.pairs[a].lower.imm != b.labels.get(sym):
                        bad += 1
                        print(f"  MISMATCH micro 0x{a:03x} {sym}")
            print(f"# relocations inside microcode: {n}, {bad} mismatches")
    elif args.cmd == "dis":
        blob = mc.vu0 if args.vu0 else mc.vu1
        start = end = None
        if args.what:
            b, start = mc.find(args.what)
            blob = b or blob
            if args.end is not None:
                end = args.end
            else:  # to the next entry point
                later = [s for _, s, _ in blob.programs() if s > start]
                end = min(later) if later else None
        print("\n".join(blob.listing(start, end)))
    elif args.cmd == "dump":
        os.makedirs(args.outdir, exist_ok=True)
        for b in (mc.vu1, mc.vu0):
            path = os.path.join(args.outdir, f"{b.name}.txt")
            with open(path, "w") as f:
                s, e = b.range
                f.write(f"# VU{b.unit} microcode from EE 0x{s:08x}-0x{e:08x}, "
                        f"{len(b.pairs)} pairs\n")
                f.write("# micro EE_va    upper [flags]                            lower\n")
                for name, s, e in b.programs():
                    f.write(f"\n# ---- {name}  0x{s:03x}-0x{e:03x} ({e - s} pairs)\n")
                    f.write("\n".join(b.listing(s, e)) + "\n")
                first = min(s for _, s, _ in b.programs()) if b.globals else max(b.pairs) + 1
                if min(b.pairs) < first:
                    f.write("\n# ---- before the first entry point\n")
                    f.write("\n".join(b.listing(min(b.pairs), first)) + "\n")
            print(f"{path}: {len(b.pairs)} pairs, {len(b.programs())} programs")
        path = os.path.join(args.outdir, "labels.txt")
        with open(path, "w") as f:
            for b in (mc.vu1, mc.vu0):
                for n, a in sorted(b.labels.items(), key=lambda kv: (kv[1], kv[0])):
                    kind = "entry" if n in b.globals else "local"
                    f.write(f"vu{b.unit} 0x{a:03x} 0x{a * 8:04x} 0x{b.pairs[a].va:08x} {kind} {n}\n")
        print(f"{path}")
    elif args.cmd == "check":
        for name, r in check(mc).items():
            print(name, r)
    return 0


if __name__ == "__main__":
    sys.exit(main())
