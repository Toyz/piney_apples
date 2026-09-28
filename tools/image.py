#!/usr/bin/env python3
"""The EE's view of memory: the main ELF plus, optionally, one overlay.

The retail ELF carries symbols and relocations for every overlay but no
bytes for them. Each overlay is a DATA/<NAME>.PRG file next to the ELF,
loaded whole (its 0x40-byte "MWo3" header included) at 0x00400800, so the
same address means different code depending on which overlay is resident
and symbol lookup has to be per overlay.

    tools/image.py info ELF [--overlay NAME]              regions and counts
    tools/image.py at   ELF VA [--overlay NAME]           symbol, reloc, bytes at an address

A stripped executable (the later volumes) loads too: its sections are
recognised by position, and names come from `<elf>.syms` when that file
exists (see piney-gen syms).

Also a library: `Program(elf_path, overlay=None, symbols=None)` with `.read(va, n)`,
`.u32(va)`, `.cstr(va)`, `.symbol_at(va)`, `.symbol_named(name)`,
`.functions()`, `.function_at(va)`, `.relocs_at(va)` and `.relocs`, where
every relocation is resolved to the exact address it produces.
"""

import argparse
import bisect
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from elf import Elf, R_MIPS, hexdump  # noqa: E402

OVERLAYS = ("gcmn", "demo", "desktop", "toppage")
STT_FUNC, STT_OBJECT, STT_SECTION, STT_FILE = 2, 1, 3, 4
SHN_UNDEF = 0


class Overlay:
    """A .PRG overlay: header fields plus the bytes as loaded."""
    HEADER = struct.Struct("<4sIIIIIII32s")

    def __init__(self, path):
        self.path = path
        with open(path, "rb") as f:
            self.data = f.read()
        (magic, self.id, self.base, self.text_size, self.data_size, self.bss_size,
         self.ctor_start, self.ctor_end, name) = self.HEADER.unpack_from(self.data)
        if magic != b"MWo3":
            raise ValueError(f"{path}: not an MWo3 overlay")
        self.name = name.split(b"\0", 1)[0].decode("ascii")
        self.text = self.base + self.HEADER.size
        self.end = self.text + self.text_size + self.data_size + self.bss_size

    def read(self, va, n):
        off = va - self.base
        b = self.data[off:off + n]
        return b + bytes(n - len(b))  # past the file is .bss


class Reloc:
    """One relocation, with `addr` the address it resolves to in the linked
    image (the jal/j target, the lui/lo16 pair, gp+offset, or the word)."""
    __slots__ = ("offset", "type", "symbol", "addr", "partner")

    def __init__(self, offset, type, symbol):
        self.offset = offset
        self.type = type
        self.symbol = symbol
        self.addr = None
        self.partner = None  # HI16 <-> LO16

    @property
    def type_name(self):
        return "R_MIPS_" + R_MIPS.get(self.type, str(self.type))


def _sext16(v):
    return v - 0x10000 if v & 0x8000 else v


# A stripped ELF keeps its section headers but not their names; the sections
# stay in the order the linker wrote them.
SECTION_ORDER = ("main", "gcmn.prg", "demo.prg", "desktop.prg", "toppage.prg", "heap")


def load_symbol_file(path, index):
    """Symbols from a sidecar file (piney-gen syms writes them for the stripped
    volumes): one per line, `section va size type name`, tab-separated, where
    section is main or an overlay name and type is FUNC or OBJECT."""
    from elf import Symbol
    out = []
    kinds = {"FUNC": STT_FUNC, "OBJECT": STT_OBJECT}
    with open(path, encoding="utf-8") as f:
        for line in f:
            if not line.strip() or line.startswith("#"):
                continue
            sec, va, size, kind, name = line.rstrip("\n").split("\t")[:5]
            shndx = index.get(sec if sec == "main" else sec + ".prg")
            if shndx is None:
                continue
            out.append(Symbol(name, int(va, 16), int(size), kinds.get(kind, 0), 1, shndx))
    return out


class Program:
    def __init__(self, elf_path, overlay=None, symbols=None):
        self.elf = elf = Elf(elf_path)
        self.path = elf_path
        index = {s.name: i for i, s in enumerate(elf.sections)}
        if "main" not in index:
            loaded = [i for i, s in enumerate(elf.sections) if s.type == 1 and s.addr]
            index.update(zip(SECTION_ORDER, loaded))
            # Sidecar names for a stripped executable, unless told otherwise.
            if symbols is None and os.path.exists(elf_path + ".syms"):
                symbols = elf_path + ".syms"
        if symbols:
            elf.symbols = list(elf.symbols) + load_symbol_file(symbols, index)
        self.main_index = index["main"]
        main = elf.sections[self.main_index]
        self.main_seg = next(s for s in elf.segments
                             if s.type == 1 and s.vaddr <= main.addr < s.vaddr + max(s.memsz, 1))
        self.overlay_index = {n: index[n + ".prg"] for n in OVERLAYS if n + ".prg" in index}
        # All overlays share one window; the ELF sizes it with empty segments.
        base = min((elf.sections[i].addr for i in self.overlay_index.values()), default=0)
        self.overlay_space = (base, max((s.vaddr + s.memsz for s in elf.segments
                                         if s.vaddr == base), default=base))

        reginfo = next((s for s in elf.sections if s.name == ".reginfo"), None)
        self.gp = struct.unpack_from("<I", elf.data, reginfo.offset + 20)[0] if reginfo else None
        if not self.gp:
            self.gp = next((s.value for s in elf.symbols if s.name == "_gp"), None)

        self.overlay = None
        self.overlay_name = None
        if overlay:
            name = overlay.lower().removesuffix(".prg")
            if name not in self.overlay_index:
                raise ValueError(f"unknown overlay {overlay!r}; one of {', '.join(OVERLAYS)}")
            path = os.path.join(os.path.dirname(os.path.abspath(elf_path)), "DATA",
                                name.upper() + ".PRG")
            self.overlay = Overlay(path)
            self.overlay_name = name
        self._other = {i for n, i in self.overlay_index.items() if n != self.overlay_name}

        self.symbols = sorted((s for s in elf.symbols if self._active(s)),
                              key=lambda s: (s.value, -s.size))
        self._starts = [s.value for s in self.symbols]
        self._named = {}
        rank = {STT_FUNC: 0, STT_OBJECT: 1}
        for s in self.symbols:
            cur = self._named.get(s.name)
            if cur is None or rank.get(s.type, 2) < rank.get(cur.type, 2):
                self._named[s.name] = s
        seen = set()
        self._functions = []
        for s in self.symbols:
            if s.type == STT_FUNC and s.size and s.value not in seen:
                seen.add(s.value)
                self._functions.append(s)
        self._fstarts = [s.value for s in self._functions]
        self._relocs = None

    # -- memory -----------------------------------------------------------

    def foreign(self, s):
        """True for a symbol that lives in an overlay other than the loaded
        one: its address means something else in this program."""
        return s.shndx in self._other

    def _active(self, s):
        if not s.name or s.type in (STT_SECTION, STT_FILE) or s.shndx == SHN_UNDEF:
            return False
        return s.shndx not in self._other

    def mapped(self, va):
        """True if `va` has bytes (or .bss) in this program."""
        ov = self.overlay
        if ov and ov.base <= va < ov.end:
            return True
        m = self.main_seg
        return m.vaddr <= va < m.vaddr + m.memsz

    def read(self, va, n):
        ov = self.overlay
        if ov and ov.base <= va < ov.end:
            return ov.read(va, n)
        m = self.main_seg
        if m.vaddr <= va < m.vaddr + m.memsz:
            return self.elf.read(va, n)
        if not ov and self.overlay_space[0] <= va < self.overlay_space[1]:
            raise KeyError(f"0x{va:08x} is overlay space; choose one with --overlay")
        raise KeyError(f"0x{va:08x} is not mapped")

    def u32(self, va):
        return struct.unpack("<I", self.read(va, 4))[0]

    def cstr(self, va, limit=256):
        b = self.read(va, limit)
        return b[:b.index(b"\0")] if b"\0" in b else b

    def string_at(self, va, min_len=3):
        """The C string at `va` if it looks like text (ASCII or Shift-JIS)."""
        try:
            b = self.read(va, 256)
        except KeyError:
            return None
        end = b.find(b"\0")
        if end < min_len:
            return None
        try:
            s = b[:end].decode("cp932")
        except UnicodeDecodeError:
            return None
        # Half-width katakana are single bytes 0xa1-0xdf: binary data decodes
        # as them all the time, game text almost never uses them.
        if not all((c.isprintable() or c in "\t\n\r") and not "\uff61" <= c <= "\uff9f"
                   for c in s):
            return None
        # A sized object much bigger than the string is a struct, not text.
        hit = self.symbol_at(va)
        if hit and hit[1] == 0 and hit[0].size > 2 * (end + 1) + 16:
            return None
        return s

    # -- symbols ----------------------------------------------------------

    def section_name(self, shndx):
        if 0 < shndx < len(self.elf.sections):
            return self.elf.sections[shndx].name
        return {0: "UND", 0xfff1: "ABS", 0xfff2: "COMMON"}.get(shndx, str(shndx))

    def symbol_named(self, name):
        return self._named.get(name)

    def symbol_at(self, va, near=0):
        """(symbol, offset) for the innermost sized symbol covering `va`, else
        a symbol starting exactly there, else the nearest one at most `near`
        bytes below. None if nothing fits."""
        i = bisect.bisect_right(self._starts, va)
        exact = None
        for j in range(i - 1, max(i - 200, 0) - 1, -1):
            s = self.symbols[j]
            if s.value <= va < s.value + s.size:
                return s, va - s.value
            if s.value == va and exact is None:
                exact = s
        if exact:
            return exact, 0
        if i and va - self.symbols[i - 1].value <= near:
            s = self.symbols[i - 1]
            return s, va - s.value
        return None

    def name_at(self, va, near=0):
        hit = self.symbol_at(va, near)
        if not hit:
            return None
        s, off = hit
        return s.name if off == 0 else f"{s.name}+0x{off:x}"

    def functions(self):
        """Sized FUNC symbols in this program, by address."""
        return self._functions

    def function_at(self, va):
        i = bisect.bisect_right(self._fstarts, va) - 1
        if i >= 0:
            f = self._functions[i]
            if f.value <= va < f.value + f.size:
                return f
        return None

    # -- relocations ------------------------------------------------------

    @property
    def relocs(self):
        """Every relocation for main and the loaded overlay, resolved."""
        if self._relocs is None:
            sections = ["main"] + ([self.overlay_name + ".prg"] if self.overlay else [])
            self._relocs = []
            for name in sections:
                self._relocs.extend(self._resolve(self.elf.relocs(name)))
            self._by_offset = {}
            for r in self._relocs:
                self._by_offset.setdefault(r.offset, []).append(r)
        return self._relocs

    def relocs_at(self, va):
        self.relocs
        return self._by_offset.get(va, ())

    def _resolve(self, raw):
        out = []
        his = {}      # symbol -> [(offset, lui rt, reloc)] in address order
        last_hi = {}  # symbol -> latest HI16 in table order
        for off, typ, sym in raw:
            r = Reloc(off, typ, sym)
            out.append(r)
            if typ == 5:
                try:
                    w = self.u32(off)
                except KeyError:
                    continue
                his.setdefault(id(sym), []).append((off, (w >> 16) & 31, r))
        for v in his.values():
            v.sort(key=lambda h: h[0])
        for r in out:
            try:
                w = self.u32(r.offset)
            except KeyError:
                continue
            typ = r.type
            if typ == 4:
                r.addr = ((r.offset + 4) & 0xf0000000) | ((w & 0x3ffffff) << 2)
            elif typ == 2:
                r.addr = w
            elif typ == 7:
                r.addr = (self.gp + _sext16(w & 0xffff)) & 0xffffffff
            elif typ == 5:
                last_hi[id(r.symbol)] = r
            elif typ == 6:
                # The linked immediates already hold the final value, so the
                # address is the partner lui's immediate plus this one. The
                # ABI pairs a LO16 with the latest HI16 in table order, but
                # code that shares one lui across uses breaks that; prefer
                # the nearest lui before it that writes the register it uses.
                hi = self._partner(his.get(id(r.symbol), ()), r.offset, (w >> 21) & 31)
                hi = hi or last_hi.get(id(r.symbol))
                lo = _sext16(w & 0xffff)
                if hi is not None:
                    r.addr = (((self.u32(hi.offset) & 0xffff) << 16) + lo) & 0xffffffff
                    r.partner = hi
                    if hi.addr is None:
                        hi.addr = r.addr
                        hi.partner = r
                else:
                    r.addr = (r.symbol.value + _sext16((lo - r.symbol.value) & 0xffff)) & 0xffffffff
        return out

    def _partner(self, his, offset, reg):
        f = self.function_at(offset)
        lo = f.value if f else offset - 0x1000
        i = bisect.bisect_left(his, offset, key=lambda h: h[0])
        for off, rt, r in reversed(his[max(i - 64, 0):i]):
            if off < lo:
                break
            if rt == reg:
                return r
        return None

    def reloc_target_name(self, r, addr=None):
        """How to name what a reloc points at: its symbol (+addend) unless
        that is a section symbol, in which case whatever covers the address."""
        s = r.symbol
        addr = r.addr if addr is None else addr
        if addr is None:
            return s.name or "?"
        if s.type == STT_SECTION or not s.name:
            return self.name_at(addr) or f"0x{addr:08x}"
        off = (addr - s.value) & 0xffffffff
        if off == 0:
            return s.name
        if off >= 0x80000000:
            return f"{s.name}-0x{0x100000000 - off:x}"
        return f"{s.name}+0x{off:x}"


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("info")
    p.add_argument("elf")
    p.add_argument("--overlay")
    p = sub.add_parser("at")
    p.add_argument("elf")
    p.add_argument("va", type=lambda s: int(s, 0))
    p.add_argument("--overlay")
    args = parser.parse_args()

    prog = Program(args.elf, args.overlay)
    if args.cmd == "info":
        m = prog.main_seg
        print(f"main     0x{m.vaddr:08x}-0x{m.vaddr + m.filesz:08x} "
              f"(+bss to 0x{m.vaddr + m.memsz:08x})  gp=0x{prog.gp:08x}")
        ov = prog.overlay
        if ov:
            print(f"{prog.overlay_name:8} 0x{ov.base:08x} header, text 0x{ov.text:08x}+0x{ov.text_size:x}, "
                  f"data +0x{ov.data_size:x}, bss +0x{ov.bss_size:x} (to 0x{ov.end:08x}), "
                  f"ctors 0x{ov.ctor_start:08x}-0x{ov.ctor_end:08x}  [{ov.path}]")
        print(f"symbols {len(prog.symbols)}, functions {len(prog.functions())}, "
              f"relocs {len(prog.relocs)}")
    else:
        hit = prog.symbol_at(args.va, near=0x10000)
        if hit:
            s, off = hit
            print(f"{s.name}+0x{off:x}  (0x{s.value:08x} size {s.size} "
                  f"in {prog.section_name(s.shndx)})")
        for r in prog.relocs_at(args.va):
            print(f"reloc {r.type_name} -> {prog.reloc_target_name(r)}")
        hexdump(args.va, prog.read(args.va, 64))
    return 0


if __name__ == "__main__":
    sys.exit(main())
