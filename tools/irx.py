#!/usr/bin/env python3
"""IOP modules (.IRX): the relocatable R3000 ELFs the PS2's I/O processor
loads - the sound driver, the synthesizer, the CD and pad drivers.

    tools/irx.py info  IRX                 sections, imports, exports
    tools/irx.py fn    IRX NAME_OR_ADDR    one function (a symbol, an export,
                                           or an address; stripped modules are
                                           cut at the next function start)
    tools/irx.py range IRX START END       a range of code
    tools/irx.py data  IRX START END       words, with string and symbol notes
    tools/irx.py funcs IRX                 function starts, named where known

Addresses are module-relative: an IRX is linked at 0 and relocated where the
IOP loads it, so its code as stored already holds base-0 addresses in its
j/jal targets and lui/addiu pairs.

Imports are stub tables in .text: the word 0x41e00000, a zero link word, a
u16 version, a u16 of flags, an 8-byte library name, then per function
`jr $ra` and `addiu $zero, $zero, N` (N the library's export number), ending
in two zero words. Exports are one table: 0x41c00000, zero, version, flags,
name, then a word per function, ending in zero; the first four are the
library's standard entries (start, stop, reserved, reserved).

Stripped modules (MODHSYN.IRX, MODMIDI.IRX, LIBSD.IRX) get their exports
named from the import stubs of modules that do carry symbols (SNDBASE.IRX and
SEWORDS.IRX name their stubs `sceHSyn_Load` and so on), so each export
number gets the name its callers use.
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import elf as elfmod  # noqa: E402
import mips  # noqa: E402

# Export numbers of the system libraries this disc's modules import, for the
# stubs of modules that carry no names of their own. libsd's are this
# version's (SEWORDS.IRX and SNDBASE.IRX name 4, 5, 11, 18, 20 and 26; the
# rest follow the library's order with no sceSdQuit at 4).
KNOWN = {
    "libsd": {4: "sceSdInit", 5: "sceSdSetParam", 6: "sceSdGetParam", 7: "sceSdSetSwitch",
              8: "sceSdGetSwitch", 9: "sceSdSetAddr", 10: "sceSdGetAddr",
              11: "sceSdSetCoreAttr", 12: "sceSdGetCoreAttr", 13: "sceSdNote2Pitch",
              14: "sceSdPitch2Note", 15: "sceSdProcBatch", 16: "sceSdProcBatchEx",
              17: "sceSdVoiceTrans", 18: "sceSdBlockTrans", 19: "sceSdVoiceTransStatus",
              20: "sceSdBlockTransStatus", 21: "sceSdSetTransCallback",
              22: "sceSdSetIRQCallback", 23: "sceSdSetEffectAttr", 24: "sceSdGetEffectAttr",
              25: "sceSdClearEffectWorkArea", 26: "sceSdSetTransIntrHandler",
              27: "sceSdSetSpu2IntrHandler"},
    "intrman": {17: "CpuSuspendIntr", 18: "CpuResumeIntr"},
    "loadcore": {6: "RegisterLibraryEntries"},
    "stdio": {4: "printf"},
    "thbase": {4: "CreateThread", 6: "StartThread", 20: "GetThreadId", 24: "SleepThread",
               26: "iWakeupThread", 33: "DelayThread", 34: "GetSystemTime",
               39: "USec2SysClock", 40: "SysClock2USec"},
    "sysclib": {8: "look_ctype_table", 12: "memcpy", 14: "memset", 20: "strcat",
                23: "strcpy", 27: "strlen", 29: "strncmp", 36: "strtol"},
}

IMPORT_MAGIC = 0x41E00000
EXPORT_MAGIC = 0x41C00000
JR_RA = 0x03E00008


class Irx:
    def __init__(self, path):
        self.path = path
        self.elf = e = elfmod.Elf(path)
        load = [s for s in e.segments if s.type == 1]
        size = max(s.vaddr + s.memsz for s in load)
        mem = bytearray(size)
        for s in load:
            mem[s.vaddr:s.vaddr + s.filesz] = e.data[s.offset:s.offset + s.filesz]
        self.mem = bytes(mem)
        text = next(s for s in e.sections if s.name == ".text")
        self.text = (text.addr, text.addr + text.size)
        self.names = {}         # addr -> name
        self.objects = {}       # addr -> size, for data symbols
        for s in e.symbols:
            if s.name and s.type in (1, 2) and s.shndx not in (0,) and s.shndx < 0xff00:
                self.names.setdefault(s.value, s.name)
                if s.type == 1:
                    self.objects[s.value] = s.size
            elif s.name and s.type == 0 and s.shndx and s.shndx < 0xff00 \
                    and not s.name.startswith("_") and s.name not in ("etext", "edata", "end"):
                self.names.setdefault(s.value, s.name)
        self.funcs = {s.value: s.size for s in e.symbols if s.type == 2 and s.size}
        self.imports = self._imports()      # [(lib, [(index, stub addr, name or None)])]
        self.exports = self._exports()      # (lib, [addr])
        for lib, stubs in self.imports:
            for idx, addr, name in stubs:
                self.names.setdefault(addr, name or f"{lib}_{idx}")

    def u32(self, a):
        return struct.unpack_from("<I", self.mem, a)[0]

    def cstr(self, a, limit=200):
        end = self.mem.find(b"\0", a, a + limit)
        if end < 0:
            return None
        return self.mem[a:end]

    def _imports(self):
        out = []
        a0, a1 = self.text
        a = a0
        while a < a1:
            if self.u32(a) == IMPORT_MAGIC and self.u32(a + 4) == 0:
                lib = self.mem[a + 12:a + 20].rstrip(b"\0").decode("latin-1")
                stubs = []
                p = a + 20
                while p + 8 <= a1:
                    w0, w1 = self.u32(p), self.u32(p + 4)
                    if w0 == 0 and w1 == 0:
                        break
                    if w0 != JR_RA:
                        break
                    idx = w1 & 0xFFFF
                    stubs.append((idx, p, KNOWN.get(lib, {}).get(idx)))
                    p += 8
                out.append((lib, stubs))
                a = p
            a += 4
        return out

    def _exports(self):
        a0, a1 = self.text
        for a in range(a0, len(self.mem) - 20, 4):
            if self.u32(a) == EXPORT_MAGIC and self.u32(a + 4) == 0:
                lib = self.mem[a + 12:a + 20].rstrip(b"\0").decode("latin-1")
                fns = []
                p = a + 20
                # The first four are the standard entries, and the first of
                # them may be zero; the table ends at the next zero.
                while p + 4 <= len(self.mem) and (self.u32(p) or len(fns) < 4):
                    fns.append(self.u32(p))
                    p += 4
                return lib, fns
        return None

    def name_exports(self, others):
        """Name this module's exports from the import stubs of `others`."""
        if not self.exports:
            return
        lib, fns = self.exports
        for o in others:
            for olib, stubs in o.imports:
                if olib != lib:
                    continue
                for idx, addr, _ in stubs:
                    nm = o.names.get(addr)
                    if nm and idx < len(fns) and not nm.startswith(olib + "_"):
                        self.names.setdefault(fns[idx], nm)
        for i, f in enumerate(fns):
            self.names.setdefault(f, f"{lib}_export{i}")

    # -- code ---------------------------------------------------------------

    def starts(self):
        """Function starts: symbols, exports, jal targets, and code after a
        jr $ra's delay slot that opens with a stack adjustment."""
        s = set(self.funcs)
        if self.exports:
            s.update(f for f in self.exports[1] if self.text[0] <= f < self.text[1])
        a0, a1 = self.text
        for a in range(a0, a1, 4):
            ins = mips.decode(self.u32(a), a)
            if ins.call and ins.target is not None and a0 <= ins.target < a1:
                s.add(ins.target)
        for lib, stubs in self.imports:
            for _, addr, _ in stubs:
                s.add(addr)
        return sorted(s)

    def end_of(self, start):
        if start in self.funcs:
            return start + self.funcs[start]
        later = [x for x in self.starts() if x > start]
        return later[0] if later else self.text[1]

    def listing(self, start, end):
        insns = [mips.decode(self.u32(a), a) for a in range(start, end, 4)]
        tracked = mips.track_lui(insns, None)
        labels = {i.target for i in insns if i.target is not None and not i.call
                  and start <= i.target < end}
        lines = []
        for ins in insns:
            if ins.pc in self.names and ins.pc != start:
                lines.append(f"{self.names[ins.pc]}:")
            if ins.pc in labels:
                lines.append(f".L{ins.pc:05x}:")
            ops = list(ins.operands)
            if ins.target is not None and ins.valid:
                t = ins.target
                if not ins.call and start <= t < end:
                    ops[-1] = f".L{t:05x}"
                elif t in self.names:
                    ops[-1] = self.names[t]
            text = f"{ins.mnemonic:<7} {', '.join(ops)}" if ops else ins.mnemonic
            note = None
            if ins.pc in tracked:
                note = self.describe(tracked[ins.pc])
            line = f"  {ins.pc:05x}  {ins.word:08x}  {text}"
            if note:
                line = f"{line:<60} # {note}"
            lines.append(line)
        return lines

    def describe(self, addr):
        parts = [f"0x{addr:05x}"]
        nm = self.names.get(addr)
        if nm is None:
            base = max((a for a in self.names if a <= addr and addr - a < 0x2000
                        and a in self.objects and addr < a + max(self.objects[a], 1)),
                       default=None)
            if base is not None:
                nm = f"{self.names[base]}+0x{addr - base:x}"
        if nm:
            parts.append(nm)
        if 0 <= addr < len(self.mem):
            s = self.cstr(addr, 80)
            if s and len(s) >= 3 and all(32 <= c < 127 or c in (9, 10) for c in s):
                parts.append('"' + s.decode("latin-1").replace("\n", "\\n") + '"')
        return " ".join(parts)

    def lookup(self, what):
        try:
            return int(what, 0)
        except ValueError:
            pass
        for a, n in self.names.items():
            if n == what:
                return a
        raise KeyError(what)


def load(path):
    """An Irx with its exports named from the other modules beside it."""
    m = Irx(path)
    if m.exports:
        others = []
        d = os.path.dirname(os.path.abspath(path))
        for f in sorted(os.listdir(d)):
            if f.upper().endswith(".IRX") and os.path.join(d, f) != os.path.abspath(path):
                try:
                    others.append(Irx(os.path.join(d, f)))
                except (ValueError, StopIteration, struct.error):
                    pass
        m.name_exports(others)
    return m


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("info")
    p.add_argument("irx")
    p = sub.add_parser("fn")
    p.add_argument("irx")
    p.add_argument("what")
    p = sub.add_parser("range")
    p.add_argument("irx")
    p.add_argument("start", type=lambda s: int(s, 0))
    p.add_argument("end", type=lambda s: int(s, 0))
    p = sub.add_parser("data")
    p.add_argument("irx")
    p.add_argument("start", type=lambda s: int(s, 0))
    p.add_argument("end", type=lambda s: int(s, 0))
    p = sub.add_parser("funcs")
    p.add_argument("irx")
    args = parser.parse_args()

    m = load(args.irx)
    if args.cmd == "info":
        for s in m.elf.sections:
            if s.name:
                print(f"{s.name:12s} 0x{s.addr:05x} {s.size:6d}")
        for lib, stubs in m.imports:
            print(f"import {lib}: " + ", ".join(f"{i}={m.names.get(a)}" for i, a, _ in stubs))
        if m.exports:
            lib, fns = m.exports
            print(f"export {lib}:")
            for i, f in enumerate(fns):
                print(f"  {i:3d} 0x{f:05x} {m.names.get(f, '')}")
        return 0
    if args.cmd == "fn":
        a = m.lookup(args.what)
        print(f"# {m.names.get(a, hex(a))} 0x{a:05x}")
        for line in m.listing(a, m.end_of(a)):
            print(line)
        return 0
    if args.cmd == "range":
        for line in m.listing(args.start, args.end):
            print(line)
        return 0
    if args.cmd == "data":
        for a in range(args.start, args.end, 4):
            w = m.u32(a)
            note = m.describe(w) if w < len(m.mem) and w in m.names else ""
            lab = m.names.get(a)
            if lab:
                print(f"{lab}:")
            print(f"  {a:05x}  {w:08x}  {note}")
        return 0
    for a in m.starts():
        print(f"0x{a:05x} {m.names.get(a, '')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
