#!/usr/bin/env python3
"""DWARF version 1 debug info, as Metrowerks CodeWarrior for PS2 writes it.

    tools/dwarf1.py cus     ELF [--files | --tus] [--overlay X]  compile units
    tools/dwarf1.py fn      ELF NAME_OR_VA [--overlay X]     one function as C
    tools/dwarf1.py type    ELF NAME [--all]                 struct/class/union/enum
    tools/dwarf1.py globals ELF [--grep RE] [--overlay X]    globals and statics
    tools/dwarf1.py lines   ELF VA [--overlay X] [--table]   source line of an address
    tools/dwarf1.py header  ELF --cu SUBSTR [--overlay X]    source file as a C header
    tools/dwarf1.py stats   ELF                              counts, vendor attrs, dirs
    tools/dwarf1.py check   ELF                              tree/symbol/demangle checks
    tools/dwarf1.py dump    ELF OFFSET [--depth N]           raw DIEs at a .debug offset

NAME_OR_VA is a DWARF name, mangled name, `Class::method` or address.
Overlays are main, gcmn, demo, desktop, toppage (".prg" optional); overlay
code all sits at 0x00400800, so addresses there match once per overlay.
`header --cu` takes a translation unit (everything it declares) or a file
included into one, e.g. sysmem.cpp (its functions and what they use).

Also a library: `Dwarf(elf_or_path)` parses .debug into a tree of `Die`
(`.off .tag .attrs .children .parent .cu`) and .line into per-CU tables;
`type_of(die)` gives a type tree that `demangle.cdecl` renders.

How CodeWarrior uses DWARF 1 (all verified against SLUS_202.67):
- One TAG_compile_unit per *function* (own low/high_pc and .line table),
  preceded by one CU per translation unit with no pc range that holds the
  TU's types and globals; function CUs refer only into their own TU's
  declaration CU. A function CU is named after the file the function is
  in, so unity builds show up: system.cpp's TU holds sysmem.cpp,
  syspad.cpp, ... and headers with inline code (eventarea.h, vector).
- Each function has exactly one TAG_lexical_block (its body) holding every
  local; inner scopes are flattened. global_variable DIEs whose type is a
  subroutine_type are declarations of functions the TU refers to.
- Every struct/class/union is TAG_class_type (0x02); unions are only
  recognisable by all members sitting at offset 0. TAG_structure_type (0x13)
  appears only as member-less `@anonN` types of vtables. No typedef,
  pointer_type or union_type DIEs exist: typedefs are resolved away.
- Bitfield AT_bit_offset counts from the least significant bit of the
  storage unit (tGS_PMODE.EN1 is bit 0), not from the MSB as DWARF 1 says.
- TAG_enumeration_type is rare (10 DIEs, e.g. CC_RENDER_STATE_TYPE); its
  element values are AT_byte_size wide (1 here), not 4 bytes. A few game
  classes have holes no member covers (ccChunkIndex+0x4): some members go
  undescribed.
- AT_language is 4 (C++) even for .c files; 0x8000 for the assembler.
- `this` is an explicit first parameter named "this"; methods without it
  are static. Unnamed or unused parameters are left out entirely (the
  mangled name still has them; `Dwarf.params` realigns the two).
- Fundamental types: long is 8 bytes (EE ABI); 0x8208 unsigned long long;
  0xa510 is a 128-bit integer (u_long128).
- Location op 0x80 <u32> is an FPU register; 0x82 dereferences a saved
  register slot. Register 2 ($v0) is a placeholder meaning "in a scratch
  register": arguments that plainly live in $a0..$a3 are recorded as 2.
- Vendor attributes: 0x2008 mangled name; 0x2013 frame (CFA = sp + size);
  0x2043..0x20c3 where s0..s7, fp are saved; 0x20d3..0x2173 f20..f30;
  0x2303 refs to the globals (and address-taken functions) a function
  uses; 0x2296/0x22a8 overlay
  id/name on TAG 0x4080, whose AT_member (0x0142) list names its CUs.
"""

import argparse
import bisect
import collections
import os
import re
import struct
import sys

from demangle import cdecl, demangle, parse as parse_mangled
from elf import Elf

TAG = {0x0001: "array_type", 0x0002: "class_type", 0x0003: "entry_point",
       0x0004: "enumeration_type", 0x0005: "formal_parameter",
       0x0006: "global_subroutine", 0x0007: "global_variable", 0x000a: "label",
       0x000b: "lexical_block", 0x000c: "local_variable", 0x000d: "member",
       0x000f: "pointer_type", 0x0010: "reference_type", 0x0011: "compile_unit",
       0x0012: "string_type", 0x0013: "structure_type", 0x0014: "subroutine",
       0x0015: "subroutine_type", 0x0016: "typedef", 0x0017: "union_type",
       0x0018: "unspecified_parameters", 0x0019: "variant",
       0x001a: "common_block", 0x001b: "common_inclusion", 0x001c: "inheritance",
       0x001d: "inlined_subroutine", 0x001e: "module",
       0x001f: "ptr_to_member_type", 0x0020: "set_type",
       0x0021: "subrange_type", 0x0022: "with_stmt",
       0x4080: "MW_overlay"}

# Attribute names by the name part (code >> 4); the low 4 bits are the form.
AT = {0x001: "sibling", 0x002: "location", 0x003: "name", 0x005: "fund_type",
      0x006: "mod_fund_type", 0x007: "user_def_type", 0x008: "mod_u_d_type",
      0x009: "ordering", 0x00a: "subscr_data", 0x00b: "byte_size",
      0x00c: "bit_offset", 0x00d: "bit_size", 0x00f: "element_list",
      0x010: "stmt_list", 0x011: "low_pc", 0x012: "high_pc", 0x013: "language",
      0x014: "member", 0x015: "discr", 0x016: "discr_value",
      0x019: "string_length", 0x01a: "common_reference", 0x01b: "comp_dir",
      0x01c: "const_value", 0x01d: "containing_type", 0x01e: "default_value",
      0x020: "inline", 0x021: "is_optional", 0x022: "lower_bound",
      0x023: "program", 0x024: "private", 0x025: "producer",
      0x026: "protected", 0x027: "prototyped", 0x028: "public",
      0x029: "pure_virtual", 0x02a: "return_addr", 0x02b: "specification",
      0x02c: "start_scope", 0x02e: "stride_size", 0x02f: "upper_bound",
      0x030: "virtual",
      # Metrowerks vendor range (0x2000+ as full codes)
      0x200: "MW_mangled_name", 0x201: "MW_frame", 0x229: "MW_overlay_id",
      0x22a: "MW_overlay_name", 0x230: "MW_global_refs"}
GPR = ["zero", "at", "v0", "v1", "a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3",
       "t4", "t5", "t6", "t7", "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7",
       "t8", "t9", "k0", "k1", "gp", "sp", "fp", "ra"]
for _i in range(8):
    AT[0x204 + _i] = f"MW_saved_s{_i}"
AT[0x20c] = "MW_saved_fp"
for _i in range(11):
    AT[0x20d + _i] = f"MW_saved_f{20 + _i}"
FORM = {1: "addr", 2: "ref", 3: "block2", 4: "block4", 5: "data2", 6: "data4",
        7: "data8", 8: "string"}

(TAG_array_type, TAG_class_type, TAG_enumeration_type, TAG_formal_parameter,
 TAG_global_subroutine, TAG_global_variable, TAG_lexical_block,
 TAG_local_variable, TAG_member, TAG_pointer_type, TAG_reference_type,
 TAG_compile_unit, TAG_structure_type, TAG_subroutine, TAG_subroutine_type,
 TAG_typedef, TAG_union_type, TAG_unspecified_parameters, TAG_inheritance,
 TAG_ptr_to_member_type, TAG_MW_overlay) = (
    0x01, 0x02, 0x04, 0x05, 0x06, 0x07, 0x0b, 0x0c, 0x0d, 0x0f, 0x10, 0x11,
    0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1c, 0x1f, 0x4080)
AGGREGATES = (TAG_class_type, TAG_structure_type, TAG_union_type,
              TAG_enumeration_type)
FUNCTIONS = (TAG_global_subroutine, TAG_subroutine)

(AT_sibling, AT_location, AT_name, AT_fund_type, AT_mod_fund_type,
 AT_user_def_type, AT_mod_u_d_type, AT_subscr_data, AT_byte_size,
 AT_bit_offset, AT_bit_size, AT_stmt_list, AT_low_pc, AT_high_pc,
 AT_language, AT_member, AT_producer, AT_private, AT_protected, AT_public,
 AT_virtual, AT_return_addr) = (
    0x0012, 0x0023, 0x0038, 0x0055, 0x0063, 0x0072, 0x0083, 0x00a3, 0x00b6,
    0x00c5, 0x00d6, 0x0106, 0x0111, 0x0121, 0x0136, 0x0142, 0x0258, 0x0248,
    0x0268, 0x0288, 0x0308, 0x02a3)
AT_MW_mangled, AT_MW_frame, AT_MW_overlay_id, AT_MW_overlay_name, \
    AT_MW_global_refs = 0x2008, 0x2013, 0x2296, 0x22a8, 0x2303

# Fundamental types: name, size. Signed variants print as the plain C name.
FT = {0x0001: ("char", 1), 0x0002: ("signed char", 1), 0x0003: ("unsigned char", 1),
      0x0004: ("short", 2), 0x0005: ("short", 2), 0x0006: ("unsigned short", 2),
      0x0007: ("int", 4), 0x0008: ("int", 4), 0x0009: ("unsigned int", 4),
      0x000a: ("long", 8), 0x000b: ("long", 8), 0x000c: ("unsigned long", 8),
      0x000d: ("void *", 4), 0x000e: ("float", 4), 0x000f: ("double", 8),
      0x0010: ("long double", 8), 0x0011: ("complex", 8),
      0x0012: ("double complex", 16), 0x0014: ("void", None), 0x0015: ("bool", 1),
      0x0016: ("long double complex", 16), 0x0017: ("label", None),
      0x8008: ("long long", 8), 0x8108: ("long long", 8),
      0x8208: ("unsigned long long", 8), 0xa510: ("u_long128", 16)}
LANG = {0x1: "C89", 0x2: "C", 0x3: "Ada83", 0x4: "C++", 0x5: "Cobol74",
        0x6: "Cobol85", 0x7: "Fortran77", 0x8: "Fortran90", 0x9: "Pascal83",
        0xa: "Modula2", 0x8000: "asm"}
MOD = {1: "ptr", 2: "ref", 3: "const", 4: "volatile"}
OP = {1: "REG", 2: "BASEREG", 3: "ADDR", 4: "CONST", 5: "DEREF2", 6: "DEREF4",
      7: "ADD", 0x80: "MW_FREG", 0x82: "MW_DEREF"}
OP_ARG = {1, 2, 3, 4, 0x80}
VOID = ("base", "void", 0x14)
MAIN = "main"

_u16 = struct.Struct("<H").unpack_from
_u32 = struct.Struct("<I").unpack_from


def at_name(code):
    base = AT.get(code >> 4)
    return base if base else f"AT_0x{code:04x}"


class Die:
    __slots__ = ("off", "tag", "attrs", "children", "parent", "cu", "dups")

    def __init__(self, off, tag, attrs):
        self.off = off
        self.tag = tag
        self.attrs = attrs
        self.children = []
        self.parent = None
        self.cu = None
        self.dups = None   # code -> [every value], only for repeated attributes

    @property
    def name(self):
        return self.attrs.get(AT_name)

    def all(self, code):
        if self.dups and code in self.dups:
            return self.dups[code]
        return [self.attrs[code]] if code in self.attrs else []

    def __repr__(self):
        return f"<Die 0x{self.off:x} {TAG.get(self.tag, hex(self.tag))} {self.name!r}>"


class TU:
    """A translation unit: the file-level CU with the types and globals,
    then one CU per function. Function CUs carry the name of the file the
    function sits in, which for unity builds (system.cpp #includes
    sysmem.cpp, syspad.cpp, ...) differs from the TU's own name."""
    __slots__ = ("name", "overlay", "cus", "decl")

    def __init__(self, cu, overlay):
        self.name = cu.name
        self.overlay = overlay
        self.cus = [cu]
        self.decl = cu if AT_low_pc not in cu.attrs else None

    def files(self):
        return list(dict.fromkeys(cu.name for cu in self.cus[1 if self.decl else 0:]))

    def functions(self):
        return [d for cu in self.cus for d in cu.children if d.tag in FUNCTIONS]


def loc_ops(b):
    """Location expression bytes -> [(op, operand-or-None)]."""
    out, p = [], 0
    while p < len(b):
        op = b[p]
        p += 1
        if op in OP_ARG and p + 4 <= len(b):
            out.append((op, _u32(b, p)[0]))
            p += 4
        else:
            out.append((op, None))
    return out


def reg_name(r):
    if r == 2:
        return "scratch"   # MW's placeholder, see module docstring
    return "$" + GPR[r] if r < 32 else f"$r{r}"


def loc_str(b):
    """Short text for a location: `$s0`, `$f20`, `sp+0x10`, `@0x00377af0`."""
    ops = loc_ops(b)
    kinds = [o for o, _ in ops]
    if kinds[-1:] == [0x82] or kinds[-1:] == [6]:
        kinds, ops = kinds[:-1], ops[:-1]
    if kinds == [1]:
        return reg_name(ops[0][1])
    if kinds == [0x80]:
        return f"$f{ops[0][1]}"
    if kinds == [3]:
        return f"@0x{ops[0][1]:08x}"
    if kinds == [2]:
        return f"{GPR[ops[0][1]] if ops[0][1] < 32 else ops[0][1]}+0x0"
    if kinds == [2, 4, 7]:
        return f"{GPR[ops[0][1]] if ops[0][1] < 32 else ops[0][1]}+0x{ops[1][1]:x}"
    if kinds == [1, 4, 7]:
        return f"{reg_name(ops[0][1])}+0x{ops[1][1]:x}"
    if kinds == [4, 7]:
        return f"+0x{ops[0][1]:x}"
    return " ".join(OP.get(o, f"op{o:#x}") + ("" if a is None else f"({a:#x})")
                    for o, a in ops)


def member_offset(die):
    b = die.attrs.get(AT_location)
    if not b:
        return None
    ops = loc_ops(b)
    if [o for o, _ in ops] == [4, 7]:
        return ops[0][1]
    return None


def innermost(t):
    while t[0] in ("ptr", "ref", "cv", "array", "memptr"):
        t = t[-1]
    return t


class Dwarf:
    def __init__(self, elf):
        self.elf = elf if isinstance(elf, Elf) else Elf(elf)
        sec = {s.name: s for s in self.elf.sections}
        if ".debug" not in sec:
            raise ValueError(f"{self.elf.path}: no .debug section")
        d = sec[".debug"]
        self.buf = self.elf.data[d.offset:d.offset + d.size]
        ln = sec.get(".line")
        self.linebuf = self.elf.data[ln.offset:ln.offset + ln.size] if ln else b""
        self.dies = {}
        self.root = Die(-1, 0, {})
        self.nulls = 0
        self.problems = []   # tree violations found while parsing
        self._parse()
        self._index()

    # ---- parsing ----

    def _parse(self):
        buf, n, dies = self.buf, len(self.buf), self.dies
        null_at = set()
        stack = [(self.root, n)]
        off = 0
        while off < n:
            while len(stack) > 1 and off >= stack[-1][1]:
                parent, end = stack.pop()
                if off != end:
                    self.problems.append(f"children of 0x{parent.off:x} overrun its "
                                         f"sibling 0x{end:x} (reached 0x{off:x})")
            if n - off < 4:
                self.problems.append(f"{n - off} trailing bytes at 0x{off:x}")
                break
            length = _u32(buf, off)[0]
            if length < 8:   # null entry: ends a sibling chain, or padding
                null_at.add(off)
                self.nulls += 1
                off += max(length, 4)
                continue
            end = off + length
            if end > n:
                raise ValueError(f"DIE at 0x{off:x} runs past .debug")
            attrs = {}
            die = Die(off, _u16(buf, off + 4)[0], attrs)
            p = off + 6
            while p < end:
                code = _u16(buf, p)[0]
                p += 2
                form = code & 15
                if form in (1, 2, 6):
                    v = _u32(buf, p)[0]
                    p += 4
                elif form == 8:
                    q = buf.index(b"\0", p, end)
                    v = buf[p:q].decode("latin-1")
                    p = q + 1
                elif form == 3:
                    k = _u16(buf, p)[0]
                    v = buf[p + 2:p + 2 + k]
                    p += 2 + k
                elif form == 5:
                    v = _u16(buf, p)[0]
                    p += 2
                elif form == 4:
                    k = _u32(buf, p)[0]
                    v = buf[p + 4:p + 4 + k]
                    p += 4 + k
                elif form == 7:
                    v = struct.unpack_from("<Q", buf, p)[0]
                    p += 8
                else:
                    raise ValueError(f"DIE 0x{off:x}: attribute 0x{code:04x} has "
                                     f"unknown form {form}")
                if code in attrs:
                    if die.dups is None:
                        die.dups = {}
                    die.dups.setdefault(code, [attrs[code]]).append(v)
                else:
                    attrs[code] = v
            if p != end:
                self.problems.append(f"DIE 0x{off:x}: attributes end at 0x{p:x}, "
                                     f"length says 0x{end:x}")
            parent = stack[-1][0]
            die.parent = parent
            die.cu = die if parent is self.root else parent.cu
            parent.children.append(die)
            dies[off] = die
            sib = attrs.get(AT_sibling)
            if sib is not None:
                if sib > stack[-1][1]:
                    self.problems.append(f"DIE 0x{off:x}: sibling 0x{sib:x} is past "
                                         f"its parent's end 0x{stack[-1][1]:x}")
                if sib > end:
                    stack.append((die, sib))
            off = end
        # Every sibling pointer must name the next child; the last may point
        # at the null entry that closes the list.
        for parent in [self.root, *dies.values()]:
            kids = parent.children
            for a, b in zip(kids, kids[1:]):
                if a.attrs.get(AT_sibling) != b.off:
                    self.problems.append(f"DIE 0x{a.off:x}: sibling "
                                         f"0x{a.attrs.get(AT_sibling, 0):x} != next "
                                         f"child 0x{b.off:x}")
            if kids:
                last = kids[-1].attrs.get(AT_sibling)
                end = parent.attrs.get(AT_sibling, n) if parent is not self.root else n
                if last is not None and last != end and last not in null_at:
                    self.problems.append(f"DIE 0x{kids[-1].off:x}: last sibling "
                                         f"0x{last:x} is neither parent end nor null")

    def _index(self):
        self.cus = [d for d in self.root.children if d.tag == TAG_compile_unit]
        self.overlay_dies = [d for d in self.root.children if d.tag == TAG_MW_overlay]
        self.overlay_of_cu = {}
        self.overlay_names = [MAIN]
        for o in self.overlay_dies:
            name = o.attrs.get(AT_MW_overlay_name, f"overlay{o.attrs.get(AT_MW_overlay_id)}")
            self.overlay_names.append(name)
            for ref in o.all(AT_member):
                self.overlay_of_cu[ref] = name
        # Source files: every CU with the same name in the same overlay.
        self.units = collections.OrderedDict()
        for cu in self.cus:
            key = (cu.name, self.overlay(cu))
            self.units.setdefault(key, []).append(cu)
        # Translation units: a CU without a pc range opens one.
        self.tus, self.tu_of = [], {}
        cur = None
        for cu in self.cus:
            ov = self.overlay(cu)
            if cur is None or AT_low_pc not in cu.attrs or ov != cur.overlay:
                cur = TU(cu, ov)
                self.tus.append(cur)
            else:
                cur.cus.append(cu)
            self.tu_of[cu.off] = cur
        self.funcs = []
        for cu in self.cus:
            for d in cu.children:
                if d.tag in FUNCTIONS and AT_low_pc in d.attrs:
                    self.funcs.append(d)
        # Address index per overlay: sorted (low, high, die)
        self._by_addr = collections.defaultdict(list)
        for f in self.funcs:
            self._by_addr[self.overlay(f)].append(
                (f.attrs[AT_low_pc], f.attrs[AT_high_pc], f))
        for v in self._by_addr.values():
            v.sort(key=lambda x: (x[0], x[1]))
        self._cu_by_addr = collections.defaultdict(list)
        for cu in self.cus:
            if AT_low_pc in cu.attrs:
                self._cu_by_addr[self.overlay(cu)].append(
                    (cu.attrs[AT_low_pc], cu.attrs[AT_high_pc], cu))
        for v in self._cu_by_addr.values():
            v.sort(key=lambda x: (x[0], x[1]))
        self._lines = {}
        self._names = None
        self._methods = None
        self._elfsyms = None
        self._align = {}
        self._vt = None
        self._byname = None

    def overlay(self, die):
        return self.overlay_of_cu.get(die.cu.off, MAIN)

    # ---- names ----

    def mangled(self, die):
        return die.attrs.get(AT_MW_mangled)

    def qualname(self, die):
        """`Class::method` for C++ functions/statics, else the plain name."""
        m = die.attrs.get(AT_MW_mangled)
        if m:
            s = parse_mangled(m)
            if s:
                return s.qualname
        return die.name or f"<anon 0x{die.off:x}>"

    def names(self):
        """name / mangled / qualified name / demangled text -> [function DIEs]"""
        if self._names is None:
            self._names = collections.defaultdict(list)
            for f in self.funcs:
                keys = {f.name, self.qualname(f)}
                m = self.mangled(f)
                if m:
                    keys.update((m, demangle(m)))
                for k in keys:
                    if k:
                        self._names[k].append(f)
        return self._names

    def func_at(self, va, overlay=None):
        """Functions containing `va`, one per overlay unless `overlay`."""
        out = []
        for ov, lst in self._by_addr.items():
            if overlay and ov != overlay:
                continue
            i = bisect.bisect_right(lst, va, key=lambda x: x[0])
            for lo, hi, f in reversed(lst[max(0, i - 4):i]):
                if lo <= va < hi:
                    out.append(f)
        return out

    def cus_at(self, va, overlay=None):
        out = []
        for ov, lst in self._cu_by_addr.items():
            if overlay and ov != overlay:
                continue
            i = bisect.bisect_right(lst, va, key=lambda x: x[0])
            for lo, hi, cu in reversed(lst[max(0, i - 8):i]):
                if lo <= va < hi:
                    out.append(cu)
        return out

    def elf_symbols(self):
        """(overlay name, value) -> [Symbol], for FUNC/OBJECT symbols."""
        if self._elfsyms is None:
            self._elfsyms = collections.defaultdict(list)
            secs = self.elf.sections
            for s in self.elf.symbols:
                if s.type not in (1, 2) or not 0 < s.shndx < len(secs):
                    continue
                name = secs[s.shndx].name
                ov = MAIN if name == "main" else name
                self._elfsyms[(ov, s.value)].append(s)
        return self._elfsyms

    def elf_address(self, die, overlay):
        """ELF symbol values for a variable DWARF left at address 0.

        CodeWarrior writes 0 for most file statics (and unused externs) even
        when the ELF keeps a local symbol for them; a unique same-named
        symbol in the right section is very likely the one."""
        if self._byname is None:
            self._byname = collections.defaultdict(list)
            secs = self.elf.sections
            for s in self.elf.symbols:
                if s.type in (0, 1) and s.value and 0 < s.shndx < len(secs):
                    sec = secs[s.shndx].name
                    self._byname[s.name].append((MAIN if sec == "main" else sec, s.value))
        name = die.attrs.get(AT_MW_mangled) or die.name
        return sorted({v for ov, v in self._byname.get(name, []) if ov == overlay})

    # ---- types ----

    def type_of(self, die):
        """Type tree for a DIE's type attribute, or None if it has none."""
        a = die.attrs
        if AT_fund_type in a:
            return self.fund(a[AT_fund_type])
        if AT_user_def_type in a:
            return self.udt(a[AT_user_def_type])
        if AT_mod_fund_type in a:
            b = a[AT_mod_fund_type]
            return self.modified(b[:-2], self.fund(_u16(b, len(b) - 2)[0]))
        if AT_mod_u_d_type in a:
            b = a[AT_mod_u_d_type]
            return self.modified(b[:-4], self.udt(_u32(b, len(b) - 4)[0]))
        return None

    def fund(self, ft):
        name = FT.get(ft, (f"FT_0x{ft:04x}", None))[0]
        return ("base", name, ft)

    def modified(self, mods, t):
        # Modifiers are listed outermost first.
        for m in reversed(mods):
            k = MOD.get(m)
            if k in ("ptr", "ref"):
                t = (k, t)
            elif k:
                t = ("cv", k, t)
            else:
                t = ("cv", f"MOD_0x{m:x}", t)
        return t

    def type_name(self, d):
        n = d.name
        if not n or n.startswith("@"):
            kind = "union" if self.is_union(d) else "struct"
            if d.tag == TAG_enumeration_type:
                kind = "enum"
            return f"{kind} {n or '@anon'}" if n else kind
        return n

    def udt(self, ref):
        d = self.dies.get(ref)
        if d is None:
            return ("base", f"<bad ref 0x{ref:x}>", None)
        tag = d.tag
        if tag in AGGREGATES or tag == TAG_typedef:
            return ("base", self.type_name(d), d)
        if tag == TAG_array_type:
            return self.array_type(d)
        if tag == TAG_subroutine_type:
            params = [self.type_of(c) or VOID for c in d.children
                      if c.tag == TAG_formal_parameter]
            va = any(c.tag == TAG_unspecified_parameters for c in d.children)
            return ("func", self.type_of(d) or VOID, params, va)
        if tag == TAG_pointer_type:
            return ("ptr", self.type_of(d) or VOID)
        if tag == TAG_reference_type:
            return ("ref", self.type_of(d) or VOID)
        return ("base", f"<{TAG.get(tag, hex(tag))} 0x{ref:x}>", d)

    def array_type(self, d):
        b = d.attrs.get(AT_subscr_data, b"")
        p, dims, elem = 0, [], None
        while p < len(b):
            fmt = b[p]
            p += 1
            if fmt == 8:   # FMT_ET: element type as one attribute
                code = _u16(b, p)[0]
                v, p = _read_form(b, p + 2, code & 15)
                elem = self.type_of(Die(-1, 0, {code: v}))
                break
            p += 4 if fmt & 4 else 2   # index type: user-defined ref or FT
            bounds = []
            for expr in (fmt & 2, fmt & 1):
                if expr:
                    k = _u16(b, p)[0]
                    bounds.append(None)
                    p += 2 + k
                else:
                    bounds.append(_u32(b, p)[0])
                    p += 4
            dims.append(bounds)
        t = elem or ("base", "?", None)
        for lo, hi in reversed(dims):
            n = None if lo is None or hi is None or hi == 0xffffffff else hi - lo + 1
            t = ("array", n, t)
        return t

    def sizeof(self, t):
        k = t[0]
        if k == "base":
            ref = t[2] if len(t) > 2 else None
            if isinstance(ref, int):
                return FT.get(ref, (None, None))[1]
            if isinstance(ref, Die):
                return ref.attrs.get(AT_byte_size)
            return None
        if k in ("ptr", "ref", "memptr"):
            return 4
        if k == "cv":
            return self.sizeof(t[2])
        if k == "array":
            s = self.sizeof(t[2])
            return None if s is None or t[1] is None else s * t[1]
        return None

    def is_union(self, d):
        mems = [c for c in d.children if c.tag == TAG_member]
        return (d.tag == TAG_union_type or
                (d.tag == TAG_class_type and len(mems) > 1 and
                 all(member_offset(m) == 0 and AT_bit_size not in m.attrs
                     for m in mems)))

    def align_of(self, t):
        k = t[0]
        if k in ("ptr", "ref", "memptr", "func"):
            return 4
        if k in ("cv", "array"):
            return self.align_of(t[-1])
        ref = t[2] if len(t) > 2 else None
        if isinstance(ref, int):
            return min(FT.get(ref, (None, 1))[1] or 1, 16)
        if isinstance(ref, Die):
            if ref.off not in self._align:
                self._align[ref.off] = 1   # guards against recursion
                al = 1
                for c in ref.children:
                    if c.tag in (TAG_member, TAG_inheritance):
                        al = max(al, self.align_of(self.type_of(c) or VOID))
                self._align[ref.off] = al
            return self._align[ref.off]
        return 1

    def has_vtable(self, cls):
        if self._vt is None:
            self._vt = set()
            for sym in self.elf.symbols:
                if sym.name.startswith("__vt__"):
                    s = parse_mangled(sym.name)
                    if s and s.classname:
                        self._vt.add(s.classname)
        return cls in self._vt or any(c.endswith("::" + cls) for c in self._vt)

    def aggregates(self, name):
        """Every class/struct/union/enum DIE called `name`. DWARF names
        nested classes bare, so `Outer::Inner` looks up `Inner`."""
        name = name.rsplit("::", 1)[-1]
        return [d for d in self.dies.values()
                if d.tag in AGGREGATES and d.name == name]

    def members_of(self, name):
        """class_members() entry for `name`, matching a bare nested name to
        its qualified form when that is unambiguous."""
        cm = self.class_members()
        if name in cm:
            return name, cm[name]
        hits = [k for k in cm if k.rsplit("::", 1)[-1] == name]
        return (hits[0], cm[hits[0]]) if len(hits) == 1 else (name, None)

    # ---- methods and statics attached to classes (from mangled names) ----

    def class_members(self):
        """class name -> {"methods": [func DIE], "statics": [var DIE]}"""
        if self._methods is None:
            self._methods = collections.defaultdict(lambda: {"methods": [], "statics": []})
            seen = set()
            for d in self.dies.values():
                if d.tag not in (*FUNCTIONS, TAG_global_variable):
                    continue
                m = d.attrs.get(AT_MW_mangled)
                s = parse_mangled(m) if m else None
                if not s or not s.classname:
                    continue
                if d.tag == TAG_global_variable:
                    t = self.type_of(d)
                    if t and t[0] == "func":
                        continue   # a reference to a method, not a variable
                    key = (m, "s")
                    if key in seen:
                        continue
                    seen.add(key)
                    self._methods[s.classname]["statics"].append(d)
                elif AT_low_pc in d.attrs:
                    self._methods[s.classname]["methods"].append(d)
        return self._methods

    # ---- line tables ----

    def line_table(self, cu):
        """[(address, line)] for a CU, ending with a line-0 end marker."""
        off = cu.attrs.get(AT_stmt_list)
        if off is None or not self.linebuf:
            return []
        if off not in self._lines:
            length, base = struct.unpack_from("<II", self.linebuf, off)
            rows = []
            for p in range(off + 8, off + length, 10):
                line, _pos, delta = struct.unpack_from("<IHI", self.linebuf, p)
                rows.append((base + delta, line))
            self._lines[off] = rows
        return self._lines[off]

    def line_for(self, cu, va):
        best = None
        for addr, line in self.line_table(cu):
            if addr > va:
                break
            if line:
                best = (addr, line)
        return best

    def line_range(self, f):
        lo, hi = f.attrs.get(AT_low_pc), f.attrs.get(AT_high_pc)
        lines = [ln for a, ln in self.line_table(f.cu) if ln and lo <= a < hi]
        return (min(lines), max(lines)) if lines else None

    # ---- rendering ----

    def decl(self, die, name=None):
        t = self.type_of(die)
        return cdecl(t if t else VOID, die.name if name is None else name)

    def params(self, f):
        """[(DIE or None, type)] for a function's parameters, `this` excluded.

        CodeWarrior drops unnamed/unused parameters from DWARF and sometimes
        the const on a pointed-to type; the mangled name has both. When the
        DWARF parameters align into the mangled list as a subsequence, the
        mangled types are used, and missing ones come back with DIE None."""
        dps = [c for c in f.children if c.tag == TAG_formal_parameter
               and c.name != "this"]
        out = [(p, self.type_of(p) or VOID) for p in dps]
        m = self.mangled(f)
        sym = parse_mangled(m) if m else None
        if not sym or not sym.is_func or len(sym.params) < len(dps):
            return out
        slots, j = [None] * len(sym.params), 0
        for p, t in out:
            want = cdecl(t)
            while j < len(sym.params) and not compatible(want, cdecl(sym.params[j])):
                j += 1
            if j == len(sym.params):
                return out   # does not align: trust DWARF alone
            slots[j] = p
            j += 1
        return list(zip(slots, sym.params))

    def has_this(self, f):
        return any(c.tag == TAG_formal_parameter and c.name == "this" for c in f.children)

    def lacks_this(self, f):
        """A method with no `this` in DWARF: static, or `this` unused (empty
        inline virtuals such as EVENTAREA::Draw lose it like any unused
        parameter), so this is only a hint."""
        m = self.mangled(f)
        sym = parse_mangled(m) if m else None
        return bool(sym and sym.is_func and sym.classname and not self.has_this(f)
                    and sym.name not in ("operator new", "operator delete",
                                         "operator new[]", "operator delete[]"))

    def prototype(self, f, locs=False, qualified=True, show_this=False):
        ret = self.type_of(f) or VOID
        parts = []
        if show_this:
            for c in f.children:
                if c.tag == TAG_formal_parameter and c.name == "this":
                    s = cdecl(self.type_of(c) or VOID, "this")
                    if locs and AT_location in c.attrs:
                        s += f" /* {loc_str(c.attrs[AT_location])} */"
                    parts.append(s)
        for p, t in self.params(f):
            if p is None:
                parts.append(cdecl(t) + " /* not in DWARF */")
                continue
            s = cdecl(t, p.name or "")
            if locs and AT_location in p.attrs:
                s += f" /* {loc_str(p.attrs[AT_location])} */"
            parts.append(s)
        if any(c.tag == TAG_unspecified_parameters for c in f.children):
            parts.append("...")
        m = self.mangled(f)
        sym = parse_mangled(m) if m else None
        if qualified:
            name = self.qualname(f)
        else:
            name = sym.name if sym else (f.name or "")
        tail = " const" if sym and sym.is_const else ""
        text = cdecl(ret, f"{name}({', '.join(parts) or 'void'}){tail}")
        if f.tag == TAG_subroutine:
            text = "static " + text
        elif self.lacks_this(f):
            text = "/*static?*/ " + text
        return text

    def frame_info(self, f):
        a = f.attrs
        parts = []
        fr = a.get(AT_MW_frame)
        if fr:
            ops = loc_ops(fr)
            if [o for o, _ in ops] == [1, 4, 7]:
                parts.append(f"frame 0x{ops[1][1]:x}")
        saves = []
        for code in sorted(a):
            if 0x2040 <= code < 0x2180 and code & 15 == 3:
                saves.append(f"{at_name(code)[9:]}@{loc_str(a[code])}")
        if AT_return_addr in a:
            saves.append(f"ra@{loc_str(a[AT_return_addr])}")
        if saves:
            parts.append("saves " + " ".join(saves))
        return ", ".join(parts)

    def render_function(self, f):
        a = f.attrs
        lo, hi = a[AT_low_pc], a[AT_high_pc]
        out = []
        m = self.mangled(f)
        if m:
            out.append(f"// {m}  ->  {demangle(m)}")
        rng = self.line_range(f)
        src = f"{f.cu.name}:{rng[0]}-{rng[1]}" if rng else f.cu.name
        out.append(f"// {src}")
        out.append(f"// {self.overlay(f)} 0x{lo:08x}-0x{hi:08x} (0x{hi - lo:x} bytes), "
                   f"DIE 0x{f.off:x} in CU 0x{f.cu.off:x}")
        fi = self.frame_info(f)
        if fi:
            out.append(f"// {fi}")
        refs = a.get(AT_MW_global_refs)
        if refs:
            names = []
            for i in range(0, len(refs) - 3, 4):
                g = self.dies.get(_u32(refs, i)[0])
                names.append(self.qualname(g) if g else f"?0x{_u32(refs, i)[0]:x}")
            out.append(f"// touches: {', '.join(names)}")
        if self.lacks_this(f):
            out.append("// no `this` in DWARF: a static member, or `this` is unused")
        out.append(self.prototype(f, locs=True, show_this=True))
        out.append("{")
        body = [c for c in f.children if c.tag == TAG_lexical_block]
        for c in f.children:
            if c.tag == TAG_local_variable:
                out.append("    " + self._local(c))
        for b in body:
            # The outermost block spans the whole function: flatten it.
            same = (b.attrs.get(AT_low_pc), b.attrs.get(AT_high_pc)) == (lo, hi)
            out.extend(self._block(b, 1, flatten=same))
        out.append("}")
        return "\n".join(out)

    def _local(self, v):
        s = self.decl(v) + ";"
        loc = v.attrs.get(AT_location)
        if loc:
            ls = loc_str(loc)
            if ls.startswith("@"):
                s = "static " + s
            s += f"  /* {ls} */"
        return s

    def _block(self, b, depth, flatten=False):
        ind = "    " * depth
        out = []
        if not flatten:
            out.append(f"{ind}{{   // 0x{b.attrs.get(AT_low_pc, 0):08x}-"
                       f"0x{b.attrs.get(AT_high_pc, 0):08x}")
            depth += 1
        inner = "    " * depth
        for c in b.children:
            if c.tag == TAG_local_variable:
                out.append(inner + self._local(c))
        for c in b.children:
            if c.tag == TAG_lexical_block:
                out.extend(self._block(c, depth))
            elif c.tag not in (TAG_local_variable,):
                out.append(f"{inner}// {TAG.get(c.tag, hex(c.tag))} {c.name or ''}")
        if not flatten:
            out.append(f"{ind}}}")
        return out

    def render_aggregate(self, d, indent="", methods=True, name=None):
        """C declaration of a class/struct/union/enum DIE, as a list of lines."""
        if d.tag == TAG_enumeration_type:
            return self._render_enum(d, indent)
        union = self.is_union(d)
        kw = "union" if union else "struct"
        size = d.attrs.get(AT_byte_size, 0)
        nm = d.name if name is None else name
        anon = not nm or nm.startswith("@")
        bases = []
        for c in d.children:
            if c.tag == TAG_inheritance:
                acc = "private" if AT_private in c.attrs else \
                    "protected" if AT_protected in c.attrs else "public"
                virt = " virtual" if AT_virtual in c.attrs else ""
                bt = self.type_of(c)
                off = member_offset(c)
                bases.append((f"{acc}{virt} {cdecl(bt)}", off, bt))
        head = f"{kw} {nm}" if not anon else kw
        if bases:
            head += " : " + ", ".join(b[0] for b in bases)
        lines = [f"{indent}{head} {{   // size 0x{size:x}"]
        ind = indent + "    "
        cursor = 0
        # The vtable pointer lives in the first polymorphic class of a chain.
        vt = not anon and self.has_vtable(nm) and not any(
            c.tag == TAG_inheritance for c in d.children)
        for text, off, bt in bases:
            bs = self.sizeof(bt) or 0
            lines.append(f"{ind}/* 0x{off or 0:03x} */ // base {cdecl(bt)} (0x{bs:x} bytes)")
            cursor = max(cursor, (off or 0) + bs)
        for c in d.children:
            if c.tag != TAG_member:
                continue
            off = member_offset(c)
            t = self.type_of(c) or VOID
            sz = self.sizeof(t)
            if off is not None and off > cursor and not union:
                lines.append(self._gap(ind, cursor, off, self.align_of(t), vt))
            core = innermost(t)
            cd = core[2] if len(core) > 2 and isinstance(core[2], Die) else None
            if cd is not None and cd.tag in AGGREGATES and \
                    (not cd.name or cd.name.startswith("@")):
                # Anonymous aggregate: spell it out inline.
                sub = self.render_aggregate(cd, ind, methods=False)
                decl = cdecl(("base", "@@", None) if core is t else
                             _replace_core(t, ("base", "@@", None)), c.name)
                tail = decl.replace("@@", "", 1).strip()
                sub[-1] = sub[-1].replace("};", f"}} {tail};")
                sub[0] = f"{ind}/* 0x{off or 0:03x} */ " + sub[0].lstrip()
                lines.extend(sub)
            else:
                decl = cdecl(t, c.name or "")
                if AT_bit_size in c.attrs:
                    bo, bs = c.attrs.get(AT_bit_offset, 0), c.attrs[AT_bit_size]
                    decl += f" : {bs}"
                    note = f"bits {bo}..{bo + bs - 1}"
                else:
                    note = f"0x{sz:x}" if sz is not None else "?"
                lines.append(f"{ind}/* 0x{off if off is not None else 0:03x} */ "
                             f"{decl};".ljust(len(ind) + 48) + f"  // {note}")
            if off is not None and sz is not None:
                cursor = max(cursor, off + sz)
        if not bases and not any(c.tag == TAG_member for c in d.children):
            lines.append(f"{ind}// no data members")
        elif not union and size > cursor:
            lines.append(self._gap(ind, cursor, size, self.align_of(("base", "", d)), vt))
        if methods and not anon:
            lines.extend(self._render_class_extras(nm, ind))
        lines.append(f"{indent}}};")
        return lines

    @staticmethod
    def _gap(ind, start, end, align, vtable):
        """Describe bytes no member covers. DWARF has no alignment attributes
        (sceVu0FVECTOR and friends are 16-byte aligned typedefs), so a gap that
        rounding up to 16 explains is only probably padding."""
        up = lambda n: (start + n - 1) // n * n
        if up(align) == end:
            why = "padding"
        elif vtable and end - start >= 4:
            why = "not in DWARF: the vtable pointer, plus any padding"
        elif up(16) == end:
            why = "padding if 16-byte aligned, else a member DWARF omits"
        else:
            why = "not in DWARF (a member CodeWarrior did not describe)"
        return f"{ind}/* 0x{start:03x} */ // 0x{end - start:x} bytes {why}"

    def _render_class_extras(self, name, ind):
        out = []
        name, cm = self.members_of(name)
        if cm and cm["statics"]:
            out.append(f"{ind}// static members")
            for v in cm["statics"]:
                loc = v.attrs.get(AT_location)
                ls = loc_str(loc) if loc else "?"
                ls = "not allocated" if ls == "@0x00000000" else ls
                out.append(f"{ind}static {self.decl(v)};  /* {ls} */")
        if cm and cm["methods"]:
            out.append(f"{ind}// methods (DWARF functions whose mangled name names this class)")
            by = collections.OrderedDict()
            for f in cm["methods"]:
                by.setdefault(self.mangled(f), []).append(f)
            last = name.rsplit("::", 1)[-1]
            for fs in by.values():
                where = ", ".join(f"{self.overlay(f)}:0x{f.attrs[AT_low_pc]:08x}" for f in fs)
                proto = self.prototype(fs[0], qualified=False)
                sym = parse_mangled(self.mangled(fs[0]))
                if sym and sym.name in (last, "~" + last):
                    # CodeWarrior's ctors/dtors return `this`; C++ spells no type
                    ret = cdecl(self.type_of(fs[0]) or VOID)
                    proto = proto[proto.index(sym.name):] + f" /* returns {ret} */"
                out.append(f"{ind}{proto};  // {where}")
        vt = self.vtable(name)
        if vt:
            out.append(f"{ind}// {vt[0]}")
            for line in vt[1:]:
                out.append(f"{ind}//   {line}")
        return out

    def vtable(self, cls):
        """Read `__vt__<cls>` from the ELF image: [header, slot lines...]."""
        mangled = "".join(f"{len(p)}{p}" for p in cls.split("::"))
        if "::" in cls:
            mangled = f"Q{cls.count('::') + 1}{mangled}"
        want = "__vt__" + mangled
        syms = [s for s in self.elf.symbols if s.name == want]
        if not syms:
            return None
        s = syms[0]
        sec = self.elf.sections[s.shndx].name if 0 < s.shndx < len(self.elf.sections) else "?"
        head = f"vtable {want} @0x{s.value:08x} ({sec}, {s.size} bytes)"
        if sec != "main":
            return [head + " - overlay data is not in the ELF image"]
        out = [head]
        idx = self.elf_symbols()
        # Overlay slots resolve in the overlay that holds the class's code.
        _, cm = self.members_of(cls)
        home = collections.Counter(self.overlay(f) for f in (cm or {}).get("methods", []))
        home = [ov for ov, _ in home.most_common()] + self.overlay_names[1:]
        for i in range(0, s.size, 4):
            w = self.elf.u32(s.value + i)
            if i == 0:
                out.append(f"+0x00 RTTI 0x{w:08x}")
                continue
            if i == 4:
                out.append(f"+0x04 this-adjust 0x{w:x}")
                continue
            if w < 0x400800:
                cands, where = idx.get((MAIN, w), []), ""
            else:
                ov = next((ov for ov in home if (ov, w) in idx), None)
                cands, where = idx.get((ov, w), []), f" [{ov}]" if ov else ""
            label = " | ".join(sorted({demangle(x.name) for x in cands if x.type == 2})) or "?"
            out.append(f"+0x{i:02x} 0x{w:08x} {label}{where}")
        return out

    def _render_enum(self, d, indent):
        b = d.attrs.get(0x00f4) or d.attrs.get(0x00f3) or b""
        width = d.attrs.get(AT_byte_size, 4)
        items, p = [], 0
        while p + width <= len(b):
            v = int.from_bytes(b[p:p + width], "little", signed=True)
            q = b.index(b"\0", p + width)
            items.append((b[p + width:q].decode("latin-1"), v))
            p = q + 1
        lines = [f"{indent}enum {d.name or ''} {{   // size 0x{width:x}"]
        for n, v in items:
            lines.append(f"{indent}    {n} = {v},")
        lines.append(f"{indent}}};")
        return lines

    # ---- globals ----

    def globals(self, overlay=None):
        """Unique (name, address) globals and file statics, with where seen.
        Returns {key: {"die": first DIE, "units": [unit names], ...}}."""
        out = collections.OrderedDict()
        for cu in self.cus:
            ov = self.overlay(cu)
            for d in cu.children:
                if d.tag not in (TAG_global_variable, TAG_local_variable):
                    continue
                t = self.type_of(d)
                if t and t[0] == "func":
                    continue   # declaration of a function, not a variable
                loc = d.attrs.get(AT_location)
                ops = loc_ops(loc) if loc else []
                addr = ops[0][1] if ops and ops[0][0] == 3 else None
                home = MAIN if addr is not None and 0 < addr < 0x400800 else ov
                if overlay and home != overlay:
                    continue
                key = (self.qualname(d), addr, home,
                       d.tag == TAG_local_variable and self.tu_of[cu.off].name)
                g = out.get(key)
                if g is None:
                    out[key] = g = {"die": d, "units": [], "type": t, "addr": addr,
                                    "home": home, "static": d.tag == TAG_local_variable}
                tu = self.tu_of[cu.off].name
                if tu not in g["units"]:
                    g["units"].append(tu)
        # A TU that declares a global it never uses gets address 0; fold those
        # into the real definition when some other TU has it.
        placed = collections.defaultdict(list)
        for key, g in out.items():
            if g["addr"] and not g["static"]:
                placed[key[0]].append(g)
        for key in [k for k, g in out.items() if g["addr"] == 0 and not g["static"]]:
            if placed.get(key[0]):
                g = placed[key[0]][0]
                g["units"].extend(u for u in out.pop(key)["units"] if u not in g["units"])
        return out


def _replace_core(t, core):
    if t[0] in ("ptr", "ref"):
        return (t[0], _replace_core(t[1], core))
    if t[0] == "cv":
        return ("cv", t[1], _replace_core(t[2], core))
    if t[0] == "array":
        return ("array", t[1], _replace_core(t[2], core))
    if t[0] == "memptr":
        return ("memptr", t[1], _replace_core(t[2], core))
    return core


def _read_form(b, p, form):
    if form in (1, 2, 6):
        return _u32(b, p)[0], p + 4
    if form == 5:
        return _u16(b, p)[0], p + 2
    if form == 7:
        return struct.unpack_from("<Q", b, p)[0], p + 8
    if form == 3:
        k = _u16(b, p)[0]
        return b[p + 2:p + 2 + k], p + 2 + k
    if form == 4:
        k = _u32(b, p)[0]
        return b[p + 4:p + 4 + k], p + 4 + k
    if form == 8:
        q = b.index(b"\0", p)
        return b[p:q].decode("latin-1"), q + 1
    raise ValueError(f"unknown form {form}")


def same_type(a, b):
    """Rendered types equal, allowing DWARF's bare nested-class names."""
    return a == b or re.sub(r"\b\w+::", "", b) == a or re.sub(r"\b\w+::", "", a) == b


def _loose(t):
    t = re.sub(r"\b(const|volatile) ", "", t)
    t = re.sub(r" (const|volatile)\b", "", t)
    return re.sub(r"\b\w+::", "", t.replace("bool", "unsigned char"))


def compatible(dwarf_t, mangled_t):
    """Same parameter type, allowing for what CodeWarrior's DWARF loses:
    nested-class qualification, const/volatile, and bool (unsigned char)."""
    return same_type(dwarf_t, mangled_t) or _loose(dwarf_t) == _loose(mangled_t)


def basename(path):
    return re.split(r"[\\/]", path)[-1] if path else "?"


def dirname(path):
    parts = re.split(r"[\\/]", path or "")
    return "\\".join(parts[:-1]) if len(parts) > 1 else "."


# ---- commands ----

def cmd_cus(dw, args):
    if args.tus:
        for tu in dw.tus:
            if args.overlay and tu.overlay != args.overlay:
                continue
            fns = tu.functions()
            pcs = [(cu.attrs[AT_low_pc], cu.attrs[AT_high_pc]) for cu in tu.cus
                   if AT_low_pc in cu.attrs]
            rng = (f"0x{min(p[0] for p in pcs):08x}-0x{max(p[1] for p in pcs):08x}"
                   if pcs else "-" * 21)
            files = [basename(f) for f in tu.files() if f != tu.name]
            inc = f"  + {', '.join(files)}" if files else ""
            print(f"0x{tu.cus[0].off:08x} {tu.overlay:12} {rng} {len(tu.cus):4} CUs "
                  f"{len(fns):4} fns  {tu.name}{inc}")
        print(f"# {len(dw.tus)} translation units")
        return
    if args.files:
        n = 0
        for (name, ov), cus in dw.units.items():
            if args.overlay and ov != args.overlay:
                continue
            n += 1
            nf = sum(1 for cu in cus for d in cu.children if d.tag in FUNCTIONS)
            pcs = [(cu.attrs[AT_low_pc], cu.attrs[AT_high_pc]) for cu in cus
                   if AT_low_pc in cu.attrs]
            rng = (f"0x{min(p[0] for p in pcs):08x}-0x{max(p[1] for p in pcs):08x}"
                   if pcs else "-" * 21)
            tus = dict.fromkeys(basename(dw.tu_of[cu.off].name) for cu in cus)
            tu = "" if list(tus) == [basename(name)] else f"  (in {', '.join(tus)})"
            print(f"{ov:12} {len(cus):4} CUs {nf:4} fns  {rng}  {name}{tu}")
        print(f"# {n} source files ({len({k[0] for k in dw.units})} distinct paths)")
        return
    counts = collections.Counter()
    for cu in dw.cus:
        ov = dw.overlay(cu)
        if args.overlay and ov != args.overlay:
            continue
        counts[ov] += 1
        a = cu.attrs
        nf = sum(1 for d in cu.children if d.tag in FUNCTIONS)
        rng = (f"0x{a[AT_low_pc]:08x}-0x{a[AT_high_pc]:08x}" if AT_low_pc in a
               else "-" * 21)
        lang = LANG.get(a.get(AT_language), hex(a.get(AT_language, 0)))
        print(f"0x{cu.off:08x} {ov:12} {rng} {nf:3} fn {lang:4} {cu.name}  "
              f"[{a.get(AT_producer, '')}]")
    print(f"# {sum(counts.values())} compile units: " +
          ", ".join(f"{k} {v}" for k, v in counts.items()))


def find_functions(dw, what, overlay=None):
    try:
        va = int(what, 0)
    except ValueError:
        va = None
    if va is not None:
        return dw.func_at(va, overlay)
    hits = dw.names().get(what, [])
    if not hits:
        # An ELF symbol name that DWARF spells differently
        for s in dw.elf.symbols:
            if s.name == what and s.type == 2:
                sec = dw.elf.sections[s.shndx].name
                hits += dw.func_at(s.value, MAIN if sec == "main" else sec)
    return [f for f in hits if not overlay or dw.overlay(f) == overlay]


def cmd_fn(dw, args):
    fs = find_functions(dw, args.what, args.overlay)
    if not fs:
        print(f"no function {args.what!r}", file=sys.stderr)
        return 1
    seen = set()
    for f in fs:
        if f.off in seen:
            continue
        seen.add(f.off)
        print(dw.render_function(f))
        print()
    return 0


def _layout_sig(dw, d):
    return (d.attrs.get(AT_byte_size), d.attrs.get(0x00f4) or d.attrs.get(0x00f3),
            tuple((c.tag, c.name, member_offset(c), c.attrs.get(AT_bit_offset),
                   c.attrs.get(AT_bit_size), cdecl(dw.type_of(c) or VOID))
                  for c in d.children if c.tag in (TAG_member, TAG_inheritance)))


def _nested_in(dw, ds, outer):
    """Of candidate DIEs for a bare nested-class name, those that a class
    called `outer` in the same TU uses as a member type (DWARF has no nesting)."""
    keep = []
    for d in ds:
        for o in d.cu.children:
            if o.tag in AGGREGATES and o.name == outer and any(
                    b is d for c in o.children if c.tag == TAG_member
                    for b in _bases(dw.type_of(c) or VOID, [])):
                keep.append(d)
                break
    return keep


def cmd_type(dw, args):
    ds = dw.aggregates(args.name)
    if "::" in args.name and len(ds) > 1:
        outer = args.name.rsplit("::", 2)[-2]
        ds = _nested_in(dw, ds, outer) or ds
    if not ds:
        like = sorted({d.name for d in dw.dies.values() if d.tag in AGGREGATES and d.name
                       and args.name.lower() in d.name.lower()})
        print(f"no type {args.name!r}" + (f"; similar: {', '.join(like[:40])}" if like else ""),
              file=sys.stderr)
        return 1
    groups = collections.OrderedDict()
    for d in ds:
        groups.setdefault(_layout_sig(dw, d), []).append(d)
    units = {dw.tu_of[d.cu.off].name for d in ds}
    print(f"// {args.name}: {len(ds)} definitions in {len(units)} translation units, "
          f"{len(groups)} distinct layout{'s' * (len(groups) != 1)}")
    for i, (sig, members) in enumerate(groups.items()):
        if i and not args.all:
            print(f"// ... {len(groups) - 1} other layouts, use --all")
            break
        d = members[0]
        files = sorted({basename(dw.tu_of[m.cu.off].name) for m in members})
        print(f"// layout {i + 1}: DIE 0x{d.off:x}, {len(members)} copies, in "
              f"{', '.join(files[:8])}{' ...' if len(files) > 8 else ''}")
        qual = args.name if "::" in args.name else None
        print("\n".join(dw.render_aggregate(d, methods=(i == 0), name=qual)))
        print()
    return 0


def cmd_globals(dw, args):
    pat = re.compile(args.grep) if args.grep else None
    rows = []
    for (qn, addr, home, _), g in dw.globals(args.overlay).items():
        if pat and not pat.search(qn):
            continue
        rows.append((addr if addr is not None else -1, qn, g, home))
    rows.sort(key=lambda r: (r[0], r[1]))
    for addr, qn, g, home in rows:
        t = g["type"] or VOID
        size = dw.sizeof(t)
        where = basename(g["units"][0]) + (f" (+{len(g['units']) - 1})" if len(g["units"]) > 1 else "")
        a = f"0x{addr:08x}" if addr and addr > 0 else "?         "
        if addr == 0:
            cands = dw.elf_address(g["die"], home)
            a = f"0x{cands[0]:08x}" if len(cands) == 1 else "unalloc   "
            where += (" [address from ELF symbol; DWARF says 0]" if len(cands) == 1 else
                      f" [DWARF says 0; {len(cands)} ELF symbols share the name]"
                      if cands else " [DWARF says 0, no ELF symbol]")
        st = "static " if g["static"] else ""
        sz = f"0x{size:x}" if size is not None else "?"
        print(f"{a} {sz:>7} {home:12} {st}{cdecl(t, qn)};  // {where}")
    print(f"# {len(rows)} globals")


def cmd_lines(dw, args):
    cus = dw.cus_at(args.va, args.overlay)
    if not cus:
        print(f"0x{args.va:08x}: no compile unit covers this address", file=sys.stderr)
        return 1
    for cu in cus:
        hit = dw.line_for(cu, args.va)
        fs = [f for f in dw.func_at(args.va, dw.overlay(cu)) if f.cu is cu]
        fn = ""
        if fs:
            f = fs[0]
            fn = f"  in {demangle(dw.mangled(f)) if dw.mangled(f) else f.name} " \
                 f"(0x{f.attrs[AT_low_pc]:08x}+0x{args.va - f.attrs[AT_low_pc]:x})"
        where = f"{cu.name}:{hit[1]}" if hit else f"{cu.name}:?"
        print(f"0x{args.va:08x} {dw.overlay(cu):12} {where}{fn}")
        if args.table:
            for a, line in dw.line_table(cu):
                mark = " <" if hit and a == hit[0] else ""
                print(f"    0x{a:08x} {line}{mark}")
    return 0


def _type_deps(dw, d, by_value):
    """Aggregate DIEs that `d` needs complete (members/bases by value)."""
    out = []
    for c in d.children:
        if c.tag not in (TAG_member, TAG_inheritance):
            continue
        t = dw.type_of(c)
        while t and t[0] in ("cv", "array"):
            t = t[-1]
        if t and t[0] == "base" and len(t) > 2 and isinstance(t[2], Die):
            if t[2].tag in AGGREGATES and t[2].name and not t[2].name.startswith("@"):
                out.append(by_value.get(t[2].name, t[2]))
            elif t[2].tag in AGGREGATES:
                out.extend(_type_deps(dw, t[2], by_value))
    return out


def _bases(t, out):
    """Collect the aggregate DIEs a type tree mentions (through pointers too)."""
    k = t[0]
    if k == "base":
        if len(t) > 2 and isinstance(t[2], Die) and t[2].tag in AGGREGATES:
            out.append(t[2])
    elif k == "func":
        if t[1] is not None:
            _bases(t[1], out)
        for p in t[2]:
            _bases(p, out)
    else:
        _bases(t[-1], out)
    return out


def _subtree_types(dw, die, out):
    t = dw.type_of(die)
    if t:
        _bases(t, out)
    for c in die.children:
        _subtree_types(dw, c, out)
    return out


def global_refs(dw, f):
    b = f.attrs.get(AT_MW_global_refs, b"")
    return [dw.dies[r] for r in (_u32(b, i)[0] for i in range(0, len(b) - 3, 4))
            if r in dw.dies]


def cmd_header(dw, args):
    needle = args.cu.lower()
    tus = [t for t in dw.tus if not args.overlay or t.overlay == args.overlay]

    def narrow(names):
        ex = [n for n in names if basename(n).lower() == needle or n.lower() == needle]
        return ex if ex else names

    whole = narrow(sorted({t.name for t in tus if needle in (t.name or "").lower()}))
    if len(whole) == 1:
        for i, t in enumerate(t for t in tus if t.name == whole[0]):
            if i:
                print()
            print_header(dw, t)
        return 0
    files = narrow(sorted({cu.name for t in tus for cu in t.cus
                           if needle in (cu.name or "").lower()}))
    if len(whole) > 1 or len(files) != 1:
        cands = whole or files
        print(f"{len(cands)} source files match {args.cu!r}:", file=sys.stderr)
        for n in cands[:60]:
            print(f"  {n}", file=sys.stderr)
        return 1
    first = True
    for t in tus:
        if any(cu.name == files[0] for cu in t.cus):
            if not first:
                print()
            first = False
            print_header(dw, t, only=files[0])
    return 0


def print_header(dw, tu, only=None):
    """A translation unit as a pseudo C header. With `only`, just the
    functions of that (included) file and the types and globals they use."""
    funcs = [f for f in tu.functions() if only is None or f.cu.name == only]
    decl_kids = tu.decl.children if tu.decl else []
    variables = [d for d in decl_kids if d.tag in (TAG_global_variable, TAG_local_variable)]
    if only is not None:
        touched = {g.off for f in funcs for g in global_refs(dw, f)}
        variables = [d for d in variables if d.off in touched]
        roots = []
        for f in funcs:
            _subtree_types(dw, f, roots)
        for v in variables:
            _subtree_types(dw, v, roots)
    else:
        roots = [d for d in decl_kids if d.tag in AGGREGATES]
    head = tu.cus[0].attrs
    pcs = [(cu.attrs[AT_low_pc], cu.attrs[AT_high_pc]) for cu in tu.cus
           if AT_low_pc in cu.attrs and (only is None or cu.name == only)]
    files = collections.Counter(f.cu.name for f in tu.functions())
    print("/*")
    print(f" * {only or tu.name}")
    if only is not None:
        print(f" * part of translation unit {tu.name}")
        print(f" * (functions from this file only; types and globals limited to what they use)")
    print(f" * {head.get(AT_producer, '')}, overlay {tu.overlay}; "
          f"{len(funcs)} functions" + (f", code 0x{min(p[0] for p in pcs):08x}-"
                                       f"0x{max(p[1] for p in pcs):08x}" if pcs else ""))
    if only is None and len(files) > 1:
        print(" * functions come from: " + ", ".join(
            f"{basename(n)} ({c})" for n, c in files.items()))
    print(" * reconstructed from DWARF 1 by tools/dwarf1.py; offsets in hex")
    print(" */")

    # Types: the roots, plus whatever they need complete (members by value),
    # dependencies first. Everything named gets a forward declaration.
    by_name = {}
    for d in roots:
        if d.name and not d.name.startswith("@"):
            by_name.setdefault(d.name, d)
    order, state, mentioned = [], set(), {}

    def visit(d):
        if d.off in state:
            return
        state.add(d.off)
        for dep in _type_deps(dw, d, by_name):
            visit(dep)
        order.append(d)
        for c in d.children:
            if c.tag in (TAG_member, TAG_inheritance):
                for m in _subtree_types(dw, c, []):
                    if m.name and not m.name.startswith("@"):
                        mentioned.setdefault(m.name, m)
    for d in by_name.values():
        visit(d)
    for d in order:
        mentioned.setdefault(d.name, d)
    fwd = [n for n, d in mentioned.items() if d.tag != TAG_enumeration_type]
    if fwd:
        print("\n/* forward declarations */")
        line = ""
        for n in fwd:
            item = f"struct {n}; "
            if len(line) + len(item) > 96:
                print(line.rstrip())
                line = ""
            line += item
        if line:
            print(line.rstrip())
    if order:
        print(f"\n/* types ({len(order)}) */")
        for i, d in enumerate(order):
            if i:
                print()
            print("\n".join(dw.render_aggregate(d)))

    # Variables, and functions referenced (declared as objects of function type).
    gl, ext = [], []
    for d in variables:
        t = dw.type_of(d) or VOID
        loc = d.attrs.get(AT_location)
        ls = loc_str(loc) if loc else "?"
        if ls == "@0x00000000":
            ls = "not allocated"
        if t[0] == "func":
            ext.append(f"{cdecl(t, dw.qualname(d))};".ljust(64) + f" /* {ls} */")
        else:
            st = "static " if d.tag == TAG_local_variable else ""
            size = dw.sizeof(t)
            sz = f" size 0x{size:x}" if size is not None else ""
            gl.append(f"{st}{cdecl(t, dw.qualname(d))};".ljust(64) + f" /* {ls}{sz} */")
    if gl:
        print(f"\n/* variables ({len(gl)}) */")
        print("\n".join(dict.fromkeys(gl)))
    if ext:
        print(f"\n/* functions referenced ({len(ext)}) */")
        print("\n".join(dict.fromkeys(ext)))
    if funcs:
        print(f"\n/* functions defined ({len(funcs)}) */")
        by_file = collections.OrderedDict()
        for f in funcs:
            by_file.setdefault(f.cu.name, []).append(f)
        for fname, fs in by_file.items():
            if len(by_file) > 1:
                print(f"\n/* {fname} */")
            for f in sorted(fs, key=lambda f: f.attrs.get(AT_low_pc, 0)):
                rng = dw.line_range(f)
                lo, hi = f.attrs.get(AT_low_pc, 0), f.attrs.get(AT_high_pc, 0)
                where = f"0x{lo:08x}-0x{hi:08x}" + (f" lines {rng[0]}-{rng[1]}" if rng else "")
                print(f"{dw.prototype(f)};".ljust(64) + f" /* {where} */")


def cmd_stats(dw, args):
    tags, attrs = collections.Counter(), collections.Counter()
    for d in dw.dies.values():
        tags[d.tag] += 1
        for code in d.attrs:
            attrs[code] += len(d.all(code))
    print(f"DIEs {len(dw.dies)} (+{dw.nulls} null entries), .debug 0x{len(dw.buf):x} bytes")
    print(f"compile units {len(dw.cus)} ({sum(1 for c in dw.cus if AT_low_pc in c.attrs)} "
          f"with code), translation units {len(dw.tus)}, source file paths "
          f"{len({k[0] for k in dw.units})}, overlays {len(dw.overlay_dies)}")
    nfun = sum(tags[t] for t in FUNCTIONS)
    ntypes = sum(tags[t] for t in AGGREGATES)
    uniq = len({d.name for d in dw.dies.values() if d.tag in AGGREGATES})
    nglob = len(dw.globals())
    print(f"functions {nfun} (with code {len(dw.funcs)}), aggregate types {ntypes} "
          f"({uniq} distinct names), unique globals/statics {nglob}")
    print("\ntags")
    for t, c in sorted(tags.items()):
        known = "" if t in TAG and t < 0x4080 else "   (vendor)"
        print(f"  0x{t:04x} {TAG.get(t, '?'):24} {c:8}{known}")
    print("\nattributes")
    for a, c in sorted(attrs.items()):
        known = "" if (a >> 4) < 0x200 else "   (vendor)"
        print(f"  0x{a:04x} {at_name(a):24} {FORM.get(a & 15, '?'):7} {c:8}{known}")
    ops = collections.Counter()
    for d in dw.dies.values():
        for code, v in d.attrs.items():
            if code == AT_location or code == AT_return_addr or \
                    (0x2013 <= code <= 0x2173 and code & 15 == 3):
                for o, _ in loc_ops(v):
                    ops[o] += 1
    print("\nlocation ops")
    for o, c in sorted(ops.items()):
        print(f"  0x{o:02x} {OP.get(o, '?'):10} {c:8}")
    ft = collections.Counter()
    for d in dw.dies.values():
        a = d.attrs
        if AT_fund_type in a:
            ft[a[AT_fund_type]] += 1
        elif AT_mod_fund_type in a:
            b = a[AT_mod_fund_type]
            ft[_u16(b, len(b) - 2)[0]] += 1
    print("\nfundamental types")
    for t, c in sorted(ft.items()):
        n, s = FT.get(t, ("?", None))
        print(f"  0x{t:04x} {n:20} size {s!s:4} {c:8}")
    print("\noverlays")
    for o in dw.overlay_dies:
        a = o.attrs
        print(f"  id {a.get(AT_MW_overlay_id)} {a.get(AT_MW_overlay_name):12} "
              f"0x{a.get(AT_low_pc, 0):08x}-0x{a.get(AT_high_pc, 0):08x} "
              f"{len(o.all(AT_member))} CUs")
    dirs = collections.defaultdict(lambda: [set(), 0, set()])
    for (name, _ov), cus in dw.units.items():
        e = dirs[dirname(name)]
        e[0].add(name)
        e[1] += len(cus)
    for tu in dw.tus:
        dirs[dirname(tu.name)][2].add(tu.name)
    print("\nsource directories (files, CUs, translation units)")
    for k, (f, c, t) in sorted(dirs.items()):
        print(f"  {len(f):5} {c:6} {len(t):4}  {k}")


def cmd_check(dw, args):
    print(f"tree: {len(dw.dies)} DIEs, {dw.nulls} null entries, "
          f"{len(dw.problems)} sibling/length problems")
    for p in dw.problems[:20]:
        print("  " + p)
    # Functions vs ELF symbol table
    idx = dw.elf_symbols()
    res = collections.Counter()
    bad = []
    for f in dw.funcs:
        lo, hi = f.attrs[AT_low_pc], f.attrs[AT_high_pc]
        names = {f.name, dw.mangled(f)}
        syms = [s for s in idx.get((dw.overlay(f), lo), []) if s.type == 2]
        named = [s for s in syms if s.name in names]
        if named and any(s.size == hi - lo for s in named):
            res["exact (address, size, name)"] += 1
        elif named:
            res["address+name, size differs"] += 1
            bad.append((f, named[0], "size"))
        elif syms:
            res["address only (name differs)"] += 1
            bad.append((f, syms[0], "name"))
        else:
            res["no ELF symbol at low_pc"] += 1
            bad.append((f, None, "none"))
    print(f"\nfunctions with code: {len(dw.funcs)}")
    for k, v in res.most_common():
        print(f"  {v:6}  {k}")
    for f, s, why in bad[:12]:
        lo, hi = f.attrs[AT_low_pc], f.attrs[AT_high_pc]
        sd = f"{s.name} size 0x{s.size:x}" if s else "-"
        print(f"    {why:5} {dw.overlay(f):12} 0x{lo:08x} size 0x{hi - lo:x} "
              f"{dw.mangled(f) or f.name}  vs  {sd}")
    # Demangled signature vs DWARF parameter types (`this` set aside)
    cat = collections.Counter()
    diffs = collections.Counter()
    samples = []
    for f in dw.funcs:
        m = dw.mangled(f)
        s = parse_mangled(m) if m else None
        if not s or not s.is_func:
            cat["no mangled signature"] += 1
            continue
        dt = [cdecl(dw.type_of(p) or VOID) for p in f.children
              if p.tag == TAG_formal_parameter and p.name != "this"]
        mt = [cdecl(t) for t in s.params]
        if dt == mt:
            cat["identical"] += 1
        elif len(dt) == len(mt) and all(same_type(a, b) for a, b in zip(dt, mt)):
            cat["identical up to nested-class qualification"] += 1
        elif len(dt) == len(mt) and all(compatible(a, b) for a, b in zip(dt, mt)):
            cat["DWARF drops const (or says unsigned char for bool)"] += 1
        elif len(dt) < len(mt) and len(dw.params(f)) == len(mt):
            cat["DWARF omits unnamed/unused params, rest align"] += 1
        else:
            cat["mismatch"] += 1
            for a, b in zip(dt, mt):
                if not same_type(a, b):
                    diffs[(a, b)] += 1
                    break
            if len(samples) < 8:
                samples.append((m, dt, mt))
    this_ok = sum(1 for f in dw.funcs if dw.has_this(f) and
                  (parse_mangled(dw.mangled(f) or "") or None) and
                  parse_mangled(dw.mangled(f)).classname)
    this_bad = sum(1 for f in dw.funcs if dw.has_this(f) and not
                   ((parse_mangled(dw.mangled(f) or "") or None) and
                    parse_mangled(dw.mangled(f)).classname))
    print(f"\ndemangled parameter types vs DWARF ({sum(cat.values())} functions with code)")
    for k, v in cat.most_common():
        print(f"  {v:6}  {k}")
    print(f"  `this` parameter: {this_ok} on methods, {this_bad} on non-methods; "
          f"{sum(1 for f in dw.funcs if dw.lacks_this(f))} methods have none (static, or "
          f"this unused)")
    for (a, b), c in diffs.most_common(12):
        print(f"  {c:5}  DWARF {a!r}  vs  mangled {b!r}")
    for m, dt, mt in samples:
        print(f"    {m}\n      DWARF   {dt}\n      mangled {mt}")
    # AT_name against the demangled member name
    names = collections.Counter()
    for d in dw.dies.values():
        m = d.attrs.get(AT_MW_mangled)
        if not m:
            continue
        s = parse_mangled(m)
        if s is None:
            names["mangled name does not demangle"] += 1
        elif d.name == s.name:
            names["AT_name == demangled name"] += 1
        elif d.name == m[:m.find("__", 1)]:
            names["AT_name == raw special name (__ct, __dt, __nw ...)"] += 1
        else:
            names["differs"] += 1
    print(f"\nAT_name vs demangled MW_mangled_name ({sum(names.values())} DIEs)")
    for k, v in names.most_common():
        print(f"  {v:6}  {k}")
    # ELF function symbols that DWARF does not describe
    have = {(dw.overlay(f), f.attrs[AT_low_pc]) for f in dw.funcs}
    missing = [s for (ov, v), ss in idx.items() for s in ss
               if s.type == 2 and (ov, v) not in have]
    print(f"\nELF FUNC symbols with no DWARF function at that address: {len(missing)}")
    kinds = collections.Counter()
    for s in missing:
        sec = dw.elf.sections[s.shndx].name
        kinds[(sec, "C++ (mangled)" if parse_mangled(s.name) else "C / asm")] += 1
    for (sec, k), c in sorted(kinds.items()):
        print(f"  {c:6}  {sec:12} {k}")
    print("  e.g. " + ", ".join(sorted({s.name for s in missing if parse_mangled(s.name)})[:12]))


def cmd_dump(dw, args):
    d = dw.dies.get(args.off)
    if d is None:
        print(f"no DIE at 0x{args.off:x}", file=sys.stderr)
        return 1

    def show(d, depth):
        a = " ".join(f"{at_name(k)}={_fmt_attr(k, v)}" for k, v in d.attrs.items()
                     if k != AT_sibling)
        print(f"{'  ' * depth}0x{d.off:08x} {TAG.get(d.tag, hex(d.tag))} {a}")
        if depth < args.depth:
            for c in d.children:
                show(c, depth + 1)
    show(d, 0)
    return 0


def _fmt_attr(code, v):
    if code in (AT_location, AT_return_addr) or (0x2013 <= code <= 0x2173 and code & 15 == 3):
        return f"[{loc_str(v)}]"
    if isinstance(v, bytes):
        return f"[{v.hex()}]"
    if isinstance(v, str):
        return repr(v)
    return f"0x{v:x}"


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    num = lambda s: int(s, 0)
    p = sub.add_parser("cus")
    p.add_argument("elf")
    p.add_argument("--files", action="store_true", help="one line per source file")
    p.add_argument("--tus", action="store_true", help="one line per translation unit")
    p.add_argument("--overlay")
    p = sub.add_parser("fn")
    p.add_argument("elf")
    p.add_argument("what", help="name, mangled name, Class::method, or address")
    p.add_argument("--overlay")
    p = sub.add_parser("type")
    p.add_argument("elf")
    p.add_argument("name")
    p.add_argument("--all", action="store_true", help="every distinct layout")
    p = sub.add_parser("globals")
    p.add_argument("elf")
    p.add_argument("--grep")
    p.add_argument("--overlay")
    p = sub.add_parser("lines")
    p.add_argument("elf")
    p.add_argument("va", type=num)
    p.add_argument("--overlay")
    p.add_argument("--table", action="store_true", help="dump the CU's whole table")
    p = sub.add_parser("header")
    p.add_argument("elf")
    p.add_argument("--cu", required=True, help="substring of the source file path")
    p.add_argument("--overlay")
    p = sub.add_parser("stats")
    p.add_argument("elf")
    p = sub.add_parser("check")
    p.add_argument("elf")
    p = sub.add_parser("dump")
    p.add_argument("elf")
    p.add_argument("off", type=num)
    p.add_argument("--depth", type=int, default=1)
    args = parser.parse_args()

    dw = Dwarf(args.elf)
    ov = getattr(args, "overlay", None)
    if ov and ov not in dw.overlay_names:
        full = [n for n in dw.overlay_names if n in (ov, ov + ".prg")]
        if not full:
            print(f"unknown overlay {ov!r}; have {', '.join(dw.overlay_names)}",
                  file=sys.stderr)
            return 1
        args.overlay = full[0]
    fn = {"cus": cmd_cus, "fn": cmd_fn, "type": cmd_type, "globals": cmd_globals,
          "lines": cmd_lines, "header": cmd_header, "stats": cmd_stats,
          "check": cmd_check, "dump": cmd_dump}[args.cmd]
    try:
        return fn(dw, args) or 0
    except BrokenPipeError:
        # `| head`: stop quietly, and keep the interpreter's final flush quiet
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        return 0


if __name__ == "__main__":
    sys.exit(main())
