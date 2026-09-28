#!/usr/bin/env python3
"""tools/eemu.py against its Rust port, crates/piney-eemu (the eemu_rs module).

The port is only worth using if it is eemu, so this runs the two side by
side and compares them after every instruction.

- Lockstep over real workloads. The harnesses' own code (test_anim,
  test_morph_rs, test_world_rs, test_demo_rs, test_desktop_rs,
  test_stream_rs, test_toppage_rs, test_dungeon_rs) drives a stand-in
  machine that runs an eemu machine and an eemu_rs machine together: the
  Rust one runs natively and traces every step; the Python one takes the
  same step, and the pc, the whole register file (GPRs, FPU, HI / LO of both
  pipelines, FCR31, ACC, the step count; vf, ACC, Q, vi and VU0 data memory
  on the VU machines) and every byte either machine wrote must agree. Hooks
  run once, against both machines; the Python side replays what they
  returned. Every call must return the same, leave the same 32 MB, and stop
  (eemu.Stop, or whatever a hook raised) at the same step with the same
  message. A workload's step budget can end it early, the two agreeing.
- Every flavour the harnesses use: eemu.Machine, test_anim's VuMachine,
  test_stream_rs's Vu0Machine (its own Python subclass, cop2 overridden, on
  eemu_rs.VuMachine; and eemu_rs.Vu0Machine), test_toppage_rs's ee_machine
  (its Python subclass, exec overridden, on eemu_rs.Machine; and
  Machine(ee_div=True)); test_dungeon_rs's run_range, which drives exec
  itself.
- Fuzzing: random words over every opcode family (SPECIAL, REGIMM, the
  immediates, jumps and branches, loads and stores into RAM, COP1, COP2
  macro mode, MMI; unsupported encodings too, which must stop alike) from
  random register states rich in the EE float edge cases, one instruction
  at a time on each flavour; random short programs run whole under
  lockstep, hooks, delay slots and likely branches included; the HLE string
  and memory functions on random strings.
- The Python surface itself: mem slices, load / store sizes and signs,
  hooks as a dict, the register views, set / set32 / get, subclass
  overrides, nested calls from hooks, Stop messages.

    python3 tools/test_eemu_rs.py              the tests (builds the module)
    python3 tools/test_eemu_rs.py bench        eemu against eemu_rs on real workloads

EEMU_RS_BUDGET scales every lockstep workload's step budget (default 1).
Skipped when the disc is not extracted or cargo is missing.
"""

import contextlib
import gc
import os
import random
import shutil
import struct
import subprocess
import sys
import time
import unittest
import weakref

TOOLS = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, TOOLS)
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(TOOLS)
ELF = volume.ELF
ISO = volume.ISO
DATA = volume.DATA
MODULE = os.path.join(TOOLS, "eemu_rs.so")
HAVE = os.path.exists(ELF) and os.path.exists(ISO) and os.path.exists(DATA) and shutil.which("cargo")

M32 = 0xFFFFFFFF
M64 = (1 << 64) - 1
ONE = 0x3F800000
SCALE = float(os.environ.get("EEMU_RS_BUDGET", "1"))


def build():
    """Build the module and put it next to this file as eemu_rs.so."""
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-eemu", "--features", "python"],
                   cwd=ROOT, check=True)
    target = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
    shutil.copyfile(os.path.join(target, "release", "libpiney_eemu.so"), MODULE + ".tmp")
    os.replace(MODULE + ".tmp", MODULE)


_MISSING = object()


@contextlib.contextmanager
def patched(obj, name, value):
    old = getattr(obj, name, _MISSING)
    setattr(obj, name, value)
    try:
        yield
    finally:
        if old is _MISSING:
            delattr(obj, name)
        else:
            setattr(obj, name, old)


# The machines ---------------------------------------------------------------

class Flavours:
    """The Python classes and their eemu_rs counterparts, made once (the
    Python ones before anything is patched)."""

    def __init__(self):
        import eemu
        import eemu_rs
        import test_anim
        import test_stream_rs
        import test_toppage_rs
        self.eemu, self.rs = eemu, eemu_rs
        self.py_machine = eemu.Machine
        # The references are eemu.py's, whatever test_anim now prefers.
        with patched(test_anim, "machine_class", test_anim._python_machine_class):
            self.py_vu = test_anim.machine_class()
            self.py_vu0 = test_stream_rs.machine_class()
        # test_stream_rs's own Vu0Machine on eemu_rs.VuMachine: its cop2
        # override, unchanged, over the native VuMachine.
        with patched(test_anim, "machine_class", lambda: eemu_rs.VuMachine):
            self.rs_vu0_sub = test_stream_rs.machine_class()
        self.toppage_ee = test_toppage_rs.ee_machine

    def py_ee(self, program):
        return self.toppage_ee(program)

    def rs_ee_sub(self, program):
        """test_toppage_rs's ee_machine on eemu_rs.Machine: its exec override."""
        with patched(self.eemu, "Machine", self.rs.Machine):
            return self.toppage_ee(program)


_FLAVOURS = None


def flavours():
    global _FLAVOURS
    if _FLAVOURS is None:
        _FLAVOURS = Flavours()
    return _FLAVOURS


# Lockstep ---------------------------------------------------------------------

class Mismatch(BaseException):
    """The two machines differ. A BaseException, so harness code's `except
    Exception` cannot swallow it."""


class Budget(BaseException):
    """The workload's step budget is spent, the two machines agreeing."""


class Counter:
    """Steps compared, across every machine a workload makes."""

    def __init__(self, budget=None):
        self.budget = budget
        self.steps = 0
        self.calls = 0
        self.machines = 0
        self.words = {}             # instruction word -> times run (None: a hook)

    def ops(self):
        """{instruction: times run}, by mips.py's names."""
        out = {}
        for w, n in self.words.items():
            k = "(hook)" if w is None else base_mnemonic(w)
            out[k] = out.get(k, 0) + n
        return out


class Pair:
    """A machine class for harness code: makes a Lockstep of a Python machine
    and a Rust one. A class instance, not a function, so storing it as a
    class attribute (test_morph_rs does) does not bind it."""

    def __init__(self, make_py, make_rs, counter):
        self.make_py, self.make_rs, self.counter = make_py, make_rs, counter

    def __call__(self, program):
        # Harness hooks close over harness objects that hold the machine, so
        # dead machines sit in reference cycles; at 64 MB a machine (RAM and
        # the pristine copy), collect them before making two more.
        gc.collect()
        return Lockstep(self.make_py(program), self.make_rs(program), self.counter)


class DualMem:
    """Both machines' RAM: reads from the Python machine's, writes to both.
    Offers a read-only buffer (struct.unpack_from and friends)."""

    __slots__ = ("a", "b")

    def __init__(self, a, b):
        self.a, self.b = a, b

    def __len__(self):
        return len(self.a)

    def __getitem__(self, k):
        return self.a[k]

    def __setitem__(self, k, v):
        if isinstance(k, slice) and not isinstance(v, (bytes, bytearray)):
            v = bytes(v)
        self.a[k] = v
        self.b[k] = v

    def index(self, *a):
        return self.a.index(*a)

    def find(self, *a):
        return self.a.find(*a)

    def __buffer__(self, flags):
        return memoryview(self.a).toreadonly()

    def __eq__(self, other):
        return self.a == other


class DualSeq:
    """A register file on both machines (r, f, vacc, vi, or one vf row):
    reads from the Python machine's, writes to both."""

    __slots__ = ("a", "b")

    def __init__(self, a, b):
        self.a, self.b = a, b         # () -> each machine's current sequence

    def __getitem__(self, k):
        return self.a()[k]

    def __setitem__(self, k, v):
        self.a()[k] = v
        self.b()[k] = v

    def __len__(self):
        return len(self.a())

    def __iter__(self):
        return iter(list(self.a()))

    def __eq__(self, other):
        return list(self.a()) == list(other)

    def __repr__(self):
        return repr(list(self.a()))


class DualVf:
    __slots__ = ("ls",)

    def __init__(self, ls):
        self.ls = ls

    def __getitem__(self, i):
        py, rs = self.ls.py, self.ls.rs
        return DualSeq(lambda: py.vf[i], lambda: rs.vf[i])

    def __setitem__(self, i, row):
        row = list(row)
        self.ls.py.vf[i] = list(row)
        self.ls.rs.vf[i] = row

    def __len__(self):
        return 32

    def __iter__(self):
        return (self[i] for i in range(32))


class DualHooks:
    """The hooks on both machines. The Python machine holds what the harness
    set (so reading a hook back gives the harness's function); the Rust one a
    wrapper that runs it once, against the Lockstep, and records what it
    returned for the Python side to replay. eemu's own HLE functions go to
    both as they are: each machine runs its own (eemu_rs natively)."""

    def __init__(self, ls):
        self.ls = ls

    def __setitem__(self, k, f):
        ls = self.ls
        ls.py.hooks[k] = f
        ls.rs.hooks[k] = f if id(f) in ls.hle else ls.wrap(k, f)

    def __getitem__(self, k):
        return self.ls.py.hooks[k]

    def __delitem__(self, k):
        del self.ls.py.hooks[k]
        del self.ls.rs.hooks[k]

    def __contains__(self, k):
        return k in self.ls.py.hooks

    def __len__(self):
        return len(self.ls.py.hooks)

    def __iter__(self):
        return iter(list(self.ls.py.hooks))

    def get(self, k, default=None):
        return self.ls.py.hooks.get(k, default)

    def pop(self, k, *default):
        if k not in self.ls.py.hooks and default:
            return default[0]
        v = self.ls.py.hooks.pop(k)
        self.ls.rs.hooks.pop(k)
        return v

    def update(self, *a, **kw):
        for k, v in dict(*a, **kw).items():
            self[k] = v

    def setdefault(self, k, default=None):
        if k not in self:
            self[k] = default
        return self[k]

    def items(self):
        return self.ls.py.hooks.items()

    def keys(self):
        return self.ls.py.hooks.keys()

    def values(self):
        return self.ls.py.hooks.values()


def outcome(fn):
    """(value, None) or (None, exception)."""
    try:
        return fn(), None
    except (Mismatch, Budget, KeyboardInterrupt):
        raise
    except BaseException as e:           # noqa: BLE001 - compared, then re-raised
        return None, e


def same_exc(a, b):
    return type(a) is type(b) and str(a) == str(b)


def py_state(m, vu, vi):
    """The Python machine's state in eemu_rs's `state()` shape."""
    return (tuple(m.r), tuple(m.f), m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps,
            tuple(map(tuple, m.vf)) if vu else DEFAULT_VF, tuple(m.vacc) if vu else (0,) * 4,
            m.q if vu else 0, tuple(m.vi) if vi else (0,) * 16)


DEFAULT_VF = ((0, 0, 0, ONE),) + ((0, 0, 0, 0),) * 31
FIELDS = ("r", "f", "hi", "lo", "hi1", "lo1", "fcr31", "acc", "steps", "vf", "vacc", "q", "vi")


class Stepper:
    """eemu.Machine.call's loop for the Python machine, one iteration at a
    time; foreign hooks are replayed from what the Rust side recorded."""

    def __init__(self, ls, addr, args, limit):
        import eemu
        m = self.m = ls.py
        self.ls, self.limit, self.eemu = ls, limit, eemu
        m.r = [0] * 32
        m.r[28] = m.p.gp
        m.r[29] = eemu.STACK_TOP
        m.r[31] = eemu.RETURN_SENTINEL
        for i, a in enumerate(args[:8]):
            m.set32(4 + i, a)
        self.pc, self.npc = addr, addr + 4
        m.steps = 0

    def step(self):
        m, pc, ls = self.m, self.pc, self.ls
        ls.py_writes.clear()
        m.steps += 1
        if m.steps > self.limit:
            raise self.eemu.Stop(f"gave up after {self.limit} steps at 0x{pc:08x}")
        hook = m.hooks.get(pc)
        if hook is not None:
            args = [m.r[4 + i] & M32 for i in range(4)]
            if id(hook) in ls.hle:
                ls.py_writes.extend(hle_writes(m, ls.hle[id(hook)], args))
                ret = hook(m, *args)
            else:
                ret = ls.replay(pc)
            m.set32(2, ret)
            self.pc = m.r[31] & M32
            self.npc = self.pc + 4
            return
        w = m.load(pc, 4)
        target = m.exec(pc, w)
        if target is self.eemu.ANNUL:
            self.pc, self.npc = self.npc + 4, self.npc + 8
        else:
            self.pc, self.npc = self.npc, (target if target is not None else self.npc + 4)


def hle_writes(m, name, a):
    """What eemu's HLE function `name` is about to write: [(address, n)]."""
    import eemu
    d, s, n = a[0], a[1], a[2]
    try:
        if name in ("memcpy", "memmove", "memset"):
            return [(d, n)]
        if name == "strcpy":
            return [(d, len(eemu._cstr(m, s)) + 1)]
        if name == "strcat":
            return [(d + len(eemu._cstr(m, d)), len(eemu._cstr(m, s)) + 1)]
    except ValueError:
        pass
    return []


class Lockstep:
    """What harness code sees as its machine: runs an eemu machine and an
    eemu_rs one together and compares them after every step."""

    def __init__(self, py, rs, counter=None):
        import eemu
        set_ = object.__setattr__
        set_(self, "py", py)
        set_(self, "rs", rs)
        set_(self, "counter", counter or Counter())
        set_(self, "stack", [])
        set_(self, "replays", [])
        set_(self, "py_writes", [])
        set_(self, "hle", {id(f): name for name, f in eemu.HLE.items()})
        set_(self, "vu", hasattr(py, "vf"))
        set_(self, "vi", hasattr(py, "vi"))
        set_(self, "mem", DualMem(py.mem, rs.mem))
        set_(self, "hooks", DualHooks(self))
        self.counter.machines += 1
        # The Python machine's stores, logged; weak references, so that the
        # machines do not keep themselves (and their 32 MB) alive.
        store = type(py).store
        log = self.py_writes
        me, pm = weakref.ref(self), weakref.ref(py)

        def logged(addr, n, value):
            log.append((addr & M32, n))
            return store(pm(), addr, n, value)
        py.store = logged
        rs.trace = lambda *a: me().on_step(*a)
        if py.mem != rs.mem:
            raise Mismatch("the two machines load different images")

    # attributes ---------------------------------------------------------------
    def __getattr__(self, name):
        if name in ("r", "f", "vacc", "vi"):
            py, rs = self.py, self.rs
            return DualSeq(lambda: getattr(py, name), lambda: getattr(rs, name))
        if name == "vf":
            return DualVf(self)
        return getattr(self.py, name)

    def __setattr__(self, name, value):
        if name == "hooks":
            self.py.hooks.clear()
            self.rs.hooks.clear()
            self.hooks.update(value)
            return
        if name in ("r", "f", "vf", "vacc", "vi"):
            setattr(self.py, name, [list(x) if name == "vf" else x for x in value])
            setattr(self.rs, name, value)
            return
        setattr(self.py, name, value)
        setattr(self.rs, name, value)

    # both machines ------------------------------------------------------------
    def both(self, what, fn):
        a, ea = outcome(lambda: fn(self.py))
        b, eb = outcome(lambda: fn(self.rs))
        if ea is not None or eb is not None:
            if not same_exc(ea, eb):
                self.fail(f"{what}: Python {ea!r}, Rust {eb!r}")
            raise ea
        if a != b:
            self.fail(f"{what}: Python {a!r}, Rust {b!r}")
        return a

    def load(self, addr, n, signed=False):
        return self.both(f"load({addr:#x}, {n})", lambda m: m.load(addr, n, signed))

    def store(self, addr, n, value):
        self.both(f"store({addr:#x}, {n})", lambda m: m.store(addr, n, value))

    def get(self, i):
        return self.both(f"get({i})", lambda m: m.get(i))

    def set(self, i, v, bits=64):
        self.both(f"set({i})", lambda m: m.set(i, v, bits))

    def set32(self, i, v):
        self.both(f"set32({i})", lambda m: m.set32(i, v))

    def vset(self, i, mask, vals):
        self.both(f"vset({i})", lambda m: m.vset(i, mask, vals))

    def changed(self):
        return self.both("changed()", lambda m: m.changed())

    # hooks ----------------------------------------------------------------------
    def wrap(self, addr, fn):
        replays = self.replays
        me = weakref.ref(self)

        def hook(rs, *args):
            try:
                ret = fn(me(), *args)
            except BaseException as e:
                replays.append((addr, e, None))
                raise
            replays.append((addr, None, ret))
            return ret
        return hook

    def replay(self, pc):
        if not self.replays:
            self.fail(f"0x{pc:08x}: the Python machine calls a hook the Rust one did not")
        addr, exc, ret = self.replays.pop(0)
        if addr != pc:
            self.fail(f"0x{pc:08x}: the Python machine calls a hook, the Rust one called 0x{addr:08x}")
        if exc is not None:
            raise exc
        return ret

    # running ---------------------------------------------------------------------
    def call(self, addr, args=(), limit=5_000_000):
        import eemu
        st = Stepper(self, addr, args, limit)
        self.stack.append(st)
        self.counter.calls += 1
        try:
            try:
                got = self.rs.call(addr, args, limit)
            except (Mismatch, Budget, KeyboardInterrupt):
                raise
            except BaseException as e:
                self.same_failure(st, e)
                raise
            if st.pc != eemu.RETURN_SENTINEL:
                self.fail(f"the Rust call returned; the Python one is at 0x{st.pc:08x}")
            want = self.py.r[2] & M32
            if got != want:
                self.fail(f"call 0x{addr:08x} returned {got:#x}, eemu {want:#x}")
            if len(self.stack) == 1:
                self.check_ram(f"after call 0x{addr:08x}")
            return got
        finally:
            self.stack.pop()

    def same_failure(self, st, e):
        """The Rust call raised e: the Python machine's next step must too."""
        _, ea = outcome(st.step)
        if ea is None:
            self.fail(f"Rust stopped with {e!r}; eemu took the step")
        if not (ea is e or same_exc(ea, e)):
            self.fail(f"Rust stopped with {e!r}; eemu with {ea!r}")
        self.compare(st.pc, None, st.pc, st.npc, count=False)

    def exec(self, pc, w):
        """test_dungeon_rs's run_range calls exec itself: both, compared."""
        self.py_writes.clear()
        a, ea = outcome(lambda: self.py.exec(pc, w))
        b, eb = outcome(lambda: self.rs.exec(pc, w))
        if ea is not None or eb is not None:
            if not same_exc(ea, eb):
                self.fail(f"0x{pc:08x}: Python {ea!r}, Rust {eb!r}", pc, w)
            raise ea
        if not (a is b or a == b):
            self.fail(f"0x{pc:08x}: exec returned {a!r}, Rust {b!r}", pc, w)
        self.compare(pc, w, None, None)
        return a

    def on_step(self, pc, w, next_pc, next_npc):
        """The Rust machine's trace: it just took the step at pc."""
        st = self.stack[-1]
        if st.pc != pc:
            self.fail(f"the Rust machine stepped 0x{pc:08x}, eemu is at 0x{st.pc:08x}", pc, w)
        _, e = outcome(st.step)
        if e is not None:
            self.fail(f"the Rust machine took the step at 0x{pc:08x}; eemu raised {e!r}", pc, w)
        words = self.counter.words
        words[w] = words.get(w, 0) + 1
        self.compare(pc, w, next_pc, next_npc)
        if (next_pc, next_npc) != (st.pc, st.npc):
            self.fail(f"after 0x{pc:08x}: Rust goes to 0x{next_pc:08x}/0x{next_npc:08x}, "
                      f"eemu to 0x{st.pc:08x}/0x{st.npc:08x}", pc, w)

    def compare(self, pc, w, next_pc, next_npc, count=True):
        py, rs = self.py, self.rs
        a, b = py_state(py, self.vu, self.vi), rs.state()
        if a != b:
            diff = []
            for name, x, y in zip(FIELDS, a, b):
                if x != y:
                    if isinstance(x, tuple):
                        diff += [f"{name}[{i}] {u!r} / {v!r}" for i, (u, v) in enumerate(zip(x, y)) if u != v][:6]
                    else:
                        diff.append(f"{name} {x!r} / {y!r}")
            self.fail("state differs (eemu / eemu_rs): " + "; ".join(diff), pc, w)
        for addr, n in self.py_writes + (rs.writes or []):
            if py.mem[addr:addr + n] != rs.mem[addr:addr + n]:
                self.fail(f"memory at 0x{addr:08x}+{n}: eemu {bytes(py.mem[addr:addr + n]).hex()} "
                          f"eemu_rs {bytes(rs.mem[addr:addr + n]).hex()}", pc, w)
        if self.vi and py.vumem != rs.vumem:
            self.fail("VU0 data memory differs", pc, w)
        if count:
            c = self.counter
            c.steps += 1
            if c.budget is not None and c.steps >= c.budget:
                raise Budget()

    def check_ram(self, where):
        a, b = self.py.mem, self.rs.mem
        if a != b:
            i = next(i for i in range(0, len(a), 4096) if a[i:i + 4096] != b[i:i + 4096])
            j = i + next(k for k in range(4096) if a[i + k] != b[i + k])
            self.fail(f"{where}: RAM differs from 0x{j:08x}")

    def fail(self, msg, pc=None, w=None):
        import mips
        if pc is not None and w is not None:
            msg = f"0x{pc:08x} {mips.decode(w, pc).text}: {msg}"
        raise Mismatch(msg)


def run_budgeted(fn):
    """Run a workload until it ends or its budget does."""
    try:
        fn()
        return "complete"
    except Budget:
        return "budget"


# Workloads: the harnesses' own code, their game side -----------------------

def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def w_anim(vu):
    """test_anim: controllers, playback and ccMorpher::Modify."""
    import test_anim
    with patched(test_anim, "machine_class", lambda: vu):
        g = test_anim.GameAnim()
    rng = random.Random(5)
    for fname, anm in (("xp_flag.cmp", "ANM_xpflnut0"), ("xdl_load.cmp", "ANM_xdl_lod0"),
                       ("eldl.cmp", "ANM_eldlmag0")):
        c = test_anim.member(fname)
        _, off, a = test_anim.find(c, anm)
        bad = test_anim.check_controllers(g, c, off, a, rng, times=2, max_tracks=3)
        assert bad == [], bad
    c = test_anim.member("xp_flag.cmp")
    scene, off, a = test_anim.find(c, "ANM_xpflnut0")
    assert test_anim.check_playback(g, c, scene, off, a, 700, 6) == []
    rng = random.Random(9)
    base = [tuple(rng.randint(-32768, 32767) for _ in range(3)) for _ in range(12)]
    tgts = [([tuple(rng.randint(-32768, 32767) for _ in range(3)) for _ in base], fb(0.37), fb(1.5))
            for _ in range(2)]
    g.modify(base, fb(1.0), tgts)


def w_morph(vu):
    """test_morph_rs: ccMorpher::Modify on random morphers."""
    import test_morph_rs
    from image import Program
    t = test_morph_rs.MorphAgainstGame("test_modify")
    rng = random.Random(0x6d6f7270)
    with patched(test_morph_rs.MorphAgainstGame, "machine", vu), \
            patched(test_morph_rs.MorphAgainstGame, "prog", Program(ELF)):
        for _ in range(120):
            t.run_game(*test_morph_rs.case(rng))


def w_world(vu):
    """test_world_rs: the collision mesh decoded by the game, land checks,
    then Kite's frames (cameraMain, ccPlayer::Main) on walking pads."""
    import test_anim
    import test_world_rs as tw
    with patched(test_anim, "machine_class", lambda: vu):
        f = tw.Field()
    rng = random.Random(21)
    for _ in range(4):
        f.land((fb(rng.uniform(-3600, 3000)), fb(rng.uniform(-6900, 6900)), fb(1200.0)))
    f.start((0, 0x45AF0000, 0x44160000), 0, 0, 3, 1)
    for pad in tw.script_pads("walk", 120, rng)[76:]:
        f.pad(*pad)
        f.frame()


def w_demo(plain, vu):
    """test_demo_rs: MoveCurNut on random pads, SetNeutral, the fader,
    ccSaveData::LoadGame, and the icons' light (a VuMachine)."""
    import eemu
    import test_demo_rs as td
    from image import Program
    td.DemoAgainstGame.p = Program(ELF, "demo")
    t = td.DemoAgainstGame("test_move_cur_nut")
    rng = random.Random(4)
    bits = [td.UP, td.DOWN, td.UP | td.DOWN, 0, 0, 0]
    with patched(eemu, "Machine", plain):
        for run in range(8):
            m = t.machine()
            t.neutral(m, run & 1)
            m.store(td.THIS + td.O["curno"], 4, rng.randrange(-1, 5))
            for _ in range(80):
                for o in (0x2d0, 0x2d4, 0x2d8):
                    m.store(td.SYS + o, 4, rng.choice(bits) if rng.random() < 0.4 else 0)
                t.calls, t.se = [], []
                m.call(td.FN["movecur"], [td.THIS])
        for script in ([0x00160360, 20, 20, 5, 2164260863], [0x00160240, 50, 2164260863],
                       [0x00160400, 20, 0, 2147483648]):
            m = t.machine()
            m.store(td.FADE, 0x98, 0)
            m.call(script[0], [td.FADE] + script[1:])
            for _ in range(25):
                t.send_packet(m)
        sym = lambda n: t.p.symbol_named(n).value  # noqa: E731
        for _ in range(3):
            m = t.machine()
            m.store(td.SAVEDATA, 4, td.SAVE)
            m.store(td.CCSYS, 4, td.SYS)
            m.hooks[sym("SetDisplayOffset__8ccSystemFii")] = lambda mm, *a: 0
            m.hooks[sym("SetSoundEnv__10ccSaveDataFv")] = lambda mm, *a: 0
            m.hooks[sym("setCameraCtrlType__Fi")] = lambda mm, *a: 0
            m.mem[td.SAVE:td.SAVE + 0x8530] = bytes(rng.randrange(256) for _ in range(0x8530))
            m.call(sym("LoadGame__10ccSaveDataFv"), [td.SAVE])
    import ccs
    import ccsmodel
    import test_anim
    with patched(test_anim, "machine_class", lambda: vu):
        # The icons' light through ccSetMatrixPacket (then VU1 in tools/vu.py).
        lit = [model for model in ccsmodel.models(ccs.Ccs(ccs.load(td.TITLE1)))
               if model.mmats and model.mmats[0].positions]
        for k, model in enumerate(lit[:4]):
            world = [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 10.0 * k, 20.0, -30.0 * k, 1.0]
            t.vu_light(world, model, model.mmats[0], 0.25 * (k + 1))


def w_desktop(plain):
    """test_desktop_rs: SelectMode on random states and pads, DrawLogo."""
    import eemu
    import test_desktop_rs as td
    from image import Program
    td.DesktopAgainstGame.p = Program(ELF, "desktop")
    t = td.DesktopAgainstGame("test_select_mode")
    rng = random.Random(3)
    pushes = [0, td.UP, td.DOWN, td.RIGHT, td.LEFT, td.OK, td.CANCEL, td.SELECT, td.START, td.OK | td.START]
    with patched(eemu, "Machine", plain):
        m = t.machine()
        for _ in range(1500):
            st = [rng.randrange(0, 6), rng.randrange(2), rng.choice([-1, -1, 0, 3]), rng.choice([0, 1, 2]),
                  rng.randrange(2), rng.randrange(2), rng.choice([0, 1]), rng.randrange(2),
                  rng.choice([0, 1 << rng.randrange(6), 1 << (19 + rng.randrange(6))]), rng.choice(pushes)]
            dsel, lr, app, check, flg, start, drawflg, lock, operate, push = st
            for off, v in ((0x190, dsel), (0x194, lr), (0x198, app), (0x19c, check), (0x1a0, flg),
                           (0x1a4, 5), (0x1ac, drawflg)):
                m.store(td.THIS + off, 4, v)
            m.store(td.THIS + 0x1b4, 1, start)
            m.store(td.THIS + 0x1b6, 1, lock)
            m.store(td.EV + 0x770, 8, operate)
            m.store(td.EV + 0x778, 2, 0xffff)
            m.store(td.GAMEOBJ + 0x4c, 4, 1)
            m.store(td.SYS + 0x2d0, 4, push)
            t.se = []
            m.call(td.SELECT_MODE, [td.THIS])
        m = t.logo_machine()
        for dsel in range(8):
            for f in range(64):
                for off, v in ((0x190, dsel), (0x194, f & 1), (0x1ac, f >> 1 & 1)):
                    m.store(td.THIS + off, 4, v)
                m.store(td.THIS + 0x1b4, 1, f >> 2 & 1)
                m.store(td.THIS + 0x1b6, 1, f >> 3 & 1)
                for a in t.anm_label:
                    m.store(a + 0x9c, 2, 256)
                t.calls = []
                t.fwd = {"logo": bool(f & 16), "in": bool(f & 32), "out": bool(f & 1), "title": bool(f & 2)}
                m.call(td.DRAW_LOGO, [td.THIS])


def w_stream(vu0):
    """test_stream_rs: WaitEnd's skips, every stream's table lookup, and
    stream 106's scene decoded and drawn frame by frame (ccStream,
    ccCoord::_SetLWMatrix's walk in VU0's integer registers)."""
    import test_stream_rs as ts
    with patched(ts, "machine_class", lambda: vu0):
        ts.skips(ts.Game())
        ts.tables(ts.Game())
        ts.run_scene(ts.Game(), 106, ["str6100"], 2)


def w_toppage(ee):
    """test_toppage_rs: ccThToppageCtrl's constructor and Main a frame at a
    time over pressed buttons: the top page, the board, reading."""
    import test_toppage_rs as tt
    from image import Program
    p = Program(ELF, "toppage")
    raws = tt.presses(160, {5: tt.DOWN, 9: tt.OK, 20: tt.DOWN, 24: tt.OK, 40: tt.CANCEL, 50: tt.UP, 55: tt.OK,
                            70: tt.OK, 80: tt.DOWN, 84: tt.DOWN, 90: tt.OK, 100: tt.CANCEL, 110: tt.CANCEL,
                            120: tt.UP, 125: tt.OK, 140: tt.CANCEL}, holds=((60, 66, tt.DOWN),))
    with patched(tt, "ee_machine", ee):
        g = tt.Game(p, [], 0, (0, 0))
        g.start([])
        prev = 0
        for raw in raws:
            g.frame((raw, raw & ~prev, prev & ~raw, raw & ~prev), -1, [])
            prev = raw


def w_dungeon(plain):
    """test_dungeon_rs: run_range drives exec itself over the DUNGEON
    constructor's fog block; WORLD_MAN::SetDungeonTexClut by call."""
    import dungeon
    import test_dungeon_rs as td
    data = dungeon.Data(ELF)
    p = data.p
    va, words, _ = data._code("__ct__7DUNGEONFi")
    stores = [va + 4 * k for k, w in enumerate(words)
              if w >> 26 == 0x2B and (w >> 21) & 31 == 16 and w & 0xFFFF == 0x50]
    m = plain(p)
    m.store(p.symbol_named("worldman").value, 4, td.WM)
    m.store(p.symbol_named("game").value, 4, td.GAME)
    for clut in range(6):
        for tex in (0, 1):
            for dtype in range(10):
                for bg in range(4):
                    m.mem[td.DG:td.DG + 0x200] = bytes(0x200)
                    m.mem[td.WM:td.WM + 0x4E0] = bytes(0x4E0)
                    m.store(td.DG + 0x10, 4, dtype)
                    m.store(td.WM + 0x140, 4, clut)
                    m.store(td.WM + 0x144, 4, tex)
                    m.store(td.WM + 0x0C, 4, bg)
                    td.run_range(m, stores[0], stores[1] + 4, {16: td.DG})
    fn = p.symbol_named("SetDungeonTexClut__9WORLD_MANFv").value
    for server in range(6):
        for ft in range(13):
            m.mem[td.WM:td.WM + 0x4E0] = bytes(0x4E0)
            m.store(td.WM + 0x10, 4, ft)
            m.store(td.GAME + 0x1C, 4, server)
            m.call(fn, (td.WM,))


def workloads(fl):
    """name -> (driver, budget, [machine factories by counter])."""
    rs = fl.rs

    def pl(c):
        return Pair(fl.py_machine, rs.Machine, c)

    def vu(c):
        return Pair(fl.py_vu, rs.VuMachine, c)
    return {
        "anim": (lambda c: w_anim(vu(c)), 5_000_000),
        "morph": (lambda c: w_morph(vu(c)), 1_000_000),
        "world": (lambda c: w_world(vu(c)), 10_000_000),
        "demo": (lambda c: w_demo(pl(c), vu(c)), 2_000_000),
        "desktop": (lambda c: w_desktop(pl(c)), 2_000_000),
        "stream subclass": (lambda c: w_stream(Pair(fl.py_vu0, fl.rs_vu0_sub, c)), 5_000_000),
        "stream native": (lambda c: w_stream(Pair(fl.py_vu0, rs.Vu0Machine, c)), 5_000_000),
        "toppage subclass": (lambda c: w_toppage(Pair(fl.py_ee, fl.rs_ee_sub, c)), 2_000_000),
        "toppage native": (lambda c: w_toppage(Pair(fl.py_ee, lambda p: rs.Machine(p, ee_div=True), c)), 2_000_000),
        "dungeon": (lambda c: w_dungeon(pl(c)), 1_000_000),
    }


# Fuzzing ------------------------------------------------------------------------

EDGE_FLOATS = (0, 0x80000000, 0x00000001, 0x807FFFFF, 0x00800000, 0x80800000, 0x7F800000, 0xFF800000,
               0x7FFFFFFF, 0xFFFFFFFF, 0x7F7FFFFF, ONE, 0xBF800000, 0x3F000000, 0x4B000000, 0x4F000000,
               0xCF000000, 0x3F800001, 0x3F7FFFFF, 0x7F000000, 0x01000000, 0x5F000000)


def rand_float(rng):
    k = rng.random()
    if k < 0.35:
        return rng.choice(EDGE_FLOATS)
    if k < 0.7:
        return fb(rng.choice([rng.uniform(-4, 4), rng.uniform(-1e6, 1e6), rng.uniform(-1e-6, 1e-6),
                              float(rng.randint(-100, 100))]))
    return rng.getrandbits(32)


def rand_reg(rng):
    k = rng.random()
    if k < 0.15:
        return rng.choice((0, 1, M32, 0x7FFFFFFF, 0x80000000, M64, 0xFFFFFFFF80000000, 1 << 63))
    if k < 0.45:
        return rng.randrange(1 << 25) & ~rng.choice((0, 3, 15))       # an address in RAM
    if k < 0.65:
        v = rng.getrandbits(32)
        return (v | (M64 ^ M32)) if v >> 31 else v                      # a 32-bit value, sign-extended
    if k < 0.8:
        return rng.getrandbits(64)
    if k < 0.9:
        return rng.getrandbits(128)
    return rng.randrange(-40, 40) & M64


def rand_word(rng):
    """A random instruction word, weighted towards what eemu_rs supports."""
    w = rng.getrandbits(32)
    k = rng.random()
    if k < 0.2:                                   # SPECIAL
        w &= 0x03FFFFFF
    elif k < 0.25:                                # REGIMM
        w = (w & 0x03FFFFFF) | 1 << 26
    elif k < 0.37:                                # MMI
        w = (w & 0x03FFFFFF) | 0x1C << 26
        if rng.random() < 0.6:
            # the (funct, sa) pairs the VuMachine names, and their neighbours
            funct, sa = rng.choice(((0x08, 0x12), (0x08, 0x05), (0x08, 0x14), (0x08, 0x16), (0x08, 0x17),
                                    (0x09, 0x0E), (0x09, 0x10), (0x28, 0x12), (0x29, 0x08), (0x29, 0x09),
                                    (0x29, 0x0E), (0x3F, rng.randrange(32)), (rng.choice((0x08, 0x09, 0x28, 0x29)),
                                                                            rng.randrange(32))))
            w = (w & ~0x7FF) | sa << 6 | funct
        elif rng.random() < 0.5:
            w = (w & ~63) | rng.choice((0x18, 0x19, 0x1A, 0x1B, 0x10, 0x11, 0x12, 0x13, 0x34, 0x36, 0x37, 0x3C,
                                        0x3E, 0x00, 0x01, 0x04, 0x20, 0x21, 0x30, 0x31))
        if rng.random() < 0.4:                    # the must-be-zero fields pmthi / pmtlo / psraw need
            w &= ~rng.choice((31 << 21, 31 << 16, 31 << 11, (31 << 16) | (31 << 11)))
    elif k < 0.52:                                # COP1
        fmt = rng.choice((0x00, 0x02, 0x04, 0x06, 0x08, 0x10, 0x10, 0x10, 0x14, rng.randrange(32)))
        w = (w & 0x001FFFFF) | 0x11 << 26 | fmt << 21
        if fmt == 0x08:
            w = (w & ~(31 << 16)) | rng.randrange(6) << 16
        if fmt == 0x14 and rng.random() < 0.7:
            w = (w & ~63) | 0x20
        if fmt in (0x02, 0x06) and rng.random() < 0.5:
            w = (w & ~(31 << 11)) | rng.choice((0, 31)) << 11
    elif k < 0.72:                                # COP2
        rs_ = rng.choice((0x01, 0x02, 0x05, 0x06, 0x08, rng.randrange(0x10, 0x20), rng.randrange(0x10, 0x20),
                          rng.randrange(0x10, 0x20)))
        w = (w & 0x001FFFFF) | 0x12 << 26 | rs_ << 21
        if rs_ >= 0x10 and rng.random() < 0.5:
            w = (w & ~63) | (0x3C + rng.randrange(4))
    elif k < 0.85:                                # loads and stores
        w = (w & 0x03FFFFFF) | rng.choice((0x20, 0x21, 0x23, 0x24, 0x25, 0x27, 0x37, 0x1E, 0x28, 0x29, 0x2B, 0x3F,
                                           0x1F, 0x31, 0x39, 0x36, 0x3E, 0x1A, 0x1B, 0x2F, 0x33, 0x22, 0x26)) << 26
        if rng.random() < 0.8:
            w = (w & ~0xFFFF) | (rng.randrange(-64, 64) & 0xFFFF)
    return w


def randomise(pair, rng):
    """The same random state (and VU0 data memory) on both machines."""
    py, rs = pair
    r = [0] + [rand_reg(rng) for _ in range(31)]
    f = [rand_float(rng) for _ in range(32)]
    extra = dict(hi=rng.getrandbits(rng.choice((32, 64, 128))), lo=rng.getrandbits(rng.choice((32, 64, 128))),
                 hi1=rng.getrandbits(rng.choice((32, 64, 128))), lo1=rng.getrandbits(rng.choice((32, 64, 128))),
                 fcr31=rng.choice((0, 1 << 23, rng.getrandbits(32))), acc=rand_float(rng))
    for m in (py, rs):
        m.r = list(r)
        m.f = list(f)
        for k, v in extra.items():
            setattr(m, k, v)
    if hasattr(py, "vf"):
        vf = [[0, 0, 0, ONE]] + [[rand_float(rng) for _ in range(4)] for _ in range(31)]
        vacc = [rand_float(rng) for _ in range(4)]
        q = rand_float(rng)
        py.vf, py.vacc, py.q = [list(x) for x in vf], list(vacc), q
        rs.vf, rs.vacc, rs.q = vf, vacc, q
    if hasattr(py, "vi"):
        vi = [0] + [rng.getrandbits(16) for _ in range(15)]
        mem = bytes(rng.getrandbits(8) for _ in range(4096)) if rng.random() < 0.1 else bytes(py.vumem)
        py.vi, rs.vi = list(vi), vi
        py.vumem[:] = mem
        rs.vumem[:] = mem


def fuzz_pairs(fl, program):
    """(label, Lockstep) for every flavour."""
    rs = fl.rs
    return [("Machine", Lockstep(fl.py_machine(program), rs.Machine(program))),
            ("VuMachine", Lockstep(fl.py_vu(program), rs.VuMachine(program))),
            ("Vu0Machine", Lockstep(fl.py_vu0(program), rs.Vu0Machine(program))),
            ("Vu0Machine subclass", Lockstep(fl.py_vu0(program), fl.rs_vu0_sub(program))),
            ("ee_div", Lockstep(fl.py_ee(program), rs.Machine(program, ee_div=True))),
            ("ee_machine subclass", Lockstep(fl.py_ee(program), fl.rs_ee_sub(program)))]


def base_mnemonic(w):
    """The instruction w is by its opcode fields, from mips.py's tables
    (before pseudo-ops and VU dest suffixes, must-be-zero fields ignored as
    eemu ignores them); None for a word they do not know."""
    import mips
    if w >> 26 == 0 and w & 63 == 0x0F:
        return "sync"                     # eemu: any sa (mips.py: sync, sync.p, else nothing)
    node = mips._ROOT[w >> 26]
    while node is not None and not isinstance(node, tuple):
        node = node(w)
    return None if node is None else node[0]


# Every instruction eemu interprets, and what test_anim's VuMachine and
# test_stream_rs's Vu0Machine add: the fuzzer must have run each one.
EEMU_OPS = set("""sll srl sra sllv srlv srav jr jalr movz movn sync mfhi mthi mflo mtlo mult multu div divu
add addu sub subu and or xor nor slt sltu dsllv dsrlv dsrav dadd daddu dsub dsubu dsll dsrl dsra dsll32
dsrl32 dsra32 bltz bgez bltzl bgezl bltzal bgezal j jal beq bne blez bgtz beql bnel blezl bgtzl addi addiu
slti sltiu andi ori xori lui daddi daddiu mult1 multu1 div1 divu1 mfhi1 mthi1 mflo1 mtlo1 mfc1 cfc1 mtc1
ctc1 bc1f bc1t bc1fl bc1tl add.s sub.s mul.s div.s sqrt.s abs.s mov.s neg.s rsqrt.s adda.s suba.s mula.s
madd.s msub.s madda.s msuba.s cvt.w.s max.s min.s c.f.s c.eq.s c.lt.s c.le.s cvt.s.w lb lbu lh lhu lw lwu
ld lq sb sh sw sd sq lwc1 swc1 cache pref""".split())
VU_OPS = set("""lqc2 sqc2 ldl ldr qmfc2.ni qmfc2.i qmtc2.ni qmtc2.i vmulq vaddq vmaddq vsubq vmsubq vadd vmadd
vmul vmax vsub vmsub vopmsub vmini vmulaq vaddaq vmaddaq vsubaq vmsubaq vadda vmadda vmula vsuba vmsuba
vitof0 vitof4 vitof12 vitof15 vftoi0 vftoi4 vftoi12 vftoi15 vabs vopmula vmove vmr32 vdiv vsqrt vrsqrt vnop
vwaitq pextlw pextuw pcpyld pcpyud pextlh psubh paddsh ppach psraw pmthi pmtlo pmaddh""".split()) | {
    f"{op}{lane}" for op in ("vadd", "vsub", "vmadd", "vmsub", "vmax", "vmini", "vmul", "vadda", "vsuba",
                             "vmadda", "vmsuba", "vmula") for lane in "xyzw"}
VU0_OPS = set("cfc2.ni cfc2.i ctc2.ni ctc2.i viadd viaddi vlqi vsqi vlqd vsqd".split())


def fuzz_instructions(fl, program, n, seed):
    """n random words on every flavour. Returns ({(flavour, outcome):
    count}, {flavour: instructions run without stopping})."""
    import eemu
    rng = random.Random(seed)
    counts, ran = {}, {}
    for label, ls in fuzz_pairs(fl, program):
        py, rs = ls.py, ls.rs
        ran[label] = set()
        for _ in range(n):
            randomise((py, rs), rng)
            w = rand_word(rng)
            pc = rng.choice((0x00100000, inf_va(0x00400800), rng.randrange(1 << 25) & ~3, 0xFFFFFFFC, 0x1FFFFFC))
            try:
                ret = ls.exec(pc, w)
                kind = "annul" if ret is eemu.ANNUL else "jump" if ret is not None else "next"
                ran[label].add(base_mnemonic(w))
            except eemu.Stop as e:
                kind = "stop: read/write" if "outside RAM" in str(e) else "stop: not interpreted"
                if "outside RAM" in str(e):
                    ran[label].add(base_mnemonic(w))
            counts[(label, kind)] = counts.get((label, kind), 0) + 1
    return counts, ran


def fuzz_programs(fl, program, n, seed):
    """n random programs per flavour, run whole under lockstep, with hooks:
    mostly instructions the flavour interprets, branches nearby, jal into
    the program, `jr ra` at the end."""
    import eemu
    rng = random.Random(seed)
    at = 0x01F00000
    stats = {"return": 0, "stop": 0, "steps": 0}
    for label, ls in fuzz_pairs(fl, program):
        ok = EEMU_OPS | (VU_OPS if ls.vu else set()) | (VU0_OPS if ls.vi else set())
        for _ in range(n):
            words = []
            for _ in range(rng.randrange(4, 40)):
                w = rand_word(rng)
                while rng.random() < 0.95 and base_mnemonic(w) not in ok:
                    w = rand_word(rng)
                op = w >> 26
                if op in (1, 4, 5, 6, 7, 0x14, 0x15, 0x16, 0x17) or (op == 0x11 and (w >> 21) & 31 == 8):
                    w = (w & ~0xFFFF) | (rng.randrange(-6, 12) & 0xFFFF)          # branch nearby
                if op in (2, 3) and rng.random() < 0.8:
                    w = (w & ~0x03FFFFFF) | ((at >> 2) + rng.randrange(40)) & 0x03FFFFFF
                if op == 0 and w & 63 in (8, 9) and rng.random() < 0.7:
                    w = (w & ~(31 << 21)) | 31 << 21                                  # jr / jalr ra
                words.append(w)
            if rng.random() < 0.8:
                words += [0x03E00008, 0]                                           # jr ra; nop
            blob = b"".join(struct.pack("<I", w) for w in words) + bytes(64)
            ls.mem[at:at + len(blob)] = blob
            hooks = {}
            if rng.random() < 0.5:
                hooks[at + 4 * rng.randrange(len(words))] = lambda mm, a0, *a: (a0 * 3 + len(a)) & M32
            if rng.random() < 0.3:
                # Only the HLE functions that do not write: a random-length
                # memcpy past the end of RAM grows eemu's bytearray, where
                # eemu_rs raises BufferError (README.md). fuzz_hle covers the
                # writers with lengths inside RAM.
                name = rng.choice(("strlen", "strcmp", "strncmp", "memcmp"))
                hooks[at + 4 * rng.randrange(len(words))] = eemu.HLE[name]
            for k, v in hooks.items():
                ls.hooks[k] = v
            args = [rand_reg(rng) & M32 for _ in range(rng.randrange(9))]
            try:
                ls.call(at, args, limit=rng.choice((50, 400, 2000)))
                stats["return"] += 1
            except (eemu.Stop, ValueError):
                stats["stop"] += 1
            stats["steps"] += ls.py.steps
            for k in hooks:
                del ls.hooks[k]
    return stats


def fuzz_hle(fl, program, n, seed):
    """eemu's HLE functions on random strings and buffers, called through
    their hooks on both machines."""
    import eemu
    rng = random.Random(seed)
    ls = Lockstep(fl.py_machine(program), fl.rs.Machine(program))
    at = 0x01E00000
    done = 0
    for _ in range(n):
        blob = bytes(rng.choice(b"abcAB\0\0\xff") for _ in range(256))
        ls.mem[at:at + 256] = blob
        name = rng.choice(sorted(eemu.HLE))
        sym = program.symbol_named(name)
        if sym is None:
            continue
        args = [at + rng.randrange(200), at + rng.randrange(200), rng.randrange(80), 0]
        try:
            ls.call(sym.value, args)
            done += 1
        except ValueError:
            pass
    return done


# Tests --------------------------------------------------------------------------

@unittest.skipUnless(HAVE, "needs the extracted disc and cargo")
class EemuRs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        build()
        cls.fl = flavours()
        cls.p = Program(ELF)

    def lockstep(self, name):
        driver, budget = workloads(self.fl)[name]
        c = Counter(int(budget * SCALE))
        t = time.time()
        how = run_budgeted(lambda: driver(c))
        gc.collect()
        ops = c.ops()
        vu = sorted(k for k in ops if k in VU_OPS | VU0_OPS)
        print(f"\n{name}: {c.steps} steps in lockstep over {c.calls} calls on {c.machines} machines, {how}, "
              f"{time.time() - t:.0f} s; {len(ops)} distinct instructions, {len(vu)} of them VU0 / MMI"
              + (f" ({' '.join(vu)})" if vu else ""), file=sys.stderr)
        self.assertGreater(c.steps, 10_000)
        return ops

    def test_lockstep_anim(self):
        ops = self.lockstep("anim")
        self.assertGreater(len(set(ops) & VU_OPS), 15)

    def test_lockstep_morph(self):
        self.lockstep("morph")

    def test_lockstep_world(self):
        self.lockstep("world")

    def test_lockstep_demo(self):
        self.lockstep("demo")

    def test_lockstep_desktop(self):
        self.lockstep("desktop")

    def test_lockstep_stream_subclass(self):
        self.assertTrue(set(self.lockstep("stream subclass")) & VU0_OPS)

    def test_lockstep_stream_native(self):
        self.assertTrue(set(self.lockstep("stream native")) & VU0_OPS)

    def test_lockstep_toppage_subclass(self):
        self.lockstep("toppage subclass")

    def test_lockstep_toppage_native(self):
        self.lockstep("toppage native")

    def test_lockstep_dungeon(self):
        self.lockstep("dungeon")

    def test_fuzz_instructions(self):
        counts, ran = fuzz_instructions(self.fl, self.p, int(20000 * SCALE), 1)
        kinds = {k for _, k in counts}
        # Every instruction each flavour interprets was run, and nothing else.
        self.assertEqual(ran["Machine"], EEMU_OPS)
        self.assertEqual(ran["ee_div"], EEMU_OPS)
        self.assertEqual(ran["VuMachine"], EEMU_OPS | VU_OPS)
        self.assertEqual(ran["Vu0Machine"], EEMU_OPS | VU_OPS | VU0_OPS)
        self.assertEqual(ran["Vu0Machine subclass"], EEMU_OPS | VU_OPS | VU0_OPS)
        self.assertEqual(kinds, {"next", "jump", "annul", "stop: read/write", "stop: not interpreted"})
        total = sum(counts.values())
        print(f"\n{total} random instructions over 6 flavours, every one alike: "
              + ", ".join(f"{k} {sum(v for (_, kk), v in counts.items() if kk == k)}" for k in sorted(kinds)),
              file=sys.stderr)

    def test_fuzz_programs(self):
        s = fuzz_programs(self.fl, self.p, int(150 * SCALE), 2)
        print(f"\nrandom programs on 6 flavours: {s['return']} returned, {s['stop']} stopped, "
              f"{s['steps']} steps, every one alike", file=sys.stderr)
        self.assertGreater(s["return"], 100)
        self.assertGreater(s["stop"], 100)

    def test_fuzz_hle(self):
        self.assertGreater(fuzz_hle(self.fl, self.p, 1500, 3), 800)

    # the Python surface ---------------------------------------------------------
    def test_api(self):
        import eemu
        rs = self.fl.rs
        p = self.p
        a, b = eemu.Machine(p), rs.Machine(p)
        self.assertIs(rs.Stop, eemu.Stop)
        self.assertIs(rs.ANNUL, eemu.ANNUL)
        self.assertIsInstance(b.mem, bytearray)
        self.assertEqual(a.mem, b.mem)
        self.assertEqual(a.pristine, b.pristine)
        self.assertEqual(dict(a.hooks), dict(b.hooks.items()))
        for m in (a, b):
            m.store(0x01000000, 16, -2)
            m.store(0x01000010, 3, 0x123456789)
            m.store(0x01000020, 40, (1 << 300) - 5)
            m.mem[0x01000100:0x01000104] = b"ab\0c"
        self.assertEqual(a.mem, b.mem)
        for n, signed in ((1, True), (2, True), (4, False), (8, True), (16, True), (3, False), (40, True),
                          (0, False)):
            self.assertEqual(a.load(0x01000000, n, signed), b.load(0x01000000, n, signed), (n, signed))
        for m in (a, b):
            m.set(5, -1)
            m.set(6, 1 << 127 | 5, bits=128)
            m.set32(7, 0x80000000)
            m.r[8] = 0x1234
            m.f[3] = 0x40000000
            m.hi = 1 << 100
        self.assertEqual(list(a.r), list(b.r))
        self.assertEqual(list(a.f), list(b.f))
        self.assertEqual((a.get(6), a.get(-1), a.hi), (b.get(6), b.get(-1), b.hi))
        self.assertEqual(b.r[4:9], list(b.r)[4:9])
        # Stops, with eemu's messages.
        for fn in (lambda m: m.load(0x01FFFFFE, 4), lambda m: m.store(0x02000000, 1, 0),
                   lambda m: m.exec(0x00100000, 0x0000000C), lambda m: m.call(0x01000100, (), limit=0),
                   lambda m: m.bad(0x00100008, 0x0000000D)):
            _, ea = outcome(lambda: fn(a))
            _, eb = outcome(lambda: fn(b))
            self.assertIsInstance(eb, eemu.Stop)
            self.assertEqual(str(ea), str(eb))
        # hooks: a dict's behaviour, the native HLE seen as eemu's function.
        strlen = p.symbol_named("strlen").value
        self.assertIs(b.hooks[strlen], eemu._strlen)
        self.assertIs(b.hooks.get(12345, "x"), "x")
        b.hooks[0x01000200] = lambda mm, a0, *r: a0 + 1
        self.assertIn(0x01000200, b.hooks)
        self.assertEqual(b.call(0x01000200, [41]), 42)
        self.assertEqual(b.hooks.pop(0x01000200)(b, 1), 2)
        self.assertEqual(len(b.hooks), len(a.hooks))
        b.hooks.update({0x01000200: lambda mm, *r: 7})
        self.assertEqual(b.call(0x01000200), 7)
        del b.hooks[0x01000200]
        # A hook that forgets its return value fails as in eemu.
        for m in (a, b):
            m.hooks[0x01000200] = lambda mm, *r: None
        _, ea = outcome(lambda: a.call(0x01000200))
        _, eb = outcome(lambda: b.call(0x01000200))
        self.assertTrue(same_exc(ea, eb), (ea, eb))
        for m in (a, b):
            del m.hooks[0x01000200]
        # A hook may call the machine, nested, as with eemu.
        for m in (a, b):
            m.store(0x01000300, 4, 0x6c6c6568)       # "hell"
            m.store(0x01000304, 4, 0x0000006f)       # "o"
            m.hooks[0x01000400] = lambda mm, *r: mm.call(strlen, [0x01000300]) + 100
        self.assertEqual(a.call(0x01000400), b.call(0x01000400))
        # Subclasses: attributes, super().__init__, an exec override.
        seen = []

        class Sub(rs.VuMachine):
            def __init__(self, program):
                super().__init__(program)
                self.note = "kept"

            def exec(self, pc, w):
                seen.append(pc)
                return super().exec(pc, w)
        s = Sub(p)
        self.assertEqual(s.note, "kept")
        s.f[12] = fb(0.5)
        s.call(p.symbol_named("sinf").value)
        self.assertGreater(len(seen), 20)
        self.assertEqual(s.steps, len(seen))
        # A machine in a reference cycle (its hooks close over what holds it,
        # as harness hooks do) is collected, as eemu's are.
        refs = []
        for cls in (rs.Machine, Sub):
            m = cls(p)
            m.hooks[0x100] = lambda mm, *r, _m=m: 0
            m.me, m.view = m, m.r
            refs.append(weakref.ref(m))
            del m
        gc.collect()
        self.assertEqual([r() for r in refs], [None, None])
        # mem can only be swapped for another bytearray of the same size.
        with self.assertRaises(ValueError):
            b.mem = bytearray(10)
        b.mem = bytearray(b.mem)
        with self.assertRaises(BufferError):
            b.mem[0:1] = b"too long"


# Benchmark -----------------------------------------------------------------------

def bench():
    """eemu and eemu_rs on the same workloads, not in lockstep; the results
    compared, the times reported."""
    build()
    fl = flavours()
    rs = fl.rs
    import test_anim
    import test_world_rs as tw

    def world(vu):
        with patched(test_anim, "machine_class", lambda: vu):
            f = tw.Field()
        t = time.time()
        f.start((0, 0x45AF0000, 0x44160000), 0, 0, 3, 1)
        rng = random.Random(11)
        states = []
        for pad in tw.script_pads("walk", 200, rng):
            f.pad(*pad)
            states.append(f.frame())
        return states, time.time() - t

    def decode(vu):
        t = time.time()
        with patched(test_anim, "machine_class", lambda: vu):
            f = tw.Field()
        return bytes(f.m.mem[tw.HEAP:f.heap]), time.time() - t

    def anim(vu):
        with patched(test_anim, "machine_class", lambda: vu):
            g = test_anim.GameAnim()
        t = time.time()
        rng = random.Random(5)
        out = []
        for fname, anm in (("xp_flag.cmp", "ANM_xpflnut0"), ("eldl.cmp", "ANM_eldlmag0")):
            c = test_anim.member(fname)
            scene, off, a = test_anim.find(c, anm)
            out.append(test_anim.check_controllers(g, c, off, a, rng, times=3, max_tracks=4))
            out.append(test_anim.check_playback(g, c, scene, off, a, 256, 40))
        return out, time.time() - t

    def stream(vu0):
        import test_stream_rs as ts
        t = time.time()
        with patched(ts, "machine_class", lambda: vu0):
            out = ts.run_scene(ts.Game(), 106, ["str6100"], 3)
        return out, time.time() - t

    rows = [("world: 200 frames of Kite walking (cameraMain + ccPlayer::Main)", world, fl.py_vu, rs.VuMachine),
            ("world: Field() set-up (DATA.BIN and Kite's animations in Python, Decode_Hit)", decode, fl.py_vu, rs.VuMachine),
            ("anim: controllers and 40 playback steps, 2 animations", anim, fl.py_vu, rs.VuMachine),
            ("stream: stream 106, 3 frames (test_stream_rs's own cop2 override)", stream, fl.py_vu0, fl.rs_vu0_sub),
            ("stream: stream 106, 3 frames (eemu_rs.Vu0Machine)", stream, fl.py_vu0, rs.Vu0Machine)]
    for label, fn, py_cls, rs_cls in rows:
        a, ta = fn(py_cls)
        b, tb = fn(rs_cls)
        same = "same results" if a == b else "RESULTS DIFFER"
        print(f"{label}\n    eemu {ta:8.2f} s   eemu_rs {tb:7.2f} s   {ta / tb:6.1f}x   {same}", flush=True)
    # Raw interpreter speed: libm's sinf, called over and over.
    from image import Program
    p = Program(ELF)
    sinf = p.symbol_named("sinf").value
    for cls in (fl.py_vu, rs.VuMachine):
        m = cls(p)
        t = time.time()
        n = 0
        while time.time() - t < 3:
            m.f[12] = fb(1.2345)
            m.call(sinf)
            n += m.steps
        dt = time.time() - t
        print(f"sinf in a loop, {cls.__module__}.{cls.__name__}: {n / dt / 1e6:.2f} M instructions/s", flush=True)
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bench":
        sys.exit(bench())
    unittest.main()
