---
number: 467
title: piney-gs's flaky tests ran out of address space: a wgpu device a test under ulimit -v lost its devices; every headless GS now shares one device, and the tests one archive
date: 2026-10-24
area: test, render
files: crates/piney-gs/src/lib.rs
started: 2026-10-24T15:44:02-06:00
took: 9m
---

# 467. piney-gs's flaky tests ran out of address space: a wgpu device a test under ulimit -v lost its devices; every headless GS now shares one device, and the tests one archive

## The flake

piney-gs's 25 tests failed in the workspace run of [[465]]: five there,
five others when run again on their own. They passed alone with one
thread. The failures were black frames (`[0, 0, 0, 255]`), zero bytes
read back, a map that failed (`r.expect("map")`) and a target of the
wrong size. Those are what a lost device gives: `read_back` returns black
once `watch_loss` has set the flag, and an out-of-memory error counts as
lost.

**The cause.** Every run that failed was under `ulimit -v 8000000`, the
rule for long jobs.
- Each test made its own device (`Gs::headless`: an instance, an
  adapter, a device). Each also read Infection's 136 MB `DATA.BIN` into
  an archive of its own.
- On this machine's driver (NVIDIA, RTX 5090) a device costs about
  600 MB of address space. One thread peaked at 1.99 GB of address space
  (`VmPeak`). Four threads peaked at 3.79 GB. The default 32 threads
  peaked at 7.79 GB, just under the 8 GB cap.
- Which tests overlap, and so the peak, changes from run to run.
  Whichever tests' allocations crossed the cap lost their devices.
- Lowering the cap to 7 GB made it fail every time: four tests failed
  and the run aborted. One thread under the same cap passed.

No GPU memory ran short: the run peaked at about 4 GB of the card's 32 GB
in use.

**The fix.**
- **The device.** `Gs::headless` now takes the process's one headless
  device (`headless_device`). It is made on first use and made again
  once lost. Its loss flag is set up once for the device, and each GS on
  it reads that flag (`Gs::on`). `Gs::new` still watches the device it
  is given.
- **The lost-device test.** `a_lost_device_stops_the_drawing_and_a_new_one_resumes_it`
  destroys its device, so it now takes one of its own. `gs_limited`
  already had one.
- **The archive.** The tests' `gs()` reads Infection's archive once
  (`OnceLock`), not once a test.

Every headless user gains the shared device too. That covers piney-game's
shot tests and piney-stream's `drain_render`, which made a device each.

**Checked.**
- `VmPeak` at 32 threads is 2.92 GB, down from 7.79 GB. The suite passes
  three times each under 4 GB and 3 GB caps. It runs in 0.55 s, down
  from 2.6 s.
- Two `cargo test --workspace --exclude piney-game` runs under the
  8 GB cap passed with no failure.
- `staff_roll_shots` still renders its frames (one looked at), and
  piney-stream's `drain_render` passes.
- clippy and fmt pass.

**Still unknown:** nothing.
