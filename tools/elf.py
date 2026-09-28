#!/usr/bin/env python3
"""32-bit little-endian MIPS ELF, as the PS2 EE uses it.

    tools/elf.py info  ELF             header, program headers, sections
    tools/elf.py syms  ELF [--grep RE] symbol table, sorted by address
    tools/elf.py read  ELF VA [LEN]    hexdump bytes at a virtual address

Also a library: `Elf(path)` gives `.segments`, `.sections`, `.symbols`, and
`.read(va, n)` / `.u32(va)` over the loaded image. `.relocs(name)` lists the
SHT_REL entries that apply to a section.
"""

import argparse
import re
import struct
import sys

PT = {0: "NULL", 1: "LOAD", 2: "DYNAMIC", 3: "INTERP", 4: "NOTE", 6: "PHDR",
      0x70000000: "MIPS_REGINFO", 0x70000001: "MIPS_RTPROC",
      0x70000002: "MIPS_OPTIONS", 0x70000003: "MIPS_ABIFLAGS"}
SHT = {0: "NULL", 1: "PROGBITS", 2: "SYMTAB", 3: "STRTAB", 4: "RELA", 5: "HASH",
       6: "DYNAMIC", 7: "NOTE", 8: "NOBITS", 9: "REL", 11: "DYNSYM",
       0x70000006: "MIPS_REGINFO", 0x7000000d: "MIPS_OPTIONS",
       0x70000005: "MIPS_DEBUG", 0x7000001e: "MIPS_DWARF"}
STT = {0: "NOTYPE", 1: "OBJECT", 2: "FUNC", 3: "SECTION", 4: "FILE"}
STB = {0: "LOCAL", 1: "GLOBAL", 2: "WEAK"}
# 123 is not in the MIPS ABI; it matches the old binutils DVP numbering
# (R_MIPS_DVP_U15_S3) and sits on VU microcode iaddiu immediates.
R_MIPS = {0: "NONE", 1: "16", 2: "32", 3: "REL32", 4: "26", 5: "HI16", 6: "LO16",
          7: "GPREL16", 8: "LITERAL", 9: "GOT16", 10: "PC16", 123: "DVP_U15_S3"}


class Segment:
    __slots__ = ("type", "offset", "vaddr", "filesz", "memsz", "flags")

    def __init__(self, *a):
        (self.type, self.offset, self.vaddr, self.filesz, self.memsz, self.flags) = a


class Section:
    __slots__ = ("name", "type", "flags", "addr", "offset", "size", "link",
                 "info", "align", "entsize")

    def __init__(self, *a):
        (self.name, self.type, self.flags, self.addr, self.offset, self.size,
         self.link, self.info, self.align, self.entsize) = a


class Symbol:
    __slots__ = ("name", "value", "size", "type", "bind", "shndx")

    def __init__(self, *a):
        (self.name, self.value, self.size, self.type, self.bind, self.shndx) = a


class Elf:
    def __init__(self, path):
        self.path = path
        with open(path, "rb") as f:
            self.data = f.read()
        d = self.data
        if d[:4] != b"\x7fELF" or d[4] != 1 or d[5] != 1:
            raise ValueError(f"{path}: not a 32-bit little-endian ELF")
        (self.type, self.machine, _, self.entry, phoff, shoff, self.flags,
         _, phentsize, phnum, shentsize, shnum, shstrndx) = struct.unpack_from(
            "<HHIIIIIHHHHHH", d, 16)

        self.segments = []
        for i in range(phnum):
            p_type, off, va, _, filesz, memsz, flags, _ = struct.unpack_from(
                "<8I", d, phoff + i * phentsize)
            self.segments.append(Segment(p_type, off, va, filesz, memsz, flags))

        self.sections = []
        raw = []
        for i in range(shnum if shoff else 0):
            raw.append(struct.unpack_from("<10I", d, shoff + i * shentsize))
        if raw:
            strtab = raw[shstrndx]
            for r in raw:
                name = self._cstr(strtab[4] + r[0])
                self.sections.append(Section(name, *r[1:]))

        self.symbols = []
        for s in self.sections:
            if s.type not in (2, 11):
                continue
            strs = self.sections[s.link]
            for off in range(s.offset, s.offset + s.size, 16):
                name, value, size, info, _, shndx = struct.unpack_from("<IIIBBH", d, off)
                self.symbols.append(Symbol(self._cstr(strs.offset + name), value, size,
                                           info & 15, info >> 4, shndx))

    def _cstr(self, off):
        end = self.data.index(b"\0", off)
        return self.data[off:end].decode("latin-1")

    def file_offset(self, va):
        for s in self.segments:
            if s.type == 1 and s.vaddr <= va < s.vaddr + s.filesz:
                return s.offset + (va - s.vaddr)
        return None

    def read(self, va, n):
        off = self.file_offset(va)
        if off is None:
            for s in self.segments:
                if s.type == 1 and s.vaddr <= va < s.vaddr + s.memsz:
                    return bytes(n)  # .bss
            raise KeyError(f"0x{va:08x} is not in a loaded segment")
        return self.data[off:off + n]

    def u32(self, va):
        return struct.unpack("<I", self.read(va, 4))[0]

    def cstr(self, va, limit=4096):
        b = self.read(va, limit)
        return b[:b.index(b"\0")] if b"\0" in b else b

    def relocs(self, name):
        """(offset, type, Symbol) for every SHT_REL entry that applies to the
        section called `name`. Offsets are virtual addresses in a linked image."""
        idx = next((i for i, s in enumerate(self.sections) if s.name == name), None)
        out = []
        for s in self.sections:
            if s.type != 9 or s.info != idx:
                continue
            for off in range(s.offset, s.offset + s.size, 8):
                r_offset, info = struct.unpack_from("<II", self.data, off)
                out.append((r_offset, info & 0xff, self.symbols[info >> 8]))
        return out

    def section_at(self, va):
        for s in self.sections:
            if s.addr and s.addr <= va < s.addr + s.size:
                return s
        return None


def hexdump(base, data):
    for i in range(0, len(data), 16):
        row = data[i:i + 16]
        hx = " ".join(f"{b:02x}" for b in row)
        asc = "".join(chr(b) if 32 <= b < 127 else "." for b in row)
        print(f"{base + i:08x}  {hx:<47}  {asc}")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("info")
    p.add_argument("elf")
    p = sub.add_parser("syms")
    p.add_argument("elf")
    p.add_argument("--grep")
    p = sub.add_parser("read")
    p.add_argument("elf")
    p.add_argument("va", type=lambda s: int(s, 0))
    p.add_argument("len", type=lambda s: int(s, 0), nargs="?", default=256)
    args = parser.parse_args()

    elf = Elf(args.elf)
    if args.cmd == "info":
        print(f"type={elf.type} machine={elf.machine} entry=0x{elf.entry:08x} "
              f"flags=0x{elf.flags:08x}")
        print("\nprogram headers")
        for s in elf.segments:
            print(f"  {PT.get(s.type, hex(s.type)):14} off=0x{s.offset:08x} "
                  f"va=0x{s.vaddr:08x} filesz=0x{s.filesz:08x} memsz=0x{s.memsz:08x} "
                  f"flags={s.flags}")
        print(f"\nsections ({len(elf.sections)})")
        for s in elf.sections:
            print(f"  {s.name:24} {SHT.get(s.type, hex(s.type)):14} addr=0x{s.addr:08x} "
                  f"off=0x{s.offset:08x} size=0x{s.size:08x} flags=0x{s.flags:x}")
        print(f"\nsymbols: {len(elf.symbols)}")
    elif args.cmd == "syms":
        pat = re.compile(args.grep) if args.grep else None
        for s in sorted(elf.symbols, key=lambda s: (s.value, s.name)):
            if pat and not pat.search(s.name):
                continue
            print(f"0x{s.value:08x} {s.size:8d} {STT.get(s.type, s.type):7} "
                  f"{STB.get(s.bind, s.bind):6} {s.shndx:5} {s.name}")
    elif args.cmd == "read":
        hexdump(args.va, elf.read(args.va, args.len))
    return 0


if __name__ == "__main__":
    sys.exit(main())
