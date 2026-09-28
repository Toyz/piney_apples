//! The `eemu_rs` Python module: [`Cpu`] behind an API that mirrors
//! `tools/eemu.py`'s `Machine` closely enough for a harness to switch with
//! its import line (`README.md` lists what differs).
//!
//! - `mem` is a real `bytearray` of 32 MB, so every idiom the harnesses use
//!   on it (slices read and written, `struct.unpack_from`, `.index`, single
//!   bytes) works unchanged. The machine holds a buffer export on it for its
//!   whole life, which pins its storage (a bytearray with an export cannot
//!   be resized or freed); the interpreter reads and writes that storage
//!   directly, only while no Python code can run.
//! - `r`, `f`, `vf`, `vacc` and `vi` are live views onto the registers.
//! - `hooks` is a mapping that keeps the interpreter's hooked-address bitmap
//!   in step with every change; eemu's own HLE functions found there run
//!   natively.
//! - A Python subclass may override `exec`, `cop2` or `mmi` as it would
//!   eemu's; the run loop calls the override for exactly the instructions
//!   eemu's would, and `super()` reaches the native code.
//! - Stops raise `eemu.Stop` itself, with eemu's messages, and a not-taken
//!   branch-likely is `eemu.ANNUL`, so harness code that names either keeps
//!   working.

use std::slice;

use pyo3::PyTraverseError;
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::{PyBufferError, PyIndexError, PyKeyError, PyTypeError, PyValueError};
use pyo3::gc::PyVisit;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyByteArray, PyBytes, PyDict, PyInt, PyIterator, PyList, PySlice, PyTuple, PyType};
use pyo3::{IntoPyObjectExt, intern};

use crate::cpu::{
    Cpu, Event, Fault, Features, Flow, HookKind, HookSet, Mem, RAM, Run, RunOpts, STACK_TOP, Trap, Traps, VU_MEM,
};
use crate::hle::{Hle, HleError, TABLE};

const M64: u128 = u64::MAX as u128;

/// What the module takes from `tools/eemu.py` and `tools/mips.py`.
struct Eemu {
    module: Py<PyModule>,
    stop: Py<PyType>,
    annul: Py<PyAny>,
    /// eemu's HLE functions, recognised by identity.
    hle: Vec<(Py<PyAny>, Hle)>,
    decode: Py<PyAny>,
}

static EEMU: PyOnceLock<Eemu> = PyOnceLock::new();

fn eemu(py: Python<'_>) -> PyResult<&'static Eemu> {
    EEMU.get_or_try_init(py, || {
        let module = py.import("eemu")?;
        let table = module.getattr("HLE")?;
        let table = table.cast::<PyDict>()?;
        let mut hle = Vec::new();
        for (name, h) in TABLE {
            if let Some(f) = table.get_item(name)? {
                hle.push((f.unbind(), h));
            }
        }
        Ok(Eemu {
            stop: module.getattr("Stop")?.cast_into::<PyType>()?.unbind(),
            annul: module.getattr("ANNUL")?.unbind(),
            hle,
            decode: py.import("mips")?.getattr("decode")?.unbind(),
            module: module.unbind(),
        })
    })
}

fn stop(py: Python<'_>, msg: String) -> PyErr {
    match eemu(py) {
        Ok(e) => PyErr::from_type(e.stop.bind(py).clone(), msg),
        Err(e) => e,
    }
}

// Python ints to register words, masked the way eemu's arithmetic masks them.

fn low32(v: &Bound<'_, PyAny>) -> PyResult<u32> {
    if let Ok(x) = v.extract::<i64>() {
        return Ok(x as u32);
    }
    v.bitand(0xffff_ffffu32)?.extract()
}

/// A hook's return value as eemu's `set32` takes it: `sx(v, 32)`, whose
/// `v &= mask` gives a non-int's TypeError its wording.
fn hook_result(v: &Bound<'_, PyAny>) -> PyResult<u32> {
    if let Ok(x) = v.extract::<i64>() {
        return Ok(x as u32);
    }
    let py = v.py();
    py.import("operator")?.call_method1("iand", (v, 0xffff_ffffu32))?.extract()
}

fn low64(v: &Bound<'_, PyAny>) -> PyResult<u64> {
    if let Ok(x) = v.extract::<i64>() {
        return Ok(x as u64);
    }
    v.bitand(u64::MAX)?.extract()
}

fn low128(v: &Bound<'_, PyAny>) -> PyResult<u128> {
    if let Ok(x) = v.extract::<u128>() {
        return Ok(x);
    }
    v.bitand(u128::MAX)?.extract()
}

/// A list index the way Python takes one (negative from the end).
fn index(i: isize, len: usize) -> PyResult<usize> {
    let k = if i < 0 { i + len as isize } else { i };
    if (0..len as isize).contains(&k) { Ok(k as usize) } else { Err(PyIndexError::new_err("list index out of range")) }
}

/// `n` values from a Python sequence, each converted by `f`.
fn values<T, const N: usize>(v: &Bound<'_, PyAny>, f: impl Fn(&Bound<'_, PyAny>) -> PyResult<T>) -> PyResult<[T; N]> {
    let items: Vec<T> = v.try_iter()?.map(|x| f(&x?)).collect::<PyResult<_>>()?;
    let n = items.len();
    items.try_into().map_err(|_| PyValueError::new_err(format!("expected {N} values, got {n}")))
}

/// A bytearray the machine reads and writes directly, pinned by a buffer
/// export for as long as the machine holds it.
struct Buffer {
    obj: Py<PyByteArray>,
    view: PyBuffer<u8>,
}

impl Buffer {
    fn adopt(v: &Bound<'_, PyAny>, len: usize, what: &str) -> PyResult<Buffer> {
        let b = v.cast::<PyByteArray>().map_err(|_| PyTypeError::new_err(format!("{what} must be a bytearray")))?;
        if b.len() != len {
            return Err(PyValueError::new_err(format!("{what} must be {len} bytes, not {}", b.len())));
        }
        let view = PyBuffer::<u8>::get(b.as_any())?;
        if view.readonly() || !view.is_c_contiguous() || view.len_bytes() != len {
            return Err(PyBufferError::new_err(format!("{what} is not a writable contiguous buffer")));
        }
        Ok(Buffer { obj: b.clone().unbind(), view })
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the export pins the bytearray's storage (it cannot be
        // resized or freed while `view` lives), the GIL is held, and no
        // Python code runs while the slice is in use.
        unsafe { slice::from_raw_parts(self.view.buf_ptr().cast::<u8>(), self.view.len_bytes()) }
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as `bytes`; `&mut self` makes this the only Rust view of
        // the storage (a machine's RAM and VU memory are distinct
        // bytearrays: they differ in size).
        unsafe { slice::from_raw_parts_mut(self.view.buf_ptr().cast::<u8>(), self.view.len_bytes()) }
    }
}

/// The hooks: eemu's `hooks` dict, and the bitmap the interpreter checks,
/// kept in step. Keys are addresses; values are called as
/// `hook(machine, a0, a1, a2, a3)` and return `$v0`.
#[pyclass(module = "eemu_rs", name = "Hooks", mapping, unsendable)]
pub struct Hooks {
    dict: Py<PyDict>,
    set: HookSet,
}

fn hook_kind(py: Python<'_>, f: &Bound<'_, PyAny>) -> PyResult<HookKind> {
    let e = eemu(py)?;
    Ok(e.hle.iter().find(|(g, _)| g.bind(py).is(f)).map_or(HookKind::Foreign, |&(_, h)| HookKind::Native(h)))
}

/// The address an int key names, if a pc can ever equal it.
fn key_addr(k: &Bound<'_, PyAny>) -> Option<u64> {
    if k.is_instance_of::<PyInt>() { k.extract::<u64>().ok() } else { None }
}

impl Hooks {
    fn empty(py: Python<'_>) -> Hooks {
        Hooks { dict: PyDict::new(py).unbind(), set: HookSet::default() }
    }

    fn put(&mut self, py: Python<'_>, k: &Bound<'_, PyAny>, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.dict.bind(py).set_item(k, v)?;
        if let Some(a) = key_addr(k) {
            self.set.insert(a, hook_kind(py, v)?);
        }
        Ok(())
    }

    fn take(&mut self, py: Python<'_>, k: &Bound<'_, PyAny>) -> PyResult<()> {
        self.dict.bind(py).del_item(k)?;
        if let Some(a) = key_addr(k) {
            self.set.remove(a);
        }
        Ok(())
    }
}

#[pymethods]
impl Hooks {
    /// Hooks usually close over objects that hold the machine: the cycle
    /// collector has to see through them.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.dict)
    }

    fn __clear__(&mut self) {
        Python::attach(|py| self.dict.bind(py).clear());
        self.set.clear();
    }

    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    fn py_new(py: Python<'_>, args: &Bound<'_, PyTuple>, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Hooks> {
        let mut h = Hooks::empty(py);
        h.update(py, args, kwargs)?;
        Ok(h)
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.dict.bind(py).len()
    }

    fn __getitem__<'py>(&self, py: Python<'py>, k: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        self.dict.bind(py).get_item(k)?.ok_or_else(|| PyKeyError::new_err(k.clone().unbind()))
    }

    fn __setitem__(&mut self, py: Python<'_>, k: &Bound<'_, PyAny>, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.put(py, k, v)
    }

    fn __delitem__(&mut self, py: Python<'_>, k: &Bound<'_, PyAny>) -> PyResult<()> {
        self.take(py, k)
    }

    fn __contains__(&self, py: Python<'_>, k: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.dict.bind(py).contains(k)
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        self.dict.bind(py).try_iter()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!("Hooks({})", self.dict.bind(py).repr()?))
    }

    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        match other.cast::<Hooks>() {
            Ok(o) => self.dict.bind(py).eq(o.borrow().dict.bind(py)),
            Err(_) => self.dict.bind(py).eq(other),
        }
    }

    #[pyo3(signature = (k, default = None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        k: &Bound<'py, PyAny>,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(self.dict.bind(py).get_item(k)?.or(default).unwrap_or_else(|| py.None().into_bound(py)))
    }

    #[pyo3(signature = (k, *default))]
    fn pop<'py>(
        &mut self,
        py: Python<'py>,
        k: &Bound<'py, PyAny>,
        default: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self.dict.bind(py).get_item(k)? {
            Some(v) => {
                self.take(py, k)?;
                Ok(v)
            }
            None if !default.is_empty() => default.get_item(0),
            None => Err(PyKeyError::new_err(k.clone().unbind())),
        }
    }

    #[pyo3(signature = (k, default = None))]
    fn setdefault<'py>(
        &mut self,
        py: Python<'py>,
        k: &Bound<'py, PyAny>,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if let Some(v) = self.dict.bind(py).get_item(k)? {
            return Ok(v);
        }
        let v = default.unwrap_or_else(|| py.None().into_bound(py));
        self.put(py, k, &v)?;
        Ok(v)
    }

    /// `dict.update`'s arguments: a mapping or pairs, and keywords.
    #[pyo3(signature = (*args, **kwargs))]
    fn update(
        &mut self,
        py: Python<'_>,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let tmp = PyDict::new(py);
        tmp.call_method("update", args, kwargs)?;
        for (k, v) in tmp.iter() {
            self.put(py, &k, &v)?;
        }
        Ok(())
    }

    fn keys<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.dict.bind(py).call_method0("keys")
    }

    fn values<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.dict.bind(py).call_method0("values")
    }

    fn items<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.dict.bind(py).call_method0("items")
    }

    fn clear(&mut self, py: Python<'_>) {
        self.dict.bind(py).clear();
        self.set.clear();
    }

    fn copy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.dict.bind(py).copy()
    }
}

/// eemu.Machine: the EE interpreter. `Machine(program, *, vu=False,
/// vi=False, ee_div=False)`; the keywords turn on test_anim's VuMachine,
/// test_stream_rs's Vu0Machine and test_toppage_rs's divide-by-zero rule.
#[pyclass(module = "eemu_rs", name = "Machine", subclass, dict, weakref, unsendable)]
pub struct PyMachine {
    cpu: Cpu,
    ram: Buffer,
    vu: Buffer,
    hooks: Py<Hooks>,
    p: Py<PyAny>,
    pristine: Py<PyAny>,
    trace: Option<Py<PyAny>>,
    /// How deep in `call` the machine is, and the outermost call's traps.
    depth: u32,
    traps: Traps,
}

impl PyMachine {
    fn create(py: Python<'_>, program: &Bound<'_, PyAny>, features: Features) -> PyResult<PyMachine> {
        let e = eemu(py)?;
        // eemu's own loading, as Python slice assignments.
        let mem = PyByteArray::new_with(py, RAM, |_| Ok(()))?;
        let elf = program.getattr("elf")?;
        let data = elf.getattr("data")?;
        for seg in elf.getattr("segments")?.try_iter()? {
            let seg = seg?;
            if seg.getattr("type")?.eq(1)? && seg.getattr("filesz")?.is_truthy()? {
                let (vaddr, offset, n): (isize, isize, isize) = (
                    seg.getattr("vaddr")?.extract()?,
                    seg.getattr("offset")?.extract()?,
                    seg.getattr("filesz")?.extract()?,
                );
                let chunk = data.get_item(PySlice::new(py, offset, offset + n, 1))?;
                mem.set_item(PySlice::new(py, vaddr, vaddr + n, 1), chunk)?;
            }
        }
        let ov = program.getattr("overlay")?;
        if !ov.is_none() {
            let (base, data): (isize, Bound<'_, PyAny>) = (ov.getattr("base")?.extract()?, ov.getattr("data")?);
            let n = data.len()? as isize;
            mem.set_item(PySlice::new(py, base, base + n, 1), data)?;
        }
        // SAFETY: nothing else touches the new bytearray while it is copied.
        let pristine = PyBytes::new(py, unsafe { mem.as_bytes() });
        let ram = Buffer::adopt(mem.as_any(), RAM, "mem")?;
        let vu = Buffer::adopt(PyByteArray::new_with(py, VU_MEM, |_| Ok(()))?.as_any(), VU_MEM, "vumem")?;
        let mut hooks = Hooks::empty(py);
        for (name, f) in e.module.bind(py).getattr("HLE")?.cast::<PyDict>()?.iter() {
            let sym = program.call_method1("symbol_named", (name,))?;
            if !sym.is_none() {
                hooks.put(py, &sym.getattr("value")?, &f)?;
            }
        }
        Ok(PyMachine {
            cpu: Cpu::new(features),
            ram,
            vu,
            hooks: Py::new(py, hooks)?,
            p: program.clone().unbind(),
            pristine: pristine.into_any().unbind(),
            trace: None,
            depth: 0,
            traps: Traps::default(),
        })
    }

    fn parts(&mut self) -> (&mut Cpu, Mem<'_>) {
        (&mut self.cpu, Mem { ram: self.ram.bytes_mut(), vu: self.vu.bytes_mut() })
    }
}

/// Whether a Python subclass (or the instance) replaces the native `name`.
fn overridden(slf: &Bound<'_, PyMachine>, base: &Bound<'_, PyType>, name: &str) -> PyResult<bool> {
    let py = slf.py();
    if let Ok(d) = slf.getattr(intern!(py, "__dict__"))
        && d.contains(name)?
    {
        return Ok(true);
    }
    for cls in slf.get_type().mro().iter() {
        if cls.getattr(intern!(py, "__dict__"))?.contains(name)? {
            return Ok(!cls.is(base));
        }
    }
    Ok(false)
}

fn run_opts(slf: &Bound<'_, PyMachine>) -> PyResult<RunOpts> {
    let base = slf.py().get_type::<PyMachine>();
    let vu = slf.borrow().cpu.features.vu;
    Ok(RunOpts {
        traps: Traps { cop2: vu && overridden(slf, &base, "cop2")?, mmi: vu && overridden(slf, &base, "mmi")? },
        exec_trap: overridden(slf, &base, "exec")?,
        trace: false,
        ticks: true,
    })
}

/// eemu's `bad`: "0x{pc:08x} {where}: {insn} is not interpreted".
fn bad_message(slf: &Bound<'_, PyMachine>, pc: u64, w: u32) -> PyResult<String> {
    let py = slf.py();
    let ins = eemu(py)?.decode.bind(py).call1((w, pc))?;
    let p = slf.borrow().p.clone_ref(py);
    let p = p.bind(py);
    let place =
        if p.hasattr("name_at")? { p.call_method1("name_at", (pc,))?.str()?.to_string() } else { String::new() };
    let text = if ins.hasattr("text")? { ins.getattr("text")? } else { ins };
    Ok(format!("0x{pc:08x} {place}: {} is not interpreted", text.str()?))
}

fn fault(slf: &Bound<'_, PyMachine>, f: Fault, limit: i64) -> PyErr {
    let py = slf.py();
    let msg = match f {
        Fault::Bad { pc, w } => bad_message(slf, pc, w),
        Fault::VuBad { pc, w } => eemu(py)
            .and_then(|e| e.decode.bind(py).call1((w, pc))?.getattr("text")?.str().map(|s| s.to_string()))
            .map(|text| format!("0x{pc:08x}: {text} is not interpreted")),
        Fault::Limit { pc, .. } => Ok(format!("gave up after {limit} steps at 0x{pc:08x}")),
        Fault::Hle(HleError::NoTerminator) => return PyValueError::new_err("subsection not found"),
        Fault::Hle(HleError::Resize) => {
            return PyBufferError::new_err("Existing exports of data: object cannot be re-sized");
        }
        f => Ok(f.to_string()),
    };
    match msg {
        Ok(m) => stop(py, m),
        Err(e) => e,
    }
}

/// What `exec` hands back to Python: None, the branch target, or ANNUL.
fn flow_to_py(py: Python<'_>, flow: Flow) -> PyResult<Py<PyAny>> {
    Ok(match flow {
        Flow::Next => py.None(),
        Flow::Jump(t) => t.into_py_any(py)?,
        Flow::Annul => eemu(py)?.annul.clone_ref(py),
    })
}

/// eemu's loop after `exec` returned `target`.
fn advance(py: Python<'_>, run: &mut Run, target: &Bound<'_, PyAny>) -> PyResult<()> {
    if target.is_none() {
        run.advance(Flow::Next);
    } else if target.is(eemu(py)?.annul.bind(py)) {
        run.advance(Flow::Annul);
    } else {
        run.pc = run.npc;
        run.npc = target.extract()?;
    }
    Ok(())
}

fn trace_step(slf: &Bound<'_, PyMachine>, pc: u64, w: Option<u32>, run: &Run) -> PyResult<()> {
    let t = slf.borrow().trace.as_ref().map(|t| t.clone_ref(slf.py()));
    if let Some(t) = t {
        t.call1(slf.py(), (pc, w, run.pc, run.npc))?;
    }
    Ok(())
}

/// Run a call to its end: native stretches, with Python hooks, overrides,
/// tracing and Ctrl-C checks between them. The machine is not borrowed
/// while Python code runs, so hooks may use it freely (even `call` it).
fn drive(slf: &Bound<'_, PyMachine>, run: &mut Run, mut opts: RunOpts, limit: i64) -> PyResult<()> {
    let py = slf.py();
    loop {
        let event = {
            let mut guard = slf.borrow_mut();
            let hooks = guard.hooks.clone_ref(py);
            let hooks = hooks.bind(py).borrow();
            opts.trace = guard.trace.is_some();
            guard.cpu.writes = opts.trace.then(Vec::new);
            let (cpu, mut mem) = guard.parts();
            cpu.run(&mut mem, &hooks.set, run, opts)
        };
        let tracing = opts.trace;
        match event {
            Ok(Event::Done) => return Ok(()),
            Ok(Event::Tick) => py.check_signals()?,
            Ok(Event::Step(pc, w)) => trace_step(slf, pc, w, run)?,
            Ok(Event::Hook) => {
                let pc = run.pc;
                let (f, a) = {
                    let m = slf.borrow();
                    let f = m.hooks.bind(py).borrow().dict.bind(py).get_item(pc)?;
                    (f, m.cpu.args())
                };
                let f = f.ok_or_else(|| PyKeyError::new_err(pc))?;
                let ret = f.call1((slf, a[0], a[1], a[2], a[3]))?;
                let v = hook_result(&ret)?;
                {
                    let mut m = slf.borrow_mut();
                    m.cpu.set32(2, v);
                    run.resume(m.cpu.r[31]);
                }
                if tracing {
                    trace_step(slf, pc, None, run)?;
                }
            }
            Ok(Event::Trap(t, w)) => {
                let pc = run.pc;
                let name = if t == Trap::Cop2 { intern!(py, "cop2") } else { intern!(py, "mmi") };
                let target = slf.call_method1(name, (pc, w))?;
                advance(py, run, &target)?;
                if tracing {
                    trace_step(slf, pc, Some(w), run)?;
                }
            }
            Ok(Event::Exec(w)) => {
                let pc = run.pc;
                let target = slf.call_method1(intern!(py, "exec"), (pc, w))?;
                advance(py, run, &target)?;
                if tracing {
                    trace_step(slf, pc, Some(w), run)?;
                }
            }
            Err(f) => return Err(fault(slf, f, limit)),
        }
    }
}

#[pymethods]
impl PyMachine {
    #[new]
    #[pyo3(signature = (program, *, vu = false, vi = false, ee_div = false))]
    fn new(py: Python<'_>, program: &Bound<'_, PyAny>, vu: bool, vi: bool, ee_div: bool) -> PyResult<PyMachine> {
        PyMachine::create(py, program, Features { vu: vu || vi, vi, ee_div })
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.hooks)?;
        visit.call(&self.p)?;
        visit.call(&self.pristine)?;
        visit.call(&self.trace)?;
        visit.call(&self.ram.obj)?;
        visit.call(&self.vu.obj)
    }

    fn __clear__(&mut self) {
        // The hooks clear themselves; the instance dict is PyO3's.
        self.trace = None;
    }

    /// Construction happens in `__new__`; this takes a subclass's
    /// `super().__init__(program)`.
    #[pyo3(signature = (*_args, **_kwargs))]
    fn __init__(&self, _args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) {}

    // memory -------------------------------------------------------------------

    #[pyo3(signature = (addr, n, signed = false))]
    fn load<'py>(
        &self,
        py: Python<'py>,
        addr: &Bound<'py, PyAny>,
        n: i64,
        signed: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let addr = low32(addr)?;
        if i128::from(addr) + i128::from(n) > RAM as i128 {
            return Err(stop(py, format!("read of {n} bytes at 0x{addr:08x} is outside RAM")));
        }
        if !(0..=16).contains(&n) {
            // The odd sizes, as eemu computes them.
            let chunk =
                self.ram.obj.bind(py).get_item(PySlice::new(py, addr as isize, addr as isize + n as isize, 1))?;
            let kw = PyDict::new(py);
            kw.set_item("signed", signed)?;
            return py.get_type::<PyInt>().call_method("from_bytes", (chunk, "little"), Some(&kw));
        }
        let a = addr as usize;
        let v = self.ram.bytes()[a..a + n as usize].iter().rev().fold(0u128, |v, &b| v << 8 | u128::from(b));
        if signed && n > 0 {
            let sh = 128 - 8 * n as u32;
            (((v << sh) as i128) >> sh).into_bound_py_any(py)
        } else {
            v.into_bound_py_any(py)
        }
    }

    fn store(&mut self, py: Python<'_>, addr: &Bound<'_, PyAny>, n: i64, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let addr = low32(addr)?;
        if i128::from(addr) + i128::from(n) > RAM as i128 {
            return Err(stop(py, format!("write of {n} bytes at 0x{addr:08x} is outside RAM")));
        }
        if n < 0 {
            return Err(PyValueError::new_err("negative shift count"));
        }
        if n <= 16 {
            let v = low128(value)?;
            if n > 0 {
                let (cpu, mem) = self.parts();
                cpu.store(mem.ram, addr, n as u32, v).map_err(|f| stop(py, f.to_string()))?;
            }
            return Ok(());
        }
        let one = 1u8.into_bound_py_any(py)?;
        let mask = one.lshift(8 * n)?.sub(1)?;
        let bytes = value.bitand(mask)?.call_method1("to_bytes", (n, "little"))?;
        let bytes = bytes.cast::<PyBytes>()?;
        let a = addr as usize;
        self.ram.bytes_mut()[a..a + n as usize].copy_from_slice(bytes.as_bytes());
        if let Some(w) = &mut self.cpu.writes {
            w.push((addr, n as u32));
        }
        Ok(())
    }

    /// [(start, end)] byte ranges that differ from the loaded image,
    /// leaving out the stack.
    fn changed(&self, py: Python<'_>) -> PyResult<Vec<(usize, usize)>> {
        let view = PyBuffer::<u8>::get(self.pristine.bind(py))?;
        let b = view.to_vec(py)?;
        let a = self.ram.bytes();
        let n = STACK_TOP as usize - 0x10_0000;
        if b.len() < n && a[..b.len()] == b[..] {
            return Err(PyIndexError::new_err("index out of range"));
        }
        let mut out = Vec::new();
        let mut i = 0;
        while i < n {
            if b.len() <= i {
                return Err(PyIndexError::new_err("index out of range"));
            }
            if a[i] != b[i] {
                let mut j = i;
                while j < n && j < b.len() && a[j] != b[j] {
                    j += 1;
                }
                out.push((i, j));
                i = j;
            } else {
                i += 1;
            }
        }
        Ok(out)
    }

    // registers ------------------------------------------------------------------

    fn get(&self, i: isize) -> PyResult<u128> {
        Ok(self.cpu.r[index(i, 32)?])
    }

    #[pyo3(signature = (i, v, bits = 64))]
    fn set(&mut self, i: isize, v: &Bound<'_, PyAny>, bits: i64) -> PyResult<()> {
        if i != 0 {
            let k = index(i, 32)?;
            self.cpu.r[k] = if bits == 128 { low128(v)? } else { (self.cpu.r[k] & !M64) | u128::from(low64(v)?) };
        }
        Ok(())
    }

    fn set32(&mut self, i: isize, v: &Bound<'_, PyAny>) -> PyResult<()> {
        if i != 0 {
            let k = index(i, 32)?;
            let v = low32(v)? as i32 as i64 as u64;
            self.cpu.r[k] = (self.cpu.r[k] & !M64) | u128::from(v);
        }
        Ok(())
    }

    /// The VuMachine's `vset`: the lanes of vf`i` that `mask` names.
    fn vset(&mut self, i: isize, mask: u32, vals: &Bound<'_, PyAny>) -> PyResult<()> {
        if i != 0 {
            let k = index(i, 32)?;
            for lane in 0..4 {
                if mask & (8 >> lane) != 0 {
                    self.cpu.vf[k][lane] = low32(&vals.get_item(lane)?)?;
                }
            }
        }
        Ok(())
    }

    /// The VuMachine's effective address for a word.
    fn ea(&self, w: u32) -> u32 {
        self.cpu.ea(w)
    }

    // execution ---------------------------------------------------------------------

    /// `call(addr, args=(), limit=5_000_000)`: run from `addr` with up to
    /// eight arguments until it returns; `$v0`'s low word.
    #[pyo3(signature = (addr, args = None, limit = 5_000_000))]
    fn call(
        slf: &Bound<'_, Self>,
        addr: &Bound<'_, PyAny>,
        args: Option<&Bound<'_, PyAny>>,
        limit: i64,
    ) -> PyResult<u32> {
        let py = slf.py();
        let addr = low64(addr)?;
        let mut words = Vec::with_capacity(8);
        if let Some(args) = args {
            for a in args.try_iter()?.take(8) {
                words.push(low32(&a?)?);
            }
        }
        let opts = run_opts(slf)?;
        let gp = slf.borrow().p.bind(py).getattr("gp")?;
        let gp = if gp.is_none() { 0 } else { low128(&gp)? };
        {
            let mut m = slf.borrow_mut();
            m.cpu.begin(gp, &words);
            m.depth += 1;
            m.traps = opts.traps;
        }
        let mut run = Run::new(addr, limit.max(0) as u64);
        let result = drive(slf, &mut run, opts, limit);
        let mut m = slf.borrow_mut();
        m.depth -= 1;
        result.map(|_| m.cpu.r[2] as u32)
    }

    /// One instruction: None, the branch target, or ANNUL.
    fn exec(slf: &Bound<'_, Self>, pc: u64, w: u32) -> PyResult<Py<PyAny>> {
        let py = slf.py();
        let cached = {
            let m = slf.borrow();
            (m.depth > 0).then_some(m.traps)
        };
        let traps = match cached {
            Some(t) => t,
            None => run_opts(slf)?.traps,
        };
        let r = {
            let mut m = slf.borrow_mut();
            if m.depth == 0 {
                // Called on its own: `writes` is this instruction's.
                m.cpu.writes = m.trace.is_some().then(Vec::new);
            }
            let (cpu, mut mem) = m.parts();
            cpu.exec(&mut mem, pc, w, traps)
        };
        match r {
            Ok(flow) => flow_to_py(py, flow),
            Err(Fault::Trap(t)) => {
                let name = if t == Trap::Cop2 { "cop2" } else { "mmi" };
                Ok(slf.call_method1(name, (pc, w))?.unbind())
            }
            Err(f) => Err(fault(slf, f, 0)),
        }
    }

    /// The VuMachine's `cop2`, native (what a Python override's `super()`
    /// reaches).
    fn cop2(slf: &Bound<'_, Self>, pc: u64, w: u32) -> PyResult<Py<PyAny>> {
        let r = {
            let mut m = slf.borrow_mut();
            let (cpu, mut mem) = m.parts();
            cpu.cop2(&mut mem, pc, w)
        };
        r.map_or_else(|f| Err(fault(slf, f, 0)), |flow| flow_to_py(slf.py(), flow))
    }

    /// The VuMachine's `mmi`, native.
    fn mmi(slf: &Bound<'_, Self>, pc: u64, w: u32) -> PyResult<Py<PyAny>> {
        let r = slf.borrow_mut().cpu.mmi(pc, w);
        r.map_or_else(|f| Err(fault(slf, f, 0)), |flow| flow_to_py(slf.py(), flow))
    }

    fn cop1_s(slf: &Bound<'_, Self>, pc: u64, w: u32, ft: usize, fs: usize, fd: usize) -> PyResult<()> {
        if ft.max(fs).max(fd) > 31 {
            return Err(PyIndexError::new_err("list index out of range"));
        }
        let r = slf.borrow_mut().cpu.cop1_s(pc, w, ft, fs, fd);
        r.map(|_| ()).map_err(|f| fault(slf, f, 0))
    }

    #[pyo3(signature = (pc, simm, cond, likely = false))]
    fn branch(&self, py: Python<'_>, pc: i64, simm: i64, cond: &Bound<'_, PyAny>, likely: bool) -> PyResult<Py<PyAny>> {
        if cond.is_truthy()? {
            return (pc.wrapping_add(4).wrapping_add(simm << 2) as u32).into_py_any(py);
        }
        flow_to_py(py, if likely { Flow::Annul } else { Flow::Next })
    }

    fn bad(slf: &Bound<'_, Self>, pc: u64, w: u32) -> PyResult<()> {
        Err(fault(slf, Fault::Bad { pc, w }, 0))
    }

    /// Everything but memory, as one tuple: (r, f, hi, lo, hi1, lo1, fcr31,
    /// acc, steps, vf, vacc, q, vi), sequences as tuples.
    fn state<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let c = &self.cpu;
        let vf = PyTuple::new(py, c.vf.iter().map(|row| PyTuple::new(py, row).unwrap()))?;
        PyTuple::new(
            py,
            [
                PyTuple::new(py, c.r)?.into_any(),
                PyTuple::new(py, c.f)?.into_any(),
                c.hi.into_bound_py_any(py)?,
                c.lo.into_bound_py_any(py)?,
                c.hi1.into_bound_py_any(py)?,
                c.lo1.into_bound_py_any(py)?,
                c.fcr31.into_bound_py_any(py)?,
                c.acc.into_bound_py_any(py)?,
                c.steps.into_bound_py_any(py)?,
                vf.into_any(),
                PyTuple::new(py, c.vacc)?.into_any(),
                c.q.into_bound_py_any(py)?,
                PyTuple::new(py, c.vi)?.into_any(),
            ],
        )
    }

    // attributes ----------------------------------------------------------------------

    #[getter]
    fn mem<'py>(&self, py: Python<'py>) -> Bound<'py, PyByteArray> {
        self.ram.obj.bind(py).clone()
    }

    #[setter(mem)]
    fn set_mem(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.ram = Buffer::adopt(v, RAM, "mem")?;
        Ok(())
    }

    #[getter]
    fn vumem<'py>(&self, py: Python<'py>) -> Bound<'py, PyByteArray> {
        self.vu.obj.bind(py).clone()
    }

    #[setter(vumem)]
    fn set_vumem(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.vu = Buffer::adopt(v, VU_MEM, "vumem")?;
        Ok(())
    }

    #[getter]
    fn hooks(&self, py: Python<'_>) -> Py<Hooks> {
        self.hooks.clone_ref(py)
    }

    /// A Hooks object is shared; anything else is copied into a new one.
    #[setter(hooks)]
    fn set_hooks(&mut self, py: Python<'_>, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.hooks = match v.cast::<Hooks>() {
            Ok(h) => h.clone().unbind(),
            Err(_) => {
                let mut h = Hooks::empty(py);
                h.update(py, &PyTuple::new(py, [v])?, None)?;
                Py::new(py, h)?
            }
        };
        Ok(())
    }

    #[getter]
    fn p(&self, py: Python<'_>) -> Py<PyAny> {
        self.p.clone_ref(py)
    }

    #[setter(p)]
    fn set_p(&mut self, v: Py<PyAny>) {
        self.p = v;
    }

    #[getter]
    fn pristine(&self, py: Python<'_>) -> Py<PyAny> {
        self.pristine.clone_ref(py)
    }

    #[setter(pristine)]
    fn set_pristine(&mut self, v: Py<PyAny>) {
        self.pristine = v;
    }

    /// `trace(pc, word, next_pc, next_npc)` after every step (word None for
    /// a hook), or None.
    #[getter]
    fn trace(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.trace.as_ref().map(|t| t.clone_ref(py))
    }

    #[setter(trace)]
    fn set_trace(&mut self, v: Option<Py<PyAny>>) {
        self.trace = v;
    }

    /// While tracing: [(address, length)] of the RAM the current step (or
    /// a lone `exec`) wrote.
    #[getter]
    fn writes(&self) -> Option<Vec<(u32, u32)>> {
        self.cpu.writes.clone()
    }

    #[getter]
    fn features<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        let f = self.cpu.features;
        d.set_item("vu", f.vu)?;
        d.set_item("vi", f.vi)?;
        d.set_item("ee_div", f.ee_div)?;
        Ok(d)
    }

    #[getter]
    fn r(slf: &Bound<'_, Self>) -> RegView {
        RegView { m: slf.clone().unbind(), which: Regs::R }
    }

    #[setter(r)]
    fn set_r(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.r = values(v, low128)?;
        Ok(())
    }

    #[getter]
    fn f(slf: &Bound<'_, Self>) -> RegView {
        RegView { m: slf.clone().unbind(), which: Regs::F }
    }

    #[setter(f)]
    fn set_f(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.f = values(v, low32)?;
        Ok(())
    }

    #[getter]
    fn vf(slf: &Bound<'_, Self>) -> VfView {
        VfView { m: slf.clone().unbind() }
    }

    #[setter(vf)]
    fn set_vf(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.vf = values(v, |row| values(row, low32))?;
        Ok(())
    }

    #[getter]
    fn vacc(slf: &Bound<'_, Self>) -> RegView {
        RegView { m: slf.clone().unbind(), which: Regs::Vacc }
    }

    #[setter(vacc)]
    fn set_vacc(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.vacc = values(v, low32)?;
        Ok(())
    }

    #[getter]
    fn vi(slf: &Bound<'_, Self>) -> RegView {
        RegView { m: slf.clone().unbind(), which: Regs::Vi }
    }

    #[setter(vi)]
    fn set_vi(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.vi = values(v, |x| Ok(low32(x)? as u16))?;
        Ok(())
    }

    #[getter]
    fn hi(&self) -> u128 {
        self.cpu.hi
    }

    #[setter(hi)]
    fn set_hi(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.hi = low128(v)?;
        Ok(())
    }

    #[getter]
    fn lo(&self) -> u128 {
        self.cpu.lo
    }

    #[setter(lo)]
    fn set_lo(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.lo = low128(v)?;
        Ok(())
    }

    #[getter]
    fn hi1(&self) -> u128 {
        self.cpu.hi1
    }

    #[setter(hi1)]
    fn set_hi1(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.hi1 = low128(v)?;
        Ok(())
    }

    #[getter]
    fn lo1(&self) -> u128 {
        self.cpu.lo1
    }

    #[setter(lo1)]
    fn set_lo1(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.lo1 = low128(v)?;
        Ok(())
    }

    #[getter]
    fn fcr31(&self) -> u32 {
        self.cpu.fcr31
    }

    #[setter(fcr31)]
    fn set_fcr31(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.fcr31 = low32(v)?;
        Ok(())
    }

    #[getter]
    fn acc(&self) -> u32 {
        self.cpu.acc
    }

    #[setter(acc)]
    fn set_acc(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.acc = low32(v)?;
        Ok(())
    }

    #[getter]
    fn q(&self) -> u32 {
        self.cpu.q
    }

    #[setter(q)]
    fn set_q(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.q = low32(v)?;
        Ok(())
    }

    #[getter]
    fn steps(&self) -> u64 {
        self.cpu.steps
    }

    #[setter(steps)]
    fn set_steps(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cpu.steps = low64(v)?;
        Ok(())
    }
}

/// test_anim's VuMachine: `VuMachine(program, *, ee_div=False)`.
#[pyclass(module = "eemu_rs", name = "VuMachine", extends = PyMachine, subclass, unsendable)]
pub struct VuMachine;

#[pymethods]
impl VuMachine {
    #[new]
    #[pyo3(signature = (program, *, ee_div = false))]
    fn new(py: Python<'_>, program: &Bound<'_, PyAny>, ee_div: bool) -> PyResult<PyClassInitializer<Self>> {
        let m = PyMachine::create(py, program, Features { vu: true, vi: false, ee_div })?;
        Ok(PyClassInitializer::from(m).add_subclass(VuMachine))
    }
}

/// test_stream_rs's Vu0Machine: `Vu0Machine(program, *, ee_div=False)`.
#[pyclass(module = "eemu_rs", name = "Vu0Machine", extends = VuMachine, subclass, unsendable)]
pub struct Vu0Machine;

#[pymethods]
impl Vu0Machine {
    #[new]
    #[pyo3(signature = (program, *, ee_div = false))]
    fn new(py: Python<'_>, program: &Bound<'_, PyAny>, ee_div: bool) -> PyResult<PyClassInitializer<Self>> {
        let m = PyMachine::create(py, program, Features { vu: true, vi: true, ee_div })?;
        Ok(PyClassInitializer::from(m).add_subclass(VuMachine).add_subclass(Vu0Machine))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Regs {
    R,
    F,
    Vacc,
    Vi,
    Vf(usize),
}

/// A live view of a register file: indexes, slices, iteration, `len`,
/// comparison with lists. `list(view)` takes a copy.
#[pyclass(module = "eemu_rs", name = "Registers", sequence, unsendable)]
pub struct RegView {
    m: Py<PyMachine>,
    which: Regs,
}

impl RegView {
    fn len(&self) -> usize {
        match self.which {
            Regs::R | Regs::F => 32,
            Regs::Vacc | Regs::Vf(_) => 4,
            Regs::Vi => 16,
        }
    }

    fn at<'py>(&self, py: Python<'py>, k: usize) -> PyResult<Bound<'py, PyAny>> {
        let m = self.m.bind(py).borrow();
        let c = &m.cpu;
        match self.which {
            Regs::R => c.r[k].into_bound_py_any(py),
            Regs::F => c.f[k].into_bound_py_any(py),
            Regs::Vacc => c.vacc[k].into_bound_py_any(py),
            Regs::Vi => c.vi[k].into_bound_py_any(py),
            Regs::Vf(i) => c.vf[i][k].into_bound_py_any(py),
        }
    }

    fn put(&self, py: Python<'_>, k: usize, v: &Bound<'_, PyAny>) -> PyResult<()> {
        let mut m = self.m.bind(py).borrow_mut();
        let c = &mut m.cpu;
        match self.which {
            Regs::R => c.r[k] = low128(v)?,
            Regs::F => c.f[k] = low32(v)?,
            Regs::Vacc => c.vacc[k] = low32(v)?,
            Regs::Vi => c.vi[k] = low32(v)? as u16,
            Regs::Vf(i) => c.vf[i][k] = low32(v)?,
        }
        Ok(())
    }

    fn list<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, (0..self.len()).map(|k| self.at(py, k)).collect::<PyResult<Vec<_>>>()?)
    }

    /// The positions a slice selects.
    fn positions(&self, s: &Bound<'_, PySlice>) -> PyResult<Vec<usize>> {
        let ix = s.indices(self.len() as isize)?;
        Ok((0..ix.slicelength).map(|j| (ix.start + j as isize * ix.step) as usize).collect())
    }
}

#[pymethods]
impl RegView {
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.m)
    }

    fn __len__(&self) -> usize {
        self.len()
    }

    fn __getitem__<'py>(&self, py: Python<'py>, idx: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        if let Ok(s) = idx.cast::<PySlice>() {
            let items = self.positions(s)?.into_iter().map(|k| self.at(py, k)).collect::<PyResult<Vec<_>>>()?;
            return Ok(PyList::new(py, items)?.into_any());
        }
        self.at(py, index(idx.extract()?, self.len())?)
    }

    fn __setitem__(&self, py: Python<'_>, idx: &Bound<'_, PyAny>, v: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(s) = idx.cast::<PySlice>() {
            let at = self.positions(s)?;
            let vals: Vec<Bound<'_, PyAny>> = v.try_iter()?.collect::<PyResult<_>>()?;
            if vals.len() != at.len() {
                return Err(PyValueError::new_err(format!(
                    "a register file cannot change size: {} values for {} registers",
                    vals.len(),
                    at.len()
                )));
            }
            return at.into_iter().zip(vals).try_for_each(|(k, x)| self.put(py, k, &x));
        }
        self.put(py, index(idx.extract()?, self.len())?, v)
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        self.list(py)?.try_iter()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(self.list(py)?.repr()?.to_string())
    }

    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(it) = other.try_iter() else { return Ok(false) };
        let other = PyList::new(py, it.collect::<PyResult<Vec<_>>>()?)?;
        self.list(py)?.eq(other)
    }
}

/// The VU0 float registers: `vf[i]` is a live view of vf`i`'s four lanes.
#[pyclass(module = "eemu_rs", name = "VfRegisters", sequence, unsendable)]
pub struct VfView {
    m: Py<PyMachine>,
}

#[pymethods]
impl VfView {
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.m)
    }

    fn __len__(&self) -> usize {
        32
    }

    fn __getitem__<'py>(&self, py: Python<'py>, idx: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let row = |k| RegView { m: self.m.clone_ref(py), which: Regs::Vf(k) };
        if let Ok(s) = idx.cast::<PySlice>() {
            let ix = s.indices(32)?;
            let rows = (0..ix.slicelength)
                .map(|j| Bound::new(py, row((ix.start + j as isize * ix.step) as usize)))
                .collect::<PyResult<Vec<_>>>()?;
            return Ok(PyList::new(py, rows)?.into_any());
        }
        Ok(Bound::new(py, row(index(idx.extract()?, 32)?))?.into_any())
    }

    fn __setitem__(&self, py: Python<'_>, i: isize, v: &Bound<'_, PyAny>) -> PyResult<()> {
        let row: [u32; 4] = values(v, low32)?;
        self.m.bind(py).borrow_mut().cpu.vf[index(i, 32)?] = row;
        Ok(())
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        let rows = (0..32)
            .map(|k| Bound::new(py, RegView { m: self.m.clone_ref(py), which: Regs::Vf(k) }))
            .collect::<PyResult<Vec<_>>>()?;
        PyList::new(py, rows)?.try_iter()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let m = self.m.bind(py).borrow();
        Ok(format!("{:?}", m.cpu.vf))
    }
}

/// test_anim's `machine_class()`: the VuMachine class.
#[pyfunction]
fn machine_class(py: Python<'_>) -> Bound<'_, PyType> {
    py.get_type::<VuMachine>()
}

/// eemu's `run_ctors`: a Machine with the overlay's constructor table run,
/// as mwLoadOverlay -> __initialize_cpp_rts does.
#[pyfunction]
fn run_ctors<'py>(py: Python<'py>, program: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyMachine>> {
    let m = Bound::new(py, PyMachine::create(py, program, Features::default())?)?;
    let ov = program.getattr("overlay")?;
    if ov.is_none() {
        return Err(PyValueError::new_err("run_ctors needs an overlay"));
    }
    let (start, end): (i64, i64) = (ov.getattr("ctor_start")?.extract()?, ov.getattr("ctor_end")?.extract()?);
    for va in (start..end).step_by(4) {
        let f = m.borrow().load(py, &va.into_bound_py_any(py)?, 4, false)?;
        PyMachine::call(&m, &f, None, 5_000_000)?;
    }
    Ok(m)
}

#[pymodule]
fn eemu_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let e = eemu(py)?;
    // eemu's module-level names, so `from eemu_rs import Stop, sx, f_to_py,
    // _cstr, ...` works wherever `from eemu import ...` did.
    for (k, v) in e.module.bind(py).dict().iter() {
        let name: String = k.extract()?;
        if name.starts_with("__")
            || v.is_instance_of::<PyModule>()
            || matches!(name.as_str(), "Machine" | "run_ctors" | "main")
        {
            continue;
        }
        m.setattr(name.as_str(), v)?;
    }
    m.add_class::<PyMachine>()?;
    m.add_class::<VuMachine>()?;
    m.add_class::<Vu0Machine>()?;
    m.add_class::<Hooks>()?;
    m.add_class::<RegView>()?;
    m.add_class::<VfView>()?;
    m.add_function(wrap_pyfunction!(machine_class, m)?)?;
    m.add_function(wrap_pyfunction!(run_ctors, m)?)?;
    Ok(())
}
