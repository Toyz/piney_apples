//! The interpreter: `tools/eemu.py`'s `Machine.exec`, `cop1_s` and `call`
//! loop, `tools/test_anim.py`'s VuMachine (VU0 macro mode, lqc2 / sqc2,
//! ldl / ldr and the MMI instructions it knows) and `tools/test_stream_rs.py`'s
//! Vu0Machine (VU0's integer registers and data memory), each behind a
//! [`Features`] switch so that a machine stops on exactly the instructions
//! its Python counterpart stops on.
//!
//! The state is held the way the Python holds it: 128-bit GPRs, HI / LO as
//! whatever `mthi` / `mtlo` put there (the whole 128-bit register; `mfhi`
//! and `pmaddh` take the bits they use), raw `u32` FPU and VU0 registers,
//! and `pc` / `npc` as 64-bit values the way Python's unbounded ints carry
//! them. Nothing checks alignment, as eemu does not.

use std::collections::HashMap;
use std::fmt;

use crate::fpu;
use crate::hle::{self, Hle, HleError};

/// Flat memory from 0 to 32 MB.
pub const RAM: usize = 32 << 20;
/// `$sp` at a call's start.
pub const STACK_TOP: u32 = (RAM - 0x1000) as u32;
/// `$ra` at a call's start: returning there ends the call.
pub const RETURN_SENTINEL: u64 = 0xffff_fff0;
/// VU0's data memory, as the Vu0Machine models it (256 quadwords).
pub const VU_MEM: usize = 4096;
const M64: u128 = u64::MAX as u128;
const ONE: u32 = 0x3f80_0000;

/// Which of the Python machines this one is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Features {
    /// test_anim's VuMachine: COP2 macro mode, lqc2 / sqc2, ldl / ldr, MMI.
    pub vu: bool,
    /// test_stream_rs's Vu0Machine on top: vi0-15, VU0 data memory, cfc2 /
    /// ctc2, viadd / viaddi, vlqi / vsqi / vlqd / vsqd.
    pub vi: bool,
    /// test_toppage_rs's `ee_machine`: `div` / `divu` by zero leave LO and
    /// HI as the EE does instead of unchanged.
    pub ee_div: bool,
}

/// Instruction classes handed back to the caller instead of executed: a
/// Python subclass overriding `cop2` or `mmi`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traps {
    pub cop2: bool,
    pub mmi: bool,
}

/// What an instruction does to control flow; eemu's `exec` returns None, a
/// target, or ANNUL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Next,
    /// Jump there after the delay slot.
    Jump(u32),
    /// A branch-likely not taken: the delay slot is skipped.
    Annul,
}

/// An instruction class the caller asked to handle itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trap {
    Cop2,
    Mmi,
}

/// Why a run stopped short: eemu's `Stop`s, an HLE function's error, or a
/// trapped instruction class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// `Machine.bad`: "0x{pc:08x} {where}: {insn} is not interpreted".
    Bad {
        pc: u64,
        w: u32,
    },
    /// The VuMachine's own form: "0x{pc:08x}: {insn} is not interpreted".
    VuBad {
        pc: u64,
        w: u32,
    },
    /// "read of {n} bytes at 0x{addr:08x} is outside RAM".
    Read {
        addr: u32,
        n: u32,
    },
    /// "write of {n} bytes at 0x{addr:08x} is outside RAM".
    Write {
        addr: u32,
        n: u32,
    },
    /// "gave up after {limit} steps at 0x{pc:08x}".
    Limit {
        limit: u64,
        pc: u64,
    },
    Hle(HleError),
    Trap(Trap),
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Fault::Bad { pc, w } | Fault::VuBad { pc, w } => {
                write!(f, "0x{pc:08x}: word 0x{w:08x} is not interpreted")
            }
            Fault::Read { addr, n } => write!(f, "read of {n} bytes at 0x{addr:08x} is outside RAM"),
            Fault::Write { addr, n } => write!(f, "write of {n} bytes at 0x{addr:08x} is outside RAM"),
            Fault::Limit { limit, pc } => write!(f, "gave up after {limit} steps at 0x{pc:08x}"),
            Fault::Hle(HleError::NoTerminator) => write!(f, "subsection not found"),
            Fault::Hle(HleError::Resize) => write!(f, "Existing exports of data: object cannot be re-sized"),
            Fault::Trap(t) => write!(f, "{t:?} trapped"),
        }
    }
}

impl std::error::Error for Fault {}

impl From<HleError> for Fault {
    fn from(e: HleError) -> Fault {
        Fault::Hle(e)
    }
}

/// The memories an instruction can touch: RAM ([`RAM`] bytes) and VU0's
/// data memory ([`VU_MEM`] bytes, only the Vu0Machine's instructions).
pub struct Mem<'a> {
    pub ram: &'a mut [u8],
    pub vu: &'a mut [u8],
}

/// The hooked addresses: eemu's `hooks` dict as far as the run loop needs
/// it. A bitmap over RAM's words answers the common "no" in one load.
#[derive(Clone)]
pub struct HookSet {
    bits: Vec<u64>,
    map: HashMap<u64, HookKind>,
}

/// What sits at a hooked address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookKind {
    /// One of eemu's HLE functions, run natively.
    Native(Hle),
    /// Anything else: the caller runs it.
    Foreign,
}

impl Default for HookSet {
    fn default() -> Self {
        HookSet { bits: vec![0; RAM / 4 / 64], map: HashMap::new() }
    }
}

impl HookSet {
    pub fn insert(&mut self, addr: u64, kind: HookKind) {
        if addr < RAM as u64 {
            let word = (addr >> 2) as usize;
            self.bits[word >> 6] |= 1 << (word & 63);
        }
        self.map.insert(addr, kind);
    }

    pub fn remove(&mut self, addr: u64) {
        if self.map.remove(&addr).is_some() && addr < RAM as u64 {
            let word = addr & !3;
            if (word..word + 4).all(|a| !self.map.contains_key(&a)) {
                let word = (addr >> 2) as usize;
                self.bits[word >> 6] &= !(1 << (word & 63));
            }
        }
    }

    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.map.clear();
    }

    #[inline]
    pub fn get(&self, pc: u64) -> Option<HookKind> {
        if pc < RAM as u64 {
            let word = (pc >> 2) as usize;
            if self.bits[word >> 6] >> (word & 63) & 1 == 0 {
                return None;
            }
        }
        self.map.get(&pc).copied()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// A call in progress: eemu's loop variables.
#[derive(Clone, Copy, Debug)]
pub struct Run {
    pub pc: u64,
    pub npc: u64,
    pub limit: u64,
}

impl Run {
    pub fn new(addr: u64, limit: u64) -> Run {
        Run { pc: addr, npc: addr + 4, limit }
    }

    /// `pc, npc = npc, target or npc + 4`, or past the delay slot for ANNUL.
    #[inline]
    pub fn advance(&mut self, flow: Flow) {
        match flow {
            Flow::Next => (self.pc, self.npc) = (self.npc, self.npc + 4),
            Flow::Jump(t) => (self.pc, self.npc) = (self.npc, u64::from(t)),
            Flow::Annul => (self.pc, self.npc) = (self.npc + 4, self.npc + 8),
        }
    }

    /// After a hook: `pc = $ra & 0xffffffff`.
    pub fn resume(&mut self, ra: u128) {
        self.pc = u64::from(ra as u32);
        self.npc = self.pc + 4;
    }
}

/// How [`Cpu::run`] should run.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunOpts {
    pub traps: Traps,
    /// Hand every instruction back ([`Event::Exec`]): a Python `exec` override.
    pub exec_trap: bool,
    /// Stop after every step ([`Event::Step`]).
    pub trace: bool,
    /// Come back every [`TICK`] steps ([`Event::Tick`]), so a caller can
    /// check for Ctrl-C during a long run.
    pub ticks: bool,
}

/// Steps between [`Event::Tick`]s.
pub const TICK: u64 = 1 << 24;

/// Why [`Cpu::run`] returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The call returned to [`RETURN_SENTINEL`].
    Done,
    /// A foreign hook at `pc`: the caller runs it, sets `$v0` and resumes.
    Hook,
    /// A trapped instruction class at `pc`, not executed.
    Trap(Trap, u32),
    /// The instruction at `pc`, fetched but not executed.
    Exec(u32),
    /// One step done (tracing): its pc, and its word (None for a native hook).
    Step(u64, Option<u32>),
    /// [`TICK`] more steps done.
    Tick,
}

/// Everything the Python machines hold besides memory.
#[derive(Clone, Debug)]
pub struct Cpu {
    pub r: [u128; 32],
    pub f: [u32; 32],
    pub hi: u128,
    pub lo: u128,
    /// The EE's second multiply / divide pipeline.
    pub hi1: u128,
    pub lo1: u128,
    pub fcr31: u32,
    /// The FPU accumulator (adda.s, madd.s ...).
    pub acc: u32,
    pub steps: u64,
    pub vf: [[u32; 4]; 32],
    pub vacc: [u32; 4],
    pub q: u32,
    pub vi: [u16; 16],
    pub features: Features,
    /// When set, every write to RAM (address, length) is appended here.
    pub writes: Option<hle::Writes>,
}

#[inline]
fn fits(addr: u32, n: u32) -> bool {
    u64::from(addr) + u64::from(n) <= RAM as u64
}

/// `load(addr, n)` for n <= 16, little-endian.
#[inline]
pub fn load(ram: &[u8], addr: u32, n: u32) -> Result<u128, Fault> {
    if !fits(addr, n) {
        return Err(Fault::Read { addr, n });
    }
    let a = addr as usize;
    Ok(match n {
        1 => u128::from(ram[a]),
        2 => u128::from(u16::from_le_bytes([ram[a], ram[a + 1]])),
        4 => u128::from(u32::from_le_bytes(ram[a..a + 4].try_into().unwrap())),
        8 => u128::from(u64::from_le_bytes(ram[a..a + 8].try_into().unwrap())),
        16 => u128::from_le_bytes(ram[a..a + 16].try_into().unwrap()),
        _ => ram[a..a + n as usize].iter().rev().fold(0, |v, &b| v << 8 | u128::from(b)),
    })
}

#[inline]
fn load32(ram: &[u8], addr: u32) -> Result<u32, Fault> {
    if !fits(addr, 4) {
        return Err(Fault::Read { addr, n: 4 });
    }
    let a = addr as usize;
    Ok(u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()))
}

/// eemu's `sx(v, 32) & M64`: the low word sign-extended to 64 bits.
#[inline]
fn sext32(v: u64) -> u128 {
    u128::from(v as u32 as i32 as i64 as u64)
}

fn words(v: u128) -> [u32; 4] {
    std::array::from_fn(|k| (v >> (32 * k)) as u32)
}

fn from_words(w: [u32; 4]) -> u128 {
    w.iter().rev().fold(0, |v, &x| v << 32 | u128::from(x))
}

fn halves(v: u128) -> [u16; 8] {
    std::array::from_fn(|k| (v >> (16 * k)) as u16)
}

fn from_halves(h: [u16; 8]) -> u128 {
    h.iter().rev().fold(0, |v, &x| v << 16 | u128::from(x))
}

/// VU0 lane arithmetic, test_anim's `arith` kinds.
#[derive(Clone, Copy)]
enum Op {
    Add,
    Sub,
    Madd,
    Msub,
    Max,
    Mini,
    Mul,
}

const BC_OPS: [Op; 7] = [Op::Add, Op::Sub, Op::Madd, Op::Msub, Op::Max, Op::Mini, Op::Mul];

/// The MMI functs the VuMachine takes over from eemu's base MMI handling.
fn vu_mmi(funct: u32) -> bool {
    matches!(funct, 0x08 | 0x09 | 0x28 | 0x29 | 0x34 | 0x36 | 0x37 | 0x3c | 0x3e | 0x3f)
}

impl Cpu {
    pub fn new(features: Features) -> Cpu {
        let mut vf = [[0; 4]; 32];
        vf[0] = [0, 0, 0, ONE];
        Cpu {
            r: [0; 32],
            f: [0; 32],
            hi: 0,
            lo: 0,
            hi1: 0,
            lo1: 0,
            fcr31: 0,
            acc: 0,
            steps: 0,
            vf,
            vacc: [0; 4],
            q: 0,
            vi: [0; 16],
            features,
            writes: None,
        }
    }

    // registers ---------------------------------------------------------------

    #[inline]
    fn a32(&self, i: usize) -> i32 {
        self.r[i] as u32 as i32
    }

    #[inline]
    fn u32r(&self, i: usize) -> u32 {
        self.r[i] as u32
    }

    #[inline]
    fn a64(&self, i: usize) -> i64 {
        self.r[i] as u64 as i64
    }

    #[inline]
    fn m64(&self, i: usize) -> u64 {
        self.r[i] as u64
    }

    /// eemu's `set(i, v)`: the low 64 bits, the upper 64 kept.
    #[inline]
    pub fn set(&mut self, i: usize, v: u64) {
        if i != 0 {
            self.r[i] = (self.r[i] & !M64) | u128::from(v);
        }
    }

    /// `set(i, v, bits=128)`.
    #[inline]
    pub fn set128(&mut self, i: usize, v: u128) {
        if i != 0 {
            self.r[i] = v;
        }
    }

    /// `set32(i, v)`: sign-extended to 64 bits.
    #[inline]
    pub fn set32(&mut self, i: usize, v: u32) {
        self.set(i, v as i32 as i64 as u64);
    }

    /// `vset(i, mask, vals)`: the lanes `mask` names (x = 8 ... w = 1); vf0
    /// is never written.
    pub fn vset(&mut self, i: usize, mask: u32, vals: [u32; 4]) {
        if i != 0 {
            for (k, v) in vals.into_iter().enumerate() {
                if mask & (8 >> k) != 0 {
                    self.vf[i][k] = v;
                }
            }
        }
    }

    /// The Vu0Machine's `vset_i`: vi0 stays 0.
    fn vset_i(&mut self, i: u32, v: u32) {
        if i & 15 != 0 {
            self.vi[(i & 15) as usize] = v as u16;
        }
    }

    /// a0-a3 as a hook sees them.
    pub fn args(&self) -> [u32; 4] {
        std::array::from_fn(|i| self.r[4 + i] as u32)
    }

    /// `call`'s set-up: fresh GPRs, `$gp`, `$sp`, the return sentinel in
    /// `$ra`, up to eight arguments (a0-a3, t0-t3), and the step count reset.
    pub fn begin(&mut self, gp: u128, args: &[u32]) {
        self.r = [0; 32];
        self.r[28] = gp;
        self.r[29] = u128::from(STACK_TOP);
        self.r[31] = u128::from(RETURN_SENTINEL);
        for (i, &a) in args.iter().take(8).enumerate() {
            self.set32(4 + i, a);
        }
        self.steps = 0;
    }

    // memory -------------------------------------------------------------------

    /// `store(addr, n, v)` for n <= 16.
    #[inline]
    pub fn store(&mut self, ram: &mut [u8], addr: u32, n: u32, v: u128) -> Result<(), Fault> {
        if !fits(addr, n) {
            return Err(Fault::Write { addr, n });
        }
        let a = addr as usize;
        ram[a..a + n as usize].copy_from_slice(&v.to_le_bytes()[..n as usize]);
        if let Some(w) = &mut self.writes {
            w.push((addr, n));
        }
        Ok(())
    }

    // execution ----------------------------------------------------------------

    /// Run until the call returns or something needs the caller: eemu's
    /// `call` loop from `run.pc` on. Native HLE hooks run here.
    pub fn run(&mut self, m: &mut Mem, hooks: &HookSet, run: &mut Run, opts: RunOpts) -> Result<Event, Fault> {
        loop {
            if run.pc == RETURN_SENTINEL {
                return Ok(Event::Done);
            }
            self.steps += 1;
            if self.steps > run.limit {
                return Err(Fault::Limit { limit: run.limit, pc: run.pc });
            }
            let pc = run.pc;
            let w = match hooks.get(pc) {
                Some(HookKind::Foreign) => return Ok(Event::Hook),
                Some(HookKind::Native(h)) => {
                    let ret = hle::call(h, m.ram, self.args(), self.writes.as_mut())?;
                    self.set32(2, ret as u32);
                    run.resume(self.r[31]);
                    None
                }
                None => {
                    let w = load32(m.ram, pc as u32)?;
                    if opts.exec_trap {
                        return Ok(Event::Exec(w));
                    }
                    match self.exec(m, pc, w, opts.traps) {
                        Ok(flow) => run.advance(flow),
                        Err(Fault::Trap(t)) => return Ok(Event::Trap(t, w)),
                        Err(e) => return Err(e),
                    }
                    Some(w)
                }
            };
            if opts.trace {
                return Ok(Event::Step(pc, w));
            }
            if opts.ticks && self.steps & (TICK - 1) == 0 {
                return Ok(Event::Tick);
            }
        }
    }

    /// One instruction: eemu's `exec` with the machine's features, trapped
    /// instruction classes returned as [`Fault::Trap`] before they touch
    /// anything.
    pub fn exec(&mut self, m: &mut Mem, pc: u64, w: u32, traps: Traps) -> Result<Flow, Fault> {
        let op = w >> 26;
        if self.features.ee_div && op == 0 && matches!(w & 63, 0x1a | 0x1b) {
            let (rs, rt) = (((w >> 21) & 31) as usize, ((w >> 16) & 31) as usize);
            if self.u32r(rt) == 0 {
                let n = self.a32(rs);
                let q: i64 = if w & 63 == 0x1a && n < 0 { 1 } else { -1 };
                self.lo = u128::from(q as u64);
                self.hi = u128::from(i64::from(n) as u64);
                return Ok(Flow::Next);
            }
        }
        if self.features.vu {
            match op {
                0x12 => {
                    return if traps.cop2 { Err(Fault::Trap(Trap::Cop2)) } else { self.cop2(m, pc, w) };
                }
                0x36 => {
                    let v = load(m.ram, self.ea(w) & !15, 16)?;
                    self.vset(((w >> 16) & 31) as usize, 0xf, words(v));
                    return Ok(Flow::Next);
                }
                0x3e => {
                    let v = from_words(self.vf[((w >> 16) & 31) as usize]);
                    self.store(m.ram, self.ea(w) & !15, 16, v)?;
                    return Ok(Flow::Next);
                }
                0x1a | 0x1b => {
                    let (rt, ea) = (((w >> 16) & 31) as usize, self.ea(w));
                    let k = ea & 7;
                    let dw = load(m.ram, ea & !7, 8)? as u64;
                    let old = self.m64(rt);
                    let v = if op == 0x1a {
                        let sh = 8 * (7 - k);
                        (dw << sh) | (old & ((1u64 << sh) - 1))
                    } else {
                        let sh = 8 * k;
                        (dw >> sh) | (old & if sh == 0 { 0 } else { u64::MAX << (64 - sh) })
                    };
                    self.set(rt, v);
                    return Ok(Flow::Next);
                }
                0x1c if vu_mmi(w & 63) => {
                    return if traps.mmi { Err(Fault::Trap(Trap::Mmi)) } else { self.mmi(pc, w) };
                }
                _ => {}
            }
        }
        self.exec_ee(m.ram, pc, w)
    }

    /// The VuMachine's effective address, `sx(rs, 32) + sx(imm, 16)`.
    #[inline]
    pub fn ea(&self, w: u32) -> u32 {
        (self.a32(((w >> 21) & 31) as usize) as i64 + i64::from(w as u16 as i16)) as u32
    }

    fn branch(pc: u64, simm: i64, cond: bool, likely: bool) -> Flow {
        if cond {
            Flow::Jump((pc as i64).wrapping_add(4).wrapping_add(simm << 2) as u32)
        } else if likely {
            Flow::Annul
        } else {
            Flow::Next
        }
    }

    /// `(lo, hi)` of mult / multu (either pipeline): each half of the product
    /// sign-extended from 32 bits.
    fn multiply(&self, signed: bool, rs: usize, rt: usize) -> (u128, u128) {
        let (v, hi) = if signed {
            let v = i64::from(self.a32(rs)) * i64::from(self.a32(rt));
            (v as u64, (v >> 32) as u64)
        } else {
            let v = u64::from(self.u32r(rs)) * u64::from(self.u32r(rt));
            (v, v >> 32)
        };
        (sext32(v), sext32(hi))
    }

    /// `(lo, hi)` of div / divu (either pipeline), quotient truncated toward
    /// zero; None for a zero divisor, which leaves LO and HI alone.
    fn divide(&self, signed: bool, rs: usize, rt: usize) -> Option<(u128, u128)> {
        let (n, d) = if signed {
            (i64::from(self.a32(rs)), i64::from(self.a32(rt)))
        } else {
            (i64::from(self.u32r(rs)), i64::from(self.u32r(rt)))
        };
        if d == 0 {
            return None;
        }
        let q = n / d;
        Some((sext32(q as u64), sext32((n - q * d) as u64)))
    }

    /// The base Machine's `exec`.
    fn exec_ee(&mut self, ram: &mut [u8], pc: u64, w: u32) -> Result<Flow, Fault> {
        let op = w >> 26;
        let rs = ((w >> 21) & 31) as usize;
        let rt = ((w >> 16) & 31) as usize;
        let rd = ((w >> 11) & 31) as usize;
        let sa = (w >> 6) & 31;
        let imm = w & 0xffff;
        let simm = i64::from(imm as u16 as i16);
        let ea = (i64::from(self.a32(rs)) + simm) as u32;
        let bad = Err(Fault::Bad { pc, w });

        match op {
            0 => {
                match w & 63 {
                    0x00 => self.set32(rd, self.u32r(rt) << sa),
                    0x02 => self.set32(rd, self.u32r(rt) >> sa),
                    0x03 => self.set32(rd, (self.a32(rt) >> sa) as u32),
                    0x04 => self.set32(rd, self.u32r(rt) << (self.r[rs] & 31)),
                    0x06 => self.set32(rd, self.u32r(rt) >> (self.r[rs] & 31)),
                    0x07 => self.set32(rd, (self.a32(rt) >> (self.r[rs] & 31)) as u32),
                    0x08 => return Ok(Flow::Jump(self.u32r(rs))),
                    0x09 => {
                        // The link is written first: jalr rd == rs jumps to it.
                        self.set(rd, pc.wrapping_add(8));
                        return Ok(Flow::Jump(self.u32r(rs)));
                    }
                    0x0a => {
                        if self.m64(rt) == 0 {
                            self.set(rd, self.m64(rs));
                        }
                    }
                    0x0b => {
                        if self.m64(rt) != 0 {
                            self.set(rd, self.m64(rs));
                        }
                    }
                    0x0f => {}
                    0x10 => self.set(rd, self.hi as u64),
                    0x12 => self.set(rd, self.lo as u64),
                    0x11 => self.hi = self.r[rs],
                    0x13 => self.lo = self.r[rs],
                    f @ (0x18 | 0x19) => {
                        (self.lo, self.hi) = self.multiply(f == 0x18, rs, rt);
                        self.set(rd, self.lo as u64);
                    }
                    f @ (0x1a | 0x1b) => {
                        if let Some((lo, hi)) = self.divide(f == 0x1a, rs, rt) {
                            (self.lo, self.hi) = (lo, hi);
                        }
                    }
                    0x20 | 0x21 => self.set32(rd, self.a32(rs).wrapping_add(self.a32(rt)) as u32),
                    0x22 | 0x23 => self.set32(rd, self.a32(rs).wrapping_sub(self.a32(rt)) as u32),
                    0x24 => self.set(rd, self.m64(rs) & self.m64(rt)),
                    0x25 => self.set(rd, self.m64(rs) | self.m64(rt)),
                    0x26 => self.set(rd, self.m64(rs) ^ self.m64(rt)),
                    0x27 => self.set(rd, !(self.m64(rs) | self.m64(rt))),
                    0x2a => self.set(rd, u64::from(self.a64(rs) < self.a64(rt))),
                    0x2b => self.set(rd, u64::from(self.m64(rs) < self.m64(rt))),
                    0x14 => self.set(rd, self.m64(rt) << (self.r[rs] & 63)),
                    0x16 => self.set(rd, self.m64(rt) >> (self.r[rs] & 63)),
                    0x17 => self.set(rd, (self.a64(rt) >> (self.r[rs] & 63)) as u64),
                    0x2c | 0x2d => self.set(rd, self.m64(rs).wrapping_add(self.m64(rt))),
                    0x2e | 0x2f => self.set(rd, self.m64(rs).wrapping_sub(self.m64(rt))),
                    0x38 => self.set(rd, self.m64(rt) << sa),
                    0x3a => self.set(rd, self.m64(rt) >> sa),
                    0x3b => self.set(rd, (self.a64(rt) >> sa) as u64),
                    0x3c => self.set(rd, self.m64(rt) << (sa + 32)),
                    0x3e => self.set(rd, self.m64(rt) >> (sa + 32)),
                    0x3f => self.set(rd, (self.a64(rt) >> (sa + 32)) as u64),
                    _ => return bad,
                }
                Ok(Flow::Next)
            }
            1 => {
                let cond = match rt {
                    0 | 2 | 0x10 => self.a64(rs) < 0,
                    1 | 3 | 0x11 => self.a64(rs) >= 0,
                    _ => return bad,
                };
                if rt >= 0x10 {
                    self.set(31, pc.wrapping_add(8));
                }
                Ok(Self::branch(pc, simm, cond, rt == 2 || rt == 3))
            }
            2 | 3 => {
                if op == 3 {
                    self.set(31, pc.wrapping_add(8));
                }
                Ok(Flow::Jump((((pc + 4) & 0xf000_0000) as u32) | ((w & 0x03ff_ffff) << 2)))
            }
            4..=7 | 0x14..=0x17 => {
                let cond = match op & 3 {
                    0 => self.m64(rs) == self.m64(rt),
                    1 => self.m64(rs) != self.m64(rt),
                    2 => self.a64(rs) <= 0,
                    _ => self.a64(rs) > 0,
                };
                Ok(Self::branch(pc, simm, cond, op >= 0x14))
            }
            0x08 | 0x09 => {
                self.set32(rt, (i64::from(self.a32(rs)) + simm) as u32);
                Ok(Flow::Next)
            }
            0x0a => {
                self.set(rt, u64::from(self.a64(rs) < simm));
                Ok(Flow::Next)
            }
            0x0b => {
                self.set(rt, u64::from(self.m64(rs) < simm as u64));
                Ok(Flow::Next)
            }
            0x0c => {
                self.set(rt, self.m64(rs) & u64::from(imm));
                Ok(Flow::Next)
            }
            0x0d => {
                self.set(rt, self.m64(rs) | u64::from(imm));
                Ok(Flow::Next)
            }
            0x0e => {
                self.set(rt, self.m64(rs) ^ u64::from(imm));
                Ok(Flow::Next)
            }
            0x0f => {
                self.set32(rt, imm << 16);
                Ok(Flow::Next)
            }
            0x18 | 0x19 => {
                self.set(rt, self.a64(rs).wrapping_add(simm) as u64);
                Ok(Flow::Next)
            }
            0x1c => {
                // MMI: only pipeline 1's multiply, divide and moves.
                match w & 63 {
                    f @ (0x18 | 0x19) => {
                        (self.lo1, self.hi1) = self.multiply(f == 0x18, rs, rt);
                        self.set(rd, self.lo1 as u64);
                    }
                    f @ (0x1a | 0x1b) => {
                        if let Some((lo, hi)) = self.divide(f == 0x1a, rs, rt) {
                            (self.lo1, self.hi1) = (lo, hi);
                        }
                    }
                    0x10 => self.set(rd, self.hi1 as u64),
                    0x12 => self.set(rd, self.lo1 as u64),
                    0x11 => self.hi1 = self.r[rs],
                    0x13 => self.lo1 = self.r[rs],
                    _ => return bad,
                }
                Ok(Flow::Next)
            }
            0x11 => {
                match rs {
                    0x00 => self.set32(rt, self.f[rd]),
                    0x02 => self.set32(
                        rt,
                        if rd == 31 {
                            self.fcr31
                        } else if rd == 0 {
                            0x2e30
                        } else {
                            0
                        },
                    ),
                    0x04 => self.f[rd] = self.u32r(rt),
                    0x06 => {
                        if rd == 31 {
                            self.fcr31 = self.u32r(rt);
                        }
                    }
                    0x08 => {
                        if rt > 3 {
                            return bad;
                        }
                        let cond = (self.fcr31 & fpu::C_BIT != 0) == (rt & 1 != 0);
                        return Ok(Self::branch(pc, simm, cond, rt >= 2));
                    }
                    0x10 => return self.cop1_s(pc, w, rt, rd, sa as usize),
                    0x14 if w & 63 == 0x20 => self.f[sa as usize] = fpu::from_int(self.f[rd]),
                    _ => return bad,
                }
                Ok(Flow::Next)
            }
            0x20 => {
                let v = load(ram, ea, 1)? as u8 as i8;
                self.set(rt, i64::from(v) as u64);
                Ok(Flow::Next)
            }
            0x24 => {
                self.set(rt, load(ram, ea, 1)? as u64);
                Ok(Flow::Next)
            }
            0x21 => {
                let v = load(ram, ea, 2)? as u16 as i16;
                self.set(rt, i64::from(v) as u64);
                Ok(Flow::Next)
            }
            0x25 => {
                self.set(rt, load(ram, ea, 2)? as u64);
                Ok(Flow::Next)
            }
            0x23 => {
                self.set32(rt, load32(ram, ea)?);
                Ok(Flow::Next)
            }
            0x27 => {
                self.set(rt, u64::from(load32(ram, ea)?));
                Ok(Flow::Next)
            }
            0x37 => {
                self.set(rt, load(ram, ea, 8)? as u64);
                Ok(Flow::Next)
            }
            0x1e => {
                self.set128(rt, load(ram, ea & !15, 16)?);
                Ok(Flow::Next)
            }
            0x28 => self.store(ram, ea, 1, self.r[rt]).map(|_| Flow::Next),
            0x29 => self.store(ram, ea, 2, self.r[rt]).map(|_| Flow::Next),
            0x2b => self.store(ram, ea, 4, self.r[rt]).map(|_| Flow::Next),
            0x3f => self.store(ram, ea, 8, self.r[rt]).map(|_| Flow::Next),
            0x1f => self.store(ram, ea & !15, 16, self.r[rt]).map(|_| Flow::Next),
            0x31 => {
                self.f[rt] = load32(ram, ea)?;
                Ok(Flow::Next)
            }
            0x39 => self.store(ram, ea, 4, u128::from(self.f[rt])).map(|_| Flow::Next),
            0x2f | 0x33 => Ok(Flow::Next), // cache, pref
            _ => bad,
        }
    }

    /// COP1's S format: eemu's `cop1_s`; `exec` passes the rt, rd and sa
    /// fields as ft, fs and fd.
    pub fn cop1_s(&mut self, pc: u64, w: u32, ft: usize, fs: usize, fd: usize) -> Result<Flow, Fault> {
        let (a, b) = (self.f[fs], self.f[ft]);
        match w & 63 {
            0x00 => self.f[fd] = fpu::add(a, b),
            0x01 => self.f[fd] = fpu::sub(a, b),
            0x02 => self.f[fd] = fpu::mul(a, b),
            0x03 => self.f[fd] = fpu::div(a, b),
            0x04 => self.f[fd] = fpu::sqrt(b), // the EE takes sqrt.s's operand from ft
            0x05 => self.f[fd] = a & 0x7fff_ffff,
            0x06 => self.f[fd] = a,
            0x07 => self.f[fd] = a ^ 0x8000_0000,
            0x16 => self.f[fd] = fpu::rsqrt(a, b),
            0x18 => self.acc = fpu::add(a, b),
            0x19 => self.acc = fpu::sub(a, b),
            0x1a => self.acc = fpu::mul(a, b),
            0x1c => self.f[fd] = fpu::add(self.acc, fpu::mul(a, b)),
            0x1d => self.f[fd] = fpu::sub(self.acc, fpu::mul(a, b)),
            0x1e => self.acc = fpu::add(self.acc, fpu::mul(a, b)),
            0x1f => self.acc = fpu::sub(self.acc, fpu::mul(a, b)),
            0x24 => self.f[fd] = fpu::to_int(a),
            0x28 => self.f[fd] = if fpu::cmp(a, b) >= 0 { a } else { b },
            0x29 => self.f[fd] = if fpu::cmp(a, b) <= 0 { a } else { b },
            f @ (0x30 | 0x32 | 0x34 | 0x36) => {
                let c = fpu::cmp(a, b);
                let cond = match f {
                    0x30 => false,
                    0x32 => c == 0,
                    0x34 => c < 0,
                    _ => c <= 0,
                };
                self.fcr31 = if cond { self.fcr31 | fpu::C_BIT } else { self.fcr31 & !fpu::C_BIT };
            }
            _ => return Err(Fault::Bad { pc, w }),
        }
        Ok(Flow::Next)
    }

    /// COP2 in macro mode: the Vu0Machine's additions first when enabled,
    /// then the VuMachine's `cop2`.
    pub fn cop2(&mut self, m: &mut Mem, pc: u64, w: u32) -> Result<Flow, Fault> {
        if self.features.vi && self.cop2_vi(m.vu, w) {
            return Ok(Flow::Next);
        }
        self.cop2_vu(pc, w)
    }

    /// VU0 data memory quadword `a` (masked to 256 quadwords).
    fn qword(vu: &[u8], a: u16) -> [u32; 4] {
        let a = usize::from(a & 0xff) * 16;
        std::array::from_fn(|k| u32::from_le_bytes(vu[a + 4 * k..a + 4 * k + 4].try_into().unwrap()))
    }

    fn store_qword(vu: &mut [u8], a: u16, mask: u32, vals: [u32; 4]) {
        let a = usize::from(a & 0xff) * 16;
        for (k, v) in vals.into_iter().enumerate() {
            if mask & (8 >> k) != 0 {
                vu[a + 4 * k..a + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
    }

    /// test_stream_rs's Vu0Machine.cop2 before it defers: true if handled.
    fn cop2_vi(&mut self, vu: &mut [u8], w: u32) -> bool {
        let (rs, rt, rd) = ((w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31);
        if rs == 0x02 {
            // cfc2 rt, vi(rd)
            let v = if rd < 16 { self.vi[rd as usize] } else { 0 };
            self.set(rt as usize, u64::from(v));
            return true;
        }
        if rs == 0x06 {
            // ctc2 rt, vi(rd)
            if rd < 16 {
                self.vset_i(rd, self.r[rt as usize] as u32);
            }
            return true;
        }
        if rs & 0x10 == 0 {
            return false;
        }
        let (funct, dest) = (w & 63, (w >> 21) & 0xf);
        let (it, is, id) = ((rt & 15) as usize, (rd & 15) as usize, (w >> 6) & 15);
        match funct {
            0x30 => {
                // viadd
                self.vset_i(id, u32::from(self.vi[is].wrapping_add(self.vi[it])));
                true
            }
            0x32 => {
                // viaddi
                let imm = (w >> 6) & 31;
                let imm = if imm & 16 != 0 { imm as i32 - 32 } else { imm as i32 };
                self.vset_i(rt, (i32::from(self.vi[is]) + imm) as u32);
                true
            }
            0x3c.. => match ((w >> 6) & 31) << 2 | (w & 3) {
                0x34 => {
                    // vlqi vf(ft), (vi(fs)++)
                    let q = Self::qword(vu, self.vi[is]);
                    self.vset(rt as usize, dest, q);
                    self.vset_i(rd, u32::from(self.vi[is]) + 1);
                    true
                }
                0x35 => {
                    // vsqi vf(fs), (vi(ft)++)
                    Self::store_qword(vu, self.vi[it], dest, self.vf[rd as usize]);
                    self.vset_i(rt, u32::from(self.vi[it]) + 1);
                    true
                }
                0x36 => {
                    // vlqd vf(ft), (--vi(fs))
                    self.vset_i(rd, u32::from(self.vi[is]).wrapping_sub(1));
                    let q = Self::qword(vu, self.vi[is]);
                    self.vset(rt as usize, dest, q);
                    true
                }
                0x37 => {
                    // vsqd vf(fs), (--vi(ft))
                    self.vset_i(rt, u32::from(self.vi[it]).wrapping_sub(1));
                    Self::store_qword(vu, self.vi[it], dest, self.vf[rd as usize]);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn arith(&self, op: Op, a: [u32; 4], y: [u32; 4]) -> [u32; 4] {
        std::array::from_fn(|k| match op {
            Op::Add => fpu::add(a[k], y[k]),
            Op::Sub => fpu::sub(a[k], y[k]),
            Op::Mul => fpu::mul(a[k], y[k]),
            Op::Madd => fpu::add(self.vacc[k], fpu::mul(a[k], y[k])),
            Op::Msub => fpu::sub(self.vacc[k], fpu::mul(a[k], y[k])),
            Op::Max => {
                if fpu::cmp(a[k], y[k]) >= 0 {
                    a[k]
                } else {
                    y[k]
                }
            }
            Op::Mini => {
                if fpu::cmp(a[k], y[k]) <= 0 {
                    a[k]
                } else {
                    y[k]
                }
            }
        })
    }

    /// test_anim's VuMachine.cop2.
    fn cop2_vu(&mut self, pc: u64, w: u32) -> Result<Flow, Fault> {
        let (rs, rt, rd) = ((w >> 21) & 31, ((w >> 16) & 31) as usize, ((w >> 11) & 31) as usize);
        if rs == 0x01 {
            // qmfc2
            self.set128(rt, from_words(self.vf[rd]));
            return Ok(Flow::Next);
        }
        if rs == 0x05 {
            // qmtc2
            self.vset(rd, 0xf, words(self.r[rt]));
            return Ok(Flow::Next);
        }
        if rs & 0x10 == 0 {
            return Err(Fault::Bad { pc, w });
        }
        let dest = (w >> 21) & 0xf;
        let (ft, fs, fd) = (rt, rd, ((w >> 6) & 31) as usize);
        let (a, b) = (self.vf[fs], self.vf[ft]);
        let bcv = [b[(w & 3) as usize]; 4];
        let qv = [self.q; 4];
        let vubad = Err(Fault::VuBad { pc, w });
        let funct = w & 63;
        if funct < 0x3c {
            let res = match funct {
                0x00..=0x1b => self.arith(BC_OPS[(funct >> 2) as usize], a, bcv),
                0x1c => self.arith(Op::Mul, a, qv),
                0x20 => self.arith(Op::Add, a, qv),
                0x21 => self.arith(Op::Madd, a, qv),
                0x24 => self.arith(Op::Sub, a, qv),
                0x25 => self.arith(Op::Msub, a, qv),
                0x28 => self.arith(Op::Add, a, b),
                0x29 => self.arith(Op::Madd, a, b),
                0x2a => self.arith(Op::Mul, a, b),
                0x2b => self.arith(Op::Max, a, b),
                0x2c => self.arith(Op::Sub, a, b),
                0x2d => self.arith(Op::Msub, a, b),
                0x2f => self.arith(Op::Mini, a, b),
                0x2e => {
                    // vopmsub
                    let prod = [fpu::mul(a[1], b[2]), fpu::mul(a[2], b[0]), fpu::mul(a[0], b[1])];
                    let v = std::array::from_fn(|k| if k < 3 { fpu::sub(self.vacc[k], prod[k]) } else { 0 });
                    self.vset(fd, dest & 0xe, v);
                    return Ok(Flow::Next);
                }
                _ => return vubad,
            };
            self.vset(fd, dest, res);
            return Ok(Flow::Next);
        }
        let code = ((w >> 6) & 31) << 2 | (w & 3);
        let acc = match code {
            0x00..=0x0f => Some((BC_OPS[(code >> 2) as usize], bcv)),
            0x18..=0x1b => Some((Op::Mul, bcv)),
            0x1c => Some((Op::Mul, qv)),
            0x20 => Some((Op::Add, qv)),
            0x21 => Some((Op::Madd, qv)),
            0x24 => Some((Op::Sub, qv)),
            0x25 => Some((Op::Msub, qv)),
            0x28 => Some((Op::Add, b)),
            0x29 => Some((Op::Madd, b)),
            0x2a => Some((Op::Mul, b)),
            0x2c => Some((Op::Sub, b)),
            0x2d => Some((Op::Msub, b)),
            _ => None,
        };
        if let Some((op, y)) = acc {
            let res = self.arith(op, a, y);
            for (k, v) in res.into_iter().enumerate() {
                if dest & (8 >> k) != 0 {
                    self.vacc[k] = v;
                }
            }
            return Ok(Flow::Next);
        }
        let shift = |code: u32| [0, 4, 12, 15][(code & 3) as usize];
        match code {
            0x10..=0x13 => {
                // vitof0/4/12/15
                let sh = shift(code);
                let res = a.map(fpu::from_int);
                let res = if sh == 0 { res } else { res.map(|x| fpu::div(x, fpu::from_int(1 << sh))) };
                self.vset(ft, dest, res);
            }
            0x14..=0x17 => {
                // vftoi0/4/12/15
                let sh = shift(code);
                let res = a.map(|x| fpu::to_int(if sh == 0 { x } else { fpu::mul(x, fpu::from_int(1 << sh)) }));
                self.vset(ft, dest, res);
            }
            0x1d => self.vset(ft, dest, a.map(|x| x & 0x7fff_ffff)), // vabs
            0x2e => {
                // vopmula
                self.vacc[0] = fpu::mul(a[1], b[2]);
                self.vacc[1] = fpu::mul(a[2], b[0]);
                self.vacc[2] = fpu::mul(a[0], b[1]);
            }
            0x30 => self.vset(ft, dest, a),                        // vmove
            0x31 => self.vset(ft, dest, [a[1], a[2], a[3], a[0]]), // vmr32
            0x38 => self.q = fpu::div(a[((w >> 21) & 3) as usize], b[((w >> 23) & 3) as usize]),
            0x39 => self.q = fpu::sqrt(b[((w >> 23) & 3) as usize]),
            0x3a => self.q = fpu::rsqrt(a[((w >> 21) & 3) as usize], b[((w >> 23) & 3) as usize]),
            0x2f | 0x3b => {} // vnop, vwaitq
            _ => return vubad,
        }
        Ok(Flow::Next)
    }

    /// test_anim's VuMachine.mmi. The instruction is named the way
    /// `mips.decode` names it, so an encoding with a must-be-zero field set
    /// (which mips.py calls `.word`) stops, as it does there.
    pub fn mmi(&mut self, pc: u64, w: u32) -> Result<Flow, Fault> {
        let (rs, rt, rd, sa) =
            (((w >> 21) & 31) as usize, ((w >> 16) & 31) as usize, ((w >> 11) & 31) as usize, (w >> 6) & 31);
        let (a, b) = (self.r[rs], self.r[rt]);
        let hi = self.hi | (self.hi1 << 64);
        let lo = self.lo | (self.lo1 << 64);
        let v = match (w & 63, sa) {
            (0x08, 0x12) => {
                // pextlw
                let (a, b) = (words(a), words(b));
                from_words([b[0], a[0], b[1], a[1]])
            }
            (0x28, 0x12) => {
                // pextuw
                let (a, b) = (words(a), words(b));
                from_words([b[2], a[2], b[3], a[3]])
            }
            (0x09, 0x0e) => (b & M64) | ((a & M64) << 64), // pcpyld
            (0x29, 0x0e) => (a >> 64) | ((b >> 64) << 64), // pcpyud
            (0x08, 0x16) => {
                // pextlh
                let (a, b) = (halves(a), halves(b));
                from_halves([b[0], a[0], b[1], a[1], b[2], a[2], b[3], a[3]])
            }
            (0x08, 0x05) => {
                // psubh
                let (a, b) = (halves(a), halves(b));
                from_halves(std::array::from_fn(|i| a[i].wrapping_sub(b[i])))
            }
            (0x08, 0x14) => {
                // paddsh
                let (a, b) = (halves(a), halves(b));
                from_halves(std::array::from_fn(|i| (a[i] as i16).saturating_add(b[i] as i16) as u16))
            }
            (0x08, 0x17) => {
                // ppach
                let (a, b) = (halves(a), halves(b));
                from_halves([b[0], b[2], b[4], b[6], a[0], a[2], a[4], a[6]])
            }
            (0x3f, _) if rs == 0 => {
                // psraw
                from_words(words(b).map(|x| ((x as i32) >> sa) as u32))
            }
            (0x29, 0x08 | 0x09) if rt == 0 && rd == 0 => {
                // pmthi / pmtlo: the other pair's upper half folds in its
                // second pipeline's (Python's `hi >> 64` of `hi | hi1 << 64`).
                let (h, h1, l, l1) = if sa == 0x08 {
                    (a & M64, a >> 64, self.lo & M64, (self.lo >> 64) | self.lo1)
                } else {
                    (self.hi & M64, (self.hi >> 64) | self.hi1, a & M64, a >> 64)
                };
                (self.hi, self.hi1, self.lo, self.lo1) = (h, h1, l, l1);
                return Ok(Flow::Next);
            }
            (0x09, 0x10) => {
                // pmaddh
                let (a, b) = (halves(a), halves(b));
                let mut h = words(hi).map(|x| i64::from(x as i32));
                let mut l = words(lo).map(|x| i64::from(x as i32));
                for (k, (is_hi, j)) in
                    [(false, 0), (false, 1), (true, 0), (true, 1), (false, 2), (false, 3), (true, 2), (true, 3)]
                        .into_iter()
                        .enumerate()
                {
                    let p = i64::from(a[k] as i16) * i64::from(b[k] as i16);
                    if is_hi {
                        h[j] += p;
                    } else {
                        l[j] += p;
                    }
                }
                let hi = from_words(h.map(|x| x as u32));
                let lo = from_words(l.map(|x| x as u32));
                (self.hi, self.hi1, self.lo, self.lo1) = (hi & M64, hi >> 64, lo & M64, lo >> 64);
                from_words([l[0] as u32, h[0] as u32, l[2] as u32, h[2] as u32])
            }
            _ => return Err(Fault::VuBad { pc, w }),
        };
        if rd != 0 {
            self.r[rd] = v;
        }
        Ok(Flow::Next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> (Vec<u8>, Vec<u8>) {
        (vec![0; RAM], vec![0; VU_MEM])
    }

    fn exec(cpu: &mut Cpu, ram: &mut [u8], w: u32) -> Result<Flow, Fault> {
        let mut vu = vec![0; VU_MEM];
        cpu.exec(&mut Mem { ram, vu: &mut vu }, 0x1000, w, Traps::default())
    }

    #[test]
    fn upper_halves_survive_64_bit_writes() {
        let (mut ram, _) = mem();
        let mut cpu = Cpu::new(Features::default());
        cpu.r[8] = u128::MAX;
        // addiu t0, zero, -1: set32 keeps bits 64-127.
        exec(&mut cpu, &mut ram, 0x2408_ffff).unwrap();
        assert_eq!(cpu.r[8], u128::MAX);
        // lui t0, 1
        exec(&mut cpu, &mut ram, 0x3c08_0001).unwrap();
        assert_eq!(cpu.r[8], !M64 | 0x10000);
    }

    #[test]
    fn mult_and_div() {
        let (mut ram, _) = mem();
        let mut cpu = Cpu::new(Features::default());
        cpu.r[4] = 0xffff_ffff; // -1
        cpu.r[5] = 3;
        // multu a0, a1 -> 0x2_ffff_fffd
        exec(&mut cpu, &mut ram, 0x0085_0019).unwrap();
        assert_eq!(cpu.lo, 0xffff_ffff_ffff_fffd);
        assert_eq!(cpu.hi, 2);
        // mult a0, a1 -> -3
        exec(&mut cpu, &mut ram, 0x0085_0018).unwrap();
        assert_eq!(cpu.lo, 0xffff_ffff_ffff_fffd);
        assert_eq!(cpu.hi, 0xffff_ffff_ffff_ffff);
        // div a0, a1: -1 / 3 = 0 remainder -1
        exec(&mut cpu, &mut ram, 0x0085_001a).unwrap();
        assert_eq!((cpu.lo, cpu.hi), (0, 0xffff_ffff_ffff_ffff));
        // divide by zero leaves them, unless ee_div.
        cpu.r[5] = 0;
        exec(&mut cpu, &mut ram, 0x0085_001a).unwrap();
        assert_eq!((cpu.lo, cpu.hi), (0, 0xffff_ffff_ffff_ffff));
        cpu.features.ee_div = true;
        cpu.r[4] = 5;
        exec(&mut cpu, &mut ram, 0x0085_001a).unwrap();
        assert_eq!((cpu.lo, cpu.hi), (0xffff_ffff_ffff_ffff, 5));
    }

    #[test]
    fn unknown_instructions_stop() {
        let (mut ram, _) = mem();
        let mut cpu = Cpu::new(Features::default());
        // syscall
        assert_eq!(exec(&mut cpu, &mut ram, 0x0000_000c), Err(Fault::Bad { pc: 0x1000, w: 0xc }));
        // lqc2 without the VU features
        assert!(matches!(exec(&mut cpu, &mut ram, 0xd800_0000), Err(Fault::Bad { .. })));
        cpu.features.vu = true;
        assert_eq!(exec(&mut cpu, &mut ram, 0xd800_0000), Ok(Flow::Next));
        // psraw with rs set is not psraw to mips.py
        assert!(matches!(exec(&mut cpu, &mut ram, 0x7020_083f), Err(Fault::VuBad { .. })));
        assert_eq!(exec(&mut cpu, &mut ram, 0x7000_083f), Ok(Flow::Next));
    }

    #[test]
    fn stores_outside_ram_stop() {
        let (mut ram, _) = mem();
        let mut cpu = Cpu::new(Features::default());
        cpu.r[4] = u128::from(RAM as u32 - 2);
        // sw zero, 0(a0)
        assert_eq!(exec(&mut cpu, &mut ram, 0xac80_0000), Err(Fault::Write { addr: RAM as u32 - 2, n: 4 }));
        assert_eq!(exec(&mut cpu, &mut ram, 0xa080_0000), Ok(Flow::Next));
    }

    #[test]
    fn branch_likely_annuls() {
        let (mut ram, _) = mem();
        let mut cpu = Cpu::new(Features::default());
        cpu.r[4] = 1;
        // beql a0, zero, +4: not taken
        assert_eq!(exec(&mut cpu, &mut ram, 0x5080_0004), Ok(Flow::Annul));
        // bne a0, zero, -1: taken back to itself
        assert_eq!(exec(&mut cpu, &mut ram, 0x1480_ffff), Ok(Flow::Jump(0x1000)));
    }

    #[test]
    fn hookset_bits() {
        let mut h = HookSet::default();
        h.insert(0x100, HookKind::Foreign);
        h.insert(0x101, HookKind::Foreign);
        assert_eq!(h.get(0x100), Some(HookKind::Foreign));
        assert_eq!(h.get(0x104), None);
        h.remove(0x100);
        assert_eq!(h.get(0x100), None);
        assert_eq!(h.get(0x101), Some(HookKind::Foreign));
        h.remove(0x101);
        assert!(h.is_empty());
        h.insert(RETURN_SENTINEL + 0x100, HookKind::Native(Hle::Strlen));
        assert_eq!(h.get(RETURN_SENTINEL + 0x100), Some(HookKind::Native(Hle::Strlen)));
    }
}
