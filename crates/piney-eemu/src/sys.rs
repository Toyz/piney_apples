//! A whole Emotion Engine program run from `_start`: the interpreter
//! ([`crate::cpu`]) under a stand-in for the EE kernel (threads, semaphores,
//! interrupt handlers, alarms), the scratchpad, RAM's uncached mirrors and
//! the hardware's registers ([`Io`]). The IOP's services (the disc, the
//! pads, the memory cards, sound) are answered by hooks the caller sets on
//! the program's own library functions ([`Sys::hook`]). Enough to run the
//! game's tasks frame after frame and watch what they do to its heap.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::cpu::{self, Bus, Cpu, Event, Fault, Features, HookKind, HookSet, Mem, RAM, Run, RunOpts, VU_MEM};

/// The end of user RAM.
pub const RAM_END: u32 = RAM as u32;
/// Where interrupt handlers run (kernel memory, below the program).
const ISR_STACK: u32 = 0x000f_f000;
/// Steps a context runs before the loop looks up again.
const SLICE: u64 = 1 << 20;

/// The scratchpad, the uncached mirrors and the hardware registers.
#[derive(Default)]
pub struct Io {
    pub spr: Vec<u8>,
    /// Hardware register words by address (0x1000_0000 - 0x1000_ffff, the GS's
    /// 0x1200_0000 - 0x1200_1fff): what was last written.
    pub regs: HashMap<u32, u32>,
    /// Reads of registers never written, by address (for a log).
    pub unknown: HashMap<u32, u32>,
    /// DMA channels started (CHCR's STR written), whose end interrupts are
    /// due.
    pub dma_started: Vec<u32>,
}

/// [`Io`] as the CPU's bus.
pub struct IoBus(pub Rc<RefCell<Io>>);

impl Io {
    fn reg_read(&mut self, addr: u32) -> u32 {
        let a = addr & !3;
        // DMA channels' CHCR: never busy (STR clear).
        if (0x1000_8000..0x1000_e000).contains(&a) && a & 0xff == 0 {
            return self.regs.get(&a).copied().unwrap_or(0) & !0x100;
        }
        // Timers' counts run.
        if matches!(a, 0x1000_0000 | 0x1000_0800 | 0x1000_1000 | 0x1000_1800) {
            let v = self.regs.entry(a).or_insert(0);
            *v = v.wrapping_add(0x40) & 0xffff;
            return *v;
        }
        match self.regs.get(&a) {
            Some(&v) => v,
            None => {
                *self.unknown.entry(a).or_insert(0) += 1;
                0
            }
        }
    }
}

impl Io {
    fn q64(&self, ram: &[u8], addr: u32) -> u64 {
        if addr & 0x8000_0000 != 0 {
            let a = (addr & 0x3ff0) as usize;
            u64::from_le_bytes(self.spr[a..a + 8].try_into().unwrap())
        } else {
            let a = (addr & 0x01ff_fff0) as usize;
            u64::from_le_bytes(ram[a..a + 8].try_into().unwrap())
        }
    }

    /// A source-chain DMA (VIF1, GIF) started at the channel `ch`: its tags
    /// walked as the DMAC walks them, MADR, TADR and CHCR's TAG left as at
    /// the end (the transfer itself goes nowhere).
    fn chain(&mut self, ram: &[u8], ch: u32) {
        let mut tadr = self.regs.get(&(ch + 0x30)).copied().unwrap_or(0);
        let mut asr: Vec<u32> = Vec::new();
        let mut madr;
        let mut last = 0u64;
        for _ in 0..1_000_000 {
            let tag = self.q64(ram, tadr);
            last = tag;
            let qwc = (tag & 0xffff) as u32;
            let id = (tag >> 28) & 7;
            let addr = ((tag >> 32) as u32) & 0x7fff_fff0 | ((tag >> 32) as u32 & 0x8000_0000);
            let irq = (tag >> 31) & 1 != 0;
            let mut end = false;
            match id {
                0 => {
                    madr = addr + 16 * qwc;
                    tadr += 16;
                    end = true;
                }
                1 => {
                    madr = tadr + 16 + 16 * qwc;
                    tadr = madr;
                }
                2 => {
                    madr = tadr + 16 + 16 * qwc;
                    tadr = addr;
                }
                3 | 4 => {
                    madr = addr + 16 * qwc;
                    tadr += 16;
                }
                5 => {
                    madr = tadr + 16 + 16 * qwc;
                    asr.push(madr);
                    tadr = addr;
                }
                6 => {
                    madr = tadr + 16 + 16 * qwc;
                    match asr.pop() {
                        Some(t) => tadr = t,
                        None => end = true,
                    }
                }
                _ => {
                    madr = tadr + 16 + 16 * qwc;
                    end = true;
                }
            }
            self.regs.insert(ch + 0x10, madr);
            self.regs.insert(ch + 0x30, tadr);
            let chcr = self.regs.get(&ch).copied().unwrap_or(0);
            if end || (irq && chcr & 0x80 != 0) {
                break;
            }
        }
        let chcr = self.regs.get(&ch).copied().unwrap_or(0);
        self.regs.insert(ch, (chcr & 0xffff & !0x100) | (((last >> 16) as u32 & 0xffff) << 16));
    }
}

impl Bus for IoBus {
    fn read(&mut self, ram: &[u8], addr: u32, n: u32) -> Option<u128> {
        let mut io = self.0.borrow_mut();
        match addr {
            0x2000_0000..0x2200_0000
            | 0x3000_0000..0x3200_0000
            | 0x8000_0000..0x8200_0000
            | 0xa000_0000..0xa200_0000 => cpu::load(ram, addr & 0x01ff_ffff, n).ok(),
            0x7000_0000..0x7000_4000 => {
                let a = (addr & 0x3fff) as usize;
                let b = io.spr.get(a..a + n as usize)?;
                Some(b.iter().rev().fold(0u128, |v, &x| v << 8 | u128::from(x)))
            }
            0x1000_0000..0x1001_0000 | 0x1200_0000..0x1200_2000 => {
                let mut v = 0u128;
                let words = n.div_ceil(4).max(1);
                for k in (0..words).rev() {
                    v = v << 32 | u128::from(io.reg_read(addr + 4 * k));
                }
                let shift = 8 * (addr & 3);
                let mask = if n >= 16 { u128::MAX } else { (1u128 << (8 * n)) - 1 };
                Some((v >> shift) & mask)
            }
            _ => None,
        }
    }

    fn write(&mut self, ram: &mut [u8], addr: u32, n: u32, v: u128) -> bool {
        let mut io = self.0.borrow_mut();
        match addr {
            0x2000_0000..0x2200_0000
            | 0x3000_0000..0x3200_0000
            | 0x8000_0000..0x8200_0000
            | 0xa000_0000..0xa200_0000 => {
                let a = (addr & 0x01ff_ffff) as usize;
                ram[a..a + n as usize].copy_from_slice(&v.to_le_bytes()[..n as usize]);
                true
            }
            0x7000_0000..0x7000_4000 => {
                let a = (addr & 0x3fff) as usize;
                if a + n as usize > io.spr.len() {
                    return false;
                }
                io.spr[a..a + n as usize].copy_from_slice(&v.to_le_bytes()[..n as usize]);
                true
            }
            0x1000_0000..0x1001_0000 | 0x1200_0000..0x1200_2000 => {
                let words = n.div_ceil(4).max(1);
                for k in 0..words {
                    let a = (addr & !3) + 4 * k;
                    let w = (v >> (32 * k)) as u32;
                    io.regs.insert(a, w);
                    if (a == 0x1000_9000 || a == 0x1000_a000) && w & 0x100 != 0 && (w >> 2) & 3 == 1 {
                        io.chain(ram, a);
                    }
                    if (0x1000_8000..0x1000_e000).contains(&a) && a & 0x3ff == 0 && w & 0x100 != 0 {
                        let ch = match a {
                            0x1000_8000 => 0,
                            0x1000_9000 => 1,
                            0x1000_a000 => 2,
                            0x1000_b000 => 3,
                            0x1000_b400 => 4,
                            0x1000_c000 => 5,
                            0x1000_c400 => 6,
                            0x1000_c800 => 7,
                            0x1000_d000 => 8,
                            _ => 9,
                        };
                        io.dma_started.push(ch);
                    }
                }
                true
            }
            _ => false,
        }
    }
}

/// A thread's state, as `ReferThreadStatus` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Dormant,
    Ready,
    Running,
    Sleeping,
    /// Waiting on a semaphore, or on the next vertical blank.
    Sema(u32),
    VSync,
    Suspended,
}

#[derive(Clone)]
struct Ctx {
    cpu: Cpu,
    run: Run,
}

/// A kernel thread.
pub struct Thread {
    pub id: u32,
    pub func: u32,
    pub stack: u32,
    pub stack_size: u32,
    pub gp: u32,
    pub init_prio: i32,
    pub prio: i32,
    pub attr: u32,
    pub option: u32,
    pub state: State,
    pub wakeups: i32,
    ctx: Option<Ctx>,
}

/// A kernel semaphore.
pub struct Sema {
    pub count: i32,
    pub max: i32,
    pub init: i32,
    pub attr: u32,
    pub option: u32,
    pub waiters: VecDeque<u32>,
}

/// A registered interrupt handler: its cause, function, argument and `$gp`.
#[derive(Clone, Copy, Debug)]
pub struct Handler {
    pub cause: u32,
    pub func: u32,
    pub arg: u32,
    pub gp: u32,
}

/// An alarm: when (in horizontal blanks), its function, argument, `$gp`.
#[derive(Clone, Copy, Debug)]
struct Alarm {
    id: u32,
    at: u64,
    func: u32,
    arg: u32,
    gp: u32,
}

/// What a hook does.
pub enum Hooked {
    /// Return to the caller with `$v0`.
    Ret(u32),
    /// The thread waits (its state already set); it returns `$v0` when it
    /// runs again.
    Wait(u32),
}

pub type HookFn = Rc<RefCell<dyn FnMut(&mut Sys, [u32; 4]) -> Hooked>>;

/// Horizontal blanks a frame (NTSC).
pub const HBLANKS: u64 = 262;

pub struct Sys {
    pub cpu: Cpu,
    pub run: Run,
    pub ram: Vec<u8>,
    pub vu: Vec<u8>,
    pub io: Rc<RefCell<Io>>,
    pub threads: Vec<Thread>,
    /// The running thread's index in `threads`, if any.
    pub cur: Option<usize>,
    ready: Vec<VecDeque<usize>>,
    pub semas: HashMap<u32, Sema>,
    next_sema: u32,
    pub handlers: Vec<Handler>,
    pub dmac: Vec<Handler>,
    pub enabled_intc: u32,
    pub enabled_dmac: u32,
    /// DMA end interrupts not yet taken (their channel not enabled).
    pub dma_pending: u32,
    alarms: Vec<Alarm>,
    next_alarm: u32,
    hookset: HookSet,
    hooks: HashMap<u32, HookFn>,
    /// Hooks on overlay code: taken only while this word is at the address.
    guards: HashMap<u32, u32>,
    /// Watches: run, then the instruction at the address runs as usual.
    watches: HashMap<u32, HookFn>,
    pub heap_start: u32,
    pub heap_end: u32,
    pub hblank: u64,
    pub vblanks: u64,
    /// Syscalls not stood in for, by number (logged once each).
    pub unknown_syscalls: HashMap<i32, u32>,
    pub log: bool,
    /// The interrupted thread's context while a handler runs.
    in_isr: bool,
    /// Syscalls the program set itself (`SetSyscall`): number -> handler.
    pub custom: HashMap<i32, u32>,
    /// COP0 Status: `ei` and `di` set and clear EIE (bit 16).
    pub status: u32,
    /// Instructions run, all contexts.
    pub executed: u64,
}

impl Sys {
    pub fn new() -> Sys {
        let io = Rc::new(RefCell::new(Io { spr: vec![0; 0x4000], ..Io::default() }));
        cpu::set_bus(Some(Box::new(IoBus(io.clone()))));
        let features = Features { vu: true, vi: true, ee_div: true, full: true };
        Sys {
            cpu: Cpu::new(features),
            run: Run::new(0, SLICE),
            ram: vec![0; RAM],
            vu: vec![0; VU_MEM],
            io,
            threads: Vec::new(),
            cur: None,
            ready: (0..128).map(|_| VecDeque::new()).collect(),
            semas: HashMap::new(),
            next_sema: 1,
            handlers: Vec::new(),
            dmac: Vec::new(),
            enabled_intc: 0,
            enabled_dmac: 0,
            dma_pending: 0,
            alarms: Vec::new(),
            next_alarm: 1,
            hookset: HookSet::default(),
            hooks: HashMap::new(),
            guards: HashMap::new(),
            watches: HashMap::new(),
            heap_start: 0,
            heap_end: 0,
            hblank: 0,
            vblanks: 0,
            unknown_syscalls: HashMap::new(),
            log: false,
            in_isr: false,
            custom: HashMap::new(),
            status: 0x7003_0c13,
            executed: 0,
        }
    }

    // memory -------------------------------------------------------------------

    pub fn r32(&self, a: u32) -> u32 {
        cpu::load(&self.ram, a, 4).map_or(0, |v| v as u32)
    }

    pub fn w32(&mut self, a: u32, v: u32) {
        let _ = self.cpu.store(&mut self.ram, a, 4, u128::from(v));
    }

    pub fn r8(&self, a: u32) -> u8 {
        cpu::load(&self.ram, a, 1).map_or(0, |v| v as u8)
    }

    /// `bytes` at `a`, through the bus (the scratchpad and the mirrors too).
    pub fn write(&mut self, a: u32, bytes: &[u8]) {
        if u64::from(a) + bytes.len() as u64 <= u64::from(RAM_END) {
            self.ram[a as usize..a as usize + bytes.len()].copy_from_slice(bytes);
            return;
        }
        for (k, &b) in bytes.iter().enumerate() {
            let _ = self.cpu.store(&mut self.ram, a + k as u32, 1, u128::from(b));
        }
    }

    pub fn cstr(&self, a: u32) -> String {
        let mut out = Vec::new();
        let mut p = a;
        loop {
            let b = self.r8(p);
            if b == 0 || out.len() > 4096 {
                break;
            }
            out.push(b);
            p += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    // hooks --------------------------------------------------------------------

    /// Run `f` instead of the function at `addr`.
    pub fn hook(&mut self, addr: u32, f: impl FnMut(&mut Sys, [u32; 4]) -> Hooked + 'static) {
        self.hooks.insert(addr, Rc::new(RefCell::new(f)));
        self.hookset.insert(u64::from(addr), HookKind::Foreign);
    }

    /// [`Sys::hook`] on an overlay's function: taken only while `word` (its
    /// first instruction) is at `addr`.
    pub fn hook_overlay(&mut self, addr: u32, word: u32, f: impl FnMut(&mut Sys, [u32; 4]) -> Hooked + 'static) {
        self.hook(addr, f);
        self.guards.insert(addr, word);
    }

    /// Calls `f` each time `addr` is reached; the code there then runs.
    pub fn watch(&mut self, addr: u32, mut f: impl FnMut(&mut Sys, [u32; 4]) + 'static) {
        self.watches.insert(
            addr,
            Rc::new(RefCell::new(move |s: &mut Sys, a: [u32; 4]| {
                f(s, a);
                Hooked::Ret(0)
            })),
        );
        self.hookset.insert(u64::from(addr), HookKind::Foreign);
    }

    /// One of eemu's HLE functions at `addr`.
    pub fn hle(&mut self, addr: u32, h: crate::hle::Hle) {
        self.hooks.remove(&addr);
        self.hookset.insert(u64::from(addr), HookKind::Native(h));
    }

    /// The function at `addr` returns `v` at once.
    pub fn stub(&mut self, addr: u32, v: u32) {
        self.hook(addr, move |_, _| Hooked::Ret(v));
    }

    // threads ------------------------------------------------------------------

    /// A stopped thread's pc and `$ra`.
    pub fn thread_pc(&self, k: usize) -> Option<(u32, u32)> {
        self.threads[k].ctx.as_ref().map(|c| (c.run.pc as u32, c.cpu.r[31] as u32))
    }

    /// A stopped thread's register `r`.
    pub fn thread_reg(&self, k: usize, r: usize) -> Option<u32> {
        self.threads[k].ctx.as_ref().map(|c| c.cpu.r[r] as u32)
    }

    /// A stopped thread's `$sp`.
    pub fn thread_sp(&self, k: usize) -> Option<u32> {
        self.threads[k].ctx.as_ref().map(|c| c.cpu.r[29] as u32)
    }

    pub fn current_id(&self) -> u32 {
        self.cur.map_or(0, |k| self.threads[k].id)
    }

    fn make_ready(&mut self, k: usize, front: bool) {
        self.threads[k].state = State::Ready;
        let p = self.threads[k].prio.clamp(0, 127) as usize;
        self.ready.iter_mut().for_each(|q| q.retain(|&x| x != k));
        if front {
            self.ready[p].push_front(k);
        } else {
            self.ready[p].push_back(k);
        }
    }

    fn best_ready(&self) -> Option<(usize, i32)> {
        self.ready.iter().enumerate().find_map(|(p, q)| q.front().map(|&k| (k, p as i32)))
    }

    /// Saves the running thread's context.
    fn save(&mut self) {
        if let Some(k) = self.cur {
            self.threads[k].ctx = Some(Ctx { cpu: self.cpu.clone(), run: self.run });
        }
    }

    fn load_ctx(&mut self, k: usize) {
        let ctx = self.threads[k].ctx.take().expect("a context to run");
        let (vf, vi, vacc, q) = (self.cpu.vf, self.cpu.vi, self.cpu.vacc, self.cpu.q);
        self.cpu = ctx.cpu;
        (self.cpu.vf, self.cpu.vi, self.cpu.vacc, self.cpu.q) = (vf, vi, vacc, q);
        self.run = ctx.run;
        self.threads[k].state = State::Running;
        self.cur = Some(k);
    }

    /// The running thread gives way if a better one is ready (after a wake
    /// or a start): it stays at the head of its queue.
    fn preempt_check(&mut self) {
        if self.in_isr {
            return;
        }
        let Some(k) = self.cur else { return };
        if let Some((_, p)) = self.best_ready()
            && p < self.threads[k].prio
        {
            self.save();
            self.make_ready(k, true);
            self.cur = None;
        }
    }

    /// The running thread stops (its state set); the next ready one runs.
    fn block(&mut self) {
        self.save();
        self.cur = None;
    }

    fn wake(&mut self, k: usize) {
        if self.threads[k].state == State::Sleeping {
            self.make_ready(k, false);
        } else {
            self.threads[k].wakeups += 1;
        }
    }

    // the kernel ---------------------------------------------------------------

    fn arg(&self, i: usize) -> u32 {
        self.cpu.r[4 + i] as u32
    }

    /// One syscall at `pc`: its number in `$v1`.
    fn syscall(&mut self, pc: u64) -> Result<(), String> {
        let n = self.cpu.r[3] as u32 as i32;
        let a = [self.arg(0), self.arg(1), self.arg(2), self.arg(3)];
        let t0 = self.cpu.r[8] as u32;
        let _ = pc;
        self.run.advance(crate::cpu::Flow::Next);
        let mut v: i64 = 0;
        let isr = n < 0;
        if let Some(&h) = self.custom.get(&n.abs()).filter(|&&h| h < RAM_END) {
            if self.log {
                eprintln!("custom syscall {n:#x} -> {h:#x} {a:x?}");
            }
            let gp = self.cpu.r[28] as u32;
            let r = self.call_isr(h, &a, gp)?;
            self.set_v0(r);
            return Ok(());
        }
        match n.abs() {
            0x74 => {
                // SetSyscall(n, handler)
                self.custom.insert(a[0] as i32, a[1]);
                if self.log {
                    eprintln!("SetSyscall({:#x}, {:#x})", a[0], a[1]);
                }
            }
            0x02 | 0x64 | 0x68 | 0x71 | 0x7d => {} // SetGsCrt, FlushCache, ..., GsPutIMR
            0x70 => v = 0x7f00,                    // GsGetIMR
            0x3c => {
                // SetupThread(gp, stack, size, args, root): the stack at the
                // top of RAM.
                let size = a[2];
                let top = if a[1] == u32::MAX { RAM_END } else { a[1].wrapping_add(size) };
                v = i64::from(top - 0x10);
                if let Some(k) = self.cur {
                    let t = &mut self.threads[k];
                    (t.stack, t.stack_size, t.gp) = (top - size, size, a[0]);
                }
            }
            0x3d => {
                // SetupHeap(start, size): to the main thread's stack.
                self.heap_start = a[0];
                self.heap_end = if a[1] == u32::MAX { RAM_END - 0x8000 } else { a[0] + a[1] };
                v = i64::from(self.heap_start);
            }
            0x3e => v = i64::from(self.heap_end),
            0x20 => {
                let p = a[0];
                let id = self.threads.iter().map(|t| t.id).max().unwrap_or(1) + 1;
                let prio = self.r32(p + 20) as i32;
                self.threads.push(Thread {
                    id,
                    func: self.r32(p + 4),
                    stack: self.r32(p + 8),
                    stack_size: self.r32(p + 12),
                    gp: self.r32(p + 16),
                    init_prio: prio,
                    prio,
                    attr: self.r32(p + 28),
                    option: self.r32(p + 32),
                    state: State::Dormant,
                    wakeups: 0,
                    ctx: None,
                });
                v = i64::from(id);
            }
            0x21 => {
                // DeleteThread
                if let Some(k) = self.find(a[0]) {
                    self.threads[k].state = State::Dormant;
                    self.threads[k].id = 0;
                    self.ready.iter_mut().for_each(|q| q.retain(|&x| x != k));
                }
            }
            0x22 => {
                // StartThread(id, arg)
                let Some(k) = self.find(a[0]) else { return Err(format!("StartThread({}) of no thread", a[0])) };
                let t = &self.threads[k];
                let mut c = Cpu::new(self.cpu.features);
                c.begin(u128::from(t.gp), &[a[1]]);
                c.set32(29, t.stack + t.stack_size - 0x10);
                self.threads[k].ctx = Some(Ctx { cpu: c, run: Run::new(u64::from(t.func), SLICE) });
                self.threads[k].wakeups = 0;
                self.threads[k].prio = self.threads[k].init_prio;
                self.make_ready(k, false);
                self.set_v0(0);
                self.preempt_check();
                return Ok(());
            }
            0x23 | 0x24 => {
                // ExitThread, ExitDeleteThread
                if let Some(k) = self.cur {
                    self.threads[k].state = State::Dormant;
                    if n.abs() == 0x24 {
                        self.threads[k].id = 0;
                    }
                    self.cur = None;
                }
                return Ok(());
            }
            0x25 | 0x26 => {
                // TerminateThread
                if let Some(k) = self.find(a[0]) {
                    self.threads[k].state = State::Dormant;
                    self.threads[k].ctx = None;
                    self.ready.iter_mut().for_each(|q| q.retain(|&x| x != k));
                    for s in self.semas.values_mut() {
                        s.waiters.retain(|&x| x != a[0]);
                    }
                }
            }
            0x29 | 0x2a => {
                // ChangeThreadPriority(id, prio)
                let id = if a[0] == 0 { self.current_id() } else { a[0] };
                if let Some(k) = self.find(id) {
                    v = i64::from(self.threads[k].prio);
                    self.threads[k].prio = a[1] as i32;
                    if self.threads[k].state == State::Ready {
                        self.make_ready(k, false);
                    }
                }
                self.set_v0(v as u32);
                if Some(self.find(id)) == Some(self.cur) && !isr {
                    // The running thread lowered itself: give way.
                    if let Some((_, p)) = self.best_ready()
                        && p < a[1] as i32
                    {
                        let k = self.cur.unwrap();
                        self.save();
                        self.make_ready(k, false);
                        self.cur = None;
                    }
                } else {
                    self.preempt_check();
                }
                return Ok(());
            }
            0x2b | 0x2c => {
                // RotateThreadReadyQueue(prio)
                let p = (a[0] as i32).clamp(0, 127) as usize;
                if let Some(k) = self.cur
                    && self.threads[k].prio as usize == p
                    && !isr
                {
                    self.set_v0(0);
                    self.save();
                    self.make_ready(k, false);
                    self.cur = None;
                    return Ok(());
                }
                if let Some(k) = self.ready[p].pop_front() {
                    self.ready[p].push_back(k);
                }
            }
            0x2f => v = i64::from(self.current_id()),
            0x30 | 0x31 => {
                // ReferThreadStatus(id, info)
                let id = if a[0] == 0 { self.current_id() } else { a[0] };
                match self.find(id) {
                    Some(k) => {
                        let t = &self.threads[k];
                        let st = match t.state {
                            State::Running => 1,
                            State::Ready => 2,
                            State::Sleeping | State::Sema(_) | State::VSync => 4,
                            State::Suspended => 8,
                            State::Dormant => 0x10,
                        };
                        let words = [
                            st,
                            t.func,
                            t.stack,
                            t.stack_size,
                            t.gp,
                            t.init_prio as u32,
                            t.prio as u32,
                            t.attr,
                            t.option,
                            u32::from(matches!(t.state, State::Sleeping))
                                | (u32::from(matches!(t.state, State::Sema(_))) << 1),
                            0,
                            t.wakeups as u32,
                        ];
                        if a[1] != 0 {
                            for (i, w) in words.iter().enumerate() {
                                self.w32(a[1] + 4 * i as u32, *w);
                            }
                        }
                        v = i64::from(st);
                    }
                    None => v = -1,
                }
            }
            0x32 => {
                // SleepThread
                let k = self.cur.ok_or("SleepThread outside a thread")?;
                if self.threads[k].wakeups > 0 {
                    self.threads[k].wakeups -= 1;
                } else {
                    self.set_v0(0);
                    self.threads[k].state = State::Sleeping;
                    self.block();
                    return Ok(());
                }
            }
            0x33 | 0x34 => {
                // WakeupThread(id)
                if let Some(k) = self.find(a[0]) {
                    self.wake(k);
                }
                self.set_v0(0);
                if !isr {
                    self.preempt_check();
                }
                return Ok(());
            }
            0x35 | 0x36 => {
                // CancelWakeupThread
                if let Some(k) = self.find(a[0]) {
                    v = i64::from(self.threads[k].wakeups);
                    self.threads[k].wakeups = 0;
                }
            }
            0x37 | 0x38 => {
                // SuspendThread
                if let Some(k) = self.find(a[0])
                    && self.threads[k].state == State::Ready
                {
                    self.ready.iter_mut().for_each(|q| q.retain(|&x| x != k));
                    self.threads[k].state = State::Suspended;
                }
            }
            0x39 | 0x3a => {
                // ResumeThread
                if let Some(k) = self.find(a[0])
                    && self.threads[k].state == State::Suspended
                {
                    self.make_ready(k, false);
                    self.set_v0(0);
                    if !isr {
                        self.preempt_check();
                    }
                    return Ok(());
                }
            }
            0x40 => {
                // CreateSema(param)
                let p = a[0];
                let id = self.next_sema;
                self.next_sema += 1;
                self.semas.insert(
                    id,
                    Sema {
                        count: self.r32(p + 8) as i32,
                        max: self.r32(p + 4) as i32,
                        init: self.r32(p + 8) as i32,
                        attr: self.r32(p + 12),
                        option: self.r32(p + 16),
                        waiters: VecDeque::new(),
                    },
                );
                v = i64::from(id);
            }
            0x41 => {
                self.semas.remove(&a[0]);
            }
            0x42 | 0x43 => {
                // SignalSema
                let Some(s) = self.semas.get_mut(&a[0]) else { return self.ret(-1) };
                if let Some(id) = s.waiters.pop_front() {
                    if let Some(k) = self.find(id) {
                        self.make_ready(k, false);
                    }
                    self.set_v0(a[0]);
                    if !isr {
                        self.preempt_check();
                    }
                    return Ok(());
                }
                s.count += 1;
                v = i64::from(a[0]);
            }
            0x44 => {
                // WaitSema
                let me = self.current_id();
                let Some(s) = self.semas.get_mut(&a[0]) else { return self.ret(-1) };
                if s.count > 0 {
                    s.count -= 1;
                    v = i64::from(a[0]);
                } else {
                    s.waiters.push_back(me);
                    let k = self.cur.ok_or("WaitSema outside a thread")?;
                    self.threads[k].state = State::Sema(a[0]);
                    self.set_v0(a[0]);
                    self.block();
                    return Ok(());
                }
            }
            0x45 | 0x46 => {
                // PollSema
                let Some(s) = self.semas.get_mut(&a[0]) else { return self.ret(-1) };
                if s.count > 0 {
                    s.count -= 1;
                    v = i64::from(a[0]);
                } else {
                    v = -419;
                }
            }
            0x47 | 0x48 => {
                // ReferSemaStatus(id, info)
                let Some(s) = self.semas.get(&a[0]) else { return self.ret(-1) };
                let words = [s.count as u32, s.max as u32, s.init as u32, s.waiters.len() as u32, s.attr, s.option];
                for (i, w) in words.iter().enumerate() {
                    self.w32(a[1] + 4 * i as u32, *w);
                }
                v = i64::from(a[0]);
            }
            0x10 | 0x12 => {
                // AddIntcHandler(cause, handler, next), AddDmacHandler
                let h = Handler { cause: a[0], func: a[1], arg: a[3], gp: self.cpu.r[28] as u32 };
                let list = if n.abs() == 0x10 { &mut self.handlers } else { &mut self.dmac };
                list.push(h);
                v = list.len() as i64;
            }
            0x11 | 0x13 => {}
            0x14 | 0x1a => {
                self.enabled_intc |= 1 << a[0];
                v = 1;
            }
            0x15 | 0x1b => {
                self.enabled_intc &= !(1 << a[0]);
                v = 1;
            }
            0x16 | 0x1c => {
                self.enabled_dmac |= 1 << a[0];
                v = 1;
            }
            0x17 | 0x1d => {
                self.enabled_dmac &= !(1 << a[0]);
                v = 1;
            }
            0x18 | 0x19 | 0x1e | 0x1f => v = 1,
            0xfc | 0xfe => {
                // SetAlarm(hblanks, func, arg), the alarm library's
                let id = self.next_alarm;
                self.next_alarm += 1;
                let gp = self.cpu.r[28] as u32;
                self.alarms.push(Alarm { id, at: self.hblank + u64::from(a[0] & 0xffff), func: a[1], arg: a[2], gp });
                v = i64::from(id);
            }
            0xfd | 0xff => {
                // ReleaseAlarm(id)
                self.alarms.retain(|x| x.id != a[0]);
            }
            0x77 => v = 1, // sceSifSetDma
            0x76 => v = -1,
            0x78..=0x7c => v = 0,
            _ => {
                *self.unknown_syscalls.entry(n).or_insert(0) += 1;
                if self.log {
                    eprintln!("syscall {n:#x} at {pc:#x} {a:x?} t0 {t0:#x}");
                }
            }
        }
        self.set_v0(v as u32);
        Ok(())
    }

    fn ret(&mut self, v: i64) -> Result<(), String> {
        self.set_v0(v as u32);
        Ok(())
    }

    fn set_v0(&mut self, v: u32) {
        self.cpu.set32(2, v);
    }

    fn find(&self, id: u32) -> Option<usize> {
        if id == 0 {
            return self.cur;
        }
        self.threads.iter().position(|t| t.id == id)
    }

    /// One COP0 instruction (`ei`, `di`, `mfc0`, `mtc0`, `eret`, `sync.p`).
    fn cop0(&mut self, pc: u64, w: u32) -> bool {
        let rs = (w >> 21) & 31;
        let rt = ((w >> 16) & 31) as usize;
        let rd = (w >> 11) & 31;
        match (w >> 26, rs) {
            (0x10, 0x00) => {
                // mfc0: Status with interrupts on; Count runs.
                let v = match rd {
                    12 => self.status,
                    9 => (self.cpu.steps as u32).wrapping_mul(2),
                    _ => 0,
                };
                self.cpu.set32(rt, v);
            }
            (0x10, 0x04) => {}
            (0x10, 0x10) => match w & 63 {
                0x38 => self.status |= 0x1_0000,
                0x39 => self.status &= !0x1_0000,
                _ => {}
            },
            _ => return false,
        }
        let _ = pc;
        self.run.advance(crate::cpu::Flow::Next);
        true
    }

    // interrupts ---------------------------------------------------------------

    /// Runs `func(args)` to its return on the interrupt stack, the running
    /// thread's context kept.
    pub fn call_isr(&mut self, func: u32, args: &[u32], gp: u32) -> Result<u32, String> {
        let saved = (self.cpu.clone(), self.run, self.cur);
        let status = self.status;
        let was = self.in_isr;
        self.in_isr = true;
        let mut c = Cpu::new(self.cpu.features);
        (c.vf, c.vi, c.vacc, c.q) = (self.cpu.vf, self.cpu.vi, self.cpu.vacc, self.cpu.q);
        c.begin(u128::from(gp), args);
        c.set32(29, ISR_STACK);
        self.cpu = c;
        self.run = Run::new(u64::from(func), SLICE);
        let r = self.exec_until_done();
        let v = self.cpu.r[2] as u32;
        let (vf, vi, vacc, q) = (self.cpu.vf, self.cpu.vi, self.cpu.vacc, self.cpu.q);
        (self.cpu, self.run, self.cur) = saved;
        self.status = status;
        (self.cpu.vf, self.cpu.vi, self.cpu.vacc, self.cpu.q) = (vf, vi, vacc, q);
        self.in_isr = was;
        r.map(|_| v)
    }

    /// Runs the current context until it returns (a handler or a call).
    fn exec_until_done(&mut self) -> Result<(), String> {
        loop {
            match self.step()? {
                Step::Done => return Ok(()),
                Step::Switch => return Err(format!("a handler blocked at {:#x}", self.run.pc)),
                Step::Go => {}
            }
        }
    }

    /// Calls `func(args)` from outside any thread (set-up), to its return.
    pub fn call(&mut self, func: u32, args: &[u32], gp: u32) -> Result<u32, String> {
        self.call_isr(func, args, gp)
    }

    /// A vertical blank: the waiting threads woken, the handlers of cause 2
    /// (its start) run, the alarms due fired.
    pub fn vblank(&mut self) -> Result<(), String> {
        self.vblanks += 1;
        let target = self.hblank + HBLANKS;
        while let Some(i) =
            self.alarms.iter().enumerate().filter(|(_, a)| a.at <= target).min_by_key(|(_, a)| a.at).map(|(i, _)| i)
        {
            let a = self.alarms.remove(i);
            self.hblank = a.at.max(self.hblank);
            self.call_isr(a.func, &[a.id, (a.at & 0xffff) as u32, a.arg], a.gp)?;
        }
        self.hblank = target;
        for k in 0..self.threads.len() {
            if self.threads[k].state == State::VSync {
                self.make_ready(k, false);
            }
        }
        let hs: Vec<Handler> = self.handlers.iter().copied().filter(|h| h.cause == 2).collect();
        if self.enabled_intc & 4 != 0 {
            for h in hs {
                self.call_isr(h.func, &[2, h.arg, 0], h.gp)?;
            }
        }
        let he: Vec<Handler> = self.handlers.iter().copied().filter(|h| h.cause == 3).collect();
        if self.enabled_intc & 8 != 0 {
            for h in he {
                self.call_isr(h.func, &[3, h.arg, 0], h.gp)?;
            }
        }
        Ok(())
    }

    /// The running thread spins until the next vertical blank (nothing
    /// below it runs meanwhile), which then comes.
    pub fn spin_vsync(&mut self) -> Result<(), String> {
        self.vblank()
    }

    // running ------------------------------------------------------------------

    /// The end interrupts of the DMA started since the last look: each
    /// enabled channel's handlers.
    fn dma_ends(&mut self) -> Result<(), String> {
        if self.in_isr || self.status & 0x1_0000 == 0 {
            return Ok(());
        }
        for ch in std::mem::take(&mut self.io.borrow_mut().dma_started) {
            self.dma_pending |= 1 << ch;
        }
        let due = self.dma_pending & self.enabled_dmac;
        if due == 0 {
            return Ok(());
        }
        self.dma_pending &= !due;
        for ch in 0..10 {
            if due & (1 << ch) == 0 {
                continue;
            }
            let hs: Vec<Handler> = self.dmac.iter().copied().filter(|h| h.cause == ch).collect();
            if self.log {
                eprintln!("dma end {ch}: {} handlers", hs.len());
            }
            for h in hs {
                self.call_isr(h.func, &[ch, h.arg, 0], h.gp)?;
            }
        }
        self.preempt_check();
        Ok(())
    }

    fn step(&mut self) -> Result<Step, String> {
        let r = self.step_inner();
        if !self.io.borrow().dma_started.is_empty() || self.dma_pending & self.enabled_dmac != 0 {
            self.dma_ends()?;
        }
        r
    }

    fn step_inner(&mut self) -> Result<Step, String> {
        let before = self.cpu.steps;
        let r = self.step_cpu();
        self.executed += self.cpu.steps.saturating_sub(before);
        r
    }

    fn step_cpu(&mut self) -> Result<Step, String> {
        if self.log && self.cpu.steps.is_multiple_of(1 << 24) {
            eprintln!("step {} thread {} isr {} pc {:#x}", self.cpu.steps, self.current_id(), self.in_isr, self.run.pc);
        }
        let mut mem = Mem { ram: &mut self.ram, vu: &mut self.vu };
        self.run.limit = self.cpu.steps + SLICE;
        let ev = self.cpu.run(&mut mem, &self.hookset, &mut self.run, RunOpts::default());
        match ev {
            Ok(Event::Done) => {
                if self.in_isr {
                    return Ok(Step::Done);
                }
                // A thread's function returned: ExitThread.
                if let Some(k) = self.cur {
                    self.threads[k].state = State::Dormant;
                    self.cur = None;
                }
                Ok(Step::Switch)
            }
            Ok(Event::Hook) if self.watches.contains_key(&(self.run.pc as u32)) => {
                let pc = self.run.pc;
                let f = self.watches[&(pc as u32)].clone();
                let args = self.cpu.args();
                let _ = (f.borrow_mut())(self, args);
                self.hookset.remove(pc);
                let mut mem = Mem { ram: &mut self.ram, vu: &mut self.vu };
                let r =
                    self.cpu.run(&mut mem, &self.hookset, &mut self.run, RunOpts { trace: true, ..RunOpts::default() });
                self.hookset.insert(pc, HookKind::Foreign);
                match r {
                    Ok(_) => Ok(Step::Go),
                    Err(Fault::Bad { pc, w }) if w >> 26 == 0x10 && self.cop0(pc, w) => Ok(Step::Go),
                    Err(e) => Err(format!("{e} under a watch")),
                }
            }
            Ok(Event::Hook)
                if self.guards.get(&(self.run.pc as u32)).is_some_and(|&w| w != self.r32(self.run.pc as u32)) =>
            {
                // Another overlay's code at the address: run it.
                let pc = self.run.pc;
                self.hookset.remove(pc);
                let mut mem = Mem { ram: &mut self.ram, vu: &mut self.vu };
                let r =
                    self.cpu.run(&mut mem, &self.hookset, &mut self.run, RunOpts { trace: true, ..RunOpts::default() });
                self.hookset.insert(pc, HookKind::Foreign);
                match r {
                    Ok(_) => Ok(Step::Go),
                    Err(Fault::Bad { pc, w }) => Err(format!("{pc:#x}: {w:08x} under an overlay hook")),
                    Err(e) => Err(format!("{e}")),
                }
            }
            Ok(Event::Hook) => {
                let pc = self.run.pc as u32;
                let f = self.hooks.get(&pc).cloned().ok_or_else(|| format!("no hook at {pc:#x}"))?;
                let args = self.cpu.args();
                let h = (f.borrow_mut())(self, args);
                match h {
                    Hooked::Ret(v) => {
                        self.set_v0(v);
                        let ra = self.cpu.r[31];
                        self.run.resume(ra);
                        self.preempt_check();
                        Ok(if self.cur.is_none() && !self.in_isr { Step::Switch } else { Step::Go })
                    }
                    Hooked::Wait(v) => {
                        // The hook set the thread's state: it resumes at $ra.
                        self.set_v0(v);
                        let ra = self.cpu.r[31];
                        self.run.resume(ra);
                        self.block();
                        Ok(Step::Switch)
                    }
                }
            }
            Ok(e) => Err(format!("{e:?}")),
            Err(Fault::Bad { pc, w }) if w & 0xfc00_003f == 0x0000_000c => {
                self.syscall(pc)?;
                Ok(if self.cur.is_none() && !self.in_isr { Step::Switch } else { Step::Go })
            }
            Err(Fault::Bad { pc, w }) if w >> 26 == 0x10 => {
                if self.cop0(pc, w) {
                    Ok(Step::Go)
                } else {
                    Err(format!("cop0 {w:08x} at {pc:#x}"))
                }
            }
            Err(Fault::Limit { .. }) => Ok(Step::Go),
            Err(e) => Err(format!("{e} (thread {})", self.current_id())),
        }
    }

    /// Runs threads until `n` more vertical blanks have passed, delivering
    /// one whenever no thread can run.
    pub fn run_vblanks(&mut self, n: u64) -> Result<(), String> {
        let end = self.vblanks + n;
        let mut at = (self.vblanks, self.executed);
        while self.vblanks < end {
            if self.vblanks != at.0 {
                at = (self.vblanks, self.executed);
            }
            if self.executed - at.1 > 4_000_000_000 {
                return Err(format!(
                    "no vertical blank for 4e9 steps: thread {} at {:#x}",
                    self.current_id(),
                    self.run.pc
                ));
            }
            if self.cur.is_none() {
                match self.best_ready() {
                    Some((k, _)) => {
                        let p = self.threads[k].prio.clamp(0, 127) as usize;
                        self.ready[p].retain(|&x| x != k);
                        self.load_ctx(k);
                    }
                    None => {
                        self.vblank()?;
                        continue;
                    }
                }
            }
            match self.step()? {
                Step::Go => {}
                Step::Switch | Step::Done => {}
            }
        }
        Ok(())
    }

    /// Starts the program at `entry` as the main thread.
    pub fn boot(&mut self, entry: u32) {
        self.cpu.begin(0, &[]);
        self.run = Run::new(u64::from(entry), SLICE);
        // _start's SetupThread makes the thread record.
        self.cur = None;
        self.threads.clear();
        // A placeholder main thread until SetupThread names it.
        self.threads.push(Thread {
            id: 1,
            func: entry,
            stack: RAM_END - 0x8000,
            stack_size: 0x8000,
            gp: 0,
            init_prio: 0,
            prio: 0,
            attr: 0,
            option: 0,
            state: State::Running,
            wakeups: 0,
            ctx: None,
        });
        self.cur = Some(0);
    }
}

impl Default for Sys {
    fn default() -> Self {
        Sys::new()
    }
}

enum Step {
    Go,
    Switch,
    Done,
}
