#!/usr/bin/env python3
"""Every way a volume's EE code or data can reach an address range.

    tools/refscan.py VOLUME LO HI [--near N]

VOLUME is infection, mutation, outbreak or quarantine (its work/<volume>/
disc/ and image). The scan covers:

- code: the executable's main segment and the four overlays' text
  (GCMN, DEMO, DESKTOP, TOPPAGE .PRG) with their static initialisers (the
  tail of .data the header's constructor table names, which mwLoadOverlay
  runs after the load), every word read as an instruction.
  From each `lui` the registers holding an address are followed forward
  (`addiu`/`daddiu`/`ori` add to them, moves copy them, any other write or
  a call ends them) for 128 instructions; every load, store or address
  built within N bytes of [LO, HI) is listed, as is an address register
  added to an index (`addu` and the like: an array base, then accessed
  through `%lo` as the compiler does it), with its base up to 8 KB below
  the range, and one passed in `$a0`-`$a3` to a call (memset, memcpy, a
  file read). `$gp`-relative
  accesses are listed the same way from the executable's `$gp`.
- data: every aligned word of every file on the disc image (the archives
  `DATA/DATA.BIN` and `STREAM/*.BIN` member by member, inflated) equal to
  an address within N bytes of the range.

What it lists is what a reader then checks by hand: no access can reach
the range without building its address one of these ways.
"""

import argparse
import os
import struct
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gzarc  # noqa: E402
import iso  # noqa: E402
from image import Overlay, Program  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = {"infection": "SLUS_202.67", "mutation": "SLUS_205.62", "outbreak": "SLUS_205.63",
       "quarantine": "SLUS_205.64"}
WINDOW = 128
LOADS = {0x20: 1, 0x21: 2, 0x23: 4, 0x24: 1, 0x25: 2, 0x27: 4, 0x37: 8, 0x1E: 16, 0x31: 4, 0x35: 8,
         0x22: 4, 0x26: 4, 0x1A: 8, 0x1B: 8}
STORES = {0x28: 1, 0x29: 2, 0x2B: 4, 0x3F: 8, 0x1F: 16, 0x39: 4, 0x3D: 8, 0x2A: 4, 0x2E: 4, 0x2C: 8, 0x2D: 8}
CALLER_SAVED = set(range(1, 16)) | {24, 25, 31}


def sext(v):
    return v - 0x10000 if v & 0x8000 else v


def rtype_dest(w):
    """The register an R-type or SPECIAL2/MMI instruction writes, if any."""
    op, fn = w >> 26, w & 63
    if op == 0 and fn in (0x08, 0x0C, 0x0D, 0x0F, 0x18, 0x19, 0x1A, 0x1B, 0x11, 0x13):
        return None
    if op in (0, 0x1C):
        return (w >> 11) & 31
    return None


def scan_code(words, base, lo, hi, gp, label, out, far=0x2000):
    """`words` read as code from `base`; an indexed access is listed when
    its base is up to `far` bytes below the range."""
    near = lambda a, n=1: a + n > lo and a < hi  # noqa: E731
    for i, w in enumerate(words):
        op, rs, rt = w >> 26, (w >> 21) & 31, (w >> 16) & 31
        at = base + 4 * i
        if gp and rs == 28 and (op in LOADS or op in STORES or op == 0x09):
            ea = (gp + sext(w & 0xFFFF)) & 0xFFFFFFFF
            n = LOADS.get(op) or STORES.get(op) or 1
            if near(ea, n):
                kind = "load" if op in LOADS else "store" if op in STORES else "addr"
                out.append((label, at, f"$gp {kind}", ea))
        if op != 0x0F:
            continue
        known = {rt: (w & 0xFFFF) << 16}
        indexed = {}  # register: an address plus an index added at run time
        for j in range(i + 1, min(i + 1 + WINDOW, len(words))):
            x = words[j]
            xa = base + 4 * j
            xop, xrs, xrt = x >> 26, (x >> 21) & 31, (x >> 16) & 31
            imm = x & 0xFFFF
            if xop in (0x09, 0x19, 0x0D) and xrs in known:
                v = (known[xrs] | imm) if xop == 0x0D else (known[xrs] + sext(imm)) & 0xFFFFFFFF
                if near(v):
                    out.append((label, xa, "addr", v))
                known[xrt] = v
                indexed.pop(xrt, None)
                continue
            if xop in (0x09, 0x19) and xrs in indexed:
                indexed[xrt] = (indexed[xrs] + sext(imm)) & 0xFFFFFFFF
                known.pop(xrt, None)
                continue
            if (xop in LOADS or xop in STORES) and xrs in known:
                ea = (known[xrs] + sext(imm)) & 0xFFFFFFFF
                n = LOADS.get(xop) or STORES.get(xop)
                if near(ea, n):
                    out.append((label, xa, "store" if xop in STORES else "load", ea))
                if xop in LOADS:
                    known.pop(xrt, None)
                    indexed.pop(xrt, None)
                continue
            if (xop in LOADS or xop in STORES) and xrs in indexed:
                # `lui; addu index; lw %lo`: an array from this base, any
                # distance below the range (the reader bounds the index).
                b = (indexed[xrs] + sext(imm)) & 0xFFFFFFFF
                if lo - far <= b < hi:
                    out.append((label, xa, "indexed " + ("store" if xop in STORES else "load"), b))
                if xop in LOADS:
                    known.pop(xrt, None)
                    indexed.pop(xrt, None)
                continue
            d = rtype_dest(x)
            if d is not None:
                srcs = [r for r in (xrs, xrt) if r in known]
                fn = x & 63
                if xop == 0 and fn in (0x21, 0x2D, 0x25) and 0 in (xrs, xrt) and srcs:
                    known[d] = known[srcs[0]]          # a move
                    indexed.pop(d, None)
                    continue
                if xop == 0 and fn in (0x21, 0x2D, 0x25) and 0 in (xrs, xrt) and xrs in indexed:
                    indexed[d] = indexed[xrs]
                    known.pop(d, None)
                    continue
                if xop == 0 and fn in (0x21, 0x2D) and len(srcs) == 1:
                    other = xrt if srcs[0] == xrs else xrs
                    if other not in known:
                        indexed[d] = known[srcs[0]]
                        if lo - far <= known[srcs[0]] < hi:
                            out.append((label, xa, "indexed base", known[srcs[0]]))
                        known.pop(d, None)
                        continue
                for r in srcs:
                    if near(known[r]) and fn in (0x21, 0x23, 0x2D, 0x2F, 0x25):
                        out.append((label, xa, "indexed base", known[r]))
                known.pop(d, None)
                indexed.pop(d, None)
                continue
            if xop == 0x03 or (xop == 0 and (x & 63) == 0x09):   # a call
                for r in (4, 5, 6, 7):
                    if r in known and near(known[r]):
                        out.append((label, xa, f"arg $a{r - 4} to a call", known[r]))
                    if r in indexed and lo - far <= indexed[r] < hi:
                        out.append((label, xa, f"indexed arg $a{r - 4} to a call", indexed[r]))
                for r in list(known):
                    if r in CALLER_SAVED:
                        known.pop(r)
                for r in list(indexed):
                    if r in CALLER_SAVED:
                        indexed.pop(r)
                continue
            if xop in (0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x18, 0x19) or xop in LOADS:
                known.pop(xrt, None)
                indexed.pop(xrt, None)
            if not known and not indexed:
                break


def scan_words(data, lo, hi, label, out):
    for k in range(0, len(data) - 3, 4):
        (v,) = struct.unpack_from("<I", data, k)
        if lo <= v < hi:
            out.append((label, k, "data word", v))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("volume", choices=EXE)
    ap.add_argument("lo", type=lambda s: int(s, 0))
    ap.add_argument("hi", type=lambda s: int(s, 0))
    ap.add_argument("--near", type=lambda s: int(s, 0), default=0x400)
    a = ap.parse_args()
    lo, hi = a.lo - a.near, a.hi + a.near
    disc = os.path.join(ROOT, "work", a.volume, "disc")
    elf = os.path.join(disc, EXE[a.volume])
    out = []
    p = Program(elf)
    m = p.main_seg
    main_bytes = p.elf.read(m.vaddr, m.filesz)
    words = list(struct.unpack_from("<%dI" % (len(main_bytes) // 4), main_bytes))
    scan_code(words, m.vaddr, lo, hi, p.gp, "main", out)
    for name in ("GCMN", "DEMO", "DESKTOP", "TOPPAGE"):
        ov = Overlay(os.path.join(disc, "DATA", name + ".PRG"))
        text = ov.read(ov.text, ov.text_size)
        scan_code(list(struct.unpack_from("<%dI" % (len(text) // 4), text)), ov.text, lo, hi, p.gp, name, out)
        # The static initialisers mwLoadOverlay runs sit at the tail of .data,
        # from the lowest the constructor table names to the table itself.
        ctors = [struct.unpack("<I", ov.read(a, 4))[0] for a in range(ov.ctor_start, ov.ctor_end, 4)]
        if ctors:
            init = ov.read(min(ctors), ov.ctor_start - min(ctors))
            scan_code(list(struct.unpack_from("<%dI" % (len(init) // 4), init)), min(ctors), lo, hi, p.gp,
                      name + " ctors", out)
    img = iso.Iso(os.path.join(ROOT, "work", a.volume, a.volume + ".iso"))
    files = 0
    for e in img.files():
        data = img.read(e.lba, e.size)
        files += 1
        scan_words(data, lo, hi, e.path, out)
        if e.path.upper().endswith(".BIN") and ("DATA.BIN" in e.path.upper() or "STREAM" in e.path.upper()):
            for mb in gzarc.members(data):
                try:
                    inner = gzarc.inflate(data, mb)
                except (zlib.error, OSError, EOFError):
                    continue
                files += 1
                scan_words(inner, lo, hi, f"{e.path}::{mb.name}", out)
    for label, at, kind, v in out:
        f = p.function_at(at) if label == "main" else None
        print(f"{label:24s} 0x{at:08x}  {kind:22s} 0x{v:08x}  {f.name if f else ''}")
    print(f"# {a.volume}: code main + 4 overlays, {files} files and members; {len(out)} hits "
          f"within 0x{a.near:x} of [0x{a.lo:08x}, 0x{a.hi:08x})")


if __name__ == "__main__":
    main()
