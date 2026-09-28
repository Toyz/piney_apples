#!/usr/bin/env python3
"""Metrowerks CodeWarrior (PS2) C++ symbol demangler.

CodeWarrior mangles in the ARM/cfront style that GCC 2.x also used:

    name__F<args>            free function          ccMalloc__FUi
    name__<class>F<args>     method                 InitFirst__6ccHeapFv
    name__<class>CF<args>    const method
    name__<class>            static data member     heapTop__6ccHeap
    __ct / __dt / __vt       ctor, dtor, vtable     __ct__13ccFrameBufferFv
    __nw __dl __as __vc ...  operators              __nw__FUi
    Q2<len>A<len>B           qualified name A::B
    <len>name<T,...>         template class, args mangled inline

Types: v c s i l x f d r b w e (void char short int long `long long` float
double `long double` bool wchar_t ...), U/S/C/V prefixes, P pointer,
R reference, A<n>_ array, F<args>_<ret> function, M<class> member pointer,
T<n> / N<count><n> repeat a previous argument. The return type of a
top-level function is not encoded. The EE's 128-bit integer (u_long128)
comes out as a length-1 name with no characters: `SetDataAdrs__9ccBltDataFP1`
is SetDataAdrs(u_long128 *), per DWARF.

    tools/demangle.py NAME...     demangle each argument
    tools/demangle.py < FILE      demangle every identifier in the text, like
                                  c++filt (e.g. piped from tools/elf.py syms)

Also a library: `demangle(name)` returns the demangled string (the input
unchanged if it does not parse), `parse(name)` returns a `Symbol` with the
pieces, and `cdecl(type, inner)` renders a type tree as a C declarator.
"""

import re
import sys

BUILTIN = {"v": "void", "c": "char", "s": "short", "i": "int", "l": "long",
           "x": "long long", "f": "float", "d": "double", "r": "long double",
           "b": "bool", "w": "wchar_t", "e": "..."}

OPERATORS = {
    "nw": "new", "dl": "delete", "nwa": "new[]", "dla": "delete[]",
    "as": "=", "eq": "==", "ne": "!=", "lt": "<", "gt": ">", "le": "<=",
    "ge": ">=", "pl": "+", "mi": "-", "ml": "*", "dv": "/", "md": "%",
    "er": "^", "ad": "&", "or": "|", "co": "~", "nt": "!", "ls": "<<",
    "rs": ">>", "apl": "+=", "ami": "-=", "amu": "*=", "adv": "/=",
    "amd": "%=", "aer": "^=", "aad": "&=", "aor": "|=", "als": "<<=",
    "ars": ">>=", "aa": "&&", "oo": "||", "pp": "++", "mm": "--",
    "cm": ",", "rm": "->*", "rf": "->", "cl": "()", "vc": "[]",
}

# Compiler-generated data members that read better as a phrase.
SPECIAL_DATA = {"__vt": "vtable for {}", "__RTTI": "RTTI for {}"}


class Fail(Exception):
    pass


# Type trees (shared with dwarf1.py):
#   ("base", name)  ("ptr", t)  ("ref", t)  ("cv", "const"|"volatile"|..., t)
#   ("array", n or None, t)  ("func", ret or None, [params], varargs)
#   ("memptr", classname, t)

def cdecl(t, inner=""):
    """Render type tree `t` as a C declaration of `inner` (may be empty)."""
    kind = t[0]
    if kind == "base":
        return f"{t[1]} {inner}" if inner else t[1]
    if kind == "cv":
        sub = t[2]
        if sub[0] in ("ptr", "ref", "memptr"):
            return _ptrlike(sub, f"{t[1]} {inner}" if inner else t[1])
        if sub[0] in ("base", "cv"):
            # `const int x` reads better than `int const x`
            return cdecl(("base", f"{t[1]} {cdecl(sub)}"), inner)
        return cdecl(sub, inner)  # cv on an array/function: fold away
    if kind in ("ptr", "ref", "memptr"):
        return _ptrlike(t, inner)
    if kind == "array":
        dim = "" if t[1] is None else str(t[1])
        return cdecl(t[2], f"{inner}[{dim}]")
    if kind == "func":
        args = ", ".join(cdecl(p) for p in t[2])
        if t[3]:
            args = f"{args}, ..." if args else "..."
        if t[1] is None:
            return f"{inner}({args})"
        return cdecl(t[1], f"{inner}({args})")
    raise ValueError(f"bad type tree {t!r}")


def _ptrlike(t, inner):
    sym = "*" if t[0] == "ptr" else "&" if t[0] == "ref" else f"{t[1]}::*"
    s = sym + inner
    if t[-1][0] in ("array", "func"):
        s = f"({s})"
    return cdecl(t[-1], s)


class Symbol:
    """A parsed mangled name. `qualname` is e.g. `ccHeap::InitFirst`."""
    __slots__ = ("name", "classname", "params", "varargs", "is_const",
                 "is_func", "text")

    def __init__(self):
        self.name = ""
        self.classname = None
        self.params = []
        self.varargs = False
        self.is_const = False
        self.is_func = False
        self.text = ""

    @property
    def qualname(self):
        return f"{self.classname}::{self.name}" if self.classname else self.name


class _Parser:
    def __init__(self, s, empty128=False):
        self.s = s
        self.i = 0
        self.args = []   # for T/N back-references
        self.empty128 = empty128

    def peek(self):
        return self.s[self.i] if self.i < len(self.s) else ""

    def take(self):
        if self.i >= len(self.s):
            raise Fail("eof")
        c = self.s[self.i]
        self.i += 1
        return c

    def number(self):
        j = self.i
        while self.i < len(self.s) and self.s[self.i].isdigit():
            self.i += 1
        if j == self.i:
            raise Fail("number")
        return int(self.s[j:self.i])

    def name(self):
        """<len><chars> - one name component, template args demangled."""
        n = self.number()
        if n == 1 and self.empty128:
            # CodeWarrior spells the EE's 128-bit integer as a length-1 name
            # with no characters: `SetDataAdrs__9ccBltDataFP1` is
            # SetDataAdrs(u_long128 *) per DWARF.
            return "u_long128"
        if n == 0 or self.i + n > len(self.s):
            raise Fail("name length")
        raw = self.s[self.i:self.i + n]
        self.i += n
        return template_name(raw)

    def qualified(self):
        """A class name: <len>X or Q<n><len>A<len>B (Q_<nn>_ past 9)."""
        if self.peek() == "Q":
            self.i += 1
            if self.peek() == "_":
                self.i += 1
                n = self.number()
                if self.take() != "_":
                    raise Fail("Q_")
            else:
                n = int(self.take())
            if n < 1:
                raise Fail("Q0")
            return "::".join(self.name() for _ in range(n))
        return self.name()

    def type(self):
        c = self.take()
        if c == "C":
            return ("cv", "const", self.type())
        if c == "V":
            return ("cv", "volatile", self.type())
        if c == "U":
            b = self.take()
            if b not in "csilx":
                raise Fail("U")
            return ("base", "unsigned " + BUILTIN[b])
        if c == "S":
            b = self.take()
            if b not in "csilx":
                raise Fail("S")
            return ("base", "signed " + BUILTIN[b])
        if c == "P":
            return ("ptr", self.type())
        if c == "R":
            return ("ref", self.type())
        if c == "A":
            n = self.number()
            if self.take() != "_":
                raise Fail("A_")
            return ("array", n, self.type())
        if c == "F":
            params, va = self.params(stop="_")
            if self.take() != "_":
                raise Fail("F_")
            return ("func", self.type(), params, va)
        if c == "M":
            cls = self.qualified()
            return ("memptr", cls, self.type())
        if c == "Q" or c.isdigit():
            self.i -= 1
            return ("base", self.qualified())
        if c == "T":
            n = self.number()
            if not 1 <= n <= len(self.args):
                raise Fail("T")
            return self.args[n - 1]
        if c in BUILTIN and c != "e":
            return ("base", BUILTIN[c])
        raise Fail(f"type {c!r}")

    def params(self, stop=""):
        """Argument list up to end of string or `stop`. Returns (types, varargs)."""
        out, va = [], False
        saved, self.args = self.args, out
        try:
            while self.i < len(self.s) and self.peek() != stop:
                if self.peek() == "e":
                    self.i += 1
                    va = True
                    continue
                if self.peek() == "N":
                    self.i += 1
                    count = int(self.take())
                    ref = int(self.take())
                    if not 1 <= ref <= len(out):
                        raise Fail("N")
                    out.extend([out[ref - 1]] * count)
                    continue
                out.append(self.type())
        finally:
            self.args = saved
        if out == [("base", "void")] and not va:
            out = []
        return out, va


def template_name(raw):
    """`vector<i,Q23std13allocator<i>>` -> `vector<int, std::allocator<int>>`."""
    lt = raw.find("<")
    if lt < 0 or not raw.endswith(">"):
        return raw
    args, depth, start = [], 0, lt + 1
    for j in range(lt + 1, len(raw) - 1):
        ch = raw[j]
        if ch == "<":
            depth += 1
        elif ch == ">":
            depth -= 1
        elif ch == "," and depth == 0:
            args.append(raw[start:j])
            start = j + 1
    args.append(raw[start:len(raw) - 1])
    return f"{raw[:lt]}<{', '.join(_template_arg(a) for a in args)}>"


def _template_arg(a):
    if re.fullmatch(r"-?\d+", a):
        return a
    p = _Parser(a)
    try:
        t = p.type()
        if p.i == len(a):
            return cdecl(t)
    except (Fail, ValueError, IndexError):
        pass
    return a


def _member_name(fname, cls):
    """Map a special function name (`__ct`, `__pl`, `__op<type>`) to C++."""
    last = re.sub(r"<.*>$", "", cls.rsplit("::", 1)[-1]) if cls else ""
    if fname == "__ct":
        if not cls:
            raise Fail("ctor without class")
        return last
    if fname == "__dt":
        if not cls:
            raise Fail("dtor without class")
        return "~" + last
    if fname.startswith("__op") and len(fname) > 4:
        p = _Parser(fname[4:])
        t = p.type()
        if p.i != len(p.s):
            raise Fail("conversion op")
        return "operator " + cdecl(t)
    if fname.startswith("__") and fname[2:] in OPERATORS:
        op = OPERATORS[fname[2:]]
        return "operator" + (" " if op[0].isalpha() else "") + op
    return template_name(fname)


def _parse_at(sym, cut, empty128):
    fname, rest = sym[:cut], sym[cut + 2:]
    if not fname or not rest:
        raise Fail("empty")
    p = _Parser(rest, empty128)
    out = Symbol()
    c = p.peek()
    if c == "Q" or c.isdigit():
        out.classname = p.qualified()
        if p.peek() == "C" and p.s[p.i + 1:p.i + 2] == "F":
            p.i += 1
            out.is_const = True
    if p.peek() == "F":
        p.i += 1
        out.is_func = True
        out.params, out.varargs = p.params()
    elif out.classname is None:
        raise Fail("no class and no F")
    if p.i != len(p.s):
        raise Fail("trailing")
    if out.is_func:
        out.name = _member_name(fname, out.classname)
        args = ", ".join(cdecl(t) for t in out.params)
        if out.varargs:
            args = f"{args}, ..." if args else "..."
        elif not out.params:
            args = "void"
        out.text = f"{out.qualname}({args})" + (" const" if out.is_const else "")
    else:
        out.name = template_name(fname)
        fmt = SPECIAL_DATA.get(fname)
        out.text = fmt.format(out.classname) if fmt else out.qualname
    return out


def parse(sym):
    """Parse a mangled name into a Symbol, or None if it is not mangled."""
    # The split point is the first `__` (not at the very start) after which
    # the remainder parses completely; names may themselves contain `__`.
    start = 1
    while True:
        cut = sym.find("__", start)
        if cut < 0:
            return None
        # `foo___Fv`: the name is `foo_`, so prefer the last of a run of '_'
        while sym[cut + 2:cut + 3] == "_" and not sym[cut + 3:cut + 4].isdigit():
            cut += 1
        for empty128 in (True, False):
            try:
                return _parse_at(sym, cut, empty128)
            except (Fail, ValueError, IndexError):
                pass
        start = cut + 1


def demangle(sym):
    """Demangled form of `sym`, or `sym` unchanged if it is not mangled."""
    s = parse(sym)
    return s.text if s else sym


# An identifier, possibly with CodeWarrior's inline template arguments.
_TOKEN = re.compile(r"[A-Za-z_$.][\w$.]*(?:<[\w$.,<>]*>[\w$.]*)*")


def main():
    args = sys.argv[1:]
    if args and args[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    if args:
        for a in args:
            print(demangle(a))
        return 0
    for line in sys.stdin:
        sys.stdout.write(_TOKEN.sub(lambda m: demangle(m.group(0)), line))
    return 0


if __name__ == "__main__":
    sys.exit(main())
