//! `tools/eemu.py`, the Emotion Engine interpreter the Python harnesses run the
//! game's own code in, ported to Rust instruction for instruction. [`cpu`] is
//! the interpreter (eemu's `Machine` with [`fpu`]'s float rules and [`hle`]'s
//! C library stand-ins, plus the VU machines behind [`cpu::Features`]),
//! stopping ([`cpu::Fault`]) on exactly what eemu stops on;
//! [`machine::Machine`] runs it over memory it owns. With the `python` feature
//! the crate is also `eemu_rs`, a drop-in for `eemu.Machine` (`README.md`);
//! `tools/test_eemu_rs.py` runs both in lockstep and fuzzes every instruction.

pub mod cpu;
pub mod fpu;
pub mod hle;
pub mod machine;
#[cfg(feature = "python")]
mod python;
