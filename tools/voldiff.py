#!/usr/bin/env python3
"""What a later volume's code adds to Infection's: the code that no name
carried from Infection covers (the `<elf>.syms` piney-gen syms writes),
section by section, and what each such stretch addresses and calls.

    tools/voldiff.py gaps ELF [--min BYTES]           the unnamed stretches
    tools/voldiff.py look ELF SECTION LO HI           their strings and calls
    tools/voldiff.py report ELF [--title T]           every stretch, as markdown
    tools/voldiff.py ported [--volume V] [--all]      the port's functions
                                                      in each later volume
    tools/voldiff.py diff NAME [--volume V] [--overlay O]  one function,
                                                      Infection's against V's

`gaps` lists, for main and each overlay, the text's size, how much of it
the carried names cover, and every stretch at least --min bytes long
(default 8192) between two named functions, with the names on either
side. On Infection itself only the embedded fonts show. A stretch is new
code or code compiled too differently to be matched (Outbreak and
Quarantine, whose compiler settings changed); `look` tells them apart:
the strings a stretch materialises (lui/addiu pairs the disassembler
tracks) and the named functions it calls, most used first.

`ported` takes every Infection function the port names - the docs'
`covers`, and the crates' citations (`gcmn 0x0059ddd0`, `main 0x...`,
where the address starts a function) - and finds it in each later volume
by its carried name:

    same        the body is Infection's, addresses masked (xfer.normalise)
    recompiled  the opcode sequence is Infection's (xfer.shape), or it
                calls the same functions in the same order and uses the
                same constants (Outbreak's compiler lays the stack out
                anew, fills delay slots, inlines sqrtf and fptoui, and
                the structures' fields moved): the compiler's doing, not
                the code's
    changed     neither; the percentages say how much of the masked words
                and of the opcode sequence agree
    missing     no name was carried

It prints each volume's counts, then its changed and missing functions
(`--all` also the recompiled), least alike first, with the pages that
cover them. A changed function whose callees share little with
Infection's may be a name the carry put on the wrong function (Outbreak's
and Quarantine's menus), not a change.
"""

import argparse
import collections
import difflib
import glob
import os
import re
import struct
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import image  # noqa: E402

TOOLS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(TOOLS)
INF_ELF = os.path.join(ROOT, "work/infection/disc/SLUS_202.67")
LATER = {
    "MUT": os.path.join(ROOT, "work/mutation/disc/SLUS_205.62"),
    "OUT": os.path.join(ROOT, "work/outbreak/disc/SLUS_205.63"),
    "QUA": os.path.join(ROOT, "work/quarantine/disc/SLUS_205.64"),
}


SECTIONS = (None, "gcmn", "desktop", "toppage", "demo")


def stretches(elf, ov):
    """One section's text: (lo, hi, named bytes, stretches), each stretch
    (size, at, the name before, the name after) between two named
    functions more than 16 bytes apart."""
    p = image.Program(elf, overlay=ov)
    fns = sorted(p.functions(), key=lambda f: f.value)
    if ov:
        lo, hi = p.overlay.text, p.overlay.text + p.overlay.text_size
    else:
        lo, hi = 0x00100000, max(f.value + f.size for f in fns if f.value < 0x00400000)
    fns = [f for f in fns if lo <= f.value < hi]
    found, named, cur, prev = [], 0, lo, None
    for f in fns:
        if f.value > cur + 16:
            found.append((f.value - cur, cur, prev.name if prev else "-", f.name))
        named += f.size
        if f.value + f.size > cur:
            cur, prev = f.value + f.size, f
    if hi > cur + 16:
        found.append((hi - cur, cur, prev.name if prev else "-", "-"))
    return lo, hi, named, found


def gaps(elf, least):
    for ov in SECTIONS:
        lo, hi, named, found = stretches(elf, ov)
        total = sum(f[0] for f in found)
        print(f"{ov or 'main'}: text {hi - lo:#x}, named {named:#x}, unnamed {total:#x} "
              f"({100 * total / max(1, hi - lo):.1f}%) in {len(found)} stretches")
        for size, at, a, b in sorted(found, reverse=True):
            if size >= least:
                print(f"  {at:08x} {size:6x}  after {a[:60]}  before {b[:60]}")


def look_data(elf, section, lo, hi):
    """A stretch's strings and calls, most used first, and how many
    function prologues (`addiu $sp, $sp, -N`) and non-zero words it has."""
    ov = None if section == "main" else section
    p = image.Program(elf, overlay=ov)
    args = ["python3", os.path.join(TOOLS, "disasm.py"), "range", elf, lo, hi]
    if ov:
        args += ["--overlay", ov]
    out = subprocess.run(args, capture_output=True, text=True).stdout
    strings, calls = collections.Counter(), collections.Counter()
    prologues = words = 0
    for line in out.splitlines():
        parts = line.split()
        if len(parts) >= 2 and re.fullmatch(r"[0-9a-f]{8}", parts[1]) and parts[1] != "00000000":
            words += 1
        if re.search(r"addiu\s+\$sp, \$sp, -", line):
            prologues += 1
        for m in re.finditer(r"# (0x[0-9a-f]{8})", line):
            try:
                s = p.cstr(int(m.group(1), 16))
            except Exception:
                continue
            if isinstance(s, bytes):
                s = s.decode("latin1")
            if s and len(s) >= 4 and all(32 <= ord(c) < 127 for c in s):
                strings[s] += 1
        m = re.search(r"jal\s+(\S+)", line)
        if m:
            calls[m.group(1)] += 1
    return strings, calls, prologues, words


def look(elf, section, lo, hi):
    strings, calls, _, _ = look_data(elf, section, lo, hi)
    print("strings:", ", ".join(s for s, _ in strings.most_common(40)))
    print("calls:", ", ".join(c for c, _ in calls.most_common(30)))


def report(elf, title):
    """Every stretch of every section as markdown: where, how long, the
    names either side, its prologues, and its strings and named calls."""
    def cell(s):
        return s.replace("|", "\\|").replace("`", "'")
    print(f"## {title}")
    print()
    for ov in SECTIONS:
        lo, hi, named, found = stretches(elf, ov)
        total = sum(f[0] for f in found)
        sec = ov or "main"
        print(f"### {sec}")
        print()
        print(f"Text {hi - lo:#x} bytes, {named:#x} named, {total:#x} unnamed "
              f"({100 * total / max(1, hi - lo):.1f}%) in {len(found)} stretches.")
        print()
        if not found:
            continue
        print("| at | bytes | after | before | fns | strings | calls |")
        print("| --- | ---: | --- | --- | ---: | --- | --- |")
        for size, at, a, b in sorted(found, key=lambda f: f[1]):
            strings, calls, fns, words = look_data(elf, sec, f"{at:#x}", f"{at + size:#x}")
            if words == 0:
                what_s, what_c = "(zero words: padding)", ""
            else:
                what_s = ", ".join(f"`{cell(s[:40])}`" for s, _ in strings.most_common(4))
                named_calls = [c for c, _ in calls.most_common() if not c.startswith("0x")]
                what_c = ", ".join(f"`{cell(c[:48])}`" for c in named_calls[:5])
            print(f"| `{at:08x}` | {size:#x} | `{cell(a[:40])}` | `{cell(b[:40])}` | {fns} | {what_s} | {what_c} |")
        print()


def covered():
    """{(section, va): pages} of every Infection function a page's `covers`
    names: `; `-separated groups `VOL FILE:0xADDR name, ...`, main's
    addresses (below 0x00400000) in main whatever the group's file."""
    out = collections.defaultdict(set)
    for path in sorted(glob.glob(os.path.join(ROOT, "docs", "*", "*.md"))):
        with open(path) as f:
            line = next((ln for ln in f if ln.startswith("covers:")), None)
        if not line:
            continue
        page = os.path.splitext(os.path.basename(path))[0]
        for group in line[len("covers:"):].split(";"):
            m = re.match(r"\s*(INF|MUT|OUT|QUA) ([^:]+):(.*)", group, re.S)
            if not m or m.group(1) != "INF":
                continue
            name = m.group(2).strip()
            if name.startswith("SLUS"):
                ov = None
            elif name.lower().endswith(".prg"):
                ov = name[:-4].lower()
            else:
                continue
            for a in re.findall(r"0x([0-9a-f]{8})", m.group(3)):
                va = int(a, 16)
                out[(None if va < 0x00400000 else ov, va)].add(page)
    cite = re.compile(r"\b(gcmn|main|desktop|toppage|demo) 0x([0-9a-f]{8})\b")
    for path in sorted(glob.glob(os.path.join(ROOT, "crates", "**", "*.rs"), recursive=True)):
        rel = os.path.relpath(path, os.path.join(ROOT, "crates"))
        crate, _, rest = rel.partition(os.sep)
        where = crate.removeprefix("piney-") + ":" + os.path.splitext(os.path.basename(rest))[0]
        with open(path, encoding="utf-8", errors="replace") as f:
            for sec, a in cite.findall(f.read()):
                va = int(a, 16)
                out[(None if sec == "main" or va < 0x00400000 else sec, va)].add(where)
    return out


# Library calls Outbreak's compiler inlines.
INLINED = {"__fixunssfsi", "fptosi", "fptoui", "sqrtf"}


def xfer_normalise(words):
    import xfer
    return xfer.normalise(words)


def callees(side, words, va):
    """The names of the functions `words` (at `va`) calls, in order; the
    library calls Outbreak's compiler inlines left out."""
    out = []
    for i, w in enumerate(words):
        if w >> 26 != 3:
            continue
        t = ((va + 4 * i + 4) & 0xF0000000) | ((w & 0x3FFFFFF) << 2)
        f = side.p.function_at(t)
        name = f.name if f is not None and f.value == t else f"{t:08x}"
        if name not in INLINED:
            out.append(name)
    return out


def constants(words):
    """The values the code compares with, masks with and loads: the
    immediates of slti, sltiu, andi, ori and xori and of an addiu or
    daddiu from $zero, whatever the instruction (a compiler may load a mask and `and`
    it, or `andi` it). An addiu from another register is left out: it adds
    a structure's field offset as often as a number, and the fields moved
    between volumes. So are an andi's 0xff and 0xffff (a zero extension,
    which one compiler does with a load and the other with a mask) and the
    address halves xfer.normalise masks."""
    out = collections.Counter()
    for w in xfer_normalise(words):
        op, rs, imm = w >> 26, (w >> 21) & 31, w & 0xFFFF
        if op == 0x0C and imm in (0xFF, 0xFFFF):
            continue
        if (op in (0x0A, 0x0B, 0x0C, 0x0D, 0x0E) or (op in (0x08, 0x09, 0x19) and rs == 0)) and imm:
            out[imm] += 1
    return out


def same_calls(a, b):
    """Callee lists equal, a name the later volume's carry missed (an
    address) standing for any."""
    return len(a) == len(b) and all(x == y or not y[:1].isalpha() and not y.startswith("_") for x, y in zip(a, b))


def body(side, va, size):
    raw = side.p.read(va, size)
    return list(struct.unpack(f"<{len(raw) // 4}I", raw[: len(raw) // 4 * 4]))


def ported(volumes, show_all):
    import xfer
    try:
        from demangle import demangle
    except ImportError:
        demangle = lambda s: s  # noqa: E731
    cov = covered()
    sections = sorted({s for s, _ in cov}, key=lambda s: s or "")
    src = {s: xfer.Side(INF_ELF, s) for s in sections}
    for vol in volumes:
        rows = []
        for sec in sections:
            dst = xfer.Side(LATER[vol], sec)
            by_name = {f.name: f for f in dst.p.functions() if dst.owns(f.value)}
            for (s, va), pages in cov.items():
                if s != sec:
                    continue
                f = src[sec].p.function_at(va)
                if f is None or f.value != va:
                    continue
                g = by_name.get(f.name)
                label = demangle(f.name) or f.name
                if g is None:
                    rows.append(("missing", 0.0, 0.0, sec, va, None, label, pages))
                    continue
                a, b = body(src[sec], f.value, f.size), body(dst, g.value, g.size)
                na, nb = xfer.normalise(a), xfer.normalise(b)
                if na == nb:
                    kind, wr, sr = "same", 1.0, 1.0
                else:
                    sa, sb = xfer.shape(a), xfer.shape(b)
                    wr = difflib.SequenceMatcher(None, na, nb, autojunk=False).ratio()
                    sr = difflib.SequenceMatcher(None, sa, sb, autojunk=False).ratio()
                    calls = same_calls(callees(src[sec], a, f.value), callees(dst, b, g.value))
                    alike = calls and constants(a) == constants(b)
                    kind = "recompiled" if sa == sb or alike else "changed"
                rows.append((kind, wr, sr, sec, va, g.value, label, pages))
        counts = collections.Counter(r[0] for r in rows)
        print(f"{vol}: {len(rows)} functions; " + ", ".join(
            f"{k} {counts[k]}" for k in ("same", "recompiled", "changed", "missing")))
        shown = ("changed", "missing", "recompiled") if show_all else ("changed", "missing")
        for kind, wr, sr, sec, va, at, label, pages in sorted(
                (r for r in rows if r[0] in shown), key=lambda r: (r[0], r[2], r[1])):
            where = f"{at:08x}" if at is not None else "--------"
            print(f"  {kind:10s} {sec or 'main':8s} {va:08x} {where} words {wr:4.0%} shape {sr:4.0%}  "
                  f"{label[:70]}  [{', '.join(sorted(pages))}]")


def listing(elf, overlay, va):
    """disasm.py's listing of the function at `va`, each line without its
    address and word, branch labels and the temporary and saved registers
    renumbered in order of appearance, and the saved registers' spills
    dropped, so that two volumes' listings line up."""
    args = ["python3", os.path.join(TOOLS, "disasm.py"), "fn", elf, hex(va), "--demangle"]
    if overlay:
        args += ["--overlay", overlay]
    out = subprocess.run(args, capture_output=True, text=True).stdout.splitlines()
    labels = {}
    regs = {}
    lines = []

    def reg(m):
        # Temporaries and saved registers by first appearance: which one
        # the compiler picked is its business.
        return regs.setdefault(m.group(0), f"$r{len(regs)}")
    for ln in out[2:]:
        m = re.match(r"\s*[0-9a-f]{8}\s+[0-9a-f]{8}\s+(.*)", ln)
        text = m.group(1) if m else ln.strip()
        for lab in re.findall(r"\.L[0-9a-f]{8}", text):
            labels.setdefault(lab, f".L{len(labels)}")
        text = re.sub(r"\.L[0-9a-f]{8}", lambda mm: labels[mm.group(0)], text)
        # Addresses in the comments differ between volumes; the names say it.
        text = re.sub(r"# 0x[0-9a-f]{8} ?", "# ", text)
        text = re.sub(r"\b0x[0-9a-f]{8}\b", "ADDR", text)
        text = re.sub(r"\$(v[01]|t[0-9]|s[0-8]|fp)\b", reg, text)
        # A save or restore of a saved register says nothing.
        if re.match(r"(sq|lq|sd|ld)\s+\$r\d+, \d+\(\$sp\)", text):
            continue
        lines.append(text.rstrip())
    return lines


def diff(name, volume, overlay):
    src = image.Program(INF_ELF, overlay=overlay)
    dst = image.Program(LATER[volume], overlay=overlay)
    f = src.symbol_named(name) or next((s for s in src.functions() if name in s.name), None)
    if f is None:
        raise SystemExit(f"no {name} in Infection")
    g = dst.symbol_named(f.name)
    if g is None:
        raise SystemExit(f"{f.name}: not carried to {volume}")
    a = listing(INF_ELF, overlay, f.value)
    b = listing(LATER[volume], overlay, g.value)
    print(f"# {f.name}: INF {f.value:08x} ({f.size} bytes), {volume} {g.value:08x} ({g.size} bytes)")
    for ln in difflib.unified_diff(a, b, "INF", volume, lineterm="", n=3):
        print(ln)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gaps")
    g.add_argument("elf")
    g.add_argument("--min", type=lambda s: int(s, 0), default=8192)
    r = sub.add_parser("report")
    r.add_argument("elf")
    r.add_argument("--title", default="")
    k = sub.add_parser("look")
    k.add_argument("elf")
    k.add_argument("section")
    k.add_argument("lo")
    k.add_argument("hi")
    d = sub.add_parser("diff")
    d.add_argument("name")
    d.add_argument("--volume", choices=sorted(LATER), default="MUT")
    d.add_argument("--overlay")
    o = sub.add_parser("ported")
    o.add_argument("--volume", choices=sorted(LATER), action="append")
    o.add_argument("--all", action="store_true")
    a = ap.parse_args()
    if a.cmd == "gaps":
        gaps(a.elf, a.min)
    elif a.cmd == "report":
        report(a.elf, a.title or os.path.basename(a.elf))
    elif a.cmd == "diff":
        diff(a.name, a.volume, a.overlay)
    elif a.cmd == "ported":
        ported(a.volume or sorted(LATER, key=list(LATER).index), a.all)
    else:
        look(a.elf, a.section, a.lo, a.hi)


if __name__ == "__main__":
    main()
