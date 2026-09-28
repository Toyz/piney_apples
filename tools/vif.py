#!/usr/bin/env python3
"""PS2 VIF codes, source-chain DMA tags and GIFtags.

A VU program and the data it works on reach the VU through the VIF as a
stream of 32-bit VIF codes, each followed by its data; the stream itself is
usually carried by a DMA chain whose tags hold two VIF codes in their upper
64 bits (TTE on). What the VU sends on to the GS starts with a GIFtag.

    tools/vif.py stream ELF VA END [--chain] [--data]   decode a VIF stream in the ELF
    tools/vif.py hex    HEXBYTES [--data]                decode VIF codes from bytes
    tools/vif.py giftag HEXBYTES                         decode a 128-bit GIFtag
    tools/vif.py dmatag HEXBYTES                         decode a 64-bit DMA tag

`stream` walks VIF codes between two addresses; with --chain it follows a
source DMA chain from VA instead (CNT/NEXT/REF/REFE/END) and decodes the VIF
codes carried in the tags and in the data they point at. Also a library:
`codes(data)` yields `VifCode`s with their data, `chain(read, va)` walks a
DMA chain, `giftag(q)` and `dmatag(d)` decode tags. Byte order is little-endian
throughout; the quantities are as the EE User's Manual and the GS User's
Manual define them.
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------------------
# VIF codes

CMD = {
    0x00: "NOP", 0x01: "STCYCL", 0x02: "OFFSET", 0x03: "BASE", 0x04: "ITOP",
    0x05: "STMOD", 0x06: "MSKPATH3", 0x07: "MARK", 0x10: "FLUSHE", 0x11: "FLUSH",
    0x13: "FLUSHA", 0x14: "MSCAL", 0x15: "MSCALF", 0x17: "MSCNT", 0x20: "STMASK",
    0x30: "STROW", 0x31: "STCOL", 0x4a: "MPG", 0x50: "DIRECT", 0x51: "DIRECTHL",
}
# UNPACK: cmd 0x60-0x7f = 011m vvll; vn+1 components of vl-sized elements.
UNPACK_NAMES = {(vn, vl): f"{('S', 'V2', 'V3', 'V4')[vn]}-{(32, 16, 8, 5)[vl]}"
                for vn in range(4) for vl in range(4) if vl < 3 or vn == 3}
STMOD = ("normal", "offset", "difference", "mode3")


def unpack_size(vn, vl):
    """Bytes of packed data per vector for an UNPACK format."""
    if vl == 3:
        return 2  # V4-5: one halfword holds 5:5:5:1
    return (vn + 1) * (4 >> vl)


class VifCode:
    """One VIF code and the data words that belong to it.

    `data` is the payload (MPG instructions, UNPACK source, STROW words...),
    `size` the bytes the code occupies in the stream including padding, `off`
    its offset in the stream. For UNPACK, `vn`, `vl`, `usn`, `flg`, `mask`,
    `addr` and `num` (vectors written) are filled in.
    """
    __slots__ = ("off", "word", "cmd", "num", "imm", "irq", "data", "size",
                 "vn", "vl", "usn", "flg", "mask", "addr")

    def __init__(self, off, word):
        self.off = off
        self.word = word
        self.imm = word & 0xffff
        self.num = (word >> 16) & 0xff
        self.cmd = (word >> 24) & 0x7f
        self.irq = bool(word >> 31)
        self.data = b""
        self.size = 4
        self.vn = self.vl = self.usn = self.flg = self.mask = self.addr = None

    @property
    def is_unpack(self):
        return self.cmd >= 0x60

    @property
    def name(self):
        if self.is_unpack:
            return "UNPACK"
        return CMD.get(self.cmd, f"cmd{self.cmd:02x}")

    @property
    def text(self):
        c, i = self.cmd, self.imm
        irq = " [i]" if self.irq else ""
        if self.is_unpack:
            fmt = UNPACK_NAMES.get((self.vn, self.vl), f"vn{self.vn}vl{self.vl}")
            bits = [fmt]
            if self.usn:
                bits.append("usn")
            if self.mask:
                bits.append("mask")
            if self.flg:
                bits.append("flg")
            return (f"UNPACK {' '.join(bits)} addr=0x{self.addr:03x} num={self.num}"
                    f" ({len(self.data)} bytes){irq}")
        if c == 0x01:
            return f"STCYCL cl={i & 0xff} wl={i >> 8}{irq}"
        if c in (0x02, 0x03, 0x04):
            return f"{self.name} 0x{i & 0x3ff:03x}{irq}"
        if c == 0x05:
            return f"STMOD {STMOD[i & 3]}{irq}"
        if c == 0x06:
            return f"MSKPATH3 {'mask' if i & 0x8000 else 'unmask'}{irq}"
        if c == 0x07:
            return f"MARK 0x{i:04x}{irq}"
        if c in (0x14, 0x15):
            return f"{self.name} 0x{i:04x}{irq}"
        if c == 0x20:
            m = struct.unpack("<I", self.data)[0]
            return f"STMASK 0x{m:08x} {mask_text(m)}{irq}"
        if c in (0x30, 0x31):
            w = struct.unpack("<4I", self.data)
            return f"{self.name} " + " ".join(f"0x{x:08x}" for x in w) + irq
        if c == 0x4a:
            n = self.num or 256
            return f"MPG {n} instructions at 0x{i:03x}{irq}"
        if c in (0x50, 0x51):
            return f"{self.name} {i or 65536} qwords{irq}"
        return self.name + irq


def mask_text(m):
    """STMASK as 16 two-bit fields, row by row (x y z w per write cycle):
    0 data, 1 row, 2 col, 3 write-protect."""
    rows = []
    for r in range(4):
        rows.append("".join("drcp"[(m >> (r * 8 + c * 2)) & 3] for c in range(4)))
    return "/".join(rows)


def codes(data, off=0, end=None, cycle=(1, 1)):
    """Yield VifCodes from `data[off:end]`. `cycle` is the STCYCL (cl, wl)
    in force at the start; it is tracked, because it sets how many data
    vectors an UNPACK consumes."""
    end = len(data) if end is None else end
    cl, wl = cycle
    while off + 4 <= end:
        word = struct.unpack_from("<I", data, off)[0]
        v = VifCode(off, word)
        n = 0
        c = v.cmd
        if c == 0x01:
            cl, wl = v.imm & 0xff, v.imm >> 8
        elif c == 0x20:
            n = 4
        elif c in (0x30, 0x31):
            n = 16
        elif c == 0x4a:
            # The instructions must start on a 64-bit boundary.
            pad = (-(off + 4)) % 8
            n = pad + (v.num or 256) * 8
            v.data = data[off + 4 + pad:off + 4 + n]
        elif c in (0x50, 0x51):
            pad = (-(off + 4)) % 16
            n = pad + (v.imm or 65536) * 16
            v.data = data[off + 4 + pad:off + 4 + n]
        elif v.is_unpack:
            v.vn, v.vl = (c >> 2) & 3, c & 3
            v.mask = bool(c & 0x10)
            v.addr = v.imm & 0x3ff
            v.usn = bool(v.imm & 0x4000)
            v.flg = bool(v.imm & 0x8000)
            num = v.num or 256
            if wl <= cl:
                vecs = num
            else:  # filling: only cl of every wl written vectors come from data
                vecs = (num // wl) * cl + min(num % wl, cl)
            n = (vecs * unpack_size(v.vn, v.vl) + 3) & ~3
            v.data = data[off + 4:off + 4 + n]
        if c in (0x20, 0x30, 0x31):
            v.data = data[off + 4:off + 4 + n]
        v.size = 4 + n
        yield v
        off += v.size


def unpack(v, row=(0, 0, 0, 0), mode=0):
    """The vectors an UNPACK writes, as lists of four 32-bit words, before
    masking. Components the format does not supply are left None (the VIF
    fills them from the mask/col registers, or keeps memory). `mode` is the
    STMOD: 1 adds the row register."""
    size = unpack_size(v.vn, v.vl)
    out = []
    for i in range(len(v.data) // size):
        chunk = v.data[i * size:(i + 1) * size]
        if v.vl == 3:
            h = struct.unpack("<H", chunk)[0]
            vec = [(h & 31) << 3, ((h >> 5) & 31) << 3, ((h >> 10) & 31) << 3, (h >> 15) << 7]
        else:
            bits = (32, 16, 8)[v.vl]
            fmt = {32: "i", 16: "h", 8: "b"}[bits]
            if v.usn:
                fmt = fmt.upper()
            vals = list(struct.unpack("<" + fmt * (v.vn + 1), chunk))
            vals = [x & 0xffffffff for x in vals]
            if v.vn == 0:  # S-xx is broadcast to all four fields
                vec = vals * 4
            else:
                vec = vals + [None] * (3 - v.vn)
        if mode == 1:
            vec = [None if x is None else (x + r) & 0xffffffff for x, r in zip(vec, row)]
        out.append(vec)
    return out


# ---------------------------------------------------------------------------
# DMA tags (source chain mode)

DMA_ID = ("REFE", "CNT", "NEXT", "REF", "REFS", "CALL", "RET", "END")


class DmaTag:
    __slots__ = ("qwc", "pce", "id", "irq", "addr", "spr", "vif")

    def __init__(self, d, vif=None):
        lo = struct.unpack_from("<Q", d)[0]
        self.qwc = lo & 0xffff
        self.pce = (lo >> 26) & 3
        self.id = (lo >> 28) & 7
        self.irq = bool((lo >> 31) & 1)
        self.addr = (lo >> 32) & 0x7fffffff
        self.spr = bool(lo >> 63)
        self.vif = vif  # the two words in the upper half, when TTE carries them

    @property
    def text(self):
        s = f"{DMA_ID[self.id]} qwc={self.qwc}"
        if self.id not in (1, 7) or self.addr:
            s += f" addr=0x{self.addr:08x}"
        if self.irq:
            s += " irq"
        if self.spr:
            s += " spr"
        return s


def dmatag(d):
    return DmaTag(d[:8], d[8:16] if len(d) >= 16 else None)


def chain(read, va, limit=4096):
    """Walk a source DMA chain from `va` as the DMAC would with TTE set.
    Yields (tag_va, DmaTag, segments), where segments are (va, bytes) in
    transfer order: the tag's two VIF words, then the data it moves.
    CALL/RET use a two-deep stack as the hardware does."""
    stack = []
    for _ in range(limit):
        tag = dmatag(read(va, 16))
        segs = [(va + 8, tag.vif)]
        n = tag.qwc * 16
        nxt = None
        if tag.id == 1:            # CNT: data follows, then the next tag
            segs.append((va + 16, read(va + 16, n)))
            nxt = va + 16 + n
        elif tag.id == 2:          # NEXT: data follows, tag at addr
            segs.append((va + 16, read(va + 16, n)))
            nxt = tag.addr
        elif tag.id in (0, 3, 4):  # REFE/REF/REFS: data at addr
            segs.append((tag.addr, read(tag.addr, n)))
            nxt = None if tag.id == 0 else va + 16
        elif tag.id == 5:          # CALL
            segs.append((va + 16, read(va + 16, n)))
            stack.append(va + 16 + n)
            nxt = tag.addr
        elif tag.id == 6:          # RET
            segs.append((va + 16, read(va + 16, n)))
            nxt = stack.pop() if stack else None
        elif tag.id == 7:          # END
            segs.append((va + 16, read(va + 16, n)))
        yield va, tag, segs
        if nxt is None:
            return
        va = nxt


class Stream:
    """A byte stream stitched from (va, bytes) segments, remembering where
    every byte came from."""

    def __init__(self, segments):
        self.parts = []
        buf = bytearray()
        for va, b in segments:
            self.parts.append((len(buf), va, len(b)))
            buf += b
        self.data = bytes(buf)

    def va(self, off):
        for start, va, n in self.parts:
            if start <= off < start + n:
                return va + off - start
        return None


# ---------------------------------------------------------------------------
# GIFtags

GIF_FLG = ("PACKED", "REGLIST", "IMAGE", "DISABLE")
GIF_REG = ("PRIM", "RGBAQ", "ST", "UV", "XYZF2", "XYZ2", "TEX0_1", "TEX0_2",
           "CLAMP_1", "CLAMP_2", "FOG", "RESERVED", "XYZF3", "XYZ3", "A+D", "NOP")
PRIM_TYPE = ("POINT", "LINE", "LINESTRIP", "TRIANGLE", "TRISTRIP", "TRIFAN",
             "SPRITE", "prim7")


def prim_text(p):
    bits = [PRIM_TYPE[p & 7]]
    for bit, name in ((3, "IIP"), (4, "TME"), (5, "FGE"), (6, "ABE"), (7, "AA1"),
                      (8, "FST"), (9, "CTXT"), (10, "FIX")):
        if p >> bit & 1:
            bits.append(name)
    return " ".join(bits)


class GifTag:
    __slots__ = ("nloop", "eop", "pre", "prim", "flg", "nreg", "regs")

    def __init__(self, q):
        lo, hi = struct.unpack_from("<QQ", q)
        self.nloop = lo & 0x7fff
        self.eop = bool((lo >> 15) & 1)
        self.pre = bool((lo >> 46) & 1)
        self.prim = (lo >> 47) & 0x7ff
        self.flg = (lo >> 58) & 3
        self.nreg = (lo >> 60) or 16
        self.regs = [(hi >> (4 * i)) & 15 for i in range(self.nreg)]

    @property
    def text(self):
        s = f"GIFtag nloop={self.nloop}{' eop' if self.eop else ''} {GIF_FLG[self.flg]}"
        if self.pre:
            s += f" prim=0x{self.prim:03x} ({prim_text(self.prim)})"
        if self.flg != 2:
            s += " regs=" + ",".join(GIF_REG[r] for r in self.regs)
        return s


def giftag(q):
    return GifTag(q)


# ---------------------------------------------------------------------------
# listing

def _hexwords(b, limit=8):
    w = [f"{x:08x}" for x in struct.unpack_from(f"<{len(b) // 4}I", b)]
    more = f" ... ({len(w)} words)" if len(w) > limit else ""
    return " ".join(w[:limit]) + more


def listing(data, base_va=None, show_data=False, va_of=None):
    """Text lines for every VIF code in `data`."""
    out = []
    for v in codes(data):
        va = va_of(v.off) if va_of else (base_va + v.off if base_va is not None else v.off)
        out.append(f"  {va:08x}  {v.word:08x}  {v.text}")
        if show_data and v.data and v.cmd not in (0x20, 0x30, 0x31):
            if v.is_unpack:
                for i, vec in enumerate(unpack(v)[:16]):
                    out.append("            " + f"[{i:3d}] " +
                               " ".join("    ----" if x is None else f"{x:08x}" for x in vec))
            else:
                out.append("            " + _hexwords(v.data))
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    num = lambda s: int(s, 0)  # noqa: E731
    p = sub.add_parser("stream")
    p.add_argument("elf")
    p.add_argument("va", type=num)
    p.add_argument("end", type=num, nargs="?")
    p.add_argument("--chain", action="store_true")
    p.add_argument("--data", action="store_true")
    p = sub.add_parser("hex")
    p.add_argument("hex")
    p.add_argument("--data", action="store_true")
    p = sub.add_parser("giftag")
    p.add_argument("hex")
    p = sub.add_parser("dmatag")
    p.add_argument("hex")
    args = parser.parse_args()

    if args.cmd == "stream":
        from elf import Elf
        elf = Elf(args.elf)
        if args.chain:
            segs = []
            for tva, tag, s in chain(elf.read, args.va):
                print(f"  {tva:08x}  DMAtag {tag.text}")
                segs += s
            st = Stream(segs)
            print("\n".join(listing(st.data, show_data=args.data, va_of=st.va)))
        else:
            if args.end is None:
                raise SystemExit("stream needs END without --chain")
            data = elf.read(args.va, args.end - args.va)
            print("\n".join(listing(data, args.va, args.data)))
    elif args.cmd == "hex":
        data = bytes.fromhex(args.hex.replace(" ", ""))
        print("\n".join(listing(data, 0, args.data)))
    elif args.cmd == "giftag":
        print(giftag(bytes.fromhex(args.hex.replace(" ", ""))).text)
    elif args.cmd == "dmatag":
        d = bytes.fromhex(args.hex.replace(" ", ""))
        t = dmatag(d)
        print(t.text)
        if t.vif:
            print("\n".join(listing(t.vif, 0)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
