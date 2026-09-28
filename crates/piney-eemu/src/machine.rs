//! A machine that owns its memory, for Rust callers and this crate's tests.
//! The Python module wraps the same [`Cpu`] around bytearrays instead
//! (`python.rs`); both drive [`Cpu::run`], so the call loop is one piece of
//! code.

use std::collections::HashMap;

use crate::cpu::{Cpu, Event, Fault, Features, HookKind, HookSet, Mem, RAM, Run, RunOpts, VU_MEM};
use crate::hle::Hle;

/// A hook in Rust: gets the registers, RAM and a0-a3, returns `$v0`.
pub type Hook = Box<dyn FnMut(&mut Cpu, &mut [u8], [u32; 4]) -> u32>;

pub struct Machine {
    pub cpu: Cpu,
    pub ram: Vec<u8>,
    pub vu: Vec<u8>,
    /// `$gp` for every call.
    pub gp: u32,
    set: HookSet,
    hooks: HashMap<u64, Hook>,
}

impl Machine {
    pub fn new(features: Features) -> Machine {
        Machine {
            cpu: Cpu::new(features),
            ram: vec![0; RAM],
            vu: vec![0; VU_MEM],
            gp: 0,
            set: HookSet::default(),
            hooks: HashMap::new(),
        }
    }

    /// Run `f` instead of the code at `addr`.
    pub fn hook(&mut self, addr: u32, f: impl FnMut(&mut Cpu, &mut [u8], [u32; 4]) -> u32 + 'static) {
        self.hooks.insert(u64::from(addr), Box::new(f));
        self.set.insert(u64::from(addr), HookKind::Foreign);
    }

    /// Run one of eemu's HLE functions instead of the code at `addr`.
    pub fn hle(&mut self, addr: u32, h: Hle) {
        self.hooks.remove(&u64::from(addr));
        self.set.insert(u64::from(addr), HookKind::Native(h));
    }

    pub fn unhook(&mut self, addr: u32) {
        self.hooks.remove(&u64::from(addr));
        self.set.remove(u64::from(addr));
    }

    /// eemu's `call(addr, args, limit)`: `$v0`'s low word.
    pub fn call(&mut self, addr: u32, args: &[u32], limit: u64) -> Result<u32, Fault> {
        self.cpu.begin(u128::from(self.gp), args);
        let mut run = Run::new(u64::from(addr), limit);
        loop {
            let mut mem = Mem { ram: &mut self.ram, vu: &mut self.vu };
            match self.cpu.run(&mut mem, &self.set, &mut run, RunOpts::default())? {
                Event::Done => return Ok(self.cpu.r[2] as u32),
                Event::Hook => {
                    let args = self.cpu.args();
                    let f = self.hooks.get_mut(&run.pc).expect("a foreign hook has a function");
                    let v = f(&mut self.cpu, &mut self.ram, args);
                    self.cpu.set32(2, v);
                    run.resume(self.cpu.r[31]);
                }
                e => unreachable!("{e:?} without traps or tracing"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{RETURN_SENTINEL, STACK_TOP};

    const JR_RA: u32 = 0x03e0_0008;
    const NOP: u32 = 0;

    fn program(m: &mut Machine, at: u32, words: &[u32]) {
        for (i, w) in words.iter().enumerate() {
            let a = at as usize + 4 * i;
            m.ram[a..a + 4].copy_from_slice(&w.to_le_bytes());
        }
    }

    #[test]
    fn calls_return_v0() {
        let mut m = Machine::new(Features::default());
        // addu v0, a0, a1; jr ra; nop
        program(&mut m, 0x1000, &[0x0085_1021, JR_RA, NOP]);
        assert_eq!(m.call(0x1000, &[40, 2], 100), Ok(42));
        assert_eq!(m.cpu.steps, 3);
        assert_eq!(m.cpu.r[29], u128::from(STACK_TOP));
        assert_eq!(m.cpu.r[31], u128::from(RETURN_SENTINEL));
    }

    #[test]
    fn delay_slots_and_likely_branches() {
        let mut m = Machine::new(Features::default());
        // 0x1000 li v0, 1
        // 0x1004 beql zero, a0, +2 (to 0x1010), taken only when a0 is 0
        // 0x1008 addiu v0, v0, 10     delay slot: skipped when not taken
        // 0x100c addiu v0, v0, 100
        // 0x1010 jr ra
        // 0x1014 addiu v0, v0, 1000   delay slot of jr
        program(&mut m, 0x1000, &[0x2402_0001, 0x5004_0002, 0x2442_000a, 0x2442_0064, JR_RA, 0x2442_03e8]);
        assert_eq!(m.call(0x1000, &[0], 100), Ok(1011));
        assert_eq!(m.call(0x1000, &[1], 100), Ok(1101));
    }

    #[test]
    fn hooks_native_and_rust() {
        let mut m = Machine::new(Features::default());
        // move s0, ra; jal 0x2000; nop; jal 0x3000; move a0, v0; jr s0; nop
        program(&mut m, 0x1000, &[0x03e0_8021, 0x0c00_0800, NOP, 0x0c00_0c00, 0x0040_2021, 0x0200_0008, NOP]);
        m.ram[0x4000..0x4006].copy_from_slice(b"hello\0");
        m.hle(0x2000, Hle::Strlen);
        m.hook(0x3000, |_, _, a| a[0] * 2);
        assert_eq!(m.call(0x1000, &[0x4000], 100), Ok(10));
        m.unhook(0x3000);
        // 0x3000 is now code: a word of zeros (nop), and on until the limit.
        assert_eq!(m.call(0x1000, &[0x4000], 50), Err(Fault::Limit { limit: 50, pc: 0x30b0 }));
    }

    #[test]
    fn stops_on_unknown_instructions() {
        let mut m = Machine::new(Features::default());
        program(&mut m, 0x1000, &[NOP, 0x0000_000c]);
        assert_eq!(m.call(0x1000, &[], 100), Err(Fault::Bad { pc: 0x1004, w: 0xc }));
    }
}
