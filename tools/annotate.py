#!/usr/bin/env python3
"""A function's listing with its object's members named.

    tools/annotate.py ELF FUNC CLASS [--overlay NAME] [--reg REG ...]

Disassembles FUNC as tools/disasm.py does and follows the registers that
hold `this` (a CLASS pointer): $a0 on entry (or the --reg ones), what
`move` copies it to, and what a stack slot it was saved to loads back.
Every load or store off such a register gets a comment naming the member
from the DWARF layout (tools/dwarf1.py), base classes and struct members
flattened (`param.real.atk`). Offsets past 0x8000 go through `lui $at, K;
addu $at, this, $at`, which is followed too. A load of the vtable pointer
(the member the class's constructor stores its vtable to) and the
`lw $t9, N($t9)` after it are named by the vtable's slot N.

It follows registers in listing order, not along the control flow: a
register reused for something else in one branch can be named wrongly in
the other. The comments are an aid for reading, not a decompilation.
"""

import argparse
import functools
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

MEMBER = re.compile(r"/\* 0x([0-9a-f]+) \*/ (?:// base (\w+) \(0x([0-9a-f]+) bytes\)|(.+?) (\*{0,2})(\w+)((?:\[\d+\])*);\s*// 0x([0-9a-f]+))")
VSLOT = re.compile(r"//\s+\+0x([0-9a-f]+) 0x[0-9a-f]+ (.+?) \[")
INSN = re.compile(r"^\s+([0-9a-f]{8})\s+[0-9a-f]{8}\s+(\S+)\s*(.*?)(\s+#.*)?$")
MEM = re.compile(r"^(\$\w+), (-?\d+)\((\$\w+)\)$")


def run(*args):
    return subprocess.run([sys.executable, *args], capture_output=True, text=True, check=False).stdout


@functools.lru_cache(maxsize=None)
def layout(elf, name):
    """(members [(offset, size, name, type)], vtable slots {offset: name},
    size) of a DWARF struct or class; empty when it has no layout."""
    text = run(os.path.join(HERE, "dwarf1.py"), "type", elf, name)
    members, slots = [], {}
    for line in text.splitlines():
        m = MEMBER.search(line)
        if m:
            off = int(m.group(1), 16)
            if m.group(2):
                members.append((off, int(m.group(3), 16), "", m.group(2)))
            else:
                ty = m.group(4).strip()
                ptr = m.group(5)
                members.append((off, int(m.group(8), 16), m.group(6) + m.group(7), ty + (" *" if ptr else "")))
        v = VSLOT.search(line)
        if v:
            slots[int(v.group(1), 16)] = v.group(2)
    return members, slots


def name_at(elf, cls, off, depth=0):
    """The member of `cls` at byte `off`: `a.b[2]+4`, or None."""
    members, _ = layout(elf, cls)
    for moff, size, name, ty in members:
        if moff <= off < moff + max(size, 1):
            rest = off - moff
            if not name:  # a base class
                inner = name_at(elf, ty, rest, depth + 1) if depth < 6 else None
                return inner if inner is not None else f"<{ty}>+{rest:#x}"
            dims = [int(d) for d in re.findall(r"\[(\d+)\]", name)]
            base = name.split("[")[0]
            if dims:
                count = 1
                for d in dims:
                    count *= d
                elem = size // count if count else size
                idx, rest = divmod(rest, elem) if elem else (0, rest)
                label = f"{base}[{idx}]"
                if rest and not ty.endswith("*") and depth < 6:
                    inner = name_at(elf, ty.split()[0], rest, depth + 1)
                    if inner is not None:
                        return f"{label}.{inner}"
                return label + (f"+{rest}" if rest else "")
            if rest and not ty.endswith("*") and depth < 6:
                inner = name_at(elf, ty.split()[0], rest, depth + 1)
                if inner is not None:
                    return f"{name}.{inner}"
            return name + (f"+{rest}" if rest else "")
    return None


def vtable_member(elf, cls):
    """The offset of the vtable pointer (a member named __vt or the
    constructor's store) and the slot names."""
    _, slots = layout(elf, cls)
    return slots


def annotate(elf, fn, cls, overlay, regs):
    args = [os.path.join(HERE, "disasm.py"), "fn", elf, fn, "--demangle"]
    if overlay:
        args += ["--overlay", overlay]
    lines = run(*args).splitlines()
    this = set(regs or ["$a0"])
    at_this = False  # $at holds this + (K << 16)
    at_hi = 0
    saved = set()  # stack offsets holding this
    vt_regs = set()  # registers holding the vtable pointer
    slots = vtable_member(elf, cls)
    out = []
    for line in lines:
        m = INSN.match(line)
        if not m:
            out.append(line)
            continue
        op, ops = m.group(2), m.group(3)
        note = ""
        parts = [p.strip() for p in ops.split(",")]
        mm = MEM.match(ops)
        if mm:
            reg, off, base = mm.group(1), int(mm.group(2)), mm.group(3)
            if base in this or (base == "$at" and at_this):
                total = off + (at_hi if base == "$at" else 0)
                nm = name_at(elf, cls, total)
                note = f"this->{nm}" if nm else f"this+{total:#x}"
                if op.startswith(("lw", "ld", "lq")) and nm is None and op == "lw":
                    pass
                if op == "lw" and reg == "$t9":
                    vt_regs.add(reg)
                    note += " (vtable)"
            elif base in vt_regs and op == "lw":
                note = f"vtable slot {off:#x}: {slots.get(off, '?')}"
                vt_regs.discard(base)
            elif base == "$sp" and op in ("sw", "sd", "sq") and reg in this:
                saved.add(off)
            elif base == "$sp" and op in ("lw", "ld", "lq") and off in saved:
                this.add(reg)
                note = "this"
                out.append(line + ("  ; " + note if note else ""))
                continue
            # A load or store through `this` changes the loaded register.
            if op.startswith("l") and reg in this and not (base == "$sp" and off in saved):
                this.discard(reg)
        elif op in ("move", "daddu", "or") and len(parts) >= 2:
            dst, src = parts[0], parts[1]
            if src in this and (op == "move" or parts[-1] in ("$zero",)):
                this.add(dst)
            elif dst in this and src not in this:
                this.discard(dst)
        elif op == "lui" and parts[0] == "$at":
            at_hi = int(parts[1], 0) << 16
            at_this = False
        elif op == "addu" and parts[0] == "$at" and len(parts) == 3:
            at_this = (parts[1] in this and parts[2] == "$at") or (parts[2] in this and parts[1] == "$at")
        elif op in ("addiu", "daddiu") and len(parts) == 3 and parts[1] in this:
            off = int(parts[2], 0)
            nm = name_at(elf, cls, off)
            note = f"&this->{nm}" if nm else f"this+{off:#x}"
            if parts[0] in this:
                this.discard(parts[0])
        elif parts and parts[0] in this and op not in ("sw", "sd", "sq", "sb", "sh", "swc1", "beq", "bne", "beqz", "bnez", "jalr", "jr"):
            this.discard(parts[0])
        if op in ("jal", "jalr"):
            # Calls clobber the temporaries and arguments.
            this -= {r for r in this if r.startswith(("$a", "$v", "$t"))}
        out.append(line + ("  ; " + note if note else ""))
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("elf")
    ap.add_argument("fn")
    ap.add_argument("cls")
    ap.add_argument("--overlay")
    ap.add_argument("--reg", action="append", help="registers holding this on entry (default $a0)")
    a = ap.parse_args()
    print(annotate(a.elf, a.fn, a.cls, a.overlay, a.reg))


if __name__ == "__main__":
    main()
