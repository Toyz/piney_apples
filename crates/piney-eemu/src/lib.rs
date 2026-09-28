//! `tools/eemu.py`, the Emotion Engine interpreter the Python harnesses run
//! the game's own code in, ported to Rust instruction for instruction.
//!
//! [`cpu`] is the interpreter: eemu's `Machine` (the EE core, the FPU with
//! the EE's own float rules from [`fpu`], eemu's C library stand-ins from
//! [`hle`]) plus `tools/test_anim.py`'s VuMachine and `tools/test_stream_rs.py`'s
//! Vu0Machine behind [`cpu::Features`]. It keeps eemu's state the way the
//! Python keeps it and stops (a [`cpu::Fault`]) on exactly what eemu stops
//! on. [`machine::Machine`] runs it over memory it owns; with the `python`
//! feature the crate is also the `eemu_rs` extension module, a drop-in for
//! `eemu.Machine` whose RAM is a real bytearray (`README.md`).
//! `tools/test_eemu_rs.py` runs both in lockstep over the harnesses'
//! workloads and fuzzes every instruction.

pub mod cpu;
pub mod fpu;
pub mod hle;
pub mod machine;
#[cfg(feature = "python")]
mod python;
