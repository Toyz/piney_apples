#!/usr/bin/env python3
"""Any global table, decoded through its DWARF type.

    tools/tables.py dump ELF NAME [--overlay X] [--tsv] [--raw] [--limit N]
    tools/tables.py list ELF [--overlay X] [--min BYTES]    arrays of structs
    tools/tables.py dumpall ELF DIR [--overlay X] [--min BYTES]
                                  every array `list` shows, each as DIR/NAME.tsv
                                  (its summary line in DIR/NAME.err)

The global is looked up in the DWARF (tools/dwarf1.py), its element type is
walked member by member, and each element becomes one row: numbers as
numbers, `char *` and `char[]` as text (Shift-JIS decoded, the game's escapes
kept as bytes), other pointers as the symbol they point at, nested structs
flattened to `outer.inner`, small arrays as lists. For an overlay the
constructor table is run first (tools/eemu.py), so tables patched at start-up
read as the game sees them; --raw skips that. Arrays DWARF declares without
a length (`T tbl[]`) take their length from the ELF symbol's size.
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dwarf1  # noqa: E402
from elf import Elf  # noqa: E402
from image import Program  # noqa: E402

INTS = {1: "b", 2: "h", 4: "i", 8: "q"}


class Reader:
    def __init__(self, elf_path, overlay, raw):
        self.prog = Program(elf_path, overlay)
        self.mem = None
        if overlay and not raw:
            import eemu
            self.mem = eemu.run_ctors(self.prog).mem

    def read(self, va, n):
        if self.mem is not None:
            return bytes(self.mem[va:va + n])
        return self.prog.read(va, n)

    def cstr(self, va, limit=512):
        if not va:
            return None
        try:
            b = self.read(va, limit)
        except Exception:
            return f"<0x{va:08x}>"
        b = b.split(b"\0", 1)[0]
        return b.decode("shift_jis", "backslashreplace")

    def name_at(self, va):
        return self.prog.name_at(va) or f"0x{va:08x}"


class Decoder:
    def __init__(self, dw, reader):
        self.dw = dw
        self.r = reader

    def scalar(self, t, raw):
        """t is a ('base', name, ft) tree for a fundamental type."""
        name = t[1]
        n = len(raw)
        if name == "float" and n == 4:
            return round(struct.unpack("<f", raw)[0], 6)
        if name in ("double", "long double") and n == 8:
            return struct.unpack("<d", raw)[0]
        if name == "u_long128":
            return "0x" + raw[::-1].hex()
        signed = not name.startswith("unsigned") and name not in ("bool",)
        if n in INTS:
            fmt = "<" + (INTS[n] if signed else INTS[n].upper())
            return struct.unpack(fmt, raw)[0]
        return raw.hex()

    def value(self, t, va, prefix, out):
        """Decode one object of type t at va into out[(column)] = value."""
        k = t[0]
        if k == "cv":
            return self.value(t[2], va, prefix, out)
        if k in ("ptr", "ref"):
            p = struct.unpack("<I", self.r.read(va, 4))[0]
            inner = dwarf1.innermost(t[1]) if t[1][0] != "func" else t[1]
            if inner[0] == "base" and inner[1] in ("char", "signed char", "unsigned char"):
                out[prefix] = self.r.cstr(p)
            elif p == 0:
                out[prefix] = None
            else:
                out[prefix] = "&" + self.r.name_at(p)
            return
        if k == "array":
            n, elem = t[1], t[2]
            size = self.dw.sizeof(elem) or 0
            if elem[0] == "base" and elem[1] in ("char", "signed char", "unsigned char") and n:
                raw = self.r.read(va, n)
                if elem[1] == "char":
                    out[prefix] = raw.split(b"\0", 1)[0].decode("shift_jis", "backslashreplace")
                    return
            if n is None or not size:
                out[prefix] = f"<array at 0x{va:08x}>"
                return
            if n > 64:
                out[prefix] = f"<{n} x {size} bytes at 0x{va:08x}>"
                return
            inner = {}
            if elem[0] == "base" and not isinstance(elem[2], dwarf1.Die):
                out[prefix] = [self.scalar(elem, self.r.read(va + i * size, size))
                               for i in range(n)]
                return
            for i in range(n):
                self.value(elem, va + i * size, f"{prefix}[{i}]", inner)
            out.update(inner)
            return
        if k == "base":
            ref = t[2] if len(t) > 2 else None
            if isinstance(ref, dwarf1.Die):
                self.aggregate(ref, va, prefix, out)
                return
            size = self.dw.sizeof(t) or 0
            out[prefix] = self.scalar(t, self.r.read(va, size)) if size else None
            return
        out[prefix] = f"<{k}>"

    def aggregate(self, d, va, prefix, out):
        # A declaration can stand in for the full type; use a definition.
        if not [c for c in d.children if c.tag == dwarf1.TAG_member]:
            full = [a for a in self.dw.aggregates(d.name or "")
                    if any(c.tag == dwarf1.TAG_member for c in a.children)]
            if full:
                d = full[0]
        if d.tag == dwarf1.TAG_enumeration_type:
            size = d.attrs.get(dwarf1.AT_byte_size, 4)
            out[prefix] = int.from_bytes(self.r.read(va, size), "little", signed=True)
            return
        for c in d.children:
            if c.tag != dwarf1.TAG_member:
                continue
            off = dwarf1.member_offset(c)
            if off is None:
                continue
            t = self.dw.type_of(c) or dwarf1.VOID
            col = f"{prefix}.{c.name}" if prefix else c.name
            bits = c.attrs.get(dwarf1.AT_bit_size)
            if bits is not None:
                size = self.dw.sizeof(t) or 4
                word = int.from_bytes(self.r.read(va + off, size), "little")
                shift = c.attrs.get(dwarf1.AT_bit_offset, 0)   # from the LSB here
                out[col] = (word >> shift) & ((1 << bits) - 1)
                continue
            self.value(t, va + off, col, out)


def find_global(dw, elf, name, overlay):
    home = overlay or dwarf1.MAIN
    for (qn, addr, where, _), g in dw.globals(overlay).items():
        if qn != name:
            continue
        if addr == 0 or addr is None:
            cands = dw.elf_address(g["die"], where)
            if len(cands) != 1:
                continue
            addr = cands[0]
        return addr, g["type"], where
    raise SystemExit(f"no global {name} in {home}")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("dump")
    p.add_argument("elf")
    p.add_argument("name")
    p.add_argument("--overlay")
    p.add_argument("--tsv", action="store_true")
    p.add_argument("--raw", action="store_true", help="do not run the overlay's constructors")
    p.add_argument("--limit", type=int)
    p = sub.add_parser("list")
    p.add_argument("elf")
    p.add_argument("--overlay")
    p.add_argument("--min", type=int, default=256)
    p = sub.add_parser("dumpall")
    p.add_argument("elf")
    p.add_argument("dir")
    p.add_argument("--overlay")
    p.add_argument("--min", type=int, default=256)
    p.add_argument("--raw", action="store_true", help="do not run the overlay's constructors")
    args = parser.parse_args()

    elf = Elf(args.elf)
    dw = dwarf1.Dwarf(elf)
    # The DWARF side names overlays "gcmn.prg"; Program takes "gcmn".
    ov = args.overlay[:-4] if args.overlay and args.overlay.endswith(".prg") else args.overlay
    args.overlay = ov + ".prg" if ov else None
    if args.cmd in ("list", "dumpall"):
        rows = struct_arrays(dw, args.overlay, args.min)
        if args.cmd == "list":
            for size, qn, tname, n, where in rows:
                print(f"{size:8}  {where:12} {tname:24} {qn}[{n if n is not None else ''}]")
            return 0
        os.makedirs(args.dir, exist_ok=True)
        reader = Reader(args.elf, ov, args.raw)
        dec = Decoder(dw, reader)
        done = 0
        for _, qn, _, _, where in rows:
            # Only this image's own tables (list shows the others' too).
            if where != (args.overlay or "main"):
                continue
            safe = "".join(c if c.isalnum() or c in "_-." else "_" for c in qn)
            try:
                addr, t, w = find_global(dw, elf, qn, args.overlay)
            except SystemExit as e:
                with open(os.path.join(args.dir, safe + ".err"), "w") as f:
                    print(e, file=f)
                continue
            with open(os.path.join(args.dir, safe + ".tsv"), "w") as f, \
                    open(os.path.join(args.dir, safe + ".err"), "w") as err:
                try:
                    dump(dw, dec, reader, qn, addr, t, w, True, None, f, err)
                except Exception as e:  # noqa: BLE001 - a table the decoder cannot read
                    print(f"# {qn}: {e!r}", file=err)
            done += 1
        print(f"{done} tables to {args.dir}", file=sys.stderr)
        return 0

    addr, t, where = find_global(dw, elf, args.name, args.overlay)
    reader = Reader(args.elf, ov, args.raw)
    dec = Decoder(dw, reader)
    return dump(dw, dec, reader, args.name, addr, t, where, args.tsv, args.limit, sys.stdout, sys.stderr)


def struct_arrays(dw, overlay, min_size):
    """The global arrays of structs of at least `min_size` bytes, largest
    first: (size, name, element type, count, image)."""
    rows = []
    for (qn, addr, where, _), g in dw.globals(overlay).items():
        t = g["type"]
        if not t or t[0] != "array":
            continue
        elem = dwarf1.innermost(t)
        if not (elem[0] == "base" and isinstance(elem[2], dwarf1.Die)):
            continue
        size = dw.sizeof(t)
        if size is None or size >= min_size:
            rows.append((size or 0, qn, elem[1], t[1], where))
    return sorted(rows, key=lambda r: -r[0])


def dump(dw, dec, reader, name, addr, t, where, tsv, limit, out_f, err_f):
    """One global: an array as a row per element (a header of the columns
    first), anything else as `name = value` lines."""
    if t[0] != "array":
        out = {}
        dec.value(t, addr, "", out)
        for k, v in out.items():
            print(f"{k or name} = {v!r}", file=out_f)
        return 0
    elem = t[2]
    esize = dw.sizeof(elem)
    n = t[1]
    if n is None:
        sym = reader.prog.symbol_named(name)
        n = sym.size // esize if sym and esize else 0
    if limit:
        n = min(n, limit)
    rows = []
    for i in range(n):
        out = {"#": i}
        dec.value(elem, addr + i * esize, "", out)
        rows.append(out)
    cols = []
    for r in rows:
        for k in r:
            if k not in cols:
                cols.append(k)
    sep = "\t" if tsv else " | "
    print(sep.join(c or "value" for c in cols), file=out_f)
    for r in rows:
        print(sep.join("" if r.get(c) is None else str(r.get(c)) for c in cols), file=out_f)
    print(f"# {name}: {n} x {esize} bytes at 0x{addr:08x} ({where})", file=err_f)
    return 0


if __name__ == "__main__":
    sys.exit(main())
