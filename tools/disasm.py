#!/usr/bin/env python3
"""EE disassembler over the ELF's symbols and relocations.

    tools/disasm.py fn    ELF NAME_OR_VA [--overlay NAME]     one function
    tools/disasm.py range ELF START END  [--overlay NAME] [--data]
    tools/disasm.py xrefs ELF NAME_OR_VA [--overlay NAME] [--hi] [--scan]
    tools/disasm.py find  ELF REGEX      [--overlay NAME]

--overlay gcmn|demo|desktop|toppage maps that .PRG at 0x00400800 and uses
its symbols there; main's always apply. --demangle prints C++ names
demangled (via tools/demangle.py, when present).

Listings label in-function branch targets, name call and jump targets, and
annotate address materialisation: first from the relocation on the
instruction (R_MIPS_HI16/LO16/GPREL16, exact even for section-relative
references), else from tracking lui through addiu/ori/loads/stores over the
function's control flow. `xrefs` reads the relocation tables, so it is exact;
--scan also decodes every function and reports lui/jal references with no
relocation behind them.
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mips  # noqa: E402
from image import Program  # noqa: E402
from elf import STT  # noqa: E402

_demangle = None


def set_demangle(on):
    """Route names through tools/demangle.py; False if it is not there."""
    global _demangle
    _demangle = None
    if not on:
        return True
    try:
        from demangle import demangle
    except ImportError:
        return False
    _demangle = demangle
    return True


def nice(name):
    if _demangle and name:
        base, plus, off = name.partition("+")
        try:
            return _demangle(base) + plus + off
        except Exception:
            return name
    return name


# ---------------------------------------------------------------------------
# listing

def _quote(s, limit=48):
    s = s if len(s) <= limit else s[:limit] + "..."
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") \
        .replace("\r", "\\r").replace("\t", "\\t") + '"'


def describe(prog, addr, name=None):
    """0xADDR name "string" for an address, as much as is known."""
    parts = [f"0x{addr:08x}"]
    name = name or prog.name_at(addr)
    if name:
        parts.append(nice(name))
    s = prog.string_at(addr)
    if s:
        parts.append(_quote(s))
    return " ".join(parts)


def _is_address(prog, addr):
    return prog.mapped(addr) or prog.symbol_at(addr) is not None


def decode_range(prog, start, end):
    return [mips.decode(prog.u32(va), va) for va in range(start, end, 4)]


def listing(prog, start, end, func=None, symbols=False):
    """Text lines for [start, end). `func` is the function symbol when the
    range is one function; `symbols` prints a label for every symbol start."""
    insns = decode_range(prog, start, end)
    tracked = mips.track_lui(insns, prog.gp)
    labels = {i.target for i in insns
              if i.target is not None and not i.call and start <= i.target < end}
    lines = []
    if func is not None:
        lines.append(f"# {nice(func.name)}  0x{func.value:08x}-0x{func.value + func.size:08x}"
                     f"  ({func.size} bytes, {prog.section_name(func.shndx)})")
        lines.append(f"{nice(func.name)}:")
    starts = {}
    if symbols:
        for s in prog.symbols:
            if start <= s.value < end:
                starts.setdefault(s.value, []).append(s.name)
    for ins in insns:
        if ins.pc in starts and not (func and ins.pc == func.value):
            for nm in starts[ins.pc]:
                lines.append(f"{nice(nm)}:")
        if ins.pc in labels:
            lines.append(f".L{ins.pc:08x}:")
        text, note = render(prog, ins, start, end, tracked)
        line = f"  {ins.pc:08x}  {ins.word:08x}  {text}"
        if note:
            line = f"{line:<64} # {note}"
        lines.append(line.rstrip())
    return lines


def render(prog, ins, start, end, tracked):
    """(instruction text, comment) with targets named and addresses resolved."""
    ops = list(ins.operands)
    note = None
    relocs = prog.relocs_at(ins.pc)
    if ins.target is not None and ins.valid:
        t = ins.target
        name = None
        for r in relocs:
            if r.type == 4:
                name = prog.reloc_target_name(r)
        if not ins.call and start <= t < end:
            name = f".L{t:08x}"
        elif name is None:
            name = prog.name_at(t)
        if name:
            ops[-1] = nice(name) if not name.startswith(".L") else name
    for r in relocs:
        # Dataflow beats the static HI16/LO16 pairing when it has an answer.
        addr = tracked.get(ins.pc, r.addr) if r.type == 6 else r.addr
        if prog.foreign(r.symbol):
            note = f"{nice(r.symbol.name)} ({prog.section_name(r.symbol.shndx)})"
        elif r.type in (5, 6, 7) and addr is not None:
            desc = describe(prog, addr, prog.reloc_target_name(r, addr))
            note = ("hi " + desc) if r.type == 5 else desc
        elif r.type == 2:
            note = "word -> " + describe(prog, r.addr, prog.reloc_target_name(r))
        elif r.type not in (4,):
            note = f"{r.type_name} {r.symbol.name}"
    if note is None and ins.pc in tracked:
        addr = tracked[ins.pc]
        if _is_address(prog, addr):
            note = describe(prog, addr)
    ins_text = f"{ins.mnemonic:<7} {', '.join(ops)}" if ops else ins.mnemonic
    return ins_text, note


def data_listing(prog, start, end):
    """Words with their pointer relocations, for data areas."""
    lines = []
    starts = {}
    for s in prog.symbols:
        if start <= s.value < end:
            starts.setdefault(s.value, []).append(s)
    for va in range(start, end, 4):
        for s in starts.get(va, ()):
            lines.append(f"{nice(s.name)}:  # {STT.get(s.type, s.type)} {s.size} bytes")
        w = prog.u32(va)
        note = None
        for r in prog.relocs_at(va):
            if r.type == 2 and prog.foreign(r.symbol):
                note = f"-> {nice(r.symbol.name)} ({prog.section_name(r.symbol.shndx)})"
            elif r.type == 2:
                note = "-> " + describe(prog, r.addr, prog.reloc_target_name(r))
        if note is None and (va in starts or prog.read(va - 1, 1) == b"\0"):
            s = prog.string_at(va, 4)  # only where a string could begin
            if s:
                note = _quote(s)
        line = f"  {va:08x}  {w:08x}  .word   0x{w:08x}"
        lines.append(f"{line:<64} # {note}" if note else line)
    return lines


# ---------------------------------------------------------------------------
# xrefs

KIND = {4: "call", 5: "hi", 6: "lo", 7: "gp", 2: "data"}


def xrefs(prog, target, size, hi=False):
    """Relocations that resolve into [target, target+size)."""
    out = []
    hi_end = target + max(size, 1)
    for r in prog.relocs:
        if r.addr is None or not (target <= r.addr < hi_end) or prog.foreign(r.symbol):
            continue
        if r.type == 5 and not hi:
            continue
        out.append(r)
    out.sort(key=lambda r: r.offset)
    return out


def scan_refs(prog, target, size):
    """Decode every function: jal/j into the target and lui pairs that
    materialise an address inside it. [(pc, kind, insn)]."""
    out = []
    hi_end = target + max(size, 1)
    for f in prog.functions():
        try:
            insns = decode_range(prog, f.value, f.value + f.size)
        except KeyError:
            continue
        for ins in insns:
            if ins.target is not None and ins.target == target and not ins.cond:
                out.append((ins.pc, "call" if ins.call else "jump", ins))
        for pc, addr in mips.track_lui(insns, prog.gp).items():
            if target <= addr < hi_end:
                ins = insns[(pc - f.value) // 4]
                out.append((pc, "gp" if ins.base == "$gp" else "lo", ins))
    return out


def resolve(prog, what):
    """NAME_OR_VA -> (address, symbol or None)."""
    try:
        va = int(what, 0)
    except ValueError:
        s = prog.symbol_named(what)
        if s is None:
            owner = [prog.section_name(x.shndx) for x in prog.elf.symbols if x.name == what]
            hint = f" (it is in {', '.join(sorted(set(owner)))}; pass --overlay)" if owner else ""
            raise SystemExit(f"no symbol {what!r}{hint}")
        return s.value, s
    f = prog.function_at(va)
    if f is not None:
        return va, f
    hit = prog.symbol_at(va)
    return va, (hit[0] if hit and hit[1] == 0 else None)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--overlay", help="gcmn, demo, desktop or toppage")
    common.add_argument("--demangle", action="store_true", help="print C++ names demangled")
    p = sub.add_parser("fn", parents=[common])
    p.add_argument("elf")
    p.add_argument("what", metavar="NAME_OR_VA")
    p = sub.add_parser("range", parents=[common])
    p.add_argument("elf")
    p.add_argument("start", type=lambda s: int(s, 0))
    p.add_argument("end", type=lambda s: int(s, 0))
    p.add_argument("--data", action="store_true", help="dump words, not instructions")
    p = sub.add_parser("xrefs", parents=[common])
    p.add_argument("elf")
    p.add_argument("what", metavar="NAME_OR_VA")
    p.add_argument("--hi", action="store_true", help="also list the lui (HI16) halves")
    p.add_argument("--scan", action="store_true",
                   help="also decode every function looking for unrelocated references")
    p = sub.add_parser("find", parents=[common])
    p.add_argument("elf")
    p.add_argument("regex")
    args = parser.parse_args()

    if args.demangle and not set_demangle(True):
        print("note: tools/demangle.py not found; names stay mangled", file=sys.stderr)
    try:
        prog = Program(args.elf, args.overlay)
        return run(prog, args)
    except (KeyError, ValueError) as e:
        print(f"error: {e.args[0] if e.args else e}", file=sys.stderr)
        return 1


def run(prog, args):
    if args.cmd == "fn":
        va, sym = resolve(prog, args.what)
        f = prog.function_at(va)
        if f is None:
            if sym is None:
                raise SystemExit(f"0x{va:08x} is not inside a sized function")
            f = sym
        end = f.value + f.size
        if not f.size:  # a bare label: run to the next symbol
            end = min((s.value for s in prog.symbols if s.value > f.value),
                      default=f.value + 4)
            end = min(end, f.value + 0x1000)
        print("\n".join(listing(prog, f.value, end, func=f)))
    elif args.cmd == "range":
        lines = (data_listing if args.data else listing)(prog, args.start, args.end,
                                                         **({} if args.data else {"symbols": True}))
        print("\n".join(lines))
    elif args.cmd == "xrefs":
        va, sym = resolve(prog, args.what)
        size = sym.size if sym is not None and sym.value == va else 1
        label = nice(sym.name) if sym is not None else f"0x{va:08x}"
        refs = xrefs(prog, va, size, hi=args.hi)
        print(f"# xrefs to {label} (0x{va:08x}, {size} bytes): {len(refs)} relocations")
        seen = set()
        for r in refs:
            seen.add(r.offset)
            where = prog.name_at(r.offset) or "?"
            kind = KIND.get(r.type, r.type_name)
            if r.type == 4 and not mips.decode(prog.u32(r.offset), r.offset).call:
                kind = "jump"
            if r.type == 2:
                text = f".word   0x{prog.u32(r.offset):08x}"
            else:
                ins = mips.decode(prog.u32(r.offset), r.offset)
                text, _ = render(prog, ins, 0, 0, {})
            off = r.addr - va
            into = f"  [+0x{off:x}]" if off else ""
            print(f"  0x{r.offset:08x}  {kind:<5} {nice(where):<44} {text}{into}")
        if args.scan:
            extra = [(pc, k, ins) for pc, k, ins in scan_refs(prog, va, size) if pc not in seen]
            print(f"# scan: {len(extra)} references without a relocation")
            for pc, kind, ins in extra:
                where = prog.name_at(pc) or "?"
                text, _ = render(prog, ins, 0, 0, {})
                print(f"  0x{pc:08x}  {kind:<5} {nice(where):<44} {text}")
    elif args.cmd == "find":
        pat = re.compile(args.regex)
        hits = []
        for s in prog.elf.symbols:
            if not s.name:
                continue
            shown = nice(s.name)
            if pat.search(s.name) or (shown != s.name and pat.search(shown)):
                hits.append((s.value, s, shown))
        hits.sort(key=lambda h: (h[0], h[1].name))
        for _, s, shown in hits:
            extra = f"  {shown}" if shown != s.name else ""
            print(f"0x{s.value:08x} {s.size:8d} {STT.get(s.type, s.type):7} "
                  f"{prog.section_name(s.shndx):12} {s.name}{extra}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
